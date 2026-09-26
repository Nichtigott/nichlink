//! Crate, directory, and library names have to agree.
//! crate 名、目录名与 library 名必须一致。
//!
//! `AGENTS.md` change rule 4 states the rule (`nichlink-<x>` / `<x>/` /
//! `nichlink_<x>`) and says why it matters: scaffold templates in `build_method/`
//! and CI reference these names too. Nothing checked it, so the rule held only
//! while reviewers remembered it — and the failure is quiet in the worst way. A
//! directory whose crate is named something else still builds here; it breaks a
//! host that writes `nichlink_<x>::…`, a scaffold that generates `<x>`, or a CI
//! step that runs `cargo test -p nichlink-<x>`.
//! `AGENTS.md` 的改动规则 4 陈述了这条规则（`nichlink-<x>` / `<x>/` / `nichlink_<x>`）并说明
//! 了它为何重要：`build_method/` 里的脚手架模板与 CI 也引用这些名字。此前没有任何东西检查它，
//! 于是这条规则只在评审者记得时成立——而它的失败是最安静的那种：目录里那个 crate 叫别的名字
//! 在这里照样能构建，破的是写着 `nichlink_<x>::…` 的宿主、生成 `<x>` 的脚手架、或跑
//! `cargo test -p nichlink-<x>` 的 CI 步骤。
//!
//! The example hosts are outside the rule rather than an exception to it: they are a
//! different family (`nichlink-example-*` with their own lib names), they are
//! `publish = false`, and `AGENTS.md` says a host documents its own types and is not
//! linted like a crate. Every other member is checked, including the gates crate.
//! 示例宿主在规则之外，而不是规则的例外：它们是另一个家族（`nichlink-example-*` 与它们自己的
//! lib 名）、`publish = false`，而 `AGENTS.md` 说宿主自己负责文档化自己的类型、不像 crate 那样
//! 参与 lint。其余每个成员都被检查，门禁 crate 本身也不例外。

use std::fs;
use std::path::Path;

use crate::{crate_directories, is_real_directory, relative, rust_sources};

/// Directories whose library name deliberately does not follow `nichlink_<dir>`.
/// 库名有意不遵循 `nichlink_<dir>` 的目录。
///
/// One entry, measured rather than assumed: the kernel's library is `nichlink`, not
/// `nichlink_core`, and every host, README and doctest writes that name. The crate
/// table in `AGENTS.md` records it as `nichlink-core (lib nichlink)`.
/// 只有一项，且是实测而非假设：内核的 library 是 `nichlink` 而不是 `nichlink_core`，每个宿主、
/// README 与 doctest 都写这个名字。`AGENTS.md` 的 crate 表把它记为
/// `nichlink-core (lib nichlink)`。
pub const LIB_NAME_EXCEPTIONS: &[(&str, &str)] = &[("core", "nichlink")];

/// One crate directory whose names disagree.
/// 一个名字互相不一致的 crate 目录。
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    /// The crate directory, relative to the workspace root.
    /// crate 目录，以工作区根为基准。
    pub directory: String,
    /// What disagrees, with both values.
    /// 什么不一致，以及两个值。
    pub reason: String,
}

