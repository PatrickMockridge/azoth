"""``process.runaway_reaction`` - the adiabatic runaway screen.

As NeqSim's ``RunawayReactionAnalyzer.analyze`` computes it. Spec:
``specs/models/process/runaway_reaction.toml``

The Python twin of ``crates/azoth-process/src/models/runaway_reaction.rs``: the adiabatic
rise, the MTSR and margin, the time to maximum rate and the verdict.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import RunawayReactionResult, RunawayVerdict
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.runaway_reaction"

#: The class's gas constant, J/(mol K).
R_GAS = 8.314462618
#: The class's default acceptable margin, K.
DEFAULT_ACCEPTABLE_MARGIN_K = 50.0
#: The class's high-severity rise, K.
HIGH_SEVERITY_DT_K = 200.0
#: The class's marginal-severity rise, K.
MARGINAL_SEVERITY_DT_K = 50.0


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    rise: float
    mtsr: float
    margin: float | None
    exceeded: bool
    heat: float
    tmr: float | None
    two_phase: bool
    verdict: RunawayVerdict


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
    process_temperature: Q,
    reaction_enthalpy: Q,
    limiting_reactant_moles: Q,
    total_mass: Q,
    specific_heat: Q,
    max_allowable_temperature: Q | None = None,
    activation_energy: Q | None = None,
    initial_heat_release_rate: float | None = None,
    acceptable_margin: Q | None = None,
    gassy_system: bool | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []

    t0 = input_to_si(spec, "process_temperature", process_temperature)
    enthalpy = input_to_si(spec, "reaction_enthalpy", reaction_enthalpy)
    reactant = input_to_si(spec, "limiting_reactant_moles", limiting_reactant_moles)
    mass = input_to_si(spec, "total_mass", total_mass)
    heat_capacity = input_to_si(spec, "specific_heat", specific_heat)
    material_limit = (
        None
        if max_allowable_temperature is None
        else input_to_si(spec, "max_allowable_temperature", max_allowable_temperature)
    )
    activation = (
        None
        if activation_energy is None
        else input_to_si(spec, "activation_energy", activation_energy)
    )
    rate = (
        None
        if initial_heat_release_rate is None
        else float(initial_heat_release_rate)
    )
    margin_input = (
        None
        if acceptable_margin is None
        else input_to_si(spec, "acceptable_margin", acceptable_margin)
    )
    apply_checks(
        checks_for(spec).on_input,
        {
            "process_temperature": t0,
            "reaction_enthalpy": enthalpy,
            "limiting_reactant_moles": reactant,
            "total_mass": mass,
            "specific_heat": heat_capacity,
            "max_allowable_temperature": material_limit,
            "activation_energy": activation,
            "initial_heat_release_rate": rate,
            "acceptable_margin": margin_input,
        }.get,
        warnings,
    )

    for name, value in {
        "process_temperature": t0,
        "reaction_enthalpy": enthalpy,
        "limiting_reactant_moles": reactant,
        "total_mass": mass,
        "specific_heat": heat_capacity,
        "max_allowable_temperature": material_limit,
        "activation_energy": activation,
        "initial_heat_release_rate": rate,
        "acceptable_margin": margin_input,
    }.items():
        if value is not None:
            _require_finite(name, value)
    if material_limit is not None and material_limit <= t0:
        raise InvalidInputError(
            "max_allowable_temperature",
            "the maximum-allowable temperature must exceed the process temperature",
        )

    heat = abs(enthalpy) * reactant
    rise = heat / (mass * heat_capacity)
    mtsr = t0 + rise

    if material_limit is None:
        margin = None
        exceeded = False
    else:
        margin = material_limit - mtsr
        exceeded = mtsr > material_limit

    if activation is not None and activation > 0.0 and rate is not None and rate > 0.0:
        tmr = heat_capacity * R_GAS * t0 * t0 / (rate * activation)
    else:
        tmr = None

    acceptable = DEFAULT_ACCEPTABLE_MARGIN_K if margin_input is None else margin_input
    if margin is None:
        if rise >= HIGH_SEVERITY_DT_K:
            verdict = RunawayVerdict.RUNAWAY_RISK
        elif rise >= MARGINAL_SEVERITY_DT_K:
            verdict = RunawayVerdict.MARGINAL
        else:
            verdict = RunawayVerdict.NO_RATING
    elif exceeded:
        verdict = RunawayVerdict.RUNAWAY_RISK
    elif margin < acceptable:
        verdict = RunawayVerdict.MARGINAL
    else:
        verdict = RunawayVerdict.ACCEPTABLE_MARGIN
    two_phase = bool(gassy_system) or exceeded

    return (
        _Solution(
            rise=rise,
            mtsr=mtsr,
            margin=margin,
            exceeded=exceeded,
            heat=heat,
            tmr=tmr,
            two_phase=two_phase,
            verdict=verdict,
        ),
        warnings,
    )


def runaway_reaction(
    process_temperature: Q,
    reaction_enthalpy: Q,
    limiting_reactant_moles: Q,
    total_mass: Q,
    specific_heat: Q,
    max_allowable_temperature: Q | None = None,
    activation_energy: Q | None = None,
    initial_heat_release_rate: float | None = None,
    acceptable_margin: Q | None = None,
    gassy_system: bool | None = None,
) -> RunawayReactionResult:
    """The adiabatic rise, MTSR, margin, time to maximum rate and runaway verdict.

    The enthalpy sign is ignored. Without a MAT the verdict reads the adiabatic rise alone:
    ``runaway_risk`` at 200 K and ``marginal`` at 50 K, both inclusive. With a MAT the
    exceeded test is strict and a positive margin below the acceptable margin is marginal.
    The TMR needs both a positive activation energy and a positive initial rate, the latter
    stated **in W/kg**.

    Args:
        process_temperature: the process temperature.
        reaction_enthalpy: the reaction enthalpy per mole of limiting reactant.
        limiting_reactant_moles: the limiting-reactant moles.
        total_mass: the reacting mass.
        specific_heat: the reacting mixture's specific heat.
        max_allowable_temperature: the MAT, above the process temperature.
        activation_energy: the apparent activation energy.
        initial_heat_release_rate: the specific heat-release rate in W/kg.
        acceptable_margin: the acceptable MAT-to-MTSR margin; absent is 50 K.
        gassy_system: whether the reaction generates permanent gas; absent is false.

    Returns:
        The adiabatic rise, MTSR, margin, total heat, TMR, two-phase flag and verdict.

    Raises:
        InvalidInputError: for a non-positive process temperature, limiting-reactant moles,
            reacting mass or specific heat, a MAT at or below the process temperature, or a
            non-finite input.
    """
    solution, warnings = _evaluate(
        process_temperature,
        reaction_enthalpy,
        limiting_reactant_moles,
        total_mass,
        specific_heat,
        max_allowable_temperature,
        activation_energy,
        initial_heat_release_rate,
        acceptable_margin,
        gassy_system,
    )
    return RunawayReactionResult(
        adiabatic_temperature_rise=from_si(solution.rise, "K"),
        mtsr=from_si(solution.mtsr, "K"),
        temperature_margin=(
            None if solution.margin is None else from_si(solution.margin, "K")
        ),
        margin_exceeded=solution.exceeded,
        total_heat_release=from_si(solution.heat, "J"),
        time_to_max_rate=(
            None if solution.tmr is None else from_si(solution.tmr, "s")
        ),
        two_phase_relief_screening_required=solution.two_phase,
        verdict=solution.verdict,
        warnings=tuple(warnings),
    )
