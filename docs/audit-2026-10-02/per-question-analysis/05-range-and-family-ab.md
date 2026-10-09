# 第九轮 · 逐题对话分析 · 范围型 `h1` 与五族 `fa` `fb`

> 口径、已核定读数与质量判定全部取自本轮任务书 `per-question-analysis/README.md`（下称"任务书"），本文只做**对话层**。
> 引用写法：`brief/<arm>/<id>.md ①n` = 精简版对话第 n 条仪器调用；`dialogues/<arm>/<id>.md:<line>` = 完整版行号；
> `dialogues/ours/_apparatus.md:<line>` / `dialogues/cg/_shared.md:<line>` = 装置桶与共享桶；
> `target/round9/logs/<id>.jsonl` 与 `target/probe-cg26/logs/<id>.txt` = **原始逐题日志**（本文凡引"全文"处均按原始日志核过，
> 因为精简版对 `h1` 的普查表做过截断：`…[工具返回：此处截断，全文 5654 字符…]`，`brief/ours/h1.md ①1`）。

## 0. 三题共同底数（后面每题不再重复）

| 题 | 我们：仪器 / 非仪器（可见） | codegraph：仪器 / nonbridge（日志内） | 最关键的一处不对称 |
| --- | --- | --- | --- |
| `h1` | 4 / 10（3 `read` + 3 `edit` + 1 `bash` + 1 `write` + 答案的 1 `read` + 1 `edit`） | 22（含 2 `init`）/ 3（3 次 cargo） | 一次 `check --census true` 给出**整张 16 行普查表**；cg 那 7 条清单要 **14 次真实查询**（+2 `init` +1 `files`）去凑 |
| `fa` | 10（9 成功 / 1 被拒）/ 24 | 19（含 2 `init`）/ 5 | 新面**由工具生成**（`apply` 直接给出可落盘 diff）vs **手抄兄弟源码**（模板来自 `node <file>`） |
| `fb` | 7 / 1（只有 1 次 `write` 记在该题名下） | 6（含 2 `init`）/ 8 | `deepen` 预览自带**另一种读法的定价**（`alternative` 段）vs 无等价物，cg 靠 g3 前例试错 |

三条跨题事实（都逐字有出处）：

1. **三题的"发现"都不来自仪器。** `h1` 的三处缺陷点位：我们臂先有一条非仪器的 `diff -ru --exclude=.git --exclude=target s1/crates h1/crates`（`dialogues/ours/_apparatus.md:1289`，同一条命令同时覆盖 h1/fd/fe，输出恰是 `audit.rs` / `query.rs` / `render.rs` 三个 hunk，`:1295/:1307/:1319`）；cg 臂自己写「maybe I can diff it against s1's copy to see the differences quickly! `diff -r trees/s1 trees/h1`」（`dialogues/cg/h1.md:255`），接着「Let me do: diff -r (nonbridge, cheap) + then read the differing files via codegraph」（同文件 `:257`），答案 ⑤ 也记「非 codegraph：`diff -r trees/s1 trees/h1`（定位三处差异）」（`brief/cg/h1.md` 交付答案 ⑤）。⇒ **两侧都是"diff 定位、仪器取证"**，仪器调用数不衡量"发现"。
2. **写动作在两臂的可见度不同（以 `fa` 为例）。** 我们臂的 6 次 `edit` / 1 次 `write` 逐条带原文记在 `fa` 自己的对话里（`dialogues/ours/fa.md` 二·step 104/115/119/121）；cg 臂的同类动作不在题目志里——它自己在共享桶里点名「my fa edits are known: button.rs layer / lib.rs const / registry.rs 4 pins / new toggle.rs」（`dialogues/cg/_shared.md:1126`）。所以"仪器 10 vs 19 / 步 24 vs 7"**不能**读成"一侧干得多"。
3. **cg 臂大量把仪器当文件读取器。** `fa` 的 19 次里 11 次是 `node <文件>` 的整文件 dump（`tests/registry.rs` 766 行 ×4、`tests/static_plan_allocations.rs` 310 行 ×2、`src/lib.rs` 86 行、`src/control/control.rs` 40 行、`src/control/object/button/button.rs` 41 行、`slider.rs` 28 行、`toggle.rs` 27 行），只有 2 次是真结构查询（`callers Toggle::paint` → `ℹ Symbol "Toggle::paint" not found — did you mean: paint, ControlFrame?`、`callers ControlHandle` → `ℹ No callers found for "ControlHandle"`），**两次都没有产出任何可用于本题的边**。我们臂的 10 次全部是结构/状态查询，读文件走 harness `read`/`bash`（不计入仪器）。

---

### `h1` · 范围型：先给可核对的全树总账、逐栏处置

**我们臂的路径**：**4** 次仪器调用，序列 = `check {face:default, census:true}` → `check {face:audit}` →（修 3 处）→ `check {face:default, census:true}` → `check {face:audit}`。

- ①1 = **总账本体**。一次返回同时给了门状态、红名单、`why` 行、`next` 行和整张普查表（原始日志 `h1.jsonl` 第 1 条全文 5654 字符；表体 16 行 = 1 行表头 + 12 行数据 + 3 行 `not covered` 自陈）。逐字（`target/round9/logs/h1.jsonl` 第 1 条）：
  - `verdict  failed (cargo exit 101)` / `tree   12 rust file(s), 18 function(s)` / `result test result: FAILED. 0 passed; 3 failed;` / `failed the_minimum_amount_filter_keeps_the_entries_at_or_above_it` / `why    …panicked at crates/report/tests/report.rs:27:5: assertion \`left == right\` failed: the zero floor keeps the two non-negative…`
  - `next   the \`why\` lines above are the failing assertion's own words: a short, stable phrase from one is a string literal in this tree, so \`search {literal: "the zero floor keeps the two"}\` finds the line that produced it. …`（**工具主动给了源行定位路径**，我们臂没用它，见下）
  - `census: 1 named numeric constant(s); a static fact about this tree, not a verdict`
  - `respelled 1000 is declared as \`SMALL_LIMIT\` (crates/core/src/limits.rs:10) and written again at crates/report/src/buckets.rs:10`
  - `unreferenced \`SMALL_LIMIT\` (crates/core/src/limits.rs:10) is not read anywhere outside tests`
  - `entry plan: 0 \`cut(\` site(s) and 0 \`graft(\` site(s) across this tree's sources (a static count; which of them this application ships is the plan's own business)`
  - `decl   no test names \`audit_unused\` (crates/core/src/audit.rs:14)`（同形 4 行：`postable`/`bucket_name`/`write_count`）
  - `declarations: 4 production \`pub fn\` name(s) appear in no test file (a text-level count: a test that reaches one without writing its name does not count here)`
  - `test-reachable: 2 of 12 production function(s) no test can reach (18 function(s) indexed in this tree; a static walk from the test files along the same name-in-call-list rule the orphan view uses)`
  - `by directory: crates/core 1 of 7 · crates/report 1 of 5`
  - `fn     no test reaches \`write_count\` (crates/report/src/render.rs:17)`
  - `branch-level: 0 constructively unreachable arm(s) in this tree (0 \`false\` guard(s), 0 never-constructed variant(s); a static read of the source text, not a coverage measurement)`
  - `not covered by the test-reachability column: dynamic dispatch, function pointers, FFI, macro expansion, and reach only through a trait method or a closure are invisible…` / `not covered by the branch-level column: a condition whose value depends on data — a field, a parameter, a comparison, a \`match\` over a value — is not judged at all…` / `not covered: this census reads exactly what the columns above name; string constants, structural duplication, runtime behaviour and claims written in prose are outside it…`
