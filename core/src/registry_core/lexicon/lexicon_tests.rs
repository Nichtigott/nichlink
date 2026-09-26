//! Tests for the shared text contracts and the pure package-root rules.
//! 共享文本契约与纯包根规则的测试。

use super::*;
use crate::registry_core::declaration::FACE_FIELD_ORDER;

/// The published values. Every one of these is on disk, in a generated
/// crate, or in a host's environment, so changing one is a format change
/// and not a refactor.
/// 已发布的值。每一个都出现在磁盘上、生成的 crate 里或宿主的环境里，因此改动
/// 其中任何一个都是格式变更，而不是重构。
#[test]
fn the_text_contracts_keep_their_published_values() {
    assert_eq!(GENERATED_LIB_FILE, "generated_lib.rs");
    assert_eq!(RUN_METHOD_CRATE, "nichlink_run_method");
    assert_eq!(FACE_FIELD_PLUGIN, "plugin");
    assert_eq!(FACE_FIELD_COLLECTOR, "collector");
    assert_eq!(SCOPE_ENV, "NICH_LINK_SCOPE");
    assert_eq!(ENTRY_ENV, "NICH_LINK_ENTRY");
    assert_eq!(BUILD_VERBOSE_ENV, "NICH_LINK_BUILD_VERBOSE");
    assert_eq!(PACKAGE_ROOT_ENV, "NICH_LINK_PACKAGE_ROOT");
    assert_eq!(NAMESPACE_ENV, "NICH_LINK_NAMESPACE");
    assert_eq!(DEFAULT_NAMESPACE, "nichlink.default");
    assert_eq!(NICHLINK_DIR, ".nichlink");
    assert_eq!(EXTERNAL_GRAFT_DIR, "external-grafts");
    assert_eq!(GRAFT_PLAN_FILE, "graft.plan");
    assert_eq!(TRACE_DIR, "traces");
    assert_eq!(TRACE_FILE, "nichlink.trace");
    assert_eq!(TRACE_FILE_ENV, "NICH_LINK_TRACE_FILE");
}

/// The package-root rule, at each of its three steps and at the boundary a
/// relative configured value crosses.
/// 包根规则的三步，以及相对配置值所跨过的边界。
#[test]
fn the_package_root_rule_prefers_the_explicit_value_then_a_real_package() {
    let fallback = Path::new("/fallback");
    // 1. An absolute configured value wins over everything else, including a
    //    working directory that is a package.
    // 1. 绝对的配置值胜过一切，包括本身就是一个包的当前目录。
    assert_eq!(
        resolve_package_root(
            Some(Path::new("/configured")),
            Some(Path::new("/work")),
            true,
            fallback
        ),
        Path::new("/configured")
    );
    // A relative one is relative to the working directory, not to the
    // process's install location.
    // 相对值相对的是当前目录，而不是进程的安装位置。
    assert_eq!(
        resolve_package_root(
            Some(Path::new("inner")),
            Some(Path::new("/work")),
            false,
            fallback
        ),
        Path::new("/work/inner")
    );
    // 2. The working directory is used only when it holds a package.
    // 2. 只有当当前目录本身是一个包时才使用它。
    assert_eq!(
        resolve_package_root(None, Some(Path::new("/work")), true, fallback),
        Path::new("/work")
    );
    assert_eq!(
        resolve_package_root(None, Some(Path::new("/work")), false, fallback),
        fallback
    );
    // 3. Nothing configured and no working directory at all is a fallback,
    //    not a panic.
    // 3. 既无配置也无当前目录时走兜底，而不是 panic。
    assert_eq!(resolve_package_root(None, None, false, fallback), fallback);
}

/// An unset or empty namespace resolves to the documented default.
/// 未设置或为空的命名空间解析为文档化的默认值。
#[test]
fn the_namespace_default_is_the_documented_one() {
    assert_eq!(resolve_namespace(None), DEFAULT_NAMESPACE);
    assert_eq!(resolve_namespace(Some("app")), "app");
}

/// The scope exemption list is the registration machinery, and the
/// registration module leads it because only its subtree is matched by
/// path prefix rather than by name.
/// 范围豁免表就是注册机制本身；注册模块排在首位，因为只有它的子树按路径前缀
/// 匹配，而不是按名字匹配。
#[test]
fn scope_exemptions_are_the_registration_machinery() {
    assert_eq!(
        SCOPE_ALWAYS_INCLUDED,
        [
            "registry_core",
            "registry",
            "rules",
            "registry_rule",
            "root_registry"
        ]
    );
    assert_eq!(
        SCOPE_ALWAYS_INCLUDED.first(),
        Some(&SCOPE_REGISTRATION_MODULE)
    );
}

