# MCP vs codegraph: the reasoning-chain fit evaluation (2026-09-29)
# MCP 对 codegraph：思维链拟合评测（2026-09-29）

The maintainer's brief, verbatim: 「你再测试一下mcp,然后看看和codegraph的区别，效果是不是更好，指令执行度，
debug效率和效果（效率为 agent 使用我们的 mcp 改对是否使用更小的思维链，相对 codegraph 的思维链更短更高
效）（效果为架构和代码质量更好条理更加清晰）以此作为测试方向编制测试项目，设计真实bug,预设完美debug路
径，测试我们的思维链的拟合程度。」
维护者原话（同上）。This record answers it with a purpose-built project, four hand-designed real bugs,
a preset ideal path per bug, and two agent arms measured with the same instrument.
本记录用一个专门编写的项目、四个手工设计的真实缺陷、每个缺陷一条预设理想路径，以及用**同一把尺子**度量的
两组代理来回答它。

## 1. Staging / 装置

- `tools/nichlink-mcp-eval` **builds the project** — `ledger`, a small ledger service (account/entry
  model, in-memory store, bucketed index, filter/summary query, report renderer, integration tests;
  10 files) — and **injects one designed bug at a time**. Each bug carries its symptom, its site, the
  **shortcut that must be refused**, and the **preset ideal debug path**: five decisive steps, each
  naming the question it answers.
  `tools/nichlink-mcp-eval` **编写这个项目**（`ledger`，10 个文件）并**逐个注入设计好的缺陷**。每个缺陷带
  症状、站点、**必须拒绝的捷径**，以及**预设理想调试路径**：五个决定性步骤，每步点名它回答的问题。
