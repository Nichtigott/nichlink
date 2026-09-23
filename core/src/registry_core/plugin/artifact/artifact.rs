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
    /// Registration data the plugin bytes are claimed to produce.
    /// 插件字节声称产出的注册声明。
    pub registration: crate::RegistrationInfo,
    /// Raw plugin bytes covered by the manifest checksum.
    /// manifest 摘要所覆盖的原始插件字节。
    pub bytes: Vec<u8>,
    /// Fingerprint of the key that signed these bytes, when the host supplied one.
    /// 签名这些字节的密钥指纹；宿主未提供时为 None。
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
    /// Only the manifest checksum was verified.
    /// 仅校验了 manifest 摘要。
    Digest,
    /// A host signature verifier also accepted the plugin.
    /// 宿主签名验证器也接受了该插件。
    Signature,
}

impl VerifiedPluginArtifact {
    /// The registration data that passed verification.
    /// 已通过校验的注册声明。
    pub const fn registration(&self) -> crate::RegistrationInfo {
        self.registration
    }

    /// The verified plugin bytes.
    /// 已校验的插件字节。
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The strongest check recorded for this artifact.
    /// 本工件实际记录的最高校验等级。
    pub const fn assurance(&self) -> PluginAssurance {
        self.assurance
    }

    /// Consume the artifact into its registration data and bytes.
    /// 消耗工件，取出注册声明与插件字节。
    pub fn into_parts(self) -> (crate::RegistrationInfo, Vec<u8>) {
        (self.registration, self.bytes)
    }
}

impl PluginArtifact {
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
}

/// One auditable plugin-selection result.
/// 一条可审计的插件筛选结果。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginDecision {
    /// The policy admitted the plugin.
    /// 策略接纳了该插件。
    Accepted,
    /// The policy refused the plugin, recording why.
    /// 策略拒绝了该插件，并记录原因。
    Rejected(PluginRejectReason),
}

/// Why a plugin-selection decision refused a manifest.
/// 插件筛选决策拒绝某个 manifest 的原因。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginRejectReason {
    /// The manifest targets a different framework.
    /// manifest 针对的是另一个框架。
    FrameworkMismatch,
    /// The policy disables the manifest's source.
    /// 策略关闭了该 manifest 的来源。
    SourceDisabled,
    /// The checksum is not a valid SHA-256 digest.
    /// 摘要不是合法的 SHA-256 值。
    InvalidDigest,
    /// An official plugin carried no usable signature.
    /// 官方插件没有可用的签名。
    MissingSignature,
    /// The official lock record does not match the manifest.
    /// 官方锁记录与该 manifest 不一致。
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
