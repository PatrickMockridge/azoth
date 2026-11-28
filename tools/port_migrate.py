#!/usr/bin/env python3
"""Scaffold `source.neqsim` and `[provenance]` for specs that lack them.

This is the migration tool the port law needs once, not a generator: it reads the spec's
own `source`/`references`, finds the NeqSim class the text already names, and writes the
structured block. It **never invents a class or a member**: a spec whose text names no
class in the index is reported and left alone, and `tools/check_port.py` then refuses it.

    python tools/port_migrate.py            # report what is missing
    python tools/port_migrate.py --write    # insert the blocks
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
import check_port  # noqa: E402

#: The ids whose text names a publication but whose class the audit established.
KNOWN = {
    "eos.pt_flash": ("TPflash", "TPflash.run"),
    "eos.bubble_pressure": ("BubblePointPressureFlash", "BubblePointPressureFlash"),
    "eos.bubble_temperature": ("BubblePointTemperatureFlash", "BubblePointTemperatureFlash"),
    "eos.dew_pressure": ("DewPointPressureFlash", "DewPointPressureFlash"),
    "eos.dew_temperature": ("DewPointTemperatureFlash", "DewPointTemperatureFlash"),
    "eos.pt_phase_envelope": ("PTPhaseEnvelopeMichelsen", "PTPhaseEnvelopeMichelsen"),
    "eos.pure_saturation": ("CubicPureSaturation", "CubicPureSaturation"),
    "eos.molar_enthalpy_entropy": ("PhasePrEos", "PhasePrEos"),
    "eos.wilke_viscosity": ("Phase", "Phase.calcViscosity"),
    "eos.nrtl_activity_coefficients": ("PhaseGENRTL", "PhaseGENRTL"),
    "eos.pu_flash": ("ThermodynamicOperations", "ThermodynamicOperations.PUflash"),
    "eos.th_flash": ("ThermodynamicOperations", "ThermodynamicOperations.THflash"),
    "eos.ts_flash": ("ThermodynamicOperations", "ThermodynamicOperations.TSflash"),
    "eos.tu_flash": ("ThermodynamicOperations", "ThermodynamicOperations.TUflash"),
    "eos.tv_flash": ("ThermodynamicOperations", "ThermodynamicOperations.TVflash"),
    "eos.chung_conductivity": ("ChungConductivityMethod", "ChungConductivityMethod"),
    "eos.chung_viscosity": ("ChungViscosityMethod", "ChungViscosityMethod"),
    "eos.costald_molar_volume": ("Costald", "Costald"),
    "eos.hayduk_minhas_diffusivity": ("HaydukMinhasDiffusivity", "HaydukMinhasDiffusivity"),
    "eos.rk_departure": ("PhaseRK", "PhaseRK"),
    "eos.siddiqi_lucas_diffusivity": ("SiddiqiLucasMethod", "SiddiqiLucasMethod"),
    "eos.srk_departure": ("PhaseSrkEos", "PhaseSrkEos"),
    "eos.srk_kappa": ("PhaseSrkEos", "PhaseSrkEos"),
    "eos.srk_z_factor": ("PhaseSrkEos", "PhaseSrkEos"),
    "eos.vdw1f_mix_binary": ("EosMixingRuleHandler", "EosMixingRuleHandler"),
    "eos.wilke_chang_diffusivity": ("WilkeChangDiffusivity", "WilkeChangDiffusivity"),
    "hydraulics.choked_flow_area": ("Orifice", "Orifice"),
    "hydraulics.control_valve_cv": ("ControlValveSizing", "ControlValveSizing"),
    "hydraulics.friction_factor_colebrook": (
        "FrictionFactorCalculator",
        "FrictionFactorCalculator",
    ),
    "hydraulics.friction_factor_haaland": (
        "FrictionFactorCalculator",
        "FrictionFactorCalculator",
    ),
    "hydraulics.friction_factor_swamee_jain": (
        "FrictionFactorCalculator",
        "FrictionFactorCalculator",
    ),
    "hydraulics.orifice_flow": ("Orifice", "Orifice"),
    "thermal.conduction_plane_wall": ("CompositeWallConduction", "CompositeWallConduction"),
}

CLASS_TOKEN = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\b")
MEMBER_TOKEN = re.compile(r"`([A-Za-z_][A-Za-z0-9_.]*)`")
WITH_JAVA = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.java\b")

#: The ids whose capture or probe is at the f0c7436 pin rather than the databank pin.
PIN_PREFERENCE = {"eos.umr_pru_flash": "f0c7436"}

#: Short backticked tokens that are units or symbols, not methods.
NON_MEMBERS = {
    "m",
    "kg",
    "K",
    "Pa",
    "J",
    "mol",
    "s",
    "bar",
    "bara",
    "cP",
    "mPa",
    "kW",
    "MW",
    "W",
    "N",
    "T",
    "P",
    "V",
    "U",
    "H",
    "S",
    "R",
    "x",
    "y",
    "z",
    "cp",
    "cv",
    "rho",
}


def ordered_pins(pins: dict[str, set[str]], text: str, calc_id: str) -> list[tuple[str, set[str]]]:
    """The full-commit pins, the one the text or the capture names first."""
    full = sorted({commit: classes for commit, classes in pins.items() if len(commit) != 7}.items())
    preferred = PIN_PREFERENCE.get(calc_id)
    if preferred is not None:
        full.sort(key=lambda pin: not pin[0].startswith(preferred))
    else:
        full.sort(key=lambda pin: pin[0][:7] not in text)
    return full


def source_text(spec: dict[str, Any]) -> str:
    source = spec.get("source") or {}
    return "\n".join(
        [
            str(source.get("standard") or ""),
            str(source.get("edition") or ""),
            *[str(value) for value in spec.get("references") or []],
            *[str(value) for value in spec.get("assumptions") or []],
        ]
    )


def primary(spec: dict[str, Any], pins: dict[str, set[str]]) -> tuple[str, str, str] | None:
    """`(class, member, commit)` read from the spec's own text, or a known gap."""
    calc_id = str(spec.get("id", ""))
    known = KNOWN.get(calc_id)
    joined = source_text(spec)
    ordered = ordered_pins(pins, joined, calc_id)

    java = [match.group(1) for match in WITH_JAVA.finditer(joined)]
    for commit, classes in ordered:
        for candidate in java:
            if candidate in classes:
                return candidate, member_for(spec, candidate), commit

    if known is not None:
        for commit, classes in ordered:
            if known[0] in classes:
                return known[0], known[1], commit

    tokens = [match.group(1) for match in CLASS_TOKEN.finditer(joined)]
    for commit, classes in ordered:
        for candidate in tokens:
            if candidate in classes:
                return candidate, member_for(spec, candidate), commit
    return None


