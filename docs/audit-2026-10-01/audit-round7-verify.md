# 第七轮对照评测审计（t13 / auditor）— MCP × codegraph，每臂 22 题

## 装置与时间线
- 桥 = 冻结快照 `target/round7/src-target/debug/xirang-mcp`，sha256 前置 `67b8f3ff2769ab66`（与 `BRIEF.md` §0 一致），构建自冻结检出 `target/round7/src` = 提交 `6d180b8`。**主检出 `target/debug/xirang-mcp` 在 t14（已落地为提交 `9998be3`）之后 `check` 第 1 行多一条 `verdict`** ⇒ 本报告一切复现/比对都用快照，未用过主检出二进制。
- **cg 臂装置版本**：数据产自 codegraph CLI **1.5.0**，日志/答案 mtime **18:37:13–19:04:04**；维护者在 **19:11** 升到 **1.6.1** 并删除 `~/.codegraph/versions/v1.5.0/`（现只剩 v1.6.1）⇒ **1.5.0 已不可复现**。22 份 cg 答案与日志的 sha256 与 `CONTROL-FREEZE.md` **22/22 逐字吻合**（我自算），本报告因此是这批 1.5.0 数据的唯一判分依据；**凡需重跑 CLI 才能判定的事项一律标「无法判定（版本已变）」，不猜**。审计全程未运行过任何 `codegraph` 命令。
- 题树：22 题 × 2 臂。只读树 `s1–s8`/`g4-<arm>` 的 `git status --porcelain` **全空**，全树无 `.codegraph/` 残留（唯一命中的 `.audit/pin-counterproof/.codegraph` 是我 t15 的审计副本）。答案 44 份、日志 44 份齐备，最长答案 **30 行**（≤30 满足）。
- **exit 码按日志逐行解析**（不采信自报）：mcp 113 行 = exit0 **108** / exit1 **2** / exit2 **3**；cg 84 条 `=== EXIT:` **全部 0**。exit2 = 客户端用法错；exit1 = 工具级拒绝（如缺 `face`）。

## 四轴（逐轴给数）
### ① token 轴：估算器 chars/3.5，分母＝各臂自己日志里的仪器回复
| 臂 | 原始字符 | 原始 token | carried（逐题 N） | carried（全臂 N） | carried/原始 |
| mcp | 166,709 | 47,631 | 341,519 | **10,691,607**（3,054,745 tok） | **64.1×** |
| cg | 149,749 | 42,785 | 403,212 | **7,156,945**（2,044,841 tok） | **47.8×** |
carried = Σ 每条回复字符 × 其后剩余调用数（1 起算），**无缓存上界**（缓存/前缀复用/旧轮裁剪都会更低）；与仓库自己的 `target/trim-fixture/carried.py` 同式，我自写脚本 `.audit/audit-metrics.py`。比值：原始 mcp/cg = **1.11×**，全臂 carried = **1.49×**（第六轮基线 1.42×）。
**会话 vs 逐进程（两个口径都要报）**：两臂**都用一次性 `--call` 进程**，t1 的"同根首答全量、后续一行"压缩**在本轮从未生效**——臂上的读数就是"逐进程"那一列。我用冻结快照在同一 12 条只读批次上实测两形：**逐进程 12,944 字符 / 单 stdio 会话 10,328 字符，差 −2,616**（t1 折叠值 2,616 字符，与 `docs/audit-2026-10-01/audit-round6-optimize-verify.md` 记的 2616 逐字相同）；折叠只发生在 `search/inspect/read/callgraph` 第二次起的 roster 上。
**实测对照（不靠估算）**：两臂会话 `usage` 的真实重发量 mcp cacheRead 63,716,608 + input 248,361、cg 56,221,568 + 182,938 ⇒ mcp/cg = **1.13×**（有缓存的实际值，低于无缓存上界的 1.49×）。**分母**：113 与 84 是各臂日志的仪器调用数，不等分母，不可混读。

### ② 链长轴
| 臂 | 步数 (step) | 思考块数（含空） | 推理字符 | 每步推理字符 | 每仪器调用推理字符 |
| mcp | 304 | 301（非空 177） | 231,237 | 761 | 2,046 |
| cg | 273 | 273（非空 172） | 404,046 | 1,480 | 4,810 |
只报字符会说"cg 链长得多"；真实形态是 **cg 步数更少（273 < 304）却每步更长（1,480 vs 761）**——正是"步数相同、每步更短"的反面。交付正文（答案文本）mcp 10,634 / cg 11,058 字符，几乎相同；差在思考。

### ③ 总调用轴（三栏，口径写清）
| 口径 | 仪器 | 其它（按用途） | 合计 |
| mcp（jsonl 记桥 + 会话记其余） | 113 | 304 = bash 154（cargo test 48 / fs·git 86 / shell grep 11 / 壳 9）+ read 37 + write 42 + edit 37 + grep 工具 1 + 协作 33 | 417 |
| cg（日志记 codegraph + 会话记其余） | 84 | 250 = bash 129（cargo test 50 / 其余 shell 79）+ read 20 + write 43 + edit 28 + 协作 30 | 334 |
| cg（只数命令，第六轮可比口径） | 84 | 129 | 213 |
| mcp（只数命令） | 113 | 154 | 267 |
分母不同不可混用：**mcp 的 jsonl 只记桥调用**（113），**cg 的 `=== CMD:` 只记 codegraph 命令**（84），两臂的"其它"都得去各自会话说。第六轮基线：mcp 140 / ≤310 / ≤450；cg 118 / 92 / 210。

