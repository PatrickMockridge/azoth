"""``process.acoustic_induced_vibration_screening`` - the Carucci-Mueller AIV screen.

As NeqSim's ``AcousticInducedVibrationScreening.screen`` computes it. Spec:
``specs/models/process/acoustic_induced_vibration_screening.toml``

The Python twin of ``crates/azoth-process/src/models/acoustic_induced_vibration_screening.rs``:
the sound power level, the design-line allowable and the margin's likelihood band.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import AcousticInducedVibrationResult, FivLikelihood
from azoth.core.units import Q, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.acoustic_induced_vibration_screening"

#: The class's kg/s to lb/s factor.
KG_PER_S_TO_LB_PER_S = 2.2046226218
#: The class's kelvin to degrees Rankine factor.
KELVIN_TO_RANKINE = 1.8
#: The Carucci-Mueller additive constant, dB.
PWL_CONSTANT = 126.1
#: The design-line intercept, dB.
DESIGN_LINE_INTERCEPT = 173.6
#: The design-line slope, dB per unit D/t.
DESIGN_LINE_SLOPE = 0.125
#: The grams in a kilogram, for the correlation's g/mol form.
GRAMS_PER_KILOGRAM = 1000.0


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    pwl: float
    allowable_pwl: float
    margin: float
    likelihood: FivLikelihood
    pressure_drop_ratio: float
    d_overt: float


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


def _band_for(margin: float) -> FivLikelihood:
    """The AIV band from the margin, with the class's exclusive upper bounds."""
    if margin < -10.0:
        return FivLikelihood.LOW
    if margin < 0.0:
        return FivLikelihood.MEDIUM
    if margin < 10.0:
        return FivLikelihood.HIGH
    return FivLikelihood.VERY_HIGH


def _evaluate(
    mass_flow: Q,
    upstream_pressure: Q,
    downstream_pressure: Q,
    temperature: Q,
    molecular_weight: Q,
    outside_diameter: Q,
    wall_thickness: Q,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []

    flow_si = input_to_si(spec, "mass_flow", mass_flow)
    upstream_si = input_to_si(spec, "upstream_pressure", upstream_pressure)
    downstream_si = input_to_si(spec, "downstream_pressure", downstream_pressure)
    temperature_si = input_to_si(spec, "temperature", temperature)
    molar_si = input_to_si(spec, "molecular_weight", molecular_weight)
    diameter_si = input_to_si(spec, "outside_diameter", outside_diameter)
    wall_si = input_to_si(spec, "wall_thickness", wall_thickness)
    apply_checks(
        checks_for(spec).on_input,
        {
            "mass_flow": flow_si,
            "upstream_pressure": upstream_si,
            "downstream_pressure": downstream_si,
            "temperature": temperature_si,
            "molecular_weight": molar_si,
            "outside_diameter": diameter_si,
            "wall_thickness": wall_si,
        }.get,
        warnings,
    )
    for name, value in (
        ("mass_flow", flow_si),
        ("upstream_pressure", upstream_si),
        ("downstream_pressure", downstream_si),
        ("temperature", temperature_si),
        ("molecular_weight", molar_si),
        ("outside_diameter", diameter_si),
        ("wall_thickness", wall_si),
    ):
        _require_finite(name, value)
    if downstream_si >= upstream_si:
        raise InvalidInputError(
            "downstream_pressure",
            "the downstream pressure must be strictly below the upstream pressure",
        )

    pressure_drop_ratio = (upstream_si - downstream_si) / upstream_si
    flow_lb_per_s = flow_si * KG_PER_S_TO_LB_PER_S
    temperature_rankine = temperature_si * KELVIN_TO_RANKINE
    d_overt = diameter_si / wall_si
    molar_gmol = molar_si * GRAMS_PER_KILOGRAM

    argument = (
        pressure_drop_ratio**3.6
        * flow_lb_per_s
        * flow_lb_per_s
        * (temperature_rankine / molar_gmol) ** 1.2
    )
    pwl = 10.0 * math.log10(argument) + PWL_CONSTANT
    allowable_pwl = DESIGN_LINE_INTERCEPT - DESIGN_LINE_SLOPE * d_overt
    margin = pwl - allowable_pwl

    return (
        _Solution(
            pwl=pwl,
            allowable_pwl=allowable_pwl,
            margin=margin,
            likelihood=_band_for(margin),
            pressure_drop_ratio=pressure_drop_ratio,
            d_overt=d_overt,
        ),
        warnings,
    )


def acoustic_induced_vibration_screening(
    mass_flow: Q,
    upstream_pressure: Q,
    downstream_pressure: Q,
    temperature: Q,
    molecular_weight: Q,
    outside_diameter: Q,
    wall_thickness: Q,
) -> AcousticInducedVibrationResult:
    """The sound power level downstream of a pressure-reducing device and its band.

    The correlation is evaluated in the class's imperial form; the molecular weight is
    converted from kg/mol to g/mol before it. The bands are exclusive upward, so -10 dB is
    medium, 0 high and +10 very high. The class's recommendation text is presentation.

    Args:
        mass_flow: the gas mass flow through the device.
        upstream_pressure: the upstream absolute pressure.
        downstream_pressure: the downstream absolute pressure, strictly below the upstream.
        temperature: the downstream temperature.
        molecular_weight: the gas molar mass.
        outside_diameter: the pipe outside diameter.
        wall_thickness: the pipe wall thickness.

    Returns:
        The sound power level, the allowable and the margin's band.

    Raises:
        InvalidInputError: for a non-positive flow, upstream pressure, temperature, molar
            mass or diameter, a downstream pressure outside ``[0, upstream)``, or a
            non-finite input.
    """
    solution, warnings = _evaluate(
        mass_flow,
        upstream_pressure,
        downstream_pressure,
        temperature,
        molecular_weight,
        outside_diameter,
        wall_thickness,
    )
    return AcousticInducedVibrationResult(
        pwl=solution.pwl,
        allowable_pwl=solution.allowable_pwl,
        margin=solution.margin,
        likelihood=solution.likelihood,
        pressure_drop_ratio=solution.pressure_drop_ratio,
        d_overt=solution.d_overt,
        warnings=tuple(warnings),
    )
