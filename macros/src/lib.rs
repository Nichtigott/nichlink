//! Proc-macro attribute for NichLink registration faces.
//! NichLink 注册面的过程宏属性。
//!
//! `#[nichlink::object]` turns a plain struct into a registration face: the
//! struct stays a real, navigable type; `preset`/`parts` fields carry the
//! construction contract; every other registration metadata field is an
//! optional attribute argument whose default matches the old compact DSL.
//! `#[nichlink::object]` 把普通结构体变成注册面：结构体仍是可导航的真实类型；
//! `preset`/`parts` 字段承载构造合同；其余注册元数据全部是可选属性参数，
//! 缺省值与旧精简 DSL 逐一相同。

use proc_macro2::{Delimiter, Span, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::{Ident, ItemStruct, LitBool, LitStr, Type, parse2};

#[proc_macro_attribute]
pub fn object(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    match expand_object(TokenStream2::from(attr), TokenStream2::from(item)) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

const VALUE_KEYS: &[&str] = &[
    "parent",
    "handle",
    "params",
    "needs_registry",
    "registry_name",
    "getting_from_other_registry",
    "registry_rule_path",
    "registry_rule",
    "admission",
    "stable_name",
    "source",
    "collector",
    "expected_output",
    "actual_output",
    "flow",
    "flow_provider",
    "plugin",
    "exports",
    "handle_traits",
    "part_traits",
    "handle_contracts",
    "part_contracts",
    "runtime_checks",
];
const GROUP_KEYS: &[&str] = &["name", "summary", "requires", "provides"];

#[derive(Default)]
struct Args {
    values: Vec<(String, TokenStream2)>,
    name: Option<(LitStr, LitStr)>,
    summary: Option<(LitStr, LitStr)>,
    requires: Vec<(LitStr, LitStr)>,
    provides: Vec<LitStr>,
}

impl Args {
    fn take_value(&mut self, key: &str) -> Option<TokenStream2> {
        let index = self.values.iter().position(|(name, _)| name == key)?;
        Some(self.values.remove(index).1)
    }

    fn literal(&mut self, key: &str) -> syn::Result<Option<LitStr>> {
        match self.take_value(key) {
            None => Ok(None),
            Some(tokens) => parse2::<LitStr>(tokens).map(Some).map_err(|error| {
                syn::Error::new(error.span(), format!("{key} expects a string literal"))
            }),
        }
    }

    fn boolean(&mut self, key: &str) -> syn::Result<Option<LitBool>> {
        match self.take_value(key) {
            None => Ok(None),
            Some(tokens) => parse2::<LitBool>(tokens).map(Some).map_err(|error| {
                syn::Error::new(error.span(), format!("{key} expects true or false"))
            }),
        }
    }

    fn path(&mut self, key: &str) -> syn::Result<Option<Type>> {
        match self.take_value(key) {
            None => Ok(None),
            Some(tokens) => parse2::<Type>(tokens).map(Some).map_err(|error| {
                syn::Error::new(error.span(), format!("{key} expects a type path"))
            }),
        }
    }

    fn list(&mut self, key: &str) -> syn::Result<Vec<TokenStream2>> {
        match self.take_value(key) {
            None => Ok(Vec::new()),
            Some(tokens) => {
                let group = single_group(&tokens, Delimiter::Bracket).ok_or_else(|| {
                    syn::Error::new_spanned(&tokens, format!("{key} expects a [ ... ] list"))
                })?;
                Ok(split_top_level(group.stream()))
            }
        }
    }

    fn leftover(&self) -> Option<syn::Error> {
        self.values.first().map(|(name, tokens)| {
            let mut expected = VALUE_KEYS.join(", ");
            expected.push_str(", ");
            expected.push_str(
                &GROUP_KEYS
                    .iter()
                    .map(|key| format!("{key}(...)"))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            syn::Error::new_spanned(
                tokens,
                format!("unknown `object` argument `{name}`; expected one of: {expected}"),
            )
        })
    }
}

fn expand_object(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let item_struct: ItemStruct = parse2(item)?;
    if !item_struct.generics.params.is_empty() || item_struct.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &item_struct.generics,
            "`#[nichlink::object]` does not support generics on the face struct",
        ));
    }

    let mut preset: Option<Type> = None;
    let mut parts: Option<Type> = None;
    match &item_struct.fields {
        syn::Fields::Named(fields) => {
            for field in &fields.named {
                let name = field.ident.as_ref().expect("named field").to_string();
                match name.as_str() {
                    "preset" => preset = Some(field.ty.clone()),
                    "parts" => parts = Some(field.ty.clone()),
                    "exports" => {
                        return Err(syn::Error::new_spanned(
                            field,
                            "`exports` is not a struct field; write `impl Kind { pub const EXPORTS: &'static [&'static str] = &[...]; }` beside the struct, or pass `exports = [...]` as an attribute argument",
                        ));
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            field,
                            format!(
                                "unknown `object` field `{name}`; a face struct only carries `preset` and `parts` types"
                            ),
                        ));
                    }
                }
            }
        }
        syn::Fields::Unit => {}
        syn::Fields::Unnamed(_) => {
            return Err(syn::Error::new_spanned(
                &item_struct,
                "`#[nichlink::object]` expects a plain struct (no tuple fields)",
            ));
        }
    }

    let kind = item_struct.ident.clone();
    let kind_str = kind.to_string();
    let kind_lit = LitStr::new(&kind_str, Span::call_site());

    let mut args = parse_args(attr)?;
    // Take the group fields up front so the remaining `values` map can be
    // consumed piecemeal without partial-move conflicts.
    // 先取走分组字段，余下的 `values` 才能逐键消耗而不触发部分移动冲突。
    let (name_zh, name_en) = args.name.take().unwrap_or_else(|| {
        (
            LitStr::new(&kind_str, Span::call_site()),
            LitStr::new(&kind_str, Span::call_site()),
        )
    });
    let (summary_zh, summary_en) = args.summary.take().unwrap_or_else(|| {
        (
            LitStr::new("", Span::call_site()),
            LitStr::new("", Span::call_site()),
        )
    });
    let requires = std::mem::take(&mut args.requires);
    let provides = std::mem::take(&mut args.provides);

    let preset_ty = preset
        .clone()
        .unwrap_or_else(|| parse_str_type("::nichlink_run_method::registry_core::NoPreset"));
    let parts_ty = parts
        .clone()
        .unwrap_or_else(|| parse_str_type("::nichlink_run_method::registry_core::NoParts"));
    let preset_name = LitStr::new(
        &preset.map_or_else(|| "NoPreset".to_owned(), |ty| compact_string(&quote!(#ty))),
        Span::call_site(),
    );
    let parts_name = LitStr::new(
        &parts.map_or_else(|| "NoParts".to_owned(), |ty| compact_string(&quote!(#ty))),
        Span::call_site(),
    );

    let handle_tokens = args.take_value("handle");
    let handle_ty = match &handle_tokens {
        Some(tokens) => parse2::<Type>(tokens.clone())
            .map_err(|error| syn::Error::new(error.span(), "handle expects a type path"))?,
        None => Type::Path(syn::TypePath {
            qself: None,
            path: kind.clone().into(),
        }),
    };
    let handle_str = match &handle_tokens {
        Some(tokens) => compact_string(tokens),
        None => kind_str.clone(),
    };
    let handle_lit = LitStr::new(&handle_str, Span::call_site());

    let source_expr = match args.literal("source")? {
        Some(literal) => quote!(#literal),
        None => quote!(__REGISTRATION_SOURCE),
    };
    let parent_expr = args
        .take_value("parent")
        .map(|tokens| quote!(#tokens))
        .unwrap_or_else(|| quote!(__REGISTRATION_PARENT));

    let params_expr = args
        .take_value("params")
        .map(|tokens| quote!(#tokens))
        .unwrap_or_else(|| quote!(stringify!(#kind)));
    let needs_registry = args
        .boolean("needs_registry")?
        .map(|literal| quote!(#literal))
        .unwrap_or_else(|| quote!(false));
    let registry_name_expr = match args.take_value("registry_name") {
        Some(tokens) => {
            let ident = parse2::<Ident>(tokens.clone()).map_err(|_| {
                syn::Error::new_spanned(&tokens, "registry_name expects an identifier")
            })?;
            quote!(stringify!(#ident))
        }
        None => quote!(__REGISTRATION_MODULE_NAME),
    };
    let getting_expr = args
        .take_value("getting_from_other_registry")
        .map(|tokens| quote!(#tokens))
        .unwrap_or_else(|| quote!(None));
    let registry_rule_path_expr = args
        .take_value("registry_rule_path")
        .map(|tokens| quote!(#tokens))
        .unwrap_or_else(|| source_expr.clone());
    let registry_rule_expr = args
        .take_value("registry_rule")
        .map(|tokens| quote!(#tokens))
        .unwrap_or_else(|| quote!(::nichlink_run_method::registry_core::RegistrationRule::ANY));
    let admission_expr = args
        .take_value("admission")
        .map(|tokens| quote!(#tokens))
        .unwrap_or_else(|| quote!(::nichlink_run_method::registry_core::Admission::ANY));
    let stable_name_expr = args
        .literal("stable_name")?
        .map(|literal| quote!(::core::option::Option::Some(#literal)))
        .unwrap_or_else(|| quote!(::core::option::Option::None));
    let expected_output = args
        .take_value("expected_output")
        .map(|tokens| quote!(#tokens))
        .unwrap_or_else(|| quote!("()"));
    let actual_output = args
        .take_value("actual_output")
        .map(|tokens| quote!(#tokens))
        .unwrap_or_else(|| quote!("()"));
    let plugin_expr = args
        .take_value("plugin")
        .map(|tokens| quote!(::core::option::Option::Some(#tokens)))
        .unwrap_or_else(|| quote!(::core::option::Option::None));

    let exports_expr = args
        .take_value("exports")
        .map(|tokens| quote!(#tokens))
        .unwrap_or_else(|| quote!(#kind::EXPORTS));

    let handle_traits = string_list(&args.list("handle_traits")?, "handle_traits")?;
    let part_traits = string_list(&args.list("part_traits")?, "part_traits")?;
    let handle_contracts = args.list("handle_contracts")?;
    let part_contracts = args.list("part_contracts")?;
    let runtime_checks = args.list("runtime_checks")?;

    let require_caps = requires.iter().map(|(cap, _)| cap).collect::<Vec<_>>();
    let require_providers = requires.iter().map(|(_, prov)| prov).collect::<Vec<_>>();

    let flow_provider_ty = args.path("flow_provider")?;
    let flow_expr = match (args.take_value("flow"), &flow_provider_ty) {
        (Some(tokens), _) => quote!(#tokens),
        (None, Some(provider)) => quote!(
            <#provider as ::nichlink_run_method::registry_core::FlowContractProvider>::FLOW_CONTRACT
        ),
        (None, None) => quote!(::nichlink_run_method::registry_core::FlowContract::NONE),
    };
    let flow_provider_expr = match &flow_provider_ty {
        Some(provider) => quote!(::core::option::Option::Some(stringify!(#provider))),
        None => quote!(::core::option::Option::None),
    };

    let collector_debug = match args.take_value("collector") {
        Some(tokens) => {
            let ident = parse2::<Ident>(tokens.clone())
                .map_err(|_| syn::Error::new_spanned(&tokens, "collector expects an identifier"))?;
            if ident != "debug" {
                return Err(syn::Error::new_spanned(
                    &ident,
                    "unsupported collector; the only value is `debug`",
                ));
            }
            true
        }
        None => false,
    };

    let handle_assert = interface_assert(&handle_ty, &handle_contracts)?;
    let parts_assert = interface_assert(&parts_ty, &part_contracts)?;

    let submit = if collector_debug {
        quote! {
            #[cfg(debug_assertions)]
            ::nichlink_debug_method::submit! { REGISTRATION }
        }
    } else {
        TokenStream2::new()
    };

    if let Some(error) = args.leftover() {
        return Err(error);
    }

    let expanded = quote! {
        #item_struct

        impl ::nichlink_run_method::registry_core::FaceExports for #kind {}

        const _: () = ::nichlink_run_method::registry_core::assert_contract::<#preset_ty, #parts_ty>();
        #handle_assert
        #parts_assert

        #[doc(hidden)]
        pub const NODE_ID: ::nichlink_run_method::registry_core::NodeId =
            ::nichlink_run_method::registry_core::NodeId::from_namespaced_path(
                env!("CARGO_PKG_NAME"),
                #source_expr,
                #kind_lit,
            );

        #[doc(hidden)]
        pub const REGISTRATION: ::nichlink_run_method::registry_core::RegistrationInfo = {
            // Anonymous import so `Kind::EXPORTS` resolves through the
            // `FaceExports` fallback when the face has no inherent
            // `EXPORTS` const; an inherent impl still takes priority.
            // 匿名导入让没有固有 `EXPORTS` 常量时 `Kind::EXPORTS` 仍能经由
            // `FaceExports` 缺省解析；固有 impl 存在时依旧优先。
            use ::nichlink_run_method::registry_core::FaceExports as _;
            ::nichlink_run_method::registry_core::RegistrationInfo {
                namespace: env!("CARGO_PKG_NAME"),
                id: NODE_ID,
                parent: #parent_expr,
                kind: #kind_lit,
                preset: #preset_name,
                parts: #parts_name,
                params: #params_expr,
                handle: #handle_lit,
                stable_name: #stable_name_expr,
                name: ::nichlink_run_method::registry_core::LocalizedText {
                    zh: #name_zh,
                    en: #name_en,
                },
                summary: ::nichlink_run_method::registry_core::LocalizedText {
                    zh: #summary_zh,
                    en: #summary_en,
                },
                exports: #exports_expr,
                needs_registry: #needs_registry,
                registry_name: #registry_name_expr,
                getting_from_other_registry: #getting_expr,
                registry_rule_path: #registry_rule_path_expr,
                registry_rule: #registry_rule_expr,
                admission: #admission_expr,
                requires: &[#(
                    ::nichlink_run_method::registry_core::RequirementSpec {
                        capability: #require_caps,
                        provider: #require_providers,
                    },
                )*],
                provides: &[#(#provides),*],
                contract: ::nichlink_run_method::registry_core::ObjectContract {
                    required_parts: <#preset_ty as ::nichlink_run_method::registry_core::PresetContract>::REQUIRED_PARTS,
                    provided_parts: <#parts_ty as ::nichlink_run_method::registry_core::PartsContract>::PROVIDED_PARTS,
                    expected_output: #expected_output,
                    actual_output: #actual_output,
                },
                flow: #flow_expr,
                flow_provider: #flow_provider_expr,
                handle_traits: #handle_traits,
                part_traits: #part_traits,
                runtime_checks: &[#(#runtime_checks),*],
                plugin: #plugin_expr,
                source: ::nichlink_run_method::registry_core::SourceLocation {
                    file: #source_expr,
                    line: line!(),
                    column: column!(),
                    function: #handle_lit,
                },
            }
        };

        #submit
    };
    Ok(expanded)
}

fn parse_str_type(path: &str) -> Type {
    syn::parse_str::<Type>(path).expect("well-known kernel path parses as a type")
}

fn parse_args(attr: TokenStream2) -> syn::Result<Args> {
    let mut args = Args::default();
    let mut seen = Vec::new();
    for chunk in split_top_level(attr) {
        let mut trees = chunk.into_iter();
        let Some(TokenTree::Ident(name)) = trees.next() else {
            return Err(syn::Error::new(
                Span::call_site(),
                "expected `name = value` or `name(...)` in the `object` attribute",
            ));
        };
        let key = name.to_string();
        if seen.contains(&key) {
            return Err(syn::Error::new_spanned(
                &name,
                format!("duplicate `object` argument `{key}`"),
            ));
        }
        seen.push(key.clone());
        match trees.next() {
            Some(TokenTree::Punct(punct)) if punct.as_char() == '=' => {
                let value: TokenStream2 = trees.collect();
                if value.is_empty() {
                    return Err(syn::Error::new_spanned(
                        &name,
                        format!("`{key}` has no value"),
                    ));
                }
                args.values.push((key, value));
            }
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
                parse_group(&mut args, &key, group.stream(), name.span())?;
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    &name,
                    format!("expected `=` or `(...)` after `{key}`"),
                ));
            }
        }
    }
    Ok(args)
}

fn parse_group(args: &mut Args, key: &str, tokens: TokenStream2, span: Span) -> syn::Result<()> {
    match key {
        "name" | "summary" => {
            let pair = parse_localized(&tokens, key)?;
            if key == "name" {
                args.name = Some(pair);
            } else {
                args.summary = Some(pair);
            }
            Ok(())
        }
        "requires" => {
            for chunk in split_top_level(tokens) {
                let trees = chunk.into_iter().collect::<Vec<_>>();
                let Some(arrow) = trees
                    .windows(2)
                    .position(|pair| is_punct(&pair[0], '=') && is_punct(&pair[1], '>'))
                else {
                    return Err(syn::Error::new(
                        span,
                        "requires entries expect `\"capability\" => \"provider\"`",
                    ));
                };
                let capability = parse2::<LitStr>(trees[..arrow].iter().cloned().collect())
                    .map_err(|_| {
                        syn::Error::new(span, "requires capability must be a string literal")
                    })?;
                let provider = parse2::<LitStr>(trees[arrow + 2..].iter().cloned().collect())
                    .map_err(|_| {
                        syn::Error::new(span, "requires provider must be a string literal")
                    })?;
                args.requires.push((capability, provider));
            }
            Ok(())
        }
        "provides" => {
            for chunk in split_top_level(tokens) {
                args.provides.push(parse2::<LitStr>(chunk).map_err(|_| {
                    syn::Error::new(span, "provides entries must be string literals")
                })?);
            }
            Ok(())
        }
        _ => Err(syn::Error::new(
            span,
            format!(
                "unknown `object` group `{key}(...)`; expected one of: {}",
                GROUP_KEYS.join(", ")
            ),
        )),
    }
}

fn parse_localized(tokens: &TokenStream2, key: &str) -> syn::Result<(LitStr, LitStr)> {
    let mut zh = None;
    let mut en = None;
    for chunk in split_top_level(tokens.clone()) {
        let mut trees = chunk.into_iter();
        let Some(TokenTree::Ident(lang)) = trees.next() else {
            return Err(syn::Error::new(
                Span::call_site(),
                format!("{key}(...) expects `zh = \"...\"` / `en = \"...\"`"),
            ));
        };
        let Some(TokenTree::Punct(punct)) = trees.next() else {
            return Err(syn::Error::new_spanned(
                &lang,
                format!("expected `=` after `{lang}`"),
            ));
        };
        if punct.as_char() != '=' {
            return Err(syn::Error::new_spanned(
                &lang,
                format!("expected `=` after `{lang}`"),
            ));
        }
        let literal = parse2::<LitStr>(trees.collect::<TokenStream2>())
            .map_err(|_| syn::Error::new_spanned(&lang, "expected a string literal"))?;
        match lang.to_string().as_str() {
            "zh" => zh = Some(literal),
            "en" => en = Some(literal),
            other => {
                return Err(syn::Error::new_spanned(
                    &lang,
                    format!("unsupported language `{other}`; use `zh` or `en`"),
                ));
            }
        }
    }
    Ok((
        zh.ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                format!("{key}(...) misses `zh = \"...\"`"),
            )
        })?,
        en.ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                format!("{key}(...) misses `en = \"...\"`"),
            )
        })?,
    ))
}

fn string_list(chunks: &[TokenStream2], key: &str) -> syn::Result<TokenStream2> {
    let literals = chunks
        .iter()
        .map(|chunk| parse2::<LitStr>(chunk.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| {
            syn::Error::new(
                Span::call_site(),
                format!("{key} entries must be string literals"),
            )
        })?;
    Ok(quote!(&[#(#literals),*]))
}

fn interface_assert(ty: &Type, contracts: &[TokenStream2]) -> syn::Result<TokenStream2> {
    if contracts.is_empty() {
        return Ok(TokenStream2::new());
    }
    let bounds = contracts
        .iter()
        .map(|tokens| {
            parse2::<Type>(tokens.clone())
                .map_err(|error| syn::Error::new(error.span(), "contract expects a trait path"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(quote! {
        const _: fn() = || {
            fn assert_impl<T>()
            where
                #(T: #bounds),*
            {
            }
            assert_impl::<#ty>();
        };
    })
}

fn single_group(tokens: &TokenStream2, delimiter: Delimiter) -> Option<proc_macro2::Group> {
    let mut trees = tokens.clone().into_iter();
    let TokenTree::Group(group) = trees.next()? else {
        return None;
    };
    (group.delimiter() == delimiter && trees.next().is_none()).then_some(group)
}

fn split_top_level(tokens: TokenStream2) -> Vec<TokenStream2> {
    let mut items = Vec::new();
    let mut current = TokenStream2::new();
    for token in tokens {
        if matches!(&token, TokenTree::Punct(punct) if punct.as_char() == ',') {
            if !current.is_empty() {
                items.push(current);
                current = TokenStream2::new();
            }
        } else {
            current.extend([token]);
        }
    }
    if !current.is_empty() {
        items.push(current);
    }
    items
}

fn is_punct(token: &TokenTree, ch: char) -> bool {
    matches!(token, TokenTree::Punct(punct) if punct.as_char() == ch)
}

fn compact_string(tokens: &TokenStream2) -> String {
    tokens
        .to_string()
        .replace(" :: ", "::")
        .replace(" (", "(")
        .replace(") ", ")")
}

#[cfg(test)]
mod tests {
    use super::expand_object;
    use proc_macro2::TokenStream;

    fn expand(attr: &str, item: &str) -> String {
        let attr = attr.parse::<TokenStream>().expect("attr tokens");
        let item = item.parse::<TokenStream>().expect("item tokens");
        expand_object(attr, item)
            .expect("expansion succeeds")
            .to_string()
    }

    #[test]
    fn minimal_face_uses_kernel_defaults() {
        let out = expand("", "pub struct Button {}");
        assert!(out.contains("pub struct Button"));
        assert!(out.contains("FaceExports for Button"));
        assert!(out.contains("pub const NODE_ID"));
        assert!(out.contains("pub const REGISTRATION"));
        assert!(out.contains("__REGISTRATION_PARENT"));
        assert!(out.contains("__REGISTRATION_SOURCE"));
        assert!(out.contains("__REGISTRATION_MODULE_NAME"));
        assert!(out.contains("\"NoPreset\""));
        assert!(out.contains("\"NoParts\""));
        assert!(out.contains("Button :: EXPORTS"));
        assert!(!out.contains("nichlink_debug_method"));
    }

    #[test]
    fn unit_struct_is_accepted_as_a_minimal_face() {
        let out = expand("", "pub struct Button;");
        assert!(out.contains("pub struct Button"));
        assert!(out.contains("\"NoPreset\""));
        assert!(out.contains("\"NoParts\""));
    }

    #[test]
    fn full_metadata_maps_to_registration_fields() {
        let out = expand(
            r#"parent = crate::control::NODE_ID, needs_registry = true, stable_name = "button.v1",
               handle = MyHandle, exports = ["control.render"], registry_name = button,
               name(zh = "按钮", en = "Button"), summary(zh = "摘要", en = "Summary"),
               requires("layout.viewport" => "ControlRegistry"), provides("control.render"),
               handle_traits = ["Drawable"], runtime_checks = [CHECK],
               expected_output = "u32", actual_output = "u32",
               flow_provider = crate::flow::ButtonFlow"#,
            "pub struct Button { preset: ControlPreset, parts: ButtonParts }",
        );
        assert!(out.contains("parent : crate :: control :: NODE_ID"));
        assert!(out.contains("needs_registry : true"));
        assert!(out.contains("stable_name : :: core :: option :: Option :: Some (\"button.v1\")"));
        assert!(out.contains("handle : \"MyHandle\""));
        assert!(out.contains("exports : [\"control.render\"]"));
        assert!(out.contains("registry_name : stringify ! (button)"));
        assert!(out.contains("zh : \"按钮\""));
        assert!(out.contains("en : \"Button\""));
        assert!(out.contains("capability : \"layout.viewport\""));
        assert!(out.contains("provider : \"ControlRegistry\""));
        assert!(out.contains("provides : & [\"control.render\"]"));
        assert!(out.contains("handle_traits : & [\"Drawable\"]"));
        assert!(out.contains("runtime_checks : & [CHECK]"));
        assert!(out.contains("expected_output : \"u32\""));
        assert!(out.contains("FLOW_CONTRACT"));
        assert!(out.contains("Option :: Some (stringify ! (crate :: flow :: ButtonFlow))"));
        assert!(out.contains("assert_contract :: < ControlPreset , ButtonParts >"));
    }

    #[test]
    fn custom_handle_emits_interface_asserts() {
        let out = expand(
            "handle = MyHandle, handle_contracts = [Drawable, Focusable]",
            "pub struct Button {}",
        );
        assert!(out.contains("assert_impl :: < MyHandle >"), "{}", out);
        assert!(out.contains("T : Drawable , T : Focusable"), "{}", out);
    }

    #[test]
    fn external_source_uses_literal_source_and_rule_path() {
        let out = expand(
            r#"source = "control/control.rs", parent = crate::control::NODE_ID"#,
            "pub struct Button {}",
        );
        assert!(out.contains("file : \"control/control.rs\""));
        assert!(out.contains("registry_rule_path : \"control/control.rs\""));
        assert!(!out.contains("__REGISTRATION_SOURCE"));
        assert!(!out.contains("nichlink_debug_method"));
    }

    #[test]
    fn collector_debug_emits_debug_submit() {
        let out = expand("collector = debug", "pub struct Button {}");
        assert!(out.contains("nichlink_debug_method :: submit"));
    }

    #[test]
    fn unknown_field_is_rejected_with_hint() {
        let attr = TokenStream::new();
        let item = "pub struct Button { wat: Thing }"
            .parse::<TokenStream>()
            .unwrap();
        let error = expand_object(attr, item).unwrap_err().to_string();
        assert!(error.contains("unknown `object` field `wat`"));
    }

    #[test]
    fn exports_field_is_rejected_with_impl_hint() {
        let attr = TokenStream::new();
        let item = "pub struct Button { exports: &'static [&'static str] }"
            .parse::<TokenStream>()
            .unwrap();
        let error = expand_object(attr, item).unwrap_err().to_string();
        assert!(error.contains("impl Kind { pub const EXPORTS"));
    }

    #[test]
    fn unknown_argument_is_rejected() {
        let attr = "frobnicate = true".parse::<TokenStream>().unwrap();
        let item = "pub struct Button {}".parse::<TokenStream>().unwrap();
        let error = expand_object(attr, item).unwrap_err().to_string();
        assert!(error.contains("unknown `object` argument `frobnicate`"));
    }

    #[test]
    fn duplicate_argument_is_rejected() {
        let attr = "stable_name = \"a\", stable_name = \"b\""
            .parse::<TokenStream>()
            .unwrap();
        let item = "pub struct Button {}".parse::<TokenStream>().unwrap();
        let error = expand_object(attr, item).unwrap_err().to_string();
        assert!(error.contains("duplicate `object` argument"));
    }
}
