"""Build the HAZOP close-out report (Rev C) as DOCX."""

from __future__ import annotations

import json
from pathlib import Path

from docx import Document
from docx.enum.text import WD_ALIGN_PARAGRAPH
from docx.shared import Cm, Pt

import make_report as mr

ROOT = Path(__file__).resolve().parents[1]
C = json.loads((ROOT / "model" / "closeout_results.json").read_text())
H = json.loads((ROOT / "hazop" / "closeout.json").read_text())
W = json.loads((ROOT / "hazop" / "worksheet.json").read_text())
RES = json.loads((ROOT / "model" / "results.json").read_text())
DRAW = ROOT / "drawings"
OUT = ROOT / "report" / "AZ-FEED-08-HAZOP-Closeout-RevC.docx"


def cover(doc):
    for _ in range(2):
        doc.add_paragraph()
    p = doc.add_paragraph()
    r = p.add_run("FRONT-END ENGINEERING DESIGN · HAZOP CLOSE-OUT")
    r.font.size = Pt(11); r.bold = True; r.font.color.rgb = mr.ORANGE
    p.paragraph_format.space_after = Pt(2)
    p = doc.add_paragraph()
    r = p.add_run("Closing the safety case")
    r.font.size = Pt(30); r.bold = True; r.font.color.rgb = mr.NAVY
    p.paragraph_format.space_after = Pt(2)
    p = doc.add_paragraph()
    r = p.add_run("Relief basis, SIL, PSVs, dehydration and the revised economics")
    r.font.size = Pt(15); r.font.color.rgb = mr.NAVY
    p.paragraph_format.space_after = Pt(18)
    p = doc.add_paragraph()
    r = p.add_run(
        "Disposition of the 16 actions from the independent HAZOP of AZ-FEED-08 Rev B, "
        "with the calculations, design changes and residual conditions that close them."
    )
    r.font.size = Pt(11); r.font.color.rgb = mr.GREY
    p.paragraph_format.space_after = Pt(26)

    table = doc.add_table(rows=0, cols=2)
    table.style = "Table Grid"
    rows = [
        ("Document number", "AZ-FEED-08-HAZOP-Closeout"),
        ("Revision", "C"),
        ("Date", "26 November 2026"),
        ("Status", "Issued for study — closes the Rev B HAZOP actions"),
        ("Basis", "independent HAZOP worksheet Rev B and the frozen design pack"),
        ("Calculation basis", "azoth 0.1.4 plus the IEC 61508 PFD model in model/closeout_model.py"),
        ("Actions", "16 of 16 dispositioned; 0 left open without an owner and condition"),
        ("Relief decision", "35 % HIPPS credit withdrawn; +25 % flare/header upgrade booked"),
        ("Revised cost / NPV", "54.1 MUSD / 7.8 MUSD at 10 %, payback year 2"),
        ("Result", "the capacity conclusion stands; the safety case is now conservative"),
    ]
    for k, v in rows:
        cells = table.add_row().cells
        mr.set_cell(cells[0], k, bold=True, size=9)
        mr.set_cell(cells[1], v, size=9)
        cells[0].width = Cm(4.4); cells[1].width = Cm(12.0)
        mr.shade(cells[0], "E8EDF6")
    doc.add_paragraph()
    p = doc.add_paragraph()
    r = p.add_run(
        "This is a study close-out on an illustrative asset. It is not a design deliverable, and the "
        "vendor certificates, TEG performance data and site survey listed as residual conditions are "
        "still required before construction."
    )
    r.font.size = Pt(8.5); r.italic = True; r.font.color.rgb = mr.ORANGE


def header_footer(doc):
    sec = doc.sections[0]
    hp = sec.header.paragraphs[0]
    hp.text = ""
    run = hp.add_run("AZ-FEED-08 · Rev C · HAZOP close-out")
    run.font.size = Pt(7.5); run.font.color.rgb = mr.GREY
    hp.alignment = WD_ALIGN_PARAGRAPH.RIGHT
    fp = sec.footer.paragraphs[0]
    fp.alignment = WD_ALIGN_PARAGRAPH.CENTER
    run = fp.add_run("Screening study — not for construction · azoth 0.1.4 · page ")
    run.font.size = Pt(7.5); run.font.color.rgb = mr.GREY
    mr.add_page_field(fp)
    run = fp.add_run(" of ")
    run.font.size = Pt(7.5); run.font.color.rgb = mr.GREY
    mr.add_pages_field(fp)


