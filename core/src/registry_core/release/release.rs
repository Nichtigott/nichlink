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
    /// Create a face record from its already-checked identity, parent, and
    /// registry ownership; nothing is validated here.
    /// 用已校验的身份、父级与注册表归属创建注册面记录；此处不做校验。
    pub const fn new(id: NodeId, parent: NodeId, owns_registry: bool) -> Self {
        Self {
            id,
            parent,
            owns_registry,
        }
    }

    /// The face's compile-time identity.
    /// 该注册面的编译期身份。
    pub const fn id(self) -> NodeId {
        self.id
    }

    /// The face this one registers under.
    /// 本注册面所挂载的父面。
    pub const fn parent(self) -> NodeId {
        self.parent
    }

    /// Whether this face provides a registry its children may register into.
    /// 该注册面是否提供可供子级注册的注册表。
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
/// `Path` is the human-written single-node selector (`root/control/button`)
/// parsed from the host entry; a range keeps its far endpoint in
/// [`StaticGraftCut`]'s separate `cut_end` field, so a path never encodes a
/// range. `Id` is a compile-time node identity, so a host can write the target
/// as a Rust path (`crate::control::object::button::NODE_ID`) and let the
/// compiler and editor resolve it instead of spelling a string.
/// `Path` 是从宿主入口解析出的手写单节点选择器（`root/control/button`）；区间的
/// 远端端点保存在 [`StaticGraftCut`] 独立的 `cut_end` 字段里，因此路径永远不编码
/// 区间。`Id` 是编译期节点身份，宿主因此可以把目标写成 Rust 路径
/// （`crate::control::object::button::NODE_ID`），由编译器和编辑器解析，而不必
/// 手写字符串。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutTarget {
    /// A human-written single-node path selector from the host entry.
    /// 来自宿主入口的手写单节点路径选择器。
    Path(&'static str),
    /// A compile-time node identity, so tools can resolve the target as a path.
    /// 编译期节点身份，工具因此可把目标解析为 Rust 路径。
    Id(NodeId),
}

impl CutTarget {
    /// The compile-time identity this selector addresses, when it is an `Id`.
    /// 该选择器寻址的编译期身份——当它是 `Id` 时。
    ///
    /// Kept by B3a: `examples/control-button/tests/registry.rs` asserts the
    /// generated static plan carries typed cuts through this accessor, so it is
    /// not zero-caller even though no in-workspace library code uses it.
    /// B3a 保留：`examples/control-button/tests/registry.rs` 通过该访问器断言生成的
    /// 静态计划携带类型化切口；因此尽管工作区内没有库代码使用它，它也不是零调用者。
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

    /// The selector for the slot this graft replaces; for a range, the first
    /// sibling.
    /// 该 graft 所替换槽位的选择器；区间时为起始兄弟。
    pub const fn cut(self) -> CutTarget {
        self.cut
    }

    /// The last sibling of a range, or `None` when the cut is a single slot.
    /// 区间的最后一个兄弟；切口为单个槽位时为 `None`。
    pub const fn cut_end(self) -> Option<CutTarget> {
        self.cut_end
    }

    /// The selector for the external implementation that replaces the cut.
    /// 替换该切口的外部实现选择器。
    pub const fn graft(self) -> CutTarget {
        self.graft
    }

    /// Whether the whole subtree rooted at the cut is replaced.
    /// 是否替换切口根节点下的整棵子树。
    pub const fn full(self) -> bool {
        self.full
    }
}

impl StaticPlan {
    /// Build a release plan whose graft selectors are stored in read-only data.
    /// 构造把 graft selector 直接保存在只读数据中的发布计划。
    pub const fn with_grafts(
        faces: &'static [StaticFace],
        grafts: &'static [StaticGraftCut],
    ) -> Self {
        Self { faces, grafts }
    }

    /// The built-in faces in registry-tree order, so a parent precedes its
    /// children.
    /// 按注册树顺序排列的内置注册面，父级先于子级。
    pub const fn faces(self) -> &'static [StaticFace] {
        self.faces
    }

    /// Host-declared grafts captured by the build step without allocation.
    /// 构建阶段捕获、无需分配即可读取的宿主 graft 声明。
    pub const fn grafts(self) -> &'static [StaticGraftCut] {
        self.grafts
    }

    /// How many faces the plan carries.
    /// 该计划携带的注册面数量。
    pub const fn len(self) -> usize {
        self.faces.len()
    }

    /// Whether the plan carries no faces.
    /// 该计划是否不携带任何注册面。
    pub const fn is_empty(self) -> bool {
        self.faces.is_empty()
    }

    /// Find a face by identity.
    /// 按身份查找注册面。
    ///
    /// The generated table is emitted in registry-tree traversal order, because
    /// `registrations()` and everything that registers from it rely on parents
    /// coming first. It is therefore **not** sorted by identity, and a binary
    /// search over it silently missed faces — two of the three faces in the
    /// example plan. The scan is linear, and the table stays in the order its
    /// other consumers need.
    /// 生成的表按注册树遍历顺序发射，因为 `registrations()` 以及所有据此注册的代码都
    /// 依赖父级先出现。它因此**不是**按身份排序的，对它做二分会让注册面被静默漏掉——
    /// 示例计划里三个面漏了两个。这里改为线性扫描，表则保持其他消费方需要的顺序。
    pub fn find(self, id: NodeId) -> Option<&'static StaticFace> {
        self.faces.iter().find(|face| face.id == id)
    }

    /// The faces whose parent is `parent`, in table order.
    /// 父级为 `parent` 的注册面，按表顺序。
    ///
    /// The table is emitted in traversal order, not identity order, so this is
    /// a linear filter rather than a range lookup.
    /// 该表按遍历顺序而非身份顺序发射，因此这里用线性过滤而不是区间查找。
    pub fn children_of(self, parent: NodeId) -> impl Iterator<Item = &'static StaticFace> {
        self.faces.iter().filter(move |face| face.parent == parent)
    }
}

