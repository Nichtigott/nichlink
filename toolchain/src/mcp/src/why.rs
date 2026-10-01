//! `nichlink.why`: one call that gathers the upstream facts a symptom at a line depends on.
//! `nichlink.why`：一次调用，把一个位置的症状所依赖的上游事实收齐。
//!
//! The measured failure: a symptom in file Z sent agents hopping — `callgraph` for the callers, then
//! a `read` for the contract, then the registry or the plan for whether the face ships. The seventh
//! round measured that shape directly: the four injected-defect questions took 5, 6, 5 and 8
//! instrument calls against the control's 5, 2, 2 and 2, and the extra hops were exactly the ones
//! this tool answers in one reply.
//! 量到的失败：一个在 Z 文件里的症状逼代理逐跳——`callgraph` 查调用者、`read` 看契约、再问注册面或计划
//! 看这个面到底发不发布。第七轮直接量到这个形状：四道注入缺陷分别花了 5、6、5、8 次仪器调用，而对照是
//! 5、2、2、2，多出来的那几跳正是这个工具一次回答的东西。
//!
//! What it does **not** answer, it names: runtime evidence, and how a grafted subtree's *runtime*
//! state looks. Those are `trace`'s and `grafts`' questions, and the reply says which one to ask.
//! 它**不**回答的，它会点名：运行期证据，以及一条被嫁接子树的**运行期**状态。那些分别归 `trace` 与
//! `grafts`，答案里会写明该问哪一个。
//!
//! The plan half is here too, and it is the half the third hard-bug class turns on: whether the file is
//! one of the sources the build **selected** (a file on disk can be absent from the scope the entry
//! declared, which is exactly what makes a new face vanish), whether a `#[cfg]` gates the definition,
//! and whether a declared graft cut names the face that owns the file. Each of those reads the one
//! implementation that owns it — `build_evidence`'s scope/pruning lines and the graft view's
//! `names_face` — so this cannot disagree with `explain` or `grafts` about the same fact.
//! 计划那一半也在这里，而它正是第三类复杂 bug 所依赖的那一半：这个文件是不是构建**选中**的那些源之一
//! （盘上有的文件可以不在入口声明的作用域里，而那正是新面消失的来路）、定义是否被 `#[cfg]` 门控、以及
//! 有没有一条已声明的 graft 切口点名拥有该文件的面。三条各自读拥有它的那份唯一实现——`build_evidence`
//! 的作用域/剪枝行与 graft 视图的 `names_face`——因此它不会与 `explain` 或 `grafts` 对同一个事实给出
//! 不同答案。

use std::path::Path;

use serde_json::Value;

use crate::build_time::{DeclaredGraft, declared_grafts};
use crate::mcp::callgraph::{caller_note, is_call_to};
use crate::mcp::source_index::{load_one, load_sources};

#[cfg(test)]
#[path = "why_tests.rs"]
mod why_tests;

/// How many doc lines of the contract the reply prints.
/// 答案打印多少行契约文档。
const CONTRACT_LINES: usize = 6;

/// The `#[cfg(…)]` attribute written directly above a definition, with its line.
/// 直接写在定义上方的 `#[cfg(…)]` 属性，及其行号。
///
/// Only the definition's own attributes are read, and the walk stops at the first line that is not a
/// doc comment or an attribute — so a gate sitting on the enclosing **module** is deliberately not
/// reported rather than guessed at, which the emitted line says.
/// 只读定义自己的属性，且走到第一行既不是文档注释也不是属性的地方就停——因此挂在**外层模块**上的门控
/// 有意不报、绝不靠猜，而输出的那一行会说清这一点。
fn gate(source: &str, definition: usize) -> Option<(usize, String)> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut index = definition.saturating_sub(1);
    while index > 0 {
        let above = lines[index - 1].trim();
        if above.starts_with("#[cfg") {
            return Some((index, above.to_owned()));
        }
        if above.starts_with("///") || above.starts_with("#[") {
            index -= 1;
            continue;
        }
        break;
    }
    None
}

