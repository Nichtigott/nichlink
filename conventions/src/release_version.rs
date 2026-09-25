//! The release version has exactly one source, and every requirement names it.
//! 发布版本只有一个来源，而每一处要求都命名它。
//!
//! The tag check and the pre-upload table check both used to read the *first*
//! `version = "…"` line of the root manifest, while every member inherits
//! `[workspace.package]`. With a centralized requirement above that section the two
//! values can disagree: a mutant measured in this round named `0.1.0` above the
//! section and `0.1.1` inside it, so `v0.1.0` passed the tag check, `--check-table`
//! reported agreement, `cargo publish` uploaded `0.1.1`, and the index wait failed
//! only after the irreversible upload. The tools now read the section, and this gate
//! keeps the invariant that makes the read unambiguous: one source, inherited by
//! every member, required at the workspace version everywhere.
//! tag 检查与上传前的表检查过去都读根清单**第一行** `version = "…"`，而每个成员继承
//! `[workspace.package]`。当该节之上还有一处集中写的依赖要求时，两个值就能不一致：本轮实测的
//! 变异在节之上命名 `0.1.0`、节内命名 `0.1.1`，于是 `v0.1.0` 通过 tag 检查、`--check-table`
//! 报告一致、`cargo publish` 上传 `0.1.1`，而 index 等待要到**不可逆上传之后**才失败。工具现在
//! 读该节，本门禁守住让这次读取无歧义的那条不变量：一个来源、每个成员继承、处处要求工作区版本。

use std::fs;
use std::path::Path;

use crate::{relative, workspace_members};

/// One manifest whose version bookkeeping disagrees with the release line.
/// 一份版本记账与发布线不一致的清单。
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    /// Manifest, relative to the workspace root.
    /// 清单，以工作区根为基准。
    pub manifest: String,
    /// What disagrees, with both values.
    /// 什么不一致，以及两个值。
    pub reason: String,
}

/// Every version-bookkeeping disagreement in the workspace, sorted.
/// 工作区里每一处版本记账不一致，已排序。
pub fn findings(root: &Path) -> Vec<Finding> {
    let mut found = Vec::new();
    let release = workspace_version(root);
    let Some(release) = release else {
        found.push(Finding {
            manifest: "Cargo.toml".to_owned(),
            reason: "[workspace.package] names no version".to_owned(),
        });
        return found;
    };
    let root_manifest = root.join("Cargo.toml");
    let root_text = fs::read_to_string(&root_manifest)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", root_manifest.display()));
    // Exactly one source. A `version = "…"` before the section is what made the old
    // first-line read name something other than the release.
    // 只有一个来源。位于该节之前的 `version = "…"` 正是旧的第一行读取会命名非发布版本的原因。
    let mut section = String::new();
    let mut seen_release_section = false;
    for (index, line) in root_text.lines().enumerate() {
        let trimmed = line.split('#').next().unwrap_or("").trim();
        if let Some(header) = trimmed.strip_prefix('[') {
            section = header.trim_end_matches(']').trim().to_owned();
            if section == "workspace.package" {
                seen_release_section = true;
            }
            continue;
        }
        if section == "workspace.package" || !trimmed.starts_with("version") {
            continue;
        }
        if trimmed.starts_with("version =") && !seen_release_section {
            found.push(Finding {
                manifest: "Cargo.toml".to_owned(),
                reason: format!(
                    "line {}: a `version = \"…\"` before `[workspace.package]` is not the \
                     release version, and reading the first one made it look like it was",
                    index + 1
                ),
            });
        }
    }
    for member in workspace_members(root) {
        let manifest = root.join(&member).join("Cargo.toml");
        let text = fs::read_to_string(&manifest)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", manifest.display()));
        let published = publishes(&text);
        // A member may not name its own version: inheritance is what makes "every
        // member is the release" true by construction rather than by comparison. Only
        // published members are held to it — an example that is `publish = false` has no
        // release line to drift from.
        // 成员不得自己命名版本：继承才让"每个成员都是发布版本"由构造而非比较成立。只有已发布的成员
        // 受此约束——`publish = false` 的示例没有可漂移的发布线。
        if published {
            let mut section = String::new();
            for line in text.lines() {
                let trimmed = line.split('#').next().unwrap_or("").trim();
                if let Some(header) = trimmed.strip_prefix('[') {
                    section = header.trim_end_matches(']').trim().to_owned();
                    continue;
                }
                if section != "package" || !trimmed.starts_with("version") {
                    continue;
                }
                if trimmed.starts_with("version.workspace") {
                    continue;
                }
                if trimmed.starts_with("version =") {
                    found.push(Finding {
                        manifest: relative(root, &manifest),
                        reason: "`[package] version` is a literal; it has to inherit \
                                 `[workspace.package]` (`version.workspace = true`) so the \
                                 release line has one source"
                            .to_owned(),
                    });
                }
            }
        }
        if !published {
            // A `publish = false` member may depend on a path with no version: nothing
            // resolves it from a registry.
            // `publish = false` 的成员可以依赖没有版本的路径：没有任何东西会从 registry 解析它。
            continue;
        }
        found.extend(requirement_findings(
            root,
            &manifest,
            &flatten_inline_tables(&text),
            &release,
        ));
    }
    found.sort();
    found.dedup();
    found
}

/// The release version, read from `[workspace.package]` only.
/// 发布版本，只从 `[workspace.package]` 读取。
pub fn workspace_version(root: &Path) -> Option<String> {
    let manifest = root.join("Cargo.toml");
    let text = fs::read_to_string(&manifest).ok()?;
    manifest_value(&text, "workspace.package", "version")
}

