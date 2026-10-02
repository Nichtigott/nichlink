# cg · `fd`（精简版；完整版见 `../cg/fd.md`）

**题面**：五族：「这个对象有问题。」

> 仪器调用 **6** · 步 0 · 输出 0 tok · 推理 0 字符 · 累计上下文 0

## 仪器调用序列（工具给了什么 + 当时 agent 在想什么）

### ①1 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fd && codegraph init .`  · exit 0 · step 26（turn 2）

**工具返回**：
```
┌  Initializing CodeGraph
│
◆  Initialized in /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fd
│
Scanning files...
Parsing code...
Resolving refs...
Linking dynamic dispatch...
│
◆  Indexed 12 files
│
●  46 nodes, 88 edges in 480ms
│
└  Done
```

**当时推理**：
```
Now g3, fa, fb (host trees), then fd, fe.

**g3**: "给这个对象加一层内部结构，但不要动它的注册树和公开路径；说明凭什么相信没动，门必须绿" (add an internal layer to this object, without touching its registration tree and public paths; explain why you believe it's untouched; gate must be green).

The g3 tree: 160K, 13 files, 102 nodes. Let me look: it's a nichlink control-button host with `.nichlink`, examples, src, tests, build.rs, Cargo.toml. Let me read its structure and lib.rs, control.rs, and one object file.

Plan for g3:
1. Read the tree (files, lib.rs, control.rs, an object file) via codegraph.
2. Determine the target object (probably Button, the README example).
3. Add an internal layer: e.g. `ButtonParts { label: String }` + `impl PartsContract for ButtonParts` + `Button { parts: ButtonParts }` + `pub fn parts(&self) -> &ButtonParts` — following the fb precedent (I saw fb's tree has exactly this!). Hmm — fb's tree (13333fd5 = same as fa) contains `ButtonParts` already... wait, that was the OLD damaged fb copy which had a "done state". Now fa and fb are restored to the clean state.

So for g3 the target is probably the same object (Button) — but which object? The question says "这个对象" (this object) — in the g3 tree there may be a designated object (e.g. the one with a `Parts`-shaped hook). Let me read the tree to find out.

4. Prove the registration tree and public paths unchanged:
   - Registration tree: run something that prints the outline (the host has `outline()` in lib.rs; maybe an example binary prints i
…[推理：此处截断，全文 4832 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**当时输出**：`Now the five remaining hosts. First a layout + diff reconnaissance:`

### ①2 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fd && codegraph files`  · exit 0 · step 36（turn 2）

**工具返回**：
```
Project Structure (12 files):

└── crates
    ├── core
    │   ├── src
    │   │   ├── audit.rs (rust, 4 symbols)
    │   │   ├── lib.rs (rust, 1 symbols)
    │   │   ├── limits.rs (rust, 2 symbols)
    │   │   ├── model.rs (rust, 4 symbols)
    │   │   └── store.rs (rust, 6 symbols)
    │   └── tests
    │       └── audit.rs (rust, 3 symbols)
    └── report
        ├── src
        │   ├── buckets.rs (rust, 4 symbols)
        │   ├── lib.rs (rust, 1 symbols)
        │   ├── query.rs (rust, 4 symbols)
        │   └── render.rs (rust, 4 symbols)
        └── tests
            ├── buckets.rs (rust, 4 symbols)
            └── report.rs (rust, 9 symbols)
```

**当时推理**：
```
Now I have a clear picture of the fa/g3 host:

- The tree pins `rows == [root/control, root/control/button, root/control/slider]` (registry.rs:54-61: `built_in_tree_has_the_expected_paths_and_derived_sources`).
- `handle_faces_record_plain_default_preset_and_parts_names` (registry.rs:72-91) pins preset/parts == NoPreset/NoParts for all three faces.
- The entry plan (lib.rs:48-54) has 2 cuts (button, slider) with graft types from `control_button_graft::*_fast`.
- `leaf_face_module_path_stays_clean` (registry.rs:97-102) pins the button's public type name — that's exactly the "公开路径" the g3 question says must not move ✓.
- The facade `ide_mirror.rs` and `static_plan_allocations.rs` tests pin static plan allocations and IDE mirror.

So for **g3** ("add an internal layer to this object, don't touch its registration tree and public paths"):
- Add a parts/preset internal layer to the **Button** face (button.rs).
- Keep: NODE_ID (namespace+path+kind) ✓ automatically; the tree row list (root/control/button) unchanged ✓; the public path `control_button::control::object::button::Button` unchanged ✓.
- Update `handle_faces_record_plain_default_preset_and_parts_names` to reflect that button now records its declared names (while control/slider keep the plain defaults) — otherwise the gate would be red. Hmm, this test is precisely about the *omitted* case; after my change, button has them declared. The test's doc says "All three built-in faces take that shape, so this pins the shipped path" 
…[推理：此处截断，全文 20176 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**当时输出**：`fd and fe are ledger-defect fixes (like the r-trees). Applying and verifying:`

### ①3 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fd && codegraph init .`  · exit 0 · step 38（turn 2）

