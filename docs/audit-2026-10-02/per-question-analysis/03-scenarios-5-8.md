# 第九轮 · 逐题对话层对比 · 只读情景 `s5` `s6` `s7` `s8`

口径照 `per-question-analysis/README.md`：读数与质量判定直接引 `docs/audit-2026-10-02/README.md` §三/§四，本文只做对话层。
两处共同的装置事实，下面每题都成立、不再重复：

1. **变体 `diff` 先于仪器调用**。两臂在开题前都用一条**非仪器**的 `bash diff` 把 s 树与基准树 s1 比对过：
   我们臂 `for x in s2 s5 s6 s7 s8; do … diff -ru --exclude=.git --exclude=target s1 $x`（`dialogues/ours/_apparatus.md:1067`，step 31），
   输出里已经有 s8 的 hunk（`-    write_count(&mut text, entries);` 被移到循环之后；原文见 ours 会话 `seq 242` 的 tool/result）；
   cg 臂 `for v in s2 s3 s4 s5 s6 s7 s8; do … diff -r --exclude=.codegraph --exclude=target s1 $v`（`dialogues/cg/_apparatus.md:1414`，s8 hunk 起于 `:1449`），
   紧随的推理逐题写出了注入（`- s8: write_count moved after the entry loop → the count line is printed at the END, not before the entries.`，`cg/_apparatus.md:1565`）。
   ⇒ 对 `s5`–`s8`，**仪器调用不是"发现"缺陷的那一步**，而是把已有先验变成可引用证据的那一步。下文"决定性的一步"按此口径写：指**哪一次仪器返回让结论在证据上成立、或让两种读法被区分开**。
2. **cg 的三栏在本题上是 0**。cg 把一题的若干 `codegraph` 调用塞进同一条 bash 批处理，逐题日志 5–7 条，但"步/输出/推理/上下文"记在共享步上，故 brief 头部是 `步 0 · 输出 0 tok · 推理 0 字符 · 累计上下文 0`（`brief/cg/s5.md` 头部）。它配到的"当时推理"是收尾写 SETUP.md 那段（`brief/cg/s6.md ①2`：`All checks pass:`），**不是**促成该调用的推理；促成它的推理在会话 `seq 203/217`，见下。
3. **这四题里两侧各自出现的"行"长什么样**（后面逐题只引其中与结论有关的那几行）：
   - 我们桥：信封行 `matches 1`、`evidence nichlink.callgraph {"function":"render"} → exit 0`（`brief/ours/s8.md ①2`），内容行 `  callers (2): …`、`  callees: from, push_str, write_count -> crates/report/src/render.rs:18`、`  also here: write_count -> crates/report/src/render.rs:18`、`  tests: crates/report/tests/report.rs`；`check` 侧另有 `failed …` / `why    …` / `next   …` / `census:` / `  not covered: …`。
   - codegraph：`**crates/report/src/render.rs** — 21 lines, 2 symbols · used by 1 file: …` 后接**带行号源码**（`node …`），边查询则是 `Callers of "write_count" (1):` / `function    render` / `  crates/report/src/render.rs:7`；另有 `files` 的树状清单。

---

### `s5` · 渲染缺了什么

**我们臂的路径**：3 次仪器调用，序列 = `check {face:default}` → `callgraph {function:render}` → `read {path:crates/report/src/render.rs, whole}`。
- ①1 `check`：`verdict  failed (cargo exit 101)`，三条红逐条点名 `failed the_minimum_amount_filter_keeps_the_entries_at_or_above_it` / `failed the_report_prints_its_count_before_the_entries` / `failed the_report_names_its_count_and_its_entries`；渲染那条的 `why` 行是失败断言自己的话：`panicked at crates/report/tests/report.rs:38:41: the count line is printed`（`brief/ours/s5.md ①1`）。
- ①2 `callgraph`：**决定性的一步**——"被调清单里没有它"与"本文件里它还在"同屏：
  `  callees: from, push_str` ／ `  also here: write_count -> crates/report/src/render.rs:17`；
  同一次返回还给了 ``  callers (2): crates/report/tests/report.rs::the_report_names_its_count_and_its_entries (a test file; outside this file's directory (crates/report/tests)), crates/report/tests/report.rs::the_report_prints_its_count_before_the_entries (a test file; outside this file's directory (crates/report/tests))``、`  tests: crates/report/tests/report.rs`，以及带 `contract`/`impl` 前缀的 `source:` 块（`brief/ours/s5.md ①2`）。
