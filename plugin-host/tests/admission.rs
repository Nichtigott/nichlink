//! End-to-end plugin admission: lock file, policy, signature, slot.
//! 端到端插件准入：锁文件、策略、签名、槽。
//!
//! The audit found that no in-tree code ever called `verify_signed`: the kernel
//! had every rule and the adapters had every execution path, but the step between
//! them — read the lock the host wrote, select the manifest against it, verify the
//! artifact, and pick the lane the result qualifies for — belonged to nobody, so
//! the Official lane could not be reached at all. These tests walk that path
//! through the public host surface, with real Ed25519 signatures and real lock
//! files, and each refusal is pinned to the rule that refused it.
//! 审计发现树内没有任何代码调用过 `verify_signed`：内核有全部规则，适配器有全部执行路径，但两者
//! 之间那一步——读宿主写下的锁、按锁筛选 manifest、校验工件、定出结果够得上的通道——不属于任何人，
//! 因此 Official 通道完全无法抵达。这些测试用真实的 Ed25519 签名与真实的锁文件走完这条路，并且每条
//! 拒绝都钉在做出拒绝的那条规则上。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use ed25519_dalek::{Signer, SigningKey};
use nichlink_plugin_host::{
    Ed25519Verifier, OFFICIAL_LOCK, PLUGIN_LOCK_DIRECTORY, PluginAdmission, TrustedPublicKey,
    plugin_catalog,
};
#[cfg(feature = "wasm")]
use nichlink_plugin_host::{USER_LOCK, lane_for};
use nichlink_run_method::{
    Admission, FlowContract, FrameworkId, LocalizedText, NodeId, ObjectContract, PluginArtifact,
    PluginManifest, PluginMode, PluginPolicy, PluginRevocation, PluginSource, PluginTrustPolicy,
    RegistrationInfo, RegistrationRule, RuntimeCheckSpec, SourceLocation, sha256_hex,
};
// The lane vocabulary is only reachable through the Wasm table in this file, so
// the imports are gated with it rather than left unused in a feature-less build.
// 本文件里通道词汇只经 Wasm 表抵达，因此这些导入与它一同门控，而不是在无特性构建里悬空。
#[cfg(feature = "wasm")]
use nichlink_run_method::{PluginAssurance, PluginChannel};

/// A module that answers `health` with `ok` and echoes its input.
/// 一个以 `ok` 回答 `health` 并回显输入的小模块。
const ECHO: &str = r#"(module
  (memory (export "memory") 1)
  (data (i32.const 0) "ok")
  (func (export "nichlink_health") (param i32 i32) (result i64) (i64.const 2))
  (func (export "nichlink_echo") (param i32 i32) (result i64)
    (i64.extend_i32_u (local.get 1))))"#;

/// The framework every fixture targets.
/// 每个夹具针对的框架。
fn framework() -> FrameworkId {
    FrameworkId::new("nichlink.test")
}

/// One fixed signing key, so a fingerprint is stable across assertions.
/// 一把固定签名密钥，使指纹在各断言之间稳定。
fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[9; 32])
}

/// The SHA-256 fingerprint of the fixture key, leaked for the `&'static` API.
/// 夹具密钥的 SHA-256 指纹，为 `&'static` API 而泄漏。
fn fingerprint(key: &SigningKey) -> &'static str {
    Box::leak(sha256_hex(key.verifying_key().as_bytes()).into_boxed_str())
}

/// A verifier that trusts exactly the fixture key.
/// 只信任夹具密钥的验证器。
fn verifier(fingerprint: &'static str, key: &SigningKey) -> Ed25519Verifier {
    Ed25519Verifier::new(Box::leak(Box::new([TrustedPublicKey::new(
        fingerprint,
        key.verifying_key().to_bytes(),
    )])))
}

/// A leaked trust-key list, which is the shape the policy takes.
/// 泄漏后的信任密钥列表，也就是策略接受的形状。
fn trusted(fingerprint: &'static str) -> &'static [&'static str] {
    Box::leak(Box::new([fingerprint]))
}

/// One registration carrying `manifest`, with every optional field empty.
/// 一份携带 `manifest` 的注册声明，其余可选字段为空。
fn registration(manifest: PluginManifest) -> RegistrationInfo {
    RegistrationInfo {
        namespace: "plugin-test",
        id: NodeId::from_path("plugin.rs", "plugin"),
        parent: nichlink_run_method::ROOT_NODE_ID,
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
        runtime_checks: &[] as &[RuntimeCheckSpec],
        plugin: Some(manifest),
        source: SourceLocation {
            file: "plugin.rs",
            line: 1,
            column: 1,
            function: "plugin",
        },
    }
}

/// One raw artifact over the echo module, signed when a key is given.
/// 一个基于 echo 模块的原始工件，给出密钥时会被签名。
fn artifact(
    source: PluginSource,
    signing: Option<&SigningKey>,
    fingerprint: Option<&'static str>,
) -> PluginArtifact {
    let bytes = wat::parse_str(ECHO).expect("valid WAT");
    let checksum = Box::leak(sha256_hex(&bytes).into_boxed_str());
    let manifest = PluginManifest {
        name: "plugin-test",
        crate_name: "plugin_test",
        version: "1.0.0",
        framework: framework(),
        source,
        mode: PluginMode::Extension,
        checksum,
        signature: None,
        public_key_fingerprint: fingerprint,
        revocation_list: (source == PluginSource::Official).then_some("official-2026"),
    };
    let mut registration = registration(manifest);
    if let Some(signing) = signing {
        // The signature covers the message the kernel builds, which includes the
        // registration — signing the old manifest-plus-bytes payload would not
        // verify, and that is the point of the pin below.
        // 签名覆盖内核构造的消息，其中包含注册声明；签旧的"manifest 加字节"载荷不会通过验证，
        // 而这正是下面那条钉子的意义。
        let encoded = signing
            .sign(&manifest.signing_payload(&registration, &bytes))
            .to_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let mut signed = manifest;
        signed.signature = Some(Box::leak(encoded.into_boxed_str()));
        registration.plugin = Some(signed);
    }
    PluginArtifact {
        registration,
        bytes,
        key_fingerprint: fingerprint.map(str::to_owned),
    }
}

/// A throwaway package root.
/// 一个一次性包根。
fn package_root() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-admission-{}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).expect("fixture root");
    root
}

