"""``pvt.constant_volume_depletion`` - NeqSim's CVD.

The Python twin of ``crates/azoth-pvt/src/constant_volume_depletion.rs``: the upward saturation
scan on the feed, the restore loop with its ``1e-8`` tolerance, 100 iterations and ``0.95``
per-iteration cap, and the cumulative mole accounting.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import ConstantVolumeDepletionResult, CvdPoint
from azoth.core.units import input_to_si, quantity
from azoth.core.warnings import Warning, WarningCode

from . import _flash, _fluid, _saturation, _volume

MODEL_ID = "pvt.constant_volume_depletion"

CELL_VOLUME_RELATIVE_TOLERANCE = 1.0e-8
MAX_CELL_VOLUME_ITERATIONS = 100
MAX_GAS_REMOVAL_FRACTION = 0.95
BISECTION_LIMIT = 500


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _scalar(name: str, value: Any) -> float:
    if value is None:
        raise InvalidInputError(name, "the calculation needs this field")
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


def _positive(name: str, value: float) -> float:
    if value > 0.0:
        return value
    raise InvalidInputError(name, f"a CVD needs a finite positive value; got {value}")


def _cvd_saturation(
    fluid: _fluid.Fluid, t: float, warnings: list[Warning]
) -> tuple[float, _flash.Flash]:
    """The class's upward scan on the feed, left at the last midpoint."""

    def flash(pressure_bar: float) -> _flash.Flash:
        return fluid.flash(t, _saturation.bar_to_pa(pressure_bar), _flash.TWO_PHASE)

    pressure_bar = 1.0
    while True:
        pressure_bar += 10.0
        current = flash(pressure_bar)
        if not (
            current.phase_count == 1 and pressure_bar < _saturation.SATURATION_PRESSURE_MAX_BAR
        ):
            break
    while True:
        pressure_bar += 10.0
        current = flash(pressure_bar)
        if not (current.phase_count > 1 and pressure_bar < _saturation.SATURATION_PRESSURE_MAX_BAR):
            break

    low = pressure_bar - 10.0
    high = pressure_bar
    iteration = 0
    while True:
        midpoint = (low + high) / 2.0
        state = flash(midpoint)
        if state.phase_count > 1:
            low = midpoint
        else:
            high = midpoint
        iteration += 1
        if (
            abs(high - low) <= _saturation.SATURATION_PRESSURE_TOLERANCE_BAR
            or iteration >= BISECTION_LIMIT
        ):
            break
    if abs(high - low) > _saturation.SATURATION_PRESSURE_TOLERANCE_BAR:
        warnings.append(
            Warning(
                WarningCode.SOLVER_NOT_CONVERGED,
                "the saturation bisection reached its cap; the last midpoint is the answer",
            )
        )
    return _saturation.bar_to_pa(midpoint), state


def _restore(
    fluid: _fluid.Fluid, saturation_volume: float, t: float, pressure: float
) -> tuple[_fluid.Fluid, float]:
    """Remove gas until the translated volume is the saturation volume."""
    removed_total = 0.0
    for _ in range(MAX_CELL_VOLUME_ITERATIONS):
        flash = fluid.flash(t, pressure, _flash.TWO_PHASE)
        current_volume = flash.volume()
        residual = (current_volume - saturation_volume) / saturation_volume
        if abs(residual) <= CELL_VOLUME_RELATIVE_TOLERANCE:
            return fluid, removed_total
        if residual < 0.0:
            raise InvalidInputError(
                "volume",
                f"the restore loop overshot the saturation volume: {current_volume} m3 against "
                f"{saturation_volume} m3",
            )
        gas = flash.gas()
        if gas is None:
            raise InvalidInputError(
                "flash",
                "the cell exceeds its saturation volume and has no gas phase to remove",
            )
        gas_moles = gas.moles(flash.total_moles)
        gas_volume = gas.volume(flash.total_moles)
        to_remove = min(
            (current_volume - saturation_volume) / gas_volume * gas_moles,
            MAX_GAS_REMOVAL_FRACTION * gas_moles,
        )
        moles = [
            max(amount - to_remove * gas.x[index], 0.0) for index, amount in enumerate(fluid.moles)
        ]
        fluid = fluid.with_moles(moles)
        removed_total += to_remove
    raise InvalidInputError(
        "volume", "the CVD gas removal did not restore the saturation volume in 100 iterations"
    )


