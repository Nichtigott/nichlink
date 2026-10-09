# 批 3 发布与 yank 命令清单（可直接执行；**本文件只准备，不在准备阶段执行** ✗）

维护者已选定 **0.2.0**，并授权队长在拿到 token 后**在本机逐条执行**。
版本线已在同一次改动里升到 `0.2.0`（根 `[workspace.package] version` + `kernel`/`macro`/`toolchain`
三处的内部 `xirang-*` 版本要求 + `Cargo.lock`）。

## 0. 前置检查（不写 index）

```sh
cargo fmt --all -- --check
cargo test --workspace --offline
cargo test --workspace --offline --all-features
cargo test -p xirang-conventions --offline        # 含 release_version：内部 xirang-* 要求须等于工作区版本
tools/xirang-publish --check-table                # 两张表与清单一致（3 个已发布包）
```

发布需要：`CARGO_REGISTRY_TOKEN`（或在 `cargo login` 后由 `~/.cargo/credentials.toml` 提供）、
网络、以及 `cargo` 能访问 index。**离线跑不动第 1 步之后的任何一条。**

## 1. 发布顺序（依赖序：kernel → macro → toolchain）

逐条执行，**每条都要等上一条在 index 上可解析**再走下一步：

```sh
cargo publish -p xirang-kernel   --locked
cargo publish -p xirang-macro    --locked
cargo publish -p xirang-toolchain --locked
```

说明：`xirang-toolchain` 的 5 个 bin（`xirang`、`cargo-xirang`、`xirang-mcp`、
`xirang-studio`、`xirang-dev`）随该包一起发布；`xirang-dev` 带
`required-features = ["dev-supervisor"]`。`conventions` 与两个示例宿主是 `publish = false` ✓。

## 2. 发布后验证（需要 index）

```sh
tools/xirang-publish --verify-consumers
```

**只有一个开关**：`--verify-consumers` 会在**已发布的 crate** 上重建消费者（对应
`tools/xirang-external-rehearsal` 的检出外那一半）。该工具的开关全集是
`--help / --publish / --yes / --allow-dirty / --check-table / --verify-consumers`
——**没有 `--verify-publish`** ✗（本清单早先按方案写的那一条是错的，2026-09-27 执行时实测：
`error: unknown argument '--verify-publish'`，exit 2）。
**在它绿之前不要 yank。**

（另记：真正"按依赖序批量发布"的入口是 `tools/xirang-publish --publish --yes` ✓——它会在
每层等 index 可解析 ✓；本次是一次性迁移的第一版发布，所以第 1 节用等价的逐条 `cargo publish`
加自建等待跑完的 ✓。）

## 3. 旧名 yank 清单（八名；`xirang-macro` 保留 ✓）

**index 实测（2026-09-29）**：八个旧名的已发布版本各为 **`0.1.0`、`0.1.1`、`0.1.3`、`0.1.4`、`0.1.5`**
（没有 `0.1.2`；**最新是 `0.1.5`，不是 `0.1.6`** —— 0.1.6 从未发布）。`xirang-kernel` 与
`xirang-toolchain` 在 index 上**尚不存在**（本次是首次发布，无旧版本可 yank）；`xirang-macro`
已发布到 `0.1.5`，**保留不 yank**。

**要真正"退役"一个旧名，必须把它的每个已发布版本都 yank**：只 yank 最新版会让
`xirang-core = "0.1"` 这类新解析退回到未 yank 的旧版本（yank 只影响**新解析**，
已有 lockfile 仍可解析被 yank 的版本 ✓）。

| 旧名 | 要 yank 的版本 | 命令 |
| --- | --- | --- |
| `xirang-core` | 0.1.0 / 0.1.1 / 0.1.3 / 0.1.4 / 0.1.5 | `for v in 0.1.0 0.1.1 0.1.3 0.1.4 0.1.5; do cargo yank --version "$v" xirang-core; done` |
| `xirang-run-method` | 同上五行 | 同上，把名字换掉 |
| `xirang-build-method` | 同上 | 同上 |
| `xirang-debug-method` | 同上 | 同上 |
| `xirang-plugin-host` | 同上 | 同上 |
| `xirang-studio` | 同上 | 同上 |
| `xirang-mcp` | 同上 | 同上 |
| `xirang-cli` | 同上 | 同上 |
| `xirang-macro` | — | **保留，不 yank** ✓ |

批量形式（**逐条执行**，别合并成一条以免中间失败无从定位；执行前先按上表逐名复核 index）：

