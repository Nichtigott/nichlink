//! The trace-driven entry point: converge from the run that happened.
//! trace 驱动的入口：从真正发生过的那次运行收敛。
//!
//! Split out of `converge.rs` when that file crossed the repository's line
//! ceiling, the same way `callgraph.rs` was split out of `tools.rs`. It answers the
//! other shape a bug report arrives in: an agent that does not yet know which face it
//! is looking at, only that a run misbehaved. Frames are matched to faces **by source
//! file** — a face is a declaration, a frame is an active function — and the values
//! those frames captured are attached to the face whose file they were captured in.
//! 当 `converge.rs` 越过本仓库的行数上限时拆出来，与 `callgraph.rs` 从 `tools.rs` 拆出的方式
//! 相同。它回答缺陷报告的另一种形状：代理还不知道自己要看哪个面，只知道某次运行行为不对。帧是**按源
//! 文件**匹配到面的——面是声明、帧是正在活动的函数——而那些帧捕获的值会被挂到"捕获它们时所在文件"的
//! 那个面上。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use nichlink_build_method::FaceView;

use crate::trace::{RecordedTrace, local_line, read_verified};

/// How many frames of one face, and how many files outside any face, are shown.
/// 一个面最多展示几个帧、以及多少个不在任何面里的文件。
const FRAMES_PER_FACE: usize = 5;

/// How many recorded values or edges one face shows before it says how many it left.
/// 一个面在说出落下多少之前最多展示几个记录值或边。
const VALUES_PER_FACE: usize = 8;

/// The most lines a trace-driven report carries before it stops adding detail.
/// trace 驱动的报告在停止追加细节之前最多携带的行数。
const MAX_CONVERGE_LINES: usize = 200;

/// Converge from the run that happened rather than from a face name.
/// 从真正发生过的那次运行收敛，而不是从一个面名收敛。
///
/// Frames are matched to faces **by their source file**, and the reply says so: a
/// face is a declaration, a frame is an active function, and the only thing tying
/// them is where the code lives. That is the honest boundary of this step, and it is
/// still a large collapse — a 350-file tree becomes the handful of files that both
/// declare a face and ran.
/// 帧是**按源文件**匹配到面的，回复里也这么写：面是声明、帧是正在活动的函数，把两者连起来的只有代码
/// 所在的位置。这是这一步诚实的边界，而它仍然是一次大幅收敛——350 个文件的树会变成"既声明了面、又真的
/// 跑了"的那几个文件。
pub(crate) fn converge_from_trace(
    root: &Path,
    faces: &[FaceView],
    limit: usize,
) -> Result<String, String> {
    let (path, artifact, header) = match read_verified(root)? {
        RecordedTrace::Absent(answer) | RecordedTrace::Refused(answer) => return Ok(answer),
        RecordedTrace::Verified {
            path,
            artifact,
            header,
            ..
        } => (path, artifact, header),
    };
    // Keyed by the face's source file: that is the key the match is made on, and
    // grouping by it keeps two faces declared in one file from being counted twice.
    // 以面的源文件为键：匹配就是按它做的，而按它分组可以避免"一个文件里声明两个面"被数两次。
    let mut ran: BTreeMap<String, RanFile> = BTreeMap::new();
    let mut outside: BTreeMap<String, usize> = BTreeMap::new();
    let mut no_callsite = 0usize;
    let mut matched_frames = 0usize;
    for frame in &artifact.frames {
        let Some(source) = frame.source.as_ref() else {
            // The outermost frame of a `with` still carries its caller's location,
            // so a frame without one is unusual rather than normal — counted, not
            // invented.
            // `with` 的最外层帧同样带着调用方位置，因此缺位置的帧是异常而非常态——只计数，不编造。
            no_callsite += 1;
            continue;
        };
        match faces
            .iter()
            .find(|face| matches_file(source.file, &face.source))
        {
            Some(face) => {
                matched_frames += 1;
                let entry = ran.entry(face.source.clone()).or_default();
                entry.faces.insert(face.path.clone());
                entry.kind = face.kind.clone();
                entry.frames += 1;
                entry.frame_ids.insert(frame.frame_id);
                if entry.samples.len() < FRAMES_PER_FACE {
                    entry.samples.push(format!(
                        "{}  {}:{}",
                        frame.function, source.file, source.line
                    ));
                }
            }
            None => *outside.entry(source.file.to_owned()).or_insert(0) += 1,
        }
    }
    let outside_frames: usize = outside.values().sum();

    let mut lines = header.lines().count();
    let mut output = header;
    output.push_str(&format!(
        "faces that ran ({} of {} declared, matched by source file)\n",
        ran.len(),
        faces.len()
    ));
    if ran.is_empty() {
        output.push_str("  (none: no frame's file declares a face in this tree)\n");
    }
    for (index, (source, entry)) in ran.iter().enumerate() {
        if index >= limit {
            output.push_str(&format!("  … +{} more files\n", ran.len() - limit));
            break;
        }
        output.push_str(&format!(
            "  {:<40} kind={:<16} {} frame(s)  {}\n",
            entry.faces.iter().cloned().collect::<Vec<_>>().join(" "),
            entry.kind,
            entry.frames,
            source
        ));
        for frame in &entry.samples {
            output.push_str(&format!("    {frame}\n"));
        }
        let omitted = entry.frames.saturating_sub(entry.samples.len());
        if omitted > 0 {
            output.push_str(&format!("    … +{omitted} more frame(s)\n"));
        }
        // What those frames *saw*, not only that they ran: the recorded locals of
        // this file's frames, and the observed edges between them. This is the half
        // an agent needs to form a hypothesis about a cross-file bug, and it is
        // already in the artifact `nichlink.trace` reads.
        // 那些帧**看见了**什么，而不只是它们跑过：本文件这些帧记录下的局部值，以及它们之间被观察到的
        // 边。这正是代理对跨文件缺陷形成假设所需要的那一半，而它本来就在 `nichlink.trace` 所读的
        // artifact 里。
        let locals: Vec<_> = artifact
            .locals
            .iter()
            .filter(|local| {
                local
                    .frame_id
                    .is_some_and(|id| entry.frame_ids.contains(&id))
            })
            .collect();
        if !locals.is_empty() && lines < MAX_CONVERGE_LINES {
            output.push_str(&format!("    values ({})\n", locals.len()));
            lines += 1;
            for local in locals.iter().take(VALUES_PER_FACE) {
                output.push_str(&format!("    {}\n", local_line(local).trim_end()));
                lines += 1;
            }
            if locals.len() > VALUES_PER_FACE {
                output.push_str(&format!(
                    "      … +{} more value(s)\n",
                    locals.len() - VALUES_PER_FACE
                ));
                lines += 1;
            }
        }
        let ids: BTreeSet<u64> = locals.iter().map(|local| local.id).collect();
        let edges: Vec<_> = artifact
            .edges
            .iter()
            .filter(|edge| ids.contains(&edge.from) || ids.contains(&edge.to))
            .collect();
        if !edges.is_empty() && lines < MAX_CONVERGE_LINES {
            output.push_str(&format!("    edges ({})\n", edges.len()));
            lines += 1;
            for edge in edges.iter().take(VALUES_PER_FACE) {
                let name = |id: u64| {
                    artifact
                        .locals
                        .iter()
                        .find(|local| local.id == id)
                        .map(|local| local.name.as_str())
                        .unwrap_or("<not recorded>")
                };
                output.push_str(&format!(
                    "      {} -> {}  ({})\n",
                    name(edge.from),
                    name(edge.to),
                    edge.label
                ));
                lines += 1;
            }
            if edges.len() > VALUES_PER_FACE {
                output.push_str(&format!(
                    "      … +{} more edge(s)\n",
                    edges.len() - VALUES_PER_FACE
                ));
                lines += 1;
            }
        }
    }
    output.push_str(&format!(
        "frames in a face {matched_frames} / outside any declared face {outside_frames}\n"
    ));
    for (file, count) in outside.iter().take(limit) {
        output.push_str(&format!("  {file} ({count})\n"));
    }
    if outside.len() > limit {
        output.push_str(&format!("  … +{} more files\n", outside.len() - limit));
    }
    if no_callsite > 0 {
        output.push_str(&format!("frames with no recorded callsite {no_callsite}\n"));
    }
    output.push_str(&format!("read plan ({} files)\n", ran.len()));
    for (index, source) in ran.keys().enumerate() {
        if index >= limit {
            break;
        }
        output.push_str(&format!("  {source:<44} (this face)\n"));
    }
    output.push_str(&format!(
        "detail: nichlink.converge node=<path> (one face's constraints) · nichlink.trace ({}) · \
         nichlink.usages (fields) · nichlink.diff (what changed)\n",
        path.display()
    ));
    Ok(output)
}

