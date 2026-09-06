//! Core registry modules. Concrete application faces live in the consuming crate.

#[path = "registry_core/authoring/mod.rs"]
#[cfg(feature = "authoring")]
pub mod authoring;
#[path = "registry_core/call_report/call_report.rs"]
pub mod call_report;
#[path = "registry_core/declaration/declaration.rs"]
pub mod declaration;
#[path = "registry_core/diagnostic/diagnostic.rs"]
pub mod diagnostic;
#[path = "registry_core/entry/entry.rs"]
pub mod entry;
#[path = "registry_core/identity/identity.rs"]
pub mod identity;
#[path = "registry_core/plugin/mod.rs"]
pub mod plugin;
#[path = "registry_core/registry/mod.rs"]
pub mod registry;
#[path = "registry_core/release/release.rs"]
pub mod release;
#[path = "registry_core/runtime/mod.rs"]
pub mod runtime;
#[path = "registry_core/syntax/syntax.rs"]
#[cfg(feature = "authoring")]
pub mod syntax;

#[cfg(feature = "authoring")]
#[allow(ambiguous_glob_reexports)]
pub use authoring::*;
pub use call_report::*;
pub use declaration::*;
pub use diagnostic::*;
pub use entry::*;
pub use identity::*;
pub use plugin::*;
pub use registry::*;
pub use release::*;
pub use runtime::*;
#[cfg(feature = "authoring")]
pub use syntax::*;
