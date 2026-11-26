"""Figures for the FEED additions: resources, cost and the 42-week schedule."""

from __future__ import annotations

import json
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Patch

ROOT = Path(__file__).resolve().parents[1]
F = json.loads((ROOT / "model" / "feed_results.json").read_text())
OUT = ROOT / "drawings"

INK, MUTED, GRID = "#0b0b0b", "#52514e", "#d9d8d4"
BLUE, ORANGE, GREEN, AMBER, PURPLE, GREY = "#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#7d5ba6", "#5a5751"
plt.rcParams.update({
    "figure.facecolor": "#fcfcfb", "axes.facecolor": "#fcfcfb", "axes.edgecolor": MUTED,
    "axes.labelcolor": INK, "text.color": INK, "xtick.color": MUTED, "ytick.color": MUTED,
    "axes.grid": True, "grid.color": GRID, "grid.alpha": 0.6,
    "axes.spines.top": False, "axes.spines.right": False, "lines.linewidth": 2.0,
    "font.size": 9, "figure.figsize": (7.2, 3.8),
})


def save(fig, name):
    for ext in ("png", "svg"):
        fig.savefig(OUT / f"{name}.{ext}", dpi=220, bbox_inches="tight", facecolor=fig.get_facecolor())
    plt.close(fig)


def resources():
    s = F["schedule"]
    weeks = np.arange(1, s["weeks"] + 1)
    h = np.array(s["histogram_hours"], dtype=float)
    cum = np.array(s["cumulative_hours"], dtype=float)
    fig, ax = plt.subplots(figsize=(7.8, 4.0))
    ax.bar(weeks, h, color=BLUE, width=0.78, label="planned hours per week")
    ax.axvspan(32, 36, color=ORANGE, alpha=0.10)
    ax.text(34, max(h) * 0.92, "shutdown", ha="center", fontsize=8, color=ORANGE)
    ax.set_xlabel("project week")
    ax.set_ylabel("manhours per week")
    ax.set_xlim(0.2, s["weeks"] + 0.8)
    ax2 = ax.twinx()
    ax2.plot(weeks, cum, color=INK, linewidth=1.8, label="cumulative hours")
    ax2.set_ylabel("cumulative manhours")
    ax2.grid(False)
    ax2.spines["top"].set_visible(False)
    ax2.spines["right"].set_visible(False)
    ax.set_title("Resource-loaded plan: 32,600 manhours, peak at the shutdown", loc="left", fontsize=10.5)
    lines = [Patch(color=BLUE, label="weekly hours"), plt.Line2D([], [], color=INK, label="cumulative")]
    ax.legend(handles=lines, frameon=False, loc="upper left", fontsize=8)
    save(fig, "fig_resources")


def cost():
    c = F["cost"]
    groups = c["groups"]
    labels = [g["group"].replace("precommissioning", "pre-comm").title() for g in groups]
    values = [g["subtotal_musd"] for g in groups]
    colors = [BLUE, GREEN, AMBER, ORANGE, PURPLE, GREY, "#8c6d46"]
    fig, ax = plt.subplots(figsize=(7.8, 4.0))
    left = 0.0
    for lab, val, col in zip(labels, values, colors, strict=True):
        ax.barh(0, val, left=left, color=col, height=0.45)
        if val >= 2.0:
            ax.text(left + val / 2, 0, f"{val:.1f}", ha="center", va="center", color="white", fontsize=8, weight="bold")
        left += val
    ax.set_xlim(0, 37.5)
    ax.set_yticks([])
    ax.set_xlabel("MUSD (AACE Class 4/5 study allowance)")
    ax.set_title(f"Recommended package cost build-up — {c['total_musd']:.1f} MUSD", loc="left", fontsize=10.5)
    handles = [Patch(color=col, label=f"{lab} {val:.1f}") for lab, val, col in zip(labels, values, colors, strict=True)]
    ax.legend(handles=handles, frameon=False, ncol=2, fontsize=8, loc="lower center", bbox_to_anchor=(0.5, -0.42))
    ax.grid(axis="y", visible=False)
    save(fig, "fig_cost")


def schedule():
    s = F["schedule"]
    acts = s["activities"]
    phases = s["phases"]
    colours = s["phase_colours"]
    fig, ax = plt.subplots(figsize=(8.4, 6.6))
    for i, a in enumerate(acts):
        ax.barh(i, a["duration"], left=a["start"], color=colours[a["phase"]], height=0.62)
    ax.set_yticks(range(len(acts)), [f'{a["id"]}  {a["name"][:44]}' for a in acts], fontsize=6.8)
    ax.invert_yaxis()
    ax.axvspan(32, 36, color=ORANGE, alpha=0.10)
    ax.axvline(32, color=ORANGE, linestyle="--", linewidth=1.0)
    ax.axvline(36, color=ORANGE, linestyle="--", linewidth=1.0)
    ax.set_xlim(0, 44)
    ax.set_xlabel("project week")
    ax.set_title("42-week implementation plan — four-week shutdown in weeks 32-36", loc="left", fontsize=10.5)
    handles = [Patch(color=colours[p], label=p) for p in phases]
    ax.legend(handles=handles, frameon=False, fontsize=7.4, loc="lower right")
    ax.grid(axis="y", visible=False)
    fig.tight_layout()
    save(fig, "fig_schedule")


def manhours():
    mh = F["manhours"]
    rows = mh["rows"]
    labels = [r["discipline"] for r in rows]
    e = [r["engineering"] for r in rows]
    f = [r["fabrication"] for r in rows]
    o = [r["offshore"] for r in rows]
    fig, ax = plt.subplots(figsize=(7.6, 4.2))
    y = np.arange(len(labels))
    ax.barh(y, e, color=BLUE, label="engineering")
    ax.barh(y, f, left=e, color=GREEN, label="fabrication")
    ax.barh(y, o, left=np.array(e) + np.array(f), color=ORANGE, label="offshore")
    ax.set_yticks(y, labels, fontsize=8)
    ax.invert_yaxis()
    ax.set_xlabel("manhours")
    ax.set_title("Discipline manhours — 32,600 h total", loc="left", fontsize=10.5)
    ax.legend(frameon=False, fontsize=8, loc="lower right")
    ax.grid(axis="y", visible=False)
    fig.tight_layout()
    save(fig, "fig_manhours")


if __name__ == "__main__":
    resources(); cost(); schedule(); manhours()
    print("feed figures written to", OUT)
