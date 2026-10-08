# nichlink-toolchain

合并后的发布面：NichLink 的七个薄执行面合成的一个 crate。

| 模块 | 原 crate | 职责 |
| --- | --- | --- |
| `build_method` | `nichlink-build-method` | 构建期文件系统 / `OUT_DIR` 编排 |
| `runtime` | `nichlink-run-method` | 运行期状态实例与 trace 绑定 |
| `call_evidence` | `nichlink-debug-method` | 观测证据面 |
| `plugin_host` | `nichlink-plugin-host` | wasm / process 插件宿主执行 |
| `studio` | `nichlink-studio` | ratatui authoring / 检视面 |
| `mcp` | `nichlink-mcp` | AI-agent stdio 桥 |
| `cli` | `nichlink-cli` | 进程粘合：argv 分派、cargo 子进程 |

宿主用法：`[dependencies] nichlink-toolchain` + `[build-dependencies] nichlink-toolchain`；
crate 根部调用 `nichlink_toolchain::run_method::host!();`，薄 `build.rs` 调用
`nichlink_toolchain::build_method::run()`。
Host usage: the crate root calls `nichlink_toolchain::run_method::host!();` and the thin `build.rs`
calls `nichlink_toolchain::build_method::run()`.

特性名沿用原先携带它们的 crate（`wasm`、`process-tools`、`authoring`、`node-graph`、
`prototype-fixtures`、`dev-supervisor`）；`default = ["wasm", "node-graph"]` 保持各面原先的默认行为。
