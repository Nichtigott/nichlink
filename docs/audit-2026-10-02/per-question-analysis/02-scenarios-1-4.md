# 第九轮 · 逐题对话分析 · 只读情景 `s1` `s2` `s3` `s4`

> 口径、已核定读数、质量判定、模板与纪律全部取自本轮任务书 `per-question-analysis/README.md`（下称"任务书"）。
> 本文只做**对话层**分析：工具给了什么（逐字）、agent 当时怎么想（逐字）、差异落在哪一行信息或哪一步流程上。
> 引用写法：`brief/<arm>/<id>.md ①n` = 精简版对话该题第 n 条仪器调用（含工具返回与推理）；
> `dialogues/<arm>/<id>.md:<line>` = 完整版对话；`target/.../answers/<id>.md` = 交付答案原文。

---

## 0. 四题共同底数（先看这张表，后面每题只引它）

| 题 | 我们侧的关键调用 | codegraph 同类作用 | 关键不对称 |
| --- | --- | --- | --- |
| `s1` | `callgraph{function}` 一次给"定义+调用者+被调用者+测试归属+源码正文" | `query` 拿符号名 → `callers` 拿边 → `node` 拿行号，**三次**才凑齐同样四样 | 我们一次查询里有"调用者边"，codegraph 的 `node` 只有"used by file"（文件级） |
| `s2` | `check --face <name>` **真跑 cargo**并给失败断言原文 | 无"面"概念（预设原话见下），只能读 cfg + 与参照树 diff | 我们是"跑出来的"，它是"读出来的" |
| `s3` | `callgraph --orphans` 的 `note` 一行**主动**声明测试函数由 harness 调用 | `callers <sym>` 只有 `ℹ No callers found`，无任何测试入口声明 | **本题的关键分野**，见下面 `s3` 一节 |
| `s4` | `search --literal` 给"全树唯一一行 use"+`read` 给归属包 | 无按文本检索，只能逐方法 `callers`（3 次）反证 | 我们多一条**证伪路径**，也多一条 `affected` 误报要清 |

四题里，**我们的工具每次调用都带回"这一行信息足以支撑哪一句结论"的自述**（`evidence:` 行、`next` 行、`note` 行），codegraph 的每条输出都是纯数据、没有边界自述——这条贯穿四题，是四题多数差异的源头。

---

### `s1` · 谁调用了 `ledger_core::store::Store::post`（调用者在另一个 crate）

**我们臂的路径**：**4** 次仪器调用，序列 = `status {root:s1}` → `callgraph {function:post}` → `why {at:crates/core/src/store.rs:21}` → `affected {files:crates/core/src/store.rs}`。

- ①1 `status`：`rust_files=12 functions=18`、`faces  ledger-core: default=[] all=[audit]`、`faces  ledger-report: default=[] all=[]`（`brief/ours/s1.md ①1`）。**这一步就定了整族题的骨架**，推理原文：「s1 (== s3 == s4): baseline tree, all green on default face; `audit` module exists behind feature and `audit_unused` never called.」（`brief/ours/s1.md ①1`）——即 `status` 那一行 `faces` 就是后面 `s2`/`s3` 的题眼。
- ①2 `callgraph {function:post}` = **决定性的一步**。一次调用同时给出五样东西（逐字）：
  `matches 1` / `crates/core/src/store.rs:21 fn post` /
  `callers (1): crates/report/tests/report.rs::store (a test file; outside this file's directory (crates/report/tests))` /
  `callees: Some, len, postable -> crates/core/src/model.rs:37, push` /
  `tests: crates/report/tests/report.rs`，
  底下还逐字附上 `contract` 两行与 `impl` 正文（`brief/ours/s1.md ①2`）。
  那一行 `(a test file; outside this file's directory (crates/report/tests))` 就是 s1 答案的骨架：调用者**在测试文件里**且**不在本目录**。
- ①3 `why {at:…store.rs:21}`：补上契约来源 `crates/core/src/store.rs:19/-20`、`pins  a test names post`、`gate  no #[cfg] attribute sits directly above this definition`（`brief/ours/s1.md ①3`）。这一步把"没有门控 ⇒ 默认面也编译它"钉住。
- ①4 `affected {files:…store.rs}`：`tests: crates/core/tests/audit.rs, crates/report/tests/buckets.rs, crates/report/tests/report.rs`（`brief/ours/s1.md ①4`）——**这条是误报**，答案自己标了：「报 3 个测试文件（见 s4：其中 2 个是同名 `new` 的误报）」（`target/round9/answers/s1.md` ⑤）。
- 跨 crate 那一跳**不是桥给的**：答案用非仪器调用读 `crates/report/Cargo.toml:7` 的 `ledger-core = { path = "../core" }` 才落实「调用点与被调用点确在两个 crate」（`target/round9/answers/s1.md` ②）。

