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
| M3 W3-2 | 发布路径记录优先（普查/分页 + 显示声明事实 + `logical_path`） | **完成** | `8869a69` `cfef385` `a751612` | `published_tests::{a_current_record_is_shown_and_its_levels_are_counted, a_published_answer_and_a_derived_answer_carry_the_same_facts}` |
| M3 W3-1b | 记录补 `parent_node`/`owns_registry` | **完成** | `6693bda` | `manifests_tests::a_module_named_parent_resolves_to_its_identity` |
| M3 W3-3 | 在线分析评审规则（门禁 `online_analysis`） | **完成**（21 个桥文件各写了理由） | 本轮 | `conventions::online_analysis_tests::every_online_answer_says_why_the_build_cannot` |
| M4 W5-2 | affected 补计划层 | **完成** | 本轮 | `affected_tests::a_changed_face_names_the_plan_entries_that_target_it` |
| M4 W5-4 | consistency 升级修复器入口 | **完成** | 本轮 | `consistency_tests::the_repair_the_comparison_hands_back_clears_the_outlier` |
| M4 W5-5 | check 失败按类型路由 | **完成** | 本轮 | `check_tests::a_red_routes_by_its_kind_rather_than_to_the_default_hint` |
| M4 W5-3 | deepen 预览附消费方 diff | **完成** | 本轮 | `apply_consumers_tests::the_consumer_story_is_the_same_in_the_preview_and_after_the_write` |
| M4 W5-6 | apply 树驱动一次成型 | **大半**（① 从树推家族契约 + ③ 写后家族判定已落地；②"一次生成完整文件"本已如此） | 上轮+本轮 | `apply_consumers_tests::{a_write_reports_whether_the_new_face_landed_like_its_siblings, an_omitted_family_field_is_inherited_and_reported}` |
| M4 W5-1 | adopt（单独设计评审后动工） | 待做 | — | — |
| M5 W6-1 | 编译墙归因 | **完成**（报告 + 10k<5min 实测达标） | `22c882f` | 见 §阶段 M5 |
| M5 W6-2 | 中大型项目参考结构 | **进行中**（验收已实测；四步已落，下一步＝兄弟集合走记录，家族分层仍开） | `c41bac3` `6fb4e42` `460eafe` `36a107b` | 见 §阶段 M5 |

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

## 阶段 M3：记录路径（**已完成**；下表保留逐项证据）

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

### W3-2 全读工具记录优先（发布路径已完成；余项见 §阶段 M5）

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

### W3-2 第二刀：记录发布**逻辑路径**（完成，本轮）

**上一轮实测指出的正是这一条**：记录里 `path` 是声明拼出的东西，对每个宏派生的面都是 `-`，于是发布
普查只能退到**源码目录**计数（一面一层）。而逻辑路径**构建期早就算出来了**——`face_view::logical_path`
沿解析后的父链走。

**改法（复用，不复制）**：
- `face_view::resolved_registry_name(face, module)` 与 `face_view::logical_paths(root, names)` 提为
  `pub(crate)`，**推导视图与清单写入方调同一对函数**——否则"注册名的回退规则"（省略时取模块名末段）
  会有两份拼写，而对同一个问题给出两个答案正是本仓最不许的事。
- `write_pruning_manifest` 现在**三遍**：① 索引「模块 → 面 id」；② 用**完整**的索引算
  `(id, 父级 id, 注册名)` 三元组；③ 逐行解析并发布。**第二遍必须独立**——子面可能先于它点名的模块被
  访问到；我第一版把解析塞在索引那趟里，钉子当场抓到：`parent:` 点名稍后访问的模块的子面拿到
  `root/child` 而不是 `root/dial/child`（注释里把这件事记下来了）。
- 新增第十三列 `logical_path`；`PruningRow` 同步读回；发布行也印 `logical_path=`；普查的层级回退链是
  **逻辑路径 → 声明路径 → 源码目录**，并且**说出用的是哪一个**。

**端到端（真 build script + `verify`，101 面）**：

```
… owns_registry  logical_path
… false          root/control/child000088
```
```
level (logical path prefix) and how many recorded faces sit under it:
  root                                         1 face(s)
  root/control                                 100 face(s)
```
⇒ 与推导侧**形状完全一致**（同一棵树两支都给出 `root 1 / root/control 100`），发布普查从 2,824 B 降到
**1,386 B**（此前那次"压到 1,966 B"是靠修剪行数与注长度换来的；现在形状本身对了，预算不是靠削出来的）。
已构建 101 面的默认 `registry` **88.7ms**（未构建 278.8ms）。

### W3-2 第三刀：把 M3 的"published vs derive 一致"做成钉子（完成，本轮）

**先把口径读清楚**：清单的验收写"t2 级对照答案与 derive **逐字节一致**"，而同一份清单又要求发布答案声明
`tree published from …`、推导答案说出它推导了——两条路**不可能**逐字节相同，**而且这是要求而不是缺陷**。
因此本仓把 M3 门禁的"答案一致"读作**事实**一致：总数行与逐层计数行。这个口径读法已写进钉子文档。

**改法**：新增 `published_tests::a_published_answer_and_a_derived_answer_carry_the_same_facts`——
同一棵树先按已发布作答、再把两份记录移走按推导作答，比对**事实行**（`faces N` 与层级行），并
`assert_ne!` 两份整答复（逼"必须说出自己来自哪里"这件事留在答案里）。

**它当场抓到一件事**（这正是这枚钉子的价值）：我手写的夹具记录里 `logical_path` 写的是
`root/control/slider`，而同一份源码推导出来是 `root/slider`（那两个面都声明父级为**根**）⇒ 发布普查说
`root 1 / root/control 1`、推导普查说 `root 2`。夹具的历史值错了，已改。**手写的记录可以编码一个推导
会反驳的事实——只有两边对账才发现。**

**101 面规模上的同一次对账**（已构建的 `h1` 树，记录的发布普查 vs 把记录移开后的推导普查，取事实行）：

```
diff <(registry 已构建 | grep -E '^faces |^  ') <(registry 推导 | grep -E '^faces |^  ')
→ 无差异（AGREE）
```
两条路都给出 `faces 101` + `root 1 face(s)` + `root/control 100 face(s)` + 同一句汇总行 ⇒ **事实逐行相同**，
而整答复不同（差在 `tree published from …` / `tree derived now …`、`freshness`、`scope` 与发布注）。

**门禁 M3 的另一半**（t4 读延迟 211s → <3s）目前只有"未构建 t4 = 55.7s、已构建 101 面 = 88.7ms"两个点；
要把五档都变成已构建的树需要每档都编译（t4 = 两万面，属 M5 的编译墙），因此这一半与 M5 一起量。

### W3-3 在线分析评审规则（完成，本轮）

清单的原文是"任何 PR 在 MCP 层新增在线分析必须回答『为什么构建期不能算』"——它是一条**评审**规则。
本仓的规矩是"新增全仓规则请加到 `conventions`，而不是加到散文文档里"，因此落成门禁
`conventions::online_analysis`：

- **触发条件**是"摸到源码的那几个调用"（`derived_faces(` / `derived_tree(` / `load_sources(` /
  `face_views_with_external(`），而不是"分析"这个看不见的词；清单住在一处，扩它只需改一行。
- **回答**写在模块文档里：`//! online: <为什么构建算不了>`，且散文至少 30 字符（那是**下限**而不是
  裁判——门禁判不了理由是否成立，它做的是让这个问题在唯一能回答它的地方无法回避，好让评审有一条可以反驳的
  句子而不是一片沉默。`W3-3` 要的正是这个）。
