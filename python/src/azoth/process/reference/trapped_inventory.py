"""``process.trapped_inventory`` - the trapped gas and liquid inventory.

The Python twin of ``crates/azoth-process/src/models/trapped_inventory.rs``. The port
source is ``TrappedInventoryCalculator.calculate``: the TP flash's phase densities, the
documented equipment and pipe volumes, and the gas and liquid shares.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import TrappedInventoryResult, TrappedInventorySegmentResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning, WarningCode
from azoth.eos import components as _components
from azoth.process.reference import _fluid_state

MODEL_ID = "process.trapped_inventory"

_KINDS = ("equipment", "pipe")


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _si(value: Any, unit: str) -> float | None:
    """A value the caller may state as a number (already SI) or a quantity."""
    if value is None:
        return None
    if hasattr(value, "to") or hasattr(value, "magnitude"):
        return float(value.to(unit).magnitude)
    return float(value)


def trapped_inventory(
    components: list[str],
    z: list[float],
    pressure: Q,
    temperature: Q,
    segments: list[dict[str, object]],
    eos: str | None = None,
    multiphase_check: bool | None = None,
    fallback_liquid_density: Q | None = None,
    segment_evidence: list[dict[str, object]] | None = None,
) -> TrappedInventoryResult:
    """The trapped gas and liquid inventory.

    Raises:
        InvalidInputError: for an empty segment list, a non-positive pressure, temperature,
            volume or fallback density, a fill outside ``[0, 1]``, a wrong
            volume-and-geometry pair, or an evidence row naming no segment.
    """
    spec = _spec()
    warnings: list[Warning] = []
    pressure_value = input_to_si(spec, "pressure", pressure)
    temperature_value = input_to_si(spec, "temperature", temperature)
    fallback_value = (
        None
        if fallback_liquid_density is None
        else input_to_si(spec, "fallback_liquid_density", fallback_liquid_density)
    )
    apply_checks(
        checks_for(spec).on_input,
        {
            "pressure": pressure_value,
            "temperature": temperature_value,
            "fallback_liquid_density": fallback_value,
        }.get,
        warnings,
    )
    if not segments:
        raise InvalidInputError(
            "segments", "the class refuses an inventory with no segment"
        )
    fallback = 800.0 if fallback_value is None else fallback_value
    if not fallback > 0.0 or not math.isfinite(fallback):
        raise InvalidInputError("fallback_liquid_density", "a density is positive")

    evidence = segment_evidence or []
    segment_ids = {str(row.get("segment_id", "")) for row in segments}
    for row in evidence:
        segment_id = str(row.get("segment_id", ""))
        if segment_id not in segment_ids:
            raise InvalidInputError(
                "segment_evidence.segment_id", f"`{segment_id}` names no segment row"
            )

    volumes: list[float] = []
    for row in segments:
        segment_id = str(row.get("segment_id", ""))
        if not segment_id.strip():
            raise InvalidInputError(
                "segments.segment_id", "a segment identifier must not be empty"
            )
        kind = str(row.get("kind", ""))
        if kind not in _KINDS:
            raise InvalidInputError(
                "segments.kind", f"`{kind}` is not `equipment` or `pipe`"
            )
        fill = _si(row.get("liquid_fill_fraction"), "dimensionless")
        if fill is None or not 0.0 <= fill <= 1.0 or not math.isfinite(fill):
            raise InvalidInputError(
                "segments.liquid_fill_fraction",
                "the class refuses a fill fraction outside [0, 1]",
            )
        if kind == "equipment":
            if row.get("internal_diameter") is not None or row.get("length") is not None:
                raise InvalidInputError(
                    "segments",
                    "an equipment segment states its volume, not a pipe geometry",
                )
            volume = _si(row.get("volume"), "m**3")
            if volume is None:
                raise InvalidInputError(
                    "segments.volume", "an equipment segment requires its volume"
                )
            if not volume > 0.0 or not math.isfinite(volume):
                raise InvalidInputError("segments.volume", "a volume is positive")
            volumes.append(volume)
        else:
            if row.get("volume") is not None:
                raise InvalidInputError(
                    "segments", "a pipe states its geometry, not a volume"
                )
            diameter = _si(row.get("internal_diameter"), "m")
            length = _si(row.get("length"), "m")
            if diameter is None or length is None:
                raise InvalidInputError(
                    "segments", "a pipe segment requires its diameter and length"
                )
            if (
                not diameter > 0.0
                or not math.isfinite(diameter)
                or not length > 0.0
                or not math.isfinite(length)
            ):
                raise InvalidInputError(
                    "segments", "a pipe's diameter and length are positive"
                )
            volumes.append(math.pi * diameter * diameter * length / 4.0)

    mixture, ideal_gas = _components.mixture_of(
        [str(name) for name in components], eos="pr" if eos is None else eos
    )
    state = _fluid_state.flash(
        mixture,
        ideal_gas,
        temperature_value,
        pressure_value,
        [float(value) for value in z],
        bool(multiphase_check),
    )
    gas_density = state.gas_density()
    liquid_density = state.liquid_density(fallback)

    results: list[TrappedInventorySegmentResult] = []
    total_volume = 0.0
    total_gas_volume = 0.0
    total_liquid_volume = 0.0
    for row, volume in zip(segments, volumes, strict=True):
        segment_id = str(row.get("segment_id", ""))
        kind = str(row.get("kind", ""))
        fill = float(_si(row.get("liquid_fill_fraction"), "dimensionless") or 0.0)
        gas_volume = volume * (1.0 - fill)
        liquid_volume = volume * fill
        gas_mass = gas_volume * gas_density
        liquid_mass = liquid_volume * liquid_density
        results.append(
            TrappedInventorySegmentResult(
                segment_id=segment_id,
                kind=kind,
                total_volume=from_si(volume, "m**3"),
                gas_volume=from_si(gas_volume, "m**3"),
                liquid_volume=from_si(liquid_volume, "m**3"),
                gas_mass=from_si(gas_mass, "kg"),
                liquid_mass=from_si(liquid_mass, "kg"),
                total_mass=from_si(gas_mass + liquid_mass, "kg"),
                gas_density=from_si(gas_density, "kg/m**3"),
                liquid_density=from_si(liquid_density, "kg/m**3"),
                liquid_fill_fraction=fill,
                internal_diameter=(
                    None
                    if row.get("internal_diameter") is None
                    else from_si(float(_si(row.get("internal_diameter"), "m") or 0.0), "m")
                ),
                length=(
                    None
                    if row.get("length") is None
                    else from_si(float(_si(row.get("length"), "m") or 0.0), "m")
                ),
            )
        )
        total_volume += volume
        total_gas_volume += gas_volume
        total_liquid_volume += liquid_volume
        if not any(str(item.get("segment_id", "")) == segment_id for item in evidence):
            warnings.append(
                Warning(
                    WarningCode.RANGE_CHECK_SKIPPED,
                    f"Segment {segment_id} has no traceable document evidence.",
                    "segments",
                )
            )

    total_gas_mass = total_gas_volume * gas_density
    total_liquid_mass = total_liquid_volume * liquid_density
    if total_liquid_volume > 0.0 and liquid_density == fallback:
        warnings.append(
            Warning(
                WarningCode.FALLBACK_TO_INPUT,
                "No liquid phase found in representative fluid; fallback liquid density used.",
                "liquid_density",
            )
        )

    return TrappedInventoryResult(
        gas_density=from_si(gas_density, "kg/m**3"),
        liquid_density=from_si(liquid_density, "kg/m**3"),
        total_volume=from_si(total_volume, "m**3"),
        total_gas_volume=from_si(total_gas_volume, "m**3"),
        total_liquid_volume=from_si(total_liquid_volume, "m**3"),
        total_gas_mass=from_si(total_gas_mass, "kg"),
        total_liquid_mass=from_si(total_liquid_mass, "kg"),
        total_mass=from_si(total_gas_mass + total_liquid_mass, "kg"),
        segments=tuple(results),
        warnings=tuple(warnings),
    )
