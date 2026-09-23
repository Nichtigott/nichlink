//! Runtime check values and specifications.
//! 运行期校验的取值与规格。

use super::*;

/// One observation in a provenance chain: which node, object, and operation
/// produced a value.
/// 来源链中的一条观测：记录哪个节点、对象与操作产出了某个取值。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProvenanceStep {
    /// Node that performed the operation.
    /// 执行该操作的节点。
    pub node: NodeId,
    /// Object the operation was performed on.
    /// 该操作所作用的对象。
    pub object: &'static str,
    /// Name of the operation that produced the value.
    /// 产出该取值的操作名称。
    pub operation: &'static str,
    /// The observed value rendered as text.
    /// 以文本形式记录的观测取值。
    pub value: String,
}

/// Ordered evidence explaining how a runtime value was produced.
/// 说明某个运行期取值如何产生的有序证据。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Provenance {
    /// Observations in the order they occurred.
    /// 按发生顺序排列的观测步骤。
    pub steps: Vec<ProvenanceStep>,
}

impl Provenance {
    /// Append one observation and return the extended chain.
    /// 追加一条观测并返回扩展后的证据链。
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

/// A value observed at runtime, tagged with the evidence that produced it.
/// 运行期观测到的取值，并携带产出它的证据。
#[derive(Clone, Debug)]
pub enum RuntimeValue {
    /// Geometry carrying both its actual and expected coordinate spaces.
    /// 同时携带实际坐标系与预期坐标系的几何取值。
    Coordinates(Coordinates),
    /// A numeric observation plus its provenance.
    /// 数值观测及其来源。
    Number {
        /// The observed number.
        /// 观测到的数值。
        value: f64,
        /// Evidence explaining where the number came from.
        /// 说明该数值由来的证据。
        provenance: Provenance,
    },
    /// A text observation plus its provenance.
    /// 文本观测及其来源。
    Text {
        /// The observed text.
        /// 观测到的文本。
        value: String,
        /// Evidence explaining where the text came from.
        /// 说明该文本由来的证据。
        provenance: Provenance,
    },
}

impl RuntimeValue {
    /// Build a numeric value carrying its provenance.
    /// 构建携带来源证据的数值。
    pub fn number(value: f64, provenance: Provenance) -> Self {
        Self::Number { value, provenance }
    }

    /// Build a text value carrying its provenance.
    /// 构建携带来源证据的文本值。
    pub fn text(value: impl Into<String>, provenance: Provenance) -> Self {
        Self::Text {
            value: value.into(),
            provenance,
        }
    }

    /// Borrow the evidence chain shared by every variant.
    /// 借用各变体共有的证据链。
    pub fn provenance(&self) -> &Provenance {
        match self {
            Self::Coordinates(coordinates) => &coordinates.provenance,
            Self::Number { provenance, .. } | Self::Text { provenance, .. } => provenance,
        }
    }

