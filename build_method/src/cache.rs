//! Discovery/identity cache writing and the active-face predicates it shares.
//! 发现/身份缓存写入及其复用的活跃注册面判定。
//!
//! The cache is only an observation record: generated Rust still comes from the
//! current source tree, and a corrupt cache is rebuilt instead of trusted. The
//! activity predicates live beside it because they answer the same question the
//! cache rows are keyed on.
//! 缓存只是观察记录：生成的 Rust 仍来自当前源码树，损坏缓存会重建而不会被信任。
//! 活跃判定与缓存放在一起，因为它回答的正是缓存行所依据的同一个问题。

use std::collections::BTreeSet;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use nichlink::lexicon;

use super::identity_cache::{source_unit_fingerprint, valid_cached_unit};
use super::node::{Node, relative_display};
use super::registry_identity::{self, NodeId};
use super::registry_syntax::{FaceSyntax, ParentSyntax};
use super::validation::parsed_face;
use super::{SourceScope, has_selected_face, node_id};

// Bump this whenever the identity input or generated-plan format changes.
// 身份输入或生成计划格式变化时必须递增，避免旧缓存混入新构建。
//
// This is not the identity schema: it versions the discovery cache's own layout,
// and the two change for different reasons.
// 这不是身份 schema：它标记发现缓存自身的版式，两者因不同的理由而变化。
pub(crate) const CACHE_SCHEMA: &str = "3";

/// Write a generated artifact only when its bytes changed.
/// 仅在内容变化时写入生成产物。
pub(crate) fn write_if_changed(path: &Path, content: &str) {
    if fs::read_to_string(path).ok().as_deref() != Some(content) {
        fs::write(path, content).expect("write generated module tree");
    }
}

pub(crate) fn collect_active_ids(
    src: &Path,
    nodes: &[Node],
    scope: &SourceScope,
    selected_ancestor: bool,
    active: &mut BTreeSet<NodeId>,
) {
    for node in nodes {
        if !scope.includes(src, node, selected_ancestor) {
            continue;
        }
        let selected_here = selected_ancestor
            || node_id(src, node).is_some_and(|id| {
                scope
                    .roots
                    .as_ref()
                    .is_some_and(|roots| roots.contains(&id))
            });
        if face_source_is_active(src, node, scope, selected_ancestor)
            && let Some(id) = node_id(src, node)
        {
            active.insert(id);
        }
        collect_active_ids(src, &node.children, scope, selected_here, active);
    }
}

/// Keep a reusable discovery snapshot outside Cargo's ephemeral OUT_DIR.
/// 将发现结果保存在 Cargo 临时 OUT_DIR 之外，供后续构建复用。
///
/// This is only an observation cache. Generated Rust still comes from the
/// current source tree, and a corrupt cache is rebuilt instead of trusted.
/// 这只是观察缓存；生成的 Rust 仍来自当前源码树，损坏缓存会重建而不会被信任。
pub(crate) fn update_discovery_cache(
    manifest: &Path,
    src: &Path,
    nodes: &[Node],
    fingerprint: &str,
) -> String {
    let target = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map_or_else(
            || manifest.join("target"),
            |path| {
                if path.is_absolute() {
                    path
                } else {
                    manifest.join(path)
                }
            },
        );
    let directory = target.join("nichlink").join("cache");
    if fs::create_dir_all(&directory).is_err() {
        return "disabled".to_owned();
    }
    let units = directory.join("units");
    if fs::create_dir_all(&units).is_err() {
        return "disabled".to_owned();
    }
    let mut content =
        format!("# schema\t{CACHE_SCHEMA}\n# fingerprint\t{fingerprint}\n# path\tnode\tunit\n");
    let mut rows = Vec::new();
    collect_discovery_rows(src, nodes, &mut rows);
    rows.sort();
    rows.dedup();
    let mut all_units_hit = true;
    let mut unit_hits = 0_usize;
    let mut unit_misses = 0_usize;
    for (path, id) in rows {
        let source_path = src.join(&path);
        let unit_fingerprint = source_unit_fingerprint(&source_path);
        let unit_path = units.join(format!("{unit_fingerprint}.tsv"));
        let cached = fs::read_to_string(&unit_path).ok();
        let unit_hit = cached
            .as_deref()
            .is_some_and(|content| valid_cached_unit(content, &path, id));
        if !unit_hit {
            // Parse only a new or corrupt unit. A valid fingerprinted unit is
            // consumed as-is by registration_check::aggregate.
            // 只有新 unit 或损坏 unit 才解析；有效 fingerprint unit 直接由
            // registration_check::aggregate 消费。
            let source = fs::read_to_string(&source_path).ok();
            let face = source
                .as_deref()
                .and_then(|source| parsed_face(source, &path));
            let kind = face
                .as_ref()
                .and_then(|face| face.path("kind"))
                .unwrap_or_default();
            let mut unit_content = format!(
                "# schema\t{CACHE_SCHEMA}\n# fingerprint\t{unit_fingerprint}\npath\t{path}\nnode\t{id}\nkind\t{kind}\n"
            );
            if let Some(face) = &face {
                if let Some(handle) = face.path("handle") {
                    writeln!(unit_content, "handle\t{handle}").unwrap();
                }
                let parent_id = cached_parent_id(src, face);
                if let Some(parent_id) = parent_id {
                    writeln!(unit_content, "parent\t{parent_id}").unwrap();
                }
                let line = face
                    .field_location("requires")
                    .map_or(face.location.line, |location| location.line);
                writeln!(unit_content, "line\t{line}").unwrap();
                for (capability, provider) in face.requirements("requires").unwrap_or_default() {
                    writeln!(unit_content, "requires\t{capability}\t{provider}").unwrap();
                }
                for provide in face.string_list("provides").unwrap_or_default() {
                    writeln!(unit_content, "provides\t{provide}").unwrap();
                }
                for name in ["parts", "exports", "handle_traits", "part_traits"] {
                    if let Some(value) = face.field(name) {
                        writeln!(unit_content, "contract\t{name}\t{value}").unwrap();
                    }
                }
            }
            all_units_hit = false;
            unit_misses += 1;
            if fs::write(&unit_path, unit_content).is_err() {
                return "disabled".to_owned();
            }
        } else {
            unit_hits += 1;
        }
        writeln!(content, "{path}\t{id}\t{unit_fingerprint}").unwrap();
    }
    let path = directory.join(format!("discovery-{fingerprint}.tsv"));
    if fs::read_to_string(&path).ok().as_deref() == Some(content.as_str()) {
        if all_units_hit {
            format!("hit (units={unit_hits})")
        } else {
            format!("miss (hits={unit_hits}, misses={unit_misses})")
        }
    } else if fs::write(path, content).is_ok() {
        format!("miss (hits={unit_hits}, misses={unit_misses}, discovery=changed)")
    } else {
        "disabled".to_owned()
    }
}