def _k_values(flash: _flash.Flash, components: int) -> tuple[float, ...]:
    gas = flash.gas()
    oil = flash.oil()
    if gas is None or oil is None:
        return tuple(1.0 for _ in range(components))
    return tuple(
        0.0 if liquid <= 1.0e-20 else vapour / liquid
        for vapour, liquid in zip(gas.x, oil.x, strict=True)
    )


def constant_volume_depletion(
    components: Sequence[Mapping[str, Any]],
    *,
    eos: str | None = None,
    mixing_rule: str | None = None,
    temperature: Any = None,
    pressures: Sequence[Any] | None = None,
    volume_correction: bool | None = None,
) -> ConstantVolumeDepletionResult:
    """A CVD over the stated pressure schedule.

    Raises:
        InvalidInputError: for an unknown ``eos`` or ``mixing_rule``, no pressures, a
            non-positive temperature or pressure, or a restore loop that overshoots, finds no
            gas, or does not converge in 100 iterations.
    """
    feed = _fluid.Fluid.from_rows(
        components,
        eos=_cubic(eos),
        kij=_kij(mixing_rule),
        corrected=True if volume_correction is None else volume_correction,
    )
    t = _positive("temperature", _scalar("temperature", temperature))
    if pressures is None or len(pressures) == 0:
        raise InvalidInputError("pressures", "a CVD needs at least one pressure")
    schedule = [
        _positive(f"pressures[{index}]", float(input_to_si(_spec(), "pressures", value)))
        for index, value in enumerate(pressures)
    ]

    warnings: list[Warning] = []
    saturation_pressure, saturation = _cvd_saturation(feed, t, warnings)
    saturation_volume = saturation.volume()
    z_saturation = saturation.zvolcorr()
    initial_moles = feed.total_moles()

    fluid = feed
    cumulative_removed = 0.0
    sampled_hydrocarbon_split = False
    points: list[CvdPoint] = []
    k_values: list[tuple[float, ...]] = []

    for pressure in schedule:
        flash = fluid.flash(t, pressure, _flash.TWO_PHASE)
        sampled_hydrocarbon_split |= flash.gas() is not None and flash.oil() is not None
        if (
            flash.phase_count > 1 or (sampled_hydrocarbon_split and pressure < saturation_pressure)
        ) and flash.volume() > saturation_volume * (1.0 + CELL_VOLUME_RELATIVE_TOLERANCE):
            fluid, removed = _restore(fluid, saturation_volume, t, pressure)
            cumulative_removed += removed
            flash = fluid.flash(t, pressure, _flash.TWO_PHASE)

        total_volume = flash.volume()
        remaining = fluid.total_moles()
        oil = flash.oil()
        gas = flash.gas()
        points.append(
            CvdPoint(
                pressure=quantity(pressure, "Pa"),
                total_volume=quantity(total_volume, "m**3"),
                relative_volume=total_volume / saturation_volume,
                cumulative_mole_fraction_depleted=1.0 - remaining / initial_moles,
                material_balance_relative_error=(
                    None
                    if abs(initial_moles - remaining - cumulative_removed) / initial_moles
                    <= 1.0e-12
                    else abs(initial_moles - remaining - cumulative_removed) / initial_moles
                ),
                zmix=flash.zvolcorr(),
                z_gas=(None if gas is None else pressure * gas.molar_volume / (_volume.R * t)),
                liquid_volume=(
                    None if oil is None else quantity(oil.volume(flash.total_moles), "m**3")
                ),
                liquid_volume_relative_to_vsat=(
                    None if oil is None else oil.volume(flash.total_moles) / saturation_volume
                ),
            )
        )
        k_values.append(_k_values(flash, len(fluid.moles)))

    return ConstantVolumeDepletionResult(
        saturation_pressure=quantity(saturation_pressure, "Pa"),
        saturation_volume=quantity(saturation_volume, "m**3"),
        z_saturation=z_saturation,
        initial_moles=quantity(initial_moles, "mol"),
        cumulative_moles_removed=quantity(cumulative_removed, "mol"),
        k_values=tuple(k_values),
        points=tuple(points),
        warnings=tuple(warnings),
    )
