//! `nichlink.why`: one call that gathers the upstream facts a symptom at a line depends on.
//! `nichlink.why`：一次调用，把一个位置的症状所依赖的上游事实收齐。
//!
//! The measured failure: a symptom in file Z sent agents hopping — `callgraph` for the callers, then
//! a `read` for the contract, then the registry or the plan for whether the face ships. The seventh
//! round measured that shape directly: the four injected-defect questions took 5, 6, 5 and 8
//! instrument calls against the control's 5, 2, 2 and 2, and the extra hops were exactly the ones
//! this tool answers in one reply.
//! 量到的失败：一个在 Z 文件里的症状逼代理逐跳——`callgraph` 查调用者、`read` 看契约、再问注册面或计划
//! 看这个面到底发不发布。第七轮直接量到这个形状：四道注入缺陷分别花了 5、6、5、8 次仪器调用，而对照是
//! 5、2、2、2，多出来的那几跳正是这个工具一次回答的东西。
//!
//! What it does **not** answer, it names: whether the definition is in the published tree and how its
//! slot is wired, whether a feature gates it, and any runtime evidence. Those are `check {face}`'s,
//! `registry`/`grafts`' and `trace`'s questions, and the reply says which one to ask.
//! 它**不**回答的，它会点名：这个定义是否在发布树里、槽位怎么接、是否被特性门控、以及运行期证据 ——
//! 那些分别归 `check {face}`、`registry`/`grafts` 与 `trace`，答案里会写明该问哪一个。

use std::path::Path;

use serde_json::Value;

use crate::mcp::callgraph::{caller_note, is_call_to};
use crate::mcp::source_index::{load_one, load_sources};

#[cfg(test)]
#[path = "why_tests.rs"]
mod why_tests;

/// How many doc lines of the contract the reply prints.
/// 答案打印多少行契约文档。
const CONTRACT_LINES: usize = 6;

/// The doc lines directly above a definition, and the line they start on.
/// 定义正上方的文档行，以及它们起始的行号。
fn contract(source: &str, definition: usize) -> Vec<(usize, String)> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut first = definition.saturating_sub(1).min(lines.len());
    while first > 0 {
        let above = lines[first - 1].trim_start();
        if above.starts_with("///") || above.starts_with("#[") {
            first -= 1;
        } else {
            break;
        }
    }
    lines[first..definition.saturating_sub(1).min(lines.len())]
        .iter()
        .enumerate()
        .filter(|(_, line)| line.trim_start().starts_with("///"))
        .map(|(offset, line)| (first + offset + 1, (*line).to_owned()))
        .take(CONTRACT_LINES)
        .collect()
}

