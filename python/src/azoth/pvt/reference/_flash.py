"""The two flashes the PVT experiments use, and the phases they answer with.

The Python twin of ``crates/azoth-pvt/src/flash.rs``. A phase is labelled with NeqSim's
``PhaseEos.init`` rule and the phases are ordered Gas, Oil, Aqueous, which is what makes the
classes' ``phase(0) = gas`` and ``phase(1) = oil`` reproducible without pretending the
multiphase solve promised an order.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from dataclasses import dataclass, field
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import Phase
from azoth.core.units import quantity
from azoth.core.warnings import Warning
from azoth.eos import pt_flash, tp_multiflash
from azoth.eos.reference._mixture_state import phase_state, reduced_parameters
from azoth.eos.reference.hydrate_inhibitor_wt import AQUEOUS, GAS, OIL, phase_label

from . import _volume

#: Which flash an experiment asks for.
TWO_PHASE = "two_phase"
MULTIPHASE = "multiphase"

_ORDER = {GAS: 0, OIL: 1, AQUEOUS: 2}


@dataclass(frozen=True)
class PhaseData:
    """One phase of a flash."""

    label: str
    fraction: float
    x: list[float]
    z_factor: float
    ln_phi: list[float]
    molar_mass: float
    molar_volume: float
    molar_volume_uncorrected: float

    def density(self) -> float:
        """Its mass density, in kg/m3."""
        return self.molar_mass / self.molar_volume

    def moles(self, total: float) -> float:
        """Its extensive amount at the flash's total, in mol."""
        return self.fraction * total

    def volume(self, total: float) -> float:
        """Its extensive volume at the flash's total, in m3."""
        return self.moles(total) * self.molar_volume

    def volume_uncorrected(self, total: float) -> float:
        """Its extensive untranslated volume at the flash's total, in m3."""
        return self.moles(total) * self.molar_volume_uncorrected


@dataclass
class Flash:
    """A converged flash."""

    phases: list[PhaseData]
    phase_count: int
    total_moles: float
    t: float
    p: float
    warnings: list[Warning] = field(default_factory=list)

    def gas(self) -> PhaseData | None:
        """The gas phase, if there is one."""
        return self._phase(GAS)

    def oil(self) -> PhaseData | None:
        """The oil phase, if there is one."""
        return self._phase(OIL)

    def aqueous(self) -> PhaseData | None:
        """The aqueous phase, if there is one."""
        return self._phase(AQUEOUS)

    def _phase(self, wanted: str) -> PhaseData | None:
        return next((phase for phase in self.phases if phase.label == wanted), None)

    def volume(self) -> float:
        """The extensive volume with the Peneloux translation applied where asked, m3."""
        return sum(phase.volume(self.total_moles) for phase in self.phases)

    def volume_uncorrected(self) -> float:
        """The extensive untranslated volume, m3."""
        return sum(phase.volume_uncorrected(self.total_moles) for phase in self.phases)

    def mass(self) -> float:
        """The flash's mass, in kg."""
        return sum(phase.moles(self.total_moles) * phase.molar_mass for phase in self.phases)

    def z_mix(self) -> float:
        """``sum_k beta_k Z_k``, the mixture compressibility."""
        return sum(phase.fraction * phase.z_factor for phase in self.phases)

    def zvolcorr(self) -> float:
        """``P V_corr/(n R T)`` of the whole flash."""
        return self.p * self.volume() / (self.total_moles * _volume.R * self.t)


def run(fluid: Any, t: float, p: float, mode: str) -> Flash:
    """Flash a fluid at a temperature and a pressure, both SI."""
    reduced = reduced_parameters(fluid.mixture, t, p)
    z = fluid.z()
    if mode == TWO_PHASE:
        phases, count, warnings = _two_phase(fluid, t, p, z, reduced)
    elif mode == MULTIPHASE:
        phases, count, warnings = _multiflash(fluid, t, p, z, reduced)
    else:
        raise InvalidInputError("mode", f"{mode!r} is not a flash mode")
    phases.sort(key=lambda phase: _ORDER[phase.label])
    return Flash(
        phases=phases,
        phase_count=count,
        total_moles=fluid.total_moles(),
        t=t,
        p=p,
        warnings=list(warnings),
    )


