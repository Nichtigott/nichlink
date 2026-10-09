//! End-to-end pins for the one-shot client, run against the real binary.
//! 一次性客户端的端到端钉子，跑在真二进制上。

//! The client exists because the measured failure was structural: the bridge had no command an
//! agent could call, so the agent wrote and debugged its own JSON-RPC client first (blocks 0–10 of
//! 25 in the evaluation's first round, against block 3 for the tool that had a CLI). These pins
//! hold the two properties that make it a *command* rather than a service: it answers without a
//! session, and it survives being read through a pipe.
//! 这个客户端的存在是因为量出来的失败是结构性的：桥没有 agent 能调的命令，于是 agent 先写、再调试自己
//! 的 JSON-RPC 客户端（评测第 1 题 25 块里块 0–10，而有 CLI 的那件工具是块 3）。下面两条钉子守住"它是
//! 命令而不是服务"的两个性质：不需要会话就能作答，以及被管道读时不会崩。

#![cfg(feature = "mcp")]

use std::process::Command;

/// The binary cargo built for this test.
/// 本测试对应的、由 cargo 构建出来的二进制。
fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_xirang-mcp")
}

/// One tool, one command, no session: the answer arrives and the exit code is 0.
/// 一个工具、一条命令、没有会话：答案到达，退出码为 0。
#[test]
fn one_call_is_one_command() {
    let output = Command::new(binary())
        .args(["--call", "xirang.status", "--root", "."])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("the client runs");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("root"), "{text}");
}

/// A reader that closes the pipe first is a reader that has enough, not a failure: `--list | head`
/// must not panic.
/// 先关掉管道的读者是"读够了"的读者，不是失败：`--list | head` 不得崩溃。
///
/// The count is 20 rather than 4 because `--list` **deliberately** prints the instruction page
/// first and the catalogue after it, and that page grows whenever the guidance does. Pinning the
/// catalogue to the first four lines was an assumption about the header's length, not about the pipe
/// — and adding the seven-shape table to the page broke exactly that assumption. Twenty still closes
/// the pipe long before the end (the listing is well past it), so what this test is *for* is
/// unchanged.
/// 取 20 而不是 4：`--list` **有意**先印指引那一页、再印目录，而那一页会随指引增长。把目录钉在"前四行"是对
/// **页眉长度**的假设，不是对管道的假设——而把七场景表加进那一页恰好打破了它。20 仍然远早于结尾就关掉管道
/// （目录长得多），因此这条测试**要测的东西**没有变。
#[test]
fn a_closed_pipe_is_not_a_failure() {
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("{} --list | head -20", binary()))
        .output()
        .expect("the shell runs");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("panicked"),
        "a closed pipe must not panic: {stderr}"
    );
    assert_eq!(output.status.code(), Some(0), "{output:?} {stderr}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("xirang."));
}