- The four bugs / 四处缺陷: ① `Filter::matches` minimum-amount guard inverted (shortcut: weaken the
  test's floor) ② the call to `write_totals` deleted from `render` (shortcut: inline the totals)
  ③ `Bucket::contains` upper bound made inclusive (shortcut: change the test's boundary) ④
  `Entry::postable` turned into `||` so a zero entry posts (shortcut: delete the check or make it
  always true).
- **Every bug is mechanically self-proved**: pristine project green → injected project red on exactly
  the designed test → the documented fix green again, and the shortcut named.
  **每个缺陷都机械自证**：原始项目绿 → 注入后恰好那条设计测试红 → 文档里的修法转绿。
- Eight trees (4 bugs × 2 arms), byte-identical within a bug, each a git checkout. The briefs forbid:
  comparing another copy, reading or running the harness that holds the truth and the preset paths,
  using git history (`git log`/`git diff HEAD~1`/`git show` — the injection is one commit back), and
  weakening tests (`#[ignore]`/`#[allow]`/relaxed assertions).
  八棵树（4 题 × 2 组），同题两份逐字节相同、各自是 git 检出。题面禁止：与别的副本比较、读或运行持有
  真值与预设路径的出题脚本、用 git 历史（注入就在上一个提交里）、以及把测试改松。
- `tools/nichlink-mcp-eval-chains` (Node) measures from each member's **own session log**: the
  multi-frame zstd is decoded frame by frame, rounds are cut at the task claim, and per round it
  reports reasoning characters/blocks, substantive tool calls, and coverage of the preset steps.
  Instrument calls are read from the logs the briefs demanded, never guessed.
  该工具（Node）从每名成员**自己的会话日志**里量：多帧 zstd 逐帧解、按任务认领切分轮次，每轮报推理字符/
  块数、实质工具调用、以及对预设步骤的覆盖。仪器调用取自任务书要求留下的日志，不靠猜。

## 2. Measured / 实测数字

| metric 度量 | ours (bridge) 我们 | codegraph | reading 读法 |
| --- | --- | --- | --- |
| reasoning characters 思维链字符 | **60,059** | **39,320** | ours **+53%** ✗ |
| substantive tool calls 实质调用 | 66 | 69 | equal 持平 |
| instrument calls 调用自己那件仪器 | 27 (`tools/call`) | **22** (CLI) | same basis: ours **+23%** ✗ |
| preset-path fit 预设路径拟合 | 5/5 per round (substance) | 5/5 per round (substance) | equal 持平 |
| fix shape 修复形状 | 8/8 one place, 1 line, restored, green | same | equal 持平 |
| shortcuts taken 走的捷径 | 0 | 0 | equal 持平 |

A second measurement explains the first: in round 1 the bridge log holds **33 frames, of which 24 are
`initialize`/`initialized`** — about **73% of the traffic is per-call protocol**, because the arm
starts a fresh process and hand-rolls JSON-RPC for every call.
第二个度量解释了第一个：第 1 题的桥日志有 **33 帧，其中 24 帧是 `initialize`/`initialized`** ——约
**73% 的流量是"每次调用一份"的协议开销**，因为这一组每次都得新起进程、手搓 JSON-RPC。

## 3. The four axes the maintainer named / 维护者点名的四个轴

- **Instruction adherence 指令执行度 — we lose 0/4** ✗. The bridge's `initialize.instructions` says
  "Use `nichlink.search` before reading source"; the MCP arm's **first source access in all four
  rounds was `read`**, round 4 used `search` **zero** times, and none of its four reports quoted the
  sentence. The codegraph arm, by contrast, treated *its* tool's caveats as authoritative: it named
  the blast-radius/`affected` false negatives in every round and refused to draw conclusions from
  them. A prose instruction that the cheap path does not make obvious was ignored every time.
  **指令执行度 0/4**：桥的 `initialize.instructions` 写着"读源码前先用 `search`"，而这一组四轮的**首次
  源码访问全是 `read`**、第 4 题 `search` 用了 **0 次**、四份报告也从未引用那句话。对照组反过来把它那件
  工具的告诫当真（每轮点名 blast radius/`affected` 的假阴性并不拿它当结论）。**一条没有让便宜路变得显眼
  的散文指令，一次都没被遵守**。
- **Efficiency 效率** — our chain is **not** shorter: +53% characters, comparable calls, and we call
  our own instrument *more* (27 vs 22).
  **效率**：我们的思维链**没有更短**（+53% 字符、调用持平、调用自家仪器还更多 27 对 22）。
- **Effect 效果** — a tie on outcomes: 8/8 sites hit exactly, 8/8 fixes confined to one place, all
  eight trees restored and green, zero shortcuts. Per round the audit reads r1 as a tie (the two
  trees' results are byte-identical blobs), r2/r3 slightly stronger for codegraph (it *proved* the
  missing edge with `callees`/`callers`; one `explore` put `<=` and "Exclusive" on one screen), r4
  stronger for codegraph on justification (a five-row predicate truth table, plus the observation
  that a bizarre `has_receipt != (amount == 0)` would also fool all nine tests) and stronger for us
  on self-discipline (we measured the shortcut, saw it turn the suite green, and reverted it).
  **效果**：结果上打平——8/8 精确命中、8/8 改动局限一处、八棵树全部还原且绿、零捷径。逐题读法：题 1
  平手（两棵树的产物逐字节相同），题 2/3 codegraph 略强，题 4 它在**判据**上略强（谓词真值表），我们在
  **自律**上略强（实测捷径变绿后还原）。
- **Fit 拟合度** — equal in substance: both arms walked the preset path. The metric is sensitive to how
  the patterns are written, so it is reported as "both reached the ideal steps", not as a score.
  **拟合度**：实质持平（两组都走到了预设步骤）；该指标对模式写法敏感，因此报"都走到了"而不是分数。

## 4. Why the chain is longer / 链为什么更长

1. **No directly callable command 没有可直呼的命令**: the arm hand-rolls JSON-RPC, builds the binary,
   writes its own log — that plumbing is reasoning tokens, and 73% of its protocol traffic is
   per-call `initialize`. codegraph is one `bash codegraph explore "…"`.
2. **Answers need a follow-up 答案常需补一次调用**: `converge` gives callee *names* without source
   (one extra `callgraph` per hop); `literal` hits carry no context; nothing answers ordinary Rust
   semantics (bounds, guard direction) — so our answers hand over facts to interpret, where
   codegraph's `explore` hands over verbatim source with the doc on the same screen.
3. **The prose instruction does not shape behaviour** (§3 above).
   **散文指令不塑造行为**（见 §3）。

## 5. Tool-defect ledger (both sides; reproduced by the captain unless noted)
## 5. 工具缺陷账（两侧；除注明外均由队长亲手复现）

- codegraph: `affected <file>` reports `No test files affected` under an integration-test layout
  although the file *is* covered by `tests/ledger.rs`; the `explore`/blast-radius coverage hints
  report covered symbols as "no covering tests found" (three rounds running); its command log claims
  "every command + raw output + EXIT" while three decisive `sed -n … tests/ledger.rs` reads are absent
  from it (a record-fidelity gap, not a rule violation).
- ours: **read-only for ordinary Rust** — `apply` writes registration faces only, so the fix must be
  made outside the bridge (all four rounds complained); **no orphan view** ("defined here, zero
  callers") — the missing-edge round had to diff callees by hand; `converge` without source;
  `literal` without context.

## 6. Queue from this evaluation / 本次评测带出的队列

① Give the bridge a **directly callable command** (`nichlink call <tool> --json …`) so an agent does
not hand-roll JSON-RPC. ② `converge`/`callgraph` should return source by default (or let `converge`
hand over "the deciding line and the doc above it"), and `literal` hits should carry `context`.
③ Add an **orphan view** ("defined in this package, zero callers").

## 7. Accounting and corrections / 记账与更正

The captain's first numbers were wrong in three places; the audit caught all three and the captain
re-verified each by hand before accepting them.
队长的第一批数字有三处错，审计全部抓到，队长逐条手工复核后才接受。

1. "Our 27 instrument calls vs its 36" compared unlike things ✗: 36 is **every** command in its log
   (8/9/9/10), of which codegraph CLI is **22** (4/6/6/6); the other 14 are `cargo`/`git`/`grep`. Same
   basis: **27 vs 22**.
2. "Both arms pointed out the `contains` doc inaccuracy" ✗ — only `report-cg-r3.md` names it (and says
   why it would not fix it in passing); the MCP reports never do. Re-checked by grep for the *claim*,
   not for the symbol name.
3. "We call our instrument less" ✗ — 27 > 22; and the bridge's protocol overhead (24 of 33 frames in
   round 1) is the hand-rolled driver, not the tool.

Independent evidence the audit added, re-checked by the captain: all eight `tests/ledger.rs` files are
one byte string (`md5 c39b1b4264989855eebf1e3ec4b2946f` × 8 ⇒ no assertion was relaxed), every tree's
diff is one file and one line, every store has a single commit and a clean status, and the hard
constraints (`tools/nichlink-mcp-eval`, `git log`/`show`/`HEAD~`/`reflog`, `diff -r`/`cmp`, other
trees' paths) have **zero** hits across all eight reports and both logs.

## 8. Dissecting the chain: where the +53% actually went / 解剖思维链：那 +53% 花在哪

The maintainer's reading — "our description surface is too bare and unorganised; our input ought to
be smaller and more precise, so our chain ought to be shorter" — is **half right, and the measurement
says which half**.
维护者读法（「描述太裸没有组织、按理输入更小更精确、约束更强、本该更好」）**对了一半**，而实测说清了是哪一半。

**The surface is not small — it is large and unordered / 描述面不小，而是"大而无序"**

| part 部分 | size 体量 |
| --- | --- |
| `initialize.instructions` (the whole workflow guidance) 流程指引全文 | **202 字符（一句话）** |
| 22 tool descriptions 工具描述 | 27,846 字符 |
| 22 input schemas 入参 schema | 8,527 字符 |
| total surface 描述面合计 | **36,373 字符** |
| largest single description 单条最大 | `diff` 2,195 / `search` 2,185 / `apply` 2,139 |
| smallest 最小 | `status` 61 |

So the token premise is false in the direction that hurts: we ship **180× more per-tool prose than
workflow guidance**, the agent reads the *first clause* of each description (that is what its own
digests quote), and nothing anywhere says *which tool answers which symptom*.
因此"输入更小"这条在**伤人的那一侧**不成立：每条工具的说明是流程指引的 180 倍，而 agent 只读每条描述的**第一小句**
（它自己的摘要就是这么引的），而且没有任何地方说**哪种症状该用哪个工具**。

**What reached the model, and how long it stayed / 实际到达模型的是什么、留了多久**

One `tools/list` payload arrived as a **17,917-character tool result — and truncated** (the arm's own
hand-rolled client capped it), after which it was carried through **52 further assistant steps**.
一次 `tools/list` 的输出以 **17,917 字符的一帧**到达，**而且被截断**（该组自己手搓的客户端设了上限），随后**被 52
个后续步骤一直携带**。

**The first third of round 1 is a client-building tax / 第 1 题前三分之一是"自建客户端税"**

Reading our arm's 25 reasoning blocks in round 1: blocks 0–10 are build the binary, check its
freshness, **write a Python MCP driver**, debug a hang (`notifications/initialized` without an id), a
`pkill` that matched its own shell, and the truncated list — including the moment it asked itself
whether a bridge "oriented at nichlink registration faces" even applies to a plain Rust project.
The diagnosis then happens in **three blocks** (the guard, its doc, the fix). Codegraph's round 1, same
brief: block 0 plans, block 2 runs one `explore`, **block 3 names the site**. Same verdict, and the
entire difference sits in the first third.
读我们这组第 1 题的 25 个推理块：块 0–10 是构建二进制、核对它新不新、**写一个 Python MCP 驱动**、排查一次挂起
（`notifications/initialized` 没带 id）、一次把自己的 shell 也匹配上的 `pkill`、以及那份被截断的清单——其中还
包括它自问"这个面向 nichlink 注册面的桥是否适用于一个普通 Rust 项目"。定罪只用了**三块**（守卫、它的文档、修法）。
codegraph 第 1 题同一份题面：块 0 计划、块 2 一条 `explore`、**块 3 点名病灶**。同样的判定，差别全在前三分之一。

**Why the prose instruction was ignored 4/4 / 那条散文指令为什么 0/4 被无视**

Because guidance lives in prose while the agent's decisions are shaped by **what an answer hands
back**. The one instruction that did work is the one embedded in a *response*: `search` with no match
says "pass `literal`", and the arm then used `literal` correctly in round 2. That is the design lesson:
put the next step in the answer, not in a preamble.
因为指引住在散文里，而 agent 的决定由**答案交回了什么**塑造。唯一生效的指令是嵌在**响应**里的那条：`search` 查不到
时会说 "pass `literal`"，而该组在第 2 题就正确用上了 `literal`。这就是设计结论：**把下一步放进答案里，而不是放进开场白**。

**Ranked causes / 排序后的成因**

1. **No client / 没有可直接调用的客户端** — dominant: it costs the first third of the first round plus a
   protocol-hang detour, and the client it writes truncates the surface. Codegraph pays none of it.
2. **Large, unordered surface / 大而无序的描述面** — 36.4k characters of essays against 202 characters
   of procedure; the first clause is what gets read.
3. **Answers need a follow-up / 答案要补一次调用** — `converge` without source, `callgraph` without
   source by default, `literal` hits without context: each costs one call plus the deliberation about
   whether to make it.
4. **Prose does not steer / 散文不塑造行为** — 0/4 compliance, while an in-answer pointer worked.

**One measurement artifact, disclosed / 一处度量瑕疵，如实说明**: the briefs capped the report at 60
lines, and **both** arms spent 3–4 blocks trimming to fit. It inflates both chains, so it does not
explain the gap — but it is my brief's cost, not the tools'.
题面把报告限制在 60 行，**两组**都为压行数花了 3–4 块推理。它同时抬高两条链，因此解释不了差距——但那是我的题面成本，
不是工具的成本。

**What this run could not judge / 这次评不到的东西**: the comparison only exercised the *lookup* half.
`check` (run a face and report the exit code and log), `status`+`faces`, `apply`/`plugin` (the write
path), `diff`/`trace`/`unified`/`adopted` were never needed by these four bugs, so "effect on
architecture and code quality" cannot be settled from this run — and those are exactly the capabilities
codegraph does not have.
这次对照只压到**查找**那一半。`check`、`status`+`faces`、`apply`/`plugin`（写入通道）、
`diff`/`trace`/`unified`/`adopted` 在这四道题里都没用上，因此"对架构与代码质量的效果"**这次判不了**——而那些恰好是
codegraph 没有的能力。

## 9. Round 2: did the four fixes move the chain? / 第二轮：那四条修复有没有让链变短

Same project, same four bugs, same device, same ruler; the four fixes from §8 had landed. Fresh trees
under `/tmp/mcp-eval2`; round 1's evidence kept untouched under `/tmp/mcp-eval`.
同一个项目、同样四道题、同一装置、同一把尺子；§8 那四条修复已经落地。新树在 `/tmp/mcp-eval2`，第一轮证据
在 `/tmp/mcp-eval` 原样保留。

| round 题 | ours 我们 | codegraph | who is shorter 谁短 |
| --- | --- | --- | --- |
| 1 (inverted guard) | **8,710** | 14,483 | ours |
| 2 (missing call) | 13,090 | **6,096** | codegraph |
| 3 (boundary) | **3,956** | 13,089 | ours |
| 4 (the trap) | 26,646 | **15,138** | codegraph |
| total 合计 | **53,233** / 56 calls | **49,166** / 44 calls | codegraph |
| round 1 for comparison 第一轮 | 60,059 / 66 | 39,320 / 69 | codegraph |

**The prediction held in part, and the honest headline is that it did not hold overall.**
**预测部分成立；如实的总标题是：整体上不成立。**

- Our own chain fell **11%** (60,059 → 53,233) and round 1 fell **61%** (22,623 → 8,710) — the
  client-building tax is gone: round 1 used 7 instrument calls and the arm never left the tool to
  grep. But the total is still **above** codegraph's, so "shorter than codegraph" is **not** what
  happened.
  我们自己的链降了 **11%**、第 1 题降 **61%**——"自建客户端税"没了（第 1 题 7 次仪器调用、全程没离开工具去
  grep ✓）；但**总量仍高于 codegraph** ⇒"比 codegraph 更短"**没有**发生。
- The split is by **family**, not by tool: we win where the verdict is a line to read against its own
  doc (rounds 1 and 3), codegraph wins where the verdict is a graph property to prove (round 2: a
  missing call edge) or a contract to weigh (round 4: the trap).
  分化按**缺陷族**而不是按工具：判定是"一行代码对着它自己的文档"时我们赢（1、3 题），判定是"要证明的图性质"
  （第 2 题：缺一条调用边）或"要权衡的契约"（第 4 题：陷阱）时 codegraph 赢。
- Codegraph's own total **rose 25%**, and most of that is it buying findings rather than thrashing: in
  round 3 it chased the `no covering tests found` warning to its mechanism (call sites inside macro
  arguments never become caller edges), and in round 4 it built the full truth table plus six reasons
  why the shortest path is wrong.
  codegraph 自己的总量**升了 25%**，而其中大半是**买回了发现**而不是空转：第 3 题它把 "no covering tests
  found" 追到机制（**宏实参里的调用点从不成为 caller 边**），第 4 题它做了完整真值表 + 六条"最短路径为什么
  是错的"理由。

**What the four fixes actually bought, measured / 那四条修复实际买到了什么（有量）**

- `callgraph` bodies by default: **4/4 rounds took the body** and the tool no longer needs
  `source: true`; the auditor records the extra call the old default cost.
  `callgraph` 默认带函数体：**4/4 轮都吃到了**，`source: true` 已成冗余。
- `literal` with `context`: decisive in round 4, where the contract is the verdict —
  `--literal "zero" --context 4` printed the whole bilingual contract in one call, and the round-4
  fix was justified by **two counter-experiments the arm ran and reverted** (only `||`→`&&` leaves
  the target test red at 3 passed; moving the guard to the caller turns **9/9 green while the public
  predicate still lies**, which is the trap).
  `literal` 带 `context`：第 4 题成为主力——`--literal "zero" --context 4` 一次打出整段中英契约；而该轮修复
  由**两次实测反证**支撑（只改 `||`→`&&` 时目标测试仍红、3 passed；把守卫挪到调用方则 **9/9 全绿而公开谓词
  仍在说谎** = 陷阱本体）。
- The in-answer pointers: `search`'s no-match pointer to `literal` was followed, but the `check`
  trailer fired **0/4** — all four rounds ran `cargo test` first and used `check` only after the fix,
  so a pointer attached to a failing `check` never appears. Guidance in the answer only works on the
  path the agent actually takes.
  答案里的指引：`search` 查不到时指向 `literal` 那条被照做了；但 `check` 的尾注 **0/4** 触发——四轮都先跑
  `cargo test`、`check` 只在修后做验证 ⇒ 挂在"失败的 check"上的指引永远不出现。**指引只在 agent 真走的那条
  路上才有用。**

**Two defects of mine that the round measured / 这轮量出来的、我自己造的两处缺陷**

1. The flow table advertised `callgraph {orphans: true}` as a first-class entry while the orphan view
   was deliberately left out. Both arms hit it independently; in round 2 it cost **2 of 10 calls**
   (exit 1 `requires function`, exit 2 `needs a value`) — a self-description defect, the repository's
   own red line. And the pin that was supposed to protect the table only asserted that the *string*
   appears in `INSTRUCTIONS`, never that the *tool* accepts the shape.
   流程表把 `callgraph {orphans: true}` 当一等入口推荐，而孤儿视图被我故意缓做 ⇒ 两位成员独立撞上，第 2 题
   白花 **10 次里的 2 次**；这是自我描述缺陷（本仓红线）。而那条本该守住它的钉子只断言那张表里**出现过这个
   字符串**，从没说**工具接受这个形状**——**钉子太弱，正是它没拦住我的原因**。
2. The client cannot express a bare boolean flag: `--orphans` alone answers `needs a value`, so the
   natural spelling of a boolean is refused.
   一次性客户端表达不了裸布尔开关：`--orphans` 单独出现时报 `needs a value`。

**Audit, round 2 / 审计（第二轮）**

- **Shortcuts: zero, 8/8 clean** — each tree's diff is one place; `tests/` untouched, no
  `#[ignore]`/`#[allow]`, and the two arms' diffs are **byte-identical** per round (the outcomes do
  not differ). All eight fixes restore the injected text verbatim.
  **零捷径，8/8 干净**：每棵树 diff 只一处、`tests/` 未动、无豁免，而两组在同题上的 **diff 逐字节相同**
  （结果层无组间差）；八处都逐字还原注入。
- **Effect: 8/8 restores the judgment's own meaning** (the floor comparison, the totals call, the
  half-open bound, the conjunction). Round 4's *reasoning* differs: ours is the harder experimental
  statement (9/9 green while the contract lies), codegraph's is the fuller argument (its truth table
  names the **uncovered** `(false, 0)` cell, plus six reasons and the `impact` chain).
  **效果：8/8 恢复了判断本身的意思**；第 4 题的**论证**风格不同：我们的更硬（实验证明"全绿而契约说谎"），
  对照组的更全（真值表点出**没有用例覆盖**的 `(false, 0)` 格 + 六条理由 + `impact` 链）。