- ①2 `check {face:audit}`：给出默认面**看不见**的那条红（`failed the_audit_counts_the_non_zero_entries` / `why    …panicked at crates/core/tests/audit.rs:24:5: assertion \`left == right\` failed: the zero entry is not counted`，`brief/ours/h1.md ①2`）。同一返回里普查表被裁到 5 行，并自陈：`… truncated: 5 of 14 census rows withheld at the limit of 5; pass \`census: true\` for the whole table (every column head is already here with its count)`——**"要整表就再传一次 `census:true`"是工具自己说的**。
- ①3 = 修复后的同一张表，**唯一变化就是证据**：`test-reachable: 1 of 12`、`by directory: crates/core 1 of 7`、`fn     no test reaches \`write_count\`` **整行消失**；而 `decl   no test names \`write_count\` (crates/report/src/render.rs:18)` **仍在**（行号 17→18），因为它是"文本层计数"。答案 ④ 把这次"行消失"当作反证逐字写上：「`fn no test reaches write_count` 一行**消失** ⇒ 修复把那条静态上的断链接了回去（不是只让测试闭嘴）」（`brief/ours/h1.md` 交付答案 ④）。
- **决定性的一步**：对"总账"是 ①1（一次调用给出全部栏目与分母）；对"三处缺陷的行号"**不是仪器**——是 3 次 harness `read`（`dialogues/ours/h1.md` 二·step 58，逐字参数 `{"file_path":"…/h1/crates/core/src/audit.rs","limit":10,"offset":4}` / `query.rs` `limit 12 offset 20` / `render.rs` `limit 16 offset 5`），而这 3 个文件正是装置桶那条 `diff -ru … s1/crates h1/crates` 的 3 个 hunk。**桥在这道题里给的是"栏 + 计数 + 门"，不是源码行。**

**codegraph 的路径**：**22** 次仪器调用（日志 `h1.txt` 25 条 = 22 codegraph + 3 nonbridge cargo；22 含 2 次 `init` ⇒ 20 次真实查询）。序列 =
`init` → `files` → `init` → `node query.rs` → `node render.rs` → `node audit.rs` → `node tests/report.rs` → `callers ×8`（`Filter::matches`、`bucket_name`、`bucket_lines`、`write_count`、`audit_count`、`audit_unused`、`Entry::postable`、`Store::post`）→ `node buckets.rs` → `node limits.rs` →（修 4 处）→ `node query.rs` → `node render.rs` → `node audit.rs` → `node buckets.rs` → `node render.rs`（`target/probe-cg26/logs/h1.txt`，`=== CMD` 共 25 条）。

- 决定性的一步 = `node <文件>`：它给的是一份**行号化的源码**，于是答案能写「每一处都先由工具钉到行号（改前 `node` 原文在日志）」（`brief/cg/h1.md` 交付答案 ④）。逐字样本：
  `**crates/report/src/query.rs** — 34 lines, 2 symbols · used by 1 file: crates/report/tests/report.rs` + `27\t            if entry.amount > min {`（`brief/cg/h1.md ①4`）；
  `**crates/report/src/render.rs** — 20 lines, 2 symbols · used by 1 file: crates/report/tests/report.rs`，正文里 `:9-13` 只有表头与循环、`write_count` 定义在 `:17`（`brief/cg/h1.md ①5`）。
- 结构性那半由 `callers` 出：`Callers of "write_count" (0):` / `ℹ No callers found for "write_count"`（`brief/cg/h1.md ①11`）；`Callers of "audit_unused" (0):`（①13）；`Callers of "Filter::matches" (1):` → 唯一调用者是那条红测试（①8）。
- 一个自陈的工具缺口被它自己记下：「`callers write_count` = 0 (before fix) ✓ … `callers audit_count` = 1 (audit_unused); the audit test's qualified-path call is presumably missed again … I'll note the gap.」（`brief/cg/h1.md ①6` 当时推理）

**逐条清单 → 能否指回某一次工具返回**（这是本题的核心对照）：

| 答案里的条目（逐字） | 我们臂：指回哪一次返回的哪一行 | cg 臂：指回哪一次调用 |
| --- | --- | --- |
| `named numeric constant` 栏：`respelled 1000 … written again at crates/report/src/buckets.rs:10`、`unreferenced SMALL_LIMIT …` | ①1 两行 `respelled` / `unreferenced`，**一次调用两行** | 需 `node buckets.rs` + `node limits.rs` 两次，且要自己把两处对上：答案 D 行逐字「`crates/report/src/buckets.rs:10` 把 `1000` 又拼了一遍，而 `crates/core/src/limits.rs:7-8` 明说"报表的分桶从这个常量读它" → **已改**读 `ledger_core::limits::SMALL_LIMIT`（行为不变，`SMALL_LIMIT` 就是 1000）」 |
| `entry plan: 0 cut( / 0 graft(` | ①1 一行，**且带处置**（答案：`这棵树根本没有嫁接入口 ⇒ 作用域走保守回退，无面被剪`） | **无对应物**：cg 全程没有"入口计划/缺口"这一栏，答案 A–G 里没有这一行 |
| `declarations: 4 production pub fn name(s) appear in no test file` | ①1 一行 + 4 行 `decl` | 无对应物；最接近的是每条 `callers` 的 0/1 计数 |
| `test-reachable: 2 of 12 … (18 function(s) indexed)` + `by directory: crates/core 1 of 7 · crates/report 1 of 5` | ①1 三行（含**分母与逐目录**） | 无对应物；`node` 头给的是每文件 `N symbols · used by M file`，没有全树分母 |
| `branch-level: 0`（+ 栏自陈不判数据相关条件） | ①1 两行（0 也要处置，答案写成「该栏的 0 只说明『没有常量假守卫、没有未构造的私有枚举变体』」） | 无对应物 |
| `not covered（各栏自陈）` | ①1 的三行 `not covered …`，答案逐条抄成一行 | 答案 G 行（`判不了`）是**agent 自己写的**：`经 trait 对象/宏/字符串驱动到达的调用落点；断言强度（tests/buckets.rs:17 只断言 lines.len() == 2，并不钉住桶名）` |
| `crates/report/src/query.rs:27 entry.amount > min` | **不是仪器**：harness `read`（step 58）+ 装置桶 diff | `node crates/report/src/query.rs`（①4） |
| `crates/report/src/render.rs:8-13` 缺 `write_count(&mut text, entries);` | **不是仪器**：同上；辅助证据是①1 的 `fn no test reaches write_count` | `node crates/report/src/render.rs`（①5）+ `callers write_count`（①11） |
| `crates/core/src/audit.rs:9 == 0` | **不是仪器**：同上；发现它的是①2 的 audit 面 | `node crates/core/src/audit.rs`（①6） |
| 间接可达性一行（`postable`/`bucket_name` 经 `Store::post`/`bucket_lines` 间接到达） | ①1 只给了 `decl   no test names \`postable\``，**这一句在该题日志内指不回**（是 agent 从源码推的；工具自陈 `a text-level count: a test that reaches one without writing its name does not count here` 只是提醒了要另找证据） | F 行对应的 4 次 `callers`（`bucket_name`/`bucket_lines`/`Entry::postable`/`Store::post`）**可以**指回 |
| G 行的 `tests/buckets.rs:17` | 无 | **在该题日志内指不回**：`h1.txt` 全程没有读 `crates/report/tests/buckets.rs`（应来自别的题的上下文或 harness 读） |

