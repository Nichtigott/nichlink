//! The tree-wide half of a `check`: what the tree declares and how that declaration can drift,
//! in the two forms that are decidable without a model.
//! `check` 的"全树"那一半：这棵树声明了什么、以及这些声明会怎么漂移——只做两种**不需要模型**就能
//! 判定的形态。
//!
//! A symptom narrows the scope, and that is the right shape for a symptom (the caller names the failing
//! assertion, the face and the call sites do the rest). An **open** question — "are there other
//! problems in this tree" — has no such narrowing, and the measured failure was that nothing answered
//! it: one arm hand-swept every public doc to find a re-spelled constant, the other only compared the
//! contracts on the symptom's own path and never saw it.
//! 症状会自己收窄范围，那对症状是对的（调用方给出失败断言，面与调用点做完剩下的事）。**开放式**的问题
// ——"这棵树里还有别的问题吗"——没有这种收窄，而量到的失败正是没有任何东西回答它：一个臂手工扫遍所有
// 公开文档才找到那处被重拼的常量，另一个只比对了症状路径上的契约、因此从没看见它。
//!
//! Every line here is a **static fact about the tree**, not a verdict about the code, and the section
//! ends by saying what it does not cover — a census that claimed completeness would be the same
//! false green this repository keeps deleting.
//! 这里的每一行都是**关于这棵树的静态事实**，不是对代码的裁定，而且这一节末尾会说明它**不覆盖**什么
// ——一份自称完备的总账，就是这个仓库一直在删的那种假绿。

use std::collections::HashSet;
use std::path::Path;

const SAMPLE: usize = 5;

/// The most indexed functions the test-reachability walk answers for.
/// 测试可达性遍历最多为多少个已索引函数作答。
///
/// The walk tests each call name it reaches against this tree's functions through the one shared
/// `is_call_to` predicate, so this is not a timer: it is where the census stops claiming to answer
/// at all, because a tree larger than this has never been measured against that column. 10,000 is
/// an order of magnitude above a single-package root and comfortably above this checkout's own tree
/// (a few thousand indexed functions); past it the column prints the skip with its own count
/// instead of walking a tree it was never sized for.
/// 这次遍历把它到达的每个调用名通过那条共享的 `is_call_to` 判据与本树的函数比对，因此这不是计时器：
/// 它是**总账停止自称作答**的那一点，因为比这更大的树从未对着那一栏量过。10,000 比单包根高一个数量级、
/// 也比本检出自己的树（几千个已索引函数）宽裕得多；超过它时那一栏打印带自身计数的跳过说明，而不是去走
/// 一棵从未按它定过尺寸的树。
const REACHABILITY_BUDGET: usize = 10_000;

/// What the test-reachability column does not cover.
/// 测试可达性那一栏**不覆盖**什么。
///
/// One sentence, emitted with the column, because a static walk that let itself be read as a
/// coverage measurement would be exactly the false green this census exists against.
/// 一句话，随那一栏输出——一次放任自己被读成"覆盖率"的静态遍历，正是这份总账要对付的那种假绿。
const REACHABILITY_BOUNDARY: &str = "  not covered by the test-reachability column: calls made through dynamic dispatch, function pointers, FFI, or macro expansion are invisible to it, so a function reached only that way is still listed; a function reached only through a trait method or a closure does not count; matching is by name across the whole tree, so a call of the same name on an unrelated type counts as reaching it; every function in a test-looking file seeds the walk (a `tests/` directory, a `_tests.rs` sibling, or a file declaring `#[test]` itself), so a production file that carries its own `#[test]` errs toward not being listed; and `main` is not listed because the process entry point is called by the OS rather than by a test. This is a static reachability walk, not a coverage measurement.";

/// One declared numeric constant, as the tree spells it.
/// 一条被声明的数值常量，按这棵树里的写法。
struct Declared {
    name: String,
    value: String,
    relative: String,
    line: usize,
}

/// Whether `text` uses `value` as a whole number, with no word character or `.` next to it.
/// `text` 是否把 `value` 当作一个完整的数字使用——两侧既不是词字符、也不是 `.`。
fn uses_whole_number(text: &str, value: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(at) = text[from..].find(value) {
        let start = from + at;
        let end = start + value.len();
        let before_ok = start == 0 || {
            let before = bytes[start - 1] as char;
            !before.is_alphanumeric() && before != '_' && before != '.'
        };
        let after_ok = end >= bytes.len() || {
            let after = bytes[end] as char;
            !after.is_alphanumeric() && after != '_' && after != '.'
        };
        if before_ok && after_ok {
            return true;
        }
        from = end;
    }
    false
}

