# nichlink MCP 优化执行记录（按《nichlink-mcp 完整执行清单（2026-10-04）》）

本文件是那份执行清单在本检出里的**执行记录**：每一项的证据、改法、验收怎么测、实测结果与提交。
执行清单本身（用户的附件）是入口；这里只记"做没做、怎么自证的"。

写法定：交付给维护者的 markdown **中文为主**，代码标识符、路径、命令、测试名保留英文原文。

## 状态总览

| 阶段 | 项 | 状态 | 提交 | 验收怎么测 |
| --- | --- | --- | --- | --- |
| 0-1 | bench 进仓 | **完成**（规模 bench 新写；hardbug 与 S1–S7 已确认可用） | 本地脚本（`.py` 不入库） | 见 §阶段 0 |
| 0-2 | 四轴回放固化 | 待做 | — | 见 §阶段 0 |
| 0-3 | 基线落盘 | **完成**（五档曲线，实测到 t4） | `1156794` | 见 §阶段 0 的曲线表 |
| M1 W4-1 | F1 显示名漂移 | **完成** | `e21d6f3` | `consistency_tests::a_displayed_field_name_is_the_spelling_the_declaration_wrote` |
| M1 W4-2 | F6 路径类校验 | **完成** | `aadf372` | `apply_tests::a_logical_path_is_refused_by_a_typed_plan_with_the_spelling_to_use` |
| M1 W4-3 | F8 离线脚手架 | **完成** | `5281de5` | `scaffold::project::tests::a_request_names_the_source_and_git_is_never_guessed` + `tools/nichlink-external-rehearsal` |
| M1 W4-4 | 拒绝消息实例化 | **完成**（adopt 指针待 W5-1） | `23aa8c3` | `apply_tests::a_refusal_carries_a_request_built_from_this_trees_names` |
| M1 W4-5 | 闭合标志 | **完成** | `d9510f6` | `consistency_tests::the_answer_opens_with_the_census_and_closes_conservatively` |
| M1 W4-6 | 离群行原文片段 | **完成** | `25ab03b` | `consistency_tests::an_outlier_carries_the_lines_it_deviates_on` |
| M1 W4-7 | 答案形状路由 | **完成** | `b53439c` | `client_tests::the_answer_shape_routing_is_on_both_pages` |
| M1 W1-2 | status 并入首行 + next 实例化 | **完成** | `d9510f6` | `consistency_tests::the_answer_opens_with_the_census_and_closes_conservatively`、`why_tests::the_answer_carries_the_census_and_a_closure_line` |
| 门禁 M1 | 整仓门禁 + hardbug 电池 | 见 §门禁 M1 | — | — |
| M2 W1-1 | 广告收敛 2+1 | **完成** | `03e427f` | `protocol_tests::the_advertised_frame_and_the_handshake_fit_their_budgets` |
| M2 W1-4 | 描述节食 | **完成** | `03e427f` | 同上（三条描述 + 三张 schema 的字节预算） |
| M2 W2-4 | apply 面清单有界化（D1） | **完成** | `6a605b8` | `apply_tests::the_write_reply_is_bounded_and_full_buys_the_tree_back` |
| M2 W5-7 | 写后响应节食（D4） | **完成** | `0d8fcd6` | `a_deepen_adds_a_layer_inside_the_face_and_prices_the_other_reading` |
| M2 W5-8 | new_project 自带初始普查 | **完成** | `0d8fcd6` | `new_project_tests::the_creation_reply_carries_the_initial_census` |
| M2 W1-3 | cargo 输出瘦身 | **完成**（边界索引合并已是现状，本次做分组折叠） | `15ab15b` | `check_tests::the_passing_groups_fold_and_verbose_undoes_it` |
| M2 W2-1 | registry 默认档普查化 | **完成** | `40cf6ca` | `registry_tests::the_default_is_a_census_whose_size_follows_the_levels` |
| M2 W2-2 | consistency 免责文本会话化 | **完成** | `0785094` | `consistency_tests::the_bounds_are_said_once_per_session_and_root` |
| M2 W2-5 | apply 成功回复节食 | **完成**（D2+D3） | `6a605b8` `0785094` | 见 `apply_refusals_tests::the_write_disclaimer_is_said_once_per_session_and_project` |
| M2 W2-6 | `--list` 瘦身 schema | **完成**（口径已裁：默认有界、按需无界） | `9f06025` | `client_tests::every_discovery_page_is_bounded_with_a_way_to_the_whole_text` |
| M2 W2-7 | `--list --all-slim` | **完成** | `9f06025` | `client_tests::the_thirteen_tool_view_is_one_small_call` |
| M3 W3-1 | 记录增肥三列 + 读写成对 | **完成** | `63ab9ec` | `manifests_tests::the_record_carries_the_source_hash_fields_and_calls_a_derivation_agrees_with`、`every_published_column_reads_back` |
| M3 W3-2 | 全读工具记录优先 | **进行中**（发布路径的普查/分页与声明事实已接上，端到端已验） | `8869a69` | `published_tests::a_current_record_is_shown_and_its_levels_are_counted` |
| M3 W3-1b | 记录补 `parent_node`/`owns_registry` | 待做 | — | 见 §阶段 M3 的下一步 |
| M3 W3-3 | 单一来源评审规则 | 待做 | — | — |
| M4 | W5-1 / W5-2 / W5-3 / W5-4 / W5-5 / W5-6 | 待做 | — | — |
| M5 | W6-1 / W6-2 | 待做 | — | — |