⇒ 两条结论：(a) **我们臂的"栏"全部一次到手**，只有"间接可达"这一句没有工具出处；(b) **cg 臂的"源码行"全部一次到手**，但**"栏"（0 值栏、分母、逐目录、每栏盲区）完全没有对应物**，G 行还得自己写，其中 `tests/buckets.rs:17` 在该题日志内无出处。

**两侧差异与归因**（都落到可指出的信息差/流程差）：

1. **信息差：栏目化 vs 逐符号化。** 我们臂的 12 行数据里有 5 行是"带分母的整树统计"（`2 of 12`、`by directory … 1 of 7`、`4 production pub fn`、`0 constructible arm(s)`、`0 cut( site(s)`）；cg 的输出单位始终是"一个符号/一个文件"（`used by 1 file`、`Callers of X (N)`、`N symbols`）。要凑出等价的表，cg 只能对每个符号各发一次 `callers`——这正是它 8 次 `callers` 的来源。
2. **信息差：0 值栏与"看不见什么"。** 总账题的价值一半在"哪些栏是 0、为什么 0 不等于没事"。我们臂的 `entry plan: 0 …`（+ `branch-level: 0` + 三条 `not covered`）是**同一块返回里的现成句子**；cg 侧没有任何 0 值栏（它不索引宏调用点、不判分支、不建"入口计划"概念），答案 G 行只能靠自己列盲区。
3. **流程差：调用数 4 vs 22（真实查询 4 vs 20）。** cg 每条清单条目 ≈ 1–2 次调用（A←`node query.rs`+`callers Filter::matches`；B←`node render.rs`+`callers write_count`；D←`node buckets.rs`+`node limits.rs`；E←`callers audit_unused`+`callers audit_count`；F←4 次 `callers`），另加 5 次**改后重读**（`render.rs` 一共读了 3 次——第 2 次读到的正是它用 `sed` 把 `write_count` 插进了 `for` 循环里的那一版，见 `dialogues/cg/h1.md:964` 的「Problem! The `sed -i '9a\…'` insertion for h1's render.rs went to the WRONG place」）。
4. **共同的流程差（对两侧都不利）：源行定位走了非仪器捷径。** 两侧都先 `diff` 出一棵参照树（s1），把"改哪三行"缩成 3 个文件；只看仪器返回，我们臂拿到的是 `crates/report/tests/report.rs:27/38/48`（**测试**行号）与 `crates/core/tests/audit.rs:24`，cg 拿到的是同一批测试行号 + 源码全文。

**代价**（任务书 §3）：仪器 **4 vs 22**（含 2 次 `init`）· 步 **5 vs 4** · 输出 **3,267 vs 5,076** tok · 推理 **6,644 vs 8,364** 字符 · 累计上下文 **1,182,080 vs 964,736**。附注：我们臂的"步 5"与我在完整版里能数到的 4 次仪器 + 10 次非仪器调用不符，属状态机归属（见"没能判定" §1）。

**质量**（任务书 §4）：我们 **`h1` 5/5 命中** ✓；它 **`h1` 完全命中** ✓（§4 的 24/26 里被判"部分"的是 `s3`/`g1`，不含 `h1`）。⇒ 本题**两种路径都拿到满分**，差异全在过程代价与证据链的形状。

**引导含义**（具体到可实施）：

1. **给普查块补一行 `next`，把 `decl` 行指向 `callgraph`。** 现在 `check` 的返回里**只有测试失败块有 `next` 行**，普查块以 `not covered: …` 收尾、没有任何后续动作提示；于是我们臂唯一一条没有工具出处的判断，恰好落在 `decl`（文本层"没人写下这个名字"）与真实的"谁能到达"之间——答案那行「`postable`/`bucket_name` 经 `Store::post`/`bucket_lines` 间接到达」（`brief/ours/h1.md` 交付答案 ②）是 agent 自己补的。可实施：`decl` 行尾加 `next callgraph {name} for reach` 或让 `census` 直接给 `declared-but-unreached: N`。依据：`h1.jsonl` 第 1 条里 `decl` 行与 `test-reachable` 行**同时存在且互相打架**（一个有 4 个名字、一个只有 2 个函数），工具的括号解释只说"是文本层计数"，没说下一步该问谁。
2. **"0 值栏"要在总账型题目的交付模板里占一行。** 我们臂答案最容易被漏掉的两行——`entry plan: 0 \`cut(\`` 的处置（"这棵树根本没有嫁接入口 ⇒ 没有『新增面被剪掉』这一路"）与 `branch-level: 0` 的处置——在 cg 侧**结构上不可能出现**。若希望两条臂可比，rubric 应要求"每栏一行处置，含 0 栏"；工具侧对应的是把 census 的表头固定成一份**可枚举的栏清单**（现在表头只写 `1 named numeric constant(s)`，其余栏名散在行里）。
3. **`census` 的"全表/裁到 5 行"开关应当默认打开或改为一句话摘要 + 计数。** 我们臂的第一次调用带了 `census:true`（说明它知道要全表），第二次没带就被裁到 5 行并收到 `… truncated: 5 of 14 census rows withheld`。可实施：把 `census:true` 的效果改成"表头 + 每栏计数"常显（现在 `5 of 14` 那句已经证明工具知道总栏数是 14），把明细留给 `--list check`。
4. **这条差异里有一半工具替不了，要写进口径。** "三处缺陷的行号"两侧都由跨树 `diff` 给出；census 的设计目标本来就是 `a static fact about this tree, not a verdict`（①1 表头逐字）。⇒ 若要减少非仪器步，正确做法不是改 census，而是在答案模板里把"源码行号"的举证类别写成"harness 读/diff"（本轮 `per-question-cost.md` 已单列 nonbridge，方向一致）。

