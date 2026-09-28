//! The commit-pin rule for the jobs that can see the registry token.
//! 能看到 registry token 的 job 的提交 pin 规则。
//!
//! A retagged upstream action runs *before* the publish step and can rewrite what that step
//! executes, which is why `release.yml`'s release job pins its two actions by commit and
//! says so in a comment. Nothing checked it: swapping either SHA back to `@v4` / `@stable`
//! left `cargo test --workspace` green, so the promise was prose (audit `LGC-LG-26`). This
//! module reads the shipped YAML and makes it a gate.
//! 一个被重新打 tag 的上游 action 会先于发布步骤运行，并能改写那一步真正执行的东西，这正是
//! `release.yml` 的发布 job 把它的两个 action 按提交 pin、并在注释里写明的原因。此前没有任何
//! 东西检查它：把任一 SHA 换回 `@v4` / `@stable`，`cargo test --workspace` 依然全绿，因此那
//! 只是个散文承诺（审计 `LGC-LG-26`）。本模块读实际出厂的 YAML，把它变成门禁。
//!
//! Boundary: the rule is scoped to the *job* that names the token, because `ci.yml`
//! deliberately keeps tag-pinned actions — that workflow carries no registry secret, so
//! pinning there is tidiness rather than safety (`release.yml`'s comment records the
//! maintainer's choice). A workflow-wide rule would fire on `ci.yml` and force a change the
//! maintainer decided against.
//! 边界：规则限定在**点名了**该 token 的那个 job，因为 `ci.yml` 有意保持 tag 形式的 action——
//! 那个工作流不携带 registry 秘密，在那里 pin 属于整洁而非安全（`release.yml` 的注释记下了维护者
//! 的选择）。整文件范围的规则会在 `ci.yml` 上误报，逼出维护者已经否决的改动。

/// The secret that turns a job's actions into code that runs on the publish path.
/// 让一个 job 的 action 变成"运行在发布路径上的代码"的那个秘密。
const REGISTRY_TOKEN: &str = "CARGO_REGISTRY_TOKEN";

/// Every action a token-holding job does not pin by commit, as one report each.
/// token 持有者的 job 里每个没有被提交地址 pin 住的 action，各报一条。
///
/// The whole file is asked; [`crate::release_workflow::findings`] is what walks the
/// checkout's workflow directory, so a workflow added later is covered by the same call.
/// 问的是整份文件；遍历检出工作流目录的是 [`crate::release_workflow::findings`]，因此以后新增的
/// 工作流也由同一次调用覆盖。
pub fn findings(text: &str) -> Vec<String> {
    unpinned_actions_where_the_token_is(text)
}

/// The `jobs:` entries of a workflow as `(name, body)` pairs.
/// 工作流的 `jobs:` 条目，形如 `(名字, 正文)`。
///
/// Job scope is what makes the action-pin rule land on the right jobs: `ci.yml` carries no
/// secret and keeps tag-pinned actions, while `release.yml`'s single job holds
/// `CARGO_REGISTRY_TOKEN` and pins by commit. A workflow-wide rule would fire on the first,
/// and a rule that ignored jobs could not tell them apart.
/// job 作用域正是让 action-pin 规则落在正确 job 上的东西：`ci.yml` 不带秘密、保持 tag 形式的
/// action，而 `release.yml` 唯一的 job 持有 `CARGO_REGISTRY_TOKEN` 并按提交 pin。整文件范围的
/// 规则会在前者上误报，而忽略 job 的规则分不清两者。
fn jobs(text: &str) -> Vec<(String, String)> {
    let mut jobs: Vec<(String, String)> = Vec::new();
    let mut in_jobs = false;
    let mut job_indent: Option<usize> = None;
    let mut current: Option<(String, String)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        if trimmed == "jobs:" {
            in_jobs = true;
            continue;
        }
        if !in_jobs {
            continue;
        }
        // A top-level key after `jobs:` ends the mapping.
        // `jobs:` 之后的顶层键结束这个映射。
        if !trimmed.is_empty() && !trimmed.starts_with('#') && indent == 0 {
            if let Some(job) = current.take() {
                jobs.push(job);
            }
            in_jobs = false;
            continue;
        }
        // A job name is a mapping key at the shallowest indent inside `jobs:`. Keys deeper
        // than that (`runs-on:`, `steps:`, a step's `env:`) belong to the job.
        // job 名字是 `jobs:` 内部最浅一层的映射键；更深的键（`runs-on:`、`steps:`、步骤的
        // `env:`）属于该 job。
        let is_job_key = !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && !trimmed.starts_with('-')
            && trimmed.ends_with(':')
            && job_indent.is_none_or(|level| indent <= level);
        if is_job_key {
            job_indent = Some(indent);
            if let Some(job) = current.take() {
                jobs.push(job);
            }
            current = Some((trimmed.trim_end_matches(':').to_owned(), String::new()));
            continue;
        }
        if let Some((_, body)) = current.as_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some(job) = current.take() {
        jobs.push(job);
    }
    jobs
}

