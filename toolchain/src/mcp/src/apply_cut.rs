//! The `cut` action: hand one face's subtree over by writing the declaration the plan reads.
//! `cut` 动作：把一个面的子树交出去，写进计划读的那条声明。
//!
//! Why this is its own module: `apply.rs` is a ratcheted file (600 code lines), and this action grew
//! past it. Splitting on the action keeps the whole write path in one place — the request shape, the
//! kernel renderer, the read-back, and the impact — rather than scattering it.
//! 为什么单独一个模块：`apply.rs` 是受棘轮约束的文件（600 代码行），而这个动作让它超了。按动作拆分把整条写
//! 路径留在一处——请求形状、内核渲染器、读回、影响面——而不是把它们散开。

use std::path::Path;

use serde_json::Value;

use crate::build_time::face_views;
use crate::mcp::apply::Outcome;

/// Hand one face's subtree over: write the declaration the build-time plan reads.
/// 把一个面的子树交出去：写进构建期计划读的那条声明。
///
/// **Writes a declaration, never a plan from scratch.** The file is the one `declared_grafts`
/// already reads (`declared.entry`), the spelling comes from the kernel's own renderer
/// (`render_graft_expression`, so the two directions of this grammar live in one place), and the
/// result is **parsed back with the kernel's parser** before it is written: a write that does not
/// read back as exactly one more entry is refused, because what this puts in a host is source code.
/// **只写一条声明，不从零造计划。** 文件就是 `declared_grafts` 已经在读的那一个（`declared.entry`），
/// 拼写来自内核自己的渲染器（`render_graft_expression`——这套语法的两个方向因此住在一处），而写盘**之前**会
/// 用内核的解析器**读回来**：读回来不是"恰好多了条目"就拒绝——因为写进宿主的是源码。
///
/// The impact it reports is the faces the cut's target covers: the build prunes the subtree a cut
/// names, so those are the faces this application stops shipping. The match is on the module path the
/// expression writes (`crate::control::object::button::NODE_ID` ⇒ `control/object/button`), and the
/// build is still the authority — `check` on the preview confirms it.
/// 它报的影响面是切口目标覆盖的那些面：构建会剪掉切口命名的子树，因此那些面就是本应用不再发布的面。匹配依据
/// 是表达式写出的模块路径（`crate::control::object::button::NODE_ID` ⇒ `control/object/button`），而权威
/// 仍是构建本身——在预览上跑 `check` 即可确认。
pub(crate) fn run_cut(root: &Path, arguments: &Value) -> Result<Outcome, String> {
    let text = |key: &str| {
        arguments
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| format!("`cut` requires `{key}`"))
    };
    let cut = text("cut")?;
    let graft = text("graft")?;
    let cut_end = arguments
        .get("to")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let full = arguments.get("full").and_then(Value::as_bool) == Some(true);
    let entry =
        nichlink_kernel::syntax::entries::render_graft_expression(&cut, cut_end, full, &graft)
            .map_err(|error| format!("this cut cannot be written: {}", error.message))?;

    let declared = crate::build_time::declared_grafts(root).map_err(|error| {
        format!(
            "this tree's graft plan is unreadable, so there is nothing to add a cut to ({error}); \
             the plan is the `static_graft_plan!` / `graft_plan!` call in the host entry"
        )
    })?;
    let plan = declared.entry.clone();
    let source = std::fs::read_to_string(&plan)
        .map_err(|error| format!("{} is not readable: {error}", plan.display()))?;
    let edited = insert_plan_entry(&source, &entry)?;
    let before = nichlink_kernel::syntax::entries::graft_entries(&source)
        .map_err(|error| {
            format!(
                "the plan does not parse before the write: {}",
                error.message
            )
        })?
        .len();
    let after = nichlink_kernel::syntax::entries::graft_entries(&edited)
        .map_err(|error| format!("the written plan does not parse: {}", error.message))?;
    if after.len() != before + 1 {
        return Err(format!(
            "the write did not produce exactly one more declaration ({before} ⇒ {}), so it was \
             refused rather than left in the host",
            after.len()
        ));
    }
    let covered = covered_faces(root, &cut);
    std::fs::write(&plan, &edited)
        .map_err(|error| format!("{} is not writable: {error}", plan.display()))?;
    Ok(Outcome {
        // The direction matters and I had it backwards once (round-9 review caught it by running the
        // thing): the build's scope **narrows to** the subtrees the cuts name, so a cut is what makes
        // a subtree part of what this application publishes — without it, a declared face is not
        // shipped at all. Saying "stops shipping" describes the opposite of what the write does.
        // 方向要紧，而我说反过一次（第九轮复核靠"跑一遍"抓到）：构建期作用域**收窄到**这些切口命名的子树，
        // 因此切口正是"让这棵子树进入本应用所发布之物"的那一步——没有它，声明了的面根本不会被发布。说成
        // "stops shipping" 描述的是这次写入的反面。
        message: format!(
            "wrote one declaration to {}\n  {entry}\nthe build's scope narrows **to** the subtrees \
             these cuts name, so this cut is what makes {} part of what this application publishes \
             (a declared face no cut names is not shipped); the build is the authority, so run \
             `check` to confirm",
            plan.display(),
            if covered.is_empty() {
                "no face whose module path this expression names (the build decides what the \
                 narrowed scope contains)"
                    .to_owned()
            } else {
                covered.join(", ")
            }
        ),
        declaration: None,
        source: plan,
        moved: false,
        // A cut has one reading: hand this subtree over. `deepen` is the action whose reply has to
        // state the other reading of its request.
        // 切口只有一种读法：把这棵子树交出去。需要说出请求的另一种读法的是 `deepen`。
        alternative: false,
    })
}

