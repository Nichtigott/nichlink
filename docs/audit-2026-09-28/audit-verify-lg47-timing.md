# 独立验证：LGC-LG-47 的并发时序（用 t73 的注入式 load 接缝）

- 验证者：kernel-auditor（t79，**非作者**；装置由 t73 造，**判定由我给**）
- 真实树**零改动**（本轮只写本文件）；我的探针与变异在 `/tmp/nk-t79`（自带 `CARGO_TARGET_DIR=/tmp/nk-t79-target`）
- 结论：`LGC-LG-47` 由 t68 的"**部分证实**"补为"**证实**"——时序这次是**测**出来的

## 1. 我自己的测量（不采信作者钉子原文）

我在副本里新写 `plugin-host/src/lazy_wasm/t79_probe_tests.rs`（`#[cfg(test)] #[path]` 挂成 `slot_state` 的兄弟测试模块；夹具前缀沿用同目录测试文件，断言与数字全是我自己的）：

- 把一次加载用 `activate_with` 的接缝**按住 400 ms**（经通道确认加载确已进入）；
- 在**加载仍在执行时**，从本线程做三项观测：

```
t79: load held 400ms; install took 42.745µs; queue lock free while loading: true; install landed: true; pending after: true
```

- **我的数字**：加载 400 ms 期间 `pending.try_lock()` **成功**（锁是空的）；同一窗口里的 `install` 只花 **42.745 µs**（≈ 加载的 1/9400），且落地；`has_pending` 在加载期间仍为 `true`。
- 断言：`install_elapsed * 4 < load_slept`（我的余量取 4 倍，实测余量约 9400 倍）；`lock_free`；`install_landed`；`pending_after`（即 t62 那条性质，我用**自己的**断言再钉一遍）。

## 2. 红侧：把接缝改回"持锁跑完整个函数体"（修前形状）

副本内把交接改回"先拿锁、锁一直持有到加载结束"（为避免同线程重锁，`publish` 改在**同一把锁**下用 `guard.is_some()` 发布，这与修前形状等价）。**我自己的数字随之翻转**：

```
t79: load held 400ms; install took 400.157971ms; queue lock free while loading: false; install landed: true; pending after: true
```

- 我的探针红：`the queue lock must be free while a load runs`；
- **install 耗时 400.157 ms = 加载耗时 400 ms**（正确实现是 42.7 µs）——同一装置、同一夹具、只差一行持锁形状，差了约 9400 倍，这就是这次"测到时序"的直接证据。
- 作者两条钉子在同一次变异下也红：`the_slot_lock_is_free_while_a_generation_compiles`（"the queue lock is free while a generation compiles…"）与 `an_install_does_not_wait_for_a_slow_load`（"the install did not wait for the 300 ms compile: 250.191644ms"）——但我给判定用的证据是**我自己那行数字**。
- 还原：`copy == pristine` 逐字节相等（Python `read_bytes`），sha256 `b67c3432ddba…`；判定只取这一轮 hash 稳定运行。

## 3. 接缝本身的性质（t73 新增装置，队长点名三项）

1. **只影响测试**：`fn activate_with(&self, load: impl FnOnce(VerifiedPluginArtifact) -> Result<WasmInstance, HostError>)` 是**私有**方法（无 `pub`），只有 `#[cfg(test)]` 挂载的同 crate 测试模块能调；`pub(super) fn activate(backend)` 现在等于 `self.activate_with(|artifact| backend.load(artifact))`，**生产路径的调用形态与行为一字未改**（交接 → 编译 → 发布 → 记录的顺序、拒绝与错误路径都保持）。测试文件被搬出生产文件（`slot_state.rs` 297 → 184 行）也属测试面。
2. **原先"不可测"那一点现在确实可测了**：`lazy_wasm` 仍是私有模块（外部集成测试依旧只能经 wasm 表 API 间接触发），但接缝把窗口暴露在**crate 自身的单元测试层**；我实际走通的路径是：在 `slot_state.rs` 里 `#[path = "t79_probe_tests.rs"]` 挂一个兄弟测试模块 → `state.activate_with(...)`（按住加载）→ 本线程 `state.pending.try_lock()` / `state.install(ValidationChannel::Local, artifact(vec![9]))` / `state.has_pending`。这条路我跑通了，且给出上面的数字。
3. **可改进处（只报告，未改源码）**：
   - 接缝闭包是 `FnOnce`，因此一条断言只能驱动一次加载；若要"多次加载序列"（例如第二次加载期间再次 `install`），需要 `FnMut` 版本或按次重建。
   - 现有钉子的"install 不被阻塞"是**通道超时**判定（500 ms）；我这次改成了**把两者耗时相比**（`install * 4 < load`），在负载高的 CI 上信噪比更好——建议把这条比较式断言也收进钉子（现在只有超时阈值）。
   - `activate_with` 没有 `#[cfg(test)]` 门控（它被 `activate` 使用，因此没有死代码警告）；它的文档应显式写明"传给它的闭包**不得重入本槽**，否则同线程重锁死锁"——我这次变异时就踩到了这个约束（用 `guard` 直接发布以规避）。

## 4. 回归面与门禁（同一批 hash）

- **回归**：t62 的 `a_generation_queued_during_a_load_stays_pending` 在整轮默认测试里仍绿；`cargo test -p xirang-plugin-host --offline` **0**（6 passed）；`--features process-tools` **0**（6 个 ok 块，含 `fault_matrix` 的 LG-32 两条）。
- `cargo test --workspace --offline` **0**（58 个 ok 块、0 FAILED）；`cargo clippy --workspace --all-targets --offline -- -D warnings` **0**；`cargo fmt --all -- --check` **0**（0 行）。
- 真实树只新增本报告；未改源码、未改他人产物、未 commit、未做 git 恢复类操作。

## 5. 未覆盖 / 新发现

1. **未覆盖**：`call`/`activate_pending` 两条并发路径我没有各自单独测（我测的是 `install` 与 `try_lock`，与审计描述的那条竞态同源）；`wasm` 特性下的真实编译时序我没有量（本机 `backend.load` 太快，接缝注入的正是这个理由）。
2. **新发现（low）**：见 §3.3 第二条——建议把"install 耗时 < 加载耗时"这一**比较式**断言收进钉子，替换/补充 500 ms 超时判定；以及 §3.3 第三条的文档补一句"闭包不得重入本槽"。
