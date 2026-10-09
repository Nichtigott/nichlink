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

use super::{guard_nesting, parse_faces};

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

#[test]
fn the_refusal_names_the_limit_that_applies() {
    // The delimiter limit is rustc's own recursion limit; the linear-run limit is
    // this workspace's, because the overflowing structure there is the tree rather
    // than the parser. A message that printed one number for both was wrong for
    // whichever shape it did not belong to.
    // 定界符上限就是 rustc 自己的递归上限；线性串上限是本工作区的，因为那里溢出的是**树**而不是
    // 解析器。对两种形状都印同一个数字，必然对它不属于的那一种说错。
    let delimiters = format!("const X: u8 = {}1{};", "(".repeat(300), ")".repeat(300));
    let error = parse_faces(&delimiters)
        .expect_err("nesting is refused")
        .to_string();
    assert!(error.contains("limit of 128"), "{error}");

    let run = format!("pub const X: u8 = {}1;", "&".repeat(2000));
    let error = parse_faces(&run)
        .expect_err("nesting is refused")
        .to_string();
    assert!(
        error.contains("limit of 1024"),
        "a linear run is measured against its own limit: {error}"
    );

    // A generic-argument chain is refused by the *parser* limit, and it has to be
    // measured on the angle brackets themselves: `X<u8, X<u8, …>>` carries a comma at
    // every level, so the linear-run counter — which a comma resets, because a comma
    // really does end a run — saw a run of one while the parser descended hundreds of
    // levels and overflowed the stack.
    // 泛型实参链由**解析器**上限拒绝，而且必须在尖括号本身上度量：`X<u8, X<u8, …>>`
    // 每一层都带一个逗号，因此（逗号会重置的）线性串计数器看到的串长度是 1，而解析器下潜了几百层
    // 并撑爆了栈。
    let arguments = format!(
        "pub struct S(pub X{}u8{});",
        "<u8, X".repeat(200),
        ">".repeat(200)
    );
    // The guard alone is asked here, not `parse_faces`: on the unfixed guard this
    // input returned `Ok` and the *parser* aborted the process, and a test cannot
    // assert anything on the far side of an abort. Measuring the guard directly makes
    // the red half of this pin reproducible — it fails as an assertion instead of
    // taking the test binary down.
    // 这里只问守卫,不经过 `parse_faces`:在未修复的守卫上,这个输入返回 `Ok`,而**解析器**会让
    // 进程 abort,测试无法在 abort 的另一侧断言任何东西。直接度量守卫,让这条钉子的"红"可复现——
    // 它表现为断言失败,而不是把测试二进制带走。
    let error = guard_nesting(&arguments)
        .expect_err("nesting is refused")
        .to_string();
    assert!(
        error.contains("limit of 128") && error.contains("generic arguments"),
        "a comma-separated generic chain is measured on its brackets: {error}"
    );
}

/// The arrow-cancelled chain the 2026-09-27 audit measured: 95 levels of `X<fn() -> …>`
/// returned `Ok` from the guard and then aborted `syn` with a stack overflow. The guard is
/// asked alone so the red half is an assertion failure rather than an abort.
/// 2026-09-27 审计实测的那条被箭头抵消的链：95 层 `X<fn() -> …>` 从守卫拿到 `Ok`，随后
/// `syn` 爆栈 abort。这里只问守卫，使红色半边是断言失败而不是 abort。
#[test]
fn an_arrow_cancelled_chain_is_refused() {
    let source = format!("type T = {}T{};", "X<fn() -> ".repeat(95), ">".repeat(95));
    let error = guard_nesting(&source)
        .expect_err("an arrow's `>` must not cancel the depth it sits inside")
        .to_string();
    assert!(error.contains("generic arguments"), "{error}");
}

