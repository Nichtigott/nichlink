# cg · `fa`（精简版；完整版见 `../cg/fa.md`）

**题面**：五族：「我要加一个新对象，和现有的差不多。」

> 仪器调用 **19** · 步 12 · 输出 17,344 tok · 推理 41,270 字符 · 累计上下文 3,858,304

## 仪器调用序列（工具给了什么 + 当时 agent 在想什么）

### ①1 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph init .`  · exit 0 · step 27（turn 2）

**工具返回**：
```
┌  Initializing CodeGraph
│
◆  Initialized in /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa
│
Scanning files...
Parsing code...
Resolving refs...
Linking dynamic dispatch...
│
◆  Indexed 14 files
│
●  108 nodes, 167 edges in 506ms
│
└  Done
```

**当时推理**：
```
Now clear:

**fd**: same shape as h1 — two defects: `query.rs:27` `> min` (inverted filter) + `render.rs` missing `write_count` call. Both hit the report target with 3 failing assertions (2 defects) — matches TREES.json "同一 target 内 3 条断言失败 = 2 处缺陷" ✓. Fix both → green. (h1 also had the audit.rs defect, fd doesn't.)

**fe**: `audit.rs:9` `== 0` (wrong) + `query.rs:27` `> min` (wrong) → the fe run showed only 1 failing test: `the_minimum_amount_filter_keeps_the_entries_at_or_above_it` ✓ (the audit defect is invisible on the default face; fe's question "这几个对象都有问题" = several objects have problems → fix both: the filter (red) and the audit count (gray face)). Hmm — "对象" (objects) here means the modules/faces. OK.

Wait, fe's diff vs s1 also shows only those two files (no render.rs difference) ✓ consistent with 1 failing test (filter) + the hidden audit issue.

Note: fd/fe lack `.gitignore` (only in s1) — that's part of their declared hashes ✓ (fine).

**g3** and **fa** have the same file layout (13 files): examples/{graft_record,health_check,tree}.rs, src/control/{object/{button,slider},registry_rule,control.rs}, src/lib.rs, tests/{health_check,ide_mirror,registry,static_plan_allocations}.rs, build.rs.

Hmm, g3 and fa have identical layouts. Are they the same content? g3 = 18e80bbd, fa = 13333fd5 → different. Let me diff them: `diff -r trees/g3 trees/fa`. Let me also read g3's lib.rs + the registry test (33 symbols) to understand the constraints for "adding an internal layer" and
…[推理：此处截断，全文 9423 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①2 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph files`  · exit 0 · step 28（turn 2）

**工具返回**：
```
Project Structure (14 files):

├── examples
│   ├── graft_record.rs (rust, 12 symbols)
│   ├── health_check.rs (rust, 3 symbols)
│   └── tree.rs (rust, 2 symbols)
├── src
│   ├── control
│   │   ├── object
│   │   │   ├── button
│   │   │   │   └── button.rs (rust, 5 symbols)
│   │   │   ├── dial
│   │   │   │   └── dial.rs (rust, 5 symbols)
│   │   │   └── slider
│   │   │       └── slider.rs (rust, 5 symbols)
│   │   ├── registry_rule
│   │   │   └── registry_rule.rs (rust, 3 symbols)
│   │   └── control.rs (rust, 6 symbols)
│   └── lib.rs (rust, 5 symbols)
├── tests
│   ├── health_check.rs (rust, 3 symbols)
│   ├── ide_mirror.rs (rust, 5 symbols)
│   ├── registry.rs (rust, 33 symbols)
│   └── static_plan_allocations.rs (rust, 19 symbols)
└── build.rs (rust, 2 symbols)
```

**当时推理**：
```
Excellent — very informative:

1. **g3 vs fa**: the ONLY difference is `.gitignore` (g3 has one, fa doesn't). Wait — that means g3 and fa trees have identical content modulo `.gitignore`! But their hashes differ (18e80bbd vs 13333fd5) ✓ consistent (the hash includes `.gitignore`).

Hmm, so g3 and fa are the same host (Button + Slider, with runtime_checks on Button).

2. **fa's button.rs** shows the API: `control_object!` with `flow: FlowContract::new(...)`, `runtime_checks: [NON_EMPTY_TEXT]`, and the comment explicitly says: "`runtime_checks` is not part of `NodeId` ... so declaring it must not move identity" — interesting! This is exactly the fa-family's shape: button.rs already has `runtime_checks`.

Hmm — this matters: is this the *clean* state (no toggle) or does it already contain the "added object"? The button.rs has a rich declaration (flow + runtime_checks). The slider.rs probably too. So the fa task ("add a new object similar to the existing ones") = add a third face file (e.g. `toggle.rs`) modeled on button/slider, mounted in control.rs, and declared in the entry plan (if grafted) — plus the tests that enumerate the tree must still pass (they may assert the exact set of paths! e.g. `built_in_tree_has_the_expected_paths_and_derived_sources` and `outline()` — the comment mentions those tests pin the paths).

So adding a NEW object will likely BREAK tests that pin the tree contents (expected paths). Hmm — "fa/... 交付后门必须绿". So the new object must be added in a way that 
…[推理：此处截断，全文 8743 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①3 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph init .`  · exit 0 · step 35（turn 2）

**工具返回**：
```
┌  Initializing CodeGraph
│
◆  Initialized in /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa
│
Scanning files...
Parsing code...
Resolving refs...
Linking dynamic dispatch...
│
◆  Indexed 13 files
│
●  102 nodes, 158 edges in 590ms
│
└  Done
```

**当时推理**：
```
The idiomatic usage (from the toolchain's own test):

```rust
struct ProbePreset;
impl PresetContract for ProbePreset { type Output = ...; const REQUIRED_PARTS: &[&str] = &["probe"]; }
struct ProbeParts;
impl PartsContract for ProbeParts { type Output = ...; const PROVIDED_PARTS: &[&str] = &["probe"]; }

__control_object! {
    ...
    preset: ProbePreset,
    parts: ProbeParts,
    ...
}
```
And `REGISTRATION.preset == "ProbePreset"`, `.parts == "ProbeParts"` (defaults from `stringify!` when `preset_name`/`parts_name` omitted ✓), and `contract.required_parts == ["probe"]`, `.provided_parts == ["probe"]`.

Let me read the full file to get the exact shapes (lines 1-46).

Then implement for fb/g3:

```rust
/// Button 的内部结构：它自己声明的那一层零件（preset 要求什么、parts 供应什么）。
pub struct ButtonPreset;
pub struct ButtonParts;

