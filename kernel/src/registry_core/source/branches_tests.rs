//! Pins for the branch-level read: rule A, rule B's three premises, and the masking trap.
//! 分支级读取的钉子：规则 A、规则 B 的三个前提，以及掩码陷阱。

use super::{BranchFacts, branch_facts};

/// The arms of one file, as the read reports them.
/// 一个文件的臂，按读取报告的样子。
fn facts(source: &str) -> BranchFacts {
    branch_facts(source)
}

/// Rule A decides exactly one literal: `false`. Everything a constant evaluator would be needed
/// for stays out, and a guard over a field or a parameter is never decided at all.
/// 规则 A 只认一个字面量：`false`。需要常量求值的一切都留在外面，而对字段或参数的守卫一律不判。
#[test]
fn only_a_literal_false_guard_is_decided() {
    let source = "\
fn f(flag: bool, amount: i64, limit: i64) {
    if false { return 1; }
    else if  false  { return 2; }
    if !true { return 3; }
    if 1 == 2 { return 4; }
    if flag { return 5; }
    if amount > limit { return 6; }
}
";
    let read = facts(source);
    assert_eq!(
        read.false_guards.len(),
        2,
        "only the two literal-`false` guards: {:?}",
        read.false_guards
    );
    assert!(
        read.false_guards.iter().all(|guard| guard.function == "f"),
        "the row names the function the guard sits in: {:?}",
        read.false_guards
    );
    assert_eq!(
        read.false_guards
            .iter()
            .map(|guard| guard.line)
            .collect::<Vec<_>>(),
        [2, 3],
        "the row reports the line of the `if` token"
    );
}

/// The guard line, not the arm's line: a caller who has to look at the source is sent to the text
/// that was decided.
/// 报守卫行而不是臂的行：要去看源码的调用方被送到被判定过的文本上。
#[test]
fn a_false_guard_reports_the_line_of_its_own_condition() {
    let source = "fn f() {\n    if\n        false\n    {\n        return;\n    }\n}\n";
    let read = facts(source);
    assert_eq!(read.false_guards.len(), 1, "{:?}", read.false_guards);
    assert_eq!(
        read.false_guards[0].line, 2,
        "the `if` token is on line 2: {:?}",
        read.false_guards
    );
}

/// Rule B's pattern half: a qualified variant path is an arm, an or-pattern is two, and a wildcard,
/// a binding or a literal is not read at all.
/// 规则 B 的模式那一半：限定变体路径是一个臂，或模式是两个，而通配符、绑定或字面量一律不读。
#[test]
fn only_a_qualified_variant_pattern_is_an_arm() {
    let source = "\
enum Band { Small, Frozen }

fn f(band: Band) -> &'static str {
    match band {
        Band::Small => \"small\",
        Band::Frozen => \"frozen\",
        _ => \"other\",
    }
}

fn g(band: Band) -> i32 {
    match band {
        Band::Small | Band::Frozen => 1,
        other => 2,
    }
}

