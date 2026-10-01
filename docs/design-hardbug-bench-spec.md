# W7 出题台规格（四类复杂 bug：注入 → 构造真值 → 机械判分 → 理想路径 → 必拒捷径）

依据：`target/hardbug/BRIEF.md`（设计种子）与 `target/hardbug/optimization-analysis.md`（A–E 五类分析）。
**纪律**：题目**不由人编写**（由生成器从既有的 `tools/nichlink-mcp-eval` 夹具派生 + 注入），**真值由构造
给出**，**判分机械**（脚本，不许模型当裁判），两臂在**同一棵注入副本**上跑，题面**不含答案**，
参照树与答案键放 `.audit/`。**证伪条件先写死**：若配 codegraph 的称职 agent 靠读
`target/nichlink/out/*.tsv` + 手工推导也能拿下同类题，则判**我们无绝对优势**。

## 通用装置

- 生成器：`tools/nichlink-mcp-hardbug build <dir> <class>`（新工具；复用时以**示例宿主**
  `examples/control-button` + `examples/control-button-graft` 为基座，复制、重指路径、丢掉示例自己的断言，
  再按下面的配方生成子对象并注入，最后写 `.audit/truth.json` 与 `.audit/README.md`）；
- 每棵树：`cargo test --offline` 的**红/绿形状必须与配方一致**，且**由生成器自己跑出来**（见"每题自证"）：
  注入态必须红、文档化修法必须绿、**还原注入后必须重新变红**（否则上面那次自证可能针对的是一棵根本没坏过
  的树）；
- 判分脚本：`tools/nichlink-mcp-hardbug score <tree> <answer> [--log <jsonl>]`，只读**答案文本 + 调用日志**，
  判：根因命中（`file:line` + 机制词）· **仪器调用数** · **步数** · 响应字符/token 估算 ·
  是否构造了反证 · 修复最小性 · 必拒捷径。规则逐条可由 `plan --scoring` 打印给审计读；
  **没有日志就把调用数与步数报 `null`**，绝不从答案的散文里猜。

### 落地形态（2026-10-01，与上面的配方有四处偏差，都是被实测逼出来的）

1. **基座是示例宿主的副本，不是手写的包**：推导注册树、构建期作用域、graft 计划是三套机制，手写夹具得把
   三样重新挣一遍；示例已经挣到了。生成器只做"复制 + 重指两条路径依赖 + 丢掉示例的 `tests/`/`examples/` +
   生成八个同族面与它们的替换面"，并且在示例形状一变时**直接拒绝**（`path = "../../toolchain"` 找不到就
   停下），不猜新形状。
2. **几何函数族住在父面自己的文件里**（`src/control/control.rs`）。生成树只挂载构建选中的**面**，单独一个
   辅助文件永远不会被挂载 —— 这是实测的：`src/geometry.rs` 与 `src/geometry/geometry.rs` 都编不过
   （`cannot find geometry in crate`）。
3. **H4 的"写反的分支"必须是一个死臂，不能是数据相关条件**。O3 的分支栏按契约**不判**数据相关条件，因此
   `if self.amount == 0 { return true }` 这种写法在这一栏里**永远不出现**，用它的题没有可判的红。
   落地夹具改用私有枚举 `ZeroArm::{Post, Refuse}`：`zero_arm()` 无条件返回 `Post` ⇒ `Refuse` 那一条按构造
   不可达（分支栏点名它），而 `postable` 上方的契约写着"零金额一律不可入账" ⇒ 两者相反正是缺陷。默认面
   （`cargo test`）**绿**，判据是分支栏那一行。
4. **H3 的理想路径现在真的成立，靠的是同批补上的 `why` 计划半边**（提交 `67a9f72`）。写这道题时发现
   §7 的 W3 行承诺"发布树/接线/门控"，而已落地的 `why` 把那三样明确推给了 `check`/`registry`/`grafts`
   ⇒ H3 当时**没有一次调用的路径**。补齐后实测：`why --at src/control/object/dial/dial.rs:16` 一次给出
   `scope not-selected (mode=auto)` + `wiring no declared cut in …/src/lib.rs names root/control/dial`。

### 测量必须旁路（2026-10-01 维护者加的约束：**前提是不会改变 AI 的泛化能力**）

判据要的三轴里，凡是"需要新增一个交付物才能量"的指标，**它已经改了那道题**。按这条把要加的字段分成两类：

| 字段 | 任务要改吗 | 处置 |
| --- | --- | --- |
| `usage{input,output,cacheRead}`（provider 自报，非估算） | 不改——事后读 harness 的会话日志 | **已加**：`tools/nichlink-mcp-hardbug tokens <session>` |
| `reasoning_chars`（链长） | 不改——同上 | **已加**（同一个子命令） |
| `fix_diff_lines` / `touched_files_outside_scope` | 要——得让修复落盘 | **不塞进现有四题**；留作新类 |
| `patch_layers`（补丁层数，"糊纸"的代理量） | 要——同上 | **同上**；且它是**新能力问题**，该有自己的题树，不是挂在 h1–h4 上 |

