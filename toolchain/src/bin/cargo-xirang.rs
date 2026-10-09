//! Cargo plugin entry: `cargo xirang <args>` invokes this binary with the
//! subcommand name as the first argument, which dispatch must skip.
//! Cargo 插件入口：`cargo xirang <args>` 会以子命令名作为第一个参数调用
//! 本二进制，分发时需要跳过它。

fn main() {
    let mut args = match xirang_toolchain::cli::argv_strings(std::env::args_os()) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("xirang: {error}");
            std::process::exit(1);
        }
    };
    // `cargo xirang <args>` passes the subcommand name at index 1; dispatch skips it.
    // `cargo xirang <args>` 把子命令名放在下标 1；分发会跳过它。
    if args.len() > 1 {
        args.remove(1);
    }
    if let Err(error) = xirang_toolchain::cli::run(args) {
        eprintln!("xirang: {error}");
        std::process::exit(1);
    }
}
