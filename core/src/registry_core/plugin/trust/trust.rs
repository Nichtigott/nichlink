//! Plugin trust policy and verification errors.
//! 插件信任策略与验证错误。

use crate::registry_core::declaration::{PluginManifest, PluginSource, RegistrationInfo};
use std::fmt;

/// A small, dependency-free trust policy for plugin bytes.
/// 一个零依赖的插件字节信任策略。
///
/// This is intentionally a checksum and key-fingerprint policy, not a
/// home-grown signature scheme. Signature verification belongs in the
/// adapter that loads a plugin; NichLink only decides whether the verified
/// identity is allowed to enter this registry.
/// 这里刻意只做摘要和公钥指纹策略，不伪造签名算法。真正的签名校验由加载插件的
/// adapter 完成，NichLink 只判断已校验身份是否可以进入注册树。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluginTrustPolicy {
    /// SHA-256 fingerprints of keys trusted to sign official plugins.
    /// 被信任可为官方插件签名的密钥 SHA-256 指纹。
    pub official_key_fingerprints: &'static [&'static str],
    /// Revocations that refuse a package/version pair outright.
    /// 直接拒绝某个包/版本组合的撤销条目。
    pub revoked: &'static [PluginRevocation],
    /// Whether the manifest checksum must be a SHA-256 digest.
    /// manifest 摘要是否必须是 SHA-256 值。
    pub require_digest: bool,
}

/// Host-provided cryptographic verifier for official plugin signatures.
/// 由宿主提供的官方插件签名密码学验证器。
///
/// NichLink deliberately does not implement a signature scheme. The host can
/// connect its existing Ed25519, platform keystore, or isolated process
/// verifier without making the registry depend on that implementation.
/// NichLink 刻意不实现具体签名算法。宿主可以接入现有的 Ed25519、平台密钥库或
/// 隔离进程验证器，而不让注册机依赖这些实现。
pub trait PluginSignatureVerifier {
    /// Return whether the host accepts this signature over the canonical payload.
    /// 宿主是否接受该签名用于给定的规范化载荷。
    ///
    /// `payload` is the bytes the kernel built for this artifact: the manifest
    /// fields, the registration that travels beside the plugin bytes, and the
    /// bytes themselves. The host supplies the cryptography and does not
    /// reassemble the message, so a payload field added later is covered by
    /// every verifier without a change on the host side — and no verifier can
    /// quietly drop a field core put in.
    /// `payload` 是内核为该工件构造的字节：manifest 字段、随插件字节同行的注册声明，以及字节
    /// 本身。宿主只提供密码学、不自行拼装消息：日后新增的载荷字段无需宿主改动即被每个验证器覆盖，
    /// 任何验证器也无法悄悄丢掉内核放进去的字段。
    fn verify(&self, manifest: PluginManifest, payload: &[u8], key_fingerprint: &str) -> bool;
}

impl<F> PluginSignatureVerifier for F
where
    F: Fn(PluginManifest, &[u8], &str) -> bool,
{
    fn verify(&self, manifest: PluginManifest, payload: &[u8], key_fingerprint: &str) -> bool {
        self(manifest, payload, key_fingerprint)
    }
}

impl PluginTrustPolicy {
    /// Development policy: no trusted keys and no digest requirement.
    /// 开发态策略：没有信任密钥，也不强制摘要。
    pub const fn open() -> Self {
        Self {
            official_key_fingerprints: &[],
            revoked: &[],
            require_digest: false,
        }
    }

