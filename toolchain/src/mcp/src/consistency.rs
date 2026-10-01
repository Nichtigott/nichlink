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

use nichlink_kernel::adoption::{AdoptionVerdict, verdict_of};
use nichlink_kernel::syntax::{FaceSyntax, parse_faces};
use serde_json::Value;

use crate::mcp::source_index::{SourceFile, load_sources};
use crate::mcp::workspace::{Member, Scope, Tree};

#[cfg(test)]
#[path = "consistency_tests.rs"]
mod consistency_tests;

/// How one shape field is compared between a specimen and a sibling.
/// 某个形状字段在标本与兄弟之间怎么比。
#[derive(Clone, Copy, PartialEq, Eq)]
enum ShapeComparison {
    /// Only whether the sibling declares the field at all: its value is this object's own name.
    /// 只比这个兄弟到底有没有声明该字段：它的取值就是这个对象自己的名字。
    Presence,
    /// The labels the specimen states must appear in the sibling's list. Extra labels are that
    /// sibling's own business, like everything else here — the comparison is one-directional.
    /// 标本写下的标签必须出现在兄弟的清单里。多出来的标签是那个兄弟自己的事，与这里其余部分一样——比较
    /// 是单向的。
    Labels,
}

/// The declared-shape fields a specimen is compared on, and how each one is compared.
/// 对比一个标本时看哪些已声明形状字段，以及每一项怎么比。
///
/// Every name here is a field name the kernel's own vocabulary carries (`FACE_FIELD_ORDER`), and the
/// values are read through the kernel's parser (`FaceSyntax`), so this list can name fields but
/// cannot invent or re-spell them. The two trait entries resolve the `*_contracts` spelling the same
/// way the build-time contract check does — a face that states a checked path does not state its
/// label a second time.
/// 这里的每个名字都是内核词表（`FACE_FIELD_ORDER`）携带的字段名，取值经内核解析器（`FaceSyntax`）读出，
/// 因此这张清单可以点名字段，却不能发明或改写它们。两条 trait 条目按构建期合同检查同一条规则解析
/// `*_contracts` 写法——写了受检路径的注册面不必再写一遍标签。
///
/// `parts` is presence-only because it names the object's **own** parts type (`ButtonParts`): two
/// siblings stating different ones is the design, not a drift, and comparing the value would make
/// every family an outlier.
/// `parts` 只比存在性，因为它点名的是对象**自己**的零件类型（`ButtonParts`）：两个兄弟写出不同的类型正是
/// 设计而非漂移，比值会让每一个同族都成了离群。
const SHAPE_FIELDS: &[(&str, ShapeComparison)] = &[
    ("parts", ShapeComparison::Presence),
    ("exports", ShapeComparison::Labels),
    ("handle_traits", ShapeComparison::Labels),
    ("part_traits", ShapeComparison::Labels),
];

/// The shape field names this comparison reads, for the lines that name the set.
/// 这次比较读哪些形状字段名，供点名该集合的行使用。
fn shape_field_names() -> Vec<&'static str> {
    SHAPE_FIELDS.iter().map(|(name, _)| *name).collect()
}

/// One field's rendered value as the labels it states.
/// 某个字段渲染后的取值，按它写下的标签拆开。
fn shape_labels(value: &str) -> Vec<&str> {
    value
        .split(", ")
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .collect()
}

/// The labels a trait field carries, from either spelling, or `None` when neither is written.
/// 某个 trait 字段携带的标签（两种写法都认），两者都没写时为 `None`。
fn trait_shape(face: &FaceSyntax, paths: &str, labels: &str) -> Option<String> {
    if face.field(paths).is_none() && face.field(labels).is_none() {
        return None;
    }
    face.string_list(labels)
        .map(|values| values.join(", "))
        .or_else(|| face.field(labels))
        .or_else(|| face.field(paths))
}

/// One parsed face's declared shape: the shape fields it carries, in the order of [`SHAPE_FIELDS`].
/// 一个已解析注册面的已声明形状：它携带的形状字段，按 [`SHAPE_FIELDS`] 的顺序。
fn declared_shape(face: &FaceSyntax) -> Vec<(String, String)> {
    let mut shape = Vec::new();
    for (name, _) in SHAPE_FIELDS {
        let value = match *name {
            "handle_traits" => trait_shape(face, "handle_contracts", "handle_traits"),
            "part_traits" => trait_shape(face, "part_contracts", "part_traits"),
            _ => face
                .string_list(name)
                .map(|values| values.join(", "))
                .or_else(|| face.field(name)),
        };
        if let Some(value) = value {
            shape.push(((*name).to_owned(), value));
        }
    }
    shape
}

