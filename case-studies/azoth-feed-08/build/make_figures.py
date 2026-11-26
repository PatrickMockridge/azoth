"""Generate the FEED case study's figures from model/results.json."""

from __future__ import annotations

import json
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import FancyBboxPatch, Rectangle

ROOT = Path(__file__).resolve().parents[1]
RES = json.loads((ROOT / "model" / "results.json").read_text())
OUT = ROOT / "drawings"
OUT.mkdir(exist_ok=True)

PALETTE = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100"]
INK, MUTED, GRID = "#0b0b0b", "#52514e", "#d9d8d4"
plt.rcParams.update(
    {
        "figure.facecolor": "#fcfcfb",
        "axes.facecolor": "#fcfcfb",
        "axes.edgecolor": MUTED,
        "axes.labelcolor": INK,
        "text.color": INK,
        "xtick.color": MUTED,
        "ytick.color": MUTED,
        "axes.grid": True,
        "grid.color": GRID,
        "grid.alpha": 0.6,
        "axes.spines.top": False,
        "axes.spines.right": False,
        "lines.linewidth": 2.0,
        "font.size": 9,
        "figure.figsize": (7.2, 3.8),
    }
)


def save(fig, name):
    for ext in ("png", "svg"):
        fig.savefig(OUT / f"{name}.{ext}", dpi=220, bbox_inches="tight", facecolor=fig.get_facecolor())
    plt.close(fig)


def capacity_chain():
    util = RES["capacity"]["utilisation"]
    items = sorted(util.items(), key=lambda kv: kv[1])
    labels = [k for k, _ in items]
    values = [v * 100.0 for _, v in items]
    colors = [PALETTE[1] if v > 100.0 else (PALETTE[3] if v > 90.0 else PALETTE[0]) for v in values]
    fig, ax = plt.subplots(figsize=(7.4, 4.0))
    bars = ax.barh(labels, values, color=colors, height=0.62)
    ax.axvline(100.0, color=INK, linestyle="--", linewidth=1.0)
    ax.text(100.8, -0.85, "design limit", color=INK, fontsize=7.5, va="bottom")
    for bar, value in zip(bars, values, strict=True):
        ax.text(value + 1.6, bar.get_y() + bar.get_height() / 2, f"{value:.0f}%", va="center", fontsize=8)
    ax.set_xlim(0, max(values) * 1.18)
    ax.set_xlabel("utilisation of the design limit, %")
    ax.set_title("Year-ten capacity chain: the separator and compressor bind first", loc="left", fontsize=10.5)
    ax.grid(axis="y", visible=False)
    save(fig, "fig_capacity_chain")


def capability():
    caps = RES["capacity"]["caps"]
    fixed = RES["capacity"]["fixed_caps"]
    keys = [k for k in caps if k in ("HP separator", "Compressor", "Flare", "Water plant", "Gas line", "PSV")]
    x = np.arange(len(keys))
    width = 0.36
    fig, ax = plt.subplots(figsize=(7.6, 4.1))
    ax.bar(x - width / 2, [caps[k] for k in keys], width, color=PALETTE[0], label="as found")
    ax.bar(x + width / 2, [fixed[k] for k in keys], width, color=PALETTE[2], label="after gas side + water")
    for i, k in enumerate(keys):
        ax.text(i - width / 2, caps[k] + 2, f"{caps[k]:.0f}", ha="center", fontsize=7.4, color=MUTED)
        ax.text(i + width / 2, fixed[k] + 2, f"{fixed[k]:.0f}", ha="center", fontsize=7.4, color=INK)
    ax.axhline(RES["train"]["current"]["gas_mmscfd"], color=MUTED, linestyle=":", linewidth=1.0)
    ax.axhline(RES["capacity"]["package_cap"], color=PALETTE[2], linestyle="-.", linewidth=1.0)
    ax.axhline(RES["train"]["potential"]["gas_mmscfd"], color=INK, linestyle="--", linewidth=1.0)
    ax.text(5.55, RES["capacity"]["package_cap"] + 2, f"package cap {RES['capacity']['package_cap']:.1f}",
            ha="right", fontsize=7.4, color=PALETTE[2])
    ax.text(5.55, RES["train"]["potential"]["gas_mmscfd"] + 2, f"well potential {RES['train']['potential']['gas_mmscfd']:.1f}",
            ha="right", fontsize=7.4, color=INK)
    ax.set_xticks(x, keys)
    ax.set_ylabel("gas rate at the constraint, MMscfd")
    ax.set_ylim(0, max(fixed.values()) * 1.12)
    ax.set_title("The cap each fix buys, and the link nobody priced", loc="left", fontsize=10.5)
    ax.legend(frameon=False, loc="upper left", fontsize=8)
    ax.grid(axis="x", visible=False)
    save(fig, "fig_capability")


