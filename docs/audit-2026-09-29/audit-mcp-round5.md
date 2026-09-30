# 第五轮审计：三十四个样本的判分、调用账、标签、硬约束与复测

## 摘要（12 条，≤24 行）
1. **四题注入缺陷 = 逐字还原，8/8**：r1…r4 两臂各自的工作树与 `pristine-N` 逐文件 diff 全空（排除 `.git/`、`target/`、`Cargo.lock`、`.codegraph/`、只在选手树里的 `.gitignore`）；每棵 r 树 `git status` 只报一个源文件、diff 恰好一处；`tests/` 未动、无 `#[ignore]`/`#[allow]`；8 棵树交付态 `cargo test --offline` 均 9 passed / rc=0。r4 陷阱：两臂都拒绝捷径（mcp 拒绝在 `post` 里加零额守卫并给出三条理由；cg 在树外副本实测捷径 = 9 绿 + 契约探针红）。
2. **八关 = 7 题两臂全对，s3 两臂同为「部分对」**。s3 只答 `audit_unused`、缺 5 个 `#[test]`（我实测 s3 树恰有 5 条 `#[test]`）——缺的一半来自两臂孤儿视图默认排测试文件这一**共同设计边界**，按 brief 记部分对。
3. **s6**：cg 与队长真值表逐格一致（差异恰为零金额两格；未覆盖类 = 零金额分录）。mcp 对预设判为「对」（它答的类「所有 `has_receipt==false` 的输入」包含 `(false,0)`），但含 **2 处与源码矛盾的事实断言**：称 `(10,false)` 的实现值为 true（实为 false）、称 `(0,true)` 与契约相同（契约要求 false）。按 brief 不改对预设的判定，单列。
4. **五族**：10 棵交付树默认面与 `--all-features` 全绿（rc=0）；`fc` 两臂台账都还在、行都合法（8 段非空、指纹我按内核算法复算全部相符），mcp 台账逐字未动、cg 纯追加 1 行。
5. **fa/fb/fc 两臂分岔是硬事实**：我用交付树的 `cargo run --example tree` 独立复核——mcp 的 `Foo`/`Toggle` **不进构建作用域**（树仍 3 面），cg 的 `Switch` 进（4 面）；代价是 cg 改了 `tests/registry.rs` 的出厂面枚举（§4 冲突）。**fb**：mcp 树 pristine（未交付加深）且其「只剩一条路」被队长实测反驳；cg 交付 5 面结构加深但改了 `tests/registry.rs`（含把一条断言 `!cut.is_full()` 反成 `cut.is_full()`）、`tests/static_plan_allocations.rs`、`examples/graft_record.rs`。
6. **调用**：mcp 仪器 154 次 + 其它 278 次（合计 432；会话工具调用口径 369）；cg 留档 191 块 = 仪器 116 + 其它 75（cargo 71、python3 4），会话另有 364 次非仪器工具调用。**被拒：mcp 8 次（fa 5、fb 1、s4 1、s6 1），cg 仪器 0 次**（cg 的 22 次 cargo 红是 exit 101 测试红 + 3 次 `fmt --check` 退出 1，属夹具既有格式差异）。
7. **复测 1 未达标**：被拒没归零，且 mcp **一次 `--list`/`--list <tool>` 都没调用**（日志零命中）——新增描述没被读过，s4 仍是同族形状试错（第四轮是键名写错，本轮是单值键给了两个值）。
8. **复测 2 达标**：`callgraph→callgraph` 16→**10**、`callgraph→read` 10→**6**（mcp 日志相邻仪器调用对）。
9. **标签**：mcp 真赢 0 / 不具鉴别力 15 / 输 2（fb、fc 字面不合）；cg 真赢 2（fb、fc）/ 不具鉴别力 15 / 输 0。**本轮样本对两臂的区分度很低**：15/17 题两臂同判。
10. **硬约束**：`git log`/`git show`/`git stash`/`git diff <历史>`/出题器/跨臂与跨题的**执行**命中 = 0（文本命中只在 BRIEF 禁令原文与推理里）。**一处边界**：mcp 在 fb 里执行过 2 次 `git checkout -- <自己改过的文件>`（回退自己的实验），不在禁令字面内、未读历史、不影响判定，如实记。
11. **装置缺陷 F 项 6 条**（见 §8）：s3 预设与工具边界、s6 预设不精确、fc 意图措辞预设实现分支、fa/fb 两条硬约束互斥、cg 台账新行是代笔人确认且 `at` 早于既有行、两份答案的「工具给的范围」口径。
12. **没能判定**：fb 的「parts 层可达门绿」我采用队长实测未复现；token 是估算，配对任务内同题开销归第一题（r1 另吃进约 4 万字首轮系统/指令）；mcp 的 `adopt` 为何被当成存在的工具无法判定。

