//! Everything an agent needs to start on one face, in one answer.
//! 代理着手处理一个面所需的全部，集中在一个答案里。
//!
//! The other tools each answer one question, which means an agent that wants to
//! touch a face has to ask several and join the answers itself. This is that join,
//! and it adds the one verdict no single tool can give: a requirement is written
//! `capability=>ProviderKind`, and whether anything in the package actually answers
//! it is a question about the whole tree. Everything here is composed from what the
//! other tools report — the build's scope and pruning, the tree's edges, the
//! declared fields — so a disagreement between this and them would be a bug in the
//! composition rather than a second derivation.
//! 别的工具各回答一个问题，因此想动一个面的代理得问好几次、再自己把答案拼起来。这里就是那次拼接，
//! 并补上任何一个工具都给不出的那个判断：需求写成 `capability=>ProviderKind`，而"包里有东西真的满足
//! 它吗"是关于整棵树的问题。这里的一切都从别的工具所报告的东西组合而来——构建的作用域与剪枝、树的边、
//! 声明的字段——因此它若与它们不一致，那是组合的缺陷，而不是第二份推导。

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use nichlink_build_method::{FaceView, face_views};
use serde_json::Value;

use crate::apply::load_registry;
use crate::evidence::{build_evidence, pruning_line, scope_line};
use crate::nodes::resolve_node;
use crate::protocol::DEFAULT_LIMIT;
use crate::registry::namespace;
use crate::trace::{RecordedTrace, read_verified};

/// How many frames of one face, and how many files outside any face, are shown.
/// 一个面最多展示几个帧、以及多少个不在任何面里的文件。
const FRAMES_PER_FACE: usize = 5;

/// One `capability=>provider` requirement, and who answers it.
/// 一条 `capability=>provider` 需求，以及谁来满足它。
struct RequirementVerdict {
    capability: String,
    provider: String,
    answered_by: Option<String>,
}