def _two_phase(
    fluid: Any, t: float, p: float, z: list[float], reduced: Any
) -> tuple[list[PhaseData], int, list[Warning]]:
    result = pt_flash(fluid.mixture, quantity(t, "K"), quantity(p, "Pa"), z)
    phases: list[PhaseData] = []
    if result.phase is Phase.TWO_PHASE:
        beta = result.vapour_fraction
        if beta is None:
            raise InvalidInputError(
                "flash", "a two-phase result carries no vapour fraction, so the split is unknown"
            )
        phases.append(
            _build(
                fluid, reduced, t, p, result.x, result.z_liquid, 1.0 - beta, result.ln_phi_liquid
            )
        )
        phases.append(
            _build(fluid, reduced, t, p, result.y, result.z_vapour, beta, result.ln_phi_vapour)
        )
        count = 2
    elif result.phase is Phase.ALL_LIQUID:
        # **The single phase is the feed, not the negative flash's hypothetical liquid.**
        # NeqSim clamps `beta` to zero and re-initialises at `z`, so the root and the
        # fugacity coefficients are recomputed there.
        state = phase_state(reduced, fluid.mixture.kij, z, liquid=True)
        phases.append(_build(fluid, reduced, t, p, z, state.z, 1.0, state.ln_phi))
        count = 1
    elif result.phase is Phase.ALL_VAPOUR:
        state = phase_state(reduced, fluid.mixture.kij, z, liquid=False)
        phases.append(_build(fluid, reduced, t, p, z, state.z, 1.0, state.ln_phi))
        count = 1
    else:
        root, ln_phi = _stable_root(fluid, reduced, z)
        phases.append(_build(fluid, reduced, t, p, z, root, 1.0, ln_phi))
        count = 1
    return phases, count, list(result.warnings)


def _stable_root(fluid: Any, reduced: Any, z: list[float]) -> tuple[float, list[float]]:
    """The stable single-phase root when the flash converged to ``x = y = z``."""
    liquid = phase_state(reduced, fluid.mixture.kij, z, liquid=True)
    vapour = phase_state(reduced, fluid.mixture.kij, z, liquid=False)

    def gibbs(ln_phi: list[float]) -> float:
        return math.fsum(fraction * ln for fraction, ln in zip(z, ln_phi, strict=True))

    if gibbs(liquid.ln_phi) <= gibbs(vapour.ln_phi):
        return liquid.z, list(liquid.ln_phi)
    return vapour.z, list(vapour.ln_phi)


def _multiflash(
    fluid: Any, t: float, p: float, z: list[float], reduced: Any
) -> tuple[list[PhaseData], int, list[Warning]]:
    result = tp_multiflash(fluid.mixture, quantity(t, "K"), quantity(p, "Pa"), z)
    phases: list[PhaseData] = []
    for index, composition in enumerate(result.x):
        fraction = result.phase_fractions[index]
        z_factor = result.z_factor[index]
        if not (math.isfinite(fraction) and fraction > 1.1e-12) or not math.isfinite(z_factor):
            continue
        ln_phi = result.ln_phi[index] if index < len(result.ln_phi) else []
        phases.append(
            _build(fluid, reduced, t, p, list(composition), z_factor, fraction, list(ln_phi))
        )
    return phases, int(result.phase_count), list(result.warnings)


def _build(
    fluid: Any,
    reduced: Any,
    t: float,
    p: float,
    x: Sequence[float],
    z_factor: float,
    fraction: float,
    ln_phi: Sequence[float],
) -> PhaseData:
    composition = list(x)
    return PhaseData(
        label=phase_label(fluid.mixture, reduced, composition, z_factor),
        fraction=fraction,
        x=composition,
        z_factor=z_factor,
        ln_phi=list(ln_phi),
        molar_mass=_volume.phase_molar_mass(fluid.mixture, composition),
        molar_volume=_volume.corrected_molar_volume(
            fluid.mixture, composition, z_factor, t, p, fluid.corrected
        ),
        molar_volume_uncorrected=_volume.molar_volume(z_factor, t, p),
    )
