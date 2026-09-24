# Changelog

English | [简体中文](#简体中文)

All notable changes to the NichLink workspace are recorded in this one file.
The nine crates are released as a single version line, so a change is described
once here instead of nine times: **per-crate changelogs are deliberately not
kept**. Release order and the reasoning behind the one-line release are in
[`docs/roadmap-1.0.md`](docs/roadmap-1.0.md).

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `deny.toml` and a `cargo-deny` CI job covering advisories, licenses, and bans.
- A CI job that compiles the non-default feature set and the feature-less
  surface: `cargo test --workspace --all-features --all-targets` and
  `cargo check --workspace --no-default-features --all-targets`. This is what
  first compiles `studio`'s `prototype-fixtures` tests, and it re-checks
  `plugin-host`'s `process-tools` through `--all-features`.
- A `LICENSE` copy inside every crate directory, so each published package
  carries its own licence text (Cargo only auto-includes a crate-local
  `LICENSE*`).
- `macro/README.md` plus the matching `readme` field for `nichlink-macro`.
- `repository` and `homepage` on all nine manifests, `documentation` where it
  was missing (`nichlink-macro`, `nichlink-run-method`), and
  `[package.metadata.docs.rs] all-features = true` on the crates whose
  non-default features are public API.
- `tools/nichlink-package-audit` now warns about and skips packages whose
  versioned NichLink dependencies are not on crates.io yet, and exits 0; the CI
  audit step no longer hides a failure behind `continue-on-error`.
- `studio`: a workspace-only `dev-supervisor` feature gating `nichlink-dev`, and
  the source-only fixture host package under `studio/tests/fixtures/node-editor/`
  that the `prototype-fixtures` tests were written against.
- `tools/nichlink-publish`: publishes the nine crates level by level in
  dependency order, waiting for each level to reach the index before starting the
  next. It is a dry run unless `--publish --yes` is given.
- A `conventions` crate (`publish = false`) whose tests are the executable form
  of five rules that previously lived only in prose: kernel purity, module
  mounting, the 450-line ceiling as a ratchet, the `missing_docs` attribute and
  the `#[allow(missing_docs)]` ban, and parseability of the Rust blocks fenced in
  the READMEs and living docs. Each gate was shown to fail on a real violation
  before being kept.
- A CI step running `cargo test --workspace --all-features --doc`.
- `tools/nichlink-visual`: a visual check that runs the real Studio binary in a
  fixed-size tmux pane, drives it with keys, and saves what the terminal actually
  shows — plain text and, with `-e`, the SGR escapes. Scenes cover the home page,
  the search overlay, the add form, the call graph, the focused call tree, an
  arrow walk through that tree, and the data-flow panel; each scene waits for the
  app and then for its own screen before capturing, because capturing between the
  two sometimes saves the previous one. Output goes to `target/visual`, so a run
  leaves nothing in the checkout.
- A rendered call-tree test asserts the picture instead of the model: it renders
  Studio with the graph overlay open, cuts the call-tree panel out of the page by
  the corners that panel drew, and checks that every node inside the drawn window
  has its label on screen, that two nodes in one column do not overlap, that a
  higher level is drawn to the right of a lower one, that every one-column hop
  has its arrow on the callee's row, and that the header counts what the model
  holds. It prints the panel it checked, so the shape can be read as well as
  asserted.
- One JSON string encoder in the kernel, `nichlink::json`, shared by the build
  diagnostics document, the MIR JSONL artifact and the generated editor
  snippets. It escapes exactly what RFC 8259 requires, and its tests assert the
  absence of raw control characters rather than a round trip, because this
  workspace's own parsers are lenient enough to accept invalid output.
- `lexicon` now owns `PACKAGE_ROOT_ENV`, `NAMESPACE_ENV` and
  `DEFAULT_NAMESPACE`, plus the pure `resolve_package_root` and
  `resolve_namespace` rules, so the environment names and the resolution order
  have one definition instead of one per surface.
- `tools/nichlink-package-audit` now checks package *contents* for all nine
  crates: every `src/**/*.rs` module and the manifest's declared README must be
  in the package. `cargo package --list` needs no registry, so this half runs
  before the first publish instead of waiting for it.

### Changed

- The graph page is the call tree and the values its cursor's function ran with,
  two panes and nothing else: the relation column that listed callers and callees
  beside the tree is gone (the tree draws those as edges), and the side-by-side
  comparison page went with it, along with its `Ctrl-W` binding, its second search
  field, its `SearchState` fields and the `4` page key. `[` and `]` now move the
  tree's share of the page, and the divider drag clamps in the same range.
- Edges leave the caller's **bottom** and enter the callee's **top**, which is
  what makes the picture a tree: side ports had turned every hop into a sideways
  shuffle. The focus sits in the middle of the picture whenever anything in the
  tree calls it, and at the top only when nothing does — which is what the kernel's
  tree actually says.
- The graph page's letters are commands and its text is not: typing a letter used
  to edit the query from a page where `m`, `v`, `g` and `e` are commands, and the
  catch-all typing arm sat in front of `e`, so that key could never fire (rustc
  cannot see that conflict, because the arm it shadowed carries a guard). One stale
  `graph_focus == 2` arm, left over from the removed comparison pane, went with it.
  The query is edited in the list page, which `/` returns to, and the footer names
  every key of both pages.
- The fixture's call graph now has a tree in it — `paint_node_editor` fans out to
  `layout_panels`, `clamp_canvas_width` and `preview_canvas_width_traced`, and the
  panel measurers meet again at the clamp — so the visual check has a fork, a depth
  and a fan-in to show; `tools/nichlink-visual tree-demo` draws it.
- The widget is told what the kernel knows, and keeps its own viewport. Node text
  carries the cut counts the budgets left out (`←n`/`→n`), a `▶` for the cursor and
  a `◆` for the focus — marks, not only colours, because the report that started
  this was a cursor whose movement was invisible in a terminal with no colour —
  while each edge carries its evidence kind as a label. The widget is now cached on
  the app: rebuilt when the focus changes (then fitted), re-stamped when the cursor
  moves while its viewport and pan/zoom travel with it, and untouched in between, so
  a drag is not interrupted. Mouse events over its tree go to it, which is what
  gives the panel wheel zoom, drag panning and click-to-select; its nodes are not
  draggable, because the tree is a view of the model and a node must not drift away
  from the level and lane it stands for.
- Boxes are given the room their own labels need: a lane is as wide as its widest
  box and a band as tall as its tallest, and each box is centred in that cell.
  Fixed pitches could not survive the names — a 27-character symbol was drawn
  across its neighbour, which is the crowding that made the tree hard to read —
  and the widget's layout has a regression test that fails on any pitch that
  ignores the content (checked against a fixed one: the long box overlaps).
- The four arrow keys follow the **drawn** grid instead of the model's own axes, so
  they keep their printed meaning in both drawers: with the calls running down the
  screen `↓` steps toward a callee and `↑` back toward the callers, with them
  running across it those two roles go to `→` and `←`, a level step follows an edge
  that actually exists (and prefers the lane it is already in), and a sideways
  press takes the nearest lane of the same band. A press with nothing in that
  direction says so instead of moving somewhere else.
- The call tree is drawn by the `rataflow` node-editor widget by default, with the
  hand-drawn canvases one key (`g`) away and still compiled in: the `node-graph`
  feature is a default feature now, so the crate ships the widget look, and the
  canvases remain the ones that draw the kernel's own facts (evidence kinds, cut
  counts, cursor versus focus) and fit a seven-cell column.
