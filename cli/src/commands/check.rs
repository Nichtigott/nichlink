//! `nichlink check`: run the registration discovery and validation pass.
//! `nichlink check`：运行注册发现与校验。
//!
//! Split out of `lib.rs`: this command owns the `--json` document contract and
//! the human line, while the shared `resolve_package`/`build_out_dir` helpers it
//! calls stay in the dispatching parent so `check`, `explain` and `grafts` all
//! read the same package root and output directory.
//! 从 `lib.rs` 拆出：本命令拥有 `--json` 文档契约与人类可读行，而它调用的共用
//! `resolve_package`/`build_out_dir` 辅助函数留在分发父模块，使 `check`、`explain`
//! 与 `grafts` 读取同一个包根与输出目录。

use std::io::Write;

use super::{build_out_dir, resolve_package};

/// Run the validation pass, optionally as one JSON document for CI.
/// 运行校验，可选地以单个 JSON 文档输出给 CI。
///
/// `--json` keeps stdout reserved for exactly one document: success is the
/// empty diagnostics document, failure is the same document with every
/// diagnostic, and the non-zero exit still comes from the returned `Err`. A
/// human run stays byte-identical to the historical output.
/// `--json` 让 stdout 只保留一个文档：成功就是空的诊断文档，失败是带全部诊断的同一
/// 文档，而非零退出仍由返回的 `Err` 给出。人类可读运行与历史输出逐字节一致。
pub(crate) fn check(
    args: &mut impl Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut json_output = false;
    let mut directory: Option<String> = None;
    for arg in args.by_ref() {
        match arg.as_str() {
            "--json" => json_output = true,
            _ if arg.starts_with('-') => return Err(format!("unexpected argument '{arg}'")),
            _ if directory.is_none() => directory = Some(arg),
            _ => return Err("check accepts at most one path".to_owned()),
        }
    }
    let directory = directory.unwrap_or_else(|| ".".to_owned());
    let (manifest, package) = resolve_package(&directory)?;
    let out_dir = build_out_dir(&manifest);
    if json_output {
        return match nichlink_build_method::check_for(&manifest, &out_dir, &package) {
            Ok(()) => {
                writeln!(out, "{}", nichlink::BuildDiagnostics::default().to_json())
                    .map_err(|error| format!("cannot write output: {error}"))?;
                Ok(())
            }
            Err(diagnostics) => {
                writeln!(out, "{}", diagnostics.to_json())
                    .map_err(|error| format!("cannot write output: {error}"))?;
                Err(format!(
                    "registration check failed ({} diagnostic(s); JSON on stdout)",
                    diagnostics.len()
                ))
            }
        };
    }
    nichlink_build_method::run_for(&manifest, &out_dir, &package)?;
    writeln!(out, "nichlink check: ok ({package})")
        .map_err(|error| format!("cannot write output: {error}"))?;
    Ok(())
}
