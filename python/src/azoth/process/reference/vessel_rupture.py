"""``process.vessel_rupture`` - the thin-wall vessel rupture crossover.

As NeqSim's ``VesselRuptureAnalyzer`` and ``MaterialStrengthCurve`` compute it, with
``VesselRuptureCalculator.vonMisesStress``. Spec:
``specs/models/process/vessel_rupture.toml``

The Python twin of ``crates/azoth-process/src/models/vessel_rupture.rs``: the thin-wall
stress, the retained-strength curve, the first margin at or below zero and the linear
crossover.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import VesselRuptureResult, VesselRuptureStep
from azoth.core.units import Q, from_si, input_records_to_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.vessel_rupture"

_PSI_TO_PA = 6894.76
_RETAINED_TEMPERATURES = (293.15, 373.15, 473.15, 573.15, 673.15, 773.15, 873.15, 973.15, 1073.15)
_RETAINED_FACTORS = (1.00, 0.95, 0.85, 0.72, 0.52, 0.32, 0.16, 0.07, 0.03)


class _Solution(NamedTuple):
    """The summary and history, in SI."""

    ruptured: bool
    rupture_time: float | None
    rupture_stress: float | None
    rupture_allowable_stress: float | None
    rupture_metal_temperature: float | None
    min_margin: float
    min_margin_time: float
    material_name: str
    ambient_yield_strength: float
    ambient_tensile_strength: float
    history: list[tuple[float, float, float, float]]


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


def _normalize_grade(grade: str) -> str:
    """The class's ``Api5LGrade.normalize``."""
    if not grade.strip():
        raise InvalidInputError("grade", "Pipe material grade must not be empty")
    key = grade.strip().upper()
    if key.startswith("GRADE "):
        key = key[6:].strip()
    metric = (
        "L175",
        "L210",
        "L245",
        "L290",
        "L320",
        "L360",
        "L390",
        "L415",
        "L450",
        "L485",
        "L555",
        "L625",
        "L690",
        "L830",
    )
    imperial = (
        "A25",
        "A",
        "B",
        "X42",
        "X46",
        "X52",
        "X56",
        "X60",
        "X65",
        "X70",
        "X80",
        "X90",
        "X100",
        "X120",
    )
    if key in metric:
        return imperial[metric.index(key)]
    return key


def _api5l_strengths(grade: str) -> tuple[float, float]:
    """The class's built-in API 5L fallback strengths, Pa."""

    def psi(yield_psi: float, tensile_psi: float) -> tuple[float, float]:
        return (yield_psi * _PSI_TO_PA, tensile_psi * _PSI_TO_PA)

    table: dict[str, tuple[float, float]] = {
        "B": psi(35000.0, 60000.0),
        "GRADE B": psi(35000.0, 60000.0),
        "X42": psi(42000.0, 60000.0),
        "X52": psi(52000.0, 66000.0),
        "X60": psi(60000.0, 75000.0),
        "X65": psi(65000.0, 77000.0),
        "X70": psi(70000.0, 82000.0),
        "X80": psi(80000.0, 90000.0),
        "A25": psi(25000.0, 45000.0),
        "A": psi(30000.0, 48000.0),
        "X46": psi(46000.0, 63000.0),
        "X56": psi(56000.0, 71000.0),
        "X90": (620.0e6, 695.0e6),
        "X100": psi(100000.0, 110000.0),
        "X120": psi(120000.0, 130000.0),
    }
    if grade not in table:
        raise InvalidInputError("grade", f"No API 5L material strength data for grade {grade}")
    return table[grade]


def _retained_factor(temperature_k: float) -> float:
    """The class's endpoint-clamped retained-strength interpolation."""
    if temperature_k <= _RETAINED_TEMPERATURES[0]:
        return _RETAINED_FACTORS[0]
    for index in range(1, len(_RETAINED_TEMPERATURES)):
        if temperature_k <= _RETAINED_TEMPERATURES[index]:
            fraction = (temperature_k - _RETAINED_TEMPERATURES[index - 1]) / (
                _RETAINED_TEMPERATURES[index] - _RETAINED_TEMPERATURES[index - 1]
            )
            return _RETAINED_FACTORS[index - 1] + fraction * (
                _RETAINED_FACTORS[index] - _RETAINED_FACTORS[index - 1]
            )
    return _RETAINED_FACTORS[-1]


