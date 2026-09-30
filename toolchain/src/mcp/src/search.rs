//! `nichlink.search`: what a name is in the sources, and what the tree says it
//! became.
//! `nichlink.search`：一个名字在源码里是什么，以及树说它变成了什么。
//!
//! The source index answers "which file or function has this name"; it said
//! nothing about the registry, which is why an agent used to grep macro names and
//! reconstruct the tree — the drift `nichlink.registry` exists to remove. A search
//! that stops at source text also cannot say whether the face it found is still
//! the one the build published, so this page answers both halves in one call: the
//! faces whose logical path, `kind`, module or `registry_name` match, each annotated
//! `ok`, `added since build`, `re-identified` or `build unknown`, followed by the
//! file and function hits exactly as before.
//! 源码索引回答"哪个文件或函数叫这个名字"；它对注册树一无所知，这正是代理过去靠 grep 宏名、自己
//! 重建那棵树的原因——而那正是 `nichlink.registry` 要消除的漂移。止步于源码文本的搜索也说不出它找到
//! 的面是否仍是构建发布的那一个，因此本页一次回答两半：逻辑路径、`kind`、模块或 `registry_name` 匹配的面，各自
//! 标注 `ok`、`added since build`、`re-identified` 或 `build unknown`；随后是与此前完全相同的文件与
//! 函数命中。
//!
//! The verdicts come from `crate::mcp::tree_delta`, the rule `nichlink.diff` states, so
//! the same face cannot be `added` here and something else there. A root whose
//! identity namespace cannot be learned still answers the source half and says the
//! tree half is unavailable, because a refusal the caller cannot act on would be
//! worse than an answer that names what is missing.
//! 结论来自 `crate::mcp::tree_delta`，也就是 `nichlink.diff` 说出的那条规则，因此同一个面不可能在这里是
//! `added`、在那里是别的。身份命名空间无从得知的根仍然回答源码那一半，并说明树那一半不可用，因为调用方
//! 无法据以行动的拒绝会比一个点名缺失之物的回答更糟。
//!
//! A **virtual manifest** is a root, and it is answered as the workspace it is: the census
//! names every member with its status, each member's matching faces are grouped under it,
//! and a member Cargo cannot resolve says `tree unavailable (reason)` instead of
//! disappearing behind a longer list of file hits.
//! **虚拟清单**是一个根，而它按它实际的样子——工作区——作答：普查点名每个成员及其状态，每个成员命中
//! 的面分组在它之下，而 Cargo 解析不了的成员说出 `tree unavailable (原因)`，而不是消失在一份更长的
//! 文件命中之后。

use std::path::Path;

use crate::build_time::FaceView;
use nichlink_kernel::identity::NodeId;
use serde_json::Value;

use crate::mcp::protocol::DEFAULT_LIMIT;
use crate::mcp::source_index::load_sources;
use crate::mcp::tree_delta::{FaceStatus, TreeDelta};
use crate::mcp::workspace::{self, Scope};

