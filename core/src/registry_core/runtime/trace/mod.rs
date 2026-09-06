#[path = "edges/edges.rs"]
pub mod edges;
#[path = "frames/frames.rs"]
pub mod frames;
#[path = "locals/locals.rs"]
pub mod locals;

pub use frames::FramePath;

include!("trace.rs");
