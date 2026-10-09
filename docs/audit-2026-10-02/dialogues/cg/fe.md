# cg · `fe`

**题面**：五族：「这几个对象都有问题。」

> 步 0 · 仪器调用 **6**（逐题日志条数）· 会话里定位到 5 条 · 输出 0 tok · 推理 0 字符 · 累计上下文 0

## 一、工具给了什么 · 当时 agent 在想什么（逐条配对）

> ⚠️ 日志 6 条 vs 会话定位 5 条：差的那些调用发生在**共享步**（一条命令里跑了多道题）或**装置步**里 —— 这正是"逐题 token 不可归因"的具体形状。

### ①1 日志第 1 条 · exit 0 · step 26（turn 2）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe && codegraph init .`

**工具返回**（逐字）：
```
┌  Initializing CodeGraph
│
◆  Initialized in /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe
│
Scanning files...
Parsing code...
Resolving refs...
Linking dynamic dispatch...
│
◆  Indexed 12 files
│
●  46 nodes, 88 edges in 520ms
│
└  Done
```

**该步推理**（逐字）：
```
Now g3, fa, fb (host trees), then fd, fe.

**g3**: "给这个对象加一层内部结构，但不要动它的注册树和公开路径；说明凭什么相信没动，门必须绿" (add an internal layer to this object, without touching its registration tree and public paths; explain why you believe it's untouched; gate must be green).

The g3 tree: 160K, 13 files, 102 nodes. Let me look: it's a xirang control-button host with `.xirang`, examples, src, tests, build.rs, Cargo.toml. Let me read its structure and lib.rs, control.rs, and one object file.

Plan for g3:
1. Read the tree (files, lib.rs, control.rs, an object file) via codegraph.
2. Determine the target object (probably Button, the README example).
3. Add an internal layer: e.g. `ButtonParts { label: String }` + `impl PartsContract for ButtonParts` + `Button { parts: ButtonParts }` + `pub fn parts(&self) -> &ButtonParts` — following the fb precedent (I saw fb's tree has exactly this!). Hmm — fb's tree (13333fd5 = same as fa) contains `ButtonParts` already... wait, that was the OLD damaged fb copy which had a "done state". Now fa and fb are restored to the clean state.

So for g3 the target is probably the same object (Button) — but which object? The question says "这个对象" (this object) — in the g3 tree there may be a designated object (e.g. the one with a `Parts`-shaped hook). Let me read the tree to find out.

4. Prove the registration tree and public paths unchanged:
   - Registration tree: run something that prints the outline (the host has `outline()` in lib.rs; maybe an example binary prints it). Before/after comparison of the outline.
   - Public paths: the object's module path (`control_button::control::object::button::Button`) and the face registration path (`root/control/button`) unchanged; also `git`-free check.
   - Gate green: cargo test before/after.

For g3 I need a "before" baseline of the registration outline. Since g3 is initially green, I can run the example binary (or a test that prints the outline) before and after the change and diff.

Let me explore g3's tree first.

**fa**: "我要加一个新对象，和现有的差不多" — add a new object similar to an existing one. In the round-9 fa tree (restored to the clean host), the deliverable: what I did, basis, command sequence, total calls; gate green after. So: add a new face (e.g. `Dial` like the existing `Button`) to the host — a new file `src/control/object/<new>/<new>.rs` with the `control_object!` macro, and (for a grafted slot) maybe also the graft side; plus (per the entry-plan rule I learned in h3) the entry must name the new face if it's an object the app publishes... Hmm! In the fa tree, is the new object required to be in the graft plan? Let me look at the fa tree's structure: it has src/, examples/, tests/, .xirang, build.rs. fa is the "control-button" host (chromium README example). Let me read its lib.rs and the tests to see what "adding an object" requires (maybe there's a test that enumerates the tree, e.g. `tests/` asserting the outline).

**fb**: "这个对象内部还不够" — deepen the object's inside: add the internal layer (Parts) — like g3! Hmm, g3 and fb are similar. g3's phrasing: "给这个对象加一层内部结构，但不要动它的注册树和公开路径" — the "不要动公开路径" constraint. fb: "这个对象内部还不够" (the object's inside isn't enough) → add internal structure. So fb = same operation, without the explicit "don't touch public paths" constraint (but obviously you shouldn't break the tree).

Hmm, but g3's tree vs fb's tree are different trees (g3=18e80bbd, fb=13333fd5 — fa==fb). So fa and fb are the same clean host; g3 is a different host (maybe with a dial object or a different shape).

Wait, that's odd: fa and fb have the SAME tree but different questions ("add a new object" vs "deepen this object's inside"). That's by design (round 8/9 families).

And g3's tree (18e80bbd) — let me inspect what it contains.

**fd**: "这个对象有问题" (this object has a problem) → fd's tree: workspace with crates/core+report (ledger-like), initially red with 3 failing assertions (2 defects). So fd = fix the ledger defects → gate green. Similar to h1 but with its own defect set.

**fe**: "这几个对象都有问题" (several objects have problems) → fe's tree: same shape, 1 failing test initially → fix → green.

Hmm, fd/fe are described as "对象" (objects) but their trees are the ledger workspace... Their defect sets: fd has 3 failing assertions in one target (2 defects); fe has 1 failing test. Let me diff fd/fe vs s1 to find their defects quickly.

Let me now do g3 + fa + fb exploration, then fd/fe.

Let me start with the trees' file layouts (codegraph files) + diffs vs known references. For fa/fb (control-button host), diff vs the h1-supply-chain host? Both are the same "control-button" host shape but different generations. Let me just read fa's tree.

Batch: 
- `./cg.sh g3 files`, `./cg.sh fa files`, `./cg.sh fd files`, `./cg.sh fe files` (4 calls)
- diff fd vs s1, fe vs s1 (nonbridge, quick)

Let me do it.
```

