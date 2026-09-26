//! Re-run the kernel's validation and report the tree delta it just published.
//! 重新运行内核的校验，并报告它刚刚发布的树差异。
//!
//! The convergence loop's last rung: `nichlink.apply` writes a face, and this says
//! whether the kernel still accepts the tree afterwards. It drives the *same* entry
//! the CLI's `check` drives (`check_for`), so a verdict here and a verdict there
//! cannot drift, and it publishes the build evidence as a side effect — which is what
//! makes the delta below describe the tree that was just verified rather than the one
//! from the last build. A failed verdict is the answer, not a tool failure: the reply
//! says `verdict failed` and carries the diagnostics, while the reply's `isError`
//! stays false because the verification itself succeeded.
//! 收敛闭环的最后一级：`nichlink.apply` 写下一个面，而这里回答内核之后是否仍然接受这棵树。它驱动
//! CLI 的 `check` 所驱动的**同一个入口**（`check_for`），因此这里的判断与那里的判断不可能漂移；它还
//! 顺带发布构建证据——这正是下面那份差异描述的是"刚刚被校验的那棵树"、而不是"上次构建的那棵树"的原因。
//! 判断失败是答案而不是工具故障：回复写出 `verdict failed` 并带上诊断，而回复的 `isError` 仍为 false，
//! 因为校验本身成功了。

use std::path::Path;

use serde_json::Value;

use crate::evidence::out_dir;

/// The most diagnostic lines one reply carries before it says it truncated.
/// 一条回复在声明被截断之前最多携带的诊断行数。
const MAX_DIAGNOSTIC_LINES: usize = 200;

/// Validate the package at `root`, then report the delta the run published.
/// 校验 `root` 处的包，然后报告那次运行发布的差异。
pub(crate) fn verify(root: &Path, arguments: &Value) -> Result<String, String> {
    let manifest = root.join("Cargo.toml");
    if !manifest.is_file() {
        return Err(format!(
            "verify needs a Cargo package: {} does not exist",
            manifest.display()
        ));
    }
    // Cargo names the package, and `check_for` stamps the process-wide build
    // namespace with it — the same value the CLI passes, so a verdict here and a
    // verdict there describe the same identity domain.
    // 包名由 Cargo 给出，而 `check_for` 会用它盖下进程级的构建命名空间——与 CLI 传入的是同一个值，
    // 因此这里的判断与那里的判断描述同一个身份域。
    let package = nichlink_build_method::package_name(&manifest)?;
    let out = out_dir(root);
    // `check_for` takes the package *directory* while `package_name` takes its manifest
    // *file*: pass the file here and the pipeline looks for `<Cargo.toml>/src` and reports
    // "is not a source directory". The CLI's local is named `manifest` and holds the
    // directory, which is how the confusion survives review.
    // `check_for` 收包**目录**，而 `package_name` 收它的清单**文件**：这里传文件，管线就会去找
    // `<Cargo.toml>/src` 并报 "is not a source directory"。CLI 的局部变量名叫 `manifest` 却装着
    // 目录——混淆就是这样通过审阅的。
    let verdict = match nichlink_build_method::check_for(root, &out, &package) {
        Ok(()) => "verdict ok (the kernel accepted the tree)\n".to_owned(),
        Err(diagnostics) => format!(
            "verdict failed ({} diagnostic(s))\n{}",
            diagnostics.len(),
            bounded(&diagnostics.render())
        ),
    };
    // The delta is the same report `nichlink.diff` gives, and it is meaningful here
    // precisely because the call above just refreshed the build's side of it.
    // 这份差异就是 `nichlink.diff` 给出的同一份报告，而它在这里有意义，正是因为上面那次调用刚刚刷新
    // 了它在构建一侧的数据。
    let delta = crate::diff::diff(root, arguments)?;
    Ok(format!("{verdict}\n{delta}"))
}

/// Keep one reply inside a size an agent can read, and say when it did not.
/// 把一条回复限制在代理读得下的规模里，并在截断时说出来。
fn bounded(text: &str) -> String {
    let lines = text.lines().count();
    if lines <= MAX_DIAGNOSTIC_LINES {
        return text.to_owned();
    }
    let head = text
        .lines()
        .take(MAX_DIAGNOSTIC_LINES)
        .collect::<Vec<_>>()
        .join("\n");
    format!("{head}\n… truncated: {lines} diagnostic lines total, {MAX_DIAGNOSTIC_LINES} shown.\n")
}

#[cfg(test)]
#[path = "verify_tests.rs"]
mod verify_tests;
