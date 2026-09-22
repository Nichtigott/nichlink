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

#[path = "identity.rs"]
mod registry_identity;
#[path = "syntax.rs"]
mod registry_syntax;

use registry_identity::{IDENTITY_SCHEMA, NodeId};
use registry_syntax::{
    FaceSyntax, GraftSyntax, ParentSyntax, application_entries, graft_entries, source_references,
};

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
#[path = "scaffold.rs"]
pub mod scaffold;
#[path = "static_plan.rs"]
mod static_plan;
#[path = "types.rs"]
mod types;
#[path = "validation.rs"]
mod validation;

/// The module path a registration source declares, for authoring surfaces.
/// 注册面源码声明的模块路径，供创作界面使用。
pub use static_plan::source_module_path;

use contracts::aggregate_contract_errors;
use discovery::{discover_root, discovery_fingerprint, emit_rerun_paths};
use identity_cache::{
    cache_directory, prime_node_id_cache, source_unit_fingerprint, valid_cached_unit,
};
use manifests::{
    write_function_manifest, write_graft_manifest, write_pruning_manifest,
    write_source_scope_manifest,
};
use renderer::render_lib;
use static_plan::static_plan;
use types::{BuildInput, Node};
use validation::{
    aggregate_parent_macro_errors, aggregate_requirements, aggregate_stable_name_errors,
    parsed_face,
};

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
        let entry = Self::resolved_entry(src, nodes);
        Self::auto_from_entry(src, nodes, &entry)
    }

    /// Resolve the host entry: `NICH_LINK_ENTRY` names it when set (a relative
    /// path resolves against the package root), otherwise the file that calls
    /// `host!()` wins over Cargo's `main.rs` preference.
    /// 解析宿主入口：设置 `NICH_LINK_ENTRY` 时以它为准（相对路径相对包根解析），
    /// 否则调用 `host!()` 的文件优先于 Cargo 对 `main.rs` 的偏好。
    fn resolved_entry(src: &Path, nodes: &[Node]) -> PathBuf {
        env::var_os("NICH_LINK_ENTRY")
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
            )
    }

    /// Derive the scope from one explicit entry source, without reading the
    /// environment, so the derivation can be pinned by a test.
    /// 从一个明确的入口源码推导作用域，不读环境变量，因此推导过程可以被测试钉住。
    fn auto_from_entry(src: &Path, nodes: &[Node], entry: &Path) -> Self {
        let Some(entry_source) = fs::read_to_string(entry).ok() else {
            return Self {
                roots: None,
                reason: "missing-entry",
            };
        };
        // Graft declarations belong to the host entry. Parse them here so a
        // malformed overlay fails at build time; the plan itself is applied by
        // the host against an external Registry at runtime or release setup.
        let cuts = graft_entries(&entry_source).unwrap_or_else(|error| {
            panic!(
                "invalid graft declaration in `{}`: {error}",
                entry.display()
            )
        });
        let faces = collect_faces(src, nodes);
        if faces.is_empty() {
            return Self {
                roots: None,
                reason: "no-registration-face",
            };
        }
        let mut selected = BTreeSet::new();
        let mut queue = vec![entry_source];
        // Forced liveness roots. Graft slots and plugin-declaring faces are
        // replaceable community surface: a minimal tree must never prune
        // them just because no reachable source mentions them, or the graft
        // plan loses its cut target at release setup.
        // 强制存活根：嫁接槽位与插件声明面是可替换的社区面，
        // minimal 树绝不能因可达源码未提及就把它们剪掉。
        for cut in &cuts {
            // A declared slot is a face the host lets someone else replace, and
            // the scope keeps exactly the subtrees the cuts name plus the plugin
            // faces below. A typed cut names its target with a Rust path to a
            // real face, so its module is matched exactly instead of guessed from
            // a registry path; a string cut names the same face by registry path.
            // Both spellings narrow, and they must narrow identically: the
            // spelling is not allowed to decide the policy. What a cut may never
            // do is narrow by accident, so an endpoint the build cannot place
            // keeps the whole tree and says why.
            // 声明出来的槽位就是宿主允许别人替换的注册面，作用域保留的正是切口命名的子树
            // （外加下面的插件面）。类型化切口用指向真实注册面的 Rust 路径命名目标，因此
            // 精确匹配模块、不必从注册路径猜测；字符串切口用注册路径命名同一个面。
            // 两种写法都收窄，而且必须收窄到同一结果：写法不允许决定策略。切口绝不允许
            // 碰巧收窄，因此构建定位不到的端点保留整棵树并说明原因。
            //
            // A range cut names two targets, and both have to stay live: the
            // overlay replaces everything between them. The string form used to
            // hand the whole `"start to end"` text to the module mapper, so the
            // module name contained a space, matched no face, and the cut pinned
            // nothing — leaving its targets to be pruned and the overlay to fail
            // with `UnknownTarget` later.
            // 区间切口命名两个目标，两者都必须保持存活：覆盖层替换它们之间的全部内容。
            // 字符串形式过去把整段 `"start to end"` 交给模块映射，于是模块名里带空格、
            // 匹配不到任何面、切口什么也没钉住——目标随后被剪掉，overlay 再以
            // `UnknownTarget` 失败。
            let resolved = match &cut.expressions {
                Some(expressions) => {
                    let cut_end = expressions.cut_end.as_deref();
                    let module = graft_expression_module(&expressions.cut, &faces);
                    let end_module = cut_end.and_then(|end| graft_expression_module(end, &faces));
                    match (module, cut_end.is_some() && end_module.is_none()) {
                        (Some(module), false) => Ok((module, end_module)),
                        // A typed cut names a real face, so a path the build
                        // cannot place is an unreadable declaration, not a
                        // smaller tree.
                        // 类型化切口命名真实注册面，因此构建定位不到的路径是读不懂的
                        // 声明，而不是一棵更小的树。
                        _ => Err("graft-typed-cut-unrecognized"),
                    }
                }
                None => match string_cut_modules(&cut.cut) {
                    // A range whose far endpoint mapped to the whole tree is not
                    // understood, so the whole tree is kept rather than pruned
                    // wrongly.
                    // 区间切口的远端映射到整棵树，说明这个区间没被读懂，因此保留整棵树
                    // 而不是错剪。
                    (Some(module), end) if !(cut.cut.contains(" to ") && end.is_none()) => {
                        Ok((module, end))
                    }
                    _ => Err("graft-root-cut"),
                },
            };
            let (module, end_module) = match resolved {
                Ok(pair) => pair,
                Err(reason) => {
                    return Self {
                        roots: None,
                        reason,
                    };
                }
            };
            queue.extend(select_module_subtree(&faces, &module, &mut selected));
            if let Some(end_module) = end_module {
                queue.extend(select_module_subtree(&faces, &end_module, &mut selected));
            }
        }
        for face in &faces {
            if face_declares_plugin(src, face)
                && selected.insert(face.id)
                && let Ok(next) = fs::read_to_string(&face.source)
            {
                queue.push(next);
            }
        }
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
                    if selected.insert(face.id)
                        && let Ok(next) = fs::read_to_string(&face.source)
                    {
                        queue.push(next);
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
    let candidates = [src.join("main.rs"), src.join("lib.rs")];
    // Cargo prefers `main.rs`, but a lib+bin host calls `host!()` in whichever
    // file owns the generated tree — often `lib.rs`, with `main.rs` left as a
    // stub. An `application!` declaration is not the only thing that can live
    // there: the declared graft plan does too, and picking the stub silently
    // dropped it. The entry is the file that calls `host!()`; Cargo's order only
    // breaks the tie.
    // Cargo 偏好 `main.rs`，但库+二进制宿主会在拥有生成树的那个文件里调用
    // `host!()`——常见情形是 `lib.rs`，而 `main.rs` 只是个空壳。那里不只可能放
    // `application!` 声明，声明的 graft 计划也在其中，选中空壳会把它静默丢掉。
    // 入口应当是调用 `host!()` 的文件；Cargo 的顺序只用来打破平局。
    if let Some(entry) = candidates.iter().find(|path| calls_host(path)) {
        return entry.clone();
    }
    let main = candidates[0].clone();
    if main.is_file() {
        return main;
    }
    let lib = candidates[1].clone();
    if lib.is_file() {
        return lib;
    }
    // Keep the existing missing-entry diagnostic and conservative fallback.
    // 保留原有 missing-entry 诊断，并继续使用保守的全树回退。
    main
}

/// Whether a source file declares this crate as a host.
/// 源文件是否把本 crate 声明为宿主。
///
/// Line-based on purpose: it only has to tell a stub entry from the real one, and
/// a leading `//` is enough to skip the comment that mentions `host!()` in every
/// documented face. A `/* … */` block comment would still count, which is the
/// conservative direction for choosing an entry.
/// 刻意按行判断：它只需把空壳入口与真正的入口区分开，而跳过以 `//` 开头的行就足以
/// 排除每个带文档的注册面里提到 `host!()` 的注释。`/* … */` 块注释仍会被算作命中，
/// 这对"选择入口"而言是保守方向。
fn calls_host(path: &Path) -> bool {
    fs::read_to_string(path).is_ok_and(|source| {
        source.lines().any(|line| {
            let line = line.trim();
            !line.starts_with("//") && line.contains("host!()")
        })
    })
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

/// Read the host entry's graft declarations once for generated release metadata.
/// 读取宿主入口中的 graft 声明，并生成正式构建可携带的静态选择器表。
pub(crate) fn host_graft_entries(src: &Path, nodes: &[Node]) -> Vec<GraftSyntax> {
    // Same entry resolution as `SourceScope::auto`: an explicit
    // `application!(entry = ...)` wins, otherwise the ordinary Cargo entry is
    // the host entry. Requiring `application!` here would silently drop a plan
    // declared where the documentation says to put it.
    // 与 `SourceScope::auto` 使用同一套入口解析：显式 `application!(entry = ...)`
    // 优先，否则按 Cargo 约定取普通入口。这里若强制要求 `application!`，就会
    // 静默丢弃按文档写在宿主入口的计划。
    let declared = application_entry_source(src, nodes);
    let file = match &declared {
        Some(file) => file.clone(),
        None => default_entry_source(src),
    };
    let Ok(source) = fs::read_to_string(&file) else {
        // A declared entry must exist, so failing to read it is an error. The
        // default entry may simply be absent (a host without `src/main.rs` or
        // `src/lib.rs`), which `SourceScope` already treats as "no scope".
        // 声明的入口必须存在，读不到即是错误；默认入口可能本来就不存在
        // （宿主既无 `src/main.rs` 也无 `src/lib.rs`），`SourceScope` 已把这种
        // 情况视为"无作用域"。
        if declared.is_some() {
            panic!("failed to read graft entry source `{}`", file.display());
        }
        return Vec::new();
    };
    let entries = graft_entries(&source).unwrap_or_else(|error| {
        panic!("invalid graft declaration in `{}`: {error}", file.display())
    });
    // The same gate rule the static plan follows: a feature the build can see
    // decides, and a gate it cannot evaluate is refused rather than guessed.
    // 与静态计划同一条规则：构建看得见的特性说了算，无法求值的门控一律拒绝而非猜测。
    entries
        .into_iter()
        .filter(|entry| match entry.cfg.as_deref() {
            Some(cfg) => match crate::static_plan::face_cfg_enabled(
                cfg,
                &crate::static_plan::feature_enabled,
            ) {
                Ok(enabled) => enabled,
                Err(message) => panic!("{message} in `{}`", file.display()),
            },
            None => true,
        })
        .collect()
}

/// One graft declaration the build step found in the host entry.
/// 构建步骤在宿主入口里发现的一条 graft 声明。
///
/// This is the authoring view of the same declaration the build captures into
/// the static plan: it exists so a tool can tell an author whether the slot
/// they are about to graft is shipped, without re-implementing the entry rule.
/// 这是构建会捕获进静态计划的那条声明的创作视图：它存在的意义是让工具无需重新
/// 实现入口规则，就能告诉作者准备嫁接的槽位会不会被发布。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredGraft {
    /// Logical path, or the Rust expression text of a typed cut.
    /// 逻辑路径，或类型化切口的 Rust 表达式原文。
    pub cut: String,
    pub graft: String,
    pub full: bool,
    /// The `cfg` gate the declaration carries, exactly as written.
    /// 声明携带的 `cfg` 门控（若有），按原文保留。
    pub cfg: Option<String>,
    /// Rust expressions, when the cut was written in the typed form.
    /// 切口写成类型化形式时的 Rust 表达式。
    pub expressions: Option<DeclaredGraftExpressions>,
    /// 1-based line of the declaration in the entry.
    /// 声明在入口中的 1 起始行号。
    pub line: usize,
}

/// Rust expressions used by one typed graft cut, in source order.
/// 一条类型化 graft 切口使用的 Rust 表达式，按源码顺序。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredGraftExpressions {
    pub cut: String,
    pub cut_end: Option<String>,
    pub graft: String,
}

