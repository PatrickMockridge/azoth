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
from azoth.core.result import (
    BlackOilCorrelationsResult,
    DeclineCurveResult,
    GasCorrelationsResult,
    WaterCorrelationsResult,
)
from azoth.pvt import layers as layers

_BLACK_OIL_CORRELATIONS = "pvt.black_oil_correlations"
_DECLINE_CURVE = "pvt.decline_curve"
_GAS_CORRELATIONS = "pvt.gas_correlations"
_WATER_CORRELATIONS = "pvt.water_correlations"

__all__ = [
    "black_oil_correlations",
    "decline_curve",
    "gas_correlations",
    "layers",
    "water_correlations",
]


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


def water_correlations(
    queries: Sequence[Mapping[str, Any]],
) -> WaterCorrelationsResult:
    """The water and brine correlation table over the stated calls.

    ``queries`` states the call and only the inputs it reads. The salinity convention is the
    calculation's own: ppm (mg/L) for the water properties, a weight fraction for the
    Batzle-Wang density, and mol/kg water for the McCain Rsw. Viscosity answers Pa*s from
    the class's cP, compressibility 1/Pa from its 1/bara, and surface tension N/m from its
    mN/m.

    Returns:
        One row per input row, with only the fields the calculation answers.

    Raises:
        InvalidInputError: for a missing field, a non-positive temperature or pressure, a
            negative salinity, a weight fraction outside [0, 0.3], or a non-finite answer.

    See :func:`azoth.pvt.reference.water_correlations.water_correlations`.
    """
    return resolve(_WATER_CORRELATIONS)(  # type: ignore[no-any-return]
        queries=queries,
    )


def decline_curve(
    calculation: str,
    *,
    qi: float | None = None,
    di: float | None = None,
    b: float | None = None,
    t: float | None = None,
    q_limit: float | None = None,
    d_nominal: float | None = None,
    d_effective: float | None = None,
    q1: float | None = None,
    t1: float | None = None,
    q2: float | None = None,
    t2: float | None = None,
    q3: float | None = None,
    t3: float | None = None,
    t_start: float | None = None,
    t_end: float | None = None,
    t_step: float | None = None,
    t_current: float | None = None,
    a: float | None = None,
    m: float | None = None,
    start_index: float | None = None,
    end_index: float | None = None,
    history_time: Sequence[float] | None = None,
    history_rate: Sequence[float] | None = None,
) -> DeclineCurveResult:
    """The decline-curve calculation over the stated inputs.

    Rates, times and cumulative volumes are unit-agnostic, the class's own statement; the
    day is the time unit and 365.25 days the year. ``calculation`` selects the answer, and
    only the inputs it reads are required. A zero decline leaves the time to the economic
    limit and the EUR absent, with a warning, rather than publishing an infinity.

    Returns:
        One result with the fields the calculation answers; the others are absent.

    Raises:
        InvalidInputError: for an unknown calculation, a missing required scalar, a
            non-positive rate where a fit needs one, a bad window, fewer than three
            history points, ``t2 <= t1``, a non-positive time step, or a non-finite answer.

    See :func:`azoth.pvt.reference.decline_curve.decline_curve`.
    """
    return resolve(_DECLINE_CURVE)(  # type: ignore[no-any-return]
        calculation=calculation,
        qi=qi,
        di=di,
        b=b,
        t=t,
        q_limit=q_limit,
        d_nominal=d_nominal,
        d_effective=d_effective,
        q1=q1,
        t1=t1,
        q2=q2,
        t2=t2,
        q3=q3,
        t3=t3,
        t_start=t_start,
        t_end=t_end,
        t_step=t_step,
        t_current=t_current,
        a=a,
        m=m,
        start_index=start_index,
        end_index=end_index,
        history_time=history_time,
        history_rate=history_rate,
    )