/// Find registration faces, source files, and Rust function declarations by name.
/// 按名字查找注册面、源码文件与 Rust 函数声明。
pub(crate) fn search(root: &Path, arguments: &Value) -> Result<String, String> {
    // The convergence layer asks the call graph about the name as it was written: a function name
    // is case-sensitive, and the face matcher below is not.
    // 收敛层按写下的原名去问调用图：函数名区分大小写，而下面的面匹配不区分。
    // Two questions live behind one tool, and they are answered differently: `query` matches
    // **names** (faces, files, functions) while `literal` matches **text** anywhere in a source
    // file, comments and string literals included. Asking both at once is refused rather than
    // silently preferring one, because the two answers are not the same shape.
    // 一个工具后面住着两个问题，而它们答法不同：`query` 匹配**名字**（面、文件、函数），`literal` 匹配
    // 源码文件里任何位置的**文本**，注释与字符串字面量都算。同时问两个会被拒绝，而不是静默偏向其中
    // 一个，因为两个答案不是同一种形状。
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize);
    if arguments.get("query").is_some() && arguments.get("literal").is_some() {
        return Err(
            "`query` matches names and `literal` matches text; pass one of them, not both"
                .to_owned(),
        );
    }
    if let Some(literal) = arguments.get("literal").and_then(Value::as_str) {
        // A hit with no context is a line number the caller then has to `read` around — the measured
        // extra call. `context` is off by default so the cheap answer stays cheap, and the trailer
        // below says the one word that turns it on.
        // 没有上下文的命中只是一个行号，调用方随后还得在它周围 `read` 一次——那正是量出来的多出来的
        // 一次调用。`context` 默认关闭，让便宜的答案保持便宜，而下面的尾注说出打开它的那一个词。
        let context = arguments
            .get("context")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(10) as usize;
        return literal_search(root, literal, limit, context);
    }
    let written = arguments
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| "nichlink.search requires `query` (a name) or `literal` (text)".to_owned())?
        .trim()
        .to_owned();
    let query = written.to_ascii_lowercase();
    if query.is_empty() {
        return Err("query must not be empty".to_owned());
    }
    let mut results = Vec::new();
    // The limit used to cut the scan short with nothing said, so a caller that got
    // `limit` rows could not tell a complete answer from a full one. The rows the limit
    // governs are counted over the **whole** scan now — the text is already loaded, so
    // counting costs no I/O — because a count taken while breaking early is a lower
    // bound, and the sentence below says `of N`.
    // 上限过去把扫描提前切断而一言不发，因此拿到 `limit` 行的调用方分不出这是完整答案还是一个被塞满
    // 的答案。现在受上限约束的那些行在**整次**扫描里计数——文本本来就已经载入，计数不花 I/O——因为
    // 在提前 break 处取到的计数只是一个下界，而下面那句话说的是「N 中的 M」。
    let mut hits = 0usize;
    let mut withheld_hits = 0usize;
    // The tree comes first: the face is the unit every other tool names, and a
    // query that names a face must not be consumed by the files that mention it.
    // 树排在前面：面是其它每个工具命名的单位，而点名了某个面的查询不能被提到它的那些文件吃掉。
    //
    // A registration file the derivation could not parse is a fact about the *tree
    // that was read*, not about what this query matched, so it is reported whether
    // or not any face matched: a query naming a face that lives in a broken file
    // used to come back "no matches" with nothing saying the file was broken
    // (`LGC-LG-11`). The stale-manifest note stays inside the guard, because
    // it is about the verdicts of the rows that follow it.
    // 推导解析不了的注册面文件是**被读到的那棵树**的事实，而不是这次查询匹配到了什么的
    // 事实，因此无论有没有面命中它都要报出来：一个点名了某个面的查询，若那个面住在坏文件
    // 里，过去会回一条"no matches"而完全不提那个文件是坏的（`LGC-LG-11`）。下面那条过期
    // 清单的备注仍留在守卫里，因为它说的是随后的行给出的结论。
    //
    // It is also not subject to `limit`: `limit` bounds the *result rows* — the face,
    // file, and function hits below — while this line is a fact about the tree that was
    // read, so it keeps appearing even when the limit has already cut the face list
    // short. Reading it as one more result row is how a caller would conclude that a
    // complete answer came back when a file was in fact dropped.
    // 它同样不受 `limit` 约束：`limit` 限的是**结果行**——下面那些面、文件与函数命中——而
    // 这一行是"读到的这棵树"的事实，因此即使上限已经截短了面的清单，这一行仍会出现。把它当成
    // 又一条结果行，会让调用方在一个文件其实被丢掉时以为拿到的是一份完整的答案。
    //
    // The workspace half follows the same rule at the top level: a virtual root's census is
    // a fact about the tree that was read, so it is stated in full — every member with its
    // status — and a member whose tree cannot be derived says so instead of contributing
    // nothing to a longer list of file hits.
    // 工作区那一半在顶层遵循同一条规则：虚拟根的普查是关于"被读到的那棵树"的事实，因此完整写出
    // ——每个成员带它的状态——而推导不出树的成员明说，而不是在一份更长的文件命中里什么都不贡献。
    match workspace::scope(root) {
        Ok(Scope::Package(namespace)) => {
            match record_face_lines(root, &query, limit, &mut hits, &mut withheld_hits)? {
                Some(lines) => results.extend(lines),
                None => {
                    let (faces, unparsable) = crate::mcp::resolve::derived_faces(root, &namespace)?;
                    let lines = package_tree_lines(
                        root,
                        &faces,
                        &unparsable,
                        &query,
                        limit,
                        &mut hits,
                        &mut withheld_hits,
                    );
                    // The source is declared only when there is something to read: on a query that
                    // matched nothing, "derived from the sources" is noise about an empty answer.
                    // 只有真有东西可读时才声明来源：什么都没匹配上时，"由源码推导而来"是对一份空答案的噪音。
                    if !lines.is_empty() {
                        results.push(DERIVED_SOURCE.to_owned());
                    }
                    results.extend(lines);
                }
            }
        }
        Ok(Scope::Workspace(members)) => {
            results.push(workspace::roster(root, &members));
            for member in &members {
                if let Some(lines) =
                    record_face_lines(&member.dir, &query, limit, &mut hits, &mut withheld_hits)?
                    && !lines.is_empty()
                {
                    results.push(format!("member {} ({})", member.name, member.status()));
                    results.extend(lines);
                    continue;
                }
                match member.derived_tree() {
                    Ok((faces, unparsable)) => {
                        let lines = package_tree_lines(
                            &member.dir,
                            &faces,
                            &unparsable,
                            &query,
                            limit,
                            &mut hits,
                            &mut withheld_hits,
                        );
                        if !lines.is_empty() {
                            results.push(format!("member {} ({})", member.name, member.status()));
                            results
                                .push(member.evidence_line(DERIVES_BECAUSE).trim_end().to_owned());
                            results.extend(lines);
                        }
                    }
                    Err(reason) => results.push(format!(
                        "member {} ({})  tree unavailable ({reason})",
                        member.name,
                        member.status()
                    )),
                }
            }
        }
        Ok(Scope::Unresolvable(reason)) => results.push(format!("tree  unavailable ({reason})")),
        Err(error) => results.push(format!("tree  unavailable ({error})")),
    }
    // The source half is unchanged: file paths and function declarations by name.
    // 源码那一半不变：按名字匹配的文件路径与函数声明。
    let files = load_sources(root)?;
    for file in &files {
        if file.relative.to_ascii_lowercase().contains(&query) {
            if hits < limit {
                results.push(format!("file  {}", file.relative));
                hits += 1;
            } else {
                withheld_hits += 1;
            }
        }
        for function in &file.functions {
            if !function.name.to_ascii_lowercase().contains(&query) {
                continue;
            }
            if hits < limit {
                // The doc's first line rides along: "the doc says A and the code writes not-A" is
                // the strongest signal this bridge prints, and the evaluation's arm needed two
                // requests to put the two halves side by side in every round.
                // 文档首行随行给出："文档说要 A、代码写着 ¬A"是本桥能打印的最强信号，而评测里那一组每轮都
                // 要两次请求才把两半并排。
                let doc = match doc_first_line(&file.source, function.line) {
                    Some(doc) => format!("  /// {doc}"),
                    None => String::new(),
                };
                results.push(format!(
                    "fn    {} -> {}:{}{doc}",
                    function.name, file.relative, function.line
                ));
                hits += 1;
            } else {
                withheld_hits += 1;
            }
        }
    }
    // Nothing matched: a face, a file, or a function. At a package root that is the whole
    // answer, exactly as before; at a workspace root it follows the census, and saying it is
    // what keeps a census — a fact about the tree that was read rather than a result — from
    // reading as a result list that was cut short.
    // 什么都没有命中：面、文件、函数都没有。在包根上这就是整个答案，与从前完全一样；在工作区根上它
    // 跟在普查之后，而说出来正是为了让普查——关于"被读到的那棵树"的事实而不是结果——不会被读成一份被
    // 截短的结果清单。
    if hits == 0 {
        // A dead end is where a caller most needs the next step: names and text are different
        // questions, and the spellings that fail *as names* (`Type::method`, an enum variant, the
        // message a failing assertion printed) are exactly the ones `literal` answers. The
        // evaluation measured this cost: three rounds in a row the narrowing step was "find where
        // this message is produced", and the answer had to come from a grep outside the bridge.
        // 死路正是调用方最需要下一步的地方：名字与文本是两个问题，而**按名字**失败的拼法
        // （`Type::method`、枚举变体、失败断言印出的那句话）恰恰是 `literal` 能答的那些。评测量出了
        // 这个代价：连着三轮，"范围收窄"那一步都是"去找这句话在哪里产出"，而答案只能来自桥外的 grep。
        results.push(format!(
            "no matches in {} — names only; for text (a message, an enum variant, a qualified \
             spelling like `Type::method`) pass `literal`",
            root.display()
        ));
    }
    if withheld_hits > 0 {
        results.push(crate::mcp::truncation::withheld(
            withheld_hits,
            hits + withheld_hits,
            limit,
            "results",
            "raise `limit` or narrow the query",
        ));
    }
    // One entry that converges: the tree layer above, then the chain this name leads into, then
    // what to do next and what was **not** looked at. The chain half is the call-graph tool's own
    // answer, composed rather than re-derived — a layer that repeated the matching rule would be
    // the second implementation this repository keeps deleting.
    // 一个收敛的入口：上面那层是树，接着是这个名字引向的链，然后是下一步该做什么、以及**没有**看什么。
    // 链那一半是调用图工具自己的答案，是**组合**而非重新推导——重复匹配规则的一层，正是本仓一直在删的
    // 第二份实现。
    if arguments.get("converge").and_then(Value::as_bool) == Some(true) {
        results.push("chain:".to_owned());
        let chain = crate::mcp::callgraph::callgraph(
            root,
            &serde_json::json!({"function": written, "limit": limit}),
        )?;
        for line in chain.lines() {
            results.push(format!("  {line}"));
        }
        results.push(
            "next: pass `path` to select one definition when several match, and `nichlink.affected` \
             with the files you change to see which tests to run"
                .to_owned(),
        );
        results.push(
            "bounds: static only — dynamic dispatch, function pointers, FFI and runtime branches \
             need a recorded trace (`nichlink.trace`), and the index covers the files under this \
             root"
                .to_owned(),
        );
    }
    Ok(results.join("\n"))
}

