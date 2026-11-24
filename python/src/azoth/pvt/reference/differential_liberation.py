"""``pvt.differential_liberation`` - NeqSim's DLE.

The Python twin of ``crates/azoth-pvt/src/differential_liberation.rs``: the gas-removal ledger,
the stock-tank oil volume from the final 288.15 K / 1.01325 bar flash, and Bo, Bg and Rs.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import DifferentialLiberationResult, DlePoint
from azoth.core.units import input_to_si, quantity
from azoth.core.warnings import Warning, WarningCode

from . import _flash, _fluid, _ledger, _saturation
from ._standard import NEQSIM_AIR_MOLAR_MASS_KG_PER_MOL, NEQSIM_GAS_CONSTANT

MODEL_ID = "pvt.differential_liberation"

T_SC = 288.15
P_SC = 101_325.0
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
    raise InvalidInputError(name, f"a DLE needs a finite positive value; got {value}")


def _dle_saturation(fluid: _fluid.Fluid, t: float, warnings: list[Warning]) -> tuple[float, float]:
    """The class's dead-first-loop scan, left at the last midpoint."""

    def flash(pressure_bar: float) -> _flash.Flash:
        return fluid.flash(t, _saturation.bar_to_pa(pressure_bar), _flash.TWO_PHASE)

    # Entered only from a two-phase state, so the class's unflashed first loop exits at 11 bar.
    pressure_bar = 11.0
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
    phase = max(state.phases, key=lambda phase: phase.fraction)
    return _saturation.bar_to_pa(midpoint), phase.volume(state.total_moles)


def differential_liberation(
    components: Sequence[Mapping[str, Any]],
    *,
    eos: str | None = None,
    mixing_rule: str | None = None,
    temperature: Any = None,
    pressures: Sequence[Any] | None = None,
    volume_correction: bool | None = None,
) -> DifferentialLiberationResult:
    """A DLE over the stated pressure schedule.

    Raises:
        InvalidInputError: for an unknown ``eos`` or ``mixing_rule``, no pressures, a
            non-positive temperature or pressure, or a two-phase point with no gas.
    """
    fluid = _fluid.Fluid.from_rows(
        components,
        eos=_cubic(eos),
        kij=_kij(mixing_rule),
        corrected=True if volume_correction is None else volume_correction,
    )
    t = _scalar("temperature", temperature)
    if pressures is None:
        raise InvalidInputError("pressures", "a DLE needs at least one pressure")
    schedule = [float(input_to_si(_spec(), "pressures", value)) for value in pressures]
    return _differential_liberation_of(fluid, t, schedule)


