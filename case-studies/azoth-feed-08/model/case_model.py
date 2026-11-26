"""Reproducible model for the offshore debottlenecking FEED case study.

This is a script version of notebooks/08-offshore-debottlenecking.ipynb from the
azoth repository, with the notebook's display calls replaced by a JSON dump so
the case study's numbers can be regenerated and diffed.  Every physical number
comes from the azoth library; the study's own assumptions (prices, schedule
rule, brownfield allowances, HIPPS relief reduction) are named as such.

Run:  python3 model/case_model.py  ->  model/results.json
"""

from __future__ import annotations

import json
import math
from pathlib import Path

import numpy as np

import azoth
import azoth.eos
import azoth.process
from azoth.core.errors import InvalidInputError, OutOfRangeError, SolverNotConvergedError

q = azoth.ureg.Quantity
R = 8.314462618
T_SC, P_SC = 288.15, 101325.0
MMSCFD_PER_MOL_S = R * T_SC / P_SC * 86400.0 / 28316.846592
BPD_PER_M3 = 1.0 / 0.158987294928

COMPONENTS_HC = [
    "methane",
    "ethane",
    "propane",
    "i-butane",
    "n-butane",
    "i-pentane",
    "n-pentane",
    "n-hexane",
    "n-heptane",
    "n-octane",
    "co2",
    "nitrogen",
]
Z_HC10 = [0.7721, 0.068, 0.045, 0.011, 0.022, 0.0105, 0.013, 0.015, 0.015, 0.0104, 0.012, 0.004]
COMPONENTS = [*COMPONENTS_HC, "water"]
FLARE_COMPONENTS = [name for name in COMPONENTS if name not in ("co2", "water")]
MIXTURE, IDEAL = azoth.eos.components.mixture_of(COMPONENTS)
MOLAR_MASS = [
    azoth.eos.components.component(name).molar_mass.to("kg/mol").magnitude for name in COMPONENTS
]

P_WH, T_WH = 95.0, 363.15
P_HP, P_MP, P_LP = 60.0, 10.0, 2.0
P_EXPORT, P_LIQ, P_RECEPTION = 90.0, 25.0, 20.0
T_HP, T_COOL = 354.15, 313.15
HC_CURRENT, HC_POTENTIAL, WATER_RATE = 1930.0, 2300.0, 300.0
D_HP, L_HP = 1.96, 7.26
D_MP, L_MP = 1.61, 8.03
D_LP, L_LP = 1.53, 7.64
K_DESIGN, K_UPGRADE = 0.11, 0.14
DRIVER_KW, PUMP_KW = 2300.0, 42.0
PSV_SET, PSV_BACK, PSV_AREA_IN2 = 66.0, 1.5, 6.38
RELIEF_PRESSURE = 72.6
WATER_CAPACITY, WATER_UPGRADE = 3000.0, 6000.0
RETENTION_OIL, RETENTION_WATER = 9.0, 6.5

GAS_PRICE, COND_PRICE, DISCOUNT, TAX = 4.0, 70.0, 0.10, 0.25
HEATING_VALUE = 1.05e3
YEARS = 10
G0, DECLINE = 163.4, 0.05
WC_START, WC_END = 0.45, 0.57
BASE_GAS = 137.1
GAS_RECEPTION = 60.0


def norm(z):
    total = float(sum(z))
    return [float(value) / total for value in z]


def mix_streams(n1, z1, n2, z2):
    if n1 <= 0.0:
        return float(n2), list(z2)
    if n2 <= 0.0:
        return float(n1), list(z1)
    total = n1 + n2
    return total, [(n1 * a + n2 * b) / total for a, b in zip(z1, z2, strict=True)]


def as_q(value, unit):
    return value.to(unit) if hasattr(value, "to") else q(value, unit)


def wet_feed(hc_n, water_n, z_hc):
    z_hc = norm(z_hc)
    total = hc_n + water_n
    z = [value * hc_n / total for value in z_hc] + [water_n / total]
    return total, norm(z)


def water_bpd(n_mol_s):
    return n_mol_s * 0.018015 / 1000.0 * 86400.0 * BPD_PER_M3


def boost_gas(n_mol_s, z, p_in, p_out, eta=0.78, t_cool=T_COOL, ratio_max=2.5):
    stages = max(1, math.ceil(math.log(p_out / p_in) / math.log(ratio_max)))
    ratio = (p_out / p_in) ** (1.0 / stages)
    power = duty = 0.0
    n, z = float(n_mol_s), norm(z)
    p, t = p_in, t_cool
    for _ in range(stages):
        comp = azoth.process.compressor(
            COMPONENTS,
            q(n, "mol/s"),
            z,
            q(p, "bar"),
            q(t, "K"),
            eta,
            outlet_pressure=q(p * ratio, "bar"),
        )
        power += comp.power.to("kW").magnitude
        cool = azoth.process.cooler(
            COMPONENTS,
            comp.outlet_n,
            list(comp.outlet_z),
            comp.outlet_p,
            comp.outlet_t,
            outlet_temperature=q(t_cool, "K"),
        )
        duty += abs(cool.outlet_duty.to("kW").magnitude)
        n = cool.outlet_n.to("mol/s").magnitude
        z = norm(list(cool.outlet_z))
        p = cool.outlet_p.to("bar").magnitude
        t = t_cool
    return {"n": n, "z": z, "power_kw": power, "duty_kw": duty, "stages": stages}


