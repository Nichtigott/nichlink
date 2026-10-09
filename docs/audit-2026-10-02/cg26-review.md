# codegraph 臂（cg26）独立复核 · verifier · 2026-10-02

**复核者立场**：cg26 与队长两边的结论都只当输入。下面每一条都附**我自己跑出来的原始输出**，
并写明**量它的时刻**（装置在复核期间一直在动，见 §1.4/§4.6）。
**我改动的文件**：只有本文件。`target/probe-cg26/**`、`target/round9/**`、`tools/**`、被冻结的 codegraph 二进制
（`target/round7/tools-upstream/v1.6.1/bin/codegraph`，`--version` → `1.6.1`，sha256 `3184bbf582b5db7c…`）全程只读；
我另外在 `/tmp/cg26v/**` 做了独立复算与复跑。

---

## 0. 结论（先读这一屏）

**这一对读数现在不能用来下「整体比 codegraph 好/差」的判断** ✗。原因不是 cg26 做错了，而是**装置只给了它 15 道题的题目态**：

| 判断 | 能不能下 | 依据 |
| --- | --- | --- |
| 15 道题的**同树性** | **能** ✓ | 15 份副本与 `target/round9/trees/<id>`、与 `TREES.json` 登记哈希**三方逐字节相等**（§1.1，复核时+交付时两次独立复算） |
| 15 道题的**逐题对错** | **能**，且我逐条核过 | 13 命中 / 2 部分（`s3`·`g1` 缺预设要求的测试侧边界声明），未发现"把非缺陷报成缺陷"（§2） |
| 15 道题的**仪器调用数/题** | **能**（两边都能按题归因） | cg26 142 次（125 答题 + 17 索引）vs 第九轮那臂 51 次（同 15 题）＝ 2.78×（不含索引 2.45×）（§3.5） |
| 15 道题的**token 三轴/题** | **不能** ✗ | 两臂都"多题共步"；第九轮审计自己已判"逐题 token 不可归因"（§3.5） |
| **整轮（26 题）的任何比值** | **不能** ✗ | cg26 交付时只有 **15/26**；缺的 11 道是题目态未就位，不是答错（§1.3） |
| "整体比 codegraph 好/差" | **不能** ✗ | 上两行相加；且缺少的 11 道恰好是**最花步数**的可改题族（`fa`–`fe`/`r1`–`r4`/`g3`/`h1`），只拿 15 道去比 26 道会**系统性低估** codegraph 侧 ✗ |

**复核期间装置被修好了一半**：23:09:31–23:13 有人把 23/24 棵树的题目态复位进 `target/round9/trees/**` **并且同步覆盖了 `target/probe-cg26/trees/**`**，
cg26 随即续跑那 11 道（我最后一次看：**19/26 份答案**，`r1`–`r4` 已出，会话**仍在增长**）。
⇒ **一旦 cg26 跑完 26 道并静默**，同一套检查（§1–§3）可以直接套到 26 题上；**在那之前，任何 26 题的比值都是半截比整段** ✗。

**封笔快照（2026-10-02 23:17:07）**：答案 **20/26**（新出的 5 份是 `r1`–`r4` + 1 份）、会话 **111 步 / 推理 395,255 / 输出 162,182**、
`fc` 仍是唯一未复位（`082e734f…` ≠ `da9c82b7…`）；**15 道已交付题的副本此刻仍逐棵等于登记哈希（13/13 + g1/g2）** ✓。

---

## 1. 装置与同构

### 1.1 哈希复算（我自己写的实现，`/tmp/cg26v/hash.mjs`；算法逐字取自 `TREES.json:3`）

**交付时刻（我 23:07 的快照，复位之前）**——cg26 的 11 个数**逐字复现**：

```
fa   b65f6dc1ac9dd7944c553cc5e2abdd2eec4ace1690526cc158e6e9dab5d78de3   （登记 13333fd5…）MISMATCH
fb   97e0e6585e1c4e255280a5f912b08aca51a7fc3b1bc4323ce8d801b129e30f69   （登记 13333fd5…）MISMATCH
fc   082e734f924d560d255bfd775405e600a38a817ea8c5e906c62c67d904489412   （登记 da9c82b7…）MISMATCH
fd   26321ffc04b6739d2b24b97fe2513e244015d3eaf2dbc2f85fe474b1b0b3e24e   （登记 139e95dc…）MISMATCH
fe   26321ffc04b6739d2b24b97fe2513e244015d3eaf2dbc2f85fe474b1b0b3e24e   （登记 e193e793…）MISMATCH
g3   4a53bf269986514b0de5fccbee0dde6b76367021395c42095c7dce7056bffa37   （登记 18e80bbd…）MISMATCH
h1   9117567fcd4d4d70459e456903cda18acad39b342db06b2dcdfbb2b981cc209e   （登记 5cb1ef9d…）MISMATCH
r1   8b5d78763c0a1a684e5e280c6a83afd4a9bcfe9b2234799bc6a8b8d5491e9548   （登记 e437654d…）MISMATCH
r2   8b5d78763c0a1a68…（与 r1 逐字节相同）r3 同、r4 同                            （各自登记值）MISMATCH
s1/s2/s3/s4/s5/s6/s7/s8/g4/h1-supply-chain/h2-claim-unkept/h3/h4 = 登记值 OK（13 棵）
```