---

### `fa` · 五族：「我要加一个新对象，和现有的差不多。」

**我们臂的路径**：**10** 次仪器调用（9 成功 / 1 被拒），序列 =
`registry` → `consistency {parent:root/control}` → `apply {action:add, …}`（预览）→ `apply {…, apply:true}` → `registry` → `consistency {parent, specimen}`（**被拒**）→ `explain {node:root/control/dial}` → `check {face:default}` → `consistency {parent}` → `consistency {specimen}`（`target/round9/logs/fa.jsonl`，10 条逐字一致）。

- ①1 `registry`：`faces 3` + 三行（`root/control Control control/control.rs fb97ddd5…` / `button … ff1c57d9…` / `slider … bdb4427c…`）+ `next   explain {node} for one face's contract, check {face} for whether it builds`；同一行还自陈树是现推的：`tree derived now (no published records at …/target/xirang/out; cannot read …/source_scope.tsv: No such file or directory (os error 2))`。
- ①2 `consistency {parent:root/control}`：`family root/control · member xirang-example-control-button · 2 member(s)` / `button 0 call(s): none` / `slider 0 call(s): none` / `outliers: 0 of 2`，并自陈 `not covered by this comparison: … a call written inside a macro body is not read as a call (the kernel's rule), so an object whose whole body is one macro invocation reads as calling nothing`——**这行正好说明这道题的"相似"判据它给不了**（两个兄弟的面体都只有一个宏调用）。
- ①3 `apply`（预览）= **模板本体**。返回里直接给出将写入的文件、声明行号、`faces 4` 与**全文 diff**（`+` 行按原始换行逐字抄录，`brief/ours/fa.md ①3`）：
  ```
  +// generated-by=XiRang
  +pub struct Dial;
  +crate::control_object! {
  +    kind: Dial,
  +    exports: ["control.render"],
  +    parent: crate::control::NODE_ID,
  +    handle_traits: ["ControlHandle"],
  +    handle_contracts: [crate::control::ControlHandle],
  +    flow: crate::FlowContract::new(crate::ContractId::new("control.render.v1"), 1, "ControlInput", "ControlFrame"),
  +}
  ```
  同一次返回里同时给了**三句相互拉扯的话**：
  `preview effect: created \`control/object/dial/dial.rs\` under parent fb97ddd5…` + `faces 4`（新面已在派生树里）；
  `consequences (static, text-level): 0 in-tree test line(s) name this face`（**新面名字在测试里出现 0 次**）；
  `entry plan: 13 \`cut(\` and 12 \`graft(\` site(s); this face's name appears at none of them — whether the application ships it is the plan's own business`（**把"会不会被发布"推回给 plan**）；
  `not covered: this lists test lines that spell the face's name; a test that counts faces without naming it, or reaches it through another spelling, does not appear here — run the suite before believing either list`（**自己承认计数钉子看不见**）。
- ①4 `apply {apply:true}`：`applied …/dial.rs` / `created \`control/object/dial/dial.rs\` under parent fb97ddd5…`；①5 `registry` → `faces 4`，新行 `root/control/dial Dial control/object/dial/dial.rs e1070833a032e106f9cdd0eff2149bc2`。
- ①6 被拒的那次逐字：`consistency takes \`parent\` or \`specimen\`, not both: \`parent\` compares siblings against their own majority, while \`specimen\` derives the parent from the anchor and compares them against the shape the ledger certifies — accepted shape: {"specimen":"<anchor>","root":"<path>"}`——**这是全轮唯一一次被拒**（`dialogue` 里 agent 自己认领：「The consistency call with both `parent` and `specimen` was refused (exit 1) — that's my call error (usage), recorded in the log as exit 1」，`brief/ours/fa.md ①4` 推理）。
- ①7 `explain {node:root/control/dial}`：`build stale (run \`xirang check\`)` / `scope unknown (no source_scope.tsv; run \`xirang check\`)` / `pruning unknown (no pruning_manifest.tsv; run \`xirang check\`)`——**工具明确把"这个面在不在作用域里"指向 `check`**。
- ①8 `check {face:default}`：`verdict  passed (cargo exit 0)` / `tree   14 rust file(s), 54 function(s)` / `elapsed 24612 ms` / `result test result: ok. 25 passed; 0 failed;`；①9 `consistency {parent}`：`3 member(s)` / `dial 0 call(s): none` / `outliers: 0 of 3`；①10 `consistency {specimen}`：`no ledger entry names \`root/control/dial\` in …/trees/fa — the ledger holds 0 entry(ies), and a specimen comparison is against the entry in force (an adoption is a lease: the newest line wins). \`adopted\` lists the anchors.`

