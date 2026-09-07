//! Ed25519 verification at the plugin execution boundary.
//! 插件执行边界上的 Ed25519 验证。

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use nichlink::{PluginManifest, PluginSignatureVerifier};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrustedPublicKey {
    pub fingerprint: &'static str,
    pub bytes: [u8; 32],
}

impl TrustedPublicKey {
    pub const fn new(fingerprint: &'static str, bytes: [u8; 32]) -> Self {
        Self { fingerprint, bytes }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Ed25519Verifier {
    keys: &'static [TrustedPublicKey],
}

impl Ed25519Verifier {
    pub const fn new(keys: &'static [TrustedPublicKey]) -> Self {
        Self { keys }
    }

    fn key(&self, fingerprint: &str) -> Option<VerifyingKey> {
        self.keys
            .iter()
            .find(|key| key.fingerprint.eq_ignore_ascii_case(fingerprint))
            .filter(|key| nichlink::sha256_hex(&key.bytes).eq_ignore_ascii_case(key.fingerprint))
            .and_then(|key| VerifyingKey::from_bytes(&key.bytes).ok())
    }
}

impl PluginSignatureVerifier for Ed25519Verifier {
    fn verify(&self, manifest: PluginManifest, bytes: &[u8], fingerprint: &str) -> bool {
        let Some(key) = self.key(fingerprint) else {
            return false;
        };
        let Some(signature) = manifest.signature.and_then(decode_signature) else {
            return false;
        };
        key.verify(&manifest.signing_payload(bytes), &signature)
            .is_ok()
    }
}

fn decode_signature(value: &str) -> Option<Signature> {
    if value.len() != 128 {
        return None;
    }
    let mut bytes = [0; 64];
    let (chunks, remainder) = value.as_bytes().as_chunks::<2>();
    debug_assert!(remainder.is_empty());
    for (index, pair) in chunks.iter().enumerate() {
        bytes[index] = hex(pair[0])? << 4 | hex(pair[1])?;
    }
    Some(Signature::from_bytes(&bytes))
}

fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer, SigningKey};
    use nichlink::{FrameworkId, PluginMode, PluginSource, sha256_hex};

    use super::*;

    #[test]
    fn verifies_metadata_and_artifact_bytes() {
        let signing = SigningKey::from_bytes(&[7; 32]);
        let verifying = signing.verifying_key();
        let fingerprint = Box::leak(sha256_hex(verifying.as_bytes()).into_boxed_str());
        let unsigned = manifest(None, fingerprint);
        let encoded = signing
            .sign(&unsigned.signing_payload(b"abc"))
            .to_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let key = TrustedPublicKey::new(fingerprint, verifying.to_bytes());
        let verifier = Ed25519Verifier::new(Box::leak(Box::new([key])));

        assert!(verifier.verify(
            manifest(Some(Box::leak(encoded.into_boxed_str())), fingerprint),
            b"abc",
            fingerprint,
        ));
        assert!(!verifier.verify(manifest(Some("bad"), fingerprint), b"abc", fingerprint));
    }

    fn manifest(signature: Option<&'static str>, fingerprint: &'static str) -> PluginManifest {
        PluginManifest {
            name: "fixture",
            crate_name: "fixture",
            version: "1.0.0",
            framework: FrameworkId::new("nichlink.default"),
            source: PluginSource::Official,
            mode: PluginMode::Extension,
            checksum: "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            signature,
            public_key_fingerprint: Some(fingerprint),
            revocation_list: Some("official-1"),
        }
    }
}
