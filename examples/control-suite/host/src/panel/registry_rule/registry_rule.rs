//! The rule every direct child of this face must satisfy (deliberately empty: this
//! example's requirement is the tree shape, not a contract).
//! 该面的直接子对象必须满足的规则（有意留空：本示例要求的是树的形状，不是契约）。

use crate::RegistrationRule;

pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new();