理由：任务一旦多一个交付物，臂就知道它在被量，于是会去优化那个量（写"小 diff"），量到的就不是能力而是
指标；而且题面一变，与 W8 那张表**不再可比**（判据纪律：只有对照自身变化或新增样本才重跑，共享样本对着
冻结集配对）。

**三轴的读法**（写入 run 记录时也必须分轴、不合数）：输入＝`inputTokens + cacheReadTokens +
cacheWriteTokens`（prompt，每一步都要重发）；输出＝`outputTokens`（模型生成的）。**单价不同，因此从不
合成一个数**；价格比由 provider 的三档定价决定，报告里给三档原文数字让人自己套。

同批的操作事实：会话日志是**多帧拼接的 zstd**，`node:zlib.zstdDecompressSync` 只解第一帧（585 KB 的记录
只吐出 284 字节并报 `Unknown frame descriptor`）⇒ 用 `zstd -dc`。

### 校准记录（判据先被真答案校准过一次）

W8 第一轮跑到一半，mcp 臂的 h4 答案当场指出夹具里有**两个**文档违规，而不是一个：`signed` 的文档写着
"A debit prints with a leading minus"，实现却在 `amount < 0` 时返回 `-self.amount` ⇒ 实测
`signed(-5) = 5`。题面的第二条线索（"no test covers the branch that does it"）**只选中未注定的那一个**，
于是真值选的和线索选的不是同一个缺陷。成员的处理比我的真值好：它两个都给了，并写明"哪条线索选中哪一个、
如果判分器的真值算的是零金额那条规则，它的修法在下面、证据是 E6"。
⇒ 已把 `signed` 的文档改成与实现一致（"always non-negative"）⇒ 一道题一个缺陷；这也说明**出题台的第一版
把"文档与实现相反"当成了背景，而它本身就是一个可被发现的缺陷**。
**本轮的处理**：frozen 树不动（两臂面对同一棵树，改动会让比较失效）；h4 在本轮按**两个读法并列**报分，
并标为"该题在本轮不具鉴别力"。**同批记录的判分器弱点**：`root_cause_hit` 是"文件 + 机制词 + 某一处 ±3 行"
的袋装规则，对一份把什么都提一遍的长答案会误判为命中 ⇒ 交给独立复核逐字段重算。

### 四类的实测红/绿（`build` 自己跑出来的原始数字）

| 类 | 注入 | 红（命令 → 退出码） | 绿 |
| --- | --- | --- | --- |
| H1 | `toggle` 的 offset 改用 `to_world` | `cargo test --offline` → **101**（偏移总和断言） | 同命令 → 0 |
| H2 | 台账 + `spinner` 缺 `handle_contracts` + `panel` 一条指纹失效 | `consistency --specimen root/control/button` → 点名 `spinner: lacks handle_traits`；`conformance --anchor root/control/panel` → `lapsed …，需要一个**人**` | 两条都作答（树本身绿） |
| H3 | 入口 8 条切口，`dial` 面存在而不在其中 | `cargo test --offline` → **101**（发布树断言）；构建期断言 `dial ∉ source_scope.tsv` | 补上切口 → 0 |
| H4 | `ZeroArm::Refuse` 死臂 + 契约相反 | 默认面**绿**；红在 `check --face default --census` 的分支栏点名 `ZeroArm::Refuse` | 修后该行消失 |

每个类的题面（`BRIEF.md`）由生成器写出，只给症状与交付字段（`root cause` / `mechanism` / `evidence` /
`counter-proof` / `fix`），并把"树只读、不许改测试、每条断言要给命令 + 原始输出 + 退出码"写死；
实测四份题面对真值关键词（离群对象名、族名、缺失声明名、死臂名、被剪面名）**命中数皆为 0**。

**调用日志取自桥自己**（`nichlink-mcp … --log <文件>` 写 `{"exit","request":[…] ,"response":…}`）——
判分读它，不从答案的散文里猜；桥的日志只覆盖桥调用，因此 `steps` 等于 `instrument_calls` 并在 `notes` 里
写明"shell 步不在这里"，绝不把两者混成一个数。

**已实测的负例**（`score` 能分辨）：一份"离群的是 slider；大概是……；我逐个读完文件确认过了；fix 是重写
文件"的答案 ⇒ `root_cause_hit: false`、`minimal_fix: false`、`counter_proof: false`、
`shortcut_taken: "\b(I (guessed|assume)|probably|might be)\b"`。


## H1 供应链驳杂（对应能力：`consistency`）