⇒ `APPARATUS.md` §2 的表**是真的**（不是算错），我独立复算得到同样的 got/declared。
⇒ 两处附带确认：`r1`–`r4` 四棵**逐字节相同** ✓；`h1` 与 `s1/s3/s4` 逐字节相同 ✓（`diff -r` 亦无差异）。

**那 15 道（本复核真正要用的）——三方相等 ✓**（`diff -r -x target -x .codegraph -x .git` 全部 `IDENTICAL`，
哈希同时等于 `TREES.json` 登记值，交付时与复核对点两次都一样）：

```
s1 s2 s3 s4 s5 s6 s7 s8 g4  h1-supply-chain  h2-claim-unkept  h3-cross-file-chain  h4-one-file-many-algorithms  = 15 棵 ✓
g1 == s3 的字节副本 ✓      g2 == s5 的字节副本 ✓
```

**复位之后（23:13/23:15 快照）**：`23/24` 棵已等于登记值，**只有 `fc` 仍未恢复**；`carrier/` 不在清单里（第九轮复核者已记 T-29）：

```
id | round9_now       | probe_copy_now   | declared         | r9==decl | copy==decl
fa | 13333fd5e006c455 | 13333fd5e006c455 | 13333fd5e006c455 | OK   | OK
fc | 082e734f924d560d | 082e734f924d560d | da9c82b7282a0ace | MISMATCH | MISMATCH
r1 | e437654d8af8eeb4 | e437654d8af8eeb4 | e437654d8af8eeb4 | OK   | OK
h1 | 5cb1ef9da257ad7b | 5cb1ef9da257ad7b | 5cb1ef9da257ad7b | OK   | OK
（其余 21 棵含 fd/fe/g3/fb/s1–s8/g4/四道 hardbug 全 OK；round9 OK=23  copies OK=23  of 25）
```

### 1.2 题面 / 包根 / 禁令

- **题面**：`BRIEF.md` §4 的 26 道题与 cg26 的答案 ID 一一对得上（15 份答案 = `s1`–`s8`·`g1`·`g2`·`g4`·
  四道 hardbug 类；**11 道没有任何答案文件**——我 `test -f` 逐棵点过，`ls answers/` 当时只有 15 个）。
- **真包根**：每次调用的日志首行是 `=== CMD: cd <副本> && codegraph …`，即"工具的工作根 = 该题副本" ✓；
  与 `BRIEF.md:7` 的"每道题一个根"等价（codegraph 用 cwd，不用 `--root`）。
- **禁令**（我把 cg26 的会话**工具调用**全量扫了一遍，`225f5295-20fb-4d73-8bb7-375b9ee71a5b`）：
  - 读 `.audit/`、`truth.json`、`target/round9/answers|logs`、`target/round8/**`：**命中 0** ✓
    （三处字面命中全部是 cg26 **自己写给队长的报告**里在复述禁令，不是执行读取）。
  - `git log/show`：**0 次** ✓（13 条含 "git" 的命令都是 `--exclude .git` 或对副本的 `diff`）。
  - 有一处**口径张力**要记：`cat target/round7/tools-upstream/v1.6.1/bin/codegraph`（＋ `ls` 同目录）——
    这是读**冻结工具本体**（t1 明确指定的那个唯一二进制），不是读第七轮的对照答案/日志；
    `BRIEF.md:17` 的字面是"不许读 `target/round7/` 下的任何东西"。**我判它不构成污染**（读的是工具，不是答案），
    但"工具在禁区里"这件事值得下一轮改：把冻结二进制拷到 `target/tools-frozen/` 再发题 ✓。

### 1.3 三个陷阱：cg26 有没有踩（这是我重点看的一条）

