//! The release workflow's one hard precondition, as a gate.
//! 把发布工作流唯一硬性前提变成门禁。
//!
//! `cargo publish` cannot be taken back, and the only thing that ties an upload to a
//! version is the tag. The workflow says so itself ("a release is a tag and nothing
//! else starts it"), and `CHANGELOG.md` promises that a manual run rehearses every
//! check without publishing. Both sentences were false once: a `publish` input made the
//! upload step reachable from a branch, with the tag check skipped. A promise in a
//! comment decays exactly like a rule in prose, so this module reads the shipped YAML
//! and refuses the shapes that break it.
//! `cargo publish` 收不回来，而把一次上传绑定到某个版本的东西只有 tag。工作流自己这么说，而
//! `CHANGELOG.md` 承诺手动运行会演练全部检查而不发布。两句话都曾为假：一个 `publish` 输入让
//! 上传步骤可以从分支到达、且跳过 tag 检查。注释里的承诺会像散文里的规则一样腐化，因此本模块读
//! 实际出厂的 YAML，拒绝破坏它的那些形状。
//!
//! Two rounds of adversarial review taught this gate to stop trusting two things: the
//! *name* of a step (an upload under another name, or with no name, was invisible) and
//! *presence of a substring*. Substrings judge neither question correctly:
//! `!(startsWith(github.ref, 'refs/tags/'))` *contains* the positive tag test, and
//! `inputs['publish']` contains no `inputs.` at all. Both questions are now answered by
//! *shape* — an upload step's `if:` must be a whitelisted conjunction of positive tests,
//! one of them the tag test, and an input is whatever names one, in either spelling, on a
//! line that is not a comment. Every workflow in `.github/workflows/` is read too, because
//! a second file is a second way to publish.
//! 两轮对抗性审查让本门禁不再相信两样东西：步骤的**名字**（换名字或不写名字的上传步骤过去
//! 不可见）与**子串是否出现**。子串两个问题都判不对：`!(startsWith(github.ref, 'refs/tags/'))`
//! **含有**那个肯定式 tag 判断，而 `inputs['publish']` 里没有 `inputs.`。现在两个问题都由
//! **形状**回答——上传步骤的 `if:` 必须是白名单允许的"肯定式测试的合取"，而输入是任何命名了它
//! 的东西（两种拼法都算，只要那行不是注释）。`.github/workflows/` 下的每个工作流也会被读取，
//! 因为第二个文件就是第二条发布路径。

use std::fs;
use std::path::Path;

#[path = "release_guard.rs"]
mod release_guard;

/// The commands that upload, whichever step they sit in.
/// 会执行上传的命令，无论它位于哪个步骤。
const UPLOAD_COMMANDS: &[&str] = &["--publish", "cargo publish"];

/// The positive tag test a step that uploads has to carry, and the anchor of the
/// whitelist [`release_guard::positive_tests`] judges a condition against.
/// 上传步骤必须带着的肯定式 tag 判断，也是 [`positive_tests`] 判定条件时所用的白名单锚点。
const TAG_GUARD: &str = "startsWith(github.ref, 'refs/tags/')";

/// The one publisher this gate already understands by name.
/// 这个门禁唯一按名字认识的发布器。
///
/// Its upload mode is the `--publish` flag the command test reads, so the file itself is
/// not followed: following it would read its `--publish` branch and call every rehearsal
/// step (`run: tools/nichlink-publish`, `--check-table`, `--verify-consumers`) an upload.
/// A local file the gate does *not* know by name is a different matter — nothing about its
/// name says which mode it runs in, so it is read.
/// 它的上传模式正是命令判定读取的 `--publish` 标志，因此不跟进这个文件本身：跟进它会读到它的
/// `--publish` 分支，从而把每个演练步骤（`run: tools/nichlink-publish`、`--check-table`、
/// `--verify-consumers`）都当成上传。而门禁**不**按名字认识的本地文件是另一回事——它的名字
/// 说明不了它以哪种模式运行，因此会被读。
const KNOWN_PUBLISHER: &str = "tools/nichlink-publish";

