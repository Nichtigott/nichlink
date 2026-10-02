# cg · `h4-one-file-many-algorithms`

**题面**：hardbug h4：单文件多算法——一个文件三套算法，恰一支写反、恰一支不可达。

> 步 3 · 仪器调用 **9**（逐题日志条数）· 会话里定位到 5 条 · 输出 4,699 tok · 推理 11,300 字符 · 累计上下文 466,688

## 一、工具给了什么 · 当时 agent 在想什么（逐条配对）

> ⚠️ 日志 9 条 vs 会话定位 5 条：差的那些调用发生在**共享步**（一条命令里跑了多道题）或**装置步**里 —— 这正是"逐题 token 不可归因"的具体形状。

### ①1 日志第 1 条 · exit 0 · step 40（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph init .`

**工具返回**（逐字）：
```
┌  Initializing CodeGraph
│
◆  Initialized in /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms
│
Scanning files...
Parsing code...
Resolving refs...
Linking dynamic dispatch...
│
◆  Indexed 4 files
│
●  16 nodes, 19 edges in 499ms
│
└  Done
```

**该步推理**（逐字）：
```
10 of 15 done (s1-s8, g1, g2). Now:
- g4 (host tree with crates, "哪些臂是没有任何执行能进入的")
- h1-supply-chain, h2-claim-unkept, h3-cross-file-chain (host+graft)
- h4-one-file-many-algorithms (tiny: 4 files)

Let me start with h4 (smallest): tree with src/lib.rs, src/model/entry.rs, src/model/model.rs, tests/postable.rs. Question: one algorithm in one file does the opposite of its docs, and no test covers that branch.

Let me read the tree via codegraph. Also h1-supply-chain/h2/h3 have the same host/graft skeleton with different injections.

Plan for h4: node the 4 files (4 calls) + maybe callees.

Let me also plan the three h-classes: each has host/ (10 object files + lib.rs + control.rs + registry_rule.rs + tests/offsets.rs or shipped.rs) and graft/ (10 files + lib.rs). The classes:
- h1-supply-chain: "one sibling calls a different family of names than its siblings do" → the symptom: offsets test fails with a total no single widget explains. The defect: one of the host object files (or graft) calls a different family of names (e.g. calls `gauge_fast::…` vs `…::gauge_slow`, or uses the wrong trait method). Let me find it: compare the 10 object files for a family difference, e.g. one file calls `X_fast::new()` while others call something else. Since they're similar, I can look for the odd one out via codegraph (callees of each object) or by reading.
- h2-claim-unkept: "a real specimen ledger entry, one sibling missing exactly one declaration, one lapsed entry" → in `host/.nichlink/adopted/entries` (a file listing adopted entries). One sibling file missing exactly one declaration (e.g. missing a `claim!`/`declare!` line), and one ledger entry whose fingerprint no longer matches. Need to read the entries file + the object files.
- h3-cross-file-chain: "the entry plan never names the new face, so the build-time scope leaves it out of the build" → the entry plan (in host/src/lib.rs or build.rs) doesn't name one face → missing from shipped registry. Compare object list vs entry plan.
- h4: as above.

These need real reading. Let me do them one at a time, starting with h4 (cheap), then h1-supply-chain, h3, h2.

Then g4 (host tree with crates/, "哪些臂是没有任何执行能进入的" — arms that no execution can enter). g4's tree: crates/... let me look at its layout (14 files, crates/). It's probably a nichlink host with crates.

Let me start h4.

For each question I should log codegraph calls into logs/<id>.txt. Note: h4's init is already logged ✓.

h4: 4 files. Let me node all 4.
```

**该步模型输出**（逐字）：
```
Now the hardbug class trees. Starting with **h4** (smallest):
```

### ①2 日志第 2 条 · exit 0 · step 41（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph files`

**工具返回**（逐字）：
```
Project Structure (4 files):

├── src
│   ├── model
│   │   ├── entry.rs (rust, 10 symbols)
│   │   └── model.rs (rust, 1 symbols)
│   └── lib.rs (rust, 1 symbols)
└── tests
    └── postable.rs (rust, 4 symbols)
```

