# 第七轮审计 · 逐题原始命令与关键行（t13 / auditor）

来源：`target/round7/log-mcp-<q>.jsonl`（一行一桥调用）与 `target/round7/log-cg-<q>.txt`（`=== CMD:` + 原始输出逐字 + `=== EXIT:`）。命令行由脚本从日志里取出、未改写。
cg 数据 = codegraph **1.5.0**，产自 18:38–19:04；CLI 于 **19:11** 升到 1.6.1（`~/.codegraph/versions/` 现只剩 v1.6.1）⇒ 凡需重跑 CLI 才能判定的条目一律标「无法判定（版本已变）」。22 份 cg 答案/日志的 sha256 与 `CONTROL-FREEZE.md` **22/22 吻合**。
我的四轴脚本：`.audit/audit-metrics.py`、`.audit/audit-repeat.py`、`.audit/two-shapes.py`；门脚本 `.audit/gates.sh` + `.audit/gates2.sh`（原始输出 `.audit/gate2-*.log`、`.audit/gate-raw-*.log`）。

## r1 — mcp 8 仪器 / cg 5 仪器

- mcp: status(exit0) → search(exit0) → search(exit0) → callgraph(exit2) → read(exit0) → callgraph(exit0) → affected(exit0) → check(exit0)
- cg : codegraph init --help | codegraph init r1-cg | codegraph explore -p r1-cg the_filter_keeps_only_entries_at_or_above_the_floor | codegraph node -p r1-cg Filter | codegraph node -p r1-cg matches
- mcp 关键行: log-mcp-r1.jsonl: 8 行；唯一 exit≠0 是第 4 行 exit=2 `expected `--<key>`, got `matches``
- mcp 关键行: 回答自报「被拒 2 次（exit 2）」⇒ 与逐行解析冲突：日志只有 1 次
- cg  关键行: log-cg-r1.txt: 5 CMD / 5 EXIT，全 exit 0（含 `init --help`、`node Filter`、`node matches`）
- **判**: 两树与 pristine-1 `diff -r` 全空（tests/ 未动、0 处 `#[ignore]`/`#[allow]`、无删断言）；门 rc=0（mcp 9 passed / cg 9 passed）⇒ **平手**（mcp 自报的被拒次数与日志冲突，见硬约束/矛盾节）

## r2 — mcp 6 仪器 / cg 2 仪器

- mcp: status(exit0) → search(exit0) → read(exit0) → callgraph(exit0) → affected(exit0) → callgraph(exit0)
- cg : codegraph init r2-cg | codegraph explore -p r2-cg render report totals buckets the_report_names_its_t
- mcp 关键行: log-mcp-r2.jsonl: 6 行全 exit 0；`callgraph --function write_totals` → `callers (0): -`（修前死代码）
- cg  关键行: log-cg-r2.txt: 2 CMD / 2 EXIT，全 exit 0
- **判**: 两树与 pristine-2 全空；门 rc=0（9/9）⇒ **平手**

## r3 — mcp 5 仪器 / cg 2 仪器

- mcp: status(exit0) → read(exit0) → callgraph(exit0) → affected(exit0) → callgraph(exit0)
- cg : codegraph init r3-cg | codegraph explore -p r3-cg buckets bucket_of Bucket contains half-open boundar
- mcp 关键行: log-mcp-r3.jsonl: 5 行全 exit 0
- cg  关键行: log-cg-r3.txt: 2 CMD / 2 EXIT，全 exit 0
- **判**: 两树与 pristine-3 全空；门 rc=0（9/9）⇒ **平手**

## r4 — mcp 8 仪器 / cg 2 仪器

- mcp: status(exit0) → read(exit0) → read(exit0) → callgraph(exit0) → callgraph(exit0) → search(exit0) → affected(exit0) → check(exit0)
- cg : codegraph init r4-cg | codegraph explore -p r4-cg Store post entry postable zero amount
- mcp 关键行: log-mcp-r4.jsonl: 8 行全 exit 0；会话 t1s36 实测两条捷径：`|| amount != 0` → 7 passed/2 failed、只留 `has_receipt` → 8 passed/1 failed，随后从 /tmp 备份还原（交付态与 pristine-4 逐字节相同）
- cg  关键行: log-cg-r4.txt: 2 CMD / 2 EXIT（init、explore）；会话里只有 grep/`git diff --stat`，**没有**任何反例构造
- **判**: 两树交付都正确且与 pristine-4 全空；**只此一题分胜负**：mcp 真构造并拒绝两条捷径（有实测红），cg 只有论证 ⇒ **mcp 真赢**