/// The graft declarations one package's host entry declares.
/// 一个包的宿主入口声明的 graft 切口。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredGrafts {
    /// The entry the build step would read.
    /// 构建步骤会读取的入口。
    pub entry: PathBuf,
    pub cuts: Vec<DeclaredGraft>,
}

/// Resolve the host entry the build step reads graft declarations from.
/// 解析构建步骤读取 graft 声明的宿主入口。
///
/// Resolution order matches the build: `NICH_LINK_ENTRY`, then a file declaring
/// `application!(entry = …)`, then the file that calls `host!()`, then Cargo's
/// `main.rs`/`lib.rs`. Unlike the build, a source tree it cannot represent is
/// an `Err`, never a panic: an authoring surface has to stay alive to say so.
/// 解析顺序与构建一致：`NICH_LINK_ENTRY`、声明 `application!(entry = …)` 的文件、
/// 调用 `host!()` 的文件、最后按 Cargo 的 `main.rs`/`lib.rs` 约定。与构建不同的是，
/// 无法表示的源码树返回 `Err` 而不是 panic：创作界面必须活着把问题说出来。
pub fn host_entry_source(root: &Path) -> Result<PathBuf, String> {
    let src = root.join("src");
    if !src.is_dir() {
        return Err(format!("no source tree at {}", src.display()));
    }
    if let Some(configured) = env::var_os("NICH_LINK_ENTRY") {
        let path = PathBuf::from(configured);
        let path = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        if !path.is_file() {
            return Err(format!(
                "NICH_LINK_ENTRY names `{}`, which is not a file",
                path.display()
            ));
        }
        return Ok(path);
    }
    if let Some(entry) = declaring_application_entry(&src)? {
        return Ok(entry);
    }
    let entry = default_entry_source(&src);
    if !entry.is_file() {
        return Err(format!(
            "no host entry at {}; expected a source file that calls `host!()`",
            entry.display()
        ));
    }
    Ok(entry)
}

