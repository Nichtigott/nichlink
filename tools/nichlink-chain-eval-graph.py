#!/usr/bin/env python3
"""Seed-driven injection-site selection for the call-chain evaluation.
调用链评测的种子驱动注入点选取。

Nothing in this file names a symbol or a source file: the edge set comes from a
codegraph index of a pristine copy of the tree, the sites come from a walk over
that edge set, and the pseudo-random choice is `random.Random(seed)`. Everything
hand-written here is a *rule* — which members are in scope, how a call site is
recognized as deletable, how a body line is recognized as mutable — never a
scenario.
本文件不含任何符号名或源文件名：边集来自 pristine 副本的 codegraph 索引，站点来自该边集上的
一次游走，伪随机选择是 `random.Random(seed)`。这里手写的全都是**规则**——哪些成员在范围内、
什么样的调用点可删、什么样的函数体行可改——从来没有场景。

Scope is derived from the manifests, not typed in: a workspace member is in scope
when its own `Cargo.toml` does not set `publish = false` (that keeps the audit
crates and the example hosts out), minus the module the evaluated instrument's own
bin target enters (that boundary keeps the instrument out of its own crime scene),
minus test files (a lesion inside a test measures the test, not the chain).
范围从清单推导而不是敲进去：某成员自己的 `Cargo.toml` 没写 `publish = false` 时该成员在范围内
（审计 crate 与示例宿主因此出局），再减去被评测仪器自己的 bin target 进入的那个模块（这条边界
让仪器不出现在自己的案发现场），再减去测试文件（测试里的病灶量的是测试，不是调用链）。
"""

from __future__ import annotations

import argparse
import json
import os
import random
import re
import sqlite3
import sys

FUNCTION_KINDS = ("function", "method", "constructor")
# A whole line that is one call statement: `path::to::f(a, b);`, `self.f(x);`,
# `Type::new(y);`. `!` is absent from the character class on purpose, so a macro
# call (`assert!(...)`) can never match, and a keyword check below rejects the
# lines that merely start with a word and end with `);` (`return f();`).
# 整行是一条调用语句：`path::to::f(a, b);`、`self.f(x);`、`Type::new(y);`。字符类里有意不含 `!`，
# 因此宏调用（`assert!(...)`）永不匹配；下面的关键字检查拒掉那些只是以词开头、以 `);` 结尾的行
# （`return f();`）。
CALL_STATEMENT = re.compile(
    r"^\s*[A-Za-z_(&*][\w:.<>\[\]&* ]*\(([^;{}]*)\)\s*;\s*(?://.*)?$"
)
CALL_FORBIDDEN_HEAD = re.compile(
    r"^\s*(let|return|if|while|match|for|else|unsafe|loop|break|continue|"
    r"const|static|type|impl|mod|use|pub|where|as|in)\b"
)
FN_SIGNATURE_HEAD = re.compile(
    r"^\s*(pub(\([^)]*\))?\s+)?(default\s+)?(const\s+)?(async\s+)?(unsafe\s+)?(extern\s+\"[^\"]*\"\s+)?fn\s"
)
# Body rewrites that cannot change a type: comparisons and boolean connectives.
# `+`/`-` are deliberately absent — a numeric addition and a string concatenation
# are the same token, so swapping one would make the copy fail to compile, and a
# copy that does not compile measures the compiler rather than the call graph.
# 不改变类型的函数体重写：比较与布尔连接。有意不要 `+`/`-`——数值加法与字符串拼接是同一个记号，
# 换掉它会让副本编译不过，而编译不过的副本量的是编译器而不是调用图。
BODY_REWRITES = (
    (re.compile(r"(?<![=!<>])==(?!=)"), "!="),
    (re.compile(r"!="), "=="),
    (re.compile(r"(?<![<>])<=(?!=)"), "<"),
    (re.compile(r"(?<![<>])>=(?!=)"), ">"),
    (re.compile(r"&&"), "||"),
    (re.compile(r"\|\|"), "&&"),
    (re.compile(r"\btrue\b"), "false"),
    (re.compile(r"\bfalse\b"), "true"),
)


