# GitHub Discussion 首帖草稿

[English](discussion-introduction.en.md) | 简体中文

## 标题

NichLink：面向 Rust 对象图的递归注册与原子嫁接

## 正文

大家好，我正在开源一个 Rust 优先的实验性基础设施：NichLink。

它解决的不是“如何定义一个 trait”，而是大型对象图中经常被分散处理的几件事：

- 对象在自己的文件里声明父注册机；
- 子对象递归进入正确的 Registry；
- 注册时检查 admission、结构 rule 和输入输出合同；
- 在中间层进行原子 graft，而不要求重写整棵对象树；
- 通过 Studio、CallTrace 和 MCP 查看真实调用和数据流。

最小声明可以很短：

```rust
crate::root_object! {
    kind: Button,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
}
```

典型的中间层替换是：

```text
NodeEditor -> Canvas2D -> WGPU
```

替换 `Canvas2D` 时，新的实现需要满足原来的输入、输出和结构合同，注册机负责验证并在失败时保持原状态。

当前仓库包含：

- `nichlink-core`：Registry、合同、admission、graft 和事务；
- `nichlink-build`：目录发现、粗修和静态计划；
- `nichlink-cli`：统一命令行入口（`nichlink new/check/build/studio/mcp`）；
- `nichlink-debug`：MIR 候选、CallTrace 和数据流证据；
- `nichlink-studio`：常驻 Ratatui 调试界面；
- `nichlink-mcp`：给 AI 使用的紧凑查询入口；
- `nichlink-plugin-host`：Wasm/process 插件适配。

我想重点听到三类反馈：

1. 这种“对象自声明、父级被动接收”的注册方式，是否比手动 `add/register/wire` 更容易维护？
2. admission、结构 rule、输入输出合同和 graft 的边界是否清楚？
3. 哪个真实项目愿意尝试一个非 UI 的例子，例如 `parser -> optimizer -> codegen`？

当前限制也写在仓库里：静态调用图对动态分派和 FFI 只能给候选；未插桩局部值不能保证恢复；Process 插件不是安全沙箱；依赖 crate 的 `build.rs` 不能读取宿主 `main.rs`。

如果你愿意试用，请附上：项目类型、采用前的接线方式、最希望解决的替换或调试问题，以及没有采用的原因。反例和批评同样欢迎。

仓库：<https://github.com/OWNER/NichLink>
路线图：[`ROADMAP.md`](../ROADMAP.md)
