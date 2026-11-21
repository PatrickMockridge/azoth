"""A fluid the laboratory experiments flash.

The Python twin of ``crates/azoth-pvt/src/fluid.rs``. The experiments are one composition
ledger: every step is a fresh :class:`Fluid`, and the classes' mutation of a shared system is
not carried.
"""

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from typing import Any

import pint

from azoth.core.errors import InvalidInputError

from . import _flash

#: Every field of the characterization row, in the order the spec declares them.
FIELDS = (
    "fluid",
    "name",
    "pseudo_component",
    "moles",
    "molar_mass",
    "density",
    "normal_boiling_point",
    "critical_temperature",
    "critical_pressure",
    "acentric_factor",
    "critical_volume",
    "racket_z",
    "racket_z_cpa",
    "parachor",
    "critical_viscosity",
    "triple_point_temperature",
    "heat_of_fusion",
    "ideal_gas_enthalpy_of_formation",
    "cp_a",
    "cp_b",
    "cp_c",
    "cp_d",
    "attractive_m",
)


def _field(row: Any, name: str) -> Any:
    """One field of a row, from a mapping or from an object with the same attributes."""
    if isinstance(row, Mapping):
        return row.get(name)
    return getattr(row, name, None)


def _scalar(value: Any) -> float:
    """One row field as an SI magnitude; a quantity crosses through its base units."""
    if isinstance(value, pint.Quantity):
        return float(value.to_base_units().magnitude)
    return float(value)


def _rows_as_mappings(rows: Sequence[Any]) -> list[dict[str, Any]]:
    """The rows as mappings, so a union can be assembled without knowing their source type."""
    converted: list[dict[str, Any]] = []
    for row in rows:
        if isinstance(row, Mapping):
            converted.append(dict(row))
            continue
        values = {name: _field(row, name) for name in FIELDS if _field(row, name) is not None}
        converted.append(values)
    return converted


@dataclass(frozen=True)
class GasTemplate:
    """The injection gas's composition in a union fluid's component order."""

    amounts: list[float]
    total: float


@dataclass
class Fluid:
    """One fluid, its amounts, and the two models a flash and an enthalpy take."""

    mixture: Any
    ideal_gas: Any
    eos: str
    moles: list[float]
    corrected: bool
    rows: list[dict[str, Any]] = field(default_factory=list, repr=False)

    @classmethod
    def from_rows(
        cls, rows: Sequence[Any], *, eos: str = "srk", kij: str = "database", corrected: bool = True
    ) -> Fluid:
        """A fluid from its component rows."""
        converted = _rows_as_mappings(rows)
        _check_rows(converted)
        from azoth.characterization import mixture_from_characterization

        mixture, ideal_gas = mixture_from_characterization(converted, eos=eos, kij=kij)
        return cls(
            mixture=mixture,
            ideal_gas=ideal_gas,
            eos=eos,
            moles=[_scalar(row["moles"]) for row in converted],
            corrected=corrected,
            rows=converted,
        )

    @classmethod
    def from_rows_pair(
        cls,
        oil: Sequence[Any],
        gas: Sequence[Any],
        *,
        eos: str = "srk",
        kij: str = "database",
        corrected: bool = True,
    ) -> tuple[Fluid, GasTemplate]:
        """An oil and an injection gas as one union fluid, with the gas's template."""
        oil_rows = _rows_as_mappings(oil)
        gas_rows = _rows_as_mappings(gas)
        _check_rows(oil_rows)
        _check_rows(gas_rows)
        union = [dict(row) for row in oil_rows]
        amounts = [0.0] * len(union)
        total = 0.0
        for row in gas_rows:
            name = row.get("name")
            index = next((at for at, held in enumerate(union) if held.get("name") == name), None)
            if index is None:
                appended = dict(row)
                appended["fluid"] = 0.0
                appended["moles"] = 0.0
                union.append(appended)
                amounts.append(0.0)
                index = len(union) - 1
            elif bool(union[index].get("pseudo_component")) != bool(row.get("pseudo_component")):
                raise InvalidInputError(
                    "injection_gas",
                    f"{name} is a base row in one fluid and a pseudo row in the other, so the "
                    "union has no one component for it",
                )
            amount = _scalar(row["moles"])
            if not math.isfinite(amount) or amount < 0.0:
                raise InvalidInputError("injection_gas", f"{name}'s amount is {amount} mol")
            amounts[index] += amount
            total += amount
        fluid = cls.from_rows(union, eos=eos, kij=kij, corrected=corrected)
        return fluid, GasTemplate(amounts=amounts, total=total)

    def total_moles(self) -> float:
        """The feed's total amount, in mol."""
        return math.fsum(self.moles)

    def z(self) -> list[float]:
        """The feed's mole fractions, normalised."""
        total = self.total_moles()
        return [moles / total for moles in self.moles]

    def mass(self) -> float:
        """The feed's mass, in kg."""
        mass = 0.0
        for index, component in enumerate(self.mixture.components):
            if component.molar_mass is None:
                raise InvalidInputError(
                    "components",
                    f"component {index} carries no molar mass, so the fluid has no mass",
                )
            mass += self.moles[index] * component.molar_mass.to("kg/mol").magnitude
        return mass

    def with_moles(self, moles: Sequence[float]) -> Fluid:
        """The same fluid at different amounts."""
        return Fluid(
            mixture=self.mixture,
            ideal_gas=self.ideal_gas,
            eos=self.eos,
            moles=list(moles),
            corrected=self.corrected,
            rows=self.rows,
        )

    def with_added(self, gas: GasTemplate, total: float) -> Fluid:
        """The fluid after adding ``total`` moles of a gas with the given template."""
        moles = list(self.moles)
        for index, amount in enumerate(gas.amounts):
            moles[index] += total * amount / gas.total
        return self.with_moles(moles)

    def flash(self, t: float, p: float, mode: str) -> _flash.Flash:
        """Flash the fluid at a temperature and a pressure, both SI."""
        if not (math.isfinite(t) and t > 0.0):
            raise InvalidInputError("temperature", f"a flash needs a positive temperature; got {t}")
        if not (math.isfinite(p) and p > 0.0):
            raise InvalidInputError("pressure", f"a flash needs a positive pressure; got {p}")
        return _flash.run(self, t, p, mode)


def _check_rows(rows: Sequence[dict[str, Any]]) -> None:
    if not rows:
        raise InvalidInputError("components", "a fluid needs at least one component row")
    fluid = float(rows[0].get("fluid", 0.0))
    if any(float(row.get("fluid", 0.0)) != fluid for row in rows):
        raise InvalidInputError(
            "components",
            "the rows belong to more than one fluid, and one id describes one fluid",
        )
    for index, row in enumerate(rows):
        if row.get("name") is None:
            raise InvalidInputError(f"components[{index}].name", "the row carries no name")
        moles = row.get("moles")
        if moles is None:
            raise InvalidInputError(f"components[{index}].moles", "the row carries no amount")
        amount = _scalar(moles)
        if not math.isfinite(amount) or amount < 0.0:
            raise InvalidInputError(
                f"components[{index}].moles", f"a component's amount is {amount} mol"
            )
    if math.fsum(_scalar(row["moles"]) for row in rows) <= 0.0:
        raise InvalidInputError("components", "the fluid's components sum to no amount")