## 阶段 0：测量地基

### 0-1 bench 进仓（规模 bench 已新写；两套既有 bench 确认可用）

执行清单说"`scale-logs/gen_scale_tree.py`、`scale-logs/scale_bench.py` 移入 `tools/`"。**这两个文件
在本检出不存**（`scale-logs/` 目录不存在，全盘也没有这两个名字）⇒ **新写**，不是搬家：
`tools/gen_scale_tree.py`（按面数生成一棵规模题树：直接写 `src/`，不走 `new_project`/`apply`——生成器
造规模，被测的是**读**那条路；生成的清单带空的 `[workspace]` 表，否则 Cargo 拒绝给包命名）与
`tools/scale_bench.py`（五档 `t1:100 / t2:1000 / t3:5000 / t4:20000 / t5:50000`，每个读数 = 一次
`--call` 的墙钟毫秒与回复字节，取 `--repeat` 次中位数，落 `scale-logs/scale-bench.json`）。

**注意**：按维护者 2026-10-04 的指示 `*.py` 不入库，因此这两个脚本**只活在本地**；它们的**口径**写在
本文件与脚本头部（可据此重建），而不是靠"仓库里有那个文件"。

另外两套既有 bench 已确认可用：`tools/nichlink-mcp-hardbug`（四类题配方 + 旁路 tokens 账 + `report`；
本轮 `build` 四类各建一棵并**自证**成功）与 S1–S7 夹具（`build-s*.sh` / `write-briefs.py` /
`score-s*.py` / `axes2.py`）。

### 0-3 基线落盘：规模曲线（2026-10-04 实测）

`tools/scale_bench.py --binary target/debug/nichlink-mcp --all --tiers t1:100,t2:1000,t3:5000
--repeat 2`（本机，debug 二进制；中位数）：

| 档 | 面数 | `registry` 默认 | `registry --full --limit 200` | `status` | `search` | `read` | `why` |
| --- | --- | --- | --- | --- | --- | --- | --- |
| t1 | 101 | 317ms / **863 B** | 339ms / 13.8 KB | 142ms | 334ms | 71ms | 390ms |
| t2 | 1001 | 2923ms / **866 B** | 3099ms / 26.9 KB | 874ms | 2966ms | 51ms | 2927ms |
| t3 | 5001 | 14102ms / **866 B** | 14365ms / 26.9 KB | 3773ms | 14233ms | 51ms | 14517ms |
| t4 | 20001 | **55706ms** / **869 B** | 56389ms / 27.0 KB | 14907ms | 57892ms | 74ms | 58829ms |

**两条读数**：

1. **W2-1 的"拍平"成立** ✓：默认 `registry` 的**字节数**在 101 / 1001 / 5001 面上是 863 / 866 / 866
   ——它是普查（计数 + 逐层），不是清单；`--full` 也在 26.9 KB 封顶（200 行一页）。清单要求"50k 档
   registry 默认 ≤2 KB"，按这条曲线它由**层级数**决定而与面数无关。