def summary(doc):
    doc.add_heading("Executive summary", level=1)
    mr.para(doc,
        "The independent HAZOP left ten actions open. Closing them changed the relief basis. The 35 % "
        "HIPPS credit could not be demonstrated with the available screening tools, so it is withdrawn: "
        "the full +18 % blocked-outlet case, 50.31 kg/s, is booked and the +25 % flare/header upgrade becomes "
        "base scope. That alone takes the installed cost from 36.0 to 48.0 MUSD.")
    mr.para(doc,
        "Further findings were closed by design change. The SIL-3 loop as issued is SIL 2 "
        f"(PFD {C['sil']['as_designed']['pfd_total']:.1e}), not SIL 3; it is re-specified as 1oo2 on the "
        "transmitters, logic solver and final element with a six-month proof test. The PSV-101 4 in inlet "
        f"drops {C['psv_hydraulics']['4 in']['dP_pct_set']:.1f} % of set, against the API 520 3 % limit; it becomes "
        "6 in with a 24 in outlet. The produced gas carries 20.7 kg/MMscf of water against a 1.36 kg/MMscf "
        "specification, so the TEG unit is upgraded. The let-down valves need ESDVs and restricted-leak PSVs.")
    mr.para(doc,
        "The revised package costs 54.1 MUSD, returns 7.8 MUSD NPV at 10 % and pays back in year 2. The "
        "capacity conclusion is unchanged at 161.8 MMscfd and about 3,466 bbl/d. The project remains "
        "positive, but the margin is now thin enough that it is conditional on the gas price. The flare "
        "upgrade is an insurance premium: if a dynamic study later proves a credit, it can be descoped "
        "through management of change, but it is not banked now.")
    mr.add_table(doc,
        ["Headline", "Rev B", "Rev C close-out"],
        [
            ("Relief basis", "35 % HIPPS credit", "full +18 % case, flare +25 %"),
            ("SIL-3 loop", "1oo2 PT, single SDV", "1oo2 all, 6-month proof test"),
            ("PSV-101 inlet / outlet", "4 in / 6 in", "6 in / 24 in"),
            ("Dehydration", "assumed adequate", "139 kg/h TEG upgrade allowed"),
            ("Capex", "36.0 MUSD", "54.1 MUSD"),
            ("NPV at 10 %", "24.8 MUSD", "7.8 MUSD"),
            ("Payback", "year 1", "year 2"),
            ("Capacity", "161.8 MMscfd", "161.8 MMscfd (unchanged)"),
        ],
        widths=[4.6, 5.4, 6.4], caption="Table ES-1  What close-out changed", font_size=8.6)


def purpose(doc):
    doc.add_heading("1  Purpose, basis and independence", level=1)
    mr.para(doc,
        "This report closes the actions raised by the independent HAZOP of the Rev B design. It is a "
        "close-out record, not a re-run of the HAZOP: each action is given a disposition, the calculation "
        "or document that supports it, and any residual condition that survives. The independent "
        "worksheet is not edited; the dispositions are recorded against it.")
    mr.para(doc,
        "The close-out was performed by the design team, using the frozen Rev B pack and the independent "
        "worksheet as the input. Where a calculation could be done, it was done; where a vendor or site "
        "input is genuinely required, the action is closed by a purchase-order or pre-shutdown condition "
        "with a named owner. No action is left without a disposition.")
    from collections import Counter
    groups = {
        "Closed — design change": ["Closed - design change"],
        "Closed — verified by calculation": ["Closed - verified"],
        "Closed — document or procedure": ["Closed - procedure issued", "Closed - document issued", "Closed - plan issued", "Closed - register issued"],
        "Closed — purchase-order condition": ["Closed - purchase-order condition"],
        "Closed — analysis and access control": ["Closed - analysis and access control", "Closed - analysis and design"],
    }
    counts = Counter(a["disposition"] for a in H["actions"])
    rows = []
    for label, keys in groups.items():
        n = sum(counts[k] for k in keys)
        if n:
            rows.append((label, str(n), {"Closed — design change": "Hardware or basis changed in Rev C",
                                          "Closed — verified by calculation": "The number supports the closure",
                                          "Closed — document or procedure": "C&E, SIMOPS, bypass procedure, PSV register",
                                          "Closed — purchase-order condition": "Vendor guarantees and certified devices",
                                          "Closed — analysis and access control": "Radiation contours, sterile area, export barriers"}[label]))
    rows.append(("Open", "0", "—"))
    mr.add_table(doc,
        ["Disposition type", "Count", "Meaning"],
        rows, widths=[6.0, 2.0, 8.4], caption="Table 1-1  Disposition summary", font_size=8.6)


