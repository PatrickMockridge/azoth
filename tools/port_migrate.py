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
#: A member is a string, or a list when the port reads more than one method.
KNOWN = {
    "eos.pt_flash": ("TPflash", "TPflash.run"),
    "eos.pr_departure": ("PhasePrEos", "PhasePrEos"),
    "eos.antoine_vapor_pressure": ("Component", "Component.getAntoineVaporPressure"),
    "eos.ideal_gas_cp": ("Component", "Component.getCp0"),
    "eos.heat_of_vaporization": ("Component", "Component.getPureComponentHeatOfVaporization"),
    "eos.liquid_heat_capacity": ("Component", "Component.getPureComponentCpLiquid"),
    "eos.liquid_viscosity_pure": (
        "Viscosity",
        ["calcPureComponentViscosity", "getViscosityPressureCorrection"],
    ),
    "eos.pr_kappa": ("AttractiveTermPr", ["AttractiveTermPr.init", "AttractiveTermPr.getm"]),
    "eos.pr_z_factor": ("Phase", "Phase.getZ"),
    "eos.pr_molar_volume": ("Phase", "Phase.getMolarVolume"),
    "eos.pr_mass_density": ("Phase", "Phase.getDensity"),
    "eos.mason_saxena_conductivity": (
        "ChungConductivityMethod",
        "ChungConductivityMethod.calcConductivity",
    ),
    "eos.tv_fraction_flash": ("TVfractionFlash", "TVfractionFlash.run"),
    "eos.ph_flash": ("PHflash", "PHflash.run"),
    "eos.ps_flash": ("PSFlash", "PSFlash.run"),
    "eos.pv_flash": ("PVflash", "PVflash.run"),
    "eos.pv_reflux_flash": ("PVrefluxflash", "PVrefluxflash"),
    "eos.pvf_flash": ("PVFflash", "PVFflash.computeBeta"),
    "eos.rachford_rice": ("RachfordRice", "RachfordRice.calcBetaNielsen2023"),
    "eos.stability_test": ("Flash", "Flash.stabilityAnalysis"),
    "eos.multiflash_stability_test": ("TPmultiflash", "TPmultiflash.stabilityAnalysis"),
    "eos.vu_flash": ("OptimizedVUflash", "OptimizedVUflash.calcdQdP"),
    "eos.vu_flash_single_comp": ("VUflashSingleComp", "VUflashSingleComp"),
    "eos.wilson_activity_coefficients": (
        "ComponentGEWilson",
        "ComponentGEWilson.getWilsonActivityCoefficient",
    ),
    "eos.pr78_kappa": ("AttractiveTermPr1978", "AttractiveTermPr1978.setm"),
    "eos.pr_alpha_ab": ("AttractiveTermPr", ["AttractiveTermPr.alpha", "AttractiveTermPr.aT"]),
    "eos.rk_alpha_ab": ("AttractiveTermRk", ["AttractiveTermRk.alpha", "AttractiveTermRk.aT"]),
    "eos.srk_alpha_ab": (
        "AttractiveTermSrk",
        ["AttractiveTermSrk.alpha", "AttractiveTermSrk.aT"],
    ),
    "eos.twu_kappa": ("AttractiveTermTwu", "AttractiveTermTwu.init"),
    "eos.pr_peneloux_shift": ("ComponentPRvolcor", "ComponentPRvolcor.getVolumeCorrection"),
    "eos.srk_peneloux_shift": ("ComponentSrkvolcor", "ComponentSrkvolcor.getVolumeCorrection"),
    "eos.rachford_rice_binary": ("RachfordRice", "RachfordRice.calcBeta"),
    "eos.rackett_molar_volume": ("Rackett", "Rackett.calcDensity"),
    "eos.tbp_fraction_properties": ("TBPfractionModel", "TBPfractionModel.PedersenTBPModelSRK"),
    "eos.chapman_enskog_diffusivity": (
        "Diffusivity",
        "Diffusivity.calcBinaryDiffusionCoefficient",
    ),
    "eos.fuller_schettler_giddings_diffusivity": (
        "FullerSchettlerGiddingsDiffusivity",
        "FullerSchettlerGiddingsDiffusivity.calcBinaryDiffusionCoefficient",
    ),
    "eos.iapws_saturation_pressure": ("Iapws_if97", "Iapws_if97.p4_T"),
    "characterization.tbp_density": (
        "TbpClosure",
        "TbpClosure.RIAZI_DAUBERT_1980.calcDensity",
    ),
    "hydraulics.darcy_weisbach": (
        "FrictionFactorCalculator",
        "FrictionFactorCalculator.calcPressureDropPerLength",
    ),
    "hydraulics.reynolds_number": ("AdiabaticPipe", "AdiabaticPipe.getReynoldsNumber"),
    "process.expander": ("Expander", "Expander.run"),
    "eos.tyn_calus_diffusivity": (
        "TynCalusDiffusivity",
        "TynCalusDiffusivity.calcBinaryDiffusionCoefficient",
    ),
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
    "eos.srk_kappa": ("AttractiveTermSrk", ["AttractiveTermSrk.init", "AttractiveTermSrk.getm"]),
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
    "hydraulics.pump_power": ("Pump", "Pump.run"),
    "thermal.conduction_plane_wall": ("CompositeWallConduction", "CompositeWallConduction"),
}

CLASS_TOKEN = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\b")
MEMBER_TOKEN = re.compile(r"`([A-Za-z_][A-Za-z0-9_.]*)`")
WITH_JAVA = re.compile(r"\b([A-Z][A-Za-z0-9_]*)\.java\b")

