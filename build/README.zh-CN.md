# nichlink-build

[English](README.md) | 简体中文

`nichlink-build` 负责构建期工作：按宿主目录发现注册面，校验父子关系和需求，维护增量身份缓存，并生成 release 使用的被动 `StaticPlan`。

宿主的 `build.rs` 保持为薄入口：

```rust
fn main() {
    nichlink_build::run();
}
```

可选入口使用语法解析而不是文本搜索：

```rust
nichlink::application!(entry = crate::main);
```

只能有一个入口声明；入口的首个模块必须解析到 `src/`（包括 `src/bin/`）下的文件。格式错误、重复声明或无法解析都会带着声明位置使构建失败。
