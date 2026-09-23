//! Shared structured diagnostics and terminal tree rendering.
//! 共享结构化诊断与终端树形渲染。

use std::fmt::{self, Write as _};

use crate::registry_core::declaration::{CallSite, ProvenanceStep};
use crate::registry_core::declaration::{OwnedSourceLocation, SourceLocation};
use crate::registry_core::identity::NodeId;

#[path = "error.rs"]
mod error;
pub use error::*;
#[path = "build.rs"]
mod build;
pub use build::*;
#[path = "topology.rs"]
mod topology;
pub use topology::*;
