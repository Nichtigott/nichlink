//! The `graft_plan!` / `static_graft_plan!` entry grammar.
//! `graft_plan!` / `static_graft_plan!` 入口语法。
//!
//! A graft cut names both sides the same way — either two Rust expressions or
//! two strings — so the generated table never mixes a resolved identity with
//! an unresolved name.
//! 一条 graft 切口的两侧命名方式必须一致——要么两个 Rust 表达式，要么两个字符串——
//! 生成表因此不会混合已解析身份与未解析名称。

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::visit::Visit;

use super::super::face::{compact, split_typed_range};
use super::super::{FaceSyntaxError, SyntaxLocation, compact_tokens, location, syntax_error};

/// A host-declared external graft cut discovered before code generation.
/// 在代码生成前从宿主入口发现的一条外部 graft 切口。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraftSyntax {
    /// The cut target: a logical path, or the Rust expression text of a typed
    /// cut. For a range this is the *start* endpoint, never `"start to end"`.
    /// 切口目标：逻辑路径，或类型化切口的 Rust 表达式原文。区间切口这里是**起点**，
    /// 绝不是 `"start to end"`。
    pub cut: String,
    /// The far endpoint of a sibling range, as data.
    /// 兄弟区间的远端端点，以数据形式携带。
    ///
    /// Why the joined-string encoding was wrong: a logical path is free-form
    /// text, so once `"a to b"` is stored in `cut` nothing downstream can tell a
    /// range from a single path that literally contains `" to "` — the four
    /// consumers that re-split `cut` cut such a path in half. The boundary is
    /// this field: only the parser may decide that a `to` token was a range
    /// separator, and every consumer reads the endpoints from `cut`/`cut_end`.
    /// Pinned by `a_path_containing_the_range_word_is_a_single_cut` (this file)
    /// and `a_string_range_keeps_both_endpoints` (`build_method`).
    /// 拼接字符串的编码错在哪：逻辑路径是自由文本，一旦把 `"a to b"` 存进 `cut`，
    /// 下游就再也分不清区间与一条字面含有 `" to "` 的路径——四处重新拆分 `cut` 的
    /// 消费方会把这种路径拦腰截断。边界就是本字段：只有解析器能判定某个 `to` token
    /// 是区间分隔符，所有消费方都从 `cut`/`cut_end` 读取端点。
    /// 由本文件的 `a_path_containing_the_range_word_is_a_single_cut` 与
    /// `build_method` 的 `a_string_range_keeps_both_endpoints` 钉住。
    pub cut_end: Option<String>,
    /// The replacement side, written the same way as `cut`: a logical selector, a
    /// Rust expression, or an identity.
    /// 替换侧，与 `cut` 使用同一种写法：逻辑选择器、Rust 表达式或身份。
    pub graft: String,
    /// Whether the replacement covers the whole subtree at the cut target rather
    /// than only its node.
    /// 替换是否覆盖切口目标的整棵子树，而不只是该节点自身。
    pub full: bool,
    /// Where the declaration was written, for diagnostics that point at it.
    /// 声明写在哪，供指向它的诊断使用。
    pub location: SyntaxLocation,
    /// The `cfg` gate the declaration carries, if any, exactly as written — with the
    /// gates of the modules around it combined in as `all(…)`, because a declaration
    /// inside a gated module inherits that gate and the build step, not this parser,
    /// is the one that evaluates it (audit `LGC-LG-29`).
    /// 声明携带的 `cfg` 门控（若有），按原文保留——并把它周围各模块的门控以 `all(…)` 并入：
    /// 被门控模块里的声明继承那道门控，而求值的是构建步骤而不是本解析器（审计 `LGC-LG-29`）。
    pub cfg: Option<String>,
    /// Present when `cut(...)` / `graft(...)` supplied Rust expressions. The
    /// renderer emits those expressions verbatim, so the compiler — and any
    /// editor that resolves Rust paths — sees the real target instead of a
    /// string the tooling would have to interpret.
    /// 当 `cut(...)` / `graft(...)` 给出 Rust 表达式时存在。渲染器会原样发射这些
    /// 表达式，因此编译器和任何能解析 Rust 路径的编辑器看到的都是真实目标，
    /// 而不是需要工具自己解释的字符串。
    pub expressions: Option<GraftExpressions>,
}

/// Rust expressions used by a typed graft cut, in source order.
/// 类型化 graft 切口使用的 Rust 表达式，按源码顺序。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraftExpressions {
    /// The cut target's Rust expression text.
    /// 切口目标的 Rust 表达式原文。
    ///
    /// For a typed range this is the start endpoint; the far one is `cut_end`.
    /// 类型化区间时这是起点，远端在 `cut_end`。
    pub cut: String,
    /// The far endpoint of a typed range, as data.
    /// 类型化区间的远端端点，以数据形式携带。
    pub cut_end: Option<String>,
    /// The replacement's Rust expression text.
    /// 替换件的 Rust 表达式原文。
    pub graft: String,
}

