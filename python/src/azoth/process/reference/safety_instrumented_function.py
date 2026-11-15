"""``process.safety_instrumented_function`` - the SIF's IEC 61508 arithmetic.

As NeqSim's ``SafetyInstrumentedFunction`` computes it. Spec:
``specs/models/process/safety_instrumented_function.toml``

The Python twin of ``crates/azoth-process/src/models/safety_instrumented_function.rs``.
"""

from __future__ import annotations

import math

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import SafetyInstrumentedFunctionResult
from azoth.core.warnings import Warning, WarningCode

MODEL_ID = "process.safety_instrumented_function"

_MIN_PFD = (0.0, 0.01, 0.001, 0.0001, 0.00001)
_MAX_PFD = (1.0, 0.1, 0.01, 0.001, 0.0001)


def _spec() -> dict[str, object]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _required_sil(target_pfd: float) -> int:
    if target_pfd >= 0.01:
        return 1
    if target_pfd >= 0.001:
        return 2
    if target_pfd >= 0.0001:
        return 3
    return 4


def _max_pfd_for_sil(sil: int) -> float:
    return {1: 0.1, 2: 0.01, 3: 0.001, 4: 0.0001}.get(sil, 1.0)


def safety_instrumented_function(
    pfd_average: float,
    claimed_sil: float,
    unmitigated_frequency_per_year: float,
    target_frequency_per_year: float,
    test_interval_hours: float,
    lambda_du_per_hour: float | None = None,
    spurious_trip_rate_per_year: float | None = None,
    mttr_hours: float | None = None,
    base_availability: float | None = None,
) -> SafetyInstrumentedFunctionResult:
    """The RRF, the required PFD and SIL, the architecture PFDs and the availability.

    Raises:
        InvalidInputError: for a PFD outside ``[0, 1]``, a claimed SIL outside 1..4 or
            not whole, a negative frequency, interval, failure rate, spurious rate or
            repair time, a base availability outside ``[0, 1]``, or a non-finite value.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    pfd = float(pfd_average)
    sil_value = float(claimed_sil)
    unmitigated = float(unmitigated_frequency_per_year)
    target = float(target_frequency_per_year)
    interval = float(test_interval_hours)
    apply_checks(
        checks.on_input,
        {
            "pfd_average": pfd,
            "claimed_sil": sil_value,
            "unmitigated_frequency_per_year": unmitigated,
            "target_frequency_per_year": target,
            "test_interval_hours": interval,
            "lambda_du_per_hour": lambda_du_per_hour,
            "spurious_trip_rate_per_year": spurious_trip_rate_per_year,
            "mttr_hours": mttr_hours,
            "base_availability": base_availability,
        }.get,
        warnings,
    )
    for name, value in (
        ("pfd_average", pfd),
        ("claimed_sil", sil_value),
        ("unmitigated_frequency_per_year", unmitigated),
        ("target_frequency_per_year", target),
        ("test_interval_hours", interval),
    ):
        if not math.isfinite(value):
            raise InvalidInputError(
                name, "a non-finite value is not a number this calculation publishes"
            )
    if sil_value != math.floor(sil_value) or not 1.0 <= sil_value <= 4.0:
        raise InvalidInputError(
            "claimed_sil",
            f"a claimed SIL is a whole number in [1, 4], and {sil_value} is not",
        )
    sil = int(sil_value)
    for optional_name, optional_value in (
        ("lambda_du_per_hour", lambda_du_per_hour),
        ("spurious_trip_rate_per_year", spurious_trip_rate_per_year),
        ("mttr_hours", mttr_hours),
        ("base_availability", base_availability),
    ):
        if optional_value is not None and not math.isfinite(float(optional_value)):
            raise InvalidInputError(
                optional_name,
                "a non-finite value is not a number this calculation publishes",
            )

    risk_reduction_factor = 1.0 / pfd if pfd > 0.0 else None
    mitigated = unmitigated * pfd
    required_pfd = 1.0 if unmitigated <= 0.0 else target / unmitigated
    required_sil = float(_required_sil(required_pfd))
    max_pfd = _max_pfd_for_sil(sil)
    proof_years = interval / 8760.0
    within = _MIN_PFD[sil] <= pfd <= _MAX_PFD[sil]
    if not within:
        warnings.append(
            Warning(
                WarningCode.OUT_OF_VALID_RANGE,
                f"a PFD of {pfd:.2e} lies outside SIL {sil}'s band "
                f"[{_MIN_PFD[sil]:.2e}, {_MAX_PFD[sil]:.2e}]; the class prints the same "
                "warning and publishes the value",
                "pfd_average",
            )
        )

    if lambda_du_per_hour is not None:
        lambda_ti = float(lambda_du_per_hour) * interval
        pfd_1oo1: float | None = lambda_ti / 2.0
        pfd_1oo2: float | None = lambda_ti * lambda_ti / 3.0
        pfd_2oo3: float | None = lambda_ti * lambda_ti
    else:
        pfd_1oo1 = pfd_1oo2 = pfd_2oo3 = None

    if base_availability is not None and mttr_hours is not None:
        base = float(base_availability)
        mttr = float(mttr_hours)
        rate = float(spurious_trip_rate_per_year or 0.0)
        availability: float | None = base * (1.0 - rate * mttr / 8760.0) if rate > 0.0 else base
    else:
        availability = None

    return SafetyInstrumentedFunctionResult(
        risk_reduction_factor=risk_reduction_factor,
        mitigated_frequency_per_year=mitigated,
        required_pfd=required_pfd,
        required_sil=required_sil,
        max_pfd_for_claimed_sil=max_pfd,
        proof_test_interval_years=proof_years,
        pfd_within_claimed_sil=within,
        pfd_1oo1=pfd_1oo1,
        pfd_1oo2=pfd_1oo2,
        pfd_2oo3=pfd_2oo3,
        availability_with_spurious_trips=availability,
        warnings=tuple(warnings),
    )
