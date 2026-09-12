//! Registration declarations and compile-time construction contracts.
//! 注册声明与编译期构造合同。

use std::fmt;

use crate::registry_core::identity::{NodeId, StableFaceId};

/// File and line captured at the declaration site.
/// 在声明点捕获的文件和行号。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
    /// The registration-face handle or logical function associated with this declaration.
    /// 与该注册面声明关联的 handle 或逻辑函数名。
    pub function: &'static str,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}:{}",
            display_file(self.file),
            self.line,
            self.column
        )
    }
}

impl SourceLocation {
    /// Whether this location was synthesized by a static analyzer.
    /// 该位置是否由静态分析器合成。
    pub fn is_synthetic(self) -> bool {
        self.file.starts_with('<') && self.file.ends_with('>')
    }

    /// Render the source location with the branch-local function name.
    /// 渲染带分支函数名的声明位置。
    pub fn describe(self) -> String {
        format!(
            "{}:{}:{} function={}",
            display_file(self.file),
            self.line,
            self.column,
            self.function
        )
    }
}

impl fmt::Display for OwnedSourceLocation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}:{}",
            display_file(&self.file),
            self.line,
            self.column
        )
    }
}

impl OwnedSourceLocation {
    /// Render the source location with its logical function name.
    /// 渲染带逻辑函数名的源码位置。
    pub fn describe(&self) -> String {
        format!("{} function={}", self, self.function)
    }
}

/// `include!` expands source faces under Cargo's OUT_DIR. Keep diagnostics
/// pointed at the repository-relative face path instead of the generated copy.
/// `include!` 会把注册面展开到 Cargo 的 OUT_DIR；诊断仍显示仓库相对路径，
/// 不把用户带到生成副本。
fn display_file(file: &str) -> &str {
    file.rsplit_once("/registration_sources/")
        .map_or(file, |(_, relative)| relative)
}

/// Bilingual text kept on the registration face.
/// 注册面上的双语文本。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalizedText {
    pub zh: &'static str,
    pub en: &'static str,
}

/// One capability requirement and the object expected to provide it.
/// 一条能力需求，以及本应提供它的对象。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequirementSpec {
    pub capability: &'static str,
    pub provider: &'static str,
}

/// Dependency admission for objects produced outside the current registry tree.
/// 当前注册树对外部注册机产物的依赖门禁。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Admission {
    pub allowed_paths: &'static [&'static str],
    pub denied_paths: &'static [&'static str],
}

impl Admission {
    pub const ANY: Self = Self::new(&[], &[]);

    pub const fn new(
        allowed_paths: &'static [&'static str],
        denied_paths: &'static [&'static str],
    ) -> Self {
        Self {
            allowed_paths,
            denied_paths,
        }
    }

    pub const fn allow_paths(paths: &'static [&'static str]) -> Self {
        Self::new(paths, &[])
    }

    pub fn accepts(self, path: &str) -> bool {
        if self.denied_paths.iter().any(|prefix| {
            path == *prefix
                || path
                    .strip_prefix(prefix)
                    .is_some_and(|rest| rest.starts_with('/'))
        }) {
            return false;
        }
        self.allowed_paths.is_empty()
            || self.allowed_paths.iter().any(|prefix| {
                path == *prefix
                    || path
                        .strip_prefix(prefix)
                        .is_some_and(|rest| rest.starts_with('/'))
            })
    }
}

/// Structural rule for faces entering a Registry.
/// 注册面进入 Registry 时必须满足的结构规范。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegistrationRule {
    pub required_preset: Option<&'static str>,
    pub required_parts: &'static [&'static str],
    pub required_exports: &'static [&'static str],
    /// Interfaces the handle type must implement, for example `ControlHandle`.
    /// handle 类型必须实现的接口，例如 `ControlHandle`。
    pub required_handle_traits: &'static [&'static str],
    /// Interfaces the parts type must implement, for example `ActionParts`.
    /// parts 类型必须实现的接口，例如 `ActionParts`。
    pub required_part_traits: &'static [&'static str],
}

impl RegistrationRule {
    pub const ANY: Self = Self::new();

    /// Start an unconstrained structural rule.
    /// 创建一个尚未添加结构要求的规则。
    pub const fn new() -> Self {
        Self {
            required_preset: None,
            required_parts: &[],
            required_exports: &[],
            required_handle_traits: &[],
            required_part_traits: &[],
        }
    }

    pub const fn require_preset(mut self, preset: &'static str) -> Self {
        self.required_preset = Some(preset);
        self
    }

    pub const fn require_parts(mut self, parts: &'static [&'static str]) -> Self {
        self.required_parts = parts;
        self
    }

    pub const fn require_exports(mut self, exports: &'static [&'static str]) -> Self {
        self.required_exports = exports;
        self
    }

    pub const fn require_handle_traits(mut self, traits: &'static [&'static str]) -> Self {
        self.required_handle_traits = traits;
        self
    }

    pub const fn require_part_traits(mut self, traits: &'static [&'static str]) -> Self {
        self.required_part_traits = traits;
        self
    }

    pub fn validate(&self, info: &RegistrationInfo) -> Vec<String> {
        let mut failures = Vec::new();
        if let Some(expected) = self.required_preset
            && info.preset != expected
        {
            failures.push(format!(
                "preset `{}` is required, received `{}`",
                expected, info.preset
            ));
        }
        for required in self.required_parts {
            if !info.contract.provided_parts.contains(required) {
                failures.push(format!("required structural part `{required}` is missing"));
            }
        }
        for required in self.required_exports {
            if !info.exports.contains(required) {
                failures.push(format!("required export `{required}` is missing"));
            }
        }
        for required in self.required_handle_traits {
            if !info.handle_traits.contains(required) {
                failures.push(format!(
                    "handle `{}` must implement interface `{required}`",
                    info.handle
                ));
            }
        }
        for required in self.required_part_traits {
            if !info.part_traits.contains(required) {
                failures.push(format!(
                    "parts `{}` must implement interface `{required}`",
                    info.parts
                ));
            }
        }
        failures
    }
}

