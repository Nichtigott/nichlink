//! NichLink's registry protocol and runtime core.

#[macro_use]
#[path = "registry_core/macros/macros.rs"]
mod registration_macros;

#[path = "registry_core.rs"]
pub mod registry_core;

pub use registry_core::*;
