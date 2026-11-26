"""FEED additions to the notebook-08 debottlenecking model.

Reads model/results.json (the reproduced notebook case) and adds the engineering
that a FEED package owes on top of the screening study:

  * design basis and operating cases
  * CO2 service, corrosion allowance and material selection
  * pressure-containing wall thickness checks (ASME VIII-1 and B31.3)
  * equipment datasheets (process and mechanical)
  * discipline manhour estimate
  * AACE Class 4/5 cost build-up that reconciles to the 36 MUSD estimate of record
  * resource-loaded 42-week project plan with manhour histogram and S-curve
  * deliverable register

Everything here is a study calculation. It inherits the notebook's illustrative
field and the library's partially_verified status; none of it is a design deliverable.

Run:  python model/feed_model.py
"""

from __future__ import annotations

import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE = json.loads((ROOT / "model" / "results.json").read_text())
B = BASE["basis"]
TR = BASE["train"]["current"]
CAP = BASE["capacity"]
REL = BASE["relief"]
PKG = BASE["packages"]
ECO = BASE["economics"]

# ---------------------------------------------------------------- constants
LIFE_YEARS = 20.0
INHIBITED_RATE = 0.15        # mm/yr, inhibited carbon steel, screening band
UNINHIBITED_RATE = 1.50      # mm/yr, uninhibited at 0.7 bar CO2 and 81 C
CA_VESSEL = 3.0              # mm
CA_PIPE = 3.0                # mm

SMYS_X65 = 448.0             # MPa
S_X65 = 0.72 * SMYS_X65      # B31.3 basic allowable at ambient, MPa
S_316L = 138.0               # MPa at design temperature
S_SA516 = 138.0              # MPa at 100 C
E_JOINT = 1.0

RATES = {"engineering": 110.0, "fabrication": 85.0, "offshore": 180.0}


def r(x, n=2):
    return round(float(x), n)


def co2_case():
    comps = BASE["meta"]["components"]
    i = comps.index("co2")
    z_vap = BASE["split"]["z_vapour_current"][i]
    z_feed = BASE["split"]["z_current"][i]
    p_hp = B["p_hp_bar"]
    p_vap = z_vap * p_hp
    p_feed = z_feed * p_hp
    ca = INHIBITED_RATE * LIFE_YEARS
    return {
        "co2_mol_pct_vapour": r(z_vap * 100, 2),
        "co2_mol_pct_feed": r(z_feed * 100, 2),
        "p_co2_vapour_bar": r(p_vap, 3),
        "p_co2_feed_bar": r(p_feed, 3),
        "temperature_c": r(B["t_hp_k"] - 273.15, 1),
        "inhibited_rate_mm_yr": INHIBITED_RATE,
        "uninhibited_rate_mm_yr": UNINHIBITED_RATE,
        "design_life_years": LIFE_YEARS,
        "ca_required_mm": r(ca, 2),
        "ca_selected_mm": CA_VESSEL,
        "uninhibited_ca_mm": r(UNINHIBITED_RATE * LIFE_YEARS, 1),
        "verdict": (
            "CO2 partial pressure below 1 bar and temperature below 100 C: carbon steel "
            "with continuous inhibition and a 3 mm corrosion allowance is acceptable. "
            "Inhibition is a barrier, not a calculation; it must be monitored."
        ),
    }


def vessel_t(p_mpa, radius_mm, s_mpa, ca_mm, e=1.0):
    """ASME VIII-1 UG-27 cylindrical shell, circumferential stress."""
    t = p_mpa * radius_mm / (s_mpa * e - 0.6 * p_mpa)
    return t, t + ca_mm


def pipe_t(p_mpa, od_mm, s_mpa, ca_mm, e=1.0, y=0.4):
    """ASME B31.3 straight pipe, eq. (3a)."""
    t = p_mpa * od_mm / (2.0 * (s_mpa * e + p_mpa * y))
    return t, t + ca_mm


