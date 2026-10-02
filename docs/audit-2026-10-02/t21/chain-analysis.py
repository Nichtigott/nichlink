#!/usr/bin/env python3
"""Align each arm's reasoning blocks with the tool call that preceded them (T-21)."""
import json, pathlib, subprocess, sys
SESSIONS = pathlib.Path.home() / ".dsh/sessions/--home-nich-Moirai_N3-nichlink--"
ARMS = {"arm-nichlink": "4fc3fd66-00c3-4cc5-b4f9-3a94b70e3b59",
        "arm-codegraph": "079dd44f-2207-4fbe-b87f-e173d2bc64f1"}
QUESTIONS = ["h1-supply-chain", "h2-claim-unkept", "h3-cross-file-chain",
             "h4-one-file-many-algorithms", "ripgrep-de2567a"]

def records(arm):
    path = sorted((SESSIONS / ARMS[arm]).glob("session*.jsonl.zstd"))[0]
    text = subprocess.run(["zstd", "-dc", str(path)], capture_output=True).stdout
    return [json.loads(l) for l in text.decode("utf-8", "replace").splitlines() if l.strip()]

def reasoning_of(r):
    # The reasoning lives under `data.message.content` — `data.content` does not exist, and reading it
    # silently produced 0 characters for every step (measured here: 0 for both arms on the first run).
    # 推理在 `data.message.content` 下——`data.content` 并不存在，读它会让每一步都静默地算出 0 字符
    # （本次实测：第一版对两臂都报 0）。
    content = ((r.get("data") or {}).get("message") or {}).get("content")
    if not isinstance(content, list): return ""
    return "".join(p.get("text", "") for p in content if p.get("type") == "reasoning")

def called(r):
    d = r.get("data") or {}
    name = d.get("name") or d.get("tool") or ""
    args = d.get("arguments") or d.get("argv") or {}
    if isinstance(args, dict): argv = " ".join(str(v) for v in args.values())
    elif isinstance(args, list): argv = " ".join(str(v) for v in args)
    else: argv = str(args)
    return name, argv

def question_of(argv):
    for k in QUESTIONS:
        if k in argv: return k
    if "entry.rs" in argv or "model" in argv: return "h4-one-file-many-algorithms"
    if "printer" in argv or "util.rs" in argv: return "ripgrep-de2567a"
    return None

def walk(arm):
    per = {k: {"steps": 0, "chars": 0, "calls": 0, "blocks": [], "out": 0, "inp": 0, "cache": 0} for k in QUESTIONS}
    current, pending, pending_result = None, None, 0
    for r in records(arm):
        kind = r.get("type")
        if kind == "tool/call":
            name, argv = called(r)
            where = question_of(argv)
            if where:
                current = where; per[where]["calls"] += 1
            pending = (name, argv[:110]); pending_result = 0
        elif kind == "tool/result":
            d = r.get("data") or {}
            text = d.get("content") or d.get("text") or ""
            if isinstance(text, list):
                text = "".join(p.get("text", "") for p in text if isinstance(p, dict))
            pending_result = len(str(text))
        elif kind == "assistant/message":
            text = reasoning_of(r)
            if current is None or not text: continue
            per[current]["steps"] += 1; per[current]["chars"] += len(text)
            usage = (r.get("data") or {}).get("usage") or {}
            per[current]["out"] += usage.get("outputTokens", 0)
            per[current]["inp"] += usage.get("inputTokens", 0)
            per[current]["cache"] += usage.get("cacheReadTokens", 0)
            per[current]["blocks"].append({"chars": len(text), "after": pending,
                                           "result_chars": pending_result, "text": text})
    return per

arms = {a: walk(a) for a in ARMS}
print(f"{'question':30s} {'arm':14s} {'steps':>6s} {'reason chars':>13s} {'calls':>6s}")
for k in QUESTIONS:
    for a in ARMS:
        row = arms[a][k]
        print(f"{k:30s} {a:14s} {row['steps']:6d} {row['chars']:13,d} {row['calls']:6d}")
    o, t = arms["arm-nichlink"][k], arms["arm-codegraph"][k]
    if t["chars"]: print(f"{'':30s} {'比值 我们/它':14s} {'':6s} {o['chars']/t['chars']:13.2f}")
    print()
# The three columns the earlier round settled on: characters **and** blocks **and** thinking per
# instrument call. Characters alone say "our chain is short"; blocks say whether we took fewer steps
# or shorter ones, and those two readings point at opposite optimizations.
# 上一轮定下的三栏：字符**与**块数**与**每仪器调用的思考量。只看字符会说"我们的链更短"；块数才说清是
# **步数更少**还是**每步更短**——这两种读法指向相反的优化方向。
print(f"{'question':30s} {'arm':14s} {'blocks':>7s} {'chars':>9s} {'calls':>6s} {'chars/call':>11s} {'long blocks':>12s}")
for k in QUESTIONS:
    for a in ARMS:
        r = arms[a][k]
        longish = [b for b in r["blocks"] if b["chars"] >= 5000]
        share = sum(b["chars"] for b in longish) * 100 // max(r["chars"], 1)
        print(f"{k:30s} {a:14s} {len(r['blocks']):7d} {r['chars']:9,d} {r['calls']:6d} "
              f"{r['chars']//max(r['calls'],1):11,d} {len(longish):5d} ({share:3d}%)")
print()
print(f"{'question':30s} {'arm':14s} {'output':>9s} {'input':>9s} {'cacheRead':>11s}")
for k in QUESTIONS:
    for a in ARMS:
        r = arms[a][k]
        print(f"{k:30s} {a:14s} {r['out']:9,d} {r['inp']:9,d} {r['cache']:11,d}")
for a in ARMS:
    real = [arms[a][k] for k in QUESTIONS[:4]]
    print(f"{'仅四道真题':30s} {a:14s} {sum(r['out'] for r in real):9,d} {sum(r['inp'] for r in real):9,d} {sum(r['cache'] for r in real):11,d}")
    print(f"{'合计':30s} {a:14s} {sum(arms[a][k]['out'] for k in QUESTIONS):9,d} "
          f"{sum(arms[a][k]['inp'] for k in QUESTIONS):9,d} {sum(arms[a][k]['cache'] for k in QUESTIONS):11,d}")
print()

if len(sys.argv) > 1 and sys.argv[1] == "blocks":
    arm = sys.argv[2] if len(sys.argv) > 2 else "arm-nichlink"
    n = int(sys.argv[3]) if len(sys.argv) > 3 else 10
    rows = sorted(((b["chars"], k, i, b) for k in QUESTIONS
                   for i, b in enumerate(arms[arm][k]["blocks"])), reverse=True)
    for chars, k, i, b in rows[:n]:
        name, argv = b["after"] or ("?", "")
        print(f"=== {chars:,d} 字符 · {k} · 第 {i} 块 · 前一次调用 `{name}` [{argv}] · 那次结果 {b['result_chars']:,d} 字符 ===")
        print(b["text"][:1100].replace("\n", " ⏎ "))
        print()
