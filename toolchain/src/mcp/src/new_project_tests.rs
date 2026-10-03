//! Tests for the scaffold write: what a preview promises, and what an apply creates.
//! 脚手架写入的测试：预览承诺了什么，落盘创建了什么。
//!
//! The default these are aimed at is "preview, never write": the first test asserts that a
//! request without `apply` leaves the destination absent, and it does that **before** it
//! reads the reply, so the mutation that flips the default to a write fails on the
//! assertion that names the directory a preview must not create.
//! 这些测试瞄准的默认是"只预览、绝不写"：第一条测试断言不带 `apply` 的请求不会让目的地出现，而且它
//! 是在读回复**之前**做的，因此把默认翻成"写"的变异会失败在那条点名"预览不得创建的目录"的断言上。

use std::path::PathBuf;

use serde_json::json;

use super::new_project;

/// A throwaway root: a directory that exists, which is all the scaffold's own root
/// question needs (`cargo metadata` is not consulted, because no member's identity is).
/// 一个一次性根：一个存在的目录，而脚手架自己的根问题需要的就这些（不查 `cargo metadata`，因为不涉及
/// 任何成员的身份）。
struct Root {
    path: PathBuf,
}

impl Drop for Root {
    /// Remove the fixture tree once the test that built it is done.
    /// 构建它的测试结束后删除夹具树。
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// A fresh root under a label that keeps concurrent tests apart.
/// 一个把并发测试彼此分开的新根，带标签。
fn root(label: &str) -> Root {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "nichlink-mcp-new-project-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("fixture root");
    Root { path }
}

/// A throwaway virtual workspace root with one member, so the entrance resolves it the
/// way Cargo does.
/// 一个带单个成员的一次性虚拟工作区根，好让入口按 Cargo 的方式解析它。
fn workspace_root(label: &str) -> Root {
    let fixture = root(label);
    std::fs::create_dir_all(fixture.path.join("host/src")).expect("member directory");
    std::fs::write(
        fixture.path.join("Cargo.toml"),
        "[workspace]\nmembers = [\"host\"]\nresolver = \"2\"\n",
    )
    .expect("workspace manifest");
    std::fs::write(
        fixture.path.join("host/Cargo.toml"),
        "[package]\nname = \"mcp-new-project-host\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("member manifest");
    std::fs::write(fixture.path.join("host/src/lib.rs"), "// host entry\n").expect("member entry");
    fixture
}

/// One tool call, as the protocol would deliver it.
/// 一次工具调用，按协议交付的样子。
fn call(fixture: &Root, arguments: serde_json::Value) -> (String, bool) {
    let reply = crate::mcp::tools::tool_call(
        &fixture.path,
        json!(1),
        &json!({"name": "nichlink.new_project", "arguments": arguments}),
    );
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("the scaffold returned no text: {reply}"))
        .to_owned();
    let failed = reply["result"]["isError"].as_bool().unwrap_or(false);
    (text, failed)
}

/// A request without `apply` writes nothing and says every path and byte it would write.
/// 不带 `apply` 的请求什么都不写，并说出它会写的每个路径与字节。
///
/// This is the zero-write pin, and it asserts the absence first: the mutation it is aimed
/// at is "preview means write", and the failure has to name the directory that appeared.
/// 这是"零写入"钉子，而且它先断言"不存在"：它瞄准的变异是"预览即写入"，而失败必须点名那个出现的目录。
#[test]
fn a_preview_writes_nothing_and_names_every_path_it_would_write() {
    let fixture = root("preview");
    let target = fixture.path.join("app");
    let reply = new_project(
        &fixture.path,
        &json!({"directory": "app", "package": "probe-host", "kind": "binary"}),
    )
    .expect("the preview answers");

    assert!(
        !target.exists(),
        "a request without `apply` must not create {} — the default is a preview, not a write. \
         reply was:\n{reply}",
        target.display()
    );
    assert!(
        std::fs::read_dir(&fixture.path)
            .expect("the root reads")
            .next()
            .is_none(),
        "the preview left something in {}:\n{reply}",
        fixture.path.display()
    );
    for expected in [
        "preview: nichlink.new_project would create the binary project `probe-host`",
        &target.display().to_string(),
        "nothing was written",
        "+ Cargo.toml",
        "+ build.rs",
        "+ src/main.rs",
        "+ .vscode/nichlink-face.code-snippets",
        "+[workspace]",
    ] {
        assert!(
            reply.contains(expected),
            "`{expected}` is part of the preview's contract:\n{reply}"
        );
    }
    // The destination line is what a reader checks against the root, so the preview has to
    // say the root too.
    // 目的地那一行是读者拿去与根核对的东西，因此预览也必须说出根。
    assert!(
        reply.contains(&format!(
            "root {}: the destination is inside it",
            fixture.path.display()
        )),
        "{reply}"
    );
}

/// An apply creates every file the preview showed, with the executor's own bytes.
/// 落盘会创建预览展示的每个文件，字节来自执行器自己。
#[test]
fn an_apply_writes_every_file_the_preview_showed() {
    let fixture = root("apply");
    let target = fixture.path.join("app");
    let reply = new_project(
        &fixture.path,
        &json!({"directory": "app", "package": "probe-host", "kind": "library", "apply": true}),
    )
    .expect("the apply answers");

    assert!(
        reply.contains("applied: created the library project `probe-host`"),
        "{reply}"
    );
    assert!(reply.contains("wrote 4 file(s) under"), "{reply}");
    for relative in [
        "Cargo.toml",
        "build.rs",
        "src/lib.rs",
        ".vscode/nichlink-face.code-snippets",
    ] {
        assert!(
            target.join(relative).is_file(),
            "{relative} was reported but is not there:\n{reply}"
        );
    }
    let manifest = std::fs::read_to_string(target.join("Cargo.toml")).expect("manifest reads");
    assert!(manifest.contains("name = \"probe-host\""), "{manifest}");
    // The executor's own `[workspace]` table is preserved, which is what keeps the new
    // project out of the workspace root it was written under.
    // 执行器自己的 `[workspace]` 表被保留，正是它让新项目不属于写下它的那个工作区根。
    assert!(manifest.starts_with("[workspace]\n"), "{manifest}");
    assert!(
        reply.contains("not a\nmember of the root above") || reply.contains("not a member"),
        "{reply}"
    );
}

/// A destination that already exists needs the request to say `confirm`, and the refusal
/// comes before anything is written into it.
/// 已经存在的目的地需要请求自己说出 `confirm`，而拒绝发生在往其中写入任何东西之前。
#[test]
fn an_existing_destination_without_confirm_is_refused() {
    let fixture = root("confirm");
    let target = fixture.path.join("app");
    std::fs::create_dir_all(&target).expect("pre-made destination");

    let error = new_project(
        &fixture.path,
        &json!({"directory": "app", "package": "probe-host", "kind": "binary", "apply": true}),
    )
    .expect_err("an existing destination without `confirm` is refused");
    assert!(error.contains("confirm: true"), "{error}");
    assert!(error.contains("already exists"), "{error}");
    assert!(
        std::fs::read_dir(&target)
            .expect("the destination reads")
            .next()
            .is_none(),
        "the refusal wrote into {}:\n{error}",
        target.display()
    );

    // The same request with `confirm` writes, so the gate is the flag and not the shape of
    // the destination.
    // 同一个请求带上 `confirm` 就能写，因此这道闸门就是那个标志，而不是目的地本身的形状。
    let applied = new_project(
        &fixture.path,
        &json!({"directory": "app", "package": "probe-host", "kind": "binary", "apply": true,
                "confirm": true}),
    )
    .expect("a confirmed apply into an existing, empty destination answers");
    assert!(applied.contains("applied:"), "{applied}");
    assert!(target.join("Cargo.toml").is_file(), "{applied}");
}

/// A non-empty destination is refused, and nothing in it is touched.
/// 非空目的地被拒绝，其中的东西一点没被碰。
#[test]
fn a_non_empty_destination_is_refused() {
    let fixture = root("non-empty");
    let target = fixture.path.join("app");
    std::fs::create_dir_all(&target).expect("pre-made destination");
    std::fs::write(target.join("keep.txt"), "mine\n").expect("existing file");

    let error = new_project(
        &fixture.path,
        &json!({"directory": "app", "package": "probe-host", "kind": "binary", "apply": true,
                "confirm": true}),
    )
    .expect_err("a non-empty destination is refused");
    assert!(error.contains("is not empty"), "{error}");
    assert_eq!(
        std::fs::read_to_string(target.join("keep.txt")).expect("the file survives"),
        "mine\n"
    );
}

/// A destination outside the root this call runs in is refused, by name, for both the
/// relative-escape and the absolute spellings.
/// 落在这个调用所运行之根以外的目的地被点名拒绝——相对逃逸与绝对路径两种拼法都是。
#[test]
fn a_destination_outside_the_root_is_refused() {
    let fixture = root("outside");
    let sibling = fixture
        .path
        .parent()
        .expect("a parent")
        .join("mcp-new-project-escape");

    let escape = new_project(
        &fixture.path,
        &json!({"directory": "../mcp-new-project-escape", "package": "probe-host",
                "kind": "binary", "apply": true, "confirm": true}),
    )
    .expect_err("a destination that steps out with `..` is refused");
    assert!(escape.contains("REFUSED"), "{escape}");
    assert!(escape.contains("`..`"), "{escape}");
    assert!(
        escape.contains(&fixture.path.display().to_string()),
        "the refusal names the root it protects:\n{escape}"
    );

    let absolute = new_project(
        &fixture.path,
        &json!({"directory": sibling.display().to_string(), "package": "probe-host",
                "kind": "binary", "apply": true, "confirm": true}),
    )
    .expect_err("an absolute destination outside the root is refused");
    assert!(absolute.contains("REFUSED"), "{absolute}");
    assert!(absolute.contains("outside the root"), "{absolute}");
    assert!(
        absolute.contains(&sibling.display().to_string()),
        "the refusal names where it would have written:\n{absolute}"
    );
    assert!(
        !sibling.exists(),
        "a refused destination must not exist: {}",
        sibling.display()
    );
}

/// On a virtual workspace root the scaffold answers against *that* root: a relative
/// destination is its child, and one that leaves the workspace is refused there too.
/// 在虚拟工作区根上，脚手架以**那个**根作答：相对目的地是它的子目录，而离开工作区的目的地在那里同样
/// 被拒绝。
#[test]
fn a_virtual_workspace_root_answers_the_scaffold_inside_itself() {
    let fixture = workspace_root("virtual");
    let target = fixture.path.join("app");

    let (text, failed) = call(
        &fixture,
        json!({"directory": "app", "package": "probe-host", "kind": "binary"}),
    );
    assert!(!failed, "{text}");
    assert!(
        text.contains(&format!(
            "root {}: the destination is inside it",
            fixture.path.display()
        )),
        "{text}"
    );
    assert!(
        !target.exists(),
        "the workspace root answered a write it never made:\n{text}"
    );

    let (escaped, failed) = call(
        &fixture,
        json!({"directory": "../elsewhere", "package": "probe-host", "kind": "binary",
                "apply": true, "confirm": true}),
    );
    assert!(failed, "{escaped}");
    assert!(escaped.contains("REFUSED"), "{escaped}");
    assert!(
        escaped.contains(&fixture.path.display().to_string()),
        "{escaped}"
    );
}

/// A request missing one of the three required arguments is refused by name.
/// 缺少三个必填参数之一的请求会被点名拒绝。
#[test]
fn a_request_missing_an_argument_is_refused_by_name() {
    let fixture = root("arguments");
    for (arguments, key) in [
        (
            json!({"package": "probe-host", "kind": "binary"}),
            "directory",
        ),
        (json!({"directory": "app", "kind": "binary"}), "package"),
        (json!({"directory": "app", "package": "probe-host"}), "kind"),
    ] {
        let error = new_project(&fixture.path, &arguments)
            .expect_err("a request without every required argument is refused");
        assert!(error.contains(key), "{key} is not named in: {error}");
    }
    let wrong = new_project(
        &fixture.path,
        &json!({"directory": "app", "package": "probe-host", "kind": "tool"}),
    )
    .expect_err("an unknown kind is refused");
    assert!(wrong.contains("`binary` or `library`"), "{wrong}");
}

/// The preview states the stop condition: what "this project exists" means, and that nothing is
/// written yet.
/// 预览说出停止条件："这个项目存在"是什么意思，以及此刻还没写任何东西。
///
/// Measured (audit T-11): the seven scenarios' stop conditions were in the flow table but not in the
/// answers, so a reader had the rule in prose and nothing in the product to check it against. For
/// this scenario the two facts are `cargo check` compiling the skeleton and `registry` listing its
/// faces — both are calls the caller can make next, which is what makes it readable rather than felt.
/// 量到的（审计 T-11）：七个场景的停止条件写在流程表里、却不在答案里，于是读者手里只有散文里的规则、没有
/// 产物可对照。这个场景的两个事实是 `cargo check` 编译过骨架、`registry` 列出它的面——两者都是调用方能接着
/// 做的调用，这才叫"读得出来"而不是"感觉够了"。
#[test]
fn the_new_project_preview_states_its_stop_condition() {
    let fixture = root("stop-condition");
    let (answer, _) = call(
        &fixture,
        json!({"directory": "fresh", "kind": "library", "package": "fresh", "apply": false}),
    );
    assert!(
        answer.contains("stop   this project exists when"),
        "{answer}"
    );
    assert!(
        answer.contains("`cargo check` compiles it")
            && answer.contains("`registry` lists its faces"),
        "the stop condition names the two calls that show it: {answer}"
    );
    assert!(
        answer.contains("nothing is written until `apply: true`"),
        "and says the preview wrote nothing: {answer}"
    );
}

/// `faces` creates the project and its registration faces in **one** call, and the reply
/// reports the tree the project now derives — so "start a project with these two objects" is
/// one round trip instead of a scaffold plus one `apply add` per object. Round 13 measured the
/// old shape at five to six calls for exactly this task.
/// `faces` 用**一次**调用把项目与它的注册面一起建出来，并且回复报告项目现在推导出的那棵树——因此
/// "用这两个对象建个项目"是一个往返，而不是一次脚手架加每个对象一次 `apply add`。第十三轮量到旧形状
/// 在这道题上要五到六次调用。
#[test]
fn faces_creates_the_project_and_its_registration_faces_in_one_call() {
    let fixture = root("faces-one-call");
    let (report, failed) = call(
        &fixture,
        json!({
            "directory": "app",
            "package": "app",
            "kind": "library",
            "faces": [
                {"fields": {"module": "button", "kind": "Button"}},
                {"fields": {"module": "slider", "kind": "Slider"}},
            ],
            "apply": true,
        }),
    );
    assert!(!failed, "{report}");
    assert!(
        fixture.path.join("app/src/button/button.rs").is_file()
            && fixture.path.join("app/src/slider/slider.rs").is_file(),
        "both faces are on disk: {report}"
    );
    // The tree is reported here rather than left to a second `registry` call: that is the
    // consolidation, and this assertion is what keeps it from silently going away.
    // 这棵树在这里报出来，而不是留给第二次 `registry` 调用：那就是这次合并，而这枚断言就是不让它悄悄消失。
    assert!(
        report.contains("faces 2") && report.contains("root/button") && report.contains("root/slider"),
        "the reply names the faces the project derives: {report}"
    );
    assert!(
        report.contains("check"),
        "and the call that says whether it compiles: {report}"
    );
}

/// A preview with `faces` writes **nothing** — not the project and not a face — and it still
/// prints the faces it would derive. The default stays preview for the whole operation, because
/// a preview that quietly created the faces would be the one write a caller cannot undo.
/// 带 `faces` 的预览**什么都不写**——既不写项目也不写面——而它仍然打印它会推导出的面。整个操作的默认仍是
/// 预览，因为一份悄悄把面建出来的预览会成为调用方唯一撤不回的那次写入。
#[test]
fn a_faces_preview_writes_nothing_at_all() {
    let fixture = root("faces-preview");
    let (report, failed) = call(
        &fixture,
        json!({
            "directory": "app",
            "package": "app",
            "kind": "library",
            "faces": [{"fields": {"module": "button", "kind": "Button"}}],
        }),
    );
    // Read the destination first: the assertion has to fail on the write, not on the reply.
    // 先读目的地：断言必须失败在那次写入上，而不是失败在回复上。
    let entries: Vec<_> = std::fs::read_dir(&fixture.path)
        .expect("fixture root")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        entries.is_empty(),
        "a preview left {entries:?} behind under {}",
        fixture.path.display()
    );
    assert!(!failed, "{report}");
    assert!(
        report.contains("nothing was written") && report.contains("root/button"),
        "the preview promises the faces without writing them: {report}"
    );
}

