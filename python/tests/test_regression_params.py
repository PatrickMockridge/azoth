"""The regression's row transforms, held to ``pvt_regression.tsv``'s base and observed states."""

from __future__ import annotations

import math

import pytest

from azoth.core.errors import InvalidInputError
from azoth.pvt.reference.constant_mass_expansion import _constant_mass_expansion_of
from azoth.pvt.reference.differential_liberation import _differential_liberation_of
from azoth.pvt.reference.regression import build_fluid

C15 = {
    "fluid": 0.0,
    "name": "C15_PC",
    "pseudo_component": True,
    "moles": 0.1,
    "molar_mass": 0.150,
    "density": 820.0,
    "normal_boiling_point": 459.785,
    "critical_temperature": 617.630088327571,
    "critical_pressure": 2317913.3235636964,
    "acentric_factor": 0.6207158082397568,
    "critical_volume": 0.0005031685170968987,
    "racket_z": 0.26187509006247484,
    "racket_z_cpa": 0.26187509006247484,
    "parachor": 410.3,
    "critical_viscosity": 2.7118350358337838e-05,
    "triple_point_temperature": 243.9455,
    "heat_of_fusion": 21846.780317779318,
    "ideal_gas_enthalpy_of_formation": -266956.0,
    "cp_a": -2.8331262101030834,
    "cp_b": 0.8287561079860862,
    "cp_c": -0.00033668595820249037,
    "cp_d": 0.0,
    "attractive_m": 1.389195974,
}


def _base(name: str, moles: float) -> dict[str, object]:
    return {"fluid": 0.0, "name": name, "pseudo_component": False, "moles": moles}


def _close(actual: float, expected: float, relative: float, label: str) -> None:
    difference = abs(actual - expected) / max(abs(expected), 1e-300)
    assert difference <= relative, f"{label}: got {actual}, expected {expected}"


def test_the_cce_base_state_is_the_capture() -> None:
    rows = [_base("methane", 0.6), _base("nC10", 0.3), C15]
    parameters = [
        {
            "parameter": "tc_multiplier_c7plus",
            "initial": 1.0,
            "lower": 0.95,
            "upper": 1.05,
        }
    ]
    fluid, _ = build_fluid(rows, parameters=parameters, values=[1.0])
    cce = _constant_mass_expansion_of(fluid, 373.15, [2.5e7, 2.0e7, 1.5e7])
    expected = [0.995484371961769, 1.1090595519559416, 1.3360225249122977]
    for index, point in enumerate(cce.points):
        _close(float(point.relative_volume), expected[index], 1e-5, f"base[{index}]")


def test_the_tc_multiplier_reproduces_the_capture_observation() -> None:
    rows = [_base("methane", 0.6), _base("nC10", 0.3), C15]
    parameters = [
        {
            "parameter": "tc_multiplier_c7plus",
            "initial": 1.0,
            "lower": 0.95,
            "upper": 1.05,
        }
    ]
    fluid, _ = build_fluid(rows, parameters=parameters, values=[1.03])
    cce = _constant_mass_expansion_of(fluid, 373.15, [2.5e7, 2.0e7, 1.5e7])
    expected = [0.9977467494220793, 1.1177723544056017, 1.344662536647291]
    for index, point in enumerate(cce.points):
        _close(float(point.relative_volume), expected[index], 1e-5, f"tuned[{index}]")


def test_the_co2_bip_reproduces_the_capture_observation() -> None:
    rows = [_base("methane", 0.45), _base("CO2", 0.05), _base("nC10", 0.50)]
    parameters = [{"parameter": "bip_co2_hc", "initial": 0.12, "lower": 0.08, "upper": 0.18}]
    fluid, _ = build_fluid(rows, parameters=parameters, values=[0.15])
    dle = _differential_liberation_of(fluid, 323.15, [2.5e7, 1.5e7, 8.0e6])
    rs = [124.46822441360351, 118.19000000135856, 54.98240543625079]
    bo = [1.297774285932201, 1.3196221126198664, 1.1668447141587335]
    rho = [657.1251440903429, 642.5894780343832, 684.8946122783024]
    for index, point in enumerate(dle.points):
        _close(float(point.rs or 0.0), rs[index], 1e-6, f"Rs[{index}]")
        _close(float(point.bo or 1.0), bo[index], 1e-6, f"Bo[{index}]")
        density = float(point.oil_density.to("kg/m**3").magnitude)
        _close(density, rho[index], 1e-6, f"density[{index}]")