/// One source file that ran, and the faces it declares.
/// 一个跑过的源文件，以及它声明的面。
#[derive(Default)]
struct RanFile {
    faces: BTreeSet<String>,
    kind: String,
    frames: usize,
    /// The frame ids that landed in this file, so the values and edges they
    /// captured can be attached to it rather than to the tree as a whole.
    /// 落在这个文件里的帧 id，使它捕获的值与边可以被挂到它上面，而不是挂到整棵树。
    frame_ids: BTreeSet<u64>,
    samples: Vec<String>,
}

/// Whether a compiler-recorded file path names the file a face's identity path names.
/// 编译器记录的路径是否就是某个面的身份路径所指的那个文件。
///
/// The declaration macros drop one leading `src/` — and nothing else — so the
/// compiler's path (relative to whatever root cargo invoked rustc from) is rewritten
/// through its **last complete `src/` segment** before comparison. A path with no
/// `src/` segment is compared as it stands, which is the `[lib] path` case where an
/// identity keeps its leading directory.
/// 声明宏只去掉一个前导 `src/`、别的都不去，因此编译器的路径（相对 cargo 调用 rustc 的那个根）在比较
/// 前会先被重写到它**最后一个完整的 `src/` 段**之后。没有 `src/` 段的路径原样比较，那正是身份保留其
/// 前导目录的 `[lib] path` 情形。
fn matches_file(recorded: &str, identity: &str) -> bool {
    let normalized = recorded.replace('\\', "/");
    let normalized = normalized.strip_prefix("./").unwrap_or(&normalized);
    if normalized == identity {
        return true;
    }
    let segments: Vec<&str> = normalized.split('/').collect();
    match segments.iter().rposition(|segment| *segment == "src") {
        Some(position) => segments[position + 1..].join("/") == identity,
        None => false,
    }
}
