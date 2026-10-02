# cg · `h2-claim-unkept`

**题面**：hardbug h2：采信未落实——真台账 + 一个同族只缺一条声明 + 一条指纹失效。

> 步 6 · 仪器调用 **7**（逐题日志条数）· 会话里定位到 7 条 · 输出 13,069 tok · 推理 34,906 字符 · 累计上下文 1,201,280

## 一、工具给了什么 · 当时 agent 在想什么（逐条配对）

### ①1 日志第 1 条 · exit 0 · step 57（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept && codegraph init .`

**工具返回**（逐字）：
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

**该步推理**（逐字）：
```
None of my hypotheses match `edc72845…`. So the fingerprint scheme is something else (likely the toolchain's own NodeId hash — e.g. sha256 over the registration-face record: file path + line + kind + ... in some canonical form defined in kernel/src/registry_core/identity).

Can I determine it without grepping the workspace? I could use the session's codegraph MCP tool on the workspace (allowed by my toolset, but would contaminate the frozen-CLI-only arm). Hmm.

Alternatively, reason from the brief: the h2 class explicitly says: "真标本台账 + 一个同族只缺一条声明 + 一条指纹失效的条目" — "one ledger entry whose fingerprint has lapsed". The ledger's own note distinguishes them: button = "the reference sibling shape ... the shape every sibling follows"; panel = "recorded before the panel was rewritten". Since both entries carry the *same* fingerprint and the two files differ, at most one can be current. The panel's note says it was recorded before a rewrite ⇒ the panel's entry is the lapsed one; the button's entry is the specimen ("真标本"), i.e. its fingerprint is the one that matches the button's current shape.

That's a sound diagnosis without needing the exact hash function: the evidence is that the two fingerprints are identical while the two named files' bytes differ (sha256 06c45576 vs d886e9b0 per file bytes), so the ledger cannot describe both; and the panel entry's own note says it predates the rewrite ⇒ the panel entry no longer describes the bytes it names (its fingerprint equals the button's, i.e. it was never updated when the panel was rewritten). Also the older timestamp (2026-09-20 vs 2026-10-01) supports it.

Hmm, but hold on: is it possible that the *button* entry is the lapsed one, and the panel entry is... no: the panel's note literally says "recorded before the panel was rewritten" — so its fingerprint describes the pre-rewrite panel. And the button entry's note "the shape every sibling follows" matches the canonical reference. Also the button's fingerprint is claimed to be the "specimen" that the fleet should match: the fact that every sibling follows the button's *shape* (except spinner missing one declaration) supports the button entry being the live specimen.

Hmm, but hmm: what if the true intent is: the fingerprint is a hash of the *shape* (not bytes), so many siblings share it. Then "one ledger entry no longer describes the bytes it names" = panel: it was recorded before the panel was rewritten (bytes changed) — same conclusion ✓.

Either way, the panel entry is the lapsed one. Good. Let me also verify with a stronger check: compare the panel.rs file's diff vs button.rs — the only differences are name substitutions (panel vs button) → the panel's *shape* is now the same as the button's. Since the panel was "rewritten" and its entry wasn't updated (fingerprint still equals the button's), and its own note says the recording predates the rewrite... 

Hmm, that reasoning has a wrinkle: if the panel's current shape == the button's shape, then a shape-based fingerprint of the panel WOULD equal the button's fingerprint. So the panel entry's fingerprint might *accidentally* be current under a shape-hash scheme. But under a byte-hash or NodeId (path+line) scheme it's stale. Ugh.

Wait — maybe the fingerprint is the NodeId, and it's the SAME for button and panel because... no, NodeId depends on file!() path → different.

Let me try to determine the scheme properly using the *other* trees? No. Let me consider using the session's codegraph MCP on the workspace to find the adoption/ledger code (the MCP tool is part of my available toolset, and this is a legitimate way to inspect the workspace without grepping). The concern about measurement contamination: the "codegraph arm" measurement is about per-question tree work. Using the workspace-level codegraph MCP would be an extra codegraph call not on the question tree — that's a deviation. But I could log it honestly as an additional call (and note it's on the outer workspace, not the题树). Hmm.

Alternatively: the h2 host's `.nichlink/adopted/entries` format — maybe documented in the workspace docs (AGENTS.md doesn't mention "adopted"). The brief says the question is "what the ledger and the sibling shapes say" — so the answer should focus on:
1. the sibling missing exactly one declaration: **spinner** (missing `handle_contracts: [crate::control::ControlHandle]` at its `control_object!` block, i.e. spinner.rs:19-23 block lacks line 23 of the reference).
2. the lapsed ledger entry: **root/control/panel** (its fingerprint equals the button's specimen fingerprint and predates the panel rewrite; the bytes it names now hash differently).

For the answer I can present the check: two entries, same fingerprint; bytes differ (06c45576… vs d886e9b0…) ⇒ at most one entry can be describing its file; the panel entry's own note/timestamp says it was recorded before the panel was rewritten ⇒ it is the lapsed one. And the constraint not holding on a later sibling = the `handle_contracts` declaration that every sibling carries but spinner omits (the adopted constraint from the reference sibling shape). Does the tree define that constraint somewhere (e.g. registry_rule has `.require_exports(["control.render"])`)? In h2's registry_rule, maybe there's a rule requiring handle contracts. Let me read h2's registry_rule.rs and control.rs to see if there's a declared constraint (the "constraint this design adopted is not holding on a later sibling").

Let me read h2's: host/src/control/registry_rule/registry_rule.rs, host/src/control/control.rs, host/src/control/object/spinner/spinner.rs (via codegraph node), and check if any test asserts handle_contracts.

Note: h2's tests pass, so no test catches spinner's missing declaration ⇒ the finding is static.

Let me also check whether the "constraint" is expressed in the `.nichlink/adopted/entries` (the ledger = adopted constraints) + registry_rule. In h1-supply-chain, registry_rule.rs required exports `["control.render"]`. For h2, maybe
…[推理：此处截断，全文 6043 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

### ①2 日志第 2 条 · exit 0 · step 60（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept && codegraph files`

