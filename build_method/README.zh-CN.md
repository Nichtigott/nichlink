# nichlink-build-method

[English](README.md) | 简体中文

`nichlink-build-method` 是 build_method 执行面：文件系统与 `OUT_DIR`
编排。它从宿主 crate 的目录布局发现注册面，喂给 kernel 校验器（父拓扑、
能力需求、合同、稳定标识），维护增量身份缓存，并渲染发布构建消费的被动
`StaticPlan`。校验规则与诊断模型本体在 `nichlink-core`；本 crate 只读
文件、写产物。

在宿主 crate 的 `build.rs` 里调用 `nichlink_build_method::run()`。构建
使用宿主包的 `CARGO_PKG_NAME` 作为身份命名空间，与声明宏捕获的身份一致。
在 Cargo 之外，CLI 通过 `run_for(manifest, out_dir, package)` 显式固定
命名空间——无需修改进程环境。

渲染出的计划落在 Cargo 的 `OUT_DIR`。宿主 crate 根部用
`nichlink_run_method::host!();` 引入——等价于
`include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))`——使
`builtin_static_plan()` 与各级 `{name}_object!` 别名在全 crate 可用。

可选的宿主入口按 Rust 语法解析，而不是文本搜索：

```rust
nichlink_run_method::application!(entry = crate::main);
```

必须有且仅有一条声明，其首个模块必须解析到 `src/`（含 `src/bin/`）下
的源文件。损坏、重复或不可解析的入口会让构建失败并给出声明位置。

设置 `NICH_LINK_ENTRY` 时它胜过以上全部：相对值相对包根解析，指不到文件时
构建失败而不是回退。构建只解析入口一次，并把同一个值交给 `SourceScope` 剪枝
与生成的 `BUILTIN_GRAFT_CUTS` 表，因此两者绝不可能描述不同的文件。

`SourceScope` 修剪保证社区面存活：graft 切口目标与插件声明面被强制
计为存活根，minimal 树绝不会剪掉 graft 计划稍后替换的槽位。

构建不应用的 `.nichlink/external-grafts/` 计划无法靠自己保活槽位。当存在
一条计划而**没有**任何声明可能命名其目标时，构建**失败**，并交出可直接粘贴的
`static_graft_plan!` 子句：这条记录永远无法生效，而发布一个嫁接静默不发生的二进制，
正是绝不能离开构建的东西。本次构建里 `#[cfg]` 关掉的声明也算数，因此合法门控的槽位
不会报错。记录→覆盖层见 [`docs/graft.md`](https://github.com/Nichtigott/nichlink/blob/main/docs/graft.md)。