/// Whether a `uses:` reference is a full commit address.
/// 一条 `uses:` 引用是否是一个完整的提交地址。
fn is_commit_sha(reference: &str) -> bool {
    reference.len() == 40 && reference.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Whether a job body line hands the job every secret there is.
/// job 正文里的一行是否把每个秘密都交给该 job。
///
/// `secrets: inherit` is how a reusable-workflow call takes the token without naming it, so a
/// job whose body carries that line holds the token exactly as much as one that spells the
/// name out — and the name search alone left the whole relay shape invisible, a bypass
/// reachable by a future edit (`N-3`, measured in the B4-conventions independent
/// verification). Widening *who* is checked does not widen *what* is demanded: the same
/// reference pinned by commit is still clean.
/// `secrets: inherit` 是可复用工作流调用在不点名的情况下取走 token 的方式，因此正文里有这一行的
/// job 与把名字写出来的 job 一样持有 token——只搜名字会让整个中继形状隐形，而那是一条未来编辑可达
/// 的绕过路径（`N-3`，在 B4-conventions 的独立验证里实测）。放宽"查谁"不放宽"要求什么"：同一个
/// 引用按提交 pin 之后依然是干净的。
fn inherits_every_secret(line: &str) -> bool {
    // A YAML sequence entry writes the key after a `- `, and a trailing comment names where
    // the value came from rather than being part of it — the same two readings the `uses:`
    // scan below applies.
    // 序列条目把键写在 `- ` 之后，行尾注释说明取值的来处、不属于取值本身——与下面 `uses:` 扫描用的
    // 同样是这两条读法。
    let content = line.strip_prefix("- ").unwrap_or(line);
    let value = content.split(" #").next().unwrap_or(content).trim();
    value
        .strip_prefix("secrets:")
        .is_some_and(|secrets| secrets.trim() == "inherit")
}

/// Every action in a job that can see `CARGO_REGISTRY_TOKEN` but is not pinned by commit.
/// 每个能看到 `CARGO_REGISTRY_TOKEN` 的 job 里，没有被提交地址 pin 住的 action。
///
/// Only a *non-comment* line makes a job a token holder: `ci.yml` says in prose that the
/// release workflow carries the secret, and reading that sentence as "this job holds the
/// token" would demand commit pins where the maintainer deliberately chose tags. A job can
/// hold it in two ways — naming the secret, or inheriting every secret (`N-3`) — and both
/// are read here.
/// 只有一个 job 里**非注释**的一行才能让它成为 token 持有者：`ci.yml` 在散文里说发布工作流携带
/// 该秘密，而把那句话读成"本 job 持有 token"会要求 commit pin——恰恰是维护者有意选择 tag 的地方。
/// job 持有它的方式有两种——点名该秘密，或继承全部秘密（`N-3`）——两种都在这里读。
fn unpinned_actions_where_the_token_is(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (job, body) in jobs(text) {
        let holds_token = body
            .lines()
            .map(str::trim)
            .filter(|line| !line.starts_with('#'))
            .any(|line| line.contains(REGISTRY_TOKEN) || inherits_every_secret(line));
        if !holds_token {
            continue;
        }
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                continue;
            }
            // A YAML sequence entry writes the key after a `- `: `- uses: actions/checkout@v4`.
            // 序列条目把键写在 `- ` 之后：`- uses: actions/checkout@v4`。
            let content = trimmed.strip_prefix("- ").unwrap_or(trimmed);
            let Some(uses) = content.strip_prefix("uses:") else {
                continue;
            };
            // A trailing YAML comment (`…@<sha> # v4.4.0`) names where the pin came from;
            // it is not part of the reference.
            // 行尾的 YAML 注释（`…@<sha> # v4.4.0`）说明这个 pin 来自哪个 tag；它不属于引用本身。
            let value = uses.split(" #").next().unwrap_or(uses).trim();
            // A local action (`./…`) is this repository's own file, and a container action
            // (`docker://…`) names an image rather than a ref.
            // 本地 action（`./…`）是本仓自己的文件，容器 action（`docker://…`）命名的是镜像而不是
            // ref。
            if value.is_empty() || value.starts_with("./") || value.starts_with("docker://") {
                continue;
            }
            match value.rsplit_once('@') {
                Some((_, reference)) if is_commit_sha(reference) => {}
                Some((action, reference)) => found.push(format!(
                    "job `{job}` holds `{REGISTRY_TOKEN}`, so `{action}` must be pinned by a \
                     40-character commit SHA; it is `{reference}`, which a retag can move"
                )),
                None => found.push(format!(
                    "job `{job}` holds `{REGISTRY_TOKEN}`, so `{value}` must name a commit \
                     (`<owner>/<repo>@<40 hex characters>`)"
                )),
            }
        }
    }
    found
}

#[cfg(test)]
#[path = "release_action_pin_tests.rs"]
mod release_action_pin_tests;
