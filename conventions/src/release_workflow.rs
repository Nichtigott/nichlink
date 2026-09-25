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
//! the *presence of a substring* (`!startsWith(github.ref, 'refs/tags/')` passes a
//! `contains` test while publishing on every non-tag ref). It now selects steps by the
//! command they run and requires the positive form of the tag test. It also reads every
//! workflow in `.github/workflows/`, because a second file is a second way to publish.
//! 两轮对抗性审查让本门禁不再相信两样东西：步骤的**名字**（换名字或不写名字的上传步骤过去
//! 不可见）与**子串是否出现**（`!startsWith(github.ref, 'refs/tags/')` 能通过 `contains`
//! 检查，却在每个非 tag ref 上发布）。现在它按步骤运行的命令挑选步骤，并要求 tag 判断为**肯定**
//! 形式。它还读取 `.github/workflows/` 下的每个工作流，因为第二个文件就是第二条发布路径。

use std::fs;
use std::path::Path;

/// The commands that upload, whichever step they sit in.
/// 会执行上传的命令，无论它位于哪个步骤。
const UPLOAD_COMMANDS: &[&str] = &["--publish", "cargo publish"];

/// The positive tag test a step that uploads has to carry.
/// 上传步骤必须带着的肯定式 tag 判断。
const TAG_GUARD: &str = "startsWith(github.ref, 'refs/tags/')";

/// Report every way the workflow lets an upload happen without a tag ref.
/// 报告该工作流容许在没有 tag ref 的情况下上传的每一种方式。
///
/// Empty means the properties hold: no input can make a manual run differ from a
/// rehearsal, every step that uploads carries the positive tag requirement itself, and
/// a workflow that can upload still offers the manual rehearsal path.
/// 返回空表示这些性质都成立：没有任何输入能让手动运行与演练不同、每个上传步骤自己带着肯定式 tag
/// 要求、而能够上传的工作流仍保留手动演练路径。
pub fn findings(text: &str) -> Vec<String> {
    let mut findings = Vec::new();
    if takes_input(text) {
        findings.push(
            "the workflow reads an input, so a manual run can differ from a rehearsal; \
             uploads have to be reachable only from a tag ref"
                .to_owned(),
        );
    }
    if uploads(text) && !text.contains("workflow_dispatch:") {
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
        if !step_uploads(body) {
            continue;
        }
        let Some(condition) = condition else {
            findings.push(format!(
                "step {} uploads with no `if:` at all, so a branch run would publish",
                index + 1
            ));
            continue;
        };
        if condition.contains("!startsWith") || condition.contains("!=") || condition.contains("||")
        {
            findings.push(format!(
                "step {} uploads under a condition that can be true without a tag ref: \
                 `{condition}`",
                index + 1
            ));
            continue;
        }
        if !condition.contains(TAG_GUARD) {
            findings.push(format!(
                "step {} uploads without a positive tag guard; its own `if:` must name \
                 `{TAG_GUARD}` rather than trusting the trigger list: `{condition}`",
                index + 1
            ));
        }
    }
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
        for finding in findings(&text) {
            found.push(format!("{name}: {finding}"));
        }
    }
    found
}

/// Whether the workflow can upload anything.
/// 该工作流是否可能上传任何东西。
fn uploads(text: &str) -> bool {
    steps(text).iter().any(|(body, _)| step_uploads(body))
}

/// Whether one step's lines run an upload.
/// 某个步骤的各行是否运行一次上传。
///
/// Comments are skipped: `ci.yml` mentions the release workflow's `--publish` in prose,
/// and a whole-file substring search read that as an upload step.
/// 注释被跳过：`ci.yml` 在散文里提到发布工作流的 `--publish`，而对整个文件做子串搜索会把它读成
/// 一个上传步骤。
fn step_uploads(body: &str) -> bool {
    body.lines()
        .map(|line| line.trim_start_matches("- ").trim())
        .filter(|line| !line.starts_with('#'))
        .any(|line| UPLOAD_COMMANDS.iter().any(|needle| line.contains(needle)))
}

/// Whether the workflow reads an input, declared or referenced.
/// 该工作流是否读取输入——声明或引用都算。
fn takes_input(text: &str) -> bool {
    text.contains("inputs.") || text.lines().any(|line| line.trim() == "inputs:")
}

