#!/usr/bin/env python3
"""Score one repair: green recovery, minimality, root cause, collateral damage.
给一次修复打分：绿恢复、改动最小性、根因或症状、附带损伤。

Every number here is computed from files and logs, so a reader can re-derive it:

* **green** — the log's own summary: exit code, `test result:` counts. A run that
  exits 0 while `ignored` grew is reported as *not* a green recovery, because a
  test that stopped running is not a test that passed.
* **minimality** — the patch's line count against the pristine copy, measured with
  a real diff, plus how many files it touched and whether the injected file is
  among them.
* **root cause or symptom** — whether the patch lands on the injected file at the
  injected line, on a test file (a symptom patch), or somewhere else entirely.
* **collateral damage** — files touched outside the injected ones, and bypass
  tokens in the added lines (`#[allow(`, `#[ignore`, `assert!(true)`, …).
这里的每个数字都从文件与日志算出来，读者可以自己复现：

* **绿**——日志自己的汇总：退出码、`test result:` 计数。退出码为 0 而 `ignored` 变多的运行记为
  **不是**绿恢复，因为不再跑的测试不是通过的测试。
* **最小性**——补丁相对 pristine 副本的行数，用真正的 diff 量；另记它碰了几个文件、注入文件在不在其中。
* **根因或症状**——补丁落在注入文件的那一行、落在测试文件（症状补丁）、还是别处。
* **附带损伤**——注入文件之外被碰的文件，以及新增行里的绕过记号（`#[allow(`、`#[ignore`、
  `assert!(true)` 等）。
"""

from __future__ import annotations

import argparse
import difflib
import json
import os
import re

BYPASS_TOKENS = (
    "#[allow",
    "#[ignore",
    "#[cfg(not(test))]",
    "assert!(true)",
    "if false",
    "todo!(",
    "unimplemented!(",
    "unwrap_or_default()",
)
TEST_RESULT = re.compile(
    r"test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored"
)


def tree(copy: str, skip: tuple[str, ...]) -> dict[str, list[str]]:
    """Every tracked-looking text file of a copy, as lines.
    副本里每个看起来受跟踪的文本文件，按行。"""
    found = {}
    for base, directories, files in os.walk(copy):
        directories[:] = [
            name
            for name in directories
            if name not in ("target", ".codegraph", ".git", ".nichlink")
        ]
        for name in files:
            path = os.path.join(base, name)
            relative = os.path.relpath(path, copy)
            if relative.startswith(skip):
                continue
            try:
                with open(path, encoding="utf-8") as handle:
                    found[relative] = handle.read().splitlines()
            except (UnicodeDecodeError, OSError):
                continue
    return found


def patch_stats(base: str, copy: str) -> dict:
    """Which files a copy differs in, and by how many added/removed lines.
    副本与基准在哪些文件上不同，以及增删了多少行。"""
    before = tree(base, ("target", ".codegraph", "docs", "tools"))
    after = tree(copy, ("target", ".codegraph", "docs", "tools"))
    changed = []
    for path in sorted(set(before) | set(after)):
        old = before.get(path, [])
        new = after.get(path, [])
        if old == new:
            continue
        diff = list(
            difflib.unified_diff(old, new, lineterm="", n=0)
        )
        added = [line for line in diff if line.startswith("+") and not line.startswith("+++")]
        removed = [line for line in diff if line.startswith("-") and not line.startswith("---")]
        changed.append(
            {
                "file": path,
                "added": len(added),
                "removed": len(removed),
                "added_lines": [line[1:] for line in added],
                "removed_lines": [line[1:] for line in removed],
            }
        )
    return {
        "files": changed,
        "changed_files": len(changed),
        "added": sum(entry["added"] for entry in changed),
        "removed": sum(entry["removed"] for entry in changed),
    }


def read_log(path: str) -> dict:
    """A cargo log's own verdict, mechanically.
    cargo 日志自己的判词，机械地读。"""
    if not path or not os.path.isfile(path):
        return {"present": False}
    with open(path, encoding="utf-8", errors="replace") as handle:
        text = handle.read()
    results = [
        {"kind": kind, "passed": int(passed), "failed": int(failed), "ignored": int(ignored)}
        for kind, passed, failed, ignored in TEST_RESULT.findall(text)
    ]
    failed_names = sorted(
        set(re.findall(r"^\s*(\S+)\s+\.\.\.\s+FAILED", text, re.M))
        | set(re.findall(r"^---- (\S+) stdout ----", text, re.M))
    )
    tail = text.strip().splitlines()[-25:]
    return {
        "present": True,
        "results": results,
        "failed_tests": failed_names,
        "error": "error: could not compile" in text or "error[E" in text,
        "tail": tail,
    }


