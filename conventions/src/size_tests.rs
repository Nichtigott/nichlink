//! Tests for the file-size gate.
//! 文件尺寸门禁的测试。

use super::*;
use crate::workspace_root;
use std::fs;
use std::path::PathBuf;

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

/// A throwaway workspace with one member, `probe`, holding the given files.
/// 一个只含一个成员 `probe` 的一次性工作区，成员下含给定文件。
///
/// The manifest deliberately carries a `[workspace.package] name` right after the
/// members array: a reader that took that section for members would build a crate
/// directory out of a borrowed name.
/// 清单刻意在 members 数组之后紧跟一个 `[workspace.package] name`：把那一节当成成员的读者
/// 会用一个借来的名字造出一个 crate 目录。
fn synthetic(files: &[(&str, &str)]) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("nichlink-size-{}-{sequence}", std::process::id()));
    fs::create_dir_all(root.join("probe/src")).expect("fixture src");
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"probe\"]\nresolver = \"2\"\n\n\
         [workspace.package]\nname = \"borrowed\"\nedition = \"2024\"\n",
    )
    .expect("fixture manifest");
    for (relative, contents) in files {
        let path = root.join("probe").join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("fixture dir");
        fs::write(&path, contents).expect("fixture file");
    }
    root
}

/// Filler that pushes a fixture file past [`CEILING`].
/// 把夹具文件推过 [`CEILING`] 的填充内容。
fn over_ceiling() -> String {
    "// filler\n".repeat(CEILING + 10)
}

