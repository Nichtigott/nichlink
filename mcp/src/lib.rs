use serde_json::{Value, json};
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

const PROTOCOL_VERSION: &str = "2025-06-18";
const DEFAULT_LIMIT: usize = 40;
const MAX_READ_LINES: usize = 240;

#[derive(Clone, Debug)]
struct Function {
    name: String,
    line: usize,
    end_line: usize,
    calls: Vec<String>,
}

#[derive(Clone, Debug)]
struct SourceFile {
    relative: String,
    source: String,
    functions: Vec<Function>,
}

/// Run the read-only MCP stdio bridge until stdin closes or the transport
/// fails.
/// 运行只读 MCP stdio 桥，直到 stdin 关闭或传输失败。
pub fn run() {
    let root = env::var_os("NICH_LINK_PACKAGE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("current directory"));
    let stdin = io::stdin();
    let mut output = io::BufWriter::new(io::stdout().lock());
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => dispatch(&root, &request),
            Err(error) => error_response(Value::Null, -32700, format!("invalid JSON: {error}")),
        };
        if response.is_null() {
            continue;
        }
        if serde_json::to_writer(&mut output, &response).is_err() {
            break;
        }
        if output.write_all(b"\n").is_err() || output.flush().is_err() {
            break;
        }
    }
}

fn dispatch(root: &Path, request: &Value) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let params = request.get("params").unwrap_or(&Value::Null);
    match method {
        "initialize" => success(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "nichlink-mcp", "version": env!("CARGO_PKG_VERSION") },
                "instructions": "Use nichlink.search before reading source; callgraph is static-heuristic."
            }),
        ),
        "notifications/initialized" | "notifications/cancelled" => Value::Null,
        "ping" => success(id, json!({})),
        "tools/list" => success(id, json!({ "tools": tools() })),
        "tools/call" => tool_call(root, id, params),
        _ => error_response(id, -32601, format!("unknown method `{method}`")),
    }
}

fn tools() -> Vec<Value> {
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

fn tool_call(root: &Path, id: Value, params: &Value) -> Value {
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

fn required_path(arguments: &Value) -> Result<String, String> {
    arguments
        .get("path")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "path is required".to_owned())
}

fn load_sources(root: &Path) -> Result<Vec<SourceFile>, String> {
    if !root.is_dir() {
        return Err(format!("source root does not exist: {}", root.display()));
    }
    let mut paths = Vec::new();
    collect_rs(root, &mut paths)?;
    paths.sort();
    paths
        .into_iter()
        .map(|path| load_file(root, &path))
        .collect()
}

fn collect_rs(directory: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read directory entry: {error}"))?;
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some("target") {
            continue;
        }
        if path.is_dir() {
            collect_rs(&path, paths)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            paths.push(path);
        }
    }
    Ok(())
}

fn load_one(root: &Path, relative: &str) -> Result<SourceFile, String> {
    let path = root.join(relative);
    if !is_safe_child(root, &path) {
        return Err("path must stay inside the configured source root".to_owned());
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
        return Err("only Rust source files can be read".to_owned());
    }
    load_file(root, &path)
}

fn load_file(root: &Path, path: &Path) -> Result<SourceFile, String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let relative = path
        .strip_prefix(root)
        .map_err(|_| "source path escaped root".to_owned())?
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    Ok(SourceFile {
        functions: parse_functions(&source),
        relative,
        source,
    })
}

fn is_safe_child(root: &Path, path: &Path) -> bool {
    let root = fs::canonicalize(root).ok();
    let path = fs::canonicalize(path).ok();
    match (root, path) {
        (Some(root), Some(path)) => path.starts_with(root),
        _ => false,
    }
}

fn resolve_root(base: &Path, requested: Option<&str>) -> Result<PathBuf, String> {
    let base = fs::canonicalize(base)
        .map_err(|error| format!("source root does not exist: {} ({error})", base.display()))?;
    let candidate = requested.map_or_else(|| base.clone(), |value| base.join(value));
    let candidate = fs::canonicalize(&candidate).map_err(|error| {
        format!(
            "requested root is not readable: {} ({error})",
            candidate.display()
        )
    })?;
    if !candidate.starts_with(&base) {
        return Err("requested root must stay inside NICH_LINK_PACKAGE_ROOT".to_owned());
    }
    if !candidate.is_dir() {
        return Err(format!(
            "requested root is not a directory: {}",
            candidate.display()
        ));
    }
    Ok(candidate)
}

fn parse_functions(source: &str) -> Vec<Function> {
    nichlink::source::function_symbols(source)
        .into_iter()
        .map(|function| Function {
            calls: nichlink::source::direct_calls(&function.body, &function.name),
            name: function.name,
            line: function.line as usize,
            end_line: function.end_line as usize,
        })
        .collect()
}

fn display_list(items: &[String]) -> String {
    if items.is_empty() {
        "-".to_owned()
    } else {
        items.join(", ")
    }
}

fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i64, message: String) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_indexes_functions_and_direct_calls() {
        let functions = parse_functions(
            "fn source() { let value = helper(1); sink(value); }\nfn helper(_: i32) {}\nfn sink(_: i32) {}",
        );
        assert_eq!(functions.len(), 3);
        assert_eq!(functions[0].name, "source");
        assert_eq!(functions[0].calls, ["helper", "sink"]);
    }

    #[test]
    fn registration_kinds_are_compact_and_deduplicated() {
        let kinds = nichlink::source::registration_kinds(
            "crate::control_object! { kind: Button, }\ncrate::control_object! { kind: Button, }",
        );
        assert_eq!(kinds, ["Button"]);
    }

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