impl Default for RegistrationRule {
    fn default() -> Self {
        Self::new()
    }
}

/// A preset declares the construction shape it expects.
/// preset 声明它要求的构造形状。
pub trait PresetContract {
    type Output;
    const REQUIRED_PARTS: &'static [&'static str];
}

/// Parts declare the construction shape an object supplies.
/// parts 声明 object 实际提供的构造形状。
pub trait PartsContract {
    type Output;
    const PROVIDED_PARTS: &'static [&'static str];
}

pub struct NoPreset;
impl PresetContract for NoPreset {
    type Output = ();
    const REQUIRED_PARTS: &'static [&'static str] = &[];
}

pub struct NoParts;
impl PartsContract for NoParts {
    type Output = ();
    const PROVIDED_PARTS: &'static [&'static str] = &[];
}

/// Export list carrier for the `#[nichlink::object]` struct form.
/// `#[nichlink::object]` 结构体形态使用的 exports 列表载体。
///
/// The attribute macro generates `impl FaceExports for Kind` with the default
/// empty list. An inherent `impl Kind { pub const EXPORTS: ... }` written next
/// to the struct wins through Rust's inherent-item priority, so authors only
/// write an impl block when the face actually exports names.
/// 属性宏为 Kind 生成默认空列表的 `impl FaceExports for Kind`；
/// 结构体旁手写的固有 `impl Kind { pub const EXPORTS: ... }` 依 Rust
/// 固有项优先规则胜出，因此只有真正需要 exports 时才写 impl 块。
pub trait FaceExports {
    const EXPORTS: &'static [&'static str] = &[];
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

/// Declarative information for an ordinary object or a registry owner.
/// 普通 object 或注册机拥有者的声明式注册信息。
#[derive(Clone, Copy, Debug)]
pub struct RegistrationInfo {
    /// Package namespace that owns this declaration.
    pub namespace: &'static str,
    pub id: NodeId,
    pub parent: NodeId,
    pub kind: &'static str,
    pub preset: &'static str,
    pub parts: &'static str,
    pub params: &'static str,
    pub handle: &'static str,
    /// Optional author-owned identity that survives source moves.
    /// 可选的作者逻辑身份，可跨源码文件移动保持不变。
    pub stable_name: Option<&'static str>,
    pub name: LocalizedText,
    pub summary: LocalizedText,
    pub exports: &'static [&'static str],
    pub needs_registry: bool,
    pub registry_name: &'static str,
    pub getting_from_other_registry: Option<&'static str>,
    pub registry_rule_path: &'static str,
    /// Rule for faces entering the Registry owned by this face.
    /// 该注册面拥有的 Registry 所使用的注册规范。
    pub registry_rule: RegistrationRule,
    /// External dependency gate for this face's registry.
    /// 该注册面所属注册机对外部依赖的门禁。
    pub admission: Admission,
    pub requires: &'static [RequirementSpec],
    pub provides: &'static [&'static str],
    pub contract: ObjectContract,
    /// Automatically comparable input/output contract for grafting.
    /// 供嫁接自动比较的输入/输出合同。
    pub flow: crate::FlowContract,
    /// Type that supplied the compile-time flow contract, when explicit.
    /// 显式提供编译期数据流合同的类型路径（如果有）。
    pub flow_provider: Option<&'static str>,
    /// Interface names declared by the handle type on this registration face.
    /// 此注册面的 handle 类型声明实现的接口名称。
    pub handle_traits: &'static [&'static str],
    /// Interface names declared by the parts type on this registration face.
    /// 此注册面的 parts 类型声明实现的接口名称。
    pub part_traits: &'static [&'static str],
    pub runtime_checks: &'static [RuntimeCheckSpec],
    /// Optional provenance for a face supplied by an external plugin crate.
    /// 外部插件 crate 提供注册面时，可附带插件来源元数据。
    pub plugin: Option<crate::PluginManifest>,
    pub source: SourceLocation,
}

impl RegistrationInfo {
    /// Return the opt-in logical identity for this face.
    /// 返回该注册面的可选逻辑稳定身份。
    pub const fn stable_face_id(self) -> StableFaceId {
        let name = match self.stable_name {
            Some(name) => name,
            None => self.kind,
        };
        StableFaceId::from_name(name)
    }

    /// Return the stable identity only when the declaration opted into one.
    /// 只有声明显式选择稳定名称时才返回稳定身份。
    ///
    /// The fallback used by `stable_face_id` is useful for diagnostics, but it
    /// is not a uniqueness promise: several independent faces may all be
    /// named `Button`. Only an explicit name can be checked globally.
    /// `stable_face_id` 的回退值适合诊断，但不代表全局唯一；多个独立注册面
    /// 可以同名为 `Button`。只有显式名称才需要做全局冲突检查。
    pub const fn explicit_stable_face_id(self) -> Option<StableFaceId> {
        match self.stable_name {
            Some(name) => Some(StableFaceId::from_name(name)),
            None => None,
        }
    }
}

