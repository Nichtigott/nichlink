#!/usr/bin/env python3
"""第九轮对话层提取器（只读、可重跑）。把两臂会话按**题**切成四样东西：

① **工具给了什么 + 当时 agent 在想什么**：逐条配对——该题逐题日志里的每一次仪器调用
   （`target/round9/logs/<id>.jsonl` / `target/probe-cg26/logs/<id>.txt`，逐字）紧挨着
   会话里那一步的 reasoning 与输出文本（逐字），以及同一步并行发出的其它调用。
   **这是回答"agent 为何在此时这么决定"的主证据链**。
② **该题里 agent 的其它动作**（读文件 / 编辑 / 跑测试 / 写答案），带那一步的推理。
③ **交付答案**全文。
④ **读数**（步 / 输出 / 推理 / 累计上下文 / 仪器调用数）。

另出 `_apparatus.md`（不点名任何题的步：读 BRIEF、摸树、写脚本、批量哈希、写报告）、
`_shared.md`（一次调用点名多题）与 `_briefing.md`（任务书与往返）。

**口径与 `tools/nichlink-per-question-cost.py` 逐字同源**（同一条 `hits()`、同一个顺序状态机），
因此逐题读数可与 `docs/audit-2026-10-02/per-question-cost.md` 对账。本脚本只读题树与会话，
输出只落在 `docs/audit-2026-10-02/dialogues/`。
"""
import json, pathlib, re, subprocess, collections, sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SESS = pathlib.Path.home() / '.dsh/sessions/--home-nich-Moirai_N3-nichlink--'
OUT = ROOT / 'docs/audit-2026-10-02/dialogues'

ARMS = [
    ('ours', 'aa54ec58-44bf-4f88-a2ed-d0dd11614b93',
     ROOT / 'target/round9/logs', ROOT / 'target/round9/answers'),
    ('cg', '225f5295-20fb-4d73-8bb7-375b9ee71a5b',
     ROOT / 'target/probe-cg26/logs', ROOT / 'target/probe-cg26/answers'),
]

IDS = ['r1', 'r2', 'r3', 'r4', 's1', 's2', 's3', 's4', 's5', 's6', 's7', 's8',
       'g1', 'g2', 'g3', 'g4', 'h1', 'fa', 'fb', 'fc', 'fd', 'fe',
       'h1-supply-chain', 'h2-claim-unkept', 'h3-cross-file-chain', 'h4-one-file-many-algorithms']
IDS = sorted(IDS, key=len, reverse=True)          # 长名优先：h1-supply-chain 不被 h1 抢先

QUESTION = {
    'r1': '注入缺陷：`cargo test --offline` 恰有 1 条失败；交付＝根因 文件:行号 + 最小修 + 反证。',
    'r2': '同上（注入缺陷 2）。', 'r3': '同上（注入缺陷 3）。', 'r4': '同上（注入缺陷 4）。',
    's1': '谁调用了 `ledger_core::store::Store::post`（调用者在另一个 crate）。',
    's2': '全绿但有一个面是红的，是哪个面。',
    's3': '哪些函数没有任何测试能到达。',
    's4': '哪些测试文件能到达这个类型、经哪条路。',
    's5': '渲染缺了什么。',
    's6': '契约与实现一致吗。',
    's7': '分桶的上限含不含 1000。',
    's8': '条数行的位置。',
    'g1': '（只读 s3 的树）这个仓库里还有哪些地方是没有任何测试能到达的——给可核对的具体函数 文件:行，并说出你看不见什么。',
    'g2': '（只读 s5 的树）用一段伪代码说明这段渲染逻辑想做什么，再指出实现与意图的差。',
    'g3': '给这个对象加一层内部结构，但不要动它的注册树和公开路径；说明凭什么相信没动，门必须绿。',
    'g4': '这棵树里有哪些臂是没有任何执行能进入的；给出每条判据，并说清哪些你判不了、为什么。',
    'h1': '（范围型）检查一下这个仓库还有没有别的问题——先给可核对的全树总账、逐栏处置。',
    'fa': '五族：「我要加一个新对象，和现有的差不多。」',
    'fb': '五族：「这个对象内部还不够。」',
    'fc': '五族：「已经采信了，现在再横向加一个。」',
    'fd': '五族：「这个对象有问题。」',
    'fe': '五族：「这几个对象都有问题。」',
    'h1-supply-chain': 'hardbug h1：供应链驳杂——同族一个兄弟调用了另一族名字，症状是偏移 160≠136。',
    'h2-claim-unkept': 'hardbug h2：采信未落实——真台账 + 一个同族只缺一条声明 + 一条指纹失效。',
    'h3-cross-file-chain': 'hardbug h3：跨文件因果链——入口计划从没点名新面，构建期作用域把它排除。',
    'h4-one-file-many-algorithms': 'hardbug h4：单文件多算法——一个文件三套算法，恰一支写反、恰一支不可达。',
}

