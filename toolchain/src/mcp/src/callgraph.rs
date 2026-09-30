//! The static call-graph answer, and the bounds it needs.
//! 静态调用图答案，以及它需要的上限。
//!
//! Lifted out of `tools.rs` when a new tool pushed that page into the line
//! ratchet: the catalog and the query implementations belong there, the one answer
//! that grows without limit belongs here, next to its two bounds and their tests.
//! 当一个新工具把 `tools.rs` 推到行数棘轮边上时把它挪出来：目录与查询实现留在那里，而这唯一会
//! 无界增长的答案连同它的两道上限与测试放在这里。
//!
//! The measured failure these bounds exist against: a common name (`new`) had 142
//! definitions in the NichUI corpus, and every call site of that name was listed for
//! each of them, which arrived as a 4.5 MB reply. An answer an agent cannot read is
//! not an answer.
//! 这些上限所针对的实测失败：常见名（`new`）在 NichUI 语料里有 142 个定义，而每个定义都列出该名字
//! 的每一个调用点，最终以 4.5 MB 的回复抵达。代理读不下的答案不算答案。

use std::path::Path;

use serde_json::Value;

use crate::mcp::source_index::{display_list, load_sources};
use crate::mcp::truncation::withheld;

/// Whether a labelled source looks like a test file, by this reader's own convenience rule.
/// 一个已标注的源码看起来是不是测试文件，按这个读取方自己的便利规则判定。
///
/// Not the conventions gate's rule and not a judgement about placement: the bridge answers from
/// text and has to name the files a reader would run, so it reads the three spellings a test
/// file has in this repository — a `tests/` directory, a `_tests.rs` sibling, or `#[test]` in
/// the file itself.
/// 不是 conventions 门禁那条规则，也不是对位置的判断：桥从文本作答，必须点名读者会去跑的那些文件，因此它
/// 读本仓里测试文件的三种拼法——`tests/` 目录、同级 `_tests.rs`、或文件里自带 `#[test]`。
pub(crate) fn looks_like_a_test(label: &str, source: &str) -> bool {
    // `#[cfg(test)]` is deliberately not a test: a production file that mounts its own test
    // module carries it, and listing that file would tell a reader to run the code under test.
    // `#[test]` is, because a file declaring tests runs them.
    // 有意不把 `#[cfg(test)]` 当成测试：挂着自家测试模块的生产文件就带着它，而把它列出来等于叫读者去跑
    // 被测代码本身。`#[test]` 算，因为声明了测试的文件会跑那些测试。
    label.contains("/tests/") || label.ends_with("_tests.rs") || source.contains("#[test]")
}

