# NichLink architecture notes

NichLink splits the workspace into one pure kernel and thin execution surfaces.
The rule that decides where new code goes:

**Kernel = protocol vocabulary + pure methods (verbs), no execution.**
Anything that reads the filesystem, touches `std::env`, spawns processes,
drives a terminal, or binds to process lifetime belongs in an execution
surface that binds kernel methods to its own context.

## Crates

| Crate | Directory | One-line responsibility |
| --- | --- | --- |
| `nichlink-core` (lib `nichlink`) | `core/` | Kernel: protocol nouns and pure methods |
| `nichlink-build-method` | `build_method/` | Build-time filesystem / `OUT_DIR` orchestration |
| `nichlink-run-method` | `run_method/` | Runtime state instance + trace binding |
| `nichlink-debug-method` | `debug_method/` | Observation evidence surface |
| `nichlink-plugin-host` | `plugin-host/` | Wasm/process plugin host execution |
| `nichlink-studio` | `studio/` | Ratatui authoring/inspection surface |
| `nichlink-mcp` | `mcp/` | AI-agent stdio bridge (currently read-only) |
| `nichlink-cli` | `cli/` | Process glue: argv dispatch, cargo subprocesses |

Host usage: `[dependencies] nichlink-run-method` +
`[build-dependencies] nichlink-build-method`; the crate root calls
`nichlink_run_method::host!();` and the thin `build.rs` calls
`nichlink_build_method::run()`.

## Kernel modules (`core/src/registry_core/`)

- `identity` — stable ids, hashing (SHA-256), namespaces
- `declaration` — face/object vocabulary
- `tree` — registry tree operations, passive recursive registration
- `syntax` — graft syntax, application/graft entry parsing
- `authoring` — face authoring parse/validation/snapshot (fs executor lives in run_method)
- `diagnostic` — `RegistryError`, `RegistrationState`, build diagnostics, topology checks
- `requirements` — capability declarations and requirements
- `plugin` — plugin protocol, slot/channel validation
- `mir` — MIR text/JSONL parsing, call-graph merge
- `source` — lexical source scanning (functions, spans, calls)
- `release` — release-time pruning/plan vocabulary

## Change rules

1. New pure logic goes into a kernel module under `core/src/registry_core/<name>/`
   and is registered in `core/src/registry_core.rs`.
2. Execution surfaces keep historical paths through shim re-exports
   (`pub use nichlink::tree; pub use nichlink::tree::*;`) — do not move public
   API paths without updating the shims.
3. Kernel code must not do I/O, read `std::env`, or depend on process lifetime.
   If it needs such a value, take it as a parameter.
4. Keep crate names, directory names, and lib names consistent
   (`nichlink-<x>` / `<x>/` / `nichlink_<x>`); scaffold templates in
   `build_method/` and CI reference them too.

## Verify

```sh
cargo fmt --all
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
```