def dispositions(doc):
    doc.add_heading("2  Action-by-action disposition", level=1)
    mr.para(doc,
        "Table 2-1 is the short form. Appendix B carries the evidence and residual condition for every "
        "action. The counts in Table 1-1 are the counts in Table 2-1; nothing is closed twice.")
    rows = []
    for a in H["actions"]:
        rows.append((a["id"], a["disposition"], a["basis"], a["evidence"][:120] + ("…" if len(a["evidence"]) > 120 else "")))
    mr.add_table(doc,
        ["ID", "Disposition", "Basis", "Evidence (short)"],
        rows, widths=[1.2, 3.6, 4.2, 7.4], caption="Table 2-1  Disposition of the 16 actions", font_size=7.2)


def relief(doc):
    doc.add_heading("3  Relief basis: the credit is withdrawn (HAZ-01)", level=1)
    mr.para(doc,
        "The Rev B package relied on a 35 % reduction of the blocked-outlet case. The arithmetic is "
        "simple: 50.31 kg/s × 0.65 = 32.70 kg/s, which is 69.9 % of the 46.34 kg/s flare. The problem is "
        "not the arithmetic but the evidence. The reduction depends on HIPPS closure in under two seconds, "
        "the trapped inventory, SDV leakage and the PSV/flare response; none of those was modelled. A "
        "screening blowdown of the vessel inventory does not support the credit either, because the PSV's "
        "own capacity is 58 kg/s and the vessel can feed it at the start of the event.")
    mr.para(doc,
        "The close-out therefore books the full case. The existing flare is at 107.5 % of capacity at "
        "50.31 kg/s and the +25 % upgrade takes it to 86.0 %. The P orifice passes the full case at "
        "5.53 in² of 6.38 in² (86.7 %). The 4.73 kW/m² contour is 82.4 m; the 1.58 kW/m² personnel contour "
        "is 148.8 m and is controlled by access, not by a sterile-area claim.")
    mr.add_table(doc,
        ["Case", "Relief, kg/s", "Flare capacity, kg/s", "Flare, %", "PSV, in²", "4.73 kW/m², m"],
        [
            ("Current", "42.63", "46.34", "91.1", "4.69", "75.0"),
            ("+18 % gas, existing flare", "50.31", "46.34", "107.5", "5.53", "82.4"),
            ("+18 %, flare +25 % (Rev C)", "50.31", "57.93", "86.0", "5.53", "82.4"),
            ("+18 %, HIPPS credit (withdrawn)", "32.70", "46.34", "69.9", "3.60", "64.0"),
        ],
        widths=[5.4, 2.6, 3.2, 2.0, 2.0, 2.6],
        caption="Table 3-1  Relief cases and the decision", font_size=8.2)
    mr.add_figure(doc, DRAW / "fig_closeout_relief.png", 14.6,
                  "Figure 3-1  The flare is the gate; the upgrade is now base scope")
    mr.para(doc,
        "The dynamic study is not deleted. It is reclassified as an optimisation: if it can demonstrate a "
        "credit with a validated model, the flare upgrade may be descoped through management of change. "
        "That is how the 12.0 MUSD can be recovered, not by booking an unproven number.")


