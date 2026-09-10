# nichlink-debug-method

[简体中文](README.zh-CN.md) | English

`nichlink-debug-method` is the observation evidence surface. Static analysis
itself — MIR text/JSONL parsing, source lexing, and evidence merging (live
edges win over MIR candidates) — lives in the kernel (`nichlink-core` `mir`
and `source` modules). This crate binds it to the toolchain and the process:

- `collector`: the inventory linker-section collection for declarations that
  opt in with the generated parent macro, for example
  `crate::workspace_object!(collector: debug, ...)`. The kernel stays free of
  inventory and linker-section dependencies.
- `mir::UnifiedCallGraph`: merges static MIR candidates with one live
  `CallTrace` run.
- `adapters`: `tracing` span creation and the petgraph-backed `CallGraph`
  topology view (DOT export).

A host that needs collection adds this crate and reads entries with
`nichlink_debug_method::registrations!(nichlink::RegistrationInfo)`.
