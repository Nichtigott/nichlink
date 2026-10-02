# cg · `fb`（精简版；完整版见 `../cg/fb.md`）

**题面**：五族：「这个对象内部还不够。」

> 仪器调用 **6** · 步 5 · 输出 8,188 tok · 推理 18,348 字符 · 累计上下文 1,493,504

## 仪器调用序列（工具给了什么 + 当时 agent 在想什么）

### ①1 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fb && codegraph init .`  · exit 0 · step 53（turn 2）

**工具返回**：
```
┌  Initializing CodeGraph
│
◆  Initialized in /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fb
│
Scanning files...
Parsing code...
Resolving refs...
Linking dynamic dispatch...
│
◆  Indexed 13 files
│
●  106 nodes, 164 edges in 557ms
│
└  Done
```

**当时推理**：
```
Now g3 is perfect:
- Gate green (default: 6 ok, 0 failed) ✓
- All three observable artifacts byte-identical before/after ✓ (tree, health_check literally; graft_record modulo the temp-dir PID) — including the NodeId hashes.
- The internal layer (ButtonPreset/ButtonParts with required/provided ["label"]) is declared and recorded in REGISTRATION.contract ✓, pinned by the updated test.

Let me also run --all-features once for g3 (the earlier run was pre-move; let me redo for the final state).

Then write fb and g3 answers, then tackle fa (the new object).

Actually, time/context is getting long. Let me be efficient: run g3 --all-features now (1 call), then write both answers, then fa.

For fb, note: fb's final state has the layer *above* the declaration (health_check would show the line shift) — that's fine for fb's question (no invariance requirement), but for consistency I could also move it below in fb. Hmm: fb's deliverable is "内部还不够" — the layer + gate green. The line shift is irrelevant. I'll leave fb as is (and mention the same technique). Hmm, but for cleanliness let me apply the same change to fb (move below + single-line import) so both trees are consistent, then re-verify fb's gate + tree (fb's gate was already green; moving the block requires a re-run).

Hmm, is consistency worth 1 more cargo run? It also makes fb's answer stronger (I can report the same invariance). Let me do it: apply the same rearrangement to fb, re-run the gate.

Both answers then report: layer be
…[推理：此处截断，全文 1663 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①2 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fb && codegraph files`  · exit 0 · step 56（turn 2）

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
│   │   │   │   └── button.rs (rust, 9 symbols)
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
My computation matches the verifier's number exactly: **106,223 B** for the 15 questions ✓ (excluding my CMD/EXIT bookkeeping lines and nonbridge output). 

Note: `fa` shows 63,745 B of codegraph output — that's from my exploration of the fa/g3 tree (the big `files`/`node tests/registry.rs` outputs, which I read while exploring for both fa and g3). Hmm, that's attributed to fa's log but partly served g3. I'll note that in BATCHING.md (shared exploration).

Now let me write fb and g3 answers, then do fa.

Let me write fb's answer now with the 总调用 line.

fb numbers: codegraph 6 calls (6 ok / 0 refused), 2 init → 4 steps; nonbridge 8 / 16,520 B. Hmm, fb's nb=8/16520 — that includes the baseline tree run, the failed patch test run, the successful test run, the tree-after run, the all-features run, the second all-features run... Let me recount: fb nonbridge calls: (1) baseline tree, (2) cargo test after patch (failed), (3) tree after, (4) all-features, (5) rearranged all-features... plus pre-restoration runs (the earlier premise-check cargo test + diff). The log includes earlier entries. I'll report "本题日志内 nonbridge 8 次 / 16,520 B（含复位前的题目态自检）".

Let me write the files.
```

### ①3 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fb && codegraph init .`  · exit 0 · step 72（turn 2）

**工具返回**：
```
┌  Initializing CodeGraph
│
◆  Initialized in /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fb
│
Scanning files...
Parsing code...
Resolving refs...
Linking dynamic dispatch...
│
◆  Indexed 13 files
│
●  102 nodes, 158 edges in 558ms
│
└  Done
```

### ①4 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fb && codegraph node examples/tree.rs`  · exit 0 · 共享/装置步

**工具返回**：
```
**examples/tree.rs** — 11 lines, 1 symbol · no other indexed file depends on it

1	//! 运行 `cargo run -p nichlink-example-control-button --example tree` 打印示例的
2	//! 注册树。
3	//! Run `cargo run -p nichlink-example-control-button --example tree` to print the
4	//! example's registration tree.
5	
6	fn main() {
7	    for row in control_button::outline() {
8	        println!("{row}");
9	    }
10	}
11
```

### ①5 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fb && codegraph node tests/ide_mirror.rs`  · exit 0 · 共享/装置步