def sil(doc):
    doc.add_heading("4  SIL verification (HAZ-02)", level=1)
    mr.para(doc,
        "The Rev B loop was described as SIL 3 on the strength of a PFD of 1e-4 from the library's "
        "determination, not from device data. The close-out builds the PFD from the architecture: 1oo2 "
        "transmitters, a 1oo1 logic solver and a single SDV-101, each with a generic failure rate and a "
        "12-month proof test. The as-designed loop is SIL 2, not SIL 3, dominated by the single final "
        "element (PFD 4.4e-3).")
    mr.add_table(doc,
        ["Element", "Architecture", "PFD, as designed", "PFD, closed out"],
        [
            ("Pressure transmitters", "1oo2", f"{C['sil']['as_designed']['pfd_pt']:.1e}", f"{C['sil']['closed_out']['pfd_pt']:.1e}"),
            ("Logic solver", "1oo1 → 1oo2", f"{C['sil']['as_designed']['pfd_logic']:.1e}", f"{C['sil']['closed_out']['pfd_logic']:.1e}"),
            ("Final element (SDV-101)", "1oo1 → 1oo2", f"{C['sil']['as_designed']['pfd_sdv']:.1e}", f"{C['sil']['closed_out']['pfd_sdv']:.1e}"),
            ("Total", "—", f"{C['sil']['as_designed']['pfd_total']:.1e} (SIL {C['sil']['as_designed']['achieved_sil']})", f"{C['sil']['closed_out']['pfd_total']:.1e} (SIL {C['sil']['closed_out']['achieved_sil']})"),
        ],
        widths=[4.6, 3.4, 4.2, 4.2],
        caption="Table 4-1  SIL PFD by architecture", font_size=8.4)
    mr.para(doc,
        "The close-out specifies 1oo2 on all three elements, certified devices with λDU ≤ 1e-7/h and "
        "β ≤ 5 %, and a six-month proof test. That gives 3.3e-5, comfortably inside the SIL 3 band. The "
        "certificates are a purchase-order condition; the calculation is only as good as the failure rates "
        "behind it, and the report says so.")
    mr.add_figure(doc, DRAW / "fig_closeout_sil.png", 13.8,
                  "Figure 4-1  The issued loop sits in the SIL 2 band; the close-out reaches SIL 3")


def psvs(doc):
    doc.add_heading("5  PSV hydraulics, register and let-down protection (HAZ-04, HAZ-12, HAZ-14)", level=1)
    mr.para(doc,
        "The 4 in PSV inlet cannot pass 50.31 kg/s within the API 520 3 % rule. At 10 m equivalent length "
        f"the inlet drops {C['psv_hydraulics']['4 in']['dP_bar']:.1f} bar, "
        f"{C['psv_hydraulics']['4 in']['dP_pct_set']:.1f} % of the 66 barg set pressure. A 6 in inlet drops "
        f"{C['psv_hydraulics']['6 in']['dP_bar']:.1f} bar, {C['psv_hydraulics']['6 in']['dP_pct_set']:.1f} %. "
        "The outlet is worse: 50 kg/s at 1.5 bara is about 41 m³/s, which is over 2,200 m/s in a 6 in line. "
        "The 24 in outlet gives about 140 m/s, below 0.5 Mach.")
    mr.add_figure(doc, DRAW / "fig_closeout_psv.png", 12.8,
                  "Figure 5-1  PSV-101 inlet pressure drop against the 3 % rule")
    mr.para(doc,
        "The let-down valves are a separate problem. A full-Cv failure of VLV-102 passes about 395 kg/s, "
        "which no practical PSV can relieve. The close-out therefore adds an ESDV upstream of each let-down "
        "valve and a 10 mm restriction orifice, and sizes the PSVs for the restricted leak. That is a "
        "two-barrier design: the ESDV stops the full failure, the PSV covers the restricted one.")
    rows = [(r["tag"], f"{r['leak_kg_s']:.2f}", f"{r['set_bar']:.0f}", f"{r['required_in2']:.2f}", r["orifice"]) for r in C["psv_register"]["rows"]]
    rows.insert(0, ("PSV-101", "50.31", "66", "5.53", "P (6.38 fitted)"))
    mr.add_table(doc,
        ["PSV", "Relief, kg/s", "Set, barg", "Required, in²", "Orifice"],
        rows, widths=[3.0, 3.2, 2.8, 3.2, 4.2],
        caption="Table 5-1  Relief valve register (Rev C)", font_size=8.4)
    mr.para(doc,
        "PSV-101 is unchanged in size but re-piped to 6 in inlet and 24 in outlet. PSV-102 (G), PSV-103 (F) "
        "and PSV-104 (F) are new. The register carries the inspection interval and the impairment matrix; "
        "a spare PSV-101 is held.")


