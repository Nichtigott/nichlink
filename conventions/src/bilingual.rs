//! The bilingual-documentation gate: every doc block carries English and Chinese.
//! 双语文档门禁：每个文档块都同时带英文与中文。
//!
//! `AGENTS.md` asks for a public item to be documented in English and then in
//! Chinese. Nothing enforced it: `missing_docs` asks whether a doc comment
//! exists, not what language it is in, so an English-only doc reached docs.rs
//! silently and the convention held only while reviewers remembered it. The
//! first run of this gate found nineteen blocks in the shipped tree already
//! drifted, which is the measurement that the rule had decayed.
//! `AGENTS.md` 要求公开项先英文、后中文地文档化。此前没有任何东西强制它：`missing_docs`
//! 只问文档注释是否存在，不问它是什么语言，因此纯英文文档会静默地进入 docs.rs，而这条约定只在
//! 评审者记得时成立。本门禁第一次运行就在出厂树里发现十九个已经漂移的块——这就是该规则已经腐化的
//! 实测。
//!
//! Boundary: `src/` trees of the crates that carry the missing-documentation
//! lint. The two example hosts are outside both rules by the same `AGENTS.md`
//! sentence — a host documents its own types — and a `tests/` tree is not the
//! documented surface.
//! 边界：带缺失文档 lint 的那些 crate 的 `src/` 树。两个示例宿主因 `AGENTS.md` 的同一句话
//! 同在这两条规则之外——宿主自己负责文档化其类型——而 `tests/` 树不是被文档化的表面。

use std::path::Path;

use crate::{crate_directories, is_real_directory, relative, rust_sources};

/// Whether the text carries at least one CJK character.
/// 文本是否至少含一个中日韩字符。
///
/// The ranges are the ones a Chinese sentence actually uses: the unified
/// ideographs and their extension A, CJK punctuation (`。`, `、`), and the
/// fullwidth forms (`，`, `：`). A block counts as Chinese when it has one of
/// them anywhere, so a line that mixes English prose with a Chinese gloss is
/// accepted and an English-only block is not.
/// 这些范围是中文句子实际会用到的：统一表意文字及其扩展 A、中日韩标点（`。`、`、`）以及全角形式
/// （`，`、`：`）。只要块内任意位置有其中一个就算中文，因此英文散文夹中文释义的块会被接受，纯英文
/// 块不会。
pub fn has_cjk(text: &str) -> bool {
    text.chars().any(|character| {
        matches!(
            character,
            '\u{3000}'..='\u{303f}'
                | '\u{3400}'..='\u{4dbf}'
                | '\u{4e00}'..='\u{9fff}'
                | '\u{f900}'..='\u{faff}'
                | '\u{ff00}'..='\u{ffef}'
                | '\u{20000}'..='\u{2a6df}'
        )
    })
}

/// One run of consecutive `///` or `//!` lines, as (line number, text) pairs.
/// 一段连续的 `///` 或 `//!` 行，元素为（行号，文本）。
///
/// A doc-shaped line *inside* a string literal is not a doc comment: the check
/// runs the raw lines against [`nichlink::source::mask_literals`], which blanks
/// literals and keeps comments. Reading the raw lines alone reported a workspace
/// test fixture whose string carried a `///` line.
/// 字符串字面量**内部**看起来像文档注释的行不是文档注释：判断把原始行与
/// [`nichlink::source::mask_literals`] 的结果对照，后者抹掉字面量、保留注释。只读原始行曾把
/// 一个工作区测试夹具报成违规——那个夹具的字符串里带着一行 `///`。
pub fn blocks(text: &str) -> Vec<Vec<(usize, String)>> {
    let masked = nichlink::source::mask_literals(text);
    let masked_lines: Vec<&str> = masked.lines().collect();
    let mut found = Vec::new();
    let mut current: Vec<(usize, String)> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let shape = trimmed.starts_with("///") || trimmed.starts_with("//!");
        let real_comment = masked_lines.get(index).is_some_and(|masked| {
            let masked = masked.trim_start();
            masked.starts_with("///") || masked.starts_with("//!")
        });
        if shape && real_comment {
            current.push((index + 1, trimmed.to_owned()));
        } else if !current.is_empty() {
            found.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        found.push(current);
    }
    found
}

/// Every doc block with no Chinese text, with its file and first line.
/// 每个不含中文的文档块，含文件名与首行。
pub fn findings(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for directory in crate_directories(root) {
        if relative(root, &directory).starts_with("examples/") {
            continue;
        }
        let source = directory.join("src");
        if !is_real_directory(&source) {
            continue;
        }
        for path in rust_sources(&source) {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            for block in blocks(&text) {
                if block.iter().any(|(_, line)| has_cjk(line)) {
                    continue;
                }
                let (line, first) = &block[0];
                found.push(format!("{}:{line} {first}", relative(root, &path)));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line with Chinese punctuation counts as Chinese, and a mixed line does
    /// too, because that is how the shipped doc blocks are written.
    /// 含中文标点的行算中文，混排的行也算，因为出厂的文档块就是这么写的。
    #[test]
    fn chinese_text_is_recognised_by_its_own_characters() {
        assert!(has_cjk("/// 中文"));
        assert!(has_cjk("/// English then 中文"));
        assert!(has_cjk("/// 中文标点：句号。"));
        assert!(!has_cjk("/// English only, ascii: 1234"));
    }

    /// A block is a run of doc lines; a non-doc line ends it, so a Chinese line
    /// above does not excuse an English-only block below.
    /// 块是一段连续的文档行；非文档行会终止它，因此上面一行中文不能为下面纯英文的块开脱。
    #[test]
    fn blocks_end_at_the_first_non_doc_line() {
        let text = "//! 中文\n//! 继续\ncode();\n/// english\n/// more english\nfn x() {}\n";
        let found = blocks(text);
        assert_eq!(found.len(), 2);
        assert!(found[0].iter().any(|(_, line)| has_cjk(line)));
        assert!(!found[1].iter().any(|(_, line)| has_cjk(line)));
    }

    /// A doc-shaped line inside a string literal is not a doc comment, which is
    /// the shape a workspace test fixture has.
    /// 字符串字面量里看起来像文档注释的行不是文档注释，而这正是工作区某个测试夹具的形状。
    #[test]
    fn a_doc_shaped_line_inside_a_string_is_not_a_doc_comment() {
        let text = "fn f() {\n    let _note = \"prose\\\n/// english only\\\n\";\n}\n";
        assert!(
            blocks(text).is_empty(),
            "a `///` inside a string is not a doc block: {:#?}",
            blocks(text)
        );
    }

    /// The shipped tree is bilingual: an English-only block anywhere under a
    /// linted crate's `src/` is a finding.
    /// 出厂树是双语的：带 lint 的 crate 的 `src/` 下任何纯英文块都是违规。
    #[test]
    fn the_shipped_tree_documents_in_both_languages() {
        let found = findings(&crate::workspace_root());
        assert!(
            found.is_empty(),
            "every doc block needs an English line and a Chinese line: {found:#?}"
        );
    }
}
