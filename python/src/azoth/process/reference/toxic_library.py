"""``process.toxic_library`` - the toxic-exposure thresholds and ppm conversion.

Spec: ``specs/models/process/toxic_library.toml``

The Python twin of ``crates/azoth-process/src/models/toxic_library.rs``: the static table,
its exact-then-lowercase lookup, and ``ppmToKgPerM3`` with the class's own ``8.314``.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import ToxicLibraryResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.toxic_library"

#: The class's gas constant, J/(mol K); deliberately not the analyzer's 8.314462618.
R_GAS = 8.314

#: ``(key, name, IDLH ppm, ERPG-2 ppm, AEGL-2 ppm)``, the class's map in full.
TABLE: tuple[tuple[str, str, float, float, float], ...] = (
    ("H2S", "Hydrogen sulfide", 100.0, 30.0, 27.0),
    ("hydrogen sulfide", "Hydrogen sulfide", 100.0, 30.0, 27.0),
    ("CO", "Carbon monoxide", 1200.0, 350.0, 83.0),
    ("carbon monoxide", "Carbon monoxide", 1200.0, 350.0, 83.0),
    ("Cl2", "Chlorine", 10.0, 3.0, 2.0),
    ("chlorine", "Chlorine", 10.0, 3.0, 2.0),
    ("NH3", "Ammonia", 300.0, 150.0, 160.0),
    ("ammonia", "Ammonia", 300.0, 150.0, 160.0),
    ("SO2", "Sulfur dioxide", 100.0, 3.0, 0.75),
    ("sulfur dioxide", "Sulfur dioxide", 100.0, 3.0, 0.75),
    ("HCN", "Hydrogen cyanide", 50.0, 10.0, 7.1),
    ("HF", "Hydrogen fluoride", 30.0, 20.0, 24.0),
    ("HCl", "Hydrogen chloride", 50.0, 20.0, 22.0),
    ("methanol", "Methanol", 6000.0, 1000.0, 530.0),
    ("benzene", "Benzene", 500.0, 150.0, 800.0),
)


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _lookup(chemical: str) -> tuple[str, float, float, float] | None:
    """The class's exact hit, then its lowercased hit."""
    for key, name, idlh, erpg2, aegl2 in TABLE:
        if key == chemical:
            return (name, idlh, erpg2, aegl2)
    lower = chemical.lower()
    for key, name, idlh, erpg2, aegl2 in TABLE:
        if key == lower:
            return (name, idlh, erpg2, aegl2)
    return None


def _ppm_to_kg_per_m3(
    ppm: float, molar_mass: float, temperature: float, pressure_pa: float
) -> float:
    """The class's ``ppmToKgPerM3``, with its bara round trip and ``R = 8.314``."""
    bara = pressure_pa / 1.0e5
    return (ppm / 1.0e6) * molar_mass * (bara * 1.0e5) / (R_GAS * temperature)


def _require_positive(name: str, value: float) -> None:
    if not math.isfinite(value) or value <= 0.0:
        raise InvalidInputError(name, f"a finite positive value is required, and {value} is not")


def toxic_library(
    chemical: str | None = None,
    concentration_ppm: float | None = None,
    molar_mass: Q | None = None,
    temperature: Q | None = None,
    pressure: Q | None = None,
) -> ToxicLibraryResult:
    """The threshold row for a chemical, and the ppm-to-kg/m3 conversion.

    Raises:
        InvalidInputError: when neither a chemical nor the conversion group is stated,
            when the group is incomplete, or when its values are out of range. An unknown
            chemical is not an error: ``known`` is false.
    """
    if chemical is None and concentration_ppm is None:
        raise InvalidInputError(
            "chemical", "a chemical or the conversion group is required, and neither was stated"
        )
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    ppm = None if concentration_ppm is None else float(concentration_ppm)
    molar_si = None if molar_mass is None else input_to_si(spec, "molar_mass", molar_mass)
    temperature_si = None if temperature is None else input_to_si(spec, "temperature", temperature)
    pressure_si = None if pressure is None else input_to_si(spec, "pressure", pressure)
    if ppm is not None:
        for name, value in (
            ("molar_mass", molar_si),
            ("temperature", temperature_si),
            ("pressure", pressure_si),
        ):
            if value is None:
                raise InvalidInputError(name, "is required with concentration_ppm")
    apply_checks(
        checks.on_input,
        {
            "concentration_ppm": ppm,
            "molar_mass": molar_si,
            "temperature": temperature_si,
            "pressure": pressure_si,
        }.get,
        warnings,
    )
    if ppm is not None and (not math.isfinite(ppm) or ppm < 0.0):
        raise InvalidInputError(
            "concentration_ppm",
            f"a finite non-negative concentration is required, and {ppm} is not",
        )
    for name, value in (
        ("molar_mass", molar_si),
        ("temperature", temperature_si),
        ("pressure", pressure_si),
    ):
        if value is not None:
            _require_positive(name, value)

    record = None if chemical is None else _lookup(chemical)
    mass = (
        None
        if ppm is None or molar_si is None or temperature_si is None or pressure_si is None
        else _ppm_to_kg_per_m3(ppm, molar_si, temperature_si, pressure_si)
    )
    return ToxicLibraryResult(
        known=None if chemical is None else record is not None,
        name=None if record is None else record[0],
        idlh_ppm=None if record is None else record[1],
        erpg2_ppm=None if record is None else record[2],
        aegl2_ppm=None if record is None else record[3],
        mass_concentration=None if mass is None else from_si(mass, "kg/m**3"),
        warnings=tuple(warnings),
    )
