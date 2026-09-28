# 独立验证：B4-core（t26 已应用修复 + t32 API 兼容 + t33 接续）

- 验证者：kernel-auditor（t34，**非作者**：t26=gates-auditor、t32=logic-adversary、t33=gates-auditor）
- 日期：2026-09-28；真实树**零改动**（本轮只写本文件）；全部夹具/副本/变异在 /tmp
- 装置：检出副本 `/tmp/nk-t34`（**自带 `CARGO_TARGET_DIR=/tmp/nk-t34-target`**）+ 我自写的 crate 外探针 `core/tests/t34_probe.rs`（5 条用例，数据与作者钉子不同）+ 9 组变异

## 0. 判定一览

| 条目 | 判定 | 一句话 |
| --- | --- | --- |
| t26 LG-04 锁 schema 五形态 | **证实** | 我的 5 形态夹具全拒（空锁未知 schema / 记录后表头 / 拼错 / 重复 / 空值），正控两条可读；变异让 `schema_matches` 恒真 → 作者钉子 + 我的探针同时红 |
| t26 LG-27 `runtime_checks` 合并 | **部分证实** | 源码读到 `if !authored.runtime_checks.is_empty() { self.runtime_checks = authored.runtime_checks; }`（与 `flow` 同规则）；我未自建夹具（构造 `RuntimeCheckSpec` 数据成本高），未做变异——覆盖缺口见 §4 |
| t26 LG-28 两入口同判据 | **证实** | `parse_requirements`（校验器）与 `try_parse_requirements_owned` 同源 `requirement_item`；我的探针对畸形列表：published 返回 2 条（有损、不报错）、strict 报 Err、validator 报 Err |
| t26/t32 API 兼容 | **证实** | `parse_requirements_owned(&str) -> Vec<OwnedRequirementSpec>` 与 `git show HEAD:` **逐字同签名**；严格入口 `try_parse_requirements_owned` 真的拒绝（我的探针 + 变异 M2 双向） |
| t26 LG-30 泛型逗号 | **证实（两半）** | `trait_names_from_paths("…ControlHandle<u8, u16>")` → 单标签 `ControlHandle`，两条路径 → `A,B`；变异 tokens.rs 半边 → 作者单测红，变异 parse.rs 半边 → 作者钉子 **与我的探针**同时红 |
| t26 KRN-C-02 doc | **证实** | `CallSite` 文档改为"One observed call site…"，`CallEdge` 保留原句（finding 的 fix_hint 逐句落地）；我的探针读源码断言两句都在 |
| t33 LG-06 `child_registry_can_survive` | **证实（代码读 + 变异）** | 谓词 `needs_registry \|\| child.is_none_or(empty)`、非 full 分支与 `replace_info` 共用；变异关掉守卫 → 作者钉子红（我未自建 overlay 夹具，§4） |
| t33 LG-07① 兄弟同名 | **证实** | 我用自己的名字/kind 构造两兄弟 → 拒绝且消息含 `duplicate sibling registry name` 与槽位名；批内同名同样拒、不同名仍成功；变异让守卫永不命中 → 作者钉子 **与我的探针**同时红 |
| t33 LG-07② `resolve_path_strict` | **证实** | 切口路径支走严格读法（`Resolution::{One,Missing,Ambiguous}`），`resolve_path` 仅剩 `resolve_record` 一个调用方；**我自己复现**：把 `resolve_node` 改回首匹配 → `resolution_tests::an_ambiguous_replacement_selector_is_refused` 红 |
| t33 LG-29 模块级 cfg | **证实（代码读 + 变异）** | `visit_item_mod` 只跳 `cfg_is_test_only`，其余门控压栈后与条目门控合成；变异恢复"任意 cfg 整棵跳过" → 作者钉子红（我未自建语法夹具，§4） |
| t33 被改写的既有夹具仍能抓回归 | **证实** | ① 堂兄弟歧义钉子：`resolve_node` 回首先匹配 → 红（我自己跑的）；② 示例 `broken-button` 钉子：**关掉父级规则校验（`transaction.rs:260` 的 `rule_failures` 置空）→ 红**（我自己跑的） |
| t33 结构化 payload 的假绿 | **记账缺陷（新条目 N-2）** | `render_requirements` 至今仍是 `pub fn … -> String` + `filter_map`（畸形条目静默丢弃），t33 把它标成 passed 只因平台不接受 completed 里的 failed 项 |
| `:107/117/126` 注释词形（t37 报、t33 收口） | **部分证实（新条目 N-1）** | 三处旧词已改 `registry_name`；但 t33 新增的同段注释里**又出现一次** `*slot name*`/`**槽位名**`（`transaction.rs:186`/`:195`），同概念旧词仍在；未误动插件槽/布局槽概念 |

## 1. 我的装置与变异（9 组，判定只取 hash 稳定那次）

探针 `/tmp/nk-t34/core/tests/t34_probe.rs`：`lg04_…`（5 反例 + 2 正控）、`lg28_and_t32_…`（两入口同判据 + published 有损）、`lg30_…`（泛型逗号）、`lg07_siblings_…`（我自己的名字/kind）、`krn_c02_…`（读源码断言两句文档）——**修复后 5 passed**。