## s1 — mcp 3 仪器 / cg 3 仪器

- mcp: status(exit0) → callgraph(exit0) → search(exit0)
- cg : codegraph init s1 | codegraph callers -p s1 post | codegraph explore -p s1 who calls Store::post ledger_report query render repor
- mcp 关键行: log-mcp-s1.jsonl: 3 行全 exit 0；第 3 行 literal `post(` 有命中（`tests/report.rs:13/14/15`，按成员分组）
- cg  关键行: log-cg-s1.txt: 3 CMD / 3 EXIT；`callers post` → `Callers of "post" (1): function store`
- **判**: 两臂答案一致（ledger-report 的集成测试 `fn store()`），口径都点明「另一个 crate」⇒ **平手**；第六轮的 6 次零贡献调用本轮为 0（见四轴④）

## s2 — mcp 3 仪器 / cg 2 仪器

- mcp: status(exit0) → search(exit0) → check(exit0)
- cg : codegraph init s2 | codegraph explore -p s2 audit feature ledger_core audit.rs the_audit_counts_th
- mcp 关键行: log-mcp-s2.jsonl: 3 行全 exit 0；`check --face audit` 正文 `exit   101`（工具自身 rc=0）
- cg  关键行: log-cg-s2.txt: 2 CMD / 2 EXIT；VERIFY 段两次 cargo test（默认绿 / all-features 红 101）
- **判**: 两臂都点名 `--all-features`（ledger-core 的 `audit` 特性面）与 `crates/core/src/audit.rs:9` ⇒ **平手**

## s3 — mcp 13 仪器 / cg 20 仪器

- mcp: status(exit0) → callgraph(exit0) → search(exit0) → callgraph(exit0) → callgraph(exit0) → callgraph(exit0) → --json(exit2) → --json(exit2) → callgraph(exit0) → callgraph(exit0) → nichlink-mcp(exit0) → nichlink-mcp(exit0) → nichlink-mcp(exit0)
- cg : codegraph init s3 | codegraph query --help | codegraph query -p s3 -k function -l 200 -j | codegraph query -p s3 -k method -l 200 -j | codegraph explore -p s3 audit_count audit_unused bucket_lines bucket_name rend | codegraph callers --help | codegraph impact --help | codegraph files --help | codegraph node -p s3 crates/core/src/audit.rs | codegraph callers -p s3 audit_count | codegraph callers -p s3 audit_unused | codegraph callers -p s3 bucket_name | codegraph callers -p s3 bucket_lines | codegraph callers -p s3 render | codegraph callers -p s3 write_count | codegraph callers -p s3 entries | codegraph callers -p s3 matches | codegraph callers -p s3 new | codegraph callers -p s3 post | codegraph callers -p s3 postable
- mcp 关键行: log-mcp-s3.jsonl: 13 行；exit0=11、exit2=2（两次 `--json` 参数位置写错，与自报一致）；自我复现 `check --census true`：`test-reachable: 1 of 12` + `no test reaches audit_unused (crates/core/src/audit.rs:14)` + `declarations: 4`
- cg  关键行: log-cg-s3.txt: 20 CMD / 20 EXIT，全 exit 0；`callers audit_count` 只报 `audit_unused`（漏 `tests/audit.rs:25`，cg 自己把这个盲点写进答案）
- **判**: **问域**：mcp 把 5 个测试文件里的函数单列并写明「the test harness calls them」；cg 写明「`#[test]` 由框架动态调用，按问域排除」。两臂都给出 `audit_unused`（audit.rs:14）且都写清边界 ⇒ **平手**（第六轮此题为 mcp 真赢，本轮 cg 补上了那半条）

## s4 — mcp 4 仪器 / cg 6 仪器

- mcp: status(exit0) → affected(exit0) → callgraph(exit0) → callgraph(exit0)
- cg : codegraph init s4 | codegraph affected --help | codegraph affected -p s4 crates/core/src/model.rs | codegraph impact -p s4 postable | codegraph callers -p s4 post | codegraph impact -p s4 -d 5 postable
- mcp 关键行: log-mcp-s4.jsonl: 4 行全 exit 0；`affected` 给 3 个测试文件，`callgraph postable` → `callers (1)`
- cg  关键行: log-cg-s4.txt: 6 CMD / 6 EXIT；`impact -d 5 postable` + `affected` 给同样 3 个文件
- **判**: 两臂都给出精确面 `tests/report.rs` 三条 + 文件级波及两处（audit.rs 在特性面才编译）⇒ **平手**

