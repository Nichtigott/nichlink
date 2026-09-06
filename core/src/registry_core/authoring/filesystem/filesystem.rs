//! Small filesystem primitives used by authoring transactions.
//! 创作事务使用的文件系统小工具。

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub(super) fn create_new(path: &Path, contents: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
    file.write_all(contents.as_bytes())
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

pub(super) fn atomic_write(path: &Path, contents: &str) -> Result<(), String> {
    let temporary = path.with_extension("nichlink.tmp");
    if temporary.exists() {
        fs::remove_file(&temporary)
            .map_err(|error| format!("cannot clear stale temporary file: {error}"))?;
    }
    create_new(&temporary, contents)?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("cannot replace {}: {error}", path.display()))
}
