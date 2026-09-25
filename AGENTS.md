# NichLink architecture notes

NichLink splits the workspace into one pure kernel and thin execution surfaces.
The rule that decides where new code goes:

**Kernel = protocol vocabulary + pure methods (verbs), no execution.**
Anything that reads the filesystem, touches `std::env`, spawns processes,
drives a terminal, or binds to process lifetime belongs in an execution
surface that binds kernel methods to its own context.

## Crates

| Crate | Directory | One-line responsibility |
| --- | --- | --- |
| `nichlink-core` (lib `nichlink`) | `core/` | Kernel: protocol nouns and pure methods |
| `nichlink-build-method` | `build_method/` | Build-time filesystem / `OUT_DIR` orchestration |
| `nichlink-run-method` | `run_method/` | Runtime state instance + trace binding |
| `nichlink-debug-method` | `debug_method/` | Observation evidence surface |
| `nichlink-macro` | `macro/` | Compile-time face field front end: accepted order, tolerant separators, diagnostics |
| `nichlink-plugin-host` | `plugin-host/` | Wasm/process plugin host execution |
| `nichlink-studio` | `studio/` | Ratatui authoring/inspection surface |
| `nichlink-mcp` | `mcp/` | AI-agent stdio bridge (currently read-only) |
| `nichlink-cli` | `cli/` | Process glue: argv dispatch, cargo subprocesses |
| `nichlink-conventions` | `conventions/` | Repository-convention gates (`publish = false`: they walk the checkout, so an unpacked copy would have nothing to check) |

Feature names intentionally follow each crate's own vocabulary instead of one
workspace-wide scheme: `core` gates the parser as `syntax`, `run_method` gates
the authoring executor as `authoring`, `studio` gates test-only fixtures as
`prototype-fixtures`, and `plugin-host` uses `wasm` / `process-tools`. Do not
rename a feature to match another crate: they are public API. `prototype-fixtures`
gates test-only fixtures — the source-only host package under
`studio/tests/fixtures/node-editor/` — and is not part of the published surface.
It is off by default, so the CI job that runs the workspace with
`--all-features` is the only gate that exercises it; keep that job green.
`studio` also gates its `nichlink-dev` rebuild supervisor behind the
non-default `dev-supervisor` feature: that binary rebuilds Studio from this
checkout, so an installed copy could never work, and `required-features` is the
per-target equivalent of `publish = false` — it is what keeps
`cargo install nichlink-studio` from shipping a tool with nothing to rebuild.
特性名有意遵循各 crate 自己的词汇，而不是全工作区统一命名：`core` 把解析器门控为
`syntax`，`run_method` 把 authoring 执行器门控为 `authoring`，`studio` 把仅测试用的
fixture 门控为 `prototype-fixtures`，`plugin-host` 使用 `wasm` / `process-tools`。
不要为对齐其他 crate 而重命名特性：它们是公开 API。`prototype-fixtures` 只门控测试用
fixture——`studio/tests/fixtures/node-editor/` 下仅源码的宿主包——不属于发布表面。它默认
关闭，因此只有以 `--all-features` 跑整个工作区的 CI 任务会碰它；保持那个任务常绿。
`studio` 还把 `nichlink-dev` 重建监督器门控在非默认的 `dev-supervisor` 特性之后：该二进制
从本检出重建 Studio，安装副本永远做不到，而 `required-features` 就是按 target 的
`publish = false` 等价物——正是它让 `cargo install nichlink-studio` 不会带出一个没有东西可
重建的工具。

Host usage: `[dependencies] nichlink-run-method` +
`[build-dependencies] nichlink-build-method`; the crate root calls
`nichlink_run_method::host!();` and the thin `build.rs` calls
`nichlink_build_method::run()`. `run_method`'s `entry` module was removed: the
build-time entry is now `host!()` plus `build.rs`, and `application!(entry = …)`
remains only as the optional source-scope discovery hint.
宿主用法：`[dependencies] nichlink-run-method` +
`[build-dependencies] nichlink-build-method`；crate 根部调用
`nichlink_run_method::host!();`，薄 `build.rs` 调用
`nichlink_build_method::run()`。`run_method` 的 `entry` 模块已移除：构建期入口现在是
`host!()` 加 `build.rs`，`application!(entry = …)` 仅作为可选的源码范围发现提示保留。

## Kernel modules (`core/src/registry_core/`)

