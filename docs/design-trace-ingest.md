# Design: real trace ingest for Studio
# 设计：Studio 的真实 trace ingest

Design only; no `.rs` file is edited and no git command is run. Citations are
`path:line`. Headings are bilingual and the body is English, matching
`docs/design-graft-record-and-health-check.md`; an English-only body is
deliberate (same choice as that file), not an oversight.

**Status (2026-09-26) / 状态.** Slice 1 is **built**: the pure document
(`run_method/src/runtime/trace/artifact/artifact.rs`), its parser and interner
(`artifact/parse.rs`), and the file half (`artifact/io.rs`) — `TraceArtifact`
with `from_trace`/`render`/`parse`/`into_trace`, `trace_artifact_path`,
`write_trace_artifact`, `read_trace_artifact` — plus `TRACE_DIR`/`TRACE_FILE`/
`TRACE_FILE_ENV` in the kernel lexicon, re-exported at the `run_method` crate
root. The Studio loader (§3.4–3.5) is slice 2 and remains unbuilt; §3.6's
"no `.rs` file is edited" sentence describes the design's own moment, and this
paragraph is the correction.
The citations below were measured at that moment; re-measured against `0.1.1`
they had drifted, so a reader should take the names as the anchor and the line
numbers as approximate: `TraceMode::parse` 121→142, `CallSite.function` 11→15,
`LocalKind::label` 14–22→25, `Observation::label` 13–18→19, `NodeId`
`Display`/`FromStr` 109/150→134/178, `local_value.rs` value 20→28, `edges.rs`
label/source 19,20→25,28, `render_tree` 284→296, `matching_locals` 164–200→191,
`GRAFT_PLAN_FILE` 76–80→94, and the lexicon published-value test now lives in
`conventions`-checked `lexicon_tests.rs`. `filesystem.rs::atomic_write` 18–27
held.
**状态（2026-09-26）。** 第一片**已建成**：纯文档、解析器与驻留表、文件一半，外加内核词典里的
三个常量，并在 `run_method` 根部重导出。Studio 的加载方（§3.4–3.5）是第二片，尚未建。下面引用
的行号是设计当时实测的；对着 `0.1.1` 重测已经漂移，因此读者应把名字当锚、把行号当近似值。

**Verdict / 结论.** Full ingest is a **1.x** feature. 1.0 should keep the demo
label it already has (`docs/roadmap-1.0.md:61`) and add nothing but a one-line
regression test; the concrete artifact API below is the 1.x target, and the 1.0
deliverable is only the honest label plus the identity rules that must hold
before the label may ever say plain `LIVE`.
完整的 ingest 是 **1.x** 特性；1.0 保留现有示例标注，只补一条回归测试。

## 1. What is missing between a recorded `CallTrace` and the DATA panel
## 1. 从已记录的 `CallTrace` 到 DATA 面板，缺的是什么

### 1.1 The trace is an in-process, owner-held value
`CallTrace` (`run_method/src/runtime/trace/call_trace.rs:37-53`) is a plain
struct whose every field is private (`pub(super)`), holding `Vec<FrameRecord>`,
three `BTreeMap` indexes, `Vec<LocalValue>`, `Vec<DataEdge>`, and two id
counters. It is created by value (`:86-112`) and moved; there is no global,
`static`, `thread_local`, `OnceLock`, or accessor that publishes a trace
(grep for `CallTrace` outside `target/` finds only owners and `&CallTrace`
borrowers). `debug_method` only borrows one: `CallGraph::from_trace(&CallTrace)`
(`debug_method/src/adapters.rs:35`) and `UnifiedCallGraph::new(&MirGraph,
&CallTrace)` (`debug_method/src/mir.rs:26`). So the trace is **owned
in-process**, never per-process and never shared.
（每个字段都是私有 `pub(super)`、由创建者按值持有，没有任何全局或访问器发布它。）

