# MCP 完善与对齐纲领（2026-09-29，维护者口述）

维护者原话：「现在开始完善和提升mcp,然后再进行测试，mcp要拥有studio的所有能力不能是只读，另外能力上要对齐codegraph，
而且要做到我说的search能力，形成强效收敛，快速锁定问题的工作流」。

## 0. 先纠正一个前提，再砍掉两件不该做的

- **桥不是只读的** ✓：它已经有 `apply`（`action: add|edit|delete`，默认预览、23 个可编辑字段、删除要 `confirm: true`），
  与 Studio 的 `submit_add`/`submit_edit` 走**同一套创作执行器**。缺的是 Studio 另外两个写动作（见 §1）。
- **不计划写 graft 计划** ✗：`graft.plan` 这种记录**全仓一个都不存在**（`find . -name graft.plan` 为空），也没有任何生产路径会写它 ⇒
  没有用户的能力不做（karpathy：不写投机抽象）。桥的 `grafts` 已经能**读**计划行与声明状态，够 agent 用。将来真要"写计划"，单独决策。
- **Studio 的 UI 机制不是能力** ✗：`hot_zones.rs`（分隔条命中、鼠标区、缩放）、`source_stamp.rs`（源指纹）属于终端交互状态，不做进桥。

## 1. 写入面对齐 Studio（实测差两个）

Studio 的四个写动作实测为 `submit_new_project` / `submit_add` / `submit_edit` / `submit_plugin`；桥覆盖后两个半（add/edit/delete），缺：

| 缺口 | Studio 入口 | 桥 | CLI |
| --- | --- | --- | --- |
| **新建项目脚手架** | `submit_new_project` → `build_time::scaffold::create_project` | ✗ 没有工具 | ✓ `nichlink new` |
| **插件准入 / 官方记录写入** | `submit_plugin` | ✗ | ✗ |

两者都要沿用 `apply` 已立的纪律：**默认预览、不可逆动作必须显式确认、在工作区根上必须解析出唯一拥有者才写（否则拒绝并列出候选）**。

## 2. 对齐 codegraph 的能力面（实测差两项）

codegraph 的命令面：`init/uninit/index/sync/status/query/explore/node/files/daemon/unlock/callers/callees/impact/affected`。
其中 `init`/`index`/`sync`/`daemon`/`unlock` 是它自己的索引维护（我们的对应物是构建期发布的记录），**不需要对齐** ✗。

| 缺口 | codegraph | 我们 |
| --- | --- | --- |
| **callees** | ✓ `callees <symbol>` | ✗ `callgraph` 只给 definitions + callers（实测 `callgraph.rs` 里只有 `callers`） |
| **affected**（按改动文件选出受影响的测试） | ✓ `affected [files…]` | ✗ 没有（材料齐：源索引 + 调用图 + 测试形状规则 + `tools/nichlink-test` 的面） |
| `files`（项目文件结构） | ✓ | 可能是 `status` + `search` 已够 ⇒ **先量再定**，不预设 |

## 3. "search 强效收敛"的工作流（维护者要的那条）

要的不是"搜到一堆命中"，而是**一次提问收敛到最小定位链**：树层（哪个包/哪个面/哪条记录）→ 调用链层（哪条边，带证据等级 `Live`/compiler candidate）
→ 实现层（哪个函数、哪段行区间），每层带证据与"下一步看什么"，并且**默认摘要 + 声明上限**、全量通路可及（`read` 现已支持整文件/行区间 ✓）。
这是把"深度三层"（见 `docs/design-mcp-workspace-tree.md` §3）落成**一个入口**，也是维护者说的"准确锁定问题的来源是算法的作用，ai 只要调用工具就能做到"。

## 4. 然后再测试（维护者要求"然后再进行测试"）

