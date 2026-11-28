"""``eos.transient_pipe`` - NeqSim's ``TransientPipe``, the isothermal drift-flux march.

Spec: ``specs/models/eos/transient_pipe.toml``.

The class divides the line into cells, carries each cell's gas and liquid mass, mixture
momentum and mixture energy, and advances them with the class's face split, an adaptive CFL
step, a drift-flux velocity split and an acoustic pressure update.  Every
``thermodynamicUpdateInterval`` steps each cell is flashed at its own pressure and the
inlet temperature, and the phase densities, viscosities, enthalpies, sound speeds and
surface tension are written back.

The Python twin of ``crates/azoth-eos/src/transient_pipe.rs``, written to mirror it line for
line.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import TransientPipeResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning
from azoth.eos.components import mixture_of
from azoth.eos.reference._heat_capacity import phase_capacity
from azoth.eos.reference._mixture_state import reduced_parameters
from azoth.eos.reference.aqueous_viscosity import aqueous_viscosity
from azoth.eos.reference.hydrate_inhibitor_wt import AQUEOUS, GAS, phase_label
from azoth.eos.reference.molar_enthalpy_entropy import molar_enthalpy_entropy
from azoth.eos.reference.parachor_mixture_surface_tension import (
    parachor_mixture_surface_tension,
)
from azoth.eos.reference.phase_transport import phase_transport
from azoth.eos.reference.tp_multiflash import tp_multiflash
from azoth.eos.reference.viscosity import viscosity

CALC_ID = "eos.transient_pipe"

#: `TransientPipe.GRAVITY`, and the same constant in `DriftFluxModel` and `FlowRegimeDetector`.
GRAVITY = 9.81
#: `TransientPipe.roughness`.
DEFAULT_ROUGHNESS = 1.0e-4
#: `TransientPipe.numberOfSections`.
DEFAULT_SECTIONS = 50
#: `TransientPipe.maxSimulationTime`.
DEFAULT_MAX_TIME = 3600.0
#: `TransientPipe.dt`, used only for the history allocation.
DEFAULT_DT = 0.1
#: `TransientPipe.cflNumber`.
DEFAULT_CFL = 0.5
#: `TransientPipe.minTimeStep`.
MIN_TIME_STEP = 1.0e-4
#: `TransientPipe.maxTimeStep`.
MAX_TIME_STEP = 10.0
#: `TransientPipe.thermodynamicUpdateInterval`.
DEFAULT_THERMO_INTERVAL = 10
#: `TransientPipe.historyInterval`; the class has no setter, so it is fixed.
HISTORY_INTERVAL = 10

MOLAR_GAS_CONSTANT = 8.314_462_618_153_24

GAS_FALLBACK = (1.0, 1.0e-5, 0.0, 340.0)
LIQUID_FALLBACK = (800.0, 1.0e-3, 0.0, 1500.0)
DEFAULT_SIGMA = 0.020
DEFAULT_WATER_SIGMA = 0.072

#: `FlowRegimeDetector.Degenerate gas viscosity fraction`.
DEGENERATE_GAS_VISCOSITY_FRACTION = 0.01
#: `FlowRegimeDetector.SHELTERING_COEFFICIENT`.
SHELTERING_COEFFICIENT = 0.01
#: `FlowRegimeDetector.ANNULAR_BRIDGING_HOLDUP`.
ANNULAR_BRIDGING_HOLDUP = 0.24
#: `FlowRegimeDetector.BUBBLE_PACKING_VOID_FRACTION`.
BUBBLE_PACKING_VOID_FRACTION = 0.52
#: `FlowRegimeDetector.DEFAULT_GAS_OIL_IFT`.
DEFAULT_GAS_OIL_IFT = 0.020

#: The three-phase refusal's reason, matched by `_Pipe._update_thermodynamics` so a
#: per-cell flash can tell it from a failed flash.
_THREE_PHASE_REASON = (
    "the flash leaves both an oil and an aqueous phase, and the class's three-phase "
    "liquid averaging has no counterpart in this model"
)

FLOW_REGIMES = (
    "STRATIFIED_SMOOTH",
    "STRATIFIED_WAVY",
    "SLUG",
    "ANNULAR",
    "DISPERSED_BUBBLE",
    "BUBBLE",
    "CHURN",
    "MIST",
    "SINGLE_PHASE_GAS",
    "SINGLE_PHASE_LIQUID",
)


@dataclass
class _Drift:
    c0: float = 1.0
    drift_velocity: float = 0.0
    gas_velocity: float = 0.0
    liquid_velocity: float = 0.0
    void_fraction: float = 0.0
    liquid_holdup: float = 0.0


@dataclass
class _Phase:
    kind: str
    fraction: float
    composition: list[float]
    z_factor: float
    density: float
    molar_mass: float
    viscosity: float
    enthalpy: float
    sound_speed: float
    volume_fraction: float


@dataclass
class _Fluid:
    gas: _Phase | None
    liquid: _Phase | None
    parachors: list[float]
    total_density: float
    surface_tension: float
    gas_beta: float


@dataclass
class _Section:
    position: float
    length: float
    diameter: float
    area: float
    inclination: float = 0.0
    elevation: float = 0.0
    roughness: float = DEFAULT_ROUGHNESS
    pressure: float = 0.0
    temperature: float = 0.0
    gas_holdup: float = 0.0
    liquid_holdup: float = 0.0
    gas_velocity: float = 0.0
    liquid_velocity: float = 0.0
    gas_density: float = 0.0
    liquid_density: float = 0.0
    gas_viscosity: float = 0.0
    liquid_viscosity: float = 0.0
    gas_sound_speed: float = 0.0
    liquid_sound_speed: float = 0.0
    surface_tension: float = 0.0
    gas_enthalpy: float = 0.0
    liquid_enthalpy: float = 0.0
    flow_regime: str = "STRATIFIED_SMOOTH"
    mixture_velocity: float = 0.0
    mixture_density: float = 0.0
    superficial_gas_velocity: float = 0.0
    superficial_liquid_velocity: float = 0.0
    gravity_pressure_gradient: float = 0.0
    friction_pressure_gradient: float = 0.0

    def update_derived(self) -> None:
        if self.gas_holdup + self.liquid_holdup > 1.0:
            total = self.gas_holdup + self.liquid_holdup
            self.gas_holdup /= total
            self.liquid_holdup /= total
        self.update_derived_with(
            self.gas_holdup * self.gas_velocity,
            self.liquid_holdup * self.liquid_velocity,
        )

    def update_derived_with(self, gas_superficial: float, liquid_superficial: float) -> None:
        self.mixture_density = (
            self.gas_holdup * self.gas_density + self.liquid_holdup * self.liquid_density
        )
        self.mixture_velocity = gas_superficial + liquid_superficial
        self.superficial_gas_velocity = gas_superficial
        self.superficial_liquid_velocity = liquid_superficial

    def conservative(self) -> list[float]:
        return [
            self.gas_density * self.gas_holdup,
            self.liquid_density * self.liquid_holdup,
            self.mixture_density * self.mixture_velocity,
            self.mixture_density
            * (self.gas_holdup * self.gas_enthalpy + self.liquid_holdup * self.liquid_enthalpy),
        ]

    def wallis_sound_speed(self) -> float:
        rho_m = self.mixture_density
        term_g = (
            self.gas_holdup / (self.gas_density * self.gas_sound_speed**2)
            if self.gas_holdup > 1e-6
            else 0.0
        )
        term_l = (
            self.liquid_holdup / (self.liquid_density * self.liquid_sound_speed**2)
            if self.liquid_holdup > 1e-6
            else 0.0
        )
        compressibility = term_g + term_l
        if compressibility <= 0.0 or rho_m <= 0.0:
            return max(self.gas_sound_speed, self.liquid_sound_speed)
        return math.sqrt(1.0 / (rho_m * compressibility))

    def clone(self) -> _Section:
        return _Section(
            position=self.position,
            length=self.length,
            diameter=self.diameter,
            area=self.area,
            inclination=self.inclination,
            elevation=self.elevation,
            roughness=self.roughness,
            pressure=self.pressure,
            temperature=self.temperature,
            gas_holdup=self.gas_holdup,
            liquid_holdup=self.liquid_holdup,
            gas_velocity=self.gas_velocity,
            liquid_velocity=self.liquid_velocity,
            gas_density=self.gas_density,
            liquid_density=self.liquid_density,
            gas_viscosity=self.gas_viscosity,
            liquid_viscosity=self.liquid_viscosity,
            gas_sound_speed=self.gas_sound_speed,
            liquid_sound_speed=self.liquid_sound_speed,
            surface_tension=self.surface_tension,
            gas_enthalpy=self.gas_enthalpy,
            liquid_enthalpy=self.liquid_enthalpy,
            flow_regime=self.flow_regime,
            mixture_velocity=self.mixture_velocity,
            mixture_density=self.mixture_density,
            superficial_gas_velocity=self.superficial_gas_velocity,
            superficial_liquid_velocity=self.superficial_liquid_velocity,
            gravity_pressure_gradient=self.gravity_pressure_gradient,
            friction_pressure_gradient=self.friction_pressure_gradient,
        )


def _molar_mass(composition: list[float], components: Any) -> float:
    return float(
        sum(
            fraction * component.molar_mass.to("kg/mol").magnitude
            for fraction, component in zip(composition, components, strict=True)
        )
    )


def _viscosity(
    names: list[str], kind: str, t: Q, p: Q, composition: list[float], mixture: Any
) -> float:
    if len(names) > 1:
        return phase_transport(names, kind, t, p, composition).mu.to("Pa*s").magnitude
    if kind == AQUEOUS:
        return aqueous_viscosity(mixture, t, p, composition).viscosity.to("Pa*s").magnitude
    return viscosity(mixture, t, p, composition).mu.to("Pa*s").magnitude


def _phase_speed(
    mixture: Any,
    ideal_gas: Any,
    reduced: Any,
    kij: Any,
    t: float,
    p: float,
    composition: list[float],
    root: float,
) -> float:
    capacity = phase_capacity(mixture, ideal_gas, reduced, kij, t, p, composition, root)
    molar_mass = _molar_mass(composition, mixture.components)
    volume = root * MOLAR_GAS_CONSTANT * t / p
    gamma = capacity.cp / capacity.cv
    return volume * math.sqrt(-gamma * capacity.d_p_d_v / molar_mass)


def _flash_phases(
    mixture: Any,
    ideal_gas: Any,
    names: list[str],
    t: float,
    p: float,
    z: list[float],
    srk_shift: bool,
) -> list[_Phase]:
    flash = tp_multiflash(mixture, from_si(t, "K"), from_si(p, "Pa"), z)
    components = mixture.components
    reduced = reduced_parameters(mixture, t, p)
    kij = mixture.effective_kij(t)
    volumes: list[float] = []
    unshifted: list[float] = []
    for index, fraction in enumerate(flash.phase_fractions):
        if fraction <= 0.0:
            continue
        composition = list(flash.x[index])
        root = flash.z_factor[index]
        shift = sum(
            value * (c.volume_shift_srk if srk_shift else c.volume_shift)
            for value, c in zip(composition, components, strict=True)
        )
        ideal = root * MOLAR_GAS_CONSTANT * t / p
        volume = ideal - shift
        if not math.isfinite(volume) or volume <= 0.0:
            raise InvalidInputError("components", "a phase's molar volume is not positive")
        volumes.append(fraction * volume)
        unshifted.append(fraction * ideal)
    total_volume = sum(volumes)
    total_unshifted = sum(unshifted)
    if total_volume <= 0.0 or total_unshifted <= 0.0:
        raise InvalidInputError("components", "the flash left no phase volume")

    phases: list[_Phase] = []
    position = 0
    for index, fraction in enumerate(flash.phase_fractions):
        if fraction <= 0.0:
            continue
        composition = list(flash.x[index])
        root = flash.z_factor[index]
        shift = sum(
            value * (c.volume_shift_srk if srk_shift else c.volume_shift)
            for value, c in zip(composition, components, strict=True)
        )
        volume = root * MOLAR_GAS_CONSTANT * t / p - shift
        molar_mass = _molar_mass(composition, components)
        kind = phase_label(mixture, reduced, composition, root)
        mu = _viscosity(names, kind, from_si(t, "K"), from_si(p, "Pa"), composition, mixture)
        enthalpy = (
            molar_enthalpy_entropy(
                mixture, ideal_gas, from_si(t, "K"), from_si(p, "Pa"), composition, root
            )
            .h.to("J/mol")
            .magnitude
            / molar_mass
            * 1000.0
        )
        speed = _phase_speed(mixture, ideal_gas, reduced, kij, t, p, composition, root)
        phases.append(
            _Phase(
                kind=kind,
                fraction=fraction,
                composition=composition,
                z_factor=root,
                density=molar_mass / volume,
                molar_mass=molar_mass,
                viscosity=mu,
                enthalpy=enthalpy,
                sound_speed=speed,
                volume_fraction=unshifted[position] / total_unshifted,
            )
        )
        position += 1
    return phases


def _fluid_state(
    mixture: Any,
    ideal_gas: Any,
    names: list[str],
    t: float,
    p: float,
    z: list[float],
    srk_shift: bool,
) -> _Fluid:
    phases = _flash_phases(mixture, ideal_gas, names, t, p, z, srk_shift)
    gas = next((phase for phase in phases if phase.kind == GAS), None)
    liquids = [phase for phase in phases if phase.kind != GAS]
    if len(liquids) > 1:
        raise InvalidInputError("components", _THREE_PHASE_REASON)
    liquid = liquids[0] if liquids else None
    parachors = [component.parachor for component in mixture.components]
    sigma = DEFAULT_SIGMA
    if gas is not None and liquid is not None:
        try:
            sigma = (
                parachor_mixture_surface_tension(
                    parachors,
                    from_si(gas.density, "kg/m**3"),
                    from_si(gas.molar_mass, "kg/mol"),
                    gas.composition,
                    from_si(liquid.density, "kg/m**3"),
                    from_si(liquid.molar_mass, "kg/mol"),
                    liquid.composition,
                )
                .sigma.to("N/m")
                .magnitude
            )
        except Exception:
            sigma = 0.0
        if sigma <= 0.0:
            sigma = DEFAULT_WATER_SIGMA if liquid.kind == AQUEOUS else DEFAULT_GAS_OIL_IFT
    elif liquid is not None:
        sigma = DEFAULT_WATER_SIGMA if liquid.kind == AQUEOUS else DEFAULT_SIGMA
    total_mass = sum(phase.fraction * phase.molar_mass for phase in phases)
    total_volume = 0.0
    for phase in phases:
        shift = sum(
            value * (c.volume_shift_srk if srk_shift else c.volume_shift)
            for value, c in zip(phase.composition, mixture.components, strict=True)
        )
        total_volume += phase.fraction * (phase.z_factor * MOLAR_GAS_CONSTANT * t / p - shift)
    return _Fluid(
        gas=gas,
        liquid=liquid,
        parachors=parachors,
        total_density=total_mass / total_volume,
        surface_tension=sigma,
        gas_beta=gas.fraction if gas is not None else 0.0,
    )


def _gradient_update(section: _Section, params: _Drift) -> None:
    section.gravity_pressure_gradient = (
        -(
            params.liquid_holdup * section.liquid_density
            + params.void_fraction * section.gas_density
        )
        * GRAVITY
        * math.sin(section.inclination)
    )
    section.friction_pressure_gradient = _friction_gradient(section, params, section.roughness)


def _friction_factor(re: float, rough: float, diameter: float) -> float:
    if re < 10.0:
        return 6.4
    if re < 2300.0:
        return 64.0 / re
    rel = rough / diameter
    term = (rel / 3.7) ** 1.11 + 6.9 / re
    return max((-1.8 * math.log10(term)) ** -2, 0.001)


def _single_gas_friction(section: _Section, rough: float) -> float:
    d = section.diameter
    rho = section.gas_density
    mu = section.gas_viscosity
    u = section.superficial_gas_velocity
    if rho <= 0.0 or mu <= 0.0 or u <= 0.0 or d <= 0.0:
        return 0.0
    f = _friction_factor(rho * u * d / mu, rough, d)
    return -f * rho * u * u / (2.0 * d)


def _single_liquid_friction(section: _Section, rough: float) -> float:
    d = section.diameter
    rho = section.liquid_density
    mu = section.liquid_viscosity
    u = section.superficial_liquid_velocity
    if rho <= 0.0 or mu <= 0.0 or u <= 0.0 or d <= 0.0:
        return 0.0
    f = _friction_factor(rho * u * d / mu, rough, d)
    return -f * rho * u * u / (2.0 * d)


def _homogeneous_friction(section: _Section, params: _Drift, rough: float) -> float:
    d = section.diameter
    u = section.superficial_gas_velocity + section.superficial_liquid_velocity
    alpha_l = params.liquid_holdup
    alpha_g = params.void_fraction
    rho = alpha_l * section.liquid_density + alpha_g * section.gas_density
    mu = alpha_l * section.liquid_viscosity + alpha_g * section.gas_viscosity
    f = _friction_factor(rho * u * d / mu, rough, d)
    return -f * rho * u * abs(u) / (2.0 * d)


def _stratified_friction(section: _Section, params: _Drift, rough: float) -> float:
    d = section.diameter
    h = params.liquid_holdup * d
    beta = 2.0 * math.acos(max(-1.0, min(1.0, 1.0 - 2.0 * h / d)))
    a_l = d * d / 8.0 * (beta - math.sin(beta))
    a_g = math.pi * d * d / 4.0 - a_l
    s_l = d * beta / 2.0
    s_g = d * (math.pi - beta / 2.0)
    if a_l < 1e-10 or a_g < 1e-10:
        return _homogeneous_friction(section, params, rough)
    u_l = params.liquid_velocity
    u_g = params.gas_velocity
    d_hl = 4.0 * a_l / s_l
    d_hg = 4.0 * a_g / s_g
    f_l = _friction_factor(
        section.liquid_density * abs(u_l) * d_hl / section.liquid_viscosity, rough, d_hl
    )
    f_g = _friction_factor(
        section.gas_density * abs(u_g) * d_hg / section.gas_viscosity, rough, d_hg
    )
    dp_l = f_l * section.liquid_density * u_l * abs(u_l) / (2.0 * d_hl)
    dp_g = f_g * section.gas_density * u_g * abs(u_g) / (2.0 * d_hg)
    return -(a_l * dp_l + a_g * dp_g) / (math.pi * d * d / 4.0)


def _annular_friction(section: _Section, params: _Drift, rough: float) -> float:
    d = section.diameter
    alpha_l = params.liquid_holdup
    delta = d / 2.0 * (1.0 - math.sqrt(max(0.0, 1.0 - alpha_l)))
    d_core = d - 2.0 * delta
    if d_core < 0.1 * d:
        return _homogeneous_friction(section, params, rough)
    u_g = params.gas_velocity
    f_g = _friction_factor(
        section.gas_density * abs(u_g) * d_core / section.gas_viscosity,
        rough + 0.5 * delta,
        d_core,
    )
    return -f_g * section.gas_density * u_g * abs(u_g) / (2.0 * d_core)


def _friction_gradient(section: _Section, params: _Drift, rough: float) -> float:
    regime = section.flow_regime
    if regime == "SINGLE_PHASE_GAS":
        return _single_gas_friction(section, rough)
    if regime == "SINGLE_PHASE_LIQUID":
        return _single_liquid_friction(section, rough)
    if regime in ("STRATIFIED_SMOOTH", "STRATIFIED_WAVY"):
        return _stratified_friction(section, params, rough)
    if regime == "ANNULAR":
        return _annular_friction(section, params, rough)
    return _homogeneous_friction(section, params, rough)


def _pressure_gradient(section: _Section, params: _Drift) -> float:
    _gradient_update(section, params)
    return section.gravity_pressure_gradient + section.friction_pressure_gradient


def _holdup_from_level(h: float, d: float) -> float:
    if h <= 0.0:
        return 0.0
    if h >= d:
        return 1.0
    beta = 2.0 * math.acos(max(-1.0, min(1.0, 1.0 - 2.0 * h / d)))
    return (beta - math.sin(beta)) / (2.0 * math.pi)


def _stratified_residual(
    h: float,
    u_sl: float,
    u_sg: float,
    d: float,
    rho_l: float,
    rho_g: float,
    mu_l: float,
    mu_g: float,
    theta: float,
) -> float:
    beta = 2.0 * math.acos(max(-1.0, min(1.0, 1.0 - 2.0 * h / d)))
    a_l = d * d / 8.0 * (beta - math.sin(beta))
    a_g = math.pi * d * d / 4.0 - a_l
    s_l = d * beta / 2.0
    s_g = d * (math.pi - beta / 2.0)
    s_i = d * math.sin(beta / 2.0)
    if a_l < 1e-10 or a_g < 1e-10:
        return float("nan")
    u_l = u_sl * math.pi * d * d / 4.0 / a_l
    u_g = u_sg * math.pi * d * d / 4.0 / a_g
    d_hl = 4.0 * a_l / s_l
    d_hg = 4.0 * a_g / (s_g + s_i)
    re_l = rho_l * abs(u_l) * d_hl / mu_l
    gas_viscosity = mu_g if mu_g > 0.0 else mu_l * DEGENERATE_GAS_VISCOSITY_FRACTION
    re_g = rho_g * abs(u_g) * d_hg / gas_viscosity
    f_l = 0.046 * re_l**-0.2 if re_l > 2000.0 else 16.0 / max(re_l, 1.0)
    f_g = 0.046 * re_g**-0.2 if re_g > 2000.0 else 16.0 / max(re_g, 1.0)
    tau_wl = f_l * rho_l * u_l * abs(u_l) / 2.0
    tau_wg = f_g * rho_g * u_g * abs(u_g) / 2.0
    tau_i = f_g * rho_g * (u_g - u_l) * abs(u_g - u_l) / 2.0
    gravity = (rho_l - rho_g) * GRAVITY * math.sin(theta)
    return (-tau_wl * s_l + tau_i * s_i) / a_l - (-tau_wg * s_g - tau_i * s_i) / a_g - gravity


def _estimate_stratified_level(
    u_sl: float,
    u_sg: float,
    d: float,
    rho_l: float,
    rho_g: float,
    mu_l: float,
    mu_g: float,
    theta: float,
) -> float:
    low = 0.01 * d
    high = 0.99 * d
    residual_low = _stratified_residual(low, u_sl, u_sg, d, rho_l, rho_g, mu_l, mu_g, theta)
    residual_high = _stratified_residual(high, u_sl, u_sg, d, rho_l, rho_g, mu_l, mu_g, theta)
    if not math.isfinite(residual_low) or not math.isfinite(residual_high):
        return 0.5 * d
    if residual_low * residual_high > 0.0:
        return low if abs(residual_low) <= abs(residual_high) else high
    for _ in range(60):
        mid = 0.5 * (low + high)
        residual_mid = _stratified_residual(mid, u_sl, u_sg, d, rho_l, rho_g, mu_l, mu_g, theta)
        if not math.isfinite(residual_mid):
            return mid
        if residual_low * residual_mid <= 0.0:
            high = mid
            residual_high = residual_mid
        else:
            low = mid
            residual_low = residual_mid
        if high - low < 1e-9 * d:
            break
    return 0.5 * (low + high)


def _drift_stratified_level(
    u_sl: float,
    u_sg: float,
    d: float,
    theta: float,
    rho_l: float,
    rho_g: float,
) -> float:
    """`DriftFluxModel.estimateStratifiedLevel`: its own level rule, not the detector's.

    The class carries two stratified-level solvers. `FlowRegimeDetector` bisects a momentum
    residual; `DriftFluxModel` adjusts a level over twenty steps. The drift-flux closure
    reads the second, so the two are kept apart here.
    """
    h = 0.5 * d
    for _ in range(20):
        alpha_l = _holdup_from_level(h, d)
        alpha_g = 1.0 - alpha_l
        if alpha_l < 0.01 or alpha_g < 0.01:
            break
        area = math.pi * d * d / 4.0
        u_l = u_sl * area / (alpha_l * area)
        u_g = u_sg * area / (alpha_g * area)
        tau_ratio = rho_g * u_g * u_g / (rho_l * u_l * u_l + 1e-10)
        gravity = (rho_l - rho_g) * GRAVITY * math.sin(theta) * d / (rho_l * u_l * u_l + 1e-10)
        adjustment = 0.05 * d * (tau_ratio - 1.0 - gravity)
        h = max(0.01 * d, min(0.99 * d, h - adjustment))
    return h


def _bubble_parameters(
    _u_m: float, _d: float, theta: float, rho_l: float, rho_g: float, sigma: float
) -> tuple[float, float]:
    c0 = 1.2
    delta_rho = max(rho_l - rho_g, 0.01)
    bubble = 1.53 * (GRAVITY * sigma * delta_rho / (rho_l * rho_l)) ** 0.25
    if abs(theta) > 0.01:
        return c0, bubble * abs(math.sin(theta))
    return c0, 0.1 * bubble


def _slug_parameters(
    u_m: float, d: float, theta: float, rho_l: float, rho_g: float
) -> tuple[float, float]:
    fr = u_m / math.sqrt(GRAVITY * d)
    c0 = 1.2 if fr > 3.5 else 1.05 + 0.15 * math.sin(theta)
    delta_rho = rho_l - rho_g
    u_horiz = 0.54 * math.sqrt(GRAVITY * d * delta_rho / rho_l)
    u_vert = 0.35 * math.sqrt(GRAVITY * d * delta_rho / rho_l)
    abs_theta = abs(theta)
    if abs_theta < math.pi / 6.0:
        return c0, u_horiz * math.cos(theta) + u_vert * math.sin(theta)
    if abs_theta > math.pi / 3.0:
        return c0, u_vert * math.sin(theta)
    w = (abs_theta - math.pi / 6.0) / (math.pi / 6.0)
    return c0, (1.0 - w) * u_horiz * math.cos(theta) + w * u_vert * math.sin(theta)


def _annular_parameters(d: float, theta: float, rho_l: float, rho_g: float) -> tuple[float, float]:
    delta_rho = rho_l - rho_g
    return 1.0, 0.2 * math.sqrt(GRAVITY * d * abs(delta_rho) / rho_l) * math.sin(theta)


def _drift_flux(section: _Section) -> _Drift:
    u_sl = section.superficial_liquid_velocity
    u_sg = section.superficial_gas_velocity
    u_m = u_sl + u_sg
    d = section.diameter
    theta = section.inclination
    rho_l = section.liquid_density
    rho_g = section.gas_density
    sigma = section.surface_tension
    params = _Drift()
    regime = section.flow_regime
    if regime in ("BUBBLE", "DISPERSED_BUBBLE"):
        params.c0, params.drift_velocity = _bubble_parameters(u_m, d, theta, rho_l, rho_g, sigma)
    elif regime == "SLUG":
        params.c0, params.drift_velocity = _slug_parameters(u_m, d, theta, rho_l, rho_g)
    elif regime in ("ANNULAR", "CHURN"):
        params.c0, params.drift_velocity = _annular_parameters(d, theta, rho_l, rho_g)
    elif regime in ("STRATIFIED_SMOOTH", "STRATIFIED_WAVY"):
        h_l = _drift_stratified_level(u_sl, u_sg, d, theta, rho_l, rho_g)
        alpha_l = _holdup_from_level(h_l, d)
        params.liquid_holdup = alpha_l
        params.void_fraction = 1.0 - alpha_l
        if u_sl + u_sg > 1e-10 and params.void_fraction > 1e-10:
            params.gas_velocity = u_sg / params.void_fraction
            params.liquid_velocity = u_sl / max(params.liquid_holdup, 1e-10)
            params.c0 = params.gas_velocity / (u_sl + u_sg)
            params.drift_velocity = 0.0
        else:
            params.c0 = 1.0
            params.drift_velocity = 0.0
    if u_m > 1e-10:
        denominator = params.c0 * u_m + params.drift_velocity
        if denominator > 1e-10:
            params.void_fraction = u_sg / denominator
        else:
            params.void_fraction = 1.0 if u_sg > 0.0 else 0.0
        params.void_fraction = max(0.0, min(1.0, params.void_fraction))
        params.liquid_holdup = 1.0 - params.void_fraction
        if params.void_fraction > 1e-10:
            params.gas_velocity = u_sg / params.void_fraction
        if params.liquid_holdup > 1e-10:
            params.liquid_velocity = u_sl / params.liquid_holdup
    return params


def _flow_oriented(section: _Section) -> _Section:
    gas_flow = section.superficial_gas_velocity
    liquid_flow = section.superficial_liquid_velocity
    if not (gas_flow <= 0.0 and liquid_flow <= 0.0 and (gas_flow < 0.0 or liquid_flow < 0.0)):
        return section
    oriented = section.clone()
    oriented.inclination = -section.inclination
    oriented.gas_velocity = -section.gas_velocity
    oriented.liquid_velocity = -section.liquid_velocity
    oriented.update_derived_with(-gas_flow, -liquid_flow)
    return oriented


def _liquid_area_fraction(h: float, d: float) -> float:
    if h <= 0.0 or d <= 0.0:
        return 0.0
    if h >= d:
        return 1.0
    beta = 2.0 * math.acos(max(-1.0, min(1.0, 1.0 - 2.0 * h / d)))
    return (beta - math.sin(beta)) / (2.0 * math.pi)


def _dispersed_bubble(
    u_sl: float, u_sg: float, d: float, rho_l: float, _rho_g: float, sigma: float
) -> bool:
    u_m = u_sl + u_sg
    sigma_eff = DEFAULT_GAS_OIL_IFT if sigma < 1e-6 else sigma
    we = rho_l * u_m * u_m * d / sigma_eff
    # `Math.sqrt(U_SG)` on a negative gas superficial velocity is NaN, and the whole
    # criterion is then false; the class does not clamp it to zero.
    if u_sg < 0.0:
        return False
    u_crit = 0.725 + 4.15 * math.sqrt(u_sg)
    ratio = u_sg / u_m if u_m != 0.0 else float("nan")
    return u_m > u_crit and we > 20.0 and ratio < 0.52


def _annular_ratio(u_sg: float, rho_l: float, rho_g: float, sigma: float) -> float:
    delta_rho = rho_l - rho_g
    if delta_rho < 1e-6:
        return 0.0
    sigma_eff = DEFAULT_GAS_OIL_IFT if sigma < 1e-6 else sigma
    u_crit = 3.1 * (sigma_eff * GRAVITY * delta_rho / (rho_g * rho_g)) ** 0.25
    return float(u_sg / u_crit)


def _kh_margin(u_sg: float, h: float, d: float, rho_l: float, rho_g: float) -> float:
    if h < 0.01 * d or h > 0.99 * d:
        return 0.0
    beta = 2.0 * math.acos(max(-1.0, min(1.0, 1.0 - 2.0 * h / d)))
    a_g = math.pi * d * d / 4.0 - d * d / 8.0 * (beta - math.sin(beta))
    s_i = d * math.sin(beta / 2.0)
    if a_g < 1e-10:
        return 0.0
    u_g = u_sg * math.pi * d * d / 4.0 / a_g
    delta_rho = rho_l - rho_g
    c2 = 1.0 - h / d
    u_crit = c2 * math.sqrt(delta_rho * GRAVITY * a_g / (rho_g * s_i))
    if u_crit <= 0.0:
        return 0.0
    return u_g / u_crit


def _wavy(
    u_sl: float,
    u_sg: float,
    h: float,
    d: float,
    theta: float,
    rho_l: float,
    rho_g: float,
    mu_l: float,
) -> bool:
    if h < 0.01 * d or h > 0.99 * d:
        return False
    beta = 2.0 * math.acos(max(-1.0, min(1.0, 1.0 - 2.0 * h / d)))
    a = math.pi * d * d / 4.0
    a_l = d * d / 8.0 * (beta - math.sin(beta))
    a_g = a - a_l
    if a_l < 1e-12 or a_g < 1e-12:
        return False
    u_g = u_sg * a / a_g
    u_l = u_sl * a / a_l
    if u_l <= 0.0:
        return False
    delta_rho = rho_l - rho_g
    cos_theta = math.cos(theta)
    if delta_rho <= 0.0 or cos_theta <= 0.0:
        return False
    nu_l = mu_l / rho_l
    u_crit = math.sqrt(
        4.0 * nu_l * delta_rho * GRAVITY * cos_theta / (SHELTERING_COEFFICIENT * rho_g * u_l)
    )
    return u_g > u_crit


def _flow_regime(section: _Section) -> str:
    section = _flow_oriented(section)
    u_sl = section.superficial_liquid_velocity
    u_sg = section.superficial_gas_velocity
    alpha_l = section.liquid_holdup
    alpha_g = section.gas_holdup
    if alpha_l <= 0.0 and alpha_g > 0.0:
        return "SINGLE_PHASE_GAS"
    if alpha_g <= 0.0 and alpha_l > 0.0:
        return "SINGLE_PHASE_LIQUID"
    if alpha_l <= 0.0 and alpha_g <= 0.0:
        return "SINGLE_PHASE_GAS" if u_sg != 0.0 else "SINGLE_PHASE_LIQUID"
    d = section.diameter
    theta = section.inclination
    rho_l = section.liquid_density
    rho_g = section.gas_density
    mu_l = section.liquid_viscosity
    mu_g = section.gas_viscosity
    sigma = section.surface_tension

    if abs(theta) > math.radians(10.0):
        if _dispersed_bubble(u_sl, u_sg, d, rho_l, rho_g, sigma):
            return "DISPERSED_BUBBLE"
        if _annular_ratio(u_sg, rho_l, rho_g, sigma) > 1.0:
            return "ANNULAR"
        if theta > 0.0:
            # `calcBubbleRiseVelocity` answers zero when the phases have no density contrast.
            u_bubble = (
                0.0
                if rho_l - rho_g < 1e-6
                else 1.53 * (GRAVITY * sigma * (rho_l - rho_g) / (rho_l * rho_l)) ** 0.25
            )
            transport = u_sg + u_sl + u_bubble
            if u_sg >= 0.0 and transport > 0.0 and 1.0 - alpha_l < BUBBLE_PACKING_VOID_FRACTION:
                void = u_sg / transport
                if void < 0.25:
                    return "BUBBLE"
            return "SLUG"
        h_l = _estimate_stratified_level(u_sl, u_sg, d, rho_l, rho_g, mu_l, mu_g, theta)
        if _kh_margin(u_sg, h_l, d, rho_l, rho_g) > 1.0:
            return "SLUG"
        if _wavy(u_sl, u_sg, h_l, d, theta, rho_l, rho_g, mu_l):
            return "STRATIFIED_WAVY"
        return "STRATIFIED_SMOOTH"

    if _dispersed_bubble(u_sl, u_sg, d, rho_l, rho_g, sigma):
        return "DISPERSED_BUBBLE"
    h_l = _estimate_stratified_level(u_sl, u_sg, d, rho_l, rho_g, mu_l, mu_g, theta)
    if _kh_margin(u_sg, h_l, d, rho_l, rho_g) > 1.0:
        return "SLUG" if _liquid_area_fraction(h_l, d) >= ANNULAR_BRIDGING_HOLDUP else "ANNULAR"
    if _wavy(u_sl, u_sg, h_l, d, theta, rho_l, rho_g, mu_l):
        return "STRATIFIED_WAVY"
    return "STRATIFIED_SMOOTH"


def _ausm_flux(left: _Section, right: _Section) -> list[float]:
    rho_l_l = left.liquid_density * left.liquid_holdup
    rho_g_l = left.gas_density * left.gas_holdup
    rho_l_r = right.liquid_density * right.liquid_holdup
    rho_g_r = right.gas_density * right.gas_holdup
    c_face = 0.5 * (left.wallis_sound_speed() + right.wallis_sound_speed())
    p_plus = _split_pressure_plus(left.mixture_velocity / c_face, left.pressure)
    p_minus = _split_pressure_minus(right.mixture_velocity / c_face, right.pressure)
    p_face = p_plus + p_minus

    m_g = _split_mach_plus(left.gas_velocity / c_face) + _split_mach_minus(
        right.gas_velocity / c_face
    )
    m_l = _split_mach_plus(left.liquid_velocity / c_face) + _split_mach_minus(
        right.liquid_velocity / c_face
    )
    mdot_g = m_g * c_face * (rho_g_l if m_g > 0.0 else rho_g_r)
    mdot_l = m_l * c_face * (rho_l_l if m_l > 0.0 else rho_l_r)
    mom_g = mdot_g * (left.gas_velocity if m_g > 0.0 else right.gas_velocity)
    mom_l = mdot_l * (left.liquid_velocity if m_l > 0.0 else right.liquid_velocity)
    h_g = left.gas_enthalpy if m_g > 0.0 else right.gas_enthalpy
    h_l = left.liquid_enthalpy if m_l > 0.0 else right.liquid_enthalpy
    u_g = left.gas_velocity if m_g > 0.0 else right.gas_velocity
    u_l = left.liquid_velocity if m_l > 0.0 else right.liquid_velocity
    energy_g = mdot_g * (h_g + 0.5 * u_g * u_g)
    energy_l = mdot_l * (h_l + 0.5 * u_l * u_l)
    u_mix = left.mixture_velocity if (m_g + m_l) > 0.0 else right.mixture_velocity
    return [mdot_g, mdot_l, mom_g + mom_l + p_face, energy_g + energy_l + p_face * u_mix]


def _split_mach_plus(mach: float) -> float:
    if abs(mach) <= 1.0:
        return 0.25 * (mach + 1.0) * (mach + 1.0)
    return 0.5 * (mach + abs(mach))


def _split_mach_minus(mach: float) -> float:
    if abs(mach) <= 1.0:
        return -0.25 * (mach - 1.0) * (mach - 1.0)
    return 0.5 * (mach - abs(mach))


def _split_pressure_plus(mach: float, pressure: float) -> float:
    if abs(mach) <= 1.0:
        return pressure * 0.25 * (mach + 1.0) * (mach + 1.0) * (2.0 - mach)
    return pressure * 0.5 * (1.0 + math.copysign(1.0, mach))


def _split_pressure_minus(mach: float, pressure: float) -> float:
    if abs(mach) <= 1.0:
        return pressure * 0.25 * (mach - 1.0) * (mach - 1.0) * (2.0 + mach)
    return pressure * 0.5 * (1.0 - math.copysign(1.0, mach))


class _Pipe:
    def __init__(
        self,
        mixture: Any,
        ideal_gas: Any,
        names: list[str],
        fluid: _Fluid,
        sections: list[_Section],
        inlet_mass_flow: float,
        outlet_mass_flow: float,
        inlet_pressure: float,
        outlet_pressure: float,
        inlet_temperature: float,
        cfl: float,
        max_time: float,
        thermo_interval: int,
        srk_shift: bool,
        inlet_boundary: str,
        outlet_boundary: str,
    ) -> None:
        self.mixture = mixture
        self.ideal_gas = ideal_gas
        self.names = names
        self.fluid = fluid
        self.sections = sections
        self.inlet_mass_flow = inlet_mass_flow
        self.outlet_mass_flow = outlet_mass_flow
        self.inlet_pressure = inlet_pressure
        self.outlet_pressure = outlet_pressure
        self.inlet_temperature = inlet_temperature
        self.cfl = cfl
        self.max_time = max_time
        self.thermo_interval = thermo_interval
        self.srk_shift = srk_shift
        self.inlet_boundary = inlet_boundary
        self.outlet_boundary = outlet_boundary
        self.dx = sections[0].length
        self.simulation_time = 0.0
        self.total_steps = 0
        self.current_step = 0
        self.mass_inventory = 0.0
        history_rows = int(max_time / (DEFAULT_DT * HISTORY_INTERVAL)) + 1
        self.history: list[list[float]] = [[0.0] * len(sections) for _ in range(history_rows)]
        self.history_index = 0

    def _fluxes(self, old: list[_Section]) -> list[list[float]]:
        n = len(old)
        fluxes = [[0.0] * 4 for _ in range(n + 1)]
        if self.inlet_boundary == "constant_flow":
            fluid = self.fluid
            gas = fluid.gas
            liquid = fluid.liquid
            rho_mix = fluid.total_density if fluid.total_density > 1e-5 else old[0].mixture_density
            area = old[0].area
            u_inlet = self.inlet_mass_flow / (rho_mix * area)
            p_inlet = old[0].pressure
            rho_g = gas.density if gas is not None else 0.0
            rho_l = liquid.density if liquid is not None else 0.0
            h_g = gas.enthalpy if gas is not None else 0.0
            h_l = liquid.enthalpy if liquid is not None else 0.0
            alpha_g = gas.volume_fraction if gas is not None else 0.0
            alpha_l = liquid.volume_fraction if liquid is not None else 0.0
            fluxes[0][0] = rho_g * alpha_g * u_inlet
            fluxes[0][1] = rho_l * alpha_l * u_inlet
            fluxes[0][2] = (rho_g * alpha_g + rho_l * alpha_l) * u_inlet * u_inlet + p_inlet
            h_total = rho_g * alpha_g * (h_g + 0.5 * u_inlet * u_inlet) + rho_l * alpha_l * (
                h_l + 0.5 * u_inlet * u_inlet
            )
            fluxes[0][3] = h_total * u_inlet + p_inlet * u_inlet
        elif self.inlet_boundary == "closed":
            fluxes[0][2] = old[0].pressure
        else:
            fluxes[0] = _ausm_flux(old[0], old[0])
        for index in range(1, n):
            fluxes[index] = _ausm_flux(old[index - 1], old[index])
        if self.outlet_boundary == "constant_pressure":
            fluxes[n] = _ausm_flux(old[n - 1], old[n - 1])
        elif self.outlet_boundary == "closed":
            fluxes[n][2] = old[n - 1].pressure
        else:
            section = old[n - 1]
            u_outlet = self.outlet_mass_flow / (section.mixture_density * section.area)
            p_outlet = section.pressure
            rho_g = section.gas_density
            rho_l = section.liquid_density
            alpha_g = section.gas_holdup
            alpha_l = section.liquid_holdup
            fluxes[n][0] = rho_g * alpha_g * u_outlet
            fluxes[n][1] = rho_l * alpha_l * u_outlet
            fluxes[n][2] = (rho_g * alpha_g + rho_l * alpha_l) * u_outlet * u_outlet + p_outlet
            h_total = rho_g * alpha_g * (
                section.gas_enthalpy + 0.5 * u_outlet * u_outlet
            ) + rho_l * alpha_l * (section.liquid_enthalpy + 0.5 * u_outlet * u_outlet)
            fluxes[n][3] = h_total * u_outlet + p_outlet * u_outlet
        return fluxes

    def _apply_boundary(self) -> None:
        sections = self.sections
        sections[0].temperature = self.inlet_temperature
        if self.inlet_boundary == "constant_pressure":
            sections[0].pressure = self.inlet_pressure
        elif self.inlet_boundary == "constant_flow":
            inlet = sections[0]
            u_m = self.inlet_mass_flow / (inlet.mixture_density * inlet.area)
            alpha_g = inlet.gas_holdup
            alpha_l = inlet.liquid_holdup
            if alpha_l < 0.001 or alpha_g < 0.001:
                inlet.gas_velocity = u_m
                inlet.liquid_velocity = u_m
            else:
                inlet.gas_velocity = u_m
                inlet.liquid_velocity = u_m
                inlet.update_derived()
                params = _drift_flux(inlet)
                v_g = params.c0 * u_m + params.drift_velocity
                v_l = (inlet.mixture_density * u_m - inlet.gas_density * alpha_g * v_g) / (
                    inlet.liquid_density * alpha_l
                )
                inlet.gas_velocity = v_g
                inlet.liquid_velocity = v_l
        else:
            sections[0].gas_velocity = 0.0
            sections[0].liquid_velocity = 0.0
        last = sections[-1]
        if self.outlet_boundary == "constant_pressure":
            last.pressure = self.outlet_pressure
        elif self.outlet_boundary == "constant_flow":
            u_outlet = self.outlet_mass_flow / (last.mixture_density * last.area)
            last.gas_velocity = u_outlet
            last.liquid_velocity = u_outlet
        else:
            last.gas_velocity = 0.0
            last.liquid_velocity = 0.0

    def _time_step(self) -> float:
        max_wave = 0.0
        nan_count = 0
        for section in self.sections:
            c = section.wallis_sound_speed()
            v_g = section.gas_velocity
            v_l = section.liquid_velocity
            p = section.pressure
            if math.isnan(v_g) or math.isnan(v_l) or math.isnan(p) or math.isnan(c):
                nan_count += 1
                continue
            v_g = max(-500.0, min(500.0, v_g))
            v_l = max(-100.0, min(100.0, v_l))
            c = max(10.0, min(1000.0, c))
            local = max(abs(v_g + c), abs(v_g - c), abs(v_l + c), abs(v_l - c))
            if math.isfinite(local):
                max_wave = max(max_wave, local)
        if nan_count > len(self.sections) // 4:
            return MIN_TIME_STEP
        if max_wave <= 0.0:
            max_wave = 100.0
        return max(MIN_TIME_STEP, min(MAX_TIME_STEP, self.cfl * self.dx / max_wave))

    def _add_sources(self, conserved: list[float], section: _Section, dt: float) -> None:
        rho_m = section.mixture_density
        if math.isnan(rho_m) or rho_m <= 0.0:
            rho_m = 100.0
        gravity = -rho_m * GRAVITY * math.sin(section.inclination)
        if not math.isnan(gravity):
            conserved[2] += gravity * dt
        params = _drift_flux(section)
        total = _pressure_gradient(section, params)
        friction = total - section.gravity_pressure_gradient
        friction = 0.0 if math.isnan(friction) else max(-1000.0, min(1000.0, friction))
        conserved[2] += friction * dt

    def _update_primitive(
        self, section: _Section, old: _Section, conserved: list[float], index: int, dt: float
    ) -> None:
        del dt
        mass_g = max(conserved[0], 1e-10)
        mass_l = max(conserved[1], 1e-10)
        momentum = conserved[2]
        rho_g = section.gas_density
        rho_l = section.liquid_density
        if math.isnan(rho_g) or rho_g <= 0.0:
            rho_g = old.gas_density
        if math.isnan(rho_l) or rho_l <= 0.0:
            rho_l = old.liquid_density
        alpha_g = mass_g / rho_g
        alpha_l = mass_l / rho_l
        total = alpha_g + alpha_l
        if total > 0.0 and not math.isnan(total):
            alpha_g /= total
            alpha_l /= total
        else:
            alpha_g = old.gas_holdup
            alpha_l = old.liquid_holdup
        alpha_g = max(0.0001, min(0.9999, alpha_g))
        alpha_l = max(0.0001, min(0.9999, alpha_l))
        total = alpha_g + alpha_l
        alpha_g /= total
        alpha_l /= total
        section.gas_holdup = alpha_g
        section.liquid_holdup = alpha_l
        rho_m = mass_g + mass_l
        u_m = momentum / max(rho_m, 1e-10)
        if math.isnan(u_m) or abs(u_m) > 200.0:
            u_m = old.mixture_velocity
        u_m = max(-200.0, min(200.0, u_m))
        if alpha_l < 0.001:
            section.gas_velocity = u_m
            section.liquid_velocity = 0.0
        elif alpha_g < 0.001:
            section.gas_velocity = 0.0
            section.liquid_velocity = u_m
        else:
            section.gas_velocity = u_m
            section.liquid_velocity = u_m
            section.update_derived()
            params = _drift_flux(section)
            c0 = params.c0
            drift = params.drift_velocity
            if math.isnan(c0) or c0 < 0.9 or c0 > 1.5:
                c0 = 1.0
            if math.isnan(drift) or abs(drift) > 10.0:
                drift = 0.0
            numerator = drift + c0 * rho_m * u_m / rho_l
            denominator = 1.0 - c0 * alpha_g + c0 * rho_g * alpha_g / rho_l
            v_g = numerator / denominator if abs(denominator) > 1e-10 else u_m
            v_l = (
                (rho_m * u_m - rho_g * alpha_g * v_g) / (rho_l * alpha_l)
                if alpha_l > 1e-10
                else u_m
            )
            if math.isnan(v_g) or abs(v_g) > 300.0:
                v_g = u_m
            if math.isnan(v_l) or abs(v_l) > 300.0:
                v_l = u_m
            section.gas_velocity = v_g
            section.liquid_velocity = v_l
        section.temperature = old.temperature
        if index != 0 and index != len(self.sections) - 1:
            rho_old = old.mixture_density
            rho_new = mass_g + mass_l
            c_mix = section.wallis_sound_speed()
            if math.isnan(c_mix) or c_mix < 10.0 or c_mix > 1000.0:
                c_mix = 200.0
            d_rho = rho_new - rho_old
            cap = 0.05 * rho_old
            d_rho = max(-cap, min(cap, d_rho))
            d_p = c_mix * c_mix * d_rho
            old_p = old.pressure
            if math.isnan(old_p) or old_p <= 0.0:
                old_p = 1e6
            cap = 0.05 * old_p
            d_p = max(-cap, min(cap, d_p))
            section.pressure = max(1e5, min(5e7, old_p + d_p))
        else:
            section.pressure = old.pressure
        section.update_derived()

    def _advance(self, dt: float) -> None:
        self._apply_boundary()
        old = [section.clone() for section in self.sections]
        fluxes = self._fluxes(old)
        for index, section in enumerate(self.sections):
            conserved_old = old[index].conservative()
            conserved = [
                conserved_old[k] - dt / self.dx * (fluxes[index + 1][k] - fluxes[index][k])
                for k in range(4)
            ]
            for k in range(4):
                if math.isnan(conserved[k]) or math.isinf(conserved[k]):
                    conserved[k] = conserved_old[k]
            self._add_sources(conserved, section, dt)
            self._update_primitive(section, old[index], conserved, index, dt)
        self._apply_boundary()
        self._update_boundary_pressures()

    def _update_boundary_pressures(self) -> None:
        sections = self.sections
        if self.inlet_boundary == "constant_flow" and len(sections) > 1:
            inlet = sections[0]
            gravity = inlet.mixture_density * GRAVITY * math.sin(inlet.inclination) * self.dx
            inlet.pressure = sections[1].pressure + gravity
        if self.outlet_boundary == "constant_flow" and len(sections) > 1:
            outlet = sections[-1]
            gravity = outlet.mixture_density * GRAVITY * math.sin(outlet.inclination) * self.dx
            outlet.pressure = sections[-2].pressure - gravity

    def _update_thermodynamics(self) -> None:
        for section in self.sections:
            try:
                fluid = _fluid_state(
                    self.mixture,
                    self.ideal_gas,
                    self.names,
                    section.temperature,
                    section.pressure,
                    self.overall_z,
                    self.srk_shift,
                )
            except InvalidInputError as error:
                # The class's per-cell try/catch keeps the previous properties when a flash
                # fails. A flash that *succeeds* into three phases is not a failure: the
                # class's three-phase averaging is the branch this model refuses, so it is
                # reported rather than swallowed as a stale cell.
                if error.reason == _THREE_PHASE_REASON:
                    raise
                continue
            except Exception:
                continue
            if fluid.gas is not None:
                section.gas_density = fluid.gas.density
                section.gas_viscosity = fluid.gas.viscosity
                section.gas_enthalpy = fluid.gas.enthalpy
                section.gas_sound_speed = fluid.gas.sound_speed
            if fluid.liquid is not None:
                section.liquid_density = fluid.liquid.density
                section.liquid_viscosity = fluid.liquid.viscosity
                section.liquid_enthalpy = fluid.liquid.enthalpy
                section.liquid_sound_speed = fluid.liquid.sound_speed
            if fluid.surface_tension > 1e-6:
                section.surface_tension = fluid.surface_tension
            else:
                section.surface_tension = (
                    DEFAULT_WATER_SIGMA
                    if fluid.liquid is not None and fluid.liquid.kind == AQUEOUS
                    else DEFAULT_SIGMA
                )

    def _store_results(self) -> None:
        self.pressure_profile = [section.pressure for section in self.sections]
        self.temperature_profile = [section.temperature for section in self.sections]
        self.holdup_profile = [section.liquid_holdup for section in self.sections]
        self.gas_velocity_profile = [section.gas_velocity for section in self.sections]
        self.liquid_velocity_profile = [section.liquid_velocity for section in self.sections]

    def _store_history(self) -> None:
        if self.history_index < len(self.history):
            for index, section in enumerate(self.sections):
                self.history[self.history_index][index] = section.pressure
            self.history_index += 1

    def _check_convergence(self) -> None:
        self.mass_inventory = sum(
            section.mixture_density * section.area * self.dx for section in self.sections
        )

    def run(self, overall_z: list[float]) -> None:
        self.overall_z = overall_z
        self._store_results()
        self._store_history()
        while self.simulation_time < self.max_time:
            dt = self._time_step()
            self._advance(dt)
            self.simulation_time += dt
            self.total_steps += 1
            self.current_step += 1
            for section in self.sections:
                section.flow_regime = _flow_regime(section)
            if self.current_step % self.thermo_interval == 0:
                self._update_thermodynamics()
            if self.total_steps % HISTORY_INTERVAL == 0:
                self._store_history()
            self._check_convergence()
        self._store_results()


def _refuse_low_points(elevation: list[float], inclinations: list[float]) -> None:
    for index in range(1, len(elevation) - 1):
        if elevation[index] < elevation[index - 1] and elevation[index] <= elevation[index + 1]:
            raise InvalidInputError(
                "elevation_profile",
                "a local minimum is an accumulation zone, and the class's liquid-accumulation "
                "tracker has no counterpart in this model",
            )
    for index in range(1, len(inclinations)):
        if inclinations[index - 1] < math.radians(5.0) and inclinations[index] > math.radians(30.0):
            raise InvalidInputError(
                "inclination_profile",
                "a riser base is an accumulation zone, and the class's liquid-accumulation "
                "tracker has no counterpart in this model",
            )


def transient_pipe(
    components: list[str],
    inlet_z: list[float],
    inlet_mass_flow: Q,
    inlet_p: Q,
    inlet_t: Q,
    length: Q,
    diameter: Q,
    number_of_sections: int,
    roughness: Q,
    inlet_boundary: str,
    outlet_boundary: str,
    elevation_profile: list[float] | None = None,
    inclination_profile: list[float] | None = None,
    outlet_pressure: Q | None = None,
    max_simulation_time: Q | None = None,
    cfl: float | None = None,
    thermodynamic_update_interval: int | None = None,
    eos: str | None = None,
) -> TransientPipeResult:
    """NeqSim ``TransientPipe.run``: the isothermal drift-flux march.

    Args:
        components: the fluid's substances.
        inlet_z: the feed's overall mole fractions.
        inlet_mass_flow: the inlet stream's mass flow, which initialises the state.
        inlet_p: the inlet stream's pressure.
        inlet_t: the inlet stream's temperature, held through the isothermal march.
        length: the line's total length.
        diameter: the internal diameter.
        number_of_sections: the number of cells.
        roughness: the wall's absolute roughness.
        inlet_boundary: ``constant_flow``, ``constant_pressure`` or ``closed``.
        outlet_boundary: ``constant_pressure``, ``constant_flow`` or ``closed``.
        elevation_profile: each cell's elevation; absent is flat.
        inclination_profile: each cell's inclination; used only when no elevation is given.
        outlet_pressure: the constant outlet pressure; absent is the class's estimate.
        max_simulation_time: the march's end; absent is the class's 3600 s.
        cfl: the CFL number; absent is 0.5.
        thermodynamic_update_interval: steps between cell flashes; absent is 10.
        eos: the cubic; absent is ``srk``.

    Returns:
        The final profiles, the pressure history, the simulation time, the step count and
        the class's mass inventory.

    Raises:
        OutOfRangeError: an input outside the spec's hard range.
        InvalidInputError: a three-phase flash, a low-point or riser-base elevation
            profile, a boundary outside the three, or a profile of the wrong length.
    """
    from azoth._models_gen import model as _model_for

    spec: dict[str, Any] = _model_for(CALC_ID)
    checks = checks_for(spec)
    warnings: list[Warning] = []
    sections_count = int(number_of_sections)
    if sections_count < 1:
        raise InvalidInputError("number_of_sections", "a line needs at least one cell")
    boundary_names = {"constant_flow", "constant_pressure", "closed"}
    if inlet_boundary not in boundary_names:
        raise InvalidInputError(
            "inlet_boundary", f"`{inlet_boundary}` is not one of {sorted(boundary_names)}"
        )
    if outlet_boundary not in boundary_names:
        raise InvalidInputError(
            "outlet_boundary", f"`{outlet_boundary}` is not one of {sorted(boundary_names)}"
        )
    values: dict[str, float] = {
        name: input_to_si(spec, name, value)
        for name, value in {
            "inlet_mass_flow": inlet_mass_flow,
            "inlet_p": inlet_p,
            "inlet_t": inlet_t,
            "length": length,
            "diameter": diameter,
            "roughness": roughness,
        }.items()
    }
    values["number_of_sections"] = sections_count
    if outlet_pressure is not None:
        values["outlet_pressure"] = input_to_si(spec, "outlet_pressure", outlet_pressure)
    if max_simulation_time is not None:
        values["max_simulation_time"] = input_to_si(
            spec, "max_simulation_time", max_simulation_time
        )
    if cfl is not None:
        values["cfl"] = float(cfl)
    if thermodynamic_update_interval is not None:
        values["thermodynamic_update_interval"] = int(thermodynamic_update_interval)
    apply_checks(checks.on_input, values.get, warnings)

    cubic = eos or "srk"
    if cubic not in ("pr", "srk"):
        raise InvalidInputError("eos", f"`{cubic}` is not one of `pr` or `srk`")
    mixture, _ideal = mixture_of(components, eos=cubic)
    srk_shift = cubic == "srk"

    if elevation_profile is not None:
        if len(elevation_profile) < sections_count:
            raise InvalidInputError(
                "elevation_profile", "the profile needs one elevation per section"
            )
    elif inclination_profile is not None and len(inclination_profile) < sections_count:
        raise InvalidInputError(
            "inclination_profile", "the profile needs one inclination per section"
        )

    dx = values["length"] / sections_count
    area = math.pi * values["diameter"] ** 2 / 4.0
    sections = [
        _Section(
            position=(index + 0.5) * dx,
            length=dx,
            diameter=values["diameter"],
            area=area,
            roughness=values["roughness"],
        )
        for index in range(sections_count)
    ]
    inclinations = [0.0] * sections_count
    elevations = [0.0] * sections_count
    if elevation_profile is not None:
        for index in range(sections_count):
            elevations[index] = elevation_profile[index]
            if index > 0:
                dz = elevation_profile[index] - elevation_profile[index - 1]
                inclinations[index] = math.atan2(dz, dx)
    elif inclination_profile is not None:
        elevation = 0.0
        for index in range(sections_count):
            inclinations[index] = inclination_profile[index]
            elevations[index] = elevation
            elevation += dx * math.sin(inclination_profile[index])
    _refuse_low_points(elevations, inclinations)

    fluid = _fluid_state(
        mixture, _ideal, components, values["inlet_t"], values["inlet_p"], list(inlet_z), srk_shift
    )
    gas = fluid.gas
    liquid = fluid.liquid
    rho_g = gas.density if gas is not None else GAS_FALLBACK[0]
    mu_g = gas.viscosity if gas is not None else GAS_FALLBACK[1]
    h_g = gas.enthalpy if gas is not None else GAS_FALLBACK[2]
    c_g = gas.sound_speed if gas is not None else GAS_FALLBACK[3]
    rho_l = liquid.density if liquid is not None else LIQUID_FALLBACK[0]
    mu_l = liquid.viscosity if liquid is not None else LIQUID_FALLBACK[1]
    h_l = liquid.enthalpy if liquid is not None else LIQUID_FALLBACK[2]
    c_l = liquid.sound_speed if liquid is not None else LIQUID_FALLBACK[3]
    sigma = fluid.surface_tension
    gas_beta = fluid.gas_beta
    u_sg = values["inlet_mass_flow"] * gas_beta / (rho_g * area)
    u_sl = values["inlet_mass_flow"] * (1.0 - gas_beta) / (rho_l * area)

    total_elevation = elevations[-1] - elevations[0] if sections_count > 0 else 0.0
    alpha_l_init = u_sl / (u_sl + u_sg + 1e-10)
    alpha_g_init = 1.0 - alpha_l_init
    rho_m = alpha_l_init * rho_l + alpha_g_init * rho_g
    if rho_m < 1.0:
        rho_m = rho_g
    hydrostatic = rho_m * GRAVITY * total_elevation
    mu_m = alpha_l_init * mu_l + alpha_g_init * mu_g
    if mu_m <= 0.0:
        mu_m = mu_g if mu_g > 0.0 else 1e-5
    u_m = u_sl + u_sg
    re = rho_m * u_m * values["diameter"] / mu_m
    f = 0.316 * re**-0.25 if re > 2300.0 else 64.0 / max(re, 1.0)
    friction = f * rho_m * u_m * u_m * values["length"] / (2.0 * values["diameter"])
    outlet_p = values.get("outlet_pressure")
    if outlet_p is None:
        outlet_p = values["inlet_p"] - hydrostatic - friction
    outlet_p = max(outlet_p, 1e5)

    for index, section in enumerate(sections):
        fraction = index / (sections_count - 1) if sections_count > 1 else 0.0
        section.pressure = values["inlet_p"] * (1.0 - fraction) + outlet_p * fraction
        section.temperature = values["inlet_t"]
        section.inclination = inclinations[index]
        section.elevation = elevations[index]
        section.gas_density = rho_g * section.pressure / values["inlet_p"]
        section.liquid_density = rho_l
        section.gas_viscosity = mu_g
        section.liquid_viscosity = mu_l
        section.surface_tension = sigma
        section.gas_enthalpy = h_g
        section.liquid_enthalpy = h_l
        section.gas_sound_speed = c_g
        section.liquid_sound_speed = c_l
        section.liquid_holdup = alpha_l_init
        section.gas_holdup = alpha_g_init
        section.liquid_velocity = u_sl / max(alpha_l_init, 0.01)
        section.gas_velocity = u_sg / max(1.0 - alpha_l_init, 0.01)
        section.update_derived()
        section.flow_regime = _flow_regime(section)

    max_time = values.get("max_simulation_time", DEFAULT_MAX_TIME)
    cfl_value = values.get("cfl", DEFAULT_CFL)
    thermo_interval = int(values.get("thermodynamic_update_interval", DEFAULT_THERMO_INTERVAL))
    pipe = _Pipe(
        mixture,
        _ideal,
        components,
        fluid,
        sections,
        values["inlet_mass_flow"],
        values["inlet_mass_flow"],
        values["inlet_p"],
        outlet_p,
        values["inlet_t"],
        cfl_value,
        max_time,
        thermo_interval,
        srk_shift,
        inlet_boundary,
        outlet_boundary,
    )
    pipe.run(list(inlet_z))

    return TransientPipeResult(
        pressure_profile=tuple(from_si(value, "Pa") for value in pipe.pressure_profile),
        temperature_profile=tuple(from_si(value, "K") for value in pipe.temperature_profile),
        liquid_holdup_profile=tuple(pipe.holdup_profile),
        gas_velocity_profile=tuple(from_si(value, "m/s") for value in pipe.gas_velocity_profile),
        liquid_velocity_profile=tuple(
            from_si(value, "m/s") for value in pipe.liquid_velocity_profile
        ),
        pressure_history=tuple(
            tuple(from_si(value, "Pa") for value in row) for row in pipe.history
        ),
        simulation_time=from_si(pipe.simulation_time, "s"),
        total_time_steps=pipe.total_steps,
        mass_inventory=from_si(pipe.mass_inventory, "kg"),
        warnings=tuple(warnings),
    )
