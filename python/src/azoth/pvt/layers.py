"""The intermediates the PVT reference kernels already compute, as named rows.

    from azoth.pvt import layers
    layers.rows("pvt.differential_liberation", {...})

A case compares the experiment's published arrays, and an array that is out says nothing
about which step moved first. The probe captures the same per-step quantities - saturation
pressure, phase compositions, Z roots, corrected volumes, densities, viscosities and the
cumulative moles a ledger removed - and those already exist inside the reference kernels.
This module exposes them by name so ``tools/neqsim_layer_diff.py`` can put the two side by
side.

The key names are the capture's, so a capture and a dump meet without a mapping table. A key
a dump does not carry is not compared; a dumper whose key set shares nothing with its
capture is an error rather than a pass.

No model gains an output here: this is an introspection surface, and a caller who wants an
answer calls the id.
"""

from __future__ import annotations

from collections.abc import Callable, Mapping
from typing import Any

#: A model's inputs, keyed as a case states them, to its intermediate rows.
Dumper = Callable[[Mapping[str, Any]], dict[str, float]]


def _cce(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`pvt.constant_mass_expansion`'s captured quantities, keyed as the capture spells them."""
    from azoth.core.units import quantity

    from .reference.constant_mass_expansion import constant_mass_expansion

    rows = [{**row, "moles": quantity(row["moles"], "mol")} for row in inputs["components"]]
    result = constant_mass_expansion(
        rows,
        eos=inputs.get("eos"),
        mixing_rule=inputs.get("mixing_rule"),
        temperature=quantity(inputs["temperature"], "K"),
        pressures=[quantity(value, "Pa") for value in inputs["pressures"]],
        volume_correction=inputs.get("volume_correction"),
    )
    rows_out: dict[str, float] = {
        "saturation_pressure_pa": result.saturation_pressure.to("Pa").magnitude,
        "saturation_volume_m3": result.saturation_volume.to("m**3").magnitude,
        "z_saturation": result.z_saturation,
        "saturation_isothermal_compressibility": result.saturation_isothermal_compressibility,
    }
    for index, point in enumerate(result.points):
        at = f"point_{index}_"
        rows_out[at + "pressure_pa"] = point.pressure.to("Pa").magnitude
        rows_out[at + "total_volume_m3"] = point.total_volume.to("m**3").magnitude
        rows_out[at + "relative_volume"] = point.relative_volume
        rows_out[at + "density"] = point.density.to("kg/m**3").magnitude
        rows_out[at + "gas_volume_m3"] = point.gas_volume.to("m**3").magnitude
        rows_out[at + "gas_standard_volume_m3"] = point.gas_standard_volume.to("m**3").magnitude
        rows_out[at + "bg"] = point.bg
        rows_out[at + "z_gas"] = point.z_gas
        for name in (
            "oil_density",
            "oil_viscosity",
            "liquid_relative_volume",
            "y_factor",
            "viscosity",
            "isothermal_compressibility",
        ):
            value = getattr(point, name)
            if value is not None:
                if name in ("oil_density",):
                    rows_out[at + name] = value.to("kg/m**3").magnitude
                elif name in ("oil_viscosity", "viscosity"):
                    rows_out[at + name] = value.to("Pa*s").magnitude
                else:
                    rows_out[at + name] = float(value)
    return rows_out


def _dle(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`pvt.differential_liberation`'s captured quantities, keyed as the capture spells them."""
    from azoth.core.units import quantity

    from .reference.differential_liberation import differential_liberation

    rows = [{**row, "moles": quantity(row["moles"], "mol")} for row in inputs["components"]]
    result = differential_liberation(
        rows,
        eos=inputs.get("eos"),
        mixing_rule=inputs.get("mixing_rule"),
        temperature=quantity(inputs["temperature"], "K"),
        pressures=[quantity(value, "Pa") for value in inputs["pressures"]],
        volume_correction=inputs.get("volume_correction"),
    )
    rows_out: dict[str, float] = {}
    if result.saturation_pressure is not None:
        rows_out["saturation_pressure_pa"] = result.saturation_pressure.to("Pa").magnitude
    if result.saturation_volume is not None:
        rows_out["saturation_volume_m3"] = result.saturation_volume.to("m**3").magnitude
    if result.voil_std is not None:
        rows_out["voil_std_m3"] = result.voil_std.to("m**3").magnitude
    for index, point in enumerate(result.points):
        at = f"point_{index}_"
        rows_out[at + "pressure_pa"] = point.pressure.to("Pa").magnitude
        rows_out[at + "total_volume_m3"] = point.total_volume.to("m**3").magnitude
        rows_out[at + "liquid_volume_m3"] = point.liquid_volume.to("m**3").magnitude
        rows_out[at + "oil_density"] = point.oil_density.to("kg/m**3").magnitude
        for name in (
            "relative_volume",
            "liquid_volume_relative_to_vsat",
            "z_gas",
            "relative_gas_gravity",
            "bo",
            "bg",
            "rs",
        ):
            value = getattr(point, name)
            if value is not None:
                rows_out[at + name] = float(value)
        for name, unit in (("gas_volume", "m**3"), ("gas_standard_volume", "m**3")):
            value = getattr(point, name)
            if value is not None:
                rows_out[at + name + "_m3"] = value.to(unit).magnitude
    return rows_out


#: Models whose per-step arithmetic is dumped by a function here.
DUMPERS: dict[str, Dumper] = {
    "pvt.constant_mass_expansion": _cce,
    "pvt.differential_liberation": _dle,
}

#: Record-list models with no interior to dump: every published row is the answer. The value
#: says why, and the layer-diff test counts each as evidence rather than silently skipping it.
NO_INTERIOR: dict[str, str] = {
    "pvt.black_oil_correlations": (
        "each row is one correlation's answer; no state lies between inputs and value"
    ),
    "pvt.gas_correlations": (
        "each row is one correlation's answer; no state lies between inputs and value"
    ),
    "pvt.water_correlations": (
        "each row is one correlation's answer; no state lies between inputs and value"
    ),
    "pvt.decline_curve": (
        "each call is one closed-form answer; the fits' grid search is the answer, not an interior"
    ),
    "pvt.gas_material_balance": (
        "each call is one regression's answer; the fit is the output, not an interior"
    ),
    "pvt.oil_material_balance": (
        "each call is one regression's answer; the fit is the output, not an interior"
    ),
    "pvt.saturation_test": (
        "the answer is the boundary itself; the scan's trial flashes are the search, not a "
        "state the capture records"
    ),
}

#: Capture blocks that are evidence rather than a case - a refusal, a fallback or a fluid the
#: id cannot express - and the count of rows in each. A block is not excused by widening a
#: tolerance; it is a measured row this id does not publish.
UNCASED_ROWS: dict[str, int] = {
    # The 350 K block: NeqSim's phase 0 at its saturation midpoint is a trace phase whose
    # volume is 7e-10 m3, so the class's relative volumes are 2e5. The id publishes the
    # material phase's volume and does not claim the class's arithmetic here.
    "pvt.differential_liberation": 8,
}


def rows(model_id: str, inputs: Mapping[str, Any]) -> dict[str, float]:
    """One model's named intermediates at one state.

    Raises:
        KeyError: for a model with no dumper, naming what is known.
    """
    dumper = DUMPERS.get(model_id)
    if dumper is None:
        raise KeyError(f"no PVT layer dumper for {model_id!r}; known: {sorted(DUMPERS)}")
    return dumper(inputs)