- **边界明说**：只判桥的生产那一半（文件里第一个 `#[cfg(test)]` 之前，且从不判 `*_tests.rs` 兄弟文件）
  ——推导的测试是**关于**推导的测试。
- **21 个文件各写了自己那句**：读源码文本的（`source_index`/`callgraph`/`impact`/`usages`/`why`/
  `search`/`digest`/`claims`/`diff`/`affected`/`locate`/`consistency`/`consistency_support`/`check`）
  说"构建发布的是面，不是调用图/不是源码文本，而这些答案是关于源码**此刻**的样子"；需要整份面的
  （`resolve`/`workspace`/`grafts`/`apply_refusals`/`new_project`/`tools`/`ownership`）说"构建之后
  新增的面根本不在记录里"。**不是 21 份同一句话**。

钉子：三种合成形状（写了理由 ⇒ 放过；什么都没写 ⇒ 报出并带 `file:line` 位置；敷衍一句 ⇒ 仍报）+ 测试那一半
不判 + 遍历覆盖桥（>40 文件）+ **对全检出断言 0 findings**。

**顺带记两处先红**：① 我把 `pub mod online_analysis;` 插在了 `#[path = "purity.rs"]` 与
`pub mod purity;` **之间** ⇒ purity 门禁的源码被当成 `online_analysis` 编译了一遍（`--list` 里出现
`online_analysis::purity_tests::*` 才暴露），修法是把属性与它的条目还在一起；② 我的 `//! online:` 行
紧跟在一条 markdown 列表项后面，`clippy::doc_lazy_continuation` 判定它是列表延续 ⇒ 在标记前补一行空
`//!`（更可读，也顺手解掉这一类）。

## 阶段 M4：反补丁感（进行中）

### W5-2 affected 补计划层（完成，本轮）

**改动**：`affected` 的每个改动文件行下面多一段 `plan:`——点名"该文件所声明的那个面"的 **graft 计划条目**
（按**选择器 + 它针对的逻辑路径**报告，不报它选中的实现：`affected` 回答"这次改动波及什么"，替换件是
`grafts` 的问题）。

**两处次序是承重的**：
1. **没有计划目录的成员一分钱不付**——先做 `.nichlink/external-grafts` 的存在性判断，因此从未有过外部 graft
   的树既不多一次推导、也不多一行字（用它把"计划层"塞进 `affected` 而不让它变成全树遍历）。
2. **两套坐标要认全**——`FaceView.source` 相对 `src/`，而 `affected` 的改动路径带 `src/` 前缀；原样比较
   会让每个面都像"没有名字"，而那是**静默漏报**（对一个确实被计划点名的面会说"没有计划点名它"）。

**钉子**：有计划且点名 ⇒ 报出选择器与逻辑路径；有计划但没点名 ⇒ **明说**"`none of the N entry(ies) here
names this face`"（沉默会被读成"根本没有计划"）；**没有计划目录的树一个 `plan:` 字都不多**。

**如实记：这个夹具我写错了两次，两次都是布局/身份写错、看起来像工具坏了。**
① 面放在 `src/shared.rs` ⇒ 构建的遍历**一个面都找不到**（面必须住在 `<dir>/<name>.rs`）；② 计划的
`target` 我按叶名写成 `shared.rs`，而身份路径是**文件自己的相对路径** `shared/shared.rs`。两次都是
"夹具错了、被断言抓到"，不是工具错——这条留给以后写夹具的人。

### W5-4 consistency 升级修复器入口（完成，本轮）

**验收的读法**：清单要"一次调用给出可执行修复，执行后 outliers: 0"。因此这条不是"答复里多一句建议"，
而是**答复里直接给出可执行的请求**：多了一行 `majority   N of M sibling(s) carry this shape`（多数派就是
本比对认的基线），以及每个离群者下的 `fix` 行——**写入路径自己的 JSON 请求**（`{"action":"edit", …}`）。

**钉子把"可执行"当真**：测试不靠读一遍来判 `fix` 行，而是把工具自己打印的 JSON **取出来、交给写入路径
执行**，再问一次比对，断言 `outliers` 归零。一条点名错字段/错面/错值的修复行会让离群继续站着。

**三处只有做了才知道的事**：
1. **`edit` 按逻辑路径指认面**：键是 `node` 而不是 `face`。第一版给 `face`，写入路径当场拒绝并交回可接受
   形状（`{"action":"edit","apply":true,"fields":{…},"node":"root/control/button"}`）。修好之后那行还是
   "粘贴即用"的。
2. **`edit` 只重写生成的面**（`// generated-by=NichLink` 是硬边界，不是疏漏）：对手写的面交回 `edit` 请求＝
   交回一条**粘贴即失败**的行。因此分两种拼法——生成的给 JSON 请求，手写的给"在那个文件里加
   `parts: SliderParts`，或先 adopt"。两个方向都有钉子。
3. **`parts` 只比存在性、比值属于对象自己**：把标本的 `parts: ButtonParts` 抄进 slider 是一次**弄坏被修
   文件**的修复。判断走 `SHAPE_FIELDS` 的 `ShapeComparison::Presence`，取值用兄弟自己的 `<Kind>Parts`。
4. `GENERATED_MARKER` 从创作模块的私有常量**搬进 `kernel::lexicon`**（文本契约的家）：桥要问同一个问题，
   而第二个读取方只抄得到一个字面量——那是两份答案。搬的时候我把它插在了 `NICHLINK_DIR` 的**文档与条目
   之间**（`missing_docs` 当场报出），与上一轮 `#[path]` 那次是同一族错误：**插入必须落在被插入条目的文档
   *之上***。

**顺带满足 600 行棘轮**：新代码把 `consistency.rs` 顶到 649 行，把三个修复辅助函数（`repair_value` /
`repair_request` / `manual_repair`）移进 `consistency_support.rs`（形状词汇本来就住那里）。

### W5-5 debug 入口路由（完成，本轮）

清单要"check 失败按**失败类型**路由（坐标族→`consistency --parent`；产物缺席→`why`）"，验收是"dbg 电池
步数方差收敛到 1-2"。因此 `next_step` 从"一条固定提示"变成**按种类路由**，三种：

| 种类 | 判据（**观察到的**，不是猜的） | 路由 |
| --- | --- | --- |
| 产物缺席 / 构建的红 | 日志里**没有** `test result:` 行（`results == 0`） | `why {at: "<编译器点名的 path:line>"}` + `check {verbose: true}` |
| 坐标族 | 失败测试名或回复自己的 `why` 行里出现**这棵树自己的**族名词（内核的 `RuntimeCheckSpec::CoordinatesInViewport.name()`，不分大小写，另收 "coordinate"/"viewport" 两种散文写法） | `consistency {parent: "<该面的父级>"}` |
| 默认（断言的红） | 其余 | 那条实测会被采纳的字面检索（T-22 起由本次答复自己实例化） |

**为此 `Observation` 多带两个事实**：`failed`（失败测试名清单）与 `first_location`（日志里第一个
`--> path:line`）——两者都在**读日志那一次**顺便取到，因此路由不额外读盘。

**钉子**（`a_red_routes_by_its_kind_rather_than_to_the_default_hint`）三种都走：构建的红路由到编译器点名的
位置、且**不**落到断言检索（那里根本没有断言可搜）；坐标族路由到该面的**父级**比对；默认那一种保留原提示。
**一处刻意的负断言**：名字里凑巧提到 offsets、但回复的 `why` 行不含族名词的失败测试**不得**被当成坐标的红
——路由的信号是这棵树自己的词汇，而不是测试名的联想。

### W5-3 deepen 预览附消费方 diff（完成，本轮）

