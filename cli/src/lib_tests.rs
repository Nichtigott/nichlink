//! End-to-end tests for the CLI dispatch surface.
//! CLI 分发表面的端到端测试。
//!
//! Split out of `lib.rs` (see the split note there): these tests drive the
//! public `run`/`run_to` entry points and the private helpers through `super`,
//! so they pin dispatch behaviour without making the dispatch page itself
//! exceed the file budget.
//! 从 `lib.rs` 拆出（见那里的拆分说明）：这些测试经 `super` 驱动公开的
//! `run`/`run_to` 入口与私有辅助函数，因此钉住分发行为，又不让分发页本身超出
//! 文件预算。

use super::{package_name, run, run_to, split_build_args};
use nichlink::identity::NodeId;
use nichlink::plugin::graft_document::GraftPlanDocument;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

/// Drive the CLI against an in-memory sink so a test can read the exact
/// stdout document a machine would parse. `run` is the same dispatch with
/// process stdout; `run_to` exists only so the document is assertable.
/// 让 CLI 写到内存接收器，使测试能读到机器会解析的那份 stdout 文档。`run` 是
/// 同一个分发、写到进程 stdout；`run_to` 的存在只是为了让文档可被断言。
fn run_capture(args: &[&str]) -> (Result<(), String>, String) {
    let mut buffer = Vec::new();
    let result = run_to(args.iter().map(|value| (*value).to_owned()), &mut buffer);
    (result, String::from_utf8(buffer).expect("utf-8 output"))
}

/// A throwaway Cargo package with an entry and registration faces, so
/// `cargo metadata` answers a package name and the build sees a real tree.
/// 一次性的 Cargo 包，带入口与注册面，使 `cargo metadata` 能给出包名、构建能看到
/// 真实的树。
fn fixture_host(label: &str, entry: &str, faces: &[(&str, &str)]) -> PathBuf {
    let root = temporary_root(label);
    fs::create_dir_all(root.join("src")).expect("fixture src");
    fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{label}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    fs::write(root.join("src/lib.rs"), entry).expect("entry");
    for (relative, source) in faces {
        let path = root.join("src").join(relative);
        fs::create_dir_all(path.parent().expect("face parent")).expect("face directory");
        fs::write(&path, source).expect("face source");
    }
    root
}

/// The three-level tree the operator commands are tested against, matching
/// the logical paths the README's example uses. The macro names follow the
/// generated per-registry aliases (`control_object!`, `object_object!`),
/// because the build's own validation requires the parent-specific macro; a
/// `root_object!` child under a registry is reported as a mismatch and would
/// make the fixture an invalid host.
/// 操作命令测试所用的三层树，逻辑路径与 README 示例一致。宏名遵循生成的按注册机
/// 别名（`control_object!`、`object_object!`），因为构建自身的校验要求父级专用
/// 宏；注册机下用 `root_object!` 的子面会被报为不匹配，夹具就成了非法宿主。
fn control_tree() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "control/control.rs",
            "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
        ),
        (
            "control/object/object.rs",
            "crate::control_object! {\n    kind: Object,\n    parent: crate::control::NODE_ID,\n    needs_registry: true,\n}\n",
        ),
        (
            "control/object/button/button.rs",
            "crate::object_object! {\n    kind: Button,\n    parent: crate::control::object::NODE_ID,\n}\n",
        ),
    ]
}

const DECLARED_BUTTON: &str = "// host entry\nnichlink_run_method::static_graft_plan!(\n    FRAMEWORK,\n    cut \"root/control/object/button\" graft \"button_fast\",\n);\n";

