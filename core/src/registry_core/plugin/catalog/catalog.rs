//! Plugin lock records and catalog matching.
//! 插件锁记录与清单匹配。

use super::*;
use crate::registry_core::identity::IDENTITY_SCHEMA;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginRecord {
    pub source: PluginSource,
    pub framework: String,
    pub package: String,
    pub version: String,
    pub crate_name: String,
    pub checksum: String,
    pub mode: PluginMode,
    pub signature: Option<String>,
    pub public_key_fingerprint: Option<String>,
    pub revocation_list: Option<String>,
}

/// Parsed plugin selections kept separate from the Rust linking entries.
/// 与 Rust 链接入口分离的插件选择记录。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PluginCatalog {
    records: Vec<PluginRecord>,
}

impl PluginCatalog {
    pub fn parse(lock: &str) -> Result<Self, String> {
        let mut records = Vec::new();
        let mut identities = BTreeSet::new();
        let mut schema = None;
        for (line_number, line) in lock.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(value) = line.strip_prefix("# nichlink-schema=") {
                schema = Some(value.trim());
                continue;
            }
            if line.starts_with('#') {
                continue;
            }
            if let Some(version) = schema {
                if version != IDENTITY_SCHEMA {
                    return Err(format!(
                        "plugin lock uses identity schema {version}, expected {IDENTITY_SCHEMA}"
                    ));
                }
            }
            let fields = line.split('|').collect::<Vec<_>>();
            if fields.len() != 7 && fields.len() != 10 {
                return Err(format!(
                    "plugin lock line {} must contain 7 or 10 fields",
                    line_number + 1
                ));
            }
            let source = PluginSource::parse(fields[0])
                .ok_or_else(|| format!("unknown plugin source `{}`", fields[0]))?;
            let mode = PluginMode::parse(fields[6])
                .ok_or_else(|| format!("unknown plugin mode `{}`", fields[6]))?;
            if fields[1..6].iter().any(|field| field.is_empty()) {
                return Err(format!(
                    "plugin lock line {} contains an empty field",
                    line_number + 1
                ));
            }
            let record = PluginRecord {
                source,
                framework: fields[1].to_owned(),
                package: fields[2].to_owned(),
                version: fields[3].to_owned(),
                crate_name: fields[4].to_owned(),
                checksum: fields[5].to_owned(),
                mode,
                signature: fields
                    .get(7)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string),
                public_key_fingerprint: fields
                    .get(8)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string),
                revocation_list: fields
                    .get(9)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string),
            };
            let identity = (
                record.source,
                record.framework.clone(),
                record.package.clone(),
                record.version.clone(),
                record.crate_name.clone(),
            );
            if !identities.insert(identity) {
                return Err(format!(
                    "plugin lock line {} duplicates package identity `{}` `{}`",
                    line_number + 1,
                    record.package,
                    record.version
                ));
            }
            records.push(record);
        }
        Ok(Self { records })
    }

    pub fn records(&self) -> &[PluginRecord] {
        &self.records
    }

    pub fn contains(&self, candidate: &PluginRecord) -> bool {
        self.records.iter().any(|record| record == candidate)
    }

    /// Check an embedded manifest against the lock record for its source.
    /// 将嵌入注册面的 manifest 与对应来源锁文件中的记录比对。
    pub fn contains_manifest(&self, manifest: PluginManifest) -> bool {
        self.records.iter().any(|record| {
            record.source == manifest.source
                && record.framework == manifest.framework.0
                && record.package == manifest.name
                && record.version == manifest.version
                && record.crate_name == manifest.crate_name
                && record.checksum == manifest.checksum
                && record.mode == manifest.mode
                && record.signature.as_deref() == manifest.signature
                && record.public_key_fingerprint.as_deref() == manifest.public_key_fingerprint
                && record.revocation_list.as_deref() == manifest.revocation_list
        })
    }
}
