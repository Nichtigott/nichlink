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
