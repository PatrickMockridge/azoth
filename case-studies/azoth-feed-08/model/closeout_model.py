"""HAZOP close-out calculations for AZ-FEED-08 Rev C.

The Rev B HAZOP left ten actions open. This module closes the technical ones with
calculations and records the design changes they force. The governing decision is to
withdraw the unproven 35 % HIPPS relief credit, book the full +18 % blocked-outlet case,
and select the +25 % flare/header upgrade. The remaining actions are closed by design
change, procedure or purchase-order condition.

Run: python model/closeout_model.py
"""

from __future__ import annotations

import json
import math
from pathlib import Path

import azoth
import azoth.eos as eos
import azoth.process as process

import case_model as base

ROOT = Path(__file__).resolve().parents[1]
RES = json.loads((ROOT / "model" / "results.json").read_text())
FEED = json.loads((ROOT / "model" / "feed_results.json").read_text())
q = azoth.ureg.Quantity

COMP = RES["meta"]["components"]
GAS_Z = RES["train"]["current"]["gas_z"]
HP_Z = RES["split"]["z_vapour_current"]
OIL_Z = RES["split"]["z_light_liquid_current"]
HP_MASS = RES["relief"]["plus_18"]["relief_kg_s"]          # 50.31 kg/s, full +18 % case
HIPPS_MASS = RES["relief"]["plus_18_hipps"]["relief_kg_s"]  # 32.70 kg/s, credited case

# ---------------------------------------------------------------- SIL (HAZ-02)
def pfd_1oo1(lam_du, test_h):
    return lam_du * test_h / 2.0


def pfd_1oo2(lam_du, beta, test_h):
    lt = lam_du * test_h
    return ((1.0 - beta) * lt) ** 2 / 3.0 + beta * lt / 2.0


def sil_block():
    # as designed: 1oo2 transmitters, 1oo1 logic solver, 1oo1 SDV, 12-month proof test
    pt_as = pfd_1oo2(5e-7, 0.10, 8760.0)
    ls_as = pfd_1oo1(1e-7, 8760.0)
    sdv_as = pfd_1oo1(1e-6, 8760.0)
    total_as = pt_as + ls_as + sdv_as
    # close-out: 1oo2 on all three, certified devices, 6-month proof test
    pt_up = pfd_1oo2(1e-7, 0.05, 4380.0)
    ls_up = pfd_1oo2(1e-7, 0.05, 4380.0)
    sdv_up = pfd_1oo2(1e-7, 0.05, 4380.0)
    total_up = pt_up + ls_up + sdv_up
    def sil(pfd):
        if pfd < 1e-4: return 4
        if pfd < 1e-3: return 3
        if pfd < 1e-2: return 2
        if pfd < 1e-1: return 1
        return 0
    return {
        "as_designed": {
            "architecture": "1oo2 PT / 1oo1 logic / 1oo1 SDV",
            "proof_test_months": 12,
            "pfd_pt": pt_as, "pfd_logic": ls_as, "pfd_sdv": sdv_as,
            "pfd_total": total_as, "achieved_sil": sil(total_as),
            "verdict": "does not achieve the NOG-070 minimum SIL 3",
        },
        "closed_out": {
            "architecture": "1oo2 PT / 1oo2 logic / 1oo2 SDV",
            "proof_test_months": 6,
            "assumptions": "vendor-certified lambda_DU <= 1e-7/h, beta <= 5 %, partial-stroke coverage",
            "pfd_pt": pt_up, "pfd_logic": ls_up, "pfd_sdv": sdv_up,
            "pfd_total": total_up, "achieved_sil": sil(total_up),
            "verdict": "meets the SIL 3 target; certificates are a purchase-order condition",
        },
    }