/// Insert one entry into a plan call, just before the parenthesis that closes it.
/// 把一条条目插进计划调用里，紧挨着结束它的那个括号之前。
///
/// The scan tracks parenthesis depth from the macro name, so it lands inside the call the entries
/// live in rather than at the first `);` in the file. It does not try to be a Rust lexer: a plan
/// call whose arguments contain unbalanced parentheses in a string or comment is refused, which is
/// the safe direction for something that rewrites a host's source.
/// 扫描从宏名起跟踪括号深度，因此落点在那条调用内部，而不是文件里第一个 `);`。它不去充当 Rust 词法器：若计划
/// 调用的实参里出现字符串或注释中的不配对括号，它**拒绝**——对一个改写宿主源码的动作来说，这是安全的方向。
fn insert_plan_entry(source: &str, entry: &str) -> Result<String, String> {
    let (name_at, name) = ["static_graft_plan!", "graft_plan!"]
        .iter()
        .find_map(|name| source.find(name).map(|at| (at, *name)))
        .ok_or_else(|| {
            "this host has no `static_graft_plan!` / `graft_plan!` call to add a cut to".to_owned()
        })?;
    let open = name_at + name.len();
    let mut depth = 0usize;
    let mut close = None;
    for (offset, ch) in source[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    close = Some(open + offset);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close.ok_or_else(|| {
        "this host's plan call has unbalanced parentheses, so this tool will not guess where its \
         entries end"
            .to_owned()
    })?;
    // The head is trimmed before the entry is appended: the source already ends its last entry with
    // a newline, so keeping it and adding another would leave a blank line the author never wrote —
    // a write that reformats the file is a write whose diff is about formatting rather than the cut.
    // 追加之前先把头部去掉尾部空白：源码本来就在最后一条条目后带换行，留着它再加一个就会多出作者从没写过的空行
    // ——重排文件的写入，其 diff 讲的是排版而不是那个切口。
    let head = source[..close].trim_end();
    let mut edited = String::with_capacity(source.len() + entry.len() + 8);
    edited.push_str(head);
    if !head.ends_with(',') {
        edited.push(',');
    }
    edited.push('\n');
    edited.push_str("    ");
    edited.push_str(entry);
    edited.push('\n');
    edited.push_str(&source[close..]);
    Ok(edited)
}

/// The faces a cut expression's module path covers, named by their node paths.
/// 切口表达式的模块路径覆盖的那些面，按它们的节点路径点名。
fn covered_faces(root: &Path, cut: &str) -> Vec<String> {
    let Some(module) = cut
        .strip_prefix("crate::")
        .and_then(|rest| rest.strip_suffix("::NODE_ID"))
    else {
        return Vec::new();
    };
    let wanted = module.replace("::", "/");
    let Ok(namespace) = crate::mcp::registry::namespace(root) else {
        return Vec::new();
    };
    let Ok(views) = face_views(root, &namespace) else {
        return Vec::new();
    };
    let mut covered: Vec<String> = views
        .iter()
        .filter(|view| {
            let dir = view
                .source
                .rsplit_once('/')
                .map_or(view.source.as_str(), |(dir, _)| dir);
            dir == wanted || dir.starts_with(&format!("{wanted}/"))
        })
        .map(|view| view.path.clone())
        .collect();
    covered.sort();
    covered
}
