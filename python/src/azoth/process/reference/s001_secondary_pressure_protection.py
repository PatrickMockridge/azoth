"""``process.s001_secondary_pressure_protection`` - the NORSOK S-001 clause 10.4.7 screen.

As NeqSim's ``S001SecondaryPressureProtectionCriteria.evaluate`` computes it. Spec:
``specs/models/process/s001_secondary_pressure_protection.toml``

The Python twin of
``crates/azoth-process/src/models/s001_secondary_pressure_protection.rs``: the pressure
bands, the frequency, leakage and proof-test criteria.
"""

from __future__ import annotations

from typing import Any

from azoth.core.range import apply_checks, checks_for
from azoth.core.result import S001SecondaryPressureProtectionResult
from azoth.core.units import Q, input_to_si
from azoth.core.warnings import Warning
from azoth.process.reference._safety_review import s001_evaluate

MODEL_ID = "process.s001_secondary_pressure_protection"

_BARA_IN_PASCALS = 1.0e5


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def s001_secondary_pressure_protection(
    maximum_event_pressure: Q | None = None,
    design_pressure: Q | None = None,
    test_pressure: Q | None = None,
    demand_frequency_per_year: float | None = None,
    target_frequency_per_year: float | None = None,
    relief_leakage_assessed: bool | None = None,
    relief_leakage_to_safe_location: bool | None = None,
    proof_test_interval_months: float | None = None,
) -> S001SecondaryPressureProtectionResult:
    """The S-001 secondary-pressure screen.

    Args:
        maximum_event_pressure: the maximum event pressure in Pa.
        design_pressure: the protected equipment's design pressure in Pa.
        test_pressure: the pressure-test pressure in Pa.
        demand_frequency_per_year: the annual demand frequency.
        target_frequency_per_year: the stated target frequency; absent resolves the band.
        relief_leakage_assessed: whether relief leakage has been assessed.
        relief_leakage_to_safe_location: whether the leakage is routed to a safe location.
        proof_test_interval_months: the proof-test interval in months; it must be positive.

    Returns:
        The resolved target, the ten criteria flags and ``acceptable``.

    Raises:
        InvalidInputError: for a negative pressure or frequency, or a non-positive
            proof-test interval.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []
    event = (
        None
        if maximum_event_pressure is None
        else input_to_si(spec, "maximum_event_pressure", maximum_event_pressure)
    )
    design = (
        None if design_pressure is None else input_to_si(spec, "design_pressure", design_pressure)
    )
    test_pressure_pa = (
        None if test_pressure is None else input_to_si(spec, "test_pressure", test_pressure)
    )
    values = {
        "maximum_event_pressure": event,
        "design_pressure": design,
        "test_pressure": test_pressure_pa,
        "demand_frequency_per_year": demand_frequency_per_year,
        "target_frequency_per_year": target_frequency_per_year,
        "proof_test_interval_months": proof_test_interval_months,
    }
    apply_checks(checks.on_input, values.get, warnings)

    outcome = s001_evaluate(
        None if event is None else event / _BARA_IN_PASCALS,
        None if design is None else design / _BARA_IN_PASCALS,
        None if test_pressure_pa is None else test_pressure_pa / _BARA_IN_PASCALS,
        demand_frequency_per_year,
        target_frequency_per_year,
        relief_leakage_assessed,
        relief_leakage_to_safe_location,
        proof_test_interval_months,
    )

    return S001SecondaryPressureProtectionResult(
        target_frequency_per_year=outcome["target_frequency_per_year"],  # type: ignore[arg-type]
        pressure_basis_complete=bool(outcome["pressure_basis_complete"]),
        test_pressure_valid=bool(outcome["test_pressure_valid"]),
        pressure_within_test=bool(outcome["pressure_within_test"]),
        frequency_configured=bool(outcome["frequency_configured"]),
        frequency_criterion_met=bool(outcome["frequency_criterion_met"]),
        leakage_assessed=bool(outcome["leakage_assessed"]),
        leakage_routed_safe=bool(outcome["leakage_routed_safe"]),
        proof_test_configured=bool(outcome["proof_test_configured"]),
        proof_test_criterion_met=bool(outcome["proof_test_criterion_met"]),
        acceptable=bool(outcome["acceptable"]),
        warnings=tuple(warnings),
    )
