//! `nichlink studio`: launch the Studio TUI for a named project.
//! `nichlink studio`：为指定项目启动 Studio TUI。
//!
//! Split out of `lib.rs` like the other subcommands: the argv shape is this
//! command's own, while the TUI and its project resolution live in
//! `nichlink-studio`. The subcommand used to drop its arguments, so
//! `nichlink studio <path>` opened the working directory even though Studio's
//! own error text tells the reader to pass a path; the path now reaches
//! `launch_with`, which resolves it exactly like `nichlink-studio <path>` does.
//! 与其他子命令一样从 `lib.rs` 拆出：参数形状归本命令，TUI 与其项目解析归属
//! `nichlink-studio`。该子命令此前丢弃自己的参数，因此即便 Studio 自己的错误文本让读者
//! 传一个路径，`nichlink studio <path>` 打开的仍是当前目录；现在该路径会到达
//! `launch_with`，其解析方式与 `nichlink-studio <path>` 完全一致。

use std::io::Write;
use std::path::PathBuf;

use super::USAGE;

/// Launch Studio for the project named on the command line, if any.
/// 为命令行指定的项目启动 Studio；未指定则走默认解析。
///
/// `--help` prints the same usage the other subcommands read, so the documented
/// argument and the dispatch cannot disagree.
/// `--help` 打印与其他子命令相同的用法，因此文档化的参数与分发不会产生分歧。
pub(crate) fn studio(
    args: &mut impl Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut project: Option<PathBuf> = None;
    for arg in args.by_ref() {
        match arg.as_str() {
            "-h" | "--help" => {
                write!(out, "{USAGE}").map_err(|error| format!("cannot write usage: {error}"))?;
                return Ok(());
            }
            _ if arg.starts_with('-') => return Err(format!("unexpected argument '{arg}'")),
            _ if project.is_none() => project = Some(PathBuf::from(arg)),
            _ => return Err("studio accepts at most one project path".to_owned()),
        }
    }
    nichlink_studio::launch_with(project).map_err(|error| error.to_string())
}
