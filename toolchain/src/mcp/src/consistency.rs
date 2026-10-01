//! `nichlink.consistency`: compare siblings under one parent, and name the ones that differ.
//! `nichlink.consistency`：比较同一父级下的兄弟，点名不一样的那几个。
//!
//! The failure this answers is the one the maintainer named "供应链驳杂": one object under a parent
//! follows a different convention from its siblings — a `button` computing offsets in a relative
//! space while `slider` and `timeline` use the world's — and every file is locally plausible, so
//! reading them one at a time does not show it. The sibling set is a fact only a registration tree
//! has; the comparison below is text (which names each file calls), and it says so.
//! 这条回答的失败正是维护者说的"供应链驳杂"：同一父级下有一个对象跟它的兄弟用了不同的约定 —— 比如
//! `button` 在相对坐标里算偏移，而 `slider` 与 `timeline` 用世界坐标 —— 每个文件单看都自洽，逐个读发现
//! 不了。**同族集合**是只有注册树才有的事实；下面这套比较读的是文本（每个文件调用了哪些名字），并且
//! 它自己说了这一点。
//!
//! It covers the **derived** tree (the normal case for a development checkout). When a member's tree
//! comes from published records instead, the reply says so and names where the fields do live rather
//! than filling them with defaults.
//! 它覆盖**推导**出来的树（开发检出的常态）。当成员的树来自发布记录时，答案会明说，并点名字段实际住在
//! 哪里，而不是拿默认值把空填上。

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

use crate::mcp::source_index::load_sources;
use crate::mcp::workspace::{Member, Scope, Tree};

#[cfg(test)]
#[path = "consistency_tests.rs"]
mod consistency_tests;

/// The sibling set: faces directly under one logical path, in tree order.
/// 同族集合：直接挂在某个逻辑路径下的面，按树序。
fn siblings<'a>(
    faces: &'a [crate::build_time::FaceView],
    parent: &str,
) -> Vec<&'a crate::build_time::FaceView> {
    let wanted = parent.trim_end_matches('/');
    faces
        .iter()
        .filter(|face| {
            face.path
                .rsplit_once('/')
                .is_some_and(|(above, _)| above == wanted)
        })
        .collect()
}

/// Which siblings differ from the majority, and what they differ by.
/// 哪些兄弟与多数派不同，以及差在哪。
///
/// Decidable, and stated as such: a name **most** siblings call and this one does not; a name this
/// one calls and **no** other sibling calls. The majority line is more than half.
/// 可判定，而且就这么说：**多数**兄弟都调、而它没调的名字；以及它调了、**别的兄弟都没调**的名字。
/// 多数线取"超过一半"。
fn deviations(sets: &[(String, BTreeSet<String>)]) -> Vec<(String, Vec<String>)> {
    let total = sets.len();
    let mut shared: Vec<(String, usize)> = Vec::new();
    let mut seen: BTreeSet<&String> = BTreeSet::new();
    for (_, names) in sets {
        seen.extend(names.iter());
    }
    for name in seen {
        let count = sets
            .iter()
            .filter(|(_, names)| names.contains(name))
            .count();
        if count * 2 > total {
            shared.push((name.clone(), count));
        }
    }
    let mut rows = Vec::new();
    for (sibling, names) in sets {
        let mut notes: Vec<String> = Vec::new();
        for (name, _) in &shared {
            if !names.contains(name) {
                notes.push(format!(
                    "does not call `{name}`, which the other siblings call"
                ));
            }
        }
        for name in names {
            let count = sets
                .iter()
                .filter(|(_, other)| other.contains(name))
                .count();
            if count == 1 {
                notes.push(format!("calls `{name}`, which no sibling calls"));
            }
        }
        if !notes.is_empty() {
            rows.push((sibling.clone(), notes));
        }
    }
    rows
}

