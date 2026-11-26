"""Draw the FEED case study's drawings: BFD and simplified P&ID (A3).

Outputs (drawings/):
  AZ-FEED-08-BFD-001.png/.svg/.pdf   block flow diagram
  AZ-FEED-08-PID-001.png/.svg/.pdf   simplified P&ID, A3 landscape
"""

from __future__ import annotations

import json
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.patches import Circle, FancyBboxPatch, Polygon, Rectangle

ROOT = Path(__file__).resolve().parents[1]
RES = json.loads((ROOT / "model" / "results.json").read_text())
OUT = ROOT / "drawings"
OUT.mkdir(exist_ok=True)

INK = "#111111"
MUTED = "#55534f"
BLUE = "#2a78d6"
ORANGE = "#eb6834"
GREEN = "#1baf7a"
AMBER = "#eda100"
PAPER = "#fdfdfb"

A3 = (16.54, 11.69)


def canvas(size=A3):
    fig = plt.figure(figsize=size, facecolor=PAPER)
    ax = fig.add_axes([0, 0, 1, 1])
    ax.set_xlim(0, size[0])
    ax.set_ylim(0, size[1])
    ax.set_aspect("equal")
    ax.axis("off")
    return fig, ax


def line(ax, pts, lw=1.5, color=INK, ls="-", z=2):
    xs, ys = zip(*pts)
    ax.add_line(Line2D(xs, ys, lw=lw, color=color, linestyle=ls, zorder=z, solid_capstyle="round"))


def arrowhead(ax, x, y, dx, dy, size=0.11, color=INK):
    import math

    ang = math.atan2(dy, dx)
    left = (x - size * math.cos(ang - 0.45), y - size * math.sin(ang - 0.45))
    right = (x - size * math.cos(ang + 0.45), y - size * math.sin(ang + 0.45))
    ax.add_patch(Polygon([(x, y), left, right], closed=True, color=color, zorder=3, lw=0))


def txt(ax, x, y, s, size=7, ha="center", va="center", color=INK, weight="normal", rotation=0, z=6):
    ax.text(x, y, s, fontsize=size, ha=ha, va=va, color=color, weight=weight, rotation=rotation, zorder=z)


def vessel_h(ax, x0, y0, w, h, color=INK, fill="#eef4fb", lw=1.5):
    ax.add_patch(
        FancyBboxPatch(
            (x0, y0), w, h,
            boxstyle=f"round,pad=0,rounding_size={h * 0.35}",
            linewidth=lw, edgecolor=color, facecolor=fill, zorder=2,
        )
    )


def vessel_v(ax, x0, y0, w, h, color=INK, fill="#eef4fb", lw=1.5):
    ax.add_patch(
        FancyBboxPatch(
            (x0, y0), w, h,
            boxstyle=f"round,pad=0,rounding_size={w * 0.35}",
            linewidth=lw, edgecolor=color, facecolor=fill, zorder=2,
        )
    )


def valve(ax, x, y, size=0.13, vertical=False):
    if vertical:
        ax.add_patch(Polygon([(x - size, y + size), (x, y), (x - size, y - size)], closed=True,
                             facecolor="white", edgecolor=INK, lw=1.3, zorder=3))
        ax.add_patch(Polygon([(x + size, y + size), (x, y), (x + size, y - size)], closed=True,
                             facecolor="white", edgecolor=INK, lw=1.3, zorder=3))
    else:
        ax.add_patch(Polygon([(x - size, y + size), (x, y), (x - size, y - size)], closed=True,
                             facecolor="white", edgecolor=INK, lw=1.3, zorder=3))
        ax.add_patch(Polygon([(x + size, y + size), (x, y), (x + size, y - size)], closed=True,
                             facecolor="white", edgecolor=INK, lw=1.3, zorder=3))


