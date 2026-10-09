# 采信 tag 与参考链路：设计（2026-09-29）

维护者的设计原话（保留措辞）：
「我再来说说所谓采信tag工作逻辑，当我们说某条路线暂时稳定且采信时，会被标记为参考链路，当横向扩展同级对象的时候会参照已经采信的代码和结构
形成规范来自动约束ai,可以精准地让ai agent来横向扩展代码，找bug也会如此，方便ai锁定问题的层级，到底是实现侧精确到哪一块的问题还是调用链的
问题，还是两者都有问题，减少ai自己的判断读取等消耗的token，准确锁定问题的来源是算法的作用，而ai只要调用工具就能做到，我们默认不投喂全量信息，
但是保留这个通路，相当于是在code graph的基础上深化。」

## 1. 先找已有的（实测），再谈要新建的

| 这套逻辑需要 | 现状 |
| --- | --- |
| "问题是实现侧还是调用链"的**分层材料** | **已经在结构里**：`RegistryError { node, path, source, message, source_chain: Vec<ProvenanceStep>, call_path: Vec<CallSite>, registration_chain: Vec<RegistrationState>, children }`（`kernel/src/registry_core/diagnostic/error.rs:15-22`）。⚠️ 同文件 `:121-126` 明说这些聚合链"没有这样的调用方，因此也没有读取器" ⇒ **有记录、没读者**，这是现成的缺口 |
| 横向扩展的**机械规范** | **已经存在**：父级 `RegistrationRule`（`require_preset`/`require_parts`/`require_exports`/`require_handle_traits`/`require_part_traits`，`kernel/src/registry_core/declaration/registration.rs:100`）+ `Admission::accepts`（`:64`）+ 面的 `contract`（required/provided parts）、`requires`/`provides`、`runtime_checks`。"同级该长成什么样"本来就是框架在机械检查的东西 ⇒ 规范不必发明，只需**从已采信的同级身上提取并差分** |
| 实现侧"哪一块"的**颗粒度** | 已有：`function_symbols(source) -> Vec<SourceFunction>`（含 `is_function`/`end_line`，`kernel/src/registry_core/source/source.rs:125`、`source/items.rs`）、`body_calls(body, target)`（`source/calls.rs:27`） |
| 调用链的**证据等级** | 已有：`unified`/`converge` 给每条关系标 `Live` 或 compiler candidate；`diff --records` 把记录分成 `ok / undeclared / stale / re-identified / unreadable` |
| **落盘位置** | 已有约定：`XIRANG_DIR = ".xirang"`，其下已有 `external-grafts/`（`graft.plan`）与 `traces/`（`kernel/src/registry_core/lexicon/lexicon.rs:121-133`）⇒ 采信台账应住 `.xirang/` 下，**不是 `target/`**（那是可丢弃的） |
| **"采信 / 标本 / 参考链路"这个概念** | **不存在** ✓（全仓 grep 只有插件签名的 `trust`，语义不同）⇒ 这一件是真的新 |

## 2. 采信台账的最小可判形状（否则它会变成会撒谎的标签）

每条采信是一条**可判定的记录**，字段：

- `anchor`：被采信的对象（`NodeId` + 逻辑路径）；横向扩展场景下是"父级 + 同级"这一对。
- `certifies`：这条采信覆盖什么——哪几条**调用边**（caller/callee 对）、哪几个**实现块**（`file:line` 区间 + 函数名）。
- `evidence`：证据种类与出处（trace 运行 / `converge` 的 Live 集合 / 记录状态 / `verify` 的两条裁决）。**没有证据的采信一律不接受**。
- `verifier` + `at`：谁验的、什么时候。
- `invalidation`：**漂移键**——涉及文件的内容指纹（构建侧已有 `discovery.fingerprint` 这一形态可借）。键一漂，工具必须**拒绝**再称它"已采信"，并说清是哪一部分动了。

⇒ **能被漂移作废的采信才叫采信**；不能作废的就是粘性标签 ✗ —— 而那正是本仓"自我描述必须与行为一致"的红线。

## 3. 四件交付（按依赖排序）