def member_for(spec: dict[str, Any], class_name: str) -> str:
    """A method the spec's edition names, or the class itself."""
    edition = str((spec.get("source") or {}).get("edition") or "")
    for token in MEMBER_TOKEN.findall(edition):
        if "/" in token or token.startswith(".") or token in NON_MEMBERS:
            continue
        if "." in token:
            owner, _, name = token.partition(".")
            if owner == class_name and name:
                return token
            continue
        if token[0].islower() and len(token) >= 4:
            return f"{class_name}.{token}"
    return class_name


def source_label(class_name: str, member: str) -> str:
    """How a value's source reads."""
    if member == class_name:
        return f"NeqSim {class_name}"
    if member.startswith(f"{class_name}."):
        return f"NeqSim {member}"
    return f"NeqSim {class_name}.{member}"


def insert_source(text: str, class_name: str, member: str, commit: str) -> str:
    lines = text.splitlines()
    if "[source]" not in lines:
        return text
    start = lines.index("[source]")
    triple = False
    end = len(lines)
    for index in range(start + 1, len(lines)):
        line = lines[index]
        triple ^= line.count('"""') % 2 == 1
        if not triple and line.startswith("["):
            end = index
            break
    block = [
        "",
        "[source.neqsim]",
        f'class = "{class_name}"',
        f'members = ["{member}"]',
        f'commit = "{commit}"',
        "",
    ]
    return "\n".join(lines[:end] + block + lines[end:]) + ("\n" if text.endswith("\n") else "")


def insert_provenance(text: str, spec: dict[str, Any], class_name: str, member: str) -> str:
    source = source_label(class_name, member)
    lines = [
        "[provenance.inputs]",
        'default = "caller"',
        "",
        "[provenance.outputs]",
        f'default = "{source}"',
        "",
    ]
    algorithm = spec.get("algorithm") or spec.get("solver") or {}
    for leaf in sorted(check_port.leaves(algorithm)):
        key = f'"{leaf}"' if "." in leaf else leaf
        lines += [f"[provenance.algorithm.{key}]", f'source = "{source}"', ""]
    block = "\n".join(lines).rstrip() + "\n"
    marker = "[implementations]"
    if marker in text.splitlines():
        lines_all = text.splitlines()
        at = lines_all.index(marker)
        return "\n".join(lines_all[:at] + block.splitlines() + [""] + lines_all[at:]) + "\n"
    return text + ("\n" if not text.endswith("\n") else "") + block


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--only", type=Path, help="one spec file, for a trial run")
    args = parser.parse_args()
    pins = check_port.pins()
    missing: list[str] = []
    written = 0
    for path, spec in check_port.specs():
        if args.only is not None and path.resolve() != args.only.resolve():
            continue
        if isinstance((spec.get("source") or {}).get("neqsim"), dict):
            continue
        found = primary(spec, pins)
        if found is None:
            missing.append(str(path.relative_to(ROOT)))
            continue
        class_name, member, commit = found
        if args.write:
            text = path.read_text(encoding="utf-8")
            text = insert_source(text, class_name, member, commit)
            text = insert_provenance(text, spec, class_name, member)
            path.write_text(text, encoding="utf-8")
            written += 1
    for path in missing:
        print(f"  no class named: {path}", file=sys.stderr)
    print(f"port_migrate: {written} written, {len(missing)} without a class")
    return 1 if missing else 0


if __name__ == "__main__":
    raise SystemExit(main())
