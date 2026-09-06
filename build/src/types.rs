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
        }
    }
}
