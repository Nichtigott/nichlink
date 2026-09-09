//! NichLink command-line interface.
//! NichLink 命令行界面。

use std::path::{Path, PathBuf};

use nichlink_build::scaffold::{self, DependencySource, ProjectKind};

const USAGE: &str = "\
nichlink — NichLink command-line interface

USAGE:
    nichlink new <name> [--lib] [--path <workspace> | --git <url>]
    nichlink studio
    nichlink mcp

COMMANDS:
    new       Create a NichLink host project in ./<name>
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

#[cfg(test)]
mod tests {
    use super::run;

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
}
