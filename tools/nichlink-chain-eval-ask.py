#!/usr/bin/env python3
"""Ask both tools the same question about one mutated copy, and record the answers.
在同一份变异副本上向两个工具问同一个问题，并把答案记下来。

The procedure is one fixed search, run identically on both sides — only the tool's
own API spelling differs:

  1. callers of the observable (level 1),
  2. callers of each level's nodes, up to the round's depth,
  3. a symbol map of every distinct file reached at the deepest level,

with the same caps on both sides, and no site knowledge anywhere in it: the walk
starts from the observable alone and follows whatever the tool under test reports.
Every call is timed and its answer measured in bytes; the concatenated text is what
the judge scores, so neither side gets a human to tidy its answer.
这个过程是固定的一次搜索，两边跑法完全一样——只有工具自己的 API 写法不同：

  1. 观察点的调用者（第 1 层），
  2. 每一层节点的调用者，直到本轮深度，
  3. 最深层到达的每个不同文件的一份符号表，

两边上限相同，过程中任何地方都不含站点知识：游走只从观察点出发，沿着**被测工具自己**报出的东西走。
每次调用都计时、答案都记字节数；判分用的就是拼接后的文本，因此没有任何一边能请人替它整理答案。

The two sides are addressed the way each one is addressed in practice: the bridge
gets the owning member as `root` and a member-relative path (that is its documented
way to answer for one package), the CLI gets the copy as its project path with
copy-relative paths. The *question*, the caps and the order are identical.
两边按各自在实践中的用法寻址：桥以所属成员为 `root` 并传成员内相对路径（那是它为单个包作答的
文档化方式），CLI 以副本为项目路径并传副本内相对路径。**问题**、上限与顺序完全一致。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import select
import subprocess
import time

FANOUT = 10
PER_LEVEL = 12
MAX_NODES = 48
MAX_CALLS = 70
CALL_TIMEOUT = 180.0

MCP_CALLER_LINE = re.compile(r"callers \((\d+)\):\s*(.*)$")
MCP_CALLER_REF = re.compile(r"([A-Za-z0-9_./\-]+\.rs)::([A-Za-z0-9_]+)")
MCP_HEADER = re.compile(r"^([\w./\-]+\.rs):(\d+) fn ([A-Za-z0-9_]+)$")


class McpClient:
    """One stdio session with the bridge, with a per-call deadline.
    与桥的一次 stdio 会话，每次调用都有截止时间。"""

    def __init__(self, binary: str, root: str):
        self.process = subprocess.Popen(
            [binary],
            cwd=root,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        self.next_id = 1
        self.call(
            "initialize",
            {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "nichlink-chain-eval", "version": "1"},
            },
        )

    def call(self, method: str, params: dict) -> tuple[dict, float]:
        request = {"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params}
        self.next_id += 1
        started = time.monotonic()
        self.process.stdin.write(json.dumps(request) + "\n")
        self.process.stdin.flush()
        ready, _, _ = select.select([self.process.stdout], [], [], CALL_TIMEOUT)
        if not ready:
            self.process.kill()
            raise SystemExit(f"bridge did not answer {method} within {CALL_TIMEOUT}s")
        line = self.process.stdout.readline()
        return json.loads(line), (time.monotonic() - started) * 1000.0

    def tool_call(self, name: str, arguments: dict) -> tuple[str, float]:
        response, elapsed = self.call("tools/call", {"name": name, "arguments": arguments})
        if "error" in response:
            return json.dumps(response["error"], ensure_ascii=False), elapsed
        content = response.get("result", {}).get("content", [])
        return "\n".join(part.get("text", "") for part in content), elapsed

    def close(self) -> None:
        try:
            self.process.stdin.close()
        except Exception:
            pass
        try:
            self.process.wait(timeout=10)
        except Exception:
            self.process.kill()


def terminal(command: list[str], cwd: str) -> tuple[str, int, float]:
    started = time.monotonic()
    completed = subprocess.run(
        command, cwd=cwd, capture_output=True, text=True, timeout=CALL_TIMEOUT
    )
    elapsed = (time.monotonic() - started) * 1000.0
    text = completed.stdout
    if completed.returncode != 0:
        text += f"\n[exit {completed.returncode}]\n" + completed.stderr
    return text, completed.returncode, elapsed


def member_of(file: str, members: list[str]) -> str:
    """The workspace member directory that owns a path.
    拥有某个路径的工作区成员目录。"""
    for member in members:
        if file == member or file.startswith(member + "/"):
            return member
    return members[0]


def parse_mcp_callers(text: str, member: str, focus: str) -> list[dict]:
    """Callers of one definition, from the bridge's own rendering.
    从桥自己的渲染里取出某个定义的调用者。"""
    in_block = False
    for line in text.splitlines():
        stripped = line.strip()
        header = MCP_HEADER.match(stripped)
        if header:
            in_block = header.group(3) == focus
            continue
        if not in_block:
            continue
        match = MCP_CALLER_LINE.search(stripped)
        if match:
            return [
                {"file": f"{member}/{path}", "name": name}
                for path, name in MCP_CALLER_REF.findall(match.group(2))
            ]
    return []


def parse_codegraph_callers(text: str) -> list[dict]:
    """Callers from the CLI's JSON, which is the CLI's own rendering.
    从 CLI 的 JSON（即它自己的渲染）里取调用者。"""
    try:
        payload = json.loads(text)
    except json.JSONDecodeError:
        return []
    callers = payload.get("callers") if isinstance(payload, dict) else None
    if not isinstance(callers, list):
        return []
    return [
        {"file": entry.get("filePath", ""), "name": entry.get("name", "")}
        for entry in callers
        if isinstance(entry, dict)
    ]


def node_key(node: dict) -> tuple[str, str]:
    return (node["file"], node["name"])


def walk(tool: str, truth: dict, copy: str, members: list[str], binary: str | None):
    """Run the fixed search on one side; return the answer and its signals.
    在一边跑那次固定搜索；返回答案与信号。"""
    observable = truth["observable"]
    depth = truth["depth"]
    log: list[dict] = []
    answer_parts: list[str] = []
    queries: dict[str, list[dict]] = {}
    total_ms = 0.0
    calls = 0
    client = McpClient(binary, copy) if tool == "mcp" else None

    def record(query: str, node: dict, text: str, elapsed: float, code: int) -> None:
        nonlocal total_ms, calls
        calls += 1
        total_ms += elapsed
        log.append(
            {
                "seq": calls,
                "query": query,
                "node": node_key(node),
                "elapsed_ms": round(elapsed, 1),
                "bytes": len(text.encode("utf-8")),
                "exit_code": code,
                "text": text,
            }
        )
        answer_parts.append(text)

    def ask_callers(node: dict) -> list[dict]:
        member = member_of(node["file"], members)
        if tool == "mcp":
            relative = node["file"][len(member) + 1 :]
            text, elapsed = client.tool_call(
                "nichlink.callgraph",
                {"function": node["name"], "path": relative, "root": member, "limit": 50},
            )
            parsed = parse_mcp_callers(text, member, node["name"])
            code = 0
        else:
            text, code, elapsed = terminal(
                ["codegraph", "callers", node["name"], "-p", ".", "--json", "-l", "50"], copy
            )
            parsed = parse_codegraph_callers(text)
        record("callers", node, text, elapsed, code)
        queries[f"{node['file']}::{node['name']}"] = [
            caller for caller in parsed if node_key(caller) != node_key(node)
        ]
        return parsed

    def ask_symbols(file: str) -> None:
        member = member_of(file, members)
        if tool == "mcp":
            relative = file[len(member) + 1 :]
            text, elapsed = client.tool_call(
                "nichlink.inspect", {"path": relative, "root": member}
            )
            code = 0
        else:
            text, code, elapsed = terminal(
                ["codegraph", "node", "-f", file, "-p", ".", "--symbols-only"], copy
            )
        record("symbols", {"file": file, "name": ""}, text, elapsed, code)

    level = [observable]
    visited = {node_key(observable)}
    for _ in range(depth):
        next_level: list[dict] = []
        for node in sorted(level, key=node_key)[:PER_LEVEL]:
            if calls >= MAX_CALLS or len(visited) >= MAX_NODES:
                break
            for caller in sorted(ask_callers(node), key=node_key)[:FANOUT]:
                key = node_key(caller)
                if key in visited:
                    continue
                visited.add(key)
                next_level.append(caller)
        level = next_level
        if not level:
            break
    files = sorted({node["file"] for node in level})[:PER_LEVEL]
    for file in files:
        if calls >= MAX_CALLS:
            break
        ask_symbols(file)
    if client is not None:
        client.close()
    answer = "\n".join(answer_parts)
    return {
        "answer": answer,
        "queries": queries,
        "calls": calls,
        "elapsed_ms": round(total_ms, 1),
        "answer_bytes": len(answer.encode("utf-8")),
        "reached_nodes": sorted(visited),
        "deepest_level_files": files,
        "log": log,
    }


def main(argv=None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tool", choices=("mcp", "codegraph"), required=True)
    parser.add_argument("--truth", required=True)
    parser.add_argument("--copy", required=True)
    parser.add_argument("--out-dir", required=True)
    parser.add_argument("--members", required=True, help="comma-separated member dirs")
    parser.add_argument("--binary", default=None)
    arguments = parser.parse_args(argv)

    with open(arguments.truth, encoding="utf-8") as handle:
        truth = json.load(handle)
    os.makedirs(arguments.out_dir, exist_ok=True)
    result = walk(
        arguments.tool,
        truth,
        os.path.abspath(arguments.copy),
        arguments.members.split(","),
        arguments.binary,
    )
    prefix = os.path.join(arguments.out_dir, arguments.tool)
    with open(f"{prefix}.answer.txt", "w", encoding="utf-8") as handle:
        handle.write(result.pop("answer"))
    with open(f"{prefix}.calls.jsonl", "w", encoding="utf-8") as handle:
        for entry in result.pop("log"):
            handle.write(json.dumps(entry, ensure_ascii=False) + "\n")
    with open(f"{prefix}.result.json", "w", encoding="utf-8") as handle:
        json.dump(result, handle, ensure_ascii=False, indent=2)
    print(
        f"{arguments.tool}: {result['calls']} calls, {result['answer_bytes']} bytes,"
        f" {result['elapsed_ms']} ms, reached {len(result['reached_nodes'])} nodes"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