def members_in_scope(root: str) -> tuple[list[str], str | None]:
    """Workspace members to inject into, and the instrument boundary directory.
    可注入的工作区成员，以及仪器边界目录。"""
    with open(os.path.join(root, "Cargo.toml"), encoding="utf-8") as handle:
        manifest = handle.read()
    listed = re.search(r"members\s*=\s*\[([^\]]*)\]", manifest, re.S)
    names = re.findall(r"\"([^\"]+)\"", listed.group(1)) if listed else []
    in_scope = []
    for name in names:
        path = os.path.join(root, name, "Cargo.toml")
        if not os.path.isfile(path):
            continue
        with open(path, encoding="utf-8") as handle:
            text = handle.read()
        if re.search(r"^\s*publish\s*=\s*false", text, re.M):
            continue
        in_scope.append(name)
    return in_scope, instrument_directory(root, in_scope)


def instrument_directory(root: str, members: list[str]) -> str | None:
    """The directory tree of the module the evaluated bin target enters.
    被评测 bin target 进入的模块所在的目录树。"""
    for member in members:
        with open(os.path.join(root, member, "Cargo.toml"), encoding="utf-8") as handle:
            text = handle.read()
        for block in re.findall(r"\[\[bin\]\](.*?)(?=\n\[|\Z)", text, re.S):
            if 'name = "nichlink-mcp"' not in block:
                continue
            entry = re.search(r'path\s*=\s*"([^"]+)"', block)
            if not entry:
                continue
            source = os.path.join(root, member, entry.group(1))
            with open(source, encoding="utf-8") as handle:
                body = handle.read()
            # The bin is thin: it names the module it enters. Resolve that module
            # through the crate root's `#[path]` declarations to a directory.
            # bin 是薄的：它点名自己进入的模块。经 crate 根的 `#[path]` 声明把该模块解析成目录。
            for module in re.findall(r"nichlink_toolchain::(\w+)", body):
                lib = os.path.join(root, member, "src", "lib.rs")
                with open(lib, encoding="utf-8") as handle:
                    lib_text = handle.read()
                found = re.search(
                    r'#\[path\s*=\s*"([^"]+)"\]\s*pub mod\s+' + re.escape(module) + r"\s*;",
                    lib_text,
                )
                if found:
                    return os.path.normpath(
                        os.path.join(member, "src", os.path.dirname(found.group(1)))
                    )
    return None


def load_nodes(root: str, db: str, member_prefixes: list[str], boundary: str | None):
    """In-scope function nodes, keyed by id, with their bodies' line ranges.
    范围内的函数节点，按 id 索引，带函数体行范围。"""
    connection = sqlite3.connect(f"file:{db}?mode=ro", uri=True)
    rows = connection.execute(
        "SELECT id, kind, name, qualified_name, file_path, start_line, end_line"
        " FROM nodes WHERE language = 'rust'"
    ).fetchall()
    nodes = {}
    for node_id, kind, name, qualified, path, start, end in rows:
        if kind not in FUNCTION_KINDS:
            continue
        relative = os.path.relpath(path, root) if os.path.isabs(path) else path
        if relative.startswith(".."):
            continue
        if not relative.endswith(".rs"):
            continue
        pieces = relative.split("/")
        if "tests" in pieces or pieces[-1].endswith("_tests.rs") or pieces[-1] == "tests.rs":
            continue
        if boundary and (relative + "/").startswith(boundary + "/"):
            continue
        member = next(
            (m for m in member_prefixes if relative.startswith(m + "/")), None
        )
        if member is None:
            continue
        nodes[node_id] = {
            "id": node_id,
            "name": name,
            "qual": qualified,
            "file": relative,
            "member": member,
            "start": start,
            "end": end,
        }
    return nodes, connection


