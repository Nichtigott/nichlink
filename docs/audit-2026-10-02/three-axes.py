#!/usr/bin/env python3
"""Three axes for the three runs: token (3 cols) · chain (3 cols) · effect slice."""
import json, pathlib, subprocess, re

SESS = pathlib.Path.home() / ".dsh/sessions/--home-nich-Moirai_N3-nichlink--"
RUNS = {
    "第九轮（我们，当前桥）": "agent-teams:nichlink-round9:arm-mcp",
    "第七轮（我们）": "agent-teams:nichlink-round7:arm-mcp",
    "第七轮（codegraph）": "agent-teams:nichlink-round7:arm-codegraph",
}
READONLY = ["s1","s2","s3","s4","s5","s6","s7","s8","g1","g2","g4"]

def session_of(label):
    for d in SESS.iterdir():
        for f in sorted(d.glob("session*.jsonl.zstd")):
            head = subprocess.run(["zstd","-dc",str(f)],capture_output=True).stdout[:4000].decode("utf-8","replace")
            if label in head:
                return d
    return None

def rows_of(d):
    text = subprocess.run(["zstd","-dc",str(sorted(d.glob("session*.jsonl.zstd"))[0])],capture_output=True).stdout
    return [json.loads(l) for l in text.decode("utf-8","replace").splitlines() if l.strip()]

def question_of(blob):
    for i in READONLY + ["r1","r2","r3","r4","fa","fb","fc","fd","fe","g3","h1",
                          "h1-supply-chain","h2-claim-unkept","h3-cross-file-chain","h4-one-file-many-algorithms"]:
        if f"trees/{i}" in blob or f"round7/{i}" in blob:
            return i
    return None

print(f"{'运行':26s} {'步数':>5s} {'推理字符':>9s} {'思考块':>6s} {'每仪器调用思考':>12s} {'未命中输入':>10s} {'缓存读':>11s} {'输出':>8s}")
for label, tag in RUNS.items():
    d = session_of(tag)
    if d is None:
        print(f"{label:26s} （找不到会话）"); continue
    reason = blocks = steps = 0
    tin = tcache = tout = 0
    tool_calls = instrument = 0
    slice_calls = 0; cur = None
    slice_in = slice_cache = slice_out = slice_reason = 0
    for r in rows_of(d):
        t = r.get("type")
        if t == "tool/call":
            tool_calls += 1
            blob = json.dumps((r.get("data") or {}).get("arguments"), ensure_ascii=False)
            if "nichlink-mcp" in blob or "codegraph" in blob:
                instrument += 1
            q = question_of(blob)
            if q in READONLY:
                cur = q
                slice_calls += 1
            elif q is not None:
                cur = None
        elif t == "assistant/message":
            steps += 1
            u = (r.get("data") or {}).get("usage") or {}
            parts = ((r.get("data") or {}).get("message") or {}).get("content") or []
            chars = 0
            for p in parts:
                if isinstance(p, dict) and p.get("type") == "reasoning":
                    chars += len(p.get("text","")); blocks += 1
            reason += chars
            tin += u.get("inputTokens",0); tcache += u.get("cacheReadTokens",0); tout += u.get("outputTokens",0)
            if cur: slice_in += u.get("inputTokens",0); slice_cache += u.get("cacheReadTokens",0)
            slice_out += 0 if not cur else u.get("outputTokens",0); slice_reason += chars if cur else 0
    per = reason / instrument if instrument else 0
    print(f"{label:26s} {steps:5d} {reason:9,d} {blocks:6d} {per:12,.0f} {tin:10,d} {tcache:11,d} {tout:8,d}")
    print(f"{'  └ 可比切面（11 道只读题）':26s} {'':5s} {slice_reason:9,d} {'':6s} {'':12s} {slice_in:10,d} {slice_cache:11,d} {slice_out:8,d}  切面内桥调用 {slice_calls}")
