//! Generate the passive crate entry from the folder layout.
//! 根据文件夹布局生成被动 crate 入口。
//!
//! The build script mirrors the local folder layout and derives a conservative
//! registration-face scope before rustc sees the generated module tree. Distributed
//! registration is routed through the collector adapter; this file does not
//! maintain a registration roster.
//! build.rs 镜像本地文件夹布局，并在 rustc 看到生成模块树之前保守推导注册面范围。
//! 分布式注册经由 collector 适配层完成，本文件不维护注册清单。

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[allow(dead_code)]
#[path = "identity.rs"]
mod registry_identity;
#[allow(dead_code)]
#[path = "syntax.rs"]
mod registry_syntax;

use registry_identity::{NodeId, IDENTITY_SCHEMA};
use registry_syntax::{application_entries, source_references, FaceSyntax, ParentSyntax};

// Bump this whenever the identity input or generated-plan format changes.
// 身份输入或生成计划格式变化时必须递增，避免旧缓存混入新构建。
const CACHE_SCHEMA: &str = "3";

#[path = "contracts.rs"]
mod contracts;
#[path = "diagnostics.rs"]
mod diagnostics;
#[path = "discovery.rs"]
mod discovery;
#[path = "identity_cache.rs"]
mod identity_cache;
#[path = "manifests.rs"]
mod manifests;
#[path = "pipeline.rs"]
mod pipeline;
#[path = "registration_check.rs"]
mod registration_check;
#[path = "renderer.rs"]
mod renderer;
#[path = "static_plan.rs"]
mod static_plan;
#[path = "types.rs"]
mod types;
#[path = "validation.rs"]
mod validation;

use contracts::aggregate_contract_errors;
use discovery::{discover_root, discovery_fingerprint, emit_rerun_paths};
use identity_cache::{
    cache_directory, prime_node_id_cache, source_unit_fingerprint, valid_cached_unit,
};
use manifests::{write_function_manifest, write_pruning_manifest, write_source_scope_manifest};
use renderer::{materialize_sources, render_lib};
use static_plan::static_plan;
use types::{BuildInput, Node};
use validation::{aggregate_requirements, aggregate_stable_name_errors, parsed_face};

static CACHED_NODE_IDS: OnceLock<BTreeMap<String, (NodeId, String)>> = OnceLock::new();

/// First-pass source scope. It gates whole registration faces before rustc
/// expands them; functions inside an enabled face are intentionally untouched.
/// 第一次源码范围修剪。在 rustc 展开前按注册面整体门控；启用注册面内部的函数
/// 不在这里处理。
#[derive(Clone, Debug)]
struct SourceScope {
    roots: Option<BTreeSet<NodeId>>,
    reason: &'static str,
}

impl SourceScope {
    fn from_environment(src: &Path, nodes: &[Node]) -> Self {
        let Some(raw) = env::var_os("NICH_LINK_SCOPE") else {
            return Self::auto(src, nodes);
        };
        let raw = raw.to_string_lossy();
        let raw = if let Some((version, values)) = raw.split_once(':') {
            if version.strip_prefix('v') != Some(IDENTITY_SCHEMA) {
                panic!(
                    "NICH_LINK_SCOPE uses identity schema `{version}`, expected `v{IDENTITY_SCHEMA}`"
                );
            }
            values
        } else {
            raw.as_ref()
        };
        if raw.trim().is_empty() || raw.trim().eq_ignore_ascii_case("all") {
            return Self {
                roots: None,
                reason: "scope-all",
            };
        }
        if raw.trim().eq_ignore_ascii_case("auto") {
            return Self::auto(src, nodes);
        }
        let ids = raw
            .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
            .filter(|value| !value.is_empty())
            .map(|value| {
                value.parse::<NodeId>().unwrap_or_else(|_| {
                    panic!("NICH_LINK_SCOPE entry `{value}` is not a 32-digit node identity")
                })
            })
            .collect::<BTreeSet<_>>();
        let mut known = BTreeSet::new();
        collect_node_ids(src, nodes, &mut known);
        let unknown = ids
            .difference(&known)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if !unknown.is_empty() {
            panic!(
                "NICH_LINK_SCOPE contains unknown node identity(s): {}",
                unknown.join(", ")
            );
        }
        Self {
            roots: Some(ids),
            reason: "explicit",
        }
    }

