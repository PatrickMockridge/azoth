"""``process.non_equilibrium_blowdown`` - the two-zone fire-exposed blowdown.

As NeqSim's ``NonEquilibriumBlowdownModel.run`` computes it. Spec:
``specs/models/process/non_equilibrium_blowdown.toml``

The Python twin of ``crates/azoth-process/src/models/non_equilibrium_blowdown.rs``: the
vapour and liquid zones, the vapour-controlled pressure, the wetted-wall boiling, the
interfacial coupling and the boil-off transfer.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.result import NonEquilibriumBlowdownPoint, NonEquilibriumBlowdownResult
from azoth.core.units import Q, from_si, input_to_si, quantity
from azoth.core.warnings import Warning
from azoth.eos import components as _components
from azoth.eos.reference._entropy_state import molar_mass_of
from azoth.eos.reference._heat_capacity import phase_capacity
from azoth.eos.reference._mixture_state import phase_state, reduced_parameters
from azoth.eos.reference.molar_enthalpy_entropy import molar_enthalpy_entropy
from azoth.eos.reference.ph_flash import ph_flash
from azoth.eos.reference.pr_molar_volume import MOLAR_GAS_CONSTANT
from azoth.eos.reference.pt_flash import check_composition, pt_flash
from azoth.eos.reference.thermal_conductivity import thermal_conductivity
from azoth.eos.reference.tv_flash import tv_flash
from azoth.eos.reference.viscosity import viscosity

MODEL_ID = "process.non_equilibrium_blowdown"

#: The class's gas constant, J/(mol K).
GAS_CONSTANT = 8.314
#: Stefan-Boltzmann constant, W/(m2 K4).
STEFAN_BOLTZMANN = 5.670374419e-8
#: Standard gravity, m/s2.
GRAVITY = 9.80665
#: The class's zone temperature floor, K.
MIN_ZONE_TEMPERATURE = 100.0
#: The Rohsenow Prandtl exponent.
ROHSENOW_PRANDTL_EXPONENT = 1.7


class _Solution(NamedTuple):
    points: list[tuple[float, float, float, float, float, float, float]]
    initial_pressure: float
    flash_fallback_count: float
    final_gas_temperature: float
    final_liquid_temperature: float
    max_temperature_bifurcation: float
    recorded_points: float


class _Phase(NamedTuple):
    molar_mass: float
    density: float
    cp: float
    cv: float


class _Zone(NamedTuple):
    molar_mass: float
    density: float
    cp_mass: float
    gamma: float


def _phase_props(
    mixture: Any,
    ideal_gas: Any,
    reduced: Any,
    kij: Any,
    t: float,
    p: float,
    composition: list[float],
    root: float,
) -> _Phase:
    molar_mass = molar_mass_of(mixture, composition)
    shift = mixture.volume_shift(composition)
    molar_volume = root * MOLAR_GAS_CONSTANT * t / p - shift
    if not molar_volume > 0.0 or not math.isfinite(molar_volume):
        raise InvalidInputError("initial_p", f"the corrected molar volume is {molar_volume} m3/mol")
    capacity = phase_capacity(mixture, ideal_gas, reduced, kij, t, p, composition, root)
    return _Phase(
        molar_mass=molar_mass,
        density=molar_mass / molar_volume,
        cp=capacity.cp,
        cv=capacity.cv,
    )


def _zone_props(mixture: Any, ideal_gas: Any, t: float, p: float, z: list[float]) -> _Zone:
    reduced = reduced_parameters(mixture, t, p)
    kij = mixture.effective_kij(reduced.t_kelvin)
    flash = pt_flash(mixture, quantity(t, "K"), quantity(p, "Pa"), z)
    if flash.phase == "two_phase":
        beta = flash.vapour_fraction
        if beta is None:
            raise InvalidInputError("initial_p", "the flash reported no vapour fraction")
        vapour = _phase_props(mixture, ideal_gas, reduced, kij, t, p, list(flash.y), flash.z_vapour)
        liquid = _phase_props(mixture, ideal_gas, reduced, kij, t, p, list(flash.x), flash.z_liquid)
        mass = beta * vapour.molar_mass + (1.0 - beta) * liquid.molar_mass
        volume = beta * (vapour.molar_mass / vapour.density) + (1.0 - beta) * (
            liquid.molar_mass / liquid.density
        )
        cp = beta * vapour.cp + (1.0 - beta) * liquid.cp
        cv = beta * vapour.cv + (1.0 - beta) * liquid.cv
        return _Zone(molar_mass=mass, density=mass / volume, cp_mass=cp / mass, gamma=cp / cv)
    if flash.phase == "trivial":
        raise InvalidInputError("initial_p", "the zone flash found a trivial solution")
    root = phase_state(reduced, kij, z, liquid=flash.phase != "all_vapour").z
    phase = _phase_props(mixture, ideal_gas, reduced, kij, t, p, z, root)
    return _Zone(
        molar_mass=phase.molar_mass,
        density=phase.density,
        cp_mass=phase.cp / phase.molar_mass,
        gamma=phase.cp / phase.cv,
    )


def _rohsenow(
    liquid_viscosity: float,
    latent_heat: float,
    liquid_density: float,
    vapour_density: float,
    surface_tension: float,
    liquid_cp: float,
    superheat: float,
    prandtl: float,
    surface_constant: float,
) -> float | None:
    if (
        liquid_viscosity <= 0.0
        or latent_heat <= 0.0
        or liquid_density <= 0.0
        or surface_tension <= 0.0
        or liquid_cp <= 0.0
        or surface_constant <= 0.0
        or prandtl <= 0.0
        or vapour_density < 0.0
        or vapour_density >= liquid_density
    ):
        return None
    if superheat <= 0.0:
        return 0.0
    buoyancy = math.sqrt(GRAVITY * (liquid_density - vapour_density) / surface_tension)
    bracket = (
        liquid_cp
        * superheat
        / (surface_constant * latent_heat * prandtl**ROHSENOW_PRANDTL_EXPONENT)
    )
    flux = liquid_viscosity * latent_heat * buoyancy * bracket**3
    return flux if math.isfinite(flux) else None


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _evaluate(
    components: list[str],
    initial_z: list[float],
    initial_t: Q,
    initial_p: Q,
    vessel_volume: Q,
    orifice_diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    fire_emissivity: float | None = None,
    fire_flame_temperature: Q | None = None,
    fire_convective_coefficient: Q | None = None,
    fire_exposed_area: Q | None = None,
    wall_mass: Q | None = None,
    wall_specific_heat: Q | None = None,
    inside_gas_coefficient: Q | None = None,
    interfacial_coefficient: Q | None = None,
    interfacial_area: Q | None = None,
    latent_heat: float | None = None,
    surface_tension: float | None = None,
    surface_fluid_constant: float | None = None,
    critical_heat_flux: float | None = None,
    effective_boiling_coefficient: Q | None = None,
    time_step: Q | None = None,
    max_time: Q | None = None,
    stop_pressure: Q | None = None,
) -> tuple[_Solution, list[Warning]]:
    spec = _spec()
    warnings: list[Warning] = []
    initial_t_si = input_to_si(spec, "initial_t", initial_t)
    initial_p_si = input_to_si(spec, "initial_p", initial_p)
    volume = input_to_si(spec, "vessel_volume", vessel_volume)
    diameter = input_to_si(spec, "orifice_diameter", orifice_diameter)
    back = input_to_si(spec, "back_pressure", back_pressure)
    coefficient = float(discharge_coefficient)
    emissivity = 0.9 if fire_emissivity is None else float(fire_emissivity)
    flame = (
        1100.0
        if fire_flame_temperature is None
        else input_to_si(spec, "fire_flame_temperature", fire_flame_temperature)
    )
    convection = (
        30.0
        if fire_convective_coefficient is None
        else input_to_si(spec, "fire_convective_coefficient", fire_convective_coefficient)
    )
    exposed = (
        0.0
        if fire_exposed_area is None
        else input_to_si(spec, "fire_exposed_area", fire_exposed_area)
    )
    metal = 0.0 if wall_mass is None else input_to_si(spec, "wall_mass", wall_mass)
    wall_cp = (
        470.0
        if wall_specific_heat is None
        else input_to_si(spec, "wall_specific_heat", wall_specific_heat)
    )
    inside = (
        30.0
        if inside_gas_coefficient is None
        else input_to_si(spec, "inside_gas_coefficient", inside_gas_coefficient)
    )
    interface_coefficient = (
        200.0
        if interfacial_coefficient is None
        else input_to_si(spec, "interfacial_coefficient", interfacial_coefficient)
    )
    interface_area = (
        0.0 if interfacial_area is None else input_to_si(spec, "interfacial_area", interfacial_area)
    )
    latent = 350000.0 if latent_heat is None else float(latent_heat)
    sigma = 0.015 if surface_tension is None else float(surface_tension)
    csf = 0.011 if surface_fluid_constant is None else float(surface_fluid_constant)
    chf = 1.0e6 if critical_heat_flux is None else float(critical_heat_flux)
    boiling = (
        5000.0
        if effective_boiling_coefficient is None
        else input_to_si(spec, "effective_boiling_coefficient", effective_boiling_coefficient)
    )
    step = 1.0 if time_step is None else input_to_si(spec, "time_step", time_step)
    span = 900.0 if max_time is None else input_to_si(spec, "max_time", max_time)
    stop = 1.5e5 if stop_pressure is None else input_to_si(spec, "stop_pressure", stop_pressure)

    for name, value in (
        ("vessel_volume", volume),
        ("orifice_diameter", diameter),
        ("discharge_coefficient", coefficient),
        ("back_pressure", back),
        ("fire_emissivity", emissivity),
        ("fire_flame_temperature", flame),
        ("fire_convective_coefficient", convection),
        ("fire_exposed_area", exposed),
        ("wall_mass", metal),
        ("wall_specific_heat", wall_cp),
        ("inside_gas_coefficient", inside),
        ("interfacial_coefficient", interface_coefficient),
        ("interfacial_area", interface_area),
        ("latent_heat", latent),
        ("surface_tension", sigma),
        ("surface_fluid_constant", csf),
        ("critical_heat_flux", chf),
        ("effective_boiling_coefficient", boiling),
        ("time_step", step),
        ("max_time", span),
        ("stop_pressure", stop),
    ):
        if not math.isfinite(value):
            raise InvalidInputError(
                name, "a non-finite value is not a number this calculation publishes"
            )
    if volume <= 0.0 or diameter <= 0.0 or initial_t_si <= 0.0 or initial_p_si <= 0.0:
        raise InvalidInputError(
            "vessel_volume",
            "the class requires the volume, orifice and initial state to be positive",
        )
    if coefficient <= 0.0 or step <= 0.0 or span <= 0.0 or stop <= 0.0:
        raise InvalidInputError(
            "discharge_coefficient",
            "the coefficient, step, horizon and stop pressure must be positive",
        )

    mixture, ideal_gas = _components.mixture_of([str(name) for name in components], eos="pr")
    check_composition([float(value) for value in initial_z], len(mixture.components), "initial_z")
    feed = [float(value) for value in initial_z]
    flash = pt_flash(mixture, quantity(initial_t_si, "K"), quantity(initial_p_si, "Pa"), feed)
    if flash.phase != "two_phase" or flash.vapour_fraction is None:
        raise InvalidInputError(
            "initial_p",
            "the class requires an initial coexisting vapour and liquid phase",
        )
    beta = flash.vapour_fraction
    gas_composition = list(flash.y)
    liquid_composition = list(flash.x)
    # **The class charges by the EOS volume**: ``getVolume("m3")`` sums each phase's
    # ``Z R T / P``, not the physical-properties density, and the inventory is scaled so
    # that volume fills the vessel.
    eos_molar_volume = (
        beta * flash.z_vapour * MOLAR_GAS_CONSTANT * initial_t_si / initial_p_si
        + (1.0 - beta) * flash.z_liquid * MOLAR_GAS_CONSTANT * initial_t_si / initial_p_si
    )
    if not eos_molar_volume > 0.0 or not math.isfinite(eos_molar_volume):
        raise InvalidInputError(
            "initial_p", f"the EOS molar volume of the feed is {eos_molar_volume} m3/mol"
        )
    total_moles = volume / eos_molar_volume
    gas_moles = beta * total_moles
    liquid_moles = (1.0 - beta) * total_moles
    gas_temperature = initial_t_si
    liquid_temperature = initial_t_si
    wall_temperature = initial_t_si
    pressure = initial_p_si
    eff_interface = interface_area if interface_area > 0.0 else volume ** (2.0 / 3.0)
    eff_external = exposed if exposed > 0.0 else 4.0 * volume ** (2.0 / 3.0)
    eff_wall_mass = metal if metal > 0.0 else 1.0
    area = math.pi * 0.25 * diameter * diameter
    fallbacks = 0.0
    time = 0.0
    points: list[tuple[float, float, float, float, float, float, float]] = []

    def record() -> None:
        points.append(
            (
                time,
                pressure,
                gas_temperature,
                liquid_temperature,
                wall_temperature,
                gas_moles * molar_mass_of(mixture, gas_composition),
                liquid_moles * molar_mass_of(mixture, liquid_composition),
            )
        )

    record()
    while time < span and pressure > stop and gas_moles > 0.0:
        try:
            gas = _zone_props(mixture, ideal_gas, gas_temperature, pressure, gas_composition)
            zone_liquid = _zone_props(
                mixture, ideal_gas, liquid_temperature, pressure, liquid_composition
            )
        except Exception:  # the class's guarded zone flash
            fallbacks += 1.0
            break
        # The class reads the compressibility from the physical-properties density at the
        # pressure the step started from, and uses it in both the rate and the flow work.
        if gas.density > 0.0 and gas.molar_mass > 0.0:
            z_gas = pressure * gas.molar_mass / (gas.density * GAS_CONSTANT * gas_temperature)
            if not math.isfinite(z_gas) or z_gas <= 0.0:
                z_gas = 1.0
        else:
            z_gas = 1.0
        gas_mass = gas_moles * gas.molar_mass
        liquid_mass = liquid_moles * zone_liquid.molar_mass
        liquid_volume = liquid_mass / zone_liquid.density if zone_liquid.density > 0.0 else 0.0
        gas_volume = max(volume - liquid_volume, 1.0e-6)
        molar_volume = gas_volume / gas_moles
        try:
            tv = tv_flash(
                mixture,
                ideal_gas,
                quantity(gas_temperature, "K"),
                quantity(molar_volume, "m**3/mol"),
                gas_composition,
            )
            pressure = max(tv.P.to("Pa").magnitude, back)
        except Exception:  # the class's guarded zone flash
            fallbacks += 1.0
            break

        critical_ratio = (2.0 / (gas.gamma + 1.0)) ** (gas.gamma / (gas.gamma - 1.0))
        pressure_ratio = back / pressure
        if pressure_ratio <= critical_ratio:
            rate = (
                coefficient
                * area
                * pressure
                * math.sqrt(gas.gamma * gas.molar_mass / (z_gas * GAS_CONSTANT * gas_temperature))
                * (2.0 / (gas.gamma + 1.0)) ** ((gas.gamma + 1.0) / (2.0 * (gas.gamma - 1.0)))
            )
        else:
            term = (2.0 * gas.gamma / (gas.gamma - 1.0)) * (
                pressure_ratio ** (2.0 / gas.gamma)
                - pressure_ratio ** ((gas.gamma + 1.0) / gas.gamma)
            )
            if term < 0.0:
                term = 0.0
            rate = (
                coefficient
                * area
                * pressure
                * math.sqrt(gas.molar_mass / (z_gas * GAS_CONSTANT * gas_temperature))
                * math.sqrt(term)
            )
        if rate < 0.0 or math.isnan(rate):
            rate = 0.0
        dm_out = min(rate * step, gas_mass)

        radiative = emissivity * STEFAN_BOLTZMANN * (flame**4 - wall_temperature**4)
        convective = convection * (flame - wall_temperature)
        fire_heat = max(radiative + convective, 0.0) * eff_external

        wetted = min(max(liquid_volume / volume, 0.0), 1.0)
        wall_to_gas = inside * eff_external * (1.0 - wetted) * (wall_temperature - gas_temperature)
        superheat = wall_temperature - liquid_temperature
        if superheat > 0.0:
            try:
                mu = float(
                    viscosity(
                        mixture,
                        quantity(liquid_temperature, "K"),
                        quantity(pressure, "Pa"),
                        liquid_composition,
                    )
                    .mu.to("Pa*s")
                    .magnitude
                )
                k = float(
                    thermal_conductivity(
                        mixture,
                        ideal_gas,
                        quantity(liquid_temperature, "K"),
                        quantity(pressure, "Pa"),
                        liquid_composition,
                    )
                    .k.to("W/(m*K)")
                    .magnitude
                )
                prandtl = mu * zone_liquid.cp_mass / k if k > 0.0 and mu > 0.0 else 5.0
                boiling_flux = _rohsenow(
                    mu,
                    latent,
                    zone_liquid.density,
                    gas.density,
                    sigma,
                    zone_liquid.cp_mass,
                    superheat,
                    prandtl,
                    csf,
                )
                if boiling_flux is None:
                    boiling_flux = boiling * superheat
            except Exception:  # a missing property falls back to the coefficient
                boiling_flux = boiling * superheat
        else:
            boiling_flux = 0.0
        boiling_flux = min(boiling_flux, chf)
        wall_to_liquid = boiling_flux * eff_external * wetted
        interface_heat = (
            interface_coefficient * eff_interface * (gas_temperature - liquid_temperature)
        )
        dm_vap = min(max(wall_to_liquid, 0.0) * step / latent, 0.5 * liquid_mass)

        wall_temperature += (
            (fire_heat - wall_to_gas - wall_to_liquid) * step / (eff_wall_mass * wall_cp)
        )
        wall_temperature = min(max(wall_temperature, MIN_ZONE_TEMPERATURE), flame)
        if gas_mass > 1.0e-9 and gas.cp_mass > 0.0:
            # **A fixed-volume open system**: ``m cv dT = Q - (h - u) dm_out``, with
            # ``h - u = Z R T / M``; without the flow work the vapour does not cool on
            # depressurisation at all.
            cv_gas_mass = gas.cp_mass / max(gas.gamma, 1.0)
            flow_work = z_gas * GAS_CONSTANT * gas_temperature / gas.molar_mass * dm_out
            d_u = (
                (wall_to_gas - interface_heat) * step
                + dm_vap * cv_gas_mass * (liquid_temperature - gas_temperature)
                - flow_work
            )
            gas_temperature += d_u / (gas_mass * cv_gas_mass)
        gas_temperature = min(max(gas_temperature, MIN_ZONE_TEMPERATURE), wall_temperature)
        if liquid_mass > 1.0e-9 and zone_liquid.cp_mass > 0.0:
            d_u = (interface_heat + wall_to_liquid) * step - dm_vap * latent
            liquid_temperature += d_u / (liquid_mass * zone_liquid.cp_mass)
        liquid_temperature = min(
            max(liquid_temperature, MIN_ZONE_TEMPERATURE),
            max(wall_temperature, gas_temperature),
        )

        if gas_mass > 0.0:
            gas_moles *= (gas_mass - dm_out) / gas_mass
        if dm_vap > 0.0 and liquid_moles > 0.0:
            moved = min(dm_vap / zone_liquid.molar_mass, liquid_moles)
            gas_total = gas_moles + moved
            if gas_total > 0.0:
                gas_composition = [
                    (gas_moles * gas_component + moved * liquid_component) / gas_total
                    for gas_component, liquid_component in zip(
                        gas_composition, liquid_composition, strict=True
                    )
                ]
            gas_moles = gas_total
            liquid_moles -= moved
        # **Flash boil-off**: liquid left superheated by the pressure drop flashes
        # isenthalpically at the new pressure, and the vapour it forms moves to the vapour
        # zone, cooling the liquid. Without this the liquid zone only boiled from wall heat
        # and stayed at its initial pressure-equilibrium state.
        liquid_mass_now = liquid_moles * molar_mass_of(mixture, liquid_composition)
        if liquid_moles > 0.0 and liquid_mass_now > 1.0e-6:
            try:
                # **The liquid system is a single oil phase, so its enthalpy is the liquid
                # root's, not the equilibrium split's**: the class's ``TPflash`` on the
                # phase system leaves one phase, and the PH flash is what creates the
                # vapour.
                reduced = reduced_parameters(mixture, liquid_temperature, pressure)
                kij = mixture.effective_kij(reduced.t_kelvin)
                root = phase_state(reduced, kij, liquid_composition, liquid=True).z
                enthalpy = float(
                    molar_enthalpy_entropy(
                        mixture,
                        ideal_gas,
                        quantity(liquid_temperature, "K"),
                        quantity(pressure, "Pa"),
                        liquid_composition,
                        root,
                    )
                    .h.to("J/mol")
                    .magnitude
                )
                solution = ph_flash(
                    mixture,
                    ideal_gas,
                    quantity(pressure, "Pa"),
                    quantity(enthalpy, "J/mol"),
                    liquid_composition,
                )
                liquid_temperature = solution.T.to("K").magnitude
                if solution.phase == "two_phase":
                    split = 0.0 if solution.vapour_fraction is None else solution.vapour_fraction
                    moved_moles = split * liquid_moles
                    vapour_mass = moved_moles * molar_mass_of(mixture, list(solution.y))
                    gas_mass_before = gas_moles * molar_mass_of(mixture, gas_composition)
                    gas_total = gas_moles + moved_moles
                    if gas_total > 0.0:
                        gas_composition = [
                            (gas_moles * gas_component + moved_moles * vapour_component) / gas_total
                            for gas_component, vapour_component in zip(
                                gas_composition, solution.y, strict=True
                            )
                        ]
                    gas_moles = gas_total
                    liquid_moles *= 1.0 - split
                    liquid_composition = list(solution.x)
                    if vapour_mass > 0.0:
                        mixed = gas_mass_before + vapour_mass
                        gas_temperature = (
                            gas_mass_before * gas_temperature + vapour_mass * liquid_temperature
                        ) / mixed
            except Exception:  # the class's guarded flash boil-off
                fallbacks += 1.0
        time += step
        record()

    max_bifurcation = 0.0
    for point in points:
        max_bifurcation = max(max_bifurcation, point[2] - point[3])
    return (
        _Solution(
            points=points,
            initial_pressure=initial_p_si,
            flash_fallback_count=fallbacks,
            final_gas_temperature=gas_temperature,
            final_liquid_temperature=liquid_temperature,
            max_temperature_bifurcation=max_bifurcation,
            recorded_points=float(len(points)),
        ),
        warnings,
    )


def non_equilibrium_blowdown(
    components: list[str],
    initial_z: list[float],
    initial_t: Q,
    initial_p: Q,
    vessel_volume: Q,
    orifice_diameter: Q,
    discharge_coefficient: float,
    back_pressure: Q,
    fire_emissivity: float | None = None,
    fire_flame_temperature: Q | None = None,
    fire_convective_coefficient: Q | None = None,
    fire_exposed_area: Q | None = None,
    wall_mass: Q | None = None,
    wall_specific_heat: Q | None = None,
    inside_gas_coefficient: Q | None = None,
    interfacial_coefficient: Q | None = None,
    interfacial_area: Q | None = None,
    latent_heat: float | None = None,
    surface_tension: float | None = None,
    surface_fluid_constant: float | None = None,
    critical_heat_flux: float | None = None,
    effective_boiling_coefficient: Q | None = None,
    time_step: Q | None = None,
    max_time: Q | None = None,
    stop_pressure: Q | None = None,
) -> NonEquilibriumBlowdownResult:
    """The two-zone fire-exposed blowdown.

    The initial feed must split into vapour and liquid. Each zone is flashed at its own
    temperature and the shared pressure; the wetted wall drives Rohsenow boiling and the
    boil-off moves liquid into the vapour zone.

    Args:
        components: the inventory's substances.
        initial_z: the inventory's composition.
        initial_t: the inventory temperature.
        initial_p: the inventory pressure.
        vessel_volume: the vessel's internal volume.
        orifice_diameter: the blowdown opening's diameter.
        discharge_coefficient: the orifice coefficient.
        back_pressure: the receiving absolute pressure.
        fire_emissivity: the flame emissivity; absent is 0.9.
        fire_flame_temperature: the flame temperature; absent is 1100 K.
        fire_convective_coefficient: the flame convection coefficient; absent is 30.
        fire_exposed_area: the fire-exposed area; absent derives 4 V^(2/3).
        wall_mass: the lumped wall mass; absent derives 1 kg.
        wall_specific_heat: the wall's specific heat; absent is 470.
        inside_gas_coefficient: the vapour-side film coefficient; absent is 30.
        interfacial_coefficient: the interfacial coefficient; absent is 200.
        interfacial_area: the interfacial area; absent derives V^(2/3).
        latent_heat: the latent heat in J/kg; absent is 350000.
        surface_tension: the surface tension in N/m; absent is 0.015.
        surface_fluid_constant: the Rohsenow constant; absent is 0.011.
        critical_heat_flux: the critical heat flux in W/m2; absent is 1e6.
        effective_boiling_coefficient: the boiling fallback; absent is 5000.
        time_step: the explicit step; absent is 1 s.
        max_time: the horizon; absent is 900 s.
        stop_pressure: the stop pressure; absent is 1.5e5 Pa.

    Returns:
        The two-zone history and its final temperatures.

    Raises:
        InvalidInputError: for a non-positive dimension, a feed that does not split, or a
            state a zone flash cannot resolve.
    """
    solution, warnings = _evaluate(
        components,
        initial_z,
        initial_t,
        initial_p,
        vessel_volume,
        orifice_diameter,
        discharge_coefficient,
        back_pressure,
        fire_emissivity,
        fire_flame_temperature,
        fire_convective_coefficient,
        fire_exposed_area,
        wall_mass,
        wall_specific_heat,
        inside_gas_coefficient,
        interfacial_coefficient,
        interfacial_area,
        latent_heat,
        surface_tension,
        surface_fluid_constant,
        critical_heat_flux,
        effective_boiling_coefficient,
        time_step,
        max_time,
        stop_pressure,
    )
    return NonEquilibriumBlowdownResult(
        points=tuple(
            NonEquilibriumBlowdownPoint(
                time=from_si(point[0], "s"),
                pressure=from_si(point[1], "Pa"),
                gas_temperature=from_si(point[2], "K"),
                liquid_temperature=from_si(point[3], "K"),
                wall_temperature=from_si(point[4], "K"),
                gas_mass=from_si(point[5], "kg"),
                liquid_mass=from_si(point[6], "kg"),
            )
            for point in solution.points
        ),
        initial_pressure=from_si(solution.initial_pressure, "Pa"),
        flash_fallback_count=solution.flash_fallback_count,
        final_gas_temperature=from_si(solution.final_gas_temperature, "K"),
        final_liquid_temperature=from_si(solution.final_liquid_temperature, "K"),
        max_temperature_bifurcation=from_si(solution.max_temperature_bifurcation, "K"),
        recorded_points=solution.recorded_points,
        warnings=tuple(warnings),
    )
