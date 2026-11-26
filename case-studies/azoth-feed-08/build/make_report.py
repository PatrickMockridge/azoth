"""Build the FEED study report (DOCX) for the offshore debottlenecking case study."""

from __future__ import annotations

import json
from pathlib import Path

from docx import Document
from docx.enum.section import WD_ORIENT, WD_SECTION
from docx.enum.table import WD_TABLE_ALIGNMENT
from docx.enum.text import WD_ALIGN_PARAGRAPH, WD_BREAK
from docx.oxml import OxmlElement
from docx.oxml.ns import qn
from docx.shared import Cm, Pt, RGBColor

ROOT = Path(__file__).resolve().parents[1]
RES = json.loads((ROOT / "model" / "results.json").read_text())
DRAW = ROOT / "drawings"
OUT = ROOT / "report" / "AZ-FEED-08-Offshore-Debottlenecking-Report.docx"

NAVY = RGBColor(0x1F, 0x38, 0x64)
GREY = RGBColor(0x59, 0x59, 0x59)
ORANGE = RGBColor(0xC5, 0x50, 0x1F)
GREEN = RGBColor(0x1B, 0x7A, 0x56)

T = RES["train"]["current"]
TP = RES["train"]["potential"]
CAP = RES["capacity"]
REL = RES["relief"]
ECON = RES["economics"]
B = RES["basis"]
SPLIT = RES["split"]["current"]
SC = REL["safety_case"]
FEED = json.loads((ROOT / "model" / "feed_results.json").read_text())
HAZ = json.loads((ROOT / "hazop" / "worksheet.json").read_text()) if (ROOT / "hazop" / "worksheet.json").exists() else None


def f(x, n=1):
    return f"{x:,.{n}f}"


def money(x, n=1):
    return f"{x:,.{n}f}"


def shade(cell, hex_color):
    tcPr = cell._tc.get_or_add_tcPr()
    shd = OxmlElement("w:shd")
    shd.set(qn("w:val"), "clear")
    shd.set(qn("w:color"), "auto")
    shd.set(qn("w:fill"), hex_color)
    tcPr.append(shd)


def set_cell(cell, text, bold=False, size=8.5, color=None, align=None):
    cell.text = ""
    p = cell.paragraphs[0]
    run = p.add_run(str(text))
    run.font.size = Pt(size)
    run.font.name = "Calibri"
    run.bold = bold
    if color is not None:
        run.font.color.rgb = color
    if align == "right":
        p.alignment = WD_ALIGN_PARAGRAPH.RIGHT
    elif align == "center":
        p.alignment = WD_ALIGN_PARAGRAPH.CENTER
    p.paragraph_format.space_after = Pt(1)
    p.paragraph_format.space_before = Pt(1)


def repeat_header(row):
    trPr = row._tr.get_or_add_trPr()
    tblHeader = OxmlElement("w:tblHeader")
    tblHeader.set(qn("w:val"), "true")
    trPr.append(tblHeader)


def add_table(doc, headers, rows, widths=None, caption=None, font_size=8.5, align_right=None, highlight_last=False):
    if caption:
        p = doc.add_paragraph()
        r = p.add_run(caption)
        r.font.size = Pt(8.5)
        r.bold = True
        r.font.color.rgb = NAVY
        p.paragraph_format.space_before = Pt(8)
        p.paragraph_format.space_after = Pt(3)
    table = doc.add_table(rows=1, cols=len(headers))
    table.style = "Table Grid"
    table.alignment = WD_TABLE_ALIGNMENT.CENTER
    table.autofit = False
    hdr = table.rows[0]
    for i, h in enumerate(headers):
        set_cell(hdr.cells[i], h, bold=True, size=font_size, color=RGBColor(0xFF, 0xFF, 0xFF))
        shade(hdr.cells[i], "1F3864")
        if widths:
            hdr.cells[i].width = Cm(widths[i])
    repeat_header(hdr)
    for ri, row in enumerate(rows):
        cells = table.add_row().cells
        for i, value in enumerate(row):
            is_num = isinstance(value, (int, float))
            set_cell(
                cells[i],
                f(value) if isinstance(value, float) else value,
                bold=highlight_last and ri == len(rows) - 1,
                size=font_size,
                align="right" if (align_right and i in align_right) or is_num else None,
            )
            if widths:
                cells[i].width = Cm(widths[i])
            if highlight_last and ri == len(rows) - 1:
                shade(cells[i], "E8F3EE")
    doc.add_paragraph().paragraph_format.space_after = Pt(2)
    return table


def add_figure(doc, path, width_cm, caption):
    p = doc.add_paragraph()
    p.alignment = WD_ALIGN_PARAGRAPH.CENTER
    p.paragraph_format.space_before = Pt(6)
    p.add_run().add_picture(str(path), width=Cm(width_cm))
    c = doc.add_paragraph()
    c.alignment = WD_ALIGN_PARAGRAPH.CENTER
    r = c.add_run(caption)
    r.font.size = Pt(8)
    r.italic = True
    r.font.color.rgb = GREY
    c.paragraph_format.space_after = Pt(8)


def para(doc, text, style=None, size=10, bold=False, italic=False, color=None, space_after=6, align=None):
    p = doc.add_paragraph(style=style)
    run = p.add_run(text)
    run.font.size = Pt(size)
    run.bold = bold
    run.italic = italic
    if color is not None:
        run.font.color.rgb = color
    p.paragraph_format.space_after = Pt(space_after)
    if align:
        p.alignment = align
    return p


def rich(doc, parts, space_after=6):
    p = doc.add_paragraph()
    for text, bold, italic in parts:
        r = p.add_run(text)
        r.bold = bold
        r.italic = italic
        r.font.size = Pt(10)
    p.paragraph_format.space_after = Pt(space_after)
    return p


def bullets(doc, items, style="List Bullet"):
    for item in items:
        p = doc.add_paragraph(item, style=style)
        p.paragraph_format.space_after = Pt(3)
        for r in p.runs:
            r.font.size = Pt(9.5)


def add_page_field(paragraph):
    run = paragraph.add_run()
    fld1 = OxmlElement("w:fldChar")
    fld1.set(qn("w:fldCharType"), "begin")
    instr = OxmlElement("w:instrText")
    instr.set(qn("xml:space"), "preserve")
    instr.text = "PAGE"
    fld2 = OxmlElement("w:fldChar")
    fld2.set(qn("w:fldCharType"), "end")
    run._r.append(fld1)
    run._r.append(instr)
    run._r.append(fld2)
    run.font.size = Pt(8)


def add_pages_field(paragraph):
    run = paragraph.add_run()
    fld1 = OxmlElement("w:fldChar")
    fld1.set(qn("w:fldCharType"), "begin")
    instr = OxmlElement("w:instrText")
    instr.set(qn("xml:space"), "preserve")
    instr.text = "NUMPAGES"
    fld2 = OxmlElement("w:fldChar")
    fld2.set(qn("w:fldCharType"), "end")
    run._r.append(fld1)
    run._r.append(instr)
    run._r.append(fld2)
    run.font.size = Pt(8)


def style_doc(doc):
    styles = doc.styles
    normal = styles["Normal"]
    normal.font.name = "Calibri"
    normal.font.size = Pt(10)
    normal.paragraph_format.space_after = Pt(6)
    normal.paragraph_format.line_spacing = 1.12
    styles["Heading 1"].paragraph_format.page_break_before = True
    for level, size, color in ((1, 15, NAVY), (2, 12, NAVY), (3, 10.5, NAVY)):
        st = styles[f"Heading {level}"]
        st.font.name = "Calibri"
        st.font.size = Pt(size)
        st.font.bold = True
        st.font.color.rgb = color
        st.paragraph_format.space_before = Pt(14 if level == 1 else 10)
        st.paragraph_format.space_after = Pt(5)
        st.paragraph_format.keep_with_next = True


def header_footer(doc):
    sec = doc.sections[0]
    hp = sec.header.paragraphs[0]
    hp.text = ""
    left = hp.add_run("AZ-FEED-08 · Rev B · Offshore debottlenecking FEED case study")
    left.font.size = Pt(7.5)
    left.font.color.rgb = GREY
    hp.alignment = WD_ALIGN_PARAGRAPH.RIGHT
    fp = sec.footer.paragraphs[0]
    fp.alignment = WD_ALIGN_PARAGRAPH.CENTER
    r = fp.add_run("Screening study — not for construction · azoth 0.1.4 · page ")
    r.font.size = Pt(7.5)
    r.font.color.rgb = GREY
    add_page_field(fp)
    r2 = fp.add_run(" of ")
    r2.font.size = Pt(7.5)
    r2.font.color.rgb = GREY
    add_pages_field(fp)


def cover(doc):
    for _ in range(2):
        doc.add_paragraph()
    p = doc.add_paragraph()
    r = p.add_run("FRONT-END ENGINEERING DESIGN")
    r.font.size = Pt(11)
    r.font.bold = True
    r.font.color.rgb = ORANGE
    p.paragraph_format.space_after = Pt(2)

    p = doc.add_paragraph()
    r = p.add_run("Offshore debottlenecking")
    r.font.size = Pt(30)
    r.bold = True
    r.font.color.rgb = NAVY
    p.paragraph_format.space_after = Pt(2)

    p = doc.add_paragraph()
    r = p.add_run("More condensate inside the flare limit")
    r.font.size = Pt(16)
    r.font.color.rgb = NAVY
    p.paragraph_format.space_after = Pt(18)

    p = doc.add_paragraph()
    r = p.add_run(
        "Platform 60/10/2, year ten · capacity diagnosis, debottlenecking scope, "
        "equipment and material specification, manhour and cost estimate, DEXPI process flow diagram, "
        "drawings, implementation plan and independent HAZOP"
    )
    r.font.size = Pt(11)
    r.font.color.rgb = GREY
    p.paragraph_format.space_after = Pt(30)

    table = doc.add_table(rows=0, cols=2)
    table.style = "Table Grid"
    rows = [
        ("Document number", "AZ-FEED-08-Report"),
        ("Revision", "B"),
        ("Date", "26 November 2026"),
        ("Status", "Issued for FEED — Rev B: materials, manhours, DEXPI and HAZOP added"),
        ("Prepared for", "azoth repository case study (illustrative asset)"),
        ("Basis", "notebook 08-offshore-debottlenecking, reproduced in model/case_model.py; FEED additions in model/feed_model.py"),
        ("Calculation library", f"azoth {RES['meta']['azoth_version']} ({', '.join(RES['meta']['backends'])})"),
        ("Verification", "All library results are partially_verified — screening values, not design deliverables"),
        ("Estimate class", "AACE Class 4/5 study allowances, no contingency"),
        ("Recommended package", "HIPPS + produced-water upgrade — 36 MUSD, 4-week shutdown"),
        ("DEXPI drawing", "AZ-FEED-08-DEXPI-PFD-001, DEXPI 2.0 Process exchange, schema-valid"),
        ("Independent HAZOP", "Completed on the frozen Rev B design; actions tracked in Section 10"),
        ("Result", f"{money(ECON['results']['HIPPS + water']['npv'], 1)} MUSD NPV at 10%, payback in year {ECON['results']['HIPPS + water']['payback']}"),
    ]
    for k, v in rows:
        cells = table.add_row().cells
        set_cell(cells[0], k, bold=True, size=9)
        set_cell(cells[1], v, size=9)
        cells[0].width = Cm(4.2)
        cells[1].width = Cm(12.2)
        shade(cells[0], "E8EDF6")

    doc.add_paragraph()
    p = doc.add_paragraph()
    r = p.add_run(
        "This is a case study built on illustrative field, history, cost and price data. "
        "It is not a real asset and none of its numbers is a design deliverable. Every azoth "
        "result carries the library's own partially_verified status and is used as screening evidence only."
    )
    r.font.size = Pt(8.5)
    r.italic = True
    r.font.color.rgb = ORANGE
    p.paragraph_format.space_before = Pt(12)


def contents(doc):
    h = doc.add_heading("Contents", level=1)
    h.paragraph_format.page_break_before = False
    items = [
        ("Executive summary", ""),
        ("1  Introduction and scope", ""),
        ("2  Basis of design", ""),
        ("3  Facility diagnosis", ""),
        ("4  Debottlenecking options and selection", ""),
        ("5  Recommended scope and equipment specification", ""),
        ("     5.8  Materials and corrosion protection", ""),
        ("     5.9  Datasheets", ""),
        ("6  Cost estimate, manhours and resources", ""),
        ("     6.6  Manhour estimate", ""),
        ("     6.7  Cost build-up", ""),
        ("7  Drawings, including the DEXPI PFD", ""),
        ("8  Implementation project plan", ""),
        ("     8.7  Resource-loaded plan", ""),
        ("9  Economics", ""),
        ("10  Independent HAZOP", ""),
        ("11  Risks, assumptions and verification", ""),
        ("12  Conclusions and recommendations", ""),
        ("Appendix A  Calculation verification register", ""),
        ("Appendix B  Equipment datasheets", ""),
        ("Appendix C  Cost estimate detail", ""),
        ("Appendix D  Project plan task list", ""),
        ("Appendix E  HAZOP worksheet", ""),
        ("Appendix F  References", ""),
    ]
    for name, _ in items:
        p = doc.add_paragraph()
        r = p.add_run(name)
        r.font.size = Pt(10)
        p.paragraph_format.space_after = Pt(3)
    doc.add_paragraph()
    p = doc.add_paragraph()
    r = p.add_run(
        "Deliverables: this report; DEXPI process flow diagram AZ-FEED-08-DEXPI-PFD-001 (XML with "
        "embedded drawing, A0 SVG/PDF); drawings AZ-FEED-08-BFD-001, AZ-FEED-08-PID-001 and "
        "AZ-FEED-08-SCH-001; workbook AZ-FEED-08-Workbook.xlsx (basis, capacity, equipment, "
        "datasheets, materials, manhours, resource plan, cost, economics, DEXPI, HAZOP, verification); "
        "and the reproducible calculation dossier in model/."
    )
    r.font.size = Pt(8.5)
    r.italic = True
    r.font.color.rgb = GREY