/// `check --json` puts exactly one document on stdout, carries every
/// diagnostic the human frame would print, and still fails the command.
/// `check --json` 在 stdout 上只放一个文档，携带人类可读帧会打印的每条诊断，并且
/// 仍然让命令失败。
#[test]
fn check_json_emits_one_document_and_still_fails() {
    let root = fixture_host(
        "cli-check-json",
        "// host\n",
        &[
            (
                "one/one.rs",
                "crate::root_object! {\n    kind: One,\n    stable_name: \"dup\",\n}\n",
            ),
            (
                "two/two.rs",
                "crate::root_object! {\n    kind: Two,\n    stable_name: \"dup\",\n}\n",
            ),
        ],
    );
    let path = root.display().to_string();
    let (result, stdout) = run_capture(&["nichlink", "check", "--json", &path]);
    assert!(result.is_err(), "a duplicate stable_name must fail");
    assert_eq!(stdout.lines().count(), 1, "exactly one document: {stdout}");
    let document: Value = serde_json::from_str(stdout.trim()).expect("one JSON document");
    assert_eq!(document["schema"], "nichlink.build-diagnostics/1");
    assert!(document["count"].as_u64().expect("count") >= 1);
    assert!(
        document["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .any(|diagnostic| diagnostic["message"]
                .as_str()
                .unwrap_or_default()
                .contains("duplicate stable_name")),
        "{document}"
    );
    fs::remove_dir_all(root).expect("cleanup");
}

/// Without `--json` the human output is byte-identical to the historical
/// `nichlink check: ok (<package>)` line, and the public `run` entry point
/// accepts the command.
/// 不带 `--json` 时人类输出与历史的 `nichlink check: ok (<package>)` 行逐字节
/// 一致，且公开的 `run` 入口接受该命令。
#[test]
fn check_without_json_keeps_the_human_line() {
    let root = fixture_host("cli-check-human", "// host\n", &[]);
    let path = root.display().to_string();
    let (result, stdout) = run_capture(&["nichlink", "check", &path]);
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(stdout, "nichlink check: ok (cli-check-human)\n");
    assert!(
        run(["nichlink".to_owned(), "check".to_owned(), path]).is_ok(),
        "the public `run` entry point accepts check"
    );
    fs::remove_dir_all(root).expect("cleanup");
}

/// `explain` resolves a logical path and a node id to the same face, and
/// reports identity, build scope, and the declared cut that names it.
/// `explain` 把逻辑路径与节点 id 解析到同一个面，并报告身份、构建作用域与命名它
/// 的已声明切口。
#[test]
fn explain_json_reports_identity_scope_and_declared_grafts() {
    let root = fixture_host("cli-explain", DECLARED_BUTTON, &control_tree());
    let path = root.display().to_string();
    let (checked, check_stdout) = run_capture(&["nichlink", "check", &path]);
    assert!(checked.is_ok(), "{checked:?} {check_stdout}");
    let (result, stdout) = run_capture(&[
        "nichlink",
        "explain",
        "--json",
        "--path",
        &path,
        "root/control/object/button",
    ]);
    assert!(result.is_ok(), "{result:?} {stdout}");
    let document: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(document["resolved"], true);
    assert_eq!(document["node"]["kind"], "Button");
    assert_eq!(
        document["node"]["source"],
        "control/object/button/button.rs"
    );
    assert_eq!(document["node"]["registry_name"], "button");
    assert_eq!(document["node"]["parent"]["path"], "root/control/object");
    assert_eq!(document["scope"]["known"], true);
    assert_eq!(document["pruning"]["known"], true);
    assert_eq!(document["declared_grafts"]["count"], 1);
    assert_eq!(
        document["declared_grafts"]["cuts"][0]["graft"],
        "button_fast"
    );

    let id = document["node"]["id"].as_str().expect("node id").to_owned();
    let (result, stdout) = run_capture(&["nichlink", "explain", "--json", "--path", &path, &id]);
    assert!(result.is_ok(), "{result:?} {stdout}");
    let by_id: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(by_id["node"]["path"], "root/control/object/button");
    fs::remove_dir_all(root).expect("cleanup");
}

