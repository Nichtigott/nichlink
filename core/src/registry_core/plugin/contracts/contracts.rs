//! Flow contracts and plugin records carried by registration declarations.
//! 注册声明携带的数据流合同与插件记录。
//!
//! These are pure protocol nouns. They live in the plugin module because that
//! is what they describe; `declaration` re-exports them so a declaration can
//! keep naming them without a second copy in the tree.
//! 这些是纯协议名词。它们住在 plugin 模块，因为描述的正是插件；`declaration`
//! 再导出它们，让声明继续以原名引用，而不在树里出现第二份副本。

#[path = "signing/signing.rs"]
mod signing;

use std::fmt;

/// A stable host identity. Package names are not enough when several
/// frameworks share one process.
/// 稳定的宿主身份。同一进程存在多个框架时，crate 名称并不足以区分目标。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FrameworkId(pub &'static str);

impl FrameworkId {
    /// Wrap a host identity; the text is used verbatim for equality and display.
    /// 包装宿主身份；该文本原样用于相等比较与展示。
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
    /// The undeclared slot id, spelled as the empty string.
    /// 未声明的插槽 id，即空字符串。
    pub const NONE: Self = Self("");

    /// Wrap a slot name; an empty string means no slot was declared.
    /// 包装插槽名；空字符串表示没有声明插槽。
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
    /// Slot this contract describes.
    /// 本合同描述的插槽。
    pub id: ContractId,
    /// Contract version; it must match exactly for either comparison to pass.
    /// 合同版本；两种比较都要求它完全一致。
    pub version: u32,
    /// Declared type label of the contract input.
    /// 合同输入端声明的类型标签。
    pub input: &'static str,
    /// Declared type label of the contract output.
    /// 合同输出端声明的类型标签。
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
    /// Target slot; the owned snapshot's twin of `FlowContract::id`.
    /// 目标插槽；owned 快照中对应 `FlowContract::id` 的一侧。
    pub id: String,
    /// Contract version; the owned twin of `FlowContract::version`.
    /// 合同版本；`FlowContract::version` 的 owned 孪生。
    pub version: u32,
    /// Declared input label; the owned twin of `FlowContract::input`.
    /// 声明的输入标签；`FlowContract::input` 的 owned 孪生。
    pub input: String,
    /// Declared output label; the owned twin of `FlowContract::output`.
    /// 声明的输出标签；`FlowContract::output` 的 owned 孪生。
    pub output: String,
}

impl OwnedFlowContract {
    /// The undeclared owned contract: empty strings and version zero.
    /// 未声明的 owned 合同：字符串为空、版本为 0。
    pub fn none() -> Self {
        Self {
            id: String::new(),
            version: 0,
            input: String::new(),
            output: String::new(),
        }
    }

    /// Whether this owned contract carries a slot id.
    /// 本 owned 合同是否带有插槽 id。
    pub fn is_declared(&self) -> bool {
        flow_is_declared(&self.id)
    }

    /// Whether the two owned contracts agree literally or after known domains compare.
    /// 两份 owned 合同是字面一致，还是在已知语义域比较后相容。
    pub fn semantically_compatible_with(&self, expected: &Self) -> bool {
        let left = FlowFields::from_owned(self);
        let right = FlowFields::from_owned(expected);
        flow_fields_equal(left, right) || flow_fields_semantically_compatible(left, right)
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
    /// Label no table entry knows; only literal equality is meaningful.
    /// 语义表不认识的标签；只有字面相等才有意义。
    Unknown,
    /// Coordinates relative to the parent node.
    /// 相对于父节点的坐标。
    LocalCoordinates,
    /// Coordinates in the host's absolute space.
    /// 宿主绝对空间中的坐标。
    AbsoluteCoordinates,
    /// Device-independent pixel units.
    /// 设备无关的像素单位。
    LogicalPixels,
    /// Device pixel units.
    /// 设备像素单位。
    PhysicalPixels,
}

impl FlowContract {
    /// The undeclared static contract: empty id, version 0, empty labels.
    /// 未声明的编译期合同：id 为空、版本为 0、标签为空。
    pub const NONE: Self = Self {
        id: ContractId::NONE,
        version: 0,
        input: "",
        output: "",
    };

