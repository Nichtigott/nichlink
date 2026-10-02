# cg · `h3-cross-file-chain`

**题面**：hardbug h3：跨文件因果链——入口计划从没点名新面，构建期作用域把它排除。

> 步 1 · 仪器调用 **13**（逐题日志条数）· 会话里定位到 4 条 · 输出 1,981 tok · 推理 2,305 字符 · 累计上下文 164,352

## 一、工具给了什么 · 当时 agent 在想什么（逐条配对）

> ⚠️ 日志 13 条 vs 会话定位 4 条：差的那些调用发生在**共享步**（一条命令里跑了多道题）或**装置步**里 —— 这正是"逐题 token 不可归因"的具体形状。

### ①1 日志第 1 条 · exit 0 · step 59（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph init .`

**工具返回**（逐字）：
```
┌  Initializing CodeGraph
│
◆  Initialized in /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain
│
Scanning files...
Parsing code...
Resolving refs...
Linking dynamic dispatch...
│
◆  Indexed 24 files
│
●  94 nodes, 119 edges in 499ms
│
└  Done
```

### ①2 日志第 2 条 · exit 0 · step 67（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph files`

**工具返回**（逐字）：
```
Project Structure (24 files):

├── graft
│   └── src
│       ├── badge_fast.rs (rust, 3 symbols)
│       ├── button_fast.rs (rust, 3 symbols)
│       ├── dial_fast.rs (rust, 3 symbols)
│       ├── gauge_fast.rs (rust, 3 symbols)
│       ├── lib.rs (rust, 4 symbols)
│       ├── panel_fast.rs (rust, 3 symbols)
│       ├── slider_fast.rs (rust, 3 symbols)
│       ├── spinner_fast.rs (rust, 3 symbols)
│       ├── timeline_fast.rs (rust, 3 symbols)
│       └── toggle_fast.rs (rust, 3 symbols)
└── host
    ├── src
    │   ├── control
    │   │   ├── object
    │   │   │   ├── badge
    │   │   │   │   └── badge.rs (rust, 5 symbols)
    │   │   │   ├── button
    │   │   │   │   └── button.rs (rust, 5 symbols)
    │   │   │   ├── dial
    │   │   │   │   └── dial.rs (rust, 5 symbols)
    │   │   │   ├── gauge
    │   │   │   │   └── gauge.rs (rust, 5 symbols)
    │   │   │   ├── panel
    │   │   │   │   └── panel.rs (rust, 5 symbols)
    │   │   │   ├── slider
    │   │   │   │   └── slider.rs (rust, 5 symbols)
    │   │   │   ├── spinner
    │   │   │   │   └── spinner.rs (rust, 5 symbols)
    │   │   │   ├── timeline
    │   │   │   │   └── timeline.rs (rust, 5 symbols)
    │   │   │   └── toggle
    │   │   │       └── toggle.rs (rust, 5 symbols)
    │   │   ├── registry_rule
    │   │   │   └── registry_rule.rs (rust, 3 symbols)
    │   │   └── control.rs (rust, 7 symbols)
    │   └── lib.rs (rust, 4 symbols)
    ├── tests
    │   └── shipped.rs (rust, 2 symbols)
    └── build.rs (rust, 2 symbols)
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

### ①3 日志第 3 条 · exit 0 · step 72（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph node host/src/lib.rs`

