# nichlink-debug

[English](README.md) | 简体中文

`nichlink-debug` 是可选的开发证据层，提供 MIR/源码证据、实时 `CallTrace`、数据流和图模型，以及显式 opt-in 的 inventory 收集器：

```rust
使用生成的父级宏并选择 debug collector，例如：
`crate::workspace_object!(collector: debug, ...)`。
```

core 不依赖 inventory 或 linker section。宿主需要收集声明时，再通过 `nichlink_debug::registrations!(nichlink::RegistrationInfo)` 读取条目。静态候选会标注证据等级，不能替代动态运行时事实。