- **配方**：在一棵 control-button 基座上，注入**一个兄弟用了另一族 API**：`button` 内部的偏移计算从
  `*_local*` 族改调 `*_world*` 族（例如把 `to_local(x)` 换成 `to_world(x)` 并**同步改一个系数**使它在
  该文件内自洽）；`slider`/`timeline` 不动。**触发条件**：同族 ≥ 8 个对象（少于 8 个时 `ls` + 逐个
  `read` 也能发现 ⇒ 我们的优势只剩成本，因此规模写进题面）；
- **真值**：离群对象名 + 它调用的那一族名（生成器从注入点上取，写成 `.audit/truth.json`）；
- **理想路径**：`consistency --parent root/control --by api` **一次**点名离群 ⇒ 再 `read` 那几行；
- **必拒捷径**：逐个 `read` 八个对象后"猜"（不给证据）；把 `slider`/`timeline` 判成离群（多数派认错）。

## H2 采信未落实（对应能力：`conformance` + 台账）

- **配方**：真台账写一条标本（`provisional`），再加一个**同族新对象只缺一条声明**
  （例如标本声明了 `parts` 层而新对象没有；或新对象没有落实标本的 `handle_traits`）；
  再放一条**已失效**条目（指纹与字节不符 ⇒ `lapsed`）；
- **真值**：哪个对象缺哪一条 · 哪一条采信失效在哪个文件；
- **理想路径**：`conformance --anchor <标本>`（判在不在、在哪失效）+ `consistency --specimen <标本>`
  （谁没照做）⇒ 两次调用内给全；
- **必拒捷径**：只说"台账里有 N 条"而不判在不在；把 `lapsed` 说成"需要改代码"（正解是**需要人**）。

## H3 跨文件因果链（对应能力：`why` + 计划/作用域）

- **配方**：入口的切口声明**故意漏掉新面**（新面文件在、声明不在 ⇒ 构建期作用域把它剪掉）；
  症状设成运行期"面不见了"或某条渲染缺一块；
- **真值**：缺哪条声明（文件 + 行）、被剪掉的面名；
- **理想路径**：`why --at <症状行>` 一次给上游事实（调用者/发布树/接线/门控/契约）⇒ 再 `registry`/`grafts`
  定点；
- **必拒捷径**：只在源码里找"看起来少了什么"，不去看计划产物；把"文件在"当成"会发布"。

## H4 单文件多算法（对应能力：`digest` + 分支级栏）

- **配方**：一个文件里三套算法（各自 `///` 契约），只在**其中一套的一个分支**上写反
  （默认面绿、恰好一支没被用例覆盖）；
- **真值**：哪一支、哪一行、与哪句契约相反；
- **理想路径**：`digest --file` 一次看清结构（三套 + 契约 + 谁被测试点名）⇒ `read` 那一支；
- **必拒捷径**：把整个文件重写；只改测试让症状消失。

## 每题自证（生成器必须自己跑出来的四件事）

1. 注入处**真红**（命令 + 原始输出 + 退出码）；文档化修法**真绿**；
2. 真值文件由**构造**得出（不从答案反推），且判分脚本能机械复核；
3. 红/绿形状与配方一致（例如 H4 恰有一支未被覆盖 ⇒ `check --census` 相应栏位与真值一致）；
4. 题面与真值**物理隔离**（真值在 `.audit/`，题面引用不指向它）——第六、七两轮的泄漏教训。

## 生成器落点（2026-10-01 侦察）

`tools/` 下的出题器与判分器都是 **Python 脚本**（`nichlink-mcp-eval` 66 KB、`nichlink-mcp-eval-chains`
21 KB，另有 `nichlink-chain-eval-*` 一整套）。⇒ **W7 生成器写成独立脚本** `tools/nichlink-mcp-hardbug`
（Python 3），**不要**往 66 KB 的既有脚本里再塞一个职责：

- 内部用 `subprocess` 调既有的出题器起底，再按本文件上面的配方注入；
  **落地时改成了复用示例宿主**（见"落地形态"第 1 条）：`scenario-project` 那棵 ledger 工作区**没有注册面
  ——没有同族、没有台账、没有入口切口**，H1/H2/H3 三类都无处落脚 ⇒ 基座改用 `examples/control-button`
  + `examples/control-button-graft`（唯一一处与本节原计划的偏差）；
- 子命令：`build <dir> <class>`（派生 + 注入 + 自证 + 写 `.audit/truth.json` + 写题面 `BRIEF.md`）·
  `score <tree> <answer> [--log <jsonl>]`（机械判分）· `plan` / `plan --scoring`（打印配方与判分规则，
  给审计读）；
- `.audit/` 的真值与判分脚本**不被题面引用**。
  **未做**：每棵树 git 单提交（第六、七轮的树是这么办的）。本批没有做，因为判分只读答案与日志、不读历史，
  而题面本来就禁止用 git 查历史；`.audit/` 与题面的物理隔离已由"题面不指向它 + 关键词命中数 0"两条证据
  覆盖。
