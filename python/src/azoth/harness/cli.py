"""`python -m azoth.harness`: the JSON contract the DSH plugin calls."""

from __future__ import annotations

import argparse
import json
import sys
import traceback
from typing import Any

from azoth.core.errors import AzothError
from azoth.harness.api import run, version_info
from azoth.harness.catalogue import SCHEMA, catalogue, spec
from azoth.harness.errors import HarnessError, InputTooLarge, OutputTooLarge

MAX_INPUT_BYTES = 1_000_000
MAX_OUTPUT_BYTES = 8_000_000


def _read_inputs(source: str) -> dict[str, Any]:
    raw = sys.stdin.buffer.read(MAX_INPUT_BYTES + 1) if source == "-" else source.encode("utf-8")
    if len(raw) > MAX_INPUT_BYTES:
        raise InputTooLarge(f"input is over the {MAX_INPUT_BYTES}-byte cap", field="inputs")
    try:
        document = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise HarnessError(f"inputs are not JSON: {exc}", field="inputs") from exc
    if not isinstance(document, dict):
        raise HarnessError("inputs must be a JSON object", field="inputs")
    return document


def _emit(document: dict[str, Any], *, pretty: bool) -> int:
    try:
        text = json.dumps(
            document,
            indent=2 if pretty else None,
            separators=None if pretty else (",", ":"),
            default=str,
            allow_nan=False,
        )
    except ValueError as exc:
        return _error(HarnessError(f"result is not finite JSON: {exc}"), pretty=pretty)
    if len(text.encode("utf-8")) > MAX_OUTPUT_BYTES:
        return _error(
            OutputTooLarge(f"output is over the {MAX_OUTPUT_BYTES}-byte cap"), pretty=pretty
        )
    sys.stdout.write(text + "\n")
    return 0


def _error(exc: Exception, *, pretty: bool, debug: bool = False) -> int:
    if debug:
        traceback.print_exc(file=sys.stderr)
    if isinstance(exc, HarnessError):
        code, kind, field = exc.exit_code, exc.kind, exc.field
    elif isinstance(exc, AzothError):
        code, kind, field = 4, type(exc).__name__, None
    else:
        code, kind, field = 5, type(exc).__name__, None
    document = {
        "schema": SCHEMA,
        "kind": "error",
        "error": {"type": kind, "field": field, "message": str(exc)},
    }
    sys.stdout.write(
        json.dumps(document, indent=2 if pretty else None, default=str, allow_nan=False) + "\n"
    )
    return code


def parser() -> argparse.ArgumentParser:
    parent = argparse.ArgumentParser(add_help=False)
    parent.add_argument("--pretty", action="store_true", help="indent the JSON")
    parent.add_argument("--debug", action="store_true", help="print a traceback on failure")
    root = argparse.ArgumentParser(description="azoth's calculation bridge")
    commands = root.add_subparsers(dest="command", required=True)

    listing = commands.add_parser("catalogue", parents=[parent])
    listing.add_argument("--query")
    listing.add_argument("--kind", choices=("calc", "model"))
    listing.add_argument("--limit", type=int, default=100)
    listing.add_argument("--offset", type=int, default=0)

    one = commands.add_parser("spec", parents=[parent])
    one.add_argument("id")
    one.add_argument("--full", action="store_true")

    calc = commands.add_parser("run", parents=[parent])
    calc.add_argument("id")
    calc.add_argument("--inputs-json", required=True, help="a JSON object, or `-` for stdin")
    calc.add_argument("--keycard")
    calc.add_argument("--backend", choices=("auto", "python", "rust"), default="auto")

    commands.add_parser("version", parents=[parent])
    return root


def main(argv: list[str] | None = None) -> int:
    args = parser().parse_args(argv)
    try:
        if args.command == "catalogue":
            document = catalogue(
                query=args.query, kind=args.kind, limit=args.limit, offset=args.offset
            )
        elif args.command == "spec":
            document = spec(args.id, full=args.full)
        elif args.command == "run":
            document = run(
                args.id,
                _read_inputs(args.inputs_json),
                keycard=args.keycard,
                backend=args.backend,
            )
        else:
            document = version_info()
    except (HarnessError, AzothError) as exc:
        return _error(exc, pretty=args.pretty, debug=args.debug)
    except Exception as exc:
        return _error(exc, pretty=args.pretty, debug=args.debug)
    return _emit(document, pretty=args.pretty)