- The call tree can also be drawn **top-down**, and a narrow panel switches to it
  automatically: the horizontal canvas needs one column per hop and an 80-column
  pane gives the tree 19 cells, so two boxes shrank to seven cells each and their
  names to tails — while the same tree laid out downwards spends one margin column
  on the edges and then gives every box the panel's full width, one band per hop,
  callers above and callees below. The switch is automatic below the width where
  the horizontal layout can still give two columns a readable box, `v` cycles
  panel-decides / top-down / left-to-right, and the title names the mode.
- The call tree is drawn as node boxes instead of a ledger. Each function is a
  bordered cell whose content wraps its **whole** name — the ledger clipped a long
  name to 15 cells and kept only its tail, so `preview_canvas_width` read as
  `…w_canvas_width` — the cursor's box takes the heavier border and the focus's
  the green one, cut counts are badges on the box, and edges leave a port on the
  caller's border, bend through the gap and end in `▶` with their evidence mark
  beside it. Columns are as wide as the widest name in them and are narrowed
  rather than dropped when the panel is one cell short, so the callers of the
  function being read stay on screen. A legend on the bottom border names the
  markers, and clicking a box selects that node. The packing reserves room for a
  neighbour before the cursor's own column takes its width, tightens the gap from
  five cells to three when that is what buys the second column, and abbreviates a
  name to its tail with `…` rather than wrapping it into a five-row sliver — all
  three because a tree drawn as one box has no edges, which is exactly what a
  narrow terminal used to show. `[` and `]` move the same split the divider drags,
  and a panel too narrow even for the tight packing says `[ ] widens` in its title
  instead of letting the reader believe the function has no neighbours.
- The New Project and plugin forms name their rows too:
  `app::state::new_project_field` and `app::state::plugin_field` hold one constant
  per row beside the label the form prints, so a `values[0]` at a call site became
  `values[new_project_field::DIRECTORY]`, and the two label tables the renderers
  carried are gone with them.
- Package READMEs, `docs/discussion-introduction*.md`, and the workspace
  layout in the root README were corrected to describe the current nine-crate
  workspace.
- `NICH_LINK_ENTRY` is now resolved once per build and drives both scope pruning
  and the generated `BUILTIN_GRAFT_CUTS` table; a value that names no file fails
  the build instead of letting one reader fall back to Cargo's `main.rs`.
- A build now **fails** when an external graft plan's target slot is named by no
  declaration (it used to be a `cargo:warning`), and the message carries the
  `static_graft_plan!` clause to paste. A declaration whose `#[cfg]` is off in
  this build still counts, so a legitimately gated slot is not reported.
- `apply_recorded_grafts` prints one `warning:`/`note:` line per report, so a
  record that is skipped can no longer be silent.
- A sibling range written from the later name to the earlier one is refused with
  the ordering rule (`registry_name` order) instead of being silently swapped.