**codegraph 的路径**：**17** 次仪器调用（`17/1/3,254/2,336/176,512`），但对话层看得很清楚：其中 **1 次 `init` + 2 次 `files` + 12 次 `node`** 是对 s1 这棵"参考树"的整树基线读（精简版把它们一并标成 `共享/装置步`，`brief/cg/s1.md ①6–①17`），真正回答 s1 的只有 3 次：

- ①4 `query post`：符号检索，三条命中逐字为
  `method post / crates/core/src/store.rs:21`、`method postable / crates/core/src/model.rs:37`、`function store / crates/report/tests/report.rs:11`（`brief/cg/s1.md ①4`）。**它的作用只是拿到 `callers` 需要的限定名 `Store::post`**。
- ①5 `callers Store::post` = **决定性的一步**：`Callers of "Store::post" (1):` / `method Store::post (rust) — crates/core/src/store.rs:21` / `function store / crates/report/tests/report.rs:11`（`brief/cg/s1.md ①5`）。
- 结论里的机制句「report 的 src（buckets/query/render）只用到 `Entry`，不碰 `Store`」靠 `node` 读三个文件——是**人读源码**得出的，不是工具给的边（`target/probe-cg26/answers/s1.md` ②）。

**两侧差异与归因**：
1. **一次查询 vs 三次查询**：s1 需要的四样东西（定义行 / 调用者 / 被调用者 / 测试归属）在我们这里被 `callgraph{function}` 一次打包（`brief/ours/s1.md ①2` 五行），在 codegraph 里分散在 `query`（符号+行号）、`callers`（边）、`node`（源码与 used-by）三次不同形状的输出里（`brief/cg/s1.md ①4/①5/①7`）。差别不是"谁更聪明"，而是**我们的 callgraph 把调用者边与测试归属做进了同一条记录**，codegraph 的 `node` 只有文件级 `used by 1 file: crates/report/tests/report.rs`（`brief/cg/s1.md ①7`）——文件级，不是符号级。
2. **"在测试文件里"这层判断**：我们直接印在调用者行上（`a test file; …`），codegraph 要靠调用者的路径落在 `tests/` 下自己认（它认得出来，但那是模型的推断）。
3. **17 这个数字的构成**（对话层可见，任务书表只给总数）：`init` 1 + `files` 2 + 基线 `node` 12 + 回答 3。基线 `node` 12 次同时服务 s1/s3/s4 与 g1（cg 自述见 `brief/cg/s2.md ⑤`：「与同骨架参照树 `s1` 逐文件 diff」）。**"17 vs 4"里有一多半是 s 族的公共基线，不是 s1 的净代价**——这条留给"没能判定"。

**代价**（任务书 §3）：仪器 **4 vs 17** · 输出 **1,202 vs 3,254** tok · 推理 **3,114 vs 2,336** 字符 · 累计上下文 **184,320 vs 176,512**。

**质量**（任务书 §4）：我们 **命中**（26/26 中一题；`callgraph` 的 `callers (1)` 与 `why` 的同一行互证）· 它 **命中**，且答案的 ④ 用「任一观察被推翻（出现第二个调用者，含限定路径）就说明我错」写出了可证伪条件（`target/probe-cg26/answers/s1.md` ④）。

**引导含义**（要具体到可实施）：这一题唯一该改的是**第 4 次调用 `affected` 的那一行**。它输出 `tests: crates/core/tests/audit.rs, crates/report/tests/buckets.rs, crates/report/tests/report.rs`（`brief/ours/s1.md ①4`），其中两条是 `Entry::new` 撞上 `Store::new` 的同名误报——而**"按名字匹配"这条免责声明只写在另一个工具的另一个视图里**：`check --census` 才印 `matching is by name, so an unrelated same-named call counts`（`brief/ours/s3.md ①2`）。可实施的改法：`affected` 的行内按定义分列并带证据等级，例如 `post -> tests/report.rs (qualified path)` / `new -> (no test calls this definition; same-name call elsewhere)`；退一步至少把 census 那句免责声明**搬到 `affected` 自己的输出里**。依据是对话里的哪一句：s1 的答案 ⑤ 不得不手写「其中 2 个是同名 `new` 的误报」，而 s4 为此又多花了 2 次调用（`search --literal` + `read`）才把它推翻（`brief/ours/s4.md ①3/①4`）。

---

### `s2` · 全绿但有一个面是红的，是哪个面

**我们臂的路径**：**3** 次仪器调用，序列 = `check {face:default}` → `check {face:audit}` → `check {face:all}`；面名来自同会话里 `s1`/`s3` 的 `status` 那一行 `faces  ledger-core: default=[] all=[audit]`（`brief/ours/s1.md ①1`，答案 ④ 自述「`status` 的面表已给线索」，`brief/ours/s2.md` 交付答案 ④）。