/// Answer "what does this line depend on?" for a `path:line`.
/// 对 `路径:行号` 回答"这一行依赖什么"。
pub(crate) fn why(root: &Path, arguments: &Value) -> Result<String, String> {
    let at = arguments
        .get("at")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|at| !at.is_empty())
        .ok_or_else(|| {
            "why needs `at`: a `path:line` inside this tree — accepted shape: \
             {\"at\":\"<relative path>:<line>\",\"root\":\"<path>\"}; a registration node is \
             `explain {node}`'s question"
                .to_owned()
        })?;
    let Some((relative, line)) = at.rsplit_once(':') else {
        return Err(format!(
            "why needs `at` as `path:line`, got `{at}` — accepted shape: \
             {{\"at\":\"<relative path>:<line>\",\"root\":\"<path>\"}}"
        ));
    };
    let Ok(line) = line.trim().parse::<usize>() else {
        return Err(format!(
            "why needs a line number after the last colon, got `{at}` — accepted shape: \
             {{\"at\":\"<relative path>:<line>\",\"root\":\"<path>\"}}"
        ));
    };
    let file = load_one(root, relative.trim())?;
    let Some(function) = file
        .functions
        .iter()
        .find(|function| function.line <= line && line <= function.end_line)
    else {
        return Ok(format!(
            "no function covers {}:{line} in {} — the index lists:\n{}\n",
            file.relative,
            root.display(),
            file.functions
                .iter()
                .map(|function| format!(
                    "  {} lines {}-{}",
                    function.name, function.line, function.end_line
                ))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    };

    let mut lines = vec![format!(
        "at {}:{} — the definition `{}` (lines {}-{})",
        file.relative, line, function.name, function.line, function.end_line
    )];
    let contract = contract(&file.source, function.line);
    if contract.is_empty() {
        lines.push("  contract   none: no `///` line sits above this definition".to_owned());
    } else {
        lines.push(
            "  contract   (from the doc above it, which is the promise this tree writes)"
                .to_owned(),
        );
        for (number, text) in &contract {
            lines.push(format!("             {}:{number} {text}", file.relative));
        }
    }

    // Who calls it — the same rule every other reader uses, so this cannot disagree with `callgraph`.
    // 谁调用它 —— 与其它读者同一条规则，因此它不会与 `callgraph` 给出不同答案。
    let sources = load_sources(root)?;
    let mut callers: Vec<String> = Vec::new();
    for source in &sources {
        for caller in &source.functions {
            if caller
                .calls
                .iter()
                .any(|call| is_call_to(call, &function.name))
            {
                let note = caller_note(&file.relative, &source.relative);
                let entry = format!("{}::{}", source.relative, caller.name);
                callers.push(match note {
                    Some(note) => format!("{entry} ({note})"),
                    None => entry,
                });
            }
        }
    }
    callers.sort();
    callers.dedup();
    if callers.is_empty() {
        lines.push("  callers    0 in this root".to_owned());
    } else {
        lines.push(format!("  callers    {}", callers.join(", ")));
    }

    // The adoption ledger, when this tree carries one: entries that name this file. The verdict is
    // `adopted`'s to compute; this says only which entries name the file, and says so.
    // 采信台账（这棵树带的话）：点名了这个文件的条目。判定归 `adopted` 算；这里只说哪些条目点了名。
    match crate::mcp::adopted::entries(root) {
        Err(error) => lines.push(format!("  adoption   ledger unreadable ({error})")),
        Ok(entries) => {
            // The parser is the kernel's (`parse_adoption`), reached through `adopted::entries`, so
            // this reader cannot drift from the verdict `adopted` prints. It says only which entries
            // name the file; the verdict is `adopted`'s to compute.
            // 解析器是内核那个（`parse_adoption`），经 `adopted::entries` 到达，因此这个读者不会与
            // `adopted` 打印的判定漂移。它只说哪些条目点名了这个文件；判定归 `adopted` 算。
            let naming: Vec<String> = entries
                .iter()
                .filter(|entry| {
                    entry
                        .files
                        .iter()
                        .any(|named| named.trim() == file.relative)
                })
                .map(|entry| entry.anchor.clone())
                .collect();
            if naming.is_empty() {
                lines.push(if entries.is_empty() {
                    "  adoption   no ledger at .nichlink/adopted/entries in this root".to_owned()
                } else {
                    format!(
                        "  adoption   {} entry(ies) in the ledger, none naming this file",
                        entries.len()
                    )
                });
            } else {
                lines.push(format!(
                    "  adoption   {} entry(ies) name this file — anchors: {} (read the verdict with \
                     `adopted`)",
                    naming.len(),
                    naming.join(", ")
                ));
            }
        }
    }

    lines.push(
        "not covered here: whether this definition is in the published tree and how its slot is \
         wired (ask `registry`/`grafts`, or `check {face}` to run the face), whether a feature gates \
         it (ask `check {face}` with that feature), and any runtime evidence (ask `trace`)"
            .to_owned(),
    );
    lines.push(
        "next   `check {face}` to run the face that compiles it, `locate {symptom}` for sibling \
         places to compare it with"
            .to_owned(),
    );
    Ok(format!("{}\n", lines.join("\n")))
}
