//! Tool catalog and the five read-only query implementations.
//! 工具目录与五个只读查询实现。
//!
//! TODO(registry-tool): every tool here reads Rust source text. A registry or
//! contract tool would need a different resource: load the host's generated
//! face snapshots (or a serialized `Registry`), run
//! `Registry::register_snapshot_batch`, and report `RegistrationSnapshot`
//! fields such as contract, admission, and registration rule. That requires
//! the authoring/build context, not a lexical source scan, and is deferred.
//! TODO(registry-tool)：这里的每个工具都只读取 Rust 源码文本。若要提供注册树或
//! 合同工具，需要另一种资源：载入宿主生成的 face 快照（或序列化的 `Registry`），
//! 调用 `Registry::register_snapshot_batch`，再报告 `RegistrationSnapshot` 的
//! contract、admission、registration rule 等字段。这需要 authoring/build 上下文，
//! 不是词法源码扫描，因此推迟实现。

use serde_json::{Value, json};
use std::path::Path;

use crate::index::{display_list, load_one, load_sources, required_path, resolve_root};
use crate::protocol::{DEFAULT_LIMIT, MAX_READ_LINES, error_response, success};

pub(crate) fn tools() -> Vec<Value> {
    vec![
        tool(
            "nichlink.search",
            "Find source files and Rust function declarations by name.",
            json!({"type":"object","properties":{"query":{"type":"string"},"root":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200}},"required":["query"]}),
        ),
        tool(
            "nichlink.inspect",
            "Summarize functions and registration declarations in one Rust file.",
            json!({"type":"object","properties":{"path":{"type":"string"},"root":{"type":"string"}},"required":["path"]}),
        ),
        tool(
            "nichlink.callgraph",
            "Show direct static callers and callees for one function.",
            json!({"type":"object","properties":{"function":{"type":"string"},"path":{"type":"string"},"root":{"type":"string"}},"required":["function"]}),
        ),
        tool(
            "nichlink.read",
            "Read a bounded source window around a line.",
            json!({"type":"object","properties":{"path":{"type":"string"},"line":{"type":"integer","minimum":1},"context":{"type":"integer","minimum":0,"maximum":120},"root":{"type":"string"}},"required":["path"]}),
        ),
        tool(
            "nichlink.status",
            "Report the source root and indexed Rust file/function counts.",
            json!({"type":"object","properties":{"root":{"type":"string"}}}),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({ "name": name, "description": description, "inputSchema": input_schema })
}

pub(crate) fn tool_call(root: &Path, id: Value, params: &Value) -> Value {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return error_response(id, -32602, "tools/call requires name".to_owned());
    };
    let arguments = params.get("arguments").unwrap_or(&Value::Null);
    let requested_root = arguments.get("root").and_then(Value::as_str);
    let root = match resolve_root(root, requested_root) {
        Ok(root) => root,
        Err(error) => {
            return success(
                id,
                json!({ "content": [{"type":"text","text":error}], "isError": true }),
            );
        }
    };
    let result = match name {
        "nichlink.search" => search(&root, arguments),
        "nichlink.inspect" => inspect(&root, arguments),
        "nichlink.callgraph" => callgraph(&root, arguments),
        "nichlink.read" => read_source(&root, arguments),
        "nichlink.status" => status(&root),
        _ => Err(format!("unknown tool `{name}`")),
    };
    match result {
        Ok(value) => success(
            id,
            json!({ "content": [{"type":"text","text":value}], "isError": false }),
        ),
        Err(error) => success(
            id,
            json!({ "content": [{"type":"text","text":error}], "isError": true }),
        ),
    }
}

