#[path = "connector/connector.rs"]
pub mod connector;
#[path = "graft_ops/graft_ops.rs"]
pub mod graft_ops;
#[path = "index/index.rs"]
pub mod index;
#[path = "inspection/inspection.rs"]
pub mod inspection;
#[path = "metadata/metadata.rs"]
pub mod metadata;
#[path = "query/query.rs"]
pub mod query;
#[path = "transaction/transaction.rs"]
pub mod transaction;

include!("registry.rs");
