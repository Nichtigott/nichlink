//! The documentation-block gate: Rust fenced in markdown must still parse.
//! 文档代码块门禁：markdown 里围栏标记的 Rust 必须仍然能解析。
//!
//! Why this exists: the root `README.md`, every crate `README.md`, and
//! `docs/*.md` together hold 50 Rust-tagged fences, and the READMEs *are* the
//! crates.io landing pages for all nine published crates (`readme =
//! "README.md"` in every manifest). Nothing extracted, compiled, or even
//! parsed one of them, so a stale signature could ship and be read by every
//! user without a single gate noticing.
//! 为什么需要它：根 `README.md`、每个 crate 的 `README.md` 与 `docs/*.md` 合计有 50 个
//! 标记为 Rust 的围栏，而这些 README **就是**九个已发布 crate 在 crates.io 上的落地页
//! （每个清单都写了 `readme = "README.md"`）。此前没有任何程序抽取、编译、甚至解析过
//! 其中任何一个，因此一份过期的签名可以发布出去被每个用户读到，而没有任何门禁察觉。
//!
//! Boundary, and why it is not the stronger check: a fence is accepted when it
//! parses either as a whole file or as a statement block, because most of these
//! blocks are deliberately excerpts — a file's contents, a macro invocation, or
//! a statement using `?`. Making them *compile* would require wrapping every
//! one in a harness with imports and a crate root, and `#![doc =
//! include_str!("../README.md")]`, the usual way to doctest a README, would turn
//! all 50 into doctests and fail the build on the excerpts that cannot compile
//! standalone. Parsing catches syntax rot today; compiling the excerpts is a
//! separate, larger piece of work and is recorded as such in the roadmap.
//! 边界，以及为什么不做更强的检查：围栏只要能作为整份文件或作为语句块解析就通过，因为这些块
//! 大多是有意的摘录——某文件的内容、一次宏调用、或一条使用 `?` 的语句。要让它们**编译**，
//! 就得给每一个套上带导入与 crate 根的脚手架；而 `#![doc = include_str!("../README.md")]`
//! 这个给 README 做 doctest 的常规做法，会把 50 个块全变成 doctest，并在那些无法独立编译的
//! 摘录上让构建失败。解析能抓住今天的语法腐化；让摘录真正编译是另一件更大的工作，已在路线图
//! 中如实登记。

use std::path::{Path, PathBuf};

use crate::{crate_directories, lines, relative};

/// One Rust-tagged fence that neither parses as a file nor as a statement block.
/// 一个既不能作为文件、也不能作为语句块解析的 Rust 围栏。
#[derive(Debug)]
pub struct Finding {
    /// Path relative to the workspace root, with `/` separators.
    /// 以工作区根为基准的路径，使用 `/` 分隔符。
    pub file: String,
    /// One-based line number of the opening fence.
    /// 起始围栏的行号（从 1 开始）。
    pub line: usize,
    /// The parser's message, which names the token it stopped at.
    /// 解析器的消息，其中点名了它停在哪一个词元。
    pub error: String,
}

/// File-name prefixes the gate does not cover, because they are records.
/// 门禁不覆盖的文件名前缀，因为它们是记录。
///
/// An audit or design document is a record of what was true when it was written,
/// and its Rust blocks are often deliberate excerpts — a `$crate` macro arm, a
/// doc-comment fragment — that were never standalone Rust. Editing such a block
/// so a new gate passes would rewrite the record, and retagging it as `text`
/// would hide that it is Rust. The living documents that a reader is expected to
/// follow (`README.md`, the crate READMEs, `docs/graft.md`, `docs/migration.md`,
/// `docs/threat-model.md`, the introduction) are all covered, and every one of
/// them parses today.
/// 审计或设计文档是"写下时事实如何"的记录，其中的 Rust 块常常是有意的摘录——一段
/// `$crate` 宏臂、一段文档注释片段——从来不是可独立成立的 Rust。为了让新门禁通过而改动这类
/// 块等于改写记录，把它们重新标成 `text` 又会掩盖"这是 Rust"。读者预期遵循的活文档
/// （`README.md`、各 crate 的 README、`docs/graft.md`、`docs/migration.md`、
/// `docs/threat-model.md`、引言）全部在覆盖范围内，且今天全部能解析。
pub const RECORD_PREFIXES: &[&str] = &["audit", "design"];