    /// Stable kind name used in check-failure messages.
    /// 用于校验失败消息的稳定种类名。
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
    /// Logical x of the rectangle's left edge.
    /// 矩形左边缘的逻辑 x 坐标。
    pub x: f32,
    /// Logical y of the rectangle's top edge.
    /// 矩形上边缘的逻辑 y 坐标。
    pub y: f32,
    /// Rectangle width in the actual coordinate space.
    /// 实际坐标系下的矩形宽度。
    pub width: f32,
    /// Rectangle height in the actual coordinate space.
    /// 实际坐标系下的矩形高度。
    pub height: f32,
    /// Width of the viewport the rectangle must fit inside.
    /// 矩形必须落在其中的视口宽度。
    pub viewport_width: f32,
    /// Height of the viewport the rectangle must fit inside.
    /// 矩形必须落在其中的视口高度。
    pub viewport_height: f32,
    /// Coordinate space the rectangle was measured in.
    /// 测量该矩形时所用的坐标系。
    pub coordinate_space: &'static str,
    /// Coordinate space the check requires; must match `coordinate_space`.
    /// 校验要求的坐标系，必须与 `coordinate_space` 相同。
    pub expected_space: &'static str,
    /// Evidence explaining where these coordinates came from.
    /// 说明该坐标由来的证据。
    pub provenance: Provenance,
}

impl Coordinates {
    /// Assemble a coordinate observation for the viewport check to validate.
    /// 组装坐标观测，交由视口检查校验。
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

/// Evidence returned when a runtime check rejects a value.
/// 运行期检查拒绝某个取值时返回的证据。
#[derive(Clone, Debug)]
pub struct RuntimeCheckFailure {
    /// Stable check name that produced the failure.
    /// 产出该失败的检查稳定名称。
    pub check: &'static str,
    /// Human-readable reason the value was rejected.
    /// 该取值被拒绝的人类可读原因。
    pub message: String,
    /// Evidence explaining where the rejected value came from.
    /// 说明被拒取值由来的证据。
    pub provenance: Provenance,
}

/// A named, parameterized check the host runs on produced values.
/// 宿主对产出取值执行的具名参数化校验。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeCheckSpec {
    /// The rectangle fits inside its viewport and both spaces match.
    /// 矩形落在视口内，且两个坐标系一致。
    CoordinatesInViewport,
    /// The value is a finite number.
    /// 取值是有限数。
    FiniteNumber,
    /// The number lies within an inclusive range; inverted bounds fail.
    /// 数值落在闭区间内；边界倒置视为失败。
    NumberInRange {
        /// Inclusive lower bound.
        /// 闭区间下界。
        min: i64,
        /// Inclusive upper bound.
        /// 闭区间上界。
        max: i64,
    },
    /// The text is not empty and not whitespace only.
    /// 文本非空且不全是空白。
    NonEmptyText,
    /// The text's character count lies within an inclusive range.
    /// 文本的字符数落在闭区间内。
    TextLength {
        /// Inclusive minimum character count.
        /// 闭区间的最小字符数。
        min: usize,
        /// Inclusive maximum character count.
        /// 闭区间的最大字符数。
        max: usize,
    },
}

impl RuntimeCheckSpec {
    /// Stable machine name written into failure evidence.
    /// 写入失败证据的稳定机器名。
    pub const fn name(self) -> &'static str {
        match self {
            Self::CoordinatesInViewport => "coordinates_in_viewport",
            Self::FiniteNumber => "finite_number",
            Self::NumberInRange { .. } => "number_in_range",
            Self::NonEmptyText => "non_empty_text",
            Self::TextLength { .. } => "text_length",
        }
    }

    /// Rust expression text that reconstructs this check in authored code.
    /// 在作者代码中重建该检查的 Rust 表达式文本。
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

    /// Build an inclusive numeric range check.
    /// 构建闭区间数值校验。
    pub const fn number_in_range(min: i64, max: i64) -> Self {
        Self::NumberInRange { min, max }
    }

    /// Build an inclusive character-length check.
    /// 构建闭区间字符数校验。
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
        split_items(value)
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
                return Ok(Self::CoordinatesInViewport);
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

    /// Evaluate the check against one value, reporting failure as evidence.
    /// 对单个取值执行检查，失败时以证据形式报告。
    pub fn run(self, value: &RuntimeValue) -> Result<(), RuntimeCheckFailure> {
        match self {
            Self::CoordinatesInViewport => check_coordinates(value),
            Self::FiniteNumber => check_finite_number(value),
            Self::NumberInRange { min, max } => check_number_range(value, min, max),
            Self::NonEmptyText => check_non_empty_text(value),
            Self::TextLength { min, max } => check_text_length(value, min, max),
        }
    }
}

/// The viewport-fitting check, for authoring code to name directly.
/// 视口适配检查，供创作代码直接引用的常量。
pub const COORDINATES_IN_VIEWPORT: RuntimeCheckSpec = RuntimeCheckSpec::CoordinatesInViewport;

/// The finite-number check, for authoring code to name directly.
/// 有限数检查，供创作代码直接引用的常量。
pub const FINITE_NUMBER: RuntimeCheckSpec = RuntimeCheckSpec::FiniteNumber;

/// The non-empty-text check, for authoring code to name directly.
/// 非空文本检查，供创作代码直接引用的常量。
pub const NON_EMPTY_TEXT: RuntimeCheckSpec = RuntimeCheckSpec::NonEmptyText;

fn split_items(value: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    for (index, character) in value.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                items.push(value[start..index].trim());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    items.push(value[start..].trim());
    items
}

fn wrong_kind(check: &'static str, expected: &str, value: &RuntimeValue) -> RuntimeCheckFailure {
    RuntimeCheckFailure {
        check,
        message: format!("expected {expected} data, received {}", value.kind()),
        provenance: value.provenance().clone(),
    }
}