def load_edges(connection, nodes):
    """Caller -> [(callee, line)] and callee -> [(caller, line)], in-scope only.
    调用者 -> [(被调者, 行)] 与被调者 -> [(调用者, 行)]，仅范围内的节点。"""
    down: dict[str, list[tuple[str, int]]] = {node_id: [] for node_id in nodes}
    up: dict[str, list[tuple[str, int]]] = {node_id: [] for node_id in nodes}
    rows = connection.execute(
        "SELECT source, target, line FROM edges WHERE kind = 'calls' AND line IS NOT NULL"
    ).fetchall()
    for source, target, line in rows:
        if source in nodes and target in nodes:
            down[source].append((target, line))
            up[target].append((source, line))
    for table in (down, up):
        for key in table:
            table[key] = sorted(set(table[key]))
    return down, up


def test_region_start(lines: list[str]) -> int:
    """First line of the file's `#[cfg(test)]` region, or past the end.
    文件里 `#[cfg(test)]` 区域的第一行，或文件末尾之后。"""
    for index, text in enumerate(lines, start=1):
        if re.match(r"^\s*#\[cfg\(test\)\]", text):
            return index
    return len(lines) + 1


def is_test_node(lines: list[str], node: dict) -> bool:
    """Whether a definition sits in a test region or carries `#[test]`.
    某个定义是否位于测试区域，或带有 `#[test]`。"""
    if node["start"] > test_region_start(lines):
        return True
    for number in range(max(node["start"] - 4, 0), node["start"] - 1):
        if re.match(r"^\s*#\[(tokio::)?test\]", lines[number]):
            return True
    return False


def chain_candidate(lines: list[str], edge_line: int, callee_name: str, caller: dict):
    """Whether the call site on `edge_line` is a statement that can be dropped.
    调用点所在的那一行是不是一条可以删掉的语句。

    "Dropped" has to mean the link is really gone: the callee's name must appear
    exactly once in the caller's body. A caller that calls the same function twice
    keeps the edge when one call site is deleted, and a truth that says the edge is
    gone would then be a truth contradicted by the tree — which is how a judge ends
    up calling a correct answer wrong.
    "摘掉"必须意味着链路真的没了：被调者的名字在调用者函数体里必须恰好出现一次。同一个函数被调用两次
    的调用者，删掉一处调用点之后边仍然在，而"边没了"的真值就会与树相矛盾——判分正是这样把正确答案判
    错的。
    """
    if not 1 <= edge_line <= len(lines):
        return None
    text = lines[edge_line - 1]
    if not CALL_STATEMENT.match(text) or CALL_FORBIDDEN_HEAD.match(text):
        return None
    if not re.search(r"\b" + re.escape(callee_name) + r"\b", text):
        return None
    body = lines[max(caller["start"] - 1, 0) : min(caller["end"], len(lines))]
    occurrences = sum(
        len(re.findall(r"\b" + re.escape(callee_name) + r"\b", line)) for line in body
    )
    if occurrences != 1:
        return None
    return {"line": edge_line, "removed": text, "callee_name_occurrences": occurrences}


def body_candidates(lines: list[str], node: dict):
    """Every (line, column, literal, replacement) rewrite candidate in one body.
    某个函数体内全部的 (行, 列, 原文, 替换) 重写候选。"""
    candidates = []
    for number in range(max(node["start"] - 1, 0), min(node["end"], len(lines))):
        text = lines[number]
        stripped = text.strip()
        if stripped.startswith("//") or stripped.startswith("#["):
            continue
        if FN_SIGNATURE_HEAD.match(text):
            continue
        for pattern, replacement in BODY_REWRITES:
            for match in pattern.finditer(text):
                candidates.append(
                    (number + 1, match.start(), match.group(0), replacement)
                )
    return candidates


