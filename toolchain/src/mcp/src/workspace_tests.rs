//! Tests for the workspace entrance: a virtual manifest names no package, so its tree
//! tools enumerate the members and answer grouped by package.
//! 工作区入口的测试：虚拟清单不命名任何包，因此它的树级工具枚举成员并按包分组作答。
//!
//! Two nails live here, and both are about an answer rather than a value. The first is
//! that a tree tool called on a virtual root names **every** member with its status and
//! really carries the host member's faces — a member dropped from the census, or a
//! `no faces` member silently skipped, turns it red and says which member is missing.
//! The second is that a degradation is stated in the body: a member whose tree cannot be
//! derived carries `tree unavailable (reason)` instead of contributing nothing, and a
//! root whose manifest Cargo cannot resolve says why rather than erroring out.
//! 这里有两枚钉子，两枚都是关于"一份答案"而不是一个取值。第一枚：在虚拟根上调用树级工具时，答案点名
//! **每一个**成员及其状态，并且真的带着宿主成员的面——把某个成员从普查里丢掉、或把 `no faces` 成员
//! 静默跳过，都会让它变红并说出缺的是哪个成员。第二枚：降级写在正文里——推导不出树的成员带着
//! `tree unavailable (原因)`，而不是什么都不贡献；Cargo 解析不了的根的清单则说出原因，而不是整条报错。

use std::path::{Path, PathBuf};

use nichlink_kernel::NodeId;
use serde_json::json;

use super::{Scope, scope};

/// One fixture workspace: the root, and the three member package names.
/// 一个夹具工作区：根，以及三个成员的包名。
struct Workspace {
    root: PathBuf,
    /// A host member: it declares one face and published nothing, so its tree was
    /// derived now and is reported as `not built`.
    /// 宿主成员：声明一个面且什么都没发布，因此它的树是现推的，报成 `not built`。
    host: String,
    /// A framework member: a library with no registration face at all.
    /// 框架成员：一个完全没有注册面的库。
    framework: String,
    /// A binary-only member with no `src/`: its tree cannot be derived.
    /// 一个没有 `src/`、只有二进制的成员：它的树推导不出来。
    tool: String,
}

impl Workspace {
    /// The census line every merged tree answer carries.
    /// 每份合并树级答案都带的那条普查行。
    ///
    /// `queried` is the face axis this census has always counted; `published` and
    /// `not built` are the evidence axis, and this fixture publishes nothing, so
    /// the one member with faces is `not built`.
    /// `queried` 是这个普查一直在数的面那一轴；`published` 与 `not built` 是证据轴，而本夹具什么
    /// 都没发布，因此那个有面的成员是 `not built`。
    fn census(&self) -> String {
        "members 3  queried 1  no faces 1  unresolvable 1  published 0  not built 1".to_owned()
    }
}

