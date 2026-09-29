# 合并阶梯事后独立复核（批 1 / 批 2 / 批 3 准备）

- 复核人：`bridge-auditor`（本单只读；唯一写入是本文件）
- 复核对象：`75e4387`（命名批次 B7+B8 + A1 収平）、`2656ddb`（批 1：`core/`→`kernel/`）、
  `13c0b13`（批 2：七合一为 `nichlink-toolchain`）、`ddaa33e`（批 3 准备：版本线 0.2.0）
- 承接：`gates-auditor` 的 t137 **部分完成**（它交回了分提交清单，未做深度探针、未创建本文件）
- 方法：**全部探针自建**；四个提交各用 `git archive <sha>` 解到 `/tmp/t139/<sha>`（不改工作树、
  不碰 `.git`），每个副本用自己的 `CARGO_TARGET_DIR`；变异实验只在 `/tmp/t139/mut`（HEAD 的
  archive 副本）里做。**未重跑作者的任何脚本作为证据**（`tools/*` 只在被复核的副本里被当作
  *被测对象*运行）。
- **作者披露**：我参与过命名批次的部分站点改动（`mcp` 三个模块改名、`run_method` 七处収平与尾部
  清扫等）⇒ **对 `75e4387` 的 verdict 含作者自审成分**，请在合议时按“非作者优先”对待该条。

## 0. 结论速览

| 提交 | verdict | 主要依据（本报告节） |
| --- | --- | --- |
| `75e4387` | **pass（含作者自审成分）** | 旧 shim 路径仍解析（自建 crate 探针 exit 0）；改名后的名字不是包（8/8 `-p` 失败）；身份红线 0 命中；§2 |
| `2656ddb` | **pass** | `core/` 不存在、`kernel/src/lib.rs` 在；`nichlink-core` 不可解析；历史 `pub use` shim 仍编译（`cargo check -p nichlink-run-method` exit 0）；§3 |
| `13c0b13` | **needs_revision** | 七旧包名 7/7 `-p` 失败 ✓、`--check-table` 3 crates ✓、模块暴露 ✓；**但** ①`tools/nichlink-package-audit` 内容半段**红**（7 个 fixture 文件不在包里，本提交引入并延续到 HEAD）②覆盖净减 **20** 个测试（已声明为遗留、未修）③记录数字错（25/49 vs 20/54）；§4 |
| `ddaa33e` | **needs_revision** | 版本线一致（根 0.2.0、四个成员 `version.workspace = true`、`--workspace-version` = 0.2.0）✓；**未执行任何发布**（0 tag、0 `.crate`、diff 仅 8 文件）✓；**但**继承 ① 的 package-audit 红 ⇒ 批 3 的“发布前门禁全绿”不能算成立；另“0.1.6 残留 0”实测 2 处（皆为测试夹具字符串）；§5 |

**整轮判定：`needs_revision`** —— 阻点只有一条实质项：`nichlink-toolchain` 的**包内容半段红**
（F-13-1 = F-14-1，同一根因、两处记账），它在批 3 真正 `cargo publish` 之前必须收口；其余为记账
与覆盖存量（F-13-2/F-13-3/F-14-2）。

---

## 1. 核心判别探针：那 25 个“未接回”的 `#[test]` 到底有没有跑

### 1.1 先量文件里的 `#[test]`（**实测 20，不是 25**）

```sh
for f in $(git ls-files 'toolchain/src/*/tests/*.rs'); do
  printf '%-52s %s\n' "$f" "$(grep -c '^\s*#\[test\]' $f)"; done
```

| 文件 | 行首 `#[test]` | 任意 `#[test]` 变体 | 解析出的 `fn` 名 |
| --- | ---: | ---: | ---: |
| `toolchain/src/runtime/tests/face_arm_defaults.rs` | 3 | 3 | 3 |
| `toolchain/src/runtime/tests/face_fields.rs` | 6 | 6 | 6 |
| `toolchain/src/runtime/tests/face_preset_parts.rs` | 1 | 1 | 1 |
| `toolchain/src/runtime/tests/kind_only_registry_name.rs` | 1 | 1 | 1 |
| `toolchain/src/runtime/tests/path_compat.rs` | 8 | 8 | 8 |
| `toolchain/src/call_evidence/tests/collector_integration.rs` | 1 | 1 | 1 |
| **合计** | **20** | **20** | **20** |

- `docs/audit-2026-09-28/audit-merge-landing.md:71` 与 `AGENTS.md`（“Known residue … 25 `#[test]`
  items”）都写 25。**实测 20**。
