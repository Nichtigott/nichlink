//! `nichlink.locate`: rank the places a symptom's own words point at, in one call.
//! `nichlink.locate`：用症状自己的词，一次把"该去看的地方"排好序。
//!
//! The measured failure this answers: an agent holding a symptom (a failing assertion's message, a
//! runtime error, a description) had no entry that took **words** and returned **places**. It had to
//! guess a symbol name and ask `callgraph`, or guess a phrase and ask `search {literal}` — and the
//! seventh round measured that guessing costs one or two extra round trips per question, which is
//! the axis this bridge is behind on (113 instrument calls against 84, of which the control spends
//! 23 on indexing: answering calls were 113 against 61).
//! 这条回答的实测失败：手上有症状（失败的断言消息、运行期报错、一句描述）的代理，没有任何入口能拿
//! **词**换**地方**。它只能猜一个符号名去问 `callgraph`，或猜一句话去问 `search {literal}` —— 而第七
//! 轮量到"猜"每题要多花一到两个往返，正是本桥落后那一条轴（仪器调用 113 对 84，其中对照有 23 次花在
//! 建索引上：答题调用是 113 对 61）。
//!
//! The ranking is **text only**, and it says so: word overlap with the function's name and with the
//! doc comments above it, plus a whole-phrase bonus when the symptom's exact words appear in the
//! file. No call graph, no build records, no runtime evidence.
//! 排序**只读文本**，而且它自己说了这一点：与函数名、与它上方文档注释的词重叠，加上"整句原样出现在
//! 文件里"的加成。没有调用图、没有构建记录、没有运行期证据。

use std::path::Path;

use serde_json::Value;

use crate::mcp::callgraph::looks_like_a_test;
use crate::mcp::source_index::load_sources;
use crate::mcp::truncation::withheld;

#[cfg(test)]
#[path = "locate_tests.rs"]
mod locate_tests;

/// How many candidates the default answer lists.
/// 默认答案列出多少个候选。
const CANDIDATES: usize = 5;

/// The words a symptom is read as: lowercased, at least three characters, stopwords dropped.
/// 症状被读成的词：小写、至少三个字符、去掉停用词。
fn words(symptom: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "the", "and", "for", "that", "this", "with", "when", "from", "into", "not", "its", "was",
        "are", "has", "have", "been", "which", "what", "where", "there", "then", "than", "but",
        "you", "our", "can", "will", "should", "would", "could", "does", "did", "doing", "is",
        "of", "to", "in", "it", "on", "at", "as", "by", "or", "if", "so", "no", "do", "we", "be",
        "an", "a", "one", "two",
    ];
    let mut found: Vec<String> = Vec::new();
    for word in symptom
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric() && character != '_')
    {
        if word.len() < 3 || STOPWORDS.contains(&word) || found.iter().any(|seen| seen == word) {
            continue;
        }
        found.push(word.to_owned());
    }
    found
}

/// The doc comments of a file, lowercased — the contract half of the pairing this bridge prints.
/// 一个文件的文档注释（小写）—— 本桥打印的那个"契约"那一半。
fn doc_text(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("///") || trimmed.starts_with("//!")
        })
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// One ranked row.
/// 一行排好序的结果。
struct Row {
    score: usize,
    file: String,
    line: usize,
    name: String,
    reasons: Vec<String>,
}

/// Answer "where should I look for this symptom?" from the symptom's own words.
/// 用症状自己的词回答"这个症状该去哪里看"。
pub(crate) fn locate(root: &Path, arguments: &Value) -> Result<String, String> {
    let symptom = arguments
        .get("symptom")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|symptom| !symptom.is_empty())
        .ok_or_else(|| {
            "locate needs `symptom`: the words of the symptom (a failing assertion's message, an \
             error, a description) — accepted shape: {\"symptom\":\"<words>\",\"root\":\"<path>\"}"
                .to_owned()
        })?;
    let wanted = words(symptom);
    if wanted.is_empty() {
        return Ok(format!(
            "no matches in {}: every word of the symptom is a stopword or shorter than three \
             characters, so there is nothing to rank — pass the symptom's own distinctive words, or \
             `search {{literal}}` for an exact phrase\n",
            root.display()
        ));
    }
    let phrase = symptom.to_lowercase();
    let sources = load_sources(root)?;
    let mut rows: Vec<Row> = Vec::new();
    for file in &sources {
        let source = file.source.to_lowercase();
        let doc = doc_text(&file.source);
        let whole_phrase = source.contains(phrase.as_str());
        for function in &file.functions {
            let name = function.name.to_lowercase();
            let mut score = if whole_phrase { 6 } else { 0 };
            let mut reasons: Vec<String> = Vec::new();
            if whole_phrase {
                reasons.push("the symptom's own words appear in this file".to_owned());
            }
            let mut in_name: Vec<String> = Vec::new();
            let mut in_doc: Vec<String> = Vec::new();
            for word in &wanted {
                if name == *word {
                    score += 5;
                    in_name.push(word.clone());
                } else if name.contains(word.as_str()) {
                    score += 2;
                    in_name.push(word.clone());
                }
                if doc.contains(word.as_str()) {
                    score += 1;
                    in_doc.push(word.clone());
                }
            }
            if !in_name.is_empty() {
                reasons.push(format!("name matches `{}`", in_name.join("`, `")));
            }
            if !in_doc.is_empty() {
                reasons.push(format!("its doc overlaps `{}`", in_doc.join("`, `")));
            }
            if looks_like_a_test(&file.relative, &file.source) {
                reasons.push("test file".to_owned());
            }
            if score > 0 {
                rows.push(Row {
                    score,
                    file: file.relative.clone(),
                    line: function.line,
                    name: function.name.clone(),
                    reasons,
                });
            }
        }
    }
    rows.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(left.file.cmp(&right.file))
            .then(left.line.cmp(&right.line))
    });
    rows.dedup_by(|left, right| left.file == right.file && left.line == right.line);
    let total = rows.len();
    if total == 0 {
        return Ok(format!(
            "no matches in {}: none of the symptom's words is in a function name or a doc comment in \
             this tree — pass fewer or different words, or `search {{literal}}` for a phrase\n",
            root.display()
        ));
    }
    let mut lines = vec![format!(
        "symptom \"{symptom}\" — {total} candidate(s), ranked by word overlap with the names and the \
         docs of this tree's functions"
    )];
    for row in rows.iter().take(CANDIDATES) {
        let reason = if row.reasons.is_empty() {
            String::new()
        } else {
            format!(" ({})", row.reasons.join("; "))
        };
        lines.push(format!(
            "  {}:{} `{}`{}",
            row.file, row.line, row.name, reason
        ));
    }
    if total > CANDIDATES {
        lines.push(withheld(
            total - CANDIDATES,
            total,
            CANDIDATES,
            "candidate locations",
            "narrow the symptom's words, or read the ranked few first",
        ));
    }
    lines.push(
        "not covered by the ranking: it reads text only, so a paraphrase that shares no word with \
         the name or the doc finds nothing; dynamic dispatch, macro expansion and generated code are \
         invisible; and a hit is a place to look, not the defect"
            .to_owned(),
    );
    lines.push(
        "next   `read {path, line}` for the body (the reply prints the contract lines too), \
         `callgraph {function}` for who calls it"
            .to_owned(),
    );
    Ok(format!("{}\n", lines.join("\n")))
}
