//! Face parser and reference-scan tests.
//! 注册面解析与引用扫描的测试。

use super::{ParentSyntax, parse_face, replace_face_macro, source_references};

#[test]
fn parses_multiline_registration_tokens_and_locations() {
    let source = r#"
crate::control_object! {
    kind: Button,
    name: { zh: "按钮", en: "Button" },
    requires: [
        "layout.viewport" => "ControlRegistry",
        "draw.basic" => "BasicDrawing",
    ],
    parent: crate::NodeId::from_path("control/control.rs", "ControlRegistry"),
    getting_from_other_registry: Some("engine"),
}
"#;
    let face = parse_face(source).unwrap().unwrap();

    assert_eq!(face.path("kind").as_deref(), Some("Button"));
    assert_eq!(face.localized("name", "zh").as_deref(), Some("按钮"));
    assert_eq!(face.field_location("requires").unwrap().line, 5);
    assert_eq!(
        face.requirements("requires").unwrap(),
        [
            ("layout.viewport".to_owned(), "ControlRegistry".to_owned()),
            ("draw.basic".to_owned(), "BasicDrawing".to_owned())
        ]
    );
    assert_eq!(
        face.parent(),
        Some(ParentSyntax::FromPath {
            source: "control/control.rs".to_owned(),
            kind: "ControlRegistry".to_owned(),
        })
    );
    assert_eq!(
        face.option_string("getting_from_other_registry"),
        Some(Some("engine".to_owned()))
    );
}

#[test]
fn parses_namespaced_package_root_parent() {
    let source = r#"
crate::control_object! {
    kind: Workspace,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
}
"#;
    let face = parse_face(source).unwrap().unwrap();

    assert_eq!(face.parent(), Some(ParentSyntax::Root));
}

/// A parent path is matched at a path-segment boundary, not by suffix: `crate::NOT_ROOT_NODE_ID`
/// is a different constant, `my_root_node_id` a different function, and `OwnNodeId::from_path` a
/// different type's constructor. Suffix matching classified all three as the parent the face
/// declared (audit `KN6`).
/// 父路径在**路径段边界**上匹配，而不是按后缀：`crate::NOT_ROOT_NODE_ID` 是另一个常量、
/// `my_root_node_id` 是另一个函数、`OwnNodeId::from_path` 是另一个类型的构造函数。按后缀匹配会
/// 把三者都判成该面声明的那个父级（审计 `KN6`）。
#[test]
fn a_parent_look_alike_is_not_the_parent() {
    let cases = [
        (
            "crate::NOT_ROOT_NODE_ID",
            "a different constant is not the root",
        ),
        ("my_root_node_id(1)", "a different function is not the root"),
        (
            "OwnNodeId::from_path(\"control/control.rs\", \"ControlRegistry\")",
            "another type's constructor is not this one",
        ),
    ];
    for (parent, why) in cases {
        let source = format!("crate::control_object! {{ kind: Workspace, parent: {parent} }}");
        let face = parse_face(&source).unwrap().unwrap();
        assert_eq!(face.parent(), None, "{why}: {parent}");
    }

    // The spellings the tree does use still classify, so the boundary rule narrows the match
    // rather than breaking it.
    // 树里真正使用的拼法仍然分类成功——边界规则是收紧匹配，而不是弄坏它。
    let real_root = "crate::control_object! { kind: Workspace, parent: crate::ROOT_NODE_ID }";
    assert_eq!(
        parse_face(real_root).unwrap().unwrap().parent(),
        Some(ParentSyntax::Root)
    );
    let own_node = "crate::control_object! { kind: Widget, parent: crate::NODE_ID }";
    assert_eq!(
        parse_face(own_node).unwrap().unwrap().parent(),
        Some(ParentSyntax::NodePath("crate".to_owned()))
    );
}

#[test]
fn rejects_duplicate_fields() {
    let source = "crate::control_object! { kind: First, kind: Second }";
    let error = parse_face(source).unwrap_err();
    assert!(error.message.contains("duplicate field `kind`"));
}

#[test]
fn replacing_a_face_preserves_its_rust_implementation() {
    let source = r#"pub struct Button;
impl Button { pub fn paint(&self) -> u32 { 7 } }
crate::control_object! {
    kind: Button,
    stable_name: "button",
}
#[test] fn paints() { assert_eq!(Button.paint(), 7); }
"#;
    let replacement = r#"crate::control_object! {
    kind: Button,
    stable_name: "button_graft",
}"#;

    let copied = replace_face_macro(source, replacement).unwrap();

    assert!(copied.contains("pub fn paint(&self) -> u32 { 7 }"));
    assert!(copied.contains("stable_name: \"button_graft\""));
    assert!(!copied.contains("stable_name: \"button\","));
    assert!(copied.contains("#[test] fn paints()"));
}

