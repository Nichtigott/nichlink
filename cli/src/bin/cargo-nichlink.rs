//! Cargo plugin entry: `cargo nichlink <args>` invokes this binary with the
//! subcommand name as the first argument, which dispatch must skip.
//! Cargo 插件入口：`cargo nichlink <args>` 会以子命令名作为第一个参数调用
//! 本二进制，分发时需要跳过它。

fn main() {
    let args = std::env::args().take(1).chain(std::env::args().skip(2));
    if let Err(error) = nichlink_cli::run(args) {
        eprintln!("nichlink: {error}");
        std::process::exit(1);
    }
}
