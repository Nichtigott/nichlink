# ours · `h2-claim-unkept`

**题面**：hardbug h2：采信未落实——真台账 + 一个同族只缺一条声明 + 一条指纹失效。

> 步 7 · 仪器调用 **5**（逐题日志条数）· 会话里定位到 2 条 · 输出 9,127 tok · 推理 13,020 字符 · 累计上下文 2,076,544

## 一、工具给了什么 · 当时 agent 在想什么（逐条配对）

> ⚠️ 日志 5 条 vs 会话定位 2 条：差的那些调用发生在**共享步**（一条命令里跑了多道题）或**装置步**里 —— 这正是"逐题 token 不可归因"的具体形状。

### ①1 日志第 1 条 · exit 0 · step 149（turn 1）

**命令**：`--call adopted --root target/round9/trees/h2-claim-unkept/host`

**工具返回**（逐字）：
```
evidence: adoption ledger at /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host/.xirang/adopted/entries (provisional by construction)
adopted since 2026-10-01T10:00:00+08:00 (unchanged since the confirmation) (provisional) — root/control/button: the reference sibling shape [evidence: traced once; confirmed by nich; why: the shape every sibling follows]
adoption lapsed at src/control/object/panel/panel.rs (changed since the confirmation); the recorded print is `root/control/button`'s current print, not this file's — a copy of another entry's bytes; needs confirmation — root/control/panel: the panel's original contract [adopted at 2026-09-20T09:00:00+08:00 by nich; why: recorded before the panel was rewritten]
provisional 1  lapsed 1
note: nothing here renews or revokes an adoption — a lapsed one needs a person to confirm the new state, and a confirmation is one more line. A later line for the same anchor is the one in force; an earlier line stays as its history, which is why a lapsed line can sit above a provisional one and still be the truth about the bytes it named.
next   `conformance {anchor: "root/control/panel"}` says whether that lease still holds and where it lapsed; a route the ledger does not name is a **new anchor** — a first confirmation, not a renewal: pass `anchor`, `certifies`, `evidence`, `verifier`, `reason` and `files` with `apply: true` and `confirm: true`, and this tool appends one line whose fingerprint it computes from those files
evidence xirang.adopted {} → exit 0

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
- `bash` → 归属 h1-supply-chain：`{"command": "cd /home/nich/Moirai_N3/nichlink; B=./target/debug/xirang-mcp; R=target/round9/trees/h1-supply-chain/host; L=target/round9/logs/h1-supply-chain.jsonl\necho '===== consistency parent control'; $B --call consistency --root $R --log $L --parent root/control; echo \"EXIT=$?\"\necho '=====
…[参数：此处截断，全文 617 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

### ①2 日志第 2 条 · exit 0 · step 154（turn 1）

**命令**：`--call conformance --root target/round9/trees/h2-claim-unkept/host --anchor root/control/button`

**工具返回**（逐字）：
```
anchor `root/control/button` — 1 revision(s) in the ledger; the last is in force (an adoption is a lease: the newest line wins)
  in force   provisional — certifies: the reference sibling shape (adopted 2026-10-01T10:00:00+08:00 by nich)
  bytes      unchanged since the confirmation
  covers     src/control/object/button/button.rs
not covered here: whether the siblings of this anchor's object follow the same shape (ask `consistency --specimen <anchor>`, which reads the declared fields from the files this entry covers and compares the siblings against them), and the shape of the ledger's own history beyond the newest line
next   `adopted` for every entry's verdict, `consistency --specimen root/control/button` for whether the siblings follow this shape
evidence xirang.conformance {"anchor":"root/control/button"} → exit 0

```

**该步推理**（逐字）：
```
The probe confirms: changing only toggle's `to_world` → `to_local` makes the offsets test pass (1 passed) on a /tmp copy. Root cause confirmed with a constructed counter-proof.

Now let me add the h2 `check` (green) and then write the four hardbug answers.

Let me run h2's check quickly.
```

### ①3 日志第 3 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`--call conformance --root target/round9/trees/h2-claim-unkept/host --anchor root/control/panel`

**工具返回**（逐字）：
```
anchor `root/control/panel` — 1 revision(s) in the ledger; the last is in force (an adoption is a lease: the newest line wins)
  in force   lapsed at src/control/object/panel/panel.rs: the bytes moved after the confirmation, so this needs a **person**, not an edit (adopted 2026-09-20T09:00:00+08:00 by nich)
  bytes      changed since the confirmation (src/control/object/panel/panel.rs)
  covers     src/control/object/panel/panel.rs
not covered here: whether the siblings of this anchor's object follow the same shape (ask `consistency --specimen <anchor>`, which reads the declared fields from the files this entry covers and compares the siblings against them), and the shape of the ledger's own history beyond the newest line
next   `adopted` for every entry's verdict, `consistency --specimen root/control/panel` for whether the siblings follow this shape
evidence xirang.conformance {"anchor":"root/control/panel"} → exit 0

```

### ①4 日志第 4 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`--call consistency --root target/round9/trees/h2-claim-unkept/host --specimen root/control/button`

**工具返回**（逐字）：
```
specimen root/control/button — ledger revision 1 is the one in force (an adoption is a lease: the newest line wins)
state      provisional — certifies: the reference sibling shape (adopted 2026-10-01T10:00:00+08:00 by nich)
covers     src/control/object/button/button.rs
shape      exports `control.render` · handle_traits `ControlHandle`
tree derived now (no published records at /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host/target/xirang/out; cannot read /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host/target/xirang/out/source_scope.tsv: No such file or directory (os error 2))
family root/control · member xirang-example-control-button · 8 sibling(s)
  outlier     spinner: lacks `handle_traits`
conformance: 1 of 8 sibling(s) do not carry the specimen's shape
not covered by this comparison: it reads the shape the ledger's files declare (parts, exports, handle_traits, part_traits) through the kernel's face parser and compares each sibling's own file against it, so a shape stated in a shared helper, produced by a macro this tree does not spell, or carried only by runtime behaviour is invisible; `parts` is compared by presence alone because it names each object's own parts type, while each label the specimen states in `exports`, `handle_traits` and `part_traits` has to appear in the sibling's list — a label the sibling states and the specimen does not is that sibling's own business; a sibling that declares **more** fields than the specimen is not reported, because the specimen is the baseline and the question is whether it was followed; and a deviation is a place to look, not a defect — whether the design should still be this way is the ledger's own question
next   `read {path, line}` for a deviating sibling's body, `conformance {anchor}` for the ledger's own verdict on this specimen
evidence xirang.consistency {"specimen":"root/control/button"} → exit 0

```

### ①5 日志第 5 条 · exit 0 · 会话里未定位（共享/装置步）

**命令**：`--call check --root target/round9/trees/h2-claim-unkept/host --face default --timeout_ms 900000`

**工具返回**（逐字）：
```
verdict  passed (cargo exit 0)
check  cargo test
face   default
root   /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host
tree   14 rust file(s), 24 function(s)
elapsed 23212 ms
log    /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host/target/xirang/out/check-default.log
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
evidence xirang.check {"face":"default","timeout_ms":900000} → exit 0

```


## 二、该题相关的其它调用（读文件 / 编辑 / 跑测试 / 写答案），带当时的推理

**调用** `bash`（step 147）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host && CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/round9/.cargo-target/hardbugs timeout 1200 cargo test --offline 2>&1 | tail -20; echo \"EXIT=${PIPESTATUS[0]}\"; echo '=== ledger'; grep -n '' .xirang/adopted/entries", "description": "h2-claim-unkept: run tests and read ledger", "timeoutMs": 1300000}
```

**结果**：
```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.88s
     Running unittests src/lib.rs (/home/nich/Moirai_N3/nichlink/target/round9/.cargo-target/hardbugs/debug/deps/control_button-8e6932e223300a0f)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/offsets.rs (/home/nich/Moirai_N3/nichlink/target/round9/.cargo-target/hardbugs/debug/deps/offsets-33842c3c9cad555a)

running 1 test
test the_rendered_offsets_add_up ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests control_button

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

EXIT=0
=== ledger
1:root/control/button|the reference sibling shape|traced once|nich|2026-10-01T10:00:00+08:00|src/control/object/button/button.rs|edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9|the shape every sibling follows
2:root/control/panel|the panel's original contract|traced once|nich|2026-09-20T09:00:00+08:00|src/control/object/panel/panel.rs|edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9|recorded before the panel was rewritten

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

**调用** `bash`（step 150）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host && for f in button spinner panel; do echo \"===== $f\"; sed -n '1,40p' src/control/object/$f/$f.rs; done", "description": "Read h2 sibling faces"}
```

**结果**：
```
===== button
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