def test_the_three_multipliers_move_only_the_pseudo_rows() -> None:
    rows = [_base("methane", 0.6), _base("nC10", 0.3), C15]
    parameters = [
        {"parameter": "tc_multiplier_c7plus", "initial": 1.0, "lower": 0.95, "upper": 1.05},
        {"parameter": "pc_multiplier_c7plus", "initial": 1.0, "lower": 0.95, "upper": 1.05},
        {"parameter": "omega_multiplier_c7plus", "initial": 1.0, "lower": 0.9, "upper": 1.1},
    ]
    fluid, _ = build_fluid(rows, parameters=parameters, values=[1.02, 0.98, 1.05])
    components = fluid.mixture.components
    assert components[0].Tc.to_base_units().magnitude == pytest.approx(190.56, rel=1e-12)
    assert components[2].Tc.to_base_units().magnitude == pytest.approx(
        617.630088327571 * 1.02, rel=1e-12
    )
    assert components[2].Pc.to_base_units().magnitude == pytest.approx(
        2317913.3235636964 * 0.98, rel=1e-12
    )
    assert components[2].omega == pytest.approx(0.6207158082397568 * 1.05, rel=1e-12)


def test_the_volume_shift_scales_the_peneloux_constant() -> None:
    rows = [_base("methane", 0.6), _base("nC10", 0.3), C15]
    parameters = [{"parameter": "volume_shift_c7plus", "initial": 1.0, "lower": 0.8, "upper": 1.2}]
    base_fluid, _ = build_fluid(rows, parameters=parameters, values=[1.0])
    scaled, _ = build_fluid(rows, parameters=parameters, values=[1.2])
    base_shift = base_fluid.mixture.components[2].volume_shift_srk
    scaled_shift = scaled.mixture.components[2].volume_shift_srk
    assert scaled_shift / base_shift == pytest.approx(1.2, rel=1e-12)


def test_the_bips_set_the_named_pairs() -> None:
    rows = [
        _base("nitrogen", 0.01),
        _base("methane", 0.59),
        _base("ethane", 0.1),
        _base("nC10", 0.2),
        C15,
    ]
    parameters = [
        {"parameter": "bip_methane_c7plus", "initial": 0.0, "lower": 0.0, "upper": 0.1},
        {"parameter": "bip_c2c6_c7plus", "initial": 0.0, "lower": 0.0, "upper": 0.05},
        {"parameter": "bip_n2_hc", "initial": 0.0, "lower": 0.02, "upper": 0.12},
    ]
    fluid, _ = build_fluid(rows, parameters=parameters, values=[0.05, 0.02, 0.07])
    kij = fluid.mixture.kij
    assert kij[1][4] == pytest.approx(0.05, rel=1e-12)
    assert kij[2][4] == pytest.approx(0.02, rel=1e-12)
    assert kij[0][1] == pytest.approx(0.07, rel=1e-12)
    assert kij[0][4] == pytest.approx(0.07, rel=1e-12)


def test_the_csp_factors_are_carried_beside_the_fluid() -> None:
    rows = [_base("methane", 1.0)]
    parameters = [{"parameter": "viscosity_csp_3", "initial": 1.0, "lower": 0.5, "upper": 2.0}]
    _, csp = build_fluid(rows, parameters=parameters, values=[1.35])
    assert csp == (1.0, 1.0, 1.35, 1.0)


def test_a_non_finite_value_is_refused() -> None:
    rows = [_base("methane", 1.0)]
    parameters = [
        {
            "parameter": "tc_multiplier_c7plus",
            "initial": 1.0,
            "lower": 0.95,
            "upper": 1.05,
        }
    ]
    with pytest.raises(InvalidInputError, match="not finite"):
        build_fluid(rows, parameters=parameters, values=[math.nan])