/// The value of `key` inside `[section]`, quotes and trailing comments removed.
/// `[section]` 中 `key` 的取值，已去掉引号与行尾注释。
fn manifest_value(text: &str, want: &str, key: &str) -> Option<String> {
    let mut section = String::new();
    for line in text.lines() {
        let trimmed = line.split('#').next().unwrap_or("").trim();
        if let Some(header) = trimmed.strip_prefix('[') {
            section = header.trim_end_matches(']').trim().to_owned();
            continue;
        }
        if section != want {
            continue;
        }
        let Some((left, right)) = trimmed.split_once('=') else {
            continue;
        };
        if left.trim() != key {
            continue;
        }
        let value = right.trim();
        return Some(value.trim_matches(['"', '\'']).to_owned());
    }
    None
}

/// Whether the manifest publishes its package.
/// 该清单是否发布它的包。
fn publishes(text: &str) -> bool {
    let mut section = String::new();
    for line in text.lines() {
        let trimmed = line.split('#').next().unwrap_or("").trim();
        if let Some(header) = trimmed.strip_prefix('[') {
            section = header.trim_end_matches(']').trim().to_owned();
            continue;
        }
        if section == "package" && trimmed.starts_with("publish") && trimmed.contains("false") {
            return false;
        }
    }
    true
}

/// Put every inline table on one line, so a requirement written across lines reads
/// like the single-line spelling it means.
/// 把每个 inline table 放到一行，使跨行书写的依赖要求读起来就是它本意的单行写法。
///
/// Section headers use `[`, so a `{` in a manifest only ever opens an inline table.
/// 段落头用的是 `[`，因此清单里的 `{` 只会打开一个 inline table。
fn flatten_inline_tables(text: &str) -> String {
    let mut flattened = String::with_capacity(text.len());
    let mut depth = 0usize;
    for character in text.chars() {
        match character {
            '{' => {
                depth += 1;
                flattened.push(character);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                flattened.push(character);
            }
            '\n' if depth > 0 => flattened.push(' '),
            _ => flattened.push(character),
        }
    }
    flattened
}

/// The requirement problems in one published manifest.
/// 一份已发布清单里的依赖要求问题。
fn requirement_findings(
    root: &Path,
    manifest: &Path,
    flattened: &str,
    release: &str,
) -> Vec<Finding> {
    let mut found = Vec::new();
    let mut section = String::new();
    let mut dotted: Option<String> = None;
    for line in flattened.lines() {
        let trimmed = line.split('#').next().unwrap_or("").trim();
        if let Some(header) = trimmed.strip_prefix('[') {
            let header = header.trim_end_matches(']').trim();
            dotted = dotted_requirement(header);
            section = header.to_owned();
            continue;
        }
        // A dotted requirement carries its version on a line of its own inside the
        // section it opened.
        // 点表形式的依赖把版本写在自己打开的那一节里的独立一行上。
        if let Some(name) = &dotted {
            if trimmed.starts_with("version") {
                record(&mut found, root, manifest, name, trimmed, release);
            }
            continue;
        }
        if !is_requirement_section(&section) {
            continue;
        }
        let Some((name, value)) = trimmed.split_once('=') else {
            continue;
        };
        let name = name.trim();
        if !name.starts_with("nichlink-") || name.contains('.') {
            continue;
        }
        record(&mut found, root, manifest, name, value.trim(), release);
    }
    found
}

/// Add a finding when a requirement's version is missing or wrong.
/// 当依赖要求的版本缺失或不对时记一条。
fn record(
    found: &mut Vec<Finding>,
    root: &Path,
    manifest: &Path,
    name: &str,
    value: &str,
    release: &str,
) {
    match version_in(value) {
        Some(version) if version == release => {}
        Some(version) => found.push(Finding {
            manifest: relative(root, manifest),
            reason: format!("`{name}` requires {version}, but the workspace version is {release}"),
        }),
        None => found.push(Finding {
            manifest: relative(root, manifest),
            reason: format!(
                "`{name}` has no `version`; a published package cannot be resolved from a \
                 registry without one"
            ),
        }),
    }
}

/// The version a requirement spells, in either the bare-string or the inline-table
/// form.
/// 依赖要求写下的版本，裸字符串与 inline table 两种形式都算。
fn version_in(value: &str) -> Option<String> {
    if let Some(version) = quoted(value.trim()) {
        return Some(version);
    }
    let (_, after) = value.split_once("version")?;
    let (_, after) = after.split_once('=')?;
    quoted(after.trim_start())
}

/// The contents of the leading quoted string in `text`, if it starts with one.
/// `text` 以引号开头时其内容。
fn quoted(text: &str) -> Option<String> {
    let quote = text.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let rest = &text[quote.len_utf8()..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_owned())
}

/// The dependency a dotted section header names, if it names an internal one.
/// 点表段落头命名的依赖（若是内部依赖）。
fn dotted_requirement(header: &str) -> Option<String> {
    let tail = header.rsplit('.').next()?;
    if header.contains("dependencies") && tail.starts_with("nichlink-") {
        Some(tail.to_owned())
    } else {
        None
    }
}
/// Whether a section holds dependency requirements.
/// 该段落是否容纳依赖要求。
fn is_requirement_section(section: &str) -> bool {
    section == "dependencies"
        || section == "build-dependencies"
        || section == "dev-dependencies"
        || section == "workspace.dependencies"
        || (section.starts_with("target.") && section.ends_with(".dependencies"))
}

#[cfg(test)]
#[path = "release_version_tests.rs"]
mod release_version_tests;
