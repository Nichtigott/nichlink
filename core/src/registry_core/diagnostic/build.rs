use super::*;

// The one JSON string encoder in the workspace; the MIR JSONL artifact and the
// generated editor snippets call the same function, so the three cannot drift.
// 工作区里唯一的 JSON 字符串编码器；MIR JSONL 工件与生成的编辑器片段调用同一个函数，
// 因此三者不会漂移。
use crate::json::push_json_string;

// ---------------------------------------------------------------------------
// Build-phase diagnostics. Pure data plus in-memory rendering; the build
// surface collects them, the kernel owns the model.
// 构建期诊断。纯数据与内存渲染；由 build 面收集，模型归 kernel 所有。

/// One declaration error before it is rendered for rustc or a terminal.
/// 在输出给 rustc 或终端之前的一条声明错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildDiagnostic {
    /// Build phase that raised this diagnostic; empty means the phase is not
    /// applicable to this diagnostic.
    /// 提出该诊断的构建阶段；为空表示该阶段不适用于此诊断。
    pub phase: &'static str,
    /// Graft branch the failure was reported under; empty means not applicable.
    /// 报告该失败时所在的 graft 分支；为空表示不适用。
    pub branch: String,
    /// Registration face identity rendered as `name (kind)`; empty means not
    /// applicable.
    /// 渲染为 `name (kind)` 的注册面身份；为空表示不适用。
    pub node: String,
    /// Declaration source file carrying the failure; empty means not applicable.
    /// 携带该失败的声明源文件；为空表示不适用。
    pub source: String,
    /// 1-based line inside `source`; 0 when the diagnostic has no line.
    /// `source` 内以 1 起始的行号；诊断没有行号时为 0。
    pub line: usize,
    /// Logical function or handle the failure is about; empty means not
    /// applicable.
    /// 失败所针对的逻辑函数或 handle；为空表示不适用。
    pub function: String,
    /// Declaration field that failed its check; empty means not applicable.
    /// 未通过检查的声明字段；为空表示不适用。
    pub field: String,
    /// Value the contract required; empty means not applicable.
    /// 契约要求的值；为空表示不适用。
    pub expected: String,
    /// Value actually observed; empty means not applicable.
    /// 实际观测到的值；为空表示不适用。
    pub actual: String,
    /// Kind of ancestor expected to provide the missing capability; empty means
    /// not applicable.
    /// 本应提供缺失能力的祖先类型；为空表示不适用。
    pub provider: String,
    /// Human-readable failure summary; empty means no message applies.
    /// 人类可读的失败摘要；为空表示没有消息。
    pub message: String,
}

impl BuildDiagnostic {
    /// Start a diagnostic in `phase` with its human-readable message.
    /// 以 `phase` 和人类可读消息开启一条诊断。
    ///
    /// Every optional field starts empty, so the builder methods below attach
    /// only the facts this diagnostic actually has.
    /// 所有可选字段起始为空，由下面的 builder 方法只挂上该诊断真正具有的事实。
    pub fn new(phase: &'static str, message: impl Into<String>) -> Self {
        Self {
            phase,
            branch: String::new(),
            node: String::new(),
            source: String::new(),
            line: 0,
            function: String::new(),
            field: String::new(),
            expected: String::new(),
            actual: String::new(),
            provider: String::new(),
            message: message.into(),
        }
    }

    /// Attach the declaration source and its 1-based line; pass `0` for no line.
    /// 挂上声明源码与以 1 起始的行号；没有行号时传 `0`。
    pub fn at(mut self, source: impl Into<String>, line: usize) -> Self {
        self.source = source.into();
        self.line = line;
        self
    }

    /// Attach the failing face identity and kind, stored together as `node (kind)`.
    /// 挂上失败注册面的身份与类型，合并存储为 `node (kind)`。
    pub fn node(mut self, node: impl Into<String>, kind: impl Into<String>) -> Self {
        self.node = format!("{} ({})", node.into(), kind.into());
        self
    }