/// Every line of every source file that contains this text, verbatim.
/// 每个源码文件里含有这段文本的那些行，原样给出。
///
/// This is the one question the bridge could not answer before, and the evaluation measured what
/// that cost: three times in three rounds the narrowing step was "the failing assertion's message
/// names a constant, find where that message is produced" — a string in the sources — and every
/// time the agent had to leave the bridge and grep. The search is deliberately literal: raw bytes,
/// case-sensitive, comments and string literals included. A masked view would be the wrong answer
/// for exactly this question, because the thing being looked for usually *is* a string literal.
/// 这是桥此前唯一答不了的问题，而评测量出了它的代价：三轮里三次，"范围收窄"的那一步都是"失败的断言
/// 文案里点名了一个常量，去找这句话是在哪里产出的"——源码里的一处字符串——而每一次代理都只能离开桥
/// 去 grep。检索刻意做成字面的：原始字节、区分大小写、注释与字符串字面量都算。对这种问题，屏蔽过的
/// 视图恰恰是错答案，因为要找的东西通常**就是**一个字符串字面量。
fn literal_search(
    root: &Path,
    literal: &str,
    limit: usize,
    context: usize,
) -> Result<String, String> {
    if literal.is_empty() {
        return Err("`literal` must not be empty".to_owned());
    }
    let mut results = vec![format!(
        "literal {literal:?} (raw bytes, case-sensitive; comments and string literals included)"
    )];
    let mut hits = 0usize;
    let mut withheld = 0usize;
    // The closure reports whether the hit was recorded: a hit past the limit must not drag its
    // context lines into an answer that already said how much it withheld.
    // 闭包回报这次命中是否被记下：超出上限的命中不得把它周围的上下文拖进一份已经声明扣下多少的答案里。
    let mut record = |file: &str, line: usize, text: &str, results: &mut Vec<String>| -> bool {
        if hits < limit {
            results.push(format!("{file}:{line}: {}", clipped(text)));
            hits += 1;
            true
        } else {
            withheld += 1;
            false
        }
    };
    match workspace::scope(root)? {
        Scope::Package(_) => {
            literal_lines(root, literal, context, &mut results, &mut record)?;
        }
        Scope::Workspace(members) => {
            for member in &members {
                let found =
                    literal_lines(&member.dir, literal, context, &mut results, &mut record)?;
                if found > 0 {
                    results.push(format!("member {} ({found} lines)", member.name));
                }
            }
        }
        Scope::Unresolvable(reason) => results.push(format!("tree  unavailable ({reason})")),
    }
    if hits == 0 {
        // The moment a caller most needs to know **which tree** was searched is the moment nothing
        // matched: "no matches" alone cannot be told from "no matches in the wrong root".
        // 调用方最需要知道**搜的是哪棵树**的时刻，恰恰是什么都没匹配上的时刻：光一句 "no matches"
        // 分不出"这棵树里没有"与"在错的根上搜了"。
        results.push(format!("no matches in {}", root.display()));
    } else if context == 0 {
        // Guidance in the answer rather than in a preamble: prose instructions measured 0/4
        // compliance, while the one pointer embedded in a response was followed immediately.
        // 指引放进答案而不是开场白：散文指令实测 0/4 被遵守，而嵌在响应里的那一条当场就被照做。
        results.push("note   pass context: 2 to print the lines around each hit here".to_owned());
    }
    if withheld > 0 {
        results.push(crate::mcp::truncation::withheld(
            withheld,
            hits + withheld,
            limit,
            "lines",
            "raise `limit` or make the literal longer",
        ));
    }
    Ok(results.join("\n"))
}

