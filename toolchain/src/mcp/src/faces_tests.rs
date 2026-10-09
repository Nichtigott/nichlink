//! Pins for the feature faces: the list names the non-default features and the targets that
//! require them, and it says what it cannot answer.
//! 特性面的钉子：清单点名非默认特性与需要它们的 target，并说出它答不了什么。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::faces_lines;

/// A throwaway package with one extra feature and a target that requires it.
/// 一个一次性包：多一个特性，以及一个需要它的 target。
fn package(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-mcp-faces-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_fixture(
        root.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"{label}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
         [features]\nextra = []\n\n[[bin]]\nname = \"tool\"\npath = \"src/tool.rs\"\n\
         required-features = [\"extra\"]\n"
        ),
    );
    write_fixture(root.join("src/lib.rs"), "// host entry\n");
    write_fixture(root.join("src/tool.rs"), "fn main() {}\n");
    root
}

fn write_fixture(path: PathBuf, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("fixture dirs");
    std::fs::write(path, text).expect("fixture file");
}

/// The list names the non-default feature, the target that requires it, and the rule that makes
/// the default face blind to files compiled only under it.
/// 清单点名非默认特性、需要它的 target，以及让默认面对"只在它之下编译的文件"失明的那条规则。
#[test]
fn the_faces_name_the_non_default_feature_and_its_required_target() {
    let root = package("faces");
    let lines = faces_lines(&root);
    let joined = lines.join("\n");
    assert!(joined.contains("default=[]"), "{joined}");
    assert!(joined.contains("all=[extra]"), "{joined}");
    assert!(joined.contains("required=[tool -> [extra]]"), "{joined}");
    assert!(
        joined.contains("cannot fail on the default face"),
        "{joined}"
    );
    assert!(
        joined.contains("before believing a green default run"),
        "the note says what to do about it: {joined}"
    );
    assert!(
        !joined.contains("is red"),
        "it must not claim to know which face is red: {joined}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A root Cargo cannot describe says so, rather than reading as a tree with no faces.
/// Cargo 描述不了的根会说出来，而不是被读成一棵没有面的树。
#[test]
fn an_undescribable_root_says_why_instead_of_saying_nothing() {
    let root = std::env::temp_dir().join(format!("xirang-mcp-faces-none-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("empty root");
    let lines = faces_lines(&root);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].starts_with("faces  unavailable ("), "{lines:?}");
    let _ = std::fs::remove_dir_all(&root);
}
