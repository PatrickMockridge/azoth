"""``pvt.separator_test`` - NeqSim's three separator experiments.

The Python twin of ``crates/azoth-pvt/src/separator_test.rs``: the continuous GOR, the staged
cascade and the multi-stage train with its first-stage optimisation.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any, cast

from azoth.core.errors import InvalidInputError
from azoth.core.result import SeparatorStageResult, SeparatorTestResult
from azoth.core.units import input_records_to_si, input_to_si, quantity

from . import _flash, _fluid, _transport

MODEL_ID = "pvt.separator_test"

T_SC = 288.15
P_SC = 101_325.0
API_WATER = 999.0
API_NUMERATOR = 141.5
API_OFFSET = 131.5
API_MIN = 500.0
API_MAX = 1100.0


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


def _positive(name: str, value: float) -> float:
    if value > 0.0:
        return value
    raise InvalidInputError(name, f"a separator test needs a finite positive value; got {value}")


def _component_moles(phase: _flash.PhaseData, total: float) -> list[float]:
    moles = phase.moles(total)
    return [fraction * moles for fraction in phase.x]


def _liquid(flash: _flash.Flash) -> _flash.PhaseData | None:
    oil = flash.oil()
    if oil is not None:
        return oil
    return next((phase for phase in flash.phases if phase.label != "gas"), None)


def _stage_oil_volume(flash: _flash.Flash) -> float:
    if flash.phase_count > 1 and len(flash.phases) > 1:
        return flash.phases[1].volume_uncorrected(flash.total_moles)
    return flash.volume_uncorrected()


def _standard_volume(fluid: _fluid.Fluid, moles: list[float]) -> float:
    standard = fluid.with_moles(moles).flash(T_SC, P_SC, _flash.TWO_PHASE)
    gas = standard.gas()
    return (
        standard.volume_uncorrected()
        if gas is None
        else gas.volume_uncorrected(standard.total_moles)
    )


def _stage_gas_volume(fluid: _fluid.Fluid, flash: _flash.Flash) -> float:
    if flash.phase_count > 1 and flash.phases and flash.phases[0].label == "gas":
        return _standard_volume(fluid, _component_moles(flash.phases[0], flash.total_moles))
    return 0.0


def _stock_tank_oil(fluid: _fluid.Fluid) -> float | None:
    standard = fluid.flash(T_SC, P_SC, _flash.TWO_PHASE)
    oil = standard.oil()
    return None if oil is None else oil.volume_uncorrected(standard.total_moles)


def _empty_stage(number: int, pressure: float, temperature: float) -> SeparatorStageResult:
    return SeparatorStageResult(
        stage_number=number,
        pressure=quantity(pressure, "Pa"),
        temperature=quantity(temperature, "K"),
        gas_volume=None,
        stage_gor=None,
        cumulative_gor=None,
        oil_fvf=None,
        gas_density=None,
        oil_density=None,
        gas_molar_mass=None,
        oil_molar_mass=None,
        gas_z_factor=None,
        oil_viscosity=None,
    )


def _row(
    stages: Sequence[SeparatorStageResult], warnings: tuple[Any, ...] = ()
) -> SeparatorTestResult:
    return SeparatorTestResult(
        total_gor=None,
        bo=None,
        rs=None,
        stock_tank_oil_density=None,
        stock_tank_api=None,
        optimal_stage_pressure=None,
        optimal_stage_temperature=None,
        maximum_oil_recovery=None,
        gor_at_optimum=None,
        bo_at_optimum=None,
        api_at_optimum=None,
        stages=tuple(stages),
        warnings=warnings,
    )


def _continuous(feed: _fluid.Fluid, stages: list[dict[str, float]]) -> SeparatorTestResult:
    gas_volumes: list[float] = []
    oil_volumes: list[float] = []
    rows: list[SeparatorStageResult] = []
    for index, stage in enumerate(stages):
        flash = feed.flash(stage["temperature"], stage["pressure"], _flash.TWO_PHASE)
        oil_volumes.append(_stage_oil_volume(flash))
        gas_volumes.append(_stage_gas_volume(feed, flash))
        rows.append(_empty_stage(index + 1, stage["pressure"], stage["temperature"]))
    stock = _stock_tank_oil(feed)
    if stock is not None:
        for index, row in enumerate(rows):
            if gas_volumes[index] > 1.0e-10:
                rows[index] = _with(
                    row, stage_gor=gas_volumes[index] / stock, oil_fvf=oil_volumes[index] / stock
                )
    return _row(rows)


def _with(row: SeparatorStageResult, **fields: Any) -> SeparatorStageResult:
    import dataclasses

    return dataclasses.replace(row, **fields)


def _staged(feed: _fluid.Fluid, stages: list[dict[str, float]]) -> SeparatorTestResult:
    fluid = feed
    gas_volumes: list[float] = []
    oil_volumes: list[float] = []
    rows: list[SeparatorStageResult] = []
    for index, stage in enumerate(stages):
        flash = fluid.flash(stage["temperature"], stage["pressure"], _flash.TWO_PHASE)
        oil_volumes.append(_stage_oil_volume(flash))
        gas_volumes.append(_stage_gas_volume(feed, flash))
        if flash.phase_count > 1 and len(flash.phases) > 1:
            fluid = fluid.with_moles(_component_moles(flash.phases[1], flash.total_moles))
        rows.append(_empty_stage(index + 1, stage["pressure"], stage["temperature"]))
    stock = _stock_tank_oil(fluid)
    if stock is not None:
        for index, row in enumerate(rows):
            if gas_volumes[index] > 1.0e-10:
                rows[index] = _with(
                    row,
                    stage_gor=gas_volumes[index] / stock,
                    oil_fvf=oil_volumes[index] / stock,
                )
    return _row(rows)


def _run_train(
    feed: _fluid.Fluid,
    stages: list[dict[str, float]],
    reservoir_temperature: float,
    reservoir_pressure: float,
) -> SeparatorTestResult:
    reservoir = feed.flash(reservoir_temperature, reservoir_pressure, _flash.MULTIPHASE)
    oil = reservoir.oil()
    reservoir_volume = (
        oil.volume_uncorrected(reservoir.total_moles)
        if oil is not None
        else reservoir.volume_uncorrected()
    )
    current = feed
    gas_volumes: list[float] = []
    gas_densities: list[float | None] = []
    oil_densities: list[float | None] = []
    gas_masses: list[float | None] = []
    oil_masses: list[float | None] = []
    gas_z: list[float | None] = []
    oil_viscosities: list[float | None] = []
    rows: list[SeparatorStageResult] = []
    stock_volume: float | None = None
    stock_density: float | None = None

    for index, stage in enumerate(stages):
        flash = current.flash(stage["temperature"], stage["pressure"], _flash.MULTIPHASE)
        gas = flash.gas()
        liquid = _liquid(flash)
        gas_volume = 0.0
        gas_density: float | None = None
        gas_mass: float | None = None
        z: float | None = None
        if gas is not None:
            gas_volume = _standard_volume(current, _component_moles(gas, flash.total_moles))
            gas_density = gas.density()
            gas_mass = gas.molar_mass
            z = gas.z_factor
        oil_density: float | None = None
        oil_mass: float | None = None
        oil_viscosity: float | None = None
        if liquid is not None:
            oil_density = liquid.density()
            oil_mass = liquid.molar_mass
            oil_viscosity = _transport.phase_viscosity(
                current.mixture,
                stage["temperature"],
                stage["pressure"],
                liquid.x,
                liquid.label,
            )
        if gas is not None and liquid is not None:
            current = current.with_moles(_component_moles(liquid, flash.total_moles))
        gas_volumes.append(gas_volume)
        gas_densities.append(gas_density)
        oil_densities.append(oil_density)
        gas_masses.append(gas_mass)
        oil_masses.append(oil_mass)
        gas_z.append(z)
        oil_viscosities.append(oil_viscosity)
        rows.append(_empty_stage(index + 1, stage["pressure"], stage["temperature"]))

        if index + 1 == len(stages):
            standard = current.flash(T_SC, P_SC, _flash.TWO_PHASE)
            oil = standard.oil() or _liquid(standard)
            if oil is not None:
                stock_volume = oil.volume_uncorrected(standard.total_moles)
                stock_density = oil.density()
            else:
                stock_volume = standard.volume_uncorrected()

    if stock_volume is None:
        raise InvalidInputError("stages", "the train's last stage did not reach a stock tank")
    if stock_density is None:
        raise InvalidInputError("stages", "the stock tank has no oil phase")
    api = None
    if API_MIN <= stock_density <= API_MAX:
        api = API_NUMERATOR / (stock_density / API_WATER) - API_OFFSET
    total_gas = sum(gas_volumes)
    total_gor = total_gas / stock_volume
    bo = reservoir_volume / stock_volume
    cumulative = 0.0
    updated: list[SeparatorStageResult] = []
    for index, row in enumerate(rows):
        cumulative += gas_volumes[index]
        gas_density = gas_densities[index]
        oil_density = oil_densities[index]
        gas_mass = gas_masses[index]
        oil_mass = oil_masses[index]
        oil_viscosity = oil_viscosities[index]
        updated.append(
            _with(
                row,
                gas_volume=quantity(gas_volumes[index], "m**3"),
                stage_gor=gas_volumes[index] / stock_volume,
                cumulative_gor=cumulative / stock_volume,
                gas_density=None if gas_density is None else quantity(gas_density, "kg/m**3"),
                oil_density=None if oil_density is None else quantity(oil_density, "kg/m**3"),
                gas_molar_mass=None if gas_mass is None else quantity(gas_mass, "kg/mol"),
                oil_molar_mass=None if oil_mass is None else quantity(oil_mass, "kg/mol"),
                gas_z_factor=gas_z[index],
                oil_viscosity=None if oil_viscosity is None else quantity(oil_viscosity, "Pa*s"),
            )
        )
    return SeparatorTestResult(
        total_gor=total_gor,
        bo=bo,
        rs=total_gor,
        stock_tank_oil_density=quantity(stock_density, "kg/m**3"),
        stock_tank_api=api,
        optimal_stage_pressure=None,
        optimal_stage_temperature=None,
        maximum_oil_recovery=None,
        gor_at_optimum=None,
        bo_at_optimum=None,
        api_at_optimum=None,
        stages=tuple(updated),
        warnings=(),
    )


def _optimisation(
    feed: _fluid.Fluid,
    stages: list[dict[str, float]],
    reservoir_temperature: float,
    reservoir_pressure: float,
    p_min: float | None,
    p_max: float | None,
    p_steps: int | None,
    t_min: float | None,
    t_max: float | None,
    t_steps: int | None,
) -> tuple[float, float, SeparatorTestResult]:
    if None in (p_min, p_max, p_steps, t_min, t_max, t_steps):
        raise InvalidInputError(
            "first_stage", "the optimisation needs its pressure and temperature grid"
        )
    p_min_value = cast(float, p_min)
    p_max_value = cast(float, p_max)
    p_points = cast(int, p_steps)
    t_min_value = cast(float, t_min)
    t_max_value = cast(float, t_max)
    t_points = cast(int, t_steps)
    if p_points < 2 or t_points < 2:
        raise InvalidInputError(
            "first_stage_pressure_steps",
            "the optimisation grid needs at least two points on each axis",
        )
    if p_min_value >= p_max_value or t_min_value >= t_max_value:
        raise InvalidInputError("first_stage", "the optimisation grid's bounds are not increasing")
    best: tuple[float, float, SeparatorTestResult] | None = None
    for p_index in range(p_points):
        pressure = p_min_value + (p_max_value - p_min_value) * p_index / (p_points - 1)
        for t_index in range(t_points):
            temperature = t_min_value + (t_max_value - t_min_value) * t_index / (t_points - 1)
            trial = [dict(stage) for stage in stages]
            trial[0]["pressure"] = pressure
            trial[0]["temperature"] = temperature
            outcome = _run_train(feed, trial, reservoir_temperature, reservoir_pressure)
            if outcome.bo is None or outcome.bo <= 0.0:
                continue
            if best is None or outcome.bo < (best[2].bo or 1.0e300):
                best = (pressure, temperature, outcome)
    if best is None:
        raise InvalidInputError(
            "first_stage", "no grid point answered a positive Bo, so there is no optimum"
        )
    return best


def separator_test(
    components: Sequence[Mapping[str, Any]],
    mode: str,
    *,
    stages: Sequence[Mapping[str, Any]],
    eos: str | None = None,
    mixing_rule: str | None = None,
    volume_correction: bool | None = None,
    reservoir_pressure: Any = None,
    reservoir_temperature: Any = None,
    optimize_first_stage: bool | None = None,
    first_stage_pressure_min: Any = None,
    first_stage_pressure_max: Any = None,
    first_stage_pressure_steps: Any = None,
    first_stage_temperature_min: Any = None,
    first_stage_temperature_max: Any = None,
    first_stage_temperature_steps: Any = None,
) -> SeparatorTestResult:
    """The separator experiment over the stated stages."""
    feed = _fluid.Fluid.from_rows(
        components,
        eos=_cubic(eos),
        kij=_kij(mixing_rule),
        corrected=True if volume_correction is None else volume_correction,
    )
    converted: list[dict[str, Any]] = input_records_to_si(_spec(), "stages", stages)
    if not converted:
        raise InvalidInputError("stages", "a separator test needs at least one stage")
    for index, stage in enumerate(converted):
        _positive(f"stages[{index}].pressure", float(stage["pressure"]))
        _positive(f"stages[{index}].temperature", float(stage["temperature"]))
    typed = [
        {"pressure": float(s["pressure"]), "temperature": float(s["temperature"])}
        for s in converted
    ]

    if mode == "continuous":
        return _continuous(feed, typed)
    if mode == "staged":
        return _staged(feed, typed)
    if mode == "train":
        p = _scalar("reservoir_pressure", reservoir_pressure)
        t = _scalar("reservoir_temperature", reservoir_temperature)
        if p is None or t is None:
            raise InvalidInputError(
                "reservoir_pressure", "the train needs a reservoir pressure and temperature"
            )
        _positive("reservoir_pressure", p)
        _positive("reservoir_temperature", t)
        result = _run_train(feed, typed, t, p)
        if optimize_first_stage:
            pressure, temperature, outcome = _optimisation(
                feed,
                typed,
                t,
                p,
                _scalar("first_stage_pressure_min", first_stage_pressure_min),
                _scalar("first_stage_pressure_max", first_stage_pressure_max),
                None if first_stage_pressure_steps is None else int(first_stage_pressure_steps),
                _scalar("first_stage_temperature_min", first_stage_temperature_min),
                _scalar("first_stage_temperature_max", first_stage_temperature_max),
                None
                if first_stage_temperature_steps is None
                else int(first_stage_temperature_steps),
            )
            import dataclasses

            result = dataclasses.replace(
                result,
                optimal_stage_pressure=quantity(pressure, "Pa"),
                optimal_stage_temperature=quantity(temperature, "K"),
                maximum_oil_recovery=1.0 / outcome.bo if outcome.bo else None,
                gor_at_optimum=outcome.total_gor,
                bo_at_optimum=outcome.bo,
                api_at_optimum=outcome.stock_tank_api,
            )
        return result
    raise InvalidInputError(
        "mode", f"{mode!r} is not a separator mode; use continuous, staged or train"
    )
