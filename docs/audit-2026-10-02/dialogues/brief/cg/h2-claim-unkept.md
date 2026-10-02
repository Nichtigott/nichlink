# cg · `h2-claim-unkept`（精简版；完整版见 `../cg/h2-claim-unkept.md`）

**题面**：hardbug h2：采信未落实——真台账 + 一个同族只缺一条声明 + 一条指纹失效。

> 仪器调用 **7** · 步 6 · 输出 13,069 tok · 推理 34,906 字符 · 累计上下文 1,201,280

## 仪器调用序列（工具给了什么 + 当时 agent 在想什么）

### ①1 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept && codegraph init .`  · exit 0 · step 57（turn 1）

**工具返回**：
```
┌  Initializing CodeGraph
│
◆  Initialized in /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept
│
Scanning files...
Parsing code...
Resolving refs...
Linking dynamic dispatch...
│
◆  Indexed 24 files
│
●  95 nodes, 121 edges in 534ms
│
└  Done
```

**当时推理**：
```
None of my hypotheses match `edc72845…`. So the fingerprint scheme is something else (likely the toolchain's own NodeId hash — e.g. sha256 over the registration-face record: file path + line + kind + ... in some canonical form defined in kernel/src/registry_core/identity).

Can I determine it without grepping the workspace? I could use the session's codegraph MCP tool on the workspace (allowed by my toolset, but would contaminate the frozen-CLI-only arm). Hmm.

Alternatively, reason from the brief: the h2 class explicitly says: "真标本台账 + 一个同族只缺一条声明 + 一条指纹失效的条目" — "one ledger entry whose fingerprint has lapsed". The ledger's own note distinguishes them: button = "the reference sibling shape ... the shape every sibling follows"; panel = "recorded before the panel was rewritten". Since both entries carry the *same* fingerprint and the two files differ, at most one can be current. The panel's note says it was recorded before a rewrite ⇒ the panel's entry is the lapsed one; the button's entry is the specimen ("真标本"), i.e. its fingerprint is the one that matches the button's current shape.

That's a sound diagnosis without needing the exact hash function: the evidence is that the two fingerprints are identical while the two named files' bytes differ (sha256 06c45576 vs d886e9b0 per file bytes), so the ledger cannot describe both; and the panel entry's own note says it predates the rewrite ⇒ the panel entry no longer describes the bytes it names (its fingerprint equals the button's, 
…[推理：此处截断，全文 6043 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①2 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept && codegraph files`  · exit 0 · step 60（turn 1）

**工具返回**：
```
Project Structure (24 files):

├── graft
│   └── src
│       ├── badge_fast.rs (rust, 3 symbols)
│       ├── button_fast.rs (rust, 3 symbols)
│       ├── dial_fast.rs (rust, 3 symbols)
│       ├── gauge_fast.rs (rust, 3 symbols)
│       ├── lib.rs (rust, 4 symbols)
│       ├── panel_fast.rs (rust, 3 symbols)
│       ├── slider_fast.rs (rust, 3 symbols)
│       ├── spinner_fast.rs (rust, 3 symbols)
│       ├── timeline_fast.rs (rust, 3 symbols)
│       └── toggle_fast.rs (rust, 3 symbols)
└── host
    ├── src
    │   ├── control
    │   │   ├── object
    │   │   │   ├── badge
    │   │   │   │   └── badge.rs (rust, 5 symbols)
    │   │   │   ├── button
    │   │   │   │   └── button.rs (rust, 5 symbols)
    │   │   │   ├── dial
    │   │   │   │   └── dial.rs (rust, 5 symbols)
    │   │   │   ├── gauge
    │   │   │   │   └── gauge.rs (rust, 5 symbols)
    │   │   │   ├── panel
    │   │   │   │   └── panel.rs (rust, 5 symbols)
    │   │   │   ├── slider
    │   │   │   │   └── slider.rs (rust, 5 symbols)
    │   │   │   ├── spinner
    │   │   │   │   └── spinner.rs (rust, 5 symbols)
    │   │   │   ├── timeline
    │   │   │   │   └── timeline.rs (rust, 5 symbols)
    │   │   │   └── toggle
    │   │   │       └── toggle.rs (rust, 5 symbols)
    │   │   ├── registry_rule
    │   │   │   └── registry_rule.rs (rust, 3 symbols)
    │   │   └── control.rs (rust, 7 symbols)
    │   └── lib.rs (rust, 4 symbols)
    ├── tests
    │   └── offsets.rs (rust, 3 symbols)
    └── build.rs (rust, 2 symbols)
```