/// The seven-field lock line a host's own plugin UI writes.
/// 宿主自己的插件界面写下的七字段锁记录。
fn record(lane: &str, manifest: PluginManifest) -> String {
    format!(
        "{lane}|{}|{}|{}|{}|{}|extension",
        manifest.framework.0,
        manifest.name,
        manifest.version,
        manifest.crate_name,
        manifest.checksum
    )
}

/// Write one lock file under `root`, creating the directory.
/// 在 `root` 下写一份锁文件，并创建目录。
fn write_lock(root: &Path, name: &str, lines: &[String]) {
    let directory = root.join(PLUGIN_LOCK_DIRECTORY);
    std::fs::create_dir_all(&directory).expect("lock directory");
    std::fs::write(directory.join(name), format!("{}\n", lines.join("\n"))).expect("lock file");
}

/// One admission over `root` with the fixture trust root.
/// 在 `root` 上用夹具信任根构造的准入。
fn admission(root: &Path, key: &SigningKey) -> PluginAdmission<Ed25519Verifier> {
    let print = fingerprint(key);
    PluginAdmission::from_package_root(
        root,
        PluginPolicy::strict(framework()),
        PluginTrustPolicy::official(trusted(print), &[]),
        verifier(print, key),
    )
    .expect("the fixture locks parse")
}

/// A table with one slot that admits exactly `channels`.
/// 一张只接纳 `channels` 的单槽表。
#[cfg(feature = "wasm")]
fn table(channels: &'static [PluginChannel]) -> nichlink_plugin_host::WasmPluginTable {
    use nichlink_plugin_host::{WasmBackend, WasmLimits, WasmPluginSlot, WasmPluginTable};
    let slot = WasmPluginSlot::new(
        "test",
        framework(),
        PluginMode::Extension,
        FlowContract::NONE,
        channels,
    );
    WasmPluginTable::with_backend(
        Box::leak(Box::new([slot])),
        WasmBackend::new(WasmLimits::default()),
    )
    .expect("the slot definition is valid")
}

