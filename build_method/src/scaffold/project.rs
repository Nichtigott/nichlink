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
                    "nichlink-run-method = {{ package = \"nichlink-run-method\", path = \"{runtime}\", version = \"0.1.0\" }}"
                ),
                format!("nichlink-build-method = {{ path = \"{build}\", version = \"0.1.0\" }}"),
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
    // A binary host demonstrates the whole runtime-evidence chain, because that is
    // the half nothing else in this workspace does: record under the mode the
    // environment asks for, then write the artifact where every reader looks
    // (`NICH_LINK_TRACE_FILE`, else `.nichlink/traces/nichlink.trace`). It is inert
    // until someone opts in — `CallTrace::runtime()` is `off` in a release build and
    // `errors-only` in a debug one — so the release path collects nothing; the point
    // is that the chain is visible and runnable, not that evidence always exists.
    // 二进制宿主演示整条运行期证据链，因为这是本工作区里别的任何东西都不做的那一半：按环境要求的模式
    // 记录，再把 artifact 写到所有读取方都看的地方（`NICH_LINK_TRACE_FILE`，否则
    // `.nichlink/traces/nichlink.trace`）。在有人 opt-in 之前它是惰性的——`CallTrace::runtime()`
    // 在 release 构建里是 `off`、在 debug 里是 `errors-only`——因此发布路径什么都不收集；意义在于那条
    // 链可见且可跑，而不是永远存在证据。
    let prelude = format!(
        "nichlink_run_method::host!();\n{}",
        if kind == ProjectKind::Library {
            "\n// A library host has no `main` to write at the end of: record around the work you\n\
             // actually run (`CallTrace::runtime`, `trace_call!`), then write it with\n\
             // `trace_artifact_path` + `write_trace_artifact`, or scaffold a binary for the demo.\n"
        } else {
            "\nfn main() {\n    \
             // Evidence is opt-in: neither `NICH_LINK_TRACE` (the collection mode) nor\n    \
             // `NICH_LINK_TRACE_FILE` (its path) is set by default, and without one of them this\n    \
             // host records and writes nothing at all.\n    \
             let asked = std::env::var_os(nichlink_run_method::lexicon::TRACE_FILE_ENV).is_some()\n        \
             || std::env::var_os(\"NICH_LINK_TRACE\").is_some();\n    \
             let mut trace = nichlink_run_method::CallTrace::runtime();\n    \
             let root = nichlink_run_method::root_node_id(env!(\"CARGO_PKG_NAME\"));\n    \
             let faces = trace.with(root, \"main\", |_| builtin_static_plan().len());\n    \
             println!(\"registered faces: {faces}\");\n    \
             if !asked {\n        return;\n    }\n    \
             let path = nichlink_run_method::trace_artifact_path(std::path::Path::new(env!(\n        \
             \"CARGO_MANIFEST_DIR\",\n    )));\n    \
             match nichlink_run_method::write_trace_artifact(&trace, &path, env!(\"CARGO_PKG_NAME\")) {\n        \
             Ok(()) => println!(\"trace written: {}\", path.display()),\n        \
             Err(error) => eprintln!(\"nichlink: trace not written: {error}\"),\n    }\n}\n"
        }
    );
    // The inner `[workspace]` table makes the generated manifest its own workspace root.
    // Without it, scaffolding inside an existing workspace fails `cargo metadata`,
    // `check` and `build` with "current package believes it's in a workspace when it's
    // not" — the repository's own rehearsal script had to append the table by hand.
    // 内部的 `[workspace]` 表让生成的清单成为它自己的工作区根。没有它，在已有工作区内脚手架会
    // 让 `cargo metadata`、`check` 与 `build` 报 "current package believes it's in a workspace
    // when it's not"——本仓库自己的演练脚本不得不手工追加这张表。
    let cargo = format!(
        "[workspace]\n\n[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\nbuild = \"build.rs\"\n\n[dependencies]\n{runtime_dependency}\n\n[build-dependencies]\n{build_dependency}\n"
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
    // A half-scaffolded project is worse than none: the manifest may name modules
    // whose files never arrived, and the next attempt refuses the directory as
    // non-empty. The emptiness check above means everything under `root` is ours,
    // so a failure removes what we wrote — unless the directory already existed,
    // in which case the error says what was left behind rather than deleting a
    // directory the reader made.
    // 半成品的项目比没有更糟：清单可能引用从未出现的模块文件，而下一次尝试会以"非空"拒绝
    // 该目录。上面的空目录检查意味着 `root` 下的一切都是我们写的，因此失败时移除我们写下的
    // 内容——除非该目录本来就存在，那种情况下错误会说明留下了什么，而不是删掉读者建的目录。
    let existed = root.exists();
    write_project(root, &project_files(package, kind, source), existed)
}

