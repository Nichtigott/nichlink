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

/// The `registry_rule` a face gets when it does not write one.
/// 注册面没有写 `registry_rule` 时得到什么。
///
/// The input is `<fallback> ; <needs_registry> ; <author expression?>`. The
/// author's expression always wins. A face that owns a registry
/// (`needs_registry: true`) takes the **canonical sibling rule** — the rule
/// module the authoring layout keeps beside the face folder — and every other
/// face keeps the permissive fallback the declarative layer passes in.
/// 输入是 `<fallback> ; <needs_registry> ; <作者表达式?>`。作者写下的表达式永远最优先。
/// 拥有注册机的面（`needs_registry: true`）取**同目录规范规则**——创作布局放在注册面
/// 目录旁的那个规则模块——其余注册面保留声明层传进来的宽松默认值。
///
/// The path is relative (`super::registry_rule`) on purpose, and that is why this
/// has to be a proc macro: a folder face file is loaded as the inner module of
/// its folder, so the rule module is its *sibling*, and `module_path!()` is a
/// string that cannot become a path. Tokens built here carry `Span::call_site()`,
/// so the relative path resolves in the author's own module.
/// 路径是相对的（`super::registry_rule`）而不是绝对的，这也正是它必须是过程宏的原因：
/// 文件夹注册面文件被载入为其文件夹的内层模块，因此规则模块是它的**兄弟**；而
/// `module_path!()` 是字符串，无法变成路径。这里生成的 token 带 `Span::call_site()`，
/// 因此相对路径在作者自己的模块里解析。
#[proc_macro]
pub fn face_rule_or(input: TokenStream) -> TokenStream {
    let mut parts = split_semicolons(Tokens::from(input)).into_iter();
    let fallback = parts.next().unwrap_or_default();
    let needs_registry = parts.next().unwrap_or_default();
    let authored = parts.next().unwrap_or_default();
    if !authored.is_empty() {
        return authored.into();
    }
    let owns_registry = needs_registry
        .into_iter()
        .map(|token| token.to_string())
        .collect::<String>();
    if owns_registry == "true" {
        return "super :: registry_rule :: REGISTRATION_RULE"
            .parse::<Tokens>()
            .expect("the canonical rule path is a static path")
            .into();
    }
    fallback.into()
}

/// Split a token stream at its top-level `;`, keeping at most three parts.
/// 在顶层 `;` 处切分 token 流，最多保留三段。
///
/// A `;` inside a group (`{ … }`, `( … )`, `[ … ]`) belongs to that group, so it
/// is never a separator here; anything after the third separator stays in the
/// third part.
/// 组（`{ … }`、`( … )`、`[ … ]`）内的 `;` 属于该组，因此绝不算分隔符；第三个分隔符
/// 之后的内容留在第三段。
fn split_semicolons(tokens: Tokens) -> Vec<Tokens> {
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

/// Which macro the normalised declaration goes back to.
/// 归一化后的声明要回到哪个宏。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    /// The generated host aliases' path.
    /// 生成的宿主别名那条路。
    Control,
    /// `external_object!`, whose matcher names the source file first.
    /// `external_object!`——它的 matcher 首要指出源文件。
    External,
}

fn normalise(input: Tokens) -> Result<Tokens, Tokens> {
    let mut list = input.into_iter().collect::<Vec<_>>();
    let mut target = Target::Control;
    if let (Some(TokenTree::Punct(at)), Some(TokenTree::Ident(name))) = (list.first(), list.get(1))
        && at.as_char() == '@'
    {
        target = match name.to_string().as_str() {
            "control" => Target::Control,
            "external" => Target::External,
            other => {
                return Err(error_at(
                    name.span(),
                    format!("unknown face field target `{other}`"),
                ));
            }
        };
        list.drain(0..2);
    }
    let input = list.into_iter().collect::<Tokens>();
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
        if name == "source" && target == Target::Control {
            return Err(error_at(
                name.span(),
                "`source` belongs to `external_object!`; a generated host alias                  records the file itself"
                    .to_owned(),
            ));
        }
        if name == "collector" {
            if collector.is_some() {
                return Err(error_at(
                    name.span(),
                    "face field `collector` is given twice".to_owned(),
                ));
            }
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
    if target == Target::Control && body.to_string() == original {
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
    // The alias's own editor splice carries the author's tokens verbatim, which is
    // right for a canonical declaration and impossible for one written with `;`
    // or in another order: `FaceFields { kind: Tool; parent: … }` is a syntax
    // error, so the editor loses every field candidate exactly where the front end
    // exists to help. Re-emitting the canonical list here keeps the aid for those
    // declarations too — the keys and values still carry the author's spans, so
    // the editor maps the cursor into this list.
    // 别名自带的编辑器拼接原样携带作者的 token：对规范声明是对的，对用 `;` 或乱序写成的
    // 声明则不可能——`FaceFields { kind: Tool; parent: … }` 是语法错误，于是编辑器恰恰
    // 在前端本该帮忙的地方失去全部字段候选。这里再发一份规范化列表，让这类声明也保住
    // 该提示：键与值仍带作者的 span，编辑器因此能把光标映射进这份列表。
    let mut normalized = Tokens::new();
    for field in &fields {
        normalized.extend(field.tokens.clone());
        normalized.extend([punct(',', Spacing::Alone)]);
    }
    output.extend([
        TokenTree::Punct(Punct::new('#', Spacing::Alone)),
        TokenTree::Group(Group::new(
            Delimiter::Bracket,
            Tokens::from_iter([
                TokenTree::Ident(Ident::new("cfg", Span::call_site())),
                TokenTree::Group(Group::new(
                    Delimiter::Parenthesis,
                    Tokens::from_iter([TokenTree::Ident(Ident::new(
                        "rust_analyzer",
                        Span::call_site(),
                    ))]),
                )),
            ]),
        )),
        punct(':', Spacing::Joint),
        punct(':', Spacing::Alone),
        TokenTree::Ident(Ident::new("nichlink_run_method", Span::call_site())),
        punct(':', Spacing::Joint),
        punct(':', Spacing::Alone),
        TokenTree::Ident(Ident::new("__face_fields", Span::call_site())),
        punct('!', Spacing::Alone),
        TokenTree::Group(Group::new(Delimiter::Brace, normalized)),
    ]);
    output.extend([
        punct(':', Spacing::Joint),
        punct(':', Spacing::Alone),
        TokenTree::Ident(Ident::new("nichlink_run_method", Span::call_site())),
        punct(':', Spacing::Joint),
        punct(':', Spacing::Alone),
        TokenTree::Ident(Ident::new(
            match target {
                Target::Control => "__nichlink_object",
                Target::External => "__external_object",
            },
            Span::call_site(),
        )),
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