- **Instruction adherence**: our flow table has seven entries, five were followed; `check` first never
  happened (0/4, see above); `literal` 2/4; query-search 2/4 (round 3 wasted one call behind the
  wrong order); `affected` 0/4. Codegraph followed its own "do not re-read" advice **4/4** (one
  `explore` per round, zero `Read`s) while its `affected` was a false negative **4/4** and its
  coverage warning wrong **4/4**.
  **指令执行度**：流程表七条、照走五条；`check` 先跑 0/4；`literal` 2/4；`query search` 2/4（第 3 题因为
  顺序错白花一次）；`affected` 0/4。对照组把它自己的"别再 Read"建议 **4/4** 照做（每题一次 `explore`、零
  `Read`），而它的 `affected` **4/4** 假阴性、覆盖率告警 **4/4** 误报。
- **Locating directness**: codegraph's locating round count is **≤ ours in 4/4** (one `explore` against
  our 2–3 hops) — a fact worth keeping even in the rounds we "won" on characters.
  **定位直接性**：对照组的定位轮数 **4/4 不大于我们**（一次 `explore` 对我们 2–3 跳）——即使在我们按字符
  数赢的轮次里，这条也成立。
- **Accounting correction (the auditor's, accepted after re-check)**: codegraph's own tables disagree
  with its own log in **3 of 4 rounds** (a claimed `--help` that never ran, cargo counts of 3 against 1
  actual, a 14-group table whose parts sum to 15), so its instrument-call count is **not**
  recomputable: all commands 12/14/12/13 = 51, codegraph-only 10/11/9/10 = **40**. Our side (7/10/6/7
  = 30) matches its per-line log exactly.
  **账目更正（审计的，复核后接受）**：对照组自己的表与自己的日志 **3/4 轮对不上**（声称跑过的 `--help`
  在日志里不存在、cargo 计 3 实 1、14 组的表各部分加总为 15）⇒ 它的仪器调用数**不可复算**：全命令
  12/14/12/13 = 51，**仅 codegraph 10/11/9/10 = 40**。我们这侧（7/10/6/7 = 30）与逐行日志完全吻合。

**Queue from round 2 / 第二轮带出的队列**（the two the round measured are first / 先做量出来的这两条）

1. **Implement the orphan view** (`callgraph {orphans: true}`), or delete the line from the table — the
   round chose implement, because the trap round's `write_totals` is the textbook dead-code case.
   **实现孤儿视图**（或删掉表里那行）——这轮选实现，因为陷阱题里的 `write_totals` 就是教科书级死代码。
2. **Accept a bare `--flag` in the client** (⇒ `true`).
   **客户端接受裸 `--flag`**（⇒ `true`）。
3. **Fix the flow table's order and branches**: a known symbol name goes to `callgraph {function}`
   first (the round measured one wasted call per such round); a symptom that lives in a product ⇒
   search the product's own literal (one call in round 2); a symptom that is an assertion message ⇒
   search a short stable phrase, because the framework appends `left:`/`right:`.
   **修流程表的顺序与分支**：符号名已知 ⇒ 首跳 `callgraph {function}`；症状在产物里 ⇒ 搜产物自己的字面量；
   症状是断言消息 ⇒ 搜短而稳定那半句（panic 会拼上框架的 `left:`/`right:`）。
4. **Return the symbol's doc first line** from `callgraph`/`search {query}` — the one gap the MCP arm
   hit in every round: "the doc says A and the code writes not-A" is the strongest signal this bridge
   can produce, and today it takes two requests to put the two halves side by side.
   **让 `callgraph`/`search {query}` 顺带回该符号的 doc 首行**——这是 MCP 组每轮都撞到的唯一缺口。
5. **Strengthen the pin** that guards the flow table: assert the *tool accepts* each shape the table
   advertises, not that the string appears.
   **加强那条守住流程表的钉子**：断言"工具接受表里承诺的每个形状"，而不是"字符串出现过"。

## 10. Scenario levels: the shapes a one-line bug never reaches / 情景关卡：一行缺陷够不到的形状

The maintainer asked for more items, realistic project shapes, and for out-performing codegraph rather
than matching it. The four injected bugs measure localization speed on a known shape; these six
measure whether a tool can **answer at all** when the question needs a framework notion or
cross-member reachability. `tools/nichlink-mcp-eval` gained `scenario-project` / `scenario-inject` /
`scenario-plan` / `scenario-check` / `scenario-probe`; the project is a two-crate workspace
(`ledger-core` + `ledger-report`) with a feature-gated module, a cross-crate caller, a zero-caller
definition, a two-site chain and a contract whose one uncovered cell no test exercises.
维护者要更多测试项、真实项目形状，并要求**超越**而不是打平。四个注入缺陷量的是"已知形状上的定位速度"；
这六关量的是当问题需要一个**框架概念**或**跨成员可达性**时，一件工具**能不能作答**。项目是两 crate 工作区
（`ledger-core` + `ledger-report`），带一个特性门控模块、一个跨 crate 调用者、一个零调用者定义、一条两处
站点的链，以及一条"有一格没有任何用例覆盖"的契约。

**Every scenario is mechanically self-proved** (`scenario-check`): on the real tree, the default face is
green wherever the scenario says it should be, and the feature face is red for exactly one scenario —
the one whose whole point is that the red is invisible on the default face.
**每一关都机械自证**（`scenario-check`）：在真树上，该绿的默认面绿，而**恰好只有一关**的特性面红——那一关的
全部要点正是"红在默认面上看不见"。

| # | scenario 关卡 | ours 我们 | codegraph | verdict 判 |
| --- | --- | --- | --- | --- |
| 1 | cross-crate caller 跨 crate 调用者 | `callers (1) … report.rs::store` ✓ 1 call | names the same caller ✓ 1 call | tie 平手 |
| 2 | which face is red 红在哪个面 | `faces … default=[] all=[audit]` + the blindness note, then `check --face audit` ✓ 2 calls | **no notion of a face** ✗ | **ours, structurally** ✓ |
| 3 | orphan view 孤儿视图 | `orphans 1 … audit_unused -> core/src/audit.rs:14` ✓ 1 call | `No callers found for "audit_unused"` ✓ but only **when the name is already known** ✗ | ours has the view ✓ |
| 4 | change impact 改动影响（该跑哪些测试） | `tests: core/tests/audit.rs` ✗ **misses `report/tests/report.rs`** | `Affected test files (2): …audit.rs, …report.rs` ✓ | **codegraph wins** ✗✗ |
| 5 | two-site chain 两处站点 | round-1/2 families, both sites reachable | same | measured in §8–§9 |
| 6 | uncovered contract cell 未覆盖的契约格 | the doc's own conjunction, read in one call | the same text, but with no doc-vs-code view | ours on adjacency |

**The one that matters is #4, and it is ours to fix.** At a virtual workspace root our `affected`
reports only the **owning member's** tests: a change to `crates/core/src/model.rs` names
`crates/core/tests/audit.rs` and never `crates/report/tests/report.rs`, although the report suite
reaches that definition through `Store::post`. Codegraph answered that same question correctly on the
same tree. Four single-bug rounds never touched this path; a scenario did.
**要害在第 4 关，而且该我们修。** 在虚拟工作区根上，我们的 `affected` 只报**拥有者成员**的测试：改
`crates/core/src/model.rs` 时它点名 `crates/core/tests/audit.rs`，从不提
`crates/report/tests/report.rs`，尽管报表套件经 `Store::post` 抵达那个定义。codegraph 在同一棵树上答对了。
四个单点缺陷轮次从未碰到这条路径，一个情景碰到了。

**Honest limits of this table / 这张表的如实边界**: #1 is a tie because the fixture's cross-crate call is
method-style (`store.post(...)`), which is *not* the fully-qualified spelling codegraph's recorded
false negative is about — sharpening the fixture with a `ledger_core::store::Store::new()` call is
queued rather than done, so #1 currently over-states nothing but proves less than it could. #5 and #6
re-run families already measured in §8–§9, so they add coverage rather than new evidence.
**#1 之所以是平手**：夹具里的跨 crate 调用是方法式（`store.post(...)`），而 codegraph 记录在案的假阴性针对的
是**全限定**拼法 ⇒ 用一条 `ledger_core::store::Store::new()` 把夹具磨尖这件事排进队列而**没有现在做**，因此
#1 既没有夸大、也比它能证明的要少。#5/#6 重跑的是 §8–§9 已量过的族，属补覆盖而不是新证据。

**Round 3's first data points / 第三轮的头两个数据点**: r2 took **5 calls with zero refusals** (the
predicted disappearance of the two refusals held), while r1 took 11 with **three** — and the three are a
**new** family the queue did not know about: `affected`'s array argument can only be spelled through
`--json`, and the refusal (`requires files, an array`) does not point at it; separately, `search`'s
empty answer does not say **which root it searched**, which cost two calls when the caller's cwd was the
checkout rather than the tree.
**第三轮头两个数据点**：r2 **5 次调用、零拒绝**（预测的"2 次被拒消失"成立 ✓），而 r1 用了 11 次、其中
**3 次被拒** —— 这 3 次属于队列**不知道的新族**：`affected` 的数组参数只能经 `--json` 拼出，而拒绝文案
（`requires files, an array`）不指向它；另外 `search` 的空答案**不说它在哪个根上搜的**，当调用方的 cwd
是检出而不是题树时，这一条白花了 2 次。

