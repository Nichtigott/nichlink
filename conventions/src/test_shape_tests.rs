//! Tests for the test-file shape gate.
//! 测试文件形状门禁的测试。

use super::*;
use crate::workspace_root;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// A throwaway workspace with one member, `probe`, holding the given files.
/// 一个只含一个成员 `probe` 的一次性工作区，成员下含给定文件。
fn synthetic(files: &[(&str, &str)]) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("nichlink-shape-{}-{sequence}", std::process::id()));
    fs::create_dir_all(&root).expect("fixture root");
    for (relative, contents) in files {
        let path = root.join("probe").join(relative);
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture dir");
        fs::write(&path, contents).expect("fixture file");
    }
    crate::fixture_manifest(&root);
    root
}

/// The rule holds on the tree this repository ships: all four families — the sibling
/// `<name>_tests.rs` files, the studio app's `tests.rs` plus its `tests/` directory, the
/// `call_evidence/tests/` integration module, and the crate-root cargo `tests/` targets —
/// are accepted, and nothing else is.
/// 这条规则在本仓库出厂的树上成立：四族全部——同级的 `<name>_tests.rs`、studio app 的
/// `tests.rs` 加它的 `tests/` 目录、`call_evidence/tests/` 集成模块、以及 crate 根的 cargo
/// `tests/` 目标——都被接受，此外没有别的。
#[test]
fn the_shipped_tree_keeps_every_test_in_a_sanctioned_shape() {
    let root = workspace_root();
    let found = violations(&root);
    assert!(
        found.is_empty(),
        "these test files sit in a fourth shape; move them to `<name>_tests.rs`, a module's \
         `tests.rs`, or a `tests/` directory: {found:#?}"
    );
}

/// A test file in any shape other than the sanctioned three is reported by path. This is the
/// tooth: a file named `checks.rs` — or `probe_test.rs` with the singular suffix — mounted
/// behind `#[cfg(test)]` and holding a `#[test]` is exactly what the rule refuses.
/// 任何不在受认可三种形状之内的测试文件都会被点名报出。这就是那颗牙：一个名为 `checks.rs`
/// ——或单数后缀的 `probe_test.rs`——挂在 `#[cfg(test)]` 之后并持有 `#[test]` 的文件，正是这条
/// 规则拒绝的东西。
#[test]
fn a_fourth_shape_is_reported_by_path() {
    for name in ["checks.rs", "probe_test.rs", "test_probe.rs"] {
        let root = synthetic(&[
            (
                "src/lib.rs",
                &format!("#[cfg(test)]\n#[path = \"{name}\"]\nmod support;\n"),
            ),
            (&format!("src/{name}"), "#[test]\nfn probe() {}\n"),
        ]);
        let found = violations(&root);
        assert_eq!(
            found,
            vec![format!("probe/src/{name}")],
            "a test file in a fourth shape is named by path"
        );
        let _ = fs::remove_dir_all(&root);
    }
}