RESULT_CAP = 2500          # 工具结果每段最多渲染多少字符（超出标注，不静默截断）
ARG_CAP = 2000             # 命令/参数每段最多渲染多少字符
INSTR_CAP = 5000           # 仪器日志单条响应上限
REASON_CAP = 6000          # 单步推理上限（逐字，通常远小于它）


def hits(blob):
    """命令/参数文本里出现的题号（去重保序）。

    **与 `per-question-cost.py` 的原版规则有一处修正**：原版用子串 `trees/<id>`，于是
    `trees/h1` 命中了 `trees/h1-supply-chain` ⇒ 那道题的每一次调用都被同时算作 `h1`，
    两侧都进"共享桶"、逐题读数凭空消失。这里改成**带右边界的匹配**（题号后不得紧跟
    `-`/字母/数字/下划线），`h1` 与 `h1-supply-chain` 因此各自可归因。
    """
    found = []
    for i in IDS:
        pat = rf'(trees|answers|logs|CARGO_TARGET_DIR)[^"\\s]*[/=]{re.escape(i)}(?![-A-Za-z0-9_])'
        if re.search(pat, blob) or f'answers/{i}.' in blob or f'logs/{i}.' in blob:
            found.append(i)
    return found


def load_session(sid):
    p = SESS / sid / 'session.v4.jsonl.zstd'
    raw = subprocess.run(['zstd', '-dc', str(p)], capture_output=True).stdout.decode('utf-8', 'replace')
    return [json.loads(l) for l in raw.splitlines() if l.strip()]


def clip(text, cap, why):
    if not text:
        return ''
    if len(text) <= cap:
        return text
    return text[:cap] + f'\n…[{why}：此处截断，全文 {len(text)} 字符；原件见 outputs/ 里的对话渲染脚本可重跑]'


def msg_text(m):
    out = []
    for part in (m or {}).get('content') or []:
        if not isinstance(part, dict):
            continue
        if part.get('type') == 'text':
            out.append(part.get('text', ''))
        elif part.get('type') == 'tool-result':
            out.append(json.dumps(part, ensure_ascii=False))
    return '\n'.join(out)


def parse_instrument(arm, i):
    """该题逐题日志里的条目（逐字）：[{kind, cmd, out, exit}]。kind ∈ inst / nonbridge。"""
    items = []
    if arm == 'ours':
        p = ROOT / f'target/round9/logs/{i}.jsonl'
        if not p.exists():
            return items
        for line in p.read_text(errors='replace').splitlines():
            if not line.strip():
                continue
            try:
                r = json.loads(line)
            except Exception:
                items.append(dict(kind='inst', cmd='(日志行无法解析)', out=line, exit='?'))
                continue
            items.append(dict(kind='inst', cmd=' '.join(r.get('request') or []),
                              out=r.get('response') or '', exit=str(r.get('exit'))))
        return items
    p = ROOT / f'target/probe-cg26/logs/{i}.txt'
    if not p.exists():
        return items
    cur = None
    for line in p.read_text(errors='replace').splitlines():
        if line.startswith('=== CMD(nonbridge)'):
            cur = dict(kind='nonbridge', cmd=line.split(':', 1)[-1].strip(), out=[], exit='?')
            items.append(cur)
        elif line.startswith('=== CMD'):
            cur = dict(kind='inst', cmd=line.split(':', 1)[-1].strip(), out=[], exit='?')
            items.append(cur)
        elif line.startswith('=== EXIT') and cur is not None:
            cur['exit'] = line.split(':', 1)[-1].strip()
            cur = None
        elif line.startswith('=== NONBRIDGE-BYTES') and cur is not None:
            cur['out'].append(line)
            cur = None
        elif cur is not None and not line.startswith('=== CMD'):
            cur['out'].append(line)
    for it in items:
        it['out'] = '\n'.join(it['out']).strip()
    return items