**该步推理**（逐字）：
```
h4 analysis:

Three algorithms in one file (src/model/entry.rs): 
1. `zero_arm()` (line 30) — an arm chooser: returns `ZeroArm::Post` always.
2. `postable()` (line 38) — contract: "postable when it carries a receipt, and a zero entry never is". The implementation returns true for any receipted entry because `zero_arm()` returns `Post` unconditionally — so `ZeroArm::Refuse` branch at line 44 is DEAD (never taken), and the contract "a zero entry never is [postable]" is violated: `Entry::new("a", 0, true).postable()` → has_receipt true, zero_arm → Post → **true** (should be false).
3. `signed()` (line 53) — magnitude, always non-negative; matches its doc.

So: exactly one branch inverted = the `zero_arm()` should decide by amount: `if self.amount == 0 { ZeroArm::Refuse } else { ZeroArm::Post }`. Let me check the class statement: "three algorithms in one file, exactly one branch inverted and exactly one branch uncovered". 

The `zero_arm()` function: its doc says "Which arm decides a zero amount" — but it ignores `amount` entirely and always returns `Post`. So the inverted/omitted logic is there: it should return `Refuse` when amount == 0.

Alternatively, the "inverted branch" could be in `signed()`: `if self.amount < 0 { -self.amount }`: for i64::MIN, `-i64::MIN` overflows — but that's not "inverted". The doc says "Always non-negative: a debit prints as its size, never with a sign" — implementation matches. Hmm, wait: is `signed` inverted? For amount = -5 → 5 ✓ non-negative. OK matches.

And `normalized_account`: doc "Accounts are case-insensitive and trimmed" → `trim().to_lowercase()` ✓ matches.

So the defect: `zero_arm()` (entry.rs:30-32) returns `ZeroArm::Post` unconditionally → the `ZeroArm::Refuse` arm (entry.rs:44) is unreachable (no execution can enter it — this is exactly h4's "one branch uncovered") and `postable()` contradicts its own doc for zero-amount entries (entry.rs:36-37 + 42-45). The tests cover only receipted non-zero (post) and unreceipted (refuse early) → the zero case and the Refuse arm are uncovered ✓ "exactly one branch uncovered".

Minimal fix: `fn zero_arm(&self) -> ZeroArm { if self.amount == 0 { ZeroArm::Refuse } else { ZeroArm::Post } }`. Files not touched: tests, model.rs, lib.rs, Cargo.toml.

Evidence via codegraph: `node src/model/entry.rs` (source at lines 30, 42-45), `callers zero_arm` (1 caller: postable), `callees postable`/`callees zero_arm`. Let me run `callers zero_arm` and `callees postable` for evidence. Also `callees an_entry_without_a_receipt_does_not_post` to show the tests' coverage (only 2 tests, neither passes amount 0).

Let me do that, then write the answer.

Also g4's question ("哪些臂是没有任何执行能进入的") is about arms unreachable by any execution — for the g4 tree. Let me handle g4 next; its tree has crates/ (14 files).

Let me run h4 evidence now + look at g4's layout/entry point.
```

### ①3 日志第 3 条 · exit 0 · step 63（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph node src/lib.rs`

**工具返回**（逐字）：
```
**src/lib.rs** — 5 lines, 0 symbols · no other indexed file depends on it

1	//! A ledger model.
2	
3	#[path = "model/model.rs"]
4	pub mod model;
5
```

**该步推理**（逐字）：
```
Excellent — decisive runtime evidence for h4:
- The compiler itself warns: `variant 'Refuse' is never constructed` at entry.rs:9 — that's the "one branch uncovered/unreachable" confirmed by rustc ✓✓. 
- Runtime: `zero+receipt postable = true` (contract says false) ✗; `zero-no-receipt postable = false` (early return on !has_receipt) — so the Refuse arm is unreachable for another reason too (the receipt check precedes); `five+receipt = true` ✓; `five-no-receipt = false` ✓; `signed(-5) = 5` ✓; `normalized(" A ") = "a"` ✓ (matches doc).
- The tree's own tests: 2 passed (only the two non-zero cases) ✓ — green, so the defect is invisible to the tests.

Now write h4's answer.

Key content:
① 症状：`postable()` 对自己文档里那条契约说谎（零金额带凭据的分录被判为可入账），而测试全绿。
② root cause: `src/model/entry.rs:30-32` `fn zero_arm(&self) -> ZeroArm { ZeroArm::Post }` — 它完全无视 `self.amount`，恒返回 `Post`；于是 `entry.rs:44` 的 `ZeroArm::Refuse` 臂永远进不去（rustc 自己报 `variant 'Refuse' is never constructed`），而 `entry.rs:36-37` 的契约（"a zero entry never is"）在 `entry.rs:38-46` 被违反。
mechanism: postable() = `if !has_receipt { false }` else match zero_arm() { Post => true, Refuse => false } ⇒ 任何带凭据的分录都 true，包括 amount == 0；真正决定"零金额"的那支被写反/写没了。
③ fix: `zero_arm()` 改为 `if self.amount == 0 { ZeroArm::Refuse } else { ZeroArm::Post }`。不动 tests/postable.rs、src/model/model.rs、src/lib.rs。
④ counter-proof: 独立调用方（scratch-h4，只依赖该树）实跑：`zero+receipt postable = true`（契约要求 false）、`five+receipt = true`、`five-no-receipt = false`、`signed(-5) = 5`、`normalized(" A ") = "a"`；若 `zero_arm` 正确，第一行必为 false。并且 rustc 的 dead-code 警告点名 `Refuse` 从未被构造 —— 这正是"恰有一支没被覆盖"。树的测试 `2 passed`（只覆盖 5+receipt / 5-no-receipt），所以绿不等于对。
   evidence (commands + raw output + exit): node entry.rs (rc=0); callers zero_arm → 1; callees postable → zero_arm; cargo test → 2 passed rc=0; scratch run → printed values rc=0 + compiler warning at entry.rs:9.
⑤ 调用清单: files; node src/lib.rs, src/model/model.rs, src/model/entry.rs, tests/postable.rs; callers zero_arm; callees postable; callees a_receipted_entry_posts (0 — 工具漏了链式调用，所以回源码读)；非 codegraph: cargo test + scratch 运行.

Let me write it.

After that: remaining deliverables: SETUP.md, BATCHING.md, PARTIAL.md, plus final task update + report. Also I should double check the answers' line counts (≤20). Let me check with wc -l after writing.

Also: for the s-questions, should I have run cargo? Not necessary (the trees' tests aren't the question). The 15 questions: 10 done (s1-s8, g1, g2) + g4 + 4 hardbug = 15 ✓ (11 files).

Let me write h4 and then finalize.
```