/// Collects the graft declarations an item-position `graft_plan!` states.
/// 收集条目位置的 `graft_plan!` 所声明的 graft 切口。
struct GraftVisitor<'a> {
    entries: &'a mut Vec<GraftSyntax>,
    error: Option<FaceSyntaxError>,
    /// The `cfg` gates of the modules this visitor is currently inside, outermost
    /// first. A declaration inside a gated module inherits them, because a gate the
    /// build can evaluate belongs to the declaration it guards — not to the parser,
    /// which cannot evaluate any of them (audit `LGC-LG-29`).
    /// 访问器当前所在各模块的 `cfg` 门控，最外层在前。被门控模块里的声明会继承它们：构建能求值
    /// 的门控属于它守护的那条声明，而不属于解析器——解析器一个门控也求值不了（审计 `LGC-LG-29`）。
    module_cfgs: Vec<String>,
}
impl<'ast> Visit<'ast> for GraftVisitor<'_> {
    /// Only a declaration written as an item belongs in the build's plan: a
    /// `graft_plan!` inside a function is a runtime expression, and a plan
    /// inside `#[cfg(test)] mod tests` describes tests. Both used to be
    /// captured, shipping test cuts into the release table and pinning faces
    /// against pruning.
    /// 只有写成条目的声明才属于构建计划：函数里的 `graft_plan!` 是运行时表达式，
    /// `#[cfg(test)] mod tests` 里的计划描述的是测试。两者过去都会被收集，把测试
    /// 切口带进发布表，并让注册面躲过剪枝。
    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        self.collect(item);
    }

    /// Walk a module unless the compiler can only drop it, and carry its gate onto
    /// what it contains.
    /// 只要编译器只可能丢弃它就走进去，并把它的门控下传给其中的内容。
    ///
    /// The gate has to be read, not merely noticed: *any* `cfg` attribute used to
    /// skip the whole module body, so a plan inside `#[cfg(feature = "fast")]` was
    /// dropped even when the feature was on — the slot silently kept running the
    /// base implementation while the source said otherwise, and nothing reported
    /// the missing cut (audit `LGC-LG-29`). Only a test-only expression is a gate
    /// this parse can settle, because a build plan is not a test build; every other
    /// gate is *kept* and attached to the declarations inside the module, where the
    /// build step evaluates it with the same rule it already applies to an
    /// entry-level gate (`build_method::graft_view::host_graft_entries`).
    /// 门控要**读**，不能只注意到：过去**任何** `cfg` 属性都会让整个模块体被跳过，于是
    /// `#[cfg(feature = "fast")]` 里的计划即便特性开着也被丢掉——槽位静默地继续跑基座实现，而
    /// 源码写着相反的话，没有任何诊断报出少掉的切口（审计 `LGC-LG-29`）。只有"仅测试"的表达式是
    /// 本次解析能定论的，因为构建计划不是测试构建；其余门控一律**保留**并附到模块内的声明上，
    /// 由构建步骤用它与条目级门控完全相同的那条规则求值
    /// （`build_method::graft_view::host_graft_entries`）。
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        let gates = module_cfgs(item);
        if gates.iter().any(|gate| cfg_is_test_only(gate)) {
            return;
        }
        let depth = self.module_cfgs.len();
        self.module_cfgs.extend(gates);
        syn::visit::visit_item_mod(self, item);
        self.module_cfgs.truncate(depth);
    }
}

/// The `cfg` gates written on a module, in source order.
/// 写在模块上的 `cfg` 门控，按源码顺序。
fn module_cfgs(item: &syn::ItemMod) -> Vec<String> {
    item.attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("cfg"))
        .filter_map(|attribute| {
            attribute
                .parse_args::<TokenStream>()
                .ok()
                .map(|tokens| compact(&tokens))
        })
        .collect()
}

