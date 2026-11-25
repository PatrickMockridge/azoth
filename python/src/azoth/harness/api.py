"""The calculation bridge's four calls: catalogue, spec, run and version."""

from __future__ import annotations

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
from azoth.harness.errors import HarnessError, UndefinedInput
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


def run(
    calc_id: str,
    inputs: Mapping[str, Any],
    *,
    keycard: str | Path | None = None,
    backend: str = "auto",
) -> dict[str, Any]:
    """Run one registered id and return its result with provenance."""
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
    return {
        "schema": SCHEMA,
        "kind": "result",
        "id": calc_id,
        "backend": selected,
        "result": document,
    }


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