fn check_coordinates(value: &RuntimeValue) -> Result<(), RuntimeCheckFailure> {
    let RuntimeValue::Coordinates(coordinates) = value else {
        return Err(wrong_kind("coordinates_in_viewport", "coordinate", value));
    };
    let finite = [
        coordinates.x,
        coordinates.y,
        coordinates.width,
        coordinates.height,
        coordinates.viewport_width,
        coordinates.viewport_height,
    ]
    .into_iter()
    .all(f32::is_finite);
    if !finite {
        return Err(RuntimeCheckFailure {
            check: "coordinates_in_viewport",
            message: "coordinate contains NaN or infinity".to_owned(),
            provenance: coordinates.provenance.clone(),
        });
    }
    if coordinates.coordinate_space != coordinates.expected_space {
        return Err(RuntimeCheckFailure {
            check: "coordinates_in_viewport",
            message: format!(
                "coordinate space `{}` does not match expected `{}`",
                coordinates.coordinate_space, coordinates.expected_space
            ),
            provenance: coordinates.provenance.clone(),
        });
    }
    let inside = coordinates.x >= 0.0
        && coordinates.y >= 0.0
        && coordinates.width >= 0.0
        && coordinates.height >= 0.0
        && coordinates.x + coordinates.width <= coordinates.viewport_width
        && coordinates.y + coordinates.height <= coordinates.viewport_height;
    if inside {
        Ok(())
    } else {
        Err(RuntimeCheckFailure {
            check: "coordinates_in_viewport",
            message: format!(
                "rect ({:.1}, {:.1}, {:.1}, {:.1}) exceeds viewport ({:.1}, {:.1})",
                coordinates.x,
                coordinates.y,
                coordinates.width,
                coordinates.height,
                coordinates.viewport_width,
                coordinates.viewport_height
            ),
            provenance: coordinates.provenance.clone(),
        })
    }
}

fn check_finite_number(value: &RuntimeValue) -> Result<(), RuntimeCheckFailure> {
    let RuntimeValue::Number { value, provenance } = value else {
        return Err(wrong_kind("finite_number", "number", value));
    };
    if value.is_finite() {
        Ok(())
    } else {
        Err(RuntimeCheckFailure {
            check: "finite_number",
            message: format!("number `{value}` is NaN or infinite"),
            provenance: provenance.clone(),
        })
    }
}

fn check_number_range(value: &RuntimeValue, min: i64, max: i64) -> Result<(), RuntimeCheckFailure> {
    let RuntimeValue::Number { value, provenance } = value else {
        return Err(wrong_kind("number_in_range", "number", value));
    };
    if min > max {
        return Err(RuntimeCheckFailure {
            check: "number_in_range",
            message: format!("invalid check bounds: minimum {min} exceeds maximum {max}"),
            provenance: provenance.clone(),
        });
    }
    if value.is_finite() && *value >= min as f64 && *value <= max as f64 {
        Ok(())
    } else {
        Err(RuntimeCheckFailure {
            check: "number_in_range",
            message: format!("number `{value}` is outside inclusive range {min}..={max}"),
            provenance: provenance.clone(),
        })
    }
}

fn check_non_empty_text(value: &RuntimeValue) -> Result<(), RuntimeCheckFailure> {
    let RuntimeValue::Text { value, provenance } = value else {
        return Err(wrong_kind("non_empty_text", "text", value));
    };
    if value.trim().is_empty() {
        Err(RuntimeCheckFailure {
            check: "non_empty_text",
            message: "text is empty or whitespace only".to_owned(),
            provenance: provenance.clone(),
        })
    } else {
        Ok(())
    }
}

