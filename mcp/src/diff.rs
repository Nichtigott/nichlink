//! What the tree says now versus what the build saw.
//! 树现在说的，与构建当时看到的。
//!
//! A text diff answers "which lines moved"; this answers "which *faces* appeared,
//! disappeared, or changed identity" — the unit the registry actually ships. It
//! compares the source-derived tree against the build's own manifest, so the two
//! sides are the same derivations everything else in this bridge reports, and an
//! unbuilt project gets told to build rather than getting an empty diff.
//! 文本 diff 回答"哪些行动了"；这里回答"哪些**面**出现、消失或换了身份"——注册机真正发布的单位。
//! 它把源码推导出的树与构建自己的清单对照，因此两侧都是这个桥其它部分报告的同一批推导；没有构建过的
//! 项目会被要求先构建，而不是拿到一份空 diff。

use std::collections::BTreeSet;
use std::path::Path;

use nichlink_build_method::{GraftPlanRow, declared_grafts, graft_plan_rows};
use serde_json::Value;

use crate::build_evidence::build_evidence;
use crate::protocol::DEFAULT_LIMIT;
use crate::registry::namespace;
use crate::tree_delta::{FaceStatus, TreeDelta};

/// Report the face-level delta between two sides of this package.
/// 报告本包两侧之间的面级差异。
///
/// Two comparisons live here because they answer the same shape of question about two
/// different pairs. The default compares the **sources** against the build's manifest — what
/// changed since the build published evidence. `records: true` compares the **external graft
/// records** against the sources: a record stores the identity it was written for, so a face
/// that changed identity under an unmoved slot silently breaks it, and no text diff sees
/// that. Both borrow the same vocabulary (added/gone/re-identified) on purpose.
/// 这里有两种比较，因为它们对两对不同的东西问的是同一种形状的问题。默认把**源码**与构建清单对照
/// ——自构建发布证据以来变了什么。`records: true` 把**外部 graft 记录**与源码对照：记录里存着它
/// 写下时针对的身份，因此槽位没动而面换了身份会**悄悄**弄坏它，而文本 diff 看不见这件事。两者有意
/// 共用同一套词汇（added/gone/re-identified）。
pub(crate) fn diff(root: &Path, arguments: &Value) -> Result<String, String> {
    let namespace = namespace(root)?;
    let (faces, unparsable) = crate::resolve::derived_faces(root, &namespace)?;
    if arguments.get("records").and_then(Value::as_bool) == Some(true) {
        return diff_records(root, &faces, &namespace, arguments, &unparsable);
    }
    // The built side and its per-face verdicts come from one rule
    // (`crate::tree_delta`), which `nichlink.search` reads too: the same face must
    // not be `added` here and something else there.
    // 构建那一侧与逐面的结论来自一条规则（`crate::tree_delta`），`nichlink.search` 也读它：
    // 同一个面不能在这里是 `added`、在那里是别的。
    let built = TreeDelta::read(root);
    if !built.known {
        return Ok(
            "no build evidence: run `nichlink check` (or `nichlink build`) first — a tree diff needs \
             the built side, and this project has never published one.\n"
                .to_owned(),
        );
    }
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize);
    let source_ids: BTreeSet<_> = faces.iter().map(|face| face.id).collect();
    let source_sources: BTreeSet<_> = faces.iter().map(|face| face.source.as_str()).collect();

    let mut output = format!(
        "namespace {namespace}\n{unparsable}build {}\nfaces {} (source) vs {} (build)\n",
        // The freshness word comes from the one place that spells it. `built.current`
        // answers the same question through the same rule (`build_output_is_current`)
        // but spells nothing, so reading the word from here is what keeps this report
        // and the others from drifting apart (the wording used to be inlined here and
        // in `overlay`/`converge`; an independent check's mutation `E` found that
        // nothing coupled the copies).
        // 新鲜度词来自唯一拼它的地方。`built.current` 经同一条规则（`build_output_is_current`）
        // 回答同一个问题，但不拼任何词；因此从这里取词，正是让本报告与其余报告不会漂移的原因
        // （这份词形过去在本文件与 `overlay`/`converge` 各内联一次；独立复核的变异 `E` 发现
        // 没有任何钉子把副本耦合起来）。
        build_evidence(root).freshness(),
        faces.len(),
        built.rows.len(),
    );
    // A face whose source is new is added; one the build has but the sources no
    // longer declare is gone.
    // 源码新出现的面是 added；构建有而源码不再声明的是 gone。
    let added: Vec<_> = faces
        .iter()
        .filter(|face| built.status(face) == FaceStatus::AddedSinceBuild)
        .collect();
    let gone: Vec<_> = built
        .rows
        .iter()
        .filter(|row| {
            !source_ids.contains(&row.id) && !source_sources.contains(row.source.as_str())
        })
        .collect();
    // Same source, different identity: the face's `kind` (an identity input)
    // changed under a file that did not move.
    // 源码相同、身份不同：文件没动，而面的 `kind`（身份输入之一）变了。
    let reidentified: Vec<_> = faces
        .iter()
        .filter_map(|face| match built.status(face) {
            FaceStatus::Reidentified(previous) => Some((face, previous)),
            FaceStatus::Ok | FaceStatus::AddedSinceBuild => None,
        })
        .collect();
    // The bucket words come from `FaceStatus::label`'s side of the tree
    // vocabulary, not from this file: the same face is annotated `ok`, `added
    // since build`, or `re-identified` by `nichlink.search`, and a count line that
    // spelled them its own way would make the two tools disagree about the word
    // even though they share the rule.
    // 桶名取自这棵树的词汇里 `FaceStatus::label` 那一侧，而不是本文件：同一个面会被
    // `nichlink.search` 标注成 `ok`、`added since build` 或 `re-identified`，一条自己另拼的
    // 计数行会让两个工具虽然共用规则、却在词形上说不到一起。
    output.push_str(&format!(
        "{} {}  gone {}  {} {}\n",
        FaceStatus::ADDED_SINCE_BUILD,
        added.len(),
        gone.len(),
        FaceStatus::REIDENTIFIED,
        reidentified.len()
    ));
    if added.is_empty() && gone.is_empty() && reidentified.is_empty() {
        output.push_str("the build matches the sources face for face\n");
        return Ok(output);
    }
    output.push_str(&format!("{}:\n", FaceStatus::ADDED_SINCE_BUILD));
    for face in added.iter().take(limit) {
        output.push_str(&format!("  + {} {} {}\n", face.path, face.kind, face.id));
    }
    output.push_str("gone:\n");
    for row in gone.iter().take(limit) {
        output.push_str(&format!("  - {} ({})\n", row.source, row.id));
    }
    output.push_str(&format!("{}:\n", FaceStatus::REIDENTIFIED));
    for (face, previous) in reidentified.iter().take(limit) {
        output.push_str(&format!("  ~ {} {} -> {}\n", face.path, previous, face.id));
    }
    Ok(output)
}

