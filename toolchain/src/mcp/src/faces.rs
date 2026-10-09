//! Which feature faces a tree compiles, and which one the default face is.
//! 一棵树编译哪些特性面，以及默认面是哪一个。
//!
//! Measured need: a defect in a file compiled only under a non-default feature cannot fail on the
//! default face, and the evaluation had exactly that round — `studio` is not in
//! `xirang-toolchain`'s `default`, so `cargo test --workspace` was green while the red was one
//! feature away. Neither this bridge nor the control tool could say which face a red run would
//! appear on, and both agents had to widen the feature set by hand.
//! 量出来的需求：只在非默认特性下编译的文件里的缺陷，在默认面上不可能失败，而评测里就有这么一轮——
//! `studio` 不在 `xirang-toolchain` 的 `default` 里，于是 `cargo test --workspace` 全绿、红只差一个
//! 特性。桥与对照工具都答不了"红会出现在哪个面"，两个代理都只能自己把特性面加宽。
//!
//! This answers the actionable half from Cargo's own view of the tree — **which faces exist and
//! which one is the default** — and says plainly what it cannot answer: which face is *red* is a
//! fact about a test run, not about the tree, and this module never guesses it. The design, and the
//! separately-decided opt-in that would observe a run, are in `docs/design-mcp-test-faces.md`.
//! 这里答的是可行动的那一半，取自 Cargo 自己对这棵树的看法——**有哪些面、哪个是默认面**——并明说它
//! 答不了什么：哪个面**是红的**是关于一次测试运行的事实、不是关于这棵树的事实，本模块从不猜它。设计、
//! 以及那个"观测一次运行"的、需单独拍板的选择，见 `docs/design-mcp-test-faces.md`。

use std::path::Path;

use serde_json::Value;

/// The face lines for the tree under this root, and the note that makes them actionable.
/// 这个根下那棵树的特性面行，以及让它们可行动的那条备注。
///
/// A tree Cargo cannot describe answers one line naming the reason rather than nothing: an answer
/// that silently dropped the faces would read as "this tree has none".
/// Cargo 描述不了的树会回一行原因，而不是什么都不回：静默丢掉特性面的答案会被读成"这棵树没有面"。
pub(crate) fn faces_lines(root: &Path) -> Vec<String> {
    let manifest = root.join("Cargo.toml");
    let metadata = match crate::mcp::workspace::metadata_json(&manifest) {
        Ok(metadata) => metadata,
        Err(reason) => return vec![format!("faces  unavailable ({reason})")],
    };
    let mut lines = Vec::new();
    let mut any_non_default = false;
    for package in metadata["packages"].as_array().into_iter().flatten() {
        let Some(name) = package["name"].as_str() else {
            continue;
        };
        let features = package["features"].as_object();
        let default = features
            .and_then(|features| features.get("default"))
            .and_then(Value::as_array)
            .map(|values| names(values))
            .unwrap_or_default();
        let all = features
            .map(|features| {
                features
                    .keys()
                    .filter(|key| key.as_str() != "default")
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !all.is_empty() {
            any_non_default = true;
        }
        let required = package["targets"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|target| {
                let name = target["name"].as_str()?;
                let required = target["required-features"].as_array()?;
                (!required.is_empty()).then(|| format!("{name} -> {}", bracket(&names(required))))
            })
            .collect::<Vec<_>>();
        lines.push(format!(
            "faces  {name}: default={} all={}{}",
            bracket(&default),
            bracket(&all),
            if required.is_empty() {
                String::new()
            } else {
                format!("  required=[{}]", required.join(", "))
            }
        ));
    }
    if lines.is_empty() {
        return vec!["faces  none (Cargo reported no package for this manifest)".to_owned()];
    }
    if any_non_default {
        lines.push(
            "note   a file compiled only under a non-default feature cannot fail on the default \
             face: run `cargo test --all-features` (or `--features <name>`) before believing a \
             green default run"
                .to_owned(),
        );
    }
    lines
}

/// The string values of a JSON array, in the order Cargo wrote them.
/// JSON 数组里的字符串值，按 Cargo 写下的顺序。
fn names(values: &[Value]) -> Vec<String> {
    values
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

/// A comma-separated list in brackets, or `[]`.
/// 方括号里的逗号分隔清单，或 `[]`。
fn bracket(names: &[String]) -> String {
    format!("[{}]", names.join(", "))
}

#[cfg(test)]
#[path = "faces_tests.rs"]
mod faces_tests;
