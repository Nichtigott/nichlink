# nichlink-plugin-host

[简体中文](README.zh-CN.md) | English

`nichlink-plugin-host` verifies and deploys plugin artifacts without exposing
untrusted bytes to the registry. The default `wasm` feature provides fuel-metered
Wasm execution bounded by linear memory, **table elements**, artifact bytes and
the engine's own strict compile limits — a table is a separate eagerly
instantiated array, so the memory ceiling alone does not bound it. Enable
`process-tools` for timeout-controlled process adapters. `HotDeployment` stages a validated graft and publishes it
atomically, leaving the last healthy snapshot visible after a failure.

## Admission: from the host's lock to a loadable artifact

`PluginAdmission` is the host-side path from the lock a host writes to an artifact that
can be installed. It reads `<package_root>/.nichlink/plugins/official.lock` and
`user.lock` — a missing file is an empty catalogue, not an error — selects the manifest
against that catalogue, and then verifies: an official artifact must pass
`verify_signed` under a configured trust root, while a user artifact takes the digest
path. `admit` returns the verified artifact, `lane_for` names the lane the assurance it
earned buys, and `install` (Wasm) or `load_process` (process) does admission and loading
in one step.

A lock line is `source|framework|package|version|crate|checksum|mode`, optionally
followed by `|signature|key-fingerprint|revocation-list`. The three trailing fields are
expectations: a record that carries one pins it, and a record written without one leaves
it to the signature check, so a lock written by a host's own plugin UI admits a signed
official plugin. An official plugin that is not in the lock is refused before any
verifier is asked, a revoked version is refused before the signature, a host with no
trust root gets `MissingOfficialKey` rather than a silent downgrade to checksums, and the
signature covers the registration that travels with the plugin bytes.

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
