//! `nichlink explain`: why a face is present, absent, or replaced.
//! `nichlink explain`：一个注册面为何存在、缺席或被替换。
//!
//! This command answers operator questions about a host the CLI cannot link:
//! what a node is, why the build (not) ships it, and which declared graft cuts
//! name it. Every rule it applies is read from the build's own surface —
//! `face_views` for the tree and identities, `declared_grafts` for the cuts,
//! `read_build_scope`/`read_pruning_manifest` for the published build output —
//! so the command cannot disagree with the build about what it is describing.
//! 本命令回答 CLI 无法链接的宿主的运维问题：节点是什么、构建为何（不）发布它、
//! 哪些已声明的 graft 切口命名了它。它应用的每条规则都读自构建自身的表面——
//! 树与身份取 `face_views`，切口取 `declared_grafts`，已发布的构建产物取
//! `read_build_scope`/`read_pruning_manifest`——因此命令不可能与构建对它描述的对象
//! 产生分歧。

use std::io::Write;

use nichlink::identity::NodeId;
use nichlink_build_method::{FaceView, face_views};
use serde_json::json;

use super::{build_out_dir, resolve_package};

// Split decision: `explain` has three output shapes that change independently —
// the per-node report (`report`), the whole-tree `--overlay` projection
// (`overlay`), and the JSON serializer both share (`json`). Each is mounted here
// with `#[path]` per the workspace rule, and this page keeps only argv parsing,
// the routing decision, and the query resolver they all depend on.
// 拆分决定：`explain` 有三种独立变化输出形状——逐节点报告（`report`）、整树
// `--overlay` 投影（`overlay`），以及两者共用的 JSON 序列化器（`json`）。各自按
// 工作区规则用 `#[path]` 在此挂载；本页只保留 argv 解析、路由决定，以及三者都依赖的
// 查询解析器。
#[path = "explain_json.rs"]
pub(crate) mod json;
#[path = "explain_overlay.rs"]
mod overlay;
#[path = "explain_report.rs"]
mod report;

