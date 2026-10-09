# cg · `h3-cross-file-chain`（精简版；完整版见 `../cg/h3-cross-file-chain.md`）

**题面**：hardbug h3：跨文件因果链——入口计划从没点名新面，构建期作用域把它排除。

> 仪器调用 **13** · 步 4 · 输出 4,523 tok · 推理 9,829 字符 · 累计上下文 600,320

## 仪器调用序列（工具给了什么 + 当时 agent 在想什么）

### ①1 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph init .`  · exit 0 · step 48（turn 1）

**工具返回**：
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

**当时推理**：
```
Perfect: `toggle.rs:3` imports `to_world` (vs siblings importing `to_local`), and `toggle.rs:16` calls `to_world(x)`. Same doc comment on both ("in the coordinate family its siblings use") — the toggle's doc is identical to the siblings', so each file "reads as correct on its own" ✓ (the class statement).

Now let me try running the test to get the actual failure output — good evidence and counter-proof. Let me run cargo test --offline in the h1-supply-chain host copy.

Risk: build time. Let me run with a generous timeout and in background if needed. Let me first check if the toolchain builds quickly... Let me just run it with timeoutMs 600000 (10 min) — the harness may move it to background if it exceeds.

Actually, `cargo test --offline` for this host requires building xirang-toolchain (a big crate with proc-macro deps like syn? maybe). The workspace's own build cache is at /home/nich/Moirai_N3/nichlink/target (the workspace target dir) — a DIFFERENT target dir than my probe cargo-target, so it will rebuild from scratch. Could take several minutes. The deps come from the local registry cache (offline OK).

Given the value (only for h1-supply-chain, h3, h2 evidence) — that's 3 questions × maybe 2-5 min = manageable. But if the toolchain build is heavy (it has a proc-macro crate, wasmtime? — plugin_host might pull heavy deps when features are off... default features = build+run, so maybe moderate).

Let me try once for h1-supply-chain and measure. If it's fast enough, I'll 
…[推理：此处截断，全文 2483 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**当时输出**：`Let me start a cargo verification run in the background while I analyze h3:`

**同一步还发了**：`bash`→h1-supply-chain

### ①2 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph files`  · exit 0 · step 49（turn 1）

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
    │   └── shipped.rs (rust, 2 symbols)
    └── build.rs (rust, 2 symbols)
```

**当时推理**：
```
h3-cross-file-chain: symptom = "A widget that exists in the sources does not appear in the shipped registry tree. cargo test --offline fails on the shipped-tree assertion." Class: "the entry plan never names the new face, so the build-time scope leaves it out of the build".

