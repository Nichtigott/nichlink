//! Capability requirements analysis over a registration declaration set.
//! Pure graph logic: the caller collects declarations and requirements from
//! wherever they live (build scans files, tools read snapshots) and the
//! kernel decides which requirements miss an ancestor provider.
//! 对注册声明集合做能力需求分析。纯图逻辑：由调用方从任意来源收集
//! 声明与需求（build 扫文件、工具读快照），kernel 判定哪些需求
//! 在祖先链上找不到提供者。

use std::collections::BTreeSet;

use crate::registry_core::diagnostic::BuildDiagnostic;
use crate::registry_core::identity::NodeId;

/// One registration declaration that may provide capabilities to its subtree.
/// 一条可能向子树提供能力的注册声明。
#[derive(Clone, Debug)]
pub struct CapabilityDeclaration {
    pub id: NodeId,
    pub parent: Option<NodeId>,
    pub kind: String,
    pub provides: Vec<String>,
}

/// One `requires = ...` entry collected from a registration face.
/// 从注册面收集到的一条 `requires = ...` 需求。
#[derive(Clone, Debug)]
pub struct CapabilityRequirement {
    pub node: NodeId,
    pub kind: String,
    pub function: String,
    pub branch: String,
    pub capability: String,
    pub provider: String,
    pub parent: Option<NodeId>,
    pub source: String,
    pub line: usize,
}

/// Every requirement whose ancestor chain offers no matching provider.
/// 祖先链上找不到匹配提供者的全部需求，逐条生成诊断。
pub fn missing_capabilities(
    requirements: Vec<CapabilityRequirement>,
    declarations: &[CapabilityDeclaration],
) -> Vec<BuildDiagnostic> {
    requirements
        .into_iter()
        .filter_map(|requirement| missing(requirement, declarations))
        .collect()
}

fn missing(
    requirement: CapabilityRequirement,
    declarations: &[CapabilityDeclaration],
) -> Option<BuildDiagnostic> {
    let provided = requirement.parent.is_some_and(|parent| {
        has_ancestor_provider(
            parent,
            &requirement.capability,
            &requirement.provider,
            declarations,
        )
    });
    if provided {
        return None;
    }
    let detail = requirement
        .parent
        .and_then(|parent| find_ancestor_capability(parent, &requirement.capability, declarations))
        .map(|declaration| declaration.kind.clone());
    let mut diagnostic = BuildDiagnostic::new(
        "requirements",
        format!("missing capability `{}`", requirement.capability),
    )
    .branch(requirement.branch)
    .node(requirement.node.to_string(), requirement.kind)
    .at(requirement.source, requirement.line)
    .function(requirement.function)
    .field(requirement.capability)
    .expected(requirement.provider.clone());
    if let Some(provider) = detail {
        diagnostic = diagnostic.provider(provider);
    }
    Some(diagnostic)
}

fn has_ancestor_provider(
    mut target: NodeId,
    capability: &str,
    expected_kind: &str,
    declarations: &[CapabilityDeclaration],
) -> bool {
    let mut visited = BTreeSet::new();
    loop {
        if !visited.insert(target) {
            return false;
        }
        if declarations.iter().any(|declaration| {
            declaration.id == target
                && declaration.kind == expected_kind
                && declaration.provides.iter().any(|item| item == capability)
        }) {
            return true;
        }
        let Some(parent) = declarations
            .iter()
            .find(|declaration| declaration.id == target)
            .and_then(|declaration| declaration.parent)
        else {
            return false;
        };
        if parent == target {
            return false;
        }
        target = parent;
    }
}

fn find_ancestor_capability<'a>(
    mut target: NodeId,
    capability: &str,
    declarations: &'a [CapabilityDeclaration],
) -> Option<&'a CapabilityDeclaration> {
    let mut visited = BTreeSet::new();
    loop {
        if !visited.insert(target) {
            return None;
        }
        if let Some(declaration) = declarations.iter().find(|declaration| {
            declaration.id == target && declaration.provides.iter().any(|item| item == capability)
        }) {
            return Some(declaration);
        }
        let parent = declarations
            .iter()
            .find(|declaration| declaration.id == target)
            .and_then(|declaration| declaration.parent)?;
        if parent == target {
            return None;
        }
        target = parent;
    }
}

#[cfg(test)]
mod tests {
    use super::{CapabilityDeclaration, CapabilityRequirement, missing_capabilities};
    use crate::registry_core::identity::{NodeId, ROOT_NODE_ID};

    fn id(byte: u8) -> NodeId {
        NodeId::from_raw([byte, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
    }

    fn declaration(
        byte: u8,
        parent: Option<u8>,
        kind: &str,
        provides: &[&str],
    ) -> CapabilityDeclaration {
        CapabilityDeclaration {
            id: id(byte),
            parent: parent.map(id),
            kind: kind.to_owned(),
            provides: provides.iter().map(|item| item.to_string()).collect(),
        }
    }

    fn requirement(
        byte: u8,
        parent: Option<u8>,
        capability: &str,
        provider: &str,
    ) -> CapabilityRequirement {
        CapabilityRequirement {
            node: id(byte),
            kind: "Leaf".to_owned(),
            function: "register".to_owned(),
            branch: "a".to_owned(),
            capability: capability.to_owned(),
            provider: provider.to_owned(),
            parent: parent.map(id),
            source: "a/leaf.rs".to_owned(),
            line: 3,
        }
    }

    #[test]
    fn satisfied_requirement_produces_no_diagnostic() {
        let declarations = vec![
            declaration(1, None, "CanvasProvider", &["render"]),
            declaration(2, Some(1), "Leaf", &[]),
        ];
        let requirements = vec![requirement(2, Some(1), "render", "CanvasProvider")];
        assert!(missing_capabilities(requirements, &declarations).is_empty());
    }

    #[test]
    fn missing_capability_is_reported_with_ancestor_detail() {
        let declarations = vec![
            declaration(1, None, "WrongProvider", &["render"]),
            declaration(2, Some(1), "Leaf", &[]),
        ];
        let requirements = vec![requirement(2, Some(1), "render", "CanvasProvider")];
        let missing = missing_capabilities(requirements, &declarations);
        assert_eq!(missing.len(), 1);
        let rendered = missing[0].clone().message;
        assert!(rendered.contains("missing capability `render`"));
    }

    #[test]
    fn root_requirement_without_parent_is_reported() {
        let declarations = vec![declaration(1, None, "Root", &[])];
        let requirements = vec![requirement(1, None, "render", "CanvasProvider")];
        assert_eq!(missing_capabilities(requirements, &declarations).len(), 1);
    }

    #[test]
    fn parent_cycle_does_not_hang() {
        let mut root = declaration(1, Some(2), "A", &[]);
        root.parent = Some(id(2));
        let mut other = declaration(2, Some(1), "B", &[]);
        other.parent = Some(id(1));
        let declarations = vec![root, other];
        let requirements = vec![requirement(2, Some(1), "render", "CanvasProvider")];
        let missing = missing_capabilities(requirements, &declarations);
        assert_eq!(missing.len(), 1);
        let _ = ROOT_NODE_ID;
    }
}