# ------------------------------------------------------- PSV hydraulics (HAZ-04)
def psv_hydraulics():
    n = HP_MASS / 0.023
    out = {}
    for label, dia in (("4 in", 0.1023), ("6 in", 0.1541)):
        r = eos.pipe_beggs_and_brills(
            COMP, q(n, "mol/s"), GAS_Z, q(66.0, "bar"), q(354.15, "K"), q(10.0, "m"),
            q(dia, "m"), q(0.0, "m"), 20, q(4.6e-5, "m"),
        )
        dp = 66.0 - r.outlet_pressure.to("bar").magnitude
        out[label] = {"dP_bar": dp, "dP_pct_set": dp / 66.0 * 100.0,
                      "passes_3pct": dp <= 0.03 * 66.0}
    # outlet velocity at 1.5 bara in a 6 in and 24 in line
    rho = 1.5e5 * 0.023 / (0.95 * 8.314462618 * 354.15)
    qv = HP_MASS / rho
    out["outlet"] = {
        "gas_density_kg_m3": rho, "volumetric_m3_s": qv,
        "velocity_6in_m_s": qv / (math.pi / 4 * 0.1541 ** 2),
        "velocity_24in_m_s": qv / (math.pi / 4 * 0.6096 ** 2),
        "selected_outlet": "24 in (about 140 m/s, below 0.5 Mach)",
    }
    out["verdict"] = "4 in inlet fails the API 520 3 % rule at 16.6 %; 6 in inlet passes at 1.7 %; outlet must be 24 in."
    return out


# ------------------------------------------------------ PSV leak sizing (HAZ-12/14)
def psv_register():
    area = math.pi / 4 * 0.010 ** 2   # 10 mm restriction orifice
    cases = [
        ("PSV-102", 60.0, 10.0, 12.0, 1.3, 0.030),
        ("PSV-103", 10.0, 2.0, 4.0, 1.25, 0.040),
        ("PSV-104", 60.0, 25.0, 25.0, 1.3, 0.030),
    ]
    rows = []
    for tag, pin, pout, pset, k, mm in cases:
        b = process.gas_blowby(q(pin, "bar"), q(354.15, "K"), q(pout, "bar"), q(area, "m**2"), k, q(mm, "kg/mol"))
        rate = b.blowby_mass_rate.to("kg/s").magnitude
        s = process.psv_sizing(
            "gas", q(pset, "bar"), 0.10, q(1.5, "bar"),
            mass_flow=q(rate, "kg/s"), temperature=q(354.15, "K"),
            molar_mass=q(mm, "kg/mol"), compressibility=0.75, specific_heat_ratio=k,
        )
        rows.append({"tag": tag, "leak_kg_s": rate, "set_bar": pset,
                     "required_in2": s.required_area_in2, "orifice": s.recommended_orifice,
                     "note": "10 mm restriction; full valve failure is isolated by ESDV"})
    return {"restriction_mm": 10.0, "rows": rows,
            "verdict": "PSVs sized for a 10 mm restricted leak; full Cv failure is unrelievable and is isolated by a new ESDV upstream."}


# ------------------------------------------------------ water dew point (HAZ-16)
def water_block():
    moles = [1000.0 * z for z in GAS_Z]
    sat = eos.saturate_with_water(COMP, moles, q(313.15, "K"), q(90.0, "bar"))
    xw = sat.water_mole_fraction
    mol_per_mmscf = 1e6 * 0.028316846592 / (8.314462618 * 288.15 / 101325.0)
    kg_per_mmscf = xw * mol_per_mmscf * 0.018015
    gas = RES["capacity"]["package_cap"]
    kg_h = kg_per_mmscf * gas / 24.0
    spec_kg = 3.0 * 0.45359237
    teg_m3h = 0.04 * kg_h
    reboiler_kw = 0.20 * kg_h
    return {
        "water_mole_fraction": xw,
        "water_kg_per_mmscf": kg_per_mmscf,
        "spec_kg_per_mmscf": spec_kg,
        "ratio_to_spec": kg_per_mmscf / spec_kg,
        "water_load_kg_h": kg_h,
        "teg_circulation_m3_h": teg_m3h,
        "reboiler_duty_kw": reboiler_kw,
        "verdict": "the gas is water-saturated at 313 K and 90 bara; the existing TEG must remove about 140 kg/h and meet 3 lb/MMscf.",
    }


# ------------------------------------------------------ radiation (HAZ-05)
def radiation():
    keep = [i for i, c in enumerate(COMP) if c not in ("co2", "water")]
    fz = [HP_Z[i] for i in keep]
    total = sum(fz)
    fz = [v / total for v in fz]
    fc = [COMP[i] for i in keep]
    rows = []
    for thr in (1.58, 4.73, 6.3):
        f = process.flare(
            fc, q(HP_MASS / 0.023 * 1.0, "mol/s"), fz, q(60.0, "bar"), q(354.15, "K"),
            ground_distance=q(100.0, "m"), radiation_flux_threshold=q(thr, "kW/m**2"),
            design_mass_flow_capacity=q(46.34, "kg/s"), design_molar_flow_capacity=q(2031.7, "mol/s"),
        )
        rows.append({"threshold_kw_m2": thr, "distance_m": f.radiation_distance.to("m").magnitude})
    return {"contours_plus18": rows,
            "verdict": "4.73 kW/m2 sterile contour 82.4 m; 1.58 kW/m2 personnel contour 148.8 m requires access control; 6.3 kW/m2 equipment contour 69.8 m."}


