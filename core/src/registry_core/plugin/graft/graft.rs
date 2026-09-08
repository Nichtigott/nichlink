//! Atomic graft commands and validation errors.
//! 原子嫁接命令与校验错误。

use super::*;
use std::fmt;

/// One path cut and its external implementation selector.
/// 一条路径切口及其外部实现选择器。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraftCut {
    /// A slash-separated logical path, for example `root/a1/b2`.
    /// 逻辑路径，例如 `root/a1/b2`。
    pub cut: String,
    /// Name, path, or NodeId of a face supplied by the external graft set.
    /// 外部 graft 集合中实现的名称、路径或 NodeId。
    pub graft: String,
    /// Optional end of a contiguous path range.
    /// 连续路径范围的可选终点。
    pub end: Option<String>,
    /// Whether the cut replaces the complete subtree rooted at `cut`.
    /// 是否替换 cut 根节点下的整棵子树。
    pub subtree: bool,
}

impl GraftCut {
    pub fn new(cut: impl Into<String>, graft: impl Into<String>) -> Self {
        Self {
            cut: cut.into(),
            graft: graft.into(),
            end: None,
            subtree: false,
        }
    }

    pub fn range(
        start: impl Into<String>,
        end: impl Into<String>,
        graft: impl Into<String>,
    ) -> Self {
        Self {
            cut: start.into(),
            graft: graft.into(),
            end: Some(end.into()),
            subtree: false,
        }
    }

    pub fn subtree(cut: impl Into<String>, graft: impl Into<String>) -> Self {
        Self {
            cut: cut.into(),
            graft: graft.into(),
            end: None,
            subtree: true,
        }
    }
}

/// A persistent overlay plan. It never moves or edits source files.
/// 持久化覆盖计划；它永不移动或编辑源码文件。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraftPlan {
    pub framework: FrameworkId,
    pub cuts: Vec<GraftCut>,
}

impl GraftPlan {
    pub fn new(framework: FrameworkId) -> Self {
        Self {
            framework,
            cuts: Vec::new(),
        }
    }

    pub fn cut(mut self, path: impl Into<String>, graft: impl Into<String>) -> Self {
        self.cuts.push(GraftCut::new(path, graft));
        self
    }

    pub fn push(&mut self, path: impl Into<String>, graft: impl Into<String>) {
        self.cuts.push(GraftCut::new(path, graft));
    }

    pub fn command(framework: FrameworkId, command: &str) -> Result<Self, String> {
        let parsed = CutGraftCommand::parse(command)?;
        let mut plan = Self::new(framework);
        if let Some(end) = parsed.end {
            let mut cut = GraftCut::range(parsed.cut, end, parsed.graft);
            cut.subtree = parsed.full;
            plan.cuts.push(cut);
        } else if parsed.full {
            plan.cuts.push(GraftCut::subtree(parsed.cut, parsed.graft));
        } else {
            plan.cuts.push(GraftCut::new(parsed.cut, parsed.graft));
        }
        Ok(plan)
    }
}

/// Parsed `cut [A/a1/b2] graft replacement` command.
/// 解析 `cut [A/a1/b2] graft replacement` 命令。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CutGraftCommand {
    pub cut: String,
    pub end: Option<String>,
    pub graft: String,
    pub full: bool,
}

impl CutGraftCommand {
    pub fn parse(command: &str) -> Result<Self, String> {
        let command = command.trim();
        let body = command
            .strip_prefix("cut ")
            .ok_or_else(|| "cut command must start with `cut`".to_owned())?;
        let (raw_cut, rest) = if let Some(inner) = body.strip_prefix('[') {
            let end = inner
                .find(']')
                .ok_or_else(|| "cut command is missing `]`".to_owned())?;
            (&inner[..end], inner[end + 1..].trim())
        } else {
            let mut parts = body.splitn(2, char::is_whitespace);
            let path = parts.next().unwrap_or_default();
            (path, parts.next().unwrap_or_default().trim())
        };
        if raw_cut.is_empty() {
            return Err("cut path must contain non-empty `/`-separated segments".to_owned());
        }
        let mut parts = rest.split_whitespace();
        let full = matches!(parts.clone().next(), Some("full"));
        if full {
            parts.next();
        }
        if parts.next() != Some("graft") {
            return Err("cut command must use `cut [path] graft <implementation>`".to_owned());
        }
        let graft = parts
            .next()
            .ok_or_else(|| "cut command is missing the graft implementation".to_owned())?;
        if parts.next().is_some() {
            return Err("cut command has unexpected trailing arguments".to_owned());
        }
        let (cut, end) = raw_cut
            .split_once(" to ")
            .map_or((raw_cut.trim(), None), |(start, finish)| {
                (start.trim(), Some(finish.trim().to_owned()))
            });
        if cut.is_empty() || end.as_deref().is_some_and(str::is_empty) {
            return Err("cut path must contain non-empty `/`-separated segments".to_owned());
        }
        Ok(Self {
            cut: cut.to_owned(),
            end,
            graft: graft.to_owned(),
            full,
        })
    }
}

/// Reasons a graft was rejected before the live tree was touched.
/// 嫁接在修改线上树之前被拒绝的原因。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraftError {
    InvalidCommand(String),
    UnknownTarget(crate::NodeId),
    UnknownReplacement(crate::NodeId),
    ContractUndeclared,
    ContractMismatch {
        expected: OwnedFlowContract,
        received: OwnedFlowContract,
    },
    FrameworkMismatch(FrameworkId),
    DuplicateCut(String),
    InvalidRange(String),
}

impl fmt::Display for GraftError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCommand(message) => formatter.write_str(message),
            Self::UnknownTarget(id) => write!(formatter, "graft target `{id}` is not registered"),
            Self::UnknownReplacement(id) => {
                write!(formatter, "graft replacement `{id}` is not registered")
            }
            Self::ContractUndeclared => {
                formatter.write_str("graft requires both nodes to declare a flow contract")
            }
            Self::ContractMismatch { expected, received } => write!(
                formatter,
                "graft contract mismatch: expected {} v{} ({} -> {}), received {} v{} ({} -> {})",
                expected.id,
                expected.version,
                expected.input,
                expected.output,
                received.id,
                received.version,
                received.input,
                received.output
            ),
            Self::FrameworkMismatch(framework) => {
                write!(formatter, "plugin does not target framework `{framework}`")
            }
            Self::DuplicateCut(path) => {
                write!(formatter, "graft plan cuts `{path}` more than once")
            }
            Self::InvalidRange(path) => {
                write!(formatter, "graft range `{path}` must share one parent")
            }
        }
    }
}

impl std::error::Error for GraftError {}