**该步模型输出**（逐字）：
```
Now the five remaining hosts. First a layout + diff reconnaissance:
```

### ①2 日志第 2 条 · exit 0 · step 36（turn 2）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe && codegraph files`

**工具返回**（逐字）：
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

**该步推理**（逐字）：
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
- Update `handle_faces_record_plain_default_preset_and_parts_names` to reflect that button now records its declared names (while control/slider keep the plain defaults) — otherwise the gate would be red. Hmm, this test is precisely about the *omitted* case; after my change, button has them declared. The test's doc says "All three built-in faces take that shape, so this pins the shipped path" — after my change, not all three do. So I must update the test (allowed; g3 is 可改) and document it. Also I could keep the same *intent* by asserting the plain defaults for control/slider and the declared names for button, plus a new assertion on `contract.required_parts/provided_parts` for button (the "not just a label" check, mirroring the toolchain's own test).

Wait — but hold on: is changing that test acceptable given the question says "不要动它的注册树和公开路径"? Editing a test doesn't move the tree or paths ✓.

Hmm, alternatively I could add the internal layer *without* declaring `preset:`/`parts:` in the macro — e.g. just define the types + impls + an accessor. But then the layer isn't *in* the registration record (no contract), which is weaker ("凭什么相信没动" would still hold, but the layer would be inert). The framework's notion of internal structure is precisely preset/parts; the old fb answer did it via the macro. I'll do the real thing (declare them) — that's the meaningful work — and update the test accordingly, documenting why.

For **fb** ("这个对象内部还不够"): same work on the same tree (its copy). fb's answer: add the internal structure (parts/preset) to the object → gate green → the internal layer is real: `contract.provided_parts == ["label"]`, and the face can expose `ButtonParts`.

