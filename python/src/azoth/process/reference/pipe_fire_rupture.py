"""``process.pipe_fire_rupture`` - the pipe strain-rate rupture.

As NeqSim's ``PipeFireRuptureStudy`` computes it. Spec:
``specs/models/process/pipe_fire_rupture.toml``

The Python twin of ``crates/azoth-process/src/models/pipe_fire_rupture.rs``: the thick-wall
von Mises stress, the Sellars-Tegart strain rate, the accumulated-strain limit, the class's
thermal mass and the spreadsheet-style release.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple, cast

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    FluidBasis,
    PipeFireRupturePoint,
    PipeFireRuptureResult,
    RuptureStatus,
)
from azoth.core.units import (
    Q,
    from_si,
    input_records_to_si,
    input_to_si,
    quantity,
)
from azoth.core.warnings import Warning
from azoth.process.reference.blowdown_pressure_profile import _evaluate as _profile_evaluate
from azoth.process.reference.blowdown_pressure_profile import _lookup as _profile_lookup
from azoth.process.reference.pipe_fire_rupture_material import _evaluate as _material

MODEL_ID = "process.pipe_fire_rupture"

_PA_PER_BAR = 1.0e5
_UNIVERSAL_GAS_CONSTANT_J_PER_KMOLK = 8314.0
_GAS_RELEASE_COEFFICIENT = 0.00527
_SHORT_PIPE_GAS_MULTIPLIER = 1.541
_LIQUID_DISCHARGE_COEFFICIENT = 0.62
_LIQUID_FLOW_CONSTANT = 11.78
_GAS_LIQUID_DENSITY_SWITCH_KG_PER_M3 = 500.0
_DEFAULT_TIME_STEP_SECONDS = 5.0
_DEFAULT_MAX_TIME_SECONDS = 3600.0
_KELVIN_OFFSET = 273.15
_CLASS_OFFSET = 273.0
_STEFAN_BOLTZMANN_W_PER_M2K4 = 5.67e-8
_STRAIN_CAP = 2.0


class _Scenario(NamedTuple):
    """One fire exposure, in the class's Celsius and SI values."""

    fire_c: float
    gas_c: float
    h: float
    fire_emissivity: float
    metal_emissivity: float
    metal_absorptivity: float
    pfp: float


class _Solution(NamedTuple):
    """The march's summary and its history, in SI."""

    status: str
    fluid_basis: str
    rupture_time: float | None
    rupture_pressure: float | None
    rupture_mean_wall_temperature: float | None
    rupture_outer_surface_temperature: float | None
    rupture_accumulated_strain: float | None
    rupture_strain_limit: float | None
    long_pipe_gas_two_sides: float | None
    long_pipe_gas_one_side: float | None
    short_pipe_gas_one_side: float | None
    liquid_one_side: float | None
    release_gas_temperature: float | None
    history: list[tuple[float, float, float, float, float, float, float, float, float]]


def _require_finite(name: str, value: float) -> None:
    """Refuse a value the arithmetic cannot carry to a published number."""
    if not math.isfinite(value):
        raise InvalidInputError(
            name, "a non-finite value is not a number this calculation publishes"
        )


