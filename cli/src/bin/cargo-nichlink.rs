//! Cargo plugin entry: `cargo nichlink <args>` invokes this binary with the
//! subcommand name as the first argument, which dispatch must skip.
//! Cargo 插件入口：`cargo nichlink <args>` 会以子命令名作为第一个参数调用
//! 本二进制，分发时需要跳过它。

fn main() {
    let mut args = match nichlink_cli::argv_strings(std::env::args_os()) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("nichlink: {error}");
            std::process::exit(1);
        }
    };
    // `cargo nichlink <args>` passes the subcommand name at index 1; dispatch skips it.
    // `cargo nichlink <args>` 把子命令名放在下标 1；分发会跳过它。
    if args.len() > 1 {
        args.remove(1);
    }
    if let Err(error) = nichlink_cli::run(args) {
        eprintln!("nichlink: {error}");
        std::process::exit(1);
    }
}
