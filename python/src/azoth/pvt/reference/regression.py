"""The PVT regression's parameters and the fluid they build.

Spec: ``specs/models/pvt/regression.toml``. ``PVTRegression`` retunes a characterised
fluid by applying each parameter to a clone and re-running the laboratory experiments
against the measured points. This module is that first step: the twelve carried transforms,
applied to the component rows before the mixture is assembled.

Five of the class's seventeen parameters stay owed. ``plus_molar_mass_multiplier``,
``gamma_alpha`` and ``gamma_eta`` need the mutable characterisation state the row boundary
does not carry; ``viscosity_lbc_multiplier`` and ``viscosity_pedersen_alpha`` need a
viscosity model the PFCT route never selects.
"""

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field, replace
from typing import Any

from azoth.core.errors import InvalidInputError

from ._fluid import Fluid

#: The twelve parameters the id carries, in the spec's spelling.
PARAMETERS: tuple[str, ...] = (
    "bip_methane_c7plus",
    "bip_c2c6_c7plus",
    "bip_co2_hc",
    "bip_n2_hc",
    "volume_shift_c7plus",
    "tc_multiplier_c7plus",
    "pc_multiplier_c7plus",
    "omega_multiplier_c7plus",
    "viscosity_csp_1",
    "viscosity_csp_2",
    "viscosity_csp_3",
    "viscosity_csp_4",
)

_CSP_INDEX = {
    "viscosity_csp_1": 0,
    "viscosity_csp_2": 1,
    "viscosity_csp_3": 2,
    "viscosity_csp_4": 3,
}

_PENELOUX_INTERCEPT = {"srk": 0.29441, "pr": 0.25969}


def _name(row: Mapping[str, Any]) -> str:
    return str(row["name"]).lower()


def _is_methane(row: Mapping[str, Any]) -> bool:
    return _name(row) in ("methane", "c1")


def _is_nitrogen(row: Mapping[str, Any]) -> bool:
    return _name(row) in ("nitrogen", "n2")


def _is_co2(row: Mapping[str, Any]) -> bool:
    return _name(row) in ("co2", "carbon dioxide")


def _is_c2c6(row: Mapping[str, Any]) -> bool:
    name = _name(row)
    return name in ("ethane", "c2", "propane", "c3", "c4", "c5", "c6") or any(
        part in name for part in ("butane", "pentane", "hexane")
    )


def _bip_overrides(
    rows: Sequence[Mapping[str, Any]], parameter: str, value: float
) -> list[tuple[int, int, float]]:
    plus = [index for index, row in enumerate(rows) if row["pseudo_component"]]
    pairs: list[tuple[int, int, float]] = []
    if parameter == "bip_methane_c7plus":
        index = next((i for i, row in enumerate(rows) if _is_methane(row)), None)
        if index is not None:
            pairs.extend((index, other, value) for other in plus)
    elif parameter == "bip_c2c6_c7plus":
        for index, row in enumerate(rows):
            if _is_c2c6(row):
                pairs.extend((index, other, value) for other in plus)
    elif parameter == "bip_co2_hc":
        index = next((i for i, row in enumerate(rows) if _is_co2(row)), None)
        if index is not None:
            pairs.extend((index, other, value) for other in range(len(rows)) if other != index)
    elif parameter == "bip_n2_hc":
        index = next((i for i, row in enumerate(rows) if _is_nitrogen(row)), None)
        if index is not None:
            pairs.extend((index, other, value) for other in range(len(rows)) if other != index)
    return pairs


def _field(index: int, row: dict[str, Any], field: str) -> float:
    value = row.get(field)
    if value is None:
        raise InvalidInputError(
            f"components[{index}].{field}",
            f"the parameter transforms need a pseudo row's {field}",
        )
    return float(value)