/// Report every way the workflow lets an upload happen without a tag ref.
/// 报告该工作流容许在没有 tag ref 的情况下上传的每一种方式。
///
/// Empty means the properties hold: no input can make a manual run differ from a
/// rehearsal, every step that uploads carries the positive tag requirement itself, and
/// a workflow that can upload still offers the manual rehearsal path.
/// 返回空表示这些性质都成立：没有任何输入能让手动运行与演练不同、每个上传步骤自己带着肯定式 tag
/// 要求、而能够上传的工作流仍保留手动演练路径。
pub fn findings(text: &str) -> Vec<String> {
    findings_in(None, text)
}

/// The same findings, with a checkout to follow a delegated script or action into.
/// 同一组发现，另外给一个检出根，用来跟进被委托的本地脚本与 action。
fn findings_in(root: Option<&Path>, text: &str) -> Vec<String> {
    let mut findings = Vec::new();
    if takes_input(text) {
        findings.push(
            "the workflow reads an input, so a manual run can differ from a rehearsal; \
             uploads have to be reachable only from a tag ref"
                .to_owned(),
        );
    }
    if uploads(root, text) && !text.contains("workflow_dispatch:") {
        // The rehearsal path is what lets the first release be checked before its tag
        // exists, so a workflow that can upload has to keep offering it.
        // 演练路径让首个发布在其 tag 存在之前就能被检查，因此能够上传的工作流必须保留它。
        findings.push(
            "the workflow can upload but has no workflow_dispatch, so a release cannot \
             be rehearsed before its tag exists"
                .to_owned(),
        );
    }
    for (index, (body, condition)) in steps(text).iter().enumerate() {
        if !step_uploads(root, body) {
            continue;
        }
        let Some(condition) = condition else {
            findings.push(format!(
                "step {} uploads with no `if:` at all, so a branch run would publish",
                index + 1
            ));
            continue;
        };
        match guard_shape(condition) {
            GuardShape::Tag => {}
            GuardShape::NoTagTest => findings.push(format!(
                "step {} uploads without a positive tag guard; its own `if:` must name \
                 `{TAG_GUARD}` rather than trusting the trigger list: `{condition}`",
                index + 1
            )),
            GuardShape::Other => findings.push(format!(
                "step {} uploads under a condition that can be true without a tag ref; only a \
                 positive conjunction of tests is whitelisted: `{condition}`",
                index + 1
            )),
        }
    }
    findings.extend(crate::release_action_pin::findings(text)); // the token-holder pin rule
    findings
}
/// Every workflow in the checkout, with the same findings asked of each.
/// 检出里的每个工作流，各自被问同一组问题。
///
/// A second workflow file is a second way to publish: the gate used to read
/// `release.yml` alone, and a branch-triggered step running `--publish --yes` in a new
/// file produced no finding at all.
/// 第二个工作流文件就是第二条发布路径：门禁过去只读 `release.yml`，而新文件里一个分支触发、
/// 运行 `--publish --yes` 的步骤完全不会产生任何发现。
pub fn workflow_findings(root: &Path) -> Vec<String> {
    let directory = root.join(".github").join("workflows");
    let Ok(entries) = fs::read_dir(&directory) else {
        return vec![format!("cannot read {}", directory.display())];
    };
    let mut paths = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "yml" || extension == "yaml")
        })
        .collect::<Vec<_>>();
    paths.sort();
    let mut found = Vec::new();
    for path in paths {
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        for finding in findings_in(Some(root), &text) {
            found.push(format!("{name}: {finding}"));
        }
    }
    found
}

/// Whether the workflow can upload anything.
/// 该工作流是否可能上传任何东西。
fn uploads(root: Option<&Path>, text: &str) -> bool {
    steps(text).iter().any(|(body, _)| step_uploads(root, body))
}