- ①3 `read --whole`：把 `17 | pub fn write_count(text: &mut String, entries: &[Entry]) {`、`18 |     text.push_str(&format!("entries: {}\n", entries.len()));` 与 8–13 行渲染体逐行摆出（`brief/ours/s5.md ①3`）。
- agent 的措辞就是①2 那两行：`callgraph {function:render}` → `callees: from, push_str`（**没有** `write_count`），`also here: write_count -> render.rs:17`（`brief/ours/s5.md` 交付答案 ③）。先验推理原文见 ours 会话 `seq 245`：`s5: filter inverted + dropped write_count call → question "渲染缺了什么" … → the count block (entries: N) is missing (write_count never called).`

**codegraph 的路径**：7 次（含 1 次 `init`，`files` 是范围验证），序列 = `init` → `files` → `node crates/report/src/render.rs` → `callers write_count` → `callees render` → `callees the_report_prints_its_count_before_the_entries` → `node crates/report/src/query.rs`。
- **决定性的一步**是 `callers write_count`：`Callers of "write_count" (0):` / `function write_count (rust) — crates/report/src/render.rs:17` / `ℹ No callers found for "write_count"`（`brief/cg/s5.md ①4`）——"有定义、零调用者"一次成立。
- 反向的边要另发：`callees render` 只回 `struct Entry [references]`（`brief/cg/s5.md ①5`）；"是哪条测试走到 render"要第三次边查询 `callees the_report_prints_its_count_before_the_entries` → `function    render`（`brief/cg/s5.md ①6`）。
- `node crates/report/src/render.rs` 给 20 行带行号源码（`①3`），但**图查询本身不给**"这个函数在本文件里没人调"这种行。促成这批调用的推理是会话 `seq 203` 的清单式计划：`s5: node crates/report/src/render.rs, callers write_count, callees render, node crates/report/tests/report.rs`。

**两侧差异与归因**：
- 信息差（很小）：我们把 `callees`（被调）与 `also here`（本文件还有谁）放进同一次返回，"定义了但没被调用"在一屏里成立；cg 把方向拆成 `callers`/`callees` 两个子命令，要两个方向就得两次调用（本例它用了 3 次边查询）。
- 流程差（更实质）：cg **没有测试运行器**。s5 的"红在哪、红在断言行第几行、断言原话是什么"在我们这边是首答 `check` 的一行（`the count line is printed`），cg 只能靠源码 + 早先的全量文件通读来构造"这条会红"（其答案 ④ 的 `会先在 expect("the count line is printed") 上炸` 不是该题仪器返回里的字）。
- 归因落到具体行：我们的 `callers (2): …` 行同时给出了**测试文件身份**（`(a test file; …)`）与**测试名**，cg 的 `callers write_count` 在 0 调用者时只给定义行、没有"谁会因为这件事变红"的线索。

