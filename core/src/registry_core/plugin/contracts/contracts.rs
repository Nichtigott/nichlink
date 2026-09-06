//! Plugin flow contracts.
//! 插件数据流合同。

use std::fmt;

/// A stable host identity. Package names are not enough when several
/// frameworks share one process.
/// 稳定的宿主身份。同一进程存在多个框架时，crate 名称并不足以区分目标。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FrameworkId(pub &'static str);

impl FrameworkId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }
}

impl fmt::Display for FrameworkId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// A logical replacement slot. Concrete implementations keep their own node identity;
/// the slot is the stable name that a graft targets.
/// 逻辑替换插槽。具体实现保留各自 node identity；嫁接针对的是稳定插槽名。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContractId(pub &'static str);

impl ContractId {
    pub const NONE: Self = Self("");

    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }
}

impl fmt::Display for ContractId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// The mechanically comparable part of a data-flow contract.
/// 数据流合同中可以机械比较的部分。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowContract {
    pub id: ContractId,
    pub version: u32,
    pub input: &'static str,
    pub output: &'static str,
}

/// Owned flow contract used by file-backed snapshots.
/// 文件快照使用的拥有型数据流合同。
///
/// Compiled declarations keep static strings. Reloaded source owns its
/// strings, so replacing a file does not leak old contracts.
/// 编译期声明继续使用静态字符串；热重载源码拥有自己的字符串，替换文件时不会泄漏旧合同。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedFlowContract {
    pub id: String,
    pub version: u32,
    pub input: String,
    pub output: String,
}

impl OwnedFlowContract {
    pub fn none() -> Self {
        Self {
            id: String::new(),
            version: 0,
            input: String::new(),
            output: String::new(),
        }
    }

    pub fn is_declared(&self) -> bool {
        !self.id.is_empty()
    }

    pub fn matches(&self, expected: FlowContract) -> bool {
        self.id == expected.id.0
            && self.version == expected.version
            && self.input == expected.input
            && self.output == expected.output
    }

    pub fn semantically_compatible_with(&self, expected: &Self) -> bool {
        self == expected
            || (self.id == expected.id
                && self.version == expected.version
                && semantic(&self.input) == semantic(&expected.input)
                && semantic(&self.output) == semantic(&expected.output)
                && semantic(&self.input) != FlowSemantic::Unknown)
    }
}

impl From<FlowContract> for OwnedFlowContract {
    fn from(contract: FlowContract) -> Self {
        Self {
            id: contract.id.0.to_owned(),
            version: contract.version,
            input: contract.input.to_owned(),
            output: contract.output.to_owned(),
        }
    }
}

impl fmt::Display for OwnedFlowContract {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} v{} ({} -> {})",
            self.id, self.version, self.input, self.output
        )
    }
}

/// Normalized semantic labels used by tooling when string contracts are too
/// coarse to explain a mismatch (for example local vs absolute coordinates).
/// 调试工具使用的规范化语义标签，避免仅凭字符串无法解释坐标域差异。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowSemantic {
    Unknown,
    LocalCoordinates,
    AbsoluteCoordinates,
    LogicalPixels,
    PhysicalPixels,
}

impl FlowContract {
    pub const NONE: Self = Self {
        id: ContractId::NONE,
        version: 0,
        input: "",
        output: "",
    };

    pub const fn new(
        id: ContractId,
        version: u32,
        input: &'static str,
        output: &'static str,
    ) -> Self {
        Self {
            id,
            version,
            input,
            output,
        }
    }

    pub const fn is_declared(self) -> bool {
        !self.id.0.is_empty()
    }

    pub fn compatible_with(self, expected: Self) -> bool {
        self.id == expected.id
            && self.version == expected.version
            && self.input == expected.input
            && self.output == expected.output
    }

    pub fn input_semantic(self) -> FlowSemantic {
        semantic(self.input)
    }

    pub fn output_semantic(self) -> FlowSemantic {
        semantic(self.output)
    }

    /// Compare both the wire type and its known semantic domain.
    /// 同时比较线上的类型名称和已知语义域。
    pub fn semantically_compatible_with(self, expected: Self) -> bool {
        self.compatible_with(expected)
            || (self.id == expected.id
                && self.version == expected.version
                && self.input_semantic() == expected.input_semantic()
                && self.output_semantic() == expected.output_semantic()
                && self.input_semantic() != FlowSemantic::Unknown)
    }
}

fn semantic(value: &str) -> FlowSemantic {
    if value == "LocalCoordinates" || value == "local_coordinates" {
        FlowSemantic::LocalCoordinates
    } else if value == "AbsoluteCoordinates" || value == "absolute_coordinates" {
        FlowSemantic::AbsoluteCoordinates
    } else if value == "LogicalPixels" || value == "logical_pixels" {
        FlowSemantic::LogicalPixels
    } else if value == "PhysicalPixels" || value == "physical_pixels" {
        FlowSemantic::PhysicalPixels
    } else {
        FlowSemantic::Unknown
    }
}

/// A handle can expose its data-flow contract once; registration faces then
/// read it without repeating input/output strings.
/// handle 只需声明一次数据流合同；注册面直接读取，不重复填写输入输出字符串。
pub trait FlowContractProvider {
    const FLOW_CONTRACT: FlowContract;
}
