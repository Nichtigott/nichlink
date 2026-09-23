//! 集成测试：真实的 Button 面声明了 `runtime_checks: [NON_EMPTY_TEXT]`，
//! `Registry::health_check` 必须在宿主边界上执行它。
//! Integration test: the real Button face declares
//! `runtime_checks: [NON_EMPTY_TEXT]`, and `Registry::health_check` must enforce
//! it at the host boundary.
//!
//! Why a direct "assert the list is non-empty" test would be wrong: a declared
//! check that never reaches `health_check` is exactly the inert metadata this
//! feature removes. The value has to cross the API: a good label is `Ok`, a
//! blank one fails, and the failure evidence names the check and the face file.
//! 直白写法错在哪：只断言声明的列表非空，正是本特性要移除的“惰性元数据”——检查必须
//! 穿过 API：合格标签 `Ok`，空白标签失败，且失败证据命名该检查与面文件。
//!
//! Boundary: this is the end-to-end twin of the five per-check unit tests in
//! `runtime_checks.rs`; it uses the compiled, registered face, not a synthetic
//! snapshot.
//! 边界：这是 `runtime_checks.rs` 五条单检查单元测试的端到端对照；它使用编译并注册的
//! 真实面，而不是合成快照。
//! Pinned by `a_real_registered_face_reports_its_declared_check`.
//! 由 `a_real_registered_face_reports_its_declared_check` 钉住。

use nichlink_run_method::{Provenance, RuntimeValue};

/// The real registered Button face: a good label passes, a blank one fails with
/// evidence that names the check and the face's source file.
/// 真实注册的 Button 面：合格标签通过，空白标签失败，证据命名该检查与面的源码文件。
#[test]
fn a_real_registered_face_reports_its_declared_check() {
    let registry = control_button::base_registry();
    let node = control_button::control::object::button::NODE_ID;
    let provenance = |value: &str| Provenance::default().push(node, "Button", "paint", value);

    let good = RuntimeValue::text("OK", provenance("OK"));
    assert!(
        registry.health_check(node, &good, Vec::new()).is_ok(),
        "a non-blank label satisfies the declared NON_EMPTY_TEXT check"
    );

    let blank = RuntimeValue::text("   ", provenance("   "));
    let error = registry
        .health_check(node, &blank, Vec::new())
        .expect_err("a whitespace-only label is rejected");

    // The aggregate's four facts point back at the failing face, and `children()`
    // carries one entry per failed check instead of forcing a `Display` scrape.
    // 聚合错误的四项事实指回失败的面；`children()` 按失败检查各带一条，宿主无需解析
    // `Display`。
    assert_eq!(error.node(), node);
    assert_eq!(error.path(), "root/control/button");
    assert_eq!(error.message(), "runtime health check failed");
    assert_eq!(error.children().len(), 1);
    assert!(
        error.children()[0].message().contains("non_empty_text"),
        "the child names the check that failed: {}",
        error.children()[0].message()
    );

    let rendered = format!("{error}");
    assert!(
        rendered.contains("non_empty_text"),
        "the rendered aggregate names the check: {rendered}"
    );
    assert!(
        rendered.contains("control/object/button/button.rs"),
        "the rendered aggregate names the face file: {rendered}"
    );
    assert!(
        error
            .source()
            .to_string()
            .contains("control/object/button/button.rs"),
        "the source reader names the face file: {}",
        error.source()
    );
}
