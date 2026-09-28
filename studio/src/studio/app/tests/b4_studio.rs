//! Pins for the B4-studio batch: module self-descriptions that match their
//! implementation, the inspector's one row list, a failed editor launch that
//! survives the graft banner, the plugin entry gate reading lines, evidence
//! re-judged after a snapshot swap, and the search memo.
//! B4-studio 批次的钉子：与实现相符的模块自述、检视器唯一一份行清单、活过 graft 横幅的
//! 编辑器启动失败、按行判定的插件入口闸门、快照替换后重新裁决的证据，以及搜索备忘。
//!
//! covers `app/write_guard.rs`, `app/search_queries.rs`, `app/support.rs`,
//! `app/navigation.rs`, `app/lifecycle.rs`, `app/graft.rs`, `app/trace.rs` and
//! `app/mutations.rs` — one assertion group per finding (`STU-C-01`…`STU-C-04`,
//! `STU-S-02/03/05/08`, `LGC-LG-50`).
//! 覆盖上列生产模块——每个 finding 一组断言（`STU-C-01`…`STU-C-04`、
//! `STU-S-02/03/05/08`、`LGC-LG-50`）。

use super::super::{PluginState, plugin_field};
use super::*;

/// A throwaway project whose `.nichlink/plugins/` is the plugin write target.
/// 一个一次性工程，其 `.nichlink/plugins/` 是插件写入目标。
fn temp_plugins(label: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "nichlink-b4-studio-{label}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let plugins = root.join(".nichlink/plugins");
    std::fs::create_dir_all(&plugins).expect("plugin directory");
    select_project(root.clone(), root.join("Cargo.toml"), "plugin-app");
    (root, plugins)
}

/// Submit one plugin form and report what the status line said.
/// 提交一份插件表单，并交回状态行说了什么。
fn submit_plugin(source: &str, fields: [&str; 5], mode: &str) -> String {
    let mut plugin = PluginState::new();
    plugin.values[plugin_field::SOURCE] = source.to_owned();
    plugin.values[plugin_field::FRAMEWORK] = fields[0].to_owned();
    plugin.values[plugin_field::PACKAGE] = fields[1].to_owned();
    plugin.values[plugin_field::VERSION] = fields[2].to_owned();
    plugin.values[plugin_field::CRATE] = fields[3].to_owned();
    plugin.values[plugin_field::CHECKSUM] = fields[4].to_owned();
    plugin.values[plugin_field::MODE] = mode.to_owned();
    let mut app = App::load();
    app.submit_plugin(&plugin);
    app.event
}

/// The four module docs that drifted away from what their file does.
/// 四份与其文件实际行为脱节的模块文档。
///
/// Each pair is "the honest sentence must be there, the old one must not": the old
/// text is quoted verbatim in the B4 findings, so a green run here is the proof
/// that the drift is gone rather than a restatement of the new wording.
/// 每一对都是“诚实的那句必须在、旧的那句必须不在”：旧句在 B4 条目里是逐字引用的，因此这里跑绿
/// 就是漂移已消失的证明，而不是对新措辞的重述。
#[test]
fn module_docs_describe_what_the_module_does() {
    for (file, source, required, removed) in [
        (
            "write_guard.rs",
            include_str!("../write_guard.rs"),
            ["writes themselves are not here", "真正的写入不在这里"],
            [
                "every operation that creates, rewrites, moves, or deletes",
                "每一个在读者打开的项目里创建、重写、移动或删除东西的操作",
            ],
        ),
        (
            "search_queries.rs",
            include_str!("../search_queries.rs"),
            ["adjacent", "相邻"],
            ["Flat, deduplicated rows", "扁平、已去重结果行"],
        ),
        (
            "support.rs",
            include_str!("../support.rs"),
            ["the `cargo` probe", "`cargo` 探测"],
            [
                "Shared interaction geometry and editor helpers",
                "交互几何与编辑器辅助",
            ],
        ),
        (
            "navigation.rs",
            include_str!("../navigation.rs"),
            ["lands on the same toggle", "落回同一个开关"],
            [
                "BackTab walks the same two panes the other way",
                "BackTab 以相反方向走同样这两块面板",
            ],
        ),
    ] {
        for needle in required {
            assert!(source.contains(needle), "{file} no longer says `{needle}`");
        }
        for needle in removed {
            assert!(
                !source.contains(needle),
                "{file} still carries the drifted wording `{needle}`"
            );
        }
    }
}

