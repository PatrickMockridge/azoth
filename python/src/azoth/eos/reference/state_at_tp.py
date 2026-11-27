"""``eos.state_at_tp`` - one thermodynamic state in one call.

Spec: ``specs/models/eos/state_at_tp.toml``. The split is ``eos.pt_flash_check``'s; the
volumetric and caloric properties are ``molar_enthalpy_entropy``'s and
``phase_capacity``'s, evaluated at each phase's own root and composition-weighted. The
phase class is a screening label from the mixture's Kay pseudo-critical point; it is
not a stability analysis.
"""

from __future__ import annotations

import math

from azoth import _models_gen
from azoth.core.errors import InvalidInputError
from azoth.core.result import Phase, PhaseClass, StateAtTpResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.eos.mixture import Mixture
from azoth.eos.reference._entropy_state import molar_mass_of
from azoth.eos.reference._heat_capacity import phase_capacity
from azoth.eos.reference._mixture_state import reduced_parameters
from azoth.eos.reference.molar_enthalpy_entropy import IdealGasModel, molar_enthalpy_entropy
from azoth.eos.reference.pr_molar_volume import MOLAR_GAS_CONSTANT
from azoth.eos.reference.pt_flash_check import pt_flash_check

MODEL_ID = "eos.state_at_tp"


def _pseudo_critical(mixture: Mixture, z: list[float]) -> tuple[float, float]:
    """The Kay pseudo-critical temperature and pressure, K and Pa."""
    tc = 0.0
    pc = 0.0
    for component, fraction in zip(mixture.components, z, strict=True):
        tc += fraction * component.Tc.to("K").magnitude
        pc += fraction * component.Pc.to("Pa").magnitude
    return tc, pc


def _classify(phase: Phase, t: float, p: float, tc_pc: float, pc_pc: float) -> PhaseClass:
    if phase is Phase.TWO_PHASE:
        return PhaseClass.TWO_PHASE
    if p >= pc_pc and t >= tc_pc:
        return PhaseClass.DENSE_SUPERCRITICAL
    if p >= pc_pc:
        return PhaseClass.DENSE_SINGLE_PHASE
    if phase is Phase.ALL_LIQUID:
        return PhaseClass.SINGLE_PHASE_LIQUID
    if phase is Phase.ALL_VAPOUR:
        return PhaseClass.SINGLE_PHASE_GAS
    return PhaseClass.UNDETERMINED


def state_at_tp(
    mixture: Mixture,
    ideal_gas: IdealGasModel,
    T: Q,
    P: Q,
    z: list[float],
    tolerance: float | None = None,
    strict: bool | None = None,
) -> StateAtTpResult:
    """The split and the bulk volumetric and caloric properties at ``(T, P)``.

    The flash is ``eos.pt_flash_check``'s. ``tolerance`` overrides its stopping
    tolerance; ``strict = false`` reports ``converged = false`` instead of raising on a
    cap.

    See :func:`azoth.eos.reference.pt_flash_check.pt_flash_check`.
    """
    spec = _models_gen.model(MODEL_ID)
    t_si = input_to_si(spec, "T", T)
    p_si = input_to_si(spec, "P", P)
    for component in mixture.components:
        if component.molar_mass is None:
            raise InvalidInputError(
                "component", "molar mass", "a card-added component needs its own molar mass"
            )

    flash = pt_flash_check(mixture, T, P, z, tolerance, strict)
    reduced = reduced_parameters(mixture, t_si, p_si)
    kij = mixture.effective_kij(t_si)

    rows: list[tuple[float, list[float], float]]
    if flash.phase is Phase.TWO_PHASE:
        beta = flash.vapour_fraction
        if beta is None:  # pragma: no cover - a split always reports beta
            raise ValueError("the flash split the feed and reported no vapour fraction")
        rows = [
            (1.0 - beta, list(flash.x), flash.z_liquid),
            (beta, list(flash.y), flash.z_vapour),
        ]
    else:
        root = flash.z_liquid if flash.phase is Phase.ALL_LIQUID else flash.z_vapour
        rows = [(1.0, list(z), root)]

    molar_mass = 0.0
    molar_volume = 0.0
    enthalpy = 0.0
    cp = 0.0
    cv = 0.0
    for fraction, composition, root in rows:
        mass = molar_mass_of(mixture, composition)
        shift = mixture.volume_shift(composition)
        volume = root * MOLAR_GAS_CONSTANT * t_si / p_si - shift
        if not volume > 0.0 or not math.isfinite(volume):
            raise ValueError(f"the corrected molar volume at {p_si} Pa is not positive")
        state = molar_enthalpy_entropy(
            mixture, ideal_gas, T=T, P=P, z=composition, compressibility=root
        )
        capacity = phase_capacity(mixture, ideal_gas, reduced, kij, t_si, p_si, composition, root)
        molar_mass += fraction * mass
        molar_volume += fraction * volume
        enthalpy += fraction * float(state.h.to("J/mol").magnitude)
        cp += fraction * capacity.cp
        cv += fraction * capacity.cv

    if not molar_volume > 0.0 or not math.isfinite(molar_volume):
        raise ValueError(f"the bulk molar volume is {molar_volume} m3/mol")

    tc_pc, pc_pc = _pseudo_critical(mixture, z)
    return StateAtTpResult(
        phase=flash.phase,
        phase_class=_classify(flash.phase, t_si, p_si, tc_pc, pc_pc),
        converged=flash.converged,
        vapour_fraction=flash.vapour_fraction,
        x=flash.x,
        y=flash.y,
        molar_mass=from_si(molar_mass, "kg/mol"),
        molar_volume=from_si(molar_volume, "m**3/mol"),
        density=from_si(molar_mass / molar_volume, "kg/m**3"),
        internal_energy=from_si(enthalpy - p_si * molar_volume, "J/mol"),
        enthalpy=from_si(enthalpy, "J/mol"),
        cp=from_si(cp, "J/(mol*K)"),
        cv=from_si(cv, "J/(mol*K)"),
        compressibility=p_si * molar_volume / (MOLAR_GAS_CONSTANT * t_si),
        system_sound_speed=None,
        interface_tension=None,
        warnings=flash.warnings,
    )