def executive_summary(doc):
    doc.add_heading("Executive summary", level=1)
    para(
        doc,
        "The 60/10/2 three-stage platform is ten years into production and the facility, not the "
        "reservoir, sets the rate. The wells can deliver about 163.4 MMscfd of gas equivalent; the "
        "facility passes 137.1 MMscfd. The first constraint is the HP separator at 99.8 % of its "
        "Souders-Brown gas load, the second the export compressor at 98.1 % of its 2.3 MW driver, "
        "and the third — the one a gas-side package would miss — the produced-water plant at 92.7 % "
        "of 3,000 bbl/d. The flare is at 91.1 % of its mass capacity and is the relief gate, not a "
        "production gate, until the gas rate rises.",
    )
    para(
        doc,
        "Seven fixes were screened and six packages valued over ten years. The recommended package "
        "is the HIPPS + water package: an export-compressor re-rate, HP separator internals, a SIL-3 "
        "HIPPS loop and a produced-water upgrade with coalescer. It fits the four-week shutdown, "
        "costs 36 MUSD, returns 24.8 MUSD after tax at a 10 % discount, pays back in year one, and "
        "raises the plateau from 137.1 to 161.8 MMscfd and condensate from 2,936 to about 3,464 bbl/d "
        "(+24.7 MMscfd and +528 bbl/d). The flare and header upgrade is the fallback at 42 MUSD and "
        "19.2 MUSD NPV.",
    )
    para(
        doc,
        "The package is a gas project with a condensate co-product. Of 98.2 MUSD of discounted "
        "revenue, 72.4 MUSD is gas and 25.8 MUSD is condensate; on gas alone the package is about "
        "−0.5 MUSD and on condensate alone −36.2 MUSD. The client's condensate-only objective does "
        "not pay at 70 USD/bbl. The recommendation is conditional on selling the incremental gas at "
        "the assumed 4 USD/MMBtu.",
    )

    add_table(
        doc,
        ["Headline", "Value", "Basis"],
        [
            ("Base rate", "137.1 MMscfd / 2,936 bbl/d", "Year-ten operating point"),
            ("Well potential", "163.4 MMscfd", "Deliverability, facility-limited"),
            ("Recommended package", "HIPPS + water", "Fixes 1 + 2 + 4 + 5"),
            ("Installed cost", "36.0 MUSD", "AACE Class 4/5 allowance"),
            ("NPV at 10 %, after tax", "24.8 MUSD", "Ten-year incremental DCF"),
            ("Payback", "Year 1", "Undiscounted cumulative"),
            ("Plateau after package", "161.8 MMscfd / ≈3,464 bbl/d", "+24.7 MMscfd / +528 bbl/d"),
            ("Shutdown", "4 weeks", "Inside the window"),
            ("Materials", "Carbon steel + 3.0 mm CA; 316L water", "CO2 pp 0.73 bar, 81 °C"),
            ("Manhours", "32,600 h", "Engineering 14,200; fabrication 9,600; offshore 8,800"),
            ("DEXPI PFD", "31 process steps, 32 streams", "Schema-valid DEXPI 2.0; A0 drawing"),
            ("Independent HAZOP", "Completed; actions tracked", "Section 10, Appendix E"),
            ("Fallback", "Flare/header upgrade, 42.0 MUSD", "NPV 19.2 MUSD"),
        ],
        widths=[5.0, 6.8, 5.0],
        caption="Table ES-1  Headline results",
        font_size=9,
    )
    para(
        doc,
        "The value is carried by gas and by fitting the shutdown, not by the condensate. A two-week "
        "slip gives back 11.0 MUSD, more than half the NPV; a drop in the gas price to zero turns the "
        "project to −36.2 MUSD. The fourth stage is close to neutral and is not recommended for this "
        "shutdown; the stabiliser fails because its two-week extension defers 11.0 MUSD of production.",
    )
    para(
        doc,
        "Rev B turns the screening study into a FEED package. It adds the material selection and "
        "corrosion allowance (carbon steel with 3.0 mm CA and continuous inhibition at 0.73 bar CO2), "
        "the pressure-containing thickness checks, a 32,600-manhour estimate across engineering, "
        "fabrication and offshore work, a resource-loaded 42-week plan, and a DEXPI 2.0 process flow "
        "diagram with a validated semantic model and drawing. The package was then reviewed by an "
        "independent HAZOP team; the worksheet and action close-out are in Section 10 and Appendix E.",
    )
    rich(
        doc,
        [
            ("What this report rests on. ", True, False),
            (
                "The field, history, costs and prices are illustrative. The separators are equilibrium "
                "stages, the compressor is isentropic with no interstage condensate, the water package "
                "is a stated capacity and cost rather than a hydrocyclone model, and the relief work is "
                "a screening PSV and flare calculation, not a relief study. The HIPPS 35 % relief "
                "reduction, the 4/6-week schedule rule, the water-treatment capacity and the brownfield "
                "costs are study assumptions. Every azoth result is partially_verified and used as "
                "screening evidence.",
                False,
                False,
            ),
        ]
    )


def introduction(doc):
    doc.add_heading("1  Introduction and scope", level=1)
    doc.add_heading("1.1  Purpose", level=2)
    para(
        doc,
        "This report is the Front-End Engineering Design (FEED) study for debottlenecking the "
        "60/10/2 offshore platform. It follows the diagnosis in notebook 08: the field is ten years "
        "into production, the wells make more gas and water and less condensate than the design case, "
        "and the client wants more condensate without exceeding the flare limit or operating outside "
        "the design envelope. The study identifies the capacity chain, sizes the relief and separator "
        "checks with the azoth library, screens seven fixes and six packages, values them with a "
        "ten-year incremental discounted cash-flow model, and recommends a scope that fits the next "
        "four-week shutdown.",
    )
    para(
        doc,
        "The report is one of four deliverables: the study itself, a drawing set (block flow diagram, "
        "simplified P&ID and schedule), a calculation workbook (basis, capacity, equipment, cost, "
        "economics, plan and verification), and this narrative. It is written as a case study for the "
        "azoth repository and doubles as a worked example of the library's process, safety and cost "
        "calculations.",
    )

    doc.add_heading("1.2  Scope and battery limits", level=2)
    bullets(
        doc,
        [
            "Facility scope: HP, MP and LP separation, export compression and cooling, produced-water "
            "treatment, condensate pumping, the HP relief path (PSV, flare header and tip) and the "
            "export gas and liquid lines.",
            "Time scope: the year-ten operating point with a ten-year production forecast; a single "
            "four-week shutdown window.",
            "Option scope: seven fixes and six packages; the fourth stage and stabiliser are carried "
            "as add-ons and priced, but not recommended.",
            "Excluded: reservoir simulation, flow assurance (hydrates, wax, slugging, sand), "
            "decommissioning, inflation and foreign exchange, detailed design, and any work outside "
            "the battery limits in Figure 7-1.",
        ],
    )

    doc.add_heading("1.3  Basis and limitations", level=2)
    para(
        doc,
        "The field, the production history, the costs and the prices are illustrative; no number here "
        "should be read as a real asset. The thermodynamic and equipment numbers are azoth library "
        "results, reproduced from notebook 08 by model/case_model.py. The library reports them as "
        "partially_verified — partly externally checked — so they are screening values and evidence "
        "for a finding, not design deliverables. Section 10.3 lists each calculation and its status.",
    )
    para(
        doc,
        "The cost estimate is AACE Class 4/5 in year-ten money, built from brownfield allowances "
        "rather than a greenfield module stack. It carries no contingency, escalation, owner's costs "
        "or detailed design, and its accuracy is correspondingly wide. Section 6 sets out the basis "
        "and exclusions.",
    )

    doc.add_heading("1.4  Report structure", level=2)
    para(
        doc,
        "Section 2 fixes the basis of design. Section 3 diagnoses the facility: the capacity chain, "
        "the relief case and the safety case. Section 4 screens the options and selects the package. "
        "Section 5 specifies the equipment. Section 6 estimates the cost. Section 7 presents the "
        "drawings. Section 8 is the implementation plan. Section 9 values the project. Section 10 "
        "collects the risks, assumptions and verification status, and Section 11 concludes. The "
        "appendices hold the calculation register, datasheets, cost detail, task list and references.",
    )


def basis_of_design(doc):
    doc.add_heading("2  Basis of design", level=1)
    doc.add_heading("2.1  The facility", level=2)
    para(
        doc,
        "The platform is the 60/10/2 three-stage manned facility from the field-development study: "
        "137 MMscfd of sales gas at design, about 6,100 bbl/d of stock-tank liquid, and no produced "
        "water. The actual history diverged from the design forecast. Pressure support held better "
        "than planned but the gas-oil ratio rose faster, so gas sits at the facility limit while "
        "condensate falls and water breaks through. The HP separator is the first stage of a "
        "three-phase retrofit: water is drawn off at the HP vessel, and the MP and LP stages are "
        "two-phase.",
    )
    para(
        doc,
        "At year ten the fluid is gassier. The C7+ fraction is lower and the condensate yield has "
        "fallen from 44 to about 21 bbl/MMscf. Produced water is 2,782 bbl/d against 2,936 bbl/d of "
        "condensate, a 48.7 % water cut. The feed to the HP separator is 2,230 mol/s, of which 13.45 "
        "mol % is water.",
    )
    add_table(
        doc,
        ["Phase", "Flow, mol/s", "Share of feed, %", "Note"],
        [
            ("Vapour", SPLIT["vapour_n"], SPLIT["vapour_n"] / SPLIT["total"] * 100, "To MP/HP compression"),
            ("Oil (light liquid)", SPLIT["light_liquid_n"], SPLIT["light_liquid_n"] / SPLIT["total"] * 100, "To MP separator"),
            ("Water (heavy liquid)", SPLIT["heavy_liquid_n"], SPLIT["heavy_liquid_n"] / SPLIT["total"] * 100, "To produced-water package"),
            ("Total", SPLIT["total"], 100.0, "2,230 mol/s at 60 bara, 354.15 K"),
        ],
        widths=[4.0, 3.4, 3.6, 5.4],
        caption="Table 2-1  HP separator phase split at the year-ten point",
        align_right=[1, 2],
        highlight_last=True,
    )

    doc.add_heading("2.2  Product slate and operating envelope", level=2)
    add_table(
        doc,
        ["Quantity", "Now", "At well potential", "Unit"],
        [
            ("Sales gas", T["gas_mmscfd"], TP["gas_mmscfd"], "MMscfd"),
            ("Stock-tank condensate", T["bpd"], TP["bpd"], "bbl/d"),
            ("Produced water", T["water_bpd"], TP["water_bpd"], "bbl/d"),
            ("Water cut", T["water_cut"] * 100, TP["water_cut"] * 100, "%"),
            ("Condensate yield", T["cgr"], TP["cgr"], "bbl/MMscf"),
            ("Compression", T["power_kw"], TP["power_kw"], "kW"),
            ("Cooler duty", T["duty_kw"], TP["duty_kw"], "kW"),
            ("HP separator gas load", T["hp_util"] * 100, TP["hp_util"] * 100, "%"),
            ("Oil retention", T["oil_retention_min"], TP["oil_retention_min"], "min"),
            ("Water retention", T["water_retention_min"], TP["water_retention_min"], "min"),
        ],
        widths=[5.4, 3.4, 4.2, 3.2],
        caption="Table 2-2  Year-ten product slate (azoth separator, compressor and flash calculations)",
        align_right=[1, 2],
    )
    para(
        doc,
        "The design envelope is fixed by the as-built equipment: 60/10/2 bara separation, 90 bara "
        "export, 313.15 K after cooling, a 2,300 kW compressor driver, a 3,000 bbl/d produced-water "
        "plant, a 6.38 in² P-orifice PSV set at 66 barg, and a flare with 46.34 kg/s design mass "
        "capacity. The HP separator is 1.96 m inside diameter by 7.26 m tangent-to-tangent, designed "
        "to a Souders-Brown K of 0.11 m/s.",
    )

    doc.add_heading("2.3  Codes and standards", level=2)
    add_table(
        doc,
        ["Area", "Standard", "Use in this study"],
        [
            ("Pressure relief", "API STD 520 Part I; API STD 521", "PSV sizing, relief load screening, flare capacity"),
            ("Flare", "API STD 537 (reference)", "Radiation distance at 4.73 kW/m²"),
            ("Separation", "API Spec 12J", "Three-phase separator sizing and retention"),
            ("SIS", "IEC 61508 / IEC 61511; NOG-070 (2020)", "HIPPS SIL determination and verification"),
            ("Secondary pressure protection", "NORSOK S-001:2020", "Pipeline two-barrier check, proof-test interval"),
            ("Gas measurement", "ISO 6976", "Calorific value and flare duty"),
            ("Cost estimate", "AACE 18R-97", "Class 4/5 accuracy statement"),
        ],
        widths=[3.2, 6.0, 7.4],
        caption="Table 2-3  Codes and standards",
    )
    para(
        doc,
        "The study applies these standards at screening level: PSV sizing through API 520, flare "
        "radiation through the point-source model, separator capacity through Souders-Brown, and the "
        "HIPPS through the NOG-070 minimum-SIL table and NORSOK S-001 criteria. None of these "
        "substitutes for the detailed relief, flare and SIL studies required before construction.",
    )

    doc.add_heading("2.4  Battery limits and interfaces", level=2)
    bullets(
        doc,
        [
            "Inlet: wellhead and test manifold at 95 bara, 363.15 K; the battery limit is the HP inlet "
            "isolation valve SDV-101.",
            "Outlet gas: export pipeline at 90 bara, 15,000 m to reception at 60 bara minimum.",
            "Outlet liquid: condensate line at 25 bara, 15,000 m to reception at 20 bara.",
            "Outlet water: treated-water disposal or reinjection; the package is sized for 6,000 bbl/d "
            "after the upgrade.",
            "Relief: the HP PSV discharges to the existing flare header and tip; the flare and header "
            "upgrade is inside the battery limit for the fallback case.",
            "Utilities, control system and electrical power are existing; the HIPPS logic solver "
            "interfaces to the existing ESD system.",
        ],
    )