/// One operator question: resolve a node, then report everything the build
/// knows about it.
/// 一次运维提问：解析一个节点，然后报告构建关于它知道的一切。
pub(crate) fn explain(
    args: &mut impl Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut json_output = false;
    let mut overlay = false;
    let mut directory: Option<String> = None;
    let mut target: Option<String> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" => json_output = true,
            "--overlay" => overlay = true,
            "--path" => {
                directory = Some(args.next().ok_or("--path requires a directory")?);
            }
            _ if arg.starts_with('-') => return Err(format!("unexpected argument '{arg}'")),
            _ if target.is_none() => target = Some(arg),
            _ => return Err("explain accepts at most one node id or logical path".to_owned()),
        }
    }
    let directory = directory.unwrap_or_else(|| ".".to_owned());
    let (manifest, package) = resolve_package(&directory)?;
    let faces = face_views(&manifest, &package)?;
    if overlay {
        if target.is_some() {
            return Err(
                "--overlay renders the whole effective tree; drop the node id or path".to_owned(),
            );
        }
        return overlay::overlay_report(&manifest, &faces, json_output, out);
    }
    let query = target.ok_or("explain requires a node id or a logical path")?;
    let face = match resolve_face(&faces, &query) {
        Ok(face) => face,
        Err(reason) => {
            if json_output {
                let report = json!({
                    "schema": "nichlink.explain/1",
                    "query": query,
                    "resolved": false,
                    "reason": reason,
                    "candidate_count": faces.len(),
                });
                writeln!(out, "{}", json::render_json(&report)).map_err(json::write_error)?;
            } else {
                writeln!(out, "unresolved: {reason}").map_err(json::write_error)?;
            }
            return Err(reason);
        }
    };
    let out_dir = build_out_dir(&manifest);
    // Published output is trusted only while it still describes these sources; see
    // `build_output_is_current` for why the fingerprint is the token.
    // 已发布的产物只在仍然描述这批源码时才被信任；为什么指纹是那枚凭据见
    // `build_output_is_current`。
    let current = nichlink_build_method::build_output_is_current(&manifest, &out_dir);
    let (scope_json, scope_note) = report::scope_report(&out_dir, face, current);
    let (pruning_json, pruning_note) = report::pruning_report(&out_dir, face, current);
    let (grafts_json, graft_note) = report::declared_report(&manifest, face);
    let kept = scope_json["kept"].as_bool();

    if json_output {
        let report = json!({
            "schema": "nichlink.explain/1",
            "query": query,
            "resolved": true,
            "kept": kept,
            "node": {
                "id": face.id.to_string(),
                "path": face.path,
                "kind": face.kind,
                "registry_name": face.registry_name,
                "module": face.module,
                "source": face.source,
                "owns_registry": face.owns_registry,
                "parent": {
                    "id": face.parent.to_string(),
                    "path": report::parent_path(&faces, face.parent),
                    "resolved": face.parent_resolved,
                },
                "identity_inputs": {
                    "namespace": package,
                    "relative_source": face.source,
                    "kind": face.kind,
                },
            },
            "scope": scope_json,
            "pruning": pruning_json,
            "declared_grafts": grafts_json,
        });
        writeln!(out, "{}", json::render_json(&report)).map_err(json::write_error)?;
    } else {
        writeln!(out, "node {}", face.id).map_err(json::write_error)?;
        writeln!(out, "  path: {}", face.path).map_err(json::write_error)?;
        writeln!(out, "  kind: {}", face.kind).map_err(json::write_error)?;
        writeln!(out, "  source: {}", face.source).map_err(json::write_error)?;
        writeln!(out, "  module: {}", face.module).map_err(json::write_error)?;
        writeln!(
            out,
            "  identity: namespace={package} relative-source={} kind={}",
            face.source, face.kind
        )
        .map_err(json::write_error)?;
        writeln!(
            out,
            "  parent: {} ({}){}",
            report::parent_path(&faces, face.parent),
            face.parent,
            if face.parent_resolved {
                ""
            } else {
                "  [unresolved: the declaration names a parent this tree does not have]"
            }
        )
        .map_err(json::write_error)?;
        writeln!(
            out,
            "  registry: owns_registry={} slot={}",
            face.owns_registry, face.registry_name
        )
        .map_err(json::write_error)?;
        for line in scope_note
            .iter()
            .chain(pruning_note.iter())
            .chain(graft_note.iter())
        {
            writeln!(out, "{line}").map_err(json::write_error)?;
        }
    }
    Ok(())
}

/// Resolve the argument the operator wrote: a 32-hex node identity, or the
/// logical path a runtime tree would print.
/// 解析操作者写下的参数：32 位十六进制节点身份，或运行期树会打印的逻辑路径。
///
/// The failure message lists the paths the tree does have, because "not found"
/// without the alternatives makes an operator guess at spelling.
/// 失败消息列出树确实拥有的路径，因为只报"找不到"会让操作者去猜拼写。
fn resolve_face<'a>(faces: &'a [FaceView], query: &str) -> Result<&'a FaceView, String> {
    if let Ok(id) = query.parse::<NodeId>() {
        return faces
            .iter()
            .find(|face| face.id == id)
            .ok_or_else(|| format!("no registration face has node id `{query}`"));
    }
    if let Some(face) = faces.iter().find(|face| face.path == query) {
        return Ok(face);
    }
    let mut paths = faces
        .iter()
        .map(|face| face.path.as_str())
        .collect::<Vec<_>>();
    paths.sort_unstable();
    let shown = paths.iter().take(8).copied().collect::<Vec<_>>().join(", ");
    Err(format!(
        "no registration face has logical path `{query}`; the tree has {} face(s): {shown}",
        paths.len()
    ))
}