/// A field spelling that leaves the vocabulary takes this constant with it:
/// the front end would otherwise keep recognising a field no face may
/// declare.
/// 字段拼写若离开词表，这个常量也必须一起走：否则前端会继续识别一个任何注册面
/// 都不许声明的字段。
#[test]
fn the_plugin_spelling_is_part_of_the_field_vocabulary() {
    assert!(
        FACE_FIELD_ORDER.contains(&FACE_FIELD_PLUGIN),
        "{FACE_FIELD_ORDER:?}"
    );
}

/// The const-context comparison used to pin `include!`'s literal.
/// 用来钉住 `include!` 字面量的 const 语境比较。
#[test]
fn same_text_compares_the_whole_text() {
    assert!(same_text("generated_lib.rs", "generated_lib.rs"));
    assert!(same_text("", ""));
    assert!(!same_text("generated_lib.rs", "generated_lib.r"));
    assert!(!same_text("generated_lib.rs", "generated_lib.rss"));
    assert!(!same_text("generated_lib.rs", "generated_lib.rt"));
    assert!(!same_text("", "generated_lib.rs"));
}

/// The registration subtree is matched by directory boundary, so the module
/// name itself and a sibling whose name merely starts the same way stay out.
/// 注册子树按目录边界匹配，因此模块名本身以及名字只是开头相同的兄弟模块都不算。
#[test]
fn the_registration_subtree_is_matched_by_directory_boundary() {
    assert!(is_registration_path("registry_core/tree/tree.rs"));
    assert!(!is_registration_path("registry_core"));
    assert!(!is_registration_path("registry_core.rs"));
    assert!(!is_registration_path("registry_core_extra/tree.rs"));
    assert!(!is_registration_path("registry/rules.rs"));
    assert!(!is_registration_path(""));
}

/// The shared predicate owns both halves of the boundary — equality and the
/// directory separator — and `is_registration_path` is that predicate minus
/// the one case the scope list matches by name.
/// 共享谓词同时拥有边界的两半——相等与目录分隔符——而 `is_registration_path` 就是
/// 它减去范围表按名字匹配的那一种情况。
#[test]
fn the_shared_prefix_check_owns_equality_and_the_directory_boundary() {
    assert!(path_is_under("registry_core", "registry_core"));
    assert!(path_is_under("registry_core/tree/tree.rs", "registry_core"));
    assert!(path_is_under("ui/controls/button.rs", "ui/controls"));
    assert!(!path_is_under("registry_core.rs", "registry_core"));
    assert!(!path_is_under(
        "registry_core_extra/tree.rs",
        "registry_core"
    ));
    assert!(!path_is_under("", "registry_core"));
    assert!(!path_is_under("ui2/button.rs", "ui"));
    // The scope exemption list matches the module name by name, so the
    // fixed-prefix form must not treat the name itself as a subtree.
    // 范围豁免表按名字匹配模块名，因此固定前缀形式不得把该名字本身当作子树。
    assert!(!is_registration_path(SCOPE_REGISTRATION_MODULE));
    assert!(path_is_under(
        "registry_core/tree/tree.rs",
        SCOPE_REGISTRATION_MODULE
    ));
}

/// The connector's strict form is the shared predicate minus equality, and
/// the two meanings are pinned side by side so an equality-inclusive gate
/// and the connector cannot silently converge again.
/// 连接器的严格形式就是共享谓词去掉相等；两种含义并排钉住，因此"含相等"的门与连接器
/// 不会再悄悄合流。
#[test]
fn the_strict_prefix_check_excludes_equality() {
    assert!(!path_is_strictly_under("registry_core", "registry_core"));
    assert!(path_is_strictly_under(
        "registry_core/tree/tree.rs",
        "registry_core"
    ));
    assert!(!path_is_strictly_under("registry_core.rs", "registry_core"));
    assert!(!path_is_strictly_under("ui2/button.rs", "ui"));
    assert!(path_is_strictly_under("ui/button.rs", "ui"));
    // Both forms agree away from equality, including on the empty prefix.
    // 除相等外两种形式一致，空前缀上也是。
    assert!(!path_is_strictly_under("", ""));
    assert_eq!(
        path_is_strictly_under("a/b", "a"),
        path_is_under("a/b", "a")
    );
}
