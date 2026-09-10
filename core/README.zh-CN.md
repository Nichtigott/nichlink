# nichlink-core

English | [简体中文](README.zh-CN.md)

NichLink 的 kernel：协议名词 + 纯方法——无 I/O、无环境绑定。这里的一切
都可内存中测试，可被任意执行面复用。

`registry_core` 下的模块：

| 模块 | 内容 |
|------|------|
| `identity` | `NodeId` 编译期 SHA-256 身份、`StableFaceId`、零依赖 SHA-256、hex 编解码 |
| `declaration` | `RegistrationInfo`/`RegistrationSnapshot`、注册规则、准入、流合同、插件 manifest、运行期检查、调用点与证据类型（`CallSite`、`CallEdge`、`EvidenceKind`、`TraceMode`） |
| `tree` | 唯一的 `Registry` 类型：同类型递归注册、页拷贝原子事务、查询、graft 覆盖/替换校验、检视 |
| `plugin` | 插件策略判定、信任/验签、artifact/catalog、graft 命令解析、槽位校验 |
| `mir` | 静态 MIR 候选解析（文本与 JSONL）与带证据归并的 `merge_call_relations`（live 边优先于 MIR 候选） |
| `requirements` | 声明图上的能力需求分析（祖先链提供者查找） |
| `source` | Rust 源码迷你词法器：函数索引、直接调用提取、注册 kind——工具面共用 |
| `release` | 零分配发布拓扑（`StaticPlan`）与编译期注册断言 |
| `diagnostic` | `RegistryError` 树、`BuildDiagnostic` 模型、注册面拓扑校验 |
| `syntax`（feature `syntax`） | 注册面解析器、应用/graft 入口发现 |
| `authoring`（feature `syntax`） | 纯 authoring 计划/渲染 helper、命名校验、`FACE` 字段词典 |

分界规则：无 I/O、无 `std::env`/时间/进程绑定的代码属于这里；执行面
（`nichlink-build-method`、`nichlink-run-method`、`nichlink-debug-method`、
`nichlink-plugin-host`、studio、mcp、cli）把这些方法绑定到各自的上下文。
