<div align="center">

<img src="./picture/NichLink_wordmark.svg" alt="NichLink ASCII 字标">

<p><strong>一种顺应社区扩展本能、服务工程协作与 agentic coding 的 Rust 代码组织模型：被动式递归注册，显式边界合同，任意层级原子替换。</strong></p>

[![license](https://img.shields.io/github/license/Nichtigott/nichlink?style=flat-square)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/Nichtigott/nichlink/ci.yml?style=flat-square&label=CI)](.github/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-Rust%201.96-8250df?style=flat-square)](Cargo.toml)

<p>
<a href="#特性"><kbd>特性</kbd></a>
<a href="#开始使用"><kbd>开始使用</kbd></a>
<a href="#注册面"><kbd>注册面</kbd></a>
<a href="#studio-与命令行"><kbd>Studio / CLI</kbd></a>
<a href="#边界"><kbd>边界</kbd></a>
</p>

</div>

项目规模越大，一次局部改动越可能波及半个仓库。NichLink 在结构层面处
理这件事：对象携带小而明确的边界；替换在接入点完成校验；每个对象的
源码位置对人和工具保持可见。

AI 辅助与 agentic coding 放大而非制造了这种压力。模型和人类评审者都
无法对大型代码库保持完整注意力，幻觉与上下文缺失是常态失效而非意外。
NichLink 因此把通常只存在于约定中的关键假设固化为可检查的结构：原子
对象、显式合同、源码溯源，以及团队可共同阅读的开发期注册图。同一套
边界同样服务于代码评审、社区协作与普通 Rust 开发——AI 是这些结构的
消费者之一，不是架构存在的理由。

这一模型也定义了社区扩展项目的方式。贡献者在原树之外交付实现，声明
所替换的边界，由宿主完成 graft 校验；相互竞争的实现可以并存，上游源
码不必沦为 patch 队列。项目保持自身形态，试验发生在命名合同之内。

NichLink 目前处于早期阶段：核心协议已可用于真实工程，静态分析与运行
时证据均显式标注自身覆盖范围。

## 特性

- **被动、递归注册。** 每个注册面在自己的文件中声明父节点，父节点只
负责接受，不维护中央子对象清单。注册面还可以拥有下一级 `Registry`，
同一规则自然递归。
- **只有一个 Registry 类型。** 根注册机和子注册机使用同一套 API；树是
注册行为产生的，不靠“根类型/叶类型”特判。
- **边界合同。** preset/parts 的输出类型和 handle 合同可以在编译期失败；
`registry_rule` 负责结构，`admission` 负责外部依赖，发布前一次收集全部
失败项。
- **原子嫁接。** 替换面向一个逻辑槽位，并且必须匹配声明的输入/输出
`FlowContract`。验证不通过时，线上注册树保持不变。
- **两步构建。** 第一阶段按宿主源码发现可达注册面，生成更小的
`StaticPlan`；之后交给正常的 rustc、LLVM 和链接器做最终代码/符号裁剪。
前者减少注册元数据和编译范围，后者负责最终机器码体积，它们不是一件事。
- **按需调试。** `off`、`errors-only`、`full` 三档追踪让发布路径默认不收集
证据。Studio 和 MCP 使用同一套注册树、调用链和数据流模型。

## 开始使用

有两种方式可以试用 NichLink。

### 方式一：安装 Studio 工具

这种方式不会把 NichLink 源码放进你的应用目录。先安装 Ratatui 工具，再
把它指向要检查的项目：

```sh
cargo install --git https://github.com/Nichtigott/nichlink --bin nichlink-studio nichlink-studio
cd /work/my-app
NICH_LINK_PACKAGE_ROOT="$PWD" nichlink-studio
```

发布到 crates.io 后，可以把 Git 地址替换为 `cargo install nichlink-studio`。

### 方式二：直接克隆源码运行

适合参与开发或想看最新实现，不会安装全局命令：

```sh
git clone https://github.com/Nichtigott/nichlink
cd nichlink
cargo run -p nichlink-studio
```

从源码仓库检查另一个项目：

```sh
NICH_LINK_PACKAGE_ROOT=/work/my-app cargo run -p nichlink-studio
```

Studio 中按 `n` 可以新建 binary 或 library 项目。向导只写 Cargo 清单、
薄 `build.rs` 和入口文件；按 `a` 才会创建第一个注册面。它不会凭空
生成 `control` 树。只有注册面确实需要结构规范时，旁边才会生成
`registry_rule/` 目录。

### 把 NichLink 接入自己的项目

注册声明属于宿主项目，所以 Cargo 需要一个很薄的构建入口：

```sh
cargo add nichlink-core --path /path/to/nichlink/core
cargo add nichlink-build --build --path /path/to/nichlink/build
```

```rust
// build.rs
fn main() {
    nichlink_build::run();
}
```

在 crate 根部只接线一次生成计划：

```rust
pub mod registry_core {
    pub use nichlink_core::*;
}

include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"));
```

不要求必须有 `main.rs`：二进制项目以 `src/main.rs` 作为入口，前端框架
这类库项目以 `src/lib.rs` 作为入口。构建适配器扫描的是宿主自己的源码；
Cargo 不会把使用者的 `main.rs` 交给依赖 crate 的 `build.rs`，因此宿主
如果还拥有自己的注册面，就需要这一次构建接线。

## 注册面

注册面分成两层。Rust 的 `struct` 和 `impl` 是真正运行的实现；声明宏
记录它在树中的位置，以及什么对象可以进入或替换它。

```rust
pub struct Canvas;

pub struct CanvasParts;
pub struct CanvasPreset;

impl nichlink_core::PresetContract for CanvasPreset {
    type Output = CanvasParts;
    const REQUIRED_PARTS: &'static [&'static str] = &["paint"];
}

impl nichlink_core::PartsContract for CanvasParts {
    type Output = CanvasParts;
    const PROVIDED_PARTS: &'static [&'static str] = &["paint"];
}

impl Canvas {
    pub fn render(&self, input: CanvasInput) -> CanvasFrame {
        // 这里仍然是普通 Rust 实现
        input.into_frame()
    }
}

// 父级宏由构建阶段按源码层级生成。
// 例如放在 node_editor 下，就使用 node_editor_object!。
crate::node_editor_object! {
    kind: Canvas,
    preset: CanvasPreset,
    parts: CanvasParts,
    handle: Canvas,
    summary: { zh: "二维画布", en: "A 2-D drawing surface" },
    exports: ["canvas.render"],
    needs_registry: false,
    requires: ["viewport" => "layout.viewport"],
    provides: ["canvas.frame"],
    expected_output: "CanvasFrame",
    actual_output: "CanvasFrame",
    flow: nichlink_core::FlowContract::new(
        nichlink_core::ContractId::new("canvas.render.v1"),
        1,
        "CanvasInput",
        "CanvasFrame",
    ),
}
```

大多数字段都有安全默认值。最小注册面可以写成
`crate::root_object! { kind: App }`，或者在子级使用生成的
`<parent>_object! { kind: Child }`。只有确实有信息时再写 `summary`、
`exports`、`requires`、`flow`，不用为了填满表格重复默认值。

这里有三种不同的校验，故意不合并：

1. **构造合同（编译期）。** `preset` 和 `parts` 的关联输出类型必须一致；
`handle_contracts` 和 `part_contracts` 让 rustc 直接证明 handle/parts 实现了
指定 trait。
2. **注册规范（构建/发布前）。** `registry_rule` 只描述进入这个注册机的
子对象必须具备的最低结构：preset、parts、exports 和接口。它不维护 kind
白名单，也不决定谁属于这个父级。缺什么会集中报告，
不会留下半注册对象。
3. **准入门禁（依赖边界）。** `admission` 规定注册面可以使用哪些外部
注册路径。越过门禁的依赖会被拒绝，错误中会带注册面、源码位置和依赖名。

替换时，两边都要发布数据流合同。宿主先比较合同 id、版本、输入和输出，
通过后才执行嫁接：

```rust
let plan = nichlink_core::GraftPlan::command(
    framework,
    "cut root/canvas graft canvas_fast",
)?;
let effective = base.overlay(&plan, &external)?;
```

扩展对象只需满足目标注册规范；替换对象还必须和槽位的输入/输出合同
语义兼容。这样换的是中间实现，不是悄悄改了一个模块名。

### 结构规范是最低要求

registry_rule 是最低边界。注册面可以拥有比规范列出的更多字段、方法、
接口或 exports，但不能少掉任何必需项。规范并不表示“对象只能有这些内容”。
这对框架演进很重要：给已有注册面增加方法不会破坏旧使用者，删除必需 part
才会。

### 父级如何约束子注册面

父对象把规范写在自己拥有的 Registry 上。子对象不需要复制这份规范；子对象
提交时，父 Registry 会检查它的 preset、parts、exports 和接口声明。
规范只是最低结构，额外的实现细节不会造成拒绝。

```rust
pub struct Control;

pub struct ControlFrame;
pub struct ActionParts;

pub trait ControlHandle {
    fn paint(&self, parts: &ButtonParts) -> ControlFrame;
}

pub trait ActionPartsContract {
    fn action_id(&self) -> &str;
}

pub struct Button;
pub struct ButtonParts {
    pub label: String,
    pub action: String,
}

impl nichlink_core::PresetContract for ActionParts {
    type Output = ButtonParts;
    const REQUIRED_PARTS: &'static [&'static str] = &["paint"];
}

impl nichlink_core::PartsContract for ButtonParts {
    type Output = ButtonParts;
    const PROVIDED_PARTS: &'static [&'static str] = &["paint"];
}

impl ControlHandle for Button {
    fn paint(&self, parts: &ButtonParts) -> ControlFrame {
        let _label = &parts.label;
        ControlFrame
    }
}

impl ActionPartsContract for ButtonParts {
    fn action_id(&self) -> &str {
        &self.action
    }
}

// 父注册面拥有 Registry，并声明子注册面的最低结构。
crate::root_object! {
    kind: Control,
    needs_registry: true,
    registry_rule: crate::RegistrationRule::new()
        .require_preset("ActionParts")
        .require_parts(&["paint"])
        .require_exports(&["control.render"])
        .require_handle_traits(&["ControlHandle"])
        .require_part_traits(&["ActionPartsContract"]),
}

// Button 提供全部必需项，同时仍可增加自己的结构。
crate::control_object! {
    kind: Button,
    preset: ActionParts,
    parts: ButtonParts,
    handle: Button,
    exports: ["control.render"],
    handle_traits: ["ControlHandle"],
    handle_contracts: [crate::ControlHandle],
    part_traits: ["ActionPartsContract"],
    part_contracts: [crate::ActionPartsContract],
}
```

约束是从父到子生效的：`Control` 拥有的 Registry 读取自己的规则，再验证
`Button`。preset 不是 `ActionParts`、`ButtonParts::PROVIDED_PARTS` 没有
`paint`、没有 `control.render` export，或者缺少两个接口中的任意一个，都会
列入同一份结构化错误。`handle_contracts` 与 `part_contracts` 还会让 rustc
直接检查两个 `impl` 是否真的存在。Button 可以继续增加字段、方法、
trait 和 export；父规则从不把额外结构当成错误。

父子归属由 `control_object!` 和 `parent` 确定，不由规则筛选 kind。外部对象能否
被调用由 `admission` 决定。这三件事不能混在一个白名单里。

早期原型里的 `RegistrationRule::new(&["Button"], &[])` 已经删除。迁移时改成
`RegistrationRule::new()`，只追加上面这些最低结构要求。外部分支的 allow/deny
移到 `Admission`；旧的 kind 列表不需要搬过去，因为 kind 从来不负责确定父子关系。

三层含义保持分开：

Rust impl / struct       真正运行的代码和内部细节
registry_rule            父 Registry 接受的最低结构
admission                允许使用的外部注册路径
FlowContract             替换边界上的输入/输出合同

### 输出增加内容时如何兼容

当前 FlowContract 会比较稳定的合同 id、版本，以及声明的输入/输出标签；
它不会从 Rust 类型自动推断字段级协变。如果新实现返回了更多内容，可以明确
选择一种做法：

- 保持相同的输出标签，在稳定 envelope（例如 CanvasFrame）中增加向后兼容
的可选字段；
- 发布新的合同版本（例如 CanvasFrame v2），一次更新所有消费者边界；或
- 增加一个适配注册面，把更丰富的输出转换回旧合同。

后两种做法会把受影响的边界显式留下，而不是静默丢弃数据。只有选定的边界
合同和目标注册规范都通过，graft 才会接受替换。

### 单个 graft 与协调 graft 集

注册树描述的是对象归属，并不等于完整的数据流图。requires 可以解析到
另一条分支上的 provider，运行时数据边也可能跨过多个注册边界。当一次替换
会影响多个消费者时，把受影响的槽位组成一个计划，一起提交：

```rust
let plan = nichlink_core::graft_plan!(framework,
    cut ["root/canvas"] graft "canvas_fast",
    cut ["root/hit_test"] graft "hit_test_fast",
    cut ["root/layout"] graft "layout_fast",
);
let effective = base.overlay(&plan, &external)?;
```

`overlay` 会暂存整组切口。每一项都会检查源端和目标端合同、目标
registry_rule 以及连接器准入；如果仍有消费者只接受旧边界，操作失败，
不会发布只完成一部分的 effective tree。

overlay 操作不会移动源码，也不会修改原树或外部树。`cut A graft X` 只替换
`A` 这个槽位，并继承它原来的子树；`cut A full graft X` 才会用外部 `X` 的整棵
子树替换 `A`。也可以选择同一父注册机下的一段兄弟节点：

```rust
let plan = nichlink_core::GraftPlan::command(
    framework,
    "cut [root/a1 to root/a3] graft replacement",
)?;
let effective = base.overlay(&plan, &external)?;
```

返回的 effective registry 保留原树和外部树不变，维持目标的逻辑路径，自动继承
未覆盖的兄弟，并在发布前重新执行目标注册规范、准入和连接器校验。

Studio 的 graft 流程使用 `g`：它创建
`.nichlink/external-grafts/<selector>/graft.plan`，并用默认编辑器打开计划，
宿主源码不会被修改。系统不存在复制或替换宿主源码的 graft 路径。

NichLink 不限定对象内部采用哪种编程范式。普通函数、trait、泛型、闭包、
依赖注入或消息传递都可以继续使用；注册面只约束它们对外暴露的边界。声明过
的 `requires/provides` 和 `FlowContract` 会在注册、连接和 graft 时自动校验。
改动实现本身不会因为“代码和以前不一样”而失败；只有某个输入找不到合法
provider、跨过 admission 门禁，或替换端无法接回原数据流时才会失败。错误会
指出断开的 capability、消费端、候选 provider、源码位置和失败阶段，而不是只给
一句“注册失败”。这不等于猜测任意 Rust 函数内部所有未声明的数据流。

## Studio 与命令行

Studio 是常驻的 Ratatui 界面，不是不断向终端追加文本的脚本。它使用
备用屏幕，监听项目文件，只有输入、窗口变化或文件事件发生时才重绘。

![NichLink Studio](./picture/NichLink_studio.png)

| 按键 | 操作 |
| --- | --- |
| `n` | 新建 binary/library 项目 |
| `a` / `e` / `d` | 添加、编辑、删除注册面 |
| `g` | 为选中注册面创建并编辑外部 graft 计划 |
| `/` | 搜索文件和函数 |
| `1`–`4` | 搜索、检视、数据、对比页面 |
| `Tab` | 在树和详情之间切换焦点 |
| 方向键 / `j` `k` | 移动选择或调整当前面板 |
| `r` / `F5` | 重新加载项目 |
| `b` / `F9` | 执行构建检查 |
| `q` / `Ctrl-C` | 退出 |

开发 Studio 自身时，可以让监督器在源码变化后重建并重启子进程：

```sh
NICH_LINK_PACKAGE_ROOT=/work/my-app \
  cargo run -p nichlink-studio --bin nichlink-dev -- watch
```

命令行入口刻意保持精简：`cargo check` 负责构建校验，`nichlink-mcp` 是给 AI
客户端使用的只读 JSON-RPC/MCP 桥，Studio 负责交互式编辑和调试：

```sh
cargo check
NICH_LINK_PACKAGE_ROOT=/work/my-app cargo run -p nichlink-mcp
```

MCP 提供 `nichlink.search`、`nichlink.inspect`、`nichlink.callgraph`、
`nichlink.read` 和 `nichlink.status`。静态调用图会标为 heuristic；动态调用
和运行时数值以 `CallTrace` 的现场证据为准。

## 什么时候值得用 NichLink

NichLink 不是 Rust 模块系统的替代品。它适合这样的项目：对象图由多人或
工具共同维护；需要替换中间层而不改动相邻节点；或者 AI 需要一份机器能读
懂的“这个对象接受什么、提供什么”的说明。小型、集中组装的应用通常直接
使用普通 Rust 更合适。

## 对比

| 方案 | 擅长解决 | 仍需应用自己处理 |
| --- | --- | --- |
| 模块、trait、依赖注入 | 命名空间、静态行为、显式构造 | 发现、父链闭合、准入、替换策略 |
| `inventory` / `linkme` | 分布式收集静态条目 | 树语义、合同、溯源、原子嫁接 |
| Bevy 风格插件 | 显式组合一个应用 | 通用源码路径和中间层合同校验 |
| CodeGraph / CodeQL | 符号和调用证据 | 运行时注册与替换决策 |
| NichLink | 被动递归注册树、合同、准入、嫁接校验、Studio/MCP 视图 | 动态分发和优化后数值仍受 Rust/编译器边界限制 |

## 边界

- 第一阶段粗修只能看到宿主 crate 的源码，看不到下游应用的 `main.rs`；
最终二进制仍由 rustc/LLVM/链接器按可达性裁剪。
- MIR 和静态扫描提供候选证据。`dyn Trait`、函数指针、FFI、运行时选择
的调用和宏生成代码，可能只能保守保留或依赖现场证据。
- 优化后的未插桩局部变量可能显示为 `unobserved`；需要真实数值时启用
`full` 追踪。
- 进程插件能隔离崩溃和资源限制，但不是操作系统级安全沙箱。Wasm/进程
加载都是可选能力。
- Studio、debug、MCP 和 plugin-host 都是可选工具；只使用 core 时，发布
产物不包含开发界面。

## 运行时追踪

```rust
let trace = nichlink_core::CallTrace::runtime(); // debug: errors-only, release: off
let quiet = nichlink_core::CallTrace::disabled();
let detailed = nichlink_core::CallTrace::full();
```

`errors-only` 只保留失败链路并丢弃成功证据；`full` 保留 frame、局部变量和
数据边，供 Studio/MCP 查看。发布构建默认不收集，除非应用明确选择。

## 工作区结构

```text
core/         nichlink-core：Registry、合同、准入、graft、声明宏
build/        nichlink-build：源码发现、缓存、第一阶段 StaticPlan
debug/        可选 CallTrace、MIR 证据、数据流和图适配器
studio/       Ratatui 编辑、搜索、watch 和源码跳转
mcp/          面向 AI 的只读 MCP 桥
plugin-host/  可选 Wasm/进程插件和原子部署
```

技术路线见 [`docs/ROADMAP.zh-CN.md`](docs/ROADMAP.zh-CN.md)，英文版见
[`docs/ROADMAP.md`](docs/ROADMAP.md)。每个 crate 的 API 说明放在各自目录。

## 构建与许可证

```sh
cargo fmt --all
cargo test --workspace --offline
cargo clippy --workspace --all-targets -- -D warnings
```

NichLink 使用 [MIT License](LICENSE)。欢迎提交真实项目中的失败案例、设计
质疑和改进 PR，也欢迎在 GitHub Issues / Discussions 讨论边界问题。

[English](README.md) · [Roadmap](docs/ROADMAP.md) · [中文路线图](docs/ROADMAP.zh-CN.md)