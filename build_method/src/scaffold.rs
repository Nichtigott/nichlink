//! Project scaffolding for new NichLink host crates.
//! 新 NichLink 宿主 crate 的项目脚手架。

use std::fs;
use std::path::{Path, PathBuf};

use nichlink::registry_core::declaration::FACE_FIELD_ORDER;

const NICHLINK_REPOSITORY: &str = "https://github.com/Nichtigott/nichlink";

/// Where an editor reads project-scoped snippets from.
/// 编辑器读取项目级 snippet 的位置。
///
/// VS Code calls this format `.code-snippets`; the file is plain JSON, so any
/// other editor can be fed the same object through `nichlink snippets --stdout`.
/// VS Code 把这个格式叫 `.code-snippets`；文件就是普通 JSON，因此任何别的编辑器都可以
/// 通过 `nichlink snippets --stdout` 拿到同一个对象。
pub const SNIPPET_FILE: &str = ".vscode/nichlink-face.code-snippets";

/// One snippet per face field: typing the field name offers an item that inserts
/// `name: ` and leaves the cursor after the colon.
/// 每个注册面字段一条 snippet：键入字段名时会出现一条插入 `name: ` 并把光标留在冒号后的候选。
///
/// An editor's field completion inserts the bare name (rust-analyzer does this
/// for every struct, macro or not), and a server-side snippet does not fire
/// inside a macro call's token tree, so the colon has to come from the editor's
/// own snippet layer. The names come from the kernel's `FACE_FIELD_ORDER`, so a
/// new field reaches every project without this file drifting.
/// 编辑器的字段补全插入的是裸名字（rust-analyzer 对任何结构体都如此，与是否宏无关），而
/// 服务端 snippet 在宏调用的 token 树里不会触发，因此冒号只能由编辑器自己的 snippet 层
/// 提供。字段名来自内核的 `FACE_FIELD_ORDER`，所以新增字段无需改这个文件就能到达每个项目。
pub fn editor_snippets() -> String {
    let mut output = String::from("{\n");
    for (index, field) in FACE_FIELD_ORDER.iter().enumerate() {
        if index > 0 {
            output.push_str(",\n");
        }
        output.push_str(&format!(
            "  \"{field}: \": {{\n    \"prefix\": [\"{field}\"],\n    \"body\": [\"{field}: $0\"],\n    \"scope\": \"rust\",\n    \"description\": \"插入 `{field}: `，光标停在冒号后 / insert `{field}: ` and stop after the colon\"\n  }}"
        ));
    }
    output.push_str("\n}\n");
    output
}

/// Write the editor snippets for an existing project, leaving the file alone
/// when it already has the current content.
/// 为已有项目写入编辑器 snippet；内容已经是最新时不改动文件。
pub fn write_editor_snippets(root: &Path) -> Result<bool, String> {
    let path = root.join(SNIPPET_FILE);
    let content = editor_snippets();
    if fs::read_to_string(&path).ok().as_deref() == Some(content.as_str()) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    fs::write(&path, content)
        .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    Ok(true)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectKind {
    Binary,
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
    Local { workspace: PathBuf },
    /// Git dependencies; Cargo can resolve the same version from crates.io
    /// once a matching release is published.
    /// Git 依赖；发布匹配版本后 Cargo 可将同一版本解析到 crates.io。
    Git { url: String },
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
    write_editor_snippets(root)?;
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
    use super::{
        DependencySource, FACE_FIELD_ORDER, ProjectKind, SNIPPET_FILE, create_project,
        editor_snippets, write_editor_snippets,
    };
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

    /// Every declared field gets exactly one snippet, keyed and prefixed by its
    /// own name, so the editor file follows the kernel vocabulary.
    /// 每个声明的字段恰好得到一条 snippet，键与前缀都是字段名，因此编辑器文件跟随内核词表。
    #[test]
    fn the_editor_snippets_cover_every_declared_field() {
        let snippets = editor_snippets();
        assert_eq!(
            snippets.matches("\"prefix\"").count(),
            FACE_FIELD_ORDER.len(),
            "one snippet per field"
        );
        for field in FACE_FIELD_ORDER {
            assert!(snippets.contains(&format!("\"{field}: \": {{")), "{field}");
            assert!(
                snippets.contains(&format!("\"prefix\": [\"{field}\"]")),
                "{field}"
            );
            assert!(
                snippets.contains(&format!("\"body\": [\"{field}: $0\"]")),
                "{field}"
            );
        }
    }

    /// Injecting twice leaves the second call with nothing to write.
    /// 注入两次时第二次无事可做。
    #[test]
    fn injecting_the_editor_snippets_is_idempotent() {
        let root = temporary_directory("snippets");
        assert!(write_editor_snippets(&root).expect("first injection"));
        let path = root.join(SNIPPET_FILE);
        let first = fs::read_to_string(&path).expect("snippet file");
        assert_eq!(first, editor_snippets());
        assert!(!write_editor_snippets(&root).expect("second injection"));
        assert_eq!(fs::read_to_string(&path).expect("snippet file"), first);
        fs::remove_dir_all(root).expect("cleanup");
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
        assert_eq!(snippets, editor_snippets());
        fs::remove_dir_all(root.parent().expect("project parent")).expect("cleanup");
    }
}
