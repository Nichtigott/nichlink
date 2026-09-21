//! Face field normalisation for the authoring macros.
//! 为作者侧宏归一化注册面字段。
//!
//! A `macro_rules!` matcher can only fail with "no rules expected `...`", and it
//! cannot reorder fields, cannot compare field names, and cannot attach a span
//! to a message of its own. This front end receives the author's tokens with
//! their spans, so it accepts `,` or `;` separators, a forgotten separator and
//! any field order, and still reports an unknown or repeated field on the exact
//! token that carries it.
//! `macro_rules!` 匹配失败只会说 "no rules expected `...`"，既不能重排字段、不能
//! 比较字段名，也无法把自己的消息挂到具体 token 上。本前端拿到的是带 span 的作者
//! token，因此接受 `,`/`;`、漏写分隔符与任意顺序，并把"未知字段/重复字段"精确报在
//! 出问题的那个 token 上。
//!
//! Splitting is shared with the build-time reader (`split_face_fields`): the
//! compiler and the build step must read one declaration the same way, so they
//! use the same tolerant splitter.
//! 切分逻辑与构建期读取器共用（`split_face_fields`）：编译器与构建步骤必须对同一份
//! 声明读出一致的字段，因此两者使用同一个宽容切分器。
//!
//! The normalised declaration goes back through `__nichlink_object!`, the
//! exported entry point of the runtime crate, so the collector mode the caller
//! chose survives the round trip. This front end is only reached when every
//! strict arm of `__control_object!` has already declined the declaration, so a
//! well-formed face expands exactly as it did before.
//! 归一化后的声明经 `__nichlink_object!`（运行时 crate 的公开入口）回到宏阶梯，
//! 调用方选择的 collector 模式因此得以保留。只有当 `__control_object!` 的全部严格
//! arm 都不接受时才会走到本前端，因此合法注册面的展开与从前完全一致。

use proc_macro::TokenStream;
use proc_macro2::{
    Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream as Tokens, TokenTree,
};

use nichlink::registry_core::declaration::FACE_FIELD_ORDER;
use nichlink::registry_core::syntax::split_face_fields;

#[proc_macro]
pub fn face_fields(input: TokenStream) -> TokenStream {
    normalise(Tokens::from(input))
        .unwrap_or_else(|error| error)
        .into()
}

fn normalise(input: Tokens) -> Result<Tokens, Tokens> {
    let original = input.to_string();
    let mut collector: Option<TokenTree> = None;
    let mut fields: Vec<Field> = Vec::new();
    for field in split_face_fields(input) {
        let tokens = field.into_iter().collect::<Vec<_>>();
        let Some(TokenTree::Ident(name)) = tokens.first().cloned() else {
            let span = tokens.first().map_or_else(Span::call_site, TokenTree::span);
            return Err(error_at(
                span,
                format!("expected a face field name, found `{}`", render(&tokens)),
            ));
        };
        match tokens.get(1) {
            Some(TokenTree::Punct(colon)) if colon.as_char() == ':' => {}
            _ => {
                return Err(error_at(
                    name.span(),
                    format!("expected `:` after face field `{name}`"),
                ));
            }
        }
        if tokens.len() == 2 {
            return Err(error_at(
                name.span(),
                format!("face field `{name}` has no value"),
            ));
        }
        if name == "collector" {
            collector = Some(tokens[2].clone());
            continue;
        }
        fields.push(Field {
            key: name.to_string(),
            span: name.span(),
            tokens: tokens.into_iter().collect(),
        });
    }

    let position = FACE_FIELD_ORDER
        .iter()
        .enumerate()
        .map(|(index, key)| (*key, index))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut seen: Vec<&str> = Vec::with_capacity(fields.len());
    for field in &fields {
        if !position.contains_key(field.key.as_str()) {
            return Err(error_at(
                field.span,
                format!(
                    "unknown face field `{}`; expected one of: {}",
                    field.key,
                    FACE_FIELD_ORDER.join(", ")
                ),
            ));
        }
        if seen.contains(&field.key.as_str()) {
            return Err(error_at(
                field.span,
                format!("face field `{}` is given twice", field.key),
            ));
        }
        seen.push(&field.key);
    }
    let Some(collector) = collector else {
        return Err(error_at(
            Span::call_site(),
            "a face declaration must carry `collector:`".to_owned(),
        ));
    };
    fields.sort_by_key(|field| position[field.key.as_str()]);

    let mut body = Tokens::new();
    body.extend([
        TokenTree::Ident(Ident::new("collector", Span::call_site())),
        punct(':', Spacing::Alone),
        collector,
        punct(',', Spacing::Alone),
    ]);
    for field in &fields {
        body.extend(field.tokens.clone());
        body.extend([punct(',', Spacing::Alone)]);
    }
    if body.to_string() == original {
        // Reordering and re-emitting produced this very token stream, so another
        // round would only recurse: the declaration already is in the accepted
        // order and still did not match, which means a field's shape is wrong.
        // 重排与重发得到的正是这串 token，再走一轮只会递归：声明已在接受顺序上却仍未
        // 匹配，说明某个字段的写法不对。
        return Err(error_at(
            Span::call_site(),
            "these face fields are in the accepted order but the declaration still did not match; \
             check each field's shape against the field list in the macro's documentation"
                .to_owned(),
        ));
    }

    let mut output = Tokens::new();
    output.extend([
        punct(':', Spacing::Joint),
        punct(':', Spacing::Alone),
        TokenTree::Ident(Ident::new("nichlink_run_method", Span::call_site())),
        punct(':', Spacing::Joint),
        punct(':', Spacing::Alone),
        TokenTree::Ident(Ident::new("__nichlink_object", Span::call_site())),
        punct('!', Spacing::Alone),
        TokenTree::Group(Group::new(Delimiter::Brace, body)),
    ]);
    Ok(output)
}

/// One `key: value` pair the author wrote.
/// 作者写下的一条 `key: value`。
struct Field {
    key: String,
    span: Span,
    tokens: Tokens,
}

fn punct(character: char, spacing: Spacing) -> TokenTree {
    TokenTree::Punct(Punct::new(character, spacing))
}

fn render(tokens: &[TokenTree]) -> String {
    tokens
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

fn error_at(span: Span, message: String) -> Tokens {
    let mut literal = Literal::string(&message);
    literal.set_span(span);
    Tokens::from_iter([
        TokenTree::Ident(Ident::new("compile_error", span)),
        punct('!', Spacing::Alone),
        // Item position: the message must be brace-delimited, like any macro
        // invocation that expands to items.
        // item 位置：与其他展开成条目的宏调用一样，消息必须用花括号定界。
        TokenTree::Group(Group::new(
            Delimiter::Brace,
            Tokens::from(TokenTree::Literal(literal)),
        )),
    ])
}