## 1. 判分总表（判据见 §2）
| 题 | mcp | cg | 一句话判据 |
| --- | --- | --- | --- |
| r1 | 对 | 对 | 逐字还原；一处比较符；病灶/行号/文档契约一致 |
| r2 | 对 | 对 | 逐字还原；补回 `write_totals` 少调一行 |
| r3 | 对 | 对 | 逐字还原；上界 `<=`→`<` 半边开 |
| r4 | 对 | 对 | 逐字还原；都拒绝捷径、都在契约处修 |
| s1 | 对 | 对 | 都点名 `crates/report/tests/report.rs:13/:14/:15`（跨 crate） |
| s2 | 对 | 对 | 都答 `audit` 特性面；都点名 `tests/audit.rs:24` |
| s3 | 部分对 | 部分对 | 都只答 `audit_unused`；缺 5 个 `#[test]`（工具边界） |
| s4 | 对 | 对 | 都点名 `crates/report/tests/report.rs` 三条（:20/:35/:44）；都说明 `affected` 更宽 |
| s5 | 对 | 对 | 都点出两处独立病灶：`query.rs:27` 判反 + `render` 漏调 `write_count` |
| s6 | 对⚠ | 对 | cg 与真值表逐格一致；mcp 判对但含 2 处与源码矛盾的事实断言 |
| s7 | 对 | 对 | 都答 1000 归 `small`、应 `large`，并点出没读 `SMALL_LIMIT` 的漂移 |
| s8 | 对 | 对 | 都答条数行在最后一行、与 `render.rs:5-6` 承诺相反 |
| fa | 中立 | 中立 | mcp 守 §4 但新面被剪（3 面）；cg 上线新面（4 面）但改测试枚举 |
| fb | 输 | 真赢 | mcp 树 pristine 未交付且「只剩一条路」被实测反驳；cg 5 面加深但改测试/示例 |
| fc | 输 | 真赢 | 意图字面要求「续期/新增记录」：mcp 台账零改动；cg 纯追加 1 行 |
| fd | 对 | 对 | 都两处最小修复、两面全绿、未改测试 |
| fe | 对 | 对 | 都两面各一处、两操作符、两面全绿 |

## 2. 逐题判据要点（只列要交代的）
- **r1–r4（逐字还原）**：diff 全空即逐字还原（`diff -r -x .git -x target -x Cargo.lock -x .codegraph -x .gitignore pristine-N rN-arm`，8 组均 exit 0）；`git status --porcelain` 只报 1 个 ` M` 源文件（没别的改动，也没有测试文件改动）；`grep -rn '#\[(ignore|allow)'` 8 树零命中。r4 两臂的修复都落在 `src/model/entry.rs`（契约处），没落在调用点。
- **s3**：`grep -c '#\[test\]' s3` = 5（与预设真值一致），故两臂各缺 5 个动态调用者；两臂答案里的「唯一/恰好 1 个」对 `src` 成立、对整树不成立。判据是两臂**共同**受工具默认排测试文件的限制。
- **s6**：真值表（队长实测）`(10,true)=T (10,false)=F (0,true)=T (0,false)=T (-5,true)=T` vs 契约 `has_receipt && amount != 0` ⇒ 差异恰为 `(0,true)`、`(0,false)`；我另行核对 s6 树：全部 `Entry::new` 调用点 7 处无一传 `false`、零额只出现一次且只喂 `audit_count` ⇒ 未覆盖类 = 零金额分录。cg 逐项命中；mcp 的类别陈述包含 `(false,0)` 故判对，但它对实现值的两处断言与源码矛盾（见摘要 3）。
- **fa**：`cargo run --offline --example tree`：`a-extend-mcp` 3 行（无 `Foo`）、`a-extend-cg` 4 行（有 `root/control/switch`）。mcp 满足 §4（未改测试）、违反「对象真的加进去」（它自己也报了这个不圆满）；cg 满足上线、违反 §4（`tests/registry.rs` 42 增 8 删：树 3→4、faces 3→4、scope 2→3，并补 switch 断言）。
- **fb**：`tree` 示例 `b-deepen-mcp` 3 行（Button 仍叶子）、`b-deepen-cg` 5 行（+`/button/label`、`/button/icon`）。cg 的 `tests/registry.rs` 是 111 行改动，含把 `assert!(!cut.is_full())` 反成 `assert!(cut.is_full())`、把叶子面钉子从 Button 改指 Slider；另改 `tests/static_plan_allocations.rs` 与 `examples/graft_record.rs`。mcp 的结论「加深只剩文件夹面一条路 ⇒ 必改公开路径/测试」与队长实测（parts 层可门绿）矛盾；cg 明确否决了该路线（「parts 是编译期类型标签，不是对象真的有了内部」）——这是真实设计判断，两臂立场都记。
- **fc 台账**：两树 `.nichlink/adopted/entries` 均在；mcp 1 行（逐字同交付前）、cg 2 行（纯追加，`git diff` = 1 insertion）。我按内核 `adoption_fingerprint`（path+0+len+0+content+0，排序后 sha256）复算：mcp 行 `9df3f08a…` 相符；cg 行 1 同、行 2 `9c40c287…` 相符；两树行数/字段 = 8、无空字段、点名的文件都存在。cg 的处置「保留不续期 + 新锚记首次确认」与其论证自洽。
- **fd/fe 门**：4 棵树默认面与 `--all-features` 都 rc=0，且 `the_audit_counts_the_non_zero_entries ... ok` 在两面都出现（audit 特性面确实跑到）。fd 两臂都只动 `crates/report/{query,render}.rs`、fe 两臂都只动 `crates/core/src/audit.rs` + `crates/report/src/query.rs`，`git diff` 恰好两行。
- **fmt 既有差异**：`crates/report/src/buckets.rs:23` 与 `tests/buckets.rs:14` 的格式差异是夹具自带；fd/fe 的 cg 提到它并保持最小 diff，**不记成引入格式问题**。

