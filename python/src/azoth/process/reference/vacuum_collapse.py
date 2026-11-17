"""``process.vacuum_collapse`` - the blocked-in cooldown sweep.

As NeqSim's ``VacuumCollapseAnalyzer.analyze`` computes it. Spec:
``specs/models/process/vacuum_collapse.toml``

The Python twin of ``crates/azoth-process/src/models/vacuum_collapse.rs``: the TP flash that
fixes the molar volume, the constant-volume cooling sweep and the make-up gas.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple, Sequence

from azoth.core.errors import InvalidInputError, SolverNotConvergedError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import Phase, VacuumCollapseResult, VacuumVerdict
from azoth.core.units import Q, from_si, input_to_si, quantity
from azoth.core.warnings import Warning
from azoth.eos import components as _databank
from azoth.eos.reference.pt_flash import check_composition, pt_flash
from azoth.eos.reference.tv_flash import tv_flash

MODEL_ID = "process.vacuum_collapse"

#: The class's gas constant, J/(mol K).
R_GAS = 8.314462618
#: The class's default atmospheric pressure, Pa.
DEFAULT_ATMOSPHERIC_PRESSURE_PA = 101325.0
#: The class's default make-up gas molar mass, kg/mol (nitrogen).
DEFAULT_MAKEUP_MOLAR_MASS = 0.0280134
#: The class's default cooling steps.
DEFAULT_COOLING_STEPS = 20.0


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    final_pressure: float
    minimum_pressure: float
    minimum_temperature: float
    vacuum_depth: float
    margin: float
    vacuum_present: bool
    within_rating: bool
    makeup_moles: float
    makeup_mass: float
    verdict: VacuumVerdict


def _require_finite(name: str, value: float) -> None:
    """Refuse a value the arithmetic cannot carry to a published number."""
    if not math.isfinite(value):
        raise InvalidInputError(
            name, "a non-finite value is not a number this calculation publishes"
        )


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _evaluate(
    components: Sequence[str],
    mole_fractions: Sequence[float],
    vessel_volume: Q,
    initial_pressure: Q,
    initial_temperature: Q,
    cold_end_temperature: Q,
    atmospheric_pressure: Q | None = None,
    external_pressure_rating: Q | None = None,
    makeup_setpoint: Q | None = None,
    makeup_gas_molar_mass: Q | None = None,
    cooling_steps: float | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []

    volume = input_to_si(spec, "vessel_volume", vessel_volume)
    p0 = input_to_si(spec, "initial_pressure", initial_pressure)
    t0 = input_to_si(spec, "initial_temperature", initial_temperature)
    cold_end = input_to_si(spec, "cold_end_temperature", cold_end_temperature)
    atmospheric = (
        DEFAULT_ATMOSPHERIC_PRESSURE_PA
        if atmospheric_pressure is None
        else input_to_si(spec, "atmospheric_pressure", atmospheric_pressure)
    )
    rating = (
        0.0
        if external_pressure_rating is None
        else input_to_si(spec, "external_pressure_rating", external_pressure_rating)
    )
    setpoint = (
        atmospheric
        if makeup_setpoint is None
        else input_to_si(spec, "makeup_setpoint", makeup_setpoint)
    )
    makeup_molar_mass = (
        DEFAULT_MAKEUP_MOLAR_MASS
        if makeup_gas_molar_mass is None
        else input_to_si(spec, "makeup_gas_molar_mass", makeup_gas_molar_mass)
    )
    steps_input = None if cooling_steps is None else float(cooling_steps)
    apply_checks(
        checks_for(spec).on_input,
        {
            "vessel_volume": volume,
            "initial_pressure": p0,
            "initial_temperature": t0,
            "cooling_steps": steps_input,
        }.get,
        warnings,
    )

    for name, value in {
        "vessel_volume": volume,
        "initial_pressure": p0,
        "initial_temperature": t0,
        "cold_end_temperature": cold_end,
        "atmospheric_pressure": atmospheric,
        "external_pressure_rating": rating,
        "makeup_setpoint": setpoint,
        "makeup_gas_molar_mass": makeup_molar_mass,
        "cooling_steps": steps_input,
    }.items():
        if value is not None:
            _require_finite(name, value)
    if cold_end >= t0:
        raise InvalidInputError(
            "cold_end_temperature",
            "the cold-end temperature must be below the initial temperature for a cooling case",
        )
    steps = DEFAULT_COOLING_STEPS if steps_input is None else steps_input
    if steps < 1.0 or abs(steps - round(steps)) > 1.0e-9:
        raise InvalidInputError(
            "cooling_steps", "the step count must be a positive integer"
        )
    if atmospheric <= 0.0:
        raise InvalidInputError(
            "atmospheric_pressure", "the atmospheric pressure must be positive"
        )
    if rating < 0.0:
        raise InvalidInputError(
            "external_pressure_rating",
            "the external-pressure rating must be at least zero",
        )
    if setpoint <= 0.0:
        raise InvalidInputError(
            "makeup_setpoint", "the make-up setpoint must be positive"
        )
    if makeup_molar_mass <= 0.0:
        raise InvalidInputError(
            "makeup_gas_molar_mass", "the make-up gas molar mass must be positive"
        )

    z = [float(value) for value in mole_fractions]
    check_composition(z, len(components), "mole_fractions")
    mixture, ideal_gas = _databank.mixture_of(list(components), eos="pr")

    flash = pt_flash(mixture, quantity(t0, "K"), quantity(p0, "Pa"), z)
    if flash.phase == Phase.TWO_PHASE:
        beta = flash.vapour_fraction
        if beta is None:
            raise InvalidInputError(
                "mole_fractions", "a two-phase split has no vapour fraction"
            )
        compressibility = (1.0 - beta) * flash.z_liquid + beta * flash.z_vapour
    elif flash.phase == Phase.ALL_LIQUID:
        compressibility = flash.z_liquid
    else:
        compressibility = flash.z_vapour
    molar_volume = compressibility * R_GAS * t0 / p0

    count = int(round(steps))
    final_pressure = p0
    minimum_pressure = p0
    minimum_temperature = t0
    for index in range(1, count + 1):
        temperature = t0 + (cold_end - t0) * index / count
        try:
            state = tv_flash(
                mixture,
                ideal_gas,
                quantity(temperature, "K"),
                quantity(molar_volume, "m**3/mol"),
                z,
            )
        except (InvalidInputError, SolverNotConvergedError):
            if index == 1:
                raise
            break
        pressure = state.P.to("Pa").magnitude
        final_pressure = pressure
        if pressure < minimum_pressure:
            minimum_pressure = pressure
            minimum_temperature = temperature

    vacuum_depth = max(0.0, atmospheric - minimum_pressure)
    margin = minimum_pressure - rating
    vacuum_present = minimum_pressure < atmospheric
    within_rating = minimum_pressure >= rating
    if not vacuum_present:
        verdict = VacuumVerdict.NO_VACUUM
    elif within_rating:
        verdict = VacuumVerdict.VACUUM_WITHIN_RATING
    else:
        verdict = VacuumVerdict.VACUUM_EXCEEDS_RATING
    makeup_moles = (
        volume * (setpoint - minimum_pressure) / (R_GAS * minimum_temperature)
        if minimum_pressure < setpoint
        else 0.0
    )

    return (
        _Solution(
            final_pressure=final_pressure,
            minimum_pressure=minimum_pressure,
            minimum_temperature=minimum_temperature,
            vacuum_depth=vacuum_depth,
            margin=margin,
            vacuum_present=vacuum_present,
            within_rating=within_rating,
            makeup_moles=makeup_moles,
            makeup_mass=makeup_moles * makeup_molar_mass,
            verdict=verdict,
        ),
        warnings,
    )


def vacuum_collapse(
    components: Sequence[str],
    mole_fractions: Sequence[float],
    vessel_volume: Q,
    initial_pressure: Q,
    initial_temperature: Q,
    cold_end_temperature: Q,
    atmospheric_pressure: Q | None = None,
    external_pressure_rating: Q | None = None,
    makeup_setpoint: Q | None = None,
    makeup_gas_molar_mass: Q | None = None,
    cooling_steps: float | None = None,
) -> VacuumCollapseResult:
    """The constant-volume cooling sweep's minimum pressure and the make-up gas.

    The blocked-in inventory is the amount that fills the rigid vessel at the stated state,
    and the sweep holds its molar volume while a constant-volume flash runs at every cooling
    step. **An absent external rating is 0 bara**, the class's full-vacuum assumption; the
    default atmosphere is 101325 Pa, the default make-up is nitrogen and the default step
    count is 20.

    Args:
        components: the inventory's substances, by name.
        mole_fractions: the composition, one entry per component.
        vessel_volume: the rigid internal volume.
        initial_pressure: the blocked-in pressure.
        initial_temperature: the blocked-in temperature.
        cold_end_temperature: the coldest temperature, below the initial one.
        atmospheric_pressure: the pressure the vacuum is measured against.
        external_pressure_rating: the vessel's external-pressure rating.
        makeup_setpoint: the pressure the make-up gas holds.
        makeup_gas_molar_mass: the make-up gas molar mass.
        cooling_steps: the number of cooling steps.

    Returns:
        The final and minimum pressures, the vacuum depth and rating margin, the make-up
        gas and the verdict.

    Raises:
        InvalidInputError: for a composition that does not match the components or sums to
            one, a non-positive volume, pressure or temperature, a cold end at or above the
            initial temperature, a non-positive make-up pressure or molar mass, a
            non-integer step count below one, a fluid the databank cannot resolve, or a
            non-finite input.
    """
    solution, warnings = _evaluate(
        components,
        mole_fractions,
        vessel_volume,
        initial_pressure,
        initial_temperature,
        cold_end_temperature,
        atmospheric_pressure,
        external_pressure_rating,
        makeup_setpoint,
        makeup_gas_molar_mass,
        cooling_steps,
    )
    return VacuumCollapseResult(
        final_pressure=from_si(solution.final_pressure, "Pa"),
        minimum_pressure=from_si(solution.minimum_pressure, "Pa"),
        minimum_pressure_temperature=from_si(solution.minimum_temperature, "K"),
        vacuum_depth=from_si(solution.vacuum_depth, "Pa"),
        margin_to_rating=from_si(solution.margin, "Pa"),
        vacuum_present=solution.vacuum_present,
        within_rating=solution.within_rating,
        makeup_gas_moles=from_si(solution.makeup_moles, "mol"),
        makeup_gas_mass=from_si(solution.makeup_mass, "kg"),
        verdict=solution.verdict,
        warnings=tuple(warnings),
    )
