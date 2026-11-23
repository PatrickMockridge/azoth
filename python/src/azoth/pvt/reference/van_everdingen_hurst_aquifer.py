"""``pvt.van_everdingen_hurst_aquifer`` - the radial aquifer influence functions.

The Python twin of ``crates/azoth-pvt/src/van_everdingen_hurst_aquifer.rs``: the positive
spectral quadrature, the class's legacy bounded-outer-radius policy, the dimensionless time and
constant, the Carter-Tracy recurrence and the influence table. ``RadialInfiniteInfluence`` is
this module's kernel; it is package-private upstream and is not a separate id.
"""

from __future__ import annotations

import math
import sys
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import VanEverdingenHurstAquiferResult
from azoth.core.units import input_to_si, quantity

MODEL_ID = "pvt.van_everdingen_hurst_aquifer"

_EULER = 0.5772156649015329
_LOWER_LOG = -40.0
_UPPER = 64.0
_LOWER_SQUARED = math.exp(2.0 * _LOWER_LOG)
_SMALL_ARGUMENT = 1.0e-3
_NEGLIGIBLE_ARGUMENT = 80.0
_TAIL_UPPER_LOG = 420.0
_TAIL = (1.0, -3.0 / 8.0, 63.0 / 128.0, -1899.0 / 1024.0, 543483.0 / 32768.0)
_PREFIX_PANELS = 96
_PI = math.pi
_MIN_NORMAL = sys.float_info.min
_MAX = sys.float_info.max


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _tail_panels() -> int:
    return math.ceil((_TAIL_UPPER_LOG - math.log(_UPPER)) / 0.5)


def _gauss_panels(
    lower: float, upper: float, panels: int, panels_to_build: int
) -> tuple[list[float], list[float]]:
    nodes = (
        -0.9602898564975363,
        -0.7966664774136267,
        -0.5255324099163290,
        -0.1834346424956498,
        0.1834346424956498,
        0.5255324099163290,
        0.7966664774136267,
        0.9602898564975363,
    )
    weights = (
        0.1012285362903763,
        0.2223810344533745,
        0.3137066458778873,
        0.3626837833783620,
        0.3626837833783620,
        0.3137066458778873,
        0.2223810344533745,
        0.1012285362903763,
    )
    width = (upper - lower) / panels
    out_nodes: list[float] = []
    out_weights: list[float] = []
    for panel in range(panels_to_build):
        for node, weight in zip(nodes, weights, strict=True):
            out_nodes.append(lower + width * (panel + 0.5 * (node + 1.0)))
            out_weights.append(weight * width * 0.5)
    return out_nodes, out_weights