def diagnosis(doc):
    doc.add_heading("3  Facility diagnosis", level=1)
    doc.add_heading("3.1  Process model and let-down", level=2)
    para(
        doc,
        "The HP oil is two-phase at 60 bara, so the throttling valve refuses it; the MP stage uses an "
        "isenthalpic flash at 10 bara followed by a separator. The MP liquid is single-phase at 10 "
        "bara, so the LP stage can use the valve. Each stage gas is cooled to 313.15 K and compressed "
        "to 90 bara in equal-ratio stages no larger than 2.5. The stock tank flashes the LP liquid at "
        "288.15 K and 1 atm; its flash gas joins the sales gas, and the liquid is the condensate "
        "product. The model reproduces the design gas rate at the year-ten point and gives the "
        "product slate used throughout this study.",
    )
    para(
        doc,
        f"The HP separator's interface settling is {f(T['settling_s'])} s against the class's "
        "one-minute check, so the produced-water package must include a coalescer. That is a "
        "performance requirement, not a capacity one, and it does not change the gas cap.",
    )

    doc.add_heading("3.2  The capacity chain", level=2)
    para(
        doc,
        "The first limit is not the compressor. At the year-ten point the HP separator is at 99.8 % of "
        "its Souders-Brown gas load, the export compression at 98.1 % of its driver, the produced-water "
        "plant at 92.7 % and the flare at 91.1 %. The gas trunk is at 76.2 % and the PSV at 73.5 %; the "
        "pump and the liquid line have ample headroom. The separator is the first constraint, the "
        "compressor the second, and the water plant the hidden third: a gas-side package that ignores "
        "the water plant cannot use the capacity it buys.",
    )
    add_table(
        doc,
        ["Constraint", "Utilisation, %", "Cap now, MMscfd", "Cap after scope, MMscfd", "Fix"],
        [
            ("HP separator gas load", CAP["utilisation"]["separator gas load"] * 100, CAP["caps"]["HP separator"], CAP["fixed_caps"]["HP separator"], "Internals, K 0.11 → 0.14"),
            ("Compressor power", CAP["utilisation"]["compressor power"] * 100, CAP["caps"]["Compressor"], CAP["fixed_caps"]["Compressor"], "Re-rate, +18 %"),
            ("Produced water", CAP["utilisation"]["produced water"] * 100, CAP["caps"]["Water plant"], CAP["fixed_caps"]["Water plant"], "Upgrade, 3,000 → 6,000 bbl/d"),
            ("Flare mass capacity", CAP["utilisation"]["flare"] * 100, CAP["caps"]["Flare"], CAP["fixed_caps"]["Flare"], "HIPPS (−35 %) or flare +25 %"),
            ("Gas line", CAP["utilisation"]["gas line"] * 100, CAP["caps"]["Gas line"], CAP["fixed_caps"]["Gas line"], "None"),
            ("PSV orifice", CAP["utilisation"]["PSV orifice"] * 100, CAP["caps"]["PSV"], CAP["fixed_caps"]["PSV"], "None"),
            ("Pump power", CAP["utilisation"]["pump power"] * 100, None, None, "None"),
            ("Liquid line", CAP["utilisation"]["liquid line"] * 100, None, None, "None"),
        ],
        widths=[4.4, 2.8, 2.8, 3.4, 3.4],
        caption="Table 3-1  The capacity chain at year ten",
        align_right=[1, 2, 3],
    )
    add_figure(doc, DRAW / "fig_capacity_chain.png", 16.0, "Figure 3-1  Utilisation of each design limit at the year-ten point")
    para(
        doc,
        "The cap table is the same fact in the other direction: the gas rate at which each constraint "
        "reaches its limit. The HP separator allows 137.4 MMscfd, the compressor 139.7, the water "
        "plant 147.8 and the flare 150.5; the gas line and PSV are beyond 175. The base case sits at "
        "137.1, so the separator margin is only 0.3 MMscfd — the facility is effectively at its "
        "separator limit before any growth.",
    )
    add_figure(doc, DRAW / "fig_capability.png", 16.0, "Figure 3-2  What each fix buys, and the water plant as the hidden link")

    doc.add_heading("3.3  The relief case", level=2)
    para(
        doc,
        "The governing relief case is the HP separator blocked outlet: the full HP vapour inflow, "
        "flashed at the relieving pressure, 42.63 kg/s. The fire case is 0.54 kg/s, so the blocked "
        "outlet governs by nearly two orders of magnitude. The installed P orifice has 6.38 in² "
        "against a required 4.69 in², 73.5 % of its capacity; at +18 % gas it needs 5.53 in² and is "
        "still a P orifice at 86.7 %. The PSV is not the gate.",
    )
    para(
        doc,
        "The flare is. The existing flare is at 91.1 % of its mass capacity; a +18 % gas rate takes it "
        "to 107.5 % and overloads it. A 25 % flare and header upgrade takes the same load to 86.0 %; a "
        "HIPPS that cuts the blocked-outlet case by 35 % takes it to 69.9 %. The radiation distance to "
        "4.73 kW/m² rises from 75.0 m to 82.4 m at +18 %, and falls to 64.0 m with the HIPPS. The "
        "12 MUSD relief fix is therefore a flare and header upgrade, not a PSV change.",
    )
    add_table(
        doc,
        ["Case", "Relief, kg/s", "PSV required, in²", "PSV use, %", "Flare use, %", "Overloaded", "Radiation, m"],
        [
            ("Current", REL["current"]["relief_kg_s"], REL["current"]["psv_required_in2"], REL["current"]["psv_utilisation"] * 100, REL["current"]["flare_mass_utilisation"] * 100, "No", REL["current"]["radiation_distance_m"]),
            ("+18 % gas", REL["plus_18"]["relief_kg_s"], REL["plus_18"]["psv_required_in2"], REL["plus_18"]["psv_utilisation"] * 100, REL["plus_18"]["flare_mass_utilisation"] * 100, "Yes", REL["plus_18"]["radiation_distance_m"]),
            ("+18 %, flare +25 %", REL["plus_18_flare25"]["relief_kg_s"], REL["plus_18_flare25"]["psv_required_in2"], REL["plus_18"]["psv_utilisation"] * 100, REL["plus_18_flare25"]["flare_mass_utilisation"] * 100, "No", REL["plus_18_flare25"]["radiation_distance_m"]),
            ("+18 %, HIPPS −35 %", REL["plus_18_hipps"]["relief_kg_s"], REL["plus_18_hipps"]["psv_required_in2"], REL["plus_18_hipps"]["psv_utilisation"] * 100, REL["plus_18_hipps"]["flare_mass_utilisation"] * 100, "No", REL["plus_18_hipps"]["radiation_distance_m"]),
        ],
        widths=[3.4, 2.4, 2.6, 2.0, 2.0, 2.0, 2.4],
        caption="Table 3-2  Relief cases at the HP separator (API 520 screening, 4.73 kW/m² contour)",
        align_right=[1, 2, 3, 4, 6],
    )
    add_figure(doc, DRAW / "fig_relief.png", 15.5, "Figure 3-3  Flare and PSV utilisation across the relief cases")

    doc.add_heading("3.4  The safety case for HIPPS", level=2)
    para(
        doc,
        "The HIPPS is a real alternative to the flare upgrade. The SIL-3 function is verifiable against "
        "NOG-070 and NORSOK S-001, and the two-barrier pipeline check is satisfied. NOG-070 gives a "
        "minimum SIL of 3 for a HIPPS pipeline SIF; a PFD of 1e-4 with 1oo2 voting achieves SIL 3 and "
        "complies. The pipeline's maximum incidental pressure is 72.6 bara against a 66 barg design, "
        "which requires two barriers; the study finds the protection adequate. NORSOK S-001 secondary "
        "pressure protection is acceptable with a 12-month proof test, relief leakage assessed and "
        "routed to a safe location.",
    )
    add_table(
        doc,
        ["Check", "Result", "Verdict"],
        [
            ("NOG-070 minimum SIL (HIPPS pipeline)", f"{SC['nog070_minimum_sil']:.0f}", "Catalogue"),
            ("NOG-070 achieved SIL", f"{SC['nog070_achieved_sil']:.0f}", "Compliant" if SC["nog070_compliant"] else "Not compliant"),
            ("SIL verification, 1oo2, PFD 1e-4", f"{SC['sil_achieved']:.0f}", "Passed" if SC["sil_passed"] else "Failed"),
            ("Pipeline maximum incidental pressure", f"{SC['pipeline_max_incidental_bar']:.1f} bara", SC["pipeline_verdict"]),
            ("NORSOK S-001 secondary protection", "Acceptable" if SC["s001_acceptable"] else "Not acceptable", "Two barriers required and provided"),
        ],
        widths=[7.8, 4.4, 4.6],
        caption="Table 3-3  Safety-case checks (azoth process safety calculations)",
    )
    para(
        doc,
        "The 35 % relief-case reduction is the study's assumption, not a library output: it depends on "
        "the HIPPS closing fast enough and on the relief path downstream of the isolation valve. The "
        "safety case and the dynamic relief study are non-shutdown activities and a schedule risk; "
        "without them the HIPPS credit cannot be booked and the flare upgrade is the fallback.",
    )


def options(doc):
    doc.add_heading("4  Debottlenecking options and selection", level=1)
    doc.add_heading("4.1  Candidate fixes", level=2)
    para(
        doc,
        "Seven fixes are screened. The first five are enablers: the compressor re-rate, the separator "
        "internals, the relief upgrade or HIPPS, and the produced-water upgrade. The last two add "
        "condensate directly: a fourth separation stage with a vapour-recovery compressor, and a "
        "stabiliser.",
    )
    add_table(
        doc,
        ["ID", "Fix", "Effect", "Capex, MUSD", "Weeks", "Role"],
        [
            (1, "Compressor re-rate", "+18 % gas capacity", 18.0, 3.0, "Enabler"),
            (2, "Separator internals", "K 0.11 → 0.14 m/s", 4.0, 3.0, "Enabler"),
            (3, "Flare/header upgrade", "+25 % relief capacity", 12.0, 4.0, "Relief enabler (fallback)"),
            (4, "HIPPS SIL-3", "−35 % blocked-outlet case", 6.0, 2.0, "Relief enabler (recommended)"),
            (5, "Produced-water upgrade", "3,000 → 6,000 bbl/d + coalescer", 8.0, 4.0, "Hidden enabler"),
            (6, "Fourth stage + VRU", "+150 bbl/d condensate", 22.0, 5.0, "Add-on (deferred)"),
            (7, "Stabiliser", "+200 bbl/d condensate", 35.0, 8.0, "Add-on (rejected)"),
        ],
        widths=[1.1, 4.2, 4.6, 2.6, 1.8, 2.5],
        caption="Table 4-1  Candidate fixes and study allowances",
        align_right=[3, 4],
    )

    doc.add_heading("4.2  Packages and screening", level=2)
    add_table(
        doc,
        ["Package", "Fixes", "Capex, MUSD", "Weeks", "Gas cap, MMscfd", "Condensate add, bbl/d", "NPV, MUSD"],
        [
            ("Gas side", "1+2+3", 34.0, 4.0, CAP["gas_side_cap"], 0.0, ECON["results"]["Gas side"]["npv"]),
            ("Gas side + water", "1+2+3+5", 42.0, 4.0, CAP["package_cap"], 0.0, ECON["results"]["Gas side + water"]["npv"]),
            ("HIPPS + water", "1+2+4+5", 36.0, 4.0, CAP["hipps_cap"], 0.0, ECON["results"]["HIPPS + water"]["npv"]),
            ("+ fourth stage", "1+2+3+5+6", 67.3, 5.0, CAP["package_cap"], 150.0, ECON["results"]["+ fourth stage"]["npv"]),
            ("+ stabiliser", "1+2+3+5+7", 77.0, 8.0, CAP["package_cap"], 200.0, ECON["results"]["+ stabiliser"]["npv"]),
            ("Both add-ons", "1+2+3+5+6+7", 99.0, 8.0, CAP["package_cap"], 350.0, ECON["results"]["Both add-ons"]["npv"]),
        ],
        widths=[3.2, 2.4, 2.2, 1.6, 2.8, 2.8, 1.8],
        caption="Table 4-2  Packages screened (NPV at 10 %, after tax)",
        align_right=[2, 3, 4, 5, 6],
    )
    para(
        doc,
        "The gas side alone is slightly negative: the water plant caps it at 147.8 MMscfd and strands "
        "the compressor and flare capacity it buys. The water upgrade is the enabler: the same gas side "
        "with the water plant reaches 161.8 MMscfd and turns the project positive. The HIPPS variant is "
        f"the best of the core packages, 6 MUSD cheaper than the flare upgrade at the same four weeks, "
        f"and {money(ECON['results']['HIPPS + water']['npv'] - ECON['results']['Gas side + water']['npv'])} "
        "MUSD better in NPV. The fourth stage is close to neutral; the stabiliser is destroyed by its "
        "two-week extension, which defers 11.0 MUSD of production.",
    )
    para(
        doc,
        "The cap table explains the ranking. The separator internals lift the separator cap from 137.4 "
        "to 174.9 MMscfd, the compressor re-rate lifts its cap from 139.7 to 161.8, the flare upgrade "
        "lifts the flare cap from 150.5 to 188.1, and the HIPPS lifts it to about 232. The water plant "
        "alone would allow 295.6. The binding constraint after the package is the compressor at 161.8, "
        "just below the 163.4 MMscfd well potential.",
    )

    doc.add_heading("4.3  Selection and fallback", level=2)
    para(
        doc,
        "The recommended package is the HIPPS + water package: the compressor re-rate, the separator "
        "internals, the HIPPS SIL-3 loop and the produced-water upgrade with coalescer. The flare and "
        "header upgrade is the fallback if the SIL-3 safety case is not approved. Both fit the "
        "four-week shutdown; the fallback costs 6 MUSD more and returns 5.6 MUSD less NPV, but it does "
        "not depend on the 35 % relief credit.",
    )
    para(
        doc,
        "The fourth stage is not recommended for this shutdown. Its 5-week duration forces compression "
        "into the window with a 3.3 MUSD labour premium, and its NPV of 15.9 MUSD is below the core "
        "package. It should be revisited at a higher condensate price or in the following shutdown. "
        "The stabiliser is rejected: 8 weeks extends the shutdown by two weeks and defers 11.0 MUSD, "
        "leaving 2.6 MUSD NPV.",
    )
    para(
        doc,
        "Neither product alone carries the capital. On gas alone the package is −0.5 MUSD; on "
        "condensate alone it is −36.2 MUSD. That is the study's most important commercial finding: the "
        "client's condensate-only objective does not pay at 70 USD/bbl, and the recommendation is "
        "conditional on selling the incremental gas.",
    )


