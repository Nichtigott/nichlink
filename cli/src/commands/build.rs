//! `nichlink build`: validate the registration tree, then run `cargo build`.
//! `nichlink build`：先校验注册树，再运行 `cargo build`。
//!
//! Split out of `lib.rs`: the command is a thin process wrapper around
//! `registration_check`, and the argv split (leading path versus passed-through
//! cargo options) is the only rule it owns.
//! 从 `lib.rs` 拆出：本命令是 `registration_check` 之上的薄进程包装，唯一拥有的规则
//! 是 argv 拆分（首个路径与透传的 cargo 选项）。

use super::{registration_check, split_build_args};

/// Validate the registration tree first; only then invoke `cargo build`
/// with the remaining arguments passed through verbatim. The path, when
/// given, must come before any cargo option.
/// 先校验注册树；通过后再调用 `cargo build`，其余参数原样透传。
/// 路径如给出，必须位于任何 cargo 选项之前。
pub(crate) fn build(args: &mut impl Iterator<Item = String>) -> Result<(), String> {
    let all: Vec<String> = args.by_ref().collect();
    let (directory, cargo_args) = split_build_args(&all);
    let directory = directory.unwrap_or_else(|| ".".to_owned());
    let package = registration_check(&directory)?;
    println!("nichlink build: registration ok ({package})");
    let manifest = std::fs::canonicalize(&directory)
        .map_err(|error| format!("cannot resolve {directory}: {error}"))?;
    let status = std::process::Command::new("cargo")
        .arg("build")
        .args(&cargo_args)
        .current_dir(&manifest)
        .status()
        .map_err(|error| format!("cannot start cargo build: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo build failed ({status})"))
    }
}
