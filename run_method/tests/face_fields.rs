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

/// Observation evidence must still point at the author's own lines, so the
/// front end must not move `line!()`/`column!()` into the generated alias. The
/// shuffled declaration stands later in this file, and that is what it reports.
/// 观测证据仍须指向作者自己的行，因此前端不能把 `line!()`/`column!()` 挪到生成的
/// 别名里。乱序声明在本文件中位置更靠后，它报出的就是那一行。
#[test]
fn a_reordered_face_reports_the_authors_lines() {
    assert_eq!(
        canonical::REGISTRATION.source.file,
        shuffled::REGISTRATION.source.file
    );
    assert!(
        shuffled::REGISTRATION.source.line > canonical::REGISTRATION.source.line,
        "each declaration reports its own line: canonical={} shuffled={}",
        canonical::REGISTRATION.source.line,
        shuffled::REGISTRATION.source.line
    );
    assert_eq!(canonical::REGISTRATION.source.column, 5);
    assert_eq!(shuffled::REGISTRATION.source.column, 5);
}
