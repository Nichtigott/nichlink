#[path = "filesystem/filesystem.rs"]
pub mod filesystem;
#[path = "manifest/mod.rs"]
pub mod manifest;
#[path = "operations/operations.rs"]
pub mod operations;
#[path = "parse/parse.rs"]
pub mod parse;
#[path = "snapshot/snapshot.rs"]
pub mod snapshot;
#[path = "validation/validation.rs"]
pub mod validation;

include!("authoring.rs");