#: Words that are NeqSim class names and also ordinary prose. A spec that names one as its
#: source has not named a class; it has used a capitalised word.
GENERIC = {
    "Standard",
    "Fluid",
    "Component",
    "Constant",
    "Units",
    "Viscosity",
    "Model",
    "System",
    "Phase",
    "Stream",
    "Data",
    "Equation",
    "Thermodynamic",
    "NeqSim",
}

#: Composite ports: the primary class and every class the spec reads.
MULTI = {
    "eos.pt_phase_envelope": (
        "PTPhaseEnvelopeMichelsen",
        ["PTPhaseEnvelopeMichelsen", "RobustPhaseEnvelope"],
    ),
}

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


def ordered_pins(pins: dict[str, set[str]]) -> list[tuple[str, set[str]]]:
    """The full-commit pins, the databank pin first.

    `check_manifest` holds a spec's citation to the revision the project vendors, so the
    structured source states the databank pin; a capture at another pin is prose in the
    probe, not the port's source.
    """
    return sorted({commit: classes for commit, classes in pins.items() if len(commit) != 7}.items())


def class_text(spec: dict[str, Any]) -> str:
    """The fields a port's *class* is named in: the source and the references.

    The assumptions and the output descriptions are excluded on purpose: they discuss
    units, papers and ordinary nouns, and a capitalised one of those can be a NeqSim
    class name by accident. The edition is included because it is where a spec names
    the class it ports ("NeqSim master `.../TVfractionFlash`: its residual...").
    """
    source = spec.get("source") or {}
    return "\n".join(
        [
            str(source.get("standard") or ""),
            str(source.get("edition") or ""),
            *[str(value) for value in spec.get("references") or []],
        ]
    )


def member_text(spec: dict[str, Any]) -> str:
    """The fields that name a *member*: the edition and the references."""
    source = spec.get("source") or {}
    return "\n".join(
        [
            str(source.get("edition") or ""),
            *[str(value) for value in spec.get("references") or []],
        ]
    )


def primary(spec: dict[str, Any], pins: dict[str, set[str]]) -> tuple[str, list[str], str] | None:
    """`(class, members, commit)` read from the spec's own text, or a known gap."""
    calc_id = str(spec.get("id", ""))
    ordered = ordered_pins(pins)

    multi = MULTI.get(calc_id)
    if multi is not None:
        for commit, classes in ordered:
            if multi[0] in classes:
                return multi[0], list(multi[1]), commit

    known = KNOWN.get(calc_id)
    if known is not None:
        members = known[1] if isinstance(known[1], list) else [known[1]]
        for commit, classes in ordered:
            if known[0] in classes:
                return known[0], list(members), commit
    joined = class_text(spec)
    java = [match.group(1) for match in WITH_JAVA.finditer(joined)]
    for commit, classes in ordered:
        for candidate in java:
            if candidate in GENERIC:
                continue
            if candidate in classes:
                return candidate, [member_for(spec, candidate)], commit

    ticked = [token for token in MEMBER_TOKEN.findall(joined) if token[:1].isupper()]
    for commit, classes in ordered:
        for candidate in ticked:
            if candidate in GENERIC:
                continue
            if candidate in classes:
                return candidate, [member_for(spec, candidate)], commit

    return None


def member_for(spec: dict[str, Any], class_name: str) -> str:
    """A method the spec's edition or references names, or the class itself."""
    for token in MEMBER_TOKEN.findall(member_text(spec)):
        if "/" in token or token.startswith(".") or token in NON_MEMBERS:
            continue
        if token.endswith(".java"):
            continue
        if "." in token:
            owner, _, name = token.partition(".")
            if owner == class_name and name and name != "java":
                return token
            continue
        # a commit hash or similar is not a method
        if len(token) >= 7 and all(c in "0123456789abcdef" for c in token.lower()):
            continue
        if token[0].islower() and len(token) >= 4:
            return f"{class_name}.{token}"
    return class_name


def source_label(class_name: str, members: list[str]) -> str:
    """How a value's source reads."""
    if len(members) == 1:
        member = members[0]
        if member == class_name:
            return f"NeqSim {class_name}"
        if member.startswith(f"{class_name}."):
            return f"NeqSim {member}"
    return f"NeqSim {class_name}"


def insert_source(text: str, class_name: str, members: list[str], commit: str) -> str:
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
    named = ", ".join(f'"{member}"' for member in members)
    block = [
        "",
        "[source.neqsim]",
        f'class = "{class_name}"',
        f"members = [{named}]",
        f'commit = "{commit}"',
        "",
    ]
    return "\n".join(lines[:end] + block + lines[end:]) + ("\n" if text.endswith("\n") else "")


def insert_provenance(text: str, spec: dict[str, Any], class_name: str, members: list[str]) -> str:
    source = source_label(class_name, members)
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
    parser.add_argument("--list", action="store_true", help="print the class chosen for each spec")
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
        class_name, members, commit = found
        if args.list:
            print(f"  {path.relative_to(ROOT)}: {class_name} {members} @ {commit[:7]}")
        if args.write:
            text = path.read_text(encoding="utf-8")
            text = insert_source(text, class_name, members, commit)
            text = insert_provenance(text, spec, class_name, members)
            path.write_text(text, encoding="utf-8")
            written += 1
    for path in missing:
        print(f"  no class named: {path}", file=sys.stderr)
    print(f"port_migrate: {written} written, {len(missing)} without a class")
    return 1 if missing else 0


if __name__ == "__main__":
    raise SystemExit(main())
