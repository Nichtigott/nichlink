# nichlink-toolchain

The publishing surface: the seven thin execution surfaces of NichLink in one crate.
合并后的发布面：NichLink 的七个薄执行面合成的一个 crate。

| module | was | responsibility |
| --- | --- | --- |
| `build_time` | `nichlink-build-method` | build-time filesystem / `OUT_DIR` orchestration |
| `runtime` | `nichlink-run-method` | runtime state instance + trace binding |
| `call_evidence` | `nichlink-debug-method` | observation evidence surface |
| `plugin_host` | `nichlink-plugin-host` | wasm / process plugin host execution |
| `studio` | `nichlink-studio` | ratatui authoring / inspection surface |
| `mcp` | `nichlink-mcp` | AI-agent stdio bridge |
| `cli` | `nichlink-cli` | process glue: argv dispatch, cargo subprocesses |

Host usage: `[dependencies] nichlink-toolchain` + `[build-dependencies] nichlink-toolchain`;
the crate root calls `nichlink_toolchain::runtime::host!();` and the thin `build.rs` calls
`nichlink_toolchain::build_time::run()`.
宿主用法：`[dependencies] nichlink-toolchain` + `[build-dependencies] nichlink-toolchain`；
crate 根部调用 `nichlink_toolchain::runtime::host!();`，薄 `build.rs` 调用
`nichlink_toolchain::build_time::run()`。

Feature names are kept from the crates that carried them (`wasm`, `process-tools`, `authoring`,
`node-graph`, `prototype-fixtures`, `dev-supervisor`); `default = ["wasm", "node-graph"]`
keeps each surface's previous default behaviour.
特性名沿用原先携带它们的 crate（`wasm`、`process-tools`、`authoring`、`node-graph`、
`prototype-fixtures`、`dev-supervisor`）；`default = ["wasm", "node-graph"]` 保持各面原先的默认行为。
