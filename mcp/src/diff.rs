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

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use nichlink_build_method::{
    build_output_is_current, declared_grafts, face_views, graft_plan_rows, read_pruning_manifest,
};
use serde_json::Value;

use crate::evidence::out_dir;
use crate::protocol::DEFAULT_LIMIT;
use crate::registry::namespace;

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
    let faces = face_views(root, &namespace)?;
    if arguments.get("records").and_then(Value::as_bool) == Some(true) {
        return diff_records(root, &faces, &namespace, arguments);
    }
    let out = out_dir(root);
    let current = build_output_is_current(root, &out);
    let Ok(built) = read_pruning_manifest(&out) else {
        return Ok(
            "no build evidence: run `nichlink check` (or `nichlink build`) first — a tree diff needs \
             the built side, and this project has never published one.\n"
                .to_owned(),
        );
    };
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize);
    let source_ids: BTreeSet<_> = faces.iter().map(|face| face.id).collect();
    let source_sources: BTreeSet<_> = faces.iter().map(|face| face.source.as_str()).collect();
    let built_ids: BTreeSet<_> = built.iter().map(|row| row.id).collect();
    let built_by_source: HashMap<_, _> = built
        .iter()
        .map(|row| (row.source.as_str(), row.id))
        .collect();

    let mut output = format!(
        "namespace {namespace}\nbuild {}\nfaces {} (source) vs {} (build)\n",
        if current {
            "current"
        } else {
            "stale (run `nichlink check`)"
        },
        faces.len(),
        built.len(),
    );
    // A face whose source is new is added; one the build has but the sources no
    // longer declare is gone.
    // 源码新出现的面是 added；构建有而源码不再声明的是 gone。
    let added: Vec<_> = faces
        .iter()
        .filter(|face| {
            !built_ids.contains(&face.id) && !built_by_source.contains_key(face.source.as_str())
        })
        .collect();
    let gone: Vec<_> = built
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
        .filter_map(|face| {
            built_by_source
                .get(face.source.as_str())
                .filter(|previous| **previous != face.id)
                .map(|previous| (face, *previous))
        })
        .collect();
    output.push_str(&format!(
        "added {}  gone {}  reidentified {}\n",
        added.len(),
        gone.len(),
        reidentified.len()
    ));
    if added.is_empty() && gone.is_empty() && reidentified.is_empty() {
        output.push_str("the build matches the sources face for face\n");
        return Ok(output);
    }
    output.push_str("added:\n");
    for face in added.iter().take(limit) {
        output.push_str(&format!("  + {} {} {}\n", face.path, face.kind, face.id));
    }
    output.push_str("gone:\n");
    for row in gone.iter().take(limit) {
        output.push_str(&format!("  - {} ({})\n", row.source, row.id));
    }
    output.push_str("reidentified:\n");
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
) -> Result<String, String> {
    let out = out_dir(root);
    let current = build_output_is_current(root, &out);
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize);
    let declared = declared_grafts(root);
    let rows = graft_plan_rows(root, faces, declared.as_ref().ok())?;
    let mut ok = Vec::new();
    let mut reidentified = Vec::new();
    let mut stale = Vec::new();
    let mut unmatched = Vec::new();
    let mut unreadable = 0usize;
    for row in &rows {
        let Some(target) = row.target else {
            unreadable += 1;
            continue;
        };
        if faces.iter().any(|face| face.id == target) {
            ok.push(row);
            continue;
        }
        let Some(path) = row.target_path.as_deref() else {
            stale.push(row);
            continue;
        };
        match faces.iter().find(|face| face.path == path) {
            Some(face) => reidentified.push((row, face.id)),
            // A typed cut stores a Rust expression instead of a logical path, so an absent
            // identity cannot be told apart from a re-identified one: saying "stale" there
            // would be a guess, and the reply says which case it is.
            // 类型化切口存的是 Rust 表达式而不是逻辑路径，因此身份缺席时无法与"身份变了"区分：
            // 在那里说"stale"就是猜，而回复会写明这是哪一种。
            None if path.contains("::") => unmatched.push(row),
            None => stale.push(row),
        }
    }
    let mut output = format!(
        "namespace {namespace}\nbuild {}\nrecords {} (external graft plans)\n",
        if current {
            "current"
        } else {
            "stale (run `nichlink check`)"
        },
        rows.len()
    );
    output.push_str(&format!(
        "ok {}  stale {}  re-identified {}  unmatched {}  unreadable {unreadable}\n",
        ok.len(),
        stale.len(),
        reidentified.len(),
        unmatched.len()
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
    if !unmatched.is_empty() {
        output.push_str("unmatched (a typed cut stores an expression, so an absent identity cannot be told from a re-identified one):\n");
        for row in unmatched.iter().take(limit) {
            output.push_str(&format!(
                "  ? {} -> {}\n",
                row.selector,
                row.target_path.as_deref().unwrap_or("-")
            ));
        }
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
