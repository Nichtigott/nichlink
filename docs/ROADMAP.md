# NichLink Roadmap

[简体中文](ROADMAP.zh-CN.md) | English

This roadmap covers NichLink itself. NichUI product work, funding, and community operations live in separate notes and are not release milestones for the registry protocol.

## Current line: 0.1 early production

- [x] Passive recursive `Registry` with atomic batch registration and grafts.
- [x] Admission, structural rules, and input/output contracts.
- [x] Folder-backed discovery, identity cache, coarse scope, and `StaticPlan`.
- [x] Runtime tracing modes: `off`, `errors-only`, and `full`.
- [x] Studio search/inspect/data/compare views and source navigation.
- [x] Read-only MCP bridge and verified Wasm/process plugin adapters.
- [x] Cross-platform CI, scale audits, symbol audits, and package checks.
- [x] Kernel/execution-surface split: one pure kernel (`nichlink-core`) holding
  protocol vocabulary and pure methods, with thin surfaces
  (`nichlink-build-method`, `nichlink-run-method`, `nichlink-debug-method`,
  `nichlink-studio`, `nichlink-mcp`, `nichlink-cli`, `nichlink-plugin-host`)
  binding those methods to their own contexts.

0.1 is suitable for experiments and selected internal production use. It does not claim complete static analysis for arbitrary Rust programs.

## Next engineering milestones

### 1. Stable evidence interchange

Version a compact call-edge artifact containing symbol, span, owning node, evidence kind, and target candidates. Stale or malformed evidence must force a conservative full-tree fallback.

### 2. Optional compiler adapter

Ship a separate, nightly-only `rustc_driver`/MIR analyzer. Stable users keep the source-scanner path. The adapter emits candidates and uncertainty, not unverifiable certainty.

### 3. Monomorphization and indirect calls

Track generic instances and methods from host roots. Represent `dyn Trait`, function pointers, FFI, and address escapes as candidate sets. A live `CallTrace` edge may promote a candidate, never silently delete an unresolved target.

### 4. Build and release proof

Feed the verified scope to normal rustc/LLVM/ThinLTO, then compare defined symbols and artifact size. Publish reproducible debug/release and 10k/100k-node benchmarks.

### 5. Protocol stability

Freeze the first face-field subset, publish schema migrations, and keep `FaceManifest`, contracts, admission, graft, and diagnostic formats versioned and independently testable.

### 6. Evidence-aware tooling

Connect MCP and Studio to Registry snapshots, contracts, diagnostics, and live provenance. Keep redaction, trust policy, and evidence level visible in every result.

### 7. Validated MCP authoring tools

Grow the MCP bridge from read-only queries to validated `add` / `edit` /
`graft` tools. Every write goes through the same kernel admission, contract,
and topology validation the build surface uses, and reports provenance back to
the caller.

## Explicit boundaries

No ordinary dependency can promise complete resolution of every dynamic Rust call or recovery of every optimized uninstrumented local variable. Process plugins are not operating-system sandboxes. A dependency crate cannot read a consuming application's `main.rs` through its own `build.rs`.

Each milestone is complete only when its tests, fallback behavior, and benchmark evidence are checked into the repository.