- `identity` — stable ids, hashing (SHA-256), namespaces
- `declaration` — face/object vocabulary
- `tree` — registry tree operations, passive recursive registration
- `syntax` — graft syntax, application/graft entry parsing
- `authoring` — face authoring parse/validation/snapshot (fs executor lives in run_method)
- `diagnostic` — `RegistryError`, `RegistrationState`, build diagnostics, topology checks
- `requirements` — capability declarations and requirements
- `plugin` — plugin protocol, slot/channel validation
- `mir` — MIR text/JSONL parsing, call-graph merge
- `source` — lexical source scanning (functions, spans, calls)
- `release` — release-time pruning/plan vocabulary
- `json` — shared JSON string encoding for every artifact this workspace writes (build diagnostics, MIR JSONL, editor snippets)
- `lexicon` — shared text contracts (generated-entry file name, runtime crate name, environment variables, `.nichlink` paths, scope exemptions)

Module mounting is uniform workspace-wide: a module file is declared by its parent
with `#[path = "<dir>/<name>.rs"] pub mod <name>;`. There is no `mod.rs`. The
`#[path]` is what gives a module directory-relative resolution for its own
children — the behaviour `mod.rs` would give — which is why the `<dir>/<name>.rs`
layout works without one. A bare `mod x;` is equally correct when the parent is a
crate root or is itself loaded with `#[path]`, and the tree uses that form in both
positions (the count is not restated here: it changes with every new module);
the rule that actually matters is that a module is never spliced in with
`include!`. A splice changes the `file!()` a face records and therefore its
`NodeId`, silently; identities are written into on-disk graft records, so the
damage would surface later as records that no longer resolve. The only `include!`
is `host!()`'s `include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))`, which
pulls in the generated plan rather than a source module. The `conventions` crate
gates both rules.
模块挂载在整个工作区统一：模块文件由其父模块用
`#[path = "<dir>/<name>.rs"] pub mod <name>;` 声明，没有 `mod.rs`。`#[path]` 的作用是让
模块以所在目录为基准解析自己的子模块——也就是 `mod.rs` 能给出的行为——这正是
`<dir>/<name>.rs` 布局无需 `mod.rs` 的原因。当父文件是 crate 根或本身经 `#[path]` 载入时，
裸 `mod x;` 同样正确，树里两种位置都有这种写法（这里不再复述具体数字：它随每个新模块变化）；真正要守的规则是绝不用 `include!` 把模块拼进来。
拼接会改变注册面记录的 `file!()`，从而静默改变它的 `NodeId`；身份会写入落盘的 graft 记录，
损害会在以后表现为不再解析的记录。唯一的 `include!` 是 `host!()` 的
`include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))`，它引入的是生成的计划而不是源码
模块。两条规则都由 `conventions` crate 门控。

`core`'s crate root re-exports the module hierarchy plus a whitelist of vocabulary
that host code, the build step, and the runtime-check surfaces write as a bare
name; every other noun is reached through its module path
(`nichlink::identity::NodeId`). The module hierarchy is the official path.
`core` 的 crate 根部重导出模块层级，外加一份白名单词汇——宿主代码、构建步骤与运行期
校验面会以裸名书写它们；其余名词一律经模块路径取得（`nichlink::identity::NodeId`）。
模块层级就是官方路径。

## Change rules

1. New pure logic goes into a kernel module under `core/src/registry_core/<name>/`
   and is registered in `core/src/registry_core.rs`.
2. Execution surfaces keep historical paths through shim re-exports
   (`pub use nichlink::tree; pub use nichlink::tree::*;`) — do not move public
   API paths without updating the shims.
3. Kernel code must not do I/O, read `std::env`, or depend on process lifetime.
   If it needs such a value, take it as a parameter.
4. Keep crate names, directory names, and lib names consistent
   (`nichlink-<x>` / `<x>/` / `nichlink_<x>`); scaffold templates in
   `build_method/` and CI reference them too.

## Verify

```sh
cargo fmt --all
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
tools/nichlink-publish --check-table
```

The last line is the release tables' gate: `tools/nichlink-publish` publishes in a
hand-maintained dependency order, so it derives the truth from the manifests and
compares both of its tables against them. It needs no network and no token.
最后一行是发布表的门禁：`tools/nichlink-publish` 按一份手工维护的依赖顺序发布，因此它从清单
推导事实，并把自己那两张表与清单对比。它不需要网络也不需要 token。

The three `cargo` commands run with `--offline` because this checkout is
developed offline. CI has a network and deliberately does **not** pass the flag: a
cold runner has an empty registry cache, so `--offline` there would fail every job
instead of making it stricter. The table check takes no such flag: it reads only
this checkout.
那三条 `cargo` 命令带 `--offline`,因为本检出在离线环境下开发。CI 有网络,因此有意**不**传
该标志:冷启动的 runner 注册表缓存为空,在那里加 `--offline` 只会让每个任务失败,而不是让它
更严格。表检查不需要这类标志:它只读本检出。

