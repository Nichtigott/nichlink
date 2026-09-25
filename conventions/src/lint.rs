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

use crate::{crate_directories, lines, relative, rust_sources};

/// Roots that must carry `#![warn(missing_docs)]`, relative to the workspace root.
/// 必须带 `#![warn(missing_docs)]` 的根，以工作区根为基准。
///
/// Derived from the tree rather than listed: a hand-kept list is one a new crate can
/// simply not appear in, and deleting `conventions`' own attribute — a crate that
/// *does* carry the lint — went unnoticed. Every crate directory's library root
/// counts except the example hosts, which document their own types by the same
/// `AGENTS.md` rule that keeps them out of the published surface, plus the one
/// binary root that carries the attribute today.
/// 从目录树推导而不是手列：手工清单正是新 crate 可以干脆不出现的那种清单，而删掉
/// `conventions` 自己的属性——一个确实带着该 lint 的 crate——此前无人发现。除示例宿主之外的每个
/// crate 目录的库根都计入（宿主自己负责文档化其类型，这与让它们不进入发布表面的是同一条
/// `AGENTS.md` 规则），外加今天带着该属性的那一个二进制根。
pub fn required_roots(root: &Path) -> Vec<String> {
    let mut roots = Vec::new();
    for directory in crate_directories(root) {
        if directory.starts_with(root.join("examples")) {
            continue;
        }
        let lib = directory.join("src/lib.rs");
        if lib.is_file() {
            roots.push(relative(root, &lib));
        }
    }
    // The MCP bridge's binary root carries the attribute as well, so removing it
    // is caught too.
    // MCP 桥的二进制根同样带着该属性，因此删掉它也会被抓到。
    let bridge = root.join("mcp/src/main.rs");
    if bridge.is_file() {
        roots.push(relative(root, &bridge));
    }
    roots.sort();
    roots.dedup();
    roots
}

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
    required_roots(root)
        .into_iter()
        .filter(|relative_path| !carries_the_lint(&root.join(relative_path)))
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
    // Built at run time so this file's own source does not contain the literal it
    // searches for: `conventions` is a crate directory like any other, so the gate
    // scans itself, and a literal needle would report this constant.
    // 运行时拼出来，使本文件自己的源码不含被搜索的字面量：`conventions` 与其他 crate 目录一样，
    // 门禁会扫描自己，而写死的针会把这个常量报出来。
    let needle = ["allow", "("].concat();
    for directory in crate_directories(root) {
        for path in rust_sources(&directory) {
            // The scan runs on the kernel's masked text — comments and literals blanked
            // — and looks for `allow(…missing_docs…)` *anywhere*, not for the item
            // attribute shape. Three spellings were measured green against the old
            // `#[allow(` search: `#![allow(missing_docs)]` at the crate root (which has
            // no `#[allow(` substring at all, and which disables the lint crate-wide
            // because the last attribute wins), a second attribute on the same line
            // (`#[allow(dead_code)] #[allow(missing_docs)]`, where the scan stopped at
            // the first `)` and then skipped the rest of the line), and
            // `#[cfg_attr(all(), allow(missing_docs))]`.
            // 扫描跑在内核屏蔽后的文本上（注释与字面量被抹掉），并搜索**任意位置**的
            // `allow(…missing_docs…)`，而不是某个项的属性形状。对旧的 `#[allow(` 搜索实测有三种
            // 写法为绿：crate 根的 `#![allow(missing_docs)]`（根本不含 `#[allow(` 子串，而它因
            // 最后一个属性生效而整 crate 关掉 lint）、同一行上的第二个属性
            // （`#[allow(dead_code)] #[allow(missing_docs)]`，扫描在第一个 `)` 停下然后跳过该行
            // 剩余部分）、以及 `#[cfg_attr(all(), allow(missing_docs))]`。
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            let masked = nichlink::source::mask_non_code(&text);
            let mut from = 0usize;
            while let Some(offset) = masked[from..].find(&needle) {
                let at = from + offset;
                // An identifier character immediately before the needle means it is part
                // of a longer name (`disallow(`), not the attribute.
                // 针之前紧邻标识符字符意味着它是更长名字的一部分（`disallow(`），而不是属性。
                let boundary = at == 0
                    || !masked[..at]
                        .chars()
                        .next_back()
                        .is_some_and(|previous| previous.is_alphanumeric() || previous == '_');
                if boundary && names_the_lint(&masked, at + needle.len() - 1) {
                    found.push(format!(
                        "{}:{}",
                        relative(root, &path),
                        masked[..at].matches('\n').count() + 1
                    ));
                }
                from = at + needle.len();
            }
        }
    }
    found
}

/// Whether the attribute whose opening paren sits at `open` names [`LINT_NAME`].
/// 开括号位于 `open` 的属性是否命名了 [`LINT_NAME`]。
///
/// The scan runs to the *matching* paren, not to the first one: `#[allow(dead_code)]
/// #[allow(missing_docs)]` on one line is two attributes, and the second one is the
/// violation. A bounded window is the fallback for an unbalanced attribute, which no
/// compiler accepts anyway.
/// 扫描走到**配对**的括号，而不是第一个：同一行上的 `#[allow(dead_code)]
/// #[allow(missing_docs)]` 是两个属性，第二个才是违规。括号不配平时回退到有界窗口——反正
/// 没有任何编译器接受那种代码。
fn names_the_lint(masked: &str, open: usize) -> bool {
    let mut depth = 0usize;
    let mut end = masked.len().min(open + 256);
    for (offset, character) in masked[open..].char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = open + offset + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    masked[..end]
        .get(open..)
        .is_some_and(|window| window.contains(LINT_NAME))
}