/// One root's literal hits, in file order then line order.
/// 一个根上的字面命中，按文件顺序、再按行顺序。
fn literal_lines(
    root: &Path,
    literal: &str,
    context: usize,
    results: &mut Vec<String>,
    record: &mut impl FnMut(&str, usize, &str, &mut Vec<String>) -> bool,
) -> Result<usize, String> {
    let mut found = 0usize;
    for file in load_sources(root)? {
        let lines = file.source.lines().collect::<Vec<_>>();
        for (index, line) in lines.iter().enumerate() {
            if !line.contains(literal) {
                continue;
            }
            found += 1;
            if !record(&file.relative, index + 1, line, results) || context == 0 {
                continue;
            }
            let start = index.saturating_sub(context);
            let end = (index + context + 1).min(lines.len());
            for (other, text) in lines.iter().enumerate().take(end).skip(start) {
                if other != index {
                    results.push(format!("  {}: {}", other + 1, clipped(text)));
                }
            }
        }
    }
    Ok(found)
}

/// The first line of the doc comment attached above a definition, when there is one.
/// 定义正上方所附文档注释的第一行（若有）。
///
/// "Above" means the contiguous `///` block immediately preceding the definition, and "first" means
/// the top of that block: that is the line a reader has to see next to the code to catch the case
/// where the promise and the implementation disagree.
/// "上方"指紧邻定义之前那段连续的 `///` 块，"第一行"指该块的顶端：要抓住"承诺与实现不一致"这种情况，
/// 读者必须把那一行与代码并排看到。
fn doc_first_line(source: &str, line: usize) -> Option<String> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut at = line.saturating_sub(2);
    let mut top: Option<&str> = None;
    while let Some(current) = lines.get(at) {
        if !current.trim_start().starts_with("///") {
            break;
        }
        top = Some(current);
        if at == 0 {
            break;
        }
        at -= 1;
    }
    let text = top?.trim_start().trim_start_matches("///").trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// One source line, capped, saying so when it was.