def pump(ax, x, y, r=0.2):
    ax.add_patch(Circle((x, y), r, facecolor="white", edgecolor=INK, lw=1.4, zorder=3))
    ax.add_patch(Polygon([(x - r * 0.45, y - r * 0.55), (x + r * 0.45, y), (x - r * 0.45, y + r * 0.55)],
                         closed=True, facecolor=INK, edgecolor=INK, zorder=4))


def bubble(ax, x, y, func, number, r=0.26):
    ax.add_patch(Circle((x, y), r, facecolor="white", edgecolor=INK, lw=1.1, zorder=5))
    txt(ax, x, y + 0.075, func, size=5.6, weight="bold")
    txt(ax, x, y - 0.085, number, size=5.4)


def signal(ax, pts, color=MUTED):
    line(ax, pts, lw=0.9, color=color, ls=(0, (2.5, 2.0)), z=4)


def title_block(ax, dwg_no, title, sheet, notes=None):
    x0, y0, w, h = 11.55, 0.35, 4.65, 2.55
    ax.add_patch(Rectangle((x0, y0), w, h, facecolor="white", edgecolor=INK, lw=1.2, zorder=5))
    for yy in (y0 + 0.55, y0 + 1.05, y0 + 1.55, y0 + 2.0):
        line(ax, [(x0, yy), (x0 + w, yy)], lw=0.6, color=MUTED, z=6)
    line(ax, [(x0 + 2.35, y0), (x0 + 2.35, y0 + 1.55)], lw=0.6, color=MUTED, z=6)
    line(ax, [(x0 + 3.55, y0), (x0 + 3.55, y0 + 1.05)], lw=0.6, color=MUTED, z=6)
    txt(ax, x0 + 0.12, y0 + 2.32, "azoth · FEED case study", size=6.2, ha="left", color=MUTED)
    txt(ax, x0 + 0.12, y0 + 2.12, title, size=7.2, ha="left", weight="bold")
    txt(ax, x0 + 0.12, y0 + 1.78, "Offshore debottlenecking — Platform 60/10/2, year ten", size=6.2, ha="left")
    txt(ax, x0 + 0.12, y0 + 1.28, "DWG NO", size=4.8, ha="left", color=MUTED)
    txt(ax, x0 + 0.12, y0 + 1.12, dwg_no, size=7.0, ha="left", weight="bold")
    txt(ax, x0 + 2.48, y0 + 1.28, "REV", size=4.8, ha="left", color=MUTED)
    txt(ax, x0 + 2.48, y0 + 1.12, "A", size=7.0, ha="left", weight="bold")
    txt(ax, x0 + 3.68, y0 + 1.28, "SHEET", size=4.8, ha="left", color=MUTED)
    txt(ax, x0 + 3.68, y0 + 1.12, sheet, size=7.0, ha="left", weight="bold")
    txt(ax, x0 + 0.12, y0 + 0.78, "SCALE", size=4.8, ha="left", color=MUTED)
    txt(ax, x0 + 0.12, y0 + 0.62, "NTS", size=6.5, ha="left")
    txt(ax, x0 + 2.48, y0 + 0.78, "DATE", size=4.8, ha="left", color=MUTED)
    txt(ax, x0 + 2.48, y0 + 0.62, "2026-11-26", size=6.5, ha="left")
    txt(ax, x0 + 3.68, y0 + 0.78, "STATUS", size=4.8, ha="left", color=MUTED)
    txt(ax, x0 + 3.68, y0 + 0.62, "FEED", size=6.5, ha="left")
    txt(ax, x0 + 0.12, y0 + 0.3,
        "Study drawing — illustrative data — not for construction. All values are screening results.",
        size=5.2, ha="left", color=ORANGE)
    txt(ax, x0 + 0.12, y0 + 0.13, "azoth process library 0.1.4 · values partially verified", size=5.0, ha="left", color=MUTED)


