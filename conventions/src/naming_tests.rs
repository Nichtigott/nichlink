//! Tests for the crate, directory, and library naming gate.
//! crate、目录与 library 命名门禁的测试。

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

/// A scaffold template or a CI step that names a package the manifests do not
/// define is reported: a rename that misses one of them fails somewhere else.
/// 脚手架模板或 CI 步骤指名清单里不存在的包会被报出：漏改其中一处的重命名会在别处失败。
#[test]
fn a_reference_to_a_package_that_does_not_exist_is_reported() {
    let root = synthetic(&[(
        "thing",
        "[package]\nname = \"nichlink-thing\"\n\n[lib]\nname = \"nichlink_thing\"\n",
    )]);
    fs::create_dir_all(root.join("build_method/src/scaffold")).expect("scaffold dir");
    fs::write(
            root.join("build_method/src/scaffold/probe.rs"),
            "let requirement = \"nichlink-missing = { path = \\\"../missing\\\", version = \\\"0.1.0\\\" }\";\n",
        )
        .expect("template file");
    fs::create_dir_all(root.join(".github/workflows")).expect("workflow dir");
    fs::write(
        root.join(".github/workflows/ci.yml"),
        "run: cargo test -p nichlink-gone --offline\n# prose about `-p nichlink-thing`\n",
    )
    .expect("workflow file");

    let found = findings(&root);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(
        found
            .iter()
            .any(|finding| finding.reason.contains("nichlink-missing")),
        "{found:#?}"
    );
    assert!(
        found
            .iter()
            .any(|finding| finding.reason.contains("nichlink-gone")),
        "{found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A dependency key is recognised however it is spaced, and a name that is
/// not a requirement is left alone.
/// 依赖键无论怎么加空格都能被认出，而不是要求位置的名称不受过问。
#[test]
fn only_requirement_positions_are_read() {
    let names = requirement_names(
        "nichlink-a = { version = \"0.1\" }\nnichlink-b={ version = \"0.1\" }\n\
             let id = format!(\"nichlink-{label}-1\");\nprose about nichlink-c in a sentence\n\
             [dependencies]\nnichlink-d = { package = \"nichlink-e\", path = \"../d\" }\n",
    );
    assert_eq!(
        names,
        vec![
            "nichlink-a".to_owned(),
            "nichlink-b".to_owned(),
            "nichlink-d".to_owned(),
            "nichlink-e".to_owned(),
        ],
        "{names:#?}"
    );
    assert_eq!(
        package_arguments("cargo test -p nichlink-x --offline\nrun-a-pipeline\n"),
        vec!["nichlink-x".to_owned()]
    );
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
