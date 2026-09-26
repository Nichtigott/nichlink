//! The static call-graph answer, and the bounds it needs.
//! 静态调用图答案，以及它需要的上限。
//!
//! Lifted out of `tools.rs` when a new tool pushed that page into the 450-line
//! ratchet: the catalog and the query implementations belong there, the one answer
//! that grows without limit belongs here, next to its two bounds and their tests.
//! 当一个新工具把 `tools.rs` 推到 450 行棘轮边上时把它挪出来：目录与查询实现留在那里，而这唯一会
//! 无界增长的答案连同它的两道上限与测试放在这里。
//!
//! The measured failure these bounds exist against: a common name (`new`) had 142
//! definitions in the NichUI corpus, and every call site of that name was listed for
//! each of them, which arrived as a 4.5 MB reply. An answer an agent cannot read is
//! not an answer.
//! 这些上限所针对的实测失败：常见名（`new`）在 NichUI 语料里有 142 个定义，而每个定义都列出该名字
//! 的每一个调用点，最终以 4.5 MB 的回复抵达。代理读不下的答案不算答案。

use std::path::Path;

use serde_json::Value;

use crate::index::{display_list, load_sources};

pub(crate) fn callgraph(root: &Path, arguments: &Value) -> Result<String, String> {
    // Two bounds, because this answer is the one that grows without limit: a
    // common name like `new` had 151 definitions and every call site of the name
    // in the tree, which arrived as a 4.5 MB reply. Definitions and callers are
    // capped separately, and both say how much they withheld.
    // 两道上限，因为这是唯一会无界增长的答案：像 `new` 这样的常见名有 151 个定义、外加树里该名字
    // 的每一个调用点，曾以 4.5 MB 的回复抵达。定义数与调用者各自设上限，且都说出自己扣下了多少。
    const DEFINITIONS: usize = 5;
    const CALLERS: usize = 20;
    let query = arguments
        .get("function")
        .and_then(Value::as_str)
        .ok_or_else(|| "nichlink.callgraph requires function".to_owned())?
        .trim();
    if query.is_empty() {
        return Err("function must not be empty".to_owned());
    }
    let path_filter = arguments.get("path").and_then(Value::as_str);
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFINITIONS, |value| value.clamp(1, 50) as usize);
    let files = load_sources(root)?;
    let mut found = Vec::new();
    for file in &files {
        if path_filter.is_some_and(|path| file.relative != path) {
            continue;
        }
        for function in &file.functions {
            if function.name == query || function.name.ends_with(&format!("::{query}")) {
                found.push((file, function));
            }
        }
    }
    if found.is_empty() {
        return Ok(format!("no static function match for `{query}`"));
    }
    let total = found.len();
    let mut output = format!("evidence: static-heuristic\nmatches {total}\n");
    if total > 1 && path_filter.is_none() {
        // Naming the ambiguity is the difference between a usable answer and a
        // dump: with several definitions the reader has to choose, and `path` is
        // how they choose.
        // 把歧义说出来，是好答案与一坨倾倒之间的区别：有多个定义时读取方必须选一个，而 `path`
        // 就是他们选的工具。
        output.push_str(&format!(
            "note: {total} definitions match `{query}`; pass `path` to select one. Callers are matched \
             by name across the whole tree, so for a common name they include unrelated call sites.\n"
        ));
    }
    for (file, function) in found.into_iter().take(limit) {
        let mut callers = files
            .iter()
            .flat_map(|candidate| {
                candidate.functions.iter().filter_map(|caller| {
                    caller
                        .calls
                        .iter()
                        .any(|call| call == &function.name || call.ends_with(&format!("::{query}")))
                        .then_some(format!("{}::{}", candidate.relative, caller.name))
                })
            })
            .collect::<Vec<_>>();
        callers.sort();
        callers.dedup();
        let callers_total = callers.len();
        let callers_text = if callers_total > CALLERS {
            format!(
                "{} … +{} more",
                callers[..CALLERS].join(", "),
                callers_total - CALLERS
            )
        } else {
            display_list(&callers)
        };
        output.push_str(&format!(
            "{}:{} fn {}\n  callers ({}): {}\n  callees: {}\n",
            file.relative,
            function.line,
            function.name,
            callers_total,
            callers_text,
            display_list(&function.calls),
        ));
    }
    if total > limit {
        output.push_str(&format!(
            "… +{} more definitions (raise `limit` or pass `path`)\n",
            total - limit
        ));
    }
    output.push_str("dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.\n");
    Ok(output)
}

#[cfg(test)]
#[path = "callgraph_tests.rs"]
mod callgraph_tests;
