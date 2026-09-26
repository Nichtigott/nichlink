//! A package whose source tree is missing is diagnosed, not fatal.
//! 源树缺失的包会得到诊断，而不是致命失败。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// A throwaway package directory holding one manifest.
/// 一个只含一份清单的一次性包目录。
fn package(manifest: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-build-missing-src-{}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("package directory");
    std::fs::write(root.join("Cargo.toml"), manifest).expect("manifest");
    root
}

/// A package with no `src/` is refused with a layout diagnostic. It used to reach
/// `expect("src directory must exist")` and take the build script down, which left
/// `check --json` with an empty stdout — the contract that command keeps.
/// 没有 `src/` 的包以一条布局诊断被拒绝。它过去会走到
/// `expect("src directory must exist")` 并打死构建脚本，让 `check --json` 的 stdout 一片空白
/// ——而那正是那条命令要守住的契约。
#[test]
fn a_package_without_a_source_tree_is_reported_as_a_layout_problem() {
    let root = package("[package]\nname = \"probe\"\nversion = \"0.1.0\"\n");
    let out_dir = root.join("target/nichlink/out");
    let diagnostics = nichlink_build_method::check_for(&root, &out_dir, "probe")
        .expect_err("a package without a source tree must be refused");
    let rendered = diagnostics.render();
    assert!(rendered.contains("face-layout"), "{rendered}");
    assert!(
        rendered.contains(&root.join("src").display().to_string()),
        "the refusal must name the tree it looked for: {rendered}"
    );
    // The machine-readable twin carries the same phase, so `check --json` can act
    // on it instead of counting an anonymous failure.
    // 机器可读的孪生携带同一个阶段，因此 `check --json` 可以据它行事，而不是数一条无名失败。
    let json = diagnostics.to_json();
    assert!(json.contains("\"phase\":\"face-layout\""), "{json}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The same call on a package that has a source tree but no faces is *not* a
/// layout problem: an empty tree is a legitimate host under construction, and the
/// two cases must be distinguishable.
/// 对"有源树但没有注册面"的包做同一次调用**不是**布局问题：空树是施工中的合法宿主，两种情形
/// 必须可区分。
#[test]
fn an_empty_source_tree_is_not_a_layout_problem() {
    let root = package("[package]\nname = \"probe\"\nversion = \"0.1.0\"\n");
    std::fs::create_dir_all(root.join("src")).expect("source tree");
    std::fs::write(root.join("src/lib.rs"), "\n").expect("library root");
    let out_dir = root.join("target/nichlink/out");
    if let Err(diagnostics) = nichlink_build_method::check_for(&root, &out_dir, "probe") {
        let rendered = diagnostics.render();
        assert!(
            !rendered.contains("face-layout"),
            "an empty tree is not a missing one: {rendered}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}
