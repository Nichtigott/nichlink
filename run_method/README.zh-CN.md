# nichlink-run-method

English | [简体中文](README.zh-CN.md)

`nichlink-run-method` 是 NichLink 的 run_method 执行面：运行期状态实例 +
trace 绑定。协议本体——`Registry` 树、事务、graft 校验、插件策略与解析器
——都在 kernel（`nichlink-core`）；本 crate 把它们绑定到进程生命周期。

留在这里的部分：

- `CallTrace` 帧栈、局部值与数据边（`trace_mode_from_env` 读取
  `NICH_LINK_TRACE`；枚举与解析器是 kernel 类型）
- 声明宏：`host!`、`application!`、`static_graft_plan!`、`graft_plan!`、
  `trace_call!` 以及生成的 `*_object!` 族
- 不受门控的 graft 记录加载器与覆盖入口：`apply_recorded_grafts`、
  `load_graft_records`、`load_graft_record`、`graft_record_root`、
  `LoadedGraft`、`GraftOverlay`，以及重导出的
  `RecordReport`/`RecordedGraft`/`ResolvedRecord`。它刻意放在 `authoring`
  之外，宿主读取 `.nichlink/external-grafts/` 无需引入 `syn`；优先级策略与
  报告见 [`docs/graft.md`](https://github.com/Nichtigott/nichlink/blob/main/docs/graft.md)。`apply_recorded_grafts` 在返回前为
  每条报告向 stderr 打印一行 `warning:`/`note:`，因此被跳过的记录（一次悄无声息从未
  发生的嫁接）不可能被忽略；同样的条目仍留在 `GraftOverlay` 中，供把证据转往自己
  日志的宿主使用。解析不了的计划、目录与计划里的 `graft` 不一致、以及身份与路径矛盾的
  记录则是 `Err`：它们都没有合法解读。
- authoring **执行器**（feature `authoring`）：把文件计划应用到宿主的
  `src/` 树。纯计划/渲染逻辑与 `FACE` 字段词典在 kernel 的 `authoring`
  模块
- 基于 live trace 的 `call_report` 渲染

宿主 crate 依赖本 crate 并在 crate 根部调用一次
`nichlink_run_method::host!();`；构建期另一半是 `nichlink-build-method`。

## 运行期校验

面上的 `runtime_checks: [...]` 是宿主 API，而不是自动钩子：内核从不观测取值，
边界由宿主拥有。请在取值跨入插件或消费者的位置，构造 `RuntimeValue` 并用该面的
`NodeId` 调用 `Registry::health_check`。没有活动 trace 时 `call_path` 传
`Vec::new()`（有 trace 时传 `CallTrace::current_path()`）。失败时错误按每条失败的
检查聚合子错误。五个读取器暴露构造函数收到的事实
（`core/src/registry_core/diagnostic/error.rs`）：

| 读取器 | 返回 | 取值情况 |
| --- | --- | --- |
| `node()` | 该失败所针对的注册面 `NodeId` | 总是有 |
| `path()` | 报告该失败时所在的逻辑注册路径 | 总是有；没有活路径的 phase 写入占位串 |
| `source()` | `&DiagnosticSource`，该失败指回的声明源码位置 | 总是有；没有声明可指的 phase 合成一个 `<…>` 位置（line 0、column 0） |
| `message()` | 人类可读的失败消息 | 总是有 |
| `children()` | 聚合的子失败，每条失败的子检查一条 | 仅当该 phase 聚合了子失败时非空 |

占位路径有 `<unknown:{node}>`
（`core/src/registry_core/tree/inspection/inspection.rs:74`）、
`<missing-parent:…>/…`
（`core/src/registry_core/tree/transaction/transaction.rs:167`、
`core/src/registry_core/tree/graft_ops/graft_ops.rs:146`）与 `<edited>/…`
（`core/src/registry_core/tree/graft_ops/graft_ops.rs:186`）；合成的来源位置有
`<runtime>`（`inspection.rs:75`）、`<registry-connector>`
（`core/src/registry_core/tree/connector/connector.rs:205`）、
`<owned-snapshot-batch>`
（`core/src/registry_core/tree/transaction/transaction.rs:83`）、`<migration>`
（`core/src/registry_core/tree/graft_ops/graft_ops.rs:54`）与 `<graft>`
（`core/src/registry_core/tree/graft_ops/graft_ops.rs:294`、
`core/src/registry_core/tree/graft_ops/reconcile.rs:166`）。

读取 `health_check` 失败的两条规则：

1. 顶层 `message()` 是固定的聚合句 `runtime health check failed`
   （`core/src/registry_core/tree/inspection/inspection.rs:110`），不包含失败检查的名字。
   其他聚合 phase 形状相同：`registration connector rejected (N face(s))`
   （`core/src/registry_core/tree/connector/connector.rs:211`）与 `snapshot batch
   rejected (N error(s))`
   （`core/src/registry_core/tree/transaction/transaction.rs:89`）。
2. 失败检查自己的名字与文本在 `children()[0].message()` 中，按
   ``check `<名字>`: <消息>`` 构造（`inspection.rs:95`）。`Display` 渲染聚合句加每条
   子错误，因此下例的 `eprintln!("{error}")` 会同时打印两者。

```rust
use nichlink_run_method::{Provenance, Registry, RuntimeValue};

fn validate(registry: &Registry, node: nichlink_run_method::NodeId, label: &str) {
    let value = RuntimeValue::text(
        label,
        Provenance::default().push(node, "Button", "paint", label),
    );
    if let Err(error) = registry.health_check(node, &value, Vec::new()) {
        eprintln!("{error}"); // 每条失败的检查一条子错误
    }
}
```

可运行的版本在
[`examples/control-button/examples/health_check.rs`](https://github.com/Nichtigott/nichlink/blob/main/examples/control-button/examples/health_check.rs)：
`cargo run -p nichlink-example-control-button --example health_check`。