实现完必须重跑并留证：① `tools/nichlink-test`（十面 + 灰测试）；② **头对头**六题 + 每个新增能力各一题，给字节与耗时的百分比；
③ 12 条落地门禁。结果写成记录（放 `docs/audit-*`），并附本次口径：字节是硬数，token 按同一折算。

## 5. 排程与约束

- **同一目录一次一个写者**：`toolchain/src/mcp/**` 当前由"优先读发布记录"那批占用（它直接决定 §4 里耗时那一栏），完成后按 §1 → §2 → §3 依次派。
- 每批都要：钉子 + 变异红/绿、真实门禁 EXIT、不提交（队长验收后统一提交）。
- 已知未闭合（本纲领之外，另行排）：`control-button-graft` 整棵树读不出来（面文件用绝对路径调用宏、`source:` 字段与实际位置不符）；
  `apply` 写入类拒绝的 `isError` 口径；`read` 之外的单包工具在根上的归属解析已覆盖。

## 6. 实施顺序与每片的验收（2026-09-29，维护者说"向下具体实施"）

维护者重申的目标：「功能不能只是只读相当于它能让agent直接能操作studio。然后就是code graph给的信息我们也要给，要更加直接地给，
而且我们的功能要比code graph多，我说的调用链对比，数据出入对比等等，还有采信租赁设计」。⇒ 六片，串行（`toolchain/src/mcp/**` 一次一个写者）：

| 片 | 内容 | 验收要点 |
| --- | --- | --- |
| **S1** | **树级答案优先读已发布记录**（在飞）：`<pkg>/target/nichlink/out/` 走 `build_method` 既有读取器，记录缺失才回落推导，答案标明来源；成员状态 `published`/`not built`/`no faces` | 有记录成员上给出"读记录 vs 现推"的耗时对比；不触发全量面推导；变异（改回一律现推）⇒ 钉子红 |
| **S2** | **Studio 写入面对齐（不是只读）**：① 新建项目脚手架（`build_time::scaffold::create_project`，与 Studio `submit_new_project` 同一执行器）；② 插件准入 / 官方记录写入（与 Studio `submit_plugin` 同一路径） | 默认预览；不可逆动作显式确认；工作区根上必须解析出**唯一拥有者**才写，否则拒绝并列出候选；钉子 + 变异；**零意外写入**（调用前后 `git status` 逐字节相同） |
| **S3** | **codegraph 信息对等，且更直接**：① `callees`（正向，现只有 callers）；② 调用方的**文件归属**与**引用它的测试文件**；③ **答案里带相关源码**（一次调用；默认仍摘要，显式要求才带，越限走截断唯一出口）；④ `affected`（按改动文件选出受影响的测试） | 六题对照里这些元素由 ✗/部分 变 ✓；`affected` 在一处真实改动上给出去重后的测试目标；源码携带不得让默认答案变大（默认仍摘要） |
| **S4** | **比 codegraph 多的三件**：① **调用链对比**（两条链的差异：只在一边的边、证据等级差异——`Live` vs compiler candidate 是它没有的维度）；② **数据出入对比**（两份 artifact／两次构建／两个宿主之间的出入；现有材料 `diff` 的 `ok/undeclared/stale/re-identified/unreadable`、`mir --against`、`unified`）；③ **采信租赁**（台账 + 漂移键 + `needs-confirmation` + 门禁，按 `docs/design-adopted-reference-chains.md` §2/§6：漂移不自动撤销也不自动续期，续期是显式追加，门禁要求人工确认） | ① 给出"哪条边只在 A / 只在 B / 两边都有但证据等级不同"；② 出入对比对同一输入可复现且标明两侧来源与新鲜度；③ 台账条目缺 `invalidation` 键即红；漂移后答案措辞变 `adoption lapsed …; needs confirmation`，且**永不出现"已保证/已验证"** |
| **S5** | **search 强效收敛入口**：一次提问 → 最小定位链（树层 → 调用链层 → 实现层），每层带证据与"下一步看什么" | 六题里每题**一次调用**给出分层结论与下一步；默认摘要 + 声明上限；全量通路可及 |
| **S6** | **测试**（维护者要求"然后再进行测试"） | `tools/nichlink-test` 十面全绿且灰测试 0；**预注册**的头对头（先写"每题用哪个工具、允许几次调用"再跑）给出字节与耗时百分比；12 条落地门禁；结果写成 `docs/audit-*` 记录并标注"截至 `<commit>`" |

