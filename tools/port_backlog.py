"""The port backlog's shared readers.

`databank/port_backlog.toml` is the inventory; `data/neqsim/port_scope.toml` is the
pinned index of the physics-tree classes, their line counts and their references; the
specs name what is already covered. `tools/check_port_backlog.py` holds the inventory
to these, and `tools/gen_port_checklist.py` renders it.
"""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BACKLOG_REL = Path("databank") / "port_backlog.toml"
SCOPE_REL = Path("data") / "neqsim" / "port_scope.toml"
CLASSES_REL = Path("data") / "neqsim" / "classes.toml"
MANIFEST_REL = Path("databank") / "manifest.toml"

STATES = ("port", "internal", "unreachable", "excluded")
SIZES = ("S", "M", "L")
INTERNAL_KINDS = ("interface", "abstract", "enum")
BACKTICK = re.compile(r"`([^`]+)`")


def read(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def scope(root: Path = ROOT) -> dict[str, dict]:
    """Class name -> its scope row, from the committed pinned index."""
    data = read(root / SCOPE_REL)
    return {row["name"]: row for row in data["class"]}


def pin_classes(root: Path = ROOT) -> set[str]:
    data = read(root / CLASSES_REL)
    names: set[str] = set()
    for pin in data.get("pin", []):
        names.update(str(name) for name in pin.get("classes", []))
    return names


def spec_classes(root: Path = ROOT) -> set[str]:
    """Every NeqSim class a spec names, as a class or as a member's owner."""
    names: set[str] = set()
    for path in sorted((root / "specs").rglob("*.toml")):
        try:
            doc = tomllib.loads(path.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError:
            continue
        neqsim = doc.get("source", {}).get("neqsim", {})
        if not isinstance(neqsim, dict):
            continue
        class_name = neqsim.get("class")
        if isinstance(class_name, str):
            names.add(class_name.split(".")[0])
        for member in neqsim.get("members", []):
            names.add(str(member).split(".")[0])
    return names


def manifest_not_ported(root: Path = ROOT) -> list[dict[str, str]]:
    """Each manifest column whose reason begins `not-ported`."""
    data = read(root / MANIFEST_REL)
    out: list[dict[str, str]] = []
    for upstream in data.get("upstreams", []):
        for item in upstream.get("files", []):
            for column in item.get("columns", []):
                reason = str(column.get("reason", ""))
                if not reason.startswith("not-ported"):
                    continue
                quoted = BACKTICK.search(reason)
                out.append(
                    {
                        "file": str(item.get("id", "")),
                        "column": str(column.get("name", "")),
                        "class": quoted.group(1) if quoted else "",
                        "reason": reason,
                    }
                )
    return out


def metric(row: dict, index: dict[str, dict]) -> tuple[int, int]:
    """The row's summed line count and its distinct out-of-row references."""
    classes = [str(name) for name in row.get("classes", [])]
    loc = sum(int(index[name]["loc"]) for name in classes if name in index)
    refs: set[str] = set()
    for name in classes:
        if name in index:
            refs.update(str(ref) for ref in index[name].get("refs", []))
    return loc, len(refs - set(classes))


def size_of(loc: int, deps: int, _new_solver: bool, _capture: str) -> str:
    """The batching size: code volume and blast radius. Difficulty is the flags."""
    if loc <= 500 and deps <= 10:
        return "S"
    if loc <= 2500:
        return "M"
    return "L"


def all_classes(rows: list[dict]) -> set[str]:
    names: set[str] = set()
    for row in rows:
        names.update(str(name) for name in row.get("classes", []))
    return names
