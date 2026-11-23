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


def _cvd(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`pvt.constant_volume_depletion`'s captured quantities, keyed as the capture spells them."""
    from azoth.core.units import quantity

    from .reference.constant_volume_depletion import constant_volume_depletion

    rows = [{**row, "moles": quantity(row["moles"], "mol")} for row in inputs["components"]]
    result = constant_volume_depletion(
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
        "initial_moles": result.initial_moles.to("mol").magnitude,
        "cumulative_moles_removed": result.cumulative_moles_removed.to("mol").magnitude,
    }
    for index, point in enumerate(result.points):
        at = f"point_{index}_"
        rows_out[at + "pressure_pa"] = point.pressure.to("Pa").magnitude
        rows_out[at + "total_volume_m3"] = point.total_volume.to("m**3").magnitude
        rows_out[at + "relative_volume"] = point.relative_volume
        rows_out[at + "cumulative_mole_fraction_depleted"] = point.cumulative_mole_fraction_depleted
        if point.material_balance_relative_error is not None:
            rows_out[at + "material_balance_relative_error"] = point.material_balance_relative_error
        rows_out[at + "zmix"] = point.zmix
        if point.z_gas is not None:
            rows_out[at + "z_gas"] = point.z_gas
        if point.liquid_volume is not None:
            rows_out[at + "liquid_volume_m3"] = point.liquid_volume.to("m**3").magnitude
        if point.liquid_volume_relative_to_vsat is not None:
            rows_out[at + "liquid_volume_relative_to_vsat"] = point.liquid_volume_relative_to_vsat
    return rows_out


def _q(value: Any, unit: str) -> Any:
    from azoth.core.units import quantity

    return None if value is None else quantity(value, unit)


def _separator(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`pvt.separator_test`'s captured quantities, keyed as the capture spells them."""
    from azoth.core.units import quantity

    from .reference.separator_test import separator_test

    components = [{**row, "moles": quantity(row["moles"], "mol")} for row in inputs["components"]]
    stages = [
        {
            "pressure": quantity(row["pressure"], "Pa"),
            "temperature": quantity(row["temperature"], "K"),
        }
        for row in inputs["stages"]
    ]
    result = separator_test(
        components,
        inputs["mode"],
        stages=stages,
        eos=inputs.get("eos"),
        mixing_rule=inputs.get("mixing_rule"),
        reservoir_pressure=_q(inputs.get("reservoir_pressure"), "Pa"),
        reservoir_temperature=_q(inputs.get("reservoir_temperature"), "K"),
        optimize_first_stage=inputs.get("optimize_first_stage"),
        first_stage_pressure_min=_q(inputs.get("first_stage_pressure_min"), "Pa"),
        first_stage_pressure_max=_q(inputs.get("first_stage_pressure_max"), "Pa"),
        first_stage_pressure_steps=inputs.get("first_stage_pressure_steps"),
        first_stage_temperature_min=_q(inputs.get("first_stage_temperature_min"), "K"),
        first_stage_temperature_max=_q(inputs.get("first_stage_temperature_max"), "K"),
        first_stage_temperature_steps=inputs.get("first_stage_temperature_steps"),
    )
    rows_out: dict[str, float] = {}
    if result.total_gor is not None:
        rows_out["total_gor"] = result.total_gor
    if result.bo is not None:
        rows_out["bo"] = result.bo
    if result.rs is not None:
        rows_out["rs"] = result.rs
    if result.stock_tank_oil_density is not None:
        rows_out["stock_tank_oil_density"] = result.stock_tank_oil_density.to("kg/m**3").magnitude
    if result.stock_tank_api is not None:
        rows_out["stock_tank_api"] = result.stock_tank_api
    if result.optimal_stage_pressure is not None:
        rows_out["optimal_stage_pressure_pa"] = result.optimal_stage_pressure.to("Pa").magnitude
    if result.optimal_stage_temperature is not None:
        rows_out["optimal_stage_temperature_k"] = result.optimal_stage_temperature.to("K").magnitude
    for name in (
        "maximum_oil_recovery",
        "gor_at_optimum",
        "bo_at_optimum",
        "api_at_optimum",
    ):
        value = getattr(result, name)
        if value is not None:
            rows_out[name] = float(value)
    for stage in result.stages:
        at = f"stage_{stage.stage_number}_"
        rows_out[at + "pressure_pa"] = stage.pressure.to("Pa").magnitude
        rows_out[at + "temperature_k"] = stage.temperature.to("K").magnitude
        for name in ("stage_gor", "cumulative_gor", "oil_fvf", "gas_z_factor"):
            value = getattr(stage, name)
            if value is not None:
                rows_out[at + name] = float(value)
        for name, unit, suffix in (
            ("gas_volume", "m**3", "_m3"),
            ("gas_density", "kg/m**3", ""),
            ("oil_density", "kg/m**3", ""),
            ("gas_molar_mass", "kg/mol", ""),
            ("oil_molar_mass", "kg/mol", ""),
            ("oil_viscosity", "Pa*s", ""),
        ):
            value = getattr(stage, name)
            if value is not None:
                rows_out[at + name + suffix] = value.to(unit).magnitude
    return rows_out


def _swelling(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`pvt.swelling_test`'s captured quantities, keyed as the capture spells them."""
    from azoth.core.units import quantity

    from .reference.swelling_test import swelling_test

    components = [{**row, "moles": quantity(row["moles"], "mol")} for row in inputs["components"]]
    injection_gas = inputs.get("injection_gas")
    gas = (
        None
        if injection_gas is None
        else [{**row, "moles": quantity(row["moles"], "mol")} for row in injection_gas]
    )
    result = swelling_test(
        components,
        gas,
        eos=inputs.get("eos"),
        mixing_rule=inputs.get("mixing_rule"),
        temperature=None
        if inputs.get("temperature") is None
        else quantity(inputs["temperature"], "K"),
        cumulative_gas_injected=inputs["cumulative_gas_injected"],
        volume_correction=inputs.get("volume_correction"),
    )
    rows_out: dict[str, float] = {
        "original_oil_moles": result.original_oil_moles.to("mol").magnitude,
        "original_oil_volume_m3": result.original_oil_volume.to("m**3").magnitude,
    }
    for index, point in enumerate(result.points):
        at = f"point_{index}_"
        rows_out[at + "cumulative_gas_injected"] = point.cumulative_gas_injected
        rows_out[at + "pressure_pa"] = point.pressure.to("Pa").magnitude
        rows_out[at + "relative_oil_volume"] = point.relative_oil_volume
    return rows_out


def _mmp(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`pvt.mmp`'s captured quantities, keyed as the capture spells them."""
    from azoth.core.units import quantity

    from .reference.mmp import mmp

    def rows(source: list[Mapping[str, Any]]) -> list[dict[str, Any]]:
        return [{**row, "moles": quantity(row["moles"], "mol")} for row in source]

    result = mmp(
        rows(inputs["oil_components"]),
        rows(inputs["gas_components"]),
        eos=inputs.get("eos"),
        mixing_rule=inputs.get("mixing_rule"),
        temperature=quantity(inputs["temperature"], "K"),
        method=inputs.get("method"),
        recovery_threshold=inputs.get("recovery_threshold"),
        number_of_pressure_points=inputs.get("number_of_pressure_points"),
        min_pressure=_q(inputs.get("min_pressure"), "Pa"),
        max_pressure=_q(inputs.get("max_pressure"), "Pa"),
        slim_tube_nodes=inputs.get("slim_tube_nodes"),
        pv_injected=inputs.get("pv_injected"),
    )
    rows_out: dict[str, float] = {}
    if result.mmp is not None:
        rows_out["mmp_pa"] = result.mmp.to("Pa").magnitude
    for index, point in enumerate(result.points):
        at = f"point_{index}_"
        rows_out[at + "pressure_pa"] = point.pressure.to("Pa").magnitude
        rows_out[at + "recovery"] = point.recovery
    return rows_out


def _property(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`pvt.property_sweep`'s captured quantities, keyed as the capture spells them."""
    from azoth.core.units import quantity

    from .reference.property_sweep import property_sweep

    components = [{**row, "moles": quantity(row["moles"], "mol")} for row in inputs["components"]]
    result = property_sweep(
        components,
        inputs["mode"],
        eos=inputs.get("eos"),
        mixing_rule=inputs.get("mixing_rule"),
        temperatures=[quantity(value, "K") for value in inputs["temperatures"]],
        pressures=[quantity(value, "Pa") for value in inputs["pressures"]],
        volume_correction=inputs.get("volume_correction"),
    )
    rows_out: dict[str, float] = {}
    for index, point in enumerate(result.points):
        at = f"point_{index}_"
        rows_out[at + "temperature_k"] = point.temperature.to("K").magnitude
        rows_out[at + "pressure_pa"] = point.pressure.to("Pa").magnitude
        for name, unit, suffix in (
            ("gas_density", "kg/m**3", ""),
            ("oil_density", "kg/m**3", ""),
            ("aqueous_density", "kg/m**3", ""),
            ("gas_viscosity", "Pa*s", ""),
            ("oil_viscosity", "Pa*s", ""),
            ("aqueous_viscosity", "Pa*s", ""),
        ):
            value = getattr(point, name)
            if value is not None:
                rows_out[at + name + suffix] = value.to(unit).magnitude
        for name in (
            "gas_phase_available",
            "oil_phase_available",
            "aqueous_phase_available",
        ):
            rows_out[at + name] = 1.0 if getattr(point, name) else 0.0
    return rows_out


#: Models whose per-step arithmetic is dumped by a function here.
DUMPERS: dict[str, Dumper] = {
    "pvt.constant_mass_expansion": _cce,
    "pvt.differential_liberation": _dle,
    "pvt.constant_volume_depletion": _cvd,
    "pvt.separator_test": _separator,
    "pvt.swelling_test": _swelling,
    "pvt.mmp": _mmp,
    "pvt.property_sweep": _property,
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
    "pvt.regression": (
        "the answer is the fitted vector and its two ledgers; the optimizer's sample values "
        "are its steps, not a state a capture names"
    ),
    "pvt.tune_to_saturation": (
        "the answer is the tuned molar mass and the last saturation pressure; the iteration "
        "path is the loop's own and no intermediate state is published"
    ),
    "pvt.van_everdingen_hurst_aquifer": (
        "each calculation is one quadrature, conversion or recurrence answer; the nodes are "
        "the method, not a state a capture names"
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