- **24 步在干什么**（完整版 `dialogues/ours/fa.md` 二 逐条可数，共 **24 次非仪器调用**）：**10 次 `bash`**（其中 **4 次是 `cargo test --offline`**：step 93、105、116、122；另 6 次是 `grep -n` / `sed -n` 读 `tests/*.rs` 的钉子，如 `{"command": "cd …/trees/fa && grep -n \"outline()\\|rows.len\\|paths.len\\|assert_eq!(faces\\|EXPECTED\\|root/control\" tests/*.rs examples/*.rs | head -40"}`）、**7 次 `read`**（step 100×2、114、118×2、120，外加 step 174 读自己的答案）、**6 次 `edit`**（dial.rs 1 次、`src/lib.rs` 2 次、`tests/registry.rs` 3 次）、**1 次 `write`**（答案）。即：**这 24 步＝"找会动的钉子 + 手改 4 个文件 + 跑 4 次 cargo + 写答案"**，其中只有 5 步（step 91/92/123/124/168）是桥调用。
- **这些步是否本来该由工具承担**：
  - **该承担的第一件：哪三处计数钉子会动。** 工具知道这件事的形状——同一族 `fb` 的 `deepen` 预览里就有 `alternative (the other reading of \`make it deeper\`): hang another face **under** this one … moves the factory-shape assertions that count faces, scope rows and slots`（`target/round9/logs/fb.jsonl` 第 3 条），而 `add` 的 `consequences` 只按**新面自己的名字**扫（`0 in-tree test line(s) name this face`），于是三条**计数**钉子（`tests/registry.rs:54` 树行表、`:283` `plan.faces().len()`、`:368` `selected.len()`）一条也照不出来。实测代价：step 116 的 cargo 输出三条红（`left: [4 行]` vs `right: [3 行]`、`left: 4 right: 3`、`left: 3 right: 2`，`dialogues/ours/fa.md` 二·step 116），随后 4 次 read/sed + 3 次 edit 才把它们改对。
  - **该承担的第二件：发布与否。** 工具的同一返回里既写 `faces 4`（新面已在派生树里）又写 `whether the application ships it is the plan's own business`（①3）。agent 先按最自然的读法把 `pub use crate::control::object::dial::Dial;` 当成"那次提及"（它甚至在注释里写死这句话，逐字见 step 114 的 read 结果：`60: /// 这条重导出就是那次提及，同时让新面与两个兄弟一样在宿主的公开表面上可寻址。`），step 105 才被 `error[E0432]: unresolved import \`crate::control::object::dial\`` 打回（`dialogues/ours/fa.md` 二·step 105），step 114 才推出 `So to name the new face from the entry, the entry must *spell the path in code*`，step 115 改成 `shipped_object_nodes()`（答案 ②：`**在代码里拼出** \`…::button/slider/dial::NODE_ID\``）。
  - **该承担的第三件：重跑门禁的循环。** 桥有 `check`（①8，`elapsed 24612 ms`，日志落在树内 `…/trees/fa/target/xirang/out/check-default.log`），但 agent 的迭代循环用的是 **raw cargo + 共享 target**：4 次 `cargo test --offline` 每条都写 `CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/round9/.cargo-target/fa`（step 93/105/116/122）。⇒ 一旦"快"与"证据在仪器里"冲突，agent 会绕开仪器；这直接解释了 24 步里为什么有 4 次 bash cargo。
  - **不该由工具承担的两件**（要诚实记账）：`impl ControlHandle for Dial`（生成的面文件只给注册标记，实现要人写，答案 ② 自陈「按其形状补 `impl ControlHandle for Dial`（与 `slider.rs:10-14` 同形）」）；以及三条期望的**改写**（改判据本身，工具只该说"这三行会动"，不该替人改）。
  - **一个被 bash 意外救掉的坑**：`dial` 这个名字。step 87 的 grep 输出里已经有 `tests/registry.rs:187: assert_eq!(kind_at(&registry, "root/control/toggle"), "Toggle");`（`dialogues/ours/fa.md` 二·step 87 结果），所以 agent 选了 `Dial`；对照 cg 选了 `Toggle` 后吃了一次 `duplicate sibling registry name`（见下）。

**codegraph 的路径**：**19** 次仪器调用（`fa.txt` 24 条 = 19 codegraph + 5 nonbridge；19 含 2 次 `init`），序列 =
`init` → `files` → `init` → `files` → `node button.rs` → `node control.rs` → `node tests/static_plan_allocations.rs` → `node src/lib.rs` → `node tests/registry.rs` → `node slider.rs` → `node tests/registry.rs` ×3 → `node toggle.rs`（**未入索引**）→ `callers Toggle::paint`（未命中）→ `sync .` → `node toggle.rs` → `callers ControlHandle`（`target/probe-cg26/logs/fa.txt`）。

- **模板从哪来**：`node src/control/object/button/button.rs` 给出 `41 lines` 的逐字源码（`kind/exports/parent/handle_contracts/flow` 五件套，`brief/cg/fa.md ①5`），`node …/slider/slider.rs`（`28 lines, 2 symbols`）再对一遍；答案 ③ 逐字：「`slider.rs:16-27` 与 `button.rs:15-40` 是现成模板（kind/exports/parent/handle_contracts/flow 五件套）」。新文件是**手抄**的（cg 的写动作不在日志里，`dialogues/cg/_shared.md:1126` 自述 `new toggle.rs`）。
- **发布规则从哪来**：`node src/lib.rs`（`86 lines, 3 symbols · used by 2 files`）给出 `36-43` 的文档原文：`the build-time scope narrows to the subtrees these cuts name: a face nobody declared is not shipped by this application`（`fa.txt:618-620`）；`node tests/registry.rs`（`766 lines, 28 symbols`）给出 `385-391` 的注释：`The parent \`control\` face is not a slot, so it is not a scope root; it survives because a selected face needs it and because the entry reaches it.`（`fa.txt:1053-1058`）。答案 ③ 正是引这两处。
- **自证门绿**：**自己跑** `cargo test`（`fa.txt` cmd 4 基线、14、17、18 `--all-features`）+ 1 次 `cargo run --offline -q --example tree`（cmd 19，改后打印注册树，答案 ⑤ 据此写 `现在打印 4 行，含 \`root/control/toggle kind=Toggle source=control/object/toggle/toggle.rs\``）。先红**4** 条：`built_in_tree_has_the_expected_paths_and_derived_sources`（`tests/registry.rs:54`）、`static_plan_carries_faces_and_the_declared_graft`（`:283`）、`the_declared_slots_define_the_build_time_scope`（`:368`）、**外加** `parent_rule_admits_a_new_kind_that_satisfies_it`（`:186`），最后这条的报错逐字为 `message: "duplicate sibling registry name \`toggle\` (already owned by 4dbbd3f9… at control/object/toggle/toggle.rs)"`（`fa.txt` cmd 14）。
- **决定性的一步**：`node tests/registry.rs` 的 `:352-391` 那段 scope 测试原文——它把"作用域=入口声明的槽位 + 入口能到达"变成可执行判据，答案 ⑤ 的反证也据此写：「若只加文件而不在入口拼路径，`tree` 仍是 3 行（未声明的面不发布）——那就等于没加」（`brief/cg/fa.md` 交付答案 ⑤）。
- 三处仪器自陈的边界（都逐字）：新文件未入索引 `No indexed file matches "src/control/object/toggle/toggle.rs". Codegraph indexes source files; configs/docs it doesn't parse won't appear — Read those directly.`（①15）；`ℹ Symbol "Toggle::paint" not found — did you mean: paint, ControlFrame?`（①16）；`◆ Synced 3 changed files` / `● Added: 1, Modified: 2 — 43 nodes in 156ms` / `● Resolved 0 pending references (8 unresolved)`（①17）。

**两侧差异与归因**：