def scope(doc):
    doc.add_heading("5  Recommended scope and equipment specification", level=1)
    doc.add_heading("5.1  Scope summary", level=2)
    para(
        doc,
        "The recommended scope is four work packages executed in one four-week shutdown, with the "
        "flare/header upgrade held as the relief fallback. The packages are independent enough to run "
        "in parallel but share isolation, scaffolding and commissioning resources.",
    )
    add_table(
        doc,
        ["WBS", "Work package", "Content", "Duration", "Cost, MUSD"],
        [
            ("1.1", "Compressor re-rate", "K-101 impeller/bundle and driver re-rate to +18 %", "3 weeks", 18.0),
            ("1.2", "Separator internals", "V-101 inlet device and vane pack, K 0.11 → 0.14 m/s", "3 weeks", 4.0),
            ("1.3", "HIPPS SIL-3", "2 × PT, logic solver, SDV-101, loop test", "2 weeks", 6.0),
            ("1.4", "Produced-water upgrade", "Hydrocyclone, coalescer, P-102; 3,000 → 6,000 bbl/d", "4 weeks", 8.0),
            ("1.5", "Relief fallback", "Flare tip and header +25 % (only if HIPPS not approved)", "4 weeks", 12.0),
        ],
        widths=[1.2, 4.0, 6.6, 2.2, 2.6],
        caption="Table 5-1  Recommended scope and fallback",
        align_right=[4],
    )

    doc.add_heading("5.2  V-101 — HP separator internals", level=2)
    para(
        doc,
        "The HP separator is a 1.96 m × 7.26 m horizontal three-phase vessel at 60 bara and 354.15 K. "
        "At the year-ten point the gas load is 99.8 % of the Souders-Brown capacity at K = 0.11 m/s, "
        "with oil retention of 10.92 min (above the 9 min minimum) and water retention of 7.83 min "
        "(above 6.5 min). The interface settling time is 17.09 s against the class's one-minute check, "
        "so the vessel cannot meet the produced-water duty without a coalescer; that is a performance "
        "requirement carried by the water package, not a capacity fix.",
    )
    para(
        doc,
        "The internals change is specified to raise the Souders-Brown K from 0.11 to 0.14 m/s, lifting "
        "the separator cap from 137.4 to 174.9 MMscfd. The scope is an inlet device, a vane pack or "
        "cyclonic mist eliminator and a revised weir/interface control; the vessel, nozzles and design "
        "pressure (66 barg) are unchanged. The vendor must guarantee the K factor and the separation "
        "efficiency at the year-ten and potential compositions, and the coalescer duty is a separate "
        "performance guarantee.",
    )
    add_table(
        doc,
        ["Parameter", "Value", "Unit", "Basis"],
        [
            ("Design pressure / temperature", "66 / 354.15", "barg / K", "As-built"),
            ("Inside diameter × T/T length", "1.96 × 7.26", "m", "As-built"),
            ("Feed", "2,230", "mol/s", "Year ten, 13.45 mol % water"),
            ("Gas load now", "99.77", "%", "azoth separator_capacity_assessment"),
            ("Souders-Brown K, now / after", "0.11 / 0.14", "m/s", "After is vendor expectation"),
            ("Gas cap now / after", "137.4 / 174.9", "MMscfd", "Study derivation"),
            ("Oil / water retention", "10.92 / 7.83", "min", "Above 9.0 / 6.5 min"),
            ("Interface settling", "17.09", "s", "Insufficient — coalescer required"),
        ],
        widths=[5.4, 3.4, 2.8, 4.6],
        caption="Table 5-2  V-101 specification summary",
        align_right=[1],
    )

    doc.add_heading("5.3  K-101 — export compressor re-rate", level=2)
    para(
        doc,
        "The export compressor lifts HP, MP and LP gas to 90 bara in equal-ratio stages of no more than "
        "2.5, with interstage cooling to 313.15 K. At the year-ten point it draws 2,256 kW against a "
        "2,300 kW driver (98.1 %), and at the well potential it would draw 2,689 kW (116.9 %). The "
        "re-rate is specified to deliver +18 % capacity, lifting the compressor cap from 139.7 to "
        "161.8 MMscfd. The three trains are staged 1 + 3 + 5 (HP, MP, LP) at equal ratio. The FEED "
        "scope is a vendor study and kit: revised impellers, a confirmed 2.7 MW driver, a revised "
        "compressor curve with anti-surge recycle and an independent overspeed trip, and the cooler "
        "duty increase from 3,885 to 4,630 kW. Torsional, lateral, impeller-stress and loss-of-utilities "
        "cases remain conditions on the vendor (HAZ-09).",
    )
    para(
        doc,
        "The compressor is the long-lead item and the schedule's critical path. The study assumes a "
        "22-week order-to-delivery for the re-rate kit; the implementation plan places the purchase "
        "order at week 8 to protect the week-32 shutdown. A slip beyond week 8 moves the shutdown, and "
        "a two-week slip costs 11.0 MUSD of deferred production.",
    )

    doc.add_heading("5.4  HIPPS — SIL-3 high-pressure protection", level=2)
    para(
        doc,
        "The HIPPS protects the HP inlet and the 66 barg pipeline from the 95 bara wellhead pressure. "
        "The study specifies two pressure transmitters in 1oo2 voting, a SIL-3 logic solver and a "
        "shutdown valve (SDV-101) on the HP inlet. The function achieves SIL 3 at PFD 1e-4 with a "
        "12-month proof test; hardware fault tolerance is 1 and diagnostic coverage 0.6. The pipeline "
        "maximum incidental pressure is 72.6 bara, which requires two barriers; NORSOK S-001 secondary "
        "pressure protection is acceptable with relief leakage assessed and routed to a safe location.",
    )
    para(
        doc,
        "The relief credit is 35 % of the blocked-outlet case, taking 50.31 kg/s to 32.70 kg/s, the "
        "flare to 69.9 % and the PSV requirement to 3.60 in². That credit is an assumption. It must be "
        "confirmed by a dynamic relief study and a SIL verification against the actual device failure "
        "rates before it is booked; if it is not, the flare/header upgrade is the fallback and the "
        "package still fits the shutdown. Proof testing and any bypass are restricted to the shutdown, "
        "with a rate cap and independent approval; a live 12-month proof test at full rate is outside "
        "the relief basis because the +18 % case reaches 107.5 % of the existing flare (HAZ-03).",
    )

    doc.add_heading("5.5  Produced-water package", level=2)
    para(
        doc,
        "The existing plant is rated 3,000 bbl/d and runs at 2,782 bbl/d (92.7 %). The upgrade doubles "
        "the capacity to 6,000 bbl/d and adds a coalescer to meet the interface-settling duty. The "
        "package comprises a hydrocyclone (V-105), a coalescer (V-106, 1.2 m ID x 4.0 m T/T, 4.3 m3, "
        "6.5 min retention), two 50 % treated-water pumps (P-102, 20 m3/h each, 15 kW, one standby), "
        "an oil-in-water analyser with diversion, and PSV-104. The water plant then stops binding at "
        "147.8 MMscfd and allows the compressor cap of 161.8 to be reached.",
    )
    para(
        doc,
        "The 17.09 s interface settling is the reason the coalescer is mandatory rather than optional. "
        "The HAZOP caught that the first datasheet sized P-102 at the current 2,782 bbl/d rather than "
        "the 6,000 bbl/d capacity; Rev B corrects the duty to 40 m3/h total. The package's performance "
        "guarantee should be written on outlet oil-in-water and on the 6,000 bbl/d capacity at the "
        "design water cut, not on the vessel size alone.",
    )

    doc.add_heading("5.6  Flare and header upgrade — fallback", level=2)
    para(
        doc,
        "The fallback is a 25 % flare and header upgrade: a new tip and a larger header to take the "
        "+18 % relief case from 107.5 % to 86.0 % of capacity. It costs 12 MUSD and takes four weeks, "
        "the same window as the water package, so it does not extend the shutdown. The radiation "
        "distance at +18 % is 82.4 m to 4.73 kW/m², against 75.0 m now; the study does not verify the "
        "platform's sterile area against that contour, and the detailed flare study must.",
    )

    doc.add_heading("5.7  Equipment list", level=2)
    add_table(
        doc,
        ["Tag", "Service", "Type", "Design P, barg", "Key size / duty", "Scope"],
        [
            ("V-101", "HP 3-phase separator", "Horizontal vessel", "66", "Ø1.96 × 7.26 m; K 0.11 → 0.14", "Modify internals"),
            ("K-101", "Export compressor", "Centrifugal, 3 trains (1+3+5 stages)", "100", "2.26 MW now; 2.69 MW potential", "Re-rate +18 %"),
            ("PSV-101", "HP relief", "PSV, P orifice", "66", "6.38 in² fitted; 4.69 in² required", "No change"),
            ("SDV-101", "HP inlet isolation", "SIL-3 shutdown valve", "100", "1oo2 initiators", "New HIPPS"),
            ("V-105", "Hydrocyclone", "Package vessel", "25", "6,000 bbl/d", "New"),
            ("V-106", "Coalescer", "Package vessel", "25", "6.5 min retention; 4.0 m T/T", "New"),
            ("PSV-102", "MP let-down relief", "PSV, V-102", "12", "Gas blowby on VLV-102 failure", "New (HAZOP)"),
            ("PSV-103", "LP let-down relief", "PSV, V-103", "4", "Gas blowby on VLV-103 failure", "New (HAZOP)"),
            ("PSV-104", "Water package relief", "PSV, V-106", "25", "LCV-102 fail-open", "New (HAZOP)"),
            ("P-102", "Treated-water pump (2 × 50 %)", "Centrifugal", "25", "20 m³/h each; 15 kW", "New"),
            ("P-101", "Condensate pump", "Centrifugal", "30", "20.4 kW; 42 kW driver", "No change"),
            ("Flare", "Flare tip and header", "Existing system", "10", "46.3 kg/s design; 91.1 % loaded", "Fallback +25 %"),
        ],
        widths=[1.4, 3.2, 3.2, 2.2, 4.6, 2.2],
        caption="Table 5-3  Equipment list for the recommended scope",
    )
    doc.add_heading("5.8  Materials and corrosion protection", level=2)
    co2 = FEED["co2"]
    para(
        doc,
        f"The HP vapour carries {co2['co2_mol_pct_vapour']:.2f} mol % CO2 at {B['p_hp_bar']:.0f} bara, a "
        f"partial pressure of {co2['p_co2_vapour_bar']:.2f} bar, at {co2['temperature_c']:.0f} °C. That is "
        "sweet service below 1 bar and below 100 °C, so carbon steel with a corrosion allowance and "
        "continuous inhibition is the economic selection. The allowance is the inhibited rate times the "
        f"design life: {co2['inhibited_rate_mm_yr']:.2f} mm/yr × {co2['design_life_years']:.0f} years = "
        f"{co2['ca_required_mm']:.1f} mm; 3.0 mm is selected. Uninhibited, the same basis would need "
        f"{co2['uninhibited_ca_mm']:.0f} mm, which is why inhibition is treated as a barrier with "
        "monitoring, not as an optional chemical programme.",
    )
    add_table(
        doc,
        ["Corrosion basis", "Value", "Unit", "Note"],
        [
            ("CO2 in HP vapour", f"{co2['co2_mol_pct_vapour']:.2f}", "mol %", "azoth three-phase separator"),
            ("CO2 partial pressure", f"{co2['p_co2_vapour_bar']:.2f}", "bar", "z_CO2 × 60 bara"),
            ("HP temperature", f"{co2['temperature_c']:.0f}", "°C", "354.15 K"),
            ("Inhibited rate (screening)", f"{co2['inhibited_rate_mm_yr']:.2f}", "mm/yr", "continuous inhibition assumed"),
            ("Uninhibited rate (screening)", f"{co2['uninhibited_rate_mm_yr']:.2f}", "mm/yr", "shows the inhibition dependency"),
            ("Design life", f"{co2['design_life_years']:.0f}", "years", "study basis"),
            ("CA required / selected", f"{co2['ca_required_mm']:.1f} / 3.0", "mm", "carbon steel, wet HC service"),
        ],
        widths=[5.6, 3.2, 2.4, 5.2],
        caption="Table 5-4  CO2 service and corrosion allowance",
        font_size=8.5,
    )
    para(
        doc,
        "The pressure-containing checks use ASME VIII-1 UG-27 for the V-101 shell and ASME B31.3 for "
        "the lines. The shell needs "
        f"{FEED['mechanical']['v101']['t_with_ca_mm']:.1f} mm including allowance and is retained at the "
        f"existing {FEED['mechanical']['v101']['t_selected_mm']:.0f} mm nominal. The 12-inch export line "
        f"needs {FEED['mechanical']['gas_line']['t_with_ca_mm']:.1f} mm and is specified at 12.7 mm; the "
        "produced-water and HIPPS lines are 316L with no allowance because corrosion product would foul "
        "the water treatment and the SIL loop.",
    )
    add_table(
        doc,
        ["Item", "t required, mm", "CA, mm", "t incl. CA, mm", "Selected, mm"],
        [
            ("V-101 shell (VIII-1 UG-27)", f"{FEED['mechanical']['v101']['t_required_mm']:.1f}", "3.0", f"{FEED['mechanical']['v101']['t_with_ca_mm']:.1f}", f"{FEED['mechanical']['v101']['t_selected_mm']:.0f}"),
            ("Export gas line 12 in (B31.3)", f"{FEED['mechanical']['gas_line']['t_required_mm']:.1f}", "3.0", f"{FEED['mechanical']['gas_line']['t_with_ca_mm']:.1f}", "12.7"),
            ("Condensate line 6 in (B31.3)", f"{FEED['mechanical']['condensate_line']['t_required_mm']:.1f}", "3.0", f"{FEED['mechanical']['condensate_line']['t_with_ca_mm']:.1f}", "7.1"),
            ("Produced-water line 8 in (B31.3)", f"{FEED['mechanical']['water_line']['t_required_mm']:.1f}", "0.0", f"{FEED['mechanical']['water_line']['t_with_ca_mm']:.1f}", "3.76"),
            ("HIPPS spool 10 in (B31.3)", f"{FEED['mechanical']['hipps_spool']['t_required_mm']:.1f}", "0.0", f"{FEED['mechanical']['hipps_spool']['t_with_ca_mm']:.1f}", "12.7"),
        ],
        widths=[6.2, 2.4, 1.6, 2.6, 2.6],
        caption="Table 5-5  Pressure-containing thickness checks",
        font_size=8.5,
    )
    add_table(
        doc,
        ["Item", "Material", "Specification", "CA, mm"],
        [(m["item"], m["material"], m["spec"], f"{m['ca_mm']:.1f}") for m in FEED["materials"]["schedule"]],
        widths=[5.6, 4.6, 4.2, 1.8],
        caption="Table 5-6  Material schedule",
        font_size=7.8,
    )
    add_table(
        doc,
        ["Class", "Service", "Size, in", "Material", "Rating", "CA, mm"],
        [(l["class"], l["service"], l["size_in"], l["material"], l["rating"], f"{l['ca_mm']:.1f}") for l in FEED["materials"]["line_classes"]],
        widths=[1.4, 4.4, 1.6, 2.6, 2.4, 1.6],
        caption="Table 5-7  Line classes",
        font_size=8.5,
    )

    doc.add_heading("5.9  Datasheets", level=2)
    para(
        doc,
        "Each package has a study-level datasheet in Appendix B: design pressure and temperature, size, "
        "material and corrosion allowance, internals, nozzles, weight, insulation, code and scope. The "
        "datasheets are the bridge between this report and a vendor enquiry; they are not yet "
        "purchase-order specifications.",
    )
    add_table(
        doc,
        ["Tag", "Service", "Type", "Design P/T", "Material", "CA", "Scope"],
        [(d["tag"], d["service"], d["type"], f"{d['design_p']} / {d['design_t']}", d["material"], d["ca"], d["scope"]) for d in FEED["datasheets"]],
        widths=[1.2, 3.6, 3.0, 1.8, 2.8, 1.2, 1.8],
        caption="Table 5-8  Datasheet register",
        font_size=7.4,
    )

    doc.add_heading("5.10  Flash gas, dehydration and let-down protection", level=2)
    para(
        doc,
        "Stock-tank flash gas is routed to the fuel-gas system, not the 90 bara sales header; the "
        "notebook's mass balance added it to sales gas, a simplification corrected in the DEXPI PFD. "
        "At the year-ten composition the flash is liquid-only at 15 °C and 1.013 bara, so the fuel-gas "
        "stream is small; the routing still matters because a future gassier stock tank must not "
        "back-pressure the sales header.",
    )
    para(
        doc,
        "The compression and cooling train delivers water-saturated gas at 90 bara and 313.15 K. The "
        "existing TEG dehydration unit is outside the debottlenecking battery limits and is assumed to "
        "be adequate, but the higher rate (137.1 to 161.8 MMscfd) and a 3 lb/MMscf pipeline specification "
        "have not been demonstrated. This is an open HAZOP action, not a closed assumption. The same "
        "review adds PSV-102 on V-102 and PSV-103 on V-103 for let-down valve fail-open, independent "
        "1oo2 LAHH/LSHH on V-101 to ESD, and fail-closed action on LCV-102. Rev B also adds the ESD "
        "hierarchy (ESD-1/2/3), fire and gas detection at the HP separator, compression and water "
        "package, blowdown on V-101 and the HP inlet spool, and ESDV/SSIV scope on the export lines; "
        "the cause-and-effect matrix is a detailed-design deliverable (HAZ-10).",
    )