### ①4 日志第 4 条 · exit 0 · step 67（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph node src/model/model.rs`

**工具返回**（逐字）：
```
**src/model/model.rs** — 3 lines, 0 symbols · no other indexed file depends on it

1	#[path = "entry.rs"]
2	pub mod entry;
3
```

**该步推理**（逐字）：
```
I have what I need. Let me write SETUP.md now — comprehensive but readable.

Content plan:

# SETUP — codegraph 臂 (cg26) 装置、索引与范围验证

## 1. 工具
- `/home/nich/Moirai_N3/nichlink/target/round7/tools-upstream/v1.6.1/bin/codegraph`；`--version` → `1.6.1`；`--help` 列出的子命令: init/index/sync/status/query/explore/context/node/files/callers/callees/impact/affected...
- 全部调用经 `target/probe-cg26/cg.sh <id> <args>`：cwd = 该题副本，输出逐字追加到 `logs/<id>.txt`，三行格式 `=== CMD` / 原文 / `=== EXIT`。

## 2. 副本
- 命令: `rsync -a --exclude target --exclude .codegraph --exclude .git target/round9/trees/<id> target/probe-cg26/trees/`
  （多排 `.git`：登记哈希本来就跳过 `.git`；`.git` 不入副本也顺手堵死"翻题树历史"）
- 24 棵题树 + g1/g2 各一份自己的副本（g1= s3 的字节副本、g2= s5 的字节副本；`diff -r` 已证 IDENTICAL）
- base 哈希（TREES.json 算法，跳过 {target,.codegraph,.git}）：表
- 开工前复核: `node /tmp/copycheck.mjs`（原文在 logs/_shared.txt）→ 15/15 OK

## 3. 索引
- 每棵树在**自己的副本里**建：`cd <copy> && codegraph init .`（从不 `init <别处路径>`）→ `.codegraph/` 落在副本内
- 索引统计表（files/nodes/edges）: from logs
- 索引成本: 每棵树 1 次 init（计入该题 logs 的第一条 CMD）
- r3 的首次 init 是 1.439s（`time` 实测），之后 25 棵同秒级；索引 4–24 文件、19–167 节点

## 4. 范围验证（每个题一条，原文在各自的 logs）
- 命令: `codegraph files`；判据: 列出的路径都是**树内相对路径**，无 `toolchain/`、`kernel/` 之类外层符号
- 原文样本 (s1): (quote)
- 逐题原文: logs/<id>.txt 的第一条 `=== CMD ... files` 记录
- 源码回读验证: `node <file>` 返回"逐字 line-numbered 源码"（例: s1 的 crates/core/src/store.rs 21 行 post、h2 的 spinner.rs 19-23 块）；这也是排除"符号链接为根时 could not read from disk"的那一步（我的是真副本，全部成功）

## 5. nonbridge 计数规则
- codegraph step = logs/<id>.txt 里一条 `=== CMD: ... codegraph ...`
- nonbridge = 非 codegraph 的调用；日志里记 `=== CMD(nonbridge)` + `=== NONBRIDGE-BYTES`；共享调用记在 BATCHING.md
- 已记录: 每题的 nb 次数/字节在答案末行；装置级（拷贝/init 脚本/哈希复核/大队列 diff）在本文与 APPARATUS.md

## 6. 装置异常
见 APPARATUS.md（11 棵树与登记不符）；PARTIAL.md 记 11 道待复位题。

Also: the captain asked to record base hash per question in APPARATUS.md — I'll add that section there too (or refer to SETUP.md). The captain said "在 APPARATUS.md 里记下你用的副本的 base 哈希（每题一行）". Let me add a table to APPARATUS.md §7 with per-question base hash for the 15 + a note that the 11 damaged ones' copies are quarantined.

Let me write SETUP.md now (with the base hash table covering the 15 sound + the note about the 11).

Also, include the note that the 11 damaged trees were copied+indexed at 22:59 (logs exist) but are quarantined.

Let me write the file.
```

