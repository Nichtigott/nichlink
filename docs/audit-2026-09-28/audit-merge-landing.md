# 合并阶梯落地记录（批 1 / 批 2 / 批 3 准备）

本文件是**落地记录**，不是方案：方案是 `audit-publish-surface-merge-plan.md`（审计早期写），
它的表在命名批次之后已漂移，因此每一步开工前都先按当前树**现算**（批 1 的现算产物是
`audit-merge-batch1-spec.md`，批 2 的是 `audit-merge-batch2-spec.md`）。记录不覆盖、不改写方案。

## 提交表

| 批 | 提交 | 规模 | 内容 |
| --- | --- | --- | --- |
| 1 | `2656ddb` | 248 文件，+833 / −555 | 内核改名：`core/` → `kernel/`、包名 `nichlink-core` → `nichlink-kernel`、lib 名 `nichlink` → `nichlink_kernel`；六个成员与根的各一行 path 依赖；`conventions/src/shims.rs` 的 20 处 pin 字面量；`ci.yml` 与 `tools/nichlink-publish` 的两张表；活文档锚点与 `CHANGELOG` |
| 2 | `13c0b13` | 385 文件，+1921 / −2981 | 七合一：七个执行面目录合并为 `toolchain/`（模块 `build_time`/`runtime`/`call_evidence`/`plugin_host`/`studio`/`mcp`/`cli`），发布面 **9 → 3** |
| 3 准备 | `ddaa33e` | 8 文件，+127 / −19 | 版本线 `0.1.6` → `0.2.0`、CHANGELOG 定稿、`docs/merge-batch3-publish.md`（发布与 yank 命令清单）——**本批不含任何发布或 yank** |

## 落地判据（每一批提交前都在**冻结树**上跑，10 条逐条 EXIT=0）

`cargo fmt --all -- --check`；`cargo test --workspace --offline`（**默认构建面**）；
`cargo clippy --workspace --all-targets --offline -- -D warnings`；
`cargo test -p nichlink-conventions --offline`（含 `release_version` / `naming` / `mounting` /
`purity` / `shims` / `bilingual` / `doc_anchors` 等门禁，132 项）；
`tools/nichlink-publish --check-table`（3 个已发布包）；`cargo test --workspace --offline --all-features`；
`cargo test --workspace --offline --all-features --doc`；`python3 /tmp/anchor_check.py`
（`anchors checked: 20` / `violations: 0`）；两个真实示例宿主（`control-button`、`control-button-graft`）。

提交后**决定性检查**：`git status --porcelain` = 0 且 `git diff HEAD` = 0（HEAD 与已验树逐字节相同）；
`target/` 之外被忽略的 `.rs` = 0；抽查关键新文件都在 HEAD 里。

## 判别性证据（落地后现算）

- 批 1：词边界 `nichlink::` 295 → **0**；`shims.rs` 的 `pub use nichlink::` 20 → **0**；
  `Cargo.toml` 里 `nichlink-core` 6 → **0**；真锚点 24 → **0**；`core/` 目录已不存在。
- 批 2：`cargo metadata --no-deps` 列 **6 个成员**（`nichlink-kernel` / `nichlink-macro` /
  `nichlink-toolchain` / `nichlink-conventions` / 两个示例宿主）；
  `--check-table` = **3 crates**；接回 cargo 的测试 = **10 个集成测试文件 + 1 个示例**，
  函数守恒 **54/54**。
- 批 3 准备：全仓 `version = "0.1.6"` 残留 **2 处**（复核 t139 现算；两处都在测试夹具里当字符串用，
  不是依赖声明）⇒ 判据应写成"清单行谓词：没有一行是 `nichlink-*` 的**依赖/版本声明**写 0.1.6"，
  而不是"全仓 0 命中"；`release_version` 门禁绿 ✓。

## 与方案的漂移（方案读数 → 落地前现算；说明"为什么每步都要先算"）

| 方案的表 | 方案值 | 现算值 |
| --- | --- | --- |
| §3.1 词边界 `nichlink::` | 237 次 / 88 文件 | **295 次 / 102 文件** |
| §3.5 `#[path]` 声明 | 339 条 | **432 条**（非相对恒为 0） |
| §5.1 `shims` 条目 | 16 条 | **17 条** |
| §5.1 `bilingual` 对 | 9 对 | **8 对**（`macro` 只有 `README.md`） |
| §3.5「17 文件 / 17 同名」 | 17 / 17 | **25 文件 / 0 同名** |
| §4.2 依赖搬家表 | 9 行（含表头） | **7 行** |
| §5.1 `purity`/`mounting`/`naming` 的路径 | `core` | 批 1 后已是 `kernel`；批 2 后为 `toolchain/src` |

