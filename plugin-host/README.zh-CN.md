# nichlink-plugin-host

[English](README.md) | 简体中文

`nichlink-plugin-host` 在插件进入 Registry 前校验并部署 artifact。默认的 `wasm` 能力提供带 fuel 计量的执行，并受线性内存、**表元素**、工件字节数以及引擎自身的严格编译限制约束——表是一块独立的、即时实例化的数组，因此单靠内存上限约束不了它。启用 `process-tools` 后可使用带超时的进程适配器。`HotDeployment` 会先验证 graft，再一次性发布；失败时保留最后一个健康快照。

## 准入：从宿主的锁到可加载工件

`PluginAdmission` 是宿主侧那条从"宿主写下的锁"到"可安装工件"的路。它读
`<package_root>/.nichlink/plugins/official.lock` 与 `user.lock`——文件缺失是空目录而不是错误
——按这份目录筛选 manifest，然后校验：官方工件必须在配置好的信任根下通过 `verify_signed`，
用户工件走纯摘要路径。`admit` 返回已验证工件，`lane_for` 给出它换来的保证等级所对应的通道，
`install`（Wasm）或 `load_process`（进程）一步完成准入与加载。

锁记录是 `source|framework|package|version|crate|checksum|mode`，其后可选
`|signature|key-fingerprint|revocation-list`。末尾三个字段是**期望**：记录携带某个值就把它钉住，
没写就交给签名校验，因此宿主自己的插件界面写下的锁也能准入已签名的官方插件。锁里没有登记的官方
插件在咨询任何验证器之前就被拒绝，已吊销版本先于签名被拒绝，没有信任根的宿主拿到
`MissingOfficialKey` 而不是被静默降级为纯摘要，而签名覆盖随插件字节同行的注册声明。

## Wasm ABI 握手

Wasm 插件可以导出 `nichlink_abi_version() -> i32`。宿主会在接受实例前与 `nichlink::plugin::PLUGIN_ABI_VERSION` 比较。缺少导出视为旧插件并保持兼容；导出但版本不同则返回 `HostError::Abi`。

ABI 版本只描述线协议。资源限制、健康检查和操作名称仍会独立校验，握手不能绕过这些检查。