| 陷阱 | 判据（我自己扫的） | 结果 |
| --- | --- | --- |
| ① 索引缺失 ⇒ 静默用**外层工作区**索引 | 15 份日志的**输出**里（排除 `=== CMD` 行）搜 `toolchain/`·`kernel/`·`conventions/`·`macro/`·`tools/`·`hardbug-runs`·`round7\|8\|9`·`docs/audit` | **0 命中** ✓；每棵树开工前都有 `codegraph files`，只列树内相对路径（`SETUP.md:50-74` 的 s1 原文我复核过） |
| ② 符号链接为根 ⇒ `could not read from disk` | 副本是真 `rsync` 拷贝；`.codegraph` 只出现在 `target/probe-cg26/trees/**`（**26 个**），`target/round9/trees/**` 里 **0 个** ✓ | **没踩** ✓（也没污染题树原件） |
| ③ 返回的符号落在**题树之外** | 15 份日志输出中，所有被点名的路径都在题树内；抽验原文：`callers Store::post` → `method Store::post (rust) — crates/core/src/store.rs:21` / `function store — crates/report/tests/report.rs:11` | **没踩** ✓ |

**陷阱 ③ 的反面样本（cg26 自己抓到的那次）**：`codegraph explore …` 在 s3 上对 `audit_count` 报
`no tests found within 3 caller hops`，而 `crates/core/tests/audit.rs:25` 明明有 `ledger_core::audit::audit_count(&entries)`。
⇒ 这是**工具漏边**（限定路径调用没进图），cg26 没有把它当成"死代码"，而是回源核过并在 `s3`/`g1` 里写明 ⇒ **处置正确** ✓。

### 1.4 装置时间线：cg26 的因果判断**证据不足**（详见 §4.1）

我的独立测量（23:09–23:15，复位**动作中**拍的）：

```
fc（唯一还没被复位的树）  dir mtime 22:51:07  —  但最新源文件：21:32:28 src/control/object/dial/dial.rs、21:32:39 tests/registry.rs
fd / fe（复位前）        最新源文件 21:33:43 / 21:33:42
s2 s3 s4 s6 s7 s8        22:50:5x 被改的**唯一文件**是 <树>/.git/index（22:50:51…22:50:58）
r1 r2 r3 r4 的 dir mtime 19:46:40 / 19:51:31 / 19:51:32 / 19:51:33（**早于** arm-mcp 会话的 21:19 开工）
r1/h1/g3 的 git 单提交    "r1-mcp baseline" / "h1-mcp baseline" / "g3-mcp baseline"
```

⇒ 22:50–22:51 那一趟**没有重写这 11 棵树的源码**（只动了若干 `.git/index` 与目录项）；它们当时的内容是
**第九轮 arm-mcp 的交付态**（21:32–21:33 写的盘，`r1`–`r4` 还是"四个缺陷都修好"的全绿同一棵）。

---

## 2. 逐题对错（15 题，冻结；行号我都回源码读过）

判据：四道 hardbug = `target/hardbug-runs/t21/trees/<类>/.audit/truth.json`（字段 `root_cause`/`dead_arm`/`lapsed`/`must_refuse`/`fix.forbidden`）；
其余 = `tools/xirang-mcp-eval scenario-plan` 的 `preset answer`（第九轮那臂用的同一份预设）。

