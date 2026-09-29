# nichlink MCP 与 codegraph 的效率、收敛性实测（2026-09-29）

装置：nichlink 侧用 `target/debug/nichlink-mcp`（stdio JSON-RPC，驱动脚本取自 prober 的原件 `docs/audit-2026-09-29/audit-mcp-probe-raw.md` §1）；
codegraph 侧用 `~/.local/bin/codegraph` 1.5.0（`explore`，默认 `--max-files`）。
两侧都**含进程启动**（每次调用起一个进程），因此是可比的；差异是 nichlink 为 debug 构建。
题目用 t1 冻结的 6 道（C1/C2 调用链、W1/W2 符号与调用方、I1/I2 影响面）。

## 1. 每调用的成本与产出

| 维度 | nichlink MCP | codegraph |
| --- | --- | --- |
| 单次延迟（示例宿主 13 文件） | `status` 14–21 ms；`registry` 66–102 ms；`grafts` 55–100 ms | 6 题 468–759 ms |
| 单次延迟（工作区根 444 文件） | `status` **964 ms**；`registry` 52 ms | 与规模无关（索引已建） |
| 单次答案体积 | 0.4–1.4 KB（记录里逐调用 52–1422 chars） | 17–29 KB |
| 一次性成本 | 无索引；每次现读树 | 索引 29.7 MB / 444 文件 / 5,492 节点 / 17,502 边，需 `sync` 保鲜 |
| 6 题里给出显式调用路径的 | 1 跳（`callgraph` 的 callers/callees），多跳要多次调用 | **0/6**（grep `→`/hop 全 0；返回的是符号袋 + 逐字源码） |

读法：**小包上我们快约 10 倍、答案小 10–50 倍**；**到仓库规模，扫描类工具（`status`）的成本随树增长到约 1 s，与 codegraph 的固定 0.6 s 打平甚至更差**，而 registry 范围内的工具（`registry` 52 ms）仍然很快。
两边都含进程启动；codegraph 的 0.6 s 大部分是 Node 启动，长期驻留的 MCP 服务会摊薄它 ⇒ 这一栏不能当成稳态吞吐比较。

## 2. 收敛性

**确定性：两边都是确定的。**
- nichlink：同一棵树、同一调用跑两次，字节与 sha256 相同（`status` 466/466 B、`registry` 795/795、`grafts` 543/543）。
- codegraph：同一句 + 同一 `--max-files`，连跑两次 21913 字节、sha 相同。
- ⇒ 先前"codegraph 不可复现"的结论已更正为"差别来自参数与措辞"（见 `audit-mcp-workflow.md` §⑦）。

**停止条件：codegraph 缺，我们半有。**
- codegraph 的 `Found N symbols across M files` 是**显示上限的线性函数**：C1 同一句 `--max-files 1/2/3/5/10/默认` → `9/16/24/35/35/35` 个符号、`1/2/3/5/5/5` 个文件。默认恰好等于自然天花板 ⇒ **答案里没有任何一处能让你判断"是查完了还是被截了"**。调用方清单同样 `+N more`，而那个 more 的单位（调用点还是文件）从返回里读不出。
- 我们：多数会截断的工具会印出**被扣掉的数量**并给"raise limit"提示（`callgraph`、`build_evidence`、`converge`、`trace`、`mir::bounded`），但**词形各写各的**、部分工具静默截断，且 `mir {jsonl}` 把截断句子写进 JSONL 正文、导致"存下来读不回"（`MirParseError { line: 401 }`）。这两件正在修。

**证据状态：这是我们结构上的优势。**
- `unified`/`converge` 给每条关系标 `evidence = Live`（观测到的）或 compiler candidate（编译器候选）；`diff --records` 把每条记录分成 `ok / undeclared / stale / re-identified / unreadable`。⇒ 答案带着"它凭什么可信"。
- codegraph 唯一相近的是 `⚠️ no covering tests found`，而它实测三处全是假阴性、甚至与同一份返回里的源码自相矛盾 ⇒ 不能作为证据状态使用。

