//! The compact registration-rule spelling is the kernel's public entry point.
//! 紧凑注册规范拼法是内核公开的入口。
//!
//! Studio needed this rendering and, because the kernel kept the owned-value
//! renderer private, wrote a second copy of the same rules (`FIXR-01`, the same
//! family as the admission copy pinned by `core/tests/compact_admission_entry.rs`:
//! `studio/src/studio/app/source_index.rs` assembled its `preset:/parts:/exports:`
//! clauses itself). This file is the outside-of-the-crate pin: it reaches the
//! renderer through `nichlink::authoring::parse`, which is exactly the path a
//! surface has, so a private renderer cannot satisfy it — this pin stops compiling
//! instead.
//! Studio 需要这份渲染，而内核把“从拥有型取值渲染”的入口设成私有，于是它写出了同一套规则的
//! 第二份副本（`FIXR-01`，与 `core/tests/compact_admission_entry.rs` 钉的 admission 副本同族：
//! `studio/src/studio/app/source_index.rs` 自己装配 `preset:/parts:/exports:` 子句）。本文件是
//! crate 之外的钉子：它经 `nichlink::authoring::parse` 取渲染器，这正是执行面拥有的那条路径，
//! 因此私有渲染器无法满足它——钉子会直接编译失败，而不是悄悄通过。
//!
//! The second half is the proof that exposing the renderer changed nothing on the
//! wire: the historical source-text branch (`rule_syntax_from_text`) now renders
//! through the public function, and the bytes it produces must be the bytes
//! consumers already read. The table below is that before/after对照: every row was
//! asserted against the private implementation too.
//! 另一半是“公开渲染器没有改动线上字节”的证明：历史源码文本分支（`rule_syntax_from_text`）现在
//! 经公开函数渲染，而它产出的字节必须正是消费方已在读的字节。下面那张表就是前后对照：每一行在私有
//! 实现上也断言过。

// The renderer lives behind the parser's feature; without `syntax` there is nothing
// here to call. A whole-workspace build turns it on through other members.
// 渲染器在解析器的特性之后；没有 `syntax` 时这里无物可调。整工作区构建会经其他成员打开它。
#![cfg(feature = "syntax")]

use nichlink::authoring::parse::{
    compact_registration_rule, parse_registration_rule_owned, rule_syntax_from_text,
};
use nichlink::declaration::OwnedRegistrationRule;

/// One rule in the owned form the renderer takes, from field spellings.
/// 渲染器取用的拥有型规则，由各字段的拼法构造。
fn rule(
    preset: Option<&str>,
    parts: &[&str],
    exports: &[&str],
    handle_traits: &[&str],
    part_traits: &[&str],
) -> OwnedRegistrationRule {
    let owned = |values: &[&str]| {
        values
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>()
    };
    OwnedRegistrationRule {
        required_preset: preset.map(str::to_owned),
        required_parts: owned(parts),
        required_exports: owned(exports),
        required_handle_traits: owned(handle_traits),
        required_part_traits: owned(part_traits),
    }
}

