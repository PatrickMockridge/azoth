"""The PVT standard-state module, held to the two classes' literals."""

from __future__ import annotations

from azoth.pvt.reference import _standard as s


def test_the_black_oil_literals_are_the_class_ones() -> None:
    assert s.bar_to_psia(1.0) == 14.5038
    assert s.scf_stb_to_sm3_sm3(1.0) == 0.178_108
    assert s.kg_m3_to_lb_ft3(16.018_46) == 1.0
    assert s.pa_s_to_centipoise(0.001) == 1.0
    assert s.celsius_to_rankine(0.0) == 491.67


def test_the_saturation_literals_are_the_other_class_ones() -> None:
    assert s.saturation_psia_to_bar(1.0) == 0.068_947_6
    assert s.saturation_bar_to_psia(1.0) == 1.0 / 0.068_947_6
    assert s.saturation_scf_stb_to_sm3_sm3(1.0) == 0.178_107_606_679_03


def test_the_two_families_are_not_the_same_number() -> None:
    assert s.SM3_PER_SCF_STB != s.SATURATION_SM3_PER_SCF_STB
    assert s.psia_to_bar(1.0) != s.saturation_psia_to_bar(1.0)


def test_the_standard_state_is_the_class_standard_state() -> None:
    assert s.STANDARD_TEMPERATURE_K == 288.15
    assert s.STANDARD_PRESSURE_BAR == 1.01325
    assert s.STANDARD_PRESSURE_PA == 101_325.0


def test_temperature_conversions_are_the_class_arithmetic() -> None:
    assert s.kelvin_to_celsius(288.15) == 288.15 - 273.15
    assert s.celsius_to_kelvin(15.0) == 288.15
    assert s.celsius_to_fahrenheit(100.0) == 212.0
    assert s.fahrenheit_to_celsius(32.0) == 0.0
    assert s.kelvin_to_rankine(273.15) == 491.67


def test_the_laboratory_literals_are_the_class_ones() -> None:
    assert s.NEQSIM_AIR_MOLAR_MASS_KG_PER_MOL == 0.028
    assert s.CCE_BG_INTERNAL_FACTOR == 1.0e5
    assert s.CVD_CELL_VOLUME_RELATIVE_TOLERANCE == 1.0e-8
    assert s.CVD_MAX_CELL_VOLUME_ITERATIONS == 100
    assert s.CVD_MAX_GAS_REMOVAL_FRACTION == 0.95
    assert s.SEPARATOR_API_WATER_DENSITY == 999.0
    assert s.SEPARATOR_API_NUMERATOR == 141.5
    assert s.SEPARATOR_API_OFFSET == 131.5
    assert s.SEPARATOR_API_DENSITY_MIN == 500.0
    assert s.SEPARATOR_API_DENSITY_MAX == 1100.0
    assert s.WAXY_OIL_A1 == 37.82
    assert s.WAXY_OIL_A2 == 83.96
    assert s.WAXY_OIL_A3 == 8.559e6
    assert s.VISCOSITY_FLOOR_PA_S == 1.0e-5
    assert s.MMP_DEFAULT_THRESHOLD == 0.90
    assert s.MMP_DEFAULT_PRESSURE_POINTS == 10
    assert s.MMP_DEFAULT_MIN_PRESSURE_BAR == 100.0
    assert s.MMP_DEFAULT_MAX_PRESSURE_BAR == 500.0
    assert s.MMP_DEFAULT_NODES == 100
    assert s.MMP_DEFAULT_PV_INJECTED == 1.2
    assert s.MMP_KEY_TIE_LINE_PRESSURE_TOLERANCE_BAR == 0.5
    assert s.MMP_KEY_TIE_LINE_LENGTH_TOLERANCE == 0.01
    assert s.SWELLING_DEFAULT_TEMPERATURE_K == 298.15


def test_the_saturation_grids_are_the_class_grids() -> None:
    assert s.SATURATION_PRESSURE_MIN_BAR == 1.0
    assert s.SATURATION_PRESSURE_MAX_BAR == 1000.0
    assert s.SATURATION_PRESSURE_STEP_BAR == 10.0
    assert s.SATURATION_PRESSURE_TOLERANCE_BAR == 1.0e-5
    assert s.SATURATION_TEMPERATURE_MIN_K == 30.0
    assert s.SATURATION_TEMPERATURE_MAX_K == 1200.0
    assert s.SATURATION_TEMPERATURE_STEP_K == 10.0
    assert s.SATURATION_TEMPERATURE_TOLERANCE_K == 1.0e-5
    assert s.SATURATION_BISECTION_LIMIT == 500