/// 一行源码，设上限，被截时说出来。
fn clipped(line: &str) -> String {
    const COLUMNS: usize = 200;
    if line.chars().count() <= COLUMNS {
        return line.trim().to_owned();
    }
    let head = line.chars().take(COLUMNS).collect::<String>();
    format!("{} … (line cut at {COLUMNS} columns)", head.trim_end())
}

/// Why this answer derives instead of reading a member's published records.
/// 这份答案为什么推导，而不是读成员的已发布记录。
///
/// A face is matched on its logical path, `kind`, module and `registry_name`, and
/// the published record carries none of the first two and only the source for the
/// rest — so the tree half of a search reads the sources. The records are still
/// read first: they are what tells this answer whether a member was built at all,
/// and every group says which tree its rows came from.
/// 面是按逻辑路径、`kind`、模块与 `registry_name` 匹配的；已发布记录从 2026-09-29 起携带声明拼出的
/// 这四项，但仍不携带模块，也不携带"解析不了的面"与"外部面"这两张清单——因此搜索的树那一半仍推导，
/// 理由是它要报告的比一张表多。记录仍然先被读取：正是它告诉这份答案一个成员到底有没有被构建过，而每个
/// 分组都会说出它的行来自哪棵树。
const DERIVES_BECAUSE: &str = "the published record now carries a face's declared path, kind, registry_name and parent, but this answer also counts the faces the derivation cannot parse and the external ones it excludes, which only the derivation sees";

