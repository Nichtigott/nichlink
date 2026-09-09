# nichlink-build

[English](README.md) | 简体中文

`nichlink-build` 负责构建期工作：按宿主目录发现注册面，校验父子关系和需求，维护增量身份缓存，并生成 release 使用的被动 `StaticPlan`。

宿主的 `build.rs` 保持为薄入口：

```rust
fn main() {
    nichlink_build::run();
}
```

渲染出的计划落在 Cargo 的 `OUT_DIR` 中；宿主 crate 根部用
`nichlink_core::host!();` 引入——等价于
`include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))`——使整个 crate 内
可用 `builtin_static_plan()` 与各层级的 `{name}_object!` 别名。

可选入口使用语法解析而不是文本搜索：

```rust
nichlink::application!(entry = crate::main);
```

只能有一个入口声明；入口的首个模块必须解析到 `src/`（包括 `src/bin/`）下的文件。格式错误、重复声明或无法解析都会带着声明位置使构建失败。
