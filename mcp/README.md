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

And one that writes:

- `nichlink.apply`: edit a registration face through the **same authoring executor
  Studio uses**, so the kernel's admission, parent-rule, and topology checks run on
  the change. `action` is `add` or `edit`, `fields` carries the face fields, and
  `parent` names the parent by logical path or identity — the path
  `nichlink.registry` reports is directly usable. **A request is previewed unless
  `apply: true`**: the preview runs the real operation on a throwaway copy of the
  package and returns the file diff plus the registration tree that results; only
  `apply: true` writes to the project, and then it names the files it wrote. Every
  reply ends with the tree as it now stands, so the next call can be aimed with
  it.

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

Contract, admission, and registration-rule *data* are still not reported: those
live in the built `RegistrationSnapshot`s rather than in the source, so they need
the build output and not a scan (`docs/roadmap-1.0.md` item 10). `delete`, `rename`,
graft writes, plugins, project scaffolding, tree diffs, and the consistency
analysis are still to come (`docs/roadmap-1.0.md` item 7).

Call-graph results are labelled `static-heuristic`. They intentionally do not
claim to resolve dynamic dispatch, function pointers, FFI, or runtime-selected
calls; use `nichlink-debug-method` and a live `CallTrace` for those edges.
