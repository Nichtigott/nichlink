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
//!
//! online: the build publishes faces, not call edges; the call graph is a property of the sources as they are now.

use std::path::Path;

use serde_json::Value;

use crate::mcp::source_index::{display_list, load_sources};
use crate::mcp::truncation::withheld;

/// One printed line of a definition's source, marked as contract or implementation.
/// 定义源码里被打印的一行，标成"契约"或"实现"。
///
/// The doc comment **is** the contract this tree writes down, and the body is what it does; the
/// strongest signal this bridge can print is the pair disagreeing, so the reader should not have to
/// work out which printed line is which. `///` and `//!` are the doc spellings; everything else is
/// code.
/// 文档注释**就是**这棵树写下的契约，而函数体是它实际做的事；本桥能打印的最强信号就是这两者不一致，
/// 因此读者不必自己分辨哪一行是哪种。`///` 与 `//!` 是文档的拼法，其余都是代码。
fn source_line(line: &str) -> String {
    let trimmed = line.trim_start();
    let kind = if trimmed.starts_with("///") || trimmed.starts_with("//!") {
        "contract"
    } else {
        "impl"
    };
    format!("    {kind:<8} {line}\n")
}

/// Where a caller sits, when saying it saves the reader a call.
/// 调用者坐在哪里 —— 说出来能替读者省一次调用时才说。
///
/// Two decidable things, both read off the two labels: whether the caller is a test file (the same
/// three spellings `looks_like_a_test` reads), and whether it lives outside the definition's own
/// directory. The wording says **directory**, not "member": the package name is not derivable from a
/// path, and a reply that guessed one would be the kind of fact this bridge refuses to invent.
/// 两件可判定的事，都从两个标签读出来：调用者是不是测试文件（与 `looks_like_a_test` 同一套三种拼法），
/// 以及它是否住在这个定义自己的目录之外。措辞说的是**目录**而不是"成员"：包名无法从路径推出来，而
/// 猜一个正是这座桥拒绝编造的那类事实。
pub(crate) fn caller_note(definition_label: &str, caller_label: &str) -> Option<String> {
    let test = looks_like_a_test(caller_label, "");
    let caller_dir = caller_label.rsplit_once('/').map(|(dir, _)| dir);
    let outside =
        caller_dir.is_some() && caller_dir != definition_label.rsplit_once('/').map(|(dir, _)| dir);
    match (test, outside) {
        (false, false) => None,
        (true, false) => Some("a test file".to_owned()),
        (false, true) => Some(format!(
            "outside this file's directory ({})",
            caller_dir.unwrap_or_default()
        )),
        (true, true) => Some(format!(
            "a test file; outside this file's directory ({})",
            caller_dir.unwrap_or_default()
        )),
    }
}

/// Whether a labelled source looks like a test file, by this reader's own convenience rule./// 一个已标注的源码看起来是不是测试文件，按这个读取方自己的便利规则判定。
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

/// Whether a call spelled `call` names a function spelled `name`: the **one** implementation of the
/// rule the orphan view and the census's test-reachability column both read.
/// 拼作 `call` 的这次调用是否点名了名字为 `name` 的函数：孤儿视图与总账的测试可达性栏共读的那条规则的
/// **唯一**一份实现。
///
/// The rule is: the call *is* the name, or it ends with `::` followed by the name. Both spellings
/// occur in this index — `direct_calls` records the bare last identifier before a `(`, while other
/// readers hand over qualified paths — and the two views have to agree on which of them counts. A
/// rule spelled twice is a rule that drifts, so it is spelled once, here, and both views call it.
/// 规则是：调用**就是**该名字，或它以 `::` 加该名字结尾。两种拼法都出现在本索引里——`direct_calls`
/// 记录 `(` 前最后一个裸标识符，而别的读取方会交来限定路径——而两个视图必须对"哪种算数"一致。拼两遍的
/// 规则就是会漂移的规则，因此它只在这里拼一遍，两个视图都调它。
/// T-27（已定位，未修）：这条规则把**裸名**匹配到**任何**限定调用上 —— `is_call_to("Entry::new",
/// "new")` 命中（`strip_suffix("new")` 得 `"Entry::"`，`ends_with("::")` 成立）。于是 `affected`
/// 对一个改了 `Store::new` 的文件，会把只调用 `Entry::new` 的测试也报成"受影响"，`s4` 上臂
/// 因此被迫改用 `search` + `callgraph` + 逐跳读源去排除那两个无关文件（1–3 次核对调用）。
/// **修法要带"所属类型"**（只有名字时无法区分 `Entry::new` 与 `Store::new`）⇒ 签名要多一个参数，
/// 三个调用点（`affected` ×2、`digest`、`why`）都要跟着改，属单独一轮。
/// T-27 (located, not fixed): a bare name matches any qualified call, so `affected` over-reports.
/// The fix needs the owning type — the signature has to grow, and three call sites with it.
pub(crate) fn is_call_to(call: &str, name: &str) -> bool {
    call == name
        || call
            .strip_suffix(name)
            .is_some_and(|prefix| prefix.ends_with("::"))
}