/// The census, as answer lines: the header, one line per finding, and the coverage sentence.
/// 总账，按答案行给出：表头、每条发现一行、以及覆盖范围那一句。
pub(crate) fn census(root: &Path) -> Result<Vec<String>, String> {
    let sources = crate::mcp::source_index::load_sources(root)?;
    let mut declared: Vec<Declared> = Vec::new();
    for file in &sources {
        for (index, line) in file.source.lines().enumerate() {
            let Some(rest) = line.trim().strip_prefix("pub const ") else {
                continue;
            };
            let (Some(name), Some(value)) = (rest.split(':').next(), rest.split('=').nth(1)) else {
                continue;
            };
            let value = value.trim().trim_end_matches(';').trim().replace('_', "");
            if !value.is_empty() && value.chars().all(|digit| digit.is_ascii_digit()) {
                declared.push(Declared {
                    name: name.trim().to_owned(),
                    value,
                    relative: file.relative.clone(),
                    line: index + 1,
                });
            }
        }
    }
    let mut lines = vec![format!(
        "census: {} named numeric constant(s); a static fact about this tree, not a verdict",
        declared.len()
    )];
    let mut found: Vec<String> = Vec::new();
    for constant in &declared {
        for file in &sources {
            for (index, text) in file.source.lines().enumerate() {
                let at = format!("{}:{}", file.relative, index + 1);
                if at == format!("{}:{}", constant.relative, constant.line) {
                    continue;
                }
                if uses_whole_number(text, &constant.value) {
                    found.push(format!(
                        "  respelled {} is declared as `{}` ({}:{}) and written again at {at}",
                        constant.value, constant.name, constant.relative, constant.line
                    ));
                }
            }
        }
        let production = sources
            .iter()
            .filter(|file| !crate::mcp::callgraph::looks_like_a_test(&file.relative, &file.source))
            .filter(|file| file.relative != constant.relative)
            .filter(|file| file.source.contains(&constant.name))
            .count();
        if production == 0 {
            found.push(format!(
                "  unreferenced `{}` ({}:{}) is not read anywhere outside tests",
                constant.name, constant.relative, constant.line
            ));
        }
    }
    let total = found.len();
    for line in found.iter().take(SAMPLE) {
        lines.push(line.clone());
    }
    if total > SAMPLE {
        lines.push(crate::mcp::truncation::withheld(
            total - SAMPLE,
            total,
            SAMPLE,
            "census items",
            "ask per directory to see its own items",
        ));
    }
    // The third column: what the entry plan declares, as a static count. Families a and b both spent
    // real effort working out that "declaring one more cut" is what changes what the application
    // ships — and both arms derived it by experiment. The count itself is a fact this tree already
    // contains, so the census reports it and lets the caller draw the conclusion.
    // 第三栏：入口计划声明了什么，按静态计数给出。a/b 两族都花了真实力气才弄清"多声明一条 cut"才是
    // 改变"这个应用发布什么"的那一步——而两臂都是靠实验推出来的。计数本身是这棵树已有的事实，总账把它
    // 报出来，结论留给调用方。
    let cuts = sources
        .iter()
        .map(|file| file.source.matches("cut(").count())
        .sum::<usize>();
    let grafts = sources
        .iter()
        .map(|file| file.source.matches("graft(").count())
        .sum::<usize>();
    lines.push(format!(
        "  entry plan: {cuts} `cut(` site(s) and {grafts} `graft(` site(s) across this tree's sources \
         (a static count; which of them this application ships is the plan's own business)"
    ));
    // The fourth column: declarations no test writes down. The round-4 index asked for a "which
    // promises does this tree make, who verifies them, and which ones no case touches" view, and
    // the decidable first cut is the cheap one: a `pub fn` in a production file that appears in no
    // test file at all. It is a text-level fact and it says so — a test that reaches the function
    // through a face, a macro, or a call it never spells does not count here.
    // 第四栏：没有任何测试写下来的声明。第四轮的索引要的是"这棵树许了哪些承诺、谁在验证、哪些没有任何
    // 用例碰过"的视图，而可判定的第一刀是便宜的那一刀：生产文件里的 `pub fn`，在任何测试文件里一次都
    // 没出现。这是文本级事实，而且它自己会说出来——通过面、宏或一次未拼出名字的调用抵达该函数的测试，
    // 在这里不算数。
    let test_text = sources
        .iter()
        .filter(|file| crate::mcp::callgraph::looks_like_a_test(&file.relative, &file.source))
        .map(|file| file.source.as_str())
        .collect::<Vec<_>>();
    let mut unnamed: Vec<String> = Vec::new();
    for file in &sources {
        if crate::mcp::callgraph::looks_like_a_test(&file.relative, &file.source) {
            continue;
        }
        for (index, line) in file.source.lines().enumerate() {
            let Some(rest) = line.trim().strip_prefix("pub fn ") else {
                continue;
            };
            let name = rest
                .split(['(', '<', ' '])
                .next()
                .unwrap_or_default()
                .trim();
            if name.is_empty() || name == "main" {
                continue;
            }
            if !test_text.iter().any(|text| text.contains(name)) {
                unnamed.push(format!(
                    "  no test names `{name}` ({}:{})",
                    file.relative,
                    index + 1
                ));
            }
        }
    }
    let unnamed_total = unnamed.len();
    for line in unnamed.iter().take(SAMPLE) {
        lines.push(line.clone());
    }
    if unnamed_total > SAMPLE {
        lines.push(crate::mcp::truncation::withheld(
            unnamed_total - SAMPLE,
            unnamed_total,
            SAMPLE,
            "unverified declarations",
            "ask per directory to see its own items",
        ));
    }
    lines.push(format!(
        "  declarations: {unnamed_total} production `pub fn` name(s) appear in no test file (a \
         text-level count: a test that reaches one without writing its name does not count here)"
    ));
    // The fifth column: production functions **no test can reach**, by walking the call graph the
    // name view and the orphan view already walk. This is the behavioural half of the round-4
    // question and it is deliberately not the orphan question: "nothing calls it" and "no test can
    // reach it" are two different facts about a function, so this column decides what a call names
    // through the one shared `callgraph::is_call_to` and shares nothing else with `orphans`.
    // 第五栏：**没有任何测试能到达**的生产函数，沿名字视图与孤儿视图本来就在走的那张调用图走。这是第四轮
    // 那个问题的行为级那一半，而且它**有意**不是孤儿问题："没人调用它"与"没有测试能到达它"是关于同一个函数
    // 的两件不同事实，因此这一栏经**唯一**那份 `callgraph::is_call_to` 判定一次调用点名了什么，其余什么都不
    // 与 `orphans` 共享。
    lines.extend(reachability_column(&sources));
    lines.push(
        "  not covered: named numeric constants, production `pub fn` names, and one static walk from \
         test files are what this census reads; string constants, structural duplication, runtime \
         behaviour, and claims written in prose are outside it, and the walk is not a coverage \
         measurement (its own column names the call shapes it cannot see)"
            .to_owned(),
    );
    Ok(lines)
}