**当时推理**：
```
Now h2-claim-unkept. Content:

① 症状：树能编译、测试全绿（offsets 1 passed），但台账里两条记录写了同一个指纹，而其中一个同族少了一条声明。
② `root cause`：
   (a) 台账失效条目 = `host/.nichlink/adopted/entries` 第 2 行（`root/control/panel` … `edc72845…` … "recorded before the panel was rewritten"）：它与第 1 行（button 标本）指纹逐字相同，而被指名的两个文件字节不同（button.rs sha256=06c45576…，panel.rs=d886e9b0…）⇒ 至少一条不描述它指名的字节；panel 那条自己写着"在 panel 被重写之前记录"，时间戳 2026-09-20 早于 button 的 2026-10-01 ⇒ 失效的是 panel 那条。
   (b) 少一条声明的兄弟 = `host/src/control/object/spinner/spinner.rs:19-23` 的 `control_object!`：只有 kind/exports/parent 三条，缺 `handle_contracts: [crate::control::ControlHandle],`（参照 `button.rs:23`）。
   `mechanism`：约束来自 `control.rs:13-16`（"The interface every direct child must implement"）——每个兄弟在自己的注册面里复述一遍 `handle_contracts`；spinner 少了这一条，而 `registry_rule.rs:5-6` 只 `require_exports(&["control.render"])`，没有任何东西要求 handle_contracts ⇒ 缺失不被门禁抓到，测试全绿。
③ `fix`：spinner.rs:23 前补 `handle_contracts: [crate::control::ControlHandle],`；台账里把 panel 那条的指纹按现存字节重录（或删除该失效条目）。不动 `control.rs`、不动 `registry_rule.rs`、不动 offsets 测试。
④ `counter-proof`：`diff` 把 8 个兄弟逐个与 button 比，只有 spinner 多出一个"缺行"hunk（其它 7 个只差名字替换）⇒ "恰好缺一条"是可核对的；台账侧：若两条记录其实都还描述字节，指纹就不该相同而文件字节不同（两个 sha256 都算过）；若失效的是 button 那条，panel 那条的 note/time 就失去意义——两条互斥，note 与时间戳指向 panel。
   `evidence`：`cat host/.nichlink/adopted/entries`（rc=0）；`sha256sum`（rc=0，见答案：06c45576…/d886e9b0…）；`node …/spinner/spinner.rs`（rc=0，块 19-23 缺一行）；`cargo test --offline` → `the_rendered_offsets_add_up ... ok`、`test result: ok. 1 passed; 0 failed`（rc=0）。
…[推理：此处截断，全文 2305 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①3 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept && codegraph node host/src/control/registry_rule/registry_rule.rs`  · exit 0 · step 65（turn 1）

**工具返回**：
```
**host/src/control/registry_rule/registry_rule.rs** — 7 lines, 1 symbol · no other indexed file depends on it

1	//! The control parent's minimum structure for a direct child.
2	
3	use crate::RegistrationRule;
4	
5	pub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::new()
6	    .require_exports(&["control.render"]);
7
```

**当时推理**：
```
All answers are ≤20 lines ✓. Step counts are recorded.

Now the supporting docs. Note h2 shows nb=2 but the answer says 4 nonbridge calls — because two of them (the `diff` per-sibling and the earlier `cat`) — let me check: for h2 the logged nb calls are: `cat entries`, `cargo test`, and...? The `sha256sum + diff` call was run with plain bash (not nb.sh) — that one wasn't logged into h2's log! Let me check: I ran `sha256sum ... ; diff ...` in a plain bash call (not via nb.sh). So it's not in the log. Similarly the ledger `cat` was logged ✓ (via nb.sh).

For completeness of the record, let me append the missing nonbridge records to the logs. I can do that honestly by noting them in BATCHING.md (the call outputs are summarized there), or by re-running them through nb.sh (they're deterministic). Re-running is cleaner: the diff/sha256sum call for h2. Let me re-run that one through nb.sh so the log holds it verbatim (it's the key evidence for "exactly one missing declaration").

Similarly, other plain bash calls that contained code inspection:
- `diff -r s1 vs s2..s8` (served s2..s8) — not logged anywhere. Let me log it once (via nb.sh with a neutral id, e.g. into a shared file `logs/_shared.txt`) and reference it from BATCHING.md. Hmm, the task says per-question logs; shared calls go in batching.md. I'll log the shared diff to `logs/_shared.txt` with CMD/EXIT via nb.sh, and cite it in BATCHING.md.
- `diff -r r1 r2|r3|r4` etc. — apparatus, logged in APPARATUS.md text. Let me also p
…[推理：此处截断，全文 2838 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①4 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept && codegraph node host/src/control/object/spinner/spinner.rs`  · exit 0 · step 67（turn 1）

