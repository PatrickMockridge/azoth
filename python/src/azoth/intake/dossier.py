"""The authored layers of an intake dossier.

A dossier says where the material came from (`sources`), what it claims (`claims`),
and how each input of a target is bound (`targets`). The register, the requests and
the verdict are **not** authored: :mod:`azoth.intake.check` derives them, so a
review cannot certify itself. The parser here is the runtime authority; the JSON
Schema in ``specs/schema/intake.schema.json`` is the authoring contract and the
tests hold the two together.
"""

from __future__ import annotations

import json
import tomllib
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from azoth.core.errors import InvalidInputError

SCHEMA = "azoth.intake/1"

_KINDS = frozenset({"prompt", "file", "calculation"})
_STATUSES = frozenset({"used", "blocked", "absent"})
_OUTCOMES = frozenset({"used", "blocked", "absent"})
_PREMISE_VERDICTS = frozenset({"evidenced", "assumed", "violated", "not_applicable"})
_AUDIT_VERDICTS = frozenset({"signed", "returned"})


class DossierInvalid(InvalidInputError):
    """A dossier that cannot be read as one. Not a verdict: there is nothing to review."""


def _mapping(value: Any, field: str) -> Mapping[str, Any]:
    if not isinstance(value, Mapping):
        raise DossierInvalid(field, "expected an object")
    return value


def _sequence(value: Any, field: str) -> Sequence[Any]:
    if isinstance(value, (str, bytes)) or not isinstance(value, Sequence):
        raise DossierInvalid(field, "expected a list")
    return value


def _text(value: Any, field: str) -> str:
    if not isinstance(value, str) or not value:
        raise DossierInvalid(field, "expected a non-empty string")
    return value


def _text_or_none(value: Any, field: str) -> str | None:
    if value is None:
        return None
    return _text(value, field)


def _flag(value: Any, field: str) -> bool:
    if not isinstance(value, bool):
        raise DossierInvalid(field, "expected true or false")
    return value


def _only_keys(document: Mapping[str, Any], allowed: set[str], field: str) -> None:
    unknown = set(document) - allowed
    if unknown:
        raise DossierInvalid(field, f"unknown keys: {sorted(unknown)}")


@dataclass(frozen=True, slots=True)
class Source:
    """One rung of the source ladder."""

    id: str
    kind: str
    locator: str
    excerpt: str
    rank: int | None = None
    status: str = "used"
    covers: tuple[str, ...] = ()
    note: str | None = None
    sha256: str | None = None


@dataclass(frozen=True, slots=True)
class RungOutcome:
    """A higher rung's outcome, recorded so a lower rung's use is visible."""

    rung: str
    outcome: str
    note: str | None = None


@dataclass(frozen=True, slots=True)
class Claim:
    """One supplied statement, with the words it was read from."""

    id: str
    value: Any
    quote: str
    source: str
    unit: str | None = None
    rung: str | None = None
    rungs_above: tuple[RungOutcome, ...] = ()


@dataclass(frozen=True, slots=True)
class Binding:
    """Exactly one of: a claim, a licensed rule, an absence, or a conflict."""

    claim: str | None = None
    rule: str | None = None
    basis: Mapping[str, Any] | None = None
    absent: bool = False
    conflicted: bool = False
    note: str | None = None


@dataclass(frozen=True, slots=True)
class InputBinding:
    name: str
    binding: Binding


@dataclass(frozen=True, slots=True)
class Target:
    id: str
    inputs: tuple[InputBinding, ...]


@dataclass(frozen=True, slots=True)
class Premise:
    statement: str
    verdict: str
    basis: str | None = None


@dataclass(frozen=True, slots=True)
class Audit:
    reviewer: str
    verdict: str
    notes: str | None = None


@dataclass(frozen=True, slots=True)
class Dossier:
    id: str
    sources: tuple[Source, ...]
    claims: tuple[Claim, ...]
    targets: tuple[Target, ...]
    premises: tuple[Premise, ...] = ()
    audit: Audit | None = None

    def source(self, source_id: str) -> Source:
        for source in self.sources:
            if source.id == source_id:
                return source
        raise DossierInvalid("sources", f"no source {source_id!r}")

    def claim(self, claim_id: str) -> Claim:
        for claim in self.claims:
            if claim.id == claim_id:
                return claim
        raise DossierInvalid("claims", f"no claim {claim_id!r}")


