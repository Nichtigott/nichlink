//! Tests for the two bounds `nichlink.callgraph` needs: definitions and callers.
//! `nichlink.callgraph` 需要的两道上限的测试：定义数与调用者数。
//!
//! The measured failure these exist against: a common name (`new`) has 142
//! definitions in the NichUI corpus, and every call site of that name was listed for
//! each of them, which arrived as a 4.5 MB reply. An answer that an agent cannot read
//! is not an answer, so both bounds are pinned here.
//! 这些测试所针对的实测失败：常见名（`new`）在 NichUI 语料里有 142 个定义，而每个定义都列出该名字
//! 在树里的每一个调用点，最终以 4.5 MB 的回复抵达。代理读不下的答案不算答案，因此两道上限都钉在这里。

use std::path::PathBuf;

use serde_json::json;

use super::callgraph;

/// A throwaway source root: `definitions` files each declaring `fn new`, and one
/// file holding `callers` functions that all call it.
/// 一个一次性源码根：`definitions` 个文件各声明一个 `fn new`，外加一个文件，里面有 `callers`
/// 个函数都调用它。
fn tree(label: &str, definitions: usize, callers: usize) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "mcp-callgraph-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    for index in 0..definitions {
        let directory = root.join(format!("defs{index}"));
        std::fs::create_dir_all(&directory).expect("definition directory");
        std::fs::write(
            directory.join(format!("defs{index}.rs")),
            "pub fn new() {}\n",
        )
        .expect("definition");
    }
    let callers_directory = root.join("callers");
    std::fs::create_dir_all(&callers_directory).expect("caller directory");
    let mut body = String::new();
    for index in 0..callers {
        body.push_str(&format!("pub fn caller{index}() {{ new(); }}\n"));
    }
    std::fs::write(callers_directory.join("callers.rs"), body).expect("callers");
    root
}

/// A name with several definitions is named as ambiguous, the extra definitions
/// are withheld with their count, and the caller list is capped rather than
/// dumped.
/// 有多个定义的名字会被说成歧义，多出来的定义连同数量一起被扣下，调用者清单被截断而不是倾倒。
#[test]
fn a_common_name_is_bounded_and_its_ambiguity_is_named() {
    let root = tree("ambiguous", 3, 25);
    let reply =
        callgraph(&root, &json!({"function": "new", "limit": 1})).expect("the answer renders");
    assert!(reply.contains("matches 3"), "{reply}");
    assert!(reply.contains("note: 3 definitions match"), "{reply}");
    assert!(reply.contains("pass `path` to select one"), "{reply}");
    // One definition shown, two withheld, both stated.
    // 显示一个定义、扣下两个，两件事都写出来。
    assert!(reply.contains("… +2 more definitions"), "{reply}");
    // The caller list is capped: 25 callers, 20 shown.
    // 调用者清单有上限：25 个调用者，显示 20 个。
    assert!(reply.contains("callers (25):"), "{reply}");
    assert!(reply.contains("… +5 more"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// `path` removes the ambiguity, so the note must disappear — it is advice, not
/// boilerplate.
/// `path` 消除了歧义，因此那句提示必须消失——它是建议，不是套话。
#[test]
fn a_path_filter_removes_the_ambiguity_note() {
    let root = tree("filtered", 3, 2);
    let reply = callgraph(&root, &json!({"function": "new", "path": "defs1/defs1.rs"}))
        .expect("the answer renders");
    assert!(reply.contains("matches 1"), "{reply}");
    assert!(!reply.contains("definitions match"), "{reply}");
    assert!(reply.contains("defs1/defs1.rs:1 fn new"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}
