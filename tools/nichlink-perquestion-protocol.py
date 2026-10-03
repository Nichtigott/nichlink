#!/usr/bin/env python3
"""逐题协议（per-question protocol）· 第十轮：**一题一会话、一题一棵树、两臂同形状**。

为什么要换这个跑法：多题一会话时，"逐题 token"在**构造上**就切不出来（两臂都在会话开头做跨题批扫、
之后用 `cd` 后的相对路径干活），任何切法都是假的；而"整轮"又因两侧回合结构不同而不可比。
⇒ **一题一会话**之后，每一题的 token 就是**那个会话自己的 usage 合计**（harness 本来就记了，不需要归属切分）。

三个子命令：
  `setup`   —— 从 `target/round9/trees/` 制备两臂各自的 26 份副本（+ `carrier/`），并**逐棵复算 sha256 对回 `TREES.json`**（含 `g1`=s3、`g2`=s5 的别名）；
  `brief <arm> <id>` —— 打印该题的 run brief（题面 + 工具 + 记录要求 + 禁令），供一次性会话直接使用；
  `measure` —— 扫描会话目录，按每题任务书里的探针串 `ROUND10 <arm> <id>` 找到**该题的会话**，
              从它自己的 `usage` 合计出三档 token / 步 / 推理字符（**无需归属切分** ✓），再与逐题仪器日志对账。

**token 口径**：`usage` 是**每步增量**（`inputTokens` / `cacheReadTokens` / `write` / `outputTokens`），
一题一会话时**直接求和**就是该题的用量；要分"答题段 / 装置段"就在两个时刻的累计值上**做差**（本轮不需要）。
两侧同一把尺：同一套字段、同一个求和规则、同样把"索引成本"（codegraph 的 `init`）单列。
"""
import argparse, hashlib, json, pathlib, re, shutil, subprocess, sys, collections

ROOT = pathlib.Path(__file__).resolve().parent.parent
R9 = ROOT / 'target/round9/trees'
R10 = ROOT / 'target/round10'
SESS = pathlib.Path.home() / '.dsh/sessions/--home-nich-Moirai_N3-nichlink--'
SKIP = {'target', '.codegraph', '.git'}

IDS = ['r1', 'r2', 'r3', 'r4', 's1', 's2', 's3', 's4', 's5', 's6', 's7', 's8',
       'g1', 'g2', 'g3', 'g4', 'h1', 'fa', 'fb', 'fc', 'fd', 'fe',
       'h1-supply-chain', 'h2-claim-unkept', 'h3-cross-file-chain', 'h4-one-file-many-algorithms']