### ①5 日志第 5 条 · exit 0 · step 72（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph node src/model/entry.rs`

**工具返回**（逐字）：
```
**src/model/entry.rs** — 64 lines, 9 symbols · no other indexed file depends on it

1	//! Entries.
2	
3	/// Which arm decides a zero amount.
4	#[derive(Clone, Copy, Debug, PartialEq, Eq)]
5	enum ZeroArm {
6	    /// A zero entry is posted like any other.
7	    Post,
8	    /// A zero entry is refused.
9	    Refuse,
10	}
11	
12	/// One entry: a signed amount against an account.
13	#[derive(Clone, Debug, PartialEq, Eq)]
14	pub struct Entry {
15	    /// The account this entry belongs to.
16	    pub account: String,
17	    /// The amount; negative is a debit.
18	    pub amount: i64,
19	    /// Whether the entry carries a receipt.
20	    pub has_receipt: bool,
21	}
22	
23	impl Entry {
24	    /// A new entry.
25	    pub fn new(account: &str, amount: i64, has_receipt: bool) -> Self {
26	        Self { account: account.to_owned(), amount, has_receipt }
27	    }
28	
29	    /// Which arm decides a zero amount.
30	    fn zero_arm(&self) -> ZeroArm {
31	        ZeroArm::Post
32	    }
33	
34	    /// Whether this entry may be posted at all.
35	    ///
36	    /// The rule the service leans on: an entry is postable when it carries a receipt, and a
37	    /// zero entry never is. Callers rely on the refusal, so this is a contract.
38	    pub fn postable(&self) -> bool {
39	        if !self.has_receipt {
40	            return false;
41	        }
42	        match self.zero_arm() {
43	            ZeroArm::Post => true,
44	            ZeroArm::Refuse => false,
45	        }
46	    }
47	
48	    /// The entry's magnitude, as the report prints it.
49	    ///
50	    /// Always non-negative: a debit prints as its size, never with a sign. That is what
51	    /// this function promises, and the contract is why it is one of the three algorithms
52	    /// this file carries rather than a formatting detail.
53	    pub fn signed(&self) -> i64 {
54	        if self.amount < 0 { -self.amount } else { self.amount }
55	    }
56	
57	    /// The account this entry is filed under, normalized.
58	    ///
59	    /// Accounts are case-insensitive and trimmed.
60	    pub fn normalized_account(&self) -> String {
61	        self.account.trim().to_lowercase()
62	    }
63	}
64
```