def cost(doc):
    doc.add_heading("6  Cost estimate", level=1)
    doc.add_heading("6.1  Basis and class", level=2)
    para(
        doc,
        "The estimate is a brownfield allowance estimate in year-ten US dollars, prepared to AACE "
        "Class 4/5. The allowances cover engineering, procurement, fabrication, offshore installation "
        "labour and commissioning for each fix. They are not the library's greenfield module stack: "
        "that stack prices an HP separator and a new 2.27 MW compressor at 61.7 MUSD for the two "
        "modules alone, which is not what a re-rate, an internals change, a flare tip and a water "
        "package cost. The allowances are AACE class 4/5 study numbers with the wide accuracy that "
        "implies.",
    )

    doc.add_heading("6.2  Fix and package cost", level=2)
    add_table(
        doc,
        ["ID", "Fix", "Capex, MUSD", "Weeks"],
        [
            (1, "Compressor re-rate", 18.0, 3.0),
            (2, "Separator internals", 4.0, 3.0),
            (3, "Flare/header upgrade", 12.0, 4.0),
            (4, "HIPPS SIL-3", 6.0, 2.0),
            (5, "Produced-water upgrade", 8.0, 4.0),
            (6, "Fourth stage + VRU", 22.0, 5.0),
            (7, "Stabiliser", 35.0, 8.0),
        ],
        widths=[1.2, 8.0, 3.6, 3.0],
        caption="Table 6-1  Fix allowances",
        align_right=[2, 3],
    )
    add_table(
        doc,
        ["Package", "Fixes", "Capex, MUSD", "Weeks", "NPV, MUSD"],
        [
            ("Gas side", "1+2+3", 34.0, 4.0, ECON["results"]["Gas side"]["npv"]),
            ("Gas side + water", "1+2+3+5", 42.0, 4.0, ECON["results"]["Gas side + water"]["npv"]),
            ("HIPPS + water (recommended)", "1+2+4+5", 36.0, 4.0, ECON["results"]["HIPPS + water"]["npv"]),
            ("+ fourth stage", "1+2+3+5+6", 67.3, 5.0, ECON["results"]["+ fourth stage"]["npv"]),
            ("+ stabiliser", "1+2+3+5+7", 77.0, 8.0, ECON["results"]["+ stabiliser"]["npv"]),
            ("Both add-ons", "1+2+3+5+6+7", 99.0, 8.0, ECON["results"]["Both add-ons"]["npv"]),
        ],
        widths=[5.2, 3.2, 3.0, 2.2, 3.0],
        caption="Table 6-2  Package costs and NPV",
        align_right=[2, 3, 4],
    )

    doc.add_heading("6.3  Recommended package build-up", level=2)
    add_table(
        doc,
        ["Item", "MUSD"],
        [
            ("Compressor re-rate (Fix 1)", 18.0),
            ("HP separator internals (Fix 2)", 4.0),
            ("HIPPS SIL-3 (Fix 4)", 6.0),
            ("Produced-water upgrade and coalescer (Fix 5)", 8.0),
            ("Total installed cost", 36.0),
        ],
        widths=[12.0, 4.0],
        caption="Table 6-3  Recommended package cost build-up",
        align_right=[1],
        highlight_last=True,
    )
    para(
        doc,
        "The flare/header fallback is 42.0 MUSD (Fixes 1, 2, 3 and 5). The fourth stage adds 31.3 MUSD "
        "to 67.3 MUSD, and the stabiliser adds 41.0 MUSD to 77.0 MUSD. The fourth stage also carries a "
        "3.3 MUSD labour premium when compressed into the four-week window; the stabiliser's deferment "
        "is 11.0 MUSD of production, not capex.",
    )

    doc.add_heading("6.4  Exclusions and accuracy", level=2)
    bullets(
        doc,
        [
            "Excluded: detailed design, vendor taxes and duties, freight and marine spread, owner's "
            "costs, spare parts, escalation, decommissioning and financing.",
            "No contingency is applied. AACE Class 4/5 accuracy is wide — of the order of −30 %/+50 % "
            "for Class 4 — and the estimate should be narrowed before sanction.",
            "The estimate assumes a single four-week shutdown with no overrun; a two-week overrun adds "
            "11.0 MUSD of deferred production.",
            "The allowances assume existing structures, power and control system capacity are adequate; "
            "structural and electrical surveys are due in detailed design.",
        ],
    )

    doc.add_heading("6.5  Library benchmark", level=2)
    para(
        doc,
        "The azoth equipment cost model gives a purchased cost of 23,230 USD for the HP separator and "
        "3.37 MUSD for the export compressor, and total module costs of 0.11 and 16.50 MUSD. The "
        "greenfield topsides stack for those two modules is 61.7 MUSD. That number is useful as a "
        "scale check, not as the project estimate: it prices new equipment on a greenfield topsides, "
        "while the debottlenecking work is brownfield modification and package supply. The study's "
        "brownfield allowances are the estimate of record.",
    )


    doc.add_heading("6.6  Manhour estimate", level=2)
    mh = FEED["manhours"]
    para(
        doc,
        f"The estimate is {mh['totals']['total']:,} manhours: {mh['totals']['engineering']:,} engineering, "
        f"{mh['totals']['fabrication']:,} fabrication and {mh['totals']['offshore']:,} offshore. The split "
        "matters because the rates differ by more than a factor of two: engineering is costed at "
        f"{mh['rates_usd_per_h']['engineering']:.0f} USD/h, fabrication at {mh['rates_usd_per_h']['fabrication']:.0f} "
        f"and offshore at {mh['rates_usd_per_h']['offshore']:.0f}. Direct labour is "
        f"{mh['labour_cost_total_musd']:.2f} MUSD; the balance of the 36 MUSD is equipment, bulk material, "
        "marine spread, vendor services and owner costs.",
    )
    add_table(
        doc,
        ["Discipline", "Engineering", "Fabrication", "Offshore", "Total"],
        [(r["discipline"], f"{r['engineering']:,}", f"{r['fabrication']:,}", f"{r['offshore']:,}", f"{r['total']:,}") for r in mh["rows"]]
        + [("Total", f"{mh['totals']['engineering']:,}", f"{mh['totals']['fabrication']:,}", f"{mh['totals']['offshore']:,}", f"{mh['totals']['total']:,}")],
        widths=[6.0, 2.8, 2.8, 2.6, 2.6],
        caption="Table 6-4  Discipline manhours",
        font_size=8.2,
    )
    para(
        doc,
        "The offshore hours are concentrated in the four-week window and are the reason the histogram "
        "peaks there. The estimate assumes the compressor and water-package work is executed in parallel "
        "and that the HIPPS loop test is completed inside the window; a serial execution would add about "
        "two weeks and 11 MUSD of deferred production.",
    )

    doc.add_heading("6.7  Cost build-up", level=2)
    c = FEED["cost"]
    para(
        doc,
        f"Table 6-5 distributes the {c['total_musd']:.1f} MUSD estimate of record across its cost groups. "
        f"The build-up reconciles to the package capex (difference {abs(c['total_musd'] - c['estimate_of_record_musd']):.2f} MUSD). "
        "It is an allocation of a Class 4/5 allowance, not a quantity-surveyed estimate; no separate "
        "contingency line is added, because the allowances already carry vendor and installation "
        "contingency. The HAZOP actions in Section 10 carry an estimated 1.8 MUSD for level trips, "
        "PSVs, the standby pump, the larger coalescer and F&G/ESD additions; that sits inside the "
        "Class 4/5 accuracy band and does not change the 36.0 MUSD estimate of record. Risk-based "
        "contingency belongs to the owner's sanction estimate. The fix "
        "allowances in Table 6-3 are installed totals; Table 6-5 redistributes the same 36.0 MUSD by "
        "cost type, so the equipment line is not comparable with an individual fix allowance.",
    )
    add_table(
        doc,
        ["Cost group", "Scope", "MUSD"],
        [(g["group"].replace("precommissioning", "Pre-commissioning").title(), "; ".join(i["item"] for i in g["items"]), f"{g['subtotal_musd']:.1f}") for g in c["groups"]]
        + [("Total", "Recommended HIPPS + water package", f"{c['total_musd']:.1f}")],
        widths=[3.8, 10.0, 2.2],
        caption="Table 6-5  Cost build-up (AACE Class 4/5, no separate contingency)",
        font_size=8.0,
    )


def drawings(doc):
    doc.add_heading("7  Drawings, including the DEXPI PFD", level=1)
    para(
        doc,
        "The drawing set carries the study's process and schedule information. The process flow "
        "diagram is issued as a DEXPI 2.0 Process exchange with an embedded drawing, generated from the "
        "same flowsheet that the model runs, so the topology and the picture cannot drift apart. The "
        "P&ID is simplified: it shows function, not fabrication. Line classes, nozzle sizes, supports "
        "and detailed loop drawings are excluded and belong to detailed design.",
    )
    add_table(
        doc,
        ["Drawing", "Title", "Rev", "Status"],
        [
            ("AZ-FEED-08-DEXPI-PFD-001", "Process flow diagram — DEXPI 2.0 Process exchange with drawing", "B", "Study"),
            ("AZ-FEED-08-BFD-001", "Block flow diagram — year-ten base case and debottleneck scope", "B", "Study"),
            ("AZ-FEED-08-PID-001", "Simplified P&ID — HP separation, relief and produced water", "B", "Study"),
            ("AZ-FEED-08-SCH-001", "Implementation schedule — phases and shutdown window", "B", "Study"),
        ],
        widths=[4.4, 9.0, 1.6, 1.8],
        caption="Table 7-1  Drawing register",
    )
    add_figure(doc, DRAW / "AZ-FEED-08-BFD-001.png", 16.5, "Drawing AZ-FEED-08-BFD-001 Rev B — block flow diagram")
    add_figure(
        doc,
        DRAW / "AZ-FEED-08-DEXPI-PFD-001.png",
        16.5,
        "Drawing AZ-FEED-08-DEXPI-PFD-001 Rev B — DEXPI 2.0 process flow diagram "
        "(A0 original; reduced for the report; full-size PDF and XML in drawings/)",
    )
    para(
        doc,
        "The DEXPI exchange carries 31 process steps (26 equipment items plus the feed and four "
        "products) and 32 streams: three separators, the staged compression trains (1 + 3 + 5 stages "
        "with intercooling), the let-down valves, the stock tank and the sales-gas mixer. It validates "
        "against the vendored DEXPI 2.0 schema with no diagnostics. The exporter reports semantic_valid "
        "true and a converged run; the approval status is NOT_A_DEXPI_EV_CERTIFICATE and the CAE round "
        "trip is QUALIFICATION_REQUIRED, which is the library's honest statement that a valid exchange "
        "is not a certified one. The PFD deliberately omits instruments, PSV-101 and the flare; those "
        "are on the P&ID.",
    )

    # landscape page for the P&ID
    new = doc.add_section(WD_SECTION.NEW_PAGE)
    new.orientation = WD_ORIENT.LANDSCAPE
    new.page_width, new.page_height = doc.sections[0].page_height, doc.sections[0].page_width
    new.left_margin = Cm(1.6)
    new.right_margin = Cm(1.6)
    new.top_margin = Cm(1.4)
    new.bottom_margin = Cm(1.2)
    p = doc.add_paragraph()
    p.alignment = WD_ALIGN_PARAGRAPH.CENTER
    p.paragraph_format.space_before = Pt(0)
    p.paragraph_format.space_after = Pt(0)
    p.add_run().add_picture(str(DRAW / "AZ-FEED-08-PID-001.png"), width=Cm(23.5))
    c = doc.add_paragraph()
    c.alignment = WD_ALIGN_PARAGRAPH.CENTER
    r = c.add_run("Drawing AZ-FEED-08-PID-001 Rev B — simplified P&ID (A3 original; reduced to A4 landscape)")
    r.font.size = Pt(8)
    r.italic = True
    r.font.color.rgb = GREY

    # back to portrait
    back = doc.add_section(WD_SECTION.NEW_PAGE)
    back.orientation = WD_ORIENT.PORTRAIT
    back.page_width, back.page_height = doc.sections[0].page_width, doc.sections[0].page_height
    back.left_margin = Cm(2.0)
    back.right_margin = Cm(2.0)
    back.top_margin = Cm(2.2)
    back.bottom_margin = Cm(2.0)
    add_figure(doc, DRAW / "AZ-FEED-08-SCH-001.png", 16.5, "Drawing AZ-FEED-08-SCH-001 Rev B — implementation schedule")