/// A test-only file with no test item of its own is support code, not a misplaced test. The
/// shipped tree's `kernel/src/registry_core/tree/graft_ops/fixtures.rs` is that case, and the
/// rule is about where tests live rather than about which files a test build compiles.
/// 自身不持有测试条目的仅测试文件是支撑代码，而不是放错位置的测试。出厂树的
/// `kernel/src/registry_core/tree/graft_ops/fixtures.rs` 就是这种情形，而这条规则管的是测试住在
/// 哪里，不是测试构建会编译哪些文件。
#[test]
fn a_test_only_support_file_without_tests_is_not_judged() {
    let root = synthetic(&[
        (
            "src/lib.rs",
            "#[cfg(test)]\n#[path = \"checks.rs\"]\nmod checks;\n",
        ),
        ("src/checks.rs", "pub fn helper() -> u32 { 7 }\n"),
    ]);
    assert_eq!(
        violations(&root),
        Vec::<String>::new(),
        "a fixture module is not a test file"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The sibling spelling is accepted: a single-file module's tests are `<name>_tests.rs` beside
/// it, mounted behind `#[cfg(test)]` — and a crate-root `src/tests.rs` is the same convention
/// one level up.
/// 同级拼法被接受：单文件模块的测试是它旁边的 `<name>_tests.rs`，挂在 `#[cfg(test)]` 之后
/// ——而 crate 根的 `src/tests.rs` 是同一约定往上提了一层。
#[test]
fn a_sibling_test_file_is_accepted() {
    let root = synthetic(&[
        ("src/lib.rs", "pub mod probe;\n#[cfg(test)]\nmod tests;\n"),
        (
            "src/probe.rs",
            "pub fn probe() {}\n#[cfg(test)]\n#[path = \"probe_tests.rs\"]\nmod probe_tests;\n",
        ),
        ("src/probe_tests.rs", "#[test]\nfn probe() {}\n"),
        ("src/tests.rs", "#[test]\nfn bare_mod_tests() {}\n"),
    ]);
    assert_eq!(
        violations(&root),
        Vec::<String>::new(),
        "`<name>_tests.rs` and a crate-root `tests.rs` are both sanctioned"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A directory module's tests live in its `tests.rs` and under its `tests/` directory, and the
/// `tests/` directory is accepted wherever it appears — including the crate root, which is how
/// the cargo integration targets are laid out.
/// 目录模块的测试住在它的 `tests.rs` 与 `tests/` 目录下，而 `tests/` 目录出现在哪里都被接受
/// ——包括 crate 根，cargo 集成目标正是这样布局的。
#[test]
fn tests_rs_and_a_tests_directory_are_accepted() {
    let root = synthetic(&[
        ("src/lib.rs", "#[path = \"app/app.rs\"]\npub mod app;\n"),
        (
            "src/app/app.rs",
            "#[cfg(test)]\n#[path = \"tests.rs\"]\npub mod tests;\n",
        ),
        (
            "src/app/tests.rs",
            "#[path = \"tests/call_tree.rs\"]\nmod call_tree;\n#[test]\nfn app_root() {}\n",
        ),
        ("src/app/tests/call_tree.rs", "#[test]\nfn tree() {}\n"),
        ("tests/registry.rs", "#[test]\nfn integration() {}\n"),
    ]);
    assert_eq!(
        violations(&root),
        Vec::<String>::new(),
        "a directory module's `tests.rs` and `tests/` are sanctioned"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A `tests.rs` whose parent is not a module directory is a fourth shape. The module
/// directory test is what distinguishes it from the sanctioned spelling: `<dir>/<dir>.rs`
/// makes `<dir>` a module, and a `tests.rs` in a directory that has no such root file is a
/// test file at an address this rule does not recognise.
/// 父目录不是模块目录的 `tests.rs` 是第四种形状。区分它与受认可拼法的正是"目录本身是不是模块"
/// 这一判据：`<dir>/<dir>.rs` 让 `<dir>` 成为一个模块，而一个没有这种根文件的目录里的
/// `tests.rs`，是本规则不认识的地址上的测试文件。
#[test]
fn a_tests_rs_outside_a_module_directory_is_reported() {
    let root = synthetic(&[
        (
            "src/lib.rs",
            "#[cfg(test)]\n#[path = \"plain/tests.rs\"]\nmod tests;\n",
        ),
        ("src/plain/tests.rs", "#[test]\nfn stray() {}\n"),
    ]);
    assert_eq!(
        violations(&root),
        vec!["probe/src/plain/tests.rs".to_owned()],
        "`plain/` is not a module directory, so `plain/tests.rs` is not its test half"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The `tests/` component is read below the member root, not from the checkout's own address:
/// a checkout that happens to live under a directory named `tests` must not turn every file
/// into a sanctioned test file.
/// `tests/` 组件是从成员根**之下**读的，而不是从检出自身的地址读的：一个恰好住在名为 `tests`
/// 的目录下的检出，不得把每个文件都变成受认可的测试文件。
#[test]
fn the_tests_component_is_read_below_the_member() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    // The checkout itself lives under a directory named `tests`, which a whole-path reader
    // would mistake for the sanctioned component.
    // 检出本身住在名为 `tests` 的目录之下——"读整条路径"的实现会把这个祖先目录误认成受认可的
    // 组件。
    let root = std::env::temp_dir()
        .join("nichlink-shape-tests")
        .join(format!("{}-{sequence}", std::process::id()));
    fs::create_dir_all(root.join("probe/src")).expect("fixture src");
    fs::write(root.join("probe/src/lib.rs"), "#[cfg(test)]\nmod plain;\n").expect("fixture lib");
    fs::write(root.join("probe/src/plain.rs"), "#[test]\nfn plain() {}\n").expect("fixture file");
    crate::fixture_manifest(&root);

    let member = root.join("probe");
    assert!(
        !is_legal_test_file(&member, &member.join("src/plain.rs")),
        "`plain.rs` has no sanctioned test shape; an ancestor `tests` directory is not proof"
    );
    assert!(
        is_legal_test_file(&member, &member.join("src/anything_tests.rs")),
        "the sibling spelling is the shape"
    );
    let _ = fs::remove_dir_all(&root);
}
