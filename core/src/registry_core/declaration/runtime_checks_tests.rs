//! Boundary tests for the five runtime checks.
//! 五条运行期检查的边界测试。
//!
//! A separate page so the module under test stays inside the size ratchet: a test
//! module is excluded from it, and this one was the larger third of the file.
//! 独立一页，使被测模块留在尺寸棘轮之内：测试模块不受棘轮约束，而这一份是文件中更大的
//! 那三分之一。
//!
//! Why a direct "call `run` and look at `is_err`" implementation would be wrong:
//! each check owns several distinct failure arms (wrong kind, non-finite geometry,
//! coordinate-space mismatch, inverted bounds, unrepresentable bounds, range
//! overflow), and a boolean-only assertion cannot tell them apart — a check that
//! rejected *every* value would pass it. These tests therefore assert the exact
//! [`RuntimeCheckFailure`] evidence a host consumes: `check`, `message`, and
//! `provenance`.
//! Boundary: `RuntimeCheckFailure` deliberately does not implement `PartialEq`, so
//! the assertions are field-level rather than whole-struct.
//! 直白写法错在哪：只调用 `run` 再看 `is_err` 会掩盖每条检查各自的多个失败分支（类型
//! 不符、坐标非有限、坐标系不符、边界倒置、边界无法精确表示、越界），而"拒绝一切取值"的
//! 实现也能通过布尔断言。因此这些测试断言宿主真正消费的 [`RuntimeCheckFailure`] 证据：
//! `check`、`message` 与 `provenance`。
//! 边界：`RuntimeCheckFailure` 刻意不实现 `PartialEq`，所以断言按字段进行而非整结构比较。

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

/// A bound f64 cannot state exactly is refused instead of compared. The old
/// comparison rounded both bounds through f64, so
/// `number_in_range(9007199254740993, 9007199254740993)` accepted
/// `9007199254740992.0` — a number below its own minimum.
/// f64 无法精确说出的边界会被拒绝，而不是拿来比较。旧的比较把两个边界都经 f64 舍入，
/// 因此 `number_in_range(9007199254740993, 9007199254740993)` 会接受
/// `9007199254740992.0`——一个低于它自己下限的数。
#[test]
fn number_in_range_refuses_a_bound_it_cannot_state_exactly() {
    let beyond = RuntimeCheckSpec::NumberInRange {
        min: 9_007_199_254_740_993,
        max: 9_007_199_254_740_993,
    };
    let failure = check_failure(
        beyond,
        &RuntimeValue::number(9_007_199_254_740_992.0, provenance("9007199254740992")),
    );
    assert_eq!(failure.check, "number_in_range");
    assert!(
        failure.message.contains("cannot be represented exactly"),
        "{}",
        failure.message
    );

    // The largest exactly representable bound still compares, so the refusal
    // is about precision and not about magnitude.
    // 仍然精确可表示的最大边界照常比较，因此这次拒绝针对的是精度而不是大小。
    let exact = RuntimeCheckSpec::NumberInRange {
        min: 9_007_199_254_740_992,
        max: 9_007_199_254_740_992,
    };
    assert!(
        exact
            .run(&RuntimeValue::number(
                9_007_199_254_740_992.0,
                provenance("x")
            ))
            .is_ok()
    );
    assert!(
        exact
            .run(&RuntimeValue::number(
                9_007_199_254_740_994.0,
                provenance("y")
            ))
            .is_err()
    );
}
