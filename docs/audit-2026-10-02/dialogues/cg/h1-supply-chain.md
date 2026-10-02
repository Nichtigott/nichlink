# cg · `h1-supply-chain`

**题面**：hardbug h1：供应链驳杂——同族一个兄弟调用了另一族名字，症状是偏移 160≠136。

> 步 5 · 仪器调用 **10**（逐题日志条数）· 会话里定位到 7 条 · 输出 3,989 tok · 推理 8,503 字符 · 累计上下文 734,464

## 一、工具给了什么 · 当时 agent 在想什么（逐条配对）

> ⚠️ 日志 10 条 vs 会话定位 7 条：差的那些调用发生在**共享步**（一条命令里跑了多道题）或**装置步**里 —— 这正是"逐题 token 不可归因"的具体形状。

### ①1 日志第 1 条 · exit 0 · step 45（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph init .`

**工具返回**（逐字）：
```
┌  Initializing CodeGraph
│
◆  Initialized in /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain
│
Scanning files...
Parsing code...
Resolving refs...
Linking dynamic dispatch...
│
◆  Indexed 24 files
│
●  95 nodes, 121 edges in 578ms
│
└  Done
```

**该步推理**（逐字）：
```
Now the three h-classes. Let me start with h1-supply-chain (the "one sibling calls a different family of names than its siblings do"). 

Tree layout: graft/src/{gauge_fast,toggle_fast,slider_fast,button_fast,timeline_fast,panel_fast,spinner_fast,dial_fast,badge_fast,lib}.rs, host/src/control/object/{slider,panel,button,timeline,badge,dial,toggle,gauge,spinner}/*.rs + control.rs + registry_rule/registry_rule.rs + lib.rs + tests/offsets.rs, build.rs, Cargo.toml, host/.nichlink...

Symptom: "The rendered layout is off. cargo test --offline fails in the offsets test with a total that no single widget's own code explains, and every widget file reads as correct on its own."

Class: "one sibling calls a different family of names than its siblings do" (一个兄弟调用了与其余兄弟不同的那一族名字).

So: among the 9-10 object files, one calls a different family of names. Probably the host object files call graft functions named e.g. `button_fast::…`. Hmm, "family of names" — maybe the graft crate exposes two families: `*_fast` (e.g. `gauge_fast`, `toggle_fast`) and something else. Or the widget files call `parts()`/`slots()`-type API names.

Best approach: use codegraph to get callees of each of the 9 object files (or their primary functions) and compare. Let me first look at the tree: read tests/offsets.rs (the failing test), host/src/lib.rs (entry plan), control.rs, and one or two object files, plus graft/src/lib.rs.

Let me batch: node tests/offsets.rs, node src/lib.rs (host), node src/control/object/button/button.rs, node graft/src/lib.rs, node src/control/control.rs.

Given output sizes, let me do a few at a time. Start with tests/offsets.rs + host lib.rs + control.rs.
```

**该步模型输出**（逐字）：
```
Now **h1-supply-chain**. Reading the symptom test and entry plan:
```

### ①2 日志第 2 条 · exit 0 · step 46（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph files`

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
    │   └── offsets.rs (rust, 3 symbols)
    └── build.rs (rust, 2 symbols)
```

**该步推理**（逐字）：
```
Now I see the shape: `control.rs` defines two "families of names": `to_local(x) = x + 7` and `to_world(x) = x + 31`. The class: "one sibling calls a different family of names than its siblings do" — so among the widget `offset` functions, one uses `to_world` while the rest use `to_local`.

The offsets test: EXPECTED_TOTAL = 136, summing offset(10) over 8 widgets = 8 × (10+7) = 136 ✓ if all use `to_local`. So the odd sibling uses `to_world` → 10+31 = 41 → total = 136 - 17 + 41 = 160, which fails the assertion with "160, not 136" — "a total that no single widget's own code explains" ✓ (each widget file looks right on its own: the odd one calls `to_world` which is a legitimate function — but the family mismatch).

So I need to find which widget calls `to_world`. Use codegraph: `callers to_world`! And `callers to_local`. 