2. **读路径是 O(面数)/次调用** ✗✗：`registry`/`search`/`why`/`status` 每次都重走一遍 `src/` 的推导，
   317ms → 2.9s → 14.1s → **55.7s**（≈2.8ms/面，线性）⇒ **t4 实测 55.7s**、外推 50k ≈ 140s，
   与清单里"t4 读延迟 211s"是同一个现象（本机更快，量级一致）。只有 `read` 是平的（51ms，它只读一个文件）。**这就是 M3（记录路径）要消灭的东西**，而它现在
   有了一台可复现的量器与一条基线。

四轴预算基线（写入 175,959 / 未命中 17,683 / 命中 36,226 / 输出 9,824 / 成本 291,967）来自四轴估算
§3，那份文档不在检出里，因此要在两臂重跑时**从会话日志重算**（方法已有：`tools/nichlink-mcp-hardbug
tokens <session>` 或 `target/bench-scenarios/axes2.py`，语料是那 12 个会话）。

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

## 阶段 M2：经济性主刀（进展）

### W1-1 广告收敛 2+1（完成，`03e427f`）

**证据**：`tools/list` 一帧交出全部二十七个工具（约 36,000 字符散文）；`initialize` 交出的长指引页约
7,000 字符。那一轮量到这两个数字正是"第一个决定很贵"的来路（25 步 vs 对照工具的 14 步）。

**改法**（**不删任何能力**）：新增 `tools::advertised()`，`tools/list` 只广告 `nichlink.check` /
`nichlink.apply` / `nichlink_tools`；`tools()` 仍是完整目录（28 项），`--list`、`--list <tool>`、
分派、归属分类都读它。`nichlink_tools` 不给参 ⇒ 与 `--list` **同一份实现**的一行式清单
（`client::list_tool_lines`）；`tool: "<名字>"` ⇒ 与 `--list <tool>` **同一份实现**的整页
（`client::describe_tool`）——广告与手册各只有一份实现，因此不可能漂移。长指引页改名 `GUIDANCE`
（`--shapes` 仍印它，内容一字未删）；`INSTRUCTIONS` 变成握手用的短文本。

**实测**（线上回复的形状）：广告帧 ≤ **4,200** 字节、三张广告 schema 合计 ≤ **4,300**、
`INSTRUCTIONS` ≤ **550** 字节；目录仍有 **28** 项（≥27，一个能力没少）。

**命名注**：`nichlink_tools` 按清单原文。它不是 `nichlink.*` 命名空间里的能力，而是"关于目录本身"的
那一个，因此一行式清单的钉子放行这一个例外（`first.starts_with("nichlink.") || first.starts_with("nichlink_tools — ")`）。

### W1-4 描述节食（完成，`03e427f`）

**改法**：广告位工具各带一份**只留决策信息**的描述（check ~470 / apply ~430 字节）与一张**只留决定
这次调用的键**的 schema。细则没有被删：完整描述与完整 schema 仍在 `tools()`（`--list <tool>` 印）、
拒绝文案仍带实例化形状（W4-4）。`apply` 的广告 schema 里也补上了 `full`（W2-4 新键）与
`cut`/`graft`/`to`/`full`（此前完整 schema 里**根本没有**这四个键——一处承诺面落后于能力面的旧缺陷）。

**口径冲突（记下来，等维护者或下一轮裁）**：W1-4 说"细则全文迁入 `--list`"，而 W2-6 说"`--list` 返回
瘦身 schema"。两者指向同一个出口的两种形态。当前实现站在 W1-4 那一边（`--list <tool>` 仍是完整描述），
W2-6/W2-7 因此标"待定"而不是"完成"——不把没做的说成做了。

### W2-4 apply 面清单有界化（完成，`6a605b8`）

**证据**：每次写入印出**整棵树的每个面**。九面夹具上是几行，五万面的树上就是整份载荷。

**改法**：默认回复 = 普查行 `faces N` + 发生变动的那个面（`changed  <path>  <kind>  <source>`）
+ `truncation::withheld` 扣下行（出路 `full: true`）；`full: true` 买回全量。`full` 同时进广告 schema
与完整目录 schema。**实测**：预览 ≤4.4K、落盘 ≤2.2K 字节（钉子直接量 `reply.len()`）。