ALIAS = {'g1': 's3', 'g2': 's5'}                      # 复用根：g1≡s3、g2≡s5（各拷一份独立副本）
QUESTION = {
    'r1': '注入缺陷：`cargo test --offline` 恰有 1 条失败。交付＝根因 文件:行号 + 最小修 + 反证。',
    'r2': '注入缺陷：`cargo test --offline` 恰有 1 条失败。交付＝根因 文件:行号 + 最小修 + 反证。',
    'r3': '注入缺陷：`cargo test --offline` 恰有 1 条失败。交付＝根因 文件:行号 + 最小修 + 反证。',
    'r4': '注入缺陷：`cargo test --offline` 恰有 1 条失败。交付＝根因 文件:行号 + 最小修 + 反证。',
    's1': '谁调用了 `ledger_core::store::Store::post`（调用者在另一个 crate）。',
    's2': '全绿但有一个面是红的，是哪个面。', 's3': '哪些函数没有任何测试能到达。',
    # s4 的题面在源 BRIEF 里没有先行词（"这个类型"），实测同一臂两次跑会读成不同实体
    # （Store vs Entry），而第七轮预设是 Store 口径 ⇒ 本轮把所指写死，保证两臂同题。
    's4': '哪些测试文件能到达 `ledger_core::store::Store`（即 s1 点名的那个类型）、经哪条路。', 's5': '渲染缺了什么。',
    's6': '契约与实现一致吗。', 's7': '分桶的上限含不含 1000。', 's8': '条数行的位置。',
    'g1': '还有哪些地方是没有任何测试能到达的——给可核对的具体函数 文件:行，并说出你看不见什么。',
    'g2': '用一段伪代码说明这段渲染逻辑想做什么，再指出实现与意图的差。',
    'g3': '给这个对象加一层内部结构，但不要动它的注册树和公开路径；说明凭什么相信没动，门必须绿。',
    'g4': '这棵树里有哪些臂是没有任何执行能进入的；给出每条判据，并说清哪些你判不了、为什么。',
    'h1': '（范围型）检查这个仓库还有没有别的问题——先给可核对的全树总账、逐栏处置。',
    'fa': '「我要加一个新对象，和现有的差不多。」', 'fb': '「这个对象内部还不够。」',
    'fc': '「已经采信了，现在再横向加一个。」（台账必须还在）', 'fd': '「这个对象有问题。」',
    'fe': '「这几个对象都有问题。」',
    'h1-supply-chain': '渲染出来的布局不对：`cargo test --offline` 在偏移测试里失败，而那个总和**没有任何单个 widget 自己的代码能解释**——每一个 widget 文件单看都正确。交付＝根因 文件:行号 + 机制 + 最小修 + 反证。',
    'h2-claim-unkept': '这份设计采纳的一条约束，在**后来的某个兄弟**身上没有成立；并且**有一条台账条目**不再描述它点名的字节。树能构建、它的测试也过——问题在台账与兄弟的形状里，不在测试运行里。',
    'h3-cross-file-chain': '`every_declared_widget_ships` 失败：源码里有一个面已经写好，但**发布出来的注册树里没有它**。找出为什么、给最小修。',
    'h4-one-file-many-algorithms': '一个文件里有多套算法，其中有的不对劲。找出问题、给判据，并说清哪一部分你判不了。',
}
# 只读题（第九轮 BRIEF §3 的原文口径）：`s*`/`g4` **与四道 hardbug 类**只读；
# `r1–r4`/`fa–fe`/`g3`/`h1` 可改。本轮我漏了 hardbug 四类（把它们标成可改），是装置错误。
READ_ONLY = {'s1', 's2', 's3', 's4', 's5', 's6', 's7', 's8', 'g1', 'g2', 'g4',
             'h1-supply-chain', 'h2-claim-unkept', 'h3-cross-file-chain', 'h4-one-file-many-algorithms'}
# 这几道题的宿主包在 `host/` 子目录里：`--root` 必须是那个 package 根，指到树根会答成"空树/faces unavailable"
PKG_ROOT = {'h1-supply-chain': 'host', 'h2-claim-unkept': 'host', 'h3-cross-file-chain': 'host'}


def treehash(root):
    h = hashlib.sha256(); files = []
    for p in root.rglob('*'):
        if p.is_file():
            rel = p.relative_to(root)
            if not any(x in SKIP for x in rel.parts):
                files.append((str(rel).replace('\\', '/'), p))
    for rel, p in sorted(files):
        b = p.read_bytes()
        h.update(rel.encode()); h.update(b'\x00'); h.update(str(len(b)).encode()); h.update(b'\x00'); h.update(b); h.update(b'\x00')
    return h.hexdigest()


