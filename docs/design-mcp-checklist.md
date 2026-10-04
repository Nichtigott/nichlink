# nichlink MCP 优化执行记录（按《nichlink-mcp 完整执行清单（2026-10-04）》）

本文件是那份执行清单在本检出里的**执行记录**：每一项的证据、改法、验收怎么测、实测结果与提交。
执行清单本身（用户的附件）是入口；这里只记"做没做、怎么自证的"。

写法定：交付给维护者的 markdown **中文为主**，代码标识符、路径、命令、测试名保留英文原文。

## 状态总览

| 阶段 | 项 | 状态 | 提交 | 验收怎么测 |
| --- | --- | --- | --- | --- |
| 0-1 | bench 进仓 | 进行中 | — | 见 §阶段 0 |
| 0-2 | 四轴回放固化 | 待做 | — | 见 §阶段 0 |
| 0-3 | 基线落盘 | 待做 | — | 见 §阶段 0 |
| M1 W4-1 | F1 显示名漂移 | **完成** | `e21d6f3` | `consistency_tests::a_displayed_field_name_is_the_spelling_the_declaration_wrote` |
| M1 W4-2 | F6 路径类校验 | **完成** | `aadf372` | `apply_tests::a_logical_path_is_refused_by_a_typed_plan_with_the_spelling_to_use` |
| M1 W4-3 | F8 离线脚手架 | **完成** | `5281de5` | `scaffold::project::tests::a_request_names_the_source_and_git_is_never_guessed` + `tools/nichlink-external-rehearsal` |
| M1 W4-4 | 拒绝消息实例化 | **完成**（adopt 指针待 W5-1） | `23aa8c3` | `apply_tests::a_refusal_carries_a_request_built_from_this_trees_names` |
| M1 W4-5 | 闭合标志 | **完成** | `d9510f6` | `consistency_tests::the_answer_opens_with_the_census_and_closes_conservatively` |
| M1 W4-6 | 离群行原文片段 | **完成** | `25ab03b` | `consistency_tests::an_outlier_carries_the_lines_it_deviates_on` |
| M1 W4-7 | 答案形状路由 | **完成** | `b53439c` | `client_tests::the_answer_shape_routing_is_on_both_pages` |
| M1 W1-2 | status 并入首行 + next 实例化 | **完成** | `d9510f6` | `consistency_tests::the_answer_opens_with_the_census_and_closes_conservatively`、`why_tests::the_answer_carries_the_census_and_a_closure_line` |
| 门禁 M1 | 整仓门禁 + hardbug 电池 | 见 §门禁 M1 | — | — |
| M2 | W1-1 / W1-4 / W1-3 / W2-1 / W2-2 / W2-4 / W2-5 / W2-6 / W2-7 / W5-7 / W5-8 | 待做 | — | — |
| M3 | W3-1 / W3-2 / W3-3 | 待做 | — | — |
| M4 | W5-1 / W5-2 / W5-3 / W5-4 / W5-5 / W5-6 | 待做 | — | — |
| M5 | W6-1 / W6-2 | 待做 | — | — |

## 阶段 0：测量地基

### 0-1 bench 进仓

执行清单说"`scale-logs/gen_scale_tree.py`、`scale-logs/scale_bench.py` 移入 `tools/`"。**这两个文件
在本检出不存**（`scale-logs/` 目录不存在，全盘也没有这两个名字）⇒ 规模 bench 的生成器与量器要**新写**，
不是搬家。已在 `target/bench-scenarios/` 确认可用的是另外两套：`tools/nichlink-mcp-hardbug`
（四类题配方 + 旁路 tokens 账 + `report`）与 S1–S7 夹具（`build-s*.sh` / `write-briefs.py` /
`score-s*.py` / `axes2.py`）。

### 0-3 基线落盘

清单给的基线是四轴估算 §3（写入 175,959 / 未命中 17,683 / 命中 36,226 / 输出 9,824 / 成本
291,967）。那份文档不在检出里，因此基线要**从会话日志重算**——方法已有（`tools/nichlink-mcp-hardbug
tokens <session>`，或 `target/bench-scenarios/axes2.py`），语料是那 12 个会话。

