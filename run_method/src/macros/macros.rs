//! Registration-face macros.
//! 注册面宏。

#[path = "entry.rs"]
mod entry;
#[path = "face.rs"]
mod face;
#[path = "trace.rs"]
mod trace;

pub use self::face::FaceFields;
