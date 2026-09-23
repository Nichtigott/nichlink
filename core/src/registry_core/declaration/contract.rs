//! Compile-time construction contracts for registration faces.
//! 注册面的编译期构造合同。
//!
//! A preset declares the parts it expects, a parts type declares what it
//! supplies, and [`assert_contract`] forces the two output types to match during
//! macro expansion. [`ObjectContract`] is the runtime-readable record of that
//! relationship.
//! preset 声明它要求的 parts，parts 类型声明它提供的 parts，[`assert_contract`] 在宏
//! 展开时强制两者的输出类型相同。[`ObjectContract`] 是这层关系的运行时可读记录。

use super::*;

/// A preset declares the construction shape it expects.
/// preset 声明它要求的构造形状。
pub trait PresetContract {
    /// Concrete type this preset constructs.
    /// 该 preset 构造出的具体类型。
    type Output;
    /// Part names that must all be supplied before the preset can construct.
    /// 该 preset 完成构造前必须提供的全部 part 名称。
    const REQUIRED_PARTS: &'static [&'static str];
}

/// Parts declare the construction shape an object supplies.
/// parts 声明 object 实际提供的构造形状。
pub trait PartsContract {
    /// Concrete type these parts construct.
    /// 这些 parts 构造出的具体类型。
    type Output;
    /// Part names this type actually supplies.
    /// 该类型实际提供的 part 名称。
    const PROVIDED_PARTS: &'static [&'static str];
}

/// Marker preset for a face that is constructed without a preset.
/// 不需要 preset 即可构造的注册面所使用的标记类型。
pub struct NoPreset;

impl PresetContract for NoPreset {
    type Output = ();
    const REQUIRED_PARTS: &'static [&'static str] = &[];
}

/// Marker parts for a face that supplies no parts.
/// 不提供任何 part 的注册面所使用的标记类型。
pub struct NoParts;

impl PartsContract for NoParts {
    type Output = ();
    const PROVIDED_PARTS: &'static [&'static str] = &[];
}

/// Force preset and parts output types to match during macro expansion.
/// 在宏展开时强制 preset 与 parts 的输出类型相同。
///
/// ```compile_fail
/// use nichlink::{assert_contract, PartsContract, PresetContract};
///
/// struct Expected;
/// struct Supplied;
/// impl PresetContract for Expected {
///     type Output = Expected;
///     const REQUIRED_PARTS: &'static [&'static str] = &[];
/// }
/// impl PartsContract for Supplied {
///     type Output = Supplied;
///     const PROVIDED_PARTS: &'static [&'static str] = &[];
/// }
///
/// const _: () = assert_contract::<Expected, Supplied>();
/// ```
pub const fn assert_contract<P, T>()
where
    P: PresetContract,
    T: PartsContract<Output = P::Output>,
{
}

impl ObjectContract {
    /// Copy this borrowed contract into the owned form a snapshot can retain.
    /// 将该借用合同复制为快照可长期持有的拥有所有权形式。
    pub fn into_owned(self) -> OwnedObjectContract {
        OwnedObjectContract {
            required_parts: self
                .required_parts
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            provided_parts: self
                .provided_parts
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            expected_output: self.expected_output.to_owned(),
            actual_output: self.actual_output.to_owned(),
        }
    }

    /// Return one message per unmet part or output requirement; an empty result
    /// means the contract holds. `object` names the face being checked.
    /// 每个未满足的 part 或输出要求各返回一条消息；结果为空表示合同成立。
    /// `object` 是被检查的注册面名称。
    pub fn validate(&self, object: &str) -> Vec<String> {
        validate_object_contract(
            self.required_parts,
            self.provided_parts,
            self.expected_output,
            self.actual_output,
            object,
        )
    }
}

/// Runtime-readable form of the construction contract.
/// 构造合同的运行时可读形式。
#[derive(Clone, Copy, Debug)]
pub struct ObjectContract {
    /// Part names the preset declared it needs.
    /// preset 声明它需要的 part 名称。
    pub required_parts: &'static [&'static str],
    /// Part names the parts type supplied.
    /// parts 类型实际提供的 part 名称。
    pub provided_parts: &'static [&'static str],
    /// Output type name the preset expects.
    /// preset 期望的输出类型名。
    pub expected_output: &'static str,
    /// Output type name the parts type actually produces.
    /// parts 类型实际产出的输出类型名。
    pub actual_output: &'static str,
}