def _modulus_squared(u: float) -> float:
    if u < 12.0:
        term = 0.5 * u
        j = term
        harmonic = 0.0
        total = term * (1.0 - 2.0 * _EULER)
        for k in range(1, 100):
            term *= -0.25 * u * u / (k * (k + 1.0))
            harmonic += 1.0 / k
            j += term
            total += term * (2.0 * harmonic + 1.0 / (k + 1.0) - 2.0 * _EULER)
            if abs(term) < 1.0e-20:
                y = (2.0 * math.log(0.5 * u) * j - 2.0 / u - total) / _PI
                return j * j + y * y
        raise RuntimeError("aquifer Bessel modulus series did not converge")
    p = 1.0
    q = 0.0
    term = 1.0
    previous = 1.0
    for k in range(1, 65):
        odd = 2.0 * k - 1.0
        term *= (4.0 - odd * odd) / (8.0 * k * u)
        if abs(term) > previous:
            break
        signed = term if (k // 2) % 2 == 0 else -term
        if k % 2 == 0:
            p += signed
        else:
            q += signed
        previous = abs(term)
    return 2.0 / (_PI * u) * (p * p + q * q)


def _build_quadrature() -> tuple[list[float], list[float]]:
    panels = math.ceil((math.log(_UPPER) - _LOWER_LOG) / 0.5)
    nodes, weights = _gauss_panels(_LOWER_LOG, math.log(_UPPER), panels, panels)
    for index, node in enumerate(nodes):
        u = math.exp(node)
        nodes[index] = u * u
        weights[index] *= 4.0 / (_PI * _PI * u * u * _modulus_squared(u))
    return nodes, weights


def _tail_quadrature(panels_to_build: int) -> tuple[list[float], list[float], list[float]]:
    nodes, ordinary = _gauss_panels(
        math.log(_UPPER), _TAIL_UPPER_LOG, _tail_panels(), panels_to_build
    )
    frequencies: list[float] = []
    weights: list[float] = []
    suffix: list[float] = []
    for index, node in enumerate(nodes):
        u = math.exp(node)
        inverse_squared = (1.0 / u) / u
        polynomial = _TAIL[-1]
        for k in range(len(_TAIL) - 2, -1, -1):
            polynomial = _TAIL[k] + inverse_squared * polynomial
        frequencies.append(u)
        weights.append((2.0 / _PI) * ordinary[index] / u * polynomial)
        suffix.append(0.0)
    boundary = math.log(_UPPER) + (
        _TAIL_UPPER_LOG - math.log(_UPPER)
    ) / _tail_panels() * panels_to_build
    maximum = 0.0 if panels_to_build == _tail_panels() else math.exp(-boundary)
    for index in range(len(weights) - 1, -1, -1):
        maximum = max(maximum, weights[index])
        suffix[index] = maximum
    return frequencies, weights, suffix


class _Tables:
    """The kernel's fixed grids, built once."""

    __slots__ = ("low_quadrature", "ein_at_fifty", "quadrature", "tail_prefix")

    def __init__(self) -> None:
        self.low_quadrature = _gauss_panels(0.0, 1.0, 16, 16)
        self.ein_at_fifty = _small_frequency_integral_with(self.low_quadrature, 50.0)[0]
        self.quadrature = _build_quadrature()
        self.tail_prefix = _tail_quadrature(_PREFIX_PANELS)


_TABLES: _Tables | None = None
_FULL_TAIL: tuple[list[float], list[float], list[float]] | None = None


def _tables() -> _Tables:
    global _TABLES
    if _TABLES is None:
        _TABLES = _Tables()
    return _TABLES


def _full_tail() -> tuple[list[float], list[float], list[float]]:
    global _FULL_TAIL
    if _FULL_TAIL is None:
        _FULL_TAIL = _tail_quadrature(_tail_panels())
    return _FULL_TAIL


def _small_frequency_integral_with(
    low: tuple[list[float], list[float]], x: float
) -> tuple[float, float]:
    if x <= _SMALL_ARGUMENT:
        return (
            x * (1.0 - x * (0.25 - x * (1.0 / 18.0 - x / 96.0))),
            1.0 - x * (0.5 - x * (1.0 / 6.0 - x / 24.0)),
        )
    value = 0.0
    derivative = 0.0
    for node, weight in zip(low[0], low[1], strict=True):
        value += weight * -math.expm1(-x * node) / node
        derivative += weight * math.exp(-x * node)
    return value, derivative


def _loss_and_slope(argument: float) -> tuple[float, float]:
    if argument <= _SMALL_ARGUMENT:
        return (
            argument * (1.0 - argument * (0.5 - argument * (1.0 / 6.0 - argument / 24.0))),
            1.0 - argument * (1.0 - argument * (0.5 - argument / 6.0)),
        )
    if argument >= _NEGLIGIBLE_ARGUMENT:
        return 1.0, 0.0
    return -math.expm1(-argument), math.exp(-argument)


def _evaluate(time: float) -> tuple[float, float]:
    if not math.isfinite(time) or time < 0.0:
        raise InvalidInputError(
            "dimensionless_time",
            "the dimensionless aquifer time must be finite and nonnegative",
        )
    if time == 0.0:
        return 0.0, 0.0
    tables = _tables()
    x = time * _LOWER_SQUARED
    if x > 50.0:
        low = (0.5 * (tables.ein_at_fifty + math.log(x / 50.0)), 0.5 / time)
    else:
        low = _small_frequency_integral_with(tables.low_quadrature, x)
    if x <= 50.0:
        low = (low[0] * 0.5, low[1] * 0.5 * _LOWER_SQUARED)
    pressure = low[0]
    derivative = low[1]
    for frequency_squared, weight in zip(
        tables.quadrature[0], tables.quadrature[1], strict=True
    ):
        argument = time * frequency_squared
        loss, slope = _loss_and_slope(argument)
        pressure += weight * loss
        derivative += weight * frequency_squared * slope
    root_time = math.sqrt(time)
    tail = tables.tail_prefix
    for index in range(_tail_panels() * 8):
        if index == len(tail[0]):
            tail = _full_tail()
        scaled_frequency = root_time * tail[0][index]
        argument = scaled_frequency * scaled_frequency
        if argument >= _NEGLIGIBLE_ARGUMENT and tail[2][index] < 0.25 * math.ulp(pressure):
            break
        weight = tail[1][index]
        if argument >= _NEGLIGIBLE_ARGUMENT:
            pressure += weight
        else:
            loss, slope = _loss_and_slope(argument)
            pressure += weight * loss
            derivative += weight * argument * slope / time
    return pressure, derivative


def _influence(time: float, outer_radius: float | None) -> tuple[float, float]:
    radius = math.inf if outer_radius is None else float(outer_radius)
    if math.isnan(radius):
        raise InvalidInputError(
            "dimensionless_outer_radius",
            "the aquifer outer-radius argument must not be NaN",
        )
    result = _evaluate(time)
    if time == 0.0 or radius <= 0.0 or math.isinf(radius):
        return result
    denominator = radius * radius - 1.0
    if denominator > 1.0e-30:
        bounded = 2.0 * time / denominator + math.log(radius) - 0.75
        if bounded > result[0]:
            result = (bounded, 2.0 / denominator)
    return result


def dimensionless_pressure(time: float, outer_radius: float | None = None) -> float:
    """The infinite-acting pressure, or the legacy bounded-outer-radius maximum."""
    return _influence(time, outer_radius)[0]


def dimensionless_pressure_derivative(time: float, outer_radius: float | None = None) -> float:
    """The derivative of the selected dimensionless-pressure branch."""
    return _influence(time, outer_radius)[1]


def _normal_positive(value: float) -> bool:
    return math.isfinite(value) and value >= _MIN_NORMAL


def _get_exponent(value: float) -> int:
    if value == 0.0:
        return -1023
    if not math.isfinite(value):
        return 1024
    if abs(value) < _MIN_NORMAL:
        return -1023
    return math.frexp(value)[1] - 1


def dimensionless_time(
    permeability: float,
    time: float,
    porosity: float,
    viscosity: float,
    total_compressibility: float,
    reservoir_radius: float,
) -> float:
    """The dimensionless time, with the class's exponent-scaled fallback."""
    if (
        not math.isfinite(permeability)
        or permeability < 0.0
        or not math.isfinite(time)
        or time < 0.0
        or not math.isfinite(porosity)
        or porosity <= 0.0
        or porosity > 1.0
        or not math.isfinite(viscosity)
        or viscosity <= 0.0
        or not math.isfinite(total_compressibility)
        or total_compressibility <= 0.0
        or not math.isfinite(reservoir_radius)
        or reservoir_radius <= 0.0
    ):
        raise InvalidInputError(
            "dimensionless_time",
            "the aquifer time needs finite nonnegative permeability and time, porosity in (0,1], "
            "and positive viscosity, compressibility (1/Pa) and inner radius",
        )
    if permeability == 0.0 or time == 0.0:
        return 0.0
    numerator = permeability * time
    first = porosity * viscosity
    second = first * total_compressibility
    third = second * reservoir_radius
    denominator = third * reservoir_radius
    result = numerator / denominator
    if (
        _normal_positive(numerator)
        and _normal_positive(first)
        and _normal_positive(second)
        and _normal_positive(third)
        and _normal_positive(denominator)
        and result > 2.0 * _MIN_NORMAL
        and result < 0.5 * _MAX
    ):
        return result
    return _scaled_physical_ratio(
        [permeability, time],
        [porosity, viscosity, total_compressibility, reservoir_radius, reservoir_radius],
    )


def aquifer_constant(
    porosity: float,
    total_compressibility: float,
    thickness: float,
    reservoir_radius: float,
    encroachment_angle: float,
) -> float:
    """The aquifer constant ``U = 2 pi f phi c_t h r_i^2``, in reservoir m3 per Pa."""
    if (
        not math.isfinite(porosity)
        or porosity < 0.0
        or porosity > 1.0
        or not math.isfinite(total_compressibility)
        or total_compressibility < 0.0
        or not math.isfinite(thickness)
        or thickness < 0.0
        or not math.isfinite(reservoir_radius)
        or reservoir_radius <= 0.0
        or not math.isfinite(encroachment_angle)
        or encroachment_angle < 0.0
        or encroachment_angle > 360.0
    ):
        raise InvalidInputError(
            "aquifer_constant",
            "the aquifer storage needs finite porosity in [0,1], nonnegative compressibility "
            "(1/Pa) and thickness, positive inner radius and an angle in [0,360] degrees",
        )
    if (
        porosity == 0.0
        or total_compressibility == 0.0
        or thickness == 0.0
        or encroachment_angle == 0.0
    ):
        return 0.0
    fraction = encroachment_angle / 360.0
    first = 2.0 * _PI * fraction
    second = first * porosity
    third = second * total_compressibility
    fourth = third * thickness
    fifth = fourth * reservoir_radius
    result = fifth * reservoir_radius
    if (
        _normal_positive(fraction)
        and _normal_positive(first)
        and _normal_positive(second)
        and _normal_positive(third)
        and _normal_positive(fourth)
        and _normal_positive(fifth)
        and result > 2.0 * _MIN_NORMAL
        and result < 0.5 * _MAX
    ):
        return result
    return _scaled_physical_ratio(
        [
            2.0 * _PI,
            encroachment_angle,
            porosity,
            total_compressibility,
            thickness,
            reservoir_radius,
            reservoir_radius,
        ],
        [360.0],
    )


def _scaled_physical_ratio(numerator: list[float], denominator: list[float]) -> float:
    mantissa = 1.0
    exponent = 0
    for factor in numerator:
        power = _get_exponent(factor)
        mantissa *= math.ldexp(factor, -power)
        exponent += power
    for factor in denominator:
        power = _get_exponent(factor)
        mantissa /= math.ldexp(factor, -power)
        exponent -= power
    result = math.ldexp(mantissa, exponent)
    if result <= 2.0 * _MIN_NORMAL or result >= 0.5 * _MAX or not math.isfinite(result):
        raise InvalidInputError(
            "aquifer_conversion",
            "the positive conversion sits at a double precision boundary the class resolves "
            "with exact decimal arithmetic, which this port does not carry",
        )
    return result


def _scaled_product_ratio(
    first: float,
    second: float,
    third: float,
    pressure: float,
    normalized_denominator: float,
) -> float:
    if first == 0.0 or second == 0.0 or third == 0.0:
        return 0.0
    first_exponent = _get_exponent(first)
    second_exponent = _get_exponent(second)
    third_exponent = _get_exponent(third)
    pressure_exponent = _get_exponent(pressure)
    denominator_exponent = _get_exponent(normalized_denominator)
    mantissa = (
        math.ldexp(first, -first_exponent)
        * math.ldexp(second, -second_exponent)
        * math.ldexp(third, -third_exponent)
        / math.ldexp(pressure, -pressure_exponent)
        / math.ldexp(normalized_denominator, -denominator_exponent)
    )
    return math.ldexp(
        mantissa,
        first_exponent
        + second_exponent
        + third_exponent
        - pressure_exponent
        - denominator_exponent,
    )


def cumulative_influx_carter_tracy(
    dimensionless_times: list[float],
    pressure_drops: list[float],
    aquifer_constant: float,
    outer_radius: float | None = None,
) -> list[float]:
    """The Carter-Tracy cumulative water influx history, in m3."""
    if not dimensionless_times or len(dimensionless_times) != len(pressure_drops):
        raise InvalidInputError(
            "dimensionless_times",
            "the Carter-Tracy history needs nonempty times and drops of equal length",
        )
    if not math.isfinite(aquifer_constant) or aquifer_constant < 0.0:
        raise InvalidInputError(
            "aquifer_constant_value",
            "the aquifer constant must be finite and nonnegative",
        )
    if outer_radius is not None and math.isnan(outer_radius):
        raise InvalidInputError(
            "dimensionless_outer_radius",
            "the aquifer outer-radius argument must not be NaN",
        )
    for index, (time, drop) in enumerate(
        zip(dimensionless_times, pressure_drops, strict=True)
    ):
        if (
            not math.isfinite(time)
            or time < 0.0
            or (index > 0 and time <= dimensionless_times[index - 1])
            or not math.isfinite(drop)
        ):
            raise InvalidInputError(
                f"dimensionless_times[{index}]",
                "times must be finite, nonnegative and strictly increasing, and drops finite",
            )
    influx = [0.0] * len(dimensionless_times)
    if aquifer_constant == 0.0:
        return influx
    for index in range(1, len(dimensionless_times)):
        pd, pd_prime = _influence(dimensionless_times[index], outer_radius)
        if not math.isfinite(pd) or pd <= 0.0 or not math.isfinite(pd_prime):
            raise InvalidInputError(
                f"dimensionless_times[{index}]",
                "the Carter-Tracy influence is not finite and positive at this index",
            )
        elasticity = dimensionless_times[index] * pd_prime / pd
        denominator = 1.0 - (dimensionless_times[index - 1] / dimensionless_times[index]) * elasticity
        retained = (1.0 - elasticity) / denominator
        if denominator == 0.0 or not math.isfinite(retained):
            raise InvalidInputError(
                f"dimensionless_times[{index}]",
                "the Carter-Tracy transition is singular or unrepresentable at this index",
            )
        next_influx = retained * influx[index - 1] + _scaled_product_ratio(
            dimensionless_times[index] - dimensionless_times[index - 1],
            aquifer_constant,
            pressure_drops[index],
            pd,
            denominator,
        )
        influx[index] = 0.0 if next_influx < 0.0 else next_influx
        if not math.isfinite(influx[index]):
            raise InvalidInputError(
                f"dimensionless_times[{index}]",
                "the Carter-Tracy cumulative influx is unrepresentable at this index",
            )
    return influx


def influence_table(
    dimensionless_times: list[float], outer_radius: float | None = None
) -> list[float]:
    """The dimensionless pressures of an influence table."""
    return [dimensionless_pressure(time, outer_radius) for time in dimensionless_times]


def _required(name: str, value: float | None) -> float:
    if value is None:
        raise InvalidInputError(name, "the calculation needs this field")
    return float(value)


def _required_si(spec: dict[str, Any], name: str, value: Any) -> float:
    if value is None:
        raise InvalidInputError(name, "the calculation needs this field")
    return input_to_si(spec, name, value)


def _row(**values: Any) -> VanEverdingenHurstAquiferResult:
    return VanEverdingenHurstAquiferResult(
        dimensionless_pressure=values.get("dimensionless_pressure"),
        dimensionless_pressure_derivative=values.get("dimensionless_pressure_derivative"),
        dimensionless_time=values.get("dimensionless_time"),
        aquifer_constant=values.get("aquifer_constant"),
        dimensionless_pressures=values.get("dimensionless_pressures"),
        cumulative_influx=values.get("cumulative_influx"),
        warnings=tuple(values.get("warnings", ())),
    )


def van_everdingen_hurst_aquifer(
    calculation: str,
    *,
    dimensionless_time_value: float | None = None,
    dimensionless_times: Any = None,
    dimensionless_outer_radius: float | None = None,
    permeability: Any = None,
    time: Any = None,
    porosity: float | None = None,
    viscosity: Any = None,
    total_compressibility: float | None = None,
    reservoir_radius: Any = None,
    thickness: Any = None,
    encroachment_angle: float | None = None,
    aquifer_constant_value: float | None = None,
    pressure_drop: Any = None,
) -> VanEverdingenHurstAquiferResult:
    """The aquifer calculation over the stated inputs.

    Returns:
        The published fields; the others are absent.

    Raises:
        InvalidInputError: for an unknown calculation, a missing input, a physical input
            outside its domain, a refused Carter-Tracy history or an unrepresentable answer.
    """
    spec = _spec()
    if calculation == "dimensionless_pressure":
        td = _required("dimensionless_time_value", dimensionless_time_value)
        return _row(
            dimensionless_pressure=dimensionless_pressure(td, dimensionless_outer_radius)
        )
    if calculation == "dimensionless_pressure_derivative":
        td = _required("dimensionless_time_value", dimensionless_time_value)
        return _row(
            dimensionless_pressure_derivative=dimensionless_pressure_derivative(
                td, dimensionless_outer_radius
            )
        )
    if calculation == "dimensionless_time":
        return _row(
            dimensionless_time=dimensionless_time(
                _required_si(spec, "permeability", permeability),
                _required_si(spec, "time", time),
                _required("porosity", porosity),
                _required_si(spec, "viscosity", viscosity),
                _required("total_compressibility", total_compressibility),
                _required_si(spec, "reservoir_radius", reservoir_radius),
            )
        )
    if calculation == "aquifer_constant":
        return _row(
            aquifer_constant=aquifer_constant(
                _required("porosity", porosity),
                _required("total_compressibility", total_compressibility),
                _required_si(spec, "thickness", thickness),
                _required_si(spec, "reservoir_radius", reservoir_radius),
                _required("encroachment_angle", encroachment_angle),
            )
        )
    if dimensionless_times is not None:
        times = [float(value) for value in dimensionless_times]
    if calculation == "carter_tracy":
        if dimensionless_times is None:
            raise InvalidInputError(
                "dimensionless_times",
                "the Carter-Tracy recurrence needs the dimensionless time history",
            )
        if pressure_drop is None:
            raise InvalidInputError(
                "pressure_drop",
                "the Carter-Tracy recurrence needs the pressure-drop history",
            )
        drops = [input_to_si(spec, "pressure_drop", value) for value in pressure_drop]
        influx = cumulative_influx_carter_tracy(
            times,
            drops,
            _required("aquifer_constant_value", aquifer_constant_value),
            dimensionless_outer_radius,
        )
        return _row(cumulative_influx=[quantity(value, "m**3") for value in influx])
    if calculation == "influence_table":
        if dimensionless_times is None:
            raise InvalidInputError(
                "dimensionless_times",
                "the influence table needs the dimensionless time history",
            )
        return _row(dimensionless_pressures=influence_table(times, dimensionless_outer_radius))
    raise InvalidInputError(
        "calculation", f"{calculation!r} is not one of the aquifer calculations"
    )