crate::control_object! {
    kind: Button,
    exports: ["control.render"],
    parent: crate::control::NODE_ID,
    handle_contracts: [crate::control::ControlHandle],
}
===== spinner
//! The spinner face.

use crate::control::{ControlFrame, ControlHandle, to_local};

pub struct Spinner;

impl ControlHandle for Spinner {
    /// Paint this widget onto the frame it was handed.
    fn paint(&self) -> ControlFrame {
        ControlFrame
    }
}

/// This widget's offset, in the coordinate family its siblings use.
pub fn offset(x: i32) -> i32 {
    to_local(x)
}

crate::control_object! {
    kind: Spinner,
    exports: ["control.render"],
    parent: crate::control::NODE_ID,
}
===== panel
//! The panel face.

use crate::control::{ControlFrame, ControlHandle, to_local};

pub struct Panel;

impl ControlHandle for Panel {
    /// Paint this widget onto the frame it was handed.
    fn paint(&self) -> ControlFrame {
        ControlFrame
    }
}

/// This widget's offset, in the coordinate family its siblings use.
pub fn offset(x: i32) -> i32 {
    to_local(x)
}

crate::control_object! {
    kind: Panel,
    exports: ["control.render"],
    parent: crate::control::NODE_ID,
    handle_contracts: [crate::control::ControlHandle],
}

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
- `bash` → 归属 h3-cross-file-chain：`{"command": "cd /home/nich/Moirai_N3/nichlink; B=./target/debug/xirang-mcp; R=target/round9/trees/h3-cross-file-chain/host; L=target/round9/logs/h3-cross-file-chain.jsonl\necho '===== check'; $B --call check --root $R --log $L --face default --timeout_ms 900000 2>&1 | sed -n '1,22p'; echo \"EXIT=$
…[参数：此处截断，全文 592 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

**调用** `bash`（step 155）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host/src/control/object && for f in button spinner panel badge; do echo \"=== $f\"; grep -n \"control_object!\\|kind:\\|exports:\\|parent:\\|handle_contracts:\\|^}\" $f/$f.rs | tail -8; done", "description": "Exact macro block line numbers in h2"}
```