    /// Policy that requires a digest and trusts only the listed official keys.
    /// 强制摘要、且只信任所列官方密钥的策略。
    pub const fn official(
        keys: &'static [&'static str],
        revoked: &'static [PluginRevocation],
    ) -> Self {
        Self {
            official_key_fingerprints: keys,
            revoked,
            require_digest: true,
        }
    }

    /// Run the checksum, revocation, and official-key checks; does no signature work.
    /// 执行摘要、撤销与官方密钥检查；本身不做签名运算。
    pub fn verify(
        self,
        manifest: PluginManifest,
        bytes: &[u8],
        key_fingerprint: Option<&str>,
    ) -> Result<(), PluginTrustError> {
        if self.require_digest && !super::artifact::is_sha256(manifest.checksum) {
            return Err(PluginTrustError::InvalidDigest);
        }
        if !manifest.verify_bytes(bytes) {
            return Err(PluginTrustError::DigestMismatch);
        }
        if self
            .revoked
            .iter()
            .any(|entry| entry.package == manifest.name && entry.version == manifest.version)
        {
            return Err(PluginTrustError::Revoked);
        }
        if manifest.source == PluginSource::Official && !self.official_key_fingerprints.is_empty() {
            if manifest.revocation_list.is_none() {
                return Err(PluginTrustError::MissingRevocationList);
            }
            if manifest
                .signature
                .is_none_or(|signature| signature.trim().is_empty())
            {
                return Err(PluginTrustError::MissingSignature);
            }
            let fingerprint = key_fingerprint
                .or(manifest.public_key_fingerprint)
                .ok_or(PluginTrustError::MissingOfficialKey)?;
            if !super::artifact::is_sha256(fingerprint) {
                return Err(PluginTrustError::UntrustedOfficialKey);
            }
            if !self
                .official_key_fingerprints
                .iter()
                .any(|allowed| allowed.eq_ignore_ascii_case(fingerprint))
            {
                return Err(PluginTrustError::UntrustedOfficialKey);
            }
        }
        Ok(())
    }

    /// Verify metadata and delegate the actual signature operation to the host.
    /// 校验元数据，并把真正的签名操作委托给宿主。
    ///
    /// The verifier is handed `manifest.signing_payload(registration, bytes)`,
    /// the canonical message that covers the registration as well as the
    /// manifest and the bytes; the policy's own checks still read the raw
    /// `bytes`, because a checksum is over the bytes and not over the message.
    /// 交给验证器的是 `manifest.signing_payload(registration, bytes)`——同时覆盖注册声明、
    /// manifest 与字节的规范消息；策略自身的检查仍读原始 `bytes`，因为摘要是对字节而不是对消息。
    pub fn verify_with<V: PluginSignatureVerifier>(
        self,
        manifest: PluginManifest,
        registration: &RegistrationInfo,
        bytes: &[u8],
        key_fingerprint: Option<&str>,
        verifier: &V,
    ) -> Result<(), PluginTrustError> {
        self.verify(manifest, bytes, key_fingerprint)?;
        if manifest.source != PluginSource::Official {
            return Ok(());
        }
        // A production verifier must have an explicit trust root. An empty
        // key list is useful for development only and must never silently
        // downgrade official plugins to checksum-only validation.
        // 生产验证器必须显式配置信任根。空公钥列表只适合开发态，不能让官方插件
        // 悄悄降级为仅摘要校验。
        if self.official_key_fingerprints.is_empty() {
            return Err(PluginTrustError::MissingOfficialKey);
        }
        let fingerprint = key_fingerprint
            .or(manifest.public_key_fingerprint)
            .ok_or(PluginTrustError::MissingOfficialKey)?;
        let payload = manifest.signing_payload(registration, bytes);
        if !verifier.verify(manifest, &payload, fingerprint) {
            return Err(PluginTrustError::SignatureNotVerified);
        }
        Ok(())
    }
}

/// A withdrawn package version, matched exactly by package and version.
/// 已撤销的包版本，按包名与版本精确匹配。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluginRevocation {
    /// Package name being withdrawn.
    /// 被撤销的包名。
    pub package: &'static str,
    /// Version being withdrawn.
    /// 被撤销的版本。
    pub version: &'static str,
}

/// Why plugin trust verification refused an artifact.
/// 插件信任校验拒绝某个工件的原因。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginTrustError {
    /// Artifact carried no plugin manifest.
    /// 工件没有携带插件 manifest。
    MissingManifest,
    /// Checksum is not a SHA-256 digest while a digest is required.
    /// 要求摘要时，校验和却不是 SHA-256 值。
    InvalidDigest,
    /// Bytes do not hash to the manifest checksum.
    /// 字节的散列与 manifest 摘要不符。
    DigestMismatch,
    /// Official plugin has no non-empty signature.
    /// 官方插件没有非空签名。
    MissingSignature,
    /// Official plugin has no revocation-list snapshot.
    /// 官方插件没有撤销列表快照。
    MissingRevocationList,
    /// No signing-key fingerprint was available to check.
    /// 没有可用于校验的签名密钥指纹。
    MissingOfficialKey,
    /// Signing key is not in the policy's trusted list.
    /// 签名密钥不在策略的信任列表中。
    UntrustedOfficialKey,
    /// Host verifier rejected the signature.
    /// 宿主验证器拒绝了该签名。
    SignatureNotVerified,
    /// Package version appears in the revocation list.
    /// 包版本出现在撤销列表中。
    Revoked,
    /// Signature assurance was requested for a source no verifier is consulted for.
    /// 对一个不会咨询验证器的来源请求了签名保证。
    ///
    /// [`PluginTrustPolicy::verify_with`] consults a verifier only for the official
    /// lane, so recording `PluginAssurance::Signature` for a user artifact would
    /// make the field — documented as the strongest check actually performed — false.
    /// The refusal is explicit instead of a silent downgrade, because the caller
    /// asked for a check this lane cannot perform; the digest path stays available.
    /// [`PluginTrustPolicy::verify_with`] 只为官方通道咨询验证器，因此对用户工件记录
    /// `PluginAssurance::Signature` 会让那个"实际执行过的最强检查"字段变成假话。这里显式
    /// 拒绝而不是静默降级，因为调用方请求的是本通道做不到的检查；纯摘要路径仍然可用。
    SignatureLaneRequired,
}

