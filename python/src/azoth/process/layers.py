"""The intermediates a unit operation's kernel already computes, as named rows.

    from azoth.process import layers
    layers.rows("process.pump", {"components": ["n-butane"], "inlet_n": 1.0, ...})

**A case compares a total, and a total that is 4% out says nothing about where.** A process
capture carries the machine's own intermediates beside the record - the shaft power, the
entropy the step produced - and those already exist inside the kernel on the way to the
answer. This exposes them by name so that the two sides can be put side by side and the
first layer that moved can be named.

# The sibling of `azoth.eos.layers`, and different in one way

The `eos` module reads a *reference* kernel's own solved state, because the reference is
where that kernel's arithmetic is written. A process model has two implementations too, and
its Python reference is the twin of the Rust kernel rather than the original - so this reads
the reference's intermediates through a factored entry point (`_states`, `_route`) rather
than re-deriving them. **A layer computed by a route the kernel does not take would be a
second implementation**, and a divergence in it would be a disagreement with ourselves.

# What counts as a layer

Only what the oracle also reports. `ProcessProbe` prints the record at every port and the
machine's own numbers beside it, and a layer whose key the capture has no counterpart for is
simply not compared - `tools/neqsim_layer_diff.py` says so rather than passing it.

That constraint is the point rather than a limitation: `Pump` exposes no getter for the
isentropic outlet, so the pump's interior is oracled through the two quantities it *does*
expose - the shaft power and the entropy the step produced - and the second of those is the
second-law statement of the same head.
"""

from __future__ import annotations

from collections.abc import Callable, Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Any

#: A model's declared inputs, keyed as a case states them, to its layers. The inputs are
#: bare numbers in the spec's units - the same ones `specs/cases/process/*.toml` carries -
#: so a caller states a state the way every other test in the suite does.
Dumper = Callable[[Mapping[str, Any]], dict[str, float]]

ROOT = Path(__file__).resolve().parents[4]
CAPTURES = ROOT / "validation" / "neqsim" / "captures"

#: The key a block's own label is kept under. `#` is not a character a probe key starts
#: with, so it cannot collide with a row.
LABEL_KEY = "#label"