- `nichlink-plugin-host`'s process backend now enforces the limits it declares.
  The request frame is written from its own thread, stdout and stderr are drained
  from theirs, and the deadline bounds the whole call, so
  `ProcessLimits::max_output_bytes` (1 MiB by default) is the real ceiling. It
  used to be the ~64 KiB pipe buffer, and exceeding it reported `Timeout`.
- `petgraph` moved from 0.6 to 0.8. It backs `CallGraph`'s private fields only,
  so no public API changes.
- `studio/src/studio/ui/forms.rs` mounts `face_fields` with `#[path]`, like its
  four siblings. This was a style difference rather than a defect: both spellings
  resolve, because a `#[path]`-loaded parent resolves children by directory.
- Studio, the authoring executor and the MCP bridge resolve the project through
  one kernel rule instead of three copies. Behaviour is unchanged; the one
  deliberate difference is stated where it lives: the bridge's last-resort
  fallback stays the working directory, because a stdio bridge is started inside
  the project the agent works on, whereas a compiled-in manifest path belongs to
  the machine that built the binary.
- `wasmi` moved from the 0.42.1 line to a `1.0.9` floor (the lock resolves
  1.1.0). The 0.x caret requirement could never reach the fixes that matter for a
  sandbox — the memory read/write integer overflow, growing memory past the
  system limit, `rem_s(MAX, -1)` trapping where the spec demands 0, and the
  loop-local and wide-arithmetic miscompilations. One call site changed:
  `Linker::instantiate` plus `PreInstance::start` became
  `instantiate_and_start`, which runs the same `start` function, still with fuel
  set before it. Fuel remains the pre-2.0 kind, so `fuel_per_call` keeps its
  meaning; its doc now says the unit belongs to the engine and a major upgrade
  can re-scale it.
- Five duplicated rules now have one implementation each: the cut-endpoint
  rendering shared by the manifest writer and the CLI report, the CLI's JSON
  error document and command error text, Studio's plan path (from the runtime
  loader's `graft_record_root`), the PascalCase kind derivation (the kernel's
  `pascal_case`) and the `::`-bounded symbol match (the kernel's `same_symbol`,
  now public). Two of those had already diverged.
- **The registration-face authoring surface is now the size of what an author
  actually decides.** `handle` and `params` are the kind by rule and
  `registry_name` is the declaring module's last path segment, so
  `__control_object!` and `__external_object!` keep one arm each and the three
  spellings of one type are gone from declaration files; the minimum face is
  `control_object! { kind: Button }`. Trait labels are derived from the
  compiler-checked contract paths by one rule shared by all three readers (the
  file view in `FaceSyntax::string_list`, the proc macro
  `__face_trait_labels_or!`, and the applier `apply_trait_contract`), so a label
  can no longer disagree with the path the compiler checks. `FACE_FIELD_ORDER`
  went from 29 accepted keys to 24.
- **Studio's add/edit form is 26 slots, not 30**: 22 author rows followed by one
  read-only machine-value strip, with the slots named in `authoring::face_field`
  so the Studio write paths no longer index the array with bare literals. Four
  slots left with the fields they described. Help text was corrected wherever it
  contradicted the code: the tree slot claimed an override that the
  registry-name rule forbids, `external source note` said "provenance only" for
  a field registry resolution reads (it is now labelled `dependency registry`),
  and the kind/parts rows were marked derived while the form edits them.
- The call report states one fact once: `handle=` sat next to `kind=` on every
  reported function, and the search matched `params` as a second key for the
  same string. Both are the kind by rule, so only the kind is printed and only
  the kind is searched; `RegistrationInfo` keeps both fields for hosts that read
  them.
- A typed graft cut now proves the output types across the two faces it joins:
  the build emits `assert_contract::<{cut}::__Preset, {graft}::__Parts>()` beside
  `BUILTIN_GRAFT_CUTS`, so a replacement face whose parts do not supply the
  preset's parts fails the build. String and selector cuts emit no assertion, and
  the code says so where the accumulator is filled rather than falling back to
  the deleted string comparison.

### Removed

- The author-written `expected_output` / `actual_output` pair. It was compared
  only with itself in two places (`release.rs` and
  `build_method::check_output`) and never described the real types: the button
  face declared `"ControlFrame"` while its `NoPreset`/`NoParts` output is `()`.
  The type-level fact it pretended to state is now proven by the per-face
  `assert_contract` and by the per-cut assertion above. The fields, both
  comparisons, the cache keys, the authoring plumbing and the Studio slots are
  gone.
- `authoring::FACE_FIELD_NAMES`: a 30-label table read only by tests and a
  near-duplicate of the presentation labels. The layout's names live in
  `authoring::face_field` now, and the tests that used the table assert the
  stronger property instead: the slots are dense and each has presentation
  metadata.
- Rows Studio printed more than once for one fact: a `values` runtime-snapshot
  row and a `declared` file:line:function row in the search details panel (the
  file is already the `path` row), plus the `params`/`handle` rows that repeated
  the `kind` row in both details panels. `params`, `handle` and `kind` are one
  string: both macros expand them from `stringify!($kind)`.

### Fixed

