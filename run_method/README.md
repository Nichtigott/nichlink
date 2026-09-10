# nichlink-run-method

[简体中文](README.zh-CN.md) | English

`nichlink-run-method` is the run_method execution surface of NichLink: runtime
state instances plus trace bindings. The protocol itself — the `Registry`
tree, transactions, graft validation, plugin policy, and parsers — lives in
the kernel (`nichlink-core`); this crate binds it to the process lifetime.

What stays here:

- `CallTrace` frame stacks, locals, and data edges (`TraceMode::from_env`
  reads `NICH_LINK_TRACE`; the enum and its parser are kernel types)
- Declaration macros: `host!`, `application!`, `static_graft_plan!`,
  `graft_plan!`, `trace_call!`, and the generated `*_object!` family
- The authoring **executor** (feature `authoring`): applies file plans to the
  host's `src/` tree. The pure plan/render logic and the `FACE` field
  dictionary live in the kernel `authoring` module
- `call_report` rendering over a live trace

A host crate depends on this crate and calls `nichlink_run_method::host!();`
once at the crate root; the build-time half is `nichlink-build-method`.
