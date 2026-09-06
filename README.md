# NichLink

[简体中文](README.zh-CN.md) | English

NichLink is an open-source, Rust-first protocol for declarative object
registration, contract checking, and atomic grafting. An object declares its
own parent Registry; parent modules do not maintain a child roster.

The workspace also contains build planning, optional runtime evidence, a
Ratatui Studio, an MCP bridge for AI-assisted queries, and isolated plugin
adapters. The current 0.1 line is early production: useful for real projects,
with the static-analysis and plugin boundaries documented below.

## Smallest useful declaration

```rust
pub struct Button;

nichlink::control_object! {
    kind: Button,
}
```

Fields such as `name`, `summary`, `parts`, `parent`, and `registry_name` have
defaults. Add `needs_registry`, `admission`, `registry_rule`, or input/output
contracts only when the object actually needs those boundaries.

For a host package, keep the build adapter small:

```rust
// build.rs
fn main() {
    nichlink_build::run();
}
```

`nichlink-build` discovers the folder-backed registration tree and emits the
static plan before rustc type-checks the generated module. Cargo still requires
this one host-side build entry; a dependency crate cannot inspect its consumer's
`main.rs` automatically.

## First run

From this repository, start the development Studio:

```sh
cd prototypes/registation_test/nichlink
cargo run -p nichlink-studio
```

Press `n` inside Studio to create a project. The wizard asks for a directory,
package name, and `binary`/`library` kind, then writes only the Cargo metadata,
thin `build.rs`, and entry point. The registration tree starts empty: press `a`
to create the first root face. If that face requests a Registry, Studio creates
its local `registry_rule/registry_rule.rs` beside it. Studio switches to the new
project immediately; no hand-created folders or environment variables are required.

The generated entry point re-exports the NichLink declaration macros. Registration
files keep the explicit `crate::control_object!` path, so rust-analyzer can
complete macro fields inside the project. The core and build crates remain normal
Cargo dependencies; Cargo does not copy third-party source into `src/`.

To use NichLink in a new application, add the two path dependencies while the
project is local:

```sh
cargo add nichlink-core --path /path/to/nichlink/core
cargo add nichlink-build --build --path /path/to/nichlink/build
```

Keep this `build.rs` in the application root:

```rust
fn main() {
    nichlink_build::run();
}
```

The generated module is included once at the crate root. The small shim keeps
the generated `registry_core` name stable; it is metadata plumbing, not a child
registration list:

```rust
// src/main.rs
#[macro_use]
extern crate nichlink_core as nichlink;

pub mod registry_core {
    pub use nichlink::*;
}

include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"));

fn main() {
    println!("registered faces: {}", registrations().len());
}
```

Put each declaration in the canonical folder layout, for example
`src/workspace/workspace.rs`:

```rust
pub struct Workspace;

crate::control_object! { kind: Workspace }
```

Run `cargo check` once to generate the plan. Set `NICH_LINK_PACKAGE_ROOT` and
launch Studio to inspect or edit that application's `src/` tree:

```sh
NICH_LINK_PACKAGE_ROOT=/work/my-app cargo run -p nichlink-studio
```

When you are already in the host project directory, the environment variable
is optional:

```sh
cd /work/my-app
cargo run --manifest-path /path/to/nichlink/studio/Cargo.toml
```

A `main.rs` is not required. Library packages use `src/lib.rs` as the automatic
scope entry; packages with neither `main.rs` nor `lib.rs` keep the safe full-tree
fallback. Launching Studio without `NICH_LINK_PACKAGE_ROOT` shows an empty root
when the workspace has no host `src/` directory.

### If you are building a framework (library)

The framework owns its registration faces and publishes the generated plan:

```text
my-framework/
  build.rs
  src/lib.rs
  src/workspace/workspace.rs
  src/workspace/registry_rule/registry_rule.rs
  src/workspace/object/panel/panel.rs
```

Every face that owns a Registry may keep its structural contract in a local
`registry_rule/` folder. It constrains incoming face shape; the separate
`admission` field controls which outside objects those faces may use. Neither is
a child roster. Existing `registry/rules/rules.rs` layouts remain readable, while
newly authored projects use `registry_rule/`.

`src/lib.rs` uses the same generated-module prelude shown above, but has no
`main` function:

```rust
#[macro_use]
extern crate nichlink_core as nichlink;

pub mod registry_core {
    pub use nichlink::*;
}

include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"));
```

Consumers only add `my-framework` and call its public API. They do not copy the
framework's registration files. A consumer that adds its own faces gets its own
`build.rs` and generated plan; Cargo cannot use the consumer's `main.rs` to prune
the already-built dependency automatically.

### If you are building a main application (binary)

If the application only uses a framework, add that framework and skip
`nichlink-build` entirely. Add `nichlink-build`, the thin `build.rs`, and the
generated-module prelude only when the application owns additional registration
faces. Its `src/main.rs` can then call the generated root-level `registrations()`
or any application API as usual.

## Crates

```text
nichlink/
  core/         nichlink-core: Registry, transactions, contracts, admission,
                grafts, diagnostics, identity, and declaration macros
  build/        nichlink-build: source discovery, manifest parsing, cache,
                coarse scope, and StaticPlan generation
  debug/        nichlink-debug: optional collector, MIR evidence, CallTrace,
                data-flow and graph adapters
  studio/       nichlink-studio: Ratatui UI, watch, search, source navigation,
                and editor integration
  mcp/          nichlink-mcp: stdio MCP bridge for compact source and graph queries
  plugin-host/  nichlink-plugin-host: verified Wasm/process adapters, lazy
                loading, and atomic deployment
```

`nichlink-core` has no dependency on inventory, Studio, fixtures, or a concrete
root registry. It can be packaged independently. The other crates are workspace
tools and are released after their workspace dependencies.