def mechanical():
    v = vessel_t(6.6, B["d_hp_m"] * 1000.0 / 2.0, S_SA516, CA_VESSEL)
    gas = pipe_t(10.0, 323.9, S_X65, CA_PIPE)
    cond = pipe_t(3.0, 168.3, S_X65, CA_PIPE)
    water = pipe_t(2.5, 219.1, S_316L, 0.0)
    hipps = pipe_t(10.0, 273.0, S_316L, 0.0)
    return {
        "v101": {
            "design_pressure_barg": 66.0, "design_temperature_c": 100.0,
            "inside_diameter_mm": r(B["d_hp_m"] * 1000.0, 0),
            "tangent_length_mm": r(B["l_hp_m"] * 1000.0, 0),
            "s_mpa": S_SA516, "e": E_JOINT,
            "t_required_mm": r(v[0], 2), "ca_mm": CA_VESSEL, "t_with_ca_mm": r(v[1], 2),
            "t_selected_mm": r(math.ceil(v[1] / 2.0) * 2.0, 0),
        },
        "gas_line": {
            "size_in": 12, "od_mm": 323.9, "design_pressure_barg": 100.0,
            "material": "API 5L X65 PSL2", "s_mpa": r(S_X65, 0),
            "t_required_mm": r(gas[0], 2), "ca_mm": CA_PIPE, "t_with_ca_mm": r(gas[1], 2),
            "t_selected_mm": 12.7, "schedule": "12.7 mm WT",
        },
        "condensate_line": {
            "size_in": 6, "od_mm": 168.3, "design_pressure_barg": 30.0,
            "material": "API 5L X65 PSL2", "s_mpa": r(S_X65, 0),
            "t_required_mm": r(cond[0], 2), "ca_mm": CA_PIPE, "t_with_ca_mm": r(cond[1], 2),
            "t_selected_mm": 7.1, "schedule": "SCH 40",
        },
        "water_line": {
            "size_in": 8, "od_mm": 219.1, "design_pressure_barg": 25.0,
            "material": "ASTM A312 TP316L", "s_mpa": S_316L,
            "t_required_mm": r(water[0], 2), "ca_mm": 0.0, "t_with_ca_mm": r(water[1], 2),
            "t_selected_mm": 3.76, "schedule": "SCH 10S",
        },
        "hipps_spool": {
            "size_in": 10, "od_mm": 273.0, "design_pressure_barg": 100.0,
            "material": "ASTM A312 TP316L", "s_mpa": S_316L,
            "t_required_mm": r(hipps[0], 2), "ca_mm": 0.0, "t_with_ca_mm": r(hipps[1], 2),
            "t_selected_mm": 12.7, "schedule": "12.7 mm WT",
        },
    }


