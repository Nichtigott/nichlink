//! Registration declarations and compile-time construction contracts.
//! 注册声明与编译期构造合同。

use std::fmt;

use crate::registry_core::identity::{NodeId, StableFaceId};
use crate::registry_core::runtime::RuntimeCheckSpec;

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