/// The inspector count and the inspector drawing are the same list.
/// 检视器的行数与检视器的绘制是同一份清单。
///
/// The count used to be a literal `14` while the renderer drew 12 rows, so two
/// `↓` presses past the last row moved a selection nothing could show (audit
/// `STU-S-02`).
/// 行数过去是一个字面量 `14`，而渲染器只画 12 行，因此在最后一行之后再按两次 `↓` 会把选中项
/// 移到一个没有任何东西能显示的位置（审计 `STU-S-02`）。
#[test]
#[cfg(feature = "prototype-fixtures")]
fn the_inspector_count_is_the_row_list_it_draws() {
    let Some(app) = fixture_app() else {
        return;
    };
    assert!(
        app.selected_info().is_some(),
        "premise: the fixture registers a face and load selects it"
    );
    assert_eq!(app.detail_rows().len(), 12, "the rows a reader sees");
    assert_eq!(
        app.detail_field_count(),
        app.detail_rows().len(),
        "the count is the renderer's own list"
    );
}

/// A missing file makes the editor handoff fail visibly, and the failure is
/// returned so a success banner cannot be assigned over it.
/// 文件缺失会让编辑器交接可见地失败，而失败作为返回值交回，因此成功横幅无法覆盖它。
#[test]
fn a_failed_editor_launch_is_returned_not_only_shown() {
    let mut app = App::load();
    let missing = std::env::temp_dir().join("nichlink-b4-studio-missing-source.rs");
    let _ = std::fs::remove_file(&missing);
    let failure = app
        .open_editor_file(missing.clone(), 1)
        .expect_err("a path that is not a file cannot be opened");
    assert!(failure.contains("Editor failed:"), "{failure}");
    assert!(app.editor_request.is_none(), "no editor was launched");
    assert_eq!(
        app.event, failure,
        "the event line carries the same failure"
    );

    // The graft call site consumes that result instead of overwriting it (audit
    // `LGC-LG-50`); the banner is built first and the failure is prepended.
    // graft 的调用点消费这个返回值而不是覆盖它（审计 `LGC-LG-50`）：先构造横幅，再把失败
    // 放在它前面。
    let graft = include_str!("../graft.rs");
    assert!(
        graft.contains("open_editor_file(path.clone(), 1).err()"),
        "{graft}"
    );
    // The claim is unchanged — the failure is prepended to the banner — and since
    // `STU-S-18` the line is also routed to the alert channel in the same arm.
    // 断言没变——失败被放在横幅前面——而自 `STU-S-18` 起，同一条臂还把该行交给 alert 通道。
    assert!(
        graft.contains("(Some(failure), _) => self.alert(format!(\"{failure}\\n{banner}\")),"),
        "{graft}"
    );
}

