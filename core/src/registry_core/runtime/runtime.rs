// Runtime values, provenance, checks, and logical call tracing.
// 运行时值、来源链、检查与逻辑调用追踪。

use crate::registry_core::identity::NodeId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProvenanceStep {
    pub node: NodeId,
    pub object: &'static str,
    pub operation: &'static str,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Provenance {
    pub steps: Vec<ProvenanceStep>,
}

impl Provenance {
    pub fn push(
        mut self,
        node: NodeId,
        object: &'static str,
        operation: &'static str,
        value: impl Into<String>,
    ) -> Self {
        self.steps.push(ProvenanceStep {
            node,
            object,
            operation,
            value: value.into(),
        });
        self
    }
}

#[derive(Clone, Debug)]
pub enum RuntimeValue {
    Coordinates(Coordinates),
    Number {
        value: f64,
        provenance: Provenance,
    },
    Text {
        value: String,
        provenance: Provenance,
    },
}

impl RuntimeValue {
    pub fn number(value: f64, provenance: Provenance) -> Self {
        Self::Number { value, provenance }
    }

    pub fn text(value: impl Into<String>, provenance: Provenance) -> Self {
        Self::Text {
            value: value.into(),
            provenance,
        }
    }

    pub fn provenance(&self) -> &Provenance {
        match self {
            Self::Coordinates(coordinates) => &coordinates.provenance,
            Self::Number { provenance, .. } | Self::Text { provenance, .. } => provenance,
        }
    }

    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Coordinates(_) => "coordinates",
            Self::Number { .. } => "number",
            Self::Text { .. } => "text",
        }
    }
}

/// UI coordinates with actual and expected coordinate spaces.
/// 同时携带实际坐标系和预期坐标系的 UI 坐标。
#[derive(Clone, Debug)]
pub struct Coordinates {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub coordinate_space: &'static str,
    pub expected_space: &'static str,
    pub provenance: Provenance,
}