1. **模板：工具生成 vs 手抄。** 我们臂的新面是 `apply` 返回里的 `+` 行（含生成标记 `+// generated-by=XiRang`、双语文档行、`flow` 的 `FlowContract::new(...)` 全参），agent 只补了 trait 实现；cg 得把 41 行源码读下来、按 5 个字段抄一遍、再自己写 `tests/registry.rs` 的 4 处期望。**这不是"谁更聪明"**：`apply` 这条路径在 codegraph 里根本不存在（它的写命令只有 `init/sync`），代码生成的差就是工具的差。
2. **发布/作用域：同一个坑，两种提示。** 两侧都踩到了"新面必须在入口以表达式点名"。我们臂拿到的是一句**免责提示**（`whether the application ships it is the plan's own business`）+ 一个**误导性的 `faces 4`**，于是先写 `pub use` → E0432 → 改；cg 拿到的是**规则原文**（lib.rs 文档 + registry.rs 注释），一步到位、没有浪费轮次。⇒ 归因：**信息差在"提示 vs 原文"**，不在理解力；而且桥其实已经知道答案（`explain` 的 `scope unknown (no source_scope.tsv; run xirang check)` 说明它把这件事挂在 `check` 上）。
3. **计数钉子：两侧都只能靠红测试发现，两侧的工具都给不了。** 我们臂 3 条红、cg 臂 4 条红（多的是名字冲突）。差别只在事后成本：我们臂用 6 次 harness `read`/`grep` + 3 次 `edit` 修期望；cg 用 4 次 `node tests/registry.rs` 整文件 dump + 4 处 edit（编辑不可见）。
4. **名字冲突：两侧工具都不查重，但一侧被 shell 意外救掉。** cg 自己 dump 过 2 次的 `tests/registry.rs` 里就有 `180\t    sibling.kind = "Toggle".to_owned();` / `187\t    assert_eq!(kind_at(&registry, "root/control/toggle"), "Toggle");`（`fa.txt` cmd 10、12），它仍然选了 `Toggle`；我们臂的 `grep`（step 87）把同一行摆在了眼前，于是选 `Dial`。⇒ **信息差可修理**：工具已经会按名字扫"哪些测试行提到这个面"，只是扫的是**新面自己的名字**，不是**新面的 kind 是否已被树内文本占用**。

**代价**（任务书 §3）：仪器 **10 vs 19**（我们 9 成功 1 被拒）· 步 **24 vs 7** · 输出 **14,374 vs 6,822** tok · 推理 **28,676 vs 9,699** 字符 · 累计上下文 **5,558,016 vs 2,294,528**。
**质量**（任务书 §4）：我们 **`fa` 5/5 命中** ✓；它 **`fa` 完全命中** ✓。⇒ 与 `h1` 一样，**两臂同分**，差别落在步数与证据链。

**引导含义**（具体到可实施）：

1. **`apply add` 的 `consequences` 要加"会动的计数钉子"，按 file:line 列出。** 依据是它自己的 `not covered` 行（`a test that counts faces without naming it … does not appear here — run the suite before believing either list`，①3）与 `deepen` 的 `alternative` 段已经会算同类事（fb 预览逐字：`moves the factory-shape assertions that count faces, scope rows and slots`）。直接收益：省掉 3 次 read/sed + 1 次 cargo 红轮（step 116→119/121→122）。
2. **`apply add` 的预览应把"派生树"与"构建期作用域"并排给出，而不是 `faces 4` 加一句免责。** 现在 `registry`/`apply` 都把新面列进 `faces N`（①3、①5），而 `source_scope.tsv` 在 `check` 之前根本不存在（`cannot read …/source_scope.tsv: No such file or directory (os error 2)`，①1）；`explain` 只回 `scope unknown (…; run xirang check)`（①7）。可实施：预览里加一行 `scope  # selected 2 → 3 (with this face named from the entry)` 或直接复跑作用域扫描（它是纯文本扫描，成本极低）。依据：step 105 的 `error[E0432]: unresolved import \`crate::control::object::dial\``——这一轮红是本可以不出现在 24 步里的。
3. **`apply add` 应在预览里查一遍 `kind` / `registry_name` 是否与树内文本（测试的合成样例）相撞。** 依据：cg 的 `duplicate sibling registry name \`toggle\` (already owned by 4dbbd3f9… at control/object/toggle/toggle.rs)`（`fa.txt` cmd 14）；而"这个名字已出现在 `tests/registry.rs:180/:187`"是纯文本事实，工具的 `consequences` 扫描器本来就走遍全树测试行。
4. **`check` 要能复用/接受共享 `CARGO_TARGET_DIR`（或在返回里明说自己用哪个 target 目录）。** 依据：4 次 raw cargo 都写死了 `CARGO_TARGET_DIR=…/round9/.cargo-target/fa`（step 93/105/116/122），而 `check` 的日志落在树内 `target/xirang/out/check-default.log`、`elapsed 24612 ms`（①8）。迭代循环一旦被"慢"推出去，逐题归因与证据链就同时受损（这 4 步在任何仪器统计里都看不见）。
5. **被拒调用的文案已经足够好，不用改——但可以省一次往返。** `consistency takes \`parent\` or \`specimen\`, not both: … accepted shape: {"specimen":"<anchor>","root":"<path>"}`（①6）直接给了可用形状，agent 之后单传两次即成功（①9/①10）。这是本轮唯一一次被拒，成本 1 次调用，属"好文案"的正面样本。

---

### `fb` · 五族：「这个对象内部还不够。」

**我们臂的路径**：**7** 次仪器调用，序列 =
`registry` → `usages {node:root/control/button}` → `apply {action:deepen, …}`（预览）→ `apply {…, apply:true}` → `registry` → `usages {node:root/control/button}` → `check {face:default}`（`target/round9/logs/fb.jsonl`，7 条全部 `exit=0`）。

- ①1 `registry`：`faces 3` + 三行表（与 `fa` 改前逐字节同）。
- ①2 `usages`：`node ff1c57d9…` / `path root/control/button` / `parent fb97ddd5… root/control` / `children (0)` / `fields unreadable (this module was not generated by XiRang)` / `unreadable faces 3 (hand-written modules are not read back)`——**改前的"内部是空的"这一半由它证明**。
- ①3 `apply deepen`（预览）= **决定性的一步**。逐字给出四段：
  - `preview effect: deepened root/control/button: \`Button\` now holds \`ButtonParts\` with 1 part(s) ("label"); its declaration, its public path, its tree row and the factory-shape pins were not touched — \`check {face}\` is what shows those pins still hold`（**三个"没动"是明说的**）；
  - diff 全文（按原换行合并为一行；`+pub struct Button { parts: ButtonParts, }` / `+pub struct ButtonParts { pub label: String, }` / `+impl xirang_toolchain::runtime::PartsContract for ButtonParts { type Output = ButtonParts; const PROVIDED_PARTS: &'static [&'static str] = &["label"]; }` / `+impl Button { pub fn parts(&self) -> &ButtonParts { &self.parts } }`；**`crate::control_object! {}` 声明块逐字未动**）；
  - `alternative (the other reading of \`make it deeper\`): hang another face **under** this one — that needs \`needs_registry: true\` plus its own \`registration_rule\`, adds a row per child to the registration tree, adds a segment to this face's public module path, forces its graft slot to \`full graft\`, and moves the factory-shape assertions that count faces, scope rows and slots; the layer above was measured to leave all of them alone`；
  - `note   this face stops being a unit struct: anything that used the type as a value (\`let _ = Kind;\`, a literal pattern) has to be updated by hand — this action writes the face's own file and does not touch callers`。
