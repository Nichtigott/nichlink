# nichlink-debug

[简体中文](README.zh-CN.md) | English

`nichlink-debug` is the optional development evidence layer. It provides MIR
and source evidence, live `CallTrace` adapters, data-flow and graph models, and
an inventory collector for declarations that explicitly opt in with
`nichlink::control_object!(collector: debug, ...)`.

The core crate stays free of inventory and linker-section dependencies. A host
that needs collection adds this crate and reads entries with
`nichlink_debug::registrations!(nichlink::RegistrationInfo)`.
