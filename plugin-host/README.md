# nichlink-plugin-host

[简体中文](README.zh-CN.md) | English

`nichlink-plugin-host` verifies and deploys plugin artifacts without exposing
untrusted bytes to the registry. The default `wasm` feature provides fuel-metered
Wasm execution bounded by linear memory, **table elements**, artifact bytes and
the engine's own strict compile limits — a table is a separate eagerly
instantiated array, so the memory ceiling alone does not bound it. Enable
`process-tools` for timeout-controlled process adapters. `HotDeployment` stages a validated graft and publishes it
atomically, leaving the last healthy snapshot visible after a failure.

## Wasm ABI handshake

Wasm plugins may export `nichlink_abi_version() -> i32`. The host compares that
value with `nichlink::plugin::PLUGIN_ABI_VERSION` before it accepts the instance. A
missing export is treated as a legacy plugin and remains loadable for
compatibility; an exported but different version is rejected with
`HostError::Abi`. This makes a breaking ABI change explicit without forcing
all existing plugins to be rebuilt at once.

The ABI version is only the wire-contract version. Resource limits, health
checks, and operation names are still validated independently during load and
call, so a plugin cannot use the handshake to bypass those checks.