/// An entry the executor refuses leaves the destination **untouched**: the project is built
/// beside it and moved in with one rename, so a refused face cannot leave a half-created
/// project — and it cannot leave a staging directory either.
/// 被执行器拒绝的条目会让目的地**原封不动**：项目在它旁边建好、用一次 rename 移进去，因此一个被拒的面
/// 既不会留下半个项目，也不会留下暂存目录。
#[test]
fn a_refused_face_leaves_the_destination_and_no_staging_behind() {
    let fixture = root("faces-refused");
    let (report, failed) = call(
        &fixture,
        json!({
            "directory": "app",
            "package": "app",
            "kind": "library",
            "faces": [
                {"fields": {"module": "button", "kind": "Button"}},
                {"parent": "nowhere", "fields": {"module": "slider", "kind": "Slider"}},
            ],
            "apply": true,
        }),
    );
    assert!(failed, "a face whose parent does not exist is refused: {report}");
    let entries: Vec<_> = std::fs::read_dir(&fixture.path)
        .expect("fixture root")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        entries.is_empty(),
        "the refused batch left {entries:?} under {}",
        fixture.path.display()
    );
    assert!(
        report.contains("nothing was written"),
        "and the refusal says so: {report}"
    );

    // The shape checks run before anything is built, so a key this entry does not take is
    // refused by name rather than ignored.
    // 形状检查发生在建任何东西之前，因此这条条目不接受的键会被点名拒绝，而不是被忽略。
    let (shape, failed) = call(
        &fixture,
        json!({
            "directory": "app2",
            "package": "app2",
            "kind": "library",
            "faces": [{"fields": {"module": "button"}, "bogus": 1}],
            "apply": true,
        }),
    );
    assert!(failed, "{shape}");
    assert!(
        shape.contains("faces[0].bogus"),
        "the refusal names the key and the position: {shape}"
    );
    assert!(
        !fixture.path.join("app2").exists(),
        "and nothing was created for it: {shape}"
    );
}

