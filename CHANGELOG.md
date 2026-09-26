# Changelog

English | [简体中文](#简体中文)

All notable changes to the NichLink workspace are recorded in this one file.
The nine crates are released as a single version line, so a change is described
once here instead of nine times: **per-crate changelogs are deliberately not
kept**. Release order and the reasoning behind the one-line release are in
[`docs/roadmap-1.0.md`](docs/roadmap-1.0.md).

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Release state:** `0.1.0` is published. All nine crates went to crates.io together
on 2026-09-25, and the release workflow's last step built a throwaway consumer
outside the checkout and resolved all nine by version. `0.1.1` followed on
2026-09-26, the same way and with the same last step; `tools/nichlink-package-audit`
now builds all nine packaged tarballs instead of skipping the eight whose
requirements had not reached the index. `0.1.3` followed on 2026-09-26 (run
`36236583886`, with `--verify-consumers` green in the same run); `0.1.2` was never
published, so its changes shipped in `0.1.3`, and the audit is back to `verified:`
all nine with `skipped: none`. The version line stays on
**0.1.x** while the design is still being deepened: each later release is a small
step (`0.1.4`, …), and "1.0" names the milestone in
[`docs/roadmap-1.0.md`](docs/roadmap-1.0.md) rather than a published version.
Raising the line to `1.0.0` is a separate decision that would move every internal
`version = "0.1.3"` requirement with it, and that step is what freezes the public
surface. The third-party audit's fixes below moved the workspace version and every
internal requirement together, which is what lets a cross-crate API change ship
without a red package audit; `0.1.3` moved the same way and for the same reason
(`mcp` now uses `nichlink_build_method::face_views`), and publishing it closed the
waiting state that change opened.
**发布状态：** `0.1.0` 已发布。九个 crate 于 2026-09-25 一同上了 crates.io，发布工作流的最后
一步在本检出之外构建了一个一次性消费者，按版本解析到全部九个。`0.1.1` 于 2026-09-26 以同样的
方式跟进、同样有最后一步；`tools/nichlink-package-audit` 现在会构建全部九个包的 tarball，而不再
跳过那八个依赖尚未进入 index 的 crate。`0.1.3` 于 2026-09-26 跟进（run `36236583886`，
`--verify-consumers` 在同一次运行里通过）；`0.1.2` 从未发布，它的改动随 `0.1.3` 一起走，包审计
也回到九个全部 `verified`、`skipped: none`。设计仍在深化期间，版本线保持
**0.1.x**：其后的每次发布都是小步（`0.1.4`……），而"1.0"是
[`docs/roadmap-1.0.md`](docs/roadmap-1.0.md) 里的里程碑名，不是已发布的版本。把版本线抬到
`1.0.0` 是另一个决定，需要连同每一处内部 `version = "0.1.3"` 要求一起移动——那一步才是冻结
公开面。下面三方审查的修复把工作区版本与每一处内部要求一同移动，这正是让一次跨 crate 的 API
改动得以随版本发布、而不让包审计变红的原因；`0.1.3` 以同样的方式、同样的理由移动（`mcp` 现在
使用 `nichlink_build_method::face_views`），而把它发布出去，正是关掉那次改动打开的等待态。

## [Unreleased]

### Changed

- `mcp`'s READMEs say which path `nichlink.apply`'s `node`/`parent` take: the
  *registry* path `nichlink.registry` reports (`root/control/button`), not the
  file's (`control/object/button/button.rs`). Aiming with the file-shaped one is
  refused by name, which a play session against a scaffolded host walked into.

### Fixed

- A preview no longer reports the operation as done. `nichlink.apply` runs the real
  operation on a throwaway copy, and the executor describes what it did in the past
  tense, so a delete preview printed `would move …` and then, in the same reply,
  ``moved `button` to …`` — a sentence true of the copy and false of the project.
  The reply now scopes that sentence (`preview effect: …`) rather than dropping it,
  because it carries the parent identity and the old-to-new module names the diff
  does not; an apply still prints it bare. Found by playing the bridge against a
  scaffolded host: the real project had no trash entry and the reply said it had
  moved one.

## [0.1.3] — 2026-09-26

### Added

- The MCP bridge's write path, `nichlink.apply`: an agent can create or rewrite a
  registration face through the **same authoring executor Studio uses**, so the
  kernel's admission, parent-rule, and topology checks run on the change instead of
  being re-implemented in the bridge. `mcp` gained a `nichlink-run-method`
  dependency (feature `authoring`) for it; the version line stays `0.1.3`, which is
  still unreleased, so no new symbol needed a new version to be distinguishable
  from a published one. `action` is `add` or `edit`, `parent` takes a logical path
  (the one `nichlink.registry` reports) or an identity, and **a request is previewed
  unless `apply: true`**: the preview runs the real operation against a throwaway
  copy of the package — copying, rather than writing and reverting, is what cannot
  leave a half-edited tree behind — and returns the file diff plus the registration
  tree that results; `apply: true` writes to the project and names the files it
  wrote and anchors the declaration it produced as `<path>:<line>`, the same shape a
  refusal uses for its `file:line:column`. Both ends of the preview are the same
  `run` call, which is why a preview cannot drift from the apply. `rename` changes `fields.module` and `delete` moves
  the module into NichLink's recoverable trash; both reuse the same preview. And
  `edit` is a **patch**: the face is read back first (`authored_face`, new public
  API), the request's fields are overlaid, and the complete set is handed to the
  executor — the executor's own contract is "rebuild from what you are given",
  which is right for an editor showing every field and wrong for an agent asking
  for one change, because it would blank the other twenty-two. Graft writes,
  plugins, project scaffolding, tree diffs, and the consistency analysis are still
  to come (`docs/roadmap-1.0.md` item 7). The executor's boundary is inherited
  unchanged and now documented: it rewrites the faces **NichLink generated** and
  refuses a hand-written one (`this module was not generated by NichLink`), because
  rewriting a file it did not author would discard content it does not model.
- `nichlink_build_method::source_layout` and `SourceLayout` are public API: the
  write path needs the same answer the build uses about where a package's faces
  live, and guessing `src/` there would author a face the build never reads.
- `nichlink-mcp`'s `nichlink.registry` tool: the registration faces this package
  declares, one row per face with its logical path, kind, source, and the
  `NodeId` the host compiled. The rows are derived by
  `nichlink_build_method::face_views` — the same derivation the CLI's `explain`
  uses — so `mcp` gained a `nichlink-build-method` dependency and the version line
  moved to `0.1.3`. This closes a standing dishonesty in the bridge: it
  registered only five source-text tools while its documentation had advertised a
  registry query, so an agent re-derived the tree by grepping for macro names.
  The namespace is the part that hides a real design question: a package name
  *is* the `NodeId` namespace, so the tool resolves `NICH_LINK_NAMESPACE` first,
  then the name Cargo reports, and otherwise **refuses** — it never falls back to
  `nichlink.default`, because every id below such an answer names a node no host
  compiled. Contract, admission, and registration-rule data still need the built
  face snapshots and are still absent from the bridge.
- A library target outside `src/` is read where it is. A package whose manifest
  declares `[lib] path = "host/lib.rs"` now has its registration faces read from
  `host/`, and each face's identity path is `host/…` — the manifest-relative path,
  because the declaration macros drop one leading `src/` and nothing else. The two
  decisions that had been one hardcoded constant, `<root>/src`, are now two bases
  resolved once (`build_method/src/source_layout.rs`): the tree the walk reads and
  the base an identity path is relative to; `relative_display` and the
  `src.join(relative)` inversions keep working, and entry resolution — the one
  place that needs both — takes the layout. A `[lib] path` naming no file is a
  `face-layout` diagnostic naming it instead of a walk of some other tree; the
  manifest read is deliberately narrow (one key, one table, quotes, comments, the
  dotted `lib.path = "…"` spelling) and never spawns `cargo`, because the pipeline
  runs inside a build script. `[[bin]]` targets do not move the root: a package may
  have several and nothing chooses between them. `build_method/src/entry.rs` came
  back **under** the 450-line ceiling on the way (its conventional-entry choice
  moved to `entry_default.rs`), so its size-ratchet entry is deleted rather than
  enlarged.
- `nichlink_build_method::package_name`: the Cargo-authoritative package-name
  read moved out of `nichlink-cli`, so the command line and the MCP bridge ask one
  authority instead of each carrying a copy. The CLI's commands behave exactly as
  before, and the two tests that pinned the private copy moved with it. Nothing in
  the build pipeline calls it: a build script receives `CARGO_PKG_NAME` from Cargo,
  which is the same value.
- `nichlink-plugin-host`'s `PluginAdmission`: the host-side path from the plugin
  locks to a loadable artifact. It reads
  `<package_root>/.nichlink/plugins/{official,user}.lock`, selects the manifest
  against that catalogue, verifies it (`verify_signed` for an official source,
  `verify_artifact` for a user one), and names the lane the earned assurance
  buys; `install` (Wasm) and `load_process` (process) do admission and loading in
  one step. Before this, **no in-tree code outside `core` called
  `verify_signed`**, so the Official lane was unreachable.
- Studio's trace **loader**, the consumer half of the artifact above:
  `TraceStatus { Absent, Loaded, Mismatch { reason } }` and `App::install_trace`,
  run once at startup. Discovery is `trace_artifact_path`, and four identity
  checks decide whether the artifact belongs to this session — the parser refuses
  an unsupported layout version, then the namespace, the registry root, and every
  recorded node that no longer resolves. A matching artifact puts the recorded
  values in the DATA panel; a refused one installs nothing, keeps the registry
  visible, and states the reason on the event line.
- `nichlink-plugin-host`'s process adapter can narrow the child:
  `ProcessLimits::inherit_env` (default `true`) clears the environment,
  `ProcessProgram::environment` names the variables the child may then see, and
  `ProcessProgram::current_dir` chooses where it runs.
- The trace artifact's first slice in `nichlink-run-method`: `TraceArtifact`
  (`from_trace`, `render`, `parse`, `into_trace`), `write_trace_artifact`,
  `read_trace_artifact`, and `trace_artifact_path`, plus the
  `TRACE_DIR`/`TRACE_FILE`/`TRACE_FILE_ENV` contracts in the kernel lexicon. A
  host can now write its recorded `CallTrace` as a versioned document that a
  separate reader can rebuild; the Studio loader is the next slice
  (`docs/design-trace-ingest.md` §3.5). `write_trace_artifact` takes the host's
  identity namespace as a parameter — for a host crate that is
  `env!("CARGO_PKG_NAME")` — because the process cannot read its own compiled
  namespace back and the reader's `NICH_LINK_NAMESPACE` is not it. The document is
  line-oriented text:
  values escape `\\`, `\t`, `\n`, `\r`, no raw control character reaches the
  file, and a malformed or unknown-key document is refused by line rather than
  guessed.

### Changed

- `nichlink-build-method` depends on `serde_json` for `package_name`. It is the
  only JSON the crate reads, and `syn` was already there, so a host's build script
  pays for one more leaf rather than a second implementation of the rule.
- Documentation claims re-measured and corrected for the tool set and the trace
  loader: the root READMEs and both discussion introductions still said Studio
  rendered a built-in sample trace and had no ingest path, and that the bridge had
  no registry query. The MCP tool lists now include `nichlink.registry`, and the
  trace sentence says Studio loads the artifact its host writes and reports
  `TRACE: none` without one.
- Studio's built-in trace sample is gone. With a loader in place it could only
  mislead, so the legend reads `TRACE: none` when nothing is loaded, `LIVE` when
  an artifact passed the identity checks, and `TRACE mismatch` when one was
  refused; the DATA panel drops `· built-in sample ·` and says `no trace
  attached` or the mismatch reason. A session can no longer label evidence it
  does not have.
- Studio reads the host crate's own package name and authors under it, instead of
  defaulting to `nichlink.default`. The two ends have to agree — the host's build
  script stamps `env!("CARGO_PKG_NAME")` as the identity namespace — and a session
  that rebuilt the tree under a different name put every recorded `NodeId` (a
  trace, a graft record) out of reach. The name now comes from Cargo
  (`nichlink_build_method::package_name`), the same authority the CLI and the MCP
  bridge use, rather than from a literal `[package] name` line scan: TOML's dotted
  `package.name = "x"` form is the same table without a `[package]` header, so the
  scan found nothing and Studio authored such a project in the wrong identity
  domain. `NICH_LINK_NAMESPACE` still overrides everything verbatim, and a
  manifest Cargo names no package for (a virtual workspace root) still falls back
  to the documented default — authoring creates a tree, so a default is meaningful
  there.
- Studio's hand-drawn call-tree canvases are gone: `rataflow` (the `node-graph`
  feature, default on) is the only call-tree drawer, 1756 lines of canvas code
  and their geometry tests went with them, and the `g` (drawer) and `v` (layout)
  keys no longer exist. A build without `node-graph` keeps the panel and says
  which feature the tree needs instead of drawing nothing.