**工具返回**：
```
┌  Initializing CodeGraph
│
◆  Initialized in /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fd
│
Scanning files...
Parsing code...
Resolving refs...
Linking dynamic dispatch...
│
◆  Indexed 12 files
│
●  46 nodes, 87 edges in 486ms
│
└  Done
```

**当时推理**：
```
fd and fe are green on both faces ✓ (default 4 = 1+3; all-features 5 = 1 audit + 1 + 3).

Now write the answers for fd and fe (short, in the f-family deliverable shape: 做了什么、依据、命令序列、总调用次数 + the standard 5 sections).

fd answer:
① 症状：初始红——同一 target（crates/report/tests/report.rs）3 条断言失败 = 2 处缺陷：`the_minimum_amount_filter_keeps_the_entries_at_or_above_it`（left 1/right 2）、`the_report_prints_its_count_before_the_entries`（`:38` expect 炸）、`the_report_names_its_count_and_its_entries`（`:48` contains 失败）。
② 根因：A `crates/report/src/query.rs:27` `if entry.amount > min`（方向反，应 `<`）；B `crates/report/src/render.rs:8-13` 缺 `write_count(...)` 调用（函数在 `:17`，callers=0）。
③ 改动清单：A `<`；B 在表头后补 `write_count(&mut text, entries);`。未动测试。
④ 反证：两次失败文本互证（A 报 left=1 与反向语义一致；B 报"count block is written"且打印文本只有表头+分录）；修后默认面 4 passed rc=0、--all-features 5 passed rc=0。
⑤ 调用清单 + 命令序列：codegraph node query.rs/render.rs（改前/改后）；cargo test（改前 rc=101、改后 两次 rc=0）。
nonbridge: 3 cargo + 1 diff ≈ ...

fe answer:
① 症状：初始红 1 条：`the_minimum_amount_filter_keeps_the_entries_at_or_above_it`；另有 1 处默认面看不见的缺陷（audit 面）。
② 根因：A `crates/report/src/query.rs:27` `> min`；B `crates/core/src/audit.rs:9` `== 0`（应为 `!= 0`），被 `#![cfg(feature="audit")]`（crates/core/tests/audit.rs:10）+ 模块 cfg 挡住。
③ 改动：A `<`；B `!= 0`。
④ 反证：A 的 left=1 与反向语义一致；B 在默认面看不见——若我只修 A，`--all-features` 面会红（实测两条命令对比：默认面绿但 all-features 才暴露 B）；修后两面都绿（default 4 passed rc=0；--all-features 5 passed rc=0）。
⑤ 调用清单与命令序列。
nonbridge: 3 cargo + 1 diff.

