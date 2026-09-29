# B3 诊断：`REGISTRATION` 为何在 `canonical|shuffled|parens` 缺失（只读，一页）

诊断对象：`toolchain/src/runtime/tests/face_fields.rs` 等 6 个「依赖 crate 内部/宏生成名」的测试文件
（t122 分类表的 (b) 类；t123–t126 四次尝试均卡在此处）。**本单只读，未改任何源码。**

## 1. 宏链：`REGISTRATION` 是**第四级转调**才定义的

| 级 | 宏 | 位置 | 导出 | 作用 |
| --- | --- | --- | --- | --- |
| 1 | `__nichlink_object!` | `runtime/src/macros/face_objects.rs:8` | `#[macro_export]` ✓（:7） | 补 `collector:`（缺省 `development`）后转调 → `$crate::__control_object!`（:10 / :13） |
| 2 | `__control_object!` | `…/face_objects.rs:19` | `#[macro_export]` ✓（:18） | **严格 arm**（:35–135）末尾转调 → `$crate::__registration_face!`（:81）；不匹配者走**宽容前端** → `$crate::runtime::face_fields!`（:161） |
| 3 | `__registration_face!` | `runtime/src/macros/face_registration.rs:82` | `#[macro_export]` ✓（:81） | 产出 `pub const NODE_ID`（:139）与 **`pub const REGISTRATION`（:161）**，再转调 → `$crate::__submit_registration!($collector; REGISTRATION)`（:224） |
| 4 | `__submit_registration!` | 同文件（runtime 宏族） | — | 登记到运行期 |

⇒ **`REGISTRATION` 定义在第 3 级的展开里、落点就是调用者所在的那个模块作用域**（`mod canonical { … }` ⇒ `canonical::REGISTRATION`）。
⇒ `face_fields.rs:33` 的 `canonical::REGISTRATION` 之所以找不到，只可能是**第 3 级没有被展开**（链断了），而不是「定义在别处」。

## 2. 链为什么会断：**同一 crate 内的 `macro_export` 宏不能经绝对路径调用**

- 编译器原文（t123 第 2 轮实测）：`error: macro-expanded `macro_export` macros from the current crate cannot be referred to by absolute paths` ✗
- 本仓自己的记载（`runtime/src/authoring/operations/migration_tests.rs:66-69`）：`crate::runtime::test_object!` 这类**路径写法**只在“父模块改名”等场景里被讨论，而它能跑通的形态是**同文件/同模块文本作用域**下的裸名调用 ✓。
- 链里有两处**路径调用**：`:81` 的 `$crate::__registration_face!` ✓（手写宏，路径可用）与 **`:161` 的 `$crate::runtime::face_fields!`** ✗ —— 后者是**前端生成的**宏（`face_fields!` 由 `authoring/manifest/face` 前端按父级形状产出 ✓，与 `__face_rule_or!`（:118）同类）⇒ **它是整条链最脆的一环** ✗。
- 因此语义根因：**任何走进“宽容 arm”（`:157-162`）的声明都会掉进前端路径调用** ✗ ⇒ 展开不成立 ⇒ `canonical|shuffled|parens|external_shuffled|handle_without_preset_or_parts…` 里的 `REGISTRATION`/`NODE_ID` 全缺 ✗（t126 计数 19 条正落在这几个模块 ✓）。

## 3. 能跑的既有测试 vs `face_fields.rs` 的差异

| 维度 | 能跑的既有形态 | `toolchain/src/runtime/tests/face_fields.rs` |
| --- | --- | --- |
| 调用可见性 | **文本作用域**（同文件/同模块树内裸名；如 `runtime/src/lib.rs:63` 的别名宏 `$crate::external_object!{…}` ✓ 由本模块直接可见 ✓） | 挂到 `macros/face_objects.rs` 的**子模块**里 ✓（t124/t126 已证实这步有效：88 → 34），但**调用仍带前缀**（t125 铁证 `nichlink_toolchain::runtime::__nichlink_object!` ✗，t126 已剥成裸名 ✓ 仍有 19 条 ✗） |
| 字段分隔 | 严格 arm：`collector:, kind:` 之后**全部逗号**、且顺序须与匹配器（`face_objects.rs:35-61`）一致 | `shuffled`/`parens`…用 **`;` 分隔**（`face_fields.rs` 里 `;$` 行数很多 ✓）⇒ **落入宽容 arm** ✗ |
| `collector:` | 可省（缺省 `development`，:13） | 省略（同左）✓，非差异 |
| `needs_registry` | `$(needs_registry: $expr,)?`（:44）⇒ 可选、**必须带逗号** | 写了 `needs_registry: true` ✓ 但**顺序/分隔与匹配器不一致**时无效 ✗ |
| `parent` | `$(parent: $expr,)?`（:45） | 写了 `parent: crate::runtime::registry_core::root_node_id("face-fields-test")`（t126 已归一 ✓） |
| `registry_rule` | `$(registry_rule: $rule:expr,)?`（:48） | 写了，但在 `;` 版里排到 `kind:` 前 ✗ ⇒ **顺序不合** |

## 4. 4 条 `cannot find nichlink_toolchain in the crate root` 的来源（file:line）

宏体里没有残留 ✓（`grep -rn nichlink_toolchain runtime/src/macros/*.rs` = 空 ✓）。定向 grep 命中的是：

- `toolchain/tests/build_time_outside_src_layout.rs（原路径见下）`（`use nichlink_toolchain::build_time::{check_for, face_views};` ✗ 真代码）
- `toolchain/tests/build_time_outside_src_layout.rs（原路径见下）`、`:41`（**字符串字面量**里的 `nichlink_toolchain::root_object!` / `root_node_id` ✗ 不参与编译）
- `toolchain/tests/build_time_missing_source_tree.rs（原路径见下）`、`:58`（`nichlink_toolchain::build_time::check_for(...)` ✗ 真代码）

⇒ 这 4 条**不是**宏体里的旧名，而是**同批未搬的 (a) 类文件**里残留的旧 crate 名 ✓（它们属 (a) 类，按计划搬进 `toolchain/tests/` 时统一改写为 `nichlink_toolchain::build_time::…` ✓）；编译器把它们报在 (b) 文件的**调用点**上，是因为 (b) 的展开把它们带了进来 ✓。

## 5. 建议的**单一修法**（test-side，不动生产代码）

**把 6 个 (b) 文件的宏调用全部归一成「严格 arm 的形状」**：`collector`（可省）→ `kind` 之后按 `face_objects.rs:35-61` 的**声明顺序**、**一律逗号**（把 `;` 换成 `,`，把 `registry_rule` 移回 `parent` 之后），使其**不落入宽容 arm**（`:157`）⇒ 链只需走到 `__registration_face!`（手写、路径可用 ✓）即可产出 `REGISTRATION` ✓。
次要（若个别声明必须走到前端）：让该文件在**它自己的模块里**加一行文本作用域别名（如 `use crate::runtime::face_fields as _;` 不可行时，改成把 `face_fields!` 也做成手写别名宏，属**生产侧**改动 ⇒ 建议单独立单，**不在本单做** ✗）。

## 6. 未解决项（如实）

- `canonical`（逗号、且顺序看似合规 ✓）也报了 15 条 ✗ ⇒ 说明**除「宽容 arm」外还有第二个未定位因素**（下一步建议：对 `canonical` 单独做一次最小复现，或对 `__registration_face!` 展开取 `cargo expand` 级别的证据 ✓）。
- 本次未做任何临时改动（无 /tmp 复现落地、无临时断言）⇒ 无需回滚 ✓。
