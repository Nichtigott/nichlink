//! Which tests a set of changed files reaches — the question a reader asks before running
//! anything, answered from the same source index every other tool reads.
//! 一组改动过的文件能触到哪些测试——读者在"跑什么"之前要问的问题，用与其它工具同一份源码索引作答。
//!
//! The answer is static and says so: a file whose definitions a test calls is covered by that
//! test, and a file no test mentions is a file whose package suite is the smallest honest
//! answer. Nothing here decides that a test *fails*; it decides what to run, which is the part
//! that can be derived from text and the part an agent otherwise guesses at.
//! 这个答案是静态的，而且它自己说出来：测试调用了某个文件里的定义，那个文件就由那个测试覆盖；没有测试
//! 提到的文件，最小而诚实的答案就是它所在包的整套测试。这里不判定任何测试会**失败**；它判定的是跑什么，
//! 而这一半可以从文本推出，也是代理本来会靠猜的那一半。

use std::path::Path;

use serde_json::Value;

use crate::mcp::source_index::{SourceFile, display_list, load_sources};
use crate::mcp::truncation::withheld;

/// How many tests one changed file may name before the reply says how many it withheld.
/// 一个改动过的文件最多点名多少个测试，超出后回复要说出扣下了多少。
const TESTS_PER_FILE: usize = 10;

pub(crate) fn affected(root: &Path, arguments: &Value) -> Result<String, String> {
    let files = arguments
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "nichlink.affected requires `files`, an array of workspace paths".to_owned()
        })?
        .iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if files.is_empty() {
        return Err("`files` must name at least one path".to_owned());
    }
    // The index is loaded once per member a changed file lives in, and a member that publishes
    // no sources is skipped rather than reported as a package with no tests: the answer is a
    // list of things to run.
    // 每个"改动文件所属的成员"只加载一次索引；没有源码可读的成员被跳过，而不是报成一个没有测试的包：
    // 这个答案是一份"要跑什么"的清单。
    let mut output = String::from("evidence: static-heuristic\n");
    for path in &files {
        let (label_prefix, owner) = owner_of(root, path);
        let sources = match owner {
            Some(member) => load_sources(&member)?,
            None => load_sources(root)?,
        };
        let touched = sources.iter().find(|file| {
            file.relative == *path || format!("{label_prefix}{}", file.relative) == *path
        });
        let Some(touched) = touched else {
            output.push_str(&format!("{path}: not in the index\n"));
            continue;
        };
        let names = touched
            .functions
            .iter()
            .map(|function| function.name.clone())
            .collect::<Vec<_>>();
        let mut tests = sources
            .iter()
            .filter(|candidate| is_test_file(candidate))
            .filter(|candidate| {
                candidate.functions.iter().any(|caller| {
                    caller.calls.iter().any(|call| {
                        names
                            .iter()
                            .any(|name| call == name || call.ends_with(&format!("::{name}")))
                    })
                })
            })
            .map(|candidate| format!("{label_prefix}{}", candidate.relative))
            .collect::<Vec<_>>();
        tests.sort();
        tests.dedup();
        output.push_str(&format!(
            "{}: {} definition(s)\n",
            touched.relative,
            names.len()
        ));
        if tests.is_empty() {
            output.push_str(
                "  tests: none reference these definitions; run the owning package's suite\n",
            );
        } else {
            output.push_str(&format!(
                "  tests: {}\n",
                display_list(&tests[..tests.len().min(TESTS_PER_FILE)])
            ));
            if tests.len() > TESTS_PER_FILE {
                output.push_str(&format!(
                    "  {}\n",
                    withheld(
                        tests.len() - TESTS_PER_FILE,
                        tests.len(),
                        TESTS_PER_FILE,
                        "test files",
                        "pass one changed file per call to see its own list in full"
                    )
                ));
            }
        }
    }
    output.push_str("run: `cargo test -p <package>` for the packages the listed tests belong to\n");
    Ok(output)
}

/// The member a path lives in, as `(label prefix, member directory)`.
/// 一个路径所在的成员，形如 `(标签前缀, 成员目录)`。
///
/// Ownership is the root prefix on a virtual workspace and "this root" on a single package,
/// which is the same rule the ownership resolver applies to named paths.
/// 归属在虚拟工作区上按根前缀判定，在单包上就是"这个根"——与归属解析器对点名路径用的同一条规则。
fn owner_of(root: &Path, path: &str) -> (String, Option<std::path::PathBuf>) {
    let Ok(crate::mcp::workspace::Scope::Workspace(members)) = crate::mcp::workspace::scope(root)
    else {
        return (String::new(), None);
    };
    for member in members {
        let prefix = member
            .dir
            .strip_prefix(root)
            .map(|relative| {
                nichlink_kernel::declaration::portable_path(&relative.to_string_lossy())
            })
            .unwrap_or_else(|_| member.name.clone());
        if path == prefix || path.starts_with(&format!("{prefix}/")) {
            return (format!("{prefix}/"), Some(member.dir));
        }
    }
    (String::new(), None)
}

/// Whether an indexed file is a test file, by the same convenience rule `callgraph` prints with.
/// 一个已索引的文件是不是测试文件，用的是 `callgraph` 打印时那条便利规则。
fn is_test_file(file: &SourceFile) -> bool {
    // The same tightening `callgraph` prints with: a file that merely mounts a test module is
    // production, and naming it would tell the reader to run the code under test.
    // 与 `callgraph` 打印时同一条收紧后的规则：只是挂了测试模块的文件是生产代码，把它列出来等于叫读者去跑
    // 被测代码本身。
    file.relative.contains("/tests/")
        || file.relative.ends_with("_tests.rs")
        || file.source.contains("#[test]")
}

#[cfg(test)]
#[path = "affected_tests.rs"]
mod affected_tests;