fn h(band: Band) -> i32 {
    match band { Band::Small if band as i32 > 0 => 1, Band::Frozen => 2 }
}
";
    let read = facts(source);
    let arms = read
        .matched_variants
        .iter()
        .map(|arm| {
            (
                arm.function.as_str(),
                arm.enum_name.as_str(),
                arm.variant.as_str(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        arms,
        [
            ("f", "Band", "Small"),
            ("f", "Band", "Frozen"),
            ("g", "Band", "Small"),
            ("g", "Band", "Frozen"),
            ("h", "Band", "Small"),
            ("h", "Band", "Frozen"),
        ],
        "the wildcard, the binding and the guard are not arms of this rule: {:?}",
        read.matched_variants
    );
    assert_eq!(
        read.matched_variants[1].line, 6,
        "the row reports the arm's pattern line: {:?}",
        read.matched_variants
    );
    // The enum's two premises are read from the declaration itself.
    assert_eq!(read.enums.len(), 1, "{:?}", read.enums);
    assert!(
        read.enums[0].judged,
        "a private enum with no attribute is judged"
    );
}

/// The trap the read exists to survive: `Enum::Variant` written inside a comment or a string is not
/// a construction, and one written inside a `match` arm's own pattern is not one either.
/// 这次读取存在的理由——那个陷阱：写在注释或字符串里的 `Enum::Variant` 不是构造，而写在某个 `match`
/// 臂自己模式里的那个也不是。
#[test]
fn a_variant_in_a_comment_a_string_or_a_pattern_is_not_a_construction() {
    let source = "\
enum Band { Small, Frozen }

// Band::Frozen is nowhere constructed in this tree.
fn f(band: Band) {
    let _ = \"Band::Frozen\";
    match band { Band::Frozen => {} _ => {} }
}
";
    let read = facts(source);
    let paths = read
        .variant_paths
        .iter()
        .map(|path| format!("{}::{}", path.enum_name, path.variant))
        .collect::<Vec<_>>();
    assert!(
        paths.is_empty(),
        "the comment, the string and the arm's own pattern are not constructions: {:?}",
        read.variant_paths
    );
}

/// The four places a construction *is* spelled, and the one place it is not: a `use` declaration
/// imports a name, it does not build a value.
/// 构造**确实**被拼出的四处，以及它不被拼出的一处：`use` 声明引入名字，并不造值。
#[test]
fn a_construction_is_read_outside_patterns_and_use_lines() {
    let source = "\
use crate::Band::Frozen;

enum Band { Small, Frozen }

fn f() -> Band {
    let a = Band::Frozen;
    let b = (Band::Frozen);
    let c = Some(Band::Frozen);
    consume(Band::Frozen);
    a
}

fn g() -> Band { Band::Small }
";
    let read = facts(source);
    let paths = read
        .variant_paths
        .iter()
        .map(|path| (path.enum_name.as_str(), path.variant.as_str(), path.line))
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        [
            ("Band", "Frozen", 6),
            ("Band", "Frozen", 7),
            ("Band", "Frozen", 8),
            ("Band", "Frozen", 9),
            ("Band", "Small", 13),
        ],
        "the `use` line on line 1 is not a construction: {:?}",
        read.variant_paths
    );
}

/// Rule B's visibility premise: a declaration beginning with `pub` is never judged, and neither is
/// one carrying an attribute that could build a value.
/// 规则 B 的可见性前提：以 `pub` 开头的声明一律不判，带着能造值的属性的声明也一样。
#[test]
fn a_public_enum_and_a_building_derive_are_not_judged() {
    let source = "\
pub enum Public { Open, Dormant }

#[derive(Default)]
enum Defaulted { Small, Frozen }

#[derive(Debug, PartialEq)]
enum Plain { Small, Frozen }

#[non_exhaustive]
enum Open { Small, Frozen }

enum Bare { Small, Frozen }

pub(crate) enum Scoped { Small, Frozen }

fn f(value: Public) {
    match value { Public::Open => {} Public::Dormant => {} }
}
";
    let read = facts(source);
    let judged = read
        .enums
        .iter()
        .map(|declaration| (declaration.name.as_str(), declaration.judged))
        .collect::<Vec<_>>();
    assert_eq!(
        judged,
        [
            ("Public", false),
            ("Defaulted", false),
            ("Plain", true),
            ("Open", false),
            ("Bare", true),
            ("Scoped", false),
        ],
        "only a declaration that cannot be built from outside and cannot build a value is judged"
    );
    // The `pub` declaration's arm is still read — the join, not this read, decides it is not
    // reported. A read that dropped the arm here could not tell "not judged" from "not seen".
    // `pub` 声明的臂仍然会被读到——是连接而不是这次读取决定它不被报告。在这里丢掉该臂的读取就无法把
    // "不判"与"没看见"区分开。
    assert_eq!(
        read.matched_variants.len(),
        2,
        "{:?}",
        read.matched_variants
    );
}

/// A declaration with no attribute above it at all is judged, and a blank line between the derives
/// and the declaration does not hide them.
/// 正上方没有任何属性的声明会被判定，而派生与声明之间的空行不会把它们藏起来。
#[test]
fn attributes_are_collected_across_blank_lines() {
    let source = "\
#[derive(Clone)]

// A doc comment, which is prose.

#[derive(Copy)]
enum Band { Small, Frozen }
";
    let read = facts(source);
    assert_eq!(read.enums.len(), 1, "{:?}", read.enums);
    assert!(
        read.enums[0].judged,
        "value-free derives across blank lines are still value-free: {:?}",
        read.enums
    );
}

/// A `match` nested inside an arm's body is read like any other, and the enclosing function is the
/// innermost one.
/// 嵌在臂体里的 `match` 与其它 `match` 一样被读到，而所属函数是最内层的那个。
#[test]
fn a_nested_match_is_read_and_attributed_to_the_innermost_function() {
    let source = "\
enum Outer { A, B }

fn outer(value: Outer) -> i32 {
    fn inner(value: Outer) -> i32 {
        match value { Outer::A => 1, Outer::B => 2 }
    }
    match value {
        Outer::A => inner(value),
        Outer::B => if false { 3 } else { 4 },
    }
}
";
    let read = facts(source);
    let functions = read
        .matched_variants
        .iter()
        .map(|arm| (arm.function.as_str(), arm.variant.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        functions,
        [
            ("inner", "A"),
            ("inner", "B"),
            ("outer", "A"),
            ("outer", "B"),
        ],
        "{:?}",
        read.matched_variants
    );
    assert_eq!(read.false_guards.len(), 1, "{:?}", read.false_guards);
}

/// An arm written without a trailing comma still ends where its block ends, so the arm after it is
/// read rather than swallowed.
/// 没有尾随逗号的臂仍然在它的块结束处结束，因此它后面的臂会被读到而不是被吞掉。
#[test]
fn an_arm_whose_body_is_a_block_ends_at_its_closing_brace() {
    let source = "\
enum Band { Small, Frozen }

fn f(band: Band) -> i32 {
    match band {
        Band::Small => { let x = 1; x }
        Band::Frozen => { 2 }
    }
}
";
    let read = facts(source);
    let variants = read
        .matched_variants
        .iter()
        .map(|arm| arm.variant.as_str())
        .collect::<Vec<_>>();
    assert_eq!(variants, ["Small", "Frozen"], "{:?}", read.matched_variants);
}

/// A struct-variant pattern, an or-pattern and an arm guard are all still patterns, and none of
/// them is read as a construction.
/// 结构体变体模式、或模式与臂守卫都仍是模式，其中任何一个都不会被读成构造。
#[test]
fn a_struct_variant_pattern_is_a_pattern() {
    let source = "\
enum Shape { Round { radius: i32 }, Square }

fn f(shape: Shape) -> i32 {
    match shape {
        Shape::Round { radius } => radius,
        Shape::Square => if false { 0 } else { 1 },
    }
}
";
    let read = facts(source);
    assert!(
        read.variant_paths.is_empty(),
        "both arms are patterns, not constructions: {:?}",
        read.variant_paths
    );
    assert_eq!(
        read.matched_variants.len(),
        2,
        "{:?}",
        read.matched_variants
    );
    assert_eq!(read.false_guards.len(), 1, "{:?}", read.false_guards);
}

/// The names a file imports from **outside** its own crate, which is what keeps a two-segment
/// pattern from being read as this tree's own enum. A `crate`/`self`/`super` root stays inside the
/// crate and is therefore not recorded.
/// 一个文件从**自己 crate 之外**引入的名字——正是它让两段模式不被读成本树自己的枚举。根为 `crate`/
/// `self`/`super` 的留在本 crate 内，因此不被记录。
#[test]
fn a_use_from_outside_the_crate_is_recorded_as_a_foreign_import() {
    let source = "\
use ledger_core::Band;
pub use other::{Entry, State};
use crate::model::Local;
use super::Parent;
use self::Sibling;
";
    let read = facts(source);
    let mut names = read.foreign_imports.clone();
    names.sort();
    assert_eq!(
        names,
        ["Band", "Entry", "State", "ledger_core", "other"],
        "every identifier of a foreign `use` is recorded, and no in-crate root is"
    );
    assert!(
        read.variant_paths.is_empty(),
        "none of these lines is a construction: {:?}",
        read.variant_paths
    );
}

/// The read is a text read: an arm outside any function is not attributed to one, and the caller
/// is told so rather than handed an invented name.
/// 这次读取是文本读取：不在任何函数里的臂不会被归给某个函数，调用方会被告知这一点，而不是拿到一个
/// 凭空造出的名字。
#[test]
fn an_arm_outside_a_function_has_no_function_name() {
    let source = "macro_rules! m { () => { if false { } } }\n";
    let read = facts(source);
    assert_eq!(read.false_guards.len(), 1, "{:?}", read.false_guards);
    assert!(
        read.false_guards[0].function.is_empty(),
        "no function encloses it: {:?}",
        read.false_guards
    );
}
