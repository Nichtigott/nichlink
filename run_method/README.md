# nichlink-run-method

[简体中文](README.zh-CN.md) | English

`nichlink-run-method` is the run_method execution surface of NichLink: runtime
state instances plus trace bindings. The protocol itself — the `Registry`
tree, transactions, graft validation, plugin policy, and parsers — lives in
the kernel (`nichlink-core`); this crate binds it to the process lifetime.

What stays here:

- `CallTrace` frame stacks, locals, and data edges (`trace_mode_from_env`
  reads `NICH_LINK_TRACE`; the enum and its parser are kernel types)
- Declaration macros: `host!`, `application!`, `static_graft_plan!`,
  `graft_plan!`, `trace_call!`, and the generated `*_object!` family
- The ungated graft-record loader and overlay entry: `apply_recorded_grafts`,
  `load_graft_records`, `load_graft_record`, `graft_record_root`, `LoadedGraft`,
  `GraftOverlay`, and the re-exported `RecordReport`/`RecordedGraft`/
  `ResolvedRecord`. It is deliberately outside `authoring`, so a host reads
  `.nichlink/external-grafts/` without pulling in `syn`; the precedence policy
  and reports are in [`docs/graft.md`](https://github.com/Nichtigott/nichlink/blob/main/docs/graft.md). `apply_recorded_grafts`
  prints one `warning:`/`note:` line per report to stderr before returning, so a
  record that is skipped (a graft that silently never happens) cannot go
  unnoticed; the same items stay in `GraftOverlay` for a host that routes evidence
  into its own log. A plan that cannot be parsed, a directory that disagrees with
  its plan's `graft`, and a contradictory identity-vs-path record are `Err`
  instead: none of them has a legitimate reading.
- The authoring **executor** (feature `authoring`): applies file plans to the
  host's `src/` tree. The pure plan/render logic and the `FACE` field
  dictionary live in the kernel `authoring` module
- `call_report` rendering over a live trace

A host crate depends on this crate and calls `nichlink_run_method::host!();`
once at the crate root; the build-time half is `nichlink-build-method`.

## Trace artifacts

A trace a host recorded can leave the process as one file that another process
reads back. `write_trace_artifact` and `read_trace_artifact` are the two ends, and
`trace_artifact_path` is the single path decision both ask for, so an override
cannot move the file for one end and not the other:

```rust
use std::path::Path;

use nichlink_run_method::{
    CallTrace, read_trace_artifact, trace_artifact_path, write_trace_artifact,
};

fn record(trace: &CallTrace, package_root: &Path) -> Result<(), String> {
    let path = trace_artifact_path(package_root);
    // The namespace is what the declaration macros stamped this crate with.
    // A process cannot read `CARGO_PKG_NAME` back, so the host passes it.
    write_trace_artifact(trace, &path, env!("CARGO_PKG_NAME"))?;
    let (artifact, rebuilt) = read_trace_artifact(&path)?;
    eprintln!("{}: {} local(s)", artifact.namespace, rebuilt.locals().len());
    Ok(())
}
```

`NICH_LINK_TRACE_FILE` overrides the path for both ends — absolute, or relative
to the package root — and otherwise it is
`<package_root>/.nichlink/traces/nichlink.trace`; the directory is created by the
writer. Write only when `NICH_LINK_TRACE` selected a collecting mode: under `off`
the trace is empty, and an empty artifact is worse than none. The document is one
recording, not the newest of a series — each write replaces the file — and a
reader refuses another version, an unknown key, or an identity that disagrees
with what it resolved. The format, the identity checks, and the reasoning behind
each are in
[`docs/design-trace-ingest.md`](https://github.com/Nichtigott/nichlink/blob/main/docs/design-trace-ingest.md).

## Runtime checks

`runtime_checks: [...]` on a face is a host API, not an automatic hook: the
kernel never observes a value, so the host owns the boundary. Build the
`RuntimeValue` and call `Registry::health_check` with the face's `NodeId` where
the value crosses into a plugin or consumer. Pass `Vec::new()` as `call_path`
when no trace is active (`CallTrace::current_path()` when one is). On failure the
error aggregates one child per failed check. Five readers expose the facts the
constructor was given (`core/src/registry_core/diagnostic/error.rs`):

| Reader | Returns | Population |
| --- | --- | --- |
| `node()` | The `NodeId` of the face the failure is about | always set |
| `path()` | The logical registration path the failure was reported at | always set; a phase with no live path writes a placeholder instead |
| `source()` | `&DiagnosticSource`, the declaration source the failure points back to | always set; a phase with no declaration to point at synthesizes a `<…>` location at line 0, column 0 |
| `message()` | The human-readable failure message | always set |
| `children()` | The aggregated child failures, one per failed sub-check | empty unless the phase aggregated sub-failures |

Placeholder paths are `<unknown:{node}>`
(`core/src/registry_core/tree/inspection/inspection.rs:74`),
`<missing-parent:…>/…`
(`core/src/registry_core/tree/transaction/transaction.rs:167`,
`core/src/registry_core/tree/graft_ops/graft_ops.rs:146`), and `<edited>/…`
(`core/src/registry_core/tree/graft_ops/graft_ops.rs:186`); synthesized sources
are `<runtime>` (`inspection.rs:75`), `<registry-connector>`
(`core/src/registry_core/tree/connector/connector.rs:205`),
`<owned-snapshot-batch>`
(`core/src/registry_core/tree/transaction/transaction.rs:83`), `<migration>`
(`core/src/registry_core/tree/graft_ops/graft_ops.rs:54`), and `<graft>`
(`core/src/registry_core/tree/graft_ops/graft_ops.rs:294`,
`core/src/registry_core/tree/graft_ops/reconcile.rs:166`).

Two rules for reading a `health_check` failure:

1. The top-level `message()` is the fixed aggregate sentence `runtime health
   check failed`
   (`core/src/registry_core/tree/inspection/inspection.rs:110`); it does not
   name the failing check. Other aggregating phases keep the same shape:
   `registration connector rejected (N face(s))`
   (`core/src/registry_core/tree/connector/connector.rs:211`) and `snapshot
   batch rejected (N error(s))`
   (`core/src/registry_core/tree/transaction/transaction.rs:89`).
2. The failing check's own name and text are in `children()[0].message()`, built
   as ``check `<name>`: <message>`` (`inspection.rs:95`). `Display` renders the
   aggregate plus every child, which is why `eprintln!("{error}")` in the
   example below prints both.

```rust
use nichlink_run_method::{Provenance, Registry, RuntimeValue};

fn validate(registry: &Registry, node: nichlink_run_method::NodeId, label: &str) {
    let value = RuntimeValue::text(
        label,
        Provenance::default().push(node, "Button", "paint", label),
    );
    if let Err(error) = registry.health_check(node, &value, Vec::new()) {
        eprintln!("{error}"); // one child per failed check
    }
}
```

A runnable version lives at
[`examples/control-button/examples/health_check.rs`](https://github.com/Nichtigott/nichlink/blob/main/examples/control-button/examples/health_check.rs):
`cargo run -p nichlink-example-control-button --example health_check`.

