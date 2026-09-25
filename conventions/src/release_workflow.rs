//! The release workflow's one hard precondition, as a gate.
//! 把发布工作流唯一硬性前提变成门禁。
//!
//! `cargo publish` cannot be taken back, and the only thing that ties an upload to
//! a version is the tag. The workflow says so itself ("a release is a tag and
//! nothing else starts it"), and `CHANGELOG.md` promises that a manual run
//! rehearses every check without publishing. Both sentences were false: a
//! `publish` input made the upload step reachable from a branch, with the tag
//! check skipped, so a manual run could upload whatever version the manifests
//! happened to carry. A promise in a comment decays exactly like a rule in prose,
//! so this module reads the shipped YAML and refuses the shapes that break it.
//! `cargo publish` 收不回来，而把一次上传绑定到某个版本的东西只有 tag。工作流自己这么说
//! （"一次发布就是一个 tag，且只有 tag 会启动它"），`CHANGELOG.md` 也承诺手动运行会演练全部
//! 检查而不发布。两句话都曾是假的：一个 `publish` 输入让上传步骤可以从分支到达、且跳过 tag
//! 检查，于是手动运行能上传清单里恰好写着的任何版本。注释里的承诺会像散文里的规则一样腐化，
//! 因此本模块读实际出厂的 YAML，拒绝破坏它的那些形状。

/// Steps that must not run unless the ref is a tag.
/// 除非 ref 是 tag，否则不得运行的步骤。
///
/// Each one either uploads or validates an upload, so a run that reached it on a
/// branch would be either irreversible or meaningless.
/// 它们要么在上传，要么在校验一次上传，因此一次在分支上到达它们的运行要么不可撤销、要么毫无
/// 意义。
pub const GUARDED_STEPS: &[&str] = &[
    "Publish in dependency order",
    "A stranger can consume the release",
];

/// Report every way the workflow lets an upload happen without a tag ref.
/// 报告该工作流容许在没有 tag ref 的情况下上传的每一种方式。
///
/// Empty means the three properties hold: no input can make a manual run differ
/// from a rehearsal, every guarded step carries the tag requirement itself, and
/// the manual rehearsal path still exists for a release that has no tag yet.
/// 返回空表示三条性质都成立：没有任何输入能让手动运行与演练不同、每个受守卫的步骤自己带着
/// tag 要求、而尚未打 tag 的发布仍有手动演练路径。
pub fn findings(text: &str) -> Vec<String> {
    let mut findings = Vec::new();
    // The trigger list is not the guard. A step condition that repeats the tag
    // requirement keeps holding when someone later adds a `branches:` trigger or
    // a new event; a condition that reads `inputs.publish` does not.
    // 触发列表不是守卫。重复写出 tag 要求的步骤条件，在以后有人加上 `branches:` 触发或新事件
    // 时依然成立；而读 `inputs.publish` 的条件不成立。
    // Both spellings count: the declaration under `workflow_dispatch` (`inputs:`) and
    // any reference to it (`inputs.publish`, `${{ inputs.publish }}`). The
    // declaration alone is enough to make a manual run differ from a rehearsal, and
    // the reference alone is enough to act on it — reporting only the reference
    // would leave the gap this test caught.
    // 两种拼法都要算：`workflow_dispatch` 下的声明（`inputs:`）与对它的任何引用
    // （`inputs.publish`、`${{ inputs.publish }}`）。只有声明就足以让手动运行与演练不同，只有
    // 引用就足以据此行动——只报引用会留下这个测试抓到的缺口。
    let takes_input = text.contains("inputs.") || text.lines().any(|line| line.trim() == "inputs:");
    if takes_input {
        findings.push(
            "the release workflow reads an input, so a manual run can differ from a rehearsal; \
             uploads have to be reachable only from a tag ref"
                .to_owned(),
        );
    }
    if !text.contains("workflow_dispatch:") {
        findings.push(
            "the release workflow has no workflow_dispatch, so a release cannot be rehearsed \
             before its tag exists"
                .to_owned(),
        );
    }
    let parsed = steps(text);
    for name in GUARDED_STEPS {
        match parsed.iter().find(|(step, _)| step == name) {
            None => findings.push(format!(
                "step `{name}` is gone; this gate guards it by name and must be updated with it"
            )),
            Some((_, body)) => {
                let guarded = body.lines().any(|line| {
                    let trimmed = line.trim_start();
                    trimmed.starts_with("if:") && trimmed.contains("refs/tags/")
                });
                if !guarded {
                    findings.push(format!(
                        "step `{name}` can run without a tag ref: its own `if:` must name \
                         `refs/tags/` rather than trusting the trigger list"
                    ));
                }
            }
        }
    }
    findings
}

