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
            value_span: tokens.get(1).map_or_else(|| name.span(), TokenTree::span),
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

    let mut output = mirror_item(&fields);
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

/// How the mirror carries one face field.
/// 镜像如何承载一个注册面字段。
///
/// The authoring grammar mixes three kinds of spelling, and the mirror — which
/// is real Rust compiled under `cfg(rust_analyzer)` — has to translate each one
/// without inventing a type error:
/// 作者侧语法混有三种写法，而镜像是在 `cfg(rust_analyzer)` 下编译的真实 Rust，
/// 必须逐类翻译，且不能凭空造出类型错误：
///
/// * `Value` fields are written as expressions (`parent: crate::control::NODE_ID`,
///   `flow: FlowContract::new(…)`), so the mirror echoes them and lets `_` infer
///   the type. Path completion inside them, and the type candidates at an empty
///   value position, both keep working.
/// * `Type` fields (`kind`, `preset`, `parts`, `handle`, `flow_provider`) name a
///   type, and a type is not a value: the mirror moves the author's tokens into
///   the annotation and writes `loop {}` as the value, which coerces to whatever
///   the type is. A unit marker type still completes at `kind: `, and a non-unit
///   preset type no longer produces `expected (), found …`.
/// * `Replace` fields are braced localized text (`name`, `summary`), arrow lists
///   (`requires`), trait-path lists (`handle_contracts`), a bare slot identifier
///   (`registry_name`), `None` (whose `Option<T>` cannot be inferred), or an
///   aggregate list that may be empty (whose element type cannot be inferred).
///   The mirror keeps their names and replaces their values.
///
/// * `Value` 字段写成表达式（`parent: crate::control::NODE_ID`、
///   `flow: FlowContract::new(…)`），镜像原样回写、由 `_` 推导类型：值内部的路径补全与
///   空值位的类型候选都不受影响。
/// * `Type` 字段（`kind`、`preset`、`parts`、`handle`、`flow_provider`）命名的是类型，
///   而类型不是值：镜像把作者的 token 放进类型注解，值写 `loop {}`（可强制转换成该类型）。
///   单元标记类型在 `kind: ` 处照旧有候选，而非单元的 preset 类型也不会再报
///   `expected (), found …`。
/// * `Replace` 字段是花括号本地化文本（`name`、`summary`）、箭头列表（`requires`）、
///   trait 路径列表（`handle_contracts`）、裸槽位标识符（`registry_name`）、`None`
///   （`Option<T>` 推导不出来），或可能为空的聚合列表（元素类型推导不出来）。镜像保留
///   它们的字段名，替换它们的值。
#[derive(Clone, Copy, PartialEq, Eq)]
enum MirrorField {
    Value,
    Type,
    Replace,
}

/// Which of the three shapes one face field has.
/// 一个注册面字段属于三种写法中的哪一种。
fn mirror_field(key: &str) -> MirrorField {
    match key {
        "kind" | "preset" | "parts" | "handle" | "flow_provider" => MirrorField::Type,
        "source" | "params" | "stable_name" | "needs_registry" | "parent"
        | "registry_rule_path" | "registry_rule" | "admission" | "expected_output"
        | "actual_output" | "flow" | "plugin" => MirrorField::Value,
        // `name`, `summary`, `exports`, `registry_name`,
        // `getting_from_other_registry`, `handle_traits`, `handle_contracts`,
        // `part_traits`, `part_contracts`, `requires`, `provides`,
        // `runtime_checks` — and any field added later, which defaults to the
        // safe shape rather than to a guess.
        // 其余字段——以及以后新增的字段——默认走安全的那一种，而不是靠猜。
        _ => MirrorField::Replace,
    }
}

