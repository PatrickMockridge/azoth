"""What the bridge can run, and the spec behind one id."""

from __future__ import annotations

import json
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
    needle = (query or "").lower()
    matches: list[tuple[str, dict[str, Any]]] = []
    for entry_kind, entry in _BY_ID.values():
        if kind is not None and entry_kind != kind:
            continue
        if (
            needle
            and needle not in str(entry["id"]).lower()
            and needle not in str(entry.get("name", "")).lower()
        ):
            continue
        matches.append((entry_kind, entry))
    matches.sort(key=lambda pair: str(pair[1]["id"]))
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
            }
            for entry_kind, entry in page
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