| 题 | cg26 的落点 | 与真值/预设比 | 判定 | 我核过的行号证据（源码原文） |
| --- | --- | --- | --- | --- |
| `h1-supply-chain` | `toggle.rs:16` `to_world`（`root cause`+`mechanism` 单字段齐全）；8 个兄弟 `to_local` | truth：file `src/control/object/toggle/toggle.rs` line **16**，symbol `offset`，mechanism `to_world`/`world` | **命中** ✓ | toggle.rs:3 `use …to_world`、`:16 to_world(x)`；dial/badge/button/gauge/panel/slider/spinner/timeline 各 `:16 to_local`（8 个）；`offsets.rs:5 EXPECTED_TOTAL=136`、`:9-17` 8 项相加；7×17+41=**160**、truth 的 red 原文 `left: 160` ✓ |
| `h2-claim-unkept` | 两处都报：spinner.rs:19-23 缺 `handle_contracts`；台账第 2 行（panel）指纹失效 | truth：`root_cause` spinner.rs:**23**、mechanism `handle_contracts`/`handle_traits`；`lapsed`=panel.rs；`declaration.lacks`=`handle_contracts` | **命中**（诊断）／**fix 违禁** ✗（§4.3） | spinner.rs:19-23 只有 kind/exports/parent；button.rs:23 有 `handle_contracts`；`.xirang/adopted/entries` 两行指纹**逐字相同** `edc72845cc31…`，而 `sha256sum` 得 button.rs `06c45576d883e961…`、panel.rs `d886e9b0c1281eac…`（与 cg26 报的一致） |
| `h3-cross-file-chain` | `host/src/lib.rs:48-66` 计划 8 对 `cut/graft`，独缺 dial；补在 `:65` 之后 | truth：`root_cause` `src/lib.rs`:**48**、symbol `static_graft_plan`、mechanism `dial`/`cut`；`pruned` face `root/control/dial` | **命中** ✓（一处措辞不准，§4.5） | lib.rs:48 `static_graft_plan!(`、:50-65 八对、无 dial；`shipped.rs:8-23` 断言 9 名（含 dial）；truth red 原文打印 9 行无 dial ✓ |
| `h4-one-file-many-algorithms` | `entry.rs:30-32` `zero_arm` 恒 `Post`；`:44` `Refuse` 按构造不可达 | truth：`root_cause` `src/model/entry.rs`:**31**、symbol `zero_arm`、mechanism `ZeroArm::Post`；`dead_arm` line **44** variant `Refuse`；contract line **37** | **命中** ✓ | entry.rs:31 `ZeroArm::Post`；:44 `ZeroArm::Refuse => false`；契约 :36-37；tests 只有 2 条（5±receipt）。**我独立复跑**（`/tmp` 副本）：`warning: variant 'Refuse' is never constructed --> src/model/entry.rs:9:5`、探针输出 `zero+receipt=true / zero-no-receipt=false / five+receipt=true / five-no-receipt=false / signed(-5)=5 / normalized(" A ")="a"` —— 与 cg26 逐字相同 ✓ |
| `s1` | 唯一调用者 `crates/report/tests/report.rs:11 fn store()`，`:13/:14/:15` 三次 `post`；定义 `core/src/store.rs:21` | 预设：至少点名 `report.rs` 那三处（跨 crate、全限定/构造式） | **命中** ✓ | report.rs:5 `use ledger_core::store::Store;`、:11 `fn store()`、:13-15 `store.post(…expect…)`；store.rs:21 `pub fn post` ✓ |
| `s2` | 红的**面** = `audit`；根因 `core/src/audit.rs:9` `== 0` 写反 | 预设：`audit`（只有 `--features audit` 才编译、才红） | **命中** ✓ | s2 与 s1 逐文件 diff **只差 audit.rs:9**（我 diff 原文：`- …amount != 0` / `+ …amount == 0`）；tests/audit.rs:10 `#![cfg(feature="audit")]`；core/src/lib.rs:8 `#[cfg(feature="audit")]` ✓ |
| `s3` | `audit_unused`（audit.rs:14）0 caller；`audit_count`(:8) 特性面可达 | 预设：`audit_unused` **＋必须声明"5 个 `#[test]` 函数由框架运行期调用、不计入"**（F1 口径：不说这一半记**部分对**） | **部分** ✗ | audit.rs:14 `audit_unused` ✓、:8 `audit_count` ✓；我数了 s3 树 `#[test]` = **5**（report.rs:19/34/43、buckets.rs:10、audit.rs:14）——cg26 的答案**只字未提**这 5 个函数（对比：arm-mcp 的 `s3.md:11` 写了"5 个测试文件里的函数无静态调用者（测试框架调用，不算）"） |
| `s4` | 只有 `report.rs` 能到达 `Store`，路 = `:5 use` → `:11 store()` → `Store::{new:15,post:21,entries:31}` | 预设：`crates/report/tests/report.rs`（经 report→core 调用抵达） | **命中** ✓ | 逐跳我读过：report.rs:5/11/12/13-15/16/26/37/46；store.rs:15/21/31 ✓ |
| `s5` | 缺整块条数行：`render.rs:7` 不再调 `write_count`（定义 `:17`）；第二处 `query.rs:27` `>` | 预设：两处——`query.rs` 下限守卫写反 + `render.rs` 的 `write_count` 调用被删 | **命中** ✓ | s5 与 s1 diff：query.rs:27 `>`（原 `<`）＋ render.rs 删掉 `write_count(&mut text, entries);`；s5 render.rs 里 `write_count` 定义落在 `:17` ✓ |
| `s6` | `model.rs:38` `\|\|` 应为 `&&`；两格反例（0,true）/（0,false）都 true；无测试覆盖该契约 | 预设：谓词 `has_receipt \|\| amount == 0` vs 契约 `&& != 0`；不一致的**恰好是零金额那两格**；未覆盖的类是零金额 | **命中** ✓ | model.rs:38 `self.has_receipt \|\| self.amount == 0`；契约 :32-36；tests 只 post -500/500/5000 ✓ |
| `s7` | **含** 1000：`buckets.rs:10 <= 1000`；并指出把 `1000` 又拼了一遍、应读 `limits.rs:10 SMALL_LIMIT` | 预设：不一致——`buckets.rs` 重拼上限，唯一家是 `core/src/limits.rs` 的 `SMALL_LIMIT` | **命中** ✓ | buckets.rs:10 `<= 1000`、limits.rs:10 `SMALL_LIMIT: i64 = 1000`、limits.rs:7-9 说明；tests/buckets.rs:7-9 意图、:13-14 取 999/1000、:17 `assert_eq!(lines.len(), 2)` —— cg26 逐条行号都对 ✓ |
| `s8` | 条数行在**末尾**：`render.rs:12` 在循环 `:9-11` 之后 | 预设：文档写"表头、条数、然后每条一行"，实现把 `write_count` 放在循环之后 | **命中** ✓ | s8 render.rs:9-11 循环、:12 `write_count(…)`、:5-6 文档；report.rs:38-40 断言 `count < first_entry` ✓ |
| `g1` | 函数级 `audit_unused` + `audit_count`（默认面）；分支级 `buckets.rs:8-9`、`store.rs:22-24`、`query.rs:21-25`；⑤"看不见什么"5 条 | 预设同 `s3`（同一棵树、同一问域）：`audit_unused` + 边界声明 | **部分** ✗（同 `s3` 的边界缺失）；**分支级那三条我全部独立核实为真** ✓ | `.matches(` 全树只在 report.rs:26（只设 `min_amount:Some(0)`，`account=None`）→ query.rs:21-25 测试进不去 ✓；`.post(` 只在 report.rs:13-15 且都合法 → store.rs:22-24 的 `return None` 进不去 ✓；`bucket_lines` 只在 tests/buckets.rs:16 且只喂 999/1000 → buckets.rs:8-9 的 `amount < 0` 进不去 ✓ |
| `g2` | 伪代码（表头 → `write_count` → 循环 → 返回）＋差 = `render.rs` 没这条调用；旁列 `query.rs:27` | 预设（level 5）：两处缺陷、病灶分别落 render 与 query | **命中** ✓ | 与 `s5` 同源；伪代码里的 `write_count` 行为（`entries: {len}\n`）核过 s1/s5 的 render.rs:17-19 ✓ |
| `g4` | 判死两支：`bands.rs:36-41`（`if false`）、`bands.rs:54`（`Band::Frozen`，私有枚举无构造） | 第九轮审计口径：说清"哪些臂判不了、为什么"；两支与 arm-mcp 完全一致 | **命中** ✓ | bands.rs:24 `enum Band`（无 pub）、:23 只派生 Copy/Clone/Debug/PartialEq/Eq、:36 `if false`、:42 `amount > limit`、:54 `Band::Frozen`、:61 唯一构造 `Band::Small`、:68 `pub enum State`、:92 `State::Dormant` ✓；cg26 还**拒绝**把 `band_word`/`state_word` 报成死代码（工具报 0 caller，而 bands_tests.rs:14/19 在调），对照 `callees` 原文属实 ✓ |

