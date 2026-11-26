# AZ-FEED-08 — Offshore debottlenecking FEED case study

**Rev C · HAZOP close-out · 26 November 2026 · illustrative asset · screening study, not for construction**

This package turns [notebook 08](../../notebooks/08-offshore-debottlenecking.ipynb) into a FEED study
and then closes the safety case. Rev B issued the design, equipment, materials, manhours, cost, DEXPI
drawing and plan, and recorded an independent HAZOP. Rev C dispositions all 16 HAZOP actions with
calculations, design changes or purchase-order conditions — and changes the relief basis because the
35 % HIPPS credit could not be proven.

**Rev C supersedes the Rev B action statuses and economics.** The Rev B report remains the design basis;
the close-out report is the decision record.

## How this was produced

This package is the output of one DeepSeek Harness session, run as follow-on work from notebook 08. All
the work took roughly half an hour and roughly 12 million tokens using the azoth DeepSeek Harness.

The prompts were:

1. "create a front end design study based upon the jupyter notebook 8: the debottlenecking study"
2. "the front end engineering design (FEED) study should produce further specification on equipment, materials, manhours and costs, it should produce drawing in dexpi (a process flow diagram at least), an engineering report and an engineering project plan. It should contain all the calculations necessary to justify the engineering conclusions and decisions. When the work is complete it should be subject the an independent HAZOP."
3. "close out what's still open"

The first round closed out in 23 minutes. The close-out prompt then closed the remainder in roughly
7 minutes.

## Headline result

| | Rev B | Rev C (close-out) |
|---|---|---|
| Sales gas | 137.1 → 161.8 MMscfd | 137.1 → 161.8 MMscfd (unchanged) |
| Condensate | 2,936 → ≈3,466 bbl/d | unchanged |
| Relief basis | 35 % HIPPS credit | full +18 % case, flare +25 % |
| SIL-3 loop | 1oo2 PT / single SDV | 1oo2 all, 6-month proof test |
| PSV-101 | 4 in / 6 in | 6 in / 24 in |
| Capex | 36.0 MUSD | **54.1 MUSD** |
| NPV at 10 %, after tax | 24.8 MUSD | **7.8 MUSD** |
| Payback | year 1 | year 2 |

The capacity conclusion is unchanged. The value falls because the project now buys its relief capacity
instead of assuming it.

## Why the close-out changed the answer

- **HAZ-01 — credit withdrawn.** The 35 % credit (50.31 → 32.70 kg/s) depended on HIPPS closure, trapped
  inventory, SDV leakage and flare response that were never modelled. The full case is 107.5 % of the
  existing flare; the +25 % upgrade takes it to 86.0 %.
- **HAZ-02 — SIL 2, not SIL 3.** The issued 1oo2 PT / 1oo1 logic / 1oo1 SDV loop gives PFD 5.0e-03
  (SIL 2). Rev C specifies 1oo2 on all three elements with certified devices and a 6-month proof test
  (PFD 3.3e-05).
- **HAZ-04 — 4 in PSV inlet fails.** The inlet drops 16.6 % of set against the API 520 3 % limit; 6 in
  drops 1.7 %. The 6 in outlet would exceed 2,200 m/s; 24 in gives about 140 m/s.
- **HAZ-16 — dehydration gap.** The gas carries 20.7 kg/MMscf against a 1.36 kg/MMscf specification
  (15×). Rev C carries 139 kg/h of TEG duty, 5.6 m³/h circulation and a 1.5 MUSD upgrade allowance.
- **HAZ-12/14 — let-down protection.** A full-Cv failure passes about 395 kg/s, which no PSV can relieve.
  Rev C adds ESDV-102/103 and 10 mm restrictors; PSV-102 (G), PSV-103 (F) and PSV-104 (F) cover the
  restricted leak.
- **HAZ-05 — radiation.** 1.58 kW/m² contour 148.8 m, 4.73 kW/m² 82.4 m, 6.3 kW/m² 69.8 m; access is
  controlled inside 149 m and the 82.4 m contour is the sterile area.

## Deliverables

