#!/usr/bin/env python3
"""Keep a port on the backlog item it claims.

A commit that introduces a NeqSim class no spec already names carries a trailer

    Port-Item: <backlog row id>

The class must be in that row's `classes`; a spec that adds a class outside the row, or
adds one with no trailer, is refused. A change that introduces no new class needs no
trailer, so corrections and maintenance are not blocked.

    python tools/check_port_item.py --range origin/main..HEAD
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tomllib
from pathlib import Path

import port_backlog as backlog

ROOT = Path(__file__).resolve().parent.parent
TRAILER = re.compile(r"^Port-Item:\s*(.+)$", re.MULTILINE)


def _git(root: Path, *arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(root), *arguments], capture_output=True, text=True, check=False
    )


def spec_classes(text: str) -> set[str]:
    doc = tomllib.loads(text)
    neqsim = doc.get("source", {}).get("neqsim", {})
    if not isinstance(neqsim, dict):
        return set()
    names: set[str] = set()
    class_name = neqsim.get("class")
    if isinstance(class_name, str):
        names.add(class_name.split(".")[0])
    for member in neqsim.get("members", []):
        names.add(str(member).split(".")[0])
    return names


def per_spec(root: Path) -> dict[str, set[str]]:
    out: dict[str, set[str]] = {}
    for path in sorted((root / "specs").rglob("*.toml")):
        try:
            out[path.relative_to(root).as_posix()] = spec_classes(path.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError:
            continue
    return out


def check(root: Path = ROOT, revision_range: str = "origin/main..HEAD") -> list[str]:
    problems: list[str] = []
    base, _, head = revision_range.partition("..")
    if not base or not head:
        return [f"range {revision_range!r} is not BASE..HEAD"]
    diff = _git(root, "diff", "--name-only", revision_range, "--", "specs")
    if diff.returncode != 0:
        return [f"git diff {revision_range} failed: {diff.stderr.strip()}"]
    changed = [line for line in diff.stdout.splitlines() if line.endswith(".toml")]
    if not changed:
        return []

    trailers = TRAILER.findall(_git(root, "log", "--format=%B", revision_range).stdout)
    ids = [item.strip() for value in trailers for item in value.split(",") if item.strip()]
    backlog_data = backlog.read(root / backlog.BACKLOG_REL)
    rows = {
        str(row.get("id")): row
        for row in list(backlog_data.get("package", [])) + list(backlog_data.get("process", []))
    }
    scope: set[str] = set()
    for item in ids:
        row = rows.get(item)
        if row is None:
            problems.append(f"Port-Item {item!r} names no backlog row")
            continue
        scope.update(str(name) for name in row.get("classes", []))

    specs = per_spec(root)
    for path in changed:
        current = specs.get(path, set())
        previous = _git(root, "show", f"{base}:{path}")
        before = spec_classes(previous.stdout) if previous.returncode == 0 else set()
        new = current - before
        if not new:
            continue
        if not ids:
            problems.append(f"{path}: adds {sorted(new)} with no Port-Item trailer")
            continue
        for name in sorted(new):
            if name not in scope:
                problems.append(f"{path}: {name} is outside the Port-Item scope")
            for other, names in specs.items():
                if other != path and name in names:
                    problems.append(f"{path}: {name} is already named by {other}")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--range", default="origin/main..HEAD", dest="revision_range")
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    revision_range = args.revision_range
    if _git(args.root, "rev-parse", "--verify", revision_range.split("..")[0]).returncode != 0:
        revision_range = "HEAD^..HEAD"
    if _git(args.root, "rev-parse", "--verify", revision_range.split("..")[0]).returncode != 0:
        print("check_port_item: no base revision; skipping")
        return 0
    problems = check(args.root, revision_range)
    if problems:
        for problem in problems:
            print(f"  ERROR  {problem}", file=sys.stderr)
        print(f"check_port_item: FAILED with {len(problems)} problem(s)", file=sys.stderr)
        return 1
    print("check_port_item: OK (every new class is in its item's scope)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
