//! Plugin execution policy and adapter contracts.
//! 插件执行策略与适配器合同。
//!
//! `PluginMode`, `PluginSource` and `PluginManifest` live in `declaration`
//! because registration declarations carry them; this module imports them from
//! there instead of re-exporting them, so the plugin namespace does not become
//! a second home for the same item.
//! `PluginMode`、`PluginSource` 与 `PluginManifest` 住在 `declaration`——注册声明持有
//! 它们；本模块从那里导入而不是再导出，插件命名空间因此不会成为同一个 item 的第二个家。

use crate::registry_core::declaration::{FrameworkId, PluginManifest, PluginSource};

use super::*;

/// Where plugin code executes. Native code is trusted process code; the other
/// two modes are explicit isolation choices for untrusted extensions.
/// 插件代码的执行隔离方式。Native 与宿主同进程；另外两种是明确的隔离选项。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginAdapter {
    /// Native adapter: plugin code runs in the host process and is trusted.
    /// Native 适配器：插件代码在宿主进程内运行，视为可信。
    Native,
    /// Wasm adapter: plugin code runs inside a Wasm sandbox.
    /// Wasm 适配器：插件代码在 Wasm 沙箱内运行。
    Wasm,
    /// Process adapter: plugin code runs in a separate OS process.
    /// 进程适配器：插件代码在独立操作系统进程中运行。
    Process,
}

/// Generic execution backend for a plugin artifact.
/// 插件工件的泛型执行后端。
///
/// The associated instance keeps dispatch static: a native, WASM, or
/// process adapter can be selected by the host without putting every plugin
/// implementation behind one global trait object.
/// 使用关联类型保持静态分发：宿主可以选择 native、WASM 或进程适配器，
/// 不必把所有插件实现塞进一个全局 trait object。
pub trait PluginRuntime {
    /// The loaded form a caller drives once `load` has succeeded.
    /// `load` 成功后调用方驱动的已加载实例类型。
    type Instance;

    /// Why loading failed. Structured so an adapter reports its own error type
    /// instead of flattening every failure into a string.
    /// 加载失败的原因。结构化后适配器报告自己的错误类型，而不必把每种失败压平成字符串。
    type Error: std::error::Error;

    /// Report which isolation mode this runtime provides.
    /// 报告本运行时所提供的隔离方式。
    fn adapter(&self) -> PluginAdapter;

    /// Load one manifest's bytes, surfacing the adapter's own failure type.
    /// 加载某个 manifest 的字节，失败时给出适配器自己的错误类型。
    fn load(&self, manifest: PluginManifest, bytes: &[u8]) -> Result<Self::Instance, Self::Error>;
}

/// Host policy for selecting which linked plugin manifests may participate.
/// 宿主选择已链接插件是否参与的策略。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluginPolicy {
    /// Framework whose manifests this policy governs.
    /// 本策略所管辖 manifest 的目标框架。
    pub framework: FrameworkId,
    /// Whether official-sourced manifests may be selected.
    /// 是否允许选中官方来源的 manifest。
    pub allow_official: bool,
    /// Whether user-sourced manifests may be selected.
    /// 是否允许选中用户来源的 manifest。
    pub allow_user: bool,
    /// Reject manifests whose checksum is not a cryptographic digest.
    /// 拒绝不是密码学摘要的插件 manifest。
    pub require_digest: bool,
}

impl PluginPolicy {
    /// Deny every source; for hosts that link no external plugin.
    /// 拒绝所有来源；供不链接外部插件的宿主使用。
    pub const fn local(framework: FrameworkId) -> Self {
        Self {
            framework,
            allow_official: false,
            allow_user: false,
            require_digest: true,
        }
    }

    /// Admit both official and user sources without a digest requirement.
    /// 同时接受官方与用户来源，且不要求密码学摘要。
    pub const fn open(framework: FrameworkId) -> Self {
        Self {
            framework,
            allow_official: true,
            allow_user: true,
            require_digest: false,
        }
    }

    /// Open policy with cryptographic checksum enforcement enabled.
    /// 开放来源但强制校验密码学摘要的策略。
    pub const fn strict(framework: FrameworkId) -> Self {
        Self {
            framework,
            allow_official: true,
            allow_user: true,
            require_digest: true,
        }
    }

    /// Whether the manifest is admitted, collapsing the reason to a boolean.
    /// 该 manifest 是否被接纳；原因被折叠成一个布尔值。
    ///
    /// `catalog` is the host's official lock record. Pass it whenever the host
    /// has one: an official manifest is only compared against the lock when a
    /// catalog is supplied, so `None` skips that check. The predicate takes the
    /// catalog rather than assuming it away because the old form hardcoded
    /// `None`, which made `PluginRejectReason::LockMismatch` unreachable for
    /// every caller — a self-declared `Official` manifest was admitted on its
    /// own word.
    /// `catalog` 是宿主的官方锁记录；宿主手上有它就应当传入：只有提供目录时，官方 manifest 才会
    /// 与锁记录比对，`None` 会跳过该检查。本谓词把目录作为参数、而不是假称它不存在，因为旧写法
    /// 硬编码 `None`，使 `PluginRejectReason::LockMismatch` 对每个调用方都不可达——一个自称
    /// `Official` 的 manifest 仅凭自述就被接纳。
    pub fn accepts(self, plugin: PluginManifest, catalog: Option<&PluginCatalog>) -> bool {
        matches!(self.decision(plugin, catalog), PluginDecision::Accepted)
    }