/// Read the host entry and return the graft declarations it carries.
/// 读取宿主入口并返回它携带的 graft 声明。
///
/// Feature gates are reported, not evaluated: the authoring surface shows the
/// gate so the author can see that a feature decides, and the build remains the
/// only place that follows it.
/// 特性门控只上报、不求值：创作界面显示门控让作者知道"由特性决定"，而跟随门控
/// 始终只发生在构建里。
pub fn declared_grafts(root: &Path) -> Result<DeclaredGrafts, String> {
    let entry = host_entry_source(root)?;
    let source = fs::read_to_string(&entry)
        .map_err(|error| format!("cannot read host entry {}: {error}", entry.display()))?;
    let entries = graft_entries(&source)
        .map_err(|error| format!("invalid graft declaration in {}: {error}", entry.display()))?;
    Ok(DeclaredGrafts {
        entry,
        cuts: entries
            .into_iter()
            .map(|entry| DeclaredGraft {
                cut: entry.cut,
                graft: entry.graft,
                full: entry.full,
                cfg: entry.cfg,
                expressions: entry
                    .expressions
                    .map(|expressions| DeclaredGraftExpressions {
                        cut: expressions.cut,
                        cut_end: expressions.cut_end,
                        graft: expressions.graft,
                    }),
                line: entry.location.line,
            })
            .collect(),
    })
}

