//! App source stamps and graph focus navigation.
//! App 源码变更戳与调用图焦点导航。

use super::support::package_root;
use super::*;
use crate::studio::app::call_tree_queries::TreeStep;

pub(super) fn source_stamp() -> u128 {
    let package_root = package_root();
    let mut files = Vec::new();
    for root in [
        package_root.join("src"),
        package_root.join("studio/src"),
        package_root.join(".nichlink/plugins"),
        // A plan is an authoring record: editing or deleting one must refresh
        // the graft screen, even though the registration tree does not change.
        // 计划是创作记录：编辑或删除它必须刷新 graft 界面，尽管注册树本身没变。
        package_root
            .join(nichlink_run_method::lexicon::NICHLINK_DIR)
            .join(nichlink_run_method::lexicon::EXTERNAL_GRAFT_DIR),
    ] {
        stamp_directory(&root, &mut files);
    }
    for file in [
        package_root.join("Cargo.toml"),
        package_root.join("build.rs"),
    ] {
        stamp_file(&file, &mut files);
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut stamp = 0u128;
    for (path, modified, size) in files {
        for byte in path.to_string_lossy().bytes() {
            stamp = stamp.wrapping_mul(1_000_003).wrapping_add(u128::from(byte));
        }
        stamp = stamp.wrapping_mul(1_000_003).wrapping_add(modified);
        stamp = stamp.wrapping_mul(1_000_003).wrapping_add(u128::from(size));
    }
    stamp
}

fn stamp_file(path: &std::path::Path, files: &mut Vec<(std::path::PathBuf, u128, u64)>) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_nanos());
    files.push((path.to_owned(), modified, metadata.len()));
}

pub(super) fn stamp_directory(
    path: &std::path::Path,
    files: &mut Vec<(std::path::PathBuf, u128, u64)>,
) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.is_dir() {
            if path
                .file_name()
                .is_some_and(|name| name == "target" || name == ".git")
            {
                continue;
            }
            stamp_directory(&path, files);
            continue;
        }
        if !metadata.is_file() {
            continue;
        }
        let relevant = path.extension().and_then(|extension| extension.to_str()) == Some("rs")
            || path.file_name().is_some_and(|name| {
                matches!(
                    name.to_str(),
                    Some("Cargo.toml" | "Cargo.lock" | "official.lock")
                )
            });
        if !relevant {
            continue;
        }
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_nanos());
        files.push((path, modified, metadata.len()));
    }
}

/// Widen or narrow the call tree's share of the page, one step at a time and
/// within the same range the divider drag uses, so the key and the mouse cannot
/// disagree about how wide the tree may get.
/// 加宽或收窄调用树在页面上所占的份额：每次一步，且与拖动分隔线使用同一范围，因此按键与鼠标
/// 不会对"树最多能多宽"各说一套。
impl App {
    pub(super) fn shift_graph_split(&mut self, narrower: bool) {
        let step: i16 = if narrower { -4 } else { 4 };
        self.graph_split_percent = (self.graph_split_percent as i16 + step)
            .clamp(35, 65)
            .unsigned_abs();
        self.event = format!(
            "tree share {}% — [ narrows it, ] widens it",
            self.graph_split_percent
        );
    }
}

/// One arrow press in the graph page: the tree's cursor when the tree holds the
/// focus, the value pane's own cursor when it does not.
/// 调用图页的一次方向键：树持有焦点时移动树的游标，否则移动取值面板自己的游标。
impl App {
    pub(super) fn step_graph_cursor(
        &mut self,
        search: &mut SearchState,
        key: crossterm::event::KeyCode,
        tree_len: usize,
        data_len: usize,
    ) {
        use crossterm::event::KeyCode;
        if search.graph_focus != 0 {
            // The value pane is a list, so only its two ends are directions.
            // 取值面板是一个列表，因此只有上下两个方向。
            match key {
                KeyCode::Up => search.data_selected = search.data_selected.saturating_sub(1),
                KeyCode::Down => {
                    search.data_selected =
                        (search.data_selected + 1).min(data_len.saturating_sub(1));
                }
                _ => {}
            }
            return;
        }
        let step = match key {
            KeyCode::Up => TreeStep::Up,
            KeyCode::Down => TreeStep::Down,
            KeyCode::Left => TreeStep::Left,
            _ => TreeStep::Right,
        };
        self.hop_call_tree(search, step);
        // A tree that does not answer the step reports that and keeps its cursor;
        // the clamp is for a cursor left past the end by a re-centred tree.
        // 树若回答不了这一步会说明并保持游标；这里的钳制是给"重新居中后游标越界"用的。
        search.outline_selected = search.outline_selected.min(tree_len.saturating_sub(1));
    }
}

pub(super) fn advance_graph_focus(search: &mut SearchState) {
    // Two panes: the tree, then the values its cursor's function ran with.
    // 两块面板：调用树，然后是它游标所在函数运行时的取值。
    search.graph_focus = match search.graph_focus {
        0 => 1,
        _ => 0,
    };
    search.outline_focus = search.graph_focus == 0;
}

pub(super) fn retreat_graph_focus(search: &mut SearchState) {
    // BackTab walks the same two panes the other way.
    // BackTab 以相反方向走同样这两块面板。
    search.graph_focus = match search.graph_focus {
        0 => 1,
        _ => 0,
    };
    search.outline_focus = search.graph_focus == 0;
}