**工具返回**：
```
**tests/ide_mirror.rs** — 54 lines, 2 symbols · no other indexed file depends on it

1	//! The IDE mirror must type-check, not only parse.
2	//! IDE 镜像必须能通过类型检查，而不只是能解析。
3	//!
4	//! `rust-analyzer` applies `#[path]` only at the top level of a file or an
5	//! expansion, so a nested face is loaded a second time as a crate-root shadow.
6	//! In that shadow `super` is the crate root, while the face's derived rule path
7	//! (`super::registry_rule::REGISTRATION_RULE`) names a module beside the real
8	//! face file — so the whole crate failed to type-check under `--cfg
9	//! rust_analyzer`, which is the cfg an editor uses and `rustc` never does. The
10	//! commands below are the check that catches it.
11	//! `rust-analyzer` 只在文件或展开的顶层应用 `#[path]`，因此嵌套面会被第二次载入为 crate
12	//! 根影子。影子里的 `super` 是 crate 根，而注册面派生的规则路径
13	//! （`super::registry_rule::REGISTRATION_RULE`）命名的是真实面文件旁边的模块——于是整个
14	//! crate 在 `--cfg rust_analyzer` 下类型检查失败，而那正是编辑器使用、`rustc` 从不使用的
15	//! cfg。下面的命令就是抓住这件事的检查。
16	
17	use std::path::Path;
18	use std::process::Command;
19	
20	/// Run one host crate through the IDE cfg in a private target directory.
21	/// 在一个私有 target 目录里跑一个宿主 crate 的 IDE cfg。
22	fn check_under_rust_analyzer(package: &str, target: &Path) {
23	    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
24	    let output = Command::new(cargo)
25	        .args(["rustc", "-p", package, "--lib", "--offline"])
26	        .arg("--target-dir")
27	        .arg(target)
28	        .args(["--", "--cfg", "rust_analyzer"])
29	        .current_dir(env!("CARGO_MANIFEST_DIR"))
30	        .output()
31	        .expect("run cargo rustc with the rust_analyzer cfg");
32	    assert!(
33	        output.status.success(),
34	        "`{package}` does not type-check under --cfg rust_analyzer:\n{}",
35	        String::from_utf8_lossy(&output.stderr)
36	    );
37	}
38	
39	/// Ignored by default: it builds this package and its dependencies into a target
40	/// directory of its own, which a nested `cargo` needs so it cannot deadlock on
41	/// the lock the running test already holds. CI runs it explicitly.
42	/// 默认忽略：它把本包及其依赖构建进一个自己的 target 目录——嵌套 `cargo` 必须如此，否则会
43	/// 与正在运行的测试持有的锁互等。CI 显式运行它。
44	#[test]
45	#[ignore = "runs a separate cargo build; CI runs it explicitly"]
46	fn the_ide_mirror_type_checks_with_nested_faces() {
47	    let target = std::env::temp_dir().join("nichlink-ide-mirror-target");
48	    // The nested-face host is the one that failed; the mirror-only host guards
49	    // the other direction, where a face declares no registry to own.
50	    // 嵌套面宿主是失败的那一个；只有镜像的宿主守住另一侧——注册面没有注册机可拥有。
51	    check_under_rust_analyzer("nichlink-example-control-button", &target);
52	    check_under_rust_analyzer("nichlink-example-control-button-graft", &target);
53	}
54
```

### ①6 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fb && codegraph node src/control/object/button/button.rs`  · exit 0 · 共享/装置步

**工具返回**：
```
**src/control/object/button/button.rs** — 68 lines, 2 symbols · used by 1 file: src/control/control.rs

1	//! Button 叶子面：它是 Control 的直接子对象，不再拥有自己的 Registry。
2	//! Button leaf face: a direct child of Control that owns no Registry of its own.
3	
4	use crate::control::{ControlFrame, ControlHandle};
5	use nichlink_toolchain::runtime::{ContractId, FlowContract, NON_EMPTY_TEXT, PartsContract, PresetContract};
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
17	    preset: ButtonPreset,
18	    parts: ButtonParts,
19	    exports: ["control.render"],
20	    parent: crate::control::NODE_ID,
21	    handle_contracts: [crate::control::ControlHandle],
22	    flow: FlowContract::new(
23	        ContractId::new("control.render.v1"),
24	        1,
25	        "ControlInput",
26	        "ControlFrame",
27	    ),
28	    // A real declared check: a Button's label crosses into the renderer as text
29	    // and must not be blank. The kernel cannot observe that label, so the host
30	    // calls `Registry::health_check` at the boundary; the declaration only
31	    // states what "valid" means.
32	    // 一条真实的已声明检查：Button 的标签以文本形式跨入渲染器，不能为空白。内核无法
33	    // 观测该标签，因此由宿主在边界处调用 `Registry::health_check`；声明只说明“有效”
34	    // 的含义。
35	    // `runtime_checks` is not part of `NodeId` (namespace + relative path + kind),
36	    // so declaring it must not move identity; `built_in_tree_has_the_expected_paths_and_derived_sources`
37	    // and `outline()` pin that.
38	    // `runtime_checks` 不是 `NodeId`（命名空间 + 相对路径 + kind）的组成部分，因此声明
39	    // 它不该移动身份；`built_in_tree_has_the_expected_paths_and_derived_sources` 与
40	    // `outline()` 钉住这一点。
41	    runtime_checks: [NON_EMPTY_TEXT],
42	}
43	
44	/// Button 的内部结构：这个对象由哪些零件构成。
45	/// The Button's internal structure: the parts this object is made of.
46	///
47	/// 这一层只进 `REGISTRATION.contract`（`preset` 要求什么、`parts` 供应什么），
48	/// 不进 `NodeId`（namespace + 相对源码路径 + kind），也不改公开路径。
49	pub struct ButtonParts {
50	    /// The label this face carries.
51	    /// 本面携带的 label。
52	    pub label: String,
53	}
54	
55	/// 造一个 Button 所需的零件：一个 label。
56	/// The parts a Button requires: one label.
57	pub struct ButtonPreset;
58	
59	impl PresetContract for ButtonPreset {
60	    type Output = ();
61	    const REQUIRED_PARTS: &'static [&'static str] = &["label"];
62	}
63	
64	impl PartsContract for ButtonParts {
65	    type Output = ();
66	    const PROVIDED_PARTS: &'static [&'static str] = &["label"];
67	}
68
```