def log_calls(arm, i):
    """仪器调用数：从逐题日志文件数（唯一可逐题归因的那一栏；口径同 per-question-cost.py）。"""
    if arm == 'ours':
        p = ROOT / f'target/round9/logs/{i}.jsonl'
        return len([l for l in p.read_text(errors='replace').splitlines() if l.strip()]) if p.exists() else 0
    p = ROOT / f'target/probe-cg26/logs/{i}.txt'
    return p.read_text(errors='replace').count('=== CMD:') if p.exists() else 0


def build(arm, sid):
    """顺序状态机归属（与 per-question-cost.py 逐字同源）+ 全文索引（推理/调用按 (turn, step) 配对）。"""
    rows = load_session(sid)

    res_by_call, step_msg, step_calls = {}, {}, collections.defaultdict(list)
    for r in rows:
        t, d = r.get('type'), r.get('data') or {}
        if t == 'tool/result':
            res_by_call[d.get('message', {}).get('toolCallId')] = msg_text(d.get('message'))
        elif t == 'assistant/message':
            u = d.get('usage') or {}
            if u.get('cacheReadTokens') is None:
                continue
            parts = (d.get('message') or {}).get('content') or []
            reasoning = '\n'.join(p.get('text', '') for p in parts if isinstance(p, dict) and p.get('type') == 'reasoning')
            text = '\n'.join(p.get('text', '') for p in parts if isinstance(p, dict) and p.get('type') == 'text')
            step_msg[(d.get('turn'), d.get('step'))] = dict(usage=u, reasoning=reasoning, text=text)

    per = collections.defaultdict(lambda: dict(steps=0, calls=0, out=0, reason=0, cache=0))
    per_inst, per_calls, per_msg = (collections.defaultdict(list) for _ in range(3))
    apparatus = dict(steps=0, calls=0, out=0, reason=0, cache=0)
    app_calls, app_msg = [], []
    shared = dict(steps=0, calls=0, out=0, reason=0, cache=0)
    shared_calls, shared_msg = [], []
    order, cur = 0, None

    def is_inst(name, args):
        return ('nichlink-mcp' in args) if arm == 'ours' else ('cg.sh' in args or 'codegraph' in args)

    for r in rows:
        t, d = r.get('type'), r.get('data') or {}
        if t == 'tool/call':
            order += 1
            name, args = d.get('name'), d.get('arguments') or ''
            h = hits(args)
            cur = h[0] if len(h) == 1 else (('shared', tuple(h)) if len(h) > 1 else None)
            rec = dict(turn=d.get('turn'), step=d.get('step'), order=order, name=name, args=args,
                       res=res_by_call.get(d['callId'], ''), hits=h, q=cur,
                       inst=is_inst(name, args))
            step_calls[(d.get('turn'), d.get('step'))].append(rec)
            if cur is None:
                apparatus['calls'] += 1
                app_calls.append(rec)
            elif isinstance(cur, tuple):
                shared['calls'] += 1
                shared_calls.append(rec)
            else:
                per[cur]['calls'] += 1
                per_calls[cur].append(rec)
                if rec['inst']:
                    per_inst[cur].append(rec)
        elif t == 'assistant/message':
            u = d.get('usage') or {}
            if u.get('cacheReadTokens') is None:
                continue
            key = (d.get('turn'), d.get('step'))
            sm = step_msg.get(key, dict(reasoning='', text=''))
            row = dict(turn=d.get('turn'), step=d.get('step'), usage=u,
                       reasoning=sm['reasoning'], text=sm['text'])
            if isinstance(cur, str):
                v = per[cur]
                v['steps'] += 1
                v['out'] += u.get('outputTokens', 0)
                v['reason'] += len(sm['reasoning'])
                v['cache'] += u.get('cacheReadTokens', 0)
                per_msg[cur].append(row)
            elif isinstance(cur, tuple):
                shared['steps'] += 1
                shared['out'] += u.get('outputTokens', 0)
                shared['reason'] += len(sm['reasoning'])
                shared['cache'] += u.get('cacheReadTokens', 0)
                shared_msg.append(row)
            else:
                apparatus['steps'] += 1
                apparatus['out'] += u.get('outputTokens', 0)
                apparatus['reason'] += len(sm['reasoning'])
                apparatus['cache'] += u.get('cacheReadTokens', 0)
                app_msg.append(row)
    return dict(per=per, per_inst=per_inst, per_calls=per_calls, per_msg=per_msg,
                app_calls=app_calls, app_msg=app_msg, shared_calls=shared_calls, shared_msg=shared_msg,
                shared=shared, apparatus=apparatus, step_msg=step_msg, step_calls=step_calls)