    /// Build a declared contract from its slot, version, and two type labels.
    /// 由插槽、版本与两个类型标签构造一份已声明合同。
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

    /// Whether a slot id was declared; the compiled twin of the owned check.
    /// 是否声明了插槽 id；owned 检查的编译期孪生。
    pub const fn is_declared(self) -> bool {
        flow_is_declared(self.id.0)
    }

    /// Whether the two contracts are literally the same wire contract.
    /// 两份合同字面上是否是同一个线上合同。
    ///
    /// This is the literal half of the comparison pair. Spelling differences in
    /// known semantic domains are handled by
    /// [`FlowContract::semantically_compatible_with`], not here, so a caller
    /// that means "same domain" must call that method.
    /// 这是比较对中的字面一半。已知语义域里的拼写差异由
    /// [`FlowContract::semantically_compatible_with`] 处理，不在本方法；因此想要
    /// "同一语义域"的调用方必须调用那个方法。
    pub fn compatible_with(self, expected: Self) -> bool {
        flow_fields_equal(
            FlowFields::from_declared(self),
            FlowFields::from_declared(expected),
        )
    }

    /// Normalized semantic domain of the declared input label.
    /// 声明的输入标签所归入的语义域。
    pub fn input_semantic(self) -> FlowSemantic {
        flow_semantic(self.input)
    }

    /// Normalized semantic domain of the declared output label.
    /// 声明的输出标签所归入的语义域。
    pub fn output_semantic(self) -> FlowSemantic {
        flow_semantic(self.output)
    }

    /// Compare both the wire type and its known semantic domain.
    /// 同时比较线上的类型名称和已知语义域。
    pub fn semantically_compatible_with(self, expected: Self) -> bool {
        let left = FlowFields::from_declared(self);
        let right = FlowFields::from_declared(expected);
        flow_fields_equal(left, right) || flow_fields_semantically_compatible(left, right)
    }
}

/// The four fields every flow comparison reads, borrowed from either twin.
/// 每次 flow 比较都会读取的四个字段，从任一孪生借用而来。
///
/// `FlowContract` and `OwnedFlowContract` used to compare these fields in two
/// separate bodies, so the two *pairs* of entry points could drift apart on, say,
/// whether the version participates. Borrowing the fields once keeps each pair on
/// one comparison: the literal pair is `FlowContract::compatible_with` and
/// `OwnedFlowContract`'s `PartialEq`, and the semantic pair is both
/// `semantically_compatible_with` methods. `compatible_with` deliberately stays
/// literal-only — it is not the compiled spelling of the semantic entry point —
/// and `static_and_owned_flow_contracts_compare_identically` pins each pairing
/// separately so the distinction cannot hide behind the other.
/// `FlowContract` 与 `OwnedFlowContract` 过去在两个各自的方法体里比较这些字段，因此
/// 两*对*入口可能在"版本号是否参与比较"这类点上悄悄分叉。只借用一次字段，每一对就共用
/// 一套比较：字面对是 `FlowContract::compatible_with` 与 `OwnedFlowContract` 的
/// `PartialEq`，语义对是两个 `semantically_compatible_with`。`compatible_with` 有意
/// 只做字面比较——它不是语义入口的编译期写法——而
/// `static_and_owned_flow_contracts_compare_identically` 分别钉住两对，使这一区分无法
/// 借另一对藏起来。
#[derive(Clone, Copy)]
struct FlowFields<'a> {
    id: &'a str,
    version: u32,
    input: &'a str,
    output: &'a str,
}

impl<'a> FlowFields<'a> {
    fn from_declared(contract: FlowContract) -> Self {
        Self {
            id: contract.id.0,
            version: contract.version,
            input: contract.input,
            output: contract.output,
        }
    }

    fn from_owned(contract: &'a OwnedFlowContract) -> Self {
        Self {
            id: &contract.id,
            version: contract.version,
            input: &contract.input,
            output: &contract.output,
        }
    }
}

