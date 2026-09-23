//! Face authoring data, pure validation, and the parsing built on `syn`.
//! 注册面创作数据、纯校验，以及建立在 `syn` 之上的解析。
//!
//! The name table and presentation metadata are plain data: they are useful to
//! any surface that shows a face, whether or not the `syntax` feature is on, so
//! they are not behind the gate. Only the parser — and the snapshot layer built
//! on it — need `syn`. The file names say which is which.
//! 名字表与展示元数据是纯数据：任何展示注册面的执行面都用得上，无论 `syntax` 特性是否
//! 打开，因此不放在门控里。需要 `syn` 的只有解析器，以及建立在它之上的快照层。文件名
//! 直接说明哪个是哪个。

#[path = "field_names.rs"]
mod field_names;
pub use field_names::*;
#[path = "field_presentation.rs"]
mod field_presentation;
pub use field_presentation::*;

#[path = "parse/parse.rs"]
#[cfg(feature = "syntax")]
pub mod parse;
#[path = "snapshot/snapshot.rs"]
#[cfg(feature = "syntax")]
pub mod snapshot;
#[path = "validation/validation.rs"]
pub mod validation;