/// One sibling's shape against the specimen's: what it lacks, and which stated labels it is missing.
/// 某个兄弟的形状对标本而言缺什么、以及缺了哪些已写下的标签。
///
/// One direction only, and that is the point: the specimen is the baseline, so the question is
/// whether it was followed, not whether a sibling invented something more. A sibling carrying a
/// field, or a label, that the specimen does not is therefore not a row here, and the boundary
/// sentence says so.
/// 只判一个方向，而这就是要点：标本是基准，因此问题是"有没有照它做"，不是"兄弟有没有多发明什么"。因此一个
/// 携带了标本没有的字段或标签的兄弟不在这里成行，而边界句会说清这一点。
fn shape_gaps(specimen: &[(String, String)], sibling: &[(String, String)]) -> Vec<String> {
    let mut notes = Vec::new();
    for (name, value) in specimen {
        let Some((_, other)) = sibling.iter().find(|(field, _)| field == name) else {
            notes.push(format!("lacks `{name}`"));
            continue;
        };
        let comparison = SHAPE_FIELDS
            .iter()
            .find(|(field, _)| field == name)
            .map(|(_, comparison)| *comparison);
        if comparison != Some(ShapeComparison::Labels) {
            continue;
        }
        let stated = shape_labels(other);
        for label in shape_labels(value) {
            if !stated.contains(&label) {
                notes.push(format!("lacks `{name}: {label}`"));
            }
        }
    }
    notes
}

/// The one face a ledger-named file declares, or why the shape could not be read from it.
/// 台账点名的一个文件所声明的那个注册面；读不出形状时给出原因。
///
/// A missing or multiple declaration is a reason, never an empty shape: "this file declares none of
/// these fields" and "this file's declaration could not be read" are different facts, and filling
/// the second with the first is how a comparison would silently report `0 deviations`.
/// 声明缺失或不止一个时给的是原因，绝不是一个空形状："这个文件没有声明这些字段"与"这个文件的声明读不出来"
/// 是两件不同的事实，而用前者去填后者，正是一份比较会静默报出 `0 deviations` 的来路。
fn one_face(text: &str) -> Result<FaceSyntax, String> {
    let mut faces = parse_faces(text).map_err(|error| error.to_string())?;
    match faces.len() {
        0 => Err("it declares no registration face".to_owned()),
        1 => Ok(faces.pop().expect("exactly one face")),
        count => Err(format!(
            "it declares {count} registration faces, so none was selected"
        )),
    }
}

/// One rendered shape, or the line that says there was none to read.
/// 渲染后的形状；无可读字段时给出那一行说明。
fn shape_line(shape: &[(String, String)]) -> String {
    if shape.is_empty() {
        return format!(
            "none of the fields this comparison reads ({})",
            shape_field_names().join(", ")
        );
    }
    shape
        .iter()
        .map(|(name, value)| format!("{name} `{value}`"))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// The indexed text of a face's own source file, when the index has it.
/// 某个面自己那个源文件的已索引文本（索引里有的话）。
fn source_text<'a>(sources: &'a [SourceFile], face_source: &str) -> Option<&'a str> {
    let wanted = format!("src/{}", face_source.trim_start_matches("./"));
    sources
        .iter()
        .find(|file| file.relative == wanted)
        .map(|file| file.source.as_str())
}

