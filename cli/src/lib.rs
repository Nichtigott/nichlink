//! NichLink command-line interface.
//! NichLink 命令行界面。

use std::path::{Path, PathBuf};

use nichlink_build::scaffold::{self, DependencySource, ProjectKind};

const USAGE: &str = "\
nichlink — NichLink command-line interface

USAGE:
    nichlink new <name> [--lib] [--path <workspace> | --git <url>]
    nichlink check [path]
    nichlink build [path] [cargo options]
    nichlink studio
    nichlink mcp

COMMANDS:
    new       Create a NichLink host project in ./<name>
    check     Run the registration discovery and validation pass without compiling
    build     Validate the registration tree, then run cargo build
    studio    Launch the Studio TUI for the current project
    mcp       Run the read-only MCP stdio bridge

OPTIONS:
    --lib             Create a library project instead of a binary
    --path <dir>      Source nichlink-core/build from a local checkout
    --git <url>       Source nichlink-core/build from a Git repository
";

pub fn main() -> Result<(), String> {
    run(std::env::args())
}

/// Dispatch one command from an argv-style iterator (the program name is
/// consumed and ignored). Shared by the `nichlink` and `cargo-nichlink`
/// binaries.
/// 从 argv 风格的迭代器分发一条命令（程序名会被消耗忽略）。`nichlink` 与
/// `cargo-nichlink` 两个二进制共用。
pub fn run(argv: impl IntoIterator<Item = String>) -> Result<(), String> {
    let mut args = argv.into_iter().skip(1);
    match args.next().as_deref() {
        None | Some("--help") | Some("-h") | Some("help") => {
            print!("{USAGE}");
            Ok(())
        }
        Some("new") => new(&mut args),
        Some("check") => check(&mut args),
        Some("build") => build(&mut args),
        Some("studio") => nichlink_studio::launch().map_err(|error| error.to_string()),
        Some("mcp") => {
            nichlink_mcp::run();
            Ok(())
        }
        Some(other) => Err(format!("unknown command '{other}' (see --help)")),
    }
}

fn new(args: &mut impl Iterator<Item = String>) -> Result<(), String> {
    let mut name: Option<String> = None;
    let mut lib = false;
    let mut path: Option<PathBuf> = None;
    let mut git: Option<String> = None;
    let mut args = args.by_ref().peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--lib" => lib = true,
            "--path" => {
                let value = args.next().ok_or("--path requires a directory")?;
                path =
                    Some(std::fs::canonicalize(&value).unwrap_or_else(|_| PathBuf::from(&value)));
            }
            "--git" => git = Some(args.next().ok_or("--git requires a URL")?),
            _ if name.is_none() => name = Some(arg),
            _ => return Err(format!("unexpected argument '{arg}'")),
        }
    }
    let name = name.ok_or("new requires a package name")?;
    let source = match (path, git) {
        (Some(workspace), _) => DependencySource::Local { workspace },
        (None, Some(url)) => DependencySource::Git { url },
        (None, None) => scaffold::detected_source(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            &std::env::current_exe()
                .map_err(|error| format!("cannot locate current executable: {error}"))?,
        ),
    };
    let root = std::env::current_dir()
        .map_err(|error| format!("cannot read current directory: {error}"))?
        .join(&name);
    let kind = if lib {
        ProjectKind::Library
    } else {
        ProjectKind::Binary
    };
    scaffold::create_project(&root, &name, kind, &source)?;
    println!(
        "created {} project at {}",
        if lib { "library" } else { "binary" },
        root.display()
    );
    Ok(())
}

fn check(args: &mut impl Iterator<Item = String>) -> Result<(), String> {
    let directory = args.next().unwrap_or_else(|| ".".to_owned());
    if args.next().is_some() {
        return Err("check accepts at most one path".to_owned());
    }
    let package = registration_check(&directory)?;
    println!("nichlink check: ok ({package})");
    Ok(())
}

/// Validate the registration tree first; only then invoke `cargo build`
/// with the remaining arguments passed through verbatim. The path, when
/// given, must come before any cargo option.
/// 先校验注册树；通过后再调用 `cargo build`，其余参数原样透传。
/// 路径如给出，必须位于任何 cargo 选项之前。
fn build(args: &mut impl Iterator<Item = String>) -> Result<(), String> {
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

/// Run registration discovery and validation for the host project at
/// `directory`, returning the package name on success.
/// 为 `directory` 处的宿主项目运行注册发现与校验，成功时返回包名。
fn registration_check(directory: &str) -> Result<String, String> {
    let manifest = std::fs::canonicalize(directory)
        .map_err(|error| format!("cannot resolve {directory}: {error}"))?;
    if !manifest.join("Cargo.toml").is_file() {
        return Err(format!("{} has no Cargo.toml", manifest.display()));
    }
    let package = package_name(&manifest)?;
    // The identity pass reads CARGO_PKG_NAME at runtime. Cargo build scripts
    // receive it from Cargo; standalone runs must provide it themselves.
    // Soundness: this runs on the main thread before any worker threads
    // exist, and the process exits after the pipeline finishes.
    // 身份计算在运行期读取 CARGO_PKG_NAME。build script 由 Cargo 注入；
    // 独立运行必须自行提供。此处仅在主线程、任何工作线程启动前设置，
    // 且进程在管线结束后退出。
    unsafe { std::env::set_var("CARGO_PKG_NAME", &package) };
    let out_dir = manifest.join("target/nichlink/out");
    nichlink_build::run_for(&manifest, &out_dir)?;
    Ok(package)
}

/// Read the package name from `[package] name = "..."`. Falls back to the
/// manifest directory name for workspace-inherited names.
/// 从 `[package] name = "..."` 读取包名；workspace 继承名回退到目录名。
fn package_name(manifest: &Path) -> Result<String, String> {
    let content = std::fs::read_to_string(manifest.join("Cargo.toml"))
        .map_err(|error| format!("cannot read manifest: {error}"))?;
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("name = \"") {
            if let Some(name) = rest.split('"').next().filter(|name| !name.is_empty()) {
                return Ok(name.to_owned());
            }
        }
    }
    manifest
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "cannot determine package name".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{run, split_build_args};

    #[test]
    fn help_and_missing_command_succeed() {
        assert!(run(["nichlink".to_owned(), "--help".to_owned()]).is_ok());
        assert!(run(["nichlink".to_owned()]).is_ok());
    }

    #[test]
    fn unknown_command_is_an_error() {
        let result = run(["nichlink".to_owned(), "bogus".to_owned()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown command 'bogus'"));
    }

    #[test]
    fn new_requires_a_package_name() {
        let result = run(["nichlink".to_owned(), "new".to_owned()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("package name"));
    }

    #[test]
    fn build_args_split_leading_path_from_cargo_options() {
        let owned = |items: &[&str]| {
            items
                .iter()
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
        };
        let (path, rest) = split_build_args(&owned(&["app", "--release"]));
        assert_eq!(path.as_deref(), Some("app"));
        assert_eq!(rest, owned(&["--release"]));
        let (path, rest) = split_build_args(&owned(&["--release"]));
        assert_eq!(path, None);
        assert_eq!(rest, owned(&["--release"]));
        let (path, rest) = split_build_args(&[]);
        assert_eq!(path, None);
        assert!(rest.is_empty());
    }
}
