//! App source stamps and graph focus navigation.
//! App 源码变更戳与调用图焦点导航。

use super::support::package_root;
use super::*;

pub(super) fn source_stamp() -> u128 {
    let package_root = package_root();
    let mut files = Vec::new();
    for root in [
        package_root.join("src"),
        package_root.join("studio/src"),
        package_root.join(".nichlink/plugins"),
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

pub(super) fn advance_graph_focus(search: &mut SearchState) {
    if search.compare_query.is_none() && search.compare_center.is_none() {
        search.graph_side = 0;
        search.graph_focus = match search.graph_focus {
            0 => 2,
            2 => 3,
            _ => 0,
        };
        search.outline_focus = search.graph_focus == 2;
        return;
    }
    match (search.graph_focus, search.graph_side) {
        (0, _) => {
            search.graph_focus = 1;
            search.graph_side = 1;
        }
        (1, _) => {
            search.graph_focus = 2;
            search.graph_side = 0;
        }
        (2, 0) => search.graph_side = 1,
        (2, 1) => {
            search.graph_focus = 3;
            search.graph_side = 0;
        }
        (3, 0) => search.graph_side = 1,
        _ => {
            search.graph_focus = 0;
            search.graph_side = 0;
        }
    }
    search.outline_focus = search.graph_focus == 2;
}

pub(super) fn retreat_graph_focus(search: &mut SearchState) {
    if search.compare_query.is_none() && search.compare_center.is_none() {
        search.graph_side = 0;
        search.graph_focus = match search.graph_focus {
            0 => 3,
            3 => 2,
            _ => 0,
        };
        search.outline_focus = search.graph_focus == 2;
        return;
    }
    match (search.graph_focus, search.graph_side) {
        (0, _) => {
            search.graph_focus = 3;
            search.graph_side = 1;
        }
        (1, _) => {
            search.graph_focus = 0;
            search.graph_side = 0;
        }
        (2, 0) => {
            search.graph_focus = 1;
            search.graph_side = 1;
        }
        (2, 1) => search.graph_side = 0,
        (3, 0) => search.graph_side = 1,
        (3, 1) => {
            search.graph_focus = 2;
            search.graph_side = 1;
        }
        _ => {
            search.graph_focus = 0;
            search.graph_side = 0;
        }
    }
    search.outline_focus = search.graph_focus == 2;
}