fn check_text_length(
    value: &RuntimeValue,
    min: usize,
    max: usize,
) -> Result<(), RuntimeCheckFailure> {
    let RuntimeValue::Text { value, provenance } = value else {
        return Err(wrong_kind("text_length", "text", value));
    };
    let length = value.chars().count();
    if min <= max && (min..=max).contains(&length) {
        Ok(())
    } else {
        Err(RuntimeCheckFailure {
            check: "text_length",
            message: if min > max {
                format!("invalid check bounds: minimum {min} exceeds maximum {max}")
            } else {
                format!("text length {length} is outside inclusive range {min}..={max}")
            },
            provenance: provenance.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    //! Boundary tests for the five runtime checks.
    //! 五条运行期检查的边界测试。
    //!
    //! Why a direct "call `run` and look at `is_err`" implementation would be
    //! wrong: each check owns several distinct failure arms (wrong kind,
    //! non-finite geometry, coordinate-space mismatch, inverted bounds, range
    //! overflow), and a boolean-only assertion cannot tell them apart — a check
    //! that rejected *every* value would pass it. These tests therefore assert
    //! the exact [`RuntimeCheckFailure`] evidence a host consumes: `check`,
    //! `message`, and `provenance`.
    //! Boundary: `RuntimeCheckFailure` deliberately does not implement
    //! `PartialEq`, so the assertions are field-level rather than whole-struct.
    //! Pinned by these five tests; `Registry::health_check` aggregates the same
    //! failures in `examples/control-button/tests/health_check.rs`.
    //! 直白写法错在哪：只调用 `run` 再看 `is_err` 会掩盖每条检查各自的多个失败分支
    //! （类型不符、坐标非有限、坐标系不符、边界倒置、越界），而“拒绝一切取值”的实现也
    //! 能通过布尔断言。因此这些测试断言宿主真正消费的 [`RuntimeCheckFailure`] 证据：
    //! `check`、`message` 与 `provenance`。
    //! 边界：`RuntimeCheckFailure` 刻意不实现 `PartialEq`，所以断言按字段进行而非整结构比较。
    //! 由这五条测试钉住；`Registry::health_check` 在
    //! `examples/control-button/tests/health_check.rs` 聚合同样的失败。

    use super::*;

    /// One provenance step whose `value` records the observed value verbatim.
    /// 一条来源步骤，其 `value` 原样记录被观测的取值。
    fn provenance(value: &str) -> Provenance {
        Provenance::default().push(NodeId::from_raw([9; 16]), "Button", "paint", value)
    }

    /// Run a check that must fail, so a passing check cannot hide behind the
    /// assertion.
    /// 运行一条必须失败的检查，使“检查通过”不会伪装成断言通过。
    fn check_failure(check: RuntimeCheckSpec, value: &RuntimeValue) -> RuntimeCheckFailure {
        check.run(value).expect_err("the check must fail")
    }

    /// Inside the viewport passes; overflow and a coordinate-space mismatch
    /// fail, each naming `coordinates_in_viewport` and carrying the observation.
    /// 位于视口内通过；越界与坐标系不符失败，且都命名为 `coordinates_in_viewport`
    /// 并携带观测来源。
    #[test]
    fn coordinates_in_viewport_accepts_inside_and_rejects_overflow_and_space_mismatch() {
        let check = RuntimeCheckSpec::CoordinatesInViewport;
        let inside = RuntimeValue::Coordinates(Coordinates::new(
            10.0,
            10.0,
            90.0,
            40.0,
            100.0,
            60.0,
            "logical",
            "logical",
            provenance("10,10,90,40"),
        ));
        assert!(
            check.run(&inside).is_ok(),
            "a rect that fits the viewport passes"
        );

        // `x + width = 110` exceeds the 100-wide viewport.
        // `x + width = 110` 超过了 100 宽的视口。
        let overflow = RuntimeValue::Coordinates(Coordinates::new(
            50.0,
            10.0,
            60.0,
            40.0,
            100.0,
            60.0,
            "logical",
            "logical",
            provenance("50,10,60,40"),
        ));
        let failure = check_failure(check, &overflow);
        assert_eq!(failure.check, "coordinates_in_viewport");
        assert_eq!(
            failure.message,
            "rect (50.0, 10.0, 60.0, 40.0) exceeds viewport (100.0, 60.0)"
        );
        assert_eq!(failure.provenance, provenance("50,10,60,40"));

        // A boundary that is not "more overflow" but a contract mismatch: the
        // observed space must equal the expected one.
        // 这不是“更多越界”，而是合同不符：观测坐标系必须等于预期坐标系。
        let mismatch = RuntimeValue::Coordinates(Coordinates::new(
            10.0,
            10.0,
            90.0,
            40.0,
            100.0,
            60.0,
            "logical",
            "screen",
            provenance("10,10,90,40"),
        ));
        let failure = check_failure(check, &mismatch);
        assert_eq!(failure.check, "coordinates_in_viewport");
        assert_eq!(
            failure.message,
            "coordinate space `logical` does not match expected `screen`"
        );
        assert_eq!(failure.provenance, provenance("10,10,90,40"));
    }

    /// A finite number passes; NaN and infinity fail with the check's own
    /// message.
    /// 有限数通过；NaN 与无穷失败，并给出该检查自己的消息。
    #[test]
    fn finite_number_accepts_finite_and_rejects_nan_and_infinity() {
        let check = RuntimeCheckSpec::FiniteNumber;
        assert!(
            check
                .run(&RuntimeValue::number(1.0, provenance("1.0")))
                .is_ok(),
            "1.0 is finite"
        );

        let failure = check_failure(check, &RuntimeValue::number(f64::NAN, provenance("NaN")));
        assert_eq!(failure.check, "finite_number");
        assert_eq!(failure.message, "number `NaN` is NaN or infinite");
        assert_eq!(failure.provenance, provenance("NaN"));

        let failure = check_failure(
            check,
            &RuntimeValue::number(f64::INFINITY, provenance("inf")),
        );
        assert_eq!(failure.check, "finite_number");
        assert_eq!(failure.message, "number `inf` is NaN or infinite");
        assert_eq!(failure.provenance, provenance("inf"));
    }

    /// The range is inclusive at both ends; a value outside it and an inverted
    /// `min > max` range fail, and the inverted range reports the bounds rather
    /// than the value.
    /// 区间两端都包含；越界值与 `min > max` 的倒置区间失败，且倒置区间报告的是边界
    /// 而不是取值。
    #[test]
    fn number_in_range_is_inclusive_and_reports_inverted_bounds() {
        let check = RuntimeCheckSpec::NumberInRange { min: 0, max: 10 };
        assert!(
            check
                .run(&RuntimeValue::number(0.0, provenance("0.0")))
                .is_ok(),
            "the lower bound is inclusive"
        );
        assert!(
            check
                .run(&RuntimeValue::number(10.0, provenance("10.0")))
                .is_ok(),
            "the upper bound is inclusive"
        );

        let failure = check_failure(check, &RuntimeValue::number(10.5, provenance("10.5")));
        assert_eq!(failure.check, "number_in_range");
        assert_eq!(
            failure.message,
            "number `10.5` is outside inclusive range 0..=10"
        );
        assert_eq!(failure.provenance, provenance("10.5"));

        // A range that can never be satisfied is a check-authoring mistake, so
        // the evidence names the bounds; it must not be confused with the
        // out-of-range message.
        // 永远无法满足的区间是检查书写的错误，因此证据命名边界；不能与越界消息混淆。
        let inverted = RuntimeCheckSpec::NumberInRange { min: 10, max: 0 };
        let failure = check_failure(inverted, &RuntimeValue::number(5.0, provenance("5.0")));
        assert_eq!(failure.check, "number_in_range");
        assert_eq!(
            failure.message,
            "invalid check bounds: minimum 10 exceeds maximum 0"
        );
        assert_eq!(failure.provenance, provenance("5.0"));
    }

    /// Non-blank text passes; the empty string and whitespace-only text fail
    /// with the same message, because both are blank.
    /// 非空白文本通过；空串与纯空白文本以同一条消息失败，因为二者都是空白。
    #[test]
    fn non_empty_text_rejects_empty_and_whitespace_only() {
        let check = RuntimeCheckSpec::NonEmptyText;
        assert!(
            check.run(&RuntimeValue::text("x", provenance("x"))).is_ok(),
            "one visible character is not empty"
        );

        for blank in ["", "   "] {
            let failure = check_failure(check, &RuntimeValue::text(blank, provenance(blank)));
            assert_eq!(failure.check, "non_empty_text");
            assert_eq!(failure.message, "text is empty or whitespace only");
            assert_eq!(failure.provenance, provenance(blank));
        }
    }

    /// `text_length` counts Unicode scalar values, not bytes, so a
    /// three-character Japanese word passes a `1..=3` range while a fourth
    /// character fails.
    /// `text_length` 统计 Unicode 标量值而不是字节，因此三字日语词通过 `1..=3`
    /// 区间，第四个字符失败。
    #[test]
    fn text_length_counts_characters_not_bytes() {
        let check = RuntimeCheckSpec::TextLength { min: 1, max: 3 };
        assert!(
            check
                .run(&RuntimeValue::text("abc", provenance("abc")))
                .is_ok(),
            "three ASCII characters"
        );
        assert!(
            check
                .run(&RuntimeValue::text("日本語", provenance("日本語")))
                .is_ok(),
            "three characters even though they are nine bytes"
        );

        let failure = check_failure(check, &RuntimeValue::text("abcd", provenance("abcd")));
        assert_eq!(failure.check, "text_length");
        assert_eq!(
            failure.message,
            "text length 4 is outside inclusive range 1..=3"
        );
        assert_eq!(failure.provenance, provenance("abcd"));
    }
}