/// Whether a `cfg` expression is true in a test build and in no other.
/// 一个 `cfg` 表达式是否只在测试构建里为真。
///
/// The same three-state reading `conventions::size` uses for the same question, and
/// for the same reason: `not(test)` and `feature = "…"` are not test-only, so a
/// module carrying them must be visited. `test` alone is test-only; a conjunction is
/// test-only as soon as one conjunct is; a disjunction only when every branch is —
/// an empty `any()` is true in no build at all, not in a test one. An expression this
/// parse cannot read is not one it may skip: keeping it hands the gate to the build
/// step, which refuses what it cannot evaluate.
/// 与 `conventions::size` 对同一个问题使用的同一套三态读法，理由也相同：`not(test)` 与
/// `feature = "…"` 不是仅测试，带着它们的模块必须被访问。单独的 `test` 是仅测试；合取里只要
/// 有一个合取项是仅测试，整体就是；析取要求每个分支都是——空 `any()` 在任何构建里都不成立，
/// 而不是只在测试构建里成立。读不出来的表达式不是可以跳过的：保留它等于把门控交给构建步骤，
/// 而构建步骤会拒绝它求不了值的东西。
fn cfg_is_test_only(expression: &str) -> bool {
    syn::parse_str::<syn::Meta>(expression).is_ok_and(|meta| cfg_meta_is_test_only(&meta))
}

/// Whether one parsed `cfg` operand is test-only.
/// 一个已解析的 `cfg` 操作数是否仅测试。
fn cfg_meta_is_test_only(meta: &syn::Meta) -> bool {
    match meta {
        syn::Meta::Path(path) => path.is_ident("test"),
        syn::Meta::List(list) => {
            let Ok(nested) = list.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            ) else {
                return false;
            };
            if list.path.is_ident("all") {
                nested.iter().any(cfg_meta_is_test_only)
            } else if list.path.is_ident("any") {
                !nested.is_empty() && nested.iter().all(cfg_meta_is_test_only)
            } else {
                false
            }
        }
        syn::Meta::NameValue(_) => false,
    }
}

/// `entry`'s own gate combined with the gates of the modules around it.
/// `entry` 自己的门控与包围它的各模块门控的组合。
///
/// `None` means "no gate at all", which is not the same as an empty gate: the
/// downstream evaluator treats a missing gate as unconditional and refuses one it
/// cannot read.
/// `None` 表示"完全没有门控"，这与空门控不同：下游求值器把缺失的门控当作无条件，而读不出来的
/// 门控会被它拒绝。
fn combined_cfg(module_cfgs: &[String], own: Option<&str>) -> Option<String> {
    let mut gates = module_cfgs.to_vec();
    gates.extend(own.map(str::to_owned));
    match gates.len() {
        0 => None,
        1 => gates.pop(),
        _ => Some(format!("all({})", gates.join(", "))),
    }
}

