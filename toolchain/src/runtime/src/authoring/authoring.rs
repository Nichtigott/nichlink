//! File-backed authoring for NichLink registration faces.
//! NichLink 注册面的文件化创作支持。

use std::fs;
use std::path::{Path, PathBuf};

use crate::runtime::{NodeId, ROOT_NODE_ID, RegistrationSnapshot, Registry};

use self::context::{
    is_parent_component, normalize_kind_name, normalized_path, package_root, source_root,
    validate_name,
};
use self::filesystem::{atomic_write, create_new};
use self::parse::trait_names_from_paths;

const GENERATED_MARKER: &str = "// generated-by=NichLink";

#[path = "context.rs"]
pub mod context;
#[path = "external_graft/external_graft.rs"]
pub mod external_graft;
#[path = "face_file.rs"]
mod face_file;
#[path = "filesystem.rs"]
pub mod filesystem;
#[path = "manifest/manifest.rs"]
pub mod manifest;
#[path = "operations/operations.rs"]
pub mod operations;
#[path = "parse/parse.rs"]
pub mod parse;
#[path = "snapshot/snapshot.rs"]
pub mod snapshot;
// The historical public path stays: a host that already writes
// `crate::runtime::authoring::validation::…` keeps compiling. The module itself is
// `context` (audit `NAM-03`); this alias is pinned by `conventions/src/shims.rs`, so it cannot
// be deleted by accident.
// 历史公开路径保留：已经写着 `crate::runtime::authoring::validation::…` 的宿主照样编译。
// 模块本体现在是 `context`（审计 `NAM-03`）；这条别名由 `conventions/src/shims.rs` 钉住，
// 不会因疏忽被删掉。
pub use self::context as validation;

pub use self::face_file::*;
/// The parsed face metadata shared by the form and the file authoring API.
/// 表单与文件创作 API 共用的注册面元数据模型。
pub(crate) use self::manifest::FaceManifest;
