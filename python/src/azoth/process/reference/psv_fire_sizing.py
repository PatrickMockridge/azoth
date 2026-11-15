"""``process.psv_fire_sizing`` - the API 521 dynamic fire-case sizing.

As NeqSim's ``ReliefValveSizing.dynamicFireSizing`` computes it. Spec:
``specs/models/process/psv_fire_sizing.toml``

The Python twin of ``crates/azoth-process/src/models/psv_fire_sizing.rs``: the design
flow, the one-minute heat-up, the 21 % gas area and the blowdown/Cv/orifice helpers.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import PsvFireSizingResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.psv_fire_sizing"

_ORIFICES: tuple[tuple[str, float], ...] = (
    ("D", 0.110),
    ("E", 0.196),
    ("F", 0.307),
    ("G", 0.503),
    ("H", 0.785),
    ("J", 1.287),
    ("K", 1.838),
    ("L", 2.853),
    ("M", 3.600),
    ("N", 4.340),
    ("P", 6.380),
    ("Q", 11.05),
    ("R", 16.0),
    ("T", 26.0),
)


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _select_orifice(required_area_in2: float) -> tuple[str, float]:
    for letter, area in _ORIFICES:
        if area >= required_area_in2:
            return letter, area
    return "T", 26.0


def _require_positive(name: str, value: float) -> None:
    if not math.isfinite(value) or value <= 0.0:
        raise InvalidInputError(name, f"a finite positive value is required, and {value} is not")


def _gas_area(
    mass_flow: float,
    set_pressure: float,
    overpressure_fraction: float,
    back_pressure: float,
    temperature: float,
    molar_mass: float,
    compressibility: float,
    specific_heat_ratio: float,
) -> tuple[float, float, str, float, float, float, float]:
    k = specific_heat_ratio
    relieving_pressure = set_pressure * (1.0 + overpressure_fraction)
    back_pressure_fraction = back_pressure / relieving_pressure
    kd = 0.975
    kc = 1.0
    if back_pressure_fraction > 0.5:
        kb = max(1.0 - (back_pressure_fraction - 0.5) * 2.0, 0.1)
    else:
        kb = 1.0
    c = 520.0 * math.sqrt(k * (2.0 / (k + 1.0)) ** ((k + 1.0) / (k - 1.0)))
    molar_mass_g_per_mol = molar_mass * 1000.0
    pressure_psia = relieving_pressure / 6894.76
    mass_flow_lb_per_hr = mass_flow * 7936.64
    temperature_rankine = temperature * 1.8
    area_in2 = (
        mass_flow_lb_per_hr
        / (c * kd * kb * kc * pressure_psia)
        * math.sqrt(temperature_rankine * compressibility / molar_mass_g_per_mol)
    )
    letter, selected_area_in2 = _select_orifice(area_in2)
    return (
        area_in2 * 6.4516e-4,
        area_in2,
        letter,
        selected_area_in2,
        back_pressure_fraction,
        kb,
        kc,
    )


def psv_fire_sizing(
    initial_mass: Q,
    initial_pressure: Q,
    set_pressure: Q,
    initial_temperature: Q,
    fire_heat_input: Q,
    vessel_volume: Q,
    molar_mass: Q,
    specific_heat_ratio: float,
    compressibility: float,
    heat_capacity: Q,
    blowdown_time: Q,
    blowdown_percent: float | None = None,
    orifice_area: Q | None = None,
    cv_discharge_coefficient: float | None = None,
) -> PsvFireSizingResult:
    """The design flow, the 21 % area and the blowdown, Cv and next-orifice helpers.

    Raises:
        InvalidInputError: for a non-positive mass, time, heat capacity, pressure,
            temperature, molar mass or compressibility; an exponent at or below one; a
            negative fire heat input; a blowdown percentage outside ``[0, 100]``; or an
            area and coefficient that are not stated together.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    initial_mass_si = input_to_si(spec, "initial_mass", initial_mass)
    initial_pressure_si = input_to_si(spec, "initial_pressure", initial_pressure)
    set_pressure_si = input_to_si(spec, "set_pressure", set_pressure)
    initial_temperature_si = input_to_si(spec, "initial_temperature", initial_temperature)
    fire_heat_input_si = input_to_si(spec, "fire_heat_input", fire_heat_input)
    vessel_volume_si = input_to_si(spec, "vessel_volume", vessel_volume)
    molar_mass_si = input_to_si(spec, "molar_mass", molar_mass)
    heat_capacity_si = input_to_si(spec, "heat_capacity", heat_capacity)
    blowdown_time_si = input_to_si(spec, "blowdown_time", blowdown_time)
    orifice_area_si = (
        None if orifice_area is None else input_to_si(spec, "orifice_area", orifice_area)
    )

    apply_checks(
        checks.on_input,
        {
            "initial_mass": initial_mass_si,
            "initial_pressure": initial_pressure_si,
            "set_pressure": set_pressure_si,
            "initial_temperature": initial_temperature_si,
            "fire_heat_input": fire_heat_input_si,
            "vessel_volume": vessel_volume_si,
            "molar_mass": molar_mass_si,
            "specific_heat_ratio": specific_heat_ratio,
            "compressibility": compressibility,
            "heat_capacity": heat_capacity_si,
            "blowdown_time": blowdown_time_si,
            "blowdown_percent": blowdown_percent,
            "orifice_area": orifice_area_si,
            "cv_discharge_coefficient": cv_discharge_coefficient,
        }.get,
        warnings,
    )

    for name, value in (
        ("initial_mass", initial_mass_si),
        ("set_pressure", set_pressure_si),
        ("initial_temperature", initial_temperature_si),
        ("molar_mass", molar_mass_si),
        ("compressibility", compressibility),
        ("heat_capacity", heat_capacity_si),
        ("blowdown_time", blowdown_time_si),
    ):
        _require_positive(name, value)
    if not math.isfinite(fire_heat_input_si) or fire_heat_input_si < 0.0:
        raise InvalidInputError(
            "fire_heat_input",
            f"a finite non-negative heat input is required, and {fire_heat_input_si} is not",
        )
    if not math.isfinite(specific_heat_ratio) or specific_heat_ratio <= 1.0:
        raise InvalidInputError(
            "specific_heat_ratio",
            f"an exponent above one is required, and {specific_heat_ratio} is not",
        )
    if blowdown_percent is not None and (
        not math.isfinite(blowdown_percent) or not 0.0 <= blowdown_percent <= 100.0
    ):
        raise InvalidInputError(
            "blowdown_percent",
            f"a percentage in [0, 100] is required, and {blowdown_percent} is not",
        )

    overpressure_fraction = 0.21
    delta_t = fire_heat_input_si * 60.0 / (initial_mass_si * heat_capacity_si)
    relieving_temperature = initial_temperature_si + delta_t
    peak_mass_flow = fire_heat_input_si / (heat_capacity_si * 50.0)
    average_mass_flow = initial_mass_si / blowdown_time_si
    design_mass_flow = max(peak_mass_flow, average_mass_flow)
    area_m2, area_in2, recommended, selected, back_fraction, kb, kc = _gas_area(
        design_mass_flow,
        set_pressure_si,
        overpressure_fraction,
        101_325.0,
        relieving_temperature,
        molar_mass_si,
        compressibility,
        specific_heat_ratio,
    )
    if orifice_area_si is not None and cv_discharge_coefficient is None:
        raise InvalidInputError("cv_discharge_coefficient", "is required with orifice_area")
    if orifice_area_si is None and cv_discharge_coefficient is not None:
        raise InvalidInputError("orifice_area", "is required with cv_discharge_coefficient")
    if orifice_area_si is None:
        flow_coefficient = None
    else:
        assert cv_discharge_coefficient is not None
        flow_coefficient = 24.6 * float(cv_discharge_coefficient) * (orifice_area_si / 6.4516e-4)
    blowdown_pressure = (
        None
        if blowdown_percent is None
        else from_si(set_pressure_si * (1.0 - blowdown_percent / 100.0), "Pa")
    )
    return PsvFireSizingResult(
        overpressure_fraction=overpressure_fraction,
        required_area=from_si(area_m2, "m**2"),
        required_area_in2=area_in2,
        recommended_orifice=recommended,
        selected_area_in2=selected,
        back_pressure_fraction=back_fraction,
        discharge_coefficient=0.975,
        back_pressure_correction=kb,
        combination_correction=kc,
        blowdown_pressure=blowdown_pressure,
        flow_coefficient=flow_coefficient,
        warnings=tuple(warnings),
    )
