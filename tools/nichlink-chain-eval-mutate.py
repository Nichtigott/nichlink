#!/usr/bin/env python3
"""Apply (or revert) one constructed fault in a /tmp copy, and record its truth.
在 /tmp 副本里施加（或回滚）一处构造故障，并记录真值。

The truth is what was constructed, not what someone read: the file, the line, the
symbol, the operator that changed, the call that disappeared. `--revert` puts the
copy back byte for byte, which is what makes the repair scoring honest: the
"root-cause repair" is the exact inverse of the injection, and its diff is exactly
one hunk.
真值就是构造出来的东西，不是谁读出来的：文件、行、符号、被换掉的运算符、被删掉的调用。
`--revert` 把副本逐字节放回原样，这正是修复评分能诚实的原因："根因修复"就是注入的精确逆操作，
它的 diff 恰好是一处 hunk。

Forward order matters and is fixed: the body rewrite replaces characters on one
line (the line count never moves), then the call statement is deleted. Reversal
runs the same two steps backwards.
正向顺序有讲究而且是固定的：先做函数体重写（只替换一行里的字符，行数不变），再删除那条调用
语句。回滚按相反顺序做同样两步。
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys


def read_lines(path: str) -> list[str]:
    with open(path, encoding="utf-8") as handle:
        return handle.read().splitlines()


def write_lines(path: str, lines: list[str]) -> None:
    with open(path, "w", encoding="utf-8") as handle:
        handle.write("\n".join(lines) + "\n")


def digest(path: str) -> str:
    with open(path, "rb") as handle:
        return hashlib.sha256(handle.read()).hexdigest()


def apply(copy: str, truth: dict) -> None:
    edits: dict[str, list[str]] = {}
    for key in ("body_rewrite", "deletion"):
        edit = truth.get(key)
        if not edit:
            continue
        path = os.path.join(copy, edit["file"])
        if path not in edits:
            edits[path] = read_lines(path)
        lines = edits[path]
        if key == "body_rewrite":
            row = edit["line"] - 1
            column = edit["column"]
            before = edit["from"]
            text = lines[row]
            if text[column : column + len(before)] != before:
                raise SystemExit(
                    f"body rewrite no longer matches {edit['file']}:{edit['line']}"
                    f" column {column}: expected {before!r} in {text!r}"
                )
            lines[row] = text[:column] + edit["to"] + text[column + len(before) :]
        else:
            row = edit["line"] - 1
            if lines[row] != edit["removed"]:
                raise SystemExit(
                    f"deletion no longer matches {edit['file']}:{edit['line']}:"
                    f" expected {edit['removed']!r} but the line is {lines[row]!r}"
                )
            del lines[row]
    for path, lines in edits.items():
        write_lines(path, lines)


def revert(copy: str, truth: dict) -> None:
    edits: dict[str, list[str]] = {}
    for key in ("deletion", "body_rewrite"):
        edit = truth.get(key)
        if not edit:
            continue
        path = os.path.join(copy, edit["file"])
        if path not in edits:
            edits[path] = read_lines(path)
        lines = edits[path]
        if key == "deletion":
            lines.insert(edit["line"] - 1, edit["removed"])
        else:
            row = edit["line"] - 1
            column = edit["column"]
            after = edit["to"]
            text = lines[row]
            if text[column : column + len(after)] != after:
                raise SystemExit(
                    f"revert no longer matches {edit['file']}:{edit['line']}"
                    f" column {column}: expected {after!r} in {text!r}"
                )
            lines[row] = text[:column] + edit["from"] + text[column + len(after) :]
    for path, lines in edits.items():
        write_lines(path, lines)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--plan", required=True)
    parser.add_argument("--round", type=int, required=True)
    parser.add_argument("--copy", required=True)
    parser.add_argument("--truth", required=True, help="truth JSON, written or read")
    parser.add_argument("--revert", action="store_true")
    arguments = parser.parse_args(argv)

    if arguments.revert:
        with open(arguments.truth, encoding="utf-8") as handle:
            truth = json.load(handle)
        revert(arguments.copy, truth)
        truth["state"] = "reverted"
        with open(arguments.truth, "w", encoding="utf-8") as handle:
            json.dump(truth, handle, ensure_ascii=False, indent=2)
        print(f"round {truth['round']}: reverted")
        return 0

    with open(arguments.plan, encoding="utf-8") as handle:
        plan = json.load(handle)
    picked = next(item for item in plan["plan"] if item["round"] == arguments.round)
    truth = dict(picked)
    truth["state"] = "injected"
    touched = sorted(
        {
            edit["file"]
            for edit in (truth.get("body_rewrite"), truth.get("deletion"))
            if edit
        }
    )
    truth["touched_files"] = touched
    truth["sha256_before"] = {
        path: digest(os.path.join(arguments.copy, path)) for path in touched
    }
    seal_pristine(arguments.copy)
    apply(arguments.copy, truth)
    seal_injection(arguments.copy, arguments.round)
    truth["sha256_after"] = {
        path: digest(os.path.join(arguments.copy, path)) for path in touched
    }
    truth["mutated_lines"] = {
        edit["file"]: edit["line"]
        for edit in (truth.get("body_rewrite"), truth.get("deletion"))
        if edit
    }
    with open(arguments.truth, "w", encoding="utf-8") as handle:
        json.dump(truth, handle, ensure_ascii=False, indent=2)
    detail = []
    if truth.get("body_rewrite"):
        edit = truth["body_rewrite"]
        detail.append(f"{edit['file']}:{edit['line']} {edit['from']!r}->{edit['to']!r}")
    if truth.get("deletion"):
        edit = truth["deletion"]
        detail.append(f"{edit['file']}:{edit['line']} deleted {edit['removed'].strip()!r}")
    print(
        f"round {arguments.round}: d={truth['depth']} {truth['layer']}"
        f" site={truth['site']['file']}::{truth['site']['name']} | " + " | ".join(detail)
    )
    return 0


def git(copy, *args):
    """Run one git command in the copy, quietly."""
    return subprocess.run(
        ["git", *args], cwd=copy, capture_output=True, text=True
    )


def seal_pristine(copy):
    """Commit the pristine tree, so the injection can be shown as a diff later.

    Measured need (evaluation round 2): the control arm's work tree was not a checkout, so its only
    evidence for "this is what I changed" was a before/after grep and the audit could not ask for a
    diff. Two commits — pristine, then injected — keep the injector's own record out of the tree
    while making `git diff HEAD~1` the verification instrument the solvers were asked for. `target/`
    and `.codegraph/` are ignored: the copy's build directory is hundreds of megabytes and is not
    part of the question.

    给原始树一次提交，好让注入日后能以 diff 示人。第 2 轮量出的需求：对照组的工作树不是检出，它证明
    "我改了什么"的唯一凭证是改前/改后 grep，而审计要不到 diff。两次提交——原始、再注入——把注入记录
    留在树外，同时让 `git diff HEAD~1` 成为当初要求解题者给出的核对仪器。`target/` 与 `.codegraph/`
    被忽略：副本的构建目录有数百 MB，而且不属于这道题。
    """
    if git(copy, "rev-parse", "--git-dir").returncode == 0:
        return
    with open(os.path.join(copy, ".gitignore"), "a", encoding="utf-8") as handle:
        handle.write("\ntarget/\n.codegraph/\n")
    git(copy, "init", "-q")
    git(copy, "add", "-A")
    git(
        copy,
        "-c",
        "user.email=eval@nichlink.invalid",
        "-c",
        "user.name=nichlink eval",
        "commit",
        "-q",
        "-m",
        "import: the tree before this round's injection",
    )


def seal_injection(copy, round_number):
    """Commit the mutated tree as a second commit, so `git diff HEAD~1` is the mutation."""
    git(copy, "add", "-A")
    git(
        copy,
        "-c",
        "user.email=eval@nichlink.invalid",
        "-c",
        "user.name=nichlink eval",
        "commit",
        "-q",
        "-m",
        f"round {round_number}: the injected defect",
    )


if __name__ == "__main__":
    raise SystemExit(main())
