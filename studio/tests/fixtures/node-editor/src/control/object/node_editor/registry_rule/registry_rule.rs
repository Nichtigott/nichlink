//! NodeEditor 对直接子对象的最低结构要求。
//! NodeEditor's minimum structure for a direct child.

use crate::RegistrationRule;

pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()
    .require_exports(&["node_editor.render"])
    .require_handle_traits(&["NodeEditorHandle"]);
