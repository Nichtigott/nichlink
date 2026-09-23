# nichlink-studio

[简体中文](README.zh-CN.md) | English

`nichlink-studio` is the standalone Ratatui authoring and diagnostics UI.
End users normally reach it through the unified CLI (`nichlink studio` from
`nichlink-cli`); this crate also ships the `nichlink-studio` binary for direct
use, plus a workspace-only `nichlink-dev` rebuild supervisor behind the
non-default `dev-supervisor` feature (it rebuilds from a checkout, so an
installed copy could never work — hence `cargo install` leaves it out).

Run it with `cargo run --manifest-path studio/Cargo.toml`. Set
`NICH_LINK_PACKAGE_ROOT` to the project directory whose `src/` and plugin
catalog should be read or edited. Set `NICH_LINK_HOST_MANIFEST` when MIR
inspection and release builds should target a manifest other than
`<package-root>/Cargo.toml`. `NICH_LINK_NAMESPACE` isolates authored snapshots
when several libraries share one process.

`g` opens the external-graft screen for the selected face. It writes
`.nichlink/external-grafts/<selector>/graft.plan`, never host source, and it
keeps the three graft layers apart: the `static_graft_plan!` **declaration** the
build reads (the screen shows whether the selected slot is declared and prints
the clause to paste), the `Registry::overlay` **application** the host performs
at runtime, and the plan **record** the screen lists, opens, re-scopes with `f`,
and moves to `.nichlink/trash/external-grafts/` with `d`. Writing a record for a
slot no declaration names is allowed — declaring it afterwards is a legitimate
order — but the status line warns that the release prunes that slot and the
runtime skips the record as `UnkeptSlot`, and repeats the clause to add.
