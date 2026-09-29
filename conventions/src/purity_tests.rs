//! The purity gate's tests, kept beside it in their own file so the gate itself
//! stays inside the line ceiling — the same split `doc_blocks_tests.rs` records.
//! purity 门禁的测试，放在它旁边的独立文件里，使门禁本体留在行数上限之内——与
//! `doc_blocks_tests.rs` 记录的同一种拆分。

use super::*;
use crate::workspace_root;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// A throwaway checkout with the given files under it.
/// 一个只含给定文件的一次性检出。
fn synthetic(files: &[(&str, &str)]) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-purity-{}-{}-{sequence}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    for (relative, contents) in files {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("fixture dir");
        fs::write(&path, contents).expect("fixture file");
    }
    crate::fixture_manifest(&root);
    root
}

/// A brace import names a forbidden module without ever spelling `std::fs`,
/// and `std::io` is I/O like everything else in the list.
/// 树形导入从未拼出 `std::fs` 却命名了被禁的模块，而 `std::io` 与表里其余各项一样是 I/O。
#[test]
fn a_brace_import_and_std_io_are_violations() {
    let root = synthetic(&[(
        "kernel/src/probe.rs",
        "use std::{env, fs};\n\npub fn probe() -> String {\n    \
         let _ = fs::read_to_string(\"/etc/hostname\");\n    \
         let _ = env::var(\"HOME\");\n    \
         let _ = std::io::stdout();\n    String::new()\n}\n",
    )]);
    let found = findings(&root);
    let tokens = found
        .iter()
        .map(|finding| finding.token)
        .collect::<BTreeSet<_>>();
    assert!(
        tokens.contains("std::io"),
        "std::io is I/O and belongs in the list: {found:#?}"
    );
    assert!(
        tokens.contains("std::env") && tokens.contains("std::fs"),
        "a brace import names both modules: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Prose that names a forbidden module is not a violation, and the code
/// beside it still is.
/// 点名被禁模块的散文不是违规，而它旁边的代码依然是。
#[test]
fn comments_and_strings_are_not_the_code_that_is_scanned() {
    let root = synthetic(&[(
        "kernel/src/probe.rs",
        "/* std::fs is banned here, and this block comment says so */\n\
         /// `let probe = \"std::env\";` is prose too.\n\
         pub fn probe() -> usize {\n    \
         let _note = \"std::thread::spawn\";\n    \
         let _ = std::fs::metadata(\"/tmp\");\n    1\n}\n",
    )]);
    let found = findings(&root);
    let lines = found.iter().map(|finding| finding.line).collect::<Vec<_>>();
    assert_eq!(
        lines,
        vec![5],
        "only the real call is a violation: a block comment, a doc comment and a \
         string literal are prose, and line 5 is the code beside them: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The four spellings a line-based scan was measured to miss: a brace import split
/// across lines, spaces around the separators, a path split across a newline, and a
/// compile-time environment read.
/// 逐行扫描实测漏掉的四种写法：跨行的树形导入、分隔符两旁的空格、被换行切开的路径，以及
/// 编译期读环境。
#[test]
fn the_ways_a_path_can_hide_from_a_line_scan_are_violations() {
    let root = synthetic(&[(
        "kernel/src/probe.rs",
        "use std::{\n    env,\n    fs,\n};\n\n\
         pub fn probe() {\n    \
         let _ = std :: fs :: metadata(\"/tmp\");\n    \
         let _ = std::\n        env::var(\"HOME\");\n    \
         let _ = option_env!(\"HOME\");\n}\n",
    )]);
    let found = findings(&root);
    let tokens = found
        .iter()
        .map(|finding| finding.token)
        .collect::<BTreeSet<_>>();
    assert!(
        tokens.contains("std::fs"),
        "spaces around the separators: {found:#?}"
    );
    assert!(
        tokens.contains("std::env"),
        "a split path and a brace import: {found:#?}"
    );
    assert!(
        tokens.contains("option_env!"),
        "a compile-time env read: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// An aliased `std` is the same capability, and the alias is found wherever it is
/// declared in the file.
/// 别名化的 `std` 是同一种能力，而别名在文件里任何位置声明都能被找到。
#[test]
fn an_aliased_std_is_still_std() {
    let root = synthetic(&[(
        "kernel/src/probe.rs",
        "use std as s;\n\npub fn probe() {\n    let _ = s::fs::metadata(\"/tmp\");\n}\n",
    )]);
    let found = findings(&root);
    assert!(
        found.iter().any(|finding| finding.token == "std::fs"),
        "`s::fs` is `std::fs` under an alias: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The walk has a floor, so a renamed kernel directory cannot read as "clean".
/// 遍历有一个下限，因此内核目录一旦改名就不会读作"干净"。
#[test]
fn the_walk_covers_the_kernel_tree() {
    let sources = rust_sources(&workspace_root().join("kernel").join("src"));
    assert!(
        sources.len() > 40,
        "the purity walk found only {} files; it is supposed to cover kernel/src",
        sources.len()
    );
}

/// The kernel stays pure: the audit verdict is now a gate.
/// 内核保持纯净：审计结论现在是一道门禁。
#[test]
fn the_kernel_does_no_io_and_reads_no_environment() {
    let root = workspace_root();
    let found = findings(&root);
    assert!(
        found.is_empty(),
        "kernel purity is a documented promise; move the value in as a parameter instead: {found:#?}"
    );
}

/// The idiomatic import spelling is a violation. Deleting every whitespace character
/// used to glue `use std::fs;` into `usestd::fs;`, and the left-boundary test then
/// read the `e` of `use` as part of a longer name.
/// 最惯用的 import 拼写就是违规。过去删掉每个空白字符会把 `use std::fs;` 粘成
/// `usestd::fs;`，左边界测试于是把 `use` 的 `e` 读成更长名字的一部分。
#[test]
fn a_plain_import_is_a_violation() {
    let root = synthetic(&[(
        "kernel/src/probe.rs",
        "use std::fs;\n\npub fn probe() {\n    let _ = fs::read(\"/etc/hostname\");\n}\n",
    )]);
    let found = findings(&root);
    assert!(
        found.iter().any(|finding| finding.token == "std::fs"),
        "`use std::fs;` is the same capability as `std::fs::read`: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The other spellings of the same capability: `use ::std as s;`,
/// `use std::{self as s};`, `extern crate std as s;`, a glob import, and a nested
/// brace group.
/// 同一种能力的其余拼写：`use ::std as s;`、`use std::{self as s};`、
/// `extern crate std as s;`、glob 导入，以及嵌套花括号组。
#[test]
fn alias_glob_and_nested_brace_spellings_are_violations() {
    let cases = [
        (
            "use ::std as s;",
            "use ::std as s;\npub fn probe() { let _ = s::fs::read(\"x\"); }\n",
        ),
        (
            "use std::{self as s};",
            "use std::{self as s};\npub fn probe() { let _ = s::fs::read(\"x\"); }\n",
        ),
        (
            "extern crate std as s;",
            "extern crate std as s;\npub fn probe() { let _ = s::fs::read(\"x\"); }\n",
        ),
        (
            "use std::*;",
            "use std::*;\npub fn probe() { let _ = fs::read(\"x\"); }\n",
        ),
        (
            "use std::{{fs}, env};",
            "use std::{{fs}, env};\npub fn probe() { let _ = fs::read(\"x\"); }\n",
        ),
    ];
    for (name, source) in cases {
        let root = synthetic(&[("kernel/src/probe.rs", source)]);
        let found = findings(&root);
        assert!(!found.is_empty(), "`{name}` is a violation: {found:#?}");
        let _ = fs::remove_dir_all(&root);
    }
}

/// A name that merely ends in a forbidden path is not a capability, and a bare
/// module name is only suspicious where a glob import made it reachable.
/// 只是以被禁路径结尾的名字不是一种能力，而裸模块名只有在 glob 导入让它可达的地方才可疑。
#[test]
fn lookalike_names_are_not_violations() {
    let root = synthetic(&[(
        "kernel/src/probe.rs",
        "pub fn probe() {\n    \
         let _ = my_env!(\"HOME\");\n    \
         let _ = reth::fs::read(\"x\");\n    \
         let _ = fs::read(\"x\");\n}\n",
    )]);
    assert!(
        findings(&root).is_empty(),
        "without a glob import, a local `fs` module is not std::fs"
    );
    let _ = fs::remove_dir_all(&root);
}

/// `#[cfg(any())]` means "never compiled", and the ordinary spelling puts the attribute and
/// the item it governs at the *same* indentation — only the item's body is deeper. The
/// exemption blanked the deeper lines alone, so a top-level attribute above a top-level
/// `fn` left that `fn` in the text and the most orthodox spelling was the one it reported.
/// `#[cfg(any())]` 意味着"永不编译"，而最普通的拼法把属性与它管辖的条目放在**同一缩进**上——
/// 只有条目的主体更深。豁免过去只抹更深的行，于是一个顶层属性放在顶层 `fn` 之上时，那个 `fn` 留在
/// 文本里，而它报出来的正是最正统的那种拼法。
#[test]
fn a_never_compiled_item_at_the_same_indentation_is_exempt() {
    let exempt = synthetic(&[(
        "kernel/src/probe.rs",
        "#[cfg(any())]\nfn never() {\n    let _ = std::fs::read(\"/etc/hostname\");\n}\n",
    )]);
    assert!(
        findings(&exempt).is_empty(),
        "the exemption exists for exactly this spelling"
    );
    let _ = fs::remove_dir_all(&exempt);

    // The exemption is for the item below the attribute, not for the file: code that
    // follows it is still read.
    // 豁免针对的是属性下面那个条目，而不是整个文件：它之后的代码仍会被读。
    let live = synthetic(&[(
        "kernel/src/probe.rs",
        "#[cfg(any())]\nfn never() {}\n\nfn live() {\n    let _ = std::fs::read(\"x\");\n}\n",
    )]);
    assert!(
        !findings(&live).is_empty(),
        "a violation outside the attribute is still reported"
    );
    let _ = fs::remove_dir_all(&live);
}