1. **补读者**：`RegistryError` 的三个链（`source_chain`/`call_path`/`registration_chain`）现在**有记录没读者** ⇒ 先让它们能被读出来并展示分层，这是"锁层"的地基（材料已在，缺的是消费者）。
2. **规范提取与差分（横向扩展的自动约束）**：给"父级 + 一个已采信子面"，输出这份**规范**（父级 rule + 该子面的 contract/parts/exports/traits/runtime checks），并对候选子面做差分——缺哪一条、与规范哪一项不符。约束来自**机械规则**，不是模型的记忆。
3. **分层定位（锁层）**：输入一条症状（构建诊断 / 被拒的面 / stale 记录 / 失败的校验），输出层的判定：
   - **实现侧** ⇒ 精确到函数与行区间（`function_symbols` + `body_calls` 的颗粒度）；
   - **调用链侧** ⇒ 哪条边，以及它的证据等级（`Live` / compiler candidate）；
   - **两者都有** ⇒ 分开写，并说清边界。
   定位是**算法**的活，agent 只调工具 ⇒ 这正是"减少 ai 自己的判断与读取"的落点。
4. **保留全量通路（默认不投喂，但通路要在）**：`read` 的整文件/行区间（现在是 ±40 行窗口、且不报文件总行数）、发布记录 `<pkg>/target/xirang/out/`、trace artifact `.xirang/traces/` 三条通路明确可及，并由答案告诉 agent"要全量就走哪条"。

## 4. 与 codegraph 的关系

维护者原话：「相当于是在 code graph 的基础上深化」。codegraph 的粒度停在"符号 + 文件 + 调用边"，它没有面、注册树、graft 记录、能力契约、判据层级与证据等级；
我们在这层之上加**采信**（谁被验过）、**规范**（同级该长什么样）、**判层**（问题在哪一层）——三件都是它结构上做不了的。

## 5. 落地顺序与前置

- **前置**：先把"漂移作废"做成机械检查（每条采信都要有 invalidation 键，键漂了就红），照 `.xirang/` 既有棘轮（`BASELINE`/`SHIMS`）的形态；否则台账会撒谎 ✗。
- 顺序：① 补 `RegistryError` 链的读者 → ② 规范提取与差分 → ③ 采信台账 + 门禁 → ④ 四条通路工具化。
- **在飞依赖**：测试治理与"全树入口"两批正在跑；`toolchain/src/mcp/**` 同一目录不安排两个写者。
- 本记录只含设计与现状证据，不含实现。

## 6. 采信是"暂时采信"：可改，但改动要人工确认（2026-09-29 维护者补充）

维护者原话：「采信是可以改的松动的，但是要强制人去确认可修改，因为肯定是稳步发展的，只是暂时采信」。

⇒ 对 §2 的修正：我原先只写"漂移即拒绝"，那太硬 ✗。正确的语义是**租约，不是证书**：

- **条目天生是临时的**：带 `provisional: true` 与"被采信时的指纹"，措辞上**不是**"已验证 / 已保证"。
- **漂移不自动撤销，也不自动续期** ✗✓：漂移把条目置为 `needs-confirmation`。工具在**答案里**就不再称它"已采信"（agent 因此拿不到过期的绿灯 ✓），但它既不悄悄续期，也不因此回退代码。
- **续期是一次显式追加**：`confirm` 必须带**新指纹 + 证据 + 谁确认的 + 为什么**；台账**只追加不覆盖**，因此"这条链路被采信过几次、每次为什么重新确认"是可读的历史。
- **门禁在 `needs-confirmation` 时红，而它要求的动作是"人工确认"，不是"改代码"** —— 这正是"强制人去确认"的机械形态；形状照 `.xirang/` 既有的棘轮（`BASELINE` 的过期条目会红、`SHIMS` 的钉住语句会红）。
- **答案措辞随之定死**：正常时说 `adopted since <date> at <fingerprint> (provisional)`；漂移后说 `adoption lapsed at <file>; needs confirmation`。**永远不出现**"已保证 / 已验证"这类口气。

这样"稳步发展"与"强制确认"同时成立：开发可以随便往前走，但**每一次让某条链路重新配得上"采信"这两个字，都要有人签字** ✓。