    /// Derive a conservative face scope from the package entry source.
    /// 从包入口源码保守推导注册面作用域。
    ///
    /// This lives in the build step so the same parser and the same node identity
    /// implementation are used for discovery and identity. A source scan is
    /// not a complete rustc call graph; when no face can be proven, or when
    /// dynamic/generated code is present, the safe result is the full tree.
    /// 逻辑放在构建步骤中，发现和身份计算共用同一套解析与 node identity 实现。源码扫描
    /// 不是完整 rustc 调用图；无法证明注册面或发现动态/生成代码时，安全结果是全树。
    fn auto(src: &Path, nodes: &[Node]) -> Self {
        let entry = env::var_os("NICH_LINK_ENTRY")
            .map(PathBuf::from)
            .map_or_else(
                || {
                    application_entry_source(src, nodes)
                        .unwrap_or_else(|| default_entry_source(src))
                },
                |path| {
                    if path.is_absolute() {
                        path
                    } else {
                        src.parent().unwrap_or(src).join(path)
                    }
                },
            );
        let Some(entry_source) = fs::read_to_string(&entry).ok() else {
            return Self {
                roots: None,
                reason: "missing-entry",
            };
        };
        let faces = collect_faces(src, nodes);
        if faces.is_empty() {
            return Self {
                roots: None,
                reason: "no-registration-face",
            };
        }
        let mut selected = BTreeSet::new();
        let mut queue = vec![entry_source];
        let mut scanned = BTreeSet::new();
        let mut uncertain = false;
        while let Some(source) = queue.pop() {
            if !scanned.insert(source.clone()) {
                continue;
            }
            let Ok(references) = source_references(&source) else {
                return Self {
                    roots: None,
                    reason: "unparseable-or-dynamic-source",
                };
            };
            uncertain |= references.conservative;
            for path in &references.paths {
                let deepest = faces
                    .iter()
                    .filter(|face| path_mentions_module(path, &face.module))
                    .map(|face| face.module.len())
                    .max();
                for face in faces.iter().filter(|face| {
                    deepest == Some(face.module.len()) && path_mentions_module(path, &face.module)
                }) {
                    if selected.insert(face.id) {
                        if let Ok(next) = fs::read_to_string(&face.source) {
                            queue.push(next);
                        }
                    }
                }
            }
        }
        if selected.is_empty() || uncertain {
            return Self {
                roots: None,
                reason: "conservative-fallback",
            };
        }
        Self {
            roots: Some(selected),
            reason: "auto",
        }
    }

    fn includes(&self, src: &Path, node: &Node, selected_ancestor: bool) -> bool {
        if node.name == "registry_core"
            || node
                .file
                .as_ref()
                .is_some_and(|file| relative_display(src, file).starts_with("registry_core/"))
            || node.name == "registry"
            || node.name == "rules"
            || node.name == "registry_rule"
            || node.name == "root_registry"
        {
            return true;
        }
        let Some(roots) = &self.roots else {
            return true;
        };
        let selected_here = node_id(src, node).is_some_and(|id| roots.contains(&id));
        selected_ancestor
            || selected_here
            || node
                .children
                .iter()
                .any(|child| self.includes(src, child, selected_ancestor || selected_here))
    }
}

/// Pick the ordinary Cargo entry when no `application!` declaration exists.
/// 没有 `application!` 声明时，按 Cargo 约定选择默认入口。
fn default_entry_source(src: &Path) -> PathBuf {
    let main = src.join("main.rs");
    if main.is_file() {
        return main;
    }
    let lib = src.join("lib.rs");
    if lib.is_file() {
        return lib;
    }
    // Keep the existing missing-entry diagnostic and conservative fallback.
    // 保留原有 missing-entry 诊断，并继续使用保守的全树回退。
    main
}

