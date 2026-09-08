//! Compact registration topology retained by release applications.
//! 正式应用保留的紧凑注册拓扑。

use crate::{NodeId, RegistrationInfo, RegistrationRule};

/// One built-in face after registration checks have passed.
/// 通过注册检查后的一个内置注册面。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StaticFace {
    id: NodeId,
    parent: NodeId,
    owns_registry: bool,
}

impl StaticFace {
    pub const fn new(id: NodeId, parent: NodeId, owns_registry: bool) -> Self {
        Self {
            id,
            parent,
            owns_registry,
        }
    }

    pub const fn id(self) -> NodeId {
        self.id
    }

    pub const fn parent(self) -> NodeId {
        self.parent
    }

    pub const fn owns_registry(self) -> bool {
        self.owns_registry
    }
}

/// A zero-allocation view of the built-in registration tree.
/// 内置注册树的零分配视图。
#[derive(Clone, Copy, Debug)]
pub struct StaticPlan {
    faces: &'static [StaticFace],
}

/// One host-declared external graft cut retained in release metadata.
/// 正式构建保留的一条宿主外部 graft 切口元数据。
///
/// The table stores selectors only; it never pulls implementation code into
/// the binary. The host resolves these selectors against an external Registry
/// when it chooses to enable an overlay.
/// 表中只保存选择器，不会把实现代码拉进二进制。宿主启用覆盖层时，再将选择器
/// 解析到外部 Registry。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StaticGraftCut {
    cut: &'static str,
    graft: &'static str,
    full: bool,
}

impl StaticGraftCut {
    pub const fn new(cut: &'static str, graft: &'static str, full: bool) -> Self {
        Self { cut, graft, full }
    }

    pub const fn cut(self) -> &'static str {
        self.cut
    }

    pub const fn graft(self) -> &'static str {
        self.graft
    }

    pub const fn full(self) -> bool {
        self.full
    }
}

impl StaticPlan {
    pub const fn new(faces: &'static [StaticFace]) -> Self {
        Self { faces }
    }

    pub const fn faces(self) -> &'static [StaticFace] {
        self.faces
    }

    pub const fn len(self) -> usize {
        self.faces.len()
    }

    pub const fn is_empty(self) -> bool {
        self.faces.is_empty()
    }

    /// Find a face in the identity-sorted generated table.
    /// 在按身份排序的生成表中查找注册面。
    pub fn find(self, id: NodeId) -> Option<&'static StaticFace> {
        self.faces
            .binary_search_by_key(&id, |face| face.id)
            .ok()
            .map(|index| &self.faces[index])
    }

    pub fn children_of(self, parent: NodeId) -> impl Iterator<Item = &'static StaticFace> {
        self.faces.iter().filter(move |face| face.parent == parent)
    }
}

/// Evaluate mounting and construction rules during crate generation.
/// 在生成 crate 时求值挂载规则与构造规则。
#[doc(hidden)]
pub const fn assert_static_registration(rule: RegistrationRule, info: RegistrationInfo) {
    if let Some(required) = rule.required_preset
        && !str_eq(required, info.preset)
    {
        panic!("static registration failed: wrong preset; see declaration source");
    }
    if !contains_all(info.contract.provided_parts, rule.required_parts) {
        panic!("static registration failed: missing structural part; see declaration source");
    }
    if !contains_all(info.exports, rule.required_exports) {
        panic!("static registration failed: missing export; see declaration source");
    }
    if !contains_all(info.handle_traits, rule.required_handle_traits) {
        panic!("static registration failed: missing handle interface; see declaration source");
    }
    if !contains_all(info.part_traits, rule.required_part_traits) {
        panic!("static registration failed: missing parts interface; see declaration source");
    }
    if !contains_all(info.contract.provided_parts, info.contract.required_parts) {
        panic!(
            "static registration failed: preset parts are not satisfied; see declaration source"
        );
    }
    if !str_eq(info.contract.expected_output, info.contract.actual_output) {
        panic!("static registration failed: output contract mismatch; see declaration source");
    }
}

const fn contains_all(actual: &[&str], required: &[&str]) -> bool {
    let mut index = 0;
    while index < required.len() {
        if !has_str(actual, required[index]) {
            return false;
        }
        index += 1;
    }
    true
}

const fn has_str(values: &[&str], needle: &str) -> bool {
    let mut index = 0;
    while index < values.len() {
        if str_eq(values[index], needle) {
            return true;
        }
        index += 1;
    }
    false
}

const fn str_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}