/// Every markdown file the gate covers, sorted, existing ones only.
/// 门禁覆盖的每个 markdown 文件，已排序，仅包含存在者。
pub fn markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut files = vec![root.join("README.md"), root.join("README.zh-CN.md")];
    if let Ok(entries) = std::fs::read_dir(root.join("docs")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "md") {
                files.push(path);
            }
        }
    }
    for directory in crate_directories(root) {
        files.push(directory.join("README.md"));
        files.push(directory.join("README.zh-CN.md"));
    }
    files.retain(|path| path.is_file() && !is_record(path));
    files.sort();
    files.dedup();
    files
}

/// Whether a markdown path is a record rather than living documentation.
/// 该 markdown 路径是记录而不是活文档。
pub fn is_record(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    RECORD_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// Every fenced Rust block that does not parse.
/// 每一个无法解析的 Rust 围栏块。
pub fn findings(root: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    for path in markdown_files(root) {
        let file = relative(root, &path);
        let mut open: Option<(usize, String)> = None;
        for (index, line) in lines(&path).iter().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("```") {
                if let Some((_, code)) = open.as_mut() {
                    code.push_str(line);
                    code.push('\n');
                }
                continue;
            }
            match open.take() {
                // A closing fence ends the block; the parser message is what
                // makes the report actionable, so it is kept verbatim.
                // 闭合围栏结束该块；解析器的消息让报告可操作，因此原样保留。
                Some((start, code)) => {
                    if let Err(error) = parses(&code) {
                        found.push(Finding {
                            file: file.clone(),
                            line: start,
                            error,
                        });
                    }
                }
                None => {
                    // `rust,ignore` and friends name the same language; only the
                    // text before the first comma selects it.
                    // `rust,ignore` 之类命名的是同一种语言；只有第一个逗号之前的文本用于选择。
                    let info = trimmed.trim_start_matches('`').trim();
                    let language = info.split(',').next().unwrap_or("").trim();
                    if language == "rust" {
                        open = Some((index + 1, String::new()));
                    }
                }
            }
        }
    }
    found
}

/// Whether a fence's contents parse as a file or as a statement block.
/// 围栏内容是否能作为文件或语句块解析。
fn parses(code: &str) -> Result<(), String> {
    if syn::parse_file(code).is_ok() {
        return Ok(());
    }
    // Most excerpts are statements rather than items, so a block is tried second
    // and its message is reported only when both readings fail.
    // 大多数摘录是语句而不是项，因此第二次尝试按块解析，只有两种读法都失败时才报告消息。
    let wrapped = format!("{{\n{code}\n}}");
    match syn::parse_str::<syn::Block>(&wrapped) {
        Ok(_) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_root;

    /// Every Rust block in the READMEs and docs parses.
    /// README 与文档里的每个 Rust 块都能解析。
    #[test]
    fn documented_rust_blocks_parse() {
        let root = workspace_root();
        let found = findings(&root);
        assert!(
            found.is_empty(),
            "these documented Rust blocks no longer parse: {found:#?}"
        );
    }

    /// The gate can fail, demonstrated without touching the repository.
    /// 门禁能失败，且演示过程不触碰仓库。
    #[test]
    fn a_broken_block_is_reported() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("README.md"),
            "# A host\n\n```rust\npub struct Broken {\n```\n",
        )
        .unwrap();
        let found = findings(directory.path());
        assert_eq!(found.len(), 1, "{found:#?}");
        assert_eq!(found[0].line, 3);
    }

    /// A statement excerpt and a whole file are both accepted.
    /// 语句摘录与整份文件都被接受。
    #[test]
    fn both_readings_are_accepted() {
        assert!(parses("let value = build()?;").is_ok());
        assert!(parses("pub struct Canvas;\n").is_ok());
        assert!(parses("let value = ;").is_err());
    }
}