## 3. 调用账（分母先对齐）
**mcp**（口径 A）：**仪器 154 次**（= `log-mcp-*.jsonl` 行数，一次桥调用一行）+ **其它 278 次** = 432。278 = 非仪器 bash 146（cargo 71 / 纯 shell 探查 66 / python3 9）+ 非 bash 工具 132（read 25、write 18、edit 22、agent_teams 41 = claim 13 + update 28、send_message 26 = 桥内 13 + 原生 13）。口径 B：会话共 369 次工具调用（bash 237，其中 91 条是仪器启动器；一条 bash 可含多次仪器调用，故 154 > 91）。
**cg**（口径 A）：**留档 191 个 `=== CMD:` 块 = 仪器 116（codegraph）+ 其它 75（cargo 71、python3 4）**。口径 B：会话 450 次工具调用 = bash 276（含 codegraph 的 86、含 cargo 的 54、纯 shell 探查 131、python3 5）+ 非 bash 174（edit 56、write 26、read 25、grep 2、agent_teams 39 = claim 13 + update 26、send_message 26 = 桥内 13 + 原生 13）。
⚠ 不要把 cg 的「全命令 191」去比 mcp 的「仅仪器 154」。

| 题 | mcp 仪器/被拒/其它/≈tok | cg 仪器/被拒/留档其它/≈tok |
| --- | --- | --- |
| r1 | 7 / 0 / 21 / 21.0k | 5 / 0 / 3 / 23.8k |
| r2 | 9 / 0 / 16 / 6.1k | 6 / 0 / 3 / 6.7k |
| r3 | 8 / 0 / 14 / 6.6k | 5 / 0 / 3 / 6.8k |
| r4 | 9 / 0 / 15 / 8.5k | 6 / 0 / 5 / 24.8k |
| s1 | 9 / 0 / 6 / 10.0k | 6 / 0 / 0 / 18.6k |
| s2 | 6 / 0 / 5 / 1.2k | 5 / 0 / 5 / 1.3k |
| s3 | 9 / 0 / 6 / 14.3k | 21 / 0 / 0 / 19.5k |
| s4 | 10 / 1 / 7 / 1.3k | 5 / 0 / 2 / 1.2k |
| s5 | 9 / 0 / 16 / 18.1k | 4 / 0 / 1 / 18.2k |
| s6 | 12 / 1 / 7 / 1.7k | 4 / 0 / 4 / 1.0k |
| s7 | 8 / 0 / 6 / 13.4k | 4 / 0 / 1 / 11.4k |
| s8 | 6 / 0 / 7 / 1.4k | 3 / 0 / 1 / 1.0k |
| fa | 16 / 5 / 52 / 40.3k | 6 / 0 / 8 / 89.5k |
| fb | 5 / 1 / 50 / 40.3k | 9 / 0 / 16 / 70.9k |
| fc | 11 / 0 / 16 / 14.8k | 14 / 0 / 10 / 34.8k |
| fd | 8 / 0 / 17 / 13.2k | 7 / 0 / 6 / 18.0k |
| fe | 12 / 0 / 17 / 13.1k | 6 / 0 / 7 / 14.0k |
| **Σ** | **154 / 8 / 278 / 225.4k** | **116 / 0 / 75 / 361.6k** |