- ①1 `check --face default`：`verdict  passed (cargo exit 0)` + 七条 `result test result: ok.`（其中四条 `0 passed`）（`brief/ours/s2.md ①1`）。7 个二进制里 4 个 `0 passed`——这正是"绿=没跑"的形状。
- ①2 `check --face audit` = **决定性的一步**，逐字：
  `verdict  failed (cargo exit 101)` / `check  cargo test --features audit` / `failed the_audit_counts_the_non_zero_entries` /
  `why    the_audit_counts_the_non_zero_entries: thread '…' panicked at crates/core/tests/audit.rs:24:5: assertion \`left == right\` failed: the zero entry is not counted`（`brief/ours/s2.md ①2`）。
  这一步同时给了三件事：**哪个面**（`--features audit`）、**哪条测试**、**断言原文**。
- ①3 `check --face all`：`verdict  failed (cargo exit 101)`，同一条红（`brief/ours/s2.md ①3`）——用来排除"特性合流后还有第二条".
- 答案的机制句直接引工具：`crates/core/src/lib.rs:8-9` 的 `#[cfg(feature = "audit")]` 与 `crates/core/tests/audit.rs:10` 的 `#![cfg(feature = "audit")]` ⇒ 默认面**根本不编译这两者**。

**codegraph 的路径**：**7** 次仪器调用（`7/1/1,225/0/119,552`），序列 = `init` → `files` → `node audit.rs` → `node tests/audit.rs` → `callers audit_count` → `callees the_audit_counts_the_non_zero_entries` → `node lib.rs`。

- ①3 `node crates/core/src/audit.rs`：逐字源码，第 9 行 `.filter(|entry| entry.amount == 0)`（`brief/cg/s2.md ①3`）。
- ①7 `node crates/core/src/lib.rs`：`8  #[cfg(feature = "audit")]` / `9  pub mod audit;`（`brief/cg/s2.md ①7`）——**cfg 门是它拿到的**。
- **决定性的一步不在 codegraph 里，而在一次非仪器共享 `diff -r -x .codegraph trees/s1 trees/s2`**。推理原文（`dialogues/cg/_apparatus.md:1554`）：
  「with this diff, the test `the_audit_counts_the_non_zero_entries` (in `crates/core/tests/audit.rs`, gated behind `feature = "audit"`) would FAIL when the audit feature is on. And with the default features, that test isn't compiled → everything green on the default face. So: "全绿" = default face green; "有一个面是红的" = the `audit` feature face」。
  答案 ④ 也承认这个依据：「与同骨架参照树 `s1` 逐文件 diff，只差 `audit.rs:9` 这一行 ⇒ 红的面只可能是编译该行唯一测试的那个面」（`target/probe-cg26/answers/s2.md` ④）。
- **它一次 cargo 都没跑**：s2 的 nonbridge 自述是「0 次 / 0 字节 own + 1 次共享（`diff -r`）」（`target/probe-cg26/answers/s2.md` 末行）。"默认面全绿"是**推出来的**（那条测试默认面不编译），不是**跑出来的**。

**两侧差异与归因**：
1. **"面"这个枚举在谁手里**：我们这边 `status` 直接印 `faces ledger-core: default=[] all=[audit]`，`check --face <name>` 是一个**可寻址的动词**；codegraph 完全没有面的概念——预设原话即：「结构上无此能力：它没有「面」的概念，也没有按特性跑测试的入口」（`tools/xirang-mcp-eval:936`）。所以它只能把"面"重建成"读 cfg + 与已知绿的参照树做字节 diff"。
2. **"绿"的证据等级**：我们的是 `verdict  passed (cargo exit 0)`（工具代跑），它的是"那条测试默认面不编译 ⇒ 不可能红"。两种都能得出正确答案，但**只有我们这一侧的红是被观测到的**：`failed …` + panic 行原文（`brief/ours/s2.md ①2`）。
3. **面名需要跨题继承**：我们的 s2 日志里只有 3 条 `check`，没有 `status`——面名是从同会话 `s1`/`s3` 的 `status` 复用来的。这不是缺陷，但意味着**单看 s2 这道题的日志，工具并没有告诉 agent 这个树有哪些面**。

**代价**（任务书 §3）：仪器 **3 vs 7** · 输出 **4,369 vs 1,225** tok · 推理 **9,389 vs 0** 字符 · 累计上下文 **223,488 vs 119,552**。

**质量**（任务书 §4）：我们 **命中**（`check --face audit` 直接给出面名与失败断言）· 它 **命中**，且答案 ④ 明说了它的依据是 diff 而不是测试运行；本轮 cg 侧无 s2 相关的减法项。