- ①4 `apply … apply:true`：`applied …/button.rs` / `deepened root/control/button: …`；①5 `registry` 与改前**逐字节相同**；①6 `usages` 与改前逐字节相同；①7 `check {face:default}`：`verdict  passed (cargo exit 0)` / `tree   13 rust file(s), 53 function(s)` / `elapsed 23412 ms` / `result … 25 passed …`。
- 非仪器（该题名下只有 1 步，其余在共享桶）：`write` 答案（step 143）；答案 ⑤ 记 `shell（非桥调用）：\`sha256sum\` 快照 \`diff\`（只有 button.rs 变化）`，答案 ③ 的三条不变量（`registry` 逐字节同、`usages` 逐字节同、快照只差一行）全部由这些 shell 步支撑。

**codegraph 的路径**：**6** 次仪器调用（`fb.txt` 14 条 = 6 codegraph + 8 nonbridge；6 含 2 次 `init`），序列 =
`init`（`◆ Indexed 13 files` / `● 106 nodes, 164 edges in 557ms`）→ `files`（`Project Structure (13 files)`）→ `init`（`● 102 nodes, 158 edges in 558ms`）→ `node examples/tree.rs`（`11 lines, 1 symbol · no other indexed file depends on it`，内容是 `for row in control_button::outline() { println!("{row}"); }`）→ `node tests/ide_mirror.rs`（`54 lines, 2 symbols`）→ `node src/control/object/button/button.rs`（**改后**：`68 lines, 2 symbols · used by 1 file: src/control/control.rs`，正文里 `:17-18` 的宏多出 `preset: ButtonPreset,` / `parts: ButtonParts,` 两行，`:44-67` 是 `ButtonParts`/`ButtonPreset` 两个类型与两个 impl，`brief/cg/fb.md ①6`）。

- **词汇从哪来（本题最关键的信息差）**：codegraph 索引里只有题树内 13 个文件；"内部层"的词法在**本仓内核**里——答案 ③ 逐字引：`kernel/src/registry_core/declaration/contract.rs:26-33`（`PartsContract{ Output, PROVIDED_PARTS }`）、`:35-51`（`NoPreset/NoParts` 默认）、`toolchain/src/runtime/src/macros/face_objects.rs:38-39`（宏字段 `$(preset: $preset:ty,)? $(parts: $parts:ty,)?`）、`:62-88`、`contract.rs:72-77`（`assert_contract`）。这些路径**不在任何 `codegraph node` 的输出里**（全部来自 harness 读或别的题的上下文）。
- **踩的坑（逐字）**：「先写 `type Output = ButtonParts` 时，构建期生成的 `assert_contract::<host::__Preset, graft::__Parts>()` 直接编译失败：`expected ButtonParts, found ()`（生成的 `generated_lib.rs:154`，机器在 `/tmp/fb_after.out` 的日志里）。改成 `Output = ()` 后成立（嫁接侧 `NoParts::Output = ()`）。」（`brief/cg/fb.md` 交付答案 ③）
- **改了一条出厂钉子**：答案 ① 逐字「`tests/registry.rs:72` 钉着 `REGISTRATION.parts == "NoParts"`——对象内部是空的」；它把 `tests/registry.rs:64-91` 的期望改成"control/slider 省略 ⇒ 默认名；button 声明 ⇒ 作者写的名字"，并在答案里为这次改判据辩护：「我没有"把红改绿"：被改的那条测试改的是**合同期望**（button 不再省略），不是把失败断言删掉。」（④）
- **自证门绿**：自己跑 `cargo test --offline` ×5（`fb.txt` cmd 4/8/10/12/13，其中 cmd 12/13 带 `--all-features`），`cargo run -q --example tree` ×3（cmd 7/9/11）比对注册树字节（答案 ⑤：`cargo run --example tree` 前后**逐字节相同**（sha `571b70bc642f6aa1`））。

**两侧差异与归因**：

1. **改动落在哪一层，是工具一句话定的。** 我们臂的 `deepen` 只加 **Rust 类型层**（`Button { parts: ButtonParts }` + `PartsContract` + `parts()`），**宏声明块一字不动**，于是 `REGISTRATION.parts` 仍是默认、出厂钉子不动、只改 1 个文件、门 27 passed（答案 ②③）。cg 选择在宏里**声明** `preset:`/`parts:`（合同真的变了），于是必须改 `tests/registry.rs:64-91` 的期望、并为改判据写辩护。分歧点就在 `deepen` 预览那两行：`its declaration, its public path, its tree row and the factory-shape pins were not touched` 与 `alternative` 段的定价。**cg 没有等价物**：它的 6 次调用里没有一次问"另一种读法要付什么代价"，答案里也没有第二读法的讨论。
2. **词汇与合同约束：工具给出 vs 去读内核 + 编译一次。** 我们臂的生成结果里 `impl PartsContract` 是工具写好的（连 `type Output = ButtonParts` 都替你选了，且因为没声明 `preset:` 而不触发 `assert_contract` 的配对约束）；cg 得自己搞清 `PresetContract`/`PartsContract`/`Output` 的配对，并在 `Output` 上编译失败一次。
3. **"没动"的证明方式：桥的前后对照 vs 自己跑 example。** 我们臂用 `registry`/`usages` 前后逐字节相同（两次调用即可核对，①1 vs ①5、①2 vs ①6）；cg 只能 `cargo run --example tree` 打印后 `sha256`/`diff`（8 次 nonbridge 里的 3 次）。⇒ codegraph 的索引里**没有"注册树"这个对象**（它是构建期产物），所以这类"树上有没有动"的问题它答不了。
4. **证据链的完整度（要诚实记一笔）**：cg 的答案 ⑤ 写 `node src/control/object/button/button.rs`（改前/改后），但 `fb.txt` 里 `node button.rs` **只有 1 次**（末尾，改后 68 行那版）；"改前"的源码证据不在本题日志内（在 g3 的上下文里）。同样，答案 ⑤ 引的 `tests/registry.rs:38`/`:97` 也不是本题日志内的读。⇒ 单会话臂跨题共享上下文是合法的，但"每题每条能否指回本题某一次返回"在这题上对 cg 不成立。

