<div align="center">

<img src="./picture/NichLink_wordmark.svg" alt="NichLink ASCII wordmark">

<p><strong>A Rust code-organization model built for how communities actually extend software, for engineering collaboration, and for agentic coding: passive recursive registration, explicit contracts, atomic replacement at any level.</strong></p>

[![license](https://img.shields.io/github/license/Nichtigott/nichlink?style=flat-square)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/Nichtigott/nichlink/ci.yml?style=flat-square&label=CI)](.github/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-Rust%201.96-8250df?style=flat-square)](Cargo.toml)

<p>
<a href="#features"><kbd>Features</kbd></a>
<a href="#get-started"><kbd>Get started</kbd></a>
<a href="#registration-faces"><kbd>Registration faces</kbd></a>
<a href="#studio-and-cli"><kbd>Studio / CLI</kbd></a>
<a href="#boundaries"><kbd>Boundaries</kbd></a>
</p>

</div>

In a large Rust project, a localized change can cut across half the
repository. NichLink addresses this at the structural level: objects carry
small, explicit boundaries; a replacement is validated where it plugs in; and
the source location of every object remains visible to the people and tools
working on it.

AI-assisted and agentic coding amplify this pressure rather than create it.
Neither a model nor a human reviewer sustains complete attention over a large
codebase; hallucinated references and missed context are routine failure
modes, not exceptions. NichLink therefore turns assumptions that usually live
in convention into inspectable structure: atomic objects, explicit contracts,
source provenance, and a development-time registry graph that a team can read
together. The same boundaries serve code review, community contributions, and
ordinary Rust development — AI agents are one consumer of this structure, not
the reason it exists.

The model also defines how a community extends a project. A contributor ships
an implementation beside the original tree, declares the boundary it
replaces, and lets the host validate the graft. Competing implementations
coexist without turning upstream into a patch queue: the project keeps its
shape, and experimentation happens inside named contracts.

NichLink is early-stage software: the core protocol is usable in real projects
today, and both static analysis and runtime evidence report their coverage
explicitly.

## Features

- **Passive, recursive registration.** A face declares its parent in its own
  file. The parent accepts it; no central child list is maintained. A face may
  own another `Registry`, so the same rule repeats at every depth.
- **One registry type.** A root registry and a nested registry use the same API.
  The tree comes from registration, not from special root/leaf types.
- **Contracts at the boundary.** Preset/parts output types and handle contracts
  can fail at compile time. Structural `registry_rule` checks and external
  `admission` checks return all failures before publication.
- **Atomic grafting.** A replacement targets a logical slot and must match the
  declared input/output `FlowContract`. Validation happens before the live tree
  is changed.
- **Two build passes.** The first pass discovers the reachable registration
  faces and emits a smaller `StaticPlan`. The normal Rust compiler and linker
  then perform the final code and symbol elimination. These are separate goals:
  less registration metadata first, smaller machine code at the end.
- **Debug only when requested.** `off`, `errors-only`, and `full` tracing keep
  the release path free of evidence collection unless an application opts in.
  Studio and the MCP bridge consume the same registry and call/data-flow model.

## Get started

There are two supported ways to try NichLink.

### Install the Studio binary

This keeps the NichLink source outside your application. Install the Ratatui
tool from the Git repository, then point it at the project you want to inspect:

```sh
cargo install --git https://github.com/Nichtigott/nichlink --bin nichlink-studio nichlink-studio
cd /work/my-app
NICH_LINK_PACKAGE_ROOT="$PWD" nichlink-studio
```

For a released crate, replace the Git source with `cargo install nichlink-studio`.

### Run from a clone

This is handy while developing NichLink itself and does not install anything:

```sh
git clone https://github.com/Nichtigott/nichlink
cd nichlink
cargo run -p nichlink-studio
```

To inspect another project from the clone:

```sh
NICH_LINK_PACKAGE_ROOT=/work/my-app cargo run -p nichlink-studio
```

Studio's `n` action creates a binary or library project. It writes the manifest,
the thin build entry, and the source entry point; the first face is added with
`a`. It does not invent a `control` tree. A registry owner gets a local
`registry_rule/` directory only when that rule is needed.

### Add NichLink to an application

The application owns its declarations, so Cargo needs one small build adapter:

```sh
cargo add nichlink-core --path /path/to/nichlink/core
cargo add nichlink-build --build --path /path/to/nichlink/build
```

```rust
// build.rs
fn main() {
    nichlink_build::run();
}
```

In the crate root, connect the generated plan once:

```rust
pub mod registry_core {
    pub use nichlink_core::*;
}

include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"));
```

`main.rs` is optional. A binary uses `src/main.rs` as the application entry; a
framework library uses `src/lib.rs`. The build adapter scans the host crate's
own source tree. Cargo does not give a dependency's `build.rs` access to the
consumer's `main.rs`, so a consumer that owns faces needs its own adapter.

## Registration faces

A registration face has two layers. The Rust `struct` and its `impl` contain the
real implementation. The declaration records where that implementation belongs
and what may enter or replace it.

```rust
pub struct Canvas;

pub struct CanvasParts;
pub struct CanvasPreset;

impl nichlink_core::PresetContract for CanvasPreset {
    type Output = CanvasParts;
    const REQUIRED_PARTS: &'static [&'static str] = &["paint"];
}

impl nichlink_core::PartsContract for CanvasParts {
    type Output = CanvasParts;
    const PROVIDED_PARTS: &'static [&'static str] = &["paint"];
}

impl Canvas {
    pub fn render(&self, input: CanvasInput) -> CanvasFrame {
        // the implementation stays ordinary Rust
        input.into_frame()
    }
}

// The generated parent macro expresses the source hierarchy.
// For a face under `node_editor`, use `node_editor_object!` here.
crate::node_editor_object! {
    kind: Canvas,
    preset: CanvasPreset,
    parts: CanvasParts,
    handle: Canvas,
    summary: { zh: "二维画布", en: "A 2-D drawing surface" },
    exports: ["canvas.render"],
    needs_registry: false,
    requires: ["viewport" => "layout.viewport"],
    provides: ["canvas.frame"],
    expected_output: "CanvasFrame",
    actual_output: "CanvasFrame",
    flow: nichlink_core::FlowContract::new(
        nichlink_core::ContractId::new("canvas.render.v1"),
        1,
        "CanvasInput",
        "CanvasFrame",
    ),
}
```

Most fields have safe defaults. The smallest face is simply
`crate::root_object! { kind: App }` or a generated `<parent>_object!` with a
`kind`. Add `summary`, `exports`, `requires`, or `flow` when they carry useful
information; do not repeat defaults just to fill a table.

There are three different checks, and they are deliberately not merged:

1. **Construction contract (compile time).** `preset` and `parts` must agree
   on their associated output type. `handle_contracts` and `part_contracts` ask
   rustc to prove that the handle and parts types implement the required traits.
2. **Registration rule (build/publication time).** `registry_rule` describes only
   the minimum child shape: preset, parts, exports, and interfaces. It does not
   keep a kind allowlist or decide parentage. Missing structure is reported
   together instead of producing
   a half-registered object.
3. **Admission (dependency gate).** `admission` controls which outside registry
   paths this face may use. A call to an object outside that gate is rejected;
   the resulting diagnostic names the face, source location, and dependency.

For a replacement, both sides publish a flow contract. The host validates the
contract id, version, input, and output before applying the graft:

```rust
let plan = nichlink_core::GraftPlan::command(
    framework,
    "cut root/canvas graft canvas_fast",
)?;
let effective = base.overlay(&plan, &external)?;
```

An extension only needs to satisfy the target registration rule. A replacement
also needs an input/output contract that is semantically compatible with the
slot. This is what makes a middle-layer swap explicit rather than an accidental
module rename.

### Structural rules are minimums

registry_rule is a lower bound. A face may contain more fields, methods,
interfaces, or exports than the rule lists; it may not omit a required one.
The rule does not claim that the listed fields are the complete shape of the
object. This matters for framework evolution: adding a method to an existing
face does not break older consumers, while removing a required part does.

### How a parent constrains its children

The parent writes the rule on the Registry it owns. A child does not copy that
rule; when the child is submitted, the parent Registry validates the child's
preset, parts, exports, and declared interfaces. The rule is a minimum
shape, so extra implementation details are fine.

```rust
pub struct Control;

pub struct ControlFrame;
pub struct ActionParts;

pub trait ControlHandle {
    fn paint(&self, parts: &ButtonParts) -> ControlFrame;
}

pub trait ActionPartsContract {
    fn action_id(&self) -> &str;
}

pub struct Button;
pub struct ButtonParts {
    pub label: String,
    pub action: String,
}

impl nichlink_core::PresetContract for ActionParts {
    type Output = ButtonParts;
    const REQUIRED_PARTS: &'static [&'static str] = &["paint"];
}

impl nichlink_core::PartsContract for ButtonParts {
    type Output = ButtonParts;
    const PROVIDED_PARTS: &'static [&'static str] = &["paint"];
}

impl ControlHandle for Button {
    fn paint(&self, parts: &ButtonParts) -> ControlFrame {
        let _label = &parts.label;
        ControlFrame
    }
}

impl ActionPartsContract for ButtonParts {
    fn action_id(&self) -> &str {
        &self.action
    }
}

// The parent face owns the Registry and declares its minimum child shape.
crate::root_object! {
    kind: Control,
    needs_registry: true,
    registry_rule: crate::RegistrationRule::new()
        .require_preset("ActionParts")
        .require_parts(&["paint"])
        .require_exports(&["control.render"])
        .require_handle_traits(&["ControlHandle"])
        .require_part_traits(&["ActionPartsContract"]),
}

// Button supplies every required item, and may add more.
crate::control_object! {
    kind: Button,
    preset: ActionParts,
    parts: ButtonParts,
    handle: Button,
    exports: ["control.render"],
    handle_traits: ["ControlHandle"],
    handle_contracts: [crate::ControlHandle],
    part_traits: ["ActionPartsContract"],
    part_contracts: [crate::ActionPartsContract],
}
```

The constraint runs from parent to child: Control's Registry reads its rule and
validates Button. A preset other than ActionParts, no `paint` in
`ButtonParts::PROVIDED_PARTS`, no `control.render` export, or either missing
interface is included in one structured report. `handle_contracts` and
`part_contracts` additionally ask rustc to prove that both impls exist. Button may add
fields, methods, traits, and exports; extra structure is never rejected.

Parentage comes from `control_object!` and `parent`, not a kind filter. External
calls are controlled separately by `admission`.

Older prototypes used `RegistrationRule::new(&["Button"], &[])`. That form is
gone. Replace it with `RegistrationRule::new()` plus only the required shape
methods shown above. Move external path allow/deny entries to `Admission`; do
not move the old kind list there, because kind never decides parentage.

The three layers stay separate:

    Rust impl / struct       actual code and private details
    registry_rule            minimum shape accepted by the parent Registry
    admission                outside registry paths this face may use
    FlowContract             wire-level input/output at a replacement boundary

### Output extensions and compatibility

The current flow contract compares a stable contract id, version, and the
declared input/output labels. It does not guess field-level covariance from
Rust types. If a new implementation returns more information, use one of these
explicit designs:

- keep the same output label and add backward-compatible fields to a stable
  envelope (CanvasFrame with optional metadata);
- publish a new contract version (CanvasFrame v2) and update every consumer
  boundary in one change; or
- add an adapter face that converts the richer output back to the old contract.

The last two choices make the affected boundary visible instead of silently
discarding data. A graft is accepted only after the chosen boundary contracts
and the destination registration rule pass.

### One graft or a coordinated graft set

The registration tree describes ownership, not every data-flow edge. A
requires edge can resolve a provider on another branch, and runtime data edges
may cross several registry boundaries. When a replacement changes several
consumers, describe the affected slots as one plan and commit them together:

```rust
let plan = nichlink_core::graft_plan!(framework,
    cut ["root/canvas"] graft "canvas_fast",
    cut ["root/hit_test"] graft "hit_test_fast",
    cut ["root/layout"] graft "layout_fast",
);
let effective = base.overlay(&plan, &external)?;
```

`overlay` stages the whole set. Every cut checks source and destination
contracts, destination registry_rule, and connector admission. If one consumer
still expects the old boundary, the operation fails without publishing a
partial effective tree.

The overlay operation never moves source code or mutates either registry. Use
`cut A graft X` to replace only slot `A` (its existing children are inherited),
or `cut A full graft X` to replace the complete subtree rooted at `A` with the
external subtree rooted at `X`. A path range can target contiguous siblings:

```rust
let plan = nichlink_core::GraftPlan::command(
    framework,
    "cut [root/a1 to root/a3] graft replacement",
)?;
let effective = base.overlay(&plan, &external)?;
```

The returned effective registry keeps the base tree and external tree
unchanged, preserves the logical target path, inherits untouched siblings, and
re-runs destination-rule, admission, and connector checks before publication.

Studio's graft workflow uses `g`. It creates
`.nichlink/external-grafts/<selector>/graft.plan`, opens that plan in the
configured editor, and leaves the host source untouched. There is no
source-copy or source-replacement graft path.

NichLink does not prescribe the programming paradigm inside a face. Functions,
traits, generics, closures, dependency injection, and message passing remain
ordinary Rust. The registration face governs the published boundary. Declared
`requires/provides` and `FlowContract` edges are checked automatically during
registration, connection, and grafting. Changing an implementation is fine; a
failure occurs only when an input has no admitted provider or a replacement no
longer reconnects to the declared data flow. The diagnostic names the broken
capability, consumer, candidate provider, source location, and phase. This is
not a claim that NichLink guesses every undeclared value flow inside arbitrary
Rust code.

## Studio and CLI

Studio is a resident Ratatui application, not a stream of printed snapshots. It
opens an alternate terminal screen, watches the selected project, and redraws
on input, resize, or a file event.

![NichLink Studio](./picture/NichLink_studio.png)

| Key | Action |
| --- | --- |
| `n` | New binary/library project |
| `a` / `e` / `d` | Add, edit, or delete a face |
| `g` | Create and edit an external graft plan for the selected face |
| `/` | Search files and functions |
| `1`–`4` | Search, inspect, data, compare pages |
| `Tab` | Move focus between tree and details |
| arrows / `j` `k` | Move or resize the focused panel |
| `r` / `F5` | Reload the project |
| `b` / `F9` | Run the build check |
| `q` / `Ctrl-C` | Quit |

For source-driven hot rebuild while working on Studio itself:

```sh
NICH_LINK_PACKAGE_ROOT=/work/my-app \
  cargo run -p nichlink-studio --bin nichlink-dev -- watch
```

The command-line surface is intentionally small. `cargo check` is the build
validation command, `nichlink-mcp` is the read-only JSON-RPC/MCP bridge for AI
clients, and Studio is the interactive authoring/debug surface:

```sh
cargo check
NICH_LINK_PACKAGE_ROOT=/work/my-app cargo run -p nichlink-mcp
```

MCP tools include `nichlink.search`, `nichlink.inspect`, `nichlink.callgraph`,
`nichlink.read`, and `nichlink.status`. Static call-graph answers are labelled
heuristic; live `CallTrace` evidence is authoritative for dynamic calls and
runtime values.

## When NichLink is worth it

NichLink is not a replacement for Rust's module system. It earns its place when
an object graph is maintained by several people or tools, when a middle layer
must be swapped without rebuilding its neighbours, or when an AI agent needs a
machine-readable explanation of what an object accepts and provides. For a
small application assembled in one place, ordinary Rust is usually the better
choice.

## Comparison

| Approach | Solves well | Leaves to the application |
| --- | --- | --- |
| Modules, traits, DI | Namespaces, static behaviour, explicit construction | Discovery, parent closure, admission, replacement policy |
| `inventory` / `linkme` | Distributed collection of static items | Tree semantics, contracts, provenance, atomic grafts |
| Bevy-style plugins | Explicit composition of an application | Generic source paths and middle-layer contract checks |
| CodeGraph / CodeQL | Symbol and call evidence | Runtime registration and replacement decisions |
| NichLink | Passive recursive tree, contracts, admission, graft validation, Studio/MCP views | Rust's own rules for dynamic dispatch and optimised values |

## Boundaries

- The coarse build pass sees the host crate's source, not a downstream
  consumer's source. The final binary still follows rustc/LLVM/linker reachability.
- MIR and static scans provide candidates. `dyn Trait`, function pointers, FFI,
  runtime-selected calls, and macro-generated code may require conservative
  retention or live evidence.
- Uninstrumented optimized locals may be shown as `unobserved`. Use the `full`
  tracing mode when values, not just types, matter.
- Process plugins isolate crashes and resource limits; they are not an
  operating-system security sandbox. Wasm/process loading is optional.
- Studio, debug, MCP, and plugin-host code are optional tools. A core-only
  release keeps the registry protocol without the development UI.

## Runtime tracing

```rust
let trace = nichlink_core::CallTrace::runtime(); // debug: errors-only, release: off
let quiet = nichlink_core::CallTrace::disabled();
let detailed = nichlink_core::CallTrace::full();
```

`errors-only` keeps failed chains and discards successful evidence. `full` keeps
frames, locals, and data edges for Studio/MCP inspection. The default release
path collects nothing unless the application opts in.

## Workspace layout

```text
core/         nichlink-core: Registry, contracts, admission, grafts, macros
build/        nichlink-build: source discovery, cache, coarse StaticPlan
debug/        optional CallTrace, MIR evidence, data-flow and graph adapters
studio/       Ratatui authoring, search, watch and source navigation
mcp/          read-only MCP bridge for AI-assisted queries
plugin-host/  optional Wasm/process adapters and atomic deployment
```

The technical roadmap is in [`docs/ROADMAP.md`](docs/ROADMAP.md), with a Chinese
version at [`docs/ROADMAP.zh-CN.md`](docs/ROADMAP.zh-CN.md). Package-specific API
notes live beside each crate.

## Build and license

```sh
cargo fmt --all
cargo test --workspace --offline
cargo clippy --workspace --all-targets -- -D warnings
```

NichLink is released under the [MIT License](LICENSE). Contributions, design
critique, and real-world failure reports are welcome in GitHub Issues and
Discussions.

[简体中文](README.zh-CN.md) · [Roadmap](docs/ROADMAP.md) · [中文路线图](docs/ROADMAP.zh-CN.md)
