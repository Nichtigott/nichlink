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
