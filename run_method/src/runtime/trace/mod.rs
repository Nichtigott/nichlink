#[path = "edges/edges.rs"]
pub mod edges;
#[path = "frames/frames.rs"]
pub mod frames;
#[path = "locals/locals.rs"]
pub mod locals;

pub use frames::FramePath;
pub use nichlink::CallSite;

include!("trace.rs");
pub use nichlink::TraceMode;
