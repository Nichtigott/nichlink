# ours · `h4-one-file-many-algorithms`（精简版；完整版见 `../ours/h4-one-file-many-algorithms.md`）

**题面**：hardbug h4：单文件多算法——一个文件三套算法，恰一支写反、恰一支不可达。

> 仪器调用 **2** · 步 4 · 输出 3,873 tok · 推理 10,554 字符 · 累计上下文 1,175,936

## 仪器调用序列（工具给了什么 + 当时 agent 在想什么）

### ①1 `--call check --root target/round9/trees/h4-one-file-many-algorithms --face default --census true --timeout_ms 300000`  · exit 0 · step 151（turn 1）

**工具返回**：
```
verdict  passed (cargo exit 0)
check  cargo test
face   default
root   /home/nich/Moirai_N3/nichlink/target/round9/trees/h4-one-file-many-algorithms
tree   4 rust file(s), 7 function(s)
elapsed 600 ms
log    /home/nich/Moirai_N3/nichlink/target/round9/trees/h4-one-file-many-algorithms/target/nichlink/out/check-default.log
exit   0
result test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
result test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
result test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
census: 0 named numeric constant(s); a static fact about this tree, not a verdict
  entry plan: 0 `cut(` site(s) and 0 `graft(` site(s) across this tree's sources (a static count; which of them this application ships is the plan's own business)
  decl   no test names `signed` (src/model/entry.rs:53)
  decl   no test names `normalized_account` (src/model/entry.rs:60)
  declarations: 2 production `pub fn` name(s) appear in no test file (a text-level count: a test that reaches one without writing its name does not count here)
  test-reachable: 2 of 5 production function(s) no test can reach (7 function(s) indexed in this tree; a static walk from the test files along the same name-in-call-list rule the orphan view uses)
  by directory: src/model 2 of 5
  fn     no test reaches `signed` (src/model/entry.rs:53)
  fn     no test reaches `normalized_account` (src/model/entry.rs:60)
  not covered by the test-reachability column: dynamic dispatch, function pointers, FFI, macro expansion, and reach only through a trait method or a closure are invisible, so a function reached only that way stays listed; a test-looking file (`tests/`, `_tests.rs`, or `#[test]`) seeds the walk, so a production file with its own `#[test]` is likely not listed; matching is by name, so an unrelated same-named call counts; `main` is never listed. A static walk, not a coverage measurement; `--list check` has the full text.
  branch-level: 1 constructively unreachable arm(s) in this tree (0 `false` guard(s), 1 never-constructed variant(s); a static read of the source text, not a coverage measurement)
  no construction of `ZeroArm::Refuse` is spelled in this tree, so the arm matching it in `postable` at src/model/entry.rs:44 can never be entered (the enum is private, so a constructor outside this tree cannot spell the variant either); the contract above it says 34: Whether this entry may be posted at all. / 36: The rule the service leans on: an entry is postable when it carries a receipt, and a / 37: zero entry never is. Callers rely on the refusal, so this is a contract.
  not covered by the branch-level column: a condition whose value depends on data — a field, a parameter, a comparison, a `match` over a value — is not judged at all, so an arm no run has taken yet stays invisible here; `false` is the only guard literal decided, so `1 == 2`, `!true`, a `const` bool and `cfg!(…)` are not read; macro expansion, dynamic dispatch, function pointers and FFI are invisible, while a `macro_rules!` body this tree writes **is** text — an `if false` inside one is listed (and when that body sits outside any function, its row names no function, because there is none to name), and an arm that only exists after expansion is invisible; a construction this tree does not spell (a derive that builds a value, `unsafe`, a consumer outside this root) would falsify a row; a `pub` enum is never judged, an arm reached through a wildcard or a binding is not read, and an enum name this file imports from another crate is conservatively skipped, so a same-named foreign enum's arms are a miss here rather than a false row. A static read of the source text, not a coverage measurement; `--list check` has the full text.
  not covered: this census reads exactly what the columns above name; string constants, structural duplication, runtime behaviour and claims written in prose are outside it, and the static walk is not a coverage measurement. `--list check` has the full text
evidence nichlink.check {"census":true,"face":"default","timeout_ms":300000} → exit 0

```

**当时推理**：
```
Excellent. Key h3 evidence from `why {at: dial.rs:15}`:
- `siblings 8 file(s) ... define offset too`
- `callers 0 in this root`
- `pins no test names offset`
- `wiring no declared cut in .../src/lib.rs names root/control/dial — the entry declares 8 cut(s), and a face no cut names is one the build replaces with nothing`

And `registry` shows 10 faces (dial IS in the derived tree) but the *shipped* tree (base_registry via the test) lacks dial. The test `every_declared_widget_ships` in tests/shipped.rs:19 names dial.

Hmm wait — interesting: in h3 the derived tree has dial but the shipped tree doesn't (the test says "dial is declared and does not ship"). So in h3, the dial face exists in the sources and is *derived*, but the build-time scope (8 cuts in lib.rs) doesn't name it → not shipped.

Hmm, but in fa the same thing happened (dial not mounted). In h3, is dial mounted (compiled)? The `why` shows the file has definitions and it's in the derived tree. Let me check the test to see the exact assertion and what "declared" means (maybe there's a DECLARED list in the test or a `widgets()` function). Let me read tests/shipped.rs.

Then the root cause: the entry plan (`src/lib.rs`'s `static_graft_plan!`) never names `root/control/dial` → the build-time scope leaves dial out of the build. But hmm: the class says "the entry plan never names the new face, so the build-time scope leaves it out of the build". The fix: declare it (cut+graft) at the entry — requires a `dial_fast` in the gra
…[推理：此处截断，全文 1718 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**同一步还发了**：`bash`→h3-cross-file-chain

### ①2 `--call digest --root target/round9/trees/h4-one-file-many-algorithms --file src/model/entry.rs`  · exit 0 · 共享/装置步

**工具返回**：
```
file src/model/entry.rs — 5 function(s), 63 line(s)
  src/model/entry.rs:25-27 `new` — 1 call(s) out, 2 caller(s); named by a test; contract: A new entry.
  src/model/entry.rs:30-32 `zero_arm` — 0 call(s) out, 1 caller(s); contract: Which arm decides a zero amount.
  src/model/entry.rs:38-46 `postable` — 1 call(s) out, 2 caller(s); named by a test; contract: Whether this entry may be posted at all.
  src/model/entry.rs:53-55 `signed` — 0 call(s) out, 0 caller(s); contract: The entry's magnitude, as the report prints it.
  src/model/entry.rs:60-62 `normalized_account` — 2 call(s) out, 0 caller(s); contract: The account this entry is filed under, normalized.
not covered here: which branches are dead and which are covered (ask `check {face}` for the whole-tree census), the contract in full (ask `read {path, line}`), and whether the functions are reachable from tests by the static walk (that column is in the same census)
next   `read {path, line}` for a body, `callgraph {function}` for one function's callers and callees
evidence nichlink.digest {"file":"src/model/entry.rs"} → exit 0

```


## 交付答案（逐字）

# h4-one-file-many-algorithms — 三套算法一个文件，恰一支写反、恰一支没被覆盖
root cause — `src/model/entry.rs:31`：`fn zero_arm(&self)`（定义 `:30-32`）**无条件返回 `ZeroArm::Post`**。
它自己的枚举文档写着两个含义相反的臂（`:6` "A zero entry is posted like any other." / `:9` "A zero entry is refused."），
而 `postable` 的契约（`:36-37`「an entry is postable when it carries a receipt, and **a zero entry never is**」）
要求的是另一支。没被覆盖的那一支是 `:44`（`ZeroArm::Refuse => false`）。
mechanism — `postable`（`:38-46`）先做收据检查（`:39-41`），再把零金额的决定**委托**给 `zero_arm()`
（`:42` 的 `match self.zero_arm()`，`:43` Post⇒true、`:44` Refuse⇒false）。`zero_arm` 恒返回 `Post`
⇒ 带凭据的零金额分录被判可入账，与 `:36-37` 的契约相反；而 `Refuse` 这个变体在本树里**没有任何构造点**
（`enum ZeroArm` 是私有的，`:5-10`，树外也拼不出），所以 `:44` 那一臂按构造不可达——
这正是"没有测试覆盖它"的结构性原因，而不是"测试恰好漏了"。
evidence — ① `cargo test --offline`（root=h4-one-file-many-algorithms，exit **0**）：`2 passed; 0 failed`
（`tests/postable.rs` 只用 `Entry::new("a", 5, …)`，从不构造零金额 ⇒ 绿得并不说明那条规则对）。
② `check {face:default, census:true}` → `verdict passed (cargo exit 0)`，`branch-level: 1 constructively unreachable arm(s)`
＋ `no construction of `ZeroArm::Refuse` is spelled in this tree, so the arm matching it in `postable` at
src/model/entry.rs:44 can never be entered (the enum is private, …)`，并把 `:34/:36/:37` 的契约原文引在上面。
③ `digest {file:src/model/entry.rs}` → 5 个函数：`new`(25-27)、`zero_arm`(30-32)、`postable`(38-46)、
`signed`(53-55)、`normalized_account`(60-62)（后两个无人调用：`0 caller(s)`），即"一个文件多套算法"的形状。
④ 源码逐行：`:31` 返回 `Post`、`:43/:44` 的映射本身是对的（写反的是**返回值**，不是 match）。
counter-proof — 若写反的其实是 `:43-44` 的映射或收据检查，`:31` 就该返回 `Refuse`、`Refuse` 臂也该有构造点——
census 说的是相反（`Refuse` 无构造点、`:44` 不可达）；若"只是没测到"而非"按构造不可达"，
census 的判据（私有枚举 + 本树无构造）就不该成立——它成立。构造证据（/tmp 副本，不碰本树）：
加一条断言"带凭据的零金额分录不可入账"的新测试文件后，**原样的树红**（`assertion failed`，exit 非 0），
只把 `:31` 的 `ZeroArm::Post` 改成 `ZeroArm::Refuse` 后**同一条测试绿**（`1 passed`）——
两边的树内套件都仍然全绿，坐实"没有任何测试覆盖这一支"。
fix — 最小改动：`src/model/entry.rs:31` 的 `ZeroArm::Post` → `ZeroArm::Refuse`（一个标识符）。
未改动：`tests/postable.rs`（不许改测试；绿不是判据）、`ZeroArm` 的两个臂与 `:43/:44` 的映射（它们是对的）、
`src/model/model.rs`、`src/lib.rs`。本树**只读**，以上均未落盘；探针只在 /tmp 副本上做。
调用清单（`target/round9/logs/h4-one-file-many-algorithms.jsonl`）：check(default, census) · digest(entry.rs)。
总调用：成功 2 / 被拒 0