## s5 — mcp 7 仪器 / cg 3 仪器

- mcp: status(exit0) → search(exit0) → read(exit0) → callgraph(exit0) → affected(exit0) → affected(exit0) → read(exit0)
- cg : codegraph init s5 | codegraph explore -p s5 render write_count bucket_lines the_report_names_its_c | codegraph callers -p s5 write_count
- mcp 关键行: log-mcp-s5.jsonl: 7 行全 exit 0；`callgraph --function write_count` → `callers (0): -`
- cg  关键行: log-cg-s5.txt: 3 CMD / 3 EXIT；`callers write_count` → `ℹ No callers found`
- **判**: 两臂都给出两处病灶（render.rs 漏调 write_count；query.rs 下限方向）；**独立性两臂都只是推理、都没有构造**（会话里无「只修其一再跑」）⇒ **平手**，且此题对「构造独立性」这一判据两臂同判未达成

## s6 — mcp 4 仪器 / cg 4 仪器

- mcp: status(exit0) → search(exit0) → callgraph(exit0) → callgraph(exit0)
- cg : codegraph init s6 | codegraph explore -p s6 Entry postable has_receipt zero amount contract Store  | codegraph callers -p s6 postable | codegraph callers -p s6 post
- mcp 关键行: log-mcp-s6.jsonl: 4 行全 exit 0；另有独立真值表探针 `/tmp/s6_probe.py`（四格枚举）
- cg  关键行: log-cg-s6.txt: 4 CMD / 4 EXIT；给出四格（契约/实现）对照
- **判**: 按预设容忍度：两臂都点「零金额那一侧」并含 `(false,0)`，都未写成「`has_receipt == false` 两格」⇒ 都算对 ⇒ **平手**

## s7 — mcp 5 仪器 / cg 2 仪器

- mcp: status(exit0) → read(exit0) → callgraph(exit0) → search(exit0) → search(exit0)
- cg : codegraph init s7 | codegraph explore -p s7 bucket_name bucket_lines SMALL_LIMIT limits 1000 small
- mcp 关键行: log-mcp-s7.jsonl: 5 行全 exit 0；`search --literal 1000` 只两处（limits.rs:10 / buckets.rs:10）
- cg  关键行: log-cg-s7.txt: 2 CMD / 2 EXIT
- **判**: 两臂都判「恰好 1000 应为 large、`<=` 应为 `<`」并都指出 buckets.rs:10 重拼常量 ⇒ **平手**

## s8 — mcp 4 仪器 / cg 2 仪器

- mcp: status(exit0) → read(exit0) → callgraph(exit0) → callgraph(exit0)
- cg : codegraph init s8 | codegraph explore -p s8 render write_count entries count line order
- mcp 关键行: log-mcp-s8.jsonl: 4 行全 exit 0
- cg  关键行: log-cg-s8.txt: 2 CMD / 2 EXIT
- **判**: 两臂都判「条数行在输出末尾、对不上 render.rs:5-6 的顺序承诺」⇒ **平手**

## fa — mcp 7 仪器 / cg 3 仪器

- mcp: status(exit0) → apply(exit1) → apply(exit0) → apply(exit0) → registry(exit0) → usages(exit0) → check(exit0)
- cg : codegraph init a-extend-cg | codegraph explore -p a-extend-cg control_object static_graft_plan Control Butt | codegraph affected -p a-extend-cg src/control/object/toggle/toggle.rs
- mcp 关键行: log-mcp-fa.jsonl: 7 行；exit0=6、exit1=1（`apply` 缺 `fields.module` 的工具级拒绝，随后重试成功）
- cg  关键行: log-cg-fa.txt: 3 CMD / 3 EXIT，全 exit 0
- **判**: 门自己跑：a-extend-mcp 27 passed / a-extend-cg 27 passed（两面都 rc=0）；`cargo run --example tree` 两树都 4 行；两臂都加了第三条 `cut(...)` 并更新形状钉子 ⇒ **平手**。差异：mcp 另改了 `Cargo.toml` 一句注释（4 files changed, 114+/31−），而它自报的 `git status` 漏了这一项 ⇒ 一致性缺口，非交付缺陷

## fb — mcp 6 仪器 / cg 3 仪器