/// The historical rule sources and the bytes their compact form already had.
/// 历史的规则源码，以及它们的紧凑形式早已有的字节。
///
/// Read by the source-text test below; every pair is a spelling this repository
/// ships or a shape the compiler accepts, so the merge must not move any of them.
/// 由下面的源码文本测试读取；每一对都是本仓库出厂的拼法或编译器接受的形状，因此并入不得移动其中
/// 任何一个字节。
const HISTORICAL_SOURCES: &[(&str, &str)] = &[
    (
        "pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::ANY;",
        "ANY",
    ),
    (
        "pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new();",
        "ANY",
    ),
    (
        "pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()\
         .require_preset(\"ActionParts\");",
        "preset:ActionParts",
    ),
    (
        "pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()\
         .require_parts(&[\"paint\", \"shape\"]);",
        "parts:paint,shape",
    ),
    (
        "pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()\
         .require_exports(&[\"control.render\"]);",
        "exports:control.render",
    ),
    (
        "pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()\
         .require_handle_traits(&[\"ControlHandle\"]);",
        "handle:ControlHandle",
    ),
    (
        "pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()\
         .require_part_traits(&[\"ActionParts\"]);",
        "part_trait:ActionParts",
    ),
    (
        "pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()\n\
         .require_exports(&[\"control.render\"])\n\
         .require_handle_traits(&[\"ControlHandle\"]);",
        "exports:control.render;handle:ControlHandle",
    ),
    (
        "pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()\
         .require_preset(\"ActionParts\")\
         .require_parts(&[\"paint\"])\
         .require_exports(&[\"control.render\"])\
         .require_handle_traits(&[\"ControlHandle\"])\
         .require_part_traits(&[\"ActionParts\"]);",
        "preset:ActionParts;parts:paint;exports:control.render;handle:ControlHandle;part_trait:ActionParts",
    ),
    // The degenerate spelling the historical branch already had: a present but
    // empty preset renders `preset:` (an empty *list* clause is skipped instead).
    // The merge must keep that byte too, which is why it is pinned rather than
    // "fixed" here — `render_registration_rule` never emits it and the parser
    // refuses it, so no surface can reach it, but the bytes still may not move.
    // 历史分支早就有的退化拼法：preset 出现但为空时渲染 `preset:`（而空*列表*子句会被跳过）。
    // 并入后这个字节也必须保持，因此这里把它钉住而不是顺手“修好”——
    // `render_registration_rule` 从不产生它、解析器也拒绝它，因此没有执行面能抵达，
    // 但字节仍不得移动。
    (
        "pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()\
         .require_preset(\"\");",
        "preset:",
    ),
];

/// The source-text branch renders the historical bytes, unchanged by the merge.
/// 源码文本分支渲染出历史字节，不因并入而改变。
///
/// This is the same table before and after routing the branch through the public
/// renderer, so a green run here is the byte-identity proof, not a restatement of
/// the new implementation.
/// 并入前后用的是同一张表，因此这里跑绿就是逐字节不变的证明，而不是对新实现的重述。
#[test]
fn historical_rule_spellings_render_unchanged_bytes() {
    for (source, expected) in HISTORICAL_SOURCES {
        let rendered = rule_syntax_from_text(source);
        assert_eq!(rendered, *expected, "the bytes of `{source}` moved");
    }
}

/// The owned rule a compact spelling parses into still renders as that spelling.
/// 紧凑拼法解析出的拥有型规则，仍然渲染回同一种拼法。
///
/// Kept for the case above: it says the branch and the value agree, which is what
/// lets the public renderer become the only spelling.
/// 为上面的情形保留：它说明分支与取值一致，而正是这一点让公开渲染器可以成为唯一的拼法来源。
#[test]
fn the_source_text_branch_agrees_with_the_parsed_value() {
    for (source, expected) in HISTORICAL_SOURCES {
        // The degenerate `preset:` row is refused by the parser (see the table),
        // so there is no value to agree with; every other row must parse.
        // 退化行 `preset:` 被解析器拒绝（见上表），因此没有可对照的取值；其余每一行都必须可解析。
        let Ok(parsed) = parse_registration_rule_owned(expected) else {
            continue;
        };
        let from_branch = parse_registration_rule_owned(&rule_syntax_from_text(source))
            .expect("the branch renders a readable value");
        assert_eq!(
            from_branch, parsed,
            "the branch and the value disagree for `{source}`"
        );
    }
}

