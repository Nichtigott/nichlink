//! The kind-only face shape and the `registry_name` it derives.
//! 只有 kind 的注册面形态，以及它推导出的 `registry_name`。

/// A face declared with `collector` and `kind` and nothing else: no `handle`,
/// no explicit `registry_name`, no `needs_registry`.
/// 只写 `collector` 与 `kind`、别无其它字段的注册面：没有 `handle`，没有显式
/// `registry_name`，也没有 `needs_registry`。
mod kind_only {
    nichlink_run_method::__control_object! {
        collector: development,
        kind: KindOnlyFace,
    }
}

/// `{ collector, kind }` is the smallest reachable face spelling. B1 deleted the
/// arm that used to catch this shape, so it now falls through to the arm that
/// defaults every post-`kind` field; this test pins the `registry_name` that
/// surviving path produces. It is derived from the declaration's own module path
/// (`last_path_segment(module_path!())`), so it is the *module* name here, not
/// the kind. That is the pre-1.0 behaviour and is intentionally unchanged: the
/// point of the test is equivalence, so if a later batch makes the value
/// kind-derived, this assertion is the one that must be updated deliberately.
/// `{ collector, kind }` 是可达的最小注册面写法。B1 删除了过去接住这一形态的 arm，
/// 它现在落到把 `kind` 之后每个字段取默认值的那个 arm；本测试钉住这条存活路径产出的
/// `registry_name`。它由声明自身所在的模块路径推导
/// （`last_path_segment(module_path!())`），因此这里是**模块**名而不是 kind。这是
/// 1.0 之前的行为，有意保持不变：本测试的目的就是等价性，若后续批次改成由 kind 推导，
/// 需要显式修改的正是这条断言。
#[test]
fn a_kind_only_face_names_its_registry_after_its_module() {
    assert_eq!(kind_only::REGISTRATION.kind, "KindOnlyFace");
    assert_eq!(kind_only::REGISTRATION.registry_name, "kind_only");
    assert_ne!(
        kind_only::REGISTRATION.registry_name,
        kind_only::REGISTRATION.kind,
        "the derived name is the module segment, not the kind"
    );
}