**引导含义**：
- 我们侧：**把"这棵树有哪些面、哪些面红"搬进 `check` 自己的输出**。现在面表只在 `status` 里（`faces  ledger-core: default=[] all=[audit]`，`brief/ours/s1.md ①1`），而 `check` 一次只回答一个 `--face`；s2 这道题的题面是"有一个面是红的"，**最自然的单次调用应当是"不带 `--face` 跑一遍并逐面给一行 verdict"**（当前必须 3 次调用 + 从别的题继承面名）。依据：`brief/ours/s2.md` 的 ①1/①2/①3 三条日志里，没有任何一条自己列出了面集合。
- 同样值得搬的还有 `status` 的 `note` 行——「a file compiled only under a non-default feature cannot fail on the default face: run `cargo test --all-features` … before believing a green default run」（`brief/ours/s1.md ①1`）。这句话就是 s2 的答案本身，却印在**另一个动词**上、在你还不知道自己需要它的时候。把它做成 `check --face default` 通过时的一行 `caveat:`，s2 就从"3 次调用 + 跨题继承"变成"1 次调用"。

---

### `s3` · 哪些函数没有任何测试能到达（**本轮关键题**）

**我们臂的路径**：**3** 次仪器调用，序列 = `status {root:s3}` → `check {face:default, census:true}` → `callgraph {orphans:true}`。

- ①1 `status`：`rust_files=12 functions=18`、`faces  ledger-core: default=[] all=[audit]`（`brief/ours/s3.md ①1`）。
- ①2 `check --face default --census true` = 给出**第一半**答案与**边界声明**：
  `test-reachable: 1 of 12 production function(s) no test can reach (18 function(s) indexed in this tree; a static walk from the test files along the same name-in-call-list rule the orphan view uses)` /
  `by directory: crates/core 1 of 7` /
  `fn     no test reaches \`audit_unused\` (crates/core/src/audit.rs:14)`，
  以及盲区自述里的一句：`a test-looking file (\`tests/\`, \`_tests.rs\`, or \`#[test]\`) seeds the walk, so a production file with its own \`#[test]\` is likely not listed`（`brief/ours/s3.md ①2`）。注意 `12 production function(s)` 与 `18 function(s) indexed` 这个**两个数字的差**本身就是框架。
- ①3 `callgraph {orphans:true}` = **决定性的一步**，逐字三行：
  `orphans 1 (defined here, no static caller in this tree)`
  `note   5 function(s) in test files have no static caller either; the test harness calls them, so they are counted here rather than listed above`
  `  fn    audit_unused -> crates/core/src/audit.rs:14`（`brief/ours/s3.md ①3`；同一行也在 `target/round9/logs/s3.jsonl` 第 3 条里逐字存在）
  **题目要的那"另一半"就在 `note` 这一行里，且带数字（5）。**
- 答案把这一行原样兑现：`callgraph {orphans:true}` → `orphans 1`，同名同址；**并注明 5 个测试文件里的函数无静态调用者（测试框架调用，不算）**（`target/round9/answers/s3.md:11`，即任务书 §4 点名的 `s3.md:11`）。
- ④ 还把工具自述的盲区逐条抄进答案（`brief/ours/s3.md` 交付答案 ④）。这三条里没有一条是模型自己补的，全在工具输出里。

**codegraph 的路径**：**20** 次仪器调用（`20/0/0/0/0`），序列 = `init` → `files` → **12 次 `callers`（逐个生产符号）** → `callees the_audit_counts_the_non_zero_entries` → `callees store` → `affected <6 个 src 文件>` → `explore "audit_count audit_unused reachability from tests"` → `node audit.rs` → `node tests/audit.rs`（`brief/cg/s3.md ①1–①20`；我核过原始日志 `target/probe-cg26/logs/s3.txt`，`=== CMD` 正好 20 条，符号表与上面一致）。

- ①9 `callers audit_unused` = **决定性的一步**，逐字只有：
  `Callers of "audit_unused" (0):` / `function audit_unused (rust) — crates/core/src/audit.rs:14` / `ℹ No callers found for "audit_unused"`（`brief/cg/s3.md ①9`）。**没有第二行说明。**
- ①18 `explore audit_count audit_unused reachability from tests` 的 blast radius 行：
  `- \`audit_count\` (crates/core/src/audit.rs:8) — 1 caller in \`crates/core/src/audit.rs\`; no tests found within 3 caller hops`（`brief/cg/s3.md ①18`）——codegraph 确实有"测试"这个概念，但只把测试当作**搜索目标**（"no tests found within N hops"），从不把测试函数当作**入口**来描述。