**"有没有把不是缺陷的东西报成缺陷"**：**15 题里未发现一例** ✓。
反过来还有一处**主动防守**：`g4` 里 codegraph 报 `band_word`/`state_word` "0 callers"，cg26 回源核到
`bands_tests.rs:14/19` 后明确**没有**把它俩列成死代码（这是最容易误报的一格）。

**两处要做减法的地方**（不改判定，但要写下来）：`h3` 的"没有任何第三处引用"不准确
（`graft/src/lib.rs:17 pub mod dial_fast;`、`:32 dial_fast::REGISTRATION` 就是第三处；不影响根因与修法）；
`s4` 把 `#[test]` 的行号写成了 `fn` 的行号（20/35/44 是 `fn`，属性在 19/34/43）。

---

## 3. 代价重算（自己从会话原始日志量）

### 3.1 口径自证（先证明我的口径 = 团队公布口径）

我的脚本（`zstd -dc` 解多帧 → 逐行 JSON → `assistant/message` 计步、`usage.{inputTokens,cacheReadTokens,outputTokens}`、
`content[].type=="reasoning"` 计推理字符）跑第九轮那臂的会话
`aa54ec58-44bf-4f88-a2ed-d0dd11614b93`（会话头标签 `agent-teams:xirang-round9:arm-mcp` ✓），得到：

```
steps=182  reason=278,127  tin=208,653  tcache=36,192,640  tout=152,225   步均上下文=198,861  结束上下文=342,656
```

与 `docs/audit-2026-10-02/all-runs-table.md:10/26` 的 `182 / 278,127 / 208,653 / 36,192,640 / 152,225 / 198,860 / 342,656` **逐字相同** ✓
⇒ 口径一致（`步均上下文 = 缓存读 ÷ 步`；`结束上下文 = 末步 cacheReadTokens`；这两条是我按公布数反推并验证的）。

### 3.2 并排（**都带时刻**）

| 会话 | 题量 | 步 | 推理字符 | 未命中输入 | 缓存读 | 输出 | 步均上下文 | 结束上下文 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 我们·第九轮 `arm-mcp`（`aa54ec58`，已结束） | 26 | **182** | **278,127** | **208,653** | **36,192,640** | **152,225** | **198,861** | **342,656** |
| codegraph `cg26`（`225f5295`，**仍在增长**） | 15→19 | 84（23:10 快照）/ 98（23:15）/ **103**（23:15:59） | 354,271 | 108,368 | 14,799,872 | 150,173 | 143,688 | 245,632 |

