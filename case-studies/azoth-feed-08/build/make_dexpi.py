"""Build the DEXPI 2.0 process model and PFD for the debottlenecked train.

The flowsheet is the recommended-scope steady state at the year-ten design case:
three-stage separation, staged compression to 90 bara with intercooling, a stock
tank, and the produced-water draw. It is exported as a DEXPI 2.0 Process exchange
with its drawing, validated against the vendored schema, and rendered to SVG/PNG/PDF.

Run from the package root:  python build/make_dexpi.py
"""

from __future__ import annotations

import json
import math
from pathlib import Path

import azoth.dexpi as dexpi

ROOT = Path(__file__).resolve().parents[1]
RES = json.loads((ROOT / "model" / "results.json").read_text())
OUT = ROOT / "drawings"
OUT.mkdir(exist_ok=True)

Z = RES["split"]["z_current"]
COMP = RES["meta"]["components"]
PALETTE = ROOT.parent.parent / "specs" / "unit_ops"


def zvec(z):
    return "[" + ", ".join(repr(float(v)) for v in z) + "]"


class Flowsheet:
    def __init__(self):
        self.instances = []
        self.connections = []

    def add(self, tag, unit, params):
        self.instances.append((tag, unit, params))
        return tag

    def connect(self, src, dst):
        self.connections.append((src, dst))

    def toml(self, layout):
        lines = [
            'id = "flowsheets.feed08_debottleneck"',
            'name = "AZ-FEED-08 modified separation and compression train"',
            'products = ["Sales gas", "Condensate", "Produced water", "Fuel gas"]',
            "",
            "[[inputs]]",
            'name = "Wellstream"',
            f"components = {json.dumps(COMP)}",
            "n = 2230.0",
            f"z = {zvec(Z)}",
            "P = 60.0e5",
            "T = 354.15",
            "",
        ]
        for tag, unit, params in self.instances:
            lines += [f'[[instances]]\nid = "{tag}"\nunit = "unit_ops.{unit}"']
            if params:
                lines.append("[instances.parameters]")
                for key, value in params.items():
                    lines.append(f"{key} = {value}")
            lines.append("")
        for src, dst in self.connections:
            lines += ["[[connections]]", f'from = "{src}"', f'to = "{dst}"', ""]
        lines.append("[layout.inputs]")
        lines.append(f'Wellstream = [{layout["Wellstream"][0]}, {layout["Wellstream"][1]}]')
        lines.append("")
        lines.append("[layout.instances]")
        for tag, (x, y) in layout["instances"].items():
            lines.append(f'{tag} = [{x}, {y}]')
        lines.append("")
        lines.append("[layout.products]")
        for name, (x, y) in layout["products"].items():
            lines.append(f'"{name}" = [{x}, {y}]')
        return "\n".join(lines) + "\n"


def staged(fs, prefix, gas_in, p0, p_out, stages, x0, y, xstep=210):
    """A compressor/cooler ladder; returns the final outlet and last x."""
    prev = gas_in
    x = x0
    for i in range(stages):
        ratio = (p_out / p0) ** (1.0 / stages)
        p = p0 * ratio ** (i + 1)
        k = f"{prefix}{chr(65 + i)}"
        fs.add(k, "compressor", {"outlet_pressure": f"{p:.6f}e5", "isentropic_efficiency": 0.78})
        fs.connect(prev, f"{k}.inlet")
        prev = f"{k}.outlet"
        x += xstep
        e = f"E-{prefix.split('-')[1]}{chr(65 + i)}"
        fs.add(e, "cooler", {"outlet_temperature": 313.15, "pressure_drop": 0.0})
        fs.connect(prev, f"{e}.inlet")
        prev = f"{e}.outlet"
        x += xstep
    return prev, x