### W2-5 apply 成功回复节食（D2 完成，D3 仍开，`6a605b8`）

**D2**：`editable_fields_line` 是**拒绝**的辅助，却搭在每条成功回复上。成功回复删掉，拒绝路径照旧带。
那条"每次回复都要带形状"的旧钉子按新意图改写，并且**两半都断言**（成功里没有、拒绝里有）——否则
"搬走了"与"没了"看起来一样。

**D3（仍开）**：`consequences` 的固定块（entry plan 行 + "not covered" 免责）**会话化**——同会话第二次
`apply` 不再含全文。它需要进程级会话状态，而桥同时有长期 stdio 服务与 `--call` 一次性两条入口；
"什么算同一个会话"要一个设计决定，因此留到 M2 收尾时定，而不是先写一个可能错的实现。

### W5-7 写后响应节食（完成，`0d8fcd6`）

`deepen` 的 `alternative` 与 `note` 两块是**决策辅助**，属于"决定还没做"的那一刻：预览给全文，落盘改印
一行 `alternative was stated in preview; …`。验收两半（预览含全文、落盘不含）都钉在同一条测试里。

### W5-8 new_project 自带初始普查（完成，`0d8fcd6`）

落盘/预览回复新增 `tree: 1 package, N rust file(s); … M of them own a registry — \`registry\` would
say the same thing`，用的是写入路径**刚刚**读过的那批源码（`load_sources` + `face_views`）⇒ 零额外
推导，消灭了"写后立刻再发一次 `registry`"那一步。

### 尚未开始（M2 余项）

`W1-3` cargo 输出瘦身 · `W2-1` registry 分页/普查化 · `W2-2` consistency 免责文本会话化 ·
`W2-6`/`W2-7`（口径待裁） · 决策点 `DP-1`/`DP-2`/`DP-3`（要门禁实测才裁，不许预设）。

### 修 W1-1 自己带出来的两处（`4f5cf85`）

真跑一遍才发现的，**两条钉子当时都放行**——记下来是因为这正是"钉子钉的是那句话，不是那个能力"的老毛病：

1. **`--call nichlink_tools` 解析不到**：命令行对每个裸名补 `nichlink.` 前缀，而目录工具没有点号
   ⇒ 解析成 `nichlink.nichlink_tools` ⇒ 广告让会话"用它去够其余一切"的那个工具在命令行上回
   `unknown tool`。修法：`client::resolve_name`——**精确命中目录名者胜**，只在需要时补前缀；三处解析
   都改走它。钉子两半都钉（新名字可达、`callgraph` 仍解析成 `nichlink.callgraph`）。
2. **预算量错了对象**：原钉子量 tools 数组的 serde 序列化，而客户端付的是**完整回复行**（含
   JSON-RPC 信封，实测 4,245）。现在钉子走真 stdio 入口量那一行；据此把两条广告描述各削一句，
   实测线上那一行 = **4,194 ≤ 4,200**。

**可复用两条**：**(a)** "预算量在哪个形状上"必须与"客户端付费的形状"一致——量数组不等于量回复；
**(b)** 新广告出去的名字必须**自己可达**，钉子要钉"那个名字真的能调到"，不是"那句话真的写了"。

## 下一轮从这里接着做（本轮的收尾状态）

**已闭合**：阶段 0 的可测部分 + 阶段 M1 全部 + M2 的 W1-1 / W1-4 / W2-4 / W2-5(D2) / W5-7 / W5-8。
**当前门禁**：`cargo fmt --check`、`cargo test --workspace --offline`、两面 `clippy -D warnings`、
`tools/nichlink-publish --check-table`、`cargo test -p nichlink-conventions`（含 600/800 棘轮）
全绿；`tools/nichlink-test` 十面在修掉 studio 那条旧断言后应全绿（下次整跑复核）；`tools/nichlink-
external-rehearsal` 绿（W4-3 的验收）。

**明确仍开**（不许说成做完了）：