def render_step_context(st, step_msg, step_calls, skip_call):
    """把一个 (turn, step) 的推理/输出/同步其它调用渲染出来。"""
    out = []
    key = (st.get('turn'), st.get('step'))
    sm = step_msg.get(key)
    if sm:
        if sm['reasoning'].strip():
            out.append('**该步推理**（逐字）：\n```\n' + clip(sm['reasoning'].strip(), REASON_CAP, '推理') + '\n```')
        if sm['text'].strip():
            out.append('**该步模型输出**（逐字）：\n```\n' + clip(sm['text'].strip(), ARG_CAP, '输出文本') + '\n```')
    others = [c for c in step_calls.get(key, []) if c['order'] != skip_call]
    if others:
        lines = []
        for c in others:
            q = c['q'] if isinstance(c['q'], str) else ('共享:' + ','.join(c['q'][1]) if isinstance(c['q'], tuple) else '装置')
            lines.append(f'- `{c["name"]}` → 归属 {q}：`{clip(c["args"].replace(chr(10), " "), 300, "参数")}`')
        out.append('**同一步并行发出的其它调用**：\n' + '\n'.join(lines))
    return '\n\n'.join(out)


def render_question(arm, i, data, answers_dir):
    v = data['per'][i]
    inst_items = parse_instrument(arm, i)
    inst_log = [x for x in inst_items if x['kind'] == 'inst']
    nb_log = [x for x in inst_items if x['kind'] == 'nonbridge']
    # 会话里有定位的调用：状态机归属该题的仪器调用 + 共享步里点名该题的仪器调用
    located = sorted(data['per_inst'][i] + [c for c in data['shared_calls'] if c['inst'] and i in c['hits']],
                     key=lambda c: c['order'])
    lines = [f'# {arm} · `{i}`\n',
             f'**题面**：{QUESTION.get(i, "（见 BRIEF）")}\n',
             f'> 步 {v["steps"]} · 仪器调用 **{log_calls(arm, i)}**（逐题日志条数）· 会话里定位到 {len(located)} 条 · '
             f'输出 {v["out"]:,} tok · 推理 {v["reason"]:,} 字符 · 累计上下文 {v["cache"]:,}\n',
             '## 一、工具给了什么 · 当时 agent 在想什么（逐条配对）\n']
    if len(inst_log) != len(located):
        lines.append(f'> ⚠️ 日志 {len(inst_log)} 条 vs 会话定位 {len(located)} 条：差的那些调用发生在'
                     f'**共享步**（一条命令里跑了多道题）或**装置步**里 —— 这正是"逐题 token 不可归因"的具体形状。\n')
    for k, item in enumerate(inst_log):
        c = located[k] if k < len(located) else None
        where = (f'step {c["step"]}（turn {c["turn"]}）' if c else '会话里未定位（共享/装置步）')
        lines.append(f'### ①{k + 1} 日志第 {k + 1} 条 · exit {item["exit"]} · {where}\n')
        lines.append(f'**命令**：`{clip(item["cmd"], ARG_CAP, "命令")}`\n')
        lines.append(f'**工具返回**（逐字）：\n```\n{clip(item["out"], INSTR_CAP, "工具返回")}\n```\n')
        if c:
            ctx = render_step_context(c, data['step_msg'], data['step_calls'], c['order'])
            if ctx:
                lines.append(ctx + '\n')
    if nb_log:
        lines.append('\n### 该题的**非仪器**调用（臂自己记的，日志里带 `=== CMD(nonbridge)`）\n')
        for k, item in enumerate(nb_log, 1):
            lines.append(f'**非桥 {k}**：`{clip(item["cmd"], 400, "命令")}`\n```\n{clip(item["out"], 800, "输出")}\n```\n')

    mine = [c for c in data['per_calls'][i] if c not in data['per_inst'][i]]
    lines.append('\n## 二、该题相关的其它调用（读文件 / 编辑 / 跑测试 / 写答案），带当时的推理\n')
    seen_steps = set()
    for c in sorted(mine, key=lambda c: c['order']):
        key = (c['turn'], c['step'])
        lines.append(f'**调用** `{c["name"]}`（step {c["step"]}）：\n```\n{clip(c["args"], ARG_CAP, "参数")}\n```\n')
        lines.append(f'**结果**：\n```\n{clip(c["res"], RESULT_CAP, "结果")}\n```\n')
        if key not in seen_steps:
            seen_steps.add(key)
            ctx = render_step_context(c, data['step_msg'], data['step_calls'], -1)
            if ctx:
                lines.append(ctx + '\n')
    p = answers_dir / f'{i}.md'
    lines.append('\n## 三、交付答案（逐字）\n')
    lines.append(p.read_text(errors='replace') if p.exists() else '（没有交付答案）')
    return '\n'.join(lines)