**Queue from the scenario levels / 情景关卡带出的队列**（#4 first / 第 4 条优先）

1. **`affected` must follow cross-member reachability** at a workspace root — it is the one scenario we
   lost, and codegraph answers it.
   **`affected` 必须追跨成员可达性**——这是唯一输掉的一关，而 codegraph 答对了。
2. **Array arguments** should be spellable without `--json` (a repeated flag is the natural shape) and
   the refusal must name `--json` until they are.
   **数组参数**应能不经 `--json` 拼出（重复开关是自然形状），在做到之前，拒绝文案必须点名 `--json`。
3. **Every answer should carry the root it answered from** — the empty answer is where it matters most,
   because "no matches" is exactly the moment a caller wonders whether it is asking the right tree.
   **每个答案都应带上它作答的根**——空答案处最要紧，因为 "no matches" 正是调用方怀疑自己问错树的时刻。
4. **Sharpen scenario 1** with a fully-qualified cross-crate call, so the recorded codegraph false
   negative is reproduced rather than assumed.
   **磨尖第 1 关**：加一条全限定的跨 crate 调用，让 codegraph 记录在案的假阴性被**复现**而不是被假定。

## 11. Round 3: what the fixes bought, measured / 第三轮：那批修复买到了什么（有量）

Same twelve-action brief as round 2, byte for byte, on fresh trees under `/tmp/mcp-eval3`; the tree
blobs are identical to round 2's (the auditor checked `ls-tree` blob-for-blob), so the ruler did not
move while the tool did.
题面与第二轮**逐字相同**，新树在 `/tmp/mcp-eval3`；审计核过两轮的树 `ls-tree` **blob 逐一相同** ⇒ 尺子
没动，动的是工具。