- 记录里的**总数** 74 是对的：**54**（`toolchain/tests/*.rs` 的 10 个集成测试文件，逐文件实测
  2+3+6+27+1+1+5+1+4+4 = 54，与落地记录“函数守恒 54/54”一致）+ **20** = 74 ✓。错的是**拆分**
  （记录写 49/25，实为 54/20，差 5）。

### 1.2 现在：两个面各跑一次，20 个名字一个都不在

```sh
cargo test --workspace --offline            -- --list | grep -c ': test$'   # 654
cargo test --workspace --offline --all-features -- --list | grep -c ': test$'   # 1009
```

- 把 §1.1 解析出的 **20 个测试名**逐个拿去两个清单里找：**默认面命中 0 / 20，`--all-features`
  面命中 0 / 20**。
- **正对照**（证明“找不到”不是因为清单方法失效）：`toolchain/tests/plugin_host_admission.rs` 的
  三个 `#[test]` 名在 `--all-features` 清单里**全部命中**；在默认面**一个都不命中**——因为那三个
  测试门控在 `plugins` 之后，默认特性下 `plugins` 关着。这正是 AGENTS.md 说的“`0 passed` 读作
  ‘什么都没跑’”的同一种形状。

### 1.3 归属：用变异把“这些文件不在任何 target 里”定死

`/tmp/t139/mut` = `git archive HEAD` 的副本；在**六个文件**顶部各插一行
`compile_error!("T139 PROBE: this file must not be compiled");`，然后：

| 命令（副本内，独立 `CARGO_TARGET_DIR`） | 结果 |
| --- | --- |
| `cargo check --workspace --all-targets --offline --all-features` | **exit 0**（`Finished dev profile`） |
| `cargo check --workspace --all-targets --offline`（默认特性） | **exit 0** |

⇒ 六个文件**不属于任何 cargo target**（真属于的话，`compile_error!` 必然让上面两条红）。

**牙齿对照**（同一副本，把变异放进一个**确实被编译**的文件 `toolchain/src/runtime/src/lib.rs`）：

```
error[E0753]: expected outer doc comment      # exit 101
error: could not compile `nichlink-toolchain` (lib) ...
```

⇒ 探针有牙：同一个变异手法打在挂载文件上，构建立刻红 ✓。

### 1.4 跨提交：这 20 个测试在合并**之前是活的**

在 `/tmp/t139/75e4387` 副本里逐文件 `--list`（当年它们是 `run_method/tests/`、`debug_method/tests/`
下的集成测试目标）：

| 当年目标 | `--list` 条目 |
| --- | ---: |
| `run_method/tests/face_fields` | 6 |
| `run_method/tests/face_arm_defaults` | 3 |
| `run_method/tests/face_preset_parts` | 1 |
| `run_method/tests/kind_only_registry_name` | 1 |
| `run_method/tests/path_compat` | 7（加 `--features authoring` = **8**） |
| `debug_method/tests/collector_integration` | 1 |
| **合计** | **19 + 1（authoring 面）= 20** |

⇒ **净覆盖减少 20 个测试**，且**发生在 `13c0b13`**（合并把 `tests/` 目录放进了
`toolchain/src/<module>/tests/`，那里不是任何 target 的输入）。

### 1.5 结论（本单核心必做项）

- **是覆盖降级**：20 个原本在 CI 默认面/`authoring` 面跑的测试，自 `13c0b13` 起一个都不跑。
- **但不是“未声明的降级”**：`docs/audit-2026-09-28/audit-merge-landing.md:71`（“批 2 遗留：6 个
  模块内测试文件 / 25 个 `#[test]` 未接回”）、`AGENTS.md`（Known residue）与
  `docs/b3-registration-diagnosis.md`（成因 + F1/F2 两条候选修法）三处都点了名。
- **要修的是账与口径**：(a) 数字 20（非 25）、拆分 20/54（非 25/49）；(b) 这批测试**曾经是活的**，
  所以“未接回”实为“合并当轮净减 20 个测试”，建议在遗留条目里写成这个事实（见 F-13-2/F-13-3）。

---

## 2. `75e4387`（命名批次 B7+B8 + A1 収平）— **verdict：pass（含作者自审成分）**

