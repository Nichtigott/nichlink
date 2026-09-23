//! The file-size gate, as a ratchet rather than a wish.
//! 文件尺寸门禁，以棘轮而不是愿望的形式。
//!
//! `docs/roadmap-1.0.md` recorded the size pass as done and stated that non-test
//! source files stay at or under 450 lines. The fourth audit round measured the
//! tree and found fifteen files over it, the largest at 751 lines. A ceiling
//! nothing measures is not a ceiling, so this module measures it and pins the
//! current exceptions explicitly.
//! `docs/roadmap-1.0.md` 把尺寸收口记为完成，并声明非测试源码文件不超过 450 行。第四轮
//! 审计实测源码树，发现 15 个文件超过它，最大 751 行。没有东西去量的上限不是上限，因此本
//! 模块去量它，并把当前的例外显式钉住。
//!
//! The list holds fourteen today: moving the shared JSON encoder out of
//! `diagnostic/build.rs` brought that file back under the ceiling, and the
//! ratchet made removing its entry mandatory rather than optional.
//! 这份清单今天有 14 项：把共用的 JSON 编码器移出 `diagnostic/build.rs` 让该文件缩回上限
//! 之内，而棘轮让删除对应项成为必然而不是可选。
//!
//! The ratchet has two teeth, and both matter. A newly oversized file fails the
//! gate, and a baseline entry that has shrunk back under the ceiling also fails
//! it, because a stale exception list is how a ratchet quietly becomes
//! permission. The list can therefore only shrink, and shrinking it is a
//! one-line change with a visible diff.
//! 这道棘轮有两齿，两齿都重要。新增超标文件会让门禁失败；某个基线项已缩回上限之内同样会
//! 让门禁失败，因为过期的例外清单正是一道棘轮悄悄变成许可证的方式。因此这份清单只能变短，
//! 而缩短它是一行改动，diff 可见。
//!
//! Boundary: test-only files are exempt, because the ceiling is about the code a
//! maintainer reads to understand behaviour. A file counts as test-only when it
//! sits under a `tests/` directory or its name is `test`/`tests`, starts with
//! `test_`, or ends with `_tests`.
//! 边界：仅测试文件豁免，因为上限针对的是维护者为了理解行为而要读的代码。文件在 `tests/`
//! 目录下，或其名字是 `test`/`tests`、以 `test_` 开头、或以 `_tests` 结尾时，算仅测试文件。

use std::path::Path;

use crate::{crate_directories, lines, relative, rust_sources};

/// The documented ceiling, in lines, for a non-test source file.
/// 文档声明的非测试源码文件行数上限。
pub const CEILING: usize = 450;

/// Files that were already over [`CEILING`] when the gate was added.
/// 门禁加入时就已经超过 [`CEILING`] 的文件。
///
/// Each entry is a debt with a measured size, not a permission. Remove an entry
/// the moment the file comes back under the ceiling; the gate fails on a stale
/// entry precisely so that removal cannot be forgotten.
/// 每一项都是带实测大小的欠账，不是许可。文件缩回上限之内的那一刻就删掉对应项；门禁会在
/// 过期项上失败，正是为了让"忘记删除"不可能发生。
pub const BASELINE: &[(&str, usize)] = &[
    ("core/src/registry_core/declaration/runtime_checks.rs", 751),
    ("core/src/registry_core/plugin/contracts/contracts.rs", 645),
    ("build_method/src/entry.rs", 572),
    ("core/src/registry_core/tree/connector/connector.rs", 506),
    ("core/src/registry_core/syntax/entries/graft.rs", 503),
    ("studio/src/studio/app/graft.rs", 499),
    ("core/src/registry_core/syntax/face.rs", 484),
    ("build_method/src/scope.rs", 480),
    ("core/src/registry_core/declaration/registration.rs", 465),
    ("build_method/src/graft_plan_check.rs", 465),
    ("core/src/registry_core/tree/graft_ops/overlay.rs", 463),
    ("run_method/src/authoring/operations/operations.rs", 459),
    ("run_method/src/runtime/trace/frames/frames.rs", 452),
    ("run_method/src/authoring/external_graft/plan.rs", 452),
];

/// Whether a path is a test-only source file.
/// 该路径是否为仅测试源码文件。
pub fn is_test_file(path: &Path) -> bool {
    if path
        .components()
        .any(|component| component.as_os_str() == "tests")
    {
        return true;
    }
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    stem == "test" || stem == "tests" || stem.starts_with("test_") || stem.ends_with("_tests")
}

/// Every non-test source file over [`CEILING`], as `(relative path, lines)`.
/// 每个超过 [`CEILING`] 的非测试源码文件，形式为 `(相对路径, 行数)`。
pub fn oversized(root: &Path) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    for directory in crate_directories(root) {
        for path in rust_sources(&directory.join("src")) {
            if is_test_file(&path) {
                continue;
            }
            let count = lines(&path).len();
            if count > CEILING {
                found.push((relative(root, &path), count));
            }
        }
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_root;

    /// The measured set of oversized files equals the pinned debt exactly.
    /// 实测的超标文件集合与钉住的欠账完全一致。
    #[test]
    fn the_size_ceiling_holds_except_for_the_pinned_debt() {
        let root = workspace_root();
        let actual = oversized(&root);
        let mut expected: Vec<(String, usize)> = BASELINE
            .iter()
            .map(|(path, count)| ((*path).to_owned(), *count))
            .collect();
        expected.sort();
        let new: Vec<_> = actual
            .iter()
            .filter(|entry| !expected.contains(entry))
            .collect();
        let stale: Vec<_> = expected
            .iter()
            .filter(|entry| !actual.contains(entry))
            .collect();
        assert!(
            new.is_empty(),
            "new files over the {CEILING}-line ceiling; split them: {new:#?}"
        );
        assert!(
            stale.is_empty(),
            "these entries are stale or their file changed size; update BASELINE \
             (an entry that shrank back under the ceiling must be removed): {stale:#?}"
        );
    }

    /// Test-only files stay exempt, and the exemption is name-shaped.
    /// 仅测试文件保持豁免，且该豁免是按名字判定的。
    #[test]
    fn test_only_files_are_recognised() {
        assert!(is_test_file(Path::new("cli/src/lib_tests.rs")));
        assert!(is_test_file(Path::new("core/src/tree/record_tests.rs")));
        assert!(is_test_file(Path::new("studio/tests/graph.rs")));
        assert!(!is_test_file(Path::new(
            "core/src/registry_core/syntax/face.rs"
        )));
        assert!(!is_test_file(Path::new("studio/src/studio/app/graft.rs")));
    }
}