    /// Attach the graft branch the failure belongs to.
    /// 挂上该失败所属的 graft 分支。
    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = branch.into();
        self
    }

    /// Attach the logical function or handle the failure is about.
    /// 挂上该失败所针对的逻辑函数或 handle。
    pub fn function(mut self, function: impl Into<String>) -> Self {
        self.function = function.into();
        self
    }

    /// Attach the declaration field that failed its check.
    /// 挂上未通过检查的声明字段。
    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.field = field.into();
        self
    }

    /// Attach the value the contract required.
    /// 挂上契约要求的值。
    pub fn expected(mut self, expected: impl Into<String>) -> Self {
        self.expected = expected.into();
        self
    }

    /// Attach the value actually observed.
    /// 挂上实际观测到的值。
    pub fn actual(mut self, actual: impl Into<String>) -> Self {
        self.actual = actual.into();
        self
    }

    /// Attach the kind of ancestor expected to provide the missing capability.
    /// 挂上本应提供缺失能力的祖先类型。
    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = provider.into();
        self
    }
}

/// Sorted, deduplicated collection of [`BuildDiagnostic`]s.
/// 排序去重后的 [`BuildDiagnostic`] 集合。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BuildDiagnostics {
    items: Vec<BuildDiagnostic>,
}

impl BuildDiagnostics {
    /// Append one diagnostic; duplicates are kept here and collapsed only by
    /// [`BuildDiagnostics::iter`], `len`, and `render`.
    /// 追加一条诊断；重复项在此保留，仅由 [`BuildDiagnostics::iter`]、`len` 与 `render`
    /// 折叠。
    pub fn push(&mut self, diagnostic: BuildDiagnostic) {
        self.items.push(diagnostic);
    }

    /// Move every diagnostic of `other` into this collection.
    /// 把 `other` 的全部诊断移入本集合。
    pub fn extend(&mut self, other: Self) {
        self.items.extend(other.items);
    }

    /// Whether nothing has been collected; true exactly when `len` is `0`.
    /// 是否尚未收集到任何诊断；当且仅当 `len` 为 `0` 时为真。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Iterate the diagnostics in the exact order [`BuildDiagnostics::render`]
    /// prints them, with byte-identical duplicates collapsed.
    /// 按 [`BuildDiagnostics::render`] 打印它们的完全相同顺序迭代诊断，并折叠完全相同的
    /// 重复项。
    ///
    /// The read surface and the human renderer must agree: a CI job that counts
    /// `iter()` items while a developer reads `render()` has to see the same
    /// set, or the two disagree about whether "the build is clean". Sorting and
    /// deduplicating here (rather than once per caller) is what keeps them from
    /// drifting; `render` itself is written in terms of this iterator.
    /// 读取面与人类可读渲染必须一致：CI 数 `iter()` 的条目、开发者读 `render()`，
    /// 两者必须看到同一集合，否则会对"构建是否干净"给出不同答案。把排序与去重放在
    /// 这里（而不是每个调用方各做一遍）正是防止两者漂移的办法；`render` 本身就以
    /// 这个迭代器书写。
    pub fn iter(&self) -> impl Iterator<Item = &BuildDiagnostic> {
        let mut items = self.items.iter().collect::<Vec<_>>();
        items.sort_by(|left, right| {
            (
                left.phase,
                &left.source,
                left.line,
                &left.node,
                &left.message,
            )
                .cmp(&(
                    right.phase,
                    &right.source,
                    right.line,
                    &right.node,
                    &right.message,
                ))
        });
        items.dedup();
        items.into_iter()
    }

    /// The number of distinct diagnostics [`BuildDiagnostics::render`] would
    /// print.
    /// [`BuildDiagnostics::render`] 会打印的去重后诊断条数。
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// Render all distinct diagnostics as one human-readable report, or the
    /// empty string when there are none.
    /// 把所有去重后的诊断渲染为一份人类可读报告；没有诊断时返回空字符串。
    pub fn render(&self) -> String {
        if self.items.is_empty() {
            return String::new();
        }
        let mut output = String::from("NICHLink BUILD CHECK FAILED / NichLink 构建检查失败\n");
        for (index, diagnostic) in self.iter().enumerate() {
            if index > 0 {
                output.push('\n');
            }
            render_build_item(&mut output, diagnostic);
        }
        output
    }

