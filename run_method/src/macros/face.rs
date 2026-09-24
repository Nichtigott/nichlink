//! Registration-face macro ladder and the editor field mirror.
//! 注册面宏阶梯与编辑器字段镜像。

#[path = "face_external.rs"]
mod face_external;
#[path = "face_helpers.rs"]
mod face_helpers;
#[path = "face_objects.rs"]
mod face_objects;
#[path = "face_registration.rs"]
mod face_registration;

/// The field vocabulary of the authoring macros, as real Rust fields.
/// 作者侧宏的字段词表，以真实 Rust 字段表示。
///
/// Nothing constructs this type; a face macro splices the author's own tokens
/// into a literal of it under `cfg(rust_analyzer)` so an editor can complete the
/// field names inside `crate::<name>_object! { … }`. Each field documents its
/// meaning, its default and one example.
/// 没有任何代码构造这个类型；注册面宏在 `cfg(rust_analyzer)` 下把作者的 token 拼进它的
/// 字面量，编辑器因此能在 `crate::<name>_object! { … }` 里补全字段名。每个字段都写明含义、
/// 默认值与一个示例。
//
// The mirror that fills this type is *valid, type-correct Rust*: an editor that
// turns `cfg(rust_analyzer)` on compiles it, and a mirror that only almost
// compiles shows up as errors on the author's own lines. The front end
// (`face_fields_mirror!` for generated aliases, `face_fields!` for
// `external_object!`) therefore emits a function whose annotation is `_` for a
// field the author wrote as an expression, the author's type for a field that
// names a type (`kind`, `preset`, `parts`, `handle`, `flow_provider`), and the
// rigid `__Any` parameter for everything else, with `loop {}` as the value.
// `name: { zh: "…", en: "…" }` and `requires: [a => b]` are not expressions, so
// their values are replaced; their names are what an editor needs from the
// mirror, and the runtime macro chain is what reports a wrong shape.
// 填充这个类型的镜像本身是**合法且类型正确**的 Rust：打开 `cfg(rust_analyzer)` 的编辑器
// 会编译它，而一个差一点才编译得过的镜像会在作者自己的代码行上报错。因此前端（生成的别名
// 用 `face_fields_mirror!`、`external_object!` 用 `face_fields!`）发出的函数：作者写成
// 表达式的字段用 `_` 注解；命名类型的字段（`kind`、`preset`、`parts`、`handle`、
// `flow_provider`）用作者的类型；其余字段用刚性参数 `__Any`，值写 `loop {}`。
// `name: { zh: "…", en: "…" }` 与 `requires: [a => b]` 不是表达式，因此其值被替换——编辑器
// 需要的是它们的字段名，而形状错误由运行期宏阶梯报告。
//
// The declaration order below is the order the compact macro arm accepts, and an
// editor lists fields in that order.
// 下面的声明顺序就是紧凑 arm 接受的顺序，编辑器也按这个顺序列出字段。
#[doc(hidden)]
pub struct FaceFields<
    SourceValue,
    KindValue,
    PresetValue,
    PartsValue,
    NameValue,
    SummaryValue,
    ExportsValue,
    StableNameValue,
    NeedsRegistryValue,
    ParentValue,
    GettingFromOtherRegistryValue,
    RegistryRulePathValue,
    RegistryRuleValue,
    AdmissionValue,
    HandleTraitsValue,
    HandleContractsValue,
    PartTraitsValue,
    PartContractsValue,
    RequiresValue,
    ProvidesValue,
    FlowValue,
    FlowProviderValue,
    PluginValue,
    RuntimeChecksValue,