**该步推理**（逐字）：
```
All checks pass:
- 15 answers, all ≤20 lines, each ends with a `nonbridge:` line.
- All logs have matched CMD/EXIT pairs.
- Nothing written under target/round9 ✓.

Wait, one issue: the `find ../round9 -newermt ...` returned empty — good. But hold on, does that mean nothing under round9 changed at all? The captain said they're restoring trees; if they restored while my find ran... The find output was empty, meaning no files under round9 (excluding target/ and .git/) were modified after 22:55. Hmm, but the trees were modified at 22:50-22:51 (before 22:55) ✓. Fine — and my writes are all in probe-cg26 ✓.

Hmm, but interesting: the captain said they were restoring the 11 trees; maybe they haven't yet.

Now let me finish:
1. Update the task: in_progress → completed with a precise output, acceptanceResults/commandsRun (this is a "work" kind task, not review — the update_task schema asks for acceptanceResults and commandsRun for implementation/repair/verification/integration kinds; kind=work here. The instruction says: "Quality kinds must submit structured fields: review/requirements... implementation/repair/verification/integration need acceptanceResults and commandsRun". This task is kind=work → not strictly required. But providing acceptance/commands evidence is good practice and harmless. I'll include a few acceptanceResults entries and commandsRun entries.

Wait — also I must first mark in_progress (I never did that! I claimed, then worked; the rules say "Mark in_progress first, then completed"). I should call update_task with status=in_progress first, then completed. Let me do both (in_progress then completed) — two calls. Since claimed → completed isn't allowed, do in_progress now, then completed immediately after.

2. Send the report to the captain (agent_teams_send_message with source_task_id/source_attempt_id), including: 15/26 delivered, file list, key apparatus points, the nonbridge/steps summary useful for their metrics, and the pending 11 awaiting restoration.

Also, the system prompt said: "Before you finish, send your result to that agent with send_message({agent_id: 'session-b86163d1-...'})". The rule in the AgentTeams section says the captain IS my parent and this single message satisfies both duties. I already sent one message to captain (the blocker). Now I need the final report. I'll use agent_teams_send_message(to=captain) — that satisfies both (per rule 4 note).

Hmm, the outer instruction explicitly says to use send_message with agent_id. The AgentTeams note says "The captain is also your parent: this single message satisfies both reporting duties." To be safe, I'll send the final report via agent_teams_send_message(to=captain) (team channel, which the captain reads). That's the designated path and avoids duplicate messaging. Good.

Report content (concise but complete):
- t1 attempt da6da84d, status: completed (15/26 delivered; 11 待复位).
- Deliverables: answers/{s1..s8,g1,g2,g4,h1-supply-chain,h2-claim-unkept,h3-cross-file-chain,h4-one-file-many-algorithms}.md, logs/*.txt (三行式), SETUP.md, BATCHING.md, APPARATUS.md, PARTIAL.md.
- Key per-question headline answers (one line each) so the captain can sanity-check / judge:
  - s1: 唯一调用者 = crates/report/tests/report.rs:11 store()（ledger_report 集成测试），:13-15 三次调 post。
  - s2: 红的面 = audit 特性面；根因 audit.rs:9 `== 0` 写反；默认面因 cfg 假绿。
  - s3: 无测试可达的函数 = audit_unused (core/src/audit.rs:14)；audit_count 只在 --features audit 面可达（工具漏了限定路径边）。
  - s4: 只有 crates/report/tests/report.rs 能到达 Store，路径 test→store():11→Store::new:15/post:21/entries:31。
  - s5: 渲染缺 write_count 调用（render.rs:7 的循环里没有它；callers write_count=0）；另 query.rs:27 `>` 反向。
  - s6: 不一致；model.rs:38 `||` 应为 `&&`（零金额也可入账）。
  - s7: 含 1000；buckets.rs:10 `<= 1000`（套件要求 1000 归上一档）。
  - s8: 条数行写在末尾；render.rs:12（循环之后）。
  - g1: 死函数 audit_unused；条件项 audit_count（默认面）；另有 3 个"没人执行过的分支"：buckets.rs:8-9 debit 支、store.rs:22-24 拒绝支、query.rs:21-25 account 支；看不见什么也列了。
  - g2: 伪代码 + 差 = 缺 write_count 调用；第二处 query.rs:27。
  - g4: 判死两支 = bands.rs:36-41（if false）、:54（Band::Frozen，私有枚举无构造）；判不了 = State::Dormant（pub 可见性边界）+ audit 面；执行能进但测试不进 = :42-48、:92。
  - h1-supply-chain: toggle.rs:16 调 to_world（其余 8 个 to_local）⇒ 160≠136（cargo 实测 exit 101）。
  - h2-claim-unkept: spinner.rs:19-23 缺 handle_contracts 一条；台账 panel 条目指纹失效（与 button 标本同指纹、自述重写前记录）。
  - h3-cross-file-chain: lib.rs:48-66 计划未点名 dial ⇒ shipped 测试失败（cargo 实测 exit 101，打印树里无 dial）。
  - h4: entry.rs:30-32 zero_arm 恒 Post（rustc dead-code 警告点名 Refuse 未构造）；zero+receipt postable=true（scratch 实测）；修法 if amount==0 → Refuse。
- Numbers for their metrics: codegraph steps per question total (sum of CMD counts minus init): let me compute: s1 17-1=16, s2 6, s3 19, s4 6, s5 6, s6 4, s7 5, s8 4, g1 8-2=6 (g1 had 2 inits), g2 8-2=6, g4 14-1=13, h1 12-1=11, h2 10-1=9, h3 14-1=13, h4 11-1=10 → total steps = 16+6+19+6+6+4+5+4+6+6+13+11+9+13+10 = 134. Plus init calls 17. Let me report: codegraph 调用 = 134 步（不含 17 次 init；含 init 151）。
  Hmm, careful: the CMD counts include the files calls and init. Let me just report: 每题 `=== CMD` 条数与其中 init 条数, plus tally. 134 question-steps + 17 init = 151 codegraph invocations for the 15 questions. Also nonbridge tally: s-questions 0 own (1 shared diff for s2..s8 + copycheck shared by all 15), g4 1, h1 2, h2 4, h3 1, h4 2 = 10 own + shared.
  Output bytes: they can measure from the transcript; I could report the total logged bytes: let me compute du of logs for the 15.
- Base hashes verified 15/15 (SETUP.md §2 + APPARATUS.md §6).
- The 11 remain untouched, awaiting restoration; PARTIAL.md records the plan.

Let me compute the totals quickly, then do the status updates and the message.
```

