//! The `nichlink` entry binary: process glue for the command-line interface.
//! `nichlink` 入口二进制：命令行界面的进程粘合层。
//!
//! `main` forwards straight to `nichlink_cli::main`, which dispatches the argv
//! NichLink was invoked with. A failure is printed to stderr and exits non-zero,
//! so a shell pipeline sees the command fail.
//! `main` 直接转发给 `nichlink_cli::main`，由它分发 NichLink 被调用时的 argv。
//! 失败时打印到 stderr 并以非零码退出，因此 shell 管线能看见命令失败。

fn main() {
    if let Err(error) = nichlink_cli::main() {
        eprintln!("nichlink: {error}");
        std::process::exit(1);
    }
}