/// Compare the shape one ledger entry certifies against the shapes of its object's siblings.
/// 把台账某一条条目所采信的**形状**与它那个对象的兄弟逐个比对。
///
/// The baseline is the ledger's own choice, not this tool's: an adoption is a lease, so the entry in
/// force is the newest line for that anchor, and a lapsed one is reported as lapsed before anything
/// is compared — the bytes a person confirmed have moved, so what is compared below is today's
/// bytes and the answer says which of the two it read.
/// 基准是台账自己选的，不是本工具选的：采信是租约，因此生效的是该 anchor 的最后一条，而失效的那条会在
/// 比对之前先被报成失效——人确认过的字节已经动过，因此下面比的是今天的字节，而答案会说清它读的是哪一个。
fn specimen_comparison(root: &Path, anchor: &str) -> Result<String, String> {
    let entries = crate::mcp::adopted::entries(root)?;
    let history = entries
        .iter()
        .filter(|entry| entry.anchor == anchor)
        .collect::<Vec<_>>();
    let Some(effective) = history.last() else {
        return Ok(format!(
            "no ledger entry names `{anchor}` in {} — the ledger holds {} entry(ies), and a specimen \
             comparison is against the entry in force (an adoption is a lease: the newest line \
             wins). `adopted` lists the anchors.\n",
            root.display(),
            entries.len()
        ));
    };
    let current = crate::mcp::adopted::read_files(root, &effective.files)?;
    let mut lines = vec![format!(
        "specimen {anchor} — ledger revision {} is the one in force (an adoption is a lease: the \
         newest line wins)",
        history.len()
    )];
    match verdict_of(effective, &current) {
        AdoptionVerdict::Provisional => lines.push(format!(
            "state      provisional — certifies: {} (adopted {} by {})",
            effective.certifies, effective.at, effective.verifier
        )),
        AdoptionVerdict::Lapsed { file } => lines.push(format!(
            "state      lapsed at {file} — the bytes a person confirmed have moved, so the shape \
             below is today's bytes, not the certified ones, and this needs a **person** rather than \
             an edit (adopted {} by {})",
            effective.at, effective.verifier
        )),
    }
    lines.push(format!("covers     {}", effective.files.join(", ")));
    // The specimen's shape comes from the bytes the lease rests on, through the kernel's parser.
    // 标本的形状取自租约依托的那些字节，经内核的解析器读出。
    let mut shape = Vec::new();
    let mut unreadable = Vec::new();
    for (file, text) in &current {
        match one_face(text) {
            Ok(face) => {
                for (name, value) in declared_shape(&face) {
                    if !shape.contains(&(name.clone(), value.clone())) {
                        shape.push((name, value));
                    }
                }
            }
            Err(reason) => unreadable.push(format!("{file}: {reason}")),
        }
    }
    lines.push(format!("shape      {}", shape_line(&shape)));
    if !unreadable.is_empty() {
        lines.push(format!("unreadable {}", unreadable.join("; ")));
    }
    let parent = anchor
        .trim_end_matches('/')
        .rsplit_once('/')
        .map(|(above, _)| above.to_owned());
    let Some(parent) = parent else {
        lines.push(format!(
            "no siblings: `{anchor}` is a root path, and this comparison needs a parent to gather \
             the sibling set from"
        ));
        lines.extend(specimen_bounds());
        return Ok(format!("{}\n", lines.join("\n")));
    };
    let members = match crate::mcp::workspace::scope(root)? {
        Scope::Package(namespace) => vec![Member::package(root, namespace)],
        Scope::Workspace(members) => members,
        Scope::Unresolvable(reason) => {
            lines.push(format!("no siblings: {reason}"));
            lines.extend(specimen_bounds());
            return Ok(format!("{}\n", lines.join("\n")));
        }
    };
    let sources = load_sources(root)?;
    for member in &members {
        let faces = match member.tree()? {
            Tree::Derived { faces, unparsable } => {
                // Same reason as the parent mode: a file the derivation refused is missing from the
                // sibling set, and an answer that counted the set without saying so would read as
                // complete.
                // 理由与 parent 模式相同：推导拒绝掉的文件不在同族集合里，而一个不说明这点就清点集合的
                // 答案会被读成完整的。
                if !unparsable.trim().is_empty() {
                    lines.push(format!(
                        "member {}: {} (such a file is not in the sibling set below)",
                        member.name,
                        unparsable.trim_end()
                    ));
                }
                faces
            }
            Tree::Published(_) => {
                lines.push(format!(
                    "member {}: the tree comes from published records, which do not carry the \
                     declared fields this comparison reads — ask `registry --full` for the records, \
                     or derive the sources and ask again",
                    member.name
                ));
                continue;
            }
        };
        let siblings = siblings(faces, &parent)
            .into_iter()
            .filter(|face| face.path.trim_end_matches('/') != anchor.trim_end_matches('/'))
            .collect::<Vec<_>>();
        if siblings.is_empty() {
            lines.push(format!(
                "family {parent} · member {} · 0 sibling(s) (no other face sits directly under it)",
                member.name
            ));
            continue;
        }
        let mut rows: Vec<String> = Vec::new();
        let mut differing = 0usize;
        let mut unread = 0usize;
        for face in &siblings {
            let label = face
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&face.path)
                .to_owned();
            let sibling_shape = match source_text(&sources, &face.source) {
                Some(text) => match one_face(text) {
                    Ok(parsed) => declared_shape(&parsed),
                    Err(reason) => {
                        unread += 1;
                        rows.push(format!("  unreadable  {label}: {reason}"));
                        continue;
                    }
                },
                None => {
                    unread += 1;
                    rows.push(format!(
                        "  unreadable  {label}: its source `{}` is not in this tree's index",
                        face.source
                    ));
                    continue;
                }
            };
            let gaps = shape_gaps(&shape, &sibling_shape);
            if gaps.is_empty() {
                continue;
            }
            differing += 1;
            rows.push(format!("  outlier     {label}: {}", gaps.join("; ")));
        }
        lines.push(format!(
            "family {parent} · member {} · {} sibling(s)",
            member.name,
            siblings.len()
        ));
        lines.extend(rows);
        lines.push(format!(
            "conformance: {differing} of {} sibling(s) do not carry the specimen's shape{}",
            siblings.len(),
            if unread == 0 {
                String::new()
            } else {
                format!(" ({unread} could not be read and are counted as neither)")
            }
        ));
    }
    lines.extend(specimen_bounds());
    Ok(format!("{}\n", lines.join("\n")))
}

