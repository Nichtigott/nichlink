//! The release-workflow gate's tests, in their own file so the gate stays inside the
//! line ceiling — the split `doc_blocks_tests.rs` and `purity_tests.rs` record.
//! 发布工作流门禁的测试，放在独立文件里使门禁留在行数上限之内——与 `doc_blocks_tests.rs`、
//! `purity_tests.rs` 记录的同一种拆分。

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

/// The negation category a substring test cannot see: `!(startsWith(…))` and the spaced
/// `! startsWith(…)` both *contain* the positive tag test, so a `contains` check accepted
/// them while the step published on every non-tag ref. The shape is what has to be judged,
/// not the substring.
/// 子串判定看不见的取反类别：`!(startsWith(…))` 与带空格的 `! startsWith(…)` 都**含有**那个
/// 肯定式 tag 判断，因此 `contains` 检查接受它们，而那个步骤在每个非 tag ref 上都会发布。
/// 要判定的是形状，不是子串。
#[test]
fn both_spellings_of_a_negated_tag_test_are_reported() {
    let fixtures = [
        ("a negated call", "!(startsWith(github.ref, 'refs/tags/'))"),
        (
            "a spaced negation",
            "! startsWith(github.ref, 'refs/tags/')",
        ),
    ];
    let mut missed = Vec::new();
    for (name, condition) in fixtures {
        let text = format!(
            "on:\n  workflow_dispatch:\njobs:\n  release:\n    steps:\n      - name: P\n        \
             if: {condition}\n        run: tools/nichlink-publish --publish --yes\n"
        );
        let found = findings(&text);
        if !found
            .iter()
            .any(|finding| finding.contains("true without a tag ref"))
        {
            missed.push(format!("{name}: {found:#?}"));
        }
    }
    assert!(
        missed.is_empty(),
        "a negated guard publishes on branches:\n{}",
        missed.join("\n")
    );
}

/// The whitelist is a whitelist. Each of these carries the tag test as text and still
/// cannot be read as a conjunction of positive tests, so each is reported.
/// 白名单就是白名单。下列每条都以文本形式带着 tag 判断，却都读不成"肯定式测试的合取"，因此
/// 都被报出。
///
/// The last two are the spellings a substring blacklist could not see (audit `LGC-LG-25`):
/// `startsWith(github.ref, 'refs/tags/') == false` *contains* the positive tag test and
/// contains none of `!`, `!=`, `||`, so a `contains` test called it a correct guard while
/// GitHub read it as "only on a non-tag ref" — a publish on every branch push. t16 measured
/// four such equivalent negations passing the old check with zero findings.
/// 最后两条是子串黑名单看不见的拼法（审计 `LGC-LG-25`）：
/// `startsWith(github.ref, 'refs/tags/') == false` **含有**那个肯定式 tag 判断，且不含 `!`、
/// `!=`、`||`，因此 `contains` 判定会称它为正确守卫，而 GitHub 把它读成"只在非 tag ref 上"——
/// 每次分支 push 都发布。t16 实测四种等价否定在旧检查下都是 0 发现。
#[test]
fn shapes_off_the_whitelist_are_reported() {
    let fixtures = [
        (
            "a disjunction",
            "github.event_name == 'push' || startsWith(github.ref, 'refs/tags/')",
        ),
        (
            "an inequality",
            "github.ref != 'refs/tags/v0.1.0' && startsWith(github.ref, 'refs/tags/')",
        ),
        (
            "an operator the gate does not enumerate",
            "github.run_number > 3 && startsWith(github.ref, 'refs/tags/')",
        ),
        (
            "a comparison to false",
            "github.event_name == 'push' && startsWith(github.ref, 'refs/tags/') == false",
        ),
        (
            "a comparison to zero",
            "startsWith(github.ref, 'refs/tags/') == 0",
        ),
    ];
    let mut missed = Vec::new();
    for (name, condition) in fixtures {
        let text = format!(
            "on:\n  workflow_dispatch:\njobs:\n  release:\n    steps:\n      - name: P\n        \
             if: {condition}\n        run: tools/nichlink-publish --publish --yes\n"
        );
        let found = findings(&text);
        if !found
            .iter()
            .any(|finding| finding.contains("true without a tag ref"))
        {
            missed.push(format!("{name}: {found:#?}"));
        }
    }
    assert!(
        missed.is_empty(),
        "a shape off the whitelist is reported:\n{}",
        missed.join("\n")
    );
}

