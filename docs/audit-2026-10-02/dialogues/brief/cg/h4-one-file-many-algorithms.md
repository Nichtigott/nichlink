# cg · `h4-one-file-many-algorithms`（精简版；完整版见 `../cg/h4-one-file-many-algorithms.md`）

**题面**：hardbug h4：单文件多算法——一个文件三套算法，恰一支写反、恰一支不可达。

> 仪器调用 **9** · 步 2 · 输出 3,695 tok · 推理 8,429 字符 · 累计上下文 339,328

## 仪器调用序列（工具给了什么 + 当时 agent 在想什么）

### ①1 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph init .`  · exit 0 · step 63（turn 1）

**工具返回**：
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

**当时推理**：
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
④ counter-proof: 独立调用方（scratch-h4，只依赖该树）实跑：`zero+receipt postable = true`（契约要求 false）、`five+receipt = true`、`five-no-receipt = false`、`signed(-5) = 5`、`normalized
…[推理：此处截断，全文 2529 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①2 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph files`  · exit 0 · step 67（turn 1）

**工具返回**：
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

**当时推理**：
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
- codegraph step = logs/<id
…[推理：此处截断，全文 2336 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①3 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph node src/lib.rs`  · exit 0 · step 72（turn 1）

**工具返回**：
```
**src/lib.rs** — 5 lines, 0 symbols · no other indexed file depends on it

1	//! A ledger model.
2	
3	#[path = "model/model.rs"]
4	pub mod model;
5
```

**当时推理**：
```
All checks pass:
- 15 answers, all ≤20 lines, each ends with a `nonbridge:` line.
- All logs have matched CMD/EXIT pairs.
- Nothing written under target/round9 ✓.

Wait, one issue: the `find ../round9 -newermt ...` returned empty — good. But hold on, does that mean nothing under round9 changed at all? The captain said they're restoring trees; if they restored while my find ran... The find output was empty, meaning no files under round9 (excluding target/ and .git/) were modified after 22:55. Hmm, but the trees were modified at 22:50-22:51 (before 22:55) ✓. Fine — and my writes are all in probe-cg26 ✓.

Hmm, but interesting: the captain said they were restoring the 11 trees; maybe they haven't yet.

Now let me finish:
1. Update the task: in_progress → completed with a precise output, acceptanceResults/commandsRun (this is a "work" kind task, not review — the update_task schema asks for acceptanceResults and commandsRun for implementation/repair/verification/integration kinds; kind=work here. The instruction says: "Quality kinds must submit structured fields: review/requirements... implementation/repair/verification/integration need acceptanceResults and commandsRun". This task is kind=work → not strictly required. But providing acceptance/commands evidence is good practice and harmless. I'll include a few acceptanceResults entries and commandsRun entries.

Wait — also I must first mark in_progress (I never did that! I claimed, then worked; the rules say "Mark in_progress first
…[推理：此处截断，全文 5635 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①4 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph node src/model/model.rs`  · exit 0 · 共享/装置步

**工具返回**：
```
**src/model/model.rs** — 3 lines, 0 symbols · no other indexed file depends on it

1	#[path = "entry.rs"]
2	pub mod entry;
3
```

### ①5 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph node src/model/entry.rs`  · exit 0 · 共享/装置步

**工具返回**：
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

### ①6 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph node tests/postable.rs`  · exit 0 · 共享/装置步

**工具返回**：
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

### ①7 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph callers zero_arm`  · exit 0 · 共享/装置步

**工具返回**：
```
Callers of "zero_arm" (1):

method Entry::zero_arm (rust) — src/model/entry.rs:30
method      postable
  src/model/entry.rs:38
```

### ①8 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph callees postable`  · exit 0 · 共享/装置步

**工具返回**：
```
Callees of "postable" (1):

method Entry::postable (rust) — src/model/entry.rs:38
method      zero_arm
  src/model/entry.rs:30
```

### ①9 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h4-one-file-many-algorithms && codegraph callees a_receipted_entry_posts`  · exit 0 · 共享/装置步

**工具返回**：
```
Callees of "a_receipted_entry_posts" (0):

function a_receipted_entry_posts (rust) — tests/postable.rs:4
ℹ No callees found for "a_receipted_entry_posts"
```


## 交付答案（逐字）

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