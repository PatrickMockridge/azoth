"""``process.pipe_fire_rupture_material`` - the pipe material curves.

As NeqSim's ``PipeFireRuptureMaterial`` computes it. Spec:
``specs/models/process/pipe_fire_rupture_material.toml``

The Python twin of ``crates/azoth-process/src/models/pipe_fire_rupture_material.rs``: the
six workbook tables, their clamped interpolation, the Sellars-Tegart strain rate and the
class's temperature-correction factor.
"""

from __future__ import annotations

import math
from typing import Any, NamedTuple

from azoth.core.errors import InvalidInputError
from azoth.core.range import apply_checks, checks_for
from azoth.core.result import PipeFireRuptureMaterialResult
from azoth.core.units import Q, from_si, input_records_to_si, input_to_si
from azoth.core.warnings import Warning

MODEL_ID = "process.pipe_fire_rupture_material"

#: The class's absolute-temperature offset.
_KELVIN_OFFSET = 273.15
#: The class's own absolute-temperature offset, applied after the conversion.
_CLASS_OFFSET = 273.0
#: The class's per-minute conversion for the strain rate.
_SECONDS_PER_MINUTE = 60.0

#: The workbook temperature grid, degC.
_GRID_C = (
    0.0,
    100.0,
    200.0,
    300.0,
    400.0,
    500.0,
    600.0,
    700.0,
    750.0,
    800.0,
    900.0,
    1000.0,
    1100.0,
    1150.0,
    1350.0,
)

#: The SS316 and 6Mo workbook heat capacity, J/(kg K).
_STAINLESS_CP = (
    479.81656050955417,
    495.06496815286624,
    511.3299363057325,
    520.4789808917197,
    528.6114649681529,
    538.7770700636943,
    549.9592356687898,
    560.1248407643312,
    564.0,
    568.2573248407643,
    574.3566878980891,
    574.3566878980891,
    574.3566878980891,
    574.3566878980891,
    574.0,
)

#: The SS316 and 6Mo workbook thermal conductivity, W/(m K).
_STAINLESS_K = (
    13.5,
    14.9,
    16.7,
    18.3,
    19.8,
    21.3,
    22.7,
    24.2,
    24.8,
    25.6,
    27.1,
    28.6,
    30.5,
    34.2,
    34.2,
)


class _Table(NamedTuple):
    """One temperature-dependent material table."""

    name: str
    heat_capacity: tuple[float, ...]
    thermal_conductivity: tuple[float, ...]
    ultimate_tensile_strength: tuple[float, ...]
    strain_effect: tuple[float, ...]
    rupture_strain_limit: tuple[float, ...]
    density_kg_m3: float
    a: float
    alpha: float
    n: float
    activation_temperature_k: float
    correction_rate_per_minute: float
    correction_limit_c: float


_DUPLEX_22CR = _Table(
    "22Cr duplex",
    (480.0, 500.0, 530.0, 550.0, 590.0, 635.0, 670.0, 710.0, 730.0, 750.0, 790.0, 840.0, 870.0, 879.9, 900.0),
    (15.0, 16.0, 17.0, 18.0, 20.0, 24.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0),
    (650.0, 605.0, 553.0, 540.0, 533.0, 462.0, 371.0, 247.0, 202.0, 157.0, 78.0, 28.0, 12.0, 4.0, 0.0),
    (1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.9, 0.7, 1.0, 1.4, 2.3, 2.3, 2.3),
    (0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.23, 0.26, 0.33, 0.5, 0.5, 0.5),
    7850.0,
    12311757039.742393,
    0.03,
    2.156323825544742,
    34115.772963646705,
    0.03,
    900.0,
)

_SS_316 = _Table(
    "SS 316",
    _STAINLESS_CP,
    _STAINLESS_K,
    (485.0, 467.0, 429.0, 426.0, 421.0, 398.0, 363.0, 277.0, 208.5, 140.0, 80.0, 45.0, 20.0, 10.0, 0.0),
    (1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.7, 0.5, 0.3, 0.25, 0.3, 0.3, 0.3),
    (0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.16, 0.17, 0.18, 0.2, 0.23, 0.25, 0.3),
    7850.0,
    23449524917238.52,
    0.015,
    3.9045874502366082,
    42315.65824131732,
    0.05,
    900.0,
)

