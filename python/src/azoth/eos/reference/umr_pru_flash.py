"""``eos.umr_pru_flash`` - the UMR-PRU isothermal flash.

Spec: ``specs/models/eos/umr_pru_flash.toml``. The fluid is
:func:`azoth.eos.components.umr_pru_mixture_of`'s: Peng-Robinson with the standard PR
alpha, the UMR rule on the ``_umr`` UNIFAC tables and NeqSim's quadratic co-volume
(``bmixType = 1``). The loop is ``eos.pt_flash``'s successive substitution; the spec
declares no fallback, so the second-order scheme - which needs the composition
derivatives the UMR rule does not provide - is never entered.
"""

from __future__ import annotations

from collections.abc import Sequence

from azoth.core.result import UmrPruFlashResult
from azoth.core.units import Q
from azoth.eos.components import umr_pru_mixture_of
from azoth.eos.reference.pt_flash import _pt_flash

MODEL_ID = "eos.umr_pru_flash"


def umr_pru_flash(
    components: Sequence[str],
    T: Q,
    P: Q,
    z: list[float],
    tolerance: float | None = None,
) -> UmrPruFlashResult:
    """The UMR-PRU flash at a temperature and pressure.

    Args:
        components: the substances by name; the databank and the keycard supply the
            critical constants, the cp polynomial and the UNIFAC group decomposition.
        T: absolute temperature.
        P: absolute pressure.
        z: overall mole fractions. Checked rather than renormalised.
        tolerance: the outer stopping tolerance, or ``None`` for the spec's 1e-10.

    Returns:
        The split and both phases' fugacity coefficients, with ``converged`` false
        only when the loop hit its cap.

    Raises:
        InvalidInputError: if a name is not in the databank, has no cp polynomial or
            no ``UNIFACcompUMRPRU`` decomposition, or ``z`` is malformed.
        OutOfRangeError: if ``T`` or ``P`` is not positive.
        SolverNotConvergedError: if a K-value becomes non-finite.

    See :func:`azoth.eos.reference.pt_flash._pt_flash`.
    """
    mixture, _ = umr_pru_mixture_of([str(name) for name in components])
    result, converged = _pt_flash(mixture, T, P, z, tolerance, False, model_id=MODEL_ID)
    return UmrPruFlashResult(
        vapour_fraction=result.vapour_fraction,
        x=result.x,
        y=result.y,
        k=result.k,
        ln_phi_liquid=result.ln_phi_liquid,
        ln_phi_vapour=result.ln_phi_vapour,
        z_liquid=result.z_liquid,
        z_vapour=result.z_vapour,
        min_t_over_tc=result.min_t_over_tc,
        phase=result.phase,
        converged=converged,
        iterations=result.iterations,
        residual=result.residual,
        warnings=result.warnings,
    )
