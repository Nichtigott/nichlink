#!/usr/bin/env python3
"""逐题消耗（第三版）：归属规则=（a）命令文本里的一切题号线索（cd/--root/--manifest-path/CARGO_TARGET_DIR/answers/<id>/logs/<id>）✓；
（b）**不点名题树的调用 ⇒ 装置/收尾桶** ✓（读过会话确认：那是读 BRIEF/TREES、摸树、写脚本、批量 diff/哈希核对、写报告 ✓
——它们可归因到"装置"，**不可归因到题** ✗）。原规则：
`cd …/trees/<id>` · `--root` · `--manifest-path` · `CARGO_TARGET_DIR=…/<id>` ·
答案文件 `answers/<id>.md` · 日志 `<id>.jsonl|.txt` · 任意 `trees/<id>` ✓。
输入侧按规则摊：**每一步的 cacheRead 记在"那一步所属的题"上**（整轮求和不变 ✓）。
多题同现 ⇒ 共享桶 ✗；仍读不出的 ⇒ 我在会话里逐条读（见脚本末的自报口径 ✓）。"""
import json, pathlib, subprocess, collections, re
S = pathlib.Path.home()/'.dsh/sessions/--home-nich-Moirai_N3-nichlink--'
IDS = ['r1','r2','r3','r4','s1','s2','s3','s4','s5','s6','s7','s8','g1','g2','g3','g4','h1',
       'fa','fb','fc','fd','fe','h1-supply-chain','h2-claim-unkept','h3-cross-file-chain','h4-one-file-many-algorithms']
IDS = sorted(IDS, key=len, reverse=True)          # 长名优先，避免 h1 抢先匹配 h1-supply-chain
OURS = 'aa54ec58-44bf-4f88-a2ed-d0dd11614b93'
def session_of(tag):
    for d in S.iterdir():
        for f in sorted(d.glob('session*.jsonl.zstd')):
            if tag in subprocess.run(['zstd','-dc',str(f)],capture_output=True).stdout[:8000].decode('utf-8','replace'):
                return d.name
    return None
CG = session_of('nichlink-cg26:cg26')
def hits(blob):
    """命令文本里出现的题号（去重、保序）。

    **2026-10-03 修正**：原版用子串 `trees/<id>`，于是 `trees/h1` 命中了 `trees/h1-supply-chain`
    ⇒ 那道题的每一次调用都被同时算作 `h1`（两个题号）⇒ 双双落进"共享桶"，`h1-supply-chain`
    的逐题读数（4 步 / 3,194 输出 / 8,521 推理 / 1,112,832 上下文）在旧表里凭空消失。
    这里改成**带右边界的匹配**（题号后不得紧跟 `-`/字母/数字/下划线）。
    """
    found=[]
    for i in IDS:
        pat = rf'(trees|answers|logs|CARGO_TARGET_DIR)[^"\\s]*[/=]{re.escape(i)}(?![-A-Za-z0-9_])'
        cg_arg = re.search(rf'cg\.sh\s+{re.escape(i)}(?![-A-Za-z0-9_])', blob)
        if re.search(pat, blob) or cg_arg or f'answers/{i}.' in blob or f'logs/{i}.' in blob:
            found.append(i)
    return found
