//! The identity namespace Studio authors under, read from the host manifest.
//! Studio 创作所用的身份命名空间，读自宿主清单。
//!
//! A host's build script stamps `env!("CARGO_PKG_NAME")` as the identity namespace
//! of every registration face it compiles, and every `NodeId` is a hash over
//! `(namespace, source path, declared name)`. Studio rebuilds the same tree from
//! source, so it has to derive the *same* namespace or its ids name nodes the host
//! never compiled: a recorded trace, a graft record on disk, and the editor's tree
//! would each live in a different identity domain. Reading `[package] name` out of
//! the target manifest is what makes the two agree without asking the user to
//! export `NICH_LINK_NAMESPACE`.
//! 宿主的构建脚本把 `env!("CARGO_PKG_NAME")` 盖成它编译的每个注册面的身份命名空间，而每个
//! `NodeId` 都是对 `(命名空间, 源码路径, 声明名)` 的散列。Studio 从源码重建同一棵树，因此必须
//! 推出**同一个**命名空间，否则它的 id 指的是宿主从未编译过的节点：已记录的 trace、落盘的 graft
//! 记录、编辑器里的树会各处在不同的身份域。从目标清单读 `[package] name` 正是让两端在不要求用户
//! 导出 `NICH_LINK_NAMESPACE` 的情况下取得一致的东西。
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
/// side reads, so an override moves both ends together — and otherwise the
/// `[package] name` is the namespace, because that is exactly what the build
/// script's `env!("CARGO_PKG_NAME")` stamps. A manifest with no `[package]` (a
/// virtual workspace root, say) falls back to the documented default.
/// `NICH_LINK_NAMESPACE` 一旦设置就原样胜出——与构建侧读的同一条规则，因此覆盖会把两端一起
/// 挪动——否则 `[package] name` 就是命名空间，因为构建脚本的 `env!("CARGO_PKG_NAME")` 盖的正是
/// 它。没有 `[package]` 的清单（比如虚拟工作区根）回落到文档化的默认值。
pub(super) fn namespace_for(manifest: &Path, configured: Option<&str>) -> String {
    if let Some(configured) = configured {
        return configured.to_owned();
    }
    package_name(manifest)
        .unwrap_or_else(|| nichlink_run_method::lexicon::DEFAULT_NAMESPACE.to_owned())
}

/// The literal `[package] name` in a manifest, when it is a package manifest.
/// 清单里字面的 `[package] name`；该清单是包的清单时才有值。
///
/// Scoped to the section and to a literal value: a `name` under `[dependencies]`
/// is another package's name, a commented-out `name` is not a name, and Cargo does
/// not allow `name` to be inherited from a workspace — so a missing literal means
/// this is not a package manifest rather than a value read from somewhere else.
/// 限定在该节内、且必须是字面值：`[dependencies]` 下的 `name` 是别的包的名字，被注释掉的
/// `name` 不是名字，而 Cargo 不允许 `name` 从工作区继承——因此找不到字面值只说明这不是一份包的
/// 清单，而不是某个别处读来的取值。
pub(super) fn package_name(manifest: &Path) -> Option<String> {
    let text = std::fs::read_to_string(manifest).ok()?;
    let mut in_package = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(header) = trimmed.strip_prefix('[') {
            in_package = header.trim_end_matches(']').trim() == "package";
            continue;
        }
        if !in_package || trimmed.starts_with('#') {
            continue;
        }
        let Some((left, right)) = trimmed.split_once('=') else {
            continue;
        };
        if left.trim() != "name" {
            continue;
        }
        let rest = right.trim_start();
        let value = match rest.chars().next() {
            Some(quote @ ('"' | '\'')) => rest[1..]
                .split(quote)
                .next()
                .unwrap_or("")
                .trim()
                .to_owned(),
            _ => rest
                .split(['#', ' ', '\t'])
                .next()
                .unwrap_or("")
                .trim()
                .to_owned(),
        };
        if !value.is_empty() {
            return Some(value);
        }
    }
    None
}
