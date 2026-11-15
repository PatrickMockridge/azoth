"""``process.fire_case_relief`` - the API 521 fire-case relief load.

As NeqSim's ``FireCaseRelief`` no-fluid path computes it. Spec:
``specs/models/process/fire_case_relief.toml``

The Python twin of ``crates/azoth-process/src/models/fire_case_relief.rs``: the wetted
area, Table 4's heat input and ``Q / latent_heat``.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.range import apply_checks, checks_for
from azoth.core.result import FireCaseReliefResult
from azoth.core.units import Q, from_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.fire_case_relief"


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _fire_heat_input(area_m2: float, has_drainage: bool) -> float:
    area_ft2 = area_m2 * 10.7639
    coefficient = 21_000.0 if has_drainage else 34_500.0
    heat_input: float = coefficient * area_ft2**0.82 * 0.29307107
    return heat_input


def fire_case_relief(
    has_drainage: bool,
    has_fire_fighting: bool,
    wetted_area: Q | None = None,
    vessel_diameter: Q | None = None,
    wetted_height: Q | None = None,
    latent_heat: float | None = None,
    relieving_temperature: Q | None = None,
) -> FireCaseReliefResult:
    """The wetted area, the Table 4 heat input and the vapour relief rate.

    Raises:
        InvalidInputError: for a negative area, diameter, height or latent heat, or a
            non-positive relieving temperature.
    """
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []
    _ = has_fire_fighting

    area_si = None if wetted_area is None else input_to_si(spec, "wetted_area", wetted_area)
    diameter_si = (
        None if vessel_diameter is None else input_to_si(spec, "vessel_diameter", vessel_diameter)
    )
    height_si = None if wetted_height is None else input_to_si(spec, "wetted_height", wetted_height)
    temperature_si = (
        None
        if relieving_temperature is None
        else input_to_si(spec, "relieving_temperature", relieving_temperature)
    )
    apply_checks(
        checks.on_input,
        {
            "wetted_area": area_si,
            "vessel_diameter": diameter_si,
            "wetted_height": height_si,
            "latent_heat": latent_heat,
            "relieving_temperature": temperature_si,
        }.get,
        warnings,
    )

    if area_si is not None and area_si > 0.0:
        area = area_si
    elif (
        diameter_si is not None and height_si is not None and diameter_si > 0.0 and height_si > 0.0
    ):
        area = math.pi * diameter_si * height_si
    else:
        area = 0.0
    heat_input = _fire_heat_input(area, has_drainage) if area > 0.0 else 0.0
    latent = 350_000.0 if latent_heat is None or latent_heat <= 0.0 else latent_heat
    relief_rate = heat_input / latent
    return FireCaseReliefResult(
        latent_heat=latent,
        relief_rate=from_si(relief_rate, "kg/s"),
        relief_rate_kg_per_hr=from_si(relief_rate, "kg/h"),
        relieving_temperature=(None if temperature_si is None else from_si(temperature_si, "K")),
        warnings=tuple(warnings),
    )
