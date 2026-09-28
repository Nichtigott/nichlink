//! The commit-pin rule's pins, in their own file so the gate stays inside the line ceiling.
//! 提交 pin 规则的钉子，放在独立文件里使门禁留在行数上限之内。

use super::*;
use crate::workspace_root;

/// A workflow whose single job holds the token and leaves an action on a moving tag is
/// reported, and the same workflow with the commit address is not.
/// 唯一的 job 持有 token、而某个 action 仍挂在会移动的 tag 上的工作流会被报出；同一个工作流换成
/// 提交地址后不会。
///
/// Red before the fix: nothing read `uses:` at all, so both spellings were green — a retag of
/// `actions/checkout` would have run on the publish path with the secret in scope (audit
/// `LGC-LG-26`).
/// 修前为红：没有任何东西读 `uses:`，因此两种拼法都全绿——`actions/checkout` 被重新打 tag 后
/// 会在发布路径上运行，而秘密就在它的作用域里（审计 `LGC-LG-26`）。
#[test]
fn a_token_holding_job_must_pin_its_actions() {
    let pinned = "11d5960a326750d5838078e36cf38b85af677262";
    let workflow = |reference: &str| {
        format!(
            "on:\n  push:\n    tags: ['v*']\njobs:\n  release:\n    runs-on: ubuntu-latest\n    \
             steps:\n      - uses: actions/checkout@{reference}\n      - run: cargo publish\n        \
             env:\n          CARGO_REGISTRY_TOKEN: ${{{{ secrets.CARGO_REGISTRY_TOKEN }}}}\n"
        )
    };
    let found = findings(&workflow("v4"));
    assert_eq!(
        found.len(),
        1,
        "an unpinned action in the token-holding job must be reported once: {found:#?}"
    );
    assert!(
        found[0].contains("actions/checkout") && found[0].contains("40-character commit SHA"),
        "the report names the action and the rule: {found:#?}"
    );

    let clean = findings(&workflow(pinned));
    assert!(
        clean.is_empty(),
        "a commit-pinned action needs no report: {clean:#?}"
    );
}

/// A job that activates every secret inherits the registry token, so its remote workflow
/// reference has to be pinned too.
/// 把每个秘密都启用的 job 继承了 registry token，因此它引用的远程工作流也必须按提交 pin。
///
/// `secrets: inherit` is how a reusable-workflow call receives the token without naming it,
/// and this rule's holder test used to look for the *name*: a relay job could take the whole
/// secret set, call an upstream workflow at a moving tag, and the gate stayed silent
/// (`N-3`, measured in the B4-conventions independent verification). Not seeing the name is
/// not the same as not having the secret.
/// `secrets: inherit` 是可复用工作流调用在不点名的情况下拿到 token 的方式，而本规则的持有者判据
/// 过去只找那个**名字**：一个中继 job 可以取走整套秘密、在会移动的 tag 上调用上游工作流，而门禁
/// 一声不吭（`N-3`，在 B4-conventions 的独立验证里实测）。名字没出现不等于秘密没到手。
///
/// The second half is the legal control: the same relay shape pinned by commit is clean, so
/// the new clause widens *who* is checked rather than *what* is demanded.
/// 后半是合法对照：同一个中继形状、按提交 pin 之后是干净的——新条款放宽的是"查谁"，不是"要求什么"。
#[test]
fn an_inheriting_relay_job_must_pin_its_reusable_workflow() {
    let pinned = "11d5960a326750d5838078e36cf38b85af677262";
    let relay = |reference: &str| {
        format!(
            "on:\n  push:\n    tags: ['v*']\njobs:\n  relay:\n    uses: \
             octo/relay/.github/workflows/publish.yml@{reference}\n    secrets: inherit\n"
        )
    };
    let found = findings(&relay("v1"));
    assert_eq!(
        found.len(),
        1,
        "a relay job inherits the token, so its workflow reference must be pinned: {found:#?}"
    );
    assert!(
        found[0].contains("octo/relay/.github/workflows/publish.yml")
            && found[0].contains("40-character commit SHA"),
        "the report names the workflow and the rule: {found:#?}"
    );

    let clean = findings(&relay(pinned));
    assert!(
        clean.is_empty(),
        "a commit-pinned relay call needs no report: {clean:#?}"
    );
}