**工具返回**（逐字）：
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

**该步推理**（逐字）：
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
⑤ 调用清单：`files`；`node host/src/control/object/spinner/spinner.rs`、`…/control.rs`、`…/registry_rule/registry_rule.rs`；`query REGISTRATION_RULE`、`callers REGISTRATION_RULE`（0 caller——宏引用的边图里没有）；非 codegraph：`cat entries`、`sha256sum`、`diff` 逐兄弟、`cargo test`。

Let me also double check that h2's toggle uses to_local (so no second defect). From the earlier layout: h2's tree is the "ledger" class; the offsets test passes ✓ ⇒ no family mismatch there.

Also note: h2's `.nichlink/adopted/entries` is a 2-line file; format pipe-separated.

Line count: keep ≤20. Let me write it compactly.

One more check: the fingerprint comparison — I should be careful in claiming "panel 那条失效" as the unique answer. Let me present it as: 失效那条 = panel（判据：两录指纹相同而两文件字节不同 + panel 自述"重写前记录" + 时间戳更早）. Good.

Let me write the file.
```

### ①3 日志第 3 条 · exit 0 · step 65（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept && codegraph node host/src/control/registry_rule/registry_rule.rs`

**工具返回**（逐字）：
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

**该步推理**（逐字）：
```
All answers are ≤20 lines ✓. Step counts are recorded.

Now the supporting docs. Note h2 shows nb=2 but the answer says 4 nonbridge calls — because two of them (the `diff` per-sibling and the earlier `cat`) — let me check: for h2 the logged nb calls are: `cat entries`, `cargo test`, and...? The `sha256sum + diff` call was run with plain bash (not nb.sh) — that one wasn't logged into h2's log! Let me check: I ran `sha256sum ... ; diff ...` in a plain bash call (not via nb.sh). So it's not in the log. Similarly the ledger `cat` was logged ✓ (via nb.sh).

For completeness of the record, let me append the missing nonbridge records to the logs. I can do that honestly by noting them in BATCHING.md (the call outputs are summarized there), or by re-running them through nb.sh (they're deterministic). Re-running is cleaner: the diff/sha256sum call for h2. Let me re-run that one through nb.sh so the log holds it verbatim (it's the key evidence for "exactly one missing declaration").

Similarly, other plain bash calls that contained code inspection:
- `diff -r s1 vs s2..s8` (served s2..s8) — not logged anywhere. Let me log it once (via nb.sh with a neutral id, e.g. into a shared file `logs/_shared.txt`) and reference it from BATCHING.md. Hmm, the task says per-question logs; shared calls go in batching.md. I'll log the shared diff to `logs/_shared.txt` with CMD/EXIT via nb.sh, and cite it in BATCHING.md.
- `diff -r r1 r2|r3|r4` etc. — apparatus, logged in APPARATUS.md text. Let me also put the treecheck/copycheck scripts' outputs there (I have the text in this transcript; I'll write the key results into APPARATUS.md, which is enough).
- `diff -r trees/s1 trees/g4` — that one WAS via nb? No: I ran it inside a bash call with `nb() { ./nb.sh "$@"; }` defined but then used `diff -r -x .codegraph trees/s1 trees/g4` directly (plain). Hmm, the answer for g4 says nonbridge 1 (the diff) with ~200 bytes, but it wasn't logged. Let me re-run it via nb.sh into g4's log.
- `node /tmp/treecheck.mjs` and `node /tmp/copycheck.mjs` — the hashes; log them to logs/_shared.txt too? Their output is already reproduced in APPARATUS.md and SETUP.md. I'll cite APPARATUS.md.
- `cat` of the four hardbug BRIEFs (1 call) + `find` layouts — apparatus/题面, noted in SETUP.md.

To keep this bounded and honest, let me:
1. Re-run through nb.sh: (a) the h2 per-sibling diff+sha256sum, (b) the s1-vs-s2..s8 diff into logs/_shared.txt, (c) the s1-vs-g4 diff into g4's log, (d) the copycheck hash script into logs/_shared.txt.
2. Then write BATCHING.md listing all shared calls with byte counts.
3. Write SETUP.md.
4. Update APPARATUS.md with base-hash-per-question.
5. PARTIAL.md.

Let me do step 1 in one bash call (4 nb.sh invocations; each nb.sh invocation is a separate nonbridge call → attribution clean: (a) → h2, (b)+(d) → shared, (c) → g4).
```

### ①4 日志第 4 条 · exit 0 · step 67（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept && codegraph node host/src/control/object/spinner/spinner.rs`

**工具返回**（逐字）：
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

**该步推理**（逐字）：
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
- codegraph step = logs/<id>.txt 里一条 `=== CMD: ... codegraph ...`
- nonbridge = 非 codegraph 的调用；日志里记 `=== CMD(nonbridge)` + `=== NONBRIDGE-BYTES`；共享调用记在 BATCHING.md
- 已记录: 每题的 nb 次数/字节在答案末行；装置级（拷贝/init 脚本/哈希复核/大队列 diff）在本文与 APPARATUS.md

## 6. 装置异常
见 APPARATUS.md（11 棵树与登记不符）；PARTIAL.md 记 11 道待复位题。

Also: the captain asked to record base hash per question in APPARATUS.md — I'll add that section there too (or refer to SETUP.md). The captain said "在 APPARATUS.md 里记下你用的副本的 base 哈希（每题一行）". Let me add a table to APPARATUS.md §7 with per-question base hash for the 15 + a note that the 11 damaged ones' copies are quarantined.

Let me write SETUP.md now (with the base hash table covering the 15 sound + the note about the 11).

Also, include the note that the 11 damaged trees were copied+indexed at 22:59 (logs exist) but are quarantined.

Let me write the file.
```

### ①5 日志第 5 条 · exit 0 · step 68（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept && codegraph node host/src/control/control.rs`

**工具返回**（逐字）：
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

