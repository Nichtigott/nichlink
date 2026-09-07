//! Plugin execution policy and adapter contracts.
//! 插件执行策略与适配器合同。

use super::*;

/// Whether a plugin adds a new capability or replaces an existing slot.
/// 插件是增加能力还是替换已有插槽。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginMode {
    Extension,
    Replacement,
}

/// Where plugin code executes. Native code is trusted process code; the other
/// two modes are explicit isolation choices for untrusted extensions.
/// 插件代码的执行隔离方式。Native 与宿主同进程；另外两种是明确的隔离选项。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginAdapter {
    Native,
    Wasm,
    Process,
}

impl PluginAdapter {
    pub const fn requires_isolation(self) -> bool {
        !matches!(self, Self::Native)
    }
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
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "official" => Some(Self::Official),
            "user" => Some(Self::User),
            _ => None,
        }
    }
}

impl PluginMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "extension" => Some(Self::Extension),
            "replacement" => Some(Self::Replacement),
            _ => None,
        }
    }
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
    type Instance;

    fn adapter(&self) -> PluginAdapter;

    fn load(&self, manifest: PluginManifest, bytes: &[u8]) -> Result<Self::Instance, String>;
}

/// Host policy for selecting which linked plugin manifests may participate.
/// 宿主选择已链接插件是否参与的策略。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluginPolicy {
    pub framework: FrameworkId,
    pub allow_official: bool,
    pub allow_user: bool,
    /// Reject manifests whose checksum is not a cryptographic digest.
    /// 拒绝不是密码学摘要的插件 manifest。
    pub require_digest: bool,
}

impl PluginPolicy {
    pub const fn local(framework: FrameworkId) -> Self {
        Self {
            framework,
            allow_official: false,
            allow_user: false,
            require_digest: true,
        }
    }

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