/// Whether one step's lines run an upload, directly or through a local file.
/// 某个步骤的各行是否执行一次上传——直接执行，或经由一个本地文件。
///
/// Comments are skipped: `ci.yml` mentions the release workflow's `--publish` in prose,
/// and a whole-file substring search read that as an upload step.
/// 注释被跳过：`ci.yml` 在散文里提到发布工作流的 `--publish`，而对整个文件做子串搜索会把它读成
/// 一个上传步骤。
///
/// The step's own lines are matched with whitespace runs collapsed and line continuations
/// joined, because `cargo  publish` (two spaces) and a `\`-continued `cargo \` + `publish`
/// are the same command as the spelling the gate knew.
/// 步骤自己的行在匹配前会折叠空白串、拼接续行，因为 `cargo  publish`（两个空格）与
/// `cargo \` + `publish` 这种续行，与门禁认识的那种拼写是同一条命令。
fn step_uploads(root: Option<&Path>, body: &str) -> bool {
    let joined = join_continuations(body);
    let own_lines = joined
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.starts_with('#'));
    if own_lines.clone().any(names_upload) {
        return true;
    }
    // A step may hand the upload to a local script or a local action instead of spelling
    // the command itself. The gate used to read only the step's own lines, so
    // `run: tools/release-all` and `uses: ./.github/actions/publish` each produced no
    // finding at all — the upload moved into a file, and the file was never opened.
    // 步骤可以把上传交给本地脚本或本地 action，而不是自己写出命令。门禁过去只读步骤自己的行，
    // 因此 `run: tools/release-all` 与 `uses: ./.github/actions/publish` 都不产生任何发现——
    // 上传搬进了一个文件，而那个文件从未被打开。
    root.is_some_and(|root| {
        own_lines
            .filter_map(|line| delegated_path(root, line))
            .any(|path| file_uploads(&path))
    })
}

/// Whether one line names an upload command, after collapsing whitespace runs.
/// 折叠空白串之后，某一行是否点明一条上传命令。
fn names_upload(line: &str) -> bool {
    let mut collapsed = String::with_capacity(line.len());
    let mut previous_space = false;
    for character in line.chars() {
        if character.is_whitespace() {
            if !previous_space {
                collapsed.push(' ');
            }
            previous_space = true;
            continue;
        }
        previous_space = false;
        collapsed.push(character);
    }
    UPLOAD_COMMANDS
        .iter()
        .any(|needle| collapsed.contains(needle))
}

/// Join a `\`-continued command onto one line, so the continuation cannot hide it.
/// 把以 `\` 续行的命令接成一行，使续行藏不住它。
fn join_continuations(body: &str) -> String {
    let mut joined = String::with_capacity(body.len());
    for line in body.lines() {
        let trimmed = line.trim_end();
        match trimmed.strip_suffix('\\') {
            Some(head) => {
                joined.push_str(head);
                joined.push(' ');
            }
            None => {
                joined.push_str(line);
                joined.push('\n');
            }
        }
    }
    joined
}