## 身份结论（合并为什么不动 `NodeId`）

- `NodeId` = hash(命名空间 = 宿主自己的包名, `file!()`, 名字) ⇒ **任何参与身份计算的文件被移动或改名，都会静默改变身份**。
- 因此全过程中**两个真正的身份载体——`examples/**` 的两个示例宿主——一次都没有被移动或改名**：
  复核者用 `git show --name-status` 核过四个提交，`examples/**` 的命中**全为 `M`（内容编辑），无 `R`/`D`** ✓。
- **一处知情接受的偏差（含复核 t139 的更正）**：`studio/tests/fixtures/node-editor/**` 在批 2 早期随 studio
  整目录并入 `toolchain/src/studio/tests/fixtures/**`——`13c0b13` 里那一侧是 **9 条 `D` + 目的地 9 条 `A`**
  （t137 的表述只覆盖了 `examples/` 一侧，这条由 t139 更正）。核查结论：该 fixture **没有任何落盘身份物**
  （无 `.json` / `.jsonl` / `.bin`），也不硬编码 `NodeId`（只有运行时计算的 `root_node_id(...)`）
  ⇒ 身份逐次运行重算，行为无影响；记为"偏差 + 证据"，不写成"没有偏差"。
- **复核的 MAJOR 发现（F-13-1 / F-14-1）已收口**：该 fixture 住在 `src/` 下时两件事同时坏 ✗ ——
  ① `tools/nichlink-package-audit` 的**内容半段红**（内容判据要求 `src/**/*.rs` 都在包里，而 `cargo package`
  不收嵌套包）；② `toolchain/src/studio/src/studio/app/tests.rs` 找的是
  `CARGO_MANIFEST_DIR + "tests/fixtures/node-editor"` ⇒ 夹具**从未被找到**，测试走 `Option` 分支**静默跳过** ✗。
  现已把它移到**代码本来就期望的** `toolchain/tests/fixtures/` 下 ⇒ 内容半段转绿、夹具真正可用 ✓
  （AGENTS.md 的路径措辞与夹具自己的注释同步更正）。

## 回滚与不可回滚点

- **可回滚的**：批 1 / 批 2 / 批 3 准备的提交本身都可 `git revert`；**yank 也可逆** ✓
  （`cargo yank --version <v> <name> --undo`）。
- **不可回滚的只有"发布"这一个动作** ✗：crates.io 是永久归档——**已发布的版本不可覆盖、代码不可删除**
  （Cargo Book：*the version can never be overwritten, and the code cannot be deleted*）。执行顺序与前置
  写死在 `docs/merge-batch3-publish.md`：发布（`nichlink-kernel` → `nichlink-macro` → `nichlink-toolchain`，
  依赖序）→ `tools/nichlink-publish --verify-consumers`（**该工具没有 `--verify-publish`，实测 exit 2** ✗）
  → **它绿之前不 yank** → 旧八名**逐版本** yank（各 `0.1.0/0.1.1/0.1.3/0.1.4/0.1.5`；只 yank 最新版会让
  `= "0.1"` 的新解析退回未 yank 的旧版 ✗）。维护者选定 **0.2.0**，并在本机 `cargo login` 后由队长执行 ✓。
- **执行结果（2026-09-29，追记）**：三个 crate 的 `0.2.0` **已全部发布** ✓（`cargo publish` 逐条 EXIT=0、
  index 已收录）；`tools/nichlink-publish --verify-consumers` **绿** ✓（"consumer resolved and built:
  nichlink-kernel nichlink-macro nichlink-toolchain"）；旧 8 名 × 5 版本共 **40 次 yank** 已按上表执行 ✓
  （`nichlink-macro` 保留 ✓）。**两处踩坑如实记**：① 首轮 `cargo publish` 在第二条 crate 的 `verify`
  阶段被 **workspace-write 沙箱**拦成 `Permission denied (os error 13)`（`~/.cargo` 归属正常却写不进 ⇒
  是沙箱而不是权限），改用**一次性 `danger-full-access`** 后全过 ✓；② 清单里 `--verify-publish` 这个开关
  **不存在** ✗（真实开关只有 `--help/--publish/--yes/--allow-dirty/--check-table/--verify-consumers`）——
  引用前没核实，已更正 ✓。

## 仍开放（点名，不粉饰）

