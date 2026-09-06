//! Plugin manifest metadata.
//! 插件 manifest 元数据。

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PluginManifest {
    pub name: &'static str,
    pub crate_name: &'static str,
    pub version: &'static str,
    pub framework: FrameworkId,
    pub source: PluginSource,
    pub mode: PluginMode,
    pub checksum: &'static str,
    /// Hex-encoded Ed25519 signature over `signing_payload`, when supplied.
    /// `signing_payload` 上的十六进制 Ed25519 签名；插件没有签名时为 None。
    pub signature: Option<&'static str>,
    /// Fingerprint of the public key used for the signature.
    /// 用于签名的公钥指纹。
    pub public_key_fingerprint: Option<&'static str>,
    /// Identifier of the revocation-list snapshot used by the publisher.
    /// 发布者使用的撤销列表快照标识。
    pub revocation_list: Option<&'static str>,
}

impl PluginManifest {
    pub fn targets(self, framework: FrameworkId) -> bool {
        self.framework.0 == framework.0
    }

    /// Verify the manifest digest against plugin bytes before native loading.
    /// 在 native 加载前，用插件字节验证 manifest 摘要。
    pub fn verify_bytes(self, bytes: &[u8]) -> bool {
        let expected = self
            .checksum
            .strip_prefix("sha256:")
            .unwrap_or(self.checksum);
        expected.len() == 64 && crate::sha256_hex(bytes).eq_ignore_ascii_case(expected)
    }

    /// Build the canonical bytes covered by an official plugin signature.
    /// 构造官方插件签名覆盖的规范化字节。
    pub fn signing_payload(self, bytes: &[u8]) -> Vec<u8> {
        let source = match self.source {
            PluginSource::Official => "official",
            PluginSource::User => "user",
        };
        let mode = match self.mode {
            PluginMode::Extension => "extension",
            PluginMode::Replacement => "replacement",
        };
        let fields = [
            self.name,
            self.crate_name,
            self.version,
            self.framework.0,
            source,
            mode,
            self.checksum,
            self.public_key_fingerprint.unwrap_or(""),
            self.revocation_list.unwrap_or(""),
        ];
        let mut payload = Vec::with_capacity(bytes.len() + 128);
        for field in fields {
            payload.extend_from_slice(&(field.len() as u64).to_le_bytes());
            payload.extend_from_slice(field.as_bytes());
        }
        payload.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        payload.extend_from_slice(bytes);
        payload
    }
}
