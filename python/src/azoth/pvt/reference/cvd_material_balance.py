"""``pvt.cvd_material_balance`` - the constant-volume-depletion accounting.

The Python twin of ``crates/azoth-pvt/src/cvd_material_balance.rs``: the vapor inventory from
the pressure and Z ratios, the named produced/remaining/liquid component amounts, the four
closures and the liquid-state resolution. The class's cell temperature and its
sample/experiment/data-use references are read by no formula and are not inputs.
"""

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from typing import Any

from azoth.core.errors import InvalidInputError
from azoth.core.result import CvdMaterialBalanceResult, CvdStage
from azoth.core.units import to_si

MODEL_ID = "pvt.cvd_material_balance"

_COMPOSITION_TOLERANCE = 1.0e-10


def _spec() -> dict[str, Any]:
    from azoth._models_gen import model

    return model(MODEL_ID)


def _field(name: str, value: Any) -> float:
    declaration = _spec()["inputs"]["observations"]["fields"][name]
    if declaration["unit"] == "dimensionless":
        return float(value)
    return float(to_si(value, declaration["unit"], f"observations.{name}"))


def _row(row: Mapping[str, Any], index: int) -> tuple[float, float, float, float]:
    pressure = _field("pressure", row["pressure"])
    gas_z = _field("gas_z", row["gas_z"])
    liquid_volume_fraction = _field("liquid_volume_fraction", row["liquid_volume_fraction"])
    cumulative = _field("cumulative_produced", row["cumulative_produced"])
    if (
        not math.isfinite(pressure)
        or pressure <= 0.0
        or not math.isfinite(gas_z)
        or gas_z <= 0.0
        or not math.isfinite(liquid_volume_fraction)
        or not 0.0 <= liquid_volume_fraction < 1.0
        or not math.isfinite(cumulative)
        or not 0.0 <= cumulative < 1.0
    ):
        raise InvalidInputError(
            "observations",
            "each row needs a finite positive pressure and Z and volume/production fractions "
            f"in [0, 1); row {index} does not",
        )
    return pressure, gas_z, liquid_volume_fraction, cumulative


