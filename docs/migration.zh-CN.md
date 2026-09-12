# 迁移说明

[English](migration.md) | 简体中文

## 从旧的多功能布局迁移

注册协议位于 `nichlink-core`；构建发现、身份缓存、scope 和 `StaticPlan` 位于 `nichlink-build-method`；MIR 和数据流证据位于 `nichlink-debug-method`；TUI 位于 `nichlink-studio`；隔离插件执行位于 `nichlink-plugin-host`；统一命令行入口（`nichlink new/check/build/studio/mcp`）位于 `nichlink-cli`。

应用通过 `Registry::root_for_namespace` 提供自己的根 Registry。release 使用宿主生成的 `StaticPlan`，本 workspace 不提供固定的 `root_registry`。

## 外部声明

跨 crate 声明改用 `#[nichlink::object(source = "...", parent = ...)]`，
由 collector 自动链接，必须填写插件来源、版本、目标框架和父节点身份。开发构建
只有在宿主显式启用 `collector = debug` 时才发现这些声明；core 本身不依赖
inventory。release 应通过已验证插件 artifact 或应用自有输入保留外部注册面。

## 从声明宏 DSL 迁移

注册面现在是普通结构体。旧的
`crate::<parent>_object! { kind: Button, preset: ..., parts: ..., parent: ... }`
写法改为：

```rust
#[nichlink::object(parent = crate::control::NODE_ID)]
pub struct Button {
    preset: ActionParts,
    parts: ButtonParts,
}
```

结构体名取代 `kind`；属性参数使用 `name = value` 语法；
`name`/`summary`/`requires`/`provides` 使用组语法；`exports` 移到固有常量里：

```rust
impl Button {
    pub const EXPORTS: &'static [&'static str] = &["control.render"];
}
```

位于文件夹面之下的注册面可以省略 `parent`，构建阶段会按目录树推导挂载位置。
`nichlink::external_object!` 已删除，改为给属性传 `source = "..."`。

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
