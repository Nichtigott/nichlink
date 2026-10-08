//! The declaration macros read `crate::NICHLINK_NAMESPACE`, not the package name (audit `M7`, P3.3).
//! 声明宏读的是 `crate::NICHLINK_NAMESPACE`，不是包名（审计 `M7`，P3.3）。

/// A namespace **deliberately different from this crate's package name**.
/// 一个**故意不同于本 crate 包名**的命名空间。
///
/// That difference is the whole pin: a partitioned crate mounts the same face file through another
/// `#[path]`, so if the macros read `env!("CARGO_PKG_NAME")` at the declaration site they would hash
/// the *ghost crate's* name and silently rename every face. With the constant, a crate names its
/// identity domain explicitly — which is what a partition needs.
/// 这个差别就是整条钉子：分区后的 crate 会用另一个 `#[path]` 挂载同一个面文件，因此如果宏在声明处读
/// `env!("CARGO_PKG_NAME")`，它散列的就是**幽灵 crate 的**包名，从而静默重命名每一个面。有了常量之后，
/// 一个 crate 可以显式命名自己的身份域——这正是分区需要的。
pub const NICHLINK_NAMESPACE: &str = "p33-probe-domain";

/// One face per macro family: `__nichlink_object!` is what the generated aliases call (and what
/// `face_objects.rs` implements), `external_object!` is `face_external.rs`. Both used to bake the
/// package name in.
/// 每族宏一个面：`__nichlink_object!` 是生成别名所调用的（也是 `face_objects.rs` 实现的），
/// `external_object!` 是 `face_external.rs`。两者过去都把包名烤进去。
pub mod probe {
    /// The marker type the declaration below names.
    /// 下面那条声明命名的标记类型。
    pub struct Probe;

    nichlink_toolchain::__nichlink_object! {
        kind: Probe,
        parent: nichlink_toolchain::run_method::registry_core::root_node_id("p33-probe-domain"),
        registry_rule: nichlink_toolchain::run_method::registry_core::RegistrationRule::ANY,
    }
}

/// The external family's half of the same pin.
/// 同一件事在外部族那一半。
pub mod external_probe {
    /// The marker type the declaration below names.
    /// 下面那条声明命名的标记类型。
    pub struct Extern;

    nichlink_toolchain::run_method::external_object! {
        collector: development,
        kind: Extern,
    }
}

/// Both faces hash the **constant** — and not the hash the package name would give.
/// 两个面散列的都是那个**常量**——而不是包名会给出的那个散列。
#[test]
fn the_declaration_reads_the_root_constant_instead_of_the_package_name() {
    use nichlink_toolchain::run_method::NodeId;
    use nichlink_toolchain::run_method::registry_core::manifest_relative_source;

    let relative = manifest_relative_source(env!("CARGO_MANIFEST_DIR"), file!());
    let expected = |kind: &str| NodeId::from_namespaced_path(NICHLINK_NAMESPACE, relative, kind);
    let from_package =
        |kind: &str| NodeId::from_namespaced_path(env!("CARGO_PKG_NAME"), relative, kind);

    // The fixture is only meaningful while the two sources differ: that difference is what makes the
    // assertions below about *which* source the macro read.
    // 夹具只在两个来源不同时才有意义：正是那个差别让下面的断言是在问"宏读的是哪一个来源"。
    for kind in ["Probe", "Extern"] {
        assert_ne!(
            expected(kind),
            from_package(kind),
            "the probe domain must differ from the package name"
        );
    }
    assert_eq!(
        crate::probe::NODE_ID,
        expected("Probe"),
        "`__nichlink_object!` hashes the root constant"
    );
    assert_eq!(
        crate::external_probe::NODE_ID,
        expected("Extern"),
        "`external_object!` hashes the same constant"
    );
}