/// Every naming disagreement in the workspace, sorted by directory.
/// 工作区里每一处命名不一致，按目录排序。
pub fn findings(root: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    // Every member's package name, including the example hosts: they are outside
    // the naming rule below, but CI runs `cargo test -p nichlink-example-…`, so a
    // reference to one of them is a reference to a real package.
    // 每个成员的包名，包括示例宿主：它们在下面的命名规则之外，但 CI 会跑
    // `cargo test -p nichlink-example-…`，因此对它们的引用是对真实包的引用。
    let manifests: Vec<(std::path::PathBuf, String, String, String)> = crate_directories(root)
        .into_iter()
        .map(|directory| {
            let text = fs::read_to_string(directory.join("Cargo.toml"))
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()));
            (
                directory.clone(),
                relative(root, &directory),
                manifest_value(&text, "package", "name").unwrap_or_default(),
                text,
            )
        })
        .collect();
    let packages: Vec<String> = manifests
        .iter()
        .map(|(_, _, package, _)| package.clone())
        .filter(|package| !package.is_empty())
        .collect();
    for (directory, relative_directory, package, text) in &manifests {
        // The example hosts are a family of their own; see the module docs.
        // 示例宿主自成一个家族；见模块文档。
        if relative_directory.starts_with("examples/") {
            continue;
        }
        let Some(name) = directory.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let expected_package = format!("nichlink-{}", name.replace('_', "-"));
        match package.as_str() {
            "" => found.push(Finding {
                directory: relative_directory.clone(),
                reason: "no `[package] name` in Cargo.toml".to_owned(),
            }),
            actual if actual == expected_package => {}
            actual => found.push(Finding {
                directory: relative_directory.clone(),
                reason: format!("package name is `{actual}`, not `{expected_package}`"),
            }),
        }
        let expected_lib = LIB_NAME_EXCEPTIONS
            .iter()
            .find(|(exception, _)| *exception == name)
            .map(|(_, lib)| (*lib).to_owned())
            // A Rust library name cannot carry a hyphen, so a hyphenated directory
            // (`plugin-host/`) maps to an underscore (`nichlink_plugin_host`). This
            // gate reported the workspace's own `plugin-host` before the fold was
            // here, which is the pin below.
            // Rust 的 library 名不能带连字符，因此带连字符的目录（`plugin-host/`）映射为下划线
            // （`nichlink_plugin_host`）。在这次折叠出现之前，本门禁曾把工作区自己的
            // `plugin-host` 报出来——正是下面那条钉子。
            .unwrap_or_else(|| format!("nichlink_{}", name.replace('-', "_")));
        match manifest_value(text, "lib", "name") {
            Some(actual) if actual == expected_lib => {}
            Some(actual) => found.push(Finding {
                directory: relative_directory.clone(),
                reason: format!("lib name is `{actual}`, not `{expected_lib}`"),
            }),
            // No `[lib] name` means cargo derives it from the package name, which
            // the check above already tied to the directory.
            // 没有 `[lib] name` 表示 cargo 从包名推导它，而包名已被上面的检查绑到目录名。
            None => {}
        }
    }
    found.extend(referenced_names(root, &packages));
    found.sort();
    found.dedup();
    found
}

/// Every package name the scaffold templates and CI name, checked against the manifests.
/// 脚手架模板与 CI 指名的每个包名，与清单核对。
///
/// `AGENTS.md` change rule 4 says the scaffold templates in `build_method/` and CI
/// reference these names too, and the manifest walk above cannot see them: a rename
/// that misses a template produces a host whose generated manifest requires a crate
/// that does not exist, and a CI step that runs `cargo test -p <old name>` fails only
/// when it runs.
/// `AGENTS.md` 改动规则 4 说 `build_method/` 里的脚手架模板与 CI 也引用这些名字，而上面的清单
/// 遍历看不到它们：漏改一个模板会产出"生成的清单要求一个不存在的 crate"的宿主，而 CI 里
/// `cargo test -p <旧名>` 只会在真正跑到时才失败。
///
/// Boundary: only positions where a name is a *requirement* are read — a dependency
/// key (`nichlink-x = { … }`) or `package = "nichlink-x"` in a template, and a
/// `-p`/`--package` argument in a workflow. Everywhere else a `nichlink-…` string is
/// a tool name, a job name, or prose, and reporting those would make the gate noise.
/// 边界：只读名字处于**要求**位置的地方——模板里的依赖键（`nichlink-x = { … }`）或
/// `package = "nichlink-x"`，以及工作流里的 `-p`/`--package` 实参。其他位置的 `nichlink-…`
/// 字符串是工具名、任务名或散文，报出来只会让门禁变成噪声。
fn referenced_names(root: &Path, packages: &[String]) -> Vec<Finding> {
    let mut found = Vec::new();
    let check = |path: &Path, name: &str, found: &mut Vec<Finding>| {
        if !packages.iter().any(|package| package == name) {
            found.push(Finding {
                directory: relative(root, path),
                reason: format!("names `{name}`, which is not a workspace package"),
            });
        }
    };
    let scaffold = root.join("build_method/src/scaffold");
    if is_real_directory(&scaffold) {
        for path in rust_sources(&scaffold) {
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            for name in requirement_names(&text) {
                check(&path, &name, &mut found);
            }
        }
    }
    let workflows = root.join(".github/workflows");
    if is_real_directory(&workflows) {
        let Ok(entries) = fs::read_dir(&workflows) else {
            return found;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|extension| extension != "yml") {
                continue;
            }
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
            for name in package_arguments(&text) {
                check(&path, &name, &mut found);
            }
        }
    }
    found
}