- ①17 `affected <6 src files>`：`Affected test files (3): crates/core/tests/audit.rs / crates/report/tests/buckets.rs / crates/report/tests/report.rs`（`brief/cg/s3.md ①17`）——测试在这里是"一串文件"，没有入口语义。
- ①15 `callees the_audit_counts_the_non_zero_entries` 只回 `method new / crates/core/src/model.rs:21`，**漏掉测试体里 `ledger_core::audit::audit_count(&entries)` 这条限定路径边**；这一点它自己抓住了，写进答案 ②：「工具漏了这条限定路径的边」（`target/probe-cg26/answers/s3.md:5`），并用 `node` 读源码证伪。

**★ 为什么它没想到"5 个 `#[test]` 由框架运行期调用"——是工具没给，还是它自己没想到？**

**两条都成立，但主因是工具没给；"它自己没想到"是前者的直接后果，而不是独立的疏忽。** 证据链如下：

1. **工具从未给过这句话，而且不给的形状是"沉默"而不是"错误"。** 整个 cg26 语料（`docs/audit-2026-10-02/dialogues/cg/**` 与 `target/probe-cg26/logs/**`）里，`harness` / "由框架运行期调用" 这类表述的全部出现位置只有两处，且**都在复核之后**、都不是工具输出：(a) 队长转来的复核消息（`dialogues/cg/_briefing.md:107`），(b) agent 据此写的补注草稿（`dialogues/cg/s3.md:270-277`）。codegraph 在这一题上与"测试入口"最接近的三次输出分别是：`ℹ No callers found`（①9，无附注）、`no tests found within 3 caller hops`（①18，把测试当搜索者）、`Affected test files (3)`（①17，把测试当文件清单）——**没有任何一条把"测试函数自身也没有静态调用者"讲出来**。
2. **它自己也没去问那一问，而且它列不出问的对象。** 答案 ⑤ 自述：「全树 12 个符号各一次 `callers`（`Entry::{new,postable}`、`Store::{new,post,entries}`、`audit_count`、`audit_unused`、`bucket_name`、`bucket_lines`、`Filter::matches`、`render`、`write_count`）」（`target/probe-cg26/answers/s3.md:8`）——**12 个全是生产符号**。原始日志证实：20 条 CMD 里 12 条 `callers` 逐一对应上面这 12 个名字，**没有一条 `callers` 打在任何一个 `#[test]` 函数上**（`target/probe-cg26/logs/s3.txt:43-155`）。
   这一点很关键：**它不需要"想到"，它只需要把同一句 `callers` 打在测试函数上，工具就会回一个和 `audit_unused` 一模一样的 `No callers found`**——第 5 个"死函数"就会自己跳出来。它没打，是因为**它的符号清单是从"生产文件"里抄的，而没有任何一条工具输出提示它清单不完整**。`files` 只给了 `crates/report/tests/report.rs (rust, 9 symbols)` 这样的数字（`brief/cg/s3.md ②`），没有给"这 9 个里哪些是 `#[test]` 入口"。
3. **它的注意力被另一个（真实的）工具缺陷占满了。** 同一题的推理里，它记下的是「the codegraph caller table shows `audit_count`'s caller as only `audit_unused`, missing the test's qualified-path call. I should mention in s3's ⑤/④.」（`brief/cg/s2.md` ② 该步推理逐字）——它把"测试"这个念头用在了**工具漏边**上，并用 `node tests/audit.rs` 把那条边补了回来；补回来的恰恰是"测试能到达 `audit_count`"这半边，**反而更远离了"测试函数自己不被到达"那半边**。
4. **这个框架是被复核者带进来的，不是被工具带进来的。** 它拿到这半边的唯一时刻是复核之后。注入这句话的正是队长转来的复核消息（原文逐字）：「**s3/g1 判"部分"** ✗（预设要求声明"5 个 `#[test]` 由框架运行期调用、不计入" ✓ **你没写** ✓、**另一臂写了** ✓）—— 续跑时注意同类边界 ✓」（`dialogues/cg/_briefing.md:107`）。agent 随后复述：「the verifier said s3/g1 were judged "partial" because the preset required declaring that 5 `#[test]`s are invoked by the framework at runtime and aren't reached by other code」（`dialogues/cg/s3.md:270`），接「Since the captain asked me to … mind that boundary … I can cheaply append an evidence note to s3.md and g1.md」（`dialogues/cg/s3.md:272`），草稿逐字为「补注（复核者口径）：本树共 5 个 `#[test]`（…）——它们是**框架运行期直接调用**的入口」（`dialogues/cg/s3.md:275`），并落盘（`target/probe-cg26/answers/s3.md:12`）。**"（复核者口径）"这五个字就是答案：口径是外部输入的一条完整句子，agent 只做了抄写。**
   补一条**跨全会话**的旁证：`ℹ No callers found for "X"` 是 codegraph 表达"零调用者"的唯一形状，全轮 26 题里出现了 11 次（`audit_unused`、`write_count`、`band_word`、`state_word`、`ControlHandle`、`write_totals`、`REGISTRATION_RULE` …），**没有一次是打在 `#[test]` 函数上的**（`target/probe-cg26/logs/*.txt` 全量检索）。也就是说：**这条信息不是"工具说错了"，而是"从来没被问过"**——而它之所以没被问，是因为工具的符号清单里没有"哪些是测试入口"这一栏。