**这两行现在**不能**相除** ✗：分子是 15–19 道题（且含装置/复核/写作开销），分母是 26 道题。
只有"**仪器调用/题**"这一栏两边都能按题归因（§3.5）。

### 3.3 建索引成本**单列**（不混进答题成本）

| 项 | 值 | 原始出处 |
| --- | --- | --- |
| 15 道的 `codegraph init` 次数 | **17** 次（`g1`/`g2` 各多一次：第一次 rsync 把 s3/s5 嵌成了子目录，重拷重来） | 逐份日志数 `=== CMD: … codegraph init` |
| 15 道合计的**索引墙钟** | **8.538 s**（均值 502 ms；`g1` 501+493、`g2` 493+466，其余各约 471–578 ms；`Indexed 12 files · 46 nodes, 88 edges in 490ms` 这类原文） | 各 `init` 块里的 `in <n>ms` |
| 能扣/不能扣 | init 是**在同一条会话里**发生的 ⇒ 它的 token **已混进**上表的"步/缓存读/输出"里，**扣不出去** ✗；能单列的只有"17 次调用 + 8.5 s 墙钟"和"142 次里的 17 次" | — |

### 3.4 字节口径纠错（原句把**日志文件大小**当成了**工具输出**）✗

`t1` 的交付语（及 `SETUP.md` 口径）写"输出 **134,213 B**"。我逐块解析这 15 份日志：

```
15 份日志**文件**总字节            = 134,213      ← t1 引用的其实是这个
其中 codegraph **输出**字节（CMD 头与 EXIT 行之间）= 106,223
其中 nonbridge 输出（NONBRIDGE-BYTES 之和）       =   8,578      ← 同一句里又被单列了一次 ⇒ 重复计入
差额（=== CMD/=== EXIT 骨架行等）                 =  19,412
```

⇒ 正确说法是：**codegraph 输出 106,223 B**（不是 134,213 B，虚高 **+26.3%**）；`9 条 nonbridge 调用 / 8,578 B` ✓（这两个数我逐字复现）。
这一栏会影响"输出/题"的对外引用，必须在冻结读数前改掉。

### 3.5 唯一可逐题归因的那一栏：仪器调用数

| 题 | cg26 codegraph 调用（其中 init） | 第九轮 arm-mcp 桥调用 | 比值 |
| --- | --- | --- | --- |
| s1 | 17 (1) | 4 | 4.25 |
| s2 | 7 (1) | 3 | 2.33 |
| s3 | 20 (1) | 3 | 6.67 |
| s4 | 7 (1) | 4 | 1.75 |
| s5 | 7 (1) | 3 | 2.33 |
| s6 | 5 (1) | 3 | 1.67 |
| s7 | 6 (1) | 4 | 1.50 |
| s8 | 5 (1) | 4 | 1.25 |
| g1 | 8 (2) | 3 | 2.67 |
| g2 | 8 (2) | 3 | 2.67 |
| g4 | 13 (1) | 3 | 4.33 |
| h1-supply-chain | 10 (1) | 4 | 2.50 |
| h2-claim-unkept | 7 (1) | 5 | 1.40 |
| h3-cross-file-chain | 13 (1) | 3 | 4.33 |
| h4-one-file-many-algorithms | 9 (1) | 2 | 4.50 |
| **合计** | **142**（125 答题 + 17 索引） | **51** | **2.78×**（不含索引 2.45×） |

（arm-mcp 的 51 = `target/round9/logs/<id>.jsonl` 行数；其全轮 117 行 ✓ 与 `audit-round9.md:§四` 的"仪器调用（桥）117"一致。）

**口径警告（必须一起读）**：调用数 ≠ 步数 ≠ 成本。两个工具的**调用粒度不同**（codegraph 是一次一问的 CLI；
桥一次调用可回答一个更大的问题），而且 cg26 对四道 hardbug 的调用数（10/7/13/9）
**低于**第七轮 codegraph 单题会话的（23/36/19/21，见 `per-question-table.md:9-16`），却又**高于**桥。
这一栏可以证明"cg26 一次没用工具偷懒"，**不能**单独当"谁更省"的证据 ✗。

---

## 4. 反证：我认为判错或证据不足的地方（5 条）

### 4.1 （最重要）`APPARATUS.md` §4 的因果："22:50–22:51 又跑过一次可改树复位" —— **不成立** ✗

