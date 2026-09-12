# nichlink-run-method

English | [简体中文](README.zh-CN.md)

`nichlink-run-method` 是 NichLink 的 run_method 执行面：运行期状态实例 +
trace 绑定。协议本体——`Registry` 树、事务、graft 校验、插件策略与解析器
——都在 kernel（`nichlink-core`）；本 crate 把它们绑定到进程生命周期。

留在这里的部分：

- `CallTrace` 帧栈、局部值与数据边（`TraceMode::from_env` 读取
  `NICH_LINK_TRACE`；枚举与解析器是 kernel 类型）
- 声明宏：`host!`、`application!`、`static_graft_plan!`、`graft_plan!`、
  `trace_call!`，以及重导出的 `object` 属性宏（`#[nichlink::object]`）
- authoring **执行器**（feature `authoring`）：把文件计划应用到宿主的
  `src/` 树。纯计划/渲染逻辑与 `FACE` 字段词典在 kernel 的 `authoring`
  模块
- 基于 live trace 的 `call_report` 渲染

宿主 crate 依赖本 crate 并在 crate 根部调用一次
`nichlink_run_method::host!();`；构建期另一半是 `nichlink-build-method`。