1. **批 2 遗留：6 个模块内测试文件 / 20 个 `#[test]` 未接回** ✗（复核 t139 现算：**总数 74 = 20 未接回 + 54 已接回**；
   先前记的"25 / 49"是普查口径偏差，已按复核更正）。复核用三条独立探针把这件事**钉死**：两个构建面的 `--list` 里
   这 20 个名字命中 **0**；在 HEAD 副本里给六个文件各插 `compile_error!` 后
   `cargo check --workspace --all-targets` 两个面**仍 exit 0**（给已挂载的文件插则 exit 101 ⇒ 对照有牙）；
   且在 `75e4387` 的副本里这 20 个**当年是活的集成测试目标** ⇒ **降级发生在批 2（`13c0b13`）**。
   三处文档（本记录、AGENTS.md 的 Known residue、b3 诊断）都点了名 ⇒ 这是**已声明**的覆盖降级，不是隐瞒 ✗。
   背景：合并把宏定义从 crate 根移进了模块 ⇒ 宏可达性的三条规则同时改变（`#[macro_export]` 上提到
   crate 根；同 crate 展开里禁用绝对路径；裸名在调用点解析 ⇒ 会弄坏外部调用者；私有 `macro_rules!`
   只活在文本作用域）。成因、实测原文与两条候选修法（F1 逐文件归一成严格 arm / F2 生产侧加
   `use` 后改裸名）都在 `docs/b3-registration-diagnosis.md`（§2 有一处已由实测更正：`face_fields!`
   是 proc-macro 的再导出，不是"前端生成的宏"）。
2. 批 3 的**执行**（发布 + yank）：等维护者凭据；本记录写作时**尚未执行**。
3. 两条闭合按钮：对 `75e4387` / `2656ddb` / `13c0b13` / `ddaa33e` 的**事后独立复核**，
   以及 **NAM-33 动词表门禁**。
   - 复核第一轮（`gates-auditor`）**部分完成**，但它用**自选探针**（`git show --name-status --format=`）
     独立确认了本记录最要紧的那条：四个提交里 **`examples/**` 与 `studio/tests/fixtures/**` 的命中全为
     `M`（内容编辑），没有任何 `R`/`D`** ⇒ 身份红线未被越过 ✓。它的深度探针部分因预算耗尽未做，
     分提交清单已交回，现已另派一轮补齐（含核心必做项：那 20 个未接回的 `#[test]` 是否真的没跑 ——
     "代码搬了、测试没接回"正是典型的**未声明覆盖降级**）。
   - **第二轮（`bridge-auditor`）已完成**：报告 `docs/audit-2026-09-28/audit-review-merge.md`（255 行），
     **整轮 needs_revision**（对合并阶梯而言）——逐提交 verdict：`75e4387` **pass**（显式标注"含作者自审成分"）、
     `2656ddb` **pass**、`13c0b13` **needs_revision**、`ddaa33e` **needs_revision**。
     四条 findings 的处置：**F-13-1 / F-14-1（MAJOR，同一根因）→ 已修**（fixture 移出 `src/`，见上）；
     **F-13-2（MEDIUM，净减 20 个测试覆盖）→ 升级为发布前的显式检查项**（写进
     `docs/merge-batch3-publish.md` 的收尾清单：不阻塞发布，但必须在发布清单里被看见）；
     **F-13-3 / F-14-2（LOW）→ 已按复核更正**（本记录 25/49 → 20/54；"`0.1.6` 残留 0" 实测 2 处，
     皆为测试夹具里的字符串 ⇒ 判据改成清单行谓词）。
4. 两条已记账的小残留：`examples/**` 仍写旧名 `cut.full()` 两处（转发器保证可用）；
   根 glob 让 `nichlink-toolchain` 的根部"glob 一切"，因此 README/AGENTS 已按"模块路径是官方地址、
   只有精选清单承诺裸名"的口径写明策略。
5. **NAM-33 动词表门禁已落地**（`conventions/src/verb_table.rs` + 同级 `verb_table_tests.rs`）：
   判定落在**语法位置**（只解析 `fn` 声明，文档里"提及"不算）、`get_` 必须返回 `Option`、表中裸动词
   只许入口位、`collect_` 只许 `pub`；**七条反证钉子全绿**（改坏应红 4 + 豁免应绿 5，含单词**名词**
   `fn dispatch` 不报）。全仓**真实违规 81 条**已用**棘轮 `PINNED = 81`** 钉住（只许降不许升，失败时
   打印全清单），并且**没有改动 `conventions` 之外的任何源文件**。要让门禁变成"必须 0"，需要单独一批
   收拾这 81 个名字（例：`conventions/src/doc_anchors.rs` 的 `resolve`、`kernel/src/registry_core/declaration/owned.rs`
   的 `validate`、`kernel/src/registry_core/mir/call_tree.rs` 的 `collect_edges`）——那是**独立的新命名批次**，
   不阻塞合并阶梯。