5. **对照面：我们这边同一句话是工具主动印的，而且印在正确的那一刻。** `orphans` 视图在给出孤儿的同时印出 `note 5 function(s) in test files have no static caller either; the test harness calls them, so they are counted here rather than listed above`（`brief/ours/s3.md ①3`）。agent 不需要问，只需要抄；答案里也确实只做了抄写（`target/round9/answers/s3.md:11`）。

**结论（归因）**：这是**信息差**（工具输出里有没有"测试入口"这条边界），不是"它更笨"。判据也支持这个定性——预设自己写明：「两臂的工具都默认排测试文件，缺的那一半是**工具边界**，不是答错，但**边界必须被说出来**」（`tools/xirang-mcp-eval:942`）。差别在于：**我们的工具把边界说出来了，codegraph 的工具没有；于是同样的"默认排除测试文件"，一侧的答案自动带上边界，另一侧必须靠复核者补。**

**代价**（任务书 §3）：仪器 **3 vs 20** · 输出 **2,727 vs 0** tok · 推理 **4,495 vs 0** 字符 · 累计上下文 **163,840 vs 0**（cg 侧三栏为 0 是状态机归属问题：本题的 20 条调用多发生在共享/装置步，会话里未定位到该题自己的推理——见"没能判定" §3）。

**质量**（任务书 §4）：我们 **命中**，依据 `target/round9/answers/s3.md:11` 那一行；它 **判"部分"** ✗，依据任务书 §4：「两题都答对了死函数 `audit_unused`，但**都没声明**"5 个 `#[test]` 函数由框架运行期调用、不计入"」。另需并记一条**同一判定的后续**（不改本次口径，供读者知道全貌）：补注落盘后，`cg26-review-2.md:20` 记「t3 已按 t2 的口径补上…⇒ 本轮**改判命中** ✓」；即补注确实把它从"部分"救了回来，但补注的来源是复核者而非工具。

**引导含义**（本题是全篇最关键的一条，落到可实施）：
1. **把我们已有的那句 `note` 升格成一等字段，而不是夹在孤儿表中间的一行。** 现在它是 `callgraph --orphans` 输出里的第二行（`brief/ours/s3.md ①3`）；判分要的是"答案里说出来"，而它说的是"counted here rather than listed above"。可实施改法：把它变成一条固定边界的机器可引用行，例如
   `boundary: test entry points are excluded: 5 #[test] function(s) are invoked by the harness at run time and have no static caller (not listed as orphans)`——
   这样"引用一行即可得分"从 3 步调用缩到 1 步，也把 `5` 这个数字固定在输出里（现在是 `5 function(s)`，同样够用）。
2. **给测试入口一个可枚举视图，堵住"清单是从哪来的"这个漏洞。** cg 侧的失败机制是"它列了 12 个生产符号，没有一个工具输出告诉它该列 17 个"。我们侧虽然靠 `note` 补上了，但同样没有可枚举的入口清单——`18 function(s) indexed`（`brief/ours/s3.md ①2`）不等于"哪 5 个是入口"。可实施：`callgraph --orphans` 增加 `--include-tests`（或 `orphans all`）直接把 5 条列出来，并在默认视图里点明"另有 N 条，用 `--include-tests` 列出"。
3. **把这条边界写进 `check --census` 的列头**（现在是 `test-reachable: 1 of 12 production function(s) no test can reach (18 function(s) indexed in this tree…)`）。可实施：改成 `test-reachable: 1 of 12 production function(s) no test can reach; +5 harness-invoked #[test] entries are entries, not targets (18 indexed)`——让"12 vs 18 的差"在**同一行**里被解释掉，因为现在这个差要靠读者自己去 `not covered` 段找（`brief/ours/s3.md ①2` 的 `a test-looking file … seeds the walk`）。
4. **对"如何引导 AI"的一般化**（可写进提示词或答案模板）：凡是可达性/孤儿类问题，答案模板固定要求一句"入口边界"——`#[test]`/`main`/`#[no_mangle]`/框架回调属于入口，不在"被到达"的问域内。依据：预设 F1 的判分口径是"边界必须被说出来"（`tools/xirang-mcp-eval:942`），而工具把它说出来的时候（我们）得分、没说的时候（cg）掉分——**这条分数完全由工具输出决定，不应留给模型自觉。**

