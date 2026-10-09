//! XiRang Studio executable entry.
//! XiRang Studio 可执行入口。

use std::io;
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("xirang-toolchain: {error}");
        std::process::exit(1);
    }
}

/// Parse the command line and hand it to the library.
/// 解析命令行，并交给库。
fn run() -> io::Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    let project = match (arguments.next(), arguments.next()) {
        (None, _) => None,
        (Some(flag), None) if flag == "-h" || flag == "--help" => {
            println!("{}", usage());
            return Ok(());
        }
        (Some(path), None) => Some(PathBuf::from(path)),
        (Some(_), Some(extra)) => {
            return Err(io::Error::other(format!(
                "expected at most one project path, got an extra argument `{}`\n\n{}",
                extra.to_string_lossy(),
                usage()
            )));
        }
    };
    xirang_toolchain::studio::launch_with(project)
}

/// What the executable accepts.
/// 该可执行文件接受什么。
fn usage() -> String {
    "xirang-toolchain [PROJECT]
\n  PROJECT  a XiRang host project directory (defaults to XIRANG_PACKAGE_ROOT,
\n           then to the working directory when it holds a Cargo.toml)"
        .to_owned()
}
