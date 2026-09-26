//! Tests for the two bounds `nichlink.callgraph` needs: definitions and callers.
//! `nichlink.callgraph` 需要的两道上限的测试：定义数与调用者数。
//!
//! The measured failure these exist against: a common name (`new`) has 151
//! definitions in a real tree, and every call site of that name was listed for
//! each of them, which arrived as a 4.5 MB reply. An answer that an agent cannot
//! read is not an answer, so both bounds are pinned here.
//! 这些测试所针对的实测失败：常见名（`new`）在真实树里有 151 个定义，而每个定义都列出该名字在
//! 树里的每一个调用点，最终以 4.5 MB 的回复抵达。代理读不下的答案不算答案，因此两道上限都钉在这里。

use std::path::PathBuf;

use serde_json::json;

/// A throwaway package: `cargo metadata` can name it, so the tools that need a
/// namespace get one, and it holds no face so every evidence answer is the empty
/// one.
/// 一个一次性包：`cargo metadata` 能给它命名，因此需要命名空间的工具能得到一个；它不含任何面，
/// 所以每个证据答案都是空的那一种。
fn package(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-dispatch-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    root
}

/// The three evidence tools must be advertised with the argument that narrows
/// each one, because an agent can only call what `tools/list` names.
/// 三个证据工具必须被列出，并且带上各自缩小范围的那个参数，因为代理只能调用 `tools/list` 点名的东西。
#[test]
fn the_evidence_tools_are_advertised_with_their_narrowing_arguments() {
    let listed = super::tools();
    for (name, key) in [
        ("nichlink.explain", "node"),
        ("nichlink.diff", "limit"),
        ("nichlink.trace", "query"),
    ] {
        let tool = listed
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("{name} is not advertised"));
        assert!(
            tool["inputSchema"]["properties"].get(key).is_some(),
            "{name} does not advertise `{key}`: {tool}"
        );
    }
}

/// Every new name is wired to its implementation, not only to the catalog.
/// 每个新名字都接到了实现上，而不只是接进目录。
#[test]
fn the_evidence_tools_are_dispatched_to_their_implementations() {
    let root = package("dispatch");
    for (name, expected) in [
        ("nichlink.explain", "scope unknown"),
        ("nichlink.diff", "no build evidence"),
        ("nichlink.trace", "trace absent"),
    ] {
        let reply = super::tool_call(&root, json!(1), &json!({"name": name, "arguments": {}}));
        let text = reply["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("{name} returned no text: {reply}"));
        assert!(text.contains(expected), "{name}: {text}");
        assert_eq!(reply["result"]["isError"], false, "{name}: {reply}");
    }
    let _ = std::fs::remove_dir_all(&root);
}