def pick_round(
    nodes: dict,
    down: dict,
    up: dict,
    sources: dict,
    meta: dict,
    seed: int,
    depth: int,
    layer: str,
    member: str | None = None,
    avoid_sites: frozenset = frozenset(),
    attempts: int = 400,
):
    """Walk `depth` hops up from a seed-picked observable and build one injection.
    由种子挑出的观察点向上走 `depth` 跳，构造一次注入。"""
    rng = random.Random(seed)
    name_counts = meta["name_counts"]
    is_test = meta["is_test"]
    body_cache = meta["body_cache"]
    observables = meta["observables"][:]
    rng.shuffle(observables)
    # The rotated member constrains where the injection lands, not where the walk
    # starts: a chain that crosses a crate boundary is a legitimate chain. The
    # observables that live in the target member are tried first so the walk stays
    # cheap.
    # 轮转的成员约束的是注入落在哪里，而不是游走从哪里开始：跨 crate 边界的链也是合法的链。先试
    # 落在目标成员里的观察点，游走才便宜。
    if member is not None:
        observables.sort(key=lambda node_id: nodes[node_id]["member"] != member)
    for observable_id in observables[:attempts]:
        chain = [observable_id]
        hops = []
        failed = False
        for _ in range(depth):
            current = chain[-1]
            callers = [
                (caller, line)
                for caller, line in up[current]
                if caller not in chain
                and name_counts[nodes[caller]["name"]] == 1
                and caller not in is_test
            ]
            if not callers:
                failed = True
                break
            caller, line = callers[rng.randrange(len(callers))]
            hops.append({"caller": caller, "callee": current, "line": line})
            chain.append(caller)
        if failed:
            continue
        site = nodes[chain[-1]]
        if site["id"] in avoid_sites:
            continue
        if member is not None and site["member"] != member:
            continue
        if site["id"] not in body_cache:
            body_cache[site["id"]] = body_candidates(sources[site["file"]], site)
        candidates = body_cache[site["id"]]
        # The layer decides which fault is constructed, and only that fault: an
        # `impl` round rewrites a body and leaves every call in place, a `chain`
        # round deletes one call and leaves every body alone, `both` does both at
        # the same site. Storing both edits for every layer would make `chain` and
        # `both` the same experiment while the record still claimed two layers.
        # 层次决定构造哪一种故障，而且只构造那一种：`impl` 轮改函数体、把每个调用都留着，`chain` 轮
        # 删一处调用、不碰任何函数体，`both` 在同一点两者都做。给每一层都存两处编辑会让 `chain` 与
        # `both` 变成同一个实验，而记录里还写着两个层次。
        if layer in ("impl", "both") and not candidates:
            continue
        rewrite = (
            candidates[rng.randrange(len(candidates))]
            if layer in ("impl", "both") and candidates
            else None
        )
        hop = hops[-1]
        hop_caller = nodes[hop["caller"]]
        hop_lines = sources[hop_caller["file"]]
        deletion = (
            chain_candidate(
                hop_lines, hop["line"], nodes[hop["callee"]]["name"], hop_caller
            )
            if layer in ("chain", "both")
            else None
        )
        if layer in ("chain", "both") and deletion is None:
            continue
        # The deletion and the body rewrite may live in the same file. The body
        # rewrite replaces characters on one line (the line count does not move),
        # so the deletion's recorded line number stays valid when both are applied.
        # 删除与函数体重写可能在同一文件。函数体重写只替换一行里的字符（行数不变），因此两者同时
        # 施加时，删除记下的行号仍然有效。
        if deletion is not None and hop["caller"] == chain[-1]:
            deletion = dict(deletion)
            deletion["same_file_as_body"] = True
        return {
            "seed": seed,
            "depth": depth,
            "layer": layer,
            "chain": [nodes[node_id] for node_id in chain],
            "observable": nodes[chain[0]],
            "site": site,
            "hop": hop,
            "body_rewrite": None
            if rewrite is None
            else {
                "file": site["file"],
                "line": rewrite[0],
                "column": rewrite[1],
                "from": rewrite[2],
                "to": rewrite[3],
            },
            "deletion": None
            if deletion is None
            else {
                "file": nodes[hop["caller"]]["file"],
                "line": deletion["line"],
                "removed": deletion["removed"],
            },
        }
    return None


