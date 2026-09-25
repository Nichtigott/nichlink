//! Runtime call frames, observed data edges, and locals.
//! 运行时调用帧、已观测数据边与局部值。

#[path = "call_trace.rs"]
mod call_trace;
#[path = "edges/edges.rs"]
pub mod edges;
#[path = "frames/frames.rs"]
pub mod frames;
#[path = "locals/locals.rs"]
pub mod locals;

use crate::registry_core::declaration::SourceLocation;
use crate::registry_core::identity::NodeId;

pub use frames::FramePath;
pub use nichlink::CallSite;
pub use nichlink::declaration::source_file_matches;

pub use self::call_trace::*;

pub use nichlink::TraceMode;
