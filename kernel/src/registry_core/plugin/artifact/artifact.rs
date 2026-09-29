//! Verified plugin artifacts and selection decisions.
//! 已验证插件工件与筛选决策。

use super::*;
use crate::registry_core::declaration::PluginSource;
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

    /// Verify the bytes under `policy` **and** ask the host's signature verifier,
    /// recording the stronger assurance that second check buys.
    /// 在 `policy` 下校验字节，**并**询问宿主的签名验证器，记录第二次校验换来的更强保证。
    ///
    /// This is the only constructor of [`PluginAssurance::Signature`], and therefore
    /// the only way into a channel that requires it
    /// ([`PluginChannel::Official`](crate::plugin::PluginChannel)). Revocation is
    /// consulted before the verifier is asked, because the policy runs first:
    /// a revoked version is refused here exactly as it is on the digest-only path,
    /// whatever the signature says. Without this entry point the Official lane
    /// could never admit anything, and the control the threat model names would
    /// never run.
    /// 这是 [`PluginAssurance::Signature`] 的唯一构造入口，因而也是进入要求该等级的通道
    /// （[`PluginChannel::Official`](crate::plugin::PluginChannel)）的唯一途径。撤销在询问
    /// 验证器**之前**就被检查，因为策略先运行：无论签名说什么，已吊销版本在这里与纯摘要路径上
    /// 一样被拒。没有这个入口，Official 通道永远接纳不了任何东西，威胁模型点名的那个控制也就
    /// 永远不会运行。
    pub fn verify_signed(
        self,
        policy: PluginTrustPolicy,
        verifier: &impl PluginSignatureVerifier,
    ) -> Result<VerifiedPluginArtifact, PluginTrustError> {
        let manifest = self
            .registration
            .plugin
            .ok_or(PluginTrustError::MissingManifest)?;
        // `verify_with` consults a verifier only for the official lane, so this
        // entry point could otherwise record `Signature` for a user artifact whose
        // signature nothing checked — and the field it writes is documented as the
        // strongest check actually performed. Refuse instead of downgrading
        // silently: the caller asked for a signature check this lane cannot perform,
        // and the digest path stays available for that artifact.
        // `verify_with` 只为官方通道咨询验证器，因此本入口否则会为一个没有检查过签名的用户工件
        // 记录 `Signature`——而它写下的那个字段的文档写的是"实际执行过的最强检查"。这里拒绝而不是
        // 静默降级：调用方请求的是本通道做不到的签名检查，而那个工件仍然可用纯摘要路径。
        if manifest.source != PluginSource::Official {
            return Err(PluginTrustError::SignatureLaneRequired);
        }
        policy.verify_with(
            manifest,
            &self.registration,
            &self.bytes,
            self.key_fingerprint.as_deref(),
            verifier,
        )?;
        Ok(VerifiedPluginArtifact {
            registration: self.registration,
            bytes: self.bytes,
            assurance: PluginAssurance::Signature,
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

#[cfg(test)]
#[path = "artifact_tests.rs"]
mod artifact_tests;