**改动**：每次 `deepen` 的回复（**预览与落盘都带**）多两行消费方说法——
`slots      the layer adds N filling option(s) inside \`Kind\`: …`（这一层提供的槽位填充选项）与
`consumers  K of N graft plan entry(ies) target this face or its subtree (…) — this layer leaves every
target and path alone, so those plans keep pointing where they point and gain the options above`。
消费方＝**从面自己的文件之外读它的东西**：针对它/它子树的 graft 计划条目，以及填充它零件层槽位的那些。
计划目录不存在的树只印 `slots` 一行（"没有消费方"不需要一句噪声）。

**验收的实质**是"预览不是广告"，钉子就照这个写：`preview` 与 `applied` 两条回复的
`slots`/`consumers` 行**逐字相等**。

**它当场抓到一处真缺陷（这枚钉子的价值所在）**：预览在**副本**里跑，而副本按设计**跳过 `.nichlink`**
（`preview::skipped_directory` 与 `target`/`.git` 同列）⇒ 我第一版在**工作目录**里读计划，于是预览说
"没有计划针对这个面"、落盘说"1 of 1 条针对它"——**正是"预览是广告"那个缺陷**。修法：两半各读各的树——
面取自工作树（被做深的那个面在那里），**计划取自项目**（入口在那里）。`deepen` 因此与 `promote` 一样
同时拿到工作目录与项目根，签名与注释都写明理由。

**顺带**：新代码把 `apply_tests.rs` 顶到 820 行代码行（超 800 的测试上限），把这条钉子拆进
`apply_consumers_tests.rs`（`package` 提为 `pub(crate)` 复用）。

### W5-6 apply 树驱动一次成型（本轮完成第 ③ 半：写后家族判定）

清单这条有三半：① 从树自行推导家族契约与模板；② 一次生成完整文件；③ 响应附 diff + 计划后果 +
**写后家族判定（`outliers: x of N`）**。本轮落的是 **③**（验收"零写后回查"的那一半）。

**改动**：`report()` 现在把**家族判定**搭在每条写入回复上（预览与落盘都有）——新增
`mcp/src/apply_family.rs`，它在**变更之后的树**上（预览＝副本）取"该面所在父级的孩子们"，按**形状信号**的
字段名集合跑 `consistency` 自己的多数派算术（`deviations`），给出：

```
family     4 sibling(s) under root/control; outliers: 1 of 4
  outlier     lever: declares `exports`, which no sibling declares; declares `handle_traits`, which
              no sibling declares — `consistency {parent: "root/control"}` prints the fix
```

**一条规则不是两条**：多数派算术用 `consistency::deviations`，形状词汇用 `consistency_support` 的
`declared_shape`/`one_face`/`Wording`（本轮把它们提到 `pub(crate)` 并重导出）——在这里抄第二份，就是
"outlier"在关于同一棵树的两份答案里开始有两个意思的来路。只有自己一家时不给判定（对空无一物的判定不是判定）。

**真机实测**（`/tmp/lever-drift` 上 `apply add`）：新面 `knob` 与三个兄弟一致，而前几轮实验留下的
`lever` 被点名为唯一离群者——判定跟着**父级**走，不是一条固定路径。

**钉子**：一致的面报 `outliers: 0 of 3`；多声明一个字段的面被点名 `outliers: 1 of 4` 并带上"哪个调用
交回修复"；判定跟着面落地的**父级**走（第二个根子面按根自己的孩子们判）。

**两处先红，都是我自己前几轮立下的门禁抓到的**（这正是它们的价值）：
- `conventions::online_analysis`（W3-3 立的）当场报 `apply_family.rs` "computes online without saying why"
  ——补上 `//! online: 判定说的是**写入刚留下的**那棵树：刚创建的面根本不在任何记录里，而兄弟可能自构建以来
  已变"；
- `conventions::size`（600 行棘轮）报 `apply.rs` 602→603 行 ⇒ 把打印搬进 `apply_family::verdict_lines`、
  把 `work` 绑定提到 `consequences` 之前。

**仍开**：① 从树推家族契约与模板（`add` 自动继承兄弟的形状，调用方不必逐字段拼）；② 一次生成完整文件；
验收里的 S4 ≤2 次仪器调用与 S6 = 预览→落盘 2 调用属两臂实机。

### W5-6 第①半：从树自行推导家族契约（完成，本轮）

**改动**：`apply add` 时调用方**没写**的家族字段，由**它正在加入的那一家**补上，并且回复**说出来**。
新增 `apply_family::complete_add_fields` / `inherited_fields`：取父级的孩子，按形状信号解析每个兄弟声明的
`exports` / `handle_traits` / `part_traits` 标签，取**多数派**（出现次数 > 半数）合成声明里那种
**逗号分隔**的拼写（渲染器按逗号切分并逐个加引号），填进请求并给出一行

```
inherited  exports = control.render from the family under root/control; pass it explicitly to override
```

**三条边界（都是有意为之）**：① **`parts` 从不继承**——它点名对象自己的零件类型，抄兄弟的值按构造就是错的
（与形状比对同一条规则）；② **显式给的值永远赢**，树是**补全**请求、从不推翻它；③ 父级的两种拼法都读
（顶层 `parent` 或 `fields` 里的），否则写了第二种的调用方会**静默地**完全得不到继承。

**同一处的两个先红**：`conventions::online_analysis`（W3-3）报新调用点缺 `//! online:`（补："判定的是写入
**刚留下**的那棵树"）；`conventions::size` 报 `apply.rs` 614 行 ⇒ 把 `W5-3`/`W5-6`/`W5-7` 三组注记整体搬进
新模块 `apply_notes.rs`（报告的职责是渲染执行器的事实；这三组是"关于调用方那个决定"的事实）。

**真机实测**（两个兄弟都声明了 `exports: ["control.render"]` 与 `handle_traits: ["ControlHandle"]`，再加一个
不写这两项的 `gamma`）：回复给出两行 `inherited …`，**写下的文件里带着它们**，而家族判定同时说
`outliers: 0 of 3`——W5-6 的两半合起来正好是"继承契约、然后确认家族干净"。

**钉子**：省略 ⇒ 继承**并报告**且文件里落成声明的拼写；显式 ⇒ 不覆盖、也不为该字段出继承行，而未写的另一个
字段仍然被继承。**W5-6 由此只剩验收里的步数（S4 ≤2 次仪器调用、S6 = 预览→落盘 2 调用）属两臂实机；"一次生成
完整文件"本就是这个动作的做法。**

## 阶段 M5：框架规模墙（W6-1 归因已启动）

### W6-1 编译墙归因（本轮交出报告；验收的 10k<5min 已实测达标）

**量法**（可复现，脚本是本地 `/tmp/wall.sh`，数据落 `scale-logs/compile-wall.json`）：用
`gen_scale_tree.py --host` 造一棵可构建的单 crate 宿主 ⇒ `cargo build` 预热 ⇒ `touch src/lib.rs` ⇒ 计时
`cargo build`（＝构建脚本 + rustc 编这个 crate）⇒ 再**直接把构建脚本二进制按 cargo 自己的
`CARGO_MANIFEST_DIR`/`OUT_DIR`/`CARGO_PKG_NAME` 跑一遍**并计时 ⇒ rustc 那一份 = 总 − 构建脚本；
`generated_lib.rs` 的字节数由构建产物读。

**实测（2026-10-04，debug 档，暖依赖缓存）**：

