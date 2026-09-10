//! Plugin flow contracts.
//! 插件数据流合同。
//!
//! The contract types live in the kernel crate because registration
//! declarations carry them; they are re-exported here for compatibility.
//! 合同类型定义在 kernel（注册声明持有它们），此处为兼容而重导出。

pub use nichlink::{
    ContractId, FlowContract, FlowContractProvider, FlowSemantic, FrameworkId, OwnedFlowContract,
};