/// Build the editor-only field mirror as a real, type-correct item.
/// 把仅供编辑器的字段镜像构造成一个真实且类型正确的条目。
///
/// An editor reads a macro's token tree only when it expands to a struct
/// literal, so the mirror has to be one — and because the editor compiles it
/// under `cfg(rust_analyzer)`, it also has to be valid Rust. Two rules make
/// that true without losing either completion:
/// 编辑器只有看到展开成结构体字面量时才读得到宏的 token 树，因此镜像必须是字面量；又因为
/// 编辑器在 `cfg(rust_analyzer)` 下编译它，它还必须是合法 Rust。两条规则同时满足这两点，
/// 且两个补全都不丢：
///
/// * **written fields** keep the author's tokens and take type argument `_`, so
///   the value infers its own type and path completion inside it still works;
/// * **unwritten or non-expression fields** take the rigid `__Any` parameter and
///   the value `loop {}`, which coerces to it. A rigid parameter is never an
///   inference variable, so the literal is fully typed: no `E0282`, no
///   `expected (), found …`. It still answers the value position, so picking a
///   marker type at `kind: ` completes exactly as before.
/// * **作者写下的字段**保留其 token 并取类型参数 `_`，值因此自行推导类型，值内部的路径
///   补全照旧；
/// * **未写或非表达式的字段**取刚性参数 `__Any`、值为可强制转换成它的 `loop {}`。刚性参数
///   不是推导变量，字面量因此完全定型：既没有 `E0282`，也没有
///   `expected (), found …`。它仍然回答值位，所以在 `kind: ` 处挑选标记类型的补全保持不变。
///
/// The function is never called and never compiled into a host: `rustc` only
/// expands this item when `--cfg rust_analyzer` is set, and the body inside an
/// anonymous `const` keeps two faces in one file from colliding on the name.
/// 这个函数永不执行、也永不编进宿主：只有设置 `--cfg rust_analyzer` 时 `rustc` 才会展开
/// 这个条目；函数体放在匿名 `const` 里，因此同一文件里的两个注册面不会重名。
fn mirror_item(fields: &[Field]) -> Tokens {
    let mut arguments = Tokens::new();
    let mut assignments = Tokens::new();
    for (index, key) in FACE_FIELD_ORDER.iter().enumerate() {
        if index > 0 {
            arguments.extend([punct(',', Spacing::Alone)]);
        }
        let field = fields.iter().find(|field| field.key == *key);
        let written = field.filter(|field| field.tokens.clone().into_iter().count() > 2);
        match (mirror_field(key), written) {
            // The author's expression, typed by inference.
            (MirrorField::Value, Some(field)) => {
                arguments.extend([ident("_")]);
                assignments.extend(field.tokens.clone());
            }
            // The author's type, with a value that coerces to it.
            (MirrorField::Type, Some(field)) => {
                arguments.extend(field.tokens.clone().into_iter().skip(2));
                assignments.extend(loop_value(field));
            }
            // A name the editor needs and a shape the runtime chain checks.
            (_, field) => {
                arguments.extend([ident("__Any")]);
                if let Some(field) = field {
                    assignments.extend(loop_value(field));
                }
            }
        }
        if field.is_some() {
            assignments.extend([punct(',', Spacing::Alone)]);
        }
    }

    // `FaceFields { <the author's fields> ..loop {} }`
    let mut literal = assignments;
    literal.extend([
        punct('.', Spacing::Joint),
        punct('.', Spacing::Alone),
        ident("loop"),
        group(Delimiter::Brace, Tokens::new()),
    ]);
    let literal = group(Delimiter::Brace, literal);

    // `let _: FaceFields<<arguments>> = FaceFields <literal>;`
    let mut body = Tokens::new();
    body.extend([ident("let"), ident("_"), punct(':', Spacing::Alone)]);
    body.extend(tokens_of("::nichlink_run_method::macros::FaceFields"));
    body.extend([punct('<', Spacing::Alone)]);
    body.extend(arguments);
    body.extend([punct('>', Spacing::Alone), punct('=', Spacing::Alone)]);
    body.extend(tokens_of("::nichlink_run_method::macros::FaceFields"));
    body.extend([literal, punct(';', Spacing::Alone)]);
    let body = group(Delimiter::Brace, body);

    // `#[allow(…)] fn __face_fields<__Any>() <body>`
    let mut function = tokens_of("#[allow(unreachable_code, dead_code)]");
    function.extend([
        ident("fn"),
        ident("__face_fields"),
        punct('<', Spacing::Alone),
    ]);
    function.extend([ident("__Any"), punct('>', Spacing::Alone)]);
    function.extend([group(Delimiter::Parenthesis, Tokens::new()), body]);
    let function = group(Delimiter::Brace, function);

    // `#[cfg(rust_analyzer)] const _: () = <function>;`
    //
    // `_` is a token proc-macro2's string parser refuses, so it is inserted as
    // an identifier; every brace-delimited piece is built as a group because a
    // partially parsed token stream with an open brace is a lex error.
    // `_` 是 proc-macro2 的字符串解析器拒绝的 token，因此以标识符插入；每一段花括号都
    // 构造成 group，因为带未闭合花括号的片段解析会报词法错误。
    let mut item = tokens_of("#[cfg(rust_analyzer)]");
    item.extend([ident("const"), ident("_"), punct(':', Spacing::Alone)]);
    item.extend([group(Delimiter::Parenthesis, Tokens::new())]);
    item.extend([
        punct('=', Spacing::Alone),
        function,
        punct(';', Spacing::Alone),
    ]);
    item
}

