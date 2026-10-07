//! Control 对直接子对象的最低结构要求。
//! Control's minimum structure for a direct child.

use crate::RegistrationRule;

pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()
    .require_exports(&["control.render"])
    .require_handle_traits(&["ControlHandle"]);
