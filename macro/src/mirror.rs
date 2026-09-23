//! Editor-only field mirror for the face macros.
//! 注册面宏的编辑器专用字段镜像。
//!
//! An editor reads a macro's token tree only when it expands to a struct
//! literal, so the mirror has to be one — and because the editor compiles it
//! under `cfg(rust_analyzer)`, it also has to be valid Rust. This page owns the
//! mirror's shape, the translation of each field kind, and the small token
//! builders the rest of the front end reuses.
//! 编辑器只有看到展开成结构体字面量时才读得到宏的 token 树，因此镜像必须是字面量；
//! 又因为编辑器在 `cfg(rust_analyzer)` 下编译它，它还必须是合法 Rust。本页拥有镜像的
//! 形状、每种字段写法的翻译，以及前端其余部分复用的小型 token 构造器。

use proc_macro2::{
    Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream as Tokens, TokenTree,
};

use nichlink::lexicon;
use nichlink::registry_core::declaration::FACE_FIELD_ORDER;

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
        "source"
        | "params"
        | "stable_name"
        | "needs_registry"
        | "parent"
        | "registry_rule_path"
        | "registry_rule"
        | "admission"
        | "expected_output"
        | "actual_output"
        | "flow"
        | lexicon::FACE_FIELD_PLUGIN => MirrorField::Value,
        // `name`, `summary`, `exports`, `registry_name`,
        // `getting_from_other_registry`, `handle_traits`, `handle_contracts`,
        // `part_traits`, `part_contracts`, `requires`, `provides`,
        // `runtime_checks` — and any field added later, which defaults to the
        // safe shape rather than to a guess.
        // 其余字段——以及以后新增的字段——默认走安全的那一种，而不是靠猜。
        _ => MirrorField::Replace,
    }
}

/// One `key: value` pair the author wrote.
/// 作者写下的一条 `key: value`。
pub(crate) struct Field {
    pub(crate) key: String,
    pub(crate) span: Span,
    /// The span of the `:` that opens the value, which is where an editor's
    /// cursor sits while a value is being typed.
    /// 打开值的 `:` 的 span；作者输入值时，编辑器光标正停在那里。
    pub(crate) value_span: Span,
    pub(crate) tokens: Tokens,
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
pub(crate) fn mirror_item(fields: &[Field]) -> Tokens {
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
    let face_fields = format!("::{}::macros::FaceFields", lexicon::RUN_METHOD_CRATE);
    let mut body = Tokens::new();
    body.extend([ident("let"), ident("_"), punct(':', Spacing::Alone)]);
    body.extend(tokens_of(&face_fields));
    body.extend([punct('<', Spacing::Alone)]);
    body.extend(arguments);
    body.extend([punct('>', Spacing::Alone), punct('=', Spacing::Alone)]);
    body.extend(tokens_of(&face_fields));
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

/// One punctuation token with the given spacing.
/// 一个带指定 spacing 的标点 token。
pub(crate) fn punct(character: char, spacing: Spacing) -> TokenTree {
    TokenTree::Punct(Punct::new(character, spacing))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::front_end::split_mirror_fields;
    use crate::normalise;

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
            compact(&output).contains(&format!(
                "::{}::__nichlink_object!",
                lexicon::RUN_METHOD_CRATE
            )),
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