    pub fn accepts(self, plugin: PluginManifest) -> bool {
        matches!(self.decision(plugin, None), PluginDecision::Accepted)
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

    /// Apply source/framework policy and, for official plugins, a lock file.
    /// 同时应用来源/框架策略，并对官方插件校验锁文件。
    pub fn accepts_with_catalog(self, plugin: PluginManifest, catalog: &PluginCatalog) -> bool {
        matches!(
            self.decision(plugin, Some(catalog)),
            PluginDecision::Accepted
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(PluginPolicy::open(framework).accepts(official));
        assert!(!PluginPolicy::local(framework).accepts(official));
        assert!(PluginPolicy::open(framework).accepts(user));
        assert!(!PluginPolicy::open(FrameworkId::new("com.other.app")).accepts(official));
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
    fn trust_policy_checks_bytes_key_and_revocation() {
        const DIGEST: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        const KEYS: &[&str] = &[KEY];
        const REVOKED: &[PluginRevocation] = &[];
        let manifest = PluginManifest {
            name: "official.canvas",
            crate_name: "official_canvas",
            version: "1.0.0",
            framework: FrameworkId::new("nichlink.default"),
            source: PluginSource::Official,
            mode: PluginMode::Replacement,
            checksum: DIGEST,
            signature: Some("adapter-verified-signature"),
            public_key_fingerprint: Some(KEY),
            revocation_list: Some("official-2026-09"),
        };
        let policy = PluginTrustPolicy::official(KEYS, REVOKED);
        assert_eq!(policy.verify(manifest, b"abc", Some(KEY)), Ok(()));
        assert_eq!(
            policy.verify(
                PluginManifest {
                    public_key_fingerprint: None,
                    ..manifest
                },
                b"abc",
                None,
            ),
            Err(PluginTrustError::MissingOfficialKey)
        );
        assert_eq!(
            policy.verify(
                PluginManifest {
                    revocation_list: None,
                    ..manifest
                },
                b"abc",
                Some(KEY),
            ),
            Err(PluginTrustError::MissingRevocationList)
        );
        let revoked = PluginTrustPolicy::official(
            KEYS,
            &[PluginRevocation {
                package: "official.canvas",
                version: "1.0.0",
            }],
        );
        assert_eq!(
            revoked.verify(manifest, b"abc", Some(KEY)),
            Err(PluginTrustError::Revoked)
        );
    }

    struct AcceptingVerifier;

    impl PluginSignatureVerifier for AcceptingVerifier {
        fn verify(&self, manifest: PluginManifest, bytes: &[u8], key_fingerprint: &str) -> bool {
            manifest.signature == Some("adapter-verified-signature")
                && bytes == b"abc"
                && key_fingerprint.len() == 64
        }
    }

    #[test]
    fn trust_policy_can_delegate_signature_verification() {
        const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let manifest = PluginManifest {
            name: "official.canvas",
            crate_name: "official_canvas",
            version: "1.0.0",
            framework: FrameworkId::new("nichlink.default"),
            source: PluginSource::Official,
            mode: PluginMode::Extension,
            checksum: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            signature: Some("adapter-verified-signature"),
            public_key_fingerprint: Some(KEY),
            revocation_list: Some("official-2026"),
        };
        let policy = PluginTrustPolicy::official(&[KEY], &[]);
        assert_eq!(
            policy.verify_with(manifest, b"abc", Some(KEY), &AcceptingVerifier),
            Ok(())
        );
        assert_eq!(
            policy.verify_with(manifest, b"abc", Some(KEY), &RejectingVerifier),
            Err(PluginTrustError::SignatureNotVerified)
        );
    }

    struct RejectingVerifier;

    impl PluginSignatureVerifier for RejectingVerifier {
        fn verify(&self, _: PluginManifest, _: &[u8], _: &str) -> bool {
            false
        }
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

    #[test]
    fn plugin_lock_parser_keeps_official_and_user_records_typed() {
        let catalog = PluginCatalog::parse(
            "# source|framework|package|version|crate|checksum|mode|signature|key|revocations\n\
             official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|replacement|sig-v1|key-v1|official-2026\n\
             user|com.nichui.editor|local|0.1.0|local_canvas|sha256:b|extension\n",
        )
        .expect("lock should parse");
        assert_eq!(catalog.records().len(), 2);
        assert_eq!(catalog.records()[0].source, PluginSource::Official);
        assert_eq!(catalog.records()[0].signature.as_deref(), Some("sig-v1"));
        assert_eq!(
            catalog.records()[0].revocation_list.as_deref(),
            Some("official-2026")
        );
        assert_eq!(catalog.records()[1].mode, PluginMode::Extension);
    }

    #[test]
    fn plugin_lock_parser_rejects_ambiguous_duplicate_identity() {
        let lock = "official|com.nichui.editor|canvas|1.0.0|canvas|sha256:a|extension\n\
                    official|com.nichui.editor|canvas|1.0.0|canvas|sha256:b|extension\n";
        let error = PluginCatalog::parse(lock).unwrap_err();
        assert!(error.contains("line 2"));
        assert!(error.contains("duplicates package identity"));
    }

    #[test]
    fn plugin_lock_parser_rejects_unknown_identity_schema() {
        let error = PluginCatalog::parse(
            "# nichlink-schema=2\n\
             user|com.nichui.editor|local|0.1.0|local_canvas|sha256:b|extension\n",
        )
        .unwrap_err();
        assert!(error.contains("identity schema 2"));
        assert!(error.contains("expected 3"));
    }

    #[test]
    fn graft_command_has_a_small_unambiguous_grammar() {
        assert_eq!(
            GraftCommand::parse("graft canvas2d_v2 to canvas2d").unwrap(),
            GraftCommand {
                replacement: "canvas2d_v2".to_owned(),
                target: "canvas2d".to_owned(),
            }
        );
        assert!(GraftCommand::parse("graft canvas2d").is_err());
    }
}