def materials():
    return {
        "philosophy": [
            "Carbon steel is the default for the dry and wet hydrocarbon service, with a 3.0 mm corrosion allowance and continuous CO2 inhibition.",
            "316L is used for the produced-water package, HIPPS wetted parts and instrument connections where carbon-steel corrosion products would foul the water treatment.",
            "No H2S is present, so NACE MR0175/ISO 15156 is not triggered; B7M bolting is nonetheless specified as a sour-service margin for future composition change.",
            "External protection is ISO 12944 C5-M at 320 micrometres; hot surfaces above 60 C are PIR-insulated with 0.5 mm stainless cladding.",
            "The flare tip is Inconel 625; the header is carbon steel with a 3 mm allowance and external ceramic insulation.",
        ],
        "schedule": [
            {"item": "V-101 shell and heads", "material": "SA-516 Gr 70N", "spec": "ASME VIII-1", "ca_mm": 3.0, "notes": "existing vessel; internals change only"},
            {"item": "V-101 internals", "material": "316L", "spec": "ASME VIII-1 / vendor", "ca_mm": 0.0, "notes": "inlet device, vane pack, weir"},
            {"item": "V-102 / V-103 shells", "material": "SA-516 Gr 70N", "spec": "ASME VIII-1", "ca_mm": 3.0, "notes": "existing, no change"},
            {"item": "K-101 casing / impellers", "material": "CS / 17-4PH", "spec": "API 617", "ca_mm": 0.0, "notes": "vendor re-rate kit"},
            {"item": "HIPPS body and spool", "material": "316L", "spec": "API 6D / IEC 61508", "ca_mm": 0.0, "notes": "SIL-3 loop"},
            {"item": "Hydrocyclone liners", "material": "Super Duplex 25Cr", "spec": "vendor", "ca_mm": 0.0, "notes": "produced water"},
            {"item": "Coalescer vessel", "material": "316L", "spec": "ASME VIII-1", "ca_mm": 0.0, "notes": "produced water"},
            {"item": "Produced-water piping", "material": "316L / GRP", "spec": "B31.3", "ca_mm": 0.0, "notes": "sweet, chloride-bearing"},
            {"item": "Export gas line", "material": "API 5L X65 PSL2", "spec": "B31.3 / DNV", "ca_mm": 3.0, "notes": "3LPP external"},
            {"item": "Condensate line", "material": "API 5L X65 PSL2", "spec": "B31.3", "ca_mm": 3.0, "notes": "3LPP external"},
            {"item": "Flare tip", "material": "Inconel 625", "spec": "API 537", "ca_mm": 0.0, "notes": "fallback scope"},
            {"item": "Structural steel", "material": "S355 J2", "spec": "EN 1993", "ca_mm": 0.0, "notes": "H-60 fireproofing at HP"},
            {"item": "Bolting", "material": "A193 B7M / A194 2HM", "spec": "ASME", "ca_mm": 0.0, "notes": "sour margin"},
            {"item": "Gaskets", "material": "Spiral wound 316L / graphite", "spec": "ASME B16.20", "ca_mm": 0.0, "notes": "all hydrocarbon flanges"},
        ],
        "line_classes": [
            {"class": "A1", "service": "Wet HP gas", "size_in": "12", "material": "X65", "rating": "ANSI 900", "ca_mm": 3.0, "insulation": "PIR + SS cladding"},
            {"class": "A2", "service": "Wet HP/MP oil", "size_in": "6", "material": "X65", "rating": "ANSI 900", "ca_mm": 3.0, "insulation": "PIR + SS cladding"},
            {"class": "A3", "service": "Produced water", "size_in": "8", "material": "316L", "rating": "ANSI 300", "ca_mm": 0.0, "insulation": "none"},
            {"class": "A4", "service": "Relief / flare", "size_in": "12", "material": "CS", "rating": "ANSI 300", "ca_mm": 3.0, "insulation": "ceramic fibre"},
            {"class": "A5", "service": "Condensate export", "size_in": "6", "material": "X65", "rating": "ANSI 600", "ca_mm": 3.0, "insulation": "cellular glass"},
        ],
    }