## Runtime tracing modes

Tracing is optional and policy-driven. A normal application can use the
runtime default, which is `errors-only` in debug builds and `off` in release
builds:

```rust
let trace = nichlink::CallTrace::runtime();
```

Use `CallTrace::disabled()` to force the zero-evidence path, or
`CallTrace::full()` when a Studio/debug session needs every frame, local, and
data edge. For fallible work, wrap the operation with
`trace.with_result(|trace| ...)`; in `errors-only` mode successful evidence is
rolled back while an `Err` or panic keeps the failure chain.

The runtime default can be overridden at process start without a feature flag:

```sh
NICH_LINK_TRACE=full cargo run
```

Accepted values are `off`, `errors-only`, and `full` (with `errors` and
`errors_only` accepted as readable aliases).

## MCP bridge

The optional `nichlink-mcp` binary exposes compact source and call-graph
queries over MCP stdio. A generic client configuration looks like:

```json
{
  "mcpServers": {
    "nichlink": {
      "command": "cargo",
      "args": ["run", "--release", "-p", "nichlink-mcp"],
      "env": { "NICH_LINK_PACKAGE_ROOT": "/work/my-app" }
    }
  }
}
```

The bridge is read-only and keeps the configured project root as its file
boundary. Its static call graph is deliberately labelled heuristic; live
`CallTrace` evidence remains authoritative for dynamic calls and values.

## Why NichLink exists

Use NichLink when an object graph needs three things at once: each object declares
where it belongs, contracts reject incompatible objects before publication, and a
middle layer can be replaced atomically. For a small application, ordinary Rust
modules, traits, or constructor injection are usually simpler.

## Studio at a glance

![](./picture/NichLink_studio.png)

The Studio is an optional, event-driven Ratatui tool. It is not part of the core
runtime and is only redrawn after input, resize, or watch events.

## Comparison

| Approach | Good at | Missing from it | NichLink adds |
| --- | --- | --- | --- |
| Modules / traits | Namespaces and static behavior | Discovery, admission, recursive topology | A declaration and contract layer |
| Dependency injection | Explicit construction and testing | Registry tree and atomic middle-layer replacement | Passive registration and graft transactions |
| `inventory` / `linkme` | Distributed flat collection | Contracts, parent closure, provenance | Optional collector plus tree semantics |
| Bevy plugins | Explicit application composition | Generic object paths and graft validation | Object-owned registration and contracts |
| CodeGraph / CodeQL | Symbol and call evidence | Runtime registry decisions | Evidence consumed by Studio/MCP |

## Capability boundaries

- Static call results are evidence, not a promise for every `dyn Trait`, function
  pointer, FFI, or runtime-selected call.
- Optimized locals without instrumentation may be `unobserved`; use `full` tracing
  when values matter.
- Process plugins isolate failures and resources but are not OS security sandboxes.
- A dependency crate's `build.rs` cannot read a consuming application's `main.rs`;
  the host supplies the one build entry.

## Namespace isolation

Every compiled registration face carries the package namespace from
`CARGO_PKG_NAME`. Its `NodeId` and root identity include that namespace, so two
libraries may use the same relative source path, kind, or root display name
without sharing identities. A `Registry` created with
`Registry::root_for_namespace(framework, namespace)` rejects snapshots from a
different namespace before mutating its transaction.

The legacy `ROOT_NODE_ID` and `Registry::root()` remain available for the
default compatibility namespace. New hosts should choose an explicit stable
namespace, for example:

```rust
let registry = nichlink::Registry::root_for_namespace(
    nichlink::FrameworkId::new("my-app"),
    "my-company.my-app",
);
```

Studio reads `NICH_LINK_NAMESPACE`; this lets two libraries run in one process
without cross-loading authored faces. Set `NICH_LINK_PACKAGE_ROOT` to the host
project directory before launching Studio so authoring reads and writes that
project's `src/` tree. The namespace is an isolation boundary, not a display
name: identical names are allowed in separate registries.

For example:

```sh
NICH_LINK_PACKAGE_ROOT=/work/my-app \
NICH_LINK_NAMESPACE=my-app \
cargo run -p nichlink-studio
```

## Build and package

```sh
cargo fmt --all
cargo test --workspace --offline
cargo test --workspace --release --offline
cargo package -p nichlink-core --allow-dirty --offline
cargo package -p nichlink-build --allow-dirty --offline
cargo package -p nichlink-mcp --allow-dirty --offline
```

`tools/nichlink-release-audit` also writes release evidence under
`target/nichlink-audit/release/`: artifact sizes, defined-symbol lists, and the
node/function manifest used to review final linker pruning.

The dependent packages (`nichlink-debug`, `nichlink-studio`, and
`nichlink-plugin-host`) use the published `nichlink-core` version when Cargo
creates an upload package. Publish or stage the core package in a registry
first, then package those crates in dependency order; a local offline package
run cannot resolve an unpublished crate from crates.io.

The parent directory contains the original prototype tests and external module
fixtures. They are intentionally outside this workspace and are not included
when packaging `nichlink/`.

The workspace declares Rust `1.82` as its MSRV. CI also runs the moving
`stable` toolchain (currently `1.96` in the development environment), so the
latest compiler is checked without hard-coding a version that will go stale.

The public technical roadmap is `ROADMAP.md`. Migration and threat details are
kept as supplemental notes; they are not required to understand the core API.

The independent CI definition is in `.github/workflows/ci.yml`. Run
`tools/nichlink-package-audit` for the local package checks and the dependency
order required before publishing the remaining crates.

Language index: [简体中文 README](README.zh-CN.md) and
[中文路线图](ROADMAP.zh-CN.md). Supplemental notes also provide Chinese links.
