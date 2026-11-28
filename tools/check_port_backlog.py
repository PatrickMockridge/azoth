#!/usr/bin/env python3
"""Hold `databank/port_backlog.toml` to the pin and to the specs.

The inventory is full when every class in `data/neqsim/port_scope.toml` is either named
by a spec or owned by exactly one backlog row, and every row's size metric matches the
committed scope index. A row that is fully covered is stale and is refused; a row whose
tier is unset, whose state is unknown, or whose class is not at the pin is refused too.

    python tools/check_port_backlog.py
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

import port_backlog as backlog

ROOT = Path(__file__).resolve().parent.parent
TIERS = range(0, 5)


def check(root: Path = ROOT) -> list[str]:
    problems: list[str] = []
    data = backlog.read(root / backlog.BACKLOG_REL)
    rows = list(data.get("package", [])) + list(data.get("process", []))
    index = backlog.scope(root)
    pin = backlog.pin_classes(root)
    covered = backlog.spec_classes(root)

    seen_ids: set[str] = set()
    owner: dict[str, str] = {}
    for row in rows:
        row_id = str(row.get("id", ""))
        if not row_id:
            problems.append("a row has no id")
            continue
        if row_id in seen_ids:
            problems.append(f"{row_id}: duplicate id")
        seen_ids.add(row_id)
        tier = row.get("tier")
        if not isinstance(tier, int) or tier not in TIERS:
            problems.append(f"{row_id}: tier {tier!r} is not 0-4; the row is unclassified")
        state = row.get("state")
        if state not in backlog.STATES:
            problems.append(f"{row_id}: state {state!r} is not one of {backlog.STATES}")
        if not str(row.get("title", "")).strip():
            problems.append(f"{row_id}: no title")
        classes = [str(name) for name in row.get("classes", [])]
        if not classes:
            problems.append(f"{row_id}: no classes")
        for name in classes:
            if name not in pin:
                problems.append(f"{row_id}: {name} is not a class at the pin")
            if name in owner:
                problems.append(f"{row_id}: {name} is already owned by {owner[name]}")
            else:
                owner[name] = row_id
        if state in ("unreachable", "excluded") and not str(row.get("blocked_by", "")).strip():
            problems.append(f"{row_id}: {state} needs a blocked_by reason")
        if state == "port" and classes and all(name in covered for name in classes):
            problems.append(f"{row_id}: every class is covered; the row is stale")
        if state == "port":
            loc, deps = backlog.metric(row, index)
            if int(row.get("loc", -1)) != loc:
                problems.append(f"{row_id}: loc is {row.get('loc')}, the index gives {loc}")
            if int(row.get("deps", -1)) != deps:
                problems.append(f"{row_id}: deps is {row.get('deps')}, the index gives {deps}")
            expected = backlog.size_of(
                loc, deps, bool(row.get("new_solver", False)), str(row.get("capture", ""))
            )
            if row.get("size") != expected:
                problems.append(f"{row_id}: size is {row.get('size')}, the metric gives {expected}")

    for name in sorted(set(index) - covered - set(owner)):
        problems.append(f"{name}: a physics class in no row and no spec")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT)
    args = parser.parse_args()
    problems = check(args.root)
    if problems:
        for problem in problems:
            print(f"  ERROR  {problem}", file=sys.stderr)
        print(f"check_port_backlog: FAILED with {len(problems)} problem(s)", file=sys.stderr)
        return 1
    print("check_port_backlog: OK (every physics class is owned, every row is at the pin)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
