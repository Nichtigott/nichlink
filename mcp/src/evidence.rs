//! The build's own evidence for one face, or for the tree it scoped.
//! 构建对某个面、或对它划定作用域的整棵树给出的证据。
//!
//! The read tools answer from source text; the build answers from
//! `target/nichlink/out`. The two can disagree — a file written a moment ago is
//! not yet in the tree the host compiled — and until this module existed only the
//! CLI's `explain` could tell them apart. It reads the build's own files through
//! `build_method`'s readers, so an agent can ask "is this face in the shipped
//! scope, and does pruning strip it" without a second derivation of its own.
//! 读工具用源码文本作答，构建用 `target/nichlink/out` 作答。两者可以不一致——刚写下的文件还不在
//! 宿主编译出的树里——而在这个模块出现之前，只有 CLI 的 `explain` 能把它们区分开。它经
//! `build_method` 的读取器读构建自己的文件，因此代理能直接问"这个面在发布作用域里吗、剪枝会不会
//! 剥掉它"，而不需要自己再做一份推导。
//!
//! What it deliberately does **not** report: declared graft state (`static_graft_plan!`
//! cuts stay in the `grafts` verb and the host entry), and contract/admission
//! fields (those live in built `RegistrationSnapshot`s and need a loaded registry).
//! 它有意**不**报告：声明的 graft 状态（`static_graft_plan!` 的切口仍归 `grafts` verb 与宿主
//! 入口），以及 contract/admission 字段（那些住在已构建的 `RegistrationSnapshot` 里，需要一个
//! 已加载的注册机）。

use std::path::{Path, PathBuf};

use nichlink_build_method::{
    BuildScopeView, FaceView, PruningRow, build_output_is_current, face_views, read_build_scope,
    read_pruning_manifest,
};
use serde_json::Value;

use crate::nodes::resolve_node;
use crate::protocol::DEFAULT_LIMIT;
use crate::registry::namespace;

/// The directory the build publishes its evidence into.
/// 构建发布其证据的目录。
pub(crate) fn out_dir(root: &Path) -> PathBuf {
    root.join("target/nichlink/out")
}

/// Report the build's evidence for one face, or for the whole scoped tree.
/// 报告构建对某个面的证据，或对整棵被划定作用域的树给出的证据。
///
/// `node` names one face by logical path or identity; omitting it reports the tree
/// projection. `limit` bounds the projection's rows, because a tree report is the
/// one answer here that grows with the project.
/// `node` 用逻辑路径或身份点名一个面；省略它则报告树的投影。`limit` 限制投影的行数，因为树报告是
/// 这里唯一随项目变大的答案。
pub(crate) fn explain(root: &Path, arguments: &Value) -> Result<String, String> {
    let namespace = namespace(root)?;
    let faces = face_views(root, &namespace)?;
    let out = out_dir(root);
    // A missing or stale build is not an error: it is the answer to "why does
    // this face not know whether it ships".
    // 缺失或过期的构建不是错误：它正是"这个面为什么不知道自己发不发布"的答案。
    let current = build_output_is_current(root, &out);
    let scope = read_build_scope(&out).ok();
    let pruning = read_pruning_manifest(&out).ok();
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize);
    match arguments.get("node").and_then(Value::as_str) {
        Some(target) => {
            let id = resolve_node(root, &namespace, target)?;
            let face = faces
                .iter()
                .find(|face| face.id == id)
                .ok_or_else(|| format!("no face in the derived tree has identity {id}"))?;
            Ok(node_report(
                &namespace,
                face,
                scope.as_ref(),
                pruning.as_deref(),
                current,
            ))
        }
        None => Ok(tree_report(
            &namespace,
            &faces,
            scope.as_ref(),
            pruning.as_deref(),
            current,
            limit,
        )),
    }
}

/// One face, plus what the build says about its scope and pruning.
/// 一个面，外加构建对其作用域与剪枝的说法。
fn node_report(
    namespace: &str,
    face: &FaceView,
    scope: Option<&BuildScopeView>,
    pruning: Option<&[PruningRow]>,
    current: bool,
) -> String {
    let mut output = format!(
        "namespace {namespace}\nnode {}\n  path {}\n  kind {}\n  source {}\n  module {}\n  parent {}{}\n  slot {}\n",
        face.id,
        face.path,
        face.kind,
        face.source,
        face.module,
        face.parent,
        if face.parent_resolved {
            ""
        } else {
            " (unresolved)"
        },
        face.registry_name,
    );
    output.push_str(&format!(
        "build {}\n",
        if current {
            "current"
        } else {
            "stale (run `nichlink check`)"
        }
    ));
    output.push_str(&scope_line(scope, face));
    output.push_str(&pruning_line(pruning, face));
    output
}

