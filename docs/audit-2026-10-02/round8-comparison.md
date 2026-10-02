# 第八轮逐题对照表（2026-10-02）

口径：**预设答案**（`tools/nichlink-mcp-eval` 的关卡预设 + 四道 hardbug 的 `truth.json`）· **本轮**（现桥，
`target/round8/answers/`）· **第七轮同臂**（`answer-mcp-*`，对"回归"最直接的一列）· **第七轮对照臂**
（`answer-cg-*`）。**四道 hardbug 是第七轮之后新加的题**，第七轮没有它们 ⇒ 那四行的对照换成 **T-21 的对照臂**
（`target/hardbug-runs/t21/answers/arm-codegraph/`）+ 登记真值。行号都由复核者回源码实核过。

## 有效 15 题

| 题 | 问题（一句） | 预设 / 真值 | 本轮（现桥） | 第七轮同臂 | 第七轮对照臂 | 判定 |
| --- | --- | --- | --- | --- | --- | --- |
| `s1` | 谁调用了 `Store::post`（调用者在另一个 crate） | 至少点名 `report.rs` 那三处 | `report.rs:13-15`（唯一调用者）+ 定义 `store.rs:21`；**1 次调用** | `report.rs:11` 的 `store()`，`:13/14/15` 三处 | `report.rs:11` + 三处 `:20/35/44`；**3 次调用** | **持平**（更省：1 vs 3） |
| `s2` | 全绿但有一个面是红的，是哪个面 | `audit` 面 | 红面 `audit`；病灶 `core/src/audit.rs:9`（`== 0` 应为 `!= 0`）；门控 `lib.rs:8`；断言 `tests/audit.rs:24` | 同上（含失败断言原文） | 同上 | **持平** |
| `s3` | 哪些函数没有任何测试能到达 | `audit_unused` | 恰好 1 个：`audit_unused`（`core/src/audit.rs:14`）**并写出边界**（测试文件里 5 个同名函数由 harness 调用） | 同上（`test-reachable: 1 of 12`） | 同上 | **持平** |
| `s4` | 哪些测试文件能到达这个类型、经哪条路 | 三条路，各自经谁 | 命令面**对**；但说"后两个经 `Store::post→postable`" ✗（`buckets.rs:13-14` 实走 `Entry::new`，`model.rs:21`） | **说对了**（`report.rs` 经 `post`；`buckets.rs`/`audit.rs` 经 `Entry::new`） | 只给 `postable ← post ← store()` 那一路 | **变差** ✗（第七轮同臂本来是对的） |
| `s5` | 渲染缺了什么 | 两处病灶 | `render.rs:7-14` 从不调 `write_count`（`:17-19`，全树 0 调用点）+ `query.rs:27` 下限方向反 | 同上 | 同上 | **持平** |
| `s6` | 契约与实现一致吗 | 不一致（`\|\|` vs "带凭据且非零"） | 修法**对**（`model.rs:38` → `&&`/`!= 0`）；但"N 无凭据的非零分录变成可入账"与实现矛盾（该格两侧同为 `false`）✗，未覆盖类只写 `has_receipt == false` | 契约/实现在 `model.rs:32-36`/`:37-38`，判**不一致** ✓ 无错述 | `model.rs:33-37`/`:38-39` + 全树 `Entry::new(..., true)` | **变差** ✗（错误断言） |
| `s7` | 分桶的上限含不含 1000 | 1000 应归 `large` | `buckets.rs:10` `<= 1000` ⇔ `limits.rs:10` `SMALL_LIMIT=1000` + 文档"below" | 同上 | 同上 | **持平** |
| `s8` | 条数行的位置 | 应在分录之前 | `render.rs:12` 在循环（`:9-11`）**之后**；位置断言 `report.rs:40` | 同上 | 同上 | **持平** |
| `g1` | 还有哪些地方没有任何测试能到 | `audit_unused` | 1 个 + 3 个"文本层没被点名但能到达"（`model.rs:37`/`buckets.rs:7`/`render.rs:18`）+ 完整"我看不见什么"；**3 次调用** | `test-reachable: 1 of 12` + 逐个成员交叉验证 | 逐符号 `callers`，给 3 项 | **持平**（多一次 `affected` 核对） |
| `g2` | 渲染的意图与实现差在哪 | 差"条数块" | 伪代码 + "缺 ≠ 错位" + 点名 `query.rs:27` 是另一处 | 同上 | 同上（且明说 `query.rs` 只点名不处置） | **持平** |
| `g4` | 哪些分支没有任何执行能进入 | 两条 + 判不了的三条 | 两条按构造不可达：`bands.rs:36`（`if false`）· `bands.rs:54`（私有枚举 `Band::Frozen`）✓；**漏了 `bands.rs:91` `State::Closed`** ✗ | 同两条 ✓ | **三条**（含 `bands.rs:91`） | **持平（一处漏列）** |
| `h1-supply-chain` | 渲染偏移不对 | 真值 `toggle.rs:16` | `host/.../toggle.rs:16`：8 个兄弟唯一调 `to_world` 的；`to_local=+7`/`to_world=+31`；红了 `the_rendered_offsets_add_up` | —（新题） | T-21 对照臂：**同点 `toggle.rs:16`** ✓ | **正确，与对照持平**（对真值逐条吻合） |
| `h2-claim-unkept` | 采信未落实 | 真值 `spinner.rs` 缺 `handle_contracts` + `panel` 台账失效 | 两处都点到：`spinner.rs:19-24` 缺 `handle_contracts`；台账 `entries:2` 的 recorded 指纹是 `button` 的 ⇒ panel lapsed；**一处 off-by-one（写 `19-24`，实为 `19-23`）** | —（新题） | T-21 对照臂：`spinner.rs:23` + `panel` 指纹 ✓ | **正确，与对照持平** |
| `h3-cross-file-chain` | 一个面没被发布 | 真值 `src/lib.rs:48` | `src/lib.rs:48-66`：8 条 `cut(` 无一点名 `dial`；红了 `every_declared_widget_ships` | —（新题） | T-21 对照臂：`src/lib.rs:48` ✓ | **正确，与对照持平** |
| `h4-one-file-many-algorithms` | 一条分支不可达 | 真值 `entry.rs:31` | `entry.rs:30-32` 的 `zero_arm` 不看 `self`；`Refuse` 臂不可达；契约 `:34-37` | —（新题） | T-21 对照臂：`entry.rs:31` ✓ | **正确，与对照持平** |

