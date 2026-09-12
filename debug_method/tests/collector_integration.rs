const __REGISTRATION_SOURCE: &str = "tests/debug_probe/debug_probe.rs";

#[nichlink_run_method::object(
    collector = debug,
    parent = ::nichlink_run_method::registry_core::root_node_id(env!("CARGO_PKG_NAME")),
    registry_name = debug_probe
)]
pub struct DebugProbe;

#[test]
#[cfg(debug_assertions)]
fn debug_collector_receives_opted_in_declarations() {
    let registrations =
        nichlink_debug_method::registrations!(nichlink_run_method::RegistrationInfo)
            .copied()
            .collect::<Vec<_>>();
    assert!(registrations.iter().any(|registration| {
        registration.kind == "DebugProbe"
            && registration.id == NODE_ID
            && registration.namespace == env!("CARGO_PKG_NAME")
    }));
}