/// The registration declaration a line falls inside, by macro name and line span.
/// 某一行落在里面的那个注册面声明（宏名与行区间）。
///
/// Measured need (W8 round): `why --at <face file>:<line>` with a line inside the `control_object!`
/// block answered "no function covers this line" and stopped, and the agent had to ask again with a
/// different line. Twice in one round, on two questions. The fact that the line is inside a
/// declaration is something this tree already knows — the kernel's parser carries the macro's whole
/// span — so the answer was withholding information it had, which is exactly the shape of waste the
/// maintainer named.
/// 量出来的需求（W8 那轮）：`why --at <面文件>:<行>` 给的行号落在 `control_object!` 块里时，答案只说
/// "no function covers this line" 就停了，agent 只能换个行号再问一次。一轮里两次、两道题。而"这一行在
/// 一个声明里"是这棵树**已经知道**的事实——内核解析器带着宏的整个区间——所以那次答案是扣下了自己手里的
/// 信息，这正是维护者点名的那种浪费。
fn declaration_at(source: &str, line: usize) -> Option<(String, usize, usize)> {
    let faces = nichlink_kernel::syntax::parse_faces(source).ok()?;
    faces
        .into_iter()
        .find(|face| face.location.line <= line && line <= face.end.line)
        .map(|face| (face.macro_name, face.location.line, face.end.line))
}

/// The plan facts a symptom at one definition depends on: scope, gate, and the declared cut.
/// 一个定义处的症状所依赖的计划事实：作用域、门控、以及已声明的切口。
///
/// Every branch answers rather than staying silent. A missing `source_scope.tsv` is "this tree was
/// never built", a file that no face owns is "the scope names faces, so this file's membership is not
/// decidable here", and an entry with no cut is "nothing replaces this face's slot" — each of which is
/// a fact about this tree, not an absence of an answer.
/// 每一条分支都作答而不是沉默。缺 `source_scope.tsv` 是"这棵树从未构建过"，没有面拥有的文件是"作用域
/// 点名的是面，因此这个文件在不在其中在这里不可判定"，入口没有任何切口是"没有东西替换这个面的槽位"
/// ——每一条都是关于这棵树的事实，而不是没有答案。
fn plan_facts(root: &Path, relative: &str, source: &str, definition: usize) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(match gate(source, definition) {
        Some((line, attribute)) => format!(
            "  gate       {attribute} at {relative}:{line} gates this definition (attributes on the \
             enclosing module are not read here)"
        ),
        None => format!(
            "  gate       no `#[cfg]` attribute sits directly above this definition at \
             {relative}:{definition} (a gate on the enclosing module is not read here)"
        ),
    });
    // The owning face is what makes the scope question decidable at all: the build's scope names
    // faces, so a file no face owns has no membership to report.
    // 拥有它的那个面才是让作用域问题可判定的东西：构建的作用域点名的是面，因此没有面拥有的文件没有
    // 可报的成员资格。
    let scope_relative = relative.strip_prefix("src/").unwrap_or(relative);
    let owning = crate::mcp::registry::namespace(root)
        .ok()
        .and_then(|namespace| crate::mcp::resolve::derived_faces(root, &namespace).ok())
        .and_then(|(faces, _)| {
            faces
                .into_iter()
                .find(|face| face.source.trim_start_matches("./") == scope_relative)
        });
    let evidence = crate::mcp::build_evidence::build_evidence(root);
    let Some(face) = owning else {
        lines.push(format!(
            "  scope      `{relative}` is not any registration face's own source, and the build's \
             scope names faces — so whether it ships is `check {{face}}`'s question rather than one \
             this line can answer"
        ));
        return lines;
    };
    lines.push(format!(
        "  scope      {}",
        crate::mcp::build_evidence::scope_line(evidence.scope.as_ref(), &face).trim_end()
    ));
    lines.push(format!(
        "  pruning    {}",
        crate::mcp::build_evidence::pruning_line(evidence.pruning.as_deref(), &face).trim_end()
    ));
    // Wiring: the host entry's own declarations, matched to this face by the graft view's rule
    // rather than by a second path comparison written here.
    // 接线：宿主入口自己的声明，按 graft 视图的规则匹配到这个面，而不是在这里另写一次路径比较。
    match declared_grafts(root) {
        Err(reason) => lines.push(format!(
            "  wiring     the host entry's declarations could not be read ({reason})"
        )),
        Ok(declared) if declared.cuts.is_empty() => lines.push(format!(
            "  wiring     the host entry {} declares no graft cut, so nothing replaces this face's \
             slot (`grafts` reads the same declarations)",
            declared.entry.display()
        )),
        Ok(declared) => {
            let named = declared
                .cuts
                .iter()
                .filter(|cut: &&DeclaredGraft| cut.names_face(&face.path, Some(&face.module)))
                .collect::<Vec<_>>();
            if named.is_empty() {
                lines.push(format!(
                    "  wiring     no declared cut in {} names `{}` — the entry declares {} cut(s), \
                     and a face no cut names is one the build replaces with nothing (`grafts` lists \
                     them)",
                    declared.entry.display(),
                    face.path,
                    declared.cuts.len()
                ));
            } else {
                for cut in named {
                    lines.push(format!(
                        "  wiring     declared cut `{}` → `{}` (full={}) at {}:{}",
                        cut.cut,
                        cut.graft,
                        cut.full,
                        declared.entry.display(),
                        cut.line
                    ));
                }
            }
        }
    }
    lines
}