| 项 | 为什么还开着 |
| --- | --- |
| 门禁 M1 的两臂项（hardbug 命中 10/10、S1–S7 零用法拒绝） | 需要 LLM 臂；与 M2 的四轴重算**合并成一次**（理由见 §门禁 M1） |
| W1-3 cargo 输出瘦身 | 未动 |
| W2-1 registry 默认档普查化 | 未动；动手前要先决定"包根默认是否一律普查"——它会改动 `registry_tests` 的全部期望形状，值得单独一轮 |
| W2-2 consistency 免责文本会话化 | 未动；与 W2-5(D3) 同一个"什么算同一个会话"的设计决定 |
| W2-5 D3 | 见上 |
| W2-6 / W2-7 | 与 W1-4 的口径冲突（"细则迁入 `--list`" vs "`--list` 返回瘦身 schema"），先裁口径再动手 |
| DP-1 / DP-2 / DP-3 | 要门禁实测才裁，不许预设 |
| M3 / M4 / M5 | 未开始（W5-1 `adopt` 按清单要求"单独设计评审后动工"） |

## 阶段 M2 收尾（本轮补完）

### W1-3 cargo 输出瘦身（`15ab15b`）

**第一半已是现状**：三条边界索引早已并成一条 `SHORT_BOUNDARY`（`not covered: the reachability
column (…) · the branch column (…) · this census (…)`），默认只带它，逐栏全文只在 `census: true`
时出现——如实记，不重复做一遍。

**第二半动手**：一次 `cargo test` 每个测试二进制品一组，每组印自己的 `test result:` 行。现在**带失败的**
逐行印（上限 12，超出走 `truncation::withheld`），**通过的**折成
`result N group(s) passed — pass \`verbose: true\` for each line; first: …`；`verbose: true` 全印。
schema 两处同步。

### W2-1 registry 默认档普查化（`40cf6ca`）

默认 = 普查（`faces N` + 逐层计数，最多 20 层，超出 withheld）+ 一行"`full: true` prints the rows,
200 per page and `offset: <n>` for the next"；`full: true` = 行，一次一页（`rows 1-200 of N` +
游标，`limit` 1–200）。**拍平是承重的**：普查大小跟**层级数**走、不跟面数走（实测 4 面 vs 120 面，
答案长度只差 <40 字节，两者都 ≤2 KB）。工作区的 `full` 仍走合并视图（跨成员的页索引会是同一棵树的
第二套坐标）。原"整份清单"渲染器被分页路径取代并删除。

### W2-2 + W2-5(D3) 常量块会话化（`0785094`）

**"什么算同一个会话"这个设计决定先定下来**：单位是**服务一个客户端的那一个进程**——stdio 服务活一条
连接，"已经说过"= "已经对这个客户端说过"；一次性 `--call` 是它自己的进程，因此永远是第一次（它没有更早
的答案可回指）。新增 `mcp/src/session.rs`（进程级 `static`，键里带根 ⇒ 两棵树各有自己的第一次，测试的
临时目录不会互相污染）。接上两处常量块：`consistency` 的两支共用 `consistency-bounds:<root>`；
写入路径的 `consequences` 用 `apply-consequences:<project>`。

### W2-6 + W2-7 发现载荷（`9f06025`，口径冲突已裁）

**裁法**：`W1-4`（"细则迁入 `--list`"）与 `W2-6`（"`--list` 返回瘦身 schema"）同时成立的读法是
**默认有界、按需无界**——长文仍住在 `--list`（没搬走），而默认那次调用只为"要不要调这个工具"付费。
实现：`describe_tool` 把描述压在 250 字节，截断句**必须**走 `truncation::withheld`（本仓有一条钉子钉
这句话的唯一拼写——我第一版手写了一句，当场被它抓住）；外加一条**自适应规则**：只有省下 ≥44% 才收窄
这一页（"短十分之一、却多花一次调用"比长一点但能作答更糟）。全文：`--list <tool> --full` /
`nichlink_tools {tool, full: true}`。实测默认/全文（字节）：why 429/759 · digest 330/578 ·
check 343/4847 · apply 433/4579 · search 466/2282 · consistency 435/1329 ⇒ 每次截断都 ≤60%。
`W2-7`：`COMMON_TOOLS`（七种形状入口的并集，13 件）+ `slim_tools_page()`，由**同一份** `short_lines`
渲染（不可能与 `--list` 各说各话），末尾一行说出其余 15 件在哪；CLI `--list --all-slim`、桥
`nichlink_tools {slim: true}`。