def datasheets():
    water_bpd = TR["water_bpd"]
    q_m3h = 6000.0 * 0.158987 / 24.0   # capacity, not current flow
    pump_head = 60.0
    pump_eff = 0.65
    pump_kw = 1000.0 * 9.81 * (q_m3h / 3600.0) * pump_head / pump_eff / 1000.0
    return [
        {
            "tag": "V-101", "service": "HP three-phase separator — internals",
            "type": "Horizontal 3-phase separator, existing",
            "design_p": "66 barg", "design_t": "100 C", "size": "ID 1.96 m x 7.26 m T/T",
            "material": "SA-516 Gr 70N + 316L internals", "ca": "3.0 mm",
            "internals": "Inlet device, vane pack, weir and interface control; K 0.11 -> 0.14 m/s",
            "nozzles": "Feed 12 in; gas 12 in; oil 6 in; water 8 in; PSV 4 in; instruments 2 in",
            "weight": "30 t dry", "insulation": "PIR 50 mm + SS cladding", "code": "ASME VIII-1",
            "qty": 1, "scope": "Modify internals",
        },
        {
            "tag": "K-101", "service": "Export compressor — re-rate",
            "type": "Centrifugal, 2-stage, existing",
            "design_p": "100 barg", "design_t": "120 C", "size": "2,273 kW absorbed now; 2,689 kW at potential",
            "material": "CS casing, 17-4PH impellers", "ca": "n/a",
            "internals": "Revised impellers/bundle, dry gas seals, driver re-rate to 2.7 MW",
            "nozzles": "Suction 16 in; discharge 12 in", "weight": "27.3 t",
            "insulation": "none (acoustic lagging)", "code": "API 617 / API 614",
            "qty": 1, "scope": "Re-rate +18%",
        },
        {
            "tag": "SDV-101", "service": "HIPPS final element — HP inlet isolation",
            "type": "SIL-3 shutdown valve, 1oo2 initiators",
            "design_p": "100 barg", "design_t": "100 C", "size": "10 in",
            "material": "316L body, Duplex trim", "ca": "0.0 mm",
            "internals": "2 x PT, SIL-3 logic solver, partial-stroke test, <2 s closure",
            "nozzles": "10 in ANSI 900", "weight": "1.2 t",
            "insulation": "none", "code": "API 6D / IEC 61508",
            "qty": 1, "scope": "New HIPPS",
        },
        {
            "tag": "V-105", "service": "Produced-water hydrocyclone",
            "type": "Package hydrocyclone, 2 x 50%",
            "design_p": "25 barg", "design_t": "80 C", "size": "6,000 bbl/d total",
            "material": "Super Duplex 25Cr liners", "ca": "0.0 mm",
            "internals": "12 mm liners, reject control valve",
            "nozzles": "Inlet 8 in; water 8 in; reject 2 in", "weight": "3.5 t",
            "insulation": "none", "code": "ASME VIII-1 / vendor",
            "qty": 1, "scope": "New",
        },
        {
            "tag": "V-106", "service": "Produced-water coalescer",
            "type": "Horizontal coalescer, 1.2 m ID x 4.0 m T/T",
            "design_p": "25 barg", "design_t": "80 C", "size": "6,000 bbl/d, 6.5 min retention (4.3 m3)",
            "material": "316L", "ca": "0.0 mm",
            "internals": "Plate pack / coalescing media, oil skimmer",
            "nozzles": "Inlet 8 in; water 8 in; oil 2 in", "weight": "4.0 t",
            "insulation": "none", "code": "ASME VIII-1",
            "qty": 1, "scope": "New",
        },
        {
            "tag": "PSV-102", "service": "MP let-down overpressure protection",
            "type": "Conventional PSV, V-102",
            "design_p": "12 barg set", "design_t": "100 C", "size": "Sized for VLV-102 fail-open gas blowby",
            "material": "316L trim", "ca": "0.0 mm",
            "internals": "Sized at detailed design; outlet to MP flare/vent header",
            "nozzles": "2 in inlet x 3 in outlet", "weight": "0.1 t",
            "insulation": "none", "code": "API 520 / API 521",
            "qty": 1, "scope": "New (HAZOP action)",
        },
        {
            "tag": "PSV-103", "service": "LP let-down overpressure protection",
            "type": "Conventional PSV, V-103",
            "design_p": "4 barg set", "design_t": "100 C", "size": "Sized for VLV-103 fail-open gas blowby",
            "material": "316L trim", "ca": "0.0 mm",
            "internals": "Sized at detailed design; outlet to LP flare/vent header",
            "nozzles": "2 in inlet x 3 in outlet", "weight": "0.1 t",
            "insulation": "none", "code": "API 520 / API 521",
            "qty": 1, "scope": "New (HAZOP action)",
        },
        {
            "tag": "PSV-104", "service": "Produced-water package overpressure protection",
            "type": "Conventional PSV, V-106",
            "design_p": "25 barg set", "design_t": "80 C", "size": "Sized for LCV-102 fail-open / blocked outlet",
            "material": "316L trim", "ca": "0.0 mm",
            "internals": "Outlet routed to a safe location; no relief to the water treatment",
            "nozzles": "2 in inlet x 3 in outlet", "weight": "0.1 t",
            "insulation": "none", "code": "API 520 / API 521",
            "qty": 1, "scope": "New (HAZOP action)",
        },
        {
            "tag": "P-102", "service": "Treated-water pump (2 x 50%)",
            "type": "Centrifugal, API 610 OH1",
            "design_p": "25 barg", "design_t": "80 C", "size": f"2 x 50% at {q_m3h/2:.1f} m3/h ({q_m3h:.1f} m3/h total) at {pump_head:.0f} m",
            "material": "316L casing / Duplex trim", "ca": "0.0 mm",
            "internals": f"{pump_kw:.1f} kW absorbed per duty pump; 15 kW motor; one standby",
            "nozzles": "Suction 4 in; discharge 3 in", "weight": "0.8 t",
            "insulation": "none", "code": "API 610",
            "qty": 1, "scope": "New",
        },
        {
            "tag": "PSV-101", "service": "HP separator relief",
            "type": "Conventional PSV, P orifice",
            "design_p": "66 barg set", "design_t": "100 C", "size": "6.38 in2 fitted; 3.60 in2 required with HIPPS",
            "material": "316L trim", "ca": "0.0 mm",
            "internals": "Balanced bellows not required (constant back pressure 1.5 bar)",
            "nozzles": "4 in inlet x 6 in outlet", "weight": "0.2 t",
            "insulation": "none", "code": "API 520 / API 526",
            "qty": 1, "scope": "No change; re-certify",
        },
        {
            "tag": "Flare", "service": "Relief disposal — fallback scope",
            "type": "Existing flare and header",
            "design_p": "10 barg", "design_t": "600 C", "size": "46.34 kg/s design; 91.1% loaded",
            "material": "CS header, Inconel 625 tip", "ca": "3.0 mm",
            "internals": "+25% capacity tip and header if HIPPS is not approved",
            "nozzles": "Header 24 in", "weight": "n/a",
            "insulation": "ceramic fibre", "code": "API 537 / API 521",
            "qty": 1, "scope": "Fallback +25%",
        },
    ]


