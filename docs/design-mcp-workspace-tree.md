# MCP 的"全树"与深度：对 codegraph 的结构性优势（2026-09-29）

维护者的判断（原话）：「studio侧其实我们应该是做了全树预览的，我们可以做全树，但是我们可以更精准，而且算法层面可以做到更深度，
减少一部分思考的工作，甚至更准确地锁定问题本身，这就是我们和codegraph比的绝对优势」。
本记录只定方向与缺口证据，不含实现。

## 1. 实测缺口：全树目前只在"包"这一层成立

根 `Cargo.toml` 是 **virtual manifest**（`[workspace] members = ["kernel","macro","toolchain","conventions","examples/…"]`，没有 `[package]`）。
在**工作区根**上调用（同一进程、同一棵树）：

| 工具 | 根上的结果 |
| --- | --- |
| `nichlink.status` | ✓ 可用：`rust_files=446 functions=2904`，815 ms（纯源码扫描，不需要身份） |
| `nichlink.registry` | ✗ 拒绝：`cannot learn the identity namespace of …: …/Cargo.toml is not a package; cargo metadata listed …`（219 字节） |
| `nichlink.grafts` | ✗ 同上 |
| `nichlink.diff {records:true}` | ✗ 同上 |
| `nichlink.search {query:"Button"}` | ⚠ **降级**：先自曝 `tree unavailable (…)`，再退回文件级命中（1584 字节、22 行、**无面**） |

逐**成员**就正常：`examples/control-button` → `namespace nichlink-example-control-button` ✓；`examples/control-button-graft` 同形 ✓。
而 `toolchain/tests/fixtures/node-editor`（**不在** workspace 成员表里的嵌套包）**同样拒绝** ✗ ⇒ 原型夹具那棵树也拿不到。

带面的成员只有宿主：`examples/control-button/src` 3 个文件、`control-button-graft/src` 4 个、`node-editor/src` 2 个；
框架三个 crate（kernel/macro/toolchain）与 conventions 本身不含面。

⇒ **树机件有，入口没有。** 身份按设计是"包 = 命名空间"，所以"根"（不是包）天然没有命名空间；
要跨工作区给出全树，必须逐成员建身份上下文再合并。

## 2. 为什么这是结构性优势，不是速度优势

codegraph 的知识是"符号 + 文件 + 调用边"。它没有：包即命名空间的身份、注册面与注册树、graft 切口与落盘记录、
能力声明与 provider、构建裁决与记录状态。我们的答案能带这些，而且每条都能被钉：
面的 `ok` / `added since build` / `re-identified` / `build unknown`；记录的 `ok / undeclared / stale / re-identified / unreadable`；
关系的 `Live` 或 compiler candidate。**"更准"就来自这里：结论带着它的身份与证据。**

## 3. 深度方向：减少 agent 的思考，而不是减少它的输入

三层，都落在树语义上：
1. **从症状反推最小定位链**：输入一条构建诊断、一个被拒的面、或一条 stale 记录 ⇒ 输出"哪个面 / 哪条 requirement / 缺哪个 provider
   （或哪条 graft 记录指向它）"，带身份与证据；而不是让 agent 自己读三份报告去拼。
2. **跨包影响面**：给定一个符号或一个面，返回身份感知的反向闭包——**哪些包的哪些面**受影响，而不是文件级 grep 结果。
3. **收敛判定**：对一棵树给出"现在能不能发布 / 还开着哪些口"的结论，并**声明它的上限**（哪些面没检查、哪些包不可解析）。

## 4. 下一批的实现要点（未做）

- **根解析**：解析到 virtual manifest 时，用 `cargo metadata` 枚举成员，对每个成员建自己的身份上下文（命名空间＝包名，与 `NodeId` 的定义一致）。
- **合并视图**：按包/命名空间分组呈现，每个节点带构建裁决与记录状态；**不可解析的成员必须出现在答案里并说明原因**
  （现在 `node-editor` 与库 crate 要么沉默要么报错）。
- **降级必须自曝**：`search` 那句 `tree unavailable (…)` 是正确样板；其余工具跨包时也要这样，**不许**用文件级结果冒充树级答案。
- 顺带保留两个便宜项（不是战略，只是补形状）：`read` 报出文件总行数、支持一次读多份并声明上限。

## 5. 边界

实现在 `toolchain/src/mcp` **当前在飞批次落地之后**进行——同一目录不能有两个写者。
本记录只含方向、缺口证据与实现要点，不含代码。