/// The other half of a whitelist: the shapes it does allow stay silent, so the gate does
/// not fire on the spellings this repository ships and on the ones a maintainer would
/// reach for.
/// 白名单的另一半：它允许的形状保持沉默，因此门禁不会在本仓库出厂的拼法上、也不会在维护者会
/// 顺手写出的拼法上误报。
#[test]
fn positive_guard_shapes_are_accepted() {
    let fixtures = [
        ("the tag test alone", "startsWith(github.ref, 'refs/tags/')"),
        (
            "a conjunction",
            "github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')",
        ),
        (
            "a parenthesized conjunction",
            "(github.event_name == 'push' && startsWith(github.ref, 'refs/tags/'))",
        ),
        (
            "the explicit expression wrapper",
            "${{ github.event_name == 'push' && startsWith(github.ref, 'refs/tags/') }}",
        ),
    ];
    for (name, condition) in fixtures {
        let text = format!(
            "on:\n  workflow_dispatch:\njobs:\n  release:\n    steps:\n      - name: P\n        \
             if: {condition}\n        run: tools/nichlink-publish --publish --yes\n"
        );
        let found = findings(&text);
        assert!(
            found.is_empty(),
            "{name}: an allowed positive shape is accepted: {found:#?}"
        );
    }
}

/// `inputs['publish']` is the same read as `inputs.publish`, and the substring test read
/// neither: the bracket spelling produced no finding at all, so an input could make a
/// manual run differ from a rehearsal without the gate saying so.
/// `inputs['publish']` 与 `inputs.publish` 是同一次读取，而子串判定两者都读不到：方括号写法
/// 完全不产生发现，因此一个输入可以让手动运行与演练不同，而门禁什么都不说。
#[test]
fn a_bracketed_input_reference_is_reported() {
    let text = "\
on:
  workflow_dispatch:
jobs:
  release:
    steps:
      - name: Publish
        if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')
        run: tools/nichlink-publish --publish --yes
      - name: Rehearse
        run: echo \"${{ inputs['publish'] }}\"
";
    let found = findings(text);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].contains("reads an input"), "{found:#?}");
}