/// What the specimen comparison does not read, and the next call it points at.
/// 标本比对该读不到什么，以及它指向的下一次调用。
fn specimen_bounds() -> Vec<String> {
    vec![
        "not covered by this comparison: it reads the shape the ledger's files declare (parts, \
         exports, handle_traits, part_traits) through the kernel's face parser and compares each \
         sibling's own file against it, so a shape stated in a shared helper, produced by a macro \
         this tree does not spell, or carried only by runtime behaviour is invisible; `parts` is \
         compared by presence alone because it names each object's own parts type, while each label \
         the specimen states in `exports`, `handle_traits` and `part_traits` has to appear in the \
         sibling's list — a label the sibling states and the specimen does not is that sibling's own \
         business; a sibling that declares **more** fields than the specimen is not reported, \
         because the specimen is the baseline and the question is whether it was followed; and a \
         deviation is a place to look, not a defect — whether the design should still be this way is \
         the ledger's own question"
            .to_owned(),
        "next   `read {path, line}` for a deviating sibling's body, `conformance {anchor}` for the \
         ledger's own verdict on this specimen"
            .to_owned(),
    ]
}

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
    // Two questions, one sibling set. `parent` asks "do these children agree with each other";
    // `specimen` asks "do they follow the shape the ledger certifies" — the baseline there is the
    // entry in force rather than the majority, which is why they are separate modes rather than one
    // flag on the other.
    // 两个问题，同一份同族集合。`parent` 问"这些子级彼此一致吗"；`specimen` 问"它们照台账采信的那个形状
    // 做了吗"——后者的基准是生效条目而不是多数派，因此它们是两种模式，而不是一个模式上的开关。
    if let Some(specimen) = arguments
        .get("specimen")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|specimen| !specimen.is_empty())
    {
        if arguments.get("parent").is_some() {
            return Err(
                "consistency takes `parent` or `specimen`, not both: `parent` compares siblings \
                 against their own majority, while `specimen` derives the parent from the anchor and \
                 compares them against the shape the ledger certifies — accepted shape: \
                 {\"specimen\":\"<anchor>\",\"root\":\"<path>\"}"
                    .to_owned(),
            );
        }
        return specimen_comparison(root, specimen);
    }
    let parent = arguments
        .get("parent")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|parent| !parent.is_empty())
        .ok_or_else(|| {
            "consistency needs `parent` or `specimen`: `parent` is the logical path whose children \
             to compare, `specimen` is a ledger anchor to compare its siblings against — accepted \
             shape: {\"parent\":\"<logical path>\",\"by\":\"api|kind|source\",\"root\":\"<path>\"} \
             or {\"specimen\":\"<anchor>\",\"root\":\"<path>\"}"
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
            Tree::Derived { faces, unparsable } => {
                // A file the derivation refused is not in the sibling set, so a comparison that
                // stayed silent about it would answer "these are all the siblings" about a tree that
                // dropped one. The drop is the tree's own line; it is carried here rather than
                // re-derived.
                // 推导拒绝掉的文件不在同族集合里，因此对这件事保持沉默的比较，会对着一个**丢掉了一个
                // 文件**的树答出"兄弟就这些"。丢掉的实情是树自己那一行，这里只是搬运，不重新推导。
                if !unparsable.trim().is_empty() {
                    sections.push(format!(
                        "member {}: {} (such a file is not in the sibling set below)",
                        member.name,
                        unparsable.trim_end()
                    ));
                }
                faces
            }
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
