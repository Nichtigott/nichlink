//! Incremental identity cache helpers.
//! 增量身份缓存辅助函数。

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use super::discovery::collect_source_files;
use super::registry_identity::NodeId;
use super::types::Node;

pub(crate) fn prime_node_id_cache(manifest: &Path, src: &Path, nodes: &[Node]) {
    let target = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map_or_else(
            || manifest.join("target"),
            |path| {
                if path.is_absolute() {
                    path
                } else {
                    manifest.join(path)
                }
            },
        );
    let units = target.join("nichlink/cache/units");
    let mut values = BTreeMap::new();
    let mut files = Vec::new();
    collect_source_files(nodes, &mut files);
    for file in files {
        let relative = super::relative_display(src, &file);
        let fingerprint = source_unit_fingerprint(&file);
        let unit = units.join(format!("{fingerprint}.tsv"));
        let Ok(content) = fs::read_to_string(unit) else {
            continue;
        };
        if content
            .lines()
            .find_map(|line| line.strip_prefix("# schema\t"))
            != Some(super::CACHE_SCHEMA)
        {
            continue;
        }
        let path_matches = content
            .lines()
            .find_map(|line| line.strip_prefix("path\t"))
            .is_some_and(|path| path == relative);
        let id = content
            .lines()
            .find_map(|line| line.strip_prefix("node\t"))
            .and_then(|value| value.parse::<NodeId>().ok());
        if let (true, Some(id)) = (path_matches, id) {
            let kind = content
                .lines()
                .find_map(|line| line.strip_prefix("kind\t"))
                .unwrap_or_default()
                .to_owned();
            if !kind.is_empty() && id == super::registry_identity::package_node_id(&relative, &kind)
            {
                values.insert(relative, (id, kind));
            }
        }
    }
    let _ = super::CACHED_NODE_IDS.set(values);
}

pub(crate) fn valid_cached_unit(content: &str, path: &str, id: NodeId) -> bool {
    content
        .lines()
        .find_map(|line| line.strip_prefix("# schema\t"))
        == Some(super::CACHE_SCHEMA)
        && content.lines().find_map(|line| line.strip_prefix("path\t")) == Some(path)
        && content
            .lines()
            .find_map(|line| line.strip_prefix("node\t"))
            .and_then(|value| value.parse::<NodeId>().ok())
            == Some(id)
}

pub(crate) fn source_unit_fingerprint(path: &Path) -> String {
    let source = fs::read(path).unwrap_or_default();
    let mut input = path.to_string_lossy().as_bytes().to_vec();
    input.push(0);
    input.extend_from_slice(&source);
    NodeId::from_bytes(&input).to_string()
}

pub(crate) fn cache_directory(manifest: &Path) -> PathBuf {
    env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map_or_else(
            || manifest.join("target/nichlink/cache"),
            |path| {
                if path.is_absolute() {
                    path.join("nichlink/cache")
                } else {
                    manifest.join(path).join("nichlink/cache")
                }
            },
        )
}
