"""Figures for the HAZOP close-out report."""

from __future__ import annotations

import json
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
C = json.loads((ROOT / "model" / "closeout_results.json").read_text())
RES = json.loads((ROOT / "model" / "results.json").read_text())
OUT = ROOT / "drawings"

INK, MUTED, GRID = "#0b0b0b", "#52514e", "#d9d8d4"
BLUE, ORANGE, GREEN, AMBER, RED = "#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#b3261e"
plt.rcParams.update({
    "figure.facecolor": "#fcfcfb", "axes.facecolor": "#fcfcfb", "axes.edgecolor": MUTED,
    "axes.labelcolor": INK, "text.color": INK, "xtick.color": MUTED, "ytick.color": MUTED,
    "axes.grid": True, "grid.color": GRID, "grid.alpha": 0.6,
    "axes.spines.top": False, "axes.spines.right": False, "lines.linewidth": 2.0,
    "font.size": 9, "figure.figsize": (7.4, 3.6),
})


def save(fig, name):
    for ext in ("png", "svg"):
        fig.savefig(OUT / f"{name}.{ext}", dpi=220, bbox_inches="tight", facecolor=fig.get_facecolor())
    plt.close(fig)


def relief():
    labels = ["Current\n137.1 MMscfd", "+18 %\n50.31 kg/s", "+25 % flare\n50.31 kg/s"]
    vals = [91.10, 107.50, 86.00]
    colors = [BLUE, RED, GREEN]
    fig, ax = plt.subplots()
    bars = ax.bar(labels, vals, color=colors, width=0.55)
    ax.axhline(100.0, color=INK, linestyle="--", linewidth=1.0)
    ax.text(2.35, 101.5, "capacity", ha="right", fontsize=8)
    for b, v in zip(bars, vals):
        ax.text(b.get_x() + b.get_width() / 2, v + 2.0, f"{v:.0f}%", ha="center", fontsize=9, weight="bold")
    ax.set_ylim(0, 125)
    ax.set_ylabel("flare mass utilisation, %")
    ax.set_title("Close-out relief basis: the credit is withdrawn, the flare is upgraded", loc="left", fontsize=10.5)
    ax.grid(axis="x", visible=False)
    save(fig, "fig_closeout_relief")


def sil():
    names = ["As designed\n1oo2 PT / 1oo1 SDV", "Closed out\n1oo2 all, 6-month test", "SIL 3 band"]
    vals = [C["sil"]["as_designed"]["pfd_total"], C["sil"]["closed_out"]["pfd_total"], 1.0]
    fig, ax = plt.subplots()
    ax.axhspan(1e-4, 1e-3, color=GREEN, alpha=0.12)
    ax.text(2.28, 2.2e-4, "SIL 3", color=GREEN, fontsize=9, ha="right")
    bars = ax.bar([0, 1], [vals[0], vals[1]], color=[RED, GREEN], width=0.5)
    ax.set_yscale("log")
    ax.set_ylim(1e-6, 1e-1)
    ax.set_xticks([0, 1], names[:2])
    for b, v in zip(bars, vals[:2]):
        ax.text(b.get_x() + b.get_width() / 2, v * 1.6, f"{v:.1e}", ha="center", fontsize=9, weight="bold")
    ax.set_ylabel("PFD average (log scale)")
    ax.set_title("SIL verification: the Rev B loop is SIL 2, not SIL 3", loc="left", fontsize=10.5)
    ax.grid(axis="x", visible=False)
    save(fig, "fig_closeout_sil")


def psv():
    labels = ["4 in inlet", "6 in inlet"]
    vals = [C["psv_hydraulics"]["4 in"]["dP_pct_set"], C["psv_hydraulics"]["6 in"]["dP_pct_set"]]
    fig, ax = plt.subplots(figsize=(6.6, 3.4))
    bars = ax.bar(labels, vals, color=[RED, GREEN], width=0.45)
    ax.axhline(3.0, color=INK, linestyle="--", linewidth=1.0)
    ax.text(1.4, 3.4, "API 520 3 % limit", ha="right", fontsize=8)
    for b, v in zip(bars, vals):
        ax.text(b.get_x() + b.get_width() / 2, v + 0.4, f"{v:.1f}%", ha="center", fontsize=9, weight="bold")
    ax.set_ylabel("inlet pressure drop, % of set")
    ax.set_ylim(0, 19)
    ax.set_title("PSV-101 inlet: 4 in fails, 6 in passes", loc="left", fontsize=10.5)
    ax.grid(axis="x", visible=False)
    save(fig, "fig_closeout_psv")


def water():
    vals = [C["water"]["water_kg_per_mmscf"], C["water"]["spec_kg_per_mmscf"]]
    fig, ax = plt.subplots(figsize=(6.6, 3.4))
    bars = ax.bar(["Water-saturated\nat 313 K / 90 bar", "Pipeline spec\n3 lb/MMscf"], vals, color=[ORANGE, BLUE], width=0.45)
    ax.set_yscale("log")
    for b, v in zip(bars, vals):
        ax.text(b.get_x() + b.get_width() / 2, v * 1.5, f"{v:.1f} kg/MMscf", ha="center", fontsize=9, weight="bold")
    ax.set_ylabel("water content, kg/MMscf (log)")
    ax.set_title("Dehydration gap: 15x the pipeline specification", loc="left", fontsize=10.5)
    ax.grid(axis="x", visible=False)
    save(fig, "fig_closeout_water")


def economics():
    cases = ["Rev B\n(credit withdrawn)", "Flare only\n48.0 MUSD", "Rev C\n54.1 MUSD"]
    vals = [C["economics"]["Rev B — HIPPS credit (withdrawn)"]["npv_musd"],
            C["economics"]["Flare upgrade only"]["npv_musd"],
            C["economics"]["Rev C — flare + close-out scope"]["npv_musd"]]
    fig, ax = plt.subplots(figsize=(6.8, 3.4))
    bars = ax.bar(cases, vals, color=[MUTED, AMBER, BLUE], width=0.5)
    for b, v in zip(bars, vals):
        ax.text(b.get_x() + b.get_width() / 2, v + 0.6, f"{v:.1f}", ha="center", fontsize=9, weight="bold")
    ax.set_ylabel("NPV at 10 %, MUSD")
    ax.set_ylim(0, 28)
    ax.set_title("The safety close-out gives back value, not the project", loc="left", fontsize=10.5)
    ax.grid(axis="x", visible=False)
    save(fig, "fig_closeout_econ")


if __name__ == "__main__":
    relief(); sil(); psv(); water(); economics()
    print("close-out figures written to", OUT)
