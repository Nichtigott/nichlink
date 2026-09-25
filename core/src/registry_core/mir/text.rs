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
    ///
    /// This never fails and never reports: text that is not MIR yields an empty
    /// graph, and a graph with no functions is therefore the same answer for "there
    /// is nothing here" and "this is not MIR at all". Callers that must tell those
    /// apart check the input themselves — the command that produces MIR already
    /// knows whether `cargo rustc` succeeded — and this is the documented boundary
    /// rather than a defect to be discovered later.
    /// 本函数从不失败也从不报告：不是 MIR 的文本会得到一个空图，因此"这里什么都没有"与"这根本
    /// 不是 MIR"给出同一个答案。必须区分两者的调用方自己检查输入——产出 MIR 的那条命令本来
    /// 就知道 `cargo rustc` 是否成功——这是**写明的**边界，而不是留给以后发现的缺陷。
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

    /// Text that is not MIR yields an empty graph — the documented contract, not an
    /// error. Pinned so that turning this into a `Result` has to update the test
    /// and the doc comment together.
    /// 不是 MIR 的文本得到空图——这是写明的契约，不是错误。钉住它，使把它改成 `Result` 的人
    /// 必须连同这条测试与那段文档注释一起更新。
    #[test]
    fn text_that_is_not_mir_yields_an_empty_graph() {
        let graph = MirGraph::from_mir_text("this is not MIR at all\n");
        assert!(graph.functions.is_empty());
        assert!(graph.locals.is_empty());
    }

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