**工具返回**：
```
**host/src/control/object/spinner/spinner.rs** — 24 lines, 3 symbols · used by 1 file: host/src/control/control.rs

1	//! The spinner face.
2	
3	use crate::control::{ControlFrame, ControlHandle, to_local};
4	
5	pub struct Spinner;
6	
7	impl ControlHandle for Spinner {
8	    /// Paint this widget onto the frame it was handed.
9	    fn paint(&self) -> ControlFrame {
10	        ControlFrame
11	    }
12	}
13	
14	/// This widget's offset, in the coordinate family its siblings use.
15	pub fn offset(x: i32) -> i32 {
16	    to_local(x)
17	}
18	
19	crate::control_object! {
20	    kind: Spinner,
21	    exports: ["control.render"],
22	    parent: crate::control::NODE_ID,
23	}
24
```

**当时推理**：
```
I have what I need. Let me write SETUP.md now — comprehensive but readable.

Content plan:

# SETUP — codegraph 臂 (cg26) 装置、索引与范围验证

## 1. 工具
- `/home/nich/Moirai_N3/nichlink/target/round7/tools-upstream/v1.6.1/bin/codegraph`；`--version` → `1.6.1`；`--help` 列出的子命令: init/index/sync/status/query/explore/context/node/files/callers/callees/impact/affected...
- 全部调用经 `target/probe-cg26/cg.sh <id> <args>`：cwd = 该题副本，输出逐字追加到 `logs/<id>.txt`，三行格式 `=== CMD` / 原文 / `=== EXIT`。

## 2. 副本
- 命令: `rsync -a --exclude target --exclude .codegraph --exclude .git target/round9/trees/<id> target/probe-cg26/trees/`
  （多排 `.git`：登记哈希本来就跳过 `.git`；`.git` 不入副本也顺手堵死"翻题树历史"）
- 24 棵题树 + g1/g2 各一份自己的副本（g1= s3 的字节副本、g2= s5 的字节副本；`diff -r` 已证 IDENTICAL）
- base 哈希（TREES.json 算法，跳过 {target,.codegraph,.git}）：表
- 开工前复核: `node /tmp/copycheck.mjs`（原文在 logs/_shared.txt）→ 15/15 OK

## 3. 索引
- 每棵树在**自己的副本里**建：`cd <copy> && codegraph init .`（从不 `init <别处路径>`）→ `.codegraph/` 落在副本内
- 索引统计表（files/nodes/edges）: from logs
- 索引成本: 每棵树 1 次 init（计入该题 logs 的第一条 CMD）
- r3 的首次 init 是 1.439s（`time` 实测），之后 25 棵同秒级；索引 4–24 文件、19–167 节点

## 4. 范围验证（每个题一条，原文在各自的 logs）
- 命令: `codegraph files`；判据: 列出的路径都是**树内相对路径**，无 `toolchain/`、`kernel/` 之类外层符号
- 原文样本 (s1): (quote)
- 逐题原文: logs/<id>.txt 的第一条 `=== CMD ... files` 记录
- 源码回读验证: `node <file>` 返回"逐字 line-numbered 源码"（例: s1 的 crates/core/src/store.rs 21 行 post、h2 的 spinner.rs 19-23 块）；这也是排除"符号链接为根时 could not read from disk"的那一步（我的是真副本，全部成功）

## 5. nonbridge 计数规则
- codegraph step = logs/<id
…[推理：此处截断，全文 2336 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①5 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept && codegraph node host/src/control/control.rs`  · exit 0 · step 68（turn 1）

**工具返回**：
```
**host/src/control/control.rs** — 25 lines, 6 symbols · used by 9 files: host/src/control/object/badge/badge.rs, host/src/control/object/button/button.rs, host/src/control/object/dial/dial.rs, host/src/control/object/gauge/gauge.rs, host/src/control/object/panel/panel.rs, host/src/control/object/slider/slider.rs, host/src/control/object/spinner/spinner.rs, host/src/control/object/timeline/timeline.rs, +1 more

1	//! The control folder face, the two coordinate families its children may use, and the
2	//! handle every direct child implements.
3	
4	/// The local family: offsets are measured from the widget's own origin.
5	pub fn to_local(x: i32) -> i32 { x + 7 }
6	
7	/// The world family: offsets are measured from the frame's origin.
8	pub fn to_world(x: i32) -> i32 { x + 31 }
9	
10	/// The frame a parent face hands to its children for painting.
11	pub struct ControlFrame;
12	
13	/// The interface every direct child must implement.
14	pub trait ControlHandle {
15	    fn paint(&self) -> ControlFrame;
16	}
17	
18	pub struct Control;
19	
20	crate::root_object! {
21	    kind: Control,
22	    needs_registry: true,
23	    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
24	}
25
```

