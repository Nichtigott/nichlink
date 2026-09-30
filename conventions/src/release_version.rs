//! The release version has exactly one source, and every requirement names it.
//! 发布版本只有一个来源，而每一处要求都命名它。
//!
//! The requirements are read for **every** workspace member, including the ones that are
//! `publish = false`. A skip there was measured to be a silent hole rather than a boundary: a
//! stale `version = "0.1.5"` in an example host is skipped by this gate, by the publish table
//! check and by the package audit alike, so it drifts until the next version line makes it a
//! build failure somewhere else. What an unpublished member is free of is the *obligation* to
//! name a version at all (nothing resolves it from a registry) and the rule that `[package]
//! version` has to inherit — its own version is its own. What it is not free of is naming a
//! version: once it writes one, that version is on the release line like anyone's (audit
//! `G-05`).
//! 要求会为**每个**工作区成员读取，包括 `publish = false` 的那些。那里的跳过被实测证明是一个静默
//! 的洞而不是一条边界：示例宿主里一条陈旧的 `version = "0.1.5"` 会被本门禁、发布表检查与包审计
//! 一同跳过，于是一路漂移到下一次抬版本线，在别处以构建失败的形式现身。非发布成员真正豁免的是**必须
//! 写出**版本这件事（没有任何东西会从 registry 解析它）以及"`[package] version` 必须继承"这条规则
//! ——它自己的版本就是它自己的。它不豁免的是写下版本这件事：一旦写了，那个版本就与任何人的一样落在
//! 发布线上（审计 `G-05`）。
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
            // A `publish = false` member may e.g. carry its own `[package] version` and a
            // requirement with no version: nothing resolves it from a registry, so the two
            // rules above do not apply to it. Its *versioned* requirements do: a stale
            // `version = "0.1.5"` in an example host is the same drift as a stale one in a
            // published crate, and it is invisible everywhere else — this gate, the publish
            // table check and the package audit all skip `publish = false`, so the drift would
            // surface at the next version line and nowhere before it (audit `G-05`).
            // `publish = false` 成员可以自己写 `[package] version`、也可以依赖没有版本的路径：
            // 没有任何东西会从 registry 解析它，因此上面两条规则不适用于它。但它的**带版本**要求适用：
            // 示例宿主里一条陈旧的 `version = "0.1.5"` 与已发布 crate 里的一条是同样的漂移，而它在
            // 别处都不可见——本门禁、发布表检查与包审计都跳过 `publish = false`，因此漂移只会在下一次
            // 抬版本线时现身，之前无人报出（审计 `G-05`）。
            found.extend(requirement_findings(
                root,
                &manifest,
                &flatten_inline_tables(&text),
                &release,
                false,
            ));
            continue;
        }
        found.extend(requirement_findings(
            root,
            &manifest,
            &flatten_inline_tables(&text),
            &release,
            true,
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

/// The requirement problems in one member's manifest.
/// 一个成员清单里的依赖要求问题。
///
/// `require_version` is what distinguishes a published member from a `publish = false` one: a
/// published package with no requirement version cannot be resolved from a registry, while an
/// unpublished one is never resolved at all and may omit it. A version that *is* written is
/// held to the release line in both cases.
/// `require_version` 区分已发布成员与 `publish = false` 成员：已发布的包若要求上没有版本，就无法
/// 从 registry 解析；而未发布的包根本不会被解析，因此可以省略。**写出来的**版本在两种情况下都被绑到
/// 发布线上。
fn requirement_findings(
    root: &Path,
    manifest: &Path,
    flattened: &str,
    release: &str,
    require_version: bool,
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
                record(
                    &mut found,
                    root,
                    manifest,
                    name,
                    trimmed,
                    release,
                    require_version,
                );
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
        if name.contains('.') {
            continue;
        }
        // The package the requirement is really on: the key itself, or the name in a
        // `package = "…"` field when the dependency is renamed. Reading only keys that
        // start with `nichlink-` let `kernel = { package = "nichlink-kernel", version =
        // "0.0.9" }` name an internal dependency with a version nobody checked.
        // 这条要求真正指向的包：键本身，或依赖被重命名时 `package = "…"` 里的名字。只读以
        // `nichlink-` 开头的键，会让 `kernel = { package = "nichlink-kernel", version = "0.0.9" }`
        // 以没人检查过的版本点名一个内部依赖。
        let target = if name.starts_with("nichlink-") {
            name.to_owned()
        } else {
            match package_field(value.trim()) {
                Some(package) => package,
                None => continue,
            }
        };
        record(
            &mut found,
            root,
            manifest,
            &target,
            value.trim(),
            release,
            require_version,
        );
    }
    found
}

/// The `package = "…"` field's value in a dependency's value, when it names an internal one.
/// 依赖取值里 `package = "…"` 字段的值（若点名了一个内部依赖）。
fn package_field(value: &str) -> Option<String> {
    let at = value.find("package")?;
    let rest = value[at + "package".len()..].trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    let package = &rest[..end];
    package.starts_with("nichlink-").then(|| package.to_owned())
}

/// Add a finding when a requirement's version is missing or wrong.
/// 当依赖要求的版本缺失或不对时记一条。
///
/// `require_version` is what keeps a `publish = false` member legal while still holding the
/// version it *does* write to the release line — see [`requirement_findings`].
/// `require_version` 让 `publish = false` 成员保持合法，同时仍然把它**写出来的**版本绑到发布线上
/// ——见 [`requirement_findings`]。
fn record(
    found: &mut Vec<Finding>,
    root: &Path,
    manifest: &Path,
    name: &str,
    value: &str,
    release: &str,
    require_version: bool,
) {
    match version_in(value) {
        Some(version) if version == release => {}
        Some(version) => found.push(Finding {
            manifest: relative(root, manifest),
            reason: format!("`{name}` requires {version}, but the workspace version is {release}"),
        }),
        // An inherited requirement is legal **because the root names it**: the version then lives
        // in exactly one place, and the root's own entry is scanned as a requirement like any other
        // (this section is in `is_requirement_section`), so the value is still held to the release
        // line. Inheriting from a root that does not name the package is the finding.
        // 继承来的要求之所以合法，是因为**根点名了它**：版本因此只有一个家，而根里那条本身就是一条
        // 被扫描的要求（`workspace.dependencies` 在 `is_requirement_section` 里），取值仍被绑到发布线
        // 上。根没有点名这个包却要继承，才是那条发现。
        None if inherits_from_workspace(value) => {
            if !workspace_dependency_names(root, name) {
                found.push(Finding {
                    manifest: relative(root, manifest),
                    reason: format!(
                        "`{name}` inherits its version from `[workspace.dependencies]`, which \
                         does not name it"
                    ),
                });
            }
        }
        None if require_version => found.push(Finding {
            manifest: relative(root, manifest),
            reason: format!(
                "`{name}` has no `version`; a published package cannot be resolved from a \
                 registry without one"
            ),
        }),
        None => {}
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

/// Whether this requirement takes its version from `[workspace.dependencies]`.
/// 这条要求是否从 `[workspace.dependencies]` 取得版本。
fn inherits_from_workspace(value: &str) -> bool {
    let Some(at) = value.find("workspace") else {
        return false;
    };
    let rest = value[at + "workspace".len()..].trim_start();
    match rest.strip_prefix('=') {
        Some(rest) => rest.trim_start().starts_with("true"),
        None => false,
    }
}

/// Whether the root manifest's `[workspace.dependencies]` names this package.
/// 根清单的 `[workspace.dependencies]` 是否点名了这个包。
fn workspace_dependency_names(root: &Path, name: &str) -> bool {
    let Ok(text) = std::fs::read_to_string(root.join("Cargo.toml")) else {
        return false;
    };
    let mut in_table = false;
    for line in text.lines() {
        let trimmed = line.split('#').next().unwrap_or("").trim();
        if let Some(header) = trimmed.strip_prefix('[') {
            in_table = header.trim_end_matches(']').trim() == "workspace.dependencies";
            continue;
        }
        if !in_table {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let named = if key.starts_with("nichlink-") {
            Some(key.to_owned())
        } else {
            package_field(value.trim())
        };
        if named.as_deref() == Some(name) {
            return true;
        }
    }
    false
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