**代价**（引 §三）：仪器 3 vs 7 · 输出 1,560 vs 0 tok · 推理 3,463 vs 0 字符 · 上下文 107,008 vs 0（cg 三栏按共享步归属，见前注）。
**质量**（引 §四）：我们 26/26 命中，只读情景 `s1`–`s8`+`g1`+`g2`+`g4` 10/10；codegraph 24/26，`s3`/`g1` 判"部分"，`s5` 不在"部分"之列 ⇒ 两侧都答对。
**引导含义**：
- 我们桥：把①2 那种"同屏对照"变成显式一行，例如在 `callees:` 之后加 `not called here: write_count (defined crates/report/src/render.rs:17)`，或在 `also here:` 上加后缀 `(not called from this body)`。依据：连 agent 自己复述证据时用的都是这两行的并置（`brief/ours/s5.md` 交付答案 ③）。
- 我们桥：`check` 的 `why` 行必须保留 panic 原文（本例 `the count line is printed`）——它正是 `s5`（缺件）与 `s8`（次序）唯一的分辨行，见下题。
- cg 侧（信息差可直接指出）：`callers <fn>` 在 0 调用者时，值得补一句**哪个测试文件会因此失败**（它已有 `affected` 概念），否则"函数成了死代码"到"哪条断言会红"之间永远要人补一步。

---

### `s6` · 契约与实现一致吗

**我们臂的路径**：3 次仪器调用，序列 = `why {at:crates/core/src/model.rs:37}` → `callgraph {function:postable}` → `check {face:default}`。
- ①1 `why`：**决定性的一步**。它一次给了三样东西——契约行（`  contract   (from the doc above it, which is the promise this tree writes)` 下逐行引 `crates/core/src/model.rs:29…34`，含 `/// The rule the whole service leans on: an entry is postable when it has a receipt, and a zero`）、唯一调用者 `  callers    crates/core/src/store.rs::post`、以及没有人钉住它的那句 ``  pins       no test names `postable` — nothing pins this, so a change here is unverified until something does``（`brief/ours/s6.md ①1`）。
- ①2 `callgraph`：`  callers (1): crates/core/src/store.rs::post`、`  callees: -`、`  also here: new -> crates/core/src/model.rs:21`，`source:` 块用角色前缀把承诺与实现并排：`    contract /// Whether this entry may be posted at all.` … `    impl             self.has_receipt || self.amount == 0`（`brief/ours/s6.md ①2`）。
- ①3 `check`：`verdict  passed (cargo exit 0)`（`brief/ours/s6.md ①3`）——行为层的"没被抓住"。
- agent 的总结把三者串成一条：`why` 明说 `pins: no test names postable — nothing pins this`；`check {face:default}` **passed**：绿是「没覆盖」，不是「一致」（`brief/ours/s6.md` 交付答案 ③）。

**codegraph 的路径**：5 次（含 1 次 `init`），序列 = `init` → `files` → `node crates/core/src/model.rs` → `callers Entry::postable` → `node crates/core/src/store.rs`。
- `node crates/core/src/model.rs` 一次给全文件带行号源码：doc（`29	/// Whether this entry may be posted at all.` … `32	/// The rule the whole service leans on…`）与实现（`37	pub fn postable(&self) -> bool {` / `38	    self.has_receipt || self.amount == 0`）同屏，但**没有"哪几行是契约、哪一行是实现"的角色标注**，这个判断由 agent 自己做（`brief/cg/s6.md ①3`）。
- `callers Entry::postable` → `method Entry::postable (rust) — crates/core/src/model.rs:37` / `method      post` / `  crates/core/src/store.rs:21`（`①4`）；`node store.rs` 给守卫 `22	        if !entry.postable() {`（`①5`）。
- 它的"没人覆盖"是**源码推断**：答案 ④ 写 `另注意本树没有覆盖这条契约的测试（report 套件无零金额用例），所以约定不靠测试兜底`——该题 5 条日志里没有读测试文件的调用。