/// Whether a flow identity was declared at all.
/// 是否声明了数据流身份。
///
/// `FlowContract` compares its `ContractId` newtype while `OwnedFlowContract`
/// compares an owned `String`, so "is this contract declared?" was spelled
/// `!self.id.0.is_empty()` in one place and `!self.id.is_empty()` in another.
/// Both ask the same question about the same text; one const function answers it
/// and keeps the compiled twin usable in a const context.
/// `static_and_owned_flow_contracts_compare_identically` pins the two answers.
/// `FlowContract` 比较的是 `ContractId` newtype，`OwnedFlowContract` 比较的是自有
/// `String`，因此"该合同是否已声明"一处写成 `!self.id.0.is_empty()`，另一处写成
/// `!self.id.is_empty()`。两者问的是同一段文本的同一个问题；用一个 const 函数回答，
/// 编译期孪生也仍可用于 const 环境。
/// `static_and_owned_flow_contracts_compare_identically` 把两个答案互钉。
const fn flow_is_declared(id: &str) -> bool {
    !id.is_empty()
}

/// Whether two flow field sets are literally the same contract.
/// 两组 flow 字段是否字面上就是同一份合同。
fn flow_fields_equal(left: FlowFields<'_>, right: FlowFields<'_>) -> bool {
    left.id == right.id
        && left.version == right.version
        && left.input == right.input
        && left.output == right.output
}

/// Whether two flow field sets still agree after known semantic domains compare.
/// 已知语义域参与比较后，两组 flow 字段是否仍然相容。
///
/// The identity and version must match exactly; only the two labels are allowed
/// to be respelled through [`labels_compatible`], which is the semantic-label
/// core shared here. The compiled twin reaches this through
/// `semantically_compatible_with` — *not* through the literal-only
/// `compatible_with` — while the owned twin used to repeat the whole conjunction
/// inline, so a fourth term could be added to one and not the other.
/// 身份与版本必须完全一致；只有两个标签允许经 [`labels_compatible`]（此处的语义标签
/// 核）换一种拼写。编译期孪生经 `semantically_compatible_with`——而**不是**只做字面
/// 比较的 `compatible_with`——走到这里，而 owned 孪生过去把整个合取式内联重写了一遍，
/// 因此某一侧多出一个条件时另一侧不会跟着变。
fn flow_fields_semantically_compatible(left: FlowFields<'_>, right: FlowFields<'_>) -> bool {
    left.id == right.id
        && left.version == right.version
        && labels_compatible(left.input, right.input)
        && labels_compatible(left.output, right.output)
}

/// Whether two declared type labels may stand for the same domain.
/// 两个声明的类型标签是否可能指同一个语义域。
///
/// A label the table below does not know carries no domain information, so it
/// can only agree with itself, literally. Treating two *different* unknown
/// labels as compatible is exactly what let a type-incompatible replacement
/// occupy a slot while every check reported success.
/// 下表不认识的标签不携带任何语义域信息，因此只能与自身字面相等才算一致。把两个
/// *不同*的未知标签当作兼容，正是类型不兼容的替换件得以占位、而所有检查都报成功的
/// 原因。
fn labels_compatible(left: &str, right: &str) -> bool {
    match (flow_semantic(left), flow_semantic(right)) {
        (FlowSemantic::Unknown, FlowSemantic::Unknown) => left == right,
        (FlowSemantic::Unknown, _) | (_, FlowSemantic::Unknown) => false,
        (left, right) => left == right,
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
    /// The one contract this face declares, exposed as a compile-time constant.
    /// 本面声明的那一份合同，以编译期常量形式暴露。
    const FLOW_CONTRACT: FlowContract;
}

/// Whether a plugin adds a new capability or replaces an existing slot.
/// 插件是增加能力还是替换已有插槽。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginMode {
    /// Adds a new face without removing any existing one.
    /// 增加新面，不移除任何已有面。
    Extension,
    /// Takes over the slot occupied by an existing face.
    /// 接管已有面所占的插槽。
    Replacement,
}