    /// Serialize the diagnostics as one JSON document for a machine reader.
    /// 把诊断序列化为供机器读取的单个 JSON 文档。
    ///
    /// Hand-rolled rather than `serde_json`: the kernel deliberately has no
    /// serialization dependency (AGENTS.md rule 3 keeps the kernel pure, and
    /// `serde_json` is a `cli`/`mcp` dependency only), and the shape is eleven
    /// flat string/integer fields. Adding a derive to the kernel for that would
    /// put a new public dependency on the crate every host links.
    /// 手写而不是用 `serde_json`：内核刻意不带序列化依赖（AGENTS.md 规则 3 要求内核
    /// 保持纯净，`serde_json` 只是 `cli`/`mcp` 的依赖），而形状只有十一个扁平字符串/
    /// 整数字段。为此在内核上加 derive，等于给每个宿主都会链接的 crate 加一个新公开
    /// 依赖。
    ///
    /// Every key is always present, so a reader never has to distinguish "absent"
    /// from "empty"; `line` is `0` when the diagnostic has no line. The order is
    /// the same as [`BuildDiagnostics::iter`], so the JSON and the rendered text
    /// list the same diagnostics in the same order.
    /// 所有键始终存在，读取方无需区分"缺失"与"空"；诊断没有行号时 `line` 为 `0`。
    /// 顺序与 [`BuildDiagnostics::iter`] 相同，因此 JSON 与渲染文本列出的是同一批
    /// 诊断、同一顺序。
    pub fn to_json(&self) -> String {
        let mut output = String::from("{\"schema\":\"nichlink.build-diagnostics/1\",\"count\":");
        output.push_str(&self.len().to_string());
        output.push_str(",\"diagnostics\":[");
        for (index, diagnostic) in self.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str("{\"phase\":");
            push_json_string(&mut output, diagnostic.phase);
            output.push_str(",\"branch\":");
            push_json_string(&mut output, &diagnostic.branch);
            output.push_str(",\"node\":");
            push_json_string(&mut output, &diagnostic.node);
            output.push_str(",\"source\":");
            push_json_string(&mut output, &diagnostic.source);
            output.push_str(",\"line\":");
            output.push_str(&diagnostic.line.to_string());
            output.push_str(",\"function\":");
            push_json_string(&mut output, &diagnostic.function);
            output.push_str(",\"field\":");
            push_json_string(&mut output, &diagnostic.field);
            output.push_str(",\"expected\":");
            push_json_string(&mut output, &diagnostic.expected);
            output.push_str(",\"actual\":");
            push_json_string(&mut output, &diagnostic.actual);
            output.push_str(",\"provider\":");
            push_json_string(&mut output, &diagnostic.provider);
            output.push_str(",\"message\":");
            push_json_string(&mut output, &diagnostic.message);
            output.push('}');
        }
        output.push_str("]}");
        output
    }
}

fn render_build_item(output: &mut String, diagnostic: &BuildDiagnostic) {
    let phase = match diagnostic.phase {
        "requirements" => "requirements / 注册需求",
        "contract" => "contract / 注册合同",
        "stable-identity" => "stable identity / 稳定标识",
        "static-plan" => "static plan / 静态计划",
        other => other,
    };
    writeln!(
        output,
        "+-- phase={phase} branch={}",
        unknown(&diagnostic.branch)
    )
    .unwrap();
    if !diagnostic.node.is_empty() {
        writeln!(output, "|   node={}", diagnostic.node).unwrap();
    }
    if !diagnostic.source.is_empty() {
        if diagnostic.line == 0 {
            writeln!(output, "|   source={}", diagnostic.source).unwrap();
        } else {
            writeln!(
                output,
                "|   source={}:{}",
                diagnostic.source, diagnostic.line
            )
            .unwrap();
        }
    }
    if !diagnostic.function.is_empty() {
        writeln!(output, "|   function={}", diagnostic.function).unwrap();
    }
    if !diagnostic.field.is_empty() {
        writeln!(output, "|   field={}", diagnostic.field).unwrap();
    }
    if !diagnostic.expected.is_empty() {
        writeln!(output, "|   expected={}", diagnostic.expected).unwrap();
    }
    if !diagnostic.actual.is_empty() {
        writeln!(output, "|   actual={}", diagnostic.actual).unwrap();
    }
    if !diagnostic.provider.is_empty() {
        writeln!(output, "|   provider={}", diagnostic.provider).unwrap();
    }
    writeln!(output, "`-- {}", diagnostic.message).unwrap();
}