/// The file declaring `application!(entry = …)`, if any.
/// 声明 `application!(entry = …)` 的文件（若有）。
///
/// The build walks the discovered registration folders, which is what a host
/// normally has. The authoring query walks the package's own Rust sources
/// instead, so a declaration in `src/lib.rs` or `src/main.rs` is seen too; a
/// tree the build would reject is reported rather than panicked on.
/// 构建遍历已发现的注册目录（宿主通常如此）；创作查询改为遍历包自己的 Rust 源码，
/// 因此写在 `src/lib.rs` 或 `src/main.rs` 里的声明也能看见；构建会拒绝的树在这里
/// 只被上报，不会 panic。
fn declaring_application_entry(src: &Path) -> Result<Option<PathBuf>, String> {
    let mut files = Vec::new();
    collect_rust_sources(src, &mut files)?;
    files.sort();
    let mut declarations = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file)
            .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
        for (path, location) in application_entries(&source).map_err(|error| {
            format!(
                "invalid application! declaration in {}: {error}",
                file.display()
            )
        })? {
            if !path.starts_with("crate::") {
                return Err(format!(
                    "application! entry `{path}` in {} must start with `crate::`",
                    file.display()
                ));
            }
            if !entry_path_exists(src, &path) {
                return Err(format!(
                    "application! entry `{path}` in {} does not resolve to a source module under `{}`",
                    file.display(),
                    src.display()
                ));
            }
            declarations.push((file.clone(), path, location.line, location.column));
        }
    }
    match declarations.as_slice() {
        [] => Ok(None),
        [(file, _, _, _)] => Ok(Some(file.clone())),
        many => {
            let details = many
                .iter()
                .map(|(file, path, line, column)| {
                    format!("{path} at {}:{line}:{column}", file.display())
                })
                .collect::<Vec<_>>()
                .join(", ");
            Err(format!(
                "multiple application! entry declarations found: {details}"
            ))
        }
    }
}