| metric 度量 | round 2 | round 3 | reading 读法 |
| --- | --- | --- | --- |
| reasoning chars 思维链 | 53,233 (ours) / 49,166 | **97,142 / 77,012** | both up; see below 两轮都涨 |
| instrument calls 自家仪器调用 | 30 / 47 | **24 / 24** | ours −20%, theirs −49%; tie at 24 打平 |
| refusals 被拒 | 2 (ours) / 0 | **3 / 0** | new family, same side 新族、仍在我们侧 |
| preset-path fit 预设路径拟合 | 5/5 × 4 | 5/5 × 3, 4/5 × 1 | unchanged 基本不变 |
| repairs 修复 | 8/8 one line, byte-identical per pair | same | outcomes never differed 结果层从无组间差 |

**What the flow table actually did / 流程表实际起了什么作用.** The auditor's finding is the one worth
keeping: rounds 2, 3 and 4 followed the rewritten table **in order, with zero detours and zero
refusals** — round 2 went to the product's own literal (branch ②), round 3 correctly declined branch ③
because its assertion carries no custom message, and round 4 used the hop-by-hop route with the doc
riding along as the only deciding evidence. Round 1's deviation was **not** the order: it was two
**blanks** in the client — the answer does not say which root it searched, and an array argument can
only be spelled through `--json` while the refusal does not say so.
**审计的这条发现最值得留**：第 2/3/4 题**严格按重写后的表逐条走、零弯路零拒绝**——第 2 题走②（产物自己的
字面量）、第 3 题正确**弃用**③（它的断言没有自定义话术）、第 4 题走逐跳且 **doc 随行成为唯一裁决证据**。
第 1 题的偏离**不在顺序**，而在客户端的**两处空白**：答案不说它在哪个根上搜，以及数组参数只能经 `--json`
而拒绝文案不说明。