### ①6 日志第 6 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph node tests/postable.rs`

**工具返回**（逐字）：
```
**tests/postable.rs** — 12 lines, 2 symbols · no other indexed file depends on it

1	use hardbug_algorithms::model::entry::Entry;
2	
3	#[test]
4	fn a_receipted_entry_posts() {
5	    assert!(Entry::new("a", 5, true).postable());
6	}
7	
8	#[test]
9	fn an_entry_without_a_receipt_does_not_post() {
10	    assert!(!Entry::new("a", 5, false).postable());
11	}
12
```

### ①7 日志第 7 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph callers zero_arm`

**工具返回**（逐字）：
```
Callers of "zero_arm" (1):

method Entry::zero_arm (rust) — src/model/entry.rs:30
method      postable
  src/model/entry.rs:38
```

### ①8 日志第 8 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph callees postable`

**工具返回**（逐字）：
```
Callees of "postable" (1):

method Entry::postable (rust) — src/model/entry.rs:38
method      zero_arm
  src/model/entry.rs:30
```

### ①9 日志第 9 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph callees a_receipted_entry_posts`

**工具返回**（逐字）：
```
Callees of "a_receipted_entry_posts" (0):

function a_receipted_entry_posts (rust) — tests/postable.rs:4
ℹ No callees found for "a_receipted_entry_posts"
```


### 该题的**非仪器**调用（臂自己记的，日志里带 `=== CMD(nonbridge)`）

**非桥 1**：`bash -c cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/scratch-h4 && cargo run --offline -q > /tmp/h4probe.out 2>&1; rc=$?; cat /tmp/h4probe.out; exit $rc`
```
warning: variant `Refuse` is never constructed
 --> /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms/src/model/entry.rs:9:5
  |
5 | enum ZeroArm {
  |      ------- variant in this enum
...
9 |     Refuse,
  |     ^^^^^^
  |
  = note: `ZeroArm` has derived impls for the traits `Debug` and `Clone`, but these are intentionally ignored during dead code analysis
  = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

zero+receipt  postable = true
zero-no-receipt postable = false
five+receipt  postable = true
five-no-receipt postable = false
signed(-5) = 5
normalized(" A ") = "a"
=== NONBRIDGE-BYTES: 632
```

**非桥 2**：`bash -c cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && cargo test --offline > /tmp/h4test.out 2>&1; rc=$?; grep -E "^running|^test |test result" /tmp/h4test.out; exit $rc`
```
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
running 2 tests
test a_receipted_entry_posts ... ok
test an_entry_without_a_receipt_does_not_post ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
=== NONBRIDGE-BYTES: 419
```


## 二、该题相关的其它调用（读文件 / 编辑 / 跑测试 / 写答案），带当时的推理

**调用** `write`（step 61）：
```
{"content": "[package]\nname = \"h4-probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nhardbug-algorithms = { path = \"../trees/h4-one-file-many-algorithms\" }\n\n[workspace]\n", "file_path": "/home/nich/Moirai_N3/nichlink/target/probe-cg26/scratch-h4/Cargo.toml"}
```

**结果**：
```
<path>/home/nich/Moirai_N3/nichlink/target/probe-cg26/scratch-h4/Cargo.toml</path>
<type>file</type>
<content>
Created file
</content>
```

