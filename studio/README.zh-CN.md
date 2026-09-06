# nichlink-studio

[English](README.md) | 简体中文

`nichlink-studio` 是独立的 Ratatui 注册面编辑和诊断工具。

```sh
cargo run --manifest-path studio/Cargo.toml
```

设置 `NICH_LINK_PACKAGE_ROOT` 指向要读取或编辑的项目目录；设置 `NICH_LINK_HOST_MANIFEST` 可指定用于 MIR 检查和 release 构建的 Cargo manifest；设置 `NICH_LINK_NAMESPACE` 可隔离多个库的 authored snapshot。Studio 是开发工具，不会被带入 core 的发布运行时。
