# 批 3 发布与 yank 命令清单（可直接执行；**本文件只准备，不在准备阶段执行** ✗）

维护者已选定 **0.2.0**，并授权队长在拿到 token 后**在本机逐条执行**。
版本线已在同一次改动里升到 `0.2.0`（根 `[workspace.package] version` + `kernel`/`macro`/`toolchain`
三处的内部 `nichlink-*` 版本要求 + `Cargo.lock`）。

## 0. 前置检查（不写 index）

```sh
cargo fmt --all -- --check
cargo test --workspace --offline
cargo test --workspace --offline --all-features
cargo test -p nichlink-conventions --offline        # 含 release_version：内部 nichlink-* 要求须等于工作区版本
tools/nichlink-publish --check-table                # 两张表与清单一致（3 个已发布包）
```

发布需要：`CARGO_REGISTRY_TOKEN`（或在 `cargo login` 后由 `~/.cargo/credentials.toml` 提供）、
网络、以及 `cargo` 能访问 index。**离线跑不动第 1 步之后的任何一条。**

## 1. 发布顺序（依赖序：kernel → macro → toolchain）

逐条执行，**每条都要等上一条在 index 上可解析**再走下一步：

```sh
cargo publish -p nichlink-kernel   --locked
cargo publish -p nichlink-macro    --locked
cargo publish -p nichlink-toolchain --locked
```

说明：`nichlink-toolchain` 的 5 个 bin（`nichlink`、`cargo-nichlink`、`nichlink-mcp`、
`nichlink-studio`、`nichlink-dev`）随该包一起发布；`nichlink-dev` 带
`required-features = ["dev-supervisor"]`。`conventions` 与两个示例宿主是 `publish = false` ✓。

## 2. 发布后验证（需要 index）

```sh
tools/nichlink-publish --verify-publish --verify-consumers
```

两个开关都要绿：前者核对 index 上的版本，后者在已发布 crate 上重建消费者。
**在它绿之前不要 yank。**

## 3. 旧名 yank 清单（八名；`nichlink-macro` 保留 ✓）

yank 的版本号 = 该名字**最后发布的版本**。按仓库记录（`CHANGELOG.md` 的发布状态段）当前
发布线是 **`0.1.6`**；执行前请用 index 复核（`cargo info <name>` 或 index API），若与实际不同，
把下面的 `<v>` 换成实际版本：

| 旧名 | 最后发布版本（复核项） | 命令 |
| --- | --- | --- |
| `nichlink-core` | `0.1.6` | `cargo yank --version 0.1.6 nichlink-core` |
| `nichlink-run-method` | `0.1.6` | `cargo yank --version 0.1.6 nichlink-run-method` |
| `nichlink-build-method` | `0.1.6` | `cargo yank --version 0.1.6 nichlink-build-method` |
| `nichlink-debug-method` | `0.1.6` | `cargo yank --version 0.1.6 nichlink-debug-method` |
| `nichlink-plugin-host` | `0.1.6` | `cargo yank --version 0.1.6 nichlink-plugin-host` |
| `nichlink-studio` | `0.1.6` | `cargo yank --version 0.1.6 nichlink-studio` |
| `nichlink-mcp` | `0.1.6` | `cargo yank --version 0.1.6 nichlink-mcp` |
| `nichlink-cli` | `0.1.6` | `cargo yank --version 0.1.6 nichlink-cli` |
| `nichlink-macro` | — | **保留，不 yank** ✓ |

批量形式（逐条执行，别合并成一条以免中间失败无从定位）：

```sh
for n in nichlink-core nichlink-run-method nichlink-build-method nichlink-debug-method \
         nichlink-plugin-host nichlink-studio nichlink-mcp nichlink-cli; do
  cargo yank --version 0.1.6 "$n"
done
```

yank **只影响解析**，历史版本仍在 index 上；一旦 yank 不可撤销 ✓ ⇒ 第 2 步未绿之前不要执行本步。

## 4. 收尾

1. `CHANGELOG.md`：把 `## [0.2.0] — unreleased` 的头改成带日期的 `## [0.2.0] — YYYY-MM-DD`（发布当天）。
2. 旧 8 名 yank 之后，若 `tools/nichlink-publish` 的两张表仍写 3 个包 ⇒ `--check-table` 仍应绿 ✓。
3. **已知遗留（不阻塞发布，但要在下次发布说明里复述）**：批 2 的 `(b)` 六个模块内测试文件
   （复核 t139 现算 **20 个** `#[test]`，非 25）尚未接回 ✗ —— 诊断见 `docs/b3-registration-diagnosis.md`。
4. **发布前另加两条显式检查**（来自复核 t139 的 findings，避免"绿了但没人看见"）：
   - `tools/nichlink-package-audit` 的**内容半段必须绿**：它逐 crate 核对 `cargo package --list` 是否覆盖
     `src/**/*.rs` 与清单声明的 README；批 2 曾因 fixture 包住在 `src/` 下而红 ✗（收口见
     `docs/audit-2026-09-28/audit-merge-landing.md` 的身份结论节）。
   - 上面那 20 个未接回的 `#[test]` 属**已声明的覆盖降级** ⇒ 发布说明里必须复述，并在下个发布前决定
     是否按 `docs/b3-registration-diagnosis.md` 的 F1/F2 收口。