| 面数 | 总计 | 构建脚本 | rustc | 生成的计划 |
| --- | --- | --- | --- | --- |
| 103 | 1.56 s | 1.15 s（74%） | 0.41 s | 128 KB |
| 1,003 | 11.66 s | 9.69 s（83%） | 1.96 s | 1.21 MB |
| 4,644 | 59.78 s | 45.53 s（**76%**） | 14.25 s | 5.61 MB |
| 10,003 | 141.64 s | 103.89 s（**73%**） | 37.75 s | 12.05 MB |
| 1,003（**冷**，先 `rm -rf target`） | 91.12 s | — | — | 依赖编译的固定成本 ≈ 80 s |

**归因结论（与清单预设的方向相反）**：**墙不在宏展开/代码生成，在构建脚本自己**——发现 + 解析 + 渲染计划
+ 写 12 MB，10k 面时占 **73%**，且近乎线性；rustc 编这个单 crate 只占 27%，但**超线性**（0.41 → 1.96 →
14.25 → 37.75 s，末端 ≈ 每 4.6 倍面数涨 3.3 倍）。因此"对症"的杠杆是**构建脚本那一段**（发现/解析/写盘），
而不是让编译器少干活。

**验收**："10k 面单 crate 编译 <5min 的路径" ⇒ **实测 141.6 s（暖）**；把冷依赖那 ~80 s 加上约 222 s，
**两条都在 5 min 内** ✓。因此这一条**不需要**论证分包也不给分包模板——但有一个前提要写明：以上用的是本生成器
的**最小面**（`NoParts`、每面一个文件）。**清单里"4,641 面 15–18 min"用这棵树复现不出来**（4,644 面暖编译
59.8 s）——能把那个数字做出来的多半是更重的面形（每面带零件/契约体）或另一种调用方式（每个家族一次
`cargo test`）。**这是本轮如实记下的分歧，不是把它抹平**：按上面的归因，更重的面形会把成本压到 rustc 那一侧
（它是超线性的那一半），而本报告的杠杆结论只对"最小面"这一形状成立。

**仍开**：W6-2（中大型项目参考结构：workspace 分成员 + 家族分层规范 + 规模生成器验证树，验收 50k 文件
workspace 树 `registry`/`consistency`/`search` p95 <3s 且响应拍平）。

### W6-2 中大型项目参考结构（本轮：验收在 50k 面上实测，并修掉一处"响应不拍平"）

**造树**（生成器新增 `--workspace <成员数> <每个成员的面数>`）：`gen_scale_tree.py target/scale/w50
--workspace 20 2500` ⇒ **20 个成员 × 2,500 个面 = 50,080 个 `.rs` 文件**。成员是**普通包**（无 `build.rs`、
无 `[workspace]` 表——后者会让 cargo 看到多个工作区根并拒绝这棵树），因此发布记录**不需要编译器**：对每个成员
各跑一次 `verify`（只走管线）即可。

**实测（三跑，第三次的字节数；数据 `scale-logs/workspace-50k.json`）**：

| 工具 | 运行时间 | 响应字节 | 判定 |
| --- | --- | --- | --- |
| `registry`（默认普查） | 0.52 / 0.46 / 0.48 s | **1,961 B** | **达标且拍平**（五万面 1,961 B，而已构建的 101 面是 1,966 B） |
| `search --query child000001` | 23.5 / 24.4 / 24.0 s | 10,802 B | ✗ 约 24 s |
| `consistency --parent root/control --by shape` | 137.9 / 161.3 / 167.6 s | **2,808,408 B** | ✗ 约 2.5 min 且**完全没有拍平** |

**结论分两半**：
1. **`registry` 达标**——M3 的记录路径 + W2-1 的普查，在五万面上就是"平"的（这条验收最想看到的东西成立）。
2. **`search`/`consistency` 未达标，而瓶颈不是面的推导**：它们每次调用都要 `load_sources` 读一遍整棵树的
   源码（记录里没有函数级/调用级数据）——正是清单 W3-2 自己点名的那个余项。

**本轮顺带修掉一处真的"不拍平"**：`consistency` 的 `--parent` 模式**每个成员印一行**，五万面实测 **2,808,408
字节**；而本工具自己的承诺是"一次调用点名离群者"，默认印出每个成员与它自相矛盾。改为默认保留**家族行 + 离群者
+ 它们的原文片段**，其余扣下并附 `full: true`（与 W2-1 同一套 `truncation::withheld` 出路）⇒ 同一棵树同一
调用降到 **10,688 字节**（时间不变：成本在**读源码**，不在渲染）。钉子
`consistency_tests::the_member_rows_are_bought_not_given` 两个方向都钉（默认扣下且说出去哪儿买；`full: true`
全印）；既有那条 `the_api_signal_reads_each_siblings_own_calls…` 改成用 `full: true` 买下它要读的行。

**如实记一处正确性缺口**（不藏）：在这棵 50k 工作区上 `consistency` 答 `outliers: 0 of 0`——源码索引有
**每棵树的预算**、超了就跳过文件，于是兄弟集合是空的。它该单独立项，本轮只把它记下来。

### W6-2 续：`search`/`consistency` 那 24 s 与 183 s 花在哪儿（本轮量出归因；数据 `scale-logs/read-path-attribution.json`）

上一轮量到"`registry` 达标而 `search`/`consistency` 不达标"，本轮把**不达标那两个的时间归属**量清楚：

| 工具 | 103 文件 | 2,501 文件 | 50,020 文件 |
| --- | --- | --- | --- |
| `search --query child000001` | 0.104 s / 0.107 s（两棵树同规模） | — | **24.0 s** |
| `consistency --parent … --by shape` | 0.631 s | ~9 s（由 20 成员 183 s 折出） | **167.6 s** |

**归因**：两者都**与树里的文件数成线性**——`search` 0.104 s/103 文件 → 24 s/50,020 文件（约 480×，文件数 485×）；
`consistency` 0.63 s/103 → 9 s/2,501（约 14×，文件数 24×）。**原因不是面的推导，也不是响应渲染**（上一轮把成员行封顶
后字节数降了 266×、时间一点没动，已经把渲染这一侧排除），而是 `source_index::load_sources(root)` **每次调用都
遍历并解析根下每一个 `.rs`**，而这两个工具在看清"自己的答案要点名哪几个文件"之前就已经把它调用了。

**下一步（已定，下一轮动手）**：让成本跟着**问题的大小**走，而不是跟着树的大小走——
① `consistency` 的家族模式**只加载该家族的文件**（它从树上就知道兄弟是谁，不必先读整棵树）；
② `search` 改为**先按字面量预筛、只解析命中的文件**。
两条都不需要新记录格式，也不需要改渲染；它们直接打掉上表那条线性。

**但要如实说清它们够不到哪里**：本夹具的家族是**一个父级下 2,500 个兄弟**——那种家族里"点名/摘录每个兄弟"这件事
本身就是秒级，上面两条只省掉"读整棵树"那一份（本夹具约占 24 s），省不掉"读这个家族"。要让 `consistency` 在
**几千成员的家族**上也回到 3 s 以内，还需要第三件事：**把"每个面声明了哪些字段"放进记录**（记的是**声明级**事实，
不是现在记录里只有指纹的 `fields` 列），这样多数派与离群者的判定就能在记录上做，源码只用来给**离群者**取原文片段。
这三条（家族内定向读取 · search 预筛 · 声明级记录）是同一件事的三个梯度，按"先便宜后贵"推进。

### W6-2 三步走之第①步：`consistency` 的家族模式只读该家族（本轮，并**修掉一处真实缺陷**）

**改动**：`source_index::load_directories(root, dirs)`（只读列出的目录，不走整棵树）+ `consistency` 的
`--parent` 模式改为**在成员循环内**、按**该成员自己的根**读**这一家自己的目录**。原先那次整棵树读取是在
**工作区根**上做一次、再被所有成员复用。