def legend(ax, x0=0.4, y0=0.4):
    w, h = 4.0, 2.1
    ax.add_patch(Rectangle((x0, y0), w, h, facecolor="white", edgecolor=MUTED, lw=0.8, zorder=5))
    txt(ax, x0 + 0.15, y0 + h - 0.22, "LEGEND", size=6.8, ha="left", weight="bold")
    y = y0 + h - 0.55
    line(ax, [(x0 + 0.2, y), (x0 + 0.9, y)], lw=1.5)
    txt(ax, x0 + 1.0, y, "process line", size=5.8, ha="left")
    y -= 0.28
    line(ax, [(x0 + 0.2, y), (x0 + 0.9, y)], lw=0.9, ls=(0, (2.5, 2.0)), color=MUTED)
    txt(ax, x0 + 1.0, y, "instrument signal (electric)", size=5.8, ha="left")
    y -= 0.28
    valve(ax, x0 + 0.55, y, size=0.1)
    txt(ax, x0 + 1.0, y, "block valve / SDV", size=5.8, ha="left")
    y -= 0.28
    bubble(ax, x0 + 0.55, y, "PT", "101", r=0.15)
    txt(ax, x0 + 1.0, y, "transmitter / switch", size=5.8, ha="left")
    y -= 0.28
    pump(ax, x0 + 0.55, y, r=0.14)
    txt(ax, x0 + 1.0, y, "pump", size=5.8, ha="left")
    y -= 0.28
    line(ax, [(x0 + 0.2, y), (x0 + 0.9, y)], lw=1.6, color=GREEN)
    txt(ax, x0 + 1.0, y, "new / modified scope (Rev A)", size=5.8, ha="left")


