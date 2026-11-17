"""``process.semenov_self_heating`` - the Semenov criticality screen.

As NeqSim's ``SemenovSelfHeatingAnalyzer.analyze`` computes it. Spec:
``specs/models/process/semenov_self_heating.toml``

The Python twin of ``crates/azoth-process/src/models/semenov_self_heating.rs``: the
Semenov parameter against ``1/e``, the critical-temperature bisection and the verdict.
"""

from __future__ import annotations

from typing import Any

from azoth.core.range import apply_checks, checks_for
from azoth.core.result import SelfHeatingVerdict, SemenovSelfHeatingResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning
from azoth.process.reference._selfheating import (
    BRACKET_WARNING,
    DEFAULT_MARGINAL_RATIO,
    PSI_CRIT,
    R_GAS,
    SEMENOV_WARNING,
    critical_temperature,
    semenov_psi,
    verdict_of,
)

MODEL_ID = "process.semenov_self_heating"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def semenov_self_heating(
    volume: Q,
    surface_area: Q,
    heat_transfer_coefficient: Q,
    activation_energy: Q,
    volumetric_pre_factor: float,
    ambient_temperature: Q,
    marginal_ratio: float | None = None,
) -> SemenovSelfHeatingResult:
    """The Semenov criticality screen for a thermally thin body.

    Raises:
        InvalidInputError: for a non-positive volume, surface area, heat-transfer coefficient,
            activation energy, pre-factor or ambient temperature, or a marginal ratio outside
            (0, 1).
    """
    spec = _spec()
    warnings: list[Warning] = []
    v = input_to_si(spec, "volume", volume)
    s = input_to_si(spec, "surface_area", surface_area)
    h = input_to_si(spec, "heat_transfer_coefficient", heat_transfer_coefficient)
    e = input_to_si(spec, "activation_energy", activation_energy)
    p = float(volumetric_pre_factor)
    ta = input_to_si(spec, "ambient_temperature", ambient_temperature)
    apply_checks(
        checks_for(spec).on_input,
        {
            "volume": v,
            "surface_area": s,
            "heat_transfer_coefficient": h,
            "activation_energy": e,
            "volumetric_pre_factor": p,
            "ambient_temperature": ta,
            "marginal_ratio": None if marginal_ratio is None else float(marginal_ratio),
        }.get,
        warnings,
    )

    marginal = DEFAULT_MARGINAL_RATIO if marginal_ratio is None else float(marginal_ratio)
    psi = semenov_psi(e, v, p, h, s, ta)
    psi_ratio = psi / PSI_CRIT
    critical = critical_temperature(
        lambda temperature: semenov_psi(e, v, p, h, s, temperature) - PSI_CRIT
    )

    messages = [] if critical is not None else [BRACKET_WARNING]
    messages.append(SEMENOV_WARNING)
    return SemenovSelfHeatingResult(
        psi=psi,
        psi_crit=PSI_CRIT,
        psi_ratio=psi_ratio,
        critical_temperature=None if critical is None else from_si(critical, "K"),
        temperature_margin=None if critical is None else from_si(critical - ta, "K"),
        critical_temperature_rise=from_si(R_GAS * ta * ta / e, "K"),
        verdict=SelfHeatingVerdict(verdict_of(psi_ratio, marginal)),
        messages=tuple(messages),
        warnings=tuple(warnings),
    )
