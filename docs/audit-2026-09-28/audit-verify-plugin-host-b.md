# Independent verification of `t60` — incomplete frames in the process adapter (`LGC-LG-32`, second half)
# `t60` 独立复核 — 进程适配器的不完整帧（`LGC-LG-32` 第二半）

- Verifier / 复核人: `run-engineer` (not the author) · 非作者
- Author / 作者: `executor` (task `t60`)
- Task / 任务: `t77`
- Tree / 树: `/home/nich/Moirai_N3/nichlink`, `plugin-host/src/process/child.rs`
  (`read_frame` at `:164`, `incomplete_frame` at `:199`), unchanged by this verification
  （本复核未改动该树）
- Isolated copy / 隔离副本: `/tmp/t77-iso` (own `CARGO_TARGET_DIR`), probes appended to its
  `plugin-host/tests/fault_matrix.rs` under `mod process_faults`
  （探针放在副本内，自带 `CARGO_TARGET_DIR`）

## Verdict / 结论

**Pass — the claim holds, with one recorded boundary.** The two twin read points of a frame
now report a process-level error that names the operation, the frame shape, and the reader's
own `Display` and `kind`; the bare `Io(UnexpectedEof, "failed to fill whole buffer")` is gone
from both. The regression surface (Limit early-exit, non-zero exit preferring stderr) is
unchanged.
**通过——主张成立，并记下一条边界。** 帧的两个孪生读取点现在都给出过程级错误，点名操作、帧形状
以及读取器自身的 `Display` 与 `kind`；裸的 `Io(UnexpectedEof, "failed to fill whole buffer")`
在两个点上都不再出现。回归面（Limit 早退、非零退出优先用 stderr）未变。

## 1. The task's shape, measured on my own fixture / 任务要求的形状，用我自己的夹具实测

