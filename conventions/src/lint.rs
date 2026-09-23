//! The missing-documentation lint gate.
//! 缺失文档 lint 门禁。
//!
//! The lint itself is already enforced: `clippy -D warnings` escalates
//! `missing_docs`, so an undocumented public item fails the build. What nothing
//! checked is the two ways the protection can be removed without failing
//! anything: deleting the crate-level `#![warn(missing_docs)]`, or silencing a
//! single item with `#[allow(missing_docs)]`. Both are invisible to the lint
//! they disable, which is exactly the shape of decay this crate exists to catch.
//! lint 本身已被强制：`clippy -D warnings` 会升级 `missing_docs`，因此未文档化的公开项会
//! 让构建失败。此前无人检查的是移除该保护的两种方式——删掉 crate 级
//! `#![warn(missing_docs)]`，或用 `#[allow(missing_docs)]` 让单个项闭嘴。两者对被它们关掉的
//! lint 都是不可见的，而这正是本 crate 存在的意义所在的那种腐化。
//!
//! Boundary: the requirement covers the nine published crates' library roots.
//! Binary roots are not required, because the roadmap scoped the promise to the
//! documented published surface; the MCP bridge's `main.rs` carries the
//! attribute anyway and is listed so that removing it is also caught.
//! 边界：要求覆盖九个已发布 crate 的库根。二进制根不作要求，因为路线图把承诺限定在已文档化
//! 的发布表面上；MCP 桥的 `main.rs` 反正也带着该属性，一并列出，这样删掉它同样会被抓到。

use std::path::Path;

use crate::{crate_directories, is_comment, lines, relative, rust_sources};

/// Roots that must carry `#![warn(missing_docs)]`, relative to the workspace root.
/// 必须带 `#![warn(missing_docs)]` 的根，以工作区根为基准。
pub const REQUIRED_ROOTS: &[&str] = &[
    "core/src/lib.rs",
    "macro/src/lib.rs",
    "run_method/src/lib.rs",
    "build_method/src/lib.rs",
    "cli/src/lib.rs",
    "debug_method/src/lib.rs",
    "studio/src/lib.rs",
    "plugin-host/src/lib.rs",
    "mcp/src/lib.rs",
    "mcp/src/main.rs",
];

/// The documented lint attribute.
/// 文档化的 lint 属性。
pub const ATTRIBUTE: &str = "#![warn(missing_docs)]";

/// The head of the attribute that opens an `allow` list.
/// 打开 `allow` 列表的属性开头。
///
/// Kept as two constants so that no single source line of this file contains
/// both halves of the pattern it searches for: a one-line scan would otherwise
/// match itself whenever the two literals appear in the same condition.
/// 拆成两个常量，是为了让本文件没有任何一行同时包含被搜索模式的两半：否则只要两个
/// 字面量出现在同一行条件里，扫描就会命中自己。
pub const ALLOW_HEAD: &str = "#[allow(";

/// The lint name this gate protects.
/// 本门禁保护的 lint 名。
pub const LINT_NAME: &str = "missing_docs";

/// Required roots that do not carry [`ATTRIBUTE`].
/// 未携带 [`ATTRIBUTE`] 的必需根。
pub fn missing_roots(root: &Path) -> Vec<String> {
    REQUIRED_ROOTS
        .iter()
        .filter(|relative_path| {
            let path = root.join(relative_path);
            !lines(&path).iter().any(|line| line.trim() == ATTRIBUTE)
        })
        .map(|path| (*path).to_owned())
        .collect()
}

/// Every file that silences the lint for one item, with its line number.
/// 每个让单个项对 lint 闭嘴的文件，含行号。
///
/// The scan requires the attribute shape (`#[allow(`) rather than the bare lint
/// name, and skips comments, so prose that discusses the rule is not reported as
/// a violation of it.
/// 扫描要求属性形状（`#[allow(`）而不是裸的 lint 名，并跳过注释，因此讨论该规则的散文
/// 不会被报成违反该规则。
pub fn allow_workarounds(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for directory in crate_directories(root) {
        for path in rust_sources(&directory) {
            for (index, line) in lines(&path).iter().enumerate() {
                if is_comment(line) {
                    continue;
                }
                if line.contains(ALLOW_HEAD) && line.contains(LINT_NAME) {
                    found.push(format!("{}:{}", relative(root, &path), index + 1));
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_root;

    /// Every published library root keeps the lint switched on.
    /// 每个已发布的库根都保持 lint 打开。
    #[test]
    fn the_published_roots_keep_the_lint_on() {
        let root = workspace_root();
        let missing = missing_roots(&root);
        assert!(
            missing.is_empty(),
            "these roots must carry `{ATTRIBUTE}`; document the item instead of \
             disabling the lint: {missing:#?}"
        );
    }

    /// No item silences the lint; document it instead.
    /// 没有任何项让 lint 闭嘴；请改为补文档。
    #[test]
    fn no_item_silences_the_lint() {
        let root = workspace_root();
        let found = allow_workarounds(&root);
        assert!(
            found.is_empty(),
            "`allow(missing_docs)` is forbidden by AGENTS.md; write the bilingual \
             doc comment instead: {found:#?}"
        );
    }
}