def relief():
    rel = RES["relief"]
    cases = ["current", "+18% gas", "+18% flare +25%", "+18% HIPPS"]
    flare_vals = [
        rel["current"]["flare_mass_utilisation"] * 100,
        rel["plus_18"]["flare_mass_utilisation"] * 100,
        rel["plus_18_flare25"]["flare_mass_utilisation"] * 100,
        rel["plus_18_hipps"]["flare_mass_utilisation"] * 100,
    ]
    psv_vals = [
        rel["current"]["psv_utilisation"] * 100,
        rel["plus_18"]["psv_utilisation"] * 100,
        rel["plus_18"]["psv_utilisation"] * 100,
        rel["plus_18_hipps"]["psv_utilisation"] * 100,
    ]
    x = np.arange(len(cases))
    width = 0.36
    fig, ax = plt.subplots(figsize=(7.2, 3.9))
    ax.bar(x - width / 2, flare_vals, width, color=PALETTE[1], label="flare mass capacity")
    ax.bar(x + width / 2, psv_vals, width, color=PALETTE[0], label="PSV orifice")
    ax.axhline(100.0, color=INK, linestyle="--", linewidth=1.0)
    for xi, v in zip(x - width / 2, flare_vals, strict=True):
        ax.text(xi, v + 2, f"{v:.0f}%", ha="center", fontsize=8)
    for xi, v in zip(x + width / 2, psv_vals, strict=True):
        ax.text(xi, v + 2, f"{v:.0f}%", ha="center", fontsize=8, color=MUTED)
    ax.set_xticks(x, cases)
    ax.set_ylabel("utilisation of design capacity, %")
    ax.set_ylim(0, 125)
    ax.set_title("The flare is the relief gate; the PSV is not", loc="left", fontsize=10.5)
    ax.legend(frameon=False, loc="upper left", fontsize=8, ncols=2)
    ax.grid(axis="x", visible=False)
    save(fig, "fig_relief")


def economics():
    res = RES["economics"]["results"]
    order = ["Gas side", "Gas side + water", "HIPPS + water", "+ fourth stage"]
    colors = [PALETTE[0], PALETTE[1], PALETTE[2], PALETTE[3]]
    fig, ax = plt.subplots(figsize=(7.4, 4.0))
    years = np.arange(1, 11)
    for name, color in zip(order, colors, strict=True):
        ax.plot(years, res[name]["cum_disc"], marker="o", markersize=3, color=color, label=f"{name} ({res[name]['npv']:.1f})")
    ax.axhline(0.0, color=INK, linewidth=1.0)
    ax.set_xlabel("year")
    ax.set_ylabel("cumulative discounted cash, MUSD")
    ax.set_title("Ten-year incremental DCF at 10%: the water upgrade is the enabler", loc="left", fontsize=10.5)
    ax.legend(frameon=False, fontsize=8, title="NPV, MUSD")
    ax.set_xticks(years)
    save(fig, "fig_economics")


