use std::env;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(crate) struct Node {
    pub(crate) name: String,
    pub(crate) file: Option<PathBuf>,
    pub(crate) children: Vec<Node>,
}

pub(crate) struct BuildInput {
    pub(crate) manifest: PathBuf,
    pub(crate) src: PathBuf,
    pub(crate) out_dir: PathBuf,
    /// Whether to emit `cargo:` directives on stdout. Cargo build scripts set
    /// this; standalone CLI runs leave it off.
    /// 是否在 stdout 输出 `cargo:` 指令。Cargo build script 置位；独立 CLI
    /// 运行时不输出。
    pub(crate) emit_cargo_directives: bool,
}

impl BuildInput {
    pub(crate) fn from_environment() -> Self {
        let manifest =
            PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"));
        let src = manifest.join("src");
        let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"));
        Self {
            manifest,
            src,
            out_dir,
            emit_cargo_directives: true,
        }
    }
}