**它同时修掉了一处真实缺陷（这才是本轮最重要的产出）**：用工作区根索引去匹配成员内 `src/…` 的相对路径，
永远匹配不上 ⇒ 在工作区模式下**兄弟集合是空的**，于是判定是 `outliers: 0 of 0`——**工作区上的答案是空的**。
现在同一棵树同一调用答 **`outliers: 0 of 2500`**（真的比了 2,500 个兄弟）。

**实测（50k 面工作区，同一调用）**：

| | 修前 | 修后 |
| --- | --- | --- |
| 时间 | 167.6 s | **151.5 s** |
| 字节 | 10,688 B | 10,775 B（仍拍平） |
| 判定 | `outliers: 0 of 0`（**空集合**） | `outliers: 0 of 2500` |

**并如实说清这一步够不到哪儿**：本夹具的家族是**一个父级下 2,500 个兄弟**，单独量一个成员（2,501 文件）仍要
**12.6 s**——省掉的是"读整棵树"那份，省不掉"读这个家族"。要让几千成员的家族也回到 3 s 以内，必须走第③步
（**声明级记录**：多数派与离群者在记录上判定，源码只给离群者取原文片段）。第②步（`search` 预筛）仍未动。

### W6-2 三步走之第②步：`search` 预筛（完成）

**改动**：`source_index::load_sources_matching(root, keep)` 把索引拆成两半——**读**（每个文件的文本，照旧全读）
与**词法**（函数、调用、分支事实，按 `keep(relative, text)` 决定）。`load_sources` 就是 `keep = |_, _| true`
的那一种，因此旧行为没变、也没有第二份实现。

`search` 两处各取所需：
- `--query`（匹配**名字**）：只在文本提到该名字的文件上词法——不在字节里的名字不可能是字节里的声明；**路径那一半
  仍然看到每个文件**（这是预筛绝不能碰的性质）；
- `--literal`（匹配**文本**）：每个文件的文本照读，**一个文件都不词法**——这一模式从不查看符号扫描，省下的正是它。

**实测（50k 面工作区，`search --query child000001`）**：**24.0 s → 14.6 s**，输出**字节完全相同**（10,802 B）；
单成员（2,501 文件）`--query` 0.587 s、`--literal` 0.422 s。

**安全性的钉子（这枚比"变快了"重要）**：`a_file_is_listed_by_path_even_when_its_text_never_mentions_the_query`
——夹具把查询词拼在一个文件的**路径**里、从不拼在**文本**里，于是预筛跳过它的词法，而**路径那一半仍必须列出它**。
一个悄悄缩小了文件清单的预筛能骗过"函数找到了"那类钉子，但骗不过这一条。

**仍开第③步**：**声明级记录**——本夹具一个父级下 2,500 个兄弟，家族自己的文件仍要读（单成员 12.6 s 量于
第①步那一轮），只有把"每个面声明了哪些字段"落盘、让多数派与离群者在记录上判定、源码只给离群者取原文片段，
几千成员的家族才可能回到 3 s 以内。它同时是 W3-2 余项的正解。

### W6-2 三步走之第③步（上半）：构建期发布**声明形状**记录（本轮）

**改动**：`write_shape_manifest` 落 `shape_manifest.tsv`——每个面、它按内核 `FACE_FIELD_ORDER` 拼出的每个字段
各一行（`source`、`field`、**声明拼出的原文**），一个什么都没声明的面写一行 `-`（要计数的读者必须看得见整个
家族）。读取方 `read_shape_manifest` → `ShapeRow`，与其它记录一样走 `face_view` 的重导出。

**关键设计决定（写进注释与钉子）**：这份记录携带的是**声明**而不是**结论**——按**源码路径**做键、原样保留
声明文本（例如 `["a" , "b"]` 而不是规范化成 `["a", "b"]`）。"它意味着什么"（哪些字段参与比较、怎么比）是桥的
`SHAPE_FIELDS` 词汇；在这里规范化就是那第二份会漂移的实现，也正是本仓最不许的事。**这一层分工也让内核保持纯净**：
记录里是构建期看见的字节，判据留在执行面。

**实测**（真 `verify`，一个 50 面的成员）：记录 104 行，表头 `# source	field	value`，例如
`control/control.rs  kind  Control`。

**钉子**：表头即契约；`exports`/`handle_traits`/`kind` 的**原文**都读得回来（并断言原文的原始间距，把"不规范化"
这条决定钉住）；非注册面文件根本不在记录里。

**这一步的下半（仍开）**：让家族比对**读**这条记录（并在读之前判新鲜度——陈旧的记录会给出错的多数派），
源码只用于给**离群者**取原文片段。那才是"几千成员的家族回到 3 s 内"的那一半。

### W6-2 三步走之第③步（下半）：家族形状读声明记录 + 源码惰性化（完成，**但如实说：这一形状上它不是那堵墙**）

**改动**：`consistency` 的 `--by shape` 在构建的**声明记录新鲜时**从 `shape_manifest.tsv` 读每个兄弟声明了哪些
字段（新鲜度走构建自己的 `build_output_is_current`，不另写一条规则）；兄弟源码改为**惰性**（`LazySources`：只有
`api` 信号或离群者取原文片段时才读）。本节明确说出用的是哪棵树：
`shapes from  member <m>: the build's declaration record (shape_manifest.tsv), current` ／
`the sources, derived now (no current declaration record)`。

**端到端验证**（真 `verify` 过的 50 面成员）：两条路都答 `outliers: 0 of 50`，而 `shapes from` 行不同 ✓。
钉子 `consistency_tests::the_declaration_record_answers_which_fields_a_face_declares`（记录按 `src/…` 做键、
"什么都没声明"映射到**空集合**而不是缺失条目、空记录回 `None` 让调用方回退）。

**但实测说这条在这一形状上不是那堵墙（如实记）**：同一个 **2,501 面成员**——记录路径 **11.07 s** vs 推导路径
**10.37 s**（惰性化之前是 12.4 vs 12.6）。而**同一个成员**上：`registry` **0.97 s**（它从剪枝记录取面）、
`search` **0.68 s**。⇒ **墙是 `member_faces` 的推导本身**（每个成员解析 2,500 个面文件），不是读源码、也不是渲染
（后者已在上一轮被"字节降 266× 而时间不动"排除）。

**顺带修掉一处二次复杂度**：`deviations`（多数派算术）过去对每个名字、每个兄弟都重扫一遍集合，是
O(sets² × names)；现在只数一遍（`name → 计数`）。它在这个形状上不是主项，但它在几千成员的家族上是数千万次
`contains`。

**下一步（已由上述对比定死）**：让**兄弟集合**也从记录取——即 `registry` 已经在用的那条 W3-2 规则（剪枝清单
新鲜时按它的行构面），推导只在记录答不了时付费。那才是让 `consistency` 在几千成员家族上回到 3 s 内的那一步。

### 追问"还有 schema 呢？"：广告面 vs 能力面（本轮，**当场抓到 3 处 + 1 条钉子**）

**方法（机械，不靠记忆）**：把 `tools.rs` 的 28 个目录条目逐块切开取"声明了哪些键"（**任意深度**的
`properties`，因为 `parts` 在 `inside` 之下、`cut`/`graft` 在某个 `anyOf` 分支里都算广告），再把
每个处理函数模块里**实际读的键**取出来（读的拼写只有两种：`arguments.get("X")` 与 `text/optional_text(arguments, "X")`），
两边对差。