**代价**（任务书 §3）：仪器 **7 vs 6** · 步 **2 vs 5** · 输出 **627 vs 8,188** tok · 推理 **381 vs 18,348** 字符 · 累计上下文 **492,800 vs 1,493,504**。
**质量**（任务书 §4）：我们 **`fb` 5/5 命中** ✓；它 **`fb` 完全命中** ✓（§4 逐类点名不含 `fb`）。
**口径提醒**：我们臂这题的三栏（步 2 / 输出 627 / 推理 381）与工作量严重不符——`fb` 的答案本身就比 381 字符长，且 5 次共享/装置步（`sha256sum` 快照、`diff`）都没记在该题名下。⇒ **本题只用"仪器 7 vs 6"作对比**。

**引导含义**（具体到可实施）：

1. **把 `alternative`（另一种读法的定价）从 `deepen` 推广到 `add` 与所有写命令，并把它做成机器可引用的一行。** 依据：`fb` 预览那段逐字定价（`adds a row per child to the registration tree, adds a segment to this face's public module path, forces its graft slot to \`full graft\`, and moves the factory-shape assertions that count faces, scope rows and slots`）直接决定了本题选哪条路；`fa` 那一轮同一段也被用来排除"在 button 下面再挂一个面"的读法（答案 ④）。cg 侧完全没有这段，两者在这道题上的过程差主要来自这里。
2. **写命令的返回应把"动了什么"写成三个显式布尔：`declaration touched` / `identity touched` / `pins touched`。** 现在这层信息散在 `preview effect` 的一句英文长句里（`its declaration, its public path, its tree row and the factory-shape pins were not touched`）。依据：cg 的路线**动了 `declaration`**（加 `preset:`/`parts:`）→ 必然动 `pins`，但它是在 `cargo test` 红了以后才知道，随后还要在答案里为"改期望"辩护（④）；如果预览给一行 `pins touched: tests/registry.rs:72 (REGISTRATION.parts == "NoParts")`，这一轮红与这段辩护都可省。
3. **"内部层"的合同词汇（`PartsContract`/`PresetContract` 的 `Output` 配对与生成断言）应出现在写工具的返回里。** 依据：cg 为此读了 `kernel/.../contract.rs` 与 `toolchain/.../face_objects.rs` 四处源码，并在 `assert_contract::<host::__Preset, graft::__Parts>()` 上编译失败一次（答案 ③）。可实施：`deepen`/`apply` 的 `note` 段增一行 `generated contract assertion: assert_contract::<ButtonPreset, ButtonParts>() — both Output types must agree`（工具生成这条断言，它本就知道）。
4. **对"这个对象够了没有"这类问题，值得在产品里加一条只在预览里出现的观察句。** 我们臂的答案 ③ 能一句话说清"为什么没动"（`apply` 预览自陈唯一形状变化 `this face stops being a unit struct`），cg 的答案只能给"树字节相同 + 测试绿"。依据：两边答案的 ③/⑤ 段落逐字对比即可看出，前者的不变量是**工具声明的**，后者是**自己构造的**。

---

## 没能判定 / 口径备注

1. **`h1`/`fb` 的"步、输出、推理、上下文"三栏不可用。** `fb` 我们臂记 `步 2 · 推理 381 字符`，但该题有 7 次仪器调用、答案文本本身超过 381 字符、另有共享桶里的 `sha256sum`/`diff`；`h1` 记 `步 5`，而完整版里我能逐条数出 4 次仪器 + 10 次非仪器调用（`dialogues/ours/h1.md` 二）。⇒ 只用**仪器调用数**（任务书 §3 说它"唯一可逐题归因"）。
2. **两臂"24 步 vs 7 步"的可比性有限。** 我们臂的 24 步把 `read`/`edit`/`write` 都算作可见步；cg 臂的同类动作不进日志（`dialogues/cg/_shared.md:1126` 自述有 `new toggle.rs` / `lib.rs const` / `registry.rs 4 pins` 三处写），它计入的 5 次 nonbridge 只是被 `cg.sh` 记录到的 cargo/tree 命令。⇒ 该比值**不能**读成工作量比。
3. **cg `fa` 答案 ⑤ 的自报与日志不符（不影响判定）。** 答案写「非 codegraph：`cargo test`×3、`cargo run --example tree`×1」，而 `fa.txt` 里的 nonbridge 是 5 条：`cmd 4`（基线 `fresh_fa.out`）、`cmd 14`、`cmd 17`、`cmd 18`（`--all-features`）四次 `cargo test` + `cmd 19` 一次 `example tree`；它当时的推理里记的是 `nonbridge 5 / 16,159 B`（`brief/cg/fa.md ①6` 推理）。按纪律 §6.3，以日志为准。
4. **两侧各有一条"在本題日志内指不回"的清单条目。** 我们 `h1`：`postable`/`bucket_name` 的**间接可达**（census 只给 `decl no test names …`）；cg `h1`：G 行的 `tests/buckets.rs:17`（`h1.txt` 没有读该文件）。我不判定它们是错——只标注证据链缺口。
5. **cg `fb` 的"改前"证据不在本题日志内。** `fb.txt` 里 `node src/control/object/button/button.rs` 只有 1 次、且是改后（68 行）；答案 ⑤ 的"（改前/改后）"与 ④ 引的 `tests/registry.rs:38/:97` 都依赖别的题的上下文。跨题共享上下文是单会话臂的正常形态，但"每题每条指回本题某次返回"在这题上对 cg 不成立。
6. **我们 `h1` 推理里出现过工具里没有的词。** `dialogues/ours/h1.md:561` 的逐字推理写 `\`audit_unused\` is genuinely unreachable (orphans=1)`，而 `h1.jsonl` 全文里没有 `orphans` 字样（我用 `grep -o 'orphans' h1.jsonl` 核过，0 命中）；同一事实工具的行文是 `fn     no test reaches \`audit_unused\` (crates/core/src/audit.rs:14)`。判为 agent 的口语化转写，不影响结论。
7. **`fa`/`fb` 里"相似/更内部"的判据本身有歧义（题面一句话题的固有性质）。** 两侧都各自解释了取哪种读法（我们靠 `deepen` 的 `alternative` 段，cg 靠 g3 前例），任务书 §4 对两臂均判命中，故本文不重做该判定；但"另一种读法要不要写进答案"是 rubric 层面的问题，不是工具层的。
