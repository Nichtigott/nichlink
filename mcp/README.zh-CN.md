# nichlink-mcp

[English](README.md) | 简体中文

`nichlink-mcp` 是面向 AI 辅助开发的轻量 Model Context Protocol 服务。它通过 stdin/stdout 传输逐行 JSON-RPC，**不写 stderr**：失败是 stdout 上的错误响应——客户端本来就在读那里——因此这条流可以直接挂到 MCP 客户端。

```sh
NICH_LINK_PACKAGE_ROOT=/work/my-app nichlink mcp
```

（`nichlink` 命令由 `nichlink-cli` package 提供；也可在本仓库内
`cargo run -p nichlink-mcp` 运行。）

只读工具包括：

- `nichlink.search`：搜索文件和函数声明；
- `nichlink.inspect`：查看单文件中的函数和注册声明；
- `nichlink.callgraph`：查找函数的直接调用者和被调用者；
- `nichlink.read`：读取有大小上限的源码窗口；
- `nichlink.status`：报告源码根目录和索引数量；
- `nichlink.registry`：报告本包声明的注册面——逻辑路径、kind、源码与宿主编译出的
  `NodeId`。

前五个工具索引 Rust 源码文本；`nichlink.registry` 是第一个不这么做的：它报告的注册树来自
**构建自己的推导**（`nichlink_build_method::face_views`，也就是 CLI 的 `explain` 所用的那一份），
因此代理可以直接问注册树是什么，而不是靠 grep 宏名重建它。它仍在**只读**之列，也仍然需要
Cargo 回答一件事：包名就是 `NodeId` 命名空间，所以它调用
`nichlink_build_method::package_name`（`cargo metadata`）。`NICH_LINK_NAMESPACE` 一旦设置就原样
胜出；两者都拿不到时它**拒绝作答**，而不是回落到 `nichlink.default`——一个在默认命名空间下报告的
身份，指的是宿主从未编译过的节点。

contract、admission 与 registration rule 数据仍然没有：那些住在已构建的 `RegistrationSnapshot`
里而不是源码里，需要构建产物而不是扫描（`docs/roadmap-1.0.md` 第 10 条）。

调用图标记为 `static-heuristic`。动态分派、函数指针、FFI 和运行时选择的调用不保证静态解析，应结合 `nichlink-debug-method` 和实时 `CallTrace`。
