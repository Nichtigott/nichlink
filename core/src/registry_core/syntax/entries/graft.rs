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
    /// The `cfg` gate the declaration carries, if any, exactly as written.
    /// 声明携带的 `cfg` 门控（若有），按原文保留。
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

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        // A module the compiler may drop cannot contribute to the plan.
        // 编译器可能丢弃的模块不能参与计划。
        if item
            .attrs
            .iter()
            .any(|attribute| attribute.path().is_ident("cfg"))
        {
            return;
        }
        syn::visit::visit_item_mod(self, item);
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
                cfg: cfg.clone(),
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
    let file =
        syn::parse_file(source).map_err(|error| syntax_error(error.span(), error.to_string()))?;
    let mut entries = Vec::new();
    let mut visitor = GraftVisitor {
        entries: &mut entries,
        error: None,
    };
    visitor.visit_file(&file);
    visitor.error.map_or(Ok(entries), Err)
}

#[cfg(test)]
mod tests {
    use super::graft_entries;

    #[test]
    fn graft_parser_collects_single_and_full_cuts() {
        let source = r#"
nichlink::static_graft_plan!(FRAMEWORK,
    cut ["root/a1/b2"] graft "canvas_fast",
    cut ["root/a"] full graft "a_fast",
);
"#;
        let entries = graft_entries(source).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].cut, "root/a1/b2");
        assert!(!entries[0].full);
        assert_eq!(entries[1].graft, "a_fast");
        assert!(entries[1].full);
    }

    #[test]
    fn graft_parser_keeps_range_endpoints() {
        let source = r#"nichlink::graft_plan!(framework, cut ["root/a1" to "root/a3"] graft "replacement");"#;
        let entries = graft_entries(source).unwrap();
        // Both endpoints survive as separate data; the start is `cut`, the far
        // endpoint is `cut_end`.
        // 两个端点都作为独立数据保留：起点是 `cut`，远端是 `cut_end`。
        assert_eq!(entries[0].cut, "root/a1");
        assert_eq!(entries[0].cut_end.as_deref(), Some("root/a3"));
    }

    /// A single string path that happens to contain `" to "` is not a range: only
    /// a `to` token between two literals is. The parser must keep it whole, or
    /// every consumer that used to re-split `cut` cuts it in half.
    /// 一条字面含有 `" to "` 的字符串路径不是区间：只有两个字面量之间的 `to` token
    /// 才是。解析器必须完整保留它，否则每个过去重新拆分 `cut` 的消费方都会把它拦腰
    /// 截断。
    #[test]
    fn a_path_containing_the_range_word_is_a_single_cut() {
        let source = r#"nichlink::static_graft_plan!(FRAMEWORK, cut "root/a to b" graft "g");"#;
        let entries = graft_entries(source).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].cut, "root/a to b");
        assert_eq!(entries[0].cut_end, None);
    }

    #[test]
    fn graft_parser_accepts_unbracketed_single_cut() {
        // A declaration belongs at the entry, so the fixture is an item; the
        // tests below cover plans that sit somewhere else.
        // 声明应当写在入口处，因此夹具写成条目；位置不当的计划由下面的测试覆盖。
        let source =
            r#"nichlink::static_graft_plan!(FRAMEWORK, cut "root/a" full graft "replacement");"#;
        let entries = graft_entries(source).unwrap();
        assert_eq!(entries[0].cut, "root/a");
        assert!(entries[0].full);
    }

    #[test]
    fn graft_parser_collects_declaration_only_static_plans() {
        let source = r#"nichlink::static_graft_plan!(FRAMEWORK,
    cut "root/a" graft "replacement",
);"#;
        let entries = graft_entries(source).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].cut, "root/a");
        assert_eq!(entries[0].graft, "replacement");
    }

    /// Only an item-position declaration belongs to the build's plan.
    /// 只有条目位置的声明才属于构建计划。
    #[test]
    fn graft_parser_ignores_declarations_that_are_not_items() {
        let in_function = r#"
fn plan() {
    let _ = nichlink::graft_plan!(FRAMEWORK, cut "root/a" graft "replacement");
}
"#;
        assert!(graft_entries(in_function).unwrap().is_empty());

        let in_test_module = r#"
#[cfg(test)]
mod tests {
    nichlink::static_graft_plan!(FRAMEWORK, cut "root/a" graft "replacement");
}
"#;
        assert!(graft_entries(in_test_module).unwrap().is_empty());

        let at_entry = r#"
#[cfg(feature = "optional-graft")]
nichlink::static_graft_plan!(FRAMEWORK, cut "root/a" graft "replacement");
"#;
        let entries = graft_entries(at_entry).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].cfg.as_deref(),
            Some("feature = \"optional-graft\"")
        );
    }

    #[test]
    fn graft_parser_ignores_removed_macro_names() {
        let source =
            r#"fn plan() { nichlink::graft!(framework, cut "root/a" graft "replacement"); }"#;
        assert!(graft_entries(source).unwrap().is_empty());
    }

    /// The typed form keeps both sides as Rust expressions so the compiler and
    /// any editor that resolves Rust paths can see the real target.
    /// 类型化形式把两侧都保留为 Rust 表达式，编译器和任何能解析 Rust 路径的编辑器
    /// 因此都能看到真实目标。
    #[test]
    fn graft_parser_keeps_typed_expressions() {
        let source = r#"nichlink::static_graft_plan!(FRAMEWORK,
            cut(crate::control::object::button::NODE_ID)
                graft(graft_crate::button_fast::NODE_ID),
        );"#;
        let entries = graft_entries(source).unwrap();
        let expressions = entries[0].expressions.as_ref().expect("typed expressions");
        assert_eq!(
            expressions.cut, "crate::control::object::button::NODE_ID",
            "the host path is preserved verbatim so the compiler resolves it"
        );
        assert_eq!(expressions.graft, "graft_crate::button_fast::NODE_ID");
        assert_eq!(expressions.cut_end, None);
        assert!(!entries[0].full);
    }

    #[test]
    fn graft_parser_keeps_a_typed_sibling_range() {
        let source = r#"nichlink::static_graft_plan!(FRAMEWORK,
            cut(crate::control::object::button::NODE_ID to crate::control::object::slider::NODE_ID)
                graft(graft_crate::fast::NODE_ID),
        );"#;
        let entries = graft_entries(source).unwrap();
        let expressions = entries[0].expressions.as_ref().expect("typed expressions");
        assert_eq!(expressions.cut, "crate::control::object::button::NODE_ID");
        assert_eq!(
            expressions.cut_end.as_deref(),
            Some("crate::control::object::slider::NODE_ID")
        );
        // The top-level fields carry the same two endpoints as data; the joined
        // `"start to end"` text is gone, so nothing downstream has to re-split.
        // 顶层字段以数据形式携带同样的两个端点；拼接的 `"start to end"` 文本已删除，
        // 下游无需再拆分。
        assert_eq!(entries[0].cut, "crate::control::object::button::NODE_ID");
        assert_eq!(
            entries[0].cut_end.as_deref(),
            Some("crate::control::object::slider::NODE_ID")
        );
    }

    /// A cut may not mix a resolved identity with an unresolved name.
    /// 一条切口不允许混合"已解析身份"与"未解析名称"。
    #[test]
    fn graft_parser_rejects_mixed_typed_and_string_sides() {
        let source = r#"nichlink::static_graft_plan!(FRAMEWORK,
            cut(crate::control::NODE_ID) graft "button_fast",
        );"#;
        let error = graft_entries(source).unwrap_err();
        assert!(
            error.message.contains("both sides"),
            "unexpected message: {}",
            error.message
        );
    }
}
