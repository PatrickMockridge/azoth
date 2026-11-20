"""The Rust backend, presented with the same interface as the reference.

Each function here takes and returns exactly what
:mod:`azoth.hydraulics.reference` does, so the dispatcher can swap one for the
other and a caller cannot tell which answered. Two jobs sit between the two
representations:

* **Units in.** Quantities are converted to SI magnitudes before crossing into
  Rust. That conversion lives here, once, rather than in the extension - one
  conversion site used by the reference and the Rust path alike cannot disagree
  with itself about what a number is in.
* **Results out.** The extension's transport objects become the same frozen
  dataclasses the reference returns, so ``result.dp`` is a pint quantity either
  way and ``isinstance`` checks behave identically.

If this file is missing or out of step with the extension, the dispatcher raises
rather than falling back - a silent fallback would mean the cross-language
guarantee was being verified by nothing.
"""

from __future__ import annotations

from collections.abc import Callable, Mapping
from typing import Any, cast

from azoth import _core, _models_gen

# **The generated half.** Every id whose signature is the public wrapper's and whose every
# boundary parameter is an input the spec declares, a field of the `params` record, or a boundary
# object's derived expansion is emitted by `tools/gen_python_bridge.py`; what is left in this file
# is the ids that generator refuses, each with its reason in `--survey`. `resolve` derives the
# id's function name and looks it up in this module's globals, which the star import fills.
from azoth._rust_bridge_gen import *  # noqa: F403

# The helpers the generated adapters and the hand-written ones both call. Imported rather than
# defined here because `azoth._rust_bridge_gen` imports them too, and a module cannot import the
# file that re-exports it. `from ... import _warnings` keeps the name reachable as
# `azoth._rust_bridge._warnings`, which is where `azoth.batch._core` reads it.
from azoth.core.bridge_support import _association_spec as _association_spec
from azoth.core.bridge_support import _si as _si
from azoth.core.bridge_support import _warnings as _warnings
from azoth.core.result import (
    FailureMode,
    HydrogenPhaseResult,
    PureSaturationResult,
    RupturePoint,
    TrappedLiquidFireRuptureResult,
)
from azoth.core.units import Q, from_si, input_to_si


def pure_saturation(Tc: Q, Pc: Q, omega: float, T: Q) -> PureSaturationResult:
    """The saturation pressure of a pure component, computed in Rust.

    A model rather than a calculation - its spec fixes a procedure and the Rust side
    composes the same kernels this bridge's scalar functions call, so the two
    implementations run the same search over the same arithmetic.
    """
    spec = _models_gen.model("eos.pure_saturation")
    result = _core.pure_saturation(
        # Not `input_to_si`: that reads the unit out of the spec's declaration, and the
        # spec no longer declares `Tc` or `Pc` - a component's constants come from the
        # databank by name, and a caller holding them holds quantities already. `T` is
        # still declared and still goes through the spec.
        Tc.to("K").magnitude,
        Pc.to("Pa").magnitude,
        # A plain float: `omega` is genuinely dimensionless, so it crosses as
        # the number the caller used - the same rule the other eos calcs follow.
        omega,
        input_to_si(spec, "T", T),
    )
    return PureSaturationResult(
        p_sat=from_si(result.p_sat.magnitude_si, result.p_sat.unit),
        ln_phi=result.ln_phi,
        iterations=result.iterations,
        residual=result.residual,
        warnings=_warnings(result.warnings),
    )


#: The same table for *models*, kept separate because the calc table is asserted to
#: be exactly the calc registry - ``test_registration_completeness`` compares it by
#: equality in both directions, so a model id in it would break that contract rather
#: than extend it. Models have their own id list for the same reason, and
#: ``test_model_contract.py`` holds this table to it.