## 阶段 M1：信任清零

### W4-1 F1 显示名漂移

**证据**：`consistency` 把两个 trait 字段固定报成 `handle_traits`/`part_traits`，而内核接受
`handle_contracts`（参与编译检查的路径）与 `handle_traits`（纯标签）**两种**拼写 ⇒ 一个写
`handle_contracts: [crate::control::ControlHandle]` 的面，读者拿到的是另一个名字，而两者的取值住在
各自的位置上。

**改法**：`ShapeField` 同时带**规范键**与**拼写清单**；比较按键（同族选另一种拼写不算漂移），
`shape` 行与 `lacks` 行点名**文件真正写着的**那个拼写；`shape` 信号的存在性集合按键归并；
`shape_field_names()` 把两种拼写都列出来。

**全仓同类普查**：逐个字段 grep 展示层（`handle_traits`/`handle_contracts`/`part_*`/
`registry_rule`/`stable_name`/`needs_registry`/`runtime_checks`/`flow_provider`/
`getting_from_other_registry`），命中处只有 `apply` 的 `EDITABLE_FIELDS`、`usages` 的 21 栏回读、
`explain`/`digest` 的字段行——它们点的是**可编辑字段名**（`name_zh`/`registry_rule` 等，能原样回写
`apply`），只有 `consistency` 这一处点名了源码里不存在的拼写。⇒ 改一处即闭合。

**验收**：H2 题树重跑，`consistency` 每个可引用名可原样用于 `apply edit` 且编译绿。**本仓自证**：
新钉子两个方向都走（标本写 contracts ⇒ 显示 contracts、且兄弟写 traits 不算缺；兄弟真缺 ⇒ 行里给的
仍是标本的拼写），并对**每一种拼写**查内核词表 `FACE_FIELD_ORDER`。

### W4-2 F6 路径类校验

**证据**：`apply {action:"cut"}` 接受读者手上的逻辑路径 `root/control/dial`，并把它渲染成
`cut(root/control/dial)` 拼进一份其余每条都是 `crate::…::NODE_ID` 的计划。两种拼写在这套语法里
含义不同（逻辑路径由**构建期选择器**解析，类型化那一形由**编译器**解析）⇒ 读回来的条目不是调用方
要的那条，而写进去的是宿主的源码。

**改法**：`apply_cut::path_class_refusal` 在渲染之前、写盘之前检查输入；只在计划**通篇**类型化时拒绝
（反方向会拒绝语法本就接受的请求）。`typed_spelling` 从面自己的 `source` 经**构建自己的**
`build_time::source_module_path` 推导拼写——不从逻辑路径拼装，因为 `object` 那一层只有源码布局知道。
推导不出时说"这棵树里没有这个面"并指向 `registry`。

**验收**：喂 `root/control/dial` ⇒ 拒绝且消息含 `crate::control::object::dial::NODE_ID`（并给出
来源文件）；喂正确拼写 ⇒ 一次成功；被拒那条路上什么都没写。三条都在钉子里走了一遍。

### W4-3 F8 离线脚手架

**证据**：不在检出内运行的工具会往生成清单里写 `git = "https://github.com/…"`——一个需要联网才能解析
的来源，而调用方从没要求过。

**改法**：`DependencySource::Registry`（只写版本要求，`cargo add` 会写下的那一形）；
`detected_source` 的回落从 `Git` 改成 `Registry`，从检出内跑仍是 `path`（唯一完全离线可解析的一形）。
词表收进 `scaffold::requested_source(dependency, git, …)`：`path`/`registry`/`git`；只给 URL 而没说
`dependency: "git"` ⇒ 拒绝；`path` 而没有检出 ⇒ 拒绝并指向 `registry`，不静默换源。桥的
`new_project` 接受 `dependency`/`git` 两个键（schema 同步）；CLI 的 `--git` 照旧显式。

