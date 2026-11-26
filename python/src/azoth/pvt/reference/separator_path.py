"""``pvt.separator_path`` - the staged separator cascade onto one declared final basis.

The Python twin of ``crates/azoth-pvt/src/separator_path.rs``: each stage flashes the carried
fluid, the gas leaves and is flashed at the path's final pressure and temperature, and the
oil and aqueous phases carry to the next stage.
"""

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from typing import Any, cast

from azoth.core.errors import InvalidInputError
from azoth.core.result import SeparatorPathResult
from azoth.core.units import input_to_si_shaped, quantity
from azoth.core.warnings import Warning, WarningCode

from . import _fluid

MODEL_ID = "pvt.separator_path"

_GAS = "gas"
_OIL = "oil"
_AQUEOUS = "aqueous"
_PHASES = (0, 1, 2)


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


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


def _phase_amounts(flash: Any, label: str) -> list[float] | None:
    phase = next((phase for phase in flash.phases if phase.label == label), None)
    if phase is None:
        return None
    return [phase.fraction * flash.total_moles * fraction for fraction in phase.x]


def _accumulate(flash: Any, volumes: list[float], totals: list[list[float]]) -> None:
    index = {_GAS: 0, _OIL: 1, _AQUEOUS: 2}
    for phase in flash.phases:
        at = index.get(phase.label)
        if at is None:
            raise InvalidInputError(
                "components", "an unsupported or nonphysical separator product phase"
            )
        volume = phase.volume(flash.total_moles)
        if not math.isfinite(volume) or volume <= 0.0:
            raise InvalidInputError(
                "components", "an unsupported or nonphysical separator product phase"
            )
        volumes[at] += volume
        for component, fraction in enumerate(phase.x):
            amount = phase.fraction * flash.total_moles * fraction
            if not math.isfinite(amount) or amount < 0.0:
                raise InvalidInputError(
                    "components", "an invalid separator product component amount"
                )
            totals[at][component] += amount


def separator_path(
    components: Sequence[Mapping[str, Any]],
    pressures: Sequence[Any],
    temperatures: Sequence[Any],
    *,
    eos: str | None = None,
    mixing_rule: str | None = None,
    volume_correction: bool | None = None,
    multiphase: bool | None = None,
) -> SeparatorPathResult:
    """A staged separator path onto its declared final volume basis.

    ``pressures`` and ``temperatures`` are one entry per stage, strictly decreasing in
    pressure; the last pair is the declared final volume basis, not another stage. Each
    stage flashes the carried fluid, the gas leaves and is flashed at the final basis, and
    the oil and aqueous phases carry forward. The products are corrected volumes.

    Returns:
        The final pressure and temperature, the gas/oil/aqueous volumes, the 3xN component
        amounts, and the gas/oil ratio - absent where there is no oil phase.

    Raises:
        InvalidInputError: for no stages, a non-positive or non-finite pressure or
            temperature, pressures that are not strictly decreasing, an unsupported product
            phase, a component balance that does not close, or a non-finite volume.

    See :func:`azoth.pvt.reference.separator_path.separator_path`.
    """
    declared = _spec()
    schedule = cast(list[float], input_to_si_shaped(declared, "pressures", pressures))
    schedule_t = cast(list[float], input_to_si_shaped(declared, "temperatures", temperatures))
    if not schedule or len(schedule) != len(schedule_t):
        raise InvalidInputError(
            "pressures", "equal nonempty pressure and temperature arrays are required"
        )
    for index, (pressure, temperature) in enumerate(zip(schedule, schedule_t, strict=True)):
        if (
            not math.isfinite(pressure)
            or pressure <= 0.0
            or not math.isfinite(temperature)
            or temperature <= 0.0
            or (index > 0 and pressure >= schedule[index - 1])
        ):
            raise InvalidInputError(
                "pressures",
                "finite positive temperatures and strictly decreasing positive pressures "
                "are required",
            )

    fluid = _fluid.Fluid.from_rows(
        components,
        eos=_cubic(eos),
        kij=_kij(mixing_rule),
        corrected=True if volume_correction is None else volume_correction,
    )
    initial = list(fluid.moles)
    count = len(initial)
    mode = "multiphase" if multiphase else "two_phase"
    last = len(schedule) - 1
    final_pressure = schedule[last]
    final_temperature = schedule_t[last]
    volumes = [0.0, 0.0, 0.0]
    totals = [[0.0] * count for _ in _PHASES]
    current = fluid
    for stage in range(last + 1):
        flash = current.flash(schedule_t[stage], schedule[stage], mode)
        if stage == last:
            _accumulate(flash, volumes, totals)
            break
        if flash.gas() is not None:
            amounts = _phase_amounts(flash, _GAS)
            assert amounts is not None
            condensed = current.with_moles(amounts).flash(final_temperature, final_pressure, mode)
            _accumulate(condensed, volumes, totals)
        remaining = [0.0] * count
        any_remaining = False
        for label in (_OIL, _AQUEOUS):
            amounts = _phase_amounts(flash, label)
            if amounts is None:
                continue
            for component, amount in enumerate(amounts):
                remaining[component] += amount
            any_remaining = True
        if not any_remaining:
            break
        current = current.with_moles(remaining)

    for component, amount in enumerate(initial):
        accounted = sum(totals[phase][component] for phase in _PHASES)
        if not math.isfinite(accounted) or abs(amount - accounted) > 1.0e-8 * max(1.0, amount):
            raise InvalidInputError("components", "the separator native component balance failed")
    if any(not math.isfinite(volume) for volume in volumes):
        raise InvalidInputError("components", "a non-finite accumulated separator product volume")

    warnings: list[Warning] = []
    if volumes[1] > 0.0:
        gas_oil_ratio: float | None = volumes[0] / volumes[1]
    else:
        gas_oil_ratio = None
        warnings.append(
            Warning(
                WarningCode.OUT_OF_VALID_RANGE,
                "the path has no stock-tank oil, so its gas/oil ratio is absent rather than NaN",
            )
        )

    return SeparatorPathResult(
        final_pressure=quantity(final_pressure, "Pa"),
        final_temperature=quantity(final_temperature, "K"),
        volumes=tuple(quantity(value, "m**3") for value in volumes),
        component_moles=tuple(tuple(quantity(value, "mol") for value in row) for row in totals),
        gas_oil_ratio=gas_oil_ratio,
        warnings=tuple(warnings),
    )