- mcp: status(exit0) → converge(exit0) → converge(exit0) → apply(exit1) → apply(exit0) → apply(exit0)
- cg : codegraph init b-deepen-cg | codegraph explore -p b-deepen-cg Button ButtonParts PartsContract provided_par | codegraph affected -p b-deepen-cg src/control/object/button/button.rs
- mcp 关键行: log-mcp-fb.jsonl: 6 行；exit0=5、exit1=1（`inside.parts` 传成字符串，重试成功）
- cg  关键行: log-cg-fb.txt: 3 CMD / 3 EXIT，全 exit 0
- **判**: 门自己跑：b-deepen-mcp 27 passed（1 file, 27+/1−）/ b-deepen-cg 28 passed（2 files, 107+/10−）；两树 tree 都 3 行 ⇒ **平手**。实质差异如实记：mcp 的 Button 内部多了一层 `parts: ButtonParts` 字段但**注册面声明未动**（`REGISTRATION.parts` 仍是 `NoParts`、测试文件零改动）；cg 在面上声明了 `parts:`/`part_contracts:` → `REGISTRATION.parts == "ButtonParts"`，并更新 1 条出厂钉子

## fc — mcp 5 仪器 / cg 3 仪器

- mcp: status(exit0) → adopted(exit0) → apply(exit0) → adopted(exit0) → adopted(exit0)
- cg : codegraph init c-adopt-then-extend-cg | codegraph explore -p c-adopt-then-extend-cg control_object static_graft_plan C | codegraph affected -p c-adopt-then-extend-cg src/control/object/toggle/toggle.
- mcp 关键行: log-mcp-fc.jsonl: 5 行全 exit 0；`adopted` 三次（改前/加面后/全部改动后）
- cg  关键行: log-cg-fc.txt: 3 CMD / 3 EXIT，全 exit 0
- **判**: 我自核台账：两树 `.nichlink/adopted/entries` sha256 都是 `541cbe86…38a3`（337 B、1 行、被 git 跟踪），两树 `Cargo.toml` `8d6b35ed…`、`button.rs` `e842be74…` 与 cg 自报逐字吻合；两臂都**没有**伪造新条目、都写清了「新锚不是续期」⇒ **平手**（第六轮此题为 mcp 真赢，本轮两臂都守住了台账）

## fd — mcp 4 仪器 / cg 2 仪器

- mcp: status(exit0) → check(exit0) → check(exit0) → check(exit0)
- cg : codegraph init d-one-broken-cg | codegraph explore -p d-one-broken-cg render write_count entries count block Fi
- mcp 关键行: log-mcp-fd.jsonl: 4 行全 exit 0（3 次 `check`：改前/默认改后/特性改后）
- cg  关键行: log-cg-fd.txt: 2 CMD / 2 EXIT
- **判**: 门：两树 4 passed（默认）/ 5 passed（all-features）rc=0；`git diff --stat` 两臂都是 2 files, 2+/1− ⇒ **平手**，改动最小性相同

## fe — mcp 4 仪器 / cg 3 仪器

- mcp: status(exit0) → check(exit0) → check(exit0) → check(exit0)
- cg : codegraph init e-many-broken-cg | codegraph explore -p e-many-broken-cg audit_count non zero entries ledger repo | codegraph affected -p e-many-broken-cg crates/core/src/audit.rs
- mcp 关键行: log-mcp-fe.jsonl: 4 行全 exit 0
- cg  关键行: log-cg-fe.txt: 3 CMD / 3 EXIT
- **判**: 门：两树默认 4 passed / 特性面 5 passed rc=0；两臂都修 `audit.rs:9` 与 `query.rs:27`（2 files, 2+/2−）⇒ **平手**

## g1 — mcp 2 仪器 / cg 4 仪器

- mcp: check(exit0) → check(exit0)
- cg : codegraph init s3 | codegraph callers -p s3 audit_unused | codegraph callers -p s3 audit_count | codegraph callers -p s3 bucket_name
- mcp 关键行: log-mcp-g1.jsonl: 2 行全 exit 0（`check --face default --census true` + `--face audit` 复核）；我复现其读数：`test-reachable: 1 of 12` + `no test reaches audit_unused (crates/core/src/audit.rs:14)`
- cg  关键行: log-cg-g1.txt: 4 CMD / 4 EXIT；`callers audit_count` 的输出第 22-26 行只列 `audit_unused`
- **判**: 两臂都点名 `audit_unused`（audit.rs:14）同一处，且都写了「这一栏看不见什么」（动态分派/函数指针/FFI/宏展开、按名字匹配、测试形文件、非覆盖率）⇒ **平手**（cg 另报了 `callers` 漏掉 `tests/audit.rs:25` 的实测盲点）

## g2 — mcp 4 仪器 / cg 2 仪器

