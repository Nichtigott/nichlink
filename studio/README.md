# nichlink-studio

[简体中文](README.zh-CN.md) | English

`nichlink-studio` is the standalone Ratatui authoring and diagnostics UI.
End users normally reach it through the unified CLI (`nichlink studio` from
`nichlink-cli`); this crate also ships the `nichlink-studio` binary for direct
use and the `nichlink-dev` rebuild supervisor.

Run it with `cargo run --manifest-path studio/Cargo.toml`. Set
`NICH_LINK_PACKAGE_ROOT` to the project directory whose `src/` and plugin
catalog should be read or edited. Set `NICH_LINK_HOST_MANIFEST` when MIR
inspection and release builds should target a manifest other than
`<package-root>/Cargo.toml`. `NICH_LINK_NAMESPACE` isolates authored snapshots
when several libraries share one process.