def render_question_compact(arm, i, data, answers_dir):
    """精简版：只留分析必需的——仪器调用序列（命令 + 返回逐字 + 当时推理）+ 交付答案。"""
    v = data['per'][i]
    inst_items = [x for x in parse_instrument(arm, i) if x['kind'] == 'inst']
    located = sorted(data['per_inst'][i] + [c for c in data['shared_calls'] if c['inst'] and i in c['hits']],
                     key=lambda c: c['order'])
    L = [f'# {arm} · `{i}`（精简版；完整版见 `../{arm}/{i}.md`）\n',
         f'**题面**：{QUESTION.get(i, "（见 BRIEF）")}\n',
         f'> 仪器调用 **{log_calls(arm, i)}** · 步 {v["steps"]} · 输出 {v["out"]:,} tok · '
         f'推理 {v["reason"]:,} 字符 · 累计上下文 {v["cache"]:,}\n',
         '## 仪器调用序列（工具给了什么 + 当时 agent 在想什么）\n']
    for k, item in enumerate(inst_items):
        c = located[k] if k < len(located) else None
        where = f'step {c["step"]}（turn {c["turn"]}）' if c else '共享/装置步'
        L.append(f'### ①{k + 1} `{clip(item["cmd"], 400, "命令")}`  · exit {item["exit"]} · {where}\n')
        L.append(f'**工具返回**：\n```\n{clip(item["out"], INSTR_CAP, "工具返回")}\n```\n')
        if c:
            sm = data['step_msg'].get((c['turn'], c['step']))
            if sm and sm['reasoning'].strip():
                L.append(f'**当时推理**：\n```\n{clip(sm["reasoning"].strip(), 1500, "推理")}\n```\n')
            if sm and sm['text'].strip():
                L.append(f'**当时输出**：`{clip(sm["text"].strip(), 400, "输出")}`\n')
            others = [x for x in data['step_calls'].get((c['turn'], c['step']), []) if x['order'] != c['order']]
            if others:
                L.append('**同一步还发了**：' + '；'.join(
                    f'`{x["name"]}`→{x["q"] if isinstance(x["q"], str) else ("共享" if isinstance(x["q"], tuple) else "装置")}'
                    for x in others) + '\n')
    p = answers_dir / f'{i}.md'
    L.append('\n## 交付答案（逐字）\n')
    L.append(p.read_text(errors='replace') if p.exists() else '（没有交付答案）')
    return '\n'.join(L)


def render_bucket(title, heading, calls, msgs, step_msg, step_calls):
    lines = [f'# {title}\n', f'## {heading}\n']
    by = {}
    for r in msgs:
        by.setdefault((r['turn'], r['step']), r)
    for c in sorted(calls, key=lambda c: c['order']):
        key = (c['turn'], c['step'])
        lines.append(f'### step {c["step"]}（turn {c["turn"]}）· `{c["name"]}`\n')
        lines.append(f'```\n{clip(c["args"], ARG_CAP, "参数")}\n```\n')
        lines.append(f'**结果**：\n```\n{clip(c["res"], RESULT_CAP, "结果")}\n```\n')
        r = by.get(key)
        if r and r['reasoning'].strip():
            lines.append('**该步推理**：\n```\n' + clip(r['reasoning'].strip(), REASON_CAP, '推理') + '\n```\n')
        if r and r['text'].strip():
            lines.append('**该步输出**：\n```\n' + clip(r['text'].strip(), ARG_CAP, '输出') + '\n```\n')
    return '\n'.join(lines)


