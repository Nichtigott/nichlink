//! A hand-written face may order its fields freely and separate them with `,`
//! or `;`.
//! 手写注册面可以自由排列字段，并用 `,` 或 `;` 分隔。

mod canonical {
    nichlink_run_method::__nichlink_object! {
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
    // Read the lines through a collection so the comparison is made on values
    // rather than on two constants, which a lint would fold away.
    // 通过集合读取行号，让比较发生在值上而不是两个常量上——否则会被 lint 折叠掉。
    let lines = [
        canonical::REGISTRATION.source.line,
        shuffled::REGISTRATION.source.line,
    ];
    assert_ne!(
        lines[0], lines[1],
        "each declaration reports its own line: {lines:?}"
    );
    assert!(lines.iter().all(|line| *line > 0), "lines: {lines:?}");
    assert_eq!(canonical::REGISTRATION.source.column, 5);
    assert_eq!(shuffled::REGISTRATION.source.column, 5);
}

mod external_shuffled {
    use nichlink_run_method::registry_core::{
        ContractId, FlowContract, NoParts, NoPreset, RegistrationRule, root_node_id,
    };

    #[allow(dead_code)]
    pub struct ExternalFast;

    // Same field set as any external face, but `;`-separated and in a different
    // order: the front end must sort it and send it to `__external_object!`.
    // 与任何外部面相同的字段集合，但用 `;` 分隔且顺序不同：前端必须把它排序后送到
    // `__external_object!`。
    nichlink_run_method::external_object! {
        kind: ExternalFast;
        flow: FlowContract::new(ContractId::new("t.v1"), 1, "In", "Out");
        source: "face_fields/external_fast.rs";
        handle: ExternalFast;
        params: "ExternalFast";
        parts: NoParts;
        preset: NoPreset;
        name: { zh: "外部", en: "External" };
        summary: { zh: "外部实现", en: "External implementation" };
        exports: ["t.out"];
        needs_registry: false;
        registry_name: external_shuffled;
        parent: root_node_id(env!("CARGO_PKG_NAME"));
        getting_from_other_registry: None;
        registry_rule_path: "face_fields/external_fast.rs";
        registry_rule: RegistrationRule::ANY;
        requires: [];
        provides: [];
        expected_output: "Out";
        actual_output: "Out";
        runtime_checks: [];
    }
}

/// An external face is written by hand too, so it gets the same tolerance.
/// 外部面同样是手写的，因此享有同样的宽容。
#[test]
fn an_external_face_accepts_semicolons_and_any_order() {
    assert_eq!(external_shuffled::REGISTRATION.kind, "ExternalFast");
    assert_eq!(external_shuffled::REGISTRATION.name.en, "External");
    let registered = [external_shuffled::REGISTRATION.needs_registry];
    assert_eq!(registered, [false]);
    assert_eq!(
        external_shuffled::REGISTRATION.source.file,
        "face_fields/external_fast.rs"
    );
}
