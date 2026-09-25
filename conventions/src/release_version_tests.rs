//! Tests for the release-version gate.
//! 发布版本门禁的测试。

use super::*;

/// A throwaway workspace whose root manifest is given verbatim.
/// 一个一次性工作区，其根清单按原文给出。
fn synthetic(root_manifest: &str, members: &[(&str, &str)]) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-release-version-{}-{sequence}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("fixture root");
    fs::write(root.join("Cargo.toml"), root_manifest).expect("fixture root manifest");
    for (name, manifest) in members {
        let directory = root.join(name);
        fs::create_dir_all(directory.join("src")).expect("fixture src");
        fs::write(directory.join("Cargo.toml"), manifest).expect("fixture manifest");
        fs::write(directory.join("src/lib.rs"), "\n").expect("fixture lib");
    }
    root
}

/// The mutant this gate exists for: a centralized requirement above
/// `[workspace.package]` whose version is the stale one.
/// 本门禁为之存在的变异：位于 `[workspace.package]` 之上、版本已陈旧的集中依赖要求。
#[test]
fn a_version_line_before_the_release_section_is_reported() {
    let root = synthetic(
        "[workspace]\nmembers = [\"core\"]\n\n\
         [workspace.dependencies.nichlink-core]\npath = \"core\"\nversion = \"0.1.0\"\n\n\
         [workspace.package]\nversion = \"0.1.1\"\n",
        &[(
            "core",
            "[package]\nname = \"nichlink-core\"\nversion.workspace = true\n",
        )],
    );
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].reason.contains("before `[workspace.package]`"),
        "{found:#?}"
    );
    // The section read still finds the real release version.
    assert_eq!(workspace_version(&root).as_deref(), Some("0.1.1"));
    let _ = fs::remove_dir_all(&root);
}

/// A member that names its own version can drift from the release line.
/// 自己命名版本的成员会与发布线漂移。
#[test]
fn a_member_with_its_own_version_is_reported() {
    let root = synthetic(
        "[workspace]\nmembers = [\"core\"]\n\n[workspace.package]\nversion = \"0.1.1\"\n",
        &[(
            "core",
            "[package]\nname = \"nichlink-core\"\nversion = \"0.1.0\"\n",
        )],
    );
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].reason.contains("inherit"), "{found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// A requirement written as a multi-line inline table is read like the
/// single-line spelling it means.
/// 写成跨行 inline table 的依赖要求，会像它本意的单行写法一样被读到。
#[test]
fn a_multi_line_requirement_with_a_wrong_version_is_reported() {
    let root = synthetic(
        "[workspace]\nmembers = [\"macro\"]\n\n[workspace.package]\nversion = \"0.1.1\"\n",
        &[(
            "macro",
            "[package]\nname = \"nichlink-macro\"\nversion.workspace = true\n\n\
             [dependencies]\nnichlink-core = {\n    path = \"../core\",\n    \
             version = \"0.2.0\",\n    features = [\"syntax\"],\n}\n",
        )],
    );
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].reason.contains("requires 0.2.0"), "{found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// A published package that depends on a path with no version cannot be
/// resolved from a registry.
/// 已发布的包若依赖没有版本的路径，就无法从 registry 解析。
#[test]
fn a_published_requirement_without_a_version_is_reported() {
    let root = synthetic(
        "[workspace]\nmembers = [\"cli\"]\n\n[workspace.package]\nversion = \"0.1.1\"\n",
        &[(
            "cli",
            "[package]\nname = \"nichlink-cli\"\nversion.workspace = true\n\n\
             [dependencies]\nnichlink-core = { path = \"../core\" }\n",
        )],
    );
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].reason.contains("no `version`"), "{found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// A `publish = false` member may depend on a path with no version.
/// `publish = false` 的成员可以依赖没有版本的路径。
#[test]
fn an_unpublished_member_may_omit_the_version() {
    let root = synthetic(
        "[workspace]\nmembers = [\"demo\"]\n\n[workspace.package]\nversion = \"0.1.1\"\n",
        &[(
            "demo",
            "[package]\nname = \"demo\"\nversion.workspace = true\npublish = false\n\n\
             [dependencies]\nnichlink-core = { path = \"../core\" }\n",
        )],
    );
    let found = findings(&root);
    assert!(
        found.is_empty(),
        "an unpublished member may inherit and omit the requirement version: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The workspace's own manifests agree.
/// 工作区自己的清单是一致的。
#[test]
fn the_shipped_manifests_name_one_version() {
    let found = findings(&crate::workspace_root());
    assert!(
        found.is_empty(),
        "the release line has to have one source and one value everywhere: {found:#?}"
    );
}