# ------------------------------------------------------------------ manhours
MANHOURS = [
    # discipline, engineering, fabrication, offshore
    ("Process", 1200, 0, 0),
    ("Mechanical", 1800, 2400, 1200),
    ("Piping", 2400, 3200, 1800),
    ("Electrical & instrumentation", 2600, 1600, 1500),
    ("Structural", 800, 1200, 1000),
    ("Safety & risk", 1100, 0, 300),
    ("Project engineering & controls", 2000, 400, 500),
    ("Procurement & expediting", 1400, 0, 0),
    ("QA / commissioning", 900, 800, 2500),
]


def manhour_block():
    rows = []
    tot = {"engineering": 0, "fabrication": 0, "offshore": 0}
    for name, e, f, o in MANHOURS:
        rows.append({"discipline": name, "engineering": e, "fabrication": f, "offshore": o,
                     "total": e + f + o})
        tot["engineering"] += e; tot["fabrication"] += f; tot["offshore"] += o
    tot["total"] = sum(tot.values())
    cost = {k: r(tot[k] * RATES[k] / 1e6, 3) for k in RATES}
    return {"rows": rows, "totals": tot, "rates_usd_per_h": RATES,
            "labour_cost_musd": cost, "labour_cost_total_musd": r(sum(cost.values()), 3)}


# ------------------------------------------------------------------ cost
COST = {
    "equipment": [
        ("Compressor re-rate kit", 11.0),
        ("HP separator internals", 2.3),
        ("HIPPS SIL-3", 2.9),
        ("Produced-water package", 3.8),
    ],
    "bulk": [
        ("Piping, valves and fittings", 2.0),
        ("Electrical and instrumentation", 1.4),
        ("Structural and architectural", 0.8),
    ],
    "fabrication": [
        ("Shop labour (9,600 h)", 0.82),
        ("Consumables and materials", 0.50),
        ("NDT and testing", 0.30),
        ("Painting and insulation", 0.20),
        ("Shop overhead", 0.28),
    ],
    "offshore": [
        ("Marine spread and logistics", 1.85),
        ("Offshore labour (8,800 h)", 1.58),
        ("Scaffolding and access", 0.40),
        ("Commissioning consumables", 0.47),
    ],
    "engineering": [
        ("Engineering labour (14,200 h)", 1.56),
        ("Specialist studies (SIL, relief, structural)", 0.44),
        ("Third-party and software", 0.16),
        ("Project controls", 0.24),
    ],
    "owner": [
        ("Operations readiness and training", 0.50),
        ("Spares and first fills", 0.50),
        ("Permits, inspection and MOC", 0.30),
    ],
    "precommissioning": [("Pre-commissioning and start-up", 1.70)],
}


