//! Tests for the portable rendering of a recorded source path, and for the
//! search predicate built on it.
//! 记录源码路径的可移植渲染，以及建立在它之上的搜索谓词的测试。

use super::*;

/// A path recorded on Windows renders with `/`, and one that is already
/// portable comes back byte for byte.
/// 在 Windows 上记录的路径以 `/` 渲染，而已可移植的路径逐字节原样返回。
///
/// `file!()` keeps the host separator and a declaration cannot rewrite it at
/// compile time without allocating, so this fold is what lets every surface that
/// displays a source agree on one spelling: `SourceLocation`'s `Display` and
/// `describe`, and the graft-plan check that embeds a plan path in its report.
/// `file!()` 保留宿主分隔符，而声明在编译期无法在不分配的前提下改写它，因此这次折叠
/// 正是让每个显示源码的面在一种拼法上取得一致的东西：`SourceLocation` 的 `Display`
/// 与 `describe`，以及把计划路径写进报告的计划检查。
#[test]
fn a_recorded_path_renders_portably_on_every_platform() {
    assert_eq!(
        portable_path(r"control\object\button\button.rs"),
        "control/object/button/button.rs"
    );
    assert_eq!(
        portable_path("control/object/button/button.rs"),
        "control/object/button/button.rs"
    );
    // The absolute form is folded too: only the separator changes.
    // 绝对路径同样被折叠：改变的只有分隔符。
    assert_eq!(
        portable_path(r"C:\proj\src\control\control.rs"),
        "C:/proj/src/control/control.rs"
    );
    // An empty path has no separator to fold and stays empty rather than
    // becoming a root.
    // 空路径没有分隔符可折叠，保持为空而不会变成一个根。
    assert_eq!(portable_path(""), "");
}

/// A search for a source reaches a Windows-recorded path through the `/`
/// spelling the same surface displays.
/// 对源码的搜索能通过同一个面所显示的 `/` 拼法，命中在 Windows 上记录的路径。
///
/// Without the fold the user can read `control/widget.rs` on screen, type it into
/// the search box, and get nothing — the value being searched is
/// `control\widget.rs`. `[实测]`: the Windows CI job failed on exactly this
/// split, where `REGISTRATION.source.file` compared unequal to its `/` spelling.
/// 缺了折叠，用户可以在屏幕上读到 `control/widget.rs`、把它敲进搜索框、然后一无所获——
/// 被搜索的值是 `control\widget.rs`。`[实测]`：Windows CI 任务正是栽在这个分裂上，
/// 当时 `REGISTRATION.source.file` 与它的 `/` 拼法比较不相等。
#[test]
fn a_query_with_slashes_matches_a_backslashed_source() {
    assert!(source_file_matches(
        r"run_method\tests\external_compact_face.rs",
        "run_method/tests/external"
    ));
    // A caller that searched with the raw value's spelling keeps working, and
    // case is still folded.
    // 按原始值拼法搜索的调用方继续可用，大小写依然被折叠。
    assert!(source_file_matches(
        r"run_method\tests\external_compact_face.rs",
        "external_compact"
    ));
    assert!(source_file_matches(
        r"Run_Method\Tests\External.rs",
        "tests/external"
    ));
    // The fold is a separator fold, not a substring rescue: an unrelated path
    // still does not match.
    // 这次折叠是分隔符折叠而不是子串救场：无关路径依然不匹配。
    assert!(!source_file_matches(
        r"run_method\tests\external.rs",
        "studio/tests"
    ));
    // A POSIX-recorded path is unaffected by the fold's presence.
    // POSIX 记录的路径不受折叠是否存在的影响。
    assert!(source_file_matches(
        "run_method/tests/external.rs",
        "tests/external"
    ));
}