/// Owned registration metadata used by file-backed authoring and reloads.
/// 文件创作与热刷新使用的拥有所有权注册元数据。
///
/// Compile-time declarations keep `RegistrationInfo` as a small static value.
/// Runtime-authored values must not borrow from a process-global intern pool,
/// so this type owns every reloadable field and can be dropped with the
/// snapshot that produced it.
/// 编译期声明继续使用小型静态 `RegistrationInfo`；运行时创作值不能借用进程级
/// intern 池，因此这里拥有所有可重载字段，并随快照一起释放。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegistrationSnapshot {
    /// Package namespace that owns this declaration.
    pub namespace: String,
    pub id: NodeId,
    pub parent: NodeId,
    pub kind: String,
    pub preset: String,
    pub parts: String,
    pub params: String,
    pub handle: String,
    pub stable_name: Option<String>,
    pub name: OwnedLocalizedText,
    pub summary: OwnedLocalizedText,
    pub exports: Vec<String>,
    pub needs_registry: bool,
    pub registry_name: String,
    pub getting_from_other_registry: Option<String>,
    pub registry_rule_path: String,
    pub registry_rule: OwnedRegistrationRule,
    pub admission: OwnedAdmission,
    pub requires: Vec<OwnedRequirementSpec>,
    pub provides: Vec<String>,
    pub contract: OwnedObjectContract,
    pub flow: crate::OwnedFlowContract,
    pub flow_provider: Option<String>,
    pub handle_traits: Vec<String>,
    pub part_traits: Vec<String>,
    pub runtime_checks: Vec<RuntimeCheckSpec>,
    pub plugin: Option<crate::PluginManifest>,
    pub source: OwnedSourceLocation,
}

impl RegistrationSnapshot {
    /// Return the same logical identity used by compiled declarations.
    /// 返回与编译期声明一致的逻辑稳定身份。
    pub fn stable_face_id(&self) -> StableFaceId {
        StableFaceId::from_name(self.stable_name.as_deref().unwrap_or(&self.kind))
    }

