"""``eos.pt_flash_check`` - the isothermal flash with a convergence flag.

Spec: ``specs/models/eos/pt_flash_check.toml``. The arithmetic is ``eos.pt_flash``'s
(``_pt_flash`` in this package); this module exposes the caller's tolerance and turns
the iteration cap into a result when ``strict = false``.
"""

from __future__ import annotations

from azoth.core.result import PtFlashCheckResult
from azoth.core.units import Q
from azoth.eos.mixture import Mixture
from azoth.eos.reference.pt_flash import _pt_flash


def pt_flash_check(
    mixture: Mixture,
    T: Q,
    P: Q,
    z: list[float],
    tolerance: float | None = None,
    strict: bool | None = None,
) -> PtFlashCheckResult:
    """The flash with a settable tolerance and a non-throwing cap.

    ``strict = true`` (the absent default) raises on a cap. ``strict = false`` returns
    the last iterate with ``converged = false`` and a ``SOLVER_NOT_CONVERGED``
    warning. A non-finite K-value still raises under either setting.

    See :func:`azoth.eos.reference.pt_flash._pt_flash`.
    """
    result, converged = _pt_flash(mixture, T, P, z, tolerance, strict)
    return PtFlashCheckResult(
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