cg26 的原文：`TREES.json mtime 2026-10-02 22:49:10；这 11 棵树的 mtime：22:50:45（r1）… 22:51:10（fe），逐棵间隔约 1.2 s 的顺序重写`
⇒ 它由此推断"登记之后又被复位过一趟，而复位的结果与登记态不同"。

**我用来否它的独立手段（三条，互不依赖）**：

1. **`fc` 到 23:15 仍是未复位的那一棵**（唯一）——它的目录 mtime 是 `22:51:07`，但**最新源文件**是
   `21:32:28 dial/dial.rs`、`21:32:39 tests/registry.rs`，即**第九轮 arm-mcp 会话期间**写的（该会话 21:19–21:40）。
   ⇒ 22:51 只改了目录项，没重写源码；cg26 看到的内容是**上一臂的交付态**。
2. **`s2`/`s3`/`s4`/`s6`/`s7`/`s8` 在 22:50:51–22:50:58 被改的唯一文件是 `.git/index`**
   （`find … -newermt 22:40 ! -newermt 23:00` 原文），`r1`–`r4`/`h1`/`g3` 的 dir mtime 分别是
   `19:46/19:51/19:51/19:51`、`19:47`、`10-01 18:35`（**都早于** arm-mcp 的 21:19 开工）。
3. **语义自证**：`r1`–`r4` 交付时逐字节相同**且全绿**（cg26 自己跑的 `9 passed / 0 failed`）——这正是
   `BRIEF.md:19` 要求的"交付后 `cargo test` 必须绿"；`fb` 带 `ButtonParts`/`PartsContract`、`fa` 多出 `dial`
   也都是"完工态"（cg26 自己就是这么写的）。**"四个注入缺陷都被修好"只可能是答题臂干的，不可能是一趟复位干的** ✗。

**结论**：那 11 棵树的题目态丢失，原因是**第九轮 arm-mcp 的就地修改没有被还原**（它的 `*-mcp baseline`
提交还在树里：`9b0732c r1-mcp baseline`、`fde3250 h1-mcp baseline`、`492a959 g3-mcp baseline`）。
cg26 因此**该暂停**（判决正确 ✓），但它写的**因果是错的**，而因果错误会让下一轮把力气花在"查那趟复位脚本"上 ✗。

### 4.2 "输出 134,213 B" 是把日志文件大小当工具输出（§3.4）—— 可复算的虚高 **+26.3%** ✗

### 4.3 `h2` 的 `fix` 里含**被明令禁止**的动作 ✗

truth 的 `fix.forbidden = ["delete the ledger", "remove the entry"]`；`green.evidence` 更写明
`the bytes moved after the confirmation, so this needs a **person**, not an edit`。
cg26 的 `h2` ③ 写："台账把 panel 那条**按其现存字节重录指纹（或删掉该失效条目）**" ⇒
"重录"是**编辑**，"删掉"就是 `remove the entry`。
对照：arm-mcp 的 `h2.md:10` 写对了——"台账**不能**靠编辑修……失效的条目需要**人**再追加一行确认，**旧行保留为历史**"。
（`must_refuse` 的两条正则两边都没命中；这条是我**读机制**而不是跑正则得到的。）

### 4.4 `s3`/`g1` 少了预设要求的那半边 ⇒ 按 F1 口径记**部分对**，这将改掉"效果面无落后项"的说法 ✗

`scenario-plan`（level 3 判分口径）原文：真值 = `audit_unused` **外加 5 个 `#[test]` 函数**；
"只答 `audit_unused` 且**明确声明**『测试函数由框架在运行期调用、不计入』这一边界的，记**对**；
只答 `audit_unused` 而对测试那一半只字不提的，记**部分对**"。
cg26 的 `s3.md` 与 `g1.md` 都只字未提这 5 个（我数过树里正好 5 条 `#[test]`）。
⇒ **`s3`/`g1` 两题从 arm-mcp 的"持平"变成**"cg26 部分" ✗。这不是答错，是"边界必须被说出来"的口径。

### 4.5 形状偏差两处（装置级，t1 授权，但"同形状"必须写明）

1. **答案末行**：`BRIEF.md:40` 要求末行 `总调用：成功 N / 被拒 M`。cg26 **19/19 份答案都没有**，末行是 `nonbridge: …`
   （arm-mcp **26/26** 都有 ✓）。五段式 ①–⑤ 都在，行数 8–16 ≤ 30 ✓。
2. **日志形状/位置**：`BRIEF.md:12` 要求每题一份 `target/round9/logs/<id>.jsonl`（`{"request","response","exit"}`）；
   cg26 写的是 `target/probe-cg26/logs/<id>.txt`（三行式 `=== CMD`/原文/`=== EXIT`）。
   内容上**更**可核（逐字原文 + 退出码），但**位置与格式都不同** ⇒ "逐题 token 归因"用的那份 jsonl 结构对不上。

