# NichLink

[English](README.md) | 简体中文

NichLink 是一个 Rust 优先的声明式注册协议，用于递归注册对象、检查注册合同，并在中间层执行原子嫁接。对象在自己的文件中声明父 Registry，父模块不维护子对象清单。

## 最小声明

```rust
pub struct Workspace;

crate::root_object! {
    kind: Workspace,
    needs_registry: true,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
}

pub struct Button;

crate::workspace_object! {
    kind: Button,
    parent: crate::workspace::NODE_ID,
}
```

`name`、`summary`、`parts` 和 `registry_name` 都有默认值。显式 `parent` 会与宏名互相校验。只有需要边界控制时，才填写 `needs_registry`、`admission`、`registry_rule` 或输入/输出合同。

宿主的 `build.rs` 只需保留一次性入口：

```rust
fn main() {
    nichlink_build::run();
}
```

`nichlink-build` 从宿主目录发现注册面，并在 rustc 类型检查前生成 `StaticPlan`。这是 Cargo 的边界：依赖 crate 不能自动读取使用者的 `main.rs`，宿主仍需提供构建入口。

## 第一次使用

在当前仓库直接启动开发 Studio：

```sh
cd prototypes/registation_test/nichlink
cargo run -p nichlink-studio
```

启动后按 `n` 可以直接新建项目。向导会询问目录、包名和类型
（`binary` 或 `library`），确认后只生成 Cargo、`build.rs` 和入口文件。
注册树默认保持为空；按 `a` 创建第一个根注册面。如果该注册面声明需要注册机，
Studio 才会在它旁边生成自己的 `registry_rule/registry_rule.rs`。随后 Studio 会自动
切换到新项目，无需手动创建目录或再设置环境变量。

生成入口会为每个注册机创建一个声明宏：`root_object!` 把注册面挂到根，
`<parent>_object!` 把子对象挂到对应父注册机。注册文件保留明确的
`crate::...!` 路径，rust-analyzer 可以在项目内部提供宏字段补全。core 和 build
仍是普通 Cargo 依赖；Cargo 不会把第三方源码复制到项目的 `src/`。

在自己的新应用中，项目尚未发布时先添加两个本地路径依赖：

```sh
cargo add nichlink-core --path /path/to/nichlink/core
cargo add nichlink-build --build --path /path/to/nichlink/build
```

应用根目录保留这个薄 `build.rs`：

```rust
fn main() {
    nichlink_build::run();
}
```

在 crate 根部只引入一次生成模块。下面的 `registry_core` shim 用来保持生成代码的名称稳定，它只是元数据接线，不是子对象清单：

```rust
// src/main.rs
pub mod registry_core {
    pub use nichlink_core::*;
}

include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"));

fn main() {
    println!("registered faces: {}", registrations().len());
}
```

每个注册声明放在规范目录中，例如 `src/workspace/workspace.rs`：

```rust
pub struct Workspace;

crate::root_object! {
    kind: Workspace,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
}
```

先运行一次 `cargo check` 生成静态计划。设置 `NICH_LINK_PACKAGE_ROOT` 后启动 Studio，就能检查或编辑该应用的 `src/`：

```sh
NICH_LINK_PACKAGE_ROOT=/work/my-app cargo run -p nichlink-studio
```

如果你已经位于宿主项目目录，也可以不设置环境变量：

```sh
cd /work/my-app
cargo run --manifest-path /path/to/nichlink/studio/Cargo.toml
```

不要求必须有 `main.rs`。只有库入口时会自动使用 `src/lib.rs` 做粗修入口；连 `main.rs` 和 `lib.rs` 都没有时，会保留安全的全树回退。直接启动 Studio 且没有设置 `NICH_LINK_PACKAGE_ROOT` 时，如果当前 workspace 没有宿主 `src/`，界面会显示空根节点，而不会把 NichLink 自己的 build/debug 源码误当成注册面。

### 如果你要做前端框架（库）

框架自己拥有注册面，并发布生成的静态计划：

```text
my-framework/
  build.rs
  src/lib.rs
  src/workspace/workspace.rs
  src/workspace/registry_rule/registry_rule.rs
  src/workspace/object/panel/panel.rs
```

每个拥有注册机的目录都可以有自己的 `registry_rule/` 文件夹；规则文件只描述
注册对象必须满足的结构，`admission` 字段单独控制它可以使用的外部对象，二者都
不负责登记子对象。旧项目中的
`registry/rules/rules.rs` 仍可读取，新增项目统一使用 `registry_rule/`。

`src/lib.rs` 使用前面相同的生成模块接线，但不需要 `main`：

```rust
pub mod registry_core {
    pub use nichlink_core::*;
}

include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"));
```

使用者只添加 `my-framework` 并调用它公开的 API，不需要复制框架的注册文件。如果使用者还要定义自己的注册面，就在自己的项目中单独添加 `build.rs` 和生成模块。Cargo 无法使用使用者的 `main.rs` 自动裁剪已经编译好的依赖库。

### 如果你要做主程序（二进制）

