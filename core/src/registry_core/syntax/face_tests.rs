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
fn run() {
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
        "nichlink_run_method::host!();",
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
