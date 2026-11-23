"""``pvt.tune_to_saturation`` - the plus-molar-mass search.

The Python twin of ``crates/azoth-pvt/src/tune_to_saturation.rs``. The class sets the plus
fraction's molar mass, re-characterises the fluid, runs the saturation-pressure simulation
and steps the mass by ``dp/1000`` (bar), flipping the sign when the error grows. Its loop
reads the error at the top, so the exit criterion lags one run behind; this reproduces that.

**The plus row must be the fluid's only pseudo row.** The class lumps every TBP and plus row
together, while ``characterization.characterise_plus_fraction`` is the plus end alone.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import TuneToSaturationResult
from azoth.core.units import quantity

from . import _saturation
from ._fluid import Fluid

#: The class's early return, in Pa: `0.01` bar.
EARLY_RETURN_PA = 1000.0
#: The class's loop tolerance, in Pa: `1e-3` bar.
LOOP_TOLERANCE_PA = 100.0
#: The class's iteration cap.
DEFAULT_MAX_ITERATIONS = 50


def _first_carbon_number(name: str) -> int:
    digits = ""
    for char in name.lstrip("Cc"):
        if not char.isdigit():
            break
        digits += char
    if not digits:
        raise InvalidInputError(
            "plus_component",
            f"{name!r} has no leading carbon number, which the Pedersen split needs",
        )
    return int(digits)


def _rebuild(
    base: Sequence[Mapping[str, Any]],
    plus: Mapping[str, Any],
    molar_mass: float,
    plus_model: str,
    lumps: int | None,
    first_carbon: int,
    eos: str,
    kij: str,
    corrected: bool,
) -> Fluid:
    from azoth.characterization import characterise_plus_fraction, tbp_fraction_component
    from azoth.core.units import quantity

    total = sum(float(row["moles"]) for row in base) + float(plus["moles"])
    fraction = float(plus["moles"]) / total
    density = plus.get("density")
    if density is None:
        raise InvalidInputError("plus_component", "the plus row states no density")
    characterised = characterise_plus_fraction(
        quantity(molar_mass, "kg/mol"),
        quantity(float(density), "kg/m**3"),
        fraction,
        first_carbon,
        plus_model,
        lumps,
    )
    rows = [dict(row) for row in base]
    for index, (share, lump_mass, lump_density) in enumerate(
        zip(
            characterised.fraction_of_heavy_end,
            characterised.lump_molar_mass,
            characterised.lump_density,
            strict=True,
        )
    ):
        card = tbp_fraction_component(lump_mass, lump_density)
        rows.append(
            {
                "fluid": 0.0,
                "name": f"PC{index + 1}",
                "pseudo_component": True,
                "moles": float(plus["moles"]) * float(share),
                "molar_mass": lump_mass.to("kg/mol").magnitude,
                "density": lump_density.to("kg/m**3").magnitude,
                "normal_boiling_point": card.boiling_temperature.to("K").magnitude,
                "critical_temperature": card.critical_temperature.to("K").magnitude,
                "critical_pressure": card.critical_pressure.to("Pa").magnitude,
                "acentric_factor": card.acentric_factor,
                "critical_volume": card.critical_volume.to("m**3/mol").magnitude,
                "racket_z": card.racket_z,
                "racket_z_cpa": card.racket_z_cpa,
                "parachor": card.parachor,
                "critical_viscosity": card.critical_viscosity.to("Pa*s").magnitude,
                "triple_point_temperature": card.triple_point_temperature.to("K").magnitude,
                "heat_of_fusion": card.heat_of_fusion.to("J/mol").magnitude,
                "ideal_gas_enthalpy_of_formation": card.ideal_gas_enthalpy_of_formation.to(
                    "J/mol"
                ).magnitude,
                "cp_a": card.cp_a,
                "cp_b": card.cp_b,
                "cp_c": card.cp_c,
                "cp_d": card.cp_d,
                "attractive_m": card.attraction_exponent,
            }
        )
    return Fluid.from_rows(rows, eos=eos, kij=kij, corrected=corrected)


def _saturation_at(fluid: Fluid, temperature: float) -> float:
    point = _saturation.upper_saturation_pressure(fluid, temperature)
    if point.pressure is None:
        raise InvalidInputError(
            "saturation_pressure",
            "the saturation scan found no two-phase region at this temperature",
        )
    return point.pressure


def tune_to_saturation(
    components: Sequence[Mapping[str, Any]],
    plus_component: str,
    *,
    saturation_temperature: Any,
    saturation_pressure: Any,
    plus_model: str | None = None,
    number_of_lumps: int | None = None,
    eos: str | None = None,
    mixing_rule: str | None = None,
    volume_correction: bool | None = None,
    max_iterations: int | None = None,
) -> TuneToSaturationResult:
    """``TuneToSaturation`` as the id exposes it."""
    from azoth import _models_gen
    from azoth.core.units import input_records_to_si, input_to_si, to_count

    spec = _models_gen.model("pvt.tune_to_saturation")
    rows: list[dict[str, Any]] = input_records_to_si(spec, "components", list(components))
    temperature = float(input_to_si(spec, "saturation_temperature", saturation_temperature))
    target = float(input_to_si(spec, "saturation_pressure", saturation_pressure))
    if not (temperature > 0.0):
        raise InvalidInputError("saturation_temperature", "the temperature must be above zero")
    if not (target > 0.0):
        raise InvalidInputError("saturation_pressure", "the pressure must be above zero")
    index = next(
        (position for position, row in enumerate(rows) if row["name"] == plus_component), None
    )
    if index is None:
        raise InvalidInputError("plus_component", f"no row is named {plus_component!r}")
    plus = rows[index]
    if not plus["pseudo_component"]:
        raise InvalidInputError(
            "plus_component", "the plus fraction is a pseudo row, and this one is not"
        )
    other = next(
        (
            (position, row)
            for position, row in enumerate(rows)
            if position != index and row["pseudo_component"]
        ),
        None,
    )
    if other is not None:
        position, row = other
        raise InvalidInputError(
            "components",
            f"row {position} ({row['name']}) is another pseudo row; the class lumps every "
            "pseudo row together and this id re-characterises the plus end alone",
        )
    initial = plus.get("molar_mass")
    if initial is None:
        raise InvalidInputError("plus_component", "the plus row states no molar mass")
    first_carbon = _first_carbon_number(str(plus["name"]))
    base = [row for position, row in enumerate(rows) if position != index]
    model = plus_model or "pedersen"
    cubic = eos or "srk"
    rule = mixing_rule or "classic"
    if rule not in ("classic", "zero"):
        raise InvalidInputError("mixing_rule", f"{rule!r} is not one of classic and zero")
    kij = "database" if rule == "classic" else "zero"
    lumps = None if number_of_lumps is None else to_count(number_of_lumps, "number_of_lumps")
    cap = (
        DEFAULT_MAX_ITERATIONS
        if max_iterations is None
        else to_count(max_iterations, "max_iterations")
    )

    corrected = True if volume_correction is None else volume_correction
    molar_mass = float(initial)
    fluid = _rebuild(base, plus, molar_mass, model, lumps, first_carbon, cubic, kij, corrected)
    pressure = _saturation_at(fluid, temperature)
    error = pressure - target
    if abs(error) < EARLY_RETURN_PA:
        return TuneToSaturationResult(
            tuned_molar_mass=quantity(molar_mass, "kg/mol"),
            final_saturation_pressure=quantity(pressure, "Pa"),
            iterations=0.0,
            converged=True,
            warnings=(),
        )
    previous = 0.0
    sign = 1.0
    iterations = 0
    while abs(error) > LOOP_TOLERANCE_PA and iterations < cap:
        iterations += 1
        molar_mass -= sign * (error / 1.0e5) / 1000.0
        fluid = _rebuild(base, plus, molar_mass, model, lumps, first_carbon, cubic, kij, corrected)
        pressure = _saturation_at(fluid, temperature)
        if abs(previous) < abs(error):
            sign = -sign
        previous = error
        error = pressure - target
    return TuneToSaturationResult(
        tuned_molar_mass=quantity(molar_mass, "kg/mol"),
        final_saturation_pressure=quantity(pressure, "Pa"),
        iterations=float(iterations),
        converged=iterations < cap,
        warnings=(),
    )