**两侧差异与归因**：
- 信息差（这就是本题的答案形状）：我们这边"谁向谁承诺、谁依赖这个承诺、有没有测试钉住"三件事在 `why` 一次返回里各自成行（`contract` / `callers` / `pins`）；cg 侧只有"源码 + 调用者"，`pins`（钉子有无）没有对应物，只能靠通读测试文件推断。
- 流程差：我们多一次行为读数 `check`，得到 `verdict passed (cargo exit 0)`——它把"契约违背"与"没被覆盖"两件事分开；cg 无测试面，只能给静态结论。
- 另有一处信息形态差：`callgraph` 的 `source:` 用 `contract`/`impl` 前缀把 doc 与体分段打印，`why` 又把同样的 doc 作为 `contract` 展开——同一份承诺在两个不同工具里都能取到；cg 的 `node` 只有源码，角色要靠人读。

**代价**（引 §三）：仪器 3 vs 5 · 输出 3,634 vs 928 tok · 推理 6,056 vs 0 字符 · 上下文 239,104 vs 121,984。
**质量**（引 §四）：两侧都判对（我们 26/26；cg 24/26，`s6` 不在"部分"之列）。cg 的 `||` 反例表述（`Entry::new("a", 0, false)` → `true`）与我们的"偏离只在 `amount == 0` 这一支"是同一事实的两种说法。
**引导含义**：
- 我们桥：`why` 已经是本题的最优单项，唯一缺口是**定义体**——答案 ② 里那行 `pub fn postable(&self) -> bool { self.has_receipt || self.amount == 0 }` 引自第二次调用 `callgraph`。建议 `why` 增加一行 `body`（或 `impl` 前缀回读定义体），使"契约 vs 实现"一次调用闭合。
- 我们桥：`pins` 行应当**同时出现在 `check` 的输出里**（当某 face 绿、而 face 内存在被 `why` 判为 `nothing pins this` 的公开函数时给一行提示），否则"绿 = 没覆盖"这一步仍靠人推（依据：`brief/ours/s6.md` 交付答案 ③ 的整段推理就是人把两处输出拼起来的）。

---

### `s7` · 分桶的上限含不含 1000

**我们臂的路径**：4 次仪器调用，序列 = `check {face:default}` → `read {path:crates/report/src/buckets.rs, lines:1-16}` → `callgraph {function:bucket_name}` → `why {at:crates/core/src/limits.rs:10}`。
- ①1 `check`：`verdict  failed (cargo exit 101)`；`failed the_report_agrees_with_the_ledgers_own_limit`；该条的 `why` 里逐字的失败原文片段是 ``panicked at crates/report/tests/buckets.rs:17:5: assertion `left == right` failed: one amount on each side of the limit: ["bucket small"]``。**同一份返回的 census 里还有一条直接指向本题的行**：``  respelled 1000 is declared as `SMALL_LIMIT` (crates/core/src/limits.rs:10) and written again at crates/report/src/buckets.rs:10``（`brief/ours/s7.md ①1`）。
- ①2 `read`：`   10 |     } else if amount <= 1000 {`（`brief/ours/s7.md ①2`）。
- ①3 `callgraph`：`  callers (1): crates/report/src/buckets.rs::bucket_lines`、`  callees: -`、`  also here: bucket_lines -> crates/report/src/buckets.rs:19`（`brief/ours/s7.md ①3`）。
- ①4 `why --at crates/core/src/limits.rs:10` **失败**：`no function covers crates/core/src/limits.rs:10 in /home/nich/Moirai_N3/nichlink/target/round9/trees/s7` 紧跟 `the functions in this file are:` 而**列表是空的**（`brief/ours/s7.md ①4`）。agent 在答案 ⑤ 里把这次记成"该点不是函数…本次未取到契约行"（`brief/ours/s7.md` 交付答案 ⑤）。