| 探针 | 命令（副本 `/tmp/t139/75e4387`） | 原始结果 |
| --- | --- | --- |
| 旧 shim 路径仍可 `use` | 自建 crate `/tmp/t139/probe75`：`use nichlink_run_method::{authoring::validation, registry_core, runtime::trace::artifact, tree};` | **exit 0** ✓（`Checking t139-probe-75` → `Finished`） |
| 改名后的名字不是包 | `cargo pkgid -p call_trace / evidence / nodes / index / face_manifest / geometry / support / validation` | **8/8** 报 `package ID specification … did not match any packages` ✓ |
| 身份红线 | `git diff-tree --no-commit-id --name-status -r 75e4387 -- examples studio/tests/fixtures` | **0 命中**（194 文件：45 A / 46 D / 103 M，全仓改名/搬迁不在红线区）✓ |

**findings**：无阻断项。备注：本条 verdict **含作者自审成分**（我改过该批次的部分站点），建议由
非作者再抽一条（例如 `path_compat` 的 8 个路径承诺）做反证。

## 3. `2656ddb`（批 1：内核改名）— **verdict：pass**

| 探针 | 命令（副本 `/tmp/t139/2656ddb`） | 原始结果 |
| --- | --- | --- |
| `core/` 已不存在 | `test -d core` | 不存在 ✓（`kernel/src/lib.rs` 在 ✓） |
| `nichlink_core` 不可解析 | `cargo pkgid -p nichlink-core` | `error: package ID specification 'nichlink-core' did not match any packages`（提示 `nichlink-cli`）✓ |
| 历史 `pub use` shim 仍编译 | `cargo check -p nichlink-run-method --offline` | **exit 0** ✓（`registry_core`/`tree`/`plugin`/`face_file`/`validation` 等 17 条 pin 所在的 crate 编译通过） |
| 成员面 | `cargo metadata --no-deps` | 12 个包：`nichlink-kernel` + 八旧名 + 两个示例 + `nichlink-conventions` ✓（`nichlink-core` 已消失） |
| 身份红线 | 同上口径 | **0 命中**（358 文件）✓ |

**findings**：无阻断项。

## 4. `13c0b13`（批 2：七合一）— **verdict：needs_revision**

| 探针 | 命令（副本 `/tmp/t139/13c0b13`） | 原始结果 |
| --- | --- | --- |
| 七个旧包名不可解析 | `cargo pkgid -p nichlink-build-method`（run/debug/plugin-host/studio/mcp/cli 同） | **7/7** 报 `did not match any packages` ✓ |
| 发布表 | `sh tools/nichlink-publish --check-table` | `dependency table matches the manifests (3 crates)`，**exit 0** ✓ |
| 模块暴露 | 自建 crate `/tmp/t139/probe13c`：`use nichlink_toolchain::{build_time, plugin_host, runtime};` + `features = ["run","build","plugins"]` | **exit 0** ✓（三个模块路径都解析得到；更强的 `probe13b` 只报 `E0603 struct 'Node' is private` ⇒ 模块可达、具体条目私有，符合预期） |
| 身份红线 | `git diff-tree … -r 13c0b13 -- examples studio/tests/fixtures toolchain/src/studio/tests/fixtures` | `examples/` **16 M（0 R/D）** ✓；`studio/tests/fixtures/` **9 D** + `toolchain/src/studio/tests/fixtures/` **9 A** = fixture 包整体搬迁 ⇒ **与落地记录 §身份结论的“知情接受的偏差”一致**（该 fixture 无落盘身份物）✓ |
| **包内容半段** | `sh tools/nichlink-package-audit`（副本内） | **exit 1**：`contents failed: nichlink-toolchain` ✗ |

**F-13-1（MAJOR）`nichlink-toolchain` 的包内容漏掉 7 个文件（本提交引入，HEAD 仍在）**

我的探针原始输出（`13c0b13` 副本，与 HEAD 逐条相同）：

```
error: nichlink-toolchain does not package src/studio/tests/fixtures/node-editor/build.rs
error: nichlink-toolchain does not package src/studio/tests/fixtures/node-editor/src/control/control.rs
error: nichlink-toolchain does not package src/studio/tests/fixtures/node-editor/src/control/object/node_editor/node_editor.rs
error: nichlink-toolchain does not package src/studio/tests/fixtures/node-editor/src/control/object/node_editor/object/object.rs
error: nichlink-toolchain does not package src/studio/tests/fixtures/node-editor/src/control/object/node_editor/registry_rule/registry_rule.rs
error: nichlink-toolchain does not package src/studio/tests/fixtures/node-editor/src/control/registry_rule/registry_rule.rs
error: nichlink-toolchain does not package src/studio/tests/fixtures/node-editor/src/lib.rs
  contents failed: nichlink-toolchain
exit=1
```

