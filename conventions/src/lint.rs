//! The missing-documentation lint gate.
//! 缺失文档 lint 门禁。
//!
//! The lint itself is enforced by the attribute this gate requires: `clippy -D warnings`
//! escalates `missing_docs` **because** the crate root switched it from allow to warn, so an
//! undocumented public item fails the build. What nothing checked is the two ways the
//! protection can be removed without failing anything: deleting the crate-level
//! `#![warn(missing_docs)]`, or silencing a single item with `#[allow(missing_docs)]`. Both
//! are invisible to the lint they disable, which is exactly the shape of decay this crate
//! exists to catch — and the reason a declaration that merely *claims* strength
//! (`#![deny(warnings)]`) cannot be accepted as the attribute: it leaves the lint at allow
//! and takes both lines of defence with it (audit `LGC-LG-24`).
//! lint 本身由本门禁所要求的属性来强制：`clippy -D warnings` 能升级 `missing_docs`，**正是因为**
//! crate 根把它从 allow 改成了 warn，因此未文档化的公开项会让构建失败。此前无人检查的是移除该保护
//! 的两种方式——删掉 crate 级 `#![warn(missing_docs)]`，或用 `#[allow(missing_docs)]` 让单个项
//! 闭嘴。两者对被它们关掉的 lint 都是不可见的，而这正是本 crate 存在的意义所在的那种腐化——也是
//! 为什么一个仅仅**声称**强度的声明（`#![deny(warnings)]`）不能被当作该属性：它让 lint 留在
//! allow，并带走两道防线（审计 `LGC-LG-24`）。
//!
//! Boundary: the requirement covers every crate directory's library root except the example
//! hosts — which includes `conventions` itself, the one `publish = false` crate that carries the
//! lint — plus the MCP bridge's `main.rs`, the one binary root that carries the attribute today
//! (binary roots are not required; it is listed so that removing it is also caught). The function
//! below states the same boundary; this paragraph used to say "the nine published crates' library
//! roots", which undercounted by leaving out the gates crate itself — the very deletion the
//! function's history mentions (audit `G-22`).
//! 边界：要求覆盖除示例宿主之外的每个 crate 目录的库根——其中包括 `conventions` 自己，那个带着
//! 该 lint 的 `publish = false` crate——外加 MCP 桥的 `main.rs`，即今天带着该属性的那一个二进制根
//! （二进制根不作要求；列出它是为了删掉它同样会被抓到）。下面那个函数写着同一条边界；本段过去写的是
//! "九个已发布 crate 的库根"，漏掉了门禁 crate 自己——而函数的历史里提到的正是它的属性曾被删除
//! （审计 `G-22`）。

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
    let head = ["al", "low"].concat();
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
            //
            // The name and its paren are matched *separately*, with whitespace allowed
            // between them, because a fourth spelling was green against the literal search:
            // `#[allow (missing_docs)]` (a space before the paren) and a `(` carried onto
            // the next line are the same attribute to rustc. Searching the bare name is why
            // the left boundary below matters: `disallow(…)` is a different function.
            // 名字与它的括号**分开**匹配，两者之间允许空白，因为对字面搜索还有第四种写法为绿：
            // `#[allow (missing_docs)]`（括号前一个空格）以及把 `(` 折到下一行，对 rustc 是同一个
            // 属性。搜索裸名字正是下面那道左边界重要的原因：`disallow(…)` 是另一个函数。
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            let masked = nichlink_kernel::source::mask_non_code(&text);
            let mut from = 0usize;
            while let Some(offset) = masked[from..].find(&head) {
                let at = from + offset;
                // An identifier character immediately before the name means it is part
                // of a longer name (`disallow(`), not the attribute.
                // 名字之前紧邻标识符字符意味着它是更长名字的一部分（`disallow(`），而不是属性。
                let boundary = at == 0
                    || !masked[..at]
                        .chars()
                        .next_back()
                        .is_some_and(|previous| previous.is_alphanumeric() || previous == '_');
                let after = &masked[at + head.len()..];
                let spaced = after.len() - after.trim_start().len();
                if boundary
                    && after[spaced..].starts_with('(')
                    && names_the_lint(&masked, at + head.len() + spaced)
                {
                    found.push(format!(
                        "{}:{}",
                        relative(root, &path),
                        masked[..at].matches('\n').count() + 1
                    ));
                }
                from = at + head.len();
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
    for line in lines(path) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }
        if !trimmed.starts_with("#![") {
            // The file's own inner attributes end at the first item. An attribute after
            // that belongs to *that* item — or to the `mod` it sits in — so the lint is
            // not on for the crate, which is what a `#![warn(missing_docs)]` moved inside
            // a module used to look like.
            // 文件自己的内部属性在第一个条目处结束。那之后的属性属于**那个**条目——或者它所在的
            // 那个 `mod`——因此 lint 并没有对整个 crate 开启；而一个被搬进某个模块里的
            // `#![warn(missing_docs)]` 过去看起来正是"已开启"。
            return false;
        }
        if (trimmed.starts_with("#![warn(") || trimmed.starts_with("#![deny("))
            && trimmed.contains(LINT_NAME)
        {
            return true;
        }
        // `#![deny(warnings)]` used to be accepted here as "the lint, at deny strength".
        // It is not: `missing_docs` is allow-by-default, so it is not a member of the
        // `warnings` group, and this gate and clippy's `-D warnings` both stayed silent
        // while the crate lost its documentation lint (audit `LGC-LG-24`). A declaration
        // that does not name the lint does not carry it, whatever strength it claims.
        // 这里过去把 `#![deny(warnings)]` 接受为"以 deny 强度带着该 lint"。事实不是：`missing_docs`
        // 默认 allow，不属于 `warnings` 组，因此本门禁与 clippy 的 `-D warnings` 会双双沉默，而那个
        // crate 已经丢掉了文档 lint（审计 `LGC-LG-24`）。没有点名该 lint 的声明就没有带着它，无论它
        // 声称多强。
        //
        // Known boundary, deliberately not widened: a crate that switches the lint on through
        // `#![cfg_attr(all(), warn(missing_docs))]` is reported as missing the attribute even
        // though the lint is on. Reading `cfg_attr` would mean evaluating `cfg`s here, and the
        // shape that makes that unsafe is one character away — `cfg_attr(any(), …)` *claims* to
        // enable the lint while leaving it at allow, which is exactly what this gate exists to
        // catch (`LGC-LG-24`). A false "write the attribute directly" on an exotic spelling is
        // the cheaper error, and the fix it asks for is one line; measured in the
        // B4-conventions independent verification (`N-1`).
        // 已知边界，有意不放宽：用 `#![cfg_attr(all(), warn(missing_docs))]` 打开 lint 的 crate 会被
        // 报成缺少该属性，尽管 lint 实际是开的。要读 `cfg_attr` 就得在这里求值 `cfg`，而让那件事变得
        // 不安全的形状只差一个字符——`cfg_attr(any(), …)` **声称**打开 lint，实际让它留在 allow，而那
        // 正是本门禁存在的意义（`LGC-LG-24`）。对一个冷僻拼法误报"请直接写出属性"是更便宜的错，而它
        // 要求的那一行改动就是一行；在 B4-conventions 的独立验证里实测（`N-1`）。
    }
    false
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

    /// The name and its paren are two tokens to rustc, and whitespace between them — a
    /// space, or a line break — is the same attribute. The literal `allow(` search saw
    /// neither, so both spellings switched the lint off invisibly.
    /// 名字与它的括号对 rustc 是两个 token，两者之间的空白——一个空格或一次换行——是同一个属性。
    /// 按字面搜索 `allow(` 两者都看不见，于是这两种拼法都能无声地关掉 lint。
    #[test]
    fn whitespace_before_the_paren_is_still_a_violation() {
        for source in [
            "#![warn(missing_docs)]\n\n#[allow (missing_docs)]\npub struct Undocumented;\n",
            "#![warn(missing_docs)]\n\n#[allow\n(missing_docs)]\npub struct Undocumented;\n",
        ] {
            let root = synthetic(&[("zz/src/lib.rs", source)]);
            let found = allow_workarounds(&root);
            assert_eq!(
                found.len(),
                1,
                "`{source}` is the same attribute: {found:?}"
            );
            let _ = std::fs::remove_dir_all(&root);
        }
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

    /// The lint has to be on the crate root, not merely somewhere in the file: an inner
    /// attribute after the first item belongs to *that* item, or to the `mod` it sits in,
    /// so deleting the root's `#![warn(missing_docs)]` and putting one inside a module
    /// left `missing_roots` empty.
    /// lint 必须开在 crate 根上，而不只是"文件里某处"：第一个条目之后的内部属性属于**那个**条目、
    /// 或它所在的那个 `mod`，因此删掉根上的 `#![warn(missing_docs)]` 再往某个模块里放一个，
    /// `missing_roots` 仍是空的。
    #[test]
    fn an_attribute_inside_a_module_does_not_count() {
        let root = synthetic(&[
            (
                "zzprobe/Cargo.toml",
                "[package]\nname = \"nichlink-zzprobe\"\n",
            ),
            (
                "zzprobe/src/lib.rs",
                "pub struct Item;\n\nmod inner {\n    #![warn(missing_docs)]\n}\n",
            ),
        ]);
        let missing = missing_roots(&root);
        assert!(
            missing.iter().any(|path| path == "zzprobe/src/lib.rs"),
            "the crate root itself carries no lint here: {missing:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// `#![deny(warnings)]` is not `missing_docs`: the lint is allow-by-default, so the
    /// `warnings` group does not contain it. Measured with this workspace's rustc:
    /// `#![deny(warnings)]` plus an undocumented `pub fn` compiles to exit 0 with no
    /// diagnostics, while `#![warn(missing_docs)]` reports `missing documentation for the
    /// crate`. Accepting the group as the lint let the real protection be deleted while
    /// both this gate and clippy's `-D warnings` stayed silent (audit `LGC-LG-24`).
    /// `#![deny(warnings)]` 不是 `missing_docs`：该 lint 默认 allow，因此 `warnings` 组里没有它。
    /// 用本工作区的 rustc 实测：`#![deny(warnings)]` 加一个未文档化的 `pub fn` 编译 exit 0、零
    /// 诊断，而 `#![warn(missing_docs)]` 会报 `missing documentation for the crate`。把该组当作
    /// 这条 lint，会让真正的保护被删掉而本门禁与 clippy 的 `-D warnings` 双双沉默（审计
    /// `LGC-LG-24`）。
    #[test]
    fn the_warnings_group_is_not_the_missing_docs_lint() {
        let root = synthetic(&[
            (
                "zzprobe/Cargo.toml",
                "[package]\nname = \"nichlink-zzprobe\"\n",
            ),
            (
                "zzprobe/src/lib.rs",
                "#![deny(warnings)]\n\npub fn undocumented() {}\n",
            ),
        ]);
        let missing = missing_roots(&root);
        assert!(
            missing.iter().any(|path| path == "zzprobe/src/lib.rs"),
            "the warnings group does not switch missing_docs on, so this root is missing \
             the lint: {missing:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