    /// Return the explicit stable identity, if this snapshot declared one.
    /// 返回快照显式声明的稳定身份。
    pub fn explicit_stable_face_id(&self) -> Option<StableFaceId> {
        self.stable_name.as_deref().map(StableFaceId::from_name)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedLocalizedText {
    pub zh: String,
    pub en: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedRequirementSpec {
    pub capability: String,
    pub provider: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedAdmission {
    pub allowed_paths: Vec<String>,
    pub denied_paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedRegistrationRule {
    pub required_preset: Option<String>,
    pub required_parts: Vec<String>,
    pub required_exports: Vec<String>,
    pub required_handle_traits: Vec<String>,
    pub required_part_traits: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedObjectContract {
    pub required_parts: Vec<String>,
    pub provided_parts: Vec<String>,
    pub expected_output: String,
    pub actual_output: String,
}

impl OwnedAdmission {
    /// Check an external path without borrowing static declaration data.
    /// 不借用静态声明数据，直接检查外部依赖路径。
    pub fn accepts(&self, path: &str) -> bool {
        if self
            .denied_paths
            .iter()
            .any(|prefix| path_matches(path, prefix))
        {
            return false;
        }
        self.allowed_paths.is_empty()
            || self
                .allowed_paths
                .iter()
                .any(|prefix| path_matches(path, prefix))
    }
}

impl OwnedRegistrationRule {
    /// Validate all structural requirements and return every failure.
    /// 校验全部结构要求，并一次返回所有失败项。
    pub fn validate(&self, snapshot: &RegistrationSnapshot) -> Vec<String> {
        let mut failures = Vec::new();
        if let Some(expected) = &self.required_preset
            && snapshot.preset != *expected
        {
            failures.push(format!(
                "preset `{expected}` is required, received `{}`",
                snapshot.preset
            ));
        }
        for required in &self.required_parts {
            if !snapshot
                .contract
                .provided_parts
                .iter()
                .any(|part| part == required)
            {
                failures.push(format!("required structural part `{required}` is missing"));
            }
        }
        for required in &self.required_exports {
            if !snapshot.exports.iter().any(|export| export == required) {
                failures.push(format!("required export `{required}` is missing"));
            }
        }
        for required in &self.required_handle_traits {
            if !snapshot
                .handle_traits
                .iter()
                .any(|trait_name| trait_name == required)
            {
                failures.push(format!(
                    "handle `{}` must implement interface `{required}`",
                    snapshot.handle
                ));
            }
        }
        for required in &self.required_part_traits {
            if !snapshot
                .part_traits
                .iter()
                .any(|trait_name| trait_name == required)
            {
                failures.push(format!(
                    "parts `{}` must implement interface `{required}`",
                    snapshot.parts
                ));
            }
        }
        failures
    }
}

impl OwnedObjectContract {
    /// Validate the relationship between required and supplied parts.
    /// 校验所需 parts、实际提供 parts 以及返回值合同。
    pub fn validate(&self, object: &str) -> Vec<String> {
        let mut failures = Vec::new();
        for required in &self.required_parts {
            if !self
                .provided_parts
                .iter()
                .any(|provided| provided == required)
            {
                failures.push(format!(
                    "`{object}` is missing construction part `{required}`"
                ));
            }
        }
        if self.expected_output != self.actual_output {
            failures.push(format!(
                "`{object}` returns `{}`, expected `{}`",
                self.actual_output, self.expected_output
            ));
        }
        failures
    }
}

impl RegistrationSnapshot {
    /// Apply file-authored fields while retaining executable compiled metadata.
    /// 应用文件中可编辑的字段，同时保留编译产物中的可执行元数据。
    pub fn merge_authored(mut self, authored: Self) -> Self {
        self.namespace = authored.namespace;
        self.kind = authored.kind;
        self.preset = authored.preset;
        self.parts = authored.parts;
        self.params = authored.params;
        self.handle = authored.handle;
        self.stable_name = authored.stable_name;
        self.name = authored.name;
        self.summary = authored.summary;
        self.exports = authored.exports;
        self.needs_registry = authored.needs_registry;
        self.registry_name = authored.registry_name;
        self.getting_from_other_registry = authored.getting_from_other_registry;
        self.registry_rule_path = authored.registry_rule_path;
        self.registry_rule = authored.registry_rule;
        self.admission = authored.admission;
        self.requires = authored.requires;
        self.provides = authored.provides;
        // The part lists come from the compiled PresetContract and
        // PartsContract associated constants. A source-only reload cannot
        // reconstruct them, so keep that executable evidence while applying
        // the editable output labels.
        // part 列表来自已编译 trait 的关联常量；只读源码的热刷新无法可靠重建，
        // 因此保留这份可执行证据，只更新可编辑的输出标签。
        self.contract.expected_output = authored.contract.expected_output;
        self.contract.actual_output = authored.contract.actual_output;
        if authored.flow.is_declared() {
            self.flow = authored.flow;
        }
        if authored.flow_provider.is_some() {
            self.flow_provider = authored.flow_provider;
        }
        self.handle_traits = authored.handle_traits;
        self.part_traits = authored.part_traits;
        self.source = authored.source;
        self
    }
}

fn path_matches(path: &str, prefix: &str) -> bool {
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Owned declaration location used by reloadable registration faces.
/// 可重载注册面使用的自有声明位置。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedSourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub function: String,
}

impl RegistrationInfo {
    /// Copy a compiled declaration into an independently owned snapshot.
    /// 将编译期声明复制为独立拥有所有权的快照。
    pub fn into_snapshot(self) -> RegistrationSnapshot {
        RegistrationSnapshot {
            namespace: self.namespace.to_owned(),
            id: self.id,
            parent: self.parent,
            kind: self.kind.to_owned(),
            preset: self.preset.to_owned(),
            parts: self.parts.to_owned(),
            params: self.params.to_owned(),
            handle: self.handle.to_owned(),
            stable_name: self.stable_name.map(str::to_owned),
            name: OwnedLocalizedText {
                zh: self.name.zh.to_owned(),
                en: self.name.en.to_owned(),
            },
            summary: OwnedLocalizedText {
                zh: self.summary.zh.to_owned(),
                en: self.summary.en.to_owned(),
            },
            exports: self
                .exports
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            needs_registry: self.needs_registry,
            registry_name: self.registry_name.to_owned(),
            getting_from_other_registry: self.getting_from_other_registry.map(str::to_owned),
            registry_rule_path: self.registry_rule_path.to_owned(),
            registry_rule: self.registry_rule.into_owned(),
            admission: self.admission.into_owned(),
            requires: self
                .requires
                .iter()
                .map(|requirement| OwnedRequirementSpec {
                    capability: requirement.capability.to_owned(),
                    provider: requirement.provider.to_owned(),
                })
                .collect(),
            provides: self
                .provides
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            contract: self.contract.into_owned(),
            flow: self.flow.into(),
            flow_provider: self.flow_provider.map(str::to_owned),
            handle_traits: self
                .handle_traits
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            part_traits: self
                .part_traits
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            runtime_checks: self.runtime_checks.to_vec(),
            plugin: self.plugin,
            source: OwnedSourceLocation {
                file: self.source.file.to_owned(),
                line: self.source.line,
                column: self.source.column,
                function: self.source.function.to_owned(),
            },
        }
    }
}

impl Admission {
    pub fn into_owned(self) -> OwnedAdmission {
        OwnedAdmission {
            allowed_paths: self
                .allowed_paths
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            denied_paths: self
                .denied_paths
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        }
    }
}

impl RegistrationRule {
    pub fn into_owned(self) -> OwnedRegistrationRule {
        OwnedRegistrationRule {
            required_preset: self.required_preset.map(str::to_owned),
            required_parts: self
                .required_parts
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            required_exports: self
                .required_exports
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            required_handle_traits: self
                .required_handle_traits
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            required_part_traits: self
                .required_part_traits
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        }
    }
}

impl ObjectContract {
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
}

/// Runtime-readable form of the construction contract.
/// 构造合同的运行时可读形式。
#[derive(Clone, Copy, Debug)]
pub struct ObjectContract {
    pub required_parts: &'static [&'static str],
    pub provided_parts: &'static [&'static str],
    pub expected_output: &'static str,
    pub actual_output: &'static str,
}

impl ObjectContract {
    pub fn validate(&self, object: &str) -> Vec<String> {
        let mut failures = Vec::new();
        for required in self.required_parts {
            if !self
                .provided_parts
                .iter()
                .any(|provided| provided == required)
            {
                failures.push(format!(
                    "`{object}` is missing construction part `{required}`"
                ));
            }
        }
        if self.expected_output != self.actual_output {
            failures.push(format!(
                "`{object}` returns `{}`, expected `{}`",
                self.actual_output, self.expected_output
            ));
        }
        failures
    }
}

// ---------------------------------------------------------------------------
// Runtime check values and specifications.
// These types are declared here (rather than in the runtime crate) because
// `RegistrationInfo`/`RegistrationSnapshot` carry them; the runtime crate
// re-exports them at its historical paths.
// 以下类型因被 RegistrationInfo/RegistrationSnapshot 持有而定义在 kernel；
// runtime crate 会在原路径上重导出它们。

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProvenanceStep {
    pub node: NodeId,
    pub object: &'static str,
    pub operation: &'static str,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Provenance {
    pub steps: Vec<ProvenanceStep>,
}

impl Provenance {
    pub fn push(
        mut self,
        node: NodeId,
        object: &'static str,
        operation: &'static str,
        value: impl Into<String>,
    ) -> Self {
        self.steps.push(ProvenanceStep {
            node,
            object,
            operation,
            value: value.into(),
        });
        self
    }
}

#[derive(Clone, Debug)]
pub enum RuntimeValue {
    Coordinates(Coordinates),
    Number {
        value: f64,
        provenance: Provenance,
    },
    Text {
        value: String,
        provenance: Provenance,
    },
}

impl RuntimeValue {
    pub fn number(value: f64, provenance: Provenance) -> Self {
        Self::Number { value, provenance }
    }

    pub fn text(value: impl Into<String>, provenance: Provenance) -> Self {
        Self::Text {
            value: value.into(),
            provenance,
        }
    }

    pub fn provenance(&self) -> &Provenance {
        match self {
            Self::Coordinates(coordinates) => &coordinates.provenance,
            Self::Number { provenance, .. } | Self::Text { provenance, .. } => provenance,
        }
    }

    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Coordinates(_) => "coordinates",
            Self::Number { .. } => "number",
            Self::Text { .. } => "text",
        }
    }
}

/// UI coordinates with actual and expected coordinate spaces.
/// 同时携带实际坐标系和预期坐标系的 UI 坐标。
#[derive(Clone, Debug)]
pub struct Coordinates {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub coordinate_space: &'static str,
    pub expected_space: &'static str,
    pub provenance: Provenance,
}

impl Coordinates {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        viewport_width: f32,
        viewport_height: f32,
        coordinate_space: &'static str,
        expected_space: &'static str,
        provenance: Provenance,
    ) -> Self {
        Self {
            x,
            y,
            width,
            height,
            viewport_width,
            viewport_height,
            coordinate_space,
            expected_space,
            provenance,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RuntimeCheckFailure {
    pub check: &'static str,
    pub message: String,
    pub provenance: Provenance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeCheckSpec {
    CoordinatesInViewport,
    FiniteNumber,
    NumberInRange { min: i64, max: i64 },
    NonEmptyText,
    TextLength { min: usize, max: usize },
}

impl RuntimeCheckSpec {
    pub const fn name(self) -> &'static str {
        match self {
            Self::CoordinatesInViewport => "coordinates_in_viewport",
            Self::FiniteNumber => "finite_number",
            Self::NumberInRange { .. } => "number_in_range",
            Self::NonEmptyText => "non_empty_text",
            Self::TextLength { .. } => "text_length",
        }
    }

    pub fn expression(self) -> String {
        match self {
            Self::CoordinatesInViewport => "crate::COORDINATES_IN_VIEWPORT".to_owned(),
            Self::FiniteNumber => "crate::FINITE_NUMBER".to_owned(),
            Self::NumberInRange { min, max } => {
                format!("crate::RuntimeCheckSpec::number_in_range({min}, {max})")
            }
            Self::NonEmptyText => "crate::NON_EMPTY_TEXT".to_owned(),
            Self::TextLength { min, max } => {
                format!("crate::RuntimeCheckSpec::text_length({min}, {max})")
            }
        }
    }

    pub const fn number_in_range(min: i64, max: i64) -> Self {
        Self::NumberInRange { min, max }
    }

    pub const fn text_length(min: usize, max: usize) -> Self {
        Self::TextLength { min, max }
    }

    /// Parse the compact names written by the authoring form.
    /// 解析创作表单写入的紧凑检查名称。
    pub fn parse_list(value: &str) -> Result<Vec<Self>, String> {
        let value = value.trim().trim_start_matches('[').trim_end_matches(']');
        if value.is_empty() {
            return Ok(Vec::new());
        }
        split_items(value)
            .into_iter()
            .map(|item| Self::parse_one(item.trim()))
            .collect()
    }

    fn parse_one(value: &str) -> Result<Self, String> {
        let value = value.trim();
        let value = value.strip_prefix("crate::").unwrap_or(value);
        let value = value
            .strip_prefix("RuntimeCheckSpec::")
            .unwrap_or(value)
            .trim();
        match value {
            "COORDINATES_IN_VIEWPORT" | "coordinates_in_viewport" => {
                return Ok(Self::CoordinatesInViewport);
            }
            "FINITE_NUMBER" | "finite_number" => return Ok(Self::FiniteNumber),
            "NON_EMPTY_TEXT" | "non_empty_text" => return Ok(Self::NonEmptyText),
            _ => {}
        }
        if let Some(arguments) = value
            .strip_prefix("number_in_range(")
            .and_then(|rest| rest.strip_suffix(')'))
        {
            let mut values = arguments.split(',').map(str::trim);
            let min = values
                .next()
                .ok_or_else(|| "number_in_range requires min and max".to_owned())?
                .parse::<i64>()
                .map_err(|_| "number_in_range min must be an integer".to_owned())?;
            let max = values
                .next()
                .ok_or_else(|| "number_in_range requires min and max".to_owned())?
                .parse::<i64>()
                .map_err(|_| "number_in_range max must be an integer".to_owned())?;
            if values.next().is_some() {
                return Err("number_in_range accepts exactly two integers".to_owned());
            }
            return Ok(Self::NumberInRange { min, max });
        }
        if let Some(arguments) = value
            .strip_prefix("text_length(")
            .and_then(|rest| rest.strip_suffix(')'))
        {
            let mut values = arguments.split(',').map(str::trim);
            let min = values
                .next()
                .ok_or_else(|| "text_length requires min and max".to_owned())?
                .parse::<usize>()
                .map_err(|_| "text_length min must be an unsigned integer".to_owned())?;
            let max = values
                .next()
                .ok_or_else(|| "text_length requires min and max".to_owned())?
                .parse::<usize>()
                .map_err(|_| "text_length max must be an unsigned integer".to_owned())?;
            if values.next().is_some() {
                return Err("text_length accepts exactly two integers".to_owned());
            }
            return Ok(Self::TextLength { min, max });
        }
        Err(format!("unknown runtime check `{value}`"))
    }

    pub fn run(self, value: &RuntimeValue) -> Result<(), RuntimeCheckFailure> {
        match self {
            Self::CoordinatesInViewport => check_coordinates(value),
            Self::FiniteNumber => check_finite_number(value),
            Self::NumberInRange { min, max } => check_number_range(value, min, max),
            Self::NonEmptyText => check_non_empty_text(value),
            Self::TextLength { min, max } => check_text_length(value, min, max),
        }
    }
}

pub const COORDINATES_IN_VIEWPORT: RuntimeCheckSpec = RuntimeCheckSpec::CoordinatesInViewport;
pub const FINITE_NUMBER: RuntimeCheckSpec = RuntimeCheckSpec::FiniteNumber;
pub const NON_EMPTY_TEXT: RuntimeCheckSpec = RuntimeCheckSpec::NonEmptyText;

fn split_items(value: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    for (index, character) in value.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                items.push(value[start..index].trim());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    items.push(value[start..].trim());
    items
}

fn wrong_kind(check: &'static str, expected: &str, value: &RuntimeValue) -> RuntimeCheckFailure {
    RuntimeCheckFailure {
        check,
        message: format!("expected {expected} data, received {}", value.kind()),
        provenance: value.provenance().clone(),
    }
}

fn check_coordinates(value: &RuntimeValue) -> Result<(), RuntimeCheckFailure> {
    let RuntimeValue::Coordinates(coordinates) = value else {
        return Err(wrong_kind("coordinates_in_viewport", "coordinate", value));
    };
    let finite = [
        coordinates.x,
        coordinates.y,
        coordinates.width,
        coordinates.height,
        coordinates.viewport_width,
        coordinates.viewport_height,
    ]
    .into_iter()
    .all(f32::is_finite);
    if !finite {
        return Err(RuntimeCheckFailure {
            check: "coordinates_in_viewport",
            message: "coordinate contains NaN or infinity".to_owned(),
            provenance: coordinates.provenance.clone(),
        });
    }
    if coordinates.coordinate_space != coordinates.expected_space {
        return Err(RuntimeCheckFailure {
            check: "coordinates_in_viewport",
            message: format!(
                "coordinate space `{}` does not match expected `{}`",
                coordinates.coordinate_space, coordinates.expected_space
            ),
            provenance: coordinates.provenance.clone(),
        });
    }
    let inside = coordinates.x >= 0.0
        && coordinates.y >= 0.0
        && coordinates.width >= 0.0
        && coordinates.height >= 0.0
        && coordinates.x + coordinates.width <= coordinates.viewport_width
        && coordinates.y + coordinates.height <= coordinates.viewport_height;
    if inside {
        Ok(())
    } else {
        Err(RuntimeCheckFailure {
            check: "coordinates_in_viewport",
            message: format!(
                "rect ({:.1}, {:.1}, {:.1}, {:.1}) exceeds viewport ({:.1}, {:.1})",
                coordinates.x,
                coordinates.y,
                coordinates.width,
                coordinates.height,
                coordinates.viewport_width,
                coordinates.viewport_height
            ),
            provenance: coordinates.provenance.clone(),
        })
    }
}

fn check_finite_number(value: &RuntimeValue) -> Result<(), RuntimeCheckFailure> {
    let RuntimeValue::Number { value, provenance } = value else {
        return Err(wrong_kind("finite_number", "number", value));
    };
    if value.is_finite() {
        Ok(())
    } else {
        Err(RuntimeCheckFailure {
            check: "finite_number",
            message: format!("number `{value}` is NaN or infinite"),
            provenance: provenance.clone(),
        })
    }
}

fn check_number_range(value: &RuntimeValue, min: i64, max: i64) -> Result<(), RuntimeCheckFailure> {
    let RuntimeValue::Number { value, provenance } = value else {
        return Err(wrong_kind("number_in_range", "number", value));
    };
    if min > max {
        return Err(RuntimeCheckFailure {
            check: "number_in_range",
            message: format!("invalid check bounds: minimum {min} exceeds maximum {max}"),
            provenance: provenance.clone(),
        });
    }
    if value.is_finite() && *value >= min as f64 && *value <= max as f64 {
        Ok(())
    } else {
        Err(RuntimeCheckFailure {
            check: "number_in_range",
            message: format!("number `{value}` is outside inclusive range {min}..={max}"),
            provenance: provenance.clone(),
        })
    }
}

fn check_non_empty_text(value: &RuntimeValue) -> Result<(), RuntimeCheckFailure> {
    let RuntimeValue::Text { value, provenance } = value else {
        return Err(wrong_kind("non_empty_text", "text", value));
    };
    if value.trim().is_empty() {
        Err(RuntimeCheckFailure {
            check: "non_empty_text",
            message: "text is empty or whitespace only".to_owned(),
            provenance: provenance.clone(),
        })
    } else {
        Ok(())
    }
}

fn check_text_length(
    value: &RuntimeValue,
    min: usize,
    max: usize,
) -> Result<(), RuntimeCheckFailure> {
    let RuntimeValue::Text { value, provenance } = value else {
        return Err(wrong_kind("text_length", "text", value));
    };
    let length = value.chars().count();
    if min <= max && (min..=max).contains(&length) {
        Ok(())
    } else {
        Err(RuntimeCheckFailure {
            check: "text_length",
            message: if min > max {
                format!("invalid check bounds: minimum {min} exceeds maximum {max}")
            } else {
                format!("text length {length} is outside inclusive range {min}..={max}")
            },
            provenance: provenance.clone(),
        })
    }
}

/// One call edge that was observed while a `CallTrace` frame was active.
/// `CallTrace` 中实际观察到的一条调用边。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallSite {
    pub node: NodeId,
    pub function: &'static str,
    pub frame_id: u64,
    /// Stable declaration/callsite location. `None` is reserved for callers
    /// that construct a synthetic frame directly.
    /// 稳定的声明/调用位置；`None` 仅保留给直接构造合成帧的调用者。
    pub source: Option<SourceLocation>,
}

// ---------------------------------------------------------------------------
// Observed call edges and evidence. Shared by run_method tracing and
// debug_method tooling; defined here because they are pure protocol nouns.
// 已观测调用边与证据。由 run_method 追踪与 debug_method 工具共用；
// 作为纯协议名词定义在 kernel。

/// One call edge that was observed while a `CallTrace` frame was active.
/// `CallTrace` 中实际观察到的一条调用边。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallEdge {
    pub caller: CallSite,
    pub callee: CallSite,
}

/// Provenance of a relationship across runtime, MIR, and source evidence.
/// 运行时、MIR 与源码证据共用的关系来源。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceKind {
    /// Confirmed by a live `CallTrace` observation.
    Live,
    /// Candidate found by source analysis without runtime confirmation.
    Source,
    /// Candidate inferred from MIR; it may not have executed.
    Mir,
    /// Supplied by an external adapter without a local trace.
    External,
    /// A relation whose producer is not known.
    Unknown,
}

impl EvidenceKind {
    /// Compact marker used by text, DOT, and Studio renderers.
    /// 文本、DOT 与 Studio 渲染器共用的紧凑标记。
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Live => "+",
            Self::Mir => "?",
            Self::Source => "~",
            Self::External => "x",
            Self::Unknown => "!",
        }
    }

    /// Stable machine and human readable label.
    /// 稳定的机器和人类可读标签。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Mir => "mir",
            Self::Source => "source",
            Self::External => "external",
            Self::Unknown => "unknown",
        }
    }

    /// Only a live observation confirms that an edge executed.
    /// 只有 Live 观察能确认边实际执行过。
    pub const fn confirmed(self) -> bool {
        matches!(self, Self::Live)
    }
}