/// Whether the scope selected this face, and why.
/// 作用域是否选中了这个面，以及原因。
fn scope_line(scope: Option<&BuildScopeView>, face: &FaceView) -> String {
    let Some(scope) = scope else {
        return "scope unknown (no source_scope.tsv; run `nichlink check`)\n".to_owned();
    };
    if scope.all {
        return format!("scope selected (all=true, mode={})\n", scope.mode);
    }
    if scope.selected_ids.contains(&face.id) {
        return format!("scope selected (by id, mode={})\n", scope.mode);
    }
    if scope.selected_sources.contains(&face.source) {
        return format!("scope selected (by source, mode={})\n", scope.mode);
    }
    // Not selected is the interesting answer: this face is discovered but the
    // build's scope does not include it, so it is a pruning candidate rather than
    // a missing face.
    // "未选中"才是有意思的答案：这个面被发现了，但构建的作用域不含它，因此它是剪枝候选而不是
    // 一个缺失的面。
    format!("scope not-selected (mode={})\n", scope.mode)
}

/// What release pruning does to this face's symbols.
/// 发布剪枝对这个面的符号做了什么。
///
/// A manifest row whose symbol is `-` records a face with nothing tracked, so
/// "a row exists" is not "something is stripped" — the symbol has to be named.
/// 符号为 `-` 的清单行记录的是一个没有可跟踪符号的面，因此"行存在"不等于"有东西被剥掉"——必须点名
/// 那个符号。
fn pruning_line(pruning: Option<&[PruningRow]>, face: &FaceView) -> String {
    let Some(rows) = pruning else {
        return "pruning unknown (no pruning_manifest.tsv; run `nichlink check`)\n".to_owned();
    };
    let tracked: Vec<&str> = rows
        .iter()
        .filter(|row| row.id == face.id)
        .map(|row| row.symbol.as_str())
        .filter(|symbol| *symbol != "-")
        .collect();
    if !tracked.is_empty() {
        return format!("pruning strips {}\n", tracked.join(", "));
    }
    if rows.iter().any(|row| row.id == face.id) {
        return "pruning nothing to strip (the face has no tracked symbol)\n".to_owned();
    }
    "pruning nothing to strip\n".to_owned()
}

/// The whole tree the build saw, bounded.
/// 构建看到的那整棵树，有上限。
fn tree_report(
    namespace: &str,
    faces: &[FaceView],
    scope: Option<&BuildScopeView>,
    pruning: Option<&[PruningRow]>,
    current: bool,
    limit: usize,
) -> String {
    let mut output = format!("namespace {namespace}\nfaces {}\n", faces.len());
    output.push_str(&format!(
        "build {}\n",
        if current {
            "current"
        } else {
            "stale (run `nichlink check`)"
        }
    ));
    match scope {
        Some(scope) => output.push_str(&format!(
            "scope mode={} all={} reason={} selected_ids={} selected_sources={}\n",
            scope.mode,
            scope.all,
            scope.reason.as_deref().unwrap_or("-"),
            scope.selected_ids.len(),
            scope.selected_sources.len(),
        )),
        None => output.push_str("scope unknown (no source_scope.tsv; run `nichlink check`)\n"),
    }
    // Rows carry the scope verdict each, so the projection answers "what ships"
    // per face rather than only in aggregate.
    // 每一行都带自己的作用域结论，因此这个投影逐个面地回答"什么会发布"，而不只是给个总数。
    output.push_str("slots:\n");
    for face in faces.iter().take(limit) {
        let selected = scope.map_or("unknown", |scope| {
            if scope.all
                || scope.selected_ids.contains(&face.id)
                || scope.selected_sources.contains(&face.source)
            {
                "selected"
            } else {
                "not-selected"
            }
        });
        output.push_str(&format!(
            "  {:<40} {:<14} {:<10} {}\n",
            face.path, face.kind, selected, face.source
        ));
    }
    if faces.len() > limit {
        output.push_str(&format!(
            "  … +{} more (raise `limit`)\n",
            faces.len() - limit
        ));
    }
    match pruning {
        Some(rows) => {
            output.push_str(&format!("pruned {}\n", rows.len()));
            for row in rows.iter().take(limit) {
                output.push_str(&format!("  {} {} {}\n", row.id, row.source, row.symbol));
            }
            if rows.len() > limit {
                output.push_str(&format!("  … +{} more\n", rows.len() - limit));
            }
        }
        None => output.push_str("pruned unknown (no pruning_manifest.tsv; run `nichlink check`)\n"),
    }
    output
}

#[cfg(test)]
#[path = "evidence_tests.rs"]
mod evidence_tests;
