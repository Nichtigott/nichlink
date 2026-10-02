#!/usr/bin/env python3
"""逐题消耗：只归因"调用只点名这一棵树"的那些步（输出/推理可归因 ✓）；
输入侧（缓存读）逐题不可归因 ⇒ 单列末尾的累计桶 ✗。"""
import json, pathlib, subprocess, collections, sys
S = pathlib.Path.home()/'.dsh/sessions/--home-nich-Moirai_N3-nichlink--'
IDS = ['r1','r2','r3','r4','s1','s2','s3','s4','s5','s6','s7','s8','g1','g2','g3','g4','h1',
       'fa','fb','fc','fd','fe','h1-supply-chain','h2-claim-unkept','h3-cross-file-chain','h4-one-file-many-algorithms']
OURS = 'aa54ec58-44bf-4f88-a2ed-d0dd11614b93'
def session_of(tag):
    for d in S.iterdir():
        for f in sorted(d.glob('session*.jsonl.zstd')):
            if tag in subprocess.run(['zstd','-dc',str(f)],capture_output=True).stdout[:8000].decode('utf-8','replace'):
                return d.name
    return None
CG = session_of('nichlink-cg26:cg26')
def per_question(sid, marker):
    """marker=该臂的仪器名，用于区分仪器调用与其它调用。"""
    d = S/sid
    text = subprocess.run(['zstd','-dc',str(sorted(d.glob('session*.jsonl.zstd'))[0])],capture_output=True).stdout
    rows=[json.loads(l) for l in text.decode('utf-8','replace').splitlines() if l.strip()]
    per=collections.defaultdict(lambda: dict(steps=0,out=0,reason=0,inst=0,other=0,peak=0))
    shared=dict(steps=0,out=0,reason=0); other=dict(steps=0,out=0,reason=0)
    cur=None
    for r in rows:
        t=r.get('type'); dd=r.get('data') or {}
        if t=='tool/call':
            blob=json.dumps(dd.get('arguments'),ensure_ascii=False)
            hit=[i for i in IDS if f'trees/{i}' in blob or f'trees/{i}/' in blob]
            if len(hit)==1:
                cur=hit[0]
                if marker in blob: per[cur]['inst']+=1
                else: per[cur]['other']+=1
            elif len(hit)>1: cur=('shared',)
            # 不点名题树的调用：cur 保持不变（写答案/收尾）
        elif t=='assistant/message':
            u=dd.get('usage') or {}
            if u.get('cacheReadTokens') is None: continue
            parts=(dd.get('message') or {}).get('content') or []
            rs=sum(len(p.get('text','')) for p in parts if isinstance(p,dict) and p.get('type')=='reasoning')
            o=u.get('outputTokens',0); c=u.get('cacheReadTokens',0)
            if isinstance(cur,str):
                v=per[cur]; v['steps']+=1; v['out']+=o; v['reason']+=rs; v['peak']=c
            elif cur:
                shared['steps']+=1; shared['out']+=o; shared['reason']+=rs
            else:
                other['steps']+=1; other['out']+=o; other['reason']+=rs
    return per, shared, other
ours, s1, o1 = per_question(OURS, 'nichlink-mcp')
cg, s2, o2 = per_question(CG, 'codegraph') if CG else ({}, {}, {})
def logcalls(p):
    f=pathlib.Path(p)
    if not f.exists(): return 0
    t=f.read_text(errors='replace')
    return t.count('=== CMD:') if f.suffix=='.txt' else len([l for l in t.splitlines() if l.strip()])
print(f'{"题":26s}{"我们 调用/步/输出/推理":>28s}{"codegraph 调用/步/输出/推理":>32s}{"调用比":>8s}')
T=[dict(ic=0,st=0,out=0,rs=0), dict(ic=0,st=0,out=0,rs=0)]
for i in IDS:
    a=ours.get(i,{}); b=cg.get(i,{})
    ac=logcalls(f'target/round9/logs/{i}.jsonl'); bc=logcalls(f'target/probe-cg26/logs/{i}.txt')
    T[0]['ic']+=ac; T[0]['st']+=a.get('steps',0); T[0]['out']+=a.get('out',0); T[0]['rs']+=a.get('reason',0)
    T[1]['ic']+=bc; T[1]['st']+=b.get('steps',0); T[1]['out']+=b.get('out',0); T[1]['rs']+=b.get('reason',0)
    ratio=f'{ac/bc:.2f}×' if bc else '—'
    print(f'{i:26s}{ac:>8d}/{a.get("steps",0):>4d}/{a.get("out",0):>8,d}/{a.get("reason",0):>7,d}'
          f'{bc:>10d}/{b.get("steps",0):>6d}/{b.get("out",0):>9,d}/{b.get("reason",0):>8,d}{ratio:>8s}')
print(f'{"—— 可归因合计":26s}{T[0]["ic"]:>8d}/{T[0]["st"]:>4d}/{T[0]["out"]:>8,d}/{T[0]["rs"]:>7,d}'
      f'{T[1]["ic"]:>10d}/{T[1]["st"]:>6d}/{T[1]["out"]:>9,d}/{T[1]["rs"]:>8,d}{T[0]["ic"]/T[1]["ic"]:>7.2f}×')
print(f'\n不可归因（跨题共享步）：我们 {s1["steps"]} 步/{s1["out"]:,} 输出/{s1["reason"]:,} 推理 · codegraph {s2.get("steps",0)} 步/{s2.get("out",0):,} 输出/{s2.get("reason",0):,} 推理')
print(f'未归因（写答案/收尾）：我们 {o1["steps"]} 步/{o1["out"]:,} 输出 · codegraph {o2.get("steps",0)} 步/{o2.get("out",0):,} 输出')
print(f'\n⚠️ 输入侧（缓存读/未命中输入）逐题不可归因 ✗：每一步都携带全部前文 ⇒ 整轮 36,192,640 vs 38,516,224 (0.940×)')