- Forwarding a mouse *release* to the `rataflow` widget, which emits
  `NodeClicked` on release while Studio only forwarded the press — so widget
  click-to-select could never fire. It does now.

### Fixed

- Studio derived the wrong identity namespace for a TOML manifest that names its
  package in the dotted `package.name = "x"` form: that is the same table as
  `[package]`, a line scan sees no header, and the session fell back to
  `nichlink.default` while the host compiled under the real name — so the same
  faces existed in two identity domains and a trace or graft record written on one
  side could not resolve on the other. The literal reader is gone and Studio takes
  Cargo's answer, one subprocess per project adoption (the adopted namespace is
  cached in the project context, so no write path pays for it).
- The trace artifact's writer creates the directory it writes into. The
  documented host usage pairs `write_trace_artifact` with `trace_artifact_path`,
  and on a fresh project that path's `.nichlink/traces/` does not exist yet: the
  write failed with "No such file or directory", and the message named the
  writer's own temporary file instead of the missing directory. Creating it is
  what the authoring executor's writer already does for
  `.nichlink/external-grafts/`. The `run_method` READMEs now document the
  writer/reader pair and the `NICH_LINK_TRACE_FILE` override — the public API had
  no host-facing page, because the design document is not in the package.
- Studio's writers require the project the reader opened. `package_root()` still
  falls back — the session, then the environment, then the working directory — and
  that guess is reasonable for a read, because reading the wrong tree is visible.
  A write cannot afford it: a delete once resolved its root that way and moved a
  module out of whichever project the environment named. Every writer now goes
  through `with_selected_project` (or `selected_package_root`), which refuses with
  a named error when no project is selected, so no future call site can repeat the
  bug by forgetting to establish a context. Reads keep the fallback, and a launched
  session still adopts its project before the terminal is taken over, so nothing
  user-visible changes.
- `nichlink-build-method` no longer aborts when the source tree is missing. A
  package whose `[lib] path` points outside `src/` — a legal Cargo layout — used
  to reach `expect("src directory must exist")` inside discovery: the build script
  died with exit 101 and `check --json` printed nothing at all, which is the
  contract that command keeps. It is a `face-layout` diagnostic now, naming the
  tree it looked for: structured callers get it back (`check --json` writes the
  document) and the build script fails loudly with the rendered text. A directory
  that disappears mid-walk is reported the same way instead of panicking.
  `check_for`'s doc now also says its first parameter is the package **root**,
  which is what every call site already passed.
- The MCP bridge's request framing is bounded: a line past `MAX_REQUEST_BYTES`
  (1 MiB) is refused with `-32600` and its remainder drained, so a hostile or
  buggy client can no longer make the long-lived stdio server allocate without
  limit. The drain stops at the newline — a plain read into a scratch buffer took
  the next frame with it, because a slice or a pipe hands over as much as fits.
  Requests must also carry `"jsonrpc":"2.0"`, and a member that is not a JSON
  object is answered with `-32600` rather than dropped silently; notifications
  stay silent even when their envelope is wrong, as the specification requires.
