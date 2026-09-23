//! Project scaffolding for new NichLink host crates.
//! 新 NichLink 宿主 crate 的项目脚手架。
//!
//! Renders the manifest, build script, and source entry of a new host, and
//! injects the editor snippets so the first field an author writes is one pick
//! away.
//! 渲染新宿主的清单、构建脚本与源码入口，并注入编辑器 snippet，使作者写下的
//! 第一个字段就能一键得到。

use std::fs;
use std::path::{Path, PathBuf};

use super::snippets::{Editor, write_editor_snippets};

const NICHLINK_REPOSITORY: &str = "https://github.com/Nichtigott/nichlink";

/// What `nichlink new` scaffolds: a binary host or a library host.
/// `nichlink new` 生成的宿主类型：二进制宿主或库宿主。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectKind {
    /// A host whose entry is `src/main.rs`.
    /// 入口为 `src/main.rs` 的宿主。
    Binary,
    /// A host whose entry is `src/lib.rs`.
    /// 入口为 `src/lib.rs` 的宿主。
    Library,
}

impl ProjectKind {
    fn entry_file(self) -> &'static str {
        match self {
            ProjectKind::Binary => "src/main.rs",
            ProjectKind::Library => "src/lib.rs",
        }
    }
}

impl std::fmt::Display for ProjectKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            ProjectKind::Binary => "binary",
            ProjectKind::Library => "library",
        })
    }
}

/// Where a generated manifest sources the NichLink crates from.
/// 生成清单中 NichLink crate 的依赖来源。
#[derive(Clone, Debug)]
pub enum DependencySource {
    /// Path dependencies into a local NichLink checkout.
    /// 指向本地 NichLink checkout 的 path 依赖。
    Local {
        /// The checkout the generated manifest points at.
        /// 生成清单指向的检出目录。
        workspace: PathBuf,
    },
    /// Git dependencies; Cargo can resolve the same version from crates.io
    /// once a matching release is published.
    /// Git 依赖；发布匹配版本后 Cargo 可将同一版本解析到 crates.io。
    Git {
        /// The repository the generated manifest pulls from.
        /// 生成清单拉取的仓库地址。
        url: String,
    },
}

/// Detect the dependency source for a running tool binary: path dependencies
/// when the binary lives inside a NichLink checkout's own target directory;
/// portable Git dependencies otherwise.
/// 根据运行中的工具二进制位置检测依赖来源：位于 NichLink checkout 自身
/// target 目录内时使用 path 依赖，否则使用可移植的 Git 依赖。
pub fn detected_source(tool_manifest_dir: &Path, current_exe: &Path) -> DependencySource {
    let workspace = tool_manifest_dir.parent().unwrap_or_else(|| Path::new("."));
    let runs_from_workspace = current_exe.starts_with(workspace.join("target"))
        && workspace.join("core").is_dir()
        && workspace.join("build_method").is_dir();
    if runs_from_workspace {
        DependencySource::Local {
            workspace: workspace.to_path_buf(),
        }
    } else {
        DependencySource::Git {
            url: NICHLINK_REPOSITORY.to_owned(),
        }
    }
}

/// The `(runtime, build)` dependency requirement strings for a generated
/// manifest, in the two lines `Cargo.toml` needs.
/// 生成清单所需的 `(runtime, build)` 两行依赖声明，即 `Cargo.toml` 要的两条。
pub fn dependency_specs(source: &DependencySource) -> (String, String) {
    match source {
        DependencySource::Local { workspace } => {
            let runtime = toml_path(&workspace.join("run_method"));
            let build = toml_path(&workspace.join("build_method"));
            (
                format!(
                    "nichlink-run-method = {{ package = \"nichlink-run-method\", path = \"{runtime}\" }}"
                ),
                format!("nichlink-build-method = {{ path = \"{build}\" }}"),
            )
        }
        DependencySource::Git { url } => (
            format!(
                "nichlink-run-method = {{ package = \"nichlink-run-method\", git = \"{url}\", branch = \"main\", version = \"0.1.0\" }}"
            ),
            format!(
                "nichlink-build-method = {{ git = \"{url}\", branch = \"main\", version = \"0.1.0\" }}"
            ),
        ),
    }
}

/// Render the manifest, build script, and source entry for a new project.
/// 渲染新项目的 manifest、构建脚本和源码入口。
pub fn project_files(
    package: &str,
    kind: ProjectKind,
    source: &DependencySource,
) -> Vec<(&'static str, String)> {
    let (runtime_dependency, build_dependency) = dependency_specs(source);
    let prelude = format!(
        "nichlink_run_method::host!();\n{}",
        if kind == ProjectKind::Library {
            ""
        } else {
            "\nfn main() { println!(\"registered faces: {}\", builtin_static_plan().len()); }"
        }
    );
    let cargo = format!(
        "[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = \"build.rs\"\n\n[dependencies]\n{runtime_dependency}\n\n[build-dependencies]\n{build_dependency}\n"
    );
    vec![
        ("Cargo.toml", cargo),
        (
            "build.rs",
            "fn main() { nichlink_build_method::run(); }\n".to_owned(),
        ),
        (kind.entry_file(), prelude),
    ]
}

/// Validate the package name and target directory, then write the project.
/// 校验包名与目标目录后写入项目文件。
pub fn create_project(
    root: &Path,
    package: &str,
    kind: ProjectKind,
    source: &DependencySource,
) -> Result<(), String> {
    if package.is_empty() {
        return Err("package name is required".to_owned());
    }
    if !package
        .chars()
        .all(|ch| ch == '_' || ch == '-' || ch.is_ascii_alphanumeric())
    {
        return Err("package must use letters, digits, '_' or '-'".to_owned());
    }
    if root.exists()
        && fs::read_dir(root)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(true)
    {
        return Err(format!("{} is not empty", root.display()));
    }
    write_project_files(root, &project_files(package, kind, source))?;
    // A new project gets the face-field snippets too, so `kind: ` is one pick
    // away from the first field the author writes.
    // 新项目同时得到注册面字段 snippet，因此作者写下的第一个字段就能一键得到 `kind: `。
    write_editor_snippets(root, Editor::Vscode)?;
    Ok(())
}

fn write_project_files(root: &Path, files: &[(&str, String)]) -> Result<(), String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
    for (relative, content) in files {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        if let Err(error) = fs::write(&path, content) {
            return Err(format!("cannot write {}: {error}", path.display()));
        }
    }
    Ok(())
}

fn toml_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

#[cfg(test)]
mod tests {
    use super::{DependencySource, ProjectKind, create_project};
    use crate::scaffold::{Editor, SNIPPET_FILE, editor_snippets};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nichlink-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    /// A scaffolded project already carries the file, so the author never runs
    /// the command by hand.
    /// 脚手架生成的项目自带该文件，作者无需手动执行命令。
    #[test]
    fn a_new_project_carries_the_editor_snippets() {
        let root = temporary_directory("project-snippets").join("app");
        create_project(
            &root,
            "app",
            ProjectKind::Binary,
            &DependencySource::Git {
                url: "https://example.invalid/nichlink".to_owned(),
            },
        )
        .expect("project");
        let snippets = fs::read_to_string(root.join(SNIPPET_FILE)).expect("snippet file");
        assert_eq!(snippets, editor_snippets(Editor::Vscode));
        fs::remove_dir_all(root.parent().expect("project parent")).expect("cleanup");
    }
}
