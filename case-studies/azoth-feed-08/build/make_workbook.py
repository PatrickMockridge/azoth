"""Build the FEED case-study workbook: AZ-FEED-08-Workbook.xlsx."""

from __future__ import annotations

import json
from pathlib import Path

from openpyxl import Workbook
from openpyxl.styles import Alignment, Border, Font, PatternFill, Side
from openpyxl.utils import get_column_letter

ROOT = Path(__file__).resolve().parents[1]
RES = json.loads((ROOT / "model" / "results.json").read_text())
OUT = ROOT / "data" / "AZ-FEED-08-Workbook.xlsx"

NAVY = "1F3864"
LIGHT = "D9E2F3"
GREY = "F2F2F2"
ORANGE = "C5501F"
GREEN = "1B7A56"
THIN = Side(style="thin", color="BFBFBF")
BORDER = Border(left=THIN, right=THIN, top=THIN, bottom=THIN)
HDR = Font(name="Calibri", size=10, bold=True, color="FFFFFF")
BODY = Font(name="Calibri", size=10)
BOLD = Font(name="Calibri", size=10, bold=True)
SMALL = Font(name="Calibri", size=9, color="595959")


def title(ws, text, sub=None):
    ws["A1"] = text
    ws["A1"].font = Font(name="Calibri", size=15, bold=True, color=NAVY)
    if sub:
        ws["A2"] = sub
        ws["A2"].font = SMALL
    ws.sheet_view.showGridLines = False


def header_row(ws, row, values, widths=None):
    for col, value in enumerate(values, start=1):
        cell = ws.cell(row=row, column=col, value=value)
        cell.font = HDR
        cell.fill = PatternFill("solid", fgColor=NAVY)
        cell.alignment = Alignment(horizontal="center", vertical="center", wrap_text=True)
        cell.border = BORDER
    ws.row_dimensions[row].height = 30
    if widths:
        for col, width in enumerate(widths, start=1):
            ws.column_dimensions[get_column_letter(col)].width = width


def body_rows(ws, start, rows, number_formats=None, bold_last=False):
    for r, row in enumerate(rows, start=start):
        for c, value in enumerate(row, start=1):
            cell = ws.cell(row=r, column=c, value=value)
            cell.font = BODY
            cell.border = BORDER
            cell.alignment = Alignment(vertical="top", wrap_text=isinstance(value, str) and len(str(value)) > 40)
            if number_formats and c - 1 < len(number_formats) and number_formats[c - 1]:
                cell.number_format = number_formats[c - 1]
    if bold_last and rows:
        for c in range(1, len(rows[-1]) + 1):
            ws.cell(row=start + len(rows) - 1, column=c).font = BOLD


def section(ws, row, text, span=6):
    cell = ws.cell(row=row, column=1, value=text)
    cell.font = Font(name="Calibri", size=11, bold=True, color=NAVY)
    cell.fill = PatternFill("solid", fgColor=LIGHT)
    for c in range(1, span + 1):
        ws.cell(row=row, column=c).fill = PatternFill("solid", fgColor=LIGHT)
        ws.cell(row=row, column=c).border = BORDER