    /// Explain selection instead of silently dropping a linked plugin.
    /// 返回筛选原因，不再让已链接插件静默消失。
    pub fn decision(
        self,
        plugin: PluginManifest,
        catalog: Option<&PluginCatalog>,
    ) -> PluginDecision {
        if !plugin.targets(self.framework) {
            return PluginDecision::Rejected(PluginRejectReason::FrameworkMismatch);
        }
        let source_allowed = match plugin.source {
            PluginSource::Official => self.allow_official,
            PluginSource::User => self.allow_user,
        };
        if !source_allowed {
            return PluginDecision::Rejected(PluginRejectReason::SourceDisabled);
        }
        if self.require_digest && !artifact::is_sha256(plugin.checksum) {
            return PluginDecision::Rejected(PluginRejectReason::InvalidDigest);
        }
        if self.require_digest
            && plugin.source == PluginSource::Official
            && plugin
                .signature
                .is_none_or(|signature| signature.trim().is_empty())
        {
            return PluginDecision::Rejected(PluginRejectReason::MissingSignature);
        }
        if plugin.source == PluginSource::Official
            && catalog.is_some_and(|catalog| !catalog.contains_manifest(plugin))
        {
            return PluginDecision::Rejected(PluginRejectReason::LockMismatch);
        }
        PluginDecision::Accepted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_core::declaration::{ContractId, FlowContract, PluginMode};

    #[test]
    fn flow_contracts_compare_without_runtime_hashing() {
        let expected = FlowContract::new(
            ContractId::new("render.canvas2d.v1"),
            1,
            "CanvasInput",
            "CanvasFrame",
        );
        assert!(expected.compatible_with(expected));
        assert!(!expected.compatible_with(FlowContract::new(
            ContractId::new("render.canvas2d.v1"),
            1,
            "AbsoluteInput",
            "CanvasFrame",
        )));
    }

    #[test]
    fn plugin_policy_keeps_frameworks_and_sources_separate() {
        let framework = FrameworkId::new("com.nichui.editor");
        let official = PluginManifest {
            name: "official.canvas",
            crate_name: "official_canvas",
            version: "1.0.0",
            framework,
            source: PluginSource::Official,
            mode: PluginMode::Replacement,
            checksum: "sha256:official",
            signature: None,
            public_key_fingerprint: None,
            revocation_list: None,
        };
        let user = PluginManifest {
            source: PluginSource::User,
            ..official
        };
        assert!(PluginPolicy::open(framework).accepts(official, None));
        assert!(!PluginPolicy::local(framework).accepts(official, None));
        assert!(PluginPolicy::open(framework).accepts(user, None));
        assert!(!PluginPolicy::open(FrameworkId::new("com.other.app")).accepts(official, None));
    }

    /// The official lock check is reachable through `accepts`: a manifest the
    /// host's catalog does not list is refused when the catalog is supplied,
    /// and was admitted on its own word when the predicate hardcoded `None`.
    /// 官方锁检查可以经 `accepts` 抵达：宿主目录里没有的 manifest 在传入目录时被拒；而谓词硬编码
    /// `None` 时，它仅凭自述就被接纳。
    #[test]
    fn the_official_lock_check_is_reachable_through_accepts() {
        let framework = FrameworkId::new("com.nichui.editor");
        let manifest = PluginManifest {
            name: "official.canvas",
            crate_name: "official_canvas",
            version: "1.0.0",
            framework,
            source: PluginSource::Official,
            mode: PluginMode::Replacement,
            checksum: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            signature: Some("adapter-verified-signature"),
            public_key_fingerprint: None,
            revocation_list: None,
        };
        let elsewhere = PluginCatalog::parse(
            "official|com.nichui.editor|official.other|1.0.0|official_other|sha256:aa|replacement",
        )
        .expect("a lock with another package parses");

        assert!(PluginPolicy::open(framework).accepts(manifest, None));
        assert_eq!(
            PluginPolicy::open(framework).decision(manifest, Some(&elsewhere)),
            PluginDecision::Rejected(PluginRejectReason::LockMismatch)
        );
        assert!(!PluginPolicy::open(framework).accepts(manifest, Some(&elsewhere)));
    }

    #[test]
    fn strict_policy_explains_digest_rejection() {
        let manifest = PluginManifest {
            name: "local",
            crate_name: "local",
            version: "1.0.0",
            framework: FrameworkId::new("nichlink.default"),
            source: PluginSource::User,
            mode: PluginMode::Extension,
            checksum: "fixture-development",
            signature: None,
            public_key_fingerprint: None,
            revocation_list: None,
        };
        assert_eq!(
            PluginPolicy::strict(manifest.framework).decision(manifest, None),
            PluginDecision::Rejected(PluginRejectReason::InvalidDigest)
        );
        let digest = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(
            PluginManifest {
                checksum: digest,
                ..manifest
            }
            .verify_bytes(b"abc")
        );
    }

    #[test]
    fn strict_policy_accepts_prefixed_sha256_checksums() {
        let framework = FrameworkId::new("nichlink.default");
        let manifest = PluginManifest {
            name: "local",
            crate_name: "local",
            version: "1.0.0",
            framework,
            source: PluginSource::User,
            mode: PluginMode::Extension,
            checksum: "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            signature: None,
            public_key_fingerprint: None,
            revocation_list: None,
        };
        assert_eq!(
            PluginPolicy::strict(framework).decision(manifest, None),
            PluginDecision::Accepted
        );
    }

    #[test]
    fn strict_policy_rejects_an_unsigned_official_face() {
        let framework = FrameworkId::new("nichlink.default");
        let manifest = PluginManifest {
            name: "official",
            crate_name: "official",
            version: "1.0.0",
            framework,
            source: PluginSource::Official,
            mode: PluginMode::Extension,
            checksum: "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            signature: None,
            public_key_fingerprint: None,
            revocation_list: None,
        };
        assert_eq!(
            PluginPolicy::strict(framework).decision(manifest, None),
            PluginDecision::Rejected(PluginRejectReason::MissingSignature)
        );
    }
}