impl xirang_toolchain::runtime::PresetContract for ButtonPreset {
    type Output = Button;   // hmm, Output must match PartsContract::Output
    const REQUIRED_PARTS: &'static [&'static str] = &["label"];
}
impl xirang_toolchain::runtime::PartsContract for ButtonParts {
    type Output = Button;
    const PROVIDED_PARTS: &'static [&'static str] = &["label"];
}
```
Hmm: can Output = Button (the face type, a unit struct)? `assert_contract::<P, T>` requires `T: PartsContract<Output = P::Output>` → `ButtonPreset::Output == ButtonParts::Output` ✓ (both Button). Is that meaningful? The preset "requires the label part" and parts "supply label", collectively constructing a `Bu
…[推理：此处截断，全文 4398 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**同一步还发了**：`read`→装置

### ①4 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph files`  · exit 0 · step 58（turn 2）

**工具返回**：
```
Project Structure (13 files):

├── examples
│   ├── graft_record.rs (rust, 12 symbols)
│   ├── health_check.rs (rust, 3 symbols)
│   └── tree.rs (rust, 2 symbols)
├── src
│   ├── control
│   │   ├── object
│   │   │   ├── button
│   │   │   │   └── button.rs (rust, 5 symbols)
│   │   │   └── slider
│   │   │       └── slider.rs (rust, 5 symbols)
│   │   ├── registry_rule
│   │   │   └── registry_rule.rs (rust, 3 symbols)
│   │   └── control.rs (rust, 6 symbols)
│   └── lib.rs (rust, 4 symbols)
├── tests
│   ├── health_check.rs (rust, 3 symbols)
│   ├── ide_mirror.rs (rust, 5 symbols)
│   ├── registry.rs (rust, 33 symbols)
│   └── static_plan_allocations.rs (rust, 19 symbols)
└── build.rs (rust, 2 symbols)
```

**当时推理**：
```
Now the last question: **fa** — "我要加一个新对象，和现有的差不多。" (I want to add a new object, similar to the existing ones.)

Plan:
1. Look at the tree's structure: button.rs + slider.rs as models; control.rs (parent); lib.rs (entry + graft plan); tests/registry.rs (pinned rows + counts), tests/static_plan_allocations.rs (counts), examples.
2. Decide: add a third leaf face `Toggle` (or `Dial`):
   - `src/control/object/toggle/toggle.rs` modeled on button.rs.
   - Entry: the new face must be spelled in the entry (lib.rs) to be in the build-time scope. Options: (a) add a `cut(crate::control::object::toggle::NODE_ID) graft(control_button_graft::toggle_fast::NODE_ID)` — requires a graft twin in the carrier; (b) spell the path in code without a graft.
   
   Hmm. Which is right? Let me think about what the framework requires for a *new object* that the app publishes:
   - The entry plan (`static_graft_plan!`) lists graft slots. Not every face must be a slot.
   - The build-time scope comes from the entry's reachable source: "只把代码里拼出的路径当成一次引用" — any code-spelled path reference to the face makes the scanner include it.
   
   Let me test empirically: add the face file + a code-spelled mention in lib.rs, then build and see whether `outline()` includes it. That's the fastest way to learn the rule.

   Then update the tests that pin the tree (registry.rs's 3-row list; maybe static_plan_allocations counts; ide_mirror is ignored).

3. Gate green.

Let me start: read fa's slider.rs (model for a new fa
…[推理：此处截断，全文 2263 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**当时输出**：`Last question: **fa** (add a new object). Reconnaissance of the pinned face set:`

### ①5 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph node src/control/object/button/button.rs`  · exit 0 · step 59（turn 2）

**工具返回**：
```
**src/control/object/button/button.rs** — 41 lines, 2 symbols · used by 1 file: src/control/control.rs

1	//! Button 叶子面：它是 Control 的直接子对象，不再拥有自己的 Registry。
2	//! Button leaf face: a direct child of Control that owns no Registry of its own.
3	
4	use crate::control::{ControlFrame, ControlHandle};
5	use xirang_toolchain::runtime::{ContractId, FlowContract, NON_EMPTY_TEXT};
6	
7	pub struct Button;
8	
9	impl ControlHandle for Button {
10	    fn paint(&self) -> ControlFrame {
11	        ControlFrame
12	    }
13	}
14	
15	crate::control_object! {
16	    kind: Button,
17	    exports: ["control.render"],
18	    parent: crate::control::NODE_ID,
19	    handle_contracts: [crate::control::ControlHandle],
20	    flow: FlowContract::new(
21	        ContractId::new("control.render.v1"),
22	        1,
23	        "ControlInput",
24	        "ControlFrame",
25	    ),
26	    // A real declared check: a Button's label crosses into the renderer as text
27	    // and must not be blank. The kernel cannot observe that label, so the host
28	    // calls `Registry::health_check` at the boundary; the declaration only
29	    // states what "valid" means.
30	    // 一条真实的已声明检查：Button 的标签以文本形式跨入渲染器，不能为空白。内核无法
31	    // 观测该标签，因此由宿主在边界处调用 `Registry::health_check`；声明只说明“有效”
32	    // 的含义。
33	    // `runtime_checks` is not part of `NodeId` (namespace + relative path + kind),
34	    // so declaring it must not move identity; `built_in_tree_has_the_expected_paths_and_derived_sources`
35	    // and `outline()` pin that.
36	    // `runtime_checks` 不是 `NodeId`（命名空间 + 相对路径 + kind）的组成部分，因此声明
37	    // 它不该移动身份；`built_in_tree_has_the_expected_paths_and_derived_sources` 与
38	    // `outline()` 钉住这一点。
39	    runtime_checks: [NON_EMPTY_TEXT],
40	}
41
```

**当时推理**：
```
Very interesting! Line 187 of registry.rs: `assert_eq!(kind_at(&registry, "root/control/toggle"), "Toggle");` — the tests reference a **toggle** face! So the tree's tests already contemplate a toggle face (probably for the out-of-project plugin tests or "external object" tests). Let me look at the context of line 187 and line 122 (the `depth_first` at 122).

Also line 283: `assert_eq!(plan.faces().len(), 3, "every declared face is retained")` — the static plan retains 3 faces (control, button, slider). If I add a new face, this would become 4 (and the test would need updating).

Hmm! Wait: `plan.faces()` — the *build-declared static plan*'s faces. Line 187's `root/control/toggle` — let me read the surrounding test (lines 170-210) to understand.

Let me read registry.rs lines 110-210 to see these tests (the "external/plugin" ones). This tells me whether "toggle" is a face declared *outside* the project (plugin/external object) — in which case fa's answer might be to add a Toggle face *like the existing ones* (button/slider) — hmm.

Let me read that region of fa's registry.rs.
```

### ①6 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph node src/control/control.rs`  · exit 0 · step 60（turn 2）

**工具返回**：
```
**src/control/control.rs** — 40 lines, 4 symbols · used by 2 files: src/control/object/button/button.rs, src/control/object/slider/slider.rs

1	//! Control 文件夹面：它拥有一个 Registry，所有直接子对象都要满足它旁边的规则。
2	//! Control folder face: it owns a Registry, and every direct child must satisfy
3	//! the rule kept beside it.
4	
5	use xirang_toolchain::runtime::{ContractId, FlowContract};
6	
7	/// 父注册面交给子对象的绘制结果。
8	/// The frame a parent face hands to its children for painting.
9	pub struct ControlFrame;
10	
11	/// 每个直接子对象必须实现的接口。
12	/// The interface every direct child must implement.
13	pub trait ControlHandle {
14	    fn paint(&self) -> ControlFrame;
15	}
16	
17	pub struct Control;
18	
19	crate::root_object! {
20	    kind: Control,
21	    needs_registry: true,
22	    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
23	    // 规则不再重复写第二遍：`needs_registry: true` 的面省略 `registry_rule:` 时，
24	    // 规则解析到注册面旁边那份规范规则（`super::registry_rule::REGISTRATION_RULE`）。
25	    // The rule is no longer written twice: a face with `needs_registry: true` that
26	    // omits `registry_rule:` resolves to the canonical rule beside the face
27	    // (`super::registry_rule::REGISTRATION_RULE`).
28	    registry_rule_path: "src/control/registry_rule/registry_rule.rs",
29	    // A folder face may publish its own flow contract, so a `full` cut can
30	    // replace the whole subtree only when the replacement agrees with it.
31	    // 文件夹面也可以发布自己的数据流合同；因此只有替换端与之兼容时，`full`
32	    // 切口才允许换掉整棵子树。
33	    flow: FlowContract::new(
34	        ContractId::new("control.frame.v1"),
35	        1,
36	        "ControlInput",
37	        "ControlFrame",
38	    ),
39	}
40
```

**当时推理**：
```
The `toggle` in line 187 is just a *synthetic* sibling built in the test (a copy of button's REGISTRATION with kind "Toggle"), not a real face. So the tree has no toggle face — consistent with my plan to add one.

Key constraints for adding a face:
- `tests/registry.rs:54-61`: exact 3-row list (must be updated to 4 rows).
- `tests/registry.rs:283`: `plan.faces().len() == 3` (the static plan's faces) — must become 4 if the new face lands in the build-declared static plan.
- `tests/static_plan_allocations.rs:189`: `static_cuts.len()` — unaffected if I don't add a cut.
- `tests/registry.rs:72-91`: the preset/parts test (control/slider/button) — I may leave it (my new face omits preset/parts → defaults ✓) but the doc says "three built-in faces" — hmm, if my new face has a custom `handle`... Let me keep the new face's declaration minimal and consistent (no preset/parts, so it records NoPreset/NoParts like the others ✓). The test only asserts for control/button/slider ✓ (my new face isn't asserted, so it passes). Its doc comment says "All three built-in faces take that shape" → with 4 faces it's stale; I could update the comment (minor, honest).

Now: does the new face get into the static plan automatically (from the collector/scan) or only if the entry spells it? Let me test empirically:
1. Create `src/control/object/toggle/toggle.rs` modeled on slider.rs (kind Toggle, exports, parent control::NODE_ID, handle_contracts, flow).
2. Add a code-spelled mention in the entry? Let me fir
…[推理：此处截断，全文 4035 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①7 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph node tests/static_plan_allocations.rs`  · exit 0 · step 63（turn 2）

**工具返回**：
```
**tests/static_plan_allocations.rs** — 310 lines, 14 symbols · no other indexed file depends on it

1	//! Allocation counts for the release read path, measured instead of argued.
2	//! 发布读路径的分配计数：实测，而不是论证。
3	
4	// `README.md`'s cost table makes three claims this file settles:
5	// `README.md` 的成本表有三条断言由本文件定案：
6	//
7	// - "Read-only built-in topology | Static `StaticFace` slice | No startup
8	//   allocation": every read of the built-in plan must allocate nothing at all.
9	// - "只读内置拓扑 | 静态 `StaticFace` 切片 | 启动时零分配"：对内置计划的每次读取都必须
10	//   完全不分配。
11	// - "Build-declared static graft | Static selector slice inside `StaticPlan` |
12	//   Reading the declaration allocates nothing": same, for `grafts()`.
13	// - "构建期声明的静态 graft | `StaticPlan` 内的静态选择器切片 | 读该声明不分配"：同上，
14	//   针对 `grafts()`。
15	// - "`overlay_static` allocates no plan but still performs contract, admission,
16	//   and connector validation once": the honest reading, recorded in
17	//   `docs/audit-graft-vs-readme.md` as C17, is "allocates no *plan*" rather than
18	//   "allocates nothing" — so this file measures the static overlay against the
19	//   dynamic one over the same cut rather than asserting zero.
20	// - "`overlay_static` 不分配计划，但仍做一次合同/接纳/连接器校验"：诚实的读法（记在
21	//   `docs/audit-graft-vs-readme.md` 的 C17）是"不分配**计划**"，而不是"什么都不分配"——
22	//   因此本文件不假设零，而是在同一个切口上把静态 overlay 与动态 overlay 对比测量。
23	//
24	// Why a counting global allocator rather than a profiler: the claim is a count of
25	// heap allocations on one code path, and `dhat` or `valgrind` are not installed
26	// here. A counting allocator measures exactly that quantity and needs no external
27	// tool. It is process-global, which is why this file holds only two tests: they
28	// run in parallel with each other in the same binary, so each one brackets its
29	// measurement and asserts on it immediately, and the second test's assertion is a
30	// comparison of two measurements taken under the same conditions.
31	// 为什么用计数式全局分配器而不是 profiler：这条断言就是某条代码路径上堆分配的次数，而本环境
32	// 没有装 `dhat` 或 `valgrind`。计数分配器测量的正是这个量，且不需要外部工具。它是进程全局的，
33	// 因此本文件只放两条测试：它们在同一二进制里彼此并行，所以每条都用自己的括号夹住测量并立刻断言，
34	// 第二条断言的还是同条件下两次测量的对比。
35	
36	use std::alloc::{GlobalAlloc, Layout, System};
37	use std::sync::atomic::{AtomicUsize, Ordering};
38	
39	use control_button::{FRAMEWORK, base_registry, builtin_static_plan};
40	use xirang_toolchain::runtime::registry_core::{GraftPlan, Registry};
41	
42	/// Number of successful allocations since the last reset.
43	/// 自上次重置以来成功分配的次数。
44	static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
45	
46	/// Bytes of those allocations, summed.
47	/// 这些分配的字节数之和。
48	static BYTES: AtomicUsize = AtomicUsize::new(0);
49	
50	/// A `System` allocator that counts successful allocations.
51	/// 一个统计成功分配次数的 `System` 分配器。
52	struct Counting;
53	
54	// SAFETY: every method forwards to `System` unchanged and only observes the
55	// layout size it was given; no allocation decision depends on the counters.
56	// `dealloc` and `realloc` are not counted, because the question is how many
57	// allocations a path performs, not how many bytes it holds at the end. A
58	// `realloc` counts as one more allocation, which is what a growing `Vec` does.
59	// 安全性：每个方法都原样转发给 `System`，只观察它收到的布局大小；没有任何分配决定依赖这些
60	// 计数器。`dealloc` 与 `realloc` 不计数，因为问题是一条路径做了多少次分配，而不是它最后占多少
61	// 字节。`realloc` 计作一次新的分配，增长中的 `Vec` 正是如此。
62	unsafe impl GlobalAlloc for Counting {
63	    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
64	        let pointer = unsafe { System.alloc(layout) };
65	        if !pointer.is_null() {
66	            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
67	            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
68	        }
69	        pointer
70	    }
71	
72	    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
73	        let pointer = unsafe { System.alloc_zeroed(layout) };
74	        if !pointer.is_null() {
75	            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
76	            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
77	        }
78	        pointer
79	    }
80	
81	    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
82	        unsafe { System.dealloc(pointer, layout) }
83	    }
84	
85	    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
86	        let moved = unsafe { System.realloc(pointer, layout, new_size) };
87	        if !moved.is_null() {
88	            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
89	            BYTES.fetch_add(new_size, Ordering::Relaxed);
90	        }
91	        moved
92	    }
93	}
94	
95	#[global_allocator]
96	static ALLOCATOR: Counting = Counting;
97	
98	/// Allocations and bytes performed while `body` runs, as a pair.
99	/// `body` 运行期间发生的分配次数与字节数，成对给出。
100	fn measure<T>(body: impl FnOnce() -> T) -> (usize, usize, T) {
101	    let allocations = ALLOCATIONS.load(Ordering::SeqCst);
102	    let bytes = BYTES.load(Ordering::SeqCst);
103	    let value = body();
104	    (
105	        ALLOCATIONS.load(Ord
…[工具返回：此处截断，全文 14805 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**当时推理**：
```
The new face is being published (it changed the tree), and 4 registry tests fail:
1. `built_in_tree_has_the_expected_paths_and_derived_sources` — the 3-row list (needs the 4th row) ✓ expected.
2. `static_plan_carries_faces_and_the_declared_graft` — likely asserts faces().len() == 3 (update to 4).
3. `the_declared_slots_define_the_build_time_scope` — the scope test (probably lists the scoped faces; needs the toggle).
4. `parent_rule_admits_a_new_kind_that_satisfies_it` — this one registers a synthetic "Toggle" sibling with registry_name "toggle" → now collides with my real toggle face's slot name! The test asserts `kind_at(&registry, "root/control/toggle")` == "Toggle" — with my real Toggle face at path `root/control/toggle`, the synthetic sibling (registry_name "toggle") would be rejected as a duplicate slot name → the test fails.

Hmm! #4 is interesting: my new face's registry_name defaults to the kind's snake case = "toggle" — colliding with the test's synthetic face. The test's synthetic sibling was designed when no real "toggle" existed. Now that a real Toggle exists, that test needs a different synthetic name (e.g. "dial") — a legitimate test update (the test's point is "the parent rule is not a kind filter: another kind that satisfies the structure may enter" — any name works).

So I need to update 4 spots in registry.rs. Let me look at the failing assertions:
- #2 `static_plan_carries_faces_and_the_declared_graft` (around line 283: `plan.faces().len() == 3`) → 4.
- #3 
…[推理：此处截断，全文 1954 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①8 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph node src/lib.rs`  · exit 0 · step 64（turn 2）

**工具返回**：
```
**src/lib.rs** — 86 lines, 3 symbols · used by 2 files: tests/registry.rs, tests/static_plan_allocations.rs

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
54	);
55	
56	/// 按框架和包命名空间装配这个示例的注册机。
57	/// Assemble the example's registry from its framework and package namespace.
58	pub fn base_registry() -> Registry {
59	    let mut registry = Registry::root_for_namespace(FRAMEWORK, env!("CARGO_PKG_NAME"));
60	    registry
61	        .register_all(&registrations())
62	        .expect("example faces register");
63	    registry
64	}
65	
66	/// 打印注册树的逻辑路径，供示例二进制和集成测试共用。
67	/// Print the registry tree's logical paths; shared by the example binary and the
68	/// integration tests.
69	pub fn outline() -> Vec<String> {
70	    let registry = base_registry();
71	    let mut rows = registry
72	        .depth_first()
73	        .iter()
74	        .map(|info| {
75	            format!(
76	                "{}  kind={}  source={}",
77	                registry.path_for(info.id).unwrap_or_default(),
78	                info.kind,
79	                info.source.file
80	            )
81	        })
82	        .collect::<Vec<_>>();
83	    rows.sort();
84	    rows
85	}
86
```

**当时推理**：
```
Now I understand all 4 failures:

1. `built_in_tree_has_the_expected_paths_and_derived_sources` (line 54-61): rows must include the toggle row ✓ (my new face ships — proof it worked!).
2. `static_plan_carries_faces_and_the_declared_graft` (line 283): `plan.faces().len()` 3 → 4.
3. `the_declared_slots_define_the_build_time_scope` (line 368): asserts the scope table has exactly 2 slots; now 3 nodes are listed (button, slider, toggle). Let me read that test (lines ~345-380) to update it correctly — it asserts "the entry declares exactly two slots" and compares a rendered table.
4. `parent_rule_admits_a_new_kind_that_satisfies_it` (line 176-188): the synthetic sibling named "toggle" now collides with my real toggle face. Fix: rename the synthetic to a kind/name not present (e.g. "Dial"/"dial") and assert at `root/control/dial`.

Let me read the scope test (lines 345-380) to patch it.

Note the scope table output includes `# selected 3` and the three rows with NodeIds — that's the *scope* (build-time reachable faces). Let me see the assertion.

Let me print lines 344-395.
```

### ①9 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph node tests/registry.rs`  · exit 0 · step 66（turn 2）

**工具返回**：
```
**tests/registry.rs** — 766 lines, 28 symbols · no other indexed file depends on it

1	//! 集成测试：把 README 那棵 Control / Button 树当成真实宿主，逐项验证构建期
2	//! 生成、注册、父级规则、graft、项目外实现和发布态静态计划。
3	//! Integration tests: treat the README Control/Button tree as a real host and
4	//! verify the generated plan, registration, the parent rule, grafting, the
5	//! out-of-project implementation, and the release-time static plan.
6	
7	use control_button::{FRAMEWORK, base_registry, builtin_static_plan};
8	use control_button_graft::FRAMEWORK as GRAFT_FRAMEWORK;
9	use xirang_toolchain::runtime::registry_core::lexicon;
10	use xirang_toolchain::runtime::registry_core::{
11	    FrameworkId, GraftPlan, NodeId, OwnedFlowContract, PluginManifest, PluginMode, PluginSource,
12	    PluginTrustError, PluginTrustPolicy, Registry, StaticGraftCut,
13	};
14	use xirang_toolchain::runtime::{
15	    GraftPlanDocument, RecordReport, apply_recorded_grafts, graft_record_root,
16	};
17	
18	/// 读取某一逻辑路径上的 kind；找不到就失败。
19	/// Read the kind at a logical path, failing when the path is absent.
20	fn kind_at(registry: &Registry, wanted: &str) -> String {
21	    registry
22	        .depth_first()
23	        .iter()
24	        .find(|info| registry.path_for(info.id).as_deref() == Some(wanted))
25	        .map(|info| info.kind.clone())
26	        .unwrap_or_else(|| panic!("no registration face at `{wanted}`"))
27	}
28	
29	fn slot_plan() -> GraftPlan {
30	    GraftPlan::command(FRAMEWORK, "cut root/control/button graft button_fast")
31	        .expect("the graft command parses")
32	}
33	
34	/// `source` 现在由声明点推导，但必须与旧方案注入的值完全相同。
35	/// `source` is derived at the declaration site now, and must equal what the
36	/// previous design injected.
37	#[test]
38	fn built_in_tree_has_the_expected_paths_and_derived_sources() {
39	    let registry = base_registry();
40	    let mut rows = registry
41	        .depth_first()
42	        .iter()
43	        .map(|info| {
44	            format!(
45	                "{} kind={} source={}",
46	                registry.path_for(info.id).unwrap_or_default(),
47	                info.kind,
48	                info.source.portable_file()
49	            )
50	        })
51	        .collect::<Vec<_>>();
52	    rows.sort();
53	
54	    assert_eq!(
55	        rows,
56	        [
57	            "root/control kind=Control source=control/control.rs",
58	            "root/control/button kind=Button source=control/object/button/button.rs",
59	            "root/control/slider kind=Slider source=control/object/slider/slider.rs",
60	        ]
61	    );
62	}
63	
64	/// 自定义 `handle` 且省略 `preset`/`parts` 的面必须记录朴素的默认名，而不是展开后的
65	/// `"$crate :: NoPreset"` 字面 token；三个内置面都是这个形态，因此这条断言钉住的是实际
66	/// 出厂的那条路径，而不只是单元测试里的形状。
67	/// A face with a custom `handle` and an omitted `preset`/`parts` must record the
68	/// plain default names, not the expanded `"$crate :: NoPreset"` literal tokens.
69	/// All three built-in faces take that shape, so this pins the shipped path, not
70	/// only a shape in a unit test.
71	#[test]
72	fn handle_faces_record_plain_default_preset_and_parts_names() {
73	    assert_eq!(control_button::control::REGISTRATION.preset, "NoPreset");
74	    assert_eq!(control_button::control::REGISTRATION.parts, "NoParts");
75	    assert_eq!(
76	        control_button::control::object::button::REGISTRATION.preset,
77	        "NoPreset"
78	    );
79	    assert_eq!(
80	        control_button::control::object::button::REGISTRATION.parts,
81	        "NoParts"
82	    );
83	    assert_eq!(
84	        control_button::control::object::slider::REGISTRATION.preset,
85	        "NoPreset"
86	    );
87	    assert_eq!(
88	        control_button::control::object::slider::REGISTRATION.parts,
89	        "NoParts"
90	    );
91	}
92	
93	/// 叶子面以自身名字载入，公开模块路径和类型名都不变。
94	/// A leaf face loads under its own name, so its public module path and type
95	/// name are unchanged.
96	#[test]
97	fn leaf_face_module_path_stays_clean() {
98	    assert_eq!(
99	        std::any::type_name::<control_button::control::object::button::Button>(),
100	        "control_button::control::object::button::Button"
101	    );
102	}
103	
104	/// 文件夹面的条目仍按原有路径可寻址（同名子模块 + 重导出）。
105	/// A folder face's items stay addressable at their original path through the
106	/// same-named child module and its re-export.
107	#[test]
108	fn folder_face_items_stay_reachable() {
109	    let _: NodeId = control_button::control::NODE_ID;
110	    let _ = std::marker::PhantomData::<control_button::control::Control>;
111	    let _ = control_button::control::ControlFrame;
112	}
113	
114	/// 面文件以 `//!` 开头（见 `src/control/control.rs`）仍然编译，并且源码路径
115	/// 就是仓库相对路径，而不是任何构建副本。
116	/// Face files keep their `//!` header (see `src/control/control.rs`), and the
117	/// recorded source is the repository-relative path rather than a build copy.
118	#[test]
119	fn face_sources_point_at_the_real_files() {
120	    let registry = base_registry();
121	    let sources = registry
122	        .depth_first()
123	        .iter()
124	        .map(
…[工具返回：此处截断，全文 32342 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**当时推理**：
```
fa's gate is green ✓ (rc=0, 25 registry tests pass).

Now let me finish fa with:
1. `cargo test --offline --all-features` → confirm green.
2. `cargo run --example tree` → show the 4-row tree (the new object ships) — good evidence.
3. Log the new face's source via codegraph `node` (post-change evidence) + re-init fa? The index lags; `node src/control/object/toggle/toggle.rs` should work (the watcher indexes new files ~1s). Let me sleep 2 and call it.

Then write fa's answer.

Let me also double check the fa tree's Cargo.toml/graft: no changes needed ✓ (gate green).

Let me run these.
```

### ①10 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph node src/control/object/slider/slider.rs`  · exit 0 · step 67（turn 2）

**工具返回**：
```
**src/control/object/slider/slider.rs** — 28 lines, 2 symbols · used by 1 file: src/control/control.rs

1	//! Slider 叶子面：Control 的第二个直接子对象，用来演示同一父级下的兄弟区间替换。
2	//! Slider leaf face: Control's second direct child, used to demonstrate a graft
3	//! over a contiguous range of siblings under one parent.
4	
5	use crate::control::{ControlFrame, ControlHandle};
6	use xirang_toolchain::runtime::{ContractId, FlowContract};
7	
8	pub struct Slider;
9	
10	impl ControlHandle for Slider {
11	    fn paint(&self) -> ControlFrame {
12	        ControlFrame
13	    }
14	}
15	
16	crate::control_object! {
17	    kind: Slider,
18	    exports: ["control.render"],
19	    parent: crate::control::NODE_ID,
20	    handle_contracts: [crate::control::ControlHandle],
21	    flow: FlowContract::new(
22	        ContractId::new("control.render.v1"),
23	        1,
24	        "ControlInput",
25	        "ControlFrame",
26	    ),
27	}
28
```

**当时推理**：
```
The tree now has 4 rows including `root/control/toggle kind=Toggle source=control/object/toggle/toggle.rs` ✓ — the new object ships, and the gate is green (default + all-features) ✓.

But the codegraph index doesn't know the new file yet ("No indexed file matches..."). The watcher may need a `sync` or a moment; or the tree copy's index needs a re-sync. Let me run `codegraph sync .` (or re-init) to index the new file, then `node` it (for the log evidence).

Note: this is a good honest detail — the index lags the new file; a `sync` fixes it. Let me run `./cg.sh fa sync .` then `node src/control/object/toggle/toggle.rs`.

Then write fa's answer.

fa's answer content:
① 题面（用户那一句）："我要加一个新对象，和现有的差不多。"
② 做了什么：新增第三个叶子对象 `Toggle`，与现有 Button/Slider 同形：
   - 新文件 `src/control/object/toggle/toggle.rs`（`Toggle` + `impl ControlHandle` + `crate::control_object!{ kind: Toggle, exports: ["control.render"], parent: crate::control::NODE_ID, handle_contracts: [ControlHandle], flow: FlowContract::new(ContractId::new("control.render.v1"),1,"ControlInput","ControlFrame") }`）。
   - 入口 `src/lib.rs`：新增 `pub const PUBLISHED_OBJECT_FACES: [NodeId; 3]`，在代码里拼出三个对象面的路径——构建期作用域从入口的可达源码收窄，而那次扫描忽略 import/字符串/注释（这一步是"对象真的会被发布"的关键；不加就只是多了一个文件）。
③ 依据：现有两个面（`button.rs`/`slider.rs`）逐字作模板；父级 `control.rs:19-39` 的 `needs_registry`/规则；作用域规则见 `src/lib.rs` 自己的文档（"没被声明的注册面不会被这个应用发布"）；可达性扫描忽略 import/字符串/注释——因此我写的是表达式而不是 `use`。
④ 改动清单：新增 1 文件 + `src/lib.rs` 的 const；测试期望更新 4 处（`tests/registry.rs`：树行表 3→4、`plan.faces().len()`
…[推理：此处截断，全文 2123 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①11 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph node tests/registry.rs`  · exit 0 · step 68（turn 2）

**工具返回**：
```
**tests/registry.rs** — 766 lines, 28 symbols · no other indexed file depends on it

1	//! 集成测试：把 README 那棵 Control / Button 树当成真实宿主，逐项验证构建期
2	//! 生成、注册、父级规则、graft、项目外实现和发布态静态计划。
3	//! Integration tests: treat the README Control/Button tree as a real host and
4	//! verify the generated plan, registration, the parent rule, grafting, the
5	//! out-of-project implementation, and the release-time static plan.
6	
7	use control_button::{FRAMEWORK, base_registry, builtin_static_plan};
8	use control_button_graft::FRAMEWORK as GRAFT_FRAMEWORK;
9	use xirang_toolchain::runtime::registry_core::lexicon;
10	use xirang_toolchain::runtime::registry_core::{
11	    FrameworkId, GraftPlan, NodeId, OwnedFlowContract, PluginManifest, PluginMode, PluginSource,
12	    PluginTrustError, PluginTrustPolicy, Registry, StaticGraftCut,
13	};
14	use xirang_toolchain::runtime::{
15	    GraftPlanDocument, RecordReport, apply_recorded_grafts, graft_record_root,
16	};
17	
18	/// 读取某一逻辑路径上的 kind；找不到就失败。
19	/// Read the kind at a logical path, failing when the path is absent.
20	fn kind_at(registry: &Registry, wanted: &str) -> String {
21	    registry
22	        .depth_first()
23	        .iter()
24	        .find(|info| registry.path_for(info.id).as_deref() == Some(wanted))
25	        .map(|info| info.kind.clone())
26	        .unwrap_or_else(|| panic!("no registration face at `{wanted}`"))
27	}
28	
29	fn slot_plan() -> GraftPlan {
30	    GraftPlan::command(FRAMEWORK, "cut root/control/button graft button_fast")
31	        .expect("the graft command parses")
32	}
33	
34	/// `source` 现在由声明点推导，但必须与旧方案注入的值完全相同。
35	/// `source` is derived at the declaration site now, and must equal what the
36	/// previous design injected.
37	#[test]
38	fn built_in_tree_has_the_expected_paths_and_derived_sources() {
39	    let registry = base_registry();
40	    let mut rows = registry
41	        .depth_first()
42	        .iter()
43	        .map(|info| {
44	            format!(
45	                "{} kind={} source={}",
46	                registry.path_for(info.id).unwrap_or_default(),
47	                info.kind,
48	                info.source.portable_file()
49	            )
50	        })
51	        .collect::<Vec<_>>();
52	    rows.sort();
53	
54	    assert_eq!(
55	        rows,
56	        [
57	            "root/control kind=Control source=control/control.rs",
58	            "root/control/button kind=Button source=control/object/button/button.rs",
59	            "root/control/slider kind=Slider source=control/object/slider/slider.rs",
60	        ]
61	    );
62	}
63	
64	/// 自定义 `handle` 且省略 `preset`/`parts` 的面必须记录朴素的默认名，而不是展开后的
65	/// `"$crate :: NoPreset"` 字面 token；三个内置面都是这个形态，因此这条断言钉住的是实际
66	/// 出厂的那条路径，而不只是单元测试里的形状。
67	/// A face with a custom `handle` and an omitted `preset`/`parts` must record the
68	/// plain default names, not the expanded `"$crate :: NoPreset"` literal tokens.
69	/// All three built-in faces take that shape, so this pins the shipped path, not
70	/// only a shape in a unit test.
71	#[test]
72	fn handle_faces_record_plain_default_preset_and_parts_names() {
73	    assert_eq!(control_button::control::REGISTRATION.preset, "NoPreset");
74	    assert_eq!(control_button::control::REGISTRATION.parts, "NoParts");
75	    assert_eq!(
76	        control_button::control::object::button::REGISTRATION.preset,
77	        "NoPreset"
78	    );
79	    assert_eq!(
80	        control_button::control::object::button::REGISTRATION.parts,
81	        "NoParts"
82	    );
83	    assert_eq!(
84	        control_button::control::object::slider::REGISTRATION.preset,
85	        "NoPreset"
86	    );
87	    assert_eq!(
88	        control_button::control::object::slider::REGISTRATION.parts,
89	        "NoParts"
90	    );
91	}
92	
93	/// 叶子面以自身名字载入，公开模块路径和类型名都不变。
94	/// A leaf face loads under its own name, so its public module path and type
95	/// name are unchanged.
96	#[test]
97	fn leaf_face_module_path_stays_clean() {
98	    assert_eq!(
99	        std::any::type_name::<control_button::control::object::button::Button>(),
100	        "control_button::control::object::button::Button"
101	    );
102	}
103	
104	/// 文件夹面的条目仍按原有路径可寻址（同名子模块 + 重导出）。
105	/// A folder face's items stay addressable at their original path through the
106	/// same-named child module and its re-export.
107	#[test]
108	fn folder_face_items_stay_reachable() {
109	    let _: NodeId = control_button::control::NODE_ID;
110	    let _ = std::marker::PhantomData::<control_button::control::Control>;
111	    let _ = control_button::control::ControlFrame;
112	}
113	
114	/// 面文件以 `//!` 开头（见 `src/control/control.rs`）仍然编译，并且源码路径
115	/// 就是仓库相对路径，而不是任何构建副本。
116	/// Face files keep their `//!` header (see `src/control/control.rs`), and the
117	/// recorded source is the repository-relative path rather than a build copy.
118	#[test]
119	fn face_sources_point_at_the_real_files() {
120	    let registry = base_registry();
121	    let sources = registry
122	        .depth_first()
123	        .iter()
124	        .map(
…[工具返回：此处截断，全文 32342 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**当时推理**：
```
The sync indexed the new file ✓ and `node` shows it (27 lines, 2 symbols; "used by 1 file: src/control/control.rs" — hmm, interesting: the tool says the new file is used by control.rs, which is the *generated* module mount).

fa's counts: 19 codegraph calls (19 ok, 0 refused), 2 init → 17 steps; nonbridge 5 / 16,159 B; codegraph output bytes 189,725 (inflated by my big reads of registry.rs/static_plan_allocations.rs during exploration).

Hmm — the fa log's codegraph output is huge (63 KB earlier, now 189 KB?) because I piped large file dumps. That's fine; it's honest.

Wait: cg_out_bytes jumped from 63,745 to 189,725 — because the `sync .` output? No... the `node tests/registry.rs` calls (766 lines each, several times) and `static_plan_allocations.rs`. Yes, big.

Now let me write fa's answer, then finish with:
- Adding `总调用：成功 N / 被拒 M` to the 15 earlier answers + fd/fe/r1-r4/h1 (those already have it? Let me check: r1-r4, h1, fd, fe, fb, g3 answers — I wrote "总调用：" for fb and g3 ✓; r1-r4/h1/fd/fe: I didn't add it! Let me check my earlier r1 answer... I ended with "nonbridge: ..." and no 总调用 line. Right: the captain asked to add `总调用：成功 N / 被拒 M` to all.
- Update SETUP.md §5 (byte rule + .txt deviation + 总调用 definition) and APPARATUS.md/PARTIAL.md (causal correction).
- Recompute totals and report.

Let me write fa's answer first.

fa 总调用 line: 成功 19 / 被拒 0（codegraph；含 2 次 init ⇒ 17 步）; nonbridge 5 / 16,159 B.

Let me write it.
```

### ①12 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph node tests/static_plan_allocations.rs`  · exit 0 · step 72（turn 2）

**工具返回**：
```
**tests/static_plan_allocations.rs** — 310 lines, 14 symbols · no other indexed file depends on it

1	//! Allocation counts for the release read path, measured instead of argued.
2	//! 发布读路径的分配计数：实测，而不是论证。
3	
4	// `README.md`'s cost table makes three claims this file settles:
5	// `README.md` 的成本表有三条断言由本文件定案：
6	//
7	// - "Read-only built-in topology | Static `StaticFace` slice | No startup
8	//   allocation": every read of the built-in plan must allocate nothing at all.
9	// - "只读内置拓扑 | 静态 `StaticFace` 切片 | 启动时零分配"：对内置计划的每次读取都必须
10	//   完全不分配。
11	// - "Build-declared static graft | Static selector slice inside `StaticPlan` |
12	//   Reading the declaration allocates nothing": same, for `grafts()`.
13	// - "构建期声明的静态 graft | `StaticPlan` 内的静态选择器切片 | 读该声明不分配"：同上，
14	//   针对 `grafts()`。
15	// - "`overlay_static` allocates no plan but still performs contract, admission,
16	//   and connector validation once": the honest reading, recorded in
17	//   `docs/audit-graft-vs-readme.md` as C17, is "allocates no *plan*" rather than
18	//   "allocates nothing" — so this file measures the static overlay against the
19	//   dynamic one over the same cut rather than asserting zero.
20	// - "`overlay_static` 不分配计划，但仍做一次合同/接纳/连接器校验"：诚实的读法（记在
21	//   `docs/audit-graft-vs-readme.md` 的 C17）是"不分配**计划**"，而不是"什么都不分配"——
22	//   因此本文件不假设零，而是在同一个切口上把静态 overlay 与动态 overlay 对比测量。
23	//
24	// Why a counting global allocator rather than a profiler: the claim is a count of
25	// heap allocations on one code path, and `dhat` or `valgrind` are not installed
26	// here. A counting allocator measures exactly that quantity and needs no external
27	// tool. It is process-global, which is why this file holds only two tests: they
28	// run in parallel with each other in the same binary, so each one brackets its
29	// measurement and asserts on it immediately, and the second test's assertion is a
30	// comparison of two measurements taken under the same conditions.
31	// 为什么用计数式全局分配器而不是 profiler：这条断言就是某条代码路径上堆分配的次数，而本环境
32	// 没有装 `dhat` 或 `valgrind`。计数分配器测量的正是这个量，且不需要外部工具。它是进程全局的，
33	// 因此本文件只放两条测试：它们在同一二进制里彼此并行，所以每条都用自己的括号夹住测量并立刻断言，
34	// 第二条断言的还是同条件下两次测量的对比。
35	
36	use std::alloc::{GlobalAlloc, Layout, System};
37	use std::sync::atomic::{AtomicUsize, Ordering};
38	
39	use control_button::{FRAMEWORK, base_registry, builtin_static_plan};
40	use xirang_toolchain::runtime::registry_core::{GraftPlan, Registry};
41	
42	/// Number of successful allocations since the last reset.
43	/// 自上次重置以来成功分配的次数。
44	static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
45	
46	/// Bytes of those allocations, summed.
47	/// 这些分配的字节数之和。
48	static BYTES: AtomicUsize = AtomicUsize::new(0);
49	
50	/// A `System` allocator that counts successful allocations.
51	/// 一个统计成功分配次数的 `System` 分配器。
52	struct Counting;
53	
54	// SAFETY: every method forwards to `System` unchanged and only observes the
55	// layout size it was given; no allocation decision depends on the counters.
56	// `dealloc` and `realloc` are not counted, because the question is how many
57	// allocations a path performs, not how many bytes it holds at the end. A
58	// `realloc` counts as one more allocation, which is what a growing `Vec` does.
59	// 安全性：每个方法都原样转发给 `System`，只观察它收到的布局大小；没有任何分配决定依赖这些
60	// 计数器。`dealloc` 与 `realloc` 不计数，因为问题是一条路径做了多少次分配，而不是它最后占多少
61	// 字节。`realloc` 计作一次新的分配，增长中的 `Vec` 正是如此。
62	unsafe impl GlobalAlloc for Counting {
63	    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
64	        let pointer = unsafe { System.alloc(layout) };
65	        if !pointer.is_null() {
66	            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
67	            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
68	        }
69	        pointer
70	    }
71	
72	    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
73	        let pointer = unsafe { System.alloc_zeroed(layout) };
74	        if !pointer.is_null() {
75	            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
76	            BYTES.fetch_add(layout.size(), Ordering::Relaxed);
77	        }
78	        pointer
79	    }
80	
81	    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
82	        unsafe { System.dealloc(pointer, layout) }
83	    }
84	
85	    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
86	        let moved = unsafe { System.realloc(pointer, layout, new_size) };
87	        if !moved.is_null() {
88	            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
89	            BYTES.fetch_add(new_size, Ordering::Relaxed);
90	        }
91	        moved
92	    }
93	}
94	
95	#[global_allocator]
96	static ALLOCATOR: Counting = Counting;
97	
98	/// Allocations and bytes performed while `body` runs, as a pair.
99	/// `body` 运行期间发生的分配次数与字节数，成对给出。
100	fn measure<T>(body: impl FnOnce() -> T) -> (usize, usize, T) {
101	    let allocations = ALLOCATIONS.load(Ordering::SeqCst);
102	    let bytes = BYTES.load(Ordering::SeqCst);
103	    let value = body();
104	    (
105	        ALLOCATIONS.load(Ord
…[工具返回：此处截断，全文 14805 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①13 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph node tests/registry.rs`  · exit 0 · step 75（turn 2）

**工具返回**：
```
**tests/registry.rs** — 766 lines, 28 symbols · no other indexed file depends on it

1	//! 集成测试：把 README 那棵 Control / Button 树当成真实宿主，逐项验证构建期
2	//! 生成、注册、父级规则、graft、项目外实现和发布态静态计划。
3	//! Integration tests: treat the README Control/Button tree as a real host and
4	//! verify the generated plan, registration, the parent rule, grafting, the
5	//! out-of-project implementation, and the release-time static plan.
6	
7	use control_button::{FRAMEWORK, base_registry, builtin_static_plan};
8	use control_button_graft::FRAMEWORK as GRAFT_FRAMEWORK;
9	use xirang_toolchain::runtime::registry_core::lexicon;
10	use xirang_toolchain::runtime::registry_core::{
11	    FrameworkId, GraftPlan, NodeId, OwnedFlowContract, PluginManifest, PluginMode, PluginSource,
12	    PluginTrustError, PluginTrustPolicy, Registry, StaticGraftCut,
13	};
14	use xirang_toolchain::runtime::{
15	    GraftPlanDocument, RecordReport, apply_recorded_grafts, graft_record_root,
16	};
17	
18	/// 读取某一逻辑路径上的 kind；找不到就失败。
19	/// Read the kind at a logical path, failing when the path is absent.
20	fn kind_at(registry: &Registry, wanted: &str) -> String {
21	    registry
22	        .depth_first()
23	        .iter()
24	        .find(|info| registry.path_for(info.id).as_deref() == Some(wanted))
25	        .map(|info| info.kind.clone())
26	        .unwrap_or_else(|| panic!("no registration face at `{wanted}`"))
27	}
28	
29	fn slot_plan() -> GraftPlan {
30	    GraftPlan::command(FRAMEWORK, "cut root/control/button graft button_fast")
31	        .expect("the graft command parses")
32	}
33	
34	/// `source` 现在由声明点推导，但必须与旧方案注入的值完全相同。
35	/// `source` is derived at the declaration site now, and must equal what the
36	/// previous design injected.
37	#[test]
38	fn built_in_tree_has_the_expected_paths_and_derived_sources() {
39	    let registry = base_registry();
40	    let mut rows = registry
41	        .depth_first()
42	        .iter()
43	        .map(|info| {
44	            format!(
45	                "{} kind={} source={}",
46	                registry.path_for(info.id).unwrap_or_default(),
47	                info.kind,
48	                info.source.portable_file()
49	            )
50	        })
51	        .collect::<Vec<_>>();
52	    rows.sort();
53	
54	    assert_eq!(
55	        rows,
56	        [
57	            "root/control kind=Control source=control/control.rs",
58	            "root/control/button kind=Button source=control/object/button/button.rs",
59	            "root/control/slider kind=Slider source=control/object/slider/slider.rs",
60	        ]
61	    );
62	}
63	
64	/// 自定义 `handle` 且省略 `preset`/`parts` 的面必须记录朴素的默认名，而不是展开后的
65	/// `"$crate :: NoPreset"` 字面 token；三个内置面都是这个形态，因此这条断言钉住的是实际
66	/// 出厂的那条路径，而不只是单元测试里的形状。
67	/// A face with a custom `handle` and an omitted `preset`/`parts` must record the
68	/// plain default names, not the expanded `"$crate :: NoPreset"` literal tokens.
69	/// All three built-in faces take that shape, so this pins the shipped path, not
70	/// only a shape in a unit test.
71	#[test]
72	fn handle_faces_record_plain_default_preset_and_parts_names() {
73	    assert_eq!(control_button::control::REGISTRATION.preset, "NoPreset");
74	    assert_eq!(control_button::control::REGISTRATION.parts, "NoParts");
75	    assert_eq!(
76	        control_button::control::object::button::REGISTRATION.preset,
77	        "NoPreset"
78	    );
79	    assert_eq!(
80	        control_button::control::object::button::REGISTRATION.parts,
81	        "NoParts"
82	    );
83	    assert_eq!(
84	        control_button::control::object::slider::REGISTRATION.preset,
85	        "NoPreset"
86	    );
87	    assert_eq!(
88	        control_button::control::object::slider::REGISTRATION.parts,
89	        "NoParts"
90	    );
91	}
92	
93	/// 叶子面以自身名字载入，公开模块路径和类型名都不变。
94	/// A leaf face loads under its own name, so its public module path and type
95	/// name are unchanged.
96	#[test]
97	fn leaf_face_module_path_stays_clean() {
98	    assert_eq!(
99	        std::any::type_name::<control_button::control::object::button::Button>(),
100	        "control_button::control::object::button::Button"
101	    );
102	}
103	
104	/// 文件夹面的条目仍按原有路径可寻址（同名子模块 + 重导出）。
105	/// A folder face's items stay addressable at their original path through the
106	/// same-named child module and its re-export.
107	#[test]
108	fn folder_face_items_stay_reachable() {
109	    let _: NodeId = control_button::control::NODE_ID;
110	    let _ = std::marker::PhantomData::<control_button::control::Control>;
111	    let _ = control_button::control::ControlFrame;
112	}
113	
114	/// 面文件以 `//!` 开头（见 `src/control/control.rs`）仍然编译，并且源码路径
115	/// 就是仓库相对路径，而不是任何构建副本。
116	/// Face files keep their `//!` header (see `src/control/control.rs`), and the
117	/// recorded source is the repository-relative path rather than a build copy.
118	#[test]
119	fn face_sources_point_at_the_real_files() {
120	    let registry = base_registry();
121	    let sources = registry
122	        .depth_first()
123	        .iter()
124	        .map(
…[工具返回：此处截断，全文 32342 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**当时推理**：
```
All 25 answers are ≤20 lines, each has exactly one `总调用：` line and the `nonbridge:` last line ✓.

Now let me update BATCHING.md with the second-batch shared calls, then send the report.

BATCHING additions:
- `diff -r trees/s1 trees/fd` and `diff -r trees/s1 trees/fe` (served fd and fe respectively).
- `diff -r trees/g3 trees/fa` (served fa+g3 — showed they're identical mod .gitignore).
- The fa/g3 shared exploration: `node tests/registry.rs` (huge output) served both (logged in fa's log); the g3 work reused the structural knowledge.
- `cat` of the hardbug BRIEFs (apparatus/题面).
- The tree-hash scripts (`node /tmp/treecheck.mjs`, `/tmp/copycheck.mjs`) — batch 1 shared.
- Batch-2 hash verification: I used `node /tmp/treehash.mjs` on the 10 restored originals + copies (one bash call, served r1-r4/h1/g3/fa/fb/fd/fe).

Let me write these into BATCHING.md (append a section B2/C2).

Then the final report to the captain with:
- 25/26 delivered; fc pending.
- The corrected byte figure (106,223 B for the 15; 357,058 B for the 25) and the nonbridge separated.
- 总调用 lines added to all 25 answers; .txt deviation declared in SETUP.md §5.3.
- The causality correction adopted in APPARATUS.md §4 and PARTIAL.md §2.
- The new results headline (one line per question).
- The method catch (CARGO_TARGET_DIR artifact reuse) — worth reporting since it affects the "measures" discipline.
- The s3/g1 boundary note from the verifier: I should acknowledge and, if cheap, amend s3/g1 answers with the frame
…[推理：此处截断，全文 3146 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①14 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph node tests/registry.rs`  · exit 0 · step 4（turn 3）

**工具返回**：
```
**tests/registry.rs** — 766 lines, 28 symbols · no other indexed file depends on it

1	//! 集成测试：把 README 那棵 Control / Button 树当成真实宿主，逐项验证构建期
2	//! 生成、注册、父级规则、graft、项目外实现和发布态静态计划。
3	//! Integration tests: treat the README Control/Button tree as a real host and
4	//! verify the generated plan, registration, the parent rule, grafting, the
5	//! out-of-project implementation, and the release-time static plan.
6	
7	use control_button::{FRAMEWORK, base_registry, builtin_static_plan};
8	use control_button_graft::FRAMEWORK as GRAFT_FRAMEWORK;
9	use xirang_toolchain::runtime::registry_core::lexicon;
10	use xirang_toolchain::runtime::registry_core::{
11	    FrameworkId, GraftPlan, NodeId, OwnedFlowContract, PluginManifest, PluginMode, PluginSource,
12	    PluginTrustError, PluginTrustPolicy, Registry, StaticGraftCut,
13	};
14	use xirang_toolchain::runtime::{
15	    GraftPlanDocument, RecordReport, apply_recorded_grafts, graft_record_root,
16	};
17	
18	/// 读取某一逻辑路径上的 kind；找不到就失败。
19	/// Read the kind at a logical path, failing when the path is absent.
20	fn kind_at(registry: &Registry, wanted: &str) -> String {
21	    registry
22	        .depth_first()
23	        .iter()
24	        .find(|info| registry.path_for(info.id).as_deref() == Some(wanted))
25	        .map(|info| info.kind.clone())
26	        .unwrap_or_else(|| panic!("no registration face at `{wanted}`"))
27	}
28	
29	fn slot_plan() -> GraftPlan {
30	    GraftPlan::command(FRAMEWORK, "cut root/control/button graft button_fast")
31	        .expect("the graft command parses")
32	}
33	
34	/// `source` 现在由声明点推导，但必须与旧方案注入的值完全相同。
35	/// `source` is derived at the declaration site now, and must equal what the
36	/// previous design injected.
37	#[test]
38	fn built_in_tree_has_the_expected_paths_and_derived_sources() {
39	    let registry = base_registry();
40	    let mut rows = registry
41	        .depth_first()
42	        .iter()
43	        .map(|info| {
44	            format!(
45	                "{} kind={} source={}",
46	                registry.path_for(info.id).unwrap_or_default(),
47	                info.kind,
48	                info.source.portable_file()
49	            )
50	        })
51	        .collect::<Vec<_>>();
52	    rows.sort();
53	
54	    assert_eq!(
55	        rows,
56	        [
57	            "root/control kind=Control source=control/control.rs",
58	            "root/control/button kind=Button source=control/object/button/button.rs",
59	            "root/control/slider kind=Slider source=control/object/slider/slider.rs",
60	        ]
61	    );
62	}
63	
64	/// 自定义 `handle` 且省略 `preset`/`parts` 的面必须记录朴素的默认名，而不是展开后的
65	/// `"$crate :: NoPreset"` 字面 token；三个内置面都是这个形态，因此这条断言钉住的是实际
66	/// 出厂的那条路径，而不只是单元测试里的形状。
67	/// A face with a custom `handle` and an omitted `preset`/`parts` must record the
68	/// plain default names, not the expanded `"$crate :: NoPreset"` literal tokens.
69	/// All three built-in faces take that shape, so this pins the shipped path, not
70	/// only a shape in a unit test.
71	#[test]
72	fn handle_faces_record_plain_default_preset_and_parts_names() {
73	    assert_eq!(control_button::control::REGISTRATION.preset, "NoPreset");
74	    assert_eq!(control_button::control::REGISTRATION.parts, "NoParts");
75	    assert_eq!(
76	        control_button::control::object::button::REGISTRATION.preset,
77	        "NoPreset"
78	    );
79	    assert_eq!(
80	        control_button::control::object::button::REGISTRATION.parts,
81	        "NoParts"
82	    );
83	    assert_eq!(
84	        control_button::control::object::slider::REGISTRATION.preset,
85	        "NoPreset"
86	    );
87	    assert_eq!(
88	        control_button::control::object::slider::REGISTRATION.parts,
89	        "NoParts"
90	    );
91	}
92	
93	/// 叶子面以自身名字载入，公开模块路径和类型名都不变。
94	/// A leaf face loads under its own name, so its public module path and type
95	/// name are unchanged.
96	#[test]
97	fn leaf_face_module_path_stays_clean() {
98	    assert_eq!(
99	        std::any::type_name::<control_button::control::object::button::Button>(),
100	        "control_button::control::object::button::Button"
101	    );
102	}
103	
104	/// 文件夹面的条目仍按原有路径可寻址（同名子模块 + 重导出）。
105	/// A folder face's items stay addressable at their original path through the
106	/// same-named child module and its re-export.
107	#[test]
108	fn folder_face_items_stay_reachable() {
109	    let _: NodeId = control_button::control::NODE_ID;
110	    let _ = std::marker::PhantomData::<control_button::control::Control>;
111	    let _ = control_button::control::ControlFrame;
112	}
113	
114	/// 面文件以 `//!` 开头（见 `src/control/control.rs`）仍然编译，并且源码路径
115	/// 就是仓库相对路径，而不是任何构建副本。
116	/// Face files keep their `//!` header (see `src/control/control.rs`), and the
117	/// recorded source is the repository-relative path rather than a build copy.
118	#[test]
119	fn face_sources_point_at_the_real_files() {
120	    let registry = base_registry();
121	    let sources = registry
122	        .depth_first()
123	        .iter()
124	        .map(
…[工具返回：此处截断，全文 32342 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**当时推理**：
```
Important findings:

1. **The ledger fingerprint `22b54a4c…` does NOT equal `sha256(button.rs)` = `e842be74…`.** So the fingerprint is not a plain file-byte hash. As in h2, the fingerprint must be some other scheme (probably the NodeId or a hash over the declared shape). Note: I should be honest about this in the answer: I can verify the *ledger file* is intact (its own sha256), and that the named file is untouched by me, but I cannot recompute the ledger's fingerprint scheme from the tree (no code in the tree reads/writes it). Hmm — the captain says "`adopted`/`conformance` 读出来的 verdict 符合题面" — those must be the round9 arm's bridge tools. Since I can't run them (frozen codegraph only + no bridge), my premise check will be: the ledger exists, is well-formed (8 fields), names `root/control/button`, and its note says "agreed before the horizontal extension" (i.e. the题面's premise: already adopted, now extend horizontally) ✓.

Hmm, hold on. Could the fingerprint be a hash over the *declaration shape*? Let me test a couple of hypotheses quickly (like h2):
- sha256 of the button.rs file *without* the doc comments?
- sha256 of the `control_object!` block?
- Maybe it's the NodeId hash: the graft_record example printed `ff1c57d9b110ef57eac134a3cb4e46b6` for `root/control/button` — hmm, that was for the fa/g3 tree (same host). For fc's tree, the button's NodeId would be some hash — but the ledger's fingerprint is a *sha256-length* hex (64 chars) whereas the NodeId shown in graft_record
…[推理：此处截断，全文 9424 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①15 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph node src/control/object/toggle/toggle.rs`  · exit 0 · 共享/装置步

**工具返回**：
```
No indexed file matches "src/control/object/toggle/toggle.rs". Codegraph indexes source files; configs/docs it doesn't parse won't appear — Read those directly.
```

### ①16 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph callers Toggle::paint`  · exit 0 · 共享/装置步

**工具返回**：
```
ℹ Symbol "Toggle::paint" not found — did you mean: paint, ControlFrame?
```

### ①17 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph sync .`  · exit 0 · 共享/装置步

**工具返回**：
```
┌  Syncing CodeGraph
│
Scanning files...
Parsing code...
Resolving refs...
Linking dynamic dispatch...
│
◆  Synced 3 changed files
│
●  Added: 1, Modified: 2 — 43 nodes in 156ms
│
●  Resolved 0 pending references (8 unresolved)
│
└  Done
```

### ①18 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph node src/control/object/toggle/toggle.rs`  · exit 0 · 共享/装置步

**工具返回**：
```
**src/control/object/toggle/toggle.rs** — 27 lines, 2 symbols · used by 1 file: src/control/control.rs

1	//! Toggle 叶子面：Control 的第三个直接子对象，与 Button/Slider 同形。
2	//! Toggle leaf face: Control's third direct child, shaped like Button/Slider.
3	
4	use crate::control::{ControlFrame, ControlHandle};
5	use xirang_toolchain::runtime::{ContractId, FlowContract};
6	
7	pub struct Toggle;
8	
9	impl ControlHandle for Toggle {
10	    fn paint(&self) -> ControlFrame {
11	        ControlFrame
12	    }
13	}
14	
15	crate::control_object! {
16	    kind: Toggle,
17	    exports: ["control.render"],
18	    parent: crate::control::NODE_ID,
19	    handle_contracts: [crate::control::ControlHandle],
20	    flow: FlowContract::new(
21	        ContractId::new("control.render.v1"),
22	        1,
23	        "ControlInput",
24	        "ControlFrame",
25	    ),
26	}
27
```

### ①19 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fa && codegraph callers ControlHandle`  · exit 0 · 共享/装置步

**工具返回**：
```
Callers of "ControlHandle" (0):

trait ControlHandle (rust) — src/control/control.rs:13
ℹ No callers found for "ControlHandle"
```


## 交付答案（逐字）

# fa —「我要加一个新对象，和现有的差不多。」（可改；交付后门必须绿）

① 题面只有那一句；宿主里现有两个叶子对象 `Button`、`Slider`（`src/control/object/{button,slider}/…`），父级是 `Control`（`src/control/control.rs:19-39`）。
② 做了什么：加第三个叶子对象 **`Toggle`**，与现有两个同形——
   - 新文件 `src/control/object/toggle/toggle.rs`：`pub struct Toggle` + `impl ControlHandle for Toggle`，以及 `crate::control_object!{ kind: Toggle, exports: ["control.render"], parent: crate::control::NODE_ID, handle_contracts: [crate::control::ControlHandle], flow: FlowContract::new(ContractId::new("control.render.v1"), 1, "ControlInput", "ControlFrame") }`（逐项照抄 `slider.rs` 的形状，只换 kind/类型名）。
   - 入口 `src/lib.rs`：新增 `pub const PUBLISHED_OBJECT_FACES: [NodeId; 3]`，**在代码里拼出**三个对象面的路径（button/slider/toggle）。这一步是"新对象真的会被发布"的关键：构建期作用域从入口的可达源码收窄，而那次扫描**忽略 import、字符串与注释**，只把代码里拼出的路径当成一次引用（写 `use` 不算）。
③ 依据：`slider.rs:16-27` 与 `button.rs:15-40` 是现成模板（kind/exports/parent/handle_contracts/flow 五件套）；父级规则在 `src/control/registry_rule/registry_rule.rs`；作用域收窄与"没被声明就不发布"写在 `src/lib.rs:36-43` 自己的文档里。
④ 改动清单：新增 1 个文件；`src/lib.rs` 加 `PUBLISHED_OBJECT_FACES`；`tests/registry.rs` 更新 4 处**期望**（都写明理由）：树行表 3→4 行、`plan.faces().len()` 3→4、构建期作用域 `selected.len()` 2→3 并加一条 toggle 行断言、把"父级规则不看 kind"那条测试里的合成样例从 `Toggle/"toggle"` 改名为 `Dial/"dial"`（否则与真实面的槽位名相撞——内核会以 `duplicate sibling registry name` 拒绝，实测报错里点名 `control/object/toggle/toggle.rs`）。**未动**：`button.rs`/`slider.rs`/`control.rs`/graft 侧/Cargo.toml。
⑤ 命令序列与门：`cargo test --offline`（改完先 4 红 ⇒ 更新期望后 **rc=0，25+1+1 全 ok**）；`cargo test --offline --all-features` **rc=0**；`cargo run --example tree` 现在打印 4 行，含 `root/control/toggle  kind=Toggle  source=control/object/toggle/toggle.rs`。
   反证：若只加文件而不在入口拼路径，`tree` 仍是 3 行（未声明的面不发布）——那就等于没加；若父级/字段写错，`parent_rule_rejects_…`/`built_in_tree_…` 会红。我没有靠删断言变绿：4 处改的都是**期望值**，且每处都在注释里写了为什么。

总调用：成功 19 / 被拒 0（codegraph；含 2 次 init ⇒ 17 步）
调用清单：codegraph `files`、`node src/control/object/{button,slider}/…`、`node src/control/control.rs`、`node src/lib.rs`、`node tests/registry.rs`、`node tests/static_plan_allocations.rs`、`node src/control/object/toggle/toggle.rs`（改后）、`sync .`（新文件入索引）；非 codegraph：`cargo test`×3、`cargo run --example tree`×1。
nonbridge: 5 次 / 16,159 B（含复位前的题目态自检；全在 logs/fa.txt）