def _require_positive(name: str, value: float) -> None:
    """Refuse a non-positive value."""
    _require_finite(name, value)
    if value <= 0.0:
        raise InvalidInputError(name, "the class's builder refuses a non-positive value")


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _scenario(
    fire_scenario: str,
    fire_temperature: float | None,
    gas_temperature: float | None,
    convective: float | None,
    fire_emissivity: float | None,
    metal_emissivity: float | None,
    metal_absorptivity: float | None,
    pfp: float | None,
) -> _Scenario:
    """The class's three factories, or the caller's seven."""
    built_in = {
        "large_jet_fire": (1155.0, 1155.0, 100.0),
        "small_jet_fire": (1000.0, 1000.0, 100.0),
        "pool_fire": (1125.0, 1125.0, 30.0),
    }
    explicit = fire_scenario == "explicit"
    if fire_scenario not in (*built_in, "explicit"):
        raise InvalidInputError(
            "fire_scenario",
            f"`{fire_scenario}` is not `large_jet_fire`, `small_jet_fire`, `pool_fire` or "
            "`explicit`",
        )
    if not explicit and any(
        value is not None
        for value in (
            fire_temperature,
            gas_temperature,
            convective,
            fire_emissivity,
            metal_emissivity,
            metal_absorptivity,
        )
    ):
        raise InvalidInputError(
            "fire_scenario",
            "the three built-in scenarios carry their own temperature and emissivity group",
        )
    if explicit and any(
        value is None
        for value in (
            fire_temperature,
            gas_temperature,
            convective,
            fire_emissivity,
            metal_emissivity,
            metal_absorptivity,
        )
    ):
        raise InvalidInputError(
            "fire_scenario",
            "the `explicit` scenario requires all six temperature and emissivity inputs",
        )
    if explicit:
        fire_c = cast(float, fire_temperature) - _KELVIN_OFFSET
        gas_c = cast(float, gas_temperature) - _KELVIN_OFFSET
        h = cast(float, convective)
    else:
        fire_c, gas_c, h = built_in[fire_scenario]
    fire_e = 1.0 if fire_emissivity is None else fire_emissivity
    metal_e = 0.85 if metal_emissivity is None else metal_emissivity
    metal_a = 0.85 if metal_absorptivity is None else metal_absorptivity
    pfp_value = 1.0 if pfp is None else pfp
    for value in (fire_c, gas_c, h, fire_e, metal_e, metal_a, pfp_value):
        _require_finite("fire_scenario", value)
    if fire_c + _KELVIN_OFFSET <= 0.0 or gas_c + _KELVIN_OFFSET <= 0.0:
        raise InvalidInputError(
            "fire_scenario", "a non-positive absolute fire or gas temperature is not one"
        )
    for value in (fire_e, metal_e, metal_a, pfp_value):
        if not 0.0 <= value <= 1.0:
            raise InvalidInputError("fire_scenario", "the class's fractions are in [0, 1]")
    if h < 0.0:
        raise InvalidInputError(
            "convective_heat_transfer_coefficient",
            "the class refuses a negative coefficient",
        )
    return _Scenario(fire_c, gas_c, h, fire_e, metal_e, metal_a, pfp_value)


def _heat_flux_kw(scenario: _Scenario, outer_c: float) -> float:
    """The class's heat flux into the outer surface, kW/m2."""
    radiative = _STEFAN_BOLTZMANN_W_PER_M2K4 * (
        scenario.fire_emissivity
        * scenario.metal_absorptivity
        * (scenario.fire_c + _CLASS_OFFSET) ** 4
        - scenario.metal_emissivity * (outer_c + _CLASS_OFFSET) ** 4
    )
    convective = scenario.h * (scenario.gas_c - outer_c)
    return max(0.0, (radiative + convective) * scenario.pfp) / 1000.0


def _von_mises_mpa(
    pressure_barg: float, outside_diameter: float, inside_diameter: float, weight: float
) -> float:
    """The class's thick-wall von Mises stress, MPa."""
    pressure_mpa = 0.1 * max(0.0, pressure_barg)
    denominator = outside_diameter**2 - inside_diameter**2
    mean_diameter = (outside_diameter + inside_diameter) / 2.0
    inner_outer_product = inside_diameter**2 * outside_diameter**2 / mean_diameter**2
    radial = pressure_mpa * (inside_diameter**2 - inner_outer_product) / denominator
    hoop = pressure_mpa * (inside_diameter**2 + inner_outer_product) / denominator
    longitudinal = pressure_mpa * inside_diameter**2 / denominator + weight
    return math.sqrt(
        radial**2
        + hoop**2
        + longitudinal**2
        - radial * hoop
        - radial * longitudinal
        - hoop * longitudinal
    )


def _material_at(name: str, metal_c: float, stress_mpa: float) -> Any:
    """The material at one state, through the material id's own arithmetic."""
    solution, _ = _material(
        name,
        quantity(metal_c + _KELVIN_OFFSET, "K"),
        quantity(stress_mpa, "MPa"),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    )
    return solution


