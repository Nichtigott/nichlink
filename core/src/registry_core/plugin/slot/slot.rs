//! Runtime plugin-slot validation shared by every execution adapter.
//! 各执行适配器共用的运行时插件槽校验。
//!
//! Pure policy only: channel admission, framework/mode matching, and flow
//! contract comparison. Adapters map `SlotValidationError` to their own error
//! type and own all I/O.
//! 纯策略：通道准入、framework/mode 匹配与流合同比对。适配器把
//! `SlotValidationError` 映射到各自的错误类型，并负责所有 I/O。

use super::*;

/// Trust lane enabled for one runtime plugin slot.
/// 运行时插件槽允许使用的信任通道。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginChannel {
    Official,
    Community,
    Local,
}

/// Why a plugin artifact was rejected for one slot.
/// 插件工件被某个插件槽拒绝的原因。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotValidationError {
    /// Trust-lane or manifest policy violation.
    /// 信任通道或清单策略违规。
    Policy(String),
    /// Data-flow contract mismatch.
    /// 数据流合同不匹配。
    Contract(String),
}

/// Check the operation name shared by every plugin adapter.
/// 校验各插件适配器共用的操作名。
///
/// Operation names become `{prefix}_{operation}` Wasm export or process
/// entrypoint names, so they must be non-empty ASCII identifiers.
/// 操作名会成为 `{prefix}_{operation}` Wasm 导出或进程入口名，因此必须是
/// 非空 ASCII 标识符。
pub fn validate_operation_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(format!("invalid plugin operation `{name}`"));
    }
    Ok(())
}

/// Validate one verified artifact against a runtime slot definition.
/// 按运行时插件槽定义校验一个已验证工件。
pub fn validate_artifact(
    slot_name: &str,
    slot_framework: FrameworkId,
    slot_mode: PluginMode,
    slot_contract: FlowContract,
    slot_channels: &[PluginChannel],
    channel: PluginChannel,
    artifact: &VerifiedPluginArtifact,
) -> Result<(), SlotValidationError> {
    if !slot_channels.contains(&channel) {
        return Err(SlotValidationError::Policy(format!(
            "plugin slot `{}` does not allow the {channel:?} channel",
            slot_name
        )));
    }
    let registration = artifact.registration();
    let manifest = registration
        .plugin
        .ok_or_else(|| SlotValidationError::Policy("verified plugin has no manifest".to_owned()))?;
    if manifest.framework != slot_framework {
        return Err(SlotValidationError::Policy(format!(
            "plugin targets `{}`, slot `{}` belongs to `{}`",
            manifest.framework, slot_name, slot_framework
        )));
    }
    if manifest.mode != slot_mode {
        return Err(SlotValidationError::Policy(format!(
            "plugin mode {:?} does not match slot mode {:?}",
            manifest.mode, slot_mode
        )));
    }
    match channel {
        PluginChannel::Official
            if manifest.source != PluginSource::Official
                || artifact.assurance() != PluginAssurance::Signature =>
        {
            return Err(SlotValidationError::Policy(
                "official plugins require an official manifest and verified signature".to_owned(),
            ));
        }
        PluginChannel::Community | PluginChannel::Local
            if manifest.source != PluginSource::User =>
        {
            return Err(SlotValidationError::Policy(
                "community and local channels accept user manifests only".to_owned(),
            ));
        }
        _ => {}
    }
    if slot_mode == PluginMode::Replacement && !registration.flow.is_declared() {
        return Err(SlotValidationError::Contract(
            "replacement plugin has no flow contract".to_owned(),
        ));
    }
    if slot_contract.is_declared()
        && !registration
            .flow
            .semantically_compatible_with(slot_contract)
    {
        return Err(SlotValidationError::Contract(format!(
            "plugin flow {:?} does not match slot flow {:?}",
            registration.flow, slot_contract
        )));
    }
    Ok(())
}