def implementation(doc):
    doc.add_heading("8  Implementation project plan", level=1)
    doc.add_heading("8.1  Execution strategy", level=2)
    para(
        doc,
        "The plan runs from FEED close-out to benefits realisation in 42 weeks, with the four-week "
        "shutdown starting in week 32. The strategy is to do all pre-shutdown work — detailed design, "
        "procurement, fabrication and factory acceptance testing — before the window, and to confine "
        "offshore work to installation, tie-in, loop testing and commissioning. The compressor re-rate "
        "is the long-lead item and the only activity that can move the shutdown.",
    )
    para(
        doc,
        "The recommended package is executed as four parallel work fronts inside the window: the "
        "produced-water package (four weeks, the critical duration), the compressor re-rate (three "
        "weeks), the separator internals (three weeks) and the HIPPS (two weeks), followed by "
        "commissioning and handover. The flare/header fallback, if selected, runs four weeks in "
        "parallel with the water package and does not extend the window.",
    )

    doc.add_heading("8.2  Phases and milestones", level=2)
    add_table(
        doc,
        ["Phase", "Window", "Content", "Exit"],
        [
            ("A · Design", "Weeks 0–10", "FEED close-out, detailed design, HAZOP and SIL review, long-lead enquiry", "Design freeze; PO placed"),
            ("B · Procurement", "Weeks 6–32", "Compressor kit (22 wk), HIPPS (12 wk), internals (14 wk), water package (16 wk)", "All packages delivered"),
            ("C · Readiness", "Weeks 26–32", "Offshore survey, work packs, FAT, logistics and permits", "Shutdown readiness review"),
            ("D · Shutdown", "Weeks 32–36", "Installation and tie-in on four fronts; commissioning and handover", "First gas"),
            ("E · Ramp-up", "Weeks 36–40", "Performance test, ramp-up to 161.8 MMscfd", "Plateau achieved"),
            ("F · Close-out", "Weeks 40–42", "Close-out, lessons learned, benefits realisation", "Project closed"),
        ],
        widths=[2.6, 2.8, 8.0, 3.2],
        caption="Table 8-1  Project phases",
    )
    add_table(
        doc,
        ["Milestone", "Week", "Basis"],
        [
            ("FEED close-out / DG2", "4", "Planning assumption"),
            ("Compressor purchase order", "8", "Long-lead critical path"),
            ("Factory acceptance tests complete", "32", "Planning assumption"),
            ("Shutdown start", "32", "Four-week window"),
            ("First gas after shutdown", "36", "End of window"),
            ("Ramp-up complete", "40", "Planning assumption"),
            ("Close-out", "42", "Planning assumption"),
        ],
        widths=[8.0, 2.2, 6.0],
        caption="Table 8-2  Milestones",
        align_right=[1],
    )

    doc.add_heading("8.3  Shutdown schedule", level=2)
    para(
        doc,
        "The four-week window is enough for the gas side with the water upgrade and for the HIPPS "
        "variant: the longest task is the four-week water package, and the compressor, internals, "
        "HIPPS and flare work sit inside it in parallel. The fourth stage is a five-week job; it is "
        "compressed into the window with a 3.3 MUSD labour premium. The stabiliser is eight weeks, so "
        "it extends the shutdown by two weeks and defers production; on the year-ten rates that "
        "deferment is 11.0 MUSD before any manhour premium. The two add-ons together defer 14.3 MUSD.",
    )
    add_figure(doc, DRAW / "fig_gantt.png", 16.5, "Figure 8-1  Project phases and the four-week shutdown window")

    doc.add_heading("8.4  Critical path and long-lead items", level=2)
    bullets(
        doc,
        [
            "Compressor re-rate kit: 22 weeks from purchase order; order by week 8 to deliver before "
            "week 32. This is the critical path.",
            "HIPPS hardware: 12 weeks; order by week 8, factory acceptance test by week 32, loop test "
            "inside the shutdown.",
            "Separator internals: 14 weeks; the vendor must guarantee K = 0.14 m/s and separation "
            "performance.",
            "Produced-water package: 16 weeks; performance guarantee on outlet oil-in-water and capacity.",
            "Marine spread and logistics: booked by week 30; weather and vessel availability are the "
            "residual risks.",
        ],
    )

    doc.add_heading("8.5  HSE, commissioning and operations", level=2)
    para(
        doc,
        "The shutdown is a simultaneous-operations campaign on a live platform: hydrocarbon isolation, "
        "gas-freeing, hot-work permits and dropped-object controls govern the schedule. The HIPPS trip "
        "must be proof-tested before the HP inlet is returned to service, and the PSV must be "
        "re-certified. Commissioning includes leak tests, loop checks, compressor surge and performance "
        "tests, separator and water-package performance tests, and a flare radiation check before the "
        "first relief event.",
    )
    para(
        doc,
        "Operational readiness requires revised operating procedures, alarm and trip setpoints, "
        "training for the new water package and HIPPS bypass management, and a management-of-change "
        "record for the changed relief basis. The HIPPS bypass is a SIL-3 function and must be under "
        "strict permit control for the life of the modification.",
    )

    doc.add_heading("8.6  Contracting and resourcing", level=2)
    para(
        doc,
        "The study assumes an EPCm contract for the shutdown with vendor packages for the compressor "
        "re-rate, the separator internals, the HIPPS and the water package. Performance guarantees "
        "should sit with the vendors for the internals K factor, the water-package outlet quality and "
        "the HIPPS SIL. The owner retains the relief and SIL studies and the management of change.",
    )


    doc.add_heading("8.7  Resource-loaded plan", level=2)
    sch = FEED["schedule"]
    para(
        doc,
        f"The 42-week plan carries {sch['total_planned_hours']:,.0f} manhours across 26 activities. The "
        f"weekly histogram peaks at {sch['peak_week_hours']:,.0f} h in the shutdown, when the four work "
        "fronts and commissioning overlap. The S-curve is the same hours cumulatively: it is shallow "
        "through design and procurement, steep through the shutdown, and flat after first gas. That "
        "shape is the schedule risk: the plan has no float in the shutdown and only two weeks of float "
        "in the compressor delivery.",
    )
    add_figure(doc, DRAW / "fig_resources.png", 16.2, "Figure 8-2  Resource-loaded plan: weekly histogram and cumulative S-curve")
    windows = {"A Design": "Weeks 0–11", "B Procurement": "Weeks 8–31", "C Readiness": "Weeks 26–32",
               "D Shutdown": "Weeks 32–36", "E Ramp-up": "Weeks 36–40", "F Close-out": "Weeks 40–42"}
    exits = {"A Design": "Design freeze and AFC", "B Procurement": "All vendor packages delivered and FAT-tested",
             "C Readiness": "Shutdown readiness review", "D Shutdown": "First gas",
             "E Ramp-up": "Plateau achieved", "F Close-out": "Project closed"}
    phase_rows = []
    for phase in sch["phases"]:
        hours = sum(a["hours"] for a in sch["activities"] if a["phase"] == phase)
        phase_rows.append((phase, windows[phase], f"{hours:,.0f}", exits[phase]))
    add_table(
        doc,
        ["Phase", "Window", "Manhours", "Exit"],
        phase_rows,
        widths=[2.8, 3.0, 2.8, 7.4],
        caption="Table 8-3  Phase manhours (from the resource-loaded plan)",
        font_size=8.5,
    )


def economics(doc):
    doc.add_heading("9  Economics", level=1)
    doc.add_heading("9.1  Basis", level=2)
    para(
        doc,
        "The production forecast holds the gas potential at 163.4 MMscfd and declines it 5 %/yr; the "
        "baseline is capped at 137.1 MMscfd and each package at its own cap. The condensate follows "
        "the gas at the computed 21.4 bbl/MMscf, the water cut rises from 48.7 % to 57 %, and the "
        "incremental water carries a 2 USD/bbl disposal cost. Gas is 4 USD/MMBtu at 1.05e3 MMBtu/MMscf, "
        "condensate is 70 USD/bbl, the discount is 10 %, the tax is 25 % with ten-year straight-line "
        "depreciation, and the package adds 2.5 MUSD/yr fixed plus 0.33 MUSD/yr of electricity.",
    )
    add_table(
        doc,
        ["Parameter", "Value", "Unit"],
        [
            ("Gas price", "4.0", "USD/MMBtu"),
            ("Condensate price", "70.0", "USD/bbl"),
            ("Heating value", "1,050", "MMBtu/MMscf"),
            ("Discount rate", "10", "%"),
            ("Tax rate", "25", "%"),
            ("Decline", "5", "%/yr"),
            ("Water cut, start → end", "48.7 → 57.0", "%"),
            ("Fixed opex + electricity", "2.5 + 0.33", "MUSD/yr"),
            ("Incremental water disposal", "2.0", "USD/bbl"),
            ("Evaluation period", "10", "years"),
        ],
        widths=[7.4, 4.6, 3.4],
        caption="Table 9-1  Economic basis",
        align_right=[1],
    )

    doc.add_heading("9.2  Package economics", level=2)
    add_table(
        doc,
        ["Package", "Capex, MUSD", "NPV, MUSD", "Payback", "Gas PV, MUSD", "Condensate PV, MUSD"],
        [
            ("Gas side", 34.0, ECON["results"]["Gas side"]["npv"], ECON["results"]["Gas side"]["payback"], ECON["results"]["Gas side"]["gas_pv"], ECON["results"]["Gas side"]["cond_pv"]),
            ("Gas side + water", 42.0, ECON["results"]["Gas side + water"]["npv"], ECON["results"]["Gas side + water"]["payback"], ECON["results"]["Gas side + water"]["gas_pv"], ECON["results"]["Gas side + water"]["cond_pv"]),
            ("HIPPS + water", 36.0, ECON["results"]["HIPPS + water"]["npv"], ECON["results"]["HIPPS + water"]["payback"], ECON["results"]["HIPPS + water"]["gas_pv"], ECON["results"]["HIPPS + water"]["cond_pv"]),
            ("+ fourth stage", 67.3, ECON["results"]["+ fourth stage"]["npv"], ECON["results"]["+ fourth stage"]["payback"], ECON["results"]["+ fourth stage"]["gas_pv"], ECON["results"]["+ fourth stage"]["cond_pv"]),
            ("+ stabiliser", 77.0, ECON["results"]["+ stabiliser"]["npv"], ECON["results"]["+ stabiliser"]["payback"], ECON["results"]["+ stabiliser"]["gas_pv"], ECON["results"]["+ stabiliser"]["cond_pv"]),
            ("Both add-ons", 99.0, ECON["results"]["Both add-ons"]["npv"], ECON["results"]["Both add-ons"]["payback"], ECON["results"]["Both add-ons"]["gas_pv"], ECON["results"]["Both add-ons"]["cond_pv"]),
        ],
        widths=[3.8, 2.6, 2.6, 2.0, 2.8, 3.0],
        caption="Table 9-2  Package economics (NPV at 10 %, after tax)",
        align_right=[1, 2, 3, 4, 5],
    )
    add_figure(doc, DRAW / "fig_economics.png", 16.0, "Figure 9-1  Cumulative discounted cash for the core packages")

    doc.add_heading("9.3  Recommended case cash flow", level=2)
    cf = ECON["results"]["HIPPS + water"]
    rows = []
    for i in range(10):
        rows.append((
            i + 1,
            f"{cf['inc_gas'][i]:.1f}",
            f"{cf['inc_cond'][i]:.0f}",
            f"{cf['rev_gas'][i]:.2f}",
            f"{cf['rev_cond'][i]:.2f}",
            f"{cf['opex'][i]:.2f}",
            f"{cf['cash'][i]:.2f}",
            f"{cf['cum'][i]:.2f}",
        ))
    add_table(
        doc,
        ["Year", "Gas, MMscfd", "Cond., bbl/d", "Gas rev., MUSD", "Cond. rev., MUSD", "Opex, MUSD", "Cash, MUSD", "Cumulative disc., MUSD"],
        rows,
        widths=[1.2, 2.2, 2.2, 2.2, 2.4, 2.0, 2.2, 2.4],
        caption="Table 9-3  Incremental cash flow for the recommended package",
        align_right=[1, 2, 3, 4, 5, 6, 7],
    )
    para(
        doc,
        "The incremental production is fully captured in years 1–4 and falls to zero from year 5 as the "
        "declining potential drops below the 137.1 MMscfd baseline; the fixed opex continues, which is "
        "the conservative treatment carried from the notebook. The package therefore earns its return "
        "early, which is why the payback is in year one and why schedule slip is so expensive.",
    )

    doc.add_heading("9.4  Value split", level=2)
    add_table(
        doc,
        ["Case", "NPV, MUSD"],
        [
            ("Gas + condensate", ECON["split"][0]["npv"]),
            ("Gas only (condensate price 0)", ECON["split"][1]["npv"]),
            ("Condensate only (gas price 0)", ECON["split"][2]["npv"]),
            ("Pre-tax", ECON["split"][3]["npv"]),
        ],
        widths=[10.0, 4.0],
        caption="Table 9-4  Value split for the gas + water package",
        align_right=[1],
    )
    para(
        doc,
        f"Discounted revenue is {money(cf['gas_pv'], 1)} MUSD from gas "
        f"({money(cf['gas_pv'] / (cf['gas_pv'] + cf['cond_pv']) * 100, 1)} %) and "
        f"{money(cf['cond_pv'], 1)} MUSD from condensate "
        f"({money(cf['cond_pv'] / (cf['gas_pv'] + cf['cond_pv']) * 100, 1)} %). The project is a gas "
        "project with a condensate co-product, and the client's condensate-only objective is not "
        "economic at 70 USD/bbl.",
    )

    doc.add_heading("9.5  Sensitivity", level=2)
    add_figure(doc, DRAW / "fig_tornado.png", 16.0, "Figure 9-2  NPV sensitivity around the gas + water case")
    para(
        doc,
        "The gas price is the largest swing: at zero the package is the condensate-only case and loses "
        "36.2 MUSD; at 8 USD/MMBtu it gains about 54.4. The decline rate and the capex are next, then "
        "the shutdown slip; the condensate price and the discount rate are small. The HIPPS option is "
        "worth about 5.6 MUSD. The message for the client is uncomfortable but clear: the condensate "
        "gain is real, but the project's value is carried by the gas and by fitting the shutdown; a "
        "condensate-only case does not pay, and a two-week slip gives back 11.0 MUSD, more than half "
        "the NPV.",
    )


