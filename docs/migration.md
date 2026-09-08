# Migration Notes

[简体中文](migration.zh-CN.md) | English

## From the pre-five-crate layout

The registration protocol stays in `nichlink-core`. Build-time discovery,
identity caching, scope calculation, and `StaticPlan` generation are now in
`nichlink-build`; MIR and data-flow evidence are in `nichlink-debug`; the TUI
is in `nichlink-studio`; isolated plugin execution is in `nichlink-plugin-host`.

Applications provide their own root registry through
`Registry::root_for_namespace`. Release builds consume the host application's
generated `StaticPlan`; this workspace does not ship a concrete `root_registry`.

## External declarations

Use `nichlink::external_object!` for declarations owned by another crate. A
linked declaration must include plugin provenance, version, target framework,
and a parent node identity. Development builds discover these declarations when
the host explicitly opts into `collector: debug`; the core crate itself remains
free of inventory. Release applications must retain external faces through a
verified plugin artifact or another explicit application-owned input; release
builds do not create inventory linker sections.

## Studio

Run the standalone package with `cargo run --manifest-path studio/Cargo.toml`.
Use `1` through `4` to select Search, Inspect, Data, or Compare. `watch` is
provided by `nichlink-dev` and rebuilds the child Studio process after source,
Cargo, or plugin catalog changes.

## Plugin host

Install only `VerifiedPluginArtifact` values. Declare Wasm slots at compile
time; enable `process-tools` only for isolated process adapters. Existing
`PluginManifest` flow contracts and lock records remain compatible with the
new host APIs.
# Graft overlay migration

The current graft model is an immutable overlay. A host keeps its original
source tree and declares the external implementation at its entry point:

```rust
let plan = nichlink::graft_plan!(framework,
    cut ["root/canvas"] graft "canvas_fast",
    cut ["root/layout"] full graft "layout_v2",
);
let effective = base.overlay(&plan, &external)?;
```

`cut A graft X` replaces one logical slot and inherits A's children. `cut A
full graft X` replaces the entire subtree rooted at A. A range cut addresses
contiguous siblings. The base and external registries remain unchanged; the
effective view is published only after contract, registration-rule, admission,
and connector validation succeeds.

The pre-overlay `Registry::graft` API and Studio source-copy workflow have been
removed. The public execution path is `GraftPlan` followed by `Registry::overlay`.
