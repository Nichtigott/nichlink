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
            // Help is not an error, and every subcommand answers it the same way:
            // the usage banner and success. Four of them used to refuse the flag with
            // "unexpected argument" (audit `LGC-LG-44`).
            // 帮助不是错误，而且每个子命令都以同一种方式回答：用法横幅 + 成功。其中四个过去
            // 用 "unexpected argument" 拒绝这个旗标（审计 `LGC-LG-44`）。
            "-h" | "--help" => return crate::cli::usage(out),
            "--json" => json_output = true,
            _ if arg.starts_with('-') => return Err(format!("unexpected argument '{arg}'")),
            _ if directory.is_none() => directory = Some(arg),
            _ => return Err("check accepts at most one path".to_owned()),
        }
    }
    let directory = directory.unwrap_or_else(|| ".".to_owned());
    let (manifest, package) = match resolve_package(&directory) {
        Ok(resolved) => resolved,
        Err(error) => {
            // The `--json` contract is "stdout is one JSON document"; a
            // resolution failure used to return before writing anything, so a
            // machine reader got an empty stream instead of a document that
            // names the failure. The document keeps the success shape.
            // `--json` 契约是"stdout 是一个 JSON 文档"；解析失败此前在写出任何东西之前
            // 就返回，机器读者拿到的是空流，而不是点名失败的文档。该文档保持成功时的形状。
            if json_output {
                let mut diagnostics = nichlink_kernel::BuildDiagnostics::default();
                diagnostics.push(nichlink_kernel::BuildDiagnostic::new(
                    "resolve",
                    error.clone(),
                ));
                writeln!(out, "{}", diagnostics.to_json())
                    .map_err(|error| format!("cannot write output: {error}"))?;
            }
            return Err(error);
        }
    };
    let out_dir = build_out_dir(&manifest);
    if json_output {
        return match crate::build_method::check_for(&manifest, &out_dir, &package) {
            Ok(()) => {
                writeln!(
                    out,
                    "{}",
                    nichlink_kernel::BuildDiagnostics::default().to_json()
                )
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
    crate::build_method::run_for(&manifest, &out_dir, &package)?;
    writeln!(out, "nichlink check: ok ({package})")
        .map_err(|error| format!("cannot write output: {error}"))?;
    // The line the maintainer asked for: "see the terminal say the index is ready before you read
    // from it" (audit `M7`, P2.2). The historical line above is untouched — it stays the first line
    // and keeps its exact bytes — and this one names the index this run published, by generation and
    // by numbers, so a script or a person can wait on a *fact* rather than on elapsed time.
    // 维护者要的那一行："看到终端说索引就绪，再从它读"（审计 `M7`，P2.2）。上面那行历史输出没有被动过——
    // 它仍是第一行、字节不变——而这一行按 generation 与数字点名这次运行发布的索引，因此脚本或人都能等一个
    // **事实**，而不是等时间。
    let state = crate::mcp::index::state(&manifest);
    writeln!(out, "{}", crate::mcp::index::line(&manifest, &state))
        .map_err(|error| format!("cannot write output: {error}"))?;
    Ok(())
}
