//! Named indices of the registration-face authoring layout.
//! 注册面创作布局的具名下标。
//!
//! The authoring layout is the slot space the Studio add/edit form and the file
//! authoring API share. It is deliberately not the macro-key order
//! (`declaration::registration::FACE_FIELD_ORDER`): a slot may carry a value the
//! macro never sees (`parent`, `module`), and the layout holds exactly the
//! fields a face can still be authored with, so every slot has both a writer and
//! a reader. Naming the slots is what lets every consumer write
//! `values[face_field::MODULE]` instead of a bare literal, which turns moving a
//! slot into a compile error instead of a silent write to the wrong place.
//! 创作布局是 Studio 新增/编辑表单与文件创作 API 共用的槽位空间。它有意不等于宏键顺序
//! （`declaration::registration::FACE_FIELD_ORDER`）：槽位可以承载宏看不到的值
//! （`parent`、`module`），而该布局正好只包含仍可创作的字段，因此每个槽位都有写入方
//! 与读取方。给槽位起名，正是让每个使用方写 `values[face_field::MODULE]` 而不是裸字面量
//! 的原因：挪动槽位会变成编译错误，而不是静默写到错误位置。
//!
//! Four slots were removed together with their fields: `params` and `handle`
//! (both equal the kind by rule) and the `expected_output`/`actual_output` pair
//! (compared only with itself; the types are now proven by `assert_contract`).
//! The layout is therefore dense, and `FACE_FIELD_COUNT` counts declared slots.
//! 有四个槽位随字段一并删除：`params` 与 `handle`（按规则都等于 kind），以及
//! `expected_output`/`actual_output` 对（只与彼此比较过；类型现由 `assert_contract`
//! 证明）。因此布局是稠密的，`FACE_FIELD_COUNT` 就是已声明槽位的数量。

/// Index of the parent Registry slot: the node that receives this face.
/// 父 Registry 槽位的下标：接收该注册面的节点。
pub const PARENT: usize = 0;
/// Index of the module-name slot: directory and file name of the face.
/// 模块名槽位的下标：注册面所在目录与文件名。
pub const MODULE: usize = 1;
/// Index of the needs-registry slot: whether the face creates a child Registry.
/// needs-registry 槽位的下标：该注册面是否创建子 Registry。
pub const NEEDS_REGISTRY: usize = 2;
/// Index of the tree-slot slot: the label this face shows in the tree.
/// 树槽位槽位的下标：该注册面在树中显示的名字。
pub const TREE_SLOT: usize = 3;
/// Index of the registry-rule slot: structure required from children.
/// registry-rule 槽位的下标：对子级要求的结构。
pub const REGISTRY_RULE: usize = 4;
/// Index of the admission slot: external branches descendants may consume.
/// admission 槽位的下标：后代可以消费的外部分支。
pub const ADMISSION: usize = 5;
/// Index of the parts slot: concrete state/parts type of the face.
/// parts 槽位的下标：注册面的具体状态/部件类型。
pub const PARTS: usize = 6;
/// Index of the exports slot: interfaces exposed at the registration seam.
/// exports 槽位的下标：在注册接缝处暴露的接口。
pub const EXPORTS: usize = 7;
/// Index of the kind slot: the Rust and registration type name.
/// kind 槽位的下标：Rust 与注册类型名。
pub const KIND: usize = 8;
/// Index of the Chinese display-name slot.
/// 中文显示名槽位的下标。
pub const NAME_ZH: usize = 9;
/// Index of the English display-name slot.
/// 英文显示名槽位的下标。
pub const NAME_EN: usize = 10;
/// Index of the Chinese summary slot.
/// 中文摘要槽位的下标。
pub const SUMMARY_ZH: usize = 11;
/// Index of the English summary slot.
/// 英文摘要槽位的下标。
pub const SUMMARY_EN: usize = 12;
/// Index of the preset slot: the contract stating which parts are expected.
/// preset 槽位的下标：声明期望哪些部件的合同。
pub const PRESET: usize = 13;
/// Index of the stable-identity slot preserved across source moves.
/// 稳定身份槽位的下标：跨源码移动保持不变。
pub const STABLE_NAME: usize = 14;
/// Index of the external-source-note slot.
/// 外部来源说明槽位的下标。
pub const GETTING_FROM_OTHER_REGISTRY: usize = 15;
/// Index of the rule-source slot: the rule file beside the face.
/// 规则来源槽位的下标：注册面旁的规则文件。
pub const REGISTRY_RULE_PATH: usize = 16;
/// Index of the handle trait label slot, derived from the contract paths.
/// handle trait 标签槽位的下标，由契约路径推导。
pub const HANDLE_TRAITS: usize = 17;
/// Index of the handle contract path slot, checked by the compiler.
/// handle 契约路径槽位的下标，由编译器检查。
pub const HANDLE_CONTRACTS: usize = 18;
/// Index of the parts trait label slot, derived from the contract paths.
/// parts trait 标签槽位的下标，由契约路径推导。
pub const PART_TRAITS: usize = 19;
/// Index of the requires slot: capabilities consumed from named providers.
/// requires 槽位的下标：从具名提供方消费的能力。
pub const REQUIRES: usize = 20;
/// Index of the provides slot: capabilities advertised to resolution.
/// provides 槽位的下标：向解析公布的能力。
pub const PROVIDES: usize = 21;
/// Index of the runtime-checks slot retained by the trace mode.
/// 运行期校验槽位的下标，按追踪模式保留。
pub const RUNTIME_CHECKS: usize = 22;
/// Index of the graft flow contract slot.
/// 嫁接数据流合同槽位的下标。
pub const FLOW: usize = 23;
/// Index of the flow provider type slot.
/// 数据流提供方类型槽位的下标。
pub const FLOW_PROVIDER: usize = 24;
/// Index of the parts contract path slot, checked by the compiler.
/// parts 契约路径槽位的下标，由编译器检查。
pub const PART_CONTRACTS: usize = 25;

/// Number of slots in the authoring layout.
/// 创作布局的槽位数量。
pub const FACE_FIELD_COUNT: usize = 26;

#[cfg(test)]
mod tests {
    use super::*;

    /// The layout is dense and its names are in slot order, which is what the
    /// `FACE_FIELD_COUNT`-sized array and the presentation table both rely on.
    /// 布局是稠密的，且名字按槽位顺序排列，这正是按 `FACE_FIELD_COUNT` 定长的数组与
    /// 展示表所依赖的性质。
    #[test]
    fn the_named_slots_are_dense_and_ordered() {
        let slots = [
            PARENT,
            MODULE,
            NEEDS_REGISTRY,
            TREE_SLOT,
            REGISTRY_RULE,
            ADMISSION,
            PARTS,
            EXPORTS,
            KIND,
            NAME_ZH,
            NAME_EN,
            SUMMARY_ZH,
            SUMMARY_EN,
            PRESET,
            STABLE_NAME,
            GETTING_FROM_OTHER_REGISTRY,
            REGISTRY_RULE_PATH,
            HANDLE_TRAITS,
            HANDLE_CONTRACTS,
            PART_TRAITS,
            REQUIRES,
            PROVIDES,
            RUNTIME_CHECKS,
            FLOW,
            FLOW_PROVIDER,
            PART_CONTRACTS,
        ];
        assert_eq!(slots.len(), FACE_FIELD_COUNT);
        for (position, slot) in slots.iter().enumerate() {
            assert_eq!(*slot, position, "slot {position} is out of order");
        }
    }
}
