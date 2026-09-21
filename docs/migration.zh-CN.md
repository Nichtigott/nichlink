# 迁移说明

[English](migration.md) | 简体中文

## 从旧的多功能布局迁移

注册协议位于 `nichlink-core`；构建发现、身份缓存、scope 和 `StaticPlan` 位于 `nichlink-build-method`；MIR 和数据流证据位于 `nichlink-debug-method`；TUI 位于 `nichlink-studio`；隔离插件执行位于 `nichlink-plugin-host`；统一命令行入口（`nichlink new/check/build/studio/mcp`）位于 `nichlink-cli`。

应用通过 `Registry::root_for_namespace` 提供自己的根 Registry。release 使用宿主生成的 `StaticPlan`，本 workspace 不提供固定的 `root_registry`。

## 外部声明

跨 crate 声明使用 `nichlink::external_object!`，必须填写插件来源、版本、目标框架和父节点身份。开发构建只有在宿主显式启用 `collector: debug` 时才发现这些声明；core 本身不依赖 inventory。release 应通过已验证插件 artifact 或应用自有输入保留外部注册面。

## 注册面按真实路径载入

构建期渲染以前会把每个注册面复制到 `OUT_DIR/registration_sources`，再 `include!` 那份副本，于是编译器和编辑器解析到的模块是构建产物，而不是正在编辑的文件。现在注册面以真实模块载入：

- **叶子面**（不拥有子注册机）写成 `#[path = "..."] pub mod <name>;`，公开模块路径与 `type_name` 完全不变；
- **拥有子注册机的面**必须同时充当容器，因此载入同名子模块再重导出（`mod <name>; pub use <name>::*;`）。只有这一种情况会多一段模块路径，例如 `app::control::control::Control`。

两个后果：

- 面文件仍是普通模块文件，因此可以继续以 `//!` 或任何 inner attribute 开头。`include!` 展开无法引入它们——这正是旧设计必须在副本里把 `//!` 改写成 `//` 的原因。
- 声明宏改为从 `file!()` 推导 `source`、从 `module_path!()` 推导注册机名，生成树因此不再注入 `__REGISTRATION_SOURCE` / `__REGISTRATION_MODULE_NAME`。不在 `src/` 下的声明（例如集成测试）保留 cargo 记录的路径，而不是让剥离失败。

只有 `#[path]` 对 IDE 还不够。rust-analyzer 只在 `mod` 声明位于文件或展开的顶层时才应用该属性，因此声明在生成内联模块里的注册面——根以下的每一个面——永远进不了 crate：没有补全、没有跳转、没有悬停，尽管 `cargo` 与 `rustc` 都能看到它。所以每个这样的面还会得到一份仅供 IDE 的视角：

- 真实声明由 `cfg(not(rust_analyzer))` 把关；
- 顶层再加一条 `#[cfg(rust_analyzer)] #[path = "..."] mod __nichlink_ra_<路径>;`，在 rust-analyzer 真正应用 `#[path]` 的位置载入同一个文件；
- 在注册面的真实位置加 `#[cfg(rust_analyzer)] use crate::__nichlink_ra_<路径> as <名字>;`，重建它的模块路径，可见性与真实声明一致（叶子面是 `pub`，容器面是 `pub(crate)`）。

两种工具各自只看到其中一份声明，因此 `rustc` 读到的树与改动前完全一致，用户可见行为不变。`build_method` 会输出 `cargo::rustc-check-cfg=cfg(rust_analyzer)`，让多出来的 cfg 保持安静。

身份不变：`source`、`registry_name`，以及由此而来的 `NodeId`、注册路径和全部 graft 选择器都保持原值。

用 `static_graft_plan!` 声明的 graft 计划现在无论宿主是否同时声明 `application!(entry = ...)`，都会从宿主入口被捕获——与 `SourceScope` 原有的入口解析保持一致。此前缺少 `application!` 时计划会被静默丢弃。

示例见 `examples/control-button/`（README 那棵 Control/Button 树作为真实宿主）与 `examples/control-button-graft/`（在其上做 graft 的项目外实现）。

## Studio 与插件宿主

使用 `cargo run --manifest-path studio/Cargo.toml` 启动 Studio；`1`～`4` 切换 Search、Inspect、Data、Compare。`watch` 由 `nichlink-dev` 提供，会在源码、Cargo 或插件目录变化时重建子 Studio 进程。

插件宿主只接受 `VerifiedPluginArtifact`。Wasm slot 在编译期声明；进程适配器需显式启用 `process-tools`。现有 `PluginManifest` 流程合同和 lock 记录保持兼容。
# Graft overlay 迁移

当前 graft 模型是不可变覆盖层。宿主保留原始源码，只在入口声明外部实现：

```rust
let plan = nichlink::graft_plan!(framework,
    cut ["root/canvas"] graft "canvas_fast",
    cut ["root/layout"] full graft "layout_v2",
);
let effective = base.overlay(&plan, &external)?;
```

`cut A graft X` 替换一个逻辑槽位，并继承 A 原来的子节点；`cut A full graft X`
替换 A 的整棵子树；范围 cut 则选择同一父注册机下连续的兄弟。原树和外部树都
不会被修改，只有在数据流合同、注册规范、准入和连接器全部通过后才发布有效视图。

旧的 `Registry::graft` API 和 Studio 源码复制流程已经移除。公开执行路径统一为
`GraftPlan`，再调用 `Registry::overlay`。
