# nichlink-cli

`nichlink-cli` 是 NichLink 工具链的统一入口。

```sh
cargo install --git https://github.com/Nichtigott/nichlink nichlink-cli
nichlink new my-app
cd my-app && nichlink studio
```

## 命令

| 命令 | 作用 |
| --- | --- |
| `nichlink new <name> [--lib] [--path <workspace> \| --git <url>]` | 在 `./<name>` 生成 NichLink 宿主项目 |
| `nichlink studio` | 为当前项目启动 Studio TUI |
| `nichlink mcp` | 运行只读 MCP stdio 桥 |

依赖来源自动检测：从 NichLink checkout 运行的 CLI 写入 path 依赖；
全局安装的 CLI 写入 Git 依赖（带版本下限，发布后 Cargo 可解析到
crates.io）。可用 `--path` 或 `--git` 显式覆盖。

同一 package 内的 `cargo-nichlink` 二进制注册插件形式：
`cargo nichlink studio` 等价于 `nichlink studio`。

库目标（`nichlink_cli`）承载命令分发逻辑，其他二进制可以复用。

English: [README.md](README.md)
