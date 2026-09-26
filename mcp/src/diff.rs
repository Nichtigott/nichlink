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

use nichlink_build_method::{build_output_is_current, face_views, read_pruning_manifest};
use serde_json::Value;

use crate::evidence::out_dir;
use crate::protocol::DEFAULT_LIMIT;
use crate::registry::namespace;

/// Report the face-level delta between the source tree and the build's manifest.
/// 报告源码树与构建清单之间的面级差异。
pub(crate) fn diff(root: &Path, arguments: &Value) -> Result<String, String> {
    let namespace = namespace(root)?;
    let faces = face_views(root, &namespace)?;
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

#[cfg(test)]
#[path = "diff_tests.rs"]
mod diff_tests;
