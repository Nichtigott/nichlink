//! The `#[path]` loading model puts the injected registration constants in the
//! wrapper module and expands the declaration one level below them, so this
//! probe mirrors that shape: constants and `__face` are siblings.
//! `#[path]` 载入模型把注入的注册常量放在包装模块里，声明在其下一层展开；
//! 因此本探针复刻同样的形状：常量与 `__face` 是兄弟。

mod debug_probe {
    pub const __REGISTRATION_SOURCE: &str = "tests/debug_probe/debug_probe.rs";
    pub const __REGISTRATION_MODULE_NAME: &str = "debug_probe";

    mod __face {
        nichlink_run_method::__nichlink_object!(collector: debug, kind: DebugProbe);
    }

    pub use __face::*;
}

#[test]
#[cfg(debug_assertions)]
fn debug_collector_receives_opted_in_declarations() {
    let registrations =
        nichlink_debug_method::registrations!(nichlink_run_method::RegistrationInfo)
            .copied()
            .collect::<Vec<_>>();
    assert!(registrations.iter().any(|registration| {
        registration.kind == "DebugProbe"
            && registration.id == debug_probe::NODE_ID
            && registration.namespace == env!("CARGO_PKG_NAME")
    }));
}