/// The rule is job-scoped: an action on a moving tag is fine in a job that cannot see the
/// secret, which is the shape `ci.yml` ships on purpose.
/// 规则限定在 job 内：看不到秘密的 job 里，挂在会移动的 tag 上的 action 没有问题——这正是
/// `ci.yml` 有意出厂的形状。
///
/// Red without the scope: a workflow-wide rule fires on `ci.yml` (no secret, tag-pinned
/// actions) and would force a change the maintainer decided against.
/// 没有作用域时就红：整文件范围的规则会在 `ci.yml`（无秘密、tag 形式 action）上误报，逼出维护者
/// 已经否决的改动。
#[test]
fn a_job_without_the_token_keeps_its_tag_pinned_actions() {
    let text = "jobs:\n  lint:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: \
                actions/checkout@v4\n  release:\n    runs-on: ubuntu-latest\n    steps:\n      - \
                uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262\n        env:\n          \
                CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}\n";
    let found = findings(text);
    assert!(
        found.is_empty(),
        "only the job that names the token is constrained: {found:#?}"
    );
}

/// A comment naming the token does not make a job a token holder.
/// 注释里提到 token，不会让一个 job 成为 token 持有者。
///
/// `release.yml`'s own prose names the token, and `ci.yml` says in a sentence that the release
/// workflow carries the secret; reading prose as a declaration would pin actions where the
/// maintainer chose tags.
/// `release.yml` 的散文本身点到该 token，而 `ci.yml` 用一句话说发布工作流携带该秘密；把散文读成
/// 声明会去 pin 维护者有意保持 tag 的那些 action。
#[test]
fn prose_about_the_token_does_not_constrain_a_job() {
    let text = "jobs:\n  lint:\n    runs-on: ubuntu-latest\n    steps:\n      # unlike the \
                release job, this one has no CARGO_REGISTRY_TOKEN\n      - uses: \
                actions/checkout@v4\n";
    let found = findings(text);
    assert!(
        found.is_empty(),
        "a comment is not a declaration: {found:#?}"
    );
}

/// The spellings that are not an action at all are left alone: a local action in this
/// checkout, and a container action naming an image.
/// 根本不是 action 的拼法原样放过：本检出里的本地 action，以及命名镜像的容器 action。
#[test]
fn a_local_or_container_reference_needs_no_commit() {
    let text = "jobs:\n  release:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: \
                ./.github/actions/local\n      - uses: docker://alpine:3\n        env:\n          \
                CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}\n";
    let found = findings(text);
    assert!(
        found.is_empty(),
        "only remote `owner/repo@ref` references are pinned: {found:#?}"
    );
}

/// A trailing YAML comment after the pin is not part of the reference.
/// pin 之后的 YAML 行尾注释不属于引用本身。
///
/// The shipped `release.yml` writes `@<sha> # v4.4.0`, so a rule that read the comment as
/// part of the ref would fire on the very file it protects.
/// 出厂的 `release.yml` 写的是 `@<sha> # v4.4.0`，因此把注释读进 ref 的规则会在它保护的那个文件上
/// 误报。
#[test]
fn the_comment_after_a_pin_is_not_the_reference() {
    let text = "jobs:\n  release:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: \
                actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.4.0\n        env:\n          \
                CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}\n";
    let found = findings(text);
    assert!(
        found.is_empty(),
        "the SHA is the reference; the comment names its origin: {found:#?}"
    );
}

/// The two workflows this repository ships stay clean, which is the half that proves the
/// gate is not over-firing.
/// 本仓库出厂的这两个工作流保持干净，这一半证明门禁没有过火。
#[test]
fn the_shipped_workflows_hold_the_rule() {
    let found = crate::release_workflow::workflow_findings(&workspace_root());
    assert!(found.is_empty(), "a shipped workflow regressed: {found:#?}");
}

/// Breaking the shipped `release.yml` — the one file the rule exists for — turns it red.
/// 改坏出厂的 `release.yml`——这条规则唯一为之存在的文件——会让它变红。
///
/// This is the counterexample the acceptance asks for: the gate is not a rule that only
/// fires on made-up YAML. Swapping the checkout pin for `@v4` is exactly what the audit
/// measured as green before the fix.
/// 这就是验收要求的否定式反例：门禁不是只对造出来的 YAML 生效的规则。把 checkout 的 pin 换成
/// `@v4` 正是审计在修前实测为绿的那件事。
#[test]
fn the_real_workflow_turns_red_when_its_pin_becomes_a_tag() {
    let path = workspace_root().join(".github/workflows/release.yml");
    let shipped = std::fs::read_to_string(&path).expect("the shipped release workflow");
    assert!(
        findings(&shipped).is_empty(),
        "the shipped file is the thing the rule protects"
    );
    let unpinned = shipped.replace(
        "actions/checkout@11d5960a326750d5838078e36cf38b85af677262",
        "actions/checkout@v4",
    );
    assert_ne!(shipped, unpinned, "the pin is spelled as the test expects");
    let found = findings(&unpinned);
    assert_eq!(
        found.len(),
        1,
        "the mutated file must be reported exactly once: {found:#?}"
    );
    assert!(
        found[0].contains("actions/checkout") && found[0].contains("commit SHA"),
        "the report names the action and the rule: {found:#?}"
    );
}
