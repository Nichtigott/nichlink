# nichlink-studio

[English](README.md) | 简体中文

`nichlink-studio` 是独立的 Ratatui 注册面编辑和诊断工具。终端用户通常通过
统一 CLI 使用它（`nichlink-cli` 提供的 `nichlink studio`）；本 crate 同时
保留 `nichlink-studio` 二进制，以及一个仅在非默认 `dev-supervisor` 特性下才构建的
`nichlink-dev` 重建监督器（它从检出重建，安装副本永远做不到，因此 `cargo install`
不会带上它）。

```sh
cargo run --manifest-path studio/Cargo.toml
# 或者直接点名项目：
nichlink-studio /path/to/project
```

`--help` 列出该参数。没有参数时依次使用 `NICH_LINK_PACKAGE_ROOT`、持有 `Cargo.toml` 的当前
目录。**有意没有别的兜底**：没有项目的启动会带着消息以非零退出，而不是在别的目录上打开一棵
空树。设置 `NICH_LINK_PACKAGE_ROOT` 指向要读取或编辑的项目目录；设置 `NICH_LINK_HOST_MANIFEST` 可指定用于 MIR 检查和 release 构建的 Cargo manifest；设置 `NICH_LINK_NAMESPACE` 可隔离多个库的 authored snapshot。Studio 是开发工具，不会被带入 core 的发布运行时。

`g` 为选中注册面打开外部 graft 界面。它只写
`.nichlink/external-grafts/<selector>/graft.plan`，绝不写宿主源码，并且把 graft 的三个
层次分开：构建读取的 `static_graft_plan!` **声明**（界面会显示当前槽位是否已声明，并给出
可直接粘贴的子句）、宿主在运行期执行的 `Registry::overlay` **应用**，以及界面写下并管理的
计划**记录**——列出、打开、用 `f` 改范围、用 `d` 移入
`.nichlink/trash/external-grafts/`。为没有声明命名的槽位写记录是允许的（事后补声明是合法
顺序），但状态栏会警告：发布态会剪掉该槽位，运行期会把这条记录当作 `UnkeptSlot` 跳过，
并重申要补的子句。
