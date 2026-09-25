//! Recursive source-tree walking driven by caller-supplied filesystem facts.
//! 由调用方提供的文件系统事实驱动的递归源码树遍历。
//!
//! The kernel owns the algorithm and never touches the filesystem; an execution
//! surface supplies [`SourceTree`] and chooses a [`SourceWalk`] policy.
//! 内核拥有算法且从不接触文件系统；执行面提供 [`SourceTree`]，并选择 [`SourceWalk`]
//! 策略。

use std::path::{Path, PathBuf};

/// The filesystem facts a recursive source walk needs.
/// 递归源码遍历所需的文件系统事实。
///
/// The kernel owns the algorithm and never touches the filesystem; an execution
/// surface supplies these three operations.
/// 内核拥有算法且从不接触文件系统；三项操作由执行面提供。
pub trait SourceTree {
    /// Whether `path` names a directory; the walk recurses only when this is true.
    /// `path` 是否为目录；仅当为真时遍历才递归进入。
    fn is_directory(&self, path: &Path) -> bool;
    /// The direct entries of `path`; an error aborts the whole walk.
    /// `path` 的直接条目；返回错误会中止整次遍历。
    fn entries(&self, path: &Path) -> Result<Vec<PathBuf>, String>;
    /// Read a `.rs` file when the keep callback asks for its text; an error
    /// skips only that file.
    /// 当 keep 回调索取文本时读取 `.rs` 文件；返回错误只跳过该文件。
    fn read_text(&self, path: &Path) -> Result<String, String>;
}

/// Which subtrees a source walk never enters.
/// 源码遍历永不进入的子树。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceWalk {
    /// Build output, never source.
    /// 构建产物，绝不是源码。
    pub skip_target: bool,
    /// The checked-in registration machinery.
    /// 随仓库提交的注册机制。
    pub skip_registry_core: bool,
    /// The compile-error demonstration tree.
    /// 编译错误演示树。
    pub skip_compile_error_demo: bool,
}

impl SourceWalk {
    /// Enter everything; keep every `.rs` file.
    /// 进入一切目录；保留每个 `.rs` 文件。
    pub const EVERYTHING: Self = Self {
        skip_target: false,
        skip_registry_core: false,
        skip_compile_error_demo: false,
    };

    fn skips(&self, path: &Path) -> bool {
        let named = |name: &str| path.file_name().and_then(|value| value.to_str()) == Some(name);
        let within = |name: &str| path.components().any(|part| part.as_os_str() == name);
        (self.skip_target && named("target"))
            || (self.skip_registry_core && within("registry_core"))
            || (self.skip_compile_error_demo && within("compile_error_demo"))
    }
}

/// What a walk should do with one `.rs` file it found.
/// 遍历发现一个 `.rs` 文件后应当做什么。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    /// Take it without reading it.
    /// 直接收下，不读取内容。
    Yes,
    /// Skip it.
    /// 跳过。
    No,
    /// Read the text and ask again with it.
    /// 读出文本后再问一次。
    NeedSource,
}

/// How deep the walk nests before it assumes the tree is cyclic.
/// 遍历在假定树存在环之前允许达到的深度。
///
/// The kernel takes filesystem facts from its caller and cannot canonicalize a
/// path, so it cannot tell a link that points at an ancestor from a directory
/// that is simply deep. A bound is what keeps the workspace's only recursive
/// traversal from overflowing the stack; 128 directories is far past any real
/// source layout, and a tree that reaches it is reported rather than followed.
/// 内核的文件系统事实来自调用方，且无法 canonicalize 路径，因此它分不出"指向祖先的链接"
/// 与"确实很深的目录"。能阻止 workspace 唯一的递归遍历栈溢出的东西就是一条深度上限；
/// 128 层远超任何真实源码布局，而达到它的树会被报告而不是继续跟随。
pub const MAX_DEPTH: usize = 128;