**链问题：两边都不能一次收敛，但收敛形状不同。**
- 我们每多一次调用就多一跳，且带身份（`NodeId`）与来源，是**单调积累**。
- codegraph 每次返回同形的符号袋，**换措辞会换掉文件集合**（C1：默认 35/5，写死成"把每一跳列出来"后 20/2 且含路径外的 `diagnostic/error.rs`）⇒ 不积累，只是换一堆原料。

## 3. 结论

| 轴 | 谁更好 | 依据 |
| --- | --- | --- |
| 小包上的延迟 | 我们（约 10×） | 14–102 ms vs 468–759 ms |
| 仓库规模上的延迟 | 打平或 codegraph 略好 | 我们 `status` 964 ms vs 它固定 ~600 ms |
| 答案体积 | 我们（10–50×） | 0.4–1.4 KB vs 17–29 KB（但它是原料，我们是结论，不完全同类） |
| 一次给全 | codegraph（整文件逐字源码） | 6 题里它 4 次给到需要的源块 |
| 调用链 | 都不够；我们可单调积累 | 它 0/6 给路径；我们给 1 跳 |
| 停止条件（能否知道查完了） | 我们（半有） | 它那个计数被 `--max-files` 决定且不声明 |
| 证据状态 | 我们 | `Live`/compiler candidate、`ok/stale/unreadable` 分类 |
| 保鲜 | 我们（现读树） | 它的索引需 `sync`，陈旧时答案不报错 |

补两条方法学声明：① codegraph 侧是 CLI 每次起进程，nichlink 侧也是每次起进程，但真实使用中我们的桥是常驻的（`initialize` 只 3 ms），因此延迟栏偏向我们；② 本次未测"长驻 MCP 服务 vs 常驻索引服务"的稳态吞吐，也未测大仓（>10 万行）下的表现。

## 4. 更正与补测（2026-09-29，维护者问"我们应该也能读全树"）

我先前口头说"`read` 把整个文件都给了"——**错**，那只对 40 行的 `examples/control-button/src/control/object/button/button.rs` 成立。实测 `nichlink.read` 是**有界窗口**：

| 调用 | 返回 | 标头 |
| --- | --- | --- |
| `read {line: 1}`（41 行的文件） | 40 行 | `…button.rs:1-40` |
| `read {line: 1}`（473 行的 `transaction.rs`） | 42 行 | `transaction.rs:1-41` |
| `read {line: 100}` / `{line: 200}` / `{line: 400}` | 82 / 82 / 82 行 | `:60-140` / `:160-240` / `:360-440` |

⇒ 窗口是**±40 行**（上限约 81 行），而且**回复里没有文件总行数**，所以读一个 473 行的文件要 6 次调用、且**你不知道什么时候读完**——这与我在 §2 批评 codegraph 的"没有停止条件"是同一个病，只是发生在读文件这一栏。

**同题补测（codegraph 的 `node` 与 `explore`）**：
- 单文件：它 `codegraph node <file>` 一次给整份带行号的源码，标头还写着 `— 41 lines, 1 symbol · no other indexed file depends on it`（**它报总行数**）；2006 字节 / 304 ms。我们同文件 1877 字节 / 3–4 ms，但只给窗口。
- 多文件：C1 那次它返回的那 5 个文件，我们一个会话里 5 次 `tools/call` 共 **14 ms / 9640 字节**；它一次 `explore` **759 ms / 26938 字节**。⇒ 需要多次调用，但总成本仍是它的 1/54（时间）与 1/2.8（字节）。

**修订后的判定**：`一次给全` 这一栏**codegraph 确实领先，而且领先在"整文件"这个具体形状上**（它一次给全文并报总行数；我们只有窗口、还不报总行数）。我先前把理由说成"我们读不了树"同样不对——树我们有：`source_index` 在索引、`search` 按名字查面/文件/函数并带构建裁决、`inspect` 给单文件摘要（93 字节 / 3 ms）、`read` 给窗口。缺的是**整文件与多文件的读取形状**，以及**窗口自身的自曝**（总行数、还剩多少）。

⇒ 下一批 MCP 切片的候选（未做）：`read` 支持整文件/行区间并报总行数；支持一次读多份文件并声明上限。这两条会直接抹掉它目前唯一的稳赢栏，而按上面的实测我们仍然更快。
