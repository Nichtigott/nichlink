//! A `kind`-only `external_object!` takes the generated compact defaults.
//! 只写 `kind` 的 `external_object!` 取生成式紧凑形式的默认值。
//!
//! Before B3b the hidden `__external_object!` matcher required twenty fields, so
//! the documented "each one optional" external form only compiled when the
//! author restated every default by hand. This file pins the compact arm: the
//! declaration below writes `collector` and `kind` and nothing else.
//! B3b 之前，隐藏的 `__external_object!` matcher 要求二十个字段，因此文档里
//! “每一项都可省略”的外部形式只有把每个默认值手写一遍才编译得过。本文件钉住紧凑
//! arm：下面的声明只写 `collector` 与 `kind`，别的什么都不写。

/// The handle-marker type the declaration below names. `kind` is captured as an
/// identifier, so it has to be an item the author wrote before the macro.
/// 下面那条声明命名的 handle 标记类型。`kind` 以标识符捕获，因此它必须是作者在宏
/// 之前写下的条目。
pub struct CompactExternal;

nichlink_run_method::external_object! {
    collector: development,
    kind: CompactExternal,
}

/// Only `kind` was written, so `registry_name` must fall back to the last
/// segment of `module_path!()` — not to `stringify!(kind)`, which was the
/// deleted kind-only arm's output and is the uppercase spelling the module
/// convention avoids.
/// 只写了 `kind`，因此 `registry_name` 必须回退到 `module_path!()` 的末段——而不是
/// `stringify!(kind)`：那是被删掉的 kind-only arm 的产物，也是模块命名约定避开的
/// 大写拼写。
#[test]
fn a_kind_only_external_face_derives_its_registry_name_from_the_module() {
    assert_eq!(REGISTRATION.registry_name, "external_compact_face");
}

/// `parent` must default to the package root, exactly as the generated compact
/// arm does; an external face that wrote no parent still hangs somewhere known.
/// `parent` 必须像生成的紧凑 arm 一样回退到包根；没写父级的外部面仍然挂在已知位置。
#[test]
fn a_kind_only_external_face_hangs_from_the_package_root() {
    assert_eq!(
        REGISTRATION.parent,
        nichlink_run_method::root_node_id(env!("CARGO_PKG_NAME"))
    );
}

/// `preset`/`parts` have to default to `NoPreset`/`NoParts` in the *record*, not
/// just in the type position: the contract stores the name, and a wrong name
/// would silently misreport the face's structural requirements.
/// `preset`/`parts` 必须在**记录**里回退到 `NoPreset`/`NoParts`，而不只在类型位置：
/// 合同存的是名字，名字错了会静默误报注册面的结构要求。
#[test]
fn a_kind_only_external_face_defaults_preset_and_parts() {
    assert_eq!(REGISTRATION.preset, "NoPreset");
    assert_eq!(REGISTRATION.parts, "NoParts");
    // The handle defaults to the kind, and the name/params follow its spelling.
    // handle 回退到 kind，显示名与参数描述沿用它的拼写。
    assert_eq!(REGISTRATION.handle, "CompactExternal");
    assert_eq!(REGISTRATION.name.zh, "CompactExternal");
    assert_eq!(REGISTRATION.params, "CompactExternal");
}

/// An external crate has no build-generated sibling `registry_rule` module, so
/// the omitted rule must be `ANY` and never the relative-path resolver.
/// 外部 crate 没有构建生成的兄弟 `registry_rule` 模块，因此省略规则时必须得到
/// `ANY`，绝不能走相对路径解析器。
#[test]
fn a_kind_only_external_face_defaults_registry_rule_to_any() {
    let rule = REGISTRATION.registry_rule;
    assert_eq!(rule.required_preset, None);
    assert!(rule.required_parts.is_empty());
    assert!(rule.required_exports.is_empty());
    assert!(rule.required_handle_traits.is_empty());
    assert!(rule.required_part_traits.is_empty());
}

/// The remaining defaults: no registry, no cross-registry implementation, no
/// exports, requirements, provides, or runtime checks — and `source` names this
/// test file rather than an absolute path.
/// 其余默认值：不拥有注册机、实现不来自别的注册机、没有导出、需求、提供或运行期
/// 检查——`source` 命名的是本测试文件，而不是绝对路径。
#[test]
fn a_kind_only_external_face_defaults_the_remaining_fields() {
    // Read the flag through a collection so the comparison is made on a value
    // rather than on a constant clippy would fold away.
    // 通过集合读取这个布尔值，让比较发生在值上，而不是被 clippy 折叠掉的常量上。
    let registered = [REGISTRATION.needs_registry];
    assert_eq!(registered, [false]);
    assert_eq!(REGISTRATION.getting_from_other_registry, None);
    assert!(REGISTRATION.exports.is_empty());
    assert!(REGISTRATION.requires.is_empty());
    assert!(REGISTRATION.provides.is_empty());
    assert!(REGISTRATION.runtime_checks.is_empty());
    // `file!()` reaches an integration-test target as a workspace-relative path,
    // so `manifest_relative_source` has no manifest prefix to strip and returns
    // it unchanged; the default `source` is therefore this test file itself.
    // It keeps the host's separator as well — Windows records
    // `run_method\tests\external_compact_face.rs` — because a declaration cannot
    // rewrite `file!()` at compile time without allocating. The comparison is
    // therefore made on the portable form, exactly as
    // `external_source_default.rs` does. Identity is unaffected: `NodeId` folds
    // both separators to the same byte, so the two spellings are one face.
    // `file!()` 在集成测试目标里是相对工作区的路径，因此
    // `manifest_relative_source` 没有清单前缀可剥，原样返回；默认 `source`
    // 就是本测试文件。它同时保留宿主的分隔符——Windows 记录
    // `run_method\tests\external_compact_face.rs`——因为声明在编译期无法在不分配的前提下
    // 改写 `file!()`。因此比较在可移植形式上做，与 `external_source_default.rs` 一致。
    // 身份不受影响：`NodeId` 把两种分隔符折叠为同一字节，两种拼法是同一张面。
    assert_eq!(
        nichlink_run_method::registry_core::portable_path(REGISTRATION.source.file),
        "run_method/tests/external_compact_face.rs"
    );
}