pub(crate) fn callgraph(root: &Path, arguments: &Value) -> Result<String, String> {
    // Two bounds, because this answer is the one that grows without limit: the
    // measurement in the module doc above is the failure they exist against.
    // Definitions and callers are capped separately, and both say how much they
    // withheld.
    // 两道上限，因为这是唯一会无界增长的答案：上面模块文档记下的那次实测就是它们针对的失败。
    // 定义数与调用者各自设上限，且都说出自己扣下了多少。
    const DEFINITIONS: usize = 5;
    const CALLERS: usize = 20;
    // The orphan view asks about the whole tree rather than about one name, so it is answered
    // before `function` is required — otherwise the shape the table advertises would be refused by
    // an argument rule, which is exactly what the round measured.
    // 孤儿视图问的是整棵树而不是某个名字，因此在要求 `function` 之前作答——否则流程表推荐的形状会被参数
    // 规则拒掉，而那正是那轮量到的东西。
    if arguments.get("orphans").and_then(Value::as_bool) == Some(true) {
        return orphan_answer(root, arguments);
    }
    let query = arguments
        .get("function")
        .and_then(Value::as_str)
        .ok_or_else(|| "nichlink.callgraph requires function or orphans".to_owned())?
        .trim();
    if query.is_empty() {
        return Err("function must not be empty".to_owned());
    }
    let path_filter = arguments.get("path").and_then(Value::as_str);
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFINITIONS, |value| value.clamp(1, 50) as usize);
    // A name lookup at a workspace root is one question about the whole workspace, not one
    // question per member: a caller lives in a member that depends on the definition's own
    // member, so asking each member separately loses every cross-crate caller (measured: one
    // symbol had 36 callers over the whole workspace and 17 when each member searched only
    // itself). The label each file carries is what the answer prints, so a workspace hit names
    // the member it came from while a single package's hits keep their own relative paths.
    // 工作区根上的名字查询是关于整个工作区的一个问题，而不是每个成员各问一遍：调用者住在依赖定义所在
    // 成员的那个成员里，因此逐成员提问会丢掉每一个跨 crate 调用者（实测：同一个符号在整个工作区上 36 个
    // 调用者，而每个成员只搜自己时 17 个）。每个文件携带的标签就是答案打印的东西，因此工作区命中会点名
    // 它来自哪个成员，而单包命中保持自己的相对路径。
    let labelled = labelled_sources(root, path_filter)?;
    let mut found = Vec::new();
    for (label, file) in &labelled {
        if path_filter.is_some_and(|path| label != path) {
            continue;
        }
        for function in &file.functions {
            if function.name == query || function.name.ends_with(&format!("::{query}")) {
                found.push((label, function));
            }
        }
    }
    if found.is_empty() {
        return Ok(format!("no static function match for `{query}`"));
    }
    let total = found.len();
    let mut output = format!("evidence: static-heuristic\nmatches {total}\n");
    if total > 1 && path_filter.is_none() {
        // Naming the ambiguity is the difference between a usable answer and a
        // dump: with several definitions the reader has to choose, and `path` is
        // how they choose.
        // 把歧义说出来，是好答案与一坨倾倒之间的区别：有多个定义时读取方必须选一个，而 `path`
        // 就是他们选的工具。
        output.push_str(&format!(
            "note: {total} definitions match `{query}`; pass `path` to select one. Callers are matched \
             by name across the whole tree, so for a common name they include unrelated call sites.\n"
        ));
    }
    for (label, function) in found.into_iter().take(limit) {
        // The matched name and its qualified suffix are cloned into the inner closure: it runs
        // once per candidate file, and borrowing the definition from the outer scope made the
        // closure outlive it.
        // 被匹配的名字与它的限定后缀被克隆进内层闭包：它每个候选文件跑一次，而从外层作用域借用那个定义
        // 会让闭包活得比它长。
        let name = function.name.clone();
        let suffix = format!("::{query}");
        let mut callers = labelled
            .iter()
            .flat_map(|(caller_label, candidate)| {
                let prefix = format!("{caller_label}::");
                let name = name.clone();
                let suffix = suffix.clone();
                candidate
                    .functions
                    .iter()
                    .filter(move |caller| {
                        caller
                            .calls
                            .iter()
                            .any(|call| call == &name || call.ends_with(&suffix))
                    })
                    .map(move |caller| format!("{prefix}{}", caller.name))
            })
            .collect::<Vec<_>>();
        callers.sort();
        callers.dedup();
        let callers_total = callers.len();
        output.push_str(&format!("{label}:{} fn {}\n", function.line, function.name));
        output.push_str(&format!(
            "  callers ({}): {}\n",
            callers_total,
            display_list(&callers[..callers_total.min(CALLERS)])
        ));
        if callers_total > CALLERS {
            // The caller cap is this tool's own constant, not `limit`: no argument
            // raises it, so the sentence has to name the absence rather than advice a
            // caller cannot act on.
            // 调用者上限是本工具自己的常数、不是 `limit`：没有任何参数能提高它，因此那句话必须
            // 点名这个"没有"，而不是给出调用方无法执行的建议。
            output.push_str(&format!(
                "  {}\n",
                withheld(
                    callers_total - CALLERS,
                    callers_total,
                    CALLERS,
                    "callers",
                    "pass `path` to list one definition's callers; no argument raises this cap"
                )
            ));
        }
        output.push_str(&format!("  callees: {}\n", display_list(&function.calls)));
        // Which tests reference this definition: a name that a test file calls is covered by
        // that file, and a reader deciding what to run needs the file rather than the caller
        // list it already has. The rule for "a test file" is this reader's convenience — a
        // `tests/` directory, a `_tests.rs` sibling, or a file that declares `#[test]` itself —
        // and not the conventions gate's, which the bridge cannot use and which judges placement
        // rather than what a file does.
        // 哪些测试引用了这个定义：某个测试文件调用的名字就由那个文件覆盖，而"该跑什么"的读者要的是文件名，
        // 不是他已经拿到的调用者清单。"测试文件"的判据是这个读取方的便利规则——位于 `tests/` 目录、同级的
        // `_tests.rs`、或文件里带 `#[test]`——不是 conventions 门禁的那条：桥用不了它，而且它判的是位置
        // 而不是文件在做什么。
        let mut tests = labelled
            .iter()
            .filter(|(test_label, candidate)| {
                looks_like_a_test(test_label, &candidate.source)
                    && candidate.functions.iter().any(|caller| {
                        caller
                            .calls
                            .iter()
                            .any(|call| call == &name || call.ends_with(&suffix))
                    })
            })
            .map(|(test_label, _)| test_label.clone())
            .collect::<Vec<_>>();
        tests.sort();
        tests.dedup();
        if !tests.is_empty() {
            output.push_str(&format!(
                "  tests: {}\n",
                display_list(&tests[..tests.len().min(CALLERS)])
            ));
        }
        if arguments
            .get("source")
            .and_then(Value::as_bool)
            .unwrap_or(true)
            && let Some((_, file)) = labelled.iter().find(|(file_label, _)| file_label == label)
        {
            // The definition's own span, capped by this tool's own constant and declared
            // through the one truncation outlet: a reader who asked for source gets source, and
            // one who asked for a summary keeps the summary.
            // 定义自己的那一段，由本工具自己的常数设上限、并走那个唯一的截断出口：要源码的读者拿到源码，
            // 要摘要的读者仍然拿到摘要。
            const SOURCE_LINES: usize = 60;
            let lines = file.source.lines().collect::<Vec<_>>();
            let definition = function.line.saturating_sub(1);
            // Start at the doc comment's first line when there is one: "the doc says A and the code
            // writes not-A" is the strongest signal this bridge can print, and the evaluation's arm
            // hit that pairing in every round — twice over two requests each time, because the doc
            // and the body arrived from different calls.
            // 有文档注释时从它的首行开始："文档说要 A、代码写着 ¬A"是本桥能打印的最强信号，而评测里那一组
            // 每轮都撞到这个配对——每次都要两次请求，因为文档与函数体来自不同的调用。
            let mut first = definition;
            while first > 0 {
                let above = lines[first - 1].trim_start();
                if above.starts_with("///") || above.starts_with("#[") {
                    first -= 1;
                } else {
                    break;
                }
            }
            let last = function.end_line.min(lines.len());
            let shown = last.saturating_sub(first).min(SOURCE_LINES);
            output.push_str("  source:\n");
            for line in &lines[first..first + shown] {
                output.push_str(&format!("    {line}\n"));
            }
            if last.saturating_sub(first) > shown {
                output.push_str(&format!(
                    "  {}\n",
                    withheld(
                        last - first - shown,
                        last - first,
                        SOURCE_LINES,
                        "lines",
                        "pass `path` and use `nichlink.read` for the whole file"
                    )
                ));
            }
        }
    }
    if total > limit {
        output.push_str(&format!(
            "{}\n",
            withheld(
                total - limit,
                total,
                limit,
                "definitions",
                "raise `limit` or pass `path`"
            )
        ));
    }
    output.push_str("dynamic dispatch, function pointers, FFI, and runtime branches require live CallTrace evidence.\n");
    // Guidance in the answer, and the measured default: the evaluation's rounds each spent one
    // extra call to see the body after this tool had shown only names. Bodies are therefore
    // included **by default**; a caller who wants the short answer says `source: false` and the
    // answer says how to get them back.
    // 指引放进答案，默认值来自实测：评测每一轮都在这个工具只给了名字之后，多花一次调用去看函数体。
    // 因此函数体**默认包含**；想要短答案的调用方说 `source: false`，而答案会说怎么把它们要回来。
    if arguments.get("source").and_then(Value::as_bool) == Some(false) {
        output.push_str(
            "note   bodies omitted on request: drop `source: false` (or pass source: true) for \
             each definition's own lines\n",
        );
    }
    Ok(output)
}

