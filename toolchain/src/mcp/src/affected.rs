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
//!
//! online: the build publishes faces, not which test reaches which source, so this answer is about the sources as they are now.

use std::path::Path;

use serde_json::Value;

use crate::mcp::callgraph::is_call_to;
use crate::mcp::source_index::{SourceFile, display_list, load_sources};
use crate::mcp::truncation::withheld;

/// How many tests one changed file may name before the reply says how many it withheld.
/// 一个改动过的文件最多点名多少个测试，超出后回复要说出扣下了多少。
const TESTS_PER_FILE: usize = 10;

pub(crate) fn affected(root: &Path, arguments: &Value) -> Result<String, String> {
    // One path and a list of paths are the same question asked twice, so both spellings are
    // accepted: a single `--files <path>` became a string on the command line, and refusing it was
    // the round's three-call detour through `--json`.
    // 一个路径与一串路径是同一个问题问两次，因此两种写法都收：命令行上的单个 `--files <路径>` 是字符串，
    // 拒绝它正是那轮经 `--json` 绕行的三次调用。
    let files = match arguments.get("files") {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        // A comma-separated string is the third spelling of "several files", and the round measured
        // it failing: `--files "a.rs,b.rs"` reached the tool as **one** path named `a.rs,b.rs` and
        // answered `not in the index`, while the same two paths in a JSON array worked. Nothing
        // about the argument's type says "one path" — its spelling does, and a comma is how a list
        // is written on a command line.
        // 逗号分隔的字符串是"多个文件"的第三种拼法，而那一轮量到它失败：`--files "a.rs,b.rs"` 以**一个**
        // 名为 `a.rs,b.rs` 的路径到达，答 `not in the index`，而同样两个路径写成 JSON 数组就成功。参数的
        // **类型**并没有说"这是一个路径"—— 是它的**拼法**在说，而逗号正是命令行上写列表的方式。
        Some(Value::String(spelled)) => spelled
            .split(',')
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>(),
        _ => {
            return Err(
                "nichlink.affected requires `files`: one path, a list of paths, or repeated \
                 `--files` flags from the one-shot client"
                    .to_owned(),
            );
        }
    };
    let files = files
        .into_iter()
        .map(|path| path.trim().to_owned())
        .filter(|path| !path.is_empty())
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
    if let Some(above) = crate::mcp::workspace::enclosing_workspace(root) {
        // The cross-member half of this answer is the whole point of the tool, so when the root is a
        // member the caller has to know the other members are out of frame.
        // 这个答案的"跨成员"那一半正是这个工具的意义，因此当根是成员时，调用方必须知道别的成员在画面外。
        output.push_str(&format!(
            "note  this root is a member of the workspace at {}; other members are out of frame \
             here\n",
            above.display()
        ));
    }
    for path in &files {
        let (label_prefix, owner) = owner_of(root, path);
        let sources = match &owner {
            Some(member) => load_sources(member)?,
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
                // Whether a call names one of this file's definitions is `callgraph::is_call_to`'s
                // business and nobody else's: the rule has one implementation, in the module that
                // owns the call graph, and this tool, `orphans` and the census's test-reachability
                // column all go through it.
                // 一次调用是否点名了本文件里的某个定义，是 `callgraph::is_call_to` 的事、不是别人的：这条
                // 规则只有一份实现，住在拥有调用图的那个模块里，而本工具、`orphans` 与总账的测试可达性栏
                // 都经过它。
                candidate.functions.iter().any(|caller| {
                    caller
                        .calls
                        .iter()
                        .any(|call| names.iter().any(|name| is_call_to(call, name)))
                })
            })
            .map(|candidate| format!("{label_prefix}{}", candidate.relative))
            .collect::<Vec<_>>();
        // A test in another member reaches this file through the library's public surface, and the
        // member that owns the changed file cannot see it. The scenario round measured exactly this
        // miss: a change to a core model file did not name the report crate's suite, while the
        // control tool's own impact answer did. Every other member is asked the same question here.
        // 另一个成员里的测试经库的公开表面抵达这个文件，而拥有该改动文件的成员看不见它。情景轮量到的正是
        // 这一次漏报：改 core 的模型文件时没有点名 report crate 的套件，而对照工具自己那条影响面答案点名了。
        // 这里对其余每个成员问同一个问题。
        if let Ok(crate::mcp::workspace::Scope::Workspace(members)) =
            crate::mcp::workspace::scope(root)
        {
            for member in &members {
                if Some(&member.dir) == owner.as_ref() {
                    continue;
                }
                let Ok(other) = load_sources(&member.dir) else {
                    continue;
                };
                let label = member
                    .dir
                    .strip_prefix(root)
                    .map(|relative| {
                        nichlink_kernel::declaration::portable_path(&relative.to_string_lossy())
                    })
                    .unwrap_or_else(|_| member.name.clone());
                for candidate in other.iter().filter(|candidate| is_test_file(candidate)) {
                    let reaches = candidate.functions.iter().any(|caller| {
                        caller
                            .calls
                            .iter()
                            .any(|call| names.iter().any(|name| is_call_to(call, name)))
                    });
                    if reaches {
                        tests.push(format!("{label}/{}", candidate.relative));
                    }
                }
            }
        }
        tests.sort();
        tests.dedup();
        // T-27: `is_call_to` matches a **bare** definition name against a **qualified** call — which
        // is right (`Store::new` does call the `new` defined on `Store`) but loses the owner, so a
        // file defining `new` also "affects" every test calling some *other* type's `new`. Making the
        // rule bare-matches-bare would trade that over-report for an under-report, and "no test
        // covers this" is the answer a reader acts on — so the list stays and the ambiguity is named.
        // T-27：`is_call_to` 把**裸名**定义匹配到**限定**调用上 —— 这是对的（`Store::new` 确实调用了
        // `Store` 上那个 `new`），但它丢掉了属主，于是一个定义了 `new` 的文件也会"影响"所有调用**别的**
        // 类型的 `new` 的测试。改成"裸名只匹配裸名"会把误报换成漏报，而"没有测试覆盖它"是读者会照做的
        // 答案 —— 因此清单保留，改为点头这处歧义。
        let mut bare_on_qualified: Vec<String> = sources
            .iter()
            .flat_map(|file| file.functions.iter())
            .flat_map(|function| function.calls.iter())
            .filter(|call| call.contains("::"))
            .filter(|call| {
                names
                    .iter()
                    .any(|name| !name.contains("::") && is_call_to(call, name))
            })
            .cloned()
            .collect();
        bare_on_qualified.sort();
        bare_on_qualified.dedup();
        if !bare_on_qualified.is_empty() {
            output.push_str(&format!(
                "note   {} call(s) matched a **bare** definition name on a qualified call ({}), so a \
                 definition named here and another type's same-named one are indistinguishable from \
                 this list alone — check those first\n",
                bare_on_qualified.len(),
                display_list(&bare_on_qualified[..bare_on_qualified.len().min(6)])
            ));
        }
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