### 1.2 No writer, no format, no reader, no serde
Nothing serializes a trace. `render_tree` (`frames.rs:284`),
`render_provenance` (`edges.rs:242`), `render_data_flow` (`edges.rs:336`) and
`render_call_report_for_trace` (`run_method/src/call_report/call_report.rs:29`)
turn a trace into human text, but **have no callers anywhere** — not in `studio`,
`cli`, `mcp`, or any test (grep over the workspace). `render_call_report`
(`:19-25`) builds its own `CallTrace::new()` and is itself uncalled. There is no
`Serialize`/`Deserialize` on any trace type: the workspace's only serde
dependency is `serde_json` in `cli/Cargo.toml:21` and `mcp/Cargo.toml:19`;
`run_method/Cargo.toml:16-19`, `core/Cargo.toml`, and `studio/Cargo.toml:21-27`
declare none.（四个 renderer 无任何调用者；trace 类型上没有 serde。）

### 1.3 Two `&'static str` fields make derived serialization actively costly
`CallSite.function: &'static str` (`core/src/registry_core/declaration/call_evidence.rs:11`)
and `SourceLocation.file` / `SourceLocation.function: &'static str`
(`core/src/registry_core/declaration/source_location.rs:10,15`) are woven
through every frame, local, and edge. A derived `Deserialize` cannot produce a
`&'static str` without `Box::leak`, i.e. one leak per distinct decoded string.
This is the strongest single argument against "just add serde".

