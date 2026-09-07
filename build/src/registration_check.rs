//! Build-time registration requirement analysis.
//! 构建期注册需求分析。

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::registry_identity::NodeId;
use super::registry_syntax::{FaceSyntax, ParentSyntax, parse_face};

// Keep the requirement cache in lockstep with build.rs identity units.
// 让需求缓存与 build.rs 的身份单元保持同一版本。
const CACHE_SCHEMA: &str = "3";

#[derive(Clone, Debug)]
struct Requirement {
    node: NodeId,
    kind: String,
    function: String,
    branch: String,
    capability: String,
    provider: String,
    parent: Option<NodeId>,
    source: String,
    line: usize,
}

#[derive(Clone, Debug)]
struct Declaration {
    id: NodeId,
    parent: Option<NodeId>,
    kind: String,
    provides: Vec<String>,
}

#[derive(Clone, Debug)]
struct CachedFace {
    id: NodeId,
    parent: Option<NodeId>,
    kind: String,
    function: String,
    line: usize,
    requires: Vec<(String, String)>,
    provides: Vec<String>,
    contract: Vec<(String, String)>,
}

pub fn aggregate(
    src: &Path,
    include_demo: bool,
    active: Option<&BTreeSet<NodeId>>,
    cache_units: Option<&Path>,
) -> BuildDiagnostics {
    let mut requirements = Vec::new();
    let mut declarations = Vec::new();
    collect(
        src,
        src,
        include_demo,
        cache_units,
        &mut requirements,
        &mut declarations,
    );
    if let Some(active) = active {
        requirements.retain(|requirement| active.contains(&requirement.node));
        declarations.retain(|declaration| active.contains(&declaration.id));
    }

    let missing = requirements
        .into_iter()
        .filter_map(|requirement| missing_requirement(requirement, &declarations))
        .collect::<Vec<_>>();
    let mut diagnostics = BuildDiagnostics::default();
    for diagnostic in missing {
        diagnostics.push(diagnostic);
    }
    diagnostics
}

fn missing_requirement(
    requirement: Requirement,
    declarations: &[Declaration],
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

fn collect(
    root: &Path,
    dir: &Path,
    include_demo: bool,
    cache_units: Option<&Path>,
    requirements: &mut Vec<Requirement>,
    declarations: &mut Vec<Declaration>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) != Some("registry_core") {
                collect(
                    root,
                    &path,
                    include_demo,
                    cache_units,
                    requirements,
                    declarations,
                );
            }
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        if !include_demo
            && path.file_name().and_then(|name| name.to_str()) == Some("compile_error_demo.rs")
        {
            continue;
        }
        if let Some(face) = cache_units.and_then(|directory| cached_face(root, &path, directory)) {
            collect_cached_face(
                &relative_display(root, &path),
                face,
                requirements,
                declarations,
            );
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let relative = relative_display(root, &path);
        let Some(face) = parse_face(&source)
            .unwrap_or_else(|error| panic!("invalid registration face in {relative}: {error}"))
        else {
            continue;
        };
        collect_face(root, &relative, face, requirements, declarations);
    }
}

fn collect_cached_face(
    relative: &str,
    face: CachedFace,
    requirements: &mut Vec<Requirement>,
    declarations: &mut Vec<Declaration>,
) {
    let CachedFace {
        id,
        parent,
        kind,
        function,
        line,
        requires: face_requirements,
        provides,
        contract,
        ..
    } = face;
    // Keep contract fields available to the upcoming structural validator.
    let _contract_fields = contract;
    declarations.push(Declaration {
        id,
        parent,
        kind: kind.clone(),
        provides,
    });
    let branch = relative
        .rsplit_once('/')
        .map_or("<root>", |(branch, _)| branch)
        .to_owned();
    for (capability, provider) in face_requirements {
        requirements.push(Requirement {
            node: id,
            kind: kind.clone(),
            function: function.clone(),
            branch: branch.clone(),
            capability,
            provider,
            parent,
            source: relative.to_owned(),
            line,
        });
    }
}

