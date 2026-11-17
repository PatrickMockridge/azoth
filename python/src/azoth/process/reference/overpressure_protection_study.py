"""``process.overpressure_protection_study`` - the governing case, its sizing and verdict.

As NeqSim's ``OverpressureProtectionStudy.evaluate`` computes it. Spec:
``specs/models/process/overpressure_protection_study.toml``

The Python twin of ``crates/azoth-process/src/models/overpressure_protection_study.rs``: the
governing-case selection, the three sizing branches over the ported API 520 arithmetic, and
the accumulated-pressure check.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import OverpressureProtectionStudyResult
from azoth.core.units import Q, from_si, input_records_to_si, input_to_si, quantity
from azoth.core.warnings import Warning
from azoth.process.reference.overpressure_acceptance import (
    overpressure_acceptance as acceptance_check,
)
from azoth.process.reference.psv_sizing import (
    _gas_area,
    _liquid_area,
    _select_orifice,
    _two_phase_area,
)

MODEL_ID = "process.overpressure_protection_study"

#: The class's fire overpressure basis.
FIRE_OVERPRESSURE = 0.21
#: The class's non-fire overpressure basis.
NON_FIRE_OVERPRESSURE = 0.10
#: The class's liquid-density fallback, kg/m3.
LIQUID_DENSITY_FALLBACK = 700.0
#: The class's liquid-viscosity fallback, Pa s.
LIQUID_VISCOSITY_FALLBACK = 5.0e-4
#: The class's default relieving temperature, K.
DEFAULT_TEMPERATURE = 288.15
#: Square metres per square inch.
M2_PER_IN2 = 6.4516e-4
#: The default back pressure, Pa.
DEFAULT_BACK_PRESSURE = 101_325.0

#: The class's cause spellings and their labels, which the acceptance basis carries.
_CAUSE_LABELS = {
    "blocked_outlet": "Blocked outlet",
    "check_valve_leakage": "Check valve leakage",
    "gas_blow_by": "Gas blow-by",
    "liquid_blow_by": "Liquid blow-by",
    "volatile_liquid_ingress": "Volatile liquid ingress",
    "inadvertent_valve_opening": "Inadvertent valve opening",
    "control_valve_failure": "Control valve failure",
    "fire": "Fire",
    "thermal_expansion": "Thermal expansion",
    "choke_collapse": "Choke collapse",
    "tube_rupture": "Tube rupture",
    "vacuum": "Vacuum collapse",
    "other": "Other",
}

_PHASES = ("vapour", "liquid", "two_phase")


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _empty(messages: list[str], warnings: list[Warning]) -> OverpressureProtectionStudyResult:
    return OverpressureProtectionStudyResult(
        governing_name=None,
        governing_cause=None,
        governing_phase=None,
        governing_relief_rate=None,
        required_area=None,
        required_area_in2=None,
        recommended_orifice=None,
        selected_area_in2=None,
        selected_orifice_capacity=None,
        capacity_adequate=False,
        acceptance_peak_pressure=None,
        acceptance_allowable_pressure=None,
        acceptance_accumulation_factor=None,
        acceptance_margin=None,
        accepted=None,
        messages=tuple(messages),
        warnings=tuple(warnings),
    )


def overpressure_protection_study(
    mawp: Q,
    scenarios: list[dict[str, object]],
    relief_set_pressure: Q | None = None,
    back_pressure: Q | None = None,
    multiple_devices: bool | None = None,
) -> OverpressureProtectionStudyResult:
    """The governing case, its sizing and the accumulated-pressure verdict.

    Raises:
        InvalidInputError: for a non-positive MAWP or set pressure, a negative back
            pressure, a scenario whose cause or phase is not the class's, a stated
            non-positive property, or a gas governing case with no molar mass.
    """
    spec = _spec()
    warnings: list[Warning] = []
    mawp_si = input_to_si(spec, "mawp", mawp)
    set_si = (
        mawp_si
        if relief_set_pressure is None
        else input_to_si(spec, "relief_set_pressure", relief_set_pressure)
    )
    back_si = (
        DEFAULT_BACK_PRESSURE
        if back_pressure is None
        else input_to_si(spec, "back_pressure", back_pressure)
    )
    multiple = False if multiple_devices is None else multiple_devices
    apply_checks(
        checks_for(spec).on_input,
        {
            "mawp": mawp_si,
            "relief_set_pressure": None if relief_set_pressure is None else set_si,
            "back_pressure": None if back_pressure is None else back_si,
        }.get,
        warnings,
    )
    rows: list[dict[str, Any]] = input_records_to_si(spec, "scenarios", scenarios)

    for index, row in enumerate(rows):
        cause = str(row["cause"])
        phase = str(row["phase"])
        if cause not in _CAUSE_LABELS:
            raise InvalidInputError(
                f"scenarios[{index}].cause",
                f"{cause!r} is not one of the class's overpressure causes",
            )
        if phase not in _PHASES:
            raise InvalidInputError(
                f"scenarios[{index}].phase",
                f"{phase!r} is not `vapour`, `liquid` or `two_phase`",
            )
        rate = float(row["relief_rate"])
        if not math.isfinite(rate) or rate < 0.0:
            raise InvalidInputError(
                f"scenarios[{index}].relief_rate",
                f"a relief rate is finite and non-negative, and {rate} is not",
            )
        for field in (
            "relief_temperature",
            "molar_mass",
            "compressibility",
            "specific_heat_ratio",
            "density",
            "viscosity",
            "gas_density",
            "liquid_density",
            "latent_heat",
            "liquid_heat_capacity",
        ):
            value = row.get(field)
            if value is not None:
                number = float(value)
                if not math.isfinite(number) or number <= 0.0:
                    raise InvalidInputError(
                        f"scenarios[{index}].{field}",
                        f"{number} must be finite and positive",
                    )
        ratio = row.get("specific_heat_ratio")
        if ratio is not None and float(ratio) <= 1.0:
            raise InvalidInputError(
                f"scenarios[{index}].specific_heat_ratio",
                f"{ratio} is not above one",
            )
        fraction = row.get("gas_mass_fraction")
        if fraction is not None and not 0.0 <= float(fraction) <= 1.0:
            raise InvalidInputError(
                f"scenarios[{index}].gas_mass_fraction",
                f"{fraction} is outside [0, 1]",
            )

    governing: dict[str, Any] | None = None
    for row in rows:
        if row.get("credible") is False:
            continue
        if governing is None or float(row["relief_rate"]) > float(governing["relief_rate"]):
            governing = row
    if governing is None:
        return _empty(["No credible relief scenario defined; nothing to size"], warnings)

    cause = str(governing["cause"])
    phase = str(governing["phase"])
    overpressure = FIRE_OVERPRESSURE if cause == "fire" else NON_FIRE_OVERPRESSURE
    rate = float(governing["relief_rate"])
    messages: list[str] = []
    required_area: float | None = None
    required_in2: float | None = None
    recommended: str | None = None
    selected_in2: float | None = None
    capacity: float | None = None
    adequate = False

    if rate <= 0.0:
        messages.append("Governing relief rate is zero; relief device cannot be sized")
    elif phase == "vapour":
        molar_mass = governing.get("molar_mass")
        if molar_mass is None:
            raise InvalidInputError(
                "scenarios",
                f"the governing vapour scenario {governing['name']!r} has no molar mass, "
                "and the gas sizing needs one",
            )
        temperature = (
            DEFAULT_TEMPERATURE
            if governing.get("relief_temperature") is None
            else float(governing["relief_temperature"])
        )
        compressibility = float(governing.get("compressibility", 1.0))
        ratio = float(governing.get("specific_heat_ratio", 1.3))
        sizing = _gas_area(
            rate,
            set_si,
            overpressure,
            back_si,
            temperature,
            float(molar_mass),
            compressibility,
            ratio,
            False,
            False,
        )
        required_area = float(sizing["required_area"])
        required_in2 = float(sizing["required_area_in2"])
        recommended = str(sizing["recommended_orifice"])
        selected_in2 = float(sizing["selected_area_in2"])
        capacity = rate * selected_in2 / required_in2 if required_in2 > 0.0 else None
        adequate = selected_in2 >= required_in2
        if not adequate:
            messages.append(
                "Required relief area exceeds the largest standard orifice; multiple or "
                "larger relief devices are required"
            )
    elif phase == "liquid":
        stated = governing.get("density")
        if stated is not None and float(stated) > 0.0:
            density = float(stated)
        else:
            stated = governing.get("liquid_density")
            if stated is not None and float(stated) > 0.0:
                density = float(stated)
            else:
                messages.append(
                    "Liquid density not set; using default 700 kg/m3 for the liquid relief sizing"
                )
                density = LIQUID_DENSITY_FALLBACK
        viscosity = governing.get("viscosity")
        viscosity = (
            LIQUID_VISCOSITY_FALLBACK
            if viscosity is None or float(viscosity) <= 0.0
            else float(viscosity)
        )
        sizing = _liquid_area(
            rate / density,
            density,
            set_si,
            overpressure,
            back_si,
            viscosity,
            False,
        )
        required_area = float(sizing["required_area"])
        required_in2 = float(sizing["required_area_in2"])
        recommended = str(sizing["recommended_orifice"])
        selected_in2 = float(sizing["selected_area_in2"])
        capacity = rate * selected_in2 / required_in2 if required_in2 > 0.0 else None
        adequate = selected_in2 >= required_in2
        if not adequate:
            messages.append(
                "Required liquid relief area exceeds the largest standard orifice; multiple "
                "or larger relief devices are required"
            )
    else:
        missing = any(
            governing.get(field) is None
            for field in (
                "gas_mass_fraction",
                "gas_density",
                "liquid_density",
                "latent_heat",
                "liquid_heat_capacity",
            )
        )
        if missing:
            messages.append(
                "Two-phase governing case is missing omega-method inputs (gas fraction, "
                "gas/liquid density, latent heat, liquid Cp); two-phase sizing skipped"
            )
        else:
            temperature = (
                DEFAULT_TEMPERATURE
                if governing.get("relief_temperature") is None
                else float(governing["relief_temperature"])
            )
            area = _two_phase_area(
                rate,
                set_si,
                overpressure,
                back_si,
                temperature,
                float(governing["gas_mass_fraction"]),
                float(governing["gas_density"]),
                float(governing["liquid_density"]),
                float(governing["latent_heat"]),
                float(governing["liquid_heat_capacity"]),
            )
            area_in2 = area / M2_PER_IN2
            letter, selected = _select_orifice(area_in2)
            required_area = area
            required_in2 = area_in2
            recommended = letter
            selected_in2 = selected
            capacity = rate * selected / area_in2 if area_in2 > 0.0 else None
            adequate = selected >= area_in2
            if not adequate:
                messages.append(
                    "Required two-phase relief area exceeds the largest standard orifice; "
                    "multiple or larger relief devices are required"
                )

    peak = set_si * (1.0 + overpressure)
    acceptance = acceptance_check(quantity(peak, "Pa"), quantity(mawp_si, "Pa"), cause, multiple)
    if not acceptance.accepted:
        suffix = (
            "" if cause == "fire" else (", multiple devices" if multiple else ", single device")
        )
        factor = acceptance.accumulation_factor
        messages.append(
            "Relieving pressure exceeds the allowable accumulated pressure: "
            f"{factor * 100.0:.0f}% of MAWP ({mawp_si / 1.0e5:.2f} bara x {factor:.2f}) "
            f"for {_CAUSE_LABELS[cause]}{suffix}"
        )
    warnings.extend(acceptance.warnings)

    return OverpressureProtectionStudyResult(
        governing_name=str(governing["name"]),
        governing_cause=cause,
        governing_phase=phase,
        governing_relief_rate=from_si(rate, "kg/s"),
        required_area=None if required_area is None else from_si(required_area, "m**2"),
        required_area_in2=required_in2,
        recommended_orifice=recommended,
        selected_area_in2=selected_in2,
        selected_orifice_capacity=None if capacity is None else from_si(capacity, "kg/s"),
        capacity_adequate=adequate,
        acceptance_peak_pressure=from_si(peak, "Pa"),
        acceptance_allowable_pressure=acceptance.allowable_pressure,
        acceptance_accumulation_factor=acceptance.accumulation_factor,
        acceptance_margin=acceptance.margin,
        accepted=acceptance.accepted,
        messages=tuple(messages),
        warnings=tuple(warnings),
    )
