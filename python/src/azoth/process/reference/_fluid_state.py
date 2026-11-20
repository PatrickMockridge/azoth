"""The common flashed TP state the safety studies read.

The Python twin of ``crates/azoth-process/src/models/fluid_state.rs``: one flash, the
bulk and per-phase densities, the phase amounts and the capacity ratio ``Cp/Cv``,
weighted over the phases exactly as ``SystemThermo`` sums them.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.eos.reference._entropy_state import molar_mass_of
from azoth.eos.reference._heat_capacity import phase_capacity
from azoth.eos.reference._mixture_state import phase_state, reduced_parameters
from azoth.eos.reference.pr_molar_volume import MOLAR_GAS_CONSTANT
from azoth.eos.reference.pt_flash import pt_flash
from azoth.eos.reference.tp_multiflash import tp_multiflash


@dataclass(frozen=True)
class PhaseInfo:
    """One flashed phase."""

    mole_fraction: float
    composition: tuple[float, ...]
    z_factor: float
    molar_mass: float
    molar_volume: float
    density: float
    cp: float
    cv: float


@dataclass(frozen=True)
class FlashState:
    """A flashed state and the phases it carries."""

    temperature: float
    pressure: float
    bulk_density: float
    gamma: float
    gas_phase: int | None
    oil_phase: int | None
    aqueous_phase: int | None
    phases: tuple[PhaseInfo, ...]

    def gas_density(self) -> float:
        """The gas phase's density, or the bulk density when no gas phase exists."""
        if self.gas_phase is None:
            return self.bulk_density
        return self.phases[self.gas_phase].density

    def liquid_density(self, fallback: float) -> float:
        """The liquid density: oil, then aqueous, then the fallback."""
        index = self.oil_phase if self.oil_phase is not None else self.aqueous_phase
        return fallback if index is None else self.phases[index].density

    def has_gas(self) -> bool:
        """Whether the flash found a gas phase."""
        return self.gas_phase is not None

    def gas_mole_fraction(self) -> float:
        """The gas phase's mole fraction of the feed."""
        if self.gas_phase is None:
            return 0.0
        return self.phases[self.gas_phase].mole_fraction


def _phase_info(
    mixture: Any,
    ideal_gas: Any,
    reduced: Any,
    kij: Any,
    t: float,
    p: float,
    composition: list[float],
    root: float,
    mole_fraction: float,
) -> PhaseInfo:
    molar_mass = molar_mass_of(mixture, composition)
    shift = mixture.volume_shift(composition)
    molar_volume = root * MOLAR_GAS_CONSTANT * t / p - shift
    if not molar_volume > 0.0 or not math.isfinite(molar_volume):
        raise InvalidInputError(
            "flash", f"the corrected molar volume at {p:.6f} Pa is not positive"
        )
    capacity = phase_capacity(mixture, ideal_gas, reduced, kij, t, p, composition, root)
    return PhaseInfo(
        mole_fraction=mole_fraction,
        composition=tuple(composition),
        z_factor=root,
        molar_mass=molar_mass,
        molar_volume=molar_volume,
        density=molar_mass / molar_volume,
        cp=capacity.cp,
        cv=capacity.cv,
    )


def _assemble(
    t: float,
    p: float,
    phases: list[PhaseInfo],
    gas_phase: int | None,
    oil_phase: int | None,
    aqueous_phase: int | None,
) -> FlashState:
    mass = sum(phase.mole_fraction * phase.molar_mass for phase in phases)
    volume = sum(phase.mole_fraction * phase.molar_volume for phase in phases)
    cp = sum(phase.mole_fraction * phase.cp for phase in phases)
    cv = sum(phase.mole_fraction * phase.cv for phase in phases)
    return FlashState(
        temperature=t,
        pressure=p,
        bulk_density=mass / volume,
        gamma=cp / cv,
        gas_phase=gas_phase,
        oil_phase=oil_phase,
        aqueous_phase=aqueous_phase,
        phases=tuple(phases),
    )