/// The files an answer may read, each under the label its rows print.
/// 答案可以读的那些文件，各自带着它那些行会打印的标签。
///
/// Extracted so the orphan view and the name view cannot disagree about which files exist or what
/// a row is called: one rule, one implementation. A second copy here would be the drift this
/// repository keeps deleting.
/// 抽出来是为了让孤儿视图与名字视图不会对"有哪些文件、一行该叫什么"产生分歧：一条规则一份实现。在这里再抄
/// 一份就是本仓一直在删的那种漂移。
fn labelled_sources(
    root: &Path,
    path_filter: Option<&str>,
) -> Result<Vec<(String, crate::mcp::source_index::SourceFile)>, String> {
    let mut labelled: Vec<(String, crate::mcp::source_index::SourceFile)> = Vec::new();
    match crate::mcp::workspace::scope(root) {
        Ok(crate::mcp::workspace::Scope::Workspace(members)) if path_filter.is_none() => {
            for member in &members {
                let Ok(sources) = load_sources(&member.dir) else {
                    continue;
                };
                // The label is the member's directory relative to the workspace root, which is
                // the path a reader can grep for; the package name alone would name the identity
                // but not the file's place in this checkout.
                // 标签是成员目录相对工作区根的路径，也就是读者能直接 grep 的路径；只用包名会点名身份，
                // 却不说明文件在本次检出里的位置。
                let prefix = member
                    .dir
                    .strip_prefix(root)
                    .map(|relative| {
                        nichlink_kernel::declaration::portable_path(&relative.to_string_lossy())
                    })
                    .unwrap_or_else(|_| member.name.clone());
                for file in sources {
                    labelled.push((format!("{prefix}/{}", file.relative), file));
                }
            }
        }
        _ => {
            for file in load_sources(root)? {
                labelled.push((file.relative.clone(), file));
            }
        }
    }
    Ok(labelled)
}