/// `explain` refuses to present build output that predates the sources.
/// `explain` 不会把早于当前源码的构建产物当作现状提供。
///
/// It used to serve the previous build's scope as `known: true`, so the same tree
/// answered differently depending on whether a `check` had happened to run in
/// between; output left behind by a *failed* check was served the same way. The
/// failing half is pinned in `build_method::scope_view` (the run publishes no
/// fingerprint); this pins the staleness half end to end, through the real
/// command.
/// 它过去会把上一次构建的作用域当作 `known: true` 提供，于是同一棵树会因期间是否恰好跑过
/// `check` 而给出不同答案；**失败**的 check 留下的产物也是这样被提供的。失败那一半钉在
/// `build_method::scope_view`（该次运行不发布指纹）；这里端到端钉住陈旧那一半，走真实命令。
#[test]
fn explain_reports_an_unknown_scope_when_the_build_output_is_stale() {
    let root = fixture_host("cli-explain-stale", DECLARED_BUTTON, &control_tree());
    let path = root.display().to_string();
    let (checked, check_stdout) = run_capture(&["nichlink", "check", &path]);
    assert!(checked.is_ok(), "{checked:?} {check_stdout}");

    let query = [
        "nichlink",
        "explain",
        "--json",
        "--path",
        &path,
        "root/control/object/button",
    ];
    let (result, stdout) = run_capture(&query);
    assert!(result.is_ok(), "{result:?} {stdout}");
    let fresh: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(fresh["scope"]["known"], true, "{fresh}");
    assert_eq!(fresh["pruning"]["known"], true, "{fresh}");

    // A content change that keeps the host valid: the pin is about the token, not
    // about a semantic edit.
    // 一次仍然让宿主合法的内容变化：这条钉子关乎那枚凭据，而不是语义改动。
    let button = root.join("src/control/object/button/button.rs");
    let text = fs::read_to_string(&button).expect("button source");
    fs::write(&button, format!("// edited\n{text}")).expect("edited button");

    let (result, stdout) = run_capture(&query);
    assert!(result.is_ok(), "{result:?} {stdout}");
    let stale: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(
        stale["scope"]["known"], false,
        "a previous build must not answer for these sources: {stale}"
    );
    assert_eq!(stale["pruning"]["known"], false, "{stale}");
    assert_eq!(stale["resolved"], true, "the node itself is resolved live");

    // Re-publishing restores the answer, so the command is not simply pessimistic.
    // 重新发布即可恢复答案，因此本命令并非一律悲观。
    let (checked, check_stdout) = run_capture(&["nichlink", "check", &path]);
    assert!(checked.is_ok(), "{checked:?} {check_stdout}");
    let (_, stdout) = run_capture(&query);
    let current: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(current["scope"]["known"], true, "{current}");
    fs::remove_dir_all(root).expect("cleanup");
}

/// An unresolvable query is reported with its reason instead of a bare
/// failure, so an operator can correct the spelling.
/// 无法解析的查询连原因一起报告，而不是只报失败，让操作者能改正拼写。
#[test]
fn explain_reports_why_a_node_cannot_be_resolved() {
    let root = fixture_host(
        "cli-explain-missing",
        "// host\n",
        &[("one/one.rs", "crate::root_object! {\n    kind: One,\n}\n")],
    );
    let path = root.display().to_string();
    let (result, stdout) = run_capture(&[
        "nichlink",
        "explain",
        "--json",
        "--path",
        &path,
        "root/nope",
    ]);
    assert!(result.is_err());
    let document: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(document["resolved"], false);
    assert!(
        document["reason"]
            .as_str()
            .unwrap_or_default()
            .contains("root/nope"),
        "{document}"
    );
    fs::remove_dir_all(root).expect("cleanup");
}