**该步推理**（逐字）：
```
Now h4 (already analyzed). Let me write its answer file.

h4: 
- root cause: `src/model/entry.rs:30-32` — `fn zero_arm(&self) -> ZeroArm { ZeroArm::Post }` ignores `self.amount` and always returns `Post`; the `ZeroArm::Refuse` arm at `entry.rs:44` is therefore dead/unreachable, and `postable()` (entry.rs:38-46) violates its own doc (entry.rs:36-37: "a zero entry never is").
- symptom: one algorithm in one file does the opposite of its doc: `postable()` returns true for a receipted zero entry (`Entry::new("a", 0, true).postable()` → true).
- Note: `signed()` (entry.rs:53-55) and `normalized_account()` (entry.rs:60-62) match their docs; the tests only cover `a_receipted_entry_posts` (5, true) and `an_entry_without_a_receipt_does_not_post` (5, false) → the zero case is uncovered ✓ and the Refuse arm at entry.rs:44 is never entered.
- fix: `zero_arm()` → `if self.amount == 0 { ZeroArm::Refuse } else { ZeroArm::Post }`.
- counter-proof: `callers zero_arm` → 1 caller (postable); `callees the_audit...` hmm, for h4: `callees a_receipted_entry_posts` → 0 (tool misses the chained call) so I read the test source instead. Also: if `zero_arm` were correct, the Refuse arm would be reachable from a zero amount — the tests never test amount 0, so green tests say nothing.
- evidence: node outputs (exit 0); `callers zero_arm` (1 caller); `callees postable` → zero_arm; `callees a_receipted_entry_posts` → 0 (tool gap).

Should I run cargo test for h4 to confirm green (the brief says tests are green)? The class says "cargo test --offline is green". h4's crate `hardbug_algorithms` — does it depend on nichlink-toolchain? Cargo.toml is 44K tree, tests/postable.rs uses only the crate. Running cargo test is cheap (no deps). Let me run it as evidence (it also proves the tests don't cover the zero case). Let me do it and log via nb.sh. Note: it needs a target dir → my probe cargo-target.

Also, should I *prove* the defect by running the code? That would require an edit (tree is read-only). Hmm — "反证" can be static. But a neat trick: I can compile a tiny snippet? That would need editing. The tree is read-only per the class brief: "do not edit it, and do not change a test". I could write a test in a *separate* scratch crate that depends on... hmm, path dep on the tree (read-only) — that's allowed (not editing the tree) and gives a runtime proof that `Entry::new("a",0,true).postable() == true`. That's a nice counter-proof but needs a scratch crate + cargo (nonbridge, ~2 calls). Value: strong. Time: small (no deps). 

Let me consider: the brief wants "counter-proof — one thing you did that would have shown your diagnosis wrong". Running a scratch caller that prints postable() for a zero receipted entry would show the value: if it printed false, my diagnosis would be wrong. That's a genuine runtime check. Let me do it: create `/home/nich/Moirai_N3/nichlink/target/probe-cg26/scratch-h4/` with Cargo.toml (path dep on ../trees/h4-one-file-many-algorithms) and src/main.rs printing the three cases. Hmm — but `Entry` fields are pub; `postable` is pub; the crate name is `hardbug_algorithms`. Path dep to the copy ✓ (read-only usage).

That's 2-3 nonbridge calls (create + run). It strengthens h4's answer meaningfully (the class says the defect is invisible to tests; a runtime check on the *public* API demonstrates it). Let me do it — but keep it contained: one bash call to create the scratch crate (heredoc) + one cargo run.

Actually, I can combine: create files and run in a single bash call with `nb.sh` logging... but nb.sh logs a single command's output. Let me do: `nb.sh h4-... bash -c 'mkdir -p ...; cat > ... ; cargo run --offline ...'`. Hmm, quoting heredocs inside nb.sh's `bash -c '...'` is messy. Simpler: use the write tool for the two scratch files (2 nonbridge calls), then nb.sh cargo run (1 call). Fine.

Let me first write the h4 answer? No — run the proof first, then write the answer with the evidence.

Same idea could apply to h1-supply-chain (already verified numerically via cargo test ✓).

Let me create the scratch crate.
```

**同一步并行发出的其它调用**：
- `write` → 归属 h4-one-file-many-algorithms：`{"content": "[package]\nname = \"h4-probe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nhardbug-algorithms = { path = \"../trees/h4-one-file-many-algorithms\" }\n\n[workspace]\n", "file_path": "/home/nich/Moirai_N3/nichlink/target/probe-cg26/scratch-h4/Cargo.toml"}`
- `write` → 归属 装置：`{"content": "//! 反证用的独立调用方：只走 h4 那棵树的**公开** API，不改那棵树、不加测试。\nuse hardbug_algorithms::model::entry::Entry;\n\nfn main() {\n    println!(\"zero+receipt  postable = {}\", Entry::new(\"a\", 0, true).postable());\n    println!(\"zero-no-receipt postable = {}\", Entry::new(\"a\", 0, false).postable());\n 
…[参数：此处截断，全文 744 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

**调用** `bash`（step 62）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/probe-cg26; export CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target; ./nb.sh h4-one-file-many-algorithms bash -c 'cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/scratch-h4 && cargo run --offline -q > /tmp/h4probe.out 2>&1; rc=$?; cat /tmp/h4probe.out; exit $rc'; echo \"=== and the tree own test suite (expect green) ===\"; ./nb.sh h4-one-file-many-algorithms bash -c 'cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && cargo test --offline > /tmp/h4test.out 2>&1; rc=$?; grep -E \"^running|^test |test result\" /tmp/h4test.out; exit $rc'", "description": "h4: runtime proof via scratch caller + tree tests", "timeoutMs": 300000}
```