/// The whole path: a signed official artifact, its lock line, the trust root, the
/// admission, and a slot that only the Official lane can enter.
/// 整条路：已签名的官方工件、它的锁记录、信任根、准入，以及只有 Official 通道能进的槽。
#[cfg(feature = "wasm")]
#[test]
fn an_official_artifact_travels_the_lock_to_the_slot() {
    let key = signing_key();
    let root = package_root();
    let probe = artifact(PluginSource::Official, Some(&key), Some(fingerprint(&key)));
    let manifest = probe.registration.plugin.expect("manifest");
    write_lock(&root, OFFICIAL_LOCK, &[record("official", manifest)]);

    let admission = admission(&root, &key);
    assert_eq!(admission.catalog().records().len(), 1);
    let table = table(&[PluginChannel::Official]);
    let generation = admission
        .install(&table, "test", probe)
        .expect("an honest official artifact is admitted and installed");
    assert_eq!(generation, 1);
    assert_eq!(
        table.call("test", "echo", b"abc").expect("the plugin runs"),
        b"abc"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The signature covers the registration: moving the parent after signing — the
/// audit's tamper — no longer verifies, so the artifact never reaches a slot.
/// 签名覆盖注册声明：签名后移动 parent——审计的篡改——不再通过验证，因此工件到不了槽。
#[test]
fn a_tampered_registration_is_refused_at_the_host_seam() {
    let key = signing_key();
    let root = package_root();
    let mut probe = artifact(PluginSource::Official, Some(&key), Some(fingerprint(&key)));
    let manifest = probe.registration.plugin.expect("manifest");
    write_lock(&root, OFFICIAL_LOCK, &[record("official", manifest)]);
    probe.registration.parent = NodeId::from_path("elsewhere.rs", "Elsewhere");

    let error = admission(&root, &key)
        .admit(probe)
        .expect_err("a moved parent must not verify");
    assert!(
        error.to_string().contains("signature"),
        "the refusal must name the signature, not something else: {error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An official artifact the lock does not list is refused before any verifier is
/// asked, which is what makes the lock the host's own statement of intent.
/// 锁里没有登记的官方工件在咨询任何验证器之前就被拒绝，这正是锁作为宿主自身意图声明的作用。
#[test]
fn an_official_artifact_absent_from_the_lock_is_refused() {
    let key = signing_key();
    let root = package_root();
    let probe = artifact(PluginSource::Official, Some(&key), Some(fingerprint(&key)));
    // No lock at all: an empty catalogue, and the official lane still refuses.
    // 完全没有锁：空目录，而官方通道依然拒绝。
    assert_eq!(
        plugin_catalog(&root)
            .expect("absent locks parse")
            .records()
            .len(),
        0
    );

    let error = admission(&root, &key)
        .admit(probe)
        .expect_err("an unlisted official artifact must be refused");
    assert!(
        error.to_string().contains("lock"),
        "the refusal must name the lock: {error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A revoked version is refused even though the lock lists it and the signature is
/// good: the policy runs before the verifier.
/// 已吊销的版本即使锁里有、签名也对仍被拒绝：策略先于验证器运行。
#[test]
fn a_revoked_version_is_refused() {
    let key = signing_key();
    let root = package_root();
    let probe = artifact(PluginSource::Official, Some(&key), Some(fingerprint(&key)));
    let manifest = probe.registration.plugin.expect("manifest");
    write_lock(&root, OFFICIAL_LOCK, &[record("official", manifest)]);

    let print = fingerprint(&key);
    let admission = PluginAdmission::from_package_root(
        &root,
        PluginPolicy::strict(framework()),
        PluginTrustPolicy::official(
            trusted(print),
            Box::leak(Box::new([PluginRevocation {
                package: "plugin-test",
                version: "1.0.0",
            }])),
        ),
        verifier(print, &key),
    )
    .expect("the fixture locks parse");

    let error = admission
        .admit(probe)
        .expect_err("a revoked version must be refused");
    assert!(
        error.to_string().contains("revoked"),
        "the refusal must name the revocation: {error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A host with no trust root cannot admit an official plugin: the lane fails
/// closed instead of downgrading to a checksum.
/// 没有信任根的宿主无法准入官方插件：这条通道失败关闭，而不是降级为纯摘要。
#[test]
fn an_official_artifact_without_a_trust_root_is_refused() {
    let key = signing_key();
    let root = package_root();
    let probe = artifact(PluginSource::Official, Some(&key), Some(fingerprint(&key)));
    let manifest = probe.registration.plugin.expect("manifest");
    write_lock(&root, OFFICIAL_LOCK, &[record("official", manifest)]);

    let admission = PluginAdmission::from_package_root(
        &root,
        PluginPolicy::strict(framework()),
        PluginTrustPolicy::open(),
        verifier(fingerprint(&key), &key),
    )
    .expect("the fixture locks parse");

    let error = admission
        .admit(probe)
        .expect_err("an official plugin needs a trust root");
    assert!(
        error.to_string().contains("fingerprint"),
        "the refusal must say the trust root is missing: {error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A user plugin takes the digest path, earns `Digest`, and enters the user lane;
/// its lock line needs no signature because none is checked for that source.
/// 用户插件走纯摘要路径、拿到 `Digest`、进入用户通道；它的锁记录不需要签名，因为该来源不检查签名。
#[cfg(feature = "wasm")]
#[test]
fn a_user_artifact_takes_the_digest_lane() {
    let key = signing_key();
    let print = fingerprint(&key);
    let root = package_root();
    let probe = artifact(PluginSource::User, None, None);
    let manifest = probe.registration.plugin.expect("manifest");
    write_lock(&root, USER_LOCK, &[record("user", manifest)]);

    let admission = PluginAdmission::from_package_root(
        &root,
        PluginPolicy::strict(framework()),
        PluginTrustPolicy::open(),
        verifier(print, &key),
    )
    .expect("the fixture locks parse");

    let verified = admission.admit(probe).expect("the digest path admits it");
    assert_eq!(verified.assurance(), PluginAssurance::Digest);
    assert_eq!(lane_for(&verified), PluginChannel::Community);

    let user_lane = table(&[PluginChannel::Community]);
    admission
        .install(&user_lane, "test", artifact(PluginSource::User, None, None))
        .expect("a user artifact enters the community lane");
    assert_eq!(
        user_lane
            .call("test", "echo", b"abc")
            .expect("the plugin runs"),
        b"abc"
    );

    // The same artifact is refused by a slot that only admits the official lane:
    // the lane follows the assurance, not the caller's hopes.
    // 同一个工件会被只接纳官方通道的槽拒绝：通道跟着保证等级走，而不是跟着调用方的期望。
    let official_only = table(&[PluginChannel::Official]);
    assert!(
        admission
            .install(
                &official_only,
                "test",
                artifact(PluginSource::User, None, None)
            )
            .is_err(),
        "a digest-only artifact must not enter the official lane"
    );
    let _ = std::fs::remove_dir_all(&root);
}
