# cg · `h1-supply-chain`（精简版；完整版见 `../cg/h1-supply-chain.md`）

**题面**：hardbug h1：供应链驳杂——同族一个兄弟调用了另一族名字，症状是偏移 160≠136。

> 仪器调用 **10** · 步 2 · 输出 1,488 tok · 推理 1,320 字符 · 累计上下文 314,240

## 仪器调用序列（工具给了什么 + 当时 agent 在想什么）

### ①1 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph init .`  · exit 0 · step 58（turn 1）

**工具返回**：
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

**当时推理**：
```
h2 confirmed:
- `spinner.rs:19-23` block is missing the 4th declaration `handle_contracts: [crate::control::ControlHandle],` that every other sibling has (button.rs:23). The constraint "every direct child implements ControlHandle" is declared in control.rs:13-16 (the trait) and repeated in every sibling's `control_object!` via `handle_contracts:` — spinner omits it.
- The ledger: panel entry lapsed.
- `callers REGISTRATION_RULE` = 0 (another codegraph resolution gap — the macro presumably references it; not needed here).

Also interesting: in h2, `to_world` is not called by anyone (toggle uses to_local in this tree?). Let me not worry; h2's class is the ledger+declaration.

Wait — one thing to check for h2: is the offsets test passing (yes, 1 passed ✓).

Let me now double-check spinner's missing declaration is *exactly one*: diff showed only that one hunk (plus name substitutions) ✓. And the other 7 siblings match the reference modulo names ✓ (from the diff output: only the handle_contracts hunk for spinner).

Now, do I need to determine the fingerprint scheme more precisely? Let me try one more approach: maybe the fingerprint is over the whole *file path + the registration block* in some canonical way. Instead of guessing, I can state: "两条记录写了同一个指纹，而两个被指名的文件字节不同（06c45576… / d886e9b0…）⇒ 至少一条不描述它指名的字节；panel 那条自己写着'在 panel 被重写之前记录的'（时间 2026-09-20，早于 button 的 2026-10-01）⇒ 失效的是 panel 那条。" And note the caveat: the ledger's fingerprint scheme isn't in this tree, so I compared the t
…[推理：此处截断，全文 6744 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**当时输出**：`Writing the three h-class answers:`

### ①2 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph files`  · exit 0 · step 67（turn 1）

**工具返回**：
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

### ①3 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph node host/tests/offsets.rs`  · exit 0 · step 72（turn 1）

**工具返回**：
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

### ①4 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph node host/src/lib.rs`  · exit 0 · step 75（turn 1）

**工具返回**：
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

### ①5 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph node host/src/control/control.rs`  · exit 0 · 共享/装置步

**工具返回**：
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

### ①6 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph callers to_world`  · exit 0 · 共享/装置步

**工具返回**：
```
Callers of "to_world" (1):

function to_world (rust) — host/src/control/control.rs:8
function    offset
  host/src/control/object/toggle/toggle.rs:15
```

### ①7 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph callers to_local`  · exit 0 · 共享/装置步

**工具返回**：
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

### ①8 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph callees offset`  · exit 0 · 共享/装置步

**工具返回**：
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

### ①9 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph node host/src/control/object/toggle/toggle.rs`  · exit 0 · 共享/装置步

**工具返回**：
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

### ①10 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h1-supply-chain && codegraph node host/src/control/object/button/button.rs`  · exit 0 · 共享/装置步

**工具返回**：
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


## 交付答案（逐字）

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