- `file`：`toolchain/src/studio/tests/fixtures/node-editor/**`（7 个）+ 判据脚本
  `tools/nichlink-package-audit`。
- 成因：合并把 fixture 包从 `studio/tests/fixtures/` 搬进了
  `toolchain/**src**/studio/tests/fixtures/`；内容半段的规则是“每个 `src/**/*.rs` 模块都要在包里”，
  而 `cargo package --list` 不会收进一个**嵌套包**（它有自己的 `Cargo.toml`）⇒ 判据与包的规则
  直接冲突。
- 影响：`nichlink-toolchain` 交付的 tarball 缺这 7 个文件；`prototype-fixtures` 门控的测试目标在
  打包副本里没有输入。**这是批 3 `cargo publish` 之前必须收口的项**。
- `requiredFix`：按 `AGENTS.md` 已经写下的目标路径把 fixture 迁出 `src/`
  （`toolchain/studio/tests/fixtures/node-editor/`），并同步 `prototype-fixtures` 测试目标的路径与
  `conventions/src/features.rs` 的 `OFF_BY_DEFAULT` 文句；若选择保留在 `src/` 下，则必须同时改
  `tools/nichlink-package-audit` 的判据（写明“嵌套包不是本 crate 的模块”）并给出该判据的钉子。
  两条路都要有**判别性**证据（`sh tools/nichlink-package-audit` ⇒ exit 0）。

**F-13-2（MEDIUM）本提交净减 20 个测试的覆盖**

- `file`：`toolchain/src/runtime/tests/*.rs`、`toolchain/src/call_evidence/tests/collector_integration.rs`。
- 证据见 §1.3/§1.4：这 20 个 `#[test]` 在 `75e4387` 是**活的集成测试目标**（19 默认面 + 1
  `authoring` 面），在 `13c0b13` 之后两个面**均为 0**；变异探针证明它们不在任何 target 里。
- `requiredFix`：落地 `docs/b3-registration-diagnosis.md` 的 F1/F2（或在 `toolchain/tests/` 里
  重新接回等价目标），并让“测试文件守恒”成为可判据（例如 `--list` 叶子名集合对比：合并前后
  应交出同一集合，差集必须为空或逐条声明）。

**F-13-3（LOW）记录数字与树不符**

- `file`：`docs/audit-2026-09-28/audit-merge-landing.md:71`、`AGENTS.md`（Known residue 段）。
- 实测：20 个 `#[test]`（非 25）；拆分 20/54（非 25/49），总数 74 一致。
- `requiredFix`：把两处数字改成 20 与 20/54，或改成“计数不在此复述”（本仓对会漂移的数字的既有
  口径），并把 §1.4 的“曾是活目标”事实写进遗留条目。

## 5. `ddaa33e`（批 3 准备）— **verdict：needs_revision**

| 探针 | 命令（副本 `/tmp/t139/ddaa33e`） | 原始结果 |
| --- | --- | --- |
| 版本线一致 | `grep -rn '^version = "0\.' --include=Cargo.toml .`；四成员 `version.workspace = true` 实查 0 个例外 | 根 `Cargo.toml:7` = **0.2.0** ✓；`kernel`/`macro`/`toolchain`/`conventions` 全部跟随 workspace ✓；两个示例与 fixture 是自己的 0.1.0（非发布面）✓ |
| 工作区版本 | `sh tools/nichlink-publish --workspace-version` | **0.2.0**，exit 0 ✓ |
| 发布表 | `sh tools/nichlink-publish --check-table` | `matches the manifests (3 crates)`，exit 0 ✓ |
| **未执行任何发布** | `git diff-tree --no-commit-id --name-status -r ddaa33e`；`git tag`；`find . -name '*.crate'` | diff = **8 文件 / +127 −19**（`CHANGELOG.md`、`Cargo.lock`、根与五个清单、新文档 `docs/merge-batch3-publish.md`）；**0 个 tag** 含该提交；仓库内 **0 个 `.crate`** ✓ |
| **包内容半段** | `sh tools/nichlink-package-audit`（副本内） | **exit 1**，`contents failed: nichlink-toolchain` ✗（与 F-13-1 同一根因，此提交未修） |

