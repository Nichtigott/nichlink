# nichlink-mcp

[简体中文](README.zh-CN.md) | English

`nichlink-mcp` is a small Model Context Protocol server for AI-assisted
NichLink work. It uses newline-delimited JSON-RPC over stdin/stdout and keeps
all diagnostics on stderr so it can be attached directly to an MCP client.

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

Call-graph results are labelled `static-heuristic`. They intentionally do not
claim to resolve dynamic dispatch, function pointers, FFI, or runtime-selected
calls; use `nichlink-debug` and a live `CallTrace` for those edges.
