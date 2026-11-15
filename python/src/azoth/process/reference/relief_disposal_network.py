"""``process.relief_disposal_network`` - the simultaneous relief-load summation.

As NeqSim's ``ReliefDisposalNetwork`` computes it. Spec:
``specs/models/process/relief_disposal_network.toml``

The Python twin of ``crates/azoth-process/src/models/relief_disposal_network.rs``: the
simultaneous total, the peak single contribution and the row echoes.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import ReliefDisposalNetworkResult, ReliefDisposalRow
from azoth.core.units import from_si, input_records_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.relief_disposal_network"

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
_PHASES = ("vapour", "liquid", "two_phase")


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def relief_disposal_network(
    contributions: list[dict[str, object]],
) -> ReliefDisposalNetworkResult:
    """The simultaneous total, the peak single load and the echoed rows.

    Raises:
        InvalidInputError: for a negative or non-finite rate, or a cause or phase the
            class does not name.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []
    apply_checks(checks.on_input, lambda _: None, warnings)

    rows: list[dict[str, Any]] = input_records_to_si(spec, "contributions", contributions)
    total = 0.0
    peak = 0.0
    governing: str | None = None
    echoed: list[ReliefDisposalRow] = []
    for index, row in enumerate(rows):
        rate = float(row["mass_flow"])
        if not math.isfinite(rate) or rate < 0.0:
            raise InvalidInputError(
                f"contributions[{index}].mass_flow",
                f"a finite non-negative rate is required, and {rate} is not",
            )
        cause = str(row["cause"])
        phase = str(row["phase"])
        if cause not in _CAUSES:
            raise InvalidInputError(
                "cause", f"{cause!r} is not one of the class's overpressure causes"
            )
        if phase not in _PHASES:
            raise InvalidInputError("phase", f"{phase!r} is not 'vapour', 'liquid' or 'two_phase'")
        simultaneous = bool(row["simultaneous"])
        if simultaneous:
            total += rate
        if rate > peak:
            peak = rate
            governing = str(row["item_name"])
        echoed.append(
            ReliefDisposalRow(
                item_name=str(row["item_name"]),
                cause=cause,
                phase=phase,
                mass_flow=from_si(rate, "kg/s"),
                mass_flow_kg_hr=from_si(rate, "kg/h"),
                simultaneous=simultaneous,
            )
        )
    return ReliefDisposalNetworkResult(
        total_simultaneous=from_si(total, "kg/s"),
        total_simultaneous_kg_hr=from_si(total, "kg/h"),
        peak_single=from_si(peak, "kg/s"),
        governing_contributor=governing,
        contributions=tuple(echoed),
        warnings=tuple(warnings),
    )
