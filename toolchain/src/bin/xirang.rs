//! The `xirang` entry binary: process glue for the command-line interface.
//! `xirang` 入口二进制：命令行界面的进程粘合层。
//!
//! `main` forwards straight to `xirang_toolchain::cli::main`, which dispatches the argv
//! XiRang was invoked with. A failure is printed to stderr and exits non-zero,
//! so a shell pipeline sees the command fail.
//! `main` 直接转发给 `xirang_toolchain::cli::main`，由它分发 XiRang 被调用时的 argv。
//! 失败时打印到 stderr 并以非零码退出，因此 shell 管线能看见命令失败。

fn main() {
    if let Err(error) = xirang_toolchain::cli::main_entry() {
        eprintln!("xirang: {error}");
        std::process::exit(1);
    }
}