_CS_235 = _Table(
    "CS 235 ASME A106-B",
    (450.0, 480.0, 510.0, 550.0, 600.0, 660.0, 750.0, 900.0, 1450.0, 820.0, 540.0, 540.0, 540.0, 540.0, 540.0),
    (54.2, 50.95, 47.45, 43.7, 40.45, 37.2, 33.95, 30.7, 28.0, 27.4, 27.4, 27.4, 27.4, 27.4, 27.4),
    (420.0, 407.0, 397.0, 382.0, 370.0, 308.0, 189.0, 92.0, 81.5, 71.0, 53.0, 29.0, 17.0, 4.0, 0.0),
    (1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.7, 0.5, 0.5, 0.4, 0.4, 0.3, 0.3),
    (0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.2, 0.2, 0.2, 0.2, 0.3),
    7850.0,
    4425186183349.352,
    0.03,
    3.7222401933501312,
    41462.45192432549,
    0.05,
    900.0,
)

_CS_360_X52 = _Table(
    "CS 360 API 5L-X52",
    (450.0, 480.0, 510.0, 550.0, 600.0, 660.0, 750.0, 900.0, 1450.0, 820.0, 540.0, 540.0, 540.0, 540.0, 540.0),
    (54.2, 50.95, 47.45, 43.7, 40.45, 37.2, 33.95, 30.7, 28.0, 27.4, 27.4, 27.4, 27.4, 27.4, 27.4),
    (545.0, 529.0, 515.0, 496.0, 480.0, 400.0, 245.0, 120.0, 99.0, 78.0, 60.0, 38.0, 22.0, 5.0, 0.0),
    (1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5, 0.4, 0.6, 0.8, 0.9, 1.0, 1.0),
    (0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.2, 0.2, 0.2, 0.2, 0.3),
    7850.0,
    7508371948335.949,
    0.015,
    4.346909864208215,
    38301.22162583593,
    0.05,
    900.0,
)

_SUPERDUPLEX = _Table(
    "Superduplex",
    (480.0, 500.0, 530.0, 550.0, 590.0, 635.0, 670.0, 710.0, 730.0, 750.0, 790.0, 840.0, 870.0, 879.9, 900.0),
    (15.0, 16.0, 17.0, 18.0, 20.0, 24.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0, 28.0),
    (750.0, 698.0, 638.0, 638.0, 613.0, 531.0, 427.0, 284.0, 234.5, 185.0, 100.0, 31.0, 13.0, 5.0, 0.0),
    (1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.9, 0.7, 1.0, 1.2, 2.2, 2.3, 2.2),
    (0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.22, 0.23, 0.26, 0.33, 0.5, 0.5, 0.5),
    7850.0,
    352941684892.5195,
    0.03,
    2.1486991997520266,
    39301.04947184141,
    0.03,
    900.0,
)

_MO_6 = _Table(
    "6Mo",
    _STAINLESS_CP,
    _STAINLESS_K,
    (650.0, 646.0, 589.0, 557.0, 546.0, 528.0, 480.0, 380.0, 315.0, 250.0, 135.0, 90.0, 40.0, 20.0, 0.0),
    (1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.7, 0.5, 0.3, 0.3, 0.3, 0.25, 0.3),
    (0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.2, 0.2, 0.2, 0.2, 0.3),
    7850.0,
    2443499691962.7837,
    0.015,
    3.0808262954383796,
    41637.0426470313,
    0.05,
    900.0,
)


class _Solution(NamedTuple):
    """Every number the calculation produces, in SI, before the units are attached."""

    resolved_material_name: str
    density: float
    heat_capacity: float
    thermal_conductivity: float
    ultimate_tensile_strength: float
    strain_effect_factor: float
    rupture_strain_limit: float
    temperature_correction_factor: float
    strain_rate: float