**codegraph 的路径**：6 次（含 1 次 `init`），序列 = `init` → `files` → `node crates/report/src/buckets.rs` → `callers bucket_name` → `node crates/report/tests/buckets.rs` → `node crates/core/src/limits.rs`。
- **决定性的一步**其实是三处源码的会合：`10	    } else if amount <= 1000 {`（`brief/cg/s7.md ①3`）、测试自己的意图行 `7	/// An amount exactly at the small limit belongs to the next bucket up, and the report must agree`（`①5`）、常量与它的 doc `7	/// It is stated here and nowhere else on purpose: the report's buckets read it from this constant,` / `10	pub const SMALL_LIMIT: i64 = 1000;`（`①6`）。
- `callers bucket_name` → `function    bucket_lines` / `  crates/report/src/buckets.rs:19`（`①4`）——与我们的 `callers (1)` 同一条边。

**两侧差异与归因**：
- 信息差（一条**拒绝文案**的差）：`why --at` 落在常量行上时不给任何替代视图（空列表），我们臂因此少拿一处契约；cg 的 `node <任意文件>` 天然是回退视图，一次拿到常量 + 它的 doc。两臂最后都对（`limits.rs` 在各 s 树间逐字节相同，doc 内容我们臂是从装置期的 `grep -n '' crates/core/src/*.rs`（s1 树，ours 会话 `seq 225`）里读到的）。
- 流程差：我们以**符号**为单位（`callgraph {function:…}` 一次给定义 + 调用者 + 源码），cg 以**文件**为单位（要 3 次 `node` 才把实现 / 意图 / 常量凑齐）；反过来我们有 `check`，首答就把"哪条断言红、红成什么样"（`["bucket small"]`）与 census 的 `respelled 1000 …` 一起端上来。

**代价**（引 §三）：仪器 4 vs 6 · 输出 1,139 vs 0 tok · 推理 340 vs 0 字符 · 上下文 128,256 vs 0。
**质量**（引 §四）：两侧都判对（含 1000；`s7` 不在 cg 的"部分"之列）。
**引导含义**：
- 我们桥：`why --at` 命中非函数行时，拒绝文案应带**替代视图**而不是空表——例如列出该文件的符号（`SMALL_LIMIT crates/core/src/limits.rs:10`）或直接回该行源码，并在 `next` 行给 `read {path}`。依据：`brief/ours/s7.md ①4` 的返回只有 `the functions in this file are:` 后接空白，agent 只能把它记成"未取到契约行"。
- 我们桥：`check` 的 census `respelled 1000 is declared as … and written again at …` 是本簿第一条就命中的题眼；建议把它升级为带后果的半句（如 `(two spellings of one limit; the report should read the constant)`）——依据：答案 ② 正是把这条 census 与 `read` 的 `<= 1000` 拼起来才成结论（`brief/ours/s7.md` 交付答案 ②）。

---

### `s8` · 条数行的位置（重点）

**我们臂的路径**：4 次仪器调用，序列 = `check {face:default}` → `callgraph {function:render}` → `read {path:crates/report/src/render.rs, whole}` → `read {path:crates/report/tests/report.rs, lines:30-41}`。
- ①1 `check`：本树**只有一条红**，且 `why` 直接给坐标与断言原话：`panicked at crates/report/tests/report.rs:40:5: the count comes first:`；上面的 `result` 行是 `test result: FAILED. 2 passed; 1 failed;`（`brief/ours/s8.md ①1`）。这条"2 passed 1 failed + 失败落在 :40"就是把「缺件」排除掉的读数——缺件会在 `:38` 的 `expect("the count line is printed")` 就炸（cf. `s5`）。
- ①2 `callgraph`：**一眼看到位置的那一行**在 `source:` 块里，按文件序打印的 `impl` 行：
  `    impl         for entry in entries {` → `    impl         }` → **`    impl         write_count(&mut text, entries);`**；
  块外同时给了 `  callees: from, push_str, write_count -> crates/report/src/render.rs:18`、`  also here: write_count -> crates/report/src/render.rs:18`、`  tests: crates/report/tests/report.rs`（`brief/ours/s8.md ①2`）。
