#!/usr/bin/env python3
"""Score one round's two answers mechanically, and tabulate a whole run.
机械地给一轮的两份答案打分，并把整次运行汇总成表。

No model judges anything here. Four things are computed:

* **hit** — does the answer text name the injected file and the injected symbol?
  Both spellings of a file are accepted (copy-relative and member-relative),
  because both are the tool's own spelling of the same path, and the criterion is
  "did it point at the file", not "did it spell it my way".
* **precision / false positives** — the callers each side reported that the
  reference edge set does not have. A reported caller is only called a false
  positive when the callee's name does not appear anywhere in the reported
  caller's file: the name match is re-checked against the text, so a name collision
  is recorded as *unverifiable* rather than as a false positive.
* **link fidelity** — the one hop the injection changed. `impl` leaves the call in
  place, `chain` and `both` remove it; the answer's own caller list for that hop's
  callee is compared with the constructed truth.
* **layer vocabulary** — the answer's words, not its judgement. Recorded, and
  explicitly not treated as a verdict, because a keyword count is not a reader.

这里没有任何模型在判分。算四件事：

* **命中**——答案正文有没有点名被注入的文件与符号？文件接受两种拼法（副本相对与成员相对），
  因为两者都是同一个路径在工具自己嘴里的样子，判据是"有没有指向那个文件"，不是"有没有按我的写法拼"。
* **精确率/误报**——两边报出的、参考边集里没有的调用者。只有当被调者的名字**在任何地方都不出现**
  在被报调用者的文件里时，才把它叫作误报：名字匹配要回到正文里复核，因此同名撞车记为
  **无法核实**，而不是误报。
* **链路保真**——注入改动的那一跳。`impl` 把调用留着，`chain` 与 `both` 把它摘掉；拿答案自己对那一跳
  被调者的调用者名单与构造出的真值比。
* **层次词汇**——数答案里的词，不是读它的判断。记录在案，并明确**不**当作判词，因为词频不是读者。
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import re
import sqlite3
import statistics

CLAIM_MARKERS = (
    "病灶",
    "根因",
    "问题在",
    "故障点",
    "culprit",
    "root cause",
    "the bug is",
)
LAYER_WORDS = {
    "impl": ("实现", "函数体", "impl", "body", "逻辑"),
    "chain": ("调用链", "链路", "调用点", "漏掉", "chain", "link", "call site"),
}


def reference_callers(db: str) -> dict:
    """Every caller edge in the pristine index, keyed `file::name`.
    pristine 索引里的每条调用者边，按 `file::name` 索引。"""
    connection = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    nodes = {}
    for node_id, name, path in connection.execute(
        "SELECT id, name, file_path FROM nodes WHERE language = 'rust'"
    ):
        nodes[node_id] = {"name": name, "file": path.replace(os.sep, "/")}
    callers: dict[str, list[dict]] = {}
    for source, target in connection.execute("SELECT source, target FROM edges WHERE kind = 'calls'"):
        callee = nodes.get(target)
        caller = nodes.get(source)
        if callee is None or caller is None:
            continue
        callers.setdefault(f"{callee['file']}::{callee['name']}", []).append(caller)
    connection.close()
    return callers


def contains_path(answer: str, file: str, member: str) -> bool:
    """Whether the answer spells the file, copy-relative or member-relative.
    答案有没有写出这个文件，副本相对或成员相对都算。"""
    if file in answer:
        return True
    relative = file[len(member) + 1 :] if file.startswith(member + "/") else file
    return bool(re.search(re.escape(relative) + r"(?![A-Za-z0-9_])", answer))


def file_text(copy: str, file: str) -> str:
    path = os.path.join(copy, file)
    if not os.path.isfile(path):
        return ""
    with open(path, encoding="utf-8", errors="replace") as handle:
        return handle.read()


def judge_one(truth: dict, out_dir: str, db: str, copy: str, members: list[str]) -> dict:
    site = truth["site"]
    member = site["member"]
    hop = truth["hop"]
    reference = reference_callers(db)
    hop_callee = next(
        (node for node in truth["chain"] if node["id"] == hop["callee"]), None
    )
    hop_caller = next(
        (node for node in truth["chain"] if node["id"] == hop["caller"]), None
    )
    verdict = {
        "round": truth["round"],
        "seed": truth["seed"],
        "depth": truth["depth"],
        "layer": truth["layer"],
        "site": {"file": site["file"], "name": site["name"], "line": truth["mutated_lines"].get(site["file"])},
        "observable": {"file": truth["observable"]["file"], "name": truth["observable"]["name"]},
        "hop": {
            "caller": {"file": hop_caller["file"], "name": hop_caller["name"]},
            "callee": {"file": hop_callee["file"], "name": hop_callee["name"]},
        },
        "tools": {},
    }
    for tool in ("mcp", "codegraph"):
        answer_path = os.path.join(out_dir, f"{tool}.answer.txt")
        result_path = os.path.join(out_dir, f"{tool}.result.json")
        if not os.path.isfile(answer_path):
            verdict["tools"][tool] = {"present": False}
            continue
        with open(answer_path, encoding="utf-8") as handle:
            answer = handle.read()
        with open(result_path, encoding="utf-8") as handle:
            result = json.load(handle)
        named_file = contains_path(answer, site["file"], member)
        named_symbol = bool(re.search(r"\b" + re.escape(site["name"]) + r"\b", answer))
        break_named_file = contains_path(answer, hop_callee["file"], hop_callee["member"])
        break_named_symbol = bool(
            re.search(r"\b" + re.escape(hop_callee["name"]) + r"\b", answer)
        )
        named_together = bool(
            re.search(
                r"(?:"
                + re.escape(site["file"])
                + r"|"
                + re.escape(site["file"][len(member) + 1 :])
                + r")[^\n]{0,160}?\b"
                + re.escape(site["name"])
                + r"\b",
                answer,
            )
        ) or bool(
            re.search(
                r"\b"
                + re.escape(site["name"])
                + r"\b[^\n]{0,160}?(?:"
                + re.escape(site["file"])
                + r"|"
                + re.escape(site["file"][len(member) + 1 :])
                + r")",
                answer,
            )
        )
        # Precision against the reference edge set, with the construction applied.
        true_positive = 0
        missed = 0
        false_positive = []
        unverifiable = []
        for key, reported in result["queries"].items():
            file, _, name = key.rpartition("::")
            expected = list(reference.get(key, []))
            if hop_callee and file == hop_callee["file"] and name == hop_callee["name"]:
                if truth["layer"] in ("chain", "both"):
                    expected = [
                        caller
                        for caller in expected
                        if caller["file"] != hop_caller["file"]
                        or caller["name"] != hop_caller["name"]
                    ]
            expected_keys = {(caller["file"], caller["name"]) for caller in expected}
            reported_keys = {(caller["file"], caller["name"]) for caller in reported}
            true_positive += len(expected_keys & reported_keys)
            missing = expected_keys - reported_keys
            missed += len(missing)
            for candidate in sorted(reported_keys - expected_keys):
                if (
                    hop_callee is not None
                    and file == hop_callee["file"]
                    and name == hop_callee["name"]
                    and candidate == (hop_caller["file"], hop_caller["name"])
                    and truth["layer"] in ("chain", "both")
                ):
                    # The construction removed this very call, so the claim is
                    # contradicted by the tree we made: a definite false positive,
                    # no name grep needed.
                    # 构造删掉的正是这一次调用，因此这个声明被我们自己造的树否定：确定的误报，
                    # 不需要再做名字复核。
                    false_positive.append(
                        {"caller": list(candidate), "callee": key, "why": "removed hop"}
                    )
                    continue
                text = file_text(copy, candidate[0])
                if re.search(r"\b" + re.escape(name) + r"\b", text):
                    unverifiable.append({"caller": list(candidate), "callee": key})
                else:
                    false_positive.append({"caller": list(candidate), "callee": key})
        # Link fidelity on the one changed hop.
        link = "unreached"
        link_ok = None
        break_detected = None
        hop_key = f"{hop_callee['file']}::{hop_callee['name']}"
        if hop_key in result["queries"]:
            reported_keys = {
                (caller["file"], caller["name"])
                for caller in result["queries"][hop_key]
            }
            present = (hop_caller["file"], hop_caller["name"]) in reported_keys
            link = "present" if present else "absent"
            link_ok = present if truth["layer"] == "impl" else not present
            # What a static reader can see of a deleted call is that the edge is
            # gone, not the call site that carried it: the site is above the cut, so
            # no upward walk from the observable can reach it, and the deleted call
            # leaves no mention behind. "Detected" therefore means "the tool reached
            # the hop's callee and did not put the removed edge back", and the
            # question that separates a real answer from a silent one is whether the
            # tool resolved edges elsewhere in the same walk (the recall columns).
            # 静态读者能从被删的调用里看到的，是那条边没了，而不是承载它的调用点：站点在断口上方，
            # 因此从观察点向上走永远到不了它，被删的调用也不会留下任何提及。于是"检出"的含义是
            # "工具走到了那一跳的被调方，并且没有把被删的边报回来"；而区分真答案与沉默的是它在同一次
            # 游走里有没有解析出别的边（看召回列）。
            break_detected = not present
        # The hit rule follows the layer, because the two layers are two different
        # questions. `impl` breaks a body and leaves the chain intact, so the walk
        # can reach the site and the answer can name it. `chain`/`both` delete a
        # call site, and a deleted call is not findable by walking callers upward —
        # no static index can report an edge that is not in the tree. What is
        # findable is that the callee lost the link: the honest criterion there is
        # "the tool reported that node as having no callers, and named it".
        # 命中规则随层次而变，因为两层是两个不同的问题。`impl` 破坏的是函数体，链路完好，因此游走能
        # 走到站点、答案能点名它。`chain`/`both` 删掉一处调用点，而被删的调用**无法**靠向上走调用者
        # 找到——任何静态索引都报不出一条不在树里的边。能找到的是"被调方失去了这条链路"，因此那里诚实
        # 的判据是"工具报出了那个节点没有调用者，并且点了它的名"。
        prose_claims = [
            line.strip()
            for line in answer.splitlines()
            if any(marker in line for marker in CLAIM_MARKERS)
        ]
        prose_false = [
            line
            for line in prose_claims
            if not (contains_path(line, site["file"], member) or site["name"] in line)
        ]
        if truth["layer"] == "impl":
            hit_rule = "site"
            hit = bool(named_file and named_symbol)
        else:
            hit_rule = "broken-link"
            hit = bool(break_detected and break_named_file and break_named_symbol)
        verdict["tools"][tool] = {
            "present": True,
            "hit": hit,
            "hit_rule": hit_rule,
            "named_site_file": named_file,
            "named_site_symbol": named_symbol,
            "named_site_together": named_together,
            "break_named_file": break_named_file,
            "break_named_symbol": break_named_symbol,
            "break_detected": break_detected,
            "site_reached": [site["file"], site["name"]] in result["reached_nodes"],
            "answer_bytes": result["answer_bytes"],
            "elapsed_ms": result["elapsed_ms"],
            "calls": result["calls"],
            "reached_nodes": len(result["reached_nodes"]),
            "queries": len(result["queries"]),
            "true_positive_edges": true_positive,
            "missed_edges": missed,
            "false_positive_edges": false_positive,
            "unverifiable_edges": len(unverifiable),
            "link": link,
            "link_ok": link_ok,
            "layer_words": {
                layer: sum(1 for word in words if word in answer)
                for layer, words in LAYER_WORDS.items()
            },
            "prose_claim_lines": len(prose_claims),
            "prose_false_positives": prose_false[:5],
        }
    return verdict


def median(values: list[float]) -> float:
    return round(statistics.median(values), 1) if values else 0.0


def table(verdicts: list[dict]) -> str:
    """Two tables: what was found, and what it cost.
    两张表：找到了什么，以及花了多少。"""
    verdicts = sorted(verdicts, key=lambda item: item["round"])
    hit_rows = [
        "| 轮 | 深度 | 层次 | 命中规则 | 站点（文件::符号） | MCP 命中 | CG 命中 | MCP 走到站点 | CG 走到站点 | MCP 断链检出 | CG 断链检出 | MCP 链路 | CG 链路 |",
        "|---|---|---|---|---|---|---|---|---|---|---|---|---|",
    ]
    cost_rows = [
        "| 轮 | MCP 字节 | CG 字节 | MCP ms | CG ms | MCP 调用 | CG 调用 | MCP TP/FP | CG TP/FP | MCP 边召回 | CG 边召回 |",
        "|---|---|---|---|---|---|---|---|---|---|---|",
    ]
    for verdict in verdicts:
        sides = verdict["tools"]
        rule = "site" if verdict["layer"] == "impl" else "broken-link"
        cells = [
            str(verdict["round"]),
            str(verdict["depth"]),
            verdict["layer"],
            rule,
            f"{verdict['site']['file']}::{verdict['site']['name']}",
        ]
        for tool in ("mcp", "codegraph"):
            cells.append("✓" if sides.get(tool, {}).get("hit") else ("✗" if sides.get(tool, {}).get("present") else "—"))
        for tool in ("mcp", "codegraph"):
            cells.append("✓" if sides.get(tool, {}).get("site_reached") else "✗")
        for tool in ("mcp", "codegraph"):
            # The break column is a `chain`/`both` question; on an `impl` round there
            # is no break to detect and a "✗" would read as a failure.
            # 断链列是 `chain`/`both` 的问题；`impl` 轮没有断链可检，写"✗"会被读成失败。
            value = sides.get(tool, {}).get("break_detected")
            cells.append("—" if rule == "site" or value is None else ("✓" if value else "✗"))
        for tool in ("mcp", "codegraph"):
            cells.append(sides.get(tool, {}).get("link", "—"))
        hit_rows.append("| " + " | ".join(cells) + " |")
        cells = [str(verdict["round"])]
        for tool in ("mcp", "codegraph"):
            cells.append(str(sides.get(tool, {}).get("answer_bytes", "—")))
        for tool in ("mcp", "codegraph"):
            cells.append(str(sides.get(tool, {}).get("elapsed_ms", "—")))
        for tool in ("mcp", "codegraph"):
            cells.append(str(sides.get(tool, {}).get("calls", "—")))
        for tool in ("mcp", "codegraph"):
            side = sides.get(tool, {})
            cells.append(
                f"{side.get('true_positive_edges', 0)}/{len(side.get('false_positive_edges', []))}"
            )
        for tool in ("mcp", "codegraph"):
            side = sides.get(tool, {})
            total = side.get("true_positive_edges", 0) + side.get("missed_edges", 0)
            cells.append(
                f"{side.get('true_positive_edges', 0)}/{total}" if side.get("present") else "—"
            )
        cost_rows.append("| " + " | ".join(cells) + " |")
    shallow = [v for v in verdicts if v["depth"] <= 2]
    deep = [v for v in verdicts if v["depth"] >= 3]
    summary = [
        "",
        "| 分组 | 轮数 | MCP 命中 | CG 命中 | MCP 字节中位 | CG 字节中位 | MCP ms 中位 | CG ms 中位 | MCP 误报边 | CG 误报边 |",
        "|---|---|---|---|---|---|---|---|---|---|",
    ]
    for label, group in (("浅 d=1–2", shallow), ("深 d≥3", deep), ("全部", verdicts)):
        cells = [label, str(len(group))]
        for tool in ("mcp", "codegraph"):
            hits = sum(1 for v in group if v["tools"].get(tool, {}).get("hit"))
            cells.append(f"{hits}/{len(group)}")
        for tool in ("mcp", "codegraph"):
            cells.append(str(median([v["tools"][tool]["answer_bytes"] for v in group if v["tools"].get(tool, {}).get("present")])))
        for tool in ("mcp", "codegraph"):
            cells.append(str(median([v["tools"][tool]["elapsed_ms"] for v in group if v["tools"].get(tool, {}).get("present")])))
        for tool in ("mcp", "codegraph"):
            cells.append(str(sum(len(v["tools"].get(tool, {}).get("false_positive_edges", [])) for v in group)))
        summary.append("| " + " | ".join(cells) + " |")
    return "\n".join(hit_rows + [""] + cost_rows + summary)


def excerpt(text: str, lines: int = 12, width: int = 200) -> str:
    """The first `lines` lines of an answer, with every cut marked.
    一份答案的前 `lines` 行，每一处截断都标出来。"""
    rows = text.splitlines()
    body = [row[:width] + (" …[行内截断]" if len(row) > width else "") for row in rows[:lines]]
    if len(rows) > lines:
        body.append(f"…[此处截断：全文 {len(rows)} 行，此处 {lines} 行]")
    return "\n".join(body)


def fragments(verdicts: list[dict], work: str) -> str:
    """One section per round, built from the artifacts and not from memory.
    每一轮一节，内容来自产物而不是记忆。"""
    out = []
    for verdict in sorted(verdicts, key=lambda item: item["round"]):
        number = verdict["round"]
        directory = os.path.join(work, "out", f"round-{number:02d}")
        with open(os.path.join(directory, "truth.json"), encoding="utf-8") as handle:
            truth = json.load(handle)
        rule = "site" if verdict["layer"] == "impl" else "broken-link"
        out.append(f"#### 第 {number} 轮：d={verdict['depth']}，层次 `{verdict['layer']}`，种子 {verdict['seed']}")
        out.append("")
        out.append("| 项 | 值 |")
        out.append("|---|---|")
        out.append(f"| 观察点（问题里唯一的输入） | `{verdict['observable']['file']}::{verdict['observable']['name']}` |")
        out.append(f"| 注入站点 | `{verdict['site']['file']}::{verdict['site']['name']}`（第 {verdict['site']['line']} 行） |")
        chain = " → ".join(f"`{node['name']}`" for node in truth["chain"])
        out.append(f"| 链（自观察点向上） | {chain} |")
        edits = []
        if truth.get("body_rewrite"):
            edit = truth["body_rewrite"]
            edits.append(f"函数体重写：第 {edit['line']} 行 `{edit['from']}`→`{edit['to']}`")
        if truth.get("deletion"):
            edit = truth["deletion"]
            edits.append(f"调用点删除：第 {edit['line']} 行 `{edit['removed'].strip()}`")
        out.append(f"| 变异 | {'；'.join(edits)} |")
        out.append(f"| 命中判据 | `{rule}` |")
        for tool in ("mcp", "codegraph"):
            side = verdict["tools"].get(tool, {})
            if not side.get("present"):
                out.append(f"| {tool} | 无答案 |")
                continue
            out.append(
                f"| {tool} | "
                f"{'命中' if side['hit'] else '未命中'}"
                f"；字节 {side['answer_bytes']}；{side['elapsed_ms']} ms；调用 {side['calls']}"
                f"；走到站点 {side['site_reached']}"
                f"；断链检出 {side['break_detected']}"
                f"；链路 {side['link']}"
                f"；点名了站点文件/符号 {int(side['named_site_file'])}/{int(side['named_site_symbol'])}"
                f"；点名了断链节点文件/符号 {int(side['break_named_file'])}/{int(side['break_named_symbol'])}"
                f"；TP/FP {side['true_positive_edges']}/{len(side['false_positive_edges'])}"
                f"；漏边 {side['missed_edges']}"
                f" |"
            )
        out.append("")
        out.append("该轮的原始输出片段（截断处已标出）：")
        out.append("")
        for tool in ("mcp", "codegraph"):
            path = os.path.join(directory, f"{tool}.answer.txt")
            if not os.path.isfile(path):
                continue
            with open(path, encoding="utf-8") as handle:
                answer = handle.read()
            out.append(f"`{tool}.answer.txt`（共 {len(answer.encode('utf-8'))} 字节）：")
            out.append("")
            out.append("```text")
            out.append(excerpt(answer))
            out.append("```")
            out.append("")
    return "\n".join(out)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=("one", "table", "fragments"), required=True)
    parser.add_argument("--work")
    parser.add_argument("--truth")
    parser.add_argument("--out-dir")
    parser.add_argument("--db")
    parser.add_argument("--copy")
    parser.add_argument("--members")
    parser.add_argument("--out")
    parser.add_argument("--verdicts")
    arguments = parser.parse_args(argv)

    if arguments.mode == "one":
        with open(arguments.truth, encoding="utf-8") as handle:
            truth = json.load(handle)
        verdict = judge_one(
            truth,
            arguments.out_dir,
            arguments.db,
            os.path.abspath(arguments.copy),
            arguments.members.split(","),
        )
        with open(arguments.out, "w", encoding="utf-8") as handle:
            json.dump(verdict, handle, ensure_ascii=False, indent=2)
        sides = " ".join(
            f"{tool}={'hit' if verdict['tools'].get(tool, {}).get('hit') else 'miss'}"
            f"({verdict['tools'].get(tool, {}).get('link', '-')})"
            for tool in ("mcp", "codegraph")
        )
        print(f"round {verdict['round']}: {sides}")
        return 0
    if arguments.mode == "fragments":
        verdicts = []
        for path in sorted(glob.glob(arguments.verdicts)):
            with open(path, encoding="utf-8") as handle:
                verdicts.append(json.load(handle))
        render = fragments(verdicts, arguments.work or ".")
        with open(arguments.out, "w", encoding="utf-8") as handle:
            handle.write(render + "\n")
        print(f"fragments: {len(verdicts)} rounds -> {arguments.out}")
        return 0
    verdicts = []
    for path in sorted(glob.glob(arguments.verdicts)):
        with open(path, encoding="utf-8") as handle:
            verdicts.append(json.load(handle))
    render = table(verdicts)
    if arguments.out:
        with open(arguments.out, "w", encoding="utf-8") as handle:
            handle.write(render + "\n")
    print(render)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