def stock_tank(n_mol_s, z):
    z = norm(z)
    flash = azoth.eos.pt_flash(MIXTURE, q(T_SC, "K"), q(P_SC, "bar"), z)
    if flash.phase.value == "all_liquid":
        beta, x = 0.0, z
    elif flash.phase.value in {"all_vapour", "trivial"}:
        beta, x = 1.0, z
    else:
        beta = max(0.0, min(1.0, flash.vapour_fraction or 0.0))
        x = list(flash.x)
    n_liq = n_mol_s * (1.0 - beta)
    n_gas = n_mol_s * beta
    if n_liq <= 0.0:
        return {"bpd": 0.0, "gas_n": n_gas, "gas_z": z}
    m_liq = sum(a * b for a, b in zip(x, MOLAR_MASS, strict=True))
    v_liq = azoth.eos.pr_molar_volume(flash.z_liquid, q(T_SC, "K"), q(P_SC, "bar")).v
    rho = azoth.eos.pr_mass_density(q(m_liq, "kg/mol"), v_liq).rho.to("kg/m**3").magnitude
    bpd = n_liq * m_liq / rho * 86400.0 * BPD_PER_M3
    return {"bpd": bpd, "gas_n": n_gas, "gas_z": z}


def let_down(n_mol_s, z, p_in, t_in, h_in, p_out):
    z = norm(z)
    try:
        valve = azoth.process.throttling_valve(
            COMPONENTS,
            q(n_mol_s, "mol/s"),
            z,
            q(p_in, "bar"),
            as_q(t_in, "K"),
            outlet_pressure=q(p_out, "bar"),
        )
        return {
            "n": valve.outlet_n.to("mol/s").magnitude,
            "z": norm(list(valve.outlet_z)),
            "p": valve.outlet_p,
            "t": valve.outlet_t,
            "h": valve.outlet_h,
            "model": "valve",
        }
    except InvalidInputError:
        flash = azoth.eos.ph_flash(MIXTURE, IDEAL, q(p_out, "bar"), as_q(h_in, "J/mol"), z)
        return {
            "n": n_mol_s,
            "z": z,
            "p": q(p_out, "bar"),
            "t": flash.T,
            "h": as_q(h_in, "J/mol"),
            "model": "ph_flash",
        }


def hp_split(hc_n, water_n):
    total, z = wet_feed(hc_n, water_n, Z_HC10)
    try:
        multi = azoth.eos.tp_multiflash(MIXTURE, q(T_HP, "K"), q(P_HP, "bar"), z)
        vessel = azoth.process.three_phase_separator(
            COMPONENTS,
            q(total, "mol/s"),
            z,
            q(P_HP, "bar"),
            q(T_HP, "K"),
            q(0.0, "bar"),
            gas_in_aqueous=0.0,
            gas_in_oil=0.0,
            oil_in_aqueous=0.0,
            oil_in_gas=0.0,
            aqueous_in_gas=0.0,
            aqueous_in_oil=0.0,
            internal_diameter=q(D_HP, "m"),
            separator_length=q(L_HP, "m"),
            nll_fraction=0.5,
            nil_fraction=0.2,
        )
        return {
            "mode": "three_phase",
            "total": total,
            "z": z,
            "vapour_n": vessel.vapour_n.to("mol/s").magnitude,
            "vapour_z": norm(list(vessel.vapour_z)),
            "light_liquid_n": vessel.light_liquid_n.to("mol/s").magnitude,
            "light_liquid_z": norm(list(vessel.light_liquid_z)),
            "light_liquid_h": vessel.light_liquid_h,
            "light_liquid_t": vessel.light_liquid_t,
            "heavy_liquid_n": vessel.heavy_liquid_n.to("mol/s").magnitude,
            "heavy_liquid_z": norm(list(vessel.heavy_liquid_z)),
            "oil_retention_s": vessel.oil_retention_time.to("s").magnitude,
            "water_retention_s": vessel.water_retention_time.to("s").magnitude,
            "settling_s": vessel.interface_settling_time.to("s").magnitude,
            "settling_ok": vessel.settling_time_sufficient,
            "phase_count": multi.phase_count,
            "phase_fractions": list(multi.phase_fractions),
            "seeded": multi.seeded,
        }
    except (InvalidInputError, OutOfRangeError, SolverNotConvergedError):
        dry_z = [*norm(Z_HC10), 0.0]
        sep = azoth.process.separator(
            COMPONENTS,
            q(hc_n, "mol/s"),
            dry_z,
            q(P_HP, "bar"),
            q(T_HP, "K"),
            q(0.0, "bar"),
            0.0,
        )
        return {
            "mode": "free_water",
            "total": total,
            "z": z,
            "vapour_n": sep.vapour_n.to("mol/s").magnitude,
            "vapour_z": norm(list(sep.vapour_z)),
            "light_liquid_n": sep.liquid_n.to("mol/s").magnitude,
            "light_liquid_z": norm(list(sep.liquid_z)),
            "light_liquid_h": sep.liquid_h,
            "light_liquid_t": sep.liquid_t,
            "heavy_liquid_n": water_n,
            "heavy_liquid_z": [0.0] * 12 + [1.0],
            "oil_retention_s": None,
            "water_retention_s": None,
            "settling_s": None,
            "settling_ok": False,
            "phase_count": 2,
            "phase_fractions": None,
            "seeded": "free_water",
        }


