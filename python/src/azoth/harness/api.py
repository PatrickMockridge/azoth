"""The calculation bridge's calls: catalogue, spec, review, run and version."""

from __future__ import annotations

import hashlib
import inspect
import platform
from collections.abc import Mapping
from pathlib import Path
from typing import Any, cast

import azoth
from azoth._dispatch import Backend, available, fluid_inputs, select, use_backend
from azoth._models_gen import MODELS
from azoth._registry_gen import CALCS
from azoth.core.errors import AzothError
from azoth.core.serialise import to_dict
from azoth.harness.catalogue import SCHEMA, entry_for
from azoth.harness.errors import (
    DossierInvalid,
    DossierRequired,
    HarnessError,
    InputsMismatch,
    IntakeRefused,
    TargetMismatch,
    UndefinedInput,
)
from azoth.harness.inputs import decode_inputs, function_for, kwargs_for, model_kwargs


def _card(path: str | Path) -> Any:
    """A keycard under the working directory, and nothing outside it."""
    resolved = Path(path).expanduser().resolve()
    working = Path.cwd().resolve()
    if not resolved.is_relative_to(working):
        raise UndefinedInput(
            f"keycard {resolved} is outside the working directory {working}", field="keycard"
        )
    try:
        return azoth.keycard.load(resolved)
    except AzothError as exc:
        raise UndefinedInput(str(exc), field="keycard") from exc


def _reviewed(
    dossier: Mapping[str, Any],
    *,
    keycard: str | Path | None,
    target_id: str | None,
) -> tuple[Any, list[Any], dict[str, Any], str]:
    """Check a dossier, raise the bridge's error for a malformed one, and return its block."""
    from azoth.intake.check import document as intake_document
    from azoth.intake.check import review as check_dossier
    from azoth.intake.dossier import DossierInvalid as IntakeDossierInvalid
    from azoth.intake.dossier import canonical

    try:
        checked = check_dossier(dossier, keycard=keycard)
    except IntakeDossierInvalid as exc:
        raise DossierInvalid(str(exc), field=getattr(exc, "field", None)) from exc
    except AzothError as exc:
        raise DossierInvalid(str(exc)) from exc
    matches = [item for item in checked.targets if target_id is None or item.id == target_id]
    if target_id is not None and len(matches) != 1:
        raise TargetMismatch(
            f"{target_id} names {len(matches)} targets in the dossier; a run needs exactly one",
            field="id",
        )
    block = intake_document(checked, target=target_id)
    digest = hashlib.sha256(canonical(dossier).encode("utf-8")).hexdigest()
    return checked, matches, block, digest


def review(
    dossier: Mapping[str, Any],
    *,
    keycard: str | Path | None = None,
    target: str | None = None,
) -> dict[str, Any]:
    """Check an intake dossier. A refused verdict is a successful review."""
    _, _, block, digest = _reviewed(dossier, keycard=keycard, target_id=target)
    return {
        "schema": SCHEMA,
        "kind": "review",
        "id": target,
        "dossier": dict(dossier),
        "dossier_sha256": digest,
        **block,
    }


def run(
    calc_id: str,
    inputs: Mapping[str, Any] | None = None,
    *,
    keycard: str | Path | None = None,
    backend: str = "auto",
    dossier: Mapping[str, Any] | None = None,
    require_dossier: bool = False,
) -> dict[str, Any]:
    """Run one registered id and return its result with provenance.

    A dossier replaces ``inputs``: the values it constructed are the call's
    arguments, and the result envelope gains a sibling ``intake`` block. The
    block is disclosure, not a different result type.
    """
    if require_dossier and dossier is None:
        raise DossierRequired(
            f"{calc_id} requires an intake dossier in this configuration", field="dossier"
        )
    intake_block: dict[str, Any] | None = None
    if dossier is not None:
        if inputs is not None:
            raise InputsMismatch("a dossier replaces inputs; pass one or the other", field="inputs")
        _, matches, block, digest = _reviewed(dossier, keycard=keycard, target_id=calc_id)
        target = matches[0]
        if require_dossier and target.verdict == "refused":
            codes = (
                ", ".join(gap.code for gap in target.gaps if gap.class_ == "hard")
                or "no constructible input"
            )
            raise IntakeRefused(f"{calc_id} is refused by its dossier: {codes}", field="intake")
        inputs = target.inputs
        intake_block = {
            "schema": "azoth.intake/1",
            "target": calc_id,
            "dossier_sha256": digest,
            "enforcement": "strict" if require_dossier else "advisory",
            **block,
        }
    if inputs is None:
        raise UndefinedInput(f"{calc_id}: inputs are required without a dossier", field="inputs")
    kind, entry = entry_for(calc_id)
    if backend not in ("auto", "python", "rust"):
        raise UndefinedInput("backend must be auto, python or rust", field="backend")
    decoded = decode_inputs(entry, inputs)
    card = None if keycard is None else _card(keycard)
    function = function_for(entry)

    if kind == "model":
        if card is not None and not fluid_inputs(entry):
            raise UndefinedInput(
                f"{calc_id} names no fluid, so it has nowhere to apply a keycard",
                field="keycard",
            )
        kwargs = model_kwargs(entry, decoded, card=card)
    else:
        kwargs = kwargs_for(entry, decoded)
        if card is not None:
            if "card" not in inspect.signature(function).parameters:
                raise UndefinedInput(
                    f"{calc_id} does not take a keycard; it is a model-level input",
                    field="keycard",
                )
            kwargs["card"] = card

    try:
        if backend == "auto":
            result = function(**kwargs)
            selected = select()
        else:
            with use_backend(cast(Backend, backend)):
                result = function(**kwargs)
                selected = select()
    except AzothError:
        raise
    except RuntimeError as exc:
        raise HarnessError(str(exc)) from exc

    document = result.to_dict() if hasattr(result, "to_dict") else to_dict(result)
    envelope: dict[str, Any] = {
        "schema": SCHEMA,
        "kind": "result",
        "id": calc_id,
        "backend": selected,
        "result": document,
    }
    if intake_block is not None:
        envelope["intake"] = intake_block
    return envelope


def version_info() -> dict[str, Any]:
    """The bridge's version and what it can reach."""
    return {
        "schema": SCHEMA,
        "kind": "version",
        "azoth_version": azoth.__version__,
        "calcs": len(CALCS),
        "models": len(MODELS),
        "backend": select(),
        "backends": sorted(available()),
        "python": platform.python_version(),
    }
