//! The declaration macro derives `source` from `file!()`, so this probe no
//! longer injects -- and no longer has to keep in sync -- a sibling
//! `__REGISTRATION_SOURCE` constant.
//! 声明宏从 `file!()` 推导 `source`，因此本探针不再注入、也无需同步一个同级的
//! `__REGISTRATION_SOURCE` 常量。

nichlink_run_method::__nichlink_object!(collector: debug, kind: DebugProbe);

#[test]
#[cfg(debug_assertions)]
fn debug_collector_receives_opted_in_declarations() {
    let registrations =
        nichlink_debug_method::registrations!(nichlink_run_method::RegistrationInfo)
            .copied()
            .collect::<Vec<_>>();
    let probe = registrations
        .iter()
        .find(|registration| registration.kind == "DebugProbe")
        .expect("the opted-in declaration is collected");
    assert_eq!(probe.id, NODE_ID);
    assert_eq!(probe.namespace, env!("CARGO_PKG_NAME"));
    // A declaration outside `src/` keeps the path cargo recorded instead of
    // failing the manifest strip, so it still has a stable, unique identity.
    // 不在 `src/` 下的声明保留 cargo 记录的路径，而不是让 manifest 剥离失败，
    // 因此它仍有一个稳定且唯一的身份。
    assert_eq!(probe.source.file, file!());
    // Cargo records this path with the platform separator, so compare the
    // normalized form rather than assuming `/`.
    // cargo 记录该路径时使用平台分隔符，因此比较归一化后的形式，而不是假定 `/`。
    assert!(
        probe
            .source
            .file
            .replace('\\', "/")
            .ends_with("tests/collector_integration.rs")
    );
}
