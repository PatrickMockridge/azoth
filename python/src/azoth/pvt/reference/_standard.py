"""The standard state and the conversion literals the PVT package states its inputs against.

The Python twin of ``crates/azoth-pvt/src/standard.rs``. The literals are the classes' own:
``BlackOilUnits``' ``14.5038`` psia per bar and ``0.178108`` Sm3/Sm3 per scf/STB, and
``SaturationPressureCorrelation``'s ``0.0689476`` bar per psia and ``0.17810760667903``
Sm3/Sm3 per scf/STB. They are not exact SI definitions and are carried as stated.
"""

from __future__ import annotations

STANDARD_TEMPERATURE_K = 288.15
STANDARD_PRESSURE_BAR = 1.01325
STANDARD_PRESSURE_PA = 101_325.0

NEQSIM_REFERENCE_TEMPERATURE_K = 273.15
NEQSIM_REFERENCE_PRESSURE_BAR = 1.01325
NEQSIM_GAS_CONSTANT = 8.314_462_1

PSIA_PER_BAR = 14.5038
SM3_PER_SCF_STB = 0.178_108
SATURATION_BAR_PER_PSIA = 0.068_947_6
SATURATION_SM3_PER_SCF_STB = 0.178_107_606_679_03

KG_PER_M3_PER_LB_FT3 = 16.018_46
CENTIPOISE_PER_PA_S = 1000.0
RANKINE_OFFSET = 459.67
CELSIUS_OFFSET_K = 273.15

SATURATION_PRESSURE_MIN_BAR = 1.0
SATURATION_PRESSURE_MAX_BAR = 1000.0
SATURATION_PRESSURE_STEP_BAR = 10.0
SATURATION_PRESSURE_TOLERANCE_BAR = 1.0e-5
SATURATION_BISECTION_LIMIT = 500
SATURATION_TEMPERATURE_MIN_K = 30.0
SATURATION_TEMPERATURE_MAX_K = 1200.0
SATURATION_TEMPERATURE_STEP_K = 10.0
SATURATION_TEMPERATURE_TOLERANCE_K = 1.0e-5

NEQSIM_AIR_MOLAR_MASS_KG_PER_MOL = 0.028_965_46
CCE_BG_INTERNAL_FACTOR = 1.0e5
CVD_CELL_VOLUME_RELATIVE_TOLERANCE = 1.0e-8
CVD_MAX_CELL_VOLUME_ITERATIONS = 100
CVD_MAX_GAS_REMOVAL_FRACTION = 0.95
SEPARATOR_API_WATER_DENSITY = 999.0
SEPARATOR_API_NUMERATOR = 141.5
SEPARATOR_API_OFFSET = 131.5
SEPARATOR_API_DENSITY_MIN = 500.0
SEPARATOR_API_DENSITY_MAX = 1100.0
WAXY_OIL_A1 = 37.82
WAXY_OIL_A2 = 83.96
WAXY_OIL_A3 = 8.559e6
VISCOSITY_FLOOR_PA_S = 1.0e-5
MMP_DEFAULT_THRESHOLD = 0.90
MMP_DEFAULT_PRESSURE_POINTS = 10
MMP_DEFAULT_MIN_PRESSURE_BAR = 100.0
MMP_DEFAULT_MAX_PRESSURE_BAR = 500.0
MMP_DEFAULT_NODES = 100
MMP_DEFAULT_PV_INJECTED = 1.2
MMP_KEY_TIE_LINE_PRESSURE_TOLERANCE_BAR = 0.5
MMP_KEY_TIE_LINE_LENGTH_TOLERANCE = 0.01
SWELLING_DEFAULT_TEMPERATURE_K = 298.15


def bar_to_pa(bar: float) -> float:
    return bar * 1.0e5


def pa_to_bar(pa: float) -> float:
    return pa / 1.0e5


def bar_to_psia(bar: float) -> float:
    return bar * PSIA_PER_BAR


def psia_to_bar(psia: float) -> float:
    return psia / PSIA_PER_BAR


def pa_to_psia(pa: float) -> float:
    return bar_to_psia(pa_to_bar(pa))


def psia_to_pa(psia: float) -> float:
    return bar_to_pa(psia_to_bar(psia))


def kelvin_to_celsius(kelvin: float) -> float:
    return kelvin - CELSIUS_OFFSET_K


def celsius_to_kelvin(celsius: float) -> float:
    return celsius + CELSIUS_OFFSET_K


def celsius_to_fahrenheit(celsius: float) -> float:
    return celsius * 9.0 / 5.0 + 32.0


def kelvin_to_fahrenheit(kelvin: float) -> float:
    return celsius_to_fahrenheit(kelvin_to_celsius(kelvin))


def fahrenheit_to_celsius(fahrenheit: float) -> float:
    return (fahrenheit - 32.0) * 5.0 / 9.0


def fahrenheit_to_kelvin(fahrenheit: float) -> float:
    return celsius_to_kelvin(fahrenheit_to_celsius(fahrenheit))


def celsius_to_rankine(celsius: float) -> float:
    return celsius_to_fahrenheit(celsius) + RANKINE_OFFSET


def kelvin_to_rankine(kelvin: float) -> float:
    return kelvin_to_fahrenheit(kelvin) + RANKINE_OFFSET


def scf_stb_to_sm3_sm3(scf_per_stb: float) -> float:
    return scf_per_stb * SM3_PER_SCF_STB


def sm3_sm3_to_scf_stb(sm3_per_sm3: float) -> float:
    return sm3_per_sm3 / SM3_PER_SCF_STB


def saturation_psia_to_bar(psia: float) -> float:
    return psia * SATURATION_BAR_PER_PSIA


def saturation_bar_to_psia(bar: float) -> float:
    return bar / SATURATION_BAR_PER_PSIA


def saturation_scf_stb_to_sm3_sm3(scf_per_stb: float) -> float:
    return scf_per_stb * SATURATION_SM3_PER_SCF_STB


def saturation_sm3_sm3_to_scf_stb(sm3_per_sm3: float) -> float:
    return sm3_per_sm3 / SATURATION_SM3_PER_SCF_STB


def pa_s_to_centipoise(pa_s: float) -> float:
    return pa_s * CENTIPOISE_PER_PA_S


def centipoise_to_pa_s(centipoise: float) -> float:
    return centipoise / CENTIPOISE_PER_PA_S


def kg_m3_to_lb_ft3(kg_per_m3: float) -> float:
    return kg_per_m3 / KG_PER_M3_PER_LB_FT3


def lb_ft3_to_kg_m3(lb_per_ft3: float) -> float:
    return lb_per_ft3 * KG_PER_M3_PER_LB_FT3


def per_bar_to_per_psi(per_bar: float) -> float:
    return per_bar / PSIA_PER_BAR


def per_psi_to_per_bar(per_psi: float) -> float:
    return per_psi * PSIA_PER_BAR
