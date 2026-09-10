# nichlink-debug-method

[English](README.md) | 简体中文

`nichlink-debug-method` 是观测证据面。静态分析本体——MIR 文本/JSONL 解析、
源码词法、证据归并（live 边优先于 MIR 候选）——在 kernel
（`nichlink-core` 的 `mir` 与 `source` 模块）。本 crate 把它绑定到工具链
与进程：

- `collector`：linker-section inventory 收集，面向通过生成的父宏显式
  加入的声明，例如 `crate::workspace_object!(collector: debug, ...)`。
  kernel 不依赖 inventory 与 linker-section。
- `mir::UnifiedCallGraph`：把静态 MIR 候选与一次 live `CallTrace` 运行
  归并。
- `adapters`：`tracing` span 创建与基于 petgraph 的 `CallGraph` 拓扑视图
  （DOT 导出）。

需要收集能力的宿主引入本 crate，并用
`nichlink_debug_method::registrations!(nichlink::RegistrationInfo)` 读取条目。