---

### `s4` · 哪些测试文件能到达这个类型、经哪条路

**我们臂的路径**：**4** 次仪器调用，序列 = `affected {files:crates/core/src/store.rs}` → `callgraph {function:post}` → `search {literal:"use ledger_core::store::Store"}` → `read {path:crates/report/tests/report.rs, lines:1-18}`。

- ①1 `affected`：`src/store.rs: 3 definition(s)` / `tests: crates/core/tests/audit.rs, crates/report/tests/buckets.rs, crates/report/tests/report.rs`（`brief/ours/s4.md ①1`）——**先把答案报错**：3 个里 2 个到不了。
- ①3 `search --literal` = **决定性的一步**（证伪 `affected` 的那一步）：
  `literal "use ledger_core::store::Store" (raw bytes, case-sensitive; comments and string literals included)` /
  `tests/report.rs:5: use ledger_core::store::Store;` / `member ledger-report (1 lines)`（`brief/ours/s4.md ①3`）。
  逐字命中**全树只有一行**，这就是"只有 1 个测试文件能到达 `Store`"的硬证据。
- ①4 `read`：给出归属与逐跳行号——`owner ledger-report owns \`crates/report/tests/report.rs\` (its root is …/crates/report)` / `symbols 11: \`store\` lines 11-17 — Three posted entries.` / 逐行 `12 | let mut store = Store::new();`、`13 | store.post(...)`（`brief/ours/s4.md ①4`）。
- ①2 `callgraph {function:post}` 复用 s1 的那次查询结论 `callers (1): crates/report/tests/report.rs::store`（`brief/ours/s4.md ①2`）。
- 答案 ④ 明写这条误报的机制与处置：`affected` 报 3 个与前两条证据冲突，「逐跳核过源码后确认它是**误报**：它的匹配按名字，而 …3 个定义里有 `new`，两个不可达文件各自调用的却是 `crates/core/src/model.rs:21` 的 `Entry::new`（同名不同物）」，「`check` 的 census 自己写明 `matching is by name, so an unrelated same-named call counts` ⇒ 采信 `search`+`callgraph`+逐跳读源」（`target/round9/answers/s4.md` ④）。

**codegraph 的路径**：**7** 次仪器调用（`7/1/966/0/120,832`），序列 = `init` → `files` → `callers Store::new` → `callers Store::post` → `callers Store::entries` → `callees the_report_agrees_with_the_ledgers_own_limit` → `node crates/report/tests/report.rs`。

- 三次 `callers`（①3/①4/①5）是它的"穷举"：`Callers of "Store::new" (1): … function store / crates/report/tests/report.rs:11`、`Store::post` 同、`Store::entries (3)` 另加同文件三个 test（`brief/cg/s4.md ①3/①4/①5`）。
- ①6 `callees the_report_agrees_with_the_ledgers_own_limit`：`function bucket_lines / crates/report/src/buckets.rs:19` + `method new / crates/core/src/model.rs:21`（`brief/cg/s4.md ①6`）——用来证明**另一个**测试文件到不了 `Store`（它的 callees 里只有 `Entry::new`/`bucket_lines`）。
- 差异归因：**它没有"按文本检索一行 `use`"这种动词**。它证明"到不了"的方式是**逐方法 `callers` 的补集**（三个方法都只回 `report.rs`），我们证明"到得了"的方式是**一整行 `use` 的全文命中**（`member ledger-report (1 lines)`）。两条路都对，但我们的那次 `search` 是**单点、可引用的**，它的三次 `callers` 是**分散、需要读者自己求交**的。

**两侧差异与归因**：
1. **多出来的那次 `affected` 是我们自己的噪声源**：我们的 4 次里第 1 次就把 3 个（其中 2 个错的）测试文件摆上来，随后用 `search`+`read` 两次去纠正它（`brief/ours/s4.md ①1/①3/①4`）。codegraph 这一题反而**没有**这个弯路——它没用 `affected`（预设也记着「`affected` 四题全假阴性（历史实测）」，`tools/xirang-mcp-eval:950`）。**净效果是：我们的 4 次里有 1 次是自找的纠错，它的 7 次里有 3 次是逐符号补集。**
2. **行号归属的一处小账**：cg 的答案把三个 `#[test]` 标成 `report.rs:20/35/44`（那是 `fn` 行，`#[test]` 属性在 19/34/43），任务书 §4 记为「`s4` 把 `#[test]` 行号写成了 `fn` 行号」；我们的 `read` 输出带 `symbols 11: \`store\` lines 11-17` 这种符号区间、`#[test]` 行靠逐行读（`brief/ours/s4.md ①4`）。这是**引用精度**的差，不影响判定。

