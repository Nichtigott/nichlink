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
