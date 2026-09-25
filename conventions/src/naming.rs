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

use crate::{crate_directories, relative};

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
    for directory in crate_directories(root) {
        let relative_directory = relative(root, &directory);
        // The example hosts are a family of their own; see the module docs.
        // 示例宿主自成一个家族；见模块文档。
        if relative_directory.starts_with("examples/") {
            continue;
        }
        let Some(name) = directory.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let manifest = directory.join("Cargo.toml");
        let text = fs::read_to_string(&manifest)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", manifest.display()));
        let expected_package = format!("nichlink-{}", name.replace('_', "-"));
        match manifest_value(&text, "package", "name") {
            Some(actual) if actual == expected_package => {}
            Some(actual) => found.push(Finding {
                directory: relative_directory.clone(),
                reason: format!("package name is `{actual}`, not `{expected_package}`"),
            }),
            None => found.push(Finding {
                directory: relative_directory.clone(),
                reason: "no `[package] name` in Cargo.toml".to_owned(),
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
        match manifest_value(&text, "lib", "name") {
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
    found.sort();
    found.dedup();
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
        return Some(value.trim_matches('"').to_owned());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway checkout with the given members.
    /// 一个只含给定成员的一次性检出。
    fn synthetic(members: &[(&str, &str)]) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("nichlink-naming-{}-{sequence}", std::process::id()));
        for (directory, manifest) in members {
            let base = root.join(directory);
            fs::create_dir_all(base.join("src")).expect("fixture src");
            fs::write(base.join("Cargo.toml"), manifest).expect("fixture manifest");
            fs::write(base.join("src/lib.rs"), "\n").expect("fixture lib");
        }
        crate::fixture_manifest(&root);
        root
    }

    /// A member whose package name does not name its directory is reported.
    /// 包名不指涉其目录的成员会被报出。
    #[test]
    fn a_package_name_that_disagrees_with_the_directory_is_reported() {
        let root = synthetic(&[(
            "thing",
            "[package]\nname = \"nichlink-other\"\n\n[lib]\nname = \"nichlink_thing\"\n",
        )]);
        let found = findings(&root);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert!(found[0].reason.contains("nichlink-other"), "{found:#?}");
        let _ = fs::remove_dir_all(&root);
    }

    /// A lib name that is neither the rule nor the documented exception is
    /// reported, because it is the name hosts write.
    /// 既不符合规则、也不是文档化例外的 lib 名会被报出，因为它正是宿主书写的名字。
    #[test]
    fn a_lib_name_that_disagrees_is_reported() {
        let root = synthetic(&[(
            "thing",
            "[package]\nname = \"nichlink-thing\"\n\n[lib]\nname = \"thing\"\n",
        )]);
        let found = findings(&root);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert!(found[0].reason.contains("lib name"), "{found:#?}");
        let _ = fs::remove_dir_all(&root);
    }

    /// A hyphenated directory maps to an underscored lib name, because a Rust
    /// library name cannot carry a hyphen. This gate reported the workspace's own
    /// `plugin-host` before the fold existed.
    /// 带连字符的目录映射为下划线的 library 名，因为 Rust 的 library 名不能带连字符。在这次折叠
    /// 出现之前，本门禁曾把工作区自己的 `plugin-host` 报出来。
    #[test]
    fn a_hyphenated_directory_maps_to_an_underscored_lib_name() {
        let root = synthetic(&[(
            "plugin-host",
            "[package]\nname = \"nichlink-plugin-host\"\n\n[lib]\nname = \"nichlink_plugin_host\"\n",
        )]);
        assert_eq!(findings(&root), Vec::new());
        let _ = fs::remove_dir_all(&root);
    }

    /// The kernel's lib name is the one exception, and it stays accepted.
    /// 内核的 library 名是唯一的例外，且保持被接受。
    #[test]
    fn the_kernel_lib_name_is_the_documented_exception() {
        let root = synthetic(&[(
            "core",
            "[package]\nname = \"nichlink-core\"\n\n[lib]\nname = \"nichlink\"\n",
        )]);
        assert_eq!(findings(&root), Vec::new());
        let _ = fs::remove_dir_all(&root);
    }

    /// A member with no `[lib] name` is fine: cargo derives it from the package
    /// name, which is already tied to the directory.
    /// 没有 `[lib] name` 的成员没问题：cargo 从包名推导它，而包名已经被绑到目录名。
    #[test]
    fn a_derived_lib_name_is_accepted() {
        let root = synthetic(&[("thing", "[package]\nname = \"nichlink-thing\"\n")]);
        assert_eq!(findings(&root), Vec::new());
        let _ = fs::remove_dir_all(&root);
    }

    /// The example hosts are a naming family of their own and are left alone.
    /// 示例宿主自成一个命名家族，不被过问。
    #[test]
    fn an_example_host_is_outside_the_rule() {
        let root = synthetic(&[(
            "examples/control-button",
            "[package]\nname = \"nichlink-example-control-button\"\n\n[lib]\nname = \"control_button\"\n",
        )]);
        assert_eq!(findings(&root), Vec::new());
        let _ = fs::remove_dir_all(&root);
    }

    /// The workspace's own names agree.
    /// 工作区自己的名字是一致的。
    #[test]
    fn the_shipped_crates_follow_the_rule() {
        let found = findings(&crate::workspace_root());
        assert!(
            found.is_empty(),
            "crate, directory and lib names have to agree; a host writes all three: {found:#?}"
        );
    }
}
