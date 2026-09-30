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