| 变异（副本内） | 红侧（我自己跑出来的） | 还原 |
| --- | --- | --- |
| M1b `schema_matches` 恒真 | 作者 `a_plugin_lock_schema_gate_is_not_line_order_dependent` + 我的 `lg04_…` | byte-identical |
| M2 严格入口改委派给有损入口 | 作者 `the_strict_requires_entry_refuses_a_malformed_entry`、`a_malformed_requires_entry_is_refused_not_dropped` + 我的 `lg28_and_t32_…` | byte-identical |
| M3a tokens.rs 去掉深度 | 作者 `…a_comma_inside_generic_arguments_is_not_a_list_separator` | byte-identical |
| M3b parse.rs 去掉深度 | 作者 `a_generic_argument_with_a_comma_still_derives_trait_labels` + 我的 `lg30_…` | byte-identical |
| M4 LG-07① 守卫永不命中 | 作者 `two_siblings_may_not_share_one_registry_name` + 我的 `lg07_siblings_…` | byte-identical |
| M5b `resolve_node` 回首先匹配 | 作者 `resolution_tests::an_ambiguous_replacement_selector_is_refused` | byte-identical |
| M6b 父级规则校验置空（`transaction.rs:260`） | 示例宿主 `parent_rule_rejects_a_child_that_misses_a_required_export` | byte-identical |

（M5/M6 首轮我改错位置/用了不编译的补丁：`RegistrationRule::validate` 置空**不**让示例变红——真正的求值点是 `transaction.rs:260` 的 `registry.header.registration_rule.validate(&snapshot)`；M3 首轮只跑了 face 级目标，故 tokens.rs 半边需要它自己的单测才红。两处都重新做对了，上面的表是第二轮结果。）

## 2. API 兼容与新公开符号

- `git show HEAD:core/.../authoring/parse/rules.rs | grep -n "pub fn parse_requirements_owned"` = `(value: &str) -> Vec<OwnedRequirementSpec>`；工作树同签名同语义（有损），t26 的 `-> Result<…>` 已被 t32 回滚。
- **自 HEAD 以来的新公开符号（core/src 全量 surface diff）：只有 `try_parse_requirements_owned` 一个**（`compact_admission`/`compact_registration_rule` 已在 HEAD 的提交里）。它**没有**出现在 `CHANGELOG.md` / `docs/roadmap-1.0.md`（`grep` 命中 0）→ **新条目 N-3**：`[0.1.6]` 未发布段应补记这个加法式入口；含义同前几轮：新增跨 crate 公开符号使 `tools/nichlink-package-audit` 的**隔离打包**那半要等版本线推进（今天 contents half 与 core 的打包都过），发布顺序仍是 core 在前。

## 3. 队长补核的两条

- **t33 的账目失真**：事实为 `render_requirements` 仍 `-> String` 有损（`filter_map`），本次未改；其"passed"为平台压力下的假绿 → 记为 **N-2**（账目缺陷，建议在总账里显式标注"平台逼出的假绿"这一类）。
- **注释词形**：`core/.../tree/transaction/transaction.rs` 的 :107/117/126 已改 `registry_name`；同文件 diff 的 28 条注释行 + 36 条非注释行（后者是 LG-07① 的守卫代码，不是改词）；**但新增注释里又写了一次旧词**（`:186` `*slot name*`、`:195` `**槽位名**`）→ **N-1**；该文件所有 `slot` 命中都是同一概念，未误动插件槽/布局槽。

## 4. 未覆盖范围

1. LG-27：只做了源码读（合并规则）+ 未自建 `RuntimeCheckSpec` 夹具、未做变异——作者钉子的红/绿我未复现。
2. LG-06 / LG-29：我用"变异让作者钉子红 + 源码读"验证，未自建 overlay/语法夹具（构造 `GraftPlan`+外部 `Registry` 的场景成本高）。
3. t32 的"重命名金丝雀仍红"（`registration macro test_object! does not match parent panel`）我未复跑。
4. 门禁：`cargo test -p nichlink-core`、`cargo test --workspace`、`cargo test -p nichlink-conventions`、`cargo fmt --all -- --check` 在哈希钉住树上**全绿**；`cargo clippy --workspace --all-targets --offline -- -D warnings` 在写作窗口**红**，点是并发成员的 `nichlink-run-method`（`function after_last_src is never used`）与 `nichlink-conventions`（`cannot find release_action_pin in crate`）——均不在 B4-core 的交付面，本批 12 个相关文件 sha256 跑前=跑后（见 §5）。

## 5. 门禁与并发

- 22:08 起的钉住树（**同一批 hash**）：core EXIT=0、workspace EXIT=0（58 个 ok 块、0 FAILED）、clippy EXIT=0、conventions EXIT=0（127 passed）、fmt EXIT=0（0 行）。
- 22:53 起的钉住树：core EXIT=0、workspace EXIT=0（58 个 ok 块、0 FAILED）、conventions EXIT=0（117 passed）、fmt EXIT=0（0 行）；clippy EXIT=101（上述两个他人在飞错误）。
- 我 pin 的 12 个文件（两个新钉子文件、catalog/rules/owned/tokens/transaction/resolution/graft/overlay/call_evidence + 本报告）在整轮跑前跑后 sha256 一致。