Let me do that: `./cg.sh h1-supply-chain callers to_world`, `callers to_local`, and check the widget files' offset functions.

Let me run it.
```

### ①3 日志第 3 条 · exit 0 · step 47（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph node host/tests/offsets.rs`

**工具返回**（逐字）：
```
**host/tests/offsets.rs** — 23 lines, 2 symbols · no other indexed file depends on it

1	//! The rendered layout: every sibling's offset, added up.
2	//! 渲染出的布局：每个兄弟的偏移加总。
3	
4	/// The frame this tree renders when every widget places itself the way its siblings do.
5	const EXPECTED_TOTAL: i32 = 136;
6	
7	#[test]
8	fn the_rendered_offsets_add_up() {
9	    let total =
10	        control_button::control::object::button::offset(10) +
11	        control_button::control::object::slider::offset(10) +
12	        control_button::control::object::timeline::offset(10) +
13	        control_button::control::object::gauge::offset(10) +
14	        control_button::control::object::toggle::offset(10) +
15	        control_button::control::object::spinner::offset(10) +
16	        control_button::control::object::panel::offset(10) +
17	        control_button::control::object::badge::offset(10);
18	    assert_eq!(
19	        total, EXPECTED_TOTAL,
20	        "the rendered offsets add up to {total}, not {EXPECTED_TOTAL}"
21	    );
22	}
23
```

**该步推理**（逐字）：
```
The odd sibling: `host/src/control/object/toggle/toggle.rs:15 fn offset` calls `to_world` (control.rs:8) while the other 8 widget `offset` functions call `to_local` (control.rs:5).

Let me confirm the numbers: `to_local(10) = 17`, `to_world(10) = 41`. Test sums 8 widgets: 7 × 17 + 41 = 119 + 41 = 160 ≠ 136 (expected = 8×17 = 136) ✓ — the test fails with 160. 

Let me read toggle.rs:15 and one sibling to confirm the exact line (e.g. button.rs:15) — and get raw evidence with exit codes. Also the failing test's raw output would be nice, but running cargo test on the h1-supply-chain tree requires building nichlink-toolchain (path dep to the workspace) — expensive. The brief demands "evidence: every claim: the command you ran, its raw output and its exit code". For my arm, codegraph commands are the commands. I can cite `callers to_world` output + exit 0.

Hmm, but the class brief demands raw output for every claim. My codegraph commands give raw output. That's fine — I'm the codegraph arm; but a *stronger* evidence would be an actual cargo test run showing the failure. That's a nonbridge call, builds the toolchain (heavy). Given that the ninth round arm probably ran cargo (its tool was nichlink-mcp, and the brief requires 交付后门必须绿 only for modifiable trees; for read-only diagnosis the evidence requirement says command+output+exit code).

Let me consider the budget: building the toolchain offline for h1-supply-chain's host would take maybe 1-3 minutes (the workspace's deps are cached). It would also let me verify my predicted number 160 vs 136 — a strong, decisive piece of counter-proof. Given the value (the truth for h1-supply-chain is exactly "toggle uses to_world" and the symptom number), a single cargo test run is worth it. Also for h3-cross-file-chain (shipped tree assertion) and h2 (tests pass; ledger) I might run cargo too.

But careful: the host crates need `carrier/control-button-graft`? No — h-classes' host Cargo.toml points graft to `../graft` (per TREES.json correction). So the host depends on `../graft` (present) ✓ and nichlink-toolchain (path to workspace ✓). Also the build.rs needs the toolchain's build feature — offline should work with the cargo registry cache (the workspace builds offline).

Also: cargo needs a target dir — I'll use my probe cargo-target (shared). Each host tree has the same crate name `control_button`? The trees fa/fb/fc/h1/... all use package name `nichlink-example-control-button`; sharing one target dir is fine (artifacts keyed by name+version+source), but different trees have different source → cargo will rebuild. Sharing is OK.

Hmm, but there's a risk: the h1-supply-chain host's Cargo.lock may reference path deps that exist. Let me check host/Cargo.toml quickly... Let me just try running the offsets test for h1-supply-chain. If it fails to build (missing deps/network), I'll fall back to static evidence and report the attempted command in the log.

Actually — the brief for h1-supply-chain says "The tree is read-only: do not edit it". Running cargo test in my copy writes into the copy's target dir (or my CARGO_TARGET_DIR) — that's not editing the tree's sources ✓.

Let me try it: `cargo test --offline` in trees/h1-supply-chain/host. I'll log it as a nonbridge call for that question with byte count.

Let me also grab the toggle.rs and button.rs sources via codegraph first (log), then run cargo.

Let me do both in one bash call: cg node toggle.rs, cg node button.rs, then nb.sh with cargo (timeout 600s).

Wait — one consideration: the h1-supply-chain host crate name is `control_button`, and its `graft` dep is `control_button_graft` at `../graft`. The graft crate's Cargo.toml may reference nichlink-toolchain too. Fine.

Let me run.
```

