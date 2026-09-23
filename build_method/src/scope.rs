//! First-pass source scope: which registration faces survive before expansion.
//! 首轮源码作用域：rustc 展开之前哪些注册面得以存活。
//!
//! The scope decides which registration faces survive before rustc expands the
//! generated module tree. It lives in the build step so discovery and identity
//! share one parser and one node-identity implementation; entry resolution and
//! face discovery live in sibling modules and are imported here.
//! 作用域决定 rustc 展开生成模块树之前哪些注册面得以存活。它位于构建步骤中，
//! 使发现与身份计算共用同一套解析与 node identity 实现；入口解析与注册面发现位于
//! 同级模块，在此处导入使用。

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;

use nichlink::lexicon;

use super::entry::{HostEntry, path_mentions_module};
use super::faces::collect_faces;
use super::graft_view::{face_declares_plugin, graft_expression_module, string_cut_modules};
use super::node::{Node, relative_display};
use super::node_id::{collect_node_ids, node_id, select_module_subtree};
use super::registry_identity::{IDENTITY_SCHEMA, NodeId};
use super::registry_syntax::{graft_entries, source_references};

/// First-pass source scope. It gates whole registration faces before rustc
/// expands them; functions inside an enabled face are intentionally untouched.
/// 第一次源码范围修剪。在 rustc 展开前按注册面整体门控；启用注册面内部的函数
/// 不在这里处理。
#[derive(Clone, Debug)]
pub(crate) struct SourceScope {
    pub(crate) roots: Option<BTreeSet<NodeId>>,
    pub(crate) reason: &'static str,
}

