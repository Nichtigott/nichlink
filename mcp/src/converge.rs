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

use nichlink_build_method::face_views;
use serde_json::Value;

use crate::apply::load_registry;
use crate::evidence::{build_evidence, pruning_line, scope_line};
use crate::nodes::resolve_node;
use crate::protocol::DEFAULT_LIMIT;
use crate::registry::namespace;

/// One `capability=>provider` requirement, and who answers it.
/// 一条 `capability=>provider` 需求，以及谁来满足它。
struct RequirementVerdict {
    capability: String,
    provider: String,
    answered_by: Option<String>,
}

/// Report the converged starting point for one face.
/// 报告一个面的收敛起点。
pub(crate) fn converge(root: &Path, arguments: &Value) -> Result<String, String> {
    let namespace = namespace(root)?;
    let faces = face_views(root, &namespace)?;
    let target = arguments
        .get("node")
        .and_then(Value::as_str)
        .ok_or_else(|| "nichlink.converge requires node".to_owned())?;
    let id = resolve_node(root, &namespace, target)?;
    let face = faces
        .iter()
        .find(|face| face.id == id)
        .ok_or_else(|| format!("no face in the derived tree has identity {id}"))?;
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize);
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
