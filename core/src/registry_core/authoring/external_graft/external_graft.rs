//! External graft-plan authoring.
//! 外部 graft 计划创作。

#[path = "plan.rs"]
mod plan;

pub use plan::{ExternalGraftPlanFile, create_external_graft};