/// `key: loop {}` for one field, keeping the author's span on the name.
/// 一个字段的 `key: loop {}`，字段名保留作者的 span。
fn loop_value(field: &Field) -> Tokens {
    let mut tokens = field.tokens.clone().into_iter().take(1).collect::<Tokens>();
    tokens.extend([punct(':', Spacing::Alone)]);
    // The placeholder is anchored at the author's own value position, so an
    // editor still maps the cursor into the mirror and answers the value slot
    // with type candidates; a call-site span would leave it with nothing.
    // 占位符锚定在作者自己的值位，编辑器因此仍能把光标映射进镜像、在值位给出类型候选；
    // 用 call-site span 会让它什么都补不出来。
    tokens.extend([
        TokenTree::Ident(Ident::new("loop", field.value_span)),
        group(Delimiter::Brace, Tokens::new()),
    ]);
    tokens
}

/// One identifier token without a source span of its own.
/// 一个不带自身源码位置的标识符 token。
fn ident(name: &str) -> TokenTree {
    TokenTree::Ident(Ident::new(name, Span::call_site()))
}

/// One delimited group.
/// 一个带定界符的分组。
fn group(delimiter: Delimiter, tokens: Tokens) -> TokenTree {
    TokenTree::Group(Group::new(delimiter, tokens))
}

/// Parse a fixed token string. Every caller passes tokens this file controls.
/// 解析一段固定 token 字符串；所有调用方传入的都是本文件自己控制的 token。
fn tokens_of(source: &str) -> Tokens {
    source
        .parse::<Tokens>()
        .unwrap_or_else(|error| panic!("the mirror's own tokens are valid: {error} in {source:?}"))
}

/// The editor-only field mirror for a declaration the caller already split.
/// 为调用方已切分好的声明生成仅供编辑器的字段镜像。
///
/// The generated alias reaches this through `face_fields_mirror!`, because its
/// token tree is opaque to an editor and the alias may be written with `;` or in
/// any order. Unlike the full front end it tolerates a field the author has not
/// finished (`kind:` with no value) and never re-dispatches to the runtime.
/// 生成的别名通过 `face_fields_mirror!` 走到这里：它的 token 树对编辑器不透明，而且别名
/// 可能用 `;` 或任意顺序书写。与完整前端不同，它容忍作者还没写完的字段（`kind:` 后面
/// 还没有值），也绝不回派到运行期。
#[proc_macro]
pub fn face_fields_mirror(input: TokenStream) -> TokenStream {
    match split_mirror_fields(Tokens::from(input)) {
        Ok(fields) => mirror_item(&fields).into(),
        // A declaration this cannot split is one the runtime ladder will report
        // on its own; the mirror stays silent rather than adding a second error.
        // 切分不了的声明由运行期阶梯自己报错；镜像保持沉默，不再添一个错误。
        Err(_) => Tokens::new().into(),
    }
}

