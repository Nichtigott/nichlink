# nichlink-mcp

[English](README.md) | 简体中文

`nichlink-mcp` 是面向 AI 辅助开发的轻量 Model Context Protocol 服务。它通过 stdin/stdout 传输逐行 JSON-RPC，把诊断写到 stderr，能直接挂到 MCP 客户端。

```sh
NICH_LINK_PACKAGE_ROOT=/work/my-app cargo run -p nichlink-mcp
```

只读工具包括：

- `nichlink.search`：搜索文件和函数声明；
- `nichlink.inspect`：查看单文件中的函数和注册声明；
- `nichlink.callgraph`：查找函数的直接调用者和被调用者；
- `nichlink.read`：读取有大小上限的源码窗口；
- `nichlink.status`：报告源码根目录和索引数量。

调用图标记为 `static-heuristic`。动态分派、函数指针、FFI 和运行时选择的调用不保证静态解析，应结合 `nichlink-debug` 和实时 `CallTrace`。
