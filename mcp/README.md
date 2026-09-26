# nichlink-mcp

[简体中文](README.zh-CN.md) | English

`nichlink-mcp` is a small Model Context Protocol server for AI-assisted
NichLink work. It uses newline-delimited JSON-RPC over stdin/stdout and writes
nothing to stderr: a failure is an error response on stdout, where the client is
already reading, so the stream stays attachable to an MCP client.

Run it from a host project (or `nichlink mcp` via the `nichlink-cli` package):

```sh
NICH_LINK_PACKAGE_ROOT=/work/my-app nichlink mcp
```

The server exposes compact tools. Reads:

- `nichlink.search`: find files and function declarations.
- `nichlink.inspect`: list the functions and registration declarations in one file.
- `nichlink.callgraph`: find direct static callers and callees for a function.
- `nichlink.read`: read a bounded source window.
- `nichlink.status`: report the indexed source root and counts.
- `nichlink.registry`: report the registration faces this package declares —
  logical path, kind, source, and the `NodeId` the host compiled.
- `nichlink.explain`: report the build's own evidence for one face (identity,
  path, kind, source, module, parent, slot) or the tree projection it scoped. The
  registry answer above is derived from source text and is therefore always fresh;
  this one reads the files the build *published* under `target/nichlink/out`, so it
  answers what ships: whether the scope selected the face and whether release
  pruning strips its symbols. A missing or stale build is reported as such.
- `nichlink.diff`: the face-level delta between the sources now and the build's
  manifest — added, gone, and re-identified under an unmoved file (a `kind` change
  is an identity change, so only this comparison sees it).
- `nichlink.trace`: read this package's recorded trace artifact and answer with the
  headless call report it implies — what actually ran, which no static read can
  tell you. The artifact's identity is checked first (namespace, registry root,
  every frame's node); a foreign artifact is refused by name rather than drawn.
- `nichlink.mir`: read a MIR artifact — a `rustc -Zunpretty=mir` text dump or the
  compact JSONL form, chosen by extension — and report the compiler's call
  candidates. `jsonl: true` emits that JSONL, the portable channel Studio could
  already render and parse while nothing in the workspace ever wrote one. A
  malformed JSONL line fails the whole read; a text dump never fails and only ever
  yields the calls it contains. The text producer is `cargo rustc -Zunpretty=mir`
  on a nightly toolchain, and a missing artifact says so instead of reporting an
  empty graph.
- `nichlink.unified`: merge a MIR artifact with this package's recorded trace
  through `nichlink_debug_method::UnifiedCallGraph`, the one place the two evidence
  sources are joined — a call the trace confirms carries `evidence=Live` and
  *replaces* its compiler candidate, while the rest stay `evidence=Mir`. With no
  recorded trace the merge still answers, labelling every relation a compiler
  candidate.
- `nichlink.usages`: a face's neighbourhood — its parent and children as the tree
  has them, the fields `nichlink.apply` accepts read back from the generated module
  (preset, parts, names, exports, `requires`, `provides`, handle and part traits and
  contracts, registration rule, admission, flow, runtime checks), and which other
  faces mention the same capability tokens. Capability matches are on declared
  tokens rather than a resolved graph, and the reply says so; a hand-written module
  has no generated field list, so those faces are counted as unreadable rather than
  shown empty.
- `nichlink.converge`: everything an agent needs to start on one face in a single
  answer — the build's scope and pruning verdicts, the tree's edges, the declared
  fields, whether each `capability=>ProviderKind` requirement is actually answered
  (named when it is), the files to read, and which tool has the detail. When the
  package's own faces *are* rejected by the kernel, that rejection is the verdict
  here rather than an error: it already names the offending node and its source
  location, which is the moment this answer matters most. With `trace: true` it
  starts from the recorded run instead — the files that both declare a face and
  actually ran, the frames that landed in each, and the frames that fell outside any
  declared face. Frames are matched to faces by source file and the reply says so,
  because a face is a declaration while a frame is an active function.
- `nichlink.verify`: run the kernel's registration validation over this package and
  report the tree delta the run just published. It drives the same entry the CLI's
  `check` drives, so its verdict cannot drift from `nichlink check`, and it refreshes
  the build evidence as a side effect — which is why the delta describes the tree that
  was just verified. A failed verdict is the answer, not a tool failure: the reply
  says `verdict failed` with the diagnostics (phase, node, source and line) and stays
  `isError: false`, because the verification itself succeeded.

And one that writes:

- `nichlink.apply`: edit a registration face through the **same authoring executor
  Studio uses**, so the kernel's admission, parent-rule, and topology checks run on
  the change. `action` is `add` (create `fields.module` under `parent`), `edit`
  (change the `fields` the request names and keep the rest — the face is read back
  first, so a two-field request cannot blank twenty-one others), `rename` (change
  `fields.module`, keeping every other field), or `delete` (move the module into
  NichLink's recoverable trash). `node` names the face and `parent` the parent, by
  logical path or identity — the path `nichlink.registry` reports is directly
  usable, and it is the *registry* path (`root/control/button`), not the file's
  (`control/object/button/button.rs`); aiming with the file-shaped one is refused
  by name. **A request is previewed unless `apply: true`**: the preview runs the real
  operation on a throwaway copy of the package and returns the file diff plus the
  registration tree that results; only `apply: true` writes to the project, and then
  it names the files it wrote and anchors the declaration it produced as
  `<path>:<line>` — the same shape a refusal uses for its `file:line:column`. Every
  reply ends with the tree as it now stands, so the next call can be aimed with it.

The first five index Rust source text; `nichlink.registry` reports the tree the
**build** derives (`nichlink_build_method::face_views`, the same derivation the CLI's
`explain` uses), so an agent can ask what the registry is instead of
reconstructing it from macro names. Both ask Cargo one thing: a package name *is*
the `NodeId` namespace, so they resolve it through
`nichlink_build_method::package_name` (`cargo metadata`). `NICH_LINK_NAMESPACE` wins
verbatim when it is set; when neither answers, they **refuse** rather than falling
back to `nichlink.default`, because an identity under the default namespace names a
node the host never compiled.

The write path adds no second implementation of editing: it loads the package's own
faces into a `Registry`, installs an `AuthoringContext` from the resolved root and
namespace, and calls the executor. What it adds is the **preview contract** — the
real operation on a copy — which is also why the bridge still cannot corrupt a
project by accident.

The executor's own boundary is inherited unchanged: it rewrites the faces **NichLink
generated** (the files carrying its generated marker) and refuses a hand-written
one with `this module was not generated by NichLink`, because rewriting a file it
did not author would discard content it does not model. `add` works in any package;
`edit`, `rename`, and `delete` address generated faces.

Contract, admission, and registration-rule *data* are still not reported: those
live in the built `RegistrationSnapshot`s rather than in the source, so they need
the build output and not a scan (`docs/roadmap-1.0.md` item 10). Graft writes, plugins, project
scaffolding, tree diffs, and the consistency analysis are still to come
(`docs/roadmap-1.0.md` item 7).

Call-graph results are labelled `static-heuristic`. They intentionally do not
claim to resolve dynamic dispatch, function pointers, FFI, or runtime-selected
calls; use `nichlink-debug-method` and a live `CallTrace` for those edges, which is
what `nichlink.unified` does once a MIR dump and a recorded trace are both present.
