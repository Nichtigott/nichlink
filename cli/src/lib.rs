//! NichLink command-line interface.
//! NichLink 命令行界面。

// The published surface must be readable on docs.rs without leaving the page,
// so the lint is on for the whole crate; `clippy -D warnings` makes a new
// undocumented public item a failure.
// 发布表面必须能在 docs.rs 上不跳页读懂，因此 lint 开在整个 crate 上；
// `clippy -D warnings` 会让新增的、没有文档的公开项变成失败。
#![warn(missing_docs)]

use std::io::Write;
use std::path::{Path, PathBuf};

#[path = "commands/build.rs"]
mod build_command;
#[path = "commands/check.rs"]
mod check_command;
#[path = "explain.rs"]
mod explain;
#[path = "grafts.rs"]
mod grafts;
#[path = "commands/new.rs"]
mod new_command;
#[path = "commands/snippets.rs"]
mod snippets_command;

// Split decision: the dispatch surface (`main`/`run`/`run_to`), the USAGE text
// and the shared helpers stay here, so the public entry points keep their exact
// paths and every command resolves one package root and one output directory.
// Each subcommand implementation moved to `commands/<name>.rs` and is mounted
// with `#[path]`, matching the workspace-wide module-mounting rule. The tests
// moved to `lib_tests.rs` so this page stays a dispatch table rather than a
// 900-line file.
// 拆分决定：分发表面（`main`/`run`/`run_to`）、USAGE 文本与共用辅助函数留在这里，
// 因此公开入口保持原路径，且每条命令解析同一个包根与输出目录。各子命令实现移到
// `commands/<name>.rs` 并用 `#[path]` 挂载，符合全工作区的模块挂载规则。测试移到
// `lib_tests.rs`，使本页保持为一张分发表，而不是 900 行的文件。

const USAGE: &str = "\
nichlink — NichLink command-line interface

USAGE:
    nichlink new <name> [--lib] [--path <workspace> | --git <url>]
    nichlink check [path] [--json]
    nichlink build [path] [cargo options]
    nichlink snippets [path] [--editor vscode|nvim|blink|auto] [--stdout]
    nichlink explain <node-id|logical/path> [--path <dir>] [--json]
    nichlink explain --overlay [--path <dir>] [--json]
    nichlink grafts [path] [--json]
    nichlink studio
    nichlink mcp

