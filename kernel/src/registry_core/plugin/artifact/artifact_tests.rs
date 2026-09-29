//! Tests for the verified-plugin artifact and its assurance ladder.
//! 已验证插件工件及其保证等级阶梯的测试。

use super::*;
use crate::declaration::{LocalizedText, ObjectContract, SourceLocation};
use crate::plugin::{PluginChannel, validate_artifact};
use crate::{
    Admission, ContractId, FlowContract, FrameworkId, NodeId, PluginManifest, PluginMode,
    PluginSource, RegistrationInfo, RegistrationRule,
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
    fn verify(&self, manifest: PluginManifest, payload: &[u8], key_fingerprint: &str) -> bool {
        manifest.signature == Some("adapter-verified-signature")
            && !payload.is_empty()
            && key_fingerprint.len() == 64
    }
}

/// A verifier that signs by hashing: it accepts exactly the signature that
/// is the SHA-256 of the payload the kernel built. This is what makes a
/// tampered registration detectable in a test without a real key.
/// 以哈希"签名"的验证器：只接受"内核构造的载荷的 SHA-256"这个签名。测试因此不需要真密钥
/// 就能发现被篡改的注册声明。
struct PayloadDigestVerifier;

impl PluginSignatureVerifier for PayloadDigestVerifier {
    fn verify(&self, manifest: PluginManifest, payload: &[u8], key_fingerprint: &str) -> bool {
        manifest
            .signature
            .is_some_and(|signature| signature == crate::sha256_hex(payload))
            && key_fingerprint.len() == 64
    }
}

/// The hex digest this artifact's honest payload hashes to.
/// 本工件的诚实载荷散列出的十六进制摘要。
fn payload_digest(artifact: &PluginArtifact) -> &'static str {
    let manifest = artifact.registration.plugin.expect("manifest");
    let digest =
        crate::sha256_hex(&manifest.signing_payload(&artifact.registration, &artifact.bytes));
    Box::leak(digest.into_boxed_str())
}

/// The same artifact with its signature replaced by the payload digest, as a
/// hashing host would have produced it. The manifest's `signature` field is
/// not part of the payload, so the digest is the same before and after.
/// 把签名换成载荷摘要后的同一工件，等价于以哈希签名的宿主产出的结果。manifest 的
/// `signature` 字段不属于载荷，因此替换前后摘要相同。
fn signed_artifact() -> PluginArtifact {
    let mut artifact = official_artifact();
    let digest = payload_digest(&artifact);
    let mut manifest = artifact.registration.plugin.expect("manifest");
    manifest.signature = Some(digest);
    artifact.registration.plugin = Some(manifest);
    artifact
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

/// The signature must cover the registration that travels with the bytes.
/// The audit's probe tampered `registration.flow` and `parent` and the
/// artifact still reported `PluginAssurance::Signature`, passed a slot
/// contract check the honest artifact failed, and loaded.
/// 签名必须覆盖随字节同行的注册声明。审计的探针篡改了 `registration.flow` 与 `parent`，
/// 工件仍报告 `PluginAssurance::Signature`，通过了诚实工件通不过的槽位合同检查，并被加载。
#[test]
fn a_tampered_registration_no_longer_earns_a_signature() {
    let policy = PluginTrustPolicy::official(&[KEY], &[]);
    assert_eq!(
        signed_artifact()
            .verify_signed(policy, &PayloadDigestVerifier)
            .expect("the honest artifact verifies")
            .assurance(),
        PluginAssurance::Signature
    );

    let mut moved = signed_artifact();
    moved.registration.parent = NodeId::from_path("other.rs", "Other");
    assert!(
        matches!(
            moved.verify_signed(policy, &PayloadDigestVerifier),
            Err(PluginTrustError::SignatureNotVerified)
        ),
        "a moved parent must not keep the signature"
    );

    let mut rewired = signed_artifact();
    rewired.registration.flow = FlowContract::new(ContractId::new("hijacked.v1"), 1, "A", "B");
    assert!(
        matches!(
            rewired.verify_signed(policy, &PayloadDigestVerifier),
            Err(PluginTrustError::SignatureNotVerified)
        ),
        "a rewired flow contract must not keep the signature"
    );
}
