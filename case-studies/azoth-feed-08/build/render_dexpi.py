"""Render the DEXPI PFD drawing to an A0 study sheet (SVG), then PNG and PDF.

The geometry comes from the DEXPI Drawing primitives; the sheet furniture (border,
title block, notes, equipment schedule) is added here. No geometry is invented.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
XML_PATH = ROOT / "drawings" / "AZ-FEED-08-DEXPI-PFD-001.xml"
OUT = ROOT / "drawings"
PALETTE = ROOT.parent.parent / "specs" / "unit_ops"

W, H = 1189.0, 841.0          # A0 landscape, mm
M = 20.0

INK = "#111111"
MUTED = "#5a5751"
RULE = "#c9c6bf"
ACCENT = "#2a78d6"


def esc(s: str) -> str:
    return (s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;"))


def text(x, y, s, size=3.2, anchor="start", color=INK, weight="normal", family="Helvetica, Arial, sans-serif", extra=""):
    a = {"start": "start", "middle": "middle", "end": "end"}[anchor]
    return (f'<text x="{x:.2f}" y="{y:.2f}" font-size="{size:.2f}" text-anchor="{a}" '
            f'fill="{color}" font-weight="{weight}" font-family="{family}" {extra}>{esc(s)}</text>')


def line(x1, y1, x2, y2, color=INK, w=0.35, dash=None):
    d = f' stroke-dasharray="{dash}"' if dash else ""
    return f'<line x1="{x1:.2f}" y1="{y1:.2f}" x2="{x2:.2f}" y2="{y2:.2f}" stroke="{color}" stroke-width="{w}"{d}/>'


def rect(x, y, w, h, color=INK, sw=0.4, fill="none"):
    return f'<rect x="{x:.2f}" y="{y:.2f}" width="{w:.2f}" height="{h:.2f}" fill="{fill}" stroke="{color}" stroke-width="{sw}"/>'


FRIENDLY = {
    "three_phase_separator": "3-phase separator",
    "separator": "2-phase separator",
    "compressor": "compressor",
    "cooler": "cooler",
    "throttling_valve": "throttling valve",
    "mixer": "mixer",
}

SERVICE = {
    "V-101": "HP 3-phase separator",
    "K-101": "HP gas compressor",
    "E-101": "HP gas aftercooler",
    "VLV-102": "HP oil let-down valve",
    "V-102": "MP separator",
    "VLV-103": "MP liquid let-down valve",
    "V-103": "LP separator",
    "E-104": "LP liquid cooler",
    "V-104": "Stock tank / condensate flash",
    "M-101": "Sales gas mixer",
}


def service_of(tag: str) -> str:
    if tag in SERVICE:
        return SERVICE[tag]
    m = re.match(r"K-102([A-C])", tag)
    if m:
        return f"MP gas compressor, stage {m.group(1)}"
    m = re.match(r"E-102([A-C])", tag)
    if m:
        return f"MP inter/aftercooler {m.group(1)}"
    m = re.match(r"K-103([A-E])", tag)
    if m:
        return f"LP gas compressor, stage {m.group(1)}"
    m = re.match(r"E-103([A-E])", tag)
    if m:
        return f"LP inter/aftercooler {m.group(1)}"
    return ""


def main():
    import azoth.dexpi as dexpi

    xml = XML_PATH.read_text()
    # Shorten the stream labels the exporter writes: "source · 1234.5 kg/h" -> "S1: 1,235 kg/h"
    stream_index = []
    counter = {"n": 0}

    def shorten(match):
        counter["n"] += 1
        src, mass = match.group(1), float(match.group(2))
        stream_index.append({"stream": f"S{counter['n']}", "source": src, "kg_per_h": mass})
        return f"S{counter['n']}"

    pat = re.compile(r"([A-Za-z0-9_.\-]+) · ([0-9.]+) kg/h")
    xml = pat.sub(shorten, xml)
    XML_PATH.write_text(xml)
    (OUT / "AZ-FEED-08-DEXPI-PFD-001.streams.json").write_text(json.dumps(stream_index, indent=2))

    validation = dexpi.validate(xml)
    view = dexpi.view(xml, str(PALETTE))
    drawing = view["drawing"]
    b = drawing["bounds"]
    minx, maxx, miny, maxy = b["min_x"], b["max_x"], b["min_y"], b["max_y"]
    dw, dh = maxx - minx, maxy - miny

    top = M + 4
    avail_w = W - 2 * M
    drawing_area_h = 300.0
    s = min(avail_w / dw, drawing_area_h / dh)
    ox = M + (avail_w - dw * s) / 2 - minx * s
    oy = top + (drawing_area_h - dh * s) / 2 + maxy * s

    def sx(x):
        return ox + x * s

    def sy(y):
        return oy - y * s

    p = []
    p.append(f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}mm" height="{H}mm" viewBox="0 0 {W} {H}">')
    p.append(f'<rect width="{W}" height="{H}" fill="#ffffff"/>')
    p.append(rect(M - 4, M - 4, W - 2 * M + 8, H - 2 * M + 8, INK, 0.7))
    p.append(rect(M, M, W - 2 * M, H - 2 * M, INK, 0.35))

    # ---- drawing geometry -------------------------------------------------
    p.append('<g id="dexpi-drawing">')
    for group in drawing["groups"]:
        for prim in group["primitives"]:
            kind = prim["kind"]
            if kind == "polyline":
                pts = " ".join(f"{sx(q['x']):.2f},{sy(q['y']):.2f}" for q in prim["points"])
                st = prim.get("stroke", {})
                col = st.get("color", {"r": 0, "g": 0, "b": 0})
                colour = f"rgb({col['r']},{col['g']},{col['b']})"
                p.append(f'<polyline points="{pts}" fill="none" stroke="{colour}" stroke-width="{max(0.45, st.get("width", 1.0) * s * 1.35):.2f}" stroke-linejoin="round"/>')
            elif kind == "polygon":
                pts = " ".join(f"{sx(q['x']):.2f},{sy(q['y']):.2f}" for q in prim["points"])
                st = prim.get("stroke", {})
                col = st.get("color", {"r": 0, "g": 0, "b": 0})
                colour = f"rgb({col['r']},{col['g']},{col['b']})"
                fill = colour if prim.get("fill") == "Solid" else ("#ffffff" if prim.get("fill") == "Transparent" else "none")
                p.append(f'<polygon points="{pts}" fill="{fill}" stroke="{colour}" stroke-width="{max(0.45, st.get("width", 1.0) * s * 1.35):.2f}" stroke-linejoin="round"/>')
            elif kind == "text":
                content = prim.get("text", "")
                content = FRIENDLY.get(content, content)
                size = prim.get("size", 7.0) * s
                pos = prim["position"]
                col = prim.get("color", {"r": 0, "g": 0, "b": 0})
                colour = f"rgb({col['r']},{col['g']},{col['b']})"
                anchor = {"CenterCenter": "middle", "LeftCenter": "start", "RightCenter": "end"}.get(prim.get("alignment"), "middle")
                dy = size * 0.35
                p.append(text(sx(pos["x"]), sy(pos["y"]) + dy, content, size=size, anchor=anchor, color=colour))
    p.append("</g>")

    # ---- notes -------------------------------------------------------------
    ty = 360.0
    p.append(text(M + 2, ty, "NOTES", 4.2, weight="bold"))
    notes = [
        "1. Design case: year ten, 2,230 mol/s wet wellstream at 60 bara and 354.15 K (13.45 mol % water).",
        "2. Recommended scope: HIPPS + produced-water upgrade with compressor re-rate and separator internals.",
        "3. Sales gas 161.8 MMscfd at 90 bara; condensate about 3,470 bbl/d; produced water about 2,782 bbl/d.",
        "4. Compression is staged at equal ratio no larger than 2.5 with intercooling to 313.15 K.",
        "5. Relief case, HIPPS credit, PSV-101 and the flare are shown on P&ID AZ-FEED-08-PID-001.",
        "6. This drawing is generated from the DEXPI 2.0 process model in the same file set; tags per equipment list.",
        "7. AACE Class 4/5 screening study. Not for construction. All azoth results are partially verified.",
    ]
    for i, note in enumerate(notes):
        p.append(text(M + 2, ty + 9 + i * 6.2, note, 2.9, color=INK))

    # ---- equipment schedule ------------------------------------------------
    ex, ey = 620.0, 360.0
    p.append(text(ex, ey, "EQUIPMENT SCHEDULE", 4.2, weight="bold"))
    tags = [g for g in drawing["groups"] if g["kind"] == "node"]
    items = []
    for g in tags:
        texts = [q.get("text", "") for q in g["primitives"] if q["kind"] == "text"]
        if len(texts) >= 2 and texts[1] in FRIENDLY:
            items.append((texts[0], FRIENDLY[texts[1]]))
    items.sort(key=lambda t: (t[0].split("-")[0], t[0]))
    col_w = 270.0
    row_h = 5.6
    p.append(line(ex, ey + 2.5, ex + 2 * col_w, ey + 2.5, INK, 0.4))
    for i, (tag, kind) in enumerate(items):
        cx = ex + (i % 2) * col_w
        cy = ey + 8 + (i // 2) * row_h
        p.append(text(cx, cy, tag, 2.9, weight="bold"))
        p.append(text(cx + 58, cy, service_of(tag) or kind, 2.7, color=MUTED))
    rows = (len(items) + 1) // 2
    p.append(line(ex, ey + 8 + rows * row_h, ex + 2 * col_w, ey + 8 + rows * row_h, INK, 0.4))

    # ---- key stream table --------------------------------------------------
    sx0, sy0 = M + 2, 470.0
    p.append(text(sx0, sy0, "KEY STREAMS", 4.2, weight="bold"))
    p.append(text(sx0 + 300, sy0, "STREAM SUMMARY (continued)", 4.2, weight="bold"))
    p.append(line(sx0, sy0 + 2.5, sx0 + 540, sy0 + 2.5, INK, 0.4))
    p.append(line(sx0 + 300, sy0 + 2.5, sx0 + 840, sy0 + 2.5, INK, 0.4))
    for i, srow in enumerate(stream_index):
        col = 0 if i < 16 else 1
        cx = sx0 + col * 300
        cy = sy0 + 8 + (i % 16) * 4.8
        p.append(text(cx, cy, srow["stream"], 2.8, weight="bold"))
        p.append(text(cx + 16, cy, srow["source"][:22], 2.6, color=MUTED))
        p.append(text(cx + 235, cy, f"{srow['kg_per_h']:,.0f}", 2.6, anchor="end"))
    p.append(line(sx0, sy0 + 8 + 16 * 4.8, sx0 + 540, sy0 + 8 + 16 * 4.8, INK, 0.4))
    p.append(line(sx0 + 300, sy0 + 8 + 16 * 4.8, sx0 + 840, sy0 + 8 + 16 * 4.8, INK, 0.4))

    # ---- title block -------------------------------------------------------
    tw, th = 330.0, 112.0
    tx, tyy = W - M - tw, H - M - th
    p.append(rect(tx, tyy, tw, th, INK, 0.6, "#ffffff"))
    p.append(line(tx, tyy + 26, tx + tw, tyy + 26, INK, 0.4))
    p.append(line(tx, tyy + 52, tx + tw, tyy + 52, INK, 0.4))
    p.append(line(tx, tyy + 78, tx + tw, tyy + 78, INK, 0.4))
    p.append(line(tx + 190, tyy + 52, tx + 190, tyy + 78, INK, 0.4))
    p.append(text(tx + 4, tyy + 7, "azoth · FEED case study", 2.8, color=MUTED))
    p.append(text(tx + 4, tyy + 18, "PROCESS FLOW DIAGRAM", 5.6, weight="bold"))
    p.append(text(tx + 4, tyy + 36, "Debottlenecked separation and compression train", 3.4))
    p.append(text(tx + 4, tyy + 47, "Offshore platform 60/10/2 · year ten · recommended scope", 2.9, color=MUTED))
    p.append(text(tx + 4, tyy + 64, "AZ-FEED-08-DEXPI-PFD-001", 3.6, weight="bold"))
    p.append(text(tx + 4, tyy + 73, "DEXPI 2.0 Process exchange · drawing embedded", 2.7, color=MUTED))
    p.append(text(tx + 196, tyy + 64, "Rev", 2.7, color=MUTED))
    p.append(text(tx + 196, tyy + 73, "B", 4.2, weight="bold"))
    p.append(text(tx + 4, tyy + 89, "Scale", 2.7, color=MUTED))
    p.append(text(tx + 30, tyy + 89, f"1:{1/s:.1f}", 3.2))
    p.append(text(tx + 78, tyy + 89, "Sheet", 2.7, color=MUTED))
    p.append(text(tx + 100, tyy + 89, "1 of 1", 3.2))
    p.append(text(tx + 150, tyy + 89, "Date", 2.7, color=MUTED))
    p.append(text(tx + 172, tyy + 89, "2026-11-26", 3.2))
    p.append(text(tx + 4, tyy + 104, "Status: Issued for FEED study · not for construction", 3.0, weight="bold"))
    p.append(text(tx + 210, tyy + 104, "Checked / Approved", 2.6, color=MUTED))

    # revision strip
    p.append(text(M + 2, H - M - 2, "Rev A · conceptual selection.  Rev B · DEXPI PFD, materials, manhours and HAZOP added.", 2.7, color=MUTED))
    p.append("</svg>")
    svg = "\n".join(p)
    (OUT / "AZ-FEED-08-DEXPI-PFD-001.svg").write_text(svg)
    print(json.dumps({"valid": validation, "scale": round(s, 4), "bounds": b,
                      "items": len(items), "streams": len(stream_index)}, indent=1))


if __name__ == "__main__":
    main()
