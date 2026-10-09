# xirang-toolchain

The publishing surface: the seven thin execution surfaces of XiRang in one crate.
合并后的发布面：XiRang 的七个薄执行面合成的一个 crate。

| module | was | responsibility |
| --- | --- | --- |
| `build_method` | `xirang-build-method` | build-time filesystem / `OUT_DIR` orchestration |
| `runtime` | `xirang-run-method` | runtime state instance + trace binding |
| `call_evidence` | `xirang-debug-method` | observation evidence surface |
| `plugin_host` | `xirang-plugin-host` | wasm / process plugin host execution |
| `studio` | `xirang-studio` | ratatui authoring / inspection surface |
| `mcp` | `xirang-mcp` | AI-agent stdio bridge |
| `cli` | `xirang-cli` | process glue: argv dispatch, cargo subprocesses |

Host usage: `[dependencies] xirang-toolchain` + `[build-dependencies] xirang-toolchain`;
the crate root calls `xirang_toolchain::run_method::host!();` and the thin `build.rs` calls
`xirang_toolchain::build_method::run()`.
宿主用法：`[dependencies] xirang-toolchain` + `[build-dependencies] xirang-toolchain`；
crate 根部调用 `xirang_toolchain::run_method::host!();`，薄 `build.rs` 调用
`xirang_toolchain::build_method::run()`。

Feature names are kept from the crates that carried them (`wasm`, `process-tools`, `authoring`,
`node-graph`, `prototype-fixtures`, `dev-supervisor`); `default = ["wasm", "node-graph"]`
keeps each surface's previous default behaviour.
特性名沿用原先携带它们的 crate（`wasm`、`process-tools`、`authoring`、`node-graph`、
`prototype-fixtures`、`dev-supervisor`）；`default = ["wasm", "node-graph"]` 保持各面原先的默认行为。