def hazop(doc):
    doc.add_heading("10  Independent HAZOP", level=1)
    doc.add_heading("10.1  Method and independence", level=2)
    para(
        doc,
        "The completed Rev B design pack — basis of design, DEXPI PFD, equipment and material schedule, "
        "relief and HIPPS basis, and the 42-week plan — was frozen and issued to a HAZOP team that did not "
        "prepare it. The team worked node by node with the guidewords NO/NOT, MORE, LESS, REVERSE, AS WELL "
        "AS, PART OF and OTHER THAN, covering normal operation, startup, shutdown, maintenance and utility "
        "failure. The chair sequenced the process and safety roles and the scribe recorded the worksheet; "
        "the designer did not vote on findings. The worksheet is Appendix E and the workbook HAZOP sheet.",
    )
    para(
        doc,
        "This is a study-level HAZOP on a screening design, not a formal IEC 61882 workshop with a full "
        "multi-discipline owner team. It is independent of the design author, but it is not a substitute "
        "for the owner's pre-FEED or detailed-design HAZOP. Its purpose is to test whether the FEED "
        "conclusions survive a structured challenge, and to leave a traceable action list.",
    )
    doc.add_heading("10.2  Nodes and coverage", level=2)
    add_table(
        doc,
        ["Node", "Boundary", "Why it matters"],
        [
            ("1", "HP inlet, SDV-101 and HIPPS", "SIL-3 final element and the 35 % relief credit"),
            ("2", "V-101 HP three-phase separator", "First capacity constraint; level and interface control"),
            ("3", "MP let-down, VLV-102 and V-102", "Two-phase let-down; liquid carryover"),
            ("4", "LP let-down, V-103, E-104 and V-104", "Stock tank; flash gas to fuel gas; reverse flow on shutdown"),
            ("5", "Compression K-101, K-102 A/B/C, K-103 A–E", "Surge, overspeed and cooler duty after re-rate"),
            ("6", "Produced-water package V-105/V-106/P-102", "New coalescer; oil-in-water and bypass"),
            ("7", "PSV-101, relief header and flare", "Relief gate; radiation and back pressure"),
            ("8", "Export gas and condensate lines", "Battery limit; reverse flow and overpressure"),
        ],
        widths=[1.0, 6.4, 8.6],
        caption="Table 10-1  HAZOP nodes",
        font_size=8.5,
    )
    if HAZ:
        doc.add_heading("10.3  Results", level=2)
        ws = HAZ["worksheet"]
        n = len(ws)
        high = sum(1 for r in ws if r["risk"].lower() == "high")
        med = sum(1 for r in ws if r["risk"].lower() == "medium")
        actions = HAZ["actions"]
        para(
            doc,
            f"The team recorded {n} deviations: {high} high-risk and {med} medium-risk, with the rest low. "
            f"{len(actions)} actions were raised, of which {sum(1 for a in actions if a['status'].lower().startswith('closed'))} are "
            "closed in this revision and the remainder are conditions on detailed design or on the safety "
            "case. No finding invalidates the capacity conclusion; the 35 % HIPPS credit remains "
            "conditional and must be confirmed by the dynamic relief study.",
        )
        add_table(
            doc,
            ["Node", "Deviation", "Risk", "Action"],
            [(r["node"], r["deviation"], r["risk"], r["action"]) for r in ws if r["risk"].lower() in ("high", "medium")][:12],
            widths=[3.4, 5.2, 1.4, 6.0],
            caption="Table 10-2  High and medium findings",
            font_size=7.6,
        )
        doc.add_heading("10.4  Actions and close-out", level=2)
        add_table(
            doc,
            ["ID", "Action", "Owner", "Priority", "Status"],
            [(a["id"], a["action"], a["owner"], a["priority"], a["status"]) for a in actions],
            widths=[1.2, 9.8, 2.2, 1.4, 1.6],
            caption="Table 10-3  HAZOP action list",
            font_size=7.6,
        )
        para(
            doc,
            "Close-out note: HAZ-03 and HAZ-10 are recorded closed against the Rev B scope (the "
            "proof-test/bypass restriction and the F&G/ESD philosophy), but the issued procedure and the "
            "cause-and-effect matrix remain detailed-design deliverables. The independent record is not "
            "rewritten to match the design; the distinction is stated so a reader does not mistake a "
            "scope decision for a completed procedure.",
        )
        doc.add_heading("10.5  Effect on the design", level=2)
        for text in HAZ.get("design_effect", []):
            p = doc.add_paragraph(text, style="List Bullet")
            for r in p.runs:
                r.font.size = Pt(9)
    else:
        para(doc, "The HAZOP worksheet is not yet available in this build.")


def risks(doc):
    doc.add_heading("11  Risks, assumptions and verification", level=1)
    doc.add_heading("11.1  Risk register", level=2)
    add_table(
        doc,
        ["Risk", "Likelihood", "Impact", "Mitigation"],
        [
            ("HIPPS safety case not approved or 35 % credit reduced", "Medium", "Fallback +6 MUSD, NPV −5.6 MUSD", "Early dynamic relief and SIL study; hold flare/header fallback; no spend on HIPPS until concept accepted"),
            ("Compressor long-lead slips past week 8", "Medium", "Shutdown moves; 11.0 MUSD per two weeks", "Place PO by week 8; vendor kick-off; pre-order long-lead castings"),
            ("Shutdown overruns four weeks", "Medium", "11.0 MUSD per two weeks deferred", "Parallel fronts, float on non-critical tasks, daily progress, weather contingency"),
            ("Separator internals do not reach K 0.14", "Low–Medium", "Separator cap stays near 137", "Vendor performance guarantee; test data; fallback to larger internals"),
            ("Water package misses outlet quality", "Medium", "Regulatory and disposal risk", "Performance guarantee on outlet oil-in-water; coalescer redundancy"),
            ("Flare radiation contour exceeds sterile area", "Medium", "Fallback design change", "Detailed flare study; verify 82.4 m contour before selecting fallback"),
            ("Gas price below 4 USD/MMBtu", "Medium", "NPV falls; zero price −36.2 MUSD", "Gas sales agreement; stage-gate before spend"),
            ("Decline steeper than 5 %/yr", "Medium", "Decline 7 % gives 3.6 MUSD NPV", "Reservoir review; update forecast before FID"),
            ("Cost estimate growth", "Medium", "Capex +30 % gives 7.3 MUSD NPV", "Class 3 estimate at detailed design; contingency"),
            ("Simultaneous operations incident", "Low", "Safety and schedule", "SIMOPS procedures, isolation philosophy, permit control"),
        ],
        widths=[5.0, 2.0, 4.2, 5.6],
        caption="Table 11-1  Risk register",
        font_size=8,
    )

    doc.add_heading("11.2  Key assumptions", level=2)
    bullets(
        doc,
        [
            "The field and history are illustrative; the decline is a flat 5 %/yr forecast, not a "
            "reservoir simulation.",
            "The separators are equilibrium stages; the compressor is isentropic (0.78) with no "
            "interstage condensate; the produced-water package is a stated capacity and cost, not a "
            "hydrocyclone model.",
            "The relief check is a screening PSV and flare calculation, not a full relief study; the "
            "fire case is screened at 0.54 kg/s.",
            "The HIPPS 35 % relief reduction, the 4/6-week schedule rule, the water-treatment capacity "
            "and the brownfield costs are study assumptions.",
            "Flow assurance, hydrates, wax, slugging and sand are outside the study, as are "
            "decommissioning and inflation.",
            "The costs are AACE Class 4/5; the long-lead compressor is a schedule risk the deferment "
            "model does not carry.",
            "Every calculation is a screening value from a library that ships unverified or "
            "partially_verified models; none is a design deliverable.",
        ],
    )

    doc.add_heading("11.3  Verification status", level=2)
    para(
        doc,
        "The azoth calculations used in the study are listed below. All are partially_verified: partly "
        "externally checked, screening values, not design deliverables. The provenance hashes identify "
        "the exact spec and implementation used.",
    )
    add_table(
        doc,
        ["Calculation", "Use", "Key result", "Status"],
        [
            ("process.three_phase_separator", "HP phase split, retention, settling", "1,869 / 76.6 / 284.2 mol/s; 17.09 s settling", "partially_verified"),
            ("process.separator_capacity_assessment", "HP gas load and cap", "99.77 % now; 118.9 % at potential", "partially_verified"),
            ("process.flare", "Flare capacity and radiation", "91.1 % now; 107.5 % at +18 %", "partially_verified"),
            ("process.psv_sizing", "PSV orifice check", "4.69 in² required; P orifice", "partially_verified"),
            ("process.nog070_sil_determination", "HIPPS SIL", "SIL 3 achieved and compliant", "partially_verified"),
            ("process.s001_secondary_pressure_protection", "Pipeline secondary protection", "Acceptable", "partially_verified"),
            ("process.equipment_cost_estimate", "Greenfield benchmark", "16.50 MUSD modules; 61.7 MUSD topsides", "partially_verified"),
            ("process.compressor / pump / cooler / pipe", "Train and lines", "2,256 kW; 3,885 kW; line arrivals", "partially_verified"),
        ],
        widths=[5.6, 3.8, 4.6, 2.8],
        caption="Table 11-2  Verification register (summary; full register in Appendix A)",
        font_size=8,
    )

    doc.add_heading("11.4  What would change the recommendation", level=2)
    bullets(
        doc,
        [
            "A gas price below about 3.5 USD/MMBtu, or a decline steeper than about 6 %/yr, would make "
            "the package marginal or negative.",
            "A HIPPS credit materially below 35 %, or a safety case the authority does not accept, "
            "selects the flare/header fallback at 42 MUSD.",
            "A shutdown longer than four weeks, or a compressor delivery after week 32, gives back more "
            "than half the NPV.",
            "A condensate price above about 90 USD/bbl would make the fourth stage and the stabiliser "
            "worth revisiting, though the stabiliser's schedule penalty remains.",
        ],
    )


def conclusions(doc):
    doc.add_heading("12  Conclusions and recommendations", level=1)
    doc.add_heading("12.1  Conclusions", level=2)
    conclusions_list = [
        "The facility, not the reservoir, sets the rate: 137.1 MMscfd against a 163.4 MMscfd well "
        "potential, with the HP separator at 99.8 %, the compressor at 98.1 % and the produced-water "
        "plant at 92.7 % of their limits.",
        "The capacity chain has a hidden third link: a gas-side package without the water upgrade is "
        "capped at 147.8 MMscfd and is slightly negative in NPV.",
        "The flare, not the PSV, is the relief gate: the existing PSV passes the +18 % case at 86.7 %, "
        "but the flare overloads at 107.5 %.",
        "The HIPPS + water package is the best core option: 36 MUSD, four weeks, 24.8 MUSD NPV at 10 % "
        "and payback in year one, with the flare/header upgrade as the 42 MUSD fallback.",
        "The project is a gas project with a condensate co-product: 73.7 % of the discounted revenue "
        "is gas, and the condensate-only case loses 36.2 MUSD.",
        "The fourth stage is close to neutral and should be deferred; the stabiliser fails because its "
        "two-week shutdown extension defers 11.0 MUSD.",
    ]
    for i, text in enumerate(conclusions_list, start=1):
        p = doc.add_paragraph()
        r = p.add_run(f"{i}.  ")
        r.bold = True
        r.font.size = Pt(10)
        r = p.add_run(text)
        r.font.size = Pt(10)
        p.paragraph_format.space_after = Pt(5)

    doc.add_heading("12.2  Recommendations", level=2)
    recs = [
        "Approve the HIPPS + water package for detailed design, with the flare/header upgrade as the "
        "relief fallback and a gate on the HIPPS safety case.",
        "Commission the dynamic relief study and the SIL verification immediately; the 35 % credit is "
        "the package's principal technical assumption.",
        "Place the compressor re-rate purchase order by week 8 to protect the week-32 shutdown; it is "
        "the critical path.",
        "Write vendor performance guarantees for the separator internals K factor, the water-package "
        "outlet quality and the HIPPS SIL.",
        "Verify the +18 % flare radiation contour (82.4 m to 4.73 kW/m²) against the platform sterile "
        "area before selecting the fallback.",
        "Revisit the fourth stage at a higher condensate price or in the following shutdown; do not "
        "carry the stabiliser in this window.",
        "Confirm the incremental gas sales route and price before sanction; the economics are "
        "conditional on 4 USD/MMBtu.",
    ]
    for i, text in enumerate(recs, start=1):
        p = doc.add_paragraph()
        r = p.add_run(f"R{i}.  ")
        r.bold = True
        r.font.size = Pt(10)
        r = p.add_run(text)
        r.font.size = Pt(10)
        p.paragraph_format.space_after = Pt(5)

    add_table(
        doc,
        ["Decision requested", "Timing"],
        [
            ("Approve FEED and release detailed design", "Week 0"),
            ("Accept HIPPS as the relief basis, subject to the safety case", "Week 4"),
            ("Release compressor and HIPPS purchase orders", "Week 8"),
            ("Confirm shutdown window and marine spread", "Week 30"),
            ("Sanction the modification", "After Class 3 estimate"),
        ],
        widths=[11.5, 4.5],
        caption="Table 12-1  Decisions requested",
    )


