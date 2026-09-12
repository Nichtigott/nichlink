# Draft GitHub Discussion

English | [简体中文](discussion-introduction.md)

## Title

NichLink: declarative recursive registration and atomic grafting for Rust object graphs

## Post

Hi everyone. I am open-sourcing NichLink, a Rust-first infrastructure project for large object graphs.

It is not another way to define a trait. It lets an object declare its parent registry, lets children register recursively without a central roster, checks admission and input/output contracts, and atomically replaces a middle layer without rewriting the whole tree. Studio, `CallTrace`, and MCP make the resulting calls and data flow inspectable.

```rust
#[nichlink::object(parent = crate::root_node_id(env!("CARGO_PKG_NAME")))]
pub struct Button;
```

A typical replacement looks like `NodeEditor -> Canvas2D -> WGPU`: a new `Canvas2D` must satisfy the old input, output, and structural contracts before the graft is published.

The repository contains `nichlink-core`, `nichlink-build-method`, `nichlink-run-method`, `nichlink-debug-method`, `nichlink-cli`, `nichlink-studio`, `nichlink-mcp`, and `nichlink-plugin-host`.

I would value feedback on three points:

1. Is object-owned registration easier to maintain than manual `add/register/wire` calls?
2. Are admission, structural rules, I/O contracts, and grafts clearly separated?
3. Which non-UI project would try a small example, such as `parser -> optimizer -> codegen`?

Known limits are explicit: static calls are heuristic around dynamic dispatch and FFI; uninstrumented locals may be unavailable; process plugins are not security sandboxes; and Cargo build scripts cannot read a consuming `main.rs` from a dependency crate.

Repository: <https://github.com/OWNER/NichLink>  
Roadmap: [`ROADMAP.md`](../ROADMAP.md)