**验收**（清单口径：断网沙箱 `new_project` → `cargo build` 绿）：走 `tools/nichlink-external-rehearsal`
的第三条腿——它在检出之外用本检出的 CLI 生成二进制与库两个项目并各跑 `cargo test --offline`。

### W4-4 拒绝消息实例化 + 边界文档化

**证据**：写入工具的拒绝都点了形状，但用的是占位符（`<node>`/`<snake_case>`/`<Kind>`）⇒ 读者还得自己
找合法父面、想一个不撞的模块名、把切口的类型化拼写推出来，而这三件都是**请求所在那棵树**的事实。
S1/S6/S7 上的用法型拒绝正是这么来的。

**改法**：`apply::write_example(root, namespace, action)` 按动作搭完整请求（`parent` 取第一个拥有
注册机的真实面、`module` 取树里没人用的名字、`node` 取真实面路径）；`cut` 示例的 `cut` 半取自
`typed_spelling`、`graft` 半取自计划**已经在用**的拼写（`apply_cut::plan_graft_example`，经内核
解析器读回）；`promote` 的 `selector` 取本包真实的 `external-grafts` 目录。接进 `apply` 的
缺/错 action、`add`/`edit`/`rename`/`delete`/`deepen`/`cut` 的缺键支、`promote` 的缺
selector/confirm 支、`new_project` 的三个必填键支。`apply` 的**描述**（客户端真正拿到的那份）写明
分界：add/deepen/cut/promote 是**追加**类（改声明，能落到手写面上）；edit/rename/delete 是**改写**类
（改写文件，只落生成过的面）；执行器的 `this module was not generated by NichLink` 拒绝也附同一句。

**仍开的一半**：清单要的那句"指向 adopt（W5-1）"**还没写**——`adopt` 这个动作不存在，现在写就是承诺面
超过能力面。等 W5-1 落地后补，并同时补一条钉子。

**验收**：S1/S6/S7 重跑用法拒绝 = 0（待合并重跑）；agent 仅读描述即可预知放行/拒绝（本仓自证：钉子直接
读 `tools()` 交出去的 `description`）。

### W4-5 闭合标志

**证据**：debug 电池上两个方向都量到——一个停在一处偏离、对其余只字不提的答案让代理猜要不要继续；
一个总带 `next` 行的答案让每个答案看起来都没做完。

**改法**：`tools::closure(closed, remaining)`——只说两种之一，判定**从保守**：closed 只在"这个答案
自己的形状会提出的每个问题都已被回答"时写；任何读不了的文件、任何被点名的偏离都是 open，并附上解决它
的命令（命令是**可粘贴的 `--call` 拼法**，已用真二进制验过旗标与数字解释）。`consistency` 两支
（specimen/parent）与 `why`（主答案 + "这一行不在任何函数里"那条早退）都接上。

### W4-6 离群行原文片段

**证据**：只点名字段 ⇒ 读者还要再开一次文件（多一次仪器调用）；对照工具对同一个问题回整份文件
6,781 字符的片段。

**改法**：离群者**自己有**的那行先引（`lines_naming`，行首带 `路径:行号`）；缺字段那种引不出行的
情况用它的**声明块**代替（`declaration_block`，从宏调用到收尾大括号）；随后引标本（多数派）的对位行。
父面 `api`/`shape` 信号同理：本兄弟自己多调/多声明的引它自己的文件，别人都有的那个名字引**多数派写它的
地方**。上限 `EXCERPT_LIMIT = 10` 行/离群者，单行 `LINE_LIMIT = 200` 字符后截断带 `…`；答案的边界句
里写明"引的是摘录不是文件"。

### W4-7 答案形状路由

**证据**：两种问题形状要两种答案（理解型＝"这个文件是干什么的"由整份文件回答；结构型＝"里面有什么、
谁调用它"由有界答案回答），而这条规则只活在设计文档里。

**改法**：`SHAPES_SHORT`（`--list` 真正印的那页）与 `INSTRUCTIONS`（`initialize` 交的那页）各加一条
`**Answer shape**` / `**答案的形状**` 路由句；`locate` 的 `next` 行换成**排名第一那个文件**的
`--call read --path <file> --line <n>` 与 `--call callgraph --function <name>`，并新增 `shape` 行
指向 `--call read --path <file> --whole`。`--list` 总长仍在 5000 字符预算内（实测 4,991）。

