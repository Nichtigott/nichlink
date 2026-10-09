# ours · `fc`（精简版；完整版见 `../ours/fc.md`）

**题面**：五族：「已经采信了，现在再横向加一个。」

> 仪器调用 **11** · 步 12 · 输出 5,860 tok · 推理 9,527 字符 · 累计上下文 3,076,608

## 仪器调用序列（工具给了什么 + 当时 agent 在想什么）

### ①1 `--call adopted --root target/round9/trees/fc`  · exit 0 · step 129（turn 1）

**工具返回**：
```
evidence: adoption ledger at /home/nich/Moirai_N3/nichlink/target/round9/trees/fc/.xirang/adopted/entries (provisional by construction)
adopted since 12:32:28 (unchanged since the confirmation) (provisional) — root/control/button: the button renders its label and reports its offset in the family coordinate system [evidence: the shipped shape pins in tests/ plus a manual render at 2 scales; confirmed by nich; why: agreed before the horizontal extension]
provisional 1  lapsed 0
note: nothing here renews or revokes an adoption — a lapsed one needs a person to confirm the new state, and a confirmation is one more line. A later line for the same anchor is the one in force; an earlier line stays as its history, which is why a lapsed line can sit above a provisional one and still be the truth about the bytes it named.
next   a route this ledger does not name is a **new anchor** — a first confirmation, not a renewal: pass `anchor`, `certifies`, `evidence`, `verifier`, `reason` and `files` together with `apply: true` and `confirm: true`, and this tool appends one line whose fingerprint it computes from those files
evidence xirang.adopted {} → exit 0

```

**当时推理**：
```
Wait: fc's button.rs now sha256 = e842be74... (same as the original fa button.rs). But the ledger says the adopted fingerprint is 22b54a4c...! And the diff showed NO differences in `src/control/object/button/button.rs` between fa and fc (only lib.rs, tests/registry.rs, and the new dial dir differ — the latter two because I modified fa).

Hold on: fc/button.rs = e842be74 ≠ ledger's 22b54a4c ⇒ **the ledger entry's fingerprint does NOT match the current bytes**! That's the fc story: "已经采信了，现在再横向加一个" — hmm, but TREES.json says fc = "clean host + 真实采信台账（root/control/button 生效）——门绿且台账在" (clean host + real ledger (root/control/button in force) — gate green and ledger present).

Hmm, "root/control/button 生效" (in force). But the fingerprint 22b54a4c doesn't match e842be74... Let me compute the sha256 exactly as the tool does. Maybe the tool's fingerprint is a different hash algorithm/format (e.g. sha256 of the file with normalization, or blake3 truncated). The `adopted` tool will tell us. Let me run `adopted`.

Also the TREES.json hash algorithm note says sha256 with `\0` separators etc. for the *tree* hash, not the ledger fingerprint. The ledger fingerprint might be computed by the tool differently (e.g. sha256 of the file bytes). Let me just run `adopted` and see what it says about entry validity.

Let me run `adopted` and `conformance` for fc now.
```

### ①2 `--call conformance --root target/round9/trees/fc --anchor root/control/button`  · exit 0 · step 130（turn 1）

**工具返回**：
```
anchor `root/control/button` — 1 revision(s) in the ledger; the last is in force (an adoption is a lease: the newest line wins)
  in force   provisional — certifies: the button renders its label and reports its offset in the family coordinate system (adopted 12:32:28 by nich)
  bytes      unchanged since the confirmation
  covers     src/control/object/button/button.rs
not covered here: whether the siblings of this anchor's object follow the same shape (ask `consistency --specimen <anchor>`, which reads the declared fields from the files this entry covers and compares the siblings against them), and the shape of the ledger's own history beyond the newest line
next   `adopted` for every entry's verdict, `consistency --specimen root/control/button` for whether the siblings follow this shape
evidence xirang.conformance {"anchor":"root/control/button"} → exit 0

```

