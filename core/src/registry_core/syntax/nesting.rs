//! The nesting pre-scan that keeps a stack overflow out of `syn`.
//! 把栈溢出挡在 `syn` 之外的嵌套预扫描。
//!
//! `syn` is recursive descent without a depth guard of its own, and
//! `proc-macro2` guards only its lexer, so a deeply nested source **aborts the
//! process**: a stack overflow is not a catchable panic, and no `Result` can
//! report it. `parse_faces(&"(".repeat(60_000))` is the smallest reproducer.
//! `syn` 是没有自带深度守卫的递归下降解析器，而 `proc-macro2` 只守了它自己的词法器，因此
//! 深层嵌套的源码会让进程 **abort**：栈溢出不是可捕获的 panic，任何 `Result` 都报不出来。
//! `parse_faces(&"(".repeat(60_000))` 是最小的复现。
//!
//! Three shapes are measured here, and the third is the one that took a second
//! pass to find: delimiter groups (`(`, `[`, `{`), generic-argument chains
//! (`Vec<Vec<…>>`), and a **linear run** — tokens the parser folds into one
//! nested expression or type without any delimiter or angle bracket to count,
//! such as `& & & …`, `* * * …`, `! ! ! …`, `|| || || …` or `1 + 1 + 1 + …`.
//!
//! The third shape is why this is about the *tree* and not only the parse: a run
//! of binary operators is parsed by a loop, but it produces a left-nested
//! `ExprBinary` whose **`Drop` recurses once per operator**, so 60 000 `+` tokens
//! abort the process the same way 60 000 parentheses do. Measured: a run of
//! 16 384 survives even a 256 KiB stack, while 60 000 aborts on an eight-megabyte
//! one, which is why the run limit below is a thousand tokens rather than the
//! parser-recursion limit of 128.
//!
//! The scan runs on `proc-macro2`'s own token stream rather than on the raw text,
//! because that is where string literals and comments have already been removed —
//! a lexical scan of the text would count the brackets inside a `"…"` and refuse a
//! source rustc accepts. The walk is iterative for the same reason the guard
//! exists: measuring nesting must not itself nest.
//! 这里量三种形状，而第三种是第二轮才找到的：定界符组（`(`、`[`、`{`）、泛型实参链
//! （`Vec<Vec<…>>`），以及**线性串**——解析器会折叠成一个嵌套表达式或类型、却没有任何定界符
//! 或尖括号可供计数的 token 串，例如 `& & & …`、`* * * …`、`! ! ! …`、`|| || || …` 或
//! `1 + 1 + 1 + …`。
//!
//! 第三种形状正是这件事关乎**树**而不只是解析的原因：一串二元运算符是用循环解析的，但它产出
//! 一个左嵌套的 `ExprBinary`，而它的 **`Drop` 每个运算符递归一层**，因此六万个 `+` token 与
//! 六万个括号一样会让进程 abort。实测：16 384 个的串即使在 256 KiB 栈上也活着，而 60 000 个
//! 在八兆栈上会 abort——这就是下面的串上限取一千个 token、而不是解析递归上限 128 的原因。
//!
//! 扫描跑在 `proc-macro2` 自己的 token 流上而不是原始文本上，因为字符串字面量与注释在那里已经
//! 被剔除——按文本做词法扫描会把 `"…"` 里的括号算进去，从而拒绝一份 rustc 能接受的源码。遍历是
//! 迭代的，理由与守卫本身相同：量嵌套这件事自己不能嵌套。

use std::fmt;

use super::face::{FaceSyntaxError, syntax_error};

/// The deepest nesting a source may reach.
/// 源码允许达到的最大嵌套深度。
///
/// 128 is `rustc`'s own default `recursion_limit`, so nothing a compiler accepts
/// is refused here, and the value is a number someone else already chose for the
/// same job.
/// 128 是 `rustc` 自己的默认 `recursion_limit`，因此编译器能接受的源码不会在这里被拒，
/// 而这个数字是别人为同一件事已经选过的。
pub(crate) const LIMIT: usize = 128;