def _boundary_result(raw: Any, result_type: Any, *, liquid_first: bool) -> Any:
    """Shared unpacking for the two phase-boundary models.

    The two differ only in whether the held phase is the liquid, so the `z_liquid`
    and `z_vapour` fields land in the opposite order. Doing that here rather than
    twice is the same argument the Rust side makes for one shared iteration.
    """
    held, incipient = (raw.z_liquid, raw.z_vapour) if liquid_first else (raw.z_vapour, raw.z_liquid)
    return result_type(
        pressure=from_si(raw.pressure.magnitude_si, raw.pressure.unit),
        incipient=tuple(raw.incipient),
        k=tuple(raw.k),
        z_liquid=held if liquid_first else incipient,
        z_vapour=incipient if liquid_first else held,
        min_t_over_tc=raw.min_t_over_tc,
        iterations=raw.iterations,
        residual=raw.residual,
        warnings=_warnings(raw.warnings),
    )


def _boundary_temperature_result(raw: Any, result_type: Any) -> Any:
    """Unpack a temperature boundary, whose scalar is a temperature rather than a
    pressure. The `z_liquid` and `z_vapour` fields already name the phases they say
    they do, so there is no swap here."""
    return result_type(
        temperature=from_si(raw.temperature.magnitude_si, raw.temperature.unit),
        incipient=tuple(raw.incipient),
        k=tuple(raw.k),
        z_liquid=raw.z_liquid,
        z_vapour=raw.z_vapour,
        min_t_over_tc=raw.min_t_over_tc,
        iterations=raw.iterations,
        residual=raw.residual,
        warnings=_warnings(raw.warnings),
    )


def hydrogen_phase(
    T: Q, P: Q, hydrogen_type: str = "normal", compressed_phase: str = "vapour"
) -> HydrogenPhaseResult:
    """The Leachman hydrogen phase state, computed in Rust."""
    spec = _models_gen.model("eos.hydrogen_phase")
    result = _core.hydrogen_phase(
        input_to_si(spec, "T", T), input_to_si(spec, "P", P), hydrogen_type, compressed_phase
    )
    return HydrogenPhaseResult(
        z_factor=result.z_factor,
        u=from_si(result.u.magnitude_si, result.u.unit),
        h=from_si(result.h.magnitude_si, result.h.unit),
        s=from_si(result.s.magnitude_si, result.s.unit),
        cv=from_si(result.cv.magnitude_si, result.cv.unit),
        cp=from_si(result.cp.magnitude_si, result.cp.unit),
        g=from_si(result.g.magnitude_si, result.g.unit),
        warnings=_warnings(result.warnings),
    )


def overlay_from(card: Any) -> Any:
    """A keycard as the extension reads it.

    The card's ``pint`` quantities become SI magnitudes here, the way every other
    dimensioned value crosses - the extension takes numbers, and one conversion site is
    what keeps the two languages working in the same units.

    **Nothing is resolved here.** This hands the card over as data; what a name resolves
    to is `azoth_eos::databank`'s answer, and `python/tests/test_data_agreement.py`
    compares it against `azoth.eos.components`' answer for the same card. Two
    implementations of one merge rule, held to each other the way the two kernels are.
    """
    components: list[_core.ComponentArguments] = [
        {
            "name": name,
            "tc": _si_of(stated.parameters.get("Tc")),
            "pc": _si_of(stated.parameters.get("Pc")),
            "omega": _plain(stated.parameters.get("omega")),
            "ion": stated.is_ion,
            "ionic_charge": _plain(stated.parameters.get("ionic_charge")),
            # Metres, which is the card's unit and the merge's input. The crossing to the
            # databank's ångström happens in one place, on the Rust side, so there is no
            # second conversion here to disagree with it.
            "deshmukh_mather_diameter": _si_of(stated.parameters.get("deshmukh_mather_diameter")),
            "dielectric": _dielectric(stated.parameters),
            # **Each of these crosses in the unit the Rust field is in**, which is the rule
            # the `deshmukh_mather_diameter` comment on the Rust side records being broken
            # once. Most are SI base units and `_si_of` is right for them;
            # `lennard_jones_diameter` is ångström on the Rust side, and crossing it as
            # metres would be ten orders of magnitude out with no symptom until a
            # correlation ran.
            "molar_mass": _si_of(stated.parameters.get("molar_mass")),
            "normal_liquid_density": _si_of(stated.parameters.get("normal_liquid_density")),
            "critical_volume": _si_of(stated.parameters.get("critical_volume")),
            "triple_point_temperature": _si_of(stated.parameters.get("triple_point_temperature")),
            "heat_of_fusion": _si_of(stated.parameters.get("heat_of_fusion")),
            "rackett_z": _plain(stated.parameters.get("rackett_z")),
            "parachor": _plain(stated.parameters.get("parachor")),
            "lennard_jones_diameter": _in(
                stated.parameters.get("lennard_jones_diameter"), "angstrom"
            ),
            "lennard_jones_energy": _si_of(stated.parameters.get("lennard_jones_energy")),
            "alpha_params": _alpha_params(stated.parameters),
            "liquid_conductivity": _coefficients(stated.parameters, "liquid_conductivity_", 3),
            "liqvisc": _coefficients(stated.parameters, "liqvisc_", 4),
            "liqvisc_model": stated.liqvisc_model,
            "wax_former": stated.wax_former,
            "cp_solid": _coefficients(stated.parameters, "cp_solid_", 4),
            "cp_liquid": _coefficients(stated.parameters, "cp_liquid_", 5),
            "solid_density_coefs": _coefficients(stated.parameters, "solid_density_coefs_", 4),
            "liquid_density_coefs": _coefficients(stated.parameters, "liquid_density_coefs_", 4),
        }
        for name, stated in sorted(card.components.items())
    ]
    kij = [(a, b, value) for (a, b), value in sorted(card.kij.items())]
    return _core.overlay(components, kij)


