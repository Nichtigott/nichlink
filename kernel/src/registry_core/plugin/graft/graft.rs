//! Atomic graft commands and validation errors.
//! 原子嫁接命令与校验错误。

use crate::registry_core::declaration::{FrameworkId, OwnedFlowContract};
use std::fmt;

#[path = "document.rs"]
pub mod document;

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
    /// A single-node cut: replace `cut` by `graft`, with no range end.
    /// 单节点切口：用 `graft` 替换 `cut`，不带区间终点。
    pub fn new(cut: impl Into<String>, graft: impl Into<String>) -> Self {
        Self {
            cut: cut.into(),
            graft: graft.into(),
            end: None,
            subtree: false,
        }
    }

    /// A contiguous cut from `start` to `end`, with both endpoints usable.
    /// 从 `start` 到 `end` 的连续切口，两个端点都可用。
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

    /// A cut that replaces the whole subtree rooted at `cut`.
    /// 替换 `cut` 根节点下整棵子树的切口。
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
    /// Framework the plan's cuts belong to.
    /// 本计划各切口所属的框架。
    pub framework: FrameworkId,
    /// The cuts to apply, in the order they were added.
    /// 待应用的切口，按加入顺序排列。
    pub cuts: Vec<GraftCut>,
}

impl GraftPlan {
    /// An empty plan for one framework.
    /// 为某个框架新建的空计划。
    pub fn new(framework: FrameworkId) -> Self {
        Self {
            framework,
            cuts: Vec::new(),
        }
    }

    /// Builder form: append a single-node cut and return the plan.
    /// 构建器写法：追加一条单节点切口并返回计划。
    pub fn cut(mut self, path: impl Into<String>, graft: impl Into<String>) -> Self {
        self.cuts.push(GraftCut::new(path, graft));
        self
    }

    /// Append a single-node cut to this plan in place.
    /// 就地向本计划追加一条单节点切口。
    pub fn push(&mut self, path: impl Into<String>, graft: impl Into<String>) {
        self.cuts.push(GraftCut::new(path, graft));
    }

    /// Parse one `cut … graft …` command into a single-cut plan.
    /// 把一条 `cut … graft …` 命令解析成含单条切口的计划。
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
    /// Start of the cut path or of a range.
    /// 切口路径或区间的起点。
    pub cut: String,
    /// Far endpoint when the command spelled a range.
    /// 命令写出区间时的远端终点。
    pub end: Option<String>,
    /// External implementation selector.
    /// 外部实现选择器。
    pub graft: String,
    /// Whether the cut replaces the whole subtree.
    /// 该切口是否替换整棵子树。
    pub full: bool,
}

/// Split a `cut [ … ]` argument at the command grammar's standalone `to` word.
/// 在命令语法的独立 `to` 词处拆分 `cut [ … ]` 参数。
///
/// Why not `split_once(" to ")`: this grammar's separator is the *word* `to`,
/// the same word the macro form's token grammar spells (`cut ["a" to "b"]`).
/// Comparing a substring ties the rule to the exact number of surrounding
/// spaces and keeps a second definition of "range" alive now that the
/// declaration parser stores the far endpoint in `GraftSyntax::cut_end` instead
/// of joining it into the path text. Boundary: this is the only remaining place
/// a `to` is interpreted, because a command string is text a user typed. The
/// declaration grammar has a form this text protocol has none of — a single
/// bracketed string literal, `cut ["root/a to b"]`, which is one path — and
/// that is exactly why the parser, not this function, owns range-ness. A `to`
/// that is the whole argument, or has no text on one side, is an ordinary path
/// segment exactly as the substring form treated it.
/// 为什么不用 `split_once(" to ")`：本语法的分隔符是**词** `to`，与宏形式的 token
/// 语法（`cut ["a" to "b"]`）是同一个词。比较子串会让规则取决于两侧空格的确切数量，
/// 并在声明解析器已把远端存进 `GraftSyntax::cut_end`、不再拼进路径文本之后，让第二份
/// “区间”定义继续存在。边界：这是最后一处解释 `to` 的地方，因为命令字符串是用户敲下
/// 的文本。声明语法有一种本文本协议没有的形式——单个带方括号的字符串字面量
/// `cut ["root/a to b"]`，它是一条路径——这正是区间性由解析器而非本函数拥有的原因。
/// 整个参数就是 `to`、或某一侧没有文本的 `to`，与子串形式一样只是普通路径段。
/// Pinned by `cut_command_reads_a_range_as_two_endpoints` and
/// `cut_command_keeps_a_word_boundary_to_as_path_text` in
/// `tree/graft_ops/graft_ops.rs`.
/// 由 `tree/graft_ops/graft_ops.rs` 的 `cut_command_reads_a_range_as_two_endpoints`
/// 与 `cut_command_keeps_a_word_boundary_to_as_path_text` 钉住。
fn split_command_range(raw_cut: &str) -> (&str, Option<&str>) {
    let mut cursor = 0;
    for word in raw_cut.split_whitespace() {
        let Some(offset) = raw_cut[cursor..].find(word) else {
            break;
        };
        let found = cursor + offset;
        // `split_whitespace` already guarantees a word boundary; requiring text
        // on both sides keeps the argument `to` itself an ordinary path, which
        // is what the substring form did.
        // `split_whitespace` 已保证词边界；要求两侧都有文本，才能让参数本身就是
        // `to` 的情形仍按普通路径处理——子串形式正是如此。
        if word == "to" && found > 0 && found + word.len() < raw_cut.len() {
            return (
                raw_cut[..found].trim(),
                Some(raw_cut[found + word.len()..].trim()),
            );
        }
        cursor = found + word.len();
    }
    (raw_cut.trim(), None)
}

