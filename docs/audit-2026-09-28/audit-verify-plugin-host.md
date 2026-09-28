# 独立验证：plugin-host 两轮修复（t44 的 LGC-LG-32 + t62 的 LGC-LG-47）

- 验证者：kernel-auditor（t68，**非作者**：t44/t62 均未参与）
- 真实树**零改动**（本轮只写本文件）；夹具/变异在 `/tmp/nk-t68`（自带 `CARGO_TARGET_DIR=/tmp/nk-t68-target`）
- 判定：**LGC-LG-32 证实；LGC-LG-47 部分证实**（行为面证实，并发时序另见 §3）

## 1. LGC-LG-32（完整帧后不退出必须交付答案）—— 证实

**我的装置与测量**（不采信作者钉子的绿，用我自己的观测）：
- **计时**：在真实树上单独跑作者的钉子 `fault_matrix::an_answer_from_a_child_that_never_exits_is_delivered`（子进程写完 6 字节帧后 `exec sleep 30`，deadline 600 ms）→ 测试体内耗时 **0.06 s**、整条 cargo 仅 15 s（编译为主）。答案在**帧到达时**交付，而不是等到 deadline。
- **`/proc` 自查（队长点名①）**：跑前/跑后各扫一次 `ps -eo pid,cmd | grep "[s]leep 30"` → **0 / 0**，没有留下任何子进程。也就是说 `kill_and_reap` 真的杀掉并回收了那个会睡 30 秒的子进程。
- **过修方向**：`a_child_that_never_writes_a_frame_still_times_out`（`sleep 30`、250 ms 上限、从不写帧）在真实树上 `ok` —— Timeout 仍然当且仅当到期前没有完整帧。

**变异反证（副本内，还原后与 pristine/真实树三方 byte-identical）**
| 变异 | 红侧原文 |
| --- | --- |
| M1：删掉「帧到达即结束调用」分支（旧行为） | `an_answer_from_a_child_that_never_exits_is_delivered ... FAILED`；`a delivered answer must not decay into Timeout, got Timeout` |
| M3：到期分支改成交付手头的帧（过修） | `a_child_that_never_writes_a_frame_still_times_out ... FAILED`；`no frame by the deadline is a Timeout, got Ok([])` |

两条钉子在两组变异下**各自独立**（M1 只红交付那半，M3 只红超时那半），与 t44 自述的"两条钉子独立"一致。

## 2. LGC-LG-47（`backend.load` 持锁 + 失败路径盲目清零）—— 部分证实

**我自己的结构核对**（读码 + 源码定位，不采信自述）：`activate()` 的顺序是
`has_pending` 检查 → `take_pending()`（函数内取锁、**返回即释放**）→ `backend.load(...)` → `publish_pending()`（在与 `install` 写队列同一把锁下、由**队列自身状态**发布 `has_pending`）。
`grep -n "has_pending.store"` 只剩三处：`install`（true）、`publish_pending`（`pending.is_some()`），失败路径调用的也是 `publish_pending()` 而**不是**盲目清零——失败时的"重试＝重新 install"写进了 `activation_error` 的字段文档。

**变异反证**：M2 把 `publish_pending` 的发布改回 `has_pending.store(false, …)` → 作者钉子 `lazy_wasm::slot_state::pending_generation_tests::a_generation_queued_during_a_load_stays_pending ... FAILED`，原文 `a generation queued during the load is still pending` ✓（加载期间排队的代际仍待定）。

**未独立复现的一半（队长点名②）**：我没有建成"一边 `call` 一边 `install` 看是否仍串行"的**并发时序探针**——`lazy_wasm` 是私有模块（`plugin-host/src/lib.rs:15` 的 `mod lazy_wasm;`），外部只能经 wasm 表 API 触发激活，而一次 `backend.load`（编译+实例化+`health_check`）在本机快到来不足以稳定测出串行化差异。因此"锁确实在 `backend.load` 之前释放"这一条我只有**结构证据 + M2 红侧 + 作者的排队钉子**，没有独立的时序测量。建议后续给 `slot_state` 加一条**注入式**探针（把 `backend.load` 的入口换成受控闭包，在其中断言锁可用），那才是能测这一条的装置。

## 3. 装置与边界

- 副本 `/tmp/nk-t68`（rsync 排除 target/.git），所有变异在自己的 `target` 目录里跑；三条变异都是 `path.write_bytes()` 还原后与 pristine 及**真实树当前内容**逐字节比较（全 True），真实树 sha256 在本轮开场与结束一致（`process.rs 368de072…`、`slot_state.rs d768af9b…`）。
- 我没改任何源码；没用 `git checkout/restore/stash`。

## 4. 门禁（同一批 hash）

`cargo test -p nichlink-plugin-host --offline` **0**（4 passed）；`--features process-tools` **0**（含 fault_matrix 两条钉子）；`cargo test --workspace --offline` **0**（58 个 ok 块、0 FAILED，**未触发那条已知离线环境用例**）；`cargo clippy --workspace --all-targets --offline -- -D warnings` **0**（首跑红在并发在飞的 `studio`：「lifetime may not live long enough」，落定后复跑即绿）；`cargo fmt --all -- --check` **0**（0 行）。

## 5. 未覆盖 / 新发现

1. **未覆盖**：LGC-LG-47 的并发时序（§2 末）；`t60`（不完整帧的错误上下文）按队长安排另行补验；我也没有复跑 t44 自述的其它邻域（`drain_to_eof` 的满管道子类）。
2. **新发现（low）**：`an_answer_from_a_child_that_never_exits_is_delivered` 的**默认** `ProcessLimits` 并非 600 ms —— 若把那两行 `timeout` 覆盖删掉，该钉子仍会过，但它就不再覆盖"帧到即交付、不等 deadline"这一点（因为 30 s 的 sleep 与默认 deadline 的关系决定了它是靠交付路径还是靠超时路径通过的）。建议把"交付时刻 < deadline"这一断言之外的**默认值**也钉住（或在夹具注释里写明为什么必须显式设 600 ms）。