- The process backend no longer has unbounded blocks. Three were measured and
  removed: a `try_wait` poll that never drained stdout, so a healthy child
  writing more than the pipe buffer was killed and reported as `Timeout` (the
  measured boundary was a 65 536-byte frame succeeding and 65 537 bytes timing
  out); a `stdin` write that happened before the deadline was armed, so a child
  that never reads stdin pinned the caller past it (1 MiB blocked for the child's
  whole lifetime and then reported a broken pipe); and a frame read that sat
  outside the deadline. Five regression tests pin these.
- The `compile_fail,E0433` doctest that pins the deleted `edit_module` was
  compiled by no command, because the only `--all-features` job passed
  `--all-targets` and that skips doctests entirely. It is compiled now.
- `MirGraph::to_jsonl` emitted a raw control character for a name that contained
  one, producing an artifact every strict JSON reader rejects while this crate's
  own lenient parser still accepted it. Reproduced before the fix (a tab in a
  function name made both emitted lines invalid) and verified after it against an
  independent strict parser. The scaffold snippet writer had the same weak escape
  for its VS Code JSON; both now use the kernel encoder.
- `tools/nichlink-package-audit` read a crate's dependency list with a loop that
  split on whitespace, so `nichlink-studio` and `nichlink-cli` were evaluated
  with only their first dependency and the summary listed phantom crates.
  Dependencies are comma separated and parsed as such, and the summary names each
  crate exactly once.
- The process adapter no longer reports a spurious failure when several plugin
  calls run at once. `exec` can transiently reject a freshly staged executable
  with `ETXTBSY` ("Text file busy"), which surfaced as `ExecutableFileBusy` under
  a parallel test run; spawning is now retried a bounded number of times and
  anything else is still reported unchanged.
- Dead code that `#[allow(dead_code)]` and `#[allow(unused_imports)]` were
  holding down is gone: a write-only `RegistryHeader::name` and its unused
  `admission()` accessor (with the module-level allows that hid them),
  `EntryPages::iter()`, `FaceManifest::get()`/`fields()`, `SearchRow::source_index`
  (written four times, read never), a `state::FACE_FORM_ORDER` re-export nothing
  referenced, a stale `#[allow(dead_code)]` on a function that does have a
  caller, and about forty unused imports. Removing the allows rather than
  keeping them is what found the last group: rustc names the real set.
- Every published crate now carries `#![warn(missing_docs)]` and documents its
  whole public surface (about 700 items across the nine crates), so a release
  cannot add an undocumented public item. Two downstream warnings found while
  enabling it are fixed with it: the `NODE_ID`/`REGISTRATION` constants emitted
  by `__registration_face!` and the generated `builtin_static_plan()` /
  `registrations()` / container modules now carry docs, so a host that turns the
  lint on no longer sees warnings in generated code it cannot edit.
- `nichlink-dev` can no longer be installed broken: it sits behind the
  non-default `dev-supervisor` feature, resolves the Studio binary from a
  sibling, `PATH`, or the workspace `target/debug` in that order, and reports the
  missing checkout by name when it was built from one that is gone.
- `apply_recorded_grafts` refuses, instead of skipping, three artifacts with no
  legitimate reading: a plan that does not parse (every unreadable plan is named
  at once), a record whose directory disagrees with its plan's `graft`
  (`RecordReport::SelectorDirectoryMismatch` is gone), and a record whose
  identity and path name different faces.

## [0.1.0] - 2026-09-23

First release. All nine crates are published together in dependency order:
`nichlink-core` → `nichlink-macro` / `nichlink-build-method` / `nichlink-mcp`
→ `nichlink-run-method` → `nichlink-debug-method` / `nichlink-plugin-host`
→ `nichlink-studio` → `nichlink-cli`. Dependency requirements are written as
caret `0.1.0`, so a patch release does not force dependents to republish.

- **`nichlink-core`** (library `nichlink`): the pure kernel — registration
  vocabulary, the `Registry` tree with atomic page-copy transactions, admission
  and flow contracts, graft declaration/application records, plugin policy,
  MIR/source evidence models, and the `syntax` registration-face parser.
- **`nichlink-macro`**: the compile-time face-field front end (tolerant
  separators and field order, spanned diagnostics, editor mirror).
- **`nichlink-build-method`**: build-time source discovery, identity cache,
  scope pruning, and `StaticPlan` generation.
- **`nichlink-run-method`**: runtime trace state, the `host!`/`trace_call!`
  macros, and the `authoring` executor.
- **`nichlink-debug-method`**: MIR, `CallTrace`, data-flow, and graph evidence.
- **`nichlink-plugin-host`**: verified Wasm (`wasm`, default) and process
  (`process-tools`) adapters with atomic hot deployment.
- **`nichlink-studio`**: the Ratatui authoring and inspection surface.
- **`nichlink-mcp`**: the read-only MCP stdio bridge for five source queries.
- **`nichlink-cli`**: the unified `nichlink` / `cargo-nichlink` binaries.

Known limits for this line are in the root `README.md` and
[`docs/threat-model.md`](docs/threat-model.md). Version-dependent behaviour
such as the kind-only `registry_name` derivation is recorded in
[`docs/roadmap-1.0.md`](docs/roadmap-1.0.md).

---

## 简体中文

NichLink 工作区的所有变更都记录在这一份文件里。九个 crate 按同一条版本线发布，
因此一个变更只在这里描述一次，而不是九次：**有意不按 crate 分九份维护**。发布顺序与
理由见 [`docs/roadmap-1.0.md`](docs/roadmap-1.0.md)。

