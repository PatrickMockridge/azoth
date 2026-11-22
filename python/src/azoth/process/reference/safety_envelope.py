"""``process.safety_envelope`` - the operational safety envelope.

The Python twin of ``crates/azoth-process/src/models/safety_envelope.rs``. The port
source is ``SafetyEnvelopeCalculator``'s five ``calculate*Envelope`` methods, with
``SafetyEnvelope``'s interpolation, safe test and margin.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    EnvelopePoint,
    EnvelopeQuery,
    EnvelopeSummary,
    SafetyEnvelopeResult,
)
from azoth.core.units import Q, from_si, input_to_si, quantity
from azoth.core.warnings import Warning, WarningCode
from azoth.eos import components as _components
from azoth.eos.reference.hydrate_formation_temperature import (
    hydrate_formation_temperature,
)
from azoth.process.reference import _fluid_state
from azoth.process.reference._wax_appearance import wat_temperature

MODEL_ID = "process.safety_envelope"

_ATMOSPHERIC_BAR = 1.01325
_CO2_TRIPLE_BAR = 5.18
_CO2_TRIPLE_K = 216.55
_KINDS = ("hydrate", "wax", "co2_freezing", "mdmt", "phase_envelope")
_NAMES = {
    "hydrate": "Hydrate Formation",
    "wax": "Wax Appearance",
    "co2_freezing": "CO2 Freezing",
    "mdmt": "MDMT",
    "phase_envelope": "Phase Envelope",
}


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _scalar(spec: dict[str, Any], field: str, value: Any) -> float:
    """A scalar the caller may state as a number (already SI) or a quantity."""
    if hasattr(value, "to") or hasattr(value, "magnitude"):
        return input_to_si(spec, field, value)
    return float(value)


def _si(value: Any, unit: str) -> float | None:
    """A record value the caller may state as a number (already SI) or a quantity."""
    if value is None:
        return None
    if hasattr(value, "to") or hasattr(value, "magnitude"):
        return float(value.to(unit).magnitude)
    return float(value)


def _arm(
    kind: str,
    pressure_bar: list[float],
    temperature: Sequence[float | None],
    margin: list[float],
) -> dict[str, Any]:
    return {
        "envelope": kind,
        "pressure": pressure_bar,
        "temperature": temperature,
        "margin": margin,
    }


def _co2_temperature(pressure_bar: float) -> float:
    if pressure_bar < _CO2_TRIPLE_BAR:
        return float(_CO2_TRIPLE_K * (pressure_bar / _CO2_TRIPLE_BAR) ** 0.1)
    return float(_CO2_TRIPLE_K + 0.004 * (pressure_bar - _CO2_TRIPLE_BAR))


def _temperature_at(arm: dict[str, Any], pressure_bar: float) -> float | None:
    pressures = arm["pressure"]
    temperatures = arm["temperature"]
    for index in range(len(pressures) - 1):
        left, right = pressures[index], pressures[index + 1]
        if not ((left <= pressure_bar <= right) or (left >= pressure_bar >= right)):
            continue
        left_temperature, right_temperature = temperatures[index], temperatures[index + 1]
        if left_temperature is None or right_temperature is None:
            return None
        delta = right - left
        if abs(delta) < 1.0e-10:
            return float(left_temperature)
        fraction = (pressure_bar - left) / delta
        return float(left_temperature + fraction * (right_temperature - left_temperature))
    return None


def _margin_at(arm: dict[str, Any], pressure_bar: float) -> float:
    pressures = arm["pressure"]
    margins = arm["margin"]
    for index in range(len(pressures) - 1):
        left, right = pressures[index], pressures[index + 1]
        if not ((left <= pressure_bar <= right) or (left >= pressure_bar >= right)):
            continue
        delta = right - left
        if abs(delta) < 1.0e-10:
            return float(margins[index])
        fraction = (pressure_bar - left) / delta
        return float(margins[index] + fraction * (margins[index + 1] - margins[index]))
    return math.nan


def safety_envelope(
    components: list[str],
    z: list[float],
    min_pressure: Q,
    max_pressure: Q,
    num_points: int,
    eos: str | None = None,
    hydrate_margin: float | None = None,
    wax_margin: float | None = None,
    co2_margin: float | None = None,
    mdmt_margin: float | None = None,
    mdmt_design_temperature: Q | None = None,
    operating_pressure: Q | None = None,
    operating_temperature: Q | None = None,
    phase_trace: list[dict[str, Any]] | None = None,
) -> SafetyEnvelopeResult:
    """The operational safety envelope.

    Raises:
        InvalidInputError: for a non-positive pressure or temperature, fewer than two
            points, a descending range, one operating coordinate without the other, or a
            trace point that is not physical.
    """
    spec = _spec()
    warnings: list[Warning] = []
    minimum = input_to_si(spec, "min_pressure", min_pressure)
    maximum = input_to_si(spec, "max_pressure", max_pressure)
    design_value = (
        None
        if mdmt_design_temperature is None
        else input_to_si(spec, "mdmt_design_temperature", mdmt_design_temperature)
    )
    operating_p = (
        None
        if operating_pressure is None
        else input_to_si(spec, "operating_pressure", operating_pressure)
    )
    operating_t = (
        None
        if operating_temperature is None
        else input_to_si(spec, "operating_temperature", operating_temperature)
    )
    apply_checks(
        checks_for(spec).on_input,
        {
            "min_pressure": minimum,
            "max_pressure": maximum,
            "num_points": float(num_points),
            "mdmt_design_temperature": design_value,
            "operating_pressure": operating_p,
            "operating_temperature": operating_t,
        }.get,
        warnings,
    )
    count = int(num_points)
    if count < 2:
        raise InvalidInputError(
            "num_points", "the class divides by num_points - 1, so at least two points are needed"
        )
    if maximum < minimum:
        raise InvalidInputError("max_pressure", "the arm's last pressure is below its first")
    if (operating_p is None) != (operating_t is None):
        raise InvalidInputError(
            "operating_pressure", "both operating coordinates are stated, or neither"
        )
    design_temperature = 300.0 if design_value is None else design_value
    if not design_temperature > 0.0 or not math.isfinite(design_temperature):
        raise InvalidInputError("mdmt_design_temperature", "an absolute temperature is positive")
    hydrate_margin_value = (
        5.0 if hydrate_margin is None else _scalar(spec, "hydrate_margin", hydrate_margin)
    )
    wax_margin_value = 5.0 if wax_margin is None else _scalar(spec, "wax_margin", wax_margin)
    co2_margin_value = 3.0 if co2_margin is None else _scalar(spec, "co2_margin", co2_margin)
    mdmt_margin_value = 10.0 if mdmt_margin is None else _scalar(spec, "mdmt_margin", mdmt_margin)
    eos_name = "srk" if eos is None else eos
    names = [str(name) for name in components]
    if len(z) != len(names) or any(not math.isfinite(value) for value in z):
        raise InvalidInputError("z", "the composition needs one finite mole fraction per component")
    waters = [index for index, name in enumerate(names) if name.strip().lower() == "water"]
    water_content = float(z[waters[0]]) if waters else 0.0

    pressure_bar = [minimum + index / (count - 1) * (maximum - minimum) for index in range(count)]
    pressure_bar = [value / 1.0e5 for value in pressure_bar]

    arms: list[dict[str, Any]] = []
    hydrate_temperature: list[float | None] = []
    hydrate_margins: list[float] = []
    for pressure in pressure_bar:
        try:
            result = hydrate_formation_temperature(
                names, quantity(pressure * 1.0e5, "Pa"), z, eos=eos_name
            )
            hydrate_temperature.append(result.temperature.to("K").magnitude)
        except Exception:
            hydrate_temperature.append(None)
            warnings.append(
                Warning(
                    WarningCode.SOLVER_NOT_CONVERGED,
                    "the hydrate flash failed at this pressure; the point is absent",
                    "hydrate",
                )
            )
        hydrate_margins.append(hydrate_margin_value)
    arms.append(_arm("hydrate", pressure_bar, hydrate_temperature, hydrate_margins))

    wax_temperature: list[float | None] = []
    wax_margins: list[float] = []
    for pressure in pressure_bar:
        try:
            wax_temperature.append(wat_temperature(names, eos_name, 320.0, pressure * 1.0e5, z))
        except Exception:
            wax_temperature.append(None)
            warnings.append(
                Warning(
                    WarningCode.SOLVER_NOT_CONVERGED,
                    "the wax appearance search failed at this pressure; the point is absent",
                    "wax",
                )
            )
        wax_margins.append(wax_margin_value)
    arms.append(_arm("wax", pressure_bar, wax_temperature, wax_margins))

    co2_temperature = [_co2_temperature(pressure) for pressure in pressure_bar]
    arms.append(
        _arm("co2_freezing", pressure_bar, co2_temperature, [co2_margin_value] * len(pressure_bar))
    )

    mixture, ideal_gas = _components.mixture_of(names, eos=eos_name)
    mdmt_temperature: list[float | None] = []
    for pressure in pressure_bar:
        try:
            state = _fluid_state.flash(
                mixture, ideal_gas, design_temperature, pressure * 1.0e5, z, False
            )
            gamma = state.gamma
            if not math.isfinite(gamma) or gamma <= 1.0 or gamma > 2.0:
                gamma = 1.3
            if pressure <= _ATMOSPHERIC_BAR:
                value = design_temperature
            else:
                value = design_temperature * (_ATMOSPHERIC_BAR / pressure) ** (
                    (gamma - 1.0) / gamma
                )
            mdmt_temperature.append(value)
        except Exception:
            mdmt_temperature.append(design_temperature - 50.0)
            warnings.append(
                Warning(
                    WarningCode.SOLVER_NOT_CONVERGED,
                    "the MDMT flash failed at this pressure; the class's designT - 50 K is used",
                    "mdmt",
                )
            )
    arms.append(
        _arm("mdmt", pressure_bar, mdmt_temperature, [mdmt_margin_value] * len(pressure_bar))
    )

    if phase_trace is not None:
        trace_pressure: list[float] = []
        trace_temperature: list[float] = []
        for row in phase_trace:
            point_pressure = _si(row.get("pressure"), "Pa")
            point_temperature = _si(row.get("temperature"), "K")
            if (
                point_pressure is None
                or point_temperature is None
                or not point_pressure > 0.0
                or not point_temperature > 0.0
                or not math.isfinite(point_pressure)
                or not math.isfinite(point_temperature)
            ):
                raise InvalidInputError(
                    "phase_trace",
                    "a trace point needs a positive finite pressure and temperature",
                )
            trace_pressure.append(point_pressure / 1.0e5)
            trace_temperature.append(point_temperature)
        arms.append(
            _arm("phase_envelope", trace_pressure, trace_temperature, [0.0] * len(trace_pressure))
        )

    points: list[EnvelopePoint] = []
    summaries: list[EnvelopeSummary] = []
    for arm in arms:
        summaries.append(
            EnvelopeSummary(
                envelope=arm["envelope"],
                name=_NAMES[arm["envelope"]],
                point_count=float(len(arm["pressure"])),
                reference_water_content=water_content if arm["envelope"] == "hydrate" else 0.0,
                reference_wax_content=0.0,
            )
        )
        for index, pressure in enumerate(arm["pressure"]):
            temperature = arm["temperature"][index]
            margin = arm["margin"][index]
            points.append(
                EnvelopePoint(
                    envelope=arm["envelope"],
                    index=float(index),
                    pressure=from_si(pressure * 1.0e5, "Pa"),
                    temperature=None if temperature is None else from_si(temperature, "K"),
                    margin=from_si(margin, "K"),
                    safe_temperature=(
                        None if temperature is None else from_si(temperature - margin, "K")
                    ),
                )
            )

    queries: list[EnvelopeQuery] = []
    all_safe: bool | None = None
    most_limiting: str | None = None
    if operating_p is not None and operating_t is not None:
        operating_bar = operating_p / 1.0e5
        best: tuple[str, float] | None = None
        for arm in arms:
            temperature = _temperature_at(arm, operating_bar)
            safe_temperature = (
                None if temperature is None else temperature - _margin_at(arm, operating_bar)
            )
            if arm["envelope"] == "phase_envelope" or safe_temperature is None:
                safe = True
            else:
                safe = operating_t > safe_temperature
            margin = None if temperature is None else operating_t - temperature
            queries.append(
                EnvelopeQuery(
                    envelope=arm["envelope"],
                    temperature=None if temperature is None else from_si(temperature, "K"),
                    safe_temperature=(
                        None if safe_temperature is None else from_si(safe_temperature, "K")
                    ),
                    margin=None if margin is None else from_si(margin, "K"),
                    safe=safe,
                )
            )
            if margin is not None and math.isfinite(margin) and (best is None or margin < best[1]):
                best = (arm["envelope"], margin)
        all_safe = all(query.safe for query in queries)
        most_limiting = None if best is None else _NAMES[best[0]]

    return SafetyEnvelopeResult(
        envelopes=tuple(summaries),
        points=tuple(points),
        queries=tuple(queries),
        all_safe=all_safe,
        most_limiting_envelope=most_limiting,
        warnings=tuple(warnings),
    )