/// The tree half one package contributes: the file that could not be parsed, the
/// stale-manifest note, and the face rows the query matched.
/// 一个包贡献的树那一半：解析不了的文件、过期清单备注，以及查询命中的面行。
///
/// The face rows are the rows `limit` bounds, so they are counted here across every
/// package a workspace query reads; the two note lines are facts about the tree that was
/// read and are never counted (`LGC-LG-11`). The stale note is emitted only in front of
/// rows, because it is about the verdicts that follow it.
/// 面行就是 `limit` 约束的那些行，因此它们在这里跨工作区查询读到的每个包计数；那两行备注是关于
/// "被读到的那棵树"的事实，从不计数（`LGC-LG-11`）。过期备注只在行之前发出，因为它说的是随后那些
/// 行的结论。
fn package_tree_lines(
    root: &Path,
    faces: &[FaceView],
    unparsable: &str,
    query: &str,
    limit: usize,
    hits: &mut usize,
    withheld_hits: &mut usize,
) -> Vec<String> {
    let built = TreeDelta::read(root);
    let mut rows = Vec::new();
    for face in faces {
        if matches_face(face, query) {
            if *hits < limit {
                rows.push(face_line(face, &built));
                *hits += 1;
            } else {
                *withheld_hits += 1;
            }
        }
    }
    let mut lines = Vec::new();
    if !unparsable.is_empty() {
        lines.push(unparsable.trim_end().to_owned());
    }
    if !rows.is_empty() {
        // A stale manifest still answers, but every verdict below it is
        // about the tree that build saw rather than this one. The state word
        // is the one every other report prints; the clause after it belongs
        // to this note rather than to the word.
        // 过期的清单仍能作答，但它下面每个结论针对的是那次构建看到的树，而不是眼前这棵。状态词
        // 与其余每份报告打印的那一种相同；后面的从句属于这条备注，而不是那个词。
        if built.known && !built.current {
            lines.push(
                "tree  build stale (run `nichlink check`); the statuses below \
                 compare against that build"
                    .to_owned(),
            );
        }
        lines.extend(rows);
    }
    lines
}

/// Whether a face is named by the query: its logical path, kind, module, or
/// `registry_name`.
/// 查询是否点名了某个面：它的逻辑路径、kind、模块或 `registry_name`。
///
/// The source path is deliberately not one of the four: a file hit already answers
/// "which file", and letting a face match on its source would make every file line
/// appear twice — once as a face and once as a file.
/// 源码路径有意不在那四项之内：文件命中的行已经回答了"哪个文件"，而让面按源码匹配会让每个文件都出现
/// 两次——一次作为面，一次作为文件。
/// Says which tree the face rows below came from when they came from the build's records.
/// 下面那些面行来自构建记录时，说明它们读的是哪棵树。
const RECORD_SOURCE: &str = "tree  answered from the build's published records (a record carries \
                             path, kind and registry_name; the module is not in one, so a query \
                             that only matches a module derives from the sources)";

/// Says which tree they came from when they had to be derived.
/// 只能推导时，说明它们读的是哪棵树。
const DERIVED_SOURCE: &str =
    "tree  derived from the sources (this root's published records did not answer the query)";

