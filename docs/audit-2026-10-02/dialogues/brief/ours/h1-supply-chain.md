# ours · `h1-supply-chain`（精简版；完整版见 `../ours/h1-supply-chain.md`）

**题面**：hardbug h1：供应链驳杂——同族一个兄弟调用了另一族名字，症状是偏移 160≠136。

> 仪器调用 **4** · 步 4 · 输出 3,194 tok · 推理 8,521 字符 · 累计上下文 1,112,832

## 仪器调用序列（工具给了什么 + 当时 agent 在想什么）

### ①1 `--call consistency --root target/round9/trees/h1-supply-chain/host --parent root/control`  · exit 0 · step 149（turn 1）

**工具返回**：
```
tree derived now (no published records at /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain/host/target/xirang/out; cannot read /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain/host/target/xirang/out/source_scope.tsv: No such file or directory (os error 2))
family root/control · member xirang-example-control-button · 9 member(s)
  badge                    1 call(s): to_local
  button                   1 call(s): to_local
  dial                     1 call(s): to_local
  gauge                    1 call(s): to_local
  panel                    1 call(s): to_local
  slider                   1 call(s): to_local
  spinner                  1 call(s): to_local
  timeline                 1 call(s): to_local
  toggle                   1 call(s): to_world
  outlier     toggle: does not call `to_local`, which the other siblings call; calls `to_world`, which no sibling calls
outliers: 1 of 9
not covered by this comparison: it reads the **derived** tree's sibling set and each sibling's own text, so a convention that lives in a shared helper, in generated code, or in a parent rule is not visible here; and `api` compares the names called, not the units or the arithmetic — an outlier is a place to look, not a defect; and a call written inside a macro body is not read as a call (the kernel's rule), so an object whose whole body is one macro invocation reads as calling nothing
next   `read {path, line}` for the outlier's body, `explain {node}` for its declared fields
evidence xirang.consistency {"parent":"root/control"} → exit 0

```