# ---------------------------------------------------------------- BFD
def draw_bfd():
    fig, ax = canvas((13.2, 6.9))
    txt(ax, 0.3, 6.55, "BLOCK FLOW DIAGRAM — YEAR-TEN BASE CASE AND DEBOTTLENECK SCOPE", size=10, ha="left", weight="bold")
    txt(ax, 0.3, 6.28, "Platform 60/10/2 · 137.1 MMscfd sales gas · 2,936 bbl/d condensate · 2,782 bbl/d produced water",
        size=6.8, ha="left", color=MUTED)

    boxes = {
        "wells": (0.3, 4.55, 1.55, 1.0, "WELLS & MANIFOLD", "2,230 mol/s feed\n95 bara · 363 K"),
        "hp": (2.55, 4.55, 1.85, 1.0, "V-101 HP SEPARATOR", "3-phase · 60 bara\n1,869 mol/s gas"),
        "mp": (2.55, 3.0, 1.85, 0.85, "V-102 MP SEPARATOR", "10 bara"),
        "lp": (2.55, 1.55, 1.85, 0.85, "V-103 LP SEPARATOR", "2 bara"),
        "comp": (5.35, 4.55, 2.15, 1.0, "K-101 EXPORT COMPRESSION", "2 stages · 2.26 MW\n90 bara · 313 K"),
        "sales": (8.6, 4.55, 1.9, 1.0, "SALES GAS", "137.1 MMscfd\n76% of line cap."),
        "water": (2.55, 0.25, 2.3, 0.9, "PRODUCED-WATER PACKAGE", "3,000 → 6,000 bbl/d\n+ coalescer (new)"),
        "tank": (5.35, 1.55, 1.75, 0.85, "STOCK TANK", "2,936 bbl/d\n21.4 bbl/MMscf"),
        "flare": (8.6, 2.5, 1.9, 0.9, "FLARE & HEADER", "91% loaded now\n107% at +18%"),
        "liquid": (8.6, 1.55, 1.9, 0.7, "CONDENSATE EXPORT", "25 bara · 15% line use"),
    }
    for key, (x, y, w, h, title, sub) in boxes.items():
        fill = "#e8f3ee" if key in {"water", "tank"} else "#eef4fb"
        edge = GREEN if key in {"water", "tank"} else INK
        ax.add_patch(FancyBboxPatch((x, y), w, h, boxstyle="round,pad=0,rounding_size=0.08",
                                    facecolor=fill, edgecolor=edge, lw=1.3, zorder=2))
        txt(ax, x + w / 2, y + h - 0.2, title, size=6.4, weight="bold")
        txt(ax, x + w / 2, y + h / 2 - 0.16, sub, size=5.6, color=MUTED)

    def stream(x1, y1, x2, y2, label=None, lx=0, ly=0, color=INK, size=5.4):
        line(ax, [(x1, y1), (x2, y2)], lw=1.3, color=color)
        arrowhead(ax, x2, y2, x2 - x1, y2 - y1, color=color)
        if label:
            txt(ax, (x1 + x2) / 2 + lx, (y1 + y2) / 2 + ly, label, size=size, color=MUTED)

    stream(1.85, 5.05, 2.55, 5.05, "2,230 mol/s", ly=0.14)
    stream(4.4, 5.25, 5.35, 5.25, "1,869 mol/s gas", ly=0.14)
    stream(7.5, 5.05, 8.6, 5.05, "137.1 MMscfd", ly=0.14)
    stream(3.45, 4.55, 3.45, 3.85, "76.6 mol/s oil", lx=0.1, ly=0)
    stream(3.45, 3.0, 3.45, 2.4, "53.2 mol/s", lx=0.1, ly=0)
    stream(2.55, 4.8, 2.15, 4.8, "", color=INK)
    line(ax, [(2.15, 4.8), (2.15, 0.7), (2.55, 0.7)], lw=1.3)
    arrowhead(ax, 2.55, 0.7, 1, 0, color=INK)
    txt(ax, 2.28, 2.2, "284 mol/s\n2,782 bbl/d", size=5.2, color=MUTED, rotation=90)
    stream(4.4, 1.8, 5.35, 1.8, "45.7 mol/s", ly=0.14)
    stream(7.1, 1.8, 8.6, 1.8, "2,936 bbl/d", ly=0.14)
    line(ax, [(3.45, 4.55), (3.45, 4.2)], lw=0)
    # relief from HP gas header
    line(ax, [(4.75, 5.55), (4.75, 5.95), (8.6, 5.95), (8.6, 3.4)], lw=1.2, color=ORANGE)
    arrowhead(ax, 8.6, 3.4, 0, -1, color=ORANGE)
    txt(ax, 5.9, 6.08, "relief: 42.6 kg/s blocked outlet", size=5.4, color=ORANGE)
    # MP/LP gas to compression
    line(ax, [(4.4, 3.4), (4.95, 3.4), (4.95, 5.15), (5.35, 5.15)], lw=1.2)
    arrowhead(ax, 5.35, 5.15, 1, 0)
    line(ax, [(4.4, 1.95), (4.95, 1.95)], lw=1.2)
    line(ax, [(4.95, 1.95), (4.95, 5.15)], lw=1.2)
    txt(ax, 4.72, 3.9, "23.5 + 7.5 mol/s", size=5.0, color=MUTED, rotation=90)
    # labels for new scope
    txt(ax, 4.0, 1.35, "NEW", size=5.6, color=GREEN, weight="bold")
    txt(ax, 6.85, 3.55, "ENABLER: HIPPS or flare upgrade", size=5.2, color=GREEN, ha="left")
    txt(ax, 5.35, 4.34, "RE-RATE +18%", size=5.6, color=GREEN, weight="bold")
    txt(ax, 2.55, 4.36, "INTERNALS K 0.11→0.14", size=5.4, color=GREEN, weight="bold")
    txt(ax, 8.6, 2.32, "NEW SCOPE (green)", size=5.2, color=GREEN, ha="left")
    fig.savefig(OUT / "AZ-FEED-08-BFD-001.png", dpi=220, facecolor=PAPER, bbox_inches="tight")
    fig.savefig(OUT / "AZ-FEED-08-BFD-001.svg", facecolor=PAPER, bbox_inches="tight")
    fig.savefig(OUT / "AZ-FEED-08-BFD-001.pdf", facecolor=PAPER, bbox_inches="tight")
    plt.close(fig)


