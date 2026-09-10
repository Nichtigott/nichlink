# nichlink-core

[简体中文](README.zh-CN.md) | English

The kernel of NichLink: protocol nouns plus pure methods — no I/O, no
environment bindings. Everything here is testable in memory and reusable from
any execution surface.

Modules under `registry_core`:

| Module | Contents |
|--------|----------|
| `identity` | `NodeId` compile-time SHA-256 identities, `StableFaceId`, zero-dependency SHA-256, hex codecs |
| `declaration` | `RegistrationInfo`/`RegistrationSnapshot`, rules, admission, flow contracts, plugin manifests, runtime checks, call-site and evidence types (`CallSite`, `CallEdge`, `EvidenceKind`, `TraceMode`) |
| `tree` | The one `Registry` type: recursive same-type registration, atomic page-copy transactions, queries, graft overlay/replacement validation, inspection |
| `plugin` | Plugin policy decisions, trust/verification, artifact/catalog handling, graft command parsing, slot validation |
| `mir` | Static MIR candidate parsing (text and JSONL) and evidence-aware `merge_call_relations` (live edges win over MIR candidates) |
| `requirements` | Capability requirement analysis over the declaration graph (ancestor-provider lookup) |
| `source` | Rust source mini-lexer: function index, direct-call extraction, registration kinds — shared by tooling surfaces |
| `release` | Zero-allocation release topology (`StaticPlan`) and compile-time registration assertions |
| `diagnostic` | `RegistryError` tree, `BuildDiagnostic` model, face topology validation |
| `syntax` (feature `syntax`) | Registration face parser, application/graft entry discovery |
| `authoring` (feature `syntax`) | Pure authoring plan/render helpers, name validation, the `FACE` field dictionary |

The boundary rule: anything with no I/O and no `std::env`/time/process binding
belongs here; execution surfaces (`nichlink-build-method`, `nichlink-run-method`,
`nichlink-debug-method`, `nichlink-plugin-host`, studio, mcp, cli) bind these
methods to their contexts.