/// Every name a test-looking file in this tree calls, in the order the files were read.
/// 这棵树里每个像测试的文件所调用的名字，按文件被读到的顺序。
///
/// One implementation, two readers: `digest` marks a function "named by a test" with it, and `why`
/// reports whether anything pins the definition a change is about. The rule is the same one the
/// other readers use — a name that appears in a call list — so a second copy would be a second rule.
/// 一份实现、两个读者：`digest` 用它把函数标成"被测试点名"，而 `why` 用它报告"这次改动有没有东西钉着"。
/// 规则与其它读者用的是同一条——出现在调用列表里的名字——因此写第二份就是第二条规则。
pub(crate) fn names_tests_call(sources: &[crate::mcp::source_index::SourceFile]) -> Vec<String> {
    let mut named = Vec::new();
    for source in sources {
        if !looks_like_a_test(&source.relative, &source.source) {
            continue;
        }
        for function in &source.functions {
            named.extend(function.calls.iter().cloned());
        }
    }
    named
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
    // Several symbols in one call: `{"function": ["a", "b"]}`. The round measured what asking one at
    // a time costs — each answer is a whole turn, and a turn re-sends the context — while the work
    // of "which of these are called, and by whom" is one question about one tree. Every answer here
    // is byte-identical to asking for that symbol alone; they are joined, not merged.
    // 一次问几个符号：`{"function": ["a", "b"]}`。那一轮量过"逐个问"的代价——每份答案是一整轮，而一轮要把
    // 上下文再发一遍——而"这几个谁被调用、被谁调用"本就是关于一棵树的一个问题。这里每份答案与单独问那个
    // 符号**逐字相同**，只是拼接，不做合并。
    if let Some(Value::Array(items)) = arguments.get("function") {
        if items.len() > 8 {
            return Err(format!(
                "`function` takes at most 8 names in one call ({} given); repeat the call, or use \
                 `orphans` for every definition this tree never calls",
                items.len()
            ));
        }
        let mut answers = Vec::new();
        for item in items {
            let mut one = arguments.clone();
            if let Some(map) = one.as_object_mut() {
                map.insert("function".to_owned(), item.clone());
            }
            answers.push(callgraph(root, &one)?);
        }
        return Ok(answers.join("\n"));
    }
    let query = arguments
        .get("function")
        .and_then(Value::as_str)
        .ok_or_else(|| "xirang.callgraph requires function or orphans".to_owned())?
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
            // The definition half asks its own question — *which* definitions match the requested
            // query — and it keeps today's direction on purpose: `is_call_to` receives the
            // definition's spelled name as the spelling and the query as the target, so a definition
            // spelled with a path would be found by a bare query. The reverse direction
            // (`is_call_to(query, &function.name)`), which would let the **query** `inner::helper`
            // name the bare `helper`, is a query-semantics choice rather than a spelling: it was
            // measured to change behaviour (`--function inner::helper` answers "no static function
            // match" today and would hit `helper`), so it is deliberately not taken here.
            // 定义那一半问的是它自己的问题——**哪些**定义匹配这个查询——而它有意保持今天的方向：
            // `is_call_to` 收到的"拼法"是定义的名字、"目标"是查询，因此若有以路径拼写的定义，裸查询也能
            // 找到它。反方向（`is_call_to(query, &function.name)`）会让**查询** `inner::helper` 点名裸的
            // `helper`，那是**查询语义**的取舍而不是拼法：实测它会改变行为（今天 `--function
            // inner::helper` 答"no static function match"，改后命中 `helper`），因此这里有意不取。
            if is_call_to(&function.name, query) {
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
    // The definitions the cap drops are named, because dropping them **silently** was measured to
    // cost a whole diagnosis: with nine definitions of `offset` and a default limit of five, the one
    // the question was about — `toggle`, the outlier — was exactly the one cut, and the reply said
    // nothing (W8, h1). The reader then recovered only because a *callers* sentence happened to name
    // it. Naming them turns the cap into a choice the reader can act on (`path`), instead of a gap
    // they cannot see.
    // 被上限丢掉的定义要**点名**，因为静默丢掉它们被量到过一次整段诊断的代价：`offset` 有九个定义、
    // 默认上限五个，而问题所关心的那一个——离群者 `toggle`——恰恰是被切掉的那个，回复对此一言不发
    // （W8，h1）。读者之所以还能挽回，只是因为另一条**调用者**的句子碰巧点了它的名。点名它们，把这道上限
    // 变成读者能据以行动的选择（`path`），而不是一个他看不见的缺口。
    let dropped = found
        .iter()
        .skip(limit)
        .map(|(label, _)| format!("`{label}`"))
        .collect::<Vec<_>>()
        .join(", ");
    for (label, function) in found.into_iter().take(limit) {
        // The matched name is cloned into the inner closure: it runs once per candidate file, and
        // borrowing the definition from the outer scope made the closure outlive it. Whether a call
        // names that definition is `is_call_to`'s business now, as it is for `orphans`, the census's
        // test-reachability column and `affected`.
        // 被匹配的名字被克隆进内层闭包：它每个候选文件跑一次，而从外层作用域借用那个定义会让闭包活得比它
        // 长。这次调用是否点名了那个定义，现在归 `is_call_to` 管——`orphans`、总账的测试可达性栏与
        // `affected` 也一样。
        let name = function.name.clone();
        let mut callers = labelled
            .iter()
            .flat_map(|(caller_label, candidate)| {
                let prefix = format!("{caller_label}::");
                let name = name.clone();
                candidate
                    .functions
                    .iter()
                    .filter(move |caller| caller.calls.iter().any(|call| is_call_to(call, &name)))
                    .map(move |caller| format!("{prefix}{}", caller.name))
            })
            .collect::<Vec<_>>();
        callers.sort();
        callers.dedup();
        let callers_total = callers.len();
        output.push_str(&format!("{label}:{} fn {}\n", function.line, function.name));
        // Where each caller sits, when there is something to say. Measured in the seventh round:
        // `callers (1): crates/report/tests/report.rs::store` left an agent to send one more `search`
        // to work out that the caller is in another crate's test file.
        // 每个调用者坐在哪里 —— 有话可说时才说。第七轮量到：`callers (1): crates/report/tests/report.rs::store`
        // 让代理又发了一次 `search` 才弄清"调用者在另一个 crate 的测试文件里"。
        let shown: Vec<String> = callers
            .iter()
            .take(CALLERS)
            .map(|caller| {
                let caller_label = caller
                    .rsplit_once("::")
                    .map(|(label, _)| label)
                    .unwrap_or(caller.as_str());
                match caller_note(label, caller_label) {
                    Some(note) => format!("{caller} ({note})"),
                    None => caller.clone(),
                }
            })
            .collect();
        output.push_str(&format!(
            "  callers ({}): {}\n",
            callers_total,
            display_list(&shown)
        ));
        if callers_total == 0
            && let Some(above) = crate::mcp::workspace::enclosing_workspace(root)
        {
            // Measured: asked at a member root, this census cannot see the other members — and zero
            // callers is exactly the answer a reader takes as "nothing calls it". The note appears
            // only at zero, so it is a warning where it matters rather than noise everywhere.
            // 量到的：在成员根上问时这份普查看不见别的成员——而"0 个调用者"恰恰是读者会当成"没人调用它"的
            // 答案。这句话只在 0 时出现，因此它是**该出现处的警告**，而不是到处都有的噪声。
            output.push_str(&format!(
                "  note  this root is the member `{}` of the workspace at {}; a caller in another \
                 member is invisible here — ask at that root for the whole-workspace answer\n",
                root.file_name().unwrap_or_default().to_string_lossy(),
                above.display()
            ));
        }
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
        // Each callee carries the line this tree defines it at: the measured `callgraph → callgraph`
        // pair (16 forced returns in one arm's two rounds) is a walk where every hop ended with "now
        // where is *that* one defined", which this answer can already know.
        // 每个被调用者在**本树**有定义时带上它的定义行：量到的 `callgraph → callgraph`（一个臂两轮里 16 次
        // 被迫回头）就是这样一次走链——每一跳都以"那它又定义在哪"收尾，而这个答案本来就能知道。
        let definitions = labelled_sources(root, None)?;
        let located: Vec<String> = function
            .calls
            .iter()
            .map(|call| {
                for (label, file) in &definitions {
                    for candidate in &file.functions {
                        if is_call_to(call, &candidate.name) {
                            return format!("{call} -> {label}:{}", candidate.line);
                        }
                    }
                }
                call.clone()
            })
            .collect();
        output.push_str(&format!("  callees: {}\n", display_list(&located)));
        // This file's other definitions, with their lines: the measured `callgraph → read` pair (10
        // forced returns in one arm's two rounds) is a caller who got this function and still had to
        // open the file for its neighbours — the "the next hop is in the same file" case.
        // 本文件的其它定义及其行号：量到的 `callgraph → read`（一个臂两轮里 10 次被迫回头）就是"拿到了这个
        // 函数、却还得为它的邻居打开文件"——也就是"下一跳就在同一个文件里"那一族。
        if let Some((label, file)) = definitions
            .iter()
            .find(|(_, file)| file.functions.iter().any(|seen| seen.name == function.name))
        {
            let others: Vec<&crate::mcp::source_index::Function> = file
                .functions
                .iter()
                .filter(|seen| seen.name != function.name)
                .collect();
            if !others.is_empty() {
                const SHOWN: usize = 6;
                let listed = others
                    .iter()
                    .take(SHOWN)
                    .map(|seen| format!("{} -> {label}:{}", seen.name, seen.line))
                    .collect::<Vec<_>>()
                    .join(", ");
                let more = if others.len() > SHOWN {
                    format!(" (+{} more in this file)", others.len() - SHOWN)
                } else {
                    String::new()
                };
                output.push_str(&format!("  also here: {listed}{more}\n"));
            }
        }
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
                    && candidate
                        .functions
                        .iter()
                        .any(|caller| caller.calls.iter().any(|call| is_call_to(call, &name)))
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
                output.push_str(&source_line(line));
            }
            if last.saturating_sub(first) > shown {
                output.push_str(&format!(
                    "  {}\n",
                    withheld(
                        last - first - shown,
                        last - first,
                        SOURCE_LINES,
                        "lines",
                        "pass `path` and use `xirang.read` for the whole file"
                    )
                ));
            }
        }
    }
    if total > limit {
        // The cap was already announced — what it did not do was name **which** definitions it
        // dropped. Measured (W8, h1): nine definitions of `offset`, a default limit of five, and the
        // one the question was about (`toggle`) among the four cut; the reader only got there because
        // a *callers* sentence happened to name it. Naming the files turns the cap into a choice it
        // can act on: each withheld definition is what `path` takes.
        // 这道上限本来就**有**提示——它没做的是点名它丢掉了**哪些**定义。实测（W8，h1）：`offset` 九个
        // 定义、默认上限五个，而问题所关心的那一个（`toggle`）在被切掉的四个里；读者之所以还能找到，只是
        // 因为另一条**调用者**的句子碰巧点了它的名。点名文件把这道上限变成能据以行动的选择：每条被扣下的
        // 定义正是 `path` 收的东西。
        output.push_str(&format!(
            "{}\n",
            withheld(
                total - limit,
                total,
                limit,
                "definitions",
                &format!("pass `path` for one of: {dropped}; `limit` raises this cap")
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
                        xirang_kernel::declaration::portable_path(&relative.to_string_lossy())
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
    // The test half is **counted**, not listed: the harness calls those functions, so listing them
    // as orphans would be a false positive a reader has to undo — but leaving them out silently is
    // what capped both arms of the round-5 evaluation at "partly right" on the orphan question.
    // 测试那一半只**计数**、不列出：测试框架会调用它们，把它们列成孤儿是读者还得自己撤销的假阳性；
    // 而默默略过它们，正是第五轮评测两臂在孤儿题上一同止步于"部分对"的原因。
    let mut test_only: usize = 0;
    for (label, file) in &labelled {
        let is_test = looks_like_a_test(label, &file.source);
        for function in &file.functions {
            if function.name == "main" {
                continue;
            }
            let reached = called.iter().any(|call| is_call_to(call, &function.name));
            match (reached, is_test) {
                (true, _) => {}
                (false, true) => test_only += 1,
                (false, false) => found.push(format!(
                    "  fn    {} -> {label}:{}",
                    function.name, function.line
                )),
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
    if test_only > 0 {
        // The boundary is stated with its number instead of being silent: "one orphan" and "one
        // orphan plus five functions only the harness calls" are different facts about a tree, and
        // the round measured an answer that could not tell them apart.
        // 边界连同它的数字一起说出来，而不是沉默：对一棵树来说，"一个孤儿"与"一个孤儿外加五个只有
        // 测试框架调用的函数"是两件不同的事实，而那轮量到的正是一个分不清它们的答案。
        output.push_str(&format!(
            "note   {test_only} function(s) in test files have no static caller either; the test \
             harness calls them, so they are counted here rather than listed above\n"
        ));
    }
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