def build_fluid(
    components: Sequence[Mapping[str, Any]],
    *,
    eos: str = "srk",
    kij: str = "database",
    corrected: bool = True,
    parameters: Sequence[Mapping[str, Any]],
    values: Sequence[float],
) -> tuple[Fluid, tuple[float, float, float, float]]:
    """Apply the parameter vector to the rows, then assemble the mixture.

    Returns the fluid and the four PFCT correction factors. The transforms are the
    class's own; see the Rust twin for the volume-shift linearisation.

    Raises:
        InvalidInputError: for a value count that does not match, a non-finite value, or a
            pseudo row a multiplier cannot act on; the assembler's own refusals otherwise.
    """
    if len(parameters) != len(values):
        raise InvalidInputError(
            "parameters", f"{len(parameters)} parameter rows carry {len(values)} values"
        )
    rows = [dict(row) for row in components]
    overrides: list[tuple[int, int, float]] = []
    csp = [1.0, 1.0, 1.0, 1.0]
    for config, value in zip(parameters, values, strict=True):
        parameter = str(config["parameter"])
        if parameter not in PARAMETERS:
            raise InvalidInputError(
                "parameters", f"{parameter!r} is not one of the twelve carried parameters"
            )
        value = float(value)
        if not math.isfinite(value):
            raise InvalidInputError("parameters", f"{parameter} is not finite")
        if parameter in _CSP_INDEX:
            csp[_CSP_INDEX[parameter]] = value
            continue
        if parameter.startswith("bip_"):
            overrides.extend(_bip_overrides(rows, parameter, value))
            continue
        if parameter == "volume_shift_c7plus":
            intercept = _PENELOUX_INTERCEPT.get(eos.lower())
            if intercept is None:
                raise InvalidInputError("eos", f"{eos!r} has no Peneloux translation to scale")
            for index, row in enumerate(rows):
                if not row["pseudo_component"]:
                    continue
                rackett = row.get("racket_z")
                if rackett is None or abs(float(rackett)) < 1e-10:
                    rackett = 0.29056 - 0.08775 * _field(index, row, "acentric_factor")
                row["racket_z"] = intercept - value * (intercept - float(rackett))
            continue
        for index, row in enumerate(rows):
            if not row["pseudo_component"]:
                continue
            if parameter == "tc_multiplier_c7plus":
                row["critical_temperature"] = _field(index, row, "critical_temperature") * value
            elif parameter == "pc_multiplier_c7plus":
                row["critical_pressure"] = _field(index, row, "critical_pressure") * value
            elif parameter == "omega_multiplier_c7plus":
                row["acentric_factor"] = _field(index, row, "acentric_factor") * value
            else:
                raise InvalidInputError("parameters", f"no transform for {parameter!r}")
    fluid = Fluid.from_rows_with_kij(
        rows, eos=eos, kij=kij, kij_overrides=overrides, corrected=corrected
    )
    return fluid, (csp[0], csp[1], csp[2], csp[3])


#: The experiments the fit carries. SEPARATOR stays owed.
EXPERIMENTS: tuple[str, ...] = ("cce", "cvd", "dle", "viscosity")

#: The phase names a viscosity observation accepts, in the class's mapping.
PHASES: dict[str, str] = {
    "gas": "gas",
    "vapor": "gas",
    "vapour": "gas",
    "oil": "oil",
    "liquid": "oil",
    "aqueous": "aqueous",
    "water": "aqueous",
}


@dataclass(frozen=True)
class CceExperiment:
    """One CCE's schedule and its measured relative volumes."""

    temperature: float
    pressures: tuple[float, ...]
    relative_volumes: tuple[float, ...]


@dataclass(frozen=True)
class CvdExperiment:
    """One CVD's schedule and its measured dropout and gas Z."""

    temperature: float
    pressures: tuple[float, ...]
    liquid_dropout: tuple[float, ...]
    z_factor: tuple[float, ...]


@dataclass(frozen=True)
class DleExperiment:
    """One DLE's schedule and its measured Rs, Bo and oil density."""

    temperature: float
    pressures: tuple[float, ...]
    rs: tuple[float, ...]
    bo: tuple[float, ...]
    oil_density: tuple[float, ...]


@dataclass(frozen=True)
class ViscosityPoint:
    """One measured viscosity at one state."""

    temperature: float
    pressure: float
    viscosity: float
    phase: str


@dataclass(frozen=True)
class ExperimentWeights:
    """The class's per-experiment weights."""

    cce: float = 1.0
    cvd: float = 1.0
    dle: float = 1.0
    viscosity: float = 0.5


@dataclass(frozen=True)
class RegressionData:
    """Everything the fit is held to."""

    cce: tuple[CceExperiment, ...] = ()
    cvd: tuple[CvdExperiment, ...] = ()
    dle: tuple[DleExperiment, ...] = ()
    viscosity: tuple[ViscosityPoint, ...] = ()
    weights: ExperimentWeights = field(default_factory=ExperimentWeights)


@dataclass(frozen=True)
class Sample:
    """One observed value, its standard deviation and the model address."""

    observed: float
    sigma: float
    kind: tuple[Any, ...]