impl fmt::Display for PluginTrustError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingManifest => "plugin artifact has no manifest",
            Self::InvalidDigest => "plugin checksum is not a SHA-256 digest",
            Self::DigestMismatch => "plugin bytes do not match the manifest checksum",
            Self::MissingSignature => "official plugin has no signature payload",
            Self::MissingRevocationList => "official plugin has no revocation-list snapshot",
            Self::MissingOfficialKey => "official plugin has no signing-key fingerprint",
            Self::UntrustedOfficialKey => "official plugin signing key is not trusted",
            Self::SignatureNotVerified => "official plugin signature was not verified",
            Self::SignatureLaneRequired => {
                "signature assurance requires the official lane; use the digest path for other sources"
            }
            Self::Revoked => "plugin version has been revoked",
        })
    }
}

impl std::error::Error for PluginTrustError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_core::declaration::{FrameworkId, PluginMode};

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
        fn verify(&self, manifest: PluginManifest, payload: &[u8], key_fingerprint: &str) -> bool {
            manifest.signature == Some("adapter-verified-signature")
                && !payload.is_empty()
                && key_fingerprint.len() == 64
        }
    }

    /// The registration that carries one test manifest, so `verify_with` has a
    /// message to build. Every field is empty except the manifest: this test is
    /// about delegation, not about what the payload covers.
    /// 承载某个测试 manifest 的注册声明，供 `verify_with` 构造消息。除 manifest 外每个字段都为
    /// 空：本测试关心的是委托，而不是载荷覆盖了什么。
    fn registration(manifest: PluginManifest) -> crate::RegistrationInfo {
        use crate::registry_core::declaration::{
            Admission, LocalizedText, ObjectContract, RegistrationRule, SourceLocation,
        };
        crate::RegistrationInfo {
            namespace: "trust-test",
            id: crate::NodeId::from_path("trust.rs", "Trust"),
            parent: crate::ROOT_NODE_ID,
            kind: "Trust",
            preset: "",
            parts: "",
            params: "",
            handle: "",
            stable_name: None,
            name: LocalizedText { zh: "", en: "" },
            summary: LocalizedText { zh: "", en: "" },
            exports: &[],
            needs_registry: false,
            registry_name: "",
            getting_from_other_registry: None,
            registry_rule_path: "",
            registry_rule: RegistrationRule::ANY,
            admission: Admission::ANY,
            requires: &[],
            provides: &[],
            contract: ObjectContract {
                required_parts: &[],
                provided_parts: &[],
            },
            flow: crate::FlowContract::NONE,
            flow_provider: None,
            handle_traits: &[],
            part_traits: &[],
            runtime_checks: &[],
            plugin: Some(manifest),
            source: SourceLocation {
                file: "trust.rs",
                line: 1,
                column: 1,
                function: "registration",
            },
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
        let registration = registration(manifest);
        let policy = PluginTrustPolicy::official(&[KEY], &[]);
        assert_eq!(
            policy.verify_with(
                manifest,
                &registration,
                b"abc",
                Some(KEY),
                &AcceptingVerifier
            ),
            Ok(())
        );
        assert_eq!(
            policy.verify_with(
                manifest,
                &registration,
                b"abc",
                Some(KEY),
                &RejectingVerifier
            ),
            Err(PluginTrustError::SignatureNotVerified)
        );
    }

    struct RejectingVerifier;

    impl PluginSignatureVerifier for RejectingVerifier {
        fn verify(&self, _: PluginManifest, _: &[u8], _: &str) -> bool {
            false
        }
    }
}