/// A destination that **already exists and is empty** still gets the project — including its
/// faces. It is the one destination shape this tool has always accepted (the caller made the
/// directory), and the staged-then-renamed write broke it: `rename` cannot replace a directory,
/// so the call failed with `EBUSY` after the whole project had been built beside it. The
/// round-13 v2 arm measured it from the caller's side (`--directory .`).
/// **已经存在且为空**的目的地照样拿到项目——包括它的面。这是本工具一直接受的唯一一种目的地形状（目录是
/// 调用方建的），而"旁边建好再 rename"的写法把它弄坏了：`rename` 无法替换目录，于是整个项目在旁边建好
/// 之后调用以 `EBUSY` 失败。第十三轮 v2 的臂从调用方那一侧量到了它（`--directory .`）。
#[test]
fn an_existing_empty_destination_receives_the_project_and_its_faces() {
    let fixture = root("faces-existing-empty");
    let destination = fixture.path.join("app");
    std::fs::create_dir_all(&destination).expect("the caller's empty directory");
    let (report, failed) = call(
        &fixture,
        json!({
            "directory": "app",
            "package": "app",
            "kind": "library",
            "faces": [{"fields": {"module": "button", "kind": "Button"}}],
            "apply": true,
            "confirm": true,
        }),
    );
    assert!(!failed, "{report}");
    assert!(
        destination.join("src/button/button.rs").is_file() && destination.join("build.rs").is_file(),
        "the project landed in the directory the caller made: {report}"
    );
    // And nothing was left beside it: the staging directory is gone with the move.
    // 而它旁边什么都没留下：暂存目录随这次移动消失。
    let siblings: Vec<_> = std::fs::read_dir(&fixture.path)
        .expect("fixture root")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        siblings,
        vec!["app".to_owned()],
        "only the project remains: {siblings:?}"
    );
}
