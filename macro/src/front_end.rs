//! Field/argument parsing helpers for the face front end.
//! 注册面前端的字段 / 参数解析辅助函数。
//!
//! These helpers split a declaration's tokens and report a spanned error without
//! depending on the mirror's shape. The full front end reorders and validates
//! fields; the mirror variant tolerates a field the author has not finished.
//! 这些辅助函数切分声明的 token 并报告带 span 的错误，不依赖镜像的形状。完整前端负责
//! 重排与校验字段；镜像变体则容忍作者尚未写完的字段。

use proc_macro2::{
    Delimiter, Group, Ident, Literal, Spacing, Span, TokenStream as Tokens, TokenTree,
};

use nichlink::registry_core::declaration::FACE_FIELD_ORDER;

use crate::mirror::{Field, punct};

/// Split a token stream at its top-level `;`, keeping at most three parts.
/// 在顶层 `;` 处切分 token 流，最多保留三段。
///
/// A `;` inside a group (`{ … }`, `( … )`, `[ … ]`) belongs to that group, so it
/// is never a separator here; anything after the third separator stays in the
/// third part.
/// 组（`{ … }`、`( … )`、`[ … ]`）内的 `;` 属于该组，因此绝不算分隔符；第三个分隔符
/// 之后的内容留在第三段。
/// Replace every occurrence of one identifier with the given tokens.
/// 把某一标识符的每次出现替换为给定的 token。
///
/// Used to splice a caller's tokens into a template that must parse as a whole:
/// building the template by concatenating a prefix and a suffix would leave a
/// half-written item that no parser accepts. Groups are walked, so a placeholder
/// inside a block is found too, and the replacement is inserted verbatim — its
/// spans and any `$crate` hygiene survive.
/// 用于把调用方的 token 拼进一个必须整体可解析的模板：用前后缀拼接会留下半个语法项，任何
/// 解析器都不接受。遍历会进入分组，因此块内的占位符同样能找到，而替换是原样插入——它的
/// span 与任何 `$crate` 卫生性都得以保留。
pub(crate) fn splice(tokens: Tokens, placeholder: &str, replacement: &Tokens) -> Tokens {
    tokens
        .into_iter()
        .map(|token| match token {
            TokenTree::Ident(ident) if ident == placeholder => {
                TokenTree::Group(Group::new(Delimiter::None, replacement.clone()))
            }
            TokenTree::Group(group) => {
                let mut replaced = Group::new(
                    group.delimiter(),
                    splice(group.stream(), placeholder, replacement),
                );
                replaced.set_span(group.span());
                TokenTree::Group(replaced)
            }
            other => other,
        })
        .collect()
}

pub(crate) fn split_semicolons(tokens: Tokens) -> Vec<Tokens> {
    let mut parts = vec![Tokens::new()];
    for token in tokens {
        let is_separator = matches!(
            &token,
            TokenTree::Punct(punct) if punct.as_char() == ';'
        ) && parts.len() < 3;
        if is_separator {
            parts.push(Tokens::new());
        } else {
            parts
                .last_mut()
                .expect("a part is always open")
                .extend([token]);
        }
    }
    parts
}

/// Split a declaration for the mirror, tolerating unfinished fields.
/// 为镜像切分声明，容忍尚未写完的字段。
pub(crate) fn split_mirror_fields(input: Tokens) -> Result<Vec<Field>, Tokens> {
    let mut fields: Vec<Field> = Vec::new();
    for tokens in super::split_face_fields(input) {
        let tokens = tokens.into_iter().collect::<Vec<_>>();
        let Some(TokenTree::Ident(name)) = tokens.first().cloned() else {
            return Err(error_at(
                tokens.first().map_or_else(Span::call_site, TokenTree::span),
                "expected a face field name".to_owned(),
            ));
        };
        let key = name.to_string();
        if !FACE_FIELD_ORDER.contains(&key.as_str()) {
            // `collector` and unknown names are not `FaceFields` members; the
            // runtime chain reports an unknown name, and `collector` is not the
            // author's to write here.
            // `collector` 与未知名字都不是 `FaceFields` 的成员：未知名字由运行期链报错，
            // 而 `collector` 不该由作者写在这里。
            continue;
        }
        if fields.iter().any(|field| field.key == key) {
            continue;
        }
        fields.push(Field {
            key,
            span: name.span(),
            value_span: tokens.get(1).map_or_else(|| name.span(), TokenTree::span),
            tokens: tokens.into_iter().collect(),
        });
    }
    Ok(fields)
}

/// Render a token slice with single spaces between tokens.
/// 用单个空格连接 token 切片。
pub(crate) fn render(tokens: &[TokenTree]) -> String {
    tokens
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Build a `compile_error! { … }` token stream anchored at `span`.
/// 构造锚定在 `span` 的 `compile_error! { … }` token 流。
pub(crate) fn error_at(span: Span, message: String) -> Tokens {
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