**判定口径的两点更正**（照"分母/口径写清"那条纪律）：
1. 四道 hardbug 的"**变好**"是相对**第七轮同臂**说的，而那四题第七轮**没有** ⇒ 严格说法是
   **"新题且与 T-21 对照臂同为正确"**（不该记进"变好"的分母里）；
2. 按此重算，**有效 15 题的判定是**：**正确/持平 12 · 变差 2（s4 · s6）· 一处漏列（g4）**，
   另有四道新题**与对照持平**。

## 作废 11 题（装置不成立，只登记"如实"）

`r1`–`r4`（各 1 次调用）· `fd` · `fe`（各 1 次）· `fa`(3) · `fb`(1) · `fc`(3) · `g3`(1) · `h1`(2)
—— **无法判定（回归意义上）**：题面预设的起始态不存在（`r1–r4` 已修好、全绿 9 passed；`fd/fe` 与基线逐字节相同；
`fa/fb/fc/g3` 已含题面要求的改动且 `carrier/` 缺失跑不了门；`h1` 三面全绿）。
**逐份看，答案对"装置状态"的陈述是如实的** ✓（`r1–r4` 给出 `9 passed; 0 failed` + `verdict passed`；`fd/fe` 给出
"与 `s3` 逐字节相同"；`fa/fb/fc/g3` 给出 `carrier/` 缺失导致 `cargo metadata failed`；`fb/fc/g3` 还把"树里已有的
改动"逐处点出而没有冒充交付）——**这一半值得记：臂没有把"题已做完"包装成"我做完了"** ✓。

## 工具侧（回归面）

- **54 次桥调用 exit 全 0**（0 拒绝 / 0 崩溃 / 0 usage error）· `evidence` 行 **54/54** · 截断 **9** 处全自报 ✓
- **A5**（坏树上 `search --literal` 报 `no matches` 而出码 0）：**继承缺陷**，用第七轮快照二进制在同树同命令复现出**逐字相同**的输出 ✓
  ⇒ 本轮结论：**未引入新的拒绝/崩溃/误导**。
- **两处指路不准**：`conformance` 的 schema 仍要求 `anchor`（客户端 `keys:` 行让每个读者看到 `anchor*`）·
  图例 `(* = required)` 在没有星号时也印 ⇒ **已修**（`d4c28d9`，各配一条钉子）。