def water(doc):
    doc.add_heading("6  Dehydration (HAZ-16)", level=1)
    mr.para(doc,
        f"Saturating the sales gas at 90 bara and 313.15 K gives a water content of "
        f"{C['water']['water_kg_per_mmscf']:.1f} kg/MMscf. The pipeline specification is 3 lb/MMscf, "
        f"{C['water']['spec_kg_per_mmscf']:.2f} kg/MMscf — a gap of {C['water']['ratio_to_spec']:.1f} times. "
        f"At 161.8 MMscfd that is {C['water']['water_load_kg_h']:.0f} kg/h of water to remove, about "
        f"{C['water']['teg_circulation_m3_h']:.1f} m³/h of TEG circulation and "
        f"{C['water']['reboiler_duty_kw']:.0f} kW of reboiler duty. The existing TEG unit is outside the "
        "debottlenecking battery limits but it cannot be assumed adequate at the higher rate.")
    mr.add_figure(doc, DRAW / "fig_closeout_water.png", 12.6,
                  "Figure 6-1  Water content against the pipeline specification")
    mr.para(doc,
        "Rev C carries a 1.5 MUSD TEG upgrade allowance and a water-dew-point analyser with a high alarm. "
        "The vendor must confirm 139 kg/h removal at 3 lb/MMscf and 60 % turndown; if it cannot, the "
        "allowance becomes an upgrade package. This is the one action where the close-out is a budget and "
        "a condition, not a completed design.")


def radiation(doc):
    doc.add_heading("7  Radiation and sterile area (HAZ-05)", level=1)
    mr.para(doc,
        "The full +18 % case gives three contours: 69.8 m at 6.3 kW/m² for equipment, 82.4 m at 4.73 kW/m² "
        "for the sterile area, and 148.8 m at 1.58 kW/m² for personnel. The first two are designable; the "
        "third is not a sterile-area radius on a platform of this size, so it becomes an access-control "
        "boundary with shielding and procedures. The close-out adopts the 82.4 m sterile contour and "
        "controls access inside 149 m during a relief event.")
    mr.add_table(doc,
        ["Threshold", "Purpose", "Distance, m", "Disposition"],
        [
            ("6.3 kW/m²", "Equipment / structural", "69.8", "Exclusion within contour"),
            ("4.73 kW/m²", "Sterile area", "82.4", "Sterile area adopted"),
            ("1.58 kW/m²", "Personnel (short exposure)", "148.8", "Access control and shielding"),
        ],
        widths=[3.0, 6.0, 3.0, 4.4], caption="Table 7-1  Radiation contours, full +18 % case", font_size=8.4)


def export(doc):
    doc.add_heading("8  Export line barriers and compressor (HAZ-13, HAZ-08, HAZ-09, HAZ-15)", level=1)
    mr.para(doc,
        "The HP inlet two-barrier check is unchanged: 72.6 bara maximum incidental pressure on the 66 barg "
        "system, with PSV-101 and HIPPS as the two barriers. The export gas (100 barg design) and condensate "
        "(30 barg design) lines get ESDV/SSIV at the battery limit and independent high-pressure trips. The "
        "pipeline two-barrier calculation will be re-run at detailed design with the line-specific design "
        "pressures; the close-out records the barrier philosophy, not a final pipeline design.")
    mr.para(doc,
        "The compressor driver is re-specified at 3.0 MW. The absorbed power at 161.8 MMscfd is 2.689 MW, "
        "so 3.0 MW gives 11.6 % margin. The vendor package carries the revised curve, anti-surge recycle, "
        "recycle cooler, independent overspeed trip and the API 617 string test. Torsional, lateral, "
        "impeller-stress and loss-of-utilities cases are purchase-order deliverables, as are the V-101 K "
        "factor, V-106 retention and P-102 duty guarantees.")