/// Whether a crate root carries the missing-documentation lint at warn or stronger.
/// crate 根是否以 warn 或更强的方式带着缺失文档 lint。
///
/// Accepting only the literal `#![warn(missing_docs)]` refused a *stronger*
/// declaration: `#![deny(missing_docs)]` and `#![warn(missing_docs, other)]` both keep
/// the lint on and were reported as missing it.
/// 只接受字面量 `#![warn(missing_docs)]` 会拒绝**更强**的声明：
/// `#![deny(missing_docs)]` 与 `#![warn(missing_docs, other)]` 都让 lint 保持开启，却被报成
/// 缺少它。
fn carries_the_lint(path: &Path) -> bool {
    lines(path).iter().any(|line| {
        let trimmed = line.trim();
        ((trimmed.starts_with("#![warn(") || trimmed.starts_with("#![deny("))
            && trimmed.contains(LINT_NAME))
            || trimmed == "#![deny(warnings)]"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_root;

    /// A crate the tree gains must carry the attribute, so the requirement is
    /// derived from the workspace rather than from a list that a new member can
    /// simply not appear in.
    /// 目录树新增的 crate 必须带上该属性，因此要求从工作区推导，而不是来自一份新成员可以干脆
    /// 不出现的清单。
    #[test]
    fn a_new_crate_root_must_carry_the_attribute() {
        let root = synthetic(&[
            ("zznew/Cargo.toml", "[package]\nname = \"zznew\"\n"),
            ("zznew/src/lib.rs", "pub fn undocumented() {}\n"),
            (
                "examples/host/Cargo.toml",
                "[package]\nname = \"host\"\npublish = false\n",
            ),
            ("examples/host/src/lib.rs", "pub fn host_owned() {}\n"),
        ]);
        let missing = missing_roots(&root);
        assert!(
            missing.iter().any(|path| path == "zznew/src/lib.rs"),
            "a crate without the attribute must be required to carry it: {missing:?}"
        );
        assert!(
            !missing.iter().any(|path| path.contains("examples/")),
            "a host documents its own types and is not required: {missing:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// rustfmt folds a long `#[allow(…)]`, so the scan must not depend on the
    /// attribute fitting on one physical line.
    /// rustfmt 会把长的 `#[allow(…)]` 折行，因此扫描不能依赖该属性装得进一行。
    #[test]
    fn a_folded_allow_attribute_is_still_a_violation() {
        let root = synthetic(&[(
            "zz/src/lib.rs",
            "#![warn(missing_docs)]\n\n#[allow(\n    missing_docs\n)]\npub struct Undocumented;\n",
        )]);
        let found = allow_workarounds(&root);
        assert_eq!(
            found.len(),
            1,
            "the attribute is forbidden however it is formatted: {found:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A throwaway checkout with the given files under it.
    /// 一个只含给定文件的一次性检出。
    fn synthetic(files: &[(&str, &str)]) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nichlink-lint-{}-{}-{sequence}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        for (relative, contents) in files {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("fixture dir");
            std::fs::write(&path, contents).expect("fixture file");
        }
        crate::fixture_manifest(&root);
        root
    }
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
    /// The three spellings a search for `#[allow(` was measured to miss, plus the
    /// stronger declarations it wrongly refused.
    /// 搜索 `#[allow(` 时实测漏掉的三种写法，以及它错误拒绝的更强声明。
    #[test]
    fn crate_root_allows_and_second_attributes_are_violations() {
        let cases: &[(&str, &str)] = &[
            (
                "a crate-root allow, which also has no `#[allow(` substring",
                "#![warn(missing_docs)]\n#![allow(missing_docs)]\n\npub fn probe() {}\n",
            ),
            (
                "a second attribute on the same line",
                "#![warn(missing_docs)]\n\n#[allow(dead_code)] #[allow(missing_docs)]\npub struct Probe;\n",
            ),
            (
                "an attribute nested in `cfg_attr`",
                "#![warn(missing_docs)]\n\n#[cfg_attr(all(), allow(missing_docs))]\npub struct Probe;\n",
            ),
        ];
        for (shape, source) in cases {
            let root = synthetic(&[
                (
                    "zzprobe/Cargo.toml",
                    "[package]\nname = \"nichlink-zzprobe\"\n",
                ),
                ("zzprobe/src/lib.rs", source),
            ]);
            let found = allow_workarounds(&root);
            assert!(
                found
                    .iter()
                    .any(|finding| finding.starts_with("zzprobe/src/lib.rs:")),
                "{shape} must be a violation: {found:?}"
            );
            let _ = std::fs::remove_dir_all(&root);
        }
    }

    /// A stronger or multi-lint declaration keeps the lint on and is not a missing
    /// root.
    /// 更强或含多个 lint 的声明让 lint 保持开启，不算缺失的根。
    #[test]
    fn a_stronger_or_multi_lint_declaration_is_accepted() {
        for declaration in [
            "#![warn(missing_docs)]",
            "#![deny(missing_docs)]",
            "#![warn(missing_docs, missing_debug_implementations)]",
        ] {
            let root = synthetic(&[
                (
                    "zzprobe/Cargo.toml",
                    "[package]\nname = \"nichlink-zzprobe\"\n",
                ),
                (
                    "zzprobe/src/lib.rs",
                    &format!("{declaration}\n\npub fn probe() {{}}\n"),
                ),
            ]);
            let missing = missing_roots(&root);
            assert!(
                !missing.iter().any(|path| path == "zzprobe/src/lib.rs"),
                "`{declaration}` keeps the lint on: {missing:?}"
            );
            let _ = std::fs::remove_dir_all(&root);
        }
    }
}