### 4.6 复核期间装置在动 —— 任何"这一对读数"必须带时间戳 ✗

我 23:07 与 23:10 两次哈希**同一棵树**得到不同的值，就是因为 23:09:31–23:13 的复位**同时覆盖了
`target/round9/trees/**` **和** `target/probe-cg26/trees/**`（原文：`round9/trees/fa/Cargo.toml` 与
`probe-cg26/trees/fa/Cargo.toml` 同为 `23:10:58.241`，随后 `probe/fa/.codegraph` 在 `23:11:30` 重建）。
好消息：**15 道已交付题的副本内容没被改动**（哈希仍等于登记值 ✓），所以我 §2/§3.5 的判定不受影响；
坏消息：那 11 份"交付时副本"已被覆盖，其交付时状态只剩 `APPARATUS.md` §2 的自述哈希——**但我独立复算过那些哈希是真的**（§1.1），所以现场没丢 ✓。

---

## 5. 我没能判定的（如实）

1. **11 道题的对错**：复核时 `r1`–`r4` 刚出答案、`fa`–`fe`/`g3`/`h1` 还没出，且会话仍在增长 ⇒ 未冻结，不判。
2. **逐题 token**：两臂都"多题共步"（arm-mcp 的 `audit-round9.md:§四` 自己也这么判）⇒ 步/输出/推理的**每题**值不可得。
3. **26 题的步/题 · 步均上下文 · 输出/题 · 推理/题**：要等 cg26 全 26 题静默后重跑本文件 §3.1 的脚本；现在给任何值都是半截 ✗。
4. **`fc` 的复位是否最终完成**：我最后一次快照（23:15）它仍 ≠ 登记值；若在我写完后被复位，按新哈希复核即可。
5. **`fd`/`fe` 的"2 处缺陷 / 3 条断言"这类细节**：我未实跑（第九轮复核者已跑过并登记；我这一轮只做 cg26 侧复核）。
6. **`h1`/`fa`–`fe`/`g3` 那 11 道的"同题同真值"**：它们的真值源（`carrier`、`scenario-inject`）我可以重建，但**答案还没出**，没有可判的对象。

---

## 6. 给队长的行动项

1. **不要现在下整轮结论**。等 cg26 静默（26/26 答案 + 会话不再增长），然后用本文件 §1.1/§3.1 的脚本重跑一遍；
   比较表里至少要有：**同树性（26 棵树逐棵哈希）· 逐题判定（26 行）· 整会话三轴（同一套口径）· 仪器调用/题**。
2. **先修三处口径**：① "codegraph 输出"改成 106,223 B（15 道那批）；② 答案末行补 `总调用：成功 N / 被拒 M`（或在口径里显式声明"本轮末行写 nonbridge"）；
   ③ 逐题表只发"仪器调用"，token 三轴只发整会话。
3. **把 11 道的复位状态在清单里钉死**：复位后逐棵复算哈希（我 23:13 的快照是 23/24，`fc` 未复位），
   并把"复位会把 `probe-cg26/trees/**` 一起覆盖"写进装置说明——否则两臂的 base 哈希会在复核中途悄悄变。
4. **修正 `APPARATUS.md` §4 的因果**（§4.1）：不是"22:50 复位"，是"上一臂的交付态没还原"；顺带把 `T-29`（`carrier/` 入清单）与
   "冻结二进制在 `target/round7/` 禁区里"一起记上。
5. **`h2` 的 fix 口径**要写进下一轮的评分注意：失效台账条目**不许**"重录/删除"，正解是"人再追加一行确认、旧行留历史"（§4.3）。

---

### 附：本复核用到的原始命令（可重放）

```sh
# 哈希（TREES.json 自己的算法，node ≥22）
node /tmp/cg26v/hash.mjs target/round9/trees/<id>        # 另跑 target/probe-cg26/trees/<id>
# 同构
diff -r -x target -x .codegraph -x .git target/round9/trees/<id> target/probe-cg26/trees/<id>
# 输出字节（工具输出 vs 日志骨架 vs 非桥）
python3 - <<'PY'  # 见 §3.4 的三个数
# 会话三轴（口径自证见 §3.1）
python3 /tmp/cg26v/axes.py aa54ec58-44bf-4f88-a2ed-d0dd11614b93 225f5295-20fb-4d73-8bb7-375b9ee71a5b
# 真值对照
cat target/hardbug-runs/t21/trees/<类>/.audit/truth.json
python3 tools/xirang-mcp-eval scenario-plan
# h4 独立复跑（/tmp 副本，不碰题树）
cp -r target/round9/trees/h4-one-file-many-algorithms /tmp/cg26v/h4/tree && cd /tmp/cg26v/h4/tree && cargo test --offline
```