**结果**：
```
=== button
12:}
17:}
19:crate::control_object! {
20:    kind: Button,
21:    exports: ["control.render"],
22:    parent: crate::control::NODE_ID,
23:    handle_contracts: [crate::control::ControlHandle],
24:}
=== spinner
12:}
17:}
19:crate::control_object! {
20:    kind: Spinner,
21:    exports: ["control.render"],
22:    parent: crate::control::NODE_ID,
23:}
=== panel
12:}
17:}
19:crate::control_object! {
20:    kind: Panel,
21:    exports: ["control.render"],
22:    parent: crate::control::NODE_ID,
23:    handle_contracts: [crate::control::ControlHandle],
24:}
=== badge
12:}
17:}
19:crate::control_object! {
20:    kind: Badge,
21:    exports: ["control.render"],
22:    parent: crate::control::NODE_ID,
23:    handle_contracts: [crate::control::ControlHandle],
24:}

```

**该步推理**（逐字）：
```
h2 evidence complete:
- tests pass (`check` → passed).
- Ledger: two entries, **identical fingerprints** (button's and panel's) → the panel line's print is a copy of the button line's.
- `adopted`: provisional 1 / lapsed 1 (panel lapsed, "the recorded print is root/control/button's current print, not this file's — a copy of another entry's bytes").
- `consistency --specimen root/control/button`: outlier `spinner: lacks handle_traits` (1 of 8); reference shape = exports `control.render` · handle_traits `ControlHandle`.
- spinner.rs macro block at :20-23 lacks `handle_contracts: [crate::control::ControlHandle]` (which is what carries the derived `handle_traits` label) — the line where the declaration is missing is :23 (the block closes at :23 without the field; siblings have `handle_contracts` after `parent` at :22). In h2's button.rs, the block is:
```
20 crate::control_object! {
21     kind: Button,
22     exports: ["control.render"],
23     parent: crate::control::NODE_ID,
24     handle_contracts: [crate::control::ControlHandle],
25 }
```
Let me verify button's exact line numbers (I printed the file earlier without numbers for button; the sed output was whole files). Let me get exact line numbers for button.rs and panel.rs macro blocks to cite "spinner.rs:23 (where the sibling's `handle_contracts` line would be)".

Root cause (file:line) for h2:
(a) `src/control/object/spinner/spinner.rs:23` — the macro block ends without the one declaration its siblings carry (`handle_contracts: [crate::control::ControlHandle]`); the census/consistency names spinner as the outlier lacking `handle_traits`.
(b) `.xirang/adopted/entries:2` — the panel line records button's fingerprint (both fingerprints are the same string), so it never described panel.rs's bytes; `conformance --anchor root/control/panel` → lapsed.

Fix (smallest edit, not applied): add `handle_contracts: [crate::control::ControlHandle],` to spinner.rs's block (1 line); and for the ledger, **a person** must confirm the panel's new state — the tool's rule: a lapsed entry needs a person, not an edit: append one line (`adopted --anchor root/control/panel --certifies … --files src/control/object/panel/panel.rs --confirm --apply`), leaving the history line in place. Files not touched: tests, button.rs, control.rs, the other 7 siblings, and the existing ledger line 2 (history stays).

Now let me get exact line numbers for spinner/button/panel macro blocks.

Then write the 4 hardbug answers with the five fields: `root cause`, `mechanism`, `evidence`, `counter-proof`, `fix`.

Let me grab the line numbers.
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h2-claim-unkept：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host/src/control/object && for f in button spinner panel badge; do echo \"=== $f\"; grep -n \"control_object!\\|kind:\\|exports:\\|parent:\\|handle_contracts:\\|^}\" $f/$f.rs | tail -8; done", "description": "Exact mac
…[参数：此处截断，全文 329 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

**调用** `write`（step 156）：
```
{"content": "# h2-claim-unkept — 真标本台账 + 一个同族只缺一条声明 + 一条指纹失效的条目\n\nroot cause — 两处，各自独立：\n① `src/control/object/spinner/spinner.rs:23`：它的 `crate::control_object!` 块（`:19-23`）在 `parent:` 之后**直接闭合**，\n   少了兄弟都写在**同一行号**的那一条 `handle_contracts: [crate::control::ControlHandle],`（button/panel/badge 皆为 `:23`）。\n② `.xirang/adopted/entries:2`（`root/control/panel` 那一行）记录的指纹是 **button 那一行的副本**：\n   两行指纹字符串逐字相同（`edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9`），\n   因此它从未描述过 `panel.rs` 的字节。\n\nmechanism — ① 参考形状（台账在册的 `root/control/button`，`comparison` 报 `shape: exports control.render · handle_traits ControlHandle`）\n   里 `handle_traits` 这个标签是**从 `handle_contracts` 的 Rust 路径派生**的（`kernel/src/registry_core/authoring/parse/parse.rs:197-210`，\n   宏侧同规则）；spinner 少了这条声明 ⇒ 解析出的面缺 `handle_traits` ⇒ `consistency --specimen` 点名它。\n   ② 台账是\"租约\"：一行只为它写下的那些文件与那枚指纹背书；panel 行的指纹其实是 button 的，\n   与 `panel.rs` 现字节对不上（且 panel 被重写过），于是该条目 **lapsed**。\n\nevidence — ① `cargo test --offline`（root=h2-claim-unkept/host，exit **0**）；`check {face:default}` → `verdict passed (cargo exit 0)`\n   （症状不在红里，在静态事实上）。\n② `adopted` → `provisional 1  lapsed 1`：button「unchanged since the confirmation (provisional)」；\n   panel「adoption lapsed at src/control/object/panel/panel.rs (changed since the confirmation); the recorded print is\n   `root/control/button`'s current print, not this file's — a copy of another entry's bytes; needs confirmation」。\n③ `conformance {anchor:root/control/panel}` → `in force lapsed at src/control/object/panel/panel.rs: the bytes moved after\n   the confirmation, so this needs a **person**, not an edit`、`bytes changed since the confirmation`。\n④ `consistency {specimen:root/control/button}` → `family root/control · 8 sibling(s)`、`outlier spinner: lacks handle_traits`、\n   `conformance: 1 of 8 sibling(s) do not carry the specimen's shape`。\n⑤ `cat .xirang/adopted/entries`：两行指纹列逐字相同（见 ②）。\n\ncounter-proof — 若 spinner 缺的不是\"恰好一条声明\"，`consistency --specimen` 会点名它缺的**不止**
…[参数：此处截断，全文 2940 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**结果**：
```
<path>/home/nich/Moirai_N3/nichlink/target/round9/answers/h2-claim-unkept.md</path>
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

**调用** `read`（step 163）：
```
{"file_path": "/home/nich/Moirai_N3/nichlink/target/round9/answers/h2-claim-unkept.md"}
```

**结果**：
```
<path>/home/nich/Moirai_N3/nichlink/target/round9/answers/h2-claim-unkept.md</path>
<type>file</type>
<content>
1: # h2-claim-unkept — 真标本台账 + 一个同族只缺一条声明 + 一条指纹失效的条目
2: root cause — 两处，各自独立：
3: ① `src/control/object/spinner/spinner.rs:23`：它的 `crate::control_object!` 块（`:19-23`）在 `parent:` 之后**直接闭合**，
4:    少了兄弟都写在**同一行号**的那一条 `handle_contracts: [crate::control::ControlHandle],`（button/panel/badge 皆为 `:23`）。
5: ② `.xirang/adopted/entries:2`（`root/control/panel` 那一行）记录的指纹是 **button 那一行的副本**：
6:    两行指纹字符串逐字相同（`edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9`），
7:    因此它从未描述过 `panel.rs` 的字节。
8: mechanism — ① 参考形状（台账在册的 `root/control/button`，`comparison` 报 `shape: exports control.render · handle_traits ControlHandle`）
9:    里 `handle_traits` 这个标签是**从 `handle_contracts` 的 Rust 路径派生**的（`kernel/src/registry_core/authoring/parse/parse.rs:197-210`，
10:    宏侧同规则）；spinner 少了这条声明 ⇒ 解析出的面缺 `handle_traits` ⇒ `consistency --specimen` 点名它。
11:    ② 台账是"租约"：一行只为它写下的那些文件与那枚指纹背书；panel 行的指纹其实是 button 的，
12:    与 `panel.rs` 现字节对不上（且 panel 被重写过），于是该条目 **lapsed**。
13: evidence — ① `cargo test --offline`（root=h2-claim-unkept/host，exit **0**）；`check {face:default}` → `verdict passed (cargo exit 0)`
14:    （症状不在红里，在静态事实上）。
15: ② `adopted` → `provisional 1  lapsed 1`：button「unchanged since the confirmation (provisional)」；
16:    panel「adoption lapsed at src/control/object/panel/panel.rs (changed since the confirmation); the recorded print is
17:    `root/control/button`'s current print, not this file's — a copy of another entry's bytes; needs confirmation」。
18: ③ `conformance {anchor:root/control/panel}` → `in force lapsed at src/control/object/panel/panel.rs: the bytes moved after
19:    the confirmation, so this needs a **person**, not an edit`、`bytes changed since the confirmation`。
20: ④ `consistency {specimen:root/control/button}` → `family root/control · 8 sibling(s)`、`outlier spinner: lacks handle_traits`、
21:    `conformance: 1 of 8 sibling(s) do not carry the specimen's shape`。
22: ⑤ `cat .xirang/adopted/entries`：两行指纹列逐字相同（见 ②）。
23: counter-proof — 若 spinner 缺的不是"恰好一条声明"，`consistency --specimen` 会点名它缺的**不止** `handle_traits`
24: （它的 `exports` 在 `spinner.rs:21`，与参考形状一致）；若 panel 那条只是"旧但仍在描述它自己的字节"，
25: `adopted` 会判 provisional、而不是判"记录的是 button 的当前指纹"——它判的是后者，且两行指纹确实同一串。
26: 另一侧：若 panel.rs 从未变动，`conformance` 不会报 bytes changed——它报了。
27: fix — ① 最小改动：在 `src/control/object/spinner/spinner.rs:22` 之后补回 **1 行**
28:    `handle_contracts: [crate::control::ControlHa
…[结果：此处截断，全文 3068 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**该步推理**（逐字）：
```
Three files still over 30: g3 (33), h2-claim-unkept (35), h3-cross-file-chain (31). Let me trim those three manually (merge a few multi-line entries into single lines).

Let me read g3.md, h2-claim-unkept.md, h3-cross-file-chain.md and compact them.
```

**同一步并行发出的其它调用**：
- `read` → 归属 g3：`{"file_path": "/home/nich/Moirai_N3/nichlink/target/round9/answers/g3.md"}`
- `read` → 归属 h2-claim-unkept：`{"file_path": "/home/nich/Moirai_N3/nichlink/target/round9/answers/h2-claim-unkept.md"}`

**调用** `write`（step 164）：
```
{"content": "# h2-claim-unkept — 真标本台账 + 一个同族只缺一条声明 + 一条指纹失效的条目\nroot cause — 两处，各自独立：① `src/control/object/spinner/spinner.rs:23`：它的 `crate::control_object!` 块（`:19-23`）在 `parent:` 之后**直接闭合**，少了兄弟都写在**同一行号**的那一条 `handle_contracts: [crate::control::ControlHandle],`（button/panel/badge 皆为 `:23`）。② `.xirang/adopted/entries:2`（`root/control/panel` 那一行）记录的指纹是 **button 那一行的副本**：两行指纹字符串逐字相同（`edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9`），因此它从未描述过 `panel.rs` 的字节。\nmechanism — ① 参考形状（台账在册的 `root/control/button`，`consistency --specimen` 报 `shape: exports control.render · handle_traits ControlHandle`）里 `handle_traits` 这个标签是**从 `handle_contracts` 的 Rust 路径派生**的（`kernel/src/registry_core/authoring/parse/parse.rs:197-210`，宏侧同规则）；spinner 少这条声明 ⇒ 解析出的面缺 `handle_traits` ⇒ 被点名。② 台账是\"租约\"：一行只为它写下的文件与那枚指纹背书；panel 行的指纹其实是 button 的，与 `panel.rs` 现字节对不上（且 panel 被重写过）⇒ 该条目 **lapsed**。\nevidence — ① `cargo test --offline`（root=h2-claim-unkept/host，exit **0**）；`check {face:default}` → `verdict passed (cargo exit 0)`（症状不在红里，在静态事实上）。\n② `adopted` → `provisional 1  lapsed 1`：button「unchanged since the confirmation (provisional)」；panel「adoption lapsed at src/control/object/panel/panel.rs (changed since the confirmation); the recorded print is `root/control/button`'s current print, not this file's — a copy of another entry's bytes; needs confirmation」。\n③ `conformance {anchor:root/control/panel}` → `in force lapsed at src/control/object/panel/panel.rs: the bytes moved after the confirmation, so this needs a **person**, not an edit`、`bytes changed since the confirmation`。\n④ `consistency {specimen:root/control/button}` → `family root/control · 8 sibling(s)`、`outlier spinner: lacks handle_traits`、`conformance: 1 of 8 sibling(s) do not carry the specimen's shape`。\n⑤ `cat .xirang/adopted/entries`：两行指纹列逐字相同（见 ②）。\ncounter-proof — 若 spinner 缺的不是\"恰好一条声明\"，`consistency --specimen` 会点名它缺的**不止** `handle_traits`（它的 `exports` 在 `spinner.rs:21`，与参考形状一致）；若 panel 那条只是\"旧但仍在描述它自己的字节\"，`
…[参数：此处截断，全文 2809 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**结果**：
```
<path>/home/nich/Moirai_N3/nichlink/target/round9/answers/h2-claim-unkept.md</path>
<type>file</type>
<content>
Updated file
</content>
```

**该步推理**（逐字）：
```
Let me compact g3 to ≤30 (33 → 30 by merging lines 15-16, 19-20, 22-23 etc.) and h2 to ≤30 (35 → 30 by merging continuation lines).

For g3: merge lines into fewer:
- lines 2-5 (①): keep 2 lines.
- lines 6-10 (②): 3 lines.
- lines 11-18 (③): 4 lines.
- lines 19-23 (④): 3 lines.
- lines 24-32 (⑤): 8 lines → could compress to 5 by merging related calls.
- header + tally = 2.

Let me rewrite g3.md compactly (target ~26 lines).

For h2: compress evidence items to fewer lines (merge wrapped lines).

Let me rewrite both files.
```

**同一步并行发出的其它调用**：
- `write` → 归属 g3：`{"content": "# g3 — 给这个对象加一层内部结构（可改；注册树与公开路径不动，门绿）\n**① 做了什么** 对象 = **`root/control/button`**（`src/control/object/button/button.rs`；树里另外两个面是父面 `root/control` 与兄弟 `root/control/slider`）。用桥的 `deepen` 在**该面自己的文件里**加一层零件：`inside.parts = {\"label\": \"String\"}`（label 正是该面 `runtime_checks: [NON_EMPTY_TEX
…[参数：此处截断，全文 2322 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`
- `write` → 归属 h2-claim-unkept：`{"content": "# h2-claim-unkept — 真标本台账 + 一个同族只缺一条声明 + 一条指纹失效的条目\nroot cause — 两处，各自独立：① `src/control/object/spinner/spinner.rs:23`：它的 `crate::control_object!` 块（`:19-23`）在 `parent:` 之后**直接闭合**，少了兄弟都写在**同一行号**的那一条 `handle_contracts: [crate::control::ControlHandle],`（button/panel/badge 皆为 `:23`）。② `.n
…[参数：此处截断，全文 2809 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`


## 三、交付答案（逐字）

# h2-claim-unkept — 真标本台账 + 一个同族只缺一条声明 + 一条指纹失效的条目
root cause — 两处，各自独立：① `src/control/object/spinner/spinner.rs:23`：它的 `crate::control_object!` 块（`:19-23`）在 `parent:` 之后**直接闭合**，少了兄弟都写在**同一行号**的那一条 `handle_contracts: [crate::control::ControlHandle],`（button/panel/badge 皆为 `:23`）。② `.xirang/adopted/entries:2`（`root/control/panel` 那一行）记录的指纹是 **button 那一行的副本**：两行指纹字符串逐字相同（`edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9`），因此它从未描述过 `panel.rs` 的字节。
mechanism — ① 参考形状（台账在册的 `root/control/button`，`consistency --specimen` 报 `shape: exports control.render · handle_traits ControlHandle`）里 `handle_traits` 这个标签是**从 `handle_contracts` 的 Rust 路径派生**的（`kernel/src/registry_core/authoring/parse/parse.rs:197-210`，宏侧同规则）；spinner 少这条声明 ⇒ 解析出的面缺 `handle_traits` ⇒ 被点名。② 台账是"租约"：一行只为它写下的文件与那枚指纹背书；panel 行的指纹其实是 button 的，与 `panel.rs` 现字节对不上（且 panel 被重写过）⇒ 该条目 **lapsed**。
evidence — ① `cargo test --offline`（root=h2-claim-unkept/host，exit **0**）；`check {face:default}` → `verdict passed (cargo exit 0)`（症状不在红里，在静态事实上）。
② `adopted` → `provisional 1  lapsed 1`：button「unchanged since the confirmation (provisional)」；panel「adoption lapsed at src/control/object/panel/panel.rs (changed since the confirmation); the recorded print is `root/control/button`'s current print, not this file's — a copy of another entry's bytes; needs confirmation」。
③ `conformance {anchor:root/control/panel}` → `in force lapsed at src/control/object/panel/panel.rs: the bytes moved after the confirmation, so this needs a **person**, not an edit`、`bytes changed since the confirmation`。
④ `consistency {specimen:root/control/button}` → `family root/control · 8 sibling(s)`、`outlier spinner: lacks handle_traits`、`conformance: 1 of 8 sibling(s) do not carry the specimen's shape`。
⑤ `cat .xirang/adopted/entries`：两行指纹列逐字相同（见 ②）。
counter-proof — 若 spinner 缺的不是"恰好一条声明"，`consistency --specimen` 会点名它缺的**不止** `handle_traits`（它的 `exports` 在 `spinner.rs:21`，与参考形状一致）；若 panel 那条只是"旧但仍在描述它自己的字节"，`adopted` 会判 provisional 而不是判"记录的是 button 的当前指纹"——它判的是后者，且两行指纹确实同一串；若 panel.rs 从未变动，`conformance` 不会报 bytes changed——它报了。
fix — ① 最小改动：`src/control/object/spinner/spinner.rs:22` 之后补回 **1 行** `handle_contracts: [crate::control::ControlHandle],`（与 button/panel/badge 的 `:23` 同形）。② 台账**不能**靠编辑修：按工具规则，失效的条目需要**人**再追加一行确认（`adopted` 的 `anchor/certifies/evidence/verifier/reason/files` + `apply/confirm`，指纹由它自己算），**旧行保留为历史**。未改动：`tests/`（不许改测试）、`src/control/control.rs`、其余 7 个兄弟、`.xirang/adopted/entries:1`（button 那行）以及 panel 行本身（历史不改写）。本树**只读**，以上均未落盘。
调用清单（`target/round9/logs/h2-claim-unkept.jsonl`）：adopted · conformance(button) · conformance(panel) · consistency(specimen=button) · check(default)。
总调用：成功 5 / 被拒 0