/// The plugin entry gate asks whether the import *line* is there.
/// 插件入口闸门问的是那一条导入**行**在不在。
///
/// A commented-out copy of the line is not an import: reading it as one skipped the
/// append and then reported "Plugin selected" over an entry that never imported the
/// crate (audit `STU-S-08`).
/// 被注释掉的副本不是导入：把它读成导入会跳过追加，然后在一条从未导入该 crate 的入口上报
/// "Plugin selected"（审计 `STU-S-08`）。
#[test]
fn the_plugin_entry_gate_reads_lines_not_substrings() {
    let (root, plugins) = temp_plugins("entry-gate");
    let entry = plugins.join("user.rs");
    std::fs::write(&entry, "// use demo_plugin as _;\n").expect("seeded entry");

    let event = submit_plugin(
        "user",
        [
            "nichlink.test",
            "demo-plugin",
            "0.1.0",
            "demo_plugin",
            "sha256:00",
        ],
        "extension",
    );
    assert!(event.contains("Plugin selected"), "{event}");

    let text = std::fs::read_to_string(&entry).expect("entry readable");
    assert!(
        text.lines()
            .any(|line| line.trim() == "use demo_plugin as _;"),
        "a commented copy is not an import: {text:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Every path that swaps the registry re-judges the evidence and drops the MIR
/// snapshot built from the old tree.
/// 每一条替换注册表的路径都会重新裁决证据，并丢弃按旧树构建的 MIR 快照。
///
/// The two replacement paths (`poll_hot_reload`, `reload`) used to swap the
/// snapshot without re-running the identity judgement, so the session could keep
/// saying `LIVE` over a trace that no longer held (audit `STU-S-03`).
/// 两条替换路径（`poll_hot_reload`、`reload`）过去替换快照而不重跑身份裁决，因此会话能在
/// 一份不再成立的追踪上继续宣称 `LIVE`（审计 `STU-S-03`）。
#[test]
fn every_snapshot_swap_re_judges_the_evidence() {
    let lifecycle = include_str!("../lifecycle.rs");
    assert_eq!(
        lifecycle.matches("self.rejudge_evidence()").count(),
        // Three now: `poll_hot_reload`, `reload`, and the batch a project write hands
        // back (audit `STU-S-27` wired the third one in).
        // 现在是三条：`poll_hot_reload`、`reload`，以及项目写入交回的那一格（审计 `STU-S-27`）。
        3,
        "every replacement path judges the evidence again"
    );
    for replacement in [
        "Hot reload: registration snapshot refreshed",
        "Registration snapshot reloaded from disk",
    ] {
        assert!(lifecycle.contains(replacement), "{replacement}");
    }
    assert!(
        lifecycle.contains("Trace artifact refused: {reason}"),
        "the judgement's reason reaches the event line instead of being lost"
    );
    let trace = include_str!("../trace.rs");
    assert!(trace.contains("pub(super) fn rejudge_evidence"));
    assert!(
        trace.contains("self.mir_graph = None;"),
        "a MIR graph built from the old tree cannot survive the swap"
    );
}

/// One search scan per (query, snapshot), not one per panel.
/// 每个（查询, 快照）只扫一次，而不是每个面板一次。
///
/// The list, the RELATION column, the SELECTED SYMBOL preview and the graph page all
/// ask `search_rows` in one frame; the memo is what keeps that from being three to
/// five reads plus lexes of every face's source file (audit `STU-S-05`).
/// 结果列表、RELATION 栏、SELECTED SYMBOL 预览与调用图页在同一帧都要问 `search_rows`；备忘
/// 正是让这件事不再是三到五遍读取加词法扫描的原因（审计 `STU-S-05`）。
#[test]
fn search_rows_is_memoised_per_query_and_snapshot() {
    let app_state = include_str!("../app.rs");
    assert!(
        app_state.contains("search_memo: RefCell<Option<SearchMemo>>"),
        "the memo is the query plus the source stamp plus the rows"
    );
    let search_state = include_str!("../state/search.rs");
    assert!(
        search_state.contains("pub(crate) type SearchMemo = (String, u128,"),
        "the memo's shape is named, so the field stays one readable type"
    );
    let queries = include_str!("../search_queries.rs");
    assert!(
        queries.contains("self.search_memo.borrow()"),
        "the memo is consulted before scanning"
    );
    assert!(
        queries.contains("*stamp == self.last_source_stamp"),
        "an external edit invalidates it"
    );
    // The behaviour the memo must not change: the same query answers the same rows.
    // 备忘不得改变的行为：同一查询给出同样的行。
    let app = App::load();
    let first = app.search_rows("no-such-symbol");
    let second = app.search_rows("no-such-symbol");
    assert_eq!(first, second, "the memo must not change the answer");
}
