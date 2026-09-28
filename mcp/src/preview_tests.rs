//! Tests for the throwaway copy and the diff a preview reports.
//! 一次性副本与预览报告的 diff 的测试。

use std::path::PathBuf;

/// A directory tree that is not a package on purpose: the copy and the diff walk
/// names, not manifests, so a fixture with the directories in question is the
/// honest one.
/// 一个有意不像包的目录树：副本与 diff 遍历的是名字而不是清单，因此放上相关目录的 fixture 才是
/// 诚实的。
fn tree(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-mcp-preview-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("source directory");
    std::fs::write(root.join("src/lib.rs"), "fn a() {}\n").expect("source file");
    std::fs::create_dir_all(root.join("target")).expect("build output directory");
    std::fs::write(root.join("target/artifact"), "built\n").expect("build artifact");
    std::fs::create_dir_all(root.join(".git")).expect("version-control directory");
    std::fs::write(root.join(".git/config"), "[core]\n").expect("version-control file");
    std::fs::create_dir_all(root.join(nichlink::lexicon::NICHLINK_DIR)).expect("runtime directory");
    std::fs::write(
        root.join(nichlink::lexicon::NICHLINK_DIR).join("state"),
        "runtime\n",
    )
    .expect("runtime file");
    root
}

/// The copy and the diff skip the same three directories: the build output
/// `target/`, the version-control store `.git/`, and NichLink's runtime data
/// `.nichlink/`. Both halves were once copied and walked — every preview carried
/// `.git/` (9.9 MB in this checkout, routinely gigabytes in a real repository) and
/// reported files the edit never touched (audit `BR-C2`, `BR-4`).
/// 副本与 diff 跳过同样那三个目录：构建产物 `target/`、版本库 `.git/` 与 NichLink 的运行期数据
/// `.nichlink/`。两处过去都既复制又遍历——每一次预览都带上 `.git/`（本检出里 9.9 MB，真实仓库里
/// 常是 GB 级）并报出编辑根本没碰过的文件（审计 `BR-C2`、`BR-4`）。
///
/// This is the in-repo form of the probe t5 used out of tree (`/tmp/t5probe.py`
/// drove `nichlink.apply` over stdio against a fixture with a binary `.git/index`):
/// same set, same verdict, expressed as the unit the behaviour is decided in.
/// 这是 t5 在仓外所用探针（`/tmp/t5probe.py` 经 stdio 驱动 `nichlink.apply`，夹具里有二进制
/// `.git/index`）的仓内形态：同一集合、同一结论，只是用决定该行为的那个单位表达。
#[test]
fn a_preview_skips_the_build_output_and_both_stores() {
    let root = tree("skip");
    let copy = super::copy_package(&root).expect("the copy succeeds");
    for skipped in ["target", ".git", nichlink::lexicon::NICHLINK_DIR] {
        assert!(
            !copy.join(skipped).exists(),
            "the preview copy must skip `{skipped}`: {}",
            copy.display()
        );
    }
    assert!(
        copy.join("src/lib.rs").is_file(),
        "the copy must still carry the edited sources"
    );
    // The other half of the same rule: the diff walk skips them too, so nothing
    // under them can be reported as added, changed, or removed.
    // 同一条规则的另一半：diff 遍历也跳过它们，因此它们下面的任何东西都不会被报成新增、改动或删除。
    let diff = super::diff_package(&root, &copy).expect("the diff renders");
    for skipped in ["target", ".git", nichlink::lexicon::NICHLINK_DIR] {
        assert!(
            !diff.contains(skipped),
            "the diff walk must skip `{skipped}` too: {diff}"
        );
    }
    assert!(
        diff.is_empty(),
        "nothing changed between the tree and its copy: {diff}"
    );
    super::remove_copy(&root, &copy);
    let _ = std::fs::remove_dir_all(&root);
}