def setup():
    M = json.loads((ROOT / 'target/round9/TREES.json').read_text())['trees']
    for arm in ('ours', 'cg'):
        for d in ('trees', 'logs', 'answers'):
            (R10 / arm / d).mkdir(parents=True, exist_ok=True)
        for i in IDS:
            src = R9 / ALIAS.get(i, i)
            dst = R10 / arm / 'trees' / i
            if dst.exists():
                shutil.rmtree(dst)
            shutil.copytree(src, dst, ignore=shutil.ignore_patterns('target', '.codegraph', '.git'))
        car = R10 / arm / 'trees' / 'carrier'
        if car.exists():
            shutil.rmtree(car)
        # `carrier/` 住在 `target/round9/trees/carrier`（它是 fa/fb/fc/g3 的相对依赖目标）。
        # 少了 `trees/` 这一层时 `copytree` 会直接报错 —— 这条路径在制备那次没被跑到过。
        shutil.copytree(R9 / 'trees' / 'carrier', car, ignore=shutil.ignore_patterns('target', '.codegraph', '.git'))
        ok, bad = 0, []
        for i in IDS:
            want = M[ALIAS.get(i, i)]['sha256']
            got = treehash(R10 / arm / 'trees' / i)
            if got == want:
                ok += 1
            else:
                bad.append((i, got[:16], want[:16]))
        print(f'{arm}: 副本 {ok}/{len(IDS)} 对回登记哈希' + ('' if not bad else f'  ✗ 不符 {bad}'))
        # T-29：`carrier/` 也在这份副本里，而 `fa`/`fb`/`fc`/`g3` 的 Cargo.toml 依赖它
        # （`../carrier/control-button-graft`）⇒ 不把它算进清单，"装置未被动过"的自证就有洞：
        # 谁改了 carrier，那些题照样"对回登记哈希" ✓。它登记在 TREES.json 的 `carrier` 键下。
        # T-29: `carrier/` rides in the same copy and four questions depend on it, so leaving it out
        # of the manifest leaves a hole in "the apparatus was not touched".
        if 'carrier' in M:
            want = M['carrier']['sha256']
            got = treehash(R10 / arm / 'trees' / 'carrier')
            mark = '对回' if got == want else f'✗ 不符（实得 {got[:16]}，登记 {want[:16]}）'
            print(f'{arm}: carrier/ {mark}')
    print(f'装置根：{R10}（ours/ 与 cg/ 各有 trees·logs·answers）')


# 第十二轮（干净重跑）的装置根：**新开一棵**，于是上一轮的答案/日志/工作台都不是本轮的目标路径。
# 这一条就是第十一轮 D1/D5 的修法 —— 路径只在一个常量里决定，不手建目录。
# The round-12 root: a fresh tree, so the previous round's answers, logs and benches are not this
# round's target paths. This constant is the one place the paths are decided.
R12 = ROOT / 'target/round10/r12'
R12_TAG = 'target/round10/r12'
R12_IDS = ['h1-supply-chain', 'h2-claim-unkept', 'h3-cross-file-chain',
           'h4-one-file-many-algorithms', 'r2', 'fd', 'fa', 's7']


