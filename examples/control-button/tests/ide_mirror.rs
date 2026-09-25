//! The IDE mirror must type-check, not only parse.
//! IDE 镜像必须能通过类型检查，而不只是能解析。
//!
//! `rust-analyzer` applies `#[path]` only at the top level of a file or an
//! expansion, so a nested face is loaded a second time as a crate-root shadow.
//! In that shadow `super` is the crate root, while the face's derived rule path
//! (`super::registry_rule::REGISTRATION_RULE`) names a module beside the real
//! face file — so the whole crate failed to type-check under `--cfg
//! rust_analyzer`, which is the cfg an editor uses and `rustc` never does. The
//! commands below are the check that catches it.
//! `rust-analyzer` 只在文件或展开的顶层应用 `#[path]`，因此嵌套面会被第二次载入为 crate
//! 根影子。影子里的 `super` 是 crate 根，而注册面派生的规则路径
//! （`super::registry_rule::REGISTRATION_RULE`）命名的是真实面文件旁边的模块——于是整个
//! crate 在 `--cfg rust_analyzer` 下类型检查失败，而那正是编辑器使用、`rustc` 从不使用的
//! cfg。下面的命令就是抓住这件事的检查。

use std::path::Path;
use std::process::Command;

/// Run one host crate through the IDE cfg in a private target directory.
/// 在一个私有 target 目录里跑一个宿主 crate 的 IDE cfg。
fn check_under_rust_analyzer(package: &str, target: &Path) {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["rustc", "-p", package, "--lib", "--offline"])
        .arg("--target-dir")
        .arg(target)
        .args(["--", "--cfg", "rust_analyzer"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run cargo rustc with the rust_analyzer cfg");
    assert!(
        output.status.success(),
        "`{package}` does not type-check under --cfg rust_analyzer:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Ignored by default: it builds this package and its dependencies into a target
/// directory of its own, which a nested `cargo` needs so it cannot deadlock on
/// the lock the running test already holds. CI runs it explicitly.
/// 默认忽略：它把本包及其依赖构建进一个自己的 target 目录——嵌套 `cargo` 必须如此，否则会
/// 与正在运行的测试持有的锁互等。CI 显式运行它。
#[test]
#[ignore = "runs a separate cargo build; CI runs it explicitly"]
fn the_ide_mirror_type_checks_with_nested_faces() {
    let target = std::env::temp_dir().join("nichlink-ide-mirror-target");
    // The nested-face host is the one that failed; the mirror-only host guards
    // the other direction, where a face declares no registry to own.
    // 嵌套面宿主是失败的那一个；只有镜像的宿主守住另一侧——注册面没有注册机可拥有。
    check_under_rust_analyzer("nichlink-example-control-button", &target);
    check_under_rust_analyzer("nichlink-example-control-button-graft", &target);
}