/// `grafts` lists a readable plan with its selector, target path, graft,
/// `full`, and declared state, and reports an unreadable one with its
/// reason. Read-only: no file under the host changes.
/// `grafts` 列出一条可读计划的 selector、目标路径、graft、`full` 与声明状态，并把
/// 不可读的计划连原因一起报出。只读：宿主下没有文件被改动。
#[test]
fn grafts_json_lists_plans_and_declared_state() {
    let root = fixture_host("cli-grafts", DECLARED_BUTTON, &control_tree());
    let plan_dir = root.join(".nichlink/external-grafts/button_fast");
    fs::create_dir_all(&plan_dir).expect("plan directory");
    let target =
        NodeId::from_namespaced_path("cli-grafts", "control/object/button/button.rs", "Button");
    let document =
        GraftPlanDocument::new(target, "root/control/object/button", "button_fast", false);
    fs::write(plan_dir.join("graft.plan"), document.render()).expect("plan file");
    let broken = root.join(".nichlink/external-grafts/broken_graft");
    fs::create_dir_all(&broken).expect("broken directory");
    fs::write(broken.join("graft.plan"), "version=9\n").expect("broken plan");

    let path = root.display().to_string();
    let (result, stdout) = run_capture(&["nichlink", "grafts", "--json", &path]);
    assert!(result.is_ok(), "{result:?} {stdout}");
    let report: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(report["schema"], "nichlink.grafts/1");
    let plans = report["plans"].as_array().expect("plans");
    assert_eq!(plans.len(), 2);
    let button = plans
        .iter()
        .find(|plan| plan["selector"] == "button_fast")
        .expect("button plan");
    assert_eq!(button["target_path"], "root/control/object/button");
    assert_eq!(button["graft"], "button_fast");
    assert_eq!(button["full"], false);
    assert_eq!(button["declared"], true);
    assert_eq!(button["declared_by"]["graft"], "button_fast");
    let broken_row = plans
        .iter()
        .find(|plan| plan["selector"] == "broken_graft")
        .expect("broken plan");
    assert!(broken_row["error"].as_str().is_some(), "{broken_row}");
    fs::remove_dir_all(root).expect("cleanup");
}

/// A plans directory that exists and cannot be read is a failure, not "no
/// plans": the command's one answer is the declaration state of each plan, and
/// an unreadable directory leaves it with nothing to answer. The document still
/// reaches stdout, so a reader sees whatever was readable.
/// 存在却读不了的计划目录是失败，而不是"没有计划"：本命令唯一的答案是每条计划的声明状态，
/// 而读不了的目录让它无话可答。文档仍会写到 stdout，因此读者能看到可读的部分。
#[test]
fn grafts_reports_an_unreadable_plans_directory() {
    let root = fixture_host("cli-grafts-unreadable", DECLARED_BUTTON, &control_tree());
    // A file where the plans directory belongs, so the path exists and this is
    // not the ordinary "no plans yet" case.
    // 计划目录的位置放一个文件：路径存在，因此这不是普通的"还没有计划"。
    fs::create_dir_all(root.join(".nichlink")).expect("nichlink directory");
    fs::write(root.join(".nichlink/external-grafts"), "not a directory").expect("blocking file");

    let path = root.display().to_string();
    let (result, stdout) = run_capture(&["nichlink", "grafts", "--json", &path]);
    let error = result.expect_err("an unreadable plans directory must fail");
    assert!(error.contains("external-grafts"), "{error}");
    let report: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(report["plans"].as_array().map(Vec::len), Some(0));

    fs::remove_dir_all(root).expect("cleanup");
}

/// A host whose source tree cannot be read is a failure too: the declaration
/// column would otherwise be guessed from an empty face list.
/// 读不了源码树的宿主同样是失败：否则声明那一列会由一个空的面清单猜出来。
#[test]
fn grafts_reports_a_host_whose_sources_cannot_be_read() {
    let root = temporary_root("cli-grafts-no-src");
    // Cargo needs a target, so this package names one outside `src/`; the
    // question `grafts` asks is about `src/`, which is absent.
    // Cargo 需要一个 target，因此这个包把 target 指到 `src/` 之外；而 `grafts` 问的正是
    // `src/`，它不存在。
    fs::create_dir_all(root.join("library")).expect("library directory");
    fs::write(root.join("library/lib.rs"), "pub fn placeholder() {}\n").expect("library target");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"no-src\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"library/lib.rs\"\n",
    )
    .expect("manifest");

    let path = root.display().to_string();
    let (result, _stdout) = run_capture(&["nichlink", "grafts", &path]);
    let error = result.expect_err("a host without sources cannot be answered about");
    assert!(error.contains("source tree"), "{error}");

    fs::remove_dir_all(root).expect("cleanup");
}