def appendices(doc):
    doc.add_heading("Appendix A  Calculation verification register", level=1)
    para(
        doc,
        "Each row is one azoth calculation behind the study. Status is the library's own; provenance "
        "identifies the spec and implementation hashes returned with the result.",
    )
    add_table(
        doc,
        ["azoth id", "Scenario", "Key inputs", "Key result", "Status", "Provenance"],
        [
            ("process.three_phase_separator", "HP separator, year ten", "2,230 mol/s; 60 bara; 354.15 K; Ø1.96×7.26 m", "1,869.19 / 76.62 / 284.19 mol/s; settling 17.09 s", "partially_verified", "d0fc37427277 / 1401191864f9"),
            ("process.separator_capacity_assessment", "HP separator gas load", "K 0.11 m/s; level 0.2; horizontal", "99.77 %; max flow 0.809 m³/s", "partially_verified", "444d257c5074 / 5163245200d8"),
            ("process.separator_capacity_assessment", "At well potential", "2,227.53 mol/s; same basis", "118.9 %; infeasible", "partially_verified", "444d257c5074 / 5163245200d8"),
            ("process.flare", "Current relief", "1,869.19 mol/s; 6 MPa; design 46.34 kg/s", "91.10 %; 74.96 m", "partially_verified", "0282ff3baf7e / e49499aef85e"),
            ("process.flare", "+18 % gas", "2,205.64 mol/s", "107.50 %; overloaded", "partially_verified", "0282ff3baf7e / e49499aef85e"),
            ("process.flare", "+18 %, +25 % capacity", "Design 57.93 kg/s", "86.00 %", "partially_verified", "0282ff3baf7e / e49499aef85e"),
            ("process.flare", "+18 %, HIPPS −35 %", "1,433.67 mol/s", "69.87 %; 64.03 m", "partially_verified", "0282ff3baf7e / e49499aef85e"),
            ("process.psv_sizing", "Blocked outlet, current", "42.634 kg/s; 66 barg; 354.15 K", "4.688 in²; P; 57.97 kg/s", "partially_verified", "70df5cd128ce / b2414cf1d7ca"),
            ("process.psv_sizing", "Blocked outlet, +18 %", "50.308 kg/s", "5.531 in²; 86.7 %", "partially_verified", "70df5cd128ce / b2414cf1d7ca"),
            ("process.nog070_sil_determination", "HIPPS pipeline", "PFD 1e-4", "SIL 3, compliant", "partially_verified", "2a87677e7963 / f024f6526109"),
            ("process.s001_secondary_pressure_protection", "Pipeline secondary protection", "72.6/66/99 bara; 12 months", "Acceptable", "partially_verified", "44cbe4e54cdc / cf016d03c700"),
            ("process.equipment_cost_estimate", "Greenfield benchmark", "HP separator; 2,273 kW compressor", "16.50 MUSD modules; 61.7 MUSD", "partially_verified", "0fa76a97e7b2 / 83a022df39da"),
        ],
        widths=[3.6, 2.4, 3.4, 3.0, 2.2, 2.2],
        caption="Table A-1  Calculation register",
        font_size=7.5,
    )
    para(
        doc,
        "The FEED additions are author's calculations, reproduced in model/feed_model.py and reported "
        "with their formulae so they can be checked. They are not azoth library results.",
    )
    add_table(
        doc,
        ["Calculation", "Formula", "Result"],
        [
            ("CO2 partial pressure", "z_CO2 × P_HP", f"{FEED['co2']['p_co2_vapour_bar']:.3f} bar"),
            ("Corrosion allowance", "inhibited rate × design life", f"{FEED['co2']['ca_required_mm']:.2f} mm → 3.0 mm selected"),
            ("V-101 shell thickness", "P R / (S E − 0.6 P) + CA", f"{FEED['mechanical']['v101']['t_required_mm']:.2f} + 3.0 = {FEED['mechanical']['v101']['t_with_ca_mm']:.2f} mm"),
            ("Export gas line thickness", "P D / (2 (S E + P Y)) + CA", f"{FEED['mechanical']['gas_line']['t_required_mm']:.2f} + 3.0 = {FEED['mechanical']['gas_line']['t_with_ca_mm']:.2f} mm"),
            ("Produced-water line thickness", "P D / (2 (S E + P Y))", f"{FEED['mechanical']['water_line']['t_required_mm']:.2f} mm → 3.76 mm selected"),
            ("Manhours", "Σ discipline hours", f"{FEED['manhours']['totals']['total']:,} h"),
            ("Direct labour cost", "Σ hours × rate", f"{FEED['manhours']['labour_cost_total_musd']:.2f} MUSD"),
            ("Cost build-up", "Σ cost groups", f"{FEED['cost']['total_musd']:.1f} MUSD (reconciles to 36.0)"),
            ("Schedule hours", "Σ activity hours", f"{FEED['schedule']['total_planned_hours']:,.0f} h"),
            ("Peak resource week", "max weekly histogram", f"{FEED['schedule']['peak_week_hours']:,.0f} h/week"),
        ],
        widths=[4.0, 6.2, 6.2],
        caption="Table A-2  FEED calculations and checks",
        font_size=7.8,
    )
    para(
        doc,
        "The model also uses process.compressor, process.cooler, process.pump, process.separator, "
        "process.throttling_valve, eos.pt_flash, eos.ph_flash, eos.pipe_beggs_and_brills and "
        "process.fire_case_relief; all are part of the same azoth 0.1.4 screening basis.",
    )

    doc.add_heading("Appendix B  Equipment datasheets", level=1)
    blocks = [
        (
            "B.1  V-101 — HP three-phase separator",
            [
                ("Design pressure / temperature", "66 barg / 354.15 K"),
                ("Size", "Ø 1.96 m × 7.26 m T/T, horizontal"),
                ("Feed", "2,230 mol/s at 13.45 mol % water"),
                ("Vapour / oil / water", "1,869.19 / 76.62 / 284.19 mol/s"),
                ("Gas load", "99.77 % of Souders-Brown at K = 0.11 m/s"),
                ("Internals", "Inlet device + vane pack; K = 0.14 m/s after change"),
                ("Oil / water retention", "10.92 / 7.83 min (minimum 9.0 / 6.5)"),
                ("Interface settling", "17.09 s — insufficient; coalescer in water package"),
                ("Capacity after internals", "174.9 MMscfd"),
            ],
        ),
        (
            "B.2  K-101 — export compressor",
            [
                ("Service", "HP, MP and LP gas to 90 bara"),
                ("Stages / ratio", "Equal-ratio, ≤ 2.5 per stage"),
                ("Shaft power", "2,256 kW now; 2,689 kW at potential"),
                ("Driver", "2,300 kW; 98.1 % now, 116.9 % at potential"),
                ("Discharge temperature", "313.15 K after cooler"),
                ("Cooler duty", "3,885 kW now; 4,630 kW at potential"),
                ("Re-rate", "+18 % capacity; cap 139.7 → 161.8 MMscfd"),
                ("Isentropic efficiency", "0.78 (study assumption)"),
            ],
        ),
        (
            "B.3  HIPPS — SIL-3 high-pressure protection",
            [
                ("Function", "HP inlet/pipeline high-pressure protection"),
                ("Architecture", "2 × PT, 1oo2 voting, SIL-3 logic solver, SDV-101"),
                ("PFD average", "1e-4"),
                ("HFT / diagnostic coverage", "1 / 0.6"),
                ("Proof-test interval", "12 months"),
                ("Relief credit", "35 % of blocked-outlet case (study assumption)"),
                ("After credit", "32.70 kg/s; flare 69.9 %; PSV 3.60 in²"),
                ("Pipeline check", "72.6 bara maximum incidental; two barriers required and adequate"),
                ("S-001", "Acceptable with leakage assessed and routed safe"),
            ],
        ),
        (
            "B.4  Produced-water package",
            [
                ("Capacity", "3,000 → 6,000 bbl/d"),
                ("Current flow", "2,782 bbl/d (92.7 %)"),
                ("Equipment", "Hydrocyclone, coalescer, P-102, controls"),
                ("Coalescer basis", "Interface settling 17.09 s vs 60 s class check"),
                ("Water retention", "7.83 min (minimum 6.5)"),
                ("Capacity after upgrade", "Water plant no longer binds; 295.6 MMscfd equivalent"),
            ],
        ),
        (
            "B.5  Flare system (fallback)",
            [
                ("Design mass / molar capacity", "46.34 kg/s / 2,031.7 mol/s"),
                ("Current / +18 % load", "42.63 kg/s (91.1 %) / 50.31 kg/s (107.5 %)"),
                ("Upgrade", "+25 % capacity; 86.0 % at +18 %"),
                ("Radiation distance", "74.96 m now; 82.42 m at +18 %; 64.03 m with HIPPS"),
                ("Threshold", "4.73 kW/m²"),
            ],
        ),
    ]
    for heading, rows in blocks:
        doc.add_heading(heading, level=2)
        add_table(doc, ["Parameter", "Value"], rows, widths=[7.0, 10.0], font_size=8.5)

    doc.add_heading("Appendix C  Cost estimate detail", level=1)
    add_table(
        doc,
        ["Package", "Fixes", "Capex, MUSD", "Weeks", "Deferment, MUSD", "NPV, MUSD"],
        [
            ("Gas side", "1+2+3", 34.0, 4.0, 0.0, ECON["results"]["Gas side"]["npv"]),
            ("Gas side + water", "1+2+3+5", 42.0, 4.0, 0.0, ECON["results"]["Gas side + water"]["npv"]),
            ("HIPPS + water", "1+2+4+5", 36.0, 4.0, 0.0, ECON["results"]["HIPPS + water"]["npv"]),
            ("+ fourth stage", "1+2+3+5+6", 67.3, 5.0, 0.0, ECON["results"]["+ fourth stage"]["npv"]),
            ("+ stabiliser", "1+2+3+5+7", 77.0, 8.0, 10.97, ECON["results"]["+ stabiliser"]["npv"]),
            ("Both add-ons", "1+2+3+5+6+7", 99.0, 8.0, 14.27, ECON["results"]["Both add-ons"]["npv"]),
        ],
        widths=[3.6, 2.8, 2.4, 1.8, 2.8, 3.2],
        caption="Table C-1  Package cost summary",
        align_right=[2, 3, 4, 5],
    )
    para(
        doc,
        "Estimate class: AACE Class 4/5. Accuracy: of the order of −30 %/+50 %. Contingency: none "
        "applied. Escalation: none (year-ten money). Exclusions: detailed design, taxes and duties, "
        "freight and marine spread, owner's costs, spares, decommissioning.",
    )

    doc.add_heading("Appendix D  Project plan task list", level=1)
    add_table(
        doc,
        ["ID", "Task", "Start wk", "Duration wk", "Predecessor", "Owner"],
        [
            (a["id"], a["name"], f"{a['start']:g}", f"{a['duration']:g}", a["predecessors"] or "—", a["discipline"])
            for a in FEED["schedule"]["activities"]
        ],
        widths=[1.2, 6.6, 2.2, 2.2, 2.2, 2.4],
        caption="Table D-1  Task list",
        align_right=[2, 3],
        font_size=8,
    )
    para(
        doc,
        "Durations are planning assumptions except the shutdown tasks (D1–D5), which are the study's "
        "four-, three- and two-week execution durations. The compressor package is the critical path.",
    )

    doc.add_heading("Appendix E  HAZOP worksheet", level=1)
    if HAZ:
        para(
            doc,
            "The worksheet below is the independent team's record. Risk is the team's judgement on the "
            "study design, not a quantified risk assessment. Actions are carried into Section 10.4.",
        )
        add_table(
            doc,
            ["Node", "Guideword / deviation", "Cause", "Consequence", "Safeguard", "S / L / R", "Action"],
            [(r["node"], f'{r["guideword"]}: {r["deviation"]}', r["cause"], r["consequence"], r["safeguard"],
              f'{r["severity"]} / {r["likelihood"]} / {r["risk"]}', r["action"]) for r in HAZ["worksheet"]],
            widths=[1.5, 2.3, 2.9, 3.0, 2.7, 1.1, 2.9],
            caption="Table E-1  HAZOP worksheet (Rev B frozen design)",
            font_size=6.2,
        )
    else:
        para(doc, "The HAZOP worksheet is not yet available in this build.")

    doc.add_heading("Appendix F  References", level=1)
    refs = [
        "azoth process library 0.1.4 — process.separator, three_phase_separator, separator_capacity_assessment, compressor, pump, cooler, throttling_valve, flare, psv_sizing, blocked_outlet_relief, fire_case_relief, relief_disposal_network, equipment_cost_estimate, topsides_facility_cost, nog070_sil_determination, sil_verification, pipeline_pressure_protection, s001_secondary_pressure_protection; eos.pt_flash, ph_flash, pipe_beggs_and_brills.",
        "azoth repository, notebooks/08-offshore-debottlenecking.ipynb — the study this report reproduces.",
        "API STD 520 Part I (2014), Sizing, selection and installation of pressure-relieving devices.",
        "API STD 521 (2020), Pressure-relieving and depressuring systems.",
        "API Spec 12J, Specification for oil and gas separators.",
        "NOG-070 (2020), Application of IEC 61508 and IEC 61511 in the Norwegian petroleum industry.",
        "NORSOK S-001:2020+AC:2021, Technical safety.",
        "IEC 61508-1:2010 and IEC 61511, Functional safety.",
        "ISO 6976:1995, Natural gas — calculation of calorific values.",
        "AACE International Recommended Practice 18R-97, Cost estimate classification system.",
        "Turner, R. et al., Analysis, Synthesis and Design of Chemical Processes, 5th ed., for the module-stack benchmark.",
        "ASME Boiler and Pressure Vessel Code, Section VIII, Division 1 (2023), UG-27 — thickness of shells under internal pressure.",
        "ASME B31.3-2022, Process piping — straight-pipe wall thickness.",
        "NORSOK M-001:2024, Material selection; NORSOK M-506:2017, CO2 corrosion rate calculation model.",
        "ISO 15156 / NACE MR0175, Materials for use in H2S-containing environments (screening check: not triggered).",
        "DEXPI 2.0, Process and Plant exchange specification; the vendored schema is validated by azoth.dexpi.",
        "IEC 61882:2016, Hazard and operability studies (HAZOP) — application guide.",
        "ISO 17776:2016, Offshore production installations — major accident hazard management.",
        "API STD 610 (2021), Centrifugal pumps; API STD 617 (2022), Centrifugal compressors.",
        "ISO 12944-5:2019, Protective paint systems — C5-M corrosivity.",
    ]
    for ref in refs:
        p = doc.add_paragraph(ref, style="List Number")
        p.paragraph_format.space_after = Pt(3)
        for r in p.runs:
            r.font.size = Pt(9)


def build():
    doc = Document()
    style_doc(doc)
    sec = doc.sections[0]
    sec.page_width, sec.page_height = Cm(21.0), Cm(29.7)
    sec.left_margin = Cm(2.0)
    sec.right_margin = Cm(2.0)
    sec.top_margin = Cm(2.2)
    sec.bottom_margin = Cm(2.0)
    header_footer(doc)

    cover(doc)
    doc.add_page_break()
    contents(doc)
    executive_summary(doc)
    introduction(doc)
    basis_of_design(doc)
    diagnosis(doc)
    options(doc)
    scope(doc)
    cost(doc)
    drawings(doc)
    implementation(doc)
    economics(doc)
    hazop(doc)
    risks(doc)
    conclusions(doc)
    appendices(doc)

    OUT.parent.mkdir(exist_ok=True)
    doc.save(OUT)
    print("wrote", OUT)


if __name__ == "__main__":
    build()
