"""``process.overpressure_acceptance`` - the accumulated-pressure acceptance limit.

As NeqSim's ``OverpressureAcceptanceChecker`` computes it. Spec:
``specs/models/process/overpressure_acceptance.toml``

The Python twin of ``crates/azoth-process/src/models/overpressure_acceptance.rs``: the
three accumulation factors and the comparison in bara.
"""

from __future__ import annotations

from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import OverpressureAcceptanceResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.overpressure_acceptance"

_CAUSES = (
    "blocked_outlet",
    "check_valve_leakage",
    "gas_blow_by",
    "liquid_blow_by",
    "volatile_liquid_ingress",
    "inadvertent_valve_opening",
    "control_valve_failure",
    "fire",
    "thermal_expansion",
    "choke_collapse",
    "tube_rupture",
    "vacuum",
    "other",
)


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def overpressure_acceptance(
    peak_pressure: Q,
    mawp: Q,
    cause: str,
    multiple_devices: bool,
) -> OverpressureAcceptanceResult:
    """The accumulation factor, the allowable pressure and the acceptance verdict.

    Raises:
        InvalidInputError: for a cause the class does not name, or a non-positive or
            non-finite pressure.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []
    peak_si = input_to_si(spec, "peak_pressure", peak_pressure)
    mawp_si = input_to_si(spec, "mawp", mawp)
    apply_checks(
        checks.on_input,
        {"peak_pressure": peak_si, "mawp": mawp_si}.get,
        warnings,
    )
    if cause not in _CAUSES:
        raise InvalidInputError("cause", f"{cause!r} is not one of the class's overpressure causes")
    for name, value in (("peak_pressure", peak_si), ("mawp", mawp_si)):
        if not (value > 0.0) or value != value:
            raise InvalidInputError(
                name, f"a finite positive pressure is required, and {value} is not"
            )

    peak_bara = peak_si / 1.0e5
    mawp_bara = mawp_si / 1.0e5
    if cause == "fire":
        factor = 1.21
    elif multiple_devices:
        factor = 1.16
    else:
        factor = 1.10
    allowable_bara = mawp_bara * factor
    margin_bara = allowable_bara - peak_bara
    return OverpressureAcceptanceResult(
        accumulation_factor=factor,
        allowable_pressure=from_si(allowable_bara * 1.0e5, "Pa"),
        margin=from_si(margin_bara * 1.0e5, "Pa"),
        accepted=peak_bara <= allowable_bara,
        warnings=tuple(warnings),
    )
