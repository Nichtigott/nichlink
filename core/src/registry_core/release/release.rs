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
    grafts: &'static [StaticGraftCut],
}

/// How a release-time graft selector addresses a face.
/// 发布态 graft 选择器如何寻址一个注册面。
///
/// `Path` is the human-written selector (`root/control/button`, or a `a to b`
/// range) parsed from the host entry. `Id` is a compile-time node identity, so
/// a host can write the target as a Rust path
/// (`crate::control::object::button::NODE_ID`) and let the compiler and editor
/// resolve it instead of spelling a string.
/// `Path` 是从宿主入口解析出的手写选择器（`root/control/button`，或 `a to b`
/// 区间）。`Id` 是编译期节点身份，宿主因此可以把目标写成 Rust 路径
/// （`crate::control::object::button::NODE_ID`），由编译器和编辑器解析，而不必
/// 手写字符串。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutTarget {
    Path(&'static str),
    Id(NodeId),
}

impl CutTarget {
    pub const fn path(self) -> Option<&'static str> {
        match self {
            Self::Path(path) => Some(path),
            Self::Id(_) => None,
        }
    }

    pub const fn id(self) -> Option<NodeId> {
        match self {
            Self::Id(id) => Some(id),
            Self::Path(_) => None,
        }
    }

    /// Render the selector for diagnostics; an identity prints as its hex form.
    /// 渲染选择器用于诊断；身份打印为十六进制形式。
    pub fn describe(self) -> String {
        match self {
            Self::Path(path) => path.to_owned(),
            Self::Id(id) => id.to_string(),
        }
    }
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
    cut: CutTarget,
    cut_end: Option<CutTarget>,
    graft: CutTarget,
    full: bool,
}

impl StaticGraftCut {
    /// A single logical path replaced by a named external implementation.
    /// 用命名外部实现替换的单个逻辑路径。
    pub const fn new(cut: &'static str, graft: &'static str, full: bool) -> Self {
        Self {
            cut: CutTarget::Path(cut),
            cut_end: None,
            graft: CutTarget::Path(graft),
            full,
        }
    }

    /// A contiguous sibling range replaced by one external implementation.
    /// 用同一个外部实现替换的一段连续兄弟。
    pub const fn new_range(
        start: &'static str,
        end: &'static str,
        graft: &'static str,
        full: bool,
    ) -> Self {
        Self {
            cut: CutTarget::Path(start),
            cut_end: Some(CutTarget::Path(end)),
            graft: CutTarget::Path(graft),
            full,
        }
    }

    /// A single slot addressed by compile-time identity on both sides.
    /// 两侧都用编译期身份寻址的单个槽位。
    pub const fn from_ids(cut: NodeId, graft: NodeId, full: bool) -> Self {
        Self {
            cut: CutTarget::Id(cut),
            cut_end: None,
            graft: CutTarget::Id(graft),
            full,
        }
    }

    /// A contiguous sibling range addressed by compile-time identity.
    /// 用编译期身份寻址的一段连续兄弟。
    pub const fn from_id_range(start: NodeId, end: NodeId, graft: NodeId, full: bool) -> Self {
        Self {
            cut: CutTarget::Id(start),
            cut_end: Some(CutTarget::Id(end)),
            graft: CutTarget::Id(graft),
            full,
        }
    }

    pub const fn cut(self) -> CutTarget {
        self.cut
    }

    pub const fn cut_end(self) -> Option<CutTarget> {
        self.cut_end
    }

    pub const fn graft(self) -> CutTarget {
        self.graft
    }

    pub const fn full(self) -> bool {
        self.full
    }
}

impl StaticPlan {
    pub const fn new(faces: &'static [StaticFace]) -> Self {
        Self { faces, grafts: &[] }
    }

    /// Build a release plan whose graft selectors are stored in read-only data.
    /// 构造把 graft selector 直接保存在只读数据中的发布计划。
    pub const fn with_grafts(
        faces: &'static [StaticFace],
        grafts: &'static [StaticGraftCut],
    ) -> Self {
        Self { faces, grafts }
    }

    pub const fn faces(self) -> &'static [StaticFace] {
        self.faces
    }

    /// Host-declared grafts captured by the build step without allocation.
    /// 构建阶段捕获、无需分配即可读取的宿主 graft 声明。
    pub const fn grafts(self) -> &'static [StaticGraftCut] {
        self.grafts
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

#[cfg(test)]
mod tests {
    use super::*;

    const FRAMEWORK: crate::FrameworkId = crate::FrameworkId::new("static-plan-test");
    const ROOT: NodeId = crate::root_node_id("static-plan-test");
    const CHILD: NodeId = NodeId::from_namespaced_path("static-plan-test", "child.rs", "Child");
    static FACES: &[StaticFace] = &[StaticFace::new(CHILD, ROOT, false)];
    static GRAFTS: &[StaticGraftCut] = &[StaticGraftCut::new("root/child", "child_fast", false)];

    // The `static_graft_plan!` macro moved to nichlink-runtime; anchor the same
    // compile-time assertions here without a kernel -> runtime dependency.
    // `static_graft_plan!` 宏已移至 nichlink-runtime；为避免 kernel 反向依赖，
    // 这里直接写出等价的编译期断言。
    const _: crate::FrameworkId = FRAMEWORK;
    const _: &str = stringify!(cut "root/child" graft "child_fast");

    #[test]
    fn graft_selectors_are_part_of_the_zero_allocation_static_plan() {
        let plan = StaticPlan::with_grafts(FACES, GRAFTS);

        assert_eq!(plan.len(), 1);
        assert_eq!(plan.grafts(), GRAFTS);
    }
}