def mask_test(copy: str, test_name: str) -> dict:
    """Silence one failing test instead of fixing it — the negative control.
    让一个失败的测试闭嘴而不是修好它——负对照。

    This is not a repair anyone should ship; it exists so the rubric has a case it
    must refuse, and so a rubric that passes it is visibly broken.
    这不是任何人该出厂的修复；它存在是为了让评分表有一个必须拒绝的样本，也让"连它都通过的评分表"
    一眼可见地坏掉。
    """
    pattern = re.compile(r"^(\s*)#\[(tokio::)?test\]\s*$")
    # cargo prints a unit test as `module::path::test_name` and an integration test as
    # `test_name`; the source only ever spells the last segment.
    # cargo 把单元测试打印成 `module::path::test_name`、把集成测试打印成 `test_name`；源码里只会有最后一段。
    wanted = [test_name, test_name.rpartition("::")[2]]
    for base, directories, files in os.walk(copy):
        directories[:] = [
            name for name in directories if name not in ("target", ".codegraph", ".git")
        ]
        for name in files:
            path = os.path.join(base, name)
            try:
                with open(path, encoding="utf-8") as handle:
                    lines = handle.read().splitlines()
            except (UnicodeDecodeError, OSError):
                continue
            for index, line in enumerate(lines):
                if not any(f"fn {name}" in line for name in wanted):
                    continue
                for above in range(index - 1, max(index - 4, -1), -1):
                    match = pattern.match(lines[above])
                    if match:
                        lines.insert(
                            above, f'{match.group(1)}#[ignore = "chain-eval control"]'
                        )
                        with open(path, "w", encoding="utf-8") as handle:
                            handle.write("\n".join(lines) + "\n")
                        return {
                            "file": os.path.relpath(path, copy),
                            "line": above + 1,
                            "test": test_name,
                        }
    return {"file": None, "line": None, "test": test_name}


def main(argv=None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=("patch", "log", "mask"), required=True)
    parser.add_argument("--base")
    parser.add_argument("--copy")
    parser.add_argument("--truth")
    parser.add_argument("--log")
    parser.add_argument("--test-name")
    parser.add_argument("--exit-code", type=int, default=None)
    parser.add_argument("--out", required=True)
    arguments = parser.parse_args(argv)

    report = {"mode": arguments.mode}
    if arguments.mode == "mask":
        report.update(mask_test(arguments.copy, arguments.test_name))
    elif arguments.mode == "patch":
        with open(arguments.truth, encoding="utf-8") as handle:
            truth = json.load(handle)
        stats = patch_stats(arguments.base, arguments.copy)
        report.update(stats)
        report["injected_files"] = truth.get("touched_files", [])
        report["touched_injected_file"] = sorted(
            {
                entry["file"]
                for entry in stats["files"]
                if entry["file"] in report["injected_files"]
            }
        )
        report["bypass_tokens"] = sorted(
            {
                token
                for entry in stats["files"]
                for line in entry["added_lines"]
                for token in BYPASS_TOKENS
                if token in line
            }
        )
        report["outside_injected"] = sorted(
            {
                entry["file"]
                for entry in stats["files"]
                if entry["file"] not in report["injected_files"]
            }
        )
        report["on_injected_line"] = sorted(
            {
                entry["file"]
                for entry in stats["files"]
                if entry["file"] in report["injected_files"]
                and truth.get("mutated_lines", {}).get(entry["file"]) is not None
            }
        )
    else:
        log = read_log(arguments.log)
        log["exit_code"] = arguments.exit_code
        results = log.get("results", [])
        log["green"] = bool(
            log.get("present")
            and arguments.exit_code == 0
            and results
            and all(entry["failed"] == 0 for entry in results)
            and (not any(entry["ignored"] for entry in results))
        )
        log["green_but_ignored"] = bool(
            log.get("present")
            and arguments.exit_code == 0
            and any(entry["ignored"] for entry in results)
        )
        log["green_by_exit_only"] = arguments.exit_code == 0
        report.update(log)
    with open(arguments.out, "w", encoding="utf-8") as handle:
        json.dump(report, handle, ensure_ascii=False, indent=2)
    print(json.dumps({key: value for key, value in report.items() if key != "files" and key != "tail"}, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