def brief(arm, i):
    ins = ('./target/debug/nichlink-mcp' if arm == 'ours'
           else f'{ROOT}/target/round7/tools-upstream/v1.6.1/bin/codegraph')
    log = f'{R12_TAG}/{arm}/logs/{i}.' + ('jsonl' if arm == 'ours' else 'txt')
    root = f'{R12_TAG}/{arm}/trees/{i}' + (('/' + PKG_ROOT[i]) if i in PKG_ROOT else '')
    if arm == 'ours':
        how = (f'- 工具：`{ins} --call <tool> --root {root} …`，**每次调用都带** `--log {log}`。\n'
               f'- 先 `--call status`/`--call registry` 摸树，再用 check/why/callgraph/consistency/conformance/read/digest 等。')
    else:
        how = (f'- 工具：`{ins}`（子命令 `init/files/node/query/callers/callees/explore/affected`）。\n'
               f'- 你在**自己的副本**里工作（`{root}`，可写）：先 `cd {root} && {ins} init .`。\n'
               f'- 每次调用把原文追加到 `{log}`，三行式：`=== CMD: …` / 原始输出 / `=== EXIT: N`。')
    ro = '**只读**（不许改这棵树）' if i in READ_ONLY else '可改，但交付后 `cargo test --offline`（cg 臂同样）必须绿'
    return f"""ROUND10 {arm} {i}

你是「{arm}」这一臂的答题者，**只答这一道题**（一题一会话）。题号 `{i}`。

**题面**：{QUESTION[i]}

**树**：`{root}`（{ro}）
{how}

**答案**写到 `target/round10/r12/{arm}/answers/{i}.md`，固定五段形状：
① 症状一句，**并且先把基线写进来**：`cargo test --offline` 的**原始结论**（题目态下按题面应当有红；把原始结论抄进来）。**若基线全绿而题面说有缺陷，立刻停下并上报**——不要从 mtime、增量产物或上一轮的日志里考古（第十轮 r2 为此花了 15 步、
推理是对方的 5.25×，全部买的是"一个已不在盘上的缺陷态"）；
② 根因/结论 `文件:行号` + 机制一句；③ 最小修或改动清单；
④ 反证：**只改一处**、重跑、观察**同一条**症状是否仍在（独立性实验允许"**一次改两处 + 反向变异确认**"，
不必逐个隔离到只剩一处）。改源码**用 `edit` 工具，不许用 `sed -i`**——它会吃掉未转义的 `&`，
**还原文件也不要用 `cp` 备份/回拷**——编辑器会把文件判成"读后被改"并要求重读一次
（`file changed since it was read — re-read the file, then retry`）；那是第十轮唯一一笔与题目无关的
纯工具税（`fd` 步 27–28）。要撤销就用 `edit` 反向改回去。
第十轮量到为此白花 3 步并在善后里又花 2 步；
⑤ 调用清单（工具 + 作用，一行一条）。

**基线一律用树外的 target 目录**（第十一轮最险的一条装置缺陷）：`CARGO_TARGET_DIR=<树外新目录> cargo test --offline`
—— 还原过的树若留着上一轮的 `target/`，源码 mtime 会旧于产物，cargo 判 fresh、**不重编**、直接跑旧二进制
⇒ **报全绿而其实是上一轮的二进制**（本轮 `h1s` 与 `fd` 的第一次都这么被骗过）。**判据**：看有没有
`Compiling` —— `Finished in 0.0x s` 且没有编译行，就是"跑的不是这份源码"。取完基线请把用的那个目录写进交付。

**工具用法**（省一步）：`agent_teams_update_task` 的 `evidence_note` **只对终态任务有效**（第十轮 52 份切片里
15 份出现过 `evidence_note is for terminal tasks`，每次多花一步）；提交长 payload 可能触发
`MALFORMED_RESPONSE`（任务被记 failed 而工作白做）⇒ **先交短 completion，再把证据分次 append**
（`acceptanceResults` / `commandsRun` / `evidence_note` 都是 append-only）。

**禁令（第十二轮收紧）**：**除本轮装置根 `target/round10/r12/` 之外，整个 `target/` 与整个 `docs/`
都在禁令内** —— 包括 `target/round7|8|9`、以及 `target/round10/` 下**除 `r12/` 外**的一切
（`ours/answers/`、`ours/logs/`、`ours/work-*`、`_round10_artifacts/`、`deepdive/`、`chain/`、
`ours/answers-r11/` 等：那里有**上一轮的答案、工作台、逐题分析与渲染**）。
**为什么写成"除本轮根之外全域"而不是枚举目录**：第十轮枚举 `round7|8|9` ⇒ 漏了 7 个 ⇒ 判废 8 题；
第十一轮枚举得稍全，但同名的 `answers/`、`logs/` 与 `work-*` 又成了新的泄漏面（5 题中招）。
枚举必然不全 ✗，所以规则改成**白名单一个根**。
**唯一可读的例外**：**你自己那一份任务书** `target/round10/briefs/ours-<题号>.md`；cg 臂的 codegraph
可执行文件在 `target/round7/tools-upstream/` 下，那一个可执行文件可用。
**也不许把那些目录作为任何命令的搜索路径**（`grep -r`/`find`/`rg`/`ls -R` 一律限定在 `target/round10/r12/` 内）——
全仓搜索会顺带把对照答案与真值的**片段**打印出来（实测一次 `grep -rn` 就命中了 `round7/answer-*-g2.md` 与对话记录里的答案正文），
那同样算泄漏、会导致该题判废重跑。不许 `git log/show` 查题树历史；凡断言"某条路径"（X 经 Y 到 Z），每一跳都要回源码定义处核过（不许由命名推断）。
发现装置问题（树不对、工具异常）单独写 `target/round10/r12/{arm}/answers/APPARATUS-{i}.md`：**≤15 行**，只写"症状 / 证据（命令+原始输出）/ 你的判断"。发现装置缺陷仍然要写（那是唯一途径），但**不要写成报告**，把笔墨留给答案本身。"""


