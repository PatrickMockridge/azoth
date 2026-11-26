"""A claim's stated unit, converted to the unit the spec declares.

The source writes what it writes: ``77.36 mol%``, ``50 barg``, ``81 degC``. Pint
reads neither ``mol%`` nor ``barg``, and reading a bare ``bar`` as absolute would
be an assumption the source did not make. This module normalises the spellings
that carry a definite meaning and refuses the ones that do not; every refusal is
a gap with a request, not a silent conversion.
"""

from __future__ import annotations

import math
from dataclasses import dataclass

import pint

from azoth.core.units import ureg


class UnitGap(Exception):
    """A unit that cannot be converted without inventing something."""

    def __init__(self, code: str, detail: str, request: str) -> None:
        super().__init__(detail)
        self.code = code
        self.detail = detail
        self.request = request


@dataclass(frozen=True, slots=True)
class Normalized:
    """A scalar in the spec's unit, with the conversion it took to get there."""

    value: float
    unit: str
    note: str | None = None
    basis: str | None = None


_MOLE_PERCENT = frozenset({"mol%", "mole%", "molpercent", "molepercent"})
_MASS_PERCENT = frozenset({"wt%", "mass%", "weight%", "masspercent", "weightpercent"})
_PERCENT = frozenset({"%", "percent"})
_DIMENSIONLESS = frozenset({"dimensionless", "-", "1", "fraction", "ratio"})

_GAUGE_BAR = frozenset({"barg", "bar(g)"})
_GAUGE_PSI = frozenset({"psig", "psi(g)"})
_ABS_BAR = frozenset({"bara", "bar(a)"})
_ABS_PSI = frozenset({"psia", "psi(a)"})
_BARE_PRESSURE = frozenset({"bar", "psi"})

_BAR_TO_PA = 100_000.0
_PSI_TO_PA = 6_894.757293168
_ATM_TO_PA = 101_325.0
_STANDARD_ATM_BAR = 1.01325
_STANDARD_ATM_PSI = 14.6959487755

_CELSIUS = frozenset({"degc", "°c", "celsius"})
_FAHRENHEIT = frozenset({"degf", "°f", "fahrenheit"})


def _key(unit: str) -> str:
    return unit.strip().lower().replace(" ", "")


def _in_spec_unit(value: float, unit: str, spec_unit: str, field: str) -> float:
    try:
        converted = ureg.Quantity(value, unit).to(spec_unit)
    except (pint.errors.PintError, ValueError, TypeError) as exc:
        raise UnitGap(
            "UNIT_DIMENSION_MISMATCH",
            f"{field}: {value} {unit} does not convert to the spec's {spec_unit} ({exc})",
            f"provide {field} in a unit of {spec_unit}",
        ) from exc
    return float(converted.magnitude)


def normalize(
    value: object,
    unit: str | None,
    spec_unit: str,
    field: str,
    *,
    expect_mole: bool = False,
    interval: bool = False,
) -> Normalized:
    """One numeric claim in the spec's unit.

    ``expect_mole`` says the input is a composition: a mass-percent or an
    unqualified percent claim is then refused rather than read as a mole
    fraction, because the two are not the same number.
    """
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise UnitGap(
            "SHAPE_MISMATCH",
            f"{field}: expected a number, got {type(value).__name__}",
            f"provide {field} as a number",
        )
    number = float(value)
    if not math.isfinite(number):
        raise UnitGap("SHAPE_MISMATCH", f"{field}: {number!r} is not finite", f"provide {field}")

    if unit is None:
        if spec_unit == "dimensionless":
            return Normalized(number, spec_unit)
        raise UnitGap(
            "UNIT_MISSING",
            f"{field}: the source states {number} without a unit",
            f"provide {field} with its unit, or cite where the source states it",
        )

    key = _key(unit)
    if key in _DIMENSIONLESS:
        return Normalized(_in_spec_unit(number, "dimensionless", spec_unit, field), spec_unit)

    if key in _MOLE_PERCENT:
        if spec_unit != "dimensionless":
            raise UnitGap(
                "UNIT_DIMENSION_MISMATCH",
                f"{field}: a mole percent is dimensionless, not {spec_unit}",
                f"provide {field} in {spec_unit}",
            )
        return Normalized(number / 100.0, spec_unit, "converted mol% to mole fraction", "mole")

    if key in _MASS_PERCENT:
        if expect_mole:
            raise UnitGap(
                "COMPOSITION_BASIS_MISMATCH",
                f"{field}: the source reports a mass percent; the input is a mole fraction",
                f"provide {field} as a mole fraction or mole percent",
            )
        if spec_unit != "dimensionless":
            raise UnitGap(
                "UNIT_DIMENSION_MISMATCH",
                f"{field}: a mass percent is dimensionless, not {spec_unit}",
                f"provide {field} in {spec_unit}",
            )
        return Normalized(number / 100.0, spec_unit, "converted mass% to a mass fraction", "mass")

    if key in _PERCENT:
        if expect_mole:
            raise UnitGap(
                "UNIT_AMBIGUOUS",
                f"{field}: a bare percent does not say mole or mass",
                f"state whether {field} is a mole percent or a mass percent",
            )
        if spec_unit != "dimensionless":
            raise UnitGap(
                "UNIT_DIMENSION_MISMATCH",
                f"{field}: a percent is dimensionless, not {spec_unit}",
                f"provide {field} in {spec_unit}",
            )
        return Normalized(number / 100.0, spec_unit, "converted % to a fraction", None)

    if key in _BARE_PRESSURE:
        raise UnitGap(
            "UNIT_AMBIGUOUS",
            f"{field}: `{unit}` does not say gauge or absolute",
            f"state whether {field} is gauge ({key}g) or absolute ({key}a)",
        )

    if key in _GAUGE_BAR | _GAUGE_PSI | _ABS_BAR | _ABS_PSI:
        if key in _GAUGE_BAR:
            pascals = (number + _STANDARD_ATM_BAR) * _BAR_TO_PA
            note = "converted barg to Pa at 1.01325 bar"
        elif key in _GAUGE_PSI:
            pascals = (number + _STANDARD_ATM_PSI) * _PSI_TO_PA
            note = "converted psig to Pa at 14.6959487755 psi"
        elif key in _ABS_BAR:
            pascals = number * _BAR_TO_PA
            note = "converted bara to Pa"
        else:
            pascals = number * _PSI_TO_PA
            note = "converted psia to Pa"
        return Normalized(_in_spec_unit(pascals, "Pa", spec_unit, field), spec_unit, note)

    if key in _CELSIUS:
        if interval:
            return Normalized(
                _in_spec_unit(number, "K", spec_unit, field),
                spec_unit,
                "converted a degC difference",
            )
        return Normalized(
            _in_spec_unit(number, "degC", spec_unit, field), spec_unit, "converted degC"
        )
    if key in _FAHRENHEIT:
        if interval:
            return Normalized(
                _in_spec_unit(number * 5.0 / 9.0, "K", spec_unit, field),
                spec_unit,
                "converted a degF difference",
            )
        return Normalized(
            _in_spec_unit(number, "degF", spec_unit, field), spec_unit, "converted degF"
        )

    return Normalized(_in_spec_unit(number, unit, spec_unit, field), spec_unit)