/// Report the converged starting point for one face, or for a recorded run.
/// 报告一个面、或一次已记录运行的收敛起点。
///
/// Two entries, because a bug report arrives in two shapes: an agent already knows
/// which face it is looking at (pass `node`), or it has a run that misbehaved and
/// only the trace says what that run touched (pass `trace: true`). The second turns
/// "58,792 lines of source" into "these files declared faces and ran", which is the
/// convergence step a symbol graph cannot take.
/// 两个入口，因为缺陷报告有两种形状：代理已经知道自己在看哪个面（传 `node`），或者它手上只有一次行为
/// 不对的运行，而"那次运行碰了什么"只有 trace 说得出来（传 `trace: true`）。后者把"58,792 行源码"
/// 变成"这些文件声明了面、而且真的跑了"，这是符号图迈不出的那一步收敛。
pub(crate) fn converge(root: &Path, arguments: &Value) -> Result<String, String> {
    let namespace = namespace(root)?;
    let faces = face_views(root, &namespace)?;
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize);
    if arguments.get("trace").and_then(Value::as_bool) == Some(true) {
        return converge_from_trace(root, &faces, limit);
    }
    let target = arguments
        .get("node")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "nichlink.converge requires node (one face) or trace: true (the recorded run)"
                .to_owned()
        })?;
    let id = resolve_node(root, &namespace, target)?;
    let face = faces
        .iter()
        .find(|face| face.id == id)
        .ok_or_else(|| format!("no face in the derived tree has identity {id}"))?;
    let (current, scope, pruning) = build_evidence(root);
    // Loading the package's own faces *validates* them — the registry rejects a
    // tree whose requirement has no provider — and that refusal is the most
    // valuable answer this tool can give, so it is a verdict rather than an error.
    // A reader who only gets "the package's own faces were rejected" has to go
    // somewhere else for the node and the line; the diagnostic already carries
    // both.
    // 加载本包自己的面会**校验**它们——注册机拒绝一棵需求没有提供者的树——而那次拒绝正是这个工具能给出的
    // 最有价值的答案，因此它是一条判断而不是一个错误。只拿到"本包自己的面被拒绝"的读取方还得去别处找
    // 节点与行号；而那份诊断里两样都已经有了。
    let (offerings, authored, unreadable, rejected) = match load_registry(root, &namespace) {
        Ok(registry) => {
            // Same rule as the write path: the read-back resolves against the
            // package root the context carries, never against the process's own
            // directory.
            // 与写入路径同一条规矩：读回以上下文携带的包根为基准，绝不以进程自己的目录为基准。
            let context =
                nichlink_run_method::AuthoringContext::new(root.to_path_buf(), namespace.clone());
            let read_back =
                |id| context.scope(|| nichlink_run_method::authored_face(&registry, id));
            // The package-wide answer set: which kind offers which capability.
            // 包级的答案表：哪个 kind 提供哪个能力。
            let mut offerings: BTreeMap<String, Vec<(String, BTreeSet<String>)>> = BTreeMap::new();
            let mut unreadable = 0usize;
            for candidate in &faces {
                match read_back(candidate.id) {
                    Ok(authored) => {
                        offerings
                            .entry(authored.kind.clone())
                            .or_default()
                            .push((candidate.path.clone(), tokens(&authored.provides)));
                    }
                    Err(_) => unreadable += 1,
                }
            }
            (offerings, read_back(id), unreadable, None)
        }
        Err(rejection) => (BTreeMap::new(), Err(rejection.clone()), 0, Some(rejection)),
    };
    let requirements = authored
        .as_ref()
        .map(|authored| parse_requirements(&authored.requires, &offerings))
        .unwrap_or_default();

    let mut output = format!(
        "namespace {namespace}\nnode {}\n  path {}\n  kind {}\n",
        face.id, face.path, face.kind
    );
    output.push_str(&format!(
        "build {}\n",
        if current {
            "current"
        } else {
            "stale (run `nichlink check` before trusting the scope below)"
        }
    ));
    output.push_str(&scope_line(scope.as_ref(), face));
    output.push_str(&pruning_line(pruning.as_deref(), face));
    let children = faces
        .iter()
        .filter(|candidate| candidate.parent == id)
        .collect::<Vec<_>>();
    output.push_str(&format!("children {}\n", children.len()));
    match (&rejected, &authored) {
        (Some(rejection), _) => {
            output.push_str("kernel verdict: this package's own faces are rejected\n");
            for line in rejection.lines() {
                output.push_str(&format!("  {}\n", line.trim_end()));
            }
        }
        (None, Ok(authored)) => {
            output.push_str(&format!("requires {}\n", blank(&authored.requires)));
            for verdict in &requirements {
                match &verdict.answered_by {
                    Some(who) => output.push_str(&format!(
                        "  {} => {}  answered by {who}\n",
                        verdict.capability, verdict.provider
                    )),
                    None => output.push_str(&format!(
                        "  {} => {}  UNANSWERED (no face of kind `{}` offers `{}`)\n",
                        verdict.capability, verdict.provider, verdict.provider, verdict.capability
                    )),
                }
            }
        }
        (None, Err(error)) => output.push_str(&format!("requires unreadable ({error})\n")),
    }
    let mut plan: Vec<(String, &'static str)> = vec![(face.source.clone(), "this face")];
    if let Some(parent) = faces.iter().find(|candidate| candidate.id == face.parent) {
        plan.push((parent.source.clone(), "parent"));
    }
    for child in &children {
        plan.push((child.source.clone(), "child"));
    }
    plan.dedup_by(|left, right| left.0 == right.0);
    output.push_str(&format!("read plan ({} files)\n", plan.len()));
    for (source, why) in plan.iter().take(limit) {
        output.push_str(&format!("  {source:<44} ({why})\n"));
    }
    if plan.len() > limit {
        output.push_str(&format!("  … +{} more\n", plan.len() - limit));
    }
    if unreadable > 0 {
        output.push_str(&format!(
            "unreadable faces {unreadable} (hand-written modules declare no readable fields)\n"
        ));
    }
    output.push_str(
        "detail: nichlink.explain (build evidence) · nichlink.usages (fields and capability refs) · \
         nichlink.trace (what ran) · nichlink.diff (what changed since the build)\n",
    );
    Ok(output)
}

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
fn converge_from_trace(root: &Path, faces: &[FaceView], limit: usize) -> Result<String, String> {
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

/// Parse `capability=>provider` entries and look up who offers each capability
/// under the expected kind.
/// 解析 `capability=>provider` 条目，并在期望的 kind 之下查找谁提供该能力。
fn parse_requirements(
    requires: &str,
    offerings: &BTreeMap<String, Vec<(String, BTreeSet<String>)>>,
) -> Vec<RequirementVerdict> {
    let mut verdicts = Vec::new();
    for entry in requires.split([',', '\n', ' ', '\t']) {
        let entry = entry.trim().trim_matches(['"', '[', ']', '(', ')']);
        if entry.is_empty() {
            continue;
        }
        let Some((capability, provider)) = entry.split_once("=>") else {
            // A bare capability name is what the kernel refuses at authoring time,
            // so finding one here is evidence of a hand-written tree.
            // 裸能力名正是内核在创作期拒绝的东西，因此在这里遇到它，说明这棵树是手写的。
            verdicts.push(RequirementVerdict {
                capability: entry.to_owned(),
                provider: "?".to_owned(),
                answered_by: None,
            });
            continue;
        };
        let capability = capability.trim().to_owned();
        let provider = provider.trim().to_owned();
        let answered_by = offerings.get(&provider).and_then(|candidates| {
            candidates
                .iter()
                .find(|(_, provided)| provided.contains(&capability))
                .map(|(path, _)| path.clone())
        });
        verdicts.push(RequirementVerdict {
            capability,
            provider,
            answered_by,
        });
    }
    verdicts
}

/// The identifier-ish tokens of a manifest text field.
/// 一个清单文本字段里的标识符式记号。
fn tokens(text: &str) -> BTreeSet<String> {
    text.split(|character: char| {
        !(character.is_alphanumeric() || character == '_' || character == '.' || character == ':')
    })
    .filter(|token| token.len() > 1 && token.chars().any(|c| c.is_alphanumeric()))
    .map(str::to_owned)
    .collect()
}

/// An empty field reads as `-` rather than as nothing at all.
/// 空字段读作 `-`，而不是什么都没有。
fn blank(value: &str) -> &str {
    if value.trim().is_empty() { "-" } else { value }
}

#[cfg(test)]
#[path = "converge_tests.rs"]
mod converge_tests;