**排程纪律**：S1 完成后立刻派 S2；每片都必须带钉子 + 变异红/绿 + 真实门禁 EXIT + 不提交（队长验收后统一提交）；
每片落地后队长复跑窄门禁并**独立复验关键声明**（例如 S2 的"零意外写入"要用调用前后 `git status` 逐字节比对证明）。

## 7. 新鲜度的决定与一处归因更正（2026-09-29）

维护者：「我觉得你的推荐没有问题」⇒ 采纳**给新鲜度分等级、不改判据**：`build_output_is_current`（内容哈希）仍是唯一权威，
但答案印的是**等级**——`freshness: content-verified` / `freshness: not re-checked; last content-verified at HH:MM:SS` / `build stale (run nichlink check)` / `unknown`；
**绝不把"没核验"说成"当前"**；强校验的代价变成可复用/可选（调用方显式要求时付全费）。
**不采纳**：把判据换成 mtime ✗（本仓踩过 `cp -a`/`rsync -a` 保留 mtime：内容变了却看着新鲜）；无标注的进程内记忆化 ✗（长驻桥会把陈旧答案当当前报出去）。

### 7.1 归因更正：那 1.5 秒不是新鲜度判定 ✗

S1 报「根上剩余 1.1–1.5 s **全部**是 `build_output_is_current`（6 成员各一次）」。**我复算后判定这条不成立** ✗，逐成员实测（每项取 3 次最小值）：

| 成员 | 耗时 | `target/nichlink/out` 记录 |
| --- | --- | --- |
| `kernel` | **1063 ms** | **0 个** |
| `macro` | 76 ms | 0 个 |
| `toolchain` | 53 ms | 0 个 |
| `conventions` | **505 ms** | **0 个** |
| `examples/control-button` | **44 ms** | **6 个** ✓ |
| `examples/control-button-graft` | 52 ms | 0 个 |
| 六者之和 | 1792 ms | |
| **根一次（virtual manifest）** | **1589 ms** | 普查 `published 1 not built 0` |

⇒ 贵的是 **`kernel` 与 `conventions`（合计 1568 ms）**，而这两个成员**恰好没有发布记录** ⇒ 它们走的是**回落"现推面"**那条路（整棵源码树推导，最后得到 0 个面）。
有记录的 `control-button` 只花 44 ms ✓ —— 也就是说**读记录 + 新鲜度判定本来就很便宜**。
S1 的 stub 实验之所以"降到 88–107 ms"，是因为把判据改成恒 `true` **顺带把这些成员当成 published，连推导一起跳过了**，它把这一步的效果误记成"新鲜度判定的成本" ✗。

### 7.2 于是目标改为：不要让"没有记录的成员"付全量推导

两条互补做法（同片实施）：
1. **便宜的负数预检**：在推导面之前，先做一次廉价的"这棵树里到底有没有注册面标记"的判定；没有就直接答 0 个面，不付全量遍历。判据必须**只用于否定**（"没有标记 ⇒ 确实没有面"要成立），有标记时仍走原推导。
2. **把推导变成可要求、且分级**：没有记录时默认答 `faces: not derived (no published records; pass derive: true)`，并保留"要现推就来"的通路 —— 与新鲜度分级是同一套纪律（**不把没做的事说成做过的**）。

新鲜度分级（§7 主体）仍然实施：它的价值在**诚实**（不把未核验说成当前），而**不是**性能；性能的账记在 §7.2。
