# Migration Notes

[简体中文](migration.zh-CN.md) | English

## From the pre-five-crate layout

The registration protocol stays in `nichlink-core`. Build-time discovery,
identity caching, scope calculation, and `StaticPlan` generation are now in
`nichlink-build-method`; MIR and data-flow evidence are in `nichlink-debug-method`; the TUI
is in `nichlink-studio`; isolated plugin execution is in `nichlink-plugin-host`.
The unified command-line entry (`nichlink new/check/build/studio/mcp`) is in `nichlink-cli`.

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

## Registration faces load in place

Build-time rendering used to copy every face into
`OUT_DIR/registration_sources` and `include!` that copy, so the module the
compiler and editor tooling resolved was a build artefact rather than the file
being edited. Faces are now loaded as real modules:

- a **leaf face** (it owns no child registries) is declared as
  `#[path = "..."] pub mod <name>;`, so its public module path and its
  `type_name` are unchanged;
- a **face that owns child registries** must also be their container, so it
  loads into a same-named child module and re-exports
  (`mod <name>; pub use <name>::*;`). Only that case gains a module-path
  segment, for example `app::control::control::Control`.

Two consequences:

- A face file stays an ordinary module file, so it may keep opening with `//!`
  or any other inner attribute. `include!` expansion cannot introduce them,
  which is why the previous design had to rewrite `//!` into `//` in its copy.
- Declaration macros now derive `source` from `file!()` and the registry name
  from `module_path!()`, so the generated tree no longer injects
  `__REGISTRATION_SOURCE` / `__REGISTRATION_MODULE_NAME`. A declaration outside
  `src/` (an integration test, for example) keeps the path cargo recorded
  instead of failing the manifest strip.

Identities are unchanged: `source`, `registry_name`, and therefore `NodeId`,
registry paths, and every graft selector keep the same values.

A graft plan declared with `static_graft_plan!` is now captured from the host
entry whether or not the host also declares `application!(entry = ...)`,
matching how `SourceScope` already resolved the entry. Previously the plan was
silently dropped without the `application!` declaration.

See `examples/control-button/` for the README Control/Button tree as a real
host, together with `examples/control-button-graft/` for an out-of-project
implementation that grafts over it.

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
