# ours · `h2-claim-unkept`（精简版；完整版见 `../ours/h2-claim-unkept.md`）

**题面**：hardbug h2：采信未落实——真台账 + 一个同族只缺一条声明 + 一条指纹失效。

> 仪器调用 **5** · 步 7 · 输出 9,127 tok · 推理 13,020 字符 · 累计上下文 2,076,544

## 仪器调用序列（工具给了什么 + 当时 agent 在想什么）

### ①1 `--call adopted --root target/round9/trees/h2-claim-unkept/host`  · exit 0 · step 149（turn 1）

**工具返回**：
```
evidence: adoption ledger at /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host/.nichlink/adopted/entries (provisional by construction)
adopted since 2026-10-01T10:00:00+08:00 (unchanged since the confirmation) (provisional) — root/control/button: the reference sibling shape [evidence: traced once; confirmed by nich; why: the shape every sibling follows]
adoption lapsed at src/control/object/panel/panel.rs (changed since the confirmation); the recorded print is `root/control/button`'s current print, not this file's — a copy of another entry's bytes; needs confirmation — root/control/panel: the panel's original contract [adopted at 2026-09-20T09:00:00+08:00 by nich; why: recorded before the panel was rewritten]
provisional 1  lapsed 1
note: nothing here renews or revokes an adoption — a lapsed one needs a person to confirm the new state, and a confirmation is one more line. A later line for the same anchor is the one in force; an earlier line stays as its history, which is why a lapsed line can sit above a provisional one and still be the truth about the bytes it named.
next   `conformance {anchor: "root/control/panel"}` says whether that lease still holds and where it lapsed; a route the ledger does not name is a **new anchor** — a first confirmation, not a renewal: pass `anchor`, `certifies`, `evidence`, `verifier`, `reason` and `files` with `apply: true` and `confirm: true`, and this tool appends one line whose fingerprint it computes from those files
evidence nichlink.adopted {} → exit 0

```