/// Answer "what does this line depend on?" for a `path:line`.
/// 对 `路径:行号` 回答"这一行依赖什么"。
pub(crate) fn why(root: &Path, arguments: &Value) -> Result<String, String> {
    let at = arguments
        .get("at")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|at| !at.is_empty())
        .ok_or_else(|| {
            "why needs `at`: a `path:line` inside this tree — accepted shape: \
             {\"at\":\"<relative path>:<line>\",\"root\":\"<path>\"}; a registration node is \
             `explain {node}`'s question"
                .to_owned()
        })?;
    let Some((relative, line)) = at.rsplit_once(':') else {
        return Err(format!(
            "why needs `at` as `path:line`, got `{at}` — accepted shape: \
             {{\"at\":\"<relative path>:<line>\",\"root\":\"<path>\"}}"
        ));
    };
    let Ok(line) = line.trim().parse::<usize>() else {
        return Err(format!(
            "why needs a line number after the last colon, got `{at}` — accepted shape: \
             {{\"at\":\"<relative path>:<line>\",\"root\":\"<path>\"}}"
        ));
    };
    let file = load_one(root, relative.trim())?;
    let Some(function) = file
        .functions
        .iter()
        .find(|function| function.line <= line && line <= function.end_line)
    else {
        // A line inside a registration declaration is a **different answer**, not a miss: the
        // declaration is the thing that registers this face, and saying so costs one line while
        // leaving it out cost the round two extra calls.
        // 落在注册面声明里的行是**另一个答案**，不是没命中：那条声明正是注册这个面的东西，说出来只花一行，
        // 而不说让那一轮多花了两次调用。
        let inside = declaration_at(&file.source, line).map_or(String::new(), |(macro_name, start, end)| {
            format!(
                "this line is inside the `{macro_name}!` declaration spanning lines {start}-{end}, \
                 not inside a function — that declaration is what registers this face in the tree \
                 (its `kind` is a token in it), and `explain {{node}}` reports what it declares\n"
            )
        });
        return Ok(format!(
            "no function covers {}:{line} in {}\n{inside}the functions in this file are:\n{}\n",
            file.relative,
            root.display(),
            file.functions
                .iter()
                .map(|function| format!(
                    "  {} lines {}-{}",
                    function.name, function.line, function.end_line
                ))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    };

    let mut lines = vec![format!(
        "at {}:{} — the definition `{}` (lines {}-{})",
        file.relative, line, function.name, function.line, function.end_line
    )];
    let contract: Vec<(usize, String)> =
        crate::mcp::source_index::contract_lines(&file.source, function.line)
            .into_iter()
            .take(CONTRACT_LINES)
            .collect();
    if contract.is_empty() {
        lines.push("  contract   none: no `///` line sits above this definition".to_owned());
    } else {
        lines.push(
            "  contract   (from the doc above it, which is the promise this tree writes)"
                .to_owned(),
        );
        for (number, text) in &contract {
            lines.push(format!("             {}:{number} {text}", file.relative));
        }
    }

    // Who calls it — the same rule every other reader uses, so this cannot disagree with `callgraph`.
    // 谁调用它 —— 与其它读者同一条规则，因此它不会与 `callgraph` 给出不同答案。
    let sources = load_sources(root)?;
    let mut callers: Vec<String> = Vec::new();
    for source in &sources {
        for caller in &source.functions {
            if caller
                .calls
                .iter()
                .any(|call| is_call_to(call, &function.name))
            {
                let note = caller_note(&file.relative, &source.relative);
                let entry = format!("{}::{}", source.relative, caller.name);
                callers.push(match note {
                    Some(note) => format!("{entry} ({note})"),
                    None => entry,
                });
            }
        }
    }
    callers.sort();
    callers.dedup();
    if callers.is_empty() {
        lines.push("  callers    0 in this root".to_owned());
    } else {
        lines.push(format!("  callers    {}", callers.join(", ")));
    }

    // The adoption ledger, when this tree carries one: entries that name this file. The verdict is
    // `adopted`'s to compute; this says only which entries name the file, and says so.
    // 采信台账（这棵树带的话）：点名了这个文件的条目。判定归 `adopted` 算；这里只说哪些条目点了名。
    match crate::mcp::adopted::entries(root) {
        Err(error) => lines.push(format!("  adoption   ledger unreadable ({error})")),
        Ok(entries) => {
            // The parser is the kernel's (`parse_adoption`), reached through `adopted::entries`, so
            // this reader cannot drift from the verdict `adopted` prints. It says only which entries
            // name the file; the verdict is `adopted`'s to compute.
            // 解析器是内核那个（`parse_adoption`），经 `adopted::entries` 到达，因此这个读者不会与
            // `adopted` 打印的判定漂移。它只说哪些条目点名了这个文件；判定归 `adopted` 算。
            let naming: Vec<String> = entries
                .iter()
                .filter(|entry| {
                    entry
                        .files
                        .iter()
                        .any(|named| named.trim() == file.relative)
                })
                .map(|entry| entry.anchor.clone())
                .collect();
            if naming.is_empty() {
                lines.push(if entries.is_empty() {
                    "  adoption   no ledger at .nichlink/adopted/entries in this root".to_owned()
                } else {
                    format!(
                        "  adoption   {} entry(ies) in the ledger, none naming this file",
                        entries.len()
                    )
                });
            } else {
                lines.push(format!(
                    "  adoption   {} entry(ies) name this file — anchors: {} (read the verdict with \
                     `adopted`)",
                    naming.len(),
                    naming.join(", ")
                ));
            }
        }
    }

    // The plan half: the facts a symptom's *cause* often sits in, one or three hops away from the
    // code that shows it. Kept after the source facts and before the boundary, so the shape of the
    // reply stays "what this line is, who calls it, what the build did with it, what is not here".
    // 计划那一半：症状的**原因**常常住在这里，离显出症状的代码一到三跳。放在源码事实之后、边界之前，
    // 因此回复的形状仍是"这一行是什么、谁调用它、构建对它做了什么、这里没有什么"。
    lines.extend(plan_facts(root, &file.relative, &file.source, line));

    lines.push(
        "not covered here: how a grafted subtree looks at runtime (ask `trace`), and whether the \
         declarations above describe the tree **as it is now** — the scope and pruning lines are the \
         build's own output, and a tree built before the last edit says so on the `scope` line"
            .to_owned(),
    );
    lines.push(
        "next   `check {face}` to run the face that compiles it, `grafts` for every declared cut, \
         `locate {symptom}` for sibling places to compare it with"
            .to_owned(),
    );
    Ok(format!("{}\n", lines.join("\n")))
}