### 1.4 Studio's sample is inert, not merely fake
`sample_live_trace()` records frames only — two `with_at` calls — plus, since a
later round, **one** observed local (`studio/src/studio/app/sample.rs:27`,
pinned by that file's own test). The identity half below still stands; the
"emptiness" half no longer does, and is corrected under the bullets.
`sample_live_trace()` 只记录帧——两次 `with_at`——以及后来某一轮补上的**一个**被观测局部值
（`studio/src/studio/app/sample.rs:27`，由该文件自己的测试钉住）。下面的身份那一半仍然成立；
"空"那一半不再成立，已在条目下更正。
Consequences, all verifiable:
- `graph_locals` (`studio/src/studio/app/support.rs:199-211`) filters locals by the
  *centre function's* node, so the sample's single local is invisible for every
  registry function and the DATA panel draws `"· no live locals captured"`
  (`studio/src/studio/ui/graph/data.rs:38-42`) there. The panel can render the
  sample's value only for the sample's own node, which the registry never holds.
  （原文此处说样本不记录 locals，因此面板永远只画空状态；那一半已被后一轮修正，
  其余仍然成立：局部值按中心函数的节点过滤，样本的节点不在注册树里。）
- `call_evidence` returns `Live` only on an exact `(node, function)` match
  (`studio/src/studio/app/graph_queries.rs:196-204`). The sample's nodes come
  from `NodeId::from_path("<studio-sample>", …)` (`sample.rs:16,23`), while
  registry nodes come from `NodeId::from_namespaced_path(...)`
  (`run_method/src/macros/face_registration.rs:45-49`); the two hash domains
  differ, so the match never happens.
- `call_relations`' sample-edge loop also requires the center node to equal the
  sample's node (`graph_queries.rs:152-168`), so its two fake frames never enter
  the graph either.
So the `LIVE SAMPLE` legend (`studio/src/studio/ui/panels.rs:17-33`) and the
`· built-in sample ·` panel title (`ui/graph/data.rs:114`) label a trace that no
registry node can ever select. The honest 1.0 statement is "no trace attached",
not "sample values shown" — a local that exists but cannot be reached is not a
visible effect.（因此那两处标注标的是一条任何注册节点都选不中的 trace；"看得见的效果"
不成立，但"看得见"也不成立。）

### 1.5 Plain answer: can a host hand its trace to a separate Studio process today?
**No.** A `cargo install`ed `nichlink-studio` is a different process; the trace
is an owned, unserialized, private-field value with no writer and no reader.
Even **in-process** it cannot: `App::new` is private (`studio/src/studio/app/lifecycle.rs:39`),
`App::load` unconditionally installs the sample (`:57,80-98`), and no API
accepts a caller's `CallTrace`. The only trace I/O Studio has is the reverse
direction: it runs `cargo rustc` to obtain MIR (`lifecycle.rs:185-211`).
**不能。** 独立进程的 Studio 拿不到 trace：它是私有字段、未序列化、无写方无读方；
即使同一进程，`App::new` 私有、`App::load` 无条件装入示例，也没有 API 接受调用者的
`CallTrace`。

## 2. Transport: choose a serialized artifact file, found by an env var
## 2. 传输方式：序列化 artifact 文件，用环境变量定位

| Transport | Cost today | Needs serde? | Separate `cargo install`ed Studio? |
| --- | --- | --- | --- |
| In-process embedding (host links Studio) | New public `App` constructor/install API; a library host links ratatui + the whole TUI; couples a service to a terminal; still cannot feed a *running* host's trace to a separate TUI | No | **No** — excludes the documented `cargo install` flow |
| **Serialized artifact file** (chosen) | One pure document type + parser + writer + loader; escaping; `&'static str` interning on read | **No** — hand-parsed like `graft.plan` (`core/.../graft/document.rs:27,68,140`) | **Yes** |
| Socket / IPC | Framing, discovery (port vs socket path), liveness, auth, Windows named pipes; host must keep serving; turns Studio into a client, contrary to §4 | Possibly for framing | Yes, at much higher size |
| Env var naming a file | Not a separate transport — it is the *discovery half* of the file option | n/a | Yes |

The file option is the only one that is both honest about the failure mode
(a missing file is visibly missing) and consistent with the repo's existing
artifact precedent: a versioned, line-oriented document written by an execution
surface (`run_method/src/authoring/external_graft/plan.rs:96-127`, using
`atomic_write` at `run_method/src/authoring/filesystem/filesystem.rs:18-27`) and
read by Studio — which also keeps host and Studio decoupled, since the host may
be a long-running binary and Studio an `cargo install`ed tool.

Rejected: **serde/JSON** (would need derives plus `Box::leak` per distinct
string, §1.3); **existing `CallGraph::to_dot`**
(`debug_method/src/adapters.rs:89`) as the artifact (it drops locals, values,
and source locations — exactly the DATA panel's payload); **`NICH_LINK_TRACE`
as the path** (already the *mode* variable, `call_trace.rs:30-35`,
`call_evidence.rs:119`); **sockets** (a profiler-shaped commitment).

## 3. Minimal viable design
## 3. 最小可行设计

### 3.0 Scope decision
This is 1.x, not 1.0, for three concrete reasons: (a) `core`/`run_method` are
being edited concurrently, and the design adds a public reconstruction
constructor to `CallTrace` — the exact type under edit; (b) no serde and the
`&'static str` problem mean the reader needs a real interner, not a derive;
(c) identity is not aligned by default (§3.5). `docs/roadmap-1.0.md:177` already
allows the alternative ("real trace ingest … or permanently label as sample"),
and 1.0 takes the second branch.

### 3.1 Artifact format (1.x)
File `nichlink.trace`, line-oriented `key=value`, mirroring `graft.plan`
(`core/.../plugin/graft/document.rs:27,140`); unknown keys and other versions
are **refused**, not guessed (`document.rs:96-98,180`). Value fields are
tab-separated and backslash-escaped (`\\`, `\t`, `\n`, `\r`), because
`LocalValue.value` and `DataEdge.label` are arbitrary `String`s
(`locals/local_value.rs:20`, `edges/edges.rs:19`).

```text
version=1
namespace=my-app                 # host package name; identity anchor
root=<hex NodeId>                # host registry root; identity anchor
mode=full                        # TraceMode::parse spelling (call_evidence.rs:121-128)
frame=<id>\t<parent|->\t<node>\t<function>\t<file>\t<line>\t<column>
local=<id>\t<frame|->\t<input|let|return|consumer>\t<observed|unobserved>\t<name>\t<type>\t<value>\t<file>\t<line>\t<column>
edge=<from>\t<to>\t<label>\t<file>\t<line>\t<column>
```
Mappings are all existing public vocabulary: `LocalKind::label()`
(`locals/local_kind.rs:14-22`), `Observation::label()`
(`locals/observation.rs:13-18`), `TraceMode::parse` (`call_evidence.rs:121`) and
`NodeId` `Display`/`FromStr` (`identity/node_id.rs:109,150`); an absent
`SourceLocation` (legal on `DataEdge`, `edges/edges.rs:20`) renders as `-`.

### 3.2 Types, signatures, and module placement
The document depends on run_method nouns (`LocalValue`, `DataEdge`, `CallSite`
construction from private `FrameRecord`s), so it **cannot** live in `core`.
`core` owns only the shared text contracts, exactly as with
`GRAFT_PLAN_FILE` (`core/.../lexicon/lexicon.rs:76-80`):
`core/src/registry_core/lexicon/lexicon.rs` gains
`TRACE_FILE = "nichlink.trace"`, `TRACE_DIR = "traces"`,
`TRACE_FILE_ENV = "NICH_LINK_TRACE_FILE"`, pinned by the published-value test
(`lexicon.rs:199-210`). `run_method` is an execution surface and already does
`std::env` and file I/O (`call_trace.rs:31`, `external_graft/plan.rs:96-127`),
so the writer belongs there, **not** in `cli`, because a host depends only on
`run_method`. New file, mounted from `run_method/src/runtime/trace/trace.rs:4-11`:
`run_method/src/runtime/trace/artifact/artifact.rs` (`#[path = …] pub mod artifact;`).

```rust
// run_method/src/runtime/trace/artifact/artifact.rs  (pure half: no I/O)
pub const TRACE_ARTIFACT_VERSION: u32 = 1;

/// One recorded frame, flattened; `function`/`source` are interned on parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceFrame { pub frame_id: u64, pub parent: Option<u64>, pub node: NodeId,
    pub function: &'static str, pub source: Option<SourceLocation> }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceArtifact {
    pub version: u32, pub namespace: String, pub root: NodeId, pub mode: TraceMode,
    pub frames: Vec<TraceFrame>, pub locals: Vec<LocalValue>, pub edges: Vec<DataEdge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceArtifactError { UnsupportedVersion(u32), MissingKey(&'static str),
    UnknownKey(String), Malformed { line: usize, message: String },
    DuplicateFrame(u64), DuplicateLocal(u64), MissingParent(u64),
    DanglingEdge { from: u64, to: u64 } }

impl TraceArtifact {
    pub fn from_trace(trace: &CallTrace) -> Self;                    // reads pub(in trace) fields
    pub fn render(&self) -> String;                                  // pure
    pub fn parse(source: &str) -> Result<Self, TraceArtifactError>;  // pure; interns strings
    pub fn into_trace(self) -> Result<CallTrace, TraceArtifactError>; // pure
}

// I/O half, same module (run_method is an execution surface):
pub fn write_trace_artifact(trace: &CallTrace, path: &Path) -> Result<(), String>;
pub fn read_trace_artifact(path: &Path) -> Result<(TraceArtifact, CallTrace), String>;
```

`into_trace` is the crux and it is cheap because `CallTrace::rebuild_indexes`
(`call_trace.rs:201-233`) already derives every index from the flat `frames` /
`locals` / `edges` arenas. The constructor builds the struct with an empty
`current`, sets `next_local_id`/`next_frame_id` to `max + 1`, then calls
`rebuild_indexes()`. It does **not** replay `with_at` closures, so it cannot
mis-nest a parent link; `artifact` being a descendant of `trace` also reaches
`pub(in trace)` fields without widening any public field.

String interning: a process-global, content-keyed interner leaks each
**distinct** function/file string once (not once per frame), so repeated loads
reuse the same `&'static str`. Bounded by the number of distinct names in one
artifact. This is the honest price of not changing `CallSite`/`SourceLocation`
to `String`; changing those is a public API break and is out of scope.

### 3.3 Who writes it, and when
The **host**, explicitly, at one call site it owns; not on `Drop` and not from
`run_method` internals, because that would bind process lifetime implicitly and
a host may hold several traces. Natural site: where the host already calls
`Registry::health_check(node, value, trace.current_path())` at a value boundary
(`core/.../tree/inspection/inspection.rs:22-31`, `README.md:621-625`) — write
once after the traced operation:
```rust
let trace = CallTrace::full();
// … trace.with_at(...), trace.local(...), trace.transform(...) …
nichlink_run_method::write_trace_artifact(&trace, &path)?;
```
Host writes it **only** when `NICH_LINK_TRACE` selected a collecting mode
(`call_trace.rs:30-35`); under `off` the trace is empty and the host should skip
the write rather than publish an empty artifact.

### 3.4 How Studio finds it
Precedence: (1) `NICH_LINK_TRACE_FILE` (absolute or `package_root()`-relative);
(2) convention path `package_root()/.nichlink/traces/latest.trace`, mirroring
`external_graft_root()` (`external_graft/plan.rs:26-30`); (3) none → no trace.
A CLI `--trace <path>` flag is deferred: `nichlink_studio::launch()` takes no
arguments (`studio/src/studio/studio.rs:23`) and `cli` dispatches `studio` with
no trailing parsing (`cli/src/lib.rs:59`), so a flag changes two public surfaces;
the env var achieves the same end for 1.x.

### 3.5 Version / identity check and what Studio shows
Load once in `App::load` (beside `NICH_LINK_INITIAL_QUERY`, `lifecycle.rs:91-96`),
into `TraceStatus { Absent, Loaded, Mismatch }`; refuse, do not guess:
1. `version != TRACE_ARTIFACT_VERSION` → `Mismatch`.
2. `artifact.namespace != package_namespace()` → `Mismatch`. This was the real
   trap: hosts stamp `env!("CARGO_PKG_NAME")` (`macros/face_registration.rs:45-49`)
   while Studio's scanned snapshot used `authoring_namespace()` = active context,
   else `NICH_LINK_NAMESPACE`, else `"nichlink.default"`
   (`authoring/validation/validation.rs:56-66`,
   `studio/src/studio/app/support.rs:30-44`), so the defaults differed and an
   unconfigured Studio would draw nothing and blame itself. **Decided and built
   (2026-09-26):** a launched session now adopts its project and takes the
   namespace from `[package] name` in the target manifest, so the two ends agree
   without the environment; `NICH_LINK_NAMESPACE` still overrides both ends
   verbatim. Pin: `studio::app::tests::project::a_launched_session_authors_under_the_host_crates_package_name`
   (red before the change: `nichlink.default` against the fixture's `demo-app`).
3. `artifact.root != registry.id()` → `Mismatch`; both are
   `root_node_id(namespace)` (`identity/node_id.rs:93`, `tree/registry.rs:55-68`).
4. Every frame/local/edge `NodeId` must resolve via `registry.find(node)`; an
   unresolvable id → `Mismatch { unresolved }`. This is the only real
   "different build" detector: node ids are source-path+namespace derived
   (`identity/node_id.rs:47-57`), not build derived, so neither the trace nor
   the registry carries a build hash today. A stale artifact of the same crate
   is detected by its records pointing at faces this snapshot no longer has.
On `Mismatch` Studio installs **no** trace and shows the reason in the event
line and DATA panel, keeping the registry snapshot visible — the
`poll_hot_reload` habit of keeping the last good snapshot
(`lifecycle.rs:108-142`) applied to evidence instead of faces. It must **not**
silently fall back to the sample; that silence is what makes the current UI
dishonest. When the trace is **absent** the legend reads `TRACE: none` and the
DATA panel `no trace attached` (never a bare `LIVE`); when **loaded**, legend
`LIVE` and the panel drops `built-in sample`; when `Mismatch`, `TRACE mismatch`.
For 1.x, remove `sample_live_trace()` (`sample.rs:7-28`) and construct
`App::runtime_trace` with `CallTrace::disabled()` (`call_trace.rs:98`); the
sample is inert anyway (§1.4).

### 3.6 The 1.0 deliverable (label only)
1.0 ships **no** ingest. It keeps `LIVE SAMPLE`
(`studio/src/studio/ui/panels.rs:25`) and `· built-in sample ·`
(`ui/graph/data.rs:114`), because §3.5's checks have nothing to check without an
artifact. The only 1.0 change is a regression test that the legend can never
render a bare `LIVE` while the sample is installed. No empty enum variants and
no unused loader: the repo deletes zero-caller surface
(`docs/roadmap-1.0.md:51-52`), so the §3.2 types stay unbuilt until the artifact
exists.

## 4. What must NOT be built
## 4. 不应构建的东西

Not a profiler: no sampling, no automatic per-frame instrumentation, no
hot-path capture, no async/cross-thread aggregation, no streaming/socket, no
continuous re-read (load once; a changed artifact needs a restart), no MIR
merge in the ingest path, no multi-trace timeline or diffing. The trace stays
"the host explicitly recorded these values".

**The one case that earns real data:** a single `trace.with_at(...)` around one
registered face's handle call, with one `trace.local(...)` input and one
`trace.return_value(...)`/`trace.transform(...)` output, written at the value
boundary where the host already calls `health_check` (`inspection.rs:22-31`).
Studio then shows that one function's input, output, and transform edge for a
real node — not a call graph, not every local in the process.
（唯一场景：在宿主已调用 `health_check` 的值边界上，对一个注册面 handle 调用做一次
`with_at`，记录一个输入 `local` 与一个 `return_value`/`transform` 输出并写出 artifact。）

## 5. Tests
## 5. 测试

### 5.1 1.0 (the only tests that ship now)
`LIVE_SAMPLE` regression: render the brand (`ui/ui.rs:126-137` already has the
`TestBackend` helper) and assert the legend is `LIVE SAMPLE`, never a bare `LIVE`
(`panels.rs:25`) — one test in `studio/src/studio/ui/ui.rs` tests.

### 5.2 1.x
Unit, in `artifact.rs`:
- Render→parse→`into_trace` round-trip preserves frames, parents, locals,
  edges, and the `LocalKind`/`Observation`/`TraceMode` spellings; a value
  containing `\t`, `\n`, and `|` survives escaping.
- `parse` refuses `version=2` (`UnsupportedVersion`) and an unknown key
  (`UnknownKey`) — the `graft.plan` precedent (`document.rs:96-98,180`).
- `into_trace` rejects duplicate frame/local ids, a missing parent, and an edge
  whose `from`/`to` has no local; accepts a well-formed artifact and rebuilds
  `local_function_index` so `matching_locals(function)` (`locals/call_trace.rs:164-200`)
  resolves; `from_trace` on a recorded trace reproduces the same `render()` on a
  second pass (idempotence).

End-to-end, in `studio/tests/trace_ingest.rs` (Studio already depends on
`run_method`, `studio/Cargo.toml:25`; `run_method` cannot test Studio):
1. Create a temp host project, `select_project(root, manifest, "ingest-test")`
   (prelude at `app/tests.rs:17`).
2. Record `CallTrace::full()` with `NodeId::from_namespaced_path("ingest-test",
   <real source>, <real kind>)` for a face in the fixture, plus one `local`
   input and one `return_value` output; `write_trace_artifact` to
   `<root>/.nichlink/traces/latest.trace`.
3. `App::load()`; assert `app.graph_locals(&center)` for that face's node
   returns the recorded values (the exact call `draw_data_flow_panel` makes,
   `ui/graph/data.rs:36`).
4. Render the graph overlay with `TestBackend` and assert the buffer contains
   the recorded value and no `built-in sample`; set
   `SearchState { graph_mode: true, graph_focus: 3, center, center_function, .. }`
   (`ui/graph.rs:160-166`).
5. Identity tests: an artifact with `namespace=nichlink.default` or a foreign
   `root` yields `Mismatch`, installs nothing, and leaves the registry visible.

## Open questions for the owner
## 给 owner 的待决问题

1. **1.0 vs 1.x:** accept the §3.0 deferral, or freeze `CallTrace` for one
   signature (`from_records`/`into_trace`) so ingest can land in 1.0?
2. **Identity — decided (2026-09-26): yes.** A launched session reads `[package]
   name` from the target `Cargo.toml` and authors under it, so ingest does not
   require exporting `NICH_LINK_NAMESPACE` and the mismatch path no longer fires
   by default. Built in `studio/src/studio/app/support.rs` (`namespace_for`,
   `package_name`, `manifest_for`), pinned by
   `studio::app::tests::project::{a_launched_session_authors_under_the_host_crates_package_name,
   the_package_name_comes_from_a_literal_package_key,
   the_namespace_follows_the_manifest_unless_the_environment_names_one}`.
3. **Writer name:** `NICH_LINK_TRACE_FILE` — accept, or fold discovery into a
   `nichlink trace --host-output <path>` CLI verb?
4. **Escaping vs length-prefix:** is backslash escaping of `\t`/`\n`/`\`
   acceptable for 1.x, or should records be length-prefixed to make byte-exact
   round-trip trivial at the cost of unreadable files?
5. **Convention path lifetime:** `latest.trace` overwritten per run, or one file
   per run with an explicit path always passed? Overwrite keeps Studio's search
   simple but loses the previous run.
6. **1.0 label wording:** keep `LIVE SAMPLE`, or strengthen it to
   `TRACE: none` given §1.4 shows the sample renders nothing?