**抓到的三处（都是"能力跑到广告前面"，也就是清单 W2-4 那一族）**：
1. `consistency.by` 的 `enum` 只写 `["api","kind","source"]`——**缺 `"shape"`** ✗，而 `shape` 甚至不是可选的，它是**默认**跑的两个信号之一；
2. `consistency` 的 properties 里**没有 `full`** ✗——而它正是那个被有界化的答案**唯一的出路**（读者根本发现不了怎么买回逐成员行）；
3. `mir` 的处理函数读 `against_trace`，schema 里没有它 ✗。

**修法**：三处补齐（`by` 的 enum 加 `shape`、补 `full` 与 `against_trace` 的描述），并把 `by` 的描述从
"`api` (default)" 改成事实（**省略即同时比 `api` 与 `shape`**）。

**钉子（有牙，已验）**：`tools_tests::every_key_a_handler_reads_is_advertised_and_every_entry_is_accounted_for`
——① 每个处理函数读的键都必须被 schema 广告（**任意深度**）；② `READ_KEYS` 表必须点名**每一个**目录条目，
新工具没人加进去就红；③ `by` 的 enum 不得比处理函数接受的信号更窄。**反向验证**：把 `full` 从 schema 删掉 ⇒
红并点名 `consistency reads \`full\` but its schema does not declare it`；把 `shape` 从 enum 去掉 ⇒ 红并点名
`by must offer \`shape\``；恢复 ⇒ 绿。

**如实记的两处过程事实**：① 我第一版 `READ_KEYS` 是**手写猜的**，跑出 8 条"缺失"里有一半是**表的错**
（`closure` 根本不是入参、`inspect`/`locate` 的键不在我以为的模块里）⇒ 改成**从代码机械导出**每模块真正读的键，
只剩 1 条待判（`conformance` 的 `files` 实为 `adopted` 的续期路径读的，已按事实归位）；② 全仓 `verb_table` 门禁
当场拦下我给它起的名 `collect_properties`（私有遍历动词必须是 `visit_`/`walk_`）⇒ 改名 `walk_properties`。
**仍开**：这条对差现在是"一手维护的表 + 机械比对"，下一步可把**工具→处理模块**的映射也从 `DISPATCH` 表机械导出，
让整件事零维护。

## 阶段 M6（终局收口）：按《nichlink 终局优化方向》再落一轮

### §3 离线出生证明（本轮）

**缺陷（如实复现）**：把二进制拷到检出外跑 `new_project`，清单写的是注册局依赖
`nichlink-toolchain = { version = "0.2.0" }`，而 0.2.0 **没发布** ⇒ 刚生成的宿主第一条命令就失败：
`cargo build --offline` → `error: no matching package named nichlink-toolchain found`。
**第一印象场景在最该演示约束系统的地方演示不了**（复核记录 `target/hardbug-runs/offline-birth.txt`）。

**关键实测事实（它决定了这条检查该问哪儿）**：本机 `~/.cargo` 里**有** 0.2.0 的 `.crate` 存档、**没有**
索引条目（cargo 的 `.cache/ni/ch/nichlink-toolchain`），而 `cargo build --offline` **依然失败**
⇒ **没有索引条目的存档解析不了**。因此探针必须查**索引缓存**（并要求其中出现 `"vers":"<版本>"`），
不能查 `.crate` —— 我第一版查的就是 `.crate`，实测被这条事实推翻（并把它钉进了钉子）。

**改动**：`build_time::scaffold::{registry_release_present, offline_source_warning}`（纯本地、确定性、
无网络、不会在气隙机器上挂住），由 `new_project` 在**预览与落盘两种回复**里附加一行 `offline …`：
点名版本、它查过的位置、以及三条出路（先联网构建一次 / 改用 `dependency: "git"` / 从检出里跑以获得
`path` 依赖）。`path`/`git` 来源自带字节，因此从不警告。

**实测（两个方向）**：真实 HOME（索引无该版本）⇒ 警告出现 ✓；伪造一份已预热的索引缓存 ⇒
`grep -c '^offline'` = **0** ✓；只放 `.crate` 不放索引 ⇒ 仍警告 ✓（钉子
`new_project_tests::a_release_the_cache_does_not_have_is_reported_rather_than_left_to_fail_later`）。

### §3 续（本轮）：把修复做成"本地确实能跑起来"

上一轮只做到**说出来**（警告），本轮做到**跑得起来**：`new_project` 新增 `path` 键——**点名一个 NichLink 检出**
（`kernel/` 与 `toolchain/` 相邻，校验过），生成的清单就写指向它的 `path` 依赖。这条拼写正是给**装出来的**
工具用的：它既不在检出内、又拿不到注册表缓存，以前只能生出一份 `cargo build --offline` 必败的清单。

**边界**：① `path` 单独给就是来源（本地拼写，读它到不了网络，拒绝只会让调用方把同一件事写两遍）；② 目录不像
检出 ⇒ **按名拒绝**并点名它找过的两个目录；③ `dependency: "registry"` 与 `path` 同时给 ⇒ **按名拒绝冲突**，
不静默偏向一个；④ 只有注册局来源才谈离线警告，`path`/`git` 自带字节、从不警告。

**出生证明实测（创建 → 构建 → 普查，全部离线）**：从检出外 `/tmp/nlmcp4` 生成 → `cargo build --offline`
**Finished**（21.37 s）→ `registry` 答 `namespace probe / faces 0`（library 模板未声明面，符合预期）。
`--list` 的字节预算被这次新增顶到 5001 ⇒ 削 `SHAPES_SHORT` 一句散文到 **4,976**（预算是预算，钉子逼着做取舍）。
复核记录 `target/hardbug-runs/offline-birth.txt`。

**仍开**：① 回复**无条件**说出"选了哪条来源"（现在只在有问题时说；无条件那句会动到回复形状的既有钉子，
留下一轮）；② 装出来的二进制若既不在检出内、又拿不到缓存，仍只能靠这条警告指路——真正的解除要么发布
0.2.0，要么调用方自己给 `git`/`path`。

### M6 §4 写路径边界：把不对称写成明示 + 给拒绝一条路（本轮）

**§4 第①条（不对称写进描述）已在位**：`apply` 的广告描述本来就写着两类动作——`add`/`deepen`/`cut`/`promote`
是**追加**类（改声明，手写面是合法主体），`edit`/`rename`/`delete` 是**改写**类（只改写自己生成过的文件），
拒绝文案也重复了这条界。钉子 `apply_tests::the_write_description_states_which_actions_reach_hand_written_faces`。

**§4 第②条（s7 写链的拒绝要给出路）本轮补上**：手写面被拒时，原来只划界（"这是作者身份，收编它是另一个
决定"），没有下一步。现在多一段 `way forward:`，给出**两条可走的路**：① 只要**声明**要改（加层、加条目、
换父级）⇒ 用追加类动作，这份文件就是合法主体；② 要给**文件**改名/改写 ⇒ 在编辑器里做，并点名"模块名就是
目录名与文件名，所以改名＝那一对名字 + 每一条点名旧路径的计划条目"，且给出**列出它们的命令**
（`affected {files: ["<该面的源码>"]}`）。钉子
`apply_refusals_tests::a_handwritten_face_refusal_names_the_routes_that_do_work`。同时保留了
"taking one over is a separate, explicit adoption" 那句（既有钉子要求它在拒绝里出现）。