### ④ 反复分解（同命令重复＝信息导致／同工具换目标＝有产出／被拒后重试＝浪费）
- mcp 113 次：相邻同命令重复 **4**（信息导致）、同工具换目标 **27**（有产出）、**被拒后重试 3 次（浪费）**（fa/fb 各 1 次 `apply` 参数形状、s3 1 次 `--json` 位置）；相邻 `callgraph→callgraph` 7、`callgraph→read` 1。
- cg 84 次：相邻同命令重复 **0**、同工具换目标 20（有产出）、被拒 **0**；相邻 `callers→callers` 15（s3 逐符号核查）。
- **第六轮三处浪费的本轮读数（我自算）**：
  1. **被拒 9（含 apply 5）→ 5（含 apply 2）**：mcp 逐行 = exit1 2（fa/fb 的 `apply`，都因参数形状）+ exit2 3（s3 两次 `--json` 位置、r1 一次 `callgraph` 漏 `--function`）；cg 0。**下降**。
  2. **s1 上 6 次零贡献调用 → 0**：mcp s1 = 3 次全 exit0（status / callgraph post / search literal），**0 拒绝、0 零命中**（`literal post(` 有命中并带成员分组）；cg s1 = 3 次（init / callers post / explore）0 拒绝。**下降**。
  3. **`callgraph→callgraph` 16→10 再 → 7；`callgraph→read` 10→6 再 → 1**。**继续下降**。

## 标签（分块；集合含"平手"）
块一 = 第六轮那 21 题（r1–r4 / s1–s8 / fa–fe / g1–g3 / h1）；块二 = 新增 g4。
| 块 | mcp | cg |
| 一（21） | **1 真赢（r4）/ 20 平手 / 0 输** | 0 / 20 / **1 输（r4）** |
| 二（1，g4） | 1 平手 | 1 平手 |
| 合计（22） | **1 / 21 / 0** | **0 / 21 / 1** |
对照第六轮那 21 题：mcp 2 真赢 / 19 平手 / 0 输，cg 0 / 19 / 2 ⇒ **mcp 的胜场 2 → 1**：cg 在 **s3**（补上"测试函数由框架运行期调用"那条边界）与 **fc**（守住台账、给出前后 sha256 与 `git ls-files`）追平，mcp 在 **r4**（真构造两条捷径并拒绝）仍领先。"不具鉴别力"两轮都是 0——两臂同判一律记平手。
**样本独立性**：s1–s8 同源（同一棵 pristine + 各自注入）、fb↔g3 同能力、r4↔s6 同谓词 ⇒ 上表是块合计，逐题读数在 `audit-detail.md`，两者都要读、别把同源样本当独立样本。

## 判法要点（全部是我自己的验证，不引臂的报告）
- **四题**：8 棵树（`rN-mcp`/`rN-cg`）与 `pristine-N` 的 `diff -r`（排除 target/.git/Cargo.lock）**全部为空**——`tests/` 未动、无 `#[ignore]`/`#[allow]`、无删断言；门我自跑，8 棵树 `cargo test --offline` 全 rc=0、**9 passed / 0 failed**。
- **r4 陷阱**：mcp 会话 t1s36 真构造两条捷径——`|| amount != 0` → `7 passed; 2 failed`、只留 `has_receipt` → `8 passed; 1 failed`，随后从 `/tmp/r4-entry-good.rs` 还原（交付态与 pristine-4 逐字节相同）；cg 会话里只有 grep 与 `git diff --stat`，**没有构造记录** ⇒ 只此一题分胜负。
- **八关**：s3 问域两臂都写清（`#[test]` 由框架运行期调用，故只计数/排除）；s6 按预设容忍度两臂都点"零金额那一侧"且含 `(false,0)`，都没写成"`has_receipt == false` 两格"；**s5 两处病灶的独立性，两臂都只是推理、都没有构造**（会话里没有"只修其一再跑"），按判据同判平手。
- **五族**：门我自己跑（14 棵树 × 两面 = **28 个 rc=0**：a-extend 27、b-deepen 27/28、c-adopt 27、d-one 4/5、e-many 4/5）；`fc` 台账我自核——两树 `.xirang/adopted/entries` sha256 均 `541cbe86…38a3`（337 B / 1 行 / 被 git 跟踪），一字未动、两臂都没伪造条目；两树 `Cargo.toml 8d6b35ed…`、`button.rs e842be74…` 与 cg 自报吻合。
- **新五关**：g1 我用快照复现 `check --census true`（`test-reachable: 1 of 12` + `no test reaches audit_unused (crates/core/src/audit.rs:14)`，与 mcp 逐字一致）；g2 两臂的伪代码都是**意图**、差都指向"`render` 从不调用 `write_count`"；g3 我自跑 `cargo run --example tree` 两树都 **3 行**、改动面各只有 1 个文件、门两面绿；**g4 按答案键**——两臂都列 `bands.rs:36` 与 `bands.rs:54`，都没把 `bands.rs:42`/`store.rs:22`/`buckets.rs:8`/`query.rs:22`/`bands.rs:92` **列为臂**（cg 提 `State::Dormant` 是在"判不了"栏里，正是答案键要的边界自报），且都自报边界（数据相关不判、宏展开/动态派发/FFI 不可见、非覆盖率量度）；h1 两臂都处置了换面才现的 `crates/core/src/audit.rs:9`，mcp 另处置 census 常量栏（`buckets.rs:10` 改读 `SMALL_LIMIT`，4 files 5+/3− vs cg 3 files）。
- **门**（我自跑）：8 棵 r 树 + 14 棵族/关树 × 两面，**全部 rc=0**，0 failed。