（表内「其它」列对 mcp 取**会话口径**（因为它的日志只记仪器，其「其它」不在日志里）、对 cg 取**留档口径**；两臂的口径 A/B 已在上面写明，勿横向直接相减。）

- **被拒明细（mcp 8）**：fa 5（4 次 `apply` 形状/取值不合 schema：`--fields` 当 JSON、键名、`exports` 给了数组、`flow` 缺 `id|version|input|output`；1 次父级规则拒绝 exit 1）；fb 1（`adopt` = unknown tool，本树无此工具，fc 用的名字是 `adopted`）；s4 1（`affected --files a b`：单值键给了两个值，exit 2）；s6 1（`search --literal false`，缺显式实参名）。cg 仪器 0 被拒；25 次非零退出按命令类型分开：22 次 cargo 测试红（exit 101，修复前红 / fb 迭代里被内核 overlay 检查判红的用例）+ 3 次 `cargo fmt --check`（exit 1，正是夹具自带的 `buckets.rs:23`/`tests/buckets.rs:14` 既有格式差异，不算引入问题）。
- **token（估算：字符/3.5，标明估算）**：mcp 分栏 = 推理 162.5k 字 / 答复 21.5k / 工具输出 447.1k / 题面 158.0k ⇒ ≈225.4k tok（harness 自报 outputTokens 合计 154.6k，可作交叉核对）。cg = 推理 519.1k / 答复 18.6k / 工具输出 578.6k / 题面 149.1k ⇒ ≈361.6k tok（自报 274.6k）。cg 的推理与工具输出约为 mcp 的 1.3–3.2 倍，主要来自 fa/fb 的 cargo 迭代（16/48 次）。
- **互有胜负**：八关按任务对合计，mcp s1+s2 11.3k vs cg 19.9k、s3+s4 15.7k vs 20.7k、s5+s6 19.9k vs 19.3k、s7+s8 14.8k vs 12.3k ⇒ **mcp 在 s1–s4 明显轻，cg 在 s7–s8 略轻**；s3 的仪器调用 cg 21 次 vs mcp 9 次（cg 逐符号手查，mcp 一次 `--orphans`）。族级 cg 明显更重（fa 89.5k vs 40.3k、fb 70.9k vs 40.3k）。

## 4. 标签（口径：真赢 = 对方答错/答不出而此臂答对；不具鉴别力 = 两臂同判或答案由一条局部证据即可定；输 = 答错/答不出/未交付字面要求）
- mcp：**真赢 0、不具鉴别力 15、输 2**（fb：未交付加深且必要性断言被实测反驳；fc：台账字面要求未做）。
- cg：**真赢 2**（fb 交付了结构加深；fc 按意图字面新增记录）、**不具鉴别力 15、输 0**。
- 逐条论据（不具鉴别力的 15 题）：r1–r4 的决定性判据都是「同一文件里文档契约与代码相邻」的一处对照，一条 `grep`/`read` 就能定，两臂答案逐项相同；s1 的决定性证据是 `search ".post("`（mcp）与 `callers Store::post`（cg），一条文本搜索即可定；s2/s4/s5/s6/s7/s8 两臂答案的要点逐条一致（s6 见摘要 3）；fa 两臂各守一条互斥硬约束（装置缺陷），不能据此判优劣。**注**：s3 的「工具设计边界」不构成鉴别力，因为两臂受同一限制。

## 5. 硬约束（应零命中）
执行面命中 = 0：`git log`/`git show`/`git stash`/`git diff HEAD~`/`git diff <历史>` 在两臂会话里**一次都没执行**（逐条列出所有 `git` 命令：mcp 全是 `git status --short`、`git diff --stat`、`git check-ignore`，另有 2 次 `git checkout -- <文件>`；cg 全是 `git status --porcelain`、`git diff [--stat|文件]`、`git ls-files`）。`nichlink-mcp-eval` 只在 BRIEF 原文与"不许读它"的推理里出现，没有任何 read/bash 调用指向它。跨臂/跨题读树：按 `log-*-<id>` 与答案逐文件扫，**零命中**（无 `log-<另一臂>-*`、无 `/tmp/round5/<别题>-<另一臂>`、无别题答案）。
- **边界一处（如实记，不影响判定）**：mcp 在 fb 里执行 `git checkout -- src/control/object/button/button.rs` 与 `git checkout -- src/control/control.rs src/control/object/button/button.rs src/control/object/slider/slider.rs`，用来回退**它自己**做实验时的改动。禁令字面是 `git log`/`git show`/`git diff <历史>`/`git stash`，且未读历史、未影响答案内容（树最终 pristine）。