格式遵循 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)；版本号遵循
[语义化版本](https://semver.org/lang/zh-CN/spec/v2.0.0.html)。

### [Unreleased] 未发布

新增：

- `deny.toml` 与 `cargo-deny` CI 任务，覆盖 advisories、licenses 与 bans；
- 编译非默认特性集与无默认特性表面的 CI 任务：
  `cargo test --workspace --all-features --all-targets` 与
  `cargo check --workspace --no-default-features --all-targets`。`studio` 的
  `prototype-fixtures` 测试首次因此被编译，`plugin-host` 的 `process-tools` 也经
  `--all-features` 再检查一遍；
- 每个 crate 目录内各放一份 `LICENSE`，使每个发布的包自带许可证文本（Cargo 只会自动
  包含 crate 目录下的 `LICENSE*`）；
- `macro/README.md` 与 `nichlink-macro` 的 `readme` 字段；
- 九个 manifest 补上 `repository` 与 `homepage`，缺失处补 `documentation`
  （`nichlink-macro`、`nichlink-run-method`）；非默认特性属于公开 API 的 crate 补
  `[package.metadata.docs.rs] all-features = true`；
- `tools/nichlink-package-audit` 现在会警告并跳过版本依赖尚未上 crates.io 的包，并以 0
  退出；CI 的包审计步骤不再用 `continue-on-error` 掩盖失败；
- `studio`：新增仅限工作区的 `dev-supervisor` 特性以门控 `nichlink-dev`，并签入
  `prototype-fixtures` 测试当初针对的、仅含源码的夹具宿主包
  `studio/tests/fixtures/node-editor/`；
- `tools/nichlink-publish`：按依赖顺序**分层**发布九个 crate，每层发完等它出现在 index 里
  再进下一层。除非给出 `--publish --yes`，否则只做 dry-run。
- 新增 `conventions` crate（`publish = false`），其测试是五条此前只存在于散文中的规则的可执行
  形式：内核纯净性、模块挂载、450 行棘轮、`missing_docs` 属性与 `#[allow(missing_docs)]` 禁令、
  以及 README 与活文档里 Rust 围栏的可解析性。每道门禁在保留之前都实测过"制造违规即失败"。
- CI 新增 `cargo test --workspace --all-features --doc` 步骤。
- 新增 `tools/nichlink-visual`：视觉检查脚本，在固定尺寸的 tmux 面板里运行真实 Studio 二进制、
  用按键驱动它，并保存终端实际显示的内容——纯文本，以及加 `-e` 时带 SGR 转义的版本。场景覆盖首页、
  搜索浮层、新增表单、调用图、聚焦的调用树、在树上用方向键走动的过程，以及数据流面板；每个场景先等
  程序起来、再等它自己那一屏出现，然后才抓屏，因为在两者之间抓屏有时会存下上一屏。输出写到
  `target/visual`，因此一次运行不会往检出里写东西。
- 新增"渲染出的调用树"测试，断言的是**图**而不是模型：它带图浮层渲染 Studio，按调用树面板自己
  画出的四角把该面板从页面里裁出，然后检查窗口内每个节点都在屏幕上有标签、同一列的两个节点不重叠、
  层号更大的画在更小的右边、每一跳一列的边都在被调用者那一行有箭头、表头数出的数量与模型一致。
  它会把检查过的面板打印出来，因此这个形状既可断言也可阅读。
- 内核新增唯一的 JSON 字符串编码器 `nichlink::json`，由构建诊断文档、MIR JSONL 工件与
  生成的编辑器片段共用。它只转义 RFC 8259 要求的那一份；其测试断言"不存在原样控制字符"
  而不是做往返，因为本工作区自己的解析器宽松到会接受非法输出。
- `lexicon` 现在拥有 `PACKAGE_ROOT_ENV`、`NAMESPACE_ENV` 与 `DEFAULT_NAMESPACE`，以及纯的
  `resolve_package_root` / `resolve_namespace` 规则，因此环境变量名与解析顺序只有一处定义，
  而不是每个执行面一份。
- `tools/nichlink-package-audit` 现在检查九个 crate 的包**内容**：每个 `src/**/*.rs` 模块与
  清单声明的 README 都必须在包里。`cargo package --list` 不需要 registry，因此这一半在首次
  发布之前就生效，而不是等它。

变更：

- 调用图页现在只有调用树与"树游标所在函数运行时的取值"两块面板，别无他物：曾经与树并排的
  "调用者/被调用者"关系栏已删除（树把那些画成了边），左右对比页也随之删除，连同它的 `Ctrl-W`
  绑定、第二个搜索框、它的 `SearchState` 字段以及 `4` 页面键。`[` 与 `]` 现在移动树在页面上的
  份额，分隔线拖动使用同一范围。
- 边从调用者的**底边**离开、进入被调用者的**顶边**，这正是让这张图成为一棵树的东西：侧边端口
  把每一跳都变成了横着挪一步。只要树里有东西调用焦点，焦点就画在图的中间；只有确实没有调用者时
  才在顶部——那正是内核的树实际说的话。
- 调用图页的字母是命令、文本不是：以前在这个页面上输入字母会编辑查询，而这里 `m`、`v`、`g`、`e`
  都是命令，且兜底的输入分支排在 `e` 之前，于是那个键永远触发不了（rustc 看不见这种冲突，因为它
  遮蔽的那条 arm 带着守卫）。一条从已删除的对比面板遗留下来的 `graph_focus == 2` arm 也一并清除。
  查询在列表页编辑，`/` 回到那里，页脚把两页的按键都列出来。
- 夹具的调用图里现在有一棵树了——`paint_node_editor` 分叉到 `layout_panels`、
  `clamp_canvas_width` 与 `preview_canvas_width_traced`，而两个测量函数又在钳制处汇合——因此视觉
  检查有分叉、有深度、也有扇入可看；`tools/nichlink-visual tree-demo` 画的正是它。
- 控件现在被告知内核知道的事情，并且保留自己的视口。节点文本带上预算漏掉的裁剪计数
  （`←n`/`→n`）、游标的 `▶` 与焦点的 `◆`——是**标记**而不只是颜色，因为引出这一轮的反馈正是
  "游标移动在没有颜色的终端里看不见"——而每条边带着自己的证据种类作为标签。控件现在缓存在 App
  上：焦点变化时重建（并重新铺满），游标移动时重盖标记而视口随行，其余情况下原样复用，因此拖动
  不会被打断。落在它那棵树上的鼠标事件交给它，这正是本面板获得滚轮缩放、拖拽平移与点击选中的
  方式；它的节点不可拖动，因为这棵树是模型的视图，节点不能偏离它所代表的层与车道。
- 盒子拿到自己标签所需的宽度：车道取其中最宽盒子的宽度，带取其中最高盒子的高度，每个盒子在该格里
  居中。固定间距扛不住名字——一个 27 字符的符号会画到邻列身上，这正是让树读不懂的拥挤——控件绘制方
  的排版现在有一条回归测试，任何无视内容的间距都会让它失败（已用固定列宽验证过：长盒子确实重叠）。
- 四个方向键跟随**画出来的**网格，而不是模型自己的轴，因此它们在两种绘制方里都保持字面含义：调用沿
  屏幕向下时 `↓` 走向被调用者、`↑` 回到调用者，沿屏幕横向时这两个角色交给 `→` 与 `←`；沿层的移动走
  的是确实存在的那条边（并优先留在原车道），横向则取同一条带内最近的车道。该方向没有节点时状态行会
  说出来，而不是挪到别处去。
- 调用树默认由 `rataflow` 节点编辑器控件绘制，手绘画布只差一个按键（`g`）且仍然编入：`node-graph`
  现在是默认特性，因此 crate 出厂即是控件的样子，而画布仍保留着画内核自身事实（证据种类、裁剪计数、
  游标与焦点的区分）以及塞得进七格宽列的那一份。
- 调用树也可以**自上而下**画，并且窄面板会自动改用这种画法：横向画布每一跳要占一整列，而 80 列
  的面板只给树 19 格，于是两个盒子各缩到七格、名字只剩尾部——而同一棵树向下排只需为边花掉一列边距，
  然后让每个盒子都拿到面板整宽，一跳一条带，调用者在上、被调用者在下。当面板窄到横向画布无法再给
  两列各一个可读盒子时自动切换，`v` 在"跟随面板 / 自上而下 / 从左到右"之间循环，标题写出当前模式。
- 调用树改为画**节点盒**，不再是账本。每个函数是一个带边框的单元格，内容按**整名**换行——
  账本把长名字裁到 15 格且只留尾部，因此 `preview_canvas_width` 读作 `…w_canvas_width`——游标的
  盒子用更重的边框、焦点的盒子是绿色，裁剪计数是盒子上的徽标，边在调用者边框上留下端口、在空隙里
  转折、以 `▶` 收尾并在旁边带证据标记。列宽取该列最宽的名字，而面板小一格时列是被**收窄**而不是
  被丢弃，因此正在阅读的函数的调用者仍留在屏幕上。底边图例列出各标记，点击盒子即选中该节点。
  打包会先给邻列留出位置再让游标列取宽，在"这样能换来第二列"时把空隙从五格收到三格，并在名字放不进
  两行时改为保留尾部加 `…`，而不是折成五行窄条——这三条都因为画成一个盒子的树没有边，而窄终端过去
  显示的正是那个。`[` 与 `]` 移动分隔线拖动用的同一个比例，而连窄空隙都放不下的面板会在标题里写
  `[ ] widens`，而不是让读者以为这个函数没有邻居。
- 新建项目与插件表单的行也具名了：`app::state::new_project_field` 与
  `app::state::plugin_field` 每行一个常量，并挨着表单打印的标签，因此调用点上的 `values[0]`
  变成 `values[new_project_field::DIRECTORY]`，渲染器原先各自携带的两张标签表也随之消失；
- 包 README、`docs/discussion-introduction*.md` 与根 README 的工作区结构已更正为当前
  的九 crate 工作区；
- `NICH_LINK_ENTRY` 现在每次构建只解析一次，同时驱动作用域剪枝与生成的
  `BUILTIN_GRAFT_CUTS` 表；指不到文件的值会让构建失败，而不是让一个读取者回退到 Cargo 的
  `main.rs`；
- 外部 graft 计划的目标槽位没有任何声明命名时，构建现在**失败**（原为
  `cargo:warning`），消息里带着可直接粘贴的 `static_graft_plan!` 子句。本次构建里
  `#[cfg]` 关掉的声明仍然算数，因此合法门控的槽位不会被上报；
- `apply_recorded_grafts` 为每条报告打印一行 `warning:`/`note:`，被跳过的记录因此不可能
  沉默；
- 从靠后的名字写到靠前的名字的兄弟区间会被拒绝，并说明顺序规则（`registry_name` 顺序），
  而不是静默交换。
- `nichlink-plugin-host` 的进程后端现在真正执行它声明的限制：请求帧由独立线程写入，stdout 与
  stderr 各由独立线程排空，超时覆盖整个调用，因此 `ProcessLimits::max_output_bytes`（默认
  1 MiB）就是真实上限。此前真实上限是约 64 KiB 的管道缓冲，超过它报的是 `Timeout`。
- `petgraph` 从 0.6 升到 0.8。它只支撑 `CallGraph` 的私有字段，因此不改变任何公开 API。
- `studio/src/studio/ui/forms.rs` 改为像它的四个兄弟一样用 `#[path]` 挂载 `face_fields`。这是
  风格差异而不是缺陷：两种写法都能解析，因为经 `#[path]` 载入的父文件按目录解析子模块。
- Studio、authoring 执行器与 MCP 桥改为通过同一条内核规则解析项目，而不是三份副本。行为
  不变；唯一有意的差异写在它所在之处：桥的最后兜底仍是当前目录，因为 stdio 桥是在代理所处理
  的项目里启动的，而编译进去的清单路径属于构建该二进制的那台机器。
- `wasmi` 从 0.42.1 线移到 `1.0.9` 下限（锁文件解析到 1.1.0）。0.x 的 caret 要求永远到不了
  对沙箱真正重要的那些修复——内存读写的整数溢出、内存增长越过系统上限、`rem_s(MAX, -1)`
  在本该返回 0 时 trap、以及 loop 局部变量与 wide-arithmetic 的错误编译。只有一处调用点变化：
  `Linker::instantiate` 加 `PreInstance::start` 变成 `instantiate_and_start`，它运行同一个
  `start` 函数，燃料也仍在其之前设定。燃料仍是 2.0 之前那一类，因此 `fuel_per_call` 的含义
  不变；其文档现在说明单位属于引擎，大版本升级可能重新标定它。
- 五条重复规则各归并为一份实现：清单写入方与 CLI 报告共用的切口端点渲染、CLI 的 JSON 错误
  文档与命令错误文本、Studio 的计划路径（改用运行期加载器的 `graft_record_root`）、PascalCase
  的 kind 推导（改用内核的 `pascal_case`）以及按 `::` 边界匹配符号（改用内核的 `same_symbol`，
  现已公开）。其中两条此前已经分叉。
- **注册面作者面现在只有作者真正要决定的东西那么大。** `handle` 与 `params` 按规则就是
  kind，`registry_name` 就是声明模块路径的末段，因此 `__control_object!` 与
  `__external_object!` 各自只剩一条 arm，同一个类型的三种拼写从声明文件里消失；最小注册面
  是 `control_object! { kind: Button }`。trait 标签改由"编译器检查的契约路径"按**一条规则**
  推导，三个读取方共用它（`FaceSyntax::string_list` 的文件视图、过程宏
  `__face_trait_labels_or!`、写入方 `apply_trait_contract`），因此标签不可能与编译器检查的
  路径不一致。`FACE_FIELD_ORDER` 从 29 个可接受键收到 24 个。
- **Studio 新增/编辑表单是 26 格而不是 30 格**：22 行作者输入，加上收尾的一条只读"机器取值"；
  槽位在 `authoring::face_field` 具名，因此 Studio 的写入路径不再用裸字面量索引数组。有四个
  槽位随它们描述的字段一起消失。凡是与代码相反的帮助文本都已改正：树槽位声称可以覆盖，而
  registry_name 规则不允许；`external source note` 对一个人人参与注册机解析的字段写"仅来源
  元数据"（现改名为 `dependency registry`）；kind/parts 两行标着"推导"却可编辑。
- call report 对同一个事实只说一次：`handle=` 曾与 `kind=` 并排在每个函数上打印，搜索也曾把
  `params` 当作同一字符串的第二个键。按规则两者都是 kind，因此现在只打印 kind、只搜索 kind；
  `RegistrationInfo` 为读取它的宿主保留这两个字段。
- 有类型的嫁接切口现在跨它连接的两个面证明输出类型：构建会在 `BUILTIN_GRAFT_CUTS` 旁发出
  `assert_contract::<{cut}::__Preset, {graft}::__Parts>()`，因此替换面的 parts 若不提供 preset
  要求的 parts，构建就会失败。字符串与选择器切口不发断言，这一点写在填充累加器的地方，而不是
  退回已删除的字符串比较。

移除：

- 作者书写的 `expected_output` / `actual_output` 对。它只在两处与**自己**比较过
  （`release.rs` 与 `build_method::check_output`），从未描述真实类型：button 面写着
  `"ControlFrame"`，而它的 `NoPreset`/`NoParts` 输出是 `()`。它假装陈述的类型事实现在由按面的
  `assert_contract` 与上面的按切口断言证明。字段、两处比较、缓存键、创作侧管线与 Studio 槽位
  一并删除。
- `authoring::FACE_FIELD_NAMES`：一张只有测试读、且与展示标签近乎重复的 30 行标签表。布局的
  名字现在住在 `authoring::face_field`，原来用这张表的测试改为断言更强的性质：槽位稠密，且每个
  都有展示元数据。
- Studio 为同一个事实打印多遍的行：搜索详情面板里的 `values` 运行期快照行与 `declared`
  file:line:function 行（文件已经由 `path` 行给出），以及两个详情面板里重复 `kind` 行的
  `params`/`handle` 行。`params`、`handle` 与 `kind` 是同一个字符串：两个宏都用
  `stringify!($kind)` 展开它们。

修复：

- 进程后端不再有无限阻塞。三处都经实测后移除：从不排空 stdout 的 `try_wait` 轮询（写得超过
  管道缓冲的健康子进程会被杀掉并报成 `Timeout`，实测边界是 65 536 字节的帧成功、65 537 字节
  超时）；发生在超时启动之前的 `stdin` 写入（不读 stdin 的子进程会把调用方钉在超时之外，1 MiB
  阻塞了整个子进程生存期后报管道中断）；以及位于超时之外的帧读取。五条回归测试钉住这些边界。
- 钉住 `edit_module` 删除的 `compile_fail,E0433` doctest 此前没有任何命令编译它——唯一带
  `--all-features` 的任务传了 `--all-targets`，而它会完全跳过 doctest。现在会被编译。
- `MirGraph::to_jsonl` 对含控制字符的名字会原样写出该字符，产出严格 JSON 读取器一律拒绝的
  工件，而本 crate 自己的宽松解析器仍然接受它。修复前已复现（函数名里的制表符让两行输出都
  非法），修复后用独立的严格解析器验证。脚手架片段写入器给 VS Code JSON 用的也是同一份弱
  转义；两者现在都用内核编码器。
- `tools/nichlink-package-audit` 用一个按空白切分的循环读取 crate 的依赖表，因此
  `nichlink-studio` 与 `nichlink-cli` 只按第一个依赖被评估，摘要里还列出了并不存在的 crate。
  依赖改为逗号分隔并据此解析，摘要对每个 crate 只列一次。
- 进程适配器在多个插件调用并发时不再报出假失败。`exec` 可能瞬时以 `ETXTBSY`
  （"Text file busy"）拒绝一个刚暂存的可执行文件，在并行测试中表现为 `ExecutableFileBusy`；
  现在派生会有界重试，其他错误仍原样上报。
- 被 `#[allow(dead_code)]` 与 `#[allow(unused_imports)]` 压住的死代码已清除：只写不读的
  `RegistryHeader::name` 与其无人调用的 `admission()` 访问器（连同藏起它们的模块级 allow）、
  `EntryPages::iter()`、`FaceManifest::get()`/`fields()`、`SearchRow::source_index`（写了四次、
  一次也没读）、无人引用的 `state::FACE_FORM_ORDER` 重导出、一个其实有调用者却被标了
  `#[allow(dead_code)]` 的函数，以及约四十条未使用的 import。**删除 allow 而不是保留它**，
  正是最后这一组的发现方式：rustc 会点名真实的集合。
- 九个已发布 crate 现在都开启 `#![warn(missing_docs)]` 并补全了各自的公开面文档（九个
  crate 合计约 700 项），因此后续发布无法再引入没有文档的公开项。开启过程中发现的两处下游
  警告一并修掉：`__registration_face!` 发射的 `NODE_ID`/`REGISTRATION`，以及构建生成的
  `builtin_static_plan()` / `registrations()` / 容器模块，现在都带文档，宿主开启该 lint 后
  不会再收到指向它无法编辑的生成代码的警告。
- `nichlink-dev` 不会再以坏掉的状态被安装：它位于非默认 `dev-supervisor` 特性之后，按
  同级目录、`PATH`、工作区 `target/debug` 的顺序解析 Studio 二进制，并在它被编译时所在的
  检出已消失时报出该路径。
- `apply_recorded_grafts` 对三种没有合法解读的产物改为拒绝而不是跳过：解析不了的计划
  （一次报出全部不可读计划）、目录与计划里的 `graft` 不一致的记录
  （`RecordReport::SelectorDirectoryMismatch` 已删除）、以及身份与路径指向不同面的记录。

### [0.1.0] 首次发布（2026-09-23）

九个 crate 按依赖顺序一同发布：`nichlink-core` → `nichlink-macro` /
`nichlink-build-method` / `nichlink-mcp` → `nichlink-run-method` →
`nichlink-debug-method` / `nichlink-plugin-host` → `nichlink-studio` →
`nichlink-cli`。依赖版本要求写 caret `0.1.0`，因此补丁版本不会连锁要求依赖方重发。

各 crate 职责见上方英文列表。本版本线的已知边界见根 `README.md` 与
[`docs/threat-model.md`](docs/threat-model.md)；kind-only `registry_name` 派生等
随版本变化的行为记录在 [`docs/roadmap-1.0.md`](docs/roadmap-1.0.md)。