def _source(raw: Any, index: int) -> Source:
    field = f"sources[{index}]"
    document = _mapping(raw, field)
    _only_keys(
        document,
        {"id", "kind", "locator", "excerpt", "sha256", "rank", "status", "covers", "note"},
        field,
    )
    kind = _text(document.get("kind"), f"{field}.kind")
    if kind not in _KINDS:
        raise DossierInvalid(f"{field}.kind", f"expected one of {sorted(_KINDS)}")
    status = str(document.get("status", "used"))
    if status not in _STATUSES:
        raise DossierInvalid(f"{field}.status", f"expected one of {sorted(_STATUSES)}")
    rank = document.get("rank")
    if rank is not None and (not isinstance(rank, int) or isinstance(rank, bool) or rank < 1):
        raise DossierInvalid(f"{field}.rank", "expected a positive integer")
    covers = tuple(
        _text(item, f"{field}.covers[{index}]")
        for index, item in enumerate(_sequence(document.get("covers", ()), f"{field}.covers"))
    )
    sha = document.get("sha256")
    if sha is not None and (not isinstance(sha, str) or len(sha) != 64):
        raise DossierInvalid(f"{field}.sha256", "expected a 64-character hex digest")
    return Source(
        id=_text(document.get("id"), f"{field}.id"),
        kind=kind,
        locator=_text(document.get("locator"), f"{field}.locator"),
        excerpt=_text(document.get("excerpt"), f"{field}.excerpt"),
        rank=rank,
        status=status,
        covers=covers,
        note=_text_or_none(document.get("note"), f"{field}.note"),
        sha256=sha,
    )


def _rung(raw: Any, field: str) -> RungOutcome:
    document = _mapping(raw, field)
    _only_keys(document, {"rung", "outcome", "note"}, field)
    outcome = _text(document.get("outcome"), f"{field}.outcome")
    if outcome not in _OUTCOMES:
        raise DossierInvalid(f"{field}.outcome", f"expected one of {sorted(_OUTCOMES)}")
    return RungOutcome(
        rung=_text(document.get("rung"), f"{field}.rung"),
        outcome=outcome,
        note=_text_or_none(document.get("note"), f"{field}.note"),
    )


def _claim(raw: Any, index: int) -> Claim:
    field = f"claims[{index}]"
    document = _mapping(raw, field)
    _only_keys(
        document,
        {"id", "value", "unit", "quote", "source", "rung", "rungs_above"},
        field,
    )
    if "value" not in document:
        raise DossierInvalid(f"{field}.value", "a claim needs a value")
    value = document["value"]
    if value is None:
        raise DossierInvalid(f"{field}.value", "a claim's value cannot be null")
    return Claim(
        id=_text(document.get("id"), f"{field}.id"),
        value=value,
        quote=_text(document.get("quote"), f"{field}.quote"),
        source=_text(document.get("source"), f"{field}.source"),
        unit=_text_or_none(document.get("unit"), f"{field}.unit"),
        rung=_text_or_none(document.get("rung"), f"{field}.rung"),
        rungs_above=tuple(
            _rung(item, f"{field}.rungs_above[{position}]")
            for position, item in enumerate(
                _sequence(document.get("rungs_above", ()), f"{field}.rungs_above")
            )
        ),
    )


def _binding(raw: Mapping[str, Any], field: str) -> Binding:
    claim = _text_or_none(raw.get("claim"), f"{field}.claim")
    rule = _text_or_none(raw.get("rule"), f"{field}.rule")
    basis = raw.get("basis")
    absent = _flag(raw["absent"], f"{field}.absent") if "absent" in raw else False
    conflicted = _flag(raw["conflicted"], f"{field}.conflicted") if "conflicted" in raw else False
    note = _text_or_none(raw.get("note"), f"{field}.note")
    if basis is not None and not isinstance(basis, Mapping):
        raise DossierInvalid(f"{field}.basis", "expected an object")
    chosen = sum(value is not None for value in (claim, rule)) + int(absent) + int(conflicted)
    if chosen != 1:
        raise DossierInvalid(field, "a binding is one of claim, rule, absent or conflicted")
    if rule is not None and basis is None:
        raise DossierInvalid(f"{field}.basis", "a rule binding needs its basis")
    if rule is None and basis is not None:
        raise DossierInvalid(f"{field}.basis", "a basis belongs to a rule binding")
    return Binding(
        claim=claim, rule=rule, basis=basis, absent=absent, conflicted=conflicted, note=note
    )


def _input(raw: Any, index: int) -> InputBinding:
    field = f"inputs[{index}]"
    document = _mapping(raw, field)
    _only_keys(
        document,
        {"name", "claim", "rule", "basis", "absent", "conflicted", "note"},
        field,
    )
    return InputBinding(
        name=_text(document.get("name"), f"{field}.name"),
        binding=_binding(document, field),
    )


