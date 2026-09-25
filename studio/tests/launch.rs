//! What the Studio executable does before it takes over a terminal.
//! Studio 可执行文件在接管终端之前会做什么。
//!
//! These run the real binary, because the property under test is the process's
//! exit status and its stderr — the two things a script and a reader actually see.
//! A launch with no project used to show an empty user interface and exit 0,
//! which looked healthy and left the next authoring command to write into
//! whatever directory the process happened to be in.
//! 这些测试运行真实二进制，因为被测性质是进程的退出状态与它的 stderr——脚本与读者真正看到
//! 的两样东西。没有项目的启动过去会显示空界面并以 0 退出，那看上去是健康的，却把随后的创作
//! 命令留给进程恰好所在的那个目录。

use std::process::Command;

/// The binary under test, as Cargo builds it for this integration test.
/// 被测二进制，由 Cargo 为本集成测试构建。
fn studio() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nichlink-studio"))
}

/// A path argument that is not a project is refused, and the message names it.
/// 不是项目的路径参数被拒绝，且消息点出它。
#[test]
fn a_path_that_is_not_a_project_fails_with_a_message() {
    let output = studio()
        .arg("/nonexistent-project-for-studio-tests")
        .output()
        .expect("run the Studio binary");
    assert_eq!(
        output.status.code(),
        Some(1),
        "a missing project must not exit zero: {output:?}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not a directory"), "{stderr}");
    assert!(
        stderr.contains("nonexistent-project-for-studio-tests"),
        "the message must name the path: {stderr}"
    );
}

/// A configured root that is not a directory is refused by variable name.
/// 被配置却不是目录的根按变量名被拒绝。
#[test]
fn a_configured_root_that_is_not_a_directory_fails_with_the_variable_named() {
    let output = studio()
        .env(
            "NICH_LINK_PACKAGE_ROOT",
            "/nonexistent-root-for-studio-tests",
        )
        .output()
        .expect("run the Studio binary");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("NICH_LINK_PACKAGE_ROOT"),
        "the message must name the variable: {stderr}"
    );
}

/// `--help` explains the argument and succeeds without touching the terminal.
/// `--help` 解释该参数，并且不触碰终端就成功返回。
#[test]
fn help_prints_usage_and_succeeds() {
    let output = studio()
        .arg("--help")
        .output()
        .expect("run the Studio binary");
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("nichlink-studio [PROJECT]"), "{stdout}");
    assert!(stdout.contains("NICH_LINK_PACKAGE_ROOT"), "{stdout}");
}
