"""What the bridge can run, and the spec behind one id."""

from __future__ import annotations

import json
import re
from typing import Any

from azoth._models_gen import MODELS
from azoth._registry_gen import CALCS
from azoth.harness.errors import UndefinedInput, UnknownId

SCHEMA = "azoth.harness/1"
CATALOGUE_MAX_LIMIT = 500

#: id -> (kind, entry). A calc has no `kind` field; a model has one, but the catalogue's
#: kind is the two trees rather than the model's own dispatch kind.
_BY_ID: dict[str, tuple[str, dict[str, Any]]] = {
    **{str(entry["id"]): ("calc", entry) for entry in CALCS},
    **{str(entry["id"]): ("model", entry) for entry in MODELS},
}


def entry_for(calc_id: str) -> tuple[str, dict[str, Any]]:
    """The kind and spec entry for an id, or an unknown-id refusal."""
    try:
        return _BY_ID[calc_id]
    except KeyError as exc:
        raise UnknownId(
            f"{calc_id!r} is not a registered calculation or model", field="id"
        ) from exc


def _jsonable(value: Any) -> Any:
    return json.loads(json.dumps(value, default=str))


_QUERY_TOKEN = re.compile(r"[a-z0-9_.]+")


def _tokens(query: str) -> list[str]:
    """The query's lowercased word tokens."""
    return _QUERY_TOKEN.findall(query.lower())


def _match_score(
    entry: dict[str, Any], tokens: list[str]
) -> tuple[int, list[str], bool, bool, bool]:
    """A token's best field, weighted: id 8, name 6, output 3, input 1.

    A token is counted once, against its strongest field, so a name that repeats the id
    does not outrank an entry that matches more of the query. The three flags separate an
    id/name hit from an input/output hit, so a query that names an id is not diluted by
    every entry that merely lists the word as a field.
    """
    entry_id = str(entry["id"]).lower()
    name = str(entry.get("name", "")).lower()
    outputs = [str(key).lower() for key in (entry.get("outputs") or {})]
    inputs = [str(key).lower() for key in (entry.get("inputs") or {})]
    score = 0
    matched: list[str] = []
    id_hit = name_hit = io_hit = False
    for token in tokens:
        if token in entry_id:
            best, id_hit = 8, True
        elif token in name:
            best, name_hit = 6, True
        elif any(token in key for key in outputs):
            best, io_hit = 3, True
        elif any(token in key for key in inputs):
            best, io_hit = 1, True
        else:
            continue
        score += best
        matched.append(token)
    return score, matched, id_hit, name_hit, io_hit


def catalogue(
    *,
    query: str | None = None,
    kind: str | None = None,
    limit: int = 100,
    offset: int = 0,
) -> dict[str, Any]:
    """Every registered id matching the filters, as a page of summaries."""
    if kind not in (None, "calc", "model"):
        raise UndefinedInput("kind must be 'calc' or 'model'", field="kind")
    if limit < 1 or limit > CATALOGUE_MAX_LIMIT:
        raise UndefinedInput(f"limit must be between 1 and {CATALOGUE_MAX_LIMIT}", field="limit")
    if offset < 0:
        raise UndefinedInput("offset must not be negative", field="offset")
    tokens = _tokens(query or "")
    matches: list[tuple[int, list[str], bool, bool, bool, str, dict[str, Any]]] = []
    for entry_kind, entry in _BY_ID.values():
        if kind is not None and entry_kind != kind:
            continue
        score, matched, id_hit, name_hit, io_hit = _match_score(entry, tokens)
        if tokens and not matched:
            continue
        matches.append((score, matched, id_hit, name_hit, io_hit, entry_kind, entry))
    if tokens:
        if any(item[2] for item in matches):
            matches = [item for item in matches if item[2]]
        elif any(item[3] for item in matches):
            matches = [item for item in matches if item[3]]
        else:
            matches = [item for item in matches if item[4]]
    matches.sort(key=lambda item: (-len(item[1]), -item[0], str(item[6]["id"])))
    page = matches[offset : offset + limit]
    return {
        "schema": SCHEMA,
        "kind": "catalogue",
        "total": len(matches),
        "offset": offset,
        "limit": limit,
        "items": [
            {
                "id": str(entry["id"]),
                "name": str(entry.get("name", "")),
                "kind": entry_kind,
                "namespace": str(entry["id"]).split(".", 1)[0],
                "inputs": sorted((entry.get("inputs") or {}).keys()),
                "outputs": sorted((entry.get("outputs") or {}).keys()),
                "matched": matched,
            }
            for _, matched, _, _, _, entry_kind, entry in page
        ],
    }


def spec(calc_id: str, *, full: bool = False) -> dict[str, Any]:
    """One id's declaration, trimmed to what a caller needs unless `full` is asked."""
    _, entry = entry_for(calc_id)
    if full:
        document = _jsonable(entry)
    else:
        keep = (
            "id",
            "name",
            "kind",
            "source",
            "equation",
            "inputs",
            "outputs",
            "valid_range",
            "assumptions",
            "references",
            "solver",
            "data",
            "implementations",
        )
        document = _jsonable({key: entry[key] for key in keep if key in entry})
    return {"schema": SCHEMA, "kind": "spec", "id": calc_id, "spec": document}