/// Location is not proof of testhood: a real module that merely sits under a `tests/`
/// directory inside `src/` was exempt from the ceiling while the same file elsewhere was
/// measured. The declaration that mounts it has to carry `#[cfg(test)]` — or inherit one
/// from an ancestor, which is how this tree's nested test modules are written.
/// 位置本身不是"仅测试"的证明：只是位于 `src/` 下某个 `tests/` 目录里的真实模块过去免于度量，
/// 而同一个文件换个位置就会被量。挂载它的声明必须带 `#[cfg(test)]`——或者从祖先继承一个，本树
/// 的嵌套测试模块正是这么写的。
#[test]
fn a_module_under_src_tests_needs_a_test_declaration_to_be_exempt() {
    let root = synthetic(&[
        ("src/lib.rs", "pub mod tests;\n"),
        (
            "src/tests.rs",
            "#[path = \"tests/zz_big.rs\"]\npub mod zz_big;\n",
        ),
        ("src/tests/zz_big.rs", &over_ceiling()),
    ]);
    assert!(
        oversized(&root)
            .iter()
            .any(|(path, _)| path.ends_with("tests/zz_big.rs")),
        "nothing says this file is a test, so the ceiling measures it"
    );
    let _ = fs::remove_dir_all(&root);

    let root = synthetic(&[
        ("src/lib.rs", "pub mod tests;\n"),
        (
            "src/tests.rs",
            "#[cfg(test)]\n#[path = \"tests/zz_big.rs\"]\npub mod zz_big;\n",
        ),
        ("src/tests/zz_big.rs", &over_ceiling()),
    ]);
    assert!(
        !oversized(&root)
            .iter()
            .any(|(path, _)| path.ends_with("tests/zz_big.rs")),
        "a `#[cfg(test)]` declaration makes it a test file"
    );
    let _ = fs::remove_dir_all(&root);

    let root = synthetic(&[
        ("src/lib.rs", "#[cfg(test)]\npub mod tests;\n"),
        (
            "src/tests.rs",
            "#[path = \"tests/zz_big.rs\"]\npub mod zz_big;\n",
        ),
        ("src/tests/zz_big.rs", &over_ceiling()),
    ]);
    assert!(
        !oversized(&root)
            .iter()
            .any(|(path, _)| path.ends_with("tests/zz_big.rs")),
        "the attribute is inherited down the mount chain"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The recursion bound is a cycle stop with headroom, not a description of this tree's depth.
/// The deepest chain the tree ships is six files (five mount edges): `kernel/src/lib.rs` →
/// `registry_core.rs` → `authoring/authoring.rs` → `authoring/parse/parse.rs` →
/// `authoring/parse/flow.rs` → `authoring/parse/flow_tests.rs`. An eleven-file chain below one
/// `#[cfg(test)]` mount must still be exempt, because a bound that merely fits today's tree
/// reclassifies a test file as measured source the next time someone nests two more modules.
/// 递归上限是带余量的防环，而不是对本树深度的描述。本树出厂的最深链有 6 个文件（5 条挂载边）：
/// `kernel/src/lib.rs` → `registry_core.rs` → `authoring/authoring.rs` → `authoring/parse/parse.rs`
/// → `authoring/parse/flow.rs` → `authoring/parse/flow_tests.rs`。一条挂在单个 `#[cfg(test)]`
/// 之下的十一个文件的链必须仍然豁免，因为"刚好装下今天的树"的上限会在下次有人再嵌两层模块时，
/// 把一个测试文件重新归类成被测源码。
#[test]
fn a_deep_mount_chain_is_still_test_only() {
    let mut files: Vec<(String, String)> = vec![(
        "src/lib.rs".to_owned(),
        "#[cfg(test)]\npub mod a;\n".to_owned(),
    )];
    for letter in b'a'..=b'j' {
        let name = (letter as char).to_string();
        let body = if letter == b'j' {
            over_ceiling()
        } else {
            let next = (letter + 1) as char;
            format!("#[path = \"{next}.rs\"]\npub mod {next};\n")
        };
        files.push((format!("src/{name}.rs"), body));
    }
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, body)| (path.as_str(), body.as_str()))
        .collect();
    let root = synthetic(&borrowed);
    assert!(
        !oversized(&root)
            .iter()
            .any(|(path, _)| path.ends_with("j.rs")),
        "a ten-level chain below a `#[cfg(test)]` mount is still a test file"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Test-only files are recognised by where they sit, or by their name together
/// with a `#[cfg(test)]` mount.
/// 仅测试文件按所在位置识别，或按名字加 `#[cfg(test)]` 挂载识别。
#[test]
fn test_only_files_are_recognised() {
    assert!(is_test_shaped(Path::new(
        "toolchain/src/cli/src/lib_tests.rs"
    )));
    assert!(is_test_shaped(Path::new(
        "kernel/src/registry_core/syntax/face_tests.rs"
    )));
    assert!(is_test_shaped(Path::new(
        "toolchain/src/studio/src/studio/app/tests.rs"
    )));
    assert!(!is_test_shaped(Path::new(
        "kernel/src/registry_core/syntax/face.rs"
    )));
    assert!(is_test_by_location(Path::new(
        "toolchain/src/studio/tests/graph.rs"
    )));
    assert!(!is_test_by_location(Path::new(
        "toolchain/src/studio/src/studio/app/tests.rs"
    )));
}

/// A name is not proof: real code renamed to `x_tests.rs` and never mounted is
/// measured like anything else, which is the hole this closed.
/// 名字不是证明：把真实代码改名成 `x_tests.rs` 却从不挂载，会像别的文件一样被度量——这正是
/// 被堵上的那个洞。
#[test]
fn an_unmounted_test_shaped_file_is_measured() {
    let root = synthetic(&[("src/probe_tests.rs", &over_ceiling())]);
    let found = oversized(&root);
    assert_eq!(
        found.len(),
        1,
        "an unmounted `_tests` file is source like any other: {found:#?}"
    );
    assert!(found[0].0.ends_with("src/probe_tests.rs"), "{found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// Mounted behind `#[cfg(test)]`, the same file is exempt — under both spellings
/// this tree uses.
/// 挂在 `#[cfg(test)]` 之后，同一个文件重新豁免——本树使用的两种拼法都算。
#[test]
fn a_mounted_test_file_is_exempt() {
    let root = synthetic(&[
        (
            "src/probe.rs",
            "#[cfg(test)]\n#[path = \"probe_tests.rs\"]\nmod probe_tests;\n",
        ),
        ("src/probe_tests.rs", &over_ceiling()),
    ]);
    assert_eq!(
        oversized(&root),
        Vec::new(),
        "a mounted `#[path]` test module is exempt"
    );
    let _ = fs::remove_dir_all(&root);

    let root = synthetic(&[
        ("src/tests.rs", &over_ceiling()),
        ("src/lib.rs", "#[cfg(test)]\nmod tests;\n"),
    ]);
    assert_eq!(
        oversized(&root),
        Vec::new(),
        "the bare `mod tests;` form is exempt too"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The crate-root `build.rs` is measured like `src/`.
/// crate 根的 `build.rs` 与 `src/` 一同被度量。
#[test]
fn the_crate_root_build_script_is_measured() {
    let root = synthetic(&[("build.rs", &over_ceiling())]);
    let found = oversized(&root);
    assert_eq!(
        found.len(),
        1,
        "an oversized build script is debt like any other: {found:#?}"
    );
    assert!(found[0].0.ends_with("build.rs"), "{found:#?}");
    let _ = fs::remove_dir_all(&root);
}

/// The ceiling is the maintainer's decision, not a drift target: a file at 500
/// lines is accepted and one past 600 is still debt. Pinned so the 2026-09-28
/// relaxation from 450 to 600 cannot be undone or exceeded silently.
/// 上限是维护者的决定，不是可以漂移的目标：500 行的文件被接受，超过 600 的仍是欠账。钉住它，
/// 使 2026-09-28 从 450 到 600 的放宽不会被静默撤销或被静默越界。
#[test]
fn the_ceiling_is_the_number_the_maintainer_set() {
    assert_eq!(
        CEILING, 600,
        "the ratchet's ceiling is the maintainer's decision of 2026-09-28"
    );
    let accepted = "// filler\n".repeat(500);
    let root = synthetic(&[("src/probe.rs", &accepted)]);
    assert_eq!(
        oversized(&root),
        Vec::new(),
        "500 lines sits under a 600-line ceiling"
    );
    let _ = fs::remove_dir_all(&root);

    let root = synthetic(&[("src/probe.rs", &over_ceiling())]);
    assert_eq!(
        oversized(&root).len(),
        1,
        "past the ceiling is still measured debt"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Every spelling rustc accepts for a `#[cfg(test)]` mount is the same mount, and the
/// ceiling has to read all of them. Comparing the declaration against two literal
/// strings read `#[cfg(all(test))]` and `pub(crate) mod x;` as *not* test mounts, so a
/// test file under either spelling was measured as source — a debt report the author
/// cannot act on, on a file the gate had already agreed is exempt under the orthodox
/// spelling. Audit `G-04`.
/// rustc 接受的每一种 `#[cfg(test)]` 挂载拼法都是同一个挂载，而上限必须都能读出来。把声明与
/// 两个字面串比较，会把 `#[cfg(all(test))]` 与 `pub(crate) mod x;` 读成**不是**测试挂载，于是
/// 这两种拼法下的测试文件被当作源码度量——在门禁已按正统拼法同意豁免的同一个文件上，报出一笔
/// 作者无法处理的欠账。审计 `G-04`。
#[test]
fn every_spelling_of_a_test_mount_is_exempt() {
    let filler = over_ceiling();
    let root = synthetic(&[
        (
            "src/lib.rs",
            "#[cfg(test)]\n#[path = \"big_tests.rs\"]\nmod big_tests;\n\
             #[cfg(all(test))]\n#[path = \"big2_tests.rs\"]\nmod big2_tests;\n\
             #[cfg(test)]\npub(crate) mod big3_tests;\n",
        ),
        ("src/big_tests.rs", &filler),
        ("src/big2_tests.rs", &filler),
        ("src/big3_tests.rs", &filler),
    ]);
    let found = oversized(&root);
    assert_eq!(
        found,
        Vec::new(),
        "all three spellings are the same mount: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A `#[cfg(not(test))]` mount is the opposite of a test mount, and `all`/`any` are
/// not interchangeable: `all(test, feature = "x")` is only compiled in a test build,
/// while `any(test, feature = "x")` is compiled in an ordinary one too.
/// `#[cfg(not(test))]` 挂载是测试挂载的反面，而 `all`/`any` 不可互换：
/// `all(test, feature = "x")` 只在测试构建里编译，而 `any(test, feature = "x")` 在普通构建里
/// 也会编译。
#[test]
fn a_negative_or_conditional_cfg_is_not_a_test_mount() {
    let filler = over_ceiling();
    for (spelling, exempt) in [
        ("#[cfg(all(test))]", true),
        ("#[cfg(all(test, feature = \"probe\"))]", true),
        ("#[cfg(test)]", true),
        ("#[cfg(not(test))]", false),
        ("#[cfg(any(test, feature = \"probe\"))]", false),
        ("#[cfg(feature = \"probe\")]", false),
    ] {
        let root = synthetic(&[
            ("src/lib.rs", &format!("{spelling}\nmod probe;\n")),
            ("src/probe.rs", &filler),
        ]);
        let found = oversized(&root);
        assert_eq!(
            found.len(),
            if exempt { 0 } else { 1 },
            "`{spelling}` must {} the ceiling: {found:#?}",
            if exempt { "exempt" } else { "not exempt" }
        );
        let _ = fs::remove_dir_all(&root);
    }
}

/// The ratchet measures the file rustfmt writes, not the bytes on disk, because those
/// two are not the same file and the one that is shorter is the one that hides debt.
/// rustfmt splits the long lines it is handed, so a file written with several statements
/// per line is *under* the ceiling as stored and *over* it as read: the B1 round measured
/// `mutations.rs` at 586 lines while rustfmt makes it 616.
/// 棘轮量的是 rustfmt 写出的文件，而不是磁盘上的字节，因为两者不是同一个文件，而更短的那个正是
/// 藏住欠账的那个。rustfmt 会切开交给它的长行，因此一行里塞了几条语句的文件在**存储形态**下位于
/// 上限之内、在**阅读形态**下越过上限：B1 轮实测 `mutations.rs` 是 586 行，而 rustfmt 会把它
/// 写成 616 行。
#[test]
fn an_unformatted_file_does_not_hide_the_ratchet() {
    // Statements packed onto one line, which is the shape rustfmt rewrites.
    // 挤在一行上的语句，正是 rustfmt 会重写的形状。
    let mut packed = String::from("fn probe() {\n    ");
    for index in 0..700 {
        packed.push_str(&format!("let v{index} = {index}; "));
    }
    packed.push_str("\n}\n");
    assert!(
        packed.lines().count() < CEILING,
        "the fixture is under the ceiling as written, which is what hides it"
    );
    let root = synthetic(&[("src/probe.rs", &packed)]);
    let found = oversized(&root);
    assert_eq!(
        found.len(),
        1,
        "the file rustfmt writes is over the ceiling: {found:#?}"
    );
    assert!(
        found[0].1 > CEILING,
        "the reported size is the formatted one: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The claim the comment above [`declares_module`] makes, held to the code.
/// [`declares_module`] 上面那句注释所做的断言，与实现对齐。
///
/// The branch exists for spellings rustc accepts, and the failure it guards is a *test* file
/// measured as source: `mod  x ;` did that, because the token join left the space before the `;`
/// in place (audit `G-07`, finding `F-2`). The pin lives here rather than in `size.rs` because a
/// test-only file is exempt from the ceiling this gate enforces — the assertion is about a
/// private function, and a sibling module sees it.
/// 这条分支是为 rustc 接受的拼法而存在的，而它守的失败是**测试**文件被当作源码度量：
/// `mod  x ;` 会这样，因为那次词元连接把 `;` 前的空格留在了原地（审计 `G-07`，发现 `F-2`）。
/// 钉子放在这里而不是 `size.rs`：仅测试文件不受本门禁执行的上限约束——断言的对象是私有函数，
/// 而兄弟模块能看见它。
#[test]
fn a_space_before_the_semicolon_is_the_same_declaration() {
    assert!(declares_module("mod x;", "mod x;"));
    assert!(declares_module("mod  x ;", "mod x;"));
    assert!(declares_module("pub(crate) mod  x ;", "mod x;"));
}

/// A longer identifier that shares the keyword's prefix is not a declaration.
/// 仅仅与关键字同前缀的更长标识符不是声明。
#[test]
fn a_longer_identifier_is_not_the_declaration() {
    assert!(!declares_module("modx;", "mod x;"));
    assert!(!declares_module("module x;", "mod x;"));
}