/// The longest linear run a source may reach, in tokens.
/// 源码允许达到的最长线性串，以 token 计。
///
/// A run is measured between separators: `,` and `;` end one, and so does a brace
/// group, because a body or an item ends a chain in real code. Parentheses and
/// brackets do not end one, since `x.f().f()…` and `foo(a, b)` both keep folding
/// across them.
///
/// 1024 is chosen from measurements rather than taste: a run of 16 384 survives
/// even a 256 KiB stack while 60 000 aborts an eight-megabyte one, so this is a
/// sixteen-fold margin below the smallest run ever observed to survive, and this
/// repository's own longest run is about 91 tokens. The gate that keeps it honest
/// is `core/tests/nesting_budget.rs`: it feeds every Rust file in the workspace to
/// the guarded entry point and fails if the guard refuses one. The parser-recursion
/// limit of [`LIMIT`] does not apply here: these tokens produce one nested tree
/// rather than nested parser calls.
/// 1024 取自实测而不是口味：16 384 个的串即使在 256 KiB 栈上也活着，而 60 000 个能在八兆栈上
/// abort，因此这里比"观察到能活下来的串里最小的那个"还低十六倍，而本仓库自己最长的一条串约
/// 91 个 token。让它保持诚实的是 `core/tests/nesting_budget.rs`：它把工作区里每个 Rust 文件都
/// 喂给带守卫的入口，只要守卫拒了其中一个就失败。[`LIMIT`] 那条解析递归上限在这里不适用：
/// 这些 token 产出的是**一棵**嵌套树，而不是嵌套的解析调用。
pub(crate) const CHAIN_LIMIT: usize = 1024;

/// Which nesting shape went past [`LIMIT`].
/// 哪一种嵌套形状越过了 [`LIMIT`]。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shape {
    /// `(`, `[` and `{` groups.
    /// `(`、`[` 与 `{` 组。
    Delimiters,
    /// A run of tokens folded into one nested expression or type, with no
    /// delimiter to count: `& & & …`, `1 + 1 + …`, `Vec<Vec<…>>`.
    /// 被折叠成一个嵌套表达式或类型、却没有定界符可数的 token 串：`& & & …`、
    /// `1 + 1 + …`、`Vec<Vec<…>>`。
    ///
    /// A generic-argument chain used to have its own counter here
    /// ([`Shape::Arguments`]). It could never fire: the counter reset on every
    /// identifier and every generic argument begins with one, so the deepest
    /// `Vec<Vec<u8>>` the counter ever saw was one. The cases it was meant to catch
    /// are linear runs — `Vec<` repeated is a run of tokens the parser folds — which
    /// is why the variant is gone and the chain shape below covers it.
    /// 泛型实参链过去在这里有自己的计数器（[`Shape::Arguments`]）。它永远不会触发：计数器每遇到
    /// 一个标识符就清零，而每个泛型实参都以标识符开头，因此它见过的最深 `Vec<Vec<u8>>` 只有一层。
    /// 它本想抓的那些情况都是线性串——重复的 `Vec<` 正是解析器会折叠的 token 串——因此该变体已
    /// 删除，由下面的串形状覆盖。
    Chain,
}

impl fmt::Display for Shape {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Delimiters => "delimiters",
            Self::Chain => "tokens folded into one expression",
        })
    }
}

/// A measured nesting depth above [`LIMIT`].
/// 实测超过 [`LIMIT`] 的嵌套深度。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TooDeep {
    /// Which shape was too deep.
    /// 过深的是哪种形状。
    pub(crate) shape: Shape,
    /// How deep it actually went.
    /// 实际深到多少层。
    pub(crate) depth: usize,
    /// The limit that applies to `shape`, so the message names the number the
    /// reader has to compare against instead of one borrowed from another shape.
    /// 适用于 `shape` 的上限，使消息说明读者真正要比对的那个数字，而不是从另一种形状借来的。
    pub(crate) limit: usize,
}

impl fmt::Display for TooDeep {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "input nests {} levels of {}, above the limit of {}; \
             it is refused because the parser would overflow the stack instead of reporting an error",
            self.depth, self.shape, self.limit
        )
    }
}

