//! `nichlink grafts`: inspect the external graft plans on disk.
//! `nichlink grafts`：检视磁盘上的外部 graft 计划。
//!
//! A plan under `.nichlink/external-grafts/` is an authoring record the build
//! never opens; the build only warns when the host entry's `static_graft_plan!`
//! does not declare the slot (see `build_method/src/graft_plan_check.rs`). That
//! warning is easy to miss in a long `cargo` log, so this command makes the same
//! decision inspectable on demand, using the kernel's `GraftPlanDocument` parser
//! and the build's `declared_grafts`/`names_face` rather than a second copy of
//! either rule. Read-only: it opens files and writes nothing.
//! `.nichlink/external-grafts/` 下的计划是构建从不打开的创作记录；只有当宿主入口的
//! `static_graft_plan!` 没有声明该槽位时构建才警告（见
//! `build_method/src/graft_plan_check.rs`）。在冗长的 `cargo` 日志里这条警告容易被
//! 漏掉，因此本命令用内核的 `GraftPlanDocument` 解析器和构建的
//! `declared_grafts`/`names_face` 让同一判断可按需检视，而不是各写第二份规则。
//! 只读：只打开文件，不写任何东西。

use std::io::Write;
use std::path::Path;

use nichlink::lexicon;
use nichlink::plugin::graft_document::GraftPlanDocument;
use nichlink_build_method::{DeclaredGrafts, FaceView, declared_grafts, face_views};
use serde_json::{Value, json};

// The JSON error document and the command error text are one rule each; the
// `explain` page owns both, so this command cannot drift from it.
// JSON 错误文档与命令错误文本各是一条规则；它们归 `explain` 页面所有，因此本命令不会
// 与它漂移。
use crate::explain::json::{render_json, write_error};

use super::resolve_package;

/// List every plan under `.nichlink/external-grafts/*/graft.plan`.
/// 列出 `.nichlink/external-grafts/*/graft.plan` 下的每个计划。
pub(crate) fn grafts(
    args: &mut impl Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut json_output = false;
    let mut directory: Option<String> = None;
    for arg in args.by_ref() {
        match arg.as_str() {
            "--json" => json_output = true,
            _ if arg.starts_with('-') => return Err(format!("unexpected argument '{arg}'")),
            _ if directory.is_none() => directory = Some(arg),
            _ => return Err("grafts accepts at most one path".to_owned()),
        }
    }
    let directory = directory.unwrap_or_else(|| ".".to_owned());
    let (manifest, package) = resolve_package(&directory)?;
    let faces = face_views(&manifest, &package).unwrap_or_default();
    let declared = declared_grafts(&manifest);
    let rows = plan_rows(&manifest, &faces, declared.as_ref().ok());
    let entry = declared
        .as_ref()
        .map(|declared| declared.entry.display().to_string())
        .ok();
    let entry_error = declared.as_ref().err().cloned();

    if json_output {
        let report = json!({
            "schema": "nichlink.grafts/1",
            "entry": entry,
            "entry_error": entry_error,
            "plans": rows,
        });
        writeln!(out, "{}", render_json(&report)).map_err(write_error)?;
        return Ok(());
    }

    match (&entry, &entry_error) {
        (Some(entry), _) => writeln!(out, "host entry: {entry}").map_err(write_error)?,
        (None, Some(error)) => {
            writeln!(out, "host entry: unreadable ({error})").map_err(write_error)?
        }
        (None, None) => {}
    }
    if rows.is_empty() {
        writeln!(
            out,
            "no external graft plans under .nichlink/external-grafts/"
        )
        .map_err(write_error)?;
        return Ok(());
    }
    for row in &rows {
        match row.get("error").and_then(Value::as_str) {
            Some(error) => writeln!(
                out,
                "{}: unreadable ({error})",
                row["selector"].as_str().unwrap_or("<unknown>")
            )
            .map_err(write_error)?,
            None => {
                let declared = match row["declared"].as_bool() {
                    Some(true) => "declared",
                    Some(false) => "NOT declared by the host entry",
                    None => "declaration unknown",
                };
                writeln!(
                    out,
                    "{}: target={} graft={} full={} [{declared}]",
                    row["selector"].as_str().unwrap_or("<unknown>"),
                    row["target_path"].as_str().unwrap_or("<unknown>"),
                    row["graft"].as_str().unwrap_or("<unknown>"),
                    row["full"]
                )
                .map_err(write_error)?;
                if let Some(cut) = row["declared_by"].as_object() {
                    // Render the string values, not the `serde_json::Value`s:
                    // `Value`'s `Display` is JSON, so a quoted `"button_fast"`
                    // would reach a terminal that is not reading JSON.
                    // 渲染字符串值而不是 `serde_json::Value`：`Value` 的 `Display`
                    // 是 JSON，会把带引号的 `"button_fast"` 送到并不读 JSON 的终端。
                    writeln!(
                        out,
                        "    declared at line {} as cut `{}` graft `{}`",
                        cut["line"],
                        cut["cut"].as_str().unwrap_or("<unknown>"),
                        cut["graft"].as_str().unwrap_or("<unknown>")
                    )
                    .map_err(write_error)?;
                }
            }
        }
    }
    Ok(())
}

