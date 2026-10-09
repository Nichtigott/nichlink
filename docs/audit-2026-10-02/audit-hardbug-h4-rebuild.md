# T-16：h4 题树重建（D4 收口）

## 为什么重建

W8 的 **D4（blocker）**：冻结的 h4 树有**两处**文档违规——`signed` 与它自己的文档相反（文档说 "A debit prints
with a leading minus"，实现返回绝对值），而题面的第二条线索只选中未注定的那一个 ⇒ **真值选的缺陷 ≠ 线索选的
缺陷** ⇒ 那道题**不具鉴别力**。后果不止一道题：**整轮的链长比（0.63×）不能当工具质量的读数**——两臂各有
51%/56% 的推理花在那道坏题上。

配方在 `1c85cff` 修好（`signed` 的文档改成 "Always non-negative"），**冻结的树当时没动**（判据纪律：开跑之后
不改装置）。T-16 就是把这棵树按修好的配方重建出来。

## 重建后的树（实测）

命令：`tools/xirang-mcp-hardbug build target/hardbug-runs/t16-h4 h4-one-file-many-algorithms`

**自证**（`build` 自己打的）：`default face green (exit 0); the census names ZeroArm::Refuse at line 44` ✓
——这正是这道题要的形状：**默认面绿**，点名缺陷的是**普查的分支栏**。

**三个算法与各自的文档**（`src/model/entry.rs`）：

| 算法 | 文档说的 | 实现 | 一致？ |
| --- | --- | --- | --- |
| `zero_arm` (30-32) | "Which arm decides a zero amount."（而 `postable` 的契约写着 "a zero entry never is \[postable\]"） | **无条件返回 `Post`** | **✗ 契约未落实** ← 唯一的缺陷 |
| `postable` (38-46) | "an entry is postable when it carries a receipt, and a zero entry never is. Callers rely on the refusal, so this is a contract." | 收据分支 + `match zero_arm()`；`Refuse => false` 那一支**永不可达** | ✓ 代码没错，错在它依赖的 `zero_arm` |
| `signed` (53-55) | "**Always non-negative**: a debit prints as its size, never with a sign." | `if self.amount < 0 { -self.amount } else { self.amount }` | ✓ **与文档一致**（`1c85cff` 的修复生效） |
| `normalized_account` (60-62) | "Accounts are case-insensitive and trimmed." | `self.account.trim().to_lowercase()` | ✓ |

⇒ **恰有一处契约违规**，且题面那条线索（"does the opposite of what its own documentation promises, and no test
covers the branch that does it"）**只指得向它一处**：`zero_arm` 让 `postable` 的契约落空，而 `Refuse` 那一支
按构造不可达（普查点名 line 44）。

**真值**（`.audit/truth.json`，与本记录同时入仓）：

```
root_cause  {"file": "src/model/entry.rs", "line": 31, "symbol": "zero_arm",
             "mechanism": ["ZeroArm::Post", "zero_arm"]}
apparatus   tool_sha256  1e194b96368537cd44b150876ac02f03ec75c0474b24fcff14bc5cbe75a414df
            tree_sha256  24a47551bfe3f7548dd0aa04f1ec4a482d5692d4d9f4e0a64edd9397e8de30ad
```

## 一条**做不到**的验收，如实说

T-16 的验收写着"新树的 `truth.json` 与**旧树哈希不同**"。**这条无法核对**：W8 那棵树的哈希**从未被提交进
仓库**——它只活在 `/tmp/xirang-w8/…/.audit/truth.json` 里，已随那次清理消失（`docs/` 里搜 `tree_sha256`
只剩 D9 那一行说"`score` 打印三处哈希"，没有任何旧值）。因此这里**换一条可核对的性质**：重建后的树**恰有一处**
契约违规、且线索与真值指向同一处（上表逐行可查）。

**这本身是 D9 的续集**：`db9d5a4` 让 `build` 登记哈希，但**登记进的是那棵树的 `.audit/`**——**跑完要把它们提交
进仓库**，否则"与旧一轮不同"永远只能靠记忆。⇒ 从本笔起，每棵**进入比较的**题树的 `truth.json` 三个值都进仓库
（本文件就是第一份）。

## D4 的状态

**已收口**：配方 `1c85cff` 修好 · 树按修好的配方重建 · 线索与真值一致 · 自证（默认面绿、普查点名分支）通过。
**下一轮的链长比因此重新可用**（前提是 T-21 用的就是这棵或同等重建的树）。