**The doc adjacency is the decisive capability, and it is measurable / doc 随行是决定性能力，而且可量.**
The auditor verified that both arms' round-4 reports quote the contract, and that **both got it in one
call** (ours from `callgraph`'s doc-plus-source, codegraph from a single `explore` dump). Cross-round,
that is a change: in round 2 the MCP arm needed a **second request** for the doc in 4 of 4 rounds
(round 4 spent two `search --literal zero` calls on it), while round 3's round 4 used **zero** `search`
calls and went 7 → 5 on `--call`. The arm's own priority call therefore stands, and the queue follows
it: **do not spend the next batch on "`callees` should carry positions" (saves one call) before the doc
stays where it is (saves two calls and closes a judgement gap no test can close).**
**审计核过**：两组第 4 题的报告**都真的引用契约**，而且**都是一次调用同屏**（我们来自 `callgraph` 的
doc+源码、它来自一次 `explore` dump）。跨轮看这是个**变化**：第二轮我们那组 **4/4 都要第二次请求**才拿到
doc（第 4 题为此花了两次 `search --literal zero`），而第三轮第 4 题 **`search` 归零**、`--call` 7→5。
⇒ 成员的优先级判断成立，队列照它排：**下一批别先做"`callees` 附位置"（省 1 次），doc 留在原处更值
（省 2 次 + 关掉一个测试关不上的判断缺口）**。

**What is proven and what is not / 什么被证明了、什么没有.** The orphan view was implemented and works
(scenario S3 calls it and gets `orphans 1 … audit_unused`, one call), but in these four rounds it was
called **zero** times: round 2 reached the same "no caller" conclusion through `search`'s zero
references instead. So the prediction "the missing-edge family flips" is **untested here** — the source
shape is implemented and the scenario exercises it, while the bug rounds took another route.
**孤儿视图已实现且可用**（情景 S3 调它并一次拿到 `orphans 1 … audit_unused`），但这四轮里它被调用
**零**次：第 2 题改用 `search` 的零引用达到同一结论 ⇒"证明缺边族翻转"这条预测**在此未被检验**（源码形状
已实现、情景关卡也在用，只是这四轮走了另一条路）.

**Why the chains grew, with the causes I can name / 链为什么变长（可点名的成因）**: round 1 carried the
two client blanks above (2 calls lost to the root, 3 refusals on `affected`'s array argument); every
answer now carries the doc and the body, so fewer calls buy more reading; and codegraph's round 4 spent
46,824 characters — its largest of any round — **buying the finding that `||` → `^` is the only
single-token replacement that turns the suite green while still lying on the contract's uncovered
cell**. With one run per cell, no claim about the batch's net effect on the chain is warranted; what is
warranted is the call-count reduction and the three named causes.
**第 1 题带着上面那两处客户端空白**（根丢 2 次、`affected` 数组参数被拒 3 次）；**每个答案现在都带 doc
与函数体** ⇒ 调用更少、要读的更多；而 codegraph 的第 4 题花了 46,824 字符（它所有轮次里最长）——
**买回了"`||`→`^` 是唯一能让套件全绿、却在契约未覆盖那一格上说谎的单 token 替换"这条发现**。n=1/格，
因此不对"这批对链长的净效果"下任何结论；成立的是**调用数下降**与那三条可点名的成因。

**The round-4 trap, judged by the contract rather than by the suite / 第 4 题：按契约判，不按套件判**:
the auditor hand-checked the four-cell truth table independently of the captain's probe and confirmed
that `^` differs from the contract **only** on `(has_receipt=false, amount=0)`. Both arms quoted the
contract verbatim and neither touched `Store::post`; the same-round repair blobs are byte-identical.
**审计独立手算**了四格真值表（与队长的探针互相印证）：`^` **只在** `(has_receipt=false, amount=0)`
上与契约不同。两组都逐字引用契约、都没动 `Store::post`，同题修复 blob 逐字节相同。

**Queue, in the order the evidence now supports / 队列（按证据排）**

1. **Answer must carry the root it searched** (2 calls lost in round 1; the empty answer is where it
   matters most).
   **答案必须带上它搜索的根**（第 1 题丢 2 次；空答案处最要紧）。
2. **Array arguments without `--json`**, and until then the refusal must name `--json` (3 refusals).
   **数组参数可不经 `--json`**；在做到前拒绝文案必须点名它（被拒 3 次）。
3. **Keep the doc adjacency where it is**; do **not** trade it for `callees` positions.
   **保持 doc 随行**；不要为 `callees` 的位置把它换掉。
4. **`affected` must follow cross-member reachability** (scenario S4, the one level we lost while
   codegraph answered it).
   **`affected` 必须追跨成员可达性**（情景 S4，唯一输掉的一关）。
5. **Exercise the orphan view in a bug round** — the four rounds never called it, so its behavioural
   claim rests on scenario S3 alone.
   **让孤儿视图在缺陷轮次里被真的用到**——这四轮一次都没调，它的行为结论目前只靠情景 S3。
6. **Make round logs raw again**: round 2's per-call JSON-RPC frames allowed counts to be recomputed
   from the artifacts; round 3's summary lines did not, which is why the auditor could only give call
   counts as proxies for that comparison.
   **把轮次日志恢复成原始形态**：第二轮逐次 JSON-RPC 帧可以从产物复算计数，第三轮的摘要行不行——这正是
   审计只能拿调用数当代理量的原因。

## 12. Round 4 + the prompt families: what the numbers say / 第四轮与五族：数字说了什么

Round 4 ran the same twelve subjects as round 3 (four injected defects + eight scenario levels) on trees
regenerated by the **fixed** generator, plus five new prompt families whose brief is **one sentence from
the user and nothing else**.
第四轮把与第三轮相同的十二个样本（四道注入缺陷 + 八个情景关卡）跑在**修复后的出题器**重生的树上，并加了五个
新族——题面**只有用户那一句话**。

| axis 轴 | ours 我们 | codegraph | reading 读法 |
| --- | --- | --- | --- |
| four injected defects 四题 | 8/8 verbatim restorations 逐字还原 | 8/8 | **tie on outcome** 结果打平 |
| eight scenario levels 八关 | 15 correct / 1 partial / 0 wrong | 15 / 1 / 0 | partial: we listed one orphan and missed the five `#[test]` half |
| tags (capability) 标签（能力） | 真赢 22 · 输 2 · 不具鉴别力 0 | | the two losses are codegraph's: it has no notion of a **face** and no **orphan view** |
| instrument calls 仪器调用 | **93** (1 refused) | **302** | we are 3.2× fewer, **with two confounds**: my read-only brief forced it to copy+index+verify each question, and it built probes/mutations |
| total tool calls 总调用 | 339 · ≈**303,331** tokens | 374 · ≈338,397 | **≈0.90×** on tokens; but **subject by subject it wins some** (r1/r2/r4) |
| families 五族 | a 19 · b 9 · c 15 · d 12 · e 14 = **69** | 40 · 31 · 33 · 29 · 29 = **162** | a/c/d/e tie-leaning-ours; **b we win** (it declared `parts:` and red a pinned test) |

**Repetition is productive, and the one waste has a single source / 反复是产出性的，而唯一的浪费只有一个来源**:

| group 组 | calls | identical command ✗ | same tool, new target ✓ | retry after refusal ✗ |
| --- | --- | --- | --- | --- |
| families (ours) | 69 | **0** | 58 | **6** (all in family a's `apply` shape probing) |
| round 4 (ours) | 93 | **0** | 68 | 1 (`affected`'s `paths`) |

So the chain is not looping on itself; the information-caused part is **refusal-driven**, and that is
exactly the family today's `--list` and `--list <tool>` address.
因此链没有自我打转；信息导致的那部分**全部来自被拒重试**，而那正是今天的 `--list` 与 `--list <tool>` 针对的那族。

**The structural gaps we closed / 我们今天关掉的结构缺口** (each measured, each with a real-tree smoke):
`--list` names each tool's keys ✓ · `--list <tool>` prints its whole description ✓ · `callees` carry the
definition line of every tree-defined callee ✓ · `callgraph` adds `also here` for the file's other
definitions ✓ · **`check` now ends with a whole-tree census** (respelled constants, unreferenced
constants, the entry plan's static counts, and a sentence saying what it does **not** cover) ✓ · the
flow table names the adoption ledger and where it lives ✓ · a zero-caller answer at a member root says
the other members are invisible ✓ · the example's Studio test no longer deletes the whole `.nichlink`
(it now removes only the plan it created) ✓.
**每一条都有实测来源与真树冒烟** ✓；最后一条由五族的 c **两个臂独立发现** ✓（我们那臂只能把台账写在最后一次
跑门之后 ✓，它那臂按原 sha256 逐字节还原后追加 ✓）。

**Honest limits / 诚实的限制**：n=1 per arm ✗; tokens are estimated at characters/3.5 ✗; the round is an
aggregate of 17 subjects ✗ (per-subject attribution is by the tree path in each call's arguments ✓);
and the confounds above mean the 93:302 gap is **not** by itself proof of a better tool ✗.

## 13. 第五轮：题面只剩一句话，以及装置自己错在哪 / Round 5: one-sentence briefs, and what the fixture got wrong

**装置**：26 棵树在 `/tmp/round5` —— 四道注入缺陷 `r1`–`r4`（每臂一棵）、八个情景关卡 `s1`–`s8`（两臂共用只读树）、
五族 `a`–`e`（每臂一棵），**每臂 17 个样本**。题面与题面级硬约束在 `/tmp/round5/BRIEF.md`，判分口径在
`/tmp/round5/AUDIT-BRIEF.md`；`pristine-1..4` 由出题器新生成，专供四题的逐字还原。
队 `nichlink-round5` 共 **27 个任务**：原计划 35 个，被 `agent_teams_create` 的 inline plan 上限（**32 条**）压成 27
——八关两两合并成每臂 4 条，逐样本的日志与交付文件仍然分开。**上一轮会话就是死在哪一步**：35 条任务的完整题面
一次塞进工具调用（约 100 KB），模型吐出不合法 JSON（`MALFORMED_RESPONSE`），队没建起来。

**判分（机械，队长独立复核）**：四题 **8/8 逐字还原** ✓（两臂各自的工作树与 `pristine-N` 逐文件相同；每树只一个源文件
改动、diff 恰好一处；`tests/` 未动、无 `#[ignore]`/`#[allow]`；八棵树交付态 `cargo test --offline` 9 passed、rc=0）。
八关：`s1`/`s2`/`s4`/`s5`/`s7`/`s8` **两臂全对** ✓；**`s3` 两臂同为部分对**（都只答 `audit_unused`，缺 5 个 `#[test]`
——两臂所用工具共同的"默认排测试文件"边界）；**`s6` 分岔**：codegraph 与队长实测的真值表逐格一致，mcp 对预设判对，
但含**两处与源码矛盾的事实断言**（称 `(10,false)` 的实现值为真、称 `(0,true)` 与契约相同）。五族 10 棵树默认面与
`--all-features` 全绿 ✓。

**`r4` 陷阱题两臂都通过**：mcp 拒绝"在 `Store::post` 里加零额守卫"并给三条理由（契约明写判定责任在 `postable`、
调用点守卫只骗得过测试、它只治零额那一半）；codegraph 更进一步，在树外副本里把那条捷径**跑了一遍**
（9 条测试全绿 + 契约探针红）再拒绝。

**调用与 token（队长用独立脚本逐字符复算，与审计数字一致）**：

| 轴 | mcp 臂 | codegraph 臂 |
| --- | --- | --- |
| 仪器调用 | **154**（桥；被拒 **8**） | **116**（codegraph；仪器 0 被拒） |
| 其它工具 | 278（cargo 71 / shell 探查 66 / python 9 / 非 bash 132） | 75（cargo 71 / python3 4） |
| 留档 | 154 行 JSON | 191 个 `=== CMD:` = 仪器 116 + 其它 75 |
| 推理字符 | 162,497 | 519,099 |
| 答复字符 | 21,519 | 18,643 |
| 工具输出字符 | 447,053 | 578,613 |
| 题面字符 | 157,967 | 149,092 |
| ≈token（字符/3.5，**估算**） | **225.4k** | **361.6k** |

⇒ 本轮 **mcp ≈0.62×**（第四轮是 0.90×）。差距主要来自五族的 `a`/`b`（`a` 89.5k vs 40.3k、`b` 70.9k vs 40.3k，
是 codegraph 用 16/48 次 cargo 迭代换来的）。**八关逐题仍互有胜负**：`s1`–`s4` 我们明显轻（`s3` 一次
`callgraph {orphans}` 收工 = 9 次调用，vs 它逐符号手查 21 次），`s7`/`s8` 它略轻。口径提醒：**别拿它的
"全命令 191"去比我们的"仅仪器 154"**。

**标签**：mcp 真赢 0 / 不具鉴别力 15 / 输 2（`fb`、`fc`）；codegraph 真赢 2（`fb`、`fc`）/ 不具鉴别力 15 / 输 0。
**但这个分类有一处口径缺陷必须如实写**：本轮我给的标签集合只有"真赢 / 不具鉴别力 / 输"，**没有"平手"这一档**，
审计于是把"两臂同判"塞进了"不具鉴别力"；而按本轮预先登记的字面定义（"答案不依赖树内证据，一条 grep 就能定"），
`r1`–`r4` 这种"先跑测试拿到失败用例名、再读文档与代码那一处对照"的题**不该**算不具鉴别力。
⇒ **15/17 不能与第四轮的"不具鉴别力 0"直接对比**（两轮用的是两个定义）；按更严的读法，本轮 17 题里大约只有
`s3` 是真正"一条 grep 即定"。

**两条预登记复测**：
1. **被拒没有归零** ✗：mcp 8 次（`fa` 5、`fb` 1、`s4` 1、`s6` 1），codegraph 仪器 0 次。而且
   **`--list` / `--list <tool>` 实际调用 0 次** ✗（`s4` 日志里那处 `--list` 是错误信息自带的用法行）
   ⇒ 本轮新增的"键名 / 完整描述"兜底**从未被读过**，"`s4` 的拒绝由描述不足造成"这个归因因此**没有实测依据**；
   `fa` 的 5 次仍是 `apply` 形状（4 次 schema/取值 + 1 次父规则语义拒绝），与第四轮同族。
2. **被迫回头降了** ✓：mcp 相邻仪器调用对 `callgraph → callgraph` **16 → 10**、`callgraph → read` **10 → 6**
   （同一条尺子）；新的高频对是 `search → search` 11、`check → callgraph` 9。

**装置缺陷（本轮抓到 7 条，每条都有实测）**：
- **F1 `s3` 的预设与工具边界**：预设真值含 5 个 `#[test]`，而孤儿视图默认排测试文件 ⇒ 两臂被同一限制封顶在
  "部分对"。要么预设显式声明排除面，要么把问题限定为 `src`。
- **F2 `s6` 预设不精确**：「未覆盖的一格 = `(has_receipt=false, amount=0)`」为真但不全——未覆盖类是**零金额分录**
  （含 `(0,true)`）。已在轮中预先登记、**未临场改预设**。
- **F3 `fc` 的意图措辞预设了实现分支**：「必须处理采信台账（**续期/新增记录**）」把"作出决定并留下可核证据"写成了
  "写一行"。mcp 零改动 + 论证（满足用意、不满足字面），codegraph 满足字面。
- **F4 `fa`/`fb` 的两条硬约束互斥**：§4「不改测试」与"对象/深化真的上线 + 门绿"在现有钉子上**不可兼得**——
  mcp 两族都保住树但没上线，codegraph 两族都上线但改了 `tests/registry.rs`（`fb` 里还把
  `assert!(!cut.is_full())` 反成 `assert!(cut.is_full())`）、`tests/static_plan_allocations.rs`、
  `examples/graft_record.rs`。
- **F5 `fc` 的 codegraph 新行是"代笔的人确认"**：`verifier = nich (maintainer, requested through the round-5 agent)`，
  且 `at = 2026-09-30T13:41:53Z` **早于**既有行的 `17:00:00Z`（追加在后、时间在前）。内核只校 8 段非空与指纹，
  故**合法但语义可疑**。
- **F6 "工具给的范围"与关卡预设并不等价**：mcp 的 `affected` 给文件级并集、codegraph 给 3 个测试文件，
  两臂都另用符号级判据收窄到 `crates/report/tests/report.rs`；判分按预设取最小必跑集。
- **F7（队长自己的错）标签集合少了"平手"**：见上。

**队长独立复核用的手段**（都不重跑成员的脚本）：8 组 `diff -r … pristine-N`；从 `log-mcp-*.jsonl` 重算相邻调用对；
从两臂会话日志逐字符重算四类字符（`assistant/message` 的 reasoning/text、`tool/result` 的 `message.content`、
`user/message`）；对 `fc` 的 codegraph 树直接跑桥的 `adopted` 读出台账（`provisional 2 lapsed 0` ⇒ 它**手算的指纹
是对的**）；在 `git show HEAD:` 的版本上单跑 rustfmt，证明 `crates/report/src/buckets.rs` 与
`crates/report/tests/buckets.rs` 里的格式差异是**夹具自带**（不是选手引入）；在树外副本上原型验证 `fb` 的第四条路
（`/tmp/fb-lead-check`：给 `Button` 加内部 `PartsContract` 零件层、不写进声明 ⇒ `cargo test` rc=0、25+1 passed 与基线同）。

**诚实的限制**：n=1/臂 ✗；token 是字符/3.5 的**估算** ✗（附 harness 自报 `outputTokens` 可交叉核对）；
配对成一条任务的两题共享开销落在第一题 ✗；`fb` 的 parts 路线只有队长的原型、审计未自行复现 ✗。