impl<'a> GraftVisitor<'a> {
    fn collect(&mut self, item: &syn::ItemMacro) {
        let cfg = item.attrs.iter().find_map(|attribute| {
            attribute
                .path()
                .is_ident("cfg")
                .then(|| {
                    attribute
                        .parse_args::<TokenStream>()
                        .ok()
                        .map(|tokens| compact(&tokens))
                })
                .flatten()
        });
        let mac = &item.mac;
        let Some(name) = mac
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
        else {
            return;
        };
        if !matches!(name.as_str(), "graft_plan" | "static_graft_plan") {
            return;
        }
        let tokens = mac.tokens.clone().into_iter().collect::<Vec<_>>();
        let mut index = 0;
        while index < tokens.len() {
            if !matches!(&tokens[index], TokenTree::Ident(value) if value == "cut") {
                index += 1;
                continue;
            }
            index += 1;
            let path_token = tokens.get(index).cloned();
            index += 1;
            // `cut(<expr>)` / `cut(<expr> to <expr>)` names the target with
            // Rust expressions, so the compiler and any editor that resolves
            // Rust paths see the real face instead of a string the tooling
            // would have to interpret.
            // `cut(<expr>)` / `cut(<expr> to <expr>)` 用 Rust 表达式命名目标，
            // 让编译器和任何能解析 Rust 路径的编辑器看到真实的注册面，而不是
            // 需要工具自己解释的字符串。
            let typed_cut = match &path_token {
                Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
                    let inner = group.stream().into_iter().collect::<Vec<_>>();
                    Some(match split_typed_range(inner) {
                        Some((start, finish)) => (start, Some(finish)),
                        None => (compact_tokens(group.stream()), None),
                    })
                }
                _ => None,
            };
            let (cut, end, cut_span, typed) = match typed_cut {
                Some((start, finish)) => (start, finish, mac.span(), true),
                None => {
                    let (path_tokens, path_span, path_text) = match path_token {
                        Some(TokenTree::Group(group)) => (
                            group.stream().into_iter().collect::<Vec<_>>(),
                            group.span(),
                            None,
                        ),
                        Some(TokenTree::Literal(literal)) => {
                            let text = syn::parse_str::<syn::LitStr>(&literal.to_string())
                                .map(|value| value.value())
                                .map_err(|_| {
                                    syntax_error(literal.span(), "graft cut path must be a string")
                                });
                            let Ok(text) = text else {
                                self.error = text.err();
                                return;
                            };
                            (Vec::new(), literal.span(), Some(text))
                        }
                        _ => {
                            self.error = Some(syntax_error(
                                mac.span(),
                                "graft cut expects `[path]`, a path string, or `(<expression>)`",
                            ));
                            return;
                        }
                    };
                    let range = path_text.is_none()
                        && path_tokens.len() == 3
                        && matches!(&path_tokens[1], TokenTree::Ident(value) if value == "to");
                    let resolved = if range {
                        let start = syn::parse2::<syn::LitStr>(path_tokens[0].clone().into())
                            .map_err(|_| {
                                syntax_error(path_span, "graft range start must be a string")
                            });
                        let finish = syn::parse2::<syn::LitStr>(path_tokens[2].clone().into())
                            .map_err(|_| {
                                syntax_error(path_span, "graft range end must be a string")
                            });
                        match (start, finish) {
                            (Ok(start), Ok(finish)) => Ok((start.value(), Some(finish.value()))),
                            (Err(error), _) | (_, Err(error)) => Err(error),
                        }
                    } else {
                        Ok((
                            path_text.unwrap_or_else(|| {
                                syn::parse2::<syn::LitStr>(
                                    path_tokens.clone().into_iter().collect(),
                                )
                                .map(|value| value.value())
                                .unwrap_or_else(|_| {
                                    path_tokens
                                        .iter()
                                        .map(ToString::to_string)
                                        .collect::<String>()
                                })
                            }),
                            None,
                        ))
                    };
                    let (cut, end) = match resolved {
                        Ok(pair) => pair,
                        Err(error) => {
                            self.error = Some(error);
                            return;
                        }
                    };
                    (cut, end, path_span, false)
                }
            };
            if cut.is_empty() {
                self.error = Some(syntax_error(cut_span, "graft cut path is empty"));
                return;
            }
            let full =
                matches!(tokens.get(index), Some(TokenTree::Ident(value)) if value == "full");
            if full {
                index += 1;
            }
            if !matches!(tokens.get(index), Some(TokenTree::Ident(value)) if value == "graft") {
                self.error = Some(syntax_error(
                    mac.span(),
                    "graft cut expects `graft <implementation>`",
                ));
                return;
            }
            index += 1;
            let graft_token = tokens.get(index).cloned();
            let (graft, graft_expr) = match graft_token {
                Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
                    let text = compact_tokens(group.stream());
                    (text.clone(), Some(text))
                }
                Some(TokenTree::Literal(value)) => {
                    let Ok(text) = syn::parse_str::<syn::LitStr>(&value.to_string()) else {
                        self.error = Some(syntax_error(
                            value.span(),
                            "graft implementation must be a string literal or `(<expression>)`",
                        ));
                        return;
                    };
                    (text.value(), None)
                }
                _ => {
                    self.error = Some(syntax_error(
                        mac.span(),
                        "graft expects a string literal or `(<expression>)`",
                    ));
                    return;
                }
            };
            // A cut names both sides the same way, so the generated table
            // never has to mix a resolved identity with an unresolved name.
            // 一条切口的命名方式必须一致，生成表因此不会混合已解析身份与未解析名称。
            let expressions = match (typed, graft_expr) {
                (true, Some(graft)) => Some(GraftExpressions {
                    cut: cut.clone(),
                    cut_end: end.clone(),
                    graft,
                }),
                (false, None) => None,
                _ => {
                    self.error = Some(syntax_error(
                        mac.span(),
                        "a graft cut must name both sides with Rust expressions or both with strings",
                    ));
                    return;
                }
            };
            // The two endpoints stay separate fields; joining them into `cut`
            // would make a path that literally contains `" to "` indistinguish-
            // able from a range at every later consumer.
            // 两个端点保持独立字段；把它们拼进 `cut` 会让字面含有 `" to "` 的路径
            // 在之后每个消费方那里都与区间无法区分。
            self.entries.push(GraftSyntax {
                cut,
                cut_end: end,
                graft,
                full,
                location: location(mac.span()),
                cfg: combined_cfg(&self.module_cfgs, cfg.as_deref()),
                expressions,
            });
            index += 1;
            index += 1;
        }
    }
}

/// Parse static and dynamic graft declarations without executing them.
/// 解析静态和动态 graft 声明，不执行宏。
pub fn graft_entries(source: &str) -> Result<Vec<GraftSyntax>, FaceSyntaxError> {
    let file = super::super::nesting::parse_file(source)?;
    let mut entries = Vec::new();
    let mut visitor = GraftVisitor {
        entries: &mut entries,
        error: None,
        module_cfgs: Vec::new(),
    };
    visitor.visit_file(&file);
    visitor.error.map_or(Ok(entries), Err)
}

#[cfg(test)]
#[path = "graft_tests.rs"]
mod graft_tests;