/// An arrow-dense chain: every level carries four `fn` pointers, which costs `syn` far
/// more stack per level than a plain path does. A flat bracket count admitted 45 of these
/// and the process still aborted, which is why a level is weighted by the keywords it
/// carries.
/// 一条箭头密集的链：每一层带四个 `fn` 指针，`syn` 为此每层付出的栈远比一条普通路径多。平铺地
/// 数尖括号会放行 45 层这种形状，进程照样 abort——这就是每一层要按它携带的关键字计权的原因。
#[test]
fn an_arrow_dense_chain_is_refused() {
    let source = format!(
        "type T = {}T{};",
        "X<fn() -> fn() -> fn() -> fn() -> ".repeat(45),
        ">".repeat(45)
    );
    let error = guard_nesting(&source)
        .expect_err("a `fn`-heavy level weighs more than one")
        .to_string();
    assert!(error.contains("generic arguments"), "{error}");
}

/// A comparison is not generic nesting, however many statements carry one. The counter
/// only ever grew on `<`, so 129 separate `let _ = 1 < 2;` statements were refused as a
/// 129-level generic chain although the file contains no `>` at all.
/// 比较不是泛型嵌套，无论多少条语句带它。计数器过去只在 `<` 处增长，于是 129 条互不相干的
/// `let _ = 1 < 2;` 被当成 129 层泛型链拒绝，而整个文件里根本没有 `>`。
#[test]
fn comparisons_are_not_generic_nesting() {
    let source: String = (0..129)
        .map(|index| format!("fn f{index}() {{ let _ = 1 < 2; }}\n"))
        .collect();
    guard_nesting(&source).expect("a `<` comparison opens no generic level");
}

/// A long documentation block is documentation, not a run.
/// 一长段文档是文档，不是一条串。
///
/// `//!` becomes `#[doc = "…"]`: five tokens with **no separator anywhere** between the lines of a
/// block, so a 213-line module header measured 1025 tokens and the guard refused it. That is not a
/// hypothetical: taken from the local registry cache, `tokio/src/fs/mod.rs` has exactly that header and
/// `nichlink check .` refused it, along with four more of that crate's files — a guard that only accepts
/// this repository's own sources refuses the trees it exists to be pointed at (audit `M7`, §M7.56).
/// `//!` 会变成 `#[doc = "…"]`：五个 token，而一个文档块各行之间**没有任何分隔符**，因此一段 213 行的
/// 模块头量出 1025 个 token，守卫拒绝了它。这不是假设：取自本机 registry 缓存的 `tokio/src/fs/mod.rs`
/// 正是那样一段头，`nichlink check .` 拒绝了它，同一个 crate 还有四个文件同样被拒——一个只接受本仓库
/// 自己源码的守卫，会拒绝它本来就是要被指向的那些树（审计 `M7`，§M7.56）。
///
/// The count is chosen to be **red on the old guard**: 3000 lines ≈ 15 000 tokens, fifteen times the run
/// limit, so this cannot pass by being near a threshold.
/// 这个行数是**按"旧的守卫会红"选的**：3000 行 ≈ 15 000 个 token，是串上限的十五倍，因此它不可能靠
/// 贴着阈值蒙过去。
#[test]
fn a_documentation_block_is_not_a_run() {
    let source: String = (0..3000)
        .map(|index| format!("//! line {index} of a module header\n"))
        .collect();
    guard_nesting(&source).expect("a doc block is metadata, not a linear run");
}

/// The attribute that ends a run must not hide one: a real chain behind one is still refused.
/// 终结一条串的属性不得把串藏起来：它后面一条真串仍要被拒。
///
/// The fix for the case above was to treat `#` as a separator, and this is its other half — a
/// separator that also swallowed the expression after it would be a guard that stopped guarding.
/// 上面那个用例的修法是把 `#` 当作分隔符，而这是它的另一半——一个连它后面的表达式也一起吞掉的分隔符，
/// 就是一个不再守任何东西的守卫。
#[test]
fn an_attribute_does_not_hide_a_real_run() {
    let source = format!("#[allow(unused)]\n{};\n", "1 + ".repeat(2000) + "1");
    let error = guard_nesting(&source)
        .expect_err("2000 operators are still a run")
        .to_string();
    assert!(error.contains("above the limit of"), "{error}");
}
