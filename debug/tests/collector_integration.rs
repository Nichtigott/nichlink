const __REGISTRATION_SOURCE: &str = "tests/debug_probe/debug_probe.rs";

nichlink::__nichlink_object!(collector: debug, kind: DebugProbe);

#[test]
#[cfg(debug_assertions)]
fn debug_collector_receives_opted_in_declarations() {
    let registrations = nichlink_debug::registrations!(nichlink::RegistrationInfo)
        .copied()
        .collect::<Vec<_>>();
    assert!(registrations.iter().any(|registration| {
        registration.kind == "DebugProbe"
            && registration.id == NODE_ID
            && registration.namespace == env!("CARGO_PKG_NAME")
    }));
}