/// Every step as `(the step's lines, its assembled `if:` condition)`.
/// 每个步骤，形如 `(该步骤的各行, 拼起来的 `if:` 条件)`。
fn steps(text: &str) -> Vec<(String, Option<String>)> {
    let mut parsed: Vec<(String, Option<String>)> = Vec::new();
    let mut current: Option<(usize, String, Option<String>)> = None;
    let mut folded = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if trimmed.starts_with("- ") {
            if let Some((_, body, condition)) = current.take() {
                parsed.push((body, condition));
            }
            current = Some((indent, String::new(), None));
            folded = false;
        }
        let Some((step_indent, body, condition)) = current.as_mut() else {
            continue;
        };
        if !trimmed.is_empty() && indent <= *step_indent && !trimmed.starts_with("- ") {
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
        if folded {
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if indent > *step_indent {
                let text = condition.get_or_insert_with(String::new);
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(trimmed);
                continue;
            }
            folded = false;
        }
        if let Some(value) = content.strip_prefix("if:") {
            let value = value.trim();
            let is_folded = matches!(value, ">" | ">-" | ">+" | "|" | "|-" | "|+");
            *condition = Some(if is_folded {
                String::new()
            } else {
                value.to_owned()
            });
            folded = is_folded;
        }
    }
    if let Some((_, body, condition)) = current {
        parsed.push((body, condition));
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The workflows in this checkout satisfy every property.
    /// 本检出里的工作流满足全部性质。
    #[test]
    fn the_shipped_workflows_only_publish_a_tag() {
        let found = workflow_findings(&crate::workspace_root());
        assert!(found.is_empty(), "a shipped workflow regressed: {found:#?}");
    }

    /// The condition this gate exists for: an upload step that trusts the trigger list,
    /// with the tag check left to a different step.
    /// 本门禁为之存在的那个条件：一个信任触发列表的上传步骤，而 tag 检查被留给另一个步骤。
    #[test]
    fn an_upload_without_a_positive_guard_is_reported() {
        let text = "\
on:
  workflow_dispatch:
jobs:
  release:
    steps:
      - name: Whatever
        if: github.event_name == 'push'
        run: tools/nichlink-publish --publish --yes
";
        let found = findings(text);
        assert!(
            found
                .iter()
                .any(|finding| finding.contains("positive tag guard")),
            "an upload reachable from a branch must be reported: {found:#?}"
        );
    }

    /// The bypass a `contains` test accepted: the negated tag test.
    /// 一个 `contains` 检查接受的绕过：取反的 tag 判断。
    #[test]
    fn a_negated_tag_test_is_reported() {
        let text = "\
on:
  workflow_dispatch:
jobs:
  release:
    steps:
      - name: Publish
        if: github.event_name == 'push' && !startsWith(github.ref, 'refs/tags/')
        run: tools/nichlink-publish --publish --yes
";
        let found = findings(text);
        assert!(
            found
                .iter()
                .any(|finding| finding.contains("true without a tag ref")),
            "a negated guard publishes on branches: {found:#?}"
        );
    }

    /// A step named anything at all, and one with no name, are both seen: the gate keys
    /// on the command, not on the step's name.
    /// 名字任意的步骤、以及没有名字的步骤都能被看到：门禁以命令为准，而不是步骤名。
    #[test]
    fn the_upload_command_matters_not_the_step_name() {
        let text = "\
on:
  workflow_dispatch:
jobs:
  release:
    steps:
      - name: Rename me
        if: github.event_name == 'push'
        run: tools/nichlink-publish --publish --yes
      - if: github.event_name == 'push'
        run: cargo publish --workspace
";
        let found = findings(text);
        assert_eq!(
            found
                .iter()
                .filter(|finding| finding.contains("positive tag guard"))
                .count(),
            2,
            "both uploads are unguarded whatever they are called: {found:#?}"
        );
    }

    /// A folded condition is read as one condition, so a correctly guarded step is not
    /// reported.
    /// 折叠条件作为一个条件读取，因此守卫正确的步骤不会被报出。
    #[test]
    fn a_folded_positive_guard_is_accepted() {
        let text = "\
on:
  workflow_dispatch:
jobs:
  release:
    steps:
      - name: Publish in dependency order
        if: >-
          github.event_name == 'push' &&
          startsWith(github.ref, 'refs/tags/')
        run: tools/nichlink-publish --publish --yes
";
        let found = findings(text);
        assert!(found.is_empty(), "{found:#?}");
    }

    /// A second workflow file is read like the first.
    /// 第二个工作流文件与第一个一样被读取。
    #[test]
    fn a_second_workflow_is_checked_too() {
        let root = std::env::temp_dir().join(format!("nichlink-workflows-{}", std::process::id()));
        let workflows = root.join(".github/workflows");
        fs::create_dir_all(&workflows).expect("fixture directory");
        fs::write(
            workflows.join("release.yml"),
            "on:\n  workflow_dispatch:\njobs:\n  release:\n    steps:\n      - name: P\n        \
             if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')\n        \
             run: tools/nichlink-publish --publish --yes\n",
        )
        .expect("fixture workflow");
        fs::write(
            workflows.join("extra.yml"),
            "on:\n  push:\n    branches: [\"main\"]\njobs:\n  extra:\n    steps:\n      - \
             run: tools/nichlink-publish --publish --yes\n",
        )
        .expect("fixture workflow");
        let found = workflow_findings(&root);
        assert!(
            found
                .iter()
                .any(|finding| finding.starts_with("extra.yml:")),
            "a second workflow that publishes is reported: {found:#?}"
        );
        assert!(
            !found
                .iter()
                .any(|finding| finding.starts_with("release.yml:")),
            "the guarded workflow stays clean: {found:#?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// A manual input is still reported on its own.
    /// 手动输入依然单独被报出。
    #[test]
    fn a_manual_input_is_reported() {
        let text = "\
on:
  workflow_dispatch:
    inputs:
      publish:
        type: boolean
jobs:
  release:
    steps:
      - name: Publish
        if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')
        run: tools/nichlink-publish --publish --yes
";
        let found = findings(text);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert!(found[0].contains("reads an input"), "{found:#?}");
    }
}