### M2 余下

`DP-1`/`DP-2`/`DP-3` 与门禁 M2 的四轴重算都要**两臂实机**数据，仍按 §门禁 M1 的决定合并成一次跑。

## 阶段 M3：记录路径（进行中）

### W3-1 记录 schema 增肥（完成，`63ab9ec`）

**先核对现状再动手**：`pruning_manifest.tsv` 的表头已经是
`# node  source  symbol  path  kind  registry_name  parent` —— 前四项事实 2026-09-29 就有了
（只是磁盘上 `examples/control-button/target/…` 那份是旧构建留下的三列表，一度让我以为没做）。
缺的是 **变化探测与调用面** 那三样。

**改法**（只落构建期**已经算出**的东西）：`FaceColumns` 新增
- `source_hash` = 该面自己那份源码字节的 `sha256_hex`；
- `fields` = 声明字段指纹（按内核 `FACE_FIELD_ORDER` 枚举、`name=raw` 排序去重后散列，**并把 `kind`
  折进去**——它是身份输入且不总被写下来，只散列"写下来的东西"会把字段相同、kind 不同的两个面判成相等）；
- `calls` = 该文件直接调用的名字（**内核自己的 `direct_calls` 规则**，与桥里每个读者同一条；无调用写
  `-`）。

**读写成对**：`PruningRow` 同步新增三列并在 `read_pruning_manifest` 里读回——只写不读等于发布了没人
够得着的事实（"承诺面落后于能力面"的另一个方向）。旧的三列表照旧解析、缺列读作 `None` 而不是报错。

**钉子**：表头即契约；哈希与调用名用读者会用的**同一批内核函数重算对账**；指纹查长度与两次构建间稳定；
写入→读回逐列相等。

### W3-2 全读工具记录优先（进行中；本轮只做到"记录能被重建出来"）

**对着代码核到的现状**：`Member::tree()` 已经有 `Tree::Published` 分支，`PublishedTree` 也已经带着
`faces: Vec<PruningRow>`（含 `path`/`kind`/`registry_name`/`parent`，本轮又加了 `source_hash`/
`fields`/`calls`）——**发布路径本来就在**，缺的是"据它构造出读者要的东西"。

**顺手修掉一处自述落后**：`workspace.rs::derived_tree` 的文档说"记录不携带 `path` 与 `kind`"——
那句从 2026-09-29 起就不成立，本轮再假一层。已改成事实：记录仍**不**携带的是把 `FaceView`
**构造出来**所需的两样——解析后的父级 **`NodeId`**（记录发布的 `parent` 是声明拼出的 Rust 路径）与
`owns_registry`——以及"构建之后新增的面只在源码里"。

**W3-2 的下一步（按依赖排序，动手前先看这一段）**：
1. **W3-1b**：记录再补两列——`parent_node`（解析后的父级 `NodeId`；构建期拓扑已经算出）与
   `owns_registry`。有了它们，`FaceView` 才能**从记录构造**，而不是只能填空。
2. **`registry` 的普查/分页先走记录**（它是规模 bench 里最慢的一个，t4 55.7s）：普查只需要 `path` 逐层
   计数——连 `FaceView` 都不必构造。新鲜度用 `source_hash` 对现读字节比（比整棵树的解析便宜一个量级），
   记录不新鲜就回落并声明。答案要带 `tree published from <记录路径>`。
3. `status` / `search` / `why` 随后；注意它们慢在 **`load_sources` 遍历源码**（函数与调用面），不是慢在
   面的推导——记录里没有函数级数据，那半边要另想办法（`function_manifest.tsv` 只有「面 → 符号」）。
4. `check`/`verify` **永远 derive**（清单红线），不接记录。

### W3-2 第一刀：发布路径也普查，并**显示记录本来就携带的**声明事实（本轮）

**对着代码与真构建设施核到的三件事**（都写进注释与本节，免得下一轮再猜）：

