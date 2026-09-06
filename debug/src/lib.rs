//! Optional runtime evidence model.
//!
//! `EvidenceKind` is re-exported from core and is the single evidence type
//! used by MIR, live traces, source scanners, graph adapters, and Studio.
//! MIR and source candidates are informative only; `Live` is the sole
//! confirmed execution evidence. Locals marked `unobserved` have no runtime
//! value and must not be treated as observed data.

pub mod adapters;
pub mod collector;
pub mod mir;

#[doc(hidden)]
pub use inventory;

/// Local inventory payload avoids orphan-rule coupling to core's declaration
/// type while keeping the collected value zero-copy.
#[doc(hidden)]
pub struct CollectedRegistration(pub &'static nichlink::RegistrationInfo);

#[cfg(debug_assertions)]
inventory::collect!(CollectedRegistration);

pub use mir::{CallEvidence, CallRelation, MirCall, MirGraph, MirLocal, UnifiedCallGraph};
pub use nichlink::{
    CallEdge, CallSite, CallTrace, DataEdge, DataHop, EvidenceKind, LocalId, LocalKind, LocalValue,
    NodeId, Observation, SourceLocation,
};