def _si_of(value: Q | None) -> float | None:
    """A dimensioned card value as an SI magnitude, or ``None`` if the card omits it."""
    return None if value is None else float(value.to_base_units().magnitude)


def _dielectric(parameters: Mapping[str, Q]) -> list[float] | None:
    """The five coefficients the card states, each in its own unit, or ``None``."""
    names = ("dielectric_1", "dielectric_2", "dielectric_3", "dielectric_4", "dielectric_5")
    if not all(name in parameters for name in names):
        return None
    return [
        float(parameters["dielectric_1"].to("dimensionless").magnitude),
        float(parameters["dielectric_2"].to("K").magnitude),
        float(parameters["dielectric_3"].to("1/K").magnitude),
        float(parameters["dielectric_4"].to("1/K**2").magnitude),
        float(parameters["dielectric_5"].to("1/K**3").magnitude),
    ]


def _plain(value: Q | None) -> float | None:
    """A dimensionless card value as a bare float, or ``None`` if the card omits it."""
    return None if value is None else float(value.to("dimensionless").magnitude)


def _in(value: Q | None, unit: str) -> float | None:
    """A card value in **the unit the Rust field carries**, or ``None``.

    Not the SI base unit, which is what :func:`_si_of` gives: a length the Rust side holds
    in ångström has to cross in ångström, and one crossing in metres would be ten orders of
    magnitude out with no symptom until a correlation ran.
    """
    return None if value is None else float(value.to(unit).magnitude)


def _present(value: Q) -> float:
    """A card value the caller has checked is present, as a bare float.

    Distinct from :func:`_plain`, which takes ``None`` because a caller may omit the key:
    these two comprehensions run only over keys the guard above has already found, so the
    optional shape is a lie about what reaches them and a type checker is right to say so.
    """
    return float(value.to("dimensionless").magnitude)


def _coefficients(parameters: Mapping[str, Q], prefix: str, width: int) -> list[float] | None:
    """One coefficient set the card states, in order, or ``None`` when it states none.

    The card's own keys are one per position and the Rust side takes the set as a vector.
    ``keycard._components`` has already refused a set that is not complete, so reaching this
    with a partial one is a programming error rather than a caller's.
    """
    names = [f"{prefix}{i}" for i in range(1, width + 1)]
    if not all(name in parameters for name in names):
        return None
    return [_present(parameters[name]) for name in names]


