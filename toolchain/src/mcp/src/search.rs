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
use serde_json::Value;

use crate::mcp::protocol::DEFAULT_LIMIT;
use crate::mcp::source_index::load_sources;
use crate::mcp::tree_delta::{FaceStatus, TreeDelta};
use crate::mcp::workspace::{self, MemberState, Scope};

/// Find registration faces, source files, and Rust function declarations by name.
/// 按名字查找注册面、源码文件与 Rust 函数声明。
pub(crate) fn search(root: &Path, arguments: &Value) -> Result<String, String> {
    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| "nichlink.search requires query".to_owned())?
        .trim()
        .to_ascii_lowercase();
    if query.is_empty() {
        return Err("query must not be empty".to_owned());
    }
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize);
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
            let (faces, unparsable) = crate::mcp::resolve::derived_faces(root, &namespace)?;
            results.extend(package_tree_lines(
                root,
                &faces,
                &unparsable,
                &query,
                limit,
                &mut hits,
                &mut withheld_hits,
            ));
        }
        Ok(Scope::Workspace(members)) => {
            results.push(workspace::roster(root, &members));
            for member in &members {
                match &member.state {
                    MemberState::Derived { faces, unparsable } => {
                        let lines = package_tree_lines(
                            &member.dir,
                            faces,
                            unparsable,
                            &query,
                            limit,
                            &mut hits,
                            &mut withheld_hits,
                        );
                        if !lines.is_empty() {
                            results.push(format!("member {} ({})", member.name, member.status()));
                            results.extend(lines);
                        }
                    }
                    MemberState::Unresolvable(reason) => results.push(format!(
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
                results.push(format!(
                    "fn    {} -> {}:{}",
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
        results.push("no matches".to_owned());
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
    Ok(results.join("\n"))
}

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