fn search(root: &Path, arguments: &Value) -> Result<String, String> {
    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| "nichlink.search requires query".to_owned())?
        .trim()
        .to_ascii_lowercase();
    if query.is_empty() {
        return Err("query must not be empty".to_owned());
    }
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize);
    let files = load_sources(root)?;
    let mut results = Vec::new();
    for file in &files {
        if file.relative.to_ascii_lowercase().contains(&query) {
            results.push(format!("file  {}", file.relative));
        }
        for function in &file.functions {
            if function.name.to_ascii_lowercase().contains(&query) {
                results.push(format!(
                    "fn    {} -> {}:{}",
                    function.name, file.relative, function.line
                ));
            }
        }
        if results.len() >= limit {
            break;
        }
    }
    if results.is_empty() {
        return Ok("no matches".to_owned());
    }
    results.truncate(limit);
    Ok(results.join("\n"))
}

fn inspect(root: &Path, arguments: &Value) -> Result<String, String> {
    let relative = required_path(arguments)?;
    let file = load_one(root, &relative)?;
    let mut output = format!("file {}\n", file.relative);
    for function in &file.functions {
        output.push_str(&format!(
            "fn {} lines {}-{} calls=[{}]\n",
            function.name,
            function.line,
            function.end_line,
            function.calls.join(", ")
        ));
    }
    let registrations = nichlink::source::registration_kinds(&file.source);
    if !registrations.is_empty() {
        output.push_str("registrations: ");
        output.push_str(&registrations.join(", "));
        output.push('\n');
    }
    if file.functions.is_empty() && registrations.is_empty() {
        output.push_str("no function or registration declaration found\n");
    }
    Ok(output)
}

fn callgraph(root: &Path, arguments: &Value) -> Result<String, String> {
    let query = arguments
        .get("function")
        .and_then(Value::as_str)
        .ok_or_else(|| "nichlink.callgraph requires function".to_owned())?
        .trim();
    if query.is_empty() {
        return Err("function must not be empty".to_owned());
    }
    let files = load_sources(root)?;
    let mut found = Vec::new();
    for file in &files {
        if arguments
            .get("path")
            .and_then(Value::as_str)
            .is_some_and(|path| file.relative != path)
        {
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
    let mut output = String::from("evidence: static-heuristic\n");
    for (file, function) in found {
        let callers = files
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
        output.push_str(&format!(
            "{}:{} fn {}\n  callers: {}\n  callees: {}\n",
            file.relative,
            function.line,
            function.name,
            display_list(&callers),
            display_list(&function.calls),
        ));
    }
    output.push_str("dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.\n");
    Ok(output)
}

fn read_source(root: &Path, arguments: &Value) -> Result<String, String> {
    let relative = required_path(arguments)?;
    let file = load_one(root, &relative)?;
    let total = file.source.lines().count();
    let center = arguments
        .get("line")
        .and_then(Value::as_u64)
        .map_or(1, |line| line.max(1) as usize);
    let context = arguments
        .get("context")
        .and_then(Value::as_u64)
        .map_or(40, |value| value.min(120) as usize);
    let start = center.saturating_sub(context).max(1);
    let end = (center + context)
        .min(total.max(1))
        .min(start + MAX_READ_LINES - 1);
    let mut output = format!("{}:{}-{}\n", file.relative, start, end);
    for (index, line) in file.source.lines().enumerate() {
        let line_number = index + 1;
        if (start..=end).contains(&line_number) {
            output.push_str(&format!("{line_number:>5} | {line}\n"));
        }
    }
    Ok(output)
}

fn status(root: &Path) -> Result<String, String> {
    let files = load_sources(root)?;
    let functions = files.iter().map(|file| file.functions.len()).sum::<usize>();
    Ok(format!(
        "root {}\nrust_files={} functions={} tool=nichlink-mcp",
        root.display(),
        files.len(),
        functions
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_list_is_mcp_shaped() {
        let listed = tools();
        assert!(listed.iter().any(|tool| tool["name"] == "nichlink.search"));
        assert!(
            listed
                .iter()
                .all(|tool| tool["inputSchema"]["type"] == "object")
        );
    }
}
