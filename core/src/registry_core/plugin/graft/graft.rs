//! Atomic graft commands and validation errors.
//! 原子嫁接命令与校验错误。

use super::*;
use std::fmt;

/// A graft request is intentionally data-only, so it can be staged and
/// validated before mutating the live Registry.
/// 嫁接请求只保存数据，因此可以先暂存校验，再修改线上 Registry。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraftRequest {
    pub framework: FrameworkId,
    pub target: crate::NodeId,
    pub replacement: crate::NodeId,
    pub target_contract: OwnedFlowContract,
    pub replacement_contract: OwnedFlowContract,
    pub source: PluginSource,
}

/// Current graft semantics are an atomic move into one logical slot.
/// 当前 graft 语义是原子地把候选移动到一个逻辑槽位。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraftMode {
    Move,
}

impl GraftRequest {
    pub const fn mode(&self) -> GraftMode {
        GraftMode::Move
    }
}

/// Parsed form of the Studio command `graft <replacement> to <target>`.
/// Studio 命令 `graft <replacement> to <target>` 的解析结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraftCommand {
    pub replacement: String,
    pub target: String,
}

impl GraftCommand {
    pub fn parse(command: &str) -> Result<Self, String> {
        let mut parts = command.split_whitespace();
        if parts.next() != Some("graft") {
            return Err("graft command must start with `graft`".to_owned());
        }
        let replacement = parts
            .next()
            .ok_or_else(|| "graft command is missing the replacement".to_owned())?;
        if parts.next() != Some("to") {
            return Err("graft command must use `graft <replacement> to <target>`".to_owned());
        }
        let target = parts
            .next()
            .ok_or_else(|| "graft command is missing the target".to_owned())?;
        if parts.next().is_some() {
            return Err("graft command has unexpected trailing arguments".to_owned());
        }
        Ok(Self {
            replacement: replacement.to_owned(),
            target: target.to_owned(),
        })
    }
}

impl GraftRequest {
    pub fn new<T, R>(
        framework: FrameworkId,
        target: crate::NodeId,
        replacement: crate::NodeId,
        target_contract: T,
        replacement_contract: R,
        source: PluginSource,
    ) -> Self
    where
        T: Into<OwnedFlowContract>,
        R: Into<OwnedFlowContract>,
    {
        Self {
            framework,
            target,
            replacement,
            target_contract: target_contract.into(),
            replacement_contract: replacement_contract.into(),
            source,
        }
    }
}

/// Reasons a graft was rejected before the live tree was touched.
/// 嫁接在修改线上树之前被拒绝的原因。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraftError {
    InvalidCommand(String),
    UnknownTarget(crate::NodeId),
    UnknownReplacement(crate::NodeId),
    OverlappingSubtree,
    SameNode,
    ContractUndeclared,
    ContractMismatch {
        expected: OwnedFlowContract,
        received: OwnedFlowContract,
    },
    ReplacementHasChildren,
    FrameworkMismatch(FrameworkId),
}

impl fmt::Display for GraftError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCommand(message) => formatter.write_str(message),
            Self::UnknownTarget(id) => write!(formatter, "graft target `{id}` is not registered"),
            Self::UnknownReplacement(id) => {
                write!(formatter, "graft replacement `{id}` is not registered")
            }
            Self::OverlappingSubtree => {
                formatter.write_str("graft target and replacement cannot contain each other")
            }
            Self::SameNode => {
                formatter.write_str("graft target and replacement must be different nodes")
            }
            Self::ContractUndeclared => {
                formatter.write_str("graft requires both nodes to declare a flow contract")
            }
            Self::ContractMismatch { expected, received } => write!(
                formatter,
                "graft contract mismatch: expected {} v{} ({} -> {}), received {} v{} ({} -> {})",
                expected.id,
                expected.version,
                expected.input,
                expected.output,
                received.id,
                received.version,
                received.input,
                received.output
            ),
            Self::ReplacementHasChildren => {
                formatter.write_str("graft replacement owns a non-empty child registry")
            }
            Self::FrameworkMismatch(framework) => {
                write!(formatter, "plugin does not target framework `{framework}`")
            }
        }
    }
}

impl std::error::Error for GraftError {}
