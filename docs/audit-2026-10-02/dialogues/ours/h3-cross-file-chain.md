# ours · `h3-cross-file-chain`

**题面**：hardbug h3：跨文件因果链——入口计划从没点名新面，构建期作用域把它排除。

> 步 4 · 仪器调用 **3**（逐题日志条数）· 会话里定位到 1 条 · 输出 6,521 tok · 推理 13,740 字符 · 累计上下文 1,193,088

## 一、工具给了什么 · 当时 agent 在想什么（逐条配对）

> ⚠️ 日志 3 条 vs 会话定位 1 条：差的那些调用发生在**共享步**（一条命令里跑了多道题）或**装置步**里 —— 这正是"逐题 token 不可归因"的具体形状。

### ①1 日志第 1 条 · exit 0 · step 150（turn 1）

**命令**：`--call check --root target/round9/trees/h3-cross-file-chain/host --face default --timeout_ms 900000`

**工具返回**（逐字）：
```
verdict  failed (cargo exit 101)
check  cargo test
face   default
root   /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain/host
tree   14 rust file(s), 24 function(s)
elapsed 23211 ms
log    /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain/host/target/xirang/out/check-default.log
exit   101
result test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
result test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
failed every_declared_widget_ships
why    every_declared_widget_ships: thread 'every_declared_widget_ships' (513347) panicked at tests/shipped.rs:19:9: `dial` is declared and does not ship; the tree is:
next   the `why` lines above are the failing assertion's own words: a short, stable phrase from one is a string literal in this tree, so `search {literal: "`dial` is declared and does not"}` finds the line that produced it. If two red things may be independent, two green runs are not the evidence: fix one and re-run, and say which red survived. And a probe you built yourself that disagrees with the source is a reason to re-read that line (`read`, `search {literal}`) before rebuilding — a second look is cheaper than a second build
census: 0 named numeric constant(s); a static fact about this tree, not a verdict
  declarations: 12 production `pub fn` name(s) appear in no test file (a text-level count: a test that reaches one without writing its name does not count here)
  test-reachable: 20 of 22 production function(s) no test can reach (24 function(s) indexed in this tree; a static walk from the test files along the same name-in-call-list rule the orphan view uses)
  branch-level: 0 constructively unreachable arm(s) in this tree (0 `false` guard(s), 0 never-constructed variant(s); a static read of the source text, not a coverage measurement)
  entry plan: 11 `cut(` site(s) and 9 `graft(` site(s) across this tree's sources (a static count; which of them this application ships is the plan's own business)
  decl   no test names `to_local` (src/control/control.rs:5)
  decl   no test names `to_world` (src/control/control.rs:8)
  decl   no test names `offset` (src/control/object/badge/badge.rs:15)
  not covered: the reachability column (dynamic dispatch, function pointers, FFI, macro expansion, trait methods and closures are invisible; matching is by name; `main` is never listed) · the branch column (no data-dependent condition is judged, `false` is the only guard literal decided, a `pub` enum is never judged, and a construction this tree does not spell would falsify a row) · this census (string constants, structural duplication, runtime behaviour and claims written in prose are outside it). A static read of the source text, not a coverage measurement; `census: true` and `--list check` have the full text
… truncated: 9 of 18 census rows withheld at the limit of 5; pass `census: true` for the whole table (every column head is already here with its count)
evidence xirang.check {"face":"default","timeout_ms":900000} → exit 0

```

**该步推理**（逐字）：
```
Both classes' evidence is excellent:

**h1-supply-chain**: `consistency --parent root/control` names the outlier: `toggle: does not call to_local ... calls to_world, which no sibling calls` (1 of 9). Plus `why` shows the contract line (toggle.rs:14) and callers; the cut wiring (lib.rs:48) shows toggle is a graft slot (relevant: the sibling that gets replaced by a graft; but the defect is in the host's own file). Need: `read` of control.rs:5/8 to give the two families' arithmetic and confirm 160 vs 136.

**h2-claim-unkept**: 
- `adopted`: provisional 1 / lapsed 1: the panel entry lapsed — "changed since the confirmation; the recorded print is `root/control/button`'s current print, not this file's — a copy of another entry's bytes". Interesting! The ledger's panel entry has the SAME fingerprint as button's entry (both `edc72845...`), i.e. the panel line's fingerprint is a copy of button's — that's the second symptom: "one ledger entry no longer describes the bytes it names".
- `consistency --specimen root/control/button`: `outlier spinner: lacks handle_traits` (1 of 8) — that's "one sibling missing exactly one declaration".
- `conformance --anchor root/control/panel`: lapsed, bytes changed.

So h2's three facts: (a) reference shape (button) declares exports + handle_traits; (b) spinner lacks handle_traits (exactly one declaration missing); (c) the panel ledger entry's fingerprint is button's, not panel's → lapsed ("a copy of another entry's bytes").

Root cause (file:line): the panel entry's fingerprint line — `.xirang/adopted/entries` line 2 (the fingerprint field is a copy of line 1's). Hmm, but "root cause" for the missing declaration: `src/control/object/spinner/spinner.rs` — let me find the exact line where the declaration is missing (i.e. the `handle_contracts`/`handle_traits` in the macro block). Let me read spinner.rs and button.rs + panel.rs to give precise lines.

Let me now also think about the "mechanism" and "fix" for h2:
- root cause: two places. The BRIEF wants one file+line for "the defect". Hmm, h2's symptom has two halves (the class says: "a real specimen ledger entry, one sibling missing exactly one declaration, one lapsed entry"). I'll give: (1) `src/control/object/spinner/spinner.rs:<line>` missing `handle_traits: ["ControlHandle"]` (the one declaration the reference shape carries); (2) `.xirang/adopted/entries:2` — the panel line's fingerprint is button's (`edc72845…`), so it never described panel.rs; panel.rs's real bytes have a different fingerprint → lapsed. Actually careful: "one ledger entry no longer describes the bytes it names" — the tool says "the recorded print is `root/control/button`'s current print, not this file's — a copy of another entry's bytes". So the entry's fingerprint field = button's fingerprint. Let me verify by computing sha256 of both files — but the ledger fingerprint may not be plain sha256. In fc, the ledger's fingerprint (22b54a4c) ≠ the file's sha256 (e842be74), yet `adopted` said "unchanged since the confirmation". So the ledger's fingerprint is some other digest (maybe blake3 or sha256 of normalized content). Hmm! In h2, both entries have the same fingerprint string (edc72845...), and the tool says the panel line's recorded print is button's current print. So the tool computed button's print = edc72845... and panel's print = something else. Fine: the tool's own comparison is authoritative; I don't need to reproduce the hash.

Let me now get exact lines: read spinner.rs macro block (which fields it declares) and button.rs's block; and panel.rs (it was "rewritten" — maybe it renamed its exports or changed shape). Let me read all three quickly.

Then h3 and h4 evidence.

Let me batch: read h2's spinner/button/panel via bash grep, and run h3/h4 bridge calls in parallel.
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h2-claim-unkept：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host && for f in button spinner panel; do echo \"===== $f\"; sed -n '1,40p' src/control/object/$f/$f.rs; done", "description": "Read h2 sibling faces"}`

### ①2 日志第 2 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`--call registry --root target/round9/trees/h3-cross-file-chain/host`

**工具返回**（逐字）：
```
namespace xirang-example-control-button
tree derived now (no published records at /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain/host/target/xirang/out; cannot read /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain/host/target/xirang/out/source_scope.tsv: No such file or directory (os error 2))
faces 10
root/control                             Control        control/control.rs                     fb97ddd5f2b803d1b7f40a776d8a22d9
root/control/badge                       Badge          control/object/badge/badge.rs          f930da08aeaaffc0d35a3a6fd11751ab
root/control/button                      Button         control/object/button/button.rs        ff1c57d9b110ef57eac134a3cb4e46b6
root/control/dial                        Dial           control/object/dial/dial.rs            e1070833a032e106f9cdd0eff2149bc2
root/control/gauge                       Gauge          control/object/gauge/gauge.rs          215e1e1515ed44626fb768be00da91c9
root/control/panel                       Panel          control/object/panel/panel.rs          149511628746b7e87255f3d1a84891d2
root/control/slider                      Slider         control/object/slider/slider.rs        bdb4427ce81c9bc51e56bee7667fd2be
root/control/spinner                     Spinner        control/object/spinner/spinner.rs      5f6b87c8e55e73b461b53f862759e174
root/control/timeline                    Timeline       control/object/timeline/timeline.rs    3c26ff3d25859d5981245bd179c9bb43
root/control/toggle                      Toggle         control/object/toggle/toggle.rs        4dbbd3f9fbf51458e863ff8e25bdbef8
next   `explain {node}` for one face's contract, `check {face}` for whether it builds
evidence xirang.registry {} → exit 0

```

### ①3 日志第 3 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`--call why --root target/round9/trees/h3-cross-file-chain/host --at src/control/object/dial/dial.rs:15`

**工具返回**（逐字）：
```
at src/control/object/dial/dial.rs:15 — the definition `offset` (lines 15-17)
  contract   (from the doc above it, which is the promise this tree writes)
             src/control/object/dial/dial.rs:14 /// This widget's offset, in the coordinate family its siblings use.
  siblings   8 file(s) under src/control/object define `offset` too: src/control/object/badge/badge.rs, src/control/object/button/button.rs, src/control/object/gauge/gauge.rs, src/control/object/panel/panel.rs, src/control/object/slider/slider.rs, src/control/object/spinner/spinner.rs, src/control/object/timeline/timeline.rs, src/control/object/toggle/toggle.rs — a change here is a change to the family (`consistency --parent` compares their declared shapes)
  callers    0 in this root
  pins       no test names `offset` — nothing pins this, so a change here is unverified until something does
  adoption   no ledger at .xirang/adopted/entries in this root
  gate       no `#[cfg]` attribute sits directly above this definition at src/control/object/dial/dial.rs:15 (a gate on the enclosing module is not read here)
  scope      scope unknown (no source_scope.tsv; run `xirang check`)
  pruning    pruning unknown (no pruning_manifest.tsv; run `xirang check`)
  wiring     no declared cut in /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain/host/src/lib.rs names `root/control/dial` — the entry declares 8 cut(s), and a face no cut names is one the build replaces with nothing (`grafts` lists them)
not covered here: how a grafted subtree looks at runtime (ask `trace`), and whether the declarations above describe the tree **as it is now** — the scope and pruning lines are the build's own output, and a tree built before the last edit says so on the `scope` line
next   `check {face}` to run the face that compiles it, `grafts` for every declared cut, `locate {symptom}` for sibling places to compare it with
evidence xirang.why {"at":"src/control/object/dial/dial.rs:15"} → exit 0

```


## 二、该题相关的其它调用（读文件 / 编辑 / 跑测试 / 写答案），带当时的推理

**调用** `bash`（step 148）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain/host && CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/round9/.cargo-target/hardbugs timeout 1200 cargo test --offline 2>&1 | tail -22; echo \"EXIT=${PIPESTATUS[0]}\"", "description": "h3-cross-file-chain: run shipped test", "timeoutMs": 1300000}
```

**结果**：
```
---- every_declared_widget_ships stdout ----

thread 'every_declared_widget_ships' (511721) panicked at tests/shipped.rs:19:9:
`dial` is declared and does not ship; the tree is:
root/control  kind=Control  source=control/control.rs
root/control/badge  kind=Badge  source=control/object/badge/badge.rs
root/control/button  kind=Button  source=control/object/button/button.rs
root/control/gauge  kind=Gauge  source=control/object/gauge/gauge.rs
root/control/panel  kind=Panel  source=control/object/panel/panel.rs
root/control/slider  kind=Slider  source=control/object/slider/slider.rs
root/control/spinner  kind=Spinner  source=control/object/spinner/spinner.rs
root/control/timeline  kind=Timeline  source=control/object/timeline/timeline.rs
root/control/toggle  kind=Toggle  source=control/object/toggle/toggle.rs
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    every_declared_widget_ships

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `--test shipped`
EXIT=101

```

**该步推理**（逐字）：
```
h1-supply-chain: FAILED with total 160 vs expected 136 — matches my computation ✓.

h2-claim-unkept: tests pass (`offsets` ok). The ledger has 2 entries: button (the reference sibling shape) and panel ("the panel's original contract", "recorded before the panel was rewritten"). Symptom: "A constraint this design adopted is not holding on a later sibling, and one ledger entry no longer describes the bytes it names."

Class: "a real specimen ledger entry, one sibling missing exactly one declaration, one lapsed entry".
So: (1) one sibling (probably panel) is missing exactly one declaration that the reference shape (button) carries; (2) one ledger entry's fingerprint lapsed (its covered file's bytes moved).

Let me look at h2's button vs panel vs other siblings, and check the ledger entries via `adopted`/`conformance`.

Let me read the h2 host files: button.rs, panel.rs, control.rs and the ledger's fingerprint math. Let me use the bridge for authority.

Now let me also run h3 and h4 tests, then do the bridge diagnoses for all four.
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h3-cross-file-chain：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain/host && CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/round9/.cargo-target/hardbugs timeout 1200 cargo test --offline 2>&1 | tail -22; echo \"EXIT=${PIPESTATUS[0]}\"", "description": "h3-cross-file-chain: r
…[参数：此处截断，全文 339 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`
- `bash` → 归属 h4-one-file-many-algorithms：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h4-one-file-many-algorithms && CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/round9/.cargo-target/hardbugs timeout 1200 cargo test --offline 2>&1 | tail -18; echo \"EXIT=${PIPESTATUS[0]}\"; echo '=== files'; grep -n '' tests/p
…[参数：此处截断，全文 414 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

**调用** `bash`（step 151）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain && echo '=== tests/shipped.rs'; grep -n '' host/tests/shipped.rs; echo '=== graft lib.rs'; grep -n '' graft/src/lib.rs | head -40; echo '=== graft dial?'; ls graft/src/", "description": "Read h3 shipped test and graft crate"}
```

**结果**：
```
=== tests/shipped.rs
1://! What this host actually ships: the tree the build scoped.
2://! 这个宿主真正发布的东西：构建划定作用域后的那棵树。
3:
4:#[test]
5:fn every_declared_widget_ships() {
6:    let rows = control_button::outline();
7:    let shipped = rows.join("\n");
8:    for name in [
9:        "button",
10:        "slider",
11:        "timeline",
12:        "gauge",
13:        "toggle",
14:        "spinner",
15:        "panel",
16:        "badge",
17:        "dial",
18:    ] {
19:        assert!(
20:            shipped.contains(&format!("/{name}  kind=")),
21:            "`{name}` is declared and does not ship; the tree is:\n{shipped}"
22:        );
23:    }
24:}
=== graft lib.rs
1://! An out-of-project crate: it takes no part in the host's generated tree.
2://! 项目外 crate：它不参与宿主那棵生成树。
3:
4:use xirang_toolchain::runtime::registry_core::{FrameworkId, Registry};
5:
6:/// Must match the host framework, or the overlay rejects this tree.
7:pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.example.control-button");
8:
9:pub mod button_fast;
10:pub mod slider_fast;
11:pub mod timeline_fast;
12:pub mod gauge_fast;
13:pub mod toggle_fast;
14:pub mod spinner_fast;
15:pub mod panel_fast;
16:pub mod badge_fast;
17:pub mod dial_fast;
18:
19:/// The external implementation's own registry.
20:pub fn external_registry() -> Registry {
21:    let mut registry = Registry::root_for_namespace(FRAMEWORK, env!("CARGO_PKG_NAME"));
22:    registry
23:        .register_all(&[
24:            button_fast::REGISTRATION,
25:            slider_fast::REGISTRATION,
26:            timeline_fast::REGISTRATION,
27:            gauge_fast::REGISTRATION,
28:            toggle_fast::REGISTRATION,
29:            spinner_fast::REGISTRATION,
30:            panel_fast::REGISTRATION,
31:            badge_fast::REGISTRATION,
32:            dial_fast::REGISTRATION,
33:        ])
34:        .expect("external face registers");
35:    registry
36:}
=== graft dial?
badge_fast.rs
button_fast.rs
dial_fast.rs
gauge_fast.rs
lib.rs
panel_fast.rs
slider_fast.rs
spinner_fast.rs
timeline_fast.rs
toggle_fast.rs

```

**该步推理**（逐字）：
```
Excellent. Key h3 evidence from `why {at: dial.rs:15}`:
- `siblings 8 file(s) ... define offset too`
- `callers 0 in this root`
- `pins no test names offset`
- `wiring no declared cut in .../src/lib.rs names root/control/dial — the entry declares 8 cut(s), and a face no cut names is one the build replaces with nothing`

And `registry` shows 10 faces (dial IS in the derived tree) but the *shipped* tree (base_registry via the test) lacks dial. The test `every_declared_widget_ships` in tests/shipped.rs:19 names dial.

Hmm wait — interesting: in h3 the derived tree has dial but the shipped tree doesn't (the test says "dial is declared and does not ship"). So in h3, the dial face exists in the sources and is *derived*, but the build-time scope (8 cuts in lib.rs) doesn't name it → not shipped.

Hmm, but in fa the same thing happened (dial not mounted). In h3, is dial mounted (compiled)? The `why` shows the file has definitions and it's in the derived tree. Let me check the test to see the exact assertion and what "declared" means (maybe there's a DECLARED list in the test or a `widgets()` function). Let me read tests/shipped.rs.

Then the root cause: the entry plan (`src/lib.rs`'s `static_graft_plan!`) never names `root/control/dial` → the build-time scope leaves dial out of the build. But hmm: the class says "the entry plan never names the new face, so the build-time scope leaves it out of the build". The fix: declare it (cut+graft) at the entry — requires a `dial_fast` in the graft crate... let me check if the graft crate has dial_fast.

Let me read tests/shipped.rs and the graft crate's lib.rs + lib.rs's cut list.

Also for h4, let me run the digest + census.

Let me do these two in parallel.
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h3-cross-file-chain：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain && echo '=== tests/shipped.rs'; grep -n '' host/tests/shipped.rs; echo '=== graft lib.rs'; grep -n '' graft/src/lib.rs | head -40; echo '=== graft dial?'; ls graft/src/", "description": "Read h3 shipped test and gr
…[参数：此处截断，全文 311 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`
- `bash` → 归属 h4-one-file-many-algorithms：`{"command": "cd /home/nich/Moirai_N3/nichlink; B=./target/debug/xirang-mcp; R=target/round9/trees/h4-one-file-many-algorithms; L=target/round9/logs/h4-one-file-many-algorithms.jsonl\necho '===== check census'; $B --call check --root $R --log $L --face default --census true --timeout_ms 300000 2>&1
…[参数：此处截断，全文 508 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

**调用** `bash`（step 152）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain/host && sed -n '44,70p' src/lib.rs | cat -n | sed 's/^/ /' ; echo '--- absolute lines'; grep -n \"cut(\\|graft(\\|static_graft_plan\\|);\" src/lib.rs | sed -n '1,40p'; echo '--- dial face'; grep -n '' src/control/object/dial/dial.rs; echo '--- registry output tail (search digests.tsv)'; ls", "description": "Read h3 entry plan lines"}
```

**结果**：
```
      1	//
      2	// 字符串写法仍然完全可用，只是工具无法补全它，也不需要链接外部实现：
      3	//   cut "root/control/button" graft "button_fast"
      4	// The string form still works and needs no link, but tooling cannot complete it.
      5	xirang_toolchain::runtime::static_graft_plan!(
      6	    FRAMEWORK,
      7	    cut(crate::control::object::button::NODE_ID)
      8	        graft(control_button_graft::button_fast::NODE_ID),
      9	    cut(crate::control::object::slider::NODE_ID)
     10	        graft(control_button_graft::slider_fast::NODE_ID),
     11	    cut(crate::control::object::timeline::NODE_ID)
     12	        graft(control_button_graft::timeline_fast::NODE_ID),
     13	    cut(crate::control::object::gauge::NODE_ID)
     14	        graft(control_button_graft::gauge_fast::NODE_ID),
     15	    cut(crate::control::object::toggle::NODE_ID)
     16	        graft(control_button_graft::toggle_fast::NODE_ID),
     17	    cut(crate::control::object::spinner::NODE_ID)
     18	        graft(control_button_graft::spinner_fast::NODE_ID),
     19	    cut(crate::control::object::panel::NODE_ID)
     20	        graft(control_button_graft::panel_fast::NODE_ID),
     21	    cut(crate::control::object::badge::NODE_ID)
     22	        graft(control_button_graft::badge_fast::NODE_ID),
     23	);
     24	
     25	/// 按框架和包命名空间装配这个示例的注册机。
     26	/// Assemble the example's registry from its framework and package namespace.
     27	pub fn base_registry() -> Registry {
--- absolute lines
11:xirang_toolchain::runtime::host!();
27:pub const FRAMEWORK: FrameworkId = FrameworkId::new("xirang.example.control-button");
30:// 因此编译器与编辑器都能解析它们——写在 `cut(` 之后会补全宿主注册面路径，
31:// 写在 `graft(` 之后会补全外部 crate 路径。代价是外部实现必须被静态链接进来。
36:// 这里声明的每个 `cut(` 都是宿主交出去的槽位，而构建期作用域收窄到这些切口命名的
39:// Every `cut(` declared here is a slot the host hands over, and the build-time
48:xirang_toolchain::runtime::static_graft_plan!(
50:    cut(crate::control::object::button::NODE_ID)
51:        graft(control_button_graft::button_fast::NODE_ID),
52:    cut(crate::control::object::slider::NODE_ID)
53:        graft(control_button_graft::slider_fast::NODE_ID),
54:    cut(crate::control::object::timeline::NODE_ID)
55:        graft(control_button_graft::timeline_fast::NODE_ID),
56:    cut(crate::control::object::gauge::NODE_ID)
57:        graft(control_button_graft::gauge_fast::NODE_ID),
58:    cut(crate::control::object::toggle::NODE_ID)
59:        graft(control_button_graft::toggle_fast::NODE_ID),
60:    cut(crate::control::obje
…[结果：此处截断，全文 3756 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**该步推理**（逐字）：
```
**h4 evidence complete**:
- tests green (`2 passed`), census: `branch-level: 1 constructively unreachable arm(s)` with `no construction of ZeroArm::Refuse is spelled in this tree, so the arm matching it in postable at src/model/entry.rs:44 can never be entered`, and the contract above it quoted (lines 34-37: "a zero entry never is").
- digest names the 3 algorithms (new/zero_arm/postable/signed/normalized_account).
So: root cause = `src/model/entry.rs:31` (`ZeroArm::Post` returned where the contract requires Refuse) — the inverted branch; uncovered branch = `ZeroArm::Refuse => false` at `:44`, which the census proves is never constructed.

Wait, careful: which is "the branch that does the opposite of documentation"? The doc at :34-37 says postable requires a receipt and zero never posts. `zero_arm()` returns `ZeroArm::Post` — whose own doc (:6) says "A zero entry is posted like any other" — that IS the opposite of the contract. The `postable` match maps Post→true, Refuse→false, which is faithful; so the defect is `zero_arm`'s return value at :31. The fix: `ZeroArm::Post` → `ZeroArm::Refuse` at :31 (1 word). And the uncovered branch is `ZeroArm::Refuse => false` at :44 (which the fix would then make reachable — careful: the class says "exactly one branch inverted and exactly one branch uncovered"; after the fix, the Refuse arm becomes reachable and the Post arm becomes uncovered. Both facts hold pre-fix: one inverted (the return), one uncovered (Refuse arm)).

Hmm, is `zero_arm` itself "one of the three algorithms"? The file's doc at :51-52 says "the contract is why it is one of the three algorithms this file carries" (about `signed`). So the three algorithms = postable/zero_arm, signed, normalized_account. Fine.

**h3 evidence**:
- tests/shipped.rs:8-23 lists 9 widget names including dial and asserts `outline()` (the shipped tree) contains each.
- The shipped tree has 8 of 9 (dial missing); the derived `registry` has 10 faces (dial present).
- `why` at dial.rs:15 says: `wiring no declared cut in .../host/src/lib.rs names root/control/dial — the entry declares 8 cut(s), and a face no cut names is one the build replaces with nothing`.
- The graft crate HAS `dial_fast` (graft/src/dial_fast.rs + lib.rs:17,32) — so the missing piece is exactly the entry's plan: `static_graft_plan!` in host/src/lib.rs declares 8 cuts (badge? let me check the full list) and no `dial`.

So h3's root cause = `host/src/lib.rs` (the `static_graft_plan!` block) — the line where dial's cut should be (i.e., the plan block, around line 48-60+). Let me see the full cut list to name a precise line: the plan starts at :48; the cuts are button(:50-51), slider(:52-53), timeline, gauge, toggle, spinner, panel(?)... 8 cuts. Let me print the file's plan block to give exact lines and to identify the insertion point. And check whether `badge` is in the plan (the shipped tree has badge... yes it does: the tree shows badge in the derived list; the shipped list from the failing test output: control, badge, button, gauge, panel, slider, spinner, timeline, toggle = 9 rows = root + 8 widgets = badge, button, gauge, panel, slider, spinner, timeline, toggle. So all 8 except dial are shipped, and the plan declares 8 cuts for those 8. So dial has no cut → not shipped.

Root cause file:line: `host/src/lib.rs:48-…` — I need the line where the dial cut is missing (the last cut's line, i.e., where it should be appended). Since "the entry plan never names the new face", I'll cite the plan block: `host/src/lib.rs:48-62` (the `static_graft_plan!` invocation) and specifically the last declared cut's line as the insertion point. Let me get the exact text.

Let me also verify that h3's host has `src/control/object/dial/dial.rs` and that adding the cut (with `control_button_graft::dial_fast::NODE_ID`) would fix it — as the fix (not applied, read-only).

Let me print h3's lib.rs plan block + the h3 dial face + the host's src listing.
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h3-cross-file-chain：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain/host && sed -n '44,70p' src/lib.rs | cat -n | sed 's/^/ /' ; echo '--- absolute lines'; grep -n \"cut(\\|graft(\\|static_graft_plan\\|);\" src/lib.rs | sed -n '1,40p'; echo '--- dial face'; grep -n '' src/control/o
…[参数：此处截断，全文 421 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

**调用** `bash`（step 157）：
```
{"command": "rm -rf /tmp/h3probe && cp -r /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain /tmp/h3probe && rm -rf /tmp/h3probe/host/target /tmp/h3probe/graft/target\ncd /tmp/h3probe/host && python3 - <<'EOF'\np='src/lib.rs'\ns=open(p).read()\nold=\"\"\"    cut(crate::control::object::badge::NODE_ID)\n        graft(control_button_graft::badge_fast::NODE_ID),\n);\"\"\"\nnew=\"\"\"    cut(crate::control::object::badge::NODE_ID)\n        graft(control_button_graft::badge_fast::NODE_ID),\n    cut(crate::control::object::dial::NODE_ID)\n        graft(control_button_graft::dial_fast::NODE_ID),\n);\"\"\"\nassert old in s\nopen(p,'w').write(s.replace(old,new,1))\nprint('probe: cut(dial) added')\nEOF\nCARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/round9/.cargo-target/hardbugs timeout 900 cargo test --offline 2>&1 | tail -12; echo \"EXIT=${PIPESTATUS[0]}\"", "description": "h3 probe: add dial cut in /tmp copy and test", "timeoutMs": 900000}
```

**结果**：
```
probe: cut(dial) added

running 1 test
test every_declared_widget_ships ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests control_button

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

EXIT=0

```

**该步推理**（逐字）：
```
Wait — in h1-supply-chain's answer I listed 4 calls in the log: consistency, why, callgraph, check = 4 ✓ (matches: h1-supply-chain.jsonl had 3 + check = 4).

Now h3 and h4 answers.

h3: root cause = `host/src/lib.rs:48-66` (the `static_graft_plan!`): 8 cuts declared (button :50, slider :52, timeline :54, gauge :56, toggle :58, spinner :60, panel :62, badge :64) and **no cut for dial**; insertion point :65-66 (after badge's graft line, before `);`).
mechanism: build-time scope narrows to the subtrees the cuts name (scope.rs:249-318); a face no cut names is not selected → not mounted/shipped → `outline()` (lib.rs:69-) lacks `root/control/dial` → tests/shipped.rs:19 fails for name "dial".
evidence:
- `cargo test --offline` exit 101: `every_declared_widget_ships` panicked at tests/shipped.rs:19: "`dial` is declared and does not ship; the tree is: root/control … (9 rows, no dial)".
- `check {face:default}` → failed (exit 101), same test.
- `registry` → derived tree has **10 faces including** `root/control/dial` (so the source exists and is discovered; the gap is the shipped tree).
- `why {at: src/control/object/dial/dial.rs:15}` → `wiring no declared cut in …/host/src/lib.rs names root/control/dial — the entry declares 8 cut(s), and a face no cut names is one the build replaces with nothing`.
- The graft side already exists: `graft/src/dial_fast.rs` + `graft/src/lib.rs:17` (`pub mod dial_fast;`) and `:32` (`dial_fast::REGISTRATION` in `external_registry`).
- lib.rs:36-43 (the host's own doc) states the rule: "Every cut declared here is a slot the host hands over, and the build-time scope narrows to the subtrees these cuts name: a face nobody declared is not shipped by this application."
counter-proof: if the defect were on the graft side (a missing external implementation), `graft/src/lib.rs:17/32` wouldn't already list `dial_fast`; it does. If the defect were a wrong module path/attribute in dial.rs, `registry` wouldn't list it with a proper module (`control::object::dial` per `explain`) — it does. If the plan did name dial, the shipped rows would contain `/dial  kind=` — they don't (the panic prints all 9 rows).
Hmm, a stronger counter-proof would be an actual probe: copy the tree to /tmp, add the cut for dial, run the test → green. Let me do that! It's cheap and gives a constructed counter-proof. The graft crate is at ../graft (relative) → copying the whole h3-cross-file-chain dir preserves it. Let me do that probe.

h4: root cause = `src/model/entry.rs:31` (`zero_arm` returns `ZeroArm::Post`); the doc that it violates = :36-37 ("a zero entry never is"); the uncovered branch = `:44` (`ZeroArm::Refuse => false`), census: "no construction of `ZeroArm::Refuse` is spelled in this tree, so the arm matching it in `postable` at src/model/entry.rs:44 can never be entered".
mechanism: `postable` (:38-46) first refuses without a receipt, then delegates the zero decision to `zero_arm()` (:30-32); that function unconditionally returns `Post` (:31) whose own doc (:6) says "A zero entry is posted like any other" — the opposite of the contract → a receipted zero entry is postable. The Refuse arm (:44) can never be entered because nothing constructs `Refuse`; and the tests (tests/postable.rs) only use amount 5, so no test covers the zero path (that's why green).
evidence: `cargo test --offline` exit 0 (2 passed); `check {census}` → `branch-level: 1 constructively unreachable arm(s) … no construction of ZeroArm::Refuse … at src/model/entry.rs:44` + the contract lines quoted; `digest {file}` → 5 functions with callers; `read`/source of entry.rs lines 3-46.
counter-proof: if the zero rule were implemented as documented, the branch column wouldn't name `Refuse` as never constructed (it names exactly it); if the gap were only "untested" rather than "unreachable by construction", the census's criterion (private enum + no construction spelled) wouldn't apply — `enum ZeroArm` is private (:5) and the only construction is :31.
fix: `src/model/entry.rs:31` `ZeroArm::Post` → `ZeroArm::Refuse` (1 word). Note: this makes the Refuse arm reachable and leaves the Post arm constructed nowhere (the class's "one inverted branch" moves) — and the tests stay green (they don't cover zero), which is exactly why the defect survived. Don't touch tests. Files not touched: tests/postable.rs, src/model/model.rs?, src/lib.rs.
Let me check what other files h4 has: src/lib.rs, src/model/entry.rs, src/model/model.rs, tests/postable.rs. The doc mentions "three algorithms in one file" = entry.rs. OK.

Let me run the h3 probe first, then write h3/h4 answers.
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h3-cross-file-chain：`{"command": "rm -rf /tmp/h3probe && cp -r /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain /tmp/h3probe && rm -rf /tmp/h3probe/host/target /tmp/h3probe/graft/target\ncd /tmp/h3probe/host && python3 - <<'EOF'\np='src/lib.rs'\ns=open(p).read()\nold=\"\"\"    cut(crate::control::ob
…[参数：此处截断，全文 972 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

**调用** `write`（step 160）：
```
{"content": "# h3-cross-file-chain — 入口计划从没点名那个新面 ⇒ 构建期作用域把它排除在构建之外\n\nroot cause — `src/lib.rs:48-66` 的 `static_graft_plan!`：它声明了 **8 个 cut**（button `:50`、slider `:52`、\ntimeline `:54`、gauge `:56`、toggle `:58`、spinner `:60`、panel `:62`、badge `:64`），**没有 dial 的那一条**；\n缺的正是 `:65`（badge 的 graft 行）与 `:66`（`);`）之间应当追加的\n`cut(crate::control::object::dial::NODE_ID) graft(control_button_graft::dial_fast::NODE_ID),`。\n\nmechanism — 构建期作用域从入口推导：`cut` 是\"强制存活根\"，作用域保留的**恰好是切口命名的子树**\n（`toolchain/src/build_time/src/scope.rs:249-318`，宿主自己的文档也这么写：`src/lib.rs:36-43`\n\"the build-time scope narrows to the subtrees these cuts name: a face nobody declared is not shipped by this application\"）。\ndial 的源码在（`src/control/object/dial/dial.rs`，`registry` 也把它当**派生树**里的第 10 个面列出），\n但没有任何切口点名它 ⇒ 生成的发布树不挂载它 ⇒ `outline()`（`src/lib.rs:69-95`）里没有 `root/control/dial`\n⇒ `tests/shipped.rs:8-23` 按名字逐个断言时，`dial` 落空。\n\nevidence — ① `cargo test --offline`（root=h3-cross-file-chain/host，exit **101**）：`every_declared_widget_ships ... FAILED`，\npanic 在 `tests/shipped.rs:19`：`` `dial` is declared and does not ship; the tree is: `` 后列出 9 行\n（control + badge/button/gauge/panel/slider/spinner/timeline/toggle，**无 dial**）。\n② `check {face:default}` → `verdict failed (cargo exit 101)`，同一条（log `…/host/target/xirang/out/check-default.log`）。\n③ `registry` → `faces 10`，其中含 `root/control/dial Dial control/object/dial/dial.rs e1070833…` ⇒ 源码侧没问题，\n   差的是**发布**那一步。\n④ `why {at:src/control/object/dial/dial.rs:15}` → `wiring no declared cut in …/host/src/lib.rs names root/control/dial —\n   the entry declares 8 cut(s), and a face no cut names is one the build replaces with nothing`；\n   同一回复还说 `siblings 8 file(s) … define offset too`、`callers 0 in this root`、`pins no test names offset`。\n⑤ 外部实现**已经在了**：`graft/src/dial_fast.rs` 存在，`graft/src/lib.rs:17` 声明 `pub mod dial_fast;`，\n   `:32` 把 `dial_fast::REGISTRATION` 放进 `external_registry()` ⇒ 缺口只在宿主入口的计划里。\n\ncounter-proof — 若缺口在\"外 graft 没有 dial_fast\"，那 `graft/src/lib.
…[参数：此处截断，全文 2716 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**结果**：
```
<path>/home/nich/Moirai_N3/nichlink/target/round9/answers/h3-cross-file-chain.md</path>
<type>file</type>
<content>
Created file
</content>
```

**该步推理**（逐字）：
```
Now the probe is clean:
- A (as shipped): the "zero entry never posts" contract test FAILS.
- B (only `zero_arm`'s return flipped to `Refuse`): the same test PASSES.

And the in-tree suite is green in both cases (the uncovered branch is why).

Now let me write h3.md and h4.md.

Then: batching.md, verify tallies, final gate re-runs (all trees I changed must be green — r1-r4, fd, fe, h1, fa, fb, fc, g3: I've verified each after the change; good), update_task, and the report to the captain.

Let me write h3 and h4 answers now.
```

**同一步并行发出的其它调用**：
- `write` → 归属 h3-cross-file-chain：`{"content": "# h3-cross-file-chain — 入口计划从没点名那个新面 ⇒ 构建期作用域把它排除在构建之外\n\nroot cause — `src/lib.rs:48-66` 的 `static_graft_plan!`：它声明了 **8 个 cut**（button `:50`、slider `:52`、\ntimeline `:54`、gauge `:56`、toggle `:58`、spinner `:60`、panel `:62`、badge `:64`），**没有 dial 的那一条**；\n缺的正是 `:65`（badge 的 graft 行）与 `:
…[参数：此处截断，全文 2716 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`
- `write` → 归属 h4-one-file-many-algorithms：`{"content": "# h4-one-file-many-algorithms — 三套算法一个文件，恰一支写反、恰一支没被覆盖\n\nroot cause — `src/model/entry.rs:31`：`fn zero_arm(&self)`（定义 `:30-32`）**无条件返回 `ZeroArm::Post`**。\n它自己的枚举文档写着两个含义相反的臂（`:6` \"A zero entry is posted like any other.\" / `:9` \"A zero entry is refused.\"），\n而 `postable` 的契约（`:36-37`
…[参数：此处截断，全文 2362 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

**调用** `read`（step 165）：
```
{"file_path": "/home/nich/Moirai_N3/nichlink/target/round9/answers/h3-cross-file-chain.md"}
```

**结果**：
```
<path>/home/nich/Moirai_N3/nichlink/target/round9/answers/h3-cross-file-chain.md</path>
<type>file</type>
<content>
1: # h3-cross-file-chain — 入口计划从没点名那个新面 ⇒ 构建期作用域把它排除在构建之外
2: root cause — `src/lib.rs:48-66` 的 `static_graft_plan!`：它声明了 **8 个 cut**（button `:50`、slider `:52`、
3: timeline `:54`、gauge `:56`、toggle `:58`、spinner `:60`、panel `:62`、badge `:64`），**没有 dial 的那一条**；
4: 缺的正是 `:65`（badge 的 graft 行）与 `:66`（`);`）之间应当追加的
5: `cut(crate::control::object::dial::NODE_ID) graft(control_button_graft::dial_fast::NODE_ID),`。
6: mechanism — 构建期作用域从入口推导：`cut` 是"强制存活根"，作用域保留的**恰好是切口命名的子树**
7: （`toolchain/src/build_time/src/scope.rs:249-318`，宿主自己的文档也这么写：`src/lib.rs:36-43`
8: "the build-time scope narrows to the subtrees these cuts name: a face nobody declared is not shipped by this application"）。
9: dial 的源码在（`src/control/object/dial/dial.rs`，`registry` 也把它当**派生树**里的第 10 个面列出），
10: 但没有任何切口点名它 ⇒ 生成的发布树不挂载它 ⇒ `outline()`（`src/lib.rs:69-95`）里没有 `root/control/dial`
11: ⇒ `tests/shipped.rs:8-23` 按名字逐个断言时，`dial` 落空。
12: evidence — ① `cargo test --offline`（root=h3-cross-file-chain/host，exit **101**）：`every_declared_widget_ships ... FAILED`，
13: panic 在 `tests/shipped.rs:19`：`` `dial` is declared and does not ship; the tree is: `` 后列出 9 行
14: （control + badge/button/gauge/panel/slider/spinner/timeline/toggle，**无 dial**）。
15: ② `check {face:default}` → `verdict failed (cargo exit 101)`，同一条（log `…/host/target/xirang/out/check-default.log`）。
16: ③ `registry` → `faces 10`，其中含 `root/control/dial Dial control/object/dial/dial.rs e1070833…` ⇒ 源码侧没问题，
17:    差的是**发布**那一步。
18: ④ `why {at:src/control/object/dial/dial.rs:15}` → `wiring no declared cut in …/host/src/lib.rs names root/control/dial —
19:    the entry declares 8 cut(s), and a face no cut names is one the build replaces with nothing`；
20:    同一回复还说 `siblings 8 file(s) … define offset too`、`callers 0 in this root`、`pins no test names offset`。
21: ⑤ 外部实现**已经在了**：`graft/src/dial_fast.rs` 存在，`graft/src/lib.rs:17` 声明 `pub mod dial_fast;`，
22:    `:32` 把 `dial_fast::REGISTRATION` 放进 `external_registry()` ⇒ 缺口只在宿主入口的计划里。
23: counter-proof — 若缺口在"外 graft 没有 dial_fast"，那 `graft/src/lib.rs:17/32` 不会已经写它（会）；若缺口在 dial 面的形状/路径，
24: `registry` 与 `explain` 不会把它列成一个正常的第 10 面（会）。构造证据：把整棵树复制到 **/tmp**（不碰本树），
25: 只在 `src/lib.rs` 的 `static_graft_plan!` 里补上 dial 的那两行，重跑 → `every_declared_widget_ships ... ok`、
26: `test result: ok. 1 passed; 0 failed`（exit **0**）。
27: fix — 最小改动：在 `src/lib.rs:65` 与 `:66` 之间补 2 行（`cut(...dial::NODE_ID)` + `gra
…[结果：此处截断，全文 2834 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**同一步并行发出的其它调用**：
- `read` → 归属 h3-cross-file-chain：`{"file_path": "/home/nich/Moirai_N3/nichlink/target/round9/answers/h3-cross-file-chain.md"}`

**调用** `write`（step 166）：
```
{"content": "# h3-cross-file-chain — 入口计划从没点名那个新面 ⇒ 构建期作用域把它排除在构建之外\nroot cause — `src/lib.rs:48-66` 的 `static_graft_plan!`：它声明了 **8 个 cut**（button `:50`、slider `:52`、timeline `:54`、gauge `:56`、toggle `:58`、spinner `:60`、panel `:62`、badge `:64`），**没有 dial 的那一条**；缺的正是 `:65`（badge 的 graft 行）与 `:66`（`);`）之间应当追加的 `cut(crate::control::object::dial::NODE_ID) graft(control_button_graft::dial_fast::NODE_ID),`。\nmechanism — 构建期作用域从入口推导：`cut` 是\"强制存活根\"，作用域保留的**恰好是切口命名的子树**（`toolchain/src/build_time/src/scope.rs:249-318`；宿主自己的文档也这么写，`src/lib.rs:36-43`：\"the build-time scope narrows to the subtrees these cuts name: a face nobody declared is not shipped by this application\"）。dial 的源码在（`src/control/object/dial/dial.rs`，`registry` 也把它当**派生树**里的第 10 个面列出），但没有任何切口点名它 ⇒ 生成的发布树不挂载它 ⇒ `outline()`（`src/lib.rs:69-95`）里没有 `root/control/dial` ⇒ `tests/shipped.rs:8-23` 按名字逐个断言时 `dial` 落空。\nevidence — ① `cargo test --offline`（root=h3-cross-file-chain/host，exit **101**）：`every_declared_widget_ships ... FAILED`，panic 在 `tests/shipped.rs:19`：`` `dial` is declared and does not ship; the tree is: `` 后列出 9 行（control + badge/button/gauge/panel/slider/spinner/timeline/toggle，**无 dial**）。\n② `check {face:default}` → `verdict failed (cargo exit 101)`，同一条（log `…/host/target/xirang/out/check-default.log`）。\n③ `registry` → `faces 10`，其中含 `root/control/dial Dial control/object/dial/dial.rs e1070833…` ⇒ 源码侧没问题，差的是**发布**那一步。\n④ `why {at:src/control/object/dial/dial.rs:15}` → `wiring no declared cut in …/host/src/lib.rs names root/control/dial — the entry declares 8 cut(s), and a face no cut names is one the build replaces with nothing`；同一回复还说 `siblings 8 file(s) … define offset too`、`callers 0 in this root`、`pins no test names offset`。\n⑤ 外部实现**已经在了**：`graft/src/dial_fast.rs` 存在，`graft/src/lib.rs:17` 声明 `pub mod dial_fast;`，`:32` 把 `dial_fast::REGISTRATION` 放进 `external_registry()` ⇒ 缺口只在宿主入口的计划里。\ncounter-proof — 若缺口在\"外 graft 没有 dial_fast\"，`graft/src/lib.rs:17/32` 不会已经写它（会）；若缺口在 dial 面的形状/路径，`registr
…[参数：此处截断，全文 2654 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**结果**：
```
<path>/home/nich/Moirai_N3/nichlink/target/round9/answers/h3-cross-file-chain.md</path>
<type>file</type>
<content>
Updated file
</content>
```

**该步推理**（逐字）：
```
31 lines — one over. Merge lines 22-23 and 25-26 to save 2 lines. Let me rewrite the whole file compactly (aim 28 lines).
```

**同一步并行发出的其它调用**：
- `write` → 归属 h3-cross-file-chain：`{"content": "# h3-cross-file-chain — 入口计划从没点名那个新面 ⇒ 构建期作用域把它排除在构建之外\nroot cause — `src/lib.rs:48-66` 的 `static_graft_plan!`：它声明了 **8 个 cut**（button `:50`、slider `:52`、timeline `:54`、gauge `:56`、toggle `:58`、spinner `:60`、panel `:62`、badge `:64`），**没有 dial 的那一条**；缺的正是 `:65`（badge 的 graft 行）与 `:66`（`)
…[参数：此处截断，全文 2654 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`
- `bash` → 归属 装置：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9 && echo '=== answer line counts (must be <=30)'; for f in answers/*.md; do n=$(wc -l < $f); if [ \"$n\" -gt 30 ]; then echo \"OVER  $f $n\"; fi; done; echo '(no OVER lines above = all <=30)'; echo; python3 - <<'EOF'\nimport json,glob,re,os\
…[参数：此处截断，全文 1040 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`


## 三、交付答案（逐字）

# h3-cross-file-chain — 入口计划从没点名那个新面 ⇒ 构建期作用域把它排除在构建之外
root cause — `src/lib.rs:48-66` 的 `static_graft_plan!`：它声明了 **8 个 cut**（button `:50`、slider `:52`、timeline `:54`、gauge `:56`、toggle `:58`、spinner `:60`、panel `:62`、badge `:64`），**没有 dial 的那一条**；缺的正是 `:65`（badge 的 graft 行）与 `:66`（`);`）之间应当追加的 `cut(crate::control::object::dial::NODE_ID) graft(control_button_graft::dial_fast::NODE_ID),`。
mechanism — 构建期作用域从入口推导：`cut` 是"强制存活根"，作用域保留的**恰好是切口命名的子树**（`toolchain/src/build_time/src/scope.rs:249-318`；宿主自己的文档也这么写，`src/lib.rs:36-43`："the build-time scope narrows to the subtrees these cuts name: a face nobody declared is not shipped by this application"）。dial 的源码在（`src/control/object/dial/dial.rs`，`registry` 也把它当**派生树**里的第 10 个面列出），但没有任何切口点名它 ⇒ 生成的发布树不挂载它 ⇒ `outline()`（`src/lib.rs:69-95`）里没有 `root/control/dial` ⇒ `tests/shipped.rs:8-23` 按名字逐个断言时 `dial` 落空。
evidence — ① `cargo test --offline`（root=h3-cross-file-chain/host，exit **101**）：`every_declared_widget_ships ... FAILED`，panic 在 `tests/shipped.rs:19`：`` `dial` is declared and does not ship; the tree is: `` 后列出 9 行（control + badge/button/gauge/panel/slider/spinner/timeline/toggle，**无 dial**）。
② `check {face:default}` → `verdict failed (cargo exit 101)`，同一条（log `…/host/target/xirang/out/check-default.log`）。
③ `registry` → `faces 10`，其中含 `root/control/dial Dial control/object/dial/dial.rs e1070833…` ⇒ 源码侧没问题，差的是**发布**那一步。
④ `why {at:src/control/object/dial/dial.rs:15}` → `wiring no declared cut in …/host/src/lib.rs names root/control/dial — the entry declares 8 cut(s), and a face no cut names is one the build replaces with nothing`；同一回复还说 `siblings 8 file(s) … define offset too`、`callers 0 in this root`、`pins no test names offset`。
⑤ 外部实现**已经在了**：`graft/src/dial_fast.rs` 存在，`graft/src/lib.rs:17` 声明 `pub mod dial_fast;`，`:32` 把 `dial_fast::REGISTRATION` 放进 `external_registry()` ⇒ 缺口只在宿主入口的计划里。
counter-proof — 若缺口在"外 graft 没有 dial_fast"，`graft/src/lib.rs:17/32` 不会已经写它（会）；若缺口在 dial 面的形状/路径，`registry`/`explain` 不会把它列成一个正常的第 10 面（会）。构造证据：把整棵树复制到 **/tmp**（不碰本树），只在 `src/lib.rs` 的 `static_graft_plan!` 里补上 dial 的那两行，重跑 → `every_declared_widget_ships ... ok`、`test result: ok. 1 passed; 0 failed`（exit **0**）。
fix — 最小改动：在 `src/lib.rs:65` 与 `:66` 之间补 2 行（`cut(...dial::NODE_ID)` + `graft(...dial_fast::NODE_ID)`）。未改动：`tests/shipped.rs`（不改测试）、`src/control/object/dial/dial.rs`（面本身是好的）、`graft/**`（外部实现已存在）、其余 8 个切口。本树**只读**，以上均未落盘；探针只在 /tmp 副本上做。
调用清单（`target/round9/logs/h3-cross-file-chain.jsonl`）：check(default) · registry · why(dial.rs:15)。
总调用：成功 3 / 被拒 0
