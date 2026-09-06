# nichlink-build

[简体中文](README.zh-CN.md) | English

`nichlink-build` is the build-time half of NichLink. It discovers registration
faces from the host crate's folder layout, validates parent and requirement
relationships, maintains incremental identity caches, and emits the passive
`StaticPlan` consumed by release builds.

Call `nichlink_build::run()` from the host crate's `build.rs`. The build process
uses the host package's `CARGO_PKG_NAME` as the identity namespace, matching the
identity captured by `nichlink-core` declaration macros.

The optional host entry is parsed as Rust syntax, not text search:

```rust
nichlink::application!(entry = crate::main);
```

There must be exactly one declaration, and its first module must resolve to a
source file under `src/` (including `src/bin/`). Malformed, duplicate, or
unresolvable entries fail the build with the declaration location.