def build():
    fs = Flowsheet()
    fs.add("V-101", "three_phase_separator", {
        "pressure_drop": 0.0, "heat_input": 0.0,
        "gas_in_aqueous": 0.0, "gas_in_oil": 0.0, "oil_in_aqueous": 0.0,
        "oil_in_gas": 0.0, "aqueous_in_gas": 0.0, "aqueous_in_oil": 0.0,
    })
    fs.connect("Wellstream", "V-101.feed")
    # HP gas: one stage 60 -> 90 bara
    fs.add("K-101", "compressor", {"outlet_pressure": "90.0e5", "isentropic_efficiency": 0.78})
    fs.add("E-101", "cooler", {"outlet_temperature": 313.15, "pressure_drop": 0.0})
    fs.connect("V-101.vapour", "K-101.inlet")
    fs.connect("K-101.outlet", "E-101.inlet")
    # MP oil let-down and 3-stage compression 10 -> 90
    fs.add("VLV-102", "throttling_valve", {"outlet_pressure": "10.0e5"})
    fs.add("V-102", "separator", {"pressure_drop": 0.0, "gas_in_liquid": 0.0})
    fs.connect("V-101.light_liquid", "VLV-102.inlet")
    fs.connect("VLV-102.outlet", "V-102.feed")
    mp, _ = staged(fs, "K-102", "V-102.vapour", 10.0, 90.0, 3, 700, 520)
    # LP liquid let-down and 5-stage compression 2 -> 90
    fs.add("VLV-103", "throttling_valve", {"outlet_pressure": "2.0e5"})
    fs.add("V-103", "separator", {"pressure_drop": 0.0, "gas_in_liquid": 0.0})
    fs.connect("V-102.liquid", "VLV-103.inlet")
    fs.connect("VLV-103.outlet", "V-103.feed")
    lp, _ = staged(fs, "K-103", "V-103.vapour", 2.0, 90.0, 5, 700, 300)
    # stock tank: cool to 15 C and flash at atmospheric
    fs.add("E-104", "cooler", {"outlet_temperature": 288.15, "pressure_drop": 0.0})
    fs.add("V-104", "separator", {"pressure_drop": 98700.0, "gas_in_liquid": 0.0})
    fs.connect("V-103.liquid", "E-104.inlet")
    fs.connect("E-104.outlet", "V-104.feed")
    fs.add("M-101", "mixer", {})
    fs.connect("E-101.outlet", "M-101.feed[0]")
    fs.connect(mp, "M-101.feed[1]")
    fs.connect(lp, "M-101.feed[2]")
    fs.connect("V-104.vapour", "Fuel gas")
    fs.connect("M-101.product", "Sales gas")
    fs.connect("V-104.liquid", "Condensate")
    fs.connect("V-101.heavy_liquid", "Produced water")

    inst = {
        "V-101": (40, 120), "K-101": (260, 120), "E-101": (470, 120),
        "VLV-102": (40, 320), "V-102": (250, 320),
        "VLV-103": (40, 540), "V-103": (250, 540),
    }
    x = 470
    for i in range(3):
        inst[f"K-102{chr(65 + i)}"] = (x, 320); x += 210
        inst[f"E-102{chr(65 + i)}"] = (x, 320); x += 210
    x = 470
    for i in range(5):
        inst[f"K-103{chr(65 + i)}"] = (x, 540); x += 210
        inst[f"E-103{chr(65 + i)}"] = (x, 540); x += 210
    inst["E-104"] = (x + 40, 540)
    inst["V-104"] = (x + 250, 540)
    inst["M-101"] = (x + 480, 120)
    layout = {
        "Wellstream": (0, 120),
        "instances": inst,
        "products": {
            "Sales gas": (x + 700, 120),
            "Condensate": (x + 700, 640),
            "Fuel gas": (x + 480, 640),
            "Produced water": (40, 20),
        },
    }
    return fs, layout


def main():
    fs, layout = build()
    toml = fs.toml(layout)
    (ROOT / "model" / "feed08_pfd.toml").write_text(toml)
    xml, report = dexpi.export_process(
        toml, str(PALETTE), drawing=True,
        allow_lossy_mappings=True, with_azoth_extensions=True,
    )
    xml_path = OUT / "AZ-FEED-08-DEXPI-PFD-001.xml"
    xml_path.write_text(xml)
    validation = dexpi.validate(xml)
    view = dexpi.view(xml, str(PALETTE))
    (OUT / "AZ-FEED-08-DEXPI-PFD-001.report.json").write_text(
        json.dumps({"export": report, "validation": validation,
                    "bounds": view["drawing"]["bounds"],
                    "group_count": len(view["drawing"]["groups"])}, indent=2)
    )
    print(json.dumps({"export": report, "validation": validation,
                      "bounds": view["drawing"]["bounds"]}, indent=2))


if __name__ == "__main__":
    main()
