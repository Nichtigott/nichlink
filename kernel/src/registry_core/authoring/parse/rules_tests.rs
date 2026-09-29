//! The registration-rule source reader: what the text branch reads, and what the strict
//! sibling refuses.
//! 注册规则源读取器：文本分支读了什么，严格兄弟拒绝了什么。

use super::*;

/// A rule file the shape the renderer emits, plus a commented-out clause.
/// 一个渲染器发射形状的规则文件，外加一条被注释掉的子句。
const COMMENTED_CLAUSE: &str = r#"use crate::RegistrationRule;

pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()
    // 旧写法: .require_exports(&["wrong"])
    .require_exports(&["control.render"]);
"#;

/// A rule whose clause names its list through a constant instead of spelling the
/// literals inline, which the text branch cannot see.
/// 一条用常量而不是内联字面量来指名其列表的规则，文本分支看不见它。
const LIST_THROUGH_A_CONSTANT: &str = r#"use crate::RegistrationRule;

const EXPORTS: [&str; 1] = ["control.render"];

pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()
    .require_exports(&EXPORTS);
"#;

/// An `=` before the rule's own initializer, which the text branch reads past.
/// 出现在规则自身初始器之前的 `=`，文本分支会读过去。
const EQUAL_SIGN_BEFORE_THE_INITIALIZER: &str = r#"use crate::RegistrationRule;

const OLD_NOTE: &str = "preset:SomePreset from before";

pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()
    .require_exports(&["control.render"]);
"#;

/// The loose branch reads the commented clause, which is why the strict sibling exists.
/// 宽容分支会把注释里的子句读进去——这正是严格兄弟存在的理由。
///
/// This is the red side reproduces on the shipped entry: it is *asserted*, not described,
/// so the value the tolerant reader produces for a commented pseudo-clause is pinned
/// next to the strict reader that refuses to read it (audit `KRN-K-10`).
/// 这是出厂入口上的红侧复现：它以**断言**形式写下，而不是描述，因此宽容读取器对"注释里的伪子句"
/// 产出的取值与拒绝读它的严格读取器并排钉住（审计 `KRN-K-10`）。
#[test]
fn the_loose_branch_reads_a_commented_clause() {
    assert_eq!(
        rule_syntax_from_text(COMMENTED_CLAUSE),
        "exports:wrong",
        "the first `\\\".require_exports(\\\"` occurrence wins, and the one in the comment comes \
         first — so the real requirement is dropped and a clause nobody declared is the rule"
    );
}

/// The loose branch silently degrades to `ANY` when it cannot see a clause's value.
/// 宽容分支看不见子句取值时会静默降级成 `ANY`。
///
/// The rule above really requires `exports:control.render`; the text branch looks for a
/// quoted literal after `.require_exports(` and, finding none (the list is named by a
/// constant), drops the clause — no error, no diagnostic. `ANY` is exactly the "no
/// structural requirement" value, so a caller cannot tell a read rule from an unread one,
/// and the structural requirement the author declared is gone (audit `KRN-K-10`).
/// 上面那条规则实际要求 `exports:control.render`；文本分支在 `.require_exports(` 之后找引号字面量，
/// 找不到（列表由常量指名），于是丢掉该子句——不报错、不给诊断。`ANY` 恰恰是"没有结构要求"的取值，
/// 因此调用方分不清"读到的规则"与"没读到的规则"，而作者声明的结构要求就此消失（审计 `KRN-K-10`）。
#[test]
fn the_loose_branch_degrades_silently() {
    assert_eq!(
        rule_syntax_from_text(LIST_THROUGH_A_CONSTANT),
        "ANY",
        "the text branch loses the rule that is really there"
    );
    // The same input is refused, not degraded, by the strict sibling: the caller learns
    // that the reader cannot read this file instead of silently reading no rule.
    // 同一份输入在严格兄弟那里被拒绝而不是降级：调用方得知"读不了这个文件"，而不是静默地读到"无规则"。
    assert!(
        try_rule_syntax_from_text(LIST_THROUGH_A_CONSTANT).is_err(),
        "a shape the reader cannot recognize is refused"
    );
}

/// A clause inside a comment is not part of the rule.
/// 注释里的子句不属于规则。
#[test]
fn a_commented_clause_is_not_the_rule() {
    assert_eq!(
        try_rule_syntax_from_text(COMMENTED_CLAUSE).expect("the const is readable"),
        "exports:control.render",
        "only the const's own initializer decides the rule"
    );
}

/// An `=` outside the initializer cannot mislead the strict reader.
/// 初始器之外的 `=` 误导不了严格读取器。
#[test]
fn an_equal_sign_outside_the_initializer_is_read_past() {
    assert_eq!(
        try_rule_syntax_from_text(EQUAL_SIGN_BEFORE_THE_INITIALIZER)
            .expect("the const is readable"),
        "exports:control.render"
    );
}

/// Shapes the strict reader cannot recognize are refused, not degraded.
/// 严格读取器认不出的形状会被拒绝，而不是降级。
#[test]
fn an_unrecognizable_source_is_refused() {
    for (label, source) in [
        ("no const at all", "pub fn unrelated() {}\n"),
        (
            "a const that is not a rule",
            "pub const REGISTRATION_RULE: u32 = 7;\n",
        ),
        (
            "an unknown clause",
            "pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()\n    .require_everything(&[\"x\"]);\n",
        ),
        (
            "a clause the text branch would have read as a string",
            "pub const REGISTRATION_RULE: RegistrationRule = \"exports:x\";\n",
        ),
        ("source that is not Rust", "this is not rust at all $$$ \n"),
    ] {
        assert!(
            try_rule_syntax_from_text(source).is_err(),
            "{label} must be refused"
        );
    }
}

/// `RegistrationRule::ANY` and `new()` are both read as "no requirement".
/// `RegistrationRule::ANY` 与 `new()` 都读作"没有要求"。
#[test]
fn the_empty_spellings_are_read_as_any() {
    for source in [
        "pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::ANY;\n",
        "pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new();\n",
    ] {
        assert_eq!(
            try_rule_syntax_from_text(source).expect("the const is readable"),
            "ANY"
        );
    }
}
