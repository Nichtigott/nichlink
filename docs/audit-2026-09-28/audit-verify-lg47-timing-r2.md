# 复核：加固后的 LG-47 装置（比例判据 + 顺序信号）—— r2

- 验证者：kernel-auditor（t86，**非作者**；装置由 t73 造、t83 加固）。承接我 t79 那份报告（`audit-verify-lg47-timing.md`）
- 真实树**零改动**（本轮只写本文件）；我的运行与变异在 `/tmp/nk-t86`（自带 `CARGO_TARGET_DIR=/tmp/nk-t86-target`）
- 结论：**加固没有动摇 t79 的"证实"；两处判据都换了形态，主张与断言对象一字未变，且顺序信号把它钉得更紧**（不是放宽）

## 1. 我自己的一次运行（加固后的装置）

```
$ cargo test -p xirang-plugin-host --offline --lib slot_state_tests -- --nocapture   # 在 /tmp/nk-t86
PROBE install=63.439µs (the load was still held)
PROBE install=71.592µs load=400.072092ms
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.40s
```

与 t79 的测量对照：t79 是 `install 42.745µs` / `load 400 ms`，这里是 `63.439µs`（加载仍被按住那一刻）与 `71.592µs` / `400.072ms` —— **同一量级**（十几到几十微秒 vs 400 毫秒，相差约四个数量级），**同一结论**：加载期间槽锁可用、`install` 不等加载。

## 2. 我自己复现的红侧（修前形状：持锁跑完整个函数体）

副本内把交接改回"先拿锁、持有到加载结束"（`publish` 改在同一把锁下用 `guard.is_some()`，以免同线程重锁）：

```
PROBE install=400.134451ms load=400.069172ms
test lazy_wasm::slot_state::slot_state_tests::an_install_does_not_wait_for_a_slow_load ... FAILED
thread '…an_install_does_not_wait_for_a_slow_load' panicked at plugin-host/src/lazy_wasm/slot_state_…:
  the install must cost a fraction of the load it overlaps: install=400.134451ms load=400.069172ms
test lazy_wasm::slot_state::slot_state_tests::the_slot_lock_is_free_while_a_generation_compiles ... FAILED
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 3 filtered out; finished in 2.00s
```

- **比例断言红**（原文如上）：`install=400.134451ms` 与 `load=400.069172ms` 等长（未加固那一轮同样形状下我量到 400.157971ms；正确实现是 63–72µs），即"install 必须只花加载的一小部分"这条判据确实抓得住旧行为。
- **是失败而不是吊死**：整轮**正常结束**并有 `test result: FAILED`（2.00 s，来自顺序信号的 `recv_timeout(2 s)` 超时——加载从未被信号释放，这正是信号该报的事）。没有死锁挂起。
- 还原后 `copy == pristine` 逐字节相等，sha256 `5ba7b5d9ae4b…`（判定只取这一轮 hash 稳定运行）。

## 3. "加固没有改变主张"——逐项核对

| 意图（t79） | 断言对象（t79） | 现在（t83 之后） | 判定 |
| --- | --- | --- | --- |
| 加载期间槽锁可用 | `pending.try_lock().is_ok()` | 同一断言（`the_slot_lock_is_free_while_a_generation_compiles`）+ **新增顺序信号** `released_by_signal`：安装必须在加载被释放**之前**完成 | **同一对象，更强**：超时只能证明"最终没等"，顺序信号证明"先于释放发生" |
| `install` 不等加载 | 500 ms 通道内回报（阈值） | 比例判据 `install * 4 < load`（我 t79 报告里建议的那条）+ `landed` + `PROBE` 行 | **同一对象，判据从阈值换成比例**（自校准，不依赖机器快慢） |
| 加载期间排队的代际仍待定 | `has_pending` | 同一断言（`a_generation_queued_during_a_load_stays_pending`，t62 那条） | 未变 |

- **没有放宽**：唯一"变松"的读数方向是 500 ms 通道超时被 2 s 的*信号*超时替代——但那条超时现在只用来判定"信号没来"（一个**辅助**观测），真正的判据换成了比例式与顺序式，两者都比阈值更直接。
- **有收紧**：新增的顺序信号（`released_by_signal`）是 t79 时**不存在**的证据维度；它把"install 落在加载期间"从"耗时比较"提升为"时序先后"。
- 文档也按我 t79 的建议补了"闭包不得重入本槽，否则同线程重锁死锁"——我这次变异正是踩在这条约束上（用 `guard` 直接发布规避）。

## 4. 判定

**加固的这两条钉子仍足以支撑 `LGC-LG-47` 的"证实"**，且比 t79 那一轮更强：我用自己的运行量到同一量级（63–72µs vs 400 ms），用同一处修前形状复现出比例断言的红，且红是"失败"而不是"吊死"。t79 的判定**不被动摇**——装置换形态、判据更直接，主张与断言对象没有变化。

## 5. 门禁（同一批 hash）

`cargo test -p xirang-plugin-host --offline` **0**（6 passed）／`--features process-tools` **0**（6 个 ok 块）／`cargo clippy --workspace --all-targets --offline -- -D warnings` **0**／`cargo fmt --all -- --check` **0**（0 行）。
`cargo test --workspace --offline` **101**，**唯一失败＝ ENV-1 那条已知离线用例** `studio::app::tests::new_project::new_project_and_explicit_root_face_compile`（该块 107 passed / 1 failed），即任务书豁免的那条；plugin-host 侧不受影响。

## 6. 未覆盖

- 我只做了"持锁跑完整个函数体"这一处变异；没有对新的顺序信号本身做反向变异（例如让 `install` 在释放后才跑却仍通过）——理论上比例判据会抓住它，但我没实测。
- 我仍没有量 `wasm` 特性下的真实编译时序（本机太快，接缝注入正是为此而设）。
