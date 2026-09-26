# Studio fixture: `node-editor`

This directory is a **source-only** NichLink host package. Studio indexes a
project by reading `<package-root>/src/` as text through
`nichlink_run_method::generated_snapshots_from`, so this package exists purely
so the `#[cfg(feature = "prototype-fixtures")]` tests under
`studio/src/studio/app/tests/` have a real registration tree to search,
navigate, and resolve source paths against. Nothing here is compiled: the
package is deliberately **not** a workspace member, has no `target/` directory,
and its own `[workspace]` table keeps Cargo from folding it into the NichLink
workspace.

本目录是一个**仅源码**的 NichLink 宿主包。Studio 通过
`nichlink_run_method::generated_snapshots_from` 把 `<package-root>/src/` 当文本读取来
索引项目，因此本包的存在只是为了让 `studio/src/studio/app/tests/` 下
`#[cfg(feature = "prototype-fixtures")]` 的测试有一棵真实的注册树可供搜索、导航，并
解析源码路径。这里的内容不会被编译：本包刻意不是工作区成员，没有 `target/` 目录，其
自带的 `[workspace]` 表也让 Cargo 无法把它并入 NichLink 工作区。

## Why it exists

The ten tests it backs are the only coverage of `App::call_relations`,
`App::mir_candidates_for`, and `App::search_rows` against a multi-file host.
Deleting them was rejected for exactly that reason, so the fixture they were
written against is checked in instead.

它所支撑的十个测试是 `App::call_relations`、`App::mir_candidates_for` 与
`App::search_rows` 针对多文件宿主仅有的覆盖。正因如此，删除这些测试被否决，改为把
它们当初所用的夹具签入仓库。

## Source tree

| Path | What it provides |
| --- | --- |
| `src/lib.rs` | Host wiring (`host!()`), no registration face. |
| `src/control/control.rs` | `control` folder face owning a `Registry`, plus `ControlFrame` / `ControlHandle` contracts. |
| `src/control/registry_rule/registry_rule.rs` | The `control` admission rule for direct children. |
| `src/control/object/node_editor/node_editor.rs` | `node_editor` face (own registry) declaring `preview_canvas_width` and `preview_canvas_width_traced`. |
| `src/control/object/node_editor/registry_rule/registry_rule.rs` | The `node_editor` admission rule for direct children. |
| `src/control/object/node_editor/object/object.rs` | `object` leaf face owning `clamp_canvas_width` and `accept_canvas(canvas_name, …)`. |
| `build.rs` | The host build entry, mirroring `examples/control-button`. Never run for tests. |

## Tests that use it

- `studio/src/studio/app/tests/graph.rs` — graph navigation, panel focus,
  caller promotion, and `call_tree_targets`.
- `studio/src/studio/app/tests/source.rs` — cross-file `call_relations`, MIR
  candidates, directory-style `source_path_for`, and `search_rows` over files,
  functions, and parameters.

## Live trace evidence

Two fixture tests assert about *live* evidence, which a source tree cannot
supply: `graph_tab_reaches_both_tree_and_data_panels` needs at least two locals
for the selected tree row, and `mir_candidates_are_optional_and_keep_live_evidence_distinct`
needs a live call edge from the `node_editor` face to the `object` face. Studio
ships no demo trace: the built-in sample was removed when the artifact loader
(`studio/src/studio/app/trace.rs`) landed, so those tests install their own trace
over this fixture instead, from
`studio/src/studio/app/tests/fixtures.rs` (`fixture_live_trace`). Neither
assertion was weakened.

## 实测证据

两条夹具测试断言的是**实测**证据，而源码树提供不了它：
`graph_tab_reaches_both_tree_and_data_panels` 需要选中树行至少有两个局部值，
`mir_candidates_are_optional_and_keep_live_evidence_distinct` 需要一条从 `node_editor`
面到 `object` 面的实测调用边。Studio 不自带演示追踪：artifact 加载方
（`studio/src/studio/app/trace.rs`）落地时内置样本已被删除，因此这两条测试改为从
`studio/src/studio/app/tests/fixtures.rs`（`fixture_live_trace`）安装一条针对本夹具的
追踪。两条断言都没有被削弱。
