"""``process.heavy_gas_dispersion`` - the Britter-McQuaid dense-gas distance.

Spec: ``specs/models/process/heavy_gas_dispersion.toml``

The Python twin of ``crates/azoth-process/src/models/heavy_gas_dispersion.rs``: the signed
reduced gravity, the unfloored characteristic parameter, and the fit with its ``0.01`` floor.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import HeavyGasDispersionResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.heavy_gas_dispersion"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _reduced_gravity(source_density: float, air_density: float) -> float:
    """The class's signed reduced gravity, in m/s2."""
    return 9.81 * (source_density - air_density) / air_density


def _britter_parameter(reduced: float, volumetric_rate: float, wind_speed: float) -> float:
    """The class's characteristic parameter, over ``|g0|`` and unfloored."""
    parameter: float = (abs(reduced) * abs(reduced) * volumetric_rate / wind_speed**5) ** 0.2
    return parameter


def _distance(volumetric_rate: float, wind_speed: float, alpha: float, ratio: float) -> float:
    """The class's fitted distance, with ``alpha`` floored at 0.01."""
    scale = math.sqrt(volumetric_rate / wind_speed)
    b = -0.25 * math.log10(max(alpha, 0.01))
    log_d = -0.85 * math.log10(ratio) + b
    distance: float = scale * 10.0**log_d
    return distance


def _require_positive(name: str, value: float) -> None:
    if not math.isfinite(value) or value <= 0.0:
        raise InvalidInputError(name, f"a finite positive value is required, and {value} is not")


def heavy_gas_dispersion(
    volumetric_rate: Q,
    source_density: Q,
    air_density: Q,
    wind_speed: Q,
    concentration_ratio: float | None = None,
) -> HeavyGasDispersionResult:
    """The reduced gravity, the Britter parameter and the distance to a concentration ratio.

    Raises:
        InvalidInputError: for a non-positive rate, density or wind speed, a ratio outside
            ``(0, 1)``, or a non-finite value.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    rate_si = input_to_si(spec, "volumetric_rate", volumetric_rate)
    source_si = input_to_si(spec, "source_density", source_density)
    air_si = input_to_si(spec, "air_density", air_density)
    wind_si = input_to_si(spec, "wind_speed", wind_speed)
    ratio = None if concentration_ratio is None else float(concentration_ratio)

    apply_checks(
        checks.on_input,
        {
            "volumetric_rate": rate_si,
            "source_density": source_si,
            "air_density": air_si,
            "wind_speed": wind_si,
            "concentration_ratio": ratio,
        }.get,
        warnings,
    )
    _require_positive("volumetric_rate", rate_si)
    _require_positive("source_density", source_si)
    _require_positive("air_density", air_si)
    _require_positive("wind_speed", wind_si)
    if ratio is not None and (not math.isfinite(ratio) or not 0.0 < ratio < 1.0):
        raise InvalidInputError(
            "concentration_ratio", f"a ratio in (0, 1) is required, and {ratio} is not"
        )

    reduced = _reduced_gravity(source_si, air_si)
    alpha = _britter_parameter(reduced, rate_si, wind_si)
    downwind = None if ratio is None else _distance(rate_si, wind_si, alpha, ratio)
    return HeavyGasDispersionResult(
        reduced_gravity=reduced,
        britter_parameter=alpha,
        distance_to_concentration_ratio=None if downwind is None else from_si(downwind, "m"),
        warnings=tuple(warnings),
    )