- mcp: check(exit0) → read(exit0) → read(exit0) → callgraph(exit0)
- cg : codegraph init s5 | codegraph explore -p s5 render write_count count block order the_report_prints
- mcp 关键行: log-mcp-g2.jsonl: 4 行全 exit 0
- cg  关键行: log-cg-g2.txt: 2 CMD / 2 EXIT
- **判**: 两臂的伪代码都是**意图**（表头 → 条数 → 分录，各有 render.rs:5-6 / write_count / 用例三处依据），差都指向「`render` 从不调用 `write_count`」，且都注明 query.rs 那处是另一处 ⇒ **平手**

## g3 — mcp 5 仪器 / cg 3 仪器

- mcp: status(exit0) → apply(exit0) → apply(exit0) → registry(exit0) → usages(exit0)
- cg : codegraph init g3-cg | codegraph explore -p g3-cg Button ControlHandle paint interior registry tree f | codegraph affected -p g3-cg src/control/object/button/button.rs
- mcp 关键行: log-mcp-g3.jsonl: 5 行全 exit 0；改 `src/control/control.rs`（NodeId `fb97ddd5…` 前后不变、faces 3）
- cg  关键行: log-cg-g3.txt: 3 CMD / 3 EXIT；改 `src/control/object/button/button.rs`（150+/1−，含 3 条文件内单测 → 30 passed）
- **判**: 三条机械判据我自己跑：`cargo run --example tree` 两树都 **3 行**；改动面 mcp 只有 1 个文件 / cg 只有 1 个文件且测试文件未动；门两树两面 rc=0（27 / 30 passed）。两臂都给出「我没动什么」的可核凭据（mcp：NodeId/faces/usages/git status；cg：source_scope.tsv 与 generated_lib.rs 的 sha256）⇒ **平手**

## g4 — mcp 2 仪器 / cg 5 仪器

- mcp: status(exit0) → check(exit0)
- cg : codegraph init g4-cg | codegraph explore -p g4-cg band_word default_band state_word Band Frozen arms  | codegraph callers -p g4-cg band_word | codegraph callers -p g4-cg state_word | codegraph callers -p g4-cg default_band
- mcp 关键行: log-mcp-g4.jsonl: 2 行全 exit 0；分支级栏原文「2 constructively unreachable arm(s) … (1 `false` guard(s), 1 never-constructed variant(s))」（我用冻结快照复现，逐字相同）；答案列 `bands.rs:36` 与 `bands.rs:54`，明说 `bands.rs:42` 与 `bands.rs:91`（State::Dormant）判不了
- cg  关键行: log-cg-g4.txt: 5 CMD / 5 EXIT；`callers band_word|state_word|default_band` 三次都 `ℹ No callers found`（cg 把它作为盲点写进答案）
- **判**: 按答案键逐条核：**必列两条**两臂都给（bands.rs:36 常量假守卫 / bands.rs:54 私有 `enum Band` 的 `Band::Frozen`）；**必不列的五处**（bands.rs:42、store.rs:22、buckets.rs:8、query.rs:22、bands.rs:92）两臂都没**列进来**——cg 提到 `State::Closed/Dormant`（bands.rs:91/92）是在「我判不了」一栏里，正是答案键要求的边界自报；两臂都自报边界（数据相关不判、宏展开/动态派发/FFI 不可见、非覆盖率量度）⇒ **平手**

## h1 — mcp 4 仪器 / cg 3 仪器

- mcp: status(exit0) → check(exit0) → check(exit0) → check(exit0)
- cg : codegraph init h1-cg | codegraph explore -p h1-cg audit_count render write_count Filter min_amount le | codegraph affected -p h1-cg crates/core/src/audit.rs
- mcp 关键行: log-mcp-h1.jsonl: 4 行全 exit 0；答案给全树总账（12 生产函数/18 索引函数/2 face）并按 7 栏逐栏处置，额外修了常量栏（`buckets.rs:10` 改读 `SMALL_LIMIT`）= 4 files, 5+/3−
- cg  关键行: log-cg-h1.txt: 3 CMD / 3 EXIT；答案给 3 crate / 6 测试目标的总账表，修 3 files（含特性面 `audit.rs:9`）
- **判**: 门我自跑：h1-mcp 4/5 passed、h1-cg 4/5 passed，两面 rc=0；**换面才现的那处（`crates/core/src/audit.rs:9`）两臂都被处置**（cg 的 `git status` 含它，mcp 的答案③点名它）⇒ **平手**（mcp 多处置了 census 的常量栏）