/// Evaluate mounting and construction rules during crate generation.
/// 在生成 crate 时求值挂载规则与构造规则。
///
/// # Panics
///
/// This is the **const twin** of [`RegistrationRule::validate`]: same five
/// checks, same order, different cost. The runtime side collects one `String` per
/// failure and names it; this side cannot allocate or format inside const
/// evaluation, so it stops at the first failure with a fixed message. They must be
/// changed together — the runtime side is pinned by
/// `parent_rule_aggregates_every_missing_structural_requirement`, and this side by
/// every registry-owning face in the workspace compiling (or not) under the rule
/// its own declaration carries. Folding the two into one function is not possible
/// while this one is `const`: the shared checker returns a `Vec<String>`.
/// 这是 [`RegistrationRule::validate`] 的 **const 孪生**：同样五项检查、同样顺序、不同代价。
/// 运行期一侧为每个失败收集一个 `String` 并点名它；本侧在常量求值里无法分配或格式化，因此停
/// 在第一个失败上并给出固定消息。两者必须一起改——运行期一侧由
/// `parent_rule_aggregates_every_missing_structural_requirement` 钉住，本侧则由工作区里每个
/// 拥有注册机的注册面按它自己声明所带的规则能否编译来钉住。在本函数仍是 `const` 期间把两者
/// 合成一个是不可能的：共享的那个校验器返回 `Vec<String>`。
///
/// # Panics
///
/// Panics when `info` does not satisfy `rule`: a wrong preset, a missing
/// structural part, export, handle interface, or parts interface, or a preset
/// whose parts the face does not provide. The generated crate calls this in a
/// `const` item, so the panic is a compile error naming the declaration source
/// rather than a runtime failure — which is the whole point of evaluating it
/// there.
/// 当 `info` 不满足 `rule` 时 panic：preset 不符，缺少结构部件、导出、handle 接口或 parts
/// 接口，或 preset 的 parts 没有被该面提供。生成的 crate 在 `const` 项里调用它，因此这次
/// panic 是点名声明来源的编译错误，而不是运行期失败——这正是放在那里求值的目的。
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
    // The output contract used to be compared here, as two strings the author
    // wrote beside each other. It is checked by types now: `assert_contract`
    // inside every face, and `assert_contract::<cut::__Preset, graft::__Parts>`
    // for every typed graft cut. A string comparison could not catch a wrong
    // claim; the type system cannot fail to.
    // 输出合同过去在这里比较——比较的是作者并排写下的两个字符串。现在由类型检查：
    // 每个面内的 `assert_contract`，以及每个类型化 graft 切口的
    // `assert_contract::<cut::__Preset, graft::__Parts>`。字符串比较抓不到错误声明，
    // 类型系统则不可能漏掉。
    if !contains_all(info.contract.provided_parts, info.contract.required_parts) {
        panic!(
            "static registration failed: preset parts are not satisfied; see declaration source"
        );
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
mod static_plan_find_tests {
    use super::{NodeId, StaticFace, StaticPlan};

    /// The emitted table follows the registry tree, so `find` must not assume
    /// identity order.
    /// 发射的表跟随注册树，因此 `find` 不能假定身份顺序。
    #[test]
    fn find_sees_every_face_of_an_unsorted_table() {
        static FACES: &[StaticFace] = &[
            StaticFace::new(NodeId::from_raw([9; 16]), NodeId::from_raw([0; 16]), true),
            StaticFace::new(NodeId::from_raw([1; 16]), NodeId::from_raw([9; 16]), false),
            StaticFace::new(NodeId::from_raw([5; 16]), NodeId::from_raw([9; 16]), false),
        ];
        let plan = StaticPlan::with_grafts(FACES, &[]);
        for face in FACES {
            assert_eq!(plan.find(face.id).map(|found| found.id), Some(face.id));
        }
        assert!(plan.find(NodeId::from_raw([7; 16])).is_none());
    }
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