**§4 第③条（D5：裁确认步）本轮落，但按证据裁在准确的位置**：先把那一问的归属查清——
`consistency --specimen` 的定义（GUIDANCE 自己的话）是"把兄弟与那批文件携带的已声明形状比对，点名谁缺哪条
声明"，也就是说它问的**就是同族形状那一问**；而 W5-6③ 之后，`apply` 的**写回执自己**就报
`family N sibling(s) under <parent>; outliers: x of N` ✓ ⇒ **"新面与同族一致吗"这一问不必再花一次调用**
（D5 的前提成立）。因此改动是：**add 形状里去掉那次 `consistency --specimen`**，把停止条件改成由回执回答
（`stop when that line names no outlier and the gates are green`），并在 GUIDANCE 里保留"台账那一问"
（`conformance {anchor}` + `consistency --specimen <anchor>`）——它问的是**与台账认证过的形状是否漂移**，
与写回执的同族多数派**不是同一个问题**，不许一起裁掉。
钉子 `client_tests::the_flow_table_packages_the_seven_shapes_with_their_stop_conditions` 当场因停止条件
措辞变化而红（行为变化打红旧断言 ⇒ 跟着行为改断言且保持同样强），改后绿。`--list` 仍 **4,976 ≤ 5,000**。

### M6 §5.2 T3：家族读走记录（本轮，**部分达成，验收未过**）

**改动**：① `build_time::face_views_from_pruning(rows)` —— 由剪枝记录构造 `FaceView`（W3-1b 发布
`parent_node`/`owns_registry`/`logical_path` 的目的就是它）；缺列的行**计数**，非零则调用方回退推导，
**绝不拿更小的家族去比**。② `consistency::member_faces` 记录优先：成员已发布且 `build_output_is_current`
⇒ 面取自记录，证据行说 `tree published from … (this comparison reads the build's own record)`。
③ **顺带修掉一处真缺陷**：`shape_names_from_record` 用记录的 `source` 拼写做键（**无** `src/` 前缀），而调用方
带 `src/` 去查 ⇒ **每次查询都落空**、静默退回读每个文件，而回复还声称"读的是记录"✗✗。新增
`source_index::indexed_path`（索引坐标只在一处定义，两侧共用）后，2,500 面独立包 **7.67 → 5.03 s**。
**这是同一族缺陷的第三次**（"两套坐标混用"已在 W6-2 第①步栽过一次）。

**实测（同一台机、同一棵树）**：`--by shape` 2,500 面独立包 **5.03 s**（验收 ≤1.5 s，**未过**）· 50 面独立包
**0.32 s**（走记录 ✓）· `--by kind` 同一棵树 **10.44 s** · 对照 `registry` **0.87 s**。

**已定位的下一处（O(n²)）**：`--by kind`/`--by source` 的 `values` 多数派分支对**每个兄弟**都做一次
`counts.values().max()` + 一次全表过滤 ⇒ 2,500² ≈ 6.25M 次比较 ⇒ 10.4 s，**反而比 shape 慢**。下一刀与
`deviations` 一样改成一遍计数、每兄弟 O(1) 查询。shape 那 5 s 还需再切一段量（记录读取本身应 << 1 s）。
复核记录 `target/hardbug-runs/t3-family-record.txt`（工作区数据，不进库）。

### M6 §5.2 T3 第二刀 + 第三刀前的归因（本轮）

**第二刀（O(n²) 修掉）**：`--by kind`/`--by source` 的 `values` 多数派分支对**每个兄弟**都做一次
`counts.values().max()` + 一次全量过滤 ⇒ 2,500² ≈ 6.25M 次比较。改成"一遍计数 + 预先算好 `top` 与
`tied_at_top`"，每兄弟 O(1)（语义等价：原条件＝该取值计数等于最高计数且不止一个取值拿到它）。
实测：`--by kind` **10.44 → 5.01 s** · `--by source` **4.69 s** · `--by shape` 5.07 s（本就走 `deviations`
的一遍计数，未受影响）。

**第三刀前的决定性归因**（同一棵 2,500 面独立包）：

| 探针 | 时间 |
| --- | --- |
| `consistency --parent root/nope --by kind`（**空家族**） | **4.53 s** |
| 同上、但把 `shape_manifest.tsv` 拿走 | 4.82 s（**不是它**） |
| `consistency --by kind`（**无记录 ⇒ 推导**） | 8.27 s |
| `registry --full --limit 200`（同树对照） | 0.83 s |
| `status` / `affected` | 1.25 s / 0.80 s |

⇒ **逐兄弟的循环几乎不花钱**：空家族就付掉了大部分。那 4.5 s 是**固定路径**的成本、随成员面数线性
（50 面的 `sr` 是 0.22 s ⇒ ≈1.7 ms/面），而且**不是** shape 清单的读取。`registry` 在同一棵树上 0.83 s，
证明这条固定路径可以低一个量级。**下一探针（已定）**：在固定路径三段各打一个计时点
（`build_evidence` 新鲜性 / 记录读取 + `face_views_from_pruning` / `tree_census` 与头部装配），量出 4.5 s
落在哪一段；在那之前不再猜。复核记录 `target/hardbug-runs/t3-family-record.txt`。

### M6 §5.2 T3 第三刀：`tree_census` 每答一次就整树推导（**成员级验收已过**）

**怎么找到的**：在固定路径上打计时点（临时插桩，量完即撤）⇒
`scope 83ms · build_evidence 198ms · read_pruning 15ms · build_views 7ms · member_faces 220ms ·
**tree_census 4.755s**` —— 就是它。`tree_census` 的实现是 `load_sources(root)`（读并解析整份源码）
**加上** `derived_faces(root, &namespace)`（**再推导一整棵树**），只为了印一行
`tree: N face(s), M file(s)`，而它印在**每一条** `consistency` 答复上（包括整个答案就是一个家族的那些）。

**修法**：**记录优先**——`published_census` 读剪枝清单得到"面数 + 承载它们的源码数"（与推导报告的是同一批
**事实**），记录新鲜时就是一次清单读取；工作区根没有自己的记录 ⇒ **把成员的加起来**（否则一个 20 成员的
工作区根为了印一行要付 50,000 个面的推导）。这一行还**说出自己是哪一种读法**
（`(read from the published record)`），因为要对比两棵树的读者不该去猜计数来自哪一侧。

**实测（2,500 面独立包）**：`--by kind` 5.01 → **0.51 s** · `--by shape` 5.07 → **0.79 s**（**验收 ≤1.5 s
过 ✓**）· `--by source` 4.69 → **0.51 s** · 空家族 4.53 → **0.54 s**。
**50k 面工作区**：151.5–174 → **50.0 s**（普查行证明走记录：`50020 face(s) … (read from the published record)`）。

**仍开**：工作区根那 50 s ＝ **20 × 每成员固定成本**（单成员 0.73 s，即每成员多约 1.8 s，未逐段量）。
下一探针同法：工作区根与一个成员分别打点，找出每成员多出来的那 ~1.8 s。
复核记录 `target/hardbug-runs/t3-family-record.txt`（工作区数据，不进库）。

### M6 §5.2 T1（第一刀）：`search --query` 的源码那一半走记录

**落地**：① 新增构建期记录 **`file_manifest.tsv`**（每份 `.rs` 一行：`source` + 它声明的函数名；一个函数都没
声明的文件写 `-`，因为按**路径**匹配的读者必须看得见每份文件）——写入方 `manifests::write_file_manifest`
（用内核自己的 `function_symbols`），读取方 `build_time::read_file_manifest -> Vec<FileRow>`。
② `search` 的**源码那一半**改由它作答（`record_source_lines`）：路径与函数名都取自清单，**零文件读取**；
记录答不了才回退原扫描，预筛规则不变。③ **工作区根**读**成员**的清单（根没有自己的记录），否则对着工作区根的
查询会退回扫描整棵树。
**实测**：分段计时把这一半从 **~4 s 打到 0.17 s**（2,500 文件的包）；t3 `--query child000001` 0.47 → 0.51 s。

