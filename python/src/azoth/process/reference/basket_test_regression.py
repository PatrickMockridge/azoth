"""``process.basket_test_regression`` - the basket-test fit for E and P.

As NeqSim's ``BasketTestRegression.regress`` computes it. Spec:
``specs/models/process/basket_test_regression.toml``

The Python twin of ``crates/azoth-process/src/models/basket_test_regression.rs``.
"""

from __future__ import annotations

import math
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import BasketTestRegressionResult
from azoth.core.units import Q, from_si, input_records_to_si, input_to_si
from azoth.core.warnings import Warning
from azoth.process.reference._selfheating import GEOMETRIES, R_GAS, delta_crit_of

MODEL_ID = "process.basket_test_regression"

#: The number of distinct sizes the class recommends.
RECOMMENDED_POINT_COUNT = 3


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def basket_test_regression(
    points: list[dict[str, object]],
    effective_conductivity: Q | None = None,
) -> BasketTestRegressionResult:
    """The least-squares basket-test fit.

    Raises:
        InvalidInputError: for fewer than two points, points that all share a critical
            temperature, an unknown geometry, a non-positive dimension or critical
            temperature, or a non-positive effective conductivity.
    """
    spec = _spec()
    warnings: list[Warning] = []
    conductivity = (
        None
        if effective_conductivity is None
        else input_to_si(spec, "effective_conductivity", effective_conductivity)
    )
    apply_checks(
        checks_for(spec).on_input,
        {"effective_conductivity": conductivity}.get,
        warnings,
    )
    rows: list[dict[str, Any]] = input_records_to_si(spec, "points", points)
    if len(rows) < 2:
        raise InvalidInputError(
            "points", "At least two basket-test points are required to fit an activation energy"
        )

    n = len(rows)
    messages = []
    if n < RECOMMENDED_POINT_COUNT:
        messages.append(
            f"Only {n} basket sizes supplied; at least {RECOMMENDED_POINT_COUNT} are "
            "recommended for a defensible fit and a meaningful coefficient of determination"
        )

    x: list[float] = []
    y: list[float] = []
    for index, row in enumerate(rows):
        geometry = str(row["geometry"])
        if geometry not in GEOMETRIES:
            raise InvalidInputError(
                f"points[{index}].geometry",
                f"{geometry!r} is not one of the basket shapes",
            )
        dimension = float(row["characteristic_dimension"])
        temperature = float(row["critical_temperature"])
        if not math.isfinite(dimension) or dimension <= 0.0:
            raise InvalidInputError(
                f"points[{index}].characteristic_dimension",
                f"{dimension} must be finite and positive",
            )
        if not math.isfinite(temperature) or temperature <= 0.0:
            raise InvalidInputError(
                f"points[{index}].critical_temperature",
                f"{temperature} must be finite and positive",
            )
        x.append(1.0 / temperature)
        y.append(
            math.log(delta_crit_of(geometry) * temperature * temperature / (dimension * dimension))
        )

    mean_x = sum(x) / n
    mean_y = sum(y) / n
    sxx = sum((value - mean_x) * (value - mean_x) for value in x)
    sxy = sum((x[index] - mean_x) * (y[index] - mean_y) for index in range(n))
    if not sxx > 0.0:
        raise InvalidInputError(
            "points",
            "All basket-test points share the same critical temperature; cannot fit a slope",
        )
    slope = sxy / sxx
    intercept = mean_y - slope * mean_x

    ss_tot = sum((value - mean_y) * (value - mean_y) for value in y)
    ss_res = sum((y[index] - (intercept + slope * x[index])) ** 2 for index in range(n))
    r_squared = None if not ss_tot > 0.0 else 1.0 - ss_res / ss_tot

    activation_energy = -slope * R_GAS
    if not activation_energy > 0.0:
        messages.append(
            "Fitted activation energy is not positive; the critical temperature should fall "
            "as basket size increases, so check the ordering and units of the input data"
        )
    conductivity_provided = conductivity is not None
    if conductivity is not None and activation_energy > 0.0:
        volumetric_pre_factor: float | None = (
            math.exp(intercept) * conductivity * R_GAS / activation_energy
        )
    else:
        volumetric_pre_factor = None
        if conductivity is None:
            messages.append(
                "Effective thermal conductivity not supplied; the volumetric heat-release "
                "pre-factor could not be separated from the regression intercept"
            )

    return BasketTestRegressionResult(
        point_count=float(n),
        slope=slope,
        intercept=intercept,
        r_squared=r_squared,
        activation_energy=from_si(activation_energy, "J/mol"),
        volumetric_pre_factor=volumetric_pre_factor,
        effective_conductivity=None if conductivity is None else from_si(conductivity, "W/(m*K)"),
        conductivity_provided=conductivity_provided,
        messages=tuple(messages),
        warnings=tuple(warnings),
    )