- ①3 `read --whole` 把顺序落到行号上：`9 |     for entry in entries {` / `11 |     }` / `12 |     write_count(&mut text, entries);`（`brief/ours/s8.md ①3`）；①4 `read tests 30-41` 给意图行 `30 | /// What a reader sees first: the count, then the entries. …` 与 `40 |     assert!(count < first_entry, "the count comes first:\n{text}");`（`brief/ours/s8.md ①4`）。
- 先验推理见 ours 会话 `seq 269`：`And s8: render calls write_count AFTER the entries loop → "条数行的位置" answer: the count line is emitted last (after all entry lines), whereas the intent (render doc :5-6 …) wants it right after the header.`；跑完后的分辨推理见 `seq 277`：`Interesting: in s8 the count is still present (write_count called) but after the entry lines. … s8's failure is only the_report_prints_its_count_before_the_entries, and the_report_names_its_count_and_its_entries passes since "entries: 3" is still there.`
- 答案 ③ 的措辞就是①2 的那一行：`callgraph {function:render}` 打印的源码把 `write_count` 放在 **`for` 之后**（与文件 :9-12 逐字一致）（`brief/ours/s8.md` 交付答案 ③）。

**codegraph 的路径**：5 次（含 1 次 `init`），序列 = `init` → `files` → `node crates/report/src/render.rs` → `callers write_count` → `callees render`。
- `node crates/report/src/render.rs` 给出带行号源码，位置在这**一次文件回读**里可见：`12	    write_count(&mut text, entries);` 紧跟在 `9	    for entry in entries {` / `10	        text.push_str(…)` / `11	    }` 之后（`brief/cg/s8.md ①3`）。
- 两条图查询只给**定义行**，不给调用点行：`Callers of "write_count" (1):` / `function write_count (rust) — crates/report/src/render.rs:18` / `function    render` / `  crates/report/src/render.rs:7`（`①4`）；`Callees of "render" (2):` / `function    write_count` / `  crates/report/src/render.rs:18`（`①5`）。也就是说：**"谁调谁"成立，但"在第几行调"只存在于 `node` 的源码里**。
- 它的答案 ④ 把这层出处记错了位：`callees render / callers write_count 都把调用点钉在 render.rs 内、行号 :12（在循环 9-11 之后）`——这两条返回里实际只有 `:18` 与 `:7`，`:12` 来自 `node`（不改变判定，属证据出处问题；同类现象见 §四 对 cg `s4` 把 `#[test]` 行号写成 `fn` 行号的记录）。

**两侧差异与归因**（本题的核心）：
- **答对的路子基本相同**：两侧都是"条数行仍在、调用点在循环之后 ⇒ 位置错"，且两侧的先验都来自装置期的 `diff`（见前注）。真正不同的是**这条信息随哪一次调用来**。
- **信息差（可指到行）**：我们桥的 `callgraph` 把被查函数的**函数体逐行**附在 `source:` 块里（`contract` / `impl` 前缀 + 文件名内顺序），于是"`write_count` 在 `for` 之后"与"`callees` 里有 `write_count`"是**同一次返回**里的两行；codegraph 的 `callers`/`callees` 是纯边视图（`function render — :7`、`function write_count — :18`），**没有调用点行号、也没有函数体**，位置只能靠 `node` 回读源码——所以答案是：**cg 确实要自己读一次源码**，我们桥不必（尽管我们臂的 agent 仍多发了一次冗余的 `read --whole`）。
- **流程差**：cg 每题的固定开销里有一次 `init`；把 init 去掉后本题两侧都是 4 次（我们 4 / cg 4+1）。另外我们多一次行为读数 `check`（`2 passed; 1 failed`，失败点 `:40`），使"缺件 vs 次序"由**实测**分开；cg 的 ④ 只能给假设式反证（`若调用点行号落在 9 之前，本结论被推翻`）。

