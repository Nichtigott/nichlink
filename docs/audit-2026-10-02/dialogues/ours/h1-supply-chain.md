# ours · `h1-supply-chain`

**题面**：hardbug h1：供应链驳杂——同族一个兄弟调用了另一族名字，症状是偏移 160≠136。

> 步 4 · 仪器调用 **4**（逐题日志条数）· 会话里定位到 2 条 · 输出 3,194 tok · 推理 8,521 字符 · 累计上下文 1,112,832

## 一、工具给了什么 · 当时 agent 在想什么（逐条配对）

> ⚠️ 日志 4 条 vs 会话定位 2 条：差的那些调用发生在**共享步**（一条命令里跑了多道题）或**装置步**里 —— 这正是"逐题 token 不可归因"的具体形状。

### ①1 日志第 1 条 · exit 0 · step 149（turn 1）

**命令**：`--call consistency --root target/round9/trees/h1-supply-chain/host --parent root/control`

**工具返回**（逐字）：
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

**该步推理**（逐字）：
```
**h3**: test `every_declared_widget_ships` fails: "`dial` is declared and does not ship". The tree has 9 rows, no dial. So the dial widget exists in the sources (declared) but doesn't ship — the entry plan never names it. Let me find the "declaration" the test refers to (probably a `DECLARED` list in the test or a declaration in the sources).

**h4**: tests pass (green). The defect: one algorithm does the opposite of its documentation, no test covers the branch. From the source:
- `zero_arm()` at `src/model/entry.rs:30-32` **always returns `ZeroArm::Post`** — the enum at :5-10 has two variants (Post/Refuse) with docs; the `Refuse` arm is never constructed (never returned). `postable()` at :38-46: has_receipt check, then `match self.zero_arm() { Post => true, Refuse => false }`. Since zero_arm always returns Post, a zero-amount entry WITH a receipt is postable — contradicting the doc at :36-37 "a zero entry never is". And the `ZeroArm::Refuse` arm at :44 is uncovered (no test covers it). Also the tests only cover the receipt branch.

Hmm: "exactly one branch inverted and exactly one branch uncovered" — the inverted branch: `zero_arm` returning Post instead of Refuse (`Post`/`Refuse` inverted: "A zero entry is posted like any other" is the wrong arm). Hmm, which is "inverted"? The doc says a zero entry never posts; so `zero_arm` should return `Refuse`. The function's own doc says "Which arm decides a zero amount" — hmm. The inversion could also be described as: `match self.zero_arm() { Post => true, Refuse => false }` — that mapping is correct. So the defect is `zero_arm` returning `Post` (should be `Refuse`).

And "exactly one branch uncovered": `ZeroArm::Refuse => false` at :44 is the uncovered arm (census would say "1 never-constructed variant"? Actually `ZeroArm::Post` is constructed in `zero_arm`; `Refuse` is never constructed → the branch-level census should list `Refuse` as a never-constructed variant arm!). Let me verify with `check --census true` on the h4 tree — the branch column should report 1 constructively unreachable arm (`Refuse`), and the test-reachable column may report `zero_arm`/`normalized_account` etc.

Hmm, but the class says "恰有一支写反、恰有一支没被覆盖" (exactly one branch inverted and exactly one branch uncovered). For h4 the branch-level census should name the never-constructed variant.

Wait — is `normalized_account` also uncovered? Tests only cover `postable`. `signed()` and `normalized_account()` are presumably uncovered (no test names them). Hmm, "one branch uncovered" refers to a *branch* (arm), not a function. Let me get the census.

Also for h4 the BRIEF says: "One algorithm in one file does the opposite of what its own documentation promises, and no test covers the branch that does it." So: the algorithm = `zero_arm`/`postable` zero rule; the doc says a zero entry never posts; implementation posts it (opposite). The uncovered branch = the `Refuse` arm.

Now let me gather bridge evidence for all four classes, then write the answers.

For h4: `check {face:default, census:true}` → branch-level: 1 never-constructed variant (ZeroArm::Refuse) + test-reachable names. Also `digest {file}` — the tool "One bounded summary of one file, for the case the maintainer named: a file holding several algorithms where one branch of one of them is wrong." — that's exactly h4's tool! Let me use `digest --file src/model/entry.rs`.

For h3: read the test + find the declaration of dial + `why`/`affected`/`registry`.

For h2: `adopted` + `conformance` (button anchor OK; panel anchor lapsed) + `consistency --specimen root/control/button` (names the sibling missing a declaration).

For h1-supply-chain: `callgraph`/`consistency --parent root/control` (should name toggle as the outlier? "compares the siblings' own calls" — the API column compares called names: toggle calls `to_world`, siblings call `to_local` → the tool should name toggle as the outlier!) + read + `why`.

Let me run these per class, one bash call per class (so no cross-class batching needed).

Let me start with h1-supply-chain and h2-claim-unkept.
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h2-claim-unkept：`{"command": "cd /home/nich/Moirai_N3/nichlink; B=./target/debug/xirang-mcp; R=target/round9/trees/h2-claim-unkept/host; L=target/round9/logs/h2-claim-unkept.jsonl\necho '===== adopted'; $B --call adopted --root $R --log $L; echo \"EXIT=$?\"\necho '===== conformance button'; $B --call conformance -
…[参数：此处截断，全文 690 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

### ①2 日志第 2 条 · exit 0 · step 153（turn 1）

**命令**：`--call why --root target/round9/trees/h1-supply-chain/host --at src/control/object/toggle/toggle.rs:15`

**工具返回**（逐字）：
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

**该步推理**（逐字）：
```
h3 is fully diagnosed: the entry plan at `src/lib.rs:48-66` declares 8 cuts (button, slider, timeline, gauge, toggle, spinner, panel, badge) and **dial is absent**; the insertion point is between `:64-65` (badge's cut) and `:66` (`);`). The graft crate already provides `dial_fast` (graft/src/lib.rs:17,32).

Now, the mechanism: the build-time scope narrows to the subtrees the cuts name (scope.rs:249-318); a face no cut names is not selected → the generated tree doesn't mount/ship it → `outline()` lacks `root/control/dial` → tests/shipped.rs:19 fails.

Hmm, one nuance: in h3's host, is dial mounted? The dial face's module is not mounted (like fa), yet `registry` (derived tree) shows it — because the derived tree is a source-scan view, not the shipped tree. And `why` at dial.rs:15 worked (file-level). Also `check`'s census said 20 of 22 production functions unreachable — because the test only names widgets by string. Fine.

Wait — but did h3's host compile dial.rs? If dial isn't mounted, `tests/shipped.rs` never references it as a Rust path (only strings). And `why` read the file. So dial.rs is unmounted → the "widget that exists in the sources does not appear in the shipped registry tree" — exactly the class. ✓

Now, for h1-supply-chain, I need to check the impact claim: expected 136 = 8×17; actual 160 = 7×17 + 41. Also which file: `toggle.rs:16` calls `to_world` (import at :3; definition at control.rs:8). Fix: import & call `to_local` instead (toggle.rs:3 and :16), or 1 line if the import is shared: two edits in the same file (import + call). Files not touched: tests/offsets.rs, all 8 sibling widget files, control.rs (the two families are legitimate; both exist for a reason — hmm, is `to_world` legitimate? It's documented as "The world family: offsets are measured from the frame's origin" — the two coordinate families exist by design; so the defect is that toggle, whose doc says "in the coordinate family its siblings use", uses the world family).

Careful: maybe the intended fix is to change the *import* only? No: `to_world(x)` at :16 is the call. Minimal edit: `to_local` in both places (2 tokens in 1 file). I'll state: `src/control/object/toggle/toggle.rs:3` (`to_world` → `to_local`) and `:16` (`to_world(x)` → `to_local(x)`); tests and other files untouched.

Now let me get a couple more evidence items for h1-supply-chain: read control.rs:5/8 (I have it from the earlier grep: to_local = x+7 at :5, to_world = x+31 at :8) ✓, plus a counter-proof idea: "sum without toggle = 7*17 = 119; toggle alone with to_world gives 41; if the test's expected total were computed as all-local it's 136 = 8*17; changing toggle to to_local makes the sum 136." I can verify arithmetic by computing: 17*8 = 136 ✓, 160-136 = 24 = (41-17) ✓ (31-7 = 24).

Should I actually run a probe to prove the fix? The tree is read-only; but I could compute with a tiny standalone rust/python eval — no need: arithmetic is verifiable. Better counter-proof: run the offsets test with only toggle changed? I can't edit the tree (read-only). Hmm — the "counter-proof" field: "one thing you did that would have shown your diagnosis wrong". Options: (a) check that all 8 widgets are called by the test (so none is silently unused) — done by reading tests/offsets.rs:10-17 (all 8 named) ✓; (b) confirm the arithmetic difference is exactly the toggle delta: 160 - 136 = 24, and to_world(10) - to_local(10) = 41 - 17 = 24 ✓ — this shows one single wrong call explains the whole gap (if two widgets were wrong, the delta would be a sum of deltas); (c) confirm no sibling calls to_world (consistency: "calls to_world, which no sibling calls") ✓.

Actually, I *can* construct a stronger probe without editing the read-only tree: copy the tree to /tmp and change toggle's call there, then run the test. That doesn't edit the question tree. That's a legitimate counter-proof ("若改 toggle 为 to_local 后总数为 136，则诊断成立"). Let me do that: copy h1-supply-chain to /tmp/h1sc-probe (excluding target), sed the toggle file, run cargo test there with the shared target dir. The path deps are absolute (they point to the tree paths?) — hmm: the host's Cargo.toml has path deps to `../graft` (relative) and to the toolchain (absolute /home/nich/.../toolchain). If I copy the whole h1-supply-chain dir, `../graft` resolves within the copy ✓ and the toolchain path is absolute ✓. Let me try. This gives raw evidence for the fix without touching the tree.

Let me do that probe, and similarly for h4? For h4 the counter-proof: the census's "ZeroArm::Refuse never constructed" + doc mismatch; a probe: copy the tree, flip `zero_arm` to `Refuse`, and check that the branch-level census row disappears (and that a zero-entry-with-receipt now returns false). Hmm, but for h4 the current tests are green, so after the fix the tests would... still green (the tests never cover zero). And the census would flip to naming `Post` as never-constructed. That's a nice probe: it shows the "uncovered branch" moves, i.e. confirms the diagnosis.

For h2: counter-proof could be: check that the panel file's real digest differs from the ledger's and that button's matches — do that by running `conformance --anchor root/control/panel` (lapsed) and `--anchor root/control/button` (unchanged) — done ✓. Plus verify the ledger's two fingerprints are identical strings (grep) — they are (`edc72845...` twice) which proves "a copy of another entry's bytes" ✓.

For h3: counter-proof: show that a face the plan DOES name ships while dial doesn't — e.g. `badge` is in the plan and ships (the failing test's tree list includes badge) ✓; and the graft crate already has `dial_fast` (so the plan, not the graft, is the missing declaration). Also `why` says "no declared cut names root/control/dial". Good.

Let me run the h1-supply-chain probe (copy + fix + test) and the h4 probe (copy + fix + census) — wait, the census is a bridge call on the tree; for the probe copy I can run `check` via the bridge on the copy? The copy is inside /tmp, and
…[推理：此处截断，全文 10068 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h1-supply-chain：`{"command": "rm -rf /tmp/h1sc-probe && cp -r /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain /tmp/h1sc-probe && rm -rf /tmp/h1sc-probe/host/target && cd /tmp/h1sc-probe/host && sed -i 's/to_world/to_local/g' src/control/object/toggle/toggle.rs && grep -n \"to_local\\|to_world\" src
…[参数：此处截断，全文 581 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

### ①3 日志第 3 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`--call callgraph --root target/round9/trees/h1-supply-chain/host --function offset`

**工具返回**（逐字）：
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

### ①4 日志第 4 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`--call check --root target/round9/trees/h1-supply-chain/host --face default --timeout_ms 900000`

**工具返回**（逐字）：
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


## 二、该题相关的其它调用（读文件 / 编辑 / 跑测试 / 写答案），带当时的推理

**调用** `bash`（step 144）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain/host && grep -n '' tests/offsets.rs | head -60; echo '=== lib.rs'; grep -n '' src/lib.rs | head -60", "description": "Read h1-supply-chain offsets test"}
```

**结果**：
```
1://! The rendered layout: every sibling's offset, added up.
2://! 渲染出的布局：每个兄弟的偏移加总。
3:
4:/// The frame this tree renders when every widget places itself the way its siblings do.
5:const EXPECTED_TOTAL: i32 = 136;
6:
7:#[test]
8:fn the_rendered_offsets_add_up() {
9:    let total =
10:        control_button::control::object::button::offset(10) +
11:        control_button::control::object::slider::offset(10) +
12:        control_button::control::object::timeline::offset(10) +
13:        control_button::control::object::gauge::offset(10) +
14:        control_button::control::object::toggle::offset(10) +
15:        control_button::control::object::spinner::offset(10) +
16:        control_button::control::object::panel::offset(10) +
17:        control_button::control::object::badge::offset(10);
18:    assert_eq!(
19:        total, EXPECTED_TOTAL,
20:        "the rendered offsets add up to {total}, not {EXPECTED_TOTAL}"
21:    );
22:}
=== lib.rs
1://! XiRang 示例：README 里的 Control / Button 两层树，作为一个真实宿主库。
2://! XiRang example: the README Control/Button two-level tree as a real host
3://! library.
4://!
5://! 整个 crate 只有这里一处构建接线。`host!()` 引入构建期生成的注册计划；
6://! 注册面代码保持普通 Rust，父级不维护子对象清单。
7://! The crate has one build wiring point. `host!()` pulls in the plan the build
8://! step generated; face code stays ordinary Rust and no parent keeps a child
9://! roster.
10:
11:xirang_toolchain::runtime::host!();
12:
13:// 这个 crate 自己调用 `host!()`，所以类型化 graft 计划里的 `crate::...` 与生成
14:// 树解析到同一个 crate。宿主如果把库和二进制分开，计划必须写在调用 `host!()`
15:// 的那一个里；写在另一个 crate 里的 Rust 路径无法在这里解析。
16:// This crate calls `host!()` itself, so `crate::...` in a typed graft plan
17:// resolves in the same crate as the generated tree. A host that splits a library
18:// and a binary must keep the plan in whichever one calls `host!()`.
19:
20:// `host!()` 已在 crate 根重导出 kernel，协议名词因此已在作用域内；再显式导入
21:// 会用私有项遮蔽那个公开重导出。
22:// `host!()` re-exports the kernel at the crate root, so the protocol nouns are
23:// already in scope; importing them again would shadow that public re-export.
24:
25:/// 这个示例的宿主身份。graft 要求覆盖双方共享同一个 framework。
26:/// The example's host identity. A graft requires both sides to share it.
27:pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.example.control-button");
28:
29:// 宿主入口的 graft 计划，用**类型化**写法：两侧都是指向真实注册面的 Rust 路径，
30:// 因此编译器与编辑器都能解析它们——写在 `cut(` 之后会补全宿主注册面路径，
31:// 写在 `graft(` 之后会补全外部 crate 路径。代价是外部实现必须被静态链接进来。
32:// The host's graft plan in the **typed** form: both side
…[结果：此处截断，全文 4118 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**该步推理**（逐字）：
```
Now the 4 hardbug classes. Each tree has its own BRIEF.md with the five fields: `root cause`, `mechanism`, `evidence`, `counter-proof`, `fix`. Read-only (do not edit the tree!). Answer file `answers/<id>.md` with those labels.

Let me investigate each. Start with all four trees' sources. They're small-ish. Let me read the host sources and graft sources.

h1-supply-chain: symptom: rendered layout total is off; the offsets test fails; every widget file reads correct alone. Class: one sibling calls a different family of names than its siblings do.

Let me read the host's widget files and the offsets test + control.rs + lib.rs.

Let me start with h1-supply-chain and h2-claim-unkept (both have host/ + graft/).

Let me look at h1-supply-chain first.
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h1-supply-chain：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain/host && grep -n '' tests/offsets.rs | head -60; echo '=== lib.rs'; grep -n '' src/lib.rs | head -60", "description": "Read h1-supply-chain offsets test"}`

**调用** `bash`（step 145）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain/host && for f in src/control/object/*/*.rs; do echo \"===== $f\"; grep -n \"fn offset\\|offset(\" -A 6 $f | head -20; done", "description": "Read all widget offset functions"}
```

**结果**：
```
===== src/control/object/badge/badge.rs
15:pub fn offset(x: i32) -> i32 {
16-    to_local(x)
17-}
18-
19-crate::control_object! {
20-    kind: Badge,
21-    exports: ["control.render"],
===== src/control/object/button/button.rs
15:pub fn offset(x: i32) -> i32 {
16-    to_local(x)
17-}
18-
19-crate::control_object! {
20-    kind: Button,
21-    exports: ["control.render"],
===== src/control/object/dial/dial.rs
15:pub fn offset(x: i32) -> i32 {
16-    to_local(x)
17-}
18-
19-crate::control_object! {
20-    kind: Dial,
21-    exports: ["control.render"],
===== src/control/object/gauge/gauge.rs
15:pub fn offset(x: i32) -> i32 {
16-    to_local(x)
17-}
18-
19-crate::control_object! {
20-    kind: Gauge,
21-    exports: ["control.render"],
===== src/control/object/panel/panel.rs
15:pub fn offset(x: i32) -> i32 {
16-    to_local(x)
17-}
18-
19-crate::control_object! {
20-    kind: Panel,
21-    exports: ["control.render"],
===== src/control/object/slider/slider.rs
15:pub fn offset(x: i32) -> i32 {
16-    to_local(x)
17-}
18-
19-crate::control_object! {
20-    kind: Slider,
21-    exports: ["control.render"],
===== src/control/object/spinner/spinner.rs
15:pub fn offset(x: i32) -> i32 {
16-    to_local(x)
17-}
18-
19-crate::control_object! {
20-    kind: Spinner,
21-    exports: ["control.render"],
===== src/control/object/timeline/timeline.rs
15:pub fn offset(x: i32) -> i32 {
16-    to_local(x)
17-}
18-
19-crate::control_object! {
20-    kind: Timeline,
21-    exports: ["control.render"],
===== src/control/object/toggle/toggle.rs
15:pub fn offset(x: i32) -> i32 {
16-    to_world(x)
17-}
18-
19-crate::control_object! {
20-    kind: Toggle,
21-    exports: ["control.render"],

```

**该步推理**（逐字）：
```
The offsets test sums 8 widgets' `offset(10)` and expects 136.

Let me look at the 8 widget files' `offset` implementations. The class: "one sibling calls a different family of names than its siblings do" — e.g. one widget's offset uses a different helper (e.g. `frame_offset` vs `family_offset`), giving a different value.

Let me grep the offset functions across the host's object files.
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h1-supply-chain：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain/host && for f in src/control/object/*/*.rs; do echo \"===== $f\"; grep -n \"fn offset\\|offset(\" -A 6 $f | head -20; done", "description": "Read all widget offset functions"}`

**调用** `bash`（step 146）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain/host && grep -n '' src/control/object/toggle/toggle.rs; echo '======== button'; sed -n '1,18p' src/control/object/button/button.rs; echo '======== to_local/to_world defs'; grep -rn \"fn to_local\\|fn to_world\" src/ ; echo '======== control.rs'; grep -n '' src/control/control.rs | head -60", "description": "Read toggle and coordinate helpers"}
```

**结果**：
```
1://! The toggle face.
2:
3:use crate::control::{ControlFrame, ControlHandle, to_world};
4:
5:pub struct Toggle;
6:
7:impl ControlHandle for Toggle {
8:    /// Paint this widget onto the frame it was handed.
9:    fn paint(&self) -> ControlFrame {
10:        ControlFrame
11:    }
12:}
13:
14:/// This widget's offset, in the coordinate family its siblings use.
15:pub fn offset(x: i32) -> i32 {
16:    to_world(x)
17:}
18:
19:crate::control_object! {
20:    kind: Toggle,
21:    exports: ["control.render"],
22:    parent: crate::control::NODE_ID,
23:    handle_contracts: [crate::control::ControlHandle],
24:}
======== button
//! The button face.

use crate::control::{ControlFrame, ControlHandle, to_local};

pub struct Button;

impl ControlHandle for Button {
    /// Paint this widget onto the frame it was handed.
    fn paint(&self) -> ControlFrame {
        ControlFrame
    }
}

/// This widget's offset, in the coordinate family its siblings use.
pub fn offset(x: i32) -> i32 {
    to_local(x)
}

======== to_local/to_world defs
src/control/control.rs:5:pub fn to_local(x: i32) -> i32 { x + 7 }
src/control/control.rs:8:pub fn to_world(x: i32) -> i32 { x + 31 }
======== control.rs
1://! The control folder face, the two coordinate families its children may use, and the
2://! handle every direct child implements.
3:
4:/// The local family: offsets are measured from the widget's own origin.
5:pub fn to_local(x: i32) -> i32 { x + 7 }
6:
7:/// The world family: offsets are measured from the frame's origin.
8:pub fn to_world(x: i32) -> i32 { x + 31 }
9:
10:/// The frame a parent face hands to its children for painting.
11:pub struct ControlFrame;
12:
13:/// The interface every direct child must implement.
14:pub trait ControlHandle {
15:    fn paint(&self) -> ControlFrame;
16:}
17:
18:pub struct Control;
19:
20:crate::root_object! {
21:    kind: Control,
22:    needs_registry: true,
23:    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
24:}

```

**该步推理**（逐字）：
```
Found it: `toggle.rs:16` calls `to_world(x)` while all 7 siblings call `to_local(x)`. That's the "different family of names" — exactly the h1-supply-chain class.

Now: where do `to_local`/`to_world` come from? Maybe `use crate::control::{to_local, to_world}` or a `coords` module. And what do they do (differently)? Let me look at the imports in toggle.rs vs a sibling, and the definitions of to_local/to_world.

Let me read toggle.rs fully and button.rs fully, plus the definition of to_local/to_world (in src/control/control.rs or a coords module).
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h1-supply-chain：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain/host && grep -n '' src/control/object/toggle/toggle.rs; echo '======== button'; sed -n '1,18p' src/control/object/button/button.rs; echo '======== to_local/to_world defs'; grep -rn \"fn to_local\\|fn to_world\" src/ ; 
…[参数：此处截断，全文 427 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

**调用** `bash`（step 147）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain/host && CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/round9/.cargo-target/hardbugs timeout 1200 cargo test --offline 2>&1 | tail -25; echo \"EXIT=${PIPESTATUS[0]}\"", "description": "h1-supply-chain: run offsets test", "timeoutMs": 1300000}
```

**结果**：
```

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/offsets.rs (/home/nich/Moirai_N3/nichlink/target/round9/.cargo-target/hardbugs/debug/deps/offsets-6a46587cf677cf9d)

running 1 test
test the_rendered_offsets_add_up ... FAILED

failures:

---- the_rendered_offsets_add_up stdout ----

thread 'the_rendered_offsets_add_up' (511387) panicked at tests/offsets.rs:18:5:
assertion `left == right` failed: the rendered offsets add up to 160, not 136
  left: 160
 right: 136
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    the_rendered_offsets_add_up

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `--test offsets`
EXIT=101

```

**该步推理**（逐字）：
```
Now the arithmetic: 8 widgets at x=10.
- 7 with `to_local(10) = 17`.
- toggle with `to_world(10) = 41`.
Total = 7*17 + 41 = 119 + 41 = 160. Hmm, EXPECTED_TOTAL = 136. Let me recompute: if all 8 used to_local: 8*17 = 136 ✓. So the expected total is exactly the all-local value. Actual = 160 ≠ 136.

Hmm, but the test currently fails — let me run it to capture the raw evidence (the BRIEF requires evidence with commands and exit codes).

Wait, careful: 'the rendered layout is off. `cargo test --offline` fails in the offsets test with a total that no single widget's own code explains, and every widget file reads as correct on its own.' — right: toggle's file reads fine on its own ("in the coordinate family its siblings use" — its doc says local family, but it calls to_world). The defect: `toggle.rs:3` imports `to_world` and `:16` calls it, whereas the doc at :14 and all siblings use `to_local`.

Root cause file:line = `src/control/object/toggle/toggle.rs:16` (the call) — or the import at :3. The BRIEF asks for "the file and line you believe the defect is at". I'll give toggle.rs:16 (the call) as the root cause, noting :3 as the import that makes it compile.

Fix: `to_world(x)` → `to_local(x)` and the import at :3 → `to_local`. Do not touch tests. (Deliverable says "the smallest edit, and which files you did not touch"; the tree is read-only so I must NOT edit it — "The tree is read-only: do not edit it, and do not change a test." So the `fix` field = the smallest edit described, not applied. Careful! For the hardbug classes, the fixes must NOT be applied. Let me re-read: "The tree is **read-only**: do not edit it, and do not change a test. Every claim needs the command you ran, its raw output and its exit code". So for these 4 classes I only diagnose; no edits. Good — that also means the gate can't be green (it's red by design) — that's consistent with the class being a diagnosis task.

Note: this differs from the r*/f* families. OK.

Let me now run the offsets test to capture raw output + exit code, and also use the bridge for evidence (callgraph, consistency, locate).

Then h2, h3, h4.

Let me run h1-supply-chain's test. The host depends on xirang-toolchain via path (the sed pointed it to the checkout) and on the graft crate. Let me run `cargo test --offline` in host/. It may need to compile a lot (the toolchain) — the .cargo-target dir has no h1-supply-chain entry... it has fa fb fd fe g3 h1 r1..r4. So a cold build is needed for this host; that could take minutes. Let me try with a shared target dir per hardbug class: /target/round9/.cargo-target/h1-supply-chain.

Actually — to save time, maybe reuse the fa target dir? Different package/deps (graft). Cargo can share a target dir across projects (it keys by fingerprint), so using .cargo-target/fa for h1-supply-chain would reuse the toolchain build artifacts. That's a legit speedup: `CARGO_TARGET_DIR=.../.cargo-target/fa`. Hmm, but it could cause lock contention with other runs; fine.

Let me use a per-class dir but pre-warm by... no, simplest: use the shared `.cargo-target/fa` dir for the hardbug classes (it already has xirang-toolchain built). Actually different packages → cargo still needs to build the host + graft crates, but the toolchain dependency build is cached. Let me use `.cargo-target/hardbug` as a new shared dir; the first run will rebuild the toolchain (slow, maybe 1-3 min). Reusing fa's dir avoids that. Let me reuse `.cargo-target/fa`.

Hmm, concurrent cargo runs sharing a target dir: cargo takes a lock, so parallel runs serialize. I'll run the 4 hardbug tests sequentially (one command, sequential `cargo test` invocations per class... but that's "several problems in one shell command" → needs a batching.md entry). Hmm! The batching rule applies to "把几道题放进同一条 shell 命令" — for attribution of per-problem tokens. If I run 4 classes' tests in one bash call, I must record it in batching.md. That's fine — I'll record it (it's honest: the 4 hardbug classes shared one shell command for compiling/running their tests). But then "逐题 token 无法归因"... the batching.md line covers it.

Cleaner: keep one problem per bash call, and run them in parallel in the same message (separate shells). Cargo lock contention on the shared target dir would serialize them anyway. Let me give each class its own target dir to avoid contention: `.cargo-target/h1-supply-chain`, etc. First build of the toolchain per dir is expensive (4×). Hmm.

Alternative: the tool's `check` can run the tests; but I still need cargo.

Let me check how big the toolchain build is: the fa check took 23s (incremental, already built). A cold build of xirang-toolchain + deps... The main workspace target/debug exists; maybe I can point CARGO_TARGET_DIR at /home/nich/Moirai_N3/nichlink/target (the checkout's own target dir) — it already has all the toolchain artifacts! `CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target` for the hardbug trees. Since the toolchain path dep is the checkout itself, the fingerprints should mostly match (different profile features?). The tree's Cargo.toml has features run/authoring — the checkout's target dir has those built (from previous rounds/debug binary). Risk: writing into the main target dir (allowed — it's the workspace). It might also invalidate the checkout's build (different feature unification) — that could slow later builds of the MCP tool (but I'm done building it).

Hmm, safer: use `.cargo-target/hardbug` (new) and accept one cold build (sequential, first class). Then the other three reuse it. To avoid the batching issue, I'll run them one class per bash call, sequentially (each in its own message-ish). But parallel calls in one message would contend for the lock → they'd serialize automatically; each is a separate shell command → no batching entry needed. That's the cleanest: 4 separate bash calls (even if they queue on the cargo lock). Let me do that: 4 parallel bash calls, each with its own tree and CARGO_TARGET_DIR=.cargo-target/<clas
…[推理：此处截断，全文 7294 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h1-supply-chain：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain/host && CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/round9/.cargo-target/hardbugs timeout 1200 cargo test --offline 2>&1 | tail -25; echo \"EXIT=${PIPESTATUS[0]}\"", "description": "h1-supply-chain: run offse
…[参数：此处截断，全文 331 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`
- `bash` → 归属 h2-claim-unkept：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host && CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/round9/.cargo-target/hardbugs timeout 1200 cargo test --offline 2>&1 | tail -20; echo \"EXIT=${PIPESTATUS[0]}\"; echo '=== ledger'; grep -n '' .xirang/ado
…[参数：此处截断，全文 397 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

**调用** `bash`（step 153）：
```
{"command": "rm -rf /tmp/h1sc-probe && cp -r /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain /tmp/h1sc-probe && rm -rf /tmp/h1sc-probe/host/target && cd /tmp/h1sc-probe/host && sed -i 's/to_world/to_local/g' src/control/object/toggle/toggle.rs && grep -n \"to_local\\|to_world\" src/control/object/toggle/toggle.rs && CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/round9/.cargo-target/hardbugs timeout 900 cargo test --offline 2>&1 | tail -8; echo \"EXIT=${PIPESTATUS[0]}\"", "description": "h1-supply-chain fix probe on a /tmp copy", "timeoutMs": 900000}
```

**结果**：
```
3:use crate::control::{ControlFrame, ControlHandle, to_local};
16:    to_local(x)
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests control_button

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

EXIT=0

```

**该步推理**（逐字）：
```
h3 is fully diagnosed: the entry plan at `src/lib.rs:48-66` declares 8 cuts (button, slider, timeline, gauge, toggle, spinner, panel, badge) and **dial is absent**; the insertion point is between `:64-65` (badge's cut) and `:66` (`);`). The graft crate already provides `dial_fast` (graft/src/lib.rs:17,32).

Now, the mechanism: the build-time scope narrows to the subtrees the cuts name (scope.rs:249-318); a face no cut names is not selected → the generated tree doesn't mount/ship it → `outline()` lacks `root/control/dial` → tests/shipped.rs:19 fails.

Hmm, one nuance: in h3's host, is dial mounted? The dial face's module is not mounted (like fa), yet `registry` (derived tree) shows it — because the derived tree is a source-scan view, not the shipped tree. And `why` at dial.rs:15 worked (file-level). Also `check`'s census said 20 of 22 production functions unreachable — because the test only names widgets by string. Fine.

Wait — but did h3's host compile dial.rs? If dial isn't mounted, `tests/shipped.rs` never references it as a Rust path (only strings). And `why` read the file. So dial.rs is unmounted → the "widget that exists in the sources does not appear in the shipped registry tree" — exactly the class. ✓

Now, for h1-supply-chain, I need to check the impact claim: expected 136 = 8×17; actual 160 = 7×17 + 41. Also which file: `toggle.rs:16` calls `to_world` (import at :3; definition at control.rs:8). Fix: import & call `to_local` instead (toggle.rs:3 and :16), or 1 line if the import is shared: two edits in the same file (import + call). Files not touched: tests/offsets.rs, all 8 sibling widget files, control.rs (the two families are legitimate; both exist for a reason — hmm, is `to_world` legitimate? It's documented as "The world family: offsets are measured from the frame's origin" — the two coordinate families exist by design; so the defect is that toggle, whose doc says "in the coordinate family its siblings use", uses the world family).

Careful: maybe the intended fix is to change the *import* only? No: `to_world(x)` at :16 is the call. Minimal edit: `to_local` in both places (2 tokens in 1 file). I'll state: `src/control/object/toggle/toggle.rs:3` (`to_world` → `to_local`) and `:16` (`to_world(x)` → `to_local(x)`); tests and other files untouched.

Now let me get a couple more evidence items for h1-supply-chain: read control.rs:5/8 (I have it from the earlier grep: to_local = x+7 at :5, to_world = x+31 at :8) ✓, plus a counter-proof idea: "sum without toggle = 7*17 = 119; toggle alone with to_world gives 41; if the test's expected total were computed as all-local it's 136 = 8*17; changing toggle to to_local makes the sum 136." I can verify arithmetic by computing: 17*8 = 136 ✓, 160-136 = 24 = (41-17) ✓ (31-7 = 24).

Should I actually run a probe to prove the fix? The tree is read-only; but I could compute with a tiny standalone rust/python eval — no need: arithmetic is verifiable. Better counter-proof: run the offsets test with only toggle changed? I can't edit the tree (read-only). Hmm — the "counter-proof" field: "one thing you did that would have shown your diagnosis wrong". Options: (a) check that all 8 widgets are called by the test (so none is silently unused) — done by reading tests/offsets.rs:10-17 (all 8 named) ✓; (b) confirm the arithmetic difference is exactly the toggle delta: 160 - 136 = 24, and to_world(10) - to_local(10) = 41 - 17 = 24 ✓ — this shows one single wrong call explains the whole gap (if two widgets were wrong, the delta would be a sum of deltas); (c) confirm no sibling calls to_world (consistency: "calls to_world, which no sibling calls") ✓.

Actually, I *can* construct a stronger probe without editing the read-only tree: copy the tree to /tmp and change toggle's call there, then run the test. That doesn't edit the question tree. That's a legitimate counter-proof ("若改 toggle 为 to_local 后总数为 136，则诊断成立"). Let me do that: copy h1-supply-chain to /tmp/h1sc-probe (excluding target), sed the toggle file, run cargo test there with the shared target dir. The path deps are absolute (they point to the tree paths?) — hmm: the host's Cargo.toml has path deps to `../graft` (relative) and to the toolchain (absolute /home/nich/.../toolchain). If I copy the whole h1-supply-chain dir, `../graft` resolves within the copy ✓ and the toolchain path is absolute ✓. Let me try. This gives raw evidence for the fix without touching the tree.

Let me do that probe, and similarly for h4? For h4 the counter-proof: the census's "ZeroArm::Refuse never constructed" + doc mismatch; a probe: copy the tree, flip `zero_arm` to `Refuse`, and check that the branch-level census row disappears (and that a zero-entry-with-receipt now returns false). Hmm, but for h4 the current tests are green, so after the fix the tests would... still green (the tests never cover zero). And the census would flip to naming `Post` as never-constructed. That's a nice probe: it shows the "uncovered branch" moves, i.e. confirms the diagnosis.

For h2: counter-proof could be: check that the panel file's real digest differs from the ledger's and that button's matches — do that by running `conformance --anchor root/control/panel` (lapsed) and `--anchor root/control/button` (unchanged) — done ✓. Plus verify the ledger's two fingerprints are identical strings (grep) — they are (`edc72845...` twice) which proves "a copy of another entry's bytes" ✓.

For h3: counter-proof: show that a face the plan DOES name ships while dial doesn't — e.g. `badge` is in the plan and ships (the failing test's tree list includes badge) ✓; and the graft crate already has `dial_fast` (so the plan, not the graft, is the missing declaration). Also `why` says "no declared cut names root/control/dial". Good.

Let me run the h1-supply-chain probe (copy + fix + test) and the h4 probe (copy + fix + census) — wait, the census is a bridge call on the tree; for the probe copy I can run `check` via the bridge on the copy? The copy is inside /tmp, and
…[推理：此处截断，全文 10068 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h1-supply-chain：`{"command": "cd /home/nich/Moirai_N3/nichlink; B=./target/debug/xirang-mcp; R=target/round9/trees/h1-supply-chain/host; L=target/round9/logs/h1-supply-chain.jsonl\necho '===== check default'; $B --call check --root $R --log $L --face default --timeout_ms 900000 2>&1 | sed -n '1,16p'; echo \"EXIT=$
…[参数：此处截断，全文 603 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`
- `bash` → 归属 h1-supply-chain：`{"command": "rm -rf /tmp/h1sc-probe && cp -r /home/nich/Moirai_N3/nichlink/target/round9/trees/h1-supply-chain /tmp/h1sc-probe && rm -rf /tmp/h1sc-probe/host/target && cd /tmp/h1sc-probe/host && sed -i 's/to_world/to_local/g' src/control/object/toggle/toggle.rs && grep -n \"to_local\\|to_world\" src
…[参数：此处截断，全文 581 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

**调用** `write`（step 156）：
```
{"content": "# h1-supply-chain — 同族里一个兄弟调用了与其余兄弟不同的那一族名字\n\nroot cause — `src/control/object/toggle/toggle.rs:16`（调用）＋ `:3`（import）：`to_world(x)`。\n同族其余 8 个（badge/button/dial/gauge/panel/slider/spinner/timeline）在同一位置都是 `to_local(x)`；\n两个族定义在 `src/control/control.rs:5`（`to_local = x + 7`，局部族）与 `:8`（`to_world = x + 31`，世界族）。\n\nmechanism — `tests/offsets.rs:9-17` 把 8 个 `offset(10)` 相加。兄弟各给 17，toggle 给 41 ⇒\n7×17 + 41 = **160**，而\"每个部件都按兄弟那族摆放\"的总和是 8×17 = **136**（差值 24 恰为 31−7）。\ntoggle 自己的注释 `:14`（\"in the coordinate family its siblings use\"）与它的调用相反，所以单看每个文件都\"对\"。\n整棵树里没有任何第二个 `to_world` 调用者（`consistency --parent root/control` 的 9 个成员只点 toggle）。\n\nevidence — ① `cargo test --offline`（root=h1-supply-chain/host，exit **101**）：\n`the_rendered_offsets_add_up ... FAILED`，`assertion left == right failed: the rendered offsets add up to 160, not 136`。\n② `check {face:default}` → `verdict failed (cargo exit 101)`，同上（log `…/host/target/xirang/out/check-default.log`）。\n③ `consistency {parent:root/control}` → `badge/button/dial/gauge/panel/slider/spinner/timeline 1 call(s): to_local`、\n`toggle 1 call(s): to_world`、`outlier toggle: does not call to_local … calls to_world, which no sibling calls`、`outliers: 1 of 9`。\n④ `why {at:toggle.rs:15}` → 契约行 `:14` 仍是\"兄弟那族\"；callers=`tests/offsets.rs::the_rendered_offsets_add_up`。\n⑤ `callgraph {function:offset}` → 每个兄弟 `callees: to_local -> src/control/control.rs:5`。\n⑥ 算术：8×17=136、7×17+41=160、41−17=24。\n\ncounter-proof — 若错的是别处（例如 `to_local` 本身或另一个部件），则只改 toggle 一处**不会**让总和变 136。\n我在 **/tmp 的副本**（不碰本树）里把 `toggle.rs` 的 `to_world` 全改成 `to_local` 后重跑：\n`the_rendered_offsets_add_up ... ok`、`test result: ok. 1 passed; 0 failed`（exit **0**）。\n另一侧：若两个部件都错，160−136=24 就该是两个差值之和；实测差值恰等于单次 `to_world−to_local`。\n\nfix — 最小改动：`src/control/object/toggle/toggle.rs:3` 与 `:16` 的 `to_world` → `to_local`（同一文件、两处标识符）。\n未改动：`tests/offsets.rs`（不改测试）、`src/control/control.rs`（两个坐标族都是设计的一部分）、\n其余 8 个部件文件、`src/lib.rs` 的 graft 计划（toggle 是槽位，但缺陷在宿主自己的文件里）。\n本树**只
…[参数：此处截断，全文 2266 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**结果**：
```
<path>/home/nich/Moirai_N3/nichlink/target/round9/answers/h1-supply-chain.md</path>
<type>file</type>
<content>
Created file
</content>
```

**该步推理**（逐字）：
```
Confirmed: spinner.rs's block (19-23) lacks the `handle_contracts:` line that every sibling has at line 23. So root cause: `src/control/object/spinner/spinner.rs:23` (where the block closes without the declaration).

Now write the four hardbug answers. Each with the five labeled fields. Keep them detailed but ≤30 lines each (the BRIEF's 30-line limit applies to `answers/<id>.md`; the hardbug briefs ask for the five fields — I'll keep within ~30 lines).

Let me write them.
```

**同一步并行发出的其它调用**：
- `write` → 归属 h1-supply-chain：`{"content": "# h1-supply-chain — 同族里一个兄弟调用了与其余兄弟不同的那一族名字\n\nroot cause — `src/control/object/toggle/toggle.rs:16`（调用）＋ `:3`（import）：`to_world(x)`。\n同族其余 8 个（badge/button/dial/gauge/panel/slider/spinner/timeline）在同一位置都是 `to_local(x)`；\n两个族定义在 `src/control/control.rs:5`（`to_local = x + 7`，局部族）与 `:8`（`
…[参数：此处截断，全文 2266 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`
- `write` → 归属 h2-claim-unkept：`{"content": "# h2-claim-unkept — 真标本台账 + 一个同族只缺一条声明 + 一条指纹失效的条目\n\nroot cause — 两处，各自独立：\n① `src/control/object/spinner/spinner.rs:23`：它的 `crate::control_object!` 块（`:19-23`）在 `parent:` 之后**直接闭合**，\n   少了兄弟都写在**同一行号**的那一条 `handle_contracts: [crate::control::ControlHandle],`（button/panel/badge 皆为 `:2
…[参数：此处截断，全文 2940 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`


## 三、交付答案（逐字）

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
