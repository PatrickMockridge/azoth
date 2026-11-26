"""``pvt.gas_density_consistency`` - the gas-density identity over observed intervals.

The Python twin of ``crates/azoth-pvt/src/gas_density_consistency.rs``: ``rho = p M / (Z R T)``
with outward-rounded closed intervals and the class's exact gas constant.
"""

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import GasDensityConsistencyResult, GasDensityRow
from azoth.core.units import quantity, to_si

MODEL_ID = "pvt.gas_density_consistency"

GAS_CONSTANT = 8.31446261815324
_MIN_SUBNORMAL = 5.0e-324


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _field(name: str, value: Any) -> float:
    declaration = _spec()["inputs"]["observations"]["fields"][name]
    if declaration["unit"] == "dimensionless":
        return float(value)
    return float(to_si(value, declaration["unit"], f"observations.{name}"))


class _Interval:
    """A closed positive interval with the class's outward rounding."""

    def __init__(self, value: float, bound: float, field: str) -> None:
        if (
            not math.isfinite(value)
            or not math.isfinite(bound)
            or bound < 0.0
            or value - bound <= 0.0
            or not math.isfinite(value + bound)
            or (bound > 0.0 and not math.isfinite(math.nextafter(value + bound, math.inf)))
        ):
            raise InvalidInputError(
                field,
                "a quantity needs finite positive endpoints and a nonnegative bound",
            )
        self.value = value
        self.bound = bound

    def lower(self) -> float:
        if self.bound == 0.0:
            return self.value
        return max(math.nextafter(self.value - self.bound, -math.inf), _MIN_SUBNORMAL)

    def upper(self) -> float:
        if self.bound == 0.0:
            return self.value
        return math.nextafter(self.value + self.bound, math.inf)


def _round(value: float, upper: bool) -> float:
    return math.nextafter(value, math.inf if upper else -math.inf)


def _endpoint(
    pressure: _Interval,
    temperature: _Interval,
    compressibility: _Interval,
    molar_mass: _Interval,
    upper: bool,
) -> float:
    p = pressure.upper() if upper else pressure.lower()
    t = temperature.lower() if upper else temperature.upper()
    z = compressibility.lower() if upper else compressibility.upper()
    m = molar_mass.upper() if upper else molar_mass.lower()
    value = _round(p * m, upper)
    value = _round(value / z, upper)
    value = _round(value / GAS_CONSTANT, upper)
    return _round(value / t, upper)


def gas_density_consistency(
    observations: Sequence[Mapping[str, Any]] | None = None,
) -> GasDensityConsistencyResult:
    """Propagate each observation's gas-density interval and test the observed one against it.

    ``observations`` is one record per gas-phase state: the pressure, temperature,
    compressibility, molar mass and observed density, each with its nonnegative absolute
    bound. The implied density is ``p M / (Z R T)`` with ``R = 8.31446261815324``, and each
    arithmetic step rounds its endpoints outward, so the interval encloses every corner of
    the inputs. A row is compatible iff the observed interval overlaps the implied one,
    endpoints included.

    Returns:
        One ``rows`` record per observation with the implied nominal, the two bounds, the
        observed-minus-implied difference and the row's verdict, and whether every row is
        compatible.

    Raises:
        InvalidInputError: for no observations, a non-positive or non-finite nominal value,
            a negative or unrepresentable bound, or an unrepresentable implied interval.

    See :func:`azoth.pvt.reference.gas_density_consistency.gas_density_consistency`.
    """
    rows = list(observations or ())
    if not rows:
        raise InvalidInputError("observations", "at least one observation is required")
    out: list[GasDensityRow] = []
    for row in rows:
        pressure = _Interval(
            _field("pressure", row["pressure"]),
            _field("pressure_bound", row["pressure_bound"]),
            "pressure",
        )
        temperature = _Interval(
            _field("temperature", row["temperature"]),
            _field("temperature_bound", row["temperature_bound"]),
            "temperature",
        )
        compressibility = _Interval(
            _field("compressibility", row["compressibility"]),
            _field("compressibility_bound", row["compressibility_bound"]),
            "compressibility",
        )
        molar_mass = _Interval(
            _field("molar_mass", row["molar_mass"]),
            _field("molar_mass_bound", row["molar_mass_bound"]),
            "molar_mass",
        )
        density = _Interval(
            _field("density", row["density"]),
            _field("density_bound", row["density_bound"]),
            "density",
        )
        implied_density = (
            pressure.value
            * molar_mass.value
            / compressibility.value
            / GAS_CONSTANT
            / temperature.value
        )
        implied_lower = _endpoint(pressure, temperature, compressibility, molar_mass, False)
        implied_upper = _endpoint(pressure, temperature, compressibility, molar_mass, True)
        if (
            not math.isfinite(implied_density)
            or implied_density <= 0.0
            or implied_lower <= 0.0
            or not math.isfinite(implied_upper)
        ):
            raise InvalidInputError(
                "density",
                "the gas-density interval cannot be represented as positive finite doubles",
            )
        out.append(
            GasDensityRow(
                implied_density=quantity(implied_density, "kg/m**3"),
                implied_lower=quantity(implied_lower, "kg/m**3"),
                implied_upper=quantity(implied_upper, "kg/m**3"),
                observed_minus_implied=quantity(density.value - implied_density, "kg/m**3"),
                compatible=density.lower() <= implied_upper and density.upper() >= implied_lower,
            )
        )
    return GasDensityConsistencyResult(
        rows=tuple(out),
        all_compatible=all(row.compatible for row in out),
        warnings=(),
    )
