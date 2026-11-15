"""``process.nog070_sil_determination`` - the NOG 070 minimum-SIL table.

As NeqSim's ``Nog070SilDetermination`` computes it. Spec:
``specs/models/process/nog070_sil_determination.toml``

The Python twin of ``crates/azoth-process/src/models/nog070_sil_determination.rs``.
"""

from __future__ import annotations

import math

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import Nog070SilDeterminationResult
from azoth.core.warnings import Warning

MODEL_ID = "process.nog070_sil_determination"

_CATALOGUE = {
    "psd_process_segment": 2,
    "esd_topside_isolation": 2,
    "esd_subsea_isolation": 3,
    "hipps_pipeline": 3,
    "blowdown_hydrocarbon_segment": 2,
    "fg_gas_detection_esd": 2,
    "fg_fire_detection_esd": 2,
    "fg_hvac_shutdown": 2,
    "fg_deluge_release": 2,
    "psd_compressor_anti_surge": 2,
    "psd_high_vibration": 1,
    "psd_high_level": 2,
    "psd_low_low_level": 2,
    "psd_burner_management": 2,
    "esd_riser": 3,
    "esd_wellhead": 2,
    "esd_loading_release": 2,
    "custom": 0,
}


def _spec() -> dict[str, object]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _pfd_to_sil(pfd: float) -> int:
    if pfd >= 1.0e-1:
        return 0
    if pfd >= 1.0e-2:
        return 1
    if pfd >= 1.0e-3:
        return 2
    if pfd >= 1.0e-4:
        return 3
    return 4


def nog070_sil_determination(
    sif_type: str,
    pfd_average: float,
    minimum_sil: float | None = None,
) -> Nog070SilDeterminationResult:
    """The catalogue minimum, the applied minimum, the achieved SIL and compliance.

    Raises:
        InvalidInputError: for a SIF type the catalogue does not name, a PFD outside
            ``(0, 1)``, a minimum SIL outside ``[0, 4]`` or not whole, or a non-finite
            value.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    pfd = float(pfd_average)
    apply_checks(
        checks.on_input,
        {"pfd_average": pfd, "minimum_sil": minimum_sil}.get,
        warnings,
    )
    if sif_type not in _CATALOGUE:
        raise InvalidInputError("sif_type", f"the NOG 070 catalogue does not name {sif_type!r}")
    if not math.isfinite(pfd) or pfd <= 0.0 or pfd >= 1.0:
        raise InvalidInputError(
            "pfd_average", f"a PFD is above zero and below one, and {pfd} is not"
        )
    if minimum_sil is not None:
        value = float(minimum_sil)
        if not math.isfinite(value) or value != math.floor(value) or not 0.0 <= value <= 4.0:
            raise InvalidInputError(
                "minimum_sil",
                f"a minimum SIL is a whole number in [0, 4], and {value} is not",
            )
        minimum = int(value)
    else:
        minimum = _CATALOGUE[sif_type]

    achieved = _pfd_to_sil(pfd)
    compliant = achieved >= minimum
    factor = None if compliant else float(10 ** (minimum - achieved))

    return Nog070SilDeterminationResult(
        catalogue_minimum_sil=float(_CATALOGUE[sif_type]),
        minimum_sil=float(minimum),
        achieved_sil=float(achieved),
        compliant=compliant,
        required_pfd_reduction_factor=factor,
        warnings=tuple(warnings),
    )