/// Answer "which of these siblings differs?" for one parent.
/// 对一个父级回答"这些兄弟里谁不一样"。
pub(crate) fn consistency(root: &Path, arguments: &Value) -> Result<String, String> {
    let parent = arguments
        .get("parent")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|parent| !parent.is_empty())
        .ok_or_else(|| {
            "consistency needs `parent`: the logical path whose children to compare — accepted \
             shape: {\"parent\":\"<logical path>\",\"by\":\"api|kind|source\",\"root\":\"<path>\"}"
                .to_owned()
        })?;
    let by = arguments
        .get("by")
        .and_then(Value::as_str)
        .unwrap_or("api")
        .to_owned();
    if !matches!(by.as_str(), "api" | "kind" | "source") {
        return Err(format!(
            "consistency takes `by` as `api`, `kind` or `source`, got `{by}` — accepted shape: \
             {{\"parent\":\"<logical path>\",\"by\":\"api|kind|source\",\"root\":\"<path>\"}}"
        ));
    }
    let members = match crate::mcp::workspace::scope(root)? {
        Scope::Package(namespace) => vec![Member::package(root, namespace)],
        Scope::Workspace(members) => members,
        Scope::Unresolvable(reason) => {
            return Ok(format!(
                "{reason}\nno faces to compare: the tree could not be read\n"
            ));
        }
    };
    let sources = load_sources(root)?;
    let mut sections: Vec<String> = Vec::new();
    for member in &members {
        let faces = match member.tree()? {
            Tree::Derived { faces, .. } => faces,
            Tree::Published(_) => {
                sections.push(format!(
                    "member {}: the tree comes from published records, which do not carry the \
                     declared fields this comparison reads — ask `registry --full` for the records, \
                     or derive the sources and ask again",
                    member.name
                ));
                continue;
            }
        };
        let set = siblings(faces, parent);
        if set.is_empty() {
            sections.push(format!(
                "family {parent} · member {} · 0 members (no face sits directly under it)",
                member.name
            ));
            continue;
        }
        let mut rows: Vec<String> = vec![format!(
            "family {parent} · member {} · {} member(s)",
            member.name,
            set.len()
        )];
        let mut sets: Vec<(String, BTreeSet<String>)> = Vec::new();
        let mut values: Vec<(String, String)> = Vec::new();
        for face in &set {
            let label = face
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&face.path)
                .to_owned();
            let value = match by.as_str() {
                "kind" => face.kind.clone(),
                "source" => face.source.clone(),
                _ => {
                    // The `api` signal: the names this face's own file calls, read from the same
                    // index every other reader uses.
                    // `api` 信号：这个面自己的文件调用了哪些名字，取自其它读者用的同一份索引。
                    // "This sibling's own text" is the whole object directory, not only the face
                    // file: the face file of a framework object is often one macro invocation, and
                    // a macro body's text is deliberately not read as calls (the kernel's own rule).
                    // "这个兄弟自己的文本"是整个对象目录，而不只是那个面文件：框架对象的面文件常常只有一次
                    // 宏调用，而宏体的文本按内核自己的规则**不算调用**。
                    let directory = format!("src/{}", face.source.trim_start_matches("./"));
                    let directory = directory
                        .rsplit_once('/')
                        .map(|(dir, _)| dir.to_owned())
                        .unwrap_or(directory);
                    let names: BTreeSet<String> = sources
                        .iter()
                        .filter(|source| {
                            let path = format!("src/{}", source.relative.trim_start_matches("./"));
                            path.starts_with(&format!("{directory}/"))
                        })
                        .flat_map(|source| source.functions.iter())
                        .flat_map(|function| function.calls.iter().cloned())
                        .collect();
                    let shown = names.iter().cloned().collect::<Vec<_>>().join(", ");
                    sets.push((label.clone(), names));
                    rows.push(format!(
                        "  {label:<24} {} call(s): {}",
                        shown.split(", ").filter(|part| !part.is_empty()).count(),
                        if shown.is_empty() {
                            "none".to_owned()
                        } else {
                            shown
                        }
                    ));
                    continue;
                }
            };
            rows.push(format!("  {label:<24} {value}"));
            values.push((label.clone(), value.clone()));
        }
        let mut outliers = 0usize;
        if by == "api" {
            let found = deviations(&sets);
            for (sibling, notes) in &found {
                rows.push(format!("  outlier     {sibling}: {}", notes.join("; ")));
            }
            outliers = found.len();
        } else {
            let mut counts: std::collections::BTreeMap<&str, usize> =
                std::collections::BTreeMap::new();
            for (_, value) in &values {
                *counts.entry(value.as_str()).or_default() += 1;
            }
            let majority = counts
                .iter()
                .max_by_key(|(value, count)| (**count, std::cmp::Reverse(value.len())))
                .map(|(value, _)| *value)
                .unwrap_or_default();
            for (sibling, value) in &values {
                if value != majority {
                    // Ties are not outliers: with no single majority value there is nothing to call
                    // a deviation.
                    // 平局不算离群：没有唯一的多数值时，就没有可称为偏离的东西。
                    if counts
                        .values()
                        .filter(|count| **count == counts[value.as_str()])
                        .count()
                        > 1
                        && counts.values().max() == counts.get(value.as_str())
                    {
                        continue;
                    }
                    rows.push(format!(
                        "  outlier     {sibling}: `{by}` is `{value}` while the majority says `{majority}`"
                    ));
                    outliers += 1;
                }
            }
        }
        rows.push(format!("outliers: {outliers} of {}", sets.len()));
        sections.push(rows.join("\n"));
    }
    sections.push(
        "not covered by this comparison: it reads the **derived** tree's sibling set and each \
         sibling's own text, so a convention that lives in a shared helper, in generated code, or \
         in a parent rule is not visible here; and `api` compares the names called, not the units \
         or the arithmetic — an outlier is a place to look, not a defect; and a call written inside \
         a macro body is not read as a call (the kernel's rule), so an object whose whole body is one \
         macro invocation reads as calling nothing"
            .to_owned(),
    );
    sections.push(
        "next   `read {path, line}` for the outlier's body, `explain {node}` for its declared fields"
            .to_owned(),
    );
    Ok(format!("{}\n", sections.join("\n")))
}