def _alpha_params(parameters: Mapping[str, Q]) -> list[float] | None:
    """The card's contiguous ``alpha_param_*`` prefix, in order, or ``None``.

    The prefix is contiguous - ``keycard._components`` refuses a gap - so the count of
    names is the length, and the correlation reads the vector positionally.
    """
    names = [name for name in sorted(parameters) if name.startswith("alpha_param_")]
    if not names:
        return None
    return [_present(parameters[name]) for name in names]


def resolve(calc_id: str) -> Callable[..., Any]:
    """The bridge function for a calc id, derived from the id rather than listed.

    A calculation's id is its address here exactly as it is on the reference side:
    `hydraulics.orifice_flow` names `orifice_flow` in this module. So there is no
    table, and adding a calculation adds no entry.

    The table this replaces was hand-maintained and asserted equal to the registry by
    a test - which is a list kept in step with a list, so the test could only ever
    tell you that you had forgotten something. Deriving it means there is nothing to
    forget, and a calc whose bridge function is missing fails the same completeness
    test one step earlier.

    Raises:
        KeyError: if the id is not in the registry at all. That is a programming
            error rather than a runtime condition - the reference and the extension
            are supposed to cover the same calcs, and a test asserts they do.
    """
    from azoth._models_gen import MODELS
    from azoth._registry_gen import CALCS

    if calc_id not in {entry["id"] for entry in [*CALCS, *MODELS]}:
        raise KeyError(
            f"no Rust implementation for {calc_id!r}: it is not in the registry. "
            f"The extension covers {len(CALCS)} calc(s) and {len(MODELS)} model(s)."
        ) from None

    resolved: Callable[..., Any] = globals()[calc_id.rpartition(".")[2]]
    return resolved


def _optional_si(quantity: object) -> Q | None:
    """A quantity the run may not have reached, as the dataclass spells it.

    The Rust transport writes `None` for a number a run did not reach - an ejector handed no
    flow, an exchanger's rating where one outlet was pinned - so the bridge's job here is to
    keep that absence rather than to invent a zero for it.
    """
    if quantity is None:
        return None
    magnitude_si = quantity.magnitude_si  # type: ignore[attr-defined]
    unit = quantity.unit  # type: ignore[attr-defined]
    return from_si(magnitude_si, unit)


