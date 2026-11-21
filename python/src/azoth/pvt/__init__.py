"""PVT simulation.

NeqSim's ``pvtsimulation/`` without its ``flowassurance/``: the black-oil and gas-water
correlation tables, the decline and material-balance arithmetic, and the laboratory
experiments a reservoir fluid is described by. The namespace is its own rather than a family
under :mod:`azoth.eos` or :mod:`azoth.process`, mirroring NeqSim's package split: a PVT
experiment is a sequence of flashes and property reads, not an equation of state and not a
unit operation.

The correlation ids are scalar record lists; the laboratory ids take a fluid as
:class:`azoth.characterization.FluidComponent` rows and publish one row per pressure,
temperature or separator stage. Both languages build the fluid through
:func:`azoth.characterization.mixture_from_characterization`, so a pseudo-component crosses
the same way an assay cut does.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any

from azoth._dispatch import resolve
from azoth.core.result import BlackOilCorrelationsResult, GasCorrelationsResult
from azoth.pvt import layers as layers

_BLACK_OIL_CORRELATIONS = "pvt.black_oil_correlations"
_GAS_CORRELATIONS = "pvt.gas_correlations"

__all__ = ["black_oil_correlations", "gas_correlations", "layers"]


def black_oil_correlations(
    queries: Sequence[Mapping[str, Any]],
) -> BlackOilCorrelationsResult:
    """The black-oil correlation table over the stated calls.

    ``queries`` is one record per correlation call, keyed by the field names the spec
    declares: ``calculation`` selects the answer, and the row states only the inputs that
    call reads. A missing required field, a non-positive value where the correlation
    divides by it or takes its logarithm, and a non-finite answer are refused.

    The SI boundary is the id's: a solution gas-oil ratio in m3/m3, a temperature in K, a
    pressure in Pa and a viscosity in Pa*s. The two classes convert with different
    literals; the ``_p_sat`` calculations use ``SaturationPressureCorrelation``'s and the
    others ``BlackOilCorrelations``'.

    Returns:
        One row per input row, with only the fields the calculation answers.

    Raises:
        InvalidInputError: for a missing field, a non-positive input or a non-finite answer.

    See :func:`azoth.pvt.reference.black_oil_correlations.black_oil_correlations`.
    """
    return resolve(_BLACK_OIL_CORRELATIONS)(  # type: ignore[no-any-return]
        queries=queries,
    )


def gas_correlations(
    queries: Sequence[Mapping[str, Any]],
) -> GasCorrelationsResult:
    """The gas correlation table over the stated calls.

    ``queries`` states the call and only the inputs it reads: a gas specific gravity, the
    acid-gas mole fractions, an absolute temperature in K, a pressure in Pa and a molar mass
    in kg/mol. Pseudo-critical properties answer K and Pa, the Z factors are dimensionless,
    the density is kg/m3, and the pseudo-pressure is the class's bara^2/cP.

    Returns:
        One row per input row, with only the fields the calculation answers.

    Raises:
        InvalidInputError: for a missing field, a gas gravity outside the correlation's
            range, a reduced state Hall-Yarborough refuses, or a non-finite answer.

    See :func:`azoth.pvt.reference.gas_correlations.gas_correlations`.
    """
    return resolve(_GAS_CORRELATIONS)(  # type: ignore[no-any-return]
        queries=queries,
    )