1. **记录真的会被写出来**：把 `examples/control-button` 的 build script 逼着重跑（`touch build.rs`）后，
   新写下的 `pruning_manifest.tsv` 表头就是十列，`source_hash`/`fields` 是 64 位十六进制真值，
   `calls` 是内核规则算出来的。**它落在 cargo 的 `OUT_DIR`**（`target/debug/build/<pkg>-<hash>/out/`），
   而**桥读的是 `<包根>/target/nichlink/out/`**（`check`/CLI 刷新的位置）——同一棵树的两份记录住两个地方，
   这一点以前没写下来过。本轮把新鲜的那份拷到桥读的位置，验到端到端。
2. **发布路径过去既无界又少用**：`registry` 在已构建的树上每个面印一行、**没有分页**（W2-1 的普查只做在
   推导那支），而且它的注里写着 `path`/`kind`/`registry_name`/`parent`"are derived facts and are not in
   the record"——**这句从 2026-09-29 起就不成立**（剪枝行一直在发布它们）。这是一句**落后于行为**的自述。
3. **`path` 常常是 `-`**：示例的三个面都没声明 `path`（逻辑路径是宏在运行期派生的），因此普查的层级只能
   按**源码目录**数，答案里明写 `this record predates the \`path\` column` 之外还多一句
   `level (source directory)`——两条坐标不混在一张计数表里。

**改法**：`render_published` 接上 `full`/`offset`/`limit`（与推导那支同一套分页常量与
`truncation::withheld` 出路）；默认给**普查**（层级计数，按已发布 `path`，缺失时按源码目录并明说）；
`full: true` 给**分页的行**，每行附记录携带的 `path=`/`kind=`/`registry_name=`/`parent=`/`calls=`；
`source_hash` 与 `fields` **不逐行重印**（64 位十六进制 × 每行 = 一屏噪声），注里说清它们发布在
`pruning_manifest.tsv` 里、以及新鲜度是 `check` 的问题。注重写成两半：记录**携带**什么、读者**还要**推导
什么（构建之后新增的面）。

**端到端证据**（拷入本轮 build script 真写出的记录后，`--call registry --full` 对示例树的输出）：
`kind=Slider/Control/Button`、`parent=crate::control::NODE_ID` **直接来自记录**——这些事实过去被那句话
否认，读者只能自己推导。

**钉子**：`published_tests::a_current_record_is_shown_and_its_levels_are_counted`（默认是普查且按**已发布
的**逻辑路径计数、默认不印行；`full` 页显示五个声明列与 `calls`，`calls=-` 的情形也钉住；注里说出两个
哈希住哪）、`records_that_do_not_describe_these_sources_are_reported_stale`（注的两半都断言）。

### W3-1b 记录补 `parent_node` / `owns_registry`（完成，本轮）

**为什么需要这两列**：`FaceView` 的 `parent` 是 `NodeId`，而记录发布的 `parent` 是**声明拼出的 Rust
路径**（`crate::control::NODE_ID`）——把那段文字变成身份是读者本要自己做的工作，也是"能不能从记录构造
`FaceView`"的分界。`owns_registry` 同理（它决定"这个面是不是文件夹面"）。

**改法**：`write_pruning_manifest` 变成**两遍**——第一遍索引「每个面拥有的模块路径 → 该面的 `NodeId`」
（子面可能先于它点名的父面被访问到，因此解析不能在遍历途中做），第二遍逐行解析父级身份。解析器用的是
**构建自己的** `face_view::resolve_parent` / `parent_of`（本轮把这两个从私有提到 `pub(crate)`，而不是
在清单写入方里另写一份"这条声明命名了什么"——那种拷贝正是会漂移的东西）。解析不出时写 `-`（树不拥有的
模块、解析不了的 `parent`），并明说 `-` 的读法是"推导"。

**端到端证据**（真 build script 写出的十二列表头 + 前两行）：

```
# node	source	symbol	path	kind	registry_name	parent	source_hash	fields	calls	parent_node	owns_registry
bdb4427c…	control/object/slider/slider.rs	-	-	Slider	-	crate::control::NODE_ID	981ae775…	c760ca90…	-	fb97ddd5f2b803d1b7f40a776d8a22d9	false
fb97ddd5…	control/control.rs	-	-	Control	-	-	620a2d06…	57a3a780…	-	b6a6bea94077152dbb7dd780a2708acf	true
```
slider 的 `parent_node` **正是 control.rs 自己的 `node` 列**（子面的解析父级＝父面的身份 ✓），而 control.rs
的 `owns_registry=true` 与它声明的 `needs_registry: true` 对得上。桥里 `registry --full` 现在直接印
`owns_registry=true|false`（读记录，不推导）。

