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
- `nichlink.status`：报告源码根目录和索引数量。

这五个工具只索引 Rust 源码文本。本包目前不提供注册树或合同查询：早前的描述
曾宣称具备该能力，但没有任何工具会载入注册快照或报告 contract、admission、
registration rule 数据。`src/tools.rs` 中的 `TODO(registry-tool)` 记录了实现
这类工具所需的资源。

调用图标记为 `static-heuristic`。动态分派、函数指针、FFI 和运行时选择的调用不保证静态解析，应结合 `nichlink-debug-method` 和实时 `CallTrace`。