## 6. 复测重点读数
1. **被拒：未归零**——mcp 8 次（上表），cg 仪器 0 次。而且 **`--list` / `--list <tool>` 实际调用 0 次**（s4 日志里的 `--list` 字样来自错误信息自带的用法行，不是调用）⇒ 本轮新增的「键名/完整描述」兜底**从未被使用**，因此把 s4 的拒绝归因于描述不足缺乏实测依据。仍被拒的两次同族：fa 的 `apply` 形状（4 次 schema/取值 + 1 次语义规则），s4 的 `affected` 单值键给两个值（第四轮是键名写错，本轮是取值形态）。
2. **被迫回头：两类都降了**——mcp 相邻仪器调用对 `callgraph→callgraph` **16→10**、`callgraph→read` **10→6**（把每题的仪器序列按 log 顺序统计相邻对；`search→search` 仍有 11 次、`check→callgraph` 9 次，是新的高频对）。cg 侧的等价读数是同工具重复很重：`node→node` 24、`callers→callers` 13、`node→callers` 10，其中 s3 一个题里 12 连 `callers` + 7 连 `node`（逐符号手查）。

## 7. 装置/轮次缺陷（F 项）
- **F1 s3 预设与工具边界**：预设真值含 5 个 `#[test]`，而两臂孤儿视图默认排测试文件 ⇒ 两臂被同一限制封顶在「部分对」。建议题面或预设显式声明排除面（或把问题限定为 `src`）。
- **F2 s6 预设不精确**：「未覆盖的一格 = `(false,0)`」为真但不全——未覆盖类是零金额分录（含 `(0,true)`）。已按要求不改预设，仅记。
- **F3 fc 意图措辞预设实现分支**：「必须处理采信台账（续期/新增记录）」把「决定并留证」写成了「写行」。mcp 零改动 + 论证满足用意而不满足字面；cg 满足字面。建议意图改成「必须对台账状态作出决定并留下可核证据」。
- **F4 fa/fb 两条硬约束互斥**：§4「不改测试」与 §5.3「门必须绿 + 对象真的上线」在现有钉子上不可同时满足（cg 两族都改了测试/示例；mcp 两族都保住树但对象/深化没上线）。冲突本身是轮次设计缺陷。
- **F5 cg 台账新行是人确认的代笔**：`verifier = nich (maintainer, requested through the round-5 agent)`，且 `at = 2026-09-30T13:41:53Z` **早于**既有行 `17:00:00Z`（追加在后、时间在前）。内核只校 8 段非空与指纹，故合法但语义可疑。
- **F6 两臂的「工具给的范围」口径**：mcp `affected` 给文件级并集、cg `affected` 给 3 个测试文件，两臂都额外用符号级判据收窄——说明该关的预设（只点名 report.rs）与工具默认答案并不等价，判分按预设取最小必跑集，未把更宽集合算错。

## 8. 我没能判定的部分
- fb 的「parts 层仍可门绿」我只采用队长实测（`/tmp/fb-lead-check`，25+1 与基线同），未自行复现（brief 指定为硬事实）。
- **token 是估算且分题归因有已知偏差**：两题一任务的配对（s1+s2…）以「写第一份答案」为界，配对共享开销落在第一题；r1 还吃进约 4 万字首轮系统/指令文本（故 r1 的 21.0k/23.8k 偏高）。harness 自报 outputTokens 已一并给出可交叉核对。
- mcp 为何把 `adopt` 当作存在的工具（fb）无法判定：我没有该 arm 的工具清单快照（fc 用的正确名字是 `adopted`）。
- cg 台账新行 `at` 时间戳的来源无法证伪（可能抄自其它记录）；只记录它早于既有行这一事实。
- 我未改动任何树：所有 `cargo` 只写各自 `target/`；`.codegraph/` 由 arm 自行清理，审计未新建索引；`audit-gates/` 与解压出的会话文本都写在 `/tmp/round5/audit-gates/` 下。