### W1-2 status 并入首行 + next 实例化

**改法**：`consistency::tree_census` 给出 `tree: N face(s), M file(s)`，作为 `consistency`（两支）
与 `why` 的**首行**（`check` 本来就有 `tree   N rust file(s), M function(s)`）⇒ "先发一次 `status`
只为知道树有多大"这件事不必再发生。`why` 的 `next` 换成这个答案已经知道的名字（`--call check --face
<面>`、`--call callgraph --function <函数>`、`--call read --path <路径> --line <行>`），没有归属面时
退到 `--call registry` 而不是印 `{face}`。

## 门禁 M1

| 门禁 | 怎么跑 | 结果 |
| --- | --- | --- |
| 格式 | `cargo fmt --all -- --check` | 绿 |
| 工作区测试 | `cargo test --workspace --offline` | 绿（默认面 151 项 conventions 全绿；`size` 棘轮先红后修，见下） |
| clippy 默认面 | `cargo clippy --workspace --all-targets --offline -- -D warnings` | 绿 |
| clippy all-features 面 | `cargo clippy --workspace --all-targets --offline --all-features -- -D warnings` | 绿 |
| 发布表 | `tools/nichlink-publish --check-table` | 绿（`dependency table matches the manifests (3 crates)`） |
| 十面测试 | `tools/nichlink-test` | **先红后绿**：四个面各一项失败，全是 `studio::…::installed_studio_never_exports_cargo_git_cache_paths`——它还在断言 F8 之前的行为（见下），已在 `cd1ff1b` 跟上 |
| 生成物编译 | `tools/nichlink-external-rehearsal` | 绿：`external rehearsal passed: control-button built and tested outside the checkout (CLI half: ran; scaffold half: ran)` ⇒ **W4-3 的验收**（检出之外生成的两个项目 `cargo test --offline` 都过） |
| 棘轮 | `cargo test -p nichlink-conventions` | 绿（M1 把 `apply.rs`/`consistency.rs`/`apply_tests.rs` 顶过 600/800 ⇒ 按本仓规矩拆出 `apply_refusals.rs` / `consistency_support.rs` / `apply_cut_tests.rs`，见 `5db85cc`） |
| hardbug 电池可用 | `tools/nichlink-mcp-hardbug build <dir> <class>` ×4 | 四类各建一棵并**自证**：h1/h3 红→绿（`cargo test` 101→0）、h2 由 `consistency --specimen` 点名、h4 普查点名 `ZeroArm::Refuse` 第 44 行 |
| hardbug 电池命中 10/10 | 两臂作答 | **合并到 M2 门禁**（同下） |
| S1–S7 零用法拒绝 | 两臂重跑 | **合并到 M2 门禁**（理由见下） |

**两处"先红后绿"如实记**（都不许拿"两臂都完成了"掩盖）：

1. `size` 棘轮被 M1 自己的改动顶红 ⇒ 拆模块（`5db85cc`），不是加基线。
2. `installed_studio_never_exports_cargo_git_cache_paths` 是**被 W4-3 正确的行为变化打红的旧断言**：
   它断言装出来的副本写 `git = "…"`，而 F8 的修法正是让那一支写已发布版本要求。断言跟着行为走并更强
   （两条都不含 `git =`、不含本机路径），提交 `cd1ff1b`。

**关于把两臂重跑合并进 M2 门禁**：维护者的规矩是"一轮轮测试很贵，做到当前设计的最好再去测试"，而 M2 的
门禁本来就要"四轴重算 12 会话语料"。同一套夹具连着跑两遍，第二遍只是把第一遍的数字复述一遍。因此 M1 的
机械门禁照跑，**两臂的实机重跑只在 M2 门禁做一次**，两阶段的用量从那一次里各取所需（M1 取"用法型拒绝
= 0"，M2 取四轴）。