`cargo test --workspace` also runs the gates in the `conventions` crate, so the
default gate already fails on: I/O in `core/src`, a `mod.rs`, a second `include!`,
a file pushed past the 450-line ratchet, a missing `#![warn(missing_docs)]`, an
`#[allow(missing_docs)]`, or a fenced Rust block in a README that no longer
parses. Add a new repository-wide rule there rather than to a prose document. The
kernel's parse entries carry a nesting guard for the same reason — a stack
overflow is not a `Result`, so a pathological source would take the whole surface
down — and `core/tests/nesting_budget.rs` is the other half of it: the guard must
refuse nothing this repository ships. Adding a shape to the guard means adding a
case to `deep_input_tests.rs` and re-running that gate. CI additionally runs `cargo test --workspace --all-features --doc`: `--all-targets`
skips doctests, and the `authoring`-gated `compile_fail` pin only exists under
`--all-features`.
`cargo test --workspace` 也会跑 `conventions` crate 里的门禁,因此默认门禁已经会在下列情形
失败:`core/src` 里出现 I/O、出现 `mod.rs`、出现第二个 `include!`、文件越过 450 行棘轮、缺少
`#![warn(missing_docs)]`、出现 `#[allow(missing_docs)]`、或 README 里有不再能解析的 Rust
围栏。新增全仓规则请加到那里,而不是加到散文文档里。内核的解析入口同样带一道嵌套守卫——
栈溢出不是 `Result`,畸形源码会带走整个执行面——而 `core/tests/nesting_budget.rs` 是它的
另一半:守卫不得拒绝本仓库出厂的任何东西。给守卫加一种形状,就要在 `deep_input_tests.rs` 里
加一条用例,并重跑那道门禁。CI 另跑
`cargo test --workspace --all-features --doc`:`--all-targets` 会跳过 doctest,而门控在
`authoring` 之后的 `compile_fail` 钉子只在 `--all-features` 下存在。

`tools/nichlink-external-rehearsal` copies the two example hosts outside the
checkout, repoints their `path` dependencies here, detaches them from the
workspace and builds them from scratch. Every other check builds the examples
where the workspace's members, relative paths, shared `target/` and shared
lockfile all happen to be right; this is the one that fails when a host layout
assumption only holds in-tree. `tools/nichlink-publish --verify-consumers` is its
counterpart for published crates, and it needs the index.
`tools/nichlink-external-rehearsal` 把两个示例宿主复制到检出之外，把它们的 `path` 依赖指向
这里，让它们脱离工作区并从零构建。其他所有检查都在"工作区成员、相对路径、共享 `target/` 与
共享 lockfile 恰好都成立"的地方构建示例；只有这一条会在"宿主布局假设只在树内成立"时失败。
`tools/nichlink-publish --verify-consumers` 是它在已发布 crate 上的对应物，那条需要 index。

`tools/nichlink-package-audit` checks two different things. Package *contents* —
every `src/**/*.rs` module and the declared README must be in the package — are
verified for all nine crates from today, because `cargo package --list` needs no
registry. Packaging itself (building the tarball in isolation) still waits for
each crate's versioned `nichlink-*` dependencies to reach the index, so eight
crates are skipped until the first publish. A module that cargo does not pick up
is a crate that compiles locally and nowhere else, which is why the content half
exists.
`tools/nichlink-package-audit` 检查两件不同的事。包**内容**——每个 `src/**/*.rs` 模块与
清单声明的 README 都必须在包里——从今天起对九个 crate 全部生效，因为
`cargo package --list` 不需要 registry。打包本身（从 tarball 隔离构建）仍要等各 crate 的
带版本号 `nichlink-*` 依赖进入 index，因此首次发布前有八个 crate 被跳过。cargo 没收进去的
模块等于一个只在本地编译得过、别处都编译不过的 crate，这正是内容检查存在的原因。

All nine crates carry `#![warn(missing_docs)]`, so the clippy gate above already
refuses an undocumented public item: document it (English `///` then Chinese
`///`) rather than adding `#[allow(missing_docs)]`. Three workspace members set
`publish = false` — `conventions` and the two example hosts; of those, the two
example hosts are not linted (a host is expected to document its own types),
while `conventions` carries the lint along with the published crates.
九个 crate 都开了 `#![warn(missing_docs)]`，因此上面的 clippy 门禁已经会拒绝没有文档的
公开项：请补上文档（先英文 `///` 再中文 `///`），不要加 `#[allow(missing_docs)]`。有三个
工作区成员设了 `publish = false`——`conventions` 与两个示例宿主；其中两个示例宿主没有开该
lint（宿主自己的类型由宿主负责文档化），而 `conventions` 与已发布的 crate 一样带着它。