/// Refuse a source whose nesting would overflow the parser.
/// 拒绝一份嵌套会让解析器栈溢出的源码。
///
/// A source the lexer already refuses is left to `syn`: the scan exists for input
/// that lexes fine and then recurses, and taking over the lexer's own diagnostic
/// would only lose the position it carries.
/// 词法器已经拒绝的源码留给 `syn` 报：本扫描是为"能词法通过、随后递归"的输入存在的，
/// 抢过词法器自己的诊断只会丢掉它携带的位置。
pub(crate) fn guard(source: &str) -> Result<(), TooDeep> {
    let Ok(stream) = source.parse::<proc_macro2::TokenStream>() else {
        return Ok(());
    };
    let mut stack: Vec<(proc_macro2::TokenTree, usize)> =
        stream.into_iter().map(|tree| (tree, 0)).collect();
    // Consecutive `Ident`/`<`/`>` tokens are the only run that can grow a generic
    // argument chain; every other token kind ends the run. That keeps a file full
    // of `a < b` comparisons from ever reaching the limit, while `Vec<Vec<…>>`
    // grows one level per `<`.
    // 连续出现的 `Ident`/`<`/`>` 是唯一能长出泛型实参链的序列，任何其他词法单元都会终结该
    // 序列。因此通篇 `a < b` 比较的文件永远到不了上限，而 `Vec<Vec<…>>` 每遇一个 `<` 长一层。
    // Tokens folded into one tree since the last separator. `chain += 1` appears
    // in every arm that continues a run, so a new token kind cannot silently
    // escape the measurement the way the prefix operators escaped the first
    // version of this scan.
    // 自上一个分隔符以来被折叠进同一棵树的 token 数。每个延续串的分支里都写着 `chain += 1`，
    // 因此新增一种词法单元不会像前缀运算符逃过本扫描的第一版那样，静默地逃过这个度量。
    let mut chain = 0usize;
    while let Some((tree, depth)) = stack.pop() {
        let mut grows = true;
        match tree {
            proc_macro2::TokenTree::Group(group) => {
                let depth = depth + 1;
                if depth > LIMIT {
                    return Err(TooDeep {
                        shape: Shape::Delimiters,
                        depth,
                        limit: LIMIT,
                    });
                }
                // A brace group is a body or an item: it ends a chain. The other
                // delimiters stay inside the expression that holds them, which is
                // what lets `x.f().f()…` be counted at all.
                // 花括号组是函数体或条目：它终结一条串。其他定界符留在持有它们的表达式内部，
                // 这正是 `x.f().f()…` 能被数到的原因。
                if group.delimiter() == proc_macro2::Delimiter::Brace {
                    grows = false;
                }
                stack.extend(group.stream().into_iter().map(|tree| (tree, depth)));
            }
            proc_macro2::TokenTree::Punct(punct)
                if punct.as_char() == ',' || punct.as_char() == ';' =>
            {
                grows = false;
            }
            _ => {}
        }
        if grows {
            chain += 1;
            if chain > CHAIN_LIMIT {
                return Err(TooDeep {
                    shape: Shape::Chain,
                    depth: chain,
                    limit: CHAIN_LIMIT,
                });
            }
        } else {
            chain = 0;
        }
    }
    Ok(())
}

/// Refuse a source whose nesting would overflow `syn`, for callers that parse
/// text themselves.
/// 对自行解析文本的调用方：拒绝一份嵌套会让 `syn` 栈溢出的源码。
///
/// `parse_file` applies this to every `syn::parse_file` in this crate. It is
/// public because it is the *only* nesting measurement in the workspace: the
/// documentation gate in `conventions` hands fenced Rust to `syn` too, and a
/// pathological fence would abort that gate's process rather than fail it. A
/// second copy of the heuristic there would be a second thing to keep honest;
/// asking the kernel keeps one.
/// `parse_file` 把本函数施加于本 crate 里每一处 `syn::parse_file`。它公开是因为它是本工作区
/// **唯一**的嵌套度量：`conventions` 里的文档门禁也会把围栏 Rust 交给 `syn`，而一份病态围栏
/// 会让那道门禁的进程 abort 而不是失败。在那里复制一份启发式就等于多一个需要保持诚实的东西；
/// 向内核发问则只有一份。
pub fn guard_nesting(source: &str) -> Result<(), FaceSyntaxError> {
    guard(source).map_err(|error| FaceSyntaxError {
        message: error.to_string(),
        location: None,
    })
}

/// Parse one Rust file, refusing nesting that would overflow the parser.
/// 解析一个 Rust 文件，并拒绝会让解析器栈溢出的嵌套。
///
/// Every `syn::parse_file` call site in this crate goes through here, so the
/// guard cannot be forgotten at a new entry point without the code looking
/// wrong.
/// 本 crate 里每一处 `syn::parse_file` 都经这里，因此新增入口若漏掉守卫，代码看上去就是错的。
pub(crate) fn parse_file(source: &str) -> Result<syn::File, FaceSyntaxError> {
    guard(source).map_err(|error| FaceSyntaxError {
        message: error.to_string(),
        location: None,
    })?;
    syn::parse_file(source).map_err(|error| syntax_error(error.span(), error.to_string()))
}