def trapped_liquid_fire_rupture(
    components: list[str],
    z: list[float],
    inventory_pressure: float,
    inventory_temperature: float,
    inventory_liquid_mass: float,
    inventory_total_mass: float,
    inventory_total_volume: float,
    pipe_internal_diameter: float,
    wall_thickness: float,
    exposed_length: float,
    eos: str | None = None,
    multiphase_check: bool | None = None,
    segment_id: str | None = None,
    fire_type: str | None = None,
    environmental_factor: float | None = None,
    fire_heat_flux: float | None = None,
    fire_emissivity: float | None = None,
    fire_view_factor: float | None = None,
    flame_temperature: float | None = None,
    ambient_temperature: float | None = None,
    passive_protection_factor: float | None = None,
    fire_exposed_area: float | None = None,
    api5l_grade: str | None = None,
    ambient_tensile_strength: float | None = None,
    flange_class: str | None = None,
    relief_set_pressure: float | None = None,
    relief_limits_pressure: bool | None = None,
    max_time: float | None = None,
    time_step: float | None = None,
    wall_density: float | None = None,
    wall_heat_capacity: float | None = None,
    wall_conductivity: float | None = None,
    liquid_heat_capacity: float | None = None,
    liquid_expansion: float | None = None,
    liquid_bulk_modulus: float | None = None,
    inner_htc: float | None = None,
    outer_htc: float | None = None,
    tensile_strength_factor: float | None = None,
    wall_nodes: int | None = None,
    vapor_pocket_detection: bool | None = None,
) -> TrappedLiquidFireRuptureResult:
    """The trapped-liquid fire rupture study, computed in Rust.

    The hand-written adapter exists because `flange_class` is an optional count whose
    conversion the generator cannot place inside its expression.
    """
    spec = _models_gen.model("process.trapped_liquid_fire_rupture")

    def si(name: str, value: Any) -> float | None:
        return None if value is None else _si(spec, name, value)

    # The stub's declared order is the spec's; this binding's is the hand-written one the
    # `flange_class` conversion forced, which `test_gen_stub` declares as a known drift.
    call = cast(Any, _core.trapped_liquid_fire_rupture)
    result = call(
        list(components),
        list(z),
        eos,
        multiphase_check,
        si("inventory_pressure", inventory_pressure),
        si("inventory_temperature", inventory_temperature),
        si("inventory_liquid_mass", inventory_liquid_mass),
        si("inventory_total_mass", inventory_total_mass),
        si("inventory_total_volume", inventory_total_volume),
        segment_id,
        fire_type,
        si("environmental_factor", environmental_factor),
        si("fire_heat_flux", fire_heat_flux),
        si("fire_emissivity", fire_emissivity),
        si("fire_view_factor", fire_view_factor),
        si("flame_temperature", flame_temperature),
        si("ambient_temperature", ambient_temperature),
        si("passive_protection_factor", passive_protection_factor),
        si("fire_exposed_area", fire_exposed_area),
        api5l_grade,
        si("ambient_tensile_strength", ambient_tensile_strength),
        si("pipe_internal_diameter", pipe_internal_diameter),
        si("wall_thickness", wall_thickness),
        si("exposed_length", exposed_length),
        None if flange_class is None else int(flange_class),
        si("relief_set_pressure", relief_set_pressure),
        relief_limits_pressure,
        si("max_time", max_time),
        si("time_step", time_step),
        si("wall_density", wall_density),
        si("wall_heat_capacity", wall_heat_capacity),
        si("wall_conductivity", wall_conductivity),
        si("liquid_heat_capacity", liquid_heat_capacity),
        si("liquid_expansion", liquid_expansion),
        si("liquid_bulk_modulus", liquid_bulk_modulus),
        si("inner_htc", inner_htc),
        si("outer_htc", outer_htc),
        si("tensile_strength_factor", tensile_strength_factor),
        wall_nodes,
        vapor_pocket_detection,
    )
    trajectory = tuple(
        RupturePoint(
            time=from_si(row.time.magnitude_si, row.time.unit),
            pressure=from_si(row.pressure.magnitude_si, row.pressure.unit),
            liquid_temperature=from_si(
                row.liquid_temperature.magnitude_si, row.liquid_temperature.unit
            ),
            inner_wall_temperature=from_si(
                row.inner_wall_temperature.magnitude_si, row.inner_wall_temperature.unit
            ),
            outer_wall_temperature=from_si(
                row.outer_wall_temperature.magnitude_si, row.outer_wall_temperature.unit
            ),
            von_mises_stress=from_si(row.von_mises_stress.magnitude_si, row.von_mises_stress.unit),
            allowable_stress=from_si(row.allowable_stress.magnitude_si, row.allowable_stress.unit),
            flange_rating=from_si(row.flange_rating.magnitude_si, row.flange_rating.unit),
        )
        for row in result.trajectory
    )

    def optional(name: str) -> Any:
        value = getattr(result, name)
        return None if value is None else from_si(value.magnitude_si, value.unit)

    return TrappedLiquidFireRuptureResult(
        segment_id=result.segment_id,
        trajectory=trajectory,
        limiting_failure_mode=FailureMode(result.limiting_failure_mode),
        rupture_predicted=result.rupture_predicted,
        time_to_relief_set=optional("time_to_relief_set"),
        time_to_vapor_pocket=optional("time_to_vapor_pocket"),
        time_to_pipe_rupture=optional("time_to_pipe_rupture"),
        time_to_flange_failure=optional("time_to_flange_failure"),
        minimum_failure_time=optional("minimum_failure_time"),
        final_pressure=from_si(result.final_pressure.magnitude_si, result.final_pressure.unit),
        final_liquid_temperature=from_si(
            result.final_liquid_temperature.magnitude_si, result.final_liquid_temperature.unit
        ),
        final_outer_wall_temperature=from_si(
            result.final_outer_wall_temperature.magnitude_si,
            result.final_outer_wall_temperature.unit,
        ),
        standards_applied=tuple(result.standards_applied),
        recommendations=tuple(result.recommendations),
        warnings=_warnings(result.warnings),
    )
