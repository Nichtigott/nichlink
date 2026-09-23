//! The discovered registration module tree and its path naming.
//! 已发现的注册模块树及其路径命名。

use std::path::{Path, PathBuf};

/// One discovered registration module.
/// 一个已发现的注册模块。
#[derive(Clone, Debug)]
pub(crate) struct Node {
    pub(crate) name: String,
    pub(crate) file: Option<PathBuf>,
    pub(crate) children: Vec<Node>,
}

/// Display a discovered file path relative to the package source root.
/// 按相对包源码根的写法显示已发现文件的路径。
///
/// Generated manifests, caches and diagnostics all name sources the same way, so
/// the conversion lives in one place.
/// 生成的清单、缓存与诊断都用同一种写法命名源码，因此这份转换只写一次。
pub(crate) fn relative_display(src: &Path, path: &Path) -> String {
    path.strip_prefix(src)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