def _first_user_text(path):
    """该会话的**第一条 user/message** 的文本 —— 那才是它自己的任务书。

    不能拿"全文含探针串"定位 ✗：别的题的会话、以及队长的会话都可能引用到那个串
    （实测：`cg-r3` 的会话里出现过 `ROUND10 ours r3`，于是它被算成了 ours-r3）。
    """
    proc = subprocess.Popen(['zstd', '-dc', str(path)], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    raw = proc.stdout.read(400000).decode('utf-8', 'replace')      # 第一条 user message 就在会话开头
    proc.kill()
    for line in raw.splitlines():
        if not line.strip():
            continue
        try:
            r = json.loads(line)
        except Exception:
            continue
        if r.get('type') == 'user/message':
            d = r.get('data') or {}
            return json.dumps(d.get('content'), ensure_ascii=False) if not isinstance(d.get('content'), str) else d['content']
    return ''


SLICE_LIMIT = 1100


def _clip(text, limit=SLICE_LIMIT):
    """截断**必须带标记**。

    第十轮切片对每条返回硬截在 ~901 可见字符且**无标记**，1207 条里 661 条（55%）被截 ——
    于是所有"逐行统计元信息"的结果只能给下界，而读者无法分辨"原文没有这行"与"被我切掉了"。
    """
    if len(text) <= limit:
        return text
    return text[:limit] + f"…（截断：本条共 {len(text)} 字符）"


SLICE_LIMIT = 1100
SLICE_IDS = ['r1','r2','r3','r4','s1','s2','s3','s4','s5','s6','s7','s8','g1','g2','g3','g4',
             'h1','fa','fb','fc','fd','fe','h1-supply-chain','h2-claim-unkept','h3-cross-file-chain',
             'h4-one-file-many-algorithms']


def _clip(text, limit=SLICE_LIMIT):
    """截断**必须带标记**。

    第十轮切片对每条返回硬截在 ~901 可见字符且**无标记**：1207 条里 661 条（55%）被截 ⇒ 所有
    "逐行统计元信息"的结果只能给下界，而读者分不清"原文没有这行"与"被我切掉了"。标上总数之后，
    读者至少知道自己看到的是片段。
    """
    if len(text) <= limit:
        return text
    return text[:limit] + f"…（截断：本条共 {len(text)} 字符）"


def slices(out_dir='target/round10/dialogues'):
    """把每题每臂**最新一次**会话切成"逐条配对"的对话（推理 / 调用 / 返回），供归因分析用。"""
    out = pathlib.Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    made = []
    for i in SLICE_IDS:
        for arm in ('ours', 'cg'):
            h = latest_session(arm, i)
            if not h:
                continue
            sid, f = h
            proc = subprocess.Popen(['zstd', '-dc', str(f)], stdout=subprocess.PIPE)
            raw = proc.stdout.read().decode('utf-8', 'replace')
            proc.kill()
            lines = []
            step = 0
            for line in raw.splitlines():
                if not line.strip():
                    continue
                try:
                    row = json.loads(line)
                except Exception:
                    continue
                kind = row.get('type')
                data = row.get('data') or {}
                if kind == 'assistant/message':
                    step += 1
                    for part in ((data.get('message') or {}).get('content') or []):
                        if not isinstance(part, dict):
                            continue
                        if part.get('type') == 'reasoning' and part.get('text', '').strip():
                            lines.append(f'**[{step}] 想**：{_clip(part["text"].strip(), 900)}')
                        elif part.get('type') == 'text' and part.get('text', '').strip():
                            lines.append(f'**[{step}] 说**：{_clip(part["text"].strip(), 500)}')
                        elif part.get('type') == 'tool-call':
                            args = json.dumps(part.get('arguments'), ensure_ascii=False)
                            lines.append(f'**[{step}] 调** `{part.get("name")}` {args[:600]}')
                elif kind == 'tool/result':
                    msg = json.dumps(data.get('message'), ensure_ascii=False)
                    lines.append(f'**[{step}] 返**（{len(msg)} 字符）：{_clip(msg)}')
            (out / f'{arm}-{i}.md').write_text(f'# {arm} · `{i}`\n\n会话 {sid}\n\n' + '\n\n'.join(lines) + '\n')
            made.append(f'{arm}-{i}')
    return made


def latest_session(arm, i):
    """该题该臂**最近一次**运行的会话。

    ⚠️ 不能取 `sessions_for(...)[-1]` —— 那是**按目录名排序**的最后一个，与会话时间无关；
    重跑过的题会因此取到"中间那次"（第十轮 r2 就取到了树未复位的那次，步数/输出/推理全错位）。
    """
    hits = sessions_for(arm, i)
    return max(hits, key=lambda x: x[1].stat().st_mtime) if hits else None


def reset_question(arm, i):
    """把**可改题**的副本复位回题目态，并移存上一 attempt 的日志与答案。

    重跑前必做：成员做完可改题会**改树**（第九轮就踩过），若不复位，下一 attempt 拿到的是
    "已修好的树" ⇒ 注入缺陷不可观察，答案只能靠还原实验反推（第十轮 r4/g3 都这样中招）。
    """
    if i in READ_ONLY:
        return 'read-only, nothing to reset'
    tree = pathlib.Path(f'target/round10/{arm}/trees/{i}')
    pristine = pathlib.Path(f'target/round9/trees/{i}')
    if not tree.exists() or not pristine.exists():
        return 'missing tree'
    # 注意：**不能排除 `.nichlink`** —— 台账是题目态的一部分（fc 的题面就是"已有采信台账"），
    # 排除它会让"上一 attempt 追加过台账"的树看起来是干净的（第十轮 fc 就这么漏过）。
    subprocess.run(['rsync', '-a', '--delete', '--exclude', '.codegraph', '--exclude', '.git',
                    '--exclude', '.cargo', '--exclude', '.cg-target',
                    str(pristine) + '/', str(tree) + '/'], check=False)
    # **树内的 `target/` 必须删掉**：它是上一 attempt 的构建产物，会让第一次 `cargo test --offline`
    # 假绿（cargo 认为已构建 ⇒ 不重编 ⇒ 跑的是旧二进制，可能已是修复态），
    # 于是"注入缺陷"看不见（第十轮 r2 干净重跑时实测：fresh 指纹 0.05s 9 passed 假绿）。
    shutil.rmtree(tree / 'target', ignore_errors=True)
    for kind, ext in (('logs', 'jsonl' if arm == 'ours' else 'txt'), ('answers', 'md')):
        src = pathlib.Path(f'target/round10/{arm}/{kind}/{i}.{ext}')
        if src.exists():
            src.rename(src.with_suffix(f'.voided.{ext}'))
    return 'reset'


def sessions_for(arm, i):
    """按**该题任务书里的独特路径**在"第一条 user message"里定位（一题一会话 ⇒ 恰一个）。"""
    # 两种写法都要认：详细任务书写了 `logs/<id>.…`，简短任务书只写了 `briefs/<arm>-<id>.md`
    probes = [f'target/round10/{arm}/logs/{i}.', f'target/round10/briefs/{arm}-{i}.md']
    hits = []
    for d in sorted(SESS.iterdir()):
        if not d.is_dir():
            continue
        for f in sorted(d.glob('session*.jsonl.zstd')):
            head = _first_user_text(f)
            if any(pr in head for pr in probes):
                hits.append((d.name, f))
    return hits


def measure():
    M = json.loads((ROOT / 'target/round9/TREES.json').read_text())['trees']
    print(f'{"题":26s}{"臂":6s}{"会话":38s}{"步":>4s}{"未命中":>10s}{"缓存读":>12s}{"输出":>8s}{"推理字符":>9s}{"仪器":>5s}')
    agg = collections.defaultdict(collections.Counter)
    for i in IDS:
        for arm in ('ours', 'cg'):
            hits = sessions_for(arm, i)
            if not hits:
                print(f'{i:26s}{arm:6s}{"(未找到会话)":38s}')
                continue
            sid, f = hits[-1]
            raw = subprocess.run(['zstd', '-dc', str(f)], capture_output=True).stdout.decode('utf-8', 'replace')
            u = collections.Counter(); steps = 0
            for line in raw.splitlines():
                if not line.strip():
                    continue
                try:
                    r = json.loads(line)
                except Exception:
                    continue
                if r.get('type') != 'assistant/message':
                    continue
                us = (r.get('data') or {}).get('usage') or {}
                if us.get('cacheReadTokens') is None:
                    continue
                steps += 1
                u['in'] += us.get('inputTokens', 0); u['cr'] += us.get('cacheReadTokens', 0)
                u['out'] += us.get('outputTokens', 0)
                parts = ((r['data'].get('message') or {}).get('content')) or []
                u['reason'] += sum(len(p.get('text', '')) for p in parts if isinstance(p, dict) and p.get('type') == 'reasoning')
            logp = R10 / arm / 'logs' / (f'{i}.jsonl' if arm == 'ours' else f'{i}.txt')
            inst = 0
            if logp.exists():
                t = logp.read_text(errors='replace')
                inst = len([l for l in t.splitlines() if l.strip()]) if arm == 'ours' else t.count('=== CMD:')
            agg[arm].update(u); agg[arm]['steps'] += steps; agg[arm]['inst'] += inst; agg[arm]['n'] += 1
            print(f'{i:26s}{arm:6s}{sid:38s}{steps:>4d}{u["in"]:>10,}{u["cr"]:>12,}{u["out"]:>8,}{u["reason"]:>9,}{inst:>5d}')
    print()
    for arm in ('ours', 'cg'):
        a = agg[arm]
        den = a['in'] + a['cr']
        print(f"{arm}: {a['n']} 题 · 步 {a['steps']} · 未命中 {a['in']:,} · 缓存读 {a['cr']:,} · "
              f"输出 {a['out']:,} · 推理 {a['reason']:,} · 仪器 {a['inst']} · 命中率 {a['cr']/den*100:.2f}%")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('cmd', choices=['setup', 'brief', 'measure', 'sessions'])
    ap.add_argument('args', nargs='*')
    a = ap.parse_args()
    if a.cmd == 'setup':
        setup()
    elif a.cmd == 'brief':
        print(brief(a.args[0], a.args[1]))
    elif a.cmd == 'sessions':
        for i in IDS:
            for arm in ('ours', 'cg'):
                h = sessions_for(arm, i)
                print(f'{i:26s}{arm:6s}{[x[0] for x in h]}')
    else:
        measure()


if __name__ == '__main__':
    sys.exit(main())