/// Every `.rs` file under `directory`, following the crate's own layout.
/// `directory` 下的每个 `.rs` 文件，遵循 crate 自己的布局。
fn collect_rust_sources(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("cannot scan {}: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("cannot scan {}: {error}", directory.display()))?
            .path();
        if path.is_dir() {
            collect_rust_sources(&path, files)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    Ok(())
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

/// Map a graft cut path ("root/a/b") to its registration module ("a::b").
/// A cut at the root itself forces the whole tree.
/// 把嫁接切口路径映射为注册模块路径；根上的切口意味着保留全树。
/// The two endpoints of a string cut, as modules.
/// 字符串切口两个端点对应的模块。
///
/// `"a to b"` is a range, `"a"` a single target, and `"root"` the whole tree.
/// `"a to b"` 是区间，`"a"` 是单个目标，`"root"` 是整棵树。
fn string_cut_modules(cut: &str) -> (Option<String>, Option<String>) {
    match cut.split_once(" to ") {
        Some((start, end)) => (graft_cut_module(start), graft_cut_module(end)),
        None => (graft_cut_module(cut), None),
    }
}

fn graft_cut_module(cut: &str) -> Option<String> {
    if cut == "root" {
        return None;
    }
    let module = cut.trim_start_matches("root/").replace('/', "::");
    (!module.is_empty()).then_some(module)
}

/// Derive the module a typed graft cut targets from its Rust expression.
/// 从类型化 graft 切口的 Rust 表达式推导它指向的模块。
///
/// The expression is a path to a face's `NODE_ID`, so matching it against the
/// discovered face modules is exact: it does not depend on the registry path
/// agreeing with the module path.
/// 表达式是指向某个注册面 `NODE_ID` 的路径，因此与已发现的注册面模块逐一匹配
/// 是精确的：它不依赖注册路径与模块路径一致。
fn graft_expression_module(expression: &str, faces: &[FaceSource]) -> Option<String> {
    let expression = expression.trim();
    // Hosts spell typed cuts from the crate root, while a face's module name is
    // relative: without stripping the prefix the compare missed every real cut.
    // 宿主从 crate 根书写类型化切口，而注册面的模块名是相对的：不去掉前缀就永远匹配不到
    // 真实切口。
    let expression = expression
        .strip_prefix("crate::")
        .or_else(|| expression.strip_prefix("self::"))
        .unwrap_or(expression);
    faces
        .iter()
        .find(|face| expression == format!("{}::NODE_ID", face.module))
        .map(|face| face.module.clone())
}

/// Select every face at or under `module` and return their sources for BFS.
/// 选中该模块及其子树内的全部注册面，返回其源码文本供 BFS 继续扫描。
fn select_module_subtree(
    faces: &[FaceSource],
    module: &str,
    selected: &mut BTreeSet<NodeId>,
) -> Vec<String> {
    faces
        .iter()
        .filter(|face| path_mentions_module(&face.module, module))
        .filter(|face| selected.insert(face.id))
        .filter_map(|face| fs::read_to_string(&face.source).ok())
        .collect()
}

/// A face that declares a `plugin:` field is replaceable plugin surface and
/// must survive pruning regardless of reachability.
/// 声明了 `plugin:` 字段的注册面是可替换的插件面，无论可达性都必须存活。
fn face_declares_plugin(src: &Path, face: &FaceSource) -> bool {
    let relative = relative_display(src, &face.source);
    fs::read_to_string(&face.source)
        .ok()
        .and_then(|source| parsed_face(&source, &relative))
        .and_then(|face| face.field("plugin"))
        .is_some()
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
        if face_source_is_active(src, node, scope, selected_ancestor)
            && let Some(id) = node_id(src, node)
        {
            active.insert(id);
        }
        collect_active_ids(src, &node.children, scope, selected_here, active);
    }
}

pub fn run() {
    let input = BuildInput::from_environment();
    pipeline::run(&input);
}

/// Run the discovery and validation pipeline for an explicit host project,
/// outside a Cargo build script. `package` pins the identity namespace that
/// Cargo would otherwise inject through `CARGO_PKG_NAME`. `out_dir` receives
/// the generated plan and manifests; when validation fails the rendered
/// diagnostics are returned.
/// 在 Cargo build script 之外为显式指定的宿主项目运行发现与校验管线。
/// `package` 固定身份命名空间（Cargo 本来会通过 `CARGO_PKG_NAME` 注入）。
/// `out_dir` 接收生成的计划与清单；校验失败时返回渲染后的诊断。
pub fn run_for(manifest: &Path, out_dir: &Path, package: &str) -> Result<(), String> {
    std::fs::create_dir_all(out_dir)
        .map_err(|error| format!("create {}: {error}", out_dir.display()))?;
    registry_identity::set_package_namespace(package.to_owned());
    let input = BuildInput {
        manifest: manifest.to_path_buf(),
        src: manifest.join("src"),
        out_dir: out_dir.to_path_buf(),
        emit_cargo_directives: false,
    };
    match pipeline::run(&input) {
        Some(diagnostics) => Err(diagnostics),
        None => Ok(()),
    }
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
        _ => None,
    }
}

pub(crate) fn relative_display(src: &Path, path: &Path) -> String {
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

#[cfg(test)]
mod tests {
    use super::{
        FaceSource, default_entry_source, face_declares_plugin, graft_cut_module,
        select_module_subtree,
    };
    use crate::registry_identity::NodeId;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn face(module: &str, source: PathBuf) -> FaceSource {
        FaceSource {
            id: crate::registry_identity::package_node_id(&source.to_string_lossy(), "Kind"),
            source,
            module: module.to_owned(),
        }
    }

    /// The face files the scope proved live, named relative to `src`.
    /// 作用域证明存活的注册面文件，路径相对 `src`。
    fn selected_sources(
        src: &std::path::Path,
        nodes: &[super::Node],
        scope: &super::SourceScope,
    ) -> Vec<String> {
        let roots = scope.roots.as_ref().expect("the fixture narrows");
        let mut sources = super::collect_faces(src, nodes)
            .into_iter()
            .filter(|face| roots.contains(&face.id))
            .map(|face| super::relative_display(src, &face.source))
            .collect::<Vec<_>>();
        sources.sort();
        sources
    }

    /// A lib+bin host keeps `main.rs` as a stub and calls `host!()` in `lib.rs`.
    /// The declared graft plan lives at that call, so the entry must follow it
    /// instead of Cargo's `main.rs` preference.
    /// 库+二进制宿主把 `main.rs` 留作空壳、在 `lib.rs` 里调用 `host!()`。声明的 graft
    /// 计划就在那次调用处，因此入口必须跟随它，而不是 Cargo 对 `main.rs` 的偏好。
    #[test]
    fn the_entry_is_the_file_that_calls_host() {
        let root = std::env::temp_dir().join("nichlink-entry-fixture");
        let source = root.join("src");
        std::fs::create_dir_all(&source).expect("fixture dir");
        std::fs::write(source.join("main.rs"), "fn main() {}\n").expect("stub main");
        std::fs::write(
            source.join("lib.rs"),
            "//! docs mentioning host!() in prose\nnichlink_run_method::host!();\n",
        )
        .expect("host lib");
        assert_eq!(default_entry_source(&source), source.join("lib.rs"));

        std::fs::write(
            source.join("main.rs"),
            "nichlink_run_method::host!();\nfn main() {}\n",
        )
        .expect("host main");
        assert_eq!(default_entry_source(&source), source.join("main.rs"));
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A range cut has two endpoints and neither may be dropped.
    /// 区间切口有两个端点，哪个都不能丢。
    #[test]
    fn a_range_cut_keeps_both_endpoints() {
        assert_eq!(
            super::string_cut_modules("root/a to root/b"),
            (Some("a".to_owned()), Some("b".to_owned()))
        );
        assert_eq!(
            super::string_cut_modules("root/a"),
            (Some("a".to_owned()), None)
        );
        assert_eq!(super::string_cut_modules("root"), (None, None));
    }

    /// A typed cut narrows the scope to the face it names, and a face nobody
    /// declared is pruned: declaring the slot is what ships the face.
    /// 类型化切口把作用域收窄到它命名的注册面，没人声明的面会被剪掉：
    /// 声明槽位才是这个面被发布出来的原因。
    #[test]
    fn a_typed_cut_narrows_the_scope_to_the_declared_slot() {
        let root = std::env::temp_dir().join(format!(
            "nichlink-scope-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let src = root.join("src");
        let entry = src.join("lib.rs");
        for module in ["a", "b"] {
            std::fs::create_dir_all(src.join(module)).expect("fixture dir");
        }
        std::fs::write(
            &entry,
            "nichlink_run_method::host!();\n\
             nichlink_run_method::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut(crate::a::NODE_ID) graft(\"a_fast\"),\n\
             );\n",
        )
        .expect("host entry");
        for (module, kind) in [("a", "A"), ("b", "B")] {
            std::fs::write(
                src.join(format!("{module}/{module}.rs")),
                format!("crate::root_object! {{\n    kind: {kind},\n}}\n"),
            )
            .expect("face file");
        }
        let nodes = super::discover_root(&src);
        let scope = super::SourceScope::auto_from_entry(&src, &nodes, &entry);

        assert_eq!(
            selected_sources(&src, &nodes, &scope),
            ["a/a.rs"],
            "only the declared slot stays live"
        );
        assert_eq!(scope.reason, "auto", "the scope was narrowed, not given up");

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    /// A typed cut whose Rust path names no discovered face is an unreadable
    /// declaration: the whole tree is kept rather than pruned wrongly.
    /// 类型化切口的 Rust 路径指不到任何已发现注册面时，声明就是读不懂的：
    /// 保留整棵树，而不是错误裁剪。
    #[test]
    fn an_unplaceable_typed_cut_keeps_the_whole_tree() {
        let root = std::env::temp_dir().join(format!(
            "nichlink-scope-unrecognized-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let src = root.join("src");
        let entry = src.join("lib.rs");
        std::fs::create_dir_all(src.join("a")).expect("fixture dir");
        std::fs::write(
            &entry,
            "nichlink_run_method::host!();\n\
             nichlink_run_method::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut(crate::missing::NODE_ID) graft(\"a_fast\"),\n\
             );\n",
        )
        .expect("host entry");
        std::fs::write(
            src.join("a/a.rs"),
            "crate::root_object! {\n    kind: A,\n}\n",
        )
        .expect("face file");
        let nodes = super::discover_root(&src);
        let scope = super::SourceScope::auto_from_entry(&src, &nodes, &entry);

        assert_eq!(scope.roots, None);
        assert_eq!(scope.reason, "graft-typed-cut-unrecognized");

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_typed_cut_is_recognized_from_the_crate_root() {
        let faces = vec![face(
            "control::object::button",
            PathBuf::from("src/button.rs"),
        )];
        assert_eq!(
            super::graft_expression_module("control::object::button::NODE_ID", &faces).as_deref(),
            Some("control::object::button")
        );
        assert_eq!(
            super::graft_expression_module("crate::control::object::button::NODE_ID", &faces)
                .as_deref(),
            Some("control::object::button")
        );
    }

    #[test]
    fn graft_cut_module_maps_registry_path_to_module() {
        assert_eq!(graft_cut_module("root/b").as_deref(), Some("b"));
        assert_eq!(graft_cut_module("root/a/b").as_deref(), Some("a::b"));
        assert_eq!(graft_cut_module("root"), None);
    }

    #[test]
    fn select_module_subtree_keeps_the_whole_subtree_once() {
        let root = std::env::temp_dir().join(format!(
            "nichlink-subtree-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let a = root.join("a/a.rs");
        let b = root.join("b/b.rs");
        let child = root.join("b/child/child.rs");
        for file in [&a, &b, &child] {
            std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
            std::fs::write(file, "// face\n").expect("write face");
        }
        let faces = vec![
            face("a", a.clone()),
            face("b", b.clone()),
            face("b::child", child.clone()),
        ];
        let mut selected = BTreeSet::new();
        let sources = select_module_subtree(&faces, "b", &mut selected);
        assert_eq!(sources.len(), 2);
        assert_eq!(selected.len(), 2);
        // Selecting again is a no-op.
        assert!(select_module_subtree(&faces, "b", &mut selected).is_empty());
        // Sibling "a" is untouched.
        let ids: Vec<NodeId> = faces.iter().map(|face| face.id).collect();
        assert!(!selected.contains(&ids[0]));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn plugin_declaring_face_is_detected_from_source_text() {
        let root = std::env::temp_dir().join(format!(
            "nichlink-forced-roots-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let plain = root.join("a/a.rs");
        let plugin = root.join("c/c.rs");
        std::fs::create_dir_all(plain.parent().expect("parent")).expect("mkdir");
        std::fs::create_dir_all(plugin.parent().expect("parent")).expect("mkdir");
        std::fs::write(
            &plain,
            "crate::root_object! {\n    kind: A,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
        )
        .expect("write plain face");
        std::fs::write(
            &plugin,
            "crate::root_object! {\n    kind: C,\n    plugin: manifest,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
        )
        .expect("write plugin face");

        assert!(!face_declares_plugin(&root, &face("a", plain.clone())));
        assert!(face_declares_plugin(&root, &face("c", plugin.clone())));

        let _ = std::fs::remove_dir_all(&root);
    }

    /// A throwaway package root for the entry-resolution fixtures.
    /// 入口解析夹具使用的临时包根。
    fn entry_fixture(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "nichlink-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("src")).expect("fixture dir");
        root
    }

    /// The authoring query and the build capture must see the same cuts.
    /// 创作查询与构建捕获必须看到同一批切口。
    #[test]
    fn the_authoring_query_reads_the_same_declarations_the_build_captures() {
        let root = entry_fixture("declared-grafts");
        let src = root.join("src");
        std::fs::write(
            src.join("lib.rs"),
            "nichlink_run_method::host!();\n\
             nichlink_run_method::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut \"root/control/button\" graft \"button_fast\",\n\
                 cut(crate::control::object::slider::NODE_ID) full graft(crate::external::slider_fast::NODE_ID),\n\
             );\n",
        )
        .expect("host entry");

        assert_eq!(
            super::host_entry_source(&root).expect("entry resolves"),
            src.join("lib.rs")
        );
        let declared = super::declared_grafts(&root).expect("declarations parse");
        assert_eq!(declared.entry, src.join("lib.rs"));
        assert_eq!(declared.cuts.len(), 2);
        assert_eq!(declared.cuts[0].cut, "root/control/button");
        assert_eq!(declared.cuts[0].graft, "button_fast");
        assert!(!declared.cuts[0].full);
        assert_eq!(declared.cuts[0].expressions, None);
        assert_eq!(
            declared.cuts[1].expressions,
            Some(super::DeclaredGraftExpressions {
                cut: "crate::control::object::slider::NODE_ID".to_owned(),
                cut_end: None,
                graft: "crate::external::slider_fast::NODE_ID".to_owned(),
            })
        );
        assert!(declared.cuts[1].full);

        // The build capture is the same declaration set.
        let nodes = super::discover_root(&src);
        let captured = super::host_graft_entries(&src, &nodes);
        assert_eq!(captured.len(), declared.cuts.len());
        for (captured, declared) in captured.iter().zip(&declared.cuts) {
            assert_eq!(captured.cut, declared.cut);
            assert_eq!(captured.graft, declared.graft);
            assert_eq!(captured.full, declared.full);
            assert_eq!(captured.location.line, declared.line);
        }

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    /// A feature gate is reported for the author to see, not evaluated here.
    /// 特性门控只上报给作者看，不在这里求值。
    #[test]
    fn a_gated_declaration_reports_its_gate() {
        let root = entry_fixture("declared-grafts-gated");
        std::fs::write(
            root.join("src/lib.rs"),
            "nichlink_run_method::host!();\n\
             #[cfg(feature = \"extra\")]\n\
             nichlink_run_method::static_graft_plan!(FRAMEWORK, cut \"root/a\" graft \"a_fast\");\n",
        )
        .expect("host entry");

        let declared = super::declared_grafts(&root).expect("declarations parse");
        assert_eq!(declared.cuts.len(), 1);
        assert_eq!(declared.cuts[0].cfg.as_deref(), Some("feature = \"extra\""));
        assert_eq!(declared.cuts[0].line, 3);

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    /// `application!` names the entry, and a missing one is an error, not a panic.
    /// `application!` 指定入口；入口缺失是错误而非 panic。
    #[test]
    fn the_authoring_query_follows_application_and_reports_a_missing_entry() {
        let root = entry_fixture("declared-grafts-application");
        let src = root.join("src");
        std::fs::write(src.join("lib.rs"), "nichlink_run_method::host!();\n").expect("host entry");
        std::fs::write(
            src.join("app.rs"),
            "nichlink_run_method::application!(entry = crate::app::run);\n",
        )
        .expect("application declaration");
        assert_eq!(
            super::host_entry_source(&root).expect("entry resolves"),
            src.join("app.rs")
        );

        std::fs::remove_file(src.join("app.rs")).expect("drop declaration");
        std::fs::remove_file(src.join("lib.rs")).expect("drop entry");
        assert!(super::host_entry_source(&root).is_err());
        assert!(super::declared_grafts(&root).is_err());
        assert!(super::host_entry_source(&root.join("missing")).is_err());

        std::fs::remove_dir_all(&root).expect("cleanup");
    }
}