def run_train(hc_n, water_n, k_hp=K_DESIGN):
    hp = hp_split(hc_n, water_n)
    hp_n, hp_z = hp["vapour_n"], hp["vapour_z"]
    hp_m = sum(a * b for a, b in zip(hp_z, MOLAR_MASS, strict=True))
    hp_mass = hp_n * hp_m
    assess = azoth.process.separator_capacity_assessment(
        COMPONENTS,
        q(hp_n, "mol/s"),
        hp_z,
        q(P_HP, "bar"),
        q(T_HP, "K"),
        q(0.0, "bar"),
        0.0,
        q(D_HP, "m"),
        q(k_hp, "m/s"),
        "horizontal",
        0.2,
        "as-built HP separator data sheet",
    )
    mp_in = let_down(
        hp["light_liquid_n"],
        hp["light_liquid_z"],
        P_HP,
        hp["light_liquid_t"],
        hp["light_liquid_h"],
        P_MP,
    )
    mp = azoth.process.separator(
        COMPONENTS,
        q(mp_in["n"], "mol/s"),
        mp_in["z"],
        q(P_MP, "bar"),
        mp_in["t"],
        q(0.0, "bar"),
        0.0,
    )
    lp_in = let_down(
        mp.liquid_n.to("mol/s").magnitude,
        list(mp.liquid_z),
        P_MP,
        mp.liquid_t,
        mp.liquid_h,
        P_LP,
    )
    lp = azoth.process.separator(
        COMPONENTS,
        q(lp_in["n"], "mol/s"),
        lp_in["z"],
        q(P_LP, "bar"),
        lp_in["t"],
        q(0.0, "bar"),
        0.0,
    )
    gas_n, gas_z = 0.0, None
    power = duty = 0.0
    for n, zz, p in [
        (hp_n, hp_z, P_HP),
        (mp.vapour_n.to("mol/s").magnitude, norm(list(mp.vapour_z)), P_MP),
        (lp.vapour_n.to("mol/s").magnitude, norm(list(lp.vapour_z)), P_LP),
    ]:
        boost = boost_gas(n, zz, p, P_EXPORT)
        power += boost["power_kw"]
        duty += boost["duty_kw"]
        gas_n, gas_z = mix_streams(gas_n, gas_z, boost["n"], boost["z"])
    liquid_n = lp.liquid_n.to("mol/s").magnitude
    liquid_z = norm(list(lp.liquid_z))
    tank = stock_tank(liquid_n, liquid_z)
    sales_n = gas_n + tank["gas_n"]
    if tank["gas_n"] > 0.0:
        gas_z = mix_streams(gas_n, gas_z, tank["gas_n"], tank["gas_z"])[1]
    pump_kw = pump_head = 0.0
    if lp.liquid_p.to("bar").magnitude < P_LIQ - 0.1:
        pump = azoth.process.pump(
            COMPONENTS,
            q(liquid_n, "mol/s"),
            liquid_z,
            lp.liquid_p,
            lp.liquid_t,
            0.75,
            outlet_pressure=q(P_LIQ, "bar"),
        )
        pump_kw = pump.power.to("kW").magnitude
        pump_head = pump.head.to("m").magnitude if pump.head is not None else 0.0
    water = water_bpd(hp["heavy_liquid_n"])
    gas_mmscfd = sales_n * MMSCFD_PER_MOL_S
    return {
        "hp": hp,
        "mp": mp,
        "lp": lp,
        "tank": tank,
        "gas_mmscfd": gas_mmscfd,
        "bpd": tank["bpd"],
        "water_bpd": water,
        "cgr": tank["bpd"] / gas_mmscfd if gas_mmscfd > 0.0 else 0.0,
        "water_cut": water / (water + tank["bpd"]),
        "power_kw": power,
        "duty_kw": duty,
        "pump_kw": pump_kw,
        "pump_head": pump_head,
        "hp_util": assess.utilization,
        "hp_k": k_hp * assess.utilization,
        "oil_retention_s": hp["oil_retention_s"],
        "water_retention_s": hp["water_retention_s"],
        "settling_s": hp["settling_s"],
        "settling_ok": hp["settling_ok"],
        "mode": hp["mode"],
        "mp_model": mp_in["model"],
        "lp_model": lp_in["model"],
        "hp_mass": hp_mass,
        "hp_n": hp_n,
        "hp_z": hp_z,
        "gas_n": gas_n,
        "gas_z": gas_z,
        "sales_n": sales_n,
        "liquid_n": liquid_n,
        "liquid_z": liquid_z,
    }


def flare_z_of(z):
    return norm([z[i] for i, name in enumerate(COMPONENTS) if name not in ("co2", "water")])


def flare_case(n, z, design_mass, design_molar, ground_distance=100.0):
    return azoth.process.flare(
        FLARE_COMPONENTS,
        q(n, "mol/s"),
        flare_z_of(z),
        q(P_HP, "bar"),
        q(T_HP, "K"),
        ground_distance=q(ground_distance, "m"),
        radiation_flux_threshold=q(4.73, "kW/m**2"),
        design_mass_flow_capacity=q(design_mass, "kg/s"),
        design_molar_flow_capacity=q(design_molar, "mol/s"),
    )


def relief_and_psv(mass, z, label):
    blocked = azoth.process.blocked_outlet_relief(
        COMPONENTS,
        z,
        q(mass, "kg/s"),
        q(RELIEF_PRESSURE, "bar"),
        q(T_HP, "K"),
        scenario_name=label,
    )
    psv = azoth.process.psv_sizing(
        "gas",
        q(PSV_SET, "bar"),
        0.10,
        q(PSV_BACK, "bar"),
        mass_flow=blocked.relief_rate,
        temperature=q(T_HP, "K"),
        molar_mass=blocked.molar_mass,
        compressibility=blocked.compressibility,
        specific_heat_ratio=blocked.specific_heat_ratio,
        orifice_area=q(PSV_AREA_IN2, "inch**2").to("m**2"),
        capacity_discharge_coefficient=0.975,
    )
    return blocked, psv