Let me write both files now, th
…[推理：此处截断，全文 1678 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**同一步还发了**：`write`→共享

### ①4 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fd && codegraph files`  · exit 0 · step 72（turn 2）

**工具返回**：
```
Project Structure (12 files):

└── crates
    ├── core
    │   ├── src
    │   │   ├── audit.rs (rust, 4 symbols)
    │   │   ├── lib.rs (rust, 1 symbols)
    │   │   ├── limits.rs (rust, 2 symbols)
    │   │   ├── model.rs (rust, 4 symbols)
    │   │   └── store.rs (rust, 6 symbols)
    │   └── tests
    │       └── audit.rs (rust, 3 symbols)
    └── report
        ├── src
        │   ├── buckets.rs (rust, 4 symbols)
        │   ├── lib.rs (rust, 1 symbols)
        │   ├── query.rs (rust, 4 symbols)
        │   └── render.rs (rust, 4 symbols)
        └── tests
            ├── buckets.rs (rust, 4 symbols)
            └── report.rs (rust, 9 symbols)
```

### ①5 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fd && codegraph node crates/report/src/render.rs`  · exit 0 · step 75（turn 2）

**工具返回**：
```
**crates/report/src/render.rs** — 21 lines, 2 symbols · used by 1 file: crates/report/tests/report.rs

1	//! Rendering.
2	
3	use ledger_core::model::Entry;
4	
5	/// Render the entries: a header, the count, then one line per entry.
6	/// 渲染分录：表头、条数，然后每条一行。
7	pub fn render(entries: &[Entry]) -> String {
8	    let mut text = String::from("LEDGER REPORT\n");
9	    write_count(&mut text, entries);
10	    for entry in entries {
11	        text.push_str(&format!("{}: {}\n", entry.account, entry.amount));
12	    }
13	    text
14	}
15	
16	/// The count block.
17	/// 条数块。
18	pub fn write_count(text: &mut String, entries: &[Entry]) {
19	    text.push_str(&format!("entries: {}\n", entries.len()));
20	}
21
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

### ①6 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fd && codegraph node crates/report/src/query.rs`  · exit 0 · 共享/装置步

**工具返回**：
```
**crates/report/src/query.rs** — 34 lines, 2 symbols · used by 1 file: crates/report/tests/report.rs

1	//! Filtering entries.
2	
3	use ledger_core::model::Entry;
4	
5	/// Which entries a caller wants.
6	/// 调用方想要哪些分录。
7	#[derive(Clone, Debug, Default)]
8	pub struct Filter {
9	    /// Only entries for this account, when set.
10	    /// 只保留该账户的分录（若设置）。
11	    pub account: Option<String>,
12	    /// Only entries at or above this amount, when set.
13	    /// 只保留不小于该金额的分录（若设置）。
14	    pub min_amount: Option<i64>,
15	}
16	
17	impl Filter {
18	    /// Whether this entry passes.
19	    /// 本分录是否通过。
20	    pub fn matches(&self, entry: &Entry) -> bool {
21	        if let Some(account) = &self.account {
22	            if &entry.account != account {
23	                return false;
24	            }
25	        }
26	        if let Some(min) = self.min_amount {
27	            if entry.amount < min {
28	                return false;
29	            }
30	        }
31	        true
32	    }
33	}
34
```


## 交付答案（逐字）

# fd —「这个对象有问题。」（初始红：同一 target 3 条断言 = 2 处缺陷）

① 症状：`crates/report/tests/report.rs` 一个 target 里三条断言红：`the_minimum_amount_filter_keeps_the_entries_at_or_above_it`（`:27` left 1/right 2）、`the_report_prints_its_count_before_the_entries`（`:38` expect 炸）、`the_report_names_its_count_and_its_entries`（`:48` contains 失败）。
② 根因（2 处）：**A** `crates/report/src/query.rs:27` `if entry.amount > min`——下限过滤方向写反（应 `<`）；**B** `crates/report/src/render.rs:8-13` 没有调用 `write_count`（函数仍在 `:17`，`callers write_count`=0）⇒ 条数行整块没写出来。
   机制：A 让 `min_amount: Some(0)` 只留下负数（数出 1 条）；B 让 render 的输出只剩表头+分录，`entries: 3` 永不出现。
③ 改动清单：`query.rs:27` `>`→`<`；`render.rs` 表头之后、循环之前补 `write_count(&mut text, entries);`。**未动**测试、未动 `buckets.rs`（它的 `< 1000` 本来就是对的）。
④ 反证：A 的 `left: 1` 与反向语义推出的"只剩 -500"逐字相符；B 的失败文本把渲染结果原样打印出来（只有表头+三行分录），若 render 还在调 `write_count`，这个断言不会红。修后：默认面 **4 passed / 0 failed，rc=0**；`--all-features` **5 passed / 0 failed，rc=0**。
⑤ 命令序列 / 调用清单：codegraph `files`；`node crates/report/src/{query,render}.rs`、`node crates/report/tests/report.rs`（改前/改后）；`callers write_count`；`diff -r trees/s1 trees/fd`（定位 2 处差异）；非 codegraph：`cargo test --offline` ×3（改前 rc=101、改后默认面 rc=0、改后 `--all-features` rc=0，per-tree `CARGO_TARGET_DIR`）。

总调用：成功 6 / 被拒 0（codegraph；含 2 次 init ⇒ 4 步）

nonbridge: 4 次 / 约 7.0 KB（`diff` 1 次 + cargo 3 次，全部在 logs/fd.txt）