主程序只使用某个框架时，只添加该框架即可，不需要添加 `nichlink-build`。只有主程序自己还拥有注册面时，才添加 `nichlink-build`、薄 `build.rs` 和生成模块接线。此时 `src/main.rs` 可以像普通 Rust 程序一样调用生成在 crate 根部的 `registrations()` 或自己的业务 API。

## 工作区包

```text
core/         Registry、事务、合同、准入、graft、诊断和声明宏
build/        目录发现、清单解析、缓存、粗修和 StaticPlan
debug/        可选收集器、MIR 候选、CallTrace、数据流和图适配器
studio/       Ratatui 常驻界面、watch、搜索、源码跳转和编辑器联动
mcp/          面向 AI 的只读 stdio MCP 查询服务
plugin-host/  经校验的 Wasm/进程插件、懒加载和原子部署
```

`nichlink-core` 不依赖 inventory、Studio 或具体应用注册表，可以单独打包。其他包按工作区依赖顺序发布。

## 三档运行时追踪

默认策略是 debug 构建 `errors-only`，release 构建 `off`：

```rust
let trace = nichlink::CallTrace::runtime();
```

需要零开销路径时使用 `CallTrace::disabled()`；Studio 或调试会话使用 `CallTrace::full()`。`trace.with_result(...)` 会在 `errors-only` 模式下回滚成功操作，只保留错误或 panic 的证据链。

可用环境变量覆盖默认值：

```sh
NICH_LINK_TRACE=full cargo run
```

支持 `off`、`errors-only`、`full`，以及可读别名 `errors`、`errors_only`。

## MCP

```sh
NICH_LINK_PACKAGE_ROOT=/work/my-app cargo run -p nichlink-mcp
```

服务提供只读工具：`nichlink.search`、`nichlink.inspect`、`nichlink.callgraph`、`nichlink.read` 和 `nichlink.status`。调用图明确标为 `static-heuristic`；动态调用和运行时数值应以 `nichlink-debug` 的 `CallTrace` 为准。所有读取路径都限制在 `NICH_LINK_PACKAGE_ROOT` 内。

## 为什么需要 NichLink

当一个对象图同时需要三件事时，NichLink 才有价值：对象自己声明归属位置；注册合同在发布前拒绝不兼容对象；中间层可以原子替换。小型应用使用普通 Rust 模块、trait 或构造函数注入通常更简单。

## Studio 界面示意

![](./picture/NichLink_studio.png)

Studio 是可选的、事件驱动的 Ratatui 工具，只在输入、窗口变化或 watch 事件后重绘，不会进入 core 运行时。

## 同类型方案对比

| 方案 | 擅长 | 不负责 | NichLink 补充 |
| --- | --- | --- | --- |
| 模块 / trait | 命名空间和静态行为 | 发现、准入、递归拓扑 | 声明和合同层 |
| 依赖注入 | 显式构造和测试 | 注册树及中间层原子替换 | 被动注册和 graft 事务 |
| `inventory` / `linkme` | 分布式平面收集 | 合同、父链闭合、来源追踪 | 可选收集器加树语义 |
| Bevy Plugin | 显式组合应用 | 通用对象路径和 graft 校验 | 对象自声明与合同 |
| CodeGraph / CodeQL | 符号和调用证据 | 运行时注册决策 | 由 Studio/MCP 消费证据 |

## 能力边界

- 静态调用结果是证据，不保证完整解析 `dyn Trait`、函数指针、FFI 或运行时选择的调用；
- 优化后且未插桩的局部变量可能显示为 `unobserved`，需要真实数值时启用 `full` 追踪；
- 进程插件只隔离故障和资源，不等于操作系统安全沙箱；
- 依赖 crate 的 `build.rs` 不能读取宿主应用的 `main.rs`，宿主必须提供一次构建入口。

## 命名空间隔离

注册面的身份包含 `CARGO_PKG_NAME` 命名空间。不同库即使文件路径、kind 或根名称相同，也不会共享节点。新宿主建议使用：

```rust
let registry = nichlink::Registry::root_for_namespace(
    nichlink::FrameworkId::new("my-app"),
    "my-company.my-app",
);
```

Studio 可通过 `NICH_LINK_NAMESPACE` 和 `NICH_LINK_PACKAGE_ROOT` 选择命名空间及待编辑项目。

## 构建与发布检查

```sh
cargo fmt --all
cargo test --workspace --offline
cargo test --workspace --release --offline
cargo package -p nichlink-core --allow-dirty --offline
cargo package -p nichlink-build --allow-dirty --offline
cargo package -p nichlink-mcp --allow-dirty --offline
```

`tools/nichlink-release-audit` 会在 `target/nichlink-audit/release/` 输出产物大小、符号列表和节点/函数清单。`debug`、`studio`、`plugin-host` 需要先发布可解析的 `nichlink-core`，再按依赖顺序打包。

MSRV 为 Rust 1.96；CI 会同时运行这个固定版本和当前 stable，分别验证兼容下限与最新 Rust。技术路线见 `ROADMAP.md`；迁移和威胁模型属于可选补充，不影响核心 API 的快速上手。

英文文档：[`README.md`](README.md) 和 [`ROADMAP.md`](ROADMAP.md)。补充文档各自提供语言链接。