/// What this package defines and never calls.
/// 本包定义了、却从不调用的那些东西。
///
/// The round asked for this view by name: the trap round's `write_totals` was a textbook dead-code
/// candidate, the table advertised `orphans: true`, and the tool refused it twice — two of that
/// round's ten calls. The count is static and says so: a name counts as called when it appears in
/// some function's call list *in this tree*, so dynamic dispatch, function pointers and
/// macro-expanded calls are invisible to it, exactly as they are to the name view.
/// 这一视图是那轮点名要的：陷阱题里的 `write_totals` 是教科书级死代码候选，而流程表推荐了 `orphans: true`
/// 却被工具拒了两次——那轮 10 次调用里的 2 次。计数是静态的并且说出来：只要一个名字出现在**这棵树**某个函数
/// 的调用清单里就算被调用，因此动态派发、函数指针与宏展开出来的调用对它不可见——与名字视图一样。
fn orphan_answer(root: &Path, arguments: &Value) -> Result<String, String> {
    const ORPHANS: usize = 20;
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(ORPHANS, |value| value.clamp(1, 200) as usize);
    let labelled = labelled_sources(root, None)?;
    let mut called: Vec<&str> = Vec::new();
    for (_, file) in &labelled {
        for function in &file.functions {
            for call in &function.calls {
                called.push(call.as_str());
            }
        }
    }
    let mut found: Vec<String> = Vec::new();
    for (label, file) in &labelled {
        if looks_like_a_test(label, &file.source) {
            continue;
        }
        for function in &file.functions {
            if function.name == "main" {
                continue;
            }
            let suffix = format!("::{}", function.name);
            let reached = called
                .iter()
                .any(|call| *call == function.name || call.ends_with(&suffix));
            if !reached {
                found.push(format!(
                    "  fn    {} -> {label}:{}",
                    function.name, function.line
                ));
            }
        }
    }
    let total = found.len();
    let mut output = String::from("evidence: static-heuristic\n");
    if let Some(above) = crate::mcp::workspace::enclosing_workspace(root) {
        // Measured: asked at a member root this view calls cross-member callers invisible, so it says
        // who is invisible and where to ask instead rather than presenting them as orphans.
        // 量到的：在成员根上问时，这个视图看不见跨成员的调用者，因此它说清"谁不可见、该去哪儿问"，
        // 而不是把它们当成孤儿报出来。
        output.push_str(&format!(
            "note  this root is the member `{}` of the workspace at {}; callers in other members \
             are invisible here — ask at that root for the whole-workspace answer\n",
            root.file_name().unwrap_or_default().to_string_lossy(),
            above.display()
        ));
    }
    output.push_str(&format!(
        "orphans {total} (defined here, no static caller in this tree)\n"
    ));
    for line in found.iter().take(limit) {
        output.push_str(line);
        output.push('\n');
    }
    if total > limit {
        output.push_str(&format!(
            "{}\n",
            withheld(
                total - limit,
                total,
                limit,
                "orphans",
                "raise `limit`, or read the file the row names"
            )
        ));
    }
    output.push_str(
        "note   a name counts as called when it appears in some function's call list in this \
         tree: dynamic dispatch, function pointers and macro-expanded calls are invisible to it\n",
    );
    Ok(output)
}

#[cfg(test)]
#[path = "callgraph_tests.rs"]
mod callgraph_tests;
