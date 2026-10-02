#!/usr/bin/env python3
"""逐题并排表：值全部从**会话原始日志**与各题日志量，不采信自报。"""
import json, pathlib, subprocess
S = pathlib.Path.home()/'.dsh/sessions/--home-nich-Moirai_N3-nichlink--'
# 8 条单题探针（会话 id 来自各自报告，逐条可核）
ARMS = [
 ('h1-supply-chain','我们',      'db5b1e38-97b9-4a5e-ba1a-c8bc3c9386b1','target/probe-perq/logs/h1-nichlink.jsonl'),
 ('h1-supply-chain','codegraph', '2a3f783d-1bfa-4855-a302-c6ed0d85290e','target/probe-perq/logs/h1-codegraph.txt'),
 ('h2-claim-unkept','我们',      '77a3007a-2d86-4cd8-91e7-0200511ceab0','target/probe-perq/logs/h2-nichlink.jsonl'),
 ('h2-claim-unkept','codegraph', 'b52b67e3-dab9-47a9-8d1a-821defdfac04','target/probe-perq/logs/h2-codegraph.txt'),
 ('h3-cross-file-chain','我们',  'b9b0c605-c101-43c3-b12e-9e6925fcc03a','target/probe-perq/logs/h3-nichlink.jsonl'),
 ('h3-cross-file-chain','codegraph','aef0c999-1dc6-4b49-b59d-11faf5ca58e4','target/probe-perq/logs/h3-codegraph.txt'),
 ('h4-one-file-many-algorithms','我们','272341de-c92d-4c59-9351-17104e7999c4','target/probe-perq/logs/h4-nichlink.jsonl'),
 ('h4-one-file-many-algorithms','codegraph','39f47a9b-57ca-44a7-bd3a-8bd360218b4a','target/probe-perq/logs/h4-codegraph.txt'),
]
def axes(sid):
    d = S/sid
    if not d.exists(): return None
    text = subprocess.run(['zstd','-dc',str(sorted(d.glob('session*.jsonl.zstd'))[0])],capture_output=True).stdout
    rows=[json.loads(l) for l in text.decode('utf-8','replace').splitlines() if l.strip()]
    steps=tout=reason=tc=0; last=0
    for r in rows:
        if r.get('type')!='assistant/message': continue
        u=(r.get('data') or {}).get('usage') or {}
        if u.get('cacheReadTokens') is None: continue
        steps+=1; tout+=u.get('outputTokens',0); tc+=u.get('cacheReadTokens',0); last=u.get('cacheReadTokens',0)
        parts=((r.get('data') or {}).get('message') or {}).get('content') or []
        reason+=sum(len(p.get('text','')) for p in parts if isinstance(p,dict) and p.get('type')=='reasoning')
    return dict(steps=steps,out=tout,reason=reason,peak=last,per_step=tc//steps if steps else 0)
def calls(p):
    f=pathlib.Path(p)
    if not f.exists(): return 0
    txt=f.read_text(errors='replace')
    return txt.count('=== CMD:') if f.suffix=='.txt' else len([l for l in txt.splitlines() if l.strip()])
print(f"{'题':26s}{'工具':11s}{'步':>4s}{'输出tok':>9s}{'推理字符':>9s}{'上下文峰':>10s}{'步均':>8s}{'调用':>5s}")
for cls, who, sid, log in ARMS:
    a=axes(sid)
    if not a: print(f'{cls:26s}{who:11s}   （会话不在）'); continue
    print(f'{cls:26s}{who:11s}{a["steps"]:4d}{a["out"]:9,d}{a["reason"]:9,d}{a["peak"]:10,d}{a["per_step"]:8,d}{calls(log):5d}')

# ============ cg26 那一侧（团队跑完后可用）============
def cg26_axes(sid=None):
    """cg26 的会话：按标签找 agent-teams:nichlink-cg26:cg26；量步/输出/推理/步均。"""
    for d in S.iterdir():
        for f in sorted(d.glob('session*.jsonl.zstd')):
            head = subprocess.run(['zstd','-dc',str(f)],capture_output=True).stdout[:8000].decode('utf-8','replace')
            if 'nichlink-cg26:cg26' in head:
                return d.name, axes(d)
    return None, None

def main_cg26():
    sid, a = cg26_axes()
    if not a:
        print('\n[cg26] 还没跑完或会话未出现 —— 完工后再跑本脚本'); return
    print(f'\n[cg26] 会话 {sid}：步 {a["steps"]} · 输出 {a["out"]:,} · 推理 {a["reason"]:,} · 步均 {a["per_step"]:,} · 结束上下文 {a["peak"]:,}')
    # 与我们对齐：我们第九轮的 26 题一会话
    d = S/'aa54ec58-44bf-4f88-a2ed-d0dd11614b93'
    if d.exists():
        o = axes(d)
        print(f'[我们] 会话 aa54ec58：步 {o["steps"]} · 输出 {o["out"]:,} · 推理 {o["reason"]:,} · 步均 {o["per_step"]:,} · 结束上下文 {o["peak"]:,}')
        print(f'\n比值（我们 ÷ cg26）：步/题 {o["steps"]/26/(a["steps"]/26):.2f}× · 输出/题 {o["out"]/(a["out"]):.2f}× · '
              f'推理/题 {o["reason"]/a["reason"]:.2f}× · 步均上下文 {o["per_step"]/a["per_step"]:.2f}×')
main_cg26()