/// Split a workflow into its steps as `(name, body)`.
/// 把工作流拆成 `(名字, 主体)` 形式的步骤。
///
/// Only the shape this file has is understood: a step starts at `- name:` and owns
/// the more-indented lines below it. That is enough to answer the one question
/// above, and deliberately less than a YAML parser, which this crate must not
/// grow a dependency for.
/// 只理解本文件具有的形状：一个步骤从 `- name:` 开始，拥有其下缩进更深的行。这足以回答上面
/// 那一个问题，而且刻意少于一个 YAML 解析器——本 crate 不该为此长出一个依赖。
fn steps(text: &str) -> Vec<(String, String)> {
    let mut parsed: Vec<(String, String)> = Vec::new();
    let mut current: Option<(String, usize, String)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if let Some(name) = trimmed.strip_prefix("- name:") {
            if let Some((name, _, body)) = current.take() {
                parsed.push((name, body));
            }
            current = Some((name.trim().to_owned(), indent, String::new()));
            continue;
        }
        if let Some((_, step_indent, body)) = current.as_mut()
            && (trimmed.is_empty() || indent > *step_indent)
        {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some((name, _, body)) = current {
        parsed.push((name, body));
    }
    parsed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The workflow in this checkout satisfies all three properties.
    /// 本检出里的工作流满足全部三条性质。
    #[test]
    fn the_shipped_release_workflow_only_publishes_a_tag() {
        let path = crate::workspace_root().join(".github/workflows/release.yml");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let found = findings(&text);
        assert!(
            found.is_empty(),
            "the shipped workflow regressed: {found:#?}"
        );
    }

    /// The condition this gate exists for: an upload step that trusts the trigger
    /// list, with the tag check left on a different step.
    /// 本门禁为之存在的那个条件：一个信任触发列表的上传步骤，而 tag 检查被留在另一个步骤上。
    #[test]
    fn a_publish_step_that_can_run_on_a_branch_is_reported() {
        let text = "\
jobs:
  release:
    steps:
      - name: Publish in dependency order
        if: github.event_name == 'push' || inputs.publish
        run: tools/nichlink-publish --publish --yes
      - name: A stranger can consume the release
        if: github.event_name == 'push' || inputs.publish
        run: tools/nichlink-publish --verify-consumers
on:
  workflow_dispatch:
";
        let found = findings(text);
        assert!(
            found
                .iter()
                .any(|f| f.contains("Publish in dependency order")),
            "an upload reachable from a branch must be reported: {found:#?}"
        );
        assert!(
            found.iter().any(|f| f.contains("reads an input")),
            "the input that makes a manual run publish must be reported: {found:#?}"
        );
    }

    /// A guarded step is still not enough on its own: a manual input that changes
    /// what the run does is reported on its own, whatever the steps say.
    /// 仅有受守卫的步骤还不够：改变运行行为的手动输入自身就会被报出，无论步骤怎么写。
    #[test]
    fn a_manual_input_is_reported_even_when_every_step_is_guarded() {
        let text = "\
on:
  workflow_dispatch:
    inputs:
      publish:
        type: boolean
jobs:
  release:
    steps:
      - name: Publish in dependency order
        if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')
        run: tools/nichlink-publish --publish --yes
      - name: A stranger can consume the release
        if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')
        run: tools/nichlink-publish --verify-consumers
";
        let found = findings(text);
        assert_eq!(found.len(), 1, "{found:#?}");
        assert!(found[0].contains("reads an input"), "{found:#?}");
    }

    /// A step renamed away must not read as clean: the gate guards it by name.
    /// 被改名而消失的步骤不能读作干净：门禁是按名字守它的。
    #[test]
    fn a_guarded_step_that_was_renamed_is_reported() {
        let text = "\
on:
  workflow_dispatch:
jobs:
  release:
    steps:
      - name: Publish in dependency order
        if: startsWith(github.ref, 'refs/tags/')
        run: tools/nichlink-publish --publish --yes
";
        let found = findings(text);
        assert!(
            found
                .iter()
                .any(|f| f.contains("A stranger can consume the release")),
            "{found:#?}"
        );
    }
}
