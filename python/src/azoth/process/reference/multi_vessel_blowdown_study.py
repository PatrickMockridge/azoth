"""``process.multi_vessel_blowdown_study`` - the shared-header blowdown superposition.

As NeqSim's ``MultiVesselBlowdownStudy.run`` computes it. Spec:
``specs/models/process/multi_vessel_blowdown_study.toml``

The Python twin of ``crates/azoth-process/src/models/multi_vessel_blowdown_study.rs``: the
common grid, the per-source interpolation, the peak total and the header Mach check.
"""

from __future__ import annotations

import math
from collections.abc import Sequence
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.result import (
    MultiVesselBlowdownStudyResult,
    MultiVesselContributionRow,
    MultiVesselProfileRow,
)
from azoth.core.units import Q, from_si, input_records_to_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.multi_vessel_blowdown_study"

#: The class's gas constant, J/(mol K).
GAS_CONSTANT = 8.314
#: The largest grid this port builds, a guard the class does not carry.
MAX_GRID_ROWS = 1_000_000.0


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    profile: list[tuple[float, float]]
    contributions: list[tuple[str, float]]
    peak_total_mass_flow: float
    peak_time: float
    header_velocity: float | None
    header_mach: float | None
    max_allowable_mach: float
    header_mach_acceptable: bool


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _evaluate(
    samples: Sequence[dict[str, object]],
    header_diameter: Q | None = None,
    header_pressure: Q | None = None,
    header_temperature: Q | None = None,
    header_molar_mass: Q | None = None,
    header_heat_capacity_ratio: float | None = None,
    max_allowable_mach: float | None = None,
    grid_step: Q | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    warnings: list[Warning] = []
    step = 1.0 if grid_step is None else input_to_si(spec, "grid_step", grid_step)
    limit = 0.70 if max_allowable_mach is None else float(max_allowable_mach)
    rows = input_records_to_si(spec, "samples", samples)
    if not rows:
        raise InvalidInputError("samples", "the class requires at least one blowdown source")
    if step <= 0.0 or not math.isfinite(step):
        raise InvalidInputError("grid_step", "the class refuses a non-positive grid step")
    diameter = (
        None if header_diameter is None else input_to_si(spec, "header_diameter", header_diameter)
    )
    if diameter is not None and diameter <= 0.0:
        raise InvalidInputError(
            "header_diameter", "the class refuses a non-positive header diameter"
        )
    if not math.isfinite(limit):
        raise InvalidInputError("max_allowable_mach", "a non-finite Mach limit is not a comparison")

    order: list[str] = []
    times: dict[str, list[float]] = {}
    flows: dict[str, list[float]] = {}
    for row in rows:
        source = str(row["source"])
        time = float(row["time"])
        flow = float(row["mass_flow"])
        if not math.isfinite(time) or not math.isfinite(flow):
            raise InvalidInputError(
                "samples", "a non-finite sample is not a state this calculation uses"
            )
        if source not in times:
            order.append(source)
            times[source] = []
            flows[source] = []
        elif time <= times[source][-1]:
            raise InvalidInputError("samples", "a source's times must be strictly increasing")
        times[source].append(time)
        flows[source].append(flow)

    def interpolate(source: str, target: float) -> float:
        source_times = times[source]
        source_flows = flows[source]
        if target <= source_times[0]:
            return source_flows[0]
        if target >= source_times[-1]:
            return 0.0
        for index in range(1, len(source_times)):
            upper = source_times[index]
            if target <= upper:
                lower = source_times[index - 1]
                fraction = (target - lower) / (upper - lower) if upper > lower else 0.0
                return source_flows[index - 1] + fraction * (
                    source_flows[index] - source_flows[index - 1]
                )
        return 0.0

    end = max(source_times[-1] for source_times in times.values())
    declared = end / step
    if declared > MAX_GRID_ROWS:
        raise InvalidInputError(
            "grid_step",
            f"the grid would have {declared} rows, and this port builds at most {MAX_GRID_ROWS}",
        )

    profile: list[tuple[float, float]] = []
    per_source: dict[str, list[float]] = {source: [] for source in order}
    peak_total = 0.0
    peak_time = 0.0
    peak_index = 0
    time = 0.0
    index = 0
    while time <= end + 1.0e-9:
        total = 0.0
        for source in order:
            flow = interpolate(source, time)
            per_source[source].append(flow)
            total += flow
        if total > peak_total:
            peak_total = total
            peak_time = time
            peak_index = index
        profile.append((time, total))
        time += step
        index += 1

    velocity: float | None = None
    mach: float | None = None
    if diameter is not None:
        pressure = (
            1.5e5
            if header_pressure is None
            else input_to_si(spec, "header_pressure", header_pressure)
        )
        temperature = (
            288.15
            if header_temperature is None
            else input_to_si(spec, "header_temperature", header_temperature)
        )
        molar_mass = (
            0.020
            if header_molar_mass is None
            else input_to_si(spec, "header_molar_mass", header_molar_mass)
        )
        gamma = 1.30 if header_heat_capacity_ratio is None else float(header_heat_capacity_ratio)
        area = math.pi * 0.25 * diameter * diameter
        density = pressure * molar_mass / (GAS_CONSTANT * temperature)
        velocity = peak_total / (density * area)
        sonic = math.sqrt(gamma * GAS_CONSTANT * temperature / molar_mass)
        mach = velocity / sonic

    acceptable = mach is None or mach <= limit
    return (
        _Solution(
            profile=profile,
            contributions=[(source, per_source[source][peak_index]) for source in order],
            peak_total_mass_flow=peak_total,
            peak_time=peak_time,
            header_velocity=velocity,
            header_mach=mach,
            max_allowable_mach=limit,
            header_mach_acceptable=acceptable,
        ),
        warnings,
    )


def multi_vessel_blowdown_study(
    samples: Sequence[dict[str, object]],
    header_diameter: Q | None = None,
    header_pressure: Q | None = None,
    header_temperature: Q | None = None,
    header_molar_mass: Q | None = None,
    header_heat_capacity_ratio: float | None = None,
    max_allowable_mach: float | None = None,
    grid_step: Q | None = None,
) -> MultiVesselBlowdownStudyResult:
    """The simultaneous blowdown of sources into a shared header.

    Each source's profile is linearly interpolated onto a common grid, the totals are
    summed, and the peak total and its per-source contributions are published. The header
    Mach check runs only when the header diameter is given.

    Args:
        samples: the precomputed source trajectories in long form.
        header_diameter: the header's internal diameter; absent runs no Mach check.
        header_pressure: the header gas pressure; absent is 1.5e5 Pa.
        header_temperature: the header gas temperature; absent is 288.15 K.
        header_molar_mass: the header gas's molar mass; absent is 0.020 kg/mol.
        header_heat_capacity_ratio: the header gas's isentropic exponent; absent is 1.30.
        max_allowable_mach: the Mach limit; absent is 0.70.
        grid_step: the common-grid step; absent is 1 s.

    Returns:
        The combined profile, the contributions, the peak and the header check.

    Raises:
        InvalidInputError: for no samples, a non-increasing source time, a non-positive
            diameter or step, or a grid larger than this port builds.
    """
    solution, warnings = _evaluate(
        samples,
        header_diameter,
        header_pressure,
        header_temperature,
        header_molar_mass,
        header_heat_capacity_ratio,
        max_allowable_mach,
        grid_step,
    )
    return MultiVesselBlowdownStudyResult(
        profile=tuple(
            MultiVesselProfileRow(
                time=from_si(row[0], "s"), total_mass_flow=from_si(row[1], "kg/s")
            )
            for row in solution.profile
        ),
        contributions=tuple(
            MultiVesselContributionRow(source=source, peak_contribution=from_si(flow, "kg/s"))
            for source, flow in solution.contributions
        ),
        peak_total_mass_flow=from_si(solution.peak_total_mass_flow, "kg/s"),
        peak_time=from_si(solution.peak_time, "s"),
        header_velocity=(
            None if solution.header_velocity is None else from_si(solution.header_velocity, "m/s")
        ),
        header_mach=solution.header_mach,
        max_allowable_mach=solution.max_allowable_mach,
        header_mach_acceptable=solution.header_mach_acceptable,
        warnings=tuple(warnings),
    )
