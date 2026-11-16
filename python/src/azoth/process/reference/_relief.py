"""The overpressure family's thermo-bound helpers.

The Python twin of ``crates/azoth-process/src/models/overpressure.rs``:
``ReliefFluidState`` and ``NozzleFlow``, which are package-private in NeqSim's
``process/safety/overpressure/``. Not an id: the underscore keeps the stem off
``check_unported``'s attribution, and nothing here is registered.

**``ReliefFluidState`` reads ``getKappa``, not ``getGamma``.** NeqSim computes
``-Cp/Cv * V/P * getdPdVtn()``; the plain ``Cp/Cv`` is a different property. The capture
``process_blocked_outlet_relief.tsv`` prints both beside each state.

**The density the class computes is read by no formula** and is not computed here: none of
the four loads publishes it, and the study's density is a scenario-row input.
"""

from __future__ import annotations

import math
from dataclasses import dataclass, field
from typing import Any

from azoth.core.result import Phase, ReliefPhase
from azoth.core.units import quantity
from azoth.eos.reference._heat_capacity import phase_capacity
from azoth.eos.reference._mixture_state import phase_state, reduced_parameters
from azoth.eos.reference.pt_flash import pt_flash

#: ``NozzleFlow``'s gas constant, J/(mol K).
NOZZLE_R = 8.314462618
#: The class's default specific-heat ratio when the flash answers a non-physical one.
FALLBACK_KAPPA = 1.3
#: The class's default molar mass, kg/mol.
FALLBACK_MOLAR_MASS = 0.020
#: The class's default compressibility.
FALLBACK_COMPRESSIBILITY = 1.0
#: The class's default downstream pressure, Pa: 1.01325 bara.
DEFAULT_DOWNSTREAM_PRESSURE = 101_325.0
#: The class's default relieving temperature, K.
DEFAULT_TEMPERATURE = 288.15

#: ``ReliefFluidState``'s warning strings, which the class carries as assumption rows.
KAPPA_WARNING = (
    "Fluid Cp/Cv non-physical at relief conditions; using default specific-heat ratio 1.3"
)
MOLAR_MASS_WARNING = "Fluid molar mass non-physical; using default 0.020 kg/mol"
FLASH_WARNING = "Relief-fluid flash failed; using default vapour properties"


@dataclass(frozen=True, slots=True)
class ReliefFluidState:
    """The relieving-fluid snapshot, and the class's warning strings in their order."""

    specific_heat_ratio: float
    molar_mass: float
    compressibility: float
    phase: ReliefPhase
    assumptions: tuple[str, ...] = field(default_factory=tuple)


def _failed() -> ReliefFluidState:
    """The class's outer fallback: default vapour properties and its one warning."""
    return ReliefFluidState(
        specific_heat_ratio=FALLBACK_KAPPA,
        molar_mass=FALLBACK_MOLAR_MASS,
        compressibility=FALLBACK_COMPRESSIBILITY,
        phase=ReliefPhase.VAPOUR,
        assumptions=(FLASH_WARNING,),
    )


@dataclass(frozen=True, slots=True)
class PhaseProperties:
    """One phase's capacity and molar mass at a root."""

    molar_mass: float
    cp: float
    cv: float
    d_p_d_v: float
    root: float


def _molar_mass(mixture: Any, composition: list[float]) -> float:
    return float(
        sum(
            component.molar_mass.to("kg/mol").magnitude * fraction
            for component, fraction in zip(mixture.components, composition, strict=True)
        )
    )


def _phase_properties(
    mixture: Any,
    ideal_gas: Any,
    reduced: Any,
    kij: Any,
    t: float,
    p: float,
    composition: list[float],
    root: float,
) -> PhaseProperties:
    capacity = phase_capacity(mixture, ideal_gas, reduced, kij, t, p, composition, root)
    return PhaseProperties(
        molar_mass=_molar_mass(mixture, composition),
        cp=capacity.cp,
        cv=capacity.cv,
        d_p_d_v=capacity.d_p_d_v,
        root=root,
    )


def _single_phase_kappa(phase: PhaseProperties, t: float, p: float) -> float:
    """``-Cp/Cv * V/P * dPdV`` for one phase."""
    molar_volume = phase.root * NOZZLE_R * t / p
    return -(phase.cp / phase.cv) * (molar_volume / p) * phase.d_p_d_v


def _two_phase_kappa(
    vapour: PhaseProperties, liquid: PhaseProperties, beta: float, t: float, p: float
) -> float:
    """The same expression over the two phases, with NeqSim's own weighting.

    ``getCp``/``getCv`` are the phase totals, ``getVolume()`` is the beta-weighted molar
    volume, and ``getdPdVtn()`` sums each phase's derivative with respect to *that phase's*
    total volume, which is ``dP/dv_i`` over ``beta_i``.
    """
    cp = beta * vapour.cp + (1.0 - beta) * liquid.cp
    cv = beta * vapour.cv + (1.0 - beta) * liquid.cv
    molar_volume = (beta * vapour.root + (1.0 - beta) * liquid.root) * NOZZLE_R * t / p
    d_p_d_v = vapour.d_p_d_v / beta + liquid.d_p_d_v / (1.0 - beta)
    return -(cp / cv) * (molar_volume / p) * d_p_d_v


