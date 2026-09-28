# nichlink-mcp

[English](README.md) | 简体中文

`nichlink-mcp` 是面向 AI 辅助开发的轻量 Model Context Protocol 服务。它通过 stdin/stdout 传输逐行 JSON-RPC，**不写 stderr**：失败是 stdout 上的错误响应——客户端本来就在读那里——因此这条流可以直接挂到 MCP 客户端。

```sh
NICH_LINK_PACKAGE_ROOT=/work/my-app nichlink mcp
```

（`nichlink` 命令由 `nichlink-cli` package 提供；也可在本仓库内
`cargo run -p nichlink-mcp` 运行。）

读取类工具：

- `nichlink.search`：查找注册面、文件与函数声明。面排在最前，按逻辑路径、kind、模块或槽位名匹配，
  每个都带上构建的结论——`ok`、`added since build`、`re-identified`（文件没动而 `kind` 变了，两个身份
  都给出），或者尚未发布任何构建时的 `build unknown`；下面的文件与函数命中与此前相同。树那一半需要身份
  命名空间，因此 Cargo 叫不出名字的根仍然回答源码那一半，并说明这一点；
- `nichlink.inspect`：查看单文件中的函数和注册声明；
- `nichlink.callgraph`：查找函数的直接调用者和被调用者；
- `nichlink.read`：读取有大小上限的源码窗口；
- `nichlink.status`：报告源码根目录和索引数量；
- `nichlink.registry`：报告本包声明的注册面——逻辑路径、kind、源码与宿主编译出的
  `NodeId`。
- `nichlink.explain`：报告构建对某个面给出的证据（身份、路径、kind、源码、模块、父级、槽位），或
  它划定作用域的整棵树投影。上面那个注册树答案由源码文本推导，因此永远新鲜；这一个读构建**发布**在
  `target/nichlink/out` 下的文件，因此回答真正会发布什么：作用域是否选中该面、发布剪枝是否剥掉它的
  符号。缺失或过期的构建会被如实报告。给出 `overlay: true` 时改为渲染**覆盖**投影——每条已声明切口
  替换哪个槽位、作用域剪掉哪些面，也就是替换之后的发布态，用的是 CLI `explain --overlay` 同一次遍历。
  那是静态投影而不是 `Registry::dump`，回复里的说明写明了活的树从哪来；`overlay` 与 `node` 互斥。
- `nichlink.diff`：源码现在与构建清单之间的面级差异——新增、消失，以及文件没动而身份变了
  （`kind` 变化就是身份变化，只有这个比较看得见）。给出 `records: true` 时改为把外部 graft 记录与
  源码对照：记录里存着它写下时针对的身份，因此槽位没动而面换了身份会被报成 `re-identified`
  （`old -> now`），而不是悄悄弄坏那条 graft；`stale`、`unmatched`（类型化切口存的是表达式，身份缺席
  时无法与"身份变了"区分）与读不了的记录各自分开。
- `nichlink.trace`：读取本包已记录的 trace artifact，并用它蕴含的无终端调用报告作答——真正跑了
  什么，这是任何静态读取都说不出的。给出 `values: true` 时，同一次读取改为回答那次运行**看见了**
  什么：记录下的局部值按捕获它们的帧分组（名字、类型、值、角色、observed 或 inferred、调用点）、
  在任何被追踪调用之前捕获的那些，以及被观察到的数据边及其变换标签；点名的东西 artifact 里没有时
  会被计数而不是丢掉。artifact 的身份会先被核验（命名空间、注册机根、每个帧的节点）；
  异树 artifact 会被按名拒绝，而不是画出来。
- `nichlink.mir`：读一个 MIR artifact——`rustc -Zunpretty=mir` 文本转储或紧凑 JSONL 形式，
  按扩展名选择——报告编译器给出的调用候选。`jsonl: true` 输出那份 JSONL：Studio 本来就能渲染、
  也能解析这条可移植通道，而工作区里从没有任何东西写出过一份。`jsonl: true` 写出的是**快照**：表头
  点名 artifact 来源树的身份命名空间与注册树根，而正是它让两份 artifact 可比——属于另一棵树的快照会被
  按名拒绝，说不出自己那棵树的那份（文本转储说不出）则让差异说明它无法排除什么。给出 `against` 时，
  那份 artifact 就是基线，回答是从它向前的调用图差异：新增与消失的关系，以及函数符号。JSONL 有一行
  畸形就整体失败；文本转储从不失败，只给出它确实包含的调用。文本的生产者是 nightly 工具链上的
  `cargo rustc -Zunpretty=mir`；artifact 缺失时工具会这么说，而不是报一棵空图。
- `nichlink.unified`：把 MIR artifact 与本包已记录的 trace 经
  `nichlink_debug_method::UnifiedCallGraph` 合并——那是两份证据唯一的汇合处——被 trace 确认的调用带
  `evidence=Live` 并**取代**它的编译器候选，其余保持 `evidence=Mir`。没有已记录的 trace 时合并仍会
  作答，并把每条关系标为编译器候选。
- `nichlink.grafts`：`.nichlink/external-grafts/<selector>/graft.plan` 下的每条外部 graft
  计划，以及宿主入口的 `static_graft_plan!` 是否声明了它针对的槽位——CLI 的 `grafts` 给代理的同一个
  答案，规则也是同一条。未声明的计划是 `NOT declared by the host entry`，并计入 `unkept plans`：
  发布态会剪掉那个槽位，记录永远无法生效，而 `cargo build` 拒绝的正是这件事。读不了的计划带上原因；
  入口读不了时状态是 `unknown`，而不是错误的 `not declared`。
