//! The census is the whole-tree half of a `check`: static facts, each with the sentence that says
//! what it did not cover.
//! 总账是 `check` 的全树那一半：静态事实，每条都带着"它没覆盖什么"那一句。

use std::path::PathBuf;

use super::census;

/// A throwaway package with one respelled constant, one unreferenced constant, one production
/// function no test names, and one a test does name.
/// 一个一次性包：一个被重拼的常量、一个没人引用的常量、一个没有任何测试点名的生产函数，以及一个
/// 测试确实点名的函数。
fn package(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "nichlink-mcp-claims-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("fixture dirs");
    std::fs::create_dir_all(root.join("tests")).expect("fixture test dir");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"fixture-{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("fixture manifest");
    std::fs::write(
        root.join("src/lib.rs"),
        "//! A fixture.\n\
         pub const LIMIT: i64 = 1000;\n\
         pub const UNUSED: i64 = 7;\n\
         pub fn named_by_a_test() -> i64 { LIMIT }\n\
         pub fn no_test_names_me() -> i64 { 1 }\n",
    )
    .expect("fixture source");
    std::fs::write(
        root.join("src/other.rs"),
        "//! A second production file that spells the number again.\n\
         pub fn threshold() -> i64 { 1000 }\n",
    )
    .expect("fixture second source");
    std::fs::write(
        root.join("tests/one.rs"),
        "#[test]\nfn names_the_function() { assert_eq!(fixture_facts::named_by_a_test(), 1000); }\n",
    )
    .expect("fixture test");
    root
}

/// Every column is a static fact, and the closing sentence names the boundary.
/// 每一栏都是静态事实，而结尾那句点出边界。
#[test]
fn the_census_reports_static_facts_and_says_what_it_did_not_cover() {
    let root = package("facts");
    let lines = census(&root).expect("the census answers").join("\n");
    assert!(lines.contains("respelled 1000"), "{lines}");
    assert!(lines.contains("unreferenced `UNUSED`"), "{lines}");
    assert!(
        lines.contains("no test names `no_test_names_me`"),
        "{lines}"
    );
    assert!(
        !lines.contains("no test names `named_by_a_test`"),
        "a test names this one: {lines}"
    );
    assert!(lines.contains("entry plan"), "{lines}");
    assert!(lines.contains("not covered"), "{lines}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A throwaway package whose production side is one chain a test walks (`reachable_a` →
/// `reachable_b`) plus `unreachable` functions nothing reaches.
/// 一个一次性包：生产侧是一条测试走过的链（`reachable_a` → `reachable_b`），外加 `unreachable` 个
/// 没人到达的函数。
fn reachability_package(label: &str, unreachable: usize) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "nichlink-mcp-claims-reach-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("fixture dirs");
    std::fs::create_dir_all(root.join("tests")).expect("fixture test dir");
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"fixture-reach-{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
        ),
    )
    .expect("fixture manifest");
    let mut source = String::from(
        "//! A reachability fixture.\n\
         pub fn reachable_a() -> i64 { reachable_b() }\n\
         pub fn reachable_b() -> i64 { 1 }\n",
    );
    for index in 0..unreachable {
        source.push_str(&format!(
            "pub fn unreachable_{index}() -> i64 {{ {index} }}\n"
        ));
    }
    std::fs::write(root.join("src/lib.rs"), source).expect("fixture source");
    std::fs::write(
        root.join("tests/one.rs"),
        "#[test]\nfn walks_the_chain() { assert_eq!(fixture_reach::reachable_a(), 1); }\n",
    )
    .expect("fixture test");
    root
}

/// The pin: a test walks `reachable_a` → `reachable_b`, so the walk marks both and lists only the
/// function no test can reach.
/// 钉子：测试走过 `reachable_a` → `reachable_b`，因此遍历把两者都标为已到达，只列出没有测试能到达的
/// 那个函数。
#[test]
fn the_walk_lists_only_the_function_no_test_can_reach() {
    let root = reachability_package("chain", 1);
    let lines = census(&root).expect("the census answers").join("\n");
    assert!(lines.contains("no test reaches `unreachable_0`"), "{lines}");
    assert!(
        !lines.contains("no test reaches `reachable_a`"),
        "a test calls this one: {lines}"
    );
    assert!(
        !lines.contains("no test reaches `reachable_b`"),
        "a test call reaches this one through `reachable_a`: {lines}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The other pin: the boundary sentence itself is in the answer, with every invisibility it has to
/// admit, and the column never calls itself coverage.
/// 另一条钉子：边界那句本身就在答案里，带着它必须承认的每一种不可见，而且这一栏从不自称覆盖率。
#[test]
fn the_walk_states_its_own_boundary_in_the_answer() {
    let root = reachability_package("boundary", 1);
    let lines = census(&root).expect("the census answers").join("\n");
    assert!(lines.contains(super::REACHABILITY_BOUNDARY), "{lines}");
    for bound in [
        "dynamic dispatch",
        "function pointers",
        "macro expansion",
        "trait method",
        "closure",
        "not a coverage measurement",
    ] {
        assert!(
            super::REACHABILITY_BOUNDARY.contains(bound),
            "the boundary sentence has to name `{bound}`: {}",
            super::REACHABILITY_BOUNDARY
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The cap: five rows, then the one truncation sentence with what it withheld.
/// 上限：五行，然后是那句唯一的截断说明与被扣下的数量。
#[test]
fn the_walk_lists_five_rows_and_says_how_many_it_withheld() {
    let root = reachability_package("cap", 8);
    let lines = census(&root).expect("the census answers").join("\n");
    assert_eq!(
        lines.matches("no test reaches `").count(),
        5,
        "the cap is five rows: {lines}"
    );
    assert!(
        lines.contains("test-unreachable functions"),
        "the withheld count names what was counted: {lines}"
    );
    assert!(
        lines.contains(crate::mcp::truncation::PHRASE),
        "the cap goes through the one truncation outlet: {lines}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The budget is not decoration: a tree over it is reported as skipped with its own count instead
/// of being walked, and no reachability row is invented for it.
/// 阈值不是装饰：超过它的树会带着自身计数被报成跳过，而不是被硬走一遍，也不会为它编出任何可达性行。
#[test]
fn a_tree_over_the_function_budget_is_reported_as_skipped() {
    let root = reachability_package("over", super::REACHABILITY_BUDGET + 1);
    let lines = census(&root).expect("the census answers").join("\n");
    assert!(lines.contains("test-reachable: skipped ("), "{lines}");
    assert!(
        lines.contains(&format!("over the limit of {}", super::REACHABILITY_BUDGET)),
        "{lines}"
    );
    assert!(
        !lines.contains("no test reaches `"),
        "no rows are computed for a skipped tree: {lines}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