def forecast(gas_cap, cond_add=0.0, decline=DECLINE, wc_start=None, wc_end=None):
    wc_start = WC_START if wc_start is None else wc_start
    wc_end = WC_END if wc_end is None else wc_end
    rows = []
    for year in range(1, YEARS + 1):
        potential_gas = G0 * (1.0 - decline) ** (year - 1)
        gas = min(potential_gas, gas_cap)
        cond = gas * CGR_VALUE + cond_add
        wc = wc_start + (wc_end - wc_start) * (year - 1) / (YEARS - 1)
        water = cond * wc / (1.0 - wc)
        rows.append((year, gas, cond, water))
    return rows


def dcf(
    gas_cap,
    capex,
    deferment=0.0,
    cond_add=0.0,
    gas_price=GAS_PRICE,
    cond_price=COND_PRICE,
    discount=DISCOUNT,
    tax=TAX,
    decline=DECLINE,
    wc_start=None,
    wc_end=None,
    capex_mult=1.0,
    opex_fixed=2.5,
    electricity=0.33,
    water_cost_per_bbl=2.0,
):
    base = forecast(BASE_GAS, 0.0, decline, wc_start, wc_end)
    package = forecast(gas_cap, cond_add, decline, wc_start, wc_end)
    inc_gas = [p[1] - b[1] for p, b in zip(package, base, strict=True)]
    inc_cond = [p[2] - b[2] for p, b in zip(package, base, strict=True)]
    inc_water = [p[3] - b[3] for p, b in zip(package, base, strict=True)]
    rev_gas = [g * 365.0 * HEATING_VALUE * gas_price / 1e6 for g in inc_gas]
    rev_cond = [c * 365.0 * cond_price / 1e6 for c in inc_cond]
    opex = [opex_fixed + electricity + water_cost_per_bbl * w * 365.0 / 1e6 for w in inc_water]
    ebitda = [a + b - o for a, b, o in zip(rev_gas, rev_cond, opex, strict=True)]
    dep = capex * capex_mult / YEARS
    ebit = [value - dep for value in ebitda]
    tax_cash = [tax * max(value, 0.0) for value in ebit]
    cash = [e - t for e, t in zip(ebitda, tax_cash, strict=True)]
    disc = [(1.0 + discount) ** year for year in range(1, YEARS + 1)]
    npv = -capex * capex_mult - deferment + sum(c / d for c, d in zip(cash, disc, strict=True))
    cum = np.cumsum(cash) - capex * capex_mult - deferment
    cum_disc = np.cumsum([c / d for c, d in zip(cash, disc, strict=True)]) - capex * capex_mult - deferment
    payback = next((index + 1 for index, value in enumerate(cum) if value >= 0.0), None)
    return {
        "npv": npv,
        "payback": payback,
        "cash": cash,
        "cum": [float(v) for v in cum],
        "cum_disc": [float(v) for v in cum_disc],
        "gas_pv": sum(r / d for r, d in zip(rev_gas, disc, strict=True)),
        "cond_pv": sum(r / d for r, d in zip(rev_cond, disc, strict=True)),
        "inc_gas": inc_gas,
        "inc_cond": inc_cond,
        "inc_water": inc_water,
        "rev_gas": rev_gas,
        "rev_cond": rev_cond,
        "opex": opex,
        "tax": tax_cash,
        "ebitda": ebitda,
    }