| Document | File |
|---|---|
| **HAZOP close-out report (Rev C, 17 pp)** | `report/AZ-FEED-08-HAZOP-Closeout-RevC.pdf` (and `.docx`) |
| **FEED engineering report (Rev B, 57 pp)** | `report/AZ-FEED-08-Offshore-Debottlenecking-Report.pdf` (and `.docx`) |
| DEXPI process flow diagram (A0) | `drawings/AZ-FEED-08-DEXPI-PFD-001.pdf` · `drawings/AZ-FEED-08-DEXPI-PFD-001.xml` |
| Block flow diagram / P&ID / schedule | `drawings/AZ-FEED-08-BFD-001.pdf`, `-PID-001.pdf`, `-SCH-001.svg` |
| Calculation and estimate workbook (16 sheets) | `data/AZ-FEED-08-Workbook.xlsx` |
| Independent HAZOP worksheet | `hazop/worksheet.json` |
| Close-out register (16/16 actions) | `hazop/closeout.json` |
| Close-out calculations | `model/closeout_model.py`, `model/closeout_results.json` |

The issued `AZ-FEED-08-RevC-FEED-Package.zip` is intentionally not committed: it is the same set of
files as this folder and would add roughly 7 MB to the repository. The unpacked folder is the durable
copy.

## Engineering content (Rev B basis, Rev C changes noted)

- **Materials:** 0.732 bar CO2 at 81 °C; carbon steel with 3.0 mm CA and continuous inhibition; 316L for
  produced water, HIPPS and instruments; ISO 12944 C5-M.
- **Pressure containment:** ASME VIII-1 UG-27 (51.25 mm incl. CA, 52 mm selected); B31.3 gas line
  8.34 mm incl. CA (12.7 mm selected), condensate 7.1 mm, water 3.76 mm, HIPPS spool 12.7 mm.
- **Equipment:** 11 datasheets — V-101 internals, K-101 re-rate (3.0 MW driver), HIPPS/SDV-101 (1oo2),
  V-105 hydrocyclone, V-106 coalescer, P-102 (2 × 50 %), PSV-101/102/103/104, flare.
- **Manhours:** 32,600 h (14,200 engineering, 9,600 fabrication, 8,800 offshore); 3.96 MUSD direct labour;
  42-week plan peaking at 2,776 h/week.
- **Cost:** Rev C 54.1 MUSD — Rev B package 36.0, flare +25 % 12.0, 1oo2 SIL 1.2, PSV/ESDV hardware 1.8,
  F&G/ESD 1.0, TEG 1.5, sterile-area 0.3, 3.0 MW driver 0.3.
- **DEXPI:** 31 process steps, 32 streams, schema-valid, drawing embedded; approval
  `NOT_A_DEXPI_EV_CERTIFICATE`, CAE round trip `QUALIFICATION_REQUIRED`.

## Reproduce

```bash
# from this directory, in the repository's Python environment
python model/case_model.py           # notebook-08 reproduction
python model/feed_model.py           # FEED additions
python model/closeout_model.py       # Rev C close-out calculations
python build/make_figures.py
python build/make_feed_figures.py
python build/make_closeout_figures.py
python build/make_drawings.py
python build/make_dexpi.py
python build/render_dexpi.py
python build/make_workbook_feed.py
python build/make_report.py
python build/make_closeout_report.py
soffice --headless --convert-to pdf --outdir report report/AZ-FEED-08-Offshore-Debottlenecking-Report.docx
soffice --headless --convert-to pdf --outdir report report/AZ-FEED-08-HAZOP-Closeout-RevC.docx
```

The DEXPI export needs `azoth` and the palette at `../../specs/unit_ops` from this folder. The
scripts import `numpy`, `matplotlib`, `openpyxl` and `python-docx`; the notebook extra supplies the
first two, the repository's dev environment supplies `openpyxl`, and `python-docx` is not installed.
The report builders also need LibreOffice (`soffice`). `render_dexpi.py` writes the SVG; the issued
DEXPI PDF and PNG are conversions of that SVG, not files the listed commands rewrite.

## Residual conditions (not open actions)

- Vendor SIL certificates for the 1oo2 devices.
- TEG vendor confirmation of 139 kg/h at 3 lb/MMscf and 60 % turndown.
- Sterile-area survey and flare-boom/tip verification.
- Compressor torsional/API 617 reports at purchase order.
- Detailed cause-and-effect and SIMOPS review before the shutdown.
- The dynamic relief study is optional; it can recover the 12 MUSD flare upgrade only through MOC.

## What this package is not

- Not a real asset and not a design deliverable. All azoth results are `partially_verified`.
- Not the owner's HAZOP: independent of the designer, but study-level, not a full IEC 61882 workshop.
- Not a certified DEXPI exchange: semantically valid, not a DEXPI EV certificate.
- Not a full pipeline or dehydration design: the close-out records barriers, duty and conditions.
