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

💬 [Discuss the graft model](https://github.com/Nichtigott/nichlink/discussions)

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

### Install the NichLink CLI

This keeps the NichLink source outside your application. Install the CLI from
the Git repository; it provides the `nichlink` command, including `nichlink
studio` for the Ratatui Studio:

```sh
cargo install --git https://github.com/Nichtigott/nichlink nichlink-cli
nichlink new my-app
cd my-app && nichlink studio
```

For a released crate, replace the Git source with `cargo install nichlink-cli`.
The plugin binary also answers to `cargo nichlink <command>`. To inspect an
existing project instead, point the CLI at it:

```sh
NICH_LINK_PACKAGE_ROOT=/work/my-app nichlink studio
```

The standalone `nichlink-studio` binary remains available for direct installs.

### Run from a clone

This is handy while developing NichLink itself and does not install anything:

```sh
git clone https://github.com/Nichtigott/nichlink
cd nichlink
cargo run -p nichlink-cli -- studio
```

To inspect another project from the clone:

```sh
NICH_LINK_PACKAGE_ROOT=/work/my-app cargo run -p nichlink-cli -- studio
```

Studio's `n` action creates a binary or library project. It writes the manifest,
the thin build entry, and the source entry point; the first face is added with
`a`. It does not invent a `control` tree. A registry owner gets a local
`registry_rule/` directory only when that rule is needed.

### Add NichLink to an application

The application owns its declarations, so Cargo needs one small build adapter:

```sh
# From Git. `nichlink-build` belongs to the build dependency table.
cargo add nichlink-core --git https://github.com/Nichtigott/nichlink --branch main
cargo add nichlink-build --build --git https://github.com/Nichtigott/nichlink --branch main

# Or, while developing both projects from local checkouts:
cargo add nichlink-core --path /path/to/nichlink/core
cargo add nichlink-build --build --path /path/to/nichlink/build
```

Do not add `nichlink-build` once under `[dependencies]` and again from a
different source under `[build-dependencies]`; Cargo requires one canonical
source for a package throughout a manifest.

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

Here is a complete two-level example. `Control` is the parent face. `Button` is
a child entering the Registry owned by `Control`. The rule belongs to the
parent, so it lives under `control/registry_rule/`; the child only declares
what it supplies.

```text
src/
└── control/
    ├── control.rs                         # parent: Control
    ├── registry_rule/
    │   └── registry_rule.rs               # minimum shape of every direct child
    └── object/
        └── button/
            └── button.rs                  # child: Button
```

The parent defines the shared interfaces and declares that it owns a Registry:

```rust
// src/control/control.rs
use crate::control::registry_rule::REGISTRATION_RULE;

pub struct Control;
pub struct ControlFrame;

pub trait ControlHandle {
    type Parts;

    fn paint(&self, parts: &Self::Parts) -> ControlFrame;
}

pub trait ActionPartsContract {
    fn action_id(&self) -> &str;
}

crate::root_object! {
    kind: Control,
    needs_registry: true,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
    registry_rule_path: "src/control/registry_rule/registry_rule.rs",
    registry_rule: REGISTRATION_RULE,
}
```

`needs_registry: true` is the declaration that lets Control receive children.
`parent` places Control below the package root. Nothing here lists Button; a
second valid child does not require an edit to `control.rs`.

The parent's rule describes a minimum shape, not a kind allowlist:

```rust
// src/control/registry_rule/registry_rule.rs
use crate::RegistrationRule;

pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()
    .require_preset("ActionParts")
    .require_parts(&["label", "action"])
    .require_exports(&["control.render"])
    .require_handle_traits(&["ControlHandle"])
    .require_part_traits(&["ActionPartsContract"]);
```

This Button satisfies that rule:

```rust
// src/control/object/button/button.rs
use crate::control::{ActionPartsContract, ControlFrame, ControlHandle};
use crate::{PartsContract, PresetContract};

pub struct Button;
pub struct ActionParts;

pub struct ButtonParts {
    pub label: String,
    pub action: String,
    pub tooltip: Option<String>, // extra structure is allowed
}

impl PresetContract for ActionParts {
    type Output = ButtonParts;
    const REQUIRED_PARTS: &'static [&'static str] = &["label", "action"];
}

impl PartsContract for ButtonParts {
    type Output = ButtonParts;
    const PROVIDED_PARTS: &'static [&'static str] = &["label", "action", "tooltip"];
}

impl ControlHandle for Button {
    type Parts = ButtonParts;

    fn paint(&self, parts: &ButtonParts) -> ControlFrame {
        let _ = (&parts.label, &parts.action);
        ControlFrame
    }
}

impl ActionPartsContract for ButtonParts {
    fn action_id(&self) -> &str {
        &self.action
    }
}

crate::control_object! {
    kind: Button,
    preset: ActionParts,
    parts: ButtonParts,
    handle: Button,
    parent: crate::control::NODE_ID,
    exports: ["control.render"],
    handle_traits: ["ControlHandle"],
    handle_contracts: [crate::control::ControlHandle],
    part_traits: ["ActionPartsContract"],
    part_contracts: [crate::control::ActionPartsContract],
}
```

The `control_object!` name mirrors the parent folder, making the relationship
visible in source. The source of truth used to build the tree is
`parent: crate::control::NODE_ID`. The build step checks the macro name, folder
position, and parent together, so accidentally wiring Button to another
Registry fails before generated code is compiled.

Four layers validate this declaration:

| Check | Enforced by | What it proves here |
| --- | --- | --- |
| Parent topology | `nichlink-build` | Button's macro, folder, and `parent` all point to Control |
| Rust type contract | rustc | Both associated `Output` types are `ButtonParts`, and the real trait impls exist |
| Parent registration rule | Aggregated build diagnostics, generated const checks, and the development Registry | Preset, parts, export, and interfaces are at least the Control minimum |
| External admission | Registry connector | Cross-tree `requires` stay within Control's allowed `admission` paths |

`handle_traits` and `part_traits` are interface names retained in registration
metadata. The matching `handle_contracts` and `part_contracts` are Rust trait
paths, which make rustc prove that the implementations exist. They do not
create trait objects or vtables.

The following child is intentionally invalid. Assume `WrongPreset` and
`BrokenParts` implement `PresetContract` and `PartsContract` with the same
output type, so this example isolates failures against the parent rule:

```rust
crate::control_object! {
    kind: BrokenButton,
    preset: WrongPreset,
    parts: BrokenParts,
    parent: crate::control::NODE_ID,
    exports: ["control.preview"],
}
```

One `cargo check` reports the declaration site together with the missing
`ActionParts` preset, `control.render` export, `ControlHandle`, and
`ActionPartsContract`. Removing only `action` from
`BrokenParts::PROVIDED_PARTS` is rejected by the generated const check. Keeping
`handle_contracts: [crate::control::ControlHandle]` while deleting the real
impl produces a rustc trait-bound error at the declaration. Those paths cover
a missing declaration, a missing structural constant, and a missing Rust
implementation without pretending they are the same error.

Parentage is not selected by a kind filter, and external use is not controlled
by the registration rule:

| Layer | Question it answers |
| --- | --- |
| Rust `struct` / `impl` | How does the object actually work? |
| `parent` + parent-specific macro | Where is the object registered? |
| `registry_rule` | What is the minimum shape accepted by the parent Registry? |
| `admission` | Which external paths may this branch depend on? |
| `FlowContract` | Are both sides of a graft data-compatible? |

Older prototypes used `RegistrationRule::new(&["Button"], &[])`. That form is
gone. Replace it with `RegistrationRule::new()` plus only the required shape
methods shown above. Move external path allow/deny entries to `Admission`; do
not move the old kind list there, because kind never decides parentage.

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
consumers, describe the affected slots as one plan and commit them together.

The three participants stay separate. The framework tree and third-party
implementation remain in their own crates. The host only declares an overlay
plan in its `main.rs` or `lib.rs`:

```text
framework crate                         external graft crate
src/                                    src/
└── control/                            └── button_fast/
    ├── control.rs                          └── button_fast.rs
    └── object/button/...                        │
             │                                   │
             └──── immutable base Registry       └──── external Registry
                                  \               /
                                   GraftPlan + overlay
                                            │
                                    effective Registry
                                            │
                                  host crate: src/main.rs
```

The host entry declares cuts and external selectors. It never copies framework
or third-party source. The static macro constructs no `Vec` or `String`; the
builder writes its contents directly into the `StaticPlan`:

```rust
use nichlink_core::{FrameworkId, Registry};

const FRAMEWORK: FrameworkId = FrameworkId::new("nichui");

nichlink_core::static_graft_plan!(FRAMEWORK,
    cut "root/control/button" graft "button_fast",
);

fn registry_for_this_run(base: &Registry, external: &Registry) -> Registry {
    base.overlay_static(builtin_static_plan().grafts(), external)
        .expect("checked graft")
}
```

`overlay` returns a new effective Registry. Neither `base` nor `external` is
modified. If any cut fails its flow contract, destination parent rule,
admission, or connector check, none of the plan is published. `overlay_static`
skips dynamic `GraftPlan` and selector-string allocation. If the external
implementation is loaded at runtime, validation and effective Registry
construction still happen once.

A normal cut replaces one node and keeps the base node's children:

```text
base                              cut A/a1 graft a1_fast
A                                 A
├── a1                            ├── a1_fast       # only a1 is replaced
│   ├── b1                        │   ├── b1         # inherited from base
│   └── b2                        │   └── b2         # inherited from base
├── a2                            ├── a2
└── a3                            └── a3
```

`full` explicitly drops the base subtree and uses the external subtree:

```text
external                          cut A/a1 full graft a1_fast
a1_fast                           A
└── bx                            ├── a1_fast
                                  │   └── bx         # supplied by external
                                  ├── a2
                                  └── a3
```

When several data boundaries must change together, put all cuts in one static
declaration:

```rust
nichlink_core::static_graft_plan!(FRAMEWORK,
    cut ["root/canvas"] graft "canvas_fast",
    cut ["root/hit_test"] graft "hit_test_fast",
    cut ["root/layout"] graft "layout_fast",
);
```

Use the dynamic `graft_plan!` expression only when an editor, command, or hot
reload path must construct and modify a plan at runtime.

A path range can target contiguous siblings under one parent Registry:

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

## Static release plans, performance, and two-stage pruning

NichLink's two pruning stages solve different problems.

The first stage runs before rustc expands the generated module tree. Its unit is
a complete registration face:

1. the host crate's thin `build.rs` calls `nichlink-build`;
2. the builder reads folder-backed faces and the entry in `main.rs`, `lib.rs`,
   or `application!`;
3. it conservatively derives the faces needed by this crate and emits only
   those faces into the generated modules and `StaticPlan`;
4. dynamic dispatch, generated code, or an unresolved path forces a full-tree
   fallback instead of an unsafe deletion.

This reduces registration faces and metadata sent to rustc. It is not a
complete rustc call graph, and it does not remove individual functions from an
active face.

The second stage is the normal Rust toolchain. Rustc reachability and
monomorphisation, LLVM, ThinLTO, and linker garbage collection remove unused
functions and symbols. The workspace release profile enables ThinLTO with one
codegen unit. `tools/nichlink-release-audit` checks release artifacts, rejects a
remaining `.inventory` linker section, and can compare the symbols and bytes of
full and minimal application binaries. NichLink does not present source-level
function-name matching as compiler-accurate elimination.

A release build also does not reconstruct the built-in Registry at startup.
Generated code stores checked topology as `&'static [StaticFace]` and
build-declared grafts as `&'static [StaticGraftCut]` in the same `StaticPlan`.
`builtin_static_plan()` borrows that read-only data directly. There is no heap
allocation, global constructor, inventory walk, or startup registration loop.
Build-time `registry_rule` checks do not become per-frame runtime checks.

Static does not mean every operation in every configuration is free. It means
the costs are explicit:

| Usage | Runtime representation | Cost boundary |
| --- | --- | --- |
| Read-only built-in topology | Static `StaticFace` slice | No startup allocation; `find` is O(log n), while `children_of` currently filters in O(n) |
| Mutable development Registry | `Arc` header, 32 entry pages, and indexes | Cloning increments `Arc` counts; the first write copies only the touched page, not the tree |
| Build-declared static graft | Static selector slice inside `StaticPlan` | Reading the declaration allocates nothing; a framework with statically bound implementations needs no Registry overlay |
| Post-release plugin/graft | Selected dynamic metadata and an effective Registry | `overlay_static` allocates no plan but still performs contract, admission, and connector validation once |
| `CallTrace::runtime()` | `errors-only` in debug, `off` in release | Off collects no evidence; code that still calls tracing APIs is not promised instruction-level zero overhead |

The Registry page-copy concern therefore does not affect startup when an
application only reads the built-in static plan. Page-level COW is used only
when an application explicitly builds a mutable Registry, enables runtime
plugins, or applies a graft. Studio, MCP, debug, and plugin-host are separate
tools or optional dependencies; an application that does not link them does
not carry them in its binary.

NichLink core does not rewrite arbitrary Rust call sites. A framework that
wants a fully static implementation binding uses its own generated layer to
select a concrete Rust type or function from these checked selectors. A
runtime plugin has no implementation to link ahead of time, so it still needs
one overlay. This distinction avoids presenting dynamic loading as compile-time
magic.

Results depend on hardware, filesystem, and face contents. The repository
ships reproducible checks instead of a fixed benchmark claim:

```sh
# Build and index a large Registry
cargo run --release -p nichlink-core --example scale_audit -- 100000

# fmt, tests, Clippy, docs, release artifacts, symbols, and linker sections
tools/nichlink-release-audit

# Optional: compare two real application artifacts
NICH_LINK_FULL_BINARY=/path/to/full \
NICH_LINK_MINIMAL_BINARY=/path/to/minimal \
tools/nichlink-release-audit
```

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

The command-line surface is intentionally small. `nichlink` is the unified
entry point: `nichlink new` scaffolds host projects, `nichlink studio` is the
interactive authoring/debug surface, and `nichlink mcp` is the read-only
JSON-RPC/MCP bridge for AI clients. `cargo check` remains the build validation
command:

```sh
cargo check
NICH_LINK_PACKAGE_ROOT=/work/my-app nichlink mcp
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
cli/          nichlink-cli: unified entry (nichlink new/studio/mcp, cargo-nichlink)
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