def cost_block():
    groups = []
    total = 0.0
    for name, items in COST.items():
        sub = sum(v for _, v in items)
        total += sub
        groups.append({"group": name, "items": [{"item": i, "musd": v} for i, v in items],
                       "subtotal_musd": r(sub, 2)})
    return {"groups": groups, "total_musd": r(total, 2),
            "hazop_allowance_musd": 1.8,
            "hazop_note": "HAZOP actions (level trips, PSVs, standby pump, larger coalescer, F&G/ESD) are estimated at 1.8 MUSD and sit inside the Class 4/5 accuracy band; the 36.0 MUSD estimate of record is unchanged pending the Class 3 estimate.",
            "estimate_of_record_musd": PKG["HIPPS + water"]["capex"],
            "reconciles": abs(total - PKG["HIPPS + water"]["capex"]) < 1e-6,
            "class": "AACE Class 4/5", "accuracy": "-30% / +50%",
            "contingency": "None applied; allowances carry vendor and installation contingency."}


# --------------------------------------------------------------- schedule
# (id, phase, name, start, duration, predecessors, discipline, hours, cost_musd)
ACTIVITIES = [
    ("A1", "A Design", "FEED close-out and design basis freeze", 0, 2, "", "Process", 320, 0.35),
    ("A2", "A Design", "Relief, flare and HIPPS basis update", 2, 3, "A1", "Safety & risk", 480, 0.53),
    ("A3", "A Design", "Mechanical datasheets and vendor enquiry", 3, 4, "A1", "Mechanical", 620, 0.68),
    ("A4", "A Design", "Piping, E&I and structural detailed design", 4, 6, "A3", "Piping", 1500, 1.65),
    ("A5", "A Design", "HAZOP, SIL review and design freeze", 7, 3, "A2,A4", "Safety & risk", 420, 0.46),
    ("A6", "A Design", "Approved-for-construction issue", 10, 1, "A5", "Project engineering & controls", 180, 0.20),
    ("B1", "B Procurement", "Compressor re-rate purchase order", 8, 1, "A3", "Procurement & expediting", 220, 0.24),
    ("B2", "B Procurement", "HIPPS purchase order", 8, 1, "A3", "Procurement & expediting", 160, 0.18),
    ("B3", "B Procurement", "Separator internals purchase order", 8, 1, "A3", "Procurement & expediting", 150, 0.17),
    ("B4", "B Procurement", "Produced-water package purchase order", 8, 1, "A3", "Procurement & expediting", 170, 0.19),
    ("B5", "B Procurement", "Compressor kit manufacture and FAT", 9, 22, "B1", "Mechanical", 900, 0.99),
    ("B6", "B Procurement", "HIPPS manufacture and FAT", 9, 12, "B2", "Electrical & instrumentation", 520, 0.57),
    ("B7", "B Procurement", "Internals manufacture", 9, 14, "B3", "Mechanical", 700, 0.77),
    ("B8", "B Procurement", "Water package manufacture and FAT", 9, 16, "B4", "Mechanical", 820, 0.90),
    ("C1", "C Readiness", "Offshore survey and work-pack development", 26, 4, "A6", "Piping", 600, 0.66),
    ("C2", "C Readiness", "Isolation, gas-free and permit plan", 28, 3, "C1", "Safety & risk", 260, 0.29),
    ("C3", "C Readiness", "Logistics, marine spread and materials load-out", 28, 4, "B5,B6,B7,B8", "Project engineering & controls", 340, 0.37),
    ("D1", "D Shutdown", "Shutdown start, isolation and gas-free", 32, 0.5, "C2,C3", "Safety & risk", 160, 0.18),
    ("D2", "D Shutdown", "Compressor re-rate installation", 32.5, 3, "D1", "Mechanical", 760, 0.84),
    ("D3", "D Shutdown", "Separator internals replacement", 32.5, 3, "D1", "Mechanical", 520, 0.57),
    ("D4", "D Shutdown", "HIPPS installation and loop test", 32.5, 2, "D1", "Electrical & instrumentation", 380, 0.42),
    ("D5", "D Shutdown", "Produced-water package installation", 32.5, 3.5, "D1", "Piping", 900, 0.99),
    ("D6", "D Shutdown", "Tie-ins, leak test and reinstatement", 34.5, 1.5, "D2,D3,D4,D5", "Piping", 640, 0.70),
    ("D7", "D Shutdown", "Commissioning and first gas", 35.5, 1.5, "D6", "QA / commissioning", 380, 0.42),
    ("E1", "E Ramp-up", "Performance test and ramp-up", 36, 2, "D7", "QA / commissioning", 240, 0.26),
    ("E2", "E Ramp-up", "Operating procedures, training and MOC close-out", 36, 4, "D7", "Safety & risk", 300, 0.33),
    ("F1", "F Close-out", "Close-out, lessons learned and benefits realisation", 40, 2, "E1,E2", "Project engineering & controls", 160, 0.18),
]