def main() -> None:
    current_split = hp_split(HC_CURRENT, WATER_RATE)
    potential_split = hp_split(HC_POTENTIAL, WATER_RATE)

    current = run_train(HC_CURRENT, WATER_RATE)
    potential = run_train(HC_POTENTIAL, WATER_RATE)

    base_gas = current["gas_mmscfd"]
    global BASE_GAS, G0, CGR_VALUE, WC_START
    BASE_GAS = base_gas
    G0 = potential["gas_mmscfd"]
    CGR_VALUE = current["cgr"]
    WC_START = current["water_cut"]

    blocked_current, psv_current = relief_and_psv(
        current["hp_mass"], current["hp_z"], "HP blocked outlet"
    )
    flare_design_mass = current["hp_mass"] / 0.92
    flare_design_molar = current["hp_n"] / 0.92
    flare_current = flare_case(
        current["hp_n"], current["hp_z"], flare_design_mass, flare_design_molar
    )

    gas_line_current = azoth.eos.pipe_beggs_and_brills(
        COMPONENTS,
        current["sales_n"],
        current["gas_z"],
        q(P_EXPORT, "bar"),
        q(T_COOL, "K"),
        q(15000.0, "m"),
        q(0.3048, "m"),
        q(0.0, "m"),
        30,
        q(4.6e-5, "m"),
    )
    gas_line_potential = azoth.eos.pipe_beggs_and_brills(
        COMPONENTS,
        potential["sales_n"],
        potential["gas_z"],
        q(P_EXPORT, "bar"),
        q(T_COOL, "K"),
        q(15000.0, "m"),
        q(0.3048, "m"),
        q(0.0, "m"),
        30,
        q(4.6e-5, "m"),
    )
    liquid_line_current = azoth.eos.pipe_beggs_and_brills(
        COMPONENTS,
        current["liquid_n"],
        current["liquid_z"],
        q(P_LIQ, "bar"),
        q(T_COOL, "K"),
        q(15000.0, "m"),
        q(0.1524, "m"),
        q(0.0, "m"),
        30,
        q(4.6e-5, "m"),
    )
    liquid_line_potential = azoth.eos.pipe_beggs_and_brills(
        COMPONENTS,
        potential["liquid_n"],
        potential["liquid_z"],
        q(P_LIQ, "bar"),
        q(T_COOL, "K"),
        q(15000.0, "m"),
        q(0.1524, "m"),
        q(0.0, "m"),
        30,
        q(4.6e-5, "m"),
    )
    gas_line_drop = P_EXPORT - gas_line_current.outlet_pressure.to("bar").magnitude
    gas_line_cap = base_gas * ((P_EXPORT - GAS_RECEPTION) / gas_line_drop) ** (1.0 / 1.8)
    liquid_line_drop = P_LIQ - liquid_line_current.outlet_pressure.to("bar").magnitude

    blocked_18, psv_18 = relief_and_psv(
        current["hp_mass"] * 1.18, current["hp_z"], "HP blocked outlet +18%"
    )
    blocked_hipps, psv_hipps = relief_and_psv(
        current["hp_mass"] * 1.18 * 0.65, current["hp_z"], "HP blocked outlet +18% HIPPS"
    )
    fire = azoth.process.fire_case_relief(
        True,
        True,
        vessel_diameter=q(D_HP, "m"),
        wetted_height=q(0.98, "m"),
        latent_heat=350000.0,
        relieving_temperature=q(T_HP, "K"),
    )
    network = azoth.process.relief_disposal_network(
        [
            {
                "item_name": "HP blocked outlet",
                "cause": "blocked_outlet",
                "phase": "vapour",
                "mass_flow": blocked_current.relief_rate,
                "simultaneous": True,
            },
            {
                "item_name": "HP fire",
                "cause": "fire",
                "phase": "vapour",
                "mass_flow": fire.relief_rate,
                "simultaneous": False,
            },
        ]
    )
    flare_18 = flare_case(
        current["hp_n"] * 1.18, current["hp_z"], flare_design_mass, flare_design_molar
    )
    flare_25 = flare_case(
        current["hp_n"] * 1.18,
        current["hp_z"],
        flare_design_mass * 1.25,
        flare_design_molar * 1.25,
    )
    flare_hipps = flare_case(
        current["hp_n"] * 1.18 * 0.65,
        current["hp_z"],
        flare_design_mass,
        flare_design_molar,
    )

    npd = azoth.process.nog070_sil_determination("hipps_pipeline", 1e-4)
    sil = azoth.process.sil_verification(1e-4, 3.0, "1oo2", 8760.0)
    ppp = azoth.process.pipeline_pressure_protection(
        q(95.0, "bar"), q(66.0, "bar"), 1.1, q(64.0, "bar"), q(66.0, "bar")
    )
    s001 = azoth.process.s001_secondary_pressure_protection(
        maximum_event_pressure=72.6,
        design_pressure=66.0,
        test_pressure=99.0,
        demand_frequency_per_year=1e-4,
        target_frequency_per_year=1e-3,
        relief_leakage_assessed=True,
        relief_leakage_to_safe_location=True,
        proof_test_interval_months=12,
    )

    caps = {
        "HP separator": base_gas / current["hp_util"],
        "Compressor": base_gas / (current["power_kw"] / DRIVER_KW),
        "Flare": base_gas / flare_current.mass_utilization,
        "Water plant": base_gas / (current["water_bpd"] / WATER_CAPACITY),
        "Oil retention": base_gas * (current["oil_retention_s"] / 60.0) / RETENTION_OIL,
        "Water retention": base_gas * (current["water_retention_s"] / 60.0) / RETENTION_WATER,
        "PSV": base_gas * psv_current.mass_flow_capacity.to("kg/s").magnitude / current["hp_mass"],
        "Gas line": gas_line_cap,
    }
    fixed_caps = {
        "HP separator": caps["HP separator"] * K_UPGRADE / K_DESIGN,
        "Compressor": base_gas * 1.18,
        "Flare": caps["Flare"] * 1.25,
        "Water plant": caps["Water plant"] * WATER_UPGRADE / WATER_CAPACITY,
        "Oil retention": caps["Oil retention"],
        "Water retention": caps["Water retention"],
        "PSV": caps["PSV"],
        "Gas line": caps["Gas line"],
    }
    hipps_caps = {**fixed_caps, "Flare": caps["Flare"] / 0.65}
    gas_side_caps = {**fixed_caps, "Water plant": caps["Water plant"]}
    gas_side_cap = min(gas_side_caps[k] for k in ("HP separator", "Compressor", "Flare", "Water plant"))
    package_cap = min(fixed_caps[k] for k in ("HP separator", "Compressor", "Flare", "Water plant"))
    hipps_cap = min(hipps_caps[k] for k in ("HP separator", "Compressor", "Flare", "Water plant"))

    FIXES = [
        {"id": 1, "fix": "compressor re-rate", "effect": "+18% gas capacity", "capex": 18.0, "weeks": 3.0},
        {"id": 2, "fix": "separator internals", "effect": "K 0.11 -> 0.14 m/s", "capex": 4.0, "weeks": 3.0},
        {"id": 3, "fix": "flare/header upgrade", "effect": "+25% relief capacity", "capex": 12.0, "weeks": 4.0},
        {"id": 4, "fix": "HIPPS SIL-3", "effect": "-35% blocked-outlet case", "capex": 6.0, "weeks": 2.0},
        {"id": 5, "fix": "produced-water upgrade", "effect": "3,000 -> 6,000 bbl/d + coalescer", "capex": 8.0, "weeks": 4.0},
        {"id": 6, "fix": "fourth stage + VRU", "effect": "+150 bbl/d condensate", "capex": 22.0, "weeks": 5.0},
        {"id": 7, "fix": "stabiliser", "effect": "+200 bbl/d condensate", "capex": 35.0, "weeks": 8.0},
    ]
    PACKAGES = {
        "Gas side": {"fixes": "1+2+3", "cap": gas_side_cap, "capex": 34.0, "weeks": 4.0, "cond_add": 0.0, "deferment": 0.0},
        "Gas side + water": {"fixes": "1+2+3+5", "cap": package_cap, "capex": 42.0, "weeks": 4.0, "cond_add": 0.0, "deferment": 0.0},
        "HIPPS + water": {"fixes": "1+2+4+5", "cap": hipps_cap, "capex": 36.0, "weeks": 4.0, "cond_add": 0.0, "deferment": 0.0},
        "+ fourth stage": {"fixes": "1+2+3+5+6", "cap": package_cap, "capex": 67.3, "weeks": 5.0, "cond_add": 150.0, "deferment": 0.0},
        "+ stabiliser": {"fixes": "1+2+3+5+7", "cap": package_cap, "capex": 77.0, "weeks": 8.0, "cond_add": 200.0, "deferment": 10.97},
        "Both add-ons": {"fixes": "1+2+3+5+6+7", "cap": package_cap, "capex": 99.0, "weeks": 8.0, "cond_add": 350.0, "deferment": 14.27},
    }

    cost = azoth.process.equipment_cost_estimate(
        [
            {
                "equipment": "separator",
                "name": "HP separator",
                "inner_diameter": q(D_HP, "m"),
                "tangent_length": q(L_HP, "m"),
                "weight_total": q(30000.0, "kg"),
                "mechanical_design_present": True,
                "max_design_pressure": q(66.0, "bar"),
            },
            {
                "equipment": "compressor",
                "name": "Export compressor",
                "power": q(2273.0, "kW"),
                "compressor_type": "centrifugal",
                "driver_type": "gas-turbine",
                "include_driver": True,
                "weight_total": q(27276.0, "kg"),
                "mechanical_design_present": True,
                "max_design_pressure": q(100.0, "bar"),
            },
        ]
    )
    topsides = azoth.process.topsides_facility_cost(
        [
            {
                "name": item.name,
                "equipment_type": "Vessels" if item.equipment_type == "vessel" else "Compressors",
                "total_module_cost": item.total_module_cost,
                "weight": weight,
            }
            for item, weight in zip(cost.equipment, [q(30000.0, "kg"), q(27276.0, "kg")], strict=True)
        ],
        facility_type="fixed_platform",
        project_context="greenfield",
    )

    results = {
        name: dcf(value["cap"], value["capex"], value["deferment"], value["cond_add"])
        for name, value in PACKAGES.items()
    }
    rec = results["Gas side + water"]
    hipps = results["HIPPS + water"]
    split = [
        {"case": "gas + condensate", "npv": rec["npv"]},
        {"case": "gas only (condensate price 0)", "npv": dcf(package_cap, 42.0, cond_price=0.0)["npv"]},
        {"case": "condensate only (gas price 0)", "npv": dcf(package_cap, 42.0, gas_price=0.0)["npv"]},
        {"case": "pre-tax", "npv": dcf(package_cap, 42.0, tax=0.0)["npv"]},
    ]

    tornado_inputs = [
        ("Gas price 0", dcf(package_cap, 42.0, gas_price=0.0)["npv"]),
        ("Gas price 8", dcf(package_cap, 42.0, gas_price=8.0)["npv"]),
        ("Condensate 50", dcf(package_cap, 42.0, cond_price=50.0)["npv"]),
        ("Condensate 90", dcf(package_cap, 42.0, cond_price=90.0)["npv"]),
        ("Capex -30%", dcf(package_cap, 42.0, capex_mult=0.7)["npv"]),
        ("Capex +30%", dcf(package_cap, 42.0, capex_mult=1.3)["npv"]),
        ("Decline 3%", dcf(package_cap, 42.0, decline=0.03)["npv"]),
        ("Decline 7%", dcf(package_cap, 42.0, decline=0.07)["npv"]),
        ("Discount 8%", dcf(package_cap, 42.0, discount=0.08)["npv"]),
        ("Discount 12%", dcf(package_cap, 42.0, discount=0.12)["npv"]),
        ("Water cut -10 pp", dcf(package_cap, 42.0, wc_start=WC_START - 0.10, wc_end=WC_END - 0.10)["npv"]),
        ("Water cut +10 pp", dcf(package_cap, 42.0, wc_start=WC_START + 0.10, wc_end=WC_END + 0.10)["npv"]),
        ("Shutdown slip 2 wk", dcf(package_cap, 42.0, deferment=10.97)["npv"]),
        ("HIPPS capex", dcf(hipps_cap, 36.0)["npv"]),
    ]
    tornado = sorted(tornado_inputs, key=lambda item: abs(item[1] - rec["npv"]))
    schedule = []
    for name, value in PACKAGES.items():
        weeks = value["weeks"]
        if weeks <= 4.0:
            cls, extension, premium = "fits", 0.0, 0.0
        elif weeks <= 6.0:
            cls, extension, premium = "compressed", 0.0, 3.3 if name == "+ fourth stage" else 0.0
        else:
            cls, extension, premium = "extends", weeks - 6.0, 0.0
        schedule.append(
            {
                "package": name,
                "weeks": weeks,
                "class": cls,
                "extension_weeks": extension,
                "labour_premium": premium,
                "deferment": value["deferment"],
            }
        )

    payload = {
        "meta": {
            "azoth_version": azoth.__version__,
            "backends": sorted(azoth.backends()),
            "components": COMPONENTS,
            "molar_mass_kg_per_mol": MOLAR_MASS,
        },
        "basis": {
            "p_wh_bar": P_WH,
            "t_wh_k": T_WH,
            "p_hp_bar": P_HP,
            "p_mp_bar": P_MP,
            "p_lp_bar": P_LP,
            "p_export_bar": P_EXPORT,
            "t_hp_k": T_HP,
            "t_cool_k": T_COOL,
            "hc_current_mol_s": HC_CURRENT,
            "hc_potential_mol_s": HC_POTENTIAL,
            "water_mol_s": WATER_RATE,
            "d_hp_m": D_HP,
            "l_hp_m": L_HP,
            "k_design": K_DESIGN,
            "k_upgrade": K_UPGRADE,
            "driver_kw": DRIVER_KW,
            "psv_set_bar": PSV_SET,
            "psv_area_in2": PSV_AREA_IN2,
            "relief_pressure_bar": RELIEF_PRESSURE,
            "water_capacity_bpd": WATER_CAPACITY,
            "water_upgrade_bpd": WATER_UPGRADE,
        },
        "split": {
            "current": {
                key: current_split[key]
                for key in (
                    "total", "vapour_n", "light_liquid_n", "heavy_liquid_n",
                    "oil_retention_s", "water_retention_s", "settling_s",
                    "settling_ok", "phase_count", "phase_fractions", "mode", "seeded",
                )
            },
            "potential": {
                "vapour_n": potential_split["vapour_n"],
                "light_liquid_n": potential_split["light_liquid_n"],
                "heavy_liquid_n": potential_split["heavy_liquid_n"],
            },
            "z_current": current_split["z"],
            "z_vapour_current": current_split["vapour_z"],
            "z_light_liquid_current": current_split["light_liquid_z"],
            "z_heavy_liquid_current": current_split["heavy_liquid_z"],
            "water_bpd": water_bpd(current_split["heavy_liquid_n"]),
            "robustness": [
                {"water_mol_pct": 13.452915, "water_bpd": water_bpd(current_split["heavy_liquid_n"]), "phases": current_split["phase_count"], "water_retention_min": current_split["water_retention_s"] / 60.0},
                {"water_mol_pct": 16.0, "water_bpd": water_bpd(hp_split(HC_CURRENT, HC_CURRENT * 0.16 / 0.84)["heavy_liquid_n"]), "phases": hp_split(HC_CURRENT, HC_CURRENT * 0.16 / 0.84)["phase_count"], "water_retention_min": (hp_split(HC_CURRENT, HC_CURRENT * 0.16 / 0.84)["water_retention_s"] or 0.0) / 60.0},
                {"water_mol_pct": 18.0, "water_bpd": water_bpd(hp_split(HC_CURRENT, HC_CURRENT * 0.18 / 0.82)["heavy_liquid_n"]), "phases": hp_split(HC_CURRENT, HC_CURRENT * 0.18 / 0.82)["phase_count"], "water_retention_min": (hp_split(HC_CURRENT, HC_CURRENT * 0.18 / 0.82)["water_retention_s"] or 0.0) / 60.0},
            ],
        },
        "train": {
            "current": {
                "gas_mmscfd": current["gas_mmscfd"],
                "bpd": current["bpd"],
                "water_bpd": current["water_bpd"],
                "water_cut": current["water_cut"],
                "cgr": current["cgr"],
                "power_kw": current["power_kw"],
                "duty_kw": current["duty_kw"],
                "pump_kw": current["pump_kw"],
                "hp_util": current["hp_util"],
                "hp_k": current["hp_k"],
                "oil_retention_min": (current["oil_retention_s"] or 0.0) / 60.0,
                "water_retention_min": (current["water_retention_s"] or 0.0) / 60.0,
                "settling_s": current["settling_s"],
                "settling_ok": current["settling_ok"],
                "mode": current["mode"],
                "mp_model": current["mp_model"],
                "lp_model": current["lp_model"],
                "hp_mass_kg_s": current["hp_mass"],
                "hp_n": current["hp_n"],
                "hp_z": current["hp_z"],
                "sales_n": current["sales_n"],
                "liquid_n": current["liquid_n"],
                "gas_z": current["gas_z"],
                "liquid_z": current["liquid_z"],
            },
            "potential": {
                "gas_mmscfd": potential["gas_mmscfd"],
                "bpd": potential["bpd"],
                "water_bpd": potential["water_bpd"],
                "water_cut": potential["water_cut"],
                "cgr": potential["cgr"],
                "power_kw": potential["power_kw"],
                "duty_kw": potential["duty_kw"],
                "hp_util": potential["hp_util"],
                "oil_retention_min": (potential["oil_retention_s"] or 0.0) / 60.0,
                "water_retention_min": (potential["water_retention_s"] or 0.0) / 60.0,
            },
            "stages": {
                "hp_vapour": current["hp"]["vapour_n"],
                "hp_oil": current["hp"]["light_liquid_n"],
                "hp_water": current["hp"]["heavy_liquid_n"],
                "mp_vapour": current["mp"].vapour_n.to("mol/s").magnitude,
                "mp_liquid": current["mp"].liquid_n.to("mol/s").magnitude,
                "lp_vapour": current["lp"].vapour_n.to("mol/s").magnitude,
                "lp_liquid": current["lp"].liquid_n.to("mol/s").magnitude,
            },
        },
        "capacity": {
            "utilisation": {
                "separator gas load": current["hp_util"],
                "compressor power": current["power_kw"] / DRIVER_KW,
                "produced water": current["water_bpd"] / WATER_CAPACITY,
                "flare": flare_current.mass_utilization,
                "gas line": base_gas / gas_line_cap,
                "PSV orifice": psv_current.required_area_in2 / PSV_AREA_IN2,
                "pump power": current["pump_kw"] / PUMP_KW,
                "liquid line": liquid_line_drop / (P_LIQ - P_RECEPTION),
            },
            "caps": caps,
            "fixed_caps": fixed_caps,
            "hipps_caps": hipps_caps,
            "gas_side_cap": gas_side_cap,
            "package_cap": package_cap,
            "hipps_cap": hipps_cap,
            "gas_line_arrival_current_bar": gas_line_current.outlet_pressure.to("bar").magnitude,
            "gas_line_arrival_potential_bar": gas_line_potential.outlet_pressure.to("bar").magnitude,
            "liquid_line_arrival_current_bar": liquid_line_current.outlet_pressure.to("bar").magnitude,
            "liquid_line_arrival_potential_bar": liquid_line_potential.outlet_pressure.to("bar").magnitude,
        },
        "relief": {
            "current": {
                "relief_kg_s": blocked_current.relief_rate.to("kg/s").magnitude,
                "psv_required_in2": psv_current.required_area_in2,
                "psv_utilisation": psv_current.required_area_in2 / PSV_AREA_IN2,
                "flare_mass_utilisation": flare_current.mass_utilization,
                "flare_overloaded": flare_current.overloaded,
                "radiation_distance_m": flare_current.radiation_distance.to("m").magnitude,
                "psv_capacity_kg_s": psv_current.mass_flow_capacity.to("kg/s").magnitude,
                "psv_recommended_orifice": psv_current.recommended_orifice,
            },
            "plus_18": {
                "relief_kg_s": blocked_18.relief_rate.to("kg/s").magnitude,
                "psv_required_in2": psv_18.required_area_in2,
                "psv_utilisation": psv_18.required_area_in2 / PSV_AREA_IN2,
                "flare_mass_utilisation": flare_18.mass_utilization,
                "flare_overloaded": flare_18.overloaded,
                "radiation_distance_m": flare_18.radiation_distance.to("m").magnitude,
            },
            "plus_18_flare25": {
                "relief_kg_s": blocked_18.relief_rate.to("kg/s").magnitude,
                "psv_required_in2": psv_18.required_area_in2,
                "flare_mass_utilisation": flare_25.mass_utilization,
                "flare_overloaded": flare_25.overloaded,
                "radiation_distance_m": flare_25.radiation_distance.to("m").magnitude,
            },
            "plus_18_hipps": {
                "relief_kg_s": blocked_hipps.relief_rate.to("kg/s").magnitude,
                "psv_required_in2": psv_hipps.required_area_in2,
                "psv_utilisation": psv_hipps.required_area_in2 / PSV_AREA_IN2,
                "flare_mass_utilisation": flare_hipps.mass_utilization,
                "flare_overloaded": flare_hipps.overloaded,
                "radiation_distance_m": flare_hipps.radiation_distance.to("m").magnitude,
            },
            "fire_kg_s": fire.relief_rate.to("kg/s").magnitude,
            "network_simultaneous_kg_s": network.total_simultaneous.to("kg/s").magnitude,
            "network_peak_single_kg_s": network.peak_single.to("kg/s").magnitude,
            "flare_design_mass_kg_s": flare_design_mass,
            "safety_case": {
                "nog070_minimum_sil": npd.minimum_sil,
                "nog070_achieved_sil": npd.achieved_sil,
                "nog070_compliant": npd.compliant,
                "sil_achieved": sil.achieved_sil,
                "sil_passed": sil.verification_passed,
                "sil_hft": sil.hardware_fault_tolerance,
                "sil_dc": sil.diagnostic_coverage,
                "pipeline_max_incidental_bar": ppp.max_incidental_pressure.to("bar").magnitude,
                "pipeline_verdict": ppp.verdict,
                "pipeline_protection_adequate": ppp.protection_adequate,
                "s001_acceptable": s001.acceptable,
            },
        },
        "fixes": FIXES,
        "packages": PACKAGES,
        "economics": {
            "results": {
                name: {
                    "capex": PACKAGES[name]["capex"],
                    "npv": value["npv"],
                    "payback": value["payback"],
                    "gas_pv": value["gas_pv"],
                    "cond_pv": value["cond_pv"],
                    "cum": value["cum"],
                    "cum_disc": value["cum_disc"],
                    "cash": value["cash"],
                    "inc_gas": value["inc_gas"],
                    "inc_cond": value["inc_cond"],
                    "inc_water": value["inc_water"],
                    "rev_gas": value["rev_gas"],
                    "rev_cond": value["rev_cond"],
                    "opex": value["opex"],
                    "tax": value["tax"],
                    "ebitda": value["ebitda"],
                }
                for name, value in results.items()
            },
            "split": split,
            "tornado": [{"case": name, "npv": value, "change": value - rec["npv"]} for name, value in tornado],
            "schedule": schedule,
            "deferment_basis": 2.0 / 52.0 * (current["gas_mmscfd"] * 365.0 * HEATING_VALUE * GAS_PRICE + current["bpd"] * 365.0 * COND_PRICE) / 1e6,
        },
        "cost_library": {
            "items": [
                {
                    "name": item.name,
                    "equipment_type": item.equipment_type,
                    "purchased": item.purchased_equipment_cost,
                    "module": item.total_module_cost,
                }
                for item in cost.equipment
            ],
            "topsides_greenfield_musd": topsides.project_cost_summary[0].cost / 1e6,
        },
    }
    out = Path(__file__).with_name("results.json")
    out.write_text(json.dumps(payload, indent=2))
    print(f"wrote {out}")
    print(
        f"gas {base_gas:.2f} MMscfd | condensate {current['bpd']:.0f} bbl/d | "
        f"water {current['water_bpd']:.0f} bbl/d | CGR {current['cgr']:.2f} | "
        f"water cut {current['water_cut']*100:.1f}%"
    )
    print(
        f"caps: separator {caps['HP separator']:.1f}, compressor {caps['Compressor']:.1f}, "
        f"flare {caps['Flare']:.1f}, water {caps['Water plant']:.1f} MMscfd"
    )
    print(
        f"package cap {package_cap:.1f} MMscfd; HIPPS NPV {hipps['npv']:.2f} MUSD; "
        f"gas+water NPV {rec['npv']:.2f} MUSD"
    )


if __name__ == "__main__":
    main()
