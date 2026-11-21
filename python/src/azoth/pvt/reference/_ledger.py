"""Composition moves between experiment steps.

The Python twin of ``crates/azoth-pvt/src/ledger.rs``: each move is a pure function from one
:class:`~azoth.pvt.reference._fluid.Fluid` to the next, and the classes' mutation of a shared
system is not carried.
"""

from __future__ import annotations

import math

from azoth.core.errors import InvalidInputError

from . import _flash, _fluid


def remove_phase(fluid: _fluid.Fluid, phase: _flash.PhaseData) -> _fluid.Fluid:
    """The fluid after removing one flashed phase."""
    removed = phase.fraction * fluid.total_moles()
    moles = list(fluid.moles)
    for index, amount in enumerate(moles):
        taken = removed * phase.x[index]
        following = amount - taken
        if following < -1.0e-14 * max(abs(amount), 1.0):
            raise InvalidInputError(
                "phase",
                f"removing the phase takes {taken} mol of component {index}, and the fluid "
                f"holds {amount}",
            )
        moles[index] = max(following, 0.0)
    if math.fsum(moles) <= 0.0:
        raise InvalidInputError("phase", "removing the phase leaves no material in the cell")
    return fluid.with_moles(moles)


def add_amounts(fluid: _fluid.Fluid, amounts: list[float]) -> _fluid.Fluid:
    """The fluid after adding a component-wise amount."""
    if len(amounts) != len(fluid.moles):
        raise InvalidInputError(
            "amounts",
            f"the fluid has {len(fluid.moles)} components and the addition states {len(amounts)}",
        )
    moles = list(fluid.moles)
    for index, amount in enumerate(amounts):
        if not math.isfinite(amount) or amount < 0.0:
            raise InvalidInputError(f"amounts[{index}]", f"an addition is {amount} mol")
        moles[index] += amount
    return fluid.with_moles(moles)


def material_balance_error(initial: float, remaining: float, removed: float) -> float:
    """The relative material-balance error ``|initial - remaining - removed|/initial``."""
    return abs(initial - remaining - removed) / initial


def mass_fraction_of_phase(phase: _flash.PhaseData, total_moles: float, flash_mass: float) -> float:
    """A phase's mass fraction of the flash, ``beta M_phase / M_flash``."""
    return phase.moles(total_moles) * phase.molar_mass / flash_mass
