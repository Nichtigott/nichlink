//! Verified plugin artifacts and selection decisions.
//! 已验证插件工件与筛选决策。

use super::*;
use std::fmt;

/// External registration data together with the bytes that produced it.
/// 外部注册声明及其对应的插件字节。
///
/// A trusted build consumes this type instead of accepting an unverified
/// `RegistrationInfo`. The implementation stays opaque to the registry.
/// 严格构建只接收这个类型，不直接接收未验证的 `RegistrationInfo`；注册机仍不接触实现。
#[derive(Clone, Debug)]
pub struct PluginArtifact {
    pub registration: crate::RegistrationInfo,
    pub bytes: Vec<u8>,
    pub key_fingerprint: Option<String>,
}

/// Plugin bytes that passed the configured trust policy.
/// 已通过宿主信任策略的插件字节。
#[derive(Clone, Debug)]
pub struct VerifiedPluginArtifact {
    registration: crate::RegistrationInfo,
    bytes: Vec<u8>,
    assurance: PluginAssurance,
}

/// The strongest check actually performed on an artifact.
/// 插件工件实际完成的最高校验等级。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginAssurance {
    Digest,
    Signature,
}

impl VerifiedPluginArtifact {
    pub const fn registration(&self) -> crate::RegistrationInfo {
        self.registration
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub const fn assurance(&self) -> PluginAssurance {
        self.assurance
    }

    pub fn into_parts(self) -> (crate::RegistrationInfo, Vec<u8>) {
        (self.registration, self.bytes)
    }
}

impl PluginArtifact {
    pub fn verify(
        self,
        policy: PluginTrustPolicy,
    ) -> Result<crate::RegistrationInfo, PluginTrustError> {
        Ok(self.verify_artifact(policy)?.registration)
    }

    /// Verify bytes and return the only artifact accepted by execution hosts.
    /// 校验字节，并返回执行宿主唯一接受的工件类型。
    pub fn verify_artifact(
        self,
        policy: PluginTrustPolicy,
    ) -> Result<VerifiedPluginArtifact, PluginTrustError> {
        let manifest = self
            .registration
            .plugin
            .ok_or(PluginTrustError::MissingManifest)?;
        policy.verify(manifest, &self.bytes, self.key_fingerprint.as_deref())?;
        Ok(VerifiedPluginArtifact {
            registration: self.registration,
            bytes: self.bytes,
            assurance: PluginAssurance::Digest,
        })
    }

    /// Verify an artifact including its cryptographic signature.
    /// 校验插件工件，包括其密码学签名。
    pub fn verify_with<V: PluginSignatureVerifier>(
        self,
        policy: PluginTrustPolicy,
        verifier: &V,
    ) -> Result<crate::RegistrationInfo, PluginTrustError> {
        Ok(self.verify_artifact_with(policy, verifier)?.registration)
    }

    /// Verify bytes and signature for an execution host.
    /// 为执行宿主校验插件字节和签名。
    pub fn verify_artifact_with<V: PluginSignatureVerifier>(
        self,
        policy: PluginTrustPolicy,
        verifier: &V,
    ) -> Result<VerifiedPluginArtifact, PluginTrustError> {
        let manifest = self
            .registration
            .plugin
            .ok_or(PluginTrustError::MissingManifest)?;
        policy.verify_with(
            manifest,
            &self.bytes,
            self.key_fingerprint.as_deref(),
            verifier,
        )?;
        Ok(VerifiedPluginArtifact {
            registration: self.registration,
            bytes: self.bytes,
            assurance: if manifest.source == PluginSource::Official {
                PluginAssurance::Signature
            } else {
                PluginAssurance::Digest
            },
        })
    }
}

/// One auditable plugin-selection result.
/// 一条可审计的插件筛选结果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginDecision {
    Accepted,
    Rejected(PluginRejectReason),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginRejectReason {
    FrameworkMismatch,
    SourceDisabled,
    InvalidDigest,
    MissingSignature,
    LockMismatch,
}

impl fmt::Display for PluginRejectReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::FrameworkMismatch => "framework mismatch",
            Self::SourceDisabled => "plugin source disabled",
            Self::InvalidDigest => "checksum is not a SHA-256 digest",
            Self::MissingSignature => "official plugin signature is missing",
            Self::LockMismatch => "official lock record mismatch",
        })
    }
}

pub(super) fn is_sha256(value: &str) -> bool {
    let value = value.strip_prefix("sha256:").unwrap_or(value);
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