/// Where the plugin came from. Trust policy is deliberately separate from
/// registration structure and flow compatibility.
/// 插件来源。信任策略与注册结构、数据流兼容性刻意分离。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginSource {
    /// Published and signed by the official distribution.
    /// 由官方分发渠道发布并签名。
    Official,
    /// Supplied by a user or a third party.
    /// 由用户或第三方提供。
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
    /// Manifest name; trust and lock checks match on this package identity.
    /// manifest 名称；信任与锁检查以它作为包身份匹配。
    pub name: &'static str,
    /// Rust crate that carries the plugin implementation.
    /// 承载插件实现的 Rust crate。
    pub crate_name: &'static str,
    /// Declared plugin version.
    /// 声明的插件版本。
    pub version: &'static str,
    /// Host framework the plugin targets.
    /// 插件针对的宿主框架。
    pub framework: FrameworkId,
    /// Trust lane the plugin came from.
    /// 插件的来源信任通道。
    pub source: PluginSource,
    /// Whether the plugin extends or replaces a slot.
    /// 插件是扩展还是替换插槽。
    pub mode: PluginMode,
    /// Digest of the plugin bytes, optionally carrying a `sha256:` prefix.
    /// 插件字节的摘要，可带 `sha256:` 前缀。
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
    /// Whether the manifest names exactly this framework.
    /// manifest 指明的框架是否就是这一个。
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
    ///
    /// The payload covers the manifest's own fields, the plugin bytes, **and**
    /// every field of the `registration` those bytes are claimed to produce. A
    /// signature over only the first two left the registration unauthenticated:
    /// a tampered `parent` or `flow` still verified as `Signature` and could
    /// pass a slot contract check the honest artifact failed. The manifest's
    /// `signature` field is excluded, because a signature cannot cover the bytes
    /// that carry it.
    /// 载荷覆盖 manifest 自身字段、插件字节，**以及**这些字节声称产出的 `registration` 的每个
    /// 字段。只覆盖前两者的签名让注册声明完全未被认证：被篡改的 `parent` 或 `flow` 仍会验证为
    /// `Signature`，甚至能通过诚实工件通不过的槽位合同检查。manifest 的 `signature` 字段被排除，
    /// 因为签名无法覆盖承载它的那段字节。
    pub fn signing_payload(self, registration: &crate::RegistrationInfo, bytes: &[u8]) -> Vec<u8> {
        let mut payload = Vec::with_capacity(bytes.len() + 512);
        signing::manifest(&mut payload, self);
        signing::registration(registration, &mut payload);
        payload.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        payload.extend_from_slice(bytes);
        payload
    }
}

#[cfg(test)]
mod flow_label_tests {
    use super::{ContractId, FlowContract, OwnedFlowContract};

    /// An output label the table does not know must not be smuggled past the
    /// gate by comparing two unknowns with each other.
    /// 语义表不认识的输出标签，不得靠"两个未知标签互相比较"混过关口。
    #[test]
    fn unknown_output_labels_must_agree_literally() {
        let target = FlowContract::new(
            ContractId::new("render.v1"),
            1,
            "LocalCoordinates",
            "CanvasFrame",
        );
        let different = FlowContract::new(
            ContractId::new("render.v1"),
            1,
            "LocalCoordinates",
            "TotallyDifferentType",
        );
        assert!(!target.semantically_compatible_with(different));
        assert!(target.semantically_compatible_with(target));
    }

    /// Spelling differences are still authorised where the table knows the
    /// domain, as long as the unknown side agrees literally.
    /// 语义表认识的域仍允许拼写差异，只要未知的那一侧字面一致。
    #[test]
    fn known_domains_keep_accepting_spelling_differences() {
        let target = FlowContract::new(
            ContractId::new("render.v1"),
            1,
            "LocalCoordinates",
            "CanvasFrame",
        );
        let respelled = FlowContract::new(
            ContractId::new("render.v1"),
            1,
            "local_coordinates",
            "CanvasFrame",
        );
        assert!(target.semantically_compatible_with(respelled));
        let respelled_with_other_output = FlowContract::new(
            ContractId::new("render.v1"),
            1,
            "local_coordinates",
            "TotallyDifferentType",
        );
        assert!(!target.semantically_compatible_with(respelled_with_other_output));
    }

