# ours · `fa`（精简版；完整版见 `../ours/fa.md`）

**题面**：五族：「我要加一个新对象，和现有的差不多。」

> 仪器调用 **10** · 步 24 · 输出 14,374 tok · 推理 28,676 字符 · 累计上下文 5,558,016

## 仪器调用序列（工具给了什么 + 当时 agent 在想什么）

### ①1 `--call registry --root target/round9/trees/fa`  · exit 0 · step 91（turn 1）

**工具返回**：
```
namespace xirang-example-control-button
tree derived now (no published records at /home/nich/Moirai_N3/nichlink/target/round9/trees/fa/target/xirang/out; cannot read /home/nich/Moirai_N3/nichlink/target/round9/trees/fa/target/xirang/out/source_scope.tsv: No such file or directory (os error 2))
faces 3
root/control                             Control        control/control.rs                     fb97ddd5f2b803d1b7f40a776d8a22d9
root/control/button                      Button         control/object/button/button.rs        ff1c57d9b110ef57eac134a3cb4e46b6
root/control/slider                      Slider         control/object/slider/slider.rs        bdb4427ce81c9bc51e56bee7667fd2be
next   `explain {node}` for one face's contract, `check {face}` for whether it builds
evidence xirang.registry {} → exit 0

```

**当时推理**：
```
`static_plan_allocations.rs:188-192` asserts `builtin_static_plan().grafts().len() == 2` — that's about cut slots, not faces. Adding a leaf face doesn't change the graft plan (only if I declared a cut for it). Since `lib.rs`'s static_graft_plan declares cuts for button and slider only, adding a Dial without a cut is fine. But wait: `lib.rs:36-43` says "Every cut( declared here is a slot the host hands over, and the build-time scope narrows to the subtrees these cuts name: a face nobody declared is not shipped by this application." — Hmm! "a face nobody declared is not shipped by this application" — the build-time scope narrows to the subtrees the cuts name. So adding a Dial under root/control: is root/control itself a cut? The cuts are button and slider (both leaves). The scope = those subtrees. Then would Dial be excluded from the shipped tree? Let me test: after adding the face, `registry` (which shows the tree "as the build sees them") will tell. If Dial doesn't appear, I need to add a cut for it too (and then update the static-cut-count pin from 2 to 3 and the doc comments).

Hmm, this is the h3-cross-file-chain class's exact mechanism ("the entry plan never names the new face, so the build-time scope leaves it out of the build")! For fa, the same trap may apply: adding a face requires naming it in the entry plan (lib.rs's static_graft_plan?) — no wait, the `cut(` declares *replaceable slots*, not "which faces ship". Let me re-read:

"这里声明的每个 `cut(` 都是宿主交出去的槽位，而构建期作用域收窄到这
…[推理：此处截断，全文 5187 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①2 `--call consistency --root target/round9/trees/fa --parent root/control`  · exit 0 · step 92（turn 1）

**工具返回**：
```
tree derived now (no published records at /home/nich/Moirai_N3/nichlink/target/round9/trees/fa/target/xirang/out; cannot read /home/nich/Moirai_N3/nichlink/target/round9/trees/fa/target/xirang/out/source_scope.tsv: No such file or directory (os error 2))
family root/control · member xirang-example-control-button · 2 member(s)
  button                   0 call(s): none
  slider                   0 call(s): none
outliers: 0 of 2
not covered by this comparison: it reads the **derived** tree's sibling set and each sibling's own text, so a convention that lives in a shared helper, in generated code, or in a parent rule is not visible here; and `api` compares the names called, not the units or the arithmetic — an outlier is a place to look, not a defect; and a call written inside a macro body is not read as a call (the kernel's rule), so an object whose whole body is one macro invocation reads as calling nothing
next   `read {path, line}` for the outlier's body, `explain {node}` for its declared fields
evidence xirang.consistency {"parent":"root/control"} → exit 0

```

**当时推理**：
```
The preview shows the new face file content. Note it emits BOTH `handle_traits: ["ControlHandle"]` and `handle_contracts: [crate::control::ControlHandle]` — interesting: the authoring derives handle_traits from the contracts. Wait, it emitted handle_traits because I didn't pass handle_traits... Let me look: the generated macro has `handle_traits: ["ControlHandle"]` — derived from handle_contracts (the "path decides the label" rule). Good.

The new file needs `impl ControlHandle for Dial` to compile since handle_contracts requires it.

Also: `crate::{NoParts, NoPreset}` import — will `crate::NoParts` resolve? The host's `host!()` re-exports kernel types at the crate root, so likely yes.

And the note: "entry plan: 13 cut( and 12 graft( site(s); this face's name appears at none of them".

Now let me apply and then complete the file with the impl. Then check whether the face ships in the build (the registry test pin).

Let me apply.
```

### ①3 `--call apply --root target/round9/trees/fa --json {"action":"add","parent":"root/control","fields":{"module":"dial","kind":"Dial","exports":"control.render","handle_contracts":"crate::control::ControlHandle","flow":"control.render.v1|1|ControlInput|ControlFrame"}}`  · exit 0 · step 123（turn 1）

**工具返回**：
```
action preview
namespace xirang-example-control-button
fields: module kind preset parts name_zh name_en summary_zh summary_en exports stable_name needs_registry getting_from_other_registry registration_rule admission handle_traits handle_contracts part_traits part_contracts requires provides runtime_checks flow flow_provider — every value is a string; `needs_registry` is the one boolean, and `exports`/`handle_traits`/`requires` are spelled as strings rather than arrays
would write /home/nich/Moirai_N3/nichlink/target/round9/trees/fa/src/control/object/dial/dial.rs
declaration src/control/object/dial/dial.rs:11
preview effect: created `control/object/dial/dial.rs` under parent fb97ddd5f2b803d1b7f40a776d8a22d9
faces 4
  root/control  Control  control/control.rs
  root/control/button  Button  control/object/button/button.rs
  root/control/dial  Dial  control/object/dial/dial.rs
  root/control/slider  Slider  control/object/slider/slider.rs
consequences (static, text-level): 0 in-tree test line(s) name this face
  entry plan: 13 `cut(` and 12 `graft(` site(s); this face's name appears at none of them — whether the application ships it is the plan's own business
  not covered: this lists test lines that spell the face's name; a test that counts faces without naming it, or reaches it through another spelling, does not appear here — run the suite before believing either list

diff:
+ src/control/object/dial/dial.rs
+// generated-by=XiRang
+//! Dial registration face.
+//! Dial 注册面。
+
+use crate::{NoParts, NoPreset};
+
+/// Registration-only marker for the Dial face.
+/// 仅用于 Dial 注册面的 handle 标记，不代表运行时 object 实现。
+pub struct Dial;
+
+crate::control_object! {
+    kind: Dial,
+    exports: ["control.render"],
+    parent: crate::control::NODE_ID,
+    handle_traits: ["ControlHandle"],
+    handle_contracts: [crate::control::ControlHandle],
+    flow: crate::FlowContract::new(crate::ContractId::new("control.render.v1"), 1, "ControlInput", "ControlFrame"),
+}
evidence xirang.apply {"action":"add","fields":{"exports":"control.render","flow":"control.render.v1|1|ControlInput|ControlFrame","handle_cont… → exit 0

```

**当时推理**：
```
fa gate green: 25 passed in registry, 1 in health_check, 1 in static_plan_allocations, 0/0 others.

Now let me run the MCP evidence for fa after the change: `check {face:default}`, `registry` (4 faces), `consistency --parent root/control` (dial vs siblings), and `explain --node root/control/dial`. Also `apply` preview showed earlier. Let me capture the scope manifest as evidence.

Then write fa.md.

Then fb (deepen on the same tree): fb is the same tree content as fa (sha identical!) — but wait: since fa and fb have the same sha256, they're identical copies. I've now modified fa. fb remains the clean tree. So fb = "这个对象内部还不够" → deepen an object: `apply {action:"deepen", node:"root/control/button", inside:{parts:{...}}}` + check green. Note in fb the tree is clean (no dial).

Hmm, which object for fb? Same question as g3 — but for fb the object is presumably... the same ambiguity. The g3 answer used button (canonical in the tool's tests). For fb I'll deepen `root/control/button` as well. Hmm, but then fb and g3 are nearly identical deliveries. That's fine (different rounds/questions, same tree shape).

Hmm, wait. Let me reconsider. In the round-7 family b (「这个对象内部还不够」), the fifth round's story was: "第五轮五族 b 的题面是'这个对象内部还不够'，而桥当时唯一的写（add）作用在注册树上——于是'更深'只能被读成'它下面再挂一个面'" (from run_deepen's doc). So family b is exactly the deepen question, and the fixture's canonical node in the tool's tests is `root/control/button`. Good: fb → deepen button.

Let me now:
1. Run fa evidence calls +
…[推理：此处截断，全文 1564 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①4 `--call apply --root target/round9/trees/fa --json {"action":"add","parent":"root/control","fields":{"module":"dial","kind":"Dial","exports":"control.render","handle_contracts":"crate::control::ControlHandle","flow":"control.render.v1|1|ControlInput|ControlFrame"},"apply":true}`  · exit 0 · step 124（turn 1）

**工具返回**：
```
action apply
namespace xirang-example-control-button
fields: module kind preset parts name_zh name_en summary_zh summary_en exports stable_name needs_registry getting_from_other_registry registration_rule admission handle_traits handle_contracts part_traits part_contracts requires provides runtime_checks flow flow_provider — every value is a string; `needs_registry` is the one boolean, and `exports`/`handle_traits`/`requires` are spelled as strings rather than arrays
applied /home/nich/Moirai_N3/nichlink/target/round9/trees/fa/src/control/object/dial/dial.rs
declaration src/control/object/dial/dial.rs:11
created `control/object/dial/dial.rs` under parent fb97ddd5f2b803d1b7f40a776d8a22d9
faces 4
  root/control  Control  control/control.rs
  root/control/button  Button  control/object/button/button.rs
  root/control/dial  Dial  control/object/dial/dial.rs
  root/control/slider  Slider  control/object/slider/slider.rs
consequences (static, text-level): 0 in-tree test line(s) name this face
  entry plan: 13 `cut(` and 12 `graft(` site(s); this face's name appears at none of them — whether the application ships it is the plan's own business
  not covered: this lists test lines that spell the face's name; a test that counts faces without naming it, or reaches it through another spelling, does not appear here — run the suite before believing either list
evidence xirang.apply {"action":"add","apply":true,"fields":{"exports":"control.render","flow":"control.render.v1|1|ControlInput|ControlFrame"… → exit 0

```

**当时推理**：
```
The consistency call with both `parent` and `specimen` was refused (exit 1) — that's my call error (usage), recorded in the log as exit 1. So fa's tally has 1 refused. I need to re-run consistency with just one of them.

Let me re-run `consistency --specimen root/control/dial` (compares siblings against the declared shape) and also `consistency --parent root/control`.

Note: `specimen` requires a ledger anchor? "specimen derives the parent from the anchor and compares them against the shape the ledger certifies". fa has no ledger → maybe it errors. Let me try both and keep the one that works.

Also the scope manifest is at the cargo OUT_DIR. Let me print it.

Let me run those.
```

### ①5 `--call registry --root target/round9/trees/fa`  · exit 0 · step 168（turn 1）

**工具返回**：
```
namespace xirang-example-control-button
tree derived now (no published records at /home/nich/Moirai_N3/nichlink/target/round9/trees/fa/target/xirang/out; cannot read /home/nich/Moirai_N3/nichlink/target/round9/trees/fa/target/xirang/out/source_scope.tsv: No such file or directory (os error 2))
faces 4
root/control                             Control        control/control.rs                     fb97ddd5f2b803d1b7f40a776d8a22d9
root/control/button                      Button         control/object/button/button.rs        ff1c57d9b110ef57eac134a3cb4e46b6
root/control/dial                        Dial           control/object/dial/dial.rs            e1070833a032e106f9cdd0eff2149bc2
root/control/slider                      Slider         control/object/slider/slider.rs        bdb4427ce81c9bc51e56bee7667fd2be
next   `explain {node}` for one face's contract, `check {face}` for whether it builds
evidence xirang.registry {} → exit 0

```

**当时推理**：
```
Excellent verification:
- The 12 read-only trees (s1–s8, g4, and the 4 hardbug classes) are **byte-identical** to TREES.json's recorded hashes ⇒ I did not modify them (only ran cargo/tool reads).
- The 11 editable/修改 trees differ, exactly the set I edited (r1–r4, fd, fe, h1, fa, fb, fc, g3).

This is a great piece of evidence for the report and for APPARATUS-free operation.

Now let me write batching.md. Content:
- Definition of the tally (bridge calls only, per logs/<id>.jsonl; exit 0 = 成功, 非 0 = 被拒).
- Statement: no bridge call was ever shared between two problems; each problem's calls are in its own log; all multi-id shell commands were reconnaissance (find/diff/wc/grep/sha256sum) with **no** bridge call, hence per-题 token attribution is unaffected.
- List the multi-id recon commands (the ones that touched >1 tree id):
  1. `find`/`ls` enumerations of all trees (recon).
  2. `diff -ru r1 {r2,r3,r4}` (src+tests; recon).
  3. `diff -ru s1 {s2,s5,s6,s7,s8}` (recon).
  4. `diff -ru fa fc` and `diff -ru s1/{h1,fd,fe}` (recon).
  5. `wc -l` per tree (recon).
  6. The final tree-hash recomputation across all 26 trees (recon, no bridge calls).
- Also note: the two probe runs in /tmp (h1-supply-chain, h3, h4) used cargo only, no bridge calls, and did not touch the question trees.
- And: the tallies per题.

Let me also note the per-id bridge call counts so a reader can verify attribution: I'll list them compactly.

Let me write batching.md.
```

**同一步还发了**：`bash`→装置

### ①6 `--call consistency --root target/round9/trees/fa --parent root/control --specimen root/control/dial`  · exit 1 · 共享/装置步

**工具返回**：
```
consistency takes `parent` or `specimen`, not both: `parent` compares siblings against their own majority, while `specimen` derives the parent from the anchor and compares them against the shape the ledger certifies — accepted shape: {"specimen":"<anchor>","root":"<path>"}
```

### ①7 `--call explain --root target/round9/trees/fa --node root/control/dial`  · exit 0 · 共享/装置步

**工具返回**：
```
namespace xirang-example-control-button
node e1070833a032e106f9cdd0eff2149bc2
  path root/control/dial
  kind Dial
  source control/object/dial/dial.rs
  module control::object::dial
  parent fb97ddd5f2b803d1b7f40a776d8a22d9
  registry_name dial
build stale (run `xirang check`)
scope unknown (no source_scope.tsv; run `xirang check`)
pruning unknown (no pruning_manifest.tsv; run `xirang check`)
next   `callgraph {function}` for its callers and callees, `read {path, line}` for the body
evidence xirang.explain {"node":"root/control/dial"} → exit 0

```

### ①8 `--call check --root target/round9/trees/fa --face default --timeout_ms 900000`  · exit 0 · 共享/装置步

**工具返回**：
```
verdict  passed (cargo exit 0)
check  cargo test
face   default
root   /home/nich/Moirai_N3/nichlink/target/round9/trees/fa
tree   14 rust file(s), 54 function(s)
elapsed 24612 ms
log    /home/nich/Moirai_N3/nichlink/target/round9/trees/fa/target/xirang/out/check-default.log
exit   0
result test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
result test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
result test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
result test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
result test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
result test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
census: 0 named numeric constant(s); a static fact about this tree, not a verdict
  declarations: 2 production `pub fn` name(s) appear in no test file (a text-level count: a test that reaches one without writing its name does not count here)
  test-reachable: 6 of 10 production function(s) no test can reach (54 function(s) indexed in this tree; a static walk from the test files along the same name-in-call-list rule the orphan view uses)
  branch-level: 0 constructively unreachable arm(s) in this tree (0 `false` guard(s), 0 never-constructed variant(s); a static read of the source text, not a coverage measurement)
  entry plan: 13 `cut(` site(s) and 12 `graft(` site(s) across this tree's sources (a static count; which of them this application ships is the plan's own business)
  decl   no test names `shipped_object_nodes` (src/lib.rs:66)
  decl   no test names `outline` (src/lib.rs:87)
  not covered: the reachability column (dynamic dispatch, function pointers, FFI, macro expansion, trait methods and closures are invisible; matching is by name; `main` is never listed) · the branch column (no data-dependent condition is judged, `false` is the only guard literal decided, a `pub` enum is never judged, and a construction this tree does not spell would falsify a row) · this census (string constants, structural duplication, runtime behaviour and claims written in prose are outside it). A static read of the source text, not a coverage measurement; `census: true` and `--list check` have the full text
… truncated: 4 of 12 census rows withheld at the limit of 5; pass `census: true` for the whole table (every column head is already here with its count)
evidence xirang.check {"face":"default","timeout_ms":900000} → exit 0

```

### ①9 `--call consistency --root target/round9/trees/fa --parent root/control`  · exit 0 · 共享/装置步

**工具返回**：
```
tree derived now (no published records at /home/nich/Moirai_N3/nichlink/target/round9/trees/fa/target/xirang/out; cannot read /home/nich/Moirai_N3/nichlink/target/round9/trees/fa/target/xirang/out/source_scope.tsv: No such file or directory (os error 2))
family root/control · member xirang-example-control-button · 3 member(s)
  button                   0 call(s): none
  dial                     0 call(s): none
  slider                   0 call(s): none
outliers: 0 of 3
not covered by this comparison: it reads the **derived** tree's sibling set and each sibling's own text, so a convention that lives in a shared helper, in generated code, or in a parent rule is not visible here; and `api` compares the names called, not the units or the arithmetic — an outlier is a place to look, not a defect; and a call written inside a macro body is not read as a call (the kernel's rule), so an object whose whole body is one macro invocation reads as calling nothing
next   `read {path, line}` for the outlier's body, `explain {node}` for its declared fields
evidence xirang.consistency {"parent":"root/control"} → exit 0

```

### ①10 `--call consistency --root target/round9/trees/fa --specimen root/control/dial`  · exit 0 · 共享/装置步

**工具返回**：
```
no ledger entry names `root/control/dial` in /home/nich/Moirai_N3/nichlink/target/round9/trees/fa — the ledger holds 0 entry(ies), and a specimen comparison is against the entry in force (an adoption is a lease: the newest line wins). `adopted` lists the anchors.
evidence xirang.consistency {"specimen":"root/control/dial"} → exit 0

```


## 交付答案（逐字）

# fa —「我要加一个新对象，和现有的差不多。」（可改；交付后门必须绿）
**① 做了什么** 在 `root/control` 下横向加了第三个对象面 **`root/control/dial`**（`src/control/object/dial/dial.rs`），
形状照着兄弟 `slider`（`exports: ["control.render"]`、`handle_contracts: [crate::control::ControlHandle]`、
同一条 `control.render.v1` flow），并让它在构建期作用域里真的**发布**（见 ③ 的坑）。
**② 改动清单** - `apply {action:add, parent:root/control}`（先 `action preview` 后 `apply:true`）生成面文件；按其形状补
  `impl ControlHandle for Dial`（与 `slider.rs:10-14` 同形），否则 `handle_contracts` 编译不过。
- `src/lib.rs`：新增 `shipped_object_nodes()`，**在代码里拼出** `…::button/slider/dial::NODE_ID`
  （入口必须提到新面，见 ③）。
- `tests/registry.rs` 三处出厂形状钉子随新面移动：树行 3→4（:54-61）、静态计划面数 3→4（:284）、
  作用域选中行 2→3 并加 dial 的行断言（:368-…）。
**③ 依据（这一步是本题的坑，也是「差不多」的判据）** `apply` 自己提示：`entry plan: … this face's name appears at none of them — whether the application ships it is the plan's own business`。
构建期作用域从入口的可达源码收窄（`toolchain/src/build_time/src/scope.rs:241-356`），而参考扫描
**忽略 import、字符串与注释**（`kernel/src/registry_core/syntax/reference_scan.rs`，
钉子 `face_tests.rs:130`）⇒ 一开始我用 `use`/`pub use` 提及它，作用域仍是 `# selected 2`、dial 连模块都没挂
（`crate::control::object::dial` 未解析，E0432）。改成代码里拼出路径后：`source_scope.tsv` `# selected 3`、
`registry` 由 3 面变 4 面。
**④ 反证 + 门** - 若 dial 没真发布，`registry` 仍是 3 面、`source_scope.tsv` 仍是 2 行——两者现在都是 4 面 / 3 行。
- 若要「和现有的差不多」只看文件像不像：`consistency --parent root/control` → `3 member(s)`、`outliers: 0 of 3`。
- 门：`check {face:default}` → `verdict passed (cargo exit 0)`（1 + 25 + 1 passed，0 failed，1 ignored）。
  未把 `static_plan_allocations` 的「槽位数 2」钉子动掉——dial 不是槽位，`grafts().len()` 仍 2。
**⑤ 调用清单（`target/round9/logs/fa.jsonl`）** - `registry`（改前 3 面） · `consistency {parent:root/control}`（改前 2 成员）。
- `apply {action:add, …, }` 预览 → `faces 4`；`apply {…, apply:true}` → 落盘。
- `registry`（改后 4 面） · `explain {node:root/control/dial}` · `check {face:default}` → passed。
- `consistency {parent:root/control}` → 3 成员 0 异类；`consistency {specimen:root/control/dial}` → 本树无台账（已答）。
- 一次被**拒**的调用：`consistency` 同时传 `parent` 与 `specimen`（工具要求二选一），已改单传重跑。
- shell（非桥调用）：`cargo test --offline`（改前绿 → 命名后 3 条钉子红 → 更新钉子后绿）。
总调用：成功 9 / 被拒 1
