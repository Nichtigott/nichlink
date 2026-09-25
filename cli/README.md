# nichlink-cli

`nichlink-cli` is the single entry point for NichLink tooling.

```sh
cargo install --git https://github.com/Nichtigott/nichlink nichlink-cli
nichlink new my-app
cd my-app && nichlink studio
```

## Commands

| Command | Action |
| --- | --- |
| `nichlink new <name> [--lib] [--path <workspace> \| --git <url>]` | Scaffold a NichLink host project in `./<name>` |
| `nichlink check [path] [--json]` | Run the registration discovery and validation pass without compiling |
| `nichlink build [path] [cargo options]` | Validate the registration tree, then run `cargo build` |
| `nichlink explain <node-id \| logical/path> [--path <dir>] [--json]` | Report one node's identity, build scope, pruning state, and the declared cuts that name it |
| `nichlink explain --overlay [--path <dir>] [--json]` | Render the static overlay projection of every slot and plan (not a live tree) |
| `nichlink grafts [path] [--json]` | List `.nichlink/external-grafts/*/graft.plan`, their targets, and whether the host entry declares the slot |
| `nichlink studio` | Launch the Studio TUI for the current project |
| `nichlink mcp` | Run the read-only MCP stdio bridge |

Dependency source is detected automatically: a CLI running from a NichLink
checkout writes path dependencies; an installed CLI writes Git dependencies
(with a version floor, so Cargo resolves crates.io once published). Override
with `--path` or `--git`.

The `cargo-nichlink` binary in the same package registers the plugin form:
`cargo nichlink studio` is equivalent to `nichlink studio`.

The library target (`nichlink_cli`) holds the command dispatch so other
binaries can reuse it.

## `nichlink check --json`

`nichlink check` owns this contract (`cli/src/commands/check.rs`). With `--json`:

- Success writes exactly one JSON document to stdout and exits 0.
- Failure writes that same document, carrying every diagnostic, to stdout; the
  command then returns `Err`, so the process still exits non-zero (`nichlink`
  prints `registration check failed (N diagnostic(s); JSON on stdout)` to stderr
  and exits 1).

The document shape comes from `BuildDiagnostics::to_json`
(`core/src/registry_core/diagnostic/build.rs`):

```json
{"schema":"nichlink.build-diagnostics/1","count":N,"diagnostics":[...]}
```

Every diagnostic object always carries all eleven keys, in this order: `phase`,
`branch`, `node`, `source`, `line`, `function`, `field`, `expected`, `actual`,
`provider`, `message`. A reader never has to tell "absent" apart from "empty":
`line` is `0` when the diagnostic has no line. The human renderer
(`render_build_item`, same file) omits empty fields instead, so an empty JSON
field means "not applicable to this diagnostic", not "missing key". (`branch` is
the exception: the renderer prints `branch=<unknown>` when it is empty.)

`diagnostics` follows `BuildDiagnostics::iter`, the same order the human text
uses, and byte-identical duplicates are collapsed, so `count` is the
deduplicated count. `phase` is a stable machine string. The human renderer maps
`requirements` to `requirements / 注册需求`, `contract` to `contract / 注册合同`,
`stable-identity` to `stable identity / 稳定标识`, and `static-plan` to
`static plan / 静态计划`, and passes any other value through unchanged.

### Fields by phase

| `phase` | Constructed by | `branch` | `node` | `source` | `line` | `function` | `field` | `expected` | `actual` | `provider` | `message` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `requirements` | `core` `requirements::missing` | always | always | always | always | always | always | always | never | only when an ancestor provides the capability under a different kind | always |
| `contract` | `build_method` `contracts::check_parent_rule` | never | always | always | always | always | always | always | only for the output-contract mismatch | never | always |
| `stable-identity` | `build_method` `validation::collect_stable_names` | never | never | always | always | never | always | always | always | never | always |
| `parent-macro` | `build_method` `validation::collect_parent_macro_errors` | never | never | always | always | never | always | always | always | never | always |
| `static-plan` | `build_method` `static_plan::collect_static_faces`, `graft_plan_check::undeclared_plan_errors`; `core` `topology::validate_face_topology` | never | never | always | always `0` | never | topology checks only | missing-parent and no-registry only | topology checks only | never | always |
| `face-cfg` | `build_method` `static_plan::collect_static_faces` | never | never | always | always | never | never | never | never | never | always |
| `out-dir` | `build_method` `check_for` | never | never | never | always `0` | never | never | never | never | never | always |
| `face-layout` | `build_method` `validation::unplaced_face_errors` (the phase is chosen by `discovery::record_unplaced`) | never | never | always | `0` when the file has no position | never | never | never | never | never | always |
| `face-syntax` | `build_method` `validation::collect_face_syntax_errors` | never | never | always | `0` when the parse error has no position | never | never | never | never | never | always |
| `entry` | `build_method` `entry::application_entry_source`, `entry::resolve_host_entry_reporting`, `entry::rejected_entry` | never | never | always | `0` when the failure is about the package rather than a line | never | never | never | never | never | always |
| `scope` | `build_method` `scope::from_raw` | never | never | never | always `0` | never | never | never | never | never | always |
| `graft-entry` | `build_method` `scope::auto_from_entry_reporting` | never | never | never | always `0` | never | never | never | never | never | always |

One `static-plan` diagnostic comes from the graft-plan cross-check
(`build_method/src/graft_plan_check.rs`): it points `source` at the offending
`.nichlink/external-grafts/<selector>/graft.plan` and carries the paste-ready
`static_graft_plan!` clause in `message`.

The three `static-plan` topology checks are `parent node is missing`, `parent
does not own a registry`, and `parent cycle detected`; all three set
`field=parent` and `actual`, the first two also set `expected`, and the cycle
check does not. The fourth `static-plan` diagnostic, `parent declaration cannot
be resolved` (`static_plan::collect_static_faces`), sets only `phase`, `source`,
`line=0`, and `message`.

简体中文见 [README.zh-CN.md](README.zh-CN.md)。