For **fa** ("我要加一个新对象，和现有的差不多"): add a **new object** (a third leaf face), similar to the existing ones:
- new file `src/control/object/toggle/toggle.rs` modeled on button.rs (kind Toggle, exports, parent control::NODE_ID, handle_contracts [ControlHandle], flow, runtime_checks [NON_EMPTY_TEXT]).
- Wire at the parent? The docs say the parent keeps no child roster; the child declares `parent:`. But the build-time scope narrows to the entry's named subtrees: lib.rs's `static_graft_plan!` names button and slider cuts. A *new* face not named by the plan... In the h3 analysis, a face not named by the plan is NOT shipped ⇒ `outline()` wouldn't include it ⇒ the registry test's expected rows (3 rows) would fail if the face were shipped; conversely if it's not shipped, the tree stays 3 rows and the test passes! Hmm.

So: does adding toggle require plan declaration? The h3 case says the entry plan must name a face for it to be published. So to "add a new object" *and* have it appear in the tree, I must add a `cut(...) graft(...)` entry — but that requires a graft implementation of the new face (in the carrier's graft crate, which I'd have to extend too — the carrier is outside my question tree... hmm, `../carrier/control-button-graft` is a sibling fixture in the round9 trees dir; I copied `carrier` into my probe trees ✓ so I *could* edit it (it's a copy) — but it's shared by fa/fb/fc/g3; editing it would affect other questions' trees. Since each question's build uses `trees/<id>/../carrier/...` = the same `trees/carrier`, editing it would be shared. Hmm, that's messy but it's my copy; I could copy the carrier per question... but the cargo path is `../carrier/...` relative to the tree, so all trees share it. If I add `toggle_fast` to the carrier's graft crate, it doesn't affect fa/fb/g3/fd/fe as long as their trees don't reference it ✓ (adding a new module to the carrier crate is additive; the carrier crate's lib.rs must declare it, which doesn't affect others).

Hmm, but wait: is the graft side needed? The plan's cut/graft pairs are for *replaceable slots*: `cut(A) graft(B)` says the host face A can be replaced by the external implementation B. For a *new* object I add, a plain (non-grafted) face doesn't need a graft partner... but the plan is what scopes the build. Hmm — in this framework, is the `static_graft_plan!` the only scope source? The docs in fa's lib.rs:36-43 say: "Every `cut(` declared here is a slot the host hands over, and the build-time scope narrows to the subtrees these cuts name: a face nobody declared is not shipped by this application." So yes: to ship a new face, it must appear in the plan (as a cut) — implying it also needs a graft target.

Hmm, that makes fa's "add a new object" = add the face + a graft twin + the plan entry. That's substantial but doable:
1. `src/control/object/toggle/toggle.rs` (new face, modeled on button.rs).
2. `carrier/control-button-graft/src/toggle_fast.rs` (new graft impl, modeled on button_fast.rs) + declare `pub mod toggle_fast;` in the graft lib.rs.
3. `src/lib.rs`: add `cut(crate::control::object::toggle::NODE_ID) graft(control_button_graft::toggle_fast::NODE_ID),`.
4. Update tests that pin the tree: r
…[推理：此处截断，全文 20176 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**该步模型输出**（逐字）：
```
fd and fe are ledger-defect fixes (like the r-trees). Applying and verifying:
```

### ①3 日志第 3 条 · exit 0 · step 38（turn 2）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe && codegraph init .`

**工具返回**（逐字）：
```
┌  Initializing CodeGraph
│
◆  Initialized in /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe
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

**该步推理**（逐字）：
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

Let me write both files now, then move to fb/g3/fa.

Also, I should keep the "题面只有用户那一句话" shape: fd = "这个对象有问题。" fe = "这几个对象都有问题。" The deliverable: 做了什么、依据、命令序列、总调用次数.

Let me write them compactly (≤20 lines).
```

**同一步并行发出的其它调用**：
- `write` → 归属 共享:s1,fd：`{"content": "# fd —「这个对象有问题。」（初始红：同一 target 3 条断言 = 2 处缺陷）\n\n① 症状：`crates/report/tests/report.rs` 一个 target 里三条断言红：`the_minimum_amount_filter_keeps_the_entries_at_or_above_it`（`:27` left 1/right 2）、`the_report_prints_its_count_before_the_entries`（`:38` expect 炸）、`the_report_names_its_count_and_its_
…[参数：此处截断，全文 1383 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

### ①4 日志第 4 条 · exit 0 · step 72（turn 2）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe && codegraph files`

**工具返回**（逐字）：
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

### ①5 日志第 5 条 · exit 0 · step 75（turn 2）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe && codegraph node crates/report/src/query.rs`

**工具返回**（逐字）：
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

**该步推理**（逐字）：
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
- The s3/g1 boundary note from the verifier: I should acknowledge and, if cheap, amend s3/g1 answers with the framework-test boundary. Hmm — the verifier said s3/g1 were judged "partial" because the preset required declaring that 5 `#[test]`s are invoked by the framework at runtime and aren't reached by other code. For those trees, the 5 `#[test]`s are: core/tests/audit.rs's 1 test, report/tests/buckets.rs's 1, report/tests/report.rs's 3 = 5 ✓. So the expected answer should note: the test functions themselves are entry points invoked by the harness (not reachable from other code) — "没有任何测试能到达" applies to *non-test* code, and I should say what I can't see (including that the 5 `#[test]` bodies are only reached by the framework).

Since the captain asked me to "补一行" for the 总调用 and to mind that boundary "续跑时注意同类边界", I can cheaply append an evidence note to s3.md and g1.md (append-only supplement) mentioning the 5 framework-invoked `#[test]` entries. That's honest and improves correctness. Let me count the tests per tree: for the s3 tree: `crates/core/tests/audit.rs` has 1 test; `crates/report/tests/buckets.rs` 1; `crates/report/tests/report.rs` 3 → 5 ✓.

Let me append a short line to s3.md and g1.md:
`补注（复核者口径）：本树共 5 个 `#[test]`（core/tests/audit.rs 1、report/tests/buckets.rs 1、report/tests/report.rs 3）——它们是**框架运行期直接调用**的入口，不被任何其它代码"到达"，因此上面的可达性结论只针对**非测试代码**。`

Careful: the answers are filed as deliverables; appending a supplement is allowed (and the captain explicitly asked for the 总调用 line addition). This is a distinct, requested improvement — fine.

Let me do: BATCHING.md update + s3/g1 supplements, then report.

Note: I must not break the ≤20-line limit: s3 is 11 lines, g1 17 → +1 line each = 12/18 ✓ fine.

Let me write the patch.
```

### ①6 日志第 6 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe && codegraph node crates/core/src/audit.rs`

**工具返回**（逐字）：
```
**crates/core/src/audit.rs** — 17 lines, 2 symbols · no other indexed file depends on it

1	//! Audit totals, behind the `audit` feature.
2	//! 审计合计，门控在 `audit` 特性之后。
3	
4	use crate::model::Entry;
5	
6	/// How many entries the audit counts.
7	/// 审计计入多少条分录。
8	pub fn audit_count(entries: &[Entry]) -> usize {
9	    entries.iter().filter(|entry| entry.amount != 0).count()
10	}
11	
12	/// The audit's own view, never called by the default build.
13	/// 审计自己的视图，默认构建从不调用它。
14	pub fn audit_unused(entries: &[Entry]) -> usize {
15	    audit_count(entries) + 1
16	}
17
```


### 该题的**非仪器**调用（臂自己记的，日志里带 `=== CMD(nonbridge)`）

**非桥 1**：`bash -c cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe && CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target-fe cargo test --offline > /tmp/fresh_fe.out 2>&1; rc=$?; cat /tmp/fresh_fe.out; exit $rc`
```
Compiling ledger-core v0.1.0 (/home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe/crates/core)
   Compiling ledger-report v0.1.0 (/home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe/crates/report)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.91s
     Running unittests src/lib.rs (/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target-fe/debug/deps/ledger_core-0bf361681d20344b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/audit.rs (/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target-fe/debug/deps/audit-d07a61c9c79f199b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src
…[输出：此处截断，全文 2329 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**非桥 2**：`bash -c cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe && CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target-fe cargo test --offline  > /tmp/fe_t.out 2>&1; rc=$?; cat /tmp/fe_t.out; exit $rc`
```
Compiling ledger-report v0.1.0 (/home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe/crates/report)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.38s
     Running unittests src/lib.rs (/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target-fe/debug/deps/ledger_core-0bf361681d20344b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/audit.rs (/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target-fe/debug/deps/audit-d07a61c9c79f199b)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target-fe/debug/deps/ledger_report-050a5
…[输出：此处截断，全文 1958 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**非桥 3**：`bash -c cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe && CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target-fe cargo test --offline --all-features > /tmp/fe_t.out 2>&1; rc=$?; cat /tmp/fe_t.out; exit $rc`
```
Compiling ledger-core v0.1.0 (/home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe/crates/core)
   Compiling ledger-report v0.1.0 (/home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/fe/crates/report)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.86s
     Running unittests src/lib.rs (/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target-fe/debug/deps/ledger_core-0e5be57e8ec2a1c5)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/audit.rs (/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target-fe/debug/deps/audit-9455a446cd1e084c)

running 1 test
test the_audit_counts_the_non_zero_entries ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered o
…[输出：此处截断，全文 2110 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```


## 二、该题相关的其它调用（读文件 / 编辑 / 跑测试 / 写答案），带当时的推理


## 三、交付答案（逐字）

# fe —「这几个对象都有问题。」（初始红：默认面 1 条红 + 1 处默认面看不见的缺陷）

① 症状：默认面红 1 条：`the_minimum_amount_filter_keeps_the_entries_at_or_above_it`（`crates/report/tests/report.rs:27`，left 1/right 2）；另有一处**默认面看不见**的缺陷（只在 audit 面现形）。
② 根因（2 处、两个对象）：**A** `crates/report/src/query.rs:27` `if entry.amount > min`——方向反（应 `<`）；**B** `crates/core/src/audit.rs:9` `.filter(|entry| entry.amount == 0)`——把"非零"写反（应 `!= 0`），它被两道 cfg 挡在默认面外（模块 `crates/core/src/lib.rs:9-10` 的 `#[cfg(feature = "audit")]` + 测试 `crates/core/tests/audit.rs:10` 的 `#![cfg(feature = "audit")]`）⇒ 默认面绿、audit 面红。
   机制：A 数出 1 条（只留 -500）；B 让 `audit_count` 去数零金额分录，与它自己的文档"审计计入非零分录"相反。
③ 改动清单：`query.rs:27` `>`→`<`；`audit.rs:9` `==`→`!=`。**未动**测试、未动 `render.rs`（这棵树没缺 `write_count`）。
④ 反证：A 的 `left: 1` 与反向语义一致。B 单靠默认面**证伪不了**（它绿）——所以我跑了 `--all-features` 面：只修 A 时那一面会红，修 B 后两面都绿（默认面 4 passed rc=0；`--all-features` 5 passed rc=0）。能证伪我的观察：若 B 其实被别的门控挡住而永不编译，`--all-features` 面不会有那条 `1 passed` 的 audit 测试。
⑤ 命令序列 / 调用清单：codegraph `files`；`node crates/report/src/query.rs`、`node crates/core/src/audit.rs`（改前/改后）、`node crates/core/tests/audit.rs`；`callers audit_count`；`diff -r trees/s1 trees/fe`（定位 2 处差异）；非 codegraph：`cargo test --offline` ×4（改前 2 次：默认面+all-features；改后 2 次，per-tree `CARGO_TARGET_DIR`）。

总调用：成功 6 / 被拒 0（codegraph；含 2 次 init ⇒ 4 步）

nonbridge: 5 次 / 约 8.5 KB（`diff` 1 次 + cargo 4 次，全部在 logs/fe.txt）