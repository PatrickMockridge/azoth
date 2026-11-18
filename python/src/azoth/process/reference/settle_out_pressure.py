"""``process.settle_out_pressure`` - the connected-volume settle-out balance.

As NeqSim's ``SettleOutPressureAnalyzer.analyze`` computes it. Spec:
``specs/models/process/settle_out_pressure.toml``

The Python twin of ``crates/azoth-process/src/models/settle_out_pressure.rs``: the no-gas
inventory, the settle temperature and pressure, and the rating verdict.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    SettleOutCompartmentResult,
    SettleOutPressureResult,
    SettleOutVerdict,
)
from azoth.core.units import Q, from_si, input_records_to_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.settle_out_pressure"

#: The class's gas constant, J/(mol K).
R_GAS = 8.314462618


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    pressure: float
    temperature: float
    total_moles: float
    total_volume: float
    max_pressure: float
    min_pressure: float
    margin: float | None
    exceeded: bool
    rows: list[dict[str, Any]]
    verdict: SettleOutVerdict


def _require_finite(name: str, value: float) -> None:
    """Refuse a value the arithmetic cannot carry to a published number."""
    if not math.isfinite(value):
        raise InvalidInputError(
            name, "a non-finite value is not a number this calculation publishes"
        )


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _evaluate(
    compartments: Sequence[dict[str, object]],
    settle_out_temperature: Q | None = None,
    protected_pressure_rating: Q | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []
    apply_checks(checks_for(spec).on_input, lambda _: None, warnings)

    rows: list[dict[str, Any]] = input_records_to_si(spec, "compartments", compartments)
    if len(rows) < 2:
        raise InvalidInputError(
            "compartments",
            "at least two connected gas volumes are required for a settle-out balance",
        )
    for row in rows:
        for field in ("volume", "pressure", "temperature"):
            value = float(row[field])
            _require_finite(f"compartments.{field}", value)
            if value <= 0.0:
                raise InvalidInputError(
                    f"compartments.{field}", "a compartment value must be positive"
                )
    stated_temperature = (
        None
        if settle_out_temperature is None
        else input_to_si(spec, "settle_out_temperature", settle_out_temperature)
    )
    rating = (
        None
        if protected_pressure_rating is None
        else input_to_si(spec, "protected_pressure_rating", protected_pressure_rating)
    )
    if stated_temperature is not None:
        _require_finite("settle_out_temperature", stated_temperature)
        if stated_temperature <= 0.0:
            raise InvalidInputError(
                "settle_out_temperature", "the settle-out temperature must be positive"
            )
    if rating is not None:
        _require_finite("protected_pressure_rating", rating)
        if rating <= 0.0:
            raise InvalidInputError(
                "protected_pressure_rating", "the protected rating must be positive"
            )

    total_moles = 0.0
    weighted_temperature = 0.0
    total_volume = 0.0
    maximum_pressure = -math.inf
    minimum_pressure = math.inf
    for row in rows:
        volume = float(row["volume"])
        pressure = float(row["pressure"])
        temperature = float(row["temperature"])
        inventory = pressure * volume / (R_GAS * temperature)
        row["moles"] = inventory
        row["z_factor"] = 1.0
        total_moles += inventory
        weighted_temperature += inventory * temperature
        total_volume += volume
        maximum_pressure = max(maximum_pressure, pressure)
        minimum_pressure = min(minimum_pressure, pressure)

    settled_temperature = (
        weighted_temperature / total_moles if stated_temperature is None else stated_temperature
    )
    settled_pressure = total_moles * R_GAS * settled_temperature / total_volume

    if rating is None:
        margin = None
        exceeded = False
        verdict = SettleOutVerdict.NO_RATING
    else:
        margin = rating - settled_pressure
        exceeded = settled_pressure > rating
        verdict = SettleOutVerdict.EXCEEDS_RATING if exceeded else SettleOutVerdict.WITHIN_RATING

    return (
        _Solution(
            pressure=settled_pressure,
            temperature=settled_temperature,
            total_moles=total_moles,
            total_volume=total_volume,
            max_pressure=maximum_pressure,
            min_pressure=minimum_pressure,
            margin=margin,
            exceeded=exceeded,
            rows=rows,
            verdict=verdict,
        ),
        warnings,
    )


def settle_out_pressure(
    compartments: Sequence[dict[str, object]],
    settle_out_temperature: Q | None = None,
    protected_pressure_rating: Q | None = None,
) -> SettleOutPressureResult:
    """The settle-out pressure of connected gas volumes and the rating verdict.

    Each compartment's inventory is ``p V/(R T)`` with ``Z = 1``, the class's no-gas basis;
    the settle temperature is the mole-weighted mean unless stated, and the pressure is
    ``N R T_s / sum V``. **At least two compartments are required.** The real-gas arm needs a
    fluid and is owed; the class's fixed-point Z iteration is a no-op at this value.

    Args:
        compartments: the connected gas volumes, each with a name, volume, pressure and
            temperature.
        settle_out_temperature: the settled temperature; absent is the mole-weighted mean.
        protected_pressure_rating: the protected rating; absent publishes no verdict.

    Returns:
        The settle-out pressure and temperature, the summed inventory and volume, the
        compartment extremes, the rating margin and the per-compartment breakdown.

    Raises:
        InvalidInputError: for fewer than two compartments, a non-positive compartment
            volume, pressure or temperature, a non-positive stated temperature or rating, or
            a non-finite input.
    """
    solution, warnings = _evaluate(compartments, settle_out_temperature, protected_pressure_rating)
    return SettleOutPressureResult(
        settle_out_pressure=from_si(solution.pressure, "Pa"),
        settle_out_temperature=from_si(solution.temperature, "K"),
        settle_out_z_factor=1.0,
        total_moles=from_si(solution.total_moles, "mol"),
        total_volume=from_si(solution.total_volume, "m**3"),
        max_compartment_pressure=from_si(solution.max_pressure, "Pa"),
        min_compartment_pressure=from_si(solution.min_pressure, "Pa"),
        margin_to_rating=(None if solution.margin is None else from_si(solution.margin, "Pa")),
        exceeds_rating=solution.exceeded,
        verdict=solution.verdict,
        compartments=tuple(
            SettleOutCompartmentResult(
                name=str(row["name"]),
                volume=from_si(float(row["volume"]), "m**3"),
                pressure=from_si(float(row["pressure"]), "Pa"),
                temperature=from_si(float(row["temperature"]), "K"),
                z_factor=1.0,
                moles=from_si(float(row["moles"]), "mol"),
            )
            for row in solution.rows
        ),
        warnings=tuple(warnings),
    )
