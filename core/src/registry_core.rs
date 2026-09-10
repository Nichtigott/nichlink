//! Core registry modules. Concrete application faces live in the consuming crate.

#[path = "registry_core/authoring/mod.rs"]
#[cfg(feature = "syntax")]
pub mod authoring;
#[path = "registry_core/declaration/declaration.rs"]
pub mod declaration;
#[path = "registry_core/diagnostic/diagnostic.rs"]
pub mod diagnostic;
#[path = "registry_core/identity/identity.rs"]
pub mod identity;
#[path = "registry_core/mir/mir.rs"]
pub mod mir;
#[path = "registry_core/plugin/mod.rs"]
pub mod plugin;
#[path = "registry_core/release/release.rs"]
pub mod release;
#[path = "registry_core/requirements/requirements.rs"]
pub mod requirements;
#[path = "registry_core/source/source.rs"]
pub mod source;
#[path = "registry_core/syntax/syntax.rs"]
#[cfg(feature = "syntax")]
pub mod syntax;
#[path = "registry_core/tree/mod.rs"]
pub mod tree;

#[cfg(feature = "syntax")]
#[allow(ambiguous_glob_reexports)]
pub use authoring::*;
#[allow(ambiguous_glob_reexports)]
pub use declaration::*;
#[allow(ambiguous_glob_reexports)]
pub use diagnostic::*;
pub use identity::*;
pub use mir::*;
#[allow(ambiguous_glob_reexports)]
pub use plugin::*;
pub use release::*;
pub use requirements::*;
pub use source::*;
#[cfg(feature = "syntax")]
pub use syntax::*;
#[allow(ambiguous_glob_reexports)]
pub use tree::*;
