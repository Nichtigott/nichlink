//! File-backed authoring for NichLink registration faces.
//! NichLink 注册面的文件化创作支持。

use std::fs;
use std::path::{Path, PathBuf};

use crate::{NodeId, ROOT_NODE_ID, RegistrationSnapshot, Registry};

use self::filesystem::{atomic_write, create_new};
use self::parse::trait_names_from_paths;
use self::validation::{
    is_parent_component, normalize_kind_name, normalized_path, package_root, source_root,
    validate_name,
};

const GENERATED_MARKER: &str = "// generated-by=NichLink";

#[path = "external_graft/external_graft.rs"]
pub mod external_graft;
#[path = "face_manifest.rs"]
mod face_manifest;
#[path = "filesystem/filesystem.rs"]
pub mod filesystem;
#[path = "manifest/manifest.rs"]
pub mod manifest;
#[path = "operations/operations.rs"]
pub mod operations;
#[path = "parse/parse.rs"]
pub mod parse;
#[path = "snapshot/snapshot.rs"]
pub mod snapshot;
#[path = "validation/validation.rs"]
pub mod validation;

pub use self::face_manifest::*;
/// The parsed face metadata shared by the form and the file authoring API.
/// 表单与文件创作 API 共用的注册面元数据模型。
pub(crate) use self::manifest::FaceManifest;