## 交付答案（逐字）

# fb —「这个对象内部还不够。」（可改；交付后门必须绿）

① 起点/症状：宿主干净、门全绿，但对象 `Button` 只有"我是什么/谁实现我/我导出什么/什么流/什么运行期检查"，**没有**"我由什么构成"这一层：`tests/registry.rs:72` 钉着 `REGISTRATION.parts == "NoParts"`——对象内部是空的。
② 做了什么（依据在 ③）：给 `Button` 加一层内部结构（构造合同层）——
   `pub struct ButtonParts`（`impl PartsContract`，`PROVIDED_PARTS = ["label"]`）＋ `pub struct ButtonPreset`（`impl PresetContract`，`REQUIRED_PARTS = ["label"]`），并在 `crate::control_object!` 里声明 `preset: ButtonPreset, parts: ButtonParts`。
③ 依据（每一处都回过源码/宏定义）：框架自己的"内部结构"词汇就是这对 trait——`kernel/src/registry_core/declaration/contract.rs:26-33`（`PartsContract{ Output, PROVIDED_PARTS }`）与 `:35-51`（`NoPreset/NoParts` 默认）；宏字段 `$(preset: $preset:ty,)? $(parts: $parts:ty,)?` 在 `toolchain/src/runtime/src/macros/face_objects.rs:38-39`；省略时记录 `NoParts`、写下时原样转发（同文件 `:62-88`）；`assert_contract` 在 `contract.rs:72-77`。
   **踩过的坑**：`type Output` 必须与嫁接侧一致——先写 `type Output = ButtonParts` 时，构建期生成的 `assert_contract::<host::__Preset, graft::__Parts>()` 直接编译失败：`expected ButtonParts, found ()`（生成的 `generated_lib.rs:154`，机器在 `/tmp/fb_after.out` 的日志里）。改成 `Output = ()` 后成立（嫁接侧 `NoParts::Output = ()`）。
④ 改动清单：`src/control/object/button/button.rs`（新增两个类型 + 两个 impl；宏里加 `preset:`/`parts:` 两行；这层放在声明**之后**，使声明所在行号不动）；`tests/registry.rs:64-91`（把"三个面都省略"的期望更新为"control/slider 省略 ⇒ 默认名；button 声明 ⇒ 作者写的名字，且合同读同一批类型 `required_parts == provided_parts == ["label"]`"）。**未动**：`control.rs`、`slider.rs`、`carry`/graft 侧、任何路径。
⑤ 门与反证：`cargo test --offline` → **6 个 target 全 ok、0 failed，rc=0**；`--all-features` 同样 rc=0。`cargo run --example tree` 前后**逐字节相同**（sha `571b70bc642f6aa1`）⇒ 注册树没动。若这层动了身份，`tests/registry.rs:38`（路径+来源表）与 `:97`（公开类型名 `control_button::control::object::button::Button`）会红——它们没红。我没有"把红改绿"：被改的那条测试改的是**合同期望**（button 不再省略），不是把失败断言删掉。

总调用：成功 6 / 被拒 0（codegraph；含 2 次 init ⇒ 4 步）
调用清单：codegraph `files`、`node src/control/object/button/button.rs`（改前/改后）、`node tests/registry.rs`、`node examples/tree.rs`、`node src/control/control.rs`；非 codegraph `cargo test`×4、`cargo run --example tree`×2、`diff`×1。
nonbridge: 8 次 / 16,520 B（含复位前的题目态自检；全在 logs/fb.txt）