def _von_mises_pa(pressure_pa: float, radius_m: float, wall_m: float) -> float:
    """The class's thin-wall von Mises stress, Pa."""
    hoop = pressure_pa * radius_m / wall_m
    axial = pressure_pa * radius_m / (2.0 * wall_m)
    return math.sqrt(hoop * hoop + axial * axial - hoop * axial)


def _evaluate(
    inner_radius: Q,
    wall_thickness: Q,
    material: str,
    history: list[dict[str, object]],
    yield_strength: Q | None = None,
    tensile_strength: Q | None = None,
    grade: str | None = None,
    tensile_strength_factor: float | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    radius = input_to_si(spec, "inner_radius", inner_radius)
    wall = input_to_si(spec, "wall_thickness", wall_thickness)
    yield_value = (
        None if yield_strength is None else input_to_si(spec, "yield_strength", yield_strength)
    )
    tensile_value = (
        None
        if tensile_strength is None
        else input_to_si(spec, "tensile_strength", tensile_strength)
    )
    states: list[dict[str, float]] = input_records_to_si(spec, "history", history)
    apply_checks(
        checks.on_input,
        {
            "inner_radius": radius,
            "wall_thickness": wall,
            "yield_strength": yield_value,
            "tensile_strength": tensile_value,
            "tensile_strength_factor": tensile_strength_factor,
        }.get,
        warnings,
    )

    if radius <= 0.0 or wall <= 0.0:
        raise InvalidInputError(
            "inner_radius", "the class's constructor refuses a non-positive radius or wall"
        )
    _require_finite("inner_radius", radius)
    _require_finite("wall_thickness", wall)
    factor = 1.0 if tensile_strength_factor is None else tensile_strength_factor
    _require_finite("tensile_strength_factor", factor)
    if not 0.0 < factor <= 1.0:
        raise InvalidInputError(
            "tensile_strength_factor", "the class's setter refuses a factor outside (0, 1]"
        )
    if not states:
        raise InvalidInputError("history", "the class refuses an empty history")
    previous = -math.inf
    for state in states:
        time_value = float(state["time"])
        pressure = float(state["internal_pressure"])
        temperature = float(state["metal_temperature"])
        _require_finite("history.time", time_value)
        _require_finite("history.internal_pressure", pressure)
        _require_finite("history.metal_temperature", temperature)
        if time_value < 0.0:
            raise InvalidInputError("history.time", "a negative time is not an instant")
        if time_value <= previous:
            raise InvalidInputError(
                "history.time",
                "the class's javadoc requires strictly increasing times; the port refuses them",
            )
        previous = time_value
        if pressure < 0.0:
            raise InvalidInputError(
                "history.internal_pressure",
                "the class's von Mises helper refuses a negative pressure",
            )
        if temperature <= 0.0:
            raise InvalidInputError(
                "history.metal_temperature",
                "a non-positive absolute temperature is not a temperature",
            )

    if material == "carbon_steel":
        if grade is not None:
            raise InvalidInputError(
                "grade", "the carbon-steel branch takes its strengths, not a grade"
            )
        if yield_value is None or tensile_value is None:
            raise InvalidInputError(
                "yield_strength",
                "the carbon-steel branch requires its ambient yield and tensile strengths",
            )
        if yield_value <= 0.0 or tensile_value <= 0.0:
            raise InvalidInputError(
                "yield_strength",
                "the class's material constructor refuses a non-positive strength",
            )
        material_name = "CS"
        ambient_yield = yield_value
        ambient_tensile = tensile_value
    elif material == "api_5l":
        if yield_value is not None or tensile_value is not None:
            raise InvalidInputError(
                "grade", "the API 5L branch takes a grade, not explicit strengths"
            )
        if grade is None:
            raise InvalidInputError("grade", "the API 5L branch requires a grade")
        normalized = _normalize_grade(grade)
        ambient_yield, ambient_tensile = _api5l_strengths(normalized)
        material_name = f"API 5L {normalized}"
    else:
        raise InvalidInputError("material", f"`{material}` is not `carbon_steel` or `api_5l`")

    rows: list[tuple[float, float, float, float]] = []
    ruptured = False
    rupture_time: float | None = None
    rupture_stress: float | None = None
    rupture_allowable: float | None = None
    rupture_temperature: float | None = None
    min_margin = math.inf
    min_margin_time = float(states[0]["time"])
    previous_margin = math.nan
    previous_time = math.nan

    for state in states:
        time_value = float(state["time"])
        pressure = float(state["internal_pressure"])
        temperature = float(state["metal_temperature"])
        stress = _von_mises_pa(pressure, radius, wall)
        allowable = ambient_tensile * _retained_factor(temperature) * factor
        margin = allowable - stress
        rows.append((time_value, stress, allowable, margin))
        if margin < min_margin:
            min_margin = margin
            min_margin_time = time_value
        if not ruptured and margin <= 0.0:
            ruptured = True
            if previous_margin > 0.0 and margin != previous_margin:
                fraction = previous_margin / (previous_margin - margin)
                rupture_time = previous_time + fraction * (time_value - previous_time)
            else:
                rupture_time = time_value
            rupture_stress = stress
            rupture_allowable = allowable
            rupture_temperature = temperature
        previous_margin = margin
        previous_time = time_value

    return (
        _Solution(
            ruptured,
            rupture_time,
            rupture_stress,
            rupture_allowable,
            rupture_temperature,
            min_margin,
            min_margin_time,
            material_name,
            ambient_yield,
            ambient_tensile,
            rows,
        ),
        warnings,
    )


def vessel_rupture(
    inner_radius: Q,
    wall_thickness: Q,
    material: str,
    history: list[dict[str, object]],
    yield_strength: Q | None = None,
    tensile_strength: Q | None = None,
    grade: str | None = None,
    tensile_strength_factor: float | None = None,
) -> VesselRuptureResult:
    """The thin-wall vessel rupture crossover.

    Args:
        inner_radius: the vessel's inner radius.
        wall_thickness: the wall thickness.
        material: ``carbon_steel`` or ``api_5l``.
        history: the ``{time, internal_pressure, metal_temperature}`` states.
        yield_strength: the carbon-steel ambient yield strength.
        tensile_strength: the carbon-steel ambient tensile strength.
        grade: the API 5L grade.
        tensile_strength_factor: the fraction of the derated tensile accepted; absent is 1.

    Returns:
        The rupture verdict and state, the minimum margin and the per-state history.

    Raises:
        InvalidInputError: for a non-positive radius, wall or strength, a missing strength
            or grade, an unknown grade, a contradictory material group, an empty or
            non-increasing history, a negative pressure, a factor outside ``(0, 1]``, or a
            non-finite value.
    """
    solution, warnings = _evaluate(
        inner_radius,
        wall_thickness,
        material,
        history,
        yield_strength,
        tensile_strength,
        grade,
        tensile_strength_factor,
    )
    return VesselRuptureResult(
        ruptured=solution.ruptured,
        rupture_time=(
            None if solution.rupture_time is None else from_si(solution.rupture_time, "s")
        ),
        rupture_stress=(
            None if solution.rupture_stress is None else from_si(solution.rupture_stress, "Pa")
        ),
        rupture_allowable_stress=(
            None
            if solution.rupture_allowable_stress is None
            else from_si(solution.rupture_allowable_stress, "Pa")
        ),
        rupture_metal_temperature=(
            None
            if solution.rupture_metal_temperature is None
            else from_si(solution.rupture_metal_temperature, "K")
        ),
        min_margin=from_si(solution.min_margin, "Pa"),
        min_margin_time=from_si(solution.min_margin_time, "s"),
        material_name=solution.material_name,
        ambient_yield_strength=from_si(solution.ambient_yield_strength, "MPa"),
        ambient_tensile_strength=from_si(solution.ambient_tensile_strength, "MPa"),
        history=tuple(
            VesselRuptureStep(
                time=from_si(row[0], "s"),
                von_mises_stress=from_si(row[1], "Pa"),
                allowable_stress=from_si(row[2], "Pa"),
                margin=from_si(row[3], "Pa"),
            )
            for row in solution.history
        ),
        warnings=tuple(warnings),
    )
