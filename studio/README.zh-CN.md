# nichlink-studio

[English](README.md) | 简体中文

`nichlink-studio` 是独立的 Ratatui 注册面编辑和诊断工具。终端用户通常通过
统一 CLI 使用它（`nichlink-cli` 提供的 `nichlink studio`）；本 crate 同时
保留 `nichlink-studio` 二进制和 `nichlink-dev` 重建监督器。

```sh
cargo run --manifest-path studio/Cargo.toml
```

设置 `NICH_LINK_PACKAGE_ROOT` 指向要读取或编辑的项目目录；设置 `NICH_LINK_HOST_MANIFEST` 可指定用于 MIR 检查和 release 构建的 Cargo manifest；设置 `NICH_LINK_NAMESPACE` 可隔离多个库的 authored snapshot。Studio 是开发工具，不会被带入 core 的发布运行时。

`g` 为选中注册面打开外部 graft 界面。它只写
`.nichlink/external-grafts/<selector>/graft.plan`，绝不写宿主源码，并且把 graft 的三个
层次分开：构建读取的 `static_graft_plan!` **声明**（界面会显示当前槽位是否已声明，并给出
可直接粘贴的子句）、宿主在运行期执行的 `Registry::overlay` **应用**，以及界面写下并管理的
计划**记录**——列出、打开、用 `f` 改范围、用 `d` 移入
`.nichlink/trash/external-grafts/`。