So in the entry plan (host/src/lib.rs's `static_graft_plan!` / or the build-time scope), one face isn't named. Let me read the rest of lib.rs (lines 40-98) — the h1-supply-chain version had 8 cut/graft pairs (button, slider, timeline, gauge, toggle, spinner, panel, badge) — note `dial` is MISSING from that list in h1-supply-chain? Let me check: in h1-supply-chain's lib.rs the plan lists button, slider, timeline, gauge, toggle, spinner, panel, badge = 8 items; the object dir has 9 (badge, button, dial, gauge, panel, slider, spinner, timeline, toggle). So dial is not named in the plan! Hmm — but wait, that's in h1-supply-chain's tree (which per TREES.json is the "one sibling calls a different family of names" class). Hmm, for h1-supply-chain the defect is `toggle`'s to_world; the missing `dial` may be intentional (dial may not be a grafted slot, or the plan may name it elsewhere).

For h3-cross-file-chain, the class says the entry plan never names the *new face* → the face is left out of the build. So in h3's lib.rs, a face (probably `dial`) is missing from the plan. Let me read h3's lib.rs fully (lines 36-98) and its shipped.rs test.

Note that `scope` in build_time collects only faces named by the entry (
…[推理：此处截断，全文 1858 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①3 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph node host/src/lib.rs`  · exit 0 · step 50（turn 1）

**工具返回**：
```
**host/src/lib.rs** — 98 lines, 3 symbols · no other indexed file depends on it

1	//! XiRang 示例：README 里的 Control / Button 两层树，作为一个真实宿主库。
2	//! XiRang example: the README Control/Button two-level tree as a real host
3	//! library.
4	//!
5	//! 整个 crate 只有这里一处构建接线。`host!()` 引入构建期生成的注册计划；
6	//! 注册面代码保持普通 Rust，父级不维护子对象清单。
7	//! The crate has one build wiring point. `host!()` pulls in the plan the build
8	//! step generated; face code stays ordinary Rust and no parent keeps a child
9	//! roster.
10	
11	xirang_toolchain::runtime::host!();
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
27	pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.example.control-button");
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
48	xirang_toolchain::runtime::static_graft_plan!(
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

**当时推理**：
```
h3: The `shipped.rs` test asserts all nine widgets (button, slider, timeline, gauge, toggle, spinner, panel, badge, **dial**) ship. The graft plan in `host/src/lib.rs:48-66` names only 8 cuts — **`dial` is missing** (the "new face"). Per the docs at lib.rs:36-43, the build-time scope narrows to the subtrees the cuts name, so `dial` isn't in the shipped registry tree → the test fails on `dial`.

So root cause: `host/src/lib.rs:48-66` (the `static_graft_plan!`) — specifically the absence of a `cut(crate::control::object::dial::NODE_ID) graft(control_button_graft::dial_fast::NODE_ID)` entry (would belong after :65). The dial face exists in sources (`host/src/control/object/dial/dial.rs`, and `graft/src/dial_fast.rs` exists ✓ — the graft crate has dial_fast.rs).

Minimal fix: add the dial cut/graft line pair to the plan. Files not touched: `host/tests/shipped.rs` (test), the dial face file, graft/dial_fast.rs.

Let me verify the dial face exists and is registered in its own file (crate::control_object! macro) and that the parent's registration is passive (no child roster — the doc says "父级不维护子对象清单" — the macro registers under `parent: crate::control::NODE_ID`).

Evidence via codegraph: 
- `node host/src/control/object/dial/dial.rs` → shows the face + `control_object!` registration.
- `callers dial::NODE_ID`? Let me try `query NODE_ID` and `callers "dial::NODE_ID"` → 0 callers (nothing names it) vs button::NODE_ID → 1 caller (lib.rs). That's excellent graph evidence: compare `call
…[推理：此处截断，全文 2002 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①4 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph files`  · exit 0 · step 51（turn 1）

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
    │   └── shipped.rs (rust, 2 symbols)
    └── build.rs (rust, 2 symbols)
```

**当时推理**：
```
`NODE_ID` isn't indexed as a symbol (it's generated by the `control_object!` macro — codegraph's static parse doesn't see macro-generated items). That's why `callers dial::NODE_ID` fails. So I can't use the graph for the plan-membership claim directly; but I CAN use the graph to enumerate which files contain `control_object!`-like registrations? No, macros aren't expanded.

Alternative evidence: `codegraph node host/src/lib.rs` shows the plan listing 8 cuts (raw source, line numbers) and `node host/tests/shipped.rs` lists 9 names. That's decisive source-level evidence (line-numbered, verbatim).

Let me also check that dial isn't referenced by any other naming path: `query dial` → shows dial.rs symbols; `query "dial_fast"` → graft file. And check the grep-free approach: `codegraph query DialFast`. Also, is there any other file that names the dial face (e.g. a `registrations()` list)? The parent's registrations come from the macro-generated tree; `outline()` uses `base_registry()` → `registrations()` (generated by `host!()`/build step). The build-time scope collects from the entry (lib.rs). Since only the entry's typed paths count, and lib.rs has no `dial`, dial is scoped out.

Hmm — but hold on: does the graft plan's `cut(...)` list actually determine the scope, or is the scope determined by the *whole* crate's source? Per the docs (lib.rs:36-43) and AGENTS.md, the build-time scope narrows to the subtrees named by the entry. The `static_graft_plan!` in the entry names 8 cuts. 
…[推理：此处截断，全文 3664 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**同一步还发了**：`job_output`→装置

### ①5 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph node host/src/lib.rs`  · exit 0 · step 59（turn 1）

**工具返回**：
```
**host/src/lib.rs** — 98 lines, 3 symbols · no other indexed file depends on it

1	//! XiRang 示例：README 里的 Control / Button 两层树，作为一个真实宿主库。
2	//! XiRang example: the README Control/Button two-level tree as a real host
3	//! library.
4	//!
5	//! 整个 crate 只有这里一处构建接线。`host!()` 引入构建期生成的注册计划；
6	//! 注册面代码保持普通 Rust，父级不维护子对象清单。
7	//! The crate has one build wiring point. `host!()` pulls in the plan the build
8	//! step generated; face code stays ordinary Rust and no parent keeps a child
9	//! roster.
10	
11	xirang_toolchain::runtime::host!();
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
27	pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.example.control-button");
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
48	xirang_toolchain::runtime::static_graft_plan!(
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

### ①6 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph node host/tests/shipped.rs`  · exit 0 · step 67（turn 1）

**工具返回**：
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

### ①7 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph node host/build.rs`  · exit 0 · step 72（turn 1）

**工具返回**：
```
**host/build.rs** — 4 lines, 1 symbol · no other indexed file depends on it

1	fn main() {
2	    xirang_toolchain::build_time::run();
3	}
4
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

### ①8 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph node host/src/control/object/dial/dial.rs`  · exit 0 · step 75（turn 1）

**工具返回**：
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

### ①9 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph callers dial::NODE_ID`  · exit 0 · 共享/装置步

**工具返回**：
```
ℹ Symbol "dial::NODE_ID" not found — did you mean: paint, Dial, DialFast?
```

### ①10 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph callers button::NODE_ID`  · exit 0 · 共享/装置步

**工具返回**：
```
ℹ Symbol "button::NODE_ID" not found — did you mean: paint, Button, ButtonFast?
```

### ①11 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph node host/src/control/registry_rule/registry_rule.rs`  · exit 0 · 共享/装置步

**工具返回**：
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

### ①12 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph query dial`  · exit 0 · 共享/装置步

**工具返回**：
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

### ①13 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h3-cross-file-chain && codegraph query dial_fast`  · exit 0 · 共享/装置步

**工具返回**：
```
Search Results for "dial_fast":

file        dial_fast.rs
  graft/src/dial_fast.rs:1
```


## 交付答案（逐字）

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