## 我跑的两条门禁（原始输出 `.audit/repo-gate.log` / `.audit/probe-loop.out`）
- **仓库门禁复合 rc=0**：`cargo fmt --all -- --check && cargo test --workspace --offline && cargo clippy --workspace --all-targets --offline -- -D warnings && cargo clippy --workspace --all-targets --offline --all-features -- -D warnings && tools/xirang-publish --check-table` → workspace **688 passed / 0 failed**，`--check-table` 打印 `dependency table matches the manifests (3 crates)`。
- **夹具机械探针循环 rc=0**：`bash .audit/probe-loop.sh` → 22 棵交付树 **36 次** `cargo test --offline`（8 棵 r 树默认面 + 14 棵族/关树两两两面对）全部 rc=0 / 0 failed，外加 `g3-mcp`、`g3-cg` 的 `cargo run --example tree` 各 3 行；`PROBE_LOOP_FAIL=0`。

## 硬约束与矛盾（逐条给证据）
- **`git log/show/stash`**：两臂日志 0 处。会话里 mcp **1 处 `git stash list`**（t8s24，自检 "check git stash-free"，只列举），cg 0 处 —— 字面违反 BRIEF §4 的 `git stash` 禁令，未改任何判定。
- **跨题读别人的树**：mcp **0 处**。cg 在答 g3 时（turn 6 的 t6s21、t6s23、t6s24、t6s27–t6s31，共 **8** 条命令）**读并拷贝 `c-adopt-then-extend-cg/src/control/object/button/button.rs` 当还原基线** —— 违反 BRIEF §4「不跨题读别的题目录」（第六轮同类违规是"跨题读 s1"）；只读、未改那棵树。
- **只读树残留 `.codegraph/`**：无（10 棵只读树全无，`git status` 也全空）。
- **读越界（备查）**：mcp 在 t4（fa/fb）读了 `target/round7/src/toolchain/src/**` 共 24 次（冻结检出的工具源码，用于查 `PartsContract`/`deepen` 语义）；BRIEF §0 把可读范围写成"本题面 + 自己的题树"，此项不在三条点名约束内。
- **矛盾两处（贴两边原文）**：① r1-mcp 答案自报「被拒 2 次（exit 2，用法错误）：`callgraph the_filter_...`（缺 `--function`）与裸 `callgraph matches`」，而 `log-mcp-r1.jsonl` 逐行只有 **1 次** exit2（第 4 行裸 `callgraph matches`：``expected `--<key>`, got `matches` ``）——按 BRIEF 以逐行解析为准，采信 **1 次**。② fa-mcp 答案自报 `git status` = 3 改动 + 1 新目录，实际树上还多一项 `M Cargo.toml`（一句注释）——自报不全，交付不受影响。
- **账面不一致（备查）**：`team.json` 里 t5（mcp 臂 fc–fe）的 assignee 是 `captain`，但答案与日志由 arm-mcp 会话产出（turn 8 / t8s27–t8s28 写 `answer-mcp-fc/fd/fe.md`）。

## 逐条「通过 / 不通过 / 无法判定」
- **通过**：四轴四栏（①token 含原始+carried+两形、②链长三栏、③总调用三栏、④反复分解+三处浪费对照）· 分块标签（21 + g4）· 四题/八关/五族/新五关的判法要点 · 硬约束 5 条 · 44 份答案 ≤30 行 · 44 份日志齐备 · 22 棵交付树门全绿。
- **不通过：0 条。**
- **无法判定（版本已变）**：`CONTROL-FREEZE.md` 的复用规则要求"复用前重跑 1–2 个共享题做一致性抽查"——codegraph 已升 1.6.1 且 v1.5.0 目录被删，**无法执行** ⇒「cg 1.5.0 的输出可复现」这一条本轮**无法判定**；同理任何"重跑 `codegraph <子命令>` 看它是否仍给日志里那句话"的复核都标无法判定。我改用「日志逐字 + 冻结 sha256 + 树内源码对读」完成判定，凡此来源的结论都在 `audit-detail.md` 里注明依据。
