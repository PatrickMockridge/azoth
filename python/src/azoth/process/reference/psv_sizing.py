"""``process.psv_sizing`` - the API 520 relief-orifice arithmetic.

As NeqSim's ``ReliefValveSizing`` computes it. Spec:
``specs/models/process/psv_sizing.toml``

The Python twin of ``crates/azoth-process/src/models/psv_sizing.rs``: the gas, liquid
and two-phase area methods, the D to T ladder and the capacity helper whose unit basis
disagrees with the sizing path.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import PsvSizingResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.psv_sizing"

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

_SERVICES = ("gas", "liquid", "two_phase")


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _select_orifice(required_area_in2: float) -> tuple[str, float]:
    for letter, area in _ORIFICES:
        if area >= required_area_in2:
            return letter, area
    return "T", 26.0


def _require_finite(name: str, value: float) -> None:
    if not math.isfinite(value):
        raise InvalidInputError(name, f"a finite value is required, and {value} is not")


def _present(value: float | None, name: str, service: str) -> float:
    if value is None:
        raise InvalidInputError(name, f"required for the {service} service")
    return value


def _refuse(value: object | None, name: str, service: str) -> None:
    if value is not None:
        raise InvalidInputError(name, f"is not read by the {service} service")


def _gas_area(
    mass_flow: float,
    set_pressure: float,
    overpressure_fraction: float,
    back_pressure: float,
    temperature: float,
    molar_mass: float,
    compressibility: float,
    specific_heat_ratio: float,
    balanced_bellows: bool,
    has_rupture_disk: bool,
) -> dict[str, float | str]:
    k = specific_heat_ratio
    relieving_pressure = set_pressure * (1.0 + overpressure_fraction)
    back_pressure_fraction = back_pressure / relieving_pressure
    kd = 0.975
    kc = 0.9 if has_rupture_disk else 1.0
    critical_ratio = (2.0 / (k + 1.0)) ** (k / (k - 1.0))
    if balanced_bellows and back_pressure_fraction > critical_ratio:
        r = back_pressure_fraction
        value = math.sqrt(
            (k / (k - 1.0)) * (r ** (2.0 / k) - r ** ((k + 1.0) / k)) / (1.0 - critical_ratio)
        )
        kb = min(value, 1.0)
    elif not balanced_bellows and back_pressure_fraction > 0.5:
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
    return {
        "required_area": area_in2 * 6.4516e-4,
        "required_area_in2": area_in2,
        "recommended_orifice": letter,
        "selected_area_in2": selected_area_in2,
        "back_pressure_fraction": back_pressure_fraction,
        "back_pressure_correction": kb,
        "combination_correction": kc,
    }


def _liquid_area(
    volume_flow: float,
    liquid_density: float,
    set_pressure: float,
    overpressure_fraction: float,
    back_pressure: float,
    viscosity: float,
    balanced_bellows: bool,
) -> dict[str, float | str]:
    relieving_pressure = set_pressure * (1.0 + overpressure_fraction)
    specific_gravity = liquid_density / 999.0
    flow_gpm = volume_flow * 15850.3
    p1_psig = (relieving_pressure - 101_325.0) / 6894.76
    p2_psig = (back_pressure - 101_325.0) / 6894.76
    kd = 0.65
    if balanced_bellows and back_pressure / relieving_pressure > 0.5:
        kw = max(1.0 - 0.5 * (back_pressure / relieving_pressure - 0.5), 0.1)
    else:
        kw = 1.0
    dp = p1_psig - p2_psig
    if dp <= 0.0:
        dp = 1.0
    initial_area_in2 = flow_gpm / (38.0 * kd * kw) * math.sqrt(specific_gravity / dp)
    kv = 1.0
    if viscosity > 0.0 and initial_area_in2 > 0.0:
        area_si = initial_area_in2 * 6.4516e-4
        equivalent_diameter = math.sqrt(4.0 * area_si / math.pi)
        velocity = volume_flow / area_si
        reynolds = liquid_density * velocity * equivalent_diameter / viscosity
        if 0.0 < reynolds < 100_000.0:
            kv = 1.0 / (0.9935 + 2.878 / math.sqrt(reynolds) + 342.75 / reynolds**1.5)
            kv = min(max(kv, 0.2), 1.0)
    kp = min(0.6 + 4.0 * overpressure_fraction, 1.0) if overpressure_fraction < 0.25 else 1.0
    area_in2 = flow_gpm / (38.0 * kd * kw * kv * kp) * math.sqrt(specific_gravity / dp)
    letter, selected_area_in2 = _select_orifice(area_in2)
    return {
        "required_area": area_in2 * 6.4516e-4,
        "required_area_in2": area_in2,
        "recommended_orifice": letter,
        "selected_area_in2": selected_area_in2,
        "back_pressure_fraction": back_pressure / relieving_pressure,
        "back_pressure_correction": kw,
        "viscosity_correction": kv,
        "mass_flow": volume_flow * liquid_density,
    }


def _two_phase_area(
    mass_flow: float,
    set_pressure: float,
    overpressure_fraction: float,
    back_pressure: float,
    temperature: float,
    gas_fraction: float,
    gas_density: float,
    liquid_density: float,
    latent_heat: float,
    liquid_heat_capacity: float,
) -> float:
    p0 = set_pressure * (1.0 + overpressure_fraction)
    v_g = 1.0 / gas_density
    v_l = 1.0 / liquid_density
    mix = gas_fraction * v_g + (1.0 - gas_fraction) * v_l
    omega = gas_fraction * v_g / mix + (
        liquid_heat_capacity * temperature * p0 * (v_g - v_l) ** 2 / (latent_heat**2 * mix)
    )
    eta_c = 0.55 + 0.217 * math.log(omega) if omega <= 1.0 else 0.85 / (1.0 + omega)
    eta_c = max(eta_c, back_pressure / p0)
    g2 = p0 / mix * (2.0 * (omega * math.log(1.0 / eta_c) + (1.0 - omega) * (1.0 - eta_c)))
    mass_flux = math.sqrt(max(g2, 0.0))
    if mass_flux <= 0.0:
        return 0.0
    return mass_flow / (0.85 * mass_flux)


def _capacity(
    area: float,
    set_pressure: float,
    overpressure_fraction: float,
    back_pressure: float,
    temperature: float,
    molar_mass: float,
    compressibility: float,
    specific_heat_ratio: float,
    discharge_coefficient: float,
) -> float:
    k = specific_heat_ratio
    relieving_pressure = set_pressure * (1.0 + overpressure_fraction)
    critical_ratio = (2.0 / (k + 1.0)) ** (k / (k - 1.0))
    pressure_ratio = back_pressure / relieving_pressure
    if pressure_ratio <= critical_ratio:
        c = math.sqrt(k * (2.0 / (k + 1.0)) ** ((k + 1.0) / (k - 1.0)))
    else:
        c = math.sqrt(
            2.0
            * k
            / (k - 1.0)
            * max(
                pressure_ratio ** (2.0 / k) - pressure_ratio ** ((k + 1.0) / k),
                0.0,
            )
        )
    return (
        discharge_coefficient
        * area
        * relieving_pressure
        * c
        * math.sqrt(molar_mass / (compressibility * 8314.0 * temperature))
    )


def psv_sizing(
    service: str,
    set_pressure: Q,
    overpressure_fraction: float,
    back_pressure: Q,
    is_balanced_bellows: bool | None = None,
    has_rupture_disk: bool | None = None,
    mass_flow: Q | None = None,
    temperature: Q | None = None,
    molar_mass: Q | None = None,
    compressibility: float | None = None,
    specific_heat_ratio: float | None = None,
    volume_flow: Q | None = None,
    liquid_density: Q | None = None,
    viscosity: Q | None = None,
    gas_fraction: float | None = None,
    gas_density: Q | None = None,
    latent_heat: float | None = None,
    liquid_heat_capacity: Q | None = None,
    orifice_area: Q | None = None,
    capacity_discharge_coefficient: float | None = None,
) -> PsvSizingResult:
    """The gas, liquid or two-phase required area, the ladder and the capacity helper.

    Raises:
        InvalidInputError: for a missing branch input, an input the branch does not read,
            a back pressure at or above the relieving pressure, or a value its range
            refuses.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    set_pressure_si = input_to_si(spec, "set_pressure", set_pressure)
    back_pressure_si = input_to_si(spec, "back_pressure", back_pressure)
    mass_flow_si = None if mass_flow is None else input_to_si(spec, "mass_flow", mass_flow)
    temperature_si = None if temperature is None else input_to_si(spec, "temperature", temperature)
    molar_mass_si = None if molar_mass is None else input_to_si(spec, "molar_mass", molar_mass)
    volume_flow_si = None if volume_flow is None else input_to_si(spec, "volume_flow", volume_flow)
    liquid_density_si = (
        None if liquid_density is None else input_to_si(spec, "liquid_density", liquid_density)
    )
    viscosity_si = None if viscosity is None else input_to_si(spec, "viscosity", viscosity)
    gas_density_si = None if gas_density is None else input_to_si(spec, "gas_density", gas_density)
    liquid_heat_capacity_si = (
        None
        if liquid_heat_capacity is None
        else input_to_si(spec, "liquid_heat_capacity", liquid_heat_capacity)
    )
    orifice_area_si = (
        None if orifice_area is None else input_to_si(spec, "orifice_area", orifice_area)
    )

    apply_checks(
        checks.on_input,
        {
            "set_pressure": set_pressure_si,
            "overpressure_fraction": overpressure_fraction,
            "back_pressure": back_pressure_si,
            "mass_flow": mass_flow_si,
            "temperature": temperature_si,
            "molar_mass": molar_mass_si,
            "compressibility": compressibility,
            "specific_heat_ratio": specific_heat_ratio,
            "volume_flow": volume_flow_si,
            "liquid_density": liquid_density_si,
            "viscosity": viscosity_si,
            "gas_fraction": gas_fraction,
            "gas_density": gas_density_si,
            "latent_heat": latent_heat,
            "liquid_heat_capacity": liquid_heat_capacity_si,
            "orifice_area": orifice_area_si,
            "capacity_discharge_coefficient": capacity_discharge_coefficient,
        }.get,
        warnings,
    )

    if service not in _SERVICES:
        raise InvalidInputError(
            "service", f"the service is `gas`, `liquid` or `two_phase`, and {service!r} is not"
        )
    _require_finite("set_pressure", set_pressure_si)
    _require_finite("overpressure_fraction", overpressure_fraction)
    _require_finite("back_pressure", back_pressure_si)
    if set_pressure_si < 0.0 or overpressure_fraction < 0.0 or back_pressure_si < 0.0:
        raise InvalidInputError(
            "set_pressure", "a finite non-negative value is required for each pressure input"
        )
    relieving_pressure = set_pressure_si * (1.0 + overpressure_fraction)
    if back_pressure_si >= relieving_pressure:
        raise InvalidInputError(
            "back_pressure",
            f"a back pressure below the relieving pressure {relieving_pressure} Pa is "
            f"required, and {back_pressure_si} Pa is not",
        )
    balanced_bellows = bool(is_balanced_bellows)
    rupture_disk = bool(has_rupture_disk)

    if service == "gas":
        _refuse(volume_flow_si, "volume_flow", service)
        _refuse(viscosity_si, "viscosity", service)
        _refuse(gas_fraction, "gas_fraction", service)
        _refuse(gas_density_si, "gas_density", service)
        _refuse(latent_heat, "latent_heat", service)
        _refuse(liquid_heat_capacity_si, "liquid_heat_capacity", service)
        _refuse(liquid_density_si, "liquid_density", service)
        mass_flow_value = _present(mass_flow_si, "mass_flow", service)
        temperature_value = _present(temperature_si, "temperature", service)
        molar_mass_value = _present(molar_mass_si, "molar_mass", service)
        compressibility_value = _present(compressibility, "compressibility", service)
        specific_heat_ratio_value = _present(specific_heat_ratio, "specific_heat_ratio", service)
        gas = _gas_area(
            mass_flow_value,
            set_pressure_si,
            overpressure_fraction,
            back_pressure_si,
            temperature_value,
            molar_mass_value,
            compressibility_value,
            specific_heat_ratio_value,
            balanced_bellows,
            rupture_disk,
        )
        if orifice_area_si is not None and capacity_discharge_coefficient is None:
            raise InvalidInputError(
                "capacity_discharge_coefficient", "is required with orifice_area"
            )
        if orifice_area_si is None and capacity_discharge_coefficient is not None:
            raise InvalidInputError(
                "orifice_area", "is required with capacity_discharge_coefficient"
            )
        capacity_value = (
            None
            if orifice_area_si is None
            else _capacity(
                orifice_area_si,
                set_pressure_si,
                overpressure_fraction,
                back_pressure_si,
                temperature_value,
                molar_mass_value,
                compressibility_value,
                specific_heat_ratio_value,
                float(capacity_discharge_coefficient),
            )
        )
        return PsvSizingResult(
            relieving_pressure=from_si(relieving_pressure, "Pa"),
            back_pressure_fraction=float(gas["back_pressure_fraction"]),
            discharge_coefficient=0.975,
            required_area=from_si(float(gas["required_area"]), "m**2"),
            required_area_in2=float(gas["required_area_in2"]),
            recommended_orifice=str(gas["recommended_orifice"]),
            selected_area_in2=float(gas["selected_area_in2"]),
            back_pressure_correction=float(gas["back_pressure_correction"]),
            combination_correction=float(gas["combination_correction"]),
            viscosity_correction=None,
            mass_flow_rate=None,
            mass_flow_capacity=(
                None if capacity_value is None else from_si(capacity_value, "kg/s")
            ),
            warnings=tuple(warnings),
        )

    if service == "liquid":
        if rupture_disk:
            raise InvalidInputError("has_rupture_disk", "is not read by the liquid service")
        _refuse(mass_flow_si, "mass_flow", service)
        _refuse(temperature_si, "temperature", service)
        _refuse(molar_mass_si, "molar_mass", service)
        _refuse(compressibility, "compressibility", service)
        _refuse(specific_heat_ratio, "specific_heat_ratio", service)
        _refuse(gas_fraction, "gas_fraction", service)
        _refuse(gas_density_si, "gas_density", service)
        _refuse(latent_heat, "latent_heat", service)
        _refuse(liquid_heat_capacity_si, "liquid_heat_capacity", service)
        _refuse(orifice_area_si, "orifice_area", service)
        _refuse(capacity_discharge_coefficient, "capacity_discharge_coefficient", service)
        volume_flow_value = _present(volume_flow_si, "volume_flow", service)
        liquid_density_value = _present(liquid_density_si, "liquid_density", service)
        viscosity_value = _present(viscosity_si, "viscosity", service)
        liquid = _liquid_area(
            volume_flow_value,
            liquid_density_value,
            set_pressure_si,
            overpressure_fraction,
            back_pressure_si,
            viscosity_value,
            balanced_bellows,
        )
        return PsvSizingResult(
            relieving_pressure=from_si(relieving_pressure, "Pa"),
            back_pressure_fraction=float(liquid["back_pressure_fraction"]),
            discharge_coefficient=0.65,
            required_area=from_si(float(liquid["required_area"]), "m**2"),
            required_area_in2=float(liquid["required_area_in2"]),
            recommended_orifice=str(liquid["recommended_orifice"]),
            selected_area_in2=float(liquid["selected_area_in2"]),
            back_pressure_correction=float(liquid["back_pressure_correction"]),
            combination_correction=None,
            viscosity_correction=float(liquid["viscosity_correction"]),
            mass_flow_rate=from_si(float(liquid["mass_flow"]), "kg/s"),
            mass_flow_capacity=None,
            warnings=tuple(warnings),
        )

    if balanced_bellows:
        raise InvalidInputError("is_balanced_bellows", "is not read by the two_phase service")
    if rupture_disk:
        raise InvalidInputError("has_rupture_disk", "is not read by the two_phase service")
    _refuse(volume_flow_si, "volume_flow", service)
    _refuse(viscosity_si, "viscosity", service)
    _refuse(molar_mass_si, "molar_mass", service)
    _refuse(compressibility, "compressibility", service)
    _refuse(specific_heat_ratio, "specific_heat_ratio", service)
    _refuse(orifice_area_si, "orifice_area", service)
    _refuse(capacity_discharge_coefficient, "capacity_discharge_coefficient", service)
    mass_flow_value = _present(mass_flow_si, "mass_flow", service)
    temperature_value = _present(temperature_si, "temperature", service)
    liquid_density_value = _present(liquid_density_si, "liquid_density", service)
    gas_fraction_value = _present(gas_fraction, "gas_fraction", service)
    gas_density_value = _present(gas_density_si, "gas_density", service)
    latent_heat_value = _present(latent_heat, "latent_heat", service)
    liquid_heat_capacity_value = _present(liquid_heat_capacity_si, "liquid_heat_capacity", service)
    area = _two_phase_area(
        mass_flow_value,
        set_pressure_si,
        overpressure_fraction,
        back_pressure_si,
        temperature_value,
        gas_fraction_value,
        gas_density_value,
        liquid_density_value,
        latent_heat_value,
        liquid_heat_capacity_value,
    )
    return PsvSizingResult(
        relieving_pressure=from_si(relieving_pressure, "Pa"),
        back_pressure_fraction=back_pressure_si / relieving_pressure,
        discharge_coefficient=0.85,
        required_area=from_si(area, "m**2"),
        required_area_in2=None,
        recommended_orifice=None,
        selected_area_in2=None,
        back_pressure_correction=None,
        combination_correction=None,
        viscosity_correction=None,
        mass_flow_rate=None,
        mass_flow_capacity=None,
        warnings=tuple(warnings),
    )
