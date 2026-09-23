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
| `nichlink check [path] [--json]` | 不做编译，运行注册发现与校验 |
| `nichlink build [path] [cargo 选项]` | 先校验注册树，再运行 `cargo build` |
| `nichlink explain <node-id \| logical/path> [--path <dir>] [--json]` | 报告单个节点的身份、构建作用域、剪枝状态，以及命名它的声明切口 |
| `nichlink explain --overlay [--path <dir>] [--json]` | 渲染每个槽位与计划的静态覆盖投影（不是活的树） |
| `nichlink grafts [path] [--json]` | 列出 `.nichlink/external-grafts/*/graft.plan` 及其目标，以及宿主入口是否声明该槽位 |
| `nichlink studio` | 为当前项目启动 Studio TUI |
| `nichlink mcp` | 运行只读 MCP stdio 桥 |

依赖来源自动检测：从 NichLink checkout 运行的 CLI 写入 path 依赖；
全局安装的 CLI 写入 Git 依赖（带版本下限，发布后 Cargo 可解析到
crates.io）。可用 `--path` 或 `--git` 显式覆盖。

同一 package 内的 `cargo-nichlink` 二进制注册插件形式：
`cargo nichlink studio` 等价于 `nichlink studio`。

库目标（`nichlink_cli`）承载命令分发逻辑，其他二进制可以复用。

## `nichlink check --json`

`nichlink check` 拥有这份契约（`cli/src/commands/check.rs`）。带 `--json` 时：

- 成功时 stdout 恰好写出一份 JSON 文档，退出码为 0。
- 失败时把同一份、包含全部诊断的文档写到 stdout，随后命令返回 `Err`，进程仍以非零码
  退出（`nichlink` 向 stderr 打印
  `registration check failed (N diagnostic(s); JSON on stdout)` 后退出 1）。

文档形状由 `BuildDiagnostics::to_json` 给出
（`core/src/registry_core/diagnostic/build.rs`）：

```json
{"schema":"nichlink.build-diagnostics/1","count":N,"diagnostics":[...]}
```

每条诊断对象始终携带全部十一个键，顺序为 `phase`、`branch`、`node`、`source`、
`line`、`function`、`field`、`expected`、`actual`、`provider`、`message`。读取方无需区分
"缺失"与"空"：诊断没有行号时 `line` 为 `0`。人类可读渲染器（同文件的
`render_build_item`）则省略空字段，因此 JSON 中的空字段表示"对该诊断不适用"，而不是
"缺键"。（`branch` 是例外：它为空时渲染器打印 `branch=<unknown>`。）

`diagnostics` 的顺序即 `BuildDiagnostics::iter` 的顺序，也就是人类可读文本的顺序；完全
相同的重复项会被折叠，因此 `count` 是去重后的条数。`phase` 是稳定的机器字符串；人类
可读渲染器把 `requirements` 映射为 `requirements / 注册需求`，`contract` 映射为
`contract / 注册合同`，`stable-identity` 映射为 `stable identity / 稳定标识`，
`static-plan` 映射为 `static plan / 静态计划`，其余取值原样透传。

### 各 phase 的字段

| `phase` | 构造位置 | `branch` | `node` | `source` | `line` | `function` | `field` | `expected` | `actual` | `provider` | `message` |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `requirements` | `core/src/registry_core/requirements/requirements.rs:70` | 总是 | 总是 | 总是 | 总是 | 总是 | 总是 | 总是 | 从不 | 仅当某祖先以不同的 kind 提供该能力时（`:80-82`） | 总是 |
| `contract` | `build_method/src/contracts.rs:136`、`:173`、`:192`、`:216` | 从不 | 总是 | 总是 | 总是 | 总是 | 总是 | 总是 | 仅输出合同不匹配时（`:142`） | 从不 | 总是 |
| `stable-identity` | `build_method/src/validation.rs:149` | 从不 | 从不 | 总是 | 总是 | 从不 | 总是 | 总是 | 总是 | 从不 | 总是 |
| `parent-macro` | `build_method/src/validation.rs:59`、`:85`、`:113` | 从不 | 从不 | 总是 | 总是 | 从不 | 总是 | 总是 | 总是 | 从不 | 总是 |
| `static-plan` | `build_method/src/static_plan.rs:131`；`build_method/src/graft_plan_check.rs:157`；`core/src/registry_core/diagnostic/topology.rs:41`、`:49`、`:68` | 从不 | 从不 | 总是 | 总是 `0` | 从不 | 仅三项拓扑检查 | 仅 missing-parent 与 no-registry 两项 | 仅三项拓扑检查 | 从不 | 总是 |
| `face-cfg` | `build_method/src/static_plan.rs:109` | 从不 | 从不 | 总是 | 总是 | 从不 | 从不 | 从不 | 从不 | 从不 | 总是 |
| `out-dir` | `build_method/src/lib.rs:158` | 从不 | 从不 | 从不 | 总是 `0` | 从不 | 从不 | 从不 | 从不 | 从不 | 总是 |

有一条 `static-plan` 诊断来自 graft 计划交叉校验（`build_method/src/graft_plan_check.rs`）：
它的 `source` 指向有问题的 `.nichlink/external-grafts/<selector>/graft.plan`，并在
`message` 里带上可直接粘贴的 `static_graft_plan!` 子句。

`static-plan` 的三项拓扑检查是 `parent node is missing`、`parent does not own a
registry`、`parent cycle detected`；三者都设置 `field=parent` 与 `actual`，前两者还设置
`expected`，成环检查不设置。第四条 `static-plan` 诊断 `parent declaration cannot be
resolved`（`static_plan.rs:131`）只设置 `phase`、`source`、`line=0` 与 `message`。

English: [README.md](README.md)
