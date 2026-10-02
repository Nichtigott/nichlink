#!/usr/bin/env python3
"""26-question three axes: ours (round 9) vs codegraph (round 7's 22 + T-21's 4)."""
import json, pathlib, subprocess

SESS = pathlib.Path.home() / ".dsh/sessions/--home-nich-Moirai_N3-nichlink--"
TAGS = {
    "第九轮（我们，26 道）": "agent-teams:nichlink-round9:arm-mcp",
    "第七轮（我们，22 道）": "agent-teams:nichlink-round7:arm-mcp",
    "第七轮（codegraph，22 道）": "agent-teams:nichlink-round7:arm-codegraph",
    "T-21（codegraph，四道 hardbug）": "agent-teams:nichlink-t21:arm-codegraph",
    "T-21（我们，四道 hardbug）": "agent-teams:nichlink-t21:arm-nichlink",
}
CLASSES = ["h1-supply-chain","h2-claim-unkept","h3-cross-file-chain","h4-one-file-many-algorithms"]
R7 = ["r1","r2","r3","r4","s1","s2","s3","s4","s5","s6","s7","s8","fa","fb","fc","fd","fe","g1","g2","g3","g4","h1"]

def find(tag):
    for d in SESS.iterdir():
        for f in sorted(d.glob("session*.jsonl.zstd")):
            head = subprocess.run(["zstd","-dc",str(f)],capture_output=True).stdout[:4000].decode("utf-8","replace")
            if tag in head: return d
    return None

def axis(tag, ids):
    d = find(tag)
    if d is None: return None
    text = subprocess.run(["zstd","-dc",str(sorted(d.glob("session*.jsonl.zstd"))[0])],capture_output=True).stdout
    rows = [json.loads(l) for l in text.decode("utf-8","replace").splitlines() if l.strip()]
    reason = steps = tin = tcache = tout = 0
    hit = False
    for r in rows:
        t = r.get("type")
        if t == "tool/call":
            blob = json.dumps((r.get("data") or {}).get("arguments"), ensure_ascii=False)
            hit = any(i in blob for i in ids)
        elif t == "assistant/message" and hit:
            steps += 1
            u = (r.get("data") or {}).get("usage") or {}
            parts = ((r.get("data") or {}).get("message") or {}).get("content") or []
            reason += sum(len(p.get("text","")) for p in parts if isinstance(p,dict) and p.get("type")=="reasoning")
            tin += u.get("inputTokens",0); tcache += u.get("cacheReadTokens",0); tout += u.get("outputTokens",0)
    return dict(steps=steps, reason=reason, tin=tin, tcache=tcache, tout=tout)

def whole(tag):
    d = find(tag)
    if d is None: return None
    text = subprocess.run(["zstd","-dc",str(sorted(d.glob("session*.jsonl.zstd"))[0])],capture_output=True).stdout
    rows = [json.loads(l) for l in text.decode("utf-8","replace").splitlines() if l.strip()]
    reason = steps = tin = tcache = tout = 0
    for r in rows:
        if r.get("type") != "assistant/message": continue
        steps += 1
        u = (r.get("data") or {}).get("usage") or {}
        parts = ((r.get("data") or {}).get("message") or {}).get("content") or []
        reason += sum(len(p.get("text","")) for p in parts if isinstance(p,dict) and p.get("type")=="reasoning")
        tin += u.get("inputTokens",0); tcache += u.get("cacheReadTokens",0); tout += u.get("outputTokens",0)
    return dict(steps=steps, reason=reason, tin=tin, tcache=tcache, tout=tout)

print(f"{'运行':34s} {'步数':>5s} {'推理字符':>9s} {'未命中输入':>10s} {'缓存读':>11s} {'输出':>8s}")
for tag in TAGS:
    v = whole(TAGS[tag])
    if v: print(f"{tag:34s} {v['steps']:5d} {v['reason']:9,d} {v['tin']:10,d} {v['tcache']:11,d} {v['tout']:8,d}")
print()
print("=== 四道 hardbug 类（同一批题，两边都有）===")
for tag in ("第九轮（我们，26 道）","T-21（我们，四道 hardbug）","T-21（codegraph，四道 hardbug）"):
    v = axis(TAGS[tag], CLASSES)
    if v: print(f"{tag:34s} {v['steps']:5d} {v['reason']:9,d} {v['tin']:10,d} {v['tcache']:11,d} {v['tout']:8,d}")
