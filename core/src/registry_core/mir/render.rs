//! MIR artifact rendering: the human report and the compact JSONL artifact.
//! MIR artifact 渲染：人类可读报告与紧凑 JSONL artifact。

use std::fmt::Write as _;

use crate::json::json_string;

use super::model::MirGraph;

impl MirGraph {
    /// Render the human-readable candidate report, with one line per call and
    /// per local.
    /// 渲染人类可读的候选报告，每条调用与每个局部变量各一行。
    pub fn render(&self) -> String {
        let mut output = String::new();
        writeln!(output, "MIR CANDIDATES").unwrap();
        for call in &self.calls {
            writeln!(
                output,
                "  {} -> {} @ MIR line {}",
                call.caller, call.callee, call.mir_line
            )
            .unwrap();
        }
        if self.calls.is_empty() {
            output.push_str("  (no static call candidates)\n");
        }
        writeln!(output, "LOCALS").unwrap();
        for local in &self.locals {
            writeln!(
                output,
                "  {}::{}: {} @ MIR line {}",
                local.function, local.name, local.type_name, local.mir_line
            )
            .unwrap();
        }
        output
    }

    /// Emit a compact, line-oriented artifact that build tooling can consume.
    /// 输出构建工具可消费的紧凑逐行 artifact。
    ///
    /// Every string goes through the workspace's one JSON encoder. A local
    /// `replace`-based copy used to sit here, and it emitted a raw tab or newline
    /// for a name that contained one: the record stayed readable to this crate's
    /// lenient parser and became invalid for every strict one. The pinning test
    /// is `a_control_character_in_a_name_stays_escaped`.
    /// 每个字符串都经过工作区唯一的 JSON 编码器。这里以前有一份基于 `replace` 的本地副本，
    /// 遇到含制表符或换行的名字时会原样写出：记录对本 crate 的宽松解析器仍然可读，对任何严格
    /// 解析器都非法。钉住它的是 `a_control_character_in_a_name_stays_escaped`。
    pub fn to_jsonl(&self) -> String {
        let mut output = String::new();
        for function in &self.functions {
            writeln!(
                output,
                "{{\"kind\":\"function\",\"name\":{}}}",
                json_string(function)
            )
            .unwrap();
        }
        for call in &self.calls {
            writeln!(
                output,
                "{{\"kind\":\"call\",\"caller\":{},\"callee\":{},\"mir_line\":{}}}",
                json_string(&call.caller),
                json_string(&call.callee),
                call.mir_line
            )
            .unwrap();
        }
        for local in &self.locals {
            writeln!(
                output,
                "{{\"kind\":\"local\",\"function\":{},\"name\":{},\"type\":{},\"mir_line\":{}}}",
                json_string(&local.function),
                json_string(&local.name),
                json_string(&local.type_name),
                local.mir_line
            )
            .unwrap();
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::MirGraph;

    #[test]
    fn jsonl_artifact_round_trips() {
        let graph = MirGraph::from_jsonl(
            "{\"kind\":\"function\",\"name\":\"crate::a\"}\n{\"kind\":\"call\",\"caller\":\"crate::a\",\"callee\":\"crate::b\",\"mir_line\":3}\n",
        )
        .unwrap();
        let parsed = MirGraph::from_jsonl(&graph.to_jsonl()).unwrap();
        assert_eq!(parsed, graph);
    }

    /// The artifact must stay valid for a strict reader, which is the whole point
    /// of JSONL. This crate's own parser is lenient, so a round trip cannot show
    /// the difference; asserting the absence of raw control characters can.
    /// 工件必须对严格读取器保持合法，这正是 JSONL 的意义。本 crate 自己的解析器很宽松，
    /// 因此往返看不出差别；断言"不存在原样控制字符"可以。
    #[test]
    fn a_control_character_in_a_name_stays_escaped() {
        let graph = MirGraph::from_jsonl(
            "{\"kind\":\"function\",\"name\":\"a\\tb\"}\n{\"kind\":\"call\",\"caller\":\"a\\tb\",\"callee\":\"c\",\"mir_line\":1}\n",
        )
        .unwrap();
        let jsonl = graph.to_jsonl();
        // The newline *between* records is the format; a control character
        // *inside* a record is the defect, so the check is per line.
        // 记录之间的换行是格式本身；记录**内部**的控制字符才是缺陷，因此逐行检查。
        for line in jsonl.lines() {
            assert!(
                !line.chars().any(|character| character < '\u{20}'),
                "a raw control character makes the line invalid for a strict reader: {line:?}"
            );
        }
        assert!(jsonl.contains(r#""name":"a\tb""#), "{jsonl:?}");
    }
}