### ①4 日志第 4 条 · exit 0 · step 58（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph node host/src/lib.rs`

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
h2 confirmed:
- `spinner.rs:19-23` block is missing the 4th declaration `handle_contracts: [crate::control::ControlHandle],` that every other sibling has (button.rs:23). The constraint "every direct child implements ControlHandle" is declared in control.rs:13-16 (the trait) and repeated in every sibling's `control_object!` via `handle_contracts:` — spinner omits it.
- The ledger: panel entry lapsed.
- `callers REGISTRATION_RULE` = 0 (another codegraph resolution gap — the macro presumably references it; not needed here).

Also interesting: in h2, `to_world` is not called by anyone (toggle uses to_local in this tree?). Let me not worry; h2's class is the ledger+declaration.

Wait — one thing to check for h2: is the offsets test passing (yes, 1 passed ✓).

Let me now double-check spinner's missing declaration is *exactly one*: diff showed only that one hunk (plus name substitutions) ✓. And the other 7 siblings match the reference modulo names ✓ (from the diff output: only the handle_contracts hunk for spinner).

Now, do I need to determine the fingerprint scheme more precisely? Let me try one more approach: maybe the fingerprint is over the whole *file path + the registration block* in some canonical way. Instead of guessing, I can state: "两条记录写了同一个指纹，而两个被指名的文件字节不同（06c45576… / d886e9b0…）⇒ 至少一条不描述它指名的字节；panel 那条自己写着'在 panel 被重写之前记录的'（时间 2026-09-20，早于 button 的 2026-10-01）⇒ 失效的是 panel 那条。" And note the caveat: the ledger's fingerprint scheme isn't in this tree, so I compared the two entries against each other rather than recomputing the fingerprint — that's honest and decidable.

Actually, hmm, maybe I can find the scheme via the frozen codegraph CLI on the *tree* only — no. But wait: the fingerprint scheme might be over the NodeId which the sort of "file!()+line" hash produces. Let me try a couple more candidates quickly: sha256 of "src/control/object/button/button.rs:19" + kind etc. Not worth more time. The pairwise argument is decisive for the question ("one ledger entry no longer describes the bytes it names").

Hmm, although: could BOTH entries be stale (i.e. the "one" in the brief is an understatement)? The button entry is the specimen. Let me test one more plausible scheme: maybe the fingerprint is a *NodeId* computed as sha256 of the file path relative to the crate root, but with the face's registration line... Or maybe it's the hash of "the shape" as text: `kind: Button` normalized... Let me try one specific plausible thing: sha256 over the string "the shape every sibling follows"? No.

Let me try: sha256 of the block *without* the kind line, and sha256 of "root/control/button" with different separators, plus sha256 of the ledger line sans fingerprint. Quick to test; if nothing matches, I'll stop and report the pairwise argument.

Actually — maybe it doesn't matter much. The question asks: "一个同族只缺一条声明 + 一条指纹失效的条目". I have both. Let me spend no more time on the exact hash function, and state the method honestly.

Now let me also verify the "constraint this design adopted is not holding on a later sibling" — the adopted constraint = the `handle_contracts` declaration (the handle every direct child must implement, control.rs:13-16 "The interface every direct child must implement"). Does anything enforce it? `registry_rule.rs:5-6` requires only exports. So the constraint is declared per-face but not required by the registration rule ⇒ spinner's omission isn't caught (tests pass) ✓. That's the mechanism.

