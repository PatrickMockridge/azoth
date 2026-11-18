"""``process.flow_induced_pulsation_screening`` - the side-branch resonance screen.

As NeqSim's ``FlowInducedPulsationScreening.screen`` computes it. Spec:
``specs/models/process/flow_induced_pulsation_screening.toml``

The Python twin of ``crates/azoth-process/src/models/flow_induced_pulsation_screening.rs``:
the effective width, the acoustic eigenfrequencies, the lock-in envelopes and the severity
band.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import (
    FivLikelihood,
    FlowInducedPulsationResult,
    PulsationModeResult,
)
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.flow_induced_pulsation_screening"

#: The class's default screening Strouhal number for the side-branch modes.
DEFAULT_STROUHAL_NUMBER = 0.37
#: The class's default number of acoustic modes.
DEFAULT_MODE_COUNT = 5.0
#: Half-width of the lock-in envelope, as a fraction.
LOCK_IN_ENVELOPE_FRACTION = 0.20
#: The reference momentum flux used to grade the severity, Pa.
REFERENCE_RHO_V2 = 20000.0
#: Severity below which a locked mode is not worth pursuing.
NEGLIGIBLE_SEVERITY = 0.05


class _Mode(NamedTuple):
    """One mode's SI numbers, before the row's units are attached."""

    mode_index: float
    frequency: float
    envelope_low: float
    envelope_high: float
    locked_in: bool
    resonance_velocity: float
    resonance_velocity_low: float
    resonance_velocity_high: float


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    likelihood: FivLikelihood
    any_mode_locked_in: bool
    shedding_frequency: float
    lowest_resonance_velocity: float
    effective_width: float
    diameter_ratio: float
    rho_v2: float
    energy_term: float
    coupling_factor: float
    mode_weight: float
    severity: float
    modes: tuple[_Mode, ...]


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


def _band_for(any_mode_locked_in: bool, severity: float) -> FivLikelihood:
    """The class's band from the resonance state and severity."""
    if not any_mode_locked_in or severity < NEGLIGIBLE_SEVERITY:
        return FivLikelihood.LOW
    if severity < 0.3:
        return FivLikelihood.MEDIUM
    if severity < 1.0:
        return FivLikelihood.HIGH
    return FivLikelihood.VERY_HIGH


def _eigen_frequency(
    mode_index: int, speed_of_sound: float, acoustic_length: float, open_termination: bool
) -> float:
    """The acoustic eigenfrequency of one mode."""
    if open_termination:
        return (mode_index + 1) * speed_of_sound / (2.0 * acoustic_length)
    return (2 * mode_index + 1) * speed_of_sound / (4.0 * acoustic_length)


