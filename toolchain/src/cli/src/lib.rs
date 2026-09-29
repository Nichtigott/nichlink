//! NichLink command-line interface.
//! NichLink 命令行界面。

// The published surface must be readable on docs.rs without leaving the page,
// so the lint is on for the whole crate; `clippy -D warnings` makes a new
// undocumented public item a failure.
// 发布表面必须能在 docs.rs 上不跳页读懂，因此 lint 开在整个 crate 上；
// `clippy -D warnings` 会让新增的、没有文档的公开项变成失败。

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
#[path = "commands/studio.rs"]
mod studio_command;

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
    nichlink studio [path]
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
    studio    Launch the Studio TUI for the current project, or for `path`
    mcp       Run the MCP stdio bridge: source and registry queries, plus
              authoring writes that preview unless `apply: true`

OPTIONS:
    --lib             Create a library project instead of a binary
    --path <dir>      Source nichlink-kernel/build from a local checkout; for
                      explain, the host project to inspect (default: .)
    --git <url>       Source nichlink-kernel/build from a Git repository
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
    run(argv_strings(std::env::args_os())?)
}

/// Convert process arguments to text, naming the first that is not UTF-8.
/// 把进程参数转成文本，并点名第一个不是 UTF-8 的参数。
///
/// An argument on Linux may be any byte string, and `std::env::args()` *unwraps* the
/// conversion: `nichlink check "/tmp/proj\xff"` died with a Rust backtrace instead of
/// printing a usage error, which is not what a command-line tool owes a caller.
/// Linux 上的参数可以是任意字节串，而 `std::env::args()` 会对转换 **unwrap**：
/// `nichlink check "/tmp/proj\xff"` 会带着 Rust backtrace 死掉，而不是打印一条用法错误——
/// 这不是命令行工具该给调用方的答复。
pub fn argv_strings(
    argv: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<Vec<String>, String> {
    argv.into_iter()
        .map(|argument| {
            argument
                .into_string()
                .map_err(|bad| format!("argument {bad:?} is not valid UTF-8"))
        })
        .collect()
}

/// Write the CLI's usage banner for a command that was asked for help.
/// 为被请求帮助的命令写下 CLI 的用法横幅。
pub(crate) fn usage(out: &mut dyn Write) -> Result<(), String> {
    write!(out, "{USAGE}").map_err(|error| format!("cannot write usage: {error}"))
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
        Some("--help") | Some("-h") | Some("help") => usage(out),
        // A bare invocation is not a success: the usage still goes out, because a
        // caller who typed nothing needs to see it, and the exit status says the
        // command did not run. It used to print the usage and return `Ok`, so a shell
        // pipeline read bare `nichlink` as having succeeded (audit `LGC-LG-44`).
        // 裸调不是成功：用法照常输出（什么都没敲的调用方需要看到它），而退出状态说明命令没有运行。
        // 它过去打印用法并返回 `Ok`，于是 shell 管线把裸 `nichlink` 读成成功（审计 `LGC-LG-44`）。
        None => {
            usage(out)?;
            Err("no command given".to_owned())
        }
        Some("new") => new_command::new(&mut args, out),
        Some("check") => check_command::check(&mut args, out),
        Some("build") => build_command::build(&mut args, out),
        Some("snippets") => snippets_command::snippets(&mut args, out),
        Some("explain") => explain::explain(&mut args, out),
        Some("grafts") => grafts::grafts(&mut args, out),
        Some("studio") => studio_command::studio(&mut args, out),
        Some("mcp") => crate::mcp::run().map_err(|error| format!("mcp: {error}")),
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

/// The directory `--manifest-path` names in a passed-through cargo argument list.
/// 透传给 cargo 的参数列表里，`--manifest-path` 点名的那个目录。
///
/// Cargo accepts both `--manifest-path <p>` and `--manifest-path=<p>`, and `nichlink build` hands the
/// rest of its arguments to cargo verbatim. Reading only a leading *positional* path meant the two
/// halves of the command could describe different projects: the registration check ran on the
/// current directory while cargo built the named one, and the command printed
/// `registration ok (<cwd package>)` with cargo's exit code — so the named project's red verdict
/// was never seen and the current project's green one was reported as its conclusion (audit `S11`).
/// cargo 同时接受 `--manifest-path <p>` 与 `--manifest-path=<p>`，而 `nichlink build` 把其余参数原样
/// 交给 cargo。只读开头的**位臵**参数意味着本命令的两半可以描述不同的项目：注册校验跑在当前目录上，
/// 而 cargo 构建被点名的那个，命令还打印 `registration ok (<cwd package>)` 并只取 cargo 的退出码——
/// 被点名项目的红色判断从未被看到，当前项目的绿色判断却被当成它的结论（审计 `S11`）。
pub(crate) fn manifest_path_directory(args: &[String]) -> Option<String> {
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if let Some(value) = arg.strip_prefix("--manifest-path=") {
            return Some(value.to_owned());
        }
        if arg == "--manifest-path"
            && let Some(value) = args.get(index + 1)
        {
            return Some(value.clone());
        }
        index += 1;
    }
    None
}

/// The project `build` must validate: the named manifest's directory, the positional path, or the
/// current directory — refusing when the two ways of naming one disagree.
/// `build` 必须校验的项目：被点名清单所在目录、位臵参数、或当前目录——两种点名方式互相矛盾时拒绝。
///
/// Both halves of the command have to describe the same project; two names for two projects is a
/// usage error rather than a silent choice between them.
/// 本命令的两半必须描述同一个项目；两个名字指向两个项目时，这是用法错误，而不是在它们之间静默选一个。
pub(crate) fn build_target(
    positional: Option<String>,
    cargo_args: &[String],
) -> Result<String, String> {
    let named = manifest_path_directory(cargo_args).map(|manifest| {
        Path::new(&manifest)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
            .display()
            .to_string()
    });
    match (positional, named) {
        (None, None) => Ok(".".to_owned()),
        (Some(only), None) | (None, Some(only)) => Ok(only),
        (Some(positional), Some(named)) => {
            let same = std::fs::canonicalize(&positional)
                .ok()
                .zip(std::fs::canonicalize(&named).ok())
                .is_some_and(|(left, right)| left == right);
            if same {
                Ok(named)
            } else {
                Err(format!(
                    "`build {positional}` and `--manifest-path {named}` name different projects; \
                     pass one of them"
                ))
            }
        }
    }
}

/// Resolve one host project directory into its canonical root and the package
/// name Cargo answers for it.
/// 把一个宿主项目目录解析成规范根目录与 Cargo 为该包给出的包名。
///
/// The package name is the NodeId namespace for every operator command here, read from
/// `crate::build_time::package_name` — one authority, so these commands cannot disagree with
/// each other.
/// 包名即 NodeId 命名空间，在此由 `crate::build_time::package_name` 读取——同一权威，因此这些
/// 命令彼此不会分歧。
///
/// It is **not** one story across all surfaces, and the difference is stated rather than implied:
/// `NICH_LINK_NAMESPACE` is honored by the MCP bridge and by Studio, and read by no build-side code
/// (the declaration macros bake in `env!("CARGO_PKG_NAME")` at compile time), while nothing in
/// `cli/` reads it. With that variable set, those two report a different namespace — and therefore
/// different `NodeId`s — than this command does. Making the surfaces agree is the maintainer's
/// decision; the variable's documented purpose is a *reader's* override for trace artifacts
/// (`run_method/src/runtime/trace/snapshot/io.rs`), and until that decision is taken a reader who
/// sets it must know which side they are on (audit `S12`).
/// 但在所有执行面上**并非**同一个说法，这里把差异说出来而不是暗示：`NICH_LINK_NAMESPACE` 被 MCP 桥与
/// Studio 尊重、而没有任何构建侧代码读它（声明宏在编译期把 `env!("CARGO_PKG_NAME")` 烤进去），同时
/// `cli/` 里没有任何地方读它。一旦设置该变量，那两个执行面报告的命名空间——以及由此而来的
/// `NodeId`——就与本命令不同。让各执行面一致是维护者的决定；该变量文档化的用途是 trace artifact 的
/// **读取者覆盖**（`run_method/src/runtime/trace/snapshot/io.rs`），在这个决定做出之前，设置它的读者
/// 必须知道自己站在哪一边（审计 `S12`）。
pub(crate) fn resolve_package(directory: &str) -> Result<(PathBuf, String), String> {
    let manifest = std::fs::canonicalize(directory)
        .map_err(|error| format!("cannot resolve {directory}: {error}"))?;
    if !manifest.join("Cargo.toml").is_file() {
        return Err(format!("{} has no Cargo.toml", manifest.display()));
    }
    let package = crate::build_time::package_name(&manifest.join("Cargo.toml"))?;
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
    crate::build_time::run_for(&manifest, &out_dir, &package)?;
    Ok(package)
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;
