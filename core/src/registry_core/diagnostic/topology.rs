use super::*;

// ---------------------------------------------------------------------------
// Face topology validation. Pure graph checks over build-collected records;
// the build surface collects the records, the kernel owns the rules.
// 注册面拓扑校验。对构建期收集的记录做纯图检查；
// 记录由 build 面收集，规则归 kernel 所有。

/// One registration face participating in a topology check.
/// 参与拓扑校验的一个注册面。
#[derive(Clone, Debug)]
pub struct TopologyRecord {
    /// Identity of the face under check.
    /// 受检注册面的身份。
    pub id: NodeId,
    /// Identity of the face this one registers under; the package root for a
    /// top-level face.
    /// 本注册面所挂载的父面身份；顶层注册面则为包根。
    pub parent: NodeId,
    /// Whether this face provides a registry its children may register into.
    /// 该注册面是否提供可供子级注册的注册表。
    pub owns_registry: bool,
    /// Source file label the diagnostic points back to.
    /// 诊断指回的源文件标签。
    pub source: String,
}

/// Sort `records` by identity, then check missing parents, parents that do
/// not own a registry, and parent cycles. Returns the collected diagnostics.
/// 将 `records` 按身份排序，然后检查缺失的父节点、父节点不持有注册表、
/// 以及父链成环；返回收集到的诊断。
pub fn validate_face_topology(
    records: &mut [TopologyRecord],
    package_root: NodeId,
) -> BuildDiagnostics {
    let mut errors = BuildDiagnostics::default();
    records.sort_by_key(|record| record.id);

    let ids = records
        .iter()
        .map(|record| record.id)
        .collect::<std::collections::BTreeSet<_>>();
    let owners = records
        .iter()
        .map(|record| (record.id, record.owns_registry))
        .collect::<std::collections::BTreeMap<_, _>>();
    for record in records.iter() {
        if record.parent != package_root && !ids.contains(&record.parent) {
            errors.push(
                BuildDiagnostic::new("static-plan", "parent node is missing")
                    .at(record.source.clone(), 0)
                    .field("parent")
                    .expected("registered parent")
                    .actual(record.parent.to_string()),
            );
        } else if record.parent != package_root && owners.get(&record.parent) == Some(&false) {
            errors.push(
                BuildDiagnostic::new("static-plan", "parent does not own a registry")
                    .at(record.source.clone(), 0)
                    .field("parent")
                    .expected("registry owner")
                    .actual(record.parent.to_string()),
            );
        }
    }

    let parents = records
        .iter()
        .map(|record| (record.id, record.parent))
        .collect::<std::collections::BTreeMap<_, _>>();
    for record in records.iter() {
        let mut current = record.id;
        let mut seen = std::collections::BTreeSet::new();
        while current != package_root {
            if !seen.insert(current) {
                errors.push(
                    BuildDiagnostic::new("static-plan", "parent cycle detected")
                        .at(record.source.clone(), 0)
                        .field("parent")
                        .actual(current.to_string()),
                );
                break;
            }
            let Some(parent) = parents.get(&current).copied() else {
                break;
            };
            current = parent;
        }
    }
    errors
}

#[cfg(test)]
mod topology_tests {
    use super::{TopologyRecord, validate_face_topology};
    use crate::registry_core::identity::{NodeId, ROOT_NODE_ID};

    fn record(id: u8, parent: u8, owns_registry: bool) -> TopologyRecord {
        TopologyRecord {
            id: NodeId::from_raw([id, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            parent: NodeId::from_raw([parent, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            owns_registry,
            source: format!("face-{id}.rs"),
        }
    }

    #[test]
    fn missing_parent_is_reported() {
        let mut records = vec![record(2, 9, true)];
        let errors = validate_face_topology(&mut records, ROOT_NODE_ID);
        assert_eq!(errors.render().matches("parent node is missing").count(), 1);
    }

    #[test]
    fn parent_without_registry_is_reported() {
        let mut records = vec![record(1, 0, false), record(2, 1, true)];
        let errors = validate_face_topology(&mut records, ROOT_NODE_ID);
        assert_eq!(
            errors
                .render()
                .matches("parent does not own a registry")
                .count(),
            1
        );
    }

    #[test]
    fn parent_cycle_is_reported() {
        let mut records = vec![record(1, 2, true), record(2, 1, true)];
        let errors = validate_face_topology(&mut records, ROOT_NODE_ID);
        assert_eq!(errors.render().matches("parent cycle detected").count(), 2);
    }

    #[test]
    fn valid_chain_is_clean() {
        let root = ROOT_NODE_ID;
        let mut records = vec![
            TopologyRecord {
                parent: root,
                ..record(1, 0, true)
            },
            TopologyRecord {
                parent: NodeId::from_raw([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                ..record(2, 0, true)
            },
        ];
        let errors = validate_face_topology(&mut records, root);
        assert!(errors.is_empty());
    }
}