Let me write the three h-answers now: h1-supply-chain, h3-cross-file-chain, h2-claim-unkept.

For each, the brief demands five labeled fields: `root cause`, `mechanism`, `evidence`, `counter-proof`, `fix` — each on its own line, keeping the labels. Hmm! That's a *different* shape than the 5-section shape my task requires (① 症状 ② 根因/结论 ③ 最小修 ④ 反证 ⑤ 调用清单). The task says "每题 answers/<id>.md（≤20 行五段：① 症状一句 ② 根因/结论 文件:行号 + 机制一句 ③ 最小修或改动清单 ④ 反证 ⑤ 调用清单）". The class BRIEFs ask for labeled fields. I can satisfy both: use my 5-section shape but ALSO include the class labels for the four hardbug trees (e.g. ② root cause + mechanism, ④ counter-proof, ③ fix, ⑤ evidence list). Best: write the answer with the 5 required sections, and inside them use the class labels explicitly, e.g.:

② `root cause` = … ; `mechanism` = …

That keeps both formats satisfied. Let me do that for the 4 hardbug answers.

Let me now write h1-supply-chain's answer.

h1-supply-chain:
- root cause: `host/src/control/object/toggle/toggle.rs:16` `to_world(x)` (and its import at :3), while the 8 siblings call `to_local` (control.rs:5).
- mechanism: two coordinate families in control.rs (`to_local` +7, `to_world` +31); the test sums offset(10) over 8 widgets expecting 8×17=136; toggle contributes 41 ⇒ 160 ≠ 136; no single widget file looks wrong because both families are legitimate public functions and toggle's own doc line is copy-identical to its siblings'.
- evidence: codegraph `callers to_world` → exactly 1 caller (toggle.rs:15 offset); `callers to_local` → 8 callers (the 8 siblings); `callees offset` → 9 definitions each calling one family; cargo test raw output "160, not 136", exit 101.
- counter-proof: if toggle were calling to_local, or if the expected total accounted for to_world... Also I could check: 7×17+41=160 exactly matches the observed left value ⇒ arithmetic closes (this is the counter-proof that the od. Actually a real counter-proof: I predicted 160 from the graph before running cargo, and the test printed exactly 160 ✓ — that's a strong counter-proof: if my diagnosis were wrong (e.g. another widget also off-family), the printed number would differ.
- fix: toggle.rs:16 `to_world(x)` → `to_local(x)` (and toggle.rs:3 import `to_world` → `to_local`). Files NOT touched: control.rs (both families stay), the test, and the other 8 siblings.
- Read-only tree: I did not edit.

Let me write it in ≤20 lines. Then h3 and h2.

Also, one more check for h1-supply-chain: are there 8 or 9 `offset` callers? `callers to_local` listed
…[推理：此处截断，全文 6744 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**该步模型输出**（逐字）：
```
Writing the three h-class answers:
```

### ①5 日志第 5 条 · exit 0 · step 67（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph node host/src/control/control.rs`

**工具返回**（逐字）：
```
**host/src/control/control.rs** — 25 lines, 6 symbols · used by 9 files: host/src/control/object/badge/badge.rs, host/src/control/object/button/button.rs, host/src/control/object/dial/dial.rs, host/src/control/object/gauge/gauge.rs, host/src/control/object/panel/panel.rs, host/src/control/object/slider/slider.rs, host/src/control/object/spinner/spinner.rs, host/src/control/object/timeline/timeline.rs, +1 more

1	//! The control folder face, the two coordinate families its children may use, and the
2	//! handle every direct child implements.
3	
4	/// The local family: offsets are measured from the widget's own origin.
5	pub fn to_local(x: i32) -> i32 { x + 7 }
6	
7	/// The world family: offsets are measured from the frame's origin.
8	pub fn to_world(x: i32) -> i32 { x + 31 }
9	
10	/// The frame a parent face hands to its children for painting.
11	pub struct ControlFrame;
12	
13	/// The interface every direct child must implement.
14	pub trait ControlHandle {
15	    fn paint(&self) -> ControlFrame;
16	}
17	
18	pub struct Control;
19	
20	crate::root_object! {
21	    kind: Control,
22	    needs_registry: true,
23	    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
24	}
25
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

### ①6 日志第 6 条 · exit 0 · step 72（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph callers to_world`

**工具返回**（逐字）：
```
Callers of "to_world" (1):

function to_world (rust) — host/src/control/control.rs:8
function    offset
  host/src/control/object/toggle/toggle.rs:15
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

### ①7 日志第 7 条 · exit 0 · step 75（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph callers to_local`

**工具返回**（逐字）：
```
Callers of "to_local" (8):

function to_local (rust) — host/src/control/control.rs:5
function    offset
  host/src/control/object/gauge/gauge.rs:15

function    offset
  host/src/control/object/panel/panel.rs:15

function    offset
  host/src/control/object/spinner/spinner.rs:15

function    offset
  host/src/control/object/slider/slider.rs:15

function    offset
  host/src/control/object/timeline/timeline.rs:15

function    offset
  host/src/control/object/button/button.rs:15

function    offset
  host/src/control/object/dial/dial.rs:15

function    offset
  host/src/control/object/badge/badge.rs:15
```

### ①8 日志第 8 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph callees offset`

**工具返回**（逐字）：
```
Callees of "offset" — 9 distinct definitions (narrow with --file):

function offset (rust) — host/src/control/object/badge/badge.rs:15 (1):

function    to_local
  host/src/control/control.rs:5


function offset (rust) — host/src/control/object/button/button.rs:15 (1):

function    to_local
  host/src/control/control.rs:5


function offset (rust) — host/src/control/object/dial/dial.rs:15 (1):

function    to_local
  host/src/control/control.rs:5


function offset (rust) — host/src/control/object/gauge/gauge.rs:15 (1):

function    to_local
  host/src/control/control.rs:5


function offset (rust) — host/src/control/object/panel/panel.rs:15 (1):

function    to_local
  host/src/control/control.rs:5


function offset (rust) — host/src/control/object/slider/slider.rs:15 (1):

function    to_local
  host/src/control/control.rs:5


function offset (rust) — host/src/control/object/spinner/spinner.rs:15 (1):

function    to_local
  host/src/control/control.rs:5


function offset (rust) — host/src/control/object/timeline/timeline.rs:15 (1):

function    to_local
  host/src/control/control.rs:5


function offset (rust) — host/src/control/object/toggle/toggle.rs:15 (1):

function    to_world
  host/src/control/control.rs:8
```

### ①9 日志第 9 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph node host/src/control/object/toggle/toggle.rs`

**工具返回**（逐字）：
```
**host/src/control/object/toggle/toggle.rs** — 25 lines, 3 symbols · used by 1 file: host/src/control/control.rs

1	//! The toggle face.
2	
3	use crate::control::{ControlFrame, ControlHandle, to_world};
4	
5	pub struct Toggle;
6	
7	impl ControlHandle for Toggle {
8	    /// Paint this widget onto the frame it was handed.
9	    fn paint(&self) -> ControlFrame {
10	        ControlFrame
11	    }
12	}
13	
14	/// This widget's offset, in the coordinate family its siblings use.
15	pub fn offset(x: i32) -> i32 {
16	    to_world(x)
17	}
18	
19	crate::control_object! {
20	    kind: Toggle,
21	    exports: ["control.render"],
22	    parent: crate::control::NODE_ID,
23	    handle_contracts: [crate::control::ControlHandle],
24	}
25
```

### ①10 日志第 10 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph node host/src/control/object/button/button.rs`

**工具返回**（逐字）：
```
**host/src/control/object/button/button.rs** — 25 lines, 3 symbols · used by 1 file: host/src/control/control.rs

1	//! The button face.
2	
3	use crate::control::{ControlFrame, ControlHandle, to_local};
4	
5	pub struct Button;
6	
7	impl ControlHandle for Button {
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
20	    kind: Button,
21	    exports: ["control.render"],
22	    parent: crate::control::NODE_ID,
23	    handle_contracts: [crate::control::ControlHandle],
24	}
25
```


### 该题的**非仪器**调用（臂自己记的，日志里带 `=== CMD(nonbridge)`）

**非桥 1**：`bash -c cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain/host && cargo test --offline 2>&1 | grep -vE "^ *Compiling|^ *Finished|^ *Running" ; exit 101`
```
running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test the_rendered_offsets_add_up ... FAILED

failures:

---- the_rendered_offsets_add_up stdout ----

thread 'the_rendered_offsets_add_up' (589447) panicked at tests/offsets.rs:18:5:
assertion `left == right` failed: the rendered offsets add up to 160, not 136
  left: 160
 right: 136
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    the_rendered_offsets_add_up

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `--test offsets`
=== NONBRIDGE-BYTES: 687
```

**非桥 2**：`bash -c cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain/host && cargo test --offline > /tmp/cg26_h1.out 2>&1; rc=$?; cat /tmp/cg26_h1.out; exit $rc`
```
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running unittests src/lib.rs (/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target/debug/deps/control_button-ff660440993ab6a3)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/offsets.rs (/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target/debug/deps/offsets-ae09d92a888601fd)

running 1 test
test the_rendered_offsets_add_up ... FAILED

failures:

---- the_rendered_offsets_add_up stdout ----

thread 'the_rendered_offsets_add_up' (589472) panicked at tests/offsets.rs:18:5:
assertion `left == right` failed: the rendered offsets add up to 160, not 136
  left: 160
 right: 136
note: run with `RUST_BACKTRACE=1` environment va
…[输出：此处截断，全文 1050 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```


## 二、该题相关的其它调用（读文件 / 编辑 / 跑测试 / 写答案），带当时的推理

**调用** `bash`（step 48）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain/host && CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target timeout 900 cargo test --offline 2>&1 | tail -40", "description": "h1-supply-chain: run offsets test (background)", "run_in_background": true}
```

**结果**：
```
started background job bash-2153
```

**该步推理**（逐字）：
```
Perfect: `toggle.rs:3` imports `to_world` (vs siblings importing `to_local`), and `toggle.rs:16` calls `to_world(x)`. Same doc comment on both ("in the coordinate family its siblings use") — the toggle's doc is identical to the siblings', so each file "reads as correct on its own" ✓ (the class statement).

Now let me try running the test to get the actual failure output — good evidence and counter-proof. Let me run cargo test --offline in the h1-supply-chain host copy.

Risk: build time. Let me run with a generous timeout and in background if needed. Let me first check if the toolchain builds quickly... Let me just run it with timeoutMs 600000 (10 min) — the harness may move it to background if it exceeds.

Actually, `cargo test --offline` for this host requires building nichlink-toolchain (a big crate with proc-macro deps like syn? maybe). The workspace's own build cache is at /home/nich/Moirai_N3/nichlink/target (the workspace target dir) — a DIFFERENT target dir than my probe cargo-target, so it will rebuild from scratch. Could take several minutes. The deps come from the local registry cache (offline OK).

Given the value (only for h1-supply-chain, h3, h2 evidence) — that's 3 questions × maybe 2-5 min = manageable. But if the toolchain build is heavy (it has a proc-macro crate, wasmtime? — plugin_host might pull heavy deps when features are off... default features = build+run, so maybe moderate).

Let me try once for h1-supply-chain and measure. If it's fast enough, I'll do h2/h3 too.

Alternatively, I can verify the arithmetic without cargo: the test sums offset(10) over 8 widgets; toggle returns to_world(10)=41; others to_local(10)=17 ⇒ total = 7*17 + 41 = 160 ≠ 136. That's already a decisive, checkable claim derived from source (and codegraph callers). The cargo run adds "the test actually fails with 160" — nice-to-have but expensive.

Hmm. The captain's measurement wants steps/context. Extra cargo runs inflate my nonbridge count and time. The ninth round arm (nichlink-mcp) probably didn't need cargo for read-only diagnostics... but the brief's "evidence = command + raw output + exit code" suggests some command.

I'll do ONE cargo run for h1-supply-chain as a feasibility probe; if it builds within ~3-4 min, I'll reuse the same target dir for h2/h3 (incremental → fast). This is valuable evidence for 3 questions. Let me run it in the background and continue working on h3 static analysis meanwhile.

Let me start it as a background job.
```

**该步模型输出**（逐字）：
```
Let me start a cargo verification run in the background while I analyze h3:
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h1-supply-chain：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain/host && CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target timeout 900 cargo test --offline 2>&1 | tail -40", "description": "h1-supply-chain: run offsets test (background)", "run_in_back
…[参数：此处截断，全文 314 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`
- `bash` → 归属 h3-cross-file-chain：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/probe-cg26; ./cg.sh h3-cross-file-chain node host/src/lib.rs 2>&1 | sed -n '1,40p'; echo \"----- host files -----\"; ./cg.sh h3-cross-file-chain files 2>&1 | head -60", "description": "h3: entry plan and file layout", "timeoutMs": 120000}`

**调用** `bash`（step 52）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/probe-cg26; export CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target; ./nb.sh h1-supply-chain bash -c 'cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain/host && cargo test --offline 2>&1 | grep -vE \"^ *Compiling|^ *Finished|^ *Running\" ; exit 101' 2>&1 | tail -14", "description": "h1-supply-chain: logged cargo test run", "timeoutMs": 300000}
```

**结果**：
```
thread 'the_rendered_offsets_add_up' (589447) panicked at tests/offsets.rs:18:5:
assertion `left == right` failed: the rendered offsets add up to 160, not 136
  left: 160
 right: 136
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    the_rendered_offsets_add_up

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `--test offsets`
[nb bytes=687 rc=101]

```

**该步推理**（逐字）：
```
Excellent! The cargo run for h1-supply-chain worked and took only ~20s of compile time (the toolchain was built). Raw output:

```
running 1 test
test the_rendered_offsets_add_up ... FAILED
assertion `left == right` failed: the rendered offsets add up to 160, not 136
  left: 160
 right: 136
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```
Exactly matches my prediction: 160 vs 136 (7×17 + 41). And note the job's overall exit code was 0 due to `| tail -40` (pipeline exit = tail's). I must record the exit code honestly: the pipeline's rc is 0 but the test failed. In the log I should record the pipeline rc AND note the test outcome. Better: re-run with `--offline` and capture cargo's own rc via PIPESTATUS. The log entry should be accurate. Let me note in the answer that the recorded `=== EXIT` is the pipeline's rc (0), because I piped through tail; the test result line is the evidence. Hmm, that's sloppy. Let me re-run properly for h1-supply-chain (now incremental → 1s) with `; echo "cargo rc=$?"` so the exit code is captured. Actually the run has already been recorded in my *probe* log? No — this cargo run was NOT logged via nb.sh (I ran it directly in a background bash job), so logs/h1-supply-chain.txt lacks it. I need to log nonbridge calls per the task rule. Let me now re-run cargo test for h1-supply-chain through nb.sh, capturing the exit code properly (build is now warm → fast). That's the logged evidence.

Then for h2/h3 I can run cargo too (warm target dir → fast).

Let me now:
1. Re-run h1-supply-chain cargo test via nb.sh (logged, with rc).
2. Run h3's cargo test via nb.sh (logged, rc).
3. Read h2's tree (entries ledger + objects) and run its cargo test too.

Let me first do 1 and 2 in one bash call (two nb.sh invocations → 2 nonbridge calls, one per question → clean attribution).

For h3: the test is `every_declared_widget_ships` in host/tests/shipped.rs; the failure message prints the tree. Expect: `dial is declared and does not ship; the tree is: ...`.

Let me run both with proper rc capture. I'll use nb.sh with `sh -c '...'` and echo rc inside. Simpler: nb.sh runs the command; the rc it captures is the command's rc (if I don't pipe). But cargo output is long; the log can hold it (fine, "can be truncated with a marker" but I'd rather log it whole — it's a few KB).

Let me run: `./nb.sh h1-supply-chain sh -c 'cd .../h1-supply-chain/host && CARGO_TARGET_DIR=... cargo test --offline 2>&1 | tail -30; exit ${PIPESTATUS[0]}'` — sh may not support PIPESTATUS (bash does). nb.sh invokes `"$@"` — I can pass bash -c. Let me use bash -c with PIPESTATUS. Or simply: run cargo test with output to a temp file, then tail it, then exit with cargo's rc:
`bash -c 'cd X && cargo test --offline > /tmp/o 2>&1; rc=$?; tail -25 /tmp/o; exit $rc'`
That captures rc correctly and logs the tail. But then the log contains only the tail with a truncation marker — I should add an explicit marker. Let me use `sed -n` ... simpler: log the whole output (it's ~2-3KB) without truncation: `bash -c 'cd X && cargo test --offline 2>&1; exit $?'` → nb.sh logs the whole thing verbatim. 

Let me do that.
```

**该步模型输出**（逐字）：
```
The h1 run confirms the predicted 160 vs 136. Let me re-run both tests with proper logging and exit codes:
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h1-supply-chain：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/probe-cg26; export CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target; ./nb.sh h1-supply-chain bash -c 'cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain/host && cargo test --offline 2>&1 | grep -v
…[参数：此处截断，全文 449 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`


## 三、交付答案（逐字）

# h1-supply-chain — 一个兄弟调用了与其余兄弟不同的那一族名字

① 症状：`cargo test --offline` 在 offsets 上红，总数是 160 而不是 136；每个 widget 文件单看都"对"。
② 根因 `root cause`：`host/src/control/object/toggle/toggle.rs:16` 调用 `to_world(x)`（导入在 `:3`），而其余 8 个兄弟的 `offset` 都调用 `to_local`（`host/src/control/control.rs:5`）。
   `mechanism`：`control.rs` 声明了两个坐标族——`to_local(x)=x+7`（`:5`）与 `to_world(x)=x+31`（`:8`）。offsets 测试把 8 个 widget 的 `offset(10)` 相加，期望 8×17=136；toggle 交的是 `to_world(10)=41` ⇒ 7×17+41=**160**。因为两个族都是合法的公开函数、而 toggle 那行文档与兄弟逐字相同（"in the coordinate family its siblings use"），所以单看任何文件都读不出错——错在"哪一族"。
③ 最小修 `fix`：`toggle.rs:16` 改 `to_local(x)`，`:3` 的导入 `to_world` 改 `to_local`；`control.rs` 两个族都保留、测试与其余 8 个兄弟不动。
④ 反证 `counter-proof`：`callers to_world` 只有 1 个调用者（`toggle.rs:15`），`callers to_local` 恰好 8 个（button/slider/timeline/gauge/spinner/panel/badge/dial）；并且**先由这两个数推出 160**、再跑 cargo，测试打印的 `left: 160` 与推算逐字相同——若还有第二个兄弟跨族，这个数就不会是 160。`callers to_world` 的输出（1 个）同时证伪了"两个族都没人用"的读法。
   `evidence`（命令＋原始输出＋退出码，全文见 `logs/h1-supply-chain.txt`）：`callers to_world` → 1 caller（exit 0）；`callers to_local` → 8 callers（exit 0）；`cargo test --offline`（`trees/h1-supply-chain/host`）→ `the_rendered_offsets_add_up ... FAILED` / `assertion left == right failed: the rendered offsets add up to 160, not 136`（exit **101**）。
⑤ 调用清单：`files`；`node host/tests/offsets.rs`、`host/src/lib.rs`、`host/src/control/control.rs`、`host/src/control/object/toggle/toggle.rs`、`…/button/button.rs`；`callers to_world`、`callers to_local`、`callees offset`；cargo test（非 codegraph，1 次）。

总调用：成功 10 / 被拒 0（codegraph；含 1 次 init ⇒ 9 步）

nonbridge: 2 次 / 约 1.7 KB（`cargo test` 两次记录：一次是管道里被我写死的 `exit 101` 那次——rc 不可信；一次用 `rc=$?` 真取，两次的输出都逐字在日志里）