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

use crate::build_method::FaceView;

use crate::mcp::trace::{RecordedTrace, local_line, read_verified};
use crate::mcp::truncation::{withheld, withheld_uncounted};

/// How many frames of one face, and how many files outside any face, are shown.
/// 一个面最多展示几个帧、以及多少个不在任何面里的文件。
const FRAMES_PER_FACE: usize = 5;

/// How many recorded values or edges one face shows before it says how many it left.
/// 一个面在说出落下多少之前最多展示几个记录值或边。
const VALUES_PER_FACE: usize = 8;

/// The default number of lines a trace-driven report carries before it stops adding
/// detail.
/// trace 驱动的报告在停止追加细节之前最多携带的默认行数。
///
/// It is the default rather than the only value because a cap nothing can move is a cap
/// nothing can test: the branch that says "detail stopped here" only runs on a report
/// past this many lines, which a fixture cannot reach, so the entry point takes the cap
/// as a parameter and this constant is what the tool passes
/// ([`converge_from_trace_with`] is the parameterized body; `converge_tests` drives it
/// with a small cap).
/// 它是默认值而不是唯一取值，因为一个无法被移动的上限就是一个无法被测试的上限："细节到此为止"那一支
/// 只在本报告越过这么多行时才跑，而夹具到不了，因此入口把这个上限作为参数接收，这个常数则是工具传入
/// 的值（[`converge_from_trace_with`] 是带参数的主体；`converge_tests` 用小值驱动它）。
const MAX_CONVERGE_LINES: usize = 200;

/// How a reader reaches the per-face detail this report's **own** caps withhold.
/// 读取方怎么拿到本报告**自身**那几道上限扣下的逐面细节。
///
/// These caps are constants, not `limit`: no argument raises them, so the sentence has
/// to name the absence and point at the tool that prints the run itself. Advice a caller
/// cannot act on is what audit `LGC-LG-43` recorded against `raise \`limit\``.
/// 这几道上限是常数而不是 `limit`：没有参数能提高它们，因此那句话必须点名这个"没有"，并指向那个
/// 打印整次运行的工具。调用方无法执行的建议，正是审计 `LGC-LG-43` 记在 `raise \`limit\`` 上的问题。
const MORE_DETAIL: &str = "no argument raises this per-face cap; `nichlink.trace values: true` prints the run's values and edges";

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
///
/// The per-face detail cap is [`MAX_CONVERGE_LINES`]; [`converge_from_trace_with`] is the
/// same report with the cap passed in, which is what lets a test reach the branch that
/// says detail stopped.
/// 逐面细节的上限是 [`MAX_CONVERGE_LINES`]；[`converge_from_trace_with`] 是同一份报告、上限由调用方
/// 传入，正是它让测试能够到达"细节到此为止"的那一支。
pub(crate) fn converge_from_trace(
    root: &Path,
    faces: &[FaceView],
    limit: usize,
) -> Result<String, String> {
    converge_from_trace_with(root, faces, limit, MAX_CONVERGE_LINES)
}

/// The trace-driven report with its per-face detail cap given by the caller.
/// 逐面细节上限由调用方给出的 trace 驱动报告。
///
/// Everything but the cap is [`converge_from_trace`]'s behaviour; the sentence that
/// reports a stopped detail block names whichever cap was in force, so the two callers
/// cannot print a limit that is not the one they applied.
/// 除了这个上限之外，一切行为都属于 [`converge_from_trace`]；报告"细节停止"的那句话点名当时生效的上限，
/// 因此两个调用方都不可能打印出一个不是它们所施加的上限。
pub(crate) fn converge_from_trace_with(
    root: &Path,
    faces: &[FaceView],
    limit: usize,
    max_lines: usize,
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
    // Whether the per-face line cap below silently swallowed any detail: it is a cap
    // on the reply's size, so the values and edges it kept out were never counted, and
    // leaving that unsaid is how a shorter answer reads as a whole one.
    // 下面那个逐面行数上限是否默默吞掉了细节：它是回复规模的上限，因此它挡在门外的值与边从未被
    // 计数，而对此不作声正是一个更短的答案被读成完整答案的方式。
    let mut detail_stopped = false;
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
            output.push_str(&format!(
                "  {}\n",
                withheld(
                    ran.len() - limit,
                    ran.len(),
                    limit,
                    "files",
                    "raise `limit`"
                )
            ));
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
            output.push_str(&format!(
                "    {}\n",
                withheld(
                    omitted,
                    entry.frames,
                    FRAMES_PER_FACE,
                    "frame(s)",
                    MORE_DETAIL
                )
            ));
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
        if !locals.is_empty() && lines < max_lines {
            output.push_str(&format!("    values ({})\n", locals.len()));
            lines += 1;
            for local in locals.iter().take(VALUES_PER_FACE) {
                output.push_str(&format!("    {}\n", local_line(local).trim_end()));
                lines += 1;
            }
            if locals.len() > VALUES_PER_FACE {
                output.push_str(&format!(
                    "      {}\n",
                    withheld(
                        locals.len() - VALUES_PER_FACE,
                        locals.len(),
                        VALUES_PER_FACE,
                        "value(s)",
                        MORE_DETAIL
                    )
                ));
                lines += 1;
            }
        } else if !locals.is_empty() {
            detail_stopped = true;
        }
        let ids: BTreeSet<u64> = locals.iter().map(|local| local.id).collect();
        let edges: Vec<_> = artifact
            .edges
            .iter()
            .filter(|edge| ids.contains(&edge.from) || ids.contains(&edge.to))
            .collect();
        if !edges.is_empty() && lines < max_lines {
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
                    "      {}\n",
                    withheld(
                        edges.len() - VALUES_PER_FACE,
                        edges.len(),
                        VALUES_PER_FACE,
                        "edge(s)",
                        MORE_DETAIL
                    )
                ));
                lines += 1;
            }
        } else if !edges.is_empty() {
            detail_stopped = true;
        }
    }
    output.push_str(&format!(
        "frames in a face {matched_frames} / outside any declared face {outside_frames}\n"
    ));
    for (file, count) in outside.iter().take(limit) {
        output.push_str(&format!("  {file} ({count})\n"));
    }
    if outside.len() > limit {
        output.push_str(&format!(
            "  {}\n",
            withheld(
                outside.len() - limit,
                outside.len(),
                limit,
                "files",
                "raise `limit`"
            )
        ));
    }
    if no_callsite > 0 {
        output.push_str(&format!("frames with no recorded callsite {no_callsite}\n"));
    }
    output.push_str(&format!("read plan ({} files)\n", ran.len()));
    for (index, source) in ran.keys().enumerate() {
        if index >= limit {
            output.push_str(&format!(
                "  {}\n",
                withheld(
                    ran.len() - limit,
                    ran.len(),
                    limit,
                    "files",
                    "raise `limit`"
                )
            ));
            break;
        }
        output.push_str(&format!("  {source:<44} (this face)\n"));
    }
    if detail_stopped {
        // The per-face cap stopped adding detail, and the values it never printed were
        // never counted — the one fact this site cannot give, said out loud rather than
        // left as a shorter-looking answer.
        // 逐面上限停止了追加细节，而它从未打印的那些值也从未被计数——这是本站点给不出的那一个事实，
        // 说出来，而不是留下一个看起来更短的答案。
        output.push_str(&format!(
            "{}\n",
            withheld_uncounted(
                max_lines,
                "lines of per-face detail",
                "lower `limit` so fewer files take the detail, or read the whole run with \
                 `nichlink.trace values: true`"
            )
        ));
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
    let normalized = nichlink_kernel::declaration::portable_path(recorded);
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
