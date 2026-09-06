//! Structured build diagnostics shared by every registration check.
//! 所有注册检查共用的结构化构建诊断。

use std::fmt::Write as _;

/// One declaration error before it is rendered for rustc or a terminal.
/// 在输出给 rustc 或终端之前的一条声明错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BuildDiagnostic {
    pub(crate) phase: &'static str,
    pub(crate) branch: String,
    pub(crate) node: String,
    pub(crate) source: String,
    pub(crate) line: usize,
    pub(crate) function: String,
    pub(crate) field: String,
    pub(crate) expected: String,
    pub(crate) actual: String,
    pub(crate) provider: String,
    pub(crate) message: String,
}

impl BuildDiagnostic {
    pub(crate) fn new(phase: &'static str, message: impl Into<String>) -> Self {
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

    pub(crate) fn at(mut self, source: impl Into<String>, line: usize) -> Self {
        self.source = source.into();
        self.line = line;
        self
    }

    pub(crate) fn node(mut self, node: impl Into<String>, kind: impl Into<String>) -> Self {
        self.node = format!("{} ({})", node.into(), kind.into());
        self
    }

    pub(crate) fn branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = branch.into();
        self
    }

    pub(crate) fn function(mut self, function: impl Into<String>) -> Self {
        self.function = function.into();
        self
    }

    pub(crate) fn field(mut self, field: impl Into<String>) -> Self {
        self.field = field.into();
        self
    }

    pub(crate) fn expected(mut self, expected: impl Into<String>) -> Self {
        self.expected = expected.into();
        self
    }

    pub(crate) fn actual(mut self, actual: impl Into<String>) -> Self {
        self.actual = actual.into();
        self
    }

    pub(crate) fn provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = provider.into();
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct BuildDiagnostics {
    items: Vec<BuildDiagnostic>,
}

impl BuildDiagnostics {
    pub(crate) fn push(&mut self, diagnostic: BuildDiagnostic) {
        self.items.push(diagnostic);
    }

    pub(crate) fn extend(&mut self, other: Self) {
        self.items.extend(other.items);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn render(&self) -> String {
        if self.items.is_empty() {
            return String::new();
        }
        let mut items = self.items.clone();
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
        let mut output = String::from("NICHLink BUILD CHECK FAILED / NichLink 构建检查失败\n");
        for (index, diagnostic) in items.iter().enumerate() {
            if index > 0 {
                output.push('\n');
            }
            render_item(&mut output, diagnostic);
        }
        output
    }
}

fn render_item(output: &mut String, diagnostic: &BuildDiagnostic) {
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
        empty(&diagnostic.branch)
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

fn empty(value: &str) -> &str {
    if value.is_empty() {
        "<unknown>"
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