pub(crate) fn cached_parent_id(src: &Path, face: &FaceSyntax) -> Option<NodeId> {
    match face.parent()? {
        ParentSyntax::Root => Some(registry_identity::package_root_node_id()),
        ParentSyntax::FromPath { source, kind } => {
            Some(registry_identity::package_node_id(&source, &kind))
        }
        ParentSyntax::NodePath(module) => {
            let module = module.strip_prefix("crate::")?;
            let mut relative_path = module.split("::").collect::<PathBuf>();
            let name = relative_path.file_name()?.to_owned();
            relative_path.push(format!("{}.rs", name.to_string_lossy()));
            let parent_path = src.join(&relative_path);
            let relative = relative_display(src, &parent_path);
            let parent_source = fs::read_to_string(parent_path).ok()?;
            let parent = parsed_face(&parent_source, &relative)?;
            let kind = parent.path("kind")?;
            Some(registry_identity::package_node_id(&relative, &kind))
        }
    }
}

fn collect_discovery_rows(src: &Path, nodes: &[Node], rows: &mut Vec<(String, NodeId)>) {
    for node in nodes {
        if let Some(file) = &node.file
            && let Some(id) = node_id(src, node)
        {
            rows.push((relative_display(src, file), id));
        }
        collect_discovery_rows(src, &node.children, rows);
    }
}

pub(crate) fn source_is_active(
    src: &Path,
    node: &Node,
    scope: &SourceScope,
    selected_ancestor: bool,
) -> bool {
    scope.roots.is_none()
        || selected_ancestor
        || (node_id(src, node).is_some() && has_selected_face(src, node, false, scope))
        || node
            .file
            .as_ref()
            .is_some_and(|file| lexicon::is_registration_path(&relative_display(src, file)))
        || matches!(
            node.name.as_str(),
            "registry" | "rules" | "registry_rule" | "root_registry"
        )
}

pub(crate) fn face_source_is_active(
    src: &Path,
    node: &Node,
    scope: &SourceScope,
    selected_ancestor: bool,
) -> bool {
    node_id(src, node).is_some() && source_is_active(src, node, scope, selected_ancestor)
}

/// Keep development tooling outside the default core dependency graph.
/// 将开发工具隔离在默认核心依赖图之外。
pub(crate) fn module_feature(src: &Path, node: &Node) -> Option<&'static str> {
    let relative = node.file.as_ref().map(|file| relative_display(src, file))?;
    match relative.as_str() {
        "registry_core/authoring/authoring.rs" | "registry_core/syntax/syntax.rs" => {
            Some("authoring")
        }
        _ => None,
    }
}