**当时推理**：
```
Now BATCHING.md and PARTIAL.md, and update APPARATUS.md with the base-hash-per-question table for the 15 (the captain asked).

BATCHING.md content:
- Shared nonbridge calls (their outputs live in logs/_shared.txt unless noted):
  1. `diff -r -x .codegraph s1 <v>` for v in s2..s8 — served s2,s3,s4,s5,s6,s7,s8 (7 questions) — output 1.1KB — logs/_shared.txt. Result: s3/s4 identical to s1; s2 diff = audit.rs:9; s5 diff = query.rs:27 + render.rs:9; s6 = model.rs:38; s7 = buckets.rs:10; s8 = render.rs:9+12.
  2. `diff -r -x .codegraph trees/s1 trees/g4` — served g4 — 229 B — logs/g4.txt.
  3. `sha256sum + 逐兄弟 diff` — served h2 — 1.2KB — logs/h2-claim-unkept.txt.
  4. `cat .nichlink/adopted/entries` — served h2 (already logged).
  5. Hash checks `node /tmp/copycheck.mjs` — apparatus + all 15 (logs/_shared.txt).
  6. `node /tmp/treecheck.mjs` — apparatus (11 mismatches) — output in APPARATUS.md.
- Shared *codegraph* bash batches: each codegraph invocation is logged with its own CMD/EXIT in its question's log, so token attribution stays per question even though one bash call carried several (e.g. one call carried s2+s5 calls; one carried s6+s7+s8+g2; one carried s4+g1). List them.

PARTIAL.md content:
- What's done: 15/15 of the captain-approved scope; 11 suspended.
- Per suspended question: why (hash mismatch), what the current tree is (observed), what's needed (restored tree + re-copy + re-init), plus the observation that r1-r4 are green/identical (cargo evidence).
- Note that the 
…[推理：此处截断，全文 2000 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①6 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept && codegraph query REGISTRATION_RULE`  · exit 0 · step 72（turn 1）

**工具返回**：
```
Search Results for "REGISTRATION_RULE":

variable    REGISTRATION_RULE
  host/src/control/registry_rule/registry_rule.rs:5
```

**当时推理**：
```
All checks pass:
- 15 answers, all ≤20 lines, each ends with a `nonbridge:` line.
- All logs have matched CMD/EXIT pairs.
- Nothing written under target/round9 ✓.

Wait, one issue: the `find ../round9 -newermt ...` returned empty — good. But hold on, does that mean nothing under round9 changed at all? The captain said they're restoring trees; if they restored while my find ran... The find output was empty, meaning no files under round9 (excluding target/ and .git/) were modified after 22:55. Hmm, but the trees were modified at 22:50-22:51 (before 22:55) ✓. Fine — and my writes are all in probe-cg26 ✓.

Hmm, but interesting: the captain said they were restoring the 11 trees; maybe they haven't yet.

Now let me finish:
1. Update the task: in_progress → completed with a precise output, acceptanceResults/commandsRun (this is a "work" kind task, not review — the update_task schema asks for acceptanceResults and commandsRun for implementation/repair/verification/integration kinds; kind=work here. The instruction says: "Quality kinds must submit structured fields: review/requirements... implementation/repair/verification/integration need acceptanceResults and commandsRun". This task is kind=work → not strictly required. But providing acceptance/commands evidence is good practice and harmless. I'll include a few acceptanceResults entries and commandsRun entries.

Wait — also I must first mark in_progress (I never did that! I claimed, then worked; the rules say "Mark in_progress first
…[推理：此处截断，全文 5635 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①7 `cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept && codegraph callers REGISTRATION_RULE`  · exit 0 · step 75（turn 1）

**工具返回**：
```
Callers of "REGISTRATION_RULE" (0):