/// Write every part of a project, removing it again when any step fails.
/// 写下一个项目的每个部分；任何一步失败时再把它移除。
///
/// The emptiness check in `create_project` means everything under `root` is ours,
/// so a failure removes what we wrote — unless the directory already existed, in
/// which case the error says what was left behind rather than deleting a
/// directory the reader made.
/// `create_project` 的空目录检查意味着 `root` 下的一切都是我们写的，因此失败时移除我们
/// 写下的内容——除非该目录本来就存在，那种情况下错误会说明留下了什么，而不是删掉读者建的
/// 目录。
fn write_project(root: &Path, files: &[(&str, String)], existed: bool) -> Result<(), String> {
    let outcome = write_project_files(root, files).and_then(|()| {
        // A new project gets the face-field snippets too, so `kind: ` is one pick
        // away from the first field the author writes.
        // 新项目同时得到注册面字段 snippet，因此作者写下的第一个字段就能一键得到 `kind: `。
        write_editor_snippets(root, Editor::Vscode)
    });
    if let Err(error) = outcome {
        if existed {
            return Err(format!(
                "{error}; a partial project was left in {}",
                root.display()
            ));
        }
        let _ = fs::remove_dir_all(root);
        return Err(format!("{error}; the partial project was removed"));
    }
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
    use super::{DependencySource, ProjectKind, create_project, dependency_specs, project_files};
    use crate::scaffold::{Editor, SNIPPET_FILE, editor_snippets};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A new binary host demonstrates the runtime-evidence chain, because no
    /// other project in this repository does: record under the mode the
    /// environment asks for, then write the artifact where every reader looks.
    /// It stays inert until someone opts in, so the release path collects
    /// nothing — the point of the demo is that the *chain* is visible and
    /// runnable, not that evidence is always collected.
    /// 新的二进制宿主演示运行期证据链，因为本仓库里没有别的项目做这件事：按环境要求的模式记录，再把
    /// artifact 写到所有读取方都看的地方。在有人 opt-in 之前它保持惰性，因此发布路径什么都不收集——
    /// 这个演示的意义在于那条**链**可见且可跑，而不是永远在收集证据。
    #[test]
    fn a_binary_host_records_and_writes_a_trace() {
        let source = DependencySource::Local {
            workspace: PathBuf::from("/checkout"),
        };
        let entry = project_files("probe", ProjectKind::Binary, &source)
            .into_iter()
            .find(|(name, _)| *name == "src/main.rs")
            .expect("the binary entry is generated")
            .1;
        for needed in [
            "CallTrace::runtime()",
            "trace_artifact_path",
            "write_trace_artifact",
            "TRACE_FILE_ENV",
            "NICH_LINK_TRACE",
        ] {
            assert!(
                entry.contains(needed),
                "the demo must contain {needed}: {entry}"
            );
        }
        // A library host has no `main` to write at the end of, so it gets the pointer
        // instead of a demo that cannot run. The pointer names both APIs, which is why
        // the assertion is about the absent `main` rather than about a name appearing
        // in a comment.
        // 库宿主没有可在结尾写入的 `main`，因此它拿到的是指引，而不是一段跑不起来的演示。指引里两个
        // API 都会被点名，所以断言落在"没有 `main`"上，而不是落在"某个名字出现在注释里"。
        let library = project_files("probe", ProjectKind::Library, &source)
            .into_iter()
            .find(|(name, _)| *name == "src/lib.rs")
            .expect("the library entry is generated")
            .1;
        assert!(!library.contains("fn main"), "{library}");
        assert!(library.contains("trace_artifact_path"), "{library}");
        assert!(library.contains("CallTrace::runtime"), "{library}");
    }

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

    /// A failure part-way through leaves no half-project behind, and a directory
    /// the reader already had is reported rather than deleted.
    /// 进行到一半时的失败不会留下半个项目，而读者本来就有的目录会被如实报告而不是删掉。
    #[test]
    fn a_failed_scaffold_removes_what_it_wrote() {
        let root = temporary_directory("project-partial");
        // The first entry creates `a` as a directory, so the second cannot write
        // `a` as a file: the failure happens after something was written.
        // 第一条把 `a` 建成目录，因此第二条无法把 `a` 当文件写入：失败发生在已经写下东西之后。
        let files = vec![
            ("a/b.txt", "written first".to_owned()),
            ("a", "cannot be written".to_owned()),
        ];

        let error =
            super::write_project(&root, &files, false).expect_err("the second write must fail");
        assert!(error.contains("the partial project was removed"), "{error}");
        assert!(
            !root.exists(),
            "nothing of the failed scaffold is left behind"
        );

        fs::create_dir_all(&root).expect("a directory the reader already had");
        let error =
            super::write_project(&root, &files, true).expect_err("the second write must fail");
        assert!(error.contains("a partial project was left in"), "{error}");
        assert!(
            root.join("a/b.txt").is_file(),
            "a pre-existing directory is reported, not deleted"
        );
        let _ = fs::remove_dir_all(&root);
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
    /// Every internal requirement in a generated manifest carries a version, from both
    /// dependency sources, and the manifest is its own workspace root.
    /// 生成的清单里每处内部要求都带版本（两种依赖来源都算），且该清单是自己的根工作区。
    ///
    /// `cargo publish --dry-run` refuses a manifest whose path dependency has no
    /// `version`, and a manifest without its own `[workspace]` table cannot be built
    /// inside an existing workspace ("current package believes it's in a workspace when
    /// it's not"). Both were measured on a scaffolded project.
    /// `cargo publish --dry-run` 会拒绝带无版本路径依赖的清单，而缺少自己的 `[workspace]`
    /// 表的清单无法在已有工作区里构建（"current package believes it's in a workspace when
    /// it's not"）。两者都在脚手架产物上实测过。
    #[test]
    fn a_generated_project_can_be_published_and_built_in_a_workspace() {
        let local = DependencySource::Local {
            workspace: std::path::PathBuf::from("/tmp/nichlink"),
        };
        let git = DependencySource::Git {
            url: "https://github.com/Nichtigall/nichlink".to_owned(),
        };
        for source in [&local, &git] {
            let (runtime, build) = dependency_specs(source);
            for spec in [&runtime, &build] {
                assert!(
                    spec.contains("version = \"0.1.0\""),
                    "a published package needs a version requirement: {spec}"
                );
            }
            let files = project_files("probe", ProjectKind::Binary, source);
            let cargo = &files
                .iter()
                .find(|(name, _)| *name == "Cargo.toml")
                .expect("the manifest is generated")
                .1;
            assert!(
                cargo.starts_with("[workspace]"),
                "the generated manifest is its own workspace root: {cargo}"
            );
        }
    }
}