Fixture (mine, a shell script, not the author's pin text): a child that writes the **complete
4-byte little-endian length prefix declaring 6 bytes**, then **1 payload byte**, then `exit 0`.
夹具（我自己的 shell 脚本，不采信作者钉子原文）：子进程写完**声明 6 字节的完整 4 字节小端长度
前缀**，再写 **1 个负载字节**，然后 `exit 0`。

```
T77 message: the process adapter got no complete frame for `render`: it declared 6 bytes of
answer and the stream ended early (failed to fill whole buffer; kind UnexpectedEof)
T77 display: plugin process failed: the process adapter got no complete frame for `render`: it
declared 6 bytes of answer and the stream ended early (failed to fill whole buffer; kind
UnexpectedEof)
```

- variant / 变体: `HostError::Process(_)` — **not** `Io`（不是 `Io`）
- operation name / 操作名: `` `render` `` (the name passed to `PluginInstance::call`)
- frame shape / 帧形状: `it declared 6 bytes of answer and the stream ended early`
- reader `Display` / 读取器的 `Display`: `failed to fill whole buffer`
- reader `kind` / 读取器的 `kind`: `UnexpectedEof`

## 2. Both twin read points are covered / 两个孪生读取点都被覆盖

| read point / 读取点 | fixture / 夹具 | observed / 实测 |
| --- | --- | --- |
| 4-byte length prefix (`child.rs:170`) | 3 bytes then `exit 0` | `Process("… its 4-byte length prefix was cut off (failed to fill whole buffer; kind UnexpectedEof)")` |
| payload (`child.rs:180`) | prefix declares 6, 1 byte written, `exit 0` | `Process("… it declared 6 bytes of answer and the stream ended early (…)")` |

Each point names its own shape string, so a reader can tell *which half* of the frame was
lost — that is the discriminating detail this verification was about.
每个点写自己的形状字符串，因此读者能分辨**帧的哪一半**丢了——这正是本次复核要看的判别性细节。

## 3. Red side, reproduced by the verifier / 红侧由复核者自己复现

Mutation 1 — prefix point reverted to the old behaviour (`map_err(HostError::Io)`):
变异 1——前缀点退回旧行为（`map_err(HostError::Io)`）:
`process_faults::t77_both_read_points_are_reached ... FAILED` (the probe sees `Io`).

Mutation 2 — payload point reverted the same way / 变异 2——负载点同样退回:
`t77_incomplete_frame_names_the_operation_and_the_reader ... FAILED` **and**
`t77_both_read_points_are_reached ... FAILED`.

Both mutations were reverted; `cmp` against the live tree is byte-identical and
`sha256sum` of `plugin-host/src/process/child.rs` agrees on both sides
(`352d9fa94382c3b53424027398176d73a7b2c6c43dd1d4f9bb9e46b2ec3f8db0`), and
`diff -r plugin-host/src` between the copy and the tree is empty — **zero residue**.
两个变异都已还原；与活树 `cmp` 逐字节相同、两侧 `child.rs` 的 sha256 一致
（`352d9fa9…3f8db0`），副本与树的 `diff -r plugin-host/src` 为空——**零残留**。

## 4. Regression surface / 回归面

| surface / 面 | probe / 探针 | observed / 实测 |
| --- | --- | --- |
| `Limit` early exit (declared length over the cap is refused before allocating) | declares `0x7fffffff`, cap 1024 | `Err(Limit("output is 2147483647 bytes; limit is 1024"))` — unchanged |
| non-zero exit prefers stderr | `echo 'plugin-specific detail' >&2; exit 7` | `Err(Process("plugin-specific detail"))` — unchanged |

Author's fault matrix re-run by the verifier / 复核者重跑作者的故障矩阵:
`cargo test -p nichlink-plugin-host --offline --features process-tools` → **27 passed;
0 failed** (matches the author's claim); the default configuration passes as well (the process
module is behind `process-tools`, so those tests do not compile there — read `0 passed` as
"nothing ran").
`cargo test -p nichlink-plugin-host --offline --features process-tools` → **27 passed /
0 failed**（与作者报数一致）；默认配置同样通过（进程模块门控在 `process-tools` 之后，因此那里
这些测试不会被编译——把 `0 passed` 读作"什么都没跑"）。

## 5. The "reachability" trade-off / "可追链"这条取舍

`HostError::Process` carries a `String` and **no `#[source]` slot**, so the `io::Error` cannot
be recovered as an error object; the author instead writes its `Display` (`failed to fill whole
buffer`) and its `kind` (`UnexpectedEof`) into the message.
`HostError::Process` 携带 `String` 而**没有 `#[source]` 槽位**，因此 `io::Error` 无法作为错误对象
取回；作者改为把它的 `Display`（`failed to fill whole buffer`）与 `kind`（`UnexpectedEof`）写进消息。

**Judgement: sufficient for this seam.** The classification a caller acts on
(`UnexpectedEof` vs `TimedOut` vs `BrokenPipe`) survives as `kind`, and the human-readable
half survives as `Display`; nothing the host would branch on is lost, because the branch it
takes is the *variant* (`HostError::Process`) plus the operation name, both of which are
first-class. Adding a real source slot would mean changing the shape of a published enum
(`HostError::Process(String)` → a struct variant with `#[source]`), which is a breaking change
for `0.1.x` and is not worth it for a diagnostic seam.
**判断：对这个接口够用。** 调用方据以分支的分类（`UnexpectedEof` / `TimedOut` / `BrokenPipe`）
以 `kind` 存活，人类可读的一半以 `Display` 存活；宿主真正会分支的东西一样不少，因为它走的分支是
**变体**（`HostError::Process`）加操作名，两者都是一等公民。要加真正的源错误槽位就得改已发布枚举
的形状（`HostError::Process(String)` → 带 `#[source]` 的 struct 变体），对 `0.1.x` 是破坏性变更，
为一条诊断接口不值。

**Minimal improvement worth considering (not made here, source untouched)**: the message says
*what was declared* but not *what arrived*. Reporting the received byte count — e.g.
`it declared 6 bytes of answer and 3 arrived, then the stream ended` — would separate "the
plugin wrote nothing" from "the plugin wrote half a frame", which is the one question the
current text cannot answer. It costs a manual read loop instead of `read_exact` (a counting
reader wrapper keeps it small) and no public API change.
**值得考虑的最小改法（本次未做，源码未动）**：消息说了**声明了多少**，没说**到了多少**。报出已收到
的字节数——例如 `it declared 6 bytes of answer and 3 arrived, then the stream ended`——就能把"插件
什么都没写"与"插件写了半帧"分开，而这正是当前文本回答不了的那个问题。代价是把 `read_exact` 换成
一个计数包装的读取循环（不影响公开 API）。

## 6. Gates on the pinned tree / 静置树上的门禁

| gate / 门禁 | result / 结果 |
| --- | --- |
| `cargo test -p nichlink-plugin-host --offline` | `0 passed` (process module feature-gated), suites green |
| `cargo test -p nichlink-plugin-host --offline --features process-tools` | **27 passed, 0 failed** |
| `cargo test --workspace --offline` | `EXIT=101`, 41 suites ok; the only failure is the known offline-environment case `studio::app::tests::new_project::new_project_and_explicit_root_face_compile` (`failed to select a version for the required nichlink-run-method = "^0.1.6"`); with `-- --skip new_project_and_explicit_root_face_compile` → **`EXIT=0`, 58 suites ok**（唯一失败是已知离线的环境用例；加 `--skip` 后 EXIT=0 / 58 套件全绿） |
| `cargo test -p nichlink-conventions --offline` | **131 passed, 0 failed** |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | `EXIT=0` |
| `cargo fmt --all -- --check` | `EXIT=0` |

No source file was modified for this verification; `plugin-host/src` in the isolated copy is
byte-identical to the tree. The only artefact of this task is this document.
本次复核未修改任何源码；隔离副本里的 `plugin-host/src` 与树逐字节相同。本任务的产物只有本文档。