def main(argv=None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", required=True, help="pristine copy (codegraph index)")
    parser.add_argument("--db", default=None, help="defaults to <root>/.codegraph/codegraph.db")
    parser.add_argument("--seed", type=int, default=20260929)
    parser.add_argument(
        "--plan",
        default="1:impl,1:chain,2:impl,2:chain,3:impl,3:chain,4:impl,4:chain",
        help="comma-separated depth:layer rounds, in order",
    )
    parser.add_argument("--out", required=True, help="plan JSON path")
    parser.add_argument("--reference", default=None, help="reference graph JSON path")
    arguments = parser.parse_args(argv)

    root = os.path.abspath(arguments.root)
    db = arguments.db or os.path.join(root, ".codegraph", "codegraph.db")
    prefixes, boundary = members_in_scope(root)
    if not prefixes:
        print("no in-scope workspace member", file=sys.stderr)
        return 1
    nodes, connection = load_nodes(root, db, prefixes, boundary)
    down, up = load_edges(connection, nodes)
    connection.close()
    sources = {}
    for node in nodes.values():
        if node["file"] not in sources:
            with open(os.path.join(root, node["file"]), encoding="utf-8") as handle:
                sources[node["file"]] = handle.read().splitlines()

    name_counts: dict[str, int] = {}
    for node in nodes.values():
        name_counts[node["name"]] = name_counts.get(node["name"], 0) + 1
    is_test = {
        node_id
        for node_id, node in nodes.items()
        if is_test_node(sources[node["file"]], node)
    }
    meta = {
        "name_counts": name_counts,
        "is_test": is_test,
        "body_cache": {},
        "observables": sorted(
            (
                node_id
                for node_id, node in nodes.items()
                if up[node_id]
                and name_counts[node["name"]] == 1
                and node_id not in is_test
            ),
            key=lambda node_id: (nodes[node_id]["file"], nodes[node_id]["start"]),
        ),
    }

    plan = []
    used_sites: set[str] = set()
    for index, entry in enumerate(arguments.plan.split(",")):
        depth_text, layer = entry.split(":")
        # Members rotate with the round so that every in-scope member is measured
        # at least once; the seed still decides the site inside that member, and a
        # member that cannot carry this depth falls through to the next one in a
        # fixed order (a small member simply has no deep chain to offer, and a
        # missing round would be a hole in the table rather than a measurement).
        # 成员随轮次轮转，使每个范围内的成员至少被测到一次；成员内部的站点仍由种子决定；扛不住该
        # 深度的成员按固定顺序让给下一个（小成员本来就没有深链可给，而缺一轮是表格上的洞而不是
        # 一次测量）。
        ordered = prefixes[index % len(prefixes):] + prefixes[: index % len(prefixes)]
        picked = None
        for member in ordered:
            for attempt in range(24):
                picked = pick_round(
                    nodes,
                    down,
                    up,
                    sources,
                    meta,
                    arguments.seed + index * 7919 + attempt,
                    int(depth_text),
                    layer,
                    member=member,
                    avoid_sites=frozenset(used_sites),
                )
                if picked is not None:
                    break
            if picked is not None:
                break
        if picked is None:
            print(
                f"round {index + 1} (depth {depth_text}, {layer}) found no site in"
                f" any of {ordered}",
                file=sys.stderr,
            )
            return 2
        used_sites.add(picked["site"]["id"])
        picked["round"] = index + 1
        plan.append(picked)
    print(
        json.dumps(
            {
                "seed": arguments.seed,
                "members_in_scope": prefixes,
                "instrument_boundary": boundary,
                "plan": plan,
            },
            ensure_ascii=False,
            indent=2,
        ),
        file=open(arguments.out, "w", encoding="utf-8"),
    )
    if arguments.reference:
        print(
            json.dumps(
                {
                    "nodes": nodes,
                    "callers": {key: value for key, value in up.items() if value},
                    "callees": {key: value for key, value in down.items() if value},
                },
                ensure_ascii=False,
            ),
            file=open(arguments.reference, "w", encoding="utf-8"),
        )
    print(
        f"plan: {len(plan)} rounds, members {prefixes}, boundary {boundary},"
        f" nodes {len(nodes)}"
    )
    for picked in plan:
        print(
            f"  round {picked['round']:>2} d={picked['depth']} {picked['layer']:<5}"
            f" seed={picked['seed']} site={picked['site']['file']}::{picked['site']['name']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