**当时推理**：
```
`adopted` says: "adopted since 12:32:28 (unchanged since the confirmation) (provisional)"; "provisional 1 lapsed 0". `conformance` says "bytes unchanged since the confirmation" and it covers `src/control/object/button/button.rs`.

So the ledger's fingerprint (22b54a4c...) must be a normalized fingerprint matching e842be74's file bytes (maybe sha256 of the file's text with the "generated marker"/whitespace normalization... or maybe it's the sha256 of the file's *source-relative* content computed by the build). Whatever — the tool says the bytes are unchanged. Good. Note: fc's `pruning_manifest`/`function_manifest` may hold fingerprints; the ledger fingerprint may be a different hash (e.g. blake3). Not important.

Now the fc work: "已经采信了，现在再横向加一个" — add a sibling object horizontally, keeping the ledger.

Plan:
1. `adopted` + `conformance` (before) ✓ (done).
2. `apply add` a new sibling under root/control (dial, same shape as slider/button).
3. Complete it (impl ControlHandle) + name it from the entry (lib.rs code-spelled path) → shipped.
4. Update the two shape pins in tests/registry.rs (rows 3→4; scope selected 2→3 + dial row; plan.faces 3→4).
5. Verify: ledger file byte-identical (sha256 of .xirang/adopted/entries before/after), `adopted` → provisional 1 / lapsed 0 (still in force), `conformance` → bytes unchanged, `consistency --specimen root/control/button` (siblings vs the certified shape — interesting: "if you add one horizontally, does the new sibling follow the adopte
…[推理：此处截断，全文 2136 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①3 `--call registry --root target/round9/trees/fc`  · exit 0 · step 131（turn 1）

**工具返回**：
```
namespace xirang-example-control-button
tree derived now (no published records at /home/nich/Moirai_N3/nichlink/target/round9/trees/fc/target/xirang/out; cannot read /home/nich/Moirai_N3/nichlink/target/round9/trees/fc/target/xirang/out/source_scope.tsv: No such file or directory (os error 2))
faces 3
root/control                             Control        control/control.rs                     fb97ddd5f2b803d1b7f40a776d8a22d9
root/control/button                      Button         control/object/button/button.rs        ff1c57d9b110ef57eac134a3cb4e46b6
root/control/slider                      Slider         control/object/slider/slider.rs        bdb4427ce81c9bc51e56bee7667fd2be
next   `explain {node}` for one face's contract, `check {face}` for whether it builds
evidence xirang.registry {} → exit 0

```

**当时推理**：
```
Baseline: `consistency --specimen root/control/button` → 1 sibling (slider), 0 deviations. The ledger's certified shape: exports `control.render`, handle_traits `ControlHandle`.

Now apply the add, complete the face (impl + entry naming), update pins, and re-run the specimen comparison — the new sibling should conform (exports + handle_traits) → 0 of 2 deviations (which also proves the new object follows the adopted shape — "横向加一个，和已采信的形状一致").

Let me do the add and the follow-up edits.
```

### ①4 `--call consistency --root target/round9/trees/fc --specimen root/control/button`  · exit 0 · step 138（turn 1）

**工具返回**：
```
specimen root/control/button — ledger revision 1 is the one in force (an adoption is a lease: the newest line wins)
state      provisional — certifies: the button renders its label and reports its offset in the family coordinate system (adopted 12:32:28 by nich)
covers     src/control/object/button/button.rs
shape      exports `control.render` · handle_traits `ControlHandle`
tree derived now (no published records at /home/nich/Moirai_N3/nichlink/target/round9/trees/fc/target/xirang/out; cannot read /home/nich/Moirai_N3/nichlink/target/round9/trees/fc/target/xirang/out/source_scope.tsv: No such file or directory (os error 2))
family root/control · member xirang-example-control-button · 1 sibling(s)
conformance: 0 of 1 sibling(s) do not carry the specimen's shape
not covered by this comparison: it reads the shape the ledger's files declare (parts, exports, handle_traits, part_traits) through the kernel's face parser and compares each sibling's own file against it, so a shape stated in a shared helper, produced by a macro this tree does not spell, or carried only by runtime behaviour is invisible; `parts` is compared by presence alone because it names each object's own parts type, while each label the specimen states in `exports`, `handle_traits` and `part_traits` has to appear in the sibling's list — a label the sibling states and the specimen does not is that sibling's own business; a sibling that declares **more** fields than the specimen is not reported, because the specimen is the baseline and the question is whether it was followed; and a deviation is a place to look, not a defect — whether the design should still be this way is the ledger's own question
next   `read {path, line}` for a deviating sibling's body, `conformance {anchor}` for the ledger's own verdict on this specimen
evidence xirang.consistency {"specimen":"root/control/button"} → exit 0