/// A call edge with invocation IDs removed for topology queries.
/// 去掉调用实例编号、用于拓扑查询的逻辑调用边。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicalCallEdge {
    pub caller: CallSite,
    pub callee: CallSite,
    pub evidence: EvidenceKind,
}

// ---------------------------------------------------------------------------
// Plugin manifests and flow contracts.
// These types are declared here because `RegistrationInfo`/`RegistrationSnapshot`
// carry them; the runtime plugin modules re-export them at their historical
// paths. 以下类型因被注册声明持有而定义在 kernel；runtime 插件模块在原路径重导出。

/// A stable host identity. Package names are not enough when several
/// frameworks share one process.
/// 稳定的宿主身份。同一进程存在多个框架时，crate 名称并不足以区分目标。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FrameworkId(pub &'static str);

impl FrameworkId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }
}

impl fmt::Display for FrameworkId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// A logical replacement slot. Concrete implementations keep their own node identity;
/// the slot is the stable name that a graft targets.
/// 逻辑替换插槽。具体实现保留各自 node identity；嫁接针对的是稳定插槽名。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ContractId(pub &'static str);

impl ContractId {
    pub const NONE: Self = Self("");

    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }
}

impl fmt::Display for ContractId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

/// The mechanically comparable part of a data-flow contract.
/// 数据流合同中可以机械比较的部分。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlowContract {
    pub id: ContractId,
    pub version: u32,
    pub input: &'static str,
    pub output: &'static str,
}

