//! Build-script input paths and their environment bindings.
//! 构建脚本输入路径及其环境绑定。

use std::env;
use std::path::PathBuf;

use super::source_layout::{SourceLayout, source_layout};

/// One build run's resolved paths.
/// 一次构建运行解析出的路径。
pub(crate) struct BuildInput {
    pub(crate) manifest: PathBuf,
    /// Where the faces live. A layout problem is carried rather than raised here,
    /// because the two callers report it differently and neither wants a panic
    /// from a constructor.
    /// 注册面住在哪里。布局问题被携带而不是在这里抛出，因为两个调用方报告它的方式不同，而且都
    /// 不想要构造函数 panic。
    pub(crate) layout: Result<SourceLayout, String>,
    pub(crate) out_dir: PathBuf,
    /// Whether to emit `cargo:` directives on stdout. Cargo build scripts set
    /// this; standalone CLI runs leave it off.
    /// 是否在 stdout 输出 `cargo:` 指令。Cargo build script 置位；独立 CLI
    /// 运行时不输出。
    pub(crate) emit_cargo_directives: bool,
    /// The subtrees this package compiles instead of the whole host (`None` = the whole host), and
    /// whether it is the facade — the cross-crate half. Both come from the generator as **arguments**
    /// now: they used to arrive through `NICH_LINK_SHAPE_*`, which a generated build script can only
    /// set with `unsafe { std::env::set_var(…) }` under Rust 2024.
    /// 这个包编译哪些子树（`None`＝整个宿主），以及它是不是 facade（跨 crate 那一半）。两者现在由生成器
    /// 以**参数**给出：过去经 `NICH_LINK_SHAPE_*` 传递，而生成的构建脚本在 Rust 2024 下只能靠
    /// `unsafe { std::env::set_var(…) }` 设置它们。
    pub(crate) only: Option<String>,
    pub(crate) facade: bool,
}

impl BuildInput {
    pub(crate) fn from_environment() -> Self {
        let manifest =
            PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
        let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"));
        Self::new(manifest, out_dir, true)
    }

    /// The input for one package root, as every non-build-script caller has it.
    /// 从每个非构建脚本调用方都持有的包根构造输入。
    pub(crate) fn new(manifest: PathBuf, out_dir: PathBuf, emit_cargo_directives: bool) -> Self {
        let layout = source_layout(&manifest);
        Self {
            manifest,
            layout,
            out_dir,
            emit_cargo_directives,
            only: None,
            facade: false,
        }
    }

    /// The same input, for a fragment a partition generated: it compiles `only` instead of the whole
    /// host, or it is the facade.
    /// 同一个输入，但用于划分生成的碎片：它编译 `only` 而不是整个宿主，或者它就是 facade。
    pub(crate) fn with_shape(mut self, only: Option<String>, facade: bool) -> Self {
        self.only = only;
        self.facade = facade;
        self
    }
}