**本轮回退的一刀（重要，如实记）**：我一度把"记录的空答案在记录新鲜时即为最终答案"做成规则（它让 50k 的
`--query` 从 **96.5 s 降到 12.8 s**、t3 的 4.5 s 降到 0.50 s）——**被既有钉子当场判红**：
`search_tests::a_published_face_is_ok_and_a_new_one_is_added_since_build` 在一份**新鲜**记录旁边写下一个新面并
期待 `[added since build]`，而那个面不在任何记录里 ⇒ **空答案不可能是最终答案**。根因是 **新鲜度不会注意到
"新增的文件"**。⇒ 次序改成：**先让 `build_output_is_current` 覆盖"被发现到的文件集合"**，再让记录的空答案成为
最终答案——那一刀约值 90 s（50k）/4 s（2,500 面），是 T1 真正的大头，但建立在前一刀之上。
复核记录 `target/hardbug-runs/t1-search-record.txt`（工作区数据，不进库）。

### M6 §5.2 T1（第二刀未落地）：空答案要成为最终答案，前提是新鲜度覆盖"新增文件"

**前提诊断（有证据）**：`discovery_fingerprint` 只哈希**被模块树发现到的**文件（`collect_source_files(nodes)`）
⇒ 一份 `src/` 下新增、**没有任何 `mod` 声明**的文件**不改指纹**，`build_output_is_current` 仍答 `current`
⇒ 这正是 `search` 的 `[added since build]` 钉子抓住的东西，也是"记录的空答案＝最终答案"不成立的原因。

**尝试**：把指纹补上"**原始文件集合**"（`visit_rust_paths(scan)`：只走 `src/**/*.rs` 的**路径**、不读内容
——编辑已由原循环覆盖，读内容会让每次新鲜性检查多读一遍整棵树），两个调用点同步传 `scan`。

**结果：那条钉子仍然红**，而且诊断打印留下一个**没解释清楚的现象**：
`stored=2acca1e0 recomputed=2acca1e0 raw=3`（**新增文件之后**，三个 `.rs` 都在，而 stored 与 recomputed 相等
——按测试顺序 stored 应在新增之前写下）。⇒ **改动全部回退**（`git checkout -- toolchain/`，回到 `e0ae9d5`），
空答案规则**保持关闭**。
**下一轮第一步**：查清上表里的 `stored` 是谁、何时写的（在 `check_for` 与 `TreeDelta::read` 两处打点，或比对
两次 search 前后指纹文件的 mtime/内容），再决定"原始文件集合进指纹"是否可行、以及空答案规则的次序。

### M6 §5.2 T1（第三轮追查）：仍未落地 —— **这一轮的错误在我自己身上**

三次插桩得出三个互相矛盾的结论，根因是我**把两次测量分开跑、再跨运行推理**：
- 运行 A（只打印指纹前 8 位）：新增文件后 `stored == recomputed`、`raw=3` ⇒ 判"两侧仍一致"；
- 运行 B（只打印原始路径清单）：新增后清单**确实**由 3 变 4（多出 `dial/dial.rs`）⇒ 判"覆盖生效"；
- 运行 C（两处都改好、规则打开）：那条钉子**仍然红**，回复依旧是"记录作答 + no matches"。
⇒ 三者不在同一尺度上，跨运行推理不成立。**教训**：一次实验必须把"输入、两侧的值、判定"**打印在同一行**。
**已回退**（`git checkout -- toolchain/`，回到 `e0ae9d5`），工作树干净、`search_tests` 18 项全绿。
**下一轮的一次性实验（写死）**：在 `build_output_is_current` 里把 `stored / recomputed / raw 清单 / scan 路径`
打在**同一行**，跑一次那条钉子，直接读出"谁写的指纹、两侧差在哪、raw 是否把新文件算进去"。

### M6 §5.2 T1（第四轮）：一次性实验读出了真正的前置缺陷

按上一轮写死的方式做**同一行打印**（`stored / recomputed / raw / scan`），一次跑完得出三件事：
① 全仓只有一个 `build_output_is_current`（`scope_view.rs` 里那个，`tree_delta.rs:106` 调的就是它）；
② 两行 ONELINE 都在新增 `dial` **之前**，也就是说第二、三次搜索**根本没再查新鲜度**——因为第一次搜索时
`record_face_lines` 就已返回空答案，而我的规则把它当成了最终答案，后面的代码路径根本没走到；
③ **判错的位置是第一次断言**（`search_tests.rs:89`，查询 `root/button` 期待 `[ok]`）。
⇒ 真正的事实：**`record_face_lines` 对 `root/button` 这个逻辑路径没有匹配到任何一行**，于是返回空。规则关闭时
这没关系（空答案让位给推导，推导按逻辑路径找到了那个面 ⇒ `[ok]` ✓）；规则一开，这份空答案就变成
`no matches` ⇒ 断言红 ✗✗。
⇒ 因此这一刀的前置缺陷不是"新鲜度不覆盖新增文件"，而是：**记录路径的匹配规则比推导窄**。今天不出错，只因为
空答案会让位。修好匹配，空答案才真的意味着"没有这个面"，"空答案即最终"才既对又省。
**已回退**（回到 `e0ae9d5` + 文档），`search_tests` 18 项全绿。
**下一轮写死的一次性实验**：在 `record_face_lines` 里打印**一行**：`query / 行数 / 前 3 行的
(logical_path, kind, registry_name, source) 拼写 / 是否命中`，看 `root/button` 为什么没被匹配上。

### M6 §5.2 T1：落地（匹配补全 · 陈旧记录逐面重认 · 指纹覆盖原始文件集合 · 空答案即最终）

**怎么定的位**：按上一轮写死的方式做**一行打印**：`parsed_kind=None / row_kind=Some("Button") / judged==row_id`
⇒ 我用来重认身份的读取器没读出 `kind`；再一行打印 ⇒ `one_face` 解析**成功**，但 `string("kind")=None` 而
`field("kind")=Some("RenamedButton")` ⇒ 改用 `field` ✓。**两次一行打印就定位**（对比前两轮的三次跨运行推理）。

**四件改动**：① **匹配补全**——`record_face_lines` 的匹配拼写加上 **`logical_path`**（`path` 是**声明**写的，
宏派生的面常是 `-`；`logical_path` 才是别的工具报告、调用方手里有的那个），这正是前四轮追到的"记录路径比推导窄"；
② **陈旧记录逐面重认**——记录陈旧时，被匹配的行用**它自己那份文件此刻的 `kind`** 重算身份（只解析那一个文件），
就地改过的 `kind` 因此报 `[re-identified (旧 -> 新)]`，回复照旧说 `build stale`；两条既有钉子（"陈旧记录照样答
名字"与"改过的 kind 要报重认"）由此同时满足；③ **指纹覆盖原始文件集合**——`discovery_fingerprint` 增加
`src/**/*.rs` 的**路径**列表（不读内容），于是新增一个没有 `mod` 声明的文件也会让记录陈旧；④ **空答案即最终**
（仅当 `known && current`）。
**实测**：t3（2,500 文件）`--query zzz-nothing` **4.54 → 0.75 s**、`child000001` 0.47 → 0.80 s；50k 工作区
`zzz-nothing` **96.5 → 17.2 s**、`child000001` 12.7 → 16.2 s。
**如实说明 50k 变慢的那一格**：`build_output_is_current` 现在每次多走一遍 `src/**`，而一次搜索**最多问它三次**
（delta / `record_source_lines` / `TreeDelta::read`）⇒ 20 成员 × 3 × ~0.25 s ≈ 15 s。**下一刀：一次调用内每个
成员只问一次新鲜度**（把判定往下传；**不新增缓存**——缓存正是这一带反复出错的形状）。