/// `nichlink-…` names in dependency-key or `package = "…"` position in a template.
/// 模板中处于依赖键位置或 `package = "…"` 位置的 `nichlink-…` 名。
fn requirement_names(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut from = 0usize;
    while let Some(offset) = text[from..].find("nichlink-") {
        let start = from + offset;
        from = start + "nichlink-".len();
        let tail: String = text[from..]
            .chars()
            .take_while(|character| character.is_ascii_lowercase() || *character == '-')
            .collect();
        if tail.is_empty() {
            continue;
        }
        let after = text[from + tail.len()..].trim_start();
        // Only `=`: a template that merely starts with the name is an identifier
        // being built (`format!("nichlink-auto-{}-{}", …)`), not a requirement.
        // 只认 `=`：仅仅以该名字开头的模板是在拼标识符（`format!("nichlink-auto-{}-{}", …)`），
        // 不是一条依赖要求。
        if after.starts_with('=') {
            found.push(format!("nichlink-{tail}"));
        }
    }
    let mut from = 0usize;
    while let Some(offset) = text[from..].find("package = \"") {
        let start = from + offset + "package = \"".len();
        let end = text[start..]
            .find('"')
            .map_or(text.len(), |end| start + end);
        let value = &text[start..end];
        if value.starts_with("nichlink-") {
            found.push(value.to_owned());
        }
        from = end;
    }
    found
}

/// `nichlink-…` names passed as `-p`/`--package` to a command in a workflow.
/// 工作流中作为 `-p`/`--package` 传给命令的 `nichlink-…` 名。
fn package_arguments(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for needle in ["-p ", "--package "] {
        let mut from = 0usize;
        while let Some(offset) = text[from..].find(needle) {
            let at = from + offset;
            from = at + needle.len();
            let boundary = at == 0
                || text[..at]
                    .chars()
                    .next_back()
                    .is_some_and(|previous| previous.is_whitespace() || previous == ':');
            let name: String = text[from..]
                .chars()
                .take_while(|character| {
                    character.is_ascii_alphanumeric() || *character == '-' || *character == '_'
                })
                .collect();
            if boundary && name.starts_with("nichlink-") {
                found.push(name);
            }
        }
    }
    found
}

/// The value of `key` inside `[section]`, with quotes and trailing comments removed.
/// `[section]` 中 `key` 的取值，已去掉引号与行尾注释。
fn manifest_value(text: &str, section: &str, key: &str) -> Option<String> {
    let mut current = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(header) = trimmed.strip_prefix('[') {
            current = header.trim_end_matches(']').trim().to_owned();
            continue;
        }
        if current != section || trimmed.starts_with('#') {
            continue;
        }
        let Some((left, right)) = trimmed.split_once('=') else {
            continue;
        };
        if left.trim() != key {
            continue;
        }
        let value = right.trim();
        let value = value.split('#').next().unwrap_or(value).trim();
        return Some(value.trim_matches(['"', '\'']).to_owned());
    }
    None
}

#[cfg(test)]
#[path = "naming_tests.rs"]
mod naming_tests;
