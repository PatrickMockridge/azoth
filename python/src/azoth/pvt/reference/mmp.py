"""``pvt.mmp`` - NeqSim's minimum miscibility pressure.

The Python twin of ``crates/azoth-pvt/src/mmp.rs``: the slim-tube mixing-cell chain, the key
tie line's bisection, and the mechanism heuristic. ``rising_bubble`` delegates to the slim
tube, as the class does.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import MmpPoint, MmpResult
from azoth.core.units import input_to_si, quantity
from azoth.core.warnings import Warning, WarningCode

from . import _flash, _fluid

MODEL_ID = "pvt.mmp"
DEFAULT_THRESHOLD = 0.90
DEFAULT_POINTS = 10
DEFAULT_MIN_BAR = 100.0
DEFAULT_MAX_BAR = 500.0
DEFAULT_NODES = 100
DEFAULT_PV = 1.2
KEY_TIE_LINE_PRESSURE_TOLERANCE = 0.5
KEY_TIE_LINE_LENGTH_TOLERANCE = 0.01


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _scalar(name: str, value: Any) -> float | None:
    if value is None:
        return None
    return float(input_to_si(_spec(), name, value))


def _cubic(eos: str | None) -> str:
    name = eos or "srk"
    if name not in ("srk", "pr"):
        raise InvalidInputError("eos", f"{name!r} is not one of srk and pr")
    return name


def _kij(mixing_rule: str | None) -> str:
    rule = mixing_rule or "classic"
    if rule not in ("classic", "zero"):
        raise InvalidInputError("mixing_rule", f"{rule!r} is not one of classic and zero")
    return "database" if rule == "classic" else "zero"


def _gas_composition(
    gas: _fluid.Fluid, gas_index: list[int], t: float, pressure_bar: float, oil_len: int
) -> list[float]:
    flash = gas.flash(t, pressure_bar * 1.0e5, _flash.TWO_PHASE)
    phase = flash.gas()
    composition = phase.x if phase is not None else gas.z()
    mapped = [0.0] * oil_len
    for value, index in zip(composition, gas_index, strict=True):
        mapped[index] = value
    return mapped


def _mix(oil: _fluid.Fluid, composition: list[float], gas_moles: float) -> _fluid.Fluid:
    moles = list(oil.moles)
    for index, value in enumerate(composition):
        moles[index] += gas_moles * value
    return oil.with_moles(moles)


def _recovery_at(
    oil: _fluid.Fluid,
    gas: _fluid.Fluid,
    gas_index: list[int],
    t: float,
    pressure_bar: float,
    nodes: int,
    pv: float,
    warnings: list[Warning],
) -> float:
    pressure = pressure_bar * 1.0e5
    oil_flash = oil.flash(t, pressure, _flash.TWO_PHASE)
    gas_flash = gas.flash(t, pressure, _flash.TWO_PHASE)
    injection = _gas_composition(gas, gas_index, t, pressure_bar, len(oil.moles))

    if oil_flash.phase_count == 1 and gas_flash.phase_count == 1:
        mixture = _mix(oil, injection, gas.total_moles())
        if mixture.flash(t, pressure, _flash.TWO_PHASE).phase_count == 1:
            return 1.0

    oil_in_place = oil.total_moles()
    cells = [list(oil.moles) for _ in range(nodes)]
    steps = int(pv * nodes)
    gas_per_step = gas.total_moles() / nodes
    recovered = 0.0
    for _ in range(steps):
        for index, value in enumerate(injection):
            cells[0][index] += gas_per_step * value
        for cell in range(nodes):
            fluid = oil.with_moles(cells[cell])
            try:
                flash = fluid.flash(t, pressure, _flash.TWO_PHASE)
            except Exception:
                continue
            if flash.phase_count > 1 and cell + 1 < nodes:
                gas_phase = flash.gas()
                if gas_phase is not None:
                    forward = 0.5 * gas_phase.fraction * flash.total_moles
                    for index, value in enumerate(gas_phase.x):
                        cells[cell + 1][index] += forward * value
                    oil_phase = flash.oil()
                    if oil_phase is not None:
                        retained = oil_phase.fraction * flash.total_moles
                        cells[cell] = [fraction * retained for fraction in oil_phase.x]
        last = oil.with_moles(cells[nodes - 1])
        try:
            flash = last.flash(t, pressure, _flash.TWO_PHASE)
        except Exception:
            continue
        oil_phase = flash.oil()
        if oil_phase is not None:
            recovered += oil_phase.fraction * flash.total_moles * 0.1
    recovery = min(1.0, max(0.0, recovered / oil_in_place))
    if recovery != recovery:
        warnings.append(
            Warning(
                WarningCode.OUT_OF_VALID_RANGE,
                "the slim tube answered a non-finite recovery; the point is reported as zero",
            )
        )
        return 0.0
    return recovery


def _slim_tube(
    oil: _fluid.Fluid,
    gas: _fluid.Fluid,
    gas_index: list[int],
    t: float,
    min_bar: float,
    max_bar: float,
    points: int,
    nodes: int,
    pv: float,
    warnings: list[Warning],
) -> tuple[list[float], list[float]]:
    dp = (max_bar - min_bar) / (points - 1)
    pressures = [min_bar + index * dp for index in range(points)]
    recoveries = [
        _recovery_at(oil, gas, gas_index, t, pressure, nodes, pv, warnings)
        for pressure in pressures
    ]
    return pressures, recoveries


def _interpolate(pressures: list[float], recoveries: list[float], threshold: float) -> float | None:
    for index in range(1, len(pressures)):
        if recoveries[index] >= threshold and recoveries[index - 1] < threshold:
            slope = (pressures[index] - pressures[index - 1]) / (
                recoveries[index] - recoveries[index - 1]
            )
            return pressures[index - 1] + slope * (threshold - recoveries[index - 1])
    if recoveries[0] >= threshold:
        return pressures[0]
    if recoveries[-1] < threshold:
        return None
    return pressures[-1]


def _mechanism(pressures: list[float], recoveries: list[float], mmp: float | None) -> str:
    if mmp is None:
        return "immiscible"
    at_mmp = 0.0
    for index in range(len(pressures) - 1):
        if pressures[index] <= mmp <= pressures[index + 1]:
            fraction = (mmp - pressures[index]) / (pressures[index + 1] - pressures[index])
            at_mmp = recoveries[index] + fraction * (recoveries[index + 1] - recoveries[index])
            break
    if at_mmp > 0.99:
        return "first_contact"
    low = recoveries[1]
    mid = recoveries[len(recoveries) // 2]
    if low < 0.3 and mid < 0.6:
        return "vaporizing"
    if low > 0.5:
        return "condensing"
    return "combined"


def _key_tie_line(
    oil: _fluid.Fluid,
    gas: _fluid.Fluid,
    gas_index: list[int],
    t: float,
    min_bar: float,
    max_bar: float,
) -> float:
    mapped = [0.0] * len(oil.moles)
    for value, index in zip(gas.z(), gas_index, strict=True):
        mapped[index] = value
    full = _mix(oil, mapped, gas.total_moles())
    low = min_bar
    high = max_bar
    while high - low > KEY_TIE_LINE_PRESSURE_TOLERANCE:
        mid = (low + high) / 2.0
        length = _tie_line_length(full, t, mid)
        if length < KEY_TIE_LINE_LENGTH_TOLERANCE:
            high = mid
        else:
            low = mid
    return (low + high) / 2.0


def _tie_line_length(fluid: _fluid.Fluid, t: float, pressure_bar: float) -> float:
    try:
        flash = fluid.flash(t, pressure_bar * 1.0e5, _flash.TWO_PHASE)
    except Exception:
        return 1.0
    if flash.phase_count == 1:
        return 0.0
    first, second = flash.phases[0], flash.phases[1]
    return float(sum((a - b) ** 2 for a, b in zip(first.x, second.x, strict=True)) ** 0.5)


def mmp(
    oil_components: Sequence[Mapping[str, Any]],
    gas_components: Sequence[Mapping[str, Any]],
    *,
    eos: str | None = None,
    mixing_rule: str | None = None,
    temperature: Any = None,
    method: str | None = None,
    recovery_threshold: float | None = None,
    number_of_pressure_points: int | None = None,
    min_pressure: Any = None,
    max_pressure: Any = None,
    slim_tube_nodes: int | None = None,
    pv_injected: float | None = None,
) -> MmpResult:
    """The MMP of an oil and an injection gas."""
    oil = _fluid.Fluid.from_rows(oil_components, eos=_cubic(eos), kij=_kij(mixing_rule))
    gas = _fluid.Fluid.from_rows(gas_components, eos=_cubic(eos), kij=_kij(mixing_rule))
    t = _scalar("temperature", temperature)
    if t is None or not t > 0.0:
        raise InvalidInputError("temperature", "an MMP needs a positive temperature")
    names = [str(row["name"]) for row in oil.rows]
    gas_names = [str(row["name"]) for row in gas.rows]
    gas_index: list[int] = []
    for name in gas_names:
        if name not in names:
            raise InvalidInputError(
                "gas_components",
                f"{name} is not a component of the oil, and the class injects only the "
                "components a cell already carries",
            )
        gas_index.append(names.index(name))

    threshold = DEFAULT_THRESHOLD if recovery_threshold is None else float(recovery_threshold)
    if not 0.0 < threshold <= 1.0:
        raise InvalidInputError(
            "recovery_threshold",
            f"the threshold is a recovery fraction in (0, 1]; got {threshold}",
        )
    points = DEFAULT_POINTS if number_of_pressure_points is None else int(number_of_pressure_points)
    if points < 3:
        raise InvalidInputError(
            "number_of_pressure_points",
            "the mechanism heuristic reads the second and the penultimate recovery, so there "
            "must be at least three pressures",
        )
    min_value = _scalar("min_pressure", min_pressure)
    max_value = _scalar("max_pressure", max_pressure)
    min_bar = DEFAULT_MIN_BAR if min_value is None else min_value / 1.0e5
    max_bar = DEFAULT_MAX_BAR if max_value is None else max_value / 1.0e5
    if not (min_bar > 0.0 and max_bar > min_bar):
        raise InvalidInputError(
            "min_pressure",
            f"the pressure range must be positive and increasing; got {min_bar} to {max_bar} bar",
        )
    nodes = DEFAULT_NODES if slim_tube_nodes is None else int(slim_tube_nodes)
    if nodes < 1:
        raise InvalidInputError("slim_tube_nodes", "the slim tube needs at least one cell")
    pv = DEFAULT_PV if pv_injected is None else float(pv_injected)
    if not pv > 0.0:
        raise InvalidInputError("pv_injected", f"the injected volume is {pv} pore volumes")

    chosen = method or "slim_tube"
    if chosen == "key_tie_line":
        value = _key_tie_line(oil, gas, gas_index, t, min_bar, max_bar)
        return MmpResult(
            mmp=quantity(value * 1.0e5, "Pa"),
            mechanism="combined",
            points=(),
            warnings=(),
        )
    if chosen not in ("slim_tube", "rising_bubble"):
        raise InvalidInputError(
            "method", f"{chosen!r} is not one of slim_tube, key_tie_line and rising_bubble"
        )
    warnings: list[Warning] = []
    pressures, recoveries = _slim_tube(
        oil, gas, gas_index, t, min_bar, max_bar, points, nodes, pv, warnings
    )
    interpolated = _interpolate(pressures, recoveries, threshold)
    return MmpResult(
        mmp=None if interpolated is None else quantity(interpolated * 1.0e5, "Pa"),
        mechanism=_mechanism(pressures, recoveries, interpolated),
        points=tuple(
            MmpPoint(pressure=quantity(pressure * 1.0e5, "Pa"), recovery=recovery)
            for pressure, recovery in zip(pressures, recoveries, strict=True)
        ),
        warnings=tuple(warnings),
    )