impl SourceScope {
    pub(crate) fn from_environment(src: &Path, nodes: &[Node], entry: &HostEntry) -> Self {
        let Some(raw) = env::var_os(lexicon::SCOPE_ENV) else {
            return Self::auto(src, nodes, entry);
        };
        let raw = raw.to_string_lossy();
        let raw = if let Some((version, values)) = raw.split_once(':') {
            if version.strip_prefix('v') != Some(IDENTITY_SCHEMA) {
                panic!(
                    "{} uses identity schema `{version}`, expected `v{IDENTITY_SCHEMA}`",
                    lexicon::SCOPE_ENV
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
            return Self::auto(src, nodes, entry);
        }
        let ids = raw
            .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
            .filter(|value| !value.is_empty())
            .map(|value| {
                value.parse::<NodeId>().unwrap_or_else(|_| {
                    panic!(
                        "{} entry `{value}` is not a 32-digit node identity",
                        lexicon::SCOPE_ENV
                    )
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
                "{} contains unknown node identity(s): {}",
                lexicon::SCOPE_ENV,
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
    fn auto(src: &Path, nodes: &[Node], entry: &HostEntry) -> Self {
        Self::auto_from_entry(src, nodes, entry.path())
    }

    /// Derive the scope from one explicit entry source, without reading the
    /// environment, so the derivation can be pinned by a test.
    /// 从一个明确的入口源码推导作用域，不读环境变量，因此推导过程可以被测试钉住。
    ///
    /// The entry is passed in rather than resolved here: pruning and the
    /// generated cut table have to read the same file, and two independent
    /// resolutions are exactly how they stopped agreeing. A test pins that by
    /// handing both readers one explicit entry
    /// (`a_configured_entry_drives_the_scope_and_the_cut_table`).
    /// 入口由外部传入而不是就地解析：剪枝与生成的切口表必须读同一个文件，而两次各自
    /// 独立的解析正是它们开始不一致的原因。测试通过把同一个明确入口交给两个读取者来
    /// 钉住这一点（`a_configured_entry_drives_the_scope_and_the_cut_table`）。
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
            // overlay replaces everything between them. The far endpoint is now
            // separate data (`cut_end`) rather than part of the path text, so a
            // path that literally contains `" to "` is one target and is not
            // split here. The string form used to hand the whole `"start to end"`
            // text to the module mapper, so the module name contained a space,
            // matched no face, and the cut pinned nothing — leaving its targets
            // to be pruned and the overlay to fail with `UnknownTarget` later.
            // 区间切口命名两个目标，两者都必须保持存活：覆盖层替换它们之间的全部内容。
            // 远端现在是独立数据（`cut_end`）而不是路径文本的一部分，因此字面含有
            // `" to "` 的路径是单个目标，不会在这里被拆分。字符串形式过去把整段
            // `"start to end"` 交给模块映射，于是模块名里带空格、匹配不到任何面、
            // 切口什么也没钉住——目标随后被剪掉，overlay 再以 `UnknownTarget` 失败。
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
                None => match string_cut_modules(&cut.cut, cut.cut_end.as_deref()) {
                    // A range whose far endpoint mapped to the whole tree is not
                    // understood, so the whole tree is kept rather than pruned
                    // wrongly.
                    // 区间切口的远端映射到整棵树，说明这个区间没被读懂，因此保留整棵树
                    // 而不是错剪。
                    (Some(module), end) if !(cut.cut_end.is_some() && end.is_none()) => {
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

    pub(crate) fn includes(&self, src: &Path, node: &Node, selected_ancestor: bool) -> bool {
        if lexicon::SCOPE_ALWAYS_INCLUDED.contains(&node.name.as_str())
            || node
                .file
                .as_ref()
                .is_some_and(|file| lexicon::is_registration_path(&relative_display(src, file)))
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

#[cfg(test)]
mod tests {
    use crate::discovery::discover_root;

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
        let nodes = discover_root(&src);
        let scope = super::SourceScope::auto_from_entry(&src, &nodes, &entry);

        assert_eq!(
            selected_sources(&src, &nodes, &scope),
            ["a/a.rs"],
            "only the declared slot stays live"
        );
        assert_eq!(scope.reason, "auto", "the scope was narrowed, not given up");

        std::fs::remove_dir_all(&root).expect("cleanup");
    }

    /// The scope and the generated cut table read the entry the build resolved,
    /// not two independently resolved ones.
    /// 作用域与生成的切口表读的是构建解析出的同一个入口，而不是各自独立解析出的两个。
    ///
    /// The configured value is passed in explicitly because the value itself is
    /// what the two readers must agree on; reading the process environment here
    /// would race with every other test in this binary. The fixture makes the two
    /// entries name different slots, so a reader that ignores the configured value
    /// keeps `beta` live and reports `beta`'s cut, while a reader that follows it
    /// keeps `alpha` live and reports `alpha`'s.
    /// 被指定的值以显式参数传入，因为两个读取者必须一致的就是这个值；在这里读进程环境
    /// 会与同一二进制里的其他测试竞争。夹具让两个入口声明不同的槽位，因此忽略指定值的
    /// 读取者会让 `beta` 存活并报告 `beta` 的切口，跟随它的读取者则让 `alpha` 存活并
    /// 报告 `alpha` 的切口。
    #[test]
    fn a_configured_entry_drives_the_scope_and_the_cut_table() {
        let root = std::env::temp_dir().join(format!(
            "nichlink-scope-configured-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let src = root.join("src");
        for module in ["alpha", "beta", "preview"] {
            std::fs::create_dir_all(src.join(module)).expect("fixture dir");
        }
        // The convention entry — the file that calls `host!()` — declares `beta`.
        // 约定入口（调用 `host!()` 的文件）声明 `beta`。
        std::fs::write(
            src.join("lib.rs"),
            "nichlink_run_method::host!();\n\
             nichlink_run_method::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut(crate::beta::NODE_ID) graft(\"beta_fast\"),\n\
             );\n",
        )
        .expect("convention entry");
        // The configured entry declares `alpha` instead.
        // 被指定的入口改为声明 `alpha`。
        std::fs::write(
            src.join("preview/preview.rs"),
            "nichlink_run_method::static_graft_plan!(\n\
                 FRAMEWORK,\n\
                 cut(crate::alpha::NODE_ID) graft(\"alpha_fast\"),\n\
             );\n",
        )
        .expect("configured entry");
        for (module, kind) in [("alpha", "Alpha"), ("beta", "Beta")] {
            std::fs::write(
                src.join(format!("{module}/{module}.rs")),
                format!("crate::root_object! {{\n    kind: {kind},\n}}\n"),
            )
            .expect("face file");
        }

        let nodes = discover_root(&src);
        let entry = crate::entry::resolve_host_entry(
            &src,
            &nodes,
            Some(std::path::PathBuf::from("src/preview/preview.rs")),
        );
        assert_eq!(entry.path(), src.join("preview/preview.rs"));

        let cuts = crate::host_graft_entries(&entry).enabled;
        assert_eq!(cuts.len(), 1, "{cuts:?}");
        assert_eq!(cuts[0].cut, "crate::alpha::NODE_ID");

        let scope = super::SourceScope::auto_from_entry(&src, &nodes, entry.path());
        assert_eq!(
            selected_sources(&src, &nodes, &scope),
            ["alpha/alpha.rs"],
            "pruning follows the same entry the cut table read"
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
        let nodes = discover_root(&src);
        let scope = super::SourceScope::auto_from_entry(&src, &nodes, &entry);

        assert_eq!(scope.roots, None);
        assert_eq!(scope.reason, "graft-typed-cut-unrecognized");

        std::fs::remove_dir_all(&root).expect("cleanup");
    }
}
