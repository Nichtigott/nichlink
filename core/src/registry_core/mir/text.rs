//! Textual MIR parser for `rustc -Zunpretty=mir` output.
//! `rustc -Zunpretty=mir` 输出的文本 MIR 解析器。

use super::model::{MirCall, MirGraph, MirLocal};

impl MirGraph {
    /// Parse textual MIR emitted by rustc -Zunpretty=mir.
    /// 解析 rustc -Zunpretty=mir 输出的文本。
    ///
    /// Only function, local, and direct-call records are kept. Runtime
    /// traces remain authoritative for calls that actually executed.
    /// 这里只保留函数、局部变量和直接调用；真实执行的调用仍以运行期追踪为准。
    pub fn from_mir_text(input: &str) -> Self {
        let mut graph = Self::default();
        let mut function = String::new();
        for raw in input.lines() {
            let trimmed = raw.trim();
            if let Some(name) = mir_function_name(trimmed) {
                function = name.to_owned();
                graph.functions.insert(function.clone());
                continue;
            }
            if function.is_empty() {
                continue;
            }
            if let Some(local) = mir_local(trimmed, &function, graph.locals.len() + 1) {
                graph.locals.push(local);
            }
            if let Some(callee) = mir_call(trimmed) {
                graph.calls.push(MirCall {
                    caller: function.clone(),
                    callee,
                    mir_line: graph.calls.len() + 1,
                });
            }
        }
        graph
    }
}

fn mir_function_name(line: &str) -> Option<&str> {
    let line = line.strip_prefix("fn ")?;
    let end = line.find('(')?;
    let name = line[..end].trim();
    (!name.is_empty()).then_some(name)
}

fn mir_local(line: &str, function: &str, mir_line: usize) -> Option<MirLocal> {
    let line = line.strip_prefix("let ")?;
    let line = line.strip_prefix("mut ").unwrap_or(line);
    let (name, type_name) = line.split_once(':')?;
    let name = name.trim();
    let type_name = type_name.trim().trim_end_matches(';').trim();
    (name.starts_with('_') && !type_name.is_empty()).then(|| MirLocal {
        function: function.to_owned(),
        name: name.to_owned(),
        type_name: type_name.to_owned(),
        mir_line,
    })
}

fn mir_call(line: &str) -> Option<String> {
    let (_, expression) = line.split_once(" = ")?;
    let open = expression.find('(')?;
    let callee = expression[..open].trim().trim_start_matches("move ");
    if callee.is_empty()
        || callee.starts_with("if ")
        || callee.starts_with("match ")
        || matches!(
            callee,
            "assert" | "debug_assert" | "drop" | "panic" | "format" | "Some" | "Ok" | "Err"
        )
    {
        return None;
    }
    Some(callee.to_owned())
}

#[cfg(test)]
mod tests {
    use super::MirGraph;

    #[test]
    fn parses_native_textual_mir() {
        let graph = MirGraph::from_mir_text(
            "fn crate::outer(_1: f32) -> f32 {\n    let mut _2: f32;\n    _2 = crate::inner(move _1);\n    return;\n}\n",
        );
        assert!(graph.functions.contains("crate::outer"));
        assert_eq!(graph.calls[0].callee, "crate::inner");
        assert_eq!(graph.locals[0].name, "_2");
    }
}
