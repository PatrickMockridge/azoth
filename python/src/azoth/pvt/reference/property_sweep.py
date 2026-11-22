"""``pvt.property_sweep`` - NeqSim's density and viscosity sweeps.

The Python twin of ``crates/azoth-pvt/src/property_sweep.rs``. The wax half of the class's
four sweeps is owed with the TBP wax model, which no characterization row can express.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import PropertyPoint, PropertySweepResult
from azoth.core.units import input_to_si, quantity

from . import _flash, _fluid, _transport

MODEL_ID = "pvt.property_sweep"


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


def property_sweep(
    components: Sequence[Mapping[str, Any]],
    mode: str,
    *,
    eos: str | None = None,
    mixing_rule: str | None = None,
    temperatures: Sequence[Any] | None = None,
    pressures: Sequence[Any] | None = None,
    volume_correction: bool | None = None,
) -> PropertySweepResult:
    """A density or viscosity sweep over a grid of states.

    Raises:
        InvalidInputError: for an unknown mode, ``eos`` or ``mixing_rule``, empty or unequal
            arrays, a non-positive temperature or pressure, or a wax mode (owed with the TBP
            wax model).
    """
    fluid = _fluid.Fluid.from_rows(
        components,
        eos=_cubic(eos),
        kij=_kij(mixing_rule),
        corrected=True if volume_correction is None else volume_correction,
    )
    if temperatures is None or len(temperatures) == 0:
        raise InvalidInputError("temperatures", "a property sweep needs at least one state")
    if pressures is None or len(pressures) != len(temperatures):
        raise InvalidInputError(
            "pressures",
            f"the schedule has {len(temperatures)} temperatures and "
            f"{0 if pressures is None else len(pressures)} pressures",
        )
    if mode not in ("density", "viscosity"):
        raise InvalidInputError(
            "mode",
            f"{mode} is not one of density and viscosity; the wax modes are owed with the "
            "class's TBP wax model",
        )
    temperatures_si = [float(input_to_si(_spec(), "temperatures", value)) for value in temperatures]
    pressures_si = [float(input_to_si(_spec(), "pressures", value)) for value in pressures]

    points: list[PropertyPoint] = []
    for temperature, pressure in zip(temperatures_si, pressures_si, strict=True):
        if not temperature > 0.0:
            raise InvalidInputError("temperatures", f"a sweep temperature is {temperature}")
        if not pressure > 0.0:
            raise InvalidInputError("pressures", f"a sweep pressure is {pressure}")
        flash = fluid.flash(temperature, pressure, _flash.TWO_PHASE)
        gas = flash.gas()
        oil = flash.oil()
        aqueous = flash.aqueous()

        def viscosity(
            phase: _flash.PhaseData | None, kind: str, t: float = temperature, p: float = pressure
        ) -> float | None:
            if phase is None:
                return None
            return _transport.phase_viscosity(fluid.mixture, t, p, phase.x, kind)

        points.append(
            PropertyPoint(
                temperature=quantity(temperature, "K"),
                pressure=quantity(pressure, "Pa"),
                gas_density=(
                    None if mode != "density" or gas is None else quantity(gas.density(), "kg/m**3")
                ),
                oil_density=(
                    None if mode != "density" or oil is None else quantity(oil.density(), "kg/m**3")
                ),
                aqueous_density=(
                    None
                    if mode != "density" or aqueous is None
                    else quantity(aqueous.density(), "kg/m**3")
                ),
                gas_viscosity=(
                    None if mode != "viscosity" else _maybe_quantity(viscosity(gas, _transport.GAS))
                ),
                oil_viscosity=(
                    None if mode != "viscosity" else _maybe_quantity(viscosity(oil, _transport.OIL))
                ),
                aqueous_viscosity=(
                    None
                    if mode != "viscosity"
                    else _maybe_quantity(viscosity(aqueous, _transport.AQUEOUS))
                ),
                gas_phase_available=gas is not None,
                oil_phase_available=oil is not None,
                aqueous_phase_available=aqueous is not None,
            )
        )
    return PropertySweepResult(points=tuple(points), warnings=())


def _maybe_quantity(value: float | None) -> Any:
    return None if value is None else quantity(value, "Pa*s")
