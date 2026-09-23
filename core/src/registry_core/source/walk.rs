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
        walk: SourceWalk,
        keep: &mut dyn FnMut(&Path, Option<&str>) -> Keep,
        collected: &mut Vec<PathBuf>,
    ) -> Result<(), String> {
        for path in tree.entries(directory)? {
            if walk.skips(&path) {
                continue;
            }
            if tree.is_directory(&path) {
                visit(tree, &path, walk, keep, collected)?;
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
    visit(tree, root, walk, &mut keep, collected)
}