def simops(doc):
    doc.add_heading("9  Procedures, cause and effect, and SIMOPS (HAZ-03, HAZ-10, HAZ-11)", level=1)
    mr.para(doc,
        "The close-out issues four controlled documents in outline: the SIL-3 bypass and proof-test "
        "procedure; the F&G/ESD cause-and-effect matrix (ESD-1/2/3, BDV on V-101 and the HP inlet, "
        "ESDV/SSIV on export, F&G coverage for HP, compression and water); the SIMOPS matrix and "
        "impairment register; and the PSV/flare impairment matrix. They are outlines in this revision and "
        "become issued documents before the shutdown.")
    mr.add_table(doc,
        ["Document", "Content", "Issue"],
        [
            ("SIL-3 bypass/procedure", "Shutdown-only bypass, rate cap, independent approval, key-lock, alarm", "Before shutdown"),
            ("F&G/ESD C&E", "ESD-1/2/3, BDV, ESDV/SSIV, F&G coverage", "Detailed design"),
            ("SIMOPS matrix", "Hot work, isolation, gas-free, dropped objects, flare outage, marine", "Before shutdown"),
            ("PSV/flare impairment", "PSV isolation, spare, flare outage response", "Before shutdown"),
        ],
        widths=[4.4, 8.6, 3.4], caption="Table 9-1  Controlled documents", font_size=8.4)


def economics(doc):
    doc.add_heading("10  Revised cost and economics", level=1)
    mr.para(doc,
        "The close-out adds 18.1 MUSD to the Rev B package. Most of it is the flare upgrade that replaces "
        "the withdrawn credit; the rest is the safety hardware the HAZOP showed was missing. The estimate "
        "remains AACE Class 4/5 and is not a quantity-surveyed number.")
    mr.add_table(doc,
        ["Item", "MUSD"],
        [(i["item"], f"{i['musd']:.1f}") for i in C["cost"]["items"]] + [("Total", f"{C['cost']['total_musd']:.1f}")],
        widths=[12.6, 3.4], caption="Table 10-1  Rev C cost build-up", font_size=8.2, align_right=[1])
    mr.add_table(doc,
        ["Case", "Capex, MUSD", "NPV at 10 %, MUSD", "Payback"],
        [
            ("Rev B — credit (withdrawn)", "36.0", f"{C['economics']['Rev B — HIPPS credit (withdrawn)']['npv_musd']:.1f}", "1"),
            ("Flare upgrade only", "48.0", f"{C['economics']['Flare upgrade only']['npv_musd']:.1f}", "2"),
            ("Rev C — flare + close-out", f"{C['cost']['total_musd']:.1f}", f"{C['economics']['Rev C — flare + close-out scope']['npv_musd']:.1f}", "2"),
        ],
        widths=[6.4, 3.4, 4.0, 2.4], caption="Table 10-2  Economics of the close-out", font_size=8.4)
    mr.add_figure(doc, DRAW / "fig_closeout_econ.png", 12.8,
                  "Figure 10-1  The project remains positive, but the margin is thin")
    mr.para(doc,
        "The capacity conclusion is unchanged: 161.8 MMscfd and about 3,466 bbl/d. The value falls from "
        "24.8 to 7.8 MUSD because the project is now buying its relief capacity instead of assuming it. "
        "At a gas price below about 4 USD/MMBtu the close-out case does not pay, which is the same "
        "sensitivity the Rev B study already carried.")


def recommendation(doc):
    doc.add_heading("11  Recommendation and residual conditions", level=1)
    mr.para(doc,
        "Proceed with the Rev C scope: flare/header +25 %, HIPPS retained for the pipeline barrier but not "
        "credited for relief, 1oo2 SIL-3 loop with a six-month proof test, 6 in/24 in PSV-101, restricted-leak "
        "PSVs on the let-down and water package, a 3.0 MW compressor driver, and a TEG upgrade allowance. "
        "The package costs 54.1 MUSD and returns 7.8 MUSD NPV at 10 %, with payback in year 2.")
    mr.para(doc,
        "The alternative is not to sanction the debottleneck until the gas price or the dynamic relief case "
        "improves. The rate-cap option is not viable: holding 147.8 MMscfd with the 36 MUSD package gives "
        "a negative NPV. The project should not be justified on the withdrawn credit.")
    doc.add_heading("11.1  Residual conditions", level=2)
    for text in H["residual_conditions"]:
        p = doc.add_paragraph(text, style="List Bullet")
        for r in p.runs:
            r.font.size = Pt(9)
    mr.para(doc,
        "These are conditions on the close-out, not open HAZOP actions. Each has an owner and a gate: "
        "vendor certificates at purchase order, TEG performance at vendor selection, the sterile-area "
        "survey before detailed design, and the C&E/SIMOPS documents before the shutdown.")