```

**当时推理**：
```
fc gate green. Now verify the ledger and run the specimen comparison after:
1. ledger sha256 unchanged (already checked after the add: 697a5380...; let me re-check now after all edits).
2. `adopted` → provisional 1 / lapsed 0.
3. `conformance --anchor root/control/button` → bytes unchanged.
4. `consistency --specimen root/control/button` → now 2 siblings, 0 deviations (the new one follows the adopted shape).
5. `check` (MCP) green.
6. `registry` → 4 faces.

Let me run the fc evidence block.
```

### ①5 `--call apply --root target/round9/trees/fc --json {"action":"add","parent":"root/control","fields":{"module":"dial","kind":"Dial","exports":"control.render","handle_contracts":"crate::control::ControlHandle","flow":"control.render.v1|1|ControlInput|ControlFrame"}}`  · exit 0 · 共享/装置步

**工具返回**：
```
action preview
namespace xirang-example-control-button
fields: module kind preset parts name_zh name_en summary_zh summary_en exports stable_name needs_registry getting_from_other_registry registration_rule admission handle_traits handle_contracts part_traits part_contracts requires provides runtime_checks flow flow_provider — every value is a string; `needs_registry` is the one boolean, and `exports`/`handle_traits`/`requires` are spelled as strings rather than arrays
would write /home/nich/Moirai_N3/nichlink/target/round9/trees/fc/src/control/object/dial/dial.rs
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

### ①6 `--call apply --root target/round9/trees/fc --json {"action":"add","parent":"root/control","fields":{"module":"dial","kind":"Dial","exports":"control.render","handle_contracts":"crate::control::ControlHandle","flow":"control.render.v1|1|ControlInput|ControlFrame"},"apply":true}`  · exit 0 · 共享/装置步

**工具返回**：
```
action apply
namespace xirang-example-control-button
fields: module kind preset parts name_zh name_en summary_zh summary_en exports stable_name needs_registry getting_from_other_registry registration_rule admission handle_traits handle_contracts part_traits part_contracts requires provides runtime_checks flow flow_provider — every value is a string; `needs_registry` is the one boolean, and `exports`/`handle_traits`/`requires` are spelled as strings rather than arrays
applied /home/nich/Moirai_N3/nichlink/target/round9/trees/fc/src/control/object/dial/dial.rs
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

### ①7 `--call adopted --root target/round9/trees/fc`  · exit 0 · 共享/装置步

**工具返回**：
```
evidence: adoption ledger at /home/nich/Moirai_N3/nichlink/target/round9/trees/fc/.xirang/adopted/entries (provisional by construction)
adopted since 12:32:28 (unchanged since the confirmation) (provisional) — root/control/button: the button renders its label and reports its offset in the family coordinate system [evidence: the shipped shape pins in tests/ plus a manual render at 2 scales; confirmed by nich; why: agreed before the horizontal extension]
provisional 1  lapsed 0
note: nothing here renews or revokes an adoption — a lapsed one needs a person to confirm the new state, and a confirmation is one more line. A later line for the same anchor is the one in force; an earlier line stays as its history, which is why a lapsed line can sit above a provisional one and still be the truth about the bytes it named.
next   a route this ledger does not name is a **new anchor** — a first confirmation, not a renewal: pass `anchor`, `certifies`, `evidence`, `verifier`, `reason` and `files` together with `apply: true` and `confirm: true`, and this tool appends one line whose fingerprint it computes from those files
evidence xirang.adopted {} → exit 0

```

### ①8 `--call conformance --root target/round9/trees/fc --anchor root/control/button`  · exit 0 · 共享/装置步

**工具返回**：
```
anchor `root/control/button` — 1 revision(s) in the ledger; the last is in force (an adoption is a lease: the newest line wins)
  in force   provisional — certifies: the button renders its label and reports its offset in the family coordinate system (adopted 12:32:28 by nich)
  bytes      unchanged since the confirmation
  covers     src/control/object/button/button.rs