COMMANDS:
    new       Create a NichLink host project in ./<name>
    check     Run the registration discovery and validation pass without compiling
    build     Validate the registration tree, then run cargo build
    snippets  Inject the face-field editor snippets (VS Code project file, or
              the LuaSnip file Neovim loads)
    explain   Resolve a node id or logical path and report its identity, build
              scope, pruning, and the declared graft cuts that name it; with
              --overlay, render the static overlay projection of every slot
    grafts    List every .nichlink/external-grafts/*/graft.plan, with its
              selector, target path, graft, full flag, and whether the host
              entry declares that slot (read-only)
    studio    Launch the Studio TUI for the current project
    mcp       Run the read-only MCP stdio bridge

OPTIONS:
    --lib             Create a library project instead of a binary
    --path <dir>      Source nichlink-core/build from a local checkout; for
                      explain, the host project to inspect (default: .)
    --git <url>       Source nichlink-core/build from a Git repository
    --json            Emit one JSON document on stdout instead of human text
                      (check, explain, grafts); check still exits non-zero on a
                      failed validation
    --overlay         With explain, render the static overlay projection of the
                      build's scope and declared cuts instead of one node
    --editor <name>   Editor to write snippets for: vscode (default), nvim
                      (LuaSnip), blink (blink.cmp) or auto (every editor
                      installed on this machine, in its user-level location;
                      fuzzy-matching engines need to be named explicitly)
    --stdout          Print the snippets instead of writing them (any editor)
";

/// Entry point for the `nichlink` binary: dispatch this process's own argv.
/// `nichlink` 二进制的入口：分发本进程自己的 argv。
///
/// Reads the real process arguments, writes the command's report to stdout, and
/// returns a failure as `Err` so the binary decides the exit code.
/// 读取真实进程参数，把命令报告写到 stdout，失败以 `Err` 返回，由二进制决定退出码。
pub fn main() -> Result<(), String> {
    run(std::env::args())
}

/// Dispatch one command from an argv-style iterator (the program name is
/// consumed and ignored). Shared by the `nichlink` and `cargo-nichlink`
/// binaries, and writes its reports to process stdout.
/// 从 argv 风格的迭代器分发一条命令（程序名会被消耗忽略）。`nichlink` 与
/// `cargo-nichlink` 两个二进制共用，报告写到进程 stdout。
pub fn run(argv: impl IntoIterator<Item = String>) -> Result<(), String> {
    run_to(argv, &mut std::io::stdout())
}

/// The same dispatch as [`run`], against an explicit sink.
/// 与 [`run`] 相同的分发，但写到显式指定的输出。
///
/// The operator commands (`check --json`, `explain`, `grafts`) exist to be read
/// by a machine, so their document has to be assertable without redirecting the
/// process's stdout from a test: this entry point is what the tests drive.
/// 操作命令（`check --json`、`explain`、`grafts`）存在的意义就是被机器读取，因此它们
/// 的文档必须能在不重定向进程 stdout 的情况下被测试断言：测试驱动的就是这个入口。
pub fn run_to(argv: impl IntoIterator<Item = String>, out: &mut dyn Write) -> Result<(), String> {
    let mut args = argv.into_iter().skip(1);
    match args.next().as_deref() {
        None | Some("--help") | Some("-h") | Some("help") => {
            write!(out, "{USAGE}").map_err(|error| format!("cannot write usage: {error}"))?;
            Ok(())
        }
        Some("new") => new_command::new(&mut args),
        Some("check") => check_command::check(&mut args, out),
        Some("build") => build_command::build(&mut args),
        Some("snippets") => snippets_command::snippets(&mut args),
        Some("explain") => explain::explain(&mut args, out),
        Some("grafts") => grafts::grafts(&mut args, out),
        Some("studio") => nichlink_studio::launch().map_err(|error| error.to_string()),
        Some("mcp") => nichlink_mcp::run().map_err(|error| format!("mcp: {error}")),
        Some(other) => Err(format!("unknown command '{other}' (see --help)")),
    }
}

/// Split build args into an optional leading path and the remaining cargo
/// options. The path must come first; anything after it is passed verbatim.
/// 将 build 参数拆成可选的首个路径和其余 cargo 选项。路径必须在前，
/// 之后的所有内容原样透传。
fn split_build_args(args: &[String]) -> (Option<String>, Vec<String>) {
    match args.first() {
        Some(first) if !first.starts_with('-') => (Some(first.clone()), args[1..].to_vec()),
        _ => (None, args.to_vec()),
    }
}

/// Resolve one host project directory into its canonical root and the package
/// name Cargo answers for it.
/// 把一个宿主项目目录解析成规范根目录与 Cargo 为该包给出的包名。
///
/// The package name is the NodeId namespace, so every operator command has to
/// read it from the same authority before it can name a face.
/// 包名即 NodeId 命名空间，因此每条操作命令都必须先向同一权威读取它，才能命名一个面。
pub(crate) fn resolve_package(directory: &str) -> Result<(PathBuf, String), String> {
    let manifest = std::fs::canonicalize(directory)
        .map_err(|error| format!("cannot resolve {directory}: {error}"))?;
    if !manifest.join("Cargo.toml").is_file() {
        return Err(format!("{} has no Cargo.toml", manifest.display()));
    }
    let package = package_name(&manifest)?;
    Ok((manifest, package))
}

/// The directory the build publishes its generated plan and manifest output
/// into, relative to a package root.
/// 构建发布生成计划与清单产物的目录，相对包根。
///
/// `explain` reads `source_scope.tsv` and `pruning_manifest.tsv` from here; the
/// path is the same one `registration_check` writes.
/// `explain` 从这里读 `source_scope.tsv` 与 `pruning_manifest.tsv`；该路径与
/// `registration_check` 写出的相同。
pub(crate) fn build_out_dir(manifest: &Path) -> PathBuf {
    manifest.join("target/nichlink/out")
}

/// Run registration discovery and validation for the host project at
/// `directory`, returning the package name on success.
/// 为 `directory` 处的宿主项目运行注册发现与校验，成功时返回包名。
fn registration_check(directory: &str) -> Result<String, String> {
    let (manifest, package) = resolve_package(directory)?;
    let out_dir = build_out_dir(&manifest);
    nichlink_build_method::run_for(&manifest, &out_dir, &package)?;
    Ok(package)
}

/// Ask Cargo for the package name of the manifest at `manifest`.
/// 向 Cargo 询问 `manifest` 处 manifest 的包名。
///
/// Reading `[package] name` by hand was wrong three ways at once: it did not
/// know TOML sections, so a `[lib]` or `[[bin]]` name key could answer first; it
/// did not know single-quoted strings; and when it found nothing it silently
/// returned the directory name. The package name is the NodeId namespace, so a
/// wrong answer makes the same crate report one identity under `nichlink check`
/// and another under a real build. Cargo is the authority, and a question it
/// cannot answer is an error rather than a guess.
/// 手写读 `[package] name` 同时错了三处：它不认识 TOML 区段，于是 `[lib]` 或
/// `[[bin]]` 的 name 键可能抢先作答；它不认识单引号字符串；找不到时还会静默返回目录
/// 名。包名是 NodeId 的命名空间，答错就会让同一个 crate 在 `nichlink check` 与真实
/// 编译下报告两种身份。Cargo 才是权威；它答不出的问题一律报错，而不是猜。
fn package_name(manifest: &Path) -> Result<String, String> {
    let output = std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .arg("--manifest-path")
        .arg(manifest.join("Cargo.toml"))
        .output()
        .map_err(|error| format!("cannot run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed for {}: {}",
            manifest.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cannot read cargo metadata output: {error}"))?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or_else(|| "cargo metadata reported no packages".to_owned())?;
    // `--no-deps` lists the whole workspace, so the package has to be the one
    // whose manifest this is; taking the first entry would answer for a sibling
    // member.
    // `--no-deps` 会列出整个 workspace，因此必须挑出 manifest 正是这一个的包；取第一
    // 项会替同工作区的另一个成员作答。
    let package = packages
        .iter()
        .find(|package| {
            package["manifest_path"].as_str().is_some_and(|path| {
                Path::new(path)
                    .parent()
                    .is_some_and(|parent| same_directory(parent, manifest))
            })
        })
        .ok_or_else(|| {
            format!(
                "{} is not a package; cargo metadata listed {} workspace member(s)",
                manifest.display(),
                packages.len()
            )
        })?;
    package["name"].as_str().map(str::to_owned).ok_or_else(|| {
        format!(
            "cargo metadata reported no package name for {}",
            manifest.display()
        )
    })
}

/// Whether two paths name the same directory, following symlinks when possible.
/// 两个路径是否指向同一目录，能跟随符号链接时跟随。
fn same_directory(left: &Path, right: &Path) -> bool {
    let canonical =
        |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    canonical(left) == canonical(right)
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