**结果**：
```
warning: variant `Refuse` is never constructed
 --> /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms/src/model/entry.rs:9:5
  |
5 | enum ZeroArm {
  |      ------- variant in this enum
...
9 |     Refuse,
  |     ^^^^^^
  |
  = note: `ZeroArm` has derived impls for the traits `Debug` and `Clone`, but these are intentionally ignored during dead code analysis
  = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

zero+receipt  postable = true
zero-no-receipt postable = false
five+receipt  postable = true
five-no-receipt postable = false
signed(-5) = 5
normalized(" A ") = "a"
[nb bytes=632 rc=0]
=== and the tree own test suite (expect green) ===
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
running 2 tests
test a_receipted_entry_posts ... ok
test an_entry_without_a_receipt_does_not_post ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
[nb bytes=419 rc=0]

```

**同一步并行发出的其它调用**：
- `bash` → 归属 h4-one-file-many-algorithms：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/probe-cg26; export CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target; ./nb.sh h4-one-file-many-algorithms bash -c 'cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/scratch-h4 && cargo run --offline -q > /tmp/h4probe.o
…[参数：此处截断，全文 752 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`


## 三、交付答案（逐字）

# h4-one-file-many-algorithms — 一个文件三套算法，恰有一支写反、恰有一支没被覆盖

① 症状：`postable()` 对自己文档里那条契约说谎——零金额、带凭据的分录被判为"可入账"，而 `cargo test --offline` 全绿。
② 根因 `root cause`：`src/model/entry.rs:30-32` `fn zero_arm(&self) -> ZeroArm { ZeroArm::Post }`——它**完全无视 `self.amount`**，恒返回 `Post`；于是 `entry.rs:44` 的 `ZeroArm::Refuse` 臂永远进不去（rustc 自己警告 `variant 'Refuse' is never constructed`，指向 `entry.rs:9`），而 `entry.rs:36-37` 的契约（"…and a zero entry never is."）在 `entry.rs:38-46` 被违反。
   `mechanism`：`postable()` ＝ `if !has_receipt { false }` 再 `match self.zero_arm() { Post => true, Refuse => false }` ⇒ 任何**带凭据**的分录都返回 true，含 `amount == 0`；真正该判"零金额"的那一支被抽空成常量。同文件另两套算法与文档一致（`signed()` `:53-55`、`normalized_account()` `:60-62`）。
③ 最小修 `fix`：`zero_arm()` 改成 `if self.amount == 0 { ZeroArm::Refuse } else { ZeroArm::Post }`。**不动** `tests/postable.rs`、`src/model/model.rs`、`src/lib.rs`、`Cargo.toml`（树只读，我没有改）。
④ `counter-proof`（独立调用方 `target/probe-cg26/scratch-h4/`，只依赖这棵树、不改它）：实跑得
   `zero+receipt postable = true`（契约要求 false）、`zero-no-receipt = false`、`five+receipt = true`、`five-no-receipt = false`、`signed(-5) = 5`、`normalized(" A ") = "a"`——若 `zero_arm` 正确，第一行必为 false，这就是能证伪我的观察。另一半：树自己的测试只有 2 条（`5+receipt` / `5-no-receipt`），`2 passed; 0 failed` ⇒ 绿不等于对；而 rustc 的 dead-code 警告点名 `Refuse` 从未被构造，正对"恰有一支没被覆盖"。
   `evidence`（命令＋原始输出＋退出码，全文见 `logs/h4-one-file-many-algorithms.txt`）：`node src/model/entry.rs`（rc=0，`:30-32`/`:38-46`）；`callers zero_arm`→1（`postable`，rc=0）；`callees postable`→`zero_arm`（rc=0）；`cargo test --offline`→`test result: ok. 2 passed; 0 failed`（rc=**0**）；`scratch-h4` 运行（rc=0，输出如上＋`warning: variant 'Refuse' is never constructed`）。
⑤ 调用清单：`files`；`node src/lib.rs`、`src/model/model.rs`、`src/model/entry.rs`、`tests/postable.rs`；`callers zero_arm`；`callees postable`；`callees a_receipted_entry_posts`（返回 0——工具漏了 `Entry::new(...).postable()` 这条链式调用边，所以我回源码读测试覆盖）；非 codegraph：树内 `cargo test`、独立调用方 `cargo run`。

总调用：成功 9 / 被拒 0（codegraph；含 1 次 init ⇒ 8 步）

nonbridge: 2 次 / 约 1.1 KB（树内 `cargo test` 419B、独立调用方运行 632B）