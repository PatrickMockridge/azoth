"""``pvt.constant_mass_expansion`` - NeqSim's CCE.

The Python twin of ``crates/azoth-pvt/src/constant_mass_expansion.rs``: the upward saturation
scan left at its last midpoint, the untranslated relative volumes against the corrected phase
properties, the Y-factor and the class's Bg.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import CcePoint, ConstantMassExpansionResult
from azoth.core.units import input_to_si, quantity
from azoth.core.warnings import Warning, WarningCode
from azoth.eos.reference._mixture_state import reduced_parameters

from . import _flash, _fluid, _saturation, _transport, _volume
from ._standard import NEQSIM_GAS_CONSTANT

MODEL_ID = "pvt.constant_mass_expansion"

P_SC = 101_325.0
T_SC = 288.15
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
    raise InvalidInputError(name, f"a CCE needs a finite positive value; got {value}")


def _saturation_state(fluid: _fluid.Fluid, t: float) -> tuple[float, _flash.Flash, list[Warning]]:
    """The class's upward scan and bisection, left at the last midpoint."""

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
    warnings: list[Warning] = []
    if abs(high - low) > _saturation.SATURATION_PRESSURE_TOLERANCE_BAR:
        warnings.append(
            Warning(
                WarningCode.SOLVER_NOT_CONVERGED,
                "the saturation bisection reached its cap; the last midpoint is the answer",
            )
        )
    return _saturation.bar_to_pa(midpoint), state, warnings


def constant_mass_expansion(
    components: Sequence[Mapping[str, Any]],
    *,
    eos: str | None = None,
    mixing_rule: str | None = None,
    temperature: Any = None,
    pressures: Sequence[Any] | None = None,
    volume_correction: bool | None = None,
) -> ConstantMassExpansionResult:
    """The CCE of a fluid over a pressure schedule.

    Raises:
        InvalidInputError: for an unknown ``eos`` or ``mixing_rule``, no pressures, a
            non-positive temperature or pressure, or a phase whose properties cannot be read.
    """
    fluid = _fluid.Fluid.from_rows(
        components,
        eos=_cubic(eos),
        kij=_kij(mixing_rule),
        corrected=True if volume_correction is None else volume_correction,
    )
    t = _scalar("temperature", temperature)
    if pressures is None:
        raise InvalidInputError("pressures", "a CCE needs at least one pressure")
    schedule = [float(input_to_si(_spec(), "pressures", value)) for value in pressures]
    return _constant_mass_expansion_of(fluid, t, schedule)


def _constant_mass_expansion_of(
    fluid: _fluid.Fluid, t: float, schedule: Sequence[float]
) -> ConstantMassExpansionResult:
    """The same CCE over a fluid the caller has already built, in SI."""
    t = _positive("temperature", t)
    if len(schedule) == 0:
        raise InvalidInputError("pressures", "a CCE needs at least one pressure")
    schedule = [_positive(f"pressures[{index}]", value) for index, value in enumerate(schedule)]

    saturation_pressure, saturation, warnings = _saturation_state(fluid, t)
    saturation_volume = saturation.volume()
    z_saturation = (
        saturation_pressure * saturation_volume
        / (saturation.total_moles * NEQSIM_GAS_CONSTANT * t)
    )
    # The phase carrying the material: NeqSim's flash is single-phase at the class's
    # midpoint, while ours can leave a trace incipient gas there.
    first = max(saturation.phases, key=lambda phase: phase.fraction)
    reduced = reduced_parameters(fluid.mixture, t, saturation_pressure)
    saturation_compressibility = _volume.isothermal_compressibility(
        fluid.mixture, reduced, first.x, first.z_factor, t, saturation_pressure
    )

    points: list[CcePoint] = []
    for pressure in schedule:
        flash = fluid.flash(t, pressure, _flash.TWO_PHASE)
        phase = flash.phases[0]
        total_volume = flash.volume()
        relative_volume = total_volume / saturation_volume
        gas_volume = phase.volume(flash.total_moles)
        gas_standard_volume = phase.moles(flash.total_moles) * _volume.R * T_SC / P_SC
        oil = flash.oil()

        if flash.phase_count > 1:
            if oil is None:
                raise InvalidInputError(
                    "flash",
                    "a two-phase CCE point has no oil phase, so its liquid volume is absent",
                )
            liquid_relative_volume: float | None = (
                oil.volume(flash.total_moles) / saturation_volume
            )
            y_factor: float | None = (
                (saturation_pressure - pressure)
                / pressure
                / ((total_volume - saturation_volume) / saturation_volume)
            )
            viscosity: float | None = None
            compressibility: float | None = None
        else:
            reduced = reduced_parameters(fluid.mixture, t, pressure)
            compressibility = _volume.isothermal_compressibility(
                fluid.mixture, reduced, phase.x, phase.z_factor, t, pressure
            )
            viscosity = _transport.phase_viscosity(fluid.mixture, t, pressure, phase.x, phase.label)
            liquid_relative_volume = None
            y_factor = None

        if oil is not None:
            oil_density: float | None = oil.density()
            oil_viscosity: float | None = _transport.phase_viscosity(
                fluid.mixture, t, pressure, oil.x, _transport.OIL
            )
        else:
            oil_density = None
            oil_viscosity = None

        points.append(
            CcePoint(
                pressure=quantity(pressure, "Pa"),
                total_volume=quantity(total_volume, "m**3"),
                relative_volume=relative_volume,
                density=quantity(phase.density(), "kg/m**3"),
                gas_volume=quantity(gas_volume, "m**3"),
                gas_standard_volume=quantity(gas_standard_volume, "m**3"),
                bg=gas_volume / gas_standard_volume,
                z_gas=pressure * phase.molar_volume / (NEQSIM_GAS_CONSTANT * t),
                oil_density=None if oil_density is None else quantity(oil_density, "kg/m**3"),
                oil_viscosity=None if oil_viscosity is None else quantity(oil_viscosity, "Pa*s"),
                liquid_relative_volume=liquid_relative_volume,
                y_factor=y_factor,
                viscosity=None if viscosity is None else quantity(viscosity, "Pa*s"),
                isothermal_compressibility=compressibility,
            )
        )

    return ConstantMassExpansionResult(
        saturation_pressure=quantity(saturation_pressure, "Pa"),
        saturation_volume=quantity(saturation_volume, "m**3"),
        z_saturation=z_saturation,
        saturation_isothermal_compressibility=saturation_compressibility,
        points=tuple(points),
        warnings=tuple(warnings),
    )