**该步推理**（逐字）：
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
- Note that the 11 trees *were* copied and indexed at 22:59 (their logs hold only init/files — no answers), and that after restoration I'll re-rsync and re-init appending to the same logs.

APPARATUS.md addition: §7 base-hash table per question (15 sound, one line each) — or reference SETUP.md §2 and add the 11's quarantine note. The captain asked "记下你用的副本的 base 哈希（每题一行）" — let me add a compact per-question one-line list (15 lines).

Let me write BATCHING.md and PARTIAL.md, then append to APPARATUS.md via edit.
```

### ①6 日志第 6 条 · exit 0 · step 72（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept && codegraph query REGISTRATION_RULE`

**工具返回**（逐字）：
```
Search Results for "REGISTRATION_RULE":

variable    REGISTRATION_RULE
  host/src/control/registry_rule/registry_rule.rs:5
```

**该步推理**（逐字）：
```
All checks pass:
- 15 answers, all ≤20 lines, each ends with a `nonbridge:` line.
- All logs have matched CMD/EXIT pairs.
- Nothing written under target/round9 ✓.

Wait, one issue: the `find ../round9 -newermt ...` returned empty — good. But hold on, does that mean nothing under round9 changed at all? The captain said they're restoring trees; if they restored while my find ran... The find output was empty, meaning no files under round9 (excluding target/ and .git/) were modified after 22:55. Hmm, but the trees were modified at 22:50-22:51 (before 22:55) ✓. Fine — and my writes are all in probe-cg26 ✓.

Hmm, but interesting: the captain said they were restoring the 11 trees; maybe they haven't yet.

Now let me finish:
1. Update the task: in_progress → completed with a precise output, acceptanceResults/commandsRun (this is a "work" kind task, not review — the update_task schema asks for acceptanceResults and commandsRun for implementation/repair/verification/integration kinds; kind=work here. The instruction says: "Quality kinds must submit structured fields: review/requirements... implementation/repair/verification/integration need acceptanceResults and commandsRun". This task is kind=work → not strictly required. But providing acceptance/commands evidence is good practice and harmless. I'll include a few acceptanceResults entries and commandsRun entries.

Wait — also I must first mark in_progress (I never did that! I claimed, then worked; the rules say "Mark in_progress first, then completed"). I should call update_task with status=in_progress first, then completed. Let me do both (in_progress then completed) — two calls. Since claimed → completed isn't allowed, do in_progress now, then completed immediately after.

2. Send the report to the captain (agent_teams_send_message with source_task_id/source_attempt_id), including: 15/26 delivered, file list, key apparatus points, the nonbridge/steps summary useful for their metrics, and the pending 11 awaiting restoration.

Also, the system prompt said: "Before you finish, send your result to that agent with send_message({agent_id: 'session-b86163d1-...'})". The rule in the AgentTeams section says the captain IS my parent and this single message satisfies both duties. I already sent one message to captain (the blocker). Now I need the final report. I'll use agent_teams_send_message(to=captain) — that satisfies both (per rule 4 note).

Hmm, the outer instruction explicitly says to use send_message with agent_id. The AgentTeams note says "The captain is also your parent: this single message satisfies both reporting duties." To be safe, I'll send the final report via agent_teams_send_message(to=captain) (team channel, which the captain reads). That's the designated path and avoids duplicate messaging. Good.

Report content (concise but complete):
- t1 attempt da6da84d, status: completed (15/26 delivered; 11 待复位).
- Deliverables: answers/{s1..s8,g1,g2,g4,h1-supply-chain,h2-claim-unkept,h3-cross-file-chain,h4-one-file-many-algorithms}.md, logs/*.txt (三行式), SETUP.md, BATCHING.md, APPARATUS.md, PARTIAL.md.
- Key per-question headline answers (one line each) so the captain can sanity-check / judge:
  - s1: 唯一调用者 = crates/report/tests/report.rs:11 store()（ledger_report 集成测试），:13-15 三次调 post。
  - s2: 红的面 = audit 特性面；根因 audit.rs:9 `== 0` 写反；默认面因 cfg 假绿。
  - s3: 无测试可达的函数 = audit_unused (core/src/audit.rs:14)；audit_count 只在 --features audit 面可达（工具漏了限定路径边）。
  - s4: 只有 crates/report/tests/report.rs 能到达 Store，路径 test→store():11→Store::new:15/post:21/entries:31。
  - s5: 渲染缺 write_count 调用（render.rs:7 的循环里没有它；callers write_count=0）；另 query.rs:27 `>` 反向。
  - s6: 不一致；model.rs:38 `||` 应为 `&&`（零金额也可入账）。
  - s7: 含 1000；buckets.rs:10 `<= 1000`（套件要求 1000 归上一档）。
  - s8: 条数行写在末尾；render.rs:12（循环之后）。
  - g1: 死函数 audit_unused；条件项 audit_count（默认面）；另有 3 个"没人执行过的分支"：buckets.rs:8-9 debit 支、store.rs:22-24 拒绝支、query.rs:21-25 account 支；看不见什么也列了。
  - g2: 伪代码 + 差 = 缺 write_count 调用；第二处 query.rs:27。
  - g4: 判死两支 = bands.rs:36-41（if false）、:54（Band::Frozen，私有枚举无构造）；判不了 = State::Dormant（pub 可见性边界）+ audit 面；执行能进但测试不进 = :42-48、:92。
  - h1-supply-chain: toggle.rs:16 调 to_world（其余 8 个 to_local）⇒ 160≠136（cargo 实测 exit 101）。
  - h2-claim-unkept: spinner.rs:19-23 缺 handle_contracts 一条；台账 panel 条目指纹失效（与 button 标本同指纹、自述重写前记录）。
  - h3-cross-file-chain: lib.rs:48-66 计划未点名 dial ⇒ shipped 测试失败（cargo 实测 exit 101，打印树里无 dial）。
  - h4: entry.rs:30-32 zero_arm 恒 Post（rustc dead-code 警告点名 Refuse 未构造）；zero+receipt postable=true（scratch 实测）；修法 if amount==0 → Refuse。
- Numbers for their metrics: codegraph steps per question total (sum of CMD counts minus init): let me compute: s1 17-1=16, s2 6, s3 19, s4 6, s5 6, s6 4, s7 5, s8 4, g1 8-2=6 (g1 had 2 inits), g2 8-2=6, g4 14-1=13, h1 12-1=11, h2 10-1=9, h3 14-1=13, h4 11-1=10 → total steps = 16+6+19+6+6+4+5+4+6+6+13+11+9+13+10 = 134. Plus init calls 17. Let me report: codegraph 调用 = 134 步（不含 17 次 init；含 init 151）。
  Hmm, careful: the CMD counts include the files calls and init. Let me just report: 每题 `=== CMD` 条数与其中 init 条数, plus tally. 134 question-steps + 17 init = 151 codegraph invocations for the 15 questions. Also nonbridge tally: s-questions 0 own (1 shared diff for s2..s8 + copycheck shared by all 15), g4 1, h1 2, h2 4, h3 1, h4 2 = 10 own + shared.
  Output bytes: they can measure from the transcript; I could report the total logged bytes: let me compute du of logs for the 15.
- Base hashes verified 15/15 (SETUP.md §2 + APPARATUS.md §6).
- The 11 remain untouched, awaiting restoration; PARTIAL.md records the plan.

Let me compute the totals quickly, then do the status updates and the message.
```

