//! Where a test file is allowed to live, as a gate rather than a habit.
//! 测试文件允许住在哪里，以门禁而不是习惯的形式。
//!
//! One rule, two mechanical cases. A **directory module** keeps its tests in
//! `<dir>/tests.rs` or under `<dir>/tests/`; a **single-file module** keeps them in the
//! sibling `<name>_tests.rs`. Those three positions are the sanctioned shapes, and a
//! test-only file in any *fourth* shape is reported by path. Prose alone could not hold
//! this: nothing walked the tree, so a test file dropped at a new address compiled and ran
//! while the convention it belonged to was quietly ignored — the same failure mode the
//! size gate closed for measurement.
//! 一条规则、两种机械情形。**目录模块**把测试放在 `<dir>/tests.rs` 或 `<dir>/tests/` 之下；
//! **单文件模块**把测试放在同级的 `<name>_tests.rs`。这三个位置就是受认可的形状，而处于任何
//! **第四种**形状的仅测试文件会被点名报出。只靠散文守不住这条：没有任何程序遍历这棵树，因此一个
//! 被放到新地址的测试文件照样编译、照样运行，而它本应遵守的约定被静默忽略——与尺寸门禁为度量堵上
//! 的是同一种失效。
//!
//! Which files are judged: the ones the tree *says* are test-only — mounted behind
//! `#[cfg(test)]`, directly or through an ancestor (`crate::size::is_mounted_as_test`), or
//! living under a `tests/` directory — **and** carrying at least one `#[test]` item. The two
//! halves matter separately. Mount-based testhood is what lets a file whose *name* is not
//! test-shaped be caught at all; and the "has a test item" half is what keeps a test *support*
//! module out of the judgement, because this gate is about where tests go, not about every
//! file a test build happens to compile. `kernel/src/registry_core/tree/graft_ops/fixtures.rs`
//! is that case in the shipped tree: test-only, non-test-shaped, and holding no test of its own.
//! 判定哪些文件：树自己**说**是仅测试的那些——挂在 `#[cfg(test)]` 之后（直接挂或经由祖先挂，
//! 见 `crate::size::is_mounted_as_test`），或位于 `tests/` 目录之下——**并且**至少带一个
//! `#[test]` 条目。这两半各有各的必要性。"按挂载判定仅测试"让**名字**看起来不像测试的文件也能被
//! 抓到；而"带有测试条目"这一半把测试**支撑**模块挡在判定之外，因为本门禁管的是测试放在哪里，
//! 不是测试构建恰好编译到的每一个文件。出厂树里 `kernel/src/registry_core/tree/graft_ops/fixtures.rs`
//! 就是这种情形：仅测试、名字不像测试、且自身不持有任何测试。

use std::path::Path;

use crate::{crate_directories, relative, rust_sources};

/// Every test file that sits in an unsanctioned shape, as relative paths, sorted.
/// 每个处于不受认可形状的测试文件，形式为相对路径，已排序。
///
/// The paths are what a maintainer edits, and the rule they break is the one in this
/// module's own documentation: `<name>_tests.rs`, `tests.rs` in a directory module, or
/// anything under a `tests/` directory.
/// 返回的是维护者要改的路径，而它们违反的规则就在本模块自己的文档里：`<name>_tests.rs`、目录模块
/// 里的 `tests.rs`，或 `tests/` 目录下的任何文件。
pub fn violations(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for directory in crate_directories(root) {
        for path in rust_sources(&directory) {
            // The cheap half first: resolving a mount walks the crate's sources, and a file
            // with no test item in it cannot be a misplaced test whatever declares it.
            // 先做便宜的那一半：解析挂载会遍历 crate 的源码，而一个不含测试条目的文件无论由谁声明
            // 都不可能是放错位置的测试。
            if !has_test_item(&path) {
                continue;
            }
            if !is_test_only(&directory, &path) {
                continue;
            }
            if !is_legal_test_file(&directory, &path) {
                found.push(relative(root, &path));
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

/// Whether a test file sits in one of the sanctioned shapes.
/// 某个测试文件是否处于受认可的形状之一。
///
/// `crate_root` is the member directory the file belongs to, so the `tests/` component is
/// looked for in the path *below* the member rather than in the checkout's own address: a
/// checkout that happened to live under a directory named `tests` must not turn every file
/// into a test file.
/// `crate_root` 是该文件所属的成员目录，因此 `tests/` 组件是在成员**之下**的路径里找，而不是在
/// 检出自身的地址里找：一个恰好住在名为 `tests` 的目录下的检出，不得把每个文件都变成测试文件。
pub fn is_legal_test_file(crate_root: &Path, path: &Path) -> bool {
    let below_member = path.strip_prefix(crate_root).unwrap_or(path);
    if below_member
        .components()
        .any(|component| component.as_os_str() == "tests")
    {
        return true;
    }
    if path.file_name().and_then(|name| name.to_str()) == Some("tests.rs") {
        return parent_is_a_module_directory(path);
    }
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem.ends_with("_tests"))
}

/// Whether `path` is a whole-file test module: mounted behind `#[cfg(test)]`, or under a
/// `tests/` directory.
/// `path` 是否是整文件的测试模块：挂在 `#[cfg(test)]` 之后，或位于 `tests/` 目录之下。
fn is_test_only(crate_root: &Path, path: &Path) -> bool {
    let below_member = path.strip_prefix(crate_root).unwrap_or(path);
    below_member
        .components()
        .any(|component| component.as_os_str() == "tests")
        || crate::size::is_mounted_as_test(crate_root, path)
}

/// Whether a file declares at least one test item.
/// 文件是否至少声明了一个测试条目。
///
/// The search runs on [`nichlink_kernel::source::mask_non_code`], so a `#[test]` written
/// inside a doc comment, a string literal or a raw string is not a test item — the shipped
/// tree has all three, and reading them as tests would report a support file as a
/// misplaced test.
/// 搜索跑在 [`nichlink_kernel::source::mask_non_code`] 上，因此写在文档注释、字符串字面量或原始
/// 字符串里的 `#[test]` 不是测试条目——出厂树里这三种都有，把它们读成测试会把一个支撑文件报成
/// 放错位置的测试。
fn has_test_item(path: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return false;
    };
    let masked = nichlink_kernel::source::mask_non_code(&text);
    masked
        .lines()
        .any(|line| line.trim_start().starts_with("#[test]"))
}

/// Whether a `tests.rs` file's parent directory is a module in its own right.
/// `tests.rs` 的父目录本身是否是一个模块。
///
/// A directory module is written as `<dir>/<dir>.rs` plus its children, so `<dir>/tests.rs`
/// is that module's test half. The crate source root (`src`) counts too, because a bare
/// `mod tests;` there resolves to `src/tests.rs` and is the same convention one level up.
/// 目录模块写成 `<dir>/<dir>.rs` 加上它的子模块，因此 `<dir>/tests.rs` 就是这个模块的测试半边。
/// crate 源码根（`src`）也算，因为那里的裸 `mod tests;` 解析到的就是 `src/tests.rs`，是同一约定
/// 往上提了一层。
fn parent_is_a_module_directory(path: &Path) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };
    if parent.file_name().and_then(|name| name.to_str()) == Some("src") {
        return true;
    }
    let Some(directory) = parent.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    parent.join(format!("{directory}.rs")).is_file()
}

#[cfg(test)]
#[path = "test_shape_tests.rs"]
mod test_shape_tests;