> {
    /// External form only: the file this declaration lives in.
    /// 仅外部形式：声明所在的文件。
    /// ```ignore
    /// source: "widget/widget.rs"
    /// ```
    pub source: SourceValue,
    /// The handle-marker type this file declares. Required.
    /// 本文件声明的 handle 标记类型。必填。
    /// ```ignore
    /// kind: Widget
    /// ```
    pub kind: KindValue,
    /// Preset contract; defaults to NoPreset.
    /// preset 合同；默认 NoPreset。
    /// ```ignore
    /// preset: NoPreset
    /// ```
    pub preset: PresetValue,
    /// Parts contract; defaults to NoParts.
    /// parts 合同；默认 NoParts。
    /// ```ignore
    /// parts: NoParts
    /// ```
    pub parts: PartsValue,
    /// Display name; defaults to the kind.
    /// 显示名；默认取 kind。
    /// ```ignore
    /// name: { zh: "控件", en: "Widget" }
    /// ```
    pub name: NameValue,
    /// One-line summary; defaults to empty.
    /// 一句话摘要；默认空。
    /// ```ignore
    /// summary: { zh: "说明", en: "Summary" }
    /// ```
    pub summary: SummaryValue,
    /// Capabilities this face exports for its children.
    /// 本面向子级导出的能力。
    /// ```ignore
    /// exports: ["control.render"]
    /// ```
    pub exports: ExportsValue,
    /// Frozen logical name; omit to derive it.
    /// 固定的逻辑名；省略则自动推导。
    /// ```ignore
    /// stable_name: "widget"
    /// ```
    pub stable_name: StableNameValue,
    /// Whether this face owns a child registry.
    /// 本面是否拥有子注册机。
    /// ```ignore
    /// needs_registry: true
    /// ```
    pub needs_registry: NeedsRegistryValue,
    /// Where this face hangs. Required.
    /// 本面挂在谁下面。必填。
    /// ```ignore
    /// parent: crate::control::NODE_ID
    /// ```
    pub parent: ParentValue,
    /// `Some("name")` when the implementation comes from another registry.
    /// 实现来自另一个注册机时写 `Some("名字")`。
    /// ```ignore
    /// getting_from_other_registry: Some("engine")
    /// ```
    pub getting_from_other_registry: GettingFromOtherRegistryValue,
    /// Rule file path; defaults to the canonical sibling path.
    /// 规则文件路径；默认同目录规范路径。
    /// ```ignore
    /// registry_rule_path: "widget/registry_rule/registry_rule.rs"
    /// ```
    pub registry_rule_path: RegistryRulePathValue,
    /// Rule this face enforces on its children.
    /// 本面对子级执行的规则。
    ///
    /// A face that owns a registry (`needs_registry: true`) may omit this: the
    /// rule then resolves to the canonical one beside the face
    /// (`super::registry_rule::REGISTRATION_RULE`). Every other face keeps
    /// `RegistrationRule::ANY` and accepts its parent's rule instead.
    /// 拥有注册机的面（`needs_registry: true`）可以省略本字段：规则会解析到注册面旁边
    /// 那份规范规则（`super::registry_rule::REGISTRATION_RULE`）。其余面保留
    /// `RegistrationRule::ANY`，改为接受父级规则。
    /// ```ignore
    /// registry_rule: crate::widget::registry_rule::REGISTRATION_RULE
    /// ```
    pub registry_rule: RegistryRuleValue,
    /// Which paths this face may reach.
    /// 本面允许访问哪些路径。
    /// ```ignore
    /// admission: crate::Admission::new(&["control.*"], &[])
    /// ```
    pub admission: AdmissionValue,
    /// Trait labels the handle promises, for search.
    /// handle 承诺的 trait 标签，供检索。
    /// ```ignore
    /// handle_traits: ["ControlHandle"]
    /// ```
    pub handle_traits: HandleTraitsValue,
    /// Compile-time contracts the handle must satisfy.
    /// handle 必须满足的编译期契约。
    /// ```ignore
    /// handle_contracts: [crate::ControlHandle]
    /// ```
    pub handle_contracts: HandleContractsValue,
    /// Trait labels the parts promise, for search.
    /// parts 承诺的 trait 标签，供检索。
    /// ```ignore
    /// part_traits: ["ActionParts"]
    /// ```
    pub part_traits: PartTraitsValue,
    /// Compile-time contracts the parts must satisfy.
    /// parts 必须满足的编译期契约。
    /// ```ignore
    /// part_contracts: [crate::ActionParts]
    /// ```
    pub part_contracts: PartContractsValue,
    /// Capabilities this face needs, as `"cap" => "Provider"`.
    /// 本面需要的能力，写成 `"能力" => "提供者"`。
    /// ```ignore
    /// requires: ["layout.viewport" => "ControlRegistry"]
    /// ```
    pub requires: RequiresValue,
    /// Capabilities this face provides upward.
    /// 本面向上提供的能力。
    /// ```ignore
    /// provides: ["control.render"]
    /// ```
    pub provides: ProvidesValue,
    /// Data-flow contract shared with a replacement.
    /// 与替换件共享的数据流合同。
    /// ```ignore
    /// flow: FlowContract::new(ContractId::new("control.render.v1"), 1, "ControlInput", "ControlFrame")
    /// ```
    pub flow: FlowValue,
    /// Type that supplies the flow contract.
    /// 提供该数据流合同的类型。
    /// ```ignore
    /// flow_provider: crate::ControlHandle
    /// ```
    pub flow_provider: FlowProviderValue,
    /// Plugin surface this face can be replaced by.
    /// 本面可被哪个插件替换。
    /// ```ignore
    /// plugin: crate::PluginSpec::new("widget")
    /// ```
    pub plugin: PluginValue,
    /// Checks run at registration time.
    /// 注册时执行的运行时检查。
    /// ```ignore
    /// runtime_checks: []
    /// ```
    pub runtime_checks: RuntimeChecksValue,
}

#[cfg(test)]
mod tests {
    use nichlink::registry_core::declaration::FACE_FIELD_ORDER;

    /// The editor's field mirror and the order the front end sorts into must stay
    /// one vocabulary; a field added to one alone would silently stop being
    /// completed or silently stop being accepted.
    /// 编辑器用的字段镜像与前端排序依据必须是同一份词表；只往一边加字段，会让它悄悄
    /// 失去补全，或悄悄不再被接受。
    #[test]
    fn the_field_mirror_matches_the_declared_order() {
        let source = include_str!("face.rs");
        // Built at runtime so the needle cannot match this test's own source.
        // 在运行时拼接，避免这个 needle 匹配到测试自身的源码。
        let needle = ["pub struct ", "FaceFields"].concat();
        let start = source.find(&needle).expect("field mirror");
        let end = source[start..].find("\n}").expect("field mirror end") + start;
        let mirrored = source[start..end]
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                // Each field is `pub <name>: <Param>,`; only the name matters here.
                // 每个字段形如 `pub <name>: <Param>,`；这里只关心名字。
                let name = line.strip_prefix("pub ")?.split_once(": ")?.0;
                Some(name)
            })
            .collect::<Vec<_>>();
        assert_eq!(mirrored, FACE_FIELD_ORDER);
    }
}
