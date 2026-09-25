# Performance baseline

[简体中文](#简体中文)

This file records what the released artifacts cost on the machine the baseline
was taken on, and what the checks that guard those numbers actually assert. It
exists because "the numbers are printed" is not a budget: without a recorded
value and a ceiling, a tenfold regression only makes a CI log longer.

Taken on: Linux x86-64, `rustc 1.96.0 (ac68faa20 2026-05-25)`, release profile
(`lto = "thin"`, `codegen-units = 1`), workspace at the commit that added this
file. Re-measure with the commands below; a different machine will differ, which
is why every ceiling is an order-of-magnitude guard rather than a grade.

## Registration scale

`run_method/examples/scale_audit.rs`, run through
`tools/nichlink-scale-audit` (which adds `peak_rss_kb` when GNU `time` is
available):

```sh
cargo run --release -p nichlink-run-method --example scale_audit -- 10000 100000
```

| Nodes | Register (ms) | Index (ms) | Entries | Pages | Static-face bytes |
| --- | --- | --- | --- | --- | --- |
| 10 000 | 35 | 15 | 10 001 | 32 | 330 000 |
| 100 000 | 522 | 259 | 100 001 | 32 | 3 300 000 |

That is 5.2 µs per node registered and 2.6 µs per node indexed at 100 000 nodes,
and the page count stays flat, which is what the incremental transaction is for.

The audit now asserts a ceiling as well as the entry count: 40 µs per node for
registration and 20 µs for indexing, roughly eight times the measured values, so
an order-of-magnitude regression fails the run instead of printing a larger
number. A genuinely slower machine can raise them without editing the file:

```sh
NICHLINK_SCALE_REGISTER_US=120 NICHLINK_SCALE_INDEX_US=60 tools/nichlink-scale-audit
```

If a legitimate change moves the baseline, update the table above and the
ceilings together: a ceiling that is never re-derived slowly becomes a lie about
what the code costs.

## Release artifacts

`tools/nichlink-release-audit` builds the workspace in release mode and then
walks `target/release`, recording bytes and defined symbols per artifact into
`target/nichlink-audit/release/artifacts.tsv`. It also **fails** when any linked
artifact still carries an `.inventory` linker section, which is the release
promise that the shipping binary holds a static plan rather than a registration
inventory.

Last run: exit 0, no artifact carried the section, and the shipped binaries were

| Artifact | Bytes | Defined symbols |
| --- | --- | --- |
| `nichlink` | 4 782 240 | 6 140 |
| `cargo-nichlink` | 4 802 024 | 6 186 |
| `libnichlink_macro.so` | 1 184 160 | 5 085 |

Startup, measured by the same script on the same machine: `startup_ms=176`. The
script prints it rather than asserting it, because process start on a loaded CI
runner says more about the runner than about the binary; the assertion that
matters there is the linker-section one, which is machine-independent.

## Plugin host: what a Wasm table costs

`WasmLimits::table_elements` bounds the number of function references a module
may put in a table. That number is worth recording because a table is a separate,
eagerly instantiated array: the linear-memory ceiling does not bound it at all,
and wasmi's own `EnforcedLimits::strict()` — which the loader also applies — caps
how many tables and element segments a module may declare, not how large one
table may grow. Measured with a counting global allocator, in
`plugin-host/tests/wasm_table_cost.rs`:

```sh
cargo test --release -p nichlink-plugin-host --test wasm_table_cost -- --nocapture
```

| Declaration | Peak bytes allocated | Note |
| --- | --- | --- |
| `(table 1048576 funcref)` | 8 469 408 | 8 bytes per element |
| `(table 100000000 funcref)` under the default ceiling | 5 225 | refused before the table exists |

So the default ceiling of 4096 elements costs 32 KiB, and the hundred-million-entry
module that `a_huge_table_is_refused` pins would have cost 762 MiB. Debug and
release builds give the same per-element figure, because it is a pointer-sized
slot rather than something the optimiser can shrink. The test keeps the count in
one file with one test case on purpose: the allocator counter is process-global,
so a sibling test allocating on another thread would be counted as table cost.

## Release read path: allocations

`README.md`'s cost table claims the read-only built-in topology has "No startup
allocation", that reading a build-declared static graft "allocates nothing", and
that `overlay_static` "allocates no plan". Measured with the counting global
allocator in `examples/control-button/tests/static_plan_allocations.rs`, which
holds exactly one test because the counter is process-global:

```sh
cargo test -p nichlink-example-control-button --test static_plan_allocations -- --nocapture
```

| Phase | Allocations | Bytes |
| --- | --- | --- |
| `builtin_static_plan()` plus `faces`/`grafts`/`len`/`is_empty`, `find` hit and miss, `children_of`, and a walk over every face | 0 | 0 |
| `overlay_static` with no cuts — the floor: it clones the base tree and builds its bookkeeping | 6 | 96 |
| `overlay_static`, one cut | 60 | 3 470 |
| `overlay`, one cut, with the plan built in the same window | 77 | 4 053 |
| `overlay_static`, the example's two cuts | 107 | 5 628 |
| `overlay`, the same two cuts written textually | 138 | 6 438 |
| the one-cut `GraftPlan` on its own | 3 | 350 |

So the static plan is read for free, and the static overlay is not free but is
cheaper than the dynamic spelling of the same work, by about a fifth. The floor
is why that row of the cost table says "allocates no plan" rather than "allocates
nothing": the clone and the visited-cut set are real, and
`docs/audit-graft-vs-readme.md`'s C17 already recorded that reading. Comparing the
example's two static cuts against a one-cut dynamic plan would show the opposite,
which is why the phases separate the floor from the per-cut cost instead of
comparing two totals.

## 简体中文

本文件记录已发布产物在采集基线的那台机器上的开销，以及守护这些数字的检查究竟断言了什么。
它存在的原因是："数字被打印出来"不等于预算：没有记录值与上限时，十倍的性能退化只会让 CI
日志更长。

采集环境：Linux x86-64、`rustc 1.96.0 (ac68faa20 2026-05-25)`、release profile
（`lto = "thin"`、`codegen-units = 1`），工作区版本为加入本文件的那个提交。用下面的命令重新测量；
换一台机器数字会不同，这正是每条上限都是数量级守卫而不是打分的原因。

### 注册规模

`run_method/examples/scale_audit.rs`，经 `tools/nichlink-scale-audit` 运行（安装了 GNU `time`
时它还会给出 `peak_rss_kb`）：

```sh
cargo run --release -p nichlink-run-method --example scale_audit -- 10000 100000
```

| 节点数 | 注册 (ms) | 索引 (ms) | 条目 | 页 | 静态面字节 |
| --- | --- | --- | --- | --- | --- |
| 10 000 | 35 | 15 | 10 001 | 32 | 330 000 |
| 100 000 | 522 | 259 | 100 001 | 32 | 3 300 000 |

即 100 000 节点时每节点注册 5.2 µs、索引 2.6 µs，而页数保持不变——这正是增量事务的用途。

规模审计现在除了条目数还断言上限：注册每节点 40 µs、索引每节点 20 µs，约为实测值的八倍，
因此数量级的退化会让这次运行失败，而不是打印一个更大的数字。确实更慢的机器可以不改文件而抬高它们：

```sh
NICHLINK_SCALE_REGISTER_US=120 NICHLINK_SCALE_INDEX_US=60 tools/nichlink-scale-audit
```

如果某项正当改动移动了基线，请把上表与上限一起更新：一条永不重新推导的上限会慢慢变成关于
"代码要花多少代价"的谎言。

### 发布产物

`tools/nichlink-release-audit` 以 release 模式构建整个工作区，然后遍历 `target/release`，
把每个产物的字节数与已定义符号数记入 `target/nichlink-audit/release/artifacts.tsv`。当任何已链接
产物仍带着 `.inventory` 链接段时它还会**失败**，而这正是"发布产物持有静态计划、而不是注册
清单"这条承诺。

最近一次：退出 0，没有产物带该段，出厂二进制为

| 产物 | 字节 | 已定义符号 |
| --- | --- | --- |
| `nichlink` | 4 782 240 | 6 140 |
| `cargo-nichlink` | 4 802 024 | 6 186 |
| `libnichlink_macro.so` | 1 184 160 | 5 085 |

同一脚本在同一台机器上测得启动：`startup_ms=176`。脚本只打印它而不断言它，因为负载高的 CI
runner 上进程启动更多说明的是 runner 而不是二进制；那里真正要紧的断言是链接段那条，它与机器无关。

### 插件宿主：一张 Wasm 表要花多少

`WasmLimits::table_elements` 约束一个模块能往表里放多少个函数引用。这个数字值得记录，因为表
是一块独立的、即时实例化的数组：线性内存上限完全约束不到它，而 wasmi 自带的
`EnforcedLimits::strict()`（加载器也会施加）限制的是一个模块可以声明多少张表与多少个元素段，
而不是单张表能长到多大。用计数式全局分配器实测于
`plugin-host/tests/wasm_table_cost.rs`：

```sh
cargo test --release -p nichlink-plugin-host --test wasm_table_cost -- --nocapture
```

| 声明 | 分配峰值字节 | 说明 |
| --- | --- | --- |
| `(table 1048576 funcref)` | 8 469 408 | 每元素 8 字节 |
| 默认上限下的 `(table 100000000 funcref)` | 5 225 | 在表存在之前就被拒绝 |

因此默认上限 4096 个元素是 32 KiB，而 `a_huge_table_is_refused` 钉住的那个一亿条目模块本来
会是 762 MiB。debug 与 release 给出同一个每元素数字，因为它是一个指针大小的槽位，而不是
优化器能压缩掉的东西。该测试刻意把一个文件的测试数保持为一条：分配器计数器是进程全局的，
兄弟测试在另一线程上的分配会被算成表的开销。

### 发布读路径的分配

`README.md` 的成本表声称只读内置拓扑"启动时零分配"、读构建期声明的静态 graft"不分配"、
`overlay_static`"不分配计划"。用 `examples/control-button/tests/static_plan_allocations.rs` 里的
计数式全局分配器实测（该文件恰好只有一条测试，因为计数器是进程全局的）：

```sh
cargo test -p nichlink-example-control-button --test static_plan_allocations -- --nocapture
```

| 阶段 | 分配次数 | 字节 |
| --- | --- | --- |
| `builtin_static_plan()` 加 `faces`/`grafts`/`len`/`is_empty`、`find` 命中与落空、`children_of`，以及遍历全部注册面 | 0 | 0 |
| 不带切口的 `overlay_static`——下限：它会克隆基树并建立簿记 | 6 | 96 |
| `overlay_static`，一个切口 | 60 | 3 470 |
| `overlay`，一个切口，计划在同一窗口内构造 | 77 | 4 053 |
| `overlay_static`，示例的两个切口 | 107 | 5 628 |
| `overlay`，同样两个切口用文本形式书写 | 138 | 6 438 |
| 仅那份一切口 `GraftPlan` | 3 | 350 |

因此静态计划是免费读的；静态 overlay 不免费，但在同样的工作上比动态写法便宜约五分之一。
那个下限正是成本表那一行写"不分配计划"而不是"什么都不分配"的原因：克隆与已访问切口集合是真实的，
`docs/audit-graft-vs-readme.md` 的 C17 已经记下这个读法。若拿示例的两个静态切口去对一个切口的
动态计划，就会得到相反的结论——这正是上面把下限与按切口开销分开、而不是比较两个总数 的原因。