def per_question(sid, marker):
    d=S/sid
    rows=[json.loads(l) for l in subprocess.run(['zstd','-dc',str(sorted(d.glob('session*.jsonl.zstd'))[0])],capture_output=True).stdout.decode('utf-8','replace').splitlines() if l.strip()]
    per=collections.defaultdict(lambda: dict(steps=0,out=0,reason=0,cache=0,inst=0,other=0))
    shared=dict(steps=0,out=0,reason=0,cache=0); other=dict(steps=0,out=0,reason=0,cache=0)
    cur=None; assigned=collections.Counter()
    for r in rows:
        t=r.get('type'); dd=r.get('data') or {}
        if t=='tool/call':
            blob=json.dumps(dd.get('arguments'),ensure_ascii=False)
            hit=hits(blob)
            if len(hit)==1:
                cur=hit[0]; assigned['mech']+=1
                if marker in blob: per[cur]['inst']+=1
                else: per[cur]['other']+=1
            elif len(hit)>1: cur=('shared',tuple(hit)); assigned['shared']+=1
            else: cur=None; assigned['apparatus']+=1
        elif t=='assistant/message':
            u=dd.get('usage') or {}
            if u.get('cacheReadTokens') is None: continue
            parts=(dd.get('message') or {}).get('content') or []
            rs=sum(len(p.get('text','')) for p in parts if isinstance(p,dict) and p.get('type')=='reasoning')
            o=u.get('outputTokens',0); c=u.get('cacheReadTokens',0)
            if isinstance(cur,str): v=per[cur]; v['steps']+=1; v['out']+=o; v['reason']+=rs; v['cache']+=c
            elif cur: shared['steps']+=1; shared['out']+=o; shared['reason']+=rs; shared['cache']+=c
            else: other['steps']+=1; other['out']+=o; other['reason']+=rs; other['cache']+=c
    return per, shared, other, assigned
ours,s1,o1,a1 = per_question(OURS,'nichlink-mcp')
cg,s2,o2,a2 = per_question(CG,'codegraph') if CG else ({},{},{},{})
def logcalls(p):
    f=pathlib.Path(p)
    if not f.exists(): return 0
    t=f.read_text(errors='replace')
    return t.count('=== CMD:') if f.suffix=='.txt' else len([l for l in t.splitlines() if l.strip()])
print(f'{"题":26s}{"我们 调用/步/输出/推理/累计上下文":>40s}{"codegraph 调用/步/输出/推理/累计上下文":>44s}{"调用比":>8s}')
T=[dict(ic=0,st=0,out=0,rs=0,ca=0),dict(ic=0,st=0,out=0,rs=0,ca=0)]
for i in IDS:
    a=ours.get(i,{}); b=cg.get(i,{})
    ac=logcalls(f'target/round9/logs/{i}.jsonl'); bc=logcalls(f'target/probe-cg26/logs/{i}.txt')
    for T_,v,ic in ((T[0],a,ac),(T[1],b,bc)):
        T_['ic']+=ic; T_['st']+=v.get('steps',0); T_['out']+=v.get('out',0); T_['rs']+=v.get('reason',0); T_['ca']+=v.get('cache',0)
    print(f'{i:26s}{ac:>7d}/{a.get("steps",0):>4d}/{a.get("out",0):>8,d}/{a.get("reason",0):>7,d}/{a.get("cache",0):>11,d}'
          f'{bc:>9d}/{b.get("steps",0):>5d}/{b.get("out",0):>8,d}/{b.get("reason",0):>7,d}/{b.get("cache",0):>11,d}'
          f'{(ac/bc if bc else 0):>7.2f}×')
print(f'{"—— 可归因合计":26s}{T[0]["ic"]:>7d}/{T[0]["st"]:>4d}/{T[0]["out"]:>8,d}/{T[0]["rs"]:>7,d}/{T[0]["ca"]:>11,d}'
      f'{T[1]["ic"]:>9d}/{T[1]["st"]:>5d}/{T[1]["out"]:>8,d}/{T[1]["rs"]:>7,d}/{T[1]["ca"]:>11,d}{T[0]["ic"]/T[1]["ic"]:>7.2f}×')
print(f'\n共享桶：我们 {s1["steps"]} 步/{s1["out"]:,}/{s1["reason"]:,}/{s1["cache"]:,} · codegraph {s2.get("steps",0)} 步/{s2.get("out",0):,}/{s2.get("reason",0):,}/{s2.get("cache",0):,}')
print(f'收尾桶：我们 {o1["steps"]} 步 · codegraph {o2.get("steps",0)} 步')
print(f'\n归因次数：我们 {dict(a1)} · codegraph {dict(a2)}')
print('口径：输入侧＝把每一步的 cacheRead 记在"那一步所属的题"上（整轮求和不变 ✓，不是"该题净增的上下文" ✗）。')