fn unknown(value: &str) -> &str {
    if value.is_empty() { "<unknown>" } else { value }
}

#[cfg(test)]
mod build_diagnostic_tests {
    use super::{BuildDiagnostic, BuildDiagnostics};

    #[test]
    fn render_keeps_source_and_contract_fields_together() {
        let mut diagnostics = BuildDiagnostics::default();
        let diagnostic = BuildDiagnostic::new("contract", "output contract does not match")
            .branch("control")
            .node("abc", "Button")
            .at("control/button.rs", 12)
            .function("Button::render")
            .field("output")
            .expected("Canvas")
            .actual("String")
            .provider("CanvasProvider");
        diagnostics.push(diagnostic.clone());
        diagnostics.push(diagnostic);
        let rendered = diagnostics.render();
        assert!(rendered.contains("phase=contract / 注册合同"));
        assert!(rendered.contains("source=control/button.rs:12"));
        assert!(rendered.contains("expected=Canvas"));
        assert_eq!(
            rendered.matches("output contract does not match").count(),
            1
        );
    }

    /// The read surface and `render()` must describe one set: a caller that
    /// counts `iter()` while a human reads `render()` cannot be told two
    /// different things about a failed build.
    /// 读取面与 `render()` 必须描述同一集合：CI 数 `iter()`、人类读 `render()`，
    /// 两者对构建失败不能给出两种说法。
    #[test]
    fn iteration_len_and_json_agree_with_the_rendered_set() {
        let mut diagnostics = BuildDiagnostics::default();
        assert!(diagnostics.is_empty());
        assert_eq!(diagnostics.len(), 0);
        assert_eq!(diagnostics.iter().count(), 0);
        assert_eq!(
            diagnostics.to_json(),
            "{\"schema\":\"nichlink.build-diagnostics/1\",\"count\":0,\"diagnostics\":[]}"
        );

        diagnostics.push(BuildDiagnostic::new("contract", "second message"));
        diagnostics.push(BuildDiagnostic::new("requirements", "first message"));
        diagnostics.push(BuildDiagnostic::new("requirements", "first message"));

        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics.len(), 2);
        assert_eq!(diagnostics.iter().count(), 2);
        let rendered = diagnostics.render();
        assert_eq!(rendered.matches("first message").count(), 1);
        assert_eq!(rendered.matches("second message").count(), 1);

        let json = diagnostics.to_json();
        assert!(json.starts_with("{\"schema\":\"nichlink.build-diagnostics/1\",\"count\":2,"));
        assert!(json.ends_with("]}"));
        assert_eq!(json.matches("first message").count(), 1);
        assert_eq!(json.matches("second message").count(), 1);
        // The rendered order is the JSON order: phase sorts first and
        // `contract` sorts before `requirements`, so a reader of either sees the
        // same first failure.
        // 渲染顺序即 JSON 顺序：先按 phase 排序，`contract` 排在 `requirements`
        // 前，因此两种读取方看到的第一条失败相同。
        let first = json.find("second message").expect("contract diagnostic");
        let second = json.find("first message").expect("requirements diagnostic");
        assert!(first < second, "{json}");
    }

    /// A message can carry source text with a quote, a backslash, a newline, or
    /// a tab. Emitting those raw would break the whole document, so the escape
    /// has to leave the structural characters intact.
    /// 消息可能带有含引号、反斜杠、换行或制表的源码片段。原样写出会破坏整个文档，
    /// 因此转义必须让结构字符保持完整。
    #[test]
    fn json_escapes_quotes_backslashes_and_control_characters() {
        let mut diagnostics = BuildDiagnostics::default();
        diagnostics.push(
            BuildDiagnostic::new("contract", "expected \"Canvas\" \\ actual\nnext\ttab")
                .at("control/button.rs", 3),
        );
        let json = diagnostics.to_json();
        assert!(json.contains(r#"\"Canvas\""#), "{json}");
        assert!(json.contains(r"\\ actual"), "{json}");
        assert!(json.contains(r"\nnext\ttab"), "{json}");
        assert!(
            !json.contains('\n') && !json.contains('\t'),
            "a control character must never reach the document raw: {json}"
        );
        assert!(
            json.contains("\"source\":\"control/button.rs\",\"line\":3"),
            "{json}"
        );
    }
}