/// Split a declaration for the mirror, tolerating unfinished fields.
/// 为镜像切分声明，容忍尚未写完的字段。
fn split_mirror_fields(input: Tokens) -> Result<Vec<Field>, Tokens> {
    let mut fields: Vec<Field> = Vec::new();
    for tokens in split_face_fields(input) {
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

/// One `key: value` pair the author wrote.
/// 作者写下的一条 `key: value`。
struct Field {
    key: String,
    span: Span,
    /// The span of the `:` that opens the value, which is where an editor's
    /// cursor sits while a value is being typed.
    /// 打开值的 `:` 的 span；作者输入值时，编辑器光标正停在那里。
    value_span: Span,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Whitespace-free rendering: a token stream prints with spaces around
    /// punctuation, which says nothing about the tokens themselves.
    /// 去掉空白的渲染：token 流会在标点两侧打印空格，那与 token 本身无关。
    fn compact(source: &str) -> String {
        source
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect()
    }

    /// The mirror a generated alias asks for, with no runtime dispatch.
    /// 生成的别名索取的那份镜像，不含运行期回派。
    fn mirror(source: &str) -> String {
        mirror_item(&split_mirror_fields(source.parse().expect("tokens")).expect("splits"))
            .to_string()
    }

    /// The mirror the full front end emits, for a declaration only it accepts.
    /// 完整前端发出的镜像，用于只有它接受的声明。
    fn front_end(source: &str) -> String {
        normalise(source.parse().expect("tokens"))
            .expect("the declaration normalises")
            .to_string()
    }

    /// The 29 type arguments of the mirror's literal.
    /// 镜像字面量的 29 个类型实参。
    fn arguments(output: &str) -> Vec<&str> {
        let start = output.find("FaceFields<").expect("a FaceFields literal") + "FaceFields<".len();
        let rest = &output[start..];
        let end = rest.find('>').expect("a closed argument list");
        rest[..end].split(',').collect()
    }

    /// Every mirror is something an editor will compile, so it has to parse.
    /// 每个镜像都会被编辑器编译，因此必须能解析。
    #[test]
    fn every_mirror_is_valid_rust() {
        for source in [
            "kind: Tool,",
            "kind: Tool; parent: crate::a::NODE_ID;",
            "source: \"a/a.rs\", kind: K, name: { zh: \"控件\", en: \"Widget\" }, \
             requires: [\"cap\" => \"Provider\"],",
        ] {
            let output = mirror(source);
            syn::parse_file(&output).unwrap_or_else(|error| {
                panic!("the mirror for `{source}` parses: {error}\n{output}")
            });
        }
        // The front end emits the same item, followed by the runtime call, and a
        // declaration the compact arm refuses is the only one that reaches it.
        // 前端发出同一个条目，后面跟运行期调用；只有紧凑 arm 拒绝的声明才会走到这里。
        let output =
            front_end("@control collector: development, kind: Tool; parent: crate::a::NODE_ID;");
        syn::parse_file(&output).unwrap_or_else(|error| {
            panic!("the front end's output parses: {error}\nRAW: {output}")
        });
        assert!(
            compact(&output).contains("::nichlink_run_method::__nichlink_object!"),
            "{output}"
        );
    }

    /// A written expression is kept and typed by inference; a field that names
    /// a type is annotated with it; everything else is replaced.
    /// 写下的表达式原样保留、由推导定型；命名类型的字段用该类型注解；其余一律替换。
    #[test]
    fn the_mirror_translates_each_field_shape() {
        let output = compact(&mirror(
            "kind: Tool, preset: NoPreset, name: { zh: \"控件\", en: \"Widget\" }, \
             parent: crate::a::NODE_ID, exports: [], provides: [\"x\"], registry_name: tool,",
        ));
        // kind/preset name types: the author's type goes into the annotation and
        // the value coerces to it.
        assert!(output.contains("kind:loop{}"), "{output}");
        assert!(output.contains("preset:loop{}"), "{output}");
        // An expression keeps its own tokens so paths inside it complete.
        assert!(output.contains("parent:crate::a::NODE_ID"), "{output}");
        // Braced text, a bare slot ident, and aggregate lists are replaced.
        for replaced in [
            "name:loop{}",
            "registry_name:loop{}",
            "exports:loop{}",
            "provides:loop{}",
        ] {
            assert!(output.contains(replaced), "{replaced} missing in {output}");
        }
        assert!(output.contains("..loop{}"), "{output}");
        assert!(output.contains("fn__face_fields<__Any>()"), "{output}");

        let arguments = arguments(&output);
        assert_eq!(arguments.len(), FACE_FIELD_ORDER.len(), "{arguments:?}");
        assert_eq!(arguments[0], "__Any", "source is absent: {arguments:?}");
        assert_eq!(arguments[1], "Tool", "kind names a type: {arguments:?}");
        assert_eq!(
            arguments[2], "NoPreset",
            "preset names a type: {arguments:?}"
        );
        assert_eq!(arguments[4], "__Any", "name is replaced: {arguments:?}");
        assert_eq!(arguments[7], "__Any", "exports is replaced: {arguments:?}");
        assert_eq!(
            arguments[11], "__Any",
            "registry_name is replaced: {arguments:?}"
        );
        assert_eq!(
            arguments[12], "_",
            "parent is inferred from its value: {arguments:?}"
        );
    }

    /// A declaration with no fields still names every field for the editor.
    /// 没有任何字段的声明仍然为编辑器列出全部字段名。
    #[test]
    fn an_empty_declaration_pins_every_parameter() {
        let output = mirror("");
        syn::parse_file(&output)
            .unwrap_or_else(|error| panic!("the empty mirror parses: {error}\nRAW: {output}"));
        let output = compact(&output);
        assert!(output.contains("fn__face_fields<__Any>()"), "{output}");
        assert!(
            arguments(&output).iter().all(|item| *item == "__Any"),
            "{output}"
        );
    }
}
