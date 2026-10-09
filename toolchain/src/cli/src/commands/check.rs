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

/// Refuse a project whose dependency graph carries this workspace's own crates twice.
/// 拒绝一个依赖图里带着本工作区自己 crate 两份拷贝的项目。
///
/// The measured consequence is a type error inside a file the author cannot edit (see
/// [`crate::build_method::duplicated_own_crates`]), so the sentence has to arrive before the build does.
/// 实测的后果是作者改不了的文件里的一个类型错误（见 [`crate::build_method::duplicated_own_crates`]），
/// 因此这句话必须赶在构建之前到。
fn refuse_duplicated_own_crates(manifest: &std::path::Path) -> Result<(), String> {
    let duplicated = crate::build_method::duplicated_own_crates(manifest)?;
    let Some((name, copies)) = duplicated.first() else {
        return Ok(());
    };
    let others = duplicated
        .iter()
        .skip(1)
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    Err(format!(
        "this project's dependency graph carries `{name}` twice, and the two copies are distinct \
         types to rustc:\n  {}\n  {}\n\
         the generated tree names both — the host resolves its own copy, a graft implementation \
         resolves the one it was built against — so the failure lands inside a generated file the \
         author cannot edit (measured: `error[E0308]: mismatched types … expected `NodeId`, found a \
         different `NodeId`` at `generated_lib.rs:218`).\n\
         way forward: give every crate in this build the same source for it — either all from the \
         registry (`version = \"0.2.2\"`), or all from this checkout, which one `[patch.crates-io]` \
         table in the workspace root does for every member at once{others}",
        copies.first().cloned().unwrap_or_default(),
        copies.get(1).cloned().unwrap_or_default(),
        others = if others.is_empty() {
            String::new()
        } else {
            format!("\nthe same holds for: {}", others.join(", "))
        }
    ))
}

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
    // Two copies of this workspace's own crates cannot be linked into one build without the failure
    // landing inside a generated file, so this is asked **before** the pipeline runs — the one place a
    // reader can still be told to unify the source.
    // 本工作区自己的 crate 有两份拷贝时，一次构建不可能不把失败落进某个生成文件里，因此这一问发生在管线**之前**
    // ——那是读者还能被告知"把来源统一起来"的唯一位置。
    refuse_duplicated_own_crates(&manifest.join("Cargo.toml"))?;
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