**代价**（引 §三）：仪器 4 vs 5 · 输出 746 vs 3,307 tok · 推理 1,989 vs 5,976 字符 · 上下文 116,224 vs 123,008。
**质量**（引 §四）：两侧都判对（`s8` 不在 cg 的"部分"之列）；cg 答案 ④ 的出处措辞不准（见上），不影响判定。
**引导含义**：
- **两侧都该改的一处**：把**调用点行号**放进边那一行。我们的 `callees: … write_count -> crates/report/src/render.rs:18` 与 cg 的 `function write_count — crates/report/src/render.rs:18` 现在都是定义行；建议写成 `write_count -> render.rs:18 (called at :12)`。依据：cg 的答案 ④ 已经把 `:12` 当成了 `callees render`/`callers write_count` 的产物，而这两条原文里只有 `:18`/`:7`（`brief/cg/s8.md ①4/①5`）——这正是"图边不带调用点"造成的可复现的引用漂移。
- 我们桥：`callgraph` 的 `source:` 块是本题省下一次 `read` 的那一行，**应保留并加行号**（现在只有 `impl` 缩进没有行号，答案仍要引 `read` 的 `:9-12` 来报坐标）；若 `callees` 带上调用点行号，①3 那次 `read --whole` 就是纯冗余。
- 我们桥：`check` 的 `why` 行给断言行（`:40:5`）而非 `fn` 行（`:35`），与 cg 侧记 `:35` 形成对照——**坐标给断言行更有用**，建议保持（依据：本题的问题就是"位置"，`brief/ours/s8.md ①1` 的 `:40:5` 直接被答案用作坐标）。

---

## 没能判定 / 需要标注的地方

1. **"谁先知道答案"不可归因给仪器**：`s5`–`s8` 四题，两臂都在仪器调用之前用非仪器的 `diff` 拿到了注入（`ours/_apparatus.md:1067`、`cg/_apparatus.md:1414`/`:1565`）。因此本文只判"仪器返回让结论在证据上成立/被区分"，不判"工具发现了缺陷"。若要评"发现能力"，应禁掉装置期的 `diff` 重跑。
2. **cg 的逐题"当时推理"缺位**：其 brief 配到的是收尾推理（`brief/cg/s6.md ①2` 的 `All checks pass:`），促成调用的推理只在会话 `seq 203/217`，本文按后者引用。cg 的步/输出/推理三栏在本题上恒为 0，无法做该题的效率归因（口径见 §三）。
3. **我们臂 `s7` 答案 ⑤ 的一处出处**：答案写 `改用 read 核 crates/core/src/limits.rs:4-10`，但 `target/round9/logs/s7.jsonl` 只有 4 条调用（`check`/`read buckets.rs`/`callgraph bucket_name`/`why limits.rs:10`），没有这次 `read`；该 doc 内容最可能来自装置期在 s1 树上的 `grep -n '' crates/core/src/*.rs`（ours 会话 `seq 225`）。各 s 树的 `limits.rs` 逐字节相同（装置期 diff 未见该文件），故不影响结论，但**引用出处跨树**。
4. **我们臂一次被自己抓到的行号错**：`s5` 与 `s8` 的 `write_count` 定义分别在 `:17` 与 `:18`（缺件树少一行）。会话 `seq 315` 记录了自我更正：`Note in s5 I wrote also here: write_count -> render.rs:17 — that's the s5 tree where write_count is at line 17 … And I wrote "定义在 crates/report/src/render.rs:18-20" in §② — WRONG for s5!`。这说明**变体间行号漂移**是这类题的固有风险，工具返回里的行号（`also here -> :17`）是唯一的锚。
5. **cg 的树保真度告警**（来自其自报，本文未复核）：codegraph 会漏"限定路径调用"与 `#[path]` 挂载的 `#[cfg(test)]` 子模块调用（`cg/_apparatus.md:4358`）。本题四棵树上未见影响（`s5`/`s8` 的 `write_count` 边、`s7` 的 `bucket_lines` 边都正常），但它约束了"0 callers ⇒ 死代码"这类结论的强度。
