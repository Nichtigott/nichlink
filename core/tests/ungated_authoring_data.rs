//! Face authoring data is plain data, not parser data.
//! 注册面创作数据是纯数据，不是解析器数据。
//!
//! The name table and the presentation metadata carry no `syn` dependency, so
//! they must be reachable in a build without the `syntax` feature. Run this
//! file on its own (`cargo test -p nichlink-core --offline`) to check that:
//! a whole-workspace build enables `syntax` through the other members.
//! 名字表与展示元数据不依赖 `syn`，因此在没有 `syntax` 特性的构建里也必须可用。单独
//! 运行本文件（`cargo test -p nichlink-core --offline`）才能验证这一点：整个 workspace
//! 一起构建时，其他成员会把 `syntax` 打开。

#[test]
fn the_field_dictionary_is_reachable_without_the_syntax_feature() {
    assert_eq!(nichlink::authoring::FACE_FIELD_COUNT, 30);
    assert_eq!(
        nichlink::authoring::FACE_FIELD_NAMES.len(),
        nichlink::authoring::FACE_FIELD_COUNT
    );
    let parent = nichlink::authoring::face_field_presentation(0);
    assert!(!parent.label.is_empty(), "{parent:?}");
    assert!(!parent.help.is_empty(), "{parent:?}");
}

/// Pure validation shares that surface: it is `std::path` only.
/// 纯校验同样属于这个表面：它只用到 `std::path`。
#[test]
fn pure_validation_is_reachable_without_the_syntax_feature() {
    assert!(nichlink::authoring::validation::validate_name("control").is_ok());
    assert!(nichlink::authoring::validation::validate_name("Control").is_err());
}