**当时推理**：
```
**h3**: test `every_declared_widget_ships` fails: "`dial` is declared and does not ship". The tree has 9 rows, no dial. So the dial widget exists in the sources (declared) but doesn't ship — the entry plan never names it. Let me find the "declaration" the test refers to (probably a `DECLARED` list in the test or a declaration in the sources).

**h4**: tests pass (green). The defect: one algorithm does the opposite of its documentation, no test covers the branch. From the source:
- `zero_arm()` at `src/model/entry.rs:30-32` **always returns `ZeroArm::Post`** — the enum at :5-10 has two variants (Post/Refuse) with docs; the `Refuse` arm is never constructed (never returned). `postable()` at :38-46: has_receipt check, then `match self.zero_arm() { Post => true, Refuse => false }`. Since zero_arm always returns Post, a zero-amount entry WITH a receipt is postable — contradicting the doc at :36-37 "a zero entry never is". And the `ZeroArm::Refuse` arm at :44 is uncovered (no test covers it). Also the tests only cover the receipt branch.

Hmm: "exactly one branch inverted and exactly one branch uncovered" — the inverted branch: `zero_arm` returning Post instead of Refuse (`Post`/`Refuse` inverted: "A zero entry is posted like any other" is the wrong arm). Hmm, which is "inverted"? The doc says a zero entry never posts; so `zero_arm` should return `Refuse`. The function's own doc says "Which arm decides a zero amount" — hmm. The inversion could also be described as: `match self.zero
…[推理：此处截断，全文 4080 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**同一步还发了**：`bash`→h2-claim-unkept

### ①2 `--call why --root target/round9/trees/h1-supply-chain/host --at src/control/object/toggle/toggle.rs:15`  · exit 0 · step 153（turn 1）

**工具返回**：
```
at src/control/object/toggle/toggle.rs:15 — the definition `offset` (lines 15-17)
  contract   (from the doc above it, which is the promise this tree writes)
             src/control/object/toggle/toggle.rs:14 /// This widget's offset, in the coordinate family its siblings use.
  callers    tests/offsets.rs::the_rendered_offsets_add_up (outside this file's directory (tests))
  pins       a test names `offset`; `check {face}` is the run that shows it still holds
  adoption   no ledger at .xirang/adopted/entries in this root
  gate       no `#[cfg]` attribute sits directly above this definition at src/control/object/toggle/toggle.rs:15 (a gate on the enclosing module is not read here)
  scope      scope unknown (no source_scope.tsv; run `xirang check`)
  pruning    pruning unknown (no pruning_manifest.tsv; run `xirang check`)
  wiring     declared cut `crate::control::object::toggle::NODE_ID` → `control_button_graft::toggle_fast::NODE_ID` (full=false) at /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain/host/src/lib.rs:48
not covered here: how a grafted subtree looks at runtime (ask `trace`), and whether the declarations above describe the tree **as it is now** — the scope and pruning lines are the build's own output, and a tree built before the last edit says so on the `scope` line
next   `check {face}` to run the face that compiles it, `grafts` for every declared cut, `locate {symptom}` for sibling places to compare it with
evidence xirang.why {"at":"src/control/object/toggle/toggle.rs:15"} → exit 0

```

**当时推理**：
```
h3 is fully diagnosed: the entry plan at `src/lib.rs:48-66` declares 8 cuts (button, slider, timeline, gauge, toggle, spinner, panel, badge) and **dial is absent**; the insertion point is between `:64-65` (badge's cut) and `:66` (`);`). The graft crate already provides `dial_fast` (graft/src/lib.rs:17,32).

Now, the mechanism: the build-time scope narrows to the subtrees the cuts name (scope.rs:249-318); a face no cut names is not selected → the generated tree doesn't mount/ship it → `outline()` lacks `root/control/dial` → tests/shipped.rs:19 fails.

Hmm, one nuance: in h3's host, is dial mounted? The dial face's module is not mounted (like fa), yet `registry` (derived tree) shows it — because the derived tree is a source-scan view, not the shipped tree. And `why` at dial.rs:15 worked (file-level). Also `check`'s census said 20 of 22 production functions unreachable — because the test only names widgets by string. Fine.

Wait — but did h3's host compile dial.rs? If dial isn't mounted, `tests/shipped.rs` never references it as a Rust path (only strings). And `why` read the file. So dial.rs is unmounted → the "widget that exists in the sources does not appear in the shipped registry tree" — exactly the class. ✓

Now, for h1-supply-chain, I need to check the impact claim: expected 136 = 8×17; actual 160 = 7×17 + 41. Also which file: `toggle.rs:16` calls `to_world` (import at :3; definition at control.rs:8). Fix: import & call `to_local` instead (toggle.rs:3 and :16), or 1 line i
…[推理：此处截断，全文 10068 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**同一步还发了**：`bash`→h1-supply-chain

### ①3 `--call callgraph --root target/round9/trees/h1-supply-chain/host --function offset`  · exit 0 · 共享/装置步

**工具返回**：
```
evidence: static-heuristic
matches 9
note: 9 definitions match `offset`; pass `path` to select one. Callers are matched by name across the whole tree, so for a common name they include unrelated call sites.
src/control/object/badge/badge.rs:15 fn offset
  callers (1): tests/offsets.rs::the_rendered_offsets_add_up (outside this file's directory (tests))
  callees: to_local -> src/control/control.rs:5
  also here: paint -> src/control/object/badge/badge.rs:9
  tests: tests/offsets.rs
  source:
    contract /// This widget's offset, in the coordinate family its siblings use.
    impl     pub fn offset(x: i32) -> i32 {
    impl         to_local(x)
    impl     }
src/control/object/button/button.rs:15 fn offset
  callers (1): tests/offsets.rs::the_rendered_offsets_add_up (outside this file's directory (tests))
  callees: to_local -> src/control/control.rs:5
  also here: paint -> src/control/object/badge/badge.rs:9
  tests: tests/offsets.rs
  source:
    contract /// This widget's offset, in the coordinate family its siblings use.
    impl     pub fn offset(x: i32) -> i32 {
    impl         to_local(x)
    impl     }
src/control/object/dial/dial.rs:15 fn offset
  callers (1): tests/offsets.rs::the_rendered_offsets_add_up (outside this file's directory (tests))
  callees: to_local -> src/control/control.rs:5
  also here: paint -> src/control/object/badge/badge.rs:9
  tests: tests/offsets.rs
  source:
    contract /// This widget's offset, in the coordinate family its siblings use.
    impl     pub fn offset(x: i32) -> i32 {
    impl         to_local(x)
    impl     }
src/control/object/gauge/gauge.rs:15 fn offset
  callers (1): tests/offsets.rs::the_rendered_offsets_add_up (outside this file's directory (tests))
  callees: to_local -> src/control/control.rs:5
  also here: paint -> src/control/object/badge/badge.rs:9
  tests: tests/offsets.rs
  source:
    contract /// This widget's offset, in the coordinate family its siblings use.
    impl     pub fn offset(x: i32) -> i32 {
    impl         to_local(x)
    impl     }
src/control/object/panel/panel.rs:15 fn offset
  callers (1): tests/offsets.rs::the_rendered_offsets_add_up (outside this file's directory (tests))
  callees: to_local -> src/control/control.rs:5
  also here: paint -> src/control/object/badge/badge.rs:9
  tests: tests/offsets.rs
  source:
    contract /// This widget's offset, in the coordinate family its siblings use.
    impl     pub fn offset(x: i32) -> i32 {
    impl         to_local(x)
    impl     }
… truncated: 4 of 9 definitions withheld at the limit of 5; pass `path` for one of: `src/control/object/slider/slider.rs`, `src/control/object/spinner/spinner.rs`, `src/control/object/timeline/timeline.rs`, `src/control/object/toggle/toggle.rs`; `limit` raises this cap
dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.
next   `read {path, line}` for a body, `affected {files}` for what depends on it
evidence xirang.callgraph {"function":"offset"} → exit 0

```

### ①4 `--call check --root target/round9/trees/h1-supply-chain/host --face default --timeout_ms 900000`  · exit 0 · 共享/装置步

**工具返回**：
```
verdict  failed (cargo exit 101)
check  cargo test
face   default
root   /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain/host
tree   14 rust file(s), 24 function(s)
elapsed 22811 ms
log    /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain/host/target/xirang/out/check-default.log
exit   101
result test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
result test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
failed the_rendered_offsets_add_up
why    the_rendered_offsets_add_up: thread 'the_rendered_offsets_add_up' (514984) panicked at tests/offsets.rs:18:5: assertion `left == right` failed: the rendered offsets add up to 160, not 136
next   the `why` lines above are the failing assertion's own words: a short, stable phrase from one is a string literal in this tree, so `search {literal: "the rendered offsets add up to"}` finds the line that produced it. If two red things may be independent, two green runs are not the evidence: fix one and re-run, and say which red survived. And a probe you built yourself that disagrees with the source is a reason to re-read that line (`read`, `search {literal}`) before rebuilding — a second look is cheaper than a second build
census: 0 named numeric constant(s); a static fact about this tree, not a verdict
  declarations: 4 production `pub fn` name(s) appear in no test file (a text-level count: a test that reaches one without writing its name does not count here)
  test-reachable: 11 of 22 production function(s) no test can reach (24 function(s) indexed in this tree; a static walk from the test files along the same name-in-call-list rule the orphan view uses)
  branch-level: 0 constructively unreachable arm(s) in this tree (0 `false` guard(s), 0 never-constructed variant(s); a static read of the source text, not a coverage measurement)
  entry plan: 11 `cut(` site(s) and 9 `graft(` site(s) across this tree's sources (a static count; which of them this application ships is the plan's own business)
  decl   no test names `to_local` (src/control/control.rs:5)
  decl   no test names `to_world` (src/control/control.rs:8)
  decl   no test names `base_registry` (src/lib.rs:70)
  not covered: the reachability column (dynamic dispatch, function pointers, FFI, macro expansion, trait methods and closures are invisible; matching is by name; `main` is never listed) · the branch column (no data-dependent condition is judged, `false` is the only guard literal decided, a `pub` enum is never judged, and a construction this tree does not spell would falsify a row) · this census (string constants, structural duplication, runtime behaviour and claims written in prose are outside it). A static read of the source text, not a coverage measurement; `census: true` and `--list check` have the full text
… truncated: 3 of 12 census rows withheld at the limit of 5; pass `census: true` for the whole table (every column head is already here with its count)
evidence xirang.check {"face":"default","timeout_ms":900000} → exit 0

```


## 交付答案（逐字）

# h1-supply-chain — 同族里一个兄弟调用了与其余兄弟不同的那一族名字
root cause — `src/control/object/toggle/toggle.rs:16`（调用）＋ `:3`（import）：`to_world(x)`。
同族其余 8 个（badge/button/dial/gauge/panel/slider/spinner/timeline）在同一位置都是 `to_local(x)`；
两个族定义在 `src/control/control.rs:5`（`to_local = x + 7`，局部族）与 `:8`（`to_world = x + 31`，世界族）。
mechanism — `tests/offsets.rs:9-17` 把 8 个 `offset(10)` 相加。兄弟各给 17，toggle 给 41 ⇒
7×17 + 41 = **160**，而"每个部件都按兄弟那族摆放"的总和是 8×17 = **136**（差值 24 恰为 31−7）。
toggle 自己的注释 `:14`（"in the coordinate family its siblings use"）与它的调用相反，所以单看每个文件都"对"。
整棵树里没有任何第二个 `to_world` 调用者（`consistency --parent root/control` 的 9 个成员只点 toggle）。
evidence — ① `cargo test --offline`（root=h1-supply-chain/host，exit **101**）：
`the_rendered_offsets_add_up ... FAILED`，`assertion left == right failed: the rendered offsets add up to 160, not 136`。
② `check {face:default}` → `verdict failed (cargo exit 101)`，同上（log `…/host/target/xirang/out/check-default.log`）。
③ `consistency {parent:root/control}` → `badge/button/dial/gauge/panel/slider/spinner/timeline 1 call(s): to_local`、
`toggle 1 call(s): to_world`、`outlier toggle: does not call to_local … calls to_world, which no sibling calls`、`outliers: 1 of 9`。
④ `why {at:toggle.rs:15}` → 契约行 `:14` 仍是"兄弟那族"；callers=`tests/offsets.rs::the_rendered_offsets_add_up`。
⑤ `callgraph {function:offset}` → 每个兄弟 `callees: to_local -> src/control/control.rs:5`。
⑥ 算术：8×17=136、7×17+41=160、41−17=24。
counter-proof — 若错的是别处（例如 `to_local` 本身或另一个部件），则只改 toggle 一处**不会**让总和变 136。
我在 **/tmp 的副本**（不碰本树）里把 `toggle.rs` 的 `to_world` 全改成 `to_local` 后重跑：
`the_rendered_offsets_add_up ... ok`、`test result: ok. 1 passed; 0 failed`（exit **0**）。
另一侧：若两个部件都错，160−136=24 就该是两个差值之和；实测差值恰等于单次 `to_world−to_local`。
fix — 最小改动：`src/control/object/toggle/toggle.rs:3` 与 `:16` 的 `to_world` → `to_local`（同一文件、两处标识符）。
未改动：`tests/offsets.rs`（不改测试）、`src/control/control.rs`（两个坐标族都是设计的一部分）、
其余 8 个部件文件、`src/lib.rs` 的 graft 计划（toggle 是槽位，但缺陷在宿主自己的文件里）。
本树**只读**，以上修改未落盘；探针只作用于 /tmp 的副本。
调用清单（`target/round9/logs/h1-supply-chain.jsonl`）：consistency(parent) · why(toggle.rs:15) · callgraph(offset) · check(default)。
总调用：成功 4 / 被拒 0
