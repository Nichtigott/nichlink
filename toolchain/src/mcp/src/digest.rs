//! `nichlink.digest`: one bounded summary of one file, so a multi-algorithm file can be read at all.
//! `nichlink.digest`：一个文件的**有界**摘要，让"一个文件里好几套算法"也能先看清结构。
//!
//! The failure this answers is the fourth class the maintainer named: a file holding several
//! algorithms where one branch of one of them is wrong, and every part locally plausible. Reading
//! the whole file to find out what is in it is one call, but it is an unbounded one; this reply is
//! bounded, says which functions carry a written contract, which of them a test names, and who calls
//! them — and it says what it does not read.
//! 这条回答的失败是维护者点名的第四类：一个文件里好几套算法、错在其中一套的一个分支上，而每一部分单看
//! 都自洽。读整个文件能知道里面有什么，但那是一次**无界**的调用；这条回复有界，说清哪些函数带着写下来
//! 的契约、哪些被测试点名、谁调用它们 —— 以及它**不**读什么。

use std::path::Path;

use serde_json::Value;

use crate::mcp::source_index::load_one;

#[cfg(test)]
#[path = "digest_tests.rs"]
mod digest_tests;

/// How many function rows the default answer prints.
/// 默认答案最多打印多少行函数。
const ROWS: usize = 12;

/// The doc lines directly above a definition.
/// 定义正上方的文档行。
fn contract_lines(source: &str, definition: usize) -> Vec<String> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut first = definition.saturating_sub(1).min(lines.len());
    while first > 0 {
        let above = lines[first - 1].trim_start();
        if above.starts_with("///") || above.starts_with("#![") || above.starts_with("#[") {
            first -= 1;
        } else {
            break;
        }
    }
    lines[first..definition.saturating_sub(1).min(lines.len())]
        .iter()
        .filter(|line| line.trim_start().starts_with("///"))
        .map(|line| (*line).to_owned())
        .collect()
}

/// Answer "what is in this file, in one bounded reply?"
/// 用一次有界的回复回答"这个文件里有什么"。
pub(crate) fn digest(root: &Path, arguments: &Value) -> Result<String, String> {
    let relative = arguments
        .get("file")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|relative| !relative.is_empty())
        .ok_or_else(|| {
            "digest needs `file`: a path as this root sees it — accepted shape: \
             {\"file\":\"<relative path>\",\"root\":\"<path>\"}"
                .to_owned()
        })?;
    let file = load_one(root, relative)?;
    let sources = crate::mcp::source_index::load_sources(root)?;
    // Which functions a test file names: the same name-in-a-call-list rule the other readers use.
    // 哪些函数被测试文件点名：与其它读者同一条"调用列表里有这个名字"的规则。
    let mut named_by_test: Vec<String> = Vec::new();
    for source in &sources {
        if !crate::mcp::callgraph::looks_like_a_test(&source.relative, &source.source) {
            continue;
        }
        for function in &source.functions {
            for call in &function.calls {
                named_by_test.push(call.clone());
            }
        }
    }
    let mut lines = vec![format!(
        "file {} — {} function(s), {} line(s)",
        file.relative,
        file.functions.len(),
        file.source.lines().count()
    )];
    for function in file.functions.iter().take(ROWS) {
        let contract = contract_lines(&file.source, function.line);
        let tested = named_by_test
            .iter()
            .any(|call| crate::mcp::callgraph::is_call_to(call, &function.name));
        let mut callers: Vec<String> = Vec::new();
        for source in &sources {
            for candidate in &source.functions {
                if candidate
                    .calls
                    .iter()
                    .any(|call| crate::mcp::callgraph::is_call_to(call, &function.name))
                {
                    callers.push(format!("{}::{}", source.relative, candidate.name));
                }
            }
        }
        callers.sort();
        callers.dedup();
        lines.push(format!(
            "  {}:{}-{} `{}` — {} call(s) out, {} caller(s){}{}",
            file.relative,
            function.line,
            function.end_line,
            function.name,
            function.calls.len(),
            callers.len(),
            if tested { "; named by a test" } else { "" },
            if contract.is_empty() {
                String::new()
            } else {
                format!("; contract: {}", contract[0].trim_start_matches('/').trim())
            }
        ));
    }
    if file.functions.len() > ROWS {
        lines.push(crate::mcp::truncation::withheld(
            file.functions.len() - ROWS,
            file.functions.len(),
            ROWS,
            "function rows",
            "read the file, or ask `callgraph {function}` for one of them",
        ));
    }
    lines.push(
        "not covered here: which branches are dead and which are covered (ask `check {face}` for \
         the whole-tree census), the contract in full (ask `read {path, line}`), and whether the \
         functions are reachable from tests by the static walk (that column is in the same census)"
            .to_owned(),
    );
    lines.push(
        "next   `read {path, line}` for a body, `callgraph {function}` for one function's callers \
         and callees"
            .to_owned(),
    );
    Ok(format!("{}\n", lines.join("\n")))
}