def _pump(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.pump`'s layers, from the reference's own factored arithmetic."""
    from azoth.process.reference.pump import _states

    states = _states(
        [str(name) for name in inputs["components"]],
        float(inputs["inlet_t"]),
        float(inputs["inlet_p"]),
        [float(v) for v in inputs["inlet_z"]],
        float(inputs["outlet_pressure"]),
        float(inputs["isentropic_efficiency"]),
    )
    n = float(inputs["inlet_n"])
    dump = {
        "inlet_h": states.h_in,
        "inlet_s": states.s_in,
        "outlet_h": states.h_out,
        "outlet_T": states.outlet_t.to("K").magnitude,
        # The two quantities `Pump` exposes. `kJ/molK` and `kW` are the capture's units:
        # the probe reads `getEntropyProduction` and `getPower`, so the dump has to be in
        # the same ones or the comparison is a unit conversion nobody wrote down.
        "power_kW": n * states.dh_actual / 1000.0,
        "entropy_production_kJ_per_molK": (states.s_out - states.s_in) / 1000.0,
    }
    if inputs.get("mechanical_design"):
        from azoth.process.reference import capacity
        from azoth.process.reference.pump import _npsh_available, _npsh_required
        from azoth.process.reference.pump_mechanical import design as design_pump

        components = [str(name) for name in inputs["components"]]
        z = [float(value) for value in inputs["inlet_z"]]
        t = float(inputs["inlet_t"])
        p = float(inputs["inlet_p"])
        p_out = float(inputs["outlet_pressure"])
        speed = float(inputs.get("speed", 1000.0))
        density = capacity.system_mass_density(components, t, p, z)
        volume = capacity.system_volumetric_flow(components, t, p, z, n)
        npsh_required = _npsh_required(None, volume * 3600.0, speed)
        npsh_available = _npsh_available(components, t, p, z, density)
        maximum_suction = inputs.get("maximum_suction_pressure")
        shutoff = inputs.get("shutoff_head")
        design = design_pump(
            components,
            n,
            z,
            t,
            p,
            p_out,
            states.outlet_t.to("K").magnitude,
            n * states.dh_actual,
            float(inputs.get("isentropic_efficiency", 1.0)),
            npsh_required,
            npsh_available,
            speed,
            None,
            inputs.get("api610_pump_type"),
            inputs.get("seal_type"),
            None if maximum_suction is None else float(maximum_suction),
            None,
            None if shutoff is None else float(shutoff),
        )
        dump.update(
            {
                "rated_flow_m3_per_hr": design.rated_flow,
                "rated_head_m": design.rated_head,
                "hydraulic_power_kW": design.hydraulic_power / 1000.0,
                "rated_speed_rpm": design.rated_speed,
                "specific_speed": design.specific_speed,
                "suction_specific_speed": design.suction_specific_speed,
                "number_of_stages": float(design.number_of_stages),
                "design_pressure_bara": design.design_pressure / 1.0e5,
                "design_temperature_C": design.design_temperature - 273.15,
                "impeller_diameter_mm": design.impeller_diameter * 1000.0,
                "impeller_width_mm": design.impeller_width * 1000.0,
                "shaft_diameter_mm": design.shaft_diameter * 1000.0,
                "casing_wall_thickness_mm": design.casing_wall_thickness * 1000.0,
                "casing_mawp_bara": design.max_allowable_working_pressure / 1.0e5,
                "driver_power_kW": design.driver_power / 1000.0,
                "driver_margin": design.driver_margin,
                "suction_nozzle_size_in": design.suction_nozzle_size / 0.0254,
                "discharge_nozzle_size_in": design.discharge_nozzle_size / 0.0254,
                "minimum_flow_m3_per_hr": design.minimum_flow,
                "maximum_flow_m3_per_hr": design.maximum_flow,
                "casing_weight_kg": design.casing_weight,
                "motor_weight_kg": design.motor_weight,
                "baseplate_weight_kg": design.baseplate_weight,
                "inner_diameter_m": design.inner_diameter,
                "outer_diameter_m": design.outer_diameter,
                "wall_thickness_mm": design.wall_thickness * 1000.0,
                "tantan_length_m": design.tantan_length,
                "weight_piping_kg": design.weight_piping,
                "weight_electro_instrument_kg": design.weight_electro_instrument,
                "weight_structural_steel_kg": design.weight_structural_steel,
                "weight_total_kg": design.weight_total,
                "module_length_m": design.module_length,
                "module_width_m": design.module_width,
                "module_height_m": design.module_height,
                "design_npsh_margin_m": design.npsh_margin,
                "design_npsh_margin_ratio": design.npsh_margin_ratio,
                "effective_isentropic_efficiency": float(inputs.get("isentropic_efficiency", 1.0)),
                "inlet_m3_per_hr": volume * 3600.0,
                "inlet_density_kg_per_m3": density,
                "mass_flow_kg_per_hr": density * volume * 3600.0,
            }
        )
    return dump


def _compressor(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.compressor`'s layers, from the reference's own factored arithmetic.

    **The isentropic outlet is in the dump and not in the capture**, and that is worth
    stating: `Compressor` keeps its reversible enthalpy in a local, so the probe cannot print
    it - but the capture's efficiency-of-one row *is* that state, which is why that row is in
    the case set at all. A wrong isentropic step cannot reach both rows' enthalpies and both
    rows' entropy productions at once.
    """
    from azoth.process.reference.compressor import _states

    states = _states(
        [str(name) for name in inputs["components"]],
        float(inputs["inlet_t"]),
        float(inputs["inlet_p"]),
        [float(v) for v in inputs["inlet_z"]],
        float(inputs["outlet_pressure"]),
        float(inputs["isentropic_efficiency"]),
    )
    n = float(inputs["inlet_n"])
    return {
        "inlet_h": states.h_in,
        "inlet_s": states.s_in,
        "isentropic_h": states.h_isentropic,
        "outlet_h": states.h_out,
        "outlet_T": states.outlet_t.to("K").magnitude,
        # `getPower` is `dH = h_out - h_in` over the machine, which is what the class's own
        # `power_kW` line prints - in kW against the record's J/mol.
        "power_kW": n * states.dh_actual / 1000.0,
        "entropy_production_kJ_per_molK": (states.s_out - states.s_in) / 1000.0,
    }


def _expander(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.expander`'s layers, from the reference's own factored arithmetic.

    The same six as the compressor's plus the isentropic enthalpy, and the same reasoning:
    `Expander.run`'s second-law statement is the only other number it exposes, and the
    efficiency-of-one row is where the reversible state is the answer.
    """
    from azoth.process.reference.expander import _recovered_power_capacity, _states

    states = _states(
        [str(name) for name in inputs["components"]],
        float(inputs["inlet_t"]),
        float(inputs["inlet_p"]),
        [float(v) for v in inputs["inlet_z"]],
        float(inputs["outlet_pressure"]),
        float(inputs["isentropic_efficiency"]),
    )
    n = float(inputs["inlet_n"])
    dump = {
        "inlet_h": states.h_in,
        "inlet_s": states.s_in,
        "isentropic_h": states.h_isentropic,
        "outlet_h": states.h_out,
        "outlet_T": states.outlet_t.to("K").magnitude,
        # Negative, and it should be: `getPower` is the work the machine does, and an
        # expander produces it.
        "power_kW": n * states.dh_actual / 1000.0,
        "entropy_production_kJ_per_molK": (states.s_out - states.s_in) / 1000.0,
    }
    capacity = _recovered_power_capacity(
        n * states.dh_actual,
        None if "rated_recovered_power" not in inputs else float(inputs["rated_recovered_power"]),
    )
    if capacity.recovered_power_utilization is not None:
        dump["constraint_recoveredPower_utilization"] = capacity.recovered_power_utilization
    dump["max_utilization"] = capacity.max_utilization
    if inputs.get("mechanical_design"):
        from azoth.process.reference import capacity as _capacity
        from azoth.process.reference.expander_mechanical import design as design_expander

        components = [str(name) for name in inputs["components"]]
        t = float(inputs["inlet_t"])
        p = float(inputs["inlet_p"])
        z = [float(value) for value in inputs["inlet_z"]]
        design = design_expander(
            components,
            n,
            z,
            t,
            p,
            states.outlet_t.to("K").magnitude,
            n * states.dh_actual,
        )
        density = _capacity.system_mass_density(components, t, p, z, from_feed=True)
        volume = _capacity.system_volumetric_flow(components, t, p, z, n, from_feed=True)
        dump.update(
            {
                "wheel_diameter_mm": design.wheel_diameter * 1000.0,
                "tip_speed_m_per_s": design.tip_speed,
                "shaft_diameter_mm": design.shaft_diameter * 1000.0,
                "rated_speed_rpm": design.rated_speed,
                "max_continuous_speed_rpm": design.max_continuous_speed,
                "trip_speed_rpm": design.trip_speed,
                "first_critical_speed_rpm": design.first_critical_speed,
                "casing_pressure_bara": design.casing_pressure / 1.0e5,
                "casing_temperature_C": design.casing_temperature - 273.15,
                "casing_wall_thickness_mm": design.casing_wall_thickness * 1000.0,
                "bearing_span_mm": design.bearing_span * 1000.0,
                "wheel_weight_kg": design.wheel_weight,
                "shaft_weight_kg": design.shaft_weight,
                "casing_weight_kg": design.weight_vessel,
                "generator_weight_kg": design.generator_weight,
                "inner_diameter_mm": design.inner_diameter * 1000.0,
                "outer_diameter_mm": design.outer_diameter * 1000.0,
                "wall_thickness_mm": design.casing_wall_thickness * 1000.0,
                "tantan_length_mm": design.tantan_length * 1000.0,
                "weight_vessel_kg": design.weight_vessel,
                "weight_internals_kg": design.weight_internals,
                "weight_piping_kg": design.weight_piping,
                "weight_electro_instrument_kg": design.weight_electro_instrument,
                "weight_structural_steel_kg": design.weight_structural_steel,
                "weight_total_kg": design.weight_total,
                "module_length_m": design.module_length,
                "module_width_m": design.module_width,
                "module_height_m": design.module_height,
                "effective_isentropic_efficiency": float(inputs["isentropic_efficiency"]),
                "inlet_m3_per_hr": volume * 3600.0,
                "inlet_density_kg_per_m3": density,
                "mass_flow_kg_per_hr": density * volume * 3600.0,
                "inlet_number_of_phases": 1.0,
            }
        )
    return dump


def _pipe(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.pipe`'s layers, from the reference's own factored arithmetic.

    **Every number the capture prints about the line's interior is here**, including the
    pressure drop in the capture's own unit: the probe reports `bara` and the arithmetic is
    in pascals, so a dump that crossed the SI figure would be a unit conversion nobody wrote
    down. `passes` is the one layer with no counterpart - the capture cannot print how many
    passes the class's loop took - and it is dumped anyway so that the *path* is visible
    beside the answer it produced.
    """
    from azoth.process.reference.pipe import _route

    states = _route(
        [str(name) for name in inputs["components"]],
        float(inputs["inlet_n"]),
        [float(v) for v in inputs["inlet_z"]],
        float(inputs["inlet_p"]),
        float(inputs["inlet_t"]),
        float(inputs["length"]),
        float(inputs["diameter"]),
        float(inputs["roughness"]),
    )
    return {
        "velocity_m_per_s": states.velocity,
        "reynolds_number": states.reynolds,
        "friction_factor": states.friction_factor,
        "pressure_drop_bara": (float(inputs["inlet_p"]) - states.p_out) / 1.0e5,
        "passes": float(states.passes),
    }


def _manifold(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.manifold`'s layers, from the two routes it composes.

    **The mixture's own rows and each branch's flow**, which is what the manifold adds to
    its two children: `Manifold.run` is `localmixer.run()` then `localsplitter.run()` over
    the mixture, and the capture prints the mixture beside the branches. The branch
    *enthalpies* are deliberately not dumped: `process.splitter`'s own case declares them
    as a divergence, and the manifold's branches carry the same one - stated once, in the
    model that owns it, rather than twice.
    """
    from azoth.process.reference.manifold import DEFAULT_MINIMUM_FLOW_KG_PER_HOUR, _mass_flow
    from azoth.process.reference.mixer import _route as mix_route
    from azoth.process.reference.splitter import _route as split_route

    components = [str(name) for name in inputs["components"]]
    feed_n = [float(value) for value in inputs["feed_n"]]
    feed_z = [[float(x) for x in row] for row in inputs["feed_z"]]
    feed_p = [float(value) for value in inputs["feed_p"]]
    feed_t = [float(value) for value in inputs["feed_t"]]
    factors = [float(value) for value in inputs["split_factors"]]

    keep = [
        index
        for index in range(len(feed_n))
        if _mass_flow(components, feed_z[index], feed_n[index]) * 3600.0
        > DEFAULT_MINIMUM_FLOW_KG_PER_HOUR
    ]
    mixture = mix_route(
        components,
        [feed_n[index] for index in keep],
        [feed_z[index] for index in keep],
        [feed_p[index] for index in keep],
        [feed_t[index] for index in keep],
        None,
    )
    branches = split_route(
        components,
        mixture.product_t.to("K").magnitude,
        mixture.pressure,
        list(mixture.z),
        factors,
    )

    dump = {
        "product_n": mixture.n_total,
        "product_h": mixture.h_out,
        "product_T": mixture.product_t.to("K").magnitude,
    }
    for index, fraction in enumerate(branches.fractions):
        dump[f"products{index}_n"] = mixture.n_total * fraction
    return dump


def _heat_exchanger(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.heat_exchanger`'s layers, from the reference's own factored arithmetic.

    **The rating's interior is here and the capture does not have it.** `NTU` is a
    package-private field of `HeatExchanger` with no getter, so the probe cannot print it -
    but `getDuty` and `getThermalEffectiveness` are two numbers it can, and those two pin
    every number below: the kept side is the one whose seeded swing is larger, so
    `duty = effectiveness * C_max * span`, and the effectiveness relation then leaves
    `C_min` as its only unknown. A wrong `c_min` cannot reach the same duty and the same
    effectiveness at the same `UA`, which is why the un-oracled four are worth dumping
    beside the two that are compared.
    """
    from azoth.process.reference.heat_exchanger import _states

    def optional(key: str) -> float | None:
        value = inputs.get(key)
        return None if value is None else float(value)

    states = _states(
        [str(name) for name in inputs["hot_components"]],
        float(inputs["hot_in_n"]),
        [float(v) for v in inputs["hot_in_z"]],
        float(inputs["hot_in_p"]),
        float(inputs["hot_in_t"]),
        [str(name) for name in inputs["cold_components"]],
        float(inputs["cold_in_n"]),
        [float(v) for v in inputs["cold_in_z"]],
        float(inputs["cold_in_p"]),
        float(inputs["cold_in_t"]),
        optional("ua"),
        str(inputs.get("flow_arrangement", "counterflow")),
        optional("hot_outlet_temperature"),
        optional("cold_outlet_temperature"),
        bool(inputs.get("use_delta_t", False)),
        1.0 if inputs.get("delta_t") is None else float(inputs["delta_t"]),
    )
    # In the order the kernel computes them - the inlets, then the rating, then the
    # outlets - because the harness reports the *first* layer that moved, and the first one
    # to move should be the cause rather than the answer it produced.
    dump = {
        "hot_in_h": states.hot_in_h,
        "cold_in_h": states.cold_in_h,
    }
    if states.rating is not None:
        rating = states.rating
        dump |= {
            "hot_capacity": rating.hot_capacity,
            "cold_capacity": rating.cold_capacity,
            "c_min": rating.c_min,
            "c_max": rating.c_max,
            "capacity_ratio": rating.capacity_ratio,
            "ntu": rating.ntu,
            "effectiveness": rating.effectiveness,
            "duty_W": states.duty,
        }
    dump |= {
        "hot_out_h": states.hot_out_h,
        "cold_out_h": states.cold_out_h,
        "hot_out_T": states.hot_out_t.to("K").magnitude,
        "cold_out_T": states.cold_out_t.to("K").magnitude,
    }
    return dump


def _splitter(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.splitter`'s layers, from the reference's own factored arithmetic.

    **The branches' enthalpies are the layers where NeqSim and this library part**, and
    they are dumped so that the parting is measured rather than described. Two of the three
    are declared divergences; see `LAYER_CASES`.

    The capacity map's numeric fields are dumped under the probe's own keys. The boolean
    and the bottleneck's name are not: the layer harness compares floats, so those two are
    pinned by the case and by `test_splitter_capacity.py`. The mechanical design's nine
    numeric answers are dumped the same way when `mechanical_design` is on.

    Nothing here reads a state the kernel did not compute. A branch's pressure, temperature
    and composition are the feed's *by construction* - the kernel copies them - so dumping
    them would be dumping the input, and the entropy is a function of `(T, P, z)` the
    splitter never forms, which would make this a second implementation of
    `Stream::entropy` rather than a layer of the split.
    """
    from azoth.process.reference import capacity
    from azoth.process.reference.splitter import _capacity, _design, _route

    components = [str(name) for name in inputs["components"]]
    t = float(inputs["feed_t"])
    p = float(inputs["feed_p"])
    z = [float(v) for v in inputs["feed_z"]]
    n = float(inputs["feed_n"])
    factors = [float(f) for f in inputs["split_factors"]]

    states = _route(components, t, p, z, factors)
    dump = {"feed_h": states.h_in}
    for index, fraction in enumerate(states.fractions):
        dump[f"products{index}_n"] = n * fraction
        dump[f"products{index}_h"] = states.h_in
    # The balance a split owes, under the same name the probe prints it by. It is the
    # diverging one: `run`'s branches carry the defect, so their weighted enthalpy is not
    # the feed's.
    dump["molar_enthalpy_out_weighted"] = sum(
        fraction * states.h_in for fraction in states.fractions
    )

    dump["feed_volume_flow_m3_s"] = capacity.system_volumetric_flow(
        components, t, p, z, n, from_feed=True
    )
    stated_header = None if "header_diameter" not in inputs else float(inputs["header_diameter"])
    # The case states `corrosion_allowance` in the spec's millimetres while every other design
    # input is already SI, so it is divided by 1000 for `_design`'s metres.
    design = _design(
        components,
        t,
        p,
        z,
        n,
        factors,
        stated_header,
        bool(inputs.get("mechanical_design", False)),
        None if "design_velocity" not in inputs else float(inputs["design_velocity"]),
        None if "max_operation_pressure" not in inputs else float(inputs["max_operation_pressure"]),
        None if "tensile_strength" not in inputs else float(inputs["tensile_strength"]),
        None if "joint_efficiency" not in inputs else float(inputs["joint_efficiency"]),
        None
        if "corrosion_allowance" not in inputs
        else float(inputs["corrosion_allowance"]) / 1000.0,
    )
    effective_header = stated_header if design is None else design.header_diameter
    capacity_map = _capacity(
        components,
        t,
        p,
        z,
        n,
        None if "design_pressure_drop" not in inputs else float(inputs["design_pressure_drop"]),
        None if "max_design_velocity" not in inputs else float(inputs["max_design_velocity"]),
        effective_header,
        bool(inputs.get("capacity_analysis_enabled", False)),
    )
    for key, value in (
        ("pressure_drop_value_pa", capacity_map.pressure_drop_value),
        ("pressure_drop_design_pa", capacity_map.pressure_drop_design),
        ("pressure_drop_utilization", capacity_map.pressure_drop_utilization),
        ("velocity_value_m_s", capacity_map.velocity_value),
        ("velocity_design_m_s", capacity_map.velocity_design),
        ("velocity_utilization", capacity_map.velocity_utilization),
    ):
        if value is not None:
            dump[key] = value
    dump["max_utilization"] = capacity_map.max_utilization
    if design is not None:
        for key, value in (
            ("design_header_diameter_m", design.header_diameter),
            ("design_branch_diameter_m", design.branch_diameter),
            ("design_header_length_m", design.header_length),
            ("design_wall_thickness_m", design.wall_thickness),
            ("manifold_volume_m3", design.manifold_volume),
            ("design_total_pressure_drop_pa", design.total_pressure_drop),
            ("header_weight_kg", design.header_weight),
            ("branch_weight_kg", design.branch_weight),
            ("total_weight_kg", design.total_weight),
        ):
            dump[key] = value
    return dump


def _heater(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.heater`'s outlet record and its two outlet-temperature constraints.

    The outlet state is the flash's own answer, and the duty is `run`'s `newH - oldH`; the
    capacity map's utilisations are the one interior the record does not carry, so they are
    dumped under the probe's keys. The booleans and the bottleneck's name stay case pins.
    """
    from azoth.process.reference.heater import _capacity, _states

    components = [str(name) for name in inputs["components"]]
    n = float(inputs["inlet_n"])
    z = [float(v) for v in inputs["inlet_z"]]
    p = float(inputs["inlet_p"])
    t = float(inputs["inlet_t"])
    t_out = None if "outlet_temperature" not in inputs else float(inputs["outlet_temperature"])
    duty = None if "duty" not in inputs else float(inputs["duty"])
    drop = None if "pressure_drop" not in inputs else float(inputs["pressure_drop"])
    max_t = (
        None if "max_outlet_temperature" not in inputs else float(inputs["max_outlet_temperature"])
    )
    min_t = (
        None if "min_outlet_temperature" not in inputs else float(inputs["min_outlet_temperature"])
    )

    states = _states(components, n, z, p, t, t_out, duty, drop)
    outlet_k = states.outlet_t.to("K").magnitude
    dump = {
        "outlet_temperature_k": outlet_k,
        "outlet_h": states.h_out,
        "duty_W": states.duty,
    }
    capacity = _capacity(outlet_k, max_t, min_t)
    for key, value in (
        (
            "constraint_maxOutletTemperature_utilization",
            capacity.max_outlet_temperature_utilization,
        ),
        (
            "constraint_minOutletTemperature_utilization",
            capacity.min_outlet_temperature_utilization,
        ),
    ):
        if value is not None:
            dump[key] = value
    dump["max_utilization"] = capacity.max_utilization
    return dump


def _mixer(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.mixer`'s layers, from the reference's own factored arithmetic.

    **`mixed_enthalpy_W` is the layer worth having.** An outlet's molar enthalpy is the
    weighted total over the total flow, and a mean of the right magnitude hides a wrong
    weighting - the two inlets here are at `-20689.86` and `-22027.34` J/mol, so weighting
    them by moles or by mass gives answers `0.2%` apart while the totals are `65 kJ` apart
    and cannot be confused. `Mixer.calcMixStreamEnthalpy` is the class's own name for it.

    The joined composition and the outlet pressure are on the record and are not dumped: a
    layer the port already carries would be the validation case over again.
    """
    from azoth.process.reference.mixer import _route

    states = _route(
        [str(name) for name in inputs["components"]],
        [float(v) for v in inputs["feed_n"]],
        [[float(v) for v in row] for row in inputs["feed_z"]],
        [float(v) for v in inputs["feed_p"]],
        [float(v) for v in inputs["feed_t"]],
        None if inputs.get("outlet_pressure") is None else float(inputs["outlet_pressure"]),
    )
    return {"mixed_enthalpy_W": states.h_total}


#: One model's layers, keyed by the model id a case names.
def _shortcut_distillation_column(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.shortcut_distillation_column`'s layers, from the reference's own arithmetic.

    **The layers are the K-values and the relative volatilities, because everything
    downstream of `alpha` is rearrangement.** Fenske, Underwood, Molokanov and Kirkbride are
    closed form in `alpha_i = K_i / K_HK`, so a divergence in the stage count or the feed
    tray is located either in the flash that produced the K-values or in the correlations -
    and this dump separates the two. It is also the row where the port and NeqSim are closest,
    measured at `3e-12` on the captured propane/n-butane row, so a gap here is a plumbing bug
    rather than a physics disagreement.

    `alpha_lk_hk_from_flash` is dumped beside the per-component vectors rather than instead of
    them because the probe prints it twice - once from the flash it re-runs, once from the
    class's own `getRelativeVolatility` - and the two agreeing is what makes the capture's
    K-values the class's own rather than a second flash's.
    """
    from azoth.process.reference.shortcut_distillation_column import _states

    components = [str(name) for name in inputs["components"]]
    states = _states(
        components,
        float(inputs["feed_n"]),
        float(inputs["feed_t"]),
        float(inputs["feed_p"]),
        [float(v) for v in inputs["feed_z"]],
        str(inputs["light_key"]),
        str(inputs["heavy_key"]),
        float(inputs["light_key_recovery_distillate"]),
        float(inputs["heavy_key_recovery_bottoms"]),
        float(inputs["reflux_ratio_multiplier"]),
        None if inputs.get("condenser_pressure") is None else float(inputs["condenser_pressure"]),
        None if inputs.get("reboiler_pressure") is None else float(inputs["reboiler_pressure"]),
    )
    layers: dict[str, float] = {"alpha_lk_hk_from_flash": states.relative_volatility}
    for index, name in enumerate(components):
        layers[f"k_{name}"] = states.k_values[index]
        layers[f"alpha_{name}"] = states.alpha[index]
    return layers


def _component_splitter(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.component_splitter`'s layers, from the reference's own arithmetic.

    **The flows, temperatures and compositions are compared; the two enthalpies are declared
    divergences**, for the reason the sets beside `LAYER_CASES` give. The capture's own
    units, as bare magnitudes: the harness compares numbers.
    """
    from azoth.core.units import ureg
    from azoth.process.reference.component_splitter import component_splitter

    result = component_splitter(
        [str(name) for name in inputs["components"]],
        ureg.Quantity(float(inputs["feed_n"]), "mol/s"),
        [float(v) for v in inputs["feed_z"]],
        ureg.Quantity(float(inputs["feed_p"]), "Pa"),
        ureg.Quantity(float(inputs["feed_t"]), "K"),
        [float(v) for v in inputs["split_factors"]],
    )
    return {
        "overhead_n": result.overhead_n.to("mol/s").magnitude,
        "overhead_T": result.overhead_t.to("K").magnitude,
        "overhead_h": result.overhead_h.to("J/mol").magnitude,
        "bottoms_n": result.bottoms_n.to("mol/s").magnitude,
        "bottoms_T": result.bottoms_t.to("K").magnitude,
        "bottoms_h": result.bottoms_h.to("J/mol").magnitude,
    }


def _absorption_column(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.absorption_column`'s layers: the profile, under the capture's own key names.

    The same layer as the column's and for the same reason - a column's response is a profile
    rather than a scalar - and the capture prints the same four keys per tray, so the first tray
    that moved is the layer the diff names.
    """
    from azoth.process.reference.distillation_column import (
        _AbsorberMurphree,
        _feed,
        _Murphree,
        _states,
    )

    components = [str(name) for name in inputs["gas_components"]]
    # **The absorber's own correction**, which replaces the base's when either efficiency is
    # stated - the dispatch's rule, mirrored here so a case that states one reaches the same
    # arithmetic through either door.
    murphree = (
        None
        if "murphree_efficiency" not in inputs and "component_murphree_efficiency" not in inputs
        else _AbsorberMurphree(
            base=_Murphree(
                column_wide=_Murphree.clamp(
                    1.0
                    if "murphree_efficiency" not in inputs
                    else float(inputs["murphree_efficiency"])
                ),
                per_stage=None,
            ),
            per_component=(
                None
                if "component_murphree_efficiency" not in inputs
                else tuple(
                    _Murphree.clamp(float(v)) for v in inputs["component_murphree_efficiency"]
                )
            ),
        ).checked(len(components))
    )
    states = _states(
        components,
        float(inputs["gas_n"]),
        [float(v) for v in inputs["gas_z"]],
        float(inputs["gas_t"]),
        float(inputs["gas_p"]),
        int(inputs["number_of_stages"]),
        0,
        False,
        False,
        float(inputs["top_pressure"]),
        float(inputs["bottom_pressure"]),
        None,
        None,
        float(inputs["temperature_tolerance"]),
        int(inputs["max_iterations"]),
        None,
        None,
        str(inputs["solver_type"]) if "solver_type" in inputs else None,
        top_feed=_feed(
            [str(name) for name in inputs["solvent_components"]],
            float(inputs["solvent_n"]),
            [float(v) for v in inputs["solvent_z"]],
            float(inputs["solvent_t"]),
            float(inputs["solvent_p"]),
        ),
        tray_temperatures=(
            None
            if "tray_temperatures" not in inputs
            else tuple(float(v) for v in inputs["tray_temperatures"])
        ),
        absorber_murphree=murphree,
    )
    layers: dict[str, float] = {}
    for i in range(len(states.tray_temperature)):
        layers[f"tray{i}_temperature_K"] = states.tray_temperature[i]
        layers[f"tray{i}_pressure_bara"] = states.tray_pressure[i] / 1.0e5
        layers[f"tray{i}_gas_n"] = states.tray_gas_n[i]
        layers[f"tray{i}_liquid_n"] = states.tray_liquid_n[i]
    return layers


def _stripping_column(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.stripping_column`'s layers: the profile, under the capture's own key names.

    The same layer as the absorber's, because the class is the same machine: `StrippingColumn`
    renames the two inlets and the two products and adds no equations.
    """
    from azoth.process.reference.distillation_column import (
        _AbsorberMurphree,
        _feed,
        _Murphree,
        _states,
    )

    components = [str(name) for name in inputs["stripping_gas_components"]]
    # The inherited override, exactly as the absorber's own is built.
    murphree = (
        None
        if "murphree_efficiency" not in inputs and "component_murphree_efficiency" not in inputs
        else _AbsorberMurphree(
            base=_Murphree(
                column_wide=_Murphree.clamp(
                    1.0
                    if "murphree_efficiency" not in inputs
                    else float(inputs["murphree_efficiency"])
                ),
                per_stage=None,
            ),
            per_component=(
                None
                if "component_murphree_efficiency" not in inputs
                else tuple(
                    _Murphree.clamp(float(v)) for v in inputs["component_murphree_efficiency"]
                )
            ),
        ).checked(len(components))
    )
    states = _states(
        components,
        float(inputs["stripping_gas_n"]),
        [float(v) for v in inputs["stripping_gas_z"]],
        float(inputs["stripping_gas_t"]),
        float(inputs["stripping_gas_p"]),
        int(inputs["number_of_stages"]),
        0,
        False,
        False,
        float(inputs["top_pressure"]),
        float(inputs["bottom_pressure"]),
        None,
        None,
        float(inputs["temperature_tolerance"]),
        int(inputs["max_iterations"]),
        None,
        None,
        str(inputs["solver_type"]) if "solver_type" in inputs else None,
        top_feed=_feed(
            [str(name) for name in inputs["rich_liquid_components"]],
            float(inputs["rich_liquid_n"]),
            [float(v) for v in inputs["rich_liquid_z"]],
            float(inputs["rich_liquid_t"]),
            float(inputs["rich_liquid_p"]),
        ),
        tray_temperatures=(
            None
            if "tray_temperatures" not in inputs
            else tuple(float(v) for v in inputs["tray_temperatures"])
        ),
        absorber_murphree=murphree,
    )
    layers: dict[str, float] = {}
    for i in range(len(states.tray_temperature)):
        layers[f"tray{i}_temperature_K"] = states.tray_temperature[i]
        layers[f"tray{i}_pressure_bara"] = states.tray_pressure[i] / 1.0e5
        layers[f"tray{i}_gas_n"] = states.tray_gas_n[i]
        layers[f"tray{i}_liquid_n"] = states.tray_liquid_n[i]
    return layers


def _spec_of(inputs: Mapping[str, Any], which: str) -> Any:
    """One end's specification from a case's own inputs, or ``None`` where none is stated."""
    from azoth.process.reference.distillation_column import Specification

    kind = inputs.get(f"{which}_specification_type")
    if kind is None:
        return None
    return Specification(
        kind=str(kind),
        target=float(inputs[f"{which}_specification_target"]),
        component=(
            None
            if inputs.get(f"{which}_specification_component") is None
            else str(inputs[f"{which}_specification_component"])
        ),
    )


def _distillation_column(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.distillation_column`'s layers, from the reference's own factored arithmetic.

    **The profile is the layer, and it is the whole answer.** A column's response is not a
    scalar that a divergence can be attributed to; it is a temperature and two traffic rates on
    every tray, and the capture prints exactly those. So the dump is the profile under the
    capture's own key names - `tray3_temperature_K` and not `tray_temperature[3]` - and the
    first tray that moved is the layer the diff names.

    That is also why this model needs no divergence declaration: the two solvers reach the same
    fixed point one iteration apart, so the profile is compared directly rather than with a
    declared band.
    """
    from azoth.process.reference.distillation_column import PumparoundInlets as Inlets
    from azoth.process.reference.distillation_column import _Draws as Draws
    from azoth.process.reference.distillation_column import (
        _murphree,
        _pumparound_return_specification,
        _pumparound_returns_tear,
        _side_draw_flow_specification,
        _side_draw_flow_tear,
        _states,
    )
    from azoth.process.reference.distillation_column import _States as States

    # **Every declared input the case states is passed on, the section and the draws included.**
    # A dump that dropped one would compare a different column's profile against the capture and
    # call the difference a divergence: measured, the side-draw row's dump without its draw is
    # `4.9e-2` from the capture on tray 0.
    section = None
    if inputs.get("reactive"):
        start, end = inputs.get("reactive_start_tray"), inputs.get("reactive_end_tray")
        section = (-1, -1) if start is None or end is None else (int(start), int(end))
    draws = None
    if any(
        name in inputs
        for name in (
            "gas_side_draw_fractions",
            "liquid_side_draw_fractions",
            "pumparound_fractions",
        )
    ):

        def vector(name: str) -> tuple[float, ...] | None:
            # Absent and present-but-null are the same here: a case that states one draw kind
            # leaves the other two out entirely.
            values = inputs.get(name)
            return tuple(float(v) for v in values) if values is not None else None

        draws = (
            vector("gas_side_draw_fractions"),
            vector("liquid_side_draw_fractions"),
            vector("pumparound_fractions"),
        )

    components = [str(name) for name in inputs["components"]]
    stages = int(inputs["number_of_stages"])
    reboiler = bool(inputs["has_reboiler"])
    condenser = bool(inputs["has_condenser"])

    def solve_once(active_draws: Draws, active_returns: Inlets | None = None) -> States:
        """One inner solve at one set of draws - the same call, tear or no tear."""
        return _states(
            components,
            float(inputs["feed_n"]),
            [float(v) for v in inputs["feed_z"]],
            float(inputs["feed_t"]),
            float(inputs["feed_p"]),
            stages,
            int(inputs["feed_stage"]),
            reboiler,
            condenser,
            float(inputs["top_pressure"]),
            float(inputs["bottom_pressure"]),
            float(inputs["reboiler_temperature"]) if "reboiler_temperature" in inputs else None,
            float(inputs["condenser_temperature"]) if "condenser_temperature" in inputs else None,
            float(inputs["temperature_tolerance"]),
            int(inputs["max_iterations"]),
            _spec_of(inputs, "top"),
            _spec_of(inputs, "bottom"),
            str(inputs["solver_type"]) if "solver_type" in inputs else None,
            reactive=section,
            draws=active_draws,
            pumparound_inlets=active_returns,
            murphree_efficiency=_murphree(
                float(inputs["murphree_efficiency"]) if "murphree_efficiency" in inputs else None,
                inputs.get("tray_murphree_efficiency"),
                int(inputs["number_of_stages"]),
            ),
        )

    returns = _pumparound_return_specification(
        inputs.get("pumparound_return_tray"),
        inputs.get("pumparound_draw_tray"),
        inputs.get("pumparound_draw_fraction"),
        inputs.get("pumparound_temperature_drop"),
    )
    flows = _side_draw_flow_specification(
        inputs.get("side_draw_flow_tray"),
        inputs.get("side_draw_flow_phase"),
        None
        if inputs.get("side_draw_flow_target") is None
        else float(inputs["side_draw_flow_target"]),
        inputs.get("side_draw_flow_tolerance"),
        inputs.get("side_draw_flow_max_iterations"),
    )
    if returns is not None:
        states = _pumparound_returns_tear(
            solve_once,
            components,
            draws,
            returns,
            stages + int(reboiler) + int(condenser),
            float(inputs.get("pumparound_tolerance", 1.0e-4)),
            int(inputs.get("pumparound_max_iterations", 12)),
        )
    else:
        states = (
            solve_once(draws)
            if flows is None
            else _side_draw_flow_tear(
                solve_once, components, draws, flows, stages, reboiler, condenser
            )
        )
    layers: dict[str, float] = {}
    for i in range(len(states.tray_temperature)):
        layers[f"tray{i}_temperature_K"] = states.tray_temperature[i]
        layers[f"tray{i}_pressure_bara"] = states.tray_pressure[i] / 1.0e5
        layers[f"tray{i}_gas_n"] = states.tray_gas_n[i]
        layers[f"tray{i}_liquid_n"] = states.tray_liquid_n[i]
    # The three draws the capture states, summed: **its side-draw rows draw on one tray**, so the
    # sum is that tray's draw and the diff holds the mechanism's own number.
    layers["gas_side_draw_n"] = sum(states.gas_side_draw_n)
    layers["liquid_side_draw_n"] = sum(states.liquid_side_draw_n)
    layers["pumparound_n"] = sum(states.pumparound_n)
    return layers


def _packed_column(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.packed_column`'s layers: the base column's profile at the derived stage count.

    The same layer as the distillation column's, because the class's arithmetic is: the packing
    reaches the solve only through the stage count a constructor derives from the packed height,
    and the rest of the packing is a hydraulics report on the far side of the converged column.
    """
    from azoth.process.reference.distillation_column import _states
    from azoth.process.reference.packed_column import stage_count

    packed_height = float(inputs["packed_height"])
    states = _states(
        [str(name) for name in inputs["components"]],
        float(inputs["feed_n"]),
        [float(v) for v in inputs["feed_z"]],
        float(inputs["feed_t"]),
        float(inputs["feed_p"]),
        stage_count(packed_height),
        int(inputs["feed_stage"]),
        bool(inputs["has_reboiler"]),
        bool(inputs["has_condenser"]),
        float(inputs["top_pressure"]),
        float(inputs["bottom_pressure"]),
        float(inputs["reboiler_temperature"]) if "reboiler_temperature" in inputs else None,
        float(inputs["condenser_temperature"]) if "condenser_temperature" in inputs else None,
        float(inputs["temperature_tolerance"]),
        int(inputs["max_iterations"]),
        _spec_of(inputs, "top"),
        _spec_of(inputs, "bottom"),
        str(inputs["solver_type"]) if "solver_type" in inputs else None,
    )
    layers: dict[str, float] = {}
    for i in range(len(states.tray_temperature)):
        layers[f"tray{i}_temperature_K"] = states.tray_temperature[i]
        layers[f"tray{i}_pressure_bara"] = states.tray_pressure[i] / 1.0e5
        layers[f"tray{i}_gas_n"] = states.tray_gas_n[i]
        layers[f"tray{i}_liquid_n"] = states.tray_liquid_n[i]
    return layers


def _rate_based_packed_column(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.rate_based_packed_column`'s layers: the segment profile itself.

    **A rate-based column's four ports being out says nothing about which segment moved**, and
    the segment profile is the layer - the same reason `_distillation_column` dumps the tray
    profile. The keys are the capture's own, so a divergence names the segment it is in.

    Three mixtures are formed per segment - the two systems and an interface mix - so this is
    the closest layer the model has to its own arithmetic.

    **The two densities are deliberately not dumped.** They are the snapshot's *inputs*, read
    from NeqSim's `getDensity("kg/m3")` - the cubic's volume with the Peneloux shift - and the
    two implementations' shift paths agree to `6e-8` relative rather than to the dump's own
    band. A key that never agrees is noise in the one instrument whose job is to say which
    segment moved; the wetted area and the two film coefficients already carry what the density
    feeds.
    """
    from azoth.process.reference.rate_based_packed_column import DEFAULTS, _states

    def value(name: str, fallback: float) -> float:
        return float(inputs[name]) if name in inputs else fallback

    states = _states(
        [str(name) for name in inputs["gas_components"]],
        float(inputs["gas_n"]),
        [float(v) for v in inputs["gas_z"]],
        float(inputs["gas_p"]),
        float(inputs["gas_t"]),
        [str(name) for name in inputs["liquid_components"]],
        float(inputs["liquid_n"]),
        [float(v) for v in inputs["liquid_z"]],
        float(inputs["liquid_p"]),
        float(inputs["liquid_t"]),
        [str(name) for name in inputs["transfer_components"]]
        if "transfer_components" in inputs
        else None,
        value("column_diameter", DEFAULTS["column_diameter"]),
        value("packed_height", DEFAULTS["packed_height"]),
        int(value("number_of_segments", DEFAULTS["number_of_segments"])),
        str(inputs["packing_type"]) if "packing_type" in inputs else DEFAULTS["packing_type"],
        int(value("max_iterations", DEFAULTS["max_iterations"])),
        value("convergence_tolerance", DEFAULTS["convergence_tolerance"]),
        value("mass_transfer_correction", DEFAULTS["mass_transfer_correction"]),
        str(inputs["mass_transfer_correlation"])
        if "mass_transfer_correlation" in inputs
        else DEFAULTS["mass_transfer_correlation"],
        str(inputs["film_model"]) if "film_model" in inputs else DEFAULTS["film_model"],
        str(inputs["heat_transfer_model"])
        if "heat_transfer_model" in inputs
        else DEFAULTS["heat_transfer_model"],
        str(inputs["segment_solver"]) if "segment_solver" in inputs else DEFAULTS["segment_solver"],
        str(inputs["column_solver"]) if "column_solver" in inputs else DEFAULTS["column_solver"],
    )
    layers: dict[str, float] = {}
    for segment in states["segments"]:
        index = segment["number"]
        layers[f"segment{index}_gas_temperature_K"] = segment["gas_temperature"]
        layers[f"segment{index}_liquid_temperature_K"] = segment["liquid_temperature"]
        layers[f"segment{index}_wetted_area_m2_per_m3"] = segment["wetted_area"]
        layers[f"segment{index}_kga"] = segment["k_ga"]
        layers[f"segment{index}_kla"] = segment["k_la"]
        layers[f"segment{index}_interface_temperature_K"] = segment["interface_temperature"]
        layers[f"segment{index}_heat_transfer_rate_W"] = segment["heat_transfer_rate"]
        layers[f"segment{index}_percent_flood"] = segment["percent_flood"]
        layers[f"segment{index}_net_molar_transfer_mol_per_s"] = segment["net_molar_transfer"]
    return layers


def _as_inputs(spec: Mapping[str, Any], inputs: Mapping[str, Any]) -> dict[str, Any]:
    """A case's SI inputs as the quantities a reference's public entry takes.

    **A dumper reads a case, and a case's inputs are SI magnitudes** - the shape the Rust
    kernel and the cross-implementation comparison both use. The public reference takes
    *quantities*, because a caller of the library writes `feed_p = 5.0 * bar`. So a dumper
    has to put the unit back on, and it reads that unit from the spec's own declaration
    rather than from a table here: an input the spec gives no unit has none, and is passed
    through as the number, list or name it is.
    """
    from azoth.core.units import from_si

    declared = spec.get("inputs", {})
    out: dict[str, Any] = {}
    for name, value in inputs.items():
        unit = declared.get(name, {}).get("unit")
        # **A quantity only where the value has a dimension.** A spec declares a unit for
        # everything, and a `dimensionless` one is a *number* to the reference - an iteration
        # cap, a damping factor - so wrapping it would hand the arithmetic a quantity where it
        # reads a float, and a composition vector would be a quantity of a list.
        dimensional = bool(unit) and unit != "dimensionless"
        if dimensional and isinstance(value, (int, float)):
            out[name] = from_si(float(value), unit)
        else:
            out[name] = value
    return out


def _gibbs_reactor(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.gibbs_reactor`'s layers, from the solve the result was built from.

    **The solver's own iterations are the interior**, and the capture prints them: the
    reference's `min_iterations` is a floor that bites, so three of the capture's rows stop at
    exactly a hundred and a dump that reported the *convergence* rather than the count would
    agree with the answer and say nothing about the path. `converged` is deliberately not
    dumped: the capture writes it as `true`, which is not a number, and the iterations and the
    final error are what the comparison can hold.
    """
    from azoth.process.reference.gibbs_reactor import _spec, gibbs_reactor

    result = gibbs_reactor(**_as_inputs(_spec(), inputs))
    return {
        "iterations": float(result.iterations),
        "final_error": float(result.final_error),
        "product_T": result.product_t.to("K").magnitude,
        # **`product_h` is deliberately not dumped**, and neither are the moles nor the
        # composition. The
        # capture's `product_z` is the outlet *stream's own flash*, which on these rows reports
        # the feed - `specs/cases/process/gibbs_reactor.toml` says so and takes the class's
        # `outlet_moles` instead - and the two libraries' enthalpies diverge by `5.4e-2` at
        # 1000 K, measured and recorded there. A layer whose key the capture prints but this
        # port does not answer the same way is a *divergence to record*, not one to dump: the
        # comparison cannot be told to ignore it and would report it as a failure of the port.
    }


def _plug_flow_reactor(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.plug_flow_reactor`'s layers, from the march the result was built from.

    **The answer is a profile**, and what a capture prints *about* it is the scalars beside
    the table: the conversion, the pressure drop in the capture's own unit (bara), the outlet
    temperature and the duty. The stations are the capture's own `profile` value - one
    multi-line string - and a dump is numbers under the capture's key names, so they are not
    split out here; the residence time is on the reference's `March` and on no result field,
    so it is one of the keys the capture has and this dump does not.
    """
    from azoth.process.reference.plug_flow_reactor import _spec, plug_flow_reactor

    result = plug_flow_reactor(**_as_inputs(_spec(), inputs))
    return {
        "conversion": float(result.conversion),
        "pressure_drop_bar": result.pressure_drop.to("Pa").magnitude / 1.0e5,
        "outlet_temperature_K": result.outlet_temperature.to("K").magnitude,
        "heat_duty_W": result.heat_duty.to("W").magnitude,
    }


def _hem_release(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.hem_release`'s stations, from the reference's own arithmetic.

    **The enthalpy and entropy are not dumped.** The capture prints NeqSim's specific `h`
    and `s` on the ideal-gas datum, and this library's datum differs by a constant; every
    other station field is a state or a flux and compares directly. The four stations are
    the record's own rows, so the case holds the rate and this dump holds the profile.
    """
    from azoth.process.reference.hem_release import _states

    states = _states(
        [str(name) for name in inputs["components"]],
        [float(value) for value in inputs["upstream_z"]],
        float(inputs["upstream_t"]),
        float(inputs["upstream_p"]),
        float(inputs["diameter"]),
        float(inputs["discharge_coefficient"]),
        float(inputs["back_pressure"]),
    )
    dumped: dict[str, float] = {
        "mass_flow_kg_s": 0.0 if states.mass_flow is None else states.mass_flow
    }
    for index, row in enumerate(states.stations):
        prefix = f"station_{index}_"
        dumped[prefix + "p_pa"] = row.pressure
        dumped[prefix + "t_k"] = row.temperature
        dumped[prefix + "density_kg_m3"] = row.density
        dumped[prefix + "velocity_m_per_s"] = row.velocity
        dumped[prefix + "mass_flux_kg_m2_s"] = row.mass_flux
        dumped[prefix + "gas_mass_fraction"] = row.gas_mass_fraction
    return dumped


def _toxic_library(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.toxic_library`'s layers, from the reference's own arithmetic.

    A lookup and a conversion are separate arms: the row dumper returns the three
    thresholds when a known chemical is named, and the converted concentration when the
    conversion group is stated. An unknown name has no numeric layer.
    """
    from azoth.process.reference.toxic_library import _lookup, _ppm_to_kg_per_m3

    dumped: dict[str, float] = {}
    chemical = inputs.get("chemical")
    if chemical is not None:
        record = _lookup(str(chemical))
        if record is not None:
            dumped["idlh_ppm"] = record[1]
            dumped["erpg2_ppm"] = record[2]
            dumped["aegl2_ppm"] = record[3]
    ppm = inputs.get("concentration_ppm")
    if ppm is not None:
        dumped["mass_concentration_kg_m3"] = _ppm_to_kg_per_m3(
            float(ppm),
            float(inputs["molar_mass"]),
            float(inputs["temperature"]),
            float(inputs["pressure"]),
        )
    return dumped


def _hazardous_area(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.hazardous_area`'s layers, from the reference's own arithmetic.

    The zone and grade labels are strings and have no layer here; the three numbers are
    the ones the capture prints.
    """
    from azoth.process.reference.hazardous_area import (
        DEFAULT_AMBIENT_DENSITY,
        DEFAULT_SAFETY_FACTOR,
        _effective_diameter,
        _hazardous_distance,
        _lfl_mass_fraction,
    )

    flow = float(inputs["mass_flow"])
    density = float(inputs["exit_density"])
    velocity = float(inputs["exit_velocity"])
    lfl = float(inputs["lfl_volume_fraction"])
    molar = float(inputs["gas_molar_mass"])
    ambient = float(inputs.get("ambient_density", DEFAULT_AMBIENT_DENSITY))
    factor = float(inputs.get("safety_factor", DEFAULT_SAFETY_FACTOR))
    diameter = _effective_diameter(flow, density, velocity)
    lfl_mass = _lfl_mass_fraction(lfl, molar)
    return {
        "effective_diameter_m": diameter,
        "lfl_mass_fraction": lfl_mass,
        "hazardous_distance_m": _hazardous_distance(density, ambient, diameter, factor, lfl_mass),
    }


def _heavy_gas_dispersion(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.heavy_gas_dispersion`'s layers, from the reference's own arithmetic.

    **The distance is dumped only when a ratio is stated.** The published Britter parameter
    is the unfloored one; the fit's 0.01 floor lives inside the distance.
    """
    from azoth.process.reference.heavy_gas_dispersion import (
        _britter_parameter,
        _distance,
        _reduced_gravity,
    )

    rate = float(inputs["volumetric_rate"])
    source = float(inputs["source_density"])
    air = float(inputs["air_density"])
    wind = float(inputs["wind_speed"])
    reduced = _reduced_gravity(source, air)
    alpha = _britter_parameter(reduced, rate, wind)
    dumped: dict[str, float] = {
        "reduced_gravity_m_per_s2": reduced,
        "britter_parameter": alpha,
    }
    ratio = inputs.get("concentration_ratio")
    if ratio is not None:
        dumped["distance_to_concentration_ratio_m"] = _distance(rate, wind, alpha, float(ratio))
    return dumped


def _gaussian_plume(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.gaussian_plume`'s layers, from the reference's own arithmetic.

    **The distance is dumped only when it exists.** The capture prints the class's `NaN`
    when the scan never reaches the threshold, and a comparison against `NaN` fails at any
    tolerance, so the absent answer is absent here too.
    """
    from azoth.process.reference.gaussian_plume import (
        _sigma_y,
        _sigma_z,
        _stable,
        _terrain,
        concentration,
        distance_to_threshold,
    )

    stability = _stable(str(inputs["stability"]))
    terrain = _terrain(str(inputs["terrain"]))
    emission = float(inputs["emission_rate"])
    stack = float(inputs["effective_stack_height"])
    wind = float(inputs["wind_speed"])
    x = float(inputs["distance"])
    y = float(inputs["crosswind"])
    z = float(inputs["height"])
    threshold = float(inputs["threshold"])
    dumped: dict[str, float] = {
        "sigma_y_m": _sigma_y(x, stability, terrain),
        "sigma_z_m": _sigma_z(x, stability, terrain),
        "concentration_kg_m3": concentration(emission, stack, wind, stability, terrain, x, y, z),
        "centerline_ground_concentration_kg_m3": concentration(
            emission, stack, wind, stability, terrain, x, 0.0, 0.0
        ),
    }
    distance = distance_to_threshold(emission, stack, wind, stability, terrain, threshold)
    if distance is not None:
        dumped["distance_to_threshold_m"] = distance
    return dumped


def _gas_dispersion(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.gas_dispersion`'s layers, from the reference's own arithmetic.

    **The capture's threshold key is ppm and the model's is a volume fraction**, so the
    dump converts it back to the class's unit; the strings and booleans are the cases'
    and have no float to compare.
    """
    from azoth.core.units import quantity
    from azoth.process.reference.gas_dispersion import gas_dispersion

    result = gas_dispersion(
        [str(name) for name in inputs["components"]],
        [float(value) for value in inputs["composition"]],
        quantity(float(inputs["mass_release_rate"]), "kg/s"),
        ambient_temperature=quantity(float(inputs["ambient_temperature"]), "K"),
        atmospheric_pressure=quantity(float(inputs["atmospheric_pressure"]), "Pa"),
        wind_speed=quantity(float(inputs["wind_speed"]), "m/s"),
        pasquill_stability_class=str(inputs["pasquill_stability_class"]),
        surface_roughness=quantity(float(inputs["surface_roughness"]), "m"),
        offshore=bool(inputs["offshore"]),
        release_height=quantity(float(inputs["release_height"]), "m"),
        model_selection=str(inputs["model_selection"]),
        lower_flammable_limit=(
            None
            if inputs.get("lower_flammable_limit") is None
            else float(inputs["lower_flammable_limit"])
        ),
        flammable_endpoint_fraction_of_lfl=float(inputs["flammable_endpoint_fraction_of_lfl"]),
        toxic_component=inputs.get("toxic_component"),
        toxic_threshold=inputs.get("toxic_threshold"),
    )
    dumped: dict[str, float] = {
        "result_mass_release_rate_kg_s": result.mass_release_rate.to("kg/s").magnitude,
        "flammable_mass_release_rate_kg_s": result.flammable_mass_release_rate.to("kg/s").magnitude,
        "source_density_kg_m3": result.source_density.to("kg/m**3").magnitude,
        "air_density_kg_m3": result.air_density.to("kg/m**3").magnitude,
        "fuel_mole_fraction": result.fuel_mole_fraction,
        "fuel_mass_fraction": result.fuel_mass_fraction,
        "endpoint_fraction_of_lfl": result.flammable_endpoint_fraction,
        "flammable_cloud_volume_m3": result.flammable_cloud_volume.to("m**3").magnitude,
        "result_wind_speed_m_per_s": result.wind_speed.to("m/s").magnitude,
    }
    if result.lower_flammable_limit is not None:
        dumped["lfl_volume_fraction"] = result.lower_flammable_limit
    for field, key in (
        ("distance_to_flammable_endpoint", "distance_to_flammable_endpoint_m"),
        ("distance_to_half_lfl", "distance_to_half_lfl_m"),
        ("distance_to_lfl", "distance_to_lfl_m"),
        ("toxic_distance", "toxic_distance_m"),
    ):
        value = getattr(result, field)
        if value is not None:
            dumped[key] = value.to("m").magnitude
    if result.toxic_threshold is not None:
        dumped["result_toxic_threshold_ppm"] = result.toxic_threshold * 1.0e6
    return dumped


def _jet_fire(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.jet_fire`'s layers, from the reference's own arithmetic.

    The capture prints the default humidity as the string `default_0.5`; the model's own
    absence is the same state, so the dumper states nothing for it.
    """
    from azoth.core.units import quantity
    from azoth.process.reference.jet_fire import jet_fire as jet_fire_model

    humidity = inputs.get("relative_humidity")
    distance = inputs.get("distance")
    target = inputs.get("target_flux")
    result = jet_fire_model(
        quantity(float(inputs["mass_flow"]), "kg/s"),
        float(inputs["heat_of_combustion"]),
        float(inputs["radiative_fraction"]),
        relative_humidity=None if humidity is None else float(humidity),
        distance=None if distance is None else quantity(float(distance), "m"),
        target_flux=None if target is None else quantity(float(target), "W/m**2"),
    )
    dumped: dict[str, float] = {
        "total_radiative_power_w": result.total_radiative_power.to("W").magnitude,
    }
    if result.transmissivity is not None:
        dumped["transmissivity"] = result.transmissivity
    if result.incident_heat_flux is not None:
        dumped["incident_heat_flux_w_m2"] = result.incident_heat_flux.to("W/m**2").magnitude
    if result.distance_to_flux is not None:
        dumped["distance_to_flux_m"] = result.distance_to_flux.to("m").magnitude
    return dumped


def _pool_fire(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.pool_fire`'s layers, from the reference's own arithmetic.

    The capture's dead-parameter row is the same state under three values the class never
    reads, so it has no case; the model declares only the diameter and the burn rate.
    """
    from azoth.core.units import quantity
    from azoth.process.reference.pool_fire import pool_fire as pool_fire_model

    air = inputs.get("air_density")
    distance = inputs.get("distance")
    target = inputs.get("target_flux")
    result = pool_fire_model(
        quantity(float(inputs["pool_diameter"]), "m"),
        quantity(float(inputs["burn_rate"]), "kg/(m**2*s)"),
        air_density=None if air is None else float(air),
        distance=None if distance is None else quantity(float(distance), "m"),
        target_flux=None if target is None else quantity(float(target), "W/m**2"),
    )
    dumped: dict[str, float] = {
        "total_burn_rate_kg_s": result.total_burn_rate.to("kg/s").magnitude,
        "flame_height_m": result.flame_height.to("m").magnitude,
        "surface_emissive_power_w_m2": result.surface_emissive_power.to("W/m**2").magnitude,
    }
    if result.view_factor is not None:
        dumped["view_factor"] = result.view_factor
    if result.incident_heat_flux is not None:
        dumped["incident_heat_flux_w_m2"] = result.incident_heat_flux.to("W/m**2").magnitude
    if result.distance_to_flux is not None:
        dumped["distance_to_flux_m"] = result.distance_to_flux.to("m").magnitude
    return dumped


def _pfp_demand(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.pfp_demand`'s layers, from the reference's own arithmetic.

    The verdict and the rating are the model's own words rather than numbers, so they are
    asserted by the model's test and the cross-impl comparison, not dumped here.
    """
    from azoth.core.units import quantity
    from azoth.process.reference.pfp_demand import _evaluate

    def optional(name: str, unit: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), unit)

    alpha = inputs.get("absorptivity")
    solution, _ = _evaluate(
        quantity(float(inputs["incident_heat_flux"]), "W/m**2"),
        quantity(float(inputs["steel_thickness"]), "m"),
        quantity(float(inputs["required_survival_time"]), "s"),
        absorptivity=None if alpha is None else float(alpha),
        initial_temperature=optional("initial_temperature", "K"),
        critical_temperature=optional("critical_temperature", "K"),
        fire_temperature=optional("fire_temperature", "K"),
        pfp_conductivity=optional("pfp_conductivity", "W/(m*K)"),
        fire_type=inputs.get("fire_type"),
    )
    dumped: dict[str, float] = {
        "steel_mass_per_area_kg_m2": solution.mass_per_area,
        "bare_steel_time_to_critical_s": solution.bare,
        "required_survival_time_s": solution.survival,
        "required_pfp_thickness_m": solution.thickness,
        "required_pfp_thickness_mm": solution.thickness * 1000.0,
        "q_abs_w_m2": solution.q_abs,
    }
    if (
        solution.q_allow is not None
        and solution.steel_average is not None
        and solution.driving is not None
    ):
        dumped["q_allow_w_m2"] = solution.q_allow
        dumped["steel_avg_temperature_k"] = solution.steel_average
        dumped["driving_temperature_k"] = solution.driving
    return dumped


def _fire_water_demand(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.fire_water_demand`'s layers, from the reference's own arithmetic.

    The class works in `L/min` and `m3/h`; the solution is SI, and the capture's keys are
    the class's units. The ratio is omitted where the class answers NaN.
    """
    from azoth.core.units import quantity
    from azoth.process.reference.fire_water_demand import _evaluate

    def optional(name: str, unit: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), unit)

    rows = [
        {
            "surface_area": quantity(float(row["surface_area"]), "m**2"),
            "application_rate": float(row["application_rate"]),
        }
        for row in inputs.get("protected_objects", [])
    ]
    solution, _ = _evaluate(
        quantity(float(inputs["protected_area"]), "m**2"),
        application_rate=None
        if inputs.get("application_rate") is None
        else float(inputs["application_rate"]),
        duration=optional("duration", "s"),
        foam_concentrate_percent=None
        if inputs.get("foam_concentrate_percent") is None
        else float(inputs["foam_concentrate_percent"]),
        simultaneous_area_factor=None
        if inputs.get("simultaneous_area_factor") is None
        else float(inputs["simultaneous_area_factor"]),
        protected_objects=rows,
    )
    dumped: dict[str, float] = {
        "area_demand_lpm": solution.area_demand * 60000.0,
        "object_demand_lpm": solution.object_demand * 60000.0,
        "total_demand_lpm": solution.total_demand * 60000.0,
        "total_demand_m3_per_hour": solution.total_demand_m3_per_hour * 3600.0,
        "water_volume_m3": solution.water_volume,
        "foam_concentrate_volume_m3": solution.foam_concentrate_volume,
    }
    if solution.ratio is not None:
        dumped["object_to_area_demand_ratio"] = solution.ratio
    return dumped


def _deluge_nozzle_layout(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.deluge_nozzle_layout`'s layers, from the reference's own arithmetic.

    The criterion and the three flags are the model's own words, asserted by its test and the
    cross-impl comparison rather than dumped here.
    """
    from azoth.core.units import quantity
    from azoth.process.reference.deluge_nozzle_layout import _evaluate

    def optional(name: str, unit: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), unit)

    efficiency = inputs.get("coverage_efficiency")
    solution, _ = _evaluate(
        quantity(float(inputs["protected_area"]), "m**2"),
        float(inputs["required_density"]),
        float(inputs["nozzle_k"]),
        quantity(float(inputs["nozzle_min_pressure"]), "bar"),
        quantity(float(inputs["nozzle_max_spacing"]), "m"),
        quantity(float(inputs["nozzle_clearance"]), "m"),
        quantity(float(inputs["available_clearance"]), "m"),
        operating_pressure=optional("operating_pressure", "bar"),
        coverage_efficiency=None if efficiency is None else float(efficiency),
    )
    return {
        "required_flow_lpm": solution.required_flow * 60000.0,
        "flow_per_nozzle_lpm": solution.flow_per_nozzle * 60000.0,
        "coverage_per_nozzle_m2": solution.coverage_per_nozzle,
        "nozzle_count_from_flow": solution.count_from_flow,
        "nozzle_count_from_coverage": solution.count_from_coverage,
        "nozzle_count": solution.nozzle_count,
        "grid_spacing_m": solution.grid,
        "delivered_flow_lpm": solution.delivered_flow * 60000.0,
        "delivered_density_lpm_m2": solution.delivered_density,
    }


def _fire_monitor_coverage(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.fire_monitor_coverage`'s layers, from the reference's own arithmetic.

    The verdict and the two predicates are the model's own words, asserted by its test and
    the cross-impl comparison rather than dumped here. The flow needed is omitted where the
    drift leaves no coverage, the class's `+Infinity`.
    """
    from azoth.core.units import quantity
    from azoth.process.reference.fire_monitor_coverage import _evaluate

    def optional(name: str, unit: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), unit)

    count = inputs.get("monitor_count")
    sweep = inputs.get("sweep_angle")
    solution, _ = _evaluate(
        quantity(float(inputs["target_area"]), "m**2"),
        float(inputs["required_density"]),
        monitor_count=None if count is None else float(count),
        monitor_flow=optional("monitor_flow", "L/min"),
        effective_throw=optional("effective_throw", "m"),
        sweep_angle=None if sweep is None else float(sweep),
        wind_speed=optional("wind_speed", "m/s"),
        fall_height=optional("fall_height", "m"),
        droplet_terminal_velocity=optional("droplet_terminal_velocity", "m/s"),
        characteristic_dimension=optional("characteristic_dimension", "m"),
        line_of_sight_obstructed=inputs.get("line_of_sight_obstructed"),
    )
    dumped: dict[str, float] = {
        "total_flow_lpm": solution.total_flow * 60000.0,
        "swept_area_m2": solution.swept_area,
        "nominal_density_lpm_m2": solution.nominal_density,
        "drift_displacement_m": solution.drift,
        "wind_coverage_fraction": solution.fraction,
        "effective_density_lpm_m2": solution.effective_density,
    }
    if solution.needed is not None:
        dumped["flow_needed_for_compliance_lpm"] = solution.needed * 60000.0
    return dumped


def _fire_water_coverage_assessment(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.fire_water_coverage_assessment`'s layers, from the reference's arithmetic.

    The verdict and the two flags are the model's own words, asserted by its test and the
    cross-impl comparison rather than dumped here.
    """
    from azoth.core.units import quantity
    from azoth.process.reference.fire_water_coverage_assessment import _evaluate

    def optional(name: str, unit: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), unit)

    solution, _ = _evaluate(
        quantity(float(inputs["protected_area"]), "m**2"),
        quantity(float(inputs["required_flow"]), "L/min"),
        quantity(float(inputs["spare_supply_flow"]), "L/min"),
        quantity(float(inputs["worst_case_pressure_margin"]), "bar"),
        covered_area=optional("covered_area", "m**2"),
        existing_delivered_flow=optional("existing_delivered_flow", "L/min"),
        required_pressure_margin=optional("required_pressure_margin", "bar"),
    )
    return {
        "coverage_gap_m2": solution.gap,
        "coverage_fraction": solution.fraction,
        "flow_deficit_lpm": solution.deficit * 60000.0,
    }


def _active_fire_protection_screening(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.active_fire_protection_screening`'s layers, from the reference's arithmetic.

    The two flags are the model's own words, asserted by its test and the cross-impl
    comparison rather than dumped here.
    """
    from azoth.core.units import quantity
    from azoth.process.reference.active_fire_protection_screening import _evaluate

    solution, _ = _evaluate(
        str(inputs["scenario"]),
        quantity(float(inputs["blowdown_time_to_target"]), "s"),
        quantity(float(inputs["bare_steel_time_to_critical"]), "s"),
        quantity(float(inputs["dedicated_object_demand"]), "L/min"),
        quantity(float(inputs["general_area_demand"]), "L/min"),
    )
    return {
        "blowdown_margin_s": solution.margin,
        "selective_protection_saving_fraction": solution.saving,
    }


def _api537_flare_flame_model(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.api537_flare_flame_model`'s layers, from the reference's own arithmetic.

    The sterile zones are a record list; the capture prints one radius per target in the
    caller's order, so the dump flattens the same way.
    """
    from azoth.core.units import quantity
    from azoth.process.reference.api537_flare_flame_model import _evaluate

    def optional(name: str, unit: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), unit)

    targets = [{"flux": quantity(float(row["flux"]), "W/m**2")} for row in inputs["target_fluxes"]]
    tau = inputs.get("transmissivity")
    eta = inputs.get("acoustic_efficiency")
    solution, _ = _evaluate(
        quantity(float(inputs["mass_flow"]), "kg/s"),
        float(inputs["heat_of_combustion"]),
        float(inputs["radiative_fraction"]),
        quantity(float(inputs["exit_velocity"]), "m/s"),
        quantity(float(inputs["ground_distance"]), "m"),
        quantity(float(inputs["sound_distance"]), "m"),
        targets,
        stack_height=optional("stack_height", "m"),
        wind_speed=optional("wind_speed", "m/s"),
        transmissivity=None if tau is None else float(tau),
        acoustic_efficiency=None if eta is None else float(eta),
    )
    dumped: dict[str, float] = {
        "heat_release_w": solution.heat_release,
        "radiative_power_w": solution.radiative_power,
        "flame_length_m": solution.length,
        "flame_tilt_rad": solution.tilt,
        "flame_center_horizontal_m": solution.center_horizontal,
        "flame_center_height_m": solution.center_height,
        "flame_tip_horizontal_m": solution.tip_horizontal,
        "flame_tip_height_m": solution.tip_height,
        "heat_flux_at_ground_distance_w_m2": solution.flux,
        "sound_power_level_db": solution.pwl,
        "sound_pressure_level_db": solution.spl,
        "equivalent_flame_temperature_k": solution.temperature,
    }
    for index, (_, radius) in enumerate(solution.sterile):
        dumped[f"sterile_radius_{index}_m"] = radius
    return dumped


def _vce(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.vce`'s layers, from the reference's own arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.vce import vce as vce_model

    ambient = inputs.get("ambient_pressure")
    distance = inputs.get("distance")
    target = inputs.get("target_overpressure")
    result = vce_model(
        quantity(float(inputs["cloud_fuel_mass"]), "kg"),
        float(inputs["heat_of_combustion"]),
        float(inputs["strength_class"]),
        ambient_pressure=None if ambient is None else quantity(float(ambient), "Pa"),
        distance=None if distance is None else quantity(float(distance), "m"),
        target_overpressure=None if target is None else quantity(float(target), "Pa"),
    )
    dumped: dict[str, float] = {
        "combustion_energy_j": result.combustion_energy.to("J").magnitude,
    }
    if result.scaled_distance is not None:
        dumped["scaled_distance"] = result.scaled_distance
    if result.dimensionless_overpressure is not None:
        dumped["dimensionless_overpressure"] = result.dimensionless_overpressure
    if result.overpressure is not None:
        dumped["overpressure_pa"] = result.overpressure.to("Pa").magnitude
    if result.distance_to_overpressure is not None:
        dumped["distance_to_overpressure_m"] = result.distance_to_overpressure.to("m").magnitude
    return dumped


def _bleve(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.bleve`'s layers, from the reference's own arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.bleve import bleve as bleve_model

    humidity = inputs.get("relative_humidity")
    distance = inputs.get("distance")
    target = inputs.get("target_flux")
    result = bleve_model(
        quantity(float(inputs["fuel_mass"]), "kg"),
        float(inputs["heat_of_combustion"]),
        float(inputs["radiative_fraction"]),
        relative_humidity=None if humidity is None else float(humidity),
        distance=None if distance is None else quantity(float(distance), "m"),
        target_flux=None if target is None else quantity(float(target), "W/m**2"),
    )
    dumped: dict[str, float] = {
        "fireball_diameter_m": result.fireball_diameter.to("m").magnitude,
        "fireball_duration_s": result.fireball_duration.to("s").magnitude,
        "surface_emissive_power_w_m2": result.surface_emissive_power.to("W/m**2").magnitude,
    }
    if result.transmissivity is not None:
        dumped["transmissivity"] = result.transmissivity
    if result.incident_heat_flux is not None:
        dumped["incident_heat_flux_w_m2"] = result.incident_heat_flux.to("W/m**2").magnitude
    if result.distance_to_flux is not None:
        dumped["distance_to_flux_m"] = result.distance_to_flux.to("m").magnitude
    return dumped


def _consequence_analysis(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.consequence_analysis`'s layers, through the reference's own row evaluator.

    The capture's labels and types are strings, which the cases compare; the dumper returns
    the numeric columns the layer harness can hold to the capture.
    """
    from azoth import _models_gen
    from azoth.core.units import input_records_to_si, quantity
    from azoth.process.reference.consequence_analysis import _evaluate_row

    spec = _models_gen.model("process.consequence_analysis")
    fields = spec["inputs"]["outcomes"]["fields"]
    prepared = []
    for row in inputs["outcomes"]:
        converted: dict[str, Any] = {}
        for name, declaration in fields.items():
            value = row.get(name)
            if value is None:
                converted[name] = None
            elif declaration.get("type") in ("enum", "string", "boolean"):
                converted[name] = value
            elif declaration.get("unit") == "dimensionless":
                converted[name] = float(value)
            else:
                converted[name] = quantity(float(value), str(declaration["unit"]))
        prepared.append(converted)
    rows: list[dict[str, Any]] = input_records_to_si(spec, "outcomes", prepared)
    release = float(inputs["release_frequency_per_year"])
    distance = float(inputs["receiver_distance"])
    dumped: dict[str, float] = {"outcome_count": float(len(rows))}
    risk = 0.0
    for index, row in enumerate(rows):
        _, _, frequency, probability = _evaluate_row(index, row, release, distance)
        dumped[f"outcome{index}_frequency_per_year"] = frequency
        dumped[f"outcome{index}_fatality_probability"] = probability
        dumped[f"outcome{index}_fatality_frequency_per_year"] = frequency * probability
        risk += frequency * probability
    dumped["individual_fatality_risk_per_year"] = risk
    return dumped


def _probit(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.probit`'s layers, from the reference's own arithmetic.

    The capture prints `probability_from_dose` and `probability` as separate methods; the
    model publishes `probit` and one `probability`, so the two calls reconstruct both.
    """
    from azoth.process.reference.probit import probit as probit_model

    a = float(inputs["a"])
    b = float(inputs["b"])
    n = float(inputs["n"])
    dose = inputs.get("dose")
    stated = inputs.get("probit_value")
    dumped: dict[str, float] = {"a": a, "b": b, "n": n}
    if dose is not None:
        from_dose = probit_model(a, b, n, dose=float(dose))
        dumped["dose"] = float(dose)
        if from_dose.probit is not None:
            dumped["probit"] = from_dose.probit
        dumped["probability_from_dose"] = from_dose.probability
    if stated is not None:
        from_probit = probit_model(a, b, n, probit_value=float(stated))
        dumped["probit_value"] = float(stated)
        dumped["probability"] = from_probit.probability
    return dumped


def _leak_blowdown(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.leak_blowdown`'s series, from the reference's own arithmetic.

    **No `beta`, velocity, momentum or SMD.** The class's single-phase `getBeta()` and its
    cached density make those a record of the class's behaviour rather than the state's; the
    rate, pressure, temperature and gamma compare directly.
    """
    from azoth.process.reference.leak_blowdown import _states

    volume = float(inputs.get("vessel_volume", 1.0))
    diameter = float(inputs.get("hole_diameter", 0.01))
    coefficient = float(inputs.get("discharge_coefficient", 0.62))
    back = float(inputs.get("back_pressure", 101325.0))
    step = float(inputs.get("time_step", 1.0))
    states = _states(
        [str(name) for name in inputs["components"]],
        [float(value) for value in inputs["initial_z"]],
        float(inputs["initial_t"]),
        float(inputs["initial_p"]),
        float(inputs["duration"]),
        volume,
        diameter,
        coefficient,
        back,
        step,
        str(inputs.get("scenario_name", "Release")),
    )
    dumped: dict[str, float] = {
        "initial_density_kg_m3": states.initial_mass / volume,
        "initial_gamma": states.initial_gamma,
        "total_mass_released_kg": (
            0.0 if states.total_mass_released is None else states.total_mass_released
        ),
        "peak_mass_flow_kg_s": (
            0.0 if states.peak_mass_flow_rate is None else states.peak_mass_flow_rate
        ),
        "time_to_empty_s": 0.0 if states.time_to_empty is None else states.time_to_empty,
        "declared_points": float(states.number_of_points),
        "recorded_points": float(len(states.points)),
    }
    for index, row in enumerate(states.points):
        prefix = f"point_{index}_"
        dumped[prefix + "t_s"] = row.time
        dumped[prefix + "mass_flow_kg_s"] = row.mass_flow_rate
        dumped[prefix + "temperature_k"] = row.temperature
        dumped[prefix + "pressure_pa"] = row.pressure
        dumped[prefix + "gamma"] = states.gammas[index]
    return dumped


def _separator(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.separator`'s capacity surface, from the reference's own factored arithmetic.

    The five getters are the model's second fold, and the capture prints the intermediates
    on the way to them. **The two density rules are two layers**: `liquid_density_kg_m3` is
    the sizing family's - the liquid's own, or `1000.0` with no liquid - while
    `gas_load_factor_m_per_s` is formed from the floored one. A port that collapsed them
    would agree on every two-phase row here and disagree on a near-dry one.
    """
    from azoth.core.units import quantity
    from azoth.process.reference.separator import _capacity_states, _route, _sizing_getters

    components = [str(name) for name in inputs["components"]]
    feed_z = [float(v) for v in inputs["feed_z"]]
    states = _route(
        components,
        float(inputs["feed_n"]),
        feed_z,
        float(inputs["feed_p"]),
        quantity(float(inputs["feed_t"]), "K"),
        float(inputs["pressure_drop"]),
        float(inputs["gas_in_liquid"]),
        None if inputs.get("heat_input") is None else float(inputs["heat_input"]),
    )
    # **A mechanical design case states no vessel**, so the capacity fold has no diameter to
    # divide by; its own layers are dumped from `_mechanical_layers`.
    if inputs.get("internal_diameter") is None:
        if not inputs.get("mechanical_design"):
            return {}
        return _mechanical_layers(inputs, states)
    level = inputs.get("design_liquid_level_fraction")
    capacity = _capacity_states(
        components,
        states.temperature.to("K").magnitude,
        states.p_out,
        feed_z,
        states.n_vapour,
        list(states.liquid_z),
        float(inputs["internal_diameter"]),
        float(inputs["design_gas_load_factor"]),
        str(inputs.get("orientation") or "horizontal"),
        0.8 if level is None else float(level),
    )
    dump = {"capacity_utilization": capacity.utilization}
    gas_density = capacity.gas_density
    gas_area = capacity.gas_area
    volumetric_flow = capacity.volumetric_flow
    max_flow = capacity.max_flow
    if gas_density is None or gas_area is None or volumetric_flow is None or max_flow is None:
        return dump
    dump |= {
        "gas_density_kg_m3": gas_density,
        "gas_area_m2": gas_area,
        "gas_volumetric_flow_m3_s": volumetric_flow,
        "max_allowable_gas_flow_m3_s": max_flow,
    }
    # `liquid_density_kg_m3` is the **liquid phase's**, and a dry system has none to print:
    # the capture says `none` and the comparison would float it.
    liquid_density = capacity.liquid_density
    if capacity.has_liquid and liquid_density is not None:
        dump["liquid_density_kg_m3"] = liquid_density
    max_velocity = capacity.max_velocity
    superficial = capacity.superficial_velocity
    load_factor = capacity.load_factor
    if max_velocity is not None and superficial is not None and load_factor is not None:
        dump |= {
            "max_allowable_gas_velocity_m_per_s": max_velocity,
            "gas_superficial_velocity_m_per_s": superficial,
            "gas_load_factor_m_per_s": load_factor,
        }
    # **The sizing capture's own three layers**, on the same run: the operating area
    # fraction, the phase-indexed load factors and `sizeFromFlow`'s diameter. The indexed
    # values are dumped under the capture's per-phase key, because the record carries one
    # number per liquid phase and the capture prints one line per phase.
    sizing = _sizing_getters(
        components,
        states.temperature.to("K").magnitude,
        states.p_out,
        feed_z,
        states.n_vapour,
        list(states.liquid_z),
        float(inputs["internal_diameter"]),
        float(inputs["design_gas_load_factor"]),
        str(inputs.get("orientation") or "horizontal"),
        0.8 if level is None else float(level),
        5.0 if inputs.get("separator_length") is None else float(inputs["separator_length"]),
        0.5
        if inputs.get("liquid_level_fraction") is None
        else float(inputs["liquid_level_fraction"]),
        None if inputs.get("safety_factor") is None else float(inputs["safety_factor"]),
    )
    if sizing is not None:
        dump["gas_area_fraction"] = sizing.gas_area_fraction
        if sizing.required_diameter is not None:
            dump["sized_diameter_m"] = sizing.required_diameter
        for index, value in enumerate(sizing.rated_by_phase, start=1):
            dump[f"gas_load_factor_phase_{index}_m_per_s"] = value
    return dump


def _mechanical_layers(inputs: Mapping[str, Any], states: Any) -> dict[str, float]:
    """`process.separator`'s mechanical-design layers, under the capture's key names.

    The design's intermediates are its own arithmetic, and the capture prints them: the two
    volume flows, the shell and skid weights, the module box, the levels and the three nozzle
    diameters. A design that moved one step would move the ones after it, which is why the
    dump carries the chain rather than the final diameter alone.
    """
    from azoth.process.reference.separator import _mechanical_design

    components = [str(name) for name in inputs["components"]]
    design = _mechanical_design(
        components,
        states.temperature.to("K").magnitude,
        states.p_out,
        list(states.y),
        states.n_vapour,
        list(states.liquid_z),
        states.n_liquid,
        str(inputs.get("orientation") or "horizontal") == "horizontal",
        float(inputs["design_gas_load_factor"]),
        0.8 if inputs.get("fg") is None else float(inputs["fg"]),
        1.0
        if inputs.get("volume_safety_factor") is None
        else float(inputs["volume_safety_factor"]),
        150.0 if inputs.get("retention_time") is None else float(inputs["retention_time"]),
        1.0e7
        if inputs.get("max_operation_pressure") is None
        else float(inputs["max_operation_pressure"]),
        (
            95.147650602e6
            if inputs.get("allowable_stress") is None
            else float(inputs["allowable_stress"])
        )
        / 1.0e6,
        1.0 if inputs.get("joint_efficiency") is None else float(inputs["joint_efficiency"]),
        0.0
        if inputs.get("corrosion_allowance") is None
        else float(inputs["corrosion_allowance"]) * 1000.0,
    )
    diameter = design.inner_diameter
    return {
        "inner_diameter_m": diameter,
        "tantan_length_m": design.tantan_length,
        "outer_diameter_m": design.outer_diameter,
        "wall_thickness_m": design.wall_thickness,
        "max_design_volume_flow_m3_hr": design.max_design_volume_flow_m3_hr,
        "max_design_gas_volume_flow_m3_hr": design.max_design_gas_volume_flow_m3_hr,
        "vessel_shell_weight_kg": design.vessel_shell_weight_kg,
        "piping_weight_kg": design.vessel_shell_weight_kg * 0.4,
        "structural_weight_kg": design.vessel_shell_weight_kg * 0.1,
        "electrical_weight_kg": design.vessel_shell_weight_kg * 0.08,
        "total_weight_kg": design.total_weight_kg,
        "module_width_m": design.module_width,
        "module_length_m": design.module_length,
        "module_height_m": design.module_height,
        "vessel_volume_m3": design.vessel_volume_m3,
        "hhll_m": design.hhll,
        "hll_m": design.hll,
        "nll_m": design.nll,
        "lll_m": design.lll,
        "llll_m": design.llll,
        "weir_height_m": design.weir_height,
        "hil_m": 0.25 * diameter,
        "nil_m": design.nil,
        "lil_m": 0.15 * diameter,
        "effective_length_liquid_m": design.effective_length_liquid,
        "effective_length_gas_m": design.effective_length_gas,
        "calc_liquid_retention_time_s": design.liquid_retention_time_s,
        "calc_surge_volume_m3": design.surge_volume_m3,
        "calc_surge_time_s": design.surge_time_s,
        "calc_inlet_nozzle_id_m": design.inlet_nozzle_id,
        "calc_gas_outlet_nozzle_id_m": design.gas_outlet_nozzle_id,
        "calc_oil_outlet_nozzle_id_m": design.oil_outlet_nozzle_id,
    }


def _anti_surge_recycle(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.anti_surge_recycle`'s layers: the machine the loop converged.

    The record carries the four flows and the distance to surge; **the machine that produced
    them is the interior**, and the capture prints its inlet's molar flow, pressure,
    temperature and enthalpy beside the shaft power and the discharge state at the converged
    recycle mass. This rebuilds that state from the result's own recycle mass through the
    reference's mixer and machine, so a wrong recycle mass or a wrong mixed state moves the
    numbers. On the inactive row the recycle is zero and the reconstruction is the natural
    suction, which is the state `solve` leaves untouched.
    """
    from azoth import _models_gen
    from azoth.core.units import ureg
    from azoth.eos import components as databank
    from azoth.process.reference import capacity
    from azoth.process.reference.anti_surge_recycle import (
        DEFAULT_COOLER_TEMPERATURE,
        DEFAULT_DAMPING,
        DEFAULT_MARGIN,
        DEFAULT_MAX_ITERATIONS,
        DEFAULT_SPEED,
        DEFAULT_TOLERANCE,
        _machine,
    )
    from azoth.process.reference.anti_surge_recycle import (
        anti_surge_recycle as model,
    )
    from azoth.process.reference.mixer import _route

    spec = _models_gen.model("process.anti_surge_recycle")

    def quantity(value: Any, unit: str) -> Any:
        return ureg.Quantity(float(value), unit)

    def records(name: str) -> list[dict[str, Any]]:
        fields = spec["inputs"][name]["fields"]
        return [
            {
                key: quantity(value, fields[key]["unit"])
                for key, value in row.items()
                if value is not None
            }
            for row in inputs[name]
        ]

    components = [str(name) for name in inputs["components"]]
    feed_z = [float(value) for value in inputs["feed_z"]]
    feed_n = float(inputs["feed_n"])
    feed_p = float(inputs["feed_p"])
    feed_t = float(inputs["feed_t"])
    cooler = float(inputs.get("recycle_cooler_temperature", DEFAULT_COOLER_TEMPERATURE))
    speed = float(inputs.get("speed", DEFAULT_SPEED))
    margin = float(inputs.get("surge_control_margin", DEFAULT_MARGIN))
    curve = records("curve")
    surge_curve = records("surge_curve")

    result = model(
        components,
        quantity(feed_n, "mol/s"),
        feed_z,
        quantity(feed_p, "Pa"),
        quantity(feed_t, "K"),
        curve,
        surge_curve,
        speed=speed,
        surge_control_margin=margin,
        recycle_cooler_temperature=quantity(cooler, "K"),
        damping=float(inputs.get("damping", DEFAULT_DAMPING)),
        tolerance=float(inputs.get("tolerance", DEFAULT_TOLERANCE)),
        max_iterations=int(inputs.get("max_iterations", DEFAULT_MAX_ITERATIONS)),
    )

    mixture, _ = databank.mixture_of(components, eos="pr")
    molar_mass = capacity._molar_mass(mixture, feed_z)
    recycle_n = result.recycle_mass_flow.to("kg/s").magnitude / molar_mass
    mixed = _route(
        components,
        [feed_n, recycle_n],
        [list(feed_z), list(feed_z)],
        [feed_p, feed_p],
        [feed_t, cooler],
        None,
    )
    machine = _machine(
        components,
        mixed.n_total,
        list(mixed.z),
        mixed.pressure,
        mixed.product_t.to("K").magnitude,
        curve,
        surge_curve,
        speed,
        margin,
    )
    return {
        "compressor_inlet_n": mixed.n_total,
        "compressor_inlet_P": mixed.pressure / 1.0e5,
        "compressor_inlet_T": mixed.product_t.to("K").magnitude,
        "compressor_inlet_h": mixed.h_out,
        "compressor_inlet_kg_hr": mixed.n_total * molar_mass * 3600.0,
        "compressor_power_kw": machine.power.to("kW").magnitude,
        "compressor_outlet_p_bara": machine.outlet_p.to("bar").magnitude,
        "compressor_outlet_T": machine.outlet_t.to("K").magnitude,
        "compressor_outlet_h": machine.outlet_h.to("J/mol").magnitude,
    }


def _fault_tree(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.fault_tree`'s layers, from the reference's own arithmetic.

    The cut-set members are strings and are asserted by the case and the cross-impl test;
    the cardinalities are numbers and are dumped here.
    """
    from azoth.process.reference.fault_tree import fault_tree as evaluate

    result = evaluate(
        [dict(row) for row in inputs["nodes"]],
        float(inputs["max_cardinality"]),
    )
    layers = {
        "top_event_probability": result.top_event_probability,
        "cut_set_count": result.cut_set_count,
    }
    for index, row in enumerate(result.minimal_cut_sets):
        layers[f"cut_set{index}_cardinality"] = row.cardinality
    return layers


def _bow_tie(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.bow_tie`'s layers, from the reference's own arithmetic."""
    from azoth.process.reference.bow_tie import bow_tie as evaluate

    result = evaluate(
        [dict(row) for row in inputs["threats"]],
        [dict(row) for row in inputs["consequences"]],
        [dict(row) for row in inputs["barriers"]],
    )
    return {
        "unmitigated_frequency_per_year": result.unmitigated_frequency_per_year,
        "mitigated_frequency_per_year": result.mitigated_frequency_per_year,
        "total_rrf": result.total_rrf,
        "max_severity": result.max_severity,
    }


def _safety_instrumented_function(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.safety_instrumented_function`'s layers, from the reference's arithmetic.

    An optional output the class answers with `+Infinity` is absent from the dump, so the
    capture's `Infinity` row is not compared against it.
    """
    from azoth.process.reference.safety_instrumented_function import (
        safety_instrumented_function as evaluate,
    )

    result = evaluate(
        float(inputs["pfd_average"]),
        float(inputs["claimed_sil"]),
        float(inputs["unmitigated_frequency_per_year"]),
        float(inputs["target_frequency_per_year"]),
        float(inputs["test_interval_hours"]),
        None if inputs.get("lambda_du_per_hour") is None else float(inputs["lambda_du_per_hour"]),
        (
            None
            if inputs.get("spurious_trip_rate_per_year") is None
            else float(inputs["spurious_trip_rate_per_year"])
        ),
        None if inputs.get("mttr_hours") is None else float(inputs["mttr_hours"]),
        (None if inputs.get("base_availability") is None else float(inputs["base_availability"])),
    )
    layers = {
        "mitigated_frequency_per_year": result.mitigated_frequency_per_year,
        "required_pfd": result.required_pfd,
        "required_sil": result.required_sil,
        "max_pfd_for_claimed_sil": result.max_pfd_for_claimed_sil,
        "proof_test_interval_years": result.proof_test_interval_years,
    }
    for name in (
        "risk_reduction_factor",
        "pfd_1oo1",
        "pfd_1oo2",
        "pfd_2oo3",
        "availability_with_spurious_trips",
    ):
        value = getattr(result, name)
        if value is not None:
            layers[name] = float(value)
    return layers


def _lopa(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.lopa`'s layers, from the reference's own arithmetic.

    The layer names and the STS0131 category are not numbers; they are asserted by the
    case and the cross-impl test. A layer's RRF absent at a zero PFD is absent here too.
    """
    from azoth.core.units import quantity
    from azoth.process.reference.lopa import lopa as evaluate

    def optional(name: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), "bar")

    result = evaluate(
        float(inputs["initiating_event_frequency_per_year"]),
        float(inputs["target_frequency_per_year"]),
        float(inputs["mitigated_frequency_per_year"]),
        [dict(row) for row in inputs["layers"]],
        optional("event_pressure"),
        optional("design_pressure"),
        optional("test_pressure"),
    )
    layers: dict[str, float] = {
        "gap_to_target": result.gap_to_target,
        "total_rrf": result.total_rrf,
        "required_additional_sil": result.required_additional_sil,
        "layer_count": float(len(result.layer_contributions)),
    }
    if result.required_additional_rrf is not None:
        layers["required_additional_rrf"] = result.required_additional_rrf
    if result.sts0131_target_frequency_per_year is not None:
        layers["sts0131_target_frequency_per_year"] = result.sts0131_target_frequency_per_year
    for index, row in enumerate(result.layer_contributions):
        layers[f"layer{index}_contribution_per_year"] = row.contribution_per_year
        if row.rrf is not None:
            layers[f"layer{index}_rrf"] = row.rrf
    return layers


def _sil_verification(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.sil_verification`'s layers, from the reference's own arithmetic.

    The issue severities and categories are names; they are asserted by the case and the
    cross-impl test.
    """
    from azoth.process.reference.sil_verification import (
        sil_verification as evaluate,
    )

    result = evaluate(
        float(inputs["pfd_average"]),
        float(inputs["claimed_sil"]),
        str(inputs["architecture"]),
        float(inputs["test_interval_hours"]),
    )
    return {
        "claimed_sil": result.claimed_sil,
        "achieved_sil": result.achieved_sil,
        "pfd_average": result.pfd_average,
        "pfd_upper": result.pfd_upper,
        "hardware_fault_tolerance": result.hardware_fault_tolerance,
        "systematic_capability": result.systematic_capability,
        "diagnostic_coverage": result.diagnostic_coverage,
        "issue_count": result.issue_count,
        "error_count": result.error_count,
        "warning_count": result.warning_count,
    }


def _nog070_sil_determination(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.nog070_sil_determination`'s layers, from the reference's arithmetic."""
    from azoth.process.reference.nog070_sil_determination import (
        nog070_sil_determination as evaluate,
    )

    minimum = inputs.get("minimum_sil")
    result = evaluate(
        str(inputs["sif_type"]),
        float(inputs["pfd_average"]),
        None if minimum is None else float(minimum),
    )
    layers = {
        "catalogue_minimum_sil": result.catalogue_minimum_sil,
        "minimum_sil": result.minimum_sil,
        "achieved_sil": result.achieved_sil,
    }
    if result.required_pfd_reduction_factor is not None:
        layers["required_pfd_reduction_factor"] = result.required_pfd_reduction_factor
    return layers


def _risk_matrix(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.risk_matrix`'s layers, from the reference's own arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.risk_matrix import risk_matrix as evaluate

    price = inputs.get("product_price_per_kg")
    downtime = inputs.get("downtime_cost_per_hour")
    equipment = []
    for row in inputs["equipment"]:
        converted = dict(row)
        converted["production_loss_kg_per_hr"] = quantity(
            float(row["production_loss_kg_per_hr"]), "kg/h"
        )
        equipment.append(converted)
    result = evaluate(
        equipment,
        None if price is None else float(price),
        None if downtime is None else float(downtime),
    )
    layers = {"total_annual_risk_cost": result.total_annual_risk_cost}
    for index, row in enumerate(result.assessments):
        layers[f"assessment{index}_failures_per_year"] = row.failures_per_year
        layers[f"assessment{index}_mtbf_hours"] = row.mtbf_hours
        layers[f"assessment{index}_mttr_hours"] = row.mttr_hours
        layers[f"assessment{index}_expected_downtime_hours_per_year"] = (
            row.expected_downtime_hours_per_year
        )
        layers[f"assessment{index}_probability_level"] = row.probability_level
        layers[f"assessment{index}_production_loss_percent"] = row.production_loss_percent
        layers[f"assessment{index}_production_loss_kg_hr"] = row.production_loss_kg_hr.to(
            "kg/h"
        ).magnitude
        layers[f"assessment{index}_consequence_level"] = row.consequence_level
        layers[f"assessment{index}_risk_score"] = row.risk_score
        layers[f"assessment{index}_cost_per_failure"] = row.cost_per_failure
        layers[f"assessment{index}_annual_risk_cost"] = row.annual_risk_cost
    return layers


def _psv_sizing(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.psv_sizing`'s layers, from the reference's own arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.psv_sizing import psv_sizing as evaluate

    def optional(name: str, unit: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), unit)

    def number(name: str) -> float | None:
        value = inputs.get(name)
        return None if value is None else float(value)

    result = evaluate(
        str(inputs["service"]),
        quantity(float(inputs["set_pressure"]), "Pa"),
        float(inputs["overpressure_fraction"]),
        quantity(float(inputs["back_pressure"]), "Pa"),
        None if inputs.get("is_balanced_bellows") is None else bool(inputs["is_balanced_bellows"]),
        None if inputs.get("has_rupture_disk") is None else bool(inputs["has_rupture_disk"]),
        optional("mass_flow", "kg/s"),
        optional("temperature", "K"),
        optional("molar_mass", "kg/mol"),
        number("compressibility"),
        number("specific_heat_ratio"),
        optional("volume_flow", "m**3/s"),
        optional("liquid_density", "kg/m**3"),
        optional("viscosity", "Pa*s"),
        number("gas_fraction"),
        optional("gas_density", "kg/m**3"),
        number("latent_heat"),
        optional("liquid_heat_capacity", "J/(kg*K)"),
        optional("orifice_area", "m**2"),
        number("capacity_discharge_coefficient"),
    )
    layers: dict[str, float] = {
        "relieving_pressure": result.relieving_pressure.to("Pa").magnitude,
        "back_pressure_fraction": result.back_pressure_fraction,
        "discharge_coefficient": result.discharge_coefficient,
        "required_area": result.required_area.to("m**2").magnitude,
    }
    if result.required_area_in2 is not None:
        layers["required_area_in2"] = result.required_area_in2
    if result.selected_area_in2 is not None:
        layers["selected_area_in2"] = result.selected_area_in2
    if result.back_pressure_correction is not None:
        layers["back_pressure_correction"] = result.back_pressure_correction
    if result.combination_correction is not None:
        layers["combination_correction"] = result.combination_correction
    if result.viscosity_correction is not None:
        layers["viscosity_correction"] = result.viscosity_correction
    if result.mass_flow_rate is not None:
        layers["mass_flow_rate"] = result.mass_flow_rate.to("kg/s").magnitude
    if result.mass_flow_capacity is not None:
        layers["mass_flow_capacity"] = result.mass_flow_capacity.to("kg/s").magnitude
    return layers


def _psv_fire_sizing(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.psv_fire_sizing`'s layers, from the reference's own arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.psv_fire_sizing import psv_fire_sizing as evaluate

    result = evaluate(
        quantity(float(inputs["initial_mass"]), "kg"),
        quantity(float(inputs["initial_pressure"]), "Pa"),
        quantity(float(inputs["set_pressure"]), "Pa"),
        quantity(float(inputs["initial_temperature"]), "K"),
        quantity(float(inputs["fire_heat_input"]), "W"),
        quantity(float(inputs["vessel_volume"]), "m**3"),
        quantity(float(inputs["molar_mass"]), "kg/mol"),
        float(inputs["specific_heat_ratio"]),
        float(inputs["compressibility"]),
        quantity(float(inputs["heat_capacity"]), "J/(kg*K)"),
        quantity(float(inputs["blowdown_time"]), "s"),
        None if inputs.get("blowdown_percent") is None else float(inputs["blowdown_percent"]),
        (
            None
            if inputs.get("orifice_area") is None
            else quantity(float(inputs["orifice_area"]), "m**2")
        ),
        (
            None
            if inputs.get("cv_discharge_coefficient") is None
            else float(inputs["cv_discharge_coefficient"])
        ),
    )
    layers: dict[str, float] = {
        "overpressure_fraction": result.overpressure_fraction,
        "required_area": result.required_area.to("m**2").magnitude,
        "required_area_in2": result.required_area_in2,
        "selected_area_in2": result.selected_area_in2,
        "back_pressure_fraction": result.back_pressure_fraction,
        "discharge_coefficient": result.discharge_coefficient,
        "back_pressure_correction": result.back_pressure_correction,
        "combination_correction": result.combination_correction,
    }
    if result.blowdown_pressure is not None:
        layers["blowdown_pressure"] = result.blowdown_pressure.to("Pa").magnitude
    if result.flow_coefficient is not None:
        layers["flow_coefficient"] = result.flow_coefficient
    return layers


def _blocked_outlet_relief(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.blocked_outlet_relief`'s layers, from the reference's own arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.blocked_outlet_relief import (
        blocked_outlet_relief as evaluate,
    )

    result = evaluate(
        list(inputs["components"]),
        [float(value) for value in inputs["mole_fractions"]],
        quantity(float(inputs["inflow_rate"]), "kg/s"),
        quantity(float(inputs["relief_pressure"]), "Pa"),
        quantity(float(inputs["relief_temperature"]), "K"),
        None if inputs.get("scenario_name") is None else str(inputs["scenario_name"]),
    )
    return {
        "relief_rate_kg_per_s": result.relief_rate.to("kg/s").magnitude,
        "relief_temperature_k": result.relief_temperature.to("K").magnitude,
        "molar_mass_kg_per_mol": result.molar_mass.to("kg/mol").magnitude,
        "compressibility": result.compressibility,
        "specific_heat_ratio": result.specific_heat_ratio,
    }


def _semenov_self_heating(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.semenov_self_heating`'s layers, from the reference's own arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.semenov_self_heating import (
        semenov_self_heating as evaluate,
    )

    result = evaluate(
        quantity(float(inputs["volume"]), "m**3"),
        quantity(float(inputs["surface_area"]), "m**2"),
        quantity(float(inputs["heat_transfer_coefficient"]), "W/(m**2*K)"),
        quantity(float(inputs["activation_energy"]), "J/mol"),
        float(inputs["volumetric_pre_factor"]),
        quantity(float(inputs["ambient_temperature"]), "K"),
        None if inputs.get("marginal_ratio") is None else float(inputs["marginal_ratio"]),
    )
    layers = {
        "psi": result.psi,
        "psi_crit": result.psi_crit,
        "psi_ratio": result.psi_ratio,
        "critical_temperature_rise_k": result.critical_temperature_rise.to("K").magnitude,
    }
    if result.critical_temperature is not None:
        layers["critical_temperature_k"] = result.critical_temperature.to("K").magnitude
    if result.temperature_margin is not None:
        layers["temperature_margin_k"] = result.temperature_margin.to("K").magnitude
    return layers


def _porous_media_self_heating(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.porous_media_self_heating`'s layers, from the reference's arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.porous_media_self_heating import (
        porous_media_self_heating as evaluate,
    )

    result = evaluate(
        str(inputs["geometry"]),
        quantity(float(inputs["characteristic_dimension"]), "m"),
        quantity(float(inputs["effective_conductivity"]), "W/(m*K)"),
        quantity(float(inputs["activation_energy"]), "J/mol"),
        float(inputs["volumetric_pre_factor"]),
        quantity(float(inputs["boundary_temperature"]), "K"),
        None if inputs.get("marginal_ratio") is None else float(inputs["marginal_ratio"]),
        None
        if inputs.get("pipe_insulation_screening") is None
        else bool(inputs["pipe_insulation_screening"]),
    )
    layers = {
        "delta": result.delta,
        "delta_crit": result.delta_crit,
        "delta_ratio": result.delta_ratio,
        "critical_dimension_m": result.critical_dimension.to("m").magnitude,
        "dimension_margin_m": result.dimension_margin.to("m").magnitude,
        "fk_temperature_scale_k": result.fk_temperature_scale.to("K").magnitude,
    }
    if result.critical_temperature is not None:
        layers["critical_temperature_k"] = result.critical_temperature.to("K").magnitude
    if result.temperature_margin is not None:
        layers["temperature_margin_k"] = result.temperature_margin.to("K").magnitude
    return layers


def _self_heating_induction(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.self_heating_induction`'s layers, from the reference's arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.self_heating_induction import (
        self_heating_induction as evaluate,
    )

    def optional(name: str, unit: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), unit)

    result = evaluate(
        str(inputs["geometry"]),
        quantity(float(inputs["characteristic_dimension"]), "m"),
        quantity(float(inputs["effective_conductivity"]), "W/(m*K)"),
        float(inputs["volumetric_heat_capacity"]),
        quantity(float(inputs["activation_energy"]), "J/mol"),
        float(inputs["volumetric_pre_factor"]),
        quantity(float(inputs["boundary_temperature"]), "K"),
        optional("initial_temperature", "K"),
        optional("ignition_rise", "K"),
        optional("max_time", "s"),
        None if inputs.get("node_count") is None else float(inputs["node_count"]),
    )
    layers = {
        "peak_temperature_k": result.peak_temperature.to("K").magnitude,
        "peak_temperature_rise_k": result.peak_temperature_rise.to("K").magnitude,
        "simulated_time_s": result.simulated_time.to("s").magnitude,
        "history_count": float(len(result.history)),
    }
    if result.induction_time is not None:
        layers["induction_time_s"] = result.induction_time.to("s").magnitude
    for index in range(min(3, len(result.history))):
        point = result.history[index]
        layers[f"history{index}_time_s"] = point.time.to("s").magnitude
        layers[f"history{index}_centre_temperature_k"] = point.centre_temperature.to("K").magnitude
        layers[f"history{index}_max_temperature_k"] = point.max_temperature.to("K").magnitude
    last = max(0, len(result.history) - 3)
    for offset, index in enumerate(range(last, len(result.history))):
        point = result.history[index]
        layers[f"history_last{offset}_time_s"] = point.time.to("s").magnitude
        layers[f"history_last{offset}_centre_temperature_k"] = point.centre_temperature.to(
            "K"
        ).magnitude
        layers[f"history_last{offset}_max_temperature_k"] = point.max_temperature.to("K").magnitude
    return layers


def _basket_test_regression(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.basket_test_regression`'s layers, from the reference's arithmetic."""
    from azoth import _models_gen
    from azoth.core.units import quantity
    from azoth.process.reference.basket_test_regression import (
        basket_test_regression as evaluate,
    )

    spec = _models_gen.model("process.basket_test_regression")
    fields = spec["inputs"]["points"]["fields"]
    prepared = []
    for row in inputs["points"]:
        converted: dict[str, Any] = {}
        for name, declaration in fields.items():
            value = row.get(name)
            if value is None:
                converted[name] = None
            elif declaration.get("type") in ("enum", "string", "boolean"):
                converted[name] = value
            elif declaration.get("unit") == "dimensionless":
                converted[name] = float(value)
            else:
                converted[name] = quantity(float(value), str(declaration["unit"]))
        prepared.append(converted)

    conductivity = inputs.get("effective_conductivity")
    result = evaluate(
        prepared,
        None if conductivity is None else quantity(float(conductivity), "W/(m*K)"),
    )
    layers = {
        "point_count": result.point_count,
        "slope_k": result.slope,
        "intercept": result.intercept,
        "activation_energy_j_per_mol": result.activation_energy.to("J/mol").magnitude,
    }
    if result.r_squared is not None:
        layers["r_squared"] = result.r_squared
    if result.volumetric_pre_factor is not None:
        layers["volumetric_pre_factor_w_per_m3"] = result.volumetric_pre_factor
    if result.effective_conductivity is not None:
        layers["effective_conductivity_w_per_m_k"] = result.effective_conductivity.to(
            "W/(m*K)"
        ).magnitude
    return layers


def _hazop_template(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.hazop_template`'s layers, from the reference's own arithmetic."""
    from azoth.process.reference.hazop_template import hazop_template as evaluate

    equipment = inputs.get("equipment_type")
    result = evaluate(
        list(inputs["grid_parameters"]),
        None if equipment is None else str(equipment),
    )
    # The rows are strings and are compared by `test_model_cases`; the count is the one
    # numeric interior the capture prints.
    return {"deviation_count": result.deviation_count}


def _fmea_worksheet(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.fmea_worksheet`'s layers, from the reference's own arithmetic."""
    from azoth.process.reference.fmea_worksheet import fmea_worksheet as evaluate

    result = evaluate(list(inputs["entries"]), float(inputs["rpn_threshold"]))
    layers = {
        "entry_count": result.entry_count,
        "above_count": float(len(result.entries_above_threshold)),
    }
    if result.highest_rpn is not None:
        layers["highest_rpn"] = result.highest_rpn
    for index, row in enumerate(result.ranked_entries):
        layers[f"ranked{index}_rpn"] = row.rpn
    for index, row in enumerate(result.entries_above_threshold):
        layers[f"above{index}_rpn"] = row.rpn
    return layers


def _hazop_consequence_population(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.hazop_consequence_population`'s layers, from the reference's arithmetic."""
    from azoth.process.reference.hazop_consequence_population import (
        hazop_consequence_population as evaluate,
    )

    equipment = inputs.get("equipment_type")
    result = evaluate(
        list(inputs["deviations"]),
        None if equipment is None else str(equipment),
    )
    return {
        "deviation_count": float(len(result.populated)),
        "mapped_count": result.mapped_count,
    }


def _check_valve_leak_relief(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.check_valve_leak_relief`'s layers, from the reference's own arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.check_valve_leak_relief import (
        check_valve_leak_relief as evaluate,
    )

    def optional(name: str, unit: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), unit)

    result = evaluate(
        list(inputs["components"]),
        [float(value) for value in inputs["mole_fractions"]],
        quantity(float(inputs["nominal_diameter"]), "m"),
        quantity(float(inputs["upstream_pressure"]), "Pa"),
        None if inputs.get("leak_area_fraction") is None else float(inputs["leak_area_fraction"]),
        optional("upstream_temperature", "K"),
        optional("downstream_pressure", "Pa"),
        None
        if inputs.get("discharge_coefficient") is None
        else float(inputs["discharge_coefficient"]),
        None if inputs.get("scenario_name") is None else str(inputs["scenario_name"]),
    )
    return {
        "relief_rate_kg_per_s": result.relief_rate.to("kg/s").magnitude,
        "relief_temperature_k": result.relief_temperature.to("K").magnitude,
        "molar_mass_kg_per_mol": result.molar_mass.to("kg/mol").magnitude,
        "compressibility": result.compressibility,
        "specific_heat_ratio": result.specific_heat_ratio,
    }


def _control_valve_failure_relief(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.control_valve_failure_relief`'s layers, from the reference's arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.control_valve_failure_relief import (
        control_valve_failure_relief as evaluate,
    )

    def optional(name: str, unit: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), unit)

    result = evaluate(
        list(inputs["components"]),
        [float(value) for value in inputs["mole_fractions"]],
        quantity(float(inputs["upstream_pressure"]), "Pa"),
        optional("upstream_temperature", "K"),
        optional("downstream_pressure", "Pa"),
        optional("effective_area", "m**2"),
        None if inputs.get("cv") is None else float(inputs["cv"]),
        None if inputs.get("cv_erosion_margin") is None else float(inputs["cv_erosion_margin"]),
        None
        if inputs.get("discharge_coefficient") is None
        else float(inputs["discharge_coefficient"]),
        None if inputs.get("scenario_name") is None else str(inputs["scenario_name"]),
    )
    return {
        "relief_rate_kg_per_s": result.relief_rate.to("kg/s").magnitude,
        "relief_temperature_k": result.relief_temperature.to("K").magnitude,
        "molar_mass_kg_per_mol": result.molar_mass.to("kg/mol").magnitude,
        "compressibility": result.compressibility,
        "specific_heat_ratio": result.specific_heat_ratio,
    }


def _tube_rupture_relief(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.tube_rupture_relief`'s layers, from the reference's own arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.tube_rupture_relief import (
        tube_rupture_relief as evaluate,
    )

    def optional(name: str, unit: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), unit)

    result = evaluate(
        list(inputs["components"]),
        [float(value) for value in inputs["mole_fractions"]],
        quantity(float(inputs["tube_inner_diameter"]), "m"),
        quantity(float(inputs["high_pressure"]), "Pa"),
        None if inputs.get("number_of_open_ends") is None else float(inputs["number_of_open_ends"]),
        optional("high_temperature", "K"),
        optional("low_pressure", "Pa"),
        None
        if inputs.get("discharge_coefficient") is None
        else float(inputs["discharge_coefficient"]),
        None if inputs.get("scenario_name") is None else str(inputs["scenario_name"]),
    )
    return {
        "relief_rate_kg_per_s": result.relief_rate.to("kg/s").magnitude,
        "relief_temperature_k": result.relief_temperature.to("K").magnitude,
        "molar_mass_kg_per_mol": result.molar_mass.to("kg/mol").magnitude,
        "compressibility": result.compressibility,
        "specific_heat_ratio": result.specific_heat_ratio,
    }


def _overpressure_protection_study(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.overpressure_protection_study`'s layers, from the reference's arithmetic."""
    from azoth import _models_gen
    from azoth.core.units import quantity
    from azoth.process.reference.overpressure_protection_study import (
        overpressure_protection_study as evaluate,
    )

    spec = _models_gen.model("process.overpressure_protection_study")
    fields = spec["inputs"]["scenarios"]["fields"]
    prepared = []
    for row in inputs["scenarios"]:
        converted: dict[str, Any] = {}
        for name, declaration in fields.items():
            value = row.get(name)
            if value is None:
                converted[name] = None
            elif declaration.get("type") in ("enum", "string", "boolean"):
                converted[name] = value
            elif declaration.get("unit") == "dimensionless":
                converted[name] = float(value)
            else:
                converted[name] = quantity(float(value), str(declaration["unit"]))
        prepared.append(converted)

    def optional(name: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), "Pa")

    result = evaluate(
        quantity(float(inputs["mawp"]), "Pa"),
        prepared,
        optional("relief_set_pressure"),
        optional("back_pressure"),
        None if inputs.get("multiple_devices") is None else bool(inputs["multiple_devices"]),
    )
    layers: dict[str, float] = {}
    if result.governing_relief_rate is not None:
        layers["governing_relief_rate_kg_per_s"] = result.governing_relief_rate.to("kg/s").magnitude
    if result.required_area is not None:
        layers["required_area_m2"] = result.required_area.to("m**2").magnitude
    if result.required_area_in2 is not None:
        layers["required_area_in2"] = result.required_area_in2
    if result.selected_area_in2 is not None:
        layers["selected_area_in2"] = result.selected_area_in2
    if result.selected_orifice_capacity is not None:
        layers["selected_orifice_capacity_kg_per_s"] = result.selected_orifice_capacity.to(
            "kg/s"
        ).magnitude
    if result.acceptance_peak_pressure is not None:
        layers["acceptance_peak_pressure_bara"] = result.acceptance_peak_pressure.to(
            "bar"
        ).magnitude
    if result.acceptance_allowable_pressure is not None:
        layers["acceptance_allowable_pressure_bara"] = result.acceptance_allowable_pressure.to(
            "bar"
        ).magnitude
    if result.acceptance_accumulation_factor is not None:
        layers["acceptance_accumulation_factor"] = result.acceptance_accumulation_factor
    if result.acceptance_margin is not None:
        layers["acceptance_margin_bara"] = result.acceptance_margin.to("bar").magnitude
    return layers


def _fire_case_relief(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.fire_case_relief`'s layers, from the reference's own arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.fire_case_relief import fire_case_relief as evaluate

    result = evaluate(
        bool(inputs["has_drainage"]),
        bool(inputs["has_fire_fighting"]),
        None
        if inputs.get("wetted_area") is None
        else quantity(float(inputs["wetted_area"]), "m**2"),
        (
            None
            if inputs.get("vessel_diameter") is None
            else quantity(float(inputs["vessel_diameter"]), "m")
        ),
        (
            None
            if inputs.get("wetted_height") is None
            else quantity(float(inputs["wetted_height"]), "m")
        ),
        None if inputs.get("latent_heat") is None else float(inputs["latent_heat"]),
        (
            None
            if inputs.get("relieving_temperature") is None
            else quantity(float(inputs["relieving_temperature"]), "K")
        ),
    )
    layers: dict[str, float] = {
        "latent_heat": result.latent_heat,
        "relief_rate": result.relief_rate.to("kg/s").magnitude,
        "relief_rate_kg_per_hr": result.relief_rate_kg_per_hr.to("kg/h").magnitude,
    }
    if result.relieving_temperature is not None:
        layers["relieving_temperature"] = result.relieving_temperature.to("K").magnitude
    return layers


def _overpressure_acceptance(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.overpressure_acceptance`'s layers, from the reference's own arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.overpressure_acceptance import (
        overpressure_acceptance as evaluate,
    )

    result = evaluate(
        quantity(float(inputs["peak_pressure"]), "Pa"),
        quantity(float(inputs["mawp"]), "Pa"),
        str(inputs["cause"]),
        bool(inputs["multiple_devices"]),
    )
    return {
        "accumulation_factor": result.accumulation_factor,
        "allowable_pressure": result.allowable_pressure.to("Pa").magnitude,
        "margin": result.margin.to("Pa").magnitude,
    }


def _pipeline_pressure_protection(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.pipeline_pressure_protection`'s layers, from the reference's arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.pipeline_pressure_protection import (
        pipeline_pressure_protection as evaluate,
    )

    result = evaluate(
        quantity(float(inputs["max_source_pressure"]), "bar"),
        quantity(float(inputs["design_pressure"]), "bar"),
        float(inputs["incidental_factor"]),
        quantity(float(inputs["barrier1_setpoint"]), "bar"),
        quantity(float(inputs["barrier2_setpoint"]), "bar"),
    )
    return {
        "max_incidental_pressure": result.max_incidental_pressure.to("bar").magnitude,
        "design_margin": result.design_margin.to("bar").magnitude,
    }


def _relief_disposal_network(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.relief_disposal_network`'s layers, from the reference's arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.relief_disposal_network import (
        relief_disposal_network as evaluate,
    )

    rows = []
    for row in inputs["contributions"]:
        converted = dict(row)
        converted["mass_flow"] = quantity(float(row["mass_flow"]), "kg/s")
        rows.append(converted)
    result = evaluate(rows)
    layers: dict[str, float] = {
        "total_simultaneous": result.total_simultaneous.to("kg/s").magnitude,
        "total_simultaneous_kg_hr": result.total_simultaneous_kg_hr.to("kg/h").magnitude,
        "peak_single": result.peak_single.to("kg/s").magnitude,
    }
    for index, row in enumerate(result.contributions):
        layers[f"contribution{index}_mass_flow"] = row.mass_flow.to("kg/s").magnitude
        layers[f"contribution{index}_mass_flow_kg_hr"] = row.mass_flow_kg_hr.to("kg/h").magnitude
    return layers


def _tr3001_compliance(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.tr3001_compliance`'s layers, from the reference's own arithmetic."""
    from azoth.core.units import quantity
    from azoth.process.reference.tr3001_compliance import tr3001_compliance as evaluate

    rows = []
    for row in inputs["scenarios"]:
        converted = dict(row)
        converted["relief_rate"] = quantity(float(row["relief_rate"]), "kg/s")
        rows.append(converted)
    result = evaluate(
        rows,
        bool(inputs["capacity_adequate"]),
        None if inputs.get("required_area_in2") is None else float(inputs["required_area_in2"]),
        (
            None
            if inputs.get("acceptance_accepted") is None
            else bool(inputs["acceptance_accepted"])
        ),
        (
            None
            if inputs.get("accumulation_fraction") is None
            else float(inputs["accumulation_fraction"])
        ),
    )
    return {
        "pass_count": result.pass_count,
        "review_count": result.review_count,
        "fail_count": result.fail_count,
    }


def _blowdown_pressure_profile(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.blowdown_pressure_profile`'s two pressures, through the reference."""
    from azoth.core.units import quantity
    from azoth.process.reference.blowdown_pressure_profile import _evaluate

    points: list[dict[str, object]] = [
        {
            "time": quantity(float(row["time"]), "s"),
            "pressure": quantity(float(row["pressure"]), "bar"),
        }
        for row in inputs["points"]
    ]
    offset = inputs.get("gauge_offset")
    solution, _ = _evaluate(
        points,
        quantity(float(inputs["time"]), "s"),
        inputs.get("interpolation_mode"),
        None if offset is None else quantity(float(offset), "bar"),
    )
    return {
        "pressure_absolute_bara": solution.pressure_absolute / 1.0e5,
        "pressure_gauge_barg": solution.pressure_gauge / 1.0e5,
    }


def _pipe_fire_rupture_material(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.pipe_fire_rupture_material`'s curves, through the reference."""
    from azoth.core.units import quantity
    from azoth.process.reference.pipe_fire_rupture_material import _evaluate

    curve = inputs.get("curve")
    points: list[dict[str, object]] | None = None
    if curve is not None:
        points = [
            {
                "temperature": quantity(float(row["temperature"]), "K"),
                "heat_capacity": quantity(float(row["heat_capacity"]), "J/(kg*K)"),
                "thermal_conductivity": quantity(float(row["thermal_conductivity"]), "W/(m*K)"),
                "ultimate_tensile_strength": quantity(
                    float(row["ultimate_tensile_strength"]), "MPa"
                ),
                "strain_effect_factor": float(row["strain_effect_factor"]),
                "rupture_strain_limit": float(row["rupture_strain_limit"]),
            }
            for row in curve
        ]

    def optional(name: str, unit: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), unit)

    solution, _ = _evaluate(
        str(inputs["material_name"]),
        quantity(float(inputs["evaluation_temperature"]), "K"),
        quantity(float(inputs["evaluation_stress"]), "MPa"),
        points,
        optional("density", "kg/m**3"),
        None if inputs.get("sellars_tegart_a") is None else float(inputs["sellars_tegart_a"]),
        None
        if inputs.get("sellars_tegart_alpha") is None
        else float(inputs["sellars_tegart_alpha"]),
        None if inputs.get("sellars_tegart_n") is None else float(inputs["sellars_tegart_n"]),
        optional("activation_temperature", "K"),
        optional("correction_strain_rate", "1/s"),
        optional("correction_limit_temperature", "K"),
    )
    return {
        "density_kg_m3": solution.density,
        "heat_capacity_j_kg_k": solution.heat_capacity,
        "thermal_conductivity_w_m_k": solution.thermal_conductivity,
        "ultimate_tensile_strength_mpa": solution.ultimate_tensile_strength / 1.0e6,
        "strain_effect_factor": solution.strain_effect_factor,
        "rupture_strain_limit": solution.rupture_strain_limit,
        "temperature_correction_factor": solution.temperature_correction_factor,
        "strain_rate_per_minute": solution.strain_rate * 60.0,
    }


def _pipe_fire_rupture(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.pipe_fire_rupture`'s march, through the reference's own evaluator."""
    from azoth.core.units import quantity
    from azoth.process.reference.pipe_fire_rupture import _evaluate

    def optional(name: str, unit: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), unit)

    profile: list[dict[str, object]] = [
        {
            "time": quantity(float(row["time"]), "s"),
            "pressure": quantity(float(row["pressure"]), "bar"),
        }
        for row in inputs["pressure_profile"]
    ]
    solution, _ = _evaluate(
        quantity(float(inputs["outside_diameter"]), "mm"),
        quantity(float(inputs["nominal_wall_thickness"]), "mm"),
        str(inputs["material_name"]),
        str(inputs["fire_scenario"]),
        profile,
        optional("corrosion_allowance", "mm"),
        None
        if inputs.get("wall_thickness_undertolerance") is None
        else float(inputs["wall_thickness_undertolerance"]),
        optional("exposed_length", "m"),
        optional("weight_stress", "MPa"),
        optional("fluid_density", "kg/m**3"),
        optional("fluid_heat_capacity", "J/(kg*K)"),
        optional("gas_molecular_weight", "kg/mol"),
        optional("initial_temperature", "K"),
        optional("fire_temperature", "K"),
        optional("gas_temperature", "K"),
        optional("convective_heat_transfer_coefficient", "W/(m**2*K)"),
        None if inputs.get("fire_emissivity") is None else float(inputs["fire_emissivity"]),
        None if inputs.get("metal_emissivity") is None else float(inputs["metal_emissivity"]),
        None if inputs.get("metal_absorptivity") is None else float(inputs["metal_absorptivity"]),
        None
        if inputs.get("passive_protection_heat_flux_factor") is None
        else float(inputs["passive_protection_heat_flux_factor"]),
        optional("gauge_offset", "bar"),
        inputs.get("interpolation_mode"),
        optional("time_step", "s"),
        optional("max_time", "s"),
        None
        if inputs.get("spreadsheet_gas_thermal_mass") is None
        else bool(inputs["spreadsheet_gas_thermal_mass"]),
    )
    dumped: dict[str, float] = {
        "point_count": float(len(solution.history)),
    }
    if solution.rupture_time is not None:
        dumped.update(
            {
                "rupture_time_s": solution.rupture_time,
                "rupture_pressure_barg": solution.rupture_pressure or 0.0,
                "rupture_mean_wall_temperature_c": (solution.rupture_mean_wall_temperature or 0.0)
                - 273.15,
                "rupture_outer_surface_temperature_c": (
                    solution.rupture_outer_surface_temperature or 0.0
                )
                - 273.15,
                "rupture_accumulated_strain": solution.rupture_accumulated_strain or 0.0,
                "rupture_strain_limit": solution.rupture_strain_limit or 0.0,
                "long_pipe_gas_two_sides_kg_s": solution.long_pipe_gas_two_sides or 0.0,
                "long_pipe_gas_one_side_kg_s": solution.long_pipe_gas_one_side or 0.0,
                "short_pipe_gas_one_side_kg_s": solution.short_pipe_gas_one_side or 0.0,
                "liquid_one_side_kg_s": solution.liquid_one_side or 0.0,
                "release_gas_temperature_c": (solution.release_gas_temperature or 0.0) - 273.15,
            }
        )
    for index, row in enumerate(solution.history):
        dumped[f"history{index}_time_s"] = row[0]
        dumped[f"history{index}_pressure_barg"] = row[1]
        dumped[f"history{index}_mean_wall_temperature_c"] = row[2] - 273.15
        dumped[f"history{index}_outer_surface_temperature_c"] = row[3] - 273.15
        dumped[f"history{index}_heat_flux_kw_m2"] = row[4] / 1000.0
        dumped[f"history{index}_von_mises_stress_mpa"] = row[5]
        dumped[f"history{index}_strain_rate_per_minute"] = row[6]
        dumped[f"history{index}_accumulated_strain"] = row[7]
        dumped[f"history{index}_strain_limit"] = row[8]
    return dumped


def _vessel_rupture(inputs: Mapping[str, Any]) -> dict[str, float]:
    """`process.vessel_rupture`'s crossover, through the reference's own evaluator."""
    from azoth.core.units import quantity
    from azoth.process.reference.vessel_rupture import _evaluate

    history: list[dict[str, object]] = [
        {
            "time": quantity(float(row["time"]), "s"),
            "internal_pressure": quantity(float(row["internal_pressure"]), "Pa"),
            "metal_temperature": quantity(float(row["metal_temperature"]), "K"),
        }
        for row in inputs["history"]
    ]

    def optional(name: str, unit: str) -> Any:
        value = inputs.get(name)
        return None if value is None else quantity(float(value), unit)

    solution, _ = _evaluate(
        quantity(float(inputs["inner_radius"]), "m"),
        quantity(float(inputs["wall_thickness"]), "m"),
        str(inputs["material"]),
        history,
        optional("yield_strength", "MPa"),
        optional("tensile_strength", "MPa"),
        None if inputs.get("grade") is None else str(inputs["grade"]),
        None
        if inputs.get("tensile_strength_factor") is None
        else float(inputs["tensile_strength_factor"]),
    )
    dumped: dict[str, float] = {
        "point_count": float(len(solution.history)),
        "min_margin_pa": solution.min_margin,
        "min_margin_time_s": solution.min_margin_time,
        "ambient_yield_strength_mpa": solution.ambient_yield_strength / 1.0e6,
        "ambient_tensile_strength_mpa": solution.ambient_tensile_strength / 1.0e6,
    }
    if solution.ruptured:
        dumped.update(
            {
                "rupture_time_s": solution.rupture_time or 0.0,
                "rupture_stress_pa": solution.rupture_stress or 0.0,
                "rupture_allowable_stress_pa": solution.rupture_allowable_stress or 0.0,
                "rupture_metal_temperature_k": solution.rupture_metal_temperature or 0.0,
            }
        )
    for index, row in enumerate(solution.history):
        dumped[f"history{index}_time_s"] = row[0]
        dumped[f"history{index}_von_mises_stress_pa"] = row[1]
        dumped[f"history{index}_allowable_stress_pa"] = row[2]
        dumped[f"history{index}_margin_pa"] = row[3]
    return dumped


DUMPERS: dict[str, Dumper] = {
    "process.separator": _separator,
    "process.hem_release": _hem_release,
    "process.leak_blowdown": _leak_blowdown,
    "process.hazardous_area": _hazardous_area,
    "process.toxic_library": _toxic_library,
    "process.blowdown_pressure_profile": _blowdown_pressure_profile,
    "process.pipe_fire_rupture_material": _pipe_fire_rupture_material,
    "process.pipe_fire_rupture": _pipe_fire_rupture,
    "process.vessel_rupture": _vessel_rupture,
    "process.heavy_gas_dispersion": _heavy_gas_dispersion,
    "process.gaussian_plume": _gaussian_plume,
    "process.gas_dispersion": _gas_dispersion,
    "process.bleve": _bleve,
    "process.consequence_analysis": _consequence_analysis,
    "process.jet_fire": _jet_fire,
    "process.pfp_demand": _pfp_demand,
    "process.blocked_outlet_relief": _blocked_outlet_relief,
    "process.semenov_self_heating": _semenov_self_heating,
    "process.porous_media_self_heating": _porous_media_self_heating,
    "process.self_heating_induction": _self_heating_induction,
    "process.basket_test_regression": _basket_test_regression,
    "process.hazop_template": _hazop_template,
    "process.fmea_worksheet": _fmea_worksheet,
    "process.hazop_consequence_population": _hazop_consequence_population,
    "process.check_valve_leak_relief": _check_valve_leak_relief,
    "process.control_valve_failure_relief": _control_valve_failure_relief,
    "process.tube_rupture_relief": _tube_rupture_relief,
    "process.overpressure_protection_study": _overpressure_protection_study,
    "process.fire_case_relief": _fire_case_relief,
    "process.overpressure_acceptance": _overpressure_acceptance,
    "process.pipeline_pressure_protection": _pipeline_pressure_protection,
    "process.relief_disposal_network": _relief_disposal_network,
    "process.tr3001_compliance": _tr3001_compliance,
    "process.fire_water_demand": _fire_water_demand,
    "process.deluge_nozzle_layout": _deluge_nozzle_layout,
    "process.fire_monitor_coverage": _fire_monitor_coverage,
    "process.fire_water_coverage_assessment": _fire_water_coverage_assessment,
    "process.active_fire_protection_screening": _active_fire_protection_screening,
    "process.api537_flare_flame_model": _api537_flare_flame_model,
    "process.fault_tree": _fault_tree,
    "process.bow_tie": _bow_tie,
    "process.safety_instrumented_function": _safety_instrumented_function,
    "process.lopa": _lopa,
    "process.sil_verification": _sil_verification,
    "process.nog070_sil_determination": _nog070_sil_determination,
    "process.risk_matrix": _risk_matrix,
    "process.psv_fire_sizing": _psv_fire_sizing,
    "process.psv_sizing": _psv_sizing,
    "process.pool_fire": _pool_fire,
    "process.probit": _probit,
    "process.vce": _vce,
    "process.rate_based_packed_column": _rate_based_packed_column,
    "process.absorption_column": _absorption_column,
    "process.packed_column": _packed_column,
    "process.stripping_column": _stripping_column,
    "process.component_splitter": _component_splitter,
    "process.distillation_column": _distillation_column,
    "process.expander": _expander,
    "process.manifold": _manifold,
    "process.pipe": _pipe,
    "process.pump": _pump,
    "process.splitter": _splitter,
    "process.heater": _heater,
    "process.mixer": _mixer,
    "process.gibbs_reactor": _gibbs_reactor,
    "process.plug_flow_reactor": _plug_flow_reactor,
    "process.compressor": _compressor,
    "process.anti_surge_recycle": _anti_surge_recycle,
    "process.heat_exchanger": _heat_exchanger,
    "process.shortcut_distillation_column": _shortcut_distillation_column,
}

#: The process models whose kernel forms **nothing the port records do not carry**, and why.
#:
#: The harness exists to localise a wrong answer, and localising needs a layer between the
#: input and the output. Some kernels do not have one - their whole arithmetic *is* the
#: outlet record, so every number they form is on a port and the validation case already
#: holds it to the same capture. Declaring them here rather than leaving them absent is the
#: rule the palette follows too: nothing is silently missing, and a new process model has to
#: say which side of this it is on.
#:
#: **This is not "the model is too simple to check".** Each entry names the arithmetic and
#: what would have to exist for there to be a layer.
NO_INTERIOR: dict[str, str] = {
    "process.event_tree": (
        "the kernel is the class's own ``evaluate``: a full 2^n enumeration over the branch "
        "probabilities the caller states, with no composed model beneath it. The case set is "
        "the oracle and ``process_event_tree.tsv`` is its evidence"
    ),
    "process.throttling_valve": (
        "the kernel is one ``Stream::from_ph`` - the outlet pressure the valve applied, and "
        "the flash it runs at the inlet's enthalpy. The applied pressure *is* the outlet "
        "record, and `getDeltaPressure` and `getEntropyProduction` are both differences of "
        "the two records' own fields, so nothing is left over. A layer would need the valve "
        "to size itself, which is the `MechanicalDesign.calcValveSize()` path the palette "
        "entry's `valve_opening` was withdrawn for"
    ),
    "process.cooler": (
        "the same kernel as ``process.heater`` and therefore the same answer: `Cooler` "
        "overrides `runTransient` and some getters and not `run`, and the probe's two "
        "captures are byte-identical. A dumper here would be a second dump of the heater's "
        "arithmetic under another name, which is the thing this dict exists to refuse"
    ),
    "process.gas_scrubber": (
        "the same kernel as ``process.separator`` and therefore the same answer: `GasScrubber` "
        "does not override `run`, so the two entries' captures carry the same rows. A dumper "
        "here would be a second dump of the separator's arithmetic under another name, which "
        "is the thing this dict exists to refuse - and the class's own arithmetic, the "
        "Souders-Brown capacity metric, is mechanical design that no port field reaches"
    ),
    "process.filter": (
        "the kernel is one ``enthalpy_at`` at the pressure the drop leaves, and the temperature "
        "is the inlet's - so the outlet record is that state in full. The capture's two extra "
        "lines are the applied drop, which the model reports as ``applied_drop``, and "
        "``Cv = sqrt(dP) / massFlow``, which the class computes from the two states and this "
        "port leaves out on purpose: it is a function of the applied drop and the inlet's mass "
        "flow, both of which the result already carries, and its unit is one this library has "
        "no dimension for. **The capture's third row is uncased rather than dumped**: the "
        "clamp lands the outlet at a microbar, where this library's cubic refuses to converge "
        "and NeqSim's extrapolates - so there is no layer to compare, and that is the finding. "
        "**The loaded state does not add an interior either**: the loading fraction, "
        "breakthrough, the bypass and the replacement flag are scalars over the flashed "
        "record, and `process_filter_loading.tsv` cases them rather than dumping them here"
    ),
    "process.stirred_tank_reactor": (
        "the kernel forms the reacted composition, the outlet pressure and the flashed state, "
        "and **every one of them is on the record** - the composition is the product's ``z``, the "
        "pressure its ``P``, the state its ``T`` and ``h``. The extent is the one intermediate "
        "the record does not carry, and it is not hidden: the composition *is* the extent, "
        "applied. The duty the isothermal branch reports is a difference of the two records' "
        "enthalpies over the flow, which is an output rather than a layer"
    ),
    "process.flare": (
        "the kernel is a *pass-through*: `run` clones the inlet into the outlet, so the whole "
        "record is the answer and there is no intermediate between the two ports. Its reported "
        "numbers - the duty, the CO2 emission, and the radiation, dispersion and capacity "
        "surface - are outputs rather than layers, and the capture prints them beside the "
        "record exactly as the model does; each is a closed form over the record itself"
    ),
    "process.ejector": (
        "the kernel forms the mixing pressure, four velocities, two efficiencies' enthalpy "
        "drops and the joined fluid's static enthalpy - and the outlet record carries the "
        "answer to all of them, because the last flash *is* the state the diffuser lands on. "
        "**The one intermediate not on the record is the class's own instrumentation**: the "
        "entrainment, compression and expansion ratios and the area ratio are ratios of the "
        "record's fields (`n`, `P` and `P`), and the Mach numbers need a speed of sound the "
        "port would have to model separately rather than derive. A dumper would be dumping "
        "arithmetic the capture's own ratios already state"
    ),
    "process.three_phase_separator": (
        "the kernel forms the multiphase flash's three fractions, the three compositions and "
        "the moles the entrainment moves, and every one of them is on the three outlet "
        "records - a phase's share is its outlet's flow over the feed's, and its composition "
        "is that outlet's ``z``. The **labels** are the one intermediate the records do not "
        "carry, and a swap is not a hidden layer: the phases are named by "
        "`PhaseEos.init`'s rule and routed to the outlet that names them, so an oil sent to "
        "the water outlet is a wrong ``z`` in the capture rather than a state beside it. And "
        "the flash itself is held by the first row, which states the same feed with no "
        "entrainment - the other five are that answer with material moved, and where it moved "
        "to is the difference between their flows and its"
    ),
    "process.tank": (
        "the same kernel as ``process.separator`` at zero drop, zero entrainment and no "
        "heat input, and the capture's two-phase row is the separator's first row to the "
        "last digit. A dumper here would be a second dump of the separator's arithmetic "
        "under another name, which is the thing this dict exists to refuse. **The capture's "
        "other rows are the reason this is not a case rather than a dump**: two of them are "
        "single-phase, where `run`'s absent-oil branch writes `gasOutStream` instead of the "
        "liquid one and the two outlets come back swapped against `Separator`'s on the same "
        "feed - a construction-time state a kernel that is a pure function of its inlets "
        "has no way to reproduce"
    ),
    "process.separator_capacity_assessment": (
        "the kernel folds the same run and capacity arithmetic ``process.separator`` carries "
        "into the snapshot, and every number in the snapshot is on the model's own record - "
        "the status, the diagnostic and the two basis strings included. The capacity layers "
        "are dumped under ``process.separator`` already, against the same numbers, so a "
        "dumper here would be a second dump of that arithmetic under another name, which is "
        "the thing this dict exists to refuse"
    ),
    "process.separator_performance": (
        "the calculator's every number is a top-level output - the six fractions, the four "
        "efficiencies and the cut diameter - and the droplet size classes it integrates over "
        "are derived inside the kernel rather than carried on the record, so there is no "
        "interior the capture could name"
    ),
    "process.turbo_expander_compressor": (
        "the kernel forms the speed, both powers, both efficiencies, the compressor's "
        "polytropic head, the two ratios and the IGV opening, and **every one of them is on "
        "the model's own record** beside the two outlet streams - the model function copies "
        "``TurboOutcome`` field for field. The case's expected block already asserts all of "
        "them against the same capture, so a dumper would add a key-by-key comparison of "
        "numbers the case compares and no layer behind them"
    ),
    "process.map_turbo_expander_compressor": (
        "the kernel forms both outlet states, the shaft speed, the available and consumed "
        "powers, the residual and the status, and **every one of them is on the model's own "
        "record** beside the two outlet streams - the model function copies the outcome field "
        "for field. The case's expected block already asserts them against the same capture, "
        "so a dumper would compare numbers the case compares and no layer behind them. **The "
        "one intermediate the record does not carry is the bisection's own trial powers**, "
        "which are `process.compressor`'s chart arithmetic on the same map - held by that id's "
        "dumper and its Khader cases"
    ),
}


@dataclass(frozen=True)
class Divergence:
    """A layer the port **deliberately** does not reproduce, and how far from it it is.

    **Sometimes the layer that moved is supposed to.** `Splitter.run` does not conserve
    enthalpy: it clones the inlet fluid, zeroes it with `init(0)` and adds each component
    back phase by phase, so its outlets weight to `2.3` times what came in. A split changes
    no state, so the port answers with the feed's enthalpy - and a comparison against the
    capture's number can only ever read as a failure, at any tolerance.

    Reading it as a failure would be the wrong answer, and reading it as nothing would let
    the port drift. So the divergence is **declared and bounded**: the capture's value is
    asserted to be at least `at_least` times the port's. NeqSim repairing `Splitter.run`, or
    the port starting to agree, moves the ratio out of that band and fails - which is what
    makes this a measurement rather than a comment that quietly stops being true. The same
    shape as `crates/azoth-process/tests/stream.rs`'s
    `the_heavy_oil_viscosity_is_still_wrong_for_water`.
    """

    key: str
    #: The least the capture's value, divided by the port's, is allowed to be. A bound
    #: rather than an equality: the ratio is a ratio of two flashes' arithmetic, and the
    #: point is which of them it is near, not the digits.
    at_least: float
    #: Why the two are apart, in the shape the spec's `source` states it.
    reason: str


@dataclass(frozen=True)
class LayerCase:
    """One capture block, and the model case it is the oracle for."""

    model: str
    case: str
    capture: str
    #: Which block of the capture, counted from zero in the order the probe printed them.
    #: Positional because that is the convention the `eos` cases already use, and because a
    #: probe that named its own case ids would couple the oracle to the spec tree.
    block: int
    #: The one row the probe prints in that block that identifies it, as `(key, value)`.
    #: Checked rather than documented: the pairing is positional, so a probe row inserted
    #: or reordered would otherwise pair a dump against another state's numbers. Name the
    #: block's [`LABEL_KEY`] where there is one - a label is an identity, a state is a
    #: thing the comparison is already testing.
    identified_by: tuple[str, str]
    #: Layers compared on an **absolute** bound, with the bound, because the oracle is zero
    #: there by construction and a ratio divides by the vanishing thing.
    #:
    #: The same rule `python/tests/_helpers.py::_assert_diagnostic` applies to a solver
    #: residual, and for the same reason: a reversible pump's entropy production is zero by
    #: definition, so the capture has `-6.5780714209040525e-15` kJ/(mol*K) for the row at an
    #: efficiency of one and azoth has `9.18e-12`, which is 1400 times it and also zero. No
    #: relative tolerance can be met there, at any value.
    diagnostic: tuple[tuple[str, float], ...] = ()
    #: Layers held to a declared [`Divergence`] instead of to agreement.
    divergence: tuple[Divergence, ...] = ()


#: The component splitter's diverging layers, **per row**, because how far apart the class's
#: enthalpies are from the state's is a property of the state.
#:
#: **`ComponentSplitter.run` reports outlet enthalpies that are not the states it reports.**
#: The first row's bottoms reads `-14431.31737097011` where a *fresh* NeqSim fluid at that
#: outlet's own composition, temperature and pressure gives `-20309.24832914077`, and a fresh
#: fluid is what this port computes. The bounds sit a little under each measured ratio, so
#: what is asserted is that the divergence is still there and at least as wide - not its last
#: digits. The first row's overhead is the narrowest at `1.06`, which is worth seeing: the
#: defect is large where the outlet crosses a phase boundary and small where it does not.
_COMPONENT_SPLITTER_DIVERGENCE: tuple[Divergence, ...] = (
    Divergence(
        key="overhead_h",
        at_least=1.05,
        reason="`run` reports an enthalpy the outlet's own composition does not have; a "
        "fresh NeqSim fluid at that state gives the port's number, and on this row the two "
        "are 6% apart",
    ),
    Divergence(
        key="bottoms_h",
        at_least=1.3,
        reason="the same, and wider here: `-14431.32` against the fresh fluid's `-20309.25`",
    ),
)

#: The even-routing row, where both outlets carry the *feed's* composition and the class's
#: enthalpy is more than twice the state's.
_COMPONENT_SPLITTER_EVEN_DIVERGENCE: tuple[Divergence, ...] = (
    Divergence(
        key="overhead_h",
        at_least=2.0,
        reason="both outlets are the feed, so the state's enthalpy is the feed's `-8984.69` "
        "and the class reports `-18823.74`",
    ),
    Divergence(
        key="bottoms_h",
        at_least=2.0,
        reason="the same number on the other outlet",
    ),
)

#: The all-of-one-component row, where the overhead loses a component entirely and the two
#: enthalpies part by more than forty times.
_COMPONENT_SPLITTER_SEPARATION_DIVERGENCE: tuple[Divergence, ...] = (
    Divergence(
        key="overhead_h",
        at_least=40.0,
        reason="the overhead takes all of the methane and none of the pentane, and the class "
        "reports `-45658.96` where the state is `-1032.85`",
    ),
    Divergence(
        key="bottoms_h",
        at_least=1.3,
        reason="`-16148.11` against the state's `-21393.95`",
    ),
)


#: The splitter's caloric divergence was closed upstream in #4158: `Splitter.run` and
#: `ComponentSplitter.run` rebuild the extensive caloric state with `init(2)` after the
#: flash, so the branch enthalpies are the feed's and are compared as agreement.


def _divergences(reason: str, keys: dict[str, float]) -> tuple[Divergence, ...]:
    """A set of layers held to a measured ratio, with one reason for the set."""
    return tuple(Divergence(key=key, at_least=bound, reason=reason) for key, bound in keys.items())


_PACKED_REASON = (
    "the target's InterfaceProperties parachor answer is 0.053145 against the port's "
    "0.053030 on the interface mix; the 0.2% surface tension moves the Onda wetted area, "
    "kla and the transfer that follows it. The port uses the same parachor branch, so this "
    "is the mixed-phase state, not a different correlation."
)

_PACKED_ABSORBER = _divergences(
    _PACKED_REASON,
    {
        "segment1_wetted_area_m2_per_m3": 1.0001,
        "segment2_wetted_area_m2_per_m3": 1.0001,
        "segment3_wetted_area_m2_per_m3": 1.0001,
        "segment4_wetted_area_m2_per_m3": 1.0001,
        "segment1_kla": 1.001,
        "segment2_kla": 1.001,
        "segment3_kla": 1.001,
        "segment4_kla": 1.002,
        "segment1_percent_flood": 1.0001,
        "segment3_percent_flood": 1.0001,
        "segment4_percent_flood": 1.0001,
        "segment1_net_molar_transfer_mol_per_s": 1.004,
        "segment2_net_molar_transfer_mol_per_s": 1.004,
        "segment3_net_molar_transfer_mol_per_s": 1.004,
        "segment4_net_molar_transfer_mol_per_s": 1.004,
    },
)

_PACKED_HEAT_OFF = _divergences(
    _PACKED_REASON,
    {
        "segment3_wetted_area_m2_per_m3": 1.0002,
        "segment1_kla": 1.002,
        "segment2_kla": 1.001,
        "segment3_kla": 1.001,
        "segment4_kla": 1.002,
        "segment1_net_molar_transfer_mol_per_s": 1.004,
        "segment2_net_molar_transfer_mol_per_s": 1.004,
        "segment3_net_molar_transfer_mol_per_s": 1.004,
        "segment4_net_molar_transfer_mol_per_s": 1.004,
    },
)

_PACKED_ZERO_HEIGHT = _divergences(
    _PACKED_REASON,
    {f"segment{segment}_wetted_area_m2_per_m3": 1.0001 for segment in range(1, 5)}
    | {f"segment{segment}_kla": 1.002 for segment in range(1, 5)}
    | {f"segment{segment}_percent_flood": 1.0001 for segment in range(1, 5)},
)

_PACKED_BILLET = _divergences(
    _PACKED_REASON,
    {f"segment{segment}_wetted_area_m2_per_m3": 1.0001 for segment in range(1, 5)}
    | {f"segment{segment}_kla": 1.001 for segment in range(1, 4)}
    | {"segment4_kla": 1.002}
    | {f"segment{segment}_percent_flood": 1.0001 for segment in (1, 2, 4)}
    | {f"segment{segment}_net_molar_transfer_mol_per_s": 1.004 for segment in range(1, 5)},
)

_MURPHREE_REASON = (
    "the target's #4127 full-tray sweep lands this 0.6-efficiency column on a different "
    "fixed point; the port's staged substitution reaches its own, 10-180% away on the "
    "middle trays. The correction itself is the same formula; the recorded profile is the "
    "solver's trajectory, not a widened tolerance."
)

_MURPHREE_DIVERGENCE = _divergences(
    _MURPHREE_REASON,
    {
        "tray0_gas_n": 1.10,
        "tray0_liquid_n": 1.004,
        "tray1_temperature_K": 1.005,
        "tray1_gas_n": 1.07,
        "tray1_liquid_n": 1.05,
        "tray2_temperature_K": 1.016,
        "tray2_gas_n": 1.034,
        "tray2_liquid_n": 1.038,
        "tray3_temperature_K": 1.06,
        "tray3_gas_n": 1.18,
        "tray3_liquid_n": 1.013,
        "tray4_temperature_K": 1.058,
        "tray4_gas_n": 1.22,
        "tray4_liquid_n": 1.82,
        "tray5_gas_n": 1.004,
        "tray5_liquid_n": 2.20,
    },
)

#: The process cases a layer diff can read. **One entry per case**, so a probe row added
#: without a case - or a case whose probe row was reordered - is a mismatch
#: `python/tests/test_process_layer_diff.py` fails on rather than a silent mis-pairing.
LAYER_CASES: tuple[LayerCase, ...] = (
    LayerCase(
        model="process.hem_release",
        case="methane_50bara_choked",
        capture="process_hem_release.tsv",
        block=0,
        identified_by=("#label", "methane_50bara_choked"),
    ),
    LayerCase(
        model="process.hem_release",
        case="propane_flashes_through_the_opening",
        capture="process_hem_release.tsv",
        block=11,
        identified_by=("#label", "propane_20bara"),
    ),
    LayerCase(
        model="process.leak_blowdown",
        case="methane_50bara_ten_seconds",
        capture="process_leak_blowdown.tsv",
        block=0,
        identified_by=("#label", "pr_methane_50bara_10s_1s"),
    ),
    # The toxic library's twelve: the ten canonical rows and the two conversions; the
    # five aliases, the case-failure states, three unknown names and four evidence rows
    # are uncased.
    LayerCase(
        model="process.toxic_library",
        case="threshold_h2s",
        capture="process_toxic_library.tsv",
        block=0,
        identified_by=("#label", "threshold_H2S"),
    ),
    LayerCase(
        model="process.toxic_library",
        case="threshold_co",
        capture="process_toxic_library.tsv",
        block=2,
        identified_by=("#label", "threshold_CO"),
    ),
    LayerCase(
        model="process.toxic_library",
        case="threshold_cl2",
        capture="process_toxic_library.tsv",
        block=4,
        identified_by=("#label", "threshold_Cl2"),
    ),
    LayerCase(
        model="process.toxic_library",
        case="threshold_nh3",
        capture="process_toxic_library.tsv",
        block=6,
        identified_by=("#label", "threshold_NH3"),
    ),
    LayerCase(
        model="process.toxic_library",
        case="threshold_so2",
        capture="process_toxic_library.tsv",
        block=8,
        identified_by=("#label", "threshold_SO2"),
    ),
    LayerCase(
        model="process.toxic_library",
        case="threshold_hcn",
        capture="process_toxic_library.tsv",
        block=10,
        identified_by=("#label", "threshold_HCN"),
    ),
    LayerCase(
        model="process.toxic_library",
        case="threshold_hf",
        capture="process_toxic_library.tsv",
        block=11,
        identified_by=("#label", "threshold_HF"),
    ),
    LayerCase(
        model="process.toxic_library",
        case="threshold_hcl",
        capture="process_toxic_library.tsv",
        block=12,
        identified_by=("#label", "threshold_HCl"),
    ),
    LayerCase(
        model="process.toxic_library",
        case="threshold_methanol",
        capture="process_toxic_library.tsv",
        block=13,
        identified_by=("#label", "threshold_methanol"),
    ),
    LayerCase(
        model="process.toxic_library",
        case="threshold_benzene",
        capture="process_toxic_library.tsv",
        block=14,
        identified_by=("#label", "threshold_benzene"),
    ),
    LayerCase(
        model="process.toxic_library",
        case="h2s_100ppm_288k",
        capture="process_toxic_library.tsv",
        block=30,
        identified_by=("#label", "h2s_100ppm_288K"),
    ),
    LayerCase(
        model="process.toxic_library",
        case="h2s_100ppm_1bar_300k",
        capture="process_toxic_library.tsv",
        block=31,
        identified_by=("#label", "h2s_100ppm_1bar_300K"),
    ),
    # The hazardous area's six: the documentation state, the safety-factor and ambient
    # overrides, a tenfold flow and both other grades; the nine guards are uncased.
    LayerCase(
        model="process.hazardous_area",
        case="methane_default",
        capture="process_hazardous_area.tsv",
        block=0,
        identified_by=("#label", "methane_default"),
    ),
    LayerCase(
        model="process.hazardous_area",
        case="methane_safety_025",
        capture="process_hazardous_area.tsv",
        block=1,
        identified_by=("#label", "methane_safety_025"),
    ),
    LayerCase(
        model="process.hazardous_area",
        case="methane_flow_1",
        capture="process_hazardous_area.tsv",
        block=2,
        identified_by=("#label", "methane_flow_1"),
    ),
    LayerCase(
        model="process.hazardous_area",
        case="methane_ambient_1",
        capture="process_hazardous_area.tsv",
        block=3,
        identified_by=("#label", "methane_ambient_1"),
    ),
    LayerCase(
        model="process.hazardous_area",
        case="primary_zone",
        capture="process_hazardous_area.tsv",
        block=4,
        identified_by=("#label", "primary_zone"),
    ),
    LayerCase(
        model="process.hazardous_area",
        case="continuous_zone",
        capture="process_hazardous_area.tsv",
        block=5,
        identified_by=("#label", "continuous_zone"),
    ),
    # The dense gas's five: the analyzer's two distances, the alpha floor, equal
    # densities and a buoyant source; the six guards are uncased.
    LayerCase(
        model="process.heavy_gas_dispersion",
        case="methane_forced_heavy_lfl",
        capture="process_heavy_gas_dispersion.tsv",
        block=0,
        identified_by=("#label", "methane_forced_heavy_lfl"),
    ),
    LayerCase(
        model="process.heavy_gas_dispersion",
        case="methane_forced_heavy_half_lfl",
        capture="process_heavy_gas_dispersion.tsv",
        block=1,
        identified_by=("#label", "methane_forced_heavy_half_lfl"),
    ),
    LayerCase(
        model="process.heavy_gas_dispersion",
        case="alpha_floor",
        capture="process_heavy_gas_dispersion.tsv",
        block=2,
        identified_by=("#label", "alpha_floor"),
    ),
    LayerCase(
        model="process.heavy_gas_dispersion",
        case="equal_density",
        capture="process_heavy_gas_dispersion.tsv",
        block=3,
        identified_by=("#label", "equal_density"),
    ),
    LayerCase(
        model="process.heavy_gas_dispersion",
        case="buoyant_source",
        capture="process_heavy_gas_dispersion.tsv",
        block=4,
        identified_by=("#label", "buoyant_source"),
    ),
    # The plume's nine: every block is a state the port reproduces, so all nine are cased.
    LayerCase(
        model="process.gaussian_plume",
        case="rural_d_at_100_metres",
        capture="process_gaussian_plume.tsv",
        block=0,
        identified_by=("#label", "rural_d_100m"),
    ),
    LayerCase(
        model="process.gaussian_plume",
        case="rural_d_at_500_metres",
        capture="process_gaussian_plume.tsv",
        block=1,
        identified_by=("#label", "rural_d_500m"),
    ),
    LayerCase(
        model="process.gaussian_plume",
        case="rural_d_at_1000_metres",
        capture="process_gaussian_plume.tsv",
        block=2,
        identified_by=("#label", "rural_d_1000m"),
    ),
    LayerCase(
        model="process.gaussian_plume",
        case="urban_b_distance_to_threshold",
        capture="process_gaussian_plume.tsv",
        block=3,
        identified_by=("#label", "urban_b_500m"),
    ),
    LayerCase(
        model="process.gaussian_plume",
        case="off_centre_and_elevated_receptor",
        capture="process_gaussian_plume.tsv",
        block=4,
        identified_by=("#label", "rural_d_offcentre"),
    ),
    LayerCase(
        model="process.gaussian_plume",
        case="ground_level_source_crossing",
        capture="process_gaussian_plume.tsv",
        block=5,
        identified_by=("#label", "rural_d_crossing"),
    ),
    LayerCase(
        model="process.gaussian_plume",
        case="threshold_never_reached",
        capture="process_gaussian_plume.tsv",
        block=6,
        identified_by=("#label", "rural_d_never_above"),
    ),
    LayerCase(
        model="process.gaussian_plume",
        case="threshold_never_fallen_below",
        capture="process_gaussian_plume.tsv",
        block=7,
        identified_by=("#label", "rural_d_never_below"),
    ),
    LayerCase(
        model="process.gaussian_plume",
        case="zero_emission_answers_zero",
        capture="process_gaussian_plume.tsv",
        block=8,
        identified_by=("#label", "zero_emission"),
    ),
    # The dispersion screen's thirteen PR rows; its four SRK evidence rows and the
    # class's zero-rate refusal are declared uncased.
    LayerCase(
        model="process.gas_dispersion",
        case="methane_gaussian_plume",
        capture="process_gas_dispersion.tsv",
        block=0,
        identified_by=("#label", "pr_methane_gaussian"),
    ),
    LayerCase(
        model="process.gas_dispersion",
        case="methane_sts0131_integral_endpoint",
        capture="process_gas_dispersion.tsv",
        block=1,
        identified_by=("#label", "pr_methane_sts0131_integral"),
    ),
    LayerCase(
        model="process.gas_dispersion",
        case="methane_sts0131_cfd_endpoint",
        capture="process_gas_dispersion.tsv",
        block=2,
        identified_by=("#label", "pr_methane_sts0131_cfd"),
    ),
    LayerCase(
        model="process.gas_dispersion",
        case="carbon_dioxide_dense_and_no_fuel",
        capture="process_gas_dispersion.tsv",
        block=3,
        identified_by=("#label", "pr_co2_dense"),
    ),
    LayerCase(
        model="process.gas_dispersion",
        case="sour_gas_toxic_endpoint",
        capture="process_gas_dispersion.tsv",
        block=4,
        identified_by=("#label", "pr_h2s_toxic_gaussian"),
    ),
    LayerCase(
        model="process.gas_dispersion",
        case="manual_lfl_override",
        capture="process_gas_dispersion.tsv",
        block=5,
        identified_by=("#label", "pr_methane_manual_lfl"),
    ),
    LayerCase(
        model="process.gas_dispersion",
        case="forced_heavy_on_methane",
        capture="process_gas_dispersion.tsv",
        block=6,
        identified_by=("#label", "pr_methane_forced_heavy"),
    ),
    LayerCase(
        model="process.gas_dispersion",
        case="forced_gaussian_on_carbon_dioxide",
        capture="process_gas_dispersion.tsv",
        block=7,
        identified_by=("#label", "pr_co2_forced_gaussian"),
    ),
    LayerCase(
        model="process.gas_dispersion",
        case="benchmark_methane",
        capture="process_gas_dispersion.tsv",
        block=8,
        identified_by=("#label", "pr_benchmark_methane"),
    ),
    LayerCase(
        model="process.gas_dispersion",
        case="benchmark_carbon_dioxide_toxic",
        capture="process_gas_dispersion.tsv",
        block=9,
        identified_by=("#label", "pr_benchmark_co2_toxic"),
    ),
    LayerCase(
        model="process.gas_dispersion",
        case="benchmark_propane_dense",
        capture="process_gas_dispersion.tsv",
        block=10,
        identified_by=("#label", "pr_benchmark_propane"),
    ),
    LayerCase(
        model="process.gas_dispersion",
        case="benchmark_sour_gas_toxic",
        capture="process_gas_dispersion.tsv",
        block=11,
        identified_by=("#label", "pr_benchmark_h2s_forced_gaussian"),
    ),
    LayerCase(
        model="process.gas_dispersion",
        case="source_term_peak_rate",
        capture="process_gas_dispersion.tsv",
        block=12,
        identified_by=("#label", "pr_source_term_rate"),
    ),
    # The jet fire's seven reachable states; the zero-humidity NaN, the infinite
    # zero-distance flux and the zero-target refusal are uncased because the port refuses
    # each of them.
    LayerCase(
        model="process.jet_fire",
        case="test_state_at_twenty_metres",
        capture="process_jet_fire.tsv",
        block=0,
        identified_by=("#label", "jet_fire_test_20m"),
    ),
    LayerCase(
        model="process.jet_fire",
        case="test_state_at_eighty_metres",
        capture="process_jet_fire.tsv",
        block=1,
        identified_by=("#label", "jet_fire_test_80m"),
    ),
    LayerCase(
        model="process.jet_fire",
        case="distance_to_twelve_point_five_kw",
        capture="process_jet_fire.tsv",
        block=2,
        identified_by=("#label", "jet_fire_distance_12500"),
    ),
    LayerCase(
        model="process.jet_fire",
        case="humidity_half_at_fifty_metres",
        capture="process_jet_fire.tsv",
        block=4,
        identified_by=("#label", "jet_fire_humidity_05"),
    ),
    LayerCase(
        model="process.jet_fire",
        case="saturated_at_fifty_metres",
        capture="process_jet_fire.tsv",
        block=5,
        identified_by=("#label", "jet_fire_humidity_1"),
    ),
    LayerCase(
        model="process.jet_fire",
        case="inside_one_metre",
        capture="process_jet_fire.tsv",
        block=6,
        identified_by=("#label", "jet_fire_inside_one_metre"),
    ),
    LayerCase(
        model="process.jet_fire",
        case="small_target_reaches_the_cap",
        capture="process_jet_fire.tsv",
        block=7,
        identified_by=("#label", "jet_fire_small_target"),
    ),
    # The pool fire's six reachable states; the 12.5 kW/m2 target that exceeds the fire's
    # near-field flux, the 1 MW/m2 target, the zero target and the dead-parameter state are
    # uncased.
    LayerCase(
        model="process.pool_fire",
        case="test_state_at_the_near_bound",
        capture="process_pool_fire.tsv",
        block=0,
        identified_by=("#label", "pool_fire_test_5m1"),
    ),
    LayerCase(
        model="process.pool_fire",
        case="test_state_at_ten_metres",
        capture="process_pool_fire.tsv",
        block=1,
        identified_by=("#label", "pool_fire_test_10m"),
    ),
    LayerCase(
        model="process.pool_fire",
        case="test_state_at_twenty_metres",
        capture="process_pool_fire.tsv",
        block=2,
        identified_by=("#label", "pool_fire_test_20m"),
    ),
    LayerCase(
        model="process.pool_fire",
        case="test_state_at_fifty_metres",
        capture="process_pool_fire.tsv",
        block=3,
        identified_by=("#label", "pool_fire_test_50m"),
    ),
    LayerCase(
        model="process.pool_fire",
        case="distance_to_ten_kw",
        capture="process_pool_fire.tsv",
        block=5,
        identified_by=("#label", "pool_fire_distance_10000"),
    ),
    LayerCase(
        model="process.pool_fire",
        case="zero_distance",
        capture="process_pool_fire.tsv",
        block=6,
        identified_by=("#label", "pool_fire_zero_distance"),
    ),
    # The PFP demand's nine: every cased state, from the class's three tests through both
    # sides of the rating threshold to the floored driving temperature.
    LayerCase(
        model="process.pfp_demand",
        case="pool_fire_thin_wall_requires_h60",
        capture="process_pfp_demand.tsv",
        block=0,
        identified_by=("#label", "pfp_pool_thin_wall_h60"),
    ),
    LayerCase(
        model="process.pfp_demand",
        case="jet_fire_long_duration_selects_j120",
        capture="process_pfp_demand.tsv",
        block=1,
        identified_by=("#label", "pfp_jet_long_duration_j120"),
    ),
    LayerCase(
        model="process.pfp_demand",
        case="thick_wall_needs_no_protection",
        capture="process_pfp_demand.tsv",
        block=2,
        identified_by=("#label", "pfp_thick_wall_none"),
    ),
    LayerCase(
        model="process.pfp_demand",
        case="equal_time_needs_no_protection",
        capture="process_pfp_demand.tsv",
        block=3,
        identified_by=("#label", "pfp_equal_time_no_pfp"),
    ),
    LayerCase(
        model="process.pfp_demand",
        case="pool_fire_at_the_rating_threshold",
        capture="process_pfp_demand.tsv",
        block=4,
        identified_by=("#label", "pfp_rating_at_threshold"),
    ),
    LayerCase(
        model="process.pfp_demand",
        case="pool_fire_just_past_the_rating_threshold",
        capture="process_pfp_demand.tsv",
        block=5,
        identified_by=("#label", "pfp_rating_past_threshold"),
    ),
    LayerCase(
        model="process.pfp_demand",
        case="jet_fire_short_duration_selects_j60",
        capture="process_pfp_demand.tsv",
        block=6,
        identified_by=("#label", "pfp_jet_short_j60"),
    ),
    LayerCase(
        model="process.pfp_demand",
        case="driving_temperature_is_floored_at_one",
        capture="process_pfp_demand.tsv",
        block=7,
        identified_by=("#label", "pfp_driving_floor"),
    ),
    LayerCase(
        model="process.pfp_demand",
        case="overrides_move_the_bare_time_and_thickness",
        capture="process_pfp_demand.tsv",
        block=8,
        identified_by=("#label", "pfp_overrides"),
    ),
    # The fire-water demand's seven: the class's tests, the vessel surface, the ratio and the
    # zero area demand. The accepted negative object and the six refusals are uncased.
    LayerCase(
        model="process.fire_water_demand",
        case="area_demand_follows_application_rate",
        capture="process_fire_water_demand.tsv",
        block=0,
        identified_by=("#label", "fwd_area_demand"),
    ),
    LayerCase(
        model="process.fire_water_demand",
        case="wellhead_rate_is_double_the_process_area_rate",
        capture="process_fire_water_demand.tsv",
        block=1,
        identified_by=("#label", "fwd_wellhead_rate"),
    ),
    LayerCase(
        model="process.fire_water_demand",
        case="horizontal_vessel_surface_is_shell_and_ends",
        capture="process_fire_water_demand.tsv",
        block=2,
        identified_by=("#label", "fwd_horizontal_vessel"),
    ),
    LayerCase(
        model="process.fire_water_demand",
        case="selective_object_protection_ratio",
        capture="process_fire_water_demand.tsv",
        block=3,
        identified_by=("#label", "fwd_selective_object"),
    ),
    LayerCase(
        model="process.fire_water_demand",
        case="foam_concentrate_scales_with_water_volume",
        capture="process_fire_water_demand.tsv",
        block=4,
        identified_by=("#label", "fwd_foam"),
    ),
    LayerCase(
        model="process.fire_water_demand",
        case="simultaneous_release_factor_multiplies_the_demand",
        capture="process_fire_water_demand.tsv",
        block=5,
        identified_by=("#label", "fwd_simultaneous"),
    ),
    LayerCase(
        model="process.fire_water_demand",
        case="zero_area_demand_has_no_ratio",
        capture="process_fire_water_demand.tsv",
        block=6,
        identified_by=("#label", "fwd_zero_area_ratio"),
    ),
    # The deluge layout's six: the three nozzle factories, the low pressure, the reduced
    # efficiency and the failed clearance. The eight refusals are uncased.
    LayerCase(
        model="process.deluge_nozzle_layout",
        case="high_velocity_nozzles_are_coverage_governed",
        capture="process_deluge_nozzle_layout.tsv",
        block=0,
        identified_by=("#label", "dnl_high_velocity"),
    ),
    LayerCase(
        model="process.deluge_nozzle_layout",
        case="high_capacity_nozzles_are_coverage_governed",
        capture="process_deluge_nozzle_layout.tsv",
        block=1,
        identified_by=("#label", "dnl_high_capacity"),
    ),
    LayerCase(
        model="process.deluge_nozzle_layout",
        case="medium_velocity_nozzles_are_flow_governed",
        capture="process_deluge_nozzle_layout.tsv",
        block=2,
        identified_by=("#label", "dnl_medium_velocity"),
    ),
    LayerCase(
        model="process.deluge_nozzle_layout",
        case="a_pressure_below_the_nozzle_minimum_is_flagged",
        capture="process_deluge_nozzle_layout.tsv",
        block=3,
        identified_by=("#label", "dnl_low_pressure"),
    ),
    LayerCase(
        model="process.deluge_nozzle_layout",
        case="a_reduced_coverage_efficiency_adds_nozzles",
        capture="process_deluge_nozzle_layout.tsv",
        block=4,
        identified_by=("#label", "dnl_efficiency"),
    ),
    LayerCase(
        model="process.deluge_nozzle_layout",
        case="the_clearance_check_uses_the_nozzle_requirement",
        capture="process_deluge_nozzle_layout.tsv",
        block=5,
        identified_by=("#label", "dnl_clearance_fails"),
    ),
    # The monitor coverage's nine: the class's three tests, a marginal and a feasible state,
    # the compliance flow, the all-drift state and the stated dimension. The eleven setter
    # refusals are uncased.
    LayerCase(
        model="process.fire_monitor_coverage",
        case="a_large_deck_monitor_is_not_suitable",
        capture="process_fire_monitor_coverage.tsv",
        block=0,
        identified_by=("#label", "fmc_large_deck"),
    ),
    LayerCase(
        model="process.fire_monitor_coverage",
        case="still_air_coverage_is_full",
        capture="process_fire_monitor_coverage.tsv",
        block=1,
        identified_by=("#label", "fmc_wind_still"),
    ),
    LayerCase(
        model="process.fire_monitor_coverage",
        case="wind_drift_removes_the_coverage",
        capture="process_fire_monitor_coverage.tsv",
        block=2,
        identified_by=("#label", "fmc_wind_drift"),
    ),
    LayerCase(
        model="process.fire_monitor_coverage",
        case="an_obstructed_line_of_sight_disqualifies_a_monitor",
        capture="process_fire_monitor_coverage.tsv",
        block=3,
        identified_by=("#label", "fmc_obstructed"),
    ),
    LayerCase(
        model="process.fire_monitor_coverage",
        case="a_wind_limited_density_is_marginal",
        capture="process_fire_monitor_coverage.tsv",
        block=4,
        identified_by=("#label", "fmc_marginal"),
    ),
    LayerCase(
        model="process.fire_monitor_coverage",
        case="a_small_drift_leaves_the_density_feasible",
        capture="process_fire_monitor_coverage.tsv",
        block=5,
        identified_by=("#label", "fmc_feasible"),
    ),
    LayerCase(
        model="process.fire_monitor_coverage",
        case="the_flow_needed_for_compliance_scales_with_the_fraction",
        capture="process_fire_monitor_coverage.tsv",
        block=6,
        identified_by=("#label", "fmc_flow_needed"),
    ),
    LayerCase(
        model="process.fire_monitor_coverage",
        case="drift_can_remove_every_bit_of_the_coverage",
        capture="process_fire_monitor_coverage.tsv",
        block=7,
        identified_by=("#label", "fmc_drift_removes_coverage"),
    ),
    LayerCase(
        model="process.fire_monitor_coverage",
        case="a_stated_dimension_replaces_the_area_root",
        capture="process_fire_monitor_coverage.tsv",
        block=8,
        identified_by=("#label", "fmc_characteristic_dimension"),
    ),
    # The coverage assessment's five: the class's four tests and the coverage gap with the
    # supply in hand. The unset-supply state and the five refusals are uncased.
    LayerCase(
        model="process.fire_water_coverage_assessment",
        case="no_flow_and_no_pressure_margin_routes_to_a_dedicated_supply",
        capture="process_fire_water_coverage_assessment.tsv",
        block=0,
        identified_by=("#label", "fwca_dedicated_supply"),
    ),
    LayerCase(
        model="process.fire_water_coverage_assessment",
        case="a_pressure_deficit_alone_is_distinguished",
        capture="process_fire_water_coverage_assessment.tsv",
        block=1,
        identified_by=("#label", "fwca_pressure_deficit"),
    ),
    LayerCase(
        model="process.fire_water_coverage_assessment",
        case="a_flow_deficit_alone_is_distinguished",
        capture="process_fire_water_coverage_assessment.tsv",
        block=2,
        identified_by=("#label", "fwca_flow_deficit"),
    ),
    LayerCase(
        model="process.fire_water_coverage_assessment",
        case="a_fully_covered_area_is_compliant",
        capture="process_fire_water_coverage_assessment.tsv",
        block=3,
        identified_by=("#label", "fwca_compliant"),
    ),
    LayerCase(
        model="process.fire_water_coverage_assessment",
        case="a_coverage_gap_with_supply_in_hand",
        capture="process_fire_water_coverage_assessment.tsv",
        block=4,
        identified_by=("#label", "fwca_coverage_gap"),
    ),
    # The protection screening's five: the class's scenario pair, the two further scenarios
    # and the slower blowdown. The NaN-time and negative-demand states are uncased.
    LayerCase(
        model="process.active_fire_protection_screening",
        case="a_gas_jet_fire_is_not_extinguishable_but_blowdown_wins",
        capture="process_active_fire_protection_screening.tsv",
        block=0,
        identified_by=("#label", "afps_gas_jet"),
    ),
    LayerCase(
        model="process.active_fire_protection_screening",
        case="a_liquid_pool_fire_is_extinguishable",
        capture="process_active_fire_protection_screening.tsv",
        block=1,
        identified_by=("#label", "afps_pool_fire"),
    ),
    LayerCase(
        model="process.active_fire_protection_screening",
        case="a_two_phase_jet_fire_is_not_extinguishable",
        capture="process_active_fire_protection_screening.tsv",
        block=2,
        identified_by=("#label", "afps_two_phase_jet"),
    ),
    LayerCase(
        model="process.active_fire_protection_screening",
        case="an_ordinary_combustible_fire_is_extinguishable",
        capture="process_active_fire_protection_screening.tsv",
        block=3,
        identified_by=("#label", "afps_ordinary_combustible"),
    ),
    LayerCase(
        model="process.active_fire_protection_screening",
        case="a_slower_blowdown_leaves_a_negative_margin",
        capture="process_active_fire_protection_screening.tsv",
        block=4,
        identified_by=("#label", "afps_slower_blowdown"),
    ),
    # The flare flame's six: the class's test state, the still-air pair, the windy and the
    # documentation example, the zero flow and the reduced transmissivity. The three clamps,
    # the negative stack and the five refusals are uncased.
    LayerCase(
        model="process.api537_flare_flame_model",
        case="the_flame_length_and_sterile_zones",
        capture="process_api537_flare_flame.tsv",
        block=0,
        identified_by=("#label", "api537_flame_length"),
    ),
    LayerCase(
        model="process.api537_flare_flame_model",
        case="no_wind_leaves_the_tip_vertical",
        capture="process_api537_flare_flame.tsv",
        block=1,
        identified_by=("#label", "api537_no_wind"),
    ),
    LayerCase(
        model="process.api537_flare_flame_model",
        case="a_wind_tilts_the_flame_and_moves_the_sterile_zones",
        capture="process_api537_flare_flame.tsv",
        block=2,
        identified_by=("#label", "api537_wind"),
    ),
    LayerCase(
        model="process.api537_flare_flame_model",
        case="the_documentation_example",
        capture="process_api537_flare_flame.tsv",
        block=3,
        identified_by=("#label", "api537_doc_example"),
    ),
    LayerCase(
        model="process.api537_flare_flame_model",
        case="a_zero_mass_flow_publishes_zeroes",
        capture="process_api537_flare_flame.tsv",
        block=4,
        identified_by=("#label", "api537_zero_flow"),
    ),
    LayerCase(
        model="process.api537_flare_flame_model",
        case="a_reduced_transmissivity_halves_the_flux",
        capture="process_api537_flare_flame.tsv",
        block=5,
        identified_by=("#label", "api537_transmissivity"),
    ),
    # The fault tree's five class states. The eleven blocks after them are the class's
    # unvalidated gates and inputs, declared in `UNCASED_ROWS`.
    LayerCase(
        model="process.fault_tree",
        case="ft_and",
        capture="process_fault_tree.tsv",
        block=0,
        identified_by=("#label", "ft_and"),
    ),
    LayerCase(
        model="process.fault_tree",
        case="ft_or",
        capture="process_fault_tree.tsv",
        block=1,
        identified_by=("#label", "ft_or"),
    ),
    LayerCase(
        model="process.fault_tree",
        case="ft_voting",
        capture="process_fault_tree.tsv",
        block=2,
        identified_by=("#label", "ft_voting"),
    ),
    LayerCase(
        model="process.fault_tree",
        case="ft_beta_or",
        capture="process_fault_tree.tsv",
        block=3,
        identified_by=("#label", "ft_beta_or"),
    ),
    LayerCase(
        model="process.fault_tree",
        case="ft_cut_sets",
        capture="process_fault_tree.tsv",
        block=4,
        identified_by=("#label", "ft_cut_sets"),
    ),
    # The bow-tie's five caseable states. The eight blocks after them (an empty bow-tie,
    # a negative frequency, PFDs outside [0, 1], a NaN, no consequences, all threats
    # inactive) are declared in `UNCASED_ROWS`.
    LayerCase(
        model="process.bow_tie",
        case="bt_class_test",
        capture="process_bow_tie.tsv",
        block=0,
        identified_by=("#label", "bt_class_test"),
    ),
    LayerCase(
        model="process.bow_tie",
        case="bt_mitigation_ignored",
        capture="process_bow_tie.tsv",
        block=1,
        identified_by=("#label", "bt_mitigation_ignored"),
    ),
    LayerCase(
        model="process.bow_tie",
        case="bt_both_types",
        capture="process_bow_tie.tsv",
        block=2,
        identified_by=("#label", "bt_both_types"),
    ),
    LayerCase(
        model="process.bow_tie",
        case="bt_link_ignored",
        capture="process_bow_tie.tsv",
        block=3,
        identified_by=("#label", "bt_link_ignored"),
    ),
    LayerCase(
        model="process.bow_tie",
        case="bt_nonfunctional",
        capture="process_bow_tie.tsv",
        block=5,
        identified_by=("#label", "bt_nonfunctional"),
    ),
    # The SIF's five caseable states. The five blocks after them (a negative PFD, a NaN,
    # the two SIL refusals and the static max-PFD table) are declared in `UNCASED_ROWS`.
    LayerCase(
        model="process.safety_instrumented_function",
        case="sif_class_test",
        capture="process_safety_instrumented_function.tsv",
        block=0,
        identified_by=("#label", "sif_class_test"),
    ),
    LayerCase(
        model="process.safety_instrumented_function",
        case="sif_required_pfd_at_zero",
        capture="process_safety_instrumented_function.tsv",
        block=1,
        identified_by=("#label", "sif_required_pfd_at_zero"),
    ),
    LayerCase(
        model="process.safety_instrumented_function",
        case="sif_pfd_zero",
        capture="process_safety_instrumented_function.tsv",
        block=2,
        identified_by=("#label", "sif_pfd_zero"),
    ),
    LayerCase(
        model="process.safety_instrumented_function",
        case="sif_spurious_trips",
        capture="process_safety_instrumented_function.tsv",
        block=3,
        identified_by=("#label", "sif_spurious_trips"),
    ),
    LayerCase(
        model="process.safety_instrumented_function",
        case="sif_out_of_band",
        capture="process_safety_instrumented_function.tsv",
        block=4,
        identified_by=("#label", "sif_out_of_band"),
    ),
    # The LOPA's five caseable states. The thirteen blocks after them (the floating-point
    # boundary, the six STS0131 bands, a zero-PFD layer, a negative initiating frequency
    # and the four refusals) are declared in `UNCASED_ROWS`.
    LayerCase(
        model="process.lopa",
        case="lopa_target_met",
        capture="process_lopa.tsv",
        block=0,
        identified_by=("#label", "lopa_target_met"),
    ),
    LayerCase(
        model="process.lopa",
        case="lopa_gap",
        capture="process_lopa.tsv",
        block=1,
        identified_by=("#label", "lopa_gap"),
    ),
    LayerCase(
        model="process.lopa",
        case="lopa_boundary_equal",
        capture="process_lopa.tsv",
        block=2,
        identified_by=("#label", "lopa_boundary_equal"),
    ),
    LayerCase(
        model="process.lopa",
        case="lopa_zero_mitigated",
        capture="process_lopa.tsv",
        block=4,
        identified_by=("#label", "lopa_zero_mitigated"),
    ),
    LayerCase(
        model="process.lopa",
        case="lopa_zero_target_unmet",
        capture="process_lopa.tsv",
        block=5,
        identified_by=("#label", "lopa_zero_target_unmet"),
    ),
    # The SIL verification's five caseable states. The six blocks after them (1oo3, the
    # `4oo2` substring, the two proof boundaries, a NaN and a negative PFD) are declared
    # in `UNCASED_ROWS`.
    LayerCase(
        model="process.sil_verification",
        case="sv_canonical",
        capture="process_sil_verification.tsv",
        block=0,
        identified_by=("#label", "sv_canonical"),
    ),
    LayerCase(
        model="process.sil_verification",
        case="sv_pfd_exceeded",
        capture="process_sil_verification.tsv",
        block=1,
        identified_by=("#label", "sv_pfd_exceeded"),
    ),
    LayerCase(
        model="process.sil_verification",
        case="sv_sil4_1oo2",
        capture="process_sil_verification.tsv",
        block=2,
        identified_by=("#label", "sv_sil4_1oo2"),
    ),
    LayerCase(
        model="process.sil_verification",
        case="sv_sil3_1oo1",
        capture="process_sil_verification.tsv",
        block=3,
        identified_by=("#label", "sv_sil3_1oo1"),
    ),
    LayerCase(
        model="process.sil_verification",
        case="sv_unknown_architecture",
        capture="process_sil_verification.tsv",
        block=4,
        identified_by=("#label", "sv_unknown_architecture"),
    ),
    # The NOG 070's five caseable states. The eight blocks after them (the PFD boundary
    # sweep, the full catalogue and the six guards) are declared in `UNCASED_ROWS`.
    LayerCase(
        model="process.nog070_sil_determination",
        case="n070_hipps_compliant",
        capture="process_nog070_sil_determination.tsv",
        block=0,
        identified_by=("#label", "n070_hipps_compliant"),
    ),
    LayerCase(
        model="process.nog070_sil_determination",
        case="n070_hipps_noncompliant",
        capture="process_nog070_sil_determination.tsv",
        block=1,
        identified_by=("#label", "n070_hipps_noncompliant"),
    ),
    LayerCase(
        model="process.nog070_sil_determination",
        case="n070_custom_minimum",
        capture="process_nog070_sil_determination.tsv",
        block=2,
        identified_by=("#label", "n070_custom_minimum"),
    ),
    LayerCase(
        model="process.nog070_sil_determination",
        case="n070_custom_no_minimum",
        capture="process_nog070_sil_determination.tsv",
        block=3,
        identified_by=("#label", "n070_custom_no_minimum"),
    ),
    LayerCase(
        model="process.nog070_sil_determination",
        case="n070_sil_zero_band",
        capture="process_nog070_sil_determination.tsv",
        block=4,
        identified_by=("#label", "n070_sil_zero_band"),
    ),
    # The risk matrix's four caseable states. The six blocks after them are the values
    # the class silently maps to its top category, declared in `UNCASED_ROWS`.
    LayerCase(
        model="process.risk_matrix",
        case="rm_bands",
        capture="process_risk_matrix.tsv",
        block=0,
        identified_by=("#label", "rm_bands"),
    ),
    LayerCase(
        model="process.risk_matrix",
        case="rm_boundaries",
        capture="process_risk_matrix.tsv",
        block=1,
        identified_by=("#label", "rm_boundaries"),
    ),
    LayerCase(
        model="process.risk_matrix",
        case="rm_costs",
        capture="process_risk_matrix.tsv",
        block=2,
        identified_by=("#label", "rm_costs"),
    ),
    LayerCase(
        model="process.risk_matrix",
        case="rm_zero_failures",
        capture="process_risk_matrix.tsv",
        block=3,
        identified_by=("#label", "rm_zero_failures"),
    ),
    # The PSV's fourteen cased blocks: seven gas, five liquid and two two-phase. The
    # refused and accepted-by-the-class states sit outside the cases.
    LayerCase(
        model="process.psv_sizing",
        case="ps_gas_choked",
        capture="process_psv_sizing.tsv",
        block=0,
        identified_by=("#label", "ps_gas_choked"),
    ),
    LayerCase(
        model="process.psv_sizing",
        case="ps_gas_balanced_subcritical",
        capture="process_psv_sizing.tsv",
        block=1,
        identified_by=("#label", "ps_gas_balanced_subcritical"),
    ),
    LayerCase(
        model="process.psv_sizing",
        case="ps_gas_conventional_high_back_pressure",
        capture="process_psv_sizing.tsv",
        block=2,
        identified_by=("#label", "ps_gas_conventional_high_back_pressure"),
    ),
    LayerCase(
        model="process.psv_sizing",
        case="ps_gas_conventional_severe_back_pressure",
        capture="process_psv_sizing.tsv",
        block=3,
        identified_by=("#label", "ps_gas_conventional_severe_back_pressure"),
    ),
    LayerCase(
        model="process.psv_sizing",
        case="ps_gas_rupture_disk",
        capture="process_psv_sizing.tsv",
        block=4,
        identified_by=("#label", "ps_gas_rupture_disk"),
    ),
    LayerCase(
        model="process.psv_sizing",
        case="ps_gas_orifice_exhausted",
        capture="process_psv_sizing.tsv",
        block=5,
        identified_by=("#label", "ps_gas_orifice_exhausted"),
    ),
    LayerCase(
        model="process.psv_sizing",
        case="ps_gas_capacity_roundtrip",
        capture="process_psv_sizing.tsv",
        block=6,
        identified_by=("#label", "ps_gas_capacity_roundtrip"),
    ),
    LayerCase(
        model="process.psv_sizing",
        case="ps_liquid_low_viscosity",
        capture="process_psv_sizing.tsv",
        block=7,
        identified_by=("#label", "ps_liquid_low_viscosity"),
    ),
    LayerCase(
        model="process.psv_sizing",
        case="ps_liquid_high_viscosity",
        capture="process_psv_sizing.tsv",
        block=8,
        identified_by=("#label", "ps_liquid_high_viscosity"),
    ),
    LayerCase(
        model="process.psv_sizing",
        case="ps_liquid_low_overpressure",
        capture="process_psv_sizing.tsv",
        block=9,
        identified_by=("#label", "ps_liquid_low_overpressure"),
    ),
    LayerCase(
        model="process.psv_sizing",
        case="ps_liquid_balanced_high_back_pressure",
        capture="process_psv_sizing.tsv",
        block=10,
        identified_by=("#label", "ps_liquid_balanced_high_back_pressure"),
    ),
    LayerCase(
        model="process.psv_sizing",
        case="ps_liquid_conventional_high_back_pressure",
        capture="process_psv_sizing.tsv",
        block=11,
        identified_by=("#label", "ps_liquid_conventional_high_back_pressure"),
    ),
    LayerCase(
        model="process.psv_sizing",
        case="ps_two_phase_omega_high",
        capture="process_psv_sizing.tsv",
        block=13,
        identified_by=("#label", "ps_two_phase_omega_high"),
    ),
    LayerCase(
        model="process.psv_sizing",
        case="ps_two_phase_omega_low",
        capture="process_psv_sizing.tsv",
        block=14,
        identified_by=("#label", "ps_two_phase_omega_low"),
    ),
    # The fire sizing's three cased blocks; the two dead-input rows and the zero-blowdown
    # state the class accepts sit outside the cases.
    LayerCase(
        model="process.psv_fire_sizing",
        case="pf_peak_dominated",
        capture="process_psv_fire_sizing.tsv",
        block=0,
        identified_by=("#label", "pf_peak_dominated"),
    ),
    LayerCase(
        model="process.psv_fire_sizing",
        case="pf_average_dominated",
        capture="process_psv_fire_sizing.tsv",
        block=1,
        identified_by=("#label", "pf_average_dominated"),
    ),
    LayerCase(
        model="process.psv_fire_sizing",
        case="pf_helpers",
        capture="process_psv_fire_sizing.tsv",
        block=2,
        identified_by=("#label", "pf_helpers"),
    ),
    # The fire relief's six cased blocks: both drainage branches, the geometry, the
    # no-area state, the latent default and the unread firefighting flag. The negative
    # latent heat the class silently defaults sits outside them.
    # The protection studies: the no-credible and empty blocks carry no numeric output, so
    # they are declared in `UNCASED_ROWS` and held by the model cases and the cross-impl.
    LayerCase(
        model="process.overpressure_protection_study",
        case="study_gas",
        capture="process_overpressure_protection_study.tsv",
        block=0,
        identified_by=("#label", "study_gas"),
    ),
    LayerCase(
        model="process.overpressure_protection_study",
        case="study_liquid",
        capture="process_overpressure_protection_study.tsv",
        block=1,
        identified_by=("#label", "study_liquid"),
    ),
    LayerCase(
        model="process.overpressure_protection_study",
        case="study_two_phase",
        capture="process_overpressure_protection_study.tsv",
        block=2,
        identified_by=("#label", "study_two_phase"),
    ),
    LayerCase(
        model="process.overpressure_protection_study",
        case="study_zero_rate",
        capture="process_overpressure_protection_study.tsv",
        block=4,
        identified_by=("#label", "study_zero_rate"),
    ),
    LayerCase(
        model="process.overpressure_protection_study",
        case="study_capacity_shortfall",
        capture="process_overpressure_protection_study.tsv",
        block=5,
        identified_by=("#label", "study_capacity_shortfall"),
    ),
    LayerCase(
        model="process.overpressure_protection_study",
        case="study_acceptance_fail",
        capture="process_overpressure_protection_study.tsv",
        block=6,
        identified_by=("#label", "study_acceptance_fail"),
    ),
    LayerCase(
        model="process.overpressure_protection_study",
        case="study_fire",
        capture="process_overpressure_protection_study.tsv",
        block=7,
        identified_by=("#label", "study_fire"),
    ),
    # The tube ruptures: the raw state block, the class's zero-end reset and the no-fluid
    # branch are uncased.
    LayerCase(
        model="process.tube_rupture_relief",
        case="tr_two_ends",
        capture="process_tube_rupture_relief.tsv",
        block=1,
        identified_by=("#label", "tr_two_ends"),
    ),
    LayerCase(
        model="process.tube_rupture_relief",
        case="tr_one_end",
        capture="process_tube_rupture_relief.tsv",
        block=2,
        identified_by=("#label", "tr_one_end"),
    ),
    LayerCase(
        model="process.tube_rupture_relief",
        case="tr_subcritical",
        capture="process_tube_rupture_relief.tsv",
        block=3,
        identified_by=("#label", "tr_subcritical"),
    ),
    # The check-valve leaks: the raw state block and the no-fluid branch are uncased.
    LayerCase(
        model="process.check_valve_leak_relief",
        case="cvl_default",
        capture="process_check_valve_leak_relief.tsv",
        block=1,
        identified_by=("#label", "cvl_default"),
    ),
    LayerCase(
        model="process.check_valve_leak_relief",
        case="cvl_clamped",
        capture="process_check_valve_leak_relief.tsv",
        block=2,
        identified_by=("#label", "cvl_clamped"),
    ),
    LayerCase(
        model="process.check_valve_leak_relief",
        case="cvl_subcritical",
        capture="process_check_valve_leak_relief.tsv",
        block=3,
        identified_by=("#label", "cvl_subcritical"),
    ),
    LayerCase(
        model="process.check_valve_leak_relief",
        case="cvl_zero_diameter",
        capture="process_check_valve_leak_relief.tsv",
        block=4,
        identified_by=("#label", "cvl_zero_diameter"),
    ),
    # The blocked-outlet loads are every other block, because each row prints the raw
    # `ReliefFluidState` readings beside it. The three state blocks, the no-fluid branch
    # and the refused negative inflow are declared in `UNCASED_ROWS`.
    LayerCase(
        model="process.blocked_outlet_relief",
        case="bo_gas",
        capture="process_blocked_outlet_relief.tsv",
        block=1,
        identified_by=("#label", "bo_gas"),
    ),
    LayerCase(
        model="process.blocked_outlet_relief",
        case="bo_liquid",
        capture="process_blocked_outlet_relief.tsv",
        block=3,
        identified_by=("#label", "bo_liquid"),
    ),
    LayerCase(
        model="process.blocked_outlet_relief",
        case="bo_two_phase",
        capture="process_blocked_outlet_relief.tsv",
        block=5,
        identified_by=("#label", "bo_two_phase"),
    ),
    LayerCase(
        model="process.blocked_outlet_relief",
        case="bo_zero_inflow",
        capture="process_blocked_outlet_relief.tsv",
        block=7,
        identified_by=("#label", "bo_zero_inflow"),
    ),
    # The Semenov screen: every block is a case, from the reported critical temperature
    # through the failed bracket.
    LayerCase(
        model="process.semenov_self_heating",
        case="semenov_base",
        capture="process_semenov_self_heating.tsv",
        block=0,
        identified_by=("#label", "semenov_base"),
    ),
    LayerCase(
        model="process.semenov_self_heating",
        case="semenov_at_critical",
        capture="process_semenov_self_heating.tsv",
        block=1,
        identified_by=("#label", "semenov_at_critical"),
    ),
    LayerCase(
        model="process.semenov_self_heating",
        case="semenov_subcritical",
        capture="process_semenov_self_heating.tsv",
        block=2,
        identified_by=("#label", "semenov_subcritical"),
    ),
    LayerCase(
        model="process.semenov_self_heating",
        case="semenov_ignition",
        capture="process_semenov_self_heating.tsv",
        block=3,
        identified_by=("#label", "semenov_ignition"),
    ),
    LayerCase(
        model="process.semenov_self_heating",
        case="semenov_bulky",
        capture="process_semenov_self_heating.tsv",
        block=4,
        identified_by=("#label", "semenov_bulky"),
    ),
    LayerCase(
        model="process.semenov_self_heating",
        case="semenov_marginal_ratio",
        capture="process_semenov_self_heating.tsv",
        block=5,
        identified_by=("#label", "semenov_marginal_ratio"),
    ),
    LayerCase(
        model="process.semenov_self_heating",
        case="semenov_not_bracketed",
        capture="process_semenov_self_heating.tsv",
        block=6,
        identified_by=("#label", "semenov_not_bracketed"),
    ),
    # The porous screen: every run block is a case; the class's `setKinetics` row is
    # evidence for the identity, not a state the id can express.
    LayerCase(
        model="process.porous_media_self_heating",
        case="porous_slab",
        capture="process_porous_media_self_heating.tsv",
        block=0,
        identified_by=("#label", "porous_slab"),
    ),
    LayerCase(
        model="process.porous_media_self_heating",
        case="porous_infinite_cylinder",
        capture="process_porous_media_self_heating.tsv",
        block=1,
        identified_by=("#label", "porous_infinite_cylinder"),
    ),
    LayerCase(
        model="process.porous_media_self_heating",
        case="porous_sphere",
        capture="process_porous_media_self_heating.tsv",
        block=2,
        identified_by=("#label", "porous_sphere"),
    ),
    LayerCase(
        model="process.porous_media_self_heating",
        case="porous_cube",
        capture="process_porous_media_self_heating.tsv",
        block=3,
        identified_by=("#label", "porous_cube"),
    ),
    LayerCase(
        model="process.porous_media_self_heating",
        case="porous_equicylinder",
        capture="process_porous_media_self_heating.tsv",
        block=4,
        identified_by=("#label", "porous_equicylinder"),
    ),
    LayerCase(
        model="process.porous_media_self_heating",
        case="porous_at_critical_dimension",
        capture="process_porous_media_self_heating.tsv",
        block=5,
        identified_by=("#label", "porous_at_critical_dimension"),
    ),
    LayerCase(
        model="process.porous_media_self_heating",
        case="porous_thin",
        capture="process_porous_media_self_heating.tsv",
        block=6,
        identified_by=("#label", "porous_thin"),
    ),
    LayerCase(
        model="process.porous_media_self_heating",
        case="porous_near_critical",
        capture="process_porous_media_self_heating.tsv",
        block=7,
        identified_by=("#label", "porous_near_critical"),
    ),
    LayerCase(
        model="process.porous_media_self_heating",
        case="porous_thick",
        capture="process_porous_media_self_heating.tsv",
        block=8,
        identified_by=("#label", "porous_thick"),
    ),
    LayerCase(
        model="process.porous_media_self_heating",
        case="porous_pipe_insulation",
        capture="process_porous_media_self_heating.tsv",
        block=9,
        identified_by=("#label", "porous_pipe_insulation"),
    ),
    LayerCase(
        model="process.porous_media_self_heating",
        case="porous_not_bracketed",
        capture="process_porous_media_self_heating.tsv",
        block=10,
        identified_by=("#label", "porous_not_bracketed"),
    ),
    LayerCase(
        model="process.porous_media_self_heating",
        case="porous_kinetics_combined",
        capture="process_porous_media_self_heating.tsv",
        block=11,
        identified_by=("#label", "porous_kinetics_combined"),
    ),
    # The induction runs: six cases, and the cube refusal is evidence.
    LayerCase(
        model="process.self_heating_induction",
        case="induction_ignite",
        capture="process_self_heating_induction.tsv",
        block=0,
        identified_by=("#label", "induction_ignite"),
    ),
    LayerCase(
        model="process.self_heating_induction",
        case="induction_steady",
        capture="process_self_heating_induction.tsv",
        block=1,
        identified_by=("#label", "induction_steady"),
    ),
    LayerCase(
        model="process.self_heating_induction",
        case="induction_hot",
        capture="process_self_heating_induction.tsv",
        block=2,
        identified_by=("#label", "induction_hot"),
    ),
    LayerCase(
        model="process.self_heating_induction",
        case="induction_sphere",
        capture="process_self_heating_induction.tsv",
        block=3,
        identified_by=("#label", "induction_sphere"),
    ),
    LayerCase(
        model="process.self_heating_induction",
        case="induction_preheated",
        capture="process_self_heating_induction.tsv",
        block=4,
        identified_by=("#label", "induction_preheated"),
    ),
    LayerCase(
        model="process.self_heating_induction",
        case="induction_near_critical",
        capture="process_self_heating_induction.tsv",
        block=5,
        identified_by=("#label", "induction_near_critical"),
    ),
    # The basket fits: four cases, and the two refusals are evidence.
    LayerCase(
        model="process.basket_test_regression",
        case="basket_cube_four_points",
        capture="process_basket_test_regression.tsv",
        block=0,
        identified_by=("#label", "basket_cube_four_points"),
    ),
    LayerCase(
        model="process.basket_test_regression",
        case="basket_mixed_geometries",
        capture="process_basket_test_regression.tsv",
        block=1,
        identified_by=("#label", "basket_mixed_geometries"),
    ),
    LayerCase(
        model="process.basket_test_regression",
        case="basket_missing_conductivity",
        capture="process_basket_test_regression.tsv",
        block=2,
        identified_by=("#label", "basket_missing_conductivity"),
    ),
    LayerCase(
        model="process.basket_test_regression",
        case="basket_two_points",
        capture="process_basket_test_regression.tsv",
        block=3,
        identified_by=("#label", "basket_two_points"),
    ),
    # The template's two arms: the two-parameter grid, and a seeded separator with the FLOW
    # grid appended. The other ten blocks - the single grid, the empty grid, the manual row
    # and the seven seed branches - are declared in `UNCASED_ROWS`.
    LayerCase(
        model="process.hazop_template",
        case="ht_grid_two_params",
        capture="process_hazop_template.tsv",
        block=0,
        identified_by=("#label", "ht_grid_two_params"),
    ),
    LayerCase(
        model="process.hazop_template",
        case="ht_grid_and_seed",
        capture="process_hazop_template.tsv",
        block=11,
        identified_by=("#label", "ht_seed_separator_then_grid"),
    ),
    # The worksheet's two cased states; the inclusive-threshold block, the empty worksheet
    # and the four refusals are declared in `UNCASED_ROWS`.
    LayerCase(
        model="process.fmea_worksheet",
        case="fmea_class_test",
        capture="process_fmea_worksheet.tsv",
        block=0,
        identified_by=("#label", "fw_class_test"),
    ),
    LayerCase(
        model="process.fmea_worksheet",
        case="fmea_tie_and_inclusive_threshold",
        capture="process_fmea_worksheet.tsv",
        block=1,
        identified_by=("#label", "fw_tie"),
    ),
    # The population's two cased nodes; the catalogue, the flow sweep, the two lookups, the
    # separator, valve, stream and preserved nodes and the empty node are uncased.
    LayerCase(
        model="process.hazop_consequence_population",
        case="hp_manual_node",
        capture="process_hazop_consequence_population.tsv",
        block=4,
        identified_by=("#label", "hp_populate_manual"),
    ),
    LayerCase(
        model="process.hazop_consequence_population",
        case="hp_compressor_node",
        capture="process_hazop_consequence_population.tsv",
        block=5,
        identified_by=("#label", "hp_populate_compressor"),
    ),
    # The control-valve loads: one state block and one no-fluid block are uncased.
    LayerCase(
        model="process.control_valve_failure_relief",
        case="cvf_area",
        capture="process_control_valve_failure_relief.tsv",
        block=1,
        identified_by=("#label", "cvf_area"),
    ),
    LayerCase(
        model="process.control_valve_failure_relief",
        case="cvf_area_default_downstream",
        capture="process_control_valve_failure_relief.tsv",
        block=2,
        identified_by=("#label", "cvf_area_default_downstream"),
    ),
    LayerCase(
        model="process.control_valve_failure_relief",
        case="cvf_cv",
        capture="process_control_valve_failure_relief.tsv",
        block=3,
        identified_by=("#label", "cvf_cv"),
    ),
    LayerCase(
        model="process.control_valve_failure_relief",
        case="cvf_cv_margin",
        capture="process_control_valve_failure_relief.tsv",
        block=4,
        identified_by=("#label", "cvf_cv_margin"),
    ),
    LayerCase(
        model="process.control_valve_failure_relief",
        case="cvf_subcritical",
        capture="process_control_valve_failure_relief.tsv",
        block=5,
        identified_by=("#label", "cvf_subcritical"),
    ),
    LayerCase(
        model="process.control_valve_failure_relief",
        case="cvf_no_area",
        capture="process_control_valve_failure_relief.tsv",
        block=6,
        identified_by=("#label", "cvf_no_area"),
    ),
    LayerCase(
        model="process.fire_case_relief",
        case="fc_area_drainage",
        capture="process_fire_case_relief.tsv",
        block=0,
        identified_by=("#label", "fc_area_drainage"),
    ),
    LayerCase(
        model="process.fire_case_relief",
        case="fc_area_no_drainage",
        capture="process_fire_case_relief.tsv",
        block=1,
        identified_by=("#label", "fc_area_no_drainage"),
    ),
    LayerCase(
        model="process.fire_case_relief",
        case="fc_geometry",
        capture="process_fire_case_relief.tsv",
        block=2,
        identified_by=("#label", "fc_geometry"),
    ),
    LayerCase(
        model="process.fire_case_relief",
        case="fc_no_area",
        capture="process_fire_case_relief.tsv",
        block=3,
        identified_by=("#label", "fc_no_area"),
    ),
    LayerCase(
        model="process.fire_case_relief",
        case="fc_latent_default",
        capture="process_fire_case_relief.tsv",
        block=4,
        identified_by=("#label", "fc_latent_default"),
    ),
    LayerCase(
        model="process.fire_case_relief",
        case="fc_firefighting_unread",
        capture="process_fire_case_relief.tsv",
        block=6,
        identified_by=("#label", "fc_firefighting_unread"),
    ),
    # The acceptance's six: the fire, single, multiple, ignored-count, equality and
    # exceeded states.
    LayerCase(
        model="process.overpressure_acceptance",
        case="oa_fire",
        capture="process_overpressure_acceptance.tsv",
        block=0,
        identified_by=("#label", "oa_fire"),
    ),
    LayerCase(
        model="process.overpressure_acceptance",
        case="oa_single_at_limit",
        capture="process_overpressure_acceptance.tsv",
        block=1,
        identified_by=("#label", "oa_single_at_limit"),
    ),
    LayerCase(
        model="process.overpressure_acceptance",
        case="oa_multiple",
        capture="process_overpressure_acceptance.tsv",
        block=2,
        identified_by=("#label", "oa_multiple"),
    ),
    LayerCase(
        model="process.overpressure_acceptance",
        case="oa_fire_multiple_ignored",
        capture="process_overpressure_acceptance.tsv",
        block=3,
        identified_by=("#label", "oa_fire_multiple_ignored"),
    ),
    LayerCase(
        model="process.overpressure_acceptance",
        case="oa_exceeded",
        capture="process_overpressure_acceptance.tsv",
        block=4,
        identified_by=("#label", "oa_exceeded"),
    ),
    LayerCase(
        model="process.overpressure_acceptance",
        case="oa_single_exceeded",
        capture="process_overpressure_acceptance.tsv",
        block=5,
        identified_by=("#label", "oa_single_exceeded"),
    ),
    LayerCase(
        model="process.pipeline_pressure_protection",
        case="pp_fully_rated",
        capture="process_pipeline_pressure_protection.tsv",
        block=0,
        identified_by=("#label", "pp_fully_rated"),
    ),
    LayerCase(
        model="process.pipeline_pressure_protection",
        case="pp_two_barriers",
        capture="process_pipeline_pressure_protection.tsv",
        block=1,
        identified_by=("#label", "pp_two_barriers"),
    ),
    LayerCase(
        model="process.pipeline_pressure_protection",
        case="pp_insufficient",
        capture="process_pipeline_pressure_protection.tsv",
        block=2,
        identified_by=("#label", "pp_insufficient"),
    ),
    LayerCase(
        model="process.pipeline_pressure_protection",
        case="pp_source_at_design",
        capture="process_pipeline_pressure_protection.tsv",
        block=3,
        identified_by=("#label", "pp_source_at_design"),
    ),
    LayerCase(
        model="process.pipeline_pressure_protection",
        case="pp_barrier2_at_mip",
        capture="process_pipeline_pressure_protection.tsv",
        block=4,
        identified_by=("#label", "pp_barrier2_at_mip"),
    ),
    LayerCase(
        model="process.pipeline_pressure_protection",
        case="pp_barrier1_above_design",
        capture="process_pipeline_pressure_protection.tsv",
        block=5,
        identified_by=("#label", "pp_barrier1_above_design"),
    ),
    LayerCase(
        model="process.relief_disposal_network",
        case="dn_all_simultaneous",
        capture="process_relief_disposal_network.tsv",
        block=0,
        identified_by=("#label", "dn_all_simultaneous"),
    ),
    LayerCase(
        model="process.relief_disposal_network",
        case="dn_mixed",
        capture="process_relief_disposal_network.tsv",
        block=1,
        identified_by=("#label", "dn_mixed"),
    ),
    LayerCase(
        model="process.relief_disposal_network",
        case="dn_tie_first_wins",
        capture="process_relief_disposal_network.tsv",
        block=2,
        identified_by=("#label", "dn_tie_first_wins"),
    ),
    LayerCase(
        model="process.relief_disposal_network",
        case="dn_all_zero",
        capture="process_relief_disposal_network.tsv",
        block=3,
        identified_by=("#label", "dn_all_zero"),
    ),
    LayerCase(
        model="process.tr3001_compliance",
        case="tr_all_pass",
        capture="process_tr3001_compliance.tsv",
        block=0,
        identified_by=("#label", "tr_all_pass"),
    ),
    LayerCase(
        model="process.tr3001_compliance",
        case="tr_no_credible",
        capture="process_tr3001_compliance.tsv",
        block=1,
        identified_by=("#label", "tr_no_credible"),
    ),
    LayerCase(
        model="process.tr3001_compliance",
        case="tr_unsized",
        capture="process_tr3001_compliance.tsv",
        block=2,
        identified_by=("#label", "tr_unsized"),
    ),
    LayerCase(
        model="process.tr3001_compliance",
        case="tr_capacity_fail",
        capture="process_tr3001_compliance.tsv",
        block=3,
        identified_by=("#label", "tr_capacity_fail"),
    ),
    LayerCase(
        model="process.tr3001_compliance",
        case="tr_acceptance_fail",
        capture="process_tr3001_compliance.tsv",
        block=4,
        identified_by=("#label", "tr_acceptance_fail"),
    ),
    LayerCase(
        model="process.tr3001_compliance",
        case="tr_fire_basis_review",
        capture="process_tr3001_compliance.tsv",
        block=5,
        identified_by=("#label", "tr_fire_basis_review"),
    ),
    LayerCase(
        model="process.tr3001_compliance",
        case="tr_other_cause",
        capture="process_tr3001_compliance.tsv",
        block=6,
        identified_by=("#label", "tr_other_cause"),
    ),
    LayerCase(
        model="process.tr3001_compliance",
        case="tr_null_acceptance",
        capture="process_tr3001_compliance.tsv",
        block=7,
        identified_by=("#label", "tr_null_acceptance"),
    ),
    # The VCE's thirteen reachable states: the test state at two distances, the 5 kPa
    # contour and all ten plateaus. The 20.7 kPa target that exceeds the class-4 plateau,
    # the 1 MPa target and the zero target are uncased.
    LayerCase(
        model="process.vce",
        case="test_state_at_twenty_metres",
        capture="process_vce.tsv",
        block=0,
        identified_by=("#label", "vce_test_20m"),
    ),
    LayerCase(
        model="process.vce",
        case="test_state_at_two_hundred_metres",
        capture="process_vce.tsv",
        block=1,
        identified_by=("#label", "vce_test_200m"),
    ),
    LayerCase(
        model="process.vce",
        case="distance_to_five_kpa",
        capture="process_vce.tsv",
        block=3,
        identified_by=("#label", "vce_distance_5000"),
    ),
    LayerCase(
        model="process.vce",
        case="class_1_plateau",
        capture="process_vce.tsv",
        block=4,
        identified_by=("#label", "vce_class_1"),
    ),
    LayerCase(
        model="process.vce",
        case="class_2_plateau",
        capture="process_vce.tsv",
        block=5,
        identified_by=("#label", "vce_class_2"),
    ),
    LayerCase(
        model="process.vce",
        case="class_3_plateau",
        capture="process_vce.tsv",
        block=6,
        identified_by=("#label", "vce_class_3"),
    ),
    LayerCase(
        model="process.vce",
        case="class_4_plateau",
        capture="process_vce.tsv",
        block=7,
        identified_by=("#label", "vce_class_4"),
    ),
    LayerCase(
        model="process.vce",
        case="class_5_plateau",
        capture="process_vce.tsv",
        block=8,
        identified_by=("#label", "vce_class_5"),
    ),
    LayerCase(
        model="process.vce",
        case="class_6_plateau",
        capture="process_vce.tsv",
        block=9,
        identified_by=("#label", "vce_class_6"),
    ),
    LayerCase(
        model="process.vce",
        case="class_7_plateau",
        capture="process_vce.tsv",
        block=10,
        identified_by=("#label", "vce_class_7"),
    ),
    LayerCase(
        model="process.vce",
        case="class_8_plateau",
        capture="process_vce.tsv",
        block=11,
        identified_by=("#label", "vce_class_8"),
    ),
    LayerCase(
        model="process.vce",
        case="class_9_plateau",
        capture="process_vce.tsv",
        block=12,
        identified_by=("#label", "vce_class_9"),
    ),
    LayerCase(
        model="process.vce",
        case="class_10_plateau",
        capture="process_vce.tsv",
        block=13,
        identified_by=("#label", "vce_class_10"),
    ),
    # The BLEVE's five reachable states; the zero-humidity NaN, the unreachable target,
    # the negative fraction, the infinite zero-distance flux and the zero target are
    # uncased.
    LayerCase(
        model="process.bleve",
        case="test_state_geometry",
        capture="process_bleve.tsv",
        block=0,
        identified_by=("#label", "bleve_test_geometry"),
    ),
    LayerCase(
        model="process.bleve",
        case="flux_at_one_hundred_metres",
        capture="process_bleve.tsv",
        block=1,
        identified_by=("#label", "bleve_flux_100m"),
    ),
    LayerCase(
        model="process.bleve",
        case="humidity_half_at_fifty_metres",
        capture="process_bleve.tsv",
        block=2,
        identified_by=("#label", "bleve_humidity_05_50m"),
    ),
    LayerCase(
        model="process.bleve",
        case="distance_to_twelve_point_five_kw",
        capture="process_bleve.tsv",
        block=4,
        identified_by=("#label", "bleve_distance_12500"),
    ),
    LayerCase(
        model="process.bleve",
        case="small_target_contour",
        capture="process_bleve.tsv",
        block=5,
        identified_by=("#label", "bleve_small_target"),
    ),
    # The consequence engine's seven reachable states; the zero-distance and
    # negative-probability evidence rows are uncased.
    LayerCase(
        model="process.consequence_analysis",
        case="jet_fire_at_ten_metres",
        capture="process_consequence_analysis.tsv",
        block=0,
        identified_by=("#label", "consequence_analysis_jet_fire"),
    ),
    LayerCase(
        model="process.consequence_analysis",
        case="pool_fire_at_twenty_metres",
        capture="process_consequence_analysis.tsv",
        block=1,
        identified_by=("#label", "consequence_analysis_pool_fire"),
    ),
    LayerCase(
        model="process.consequence_analysis",
        case="vce_at_twenty_metres",
        capture="process_consequence_analysis.tsv",
        block=2,
        identified_by=("#label", "consequence_analysis_vce"),
    ),
    LayerCase(
        model="process.consequence_analysis",
        case="toxic_dispersion_at_fifty_metres",
        capture="process_consequence_analysis.tsv",
        block=3,
        identified_by=("#label", "consequence_analysis_toxic_dispersion"),
    ),
    LayerCase(
        model="process.consequence_analysis",
        case="mixed_outcomes_at_twenty_metres",
        capture="process_consequence_analysis.tsv",
        block=4,
        identified_by=("#label", "consequence_analysis_mixed"),
    ),
    LayerCase(
        model="process.consequence_analysis",
        case="zero_exposure_is_zero_probability",
        capture="process_consequence_analysis.tsv",
        block=5,
        identified_by=("#label", "consequence_analysis_zero_exposure"),
    ),
    LayerCase(
        model="process.consequence_analysis",
        case="zero_emission_is_zero_probability",
        capture="process_consequence_analysis.tsv",
        block=6,
        identified_by=("#label", "consequence_analysis_zero_emission"),
    ),
    # The probit's nine factory and stated-probit rows; the non-positive-dose guard is
    # uncased because the port refuses it.
    LayerCase(
        model="process.probit",
        case="custom_triple_at_dose_one",
        capture="process_probit.tsv",
        block=0,
        identified_by=("#label", "custom_five_at_dose_one"),
    ),
    LayerCase(
        model="process.probit",
        case="probability_at_a_stated_probit",
        capture="process_probit.tsv",
        block=1,
        identified_by=("#label", "custom_five_from_probit"),
    ),
    LayerCase(
        model="process.probit",
        case="h2s_low_dose",
        capture="process_probit.tsv",
        block=2,
        identified_by=("#label", "h2s_low"),
    ),
    LayerCase(
        model="process.probit",
        case="h2s_high_dose",
        capture="process_probit.tsv",
        block=3,
        identified_by=("#label", "h2s_high"),
    ),
    LayerCase(
        model="process.probit",
        case="thermal_low_dose",
        capture="process_probit.tsv",
        block=4,
        identified_by=("#label", "thermal_low"),
    ),
    LayerCase(
        model="process.probit",
        case="thermal_high_dose",
        capture="process_probit.tsv",
        block=5,
        identified_by=("#label", "thermal_high"),
    ),
    LayerCase(
        model="process.probit",
        case="chlorine_sample_dose",
        capture="process_probit.tsv",
        block=6,
        identified_by=("#label", "chlorine_sample"),
    ),
    LayerCase(
        model="process.probit",
        case="ammonia_sample_dose",
        capture="process_probit.tsv",
        block=7,
        identified_by=("#label", "ammonia_sample"),
    ),
    LayerCase(
        model="process.probit",
        case="blast_sample_dose",
        capture="process_probit.tsv",
        block=8,
        identified_by=("#label", "blast_sample"),
    ),
    # The separator's capacity surface. **Seven of the capture's fifteen blocks are these
    # cases**, and the other eight are declared uncased: three PR rows the cases already
    # state under another setting, four SRK rows re-casing the pin's own tests, and the
    # `GasScrubber` defaults row the port deliberately does not adopt.
    LayerCase(
        model="process.separator",
        case="the_capacity_at_the_design_vessel",
        capture="process_separator_capacity.tsv",
        block=0,
        identified_by=("#label", "pr_two_phase_vertical_d1000_k011"),
    ),
    LayerCase(
        model="process.separator",
        case="a_horizontal_vessel_derates_the_gas_area",
        capture="process_separator_capacity.tsv",
        block=1,
        identified_by=("#label", "pr_two_phase_horizontal_d1000_k011"),
    ),
    LayerCase(
        model="process.separator",
        case="the_vertical_area_ignores_the_design_liquid_level",
        capture="process_separator_capacity.tsv",
        block=3,
        identified_by=("#label", "pr_two_phase_vertical_fg02"),
    ),
    LayerCase(
        model="process.separator",
        case="a_dry_gas_uses_the_sizing_fallback",
        capture="process_separator_capacity.tsv",
        block=5,
        identified_by=("#label", "pr_dry_gas_vertical_d1000_k011"),
    ),
    LayerCase(
        model="process.separator",
        case="an_overloaded_vessel_reports_overload",
        capture="process_separator_capacity.tsv",
        block=7,
        identified_by=("#label", "pr_overloaded_vertical_d020_k011"),
    ),
    LayerCase(
        model="process.separator",
        case="entrainment_moves_the_liquid_the_metric_reads",
        capture="process_separator_capacity.tsv",
        block=8,
        identified_by=("#label", "pr_entrained_gas_in_liquid_005_vertical_d1000"),
    ),
    LayerCase(
        model="process.separator",
        case="the_metric_is_at_the_outlet_state",
        capture="process_separator_capacity.tsv",
        block=9,
        identified_by=("#label", "pr_pressure_drop_outlet_state_vertical_d1000"),
    ),
    # The sizing capture's four cased blocks. **The other five are declared uncased**: the
    # level-`0.2` row repeats the horizontal case under a stated design level, the two
    # near-critical rows are the derating measurement (a getter the port refuses), the
    # three-phase row is the capacity family's under another feed, and the liquid-only row
    # is the class's no-sizing branch, which publishes nothing to compare.
    LayerCase(
        model="process.separator",
        case="a_vertical_vessel_ignores_the_operating_level",
        capture="process_separator_sizing.tsv",
        block=0,
        identified_by=("#label", "pr_two_phase_vertical"),
    ),
    LayerCase(
        model="process.separator",
        case="the_phase_indexed_load_factor_reads_the_operating_level",
        capture="process_separator_sizing.tsv",
        block=1,
        identified_by=("#label", "pr_two_phase_horizontal"),
    ),
    LayerCase(
        model="process.separator",
        case="a_dry_gas_has_no_liquid_phase_to_index",
        capture="process_separator_sizing.tsv",
        block=6,
        identified_by=("#label", "pr_dry_gas_vertical"),
    ),
    LayerCase(
        model="process.separator",
        case="the_safety_factor_scales_the_required_diameter",
        capture="process_separator_sizing.tsv",
        block=8,
        identified_by=("#label", "pr_sizing_safety_15"),
    ),
    LayerCase(
        model="process.separator",
        case="the_mechanical_design_sizes_the_vessel_from_the_default_standards",
        capture="process_separator_mechanical_design.tsv",
        block=1,
        identified_by=("#label", "port_fixture_pr"),
    ),
    LayerCase(
        model="process.rate_based_packed_column",
        case="co2_water_absorber",
        capture="process_rate_based_packed_column.tsv",
        block=0,
        identified_by=("#label", "co2_water_absorber"),
        # **The heat rate is the model's loosest layer, and it is bounded absolutely.**
        # Measured, the four segments differ from the capture by `1.955e-4` relative - the
        # largest of any dumped key, because the heat step's capacity rates go through the
        # *phase mass* (`n M`) and the class's own `getMass()` carries the flash's own rounding.
        # A band of 1 W on a rate of order 2731 W is `3.7e-4` relative and names the key rather
        # than loosening the case.
        diagnostic=(
            ("segment1_heat_transfer_rate_W", 1.0),
            ("segment2_heat_transfer_rate_W", 1.0),
            ("segment3_heat_transfer_rate_W", 1.0),
            ("segment4_heat_transfer_rate_W", 1.0),
        ),
        divergence=_PACKED_ABSORBER,
    ),
    LayerCase(
        model="process.rate_based_packed_column",
        case="heat_transfer_disabled",
        capture="process_rate_based_packed_column.tsv",
        block=6,
        identified_by=("#label", "heat_transfer_disabled"),
        divergence=_PACKED_HEAT_OFF,
    ),
    LayerCase(
        model="process.rate_based_packed_column",
        case="zero_packed_height",
        capture="process_rate_based_packed_column.tsv",
        block=7,
        identified_by=("#label", "zero_packed_height"),
        divergence=_PACKED_ZERO_HEIGHT,
    ),
    # **The second capture this model reads**, and the one that holds the other mass-transfer
    # correlation. `billet_onda` is the same state as `co2_water_absorber` on the same cubic,
    # so it is declared uncased and the pair's ratio is what the model's own test reads;
    # `billet_schultes` is the case, and it is what holds the two implementations to carrying
    # the value at all - Python refused it until `check_unported.py` was built.
    LayerCase(
        model="process.rate_based_packed_column",
        case="billet_schultes",
        capture="process_rate_based_billet.tsv",
        block=3,
        identified_by=("#label", "plastic_schultes"),
        # The same band the Onda row takes and for the same reason, measured on this row:
        # the four segments differ from the capture by between `1.6e-4` and `6.1e-4`
        # relative - the largest of any dumped key - because the heat step's capacity rates
        # go through the *phase mass* and the class's own `getMass()` carries the flash's
        # rounding. A watt on a rate of order 2731 W is `3.7e-4` relative.
        diagnostic=(
            ("segment1_heat_transfer_rate_W", 1.0),
            ("segment2_heat_transfer_rate_W", 1.0),
            ("segment3_heat_transfer_rate_W", 1.0),
            ("segment4_heat_transfer_rate_W", 1.0),
        ),
        divergence=_PACKED_BILLET,
    ),
    # The specification rows. **Four are cases and three are declared uncased**, and the split
    # is the measurement: a purity, a flow rate, a duty and a purity at the *bottom* location
    # converge, while a recovery specification does not converge in NeqSim at all, a reflux
    # ratio lands on a state this port's own flash cannot find, and a duty under a temperature
    # pin is inert.
    LayerCase(
        model="process.distillation_column",
        case="spec_top_purity",
        capture="process_column.tsv",
        block=4,
        identified_by=("#label", "spec_top_purity_0_98_methane"),
    ),
    LayerCase(
        model="process.distillation_column",
        case="spec_top_flow_rate",
        capture="process_column.tsv",
        block=6,
        identified_by=("#label", "spec_top_flow_rate_14000_mol_per_hour"),
    ),
    LayerCase(
        model="process.distillation_column",
        case="spec_top_duty",
        capture="process_column.tsv",
        block=9,
        identified_by=("#label", "spec_top_duty_minus_20000"),
    ),
    # **The one row at the other location**, whose secant drives the reboiler's temperature
    # rather than the condenser's - so it is the row that fails if a location is ever treated
    # as a field rather than the slot a specification sits in.
    LayerCase(
        model="process.distillation_column",
        case="spec_bottom_purity",
        capture="process_column.tsv",
        block=10,
        identified_by=("#label", "spec_bottom_purity_0_98_n_butane"),
    ),
    # **The second solve's row**, on the column capture's own last block: the same binary
    # column under `solver_type = "naphtali_sandholm"`, whose layer is the same profile read
    # off a mesh solve's own state. The ladder capture holds the identical row among the ten
    # strategies - a measurement, not an oracle, so it is not a case.
    LayerCase(
        model="process.distillation_column",
        case="binary_mesh_solve",
        capture="process_column.tsv",
        block=11,
        identified_by=("#label", "binary_methane_butane_mesh_solve"),
    ),
    # The absorber's two rows: the oracle - unpinned at `1e-4` K - and the class's own
    # isothermal case, which pins every tray and therefore stops after one sweep.
    LayerCase(
        model="process.absorption_column",
        case="lean_oil_absorber",
        capture="process_absorber.tsv",
        block=2,
        identified_by=("#label", "lean_oil_absorber"),
    ),
    LayerCase(
        model="process.stripping_column",
        case="hydrocarbon_stripper",
        capture="process_absorber.tsv",
        block=5,
        identified_by=("#label", "hydrocarbon_stripper"),
    ),
    # The column's converged rows. **Both are cases, and the second is there because a looser
    # gate is a different measurement** rather than a sloppier version of the first: where a
    # solve stops is what its answer is.
    LayerCase(
        model="process.distillation_column",
        case="binary_rigorous",
        capture="process_column.tsv",
        block=2,
        identified_by=("#label", "binary_methane_butane_4_stages"),
    ),
    LayerCase(
        model="process.distillation_column",
        case="binary_looser_gate",
        capture="process_column.tsv",
        block=3,
        identified_by=("#label", "binary_methane_butane_loose"),
    ),
    # The component splitter's three rows. The two enthalpies are declared divergences - see
    # `_COMPONENT_SPLITTER_DIVERGENCE` - and the flows, compositions and temperatures are
    # compared, which is what the class gets right.
    LayerCase(
        model="process.component_splitter",
        case="near_total_separation",
        capture="process_component_splitter.tsv",
        block=0,
        identified_by=("split_factors", "0.98 0.05 0.02"),
        divergence=_COMPONENT_SPLITTER_DIVERGENCE,
    ),
    LayerCase(
        model="process.component_splitter",
        case="an_even_routing_is_the_feed_twice",
        capture="process_component_splitter.tsv",
        block=1,
        identified_by=("split_factors", "0.5 0.5 0.5"),
        divergence=_COMPONENT_SPLITTER_EVEN_DIVERGENCE,
    ),
    LayerCase(
        model="process.component_splitter",
        case="all_of_one_component",
        capture="process_component_splitter.tsv",
        block=2,
        identified_by=("split_factors", "1.0 0.5 0.0"),
        divergence=_COMPONENT_SPLITTER_SEPARATION_DIVERGENCE,
    ),
    # The shortcut column's five rows. **Their block order is the probe's**, and the two
    # degenerate rows are blocks 2 and 6: `UNCASED_ROWS` declares them, and this tuple names
    # the other five by position.
    LayerCase(
        model="process.shortcut_distillation_column",
        case="propane_nbutane_split",
        capture="process_shortcut_distillation_column.tsv",
        block=0,
        identified_by=("#label", "propane_nbutane_300K_20bara"),
    ),
    LayerCase(
        model="process.shortcut_distillation_column",
        case="pressures_stated_move_the_product_phase",
        capture="process_shortcut_distillation_column.tsv",
        block=1,
        identified_by=("#label", "pressures_stated_18_21_bara"),
    ),
    LayerCase(
        model="process.shortcut_distillation_column",
        case="wilson_fallback_superheated_gas",
        capture="process_shortcut_distillation_column.tsv",
        block=3,
        identified_by=("#label", "wilson_fallback_superheated_gas"),
    ),
    LayerCase(
        model="process.shortcut_distillation_column",
        case="binary_methane_nbutane",
        capture="process_shortcut_distillation_column.tsv",
        block=4,
        identified_by=("#label", "binary_methane_nbutane"),
    ),
    # **The Kirkbride fix's own regression state**, whose material balance the class's test
    # writes by hand; its feed tray is `4` from the corrected product-basis correlation.
    LayerCase(
        model="process.shortcut_distillation_column",
        case="asymmetric_kirkbride_regression",
        capture="process_shortcut_distillation_column.tsv",
        block=5,
        identified_by=("#label", "asymmetric_kirkbride_regression"),
    ),
    # The isentropic pair. **Both rows of each are cases, and the efficiency-of-one row is
    # the one that matters**: its reversible step *is* its answer, so the capture states the
    # isentropic enthalpy the class keeps in a local - and its entropy production is zero by
    # construction, which is why it is compared on an absolute bound.
    LayerCase(
        model="process.compressor",
        case="the_reversible_limit_at_efficiency_one",
        capture="process_compressor.tsv",
        block=1,
        identified_by=("isentropic_efficiency", "1.0"),
        # The pump's bound and the pump's reason: 1e-9 kJ/(mol*K) is 1e-6 J/(mol*K), four
        # orders below any physical irreversibility and four above the double-precision
        # rounding of an enthalpy near 2.5e4. **Measured on these rows**: the compressor's two
        # sides are `-1.4e-17` and `1.4e-15`, and the expander's `-4.8e-12` and `1.4e-12` -
        # both zero, and neither of them equal.
        diagnostic=(("entropy_production_kJ_per_molK", 1e-9),),
    ),
    LayerCase(
        model="process.compressor",
        case="the_efficiency_divides_the_step",
        capture="process_compressor.tsv",
        block=0,
        identified_by=("isentropic_efficiency", "0.75"),
    ),
    LayerCase(
        model="process.expander",
        case="the_reversible_limit_at_efficiency_one",
        capture="process_expander.tsv",
        block=1,
        identified_by=("isentropic_efficiency", "1.0"),
        # The pump's bound and the pump's reason: 1e-9 kJ/(mol*K) is 1e-6 J/(mol*K), four
        # orders below any physical irreversibility and four above the double-precision
        # rounding of an enthalpy near 2.5e4. **Measured on these rows**: the compressor's two
        # sides are `-1.4e-17` and `1.4e-15`, and the expander's `-4.8e-12` and `1.4e-12` -
        # both zero, and neither of them equal.
        diagnostic=(("entropy_production_kJ_per_molK", 1e-9),),
    ),
    LayerCase(
        model="process.expander",
        case="the_efficiency_multiplies_the_step_and_not_divides_it",
        capture="process_expander.tsv",
        block=0,
        identified_by=("isentropic_efficiency", "0.75"),
    ),
    # The mechanical design's two branch states.
    LayerCase(
        model="process.expander",
        case="the_radial_design_is_the_capture",
        capture="process_expander_mechanical_design.tsv",
        block=0,
        identified_by=(LABEL_KEY, "radial_brake"),
    ),
    LayerCase(
        model="process.expander",
        case="the_axial_design_selects_the_generator",
        capture="process_expander_mechanical_design.tsv",
        block=2,
        identified_by=(LABEL_KEY, "axial_multistage"),
    ),
    # The recovered-power constraint's five states.
    LayerCase(
        model="process.expander",
        case="the_recovered_power_is_absent_without_a_rating",
        capture="process_expander_capacity.tsv",
        block=0,
        identified_by=("case", "no_rating"),
    ),
    LayerCase(
        model="process.expander",
        case="a_rating_above_the_recovered_power_is_its_utilization",
        capture="process_expander_capacity.tsv",
        block=1,
        identified_by=("case", "rating_2_kw"),
    ),
    LayerCase(
        model="process.expander",
        case="a_rating_inside_the_hard_band_is_not_exceeded",
        capture="process_expander_capacity.tsv",
        block=2,
        identified_by=("case", "rating_1_2_kw"),
    ),
    LayerCase(
        model="process.expander",
        case="a_rating_below_the_recovered_power_trips_both_flags",
        capture="process_expander_capacity.tsv",
        block=3,
        identified_by=("case", "rating_1_kw"),
    ),
    LayerCase(
        model="process.expander",
        case="a_non_positive_rating_is_no_constraint",
        capture="process_expander_capacity.tsv",
        block=4,
        identified_by=("case", "rating_zero"),
    ),
    # The pipe's three rows, one per branch and one that diverges. The water row is a case
    # *because the case records the divergence*, not because the model reproduces it: the
    # layers that moved there are the Reynolds number and the friction factor, and this is
    # where a reader sees by how much.
    # The manifold's three rows: two outlets, three, and one feed at zero flow. It is the
    # first id whose capture prints *both* of its composed models' rows - the mixture and
    # the branches - so the dumper reports both and the comparison is the composition.
    LayerCase(
        model="process.manifold",
        case="two_feeds_two_outlets",
        capture="process_manifold.tsv",
        block=0,
        identified_by=("split_factors", "0.25 0.75"),
    ),
    LayerCase(
        model="process.manifold",
        case="two_feeds_three_outlets",
        capture="process_manifold.tsv",
        block=1,
        identified_by=("split_factors", "0.2 0.3 0.5"),
    ),
    LayerCase(
        model="process.manifold",
        case="a_zero_flow_feed_is_dropped",
        capture="process_manifold.tsv",
        block=2,
        identified_by=("split_factors", "0.5 0.5"),
    ),
    LayerCase(
        model="process.pipe",
        case="gas_methane_co2_1000m",
        capture="process_pipe.tsv",
        block=0,
        identified_by=(LABEL_KEY, "gas_methane_co2_1000m"),
    ),
    LayerCase(
        model="process.pipe",
        case="liquid_n_butane_1000m",
        capture="process_pipe.tsv",
        block=1,
        identified_by=(LABEL_KEY, "liquid_n_butane_1000m"),
    ),
    # **The water row, which was three declared divergences and is now a case like the
    # others.** NeqSim gives an aqueous phase `WaterPhysicalProperties` and the liquid
    # `Viscosity` correlation, which `eos.aqueous_viscosity` ports; before that id existed
    # this port used the PFCT form its gas and oil branches take and was `1.61` out on the
    # Reynolds number and the drop. The row is kept because it is the measurement of that.
    LayerCase(
        model="process.pipe",
        case="water_1000m",
        capture="process_pipe.tsv",
        block=2,
        identified_by=(LABEL_KEY, "liquid_water_1000m"),
    ),
    LayerCase(
        model="process.pump",
        case="butane_5_to_20_bara_at_250_k",
        capture="process_pump.tsv",
        block=0,
        identified_by=("isentropic_efficiency", "0.75"),
    ),
    LayerCase(
        model="process.pump",
        case="the_work_divides_by_the_efficiency",
        capture="process_pump.tsv",
        block=1,
        identified_by=("isentropic_efficiency", "1.0"),
        # 1e-9 kJ/(mol*K) is 1e-6 J/(mol*K): four orders below any physical irreversibility
        # and four above the double-precision rounding of an enthalpy near 2.5e4 J/mol.
        diagnostic=(("entropy_production_kJ_per_molK", 1e-9),),
    ),
    # The mechanical design's non-chart states. The chart and vendor-NPSH blocks are
    # uncased here and held by their model cases instead: the dumper's state is the
    # isentropic branch, and the chart's power is `Q dP / eta` rather than `n dh`.
    LayerCase(
        model="process.pump",
        case="the_plain_design_is_the_capture",
        capture="process_pump_mechanical_design.tsv",
        block=0,
        identified_by=(LABEL_KEY, "plain_eta_0_75"),
    ),
    LayerCase(
        model="process.pump",
        case="the_multistage_design_is_the_capture",
        capture="process_pump_mechanical_design.tsv",
        block=2,
        identified_by=(LABEL_KEY, "multistage"),
    ),
    LayerCase(
        model="process.pump",
        case="the_purchaser_design_states_its_basis",
        capture="process_pump_mechanical_design.tsv",
        block=3,
        identified_by=(LABEL_KEY, "purchaser"),
    ),
    LayerCase(
        model="process.mixer",
        case="two_feeds_at_different_pressures",
        capture="process_mixer.tsv",
        block=0,
        identified_by=("specified_outlet_pressure_bara", "null"),
    ),
    LayerCase(
        model="process.mixer",
        case="a_stated_outlet_pressure_overrides_the_lowest_feed",
        capture="process_mixer.tsv",
        block=1,
        identified_by=("specified_outlet_pressure_bara", "8.0"),
    ),
    LayerCase(
        model="process.mixer",
        case="a_sub_tolerance_spread_raises_no_flag",
        capture="process_mixer.tsv",
        block=2,
        identified_by=("feed1_P", "9.6"),
    ),
    # The splitter's two rows carry the same numbers, so the pairing is the label's and
    # not a state's.
    LayerCase(
        model="process.splitter",
        case="a_two_way_split_of_a_liquid",
        capture="process_splitter.tsv",
        block=0,
        identified_by=("split_factors", "0.3 0.7"),
    ),
    LayerCase(
        model="process.splitter",
        case="the_factors_are_normalised",
        capture="process_splitter.tsv",
        block=1,
        identified_by=("split_factors", "3.0 7.0"),
    ),
    # The capacity map's nine branches, in the probe's own order. The pairing is the
    # block's `case=` key, which is what the probe prints first.
    LayerCase(
        model="process.splitter",
        case="the_capacity_map_is_absent_until_a_setter_creates_it",
        capture="process_splitter_capacity.tsv",
        block=0,
        identified_by=("case", "no_setters"),
    ),
    LayerCase(
        model="process.splitter",
        case="the_header_diameter_alone_does_not_initialize_the_map",
        capture="process_splitter_capacity.tsv",
        block=1,
        identified_by=("case", "header_alone"),
    ),
    LayerCase(
        model="process.splitter",
        case="enabling_analysis_initializes_the_class_defaults",
        capture="process_splitter_capacity.tsv",
        block=2,
        identified_by=("case", "enabled_defaults"),
    ),
    LayerCase(
        model="process.splitter",
        case="a_sized_header_and_a_stated_drop_report_both_constraints",
        capture="process_splitter_capacity.tsv",
        block=3,
        identified_by=("case", "sized_header_design_drop"),
    ),
    LayerCase(
        model="process.splitter",
        case="a_tight_velocity_is_exceeded_with_analysis_disabled",
        capture="process_splitter_capacity.tsv",
        block=4,
        identified_by=("case", "tight_velocity_disabled"),
    ),
    LayerCase(
        model="process.splitter",
        case="a_non_positive_drop_suppresses_only_its_own_constraint",
        capture="process_splitter_capacity.tsv",
        block=5,
        identified_by=("case", "zero_design_drop"),
    ),
    LayerCase(
        model="process.splitter",
        case="a_non_positive_velocity_suppresses_only_its_own_constraint",
        capture="process_splitter_capacity.tsv",
        block=6,
        identified_by=("case", "zero_max_velocity"),
    ),
    LayerCase(
        model="process.splitter",
        case="the_utilization_is_capped_at_9_99",
        capture="process_splitter_capacity.tsv",
        block=7,
        identified_by=("case", "cap_9_99"),
    ),
    LayerCase(
        model="process.splitter",
        case="the_velocity_reads_the_two_phase_feeds_own_volume_flow",
        capture="process_splitter_capacity.tsv",
        block=8,
        identified_by=("case", "two_phase_one_outlet"),
    ),
    # The mechanical design's nine branches, in the probe's own order.
    LayerCase(
        model="process.splitter",
        case="the_default_design_sizes_at_fifteen_metres_per_second",
        capture="process_splitter_mechanical_design.tsv",
        block=0,
        identified_by=("case", "default_design"),
    ),
    LayerCase(
        model="process.splitter",
        case="a_larger_flow_rounds_the_header_to_one_inch",
        capture="process_splitter_mechanical_design.tsv",
        block=1,
        identified_by=("case", "twenty_mol"),
    ),
    LayerCase(
        model="process.splitter",
        case="a_lower_design_velocity_rounds_the_header_up",
        capture="process_splitter_mechanical_design.tsv",
        block=2,
        identified_by=("case", "design_velocity_ten"),
    ),
    LayerCase(
        model="process.splitter",
        case="the_operating_pressure_thickens_the_wall",
        capture="process_splitter_mechanical_design.tsv",
        block=3,
        identified_by=("case", "high_pressure"),
    ),
    LayerCase(
        model="process.splitter",
        case="the_tensile_strength_thickens_the_wall",
        capture="process_splitter_mechanical_design.tsv",
        block=4,
        identified_by=("case", "low_tensile"),
    ),
    LayerCase(
        model="process.splitter",
        case="the_corrosion_allowance_is_added_to_the_wall",
        capture="process_splitter_mechanical_design.tsv",
        block=5,
        identified_by=("case", "corrosion"),
    ),
    LayerCase(
        model="process.splitter",
        case="the_joint_efficiency_is_read_and_discarded",
        capture="process_splitter_mechanical_design.tsv",
        block=6,
        identified_by=("case", "joint_efficiency"),
    ),
    LayerCase(
        model="process.splitter",
        case="the_two_phase_design_reads_the_system_density",
        capture="process_splitter_mechanical_design.tsv",
        block=7,
        identified_by=("case", "two_phase"),
    ),
    LayerCase(
        model="process.splitter",
        case="the_velocity_reads_the_designed_header",
        capture="process_splitter_mechanical_design.tsv",
        block=8,
        identified_by=("case", "design_with_capacity"),
    ),
    # The heater's outlet-temperature constraints, in the probe's own order. The heater's
    # outlet record is the flash's answer; the capacity utilisations are the interior.
    LayerCase(
        model="process.heater",
        case="the_outlet_temperature_limits_are_absent_without_setters",
        capture="process_heater_capacity.tsv",
        block=0,
        identified_by=("case", "no_limits"),
    ),
    LayerCase(
        model="process.heater",
        case="a_maximum_above_the_outlet_is_its_utilization",
        capture="process_heater_capacity.tsv",
        block=1,
        identified_by=("case", "max_above"),
    ),
    LayerCase(
        model="process.heater",
        case="a_maximum_below_the_outlet_is_exceeded_but_not_hard",
        capture="process_heater_capacity.tsv",
        block=2,
        identified_by=("case", "max_below"),
    ),
    LayerCase(
        model="process.heater",
        case="a_minimum_below_the_outlet_is_its_inverse_utilization",
        capture="process_heater_capacity.tsv",
        block=3,
        identified_by=("case", "min_below"),
    ),
    LayerCase(
        model="process.heater",
        case="a_minimum_above_the_outlet_is_hard_exceeded",
        capture="process_heater_capacity.tsv",
        block=4,
        identified_by=("case", "min_above"),
    ),
    LayerCase(
        model="process.heater",
        case="both_limits_report_both_constraints_and_the_larger_bottleneck",
        capture="process_heater_capacity.tsv",
        block=5,
        identified_by=("case", "both"),
    ),
    LayerCase(
        model="process.heater",
        case="a_minimum_of_zero_is_not_a_constraint",
        capture="process_heater_capacity.tsv",
        block=6,
        identified_by=("case", "min_zero"),
    ),
    LayerCase(
        model="process.heater",
        case="the_utilization_is_capped_at_9_99",
        capture="process_heater_capacity.tsv",
        block=7,
        identified_by=("case", "min_capped"),
    ),
    LayerCase(
        model="process.heater",
        case="a_duty_specified_outlet_reads_its_flashed_temperature",
        capture="process_heater_capacity.tsv",
        block=8,
        identified_by=("case", "duty_specified"),
    ),
    # **The reactive section's block**, on the binary state the model's own case is held to:
    # NeqSim's reactive column there is bit-identical to its standard twin, which is the class's
    # own `NR = 0` claim. It sits in this capture because the layer diff pairs one capture with
    # one model, and this model's case is two blocks above.
    LayerCase(
        model="process.distillation_column",
        case="reactive_section_is_the_delegations_own",
        capture="process_column.tsv",
        block=13,
        identified_by=("#label", "binary_reactive_column_pr"),
    ),
    # **The side-draw block**, four below the reactive one and the last state in this capture: the
    # binary column with tray 3 drawing a quarter of its vapour. The three blocks above it are the
    # class's own one-tray columns, which this port refuses, and the block below it is the same
    # column drawing liquid and a pumparound, which NeqSim reconciles - see `UNCASED_ROWS`.
    LayerCase(
        model="process.distillation_column",
        case="a_tray_withdraws_a_quarter_of_its_vapour",
        capture="process_column.tsv",
        block=17,
        identified_by=("#label", "side_draw_column_binary_gas_quarter"),
    ),
    # **The Murphree rows, on the capture's last four blocks.** `0.6` is the oracle - the port
    # reproduces its whole profile to about `1e-6` - and `0_85` is NeqSim's state that the port
    # does *not* reproduce, by up to `7` K on tray 3. **The cause is not established**:
    # `applyRelaxationFast` - the obvious guess, since the class relaxes the streams and this
    # port does not - was implemented and measured on both rows, and it moves neither, so the
    # difference is in the maps rather than in the paths and it is not localised. The two
    # `_tight` rows are the measurement that both are *converged* states rather than partial
    # ones, and they are declared uncased in `UNCASED_ROWS`.
    LayerCase(
        model="process.distillation_column",
        case="murphree_0_6",
        capture="process_column.tsv",
        block=19,
        identified_by=("#label", "binary_murphree_0_6"),
        divergence=_MURPHREE_DIVERGENCE,
    ),
    # The packed column's two cases sit on blocks 12 and 13. **Its first block is the base
    # column's own `binary_rigorous` state**, because the packing does not change the separation:
    # NeqSim's `PackedColumn` at `2.0` m and its `DistillationColumn` at four stages report
    # bit-identical answers on all twenty-two captured quantities. The second is the same feed at
    # `2.3` m, where `ceil(4.6)` makes it a five-stage column instead - so it is the row that
    # fails if the height stops moving the stage count. See `UNCASED_ROWS` for the other thirteen.
    # **The pumparound's return**, a labelled substitution: the class's own pumparound state
    # lives on a column that also carries a reflux ratio, a boilup ratio, a `MESH_RESIDUAL`
    # solve and a side draw, so the recycle cannot be separated from them there - and the
    # binary column with `addLiquidPumparound("PA", 1, 3, 0.10, 5.0)` on it is what the port is
    # held to instead.
    LayerCase(
        model="process.distillation_column",
        case="pumparound_return_recycles_until_its_flow_settles",
        capture="process_column.tsv",
        block=23,
        identified_by=("#label", "side_draw_pumparound_return_binary"),
    ),
    # **The side-draw flow tear**, whose oracle is the substitution described in
    # `UNCASED_ROWS`: one tray, no ends, a gas draw specified at 25 kg/hr, and the class's own
    # candidate search landing on it in two passes.
    LayerCase(
        model="process.distillation_column",
        case="side_draw_flow_one_tray_gas_binary",
        capture="process_column.tsv",
        block=25,
        identified_by=("#label", "side_draw_flow_one_tray_gas_binary"),
    ),
    LayerCase(
        model="process.packed_column",
        case="packed_binary_2m",
        capture="process_packed_column.tsv",
        block=12,
        identified_by=(LABEL_KEY, "packed_distillation_binary_2m"),
    ),
    # **The reactive packed column, whose answer is the row above's own.** `reactive = true`
    # reaches the four middle trays and changes nothing, because this fluid has no independent
    # reaction - so the case is the *identity*, and it exists because a flag that silently did
    # nothing would otherwise look exactly like one that worked.
    LayerCase(
        model="process.packed_column",
        case="packed_binary_2m_reactive_is_the_delegations_own",
        capture="process_packed_column.tsv",
        block=15,
        identified_by=(LABEL_KEY, "packed_distillation_binary_2m_reactive"),
    ),
    LayerCase(
        model="process.packed_column",
        case="packed_binary_2m3_is_one_stage_more",
        capture="process_packed_column.tsv",
        block=13,
        identified_by=(LABEL_KEY, "packed_distillation_binary_2m3"),
    ),
    # The exchanger's five cases sit on blocks 0, 1, 2, 5 and 6. Blocks 3 and 4 are
    # NeqSim's own `runSpecifiedStream` rows, which the port deliberately does not
    # reproduce - see `UNCASED_ROWS`.
    LayerCase(
        model="process.heat_exchanger",
        case="the_effectiveness_ntu_rating",
        capture="process_heat_exchanger.tsv",
        block=0,
        identified_by=(LABEL_KEY, "ua_rating_counterflow"),
    ),
    LayerCase(
        model="process.heat_exchanger",
        case="the_rating_answers_the_flow",
        capture="process_heat_exchanger.tsv",
        block=1,
        identified_by=(LABEL_KEY, "ua_rating_hot_flow_2"),
    ),
    LayerCase(
        model="process.heat_exchanger",
        case="the_arrangement_moves_less_heat",
        capture="process_heat_exchanger.tsv",
        block=2,
        identified_by=(LABEL_KEY, "ua_rating_parallelflow"),
    ),
    LayerCase(
        model="process.heat_exchanger",
        case="a_pinned_hot_outlet",
        capture="process_heat_exchanger.tsv",
        block=5,
        identified_by=(LABEL_KEY, "reference_pin_hot_350"),
    ),
    LayerCase(
        model="process.heat_exchanger",
        case="a_pinned_cold_outlet",
        capture="process_heat_exchanger.tsv",
        block=6,
        identified_by=(LABEL_KEY, "reference_pin_cold_320"),
    ),
    # **The two reactors a total said nothing about.** The plug-flow reactor's `March` says
    # "which `layers` dumps" in its own docstring, and nothing did: the model was ported with
    # its instrument promised and unwritten, and the Gibbs reactor beside it. The captures
    # existed the whole time - `process_plug_flow_reactor.tsv` and `process_gibbs_reactor.tsv`
    # - so what was missing was a dumper and not an oracle, and each case below is the probe
    # row whose settings its inputs are.
    LayerCase(
        model="process.plug_flow_reactor",
        case="hundred_steps",
        capture="process_plug_flow_reactor.tsv",
        block=0,
        identified_by=(LABEL_KEY, "rk4_default"),
        diagnostic=(("heat_duty_W", 1.0),),
    ),
    LayerCase(
        model="process.plug_flow_reactor",
        case="isothermal",
        capture="process_plug_flow_reactor.tsv",
        block=4,
        identified_by=(LABEL_KEY, "isothermal_rk4"),
        diagnostic=(("heat_duty_W", 1.0),),
    ),
    LayerCase(
        model="process.plug_flow_reactor",
        case="catalyst_bed",
        capture="process_plug_flow_reactor.tsv",
        block=5,
        identified_by=(LABEL_KEY, "bed_rk4"),
        diagnostic=(("heat_duty_W", 1.0),),
    ),
    LayerCase(
        model="process.plug_flow_reactor",
        case="frozen_ten_steps",
        capture="process_plug_flow_reactor.tsv",
        block=7,
        identified_by=(LABEL_KEY, "rk4_ten_steps"),
        # **The duty is zero by construction on this row** - the case is adiabatic and the
        # probe's own `heat_duty_W` is `0.0` - so a ratio has nothing to divide. A bound of
        # 1 W on a duty of order 10 kW says "both are zero" rather than loosening the case.
        diagnostic=(("heat_duty_W", 1.0),),
    ),
    LayerCase(
        model="process.plug_flow_reactor",
        case="euler_ten_steps",
        capture="process_plug_flow_reactor.tsv",
        block=8,
        identified_by=(LABEL_KEY, "euler_ten_steps"),
        diagnostic=(("heat_duty_W", 1.0),),
    ),
    LayerCase(
        model="process.gibbs_reactor",
        case="reverse_water_gas_shift_at_1000_k",
        capture="process_gibbs_reactor.tsv",
        block=6,
        identified_by=(LABEL_KEY, "rwgs_1000k"),
    ),
    LayerCase(
        model="process.gibbs_reactor",
        case="reverse_water_gas_shift_at_1200_k",
        capture="process_gibbs_reactor.tsv",
        block=5,
        identified_by=(LABEL_KEY, "rwgs_1200k"),
    ),
    # The calculator's three states: two recycles below the control line at 0.05 and 0.10
    # margins, and one natural flow above it. **The block's machine state is the layer**, and
    # the inactive row's reconstruction is the natural suction, which is what `solve` leaves.
    LayerCase(
        model="process.anti_surge_recycle",
        case="the_recycle_holds_the_margin_at_five_percent",
        capture="process_anti_surge_recycle.tsv",
        block=0,
        identified_by=(LABEL_KEY, "low_flow_margin_005"),
    ),
    LayerCase(
        model="process.anti_surge_recycle",
        case="a_natural_flow_above_the_control_line_opens_nothing",
        capture="process_anti_surge_recycle.tsv",
        block=1,
        identified_by=(LABEL_KEY, "high_flow_no_recycle"),
    ),
    LayerCase(
        model="process.anti_surge_recycle",
        case="a_ten_percent_margin_moves_the_converged_state",
        capture="process_anti_surge_recycle.tsv",
        block=2,
        identified_by=(LABEL_KEY, "low_flow_margin_010"),
    ),
    # The blowdown profile's eleven states; the six refusals are in `UNCASED_ROWS`.
    LayerCase(
        model="process.blowdown_pressure_profile",
        case="step_exact",
        capture="process_blowdown_pressure_profile.tsv",
        block=0,
        identified_by=(LABEL_KEY, "blowdown_step_exact"),
    ),
    LayerCase(
        model="process.blowdown_pressure_profile",
        case="step_between",
        capture="process_blowdown_pressure_profile.tsv",
        block=1,
        identified_by=(LABEL_KEY, "blowdown_step_between"),
    ),
    LayerCase(
        model="process.blowdown_pressure_profile",
        case="clamp_before",
        capture="process_blowdown_pressure_profile.tsv",
        block=2,
        identified_by=(LABEL_KEY, "blowdown_step_clamp_before"),
    ),
    LayerCase(
        model="process.blowdown_pressure_profile",
        case="clamp_after",
        capture="process_blowdown_pressure_profile.tsv",
        block=3,
        identified_by=(LABEL_KEY, "blowdown_step_clamp_after"),
    ),
    LayerCase(
        model="process.blowdown_pressure_profile",
        case="linear_between",
        capture="process_blowdown_pressure_profile.tsv",
        block=4,
        identified_by=(LABEL_KEY, "blowdown_linear_between"),
    ),
    LayerCase(
        model="process.blowdown_pressure_profile",
        case="near_equal_upper_step",
        capture="process_blowdown_pressure_profile.tsv",
        block=5,
        identified_by=(LABEL_KEY, "blowdown_step_near_equal_upper"),
    ),
    LayerCase(
        model="process.blowdown_pressure_profile",
        case="near_equal_lower_step",
        capture="process_blowdown_pressure_profile.tsv",
        block=6,
        identified_by=(LABEL_KEY, "blowdown_step_near_equal_lower"),
    ),
    LayerCase(
        model="process.blowdown_pressure_profile",
        case="near_equal_upper_linear",
        capture="process_blowdown_pressure_profile.tsv",
        block=7,
        identified_by=(LABEL_KEY, "blowdown_linear_near_equal_upper"),
    ),
    LayerCase(
        model="process.blowdown_pressure_profile",
        case="near_equal_lower_linear",
        capture="process_blowdown_pressure_profile.tsv",
        block=8,
        identified_by=(LABEL_KEY, "blowdown_linear_near_equal_lower"),
    ),
    LayerCase(
        model="process.blowdown_pressure_profile",
        case="gauge_offset_two",
        capture="process_blowdown_pressure_profile.tsv",
        block=9,
        identified_by=(LABEL_KEY, "blowdown_gauge_offset_two"),
    ),
    LayerCase(
        model="process.blowdown_pressure_profile",
        case="gauge_offset_zero",
        capture="process_blowdown_pressure_profile.tsv",
        block=10,
        identified_by=(LABEL_KEY, "blowdown_gauge_offset_zero"),
    ),
    # The material's twelve property states; the six aliases and eight refusals are in
    # `UNCASED_ROWS`.
    LayerCase(
        model="process.pipe_fire_rupture_material",
        case="duplex_22cr_600c",
        capture="process_pipe_fire_rupture_material.tsv",
        block=0,
        identified_by=(LABEL_KEY, "material_22cr_duplex"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture_material",
        case="ss_316_600c",
        capture="process_pipe_fire_rupture_material.tsv",
        block=1,
        identified_by=(LABEL_KEY, "material_ss_316"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture_material",
        case="cs_235_600c",
        capture="process_pipe_fire_rupture_material.tsv",
        block=2,
        identified_by=(LABEL_KEY, "material_cs_235_asme_a106_b"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture_material",
        case="cs_360_x52_600c",
        capture="process_pipe_fire_rupture_material.tsv",
        block=3,
        identified_by=(LABEL_KEY, "material_cs_360_api_5l_x52"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture_material",
        case="superduplex_600c",
        capture="process_pipe_fire_rupture_material.tsv",
        block=4,
        identified_by=(LABEL_KEY, "material_superduplex"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture_material",
        case="6mo_600c",
        capture="process_pipe_fire_rupture_material.tsv",
        block=5,
        identified_by=(LABEL_KEY, "material_6mo"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture_material",
        case="duplex_22cr_800c",
        capture="process_pipe_fire_rupture_material.tsv",
        block=6,
        identified_by=(LABEL_KEY, "material_22cr_800c"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture_material",
        case="duplex_22cr_clamp_low",
        capture="process_pipe_fire_rupture_material.tsv",
        block=7,
        identified_by=(LABEL_KEY, "material_22cr_clamp_low"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture_material",
        case="duplex_22cr_clamp_high",
        capture="process_pipe_fire_rupture_material.tsv",
        block=8,
        identified_by=(LABEL_KEY, "material_22cr_clamp_high"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture_material",
        case="custom_50c",
        capture="process_pipe_fire_rupture_material.tsv",
        block=9,
        identified_by=(LABEL_KEY, "material_custom_50c"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture_material",
        case="custom_clamp_low",
        capture="process_pipe_fire_rupture_material.tsv",
        block=10,
        identified_by=(LABEL_KEY, "material_custom_clamp_low"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture_material",
        case="custom_clamp_high",
        capture="process_pipe_fire_rupture_material.tsv",
        block=11,
        identified_by=(LABEL_KEY, "material_custom_clamp_high"),
    ),
    # The pipe march's ten state blocks; the ten refusals are in `UNCASED_ROWS`.
    LayerCase(
        model="process.pipe_fire_rupture",
        case="benchmark",
        capture="process_pipe_fire_rupture.tsv",
        block=0,
        identified_by=(LABEL_KEY, "pipe_rupture_benchmark"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture",
        case="no_rupture_10s",
        capture="process_pipe_fire_rupture.tsv",
        block=1,
        identified_by=(LABEL_KEY, "pipe_no_rupture_10s"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture",
        case="small_jet_builtin",
        capture="process_pipe_fire_rupture.tsv",
        block=2,
        identified_by=(LABEL_KEY, "pipe_small_jet_builtin"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture",
        case="explicit_small_jet",
        capture="process_pipe_fire_rupture.tsv",
        block=3,
        identified_by=(LABEL_KEY, "pipe_explicit_small_jet"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture",
        case="ss_316",
        capture="process_pipe_fire_rupture.tsv",
        block=4,
        identified_by=(LABEL_KEY, "pipe_ss316"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture",
        case="liquid",
        capture="process_pipe_fire_rupture.tsv",
        block=5,
        identified_by=(LABEL_KEY, "pipe_liquid"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture",
        case="gas_thermal_mass_off",
        capture="process_pipe_fire_rupture.tsv",
        block=6,
        identified_by=(LABEL_KEY, "pipe_gas_thermal_mass_off"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture",
        case="gas_thermal_mass_on",
        capture="process_pipe_fire_rupture.tsv",
        block=7,
        identified_by=(LABEL_KEY, "pipe_gas_thermal_mass_on"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture",
        case="pool_fire",
        capture="process_pipe_fire_rupture.tsv",
        block=8,
        identified_by=(LABEL_KEY, "pipe_pool_fire"),
    ),
    LayerCase(
        model="process.pipe_fire_rupture",
        case="pfp_factor",
        capture="process_pipe_fire_rupture.tsv",
        block=9,
        identified_by=(LABEL_KEY, "pipe_pfp_01"),
    ),
    # The vessel's six states; the fourteen-grade sweep and the ten evidence rows are in
    # `UNCASED_ROWS`.
    LayerCase(
        model="process.vessel_rupture",
        case="severe_x65",
        capture="process_vessel_rupture.tsv",
        block=0,
        identified_by=(LABEL_KEY, "vessel_severe_x65"),
    ),
    LayerCase(
        model="process.vessel_rupture",
        case="mild_x65",
        capture="process_vessel_rupture.tsv",
        block=1,
        identified_by=(LABEL_KEY, "vessel_mild_x65"),
    ),
    LayerCase(
        model="process.vessel_rupture",
        case="lpg_bare",
        capture="process_vessel_rupture.tsv",
        block=2,
        identified_by=(LABEL_KEY, "vessel_lpg_bare"),
    ),
    LayerCase(
        model="process.vessel_rupture",
        case="lpg_insulated",
        capture="process_vessel_rupture.tsv",
        block=3,
        identified_by=(LABEL_KEY, "vessel_lpg_insulated"),
    ),
    LayerCase(
        model="process.vessel_rupture",
        case="x90_one_row",
        capture="process_vessel_rupture.tsv",
        block=18,
        identified_by=(LABEL_KEY, "vessel_x90_one_row"),
    ),
    LayerCase(
        model="process.vessel_rupture",
        case="l450_alias",
        capture="process_vessel_rupture.tsv",
        block=19,
        identified_by=(LABEL_KEY, "vessel_l450_alias"),
    ),
)

#: Probe rows that are **deliberately not cases**, per capture, by the count they take.
#:
#: A probe row and a case are not the same thing: a row can be evidence for a divergence
#: rather than an oracle for a state the port reproduces. The exchanger's two
#: `out_temperature_pins_*` rows are that - NeqSim's own `runSpecifiedStream` lands on an
#: enthalpy its own `PHflash` places at another temperature, so the two pinned cases are
#: held to the `reference_pin_*` rows instead. Counting them here rather than leaving the
#: block count open keeps the check exact: a probe row added without a case still fails.
UNCASED_ROWS: dict[str, int] = {
    # The dispersion capture's five: four SRK rows, the class's own tests' cubic kept as
    # evidence beside the PR oracle, and the zero-rate refusal the port raises rather than
    # reproduces.
    "process_gas_dispersion.tsv": 5,
    # The blocked-outlet capture's five: three raw `ReliefFluidState` blocks, the class's
    # no-fluid branch, and the negative inflow the port refuses.
    "process_blocked_outlet_relief.tsv": 5,
    # The control-valve capture's two: the raw state block and the no-fluid branch.
    "process_control_valve_failure_relief.tsv": 2,
    # The check-valve capture's two: the raw state block and the no-fluid branch.
    "process_check_valve_leak_relief.tsv": 2,
    # The tube-rupture capture's three: the raw state block, the class's zero-end reset
    # (which the port refuses) and the no-fluid branch.
    "process_tube_rupture_relief.tsv": 3,
    # The study's two: the no-credible and empty blocks carry no numeric output, so they
    # are held by their cases and the cross-impl rather than the layer diff.
    "process_overpressure_protection_study.tsv": 2,
    # The class's `setKinetics` row: the identity it records is the caller's multiplication,
    # which this id does not take as an input.
    "process_porous_media_self_heating.tsv": 1,
    # The cube the transient solver refuses: not a shape this id declares.
    "process_self_heating_induction.tsv": 1,
    # The two fits the class refuses: a single point and equal temperatures.
    "process_basket_test_regression.tsv": 2,
    # The template's ten: the one-parameter grid, the empty grid, the manual row and the
    # seven equipment seed blocks. The rows are strings; the cases hold them.
    "process_hazop_template.tsv": 10,
    # The worksheet's six: the inclusive-threshold block, the empty worksheet and the four
    # refusals. The empty list is not a case state.
    "process_fmea_worksheet.tsv": 6,
    # The population's nine: the whole catalogue, the flow sweep, the two lookups, the
    # separator, valve, stream and preserved nodes and the empty node.
    "process_hazop_consequence_population.tsv": 9,
    # The dense gas's six: the constructor's four guards and the ratio's two.
    "process_heavy_gas_dispersion.tsv": 6,
    # The hazardous area's nine: the constructor's six guards and the setters' three.
    "process_hazardous_area.tsv": 9,
    # The toxic library's twenty-four: five aliases, ten case-failure states, two
    # case-resolved duplicates, three unknown names and four unguarded evidence rows.
    "process_toxic_library.tsv": 24,
    # The blowdown profile's six: the point-count, time, pressure, query and offset
    # refusals.
    "process_blowdown_pressure_profile.tsv": 6,
    # The material curve's fourteen: the six resolved-name aliases and the eight
    # constructor refusals.
    "process_pipe_fire_rupture_material.tsv": 14,
    # The pipe march's ten: the weld-factor pair, the two geometry refusals, the zero
    # diameter, the unknown material and scenario, the one-point profile, the zero step and
    # the negative maximum time.
    "process_pipe_fire_rupture.tsv": 10,
    # The vessel's twenty-four: the fourteen-grade sweep, the nine refusals and the
    # non-monotonic evidence block the class accepts and the port refuses.
    "process_vessel_rupture.tsv": 24,
    # The event tree's four: the no-branch tree, the two refusals and the shift count, which
    # cannot be cased (an empty record value is not a case).
    "process_event_tree.tsv": 4,
    # The consequence engine's two: the class's infinite flux at zero distance, and the
    # negative branch probability it throws on.
    "process_consequence_analysis.tsv": 2,
    # The jet fire's three: the zero-humidity `NaN`, the infinite zero-distance flux and the
    # zero-target refusal, each the measurement its guard rests on.
    "process_jet_fire.tsv": 3,
    # The pool fire's four: the two unreachable targets the class answers with its near
    # bound, the zero-target far bound, and the dead-parameter state.
    "process_pool_fire.tsv": 4,
    # The PFP demand's nine: the two non-physical temperature states the port refuses, and
    # the class's seven constructor and setter refusals.
    "process_pfp_demand.tsv": 9,
    # The fire-water demand's seven: the accepted negative object, and the class's
    # constructor, setter and vessel refusals.
    "process_fire_water_demand.tsv": 7,
    # The deluge layout's eight: the class's constructor, pressure, efficiency and nozzle
    # refusals.
    "process_deluge_nozzle_layout.tsv": 8,
    # The monitor coverage's eleven: the constructor and each setter's refusal.
    "process_fire_monitor_coverage.tsv": 11,
    # The coverage assessment's six: the unset-supply state, where the class's NaN defaults
    # read as feasible, and the constructor and setter refusals.
    "process_fire_water_coverage_assessment.tsv": 6,
    # The protection screening's five: the class's NaN times, the negative demand its setter
    # accepts, and the three constructor refusals.
    "process_active_fire_protection_screening.tsv": 5,
    # The flare flame's nine: the three clamp states, the negative stack the class accepts,
    # and the five constructor and method refusals.
    "process_api537_flare_flame.tsv": 9,
    # The fault tree's fourteen: beta on VOTING, k of zero, above n and negative, the two
    # empty gates, duplicate names, a NaN basic, the 31-basic shift overflow, the zero
    # cardinality and the four constructor guards.
    "process_fault_tree.tsv": 14,
    # The bow-tie's eight: the empty bow-tie, no barriers, no consequences, all threats
    # inactive, a negative frequency, a PFD above one, a negative PFD and a NaN.
    "process_bow_tie.tsv": 8,
    # The SIF's five: a negative PFD, a NaN, the SIL 0 and SIL 5 refusals, and the static
    # max-PFD table for a SIL it does not know.
    "process_safety_instrumented_function.tsv": 5,
    # The LOPA's thirteen: the floating-point boundary, the six STS0131 bands and their
    # boundaries, a zero-PFD layer, a negative initiating frequency, and the four STS0131
    # refusals (a zero pressure three times and a test below design).
    "process_lopa.tsv": 13,
    # The SIL verification's six: 1oo3, the `4oo2` substring, a ten-year proof interval,
    # exactly five years, a NaN PFD and a negative PFD.
    "process_sil_verification.tsv": 6,
    # The NOG 070's eight: the PFD-to-SIL boundary sweep, the full 18-row catalogue and
    # the six guards (a zero PFD, one, above one, a NaN, and two minimum-SIL bounds).
    "process_nog070_sil_determination.tsv": 8,
    # The risk matrix's six: a negative frequency, a NaN frequency, a loss above 100 %,
    # a negative loss, a negative repair time and a negative mass loss.
    "process_risk_matrix.tsv": 6,
    # The PSV's six: a back pressure above the relieving pressure in both the liquid and
    # two-phase branches, a zero mass flow and a zero liquid volume the class refuses, and
    # the exponent of one and zero liquid density it accepts.
    "process_psv_sizing.tsv": 6,
    # The fire sizing's three: two rows differing only in the dead initial pressure and
    # volume, and the zero blowdown time the class accepts as an infinite average flow.
    "process_psv_fire_sizing.tsv": 3,
    # The fire relief's one: the negative latent heat the class silently replaces with
    # 350000 J/kg.
    "process_fire_case_relief.tsv": 1,
    # The disposal network's two: the empty network, which the schema cannot carry as a
    # case, and the negative rate the class subtracts.
    "process_relief_disposal_network.tsv": 2,
    # The TR3001 map's one: the empty scenario list, whose two checks fail.
    "process_tr3001_compliance.tsv": 1,
    # The VCE's three: two unreachable targets the class answers with its near bound, and
    # the zero-target refusal.
    "process_vce.tsv": 3,
    # The BLEVE's five: the zero-humidity NaN, the unreachable target's near bound, the
    # negative radiative fraction, the infinite zero-distance flux and the zero target.
    "process_bleve.tsv": 5,
    # The probit's one: the non-positive-dose guard, whose `-Infinity` is the measurement
    # the refusal rests on.
    "process_probit.tsv": 1,
    # The separator capacity capture's eight. **Three are the cases' own states under another
    # stated setting** - the vertical vessel at the default `0.8`, and the horizontal one at
    # `0.2` - kept as the measurement the design-liquid fraction moves a horizontal area by a
    # quarter and a vertical one by nothing. **One is the pin's
    # near-dry scrubber state, which is single-phase in this jar**: it exercises the
    # `1000.0` fallback and not the `10.0` floor, so it is evidence for the floor's
    # unreachability rather than an oracle for a floored state. **Four are SRK**, the cubic
    # `SeparatorTest` itself uses: the port is PR, so they measure what the cubic moved.
    # **The last is `GasScrubber` at minimum setup**, which prints the class defaults this
    # id does not adopt - `vertical`, `0.1`, and the `0.11` m/s `K` it inherits.
    "process_separator_capacity.tsv": 8,
    # The sizing capture's five: the level-`0.2` row repeats the horizontal case under a
    # stated design level, the two near-critical rows are the derating measurement (the
    # getter the port refuses), the three-phase row is the capacity family's under another
    # feed, and the liquid-only row is the class's no-sizing branch.
    "process_separator_sizing.tsv": 5,
    # The mechanical capture's SRK row is the class's own test fixture; the port's separator
    # is PR, so only the PR row can be an oracle for this tree's state.
    "process_separator_mechanical_design.tsv": 1,
    # The gas scrubber's two mechanical rows are the same pair of cubics. The id has no dumper
    # - its stream side is the separator's and `NO_INTERIOR` says so - so the case is the
    # oracle and the capture stays here as its evidence.
    "process_gas_scrubber_mechanical_design.tsv": 2,
    # The settling capture's two vessels: the same feed at two geometries. The id is a
    # `NO_INTERIOR` model, so the cases are the oracle and these are their evidence.
    "process_three_phase_settling.tsv": 2,
    # The enhanced chain's eleven states: the class's own six, the mist-position and
    # weir-position setters, the vane-pack and axial-cyclone design-K records, and the
    # inlet-device overrides. The id is a `NO_INTERIOR` model - the enhanced path is the
    # kernel's own arithmetic, and the cases hold its six fractions and efficiencies.
    "process_separator_performance_enhanced.tsv": 11,
    # The plug-flow reactor's five: the probe swept ten states and the model declares five
    # cases, so the five left over are the probe's own sweep - its Euler default, its coarse
    # and every-step marches, its half-activity bed and its concentration probe - kept as
    # evidence for what the step count, the refresh frequency and the bed move.
    "process_plug_flow_reactor.tsv": 5,
    # And the Gibbs reactor's six: the probe ran eight states and the model two, and the six
    # between them are the methane/oxygen and ammonia rows, which `specs/cases/process/
    # gibbs_reactor.toml` says cannot be cases at all - "a feed whose products settle at
    # `1e-15` and `1e-7`" cannot be asserted relatively.
    "process_gibbs_reactor.tsv": 6,
    # The rate-based column's seven. `_srk` is the class's own state on the cubic its test file
    # uses, kept beside the PR row that the port is held to so the pair measures what the cubic
    # moved; the two TEG rows are `SystemSrkCPAstatoil` with mixing rule 10, which PR cannot
    # represent - so they are evidence for a refusal rather than an oracle for a state - and the
    # three remaining CO2/water rows are the stripper and the two heights, which the port's own
    # test asserts as *directions* rather than as digits.
    "process_rate_based_packed_column.tsv": 7,
    # And the correlation capture's three: `billet_onda` is `co2_water_absorber`'s state
    # again and `billet_schultes` on the built-in is the identity (its constants are `1.0` and
    # `0.40`), so neither is an oracle for a state the port does not already reach; the
    # `plastic_onda` baseline is the pair's control. The case reads `plastic_schultes`.
    "process_rate_based_billet.tsv": 3,
    "process_heat_exchanger.tsv": 2,
    # The shortcut column's two degenerate rows. **Both are evidence rather than oracles**:
    # a reflux multiplier of exactly one leaves Gilliland's `X` at zero and the class returns
    # `actual_stages = Infinity` with a feed tray of zero - `(int) Math.round(Infinity) + 1`
    # wrapping through `Integer.MIN_VALUE` - and swapped keys give the class's own
    # `solved = false` with every answer at its field initialiser. The port **refuses** both,
    # so neither has a state to be its case; they stay in the capture as the measurement the
    # refusal rests on.
    "process_shortcut_distillation_column.tsv": 2,
    # The column's two deethanizer rows. **Both are the class's own failure to converge** - the
    # same column at a hard-capped 80 iterations and at 200, ending `7.3e-3` and `2.9e-2` K from
    # a `1e-5` gate, the second worse than the first. The port converges on the same column, so
    # its state is not NeqSim's and the rows are evidence rather than oracles: what the port's
    # own test asserts about them is self-consistency, and this is why they are declared here
    # rather than paired with a case.
    # The column's own, one more: the reactive block's *standard* twin, which is the same state
    # at the same gate as the case two blocks above and is kept as the measurement the reactive
    # row is identical to.
    # And the two Murphree rows at the *tighter* gate, `1e-9` rather than `1e-6`: they take 22
    # and 86 iterations where their `1e-6` twins take 15 and 62, and move no tray by more than
    # `8e-10` K. **That is the measurement the `0.85` divergence rests on** - it says NeqSim's
    # `0.85` state is converged, so the port's disagreement with it is between two fixed points
    # rather than between a converged and a partial one. They are evidence rather than oracles
    # because the port needs no second copy of a state it already reaches at the looser gate.
    # And the side-draw capture's four: the class's own three one-tray columns, and the binary
    # column drawing liquid and a pumparound. **The three are the class's own states for this
    # mechanism** - `columnReportsSideDrawAsOutletStream` and `columnEnergyBalanceIncludes-
    # SideDrawStreams` run a one-tray column with no ends and a pure feed, and the class is happy
    # with the zero-flow product that leaves: the methane rows report a bottoms of `n = 0` with an
    # enthalpy of `-Infinity` (`getMaterialOutletEnthalpy` divides an empty system's enthalpy by
    # its zero moles) and a mass balance of exactly `0.0`. **The port refuses all three by name**,
    # because its products must exist: the two methane rows have no liquid for the bottom to
    # publish, and the pentane row's one tray is all liquid, so the column's two products would be
    # the same stream and the closure counts it twice - refused as `SolverNotConverged` with a
    # residual of `0.75`.
    # The fourth is a divergence rather than a refusal. **NeqSim does not close its own balance
    # there**: it answers `RECONCILED_PRODUCTS` with a mass balance of `-2.449` kg/hr against a
    # 1000 kg/hr feed and an energy error of `6.4e-2`, and its two products come out `0.0200`
    # mol/s larger than the port's. The port closes to `9.7e-9` and treats the drawn liquid as a
    # net outlet, which is what `SimpleTray`'s own split does - a pumparound fraction *without* a
    # return. What the class's reconciliation restores is the pumparound's return, which is
    # `ColumnPumparound`'s subject and is named out of this tranche.
    # **And the three side-draw *flow* tear rows**, which are the one group here with no case
    # behind them yet: the tear's own oracle is asserted by
    # `crates/azoth-process/tests/column.rs`, because the *declaration* the model needs is not
    # decided - a flow specification is a tray, a phase, a target, a tolerance and a cap, and no
    # input shape in the spec schema carries that record yet. The three say three different
    # things: `..._one_tray_gas_methane` is the class's own state, where its one-tray column has
    # no bottom product at all and NeqSim publishes a zero flow with a `-Infinity` enthalpy that
    # this port declines to fabricate; `..._one_tray_gas_binary` is the substitution the port is
    # held to - the same shape on a fluid that *does* have a liquid - and
    # `..._five_tray_liquid_fractionator` is the class's own multistage state re-cased, where the
    # class rejects 18 of 30 candidates and never converges.
    # **And two of the three side-draw *flow* tear rows.** The first is the class's own state,
    # where its one-tray pure-methane column has no bottom product at all and NeqSim publishes a
    # zero flow with a `-Infinity` enthalpy that this port declines to fabricate. The third is the
    # class's own multistage state re-cased, where the class rejects 18 of 30 candidates and never
    # converges. The middle one is a case: the same one-tray shape on a fluid that does have a
    # liquid, which is the substitution this port is held to.
    # **And the pumparound's *return*, which is a recycle the port does not carry.** The row is
    # the binary column this model reproduces exactly with `addLiquidPumparound("PA", 1, 3, 0.10,
    # 5.0)` on it, so the difference from `binary_rigorous` is the recycle and nothing else:
    # measured, the return is the draw at exactly `T - 5 K` (`334.5849` -> `329.5849`), its flow
    # settles to a `2.8e-5` relative change in 19 iterations, the cooler takes `-438.78` W, and
    # the profile moves - tray 1 lands at `334.58` K against the ideal column's `336.15`.
    "process_column.tsv": 15,
    # One capture for two ids, because the two machines it drives are one class with two names,
    # and five of its six rows are uncased for each of them. **The pinned pair is the classes'
    # own isothermal case**: `setOutletTemperature` on every stage makes the base's gate exactly
    # zero, so the solve stops after its first sweep - and the port **refuses** the state it
    # lands on, because that state's energy closure is `0.28` against the class's own default
    # gate of `1.6e-2`. The classes' *tests* accept it only because they loosened that gate to
    # `5e-2`, a settable this port has not carried. The two loose-gate rows are the same
    # measurement one step out, and the stripper's pinned row states `MESH_RESIDUAL` besides.
    # **And the two reactive sibling rows.** `AbsorptionColumn` inherits `setReactive` and no
    # `AbsorptionColumnTest`, `StrippingColumnTest` or `PackedColumnTest` calls it, so these are
    # the measurement rather than an oracle: on this fluid the class's own solve ends
    # `FALLBACK_PRODUCTS` with a mass residual of `6.4e5` after 16 iterations, and its own log
    # calls the published products not a rigorous column result. The port refuses the states
    # neither library converges, and what it asserts about them is that refusal.
    "process_absorber.tsv": 7,
    # The packed column's thirteen: eleven stage-count rows and two solved rows. **The eleven are
    # the constructor's own arithmetic** - `PackedColumn(name, height, packing, true, true)` with
    # no feed and no solve, printing the trays it made - so there is no model state for them to be
    # a case of; the port's own test asserts the rule against all eleven. The two solved rows are
    # evidence rather than oracles: **the class's own test case converges to a state that is not a
    # column**, reporting `RIGOROUS_CONVERGED` with residuals of exactly zero while the trays
    # below the feed carry no traffic at all, and the contactor row reports
    # `FALLBACK_PRODUCTS`, which the class's own warning says means the products are a single
    # flash of the mixed feeds rather than the tray solution.
    "process_packed_column.tsv": 13,
    # The release capture's twenty-one. The probe prints the PR oracle, the SRK evidence,
    # the two states no azoth input can express (an empty fluid and the hydrate flag) and the
    # propane/n-butane row the class calls INVALID while this port converges. The layer diff
    # reads the choked methane block and the flashing propane block; the rest are cases
    # asserted on the rate and the sound speed, or evidence with no state to compare.
    "process_hem_release.tsv": 21,
    # The blowdown capture's ten: one block is layer-cased (the ten-second methane run); the
    # rest are the long horizons held to the capture as evidence, the SRK rows, the propane
    # row whose single-phase beta and cached density are class artefacts, and the refusals.
    "process_leak_blowdown.tsv": 10,
    # The pump design's five: the reversible-efficiency repeat, the sealless state (a seal
    # spelling on the plain design), the zero-flow state the model refuses, and the two chart
    # blocks, whose state is the chart branch rather than the dumper's isentropic one.
    "process_pump_mechanical_design.tsv": 5,
    # The expander design's three: the reversible-efficiency repeat and the two casing
    # temperature-derate states, which are the same radial machine under a hotter and a
    # colder inlet.
    "process_expander_mechanical_design.tsv": 3,
}


def case_inputs(model: str, case: str) -> tuple[dict[str, Any], float]:
    """A case's declared inputs and tolerance, from the generated registry.

    The registry rather than the case file: `specs/cases/process/*.toml` is what a case
    *is*, and `azoth._models_gen` is its generated form - the one the reference kernels and
    the generated stub already read.
    """
    from azoth import _models_gen

    for declared in _models_gen.MODELS:
        if declared["id"] != model:
            continue
        for entry in declared["cases"]:
            if entry["id"] == case:
                return dict(entry["inputs"]), float(entry["tolerance"])
    raise KeyError(f"{model} has no case {case!r}")


@dataclass(frozen=True)
class LayerDiff:
    """One layer both sides have, and whether it is where it is declared to be.

    The three rules live here rather than in the gate that applies them, so that the gate
    and `tools/neqsim_layer_diff.py` cannot come to different answers about one layer.
    """

    key: str
    capture: float
    azoth: float
    #: Which rule holds it: `"case"` for the case's relative tolerance, `"absolute"` for a
    #: declared [`LayerCase.diagnostic`], `"divergence"` for a declared [`Divergence`].
    by: str
    #: The bound the rule compares against.
    bound: float
    #: What the rule measured - a relative gap, an absolute one, or a ratio.
    gap: float
    ok: bool

    def describe(self) -> str:
        """The layer, both numbers, and the rule it missed."""
        if self.by == "divergence":
            return (
                f".{self.key}: the capture's {self.capture} is {self.gap:.3f} times azoth's "
                f"{self.azoth}, under the {self.bound} this divergence is declared to be at "
                f"least"
            )
        if self.by == "absolute":
            return (
                f".{self.key}: {self.azoth} against the capture's {self.capture} is "
                f"{self.gap:.3e} absolute, over the {self.bound:.1e} this layer is compared on"
            )
        return (
            f".{self.key}: {self.azoth} against the capture's {self.capture} is "
            f"{self.gap:.3e} relative, over the case's {self.bound:.1e}"
        )


def compare(
    layer_case: LayerCase,
    inputs: Mapping[str, Any],
    *,
    tolerance: float | None = None,
) -> list[LayerDiff]:
    """Every layer the dump and the capture share, in the dump's own order, with its verdict.

    **In the dumper's order and not the capture's**, because "the first layer that moved" is
    a statement about the build order of the model - the reason the dumpers put the cause
    before the answer it produced, and the reason this returns the list rather than the
    first failure.
    """
    block = capture_blocks(layer_case.capture)[layer_case.block]
    dumped = rows(layer_case.model, inputs)
    absolute = dict(layer_case.diagnostic)
    divergence = {declared.key: declared for declared in layer_case.divergence}
    out: list[LayerDiff] = []
    for key, mine in dumped.items():
        if key not in block:
            continue
        theirs = float(block[key])
        if key in divergence:
            declared = divergence[key]
            # **The distance between the two, in whichever direction it runs.** A divergence
            # whose sign flips from row to row cannot be stated as `capture / azoth >= n`,
            # because half its rows sit below one - `process.component_splitter`'s outlet
            # enthalpies are `0.71` and `0.75` on two rows and `2.09` on the third, the class
            # reporting an enthalpy below the state's on one state and above on another.
            # `max(r, 1/r)` says what the older rule said wherever the capture was the larger
            # - `process.splitter`'s three ratios are all above one, so its bounds read
            # identically - and says something on the rows where it was not.
            ratio = abs(theirs) / abs(mine) if mine else float("inf")
            ratio = max(ratio, 1.0 / ratio) if ratio > 0.0 else float("inf")
            out.append(
                LayerDiff(
                    key=key,
                    capture=theirs,
                    azoth=mine,
                    by="divergence",
                    bound=declared.at_least,
                    gap=ratio,
                    ok=ratio >= declared.at_least,
                )
            )
        elif key in absolute:
            bound = absolute[key]
            out.append(
                LayerDiff(
                    key=key,
                    capture=theirs,
                    azoth=mine,
                    by="absolute",
                    bound=bound,
                    gap=abs(mine - theirs),
                    ok=abs(mine - theirs) <= bound,
                )
            )
        else:
            bound = tolerance if tolerance is not None else case_tolerance(layer_case)
            scale = abs(theirs)
            gap = abs(mine - theirs) / scale if scale else abs(mine - theirs)
            out.append(
                LayerDiff(
                    key=key,
                    capture=theirs,
                    azoth=mine,
                    by="case",
                    bound=bound,
                    gap=gap,
                    ok=gap <= bound,
                )
            )
    return out


def case_tolerance(layer_case: LayerCase) -> float:
    """The tolerance the case itself compares with."""
    return case_inputs(layer_case.model, layer_case.case)[1]


def capture_blocks(capture: str) -> list[dict[str, str]]:
    """A capture's blocks, each as its `key=value` rows plus its label.

    The process probes print one block per case: a label line, the ports' records, the
    machine's own numbers, and a blank line. The label is the block's first line and is not
    a `key=value` row, so it is kept under [`LABEL_KEY`] rather than dropped. **It is what
    `LayerCase.identified_by` should name**, because it is the block's identity rather than
    a state: keying the pairing on a number would make an identification failure reachable
    by the very arithmetic the comparison exists to test, and two rows that differ only in
    an argument the ports do not carry - the exchanger's counterflow and parallel-flow rows
    - have no other distinguishing line between them.
    """
    text = (CAPTURES / capture).read_text(encoding="utf-8")
    blocks: list[dict[str, str]] = []
    for chunk in text.strip().split("\n\n"):
        rows: dict[str, str] = {}
        #: The key the last `key=value` line opened, so a line without one can continue it.
        opened: str | None = None
        for line in chunk.strip().splitlines():
            line = line.strip()
            if not line:
                continue
            if "=" in line:
                key, _, value = line.partition("=")
                opened = key.strip()
                rows[opened] = value.strip()
            elif opened is None:
                rows[LABEL_KEY] = line
            else:
                # **A line with no key continues the one above it.** A probe that prints a
                # profile prints `profile=` and then the table under it, and a table's rows
                # have no `=` in them: read as labels they overwrite the block's own name,
                # which is how the plug-flow capture identified itself as
                # `5.000000,733.4597,...` - a row of numbers - rather than by its case.
                rows[opened] = f"{rows[opened]}\n{line}"
        if rows:
            blocks.append(rows)
    return blocks


def rows(model: str, inputs: Mapping[str, Any]) -> dict[str, float]:
    """A model's layers, under the capture's own key names."""
    dumper = DUMPERS.get(model)
    if dumper is None:
        raise KeyError(f"no layer dump for {model!r}; DUMPERS carries {sorted(DUMPERS)}")
    return dumper(inputs)
