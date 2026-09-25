//! Pathologically nested input must be refused, not fatal.
//! 病态嵌套的输入必须被拒绝，而不是致命。
//!
//! `syn` is recursive descent with no depth guard of its own, and `proc-macro2`
//! protects only its lexer. Without the pre-scan these cases pin, a source that
//! nested a few tens of thousands of delimiters aborted the process: a stack
//! overflow is not a catchable panic, so one malformed buffer took the whole
//! surface down instead of producing the `FaceSyntaxError` the signature
//! promises. The cases below are orders of magnitude past anything rustc accepts
//! (its own default recursion limit is 128), so refusing them costs no real
//! source anything.
//! `syn` 是无自带深度守卫的递归下降解析器，而 `proc-macro2` 只保护了它自己的词法器。在
//! 这些用例钉住的预扫描出现之前，嵌套几万个定界符的源码会让进程 abort：栈溢出不是可捕获的
//! panic，因此一个畸形缓冲区就能带走整个执行面，而不是给出签名承诺的 `FaceSyntaxError`。
//! 下面的用例比 rustc 能接受的深出几个数量级（它自己的默认递归上限是 128），因此拒绝它们
//! 不会让任何真实源码付出代价。

use super::parse_faces;

/// Deep enough to overflow an eight-megabyte stack in the unguarded parser.
/// 深到足以让没有守卫的解析器在八兆栈上溢出。
const PAST_THE_STACK: usize = 60_000;

/// The refusal must name the reason, not just fail.
/// 拒绝必须说出原因，而不是单纯失败。
fn refused(case: &str, source: &str) {
    match parse_faces(source) {
        Ok(faces) => panic!(
            "{case}: expected a syntax error, but parsed {} faces",
            faces.len()
        ),
        Err(error) => assert!(
            error.to_string().contains("nest"),
            "{case}: the refusal must explain itself: {error}"
        ),
    }
}

#[test]
fn deeply_nested_parentheses_are_refused() {
    let source = format!(
        "const X: u8 = {}1{};",
        "(".repeat(PAST_THE_STACK),
        ")".repeat(PAST_THE_STACK)
    );
    refused("balanced parentheses", &source);
}

#[test]
fn an_unclosed_run_of_parentheses_is_refused() {
    // This one never overflowed: `proc-macro2`'s lexer refuses an unterminated
    // group before `syn` sees it, and the scan deliberately leaves the lexer's
    // own diagnostic alone because it carries the position. The test pins that
    // the shape stays a `Result` rather than becoming fatal later.
    // 这一条从不溢出：未闭合的组在 `syn` 看到之前就被 `proc-macro2` 的词法器拒绝，而扫描
    // 有意不抢词法器自己的诊断，因为它带位置。本测试钉住这种形状始终是 `Result`，而不是
    // 以后变成致命失败。
    let source = "(".repeat(PAST_THE_STACK);
    assert!(
        parse_faces(&source).is_err(),
        "an unterminated group must be an error, not a parse"
    );
}

#[test]
fn deeply_nested_braces_are_refused() {
    let source = format!(
        "fn f() {}{}",
        "{".repeat(PAST_THE_STACK / 3),
        "}".repeat(PAST_THE_STACK / 3)
    );
    refused("nested blocks", &source);
}

#[test]
fn deeply_nested_angle_brackets_are_refused() {
    let source = format!(
        "pub struct S(pub {}u8{});",
        "Vec<".repeat(PAST_THE_STACK / 2),
        ">".repeat(PAST_THE_STACK / 2)
    );
    refused("nested generic arguments", &source);
}

// Candidates for the third recursive shape: syn also recurses once per *prefix
// operator* (`Type::Reference`, `Expr::Unary`, `Expr::Closure`), which no
// delimiter group and no `<` grows. Each is a separate test so an abort in one
// does not hide the others.
// 第三种递归形状的候选：syn 对**前缀运算符**也每次递归一层（`Type::Reference`、
// `Expr::Unary`、`Expr::Closure`），而它们既不长定界符组也不长 `<`。每条各是一个测试，
// 因此一条 abort 不会掩盖其余几条。

#[test]
fn a_long_unary_reference_chain_is_refused() {
    let source = format!("pub struct S(pub {}u8);", "&".repeat(PAST_THE_STACK));
    refused("unary references", &source);
}

#[test]
fn a_long_mutable_reference_chain_is_refused() {
    let source = format!("pub struct S(pub {}u8);", "&mut ".repeat(PAST_THE_STACK));
    refused("mutable references", &source);
}

#[test]
fn a_long_lifetime_reference_chain_is_refused() {
    let source = format!("pub struct S<'a>(pub {}u8);", "&'a ".repeat(PAST_THE_STACK));
    refused("lifetime references", &source);
}

#[test]
fn a_long_dereference_chain_is_refused() {
    let source = format!("pub const X: () = {}&true;", "*".repeat(PAST_THE_STACK));
    refused("dereferences", &source);
}

#[test]
fn a_long_negation_chain_is_refused() {
    let source = format!("pub const X: i32 = {}1;", "-".repeat(PAST_THE_STACK));
    refused("negations", &source);
}

#[test]
fn a_long_not_chain_is_refused() {
    let source = format!("pub const X: bool = {}true;", "!".repeat(PAST_THE_STACK));
    refused("not-operators", &source);
}

#[test]
fn a_long_closure_chain_is_refused() {
    let source = format!("pub const X: () = {}1;", "|| ".repeat(PAST_THE_STACK / 2));
    refused("closures", &source);
}

#[test]
fn a_long_binary_addition_chain_is_refused() {
    // This one was written as the control — a binary chain is parsed by a
    // precedence-climbing loop, so it was expected to parse — and it aborted
    // instead. The parse is iterative; the *tree* is not. `1 + 1 + …` builds a
    // left-nested `ExprBinary` whose `Drop` recurses once per operator, so the
    // process dies on the way out of a parse that succeeded. That is why the
    // guard measures folded tokens and not only parser recursion.
    // 这一条原本是作为对照组写的——二元链由优先级爬升的循环解析，因此预期是被解析——结果它
    // abort 了。解析是迭代的，**树**不是。`1 + 1 + …` 建出一个左嵌套的 `ExprBinary`，它的
    // `Drop` 每个运算符递归一层，于是进程在离开一次已经成功的解析时死掉。这就是守卫要量"被
    // 折叠的 token"、而不只是解析递归的原因。
    let source = format!("pub const X: u32 = {}1;", "1 + ".repeat(PAST_THE_STACK));
    refused("binary chains", &source);
}
