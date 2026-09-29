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
/// Generated manifests, caches and diagnostics all name sources the same way, and
/// so does every other surface that renders a recorded `file!()` path, so the
/// conversion is not written here: it forwards to the kernel's
/// [`nichlink::declaration::portable_path`], the one fold in the workspace. A
/// second copy of the rule is how two surfaces end up spelling the same file
/// differently — and that failure is invisible on the platform whose separators
/// happen to agree.
/// 生成的清单、缓存与诊断都用同一种写法命名源码，其他渲染记录下来的 `file!()` 路径的执行面也是，
/// 因此这份转换不在这里写：它转发到内核的 [`nichlink::declaration::portable_path`]，那是全仓
/// 唯一的折叠。规则的第二份副本正是两个面对同一个文件给出不同拼法的成因——而在分隔符恰好一致的
/// 平台上，这种失败看不出来。
pub(crate) fn relative_display(src: &Path, path: &Path) -> String {
    nichlink::declaration::portable_path(&path.strip_prefix(src).unwrap_or(path).to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::relative_display;
    use std::path::Path;

    /// The fold this function produces *is* the kernel's: the assertion compares
    /// against [`nichlink::declaration::portable_path`] instead of against a
    /// literal, so a second, divergent fold in this file fails even on a platform
    /// whose separators make the difference unobservable.
    /// 本函数产出的折叠**就是**内核的那一份：断言与
    /// [`nichlink::declaration::portable_path`] 比较而不是与字面量比较，因此本文件里第二份
    /// 走向不同的折叠，会在分隔符让差异不可见的平台上同样失败。
    #[test]
    fn a_relative_display_is_the_kernel_fold() {
        let root = Path::new("pkg/src");
        let written = Path::new("pkg/src/control\\object\\button.rs");
        assert_eq!(
            relative_display(root, written),
            nichlink::declaration::portable_path("control/object/button.rs")
        );
        assert_eq!(
            relative_display(root, written),
            "control/object/button.rs",
            "a path recorded outside Windows still renders the portable way"
        );
    }
}
