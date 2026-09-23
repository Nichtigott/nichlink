//! `nichlink new`: scaffold a host project in `./<name>`.
//! `nichlink new`：在 `./<name>` 中搭建宿主项目。
//!
//! Split out of `lib.rs` because scaffolding is a self-contained execution
//! surface: it reads the current directory and the running executable to decide
//! where the `nichlink-core`/`nichlink-build-method` dependency comes from, and
//! touches no other command's state.
//! 从 `lib.rs` 拆出，因为脚手架是一块自包含的执行面：它读当前目录与正在运行的可执行
//! 文件来决定 `nichlink-core`/`nichlink-build-method` 依赖来自哪里，不触碰其他命令的
//! 状态。

use std::path::{Path, PathBuf};

use nichlink_build_method::scaffold::{self, DependencySource, ProjectKind};

pub(crate) fn new(args: &mut impl Iterator<Item = String>) -> Result<(), String> {
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