def render_briefing(arm, rows):
    out = [f'# {arm} · 任务书与往返\n']
    for n, r in enumerate(rows, 1):
        if r.get('type') == 'user/message':
            out.append(f'## 用户/队长消息（会话第 {n} 行）\n```\n{clip(str(r["data"].get("content")), 8000, "任务书")}\n```\n')
        elif r.get('type') == 'agent/inbox/spliced':
            out.append(f'## 收件箱拼接（会话第 {n} 行）\n```\n{clip(json.dumps(r.get("data"), ensure_ascii=False), 4000, "收件箱")}\n```\n')
    return '\n'.join(out)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    summary = {}
    for arm, sid, _logs, answers in ARMS:
        rows = load_session(sid)
        data = build(arm, sid)
        d_out = OUT / arm
        d_out.mkdir(parents=True, exist_ok=True)
        (d_out / '_briefing.md').write_text(render_briefing(arm, rows))
        brief = OUT / 'brief' / arm
        brief.mkdir(parents=True, exist_ok=True)
        for i in IDS:
            (d_out / f'{i}.md').write_text(render_question(arm, i, data, answers))
            (brief / f'{i}.md').write_text(render_question_compact(arm, i, data, answers))
        (d_out / '_apparatus.md').write_text(
            f'# {arm} · 装置/收尾（**不点名任何题**的步）\n\n'
            f'> 步 {data["apparatus"]["steps"]} · 调用 {data["apparatus"]["calls"]} · '
            f'输出 {data["apparatus"]["out"]:,} tok · 推理 {data["apparatus"]["reason"]:,} 字符 · '
            f'累计上下文 {data["apparatus"]["cache"]:,}\n\n'
            + render_bucket(arm + ' · 装置', '装置调用', data['app_calls'], data['app_msg'],
                            data['step_msg'], data['step_calls']))
        (d_out / '_shared.md').write_text(
            f'# {arm} · 跨题共享的步（一次调用点名多题）\n\n'
            f'> 步 {data["shared"]["steps"]} · 调用 {data["shared"]["calls"]} · '
            f'输出 {data["shared"]["out"]:,} tok · 推理 {data["shared"]["reason"]:,} 字符 · '
            f'累计上下文 {data["shared"]["cache"]:,}\n\n'
            + render_bucket(arm + ' · 共享', '共享调用', data['shared_calls'], data['shared_msg'],
                            data['step_msg'], data['step_calls']))
        summary[arm] = data
        print(f'{arm}: 26 题已渲染 → {d_out}')

    lines = ['# 逐题对话索引（两臂）\n',
             '口径（与 `tools/nichlink-per-question-cost.py` **逐字同源**，可对账）：',
             '**步**＝assistant 步；**输出**＝outputTokens 之和；**推理**＝reasoning 字符数；',
             '**累计上下文**＝该题各步 cacheRead 之和（不是净增 ✗）；',
             '**仪器**＝该题逐题日志里的调用数（`logs/<id>.jsonl` 行数 / `logs/<id>.txt` 的 `=== CMD:` 数）',
             '——这是**唯一可逐题归因**的那一栏 ✓。',
             '归属＝顺序状态机（一次调用点名**恰好一个**题号 ⇒ 该题；≥2 ⇒ 共享桶；0 ⇒ 装置桶）。\n',
             '| 题 | 我们 仪器/步/输出/推理/上下文 | codegraph 仪器/步/输出/推理/上下文 |',
             '| --- | --- | --- |']
    for i in IDS:
        f = lambda arm: (lambda v: f'{log_calls(arm, i)}/{v["steps"]}/{v["out"]:,}/{v["reason"]:,}/{v["cache"]:,}')(
            summary[arm]['per'][i])
        lines.append(f'| `{i}` | {f("ours")} | {f("cg")} |')
    for arm in ('ours', 'cg'):
        s, ap = summary[arm]['shared'], summary[arm]['apparatus']
        lines.append(f'\n**{arm}** 共享桶：步 {s["steps"]} · 调用 {s["calls"]} · 输出 {s["out"]:,} · 推理 {s["reason"]:,} · 上下文 {s["cache"]:,}  ')
        lines.append(f'**{arm}** 装置桶：步 {ap["steps"]} · 调用 {ap["calls"]} · 输出 {ap["out"]:,} · 推理 {ap["reason"]:,} · 上下文 {ap["cache"]:,}')
    lines.append('\n每题的完整渲染在 `ours/<id>.md` 与 `cg/<id>.md`；装置与共享另见 `_apparatus.md` / `_shared.md` / `_briefing.md`。')
    (OUT / 'INDEX.md').write_text('\n'.join(lines) + '\n')
    print(f'索引 → {OUT / "INDEX.md"}')


if __name__ == '__main__':
    sys.exit(main())