/// Owned flow contract used by file-backed snapshots.
/// 文件快照使用的拥有型数据流合同。
///
/// Compiled declarations keep static strings. Reloaded source owns its
/// strings, so replacing a file does not leak old contracts.
/// 编译期声明继续使用静态字符串；热重载源码拥有自己的字符串，替换文件时不会泄漏旧合同。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedFlowContract {
    pub id: String,
    pub version: u32,
    pub input: String,
    pub output: String,
}

impl OwnedFlowContract {
    pub fn none() -> Self {
        Self {
            id: String::new(),
            version: 0,
            input: String::new(),
            output: String::new(),
        }
    }

    pub fn is_declared(&self) -> bool {
        !self.id.is_empty()
    }

    pub fn matches(&self, expected: FlowContract) -> bool {
        self.id == expected.id.0
            && self.version == expected.version
            && self.input == expected.input
            && self.output == expected.output
    }

    pub fn semantically_compatible_with(&self, expected: &Self) -> bool {
        self == expected
            || (self.id == expected.id
                && self.version == expected.version
                && flow_semantic(&self.input) == flow_semantic(&expected.input)
                && flow_semantic(&self.output) == flow_semantic(&expected.output)
                && flow_semantic(&self.input) != FlowSemantic::Unknown)
    }
}

