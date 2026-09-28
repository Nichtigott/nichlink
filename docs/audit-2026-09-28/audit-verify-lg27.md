# 独立验证：LGC-LG-27（`merge_authored` 的 `runtime_checks`）—— 证实

- 验证者：kernel-auditor（t84，**非作者**；作者是 t26 gates-auditor）。本单补掉 t34 报告里**最后一个 partial**
- 真实树**零改动**（本轮只写本文件）；我的夹具与变异在 `/tmp/nk-t84`（自带 `CARGO_TARGET_DIR=/tmp/nk-t84-target`）
- 判定：**证实** —— `core/src/registry_core/declaration/owned.rs` 的 `merge_authored`（`impl RegistrationSnapshot`）遵守"作者侧非空即应用（与 `flow` 同规则），空表示文件没提"

## 1. 我自己的夹具（t34 当时缺的就是这个）

副本内 `core/tests/t84_probe.rs`：复用我 t34 的 `RegistrationSnapshot` 夹具骨架，新增一个 `with_runtime_checks(...)` 构造器，用**真实的 `nichlink::RuntimeCheckSpec` 枚举值**（`FiniteNumber` / `NonEmptyText` / `CoordinatesInViewport` / `TextLength { min, max }`），3 条用例：

| 用例 | 编译期（self） | 作者侧（authored） | 断言 |
| --- | --- | --- | --- |
| `non_empty_authored_runtime_checks_replace_the_compiled_ones` | `[FiniteNumber, NonEmptyText]` | `[CoordinatesInViewport]` | 合并后 == `[CoordinatesInViewport]`（作者侧是一次编辑） |
| `an_empty_authored_list_keeps_the_compiled_checks` | `[TextLength{2,8}]` | `[]` | 合并后 == `[TextLength{2,8}]`（空＝文件没提） |
| `the_author_side_has_no_expression_for_clearing_the_compiled_checks` | `[FiniteNumber]` | `[]` | 合并后非空（把"无法清空"作为事实钉住，见 §3） |

修复后的树上 **3 passed**（`cargo test -p nichlink-core --offline --features syntax --test t84_probe`）。

## 2. 变异反证（两个方向，都在副本内）

| 变异 | 红侧原文 | 还原 |
| --- | --- | --- |
| **MA 旧行为**：删掉 `if !authored.runtime_checks.is_empty() { … }`（静默丢弃作者侧） | 用例 1 FAILED：`left: [FiniteNumber, NonEmptyText] right: [CoordinatesInViewport]` | copy==pristine ✓，sha256 `82754addcf14…` |
| **MB 过修**：无条件 `self.runtime_checks = authored.runtime_checks.clone();` | 用例 2 FAILED `left: [] right: [TextLength { min: 2, max: 8 }]`，用例 3 亦 FAILED | copy==pristine ✓，同一 sha256 |

MA 正是这条 finding 的旧形状（源码里对 `runtime_checks` 的编辑成为空操作）；MB 是反方向的守卫，说明"非空"这个条件真的在起作用，而不只是"总是应用"。

## 3. 语义边界："非空即应用"意味着作者无法清空编译期校验 —— 我的判断

**事实**：作者侧只有两种可表达的状态——非空（＝一次编辑，覆盖编译期）与空（＝文件没提，编译期保留）。因此**经源码无法清空编译期校验**；用例 3 把这条钉住。

**判断：当前形态可接受**，理由三条：
1. **两侧的信息量不对称**：编译期校验来自被编译的注册面数据（trait 关联常量那一路的证据），作者侧同一字段来自热重载的文件文本。"空"在这套合并里已有既定含义（文件没提），而"清空"是一次**破坏性**编辑；给同一个拼法两种含义，正是这个合并函数存在的意义所要避免的静默形状。
2. **有合规的替代表达**：要"不跑某条编译期校验"，作者可以在源码里**改成更弱但非空的**检查（例如把 `NumberInRange` 换成 `FiniteNumber` 并显式写下它）——语义上等价于放宽，而且**留下痕迹**（diff 里看得见新值）。这比"空列表=清空"更可审计。
3. **与同函数的邻域一致**：`flow` 用的是同一条规则（`if authored.flow.is_declared()`），`flow_provider` 用的是 `is_some()`——都是"作者侧表达了什么就应用什么，没表达就保留"。给 `runtime_checks` 单独造一套"空即清空"的拼法是局部特例。
**报告、不建议改**：如果将来确实需要"显式清空"，可加一个显式的哨兵（例如 `runtime_checks: none` 这样的字面量，或一个 `clear_runtime_checks` 字段），让"清空"成为**写下的动作**而不是"空"的第二种解读。这不属于本轮范围。

## 4. 门禁（同一批 hash）

`cargo test -p nichlink-core --offline` **0**（187 passed）／`cargo clippy --workspace --all-targets --offline -- -D warnings` **0**／`cargo test -p nichlink-conventions --offline` **0**（132 passed）／`cargo fmt --all -- --check` **0**（0 行）。
`cargo test --workspace --offline` 在写作窗口**红**，且**不在我这批**：先是 `studio::app::tests::project_root::a_write_without_a_selected_project_is_refused` 与 `studio::app::tests::graft::…is_refused_not_guessed` 两条（studio 在飞），随后变成 core 自己的 `registry_core::authoring::snapshot::tests::the_rule_path_field_names_the_location_the_rule_would_be_read_from`——而 `core/src/registry_core/authoring/snapshot/snapshot.rs` 的 mtime 与我查看它的时间**只差 1 秒**（23:37:09 改、23:37:10 查），即有人正在编辑它。我本轮没有改任何源码或他人产物；等该文件落定后重跑即可关掉这条。

## 5. 未覆盖 / 新发现

1. **未覆盖**：我没有把 `merge_authored` 的**其它**字段逐一做变异（本单只针对 `runtime_checks`）；`runtime_checks` 从文件文本解析成 `RuntimeCheckSpec` 的那一段（`rule_method_strings` 之外的字段解析）不在本单范围。
2. **新发现（low）**：`merge_authored` 的 `runtime_checks` 分支与 `flow` 分支都靠"非空/已声明"这一形状判定"文件是否提过"，但二者的**文档措辞各自独立**；建议在函数头上收成一句通用规则（"只有作者侧表达了某字段才应用它，未表达则保留编译期值"），免得后面新增字段的人再写一套。只报告，未改源码。