/// Report how the external graft records stand against the source tree.
/// 报告外部 graft 记录相对源码树的状况。
fn diff_records(
    root: &Path,
    faces: &[nichlink_build_method::FaceView],
    namespace: &str,
    arguments: &Value,
    unparsable: &str,
) -> Result<String, String> {
    // The records report used to spell the freshness word itself; it comes from the one
    // place that spells it now, which is also why this call reads the evidence rather
    // than only the fingerprint.
    // 记录报告过去自己拼新鲜度词；现在它来自唯一拼它的地方，这也是这里读整份证据、而不只是读
    // 指纹的原因。
    let freshness = build_evidence(root).freshness();
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize);
    let declared = declared_grafts(root);
    let rows = graft_plan_rows(root, faces, declared.as_ref().ok())?;
    let mut ok = Vec::new();
    let mut reidentified = Vec::new();
    let mut stale = Vec::new();
    let mut undeclared: Vec<(
        &GraftPlanRow,
        Option<bool>,
        Option<nichlink::identity::NodeId>,
    )> = Vec::new();
    let mut unreadable = 0usize;
    for row in &rows {
        let Some(target) = row.target else {
            unreadable += 1;
            continue;
        };
        if faces.iter().any(|face| face.id == target) {
            // The identity being in the tree is only half the answer. A record no
            // `static_graft_plan!` cut names is pruned by the release and skipped at runtime, which
            // is what `nichlink.grafts` reports as "[NOT declared by the host entry]" and counts
            // under `unkept plans N` — and what the build refuses outright. Calling it `ok` here
            // gave one record two health conclusions, and the agent that read the first one would
            // ship it (audit `M1`). The row already carries whether a declaration named it, so the
            // split costs nothing.
            // 身份在树里只是答案的一半。没有任何 `static_graft_plan!` 切口点名的记录会被发布剪掉、运行期
            // 跳过——这正是 `nichlink.grafts` 报成 "[NOT declared by the host entry]" 并计入
            // `unkept plans N`、构建直接拒绝的那种。在这里称它 `ok` 会让同一条记录有两个健康结论，而读到
            // 前一个的代理会把它发出去（审计 `M1`）。该行本就携带"是否有声明点名它"，分流不需额外代价。
            match row.declared {
                Some(true) => ok.push(row),
                // `Some(false)`: read, and no cut names this slot. `None`: the host entry could not
                // be read, so nothing here can tell — both are "not known to be kept", and the
                // bucket says which one it is per row rather than flattening them.
                // `Some(false)`：读到了，但没有切口点名这个槽位。`None`：宿主入口读不了，因此这里无从
                // 判断——两者都属于"不知道它被保住"，而这个桶逐行说明是哪一种，而不是把它们抹平。
                other => undeclared.push((row, other, None)),
            }
            continue;
        }
        let Some(path) = row.target_path.as_deref() else {
            stale.push(row);
            continue;
        };
        let moved_to = faces
            .iter()
            .find(|face| face.path == path)
            .map(|face| face.id);
        // The declaration question does not depend on the identity being in the tree: a
        // slot no `static_graft_plan!` cut names is pruned by the release either way.
        // Asking it only in the branch above made this tool answer `undeclared 0` and
        // `re-identified 1` for exactly the record `nichlink.grafts` reports as
        // `[NOT declared by the host entry]` / `unkept plans 1` — two opposite
        // recommendations for one record, and the wrong one is the optimistic one
        // (audit `LGC-LG-12`). A record that is both unkept and drifted is reported as
        // unkept, with where its path points now.
        // 声明那边的问题与身份在不在树里无关：没有任何 `static_graft_plan!` 切口点名的槽位两种情形下都
        // 会被发布剪掉。只在上面的分支里问它，会让本工具对**恰恰是** `nichlink.grafts` 报成
        // `[NOT declared by the host entry]` / `unkept plans 1` 的那条记录回答 `undeclared 0` 与
        // `re-identified 1`——同一条记录两个相反的建议，而错的那个是乐观的那个（审计 `LGC-LG-12`）。
        // 既没被保住、又漂移了的记录报成"没被保住"，并附上它的路径现在指向哪里。
        if row.declared != Some(true) {
            undeclared.push((row, row.declared, moved_to));
            continue;
        }
        match moved_to {
            Some(now) => reidentified.push((row, now)),
            // There used to be an `unmatched` bucket here for "the identity is absent but the path
            // looks like a Rust expression (`::`)", because a typed *declaration* names its target
            // with an expression rather than a logical path. No plan writer ever puts an expression
            // in the plan's `target_path` — both write `registry.path_for` — and when the identity
            // is absent nothing can resolve a typed declaration's module, so `names_face` matches
            // only string cuts there. The bucket therefore described an input that cannot occur,
            // while its heading claimed to describe a real case (audit `m1`). A record naming
            // something this tree has not got is `stale`, and saying so is not a guess: the reply
            // also prints the path it looked for.
            // 这里曾有一个 `unmatched` 桶，用于"身份缺席、而路径看起来像 Rust 表达式（`::`）"的情形，
            // 因为类型化的**声明**是用表达式而不是逻辑路径命名目标的。但没有任何计划写入方会把表达式
            // 写进计划的 `target_path`——两处都写 `registry.path_for`——而身份缺席时没有任何东西能解析
            // 类型化声明的模块，因此 `names_face` 在那种情况下只可能匹配字符串切口。于是这个桶描述的
            // 是一个不可能出现的输入，标题却声称描述真实情形（审计 `m1`）。点名了本树没有的东西的记录
            // 就是 `stale`，这么说也不是猜：回复同时打印出它查找的那个路径。
            None => stale.push(row),
        }
    }
    let mut output = format!(
        "namespace {namespace}\n{unparsable}build {freshness}\nrecords {} (external graft plans)\n",
        rows.len()
    );
    output.push_str(&format!(
        "ok {}  undeclared {}  stale {}  re-identified {}  unreadable {unreadable}\n",
        ok.len(),
        undeclared.len(),
        stale.len(),
        reidentified.len()
    ));
    output.push_str("ok:\n");
    for row in ok.iter().take(limit) {
        output.push_str(&format!(
            "  {} -> {} {}\n",
            row.selector,
            row.target_path.as_deref().unwrap_or("-"),
            row.target
                .map(|target| target.to_string())
                .unwrap_or_default()
        ));
    }
    if !undeclared.is_empty() {
        output.push_str(
            "undeclared (the release prunes these slots, so the record can never take effect):\n",
        );
        for (row, declared, moved_to) in undeclared.iter().take(limit) {
            let why = match declared {
                Some(false) => "no cut in the host entry names it",
                _ => "the host entry could not be read, so nothing can tell",
            };
            let movement = moved_to.map_or_else(String::new, |now| format!(" -> {now}"));
            output.push_str(&format!(
                "  ! {} -> {}{movement}  ({why})\n",
                row.selector,
                row.target_path.as_deref().unwrap_or("-"),
            ));
        }
    }
    output.push_str("stale:\n");
    for row in stale.iter().take(limit) {
        output.push_str(&format!(
            "  - {} -> {} (no face in this tree has that identity or that path)\n",
            row.selector,
            row.target_path.as_deref().unwrap_or("-")
        ));
    }
    output.push_str("re-identified:\n");
    for (row, now) in reidentified.iter().take(limit) {
        output.push_str(&format!(
            "  ~ {} {} -> {now}  ({})\n",
            row.selector,
            row.target
                .map(|target| target.to_string())
                .unwrap_or_default(),
            row.target_path.as_deref().unwrap_or("-")
        ));
    }
    output.push_str(
        "detail: nichlink.grafts (which slots the host entry declares) · nichlink.explain (this \
         face's build evidence) · nichlink.verify (re-run the kernel and report the tree delta)\n",
    );
    Ok(output)
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod diff_tests;