def cvd_material_balance(
    component_names: Sequence[str] | None = None,
    observations: Sequence[Mapping[str, Any]] | None = None,
    vapor_compositions: Sequence[Sequence[float]] | None = None,
    balance_tolerance: float | None = None,
    minimum_liquid_moles: float | None = None,
) -> CvdMaterialBalanceResult:
    """Reconstruct a constant-volume-depletion experiment.

    ``component_names`` is the identity order of every composition and inventory row.
    ``observations`` is the saturation reference followed by the depletion rows, at
    strictly decreasing pressures and nondecreasing cumulative production;
    ``vapor_compositions`` is one composition per observation, in that order. The vapor
    inventory is ``(p/p_ref)(Z_ref/Z)(1-V_l)``; the liquid is the remainder. A row's four
    closures decide its compatibility, and ``liquid_state`` separates an absent liquid from
    one too small to resolve and one whose accounting contradicts itself.

    Returns:
        The reconstructed stages, the produced/remaining/liquid component matrices, and
        whether every stage closed.

    Raises:
        InvalidInputError: for an empty or duplicate component set, fewer than two
            observations, a row outside its range, a composition that does not already sum
            to one, a first row that is not the all-vapor reference, pressures that do not
            strictly decrease, a cumulative production that decreases, invalid limits, or
            an unrepresentable vapor inventory.

    See :func:`azoth.pvt.reference.cvd_material_balance.cvd_material_balance`.
    """
    names = [str(name) for name in component_names or ()]
    rows = list(observations or ())
    compositions = [list(row) for row in vapor_compositions or ()]
    tolerance = float(balance_tolerance) if balance_tolerance is not None else 1.0e-8
    minimum = float(minimum_liquid_moles) if minimum_liquid_moles is not None else 1.0e-7

    if not names:
        raise InvalidInputError("component_names", "at least one component is required")
    if len(set(names)) != len(names):
        raise InvalidInputError("component_names", "the component identities are not unique")
    if len(rows) < 2:
        raise InvalidInputError(
            "observations", "a saturation reference and one depletion row are required"
        )
    if len(compositions) != len(rows):
        raise InvalidInputError(
            "vapor_compositions", "one composition row is required per observation"
        )
    if not math.isfinite(tolerance) or not 0.0 <= tolerance < 1.0:
        raise InvalidInputError("balance_tolerance", "the tolerance must be finite in [0, 1)")
    if not math.isfinite(minimum) or minimum <= tolerance or minimum > 1.0:
        raise InvalidInputError(
            "minimum_liquid_moles",
            "the resolution floor must be finite, above the tolerance and at most one",
        )

    components = len(names)
    parsed: list[tuple[float, float, float, float]] = []
    for index, row in enumerate(rows):
        parsed.append(_row(row, index))
        composition = compositions[index]
        if len(composition) != components:
            raise InvalidInputError(
                "vapor_compositions", "each row must carry one value per component"
            )
        total = 0.0
        for value in composition:
            if not math.isfinite(value) or not 0.0 <= value <= 1.0:
                raise InvalidInputError(
                    "vapor_compositions", "a composition value is outside [0, 1]"
                )
            total += value
        if abs(total - 1.0) > _COMPOSITION_TOLERANCE:
            raise InvalidInputError(
                "vapor_compositions",
                "a composition must already sum to one; it is never normalised",
            )

    reference_pressure, reference_z, reference_liquid, reference_cumulative = parsed[0]
    if reference_liquid != 0.0 or reference_cumulative != 0.0:
        raise InvalidInputError(
            "observations",
            "the first row must be the all-vapor saturation reference before withdrawal",
        )
    for index in range(1, len(parsed)):
        if parsed[index][0] >= parsed[index - 1][0]:
            raise InvalidInputError("observations", "pressures must strictly decrease")
        if parsed[index][3] < parsed[index - 1][3]:
            raise InvalidInputError("observations", "cumulative production must not decrease")

    produced = [0.0] * components
    stages: list[CvdStage] = []
    produced_rows: list[tuple[float, ...]] = []
    remaining_rows: list[tuple[float, ...]] = []
    liquid_rows: list[tuple[float, ...]] = []

    for index, (pressure, gas_z, liquid_volume, cumulative) in enumerate(parsed):
        withdrawal = 0.0 if index == 0 else cumulative - parsed[index - 1][3]
        vapor = (pressure / reference_pressure) * (reference_z / gas_z) * (1.0 - liquid_volume)
        total = 1.0 - cumulative
        liquid = total - vapor
        if not math.isfinite(vapor) or (vapor == 0.0 and liquid_volume < 1.0):
            raise InvalidInputError(
                "observations", "the vapor inventory overflows or underflows at an observation"
            )
        composition = compositions[index]
        remaining = [0.0] * components
        liquid_components = [0.0] * components
        compatible = vapor >= 0.0 and vapor <= total + tolerance and liquid >= -tolerance
        component_sum = 0.0
        for component in range(components):
            produced[component] += withdrawal * composition[component]
            remaining[component] = compositions[0][component] - produced[component]
            liquid_components[component] = remaining[component] - vapor * composition[component]
            component_sum += liquid_components[component]
            compatible = (
                compatible
                and remaining[component] >= -tolerance
                and liquid_components[component] >= -tolerance
            )
        closure = component_sum - liquid
        remaining_closure = sum(remaining) - total
        produced_closure = sum(produced) - cumulative
        vapor_closure = vapor * sum(composition) - vapor
        if not math.isfinite(closure) or not math.isfinite(vapor_closure):
            raise InvalidInputError(
                "observations", "the component accounting is unrepresentable at an observation"
            )
        compatible = (
            compatible
            and abs(closure) <= tolerance
            and abs(remaining_closure) <= tolerance
            and abs(produced_closure) <= tolerance
            and abs(vapor_closure) <= tolerance
        )

        if not compatible or (liquid_volume == 0.0 and liquid > tolerance):
            state = "inconsistent"
            compatible = False
        elif liquid_volume == 0.0 and abs(liquid) <= tolerance:
            absent = all(abs(amount) <= tolerance for amount in liquid_components)
            compatible = compatible and absent
            state = "absent" if compatible else "inconsistent"
        elif liquid <= minimum:
            state = "unresolved"
            compatible = False
        else:
            resolved = all(0.0 <= amount / liquid <= 1.0 for amount in liquid_components)
            compatible = compatible and resolved
            state = "resolved" if compatible else "inconsistent"

        stages.append(
            CvdStage(
                total_moles=total,
                vapor_moles=vapor,
                liquid_moles=liquid,
                stage_withdrawn_moles=withdrawal,
                component_closure=closure,
                remaining_closure=remaining_closure,
                produced_closure=produced_closure,
                vapor_closure=vapor_closure,
                liquid_state=state,
                compatible=compatible,
            )
        )
        produced_rows.append(tuple(produced))
        remaining_rows.append(tuple(remaining))
        liquid_rows.append(tuple(liquid_components))

    return CvdMaterialBalanceResult(
        stages=tuple(stages),
        produced_component_moles=tuple(produced_rows),
        remaining_component_moles=tuple(remaining_rows),
        liquid_component_moles=tuple(liquid_rows),
        compatible=all(stage.compatible for stage in stages),
        warnings=(),
    )