def _target(raw: Any, index: int) -> Target:
    field = f"targets[{index}]"
    document = _mapping(raw, field)
    _only_keys(document, {"id", "inputs"}, field)
    inputs = tuple(
        _input(item, position)
        for position, item in enumerate(_sequence(document.get("inputs"), f"{field}.inputs"))
    )
    if not inputs:
        raise DossierInvalid(f"{field}.inputs", "a target needs at least one input")
    names = [item.name for item in inputs]
    if len(set(names)) != len(names):
        raise DossierInvalid(f"{field}.inputs", "two bindings name the same input")
    return Target(id=_text(document.get("id"), f"{field}.id"), inputs=inputs)


def _premise(raw: Any, index: int) -> Premise:
    field = f"premises[{index}]"
    document = _mapping(raw, field)
    _only_keys(document, {"statement", "verdict", "basis"}, field)
    verdict = _text(document.get("verdict"), f"{field}.verdict")
    if verdict not in _PREMISE_VERDICTS:
        raise DossierInvalid(f"{field}.verdict", f"expected one of {sorted(_PREMISE_VERDICTS)}")
    return Premise(
        statement=_text(document.get("statement"), f"{field}.statement"),
        verdict=verdict,
        basis=_text_or_none(document.get("basis"), f"{field}.basis"),
    )


def _audit(raw: Any) -> Audit:
    document = _mapping(raw, "audit")
    _only_keys(document, {"reviewer", "verdict", "notes"}, "audit")
    verdict = _text(document.get("verdict"), "audit.verdict")
    if verdict not in _AUDIT_VERDICTS:
        raise DossierInvalid("audit.verdict", f"expected one of {sorted(_AUDIT_VERDICTS)}")
    return Audit(
        reviewer=_text(document.get("reviewer"), "audit.reviewer"),
        verdict=verdict,
        notes=_text_or_none(document.get("notes"), "audit.notes"),
    )


def from_document(document: Mapping[str, Any]) -> Dossier:
    """Parse and validate one authored dossier."""
    _only_keys(
        document,
        {"schema", "id", "sources", "claims", "targets", "premises", "audit"},
        "dossier",
    )
    version = document.get("schema", SCHEMA)
    if version != SCHEMA:
        raise DossierInvalid("schema", f"expected {SCHEMA!r}, got {version!r}")
    sources = tuple(
        _source(item, index)
        for index, item in enumerate(_sequence(document.get("sources"), "sources"))
    )
    claims = tuple(
        _claim(item, index)
        for index, item in enumerate(_sequence(document.get("claims"), "claims"))
    )
    targets = tuple(
        _target(item, index)
        for index, item in enumerate(_sequence(document.get("targets"), "targets"))
    )
    if not sources:
        raise DossierInvalid("sources", "a dossier needs at least one source")
    if not claims:
        raise DossierInvalid("claims", "a dossier needs at least one claim")
    if not targets:
        raise DossierInvalid("targets", "a dossier needs at least one target")

    source_ids = [source.id for source in sources]
    if len(set(source_ids)) != len(source_ids):
        raise DossierInvalid("sources", "two sources share an id")
    claim_ids = [claim.id for claim in claims]
    if len(set(claim_ids)) != len(claim_ids):
        raise DossierInvalid("claims", "two claims share an id")
    target_ids = [target.id for target in targets]
    if len(set(target_ids)) != len(target_ids):
        raise DossierInvalid("targets", "two targets share an id")
    for claim in claims:
        if claim.source not in source_ids:
            raise DossierInvalid("claims", f"{claim.id} cites unknown source {claim.source!r}")
    for target in targets:
        for item in target.inputs:
            if item.binding.claim is not None and item.binding.claim not in claim_ids:
                raise DossierInvalid(
                    "targets", f"{target.id}.{item.name} cites unknown claim {item.binding.claim!r}"
                )
    premises = tuple(
        _premise(item, index)
        for index, item in enumerate(_sequence(document.get("premises", ()), "premises"))
    )
    audit = None if document.get("audit") is None else _audit(document["audit"])
    return Dossier(
        id=_text(document.get("id"), "id"),
        sources=sources,
        claims=claims,
        targets=targets,
        premises=premises,
        audit=audit,
    )


def load_path(path: str | Path) -> Dossier:
    """Read a TOML or JSON dossier from disk."""
    resolved = Path(path)
    text = resolved.read_text(encoding="utf-8")
    document = tomllib.loads(text) if resolved.suffix.lower() == ".toml" else json.loads(text)
    return from_document(_mapping(document, str(resolved)))


def canonical(document: Mapping[str, Any]) -> str:
    """The dossier's canonical JSON, for the run's evidence digest."""
    return json.dumps(document, sort_keys=True, separators=(",", ":"), allow_nan=False)
