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

The server exposes compact, read-only tools:

- `nichlink.search`: find files and function declarations.
- `nichlink.inspect`: list the functions and registration declarations in one file.
- `nichlink.callgraph`: find direct static callers and callees for a function.
- `nichlink.read`: read a bounded source window.
- `nichlink.status`: report the indexed source root and counts.
- `nichlink.registry`: report the registration faces this package declares —
  logical path, kind, source, and the `NodeId` the host compiled.

The first five index Rust source text; `nichlink.registry` is the first that does
not. It reports the tree the **build** derives (`nichlink_build_method::face_views`,
the same derivation the CLI's `explain` uses), so an agent can ask what the
registry is instead of reconstructing it from macro names. It is still read-only,
and it does ask Cargo one thing: a package name *is* the `NodeId` namespace, so it
resolves the name through `nichlink_build_method::package_name` (`cargo metadata`).
`NICH_LINK_NAMESPACE` wins verbatim when it is set; when neither answers, the tool
**refuses** rather than falling back to `nichlink.default`, because an identity
reported under the default namespace names a node the host never compiled.

Contract, admission, and registration-rule data are still not available: those
live in the built `RegistrationSnapshot`s rather than in the source, so they need
the build output and not a scan (`docs/roadmap-1.0.md` item 10).

Call-graph results are labelled `static-heuristic`. They intentionally do not
claim to resolve dynamic dispatch, function pointers, FFI, or runtime-selected
calls; use `nichlink-debug-method` and a live `CallTrace` for those edges.