PHASE_COLOURS = {"A Design": "#2a78d6", "B Procurement": "#1baf7a", "C Readiness": "#eda100",
                 "D Shutdown": "#eb6834", "E Ramp-up": "#7d5ba6", "F Close-out": "#5a5751"}


def schedule_block(total_target=32600.0):
    acts = []
    raw = [a[7] for a in ACTIVITIES]
    scale = total_target / sum(raw)
    for aid, phase, name, start, dur, pred, disc, hours, cost in ACTIVITIES:
        acts.append({"id": aid, "phase": phase, "name": name, "start": start,
                     "duration": dur, "finish": start + dur, "predecessors": pred,
                     "discipline": disc, "hours": r(hours * scale, 0), "cost_musd": cost})
    # weekly manhour histogram, spread uniformly over each activity
    weeks = 44
    hist = [0.0] * weeks
    for a in acts:
        if a["duration"] <= 0:
            continue
        per = a["hours"] / a["duration"]
        for w in range(int(math.floor(a["start"])), int(math.ceil(a["finish"]))):
            if 0 <= w < weeks:
                overlap = min(a["finish"], w + 1) - max(a["start"], w)
                if overlap > 0:
                    hist[w] += per * overlap
    cum = []
    running = 0.0
    for h in hist:
        running += h
        cum.append(running)
    total_hours = sum(a["hours"] for a in acts)
    return {"activities": acts, "phases": list(PHASE_COLOURS), "phase_colours": PHASE_COLOURS,
            "weeks": weeks, "histogram_hours": [r(h, 0) for h in hist],
            "cumulative_hours": [r(c, 0) for c in cum],
            "peak_week_hours": r(max(hist), 0), "total_planned_hours": r(total_hours, 0),
            "shutdown": {"start": 32, "end": 36},
            "milestones": [
                {"milestone": "Design freeze / AFC", "week": 11},
                {"milestone": "Compressor PO", "week": 8},
                {"milestone": "All vendor FATs complete", "week": 31},
                {"milestone": "Shutdown start", "week": 32},
                {"milestone": "First gas", "week": 36},
                {"milestone": "Ramp-up complete", "week": 38},
                {"milestone": "Close-out", "week": 42},
            ]}


def deliverables():
    return [
        {"id": "AZ-FEED-08-Report", "title": "FEED engineering report", "rev": "B", "format": "DOCX / PDF"},
        {"id": "AZ-FEED-08-DEXPI-PFD-001", "title": "Process flow diagram (DEXPI 2.0 Process exchange + drawing)", "rev": "B", "format": "XML / SVG / PDF"},
        {"id": "AZ-FEED-08-PID-001", "title": "Simplified P&ID", "rev": "B", "format": "SVG / PDF"},
        {"id": "AZ-FEED-08-BFD-001", "title": "Block flow diagram", "rev": "B", "format": "SVG / PDF"},
        {"id": "AZ-FEED-08-SCH-001", "title": "Implementation schedule", "rev": "B", "format": "SVG / PDF"},
        {"id": "AZ-FEED-08-Workbook", "title": "Calculation and estimate workbook", "rev": "B", "format": "XLSX"},
        {"id": "AZ-FEED-08-HAZOP", "title": "Independent HAZOP worksheet and close-out", "rev": "B", "format": "XLSX / report section"},
        {"id": "AZ-FEED-08-Calc", "title": "Calculation dossier (reproducible Python model)", "rev": "B", "format": "PY / JSON"},
    ]