def _water_index(mixture: Any) -> int | None:
    index: int | None = mixture.index_of("water")
    return None if index is None else int(index)


def _is_aqueous(composition: list[float], water: int | None) -> bool:
    if water is None or water >= len(composition):
        return False
    return composition[water] > 0.5


def _from_ordinary(
    mixture: Any,
    ideal_gas: Any,
    reduced: Any,
    kij: Any,
    t: float,
    p: float,
    z: list[float],
    flash: Any,
) -> FlashState:
    water = _water_index(mixture)
    if flash.phase == "two_phase":
        beta = flash.vapour_fraction
        if beta is None:
            raise InvalidInputError(
                "flash", "the flash split the feed and reported no vapour fraction"
            )
        vapour = _phase_info(
            mixture, ideal_gas, reduced, kij, t, p, list(flash.y), flash.z_vapour, beta
        )
        liquid = _phase_info(
            mixture, ideal_gas, reduced, kij, t, p, list(flash.x), flash.z_liquid, 1.0 - beta
        )
        aqueous = _is_aqueous(list(flash.x), water)
        return _assemble(
            t,
            p,
            [vapour, liquid],
            0,
            None if aqueous else 1,
            1 if aqueous else None,
        )
    if flash.phase == "all_vapour":
        root = phase_state(reduced, kij, z, liquid=False).z
        phase = _phase_info(mixture, ideal_gas, reduced, kij, t, p, z, root, 1.0)
        return _assemble(t, p, [phase], 0, None, None)
    if flash.phase == "all_liquid":
        root = phase_state(reduced, kij, z, liquid=True).z
        phase = _phase_info(mixture, ideal_gas, reduced, kij, t, p, z, root, 1.0)
        aqueous = _is_aqueous(z, water)
        return _assemble(t, p, [phase], None, None if aqueous else 0, 0 if aqueous else None)
    raise InvalidInputError(
        "flash", "the feed flash converged to a trivial solution, which names no phase"
    )


def flash(
    mixture: Any,
    ideal_gas: Any,
    temperature: float,
    pressure: float,
    z: list[float],
    multiphase: bool,
) -> FlashState:
    """Flash a feed at a temperature and pressure."""
    t = quantity(temperature, "K")
    p = quantity(pressure, "Pa")
    reduced = reduced_parameters(mixture, temperature, pressure)
    kij = mixture.effective_kij(reduced.t_kelvin)
    ordinary = pt_flash(mixture, t, p, z)
    if not multiphase or ordinary.phase == "trivial":
        return _from_ordinary(mixture, ideal_gas, reduced, kij, temperature, pressure, z, ordinary)

    split = tp_multiflash(mixture, t, p, z)
    if split.phase_count < 2:
        return _from_ordinary(mixture, ideal_gas, reduced, kij, temperature, pressure, z, ordinary)

    phases: list[PhaseInfo] = []
    for index, composition in enumerate(split.x):
        mole_fraction = split.phase_fractions[index]
        root = split.z_factor[index]
        if not mole_fraction > 0.0 or not math.isfinite(root) or root <= 0.0:
            continue
        phases.append(
            _phase_info(
                mixture,
                ideal_gas,
                reduced,
                kij,
                temperature,
                pressure,
                list(composition),
                root,
                mole_fraction,
            )
        )
    if not phases:
        raise InvalidInputError("flash", "the multiphase flash kept no phase")

    water = _water_index(mixture)
    gas_phase: int | None = None
    if ordinary.phase != "all_liquid":
        gas_phase = max(range(len(phases)), key=lambda index: phases[index].z_factor)
    oil_phase: int | None = None
    aqueous_phase: int | None = None
    for index, phase in enumerate(phases):
        if index == gas_phase:
            continue
        if _is_aqueous(list(phase.composition), water):
            if aqueous_phase is None:
                aqueous_phase = index
        elif oil_phase is None:
            oil_phase = index
    return _assemble(temperature, pressure, phases, gas_phase, oil_phase, aqueous_phase)