def _require_finite(name: str, value: float) -> None:
    """Refuse a value the arithmetic cannot carry to a published number."""
    if not math.isfinite(value):
        raise InvalidInputError(
            name, "a non-finite value is not a number this calculation publishes"
        )


def _require_positive(name: str, value: float) -> None:
    """Refuse a non-positive constant."""
    _require_finite(name, value)
    if value <= 0.0:
        raise InvalidInputError(name, "the class's constructor refuses a non-positive value")


def _spec() -> dict[str, Any]:
    """The generated spec, imported here so the module has no import-time cycle."""
    from azoth._models_gen import model

    return model(MODEL_ID)


def _interpolate(value: float, xs: tuple[float, ...], ys: tuple[float, ...]) -> float:
    """The class's endpoint-clamped linear interpolation."""
    if value <= xs[0]:
        return ys[0]
    for index in range(1, len(xs)):
        if value <= xs[index]:
            fraction = (value - xs[index - 1]) / (xs[index] - xs[index - 1])
            return ys[index - 1] + fraction * (ys[index] - ys[index - 1])
    return ys[-1]


def _built_in(name: str) -> _Table:
    """The class's lowercased ``contains`` selector, in its order."""
    normalized = name.lower()
    if "superduplex" in normalized or "25cr" in normalized:
        return _SUPERDUPLEX
    if "22cr" in normalized or "duplex" in normalized:
        return _DUPLEX_22CR
    if "316" in normalized:
        return _SS_316
    if "360" in normalized or "x52" in normalized or "5l" in normalized:
        return _CS_360_X52
    if "235" in normalized or "a106" in normalized:
        return _CS_235
    if "6mo" in normalized or "6 mo" in normalized:
        return _MO_6
    raise InvalidInputError(
        "material_name", f"`{name}` matches no built-in pipe fire-rupture material"
    )