not covered here: whether the siblings of this anchor's object follow the same shape (ask `consistency --specimen <anchor>`, which reads the declared fields from the files this entry covers and compares the siblings against them), and the shape of the ledger's own history beyond the newest line
next   `adopted` for every entry's verdict, `consistency --specimen root/control/button` for whether the siblings follow this shape
evidence xirang.conformance {"anchor":"root/control/button"} → exit 0

```

### ①9 `--call consistency --root target/round9/trees/fc --specimen root/control/button`  · exit 0 · 共享/装置步

**工具返回**：
```
specimen root/control/button — ledger revision 1 is the one in force (an adoption is a lease: the newest line wins)
state      provisional — certifies: the button renders its label and reports its offset in the family coordinate system (adopted 12:32:28 by nich)
covers     src/control/object/button/button.rs
shape      exports `control.render` · handle_traits `ControlHandle`
tree derived now (no published records at /home/nich/Moirai_N3/nichlink/target/round9/trees/fc/target/xirang/out; cannot read /home/nich/Moirai_N3/nichlink/target/round9/trees/fc/target/xirang/out/source_scope.tsv: No such file or directory (os error 2))
family root/control · member xirang-example-control-button · 2 sibling(s)
conformance: 0 of 2 sibling(s) do not carry the specimen's shape
not covered by this comparison: it reads the shape the ledger's files declare (parts, exports, handle_traits, part_traits) through the kernel's face parser and compares each sibling's own file against it, so a shape stated in a shared helper, produced by a macro this tree does not spell, or carried only by runtime behaviour is invisible; `parts` is compared by presence alone because it names each object's own parts type, while each label the specimen states in `exports`, `handle_traits` and `part_traits` has to appear in the sibling's list — a label the sibling states and the specimen does not is that sibling's own business; a sibling that declares **more** fields than the specimen is not reported, because the specimen is the baseline and the question is whether it was followed; and a deviation is a place to look, not a defect — whether the design should still be this way is the ledger's own question
next   `read {path, line}` for a deviating sibling's body, `conformance {anchor}` for the ledger's own verdict on this specimen
evidence xirang.consistency {"specimen":"root/control/button"} → exit 0

```

### ①10 `--call registry --root target/round9/trees/fc`  · exit 0 · 共享/装置步

**工具返回**：
```
namespace xirang-example-control-button
tree derived now (no published records at /home/nich/Moirai_N3/nichlink/target/round9/trees/fc/target/xirang/out; cannot read /home/nich/Moirai_N3/nichlink/target/round9/trees/fc/target/xirang/out/source_scope.tsv: No such file or directory (os error 2))
faces 4
root/control                             Control        control/control.rs                     fb97ddd5f2b803d1b7f40a776d8a22d9
root/control/button                      Button         control/object/button/button.rs        ff1c57d9b110ef57eac134a3cb4e46b6
root/control/dial                        Dial           control/object/dial/dial.rs            e1070833a032e106f9cdd0eff2149bc2
root/control/slider                      Slider         control/object/slider/slider.rs        bdb4427ce81c9bc51e56bee7667fd2be
next   `explain {node}` for one face's contract, `check {face}` for whether it builds
evidence xirang.registry {} → exit 0

```

### ①11 `--call check --root target/round9/trees/fc --face default --timeout_ms 900000`  · exit 0 · 共享/装置步

**工具返回**：
```
verdict  passed (cargo exit 0)
check  cargo test
face   default
root   /home/nich/Moirai_N3/nichlink/target/round9/trees/fc
tree   14 rust file(s), 54 function(s)
elapsed 23212 ms
log    /home/nich/Moirai_N3/nichlink/target/round9/trees/fc/target/xirang/out/check-default.log
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
  decl   no test names `shipped_object_nodes` (src/lib.rs:63)
  decl   no test names `outline` (src/lib.rs:84)
  not covered: the reachability column (dynamic dispatch, function pointers, FFI, macro expansion, trait methods and closures are invisible; matching is by name; `main` is never listed) · the branch column (no data-dependent condition is judged, `false` is the only guard literal decided, a `pub` enum is never judged, and a construction this tree does not spell would falsify a row) · this census (string constants, structural duplication, runtime behaviour and claims written in prose are outside it). A static read of the source text, not a coverage measurement; `census: true` and `--list check` have the full text