@dataclass(frozen=True)
class Objectives:
    """The class's second ledger, by experiment."""

    cce: float | None = None
    cvd: float | None = None
    dle: float | None = None
    viscosity: float | None = None

    def total(self) -> float:
        """The sum in the class's declaration order."""
        return (self.cce or 0.0) + (self.cvd or 0.0) + (self.dle or 0.0) + (self.viscosity or 0.0)


def _positive(name: str, value: float) -> float:
    if math.isfinite(value) and value > 0.0:
        return value
    raise InvalidInputError(name, f"a positive finite value is needed; got {value}")


def _weight(name: str, value: float) -> float:
    if math.isfinite(value) and value > 0.0:
        return value
    raise InvalidInputError(name, f"a weight must be finite and above zero; got {value}")


def samples(data: RegressionData) -> list[Sample]:
    """The samples in the class's registration order, with its standard deviations."""
    _weight("cce_weight", data.weights.cce)
    _weight("cvd_weight", data.weights.cvd)
    _weight("dle_weight", data.weights.dle)
    _weight("viscosity_weight", data.weights.viscosity)
    out: list[Sample] = []
    for experiment, cce in enumerate(data.cce):
        _positive("cce.temperature", cce.temperature)
        if len(cce.pressures) != len(cce.relative_volumes):
            raise InvalidInputError("cce", "the schedule and its relative volumes differ in length")
        for point, (pressure, relative) in enumerate(
            zip(cce.pressures, cce.relative_volumes, strict=True)
        ):
            _positive(f"cce.pressures[{point}]", pressure)
            _positive(f"cce.relative_volumes[{point}]", relative)
            out.append(Sample(relative, 0.01 / data.weights.cce, ("cce", experiment, point)))
    for experiment, cvd in enumerate(data.cvd):
        _positive("cvd.temperature", cvd.temperature)
        if not (len(cvd.pressures) == len(cvd.liquid_dropout) == len(cvd.z_factor)):
            raise InvalidInputError("cvd", "the schedule, its dropout and its Z differ in length")
        for row, pressure in enumerate(cvd.pressures):
            _positive(f"cvd.pressures[{row}]", pressure)
            out.append(
                Sample(
                    cvd.liquid_dropout[row],
                    0.01 / data.weights.cvd,
                    ("cvd", experiment, row, "liquid_dropout"),
                )
            )
            out.append(
                Sample(
                    cvd.z_factor[row],
                    0.01 / data.weights.cvd,
                    ("cvd", experiment, row, "gas_z"),
                )
            )
    for experiment, dle in enumerate(data.dle):
        _positive("dle.temperature", dle.temperature)
        if not (len(dle.pressures) == len(dle.rs) == len(dle.bo) == len(dle.oil_density)):
            raise InvalidInputError(
                "dle", "the schedule and its three observations differ in length"
            )
        for row, pressure in enumerate(dle.pressures):
            _positive(f"dle.pressures[{row}]", pressure)
            for property_name, observed in (
                ("rs", dle.rs[row]),
                ("bo", dle.bo[row]),
                ("oil_density", dle.oil_density[row]),
            ):
                out.append(
                    Sample(
                        observed,
                        max(abs(observed) * 0.01, 1e-3) / data.weights.dle,
                        ("dle", experiment, row, property_name),
                    )
                )
    for point, viscosity in enumerate(data.viscosity):
        _positive(f"viscosity[{point}].temperature", viscosity.temperature)
        _positive(f"viscosity[{point}].pressure", viscosity.pressure)
        if viscosity.phase not in ("gas", "oil", "aqueous"):
            raise InvalidInputError(
                f"viscosity[{point}].phase", f"{viscosity.phase!r} is not a phase"
            )
        out.append(
            Sample(
                viscosity.viscosity,
                max(abs(viscosity.viscosity) * 0.02, 1e-10) / data.weights.viscosity,
                ("viscosity", point),
            )
        )
    return out


def _viscosity_value(fluid: Fluid, csp: tuple[float, ...], point: ViscosityPoint) -> float:
    from . import _flash
    from ._transport import phase_viscosity_with_csp

    flash = fluid.flash(point.temperature, point.pressure, _flash.TWO_PHASE)
    phase = {
        "gas": flash.gas(),
        "oil": flash.oil(),
        "aqueous": flash.aqueous(),
    }[point.phase]
    if phase is None:
        return 0.0
    return phase_viscosity_with_csp(
        fluid.mixture, point.temperature, point.pressure, list(phase.x), csp
    )