def build():
    wb = Workbook()
    t = RES["train"]["current"]
    tp = RES["train"]["potential"]
    rel = RES["relief"]
    cap = RES["capacity"]
    econ = RES["economics"]
    split = RES["split"]["current"]

    # ---------------------------------------------------------------- cover
    ws = wb.active
    ws.title = "Cover"
    title(ws, "Offshore debottlenecking — FEED case study",
          "Platform 60/10/2, year ten · condensate uplift within the flare limit")
    ws.column_dimensions["A"].width = 30
    ws.column_dimensions["B"].width = 90
    rows = [
        ("Document number", "AZ-FEED-08-Workbook"),
        ("Revision", "A"),
        ("Date", "2026-11-26"),
        ("Status", "Issued for study — FEED case study, not for construction"),
        ("Basis", "azoth notebooks/08-offshore-debottlenecking.ipynb, reproduced in model/case_model.py"),
        ("Library", f"azoth {RES['meta']['azoth_version']} ({', '.join(RES['meta']['backends'])})"),
        ("Verification", "All library results are partially_verified: screening values, not design deliverables."),
        ("Data", "The field, history, costs and prices are illustrative. No number is a real asset."),
        ("Sheets", "Basis of Design · Capacity · Relief & Safety · Equipment List · Datasheets · Cost Estimate · Economics · Project Plan · Verification"),
    ]
    for r, (k, v) in enumerate(rows, start=4):
        ws.cell(row=r, column=1, value=k).font = BOLD
        ws.cell(row=r, column=2, value=v).font = BODY
        ws.cell(row=r, column=2).alignment = Alignment(wrap_text=True, vertical="top")

    ws["A15"] = "Headline result"
    ws["A15"].font = Font(name="Calibri", size=12, bold=True, color=NAVY)
    facts = [
        ("Base sales gas", f"{t['gas_mmscfd']:.1f} MMscfd"),
        ("Base condensate", f"{t['bpd']:.0f} bbl/d"),
        ("Base produced water", f"{t['water_bpd']:.0f} bbl/d"),
        ("Recommended package", "HIPPS + produced-water upgrade (compressor re-rate, separator internals)"),
        ("Installed cost", "36 MUSD (42 MUSD with the flare/header fallback)"),
        ("NPV at 10%", f"{econ['results']['HIPPS + water']['npv']:.1f} MUSD"),
        ("Payback", f"year {econ['results']['HIPPS + water']['payback']}"),
        ("Capacity after package", f"{cap['package_cap']:.1f} MMscfd ({t['gas_mmscfd']:.1f} → {cap['package_cap']:.1f})"),
        ("Shutdown", "4 weeks, inside the window"),
    ]
    for r, (k, v) in enumerate(facts, start=16):
        ws.cell(row=r, column=1, value=k).font = BOLD
        ws.cell(row=r, column=2, value=v).font = BODY

    # ---------------------------------------------------------------- basis
    ws = wb.create_sheet("Basis of Design")
    title(ws, "Basis of design", "Year-ten operating point and study envelope")
    header_row(ws, 4, ["Parameter", "Value", "Unit", "Basis / source"], [42, 18, 18, 70])
    s = RES["basis"]
    rows = [
        ("Hydrocarbon feed", s["hc_current_mol_s"], "mol/s", "Year-ten history (study input)"),
        ("Produced water", s["water_mol_s"], "mol/s", "Year-ten history (13.45 mol %)"),
        ("Well potential", 2300.0, "mol/s", "Well deliverability at year ten"),
        ("HP separator pressure", s["p_hp_bar"], "bara", "As-built"),
        ("HP separator temperature", s["t_hp_k"], "K", "As-built"),
        ("MP separator pressure", s["p_mp_bar"], "bara", "As-built"),
        ("LP separator pressure", s["p_lp_bar"], "bara", "As-built"),
        ("Export pressure", s["p_export_bar"], "bara", "Pipeline reception"),
        ("Cooler outlet temperature", s["t_cool_k"], "K", "As-built"),
        ("HP separator diameter", s["d_hp_m"], "m", "As-built data sheet"),
        ("HP separator tangent length", s["l_hp_m"], "m", "As-built data sheet"),
        ("Souders-Brown K, as found", s["k_design"], "m/s", "As-built data sheet"),
        ("Souders-Brown K, with new internals", s["k_upgrade"], "m/s", "Vendor expectation (study assumption)"),
        ("Compressor driver rating", s["driver_kw"], "kW", "As-built"),
        ("PSV set pressure", s["psv_set_bar"], "barg", "As-built"),
        ("PSV orifice fitted", s["psv_area_in2"], "in²", "P orifice, as-built"),
        ("Relieving pressure", s["relief_pressure_bar"], "bara", "Blocked-outlet case"),
        ("Produced-water capacity", s["water_capacity_bpd"], "bbl/d", "As-built"),
        ("Produced-water capacity after upgrade", s["water_upgrade_bpd"], "bbl/d", "Study assumption"),
    ]
    body_rows(ws, 5, rows, [None, "#,##0.00", None, None])
    for r in range(5, 5 + len(rows)):
        ws.cell(row=r, column=2).alignment = Alignment(horizontal="right")

    # ---------------------------------------------------------------- capacity
    ws = wb.create_sheet("Capacity")
    title(ws, "Capacity chain and capability", "Utilisation at the year-ten point, and the gas rate at which each constraint binds")
    header_row(ws, 4, ["Constraint", "Utilisation %", "Cap now, MMscfd", "Cap after scope, MMscfd", "Fix"], [30, 16, 20, 22, 32])
    fixes = {
        "separator gas load": "Separator internals (K 0.11 → 0.14)",
        "compressor power": "Compressor re-rate (+18%)",
        "produced water": "Produced-water upgrade (3,000 → 6,000 bbl/d)",
        "flare": "HIPPS (−35% case) or flare/header (+25%)",
        "gas line": "None — ample",
        "PSV orifice": "None — P orifice passes",
        "pump power": "None — ample",
        "liquid line": "None — ample",
    }
    cap_keys = {
        "separator gas load": "HP separator",
        "compressor power": "Compressor",
        "produced water": "Water plant",
        "flare": "Flare",
        "gas line": "Gas line",
        "PSV orifice": "PSV",
        "pump power": None,
        "liquid line": None,
    }
    rows = []
    for name, util in sorted(cap["utilisation"].items(), key=lambda kv: -kv[1]):
        key = cap_keys.get(name)
        rows.append([
            name,
            util * 100.0,
            cap["caps"][key] if key else None,
            cap["fixed_caps"][key] if key else None,
            fixes[name],
        ])
    body_rows(ws, 5, rows, [None, "0.0", "0.0", "0.0", None])
    for r in range(5, 5 + len(rows)):
        ws.cell(row=r, column=2).alignment = Alignment(horizontal="right")
    section(ws, 15, "Package caps")
    header_row(ws, 16, ["Package", "Gas cap, MMscfd", "Limiting constraint", "Fits four-week shutdown?"], [34, 18, 26, 26])
    pack_rows = [
        ("Gas side (1+2+3)", cap["gas_side_cap"], "Water plant (147.8)", "Yes"),
        ("Gas side + water (1+2+3+5)", cap["package_cap"], "Compressor re-rate (161.8)", "Yes"),
        ("HIPPS + water (1+2+4+5)", cap["hipps_cap"], "Compressor re-rate (161.8)", "Yes"),
    ]
    body_rows(ws, 17, pack_rows, [None, "0.0", None, None])
    ws.cell(row=21, column=1, value=f"Well potential: {tp['gas_mmscfd']:.1f} MMscfd. The package leaves ~{tp['gas_mmscfd'] - cap['package_cap']:.1f} MMscfd of well capacity unproduced.").font = SMALL

    # ---------------------------------------------------------------- relief
    ws = wb.create_sheet("Relief & Safety")
    title(ws, "Relief case and safety case", "Blocked-outlet relief, PSV, flare and the HIPPS safety case")
    header_row(ws, 4, ["Case", "Relief, kg/s", "PSV required, in²", "PSV utilisation, %", "Flare utilisation, %", "Flare overloaded", "Radiation distance, m (4.73 kW/m²)"],
               [24, 14, 18, 18, 18, 16, 26])
    cases = [
        ("Current", rel["current"]),
        ("+18% gas", rel["plus_18"]),
        ("+18%, flare +25%", rel["plus_18_flare25"]),
        ("+18%, HIPPS −35%", rel["plus_18_hipps"]),
    ]
    rows = []
    for name, c in cases:
        rows.append([
            name,
            c["relief_kg_s"],
            c["psv_required_in2"],
            c.get("psv_utilisation", rel["plus_18"]["psv_utilisation"]) * 100.0,
            c["flare_mass_utilisation"] * 100.0,
            "Yes" if c["flare_overloaded"] else "No",
            c["radiation_distance_m"],
        ])
    body_rows(ws, 5, rows, [None, "0.00", "0.00", "0.0", "0.0", None, "0.0"])
    ws.cell(row=10, column=1, value=f"Fire case: {rel['fire_kg_s']:.3f} kg/s. Simultaneous network: {rel['network_simultaneous_kg_s']:.2f} kg/s.").font = SMALL
    section(ws, 12, "Safety case checks (azoth process library)")
    header_row(ws, 13, ["Check", "Result", "Verdict"], [40, 24, 30])
    sc = rel["safety_case"]
    rows = [
        ("NOG-070 minimum SIL (HIPPS pipeline)", sc["nog070_minimum_sil"], "Catalogue"),
        ("NOG-070 achieved SIL", sc["nog070_achieved_sil"], "Compliant" if sc["nog070_compliant"] else "Not compliant"),
        ("SIL verification, 1oo2, PFD 1e-4", sc["sil_achieved"], "Passed" if sc["sil_passed"] else "Failed"),
        ("Pipeline maximum incidental pressure, bara", sc["pipeline_max_incidental_bar"], sc["pipeline_verdict"]),
        ("NORSOK S-001 secondary protection", "acceptable" if sc["s001_acceptable"] else "not acceptable", "Two barriers required and provided"),
    ]
    body_rows(ws, 14, rows, [None, "0.00", None])
    ws.cell(row=20, column=1, value="HFT 1.0, diagnostic coverage 0.6. The 35% relief reduction is a study assumption; the SIL-3 case is outside the shutdown.").font = SMALL

    # ---------------------------------------------------------------- equipment
    ws = wb.create_sheet("Equipment List")
    title(ws, "Equipment list", "Existing, modified and new equipment in the recommended and fallback scope")
    header_row(ws, 4, ["Tag", "Service", "Type", "Design P, barg", "Design T, K", "Key size / duty", "Scope", "Cost ref", "Shutdown wk"],
               [12, 30, 20, 14, 12, 30, 26, 10, 12])
    rows = [
        ("V-101", "HP 3-phase separator", "Horizontal vessel", 66, 354.15, "Ø1.96 × 7.26 m T/T; K 0.11 → 0.14 m/s", "Modify internals + coalescer duty in water package", "Fix 2", 3),
        ("K-101", "Export compressor", "Centrifugal, 3 trains (1+3+5 stages)", 100, 313.15, "2.26 MW now; 2.69 MW at potential; 90 bara; driver 2.7 MW", "Re-rate (+18% capacity)", "Fix 1", 3),
        ("PSV-101", "HP separator relief", "PSV, P orifice", 66, 354.15, "6.38 in² fitted; 4.69 in² required", "No change", "—", 0),
        ("SDV-101", "HP inlet isolation", "SIL-3 shutdown valve", 100, 363.15, "Full bore, 1oo2 initiators", "New HIPPS loop", "Fix 4", 2),
        ("V-105", "Produced-water hydrocyclone", "Package vessel", 25, 333.15, "6,000 bbl/d", "New package", "Fix 5", 4),
        ("V-106", "Produced-water coalescer", "Package vessel", 25, 333.15, "6,000 bbl/d; 6.5 min; 4.0 m T/T", "New package", "Fix 5", 4),
        ("P-102", "Treated-water pump (2 x 50%)", "Centrifugal", 25, 333.15, "2 x 20 m³/h; 15 kW; one standby", "New package", "Fix 5", 4),
        ("PSV-102", "MP let-down relief", "PSV", 12, 373.15, "VLV-102 fail-open blowby", "New (HAZOP)", "—", 4),
        ("PSV-103", "LP let-down relief", "PSV", 4, 373.15, "VLV-103 fail-open blowby", "New (HAZOP)", "—", 4),
        ("PSV-104", "Water package relief", "PSV", 25, 353.15, "LCV-102 fail-open", "New (HAZOP)", "—", 4),
        ("P-101", "Condensate pump", "Centrifugal", 30, 313.15, "20.4 kW; 42 kW driver", "No change", "—", 0),
        ("Flare", "Flare tip and header", "Existing flare system", 10, 354.15, "46.3 kg/s design; 91% loaded", "Fallback: +25% capacity", "Fix 3", 4),
        ("V-201", "Fourth-stage separator", "Horizontal vessel", 60, 354.15, "150 bbl/d condensate", "Deferred — not recommended", "Fix 6", 5),
        ("V-202", "Stabiliser", "Column + reboiler", 10, 400.0, "200 bbl/d condensate", "Rejected — extends shutdown", "Fix 7", 8),
    ]
    body_rows(ws, 5, rows)
    ws.cell(row=5 + len(rows) + 1, column=1, value="Tags V-201/V-202 are option tags, not part of the recommended scope. Design pressures are study values from the as-built basis.").font = SMALL

    # ---------------------------------------------------------------- datasheets
    ws = wb.create_sheet("Datasheets")
    title(ws, "Equipment datasheets (study level)", "Values from the azoth calculation basis; full screening results in the Verification sheet")
    ws.column_dimensions["A"].width = 34
    ws.column_dimensions["B"].width = 24
    ws.column_dimensions["C"].width = 16
    ws.column_dimensions["D"].width = 80
    row = 4
    sheets = {
        "V-101 · HP 3-phase separator": [
            ("Design pressure", 66.0, "barg", "As-built; PSV set 66 barg"),
            ("Operating pressure / temperature", "60 / 354.15", "bara / K", "Year-ten point"),
            ("Internal diameter", 1.96, "m", "As-built"),
            ("Tangent length", 7.26, "m", "As-built"),
            ("Feed", 2230.0, "mol/s", "1,930 HC + 300 water"),
            ("Vapour outlet", 1869.19, "mol/s", "azoth three_phase_separator"),
            ("Oil outlet", 76.62, "mol/s", "azoth three_phase_separator"),
            ("Water outlet", 284.19, "mol/s", "azoth three_phase_separator"),
            ("Gas load", 99.77, "%", "azoth separator_capacity_assessment"),
            ("Souders-Brown K, as found / after", "0.11 / 0.14", "m/s", "After is the vendor expectation (assumption)"),
            ("Oil retention", 10.92, "min", "Against 9.0 min minimum"),
            ("Water retention", 7.83, "min", "Against 6.5 min minimum"),
            ("Interface settling", 17.09, "s", "Insufficient — coalescer required"),
            ("Design temperature", 354.15, "K", "Operating"),
        ],
        "K-101 · Export compressor": [
            ("Shaft power, now / at potential", "2256 / 2689", "kW", "azoth compressor, 3 stages per train"),
            ("Driver rating", 2300.0, "kW", "98.1% now; 116.9% at potential"),
            ("Suction pressures", "60 / 10 / 2", "bara", "HP / MP / LP stages"),
            ("Discharge pressure", 90.0, "bara", "Pipeline"),
            ("Discharge temperature", 313.15, "K", "After cooler"),
            ("Compression ratio per stage", 2.5, "max", "Equal-ratio staging"),
            ("Isentropic efficiency", 0.78, "—", "Study assumption"),
            ("Re-rate", "+18%", "%", "Fix 1; cap 139.7 → 161.8 MMscfd"),
            ("Cooler duty, now / at potential", "3885 / 4630", "kW", "azoth cooler"),
        ],
        "HIPPS · SIL-3 loop": [
            ("Function", "High-pressure protection of HP inlet", "", "2 transmitters, 1oo2 voting"),
            ("SIL", 3.0, "—", "NOG-070 minimum and achieved SIL 3"),
            ("PFD average", 1e-4, "—", "azoth sil_verification, 1oo2, proof test 8,760 h"),
            ("Hardware fault tolerance / DC", "1.0 / 0.6", "—", "azoth sil_verification"),
            ("Proof-test interval", 12, "months", "NORSOK S-001 criterion"),
            ("Relief reduction credited", 35.0, "%", "Study assumption, not a library output"),
            ("Blocked-outlet case after HIPPS", 32.70, "kg/s", "azoth blocked_outlet_relief"),
            ("Pipeline maximum incidental", 72.6, "bara", "Two barriers required; adequate"),
            ("S-001 secondary protection", "acceptable", "", "azoth s001_secondary_pressure_protection"),
        ],
        "Produced-water package": [
            ("Capacity, now / after", "3,000 / 6,000", "bbl/d", "As-built / study assumption"),
            ("Current flow", 2782.2, "bbl/d", "92.7% of 3,000"),
            ("Water cut", 48.66, "%", "Year ten"),
            ("Coalescer", "Required", "", "Interface settling 17.09 s < 60 s class check"),
            ("Water retention", 7.83, "min", "Above 6.5 min"),
            ("Equipment", "Hydrocyclone + coalescer + pump", "", "Package scope"),
            ("Capacity cap after upgrade", 295.6, "MMscfd", "Water plant no longer binds"),
        ],
        "Flare system (fallback)": [
            ("Design mass capacity", 46.34, "kg/s", "From 92% of HP vapour at design"),
            ("Design molar capacity", 2031.7, "mol/s", "As above"),
            ("Current load", 42.63, "kg/s", "91.1% utilisation"),
            ("+18% load", 50.31, "kg/s", "107.5% — overloaded"),
            ("Upgrade", "+25%", "%", "Fix 3; 86.0% utilisation"),
            ("Radiation distance, current / +18%", "74.96 / 82.42", "m", "4.73 kW/m² threshold"),
            ("Radiation distance with HIPPS", 64.03, "m", "At +18% with −35% case"),
        ],
    }
    for name, rows in sheets.items():
        section(ws, row, name)
        row += 1
        header_row(ws, row, ["Parameter", "Value", "Unit", "Basis / note"])
        row += 1
        for item in rows:
            for c, value in enumerate(item, start=1):
                cell = ws.cell(row=row, column=c, value=value)
                cell.font = BODY
                cell.border = BORDER
                cell.alignment = Alignment(vertical="top", wrap_text=c == 4)
            row += 1
        row += 1

    # ---------------------------------------------------------------- cost
    ws = wb.create_sheet("Cost Estimate")
    title(ws, "Cost estimate — AACE Class 4/5 study allowances",
          "Brownfield allowances in USD million, year-ten money; no contingency, escalation or owner's costs")
    header_row(ws, 4, ["ID", "Fix", "Effect", "Capex, MUSD", "Weeks", "Used in"], [6, 28, 36, 14, 10, 34])
    used = {
        1: "Gas side, Gas + water, HIPPS + water",
        2: "Gas side, Gas + water, HIPPS + water",
        3: "Gas side, Gas + water (fallback)",
        4: "HIPPS + water (recommended)",
        5: "Gas + water, HIPPS + water",
        6: "Fourth stage option (deferred)",
        7: "Stabiliser option (rejected)",
    }
    rows = [[f["id"], f["fix"], f["effect"], f["capex"], f["weeks"], used[f["id"]]] for f in RES["fixes"]]
    body_rows(ws, 5, rows, [None, None, None, "0.0", "0.0", None])
    section(ws, 13, "Packages")
    header_row(ws, 14, ["Package", "Fixes", "Capex, MUSD", "Shutdown weeks", "Gas cap, MMscfd", "Condensate add, bbl/d", "Deferment, MUSD", "Fits 4 weeks"],
               [26, 16, 14, 14, 16, 18, 14, 12])
    rows = []
    for name, p in RES["packages"].items():
        rows.append([name, p["fixes"], p["capex"], p["weeks"], p["cap"], p["cond_add"], p["deferment"], "Yes" if p["weeks"] <= 4 else "No"])
    body_rows(ws, 15, rows, [None, None, "0.0", "0.0", "0.0", "0.0", "0.0", None])
    section(ws, 22, "Recommended package build-up (HIPPS + water)")
    header_row(ws, 23, ["Item", "MUSD"], [60, 14])
    rows = [
        ("Compressor re-rate (Fix 1)", 18.0),
        ("HP separator internals (Fix 2)", 4.0),
        ("HIPPS SIL-3 (Fix 4)", 6.0),
        ("Produced-water upgrade + coalescer (Fix 5)", 8.0),
        ("Total installed cost", 36.0),
    ]
    body_rows(ws, 24, rows, [None, "0.0"], bold_last=True)
    ws.cell(row=30, column=1, value="Fallback (flare/header instead of HIPPS): 42.0 MUSD. Add-ons: fourth stage +31.3 MUSD (to 67.3), stabiliser +41.0 MUSD (to 77.0).").font = SMALL
    section(ws, 32, "Estimate basis and exclusions")
    notes = [
        "AACE Class 4/5 (18R-97): expected accuracy is wide (roughly −30% / +50% for Class 4); no contingency is applied.",
        "Brownfield allowances cover engineering, procurement, fabrication, offshore installation labour and commissioning.",
        "Excluded: detailed design, vendor taxes and duties, freight and marine spread, owner's costs, spare parts, escalation, decommissioning.",
        "The library's greenfield module stack prices an HP separator and a new 2.27 MW compressor at 61.7 MUSD (two modules alone) — not a re-rate, an internals change, a flare tip and a water package.",
        "The 3.3 MUSD labour premium for compressing the fourth stage into the window is not in the 67.3 MUSD package capex.",
        "The stabiliser's 10.97 MUSD deferment is production deferred by the two-week shutdown extension, not capex.",
    ]
    for r, n in enumerate(notes, start=33):
        ws.cell(row=r, column=1, value=f"• {n}").font = BODY
        ws.merge_cells(start_row=r, start_column=1, end_row=r, end_column=6)
        ws.cell(row=r, column=1).alignment = Alignment(wrap_text=True, vertical="top")
        ws.row_dimensions[r].height = 28

    # ---------------------------------------------------------------- economics
    ws = wb.create_sheet("Economics")
    title(ws, "Incremental ten-year DCF", "Recommended case: HIPPS + produced-water upgrade, 10% discount, 25% tax")
    header_row(ws, 4, ["Parameter", "Value", "Unit"], [40, 16, 18])
    rows = [
        ("Gas price", 4.0, "USD/MMBtu"),
        ("Condensate price", 70.0, "USD/bbl"),
        ("Heating value", 1.05e3, "MMBtu/MMscf"),
        ("Discount rate", 10.0, "%"),
        ("Tax rate", 25.0, "%"),
        ("Depreciation", "10-year straight line", ""),
        ("Decline", 5.0, "%/yr"),
        ("Water cut, start / end", "48.66 / 57.0", "%"),
        ("Fixed opex", 2.5, "MUSD/yr"),
        ("Electricity", 0.33, "MUSD/yr"),
        ("Incremental water disposal", 2.0, "USD/bbl"),
        ("Evaluation period", 10, "years"),
    ]
    body_rows(ws, 5, rows)
    section(ws, 18, "Package economics")
    header_row(ws, 19, ["Package", "Capex, MUSD", "NPV, MUSD", "Payback year", "Gas PV, MUSD", "Condensate PV, MUSD"], [26, 14, 14, 14, 16, 18])
    rows = []
    for name, r in econ["results"].items():
        rows.append([name, r["capex"], r["npv"], r["payback"], r["gas_pv"], r["cond_pv"]])
    body_rows(ws, 20, rows, [None, "0.0", "0.00", "0", "0.0", "0.0"])
    section(ws, 27, "Recommended case cash flow (incremental)")
    header_row(ws, 28, ["Year", "Incremental gas, MMscfd", "Incremental condensate, bbl/d", "Revenue gas, MUSD", "Revenue condensate, MUSD", "Opex + water, MUSD", "Cash after tax, MUSD", "Cumulative discounted, MUSD"],
               [7, 14, 18, 16, 20, 18, 18, 16])
    # recommended case cash flow, straight from the model
    cf = econ["results"]["HIPPS + water"]
    rows = []
    for year in range(10):
        rows.append([
            year + 1,
            cf["inc_gas"][year],
            cf["inc_cond"][year],
            cf["rev_gas"][year],
            cf["rev_cond"][year],
            cf["opex"][year],
            cf["cash"][year],
            cf["cum_disc"][year],
        ])
    body_rows(ws, 29, rows, ["0", "0.0", "0", "0.00", "0.00", "0.00", "0.00", "0.00"])
    section(ws, 41, "Sensitivity (NPV change from the gas+water base case)")
    header_row(ws, 42, ["Case", "NPV, MUSD", "Change, MUSD"], [40, 16, 16])
    rows = [[r["case"], r["npv"], r["change"]] for r in econ["tornado"]]
    body_rows(ws, 43, rows, [None, "0.00", "+0.00;-0.00"])

    # ---------------------------------------------------------------- plan
    ws = wb.create_sheet("Project Plan")
    title(ws, "Implementation project plan", "42-week plan to the four-week shutdown window; durations are planning assumptions except the shutdown tasks")
    header_row(ws, 4, ["ID", "Phase", "Task", "Start wk", "Duration wk", "Finish wk", "Predecessor", "Owner", "Shutdown"],
               [6, 22, 46, 10, 12, 10, 12, 16, 10])
    tasks = [
        ("A1", "A · Design", "FEED close-out and value engineering", 0, 4, "—", "Project", ""),
        ("A2", "A · Design", "Detailed design, HAZOP and SIL review", 2, 8, "A1", "Engineering", ""),
        ("A3", "A · Procurement", "Long-lead enquiry and order placement", 4, 4, "A2", "Procurement", ""),
        ("B1", "B · Procurement", "Compressor re-rate package (long lead)", 6, 22, "A3", "Vendor", ""),
        ("B2", "B · Procurement", "HIPPS hardware and logic solver", 8, 12, "A3", "Vendor", ""),
        ("B3", "B · Procurement", "Separator internals", 10, 14, "A2", "Vendor", ""),
        ("B4", "B · Procurement", "Produced-water package", 10, 16, "A2", "Vendor", ""),
        ("C1", "C · Readiness", "Offshore survey and work packs", 26, 4, "A2", "Construction", ""),
        ("C2", "C · Readiness", "Factory acceptance tests", 28, 4, "B1,B2,B3,B4", "QA/QC", ""),
        ("C3", "C · Readiness", "Logistics, marine spread and permits", 30, 2, "C1", "Logistics", ""),
        ("D1", "D · Shutdown", "Produced-water package installation", 32, 4, "C2,C3", "Construction", "Yes"),
        ("D2", "D · Shutdown", "Compressor re-rate", 32, 3, "C2,C3", "Construction", "Yes"),
        ("D3", "D · Shutdown", "Separator internals", 32.3, 3, "C2,C3", "Construction", "Yes"),
        ("D4", "D · Shutdown", "HIPPS installation and loop test", 33, 2, "C2,C3", "Construction", "Yes"),
        ("D5", "D · Shutdown", "Flare/header fallback (if selected)", 32, 4, "C2,C3", "Construction", "Yes"),
        ("D6", "D · Shutdown", "Commissioning and handover", 35.2, 0.8, "D1,D2,D3,D4", "Commissioning", "Yes"),
        ("E1", "E · Ramp-up", "Commissioning, performance test and ramp-up", 36, 4, "D6", "Operations", ""),
        ("F1", "F · Close-out", "Close-out and benefits realisation", 40, 2, "E1", "Project", ""),
    ]
    rows = []
    for tid, phase, task, start, dur, pred, owner, shutdown in tasks:
        rows.append([tid, phase, task, start, dur, None, pred, owner, shutdown])
    body_rows(ws, 5, rows, [None, None, None, "0.0", "0.0", "0.0", None, None, None])
    for r in range(5, 5 + len(rows)):
        ws.cell(row=r, column=6, value=f"=D{r}+E{r}")
        ws.cell(row=r, column=6).number_format = "0.0"
        ws.cell(row=r, column=6).font = BODY
        ws.cell(row=r, column=6).border = BORDER
    section(ws, 24, "Milestones")
    header_row(ws, 25, ["Milestone", "Week", "Basis"], [44, 10, 44])
    rows = [
        ("FEED close-out / DG2", 4, "Planning assumption"),
        ("Compressor purchase order", 8, "Long-lead critical path"),
        ("Factory acceptance tests complete", 32, "Planning assumption"),
        ("Shutdown start", 32, "Four-week window"),
        ("First gas after shutdown", 36, "End of window"),
        ("Ramp-up complete", 40, "Planning assumption"),
        ("Close-out", 42, "Planning assumption"),
    ]
    body_rows(ws, 26, rows)

    # ---------------------------------------------------------------- verification
    ws = wb.create_sheet("Verification")
    title(ws, "Calculation verification register", "azoth calculations behind the study; every result is partially_verified — screening only")
    header_row(ws, 4, ["azoth id", "Scenario", "Key inputs", "Key result", "Verification", "Provenance"],
               [38, 26, 40, 34, 22, 22])
    rows = [
        ("process.three_phase_separator", "HP separator, year ten", "2,230 mol/s; 60 bara; 354.15 K; Ø1.96×7.26 m",
         "Vapour 1,869.19; oil 76.62; water 284.19 mol/s; settling 17.09 s", "partially_verified", "spec d0fc37427277 · rust 1401191864f9"),
        ("process.separator_capacity_assessment", "HP separator gas load", "1,869.19 mol/s; K 0.11 m/s; D 1.96 m; level 0.2",
         "Utilisation 99.77%; maximum gas flow 0.809 m³/s", "partially_verified", "spec 444d257c5074 · rust 5163245200d8"),
        ("process.separator_capacity_assessment", "HP separator at well potential", "2,227.53 mol/s; same basis",
         "Utilisation 118.9%; status infeasible", "partially_verified", "spec 444d257c5074 · rust 5163245200d8"),
        ("process.flare", "Current relief case", "1,869.19 mol/s; 6 MPa; design 46.34 kg/s",
         "Mass utilisation 91.10%; radiation distance 74.96 m", "partially_verified", "spec 0282ff3baf7e · rust e49499aef85e"),
        ("process.flare", "+18% gas", "2,205.64 mol/s; same design basis",
         "Mass utilisation 107.50%; overloaded", "partially_verified", "spec 0282ff3baf7e · rust e49499aef85e"),
        ("process.flare", "+18% with flare +25%", "2,205.64 mol/s; design 57.93 kg/s",
         "Mass utilisation 86.00%", "partially_verified", "spec 0282ff3baf7e · rust e49499aef85e"),
        ("process.flare", "+18% with HIPPS −35%", "1,433.67 mol/s; design 46.34 kg/s",
         "Mass utilisation 69.87%", "partially_verified", "spec 0282ff3baf7e · rust e49499aef85e"),
        ("process.psv_sizing", "Blocked outlet, current", "42.634 kg/s; set 66 barg; T 354.15 K",
         "Required 4.688 in²; P orifice; capacity 57.97 kg/s", "partially_verified", "spec 70df5cd128ce · rust b2414cf1d7ca"),
        ("process.psv_sizing", "Blocked outlet, +18%", "50.308 kg/s; same basis",
         "Required 5.531 in² (86.7%); P orifice", "partially_verified", "spec 70df5cd128ce · rust b2414cf1d7ca"),
        ("process.nog070_sil_determination", "HIPPS pipeline", "PFD 1e-4; sif_type hipps_pipeline",
         "Minimum SIL 3; achieved SIL 3; compliant", "partially_verified", "spec 2a87677e7963 · rust f024f6526109"),
        ("process.s001_secondary_pressure_protection", "Pipeline secondary protection", "72.6 / 66 / 99 bara; 1e-4/yr; 12 months",
         "Acceptable: all criteria met", "partially_verified", "spec 44cbe4e54cdc · rust cf016d03c700"),
        ("process.equipment_cost_estimate", "Library greenfield benchmark", "HP separator Ø1.96×7.26 m; compressor 2,273 kW",
         "16.61 MUSD for the two modules; 61.7 MUSD topsides", "partially_verified", "spec 0fa76a97e7b2 · rust 83a022df39da"),
    ]
    body_rows(ws, 5, rows)
    ws.cell(row=18, column=1, value="Status wording is the library's own. Screening values are evidence for a finding, not a design deliverable; no value here is site-verified.").font = SMALL

    for sheet in wb.worksheets:
        sheet.page_setup.orientation = "landscape"
        sheet.sheet_properties.pageSetUpPr.fitToPage = True
        sheet.page_setup.fitToWidth = 1
        sheet.page_setup.fitToHeight = 0

    wb.save(OUT)
    print("wrote", OUT)


if __name__ == "__main__":
    build()