### ①7 日志第 7 条 · exit 0 · step 75（turn 1）

**命令**：`cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept && codegraph callers REGISTRATION_RULE`

**工具返回**（逐字）：
```
Callers of "REGISTRATION_RULE" (0):

variable REGISTRATION_RULE (rust) — host/src/control/registry_rule/registry_rule.rs:5
ℹ No callers found for "REGISTRATION_RULE"
```


### 该题的**非仪器**调用（臂自己记的，日志里带 `=== CMD(nonbridge)`）

**非桥 1**：`cat trees/h2-claim-unkept/host/.nichlink/adopted/entries`
```
root/control/button|the reference sibling shape|traced once|nich|2026-10-01T10:00:00+08:00|src/control/object/button/button.rs|edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9|the shape every sibling follows
root/control/panel|the panel's original contract|traced once|nich|2026-09-20T09:00:00+08:00|src/control/object/panel/panel.rs|edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9|recorded before the panel was rewritten
=== NONBRIDGE-BYTES: 455
```

**非桥 2**：`bash -c cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept/host && cargo test --offline > /tmp/cg26_h2.out 2>&1; rc=$?; cat /tmp/cg26_h2.out; exit $rc`
```
Compiling nichlink-example-control-button-graft v0.1.0 (/home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept/graft)
   Compiling nichlink-example-control-button v0.1.0 (/home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept/host)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.38s
     Running unittests src/lib.rs (/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target/debug/deps/control_button-69ecc65bd8be5845)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/offsets.rs (/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target/debug/deps/offsets-39c084bff4dbfe76)

running 1 test
test the_rendered_offsets_add_up ... ok

test result: ok. 1 passed;
…[输出：此处截断，全文 1035 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**非桥 3**：`bash -c cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept/host/src/control/object && sha256sum button/button.rs panel/panel.rs && for o in panel slider timeline gauge toggle spinner badge dial; do echo "--- $o vs button"; diff button/button.rs $o/$o.rs; done; exit 0`
```
06c45576d883e9619cd1574cae1e5e69868cdfc0222fab1878d76b79715080f4  button/button.rs
d886e9b0c1281eac218858bb612aa6241f6594b378d0a5afc0787432816b5a55  panel/panel.rs
--- panel vs button
1c1
< //! The button face.
---
> //! The panel face.
5c5
< pub struct Button;
---
> pub struct Panel;
7c7
< impl ControlHandle for Button {
---
> impl ControlHandle for Panel {
20c20
<     kind: Button,
---
>     kind: Panel,
--- slider vs button
1c1
< //! The button face.
---
> //! The slider face.
5c5
< pub struct Button;
---
> pub struct Slider;
7c7
< impl ControlHandle for Button {
---
> impl ControlHandle for Slider {
20c20
<     kind: Button,
---
>     kind: Slider,
--- timeline vs button
1c1
< //! The button face.
---
> //! The timeline face.
5c5
< pub struct Button;
---
> pub struct Timeline;
7c7
< im
…[输出：此处截断，全文 2250 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```


## 二、该题相关的其它调用（读文件 / 编辑 / 跑测试 / 写答案），带当时的推理

**调用** `bash`（step 54）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/probe-cg26; ./nb.sh h2-claim-unkept cat trees/h2-claim-unkept/host/.nichlink/adopted/entries; echo \"===== cargo test h2 =====\"; export CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target; ./nb.sh h2-claim-unkept bash -c 'cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept/host && cargo test --offline > /tmp/cg26_h2.out 2>&1; rc=$?; cat /tmp/cg26_h2.out; exit $rc' | tail -20", "description": "h2: read adopted entries ledger and run tests", "timeoutMs": 300000}
```

**结果**：
```
root/control/button|the reference sibling shape|traced once|nich|2026-10-01T10:00:00+08:00|src/control/object/button/button.rs|edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9|the shape every sibling follows
root/control/panel|the panel's original contract|traced once|nich|2026-09-20T09:00:00+08:00|src/control/object/panel/panel.rs|edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9|recorded before the panel was rewritten
[nb bytes=455 rc=0]
===== cargo test h2 =====
     Running unittests src/lib.rs (/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target/debug/deps/control_button-69ecc65bd8be5845)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/offsets.rs (/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target/debug/deps/offsets-39c084bff4dbfe76)

running 1 test
test the_rendered_offsets_add_up ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests control_button

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

[nb bytes=1013 rc=0]

```

**该步推理**（逐字）：
```
Both cargo runs recorded with true rc=101:
- h1-supply-chain: `the_rendered_offsets_add_up` FAILED, 160 vs 136.
- h3-cross-file-chain: `every_declared_widget_ships` FAILED: "`dial` is declared and does not ship" + the full shipped tree listing (root/control + 8 children, no dial) ✓ — beautiful evidence.

Now h2-claim-unkept. Its tree: host/.nichlink/adopted/entries (a ledger file), tests/offsets.rs, and the object files. Class: "a real specimen ledger entry, one sibling missing exactly one declaration, one lapsed entry" (真标本台账 + 一个同族只缺一条声明 + 一条指纹失效的条目).

So:
- The `.nichlink/adopted/entries` ledger has entries with fingerprints (sha256 of the declared face?), one entry is "lapsed" (its fingerprint no longer describes the bytes it names — i.e., the fingerprint is stale/mismatched).
- One sibling (a widget file) is missing exactly one declaration (e.g. missing `handle_contracts: [...]` or `exports: [...]` in its `control_object!` macro).

The brief says: "A constraint this design adopted is not holding on a later sibling, and one ledger entry no longer describes the bytes it names. The tree builds and its tests pass: the question is what the ledger and the sibling shapes say, not what a test run says."

So the tests pass; I must diagnose statically:
1. Which sibling is missing exactly one declaration (compare the 9 object files' `control_object!` blocks; the odd one lacks one field, e.g. `handle_contracts` or `exports`).
2. Which ledger entry's fingerprint no longer matches the bytes it names (compare fingerprint field in entries with the actual bytes of the named file — I'd need to compute a sha256 over the named file... The fingerprint scheme is defined by the toolchain (call_evidence/adoption). Hmm, I need to figure out the fingerprint definition. That could be `sha256(file bytes)` — let me look at the entries file first.

Let me read h2's files: `.nichlink/adopted/entries` (not a .rs → codegraph `node` may fail; use nb.sh cat — nonbridge), plus the 9 object files' macro blocks.

Let me first `node host/.nichlink/adopted/entries` via codegraph — probably "not found". Use nb.sh cat.

Also let me check the h2 tree's tests/offsets.rs (same as h1's? the h2 tree's test file is offsets.rs per layout). Hmm, but the brief says "The tree builds and its tests pass". Let me confirm by running cargo test for h2 (warm build, fast). That's evidence that tests pass ✓.

Let me read the entries ledger + run cargo test in one call.

Note: `.nichlink/adopted/entries` — its format is probably JSONL with fields like `path`, `fingerprint`, `face`. Let me look.
```

**该步模型输出**（逐字）：
```
Now **h2-claim-unkept**. Reading the ledger and running its tests:
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h2-claim-unkept：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/probe-cg26; ./nb.sh h2-claim-unkept cat trees/h2-claim-unkept/host/.nichlink/adopted/entries; echo \"===== cargo test h2 =====\"; export CARGO_TARGET_DIR=/home/nich/Moirai_N3/nichlink/target/probe-cg26/cargo-target; ./nb.sh h2-claim-unkept bash -c
…[参数：此处截断，全文 565 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

**调用** `bash`（step 55）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept/host/src/control/object; echo \"=== sha256 of named files ===\"; sha256sum button/button.rs panel/panel.rs; echo \"=== ledger fingerprints ===\"; echo \"entry1 = edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9\"; echo \"=== per-object macro block diff vs button (reference) ===\"; for o in panel slider timeline gauge toggle spinner badge dial; do echo \"--- $o vs button\"; diff button/button.rs $o/$o.rs; done", "description": "h2: hash named files and diff siblings vs button"}
```

**结果**：
```
=== sha256 of named files ===
06c45576d883e9619cd1574cae1e5e69868cdfc0222fab1878d76b79715080f4  button/button.rs
d886e9b0c1281eac218858bb612aa6241f6594b378d0a5afc0787432816b5a55  panel/panel.rs
=== ledger fingerprints ===
entry1 = edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9
=== per-object macro block diff vs button (reference) ===
--- panel vs button
1c1
< //! The button face.
---
> //! The panel face.
5c5
< pub struct Button;
---
> pub struct Panel;
7c7
< impl ControlHandle for Button {
---
> impl ControlHandle for Panel {
20c20
<     kind: Button,
---
>     kind: Panel,
--- slider vs button
1c1
< //! The button face.
---
> //! The slider face.
5c5
< pub struct Button;
---
> pub struct Slider;
7c7
< impl ControlHandle for Button {
---
> impl ControlHandle for Slider {
20c20
<     kind: Button,
---
>     kind: Slider,
--- timeline vs button
1c1
< //! The button face.
---
> //! The timeline face.
5c5
< pub struct Button;
---
> pub struct Timeline;
7c7
< impl ControlHandle for Button {
---
> impl ControlHandle for Timeline {
20c20
<     kind: Button,
---
>     kind: Timeline,
--- gauge vs button
1c1
< //! The button face.
---
> //! The gauge face.
5c5
< pub struct Button;
---
> pub struct Gauge;
7c7
< impl ControlHandle for Button {
---
> impl ControlHandle for Gauge {
20c20
<     kind: Button,
---
>     kind: Gauge,
--- toggle vs button
1c1
< //! The button face.
---
> //! The toggle face.
5c5
< pub struct Button;
---
> pub struct Toggle;
7c7
< impl ControlHandle for Button {
---
> impl ControlHandle for Toggle {
20c20
<     kind: Button,
---
>     kind: Toggle,
--- spinner vs button
1c1
< //! The button face.
---
> //! The spinner face.
5c5
< pub struct Button;
---
> pub struct Spinner;
7c7
< impl ControlHandle for Button {
---
> impl ControlHandle for Spinner {
20c20
<     kind: Button,
---
>     kind: Spinner,
23d22
<     handle_contracts: [crate::control::ControlHandle],
--- badge vs button
1c1
< //! The button face.
---
> //! The badge face.
5c5
< pub struct Button;
---
> pub struct Badge;
7c7
< impl ControlHandle for Button {
---
> impl ControlHandle for Badge {
20c20
<     kind: Button,
---
>     kind: Badge,
--- dial vs button
1c1
< //! The button face.
---
> //! The dial face.
5c5
< pub struct Button;
---
> pub struct Dial;
7c7
< impl ControlHandle for Button {
---
> impl ControlHandle for Dial {
20c20
<     kind: Button,
---
>     kind: Dial,
[exit code: 1]
```

**该步推理**（逐字）：
```
h2's ledger: 2 entries, format:
`path|description|state|author|timestamp|file|fingerprint|note`

1. `root/control/button | the reference sibling shape | traced once | nich | 2026-10-01T10:00:00+08:00 | src/control/object/button/button.rs | edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9 | the shape every sibling follows`
2. `root/control/panel | the panel's original contract | traced once | nich | 2026-09-20T09:00:00+08:00 | src/control/object/panel/panel.rs | edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9 | recorded before the panel was rewritten`

Both entries have the SAME fingerprint `edc72845…`. One of them "no longer describes the bytes it names" (指纹失效的条目). The button entry is "the reference sibling shape" (the真实标本 = the reference sibling), and the panel's is "recorded before the panel was rewritten" — but both fingerprints are identical, which is suspicious: the panel entry's fingerprint should differ from the button's if it describes the panel's original bytes.

So the task: compute the fingerprint of the bytes each entry names and see which entry's fingerprint doesn't match. The fingerprint is presumably `sha256(file bytes)` of the named file (src/control/object/button/button.rs, src/control/object/panel/panel.rs). Since both entries carry the same hash, at most one can match (files differ).

Let me compute sha256 of both files (nonbridge, or... hmm, could codegraph do it? No). I'll use sha256sum (nonbridge, tiny output).

If sha256(button.rs) == edc72845... then the button entry is intact and the panel entry has lapsed (its fingerprint is the button's, i.e. copy-paste/never-updated after the rewrite) ✓ — that matches the brief: "one ledger entry no longer describes the bytes it names" and the note "recorded before the panel was rewritten".

Then: "one sibling missing exactly one declaration" — compare the 9 object files' `control_object!` blocks. The button file is the reference shape; some sibling (maybe panel?) misses exactly one declaration. Let me read all 9 object files' macro blocks and compare. Since the files are 25 lines each, I can read them via codegraph `node` (9 calls) — or better, use codegraph to compare? Let me just read them; I need the exact lines for the answer.

Actually the panel was "rewritten" — maybe the panel is the sibling missing a declaration (e.g. missing `handle_contracts: [...]`). Let me read panel.rs and compare with button.rs (the reference).

Let me first compute the hashes and read panel.rs + button.rs + the other files. To be efficient: `node` the 9 object files in h2 (9 codegraph calls) — output ~25 lines each = 225 lines. That's OK but heavy. Alternative: read panel.rs via node (1 call) and check whether the diff vs button.rs is exactly one declaration. Then verify the other 7 siblings match the reference shape (to substantiate "one sibling missing exactly one declaration"). I can do that with `diff` against button.rs (nonbridge, cheap) — one call for all.

Hmm, "别把多字节字符切一半" etc. Fine.

Plan:
1. nb.sh: sha256sum of the 9 object files + the two ledger-named files. Actually just the two named files + maybe all 9.
2. codegraph node panel.rs (logged).
3. nb.sh diff each object file vs button.rs (to show which differ).

Then conclude.

Let me be careful about the fingerprint definition: the entries are "adopted" records; the fingerprint in the real toolchain (`adoption` / call_evidence) — maybe it's sha256 of the file *bytes*. Let me test: compute sha256 of button.rs and panel.rs and see if either equals edc72845...

If neither matches, my assumption about the fingerprint scheme is wrong, and I'd need to find the scheme in the toolchain source (which I can read? The workspace's toolchain source is at /home/nich/Moirai_N3/nichlink/toolchain — reading the workspace source is allowed (only "不许改工作区源码"). But grep-restriction: "不许递归全仓 grep（只在自己副本树内或 target/probe-cg26/ 里查）" — hmm! That forbids grepping the workspace. So I can't grep the toolchain for the fingerprint definition. But I can use **codegraph** (MCP tool) on the workspace index! The session has codegraph MCP for /home/nich/Moirai_N3/nichlink. Hmm, but my task says use only the frozen CLI... "工具：只用这一个" refers to the codegraph tool for the probe. Using the session's codegraph MCP to search the *workspace* (not the tree) would be a different index — that would contaminate the "codegraph arm" measurement (an extra codegraph call on a different index). Hmm. But it's my session's built-in tool. The task's禁令 didn't forbid using the session's MCP codegraph, but the measurement's integrity (26 questions answered with the frozen CLI on per-question trees) would be compromised if I start querying the outer workspace.

Let me first test the simple hypothesis: sha256 of the file bytes.

Let me compute.
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h2-claim-unkept：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept/host/src/control/object; echo \"=== sha256 of named files ===\"; sha256sum button/button.rs panel/panel.rs; echo \"=== ledger fingerprints ===\"; echo \"entry1 = edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f
…[参数：此处截断，全文 583 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

**调用** `bash`（step 56）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept/host; node -e '\nconst fs=require(\"fs\"),{createHash}=require(\"crypto\");\nconst H=s=>createHash(\"sha256\").update(s).digest(\"hex\");\nconst f=\"src/control/object/button/button.rs\"; const b=fs.readFileSync(f,\"utf8\"); const L=b.split(\"\\n\");\nconst cands={\n \"file\":b, \"file-nonl\":b.replace(/\\n$/,\"\"),\n \"block-19-24\":L.slice(18,24).join(\"\\n\"), \"block-19-24nl\":L.slice(18,24).join(\"\\n\")+\"\\n\",\n \"block-20-24\":L.slice(19,24).join(\"\\n\"), \"path\":f, \"path:19\":f+\":19\", \"path:20\":f+\":20\",\n \"abs\":\"/home/nich/Moirai_N3/nichlink/target/round9/trees/h2-claim-unkept/host/\"+f,\n \"logical\":\"root/control/button\", \"kind\":\"kind: Button\",\n};\nfor(const [k,v] of Object.entries(cands)) console.log(H(v).slice(0,16), k);\nconsole.log(\"target  edc72845cc315b0b\");\n'\n", "description": "h2: test fingerprint scheme hypotheses"}
```

**结果**：
```
06c45576d883e961 file
fc7b31bd46f526d7 file-nonl
e37b3bb956dbffaf block-19-24
bdb109aa40444860 block-19-24nl
c8e581b45cd07ec8 block-20-24
a47e58263adbecd4 path
84c2f5f491c6e266 path:19
79fc01ca814addc5 path:20
695ae0cda3c3d6ea abs
c7743e7e97b8d1ee logical
46f07fd90d08278b kind
target  edc72845cc315b0b

```

**该步推理**（逐字）：
```
Key findings for h2:

1. **The sibling missing exactly one declaration**: `spinner`. The diff `spinner.rs vs button.rs` is the only one with an extra hunk beyond the name substitutions: `23d22 < handle_contracts: [crate::control::ControlHandle],` — meaning button.rs has `handle_contracts: [...]` at line 23, and spinner.rs **lacks it** (the diff direction: `<` is button, `>` is the other; `23d22` = delete line 23 from button to get spinner). So spinner's `control_object!` block is missing exactly one declaration: `handle_contracts: [crate::control::ControlHandle]`.

Wait, let me double check the direction: `diff button/button.rs $o/$o.rs` → `<` lines are from button.rs, `>` lines are from the other file. `23d22` with `<     handle_contracts: [crate::control::ControlHandle],` means: button.rs line 23 must be deleted to match the other file's line 22 → spinner.rs lacks that line ✓. So spinner is the "同族只缺一条声明" sibling ✓.

Let me verify the spinner file's macro block by reading it via codegraph (need line numbers in the answer + the raw evidence in the log).

2. **Ledger**: neither file's sha256 matches `edc72845…`. So the fingerprint isn't plain sha256 of the file bytes. Hmm. Both entries carry the same fingerprint. Which entry "no longer describes the bytes it names"?

Options:
(a) The fingerprint is a hash of something else (e.g. of the declared face's *shape* — the macro block's declaration text, or the registration fields).
(b) Maybe it's the sha256 of the file's *content* as of the adoption; since files changed since (spinner etc.), both could be stale. The brief says exactly ONE entry lapsed.

Let me think: the button entry says "the reference sibling shape" and the panel entry says "recorded before the panel was rewritten". Both fingerprints identical → one of them is a copy of the other. The panel was rewritten (so its bytes changed) — the note tells us the panel's entry was recorded *before* the rewrite, so its fingerprint doesn't describe the current bytes ⇒ **the panel entry is the lapsed one** ✓. The button entry is the "真实标本" (real specimen) whose fingerprint presumably matches its bytes... but plain sha256 doesn't match either. So the fingerprint must be computed over something else — likely the *shape* (declaration), not the raw file. Let me test a few candidate definitions, e.g.:
- sha256 of the macro block text,
- sha256 of the file with the doc-comment/name normalized,
- sha256 of the "declared face" summary (kind + exports + parent + handle_contracts) — a shape hash which would be EQUAL for button and panel (both have the same shape: kind differs though...).

Hmm: if the fingerprint were the shape (exports + parent + handle_contracts), then button and panel would share it, and "panel's entry recorded before the panel was rewritten" would mean the panel's shape changed — but panel's current shape matches button's... 

Let me look for evidence of the fingerprint scheme in the tree itself: maybe the tree has a README or the entries file's header/format doc, or a test asserting the ledger. The tests in the h2 tree: only offsets.rs (which passes). Also maybe the host's src has an "adopted" module that reads the entries file? Let me check the host src/lib.rs of h2 (does it mention `.nichlink/adopted`?). And the file layout: h2's host has `.nichlink/adopted/entries` only.

The scheme might be documented in the workspace's toolchain source (call_evidence / adoption), which I'm not supposed to grep... but I CAN use codegraph MCP on the workspace? That would contaminate the measurement. Alternatively, the h2 BRIEF says "one ledger entry no longer describes the bytes it names" — the diagnosis likely just needs: "panel entry's fingerprint is stale (identical to button's, recorded before the rewrite); button's entry is the specimen". Since both fingerprints are identical and one is claimed to be a real specimen, the most probable intent: the fingerprint scheme is `sha256(file bytes)`... but then neither matches.

Hold on — maybe the fingerprint is over the file *without* the doc comments, or over the `control_object!` block, or over the file with normalized line endings... Let me test a couple of hypotheses computationally:
1. sha256 of file bytes → 06c45576 (button), d886e9b0 (panel). Neither is edc72845.
2. sha256 of the file's declared "shape" line(s)?

Let me try: sha256 of the macro block text (lines 19-24 of button.rs). Compute a few candidates quickly with a script. Candidates for button.rs:
- the `control_object! { ... }` block text (exact),
- the whole file minus the first doc line,
- the file with `Button`→`X` normalized... 

Hmm, a cleaner idea: maybe the fingerprint is the sha256 of the *face's declaration* as the toolchain defines it — e.g. of `kind + exports + parent + handle_contracts` formatted as in the macro. If the panel and button have the same shape (modulo kind), normalizing the kind name away would give the same hash — but the fingerprint is "edc72845" for both, which would then be consistent with "the reference sibling shape" being shared.

But which is lapsed? The note: panel's entry "recorded before the panel was rewritten" ⇒ its fingerprint describes the *old* panel bytes. If the current panel shape equals button's shape, then the old panel's fingerprint (edc72845) is the same as the button's current one only if the OLD panel had the same shape as the button... Circular.

Let me test hypotheses about what hashes to edc72845. Let me try:
(a) sha256 of button.rs with the struct/trait names normalized? 
(b) sha256 of the `control_object!` block:
```
crate::control_object! {
    kind: Button,
    exports: ["control.render"],
    parent: crate::control::NODE_ID,
    handle_contracts: [crate::control::ControlHandle],
}
```
(c) sha256 of just `exports: ["control.render"]` etc.
(d) Maybe the fingerprint is a *node id* — e.g. NodeId = sha256 of the file path + line? The doc we read said: a face's `NodeId` comes from `file!()` and the registration site; "身份会写入落盘的 graf
…[推理：此处截断，全文 8468 字符；原件见 outputs/ 里的对话渲染脚本可重跑]
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h2-claim-unkept：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/probe-cg26/trees/h2-claim-unkept/host; node -e '\nconst fs=require(\"fs\"),{createHash}=require(\"crypto\");\nconst H=s=>createHash(\"sha256\").update(s).digest(\"hex\");\nconst f=\"src/control/object/button/button.rs\"; const b=fs.readFileSync(f,
…[参数：此处截断，全文 957 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`

**调用** `bash`（step 4）：
```
{"command": "cd /home/nich/Moirai_N3/nichlink/target/probe-cg26; python3 - <<'PY'\nimport hashlib\ndef fp(path, rel):\n    b=open(path,'rb').read(); h=hashlib.sha256()\n    h.update(rel.encode()); h.update(b'\\x00'); h.update(str(len(b)).encode()); h.update(b'\\x00'); h.update(b); h.update(b'\\x00')\n    return h.hexdigest()\nbase='trees/h2-claim-unkept/host/'\nfor rel in ['src/control/object/button/button.rs','src/control/object/panel/panel.rs']:\n    print(f'{rel:42s} adoption={fp(base+rel, rel)}')\nprint('ledger (both rows)                       = edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9')\nPY", "description": "h2: adopt fingerprint of button vs panel"}
```

**结果**：
```
src/control/object/button/button.rs        adoption=edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9
src/control/object/panel/panel.rs          adoption=57afd3b6e4eca34db53153bec87af9d5cb7f44cf7864067c999e98ccee686e62
ledger (both rows)                       = edc72845cc315b0b2c08dd89bc29600abfa50ccee87c44a4231f1260745af8e9

```

**该步推理**（逐字）：
```
The h2 path is still wrong: `trees/h2-claim-unkept/host/src/control/object/button/button.rs` — my script used `'trees/h2-claim-unkept/'+rel` where rel = `src/control/...`, giving `trees/h2-claim-unkept/src/...` ✗. The host prefix is missing. Let me fix: `trees/h2-claim-unkept/host/src/control/object/button/button.rs`.

And the nonbridge log entries:
- h2: 3 calls (455 + 1013 + 2225 = 3,693 B) ✓ matches my earlier tally (3 / 3,693).
  Note the third logged call is the `sha256sum`+`diff` one (2,225 B) — which I re-ran through nb.sh, so it IS logged ✓.
- fc: 6 calls (467+200+623+3296+3296+273 = 8,155 B) ✓ matches.

So h2's stated line says "4 次 / 约 3.5 KB" — the count 4 includes a call that wasn't logged separately (or I double-counted). The correct logged figures: 3 次 / 3,693 B. Fix h2's line to `nonbridge: 3 次 / 3,693 B（日志内；另有一次早前的 `diff`/`sha256sum` 直连 bash 未入日志 ⇒ 下界，见 SETUP.md §5）`.

Hmm, careful: is that "另有一次早前…未入日志" true for h2? Yes: I ran the per-sibling diff once directly (in the "hash named files and diff siblings vs button" call) before re-running it via nb.sh. So h2's logged count 3 + that unlogged direct run = 4 executions. So the answer's "4 次" was the *execution* count while the log has 3. To align with the log: state 3 (logged) and note the unlogged extra → lower bound. 

For fc: 6 / 8,155 ✓ aligned exactly. I'll add "（日志内，逐条在 logs/fc.txt；本栏为下界，见 SETUP.md §5）".

Also: the fc log's 3rd entry (623 B) is the failed compile run; 4th (3,296) the green run; 5th (3,296) all-features; 6th (273) tree after ✓.

Now let me also fix h2's fingerprints check (with the correct path) to substantiate the claim "panel 那行的指纹是别的文件的当前指纹".

Let me run that corrected computation.

Then do the edits:

A. h2-claim-unkept.md:
   - ③ fix wording (remove "或删掉该失效条目", add the human-reconfirmation via bridge `adopted … --apply`, append a confirmation row, keep old rows as history).
   - ④ add the evidence: the panel row's fingerprint == the *button* file's current adoption fingerprint (recomputed: `edc72845…`), i.e. the record points at another file's bytes ⇒ 记录失真（需人工重确认），不是代码缺陷.
   - nonbridge line: 3 次 / 3,693 B + lower-bound note.
   - Keep ≤20 lines (currently 14; edits may add 1-2 lines → OK).

B. fc.md:
   - ③ last bullet: rewrite the reasoning (recomputable via `adoption_fingerprint`, cite kernel path + mcp call site; flow reason).
   - nonbridge line: keep 6 / 8,155 + note lower bound.
   - ≤20 lines (currently 19 — careful! Adding text may push to 20+; keep it tight, maybe trim elsewhere).

C. CORRECTIONS.md: 3 (or 4) entries with what/why/evidence (citing `docs/audit-2026-10-02/cg26-review-2.md` §4.1 / §4.3 / §3.x line refs, plus my own recomputation).

D. SETUP.md §5: lower-bound rule + per-question logged table + the note about unlogged diff/sha calls; also note the superseded in-flight totals (234/38/357,058) and the current consistent ones.

Let me first run the corrected h2 fingerprint check.
```

**同一步并行发出的其它调用**：
- `bash` → 归属 h2-claim-unkept：`{"command": "cd /home/nich/Moirai_N3/nichlink/target/probe-cg26; python3 - <<'PY'\nimport hashlib\ndef fp(path, rel):\n    b=open(path,'rb').read(); h=hashlib.sha256()\n    h.update(rel.encode()); h.update(b'\\x00'); h.update(str(len(b)).encode()); h.update(b'\\x00'); h.update(b); h.update(b'\\x00')
…[参数：此处截断，全文 688 字符；原件见 outputs/ 里的对话渲染脚本可重跑]`


## 三、交付答案（逐字）

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