**工具返回**（逐字）：
```
**host/src/lib.rs** — 98 lines, 3 symbols · no other indexed file depends on it

1	//! NichLink 示例：README 里的 Control / Button 两层树，作为一个真实宿主库。
2	//! NichLink example: the README Control/Button two-level tree as a real host
3	//! library.
4	//!
5	//! 整个 crate 只有这里一处构建接线。`host!()` 引入构建期生成的注册计划；
6	//! 注册面代码保持普通 Rust，父级不维护子对象清单。
7	//! The crate has one build wiring point. `host!()` pulls in the plan the build
8	//! step generated; face code stays ordinary Rust and no parent keeps a child
9	//! roster.
10	
11	nichlink_toolchain::runtime::host!();
12	
13	// 这个 crate 自己调用 `host!()`，所以类型化 graft 计划里的 `crate::...` 与生成
14	// 树解析到同一个 crate。宿主如果把库和二进制分开，计划必须写在调用 `host!()`
15	// 的那一个里；写在另一个 crate 里的 Rust 路径无法在这里解析。
16	// This crate calls `host!()` itself, so `crate::...` in a typed graft plan
17	// resolves in the same crate as the generated tree. A host that splits a library
18	// and a binary must keep the plan in whichever one calls `host!()`.
19	
20	// `host!()` 已在 crate 根重导出 kernel，协议名词因此已在作用域内；再显式导入
21	// 会用私有项遮蔽那个公开重导出。
22	// `host!()` re-exports the kernel at the crate root, so the protocol nouns are
23	// already in scope; importing them again would shadow that public re-export.
24	
25	/// 这个示例的宿主身份。graft 要求覆盖双方共享同一个 framework。
26	/// The example's host identity. A graft requires both sides to share it.
27	pub const FRAMEWORK: FrameworkId = FrameworkId::new("nichlink.example.control-button");
28	
29	// 宿主入口的 graft 计划，用**类型化**写法：两侧都是指向真实注册面的 Rust 路径，
30	// 因此编译器与编辑器都能解析它们——写在 `cut(` 之后会补全宿主注册面路径，
31	// 写在 `graft(` 之后会补全外部 crate 路径。代价是外部实现必须被静态链接进来。
32	// The host's graft plan in the **typed** form: both sides are Rust paths to real
33	// faces, so the compiler and any editor resolve them. The cost is that the
34	// external implementation must be linked in.
35	//
36	// 这里声明的每个 `cut(` 都是宿主交出去的槽位，而构建期作用域收窄到这些切口命名的
37	// 子树：没有声明的注册面不会被这个应用发布。按钮和滑块都是可替换槽位，因此两条都写；
38	// 漏写一条不是"少发布一个面"这么无害，而是让那个槽位在发布态计划里失去目标。
39	// Every `cut(` declared here is a slot the host hands over, and the build-time
40	// scope narrows to the subtrees these cuts name: a face nobody declared is not
41	// shipped by this application. Button and slider are both replaceable slots, so
42	// both are declared; leaving one out does not merely ship one face less, it
43	// leaves that slot without a target in the release-time plan.
44	//
45	// 字符串写法仍然完全可用，只是工具无法补全它，也不需要链接外部实现：
46	//   cut "root/control/button" graft "button_fast"
47	// The string form still works and needs no link, but tooling cannot complete it.
48	nichlink_toolchain::runtime::static_graft_plan!(
49	    FRAMEWORK,
50	    cut(crate::control::object::button::NODE_ID)
51	        graft(control_button_graft::button_fast::NODE_ID),
52	    cut(crate::control::object::slider::NODE_ID)
53	        graft(control_button_graft::slider_fast::NODE_ID),
54	    cut(crate::control::object::timeline::NODE_ID)
55	        graft(control_button_graft::timeline_fast::NODE_ID),
56	    cut(crate::control::object::gauge::NODE_ID)
57	        graft(control_button_graft::gauge_fast::NODE_ID),
58	    cut(crate::control::object::toggle::NODE_ID)
59	        graft(control_button_graft::toggle_fast::NODE_ID),
60	    cut(crate::control::object::spinner::NODE_ID)
61	        graft(control_button_graft::spinner_fast::NODE_ID),
62	    cut(crate::control::object::panel::NODE_ID)
63	        graft(control_button_graft::panel_fast::NODE_ID),
64	    cut(crate::control::object::badge::NODE_ID)
65	        graft(control_button_graft::badge_fast::NODE_ID),
66	);
67	
68	/// 按框架和包命名空间装配这个示例的注册机。
69	/// Assemble the example's registry from its framework and package namespace.
70	pub fn base_registry() -> Registry {
71	    let mut registry = Registry::root_for_namespace(FRAMEWORK, env!("CARGO_PKG_NAME"));
72	    registry
73	        .register_all(&registrations())
74	        .expect("example faces register");
75	    registry
76	}
77	
78	/// 打印注册树的逻辑路径，供示例二进制和集成测试共用。
79	/// Print the registry tree's logical paths; shared by the example binary and the
80	/// integration tests.
81	pub fn outline() -> Vec<String> {
82	    let registry = base_registry();
83	    let mut rows = registry
84	        .depth_first()
85	        .iter()
86	        .map(|info| {
87	            format!(
88	                "{}  kind={}  source={}",
89	                registry.path_for(info.id).unwrap_or_default(),
90	                info.kind,
91	                info.source.file
92	            )
93	        })
94	        .collect::<Vec<_>>();
95	    rows.sort();
96	    rows
97	}
98
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

### ①4 日志第 4 条 · exit 0 · step 75（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph files`

**工具返回**（逐字）：
```
Project Structure (24 files):

├── graft
│   └── src
│       ├── badge_fast.rs (rust, 3 symbols)
│       ├── button_fast.rs (rust, 3 symbols)
│       ├── dial_fast.rs (rust, 3 symbols)
│       ├── gauge_fast.rs (rust, 3 symbols)
│       ├── lib.rs (rust, 4 symbols)
│       ├── panel_fast.rs (rust, 3 symbols)
│       ├── slider_fast.rs (rust, 3 symbols)
│       ├── spinner_fast.rs (rust, 3 symbols)
│       ├── timeline_fast.rs (rust, 3 symbols)
│       └── toggle_fast.rs (rust, 3 symbols)
└── host
    ├── src
    │   ├── control
    │   │   ├── object
    │   │   │   ├── badge
    │   │   │   │   └── badge.rs (rust, 5 symbols)
    │   │   │   ├── button
    │   │   │   │   └── button.rs (rust, 5 symbols)
    │   │   │   ├── dial
    │   │   │   │   └── dial.rs (rust, 5 symbols)
    │   │   │   ├── gauge
    │   │   │   │   └── gauge.rs (rust, 5 symbols)
    │   │   │   ├── panel
    │   │   │   │   └── panel.rs (rust, 5 symbols)
    │   │   │   ├── slider
    │   │   │   │   └── slider.rs (rust, 5 symbols)
    │   │   │   ├── spinner
    │   │   │   │   └── spinner.rs (rust, 5 symbols)
    │   │   │   ├── timeline
    │   │   │   │   └── timeline.rs (rust, 5 symbols)
    │   │   │   └── toggle
    │   │   │       └── toggle.rs (rust, 5 symbols)
    │   │   ├── registry_rule
    │   │   │   └── registry_rule.rs (rust, 3 symbols)
    │   │   └── control.rs (rust, 7 symbols)
    │   └── lib.rs (rust, 4 symbols)
    ├── tests
    │   └── shipped.rs (rust, 2 symbols)
    └── build.rs (rust, 2 symbols)
```

### ①5 日志第 5 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph node host/src/lib.rs`

**工具返回**（逐字）：
```
**host/src/lib.rs** — 98 lines, 3 symbols · no other indexed file depends on it

1	//! NichLink 示例：README 里的 Control / Button 两层树，作为一个真实宿主库。
2	//! NichLink example: the README Control/Button two-level tree as a real host
3	//! library.
4	//!
5	//! 整个 crate 只有这里一处构建接线。`host!()` 引入构建期生成的注册计划；
6	//! 注册面代码保持普通 Rust，父级不维护子对象清单。
7	//! The crate has one build wiring point. `host!()` pulls in the plan the build
8	//! step generated; face code stays ordinary Rust and no parent keeps a child
9	//! roster.
10	
11	nichlink_toolchain::runtime::host!();
12	
13	// 这个 crate 自己调用 `host!()`，所以类型化 graft 计划里的 `crate::...` 与生成
14	// 树解析到同一个 crate。宿主如果把库和二进制分开，计划必须写在调用 `host!()`
15	// 的那一个里；写在另一个 crate 里的 Rust 路径无法在这里解析。
16	// This crate calls `host!()` itself, so `crate::...` in a typed graft plan
17	// resolves in the same crate as the generated tree. A host that splits a library
18	// and a binary must keep the plan in whichever one calls `host!()`.
19	
20	// `host!()` 已在 crate 根重导出 kernel，协议名词因此已在作用域内；再显式导入
21	// 会用私有项遮蔽那个公开重导出。
22	// `host!()` re-exports the kernel at the crate root, so the protocol nouns are
23	// already in scope; importing them again would shadow that public re-export.
24	
25	/// 这个示例的宿主身份。graft 要求覆盖双方共享同一个 framework。
26	/// The example's host identity. A graft requires both sides to share it.
27	pub const FRAMEWORK: FrameworkId = FrameworkId::new("nichlink.example.control-button");
28	
29	// 宿主入口的 graft 计划，用**类型化**写法：两侧都是指向真实注册面的 Rust 路径，
30	// 因此编译器与编辑器都能解析它们——写在 `cut(` 之后会补全宿主注册面路径，
31	// 写在 `graft(` 之后会补全外部 crate 路径。代价是外部实现必须被静态链接进来。
32	// The host's graft plan in the **typed** form: both sides are Rust paths to real
33	// faces, so the compiler and any editor resolve them. The cost is that the
34	// external implementation must be linked in.
35	//
36	// 这里声明的每个 `cut(` 都是宿主交出去的槽位，而构建期作用域收窄到这些切口命名的
37	// 子树：没有声明的注册面不会被这个应用发布。按钮和滑块都是可替换槽位，因此两条都写；
38	// 漏写一条不是"少发布一个面"这么无害，而是让那个槽位在发布态计划里失去目标。
39	// Every `cut(` declared here is a slot the host hands over, and the build-time
40	// scope narrows to the subtrees these cuts name: a face nobody declared is not
41	// shipped by this application. Button and slider are both replaceable slots, so
42	// both are declared; leaving one out does not merely ship one face less, it
43	// leaves that slot without a target in the release-time plan.
44	//
45	// 字符串写法仍然完全可用，只是工具无法补全它，也不需要链接外部实现：
46	//   cut "root/control/button" graft "button_fast"
47	// The string form still works and needs no link, but tooling cannot complete it.
48	nichlink_toolchain::runtime::static_graft_plan!(
49	    FRAMEWORK,
50	    cut(crate::control::object::button::NODE_ID)
51	        graft(control_button_graft::button_fast::NODE_ID),
52	    cut(crate::control::object::slider::NODE_ID)
53	        graft(control_button_graft::slider_fast::NODE_ID),
54	    cut(crate::control::object::timeline::NODE_ID)
55	        graft(control_button_graft::timeline_fast::NODE_ID),
56	    cut(crate::control::object::gauge::NODE_ID)
57	        graft(control_button_graft::gauge_fast::NODE_ID),
58	    cut(crate::control::object::toggle::NODE_ID)
59	        graft(control_button_graft::toggle_fast::NODE_ID),
60	    cut(crate::control::object::spinner::NODE_ID)
61	        graft(control_button_graft::spinner_fast::NODE_ID),
62	    cut(crate::control::object::panel::NODE_ID)
63	        graft(control_button_graft::panel_fast::NODE_ID),
64	    cut(crate::control::object::badge::NODE_ID)
65	        graft(control_button_graft::badge_fast::NODE_ID),
66	);
67	
68	/// 按框架和包命名空间装配这个示例的注册机。
69	/// Assemble the example's registry from its framework and package namespace.
70	pub fn base_registry() -> Registry {
71	    let mut registry = Registry::root_for_namespace(FRAMEWORK, env!("CARGO_PKG_NAME"));
72	    registry
73	        .register_all(&registrations())
74	        .expect("example faces register");
75	    registry
76	}
77	
78	/// 打印注册树的逻辑路径，供示例二进制和集成测试共用。
79	/// Print the registry tree's logical paths; shared by the example binary and the
80	/// integration tests.
81	pub fn outline() -> Vec<String> {
82	    let registry = base_registry();
83	    let mut rows = registry
84	        .depth_first()
85	        .iter()
86	        .map(|info| {
87	            format!(
88	                "{}  kind={}  source={}",
89	                registry.path_for(info.id).unwrap_or_default(),
90	                info.kind,
91	                info.source.file
92	            )
93	        })
94	        .collect::<Vec<_>>();
95	    rows.sort();
96	    rows
97	}
98
```

### ①6 日志第 6 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph node host/tests/shipped.rs`

**工具返回**（逐字）：
```
**host/tests/shipped.rs** — 25 lines, 1 symbol · no other indexed file depends on it

1	//! What this host actually ships: the tree the build scoped.
2	//! 这个宿主真正发布的东西：构建划定作用域后的那棵树。
3	
4	#[test]
5	fn every_declared_widget_ships() {
6	    let rows = control_button::outline();
7	    let shipped = rows.join("\n");
8	    for name in [
9	        "button",
10	        "slider",
11	        "timeline",
12	        "gauge",
13	        "toggle",
14	        "spinner",
15	        "panel",
16	        "badge",
17	        "dial",
18	    ] {
19	        assert!(
20	            shipped.contains(&format!("/{name}  kind=")),
21	            "`{name}` is declared and does not ship; the tree is:\n{shipped}"
22	        );
23	    }
24	}
25
```

### ①7 日志第 7 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph node host/build.rs`

**工具返回**（逐字）：
```
**host/build.rs** — 4 lines, 1 symbol · no other indexed file depends on it

1	fn main() {
2	    nichlink_toolchain::build_time::run();
3	}
4
```

### ①8 日志第 8 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph node host/src/control/object/dial/dial.rs`

**工具返回**（逐字）：
```
**host/src/control/object/dial/dial.rs** — 25 lines, 3 symbols · used by 1 file: host/src/control/control.rs

1	//! The dial face.
2	
3	use crate::control::{ControlFrame, ControlHandle, to_local};
4	
5	pub struct Dial;
6	
7	impl ControlHandle for Dial {
8	    /// Paint this widget onto the frame it was handed.
9	    fn paint(&self) -> ControlFrame {
10	        ControlFrame
11	    }
12	}
13	
14	/// This widget's offset, in the coordinate family its siblings use.
15	pub fn offset(x: i32) -> i32 {
16	    to_local(x)
17	}
18	
19	crate::control_object! {
20	    kind: Dial,
21	    exports: ["control.render"],
22	    parent: crate::control::NODE_ID,
23	    handle_contracts: [crate::control::ControlHandle],
24	}
25
```

### ①9 日志第 9 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph callers dial::NODE_ID`

**工具返回**（逐字）：
```
ℹ Symbol "dial::NODE_ID" not found — did you mean: paint, Dial, DialFast?
```

### ①10 日志第 10 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph callers button::NODE_ID`

**工具返回**（逐字）：
```
ℹ Symbol "button::NODE_ID" not found — did you mean: paint, Button, ButtonFast?
```

### ①11 日志第 11 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph node host/src/control/registry_rule/registry_rule.rs`

**工具返回**（逐字）：
```
**host/src/control/registry_rule/registry_rule.rs** — 7 lines, 1 symbol · no other indexed file depends on it

1	//! The control parent's minimum structure for a direct child.
2	
3	use crate::RegistrationRule;
4	
5	pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()
6	    .require_exports(&["control.render"]);
7
```

### ①12 日志第 12 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph query dial`

**工具返回**（逐字）：
```
Search Results for "dial":

struct      Dial
  host/src/control/object/dial/dial.rs:5

struct      DialFast
  graft/src/dial_fast.rs:7

file        dial.rs
  host/src/control/object/dial/dial.rs:1

file        dial_fast.rs
  graft/src/dial_fast.rs:1

method      paint
  host/src/control/object/dial/dial.rs:9
  (&self) -> ControlFrame
```

### ①13 日志第 13 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph query dial_fast`

**工具返回**（逐字）：
```
Search Results for "dial_fast":

file        dial_fast.rs
  graft/src/dial_fast.rs:1
```


### 该题的**非仪器**调用（臂自己记的，日志里带 `=== CMD(nonbridge)`）

**非桥 1**：`bash -c cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain/host && cargo test --offline > /tmp/cg26_h3.out 2>&1; rc=$?; cat /tmp/cg26_h3.out; exit $rc`
```
Compiling nichlink-example-control-button-graft v0.1.0 (/home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain/graft)
   Compiling nichlink-example-control-button v0.1.0 (/home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain/host)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.43s
     Running unittests src/lib.rs (/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target/debug/deps/control_button-ec43ad9ac874301c)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/shipped.rs (/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target/debug/deps/shipped-243f9632fc5e54df)

running 1 test
test every_declared_widget_ships ... FAILED

failures:

---
…[输出：此处截断，全文 1911 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```


## 二、该题相关的其它调用（读文件 / 编辑 / 跑测试 / 写答案），带当时的推理


## 三、交付答案（逐字）

# h3-cross-file-chain — 源里有这个 widget，但它不在发布出来的注册树里

① 症状：`cargo test --offline` 在 shipped 上红：`dial` 被声明了却不在发布树里。
② 根因 `root cause`：`host/src/lib.rs:48-66` 的 `static_graft_plan!` 只点名了 8 个面（button/slider/timeline/gauge/toggle/spinner/panel/badge），**没有 dial 的那一对 `cut(...) graft(...)`**；应补的位置在 `:64-65`（badge 一对）之后。
   `mechanism`：构建期作用域收窄到入口点名的子树（同文件 `:36-43` 的说明：没人声明的面不会被这个应用发布），于是 `dial` 虽在源码里（有面文件、也有 graft 实现 `graft/src/dial_fast.rs`），却从不进生成的注册计划 ⇒ `outline()`（`lib.rs:81-97`）里没有它。
③ 最小修 `fix`：在 `lib.rs:65` 后加一对
   `cut(crate::control::object::dial::NODE_ID) graft(control_button_graft::dial_fast::NODE_ID),`
   不动 `host/tests/shipped.rs`、不动 dial 面文件、不动 graft。
④ 反证 `counter-proof`：若 dial 其实由别处进树，`outline()` 会打印它——实际打印的 9 行是 `root/control` + 8 个子面（badge/button/gauge/panel/slider/spinner/timeline/toggle），没有 dial；`query dial` 也只给出 `dial.rs` 与 `graft/src/dial_fast.rs` 两处符号（没有任何第三处引用），排除了"另一个入口点名了它"。
   `evidence`（命令＋原始输出＋退出码，全文见 `logs/h3-cross-file-chain.txt`）：`node host/src/lib.rs`（计划 8 条，exit 0）；`node host/tests/shipped.rs`（断言 9 个名字，exit 0）；`cargo test --offline`（`trees/h3-cross-file-chain/host`）→ `` `dial` is declared and does not ship; the tree is: `` + 9 行发布树清单 / `test result: FAILED. 0 passed; 1 failed`（exit **101**）。
⑤ 调用清单：`files`；`node host/src/lib.rs`、`host/tests/shipped.rs`、`host/build.rs`、`host/src/control/object/dial/dial.rs`、`host/src/control/registry_rule/registry_rule.rs`；`query dial`、`query dial_fast`；`callers dial::NODE_ID`（工具回 "not found"：`NODE_ID` 是宏生成的，图里没有 ⇒ 这条判断只能靠源码，见下）；cargo test（非 codegraph，1 次）。

总调用：成功 13 / 被拒 0（codegraph；含 1 次 init ⇒ 12 步）

nonbridge: 1 次 / 约 1.9 KB（`cargo test`，rc 用 `rc=$?` 真取）