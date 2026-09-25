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
    // A unique sibling, never a fixed name. A fixed name has to be deleted
    // before it can be reused, and that deletion is how this used to remove a
    // file it had not created: any sibling that happened to carry the temporary's
    // name was destroyed. The process id and a counter keep two writers — and the
    // leftovers of a crashed one — apart, and the temporary is therefore always
    // ours to remove, which is what the cleanup below does.
    // 唯一的同级文件，绝不用固定名字。固定名字必须先删掉才能复用，而那次删除正是它过去会
    // 删掉一个并非自己创建的文件的途径：任何恰好带着该临时名的同级文件都会被销毁。进程 id
    // 与一个计数器让两个写者（以及崩溃者的残留）互不相撞，因此临时文件始终是我们自己的，
    // 下面的清理也正是这么做的。
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");
    let temporary = path.with_file_name(format!(
        ".{name}.nichlink-{}-{sequence}.tmp",
        std::process::id()
    ));
    let outcome = create_new(&temporary, contents).and_then(|()| {
        fs::rename(&temporary, path)
            .map_err(|error| format!("cannot replace {}: {error}", path.display()))
    });
    if outcome.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A throwaway directory, unique per call.
    /// 每次调用唯一的一次性目录。
    fn fixture(label: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nichlink-atomic-{label}-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("fixture directory");
        root
    }

    /// A sibling that happens to look like a temporary is not this writer's to
    /// delete, and a successful write leaves no temporary of its own behind.
    /// 一个恰好长得像临时文件的同级文件不归本写者删除；而一次成功的写入不会留下自己的临时文件。
    #[test]
    fn writing_replaces_the_target_and_touches_nothing_else() {
        let root = fixture("sibling");
        let target = root.join("face.rs");
        fs::write(&target, "old").expect("target");
        let neighbour = root.join("face.nichlink.tmp");
        fs::write(&neighbour, "someone else's bytes").expect("neighbour");

        atomic_write(&target, "new").expect("atomic write");

        assert_eq!(fs::read_to_string(&target).expect("target"), "new");
        assert_eq!(
            fs::read_to_string(&neighbour).expect("the neighbour survives"),
            "someone else's bytes"
        );
        let leftovers = fs::read_dir(&root)
            .expect("read the directory")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains("nichlink-"))
            .count();
        assert_eq!(leftovers, 0, "no temporary of our own is left behind");

        let _ = fs::remove_dir_all(&root);
    }
}