/// Every `.rs` file under `root`, depth-first, in directory order.
/// `root` 下每个 `.rs` 文件，深度优先，按目录顺序。
///
/// The walk is the only recursive source traversal in the workspace: the build
/// step, the authoring surface and the MCP index all call it with their own
/// [`SourceTree`] and their own [`SourceWalk`] options.
/// 这是整个 workspace 唯一的递归源码遍历：构建步骤、创作面与 MCP 索引都用各自的
/// [`SourceTree`] 与 [`SourceWalk`] 选项调用它。
pub fn collect_rust_sources(
    tree: &impl SourceTree,
    root: &Path,
    walk: SourceWalk,
    mut keep: impl FnMut(&Path, Option<&str>) -> Keep,
    collected: &mut Vec<PathBuf>,
) -> Result<(), String> {
    fn visit(
        tree: &impl SourceTree,
        directory: &Path,
        depth: usize,
        walk: SourceWalk,
        keep: &mut dyn FnMut(&Path, Option<&str>) -> Keep,
        collected: &mut Vec<PathBuf>,
    ) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Err(format!(
                "source tree nests deeper than {MAX_DEPTH} directories at {}; it is cyclic, or too deep for this bound",
                directory.display()
            ));
        }
        for path in tree.entries(directory)? {
            if walk.skips(&path) {
                continue;
            }
            if tree.is_directory(&path) {
                visit(tree, &path, depth + 1, walk, keep, collected)?;
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }
            match keep(&path, None) {
                Keep::No => {}
                Keep::Yes => collected.push(path),
                Keep::NeedSource => {
                    let Ok(source) = tree.read_text(&path) else {
                        continue;
                    };
                    if keep(&path, Some(&source)) == Keep::Yes {
                        collected.push(path);
                    }
                }
            }
        }
        Ok(())
    }
    visit(tree, root, 0, walk, &mut keep, collected)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tree that points back at itself must stop with a report, not recurse
    /// until the stack overflows. The kernel cannot canonicalize a path, so the
    /// bound is the only thing standing between a link loop and a crash.
    /// 指向自身的树必须带着报告停下来，而不是递归到栈溢出。内核无法 canonicalize 路径，
    /// 因此这条上限是链接环与崩溃之间唯一的东西。
    #[test]
    fn a_cyclic_tree_stops_at_the_depth_bound() {
        struct Cyclic;

        impl SourceTree for Cyclic {
            fn is_directory(&self, _path: &Path) -> bool {
                true
            }

            fn entries(&self, path: &Path) -> Result<Vec<PathBuf>, String> {
                // Every directory holds one more directory, forever.
                // 每个目录里都还有一个目录，永远如此。
                Ok(vec![path.join("loop")])
            }

            fn read_text(&self, _path: &Path) -> Result<String, String> {
                Ok(String::new())
            }
        }

        let mut collected = Vec::new();
        let error = collect_rust_sources(
            &Cyclic,
            Path::new("/root"),
            SourceWalk::EVERYTHING,
            |_path, _source| Keep::No,
            &mut collected,
        )
        .expect_err("a cyclic tree must be reported");
        assert!(error.contains("deeper than"), "{error}");
        assert!(collected.is_empty(), "{collected:?}");
    }

    /// A tree that stays inside the bound is walked as before.
    /// 停在上限之内的树照常被遍历。
    #[test]
    fn a_flat_tree_yields_every_rust_file() {
        struct Flat(Vec<PathBuf>, Vec<PathBuf>);

        impl SourceTree for Flat {
            fn is_directory(&self, path: &Path) -> bool {
                self.0.iter().any(|directory| directory == path)
            }

            fn entries(&self, path: &Path) -> Result<Vec<PathBuf>, String> {
                Ok(self
                    .1
                    .iter()
                    .filter(|entry| entry.parent() == Some(path))
                    .cloned()
                    .collect())
            }

            fn read_text(&self, _path: &Path) -> Result<String, String> {
                Ok(String::new())
            }
        }

        let root = PathBuf::from("/src");
        let tree = Flat(
            vec![root.clone()],
            vec![root.join("a.rs"), root.join("b.txt"), root.join("nested")],
        );
        let mut collected = Vec::new();
        collect_rust_sources(
            &tree,
            &root,
            SourceWalk::EVERYTHING,
            |_path, _| Keep::Yes,
            &mut collected,
        )
        .expect("a flat tree walks");
        assert_eq!(collected, vec![root.join("a.rs")]);
    }
}
