# ours · `h3-cross-file-chain`（精简版；完整版见 `../ours/h3-cross-file-chain.md`）

**题面**：hardbug h3：跨文件因果链——入口计划从没点名新面，构建期作用域把它排除。

> 仪器调用 **3** · 步 4 · 输出 6,521 tok · 推理 13,740 字符 · 累计上下文 1,193,088

## 仪器调用序列（工具给了什么 + 当时 agent 在想什么）

### ①1 `--call check --root target/round9/trees/h3-cross-file-chain/host --face default --timeout_ms 900000`  · exit 0 · step 150（turn 1）

**工具返回**：
```
verdict  failed (cargo exit 101)
check  cargo test
face   default
root   /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain/host
tree   14 rust file(s), 24 function(s)
elapsed 23211 ms
log    /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain/host/target/nichlink/out/check-default.log
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
evidence nichlink.check {"face":"default","timeout_ms":900000} → exit 0

```

**当时推理**：
```
Both classes' evidence is excellent:

**h1-supply-chain**: `consistency --parent root/control` names the outlier: `toggle: does not call to_local ... calls to_world, which no sibling calls` (1 of 9). Plus `why` shows the contract line (toggle.rs:14) and callers; the cut wiring (lib.rs:48) shows toggle is a graft slot (relevant: the sibling that gets replaced by a graft; but the defect is in the host's own file). Need: `read` of control.rs:5/8 to give the two families' arithmetic and confirm 160 vs 136.

**h2-claim-unkept**: 
- `adopted`: provisional 1 / lapsed 1: the panel entry lapsed — "changed since the confirmation; the recorded print is `root/control/button`'s current print, not this file's — a copy of another entry's bytes". Interesting! The ledger's panel entry has the SAME fingerprint as button's entry (both `edc72845...`), i.e. the panel line's fingerprint is a copy of button's — that's the second symptom: "one ledger entry no longer describes the bytes it names".
- `consistency --specimen root/control/button`: `outlier spinner: lacks handle_traits` (1 of 8) — that's "one sibling missing exactly one declaration".
- `conformance --anchor root/control/panel`: lapsed, bytes changed.

So h2's three facts: (a) reference shape (button) declares exports + handle_traits; (b) spinner lacks handle_traits (exactly one declaration missing); (c) the panel ledger entry's fingerprint is button's, not panel's → lapsed ("a copy of another entry's bytes").

Root cause (file:line): the
…[推理：此处截断，全文 3791 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**同一步还发了**：`bash`→h2-claim-unkept

### ①2 `--call registry --root target/round9/trees/h3-cross-file-chain/host`  · exit 0 · 共享/装置步

**工具返回**：
```
namespace nichlink-example-control-button
tree derived now (no published records at /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain/host/target/nichlink/out; cannot read /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain/host/target/nichlink/out/source_scope.tsv: No such file or directory (os error 2))
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
evidence nichlink.registry {} → exit 0

```

### ①3 `--call why --root target/round9/trees/h3-cross-file-chain/host --at src/control/object/dial/dial.rs:15`  · exit 0 · 共享/装置步

**工具返回**：
```
at src/control/object/dial/dial.rs:15 — the definition `offset` (lines 15-17)
  contract   (from the doc above it, which is the promise this tree writes)
             src/control/object/dial/dial.rs:14 /// This widget's offset, in the coordinate family its siblings use.
  siblings   8 file(s) under src/control/object define `offset` too: src/control/object/badge/badge.rs, src/control/object/button/button.rs, src/control/object/gauge/gauge.rs, src/control/object/panel/panel.rs, src/control/object/slider/slider.rs, src/control/object/spinner/spinner.rs, src/control/object/timeline/timeline.rs, src/control/object/toggle/toggle.rs — a change here is a change to the family (`consistency --parent` compares their declared shapes)
  callers    0 in this root
  pins       no test names `offset` — nothing pins this, so a change here is unverified until something does
  adoption   no ledger at .nichlink/adopted/entries in this root
  gate       no `#[cfg]` attribute sits directly above this definition at src/control/object/dial/dial.rs:15 (a gate on the enclosing module is not read here)
  scope      scope unknown (no source_scope.tsv; run `nichlink check`)
  pruning    pruning unknown (no pruning_manifest.tsv; run `nichlink check`)
  wiring     no declared cut in /home/nich/Moirai_N3/nichlink/target/round9/trees/h3-cross-file-chain/host/src/lib.rs names `root/control/dial` — the entry declares 8 cut(s), and a face no cut names is one the build replaces with nothing (`grafts` lists them)