… truncated: 4 of 12 census rows withheld at the limit of 5; pass `census: true` for the whole table (every column head is already here with its count)
evidence xirang.check {"face":"default","timeout_ms":900000} → exit 0

```


## 交付答案（逐字）

# fc —「已经采信了，现在再横向加一个。」（可改；交付后台账必须还在、门必须绿）
**① 台账先说（采信现状）** `.xirang/adopted/entries` 里 1 条：`root/control/button`（provisional，12:32:28，verifier `nich`，
覆盖 `src/control/object/button/button.rs`，理由「agreed before the horizontal extension」）。
`adopted` → `provisional 1  lapsed 0`；`conformance {anchor:root/control/button}` → `bytes unchanged since the confirmation`。
台账文件字节：本轮**开工前与全部改动后同为一枚** `sha256 697a5380e448c1e52baa3d0a60423b0c8a1a7556a7231490d567b6b9223e9052`。
**② 做了什么（横向加一个）** 在 `root/control` 下加了第三个对象面 **`root/control/dial`**（`src/control/object/dial/dial.rs`），
形状照 `slider`：`exports: ["control.render"]`、`handle_contracts: [crate::control::ControlHandle]`、
同一条 `control.render.v1` flow；补 `impl ControlHandle for Dial`。
`src/lib.rs` 加 `shipped_object_nodes()`，**在代码里拼出** dial 的 `NODE_ID`——入口不提它，构建期作用域会把它剪掉
（`toolchain/src/build_time/src/scope.rs`；扫描器忽略 import，见 `kernel/src/registry_core/syntax/reference_scan.rs`）。
随新面移动的出厂形状钉子：`tests/registry.rs` 树行 3→4、静态计划面数 3→4、作用域选中行 2→3 并加 dial 行断言。
**③ 对既有条目的处置（本题的硬要求）** - **不动**：没有改 `src/control/object/button/button.rs`（新面是新文件；入口与钉子是别的文件）⇒ 该条覆盖的字节没动，
  `conformance` 判 `bytes unchanged`，`adopted` 判 `provisional`（未失效、无需人确认）。
- **不伪造确认**：台账只认 `button` 这一条路线；新对象 `root/control/dial` 是一条**新锚**——按工具自己的话，
  它是「first confirmation, not a renewal」，需要人给 `anchor/certifies/evidence/verifier/reason/files`
  再追加一行。我没有替人签字，也没有删改历史行。
- **形状比对**：`consistency --specimen root/control/button`（改前 1 兄弟 0 异类）→ 改后 `2 sibling(s)`、
  `conformance: 0 of 2 sibling(s) do not carry the specimen's shape` ⇒ 新兄弟**跟随了已采信的 exports/handle_traits 形状**。
**④ 反证 + 门** 能证伪「台账没了/没保住」：台账文件哈希变了、或 `adopted` 的行数/裁定变了、或 `conformance` 报 bytes 变动——
三者都不是（见 ①③）。`check {face:default}` → `verdict passed (cargo exit 0)`；`registry` 由 3 面变 4 面。
**⑤ 调用清单（`target/round9/logs/fc.jsonl`）** - `adopted` / `conformance {anchor}` / `registry`（改前，台账与树各读一次）。
- `consistency {specimen:root/control/button}`（改前：1 兄弟 0 异类）。
- `apply {action:add,…}` 预览 → `faces 4`；`apply {…, apply:true}` → 落盘（**未写台账**）。
- `adopted`（改后：provisional 1 / lapsed 0） · `conformance`（改后：bytes unchanged）。
- `consistency {specimen}`（改后：2 兄弟 0 异类，新面跟随已采信形状） · `registry`（改后 4 面） · `check`（passed）。
- shell（非桥调用）：`sha256sum .xirang/adopted/entries` ×3（改前 / add 后 / 全部改动后：同一枚）；
  `cargo test --offline`（更新钉子后绿）。
总调用：成功 11 / 被拒 0