def _evaluate(
    material_name: str,
    evaluation_temperature: Q,
    evaluation_stress: Q,
    curve: list[dict[str, object]] | None = None,
    density: Q | None = None,
    sellars_tegart_a: float | None = None,
    sellars_tegart_alpha: float | None = None,
    sellars_tegart_n: float | None = None,
    activation_temperature: Q | None = None,
    correction_strain_rate: Q | None = None,
    correction_limit_temperature: Q | None = None,
) -> tuple[_Solution, list[Warning]]:
    """The class's arithmetic over SI values; shared with the layer dumper."""
    spec = _spec()
    checks = checks_for(spec)
    warnings: list[Warning] = []

    label = material_name.strip()
    if not label:
        raise InvalidInputError(
            "material_name", "the class's constructor refuses an empty material name"
        )
    temperature = input_to_si(spec, "evaluation_temperature", evaluation_temperature)
    stress = input_to_si(spec, "evaluation_stress", evaluation_stress)
    stated_density = None if density is None else input_to_si(spec, "density", density)
    stated_activation = (
        None
        if activation_temperature is None
        else input_to_si(spec, "activation_temperature", activation_temperature)
    )
    stated_correction = (
        None
        if correction_strain_rate is None
        else input_to_si(spec, "correction_strain_rate", correction_strain_rate)
    )
    stated_limit = (
        None
        if correction_limit_temperature is None
        else input_to_si(spec, "correction_limit_temperature", correction_limit_temperature)
    )
    apply_checks(
        checks.on_input,
        {
            "evaluation_temperature": temperature,
            "evaluation_stress": stress,
            "density": stated_density,
            "sellars_tegart_a": sellars_tegart_a,
            "sellars_tegart_alpha": sellars_tegart_alpha,
            "sellars_tegart_n": sellars_tegart_n,
            "activation_temperature": stated_activation,
            "correction_strain_rate": stated_correction,
            "correction_limit_temperature": stated_limit,
        }.get,
        warnings,
    )

    _require_finite("evaluation_temperature", temperature)
    if temperature <= 0.0:
        raise InvalidInputError(
            "evaluation_temperature", "a non-positive absolute temperature is not a temperature"
        )
    _require_finite("evaluation_stress", stress)

    supplied = sum(
        value is not None
        for value in (
            stated_density,
            sellars_tegart_a,
            sellars_tegart_alpha,
            sellars_tegart_n,
            stated_activation,
            stated_correction,
            stated_limit,
        )
    )
    if curve is None and supplied:
        raise InvalidInputError(
            "curve", "the six constants are carried with a curve; a built-in name carries neither"
        )
    if curve is not None and supplied != 7:
        raise InvalidInputError("curve", "a caller-supplied curve requires all six constants")

    evaluation_c = temperature - _KELVIN_OFFSET
    stress_mpa = stress / 1.0e6
    _require_finite("evaluation_temperature", evaluation_c)

    if curve is not None:
        rows = input_records_to_si(spec, "curve", curve)
        capacity = int(spec["algorithm"].get("max_iterations", 0))
        if len(rows) < 2:
            raise InvalidInputError("curve", "a material curve needs at least two points")
        if len(rows) > capacity:
            raise InvalidInputError(
                "curve", f"{len(rows)} points exceeds the {capacity} this procedure carries"
            )
        grid: list[float] = []
        cp: list[float] = []
        k: list[float] = []
        uts: list[float] = []
        effect: list[float] = []
        limit: list[float] = []
        previous = -math.inf
        for row in rows:
            point_temperature = float(row["temperature"])
            _require_finite("curve.temperature", point_temperature)
            if point_temperature <= 0.0:
                raise InvalidInputError(
                    "curve.temperature", "a non-positive absolute temperature is not a temperature"
                )
            point_c = point_temperature - _KELVIN_OFFSET
            if point_c <= previous:
                raise InvalidInputError(
                    "curve.temperature", "the class requires strictly increasing temperatures"
                )
            previous = point_c
            for field in ("heat_capacity", "thermal_conductivity"):
                _require_finite(f"curve.{field}", float(row[field]))
            _require_finite(
                "curve.ultimate_tensile_strength", float(row["ultimate_tensile_strength"])
            )
            _require_finite("curve.strain_effect_factor", float(row["strain_effect_factor"]))
            _require_finite("curve.rupture_strain_limit", float(row["rupture_strain_limit"]))
            grid.append(point_c)
            cp.append(float(row["heat_capacity"]))
            k.append(float(row["thermal_conductivity"]))
            uts.append(float(row["ultimate_tensile_strength"]) / 1.0e6)
            effect.append(float(row["strain_effect_factor"]))
            limit.append(float(row["rupture_strain_limit"]))
        assert stated_density is not None
        assert sellars_tegart_a is not None
        assert sellars_tegart_alpha is not None
        assert sellars_tegart_n is not None
        assert stated_activation is not None
        assert stated_correction is not None
        assert stated_limit is not None
        _require_positive("density", stated_density)
        _require_positive("sellars_tegart_a", sellars_tegart_a)
        _require_positive("sellars_tegart_alpha", sellars_tegart_alpha)
        _require_positive("sellars_tegart_n", sellars_tegart_n)
        _require_positive("activation_temperature", stated_activation)
        _require_positive("correction_strain_rate", stated_correction)
        _require_finite("correction_limit_temperature", stated_limit)
        resolved = label
        density_kg_m3 = stated_density
        a = sellars_tegart_a
        alpha = sellars_tegart_alpha
        n = sellars_tegart_n
        activation_k = stated_activation
        correction_rate_per_minute = stated_correction * _SECONDS_PER_MINUTE
        correction_limit_c = stated_limit - _KELVIN_OFFSET
    else:
        table = _built_in(label)
        resolved = table.name
        density_kg_m3 = table.density_kg_m3
        grid = list(_GRID_C)
        cp = list(table.heat_capacity)
        k = list(table.thermal_conductivity)
        uts = list(table.ultimate_tensile_strength)
        effect = list(table.strain_effect)
        limit = list(table.rupture_strain_limit)
        a = table.a
        alpha = table.alpha
        n = table.n
        activation_k = table.activation_temperature_k
        correction_rate_per_minute = table.correction_rate_per_minute
        correction_limit_c = table.correction_limit_c

    heat_capacity_at = _interpolate(evaluation_c, tuple(grid), tuple(cp))
    conductivity_at = _interpolate(evaluation_c, tuple(grid), tuple(k))
    uts_at = _interpolate(evaluation_c, tuple(grid), tuple(uts))
    effect_at = _interpolate(evaluation_c, tuple(grid), tuple(effect))
    limit_at = _interpolate(evaluation_c, tuple(grid), tuple(limit))

    absolute_temperature = evaluation_c + _CLASS_OFFSET
    if evaluation_c >= correction_limit_c:
        correction = 1.0
    else:
        hyperbolic_sine = math.sinh(alpha * max(0.0, uts_at))
        if hyperbolic_sine <= 0.0:
            correction = 1.0
        else:
            correction = absolute_temperature / activation_k * (
                math.log(hyperbolic_sine) * n
                - math.log(correction_rate_per_minute)
                + math.log(a)
            )
    effective_stress = max(0.0, stress_mpa)
    strain_rate_per_minute = (
        effect_at
        * a
        * math.sinh(alpha * effective_stress) ** n
        * math.exp(-correction * activation_k / absolute_temperature)
    )

    return (
        _Solution(
            resolved,
            density_kg_m3,
            heat_capacity_at,
            conductivity_at,
            uts_at * 1.0e6,
            effect_at,
            limit_at,
            correction,
            strain_rate_per_minute / _SECONDS_PER_MINUTE,
        ),
        warnings,
    )