**F-14-1（MAJOR）批 3 的“发布前门禁”不能算全绿**：`tools/nichlink-package-audit` 的内容半段在
**HEAD 也仍是 exit 1**（我在工作树复跑，逐条输出与 §4 相同）。`docs/merge-batch3-publish.md` 与落地
记录把该工具当作发布前检查之一 ⇒ 真正 `cargo publish` 之前必须先关掉 F-13-1，否则会发出一个
`src/**` 内容不完整的包（这对 `nichlink-toolchain` 是不可回滚动作）。
`requiredFix`：先收口 F-13-1，再把 `sh tools/nichlink-package-audit`（exit 0）写进批 3 的前置清单并
留原始输出。

**F-14-2（LOW）“0.1.6 残留 0”的口径**：落地记录 §判别性证据写“全仓 `version = "0.1.6"` 残留 0”；
实测 **2 处**，都在**测试夹具字符串**里（`conventions/src/release_version_tests.rs:123`、
`conventions/src/features_tests.rs:16`），不是清单版本行 ⇒ 结论方向没错，但判据要写明“清单版本
行 0 残留；夹具字符串不计入”。
`requiredFix`：把该条判据写成“`grep -rn '^version = "0\.1\.6"' --include=Cargo.toml` = 0”。

## 6. 对 t137 部分完成的复核（我独立复算）

- 它的核心结论（四个提交里 `examples/**` 与 `studio/tests/fixtures/**` **没有被移动/改名**）**只对
  `examples/` 成立**：我实测 `examples/` 在四个提交里命中 16/0/16/2，状态**全为 M**，`R/D` =
  **0** ✓；但 `13c0b13` 在 `studio/tests/fixtures/` 一侧有 **9 条 D**（+ 目的地
  `toolchain/src/studio/tests/fixtures/` 9 条 A）——那是 fixture 包整体搬迁，**落地记录 §身份结论已
  把它声明为“知情接受的偏差”**（该 fixture 无落盘身份物、不硬编码 `NodeId`）⇒ 记录本身诚实，
  是 t137 的表述少了一句。
- 它的验证命令结论（`git status --porcelain | grep -v <报告> | wc -l` = 0）在**共享工作树**上
  今天不成立：本单开工时该命令给 **6**（`M AGENTS.md`、`M conventions/src/lib.rs`、
  `M docs/b3-registration-diagnosis.md`、`?? conventions/src/verb_table.rs`、
  `?? conventions/src/verb_table_tests.rs`、`?? docs/audit-2026-09-28/audit-merge-landing.md`）——
  都是**他人/前序作业的在编文件**，不是本单写入。按 t18 的教训，这类判据要写成**内容级**：
  我对本单的证明是“写入前后只有我这一个文件的增量”（§7）。

## 7. 只读声明与本次验证

- **唯一写入**：本文件。写入前后 `git status --porcelain` 从 6 条变 7 条，**新增的那一条就是本文件**
  （`?? docs/audit-2026-09-28/audit-review-merge.md`）；其余 6 条在写入前后逐字相同。
- 未改任何源码/清单/文档；未 `git commit`；未做任何 `git checkout/restore/stash`；四个提交的副本
  用 `git archive`（只读）取得，变异只在 `/tmp/t139/mut` 里做。
- 交付时跑的命令与结果：`test -f docs/audit-2026-09-28/audit-review-merge.md` ⇒ 0；
  `git status --porcelain | grep -v 'docs/audit-2026-09-28/audit-review-merge.md' | wc -l` ⇒ **6**
  （上面那 6 条**他人的**在编条目，非本单产物）。

## 8. 未覆盖 / 边界（如实）

1. 只复核了四个提交的**上述探针面**；未做 Windows/大小写不敏感文件系统面的复核，未跑
   `cargo test --all-features --doc`（CI 才跑），未跑 `tools/nichlink-external-rehearsal`
   （需要把示例复制到检出外，成本高且与本次四个提交的判据无直接关系）。
2. 批 3 的**实际发布/yank 不可复核**（本机无凭据、且本单只读）⇒ §5 只判“准备面”，不判发布动作。
3. `13c0b13` 的 `--check-table` 我在**副本**里跑通；本单未在共享工作树上跑它（避免与在编改动互相
   干扰），HEAD 面的 `tools/nichlink-package-audit` 红我在工作树上复跑过（exit 1）。
4. 结论里的“跳过的包”是预期行为：`nichlink-macro` / `nichlink-toolchain` 的**打包**半段会因
   版本依赖不在 index 上而跳过（离线环境同样如此），**内容**半段对全部 3 个包都生效——
   F-13-1 正是被内容半段抓到的。