# ------------------------------------------------------ economics (Rev C)
def economics():
    base.BASE_GAS = RES["train"]["current"]["gas_mmscfd"]
    base.G0 = RES["train"]["potential"]["gas_mmscfd"]
    base.CGR_VALUE = RES["train"]["current"]["cgr"]
    base.WC_START = RES["train"]["current"]["water_cut"]
    cap = RES["capacity"]["package_cap"]
    cases = {
        "Rev B — HIPPS credit (withdrawn)": 36.0,
        "Flare upgrade only": 48.0,
        "Rev C — flare + close-out scope": 54.1,
    }
    out = {}
    for name, capex in cases.items():
        d = base.dcf(cap, capex)
        out[name] = {"capex_musd": capex, "npv_musd": d["npv"], "payback": d["payback"]}
    return out


def cost_block():
    items = [
        ("Rev B recommended package", 36.0),
        ("Flare/header +25 % (relief basis without credit)", 12.0),
        ("1oo2 final element and SIL-3 hardware", 1.2),
        ("PSV-101 6 in inlet / 24 in outlet, PSV-102/103/104, ESDV-102/103, restrictors", 1.8),
        ("F&G, ESD/BDV and cause-and-effect additions", 1.0),
        ("TEG dehydration upgrade allowance", 1.5),
        ("Driver 3.0 MW (10 % margin over 2.689 MW)", 0.3),
        ("Sterile-area access, flare boom and survey allowance", 0.3),
    ]
    total = sum(v for _, v in items)
    return {"items": [{"item": i, "musd": v} for i, v in items], "total_musd": total}


def main():
    payload = {
        "meta": {"basis": "AZ-FEED-08 Rev B frozen design", "revision": "C",
                 "date": "2026-11-26", "library": RES["meta"]["azoth_version"]},
        "sil": sil_block(),
        "psv_hydraulics": psv_hydraulics(),
        "psv_register": psv_register(),
        "water": water_block(),
        "radiation": radiation(),
        "economics": economics(),
        "cost": cost_block(),
        "relief_basis": {
            "full_case_kg_s": HP_MASS, "credited_case_kg_s": HIPPS_MASS,
            "flare_pct_full_case_existing": RES["relief"]["plus_18"]["flare_mass_utilisation"] * 100.0,
            "flare_pct_full_case_upgraded": RES["relief"]["plus_18_flare25"]["flare_mass_utilisation"] * 100.0,
            "psv_required_in2": RES["relief"]["plus_18"]["psv_required_in2"],
            "psv_fitted_in2": RES["design_basis"]["psv_area_in2"] if "design_basis" in RES else 6.38,
            "decision": "withdraw the 35 % credit; book 50.31 kg/s and the +25 % flare upgrade",
        },
    }
    out = ROOT / "model" / "closeout_results.json"
    out.write_text(json.dumps(payload, indent=2, default=str))
    print("wrote", out)
    print("SIL as-designed PFD %.2e (SIL %d), closed-out %.2e (SIL %d)" % (
        payload["sil"]["as_designed"]["pfd_total"], payload["sil"]["as_designed"]["achieved_sil"],
        payload["sil"]["closed_out"]["pfd_total"], payload["sil"]["closed_out"]["achieved_sil"]))
    print("PSV 4 in dP %.1f%%, 6 in %.1f%%" % (
        payload["psv_hydraulics"]["4 in"]["dP_pct_set"], payload["psv_hydraulics"]["6 in"]["dP_pct_set"]))
    print("water %.1f kg/MMscf (%.1fx spec), %.0f kg/h" % (
        payload["water"]["water_kg_per_mmscf"], payload["water"]["ratio_to_spec"], payload["water"]["water_load_kg_h"]))
    print("Rev C capex %.1f, NPV %.2f" % (payload["cost"]["total_musd"], payload["economics"]["Rev C — flare + close-out scope"]["npv_musd"]))


if __name__ == "__main__":
    main()
