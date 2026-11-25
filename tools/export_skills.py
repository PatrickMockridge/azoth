#!/usr/bin/env python3
"""Export azoth's skill catalog into the native skill format of agent tools.

Reads `skills.toml` - the single source of truth - and each `SKILL.md` body, and
writes one skill per azoth skill, as YAML frontmatter (`name`, `description`) over
the body, into each target's discovery directory. The frontmatter and body are
identical for every target; only the directory and file layout differ.

Targets:

    dsh      <root>/.dsh/skills/<name>.md           DeepSeek Harness (flat)
    claude   <root>/.claude/skills/<name>/SKILL.md  Claude Code
    opencode <root>/.opencode/skills/<name>/SKILL.md  OpenCode

Usage:
    python tools/export_skills.py --target all
    python tools/export_skills.py --target claude
"""

from __future__ import annotations

import argparse
import sys
import tomllib
from collections.abc import Iterable, Sequence
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CATALOG = ROOT / "skills.toml"

#: Target -> (output subdirectory under the root, flat-file layout). A flat target
#: writes `<name>.md`; a directory target writes `<name>/SKILL.md`.
TARGETS: dict[str, tuple[str, bool]] = {
    "dsh": (".dsh/skills", True),
    "claude": (".claude/skills", False),
    "opencode": (".opencode/skills", False),
}


def _quote(value: str) -> str:
    """A YAML double-quoted scalar, escaping what YAML escapes."""
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def render(name: str, description: str, body: str) -> str:
    """One skill: `name` and `description` frontmatter over the azoth body."""
    return f"---\nname: {name}\ndescription: {_quote(description)}\n---\n{body}"


def render_extra(
    name: str,
    description: str,
    body: str,
    *,
    user_invocable: bool = True,
    when_to_use: str | None = None,
) -> str:
    """A skill that is not in `skills.toml`: a harness procedure, not a method.

    DeepSeek Harness accepts `user-invocable` and `whenToUse`; the catalog exports do not
    carry them, because a catalog skill is always discoverable and always model-invocable.
    """
    lines = ["---", f"name: {name}", f"description: {_quote(description)}"]
    if user_invocable:
        lines.append("user-invocable: true")
    if when_to_use is not None:
        lines.append(f"whenToUse: {_quote(when_to_use)}")
    lines += ["---", body]
    return "\n".join(lines)


def load_catalog(catalog: Path = CATALOG) -> list[dict[str, object]]:
    """The catalog entries, in `skills.toml` order."""
    document = tomllib.loads(catalog.read_text(encoding="utf-8"))
    return list(document.get("skill", []))


def expected(
    root: Path = ROOT,
    targets: Sequence[str] = tuple(TARGETS),
    names: Iterable[str] | None = None,
    *,
    source_root: Path = ROOT,
) -> dict[Path, str]:
    """Every file an export would write, mapped to its exact content.

    `root` is where the files go; `source_root` is the checkout the catalog and bodies are
    read from. They are the same in a launch, and separate when `--root` is an output
    directory. The rendering is separate from the writing so a launch check can compare the
    tree without touching it.
    """
    selected = None if names is None else set(names)
    files: dict[Path, str] = {}
    source = Path(source_root)
    entries = load_catalog(source / "skills.toml")
    unknown = set() if selected is None else selected - {str(e["name"]) for e in entries}
    if unknown:
        raise KeyError(f"skills not in the catalog: {sorted(unknown)}")
    for target in targets:
        subdir, flat = TARGETS[target]
        out = root / subdir
        for entry in entries:
            name = str(entry["name"])
            if selected is not None and name not in selected:
                continue
            body = source / str(entry["path"])
            if not body.is_file():
                print(f"  skip  {name}: {body} does not exist", file=sys.stderr)
                continue
            path = out / f"{name}.md" if flat else out / name / "SKILL.md"
            files[path] = render(name, str(entry["description"]), body.read_text(encoding="utf-8"))
    return files


def write_expected(files: dict[Path, str]) -> int:
    """Write rendered skills, and return how many files were written."""
    for path, text in files.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    return len(files)


def export_all(
    root: Path = ROOT,
    targets: Sequence[str] = tuple(TARGETS),
    names: Iterable[str] | None = None,
) -> int:
    """Write the catalog for `targets`, optionally only the named skills."""
    return write_expected(expected(root, targets, names))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--target",
        choices=["all", *TARGETS],
        default="all",
        help="which tool to write for (default: all)",
    )
    parser.add_argument(
        "--root", type=Path, default=ROOT, help="output root (default: the repository root)"
    )
    parser.add_argument(
        "--name",
        action="append",
        help="export only this catalog name; repeatable (default: every catalog entry)",
    )
    args = parser.parse_args()

    if not CATALOG.is_file():
        sys.exit(f"export_skills: {CATALOG} does not exist")

    targets = list(TARGETS) if args.target == "all" else [args.target]
    for target in targets:
        subdir, _ = TARGETS[target]
        written = export_all(args.root, [target], args.name)
        print(f"export_skills: wrote {written} skill file(s) to {args.root / subdir}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