/// `explain --overlay` renders the static overlay projection: the declared
/// slot carries its replacement, and the plan records are listed.
/// `explain --overlay` 渲染静态覆盖投影：已声明槽位带出它的替换件，并列出计划记录。
#[test]
fn explain_overlay_renders_the_static_projection() {
    let root = fixture_host("cli-overlay", DECLARED_BUTTON, &control_tree());
    let plan_dir = root.join(".nichlink/external-grafts/button_fast");
    fs::create_dir_all(&plan_dir).expect("plan directory");
    let target =
        NodeId::from_namespaced_path("cli-overlay", "control/object/button/button.rs", "Button");
    let document =
        GraftPlanDocument::new(target, "root/control/object/button", "button_fast", false);
    fs::write(plan_dir.join("graft.plan"), document.render()).expect("plan file");

    let path = root.display().to_string();
    let (checked, check_stdout) = run_capture(&["nichlink", "check", &path]);
    assert!(checked.is_ok(), "{checked:?} {check_stdout}");
    let (result, stdout) = run_capture(&[
        "nichlink",
        "explain",
        "--overlay",
        "--json",
        "--path",
        &path,
    ]);
    assert!(result.is_ok(), "{result:?} {stdout}");
    let report: Value = serde_json::from_str(stdout.trim()).expect("JSON report");
    assert_eq!(report["schema"], "nichlink.explain-overlay/1");
    assert_eq!(report["kind"], "static-projection");
    let button = report["slots"]
        .as_array()
        .expect("slots")
        .iter()
        .find(|slot| slot["path"] == "root/control/object/button")
        .expect("declared slot stays in the kept list");
    assert_eq!(button["replacement"]["graft"], "button_fast");
    // A parent face is not a declared slot but still ships, because the
    // selected child needs it; calling it "pruned" would misreport the tree.
    // 父面不是已声明槽位但仍然发布，因为被选中的子级需要它；说它"被剪掉"就是
    // 错误描述这棵树。
    assert!(
        report["slots"]
            .as_array()
            .expect("slots")
            .iter()
            .any(|slot| slot["path"] == "root/control/object" && slot["kept"] == true),
        "a needed parent stays in the kept list: {report}"
    );
    assert_eq!(
        report["plans"]
            .as_array()
            .expect("plans")
            .iter()
            .filter(|plan| plan["selector"] == "button_fast")
            .count(),
        1
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn help_and_missing_command_succeed() {
    assert!(run(["nichlink".to_owned(), "--help".to_owned()]).is_ok());
    assert!(run(["nichlink".to_owned()]).is_ok());
}

#[test]
fn unknown_command_is_an_error() {
    let result = run(["nichlink".to_owned(), "bogus".to_owned()]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown command 'bogus'"));
}

#[test]
fn new_requires_a_package_name() {
    let result = run(["nichlink".to_owned(), "new".to_owned()]);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("package name"));
}

#[test]
fn build_args_split_leading_path_from_cargo_options() {
    let owned = |items: &[&str]| {
        items
            .iter()
            .map(|item| item.to_string())
            .collect::<Vec<_>>()
    };
    let (path, rest) = split_build_args(&owned(&["app", "--release"]));
    assert_eq!(path.as_deref(), Some("app"));
    assert_eq!(rest, owned(&["--release"]));
    let (path, rest) = split_build_args(&owned(&["--release"]));
    assert_eq!(path, None);
    assert_eq!(rest, owned(&["--release"]));
    let (path, rest) = split_build_args(&[]);
    assert_eq!(path, None);
    assert!(rest.is_empty());
}

/// An unknown editor, or a path next to a config-dir editor, is refused
/// before anything is written.
/// 未知编辑器，或给"写配置目录"的编辑器附带路径，都在写盘之前拒绝。
#[test]
fn snippets_refuses_an_unknown_editor_and_a_stray_path() {
    let unknown = run([
        "nichlink".to_owned(),
        "snippets".to_owned(),
        "--editor".to_owned(),
        "emacs".to_owned(),
    ])
    .expect_err("unknown editor");
    assert!(unknown.contains("unknown editor 'emacs'"), "{unknown}");
    assert!(unknown.contains("vscode, nvim, blink"), "{unknown}");

    let stray = run([
        "nichlink".to_owned(),
        "snippets".to_owned(),
        "--editor".to_owned(),
        "nvim".to_owned(),
        "/tmp/somewhere".to_owned(),
    ])
    .expect_err("stray path");
    assert!(stray.contains("editor config"), "{stray}");
}

/// The command injects a parseable editor file covering the whole kernel
/// vocabulary, and refuses arguments it does not understand.
/// 该命令注入一个可解析、覆盖整个内核词表的编辑器文件，并拒绝看不懂的参数。
#[test]
fn snippets_injects_the_editor_file() {
    let root = std::env::temp_dir().join(format!(
        "nichlink-cli-snippets-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("temporary directory");
    run([
        "nichlink".to_owned(),
        "snippets".to_owned(),
        root.display().to_string(),
    ])
    .expect("snippets command");

    let path = root.join(nichlink_build_method::scaffold::SNIPPET_FILE);
    let text = std::fs::read_to_string(&path).expect("editor file");
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    let snippets = parsed.as_object().expect("an object of snippets");
    assert_eq!(
        snippets.len(),
        nichlink::registry_core::declaration::FACE_FIELD_ORDER.len()
    );
    let kind = snippets.get("kind: ").expect("the kind snippet");
    assert_eq!(kind["prefix"][0], "kind: ");
    assert_eq!(kind["body"][0], "kind: $1,");
    assert_eq!(kind["scope"], "rust");

    assert!(
        run([
            "nichlink".to_owned(),
            "snippets".to_owned(),
            "--bogus".to_owned()
        ])
        .is_err()
    );
    assert!(
        run([
            "nichlink".to_owned(),
            "snippets".to_owned(),
            "a".to_owned(),
            "b".to_owned()
        ])
        .is_err()
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}

/// A package name the old hand-rolled scan got wrong: a single-quoted name
/// with a later `[[bin]]` name, and a `name` key with no space around `=`.
/// 旧的手写扫描会读错的包名：单引号名字后面还跟着 `[[bin]]` 的 name；以及 `name`
/// 键等号两侧没有空格。
#[test]
fn package_name_comes_from_cargo_not_from_a_manifest_scan() {
    let root = temporary_root("package-name");
    let fixtures = [
        (
            "single-quoted",
            "[package]\nname = 'single-quoted'\nversion = \"0.1.0\"\n\n[[bin]]\nname = \"other-bin\"\npath = \"src/main.rs\"\n",
            "single-quoted",
        ),
        (
            "tight",
            "[package]\nname=\"tight\"\nversion = \"0.1.0\"\n\n[lib]\nname = \"different_lib\"\npath = \"src/lib.rs\"\n",
            "tight",
        ),
    ];
    for (directory, manifest, expected) in fixtures {
        let manifest_dir = root.join(directory);
        std::fs::create_dir_all(manifest_dir.join("src")).expect("source directory");
        std::fs::write(manifest_dir.join("Cargo.toml"), manifest).expect("manifest");
        std::fs::write(manifest_dir.join("src/main.rs"), "").expect("binary source");
        std::fs::write(manifest_dir.join("src/lib.rs"), "").expect("library source");
        assert_eq!(
            package_name(&manifest_dir).expect("cargo answers"),
            expected,
            "{directory}"
        );
    }
    std::fs::remove_dir_all(root).expect("cleanup");
}

/// A virtual manifest names no package, so the identity cannot be guessed:
/// the command has to say so instead of falling back to the directory name.
/// 虚拟 manifest 不命名任何包，身份因此无从猜测：命令必须说出来，而不是回退到
/// 目录名。
#[test]
fn a_manifest_without_a_package_is_an_error() {
    let root = temporary_root("virtual-manifest");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = []\nresolver = \"2\"\n",
    )
    .expect("manifest");
    let error = package_name(&root).expect_err("a virtual manifest names no package");
    assert!(error.contains("not a package"), "{error}");
    std::fs::remove_dir_all(root).expect("cleanup");
}

fn temporary_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "nichlink-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).expect("temporary directory");
    root
}