**钉子**：`manifests_tests::the_record_carries_the_source_hash_fields_and_calls_a_derivation_agrees_with`
（表头十二列即契约；`parent_node` 查形状）、`a_module_named_parent_resolves_to_its_identity`（**两种形状**
都走：按模块点名的父级 ⇒ 与父面自己的 `id` **逐字相等**；点名树里没有的模块 ⇒ `None` 而不是猜）、
`every_published_column_reads_back`（读写成对）、`published_tests::a_current_record_is_shown_and_its_levels_are_counted`
（发布行显示 `owns_registry`，注里点名 `parent_node` 发布在文件里）。

**一处如实记**：`parent_node` 的期望值我第一版硬写了 `env!("CARGO_PKG_NAME")`，而写入方用的是
`package_namespace()`（运行期读环境）⇒ 两个不同函数，测试先红。改成"与另一行自己的 `id` 比"（不需要
测试侧做任何命名空间算术）——与第十轮那条"两边必须用同一个函数"同族，本轮又踩一次。

### 本轮：把"已构建的树"造出来并实测（W3-2 的验收形状）

**为什么要造**：M3 的验收是"**built 树**全部读工具 p95 < 3s"，而此前的五档曲线都是**未构建**的树（走推导）。
`tools/gen_scale_tree.py` 因此新增 `--host`：额外写 `build.rs`、`lib.rs` 里的 `host!()`、指向本检出的
`path` 依赖 ⇒ 这棵树真的能被 `cargo build`。

**造的过程本身抓到两个坑**（第一版模板只满足**读者**、不满足**编译器**，`cargo build` 报 203 个错）：
① 面文件的模板必须照 `new_project` 写出的形状（用桥脚手架一个真两面上机、把字节读回来抄），**不能**从
"推导解析得过"出发自己编；② `ROOT_FACE` 是普通字符串不是 f-string，我按 f-string 写了 `{{`，`rustc`
读成字面双括号 ⇒ 改成单括号。

**实测（101 个面，同一棵规模树，构建 vs 未构建，中位数）**：

| 工具 | 已构建 | 未构建 |
| --- | --- | --- |
| `registry` 默认 | **70.4ms / 1,966 B** | 278.8ms / 867 B |
| `registry --full` | 70.2ms / 21.2 KB | 245.3ms / 13.9 KB |
| `status` | 86.8ms | 96.6ms |
| `search` | 70.4ms | 258.9ms |
| `read` | 44.5ms | 41.1ms |
| `why` | 203.9ms | 263.5ms |

⇒ **记录路径在 101 面上已经把 `registry`/`search` 拉到 70ms 量级**（3–4×）；`read` 两支都平（它只读一个
文件）。**但发布普查比推导普查大**：2,824 → 1,966 B（修剪后），因为记录里 `path` 列是 `-`（宏派生的逻辑
路径不写进声明）⇒ 发布普查只能用**源码目录**做层级，而本夹具每个面各占一个目录 ⇒ "一面一层"。为此把
`CENSUS_LEVELS` 20 → 10 并把发布注从 734 B 收到 ~340 B，最坏情形回到 **1,966 B ≤ 2 KB**（W2-1 的预算）。

**由此得到 W3-2 的下一条具体活**：记录应当发布**逻辑路径**（构建期已经算出——`face_view::logical_path`
沿父链走），而不是只发布声明里写的 `path`；否则"按层级计数"这件事在记录侧永远比推导侧粗一档。

**顺带修掉一处并发缺陷（本轮真被它咬到）**：会话账本原先是**进程级** `Mutex`，而 `cargo test` 在一个进程里
**并行跑线程** ⇒ `the_bounds_are_said_once_per_session_and_root` 只在全量跑时红（另一个测试的
`session::forget()` 把它的键擦了）。改成 **thread-local**：stdio 服务本来就在调用它的那个线程上读并作答，
因此生产语义不变，而测试各自一个线程、互不干扰。连跑三遍 565 项全绿。