- `nichlink.impact`：改动一个面的传递爆炸半径，走这棵树真正声明的三种依赖——它的后代、`requires`
  点名了它所提供能力的面，以及把它交出去的已声明 graft 切口。每个到达的节点带上最短跳数、所有到达它的
  理由与跳的链条。能力环被计数而不是被反复走；没走到的面会被如实报成在 `depth` 内未到达，而那并不等于
  独立。
- `nichlink.usages`：一个面的邻域——它在树里的父级与子面、`nichlink.apply` 作为输入接受的那些字段
  从生成模块里读回的结果（preset、parts、名称、exports、`requires`、`provides`、handle 与 part 的
  traits/contracts、registration rule、admission、flow、runtime checks），以及哪些别的面提到同一批
  能力记号。能力匹配发生在声明的记号上而不是一棵已解析的图，回复里写明了这一点；手写模块没有生成的
  字段清单，因此那些面被计为不可读，而不是被显示成空的。
- `nichlink.converge`：代理着手处理一个面所需的全部，集中在一个答案里——构建的作用域与剪枝判断、
  树的边、声明的字段、每条 `capability=>ProviderKind` 需求是否真的有答案（有就点名是谁）、该读哪些
  文件，以及细节在哪个工具里。当本包自己的面**确实**被内核拒绝时，那次拒绝在这里就是判断本身而不是
  一个错误：它已经点名了出问题的节点与源码位置，而那正是这个答案最重要的时刻。给出 `trace: true` 时
  它从已记录的运行出发——既声明了面、又真的跑过的那些文件，落在各自的帧，以及落在任何声明面之外的帧，
  外加那些帧捕获的值与它们之间被观察到的边，因此这份答案带的是证据而不只是位置。帧是按源文件匹配到面的，
  回复里也这么写，因为面是声明、而帧是正在活动的函数。
- `nichlink.verify`：对本包运行内核的注册校验，并报告那次运行刚刚发布的树差异。它驱动 CLI 的 `check`
  所驱动的同一个入口，因此它的判断不可能与 `nichlink check` 漂移；它还顺带刷新构建证据——这正是差异
  描述的是"刚刚被校验的那棵树"的原因。判断失败是答案而不是工具故障：回复写出 `verdict failed` 并带上
  诊断（阶段、节点、源码与行），而 `isError` 仍为 false，因为校验本身成功了。

写入类工具：

- `nichlink.apply`：经**与 Studio 相同的 authoring 执行器**编辑注册面，因此内核的准入、父规则
  与拓扑校验都会作用在这次改动上。`action` 为 `add`（在 `parent` 下创建 `fields.module`）、
  `edit`（只改请求点名的 `fields`，其余保持——会先读回该面，因此一次只改两个字段的请求不会抹掉
  另外二十一个）、`rename`（改 `fields.module`，其余字段全保留）或 `delete`（把模块移入 NichLink
  的可恢复回收目录）。`node` 命名该面、`parent` 命名父级，都可用逻辑路径或身份——`nichlink.registry`
  报告的路径可以直接用，而那是**注册树**路径（`root/control/button`），不是文件的路径
  （`control/object/button/button.rs`）；拿后者去瞄准会被按名拒绝。**除非 `apply: true`，请求只做
  预览**：预览在一份一次性的包副本上运行真实
  操作，返回文件 diff 与将得到的注册树；只有 `apply: true` 才写入项目，给出它写下的文件，并把产生的
  声明锚成 `<path>:<line>`——与拒绝时 `file:line:column` 同一种形状。每条回复都以当前这棵树收尾，
  因此下一次调用可以用它来瞄准。

源码类工具索引 Rust 源码文本——`nichlink.search` 也会匹配注册面，用的是与注册树工具同一份推导；
`nichlink.registry` 报告的注册树来自**构建自己的推导**
（`nichlink_build_method::face_views`，也就是 CLI 的 `explain` 所用的那一份），因此代理可以直接问
注册树是什么，而不是靠 grep 宏名重建它。两者都需要 Cargo 回答一件事：包名就是 `NodeId` 命名空间，
所以它们调用 `nichlink_build_method::package_name`（`cargo metadata`）。`NICH_LINK_NAMESPACE`
一旦设置就原样胜出；两者都拿不到时它们**拒绝作答**，而不是回落到 `nichlink.default`——一个在默认
命名空间下报告的身份，指的是宿主从未编译过的节点。

写入路径没有第二份编辑实现：它把包自己的注册面载入 `Registry`，用已解析的根与命名空间装上
`AuthoringContext`，然后调用执行器。它新增的是**预览契约**——在副本上运行真实操作——这也正是桥
仍然无法误伤项目的原因。

执行器自己的边界被原样继承：它重写的是 **NichLink 生成**的面（带生成标记的文件），对手写的面以
`this module was not generated by NichLink` 拒绝，因为重写一个并非它创作的文件会丢掉它并未建模的
内容。`add` 在任何包里都可用；`edit`、`rename`、`delete` 针对生成的面。

contract、admission 与 registration rule 的**数据**仍然不报告：那些住在已构建的
`RegistrationSnapshot` 里而不是源码里，需要构建产物而不是扫描（`docs/roadmap-1.0.md` 第 10 条）。
graft 写入、插件、项目脚手架、树 diff 与一致性分析仍待做（`docs/roadmap-1.0.md` 第 7 条）。

调用图标记为 `static-heuristic`。动态分派、函数指针、FFI 和运行时选择的调用不保证静态解析，应结合 `nichlink-debug-method` 和实时 `CallTrace`——MIR 转储与已记录的 trace 都在手边时，`nichlink.unified` 做的正是这件事。