fn application_entry_source(src: &Path, nodes: &[Node]) -> Option<PathBuf> {
    let mut declarations = Vec::new();
    fn visit(nodes: &[Node], declarations: &mut Vec<(PathBuf, String, usize, usize)>) {
        for node in nodes {
            if let Some(file) = &node.file {
                let source = fs::read_to_string(file).unwrap_or_else(|error| {
                    panic!(
                        "failed to read application entry source `{}`: {error}",
                        file.display()
                    )
                });
                let entries = application_entries(&source).unwrap_or_else(|error| {
                    panic!(
                        "invalid application! declaration in `{}`: {error}",
                        file.display()
                    )
                });
                for (path, location) in entries {
                    declarations.push((file.clone(), path, location.line, location.column));
                }
            }
            visit(&node.children, declarations);
        }
    }
    visit(nodes, &mut declarations);
    match declarations.as_slice() {
        [] => None,
        [(file, path, _, _)] => {
            if !path.starts_with("crate::") {
                panic!("application! entry `{path}` must start with `crate::`");
            }
            if !entry_path_exists(src, path) {
                panic!(
                    "application! entry `{path}` does not resolve to a source module under `{}`",
                    src.display()
                );
            }
            Some(file.clone())
        }
        many => {
            let details = many
                .iter()
                .map(|(file, path, line, column)| {
                    format!("{path} at {}:{line}:{column}", file.display())
                })
                .collect::<Vec<_>>()
                .join(", ");
            panic!("multiple application! entry declarations found: {details}");
        }
    }
}

fn entry_path_exists(src: &Path, path: &str) -> bool {
    let mut segments = path
        .strip_prefix("crate::")
        .unwrap_or_default()
        .split("::")
        .filter(|segment| !segment.is_empty());
    let Some(first) = segments.next() else {
        return false;
    };
    let module = src.join(first);
    module.with_extension("rs").is_file()
        || module.join("mod.rs").is_file()
        || src.join("bin").join(first).with_extension("rs").is_file()
        || (first == "main" && src.join("lib.rs").is_file())
}

fn path_mentions_module(path: &str, module: &str) -> bool {
    let path = path
        .strip_prefix("crate::")
        .or_else(|| path.strip_prefix("nichlink::"))
        .unwrap_or(path);
    path == module || path.starts_with(&format!("{module}::"))
}

#[derive(Clone, Debug)]
struct FaceSource {
    id: NodeId,
    source: PathBuf,
    module: String,
}

fn collect_faces(src: &Path, nodes: &[Node]) -> Vec<FaceSource> {
    let mut faces = Vec::new();
    collect_faces_inner(src, nodes, &mut faces);
    faces
}

fn collect_faces_inner(src: &Path, nodes: &[Node], faces: &mut Vec<FaceSource>) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if !relative.starts_with("registry_core/") {
                let cached = CACHED_NODE_IDS.get().and_then(|cache| cache.get(&relative));
                let parsed = cached.is_none().then(|| {
                    fs::read_to_string(file)
                        .ok()
                        .and_then(|source| parsed_face(&source, &relative))
                        .and_then(|face| face.path("kind"))
                });
                let (id, kind) = cached
                    .map(|(id, kind)| (*id, kind.clone()))
                    .or_else(|| {
                        parsed.flatten().map(|kind| {
                            (registry_identity::package_node_id(&relative, &kind), kind)
                        })
                    })
                    .unwrap_or_else(|| {
                        (
                            registry_identity::package_node_id(&relative, ""),
                            String::new(),
                        )
                    });
                if !kind.is_empty() {
                    let module = relative
                        .rsplit_once('/')
                        .map_or_else(|| relative.trim_end_matches(".rs"), |(parent, _)| parent)
                        .replace('/', "::");
                    faces.push(FaceSource {
                        id,
                        source: file.clone(),
                        module,
                    });
                }
            }
        }
        collect_faces_inner(src, &node.children, faces);
    }
}

fn has_selected_face(
    src: &Path,
    node: &Node,
    selected_ancestor: bool,
    scope: &SourceScope,
) -> bool {
    if scope.roots.is_none() || selected_ancestor {
        return true;
    }
    if node_id(src, node).is_some_and(|id| {
        scope
            .roots
            .as_ref()
            .is_some_and(|roots| roots.contains(&id))
    }) {
        return true;
    }
    node.children
        .iter()
        .any(|child| has_selected_face(src, child, false, scope))
}

