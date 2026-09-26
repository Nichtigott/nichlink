# nichlink-mcp

[English](README.md) | 简体中文

`nichlink-mcp` 是面向 AI 辅助开发的轻量 Model Context Protocol 服务。它通过 stdin/stdout 传输逐行 JSON-RPC，**不写 stderr**：失败是 stdout 上的错误响应——客户端本来就在读那里——因此这条流可以直接挂到 MCP 客户端。

```sh
NICH_LINK_PACKAGE_ROOT=/work/my-app nichlink mcp
```

（`nichlink` 命令由 `nichlink-cli` package 提供；也可在本仓库内
`cargo run -p nichlink-mcp` 运行。）

读取类工具：

- `nichlink.search`：搜索文件和函数声明；
- `nichlink.inspect`：查看单文件中的函数和注册声明；
- `nichlink.callgraph`：查找函数的直接调用者和被调用者；
- `nichlink.read`：读取有大小上限的源码窗口；
- `nichlink.status`：报告源码根目录和索引数量；
- `nichlink.registry`：报告本包声明的注册面——逻辑路径、kind、源码与宿主编译出的
  `NodeId`。

写入类工具：

- `nichlink.apply`：经**与 Studio 相同的 authoring 执行器**编辑注册面，因此内核的准入、父规则
  与拓扑校验都会作用在这次改动上。`action` 为 `add` 或 `edit`，`fields` 携带面的字段，`parent`
  按逻辑路径或身份命名父级——`nichlink.registry` 报告的路径可以直接用。**除非 `apply: true`，
  请求只做预览**：预览在一份一次性的包副本上运行真实操作，返回文件 diff 与将得到的注册树；只有
  `apply: true` 才写入项目，并给出它写下的文件。每条回复都以当前这棵树收尾，因此下一次调用可以
  用它来瞄准。

前五个工具索引 Rust 源码文本；`nichlink.registry` 报告的注册树来自**构建自己的推导**
（`nichlink_build_method::face_views`，也就是 CLI 的 `explain` 所用的那一份），因此代理可以直接问
注册树是什么，而不是靠 grep 宏名重建它。两者都需要 Cargo 回答一件事：包名就是 `NodeId` 命名空间，
所以它们调用 `nichlink_build_method::package_name`（`cargo metadata`）。`NICH_LINK_NAMESPACE`
一旦设置就原样胜出；两者都拿不到时它们**拒绝作答**，而不是回落到 `nichlink.default`——一个在默认
命名空间下报告的身份，指的是宿主从未编译过的节点。

写入路径没有第二份编辑实现：它把包自己的注册面载入 `Registry`，用已解析的根与命名空间装上
`AuthoringContext`，然后调用执行器。它新增的是**预览契约**——在副本上运行真实操作——这也正是桥
仍然无法误伤项目的原因。

contract、admission 与 registration rule 的**数据**仍然不报告：那些住在已构建的
`RegistrationSnapshot` 里而不是源码里，需要构建产物而不是扫描（`docs/roadmap-1.0.md` 第 10 条）。
`delete`、`rename`、graft 写入、插件、项目脚手架、树 diff 与一致性分析仍待做（`docs/roadmap-1.0.md`
第 7 条）。

调用图标记为 `static-heuristic`。动态分派、函数指针、FFI 和运行时选择的调用不保证静态解析，应结合 `nichlink-debug-method` 和实时 `CallTrace`。
