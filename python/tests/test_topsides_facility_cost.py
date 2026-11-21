"""``process.topsides_facility_cost`` against its capture."""

from __future__ import annotations

from azoth.core.units import quantity
from azoth.process.reference.topsides_facility_cost import topsides_facility_cost


def close(actual: float, expected: float) -> bool:
    return abs(actual - expected) <= 1.0e-12 * max(abs(expected), 1.0)


def unit(name: str, kind: str, tmc: float, weight: float) -> dict[str, object]:
    return {
        "name": name,
        "equipment_type": kind,
        "total_module_cost": tmc,
        "weight": quantity(weight, "kg"),
    }


def test_the_fixed_platform_matches_the_capture() -> None:
    result = topsides_facility_cost(
        [
            unit("Feed", "Streams", 0.0, 0.0),
            unit("Inlet Separator", "Vessels", 14905.679819876523, 1312.8218157927067),
            unit(
                "Export Compressor",
                "Compressors",
                1997444.3385886613,
                695625.7181328537,
            ),
            unit("Aftercooler", "Heat Exchangers", 9387.9, 228.0),
            unit(
                "Export Control Valve",
                "Valves",
                272641.47134845454,
                274.7412026459561,
            ),
        ],
        facility_type="fixed_platform",
        project_context="greenfield",
        location_factor=1.35,
        piping_bulk_factor=0.25,
        electrical_instrumentation_factor=0.16,
        insulation_paint_fireproofing_factor=0.06,
        hvac_telecom_safety_factor=0.08,
        engineering_procurement_factor=0.14,
        construction_management_factor=0.06,
        project_contingency_factor=0.15,
        owner_cost_factor=0.10,
        include_utilities_and_offsites=True,
        estimate_class=3.0,
    )
    assert len(result.capital_costs) == 10
    assert close(result.capital_costs[0].cost, 3097412.17617194)
    assert close(result.capital_costs[1].cost, 8473911.565988204)
    assert close(result.capital_costs[7].cost, 557534.1917109492)
    assert len(result.capital_cost_summary) == 2
    assert close(result.capital_cost_summary[0].cost, 1.899475884566912e7)
    assert any(
        row.name == "module.separationAndTreatmentModules" for row in result.capital_cost_breakdown
    )
    compression = next(
        row
        for row in result.capital_cost_breakdown
        if row.name == "module.compressionAndRotatingEquipment"
    )
    assert close(compression.cost, 2696549.857094693)
    assert len(result.project_costs) == 4
    assert close(result.project_cost_summary[0].cost, 2.754240032622022e7)
    assert len(result.weight_basis) == 4
    assert close(result.weight_basis[3].weight_kg.to("kg").magnitude, 1569242.882590408)
    assert len(result.material_take_off) == 9
    assert result.material_take_off[0].has_weight
    assert result.material_take_off[7].category == "electrical"
    assert not result.material_take_off[7].has_weight
    assert close(result.material_take_off[8].cost_usd, 247792.97409375518)
    assert len(result.quality_flags) == 3
    assert result.quality_flags[0].message == "Facility type factor set: Fixed platform"


def test_a_class_four_basis_adds_the_not_detailed_flag() -> None:
    result = topsides_facility_cost([unit("Inlet Separator", "Vessels", 100_000.0, 5000.0)])
    assert len(result.quality_flags) == 4
    assert (
        result.quality_flags[3].message
        == "Estimate is not a detailed Class 1-3 vendor or firm-quantity estimate."
    )


def test_a_well_is_excluded_from_the_topsides_scope() -> None:
    result = topsides_facility_cost(
        [
            unit("Inlet Separator", "Vessels", 100_000.0, 5000.0),
            unit("Well1", "Wells", 80_000_000.0, 0.0),
        ]
    )
    assert close(result.capital_costs[0].cost, 100_000.0)
    assert close(result.capital_cost_summary[1].cost, 80_000_000.0)
    assert len(result.material_take_off) == 5
    assert any("Excluded non-topsides item" in flag.message for flag in result.quality_flags)