variable REGISTRATION_RULE (rust) — host/src/control/registry_rule/registry_rule.rs:5
ℹ No callers found for "REGISTRATION_RULE"
```


## 交付答案（逐字）

# h2-claim-unkept — 真标本台账 + 一个同族只缺一条声明 + 一条指纹失效的条目

① 症状：树能编译、测试全绿（offsets `1 passed`）；问题在台账与兄弟形状上，不在测试运行上。
② `root cause`（两处）：
   - **失效台账条目**＝`host/.nichlink/adopted/entries` 第 2 行（`root/control/panel|the panel's original contract|…|2026-09-20T09:00:00+08:00|src/control/object/panel/panel.rs|edc72845…|recorded before the panel was rewritten`）。
   - **少一条声明的兄弟**＝`host/src/control/object/spinner/spinner.rs:19-23` 的 `control_object!`：只有 `kind`/`exports`/`parent` 三条，缺参照兄弟 `button.rs:23` 的 `handle_contracts: [crate::control::ControlHandle],`。
   `mechanism`：① 两条记录指纹逐字相同（`edc72845cc315b0b…`），而它们指名的两个文件字节不同（`button.rs`=06c45576d883e961…、`panel.rs`=d886e9b0c1281eac…）⇒ 两条不可能同时描述自己的字节；panel 那条自述"在 panel 被重写之前记录"、时间戳（09-20）也早于 button 标本（10-01）⇒ 失效的是 panel 条目（指纹是抄来的/重写后没重录）。② 约束写在 `control.rs:13-16`（"The interface every direct child must implement."）并由每个兄弟在自己注册面里复述一遍 `handle_contracts`；`registry_rule.rs:5-6` 只 `require_exports(&["control.render"])`，没有任何东西要求 `handle_contracts` ⇒ spinner 漏声明不会被门禁或测试抓到。
③ `fix`：`spinner.rs:23` 前补 `handle_contracts: [crate::control::ControlHandle],`；**台账既有条目一字不改、不删**——panel 那条的处置是"**需要人，不是改代码**"：由**人**在台账里**追加**一条确认行（走桥的 `adopted … --apply` 流程），旧两行原样留作历史；代码侧没有可改的东西（记录失真不是缺陷）。**不动** `control.rs`、`registry_rule.rs`、offsets 测试、其它 7 个兄弟、台账既有两行。
④ `counter-proof`：把 8 个兄弟逐个与 button 比，只有 spinner 出现"缺一行"的 hunk，其余 7 个只差名字替换 ⇒ "恰好缺一条"可核对；若"失效的是 button 条目"，则 panel 那条的 note 与更早时间戳无法解释——两条互斥，note/时间戳指向 panel，而且**可重算**：按内核 `adoption_fingerprint` 的拼法（相对路径 ++ 0x00 ++ 内容字节长度 ++ 0x00 ++ 内容 ++ 0x00，再 sha256）我复算 `src/control/object/button/button.rs` 得 `edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9` == panel 那行写的指纹，而 panel 自己的当前指纹是 `57afd3b6e4eca34d…` ⇒ **那一行记的是别的文件（button）的当前指纹**，记录失真、需人工重确认（不是代码缺陷）。工具侧的反证：`callers REGISTRATION_RULE` 返回 0（宏引用的边没进图），所以我没有依赖"谁引用了规则"来下判断，而是回源码读 `registry_rule.rs`。
   `evidence`（全文见 `logs/h2-claim-unkept.txt`）：`cat …/.nichlink/adopted/entries`→2 行（rc=0）；`sha256sum button.rs panel.rs`（rc=0）；`node …/spinner/spinner.rs`→块 `19-23` 无 `handle_contracts`（rc=0）；`cargo test --offline`（`trees/h2-claim-unkept/host`）→ `test the_rendered_offsets_add_up ... ok` / `test result: ok. 1 passed; 0 failed`（rc=**0**）。
⑤ 调用清单：`files`；`node host/src/control/object/spinner/spinner.rs`、`host/src/control/control.rs`、`host/src/control/registry_rule/registry_rule.rs`；`query REGISTRATION_RULE`、`callers REGISTRATION_RULE`；非 codegraph：`cat entries`、`sha256sum`、`diff` 逐兄弟、`cargo test`。

总调用：成功 7 / 被拒 0（codegraph；含 1 次 init ⇒ 6 步）

nonbridge: 3 次 / 3,693 B（= 本题日志里列出的 3 条：`cat entries` 455 B、`cargo test` 1013 B、`sha256sum`+逐兄弟 `diff` 2225 B；早前还有 1 次同样内容的 `diff` 直连 bash 未入日志 ⇒ **本栏是下界**，口径见 `SETUP.md` §5）