def relief_fluid_state(
    mixture: Any, ideal_gas: Any, t: float, p: float, z: list[float]
) -> ReliefFluidState:
    """``ReliefFluidState.evaluate``: flash at the relieving conditions and read the state.

    The class's non-physical-value fallbacks are reproduced, and their warning strings are
    returned in ``assumptions`` exactly as ``ReliefScenario.getAssumptions()`` carries them.
    """
    assumptions: list[str] = []
    try:
        reduced = reduced_parameters(mixture, t, p)
        kij = mixture.effective_kij(reduced.t_kelvin)
        flash = pt_flash(mixture, quantity(t, "K"), quantity(p, "Pa"), list(z))
    except Exception:
        return _failed()

    if flash.phase == Phase.TWO_PHASE:
        beta = flash.vapour_fraction
        if beta is None or not 0.0 < beta < 1.0:
            return _failed()
        try:
            vapour = _phase_properties(
                mixture, ideal_gas, reduced, kij, t, p, list(flash.y), flash.z_vapour
            )
            liquid = _phase_properties(
                mixture, ideal_gas, reduced, kij, t, p, list(flash.x), flash.z_liquid
            )
        except Exception:
            return _failed()
        kappa = _two_phase_kappa(vapour, liquid, beta, t, p)
        molar_mass = beta * vapour.molar_mass + (1.0 - beta) * liquid.molar_mass
        compressibility = beta * vapour.root + (1.0 - beta) * liquid.root
        phase = ReliefPhase.TWO_PHASE
    elif flash.phase in (Phase.ALL_VAPOUR, Phase.ALL_LIQUID):
        is_liquid = flash.phase == Phase.ALL_LIQUID
        try:
            root = phase_state(reduced, kij, list(z), liquid=is_liquid).z
            state = _phase_properties(mixture, ideal_gas, reduced, kij, t, p, list(z), root)
        except Exception:
            return _failed()
        kappa = _single_phase_kappa(state, t, p)
        molar_mass = state.molar_mass
        compressibility = state.root
        phase = ReliefPhase.LIQUID if is_liquid else ReliefPhase.VAPOUR
    else:
        return _failed()

    if math.isfinite(kappa) and kappa > 1.0:
        specific_heat_ratio = kappa
    else:
        assumptions.append(KAPPA_WARNING)
        specific_heat_ratio = FALLBACK_KAPPA
    if math.isfinite(molar_mass) and molar_mass > 0.0:
        mass = molar_mass
    else:
        assumptions.append(MOLAR_MASS_WARNING)
        mass = FALLBACK_MOLAR_MASS
    if math.isfinite(compressibility) and compressibility > 0.0:
        z_factor = compressibility
    else:
        z_factor = FALLBACK_COMPRESSIBILITY
    return ReliefFluidState(
        specific_heat_ratio=specific_heat_ratio,
        molar_mass=mass,
        compressibility=z_factor,
        phase=phase,
        assumptions=tuple(assumptions),
    )


def critical_pressure_ratio(k: float) -> float:
    """``NozzleFlow.criticalPressureRatio``: the choked ratio ``(2/(k+1))^(k/(k-1))``."""
    return float((2.0 / (k + 1.0)) ** (k / (k - 1.0)))


def gas_mass_rate(
    discharge_coefficient: float,
    area_m2: float,
    upstream_pressure_pa: float,
    downstream_pressure_pa: float,
    upstream_temperature_k: float,
    molar_mass_kg_per_mol: float,
    specific_heat_ratio: float,
) -> float:
    """``NozzleFlow.gasMassRateKgPerS``: the choked or subcritical isentropic flow, kg/s."""
    k = specific_heat_ratio
    ratio = downstream_pressure_pa / upstream_pressure_pa
    if ratio <= critical_pressure_ratio(k):
        return (
            discharge_coefficient
            * area_m2
            * upstream_pressure_pa
            * math.sqrt(
                k
                * molar_mass_kg_per_mol
                / (NOZZLE_R * upstream_temperature_k)
                * (2.0 / (k + 1.0)) ** ((k + 1.0) / (k - 1.0))
            )
        )
    term = ratio ** (2.0 / k) - ratio ** ((k + 1.0) / k)
    term = 0.0 if term < 0.0 else term
    return (
        discharge_coefficient
        * area_m2
        * upstream_pressure_pa
        * math.sqrt(
            2.0
            * molar_mass_kg_per_mol
            / (NOZZLE_R * upstream_temperature_k)
            * (k / (k - 1.0))
            * term
        )
    )