impl Coordinates {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        viewport_width: f32,
        viewport_height: f32,
        coordinate_space: &'static str,
        expected_space: &'static str,
        provenance: Provenance,
    ) -> Self {
        Self {
            x,
            y,
            width,
            height,
            viewport_width,
            viewport_height,
            coordinate_space,
            expected_space,
            provenance,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeCheckFailure {
    pub check: &'static str,
    pub message: String,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeCheckSpec {
    CoordinatesInViewport,
    FiniteNumber,
    NumberInRange { min: i64, max: i64 },
    NonEmptyText,
    TextLength { min: usize, max: usize },
}

impl RuntimeCheckSpec {
    pub const fn name(self) -> &'static str {
        match self {
            Self::CoordinatesInViewport => "coordinates_in_viewport",
            Self::FiniteNumber => "finite_number",
            Self::NumberInRange { .. } => "number_in_range",
            Self::NonEmptyText => "non_empty_text",
            Self::TextLength { .. } => "text_length",
        }
    }

    pub fn expression(self) -> String {
        match self {
            Self::CoordinatesInViewport => "crate::COORDINATES_IN_VIEWPORT".to_owned(),
            Self::FiniteNumber => "crate::FINITE_NUMBER".to_owned(),
            Self::NumberInRange { min, max } => {
                format!("crate::RuntimeCheckSpec::number_in_range({min}, {max})")
            }
            Self::NonEmptyText => "crate::NON_EMPTY_TEXT".to_owned(),
            Self::TextLength { min, max } => {
                format!("crate::RuntimeCheckSpec::text_length({min}, {max})")
            }
        }
    }

    pub const fn number_in_range(min: i64, max: i64) -> Self {
        Self::NumberInRange { min, max }
    }

    pub const fn text_length(min: usize, max: usize) -> Self {
        Self::TextLength { min, max }
    }

    /// Parse the compact names written by the authoring form.
    /// 解析创作表单写入的紧凑检查名称。
    pub fn parse_list(value: &str) -> Result<Vec<Self>, String> {
        let value = value.trim().trim_start_matches('[').trim_end_matches(']');
        if value.is_empty() {
            return Ok(Vec::new());
        }
        checks::split_items(value)
            .into_iter()
            .map(|item| Self::parse_one(item.trim()))
            .collect()
    }

    fn parse_one(value: &str) -> Result<Self, String> {
        let value = value.trim();
        let value = value.strip_prefix("crate::").unwrap_or(value);
        let value = value
            .strip_prefix("RuntimeCheckSpec::")
            .unwrap_or(value)
            .trim();
        match value {
            "COORDINATES_IN_VIEWPORT" | "coordinates_in_viewport" => {
                return Ok(Self::CoordinatesInViewport)
            }
            "FINITE_NUMBER" | "finite_number" => return Ok(Self::FiniteNumber),
            "NON_EMPTY_TEXT" | "non_empty_text" => return Ok(Self::NonEmptyText),
            _ => {}
        }
        if let Some(arguments) = value
            .strip_prefix("number_in_range(")
            .and_then(|rest| rest.strip_suffix(')'))
        {
            let mut values = arguments.split(',').map(str::trim);
            let min = values
                .next()
                .ok_or_else(|| "number_in_range requires min and max".to_owned())?
                .parse::<i64>()
                .map_err(|_| "number_in_range min must be an integer".to_owned())?;
            let max = values
                .next()
                .ok_or_else(|| "number_in_range requires min and max".to_owned())?
                .parse::<i64>()
                .map_err(|_| "number_in_range max must be an integer".to_owned())?;
            if values.next().is_some() {
                return Err("number_in_range accepts exactly two integers".to_owned());
            }
            return Ok(Self::NumberInRange { min, max });
        }
        if let Some(arguments) = value
            .strip_prefix("text_length(")
            .and_then(|rest| rest.strip_suffix(')'))
        {
            let mut values = arguments.split(',').map(str::trim);
            let min = values
                .next()
                .ok_or_else(|| "text_length requires min and max".to_owned())?
                .parse::<usize>()
                .map_err(|_| "text_length min must be an unsigned integer".to_owned())?;
            let max = values
                .next()
                .ok_or_else(|| "text_length requires min and max".to_owned())?
                .parse::<usize>()
                .map_err(|_| "text_length max must be an unsigned integer".to_owned())?;
            if values.next().is_some() {
                return Err("text_length accepts exactly two integers".to_owned());
            }
            return Ok(Self::TextLength { min, max });
        }
        Err(format!("unknown runtime check `{value}`"))
    }

    pub fn run(self, value: &RuntimeValue) -> Result<(), RuntimeCheckFailure> {
        match self {
            Self::CoordinatesInViewport => checks::coordinates(value),
            Self::FiniteNumber => checks::finite_number(value),
            Self::NumberInRange { min, max } => checks::number_range(value, min, max),
            Self::NonEmptyText => checks::non_empty_text(value),
            Self::TextLength { min, max } => checks::text_length(value, min, max),
        }
    }
}

pub const COORDINATES_IN_VIEWPORT: RuntimeCheckSpec = RuntimeCheckSpec::CoordinatesInViewport;
pub const FINITE_NUMBER: RuntimeCheckSpec = RuntimeCheckSpec::FiniteNumber;
pub const NON_EMPTY_TEXT: RuntimeCheckSpec = RuntimeCheckSpec::NonEmptyText;

pub use self::trace::{
    CallSite, CallTrace, DataEdge, DataHop, FramePath, LocalId, LocalKind, LocalValue, TraceMode,
    Observation,
};

/// One call edge that was observed while a `CallTrace` frame was active.
/// `CallTrace` 中实际观察到的一条调用边。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallEdge {
    pub caller: CallSite,
    pub callee: CallSite,
}

/// Provenance of a relationship across runtime, MIR, and source evidence.
/// 运行时、MIR 与源码证据共用的关系来源。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceKind {
    /// Confirmed by a live `CallTrace` observation.
    Live,
    /// Candidate found by source analysis without runtime confirmation.
    Source,
    /// Candidate inferred from MIR; it may not have executed.
    Mir,
    /// Supplied by an external adapter without a local trace.
    External,
    /// A relation whose producer is not known.
    Unknown,
}

impl EvidenceKind {
    /// Compact marker used by text, DOT, and Studio renderers.
    /// 文本、DOT 与 Studio 渲染器共用的紧凑标记。
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Live => "+",
            Self::Mir => "?",
            Self::Source => "~",
            Self::External => "x",
            Self::Unknown => "!",
        }
    }

    /// Stable machine and human readable label.
    /// 稳定的机器和人类可读标签。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Mir => "mir",
            Self::Source => "source",
            Self::External => "external",
            Self::Unknown => "unknown",
        }
    }

    /// Only a live observation confirms that an edge executed.
    /// 只有 Live 观察能确认边实际执行过。
    pub const fn confirmed(self) -> bool {
        matches!(self, Self::Live)
    }
}

/// A call edge with invocation IDs removed for topology queries.
/// 去掉调用实例编号、用于拓扑查询的逻辑调用边。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicalCallEdge {
    pub caller: CallSite,
    pub callee: CallSite,
    pub evidence: EvidenceKind,
}