impl From<FlowContract> for OwnedFlowContract {
    fn from(contract: FlowContract) -> Self {
        Self {
            id: contract.id.0.to_owned(),
            version: contract.version,
            input: contract.input.to_owned(),
            output: contract.output.to_owned(),
        }
    }
}

impl fmt::Display for OwnedFlowContract {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} v{} ({} -> {})",
            self.id, self.version, self.input, self.output
        )
    }
}

/// Normalized semantic labels used by tooling when string contracts are too
/// coarse to explain a mismatch (for example local vs absolute coordinates).
/// 调试工具使用的规范化语义标签，避免仅凭字符串无法解释坐标域差异。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowSemantic {
    Unknown,
    LocalCoordinates,
    AbsoluteCoordinates,
    LogicalPixels,
    PhysicalPixels,
}

impl FlowContract {
    pub const NONE: Self = Self {
        id: ContractId::NONE,
        version: 0,
        input: "",
        output: "",
    };

    pub const fn new(
        id: ContractId,
        version: u32,
        input: &'static str,
        output: &'static str,
    ) -> Self {
        Self {
            id,
            version,
            input,
            output,
        }
    }

    pub const fn is_declared(self) -> bool {
        !self.id.0.is_empty()
    }

    pub fn compatible_with(self, expected: Self) -> bool {
        self.id == expected.id
            && self.version == expected.version
            && self.input == expected.input
            && self.output == expected.output
    }

    pub fn input_semantic(self) -> FlowSemantic {
        flow_semantic(self.input)
    }

    pub fn output_semantic(self) -> FlowSemantic {
        flow_semantic(self.output)
    }

    /// Compare both the wire type and its known semantic domain.
    /// 同时比较线上的类型名称和已知语义域。
    pub fn semantically_compatible_with(self, expected: Self) -> bool {
        self.compatible_with(expected)
            || (self.id == expected.id
                && self.version == expected.version
                && self.input_semantic() == expected.input_semantic()
                && self.output_semantic() == expected.output_semantic()
                && self.input_semantic() != FlowSemantic::Unknown)
    }
}