/// Read one cache unit only when its fingerprint still matches the source.
/// 只有 fingerprint 仍与源码匹配时才读取缓存单元。
fn cached_face(root: &Path, path: &Path, directory: &Path) -> Option<CachedFace> {
    let source = fs::read(path).ok()?;
    let mut input = path.to_string_lossy().as_bytes().to_vec();
    input.push(0);
    input.extend_from_slice(&source);
    let fingerprint = NodeId::from_bytes(&input).to_string();
    let content = fs::read_to_string(directory.join(format!("{fingerprint}.tsv"))).ok()?;
    if content
        .lines()
        .find_map(|line| line.strip_prefix("# schema\t"))
        != Some(CACHE_SCHEMA)
    {
        return None;
    }
    let field = |name: &str| {
        content
            .lines()
            .find_map(|line| line.strip_prefix(&format!("{name}\t")))
    };
    let relative = relative_display(root, path);
    if field("path")? != relative {
        return None;
    }
    let id = field("node")?.parse().ok()?;
    let kind = field("kind")?.to_owned();
    if kind.is_empty() || id != super::registry_identity::package_node_id(&relative, &kind) {
        return None;
    }
    let parent = field("parent").and_then(|value| value.parse().ok());
    let function = field("handle").unwrap_or("<registration-face>").to_owned();
    let line = field("line")?.parse().ok()?;
    let mut requires = Vec::new();
    let mut provides = Vec::new();
    let mut contract = Vec::new();
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("requires\t") {
            let (capability, provider) = rest.split_once('\t')?;
            requires.push((capability.to_owned(), provider.to_owned()));
        } else if let Some(provide) = line.strip_prefix("provides\t") {
            provides.push(provide.to_owned());
        } else if let Some(rest) = line.strip_prefix("contract\t") {
            let (name, value) = rest.split_once('\t')?;
            contract.push((name.to_owned(), value.to_owned()));
        }
    }
    Some(CachedFace {
        id,
        parent,
        kind,
        function,
        line,
        requires,
        provides,
        contract,
    })
}

fn collect_face(
    root: &Path,
    relative: &str,
    face: FaceSyntax,
    requirements: &mut Vec<Requirement>,
    declarations: &mut Vec<Declaration>,
) {
    let kind = face.path("kind").unwrap_or_else(|| "<unknown>".to_owned());
    let function = face
        .path("handle")
        .unwrap_or_else(|| "<registration-face>".to_owned());
    let node = super::registry_identity::package_node_id(relative, &kind);
    let parent = resolve_parent(&face, root);
    declarations.push(Declaration {
        id: node,
        parent,
        kind: kind.clone(),
        provides: face.string_list("provides").unwrap_or_default(),
    });

    let line = face
        .field_location("requires")
        .map_or(face.location.line, |location| location.line);
    let branch = relative
        .rsplit_once('/')
        .map_or("<root>", |(branch, _)| branch)
        .to_owned();
    for (capability, provider) in face.requirements("requires").unwrap_or_default() {
        requirements.push(Requirement {
            node,
            kind: kind.clone(),
            function: function.clone(),
            branch: branch.clone(),
            capability,
            provider,
            parent,
            source: relative.to_owned(),
            line,
        });
    }
}

fn resolve_parent(face: &FaceSyntax, root: &Path) -> Option<NodeId> {
    match face.parent()? {
        ParentSyntax::Root => Some(super::registry_identity::package_root_node_id()),
        ParentSyntax::FromPath { source, kind } => {
            Some(super::registry_identity::package_node_id(&source, &kind))
        }
        ParentSyntax::NodePath(module) => {
            let module = module.strip_prefix("crate::")?;
            let mut relative = module.split("::").collect::<PathBuf>();
            let name = relative.file_name()?.to_owned();
            relative.push(format!("{}.rs", name.to_string_lossy()));
            let source = fs::read_to_string(root.join(&relative)).ok()?;
            let parent = parse_face(&source).ok()??;
            let kind = parent.path("kind")?;
            Some(super::registry_identity::package_node_id(
                &relative_display(root, &relative),
                &kind,
            ))
        }
    }
}

fn has_ancestor_provider(
    mut target: NodeId,
    capability: &str,
    expected_kind: &str,
    declarations: &[Declaration],
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
    declarations: &'a [Declaration],
) -> Option<&'a Declaration> {
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

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