impl CutGraftCommand {
    /// Parse the text grammar, including the optional `full` word and `to` range word.
    /// 解析文本语法，包括可选的 `full` 词与区间词 `to`。
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
        let (cut, end) = split_command_range(raw_cut);
        let end = end.map(str::to_owned);
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
    /// Command text did not parse; the payload is the parser's reason.
    /// 命令文本未能解析；载荷是解析器给出的原因。
    InvalidCommand(String),
    /// The cut path names no registered face.
    /// 切口路径没有指向任何已注册的面。
    UnknownTarget(crate::NodeId),
    /// The replacement identity is not registered.
    /// 替换件身份没有注册。
    UnknownReplacement(crate::NodeId),
    /// A string selector matched more than one registered face.
    /// 字符串选择器匹配到不止一个已注册的面。
    AmbiguousReplacement {
        /// Selector that matched more than one face.
        /// 匹配到不止一个面的选择器。
        selector: String,
        /// How many registered faces it matched.
        /// 它匹配到的已注册面数量。
        matches: usize,
    },
    /// A graft requires both sides to declare a flow contract.
    /// 嫁接要求两侧都声明数据流合同。
    ContractUndeclared,
    /// The two sides declare data-flow contracts that are not compatible.
    /// 两侧声明的数据流合同互不兼容。
    ContractMismatch {
        /// Contract declared by the cut target.
        /// 切口目标声明的合同。
        expected: OwnedFlowContract,
        /// Contract declared by the replacement.
        /// 替换件声明的合同。
        received: OwnedFlowContract,
    },
    /// The plugin does not target the plan's framework.
    /// 插件针对的框架不是本计划的框架。
    FrameworkMismatch(FrameworkId),
    /// The same cut path appears more than once in one plan.
    /// 同一个切口路径在同一计划中出现多次。
    DuplicateCut(String),
    /// The two endpoints of a range are unusable together: they do not share a
    /// parent, or the range is written backwards.
    /// 区间的两个端点无法配合使用：不同属一个父级，或区间被写反了。
    ///
    /// The payload is a complete reason sentence and `Display` prints it
    /// verbatim. Two different range problems need two different remedies
    /// ("share one parent" versus "write it the other way round"), so the
    /// sentence has to come from the site that knows which one applies; a fixed
    /// template around a bare selector left no room for the second.
    /// 载荷是一句完整的原因，`Display` 原样打印。两种区间问题需要两种不同的补救
    /// （“同属一个父级”与“反过来写”），因此这句话必须由知道该用哪一种的调用点给出；
    /// 用一个固定模板套一个裸选择器，第二种情况就无处表达。
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
            Self::AmbiguousReplacement { selector, matches } => write!(
                formatter,
                "graft replacement selector `{selector}` matches {matches} registered faces; use the face's NODE_ID"
            ),
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
            Self::InvalidRange(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for GraftError {}