def pipe_fire_rupture_material(
    material_name: str,
    evaluation_temperature: Q,
    evaluation_stress: Q,
    curve: list[dict[str, object]] | None = None,
    density: Q | None = None,
    sellars_tegart_a: float | None = None,
    sellars_tegart_alpha: float | None = None,
    sellars_tegart_n: float | None = None,
    activation_temperature: Q | None = None,
    correction_strain_rate: Q | None = None,
    correction_limit_temperature: Q | None = None,
) -> PipeFireRuptureMaterialResult:
    """The material's properties and Sellars-Tegart strain rate at one state.

    Args:
        material_name: the class's selector, or the label of a supplied curve.
        evaluation_temperature: the metal temperature.
        evaluation_stress: the von Mises stress.
        curve: the caller's ``{temperature, heat_capacity, thermal_conductivity,
            ultimate_tensile_strength, strain_effect_factor, rupture_strain_limit}`` rows;
            absent selects a built-in table.
        density: the material density; required with a curve.
        sellars_tegart_a: the pre-exponential constant; required with a curve.
        sellars_tegart_alpha: the alpha in 1/MPa; required with a curve.
        sellars_tegart_n: the exponent; required with a curve.
        activation_temperature: Q/R; required with a curve.
        correction_strain_rate: the correction rate; required with a curve.
        correction_limit_temperature: the temperature above which the correction is 1;
            required with a curve.

    Returns:
        The resolved name, the six interpolated properties, the correction factor and the
        strain rate.

    Raises:
        InvalidInputError: for an unknown name, a curve and constants that are not
            all-or-none, a grid under two points or over the row guard, a non-increasing or
            non-finite grid, a non-positive density or constitutive constant, a non-positive
            temperature, or a non-finite value.
    """
    solution, _ = _evaluate(
        material_name,
        evaluation_temperature,
        evaluation_stress,
        curve,
        density,
        sellars_tegart_a,
        sellars_tegart_alpha,
        sellars_tegart_n,
        activation_temperature,
        correction_strain_rate,
        correction_limit_temperature,
    )
    return PipeFireRuptureMaterialResult(
        resolved_material_name=solution.resolved_material_name,
        density=from_si(solution.density, "kg/m**3"),
        heat_capacity=from_si(solution.heat_capacity, "J/(kg*K)"),
        thermal_conductivity=from_si(solution.thermal_conductivity, "W/(m*K)"),
        ultimate_tensile_strength=from_si(solution.ultimate_tensile_strength, "MPa"),
        strain_effect_factor=solution.strain_effect_factor,
        rupture_strain_limit=solution.rupture_strain_limit,
        temperature_correction_factor=solution.temperature_correction_factor,
        strain_rate=from_si(solution.strain_rate, "1/s"),
    )