**代价**（任务书 §3）：仪器 **4 vs 7** · 输出 **1,801 vs 966** tok · 推理 **530 vs 0** 字符 · 累计上下文 **125,184 vs 120,832**。

**质量**（任务书 §4）：我们 **命中**（`search` 的唯一命中 + 逐跳 `read`）· 它 **命中**（减法项仅"把 `#[test]` 行号写成 `fn` 行号"，不改判定）。

**引导含义**：
1. 与 `s1` 同一条：**`affected` 必须自证"这条命中是限定路径还是同名"**。这题里它错报了 2/3（`brief/ours/s4.md ①1`），而 agent 是靠**另一个工具的另一种视图**（`check --census` 的那句 `matching is by name, so an unrelated same-named call counts`）才想起要怀疑它的（`target/round9/answers/s4.md` ④）。可实施：`affected` 的每一行附 `evidence: name-match` 或 `evidence: qualified path`，并在 `next` 行里直接建议 `search {literal}`（该建议现在只在 `why`/`callgraph` 的 `next` 里出现，`brief/ours/s1.md ①2/①3`）。
2. **`search --literal` 应当成为"哪些测试文件能到达 X"的首选动作**：本题它一次调用给了唯一的 `use` 行 + 归属包（`member ledger-report (1 lines)`，`brief/ours/s4.md ①3`），并把 4 次调用里的 2 次（`affected`+纠错）省成 0。可实施：`affected` 的输出末尾直接写 `next  \`search {literal}\` for the import line, \`callgraph {function}\` for the edge`——把今天由模型自己发明的这条路径固化成工具的默认建议。

---

## 没能判定 / 口径注记（纪律 §5）

1. **`s1` 的 "17 vs 4" 不是同一件事的代价。** 逐题日志条数（任务书 §3）可归因 ✓，但对话层显示 cg 的 17 条里 **1 `init` + 2 `files` + 12 次对参考树的 `node` 基线读**（`brief/cg/s1.md ①1–①17`）；这 12 次读的是整棵树，s 族里 s3/s4 与 s1 字节相同（cg 自己的 `diff` 已证），它们只各自补了 2 次 `node`（`brief/cg/s3.md ①19/①20`），其余内容靠 s1 的基线读——cg 自己的共享账本也写着「`diff -r -x .codegraph s1 <v>` for v in s2..s8 — served s2,s3,s4,s5,s6,s7,s8 (7 questions)」（`brief/cg/s1.md ①8`）。我按任务书引用该数字，但不把它读作"s1 的净代价"。
2. **cg 侧 `s2`/`s3`/`s4` 的推理为 0 字符，是"会话里未定位"而非"没有推理"。** 精简版把这些步骤标成"共享/装置步 / 会话里未定位"（`brief/cg/s3.md ①3–①17`），配的"当时推理"多为装置与其它题的文本。本文凡引 cg 的"当时的想法"，都取自**时间上最近且有原文的那一处**并标了出处：`s2` 用 `dialogues/cg/_apparatus.md:1554`（共享 diff 那一步），`s3` 用 `brief/cg/s2.md` ②（答案起笔那一步）与 `dialogues/cg/s3.md:270-275`（复核后），`s4` 用答案 ④ 本身。**这些不是该题当步的推理，不能当作它的"现场思考"**；我据此只做"它拿到了什么信息"的判断，不做"它当时在想什么"的强断言。
3. **`s3` 的 补注 与判定口径存在两处记录**：任务书 §4 记 cg `s3`/`g1` 为"部分"（我按纪律引用它）；`cg26-review-2.md:20` 记补注后"改判命中"。**我未重做判定**，只把时间线（补注出现在复核之后，`dialogues/cg/s3.md:270-277`）作为"框架是外部输入"的证据。
4. **一处自报与日志不符（纪律 §3 提醒的那类）**：我们臂 `s1` 答案 ④ 说「我把 **`read`** 到的 `report.rs:11-17` 与 `store.rs:21-27` 对读」（`target/round9/answers/s1.md` ④），但 `s1` 的逐题日志只有 4 条、其中没有 `read`（`target/round9/logs/s1.jsonl`）；同一份材料里对 s1 树的源码阅读记录是 `bash` 的 `grep -n ''`（`dialogues/ours/s1.md:257`）。**判定不受影响**（源码确实读过），但"调用清单"这一栏的自报不能当账本用。
5. **`s2` 的面名来源跨题**：`s2` 自己的 3 条日志里没有 `status`，面名 `audit` 来自同会话 `s1`/`s3` 的 `status`（`brief/ours/s1.md ①1`）。题面同树同源，故不构成错误；但若逐题独立复盘，会看到"工具没在这道题的日志里告诉它有哪些面"。
