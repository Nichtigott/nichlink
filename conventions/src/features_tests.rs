//! Tests for the feature and target manifest gate.
//! 特性与 target 清单门禁的测试。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

/// The manifest a well-shaped workspace has: the workspace-only binary carries the
/// feature that keeps it out of an installed copy, and the fixture feature is opt-in.
/// 形状正确的工作区的清单：只在检出内工作的二进制带着把它挡在安装副本之外的特性，而 fixture
/// 特性需要显式打开。
const WELL_SHAPED: &str = "\
[package]
name = \"xirang-toolchain\"
version = \"0.1.6\"

[features]
default = [\"node-graph\"]
node-graph = []
prototype-fixtures = []
dev-supervisor = []

[[bin]]
name = \"xirang-dev\"
path = \"src/bin/xirang-dev.rs\"
required-features = [\"dev-supervisor\"]
";

/// A throwaway workspace whose single member carries `manifest`.
/// 一个一次性工作区，其唯一成员带着 `manifest`。
fn workspace(manifest: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("xirang-features-{}-{sequence}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("toolchain/src")).expect("fixture directory");
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"toolchain\"]\nresolver = \"2\"\n",
    )
    .expect("root manifest");
    fs::write(root.join("toolchain/Cargo.toml"), manifest).expect("member manifest");
    root
}

/// The manifests this checkout ships satisfy the rules.
/// 本检出出厂的清单满足这些规则。
#[test]
fn the_shipped_manifests_satisfy_the_feature_rules() {
    let found = findings(&crate::workspace_root());
    assert!(found.is_empty(), "a shipped manifest regressed: {found:#?}");
}

/// The shape this gate exists for: `required-features` deleted from a target that only
/// works inside this checkout. Every other gate stayed silent on exactly that edit.
/// 本门禁为之存在的形状：从一个只在本检出内工作的 target 上删掉 `required-features`。正是那次
/// 编辑让其余门禁全部沉默。
#[test]
fn a_missing_requirement_on_a_workspace_only_target_is_reported() {
    let without = WELL_SHAPED.replace("required-features = [\"dev-supervisor\"]\n", "");
    assert!(
        !without.contains("required-features"),
        "the fixture drops the line"
    );
    let root = workspace(&without);
    let found = findings(&root);
    assert!(
        found
            .iter()
            .any(|finding| finding.contains("`xirang-dev` must carry")),
        "a deleted `required-features` is reported: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A requirement that a typo, or a default feature, made decorative is reported too: the
/// first keeps nothing out because the feature does not exist, the second because Cargo
/// turns it on anyway.
/// 被拼写错误或"默认特性"变成装饰的要求同样被报出：前者因为特性不存在而什么都挡不住，后者因为
/// Cargo 无论如何都会打开它。
#[test]
fn a_decorative_requirement_is_reported() {
    let undeclared = WELL_SHAPED.replace("[\"dev-supervisor\"]", "[\"dev-superviser\"]");
    let root = workspace(&undeclared);
    let found = findings(&root);
    assert!(
        found
            .iter()
            .any(|finding| finding.contains("which the manifest does not declare")),
        "a requirement naming no feature is reported: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);

    let defaulted = WELL_SHAPED.replace("[\"dev-supervisor\"]", "[\"node-graph\"]");
    let root = workspace(&defaulted);
    let found = findings(&root);
    assert!(
        found
            .iter()
            .any(|finding| finding.contains("requires default feature")),
        "a requirement on a default feature is reported: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A feature that gates test-only fixtures must stay off by default: turning it on ships
/// the fixture host inside the published package.
/// 门控测试用 fixture 的特性必须保持默认关闭：打开它会把 fixture 宿主一起发进已发布包。
#[test]
fn a_fixture_feature_in_default_is_reported() {
    let turned_on = WELL_SHAPED.replace(
        "default = [\"node-graph\"]",
        "default = [\"node-graph\", \"prototype-fixtures\"]",
    );
    let root = workspace(&turned_on);
    let found = findings(&root);
    assert!(
        found
            .iter()
            .any(|finding| finding.contains("must stay off by default")),
        "a fixture feature on by default is reported: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A wrapped `default` array is read as one value, so the gate cannot be defeated by
/// reformatting the manifest.
/// 被折行的 `default` 数组作为一个取值读取，因此重新排版清单绕不过门禁。
#[test]
fn a_wrapped_array_is_read_whole() {
    let wrapped = WELL_SHAPED.replace(
        "default = [\"node-graph\"]",
        "default = [\n    \"node-graph\",\n    \"prototype-fixtures\",\n]",
    );
    let root = workspace(&wrapped);
    let found = findings(&root);
    assert!(
        found
            .iter()
            .any(|finding| finding.contains("must stay off by default")),
        "a wrapped array is not a hiding place: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}
