//! Cap leaf face: a child of Button, one level deeper than the grafted faces.
//! Cap 叶子面：Button 的子对象，比被 graft 的那些面再深一层。

crate::button_object! {
    kind: Cap,
    parent: crate::control::object::button::NODE_ID,
}

pub struct Cap;
