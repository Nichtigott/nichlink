//! The family verdict a write reports: what the tree says about the face it just wrote, in the
//! **same** terms the comparison uses (audit `W5-6`).
//! 一次写入报告的家族判定：树对"它刚写下的那个面"说了什么，用的是**比对**同一套说法（审计 `W5-6`）。
//!
//! The acceptance this exists for is "零写后回查": a caller that has just created a face has to know
//! whether it landed **like its siblings**, and the cheapest place to say that is the reply to the
//! write itself. Asking `consistency` afterwards costs a second instrument call for a fact the write
//! path already had in hand.
//! 它为之存在的验收是"零写后回查"：刚创建一个面的调用方需要知道它是否**长得像它的兄弟**，而说这件事最便宜
//! 的地方就是写入自己的回复。事后去问 `consistency`，等于为一个写入路径本来就有的事实付第二次仪器调用。
//!
//! One rule, not two: the majority arithmetic is `consistency`'s own (`deviations`), and the shape
//! vocabulary is `consistency_support`'s. A second copy here is how "outlier" would come to mean two
//! different things in two answers about one tree.
//! 只有一条规则：多数派算术用的是 `consistency` 自己的（`deviations`），形状词汇用的是
//! `consistency_support` 的。在这里抄第二份，就是"outlier"在关于同一棵树的两份答案里开始有两个意思的来路。
//!
//! online: the verdict is about the tree **as the write just left it** — the face that was just
//! created is in no record at all, and its siblings may have changed since the build. A published
//! record could only answer about faces it has already seen.

use std::path::Path;

use crate::mcp::consistency::deviations;
use crate::mcp::consistency::{Wording, declared_shape, one_face};

/// The family verdict for one face's parent, as reply lines.
/// 某个面的父级那一家族的判定，按回复行给出。
///
/// Returns an empty vector when the face has no parent to compare under, when its parent has no other
/// child, or when the family cannot be derived — a verdict about a family of one is not a verdict.
/// 当这个面没有可比的父级、父级没有别的子面、或家族推导不出来时回空向量——对"只有自己一家"的判定不是判定。
/// The verdict as reply-ready text, so the report stays a report (and `apply.rs` stays under the
/// 600-code-line ratchet).
/// 判定按可直接写进回复的文本给出，好让报告保持是报告（也让 `apply.rs` 留在 600 行代码行的棘轮之内）。
pub(super) fn verdict_lines(work: &Path, namespace: &str, face_path: &str) -> String {
    let lines = family_verdict(work, namespace, face_path);
    if lines.is_empty() {
        return String::new();
    }
    format!("{}\n", lines.join("\n"))
}

pub(super) fn family_verdict(work: &Path, namespace: &str, face_path: &str) -> Vec<String> {
    let Some(parent) = face_path
        .trim_end_matches('/')
        .rsplit_once('/')
        .map(|(above, _)| above)
    else {
        return Vec::new();
    };
    let Ok(faces) = crate::mcp::resolve::derived_faces(work, namespace) else {
        return Vec::new();
    };
    let siblings: Vec<_> = faces
        .0
        .iter()
        .filter(|face| {
            face.path
                .trim_end_matches('/')
                .rsplit_once('/')
                .is_some_and(|(above, _)| above == parent)
        })
        .collect();
    if siblings.len() < 2 {
        return Vec::new();
    }
    // The shape signal, whose vocabulary a family contract is stated in: the field names each sibling
    // declares. The `api` signal compares call names and belongs to the comparison, not to a write.
    // 形状信号——家族契约就是用它的词汇陈述的：每个兄弟声明了哪些字段。`api` 信号比的是调用名，那属于比对，
    // 不属于一次写入。
    let mut sets = Vec::new();
    for face in &siblings {
        let text = std::fs::read_to_string(work.join("src").join(&face.source));
        let Ok(text) = text else {
            continue;
        };
        let Ok(parsed) = one_face(&text) else {
            continue;
        };
        let names = declared_shape(&parsed)
            .into_iter()
            .map(|field| field.spelling.to_owned())
            .collect::<std::collections::BTreeSet<_>>();
        let label = face
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&face.path)
            .to_owned();
        sets.push((label, names));
    }
    if sets.len() < 2 {
        return Vec::new();
    }
    let found = deviations(&sets, Wording::Declares);
    let mut lines = vec![format!(
        "family     {} sibling(s) under {parent}; outliers: {} of {}",
        sets.len(),
        found.len(),
        sets.len()
    )];
    for (sibling, notes) in found.iter().take(3) {
        lines.push(format!(
            "  outlier     {sibling}: {} — `consistency {{parent: \"{parent}\"}}` prints the fix",
            notes.join("; ")
        ));
    }
    lines
}