```sh
for n in xirang-core xirang-run-method xirang-build-method xirang-debug-method \
         xirang-plugin-host xirang-studio xirang-mcp xirang-cli; do
  for v in 0.1.0 0.1.1 0.1.3 0.1.4 0.1.5; do
    cargo yank --version "$v" "$n" || echo "yank 失败: $n $v"
  done
done
```

yank **只影响解析**（历史版本仍在 index 上，已锁定的 lockfile 不受影响）✓，而且 **yank 是可逆的** ✓
——`cargo yank --version <v> <name> --undo` 可撤销（Cargo Book 同一段就写着这个用法）。**真正不可逆
的是"发布"本身**：已发布的版本**永不可覆盖、代码不可删除**（crates.io 是永久归档）✗ ⇒ 想"删掉"某个
已发布版本，唯一自助手段就是 yank，删除需要联系 crates.io 支持且只适用于极窄情形 ✓。
⇒ 第 2 步（`tools/xirang-publish --verify-consumers`）未绿之前不要执行本步。

## 3.5 事后发展：旧名被**整体删除**了，随之而来的是约 24 小时的冷却窗口（2026-09-29 实测）

维护者在 2026-09-29 用 crates.io 的删除入口把**八个旧名（连同当时的三个新名，共 11 个名字）全部删除**了
✓ —— 也就是说 §3 的 yank 表是**实际做过、随后被更强的动作取代**的历史记录（先 yank 全部版本，再把
crate 整体删掉）✓。**删除是可能的**（crates.io 允许所有者删除，条件见下），但**名字不能立刻再用**：

```
error: failed to publish xirang-kernel v0.2.0 to registry at https://crates.io
Caused by:
  the remote server responded with an error (status 400 Bad Request):
  A crate with the name `xirang-kernel` was recently deleted.
  Reuse of this name will be available after 2026-09-30T07:41:46Z.
```

- **冷却窗口按名字各自计时**（约 24 小时）⇒ 上面 `xirang-kernel` 的解封时间是 **2026-09-30T07:41:46Z**；
  另两个名字在同一分钟内被删，窗口与之接近。**以发布尝试为准**：窗口未到时 `cargo publish` 会用上面
  这种 400 直接告诉你还要等多久 ✓。
- **因此重新发布必须等到窗口之后** ✓；在此之前**不要再做任何删除动作**（会把窗口往后推 ✓）。
- **窗口开着的时候正好把尾巴收完**：`docs/audit-2026-09-28/audit-merge-landing.md` 的"仍开放"清单里
  剩 ④/⑦b/⑦c/⑦d ✓；收完后三个 crate 一起作为**首次发布**发出（`0.2.0`，含全部修复 ✓），CHANGELOG 的
  `## [0.2.0]` 日期改成**真正的发布日** ✓。
- **删除的现实条件（顺带记下，供下次判断）**：crates.io 允许所有者删除 crate 的情形是"发布不到
  **72 小时**"，或（**单一 owner** + **无反向依赖** + **每月下载 < 100**）⇒ 一串互相依赖的 crate 可以
  **从叶子往根**逐个删掉 ✓；**"删除某个已发布版本"仍然不存在**（版本永远不可删、不可覆盖 ✗）；
  被删掉的名字**约 24 小时内不可复用** ✓。

## 4. 收尾

1. `CHANGELOG.md`：把 `## [0.2.0] — unreleased` 的头改成带日期的 `## [0.2.0] — YYYY-MM-DD`（发布当天）。
2. 旧 8 名 yank 之后，若 `tools/xirang-publish` 的两张表仍写 3 个包 ⇒ `--check-table` 仍应绿 ✓。
3. **已知遗留（不阻塞发布，但要在下次发布说明里复述）**：批 2 的 `(b)` 六个模块内测试文件
   （复核 t139 现算 **20 个** `#[test]`，非 25）尚未接回 ✗ —— 诊断见 `docs/b3-registration-diagnosis.md`。
4. **发布前另加两条显式检查**（来自复核 t139 的 findings，避免"绿了但没人看见"）：
   - `tools/xirang-package-audit` 的**内容半段必须绿**：它逐 crate 核对 `cargo package --list` 是否覆盖
     `src/**/*.rs` 与清单声明的 README；批 2 曾因 fixture 包住在 `src/` 下而红 ✗（收口见
     `docs/audit-2026-09-28/audit-merge-landing.md` 的身份结论节）。
   - 上面那 20 个未接回的 `#[test]` 属**已声明的覆盖降级** ⇒ 发布说明里必须复述，并在下个发布前决定
     是否按 `docs/b3-registration-diagnosis.md` 的 F1/F2 收口。