- `PluginCatalog::contains_manifest` compared the three optional provenance
  fields — signature, key fingerprint, revocation-list snapshot — for equality,
  so a seven-field official record (what a host's own plugin UI writes) could
  never match a signed official manifest, and an official plugin could not be
  admitted through a lock the host itself wrote. They are expectations now: a
  record that carries one pins it, a record that omits it leaves it to the
  signature check.
- Documentation claims re-measured and corrected: the in-flight design said
  Studio's sample records no locals (it records one, pinned by its own test), the
  post-fix audit's items 21/22/26/27/28 and its `build.rs` scope finding are fixed
  and now annotated as such, and the roadmap no longer calls the first publish the
  only remaining 1.0 item.
- The bridge's own descriptions still called it read-only after the write path
  landed: the `mcp` crate docs, four module headers, the manifest `description`,
  `nichlink mcp` in the CLI's usage text, and four places in each of the two
  READMEs. Each now says what the bridge does — source and registry queries plus
  previewed authoring writes — so a `--help` line and a docs.rs page no longer
  contradict `nichlink.apply`.

## [0.1.1] — 2026-09-26

### Added

- `deny.toml` and a `cargo-deny` CI job covering advisories, licenses, and bans.
- A CI job that compiles the non-default feature set and the feature-less
  surface: `cargo test --workspace --all-features --all-targets` and
  `cargo check --workspace --no-default-features --all-targets`. This is what
  first compiles `studio`'s `prototype-fixtures` tests, and it re-checks
  `plugin-host`'s `process-tools` through `--all-features`.
- A `LICENSE` copy inside every published crate directory, so each published
  package carries its own licence text (Cargo only auto-includes a crate-local
  `LICENSE*`). `conventions/` has none and needs none: it is `publish = false`.
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

- `tools/nichlink-publish --verify-consumers`, the half of a release rehearsal
  that only the published crates can answer: it builds a throwaway crate outside
  the checkout and `cargo add`s all nine by version, so resolution runs against
  the index rather than against local paths, then `cargo check`s them together. A
  version the index does not serve yet is reported as exactly that, with the
  command to run after publishing, instead of as a failure of the script.
- `tools/nichlink-publish --check-table`, which compares the tool's dependency and
  level tables against the manifests that decide the real publish order: manifests
  against the dependency table, the dependency table against the level order, and
  no crate listed before something it depends on. It needs no network, so the
  `features` CI job runs it on every push as well as the release workflow.
- `.github/workflows/release.yml`, triggered by a `v*` tag: it stops before
  uploading when the tag does not name the workspace version, then runs the
  all-features battery, `--check-table`, the package audit and the release-artifact
  audit, and only then publishes in dependency order and verifies that a consumer
  outside the checkout can resolve the result. `workflow_dispatch` runs every
  check without publishing, which is how the first release can be rehearsed
  before the tag exists.
- The nesting guard now measures the shape that took a second pass to find: a
  **linear token run** folded into one nested expression or type with no delimiter
  or angle bracket to count. Seven shapes still aborted the process before this —
  `& & & …`, `&mut …`, `&'a …`, `* * * …`, `- - - …`, `! ! ! …`, `|| || …`, and
  `1 + 1 + …` — and the last one is why the guard is about the *tree* rather than
  only the parse: a binary chain is parsed by a loop, but the left-nested
  `ExprBinary` it builds recurses once per operator in `Drop`, so the process died
  on the way out of a parse that had succeeded. Runs are separated by `,`, `;` and
  brace groups (parentheses and brackets do not separate one, so `x.f().f()…` is
  counted), the limit is 1024 tokens — sixteen times below the smallest run
  measured to survive a 256 KiB stack — and `core/tests/nesting_budget.rs` feeds
  every one of the workspace's 318 Rust files to the guarded entry point to prove
  the limit refuses nothing this repository ships. `deep_input_tests.rs` carries
  one case per shape.
- `examples/control-button/tests/static_plan_allocations.rs`, which measures the
  release read path's heap allocations with a counting global allocator instead of
  inferring them from the code shape. Reading the built-in plan allocates nothing
  (0 allocations, 0 bytes across `builtin_static_plan()`, both slices, `find` hit
  and miss, `children_of`, and a full walk), and `overlay_static` — not free, since
  it clones the tree and builds its visited-cut set — builds no `GraftPlan` and
  costs less than the dynamic spelling of the same cuts (60 allocations against 77
  for one cut, 107 against 138 for two). The file holds exactly one test because
  the counter is process-global and libtest threads tests separately.
- `tools/nichlink-external-rehearsal`, which makes the external-path rehearsal
  reproducible: it copies the two example hosts outside the checkout, repoints
  their path dependencies here, detaches them from the workspace and builds and
  tests them from scratch in their own `target/`. The example host's 27 tests pass
  there, and the built CLI checks that external project (`check: ok`). It is what
  catches a host layout assumption that only holds in-tree, and CI runs it on one
  matrix cell.
- `plugin-host/tests/wasm_table_cost.rs`: what a Wasm table actually costs the
  host, measured with a counting global allocator instead of estimated. A
  function reference is 8 bytes, so the default `table_elements` ceiling of 4096
  is 32 KiB, the hundred-million-entry table `a_huge_table_is_refused` pins would
  have been 762 MiB, and the limiter refuses that module after 5 225 bytes of peak
  allocation because it denies the table before it exists. The same check settled
  which limit does the work: wasmi's `EnforcedLimits::strict()` caps how many
  tables and element segments a module may declare, not how large one table may
  grow, so `WasmLimits::table_elements` is the only bound on table size.
- Two observation tests in `studio/src/studio/app/tests/evidence.rs` that replace
  an argument about `CallEvidence::Live` with a measurement: with the shipped demo
  trace installed, no edge of a real project classifies as `Live` — and the test
  asserts that the trace is installed and that its nodes belong to no node of the
  loaded project, so it cannot pass by having no trace at all. Install a trace
  over the loaded faces and the same enumeration reports exactly the edge that
  trace recorded. `Live` is thus not dead code; it is unreachable for a real
  project only because Studio ingests no real trace yet.
- Budgets in `run_method/examples/scale_audit.rs`: 40 µs per node registered and
  20 µs per node indexed, roughly eight times the measured values and overridable
  with `NICHLINK_SCALE_REGISTER_US` / `NICHLINK_SCALE_INDEX_US`, so an
  order-of-magnitude regression fails the run instead of only printing a larger
  number.
- Three steps in the `features` CI job for gaps no other job covered: clippy over
  `--all-targets --all-features`, the tmux visual check (`tools/nichlink-visual
  home graph tree-demo`, where each capture must be non-empty), and a
  `tools/nichlink-publish` dry run.
- `docs/performance-baseline.md`: what registration and indexing cost at 10k and
  100k nodes, what the release artifacts cost in bytes and defined symbols, and
  the commands that re-measure both.

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

- The third-party audit recorded in
  [`docs/audit-3p-2026-09-25.md`](docs/audit-3p-2026-09-25.md): every finding it
  lists, from the kernel's stack-overflow refusals and the Studio delete guard to
  the four release-tool defects and the new repository gates. The signature
  payload now covers the `RegistrationInfo` that travels with a plugin's bytes,
  which is a public-API change — `PluginManifest::signing_payload` and
  `PluginTrustPolicy::verify_with` take the registration, and
  `PluginSignatureVerifier::verify` receives the canonical payload instead of the
  raw bytes — so it ships with this version rather than as an unreleasable edit.
- The documentation gate now parses Rust fenced in `///` doc comments, not only
  in markdown. Rustdoc skips a block tagged `ignore`, so a macro-usage example
  that can never be a doctest of its crate was shown to every reader and checked
  by nothing; the 24 face-macro examples are now tagged `rust,ignore` and 21 of
  them are parsed by the gate (the other three are shaped by a macro matcher and
  carry an explicit `macro-input` tag, with the reason stated where the gate
  reads it).
- `atomic_write` uses a unique temporary name instead of a fixed
  `<file>.nichlink.tmp`, which it used to delete before reuse — a sibling that
  happened to carry that name was destroyed.
- A graft plan with a repeated key is refused, as every other reader in the
  workspace refuses a repeated field; the second value used to win silently.
- The built-in Studio sample records one observed local, so the DATA panel can
  show a value rather than the placeholder it rendered forever.
- The nine manifests carry `keywords` and `categories`, and the two places that
  counted the `publish = false` members say three (the two example hosts and
  `conventions`). `assert_static_registration` documents its panic and its
  relationship to the runtime rule check, and `MirGraph::from_mir_text` documents
  that text which is not MIR yields an empty graph.
- The changelog no longer claims a release that has not happened, and the crate
  documentation matches the code. The `0.1.0` section said all nine crates were
  published while nothing is on crates.io; it now says the first release is not
  cut yet, and the header states the version line once — the first release is
  `0.1.0`, "1.0" is the milestone name, and raising the line means moving the
  fourteen internal requirements with it. The README's Studio keys no longer
  list a comparison page that was removed (and now list `p`), `debug_method` is
  no longer credited with running the MIR subprocess, the install section says
  plainly that only the Git source resolves today, the `cargo run -p
  nichlink-cli` commands work again (`default-run`), the MCP README no longer
  promises stderr diagnostics, and `## Boundaries` records the face-layout rule
  and the plugin-trust boundary.
- A verified plugin signature is reachable, so the official channel can admit
  anything at all. `PluginArtifact::verify_artifact` always recorded the digest
  assurance while the official channel requires the signature assurance, so the
  lane failed closed and the control the threat model names never ran. The new
  `PluginArtifact::verify_signed` runs the whole policy chain — digest,
  revocation, official key, signature — and records the stronger assurance.
  Revocation is therefore consulted before a signature is accepted.
- `nichlink-dev` reports a Studio that exits non-zero, instead of calling a crash
  a successful session.
- A Wasm plugin can no longer buy host memory through a table, and its artifact is
  bounded before compilation. The declared memory ceiling does not bound a table —
  a table is a separate, eagerly instantiated array of function references, so
  `(table 100000000 funcref)` cost hundreds of megabytes outside `memory_bytes`.
  `WasmLimits` now carries `table_elements` and `max_module_bytes`, and modules
  compile under wasmi's own strict engine limits instead of unlimited defaults.
- A process plugin that keeps writing after its answer still delivers that
  answer. The host read one frame and stopped, so a child writing past the pipe
  buffer blocked, the host waited for an exit that could not come, killed it at
  the deadline and reported `Timeout` while discarding the response it already
  held. The answer is sent on first and the rest of stdout is drained to EOF —
  the rule stderr already followed.
- The MCP bridge cannot read outside the source root it was pointed at, and it no
  longer exits zero when its transport fails. A directory symlink under the root
  was followed because the walk asked `is_dir` and the read only compared path
  prefixes; both now go through the canonical path, and a direct request for an
  escaping path is still refused by name. A read or write failure on the stdio
  transport is returned instead of ending the loop quietly, so a client that
  cannot be answered no longer sees a successful session.
- `nichlink grafts` fails when it cannot answer its question instead of answering
  part of it. An unreadable source tree, an unreadable host entry and a plans
  directory that exists but cannot be read now make the command exit non-zero
  after writing whatever was readable, because the declaration column would
  otherwise be a guess. A plan whose text is simply broken is still a successful
  report of a broken plan.
- A generated tree that cannot be written is reported rather than panicking out
  of a structured caller. A build script still stops with the reason — it has
  nowhere to render a diagnostic, because the generated tree is what would carry
  it — while `check --json` reports it as an `out-dir` diagnostic.
- A runtime check whose numeric bounds the runtime cannot state exactly is
  refused instead of answered with a guess. The comparison rounded both bounds
  through `f64`, so `number_in_range(9007199254740993, 9007199254740993)`
  accepted `9007199254740992.0`, a number below its own minimum.
- The source walk stops at a depth bound instead of overflowing the stack. It is
  the workspace's only recursive traversal, and the kernel takes its filesystem
  facts from the caller, so it cannot canonicalize a path to tell a link that
  points at an ancestor from a directory that is simply deep; 128 directories is
  far past any real layout, and a tree that reaches it is reported.
- `function_symbols` counts lines in one forward pass. It re-counted from byte
  zero for every function it found, which made the pass quadratic in the number
  of functions — the shape MCP's index walks.
- A host crate with nested faces type-checks under the IDE's `rust_analyzer` cfg.
  `rust-analyzer` applies `#[path]` only at the top level, so every nested face is
  loaded a second time as a crate-root shadow; in that shadow `super` is the crate
  root, while the rule a registry-owning face derives names a module beside its
  own file, so `E0433 cannot find registry_rule in super` failed the whole crate
  in the editor's view. The resolver now answers both tools: `rustc` still
  compiles the sibling path verbatim, and the IDE gets the fallback instead. The
  check runs in CI (`IDE mirror (rust_analyzer cfg)`), because no other step
  compiled that cfg.
- Deleting an external graft record takes two presses, and a record whose text
  does not parse can be deleted at all. One `d` used to move the directory
  immediately, and the delete path read the plan first, so a broken record — the
  one a reader most needs to remove — could never be removed. The first `d` now
  arms and names the record, any other key clears the arm, and the second `d`
  moves it. Selection writes (the plugin form) work the same way, and a failure
  on the second of its two files puts the first one back.
- A new project resolves a relative directory against the project that is open,
  not against the directory Studio was started from, and a scaffold that fails
  part-way leaves nothing behind. A pre-existing directory is not deleted: the
  error says a partial project was left in it instead.
- Studio refuses to start with no project to open, instead of showing an empty
  tree and exiting zero. The old resolution fell back to the directory this crate
  was compiled in — the checkout, or the installed crate's sources — so a launch
  from anywhere else looked healthy while the next authoring command wrote a new
  face into NichLink's own tree. The rule now takes the session's selection, then
  a path argument, then `NICH_LINK_PACKAGE_ROOT`, then the working directory when
  it holds a `Cargo.toml`, and refuses every candidate that names something
  unusable by name. `nichlink-studio` also accepts `[PROJECT]` and `--help`, and a
  failed launch prints one line and exits non-zero before the terminal is taken
  over.
- Rewriting a face keeps its previous text where the reader can find it. The
  editor rebuilds a file from the fields it models, so anything hand-added is not
  in the result; the write was atomic, but the loss was silent and permanent. The
  previous text now lands under the same `.nichlink/trash/` the delete path
  already uses, and the message names the backup path. `delete_module` moved to
  its own module in the process, which paid for the addition: the operations page
  is still inside the size ratchet.
- A misconfigured entry or scope is a diagnostic instead of a panic.
  `NICH_LINK_ENTRY` naming no file, a malformed `application!` declaration, an
  `application!` entry that is not `crate::…` or resolves to nothing, a
  `NICH_LINK_SCOPE` value with the wrong schema or an identity no node owns, and a
  malformed graft declaration in the entry all used to panic. They failed the
  build, but took the build script down with them and left `check --json`
  printing nothing at all. Each now reports a diagnostic (`entry`, `scope`,
  `graft-entry`) and falls back conservatively — Cargo's convention, the full
  tree, an empty cut table — so one run reports every problem it can find, and no
  refused configuration can silently prune faces away. The scope value is parsed
  by a pure `SourceScope::from_raw` so the refusals are pinned without mutating
  the process environment.
- An ordinary module beside the registration faces no longer fails the build. Any
  flat `.rs` under a host's `src/` — `src/helpers.rs`, say — used to panic with a
  layout message, from the host's own `cargo build` as well as from every
  `nichlink` command that reads the tree, because discovery treated such a file
  as a mislaid registration face. Discovery now asks the question the rest of the
  build asks (`parse_face` returning `Ok(None)` means "no face here"), skips
  ordinary modules in silence, and reports a file that really is a face as a
  `face-layout` diagnostic naming the path and the layout it belongs in — so
  `check --json` produces its document instead of an empty stdout. A face that
  does not parse is reported the same way (`face-syntax`), before the stages that
  decode fields, which now skip it instead of aborting the process.
- A deeply nested source no longer aborts the process. `syn` is recursive descent
  with no depth guard, and `proc-macro2` protects only its lexer, so
  `parse_faces` on tens of thousands of nested delimiters (or a
  `Vec<Vec<…>>` chain) ended in `fatal runtime error: stack overflow` — a
  SIGABRT that no `Result` can report and no `catch_unwind` can catch. The three
  `syn::parse_file` entry points now run a nesting scan first, measured on
  `proc-macro2`'s own token stream so brackets inside string literals and
  comments cannot be mistaken for nesting, with the limit at 128 — `rustc`'s own
  default `recursion_limit`. Four shapes are pinned by
  `core/src/registry_core/syntax/deep_input_tests.rs`.
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

- The repository gates no longer accept what they were built to refuse: kernel purity
  sees `std::io`/`std::thread`/`std::os` and brace imports (`use std::{env, fs}`), has a
  walk floor, and the mounting gate finds `include!` whatever delimiter it uses (on the
  kernel's masked text, so a fixture string is not a mount); the document gate reports
  an unterminated fence and covers every markdown file under `docs/` at any depth and
  every root-level one; the lint gate reads a rustfmt-folded `#[allow(…)]` and derives
  the roots that must carry `#![warn(missing_docs)]` from the tree instead of a list a
  new crate can miss; `tools/nichlink-package-audit` derives the shipped crate set and
  their internal dependencies from the manifests; `tools/nichlink-publish --check-table`
  reads internal requirements spelled `x.workspace = true` and checks the root's
  `[workspace.dependencies]` versions; and the nesting guard names the limit that
  applies to the shape it refused (1024 for a linear run, 128 for a delimiter group)
  after the generic-argument counter, which could never fire, was removed.
- `application!(entry = …)` resolves every path segment against the package tree,
  so the canonical `<dir>/<dir>.rs` layout works (`entry = crate::control` for
  `src/control/control.rs`) and a segment that resolves to nothing in the middle is
  refused instead of passing as a function name. The resolver lives in
  `build_method/src/entry_paths.rs`, split out with the filesystem walk moved to
  `discovery.rs` to keep `entry.rs` inside the size ratchet.
- `WasmLimits::max_element_bytes` bounds what a module can make wasmi materialize
  at instantiation: a passive element segment never grows a table, so
  `table_elements` never saw it, and the measured cost was about 32 bytes per entry
  against about one byte of compact encoding — a 2 000 103-byte module with two
  million entries cost 64 070 402 bytes of host memory. The payload is measured from
  the section headers before compilation, like the artifact ceiling.
- `PluginArtifact::verify_signed` refuses a source no verifier is consulted for
  with the new `PluginTrustError::SignatureLaneRequired`, instead of recording
  `Signature` assurance that nothing checked; that artifact's digest path still
  works.
- A graft selector that starts with `.` is refused by the rule the writer, the
  parser and Studio already share: `..` used to be accepted, wrote the plan to
  `<pkg>/.nichlink/graft.plan`, and was then never listed — a record the runtime
  never applied while the author saw a created plan.
- Build output is trusted only while it still describes the sources:
  `nichlink_build_method::build_output_is_current` compares the published
  `discovery.fingerprint` against a freshly computed one, the pipeline writes that
  fingerprint only on a clean run, and `explain` (and `--overlay`) reports
  `known: false` with the existing "run `nichlink check`" note when it is missing or
  stale. A failing check no longer leaves output that `explain` presents as the
  current scope.
- A malformed or unevaluable-gated `static_graft_plan!` is a `phase=graft-entry`
  diagnostic instead of a panic: `check --json` used to exit 101 with an empty
  stdout, discarding the diagnostic `scope` had already produced. The reader now
  takes the build's diagnostics collection, all three refusal kinds report through
  it, and the parse message is byte-identical to `scope`'s so the two collapse into
  one line.
- The documentation gate handed fenced Rust straight to `syn`, so a fence nesting
  tens of thousands of delimiters aborted the gate process instead of failing it —
  the same defect the kernel's parse entries had, in the one place that still had
  it. It now asks the kernel's `guard_nesting`, which is the workspace's single
  nesting measurement and is public for exactly this reason, and
  `a_pathologically_nested_fence_is_reported_not_fatal` pins that a refusal is a
  message: with the guard removed, that test aborts.
- `tools/nichlink-publish` read its own dependency table word-wise, so a crate
  with more than one internal dependency reported only the first — `nichlink-cli`
  was checked against `nichlink-build-method` alone — and the guard that refuses
  to publish a crate before its dependencies are on the index was effectively off.
  The same traversal also handed back an edge line's dependencies as crates of
  their own, so nine table lines produced fourteen nodes. Both tables are now read
  as whole lines, and `--check-table` compares them against the manifests: the
  first run found a real drift, `nichlink-cli` depending on `nichlink-core`
  directly while the table listed only the other three. The same check reads
  workspace members with `awk` rather than a `sed` address range, because a sed
  range does not test its end address on the start line: the one-line `members`
  array made the range run on to `[workspace.package]`, whose name then entered
  the member list — invisible only because no directory of that name exists.
## [0.1.0] — 2026-09-25

The first release. All nine crates went out together in dependency order on
2026-09-25, and the workflow's last step resolved all nine from crates.io in a
throwaway consumer outside the checkout. The chain was:
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

Known limits for this line are in the root `README.md`'s `## Boundaries`
section and [`docs/threat-model.md`](docs/threat-model.md). Version-dependent behaviour
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

修复：

- 预览不再把操作报告成已完成。`nichlink.apply` 在一份一次性副本上运行真实操作，而执行器用过去时
  描述它做了什么，因此删除预览会先打印 `would move …`，又在同一份回复里打印
  ``moved `button` to …``——这句话对副本成立、对项目不成立。现在回复给这句话限定作用域
  （`preview effect: …`）而不是丢掉它，因为它带着 diff 没有的父级身份与改名前后的模块名；落盘
  时仍然原样打印。这是拿桥对着脚手架出来的宿主玩的时候抓到的：真实项目里没有任何回收条目，而
  回复说它搬走了一个。

### [0.1.3] 2026-09-26

新增：

- MCP 桥的写入路径 `nichlink.apply`：代理可以经**与 Studio 相同的 authoring 执行器**创建或重写
  注册面，因此内核的准入、父规则与拓扑校验会作用在改动上，而不是在桥里重新实现一遍。为此 `mcp`
  新增 `nichlink-run-method` 依赖（`authoring` 特性）；版本线仍是尚未发布的 `0.1.3`，因此没有
  新符号需要靠新版本与已发布版本区分。`action` 为 `add` 或 `edit`，`parent` 接受逻辑路径
  （`nichlink.registry` 报告的那条）或身份，而**除非 `apply: true`，请求只做预览**：预览在一份
  一次性的包副本上运行真实操作——用复制而不是"先写再回滚"，正是它不会留下改了一半的树的原因——
  并返回文件 diff 与将得到的注册树；`apply: true` 才写入项目，给出它写下的文件，并把产生的声明锚成
  `<path>:<line>`（与拒绝时 `file:line:column` 同一种形状）。预览的两端都是
  同一次 `run` 调用，这正是预览不可能与落盘漂移的原因。`rename` 改 `fields.module`、`delete` 把
  模块移入 NichLink 的可恢复回收目录，两者复用同一套预览。而 `edit` 是**补丁**：先读回该面
  （`authored_face`，新增公开 API），覆盖请求的字段，再把完整的一组交给执行器——执行器自己的契约是
  "用你拿到的取值重建"，这对展示所有字段的编辑器正确，而对只要求改一处的代理是错的，因为那会抹掉
  另外二十二个字段。graft 写入、插件、项目脚手架、树 diff 与一致性分析仍待做
  （`docs/roadmap-1.0.md` 第 7 条）。执行器自己的边界被原样继承并已记录在文档里：它重写的是
  **NichLink 生成**的面，对手写的面以 `this module was not generated by NichLink` 拒绝，因为
  重写一个并非它创作的文件会丢掉它并未建模的内容。
- `nichlink_build_method::source_layout` 与 `SourceLayout` 成为公开 API：写入路径需要与构建相同
  的答案——包的注册面住在哪里——而在那里猜 `src/` 会创作出构建永远不读的注册面。
- `nichlink-mcp` 的 `nichlink.registry` 工具：本包声明的注册面，每个面一行——逻辑路径、kind、
  源码，以及宿主编译出的 `NodeId`。这些行由 `nichlink_build_method::face_views` 推导——也就是
  CLI 的 `explain` 所用的同一份推导——因此 `mcp` 新增了 `nichlink-build-method` 依赖，版本线随之
  移到 `0.1.3`。这修掉了桥里一处长期存在的不实：它只注册了五个源码文本工具，文档却宣称具备注册树
  查询，于是代理靠 grep 宏名重建那棵树。命名空间是这里藏着的真正设计问题：包名**就是** `NodeId`
  命名空间，因此工具先解 `NICH_LINK_NAMESPACE`，再解 Cargo 报告的包名，否则**拒绝作答**——绝不
  回落到 `nichlink.default`，因为那种答案之下的每个 id 都指的是宿主从未编译过的节点。contract、
  admission 与 registration rule 数据仍需已构建的面快照，桥里仍然没有。
- `src/` 之外的库目标就地读取。清单声明 `[lib] path = "host/lib.rs"` 的包，其注册面现在从
  `host/` 读入，而每个面的身份路径是 `host/…`——相对清单的路径，因为声明宏只去掉一个前导
  `src/`，别的都不去。过去被写死为一个常量 `<root>/src` 的两个决定，现在是只解析一次的两个基准
  （`build_method/src/source_layout.rs`）：遍历读取的树，以及身份路径所相对的基准；
  `relative_display` 与 `src.join(relative)` 那些反向拼接照常工作，而入口解析——唯一同时需要
  两者的地方——接收布局。`[lib] path` 指不到文件时是一条点名它的 `face-layout` 诊断，而不是去
  遍历别的树；这次清单读取是有意窄的（一张表里的一个键、引号、注释、点式 `lib.path = "…"` 写法），
  并且绝不 spawn `cargo`，因为管线是在构建脚本里运行的。`[[bin]]` 目标不移动源码根：一个包可能有
  多个二进制目标，没有任何东西能在它们之间做选择。顺带把 `build_method/src/entry.rs` 带回 450 行
  **上限之内**（它的约定入口选择移到 `entry_default.rs`），因此它的尺寸棘轮项是被删除而不是被放大。
- `nichlink_build_method::package_name`：Cargo 权威的包名读取从 `nichlink-cli` 移出，因此命令行
  与 MCP 桥问的是同一个权威，而不是各带一份副本。CLI 各命令行为完全不变，钉住那份私有副本的两条
  测试随它一同移动。构建管线不调用它：构建脚本从 Cargo 拿到 `CARGO_PKG_NAME`，也就是同一个值。
- Studio 的 trace **加载方**，即上面那份 artifact 的消费一半：`TraceStatus { Absent, Loaded,
  Mismatch { reason } }` 与启动时运行一次的 `App::install_trace`。发现路径来自
  `trace_artifact_path`，四条身份检查决定这份 artifact 是否属于本会话——解析器先拒绝不支持的
  版式版本，然后是命名空间、注册树根、以及每个已不再解析的已记录节点。匹配的 artifact 把记录的
  数值放进 DATA 面板；被拒绝的不装入任何东西、保持注册表可见，并把原因写到事件行。
- `nichlink-plugin-host` 的进程适配器可以收窄子进程：`ProcessLimits::inherit_env`（默认
  `true`）清空环境，`ProcessProgram::environment` 指明子进程随后能看到哪些变量，
  `ProcessProgram::current_dir` 决定它在哪里运行。
- `nichlink-plugin-host` 的 `PluginAdmission`：宿主侧从插件锁到可加载工件的那条路。它读
  `<package_root>/.nichlink/plugins/{official,user}.lock`、按这份目录筛选 manifest、校验它
  （官方来源走 `verify_signed`，用户来源走 `verify_artifact`），并给出换来的保证等级所对应的
  通道；`install`（Wasm）与 `load_process`（进程）一步完成准入与加载。在此之前**树内 `core`
  之外没有任何代码调用过 `verify_signed`**，Official 通道因此不可达。
- `nichlink-run-method` 里 trace artifact 的第一片：`TraceArtifact`（`from_trace`、`render`、
  `parse`、`into_trace`）、`write_trace_artifact`、`read_trace_artifact` 与
  `trace_artifact_path`，以及内核词典里的 `TRACE_DIR`/`TRACE_FILE`/`TRACE_FILE_ENV` 契约。
  宿主现在能把已记录的 `CallTrace` 写成带版本的文档，由独立读取方重建；Studio 的加载方是下一片
  （`docs/design-trace-ingest.md` §3.5）。该文档是逐行文本：取值转义 `\\`、`\t`、`\n`、`\r`，
  文件里不出现任何裸控制字符，畸形或含未知键的文档按行拒绝而不是猜测。

变更：

- `nichlink-build-method` 为 `package_name` 依赖 `serde_json`。这是本 crate 读取的唯一一份 JSON，
  而 `syn` 本来就在，因此宿主的构建脚本多付的是一片叶子，而不是这条规则的第二份实现。
- 重新实测并更正了工具集与 trace 加载方的文档主张：根 README 与两份讨论引言仍在说 Studio 渲染内置
  样例 trace、尚无 ingest 路径，以及桥没有注册树查询。MCP 工具清单现在包含 `nichlink.registry`，
  而 trace 那句改成：Studio 读入宿主写出的 artifact，没有 artifact 时报告 `TRACE: none`。
- Studio 的内置 trace 示例已删除。有了加载方之后它只会误导，因此图例在什么都没装入时读
  `TRACE: none`、在 artifact 通过身份检查时读 `LIVE`、在被拒绝时读 `TRACE mismatch`；DATA
  面板不再有 `· built-in sample ·`，而是显示 `no trace attached` 或拒绝原因。会话再也不能为
  自己并不拥有的证据贴标签。
- Studio 读出宿主 crate 自己的包名，并在它之下创作，不再默认 `nichlink.default`。两端必须一致
  ——宿主的构建脚本把 `env!("CARGO_PKG_NAME")` 盖成身份命名空间——而一个在别的名字下重建注册树的
  会话会让每个已记录的 `NodeId`（trace、graft 记录）都指不到东西。包名现在来自 Cargo
  （`nichlink_build_method::package_name`），也就是 CLI 与 MCP 桥所用的同一个权威，而不再来自对
  `[package] name` 的逐行字面扫描：TOML 的点式写法 `package.name = "x"` 是同一张表却没有
  `[package]` 表头，扫描什么也找不到，于是 Studio 把这样的项目创作在错误的身份域里。
  `NICH_LINK_NAMESPACE` 仍原样覆盖一切，而 Cargo 说不出包名的清单（虚拟工作区根）仍回落到文档化
  的默认值——创作是在创建一棵树，默认值在那里是有意义的。
- Studio 的手绘调用树画布已删除：`rataflow`（`node-graph` 特性，默认开启）成为唯一的调用树
  绘制者，1756 行画布代码与它们的几何测试随之消失，`g`（切换绘制者）与 `v`（切换排布）两个按键
  不再存在。未链接 `node-graph` 的构建保留面板，并说明这棵树需要哪个特性，而不是什么都不画。
- 向 `rataflow` 控件转发鼠标**抬起**事件：它在抬起时发出 `NodeClicked`，而 Studio 此前只转发
  按下，因此控件的点击选中从来不会触发。现在会。

修复：

- Studio 对以点式 `package.name = "x"` 命名包的 TOML 清单推导出**错误的**身份命名空间：那是与
  `[package]` 相同的表，逐行扫描看不到表头，于是会话回落到 `nichlink.default`，而宿主在真实包名
  之下编译——同一批面因此存在于两个身份域里，一侧写下的 trace 或 graft 记录在另一侧解析不了。
  字面读取方已删除，Studio 改为取 Cargo 的答案，每次采纳项目一次子进程（已采纳的命名空间缓存在
  项目上下文里，因此没有写入路径为它付代价）。
- trace artifact 的写入方创建自己要写入的目录。文档化的宿主用法把 `write_trace_artifact` 与
  `trace_artifact_path` 配成一对，而在一个全新的项目里那条路径的 `.nichlink/traces/` 还不存在：
  写入以 "No such file or directory" 失败，而且消息点名的是写入方自己的临时文件，而不是缺失的
  目录。创建它正是 authoring 执行器的写入方对 `.nichlink/external-grafts/` 已经在做的事。
  `run_method` 的两份 README 现在也记下了这一对写入方/读取方与 `NICH_LINK_TRACE_FILE` 覆盖
  ——这份公开 API 此前没有任何面向宿主的页面，因为设计文档并不在包里。
- Studio 的写入方要求"读者打开的那个项目"。`package_root()` 仍会回落——先会话、再环境、最后
  工作目录——而这个猜测对读取是合理的，因为读错树看得见。写入承担不起：一次删除曾这样解析根，
  把模块从环境变量指到的那个项目里搬走。现在每个写入方都走 `with_selected_project`（或
  `selected_package_root`），没有选中项目时以具名错误拒绝，因此将来的调用点不会因为忘记建立
  上下文而重演。读取保留回落；已启动的会话仍在接管终端之前采纳项目，因此用户可见行为不变。
- `nichlink-build-method` 在源树缺失时不再中止。`[lib] path` 指向 `src/` 之外的包——一种合法的
  Cargo 布局——过去会走到 discovery 里的 `expect("src directory must exist")`：构建脚本以退出
  101 死掉，而 `check --json` 什么都不打印，而那正是那条命令要守住的契约。现在是一条
  `face-layout` 诊断，点名它找的是哪棵树：结构化调用方拿回它（`check --json` 写出文档），构建
  脚本带着渲染后的文本响亮失败。遍历中途消失的目录也以同样方式报告，而不是 panic。
  `check_for` 的文档也补上了"第一个参数是包**根目录**"——每个调用点本来就是这么传的。
- MCP 桥的请求分帧有了上限：超过 `MAX_REQUEST_BYTES`（1 MiB）的行以 `-32600` 被拒并排空其余
  部分，敌对或有 bug 的客户端再也不能让这个长期存活的 stdio 服务无限分配。排空停在换行处——
  直接读进暂存缓冲会把下一帧一起带走，因为切片或管道会给到能装下的全部内容。请求还必须携带
  `"jsonrpc":"2.0"`，而不是 JSON 对象的成员以 `-32600` 作答而不是被静默丢弃；通知即使信封不对
  也保持沉默，这是规范要求的。
- `PluginCatalog::contains_manifest` 用相等比较三个可选来源字段——签名、密钥指纹、撤销列表
  快照——于是宿主自己的插件界面写下的七字段官方记录永远匹配不上已签名的官方 manifest，官方插件
  无法经宿主自己写下的锁准入。现在它们是**期望**：记录携带某个值就把它钉住，省略它就交给签名校验。
- 重新实测并更正了若干文档主张：设计文档说 Studio 示例不记录 locals（它记录一个，由该文件自己的
  测试钉住）；post-fix 审核的第 21/22/26/27/28 条与它的 `build.rs` 作用域发现都已修完，并就地
  标注；路线图不再把首次发布称作 1.0 唯一剩余项。
- 写入路径落地之后，桥自己的描述仍称自己是"只读"：`mcp` 的 crate 文档、四处模块头、清单
  `description`、CLI 用法文本里的 `nichlink mcp`，以及两份 README 里各四处。现在每一处
  都写出桥真正做的事——源码与注册树查询，加上先预览后落盘的创作写入——因此 `--help` 的一行与
  docs.rs 的一页不再和 `nichlink.apply` 自相矛盾。

### [0.1.1] 2026-09-26

新增：

- `deny.toml` 与 `cargo-deny` CI 任务，覆盖 advisories、licenses 与 bans；
- 编译非默认特性集与无默认特性表面的 CI 任务：
  `cargo test --workspace --all-features --all-targets` 与
  `cargo check --workspace --no-default-features --all-targets`。`studio` 的
  `prototype-fixtures` 测试首次因此被编译，`plugin-host` 的 `process-tools` 也经
  `--all-features` 再检查一遍；
- 每个已发布的 crate 目录内各放一份 `LICENSE`，使每个发布的包自带许可证文本（Cargo 只会自动
  包含 crate 目录下的 `LICENSE*`）；`conventions/` 没有也不需要：它是 `publish = false`。
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

- `tools/nichlink-publish --verify-consumers`：发布演练中只有已发布的 crate 才答得出来的那一半。
  它在本检出之外建一个一次性 crate，按版本 `cargo add` 九个 crate（因此解析发生在 index 上而不是
  本地路径上），再一起 `cargo check`。index 上还没有的版本会如实报成"尚未发布"并给出发布后该跑的
  命令，而不是报成脚本失败。
- `tools/nichlink-publish --check-table`：把工具里的依赖表与层表同决定真实发布顺序的清单对比——
  清单对依赖表、依赖表对层顺序、以及"没有 crate 排在自己的依赖之前"。它不需要网络，因此
  `features` CI 任务每次 push 都会跑，发布工作流也会跑。
- `.github/workflows/release.yml`：由 `v*` tag 触发。tag 与 workspace 版本不一致时在上传前停下，
  然后跑全特性门禁、`--check-table`、包审计与产物审计，最后才按依赖顺序发布，并验证本检出之外的
  消费者能解析这次发布。`workflow_dispatch` 只跑全部检查而不发布——首次发布因此可以在 tag 存在
  之前先演练一遍。
- 嵌套守卫现在量到第三种形状，它是第二轮才被找到的：**线性 token 串**——被折叠成一个嵌套表达式
  或类型、却没有定界符或尖括号可数的 token 串。在这之前仍有七种形状会打死进程：
  `& & & …`、`&mut …`、`&'a …`、`* * * …`、`- - - …`、`! ! ! …`、`|| || …` 以及 `1 + 1 + …`；
  最后一条正是"这件事关乎**树**而不只是解析"的原因：二元链是用循环解析的，但它建出的左嵌套
  `ExprBinary` 的 `Drop` 每个运算符递归一层，于是进程在离开一次**已经成功**的解析时死掉。串以
  `,`、`;` 与花括号组为分隔（括号与方括号不分隔，因此 `x.f().f()…` 会被数到），上限 1024 个
  token——比"在 256 KiB 栈上观察到能活下来的最小串"还低十六倍——并由新增的
  `core/tests/nesting_budget.rs` 把工作区里 318 个 Rust 文件全部喂给带守卫的入口，证明这个上限
  不会拒绝本仓库出厂的任何文件。`deep_input_tests.rs` 为每种形状留了一条用例。
- 新增 `examples/control-button/tests/static_plan_allocations.rs`：用计数式全局分配器实测发布读
  路径的堆分配，而不是从代码形状推断。读内置计划完全不分配（`builtin_static_plan()`、两个切片、
  `find` 命中与落空、`children_of`、以及遍历全部面合计 0 次分配、0 字节）；`overlay_static`
  并不免费（它会克隆树并建立已访问切口集合），但不构造 `GraftPlan`，且比同样切口的动态写法便宜
  （一个切口 60 次对 77 次，两个切口 107 次对 138 次）。该文件恰好只放一条测试，因为计数器是进程
  全局的，而 libtest 会把测试放在不同线程上。
- 新增 `tools/nichlink-external-rehearsal`，让"外部路径演练"可复跑：它把两个示例宿主复制到
  检出之外、把它们的路径依赖指向这里、让它们脱离工作区，并在自己的 `target/` 里从零构建与测试。
  示例宿主的 27 条测试在那里全部通过，构建出的 CLI 也能检查那个外部项目（`check: ok`）。它抓的
  是"只在树内成立的宿主布局假设"，CI 在一个矩阵单元上跑它。
- 新增 `plugin-host/tests/wasm_table_cost.rs`：一张 Wasm 表在宿主一侧到底花多少，用计数式
  全局分配器实测而不是估计。一个函数引用是 8 字节，因此 `table_elements` 的默认上限 4096 是
  32 KiB，而 `a_huge_table_is_refused` 钉住的一亿条目表本来会是 762 MiB；限制器在峰值 5 225
  字节之后就拒绝了那个模块，因为它在表存在之前就拒绝。同一次核对还确定了是哪条限制在起作用：
  wasmi 的 `EnforcedLimits::strict()` 限制的是一个模块可以声明多少张表与多少个元素段，而不是
  单张表能长到多大，因此 `WasmLimits::table_elements` 是表大小的唯一约束。
- 新增两条运行观察 `studio/src/studio/app/tests/evidence.rs`，把关于 `CallEvidence::Live` 的
  论证换成实测：装上出厂的演示追踪后，真实工程里没有任何一条边被判为 `Live`——而且测试断言的
  是"追踪确实装上了、且它的节点不属于已加载工程的任何节点"，因此它不会因为"根本没有追踪"而
  通过。换成针对已加载注册面的追踪后，同一份枚举恰好报出那条追踪记录过的边。所以 `Live`
  不是死代码；它对真实工程不可达的唯一原因是 Studio 目前不载入真实追踪。
- `run_method/examples/scale_audit.rs` 现在带预算：注册 40 µs/节点、索引 20 µs/节点，约为实测值
  的八倍，可用 `NICHLINK_SCALE_REGISTER_US` / `NICHLINK_SCALE_INDEX_US` 覆盖，因此数量级回归会
  让运行失败，而不是只打印一个更大的数字。
- `features` CI 任务新增三步，补上没有其他任务覆盖的缺口：`--all-targets
  --all-features` 的 clippy、tmux 视觉检查（`tools/nichlink-visual home graph tree-demo`，
  每份 capture 必须非空），以及 `tools/nichlink-publish` 的 dry-run。
- 新增 `docs/performance-baseline.md`：10k/100k 节点下注册与索引的耗时、release 产物按字节与
  定义符号计的规模，以及重新测量两者的命令。

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

- 记录在 [`docs/audit-3p-2026-09-25.md`](docs/audit-3p-2026-09-25.md) 的三方审查：该文件列出的
  每一条都已修完，从内核的栈溢出拒绝与 Studio 删除守卫，到发布工具的四个缺陷与新增的仓库门禁。
  签名载荷现在覆盖随插件字节同行的 `RegistrationInfo`，这是一次公开 API 改动——
  `PluginManifest::signing_payload` 与 `PluginTrustPolicy::verify_with` 接收注册声明，
  `PluginSignatureVerifier::verify` 收到的是规范化载荷而不是原始字节——因此它随本版本发布，
  而不是作为一条发布不出去的改动留在树上。
- 文档门禁现在也解析 `///` 文档注释里的 Rust 围栏，而不只是 markdown。rustdoc 会跳过标了
  `ignore` 的块，因此一个永远无法成为本 crate doctest 的宏用法示例会给每个读者看到、却没有任何
  程序检查；24 个注册面宏示例现在标为 `rust,ignore`，其中 21 个进入门禁解析（另外 3 个的形状由宏
  匹配器决定，带显式的 `macro-input` 标记，理由写在门禁能读到的地方）。
- `atomic_write` 改用唯一临时名，不再用固定的 `<file>.nichlink.tmp`——它过去会在复用前删掉该名字，
  于是一个恰好带着它的同级文件会被销毁。
- 重复键的 graft 计划会被拒绝，正如本工作区其他每个读取者都拒绝重复字段；过去第二个取值静默获胜。
- Studio 的内置样本记录一个被观测到的局部值，因此 DATA 面板能显示数值，而不是它一直渲染的占位。
- 九个清单带上 `keywords` 与 `categories`；两处统计 `publish = false` 成员的地方改为三个（两个示例
  宿主与 `conventions`）。`assert_static_registration` 写明了它的 panic 与它和运行期规则校验的关系，
  `MirGraph::from_mir_text` 写明"不是 MIR 的文本得到空图"。
- CHANGELOG 不再声称一次没有发生的发布，crate 文档也与代码对齐。`0.1.0` 那一节曾说九个 crate
  已发布，而 crates.io 上什么都没有；现在它说首次发布尚未切割，并由头部一句话说清版本线——
  首个发布是 `0.1.0`，"1.0"是里程碑名，抬版本线要连同十四处内部要求一起移动。README 的 Studio
  键位不再列出已删除的对比页（并补上了 `p`），`debug_method` 不再被说成运行 MIR 子进程，安装段
  明说今天只有 Git 源能解析，`cargo run -p nichlink-cli` 那两条命令重新可用（`default-run`），
  MCP README 不再承诺 stderr 诊断，`## Boundaries` 记录了注册面布局规则与插件信任边界。
- 已验证的插件签名现在可达，因此官方通道终于能接纳任何东西。`PluginArtifact::verify_artifact`
  永远记录摘要等级，而官方通道要求签名等级，于是那条通道失败关闭、威胁模型点名的控制从未运行。
  新的 `PluginArtifact::verify_signed` 跑完整条策略链——摘要、撤销、官方密钥、签名——并记录更强
  的等级。因此撤销在被接受的签名之前就被检查。
- `nichlink-dev` 现在会报告以非零退出的 Studio，而不是把一次崩溃称作成功会话。
- Wasm 插件再也无法通过表买到宿主内存，且工件在编译之前就有上界。声明的内存上限并不约束表
  ——表是一块独立的、即时实例化的函数引用数组，因此 `(table 100000000 funcref)` 会在
  `memory_bytes` 之外花掉数百兆。`WasmLimits` 现在带 `table_elements` 与 `max_module_bytes`，
  并且模块在 wasmi 自身的严格引擎限制下编译，而不是默认的无限。
- 已经给出答案却继续写入的进程插件仍会交付那个答案。宿主过去读一帧就停下，于是写得超过管道
  缓冲的子进程阻塞、宿主去等一个不可能到来的退出、在超时点杀掉它并报出 `Timeout`，同时丢掉
  它其实已经拿到的响应。现在答案先送出，stdout 的其余部分排空到 EOF——这正是 stderr 早已遵循
  的规则。
- MCP 桥读不到它被指向的源码根之外，且传输失败时不再以 0 退出。根下的目录符号链接过去会被
  跟随，因为遍历问的是 `is_dir` 而读取只比较路径前缀；两者现在都经规范路径，而对逃逸路径的
  直接请求仍会按名字被拒绝。stdio 传输上的读或写失败会被返回，而不是安静地结束循环，因此
  一个服务不了的客户端不会看到"会话成功"。
- `nichlink grafts` 在答不出问题时失败，而不是只答一部分。读不了的源码树、读不了的宿主入口、
  以及存在却读不了的计划目录，现在都会让命令在写出可读部分之后以非零退出，因为声明那一列否则
  就是猜的。文本本身损坏的计划仍然是一次成功的"计划已损坏"报告。
- 写不成的生成树会被报告，而不是从结构化调用方那里 panic 出去。构建脚本仍带着原因停下——它
  没有地方渲染诊断，因为生成树正是本该承载诊断的东西——而 `check --json` 把它当作
  `out-dir` 诊断报出。
- 运行期检查中，运行期无法精确说出的数值边界会被拒绝，而不是用猜测作答。旧的比较把两个边界
  都经 `f64` 舍入，因此 `number_in_range(9007199254740993, 9007199254740993)` 会接受
  `9007199254740992.0`——一个低于它自己下限的数。
- 源码遍历在深度上限处停下，而不是栈溢出。它是 workspace 唯一的递归遍历，而内核的文件系统
  事实来自调用方，因此无法 canonicalize 路径来区分"指向祖先的链接"与"确实很深的目录"；
  128 层远超任何真实布局，达到它的树会被报告。
- `function_symbols` 用一次前向扫描数行。它过去为找到的每个函数都从第 0 字节重数一遍，使这趟
  遍历的复杂度与函数个数相乘——而 MCP 的索引正是按那种形状遍历的。
- 含嵌套面的宿主 crate 现在能在 IDE 的 `rust_analyzer` cfg 下通过类型检查。`rust-analyzer`
  只在顶层应用 `#[path]`，因此每个嵌套面都会被第二次载入为 crate 根影子；影子里的 `super` 是
  crate 根，而拥有注册机的面派生的规则命名的是它自己文件旁边的模块，于是
  `E0433 cannot find registry_rule in super` 让整个 crate 在编辑器视图里失败。解析器现在为两种
  工具各答一次：`rustc` 仍然逐字节编译那条同目录路径，IDE 拿到的是兜底值。这项检查进了 CI
  （`IDE mirror (rust_analyzer cfg)`），因为此前没有任何步骤编译过这条 cfg。
- 删除一条外部 graft 记录需要两次按键，而文本解析不了的记录现在也删得掉。过去一次 `d` 就
  立刻移动目录，且删除路径先读计划，因此坏记录——读者最需要删掉的那一种——永远删不掉。现在
  第一次 `d` 只进入待删状态并点名该记录，任何其他键解除，第二次 `d` 才移动。选择类写入（插件
  表单）同样如此，而它两个文件中第二个写失败时会把第一个恢复回去。
- 新建项目的相对目录相对**打开的项目**解析，而不是 Studio 启动时所在的目录；脚手架中途失败
  不再留下任何东西。本来就存在的目录不会被删除：错误会说"里面留下了部分项目"。
- 没有可打开的项目时 Studio 拒绝启动，而不是显示一棵空树并以 0 退出。旧的解析会回退到本
  crate 编译时所在的目录——检出目录，或已安装 crate 的源码——因此从别处启动看起来一切正常，
  而下一条创作命令会把新注册面写进 NichLink 自己的树里。规则现在依次取：本会话的选择、路径
  参数、`NICH_LINK_PACKAGE_ROOT`、持有 `Cargo.toml` 的当前目录，并把每个指不到东西的候选按
  名字拒绝。`nichlink-studio` 另外接受 `[PROJECT]` 与 `--help`；启动失败会打印一行并在接管
  终端之前以非零退出。
- 重写注册面时把先前的文本留在读者找得到的地方。编辑器用自己建模的字段重建文件，因此手工
  加进去的内容不在结果里；写入本身是原子的，但那次丢失既静默又永久。现在先前的文本落在删除
  路径本就使用的 `.nichlink/trash/` 下，且消息点出备份路径。顺带把 `delete_module` 拆成独立
  模块，正好付掉这次新增的代价：operations 页仍在尺寸棘轮之内。
- 入口或范围配置错误是诊断而不是 panic。`NICH_LINK_ENTRY` 指不到文件、畸形的
  `application!` 声明、不以 `crate::` 开头或解析不到的 `application!` 入口、schema 不对或
  含有任何节点都不拥有的身份的 `NICH_LINK_SCOPE` 取值、以及入口里畸形的 graft 声明，过去
  全都会 panic：构建确实失败了，但它把构建脚本一起打死，并让 `check --json` 什么都不打印。
  现在每一处都给出诊断（`entry`、`scope`、`graft-entry`）并**保守回退**——Cargo 约定、全树、
  空切口表——因此一次运行能报出它查得到的每个问题，而任何被拒绝的配置都不可能静默把注册面
  剪掉。范围取值由纯函数 `SourceScope::from_raw` 解析，因此这些拒绝无需改动进程环境即可钉住。
- 注册面旁边的普通模块不再让构建失败。宿主 `src/` 下任何平铺 `.rs`——比如
  `src/helpers.rs`——过去会以布局消息 panic，宿主自己的 `cargo build` 与每条读取该树的
  `nichlink` 命令都一样，因为发现过程把这种文件当成了放错位置的注册面。现在发现过程问的是
  构建其余部分问的同一个问题（`parse_face` 返回 `Ok(None)` 即"这里没有面"），普通模块静默
  跳过，而确实是面的文件变成 `phase=face-layout` 诊断并点出路径与它该在的布局——因此
  `check --json` 产出的是文档而不是空白 stdout。解析不了的面同样被报告（`face-syntax`），
  且在解码字段的各阶段之前，那些阶段现在跳过它而不是打死进程。
- 深层嵌套的源码不再打死进程。`syn` 是无深度守卫的递归下降解析器，而 `proc-macro2` 只保护
  它自己的词法器，因此对几万层嵌套定界符（或一条 `Vec<Vec<…>>` 链）调用 `parse_faces` 会以
  `fatal runtime error: stack overflow` 结束——那是一次 SIGABRT，任何 `Result` 都报不出来、
  `catch_unwind` 也拦不住。三个 `syn::parse_file` 入口现在先跑一次嵌套扫描，量在 `proc-macro2`
  自己的 token 流上，因此字符串字面量与注释里的括号不会被误当成嵌套；上限取 128——`rustc`
  自己的默认 `recursion_limit`。四种形状由
  `core/src/registry_core/syntax/deep_input_tests.rs` 钉住。
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

- 仓库门禁不再接受它们本应拒绝的东西：内核纯净性现在看得见 `std::io`/`std::thread`/`std::os` 与树形
  导入（`use std::{env, fs}`）并有"走过文件"的下限；挂载门禁无论 `include!` 用哪种定界符都能发现它
  （跑在内核屏蔽后的文本上，因此夹具字符串不算挂载）；文档门禁会报告未闭合的围栏，并覆盖 `docs/` 下
  任意深度的每个 markdown 文件与每个根级 markdown 文件；lint 门禁能读出被 rustfmt 折行的
  `#[allow(…)]`，并从目录树推导必须带 `#![warn(missing_docs)]` 的根，而不是用一份新 crate 可以漏掉的
  清单；`tools/nichlink-package-audit` 从清单推导已发布 crate 集合及其内部依赖；
  `tools/nichlink-publish --check-table` 能读 `x.workspace = true` 形式的内部需求并检查根的
  `[workspace.dependencies]` 版本；嵌套守卫在删掉永远不会触发的泛型实参计数器之后，报出**适用于**
  该形状的上限（线性串 1024、定界符组 128）。
- `application!(entry = …)` 的每一段路径都对包内目录树解析，因此规范的 `<dir>/<dir>.rs` 布局可用
  （`src/control/control.rs` 对应 `entry = crate::control`），而夹在中间解析不出任何东西的段会被拒，
  不再被当作函数名放过。解析器位于 `build_method/src/entry_paths.rs`；为使 `entry.rs` 留在尺寸棘轮
  之内，它与该文件里的文件系统遍历（并入 `discovery.rs`）一起被拆出。
- `WasmLimits::max_element_bytes` 约束一个模块能让 wasmi 在实例化时物化多少：被动元素段从不增长
  表，因此 `table_elements` 从来看不到它，而实测代价是每条约 32 字节（紧凑编码每条约一字节）——
  一个 2 000 103 字节、带两百万条目的模块花掉 64 070 402 字节宿主内存。负载与工件上限一样，在
  编译前从段头量出。
- `PluginArtifact::verify_signed` 对不会咨询验证器的来源返回新增的
  `PluginTrustError::SignatureLaneRequired`，而不是记录一个没有检查过任何东西的 `Signature` 保证；
  同一工件的纯摘要路径仍然可用。
- 以 `.` 开头的 graft 选择器被写入方、解析方与 Studio 已在共用的那条规则拒绝：`..` 过去会被接受、
  把计划写到 `<pkg>/.nichlink/graft.plan`，然后永远不会被列出——一份运行期从不应用、作者却看到
  "计划已创建"的记录。
- 构建产物只在仍然描述当前源码时才被信任：`nichlink_build_method::build_output_is_current` 把已发布
  的 `discovery.fingerprint` 与新算出的指纹比对，pipeline 只在干净的一次运行写下那枚指纹，而
  `explain`（以及 `--overlay`）在它缺失或过期时报 `known: false` 并沿用既有的"跑 `nichlink
  check`"提示。失败的 check 不再留下会被 `explain` 当作当前作用域提供的产物。
- 畸形或带不可求值门控的 `static_graft_plan!` 现在是 `phase=graft-entry` 诊断而不是 panic：
  `check --json` 过去以 101 退出、stdout 为空，并把 `scope` 已经产出的诊断丢掉。读取者现在接收构建
  的诊断集合，三类拒绝都经它上报，且解析失败的消息与 `scope` 的逐字相同，两者因此折叠成一行。
- 文档门禁此前把围栏 Rust 直接交给 `syn`，因此嵌套几万个定界符的围栏会让门禁进程 abort 而不是
  失败——与内核解析入口当初的缺陷同一类，而这是最后一个还有它的地方。现在它向内核的
  `guard_nesting` 提问，那是本工作区唯一的嵌套度量（它公开出来正是为了这件事），并由
  `a_pathologically_nested_fence_is_reported_not_fatal` 钉住"拒绝是一条消息"：把守卫删掉，该
  测试就会 abort。
- `tools/nichlink-publish` 按词读取自己的依赖表，因此有多个内部依赖的 crate 只报出第一个——
  `nichlink-cli` 只被按 `nichlink-build-method` 检查——"依赖还没上 index 就不许发布"的守卫因此
  形同虚设。同一次遍历还把边行的被依赖者当成独立 crate，九行表产出十四个节点。现在两张表都按整行
  读取，`--check-table` 会把它们与清单对比：第一次运行就找出一条真实漂移——`nichlink-cli` 直接依赖
  `nichlink-core`，而表里只列了另外三个。同一个检查用 `awk` 而不是 `sed` 地址范围读取工作区成员，
  因为 sed 的范围不在起始行上测试结束地址：单行的 `members` 数组会让范围一直跑到
  `[workspace.package]`，它的名字随后进入成员列表——只因不存在同名目录才隐形。
### [0.1.0] 首次发布（2026-09-25 已发布）

九个 crate 在发布时按依赖顺序一同发布：`nichlink-core` → `nichlink-macro` /
`nichlink-build-method` / `nichlink-mcp` → `nichlink-run-method` →
`nichlink-debug-method` / `nichlink-plugin-host` → `nichlink-studio` →
`nichlink-cli`。依赖版本要求写 caret `0.1.0`，因此补丁版本不会连锁要求依赖方重发。

各 crate 职责见上方英文列表。本版本线的已知边界见根 `README.md` 的 `## Boundaries` 一节与
[`docs/threat-model.md`](docs/threat-model.md)；kind-only `registry_name` 派生等
随版本变化的行为记录在 [`docs/roadmap-1.0.md`](docs/roadmap-1.0.md)。
