//! Plugin trust policy and verification errors.
//! 插件信任策略与验证错误。

use super::*;
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
    pub official_key_fingerprints: &'static [&'static str],
    pub revoked: &'static [PluginRevocation],
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
    pub const fn open() -> Self {
        Self {
            official_key_fingerprints: &[],
            revoked: &[],
            require_digest: false,
        }
    }

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluginRevocation {
    pub package: &'static str,
    pub version: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginTrustError {
    /// Production loading was attempted without a cryptographic trust root.
    /// 生产加载没有配置密码学信任根。
    ProductionPolicyNotStrict,
    MissingManifest,
    InvalidDigest,
    DigestMismatch,
    MissingSignature,
    MissingRevocationList,
    MissingOfficialKey,
    UntrustedOfficialKey,
    SignatureNotVerified,
    Revoked,
}

impl fmt::Display for PluginTrustError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ProductionPolicyNotStrict => {
                "production plugin loading requires digest and trusted signing keys"
            }
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