# ---------------------------------------------------------------- P&ID
def draw_pid():
    fig, ax = canvas(A3)
    txt(ax, 0.4, 11.45, "SIMPLIFIED PROCESS & INSTRUMENTATION DIAGRAM", size=11.5, ha="left", weight="bold")
    txt(ax, 0.4, 11.2, "HP separation, relief, export compression and produced water — year-ten debottleneck scope",
        size=7.2, ha="left", color=MUTED)

    # ---- inlet and HIPPS
    line(ax, [(0.35, 9.55), (2.62, 9.55)], lw=2.2)
    txt(ax, 0.4, 9.3, "FROM WELLHEAD / TEST MANIFOLD", size=5.8, ha="left", weight="bold")
    txt(ax, 0.4, 9.14, "95 bara · 363 K · 2,230 mol/s", size=5.6, ha="left", color=MUTED)
    # SDV-101
    valve(ax, 2.78, 9.55, size=0.16, vertical=True)
    txt(ax, 2.78, 9.9, "SDV-101", size=6.2, weight="bold")
    txt(ax, 2.78, 9.2, "SIL 3 · 1oo2", size=5.4, color=MUTED)
    line(ax, [(2.94, 9.55), (4.05, 9.55)], lw=2.2)
    # HIPPS transmitters + logic solver
    bubble(ax, 1.75, 10.05, "PT", "101A")
    bubble(ax, 2.3, 9.0, "PT", "101B")
    signal(ax, [(1.75, 10.31), (1.75, 10.65), (2.1, 10.65)])
    signal(ax, [(2.3, 8.74), (2.3, 8.5), (1.1, 8.5), (1.1, 10.45)])
    ax.add_patch(FancyBboxPatch((2.1, 10.45), 2.3, 0.42, boxstyle="round,pad=0,rounding_size=0.05",
                                facecolor="white", edgecolor=INK, lw=1.2, zorder=5))
    txt(ax, 3.25, 10.75, "HIPPS LOGIC SOLVER", size=5.8, weight="bold")
    txt(ax, 3.25, 10.57, "SIL 3 · PFD 1e-4 · proof test 12 mo", size=5.2, color=MUTED)
    signal(ax, [(4.4, 10.66), (4.65, 10.66), (4.65, 10.2), (2.78, 10.2), (2.78, 9.72)])
    txt(ax, 4.72, 10.2, "trip", size=5.0, color=MUTED, ha="left")

    # ---- V-101
    vessel_h(ax, 4.05, 9.05, 2.05, 0.95)
    txt(ax, 5.08, 9.86, "V-101  HP 3-PHASE SEPARATOR", size=6.6, weight="bold")
    txt(ax, 5.08, 9.42, "60 bara · 354 K", size=5.8)
    txt(ax, 5.08, 9.24, "Ø 1.96 m × 7.26 m T/T", size=5.4, color=MUTED)
    txt(ax, 5.08, 9.72, "gas load 99.8%", size=5.2, color=ORANGE)
    # nozzles
    line(ax, [(4.05, 9.55), (3.94, 9.55)], lw=2.2)
    line(ax, [(6.1, 9.7), (6.1, 9.55)], lw=0)
    # gas outlet
    line(ax, [(6.1, 9.62), (6.85, 9.62)], lw=2.2)
    line(ax, [(6.85, 9.62), (9.35, 9.62)], lw=2.2)
    txt(ax, 8.25, 9.75, "HP gas 1,869 mol/s", size=5.6, color=MUTED)
    # oil outlet + LIC
    line(ax, [(5.5, 9.05), (5.5, 8.02)], lw=1.8)
    valve(ax, 5.5, 8.22, size=0.12)
    txt(ax, 4.92, 8.22, "LV-101", size=5.4, weight="bold", ha="right")
    bubble(ax, 6.35, 8.5, "LIC", "101")
    signal(ax, [(6.35, 8.24), (6.35, 8.22), (5.63, 8.22)])
    line(ax, [(5.5, 8.02), (6.4, 8.02)], lw=1.8)
    line(ax, [(6.4, 8.02), (6.4, 7.72)], lw=1.8)
    # water outlet + LIC
    line(ax, [(4.55, 9.05), (4.55, 8.55)], lw=1.8)
    valve(ax, 4.55, 8.4, size=0.11, vertical=True)
    txt(ax, 4.05, 8.54, "LCV-102", size=5.2, ha="right")
    bubble(ax, 3.95, 8.75, "LIC", "102")
    signal(ax, [(3.95, 8.49), (3.95, 8.4), (4.43, 8.4)])
    line(ax, [(4.55, 8.55), (4.55, 7.62)], lw=1.8)
    line(ax, [(4.55, 7.62), (2.9, 7.62)], lw=1.8)
    arrowhead(ax, 2.9, 7.62, -1, 0, size=0.11)
    txt(ax, 4.6, 7.72, "produced water 284 mol/s · 2,782 bbl/d", size=5.2, color=MUTED, ha="left")

    # ---- V-102
    vessel_h(ax, 6.4, 7.35, 1.25, 0.7)
    txt(ax, 7.03, 7.9, "V-102  MP SEPARATOR", size=6.0, weight="bold")
    txt(ax, 7.03, 7.6, "10 bara · 2-phase", size=5.4)
    txt(ax, 7.03, 7.45, "Ø 1.61 × 8.03 m", size=5.0, color=MUTED)
    line(ax, [(6.4, 7.7), (6.4, 7.72)], lw=0)
    # MP gas up to suction header
    line(ax, [(7.65, 7.82), (8.15, 7.82)], lw=1.6)
    line(ax, [(8.15, 7.82), (8.15, 9.35)], lw=1.6)
    line(ax, [(8.15, 9.35), (9.35, 9.35)], lw=1.6)
    arrowhead(ax, 9.35, 9.35, 1, 0, size=0.1)
    txt(ax, 7.95, 8.9, "MP gas 23.5 mol/s", size=5.0, color=MUTED, rotation=90)
    # MP liquid to V-103
    line(ax, [(7.25, 7.35), (7.25, 6.3)], lw=1.6)
    valve(ax, 7.25, 6.55, size=0.11)
    txt(ax, 6.9, 6.55, "LV-102", size=5.2, ha="right")
    line(ax, [(7.25, 6.3), (8.45, 6.3)], lw=1.6)
    txt(ax, 7.7, 6.42, "MP liquid 53.2 mol/s", size=5.0, color=MUTED)

    # ---- V-103
    vessel_h(ax, 8.45, 6.02, 1.15, 0.62)
    txt(ax, 9.02, 6.84, "V-103  LP SEPARATOR", size=6.0, weight="bold")
    txt(ax, 9.02, 6.5, "2 bara · 2-phase", size=5.4)
    txt(ax, 9.02, 6.35, "Ø 1.53 × 7.64 m", size=5.0, color=MUTED)
    # LP gas up
    line(ax, [(9.6, 6.42), (9.95, 6.42)], lw=1.5)
    line(ax, [(9.95, 6.42), (9.95, 9.35)], lw=1.5)
    line(ax, [(9.95, 9.35), (9.35, 9.35)], lw=1.5)
    txt(ax, 10.08, 7.5, "LP gas 7.5 mol/s", size=5.0, color=MUTED, rotation=90)
    # liquid to pump
    line(ax, [(9.05, 6.02), (9.05, 5.55)], lw=1.8)
    pump(ax, 9.05, 5.3, r=0.2)
    txt(ax, 9.05, 4.94, "P-101", size=5.4, weight="bold")
    line(ax, [(9.05, 5.1), (9.05, 4.75)], lw=1.8)
    line(ax, [(9.05, 4.75), (12.2, 4.75)], lw=1.8)
    arrowhead(ax, 12.2, 4.75, 1, 0, size=0.11)
    txt(ax, 11.2, 4.9, "condensate 2,936 bbl/d · 25 bara", size=5.4, color=MUTED)

    # ---- compression
    ax.add_patch(FancyBboxPatch((9.45, 9.15), 1.6, 0.95, boxstyle="round,pad=0,rounding_size=0.08",
                                facecolor="#eef4fb", edgecolor=INK, lw=1.5, zorder=2))
    txt(ax, 10.25, 9.95, "K-101", size=6.6, weight="bold")
    txt(ax, 10.25, 9.75, "EXPORT COMPRESSOR", size=5.6)
    txt(ax, 10.25, 9.55, "2 stages · 2.26 MW", size=5.2)
    txt(ax, 10.25, 9.36, "90 bara · 313 K", size=5.2, color=MUTED)
    txt(ax, 10.25, 9.2, "98% of driver", size=5.0, color=ORANGE)
    line(ax, [(9.35, 9.62), (9.45, 9.62)], lw=2.2)
    line(ax, [(11.05, 9.62), (11.45, 9.62)], lw=2.2)
    # cooler
    ax.add_patch(Rectangle((11.45, 9.42), 0.55, 0.4, facecolor="white", edgecolor=INK, lw=1.3, zorder=3))
    txt(ax, 11.72, 9.62, "E-101", size=5.2, weight="bold")
    txt(ax, 11.72, 9.25, "aftercooler", size=5.0, color=MUTED)
    bubble(ax, 11.72, 10.2, "TIC", "101")
    signal(ax, [(11.72, 9.94), (11.72, 9.82)])
    line(ax, [(12.0, 9.62), (12.35, 9.62)], lw=2.2)
    # metering
    bubble(ax, 12.55, 10.15, "FIT", "101")
    line(ax, [(12.55, 9.89), (12.55, 9.62)], lw=0.9, ls=(0, (2.5, 2.0)), color=MUTED)
    line(ax, [(12.35, 9.62), (15.2, 9.62)], lw=2.2)
    arrowhead(ax, 15.2, 9.62, 1, 0, size=0.13)
    txt(ax, 15.25, 9.62, "EXPORT\nGAS", size=6.2, ha="left", weight="bold")
    txt(ax, 15.25, 9.25, "137.1 MMscfd\n90 bara", size=5.2, ha="left", color=MUTED)
    txt(ax, 13.2, 9.74, "sales gas", size=5.4, color=MUTED)

    # ---- relief
    line(ax, [(7.15, 9.62), (7.15, 10.3)], lw=1.4, color=ORANGE)
    valve(ax, 7.15, 10.42, size=0.13, vertical=True)
    txt(ax, 7.62, 10.42, "PSV-101", size=5.8, weight="bold", ha="left")
    txt(ax, 7.62, 10.26, "set 66 barg", size=5.2, ha="left", color=MUTED)
    line(ax, [(7.15, 10.55), (7.15, 10.85)], lw=1.4, color=ORANGE)
    line(ax, [(7.15, 10.85), (14.35, 10.85)], lw=1.6, color=ORANGE)
    txt(ax, 10.6, 10.98, "FLARE HEADER — existing, 91% of mass capacity", size=5.6, color=ORANGE)
    line(ax, [(14.35, 10.85), (14.35, 11.05)], lw=1.8, color=ORANGE)
    ax.add_patch(Polygon([(14.35, 11.05), (14.2, 11.35), (14.5, 11.35)], closed=True,
                         facecolor=AMBER, edgecolor=ORANGE, lw=0.8, zorder=4))
    txt(ax, 14.62, 11.2, "flare tip", size=5.2, ha="left", color=MUTED)
    txt(ax, 14.62, 10.85, "+18%: 107% → flare\n+25% upgrade: 86%\nHIPPS −35%: 70%", size=5.0, ha="left", color=MUTED)

    # ---- produced water package
    ax.add_patch(Rectangle((1.05, 5.55), 3.1, 1.75, facecolor="none", edgecolor=GREEN,
                           lw=1.0, linestyle=(0, (3, 2)), zorder=1))
    txt(ax, 1.05, 7.42, "PRODUCED-WATER PACKAGE (NEW)", size=5.8, color=GREEN, ha="left", weight="bold")
    vessel_v(ax, 1.35, 6.0, 0.55, 0.85, color=GREEN, fill="#e8f3ee")
    txt(ax, 1.62, 5.95, "V-104 hydrocyclone", size=4.8, color=MUTED)
    vessel_h(ax, 2.25, 6.15, 1.25, 0.55, color=GREEN, fill="#e8f3ee")
    txt(ax, 3.5, 5.95, "V-105 coalescer", size=4.8, color=MUTED)
    line(ax, [(1.62, 6.85), (1.62, 7.3)], lw=1.2, color=GREEN)
    line(ax, [(2.9, 7.62), (2.9, 7.3)], lw=1.2, color=GREEN)
    line(ax, [(1.9, 6.42), (2.25, 6.42)], lw=1.2, color=GREEN)
    line(ax, [(3.5, 6.7), (3.7, 6.7)], lw=1.2, color=GREEN)
    pump(ax, 3.95, 6.7, r=0.18)
    txt(ax, 3.95, 6.36, "P-102", size=5.0, weight="bold")
    line(ax, [(4.13, 6.7), (4.45, 6.7)], lw=1.2, color=GREEN)
    line(ax, [(4.45, 6.7), (4.45, 5.2)], lw=1.2, color=GREEN)
    line(ax, [(4.45, 5.2), (6.6, 5.2)], lw=1.2, color=GREEN)
    arrowhead(ax, 6.6, 5.2, 1, 0, color=GREEN)
    txt(ax, 6.68, 5.2, "to disposal / reinjection\n2,782 bbl/d (93% of 3,000)", size=5.0, ha="left", color=MUTED)
    txt(ax, 1.15, 5.66, "3,000 → 6,000 bbl/d · coalescer required: interface settling 17 s", size=4.9, color=MUTED, ha="left")

    # ---- notes
    nx, ny, nw, nh = 12.35, 5.5, 3.85, 3.3
    ax.add_patch(Rectangle((nx, ny), nw, nh, facecolor="white", edgecolor=MUTED, lw=0.8, zorder=5))
    txt(ax, nx + 0.12, ny + nh - 0.22, "NOTES", size=6.4, ha="left", weight="bold")
    notes = [
        "1. Simplified P&ID for the FEED case study. It shows",
        "   function, not fabrication: line classes, nozzle",
        "   sizes, materials and support are excluded.",
        "2. Green scope is the recommended package: compressor",
        "   re-rate, HP internals, HIPPS and produced-water",
        "   upgrade with coalescer. Flare/header upgrade is",
        "   the fallback if the SIL-3 case is not approved.",
        "3. The HIPPS is 1oo2, SIL 3, PFD 1e-4, proof test",
        "   12 months. It cuts the blocked-outlet relief case",
        "   by 35% (study assumption), protecting the existing",
        "   66 barg flare system and the 66 barg pipeline.",
        "4. Existing PSV-101 (P orifice, 6.38 in²) passes the",
        "   +18% case at 86.7%; the flare, not the PSV, is the",
        "   relief gate.",
        "5. Values are screening results from azoth 0.1.4",
        "   (partially verified). Not a design deliverable.",
    ]
    for i, n in enumerate(notes):
        txt(ax, nx + 0.12, ny + nh - 0.45 - i * 0.165, n, size=4.9, ha="left", color=INK if i == 0 else MUTED)

    # ---- legend and title block
    legend(ax)
    title_block(ax, "AZ-FEED-08-PID-001", "SIMPLIFIED P&ID — HP SEPARATION & RELIEF", "1 of 1")

    fig.savefig(OUT / "AZ-FEED-08-PID-001.png", dpi=220, facecolor=PAPER)
    fig.savefig(OUT / "AZ-FEED-08-PID-001.svg", facecolor=PAPER)
    fig.savefig(OUT / "AZ-FEED-08-PID-001.pdf", facecolor=PAPER)
    plt.close(fig)


if __name__ == "__main__":
    draw_bfd()
    draw_pid()
    print("drawings written to", OUT)