def _differential_liberation_of(
    fluid: _fluid.Fluid, t: float, schedule: Sequence[float]
) -> DifferentialLiberationResult:
    """The same DLE over a fluid the caller has already built, in SI."""
    t = _positive("temperature", t)
    if len(schedule) == 0:
        raise InvalidInputError("pressures", "a DLE needs at least one pressure")
    schedule = [_positive(f"pressures[{index}]", value) for index, value in enumerate(schedule)]

    warnings: list[Warning] = []
    saturation: tuple[float, float] | None = None
    total_gas_standard = 0.0
    rows: list[dict[str, Any]] = []

    for pressure in schedule:
        flash = fluid.flash(t, pressure, _flash.TWO_PHASE)
        volume = flash.volume()
        if not volume > 0.0:
            raise InvalidInputError("volume", f"the flash at {pressure} Pa has no volume")
        row: dict[str, Any] = {
            "pressure": pressure,
            "total_volume": volume,
            "liquid_volume": volume,
            "oil_density": flash.mass() / volume,
            "gas_volume": None,
            "gas_standard_volume": None,
            "z_gas": None,
            "relative_gas_gravity": None,
        }
        if flash.phase_count > 1:
            if saturation is None:
                saturation = _dle_saturation(fluid, t, warnings)
            gas = flash.gas()
            if gas is None:
                raise InvalidInputError(
                    "flash",
                    "a two-phase DLE point has no gas phase, and there is nothing to remove",
                )
            gas_standard = gas.moles(flash.total_moles) * NEQSIM_GAS_CONSTANT * T_SC / P_SC
            total_gas_standard += gas_standard
            row["z_gas"] = pressure * gas.molar_volume / (NEQSIM_GAS_CONSTANT * t)
            row["relative_gas_gravity"] = (
                gas.molar_mass / NEQSIM_AIR_MOLAR_MASS_KG_PER_MOL
            )
            oil = flash.oil()
            liquid = oil if oil is not None else flash.phases[0]
            row["oil_density"] = liquid.density()
            row["liquid_volume"] = liquid.volume(flash.total_moles)
            row["gas_volume"] = gas.volume(flash.total_moles)
            row["gas_standard_volume"] = gas_standard
            fluid = _ledger.remove_phase(fluid, gas)
        else:
            phase = flash.phases[0]
            row["oil_density"] = phase.density()
            row["liquid_volume"] = phase.volume(flash.total_moles)
        rows.append(row)

    standard = fluid.flash(T_SC, P_SC, _flash.TWO_PHASE)
    standard_oil = standard.oil()
    standard_gas = standard.gas()
    voil_std = None if standard_oil is None else standard_oil.volume(standard.total_moles)
    if standard_gas is not None:
        total_gas_standard += (
            standard_gas.moles(standard.total_moles) * NEQSIM_GAS_CONSTANT * T_SC / P_SC
        )
    else:
        warnings.append(
            Warning(
                WarningCode.OUT_OF_VALID_RANGE,
                "the final standard flash leaves no gas phase; only the removed gas is counted",
            )
        )
    if voil_std is None:
        warnings.append(
            Warning(
                WarningCode.OUT_OF_VALID_RANGE,
                "the final standard flash leaves no oil phase, so Bo, Rs and the stock-tank "
                "volume are absent",
            )
        )
    if saturation is None:
        warnings.append(
            Warning(
                WarningCode.OUT_OF_VALID_RANGE,
                "no pressure on the schedule is two-phase, so the saturation state is absent",
            )
        )

    running = 0.0
    points: list[DlePoint] = []
    for row in rows:
        relative_volume = None if saturation is None else row["total_volume"] / saturation[1]
        liquid_relative = (
            None
            if saturation is None or row["gas_standard_volume"] is None
            else row["liquid_volume"] / saturation[1]
        )
        bo = None if voil_std is None else row["liquid_volume"] / voil_std
        running += 0.0 if row["gas_standard_volume"] is None else row["gas_standard_volume"]
        rs = None if voil_std is None else (total_gas_standard - running) / voil_std
        z_gas = row["z_gas"]
        if z_gas is not None and z_gas > 1.0e-10:
            bg: float | None = row["gas_volume"] / row["gas_standard_volume"]
        else:
            bg = None
        points.append(
            DlePoint(
                pressure=quantity(row["pressure"], "Pa"),
                total_volume=quantity(row["total_volume"], "m**3"),
                relative_volume=relative_volume,
                liquid_volume=quantity(row["liquid_volume"], "m**3"),
                liquid_volume_relative_to_vsat=liquid_relative,
                oil_density=quantity(row["oil_density"], "kg/m**3"),
                gas_volume=(
                    None if row["gas_volume"] is None else quantity(row["gas_volume"], "m**3")
                ),
                gas_standard_volume=(
                    None
                    if row["gas_standard_volume"] is None
                    else quantity(row["gas_standard_volume"], "m**3")
                ),
                z_gas=z_gas,
                relative_gas_gravity=row["relative_gas_gravity"],
                bo=bo,
                bg=bg,
                rs=rs,
            )
        )

    return DifferentialLiberationResult(
        saturation_pressure=(None if saturation is None else quantity(saturation[0], "Pa")),
        saturation_volume=(None if saturation is None else quantity(saturation[1], "m**3")),
        voil_std=None if voil_std is None else quantity(voil_std, "m**3"),
        points=tuple(points),
        warnings=tuple(warnings),
    )