#[test]
fn source_references_ignore_imports_strings_comments_and_registration_data() {
    let source = r#"
use crate::unused::Thing;
fn run_button() {
    crate::control::object::button::dispatch_action("unused::fake()", true);
    // crate::comment::fake();
    crate::control_object! { kind: Fake, parent: crate::hidden::NODE_ID }
}
"#;
    let references = source_references(source).unwrap();
    assert!(
        references
            .paths
            .contains("crate::control::object::button::dispatch_action")
    );
    assert!(!references.paths.iter().any(|path| path.contains("unused")));
    assert!(!references.paths.iter().any(|path| path.contains("hidden")));
    assert!(!references.conservative);
}

/// A macro invocation's tokens are read for paths rather than ignored.
/// 宏调用的 token 会被读出路径，而不是被忽略。
///
/// The build used to walk into no macro body at all, so a face named only as
/// `vec![crate::control::object::dial::NODE_ID]` was pruned while `check` reported
/// ok and the host's `cargo check` failed with `E0433`.
/// 构建过去完全不进入宏内容，因此只被 `vec![crate::control::object::dial::NODE_ID]`
/// 命名的注册面会被剪掉，而 `check` 报 ok、宿主的 `cargo check` 以 `E0433` 失败。
#[test]
fn a_macro_invocation_is_read_for_the_paths_it_spells() {
    let referenced = "pub fn build() { let _ = vec![crate::control::object::dial::NODE_ID]; }";
    let references = source_references(referenced).unwrap();
    assert!(
        references
            .paths
            .contains("crate::control::object::dial::NODE_ID"),
        "a path inside a macro is a reference like any other: {:?}",
        references.paths
    );

    // The same file without the macro keeps the scan narrow, so this is not a
    // blanket widening.
    // 同一份文件去掉宏之后扫描仍然收窄，因此这不是无脑放宽。
    let narrow = "pub fn build() -> u8 { crate::control::dial::NODE_ID }";
    assert!(!source_references(narrow).unwrap().conservative);

    // The build's own wiring is read by its own scanners, and an invocation whose
    // tokens are only literals cannot name a node: neither may widen the scope, or
    // the automatic scope would be off for every real project (an entry always
    // contains `host!()`, and a declaration macro's contents are metadata).
    // 本构建自己的接线由各自的扫描器读取，而 token 全是字面量的调用不可能命名节点：两者都不得
    // 放宽作用域，否则每个真实工程都会失去自动作用域（入口总有 `host!()`，声明宏的内容是元数据）。
    for transparent in [
        "nichlink_toolchain::runtime::host!();",
        "static_graft_plan!(crate::FRAMEWORK, cut \"root/a\");",
        "fn f() { crate::control_object! { kind: Fake } }",
        "const NAME: &str = env!(\"CARGO_PKG_NAME\");",
        "const TWO: &str = concat!(\"a\", \"b\");",
    ] {
        assert!(
            !source_references(transparent).unwrap().conservative,
            "`{transparent}` must not widen the scope"
        );
    }
}

/// The build has to follow a declaration's `cfg` gate, so the parser keeps
/// it exactly as written.
/// 构建需要跟随声明的 `cfg` 门控，因此解析器按原文保留它。
#[test]
fn a_declaration_keeps_its_cfg_gate() {
    let face = parse_face(
        "// generated-by=NichLink\n#[cfg(feature = \"optional-face\")]\n\
             crate::root_object! {\n    kind: Widget,\n}\n",
    )
    .expect("parse")
    .expect("face");
    assert_eq!(face.cfg(), Some("feature = \"optional-face\""));
}

/// A `*_object!` macro that is not a registration declaration is refused for its own
/// reason, instead of being taken for a face that is missing `parent:`.
/// 并非注册声明的 `*_object!` 宏按它自己的理由被拒，而不是被当成缺少 `parent:` 的注册面。
///
/// Red before the fix: the field reader accepted `name: "x", size: 3` as one value (the
/// splitter only starts a field at a name the vocabulary knows), so the macro became a face
/// whose `size` was silently absorbed, and the build then reported a `parent-macro` error
/// for a macro that is not part of the registration mechanism (audit `KRN-K-23`).
/// 修前为红：字段读取器把 `name: "x", size: 3` 收成一个取值（切分器只在词表认识的键处开始
/// 字段），于是该宏成了一个面、`size` 被静默吞掉，随后构建为这个根本不属于注册机制的宏报出
/// `parent-macro` 错误（审计 `KRN-K-23`）。
#[test]
fn a_non_registration_object_macro_is_refused_with_its_own_reason() {
    let error = parse_face("widget_object! { name: \"x\", size: 3 }")
        .expect_err("this body is not a registration declaration");
    let message = error.to_string();
    assert!(
        message.contains("size"),
        "the reason names the text left over: {message}"
    );
    assert!(
        !message.contains("parent"),
        "it is not a missing parent: {message}"
    );

    // The registration vocabulary itself still parses, including a value written with a
    // top-level-looking closure inside brackets.
    // 注册词表本身仍可解析，包括括号内写出、看起来像顶层的闭包取值。
    let face = parse_face(
        "crate::control_object! {\n    kind: Button,\n    needs_registry: true,\n    flow: |a: u32| a,\n}\n",
    )
    .expect("a canonical body parses")
    .expect("one face");
    assert_eq!(face.macro_name, "control_object");
}