/// Face rows answered from the member's published records, when they can be.
/// 能从成员的已发布记录作答的那些面行。
///
/// The records carry everything a name match needs and a verdict needs — identity, source, and the
/// three declared facts — so this path costs no source walk at all. It answers `None` when the
/// records are unreadable or nothing in them matches, and the caller derives instead; that is not a
/// fallback of last resort but the honest half of the split, because a record does **not** carry the
/// face's module, and a query that only matches a module can only be answered from the sources.
/// 记录携带了名字匹配与裁决所需的一切——身份、源码路径，以及那三项声明的事实——因此这条路径完全不花
/// 源码遍历。记录读不了、或其中没有命中时返回 `None`，由调用方改为推导；那不是最后的兜底，而是这个分工
/// 诚实的那一半：记录**不**携带面的模块，而只匹配模块的查询只能由源码作答。
fn record_face_lines(
    root: &Path,
    query: &str,
    limit: usize,
    hits: &mut usize,
    withheld_hits: &mut usize,
) -> Result<Option<Vec<String>>, String> {
    let out = crate::mcp::build_evidence::out_dir(root);
    let Ok(rows) = crate::build_time::read_pruning_manifest(&out) else {
        return Ok(None);
    };
    // One row per tracked symbol, so a face appears as many times as it tracks symbols.
    // 每个被跟踪符号一行，因此一个面会出现它跟踪符号数次。
    let mut by_id: std::collections::BTreeMap<NodeId, &crate::build_time::PruningRow> =
        std::collections::BTreeMap::new();
    for row in &rows {
        by_id.entry(row.id).or_insert(row);
    }
    let built = TreeDelta::read(root);
    let mut lines = Vec::new();
    for (id, row) in &by_id {
        let declared = [
            row.path.as_deref(),
            row.kind.as_deref(),
            row.registry_name.as_deref(),
        ];
        if !declared
            .iter()
            .flatten()
            .any(|field| field.to_ascii_lowercase().contains(query))
        {
            continue;
        }
        if *hits < limit {
            lines.push(record_face_line(*id, row, &built));
            *hits += 1;
        } else {
            *withheld_hits += 1;
        }
    }
    if lines.is_empty() {
        return Ok(None);
    }
    let mut answer = vec![RECORD_SOURCE.to_owned()];
    if built.known && !built.current {
        answer.push(
            "tree  build stale (run `nichlink check`); the statuses below compare against that \
             build"
                .to_owned(),
        );
    }
    answer.extend(lines);
    Ok(Some(answer))
}

/// One face hit built from a published record.
/// 由一条已发布记录构成的面命中。
fn record_face_line(id: NodeId, row: &crate::build_time::PruningRow, built: &TreeDelta) -> String {
    let status = if !built.known {
        "build unknown (run `nichlink check`)".to_owned()
    } else {
        let verdict = built.status_of(id, &row.source);
        if let FaceStatus::Reidentified(previous) = verdict {
            format!("{} ({previous} -> {id})", verdict.label())
        } else {
            verdict.label().to_owned()
        }
    };
    format!(
        "face  {:<40} kind={:<14} registry={:<16} source={}  [{status}]",
        row.path.as_deref().unwrap_or("-"),
        row.kind.as_deref().unwrap_or("-"),
        row.registry_name.as_deref().unwrap_or("-"),
        row.source
    )
}

fn matches_face(face: &FaceView, query: &str) -> bool {
    [&face.path, &face.kind, &face.module, &face.registry_name]
        .iter()
        .any(|field| field.to_ascii_lowercase().contains(query))
}

/// One face hit, with the build's verdict attached.
/// 一条面命中，附上构建的结论。
fn face_line(face: &FaceView, built: &TreeDelta) -> String {
    let status = if !built.known {
        "build unknown (run `nichlink check`)".to_owned()
    } else {
        let verdict = built.status(face);
        if let FaceStatus::Reidentified(previous) = verdict {
            format!("{} ({previous} -> {})", verdict.label(), face.id)
        } else {
            verdict.label().to_owned()
        }
    };
    format!(
        "face  {:<40} kind={:<14} module={:<28} source={}  [{status}]",
        face.path, face.kind, face.module, face.source
    )
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod search_tests;