/// Both reference spellings are read, and prose in a comment is not a reference — the
/// same boundary the upload test already draws, because a comment cannot change what a
/// run does.
/// 两种引用拼法都会被读到，而注释里的散文不是引用——与上传判定早已画出的边界相同，因为注释
/// 改变不了运行的行为。
#[test]
fn an_input_reference_is_read_in_either_spelling_and_not_in_prose() {
    let head = "\
on:
  workflow_dispatch:
jobs:
  release:
    steps:
      - name: Publish
        if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')
        run: tools/nichlink-publish --publish --yes
      - name: Rehearse
        run: ";
    let dotted = format!("{head}echo \"${{{{ inputs.publish }}}}\"\n");
    let found = findings(&dotted);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(found[0].contains("reads an input"), "{found:#?}");

    let prose = format!("# a manual run used to tick inputs.publish\n{head}echo rehearsal\n");
    let found = findings(&prose);
    assert!(
        found.is_empty(),
        "a comment cannot read an input: {found:#?}"
    );
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

/// Three legal spellings that used to hide an unguarded upload: two spaces between
/// `cargo` and `publish`, a `\`-continued command, and a bare `-` sequence entry.
/// Each produced zero findings, and the bare `-` additionally hid the missing
/// `workflow_dispatch`, because `steps()` never opened the step.
/// 三种合法拼写过去都能藏住一次无守卫的上传：`cargo` 与 `publish` 之间两个空格、用 `\`
/// 续行的命令、以及裸 `-` 的序列条目。三者都产出 0 条发现，而裸 `-` 还顺带藏住了"缺少
/// `workflow_dispatch`"，因为 `steps()` 从未打开那个步骤。
#[test]
fn whitespace_and_yaml_spellings_do_not_hide_an_upload() {
    let fixtures = [
        (
            "two spaces",
            "      - name: P\n        run: cargo  publish --workspace\n",
        ),
        (
            "a continuation",
            "      - name: P\n        run: cargo \\\n          publish --workspace\n",
        ),
        (
            "a bare dash",
            "      -\n        run: cargo publish --workspace\n",
        ),
    ];
    for (name, steps) in fixtures {
        // Deliberately no `workflow_dispatch:`: the missing rehearsal path is the
        // second finding the bare-`-` spelling used to swallow.
        // 有意不写 `workflow_dispatch:`：缺少演练路径正是裸 `-` 那种拼写过去吞掉的第二条发现。
        let text = format!("on:\n  push:\njobs:\n  release:\n    steps:\n{steps}");
        let found = findings(&text);
        assert!(
            found
                .iter()
                .any(|finding| finding.contains("no `if:` at all")),
            "{name}: the unguarded upload is reported: {found:#?}"
        );
        assert!(
            found
                .iter()
                .any(|finding| finding.contains("workflow_dispatch")),
            "{name}: a branch-triggered publisher is reported: {found:#?}"
        );
    }
}

/// A step that hands the upload to a local script or a local action is followed:
/// `run: tools/release-all` and `uses: ./.github/actions/publish` each produced no
/// finding while the upload sat in a file the gate never opened.
/// 把上传交给本地脚本或本地 action 的步骤会被跟进：`run: tools/release-all` 与
/// `uses: ./.github/actions/publish` 过去都不产生任何发现，而上传就在门禁从未打开的文件里。
#[test]
fn a_delegated_upload_is_followed() {
    let root = std::env::temp_dir().join(format!("nichlink-delegated-{}", std::process::id()));
    let workflows = root.join(".github/workflows");
    let action = root.join(".github/actions/publish");
    fs::create_dir_all(&workflows).expect("fixture directory");
    fs::create_dir_all(&action).expect("action directory");
    fs::create_dir_all(root.join("tools")).expect("tools directory");
    fs::write(
        root.join("tools/release-all"),
        "#!/bin/sh\ncargo publish --workspace\n",
    )
    .expect("script");
    fs::write(
        action.join("action.yml"),
        "runs:\n  using: composite\n  steps:\n    - run: cargo publish --workspace\n",
    )
    .expect("action");
    fs::write(
        workflows.join("script.yml"),
        "on:\n  workflow_dispatch:\njobs:\n  release:\n    steps:\n      - run: tools/release-all\n",
    )
    .expect("fixture workflow");
    fs::write(
        workflows.join("action.yml"),
        "on:\n  workflow_dispatch:\njobs:\n  release:\n    steps:\n      - uses: ./.github/actions/publish\n",
    )
    .expect("fixture workflow");

    let found = workflow_findings(&root);
    assert!(
        found
            .iter()
            .any(|finding| finding.starts_with("script.yml:")),
        "a step that runs a local publishing script is followed: {found:#?}"
    );
    assert!(
        found
            .iter()
            .any(|finding| finding.starts_with("action.yml:")),
        "a step that uses a local publishing action is followed: {found:#?}"
    );

    // The rehearsal modes of the one publisher the gate knows by name stay clean:
    // following that file would read its `--publish` branch and flag every one.
    // 门禁按名字认识的那个发布器，其演练模式保持干净：跟进那个文件会读到它的 `--publish`
    // 分支，从而把每一个都报出来。
    fs::write(
        workflows.join("rehearsal.yml"),
        "on:\n  workflow_dispatch:\njobs:\n  release:\n    steps:\n      - run: tools/nichlink-publish --check-table\n",
    )
    .expect("fixture workflow");
    fs::write(
        root.join("tools/nichlink-publish"),
        "#!/bin/sh\nif [ \"$1\" = --publish ]; then cargo publish --workspace; fi\n",
    )
    .expect("the known publisher");
    let found = workflow_findings(&root);
    assert!(
        !found
            .iter()
            .any(|finding| finding.starts_with("rehearsal.yml:")),
        "a rehearsal of the known publisher is not an upload: {found:#?}"
    );
    let _ = fs::remove_dir_all(&root);
}