**当时推理**：
```
**h3**: test `every_declared_widget_ships` fails: "`dial` is declared and does not ship". The tree has 9 rows, no dial. So the dial widget exists in the sources (declared) but doesn't ship — the entry plan never names it. Let me find the "declaration" the test refers to (probably a `DECLARED` list in the test or a declaration in the sources).

**h4**: tests pass (green). The defect: one algorithm does the opposite of its documentation, no test covers the branch. From the source:
- `zero_arm()` at `src/model/entry.rs:30-32` **always returns `ZeroArm::Post`** — the enum at :5-10 has two variants (Post/Refuse) with docs; the `Refuse` arm is never constructed (never returned). `postable()` at :38-46: has_receipt check, then `match self.zero_arm() { Post => true, Refuse => false }`. Since zero_arm always returns Post, a zero-amount entry WITH a receipt is postable — contradicting the doc at :36-37 "a zero entry never is". And the `ZeroArm::Refuse` arm at :44 is uncovered (no test covers it). Also the tests only cover the receipt branch.

Hmm: "exactly one branch inverted and exactly one branch uncovered" — the inverted branch: `zero_arm` returning Post instead of Refuse (`Post`/`Refuse` inverted: "A zero entry is posted like any other" is the wrong arm). Hmm, which is "inverted"? The doc says a zero entry never posts; so `zero_arm` should return `Refuse`. The function's own doc says "Which arm decides a zero amount" — hmm. The inversion could also be described as: `match self.zero
…[推理：此处截断，全文 4080 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**同一步还发了**：`bash`→h1-supply-chain

### ①2 `--call conformance --root target/round9/trees/h2-claim-unkept/host --anchor root/control/button`  · exit 0 · step 154（turn 1）

**工具返回**：
```
anchor `root/control/button` — 1 revision(s) in the ledger; the last is in force (an adoption is a lease: the newest line wins)
  in force   provisional — certifies: the reference sibling shape (adopted 2026-10-01T10:00:00+08:00 by nich)
  bytes      unchanged since the confirmation
  covers     src/control/object/button/button.rs
not covered here: whether the siblings of this anchor's object follow the same shape (ask `consistency --specimen <anchor>`, which reads the declared fields from the files this entry covers and compares the siblings against them), and the shape of the ledger's own history beyond the newest line
next   `adopted` for every entry's verdict, `consistency --specimen root/control/button` for whether the siblings follow this shape
evidence nichlink.conformance {"anchor":"root/control/button"} → exit 0

```

**当时推理**：
```
The probe confirms: changing only toggle's `to_world` → `to_local` makes the offsets test pass (1 passed) on a /tmp copy. Root cause confirmed with a constructed counter-proof.

Now let me add the h2 `check` (green) and then write the four hardbug answers.

Let me run h2's check quickly.
```

### ①3 `--call conformance --root target/round9/trees/h2-claim-unkept/host --anchor root/control/panel`  · exit 0 · 共享/装置步

**工具返回**：
```
anchor `root/control/panel` — 1 revision(s) in the ledger; the last is in force (an adoption is a lease: the newest line wins)
  in force   lapsed at src/control/object/panel/panel.rs: the bytes moved after the confirmation, so this needs a **person**, not an edit (adopted 2026-09-20T09:00:00+08:00 by nich)
  bytes      changed since the confirmation (src/control/object/panel/panel.rs)
  covers     src/control/object/panel/panel.rs
not covered here: whether the siblings of this anchor's object follow the same shape (ask `consistency --specimen <anchor>`, which reads the declared fields from the files this entry covers and compares the siblings against them), and the shape of the ledger's own history beyond the newest line
next   `adopted` for every entry's verdict, `consistency --specimen root/control/panel` for whether the siblings follow this shape
evidence nichlink.conformance {"anchor":"root/control/panel"} → exit 0

```

### ①4 `--call consistency --root target/round9/trees/h2-claim-unkept/host --specimen root/control/button`  · exit 0 · 共享/装置步

**工具返回**：
```
specimen root/control/button — ledger revision 1 is the one in force (an adoption is a lease: the newest line wins)
state      provisional — certifies: the reference sibling shape (adopted 2026-10-01T10:00:00+08:00 by nich)
covers     src/control/object/button/button.rs
shape      exports `control.render` · handle_traits `ControlHandle`
tree derived now (no published records at /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host/target/nichlink/out; cannot read /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host/target/nichlink/out/source_scope.tsv: No such file or directory (os error 2))
family root/control · member nichlink-example-control-button · 8 sibling(s)
  outlier     spinner: lacks `handle_traits`
conformance: 1 of 8 sibling(s) do not carry the specimen's shape
not covered by this comparison: it reads the shape the ledger's files declare (parts, exports, handle_traits, part_traits) through the kernel's face parser and compares each sibling's own file against it, so a shape stated in a shared helper, produced by a macro this tree does not spell, or carried only by runtime behaviour is invisible; `parts` is compared by presence alone because it names each object's own parts type, while each label the specimen states in `exports`, `handle_traits` and `part_traits` has to appear in the sibling's list — a label the sibling states and the specimen does not is that sibling's own business; a sibling that declares **more** fields than the specimen is not reported, because the specimen is the baseline and the question is whether it was followed; and a deviation is a place to look, not a defect — whether the design should still be this way is the ledger's own question
next   `read {path, line}` for a deviating sibling's body, `conformance {anchor}` for the ledger's own verdict on this specimen
evidence nichlink.consistency {"specimen":"root/control/button"} → exit 0

```

### ①5 `--call check --root target/round9/trees/h2-claim-unkept/host --face default --timeout_ms 900000`  · exit 0 · 共享/装置步

**工具返回**：
```
verdict  passed (cargo exit 0)
check  cargo test
face   default
root   /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host
tree   14 rust file(s), 24 function(s)
elapsed 23212 ms
log    /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host/target/nichlink/out/check-default.log
exit   0
result test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
result test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
result test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
census: 0 named numeric constant(s); a static fact about this tree, not a verdict
  declarations: 4 production `pub fn` name(s) appear in no test file (a text-level count: a test that reaches one without writing its name does not count here)
  test-reachable: 12 of 22 production function(s) no test can reach (24 function(s) indexed in this tree; a static walk from the test files along the same name-in-call-list rule the orphan view uses)
  branch-level: 0 constructively unreachable arm(s) in this tree (0 `false` guard(s), 0 never-constructed variant(s); a static read of the source text, not a coverage measurement)
  entry plan: 11 `cut(` site(s) and 9 `graft(` site(s) across this tree's sources (a static count; which of them this application ships is the plan's own business)
  decl   no test names `to_local` (src/control/control.rs:5)
  decl   no test names `to_world` (src/control/control.rs:8)
  decl   no test names `base_registry` (src/lib.rs:70)
  not covered: the reachability column (dynamic dispatch, function pointers, FFI, macro expansion, trait methods and closures are invisible; matching is by name; `main` is never listed) · the branch column (no data-dependent condition is judged, `false` is the only guard literal decided, a `pub` enum is never judged, and a construction this tree does not spell would falsify a row) · this census (string constants, structural duplication, runtime behaviour and claims written in prose are outside it). A static read of the source text, not a coverage measurement; `census: true` and `--list check` have the full text
… truncated: 4 of 13 census rows withheld at the limit of 5; pass `census: true` for the whole table (every column head is already here with its count)
evidence nichlink.check {"face":"default","timeout_ms":900000} → exit 0

```


## 交付答案（逐字）

# h2-claim-unkept — 真标本台账 + 一个同族只缺一条声明 + 一条指纹失效的条目
root cause — 两处，各自独立：① `src/control/object/spinner/spinner.rs:23`：它的 `crate::control_object!` 块（`:19-23`）在 `parent:` 之后**直接闭合**，少了兄弟都写在**同一行号**的那一条 `handle_contracts: [crate::control::ControlHandle],`（button/panel/badge 皆为 `:23`）。② `.nichlink/adopted/entries:2`（`root/control/panel` 那一行）记录的指纹是 **button 那一行的副本**：两行指纹字符串逐字相同（`edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9`），因此它从未描述过 `panel.rs` 的字节。
mechanism — ① 参考形状（台账在册的 `root/control/button`，`consistency --specimen` 报 `shape: exports control.render · handle_traits ControlHandle`）里 `handle_traits` 这个标签是**从 `handle_contracts` 的 Rust 路径派生**的（`kernel/src/registry_core/authoring/parse/parse.rs:197-210`，宏侧同规则）；spinner 少这条声明 ⇒ 解析出的面缺 `handle_traits` ⇒ 被点名。② 台账是"租约"：一行只为它写下的文件与那枚指纹背书；panel 行的指纹其实是 button 的，与 `panel.rs` 现字节对不上（且 panel 被重写过）⇒ 该条目 **lapsed**。
evidence — ① `cargo test --offline`（root=h2-claim-unkept/host，exit **0**）；`check {face:default}` → `verdict passed (cargo exit 0)`（症状不在红里，在静态事实上）。
② `adopted` → `provisional 1  lapsed 1`：button「unchanged since the confirmation (provisional)」；panel「adoption lapsed at src/control/object/panel/panel.rs (changed since the confirmation); the recorded print is `root/control/button`'s current print, not this file's — a copy of another entry's bytes; needs confirmation」。
③ `conformance {anchor:root/control/panel}` → `in force lapsed at src/control/object/panel/panel.rs: the bytes moved after the confirmation, so this needs a **person**, not an edit`、`bytes changed since the confirmation`。
④ `consistency {specimen:root/control/button}` → `family root/control · 8 sibling(s)`、`outlier spinner: lacks handle_traits`、`conformance: 1 of 8 sibling(s) do not carry the specimen's shape`。
⑤ `cat .nichlink/adopted/entries`：两行指纹列逐字相同（见 ②）。
counter-proof — 若 spinner 缺的不是"恰好一条声明"，`consistency --specimen` 会点名它缺的**不止** `handle_traits`（它的 `exports` 在 `spinner.rs:21`，与参考形状一致）；若 panel 那条只是"旧但仍在描述它自己的字节"，`adopted` 会判 provisional 而不是判"记录的是 button 的当前指纹"——它判的是后者，且两行指纹确实同一串；若 panel.rs 从未变动，`conformance` 不会报 bytes changed——它报了。
fix — ① 最小改动：`src/control/object/spinner/spinner.rs:22` 之后补回 **1 行** `handle_contracts: [crate::control::ControlHandle],`（与 button/panel/badge 的 `:23` 同形）。② 台账**不能**靠编辑修：按工具规则，失效的条目需要**人**再追加一行确认（`adopted` 的 `anchor/certifies/evidence/verifier/reason/files` + `apply/confirm`，指纹由它自己算），**旧行保留为历史**。未改动：`tests/`（不许改测试）、`src/control/control.rs`、其余 7 个兄弟、`.nichlink/adopted/entries:1`（button 那行）以及 panel 行本身（历史不改写）。本树**只读**，以上均未落盘。
调用清单（`target/round9/logs/h2-claim-unkept.jsonl`）：adopted · conformance(button) · conformance(panel) · consistency(specimen=button) · check(default)。
总调用：成功 5 / 被拒 0
