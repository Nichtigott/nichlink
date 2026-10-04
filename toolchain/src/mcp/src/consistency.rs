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
//!
//! online: the shape comparison compares declarations that may have been added since the build, and a shape is a property of the source text.

use std::collections::BTreeSet;
use std::path::Path;

use nichlink_kernel::adoption::{AdoptionVerdict, verdict_of};
use nichlink_kernel::syntax::{FaceSyntax, parse_faces};
use serde_json::Value;

use crate::mcp::source_index::{SourceFile, load_sources};
use crate::mcp::workspace::{Member, Scope};

#[path = "consistency_support.rs"]
mod support;

pub(crate) use support::tree_census;

use support::*;

#[cfg(test)]
#[path = "consistency_tests.rs"]
mod consistency_tests;

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
    let mut remaining: Vec<String> = Vec::new();
    let mut differing = 0usize;
    let mut unread = 0usize;
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
                for field in declared_shape(&face) {
                    let seen = shape.iter().any(|kept: &DeclaredField| {
                        kept.key == field.key && kept.value == field.value
                    });
                    if !seen {
                        shape.push(field);
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
        lines.extend(specimen_bounds(root));
        return Ok(format!("{}\n", lines.join("\n")));
    };
    let members = match crate::mcp::workspace::scope(root)? {
        Scope::Package(namespace) => vec![Member::package(root, namespace)],
        Scope::Workspace(members) => members,
        Scope::Unresolvable(reason) => {
            lines.push(format!("no siblings: {reason}"));
            lines.extend(specimen_bounds(root));
            return Ok(format!("{}\n", lines.join("\n")));
        }
    };
    let sources = load_sources(root)?;
    for member in &members {
        let (evidence, faces, unparsable) = member_faces(member, RECORD_CANNOT_ANSWER)?;
        lines.push(evidence);
        // A file the derivation refused is missing from the sibling set, and an answer that counted
        // the set without saying so would read as complete.
        // 推导拒绝掉的文件不在同族集合里，而一个不说明这点就清点集合的答案会被读成完整的。
        if !unparsable.trim().is_empty() {
            lines.push(format!(
                "member {}: {} (such a file is not in the sibling set below)",
                member.name,
                unparsable.trim_end()
            ));
        }
        let siblings = siblings(&faces, &parent)
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
                        remaining
                            .push(format!("read src/{}", face.source.trim_start_matches("./")));
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
            // W4-6: the row names the field; the bytes that carry it are what a reader then opens a
            // file to find. The outlier's own line comes first and the specimen's (the majority's)
            // beside it, because "lacks X" is only actionable next to the declaration that has X.
            // W4-6：这一行点名了字段，而承载它的字节正是读者随后要打开文件去找的东西。离群者自己的行在前、
            // 标本（多数派）的在旁，因为"缺 X"只有在有 X 的那条声明旁边才是可行动的。
            let fields = gap_fields(&gaps);
            let mut shown: Vec<String> = Vec::new();
            if let Some(text) = source_text(&sources, &face.source) {
                let path = format!("src/{}", face.source.trim_start_matches("./"));
                shown.extend(lines_naming(text, &path, &fields, EXCERPT_LIMIT));
                // A field that is *absent* has no line to quote, so the sibling's own declaration
                // block stands in for it: "this is what it declares, next to what it does not" is
                // the pair a reader needs, and without it the row would show only the specimen's
                // half and leave the file to be opened anyway.
                // **缺**掉的字段没有行可引，于是用这个兄弟自己的声明块代替它："这是它声明的，旁边是它没声明的"
                // 才是读者需要的那一对；没有它，这一行只会显示标本那一半，文件还是得打开。
                if shown.is_empty() {
                    shown.extend(declaration_block(
                        text,
                        &path,
                        EXCERPT_LIMIT.saturating_sub(shown.len()),
                    ));
                }
            }
            for (file, text) in &current {
                shown.extend(lines_naming(
                    text,
                    file,
                    &fields,
                    EXCERPT_LIMIT.saturating_sub(shown.len()),
                ));
            }
            if let Some(command) = read_command(&shown) {
                remaining.push(command);
            }
            remaining.push(format!("--call check --face {anchor}"));
            // Audit `W5-4`: a comparison that only names the deviating sibling makes the reader do
            // the repair research. The two things a repair needs are already in this answer — what
            // the majority declares (the specimen's own shape) and which sibling deviates — so the
            // row carries the **request** that performs it, in the write path's own spelling, and
            // the reader pastes it rather than reconstructing it.
            // 审计 `W5-4`：只点名离群兄弟的比对，把修复的调研留给了读者。一次修复需要的两件事本来就在这份
            // 答案里——多数派声明了什么（标本自己的形状）与哪个兄弟偏离了——因此这一行带上**执行它的那个
            // 请求**，用写入路径自己的拼写，读者粘贴而不是重建。
            // A repair request is only handed back for a face the write path **may** rewrite: `edit`
            // refuses a hand-written file by design (only generated faces are NichLink's to rewrite),
            // so a `fix` line there would be a line that fails when pasted. The hand-written case gets
            // the same facts in the form that works — what to declare, in which file.
            // 只有写入路径**可以**重写的面才交回修复请求：`edit` 按设计拒绝手写的文件（只有生成的面才是
            // NichLink 可改的），因此那种情况下给一条 `fix` 就是给一条粘贴即失败的行。手写的情形拿到的是
            // 同样的事实、以行得通的形式：要声明什么、在哪个文件。
            let text = source_text(&sources, &face.source);
            match repair_request(&shape, face, &gaps, text) {
                Some(fix) => {
                    rows.push(format!(
                        "  fix         {}",
                        serde_json::to_string(&fix).unwrap_or_default()
                    ));
                    remaining.push(format!(
                        "--call apply --action edit --node {} --apply true",
                        face.path
                    ));
                }
                None => {
                    if let Some(manual) = manual_repair(&shape, &gaps, &face.source, &face.kind) {
                        rows.push(format!("  fix         {manual}"));
                    }
                }
            }
            rows.extend(shown);
        }
        lines.push(format!(
            "family {parent} · member {} · {} sibling(s)",
            member.name,
            siblings.len()
        ));
        // The majority is the specimen **because** the comparison calls it the baseline; saying so
        // with a count is what makes the `fix` rows below a repair rather than an opinion.
        // 多数派就是标本**因为**这次比对把它当基线；用一个计数把它说出来，才让下面的 `fix` 行成为修复
        // 而不是一种意见。
        lines.push(format!(
            "majority   {} of {} sibling(s) carry this shape; shape {}",
            siblings.len() - differing - unread,
            siblings.len(),
            shape_line(&shape)
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
    lines.extend(specimen_bounds(root));
    if let Some(census) = tree_census(root) {
        lines.insert(0, census);
    }
    lines.push(
        crate::mcp::tools::closure(differing == 0 && unread == 0, &remaining)
            .trim_end()
            .to_owned(),
    );
    Ok(format!("{}\n", lines.join("\n")))
}

/// What the specimen comparison does not read, and the next call it points at.
/// 标本比对该读不到什么，以及它指向的下一次调用。
fn specimen_bounds(root: &Path) -> Vec<String> {
    // Audit `W2-2`: this is a **constant** block, worth reading once. From the second comparison in
    // the same session it is one line pointing at the first — the same rule the write path's
    // `consequences` block follows (`session::first_time`, keyed by root so two trees never share it).
    // 审计 `W2-2`：这是一个**常量**块，值得读一次。同一会话里的第二次比对起，它就是一行指向第一次的回指
    // ——与写入路径的 `consequences` 块同一条规则（`session::first_time`，键带根，因此两棵树不共享）。
    if !crate::mcp::session::first_time(&format!("consistency-bounds:{}", root.display())) {
        return vec![
            "not covered: unchanged from this session's earlier comparison — the columns, the              derived-tree caveat and the `api` boundary are the ones printed then"
                .to_owned(),
        ];
    }
    vec![
        "quoted lines are an excerpt, not the file: at most 10 per deviating sibling, trimmed, and cut at 200 \
         characters with `…` — they are the lines the deviation is about, so `read {path, line}` is \
         still what a reader opens when the surrounding code is the question".to_owned(),
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

/// Why the published record cannot answer this tool's question.
/// 发布记录为什么答不了本工具的问题。
///
/// The comparison reads each sibling's declared fields and its own text; a record carries neither.
/// Saying that out loud is what lets the derived reading be the honest one rather than a silent
/// fallback.
/// 这次比较读的是每个兄弟的已声明字段与它自己的文本，而记录两样都不带。把这一点说出来，才让"此刻推导"
/// 是一个诚实的读法，而不是一次静默的回落。
const RECORD_CANNOT_ANSWER: &str = "this comparison reads each sibling's declared fields and its own text, which the published \
     record does not carry";

/// One member's faces, derived now, plus the line that says which tree this answer read.
/// 一个成员此刻推导出的面，以及说明这份答案读的是哪棵树的那一行。
///
/// The published record carries no `path` and no declared fields, so it can never answer this
/// question: the derived tree is not a fallback here but the only reading. That is why this goes
/// through `derived_tree` (the documented fallback every caller has to state) rather than `tree` —
/// the latter answers from the record and would make this tool refuse on exactly the trees that have
/// been built, which is the normal case.
/// 发布记录不携带 `path`、也不携带已声明字段，因此它永远答不了这个问题：推导树在这里不是回落而是唯一的
/// 读法。这也是它走 `derived_tree`（有文档的、每个调用方都必须说出来的那条回落）而不是 `tree` 的原因——
/// 后者从记录作答，会让本工具恰好在**已经构建过**的树上拒答，而那是常态。
fn member_faces(
    member: &Member,
    because: &str,
) -> Result<(String, Vec<crate::build_time::FaceView>, String), String> {
    let evidence = member.evidence_line(because).trim_end().to_owned();
    let (faces, unparsable) = member.derived_tree()?;
    Ok((evidence, faces, unparsable))
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
/// Which verb this comparison's rows use: the `api` signal talks about calls, the `shape` signal
/// about declarations. One core, two subjects — the alternative is a second copy of the majority
/// arithmetic, and two copies of a majority rule is exactly how the two signals would come to
/// disagree about what "outlier" means.
/// 这次比较的行用哪个动词：`api` 信号说的是**调用**，`shape` 信号说的是**声明**。一个内核、两种主语
/// ——另一种做法是把多数表决的算术再抄一份，而两份多数规则正是两个信号会对"离群"的定义产生分歧的来路。
#[derive(Clone, Copy)]
enum Wording {
    /// The `api` signal: names this face's file calls.
    /// `api` 信号：这个面的文件调用了哪些名字。
    Calls,
    /// The `shape` signal: fields this face declares.
    /// `shape` 信号：这个面声明了哪些字段。
    Declares,
}

impl Wording {
    fn missing(self, name: &str) -> String {
        match self {
            Wording::Calls => format!("does not call `{name}`, which the other siblings call"),
            Wording::Declares => {
                format!("does not declare `{name}`, which the other siblings declare")
            }
        }
    }

    fn extra(self, name: &str) -> String {
        match self {
            Wording::Calls => format!("calls `{name}`, which no sibling calls"),
            Wording::Declares => format!("declares `{name}`, which no sibling declares"),
        }
    }
}

fn deviations(sets: &[(String, BTreeSet<String>)], wording: Wording) -> Vec<(String, Vec<String>)> {
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
                notes.push(wording.missing(name));
            }
        }
        for name in names {
            let count = sets
                .iter()
                .filter(|(_, other)| other.contains(name))
                .count();
            if count == 1 {
                notes.push(wording.extra(name));
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
             shape: {\"parent\":\"<logical path>\",\"by\":\"api|kind|source|shape\",\"root\":\"<path>\"} \
             or {\"specimen\":\"<anchor>\",\"root\":\"<path>\"}"
                .to_owned()
        })?;
    // Which signals to run. **Absent `by` runs both `api` and `shape`**: a family outlier can be a
    // sibling calling something the rest do not, or a sibling **declaring a field the rest do not**,
    // and the second shape used to be invisible — a family whose only difference was one extra
    // declared field came back `outliers: 0 of 3`, so the one call this tool promises ("one call
    // names the outlier") did not name it. An explicit `by` still runs exactly one signal.
    // 跑哪些信号。**`by` 缺省时同时跑 `api` 与 `shape`**：同族的离群者可能是"调用了别人不调的"，也可能是
    // "**声明了别人不声明的字段**"，而後一种形状过去是看不见的——一个唯一差别就是多一行声明的同族回的是
    // `outliers: 0 of 3`，于是这个工具承诺的那一次调用（"一次调用点名离群者"）并没有点到它。显式给 `by`
    // 时仍然只跑一个信号。
    let mut remaining: Vec<String> = Vec::new();
    let mut any_outlier = false;
    let mut unread_rows = 0usize;
    let requested = arguments
        .get("by")
        .and_then(Value::as_str)
        .map(str::to_owned);
    if let Some(one) = requested
        .as_deref()
        .filter(|one| !matches!(*one, "api" | "kind" | "source" | "shape"))
    {
        return Err(format!(
            "consistency takes `by` as `api`, `kind`, `source` or `shape`, got `{one}` — \
                 accepted shape: {{\"parent\":\"<logical path>\",\
                 \"by\":\"api|kind|source|shape\",\"root\":\"<path>\"}}"
        ));
    }
    let signals: Vec<&str> = match requested.as_deref() {
        Some(one) => vec![one],
        None => vec!["api", "shape"],
    };
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
        let (evidence, faces, unparsable) = member_faces(member, RECORD_CANNOT_ANSWER)?;
        sections.push(evidence);
        // A file the derivation refused is not in the sibling set, so a comparison that stayed
        // silent about it would answer "these are all the siblings" about a tree that dropped one.
        // The drop is the tree's own line; it is carried here rather than re-derived.
        // 推导拒绝掉的文件不在同族集合里，因此对这件事保持沉默的比较，会对着一个**丢掉了一个文件**的树
        // 答出"兄弟就这些"。丢掉的实情是树自己那一行，这里只是搬运，不重新推导。
        if !unparsable.trim().is_empty() {
            sections.push(format!(
                "member {}: {} (such a file is not in the sibling set below)",
                member.name,
                unparsable.trim_end()
            ));
        }
        let set = siblings(&faces, parent);
        if set.is_empty() {
            // A member with no face under it contributes nothing, and the round measured this
            // line being read as an answer rather than as an early exit: a session on a tree
            // with no registered face spent steps on family tools whose every row said "0
            // members". Emit the line only when the whole family is empty, so one sentence
            // states the shape instead of one sentence per member.
            // 底下没有面的成员不贡献任何信息，而那一轮量到这一行被当成答案而不是早退：在没有注册面
            // 的树上，一个会话为每个"0 成员"的家族工具行花了步数。只在**整个家族都空**时印一句，
            // 让一句话说清形状，而不是每个成员一句。
            continue;
        }
        for signal in signals.iter().copied() {
            let mut rows: Vec<String> = vec![format!(
                "family {parent} · member {} · {} member(s) · by {signal}",
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
                let value = match signal {
                    "kind" => face.kind.clone(),
                    "source" => face.source.clone(),
                    // The `shape` signal: **which of the fields this comparison reads the face
                    // declares at all**. Presence, not value — a family's faces legitimately state
                    // different names, summaries and parts types, so comparing values would call
                    // every family an outlier; what is a drift is declaring a field the rest do not.
                    // Read through the kernel's own parser, the same reader `specimen` uses.
                    // `shape` 信号：**这个面到底声明了这次比较读的哪些字段**。只看存在，不看取值——同族
                    // 的面写出不同的名字、摘要与零件类型正是设计，比值会让每一个同族都成离群；而"声明了别人
                    // 不声明的字段"才是漂移。取值经内核自己的解析器读出，与 `specimen` 用的是同一个读取器。
                    "shape" => {
                        let names = match source_text(&sources, &face.source).map(one_face) {
                            Some(Ok(parsed)) => declared_shape(&parsed)
                                .into_iter()
                                .map(|field| field.key.to_owned())
                                .collect::<BTreeSet<String>>(),
                            Some(Err(reason)) => {
                                unread_rows += 1;
                                rows.push(format!("  {label:<24} unreadable: {reason}"));
                                continue;
                            }
                            None => {
                                unread_rows += 1;
                                rows.push(format!("  {label:<24} its file is not in the index"));
                                continue;
                            }
                        };
                        let shown = if names.is_empty() {
                            "none".to_owned()
                        } else {
                            names.iter().cloned().collect::<Vec<_>>().join(", ")
                        };
                        sets.push((label.clone(), names.clone()));
                        rows.push(format!("  {label:<24} {} field(s): {shown}", names.len()));
                        continue;
                    }
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
                                // `SourceFile::relative` is already spelled from the package root
                                // (`src/…`), which is why the directory is built with one `src/` and the
                                // comparison does not add a second: doing that made every filter miss and
                                // the whole signal answer `0 call(s)` on every tree — the defect this
                                // pin was written for.
                                // `SourceFile::relative` 本来就是从包根拼的（`src/…`），因此目录只加一次
                                // `src/`，比较时不再加第二次：加第二次会让每一个过滤都落空，于是整个信号在
                                // 每棵树上都答 `0 call(s)`——这正是这条钉子被写下来的那个缺陷。
                                source.relative.starts_with(&format!("{directory}/"))
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
            if signal == "api" || signal == "shape" {
                let wording = if signal == "api" {
                    Wording::Calls
                } else {
                    Wording::Declares
                };
                let found = deviations(&sets, wording);
                for (sibling, notes) in &found {
                    rows.push(format!("  outlier     {sibling}: {}", notes.join("; ")));
                    // W4-6: the names are in the row; the lines that carry them are what the reader
                    // would open files for. A name this sibling calls but no other does is shown in
                    // **its** file; a name the others call and this one does not is shown where the
                    // majority states it, because an absent line cannot be quoted.
                    // W4-6：名字已经在那一行里，而承载它们的源码行才是读者要打开文件去找的东西。本兄弟调了、
                    // 别人都没调的名字，在**它自己**的文件里显示；别人都调、它没调的名字，显示在多数派写它的
                    // 地方——缺掉的那一行没有东西可引。
                    let mut shown: Vec<String> = Vec::new();
                    for note in notes {
                        if shown.len() >= EXCERPT_LIMIT {
                            break;
                        }
                        let Some(name) = backticked(note) else {
                            continue;
                        };
                        let target = if note.starts_with("calls") || note.starts_with("declares") {
                            Some(sibling.clone())
                        } else {
                            set.iter().map(|face| label_of(face)).find(|other| {
                                other != sibling
                                    && sets.iter().any(|(label, names)| {
                                        label == other && names.contains(name)
                                    })
                            })
                        };
                        let Some(target) = target else {
                            continue;
                        };
                        let Some(face) = set.iter().find(|face| label_of(face) == target) else {
                            continue;
                        };
                        let Some(text) = source_text(&sources, &face.source) else {
                            continue;
                        };
                        let needles = [name.to_owned()];
                        shown.extend(lines_naming(
                            text,
                            &format!("src/{}", face.source.trim_start_matches("./")),
                            &needles,
                            EXCERPT_LIMIT.saturating_sub(shown.len()),
                        ));
                    }
                    if let Some(command) = read_command(&shown) {
                        remaining.push(command);
                    }
                    rows.extend(shown);
                }
                outliers = found.len();
                any_outlier |= outliers > 0;
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
                            "  outlier     {sibling}: `{signal}` is `{value}` while the majority says `{majority}`"
                        ));
                        outliers += 1;
                    }
                }
            }
            any_outlier |= outliers > 0;
            rows.push(format!("outliers: {outliers} of {}", sets.len()));
            sections.push(rows.join("\n"));
        }
    }
    sections.push(if crate::mcp::session::first_time(&format!(
        "consistency-bounds:{}",
        root.display()
    )) {
        "not covered by this comparison: it reads the **derived** tree's sibling set and each \
         sibling's own text, so a convention that lives in a shared helper, in generated code, or \
         in a parent rule is not visible here; and `api` compares the names called, not the units \
         or the arithmetic — an outlier is a place to look, not a defect; and a call written inside \
         a macro body is not read as a call (the kernel's rule), so an object whose whole body is one \
         macro invocation reads as calling nothing — **on such a tree `api` has nothing to compare, \
         but `by: shape` does: it compares which of the fields this comparison reads each \
         sibling declares at all, so it still names a sibling that declares an extra field; \
         `by: kind` / `by: source` compare the declared value, `specimen` against a \
         certified shape**"
            .to_owned()
    } else {
        "not covered: unchanged from this session's earlier comparison — the derived-tree caveat \
         and the meaning of an outlier are the ones printed then"
            .to_owned()
    });
    sections.push(
        "next   `read {path, line}` for the outlier's body, `explain {node}` for its declared fields, \
           and `by: shape` when the difference is a **declaration** rather than a call (an \
           extra declared field does not show up in `api`)"
            .to_owned(),
    );
    // W4-5: the census first, so the answer carries the tree's size instead of a second call, and
    // the closure last, judged conservatively — a family nobody deviates from and no unreadable file
    // is the one case that closes.
    // W4-5：普查在最前，答案自带树的大小而不必再花一次调用；闭合行在最后，判定从保守——没有任何兄弟
    // 偏离、也没有读不了的文件，才是唯一闭合的情形。
    if let Some(census) = tree_census(root) {
        sections.insert(0, census);
    }
    sections.push(
        crate::mcp::tools::closure(!any_outlier && unread_rows == 0, &remaining)
            .trim_end()
            .to_owned(),
    );
    Ok(format!("{}\n", sections.join("\n")))
}