impl Drop for Workspace {
    /// Remove the fixture tree once the test that built it is done.
    /// 构建它的测试结束后删除夹具树。
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A throwaway virtual workspace whose three members cover the three statuses.
/// 一个一次性虚拟工作区，它的三个成员覆盖三种状态。
fn workspace(label: &str) -> Workspace {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-mcp-workspace-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    let name = |member: &str| format!("mcp-ws-{label}-{member}");
    let manifest = |member: &str| {
        format!(
            "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            name(member)
        )
    };
    write(
        &root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"host\", \"framework\", \"tool\"]\nresolver = \"2\"\n",
    );
    write(&root.join("host/Cargo.toml"), &manifest("host"));
    write(&root.join("host/src/lib.rs"), "// host entry\n");
    write(
        &root.join("host/src/button/button.rs"),
        "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: \
         crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    write(&root.join("framework/Cargo.toml"), &manifest("framework"));
    write(&root.join("framework/src/lib.rs"), "// framework entry\n");
    // A package whose only target is a binary at the package root: Cargo resolves it as a
    // member, and the face derivation has no `src/` to read — the reachable
    // `unresolvable` member.
    // 唯一目标是包根下一个二进制的包：Cargo 把它解析为成员，而面的推导没有 `src/` 可读——这正是
    // 可达的那个 `unresolvable` 成员。
    write(
        &root.join("tool/Cargo.toml"),
        &format!(
            "{}autobins = false\n\n[[bin]]\nname = \"{}\"\npath = \"main.rs\"\n",
            manifest("tool"),
            name("tool")
        ),
    );
    write(&root.join("tool/main.rs"), "fn main() {}\n");
    Workspace {
        root,
        host: name("host"),
        framework: name("framework"),
        tool: name("tool"),
    }
}

/// Write one fixture file, creating its directory.
/// 写一个夹具文件并建好它的目录。
fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directories");
    std::fs::write(path, text).expect("fixture file");
}

/// The first nail: `nichlink.registry` on a virtual root names every member with its
/// status, and the host member's faces are really in the answer, under the namespace its
/// own `CARGO_PKG_NAME` compiled with.
/// 第一枚钉子：虚拟根上的 `nichlink.registry` 点名每个成员及其状态，而宿主成员的面真的在答案里，
/// 就在它自己 `CARGO_PKG_NAME` 编译所用的命名空间之下。
/// A member root is told which workspace it belongs to, so an answer can say who is invisible:
/// the scenario round measured two functions reported as orphans at a member root whose callers
/// simply live in the other member.
/// 成员根会被告知它属于哪个工作区，于是答案能说清谁不可见：情景轮量到两个函数的调用者只是住在另一个
/// 成员里，却在成员根上被报成孤儿。
#[test]
fn a_member_root_knows_the_workspace_above_it() {
    let base = std::env::temp_dir().join(format!("nichlink-member-{}", std::process::id()));
    let member = base.join("core");
    std::fs::create_dir_all(member.join("src")).expect("the fixture directory");
    std::fs::write(
        base.join("Cargo.toml"),
        "[workspace]\nmembers = [\"core\", \"report\"]\n",
    )
    .expect("the manifest");
    assert_eq!(
        crate::mcp::workspace::enclosing_workspace(&member),
        Some(base.clone()),
        "the member names its workspace"
    );
    assert_eq!(
        crate::mcp::workspace::enclosing_workspace(&base),
        None,
        "the workspace root itself has nothing above it"
    );
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn the_registry_tool_answers_a_virtual_root_with_every_member() {
    let fixture = workspace("registry");
    let reply = crate::mcp::registry::registry(&fixture.root).expect("a workspace root answers");
    assert!(reply.contains("workspace "), "{reply}");
    assert!(
        reply.contains(&fixture.census()),
        "the census must count every member with its status ({}): {reply}",
        fixture.census()
    );
    for member in [&fixture.host, &fixture.framework, &fixture.tool] {
        assert!(
            reply.contains(member.as_str()),
            "every member must appear in the answer, `{member}` did not: {reply}"
        );
    }
    assert!(
        reply.contains(&format!("namespace {}\n", fixture.host)),
        "the host member `{}` must be answered under its own namespace: {reply}",
        fixture.host
    );
    assert!(reply.contains("root/button"), "{reply}");
    assert!(
        reply.contains("tree derived now (no published records at"),
        "a member with no records must say so rather than read as an unpublished tree: {reply}"
    );
    let expected = NodeId::from_namespaced_path(&fixture.host, "button/button.rs", "Button");
    assert!(
        reply.contains(&expected.to_string()),
        "the host member's face must carry its compiled identity {expected}: {reply}"
    );
    assert!(
        reply.contains(&format!("== {} (not built)\n", fixture.host)),
        "the host member `{}` has no section: {reply}",
        fixture.host
    );
    assert!(
        reply.contains(&format!("== {} (no faces)\n", fixture.framework)),
        "the framework member `{}` must have a `no faces` section, not a silence: {reply}",
        fixture.framework
    );
    assert!(
        reply.contains(&format!("== {} (unresolvable)\n", fixture.tool)),
        "the unresolvable member `{}` has no section: {reply}",
        fixture.tool
    );
    assert!(
        reply.contains("tree unavailable (no source tree at"),
        "the unresolvable member's reason must be in the body: {reply}"
    );
}

/// The second nail: `nichlink.search` on a virtual root states the census, groups each
/// member's face hits under it, and names the degradations instead of hiding them behind
/// a longer list of file hits.
/// 第二枚钉子：虚拟根上的 `nichlink.search` 说出普查，把每个成员的面命中分组在其下，并点名降级，
/// 而不是把它们藏在一份更长的文件命中之后。
#[test]
fn a_workspace_search_states_every_member_and_its_degradation() {
    let fixture = workspace("search");
    let reply = crate::mcp::search::search(&fixture.root, &json!({"query": "button"}))
        .expect("the search answers");
    assert!(
        reply.contains(&fixture.census()),
        "the census must count every member with its status ({}): {reply}",
        fixture.census()
    );
    for member in [&fixture.host, &fixture.framework, &fixture.tool] {
        assert!(
            reply.contains(member.as_str()),
            "`{member}` is missing from the answer: {reply}"
        );
    }
    assert!(reply.contains("face  root/button"), "{reply}");
    assert!(
        reply.contains(&format!("member {} (not built)", fixture.host)),
        "the host member's hits are grouped under it: {reply}"
    );
    assert!(
        reply.contains("no registration face under src/"),
        "the framework member's status says why it has none: {reply}"
    );
    assert!(
        reply.contains(&format!(
            "member {} (unresolvable)  tree unavailable (",
            fixture.tool
        )),
        "the unresolvable member says so where the reader is looking: {reply}"
    );
    let nothing =
        crate::mcp::search::search(&fixture.root, &json!({"query": "nothing-matches-this"}))
            .expect("the search answers");
    assert!(
        nothing.contains(&fixture.census()) && nothing.contains("no matches"),
        "a workspace query with no hits still states the tree it read, and says nothing matched: \
         {nothing}"
    );
}

/// The other two tree tools answer the same virtual root, and their bodies say the same
/// two things: a status per member, and a reason for the member that has no tree.
/// 另外两个树级工具对同一个虚拟根作答，而它们的正文说同样两件事：逐成员一个状态，以及没有树的那个
/// 成员的原因。
#[test]
fn grafts_and_diff_answer_a_virtual_root_with_the_same_census() {
    let fixture = workspace("tools");
    let report = crate::mcp::grafts::grafts(&fixture.root, &json!({})).expect("grafts answers");
    assert!(
        report.contains(&fixture.census()),
        "grafts must carry the same census ({}): {report}",
        fixture.census()
    );
    assert!(
        report.contains(&format!("namespace {}\n", fixture.host)),
        "grafts must answer the host member `{}` under its own namespace: {report}",
        fixture.host
    );
    assert!(
        report.contains("tree unavailable (no source tree at"),
        "grafts must explain the unresolvable member `{}`: {report}",
        fixture.tool
    );
    let delta = crate::mcp::diff::diff(&fixture.root, &json!({"records": true}))
        .expect("the records diff answers");
    assert!(
        delta.contains(&fixture.census()),
        "diff must carry the same census ({}): {delta}",
        fixture.census()
    );
    assert!(
        delta.contains(&format!("== {} (no faces)\n", fixture.framework)),
        "diff must give the framework member `{}` a section, not a silence: {delta}",
        fixture.framework
    );
    assert!(
        delta.contains("tree unavailable (no source tree at"),
        "{delta}"
    );
}

/// A member root is still one package: pointing `root` at a member keeps the
/// single-package answer byte for byte, with no census above it.
/// 成员根本来就还是一个包：把 `root` 指向成员时，单包答案逐字节不变，上面没有普查。
#[test]
fn a_member_root_still_answers_as_one_package() {
    let fixture = workspace("member");
    let reply = crate::mcp::registry::registry(&fixture.root.join("host"))
        .expect("the member answers about itself");
    assert!(
        reply.starts_with(&format!("namespace {}\n", fixture.host)),
        "{reply}"
    );
    assert!(!reply.contains("workspace "), "{reply}");
    assert!(!reply.contains("members "), "{reply}");
    let resolved = match scope(&fixture.root.join("host")).expect("the member resolves") {
        Scope::Package(namespace) => namespace,
        _ => panic!("a member directory is a package"),
    };
    assert_eq!(resolved, fixture.host);
}

/// A root whose manifest Cargo cannot resolve is a real root whose tree cannot be
/// named — the nested fixture package's shape — so the answer carries Cargo's reason and
/// the way out instead of a bare error, and `search` still answers the source half.
/// Cargo 解析不了清单的根是一个真实存在、却无法被命名的根——正是嵌套夹具包的形状——因此答案带上
/// Cargo 的原因与出路，而不是一个光秃秃的错误；而 `search` 仍然回答源码那一半。
#[test]
fn a_root_cargo_cannot_resolve_says_why_in_the_body() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    // The dangling dependency points *outside* the package, and the inner `[workspace]`
    // table is what makes Cargo load it eagerly — the exact shape the nested fixture
    // package has (its own `[workspace]` plus path dependencies that no longer exist).
    // A missing path inside the package, with no `[workspace]`, is silently deferred.
    // 悬空的依赖指向包**之外**，而内部的 `[workspace]` 表正是让 Cargo 急切加载它的东西——这正是嵌套
    // 夹具包的形状（它自己的 `[workspace]` 加上已不存在的路径依赖）。包内缺失的路径、且没有
    // `[workspace]` 时，会被静默推迟。
    let base = std::env::temp_dir().join(format!(
        "nichlink-mcp-unresolvable-{}-{sequence}",
        std::process::id()
    ));
    let root = base.join("pkg");
    let _ = std::fs::remove_dir_all(&base);
    write(
        &root.join("Cargo.toml"),
        "[workspace]\n\n[package]\nname = \"mcp-unresolvable\"\nversion = \"0.1.0\"\n\
         edition = \"2021\"\n\n[dependencies]\ngone = { path = \"../gone\" }\n",
    );
    write(&root.join("src/lib.rs"), "// host entry\n");
    write(&root.join("src/widget.rs"), "pub fn widget_spin() {}\n");

    let reply = crate::mcp::registry::registry(&root).expect("a reason is an answer");
    assert!(reply.starts_with("unresolvable "), "{reply}");
    assert!(
        reply.contains("failed to load manifest for dependency"),
        "the reason Cargo gave must be the reason reported: {reply}"
    );
    assert!(reply.contains("NICH_LINK_NAMESPACE"), "{reply}");

    let search = crate::mcp::search::search(&root, &json!({"query": "widget"}))
        .expect("the source half still answers");
    assert!(
        search.contains("tree  unavailable ("),
        "the tree half is named as unavailable, not silently empty: {search}"
    );
    assert!(
        search.contains("failed to load manifest for dependency"),
        "{search}"
    );
    assert!(search.contains("file  src/widget.rs"), "{search}");
    let _ = std::fs::remove_dir_all(&base);
}