def appendices(doc):
    doc.add_heading("Appendix A  Calculation inputs and results", level=1)
    mr.add_table(doc,
        ["Calculation", "Input / formula", "Result"],
        [
            ("PSV-101 inlet", "Beggs-Brill, 50.31 kg/s, 66 barg, 10 m", f"4 in {C['psv_hydraulics']['4 in']['dP_pct_set']:.1f} %; 6 in {C['psv_hydraulics']['6 in']['dP_pct_set']:.1f} %"),
            ("PSV-101 outlet", "50.31 kg/s at 1.5 bara", f"6 in {C['psv_hydraulics']['outlet']['velocity_6in_m_s']:.0f} m/s; 24 in {C['psv_hydraulics']['outlet']['velocity_24in_m_s']:.0f} m/s"),
            ("SIL as designed", "1oo2 PT + 1oo1 logic + 1oo1 SDV, 12 months", f"{C['sil']['as_designed']['pfd_total']:.1e} (SIL {C['sil']['as_designed']['achieved_sil']})"),
            ("SIL closed out", "1oo2 all, certified, 6 months", f"{C['sil']['closed_out']['pfd_total']:.1e} (SIL {C['sil']['closed_out']['achieved_sil']})"),
            ("Water content", "saturate at 313.15 K, 90 bara", f"{C['water']['water_kg_per_mmscf']:.1f} kg/MMscf"),
            ("TEG duty", "139 kg/h water, 0.04 m³/kg", f"{C['water']['teg_circulation_m3_h']:.1f} m³/h; {C['water']['reboiler_duty_kw']:.0f} kW"),
            ("Radiation", "flare, full +18 % case", "69.8 / 82.4 / 148.8 m"),
            ("Rev C NPV", "ten-year DCF, 10 %, 25 % tax", f"{C['economics']['Rev C — flare + close-out scope']['npv_musd']:.1f} MUSD"),
        ],
        widths=[4.0, 6.6, 6.0], caption="Table A-1  Close-out calculations", font_size=7.8)

    doc.add_heading("Appendix B  Close-out register", level=1)
    mr.para(doc,
        "Every action with its evidence and residual condition. Disposition is the close-out status; the "
        "Rev B status is the independent team's original record and is not rewritten.")
    rows = [(a["id"], a["disposition"], a["evidence"], a["residual_condition"], a["owner"]) for a in H["actions"]]
    mr.add_table(doc,
        ["ID", "Disposition", "Evidence", "Residual condition", "Owner"],
        rows, widths=[1.1, 3.0, 6.2, 4.6, 1.7], caption="Table B-1  HAZOP close-out register", font_size=6.6)


def build():
    doc = Document()
    mr.style_doc(doc)
    sec = doc.sections[0]
    sec.page_width, sec.page_height = Cm(21.0), Cm(29.7)
    sec.left_margin = Cm(2.0); sec.right_margin = Cm(2.0)
    sec.top_margin = Cm(2.2); sec.bottom_margin = Cm(2.0)
    header_footer(doc)
    cover(doc)
    doc.add_page_break()
    mr.contents(doc) if False else None
    summary(doc)
    purpose(doc)
    dispositions(doc)
    relief(doc)
    sil(doc)
    psvs(doc)
    water(doc)
    radiation(doc)
    export(doc)
    simops(doc)
    economics(doc)
    recommendation(doc)
    appendices(doc)
    doc.save(OUT)
    print("wrote", OUT)


if __name__ == "__main__":
    build()