def tornado():
    rows = sorted(RES["economics"]["tornado"], key=lambda r: abs(r["change"]))
    names = [r["case"] for r in rows]
    values = [r["change"] for r in rows]
    base = RES["economics"]["results"]["Gas side + water"]["npv"]
    fig, ax = plt.subplots(figsize=(7.4, 4.4))
    colors = [PALETTE[2] if v >= 0 else PALETTE[1] for v in values]
    ax.barh(names, values, color=colors, height=0.62)
    ax.axvline(0.0, color=INK, linewidth=1.0)
    for i, v in enumerate(values):
        ax.text(v + (0.6 if v >= 0 else -0.6), i, f"{v:+.1f}", va="center", ha="left" if v >= 0 else "right", fontsize=7.4)
    ax.set_xlabel(f"NPV change from the gas+water case, {base:.1f} MUSD")
    ax.set_title("What moves the answer: gas price, decline and capex dominate", loc="left", fontsize=10.5)
    ax.grid(axis="y", visible=False)
    save(fig, "fig_tornado")


def gantt():
    """Two-panel schedule: project phases and the four-week shutdown window."""
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(11.6, 4.4), gridspec_kw={"width_ratios": [1.35, 1.0]})
    phases = [
        ("FEED close-out and detailed design", 0, 10, PALETTE[0]),
        ("Long-lead procurement (compressor, HIPPS)", 6, 30, PALETTE[1]),
        ("Fabrication and factory acceptance testing", 18, 32, PALETTE[3]),
        ("Offshore installation and shutdown execution", 32, 36, PALETTE[2]),
        ("Commissioning and ramp-up", 36, 40, PALETTE[0]),
        ("Close-out and benefits realisation", 40, 42, PALETTE[1]),
    ]
    for i, (name, start, end, color) in enumerate(phases):
        ax1.barh(i, end - start, left=start, color=color, height=0.5)
        ax1.text(end + 0.5, i, f"{start}-{end} wk", va="center", fontsize=7.4)
    ax1.set_yticks(range(len(phases)), [p[0] for p in phases])
    ax1.invert_yaxis()
    ax1.set_xlim(0, 46)
    ax1.axvspan(32, 36, color=PALETTE[2], alpha=0.12)
    ax1.set_xlabel("project week")
    ax1.set_title("(a) Project phases — 42-week plan", loc="left", fontsize=9.5)
    ax1.grid(axis="y", visible=False)

    tasks = [
        ("Produced-water package (4 wk)", 0, 4, PALETTE[1]),
        ("Compressor re-rate (3 wk)", 0, 3, PALETTE[0]),
        ("Separator internals (3 wk)", 0.3, 3.3, PALETTE[3]),
        ("HIPPS installation (2 wk)", 1.0, 3.0, PALETTE[2]),
        ("Flare/header fallback (4 wk)", 0, 4, PALETTE[1]),
        ("Commissioning and handover", 3.2, 4.0, PALETTE[0]),
    ]
    for i, (name, start, end, color) in enumerate(tasks):
        ax2.barh(i, end - start, left=start, color=color, height=0.5)
        ax2.text(end + 0.06, i, f"{end - start:.1f} wk", va="center", fontsize=7.2)
    ax2.set_yticks(range(len(tasks)), [t[0] for t in tasks])
    ax2.invert_yaxis()
    ax2.set_xlim(0, 4.9)
    ax2.set_xticks([0, 1, 2, 3, 4])
    ax2.set_xlabel("shutdown week")
    ax2.set_title("(b) Four-week shutdown window — parallel work fronts", loc="left", fontsize=9.5)
    ax2.grid(axis="y", visible=False)
    fig.tight_layout()
    save(fig, "fig_gantt")


if __name__ == "__main__":
    capacity_chain()
    capability()
    relief()
    economics()
    tornado()
    gantt()
    print("figures written to", OUT)