fn node_id(src: &Path, node: &Node) -> Option<NodeId> {
    let file = node.file.as_ref()?;
    let relative = relative_display(src, file);
    // Registry infrastructure defines the macros and support types; it is not
    // a user registration face and must never enter the face discovery pass.
    // 注册基础设施只提供宏和支撑类型，不是用户注册面，不能进入注册面发现。
    if relative.starts_with("registry_core/") {
        return None;
    }
    if let Some((id, _)) = CACHED_NODE_IDS.get().and_then(|cache| cache.get(&relative)) {
        return Some(*id);
    }
    let source = fs::read_to_string(file).ok()?;
    let kind = parsed_face(&source, &relative)?.path("kind")?;
    Some(registry_identity::package_node_id(&relative, &kind))
}

fn collect_node_ids(src: &Path, nodes: &[Node], ids: &mut BTreeSet<NodeId>) {
    for node in nodes {
        if let Some(id) = node_id(src, node) {
            ids.insert(id);
        }
        collect_node_ids(src, &node.children, ids);
    }
}

fn collect_active_ids(
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
        if face_source_is_active(src, node, scope, selected_ancestor) {
            if let Some(id) = node_id(src, node) {
                active.insert(id);
            }
        }
        collect_active_ids(src, &node.children, scope, selected_here, active);
    }
}

pub fn run() {
    let input = BuildInput::from_environment();
    pipeline::run(&input);
}

/// Keep a reusable discovery snapshot outside Cargo's ephemeral OUT_DIR.
/// 将发现结果保存在 Cargo 临时 OUT_DIR 之外，供后续构建复用。
///
/// This is only an observation cache. Generated Rust still comes from the
/// current source tree, and a corrupt cache is rebuilt instead of trusted.
/// 这只是观察缓存；生成的 Rust 仍来自当前源码树，损坏缓存会重建而不会被信任。
fn update_discovery_cache(
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
                for name in [
                    "parts",
                    "exports",
                    "handle_traits",
                    "part_traits",
                    "expected_output",
                    "actual_output",
                ] {
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

fn cached_parent_id(src: &Path, face: &FaceSyntax) -> Option<NodeId> {
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
            let parent_source = fs::read_to_string(src.join(&relative_path)).ok()?;
            let parent = parsed_face(&parent_source, &relative_path.to_string_lossy())?;
            let kind = parent.path("kind")?;
            Some(registry_identity::package_node_id(
                &relative_path.to_string_lossy(),
                &kind,
            ))
        }
    }
}

fn collect_discovery_rows(src: &Path, nodes: &[Node], rows: &mut Vec<(String, NodeId)>) {
    for node in nodes {
        if let Some(file) = &node.file {
            if let Some(id) = node_id(src, node) {
                rows.push((relative_display(src, file), id));
            }
        }
        collect_discovery_rows(src, &node.children, rows);
    }
}

fn source_is_active(src: &Path, node: &Node, scope: &SourceScope, selected_ancestor: bool) -> bool {
    scope.roots.is_none()
        || selected_ancestor
        || (node_id(src, node).is_some() && has_selected_face(src, node, false, scope))
        || node
            .file
            .as_ref()
            .is_some_and(|file| relative_display(src, file).starts_with("registry_core/"))
        || matches!(
            node.name.as_str(),
            "registry" | "rules" | "registry_rule" | "root_registry"
        )
}

fn face_source_is_active(
    src: &Path,
    node: &Node,
    scope: &SourceScope,
    selected_ancestor: bool,
) -> bool {
    node_id(src, node).is_some() && source_is_active(src, node, scope, selected_ancestor)
}

/// Keep development tooling outside the default core dependency graph.
/// 将开发工具隔离在默认核心依赖图之外。
fn module_feature(src: &Path, node: &Node) -> Option<&'static str> {
    let relative = node.file.as_ref().map(|file| relative_display(src, file))?;
    match relative.as_str() {
        "registry_core/authoring/authoring.rs" | "registry_core/syntax/syntax.rs" => {
            Some("authoring")
        }
        "registry_core/adapters/adapters.rs" | "registry_core/mir/mir.rs" => Some("observability"),
        _ => None,
    }
}

fn relative_display(src: &Path, path: &Path) -> String {
    path.strip_prefix(src)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn write_if_changed(path: &Path, content: &str) {
    if fs::read_to_string(path).ok().as_deref() != Some(content) {
        fs::write(path, content).expect("write generated module tree");
    }
}