/// The compact renderer is reachable from outside the crate, and its output is
/// the spelling this crate's own parser reads back.
/// 紧凑渲染器可从 crate 之外调用，且它的输出正是本 crate 自己的解析器读得回的拼法。
///
/// This is the pin `FIXR-01` needed a third time: with the renderer private the call
/// below does not compile, which is what pushed Studio into assembling the clauses
/// itself.
/// 这正是 `FIXR-01` 第三次需要的钉子：渲染器私有时，下面这次调用编译不过——而后者正是把 Studio
/// 逼出自己装配子句的原因。
#[test]
fn the_compact_renderer_is_reachable_from_outside_the_crate() {
    assert_eq!(
        compact_registration_rule(&rule(None, &[], &[], &[], &[])),
        "ANY"
    );
    assert_eq!(
        compact_registration_rule(&rule(Some("ActionParts"), &[], &[], &[], &[])),
        "preset:ActionParts"
    );
    assert_eq!(
        compact_registration_rule(&rule(None, &["paint", "shape"], &[], &[], &[])),
        "parts:paint,shape"
    );
    assert_eq!(
        compact_registration_rule(&rule(None, &[], &["control.render"], &[], &[])),
        "exports:control.render"
    );
    assert_eq!(
        compact_registration_rule(&rule(None, &[], &[], &["ControlHandle"], &[])),
        "handle:ControlHandle"
    );
    assert_eq!(
        compact_registration_rule(&rule(None, &[], &[], &[], &["ActionParts"])),
        "part_trait:ActionParts"
    );
    // The full spelling, in the historical order — the same bytes the source-text
    // table above pins, which is what makes the two directions one grammar.
    // 完整的拼法，顺序沿用历史——与上面源码文本表钉住的是同一串字节，这正是“两个方向一套语法”的
    // 含义。
    assert_eq!(
        compact_registration_rule(&rule(
            Some("ActionParts"),
            &["paint"],
            &["control.render"],
            &["ControlHandle"],
            &["ActionParts"],
        )),
        "preset:ActionParts;parts:paint;exports:control.render;handle:ControlHandle;part_trait:ActionParts"
    );
}

/// One grammar, two directions: every value the renderer emits reads back as the
/// rule it was given, or a surface following it writes a rule the kernel refuses.
/// 一套语法、两个方向：渲染器吐出的每个值都必须读回它被给的那份规则，否则照着它写的执行面会写出
/// 内核随后拒绝的规则。
#[test]
fn the_renderer_and_the_parser_read_the_same_rule() {
    for policy in [
        rule(None, &[], &[], &[], &[]),
        rule(Some("ActionParts"), &[], &[], &[], &[]),
        rule(None, &["paint"], &[], &[], &[]),
        rule(None, &[], &["control.render"], &[], &[]),
        rule(None, &[], &[], &["ControlHandle"], &[]),
        rule(None, &[], &[], &[], &["ActionParts"]),
        rule(
            Some("ActionParts"),
            &["paint", "shape"],
            &["control.render"],
            &["ControlHandle"],
            &["ActionParts"],
        ),
    ] {
        let text = compact_registration_rule(&policy);
        assert_eq!(
            parse_registration_rule_owned(&text).expect("the kernel reads the spelling it emits"),
            policy,
            "{text}"
        );
    }
}

/// The merged historical branch renders through that very function, byte for byte.
/// 被并入的历史分支正是走这个函数渲染的，逐字节一致。
#[test]
fn the_source_text_branch_renders_through_the_public_renderer() {
    for (source, expected) in HISTORICAL_SOURCES {
        let Ok(parsed) = parse_registration_rule_owned(expected) else {
            // The degenerate `preset:` row is refused by the parser; its value is
            // the one with an empty preset and no lists, spelled the same way.
            // 退化行 `preset:` 被解析器拒绝；它的取值正是“preset 为空且没有任何列表”的那一份，
            // 拼法相同。
            assert_eq!(
                rule_syntax_from_text(source),
                compact_registration_rule(&rule(Some(""), &[], &[], &[], &[]))
            );
            continue;
        };
        assert_eq!(
            rule_syntax_from_text(source),
            compact_registration_rule(&parsed),
            "the branch and the public renderer disagree for `{source}`"
        );
    }
}