    /// The compiled and owned twins share one field comparison and one
    /// semantic-label core, so each pair must get the same answer across the
    /// matrix: the literal pair (`compatible_with` ↔ owned `==`) and the semantic
    /// pair (both `semantically_compatible_with`).
    /// 编译期与 owned 孪生共用一套字段比较与一个语义标签核，因此每一对在矩阵上都必须
    /// 得到相同答案：字面对（`compatible_with` ↔ owned `==`）与语义对（两个
    /// `semantically_compatible_with`）。
    #[test]
    fn static_and_owned_flow_contracts_compare_identically() {
        let cases = [
            ("render.v1", 1, "LocalCoordinates", "CanvasFrame"),
            ("render.v1", 1, "local_coordinates", "CanvasFrame"),
            ("render.v1", 1, "LocalCoordinates", "TotallyDifferentType"),
            ("render.v1", 2, "LocalCoordinates", "CanvasFrame"),
            ("render.v2", 1, "LocalCoordinates", "CanvasFrame"),
            ("", 0, "", ""),
        ];
        for &(id, version, input, output) in &cases {
            for &(other_id, other_version, other_input, other_output) in &cases {
                let left = FlowContract::new(ContractId::new(id), version, input, output);
                let right = FlowContract::new(
                    ContractId::new(other_id),
                    other_version,
                    other_input,
                    other_output,
                );
                let owned_left = OwnedFlowContract::from(left);
                let owned_right = OwnedFlowContract::from(right);
                assert_eq!(
                    left.is_declared(),
                    owned_left.is_declared(),
                    "is_declared disagrees for {left:?}"
                );
                assert_eq!(
                    left.compatible_with(right),
                    owned_left == owned_right,
                    "literal comparison disagrees for {left:?} vs {right:?}"
                );
                assert_eq!(
                    left.semantically_compatible_with(right),
                    owned_left.semantically_compatible_with(&owned_right),
                    "semantic comparison disagrees for {left:?} vs {right:?}"
                );
            }
        }
    }

    /// F3 pin: `compatible_with` is literal-only, and the semantic entry point
    /// asks a different question. `LocalCoordinates` versus `local_coordinates`
    /// is the counterexample — the literal pair rejects the pair, the semantic
    /// pair accepts it. The matrix above cannot expose this divergence because
    /// it pairs each entry point with its true counterpart, so this test states
    /// the divergence and both pairings explicitly.
    /// F3 钉：`compatible_with` 只做字面比较，语义入口问的是另一个问题。
    /// `LocalCoordinates` 与 `local_coordinates` 就是反例——字面对拒绝这一对，语义对
    /// 接受。上面的矩阵无法暴露这处分歧，因为它把每个入口与它真正的对应物配对，因此本
    /// 测试把这处分歧与两种配对都显式写出。
    #[test]
    fn compatible_with_stays_literal_while_the_semantic_entry_point_folds_spellings() {
        let literal = FlowContract::new(
            ContractId::new("render.v1"),
            1,
            "LocalCoordinates",
            "CanvasFrame",
        );
        let respelled = FlowContract::new(
            ContractId::new("render.v1"),
            1,
            "local_coordinates",
            "CanvasFrame",
        );
        assert!(
            !literal.compatible_with(respelled),
            "compatible_with must stay a literal comparison"
        );
        assert!(
            literal.semantically_compatible_with(respelled),
            "the semantic entry point folds known-domain spellings"
        );
        assert_eq!(
            literal.compatible_with(respelled),
            OwnedFlowContract::from(literal) == OwnedFlowContract::from(respelled)
        );
        assert_eq!(
            literal.semantically_compatible_with(respelled),
            OwnedFlowContract::from(literal)
                .semantically_compatible_with(&OwnedFlowContract::from(respelled))
        );
    }
}