/// Build one report row per plan directory, sorted by selector.
/// 为每个计划目录生成一行报告，按 selector 排序。
///
/// A directory whose plan is missing or unparseable becomes a row carrying the
/// reason, not a silent skip: an operator running this command is asking exactly
/// whether the file is usable.
/// 计划缺失或解析不了的目录会成为携带原因的条目，而不是被静默跳过：运行本命令的
/// 操作者问的正是"这个文件能不能用"。
pub(crate) fn plan_rows(
    manifest: &Path,
    faces: &[FaceView],
    declared: Option<&DeclaredGrafts>,
) -> Vec<Value> {
    let directory = manifest
        .join(lexicon::NICHLINK_DIR)
        .join(lexicon::EXTERNAL_GRAFT_DIR);
    let Ok(entries) = std::fs::read_dir(&directory) else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let selector = entry.file_name().to_string_lossy().into_owned();
        let plan = entry.path().join(lexicon::GRAFT_PLAN_FILE);
        let text = match std::fs::read_to_string(&plan) {
            Ok(text) => text,
            Err(error) => {
                rows.push(json!({
                    "selector": selector,
                    "error": format!("cannot read {}: {error}", plan.display()),
                    "target": Value::Null,
                    "target_path": Value::Null,
                    "graft": Value::Null,
                    "full": Value::Null,
                    "declared": Value::Null,
                    "declared_by": Value::Null,
                }));
                continue;
            }
        };
        let document = match GraftPlanDocument::parse(&text) {
            Ok(document) => document,
            Err(error) => {
                rows.push(json!({
                    "selector": selector,
                    "error": error.to_string(),
                    "target": Value::Null,
                    "target_path": Value::Null,
                    "graft": Value::Null,
                    "full": Value::Null,
                    "declared": Value::Null,
                    "declared_by": Value::Null,
                }));
                continue;
            }
        };
        // A typed declaration names the base face by module, so the plan's
        // stored NodeId is mapped back through the same face rows the build's
        // tree walk produced. A target the current tree does not have stays
        // `None`, and only a string cut can then prove the declaration.
        // 类型化声明用模块命名基面，因此计划里存的 NodeId 会经构建树行走产生的同一批
        // 面行映射回去。当前树没有的目标保持 `None`，此时只有字符串切口能证明该声明。
        let module = faces
            .iter()
            .find(|face| face.id == document.target)
            .map(|face| face.module.as_str());
        let matched = declared.and_then(|declared| {
            declared
                .cuts
                .iter()
                .find(|cut| cut.names_face(&document.target_path, module))
        });
        let declared_state = match (declared, matched) {
            (None, _) => Value::Null,
            (Some(_), Some(_)) => Value::Bool(true),
            (Some(_), None) => Value::Bool(false),
        };
        let declared_by = matched.map(|cut| {
            json!({
                "cut": match &cut.cut_end {
                    Some(end) => format!("{} to {end}", cut.cut),
                    None => cut.cut.clone(),
                },
                "graft": cut.graft,
                "full": cut.full,
                "line": cut.line,
            })
        });
        rows.push(json!({
            "selector": selector,
            "error": Value::Null,
            "target": document.target.to_string(),
            "target_path": document.target_path,
            "graft": document.graft,
            "full": document.full,
            "declared": declared_state,
            "declared_by": declared_by,
        }));
    }
    rows.sort_by(|left, right| {
        left["selector"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["selector"].as_str().unwrap_or_default())
    });
    rows
}
