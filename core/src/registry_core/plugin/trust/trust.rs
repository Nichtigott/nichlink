//! Plugin trust policy and verification errors.
//! 插件信任策略与验证错误。

use crate::registry_core::declaration::{PluginManifest, PluginSource};
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
    /// Return whether the host accepts this signature for the given bytes and key.
    /// 宿主是否接受该签名用于给定字节与密钥。
    fn verify(&self, manifest: PluginManifest, bytes: &[u8], key_fingerprint: &str) -> bool;
}

impl<F> PluginSignatureVerifier for F
where
    F: Fn(PluginManifest, &[u8], &str) -> bool,
{
    fn verify(&self, manifest: PluginManifest, bytes: &[u8], key_fingerprint: &str) -> bool {
        self(manifest, bytes, key_fingerprint)
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
    pub fn verify_with<V: PluginSignatureVerifier>(
        self,
        manifest: PluginManifest,
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
        if !verifier.verify(manifest, bytes, fingerprint) {
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
}