not covered here: how a grafted subtree looks at runtime (ask `trace`), and whether the declarations above describe the tree **as it is now** — the scope and pruning lines are the build's own output, and a tree built before the last edit says so on the `scope` line
next   `check {face}` to run the face that compiles it, `grafts` for every declared cut, `locate {symptom}` for sibling places to compare it with
evidence nichlink.why {"at":"src/control/object/dial/dial.rs:15"} → exit 0

```


## 交付答案（逐字）

# h3-cross-file-chain — 入口计划从没点名那个新面 ⇒ 构建期作用域把它排除在构建之外
root cause — `src/lib.rs:48-66` 的 `static_graft_plan!`：它声明了 **8 个 cut**（button `:50`、slider `:52`、timeline `:54`、gauge `:56`、toggle `:58`、spinner `:60`、panel `:62`、badge `:64`），**没有 dial 的那一条**；缺的正是 `:65`（badge 的 graft 行）与 `:66`（`);`）之间应当追加的 `cut(crate::control::object::dial::NODE_ID) graft(control_button_graft::dial_fast::NODE_ID),`。
mechanism — 构建期作用域从入口推导：`cut` 是"强制存活根"，作用域保留的**恰好是切口命名的子树**（`toolchain/src/build_time/src/scope.rs:249-318`；宿主自己的文档也这么写，`src/lib.rs:36-43`："the build-time scope narrows to the subtrees these cuts name: a face nobody declared is not shipped by this application"）。dial 的源码在（`src/control/object/dial/dial.rs`，`registry` 也把它当**派生树**里的第 10 个面列出），但没有任何切口点名它 ⇒ 生成的发布树不挂载它 ⇒ `outline()`（`src/lib.rs:69-95`）里没有 `root/control/dial` ⇒ `tests/shipped.rs:8-23` 按名字逐个断言时 `dial` 落空。
evidence — ① `cargo test --offline`（root=h3-cross-file-chain/host，exit **101**）：`every_declared_widget_ships ... FAILED`，panic 在 `tests/shipped.rs:19`：`` `dial` is declared and does not ship; the tree is: `` 后列出 9 行（control + badge/button/gauge/panel/slider/spinner/timeline/toggle，**无 dial**）。
② `check {face:default}` → `verdict failed (cargo exit 101)`，同一条（log `…/host/target/nichlink/out/check-default.log`）。
③ `registry` → `faces 10`，其中含 `root/control/dial Dial control/object/dial/dial.rs e1070833…` ⇒ 源码侧没问题，差的是**发布**那一步。
④ `why {at:src/control/object/dial/dial.rs:15}` → `wiring no declared cut in …/host/src/lib.rs names root/control/dial — the entry declares 8 cut(s), and a face no cut names is one the build replaces with nothing`；同一回复还说 `siblings 8 file(s) … define offset too`、`callers 0 in this root`、`pins no test names offset`。
⑤ 外部实现**已经在了**：`graft/src/dial_fast.rs` 存在，`graft/src/lib.rs:17` 声明 `pub mod dial_fast;`，`:32` 把 `dial_fast::REGISTRATION` 放进 `external_registry()` ⇒ 缺口只在宿主入口的计划里。
counter-proof — 若缺口在"外 graft 没有 dial_fast"，`graft/src/lib.rs:17/32` 不会已经写它（会）；若缺口在 dial 面的形状/路径，`registry`/`explain` 不会把它列成一个正常的第 10 面（会）。构造证据：把整棵树复制到 **/tmp**（不碰本树），只在 `src/lib.rs` 的 `static_graft_plan!` 里补上 dial 的那两行，重跑 → `every_declared_widget_ships ... ok`、`test result: ok. 1 passed; 0 failed`（exit **0**）。
fix — 最小改动：在 `src/lib.rs:65` 与 `:66` 之间补 2 行（`cut(...dial::NODE_ID)` + `graft(...dial_fast::NODE_ID)`）。未改动：`tests/shipped.rs`（不改测试）、`src/control/object/dial/dial.rs`（面本身是好的）、`graft/**`（外部实现已存在）、其余 8 个切口。本树**只读**，以上均未落盘；探针只在 /tmp 副本上做。
调用清单（`target/round9/logs/h3-cross-file-chain.jsonl`）：check(default) · registry · why(dial.rs:15)。
总调用：成功 3 / 被拒 0
