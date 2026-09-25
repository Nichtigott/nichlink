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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::declaration::{LocalizedText, ObjectContract, SourceLocation};
    use crate::plugin::{PluginChannel, validate_artifact};
    use crate::{
        Admission, FlowContract, FrameworkId, NodeId, PluginManifest, PluginMode, PluginSource,
        RegistrationInfo, RegistrationRule,
    };

    /// A signing-key fingerprint the policy trusts, and a verifier that accepts
    /// exactly the signature this fixture carries.
    /// 策略信任的签名密钥指纹，以及一个只接受本夹具所带签名的验证器。
    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const BYTES: &[u8] = b"abc";
    /// SHA-256 of `abc`, so the manifest's checksum matches the bytes.
    /// `abc` 的 SHA-256，使 manifest 的校验和与字节一致。
    const CHECKSUM: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    struct AcceptingVerifier;

    impl PluginSignatureVerifier for AcceptingVerifier {
        fn verify(&self, manifest: PluginManifest, bytes: &[u8], key_fingerprint: &str) -> bool {
            manifest.signature == Some("adapter-verified-signature")
                && bytes == BYTES
                && key_fingerprint.len() == 64
        }
    }

    struct RejectingVerifier;

    impl PluginSignatureVerifier for RejectingVerifier {
        fn verify(&self, _: PluginManifest, _: &[u8], _: &str) -> bool {
            false
        }
    }

    /// One official plugin artifact: the only source the Official channel accepts.
    /// 一个官方插件工件：Official 通道唯一接受的来源。
    fn official_artifact() -> PluginArtifact {
        PluginArtifact {
            registration: RegistrationInfo {
                namespace: "plugin-test",
                id: NodeId::from_path("plugin.rs", "Plugin"),
                parent: crate::ROOT_NODE_ID,
                kind: "Plugin",
                preset: "",
                parts: "",
                params: "",
                handle: "PluginHandle",
                stable_name: None,
                name: LocalizedText {
                    zh: "插件",
                    en: "Plugin",
                },
                summary: LocalizedText { zh: "", en: "" },
                exports: &[],
                needs_registry: false,
                registry_name: "plugin",
                getting_from_other_registry: None,
                registry_rule_path: "<test>",
                registry_rule: RegistrationRule::ANY,
                admission: Admission::ANY,
                requires: &[],
                provides: &[],
                contract: ObjectContract {
                    required_parts: &[],
                    provided_parts: &[],
                },
                flow: FlowContract::NONE,
                flow_provider: None,
                handle_traits: &[],
                part_traits: &[],
                runtime_checks: &[],
                plugin: Some(PluginManifest {
                    name: "official.canvas",
                    crate_name: "official_canvas",
                    version: "1.0.0",
                    framework: FrameworkId::new("nichlink.test"),
                    source: PluginSource::Official,
                    mode: PluginMode::Extension,
                    checksum: CHECKSUM,
                    signature: Some("adapter-verified-signature"),
                    public_key_fingerprint: Some(KEY),
                    revocation_list: Some("official-2026"),
                }),
                source: SourceLocation {
                    file: "plugin.rs",
                    line: 1,
                    column: 1,
                    function: "plugin",
                },
            },
            bytes: BYTES.to_vec(),
            key_fingerprint: Some(KEY.to_owned()),
        }
    }

    /// The admission the Official channel gives one artifact.
    /// Official 通道给某个工件的准入结论。
    fn official_channel(artifact: &VerifiedPluginArtifact) -> Result<(), SlotValidationError> {
        validate_artifact(
            "canvas",
            FrameworkId::new("nichlink.test"),
            PluginMode::Extension,
            FlowContract::NONE,
            &[PluginChannel::Official],
            PluginChannel::Official,
            artifact,
        )
    }

    /// A verified signature is the only way into the Official lane: the digest-only
    /// path keeps the lower assurance and is refused there.
    /// 已验证的签名是进入 Official 通道的唯一途径：纯摘要路径保持较低等级，并在那里被拒。
    #[test]
    fn a_signed_artifact_is_the_only_way_into_the_official_channel() {
        let policy = PluginTrustPolicy::official(&[KEY], &[]);
        let signed = official_artifact()
            .verify_signed(policy, &AcceptingVerifier)
            .expect("a trusted signature verifies");
        assert_eq!(signed.assurance(), PluginAssurance::Signature);

        let digest_only = official_artifact()
            .verify_artifact(policy)
            .expect("the checksum path still works");
        assert_eq!(digest_only.assurance(), PluginAssurance::Digest);

        assert!(
            official_channel(&signed).is_ok(),
            "a verified signature must open the official lane"
        );
        assert!(
            official_channel(&digest_only).is_err(),
            "a checksum alone must not"
        );
    }

    /// Revocation is consulted before the signature is accepted: a revoked version
    /// is refused even with a signature the host would otherwise accept.
    /// 撤销在接受签名之前就被检查：即使签名本来会被宿主接受，已吊销版本仍被拒绝。
    #[test]
    fn a_revoked_version_is_refused_even_with_a_good_signature() {
        let policy = PluginTrustPolicy::official(
            &[KEY],
            &[PluginRevocation {
                package: "official.canvas",
                version: "1.0.0",
            }],
        );
        assert!(matches!(
            official_artifact().verify_signed(policy, &AcceptingVerifier),
            Err(PluginTrustError::Revoked)
        ));
    }

    /// Signature assurance is refused for a source no verifier is consulted for.
    /// 对不会咨询验证器的来源，签名保证被拒绝。
    ///
    /// The audit's probe read `assurance=Signature verifier_called=false`: the field
    /// is documented as the strongest check actually performed, and it was false for
    /// every user artifact. The refusal is explicit, and the digest path still
    /// accepts the same artifact.
    /// 审计的探针读到 `assurance=Signature verifier_called=false`：该字段的文档写的是"实际
    /// 执行过的最强检查"，而它对每个用户工件都是假话。这里显式拒绝，而纯摘要路径仍然接受同一个
    /// 工件。
    #[test]
    fn signature_assurance_is_refused_for_a_user_artifact() {
        let mut artifact = official_artifact();
        let mut manifest = artifact.registration.plugin.expect("manifest");
        manifest.source = PluginSource::User;
        manifest.signature = None;
        manifest.public_key_fingerprint = None;
        artifact.registration.plugin = Some(manifest);
        artifact.key_fingerprint = None;

        assert!(
            matches!(
                artifact
                    .clone()
                    .verify_signed(PluginTrustPolicy::open(), &AcceptingVerifier),
                Err(PluginTrustError::SignatureLaneRequired)
            ),
            "a lane that never calls a verifier cannot record a verified signature"
        );

        let digest = artifact
            .verify_artifact(PluginTrustPolicy::open())
            .expect("the checksum path accepts a user artifact");
        assert_eq!(digest.assurance(), PluginAssurance::Digest);
    }

    /// A refused signature never earns the stronger assurance.
    /// 被拒绝的签名永远换不到更强的保证。
    #[test]
    fn a_refused_signature_never_earns_the_stronger_assurance() {
        assert!(matches!(
            official_artifact()
                .verify_signed(PluginTrustPolicy::official(&[KEY], &[]), &RejectingVerifier),
            Err(PluginTrustError::SignatureNotVerified)
        ));
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
