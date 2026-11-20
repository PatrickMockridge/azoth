"""The rupture studies' material strength and flange ratings.

The Python twin of ``crates/azoth-process/src/models/material_strength.rs``:
``MaterialStrengthCurve``'s API 5L fallback table, the generic retained-strength grid,
the thin-wall von Mises stress and the B16.5 flange tables.
"""

from __future__ import annotations

import math

from azoth.core.errors import InvalidInputError

#: The class's psi-to-pascal factor.
PSI_TO_PA = 6894.76

_RETAINED_TEMPERATURES = (293.15, 373.15, 473.15, 573.15, 673.15, 773.15, 873.15, 973.15, 1073.15)
_RETAINED_FACTORS = (1.00, 0.95, 0.85, 0.72, 0.52, 0.32, 0.16, 0.07, 0.03)


def normalize_grade(grade: str) -> str:
    """The class's ``Api5LGrade.normalize``."""
    if not grade.strip():
        raise InvalidInputError("grade", "Pipe material grade must not be empty")
    key = grade.strip().upper()
    if key.startswith("GRADE "):
        key = key[6:].strip()
    metric = (
        "L175",
        "L210",
        "L245",
        "L290",
        "L320",
        "L360",
        "L390",
        "L415",
        "L450",
        "L485",
        "L555",
        "L625",
        "L690",
        "L830",
    )
    imperial = (
        "A25",
        "A",
        "B",
        "X42",
        "X46",
        "X52",
        "X56",
        "X60",
        "X65",
        "X70",
        "X80",
        "X90",
        "X100",
        "X120",
    )
    if key in metric:
        return imperial[metric.index(key)]
    return key


def api5l_strengths(grade: str) -> tuple[float, float]:
    """The class's built-in API 5L fallback strengths, Pa."""

    def psi(yield_psi: float, tensile_psi: float) -> tuple[float, float]:
        return (yield_psi * PSI_TO_PA, tensile_psi * PSI_TO_PA)

    table: dict[str, tuple[float, float]] = {
        "B": psi(35000.0, 60000.0),
        "GRADE B": psi(35000.0, 60000.0),
        "X42": psi(42000.0, 60000.0),
        "X52": psi(52000.0, 66000.0),
        "X60": psi(60000.0, 75000.0),
        "X65": psi(65000.0, 77000.0),
        "X70": psi(70000.0, 82000.0),
        "X80": psi(80000.0, 90000.0),
        "A25": psi(25000.0, 45000.0),
        "A": psi(30000.0, 48000.0),
        "X46": psi(46000.0, 63000.0),
        "X56": psi(56000.0, 71000.0),
        "X90": (620.0e6, 695.0e6),
        "X100": psi(100000.0, 110000.0),
        "X120": psi(120000.0, 130000.0),
    }
    if grade not in table:
        raise InvalidInputError("grade", f"No API 5L material strength data for grade {grade}")
    return table[grade]


def retained_factor(temperature_k: float) -> float:
    """The class's endpoint-clamped retained-strength interpolation."""
    if temperature_k <= _RETAINED_TEMPERATURES[0]:
        return _RETAINED_FACTORS[0]
    for index in range(1, len(_RETAINED_TEMPERATURES)):
        if temperature_k <= _RETAINED_TEMPERATURES[index]:
            fraction = (temperature_k - _RETAINED_TEMPERATURES[index - 1]) / (
                _RETAINED_TEMPERATURES[index] - _RETAINED_TEMPERATURES[index - 1]
            )
            return _RETAINED_FACTORS[index - 1] + fraction * (
                _RETAINED_FACTORS[index] - _RETAINED_FACTORS[index - 1]
            )
    return _RETAINED_FACTORS[-1]


def von_mises_pa(pressure_pa: float, radius_m: float, wall_m: float) -> float:
    """The class's thin-wall von Mises stress, Pa."""
    hoop = pressure_pa * radius_m / wall_m
    axial = pressure_pa * radius_m / (2.0 * wall_m)
    return math.sqrt(hoop * hoop + axial * axial - hoop * axial)


def allowable_rupture_stress_pa(
    ambient_tensile_pa: float,
    metal_temperature_k: float,
    tensile_strength_factor: float,
) -> float:
    """The class's ``allowableRuptureStressAt``, Pa."""
    return ambient_tensile_pa * retained_factor(metal_temperature_k) * tensile_strength_factor


def flange_class_rating_pa(flange_class: int) -> float:
    """The class's ambient B16.5 flange rating, Pa; an unconfigured class is ``+inf``."""
    table = {
        150: 19.6e5,
        300: 51.0e5,
        600: 102.0e5,
        900: 153.0e5,
        1500: 255.0e5,
        2500: 425.0e5,
    }
    return table.get(flange_class, math.inf)


def temperature_reduced_flange_rating_pa(
    ambient_rating_pa: float, metal_temperature_k: float
) -> float:
    """The class's temperature-reduced flange rating, Pa."""
    if not math.isfinite(ambient_rating_pa):
        return ambient_rating_pa
    temperature_c = metal_temperature_k - 273.15
    if temperature_c <= 100.0:
        factor = 1.0
    elif temperature_c <= 200.0:
        factor = 0.90
    elif temperature_c <= 300.0:
        factor = 0.75
    elif temperature_c <= 400.0:
        factor = 0.55
    elif temperature_c <= 500.0:
        factor = 0.35
    elif temperature_c <= 600.0:
        factor = 0.20
    else:
        factor = 0.10
    return ambient_rating_pa * factor
