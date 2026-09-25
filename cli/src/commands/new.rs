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
                // The explicit path is a claim about where a NichLink checkout is, and
                // nothing checked it: `--path /tmp` wrote `path = "/tmp/run_method"` and
                // exited 0. Refuse before anything is written, and resolve the path
                // rather than passing a typo through.
                // 显式路径是关于"NichLink 检出在哪"的主张，而过去没有任何检查：`--path /tmp`
                // 写下 `path = "/tmp/run_method"` 并退出 0。在写任何东西之前就拒绝，并解析路径
                // 而不是把拼错的东西透传下去。
                let directory = std::fs::canonicalize(&value)
                    .map_err(|error| format!("--path {value}: {error}"))?;
                if !is_checkout(&directory) {
                    return Err(format!(
                        "--path {value} is not a NichLink checkout: it has no core/, build_method/ \
                         and run_method/"
                    ));
                }
                path = Some(directory);
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

/// Whether a directory is a NichLink checkout this scaffold can point at.
/// 某个目录是否是脚手架可以指向的 NichLink 检出。
///
/// The check lives here rather than in `build_method` because the scaffold writes a
/// *dependency* into someone else's manifest: the predicate has to be enforced by the
/// caller that is about to write it, and keeping it local means the packaged CLI does
/// not need a symbol newer than the published `nichlink-build-method`.
/// 这个判断放在这里而不是 `build_method`，因为脚手架是把一条**依赖**写进别人的清单：判断必须
/// 由即将写下它的调用方执行，而放在本地意味着打包后的 CLI 不需要一个比已发布
/// `nichlink-build-method` 更新的符号。
fn is_checkout(workspace: &Path) -> bool {
    workspace.join("core").is_dir()
        && workspace.join("build_method").is_dir()
        && workspace.join("run_method").is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory is a checkout only when the three crates a scaffold points at are
    /// there; an empty or unrelated directory is refused before anything is written.
    /// 只有当脚手架要指向的三个 crate 都在时目录才算检出；空目录或无关目录会在写下任何东西之前
    /// 被拒绝。
    #[test]
    fn only_a_real_checkout_is_accepted() {
        let root =
            std::env::temp_dir().join(format!("nichlink-new-checkout-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("fixture directory");
        assert!(!is_checkout(&root), "an empty directory is not a checkout");
        for directory in ["core", "build_method", "run_method"] {
            std::fs::create_dir_all(root.join(directory)).expect("fixture directory");
        }
        assert!(is_checkout(&root), "the three crates make it a checkout");
        let _ = std::fs::remove_dir_all(&root);
    }
}