/// The production functions a static walk from this tree's test files cannot reach, and the bounds
/// that walk carries.
/// 本树的测试文件出发做一次静态遍历**到不了**的那些生产函数，以及这次遍历自带的上限。
///
/// Behavioural, and static: the walk starts at every function defined in a file the bridge's own
/// convenience rule calls a test file, follows the call lists the source index already extracted,
/// and reports the functions left over. An edge is the same fact `orphans` reads — a name appearing
/// in some function's call list — and it is read through the same function, `callgraph::is_call_to`,
/// so the two views cannot disagree about which call names which definition; dynamic dispatch,
/// function pointers, FFI and macro-expanded calls are invisible here exactly as they are there, and
/// the column says so rather than calling itself coverage.
/// 行为级、且静态：遍历从**桥自己的便利规则**判为测试文件的文件里的每个函数出发，沿源码索引已经抽出的
/// 调用清单走，把剩下的函数报出来。一条边与 `orphans` 读的是同一个事实——一个名字出现在某个函数的调用
/// 清单里——而且经过**同一个函数** `callgraph::is_call_to` 来读，因此两个视图不可能对"哪次调用点名了哪个
/// 定义"产生分歧；动态派发、函数指针、FFI 与宏展开出来的调用在这里与在那里一样不可见，而这一栏会把这点
/// 说出来，而不是自称覆盖率。
fn reachability_column(sources: &[crate::mcp::source_index::SourceFile]) -> Vec<String> {
    let indexed = sources
        .iter()
        .map(|file| file.functions.len())
        .sum::<usize>();
    let mut lines = Vec::new();
    if indexed > REACHABILITY_BUDGET {
        lines.push(format!(
            "  test-reachable: skipped ({indexed} function(s) over the limit of \
             {REACHABILITY_BUDGET}); this tree is larger than the walk was sized for, so no \
             reachability rows are computed for it"
        ));
        lines.push(REACHABILITY_BOUNDARY.to_owned());
        return lines;
    }
    // One flat list of every indexed function, with the file it came from and whether that file is
    // a test file — the same classification `looks_like_a_test` gives the other columns.
    // 一份扁平的已索引函数清单，带上它来自哪个文件、以及那个文件是不是测试文件——与其它几栏同一个
    // `looks_like_a_test` 判据。
    let flat = sources
        .iter()
        .flat_map(|file| {
            let is_test = crate::mcp::callgraph::looks_like_a_test(&file.relative, &file.source);
            file.functions
                .iter()
                .map(move |function| (file, function, is_test))
        })
        .collect::<Vec<_>>();
    // The seeds are the names of every function in a test file. Worklist rather than recursion:
    // the queue only grows when a call string is new, and there are finitely many of those, so the
    // walk terminates on a cyclic graph and cannot overflow the stack on a deep one.
    // 种子是测试文件里每个函数的名字。用工作表而不是递归：只有当某个调用字符串是新的时队列才增长，而这样
    // 的字符串是有限的，因此遍历在带环的图上会终止、在很深的图上也不会把栈撑爆。
    let mut reached: HashSet<String> = HashSet::new();
    let mut queue: Vec<String> = Vec::new();
    for (_, function, is_test) in &flat {
        if *is_test && reached.insert(function.name.clone()) {
            queue.push(function.name.clone());
        }
    }
    let mut visited = vec![false; flat.len()];
    while let Some(call) = queue.pop() {
        // Which definitions this call names is `is_call_to`'s business and nobody else's: the rule
        // has one implementation, in the module that owns the call graph, and both this column and
        // the orphan view go through it.
        // 这次调用点名了哪些定义是 `is_call_to` 的事、不是别人的：这条规则只有一份实现，住在拥有调用图的
        // 那个模块里，而本栏与孤儿视图都经过它。
        for (at, (_, function, _)) in flat.iter().enumerate() {
            if visited[at] || !crate::mcp::callgraph::is_call_to(&call, &function.name) {
                continue;
            }
            visited[at] = true;
            for callee in &flat[at].1.calls {
                if reached.insert(callee.clone()) {
                    queue.push(callee.clone());
                }
            }
        }
    }
    let mut production = 0usize;
    let mut rows = Vec::new();
    for (at, (file, function, is_test)) in flat.iter().enumerate() {
        // `main` is left out for the same reason `orphans` leaves it out: it is the process entry
        // point, the OS calls it, and listing it would be a row every reader has to undo.
        // `main` 不列，理由与 `orphans` 一样：它是进程入口、由操作系统调用，列出来是一行每个读者都得自己
        // 撤销的东西。
        if *is_test || function.name == "main" {
            continue;
        }
        production += 1;
        if visited[at] {
            continue;
        }
        rows.push(format!(
            "  no test reaches `{}` ({}:{})",
            function.name, file.relative, function.line
        ));
    }
    let total = rows.len();
    lines.push(format!(
        "  test-reachable: {total} of {production} production function(s) no test can reach \
         ({indexed} function(s) indexed in this tree; a static walk from the test files along the \
         same name-in-call-list rule the orphan view uses)"
    ));
    for row in rows.iter().take(SAMPLE) {
        lines.push(row.clone());
    }
    if total > SAMPLE {
        lines.push(crate::mcp::truncation::withheld(
            total - SAMPLE,
            total,
            SAMPLE,
            "test-unreachable functions",
            "ask per directory to see its own items",
        ));
    }
    lines.push(REACHABILITY_BOUNDARY.to_owned());
    lines
}

#[cfg(test)]
#[path = "claims_tests.rs"]
mod claims_tests;
