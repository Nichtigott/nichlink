# nichlink-build-method

[简体中文](README.zh-CN.md) | English

`nichlink-build-method` is the build_method execution surface: filesystem and
`OUT_DIR` orchestration. It discovers registration faces from the host crate's
folder layout, feeds them to the kernel validators (parent topology, capability
requirements, contracts, stable identities), maintains incremental identity
caches, and renders the passive `StaticPlan` consumed by release builds. The
validation rules and diagnostic models themselves live in `nichlink-core`; this
crate only reads files and writes artifacts.

Call `nichlink_build_method::run()` from the host crate's `build.rs`. The build
process uses the host package's `CARGO_PKG_NAME` as the identity namespace,
matching the identity captured by the declaration macros. Outside Cargo, the
CLI pins the namespace explicitly via `run_for(manifest, out_dir, package)` —
no environment mutation needed.

The rendered plan lands in Cargo's `OUT_DIR`. The host crate root pulls it in
with `nichlink_run_method::host!();` — equivalently
`include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))` — making
`builtin_static_plan()` and the per-level `{name}_object!` aliases available
crate-wide.

The optional host entry is parsed as Rust syntax, not text search:

```rust
nichlink::application!(entry = crate::main);
```

There must be exactly one declaration, and its first module must resolve to a
source file under `src/` (including `src/bin/`). Malformed, duplicate, or
unresolvable entries fail the build with the declaration location.

`SourceScope` pruning keeps the community surface alive: graft cut targets and
plugin-declaring faces are forced liveness roots, so a minimal tree never
prunes a slot that a graft plan replaces later.