fn flow_semantic(value: &str) -> FlowSemantic {
    if value == "LocalCoordinates" || value == "local_coordinates" {
        FlowSemantic::LocalCoordinates
    } else if value == "AbsoluteCoordinates" || value == "absolute_coordinates" {
        FlowSemantic::AbsoluteCoordinates
    } else if value == "LogicalPixels" || value == "logical_pixels" {
        FlowSemantic::LogicalPixels
    } else if value == "PhysicalPixels" || value == "physical_pixels" {
        FlowSemantic::PhysicalPixels
    } else {
        FlowSemantic::Unknown
    }
}

/// A handle can expose its data-flow contract once; registration faces then
/// read it without repeating input/output strings.
/// handle 只需声明一次数据流合同；注册面直接读取，不重复填写输入输出字符串。
pub trait FlowContractProvider {
    const FLOW_CONTRACT: FlowContract;
}

/// Whether a plugin adds a new capability or replaces an existing slot.
/// 插件是增加能力还是替换已有插槽。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginMode {
    Extension,
    Replacement,
}

/// Where the plugin came from. Trust policy is deliberately separate from
/// registration structure and flow compatibility.
/// 插件来源。信任策略与注册结构、数据流兼容性刻意分离。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginSource {
    Official,
    User,
}

impl PluginSource {
    #[doc(hidden)]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "official" => Some(Self::Official),
            "user" => Some(Self::User),
            _ => None,
        }
    }
}

impl PluginMode {
    #[doc(hidden)]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "extension" => Some(Self::Extension),
            "replacement" => Some(Self::Replacement),
            _ => None,
        }
    }
}

/// Plugin manifest metadata.
/// 插件 manifest 元数据。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluginManifest {
    pub name: &'static str,
    pub crate_name: &'static str,
    pub version: &'static str,
    pub framework: FrameworkId,
    pub source: PluginSource,
    pub mode: PluginMode,
    pub checksum: &'static str,
    /// Hex-encoded Ed25519 signature over `signing_payload`, when supplied.
    /// `signing_payload` 上的十六进制 Ed25519 签名；插件没有签名时为 None。
    pub signature: Option<&'static str>,
    /// Fingerprint of the public key used for the signature.
    /// 用于签名的公钥指纹。
    pub public_key_fingerprint: Option<&'static str>,
    /// Identifier of the revocation-list snapshot used by the publisher.
    /// 发布者使用的撤销列表快照标识。
    pub revocation_list: Option<&'static str>,
}

impl PluginManifest {
    pub fn targets(self, framework: FrameworkId) -> bool {
        self.framework.0 == framework.0
    }

    /// Verify the manifest digest against plugin bytes before native loading.
    /// 在 native 加载前，用插件字节验证 manifest 摘要。
    pub fn verify_bytes(self, bytes: &[u8]) -> bool {
        let expected = self
            .checksum
            .strip_prefix("sha256:")
            .unwrap_or(self.checksum);
        expected.len() == 64 && crate::sha256_hex(bytes).eq_ignore_ascii_case(expected)
    }

    /// Build the canonical bytes covered by an official plugin signature.
    /// 构造官方插件签名覆盖的规范化字节。
    pub fn signing_payload(self, bytes: &[u8]) -> Vec<u8> {
        let source = match self.source {
            PluginSource::Official => "official",
            PluginSource::User => "user",
        };
        let mode = match self.mode {
            PluginMode::Extension => "extension",
            PluginMode::Replacement => "replacement",
        };
        let fields = [
            self.name,
            self.crate_name,
            self.version,
            self.framework.0,
            source,
            mode,
            self.checksum,
            self.public_key_fingerprint.unwrap_or(""),
            self.revocation_list.unwrap_or(""),
        ];
        let mut payload = Vec::with_capacity(bytes.len() + 128);
        for field in fields {
            payload.extend_from_slice(&(field.len() as u64).to_le_bytes());
            payload.extend_from_slice(field.as_bytes());
        }
        payload.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        payload.extend_from_slice(bytes);
        payload
    }
}

/// Runtime collection policy.
/// 运行时追踪收集策略。
///
/// `Off` is the release-safe default for runtime traces. `ErrorsOnly`
/// keeps evidence only for a failed result/panic scope, while `Full` retains
/// every observed frame, local, and data edge.
/// `Off` 是运行时追踪的发布安全默认值；`ErrorsOnly` 只保留
/// 失败结果或 panic 作用域中的证据；`Full` 保留全部帧、局部值和数据边。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TraceMode {
    /// Do not allocate or retain runtime evidence.
    /// 不分配也不保留运行时证据。
    #[default]
    Off,
    /// Retain evidence only when an error scope fails.
    /// 只有错误作用域失败时才保留证据。
    ErrorsOnly,
    /// Retain all observed runtime evidence.
    /// 保留全部观察到的运行时证据。
    Full,
}

impl TraceMode {
    /// Parses the value accepted by `NICH_LINK_TRACE`.
    /// 解析 `NICH_LINK_TRACE` 支持的值。
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" | "0" | "disabled" => Some(Self::Off),
            "errors-only" | "errors_only" | "errors" => Some(Self::ErrorsOnly),
            "full" | "all" => Some(Self::Full),
            _ => None,
        }
    }
}

#[cfg(test)]
mod trace_mode_tests {
    use super::TraceMode;

    #[test]
    fn parse_accepts_documented_spellings() {
        assert_eq!(TraceMode::parse("off"), Some(TraceMode::Off));
        assert_eq!(TraceMode::parse("ERRORS-ONLY"), Some(TraceMode::ErrorsOnly));
        assert_eq!(TraceMode::parse(" full "), Some(TraceMode::Full));
        assert_eq!(TraceMode::parse("sometimes"), None);
    }
}
