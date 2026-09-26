//! The identity namespace Studio authors under, from the host manifest.
//! Studio 创作所用的身份命名空间，读自宿主清单。
//!
//! A host's build script stamps `env!("CARGO_PKG_NAME")` as the identity namespace
//! of every registration face it compiles, and every `NodeId` is a hash over
//! `(namespace, source path, declared name)`. Studio rebuilds the same tree from
//! source, so it has to derive the *same* namespace or its ids name nodes the host
//! never compiled: a recorded trace, a graft record on disk, and the editor's tree
//! would each live in a different identity domain. Cargo's answer for the adopted
//! manifest is what makes the two agree without asking the user to export
//! `NICH_LINK_NAMESPACE`.
//! 宿主的构建脚本把 `env!("CARGO_PKG_NAME")` 盖成它编译的每个注册面的身份命名空间，而每个
//! `NodeId` 都是对 `(命名空间, 源码路径, 声明名)` 的散列。Studio 从源码重建同一棵树，因此必须
//! 推出**同一个**命名空间，否则它的 id 指的是宿主从未编译过的节点：已记录的 trace、落盘的 graft
//! 记录、编辑器里的树会各处在不同的身份域。Cargo 对已采纳清单的回答，正是让两端在不要求用户
//! 导出 `NICH_LINK_NAMESPACE` 的情况下取得一致的东西。
//!
//! This used to be a literal `[package] name` line scan, and the difference is not
//! academic: TOML's dotted form (`package.name = "x"`) is the same table with no
//! `[package]` header, so the scan found nothing and Studio authored that project
//! under the documented default — an identity domain none of its recorded ids live
//! in. The authority is now `nichlink_build_method::package_name`, shared with the
//! CLI and the MCP bridge, so all three name a package the same way.
//! 这里曾经是对 `[package] name` 的逐行字面扫描，而差别不是学理上的：TOML 的点式写法
//! （`package.name = "x"`）是同一张表却没有 `[package]` 表头，因此扫描什么也找不到，Studio
//! 就在文档化的默认值之下创作那个项目——而它记录的任何 id 都不住在那个身份域里。现在的权威是
//! `nichlink_build_method::package_name`，与 CLI 和 MCP 桥共用，因此三者对包的命名方式一致。
//!
//! Split out of `support` when that file reached the size ceiling; the boundary is
//! the concern, not the line count: `support` is interaction geometry and editor
//! helpers, this is "which package is this session, and under what name".
//! 在该文件触到尺寸上限时从 `support` 拆出；界线是关注点而不是行数：`support` 是交互几何与编辑器
//! 辅助，这里是"本会话是哪个包、在什么名字之下"。

use std::path::{Path, PathBuf};

/// The manifest a session uses for one project root.
/// 一个会话为某个项目根使用的清单。
///
/// `NICH_LINK_HOST_MANIFEST` names it explicitly — absolute, relative to the root,
/// or a directory holding a `Cargo.toml` — and otherwise it is the root's own
/// `Cargo.toml`.
/// `NICH_LINK_HOST_MANIFEST` 可以显式指名它——绝对路径、相对根的路径，或一个含 `Cargo.toml`
/// 的目录——否则就是根本身的 `Cargo.toml`。
pub(super) fn manifest_for(root: &Path) -> PathBuf {
    if let Some(configured) = std::env::var_os("NICH_LINK_HOST_MANIFEST") {
        let path = PathBuf::from(configured);
        let path = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        return if path.is_dir() {
            path.join("Cargo.toml")
        } else {
            path
        };
    }
    root.join("Cargo.toml")
}

/// The identity namespace one host manifest stands for.
/// 一份宿主清单所代表的身份命名空间。
///
/// `NICH_LINK_NAMESPACE` wins verbatim when it is set — the same rule the build
/// side reads, so an override moves both ends together — and otherwise the package
/// name Cargo reports is the namespace, because that is exactly what the build
/// script's `env!("CARGO_PKG_NAME")` stamps.
/// `NICH_LINK_NAMESPACE` 一旦设置就原样胜出——与构建侧读的同一条规则，因此覆盖会把两端一起
/// 挪动——否则 Cargo 报告的包名就是命名空间，因为构建脚本的 `env!("CARGO_PKG_NAME")` 盖的
/// 正是它。
///
/// The documented default is the last resort here, and that is the deliberate
/// asymmetry with the MCP bridge's registry query: **authoring creates** a tree, so
/// for a project nobody has built yet — a virtual workspace root, or a directory
/// with no manifest — `nichlink.default` is a real answer, and the wizard that
/// scaffolds a new project depends on it. A *query* about an existing tree must not
/// invent an identity domain, which is why the bridge refuses instead.
/// 这里把文档化的默认值留作最后兜底，这正是与 MCP 桥的注册树查询之间有意的不对称：**创作是在
/// 创建**一棵树，因此对一个还没人构建过的项目——虚拟工作区根，或没有清单的目录——
/// `nichlink.default` 是真实答案，而搭建新项目的向导依赖它。针对已存在树的**查询**则不能凭空
/// 造出身份域，因此桥选择拒绝。
pub(super) fn namespace_for(manifest: &Path, configured: Option<&str>) -> String {
    if let Some(configured) = configured {
        return configured.to_owned();
    }
    nichlink_build_method::package_name(manifest)
        .unwrap_or_else(|_| nichlink_run_method::lexicon::DEFAULT_NAMESPACE.to_owned())
}
