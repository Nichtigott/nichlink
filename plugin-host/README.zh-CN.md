# nichlink-plugin-host

[English](README.md) | 简体中文

`nichlink-plugin-host` 在插件进入 Registry 前校验并部署 artifact。默认的 `wasm` 能力提供 fuel 和内存限制；启用 `process-tools` 后可使用带超时的进程适配器。`HotDeployment` 会先验证 graft，再一次性发布；失败时保留最后一个健康快照。

## Wasm ABI 握手

Wasm 插件可以导出 `nichlink_abi_version() -> i32`。宿主会在接受实例前与 `nichlink::PLUGIN_ABI_VERSION` 比较。缺少导出视为旧插件并保持兼容；导出但版本不同则返回 `HostError::Abi`。

ABI 版本只描述线协议。资源限制、健康检查和操作名称仍会独立校验，握手不能绕过这些检查。
