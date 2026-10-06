//! Pins for writing a partition: idempotent files, and a workspace config that is **merged**.
//! 写下一次拆分的钉子：幂等的文件，以及**被合并**的工作区配置。

use std::fs;
use std::path::{Path, PathBuf};

use super::write_partition;
use crate::build_time::crate_plan::{PlannedCrate, PlannedMount};

/// A throwaway workspace root.
/// 一个一次性工作区根。
fn workspace(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-write-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("workspace root");
    root
}

/// One planned crate, with the bytes the planner would have produced.
/// 一个已规划的 crate，带着规划器会产出的那些字节。
fn planned(root: &Path) -> PlannedCrate {
    PlannedCrate {
        name: "widgets".to_owned(),
        package: "myapp-widgets".to_owned(),
        namespace: "myapp".to_owned(),
        directory: root.join("myapp-widgets"),
        subtrees: vec!["control::object".to_owned()],
        ancestors: Vec::new(),
        mounts: vec![PlannedMount {
            module_path: "control::object::button".to_owned(),
            source: "control/object/button/button.rs".to_owned(),
            spelling: "../../../../host/src/control/object/button/button.rs".to_owned(),
        }],
        remap: vec![("../../../../host/src/".to_owned(), String::new())],
        lib_rs: "pub const NICHLINK_NAMESPACE: &str = \"myapp\";\n".to_owned(),
        build_rs: "fn main() {}\n".to_owned(),
        cargo_toml: "[package]\nname = \"myapp-widgets\"\npublish = false\n".to_owned(),
        config_patch: String::new(),
    }
}

/// The three files land with the planned bytes, and a second pass changes nothing — a partition is
/// something you commit, so re-running it must not produce a diff.
/// 三份文件带着规划好的字节落地，而第二遍什么都不改——拆分是要提交的东西，因此重跑不能产生 diff。
#[test]
fn writing_is_idempotent() {
    let root = workspace("idempotent");
    let planned = vec![planned(&root)];
    let first = write_partition(&root, &planned).expect("the first pass writes");
    assert_eq!(
        first.files,
        vec![
            "myapp-widgets/Cargo.toml".to_owned(),
            "myapp-widgets/build.rs".to_owned(),
            "myapp-widgets/src/lib.rs".to_owned(),
        ],
        "all three files, named relative to the workspace root"
    );
    assert!(first.config_changed, "the config did not exist yet");
    let lib = root.join("myapp-widgets/src/lib.rs");
    assert_eq!(
        fs::read_to_string(&lib).expect("written"),
        "pub const NICHLINK_NAMESPACE: &str = \"myapp\";\n"
    );

    let before = fs::read_to_string(root.join(".cargo/config.toml")).expect("config");
    let second = write_partition(&root, &planned).expect("the second pass writes");
    assert!(
        !second.config_changed,
        "every entry is already there, so the file is untouched"
    );
    assert_eq!(
        fs::read_to_string(root.join(".cargo/config.toml")).expect("config"),
        before,
        "no duplicate entries"
    );
    assert_eq!(
        fs::read_to_string(root.join("myapp-widgets/src/lib.rs")).expect("lib"),
        "pub const NICHLINK_NAMESPACE: &str = \"myapp\";\n"
    );
    let _ = fs::remove_dir_all(&root);
}

/// A config the user already maintains keeps **everything** else, and gains only the missing entries.
/// 用户已经在维护的 config 保留**其余一切**，只多出缺少的那些条目。
#[test]
fn an_existing_workspace_config_is_merged_not_replaced() {
    let root = workspace("merged");
    fs::create_dir_all(root.join(".cargo")).expect("config dir");
    fs::write(
        root.join(".cargo/config.toml"),
        "# the maintainer's own config\n[build]\nrustflags = [\"-C\", \"target-cpu=native\"]\n\n\
         [net]\ngit-fetch-with-cli = true\n",
    )
    .expect("config");
    let planned = vec![planned(&root)];
    let written = write_partition(&root, &planned).expect("the entries merge in");
    assert!(written.config_changed);
    let config = fs::read_to_string(root.join(".cargo/config.toml")).expect("config");
    for kept in [
        "# the maintainer's own config",
        "rustflags = [\"-C\", \"target-cpu=native\", \"--remap-path-prefix=../../../../host/src/=\"]",
        "[net]",
        "git-fetch-with-cli = true",
    ] {
        assert!(config.contains(kept), "kept `{kept}`: {config}");
    }
    let _ = fs::remove_dir_all(&root);
}

/// A `rustflags` this action cannot merge into is **refused by name**, with the lines to add by hand:
/// blindly rewriting a file the user maintains is how a tool eats somebody's configuration.
/// 本动作无法合并的 `rustflags` 会被**点名拒绝**，并给出该手写上去的那几行：盲目重写用户维护的文件，正是
/// 一件工具吃掉别人配置的方式。
#[test]
fn an_unmergeable_config_is_refused_with_the_lines_to_add() {
    let root = workspace("unmergeable");
    fs::create_dir_all(root.join(".cargo")).expect("config dir");
    fs::write(
        root.join(".cargo/config.toml"),
        "[build]\nrustflags = [\n    \"-C\",\n    \"target-cpu=native\",\n]\n",
    )
    .expect("config");
    let refused = write_partition(&root, &[planned(&root)]).expect_err("refused");
    assert!(
        refused.contains("--remap-path-prefix=../../../../host/src/=")
            && refused.contains("by hand"),
        "the refusal carries the lines to add: {refused}"
    );
    assert!(
        !root.join("myapp-widgets/src/lib.rs").exists(),
        "the files are not written when the config cannot be made sound"
    );
    let _ = fs::remove_dir_all(&root);
}
