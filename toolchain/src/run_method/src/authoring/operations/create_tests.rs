//! The rollback report: a failed creation says what it could not take back.
//! 回滚报告：失败的创建会说出它没能收回什么。

use super::PartialWrite;
use std::fs;
use std::path::PathBuf;

/// A directory no other test shares.
/// 一个没有别的测试共用的目录。
fn unique_directory(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "xirang-lg48-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    directory
}

/// A created file is removed, and the original error is the whole message.
/// 已创建的文件会被移除，消息就是最初那条错误。
#[test]
fn a_created_file_is_taken_back() {
    let directory = unique_directory("file");
    fs::create_dir_all(&directory).expect("fixture directory");
    let file = directory.join("widget.rs");
    fs::write(&file, "// fixture").expect("fixture file");

    let mut partial = PartialWrite::new();
    partial.track_file(file.clone());
    let message = partial.report("registration rejected".to_owned());
    assert_eq!(message, "registration rejected");
    assert!(!file.exists(), "the created file is gone");
    let _ = std::fs::remove_dir_all(&directory);
}

/// A directory this call created and left empty is removed again, and that is
/// not a rollback failure.
/// 本次调用创建、最终为空的目录会被再次移除，且这不算回滚失败。
#[test]
fn a_created_empty_directory_is_taken_back_silently() {
    let directory = unique_directory("empty");
    let mut partial = PartialWrite::new();
    partial.track_directory(&directory);
    fs::create_dir_all(&directory).expect("fixture directory");

    let message = partial.report("cannot write the face".to_owned());
    assert_eq!(message, "cannot write the face");
    assert!(!directory.exists(), "the empty directory is gone");
}

/// A directory with the caller's content in it is left alone: `DirectoryNotEmpty`
/// is somebody else's file, not a failed rollback.
/// 里面有调用方内容的目录保持原样：`DirectoryNotEmpty` 是别人的文件，不是失败的回滚。
#[test]
fn a_directory_holding_someone_elses_content_is_left_alone() {
    let directory = unique_directory("occupied");
    fs::create_dir_all(&directory).expect("fixture directory");
    let keep = directory.join("keep.rs");
    fs::write(&keep, "// not ours to remove").expect("fixture file");

    let mut partial = PartialWrite::new();
    partial.track_directory(&directory);
    let message = partial.report("cannot write the face".to_owned());
    assert_eq!(
        message, "cannot write the face",
        "a non-empty directory is not a rollback failure"
    );
    assert!(keep.exists(), "the caller's file survives");
    let _ = std::fs::remove_dir_all(&directory);
}

/// A rollback that really fails is reported, so the caller is never told the tree
/// is clean while something is still on disk.
/// 真正失败的回滚会被报告出来，因此调用方绝不会在磁盘上还留着东西时被告知树是干净的。
///
/// Red before the fix: `create.rs` discarded these errors (`let _ = fs::remove_file(&rule);`),
/// so the caller saw only the first failure. The fixture makes the removal fail
/// deterministically by handing `report` a path that is a directory: `remove_file`
/// refuses it whatever the permissions are.
/// 修前为红：`create.rs` 丢弃了这些错误（`let _ = fs::remove_file(&rule);`），因此调用方只看到
/// 最初那条错误。夹具让移除确定性地失败：交给 `report` 的路径其实是一个目录，`remove_file`
/// 无论权限如何都会拒绝它。
#[test]
fn a_failed_rollback_is_named_in_the_message() {
    let directory = unique_directory("failed");
    fs::create_dir_all(&directory).expect("fixture directory");

    let mut partial = PartialWrite::new();
    partial.track_file(directory.clone());
    let message = partial.report("cannot write the face".to_owned());
    assert!(
        message.starts_with("cannot write the face"),
        "the first failure still comes first: {message}"
    );
    assert!(
        message.contains("rollback incomplete"),
        "a rollback that could not undo its work must say so: {message}"
    );
    assert!(
        message.contains(&directory.display().to_string()),
        "the message names what is still on disk: {message}"
    );
    let _ = std::fs::remove_dir_all(&directory);
}