def _evaluate(
    outside_diameter: Q,
    nominal_wall_thickness: Q,
    material_name: str,
    fire_scenario: str,
    pressure_profile: list[dict[str, object]],
    corrosion_allowance: Q | None = None,
    wall_thickness_undertolerance: float | None = None,
    exposed_length: Q | None = None,
    weight_stress: Q | None = None,
    fluid_density: Q | None = None,
    fluid_heat_capacity: Q | None = None,
    gas_molecular_weight: Q | None = None,
    initial_temperature: Q | None = None,
    fire_temperature: Q | None = None,
    gas_temperature: Q | None = None,
    convective_heat_transfer_coefficient: Q | None = None,
    fire_emissivity: float | None = None,
    metal_emissivity: float | None = None,
    metal_absorptivity: float | None = None,
    passive_protection_heat_flux_factor: float | None = None,
    gauge_offset: Q | None = None,
    interpolation_mode: str | None = None,
    time_step: Q | None = None,
    max_time: Q | None = None,
    spreadsheet_gas_thermal_mass: bool | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    def optional(name: str, value: Q | None) -> float | None:
        return None if value is None else input_to_si(spec, name, value)

    od = input_to_si(spec, "outside_diameter", outside_diameter)
    nominal_wall = input_to_si(spec, "nominal_wall_thickness", nominal_wall_thickness)
    corrosion = optional("corrosion_allowance", corrosion_allowance)
    exposed = optional("exposed_length", exposed_length)
    weight = optional("weight_stress", weight_stress)
    density = optional("fluid_density", fluid_density)
    heat_capacity = optional("fluid_heat_capacity", fluid_heat_capacity)
    molecular_weight = optional("gas_molecular_weight", gas_molecular_weight)
    initial = optional("initial_temperature", initial_temperature)
    fire_t = optional("fire_temperature", fire_temperature)
    gas_t = optional("gas_temperature", gas_temperature)
    convective = optional(
        "convective_heat_transfer_coefficient", convective_heat_transfer_coefficient
    )
    offset = optional("gauge_offset", gauge_offset)
    step = optional("time_step", time_step)
    maximum = optional("max_time", max_time)
    apply_checks(
        checks.on_input,
        {
            "outside_diameter": od,
            "nominal_wall_thickness": nominal_wall,
            "corrosion_allowance": corrosion,
            "wall_thickness_undertolerance": wall_thickness_undertolerance,
            "exposed_length": exposed,
            "weight_stress": weight,
            "fluid_density": density,
            "fluid_heat_capacity": heat_capacity,
            "gas_molecular_weight": molecular_weight,
            "initial_temperature": initial,
            "fire_temperature": fire_t,
            "gas_temperature": gas_t,
            "convective_heat_transfer_coefficient": convective,
            "fire_emissivity": fire_emissivity,
            "metal_emissivity": metal_emissivity,
            "metal_absorptivity": metal_absorptivity,
            "passive_protection_heat_flux_factor": passive_protection_heat_flux_factor,
            "gauge_offset": offset,
            "time_step": step,
            "max_time": maximum,
        }.get,
        warnings,
    )

    _require_positive("outside_diameter", od)
    _require_positive("nominal_wall_thickness", nominal_wall)
    corrosion_value = 0.0 if corrosion is None else corrosion
    _require_finite("corrosion_allowance", corrosion_value)
    if corrosion_value < 0.0:
        raise InvalidInputError(
            "corrosion_allowance", "a negative corrosion allowance is not a thickness"
        )
    undertolerance = 0.0 if wall_thickness_undertolerance is None else wall_thickness_undertolerance
    _require_finite("wall_thickness_undertolerance", undertolerance)
    if not 0.0 <= undertolerance < 1.0:
        raise InvalidInputError(
            "wall_thickness_undertolerance",
            "the class's setter refuses a fraction outside [0, 1)",
        )
    exposed_value = 1.0 if exposed is None else exposed
    _require_positive("exposed_length", exposed_value)
    weight_mpa = 0.0 if weight is None else weight / 1.0e6
    _require_finite("weight_stress", weight_mpa)
    density_value = 1.0 if density is None else density
    _require_positive("fluid_density", density_value)
    heat_capacity_value = 2000.0 if heat_capacity is None else heat_capacity
    _require_positive("fluid_heat_capacity", heat_capacity_value)
    mw_kg_mol = 0.018 if molecular_weight is None else molecular_weight
    _require_positive("gas_molecular_weight", mw_kg_mol)
    mw = mw_kg_mol * 1000.0
    initial_k = 293.15 if initial is None else initial
    _require_positive("initial_temperature", initial_k)
    initial_c = initial_k - _KELVIN_OFFSET

    offset_bar = 1.0 if offset is None else offset / _PA_PER_BAR
    _require_finite("gauge_offset", offset_bar)
    if offset_bar < 0.0:
        raise InvalidInputError(
            "gauge_offset", "the class's profile refuses a negative gauge offset"
        )
    mode = "step_previous" if interpolation_mode is None else interpolation_mode
    if mode not in ("step_previous", "linear"):
        raise InvalidInputError(
            "interpolation_mode", f"`{mode}` is not `step_previous` or `linear`"
        )
    if not pressure_profile:
        raise InvalidInputError("pressure_profile", "the class's profile needs at least two points")
    rows: list[dict[str, float]] = input_records_to_si(spec, "pressure_profile", pressure_profile)
    _profile_evaluate(pressure_profile, quantity(0.0, "s"), mode, quantity(offset_bar, "bar"))

    scenario = _scenario(
        fire_scenario,
        fire_t,
        gas_t,
        convective,
        fire_emissivity,
        metal_emissivity,
        metal_absorptivity,
        passive_protection_heat_flux_factor,
    )

    step_seconds = _DEFAULT_TIME_STEP_SECONDS if step is None else step
    _require_positive("time_step", step_seconds)
    max_seconds = _DEFAULT_MAX_TIME_SECONDS if maximum is None else maximum
    _require_positive("max_time", max_seconds)
    step_count = math.ceil(max_seconds / step_seconds)
    cap = int(spec["algorithm"].get("max_iterations", 0))
    if step_count + 1 > cap:
        raise InvalidInputError(
            "max_time", f"{step_count} steps exceeds the {cap} this procedure carries"
        )
    spreadsheet_gas = True if spreadsheet_gas_thermal_mass is None else spreadsheet_gas_thermal_mass

    effective_wall = nominal_wall * (1.0 - undertolerance) - corrosion_value
    if effective_wall <= 0.0:
        raise InvalidInputError(
            "nominal_wall_thickness", "the effective wall thickness must be positive"
        )
    inside_diameter_m = od - 2.0 * effective_wall
    if inside_diameter_m <= 0.0:
        raise InvalidInputError(
            "outside_diameter", "the effective inside diameter must be positive"
        )
    exposed_area = math.pi * od * exposed_value
    internal_volume = math.pi / 4.0 * inside_diameter_m**2 * exposed_value

    initial_material = _material_at(material_name, initial_c, 0.0)
    metal_density = initial_material.density
    basis = (
        FluidBasis.GAS
        if density_value < _GAS_LIQUID_DENSITY_SWITCH_KG_PER_M3
        else FluidBasis.LIQUID
    )

    mean_c = initial_c
    outer_c = initial_c
    heat_flux_kw = _heat_flux_kw(scenario, outer_c)
    accumulated = 0.0
    history: list[tuple[float, float, float, float, float, float, float, float, float]] = []
    rupture: tuple[float, float, float, float, float, float] | None = None

    for step_index in range(step_count + 1):
        time_seconds = min(step_index * step_seconds, max_seconds)
        strain_factor = 1.0 + max(0.0, accumulated)
        outside = od * strain_factor
        wall = effective_wall / strain_factor
        inside = outside - 2.0 * wall
        if step_index > 0:
            current_barg = max(
                0.0, _profile_lookup(rows, time_seconds, mode == "linear") - offset_bar
            )
            previous_mean = mean_c
            previous_flux = heat_flux_kw
            previous_material = _material_at(material_name, previous_mean, 0.0)
            conductivity = previous_material.thermal_conductivity
            wall_heat_capacity = previous_material.heat_capacity
            metal_mass = metal_density * exposed_area * effective_wall
            thermal_mass = metal_mass * wall_heat_capacity
            if density_value >= _GAS_LIQUID_DENSITY_SWITCH_KG_PER_M3:
                thermal_mass += internal_volume * density_value * heat_capacity_value
            elif spreadsheet_gas:
                gas_mass = (
                    current_barg
                    * _PA_PER_BAR
                    * mw
                    / (_UNIVERSAL_GAS_CONSTANT_J_PER_KMOLK * (previous_mean + _CLASS_OFFSET))
                    * internal_volume
                )
                thermal_mass += gas_mass * heat_capacity_value
            mean_c += previous_flux * 1000.0 * exposed_area * step_seconds / thermal_mass
            outer_c = mean_c + 0.5 * previous_flux * 1000.0 * wall / conductivity
            heat_flux_kw = _heat_flux_kw(scenario, outer_c)

        pressure_barg = max(0.0, _profile_lookup(rows, time_seconds, mode == "linear") - offset_bar)
        stress_mpa = _von_mises_mpa(pressure_barg, outside, inside, weight_mpa)
        properties = _material_at(material_name, mean_c, stress_mpa)
        rate_per_minute = properties.strain_rate * 60.0
        if step_index > 0:
            accumulated = min(
                _STRAIN_CAP,
                accumulated + step_seconds * rate_per_minute / 60.0,
            )
        limit = properties.rupture_strain_limit
        history.append(
            (
                time_seconds,
                pressure_barg,
                mean_c + _KELVIN_OFFSET,
                outer_c + _KELVIN_OFFSET,
                heat_flux_kw * 1000.0,
                stress_mpa,
                rate_per_minute,
                accumulated,
                limit,
            )
        )
        if accumulated > limit:
            rupture = (
                time_seconds,
                pressure_barg,
                mean_c + _KELVIN_OFFSET,
                outer_c + _KELVIN_OFFSET,
                accumulated,
                limit,
            )
            break

    if rupture is None:
        return (
            _Solution(
                RuptureStatus.NO_RUPTURE.value,
                basis.value,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                history,
            ),
            warnings,
        )

    time_seconds, pressure_barg, mean_k, outer_k, strain, limit = rupture
    area = math.pi / 4.0 * inside_diameter_m**2
    if density_value < _GAS_LIQUID_DENSITY_SWITCH_KG_PER_M3:
        two_sides = (
            _GAS_RELEASE_COEFFICIENT
            * area
            * pressure_barg
            * _PA_PER_BAR
            * math.sqrt(mw / max(1.0, initial_c + _CLASS_OFFSET))
        )
        one_side = two_sides / 2.0
        short = one_side * _SHORT_PIPE_GAS_MULTIPLIER
        liquid = 0.0
    else:
        specific_gravity = density_value / 1000.0
        liquid = (
            (
                area
                * 1.0e6
                * _LIQUID_DISCHARGE_COEFFICIENT
                / _LIQUID_FLOW_CONSTANT
                / math.sqrt(specific_gravity / (max(pressure_barg, 1.0) * 100.0))
            )
            / 60.0
            * specific_gravity
        )
        two_sides = 0.0
        one_side = 0.0
        short = 0.0
    return (
        _Solution(
            RuptureStatus.RUPTURE.value,
            basis.value,
            time_seconds,
            pressure_barg,
            mean_k,
            outer_k,
            strain,
            limit,
            two_sides,
            one_side,
            short,
            liquid,
            initial_c + _KELVIN_OFFSET,
            history,
        ),
        warnings,
    )


def pipe_fire_rupture(
    outside_diameter: Q,
    nominal_wall_thickness: Q,
    material_name: str,
    fire_scenario: str,
    pressure_profile: list[dict[str, object]],
    corrosion_allowance: Q | None = None,
    wall_thickness_undertolerance: float | None = None,
    exposed_length: Q | None = None,
    weight_stress: Q | None = None,
    fluid_density: Q | None = None,
    fluid_heat_capacity: Q | None = None,
    gas_molecular_weight: Q | None = None,
    initial_temperature: Q | None = None,
    fire_temperature: Q | None = None,
    gas_temperature: Q | None = None,
    convective_heat_transfer_coefficient: Q | None = None,
    fire_emissivity: float | None = None,
    metal_emissivity: float | None = None,
    metal_absorptivity: float | None = None,
    passive_protection_heat_flux_factor: float | None = None,
    gauge_offset: Q | None = None,
    interpolation_mode: str | None = None,
    time_step: Q | None = None,
    max_time: Q | None = None,
    spreadsheet_gas_thermal_mass: bool | None = None,
) -> PipeFireRuptureResult:
    """The pipe fire-rupture strain-rate march.

    Args:
        outside_diameter: the pipe's outside diameter.
        nominal_wall_thickness: the nominal wall thickness.
        material_name: the material selector.
        fire_scenario: ``large_jet_fire``, ``small_jet_fire``, ``pool_fire`` or ``explicit``.
        pressure_profile: the ``{time, pressure}`` blowdown points.
        corrosion_allowance: the corrosion allowance; absent is 0.
        wall_thickness_undertolerance: the under-tolerance; absent is 0.
        exposed_length: the exposed length; absent is 1 m.
        weight_stress: the axial weight stress; absent is 0.
        fluid_density: the fluid density; absent is 1 kg/m3.
        fluid_heat_capacity: the fluid heat capacity; absent is 2000 J/(kg K).
        gas_molecular_weight: the gas molecular weight; absent is 0.018 kg/mol.
        initial_temperature: the initial temperature; absent is 293.15 K.
        fire_temperature: the explicit fire temperature.
        gas_temperature: the explicit gas temperature.
        convective_heat_transfer_coefficient: the explicit convective coefficient.
        fire_emissivity: the explicit fire emissivity.
        metal_emissivity: the explicit metal emissivity.
        metal_absorptivity: the explicit metal absorptivity.
        passive_protection_heat_flux_factor: the PFP multiplier; absent is 1.
        gauge_offset: the profile's offset; absent is 1 bar.
        interpolation_mode: ``step_previous`` (absent) or ``linear``.
        time_step: the march step; absent is 5 s.
        max_time: the simulated time; absent is 3600 s.
        spreadsheet_gas_thermal_mass: whether the gas mass enters the thermal mass;
            absent is true.

    Returns:
        The status, the fluid basis, the rupture state and release, and the history.

    Raises:
        InvalidInputError: for a non-positive geometry, fluid or temperature, an unknown
            material or scenario, a scenario group that is not all-or-none, a malformed
            profile, a non-positive step or maximum time, a step count over the cap, an
            unknown mode, or a non-finite value.
    """
    solution, warnings = _evaluate(
        outside_diameter,
        nominal_wall_thickness,
        material_name,
        fire_scenario,
        pressure_profile,
        corrosion_allowance,
        wall_thickness_undertolerance,
        exposed_length,
        weight_stress,
        fluid_density,
        fluid_heat_capacity,
        gas_molecular_weight,
        initial_temperature,
        fire_temperature,
        gas_temperature,
        convective_heat_transfer_coefficient,
        fire_emissivity,
        metal_emissivity,
        metal_absorptivity,
        passive_protection_heat_flux_factor,
        gauge_offset,
        interpolation_mode,
        time_step,
        max_time,
        spreadsheet_gas_thermal_mass,
    )
    status = RuptureStatus.RUPTURE if solution.status == "rupture" else RuptureStatus.NO_RUPTURE
    basis = FluidBasis.LIQUID if solution.fluid_basis == "liquid" else FluidBasis.GAS
    history = [
        PipeFireRupturePoint(
            time=from_si(row[0], "s"),
            pressure=from_si(row[1] * _PA_PER_BAR, "bar"),
            mean_wall_temperature=from_si(row[2], "K"),
            outer_surface_temperature=from_si(row[3], "K"),
            heat_flux=from_si(row[4], "W/m**2"),
            von_mises_stress=from_si(row[5] * 1.0e6, "MPa"),
            strain_rate=from_si(row[6] / 60.0, "1/s"),
            accumulated_strain=row[7],
            rupture_strain_limit=row[8],
        )
        for row in solution.history
    ]
    return PipeFireRuptureResult(
        status=status,
        fluid_basis=basis,
        rupture_time=(
            None if solution.rupture_time is None else from_si(solution.rupture_time, "s")
        ),
        rupture_pressure=(
            None
            if solution.rupture_pressure is None
            else from_si(solution.rupture_pressure * _PA_PER_BAR, "bar")
        ),
        rupture_mean_wall_temperature=(
            None
            if solution.rupture_mean_wall_temperature is None
            else from_si(solution.rupture_mean_wall_temperature, "K")
        ),
        rupture_outer_surface_temperature=(
            None
            if solution.rupture_outer_surface_temperature is None
            else from_si(solution.rupture_outer_surface_temperature, "K")
        ),
        rupture_accumulated_strain=solution.rupture_accumulated_strain,
        rupture_strain_limit=solution.rupture_strain_limit,
        long_pipe_gas_two_sides=(
            None
            if solution.long_pipe_gas_two_sides is None
            else from_si(solution.long_pipe_gas_two_sides, "kg/s")
        ),
        long_pipe_gas_one_side=(
            None
            if solution.long_pipe_gas_one_side is None
            else from_si(solution.long_pipe_gas_one_side, "kg/s")
        ),
        short_pipe_gas_one_side=(
            None
            if solution.short_pipe_gas_one_side is None
            else from_si(solution.short_pipe_gas_one_side, "kg/s")
        ),
        liquid_one_side=(
            None if solution.liquid_one_side is None else from_si(solution.liquid_one_side, "kg/s")
        ),
        release_gas_temperature=(
            None
            if solution.release_gas_temperature is None
            else from_si(solution.release_gas_temperature, "K")
        ),
        history=tuple(history),
        warnings=tuple(warnings),
    )