def sample_value(
    fluid: Fluid, csp: tuple[float, ...], sample: Sample, data: RegressionData
) -> float:
    """The model value one sample reads."""
    kind = sample.kind
    if kind[0] == "cce":
        from .constant_mass_expansion import _constant_mass_expansion_of

        experiment = data.cce[kind[1]]
        result = _constant_mass_expansion_of(
            fluid, experiment.temperature, [experiment.pressures[kind[2]]]
        )
        return float(result.points[0].relative_volume)
    if kind[0] == "cvd":
        from .constant_volume_depletion import _constant_volume_depletion_of

        experiment = data.cvd[kind[1]]
        result = _constant_volume_depletion_of(
            fluid, experiment.temperature, list(experiment.pressures)
        )
        point = result.points[kind[2]]
        if kind[3] == "liquid_dropout":
            return float(point.liquid_volume_relative_to_vsat or 0.0)
        return float(point.z_gas or 0.0)
    if kind[0] == "dle":
        from .differential_liberation import _differential_liberation_of

        experiment = data.dle[kind[1]]
        result = _differential_liberation_of(
            fluid, experiment.temperature, list(experiment.pressures)
        )
        point = result.points[kind[2]]
        if kind[3] == "rs":
            return float(point.rs or 0.0)
        if kind[3] == "bo":
            return float(point.bo or 1.0)
        return float(point.oil_density.to("kg/m**3").magnitude)
    if kind[0] == "viscosity":
        return _viscosity_value(fluid, csp, data.viscosity[kind[1]])
    raise InvalidInputError("samples", f"unknown sample kind {kind!r}")


def chi_square(
    fluid: Fluid, csp: tuple[float, ...], rows: Sequence[Sample], data: RegressionData
) -> float:
    """The weighted chi-square the optimizer minimises."""
    total = 0.0
    for sample in rows:
        value = sample_value(fluid, csp, sample, data)
        residual = (value - sample.observed) / sample.sigma
        total += residual * residual
    return total


def objectives(fluid: Fluid, csp: tuple[float, ...], data: RegressionData) -> Objectives:
    """The class's second ledger, computed from the tuned fluid."""
    values = Objectives()
    if data.cce:
        from .constant_mass_expansion import _constant_mass_expansion_of

        temperature = data.cce[0].temperature
        pressures = [pressure for experiment in data.cce for pressure in experiment.pressures]
        result = _constant_mass_expansion_of(fluid, temperature, pressures)
        total = 0.0
        index = 0
        for experiment in data.cce:
            for observed in experiment.relative_volumes:
                calculated = float(result.points[index].relative_volume)
                total += ((calculated - observed) / observed) ** 2
                index += 1
        if index:
            values = replace(values, cce=total / index)
    if data.cvd:
        from .constant_volume_depletion import _constant_volume_depletion_of

        total = 0.0
        points = 0
        for experiment in data.cvd:
            result = _constant_volume_depletion_of(
                fluid, experiment.temperature, list(experiment.pressures)
            )
            for row, point in enumerate(result.points):
                dropout = float(point.liquid_volume_relative_to_vsat or 0.0)
                observed = experiment.liquid_dropout[row]
                total += ((dropout - observed) / max(observed, 0.01)) ** 2
                z = float(point.z_gas or 0.0)
                observed_z = experiment.z_factor[row]
                total += ((z - observed_z) / max(abs(observed_z), 1e-12)) ** 2
            points += len(experiment.pressures)
        if points:
            values = replace(values, cvd=total / (2 * points))
    if data.dle:
        from .differential_liberation import _differential_liberation_of

        total = 0.0
        points = 0
        for experiment in data.dle:
            result = _differential_liberation_of(
                fluid, experiment.temperature, list(experiment.pressures)
            )
            for row, point in enumerate(result.points):
                calculated = (
                    float(point.rs or 0.0),
                    float(point.bo or 1.0),
                    float(point.oil_density.to("kg/m**3").magnitude),
                )
                observed = (
                    experiment.rs[row],
                    experiment.bo[row],
                    experiment.oil_density[row],
                )
                for value, target in zip(calculated, observed, strict=True):
                    if target > 0.0:
                        total += ((value - target) / target) ** 2
            points += len(experiment.pressures)
        if points:
            values = replace(values, dle=total / (3 * points))
    if data.viscosity:
        total = 0.0
        for point in data.viscosity:
            calculated = _viscosity_value(fluid, csp, point)
            if point.viscosity > 0.0:
                total += ((calculated - point.viscosity) / point.viscosity) ** 2
        values = replace(values, viscosity=total / len(data.viscosity))
    return values