def main():
    mh = manhour_block()
    cb = cost_block()
    payload = {
        "meta": {"source": "model/feed_model.py", "azoth_version": BASE["meta"]["azoth_version"],
                 "note": "FEED additions; illustrative asset; screening values only."},
        "co2": co2_case(),
        "mechanical": mechanical(),
        "materials": materials(),
        "datasheets": datasheets(),
        "manhours": mh,
        "cost": cb,
        "schedule": schedule_block(),
        "deliverables": deliverables(),
        "design_basis": {
            "design_life_years": LIFE_YEARS,
            "design_pressure_hp_barg": 66.0,
            "design_temperature_hp_c": 100.0,
            "export_pressure_barg": B["p_export_bar"],
            "sales_gas_mmscfd": r(TR["gas_mmscfd"], 2),
            "condensate_bpd": r(TR["bpd"], 0),
            "produced_water_bpd": r(TR["water_bpd"], 0),
            "water_cut_pct": r(TR["water_cut"] * 100, 1),
            "cgr_bbl_per_mmscf": r(TR["cgr"], 2),
            "shutdown_weeks": 4,
            "dehydration": "Existing TEG dehydration is outside the debottlenecking battery limits and is assumed adequate; its capacity at 161.8 MMscfd and 3 lb/MMscfd is a HAZOP action.",
            "flash_gas": "Stock-tank flash gas is routed to the fuel-gas system, not the 90 bara sales header.",
            "level_protection": "1oo2 LAHH/LSHH on V-101 to ESD; LCV-102 fail-closed; PSV-102/103/104 on the let-down and water package (HAZOP actions).",
        },
        "water_package": {
            "capacity_bpd": 6000, "current_bpd": r(TR["water_bpd"], 0),
            "utilisation_pct": r(TR["water_bpd"] / 3000.0 * 100, 1),
            "coalescer_required": True,
            "interface_settling_s": r(TR["settling_s"], 1),
            "settling_criterion_s": 60.0,
            "retention_min": r(TR["water_retention_min"], 2),
        },
        "relief_summary": {
            "blocked_outlet_kg_s": r(REL["current"]["relief_kg_s"], 2),
            "hipps_kg_s": r(REL["plus_18_hipps"]["relief_kg_s"], 2),
            "flare_utilisation_hipps_pct": r(REL["plus_18_hipps"]["flare_mass_utilisation"] * 100, 1),
            "psv_required_in2": r(REL["plus_18_hipps"]["psv_required_in2"], 2),
            "psv_fitted_in2": B["psv_area_in2"],
            "radiation_distance_hipps_m": r(REL["plus_18_hipps"]["radiation_distance_m"], 1),
        },
        "economics": {
            "npv_musd": r(ECO["results"]["HIPPS + water"]["npv"], 2),
            "capex_musd": r(PKG["HIPPS + water"]["capex"], 1),
            "payback_year": ECO["results"]["HIPPS + water"]["payback"],
            "gas_pv_musd": r(ECO["results"]["HIPPS + water"]["gas_pv"], 1),
            "cond_pv_musd": r(ECO["results"]["HIPPS + water"]["cond_pv"], 1),
        },
    }
    out = ROOT / "model" / "feed_results.json"
    out.write_text(json.dumps(payload, indent=2))
    print(f"wrote {out}")
    print(f"CO2 pp {payload['co2']['p_co2_vapour_bar']} bar; CA {payload['co2']['ca_selected_mm']} mm")
    print(f"manhours {mh['totals']['total']:,} h; labour {mh['labour_cost_total_musd']} MUSD")
    print(f"cost {cb['total_musd']} MUSD reconciles={cb['reconciles']}")
    print(f"schedule {payload['schedule']['total_planned_hours']:.0f} h; peak {payload['schedule']['peak_week_hours']:.0f} h/wk")


if __name__ == "__main__":
    main()