/// The local file a `run:` or `uses:` line hands the work to, if it names one.
/// 某条 `run:` 或 `uses:` 行把工作交给的本地文件——如果它点名了一个。
fn delegated_path(root: &Path, line: &str) -> Option<std::path::PathBuf> {
    // `uses:` names an action; only a local one (`./…`) is a file in this checkout, and a
    // `…@ref` suffix is a version, not part of the path. `run:` names a command line,
    // whose first word is the program.
    // `uses:` 命名的是一条 action；只有本地的（`./…`）才是本检出里的文件，而 `…@ref` 后缀是
    // 版本，不属于路径。`run:` 命名的是一条命令行，其第一个词就是程序。
    let local = if let Some(uses) = line.strip_prefix("uses:") {
        let value = uses.trim().split('@').next().unwrap_or("").trim();
        value.starts_with("./").then_some(value)
    } else if let Some(run) = line.strip_prefix("run:") {
        let command = run.split_whitespace().next().unwrap_or("");
        (command.starts_with("./") || command.starts_with("tools/")).then_some(command)
    } else {
        None
    }?;
    if local == KNOWN_PUBLISHER {
        return None;
    }
    let path = root.join(local);
    // A local action is a directory; the entry file is what a runner reads.
    // 本地 action 是一个目录；runner 读的是其中的入口文件。
    [
        path.clone(),
        path.join("action.yml"),
        path.join("action.yaml"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
}

/// Whether a delegated file uploads, by the same command test.
/// 被委托的文件是否上传，用的是同一条命令判定。
fn file_uploads(path: &Path) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.starts_with('#'))
        .any(names_upload)
}

use release_guard::{GuardShape, guard_shape, takes_input};

/// Every step as `(the step's lines, its assembled `if:` condition)`.
/// 每个步骤，形如 `(该步骤的各行, 拼起来的 `if:` 条件)`。
fn steps(text: &str) -> Vec<(String, Option<String>)> {
    let mut parsed: Vec<(String, Option<String>)> = Vec::new();
    let mut current: Option<(usize, String, Option<String>)> = None;
    let mut folded: Option<usize> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        // A YAML sequence entry may be a bare `-` with its keys on the lines below, which
        // `steps()` used to miss entirely: the step was invisible, so neither its upload
        // nor the missing `workflow_dispatch` was ever reported.
        // YAML 序列条目可以是一个裸 `-`，键写在下面几行；`steps()` 过去完全看不见它：那个步骤
        // 不可见，于是它的上传与"没有 workflow_dispatch"都不会被报出来。
        if trimmed == "-" || trimmed.starts_with("- ") {
            if let Some((_, body, condition)) = current.take() {
                parsed.push((body, condition));
            }
            current = Some((indent, String::new(), None));
            folded = None;
        }
        let Some((step_indent, body, condition)) = current.as_mut() else {
            continue;
        };
        if !trimmed.is_empty()
            && indent <= *step_indent
            && !(trimmed == "-" || trimmed.starts_with("- "))
        {
            // A sibling key at the step's own indentation ends the step.
            // 与步骤同缩进的兄弟键结束该步骤。
            continue;
        }
        let content = trimmed.strip_prefix("- ").unwrap_or(trimmed);
        body.push_str(content);
        body.push('\n');
        // A folded or literal `if:` puts its value on the lines below, which is how a
        // correctly guarded step used to be reported as unguarded.
        // 折叠式或字面量式 `if:` 把取值放在下面几行，一个守卫正确的步骤因此被报成没有守卫。
        if let Some(if_indent) = folded {
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            // The fold ends where the `if:` key's own indentation ends, not where the
            // sequence entry's does: a sibling key (`run:`) at the entry's body indent used
            // to be read as another line of the condition, so a shape test would see the
            // command as part of the guard.
            // 折叠在 `if:` 键自身的缩进处结束，而不是在序列条目的缩进处：与条目同级的兄弟键
            // （`run:`）过去会被读成条件的又一行，于是形状判定会把命令当成守卫的一部分。
            if indent > if_indent {
                let text = condition.get_or_insert_with(String::new);
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(trimmed);
                continue;
            }
            folded = None;
        }
        if let Some(value) = content.strip_prefix("if:") {
            let value = value.trim();
            let is_folded = matches!(value, ">" | ">-" | ">+" | "|" | "|-" | "|+");
            *condition = Some(if is_folded {
                String::new()
            } else {
                value.to_owned()
            });
            folded = is_folded.then_some(indent);
        }
    }
    if let Some((_, body, condition)) = current {
        parsed.push((body, condition));
    }
    parsed
}

#[cfg(test)]
#[path = "release_workflow_tests.rs"]
mod release_workflow_tests;
