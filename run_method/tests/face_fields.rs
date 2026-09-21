//! A hand-written face may order its fields freely and separate them with `,`
//! or `;`.
//! 手写注册面可以自由排列字段，并用 `,` 或 `;` 分隔。

mod canonical {
    nichlink_run_method::__nichlink_object! {
        collector: development,
        kind: Ordered,
        name: { zh: "有序", en: "Ordered" },
        needs_registry: true,
        registry_name: Ordered,
        parent: nichlink_run_method::registry_core::root_node_id("face-fields-test"),
        registry_rule: nichlink_run_method::registry_core::RegistrationRule::ANY,
    }
}

mod shuffled {
    nichlink_run_method::__nichlink_object! {
        collector: development;
        registry_rule: nichlink_run_method::registry_core::RegistrationRule::ANY;
        kind: Ordered;
        parent: nichlink_run_method::registry_core::root_node_id("face-fields-test");
        needs_registry: true;
        name: { zh: "有序", en: "Ordered" };
        registry_name: Ordered
    }
}

/// Both declarations name the same face, so both must produce the same
/// identity: `source` comes from `file!()` and both live in this file.
/// 两份声明指的是同一个注册面，因此身份必须一致：`source` 来自 `file!()`，而两者
/// 都在本文件里。
#[test]
fn any_order_and_separator_declares_the_same_face() {
    assert_eq!(canonical::NODE_ID, shuffled::NODE_ID);
    assert_eq!(
        canonical::REGISTRATION.needs_registry,
        shuffled::REGISTRATION.needs_registry
    );
    assert_eq!(
        canonical::REGISTRATION.parent,
        shuffled::REGISTRATION.parent
    );
    assert_eq!(
        canonical::REGISTRATION.registry_name,
        shuffled::REGISTRATION.registry_name
    );
    assert_eq!(
        canonical::REGISTRATION.name.zh,
        shuffled::REGISTRATION.name.zh
    );
    assert_eq!(
        canonical::REGISTRATION.registry_rule_path,
        shuffled::REGISTRATION.registry_rule_path
    );
}