def _evaluate(
    acoustic_length: Q,
    branch_internal_diameter: Q,
    run_internal_diameter: Q,
    main_line_velocity: Q,
    main_line_density: Q,
    speed_of_sound: Q,
    edge_radius: Q | None = None,
    termination: str | None = None,
    strouhal_number: float | None = None,
    mode_count: float | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []

    length_si = input_to_si(spec, "acoustic_length", acoustic_length)
    branch_si = input_to_si(spec, "branch_internal_diameter", branch_internal_diameter)
    run_si = input_to_si(spec, "run_internal_diameter", run_internal_diameter)
    velocity_si = input_to_si(spec, "main_line_velocity", main_line_velocity)
    density_si = input_to_si(spec, "main_line_density", main_line_density)
    sound_si = input_to_si(spec, "speed_of_sound", speed_of_sound)
    edge_si = 0.0 if edge_radius is None else input_to_si(spec, "edge_radius", edge_radius)
    strouhal = DEFAULT_STROUHAL_NUMBER if strouhal_number is None else float(strouhal_number)
    count = DEFAULT_MODE_COUNT if mode_count is None else float(mode_count)
    termination_name = "closed" if termination is None else str(termination)
    apply_checks(
        checks_for(spec).on_input,
        {
            "acoustic_length": length_si,
            "branch_internal_diameter": branch_si,
            "run_internal_diameter": run_si,
            "main_line_velocity": velocity_si,
            "main_line_density": density_si,
            "speed_of_sound": sound_si,
            "edge_radius": edge_si,
            "strouhal_number": strouhal,
            "mode_count": count,
        }.get,
        warnings,
    )
    for name, value in (
        ("acoustic_length", length_si),
        ("branch_internal_diameter", branch_si),
        ("run_internal_diameter", run_si),
        ("main_line_velocity", velocity_si),
        ("main_line_density", density_si),
        ("speed_of_sound", sound_si),
        ("edge_radius", edge_si),
        ("strouhal_number", strouhal),
        ("mode_count", count),
    ):
        _require_finite(name, value)
    if not float(count).is_integer():
        raise InvalidInputError("mode_count", "the acoustic mode count is a whole number")
    if termination_name not in ("closed", "open"):
        raise InvalidInputError(
            "termination", "the acoustic termination must be `closed` or `open`"
        )
    open_termination = termination_name == "open"

    effective_width = math.pi * branch_si / 4.0 + edge_si
    shedding_frequency = strouhal * velocity_si / effective_width
    diameter_ratio = branch_si / run_si
    rho_v2 = density_si * velocity_si * velocity_si

    modes: list[_Mode] = []
    any_locked_in = False
    strongest_weight = 0.0
    lowest_resonance_velocity = math.nan
    for mode_index in range(int(count)):
        frequency = _eigen_frequency(mode_index, sound_si, length_si, open_termination)
        envelope_low = (1.0 - LOCK_IN_ENVELOPE_FRACTION) * frequency
        envelope_high = (1.0 + LOCK_IN_ENVELOPE_FRACTION) * frequency
        locked_in = envelope_low <= shedding_frequency <= envelope_high
        resonance_velocity = frequency * effective_width / strouhal
        if locked_in:
            any_locked_in = True
            strongest_weight = max(strongest_weight, 1.0 / (2 * mode_index + 1))
        if not math.isfinite(lowest_resonance_velocity) or (
            resonance_velocity < lowest_resonance_velocity
        ):
            lowest_resonance_velocity = resonance_velocity
        modes.append(
            _Mode(
                mode_index=float(mode_index),
                frequency=frequency,
                envelope_low=envelope_low,
                envelope_high=envelope_high,
                locked_in=locked_in,
                resonance_velocity=resonance_velocity,
                resonance_velocity_low=(1.0 - LOCK_IN_ENVELOPE_FRACTION) * resonance_velocity,
                resonance_velocity_high=(1.0 + LOCK_IN_ENVELOPE_FRACTION) * resonance_velocity,
            )
        )

    energy_term = rho_v2 / REFERENCE_RHO_V2
    coupling_factor = min(diameter_ratio / 0.5, 1.0)
    severity = energy_term * coupling_factor * strongest_weight

    return (
        _Solution(
            likelihood=_band_for(any_locked_in, severity),
            any_mode_locked_in=any_locked_in,
            shedding_frequency=shedding_frequency,
            lowest_resonance_velocity=lowest_resonance_velocity,
            effective_width=effective_width,
            diameter_ratio=diameter_ratio,
            rho_v2=rho_v2,
            energy_term=energy_term,
            coupling_factor=coupling_factor,
            mode_weight=strongest_weight,
            severity=severity,
            modes=tuple(modes),
        ),
        warnings,
    )


def flow_induced_pulsation_screening(
    acoustic_length: Q,
    branch_internal_diameter: Q,
    run_internal_diameter: Q,
    main_line_velocity: Q,
    main_line_density: Q,
    speed_of_sound: Q,
    edge_radius: Q | None = None,
    termination: str | None = None,
    strouhal_number: float | None = None,
    mode_count: float | None = None,
) -> FlowInducedPulsationResult:
    """The side-branch pulsation likelihood and every evaluated mode.

    A closed branch is a quarter-wave tube and an open one a half-wave tube; a mode is
    locked in inside ``[0.8 f_n, 1.2 f_n]``, both ends inclusive. Only the strongest locked
    mode's ``1/(2n+1)`` weight enters the severity, and a branch with no lock-in or a
    severity below 0.05 is low. The class's recommendation text is presentation.

    Args:
        acoustic_length: the centreline length to the first acoustic boundary.
        branch_internal_diameter: the branch internal diameter.
        run_internal_diameter: the main-line internal diameter.
        main_line_velocity: the mean main-line velocity.
        main_line_density: the main-line density.
        speed_of_sound: the speed of sound in the branch fluid.
        edge_radius: the tee edge radius; absent is zero.
        termination: ``closed`` or ``open``; absent is closed.
        strouhal_number: the screening Strouhal number; absent is 0.37.
        mode_count: the number of modes; absent is five.

    Returns:
        The likelihood, the shedding frequency, the resonance summary and every mode.

    Raises:
        InvalidInputError: for a non-positive acoustic length, diameter, speed of sound or
            Strouhal number, a negative edge radius, velocity or density, a mode count below
            one or fractional, an unknown termination, or a non-finite input.
    """
    solution, warnings = _evaluate(
        acoustic_length,
        branch_internal_diameter,
        run_internal_diameter,
        main_line_velocity,
        main_line_density,
        speed_of_sound,
        edge_radius=edge_radius,
        termination=termination,
        strouhal_number=strouhal_number,
        mode_count=mode_count,
    )
    return FlowInducedPulsationResult(
        likelihood=solution.likelihood,
        any_mode_locked_in=solution.any_mode_locked_in,
        shedding_frequency=from_si(solution.shedding_frequency, "1/s"),
        lowest_resonance_velocity=from_si(solution.lowest_resonance_velocity, "m/s"),
        effective_width=from_si(solution.effective_width, "m"),
        diameter_ratio=solution.diameter_ratio,
        rho_v2=from_si(solution.rho_v2, "Pa"),
        energy_term=solution.energy_term,
        coupling_factor=solution.coupling_factor,
        mode_weight=solution.mode_weight,
        severity=solution.severity,
        modes=tuple(
            PulsationModeResult(
                mode_index=row.mode_index,
                frequency=from_si(row.frequency, "1/s"),
                envelope_low=from_si(row.envelope_low, "1/s"),
                envelope_high=from_si(row.envelope_high, "1/s"),
                locked_in=row.locked_in,
                resonance_velocity=from_si(row.resonance_velocity, "m/s"),
                resonance_velocity_low=from_si(row.resonance_velocity_low, "m/s"),
                resonance_velocity_high=from_si(row.resonance_velocity_high, "m/s"),
            )
            for row in solution.modes
        ),
        warnings=tuple(warnings),
    )
