"""Add the FEED Rev B sheets to the workbook, after the base build."""

from __future__ import annotations

import json
from pathlib import Path

from openpyxl import load_workbook
from openpyxl.styles import Alignment, Font

import make_workbook as mw

ROOT = Path(__file__).resolve().parents[1]
F = json.loads((ROOT / "model" / "feed_results.json").read_text())
H = (ROOT / "hazop" / "worksheet.json")
HAZ = json.loads(H.read_text()) if H.exists() else None
CO = (ROOT / "hazop" / "closeout.json")
CLOSE = json.loads(CO.read_text()) if CO.exists() else None
CR = (ROOT / "model" / "closeout_results.json")
CLOSERES = json.loads(CR.read_text()) if CR.exists() else None


def sheet(wb, name, heading, sub):
    ws = wb.create_sheet(name)
    mw.title(ws, heading, sub)
    return ws


def build():
    mw.build()
    wb = load_workbook(mw.OUT)
    wb["Cover"]["B5"] = "B"
    wb["Cover"]["B10"] = ("Sheets: Basis of Design · Capacity · Relief & Safety · Equipment List · Datasheets · "
                          "Materials · Manhours · Resource Plan · Cost Estimate · Economics · DEXPI · HAZOP · Verification")

    # ------------------------------------------------------------- materials
    ws = sheet(wb, "Materials", "Materials and corrosion protection",
               "CO2 service, corrosion allowance, material schedule and line classes")
    ws.column_dimensions["A"].width = 34
    ws.column_dimensions["B"].width = 24
    ws.column_dimensions["C"].width = 20
    ws.column_dimensions["D"].width = 22
    ws.column_dimensions["E"].width = 10
    ws.column_dimensions["F"].width = 44
    c = F["co2"]
    row = 4
    mw.section(ws, row, "Corrosion basis", 6); row += 1
    mw.header_row(ws, row, ["Parameter", "Value", "Unit", "Basis", "", ""]); row += 1
    crows = [
        ("CO2 in HP vapour", c["co2_mol_pct_vapour"], "mol %", "azoth three-phase separator", "", ""),
        ("CO2 partial pressure, HP vapour", c["p_co2_vapour_bar"], "bar", "z_CO2 x 60 bara", "", ""),
        ("HP temperature", c["temperature_c"], "°C", "354.15 K", "", ""),
        ("Inhibited corrosion rate", c["inhibited_rate_mm_yr"], "mm/yr", "screening band, inhibition assumed continuous", "", ""),
        ("Uninhibited corrosion rate", c["uninhibited_rate_mm_yr"], "mm/yr", "screening band; shows inhibition is essential", "", ""),
        ("Design life", c["design_life_years"], "years", "study basis", "", ""),
        ("Corrosion allowance required", c["ca_required_mm"], "mm", "rate x life", "", ""),
        ("Corrosion allowance selected", c["ca_selected_mm"], "mm", "SA-516 Gr 70N, all wet hydrocarbon service", "", ""),
        ("Uninhibited CA equivalent", c["uninhibited_ca_mm"], "mm", "not acceptable; inhibition is a barrier", "", ""),
    ]
    mw.body_rows(ws, row, crows); row += len(crows) + 1
    ws.cell(row=row, column=1, value=c["verdict"]).font = Font(name="Calibri", size=10, italic=True)
    ws.merge_cells(start_row=row, start_column=1, end_row=row, end_column=6)
    ws.cell(row=row, column=1).alignment = Alignment(wrap_text=True, vertical="top")
    ws.row_dimensions[row].height = 42
    row += 2

    mw.section(ws, row, "Pressure-containing thickness checks", 6); row += 1
    mw.header_row(ws, row, ["Item", "t required, mm", "CA, mm", "t incl. CA, mm", "Selected, mm", "Basis"]); row += 1
    m = F["mechanical"]
    trows = [
        ("V-101 shell (ASME VIII-1 UG-27)", m["v101"]["t_required_mm"], m["v101"]["ca_mm"], m["v101"]["t_with_ca_mm"], m["v101"]["t_selected_mm"], "P 6.6 MPa, R 980 mm, S 138 MPa, E 1.0"),
        ("Export gas line 12 in (B31.3)", m["gas_line"]["t_required_mm"], m["gas_line"]["ca_mm"], m["gas_line"]["t_with_ca_mm"], m["gas_line"]["t_selected_mm"], "P 10 MPa, OD 323.9 mm, X65"),
        ("Condensate line 6 in (B31.3)", m["condensate_line"]["t_required_mm"], m["condensate_line"]["ca_mm"], m["condensate_line"]["t_with_ca_mm"], m["condensate_line"]["t_selected_mm"], "P 3 MPa, OD 168.3 mm, X65"),
        ("Produced-water line 8 in (B31.3)", m["water_line"]["t_required_mm"], m["water_line"]["ca_mm"], m["water_line"]["t_with_ca_mm"], m["water_line"]["t_selected_mm"], "P 2.5 MPa, OD 219.1 mm, 316L"),
        ("HIPPS spool 10 in (B31.3)", m["hipps_spool"]["t_required_mm"], m["hipps_spool"]["ca_mm"], m["hipps_spool"]["t_with_ca_mm"], m["hipps_spool"]["t_selected_mm"], "P 10 MPa, OD 273 mm, 316L"),
    ]
    mw.body_rows(ws, row, trows); row += len(trows) + 2

    mw.section(ws, row, "Material schedule", 6); row += 1
    mw.header_row(ws, row, ["Item", "Material", "Specification", "Corrosion allowance, mm", "", "Notes"]); row += 1
    for item in F["materials"]["schedule"]:
        vals = (item["item"], item["material"], item["spec"], item["ca_mm"], "", item["notes"])
        for ci, v in enumerate(vals, start=1):
            cell = ws.cell(row=row, column=ci, value=v); cell.font = mw.BODY; cell.border = mw.BORDER
            cell.alignment = Alignment(wrap_text=True, vertical="top")
        row += 1
    row += 1
    mw.section(ws, row, "Line classes", 6); row += 1
    mw.header_row(ws, row, ["Class", "Service", "Size, in", "Material", "Rating", "CA / insulation"]); row += 1
    for item in F["materials"]["line_classes"]:
        vals = (item["class"], item["service"], item["size_in"], item["material"], item["rating"], f'{item["ca_mm"]} mm / {item["insulation"]}')
        for ci, v in enumerate(vals, start=1):
            cell = ws.cell(row=row, column=ci, value=v); cell.font = mw.BODY; cell.border = mw.BORDER
        row += 1

    # ------------------------------------------------------------- manhours
    ws = sheet(wb, "Manhours", "Manhour estimate",
               "Discipline build-up, rates and reconciliation to the cost estimate")
    ws.column_dimensions["A"].width = 34
    for col in "BCDE":
        ws.column_dimensions[col].width = 16
    ws.column_dimensions["F"].width = 14
    row = 4
    mw.section(ws, row, "Discipline manhours", 6); row += 1
    mw.header_row(ws, row, ["Discipline", "Engineering", "Fabrication", "Offshore", "Total", "Share"]); row += 1
    mh = F["manhours"]
    total = mh["totals"]["total"]
    for item in mh["rows"]:
        vals = (item["discipline"], item["engineering"], item["fabrication"], item["offshore"], item["total"], item["total"] / total)
        for ci, v in enumerate(vals, start=1):
            cell = ws.cell(row=row, column=ci, value=v); cell.font = mw.BODY; cell.border = mw.BORDER
            if ci == 6:
                cell.number_format = "0.0%"
        row += 1
    vals = ("Total", mh["totals"]["engineering"], mh["totals"]["fabrication"], mh["totals"]["offshore"], total, 1.0)
    for ci, v in enumerate(vals, start=1):
        cell = ws.cell(row=row, column=ci, value=v); cell.font = mw.BOLD; cell.border = mw.BORDER
        if ci == 6:
            cell.number_format = "0.0%"
    row += 2
    mw.section(ws, row, "Labour cost reconciliation", 6); row += 1
    mw.header_row(ws, row, ["Discipline group", "Hours", "Rate, USD/h", "Cost, MUSD", "", ""]); row += 1
    for k, label in (("engineering", "Engineering"), ("fabrication", "Fabrication"), ("offshore", "Offshore")):
        vals = (label, mh["totals"][k], mh["rates_usd_per_h"][k], mh["labour_cost_musd"][k], "", "")
        for ci, v in enumerate(vals, start=1):
            cell = ws.cell(row=row, column=ci, value=v); cell.font = mw.BODY; cell.border = mw.BORDER
        row += 1
    vals = ("Total labour", total, "", mh["labour_cost_total_musd"], "", "")
    for ci, v in enumerate(vals, start=1):
        cell = ws.cell(row=row, column=ci, value=v); cell.font = mw.BOLD; cell.border = mw.BORDER
    row += 2
    ws.cell(row=row, column=1, value=("The 32,600 h is the total estimate. Engineering, fabrication and offshore hours carry different rates. "
                                      "Vendor shop hours are inside the equipment and fabrication allowances; the schedule histogram spreads all hours by activity.")).font = mw.SMALL
    ws.merge_cells(start_row=row, start_column=1, end_row=row, end_column=6)

    # ------------------------------------------------------------- resource plan
    ws = sheet(wb, "Resource Plan", "Resource-loaded project plan",
               "42-week plan, activity list, weekly histogram and S-curve")
    ws.column_dimensions["A"].width = 8
    ws.column_dimensions["B"].width = 10
    ws.column_dimensions["C"].width = 46
    ws.column_dimensions["D"].width = 8
    ws.column_dimensions["E"].width = 9
    ws.column_dimensions["F"].width = 9
    ws.column_dimensions["G"].width = 14
    ws.column_dimensions["H"].width = 24
    ws.column_dimensions["I"].width = 8
    row = 4
    mw.header_row(ws, row, ["ID", "Phase", "Activity", "Start wk", "Duration", "Finish", "Predecessors", "Discipline", "Hours"]); row += 1
    for a in F["schedule"]["activities"]:
        vals = (a["id"], a["phase"], a["name"], a["start"], a["duration"], a["finish"], a["predecessors"], a["discipline"], a["hours"])
        for ci, v in enumerate(vals, start=1):
            cell = ws.cell(row=row, column=ci, value=v); cell.font = mw.BODY; cell.border = mw.BORDER
        row += 1
    row += 1
    mw.section(ws, row, "Weekly histogram and cumulative hours", 9); row += 1
    mw.header_row(ws, row, ["Week", "Hours", "Cumulative", "", "", "", "", "Milestone", ""]); row += 1
    milestones = {m["week"]: m["milestone"] for m in F["schedule"]["milestones"]}
    for i in range(F["schedule"]["weeks"]):
        vals = (i + 1, F["schedule"]["histogram_hours"][i], F["schedule"]["cumulative_hours"][i], "", "", "", "", milestones.get(i + 1, ""), "")
        for ci, v in enumerate(vals, start=1):
            cell = ws.cell(row=row, column=ci, value=v); cell.font = mw.BODY; cell.border = mw.BORDER
        row += 1

    # ------------------------------------------------------------- DEXPI
    ws = sheet(wb, "DEXPI", "DEXPI process flow diagram register",
               "AZ-FEED-08-DEXPI-PFD-001: DEXPI 2.0 Process exchange with embedded drawing")
    ws.column_dimensions["A"].width = 40
    ws.column_dimensions["B"].width = 70
    ws.column_dimensions["C"].width = 26
    rows = [
        ("Document", "AZ-FEED-08-DEXPI-PFD-001", "Rev B"),
        ("Format", "DEXPI 2.0 Process exchange (XML) with Drawing", "dexpi20 / process"),
        ("Model URI", "urn:azoth:dexpi:2.0:process:flowsheets_feed08_debottleneck", ""),
        ("Equipment represented", "31 process steps: 3 separators, 9 compressors, 9 coolers, 2 let-down valves, stock tank, mixer, feed and 4 products", "semantic model"),
        ("Streams represented", "32 process streams; flash gas routed to fuel gas, not the sales header", "semantic model"),
        ("Drawing primitives", "62 groups, 186 primitives (polylines, polygons, text)", "embedded drawing"),
        ("Schema validation", "valid; no diagnostics", "azoth.dexpi.validate"),
        ("Export report", "converged; semantic_valid true; approval NOT_A_DEXPI_EV_CERTIFICATE; CAE round trip QUALIFICATION_REQUIRED", "azoth.dexpi.export_process"),
        ("Rendering", "A0 landscape SVG/PDF; title block, notes, equipment schedule", "build/render_dexpi.py"),
        ("Scope note", "PFD only; instruments, PSV and flare are on the simplified P&ID AZ-FEED-08-PID-001", ""),
    ]
    row = 4
    for k, v, note in rows:
        ws.cell(row=row, column=1, value=k).font = mw.BOLD
        ws.cell(row=row, column=2, value=v).font = mw.BODY
        ws.cell(row=row, column=3, value=note).font = mw.SMALL
        for c in range(1, 4):
            ws.cell(row=row, column=c).border = mw.BORDER
            ws.cell(row=row, column=c).alignment = Alignment(wrap_text=True, vertical="top")
        row += 1

    # ------------------------------------------------------------- HAZOP
    if HAZ:
        ws = sheet(wb, "HAZOP", "Independent HAZOP worksheet",
                   "Frozen Rev B design reviewed by a separate HAZOP team")
        headers = ["Node", "Guideword", "Deviation", "Cause", "Consequence", "Safeguard", "Severity", "Likelihood", "Risk", "Action", "Owner", "Status"]
        widths = [26, 14, 34, 40, 44, 44, 9, 10, 9, 46, 12, 10]
        mw.header_row(ws, 4, headers, widths)
        r = 5
        for item in HAZ["worksheet"]:
            vals = (item["node"], item["guideword"], item["deviation"], item["cause"], item["consequence"],
                    item["safeguard"], item["severity"], item["likelihood"], item["risk"], item["action"],
                    item["owner"], item["status"])
            for ci, v in enumerate(vals, start=1):
                cell = ws.cell(row=r, column=ci, value=v); cell.font = mw.BODY; cell.border = mw.BORDER
                cell.alignment = Alignment(wrap_text=True, vertical="top")
            r += 1
        r += 1
        ws.cell(row=r, column=1, value="Actions close-out").font = Font(name="Calibri", size=11, bold=True, color=mw.NAVY)
        r += 1
        mw.header_row(ws, r, ["Action", "Finding", "Action", "Owner", "Priority", "Status"]) ; r += 1
        for a in HAZ["actions"]:
            vals = (a["id"], a["finding"], a["action"], a["owner"], a["priority"], a["status"])
            for ci, v in enumerate(vals, start=1):
                cell = ws.cell(row=r, column=ci, value=v); cell.font = mw.BODY; cell.border = mw.BORDER
                cell.alignment = Alignment(wrap_text=True, vertical="top")
            r += 1

    # ------------------------------------------------------------- closeout
    if CLOSE and CLOSERES:
        ws = sheet(wb, "Closeout", "HAZOP close-out register (Rev C)",
                   "16 of 16 actions dispositioned; relief credit withdrawn and economics reissued")
        headers = ["ID", "Disposition", "Basis", "Evidence", "Residual condition", "Owner", "Date"]
        mw.header_row(ws, 4, headers, [9, 26, 34, 60, 44, 12, 11])
        r = 5
        for a in CLOSE["actions"]:
            vals = (a["id"], a["disposition"], a["basis"], a["evidence"], a["residual_condition"], a["owner"], a["date"])
            for ci, v in enumerate(vals, start=1):
                cell = ws.cell(row=r, column=ci, value=v); cell.font = mw.BODY; cell.border = mw.BORDER
                cell.alignment = Alignment(wrap_text=True, vertical="top")
            r += 1
        r += 1
        ws.cell(row=r, column=1, value="Revised cost and economics").font = Font(name="Calibri", size=11, bold=True, color=mw.NAVY)
        r += 1
        mw.header_row(ws, r, ["Case", "Capex, MUSD", "NPV, MUSD", "Payback", "", "", ""]); r += 1
        for name, v in CLOSERES["economics"].items():
            vals = (name, v["capex_musd"], round(v["npv_musd"], 2), v["payback"], "", "", "")
            for ci, val in enumerate(vals, start=1):
                cell = ws.cell(row=r, column=ci, value=val); cell.font = mw.BODY; cell.border = mw.BORDER
            r += 1
        r += 1
        ws.cell(row=r, column=1, value="Rev C cost build-up").font = Font(name="Calibri", size=11, bold=True, color=mw.NAVY)
        r += 1
        mw.header_row(ws, r, ["Item", "MUSD", "", "", "", "", ""]); r += 1
        for item in CLOSERES["cost"]["items"]:
            vals = (item["item"], item["musd"], "", "", "", "", "")
            for ci, val in enumerate(vals, start=1):
                cell = ws.cell(row=r, column=ci, value=val); cell.font = mw.BODY; cell.border = mw.BORDER
            r += 1
        vals = ("Total", CLOSERES["cost"]["total_musd"], "", "", "", "", "")
        for ci, val in enumerate(vals, start=1):
            cell = ws.cell(row=r, column=ci, value=val); cell.font = mw.BOLD; cell.border = mw.BORDER

    for s in wb.worksheets:
        s.page_setup.orientation = "landscape"
        s.sheet_properties.pageSetUpPr.fitToPage = True
        s.page_setup.fitToWidth = 1
        s.page_setup.fitToHeight = 0
    wb.save(mw.OUT)
    print("wrote", mw.OUT)


if __name__ == "__main__":
    build()
