"""The regression's two ledgers and its sample scales."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.pvt.reference.regression import (
    CceExperiment,
    DleExperiment,
    ExperimentWeights,
    RegressionData,
    ViscosityPoint,
    build_fluid,
    chi_square,
    objectives,
    samples,
)

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


def _tc_parameter() -> dict[str, object]:
    return {
        "parameter": "tc_multiplier_c7plus",
        "initial": 1.0,
        "lower": 0.95,
        "upper": 1.05,
    }


def test_the_sample_scales_are_the_class_weights() -> None:
    data = RegressionData(
        dle=(
            DleExperiment(
                temperature=323.15,
                pressures=(2.5e7,),
                rs=(100.0,),
                bo=(1.3,),
                oil_density=(0.05,),
            ),
        ),
        viscosity=(
            ViscosityPoint(temperature=320.0, pressure=5.0e5, viscosity=3.5e-4, phase="oil"),
        ),
    )
    sigmas = [sample.sigma for sample in samples(data)]
    assert len(sigmas) == 4
    assert sigmas[0] == pytest.approx(1.0, rel=1e-15)
    assert sigmas[1] == pytest.approx(0.013, rel=1e-15)
    assert sigmas[2] == pytest.approx(0.001, rel=1e-15)
    assert sigmas[3] == pytest.approx(1.4e-5, rel=1e-15)


def test_the_cce_ledgers_are_near_zero_at_the_capture_fit() -> None:
    rows = [_base("methane", 0.6), _base("nC10", 0.3), C15]
    fluid, csp = build_fluid(rows, parameters=[_tc_parameter()], values=[1.03])
    data = RegressionData(
        cce=(
            CceExperiment(
                temperature=373.15,
                pressures=(2.5e7, 2.0e7, 1.5e7),
                relative_volumes=(
                    0.9980317970636378,
                    1.1028215960619943,
                    1.3009108004036105,
                ),
            ),
        )
    )
    rows_out = samples(data)
    assert len(rows_out) == 3
    assert all(sample.sigma == pytest.approx(0.01, rel=1e-15) for sample in rows_out)
    assert chi_square(fluid, csp, rows_out, data) < 1e-6
    ledger = objectives(fluid, csp, data)
    assert ledger.cce is not None and ledger.cce < 1e-10


def test_the_dle_ledgers_are_near_zero_at_the_capture_fit() -> None:
    rows = [_base("methane", 0.45), _base("CO2", 0.05), _base("nC10", 0.50)]
    parameters = [{"parameter": "bip_co2_hc", "initial": 0.12, "lower": 0.08, "upper": 0.18}]
    fluid, csp = build_fluid(rows, parameters=parameters, values=[0.15])
    data = RegressionData(
        dle=(
            DleExperiment(
                temperature=323.15,
                pressures=(2.5e7, 1.5e7, 8.0e6),
                rs=(
                    124.11323743589683,
                    117.87406453877271,
                    54.85724347826862,
                ),
                bo=(
                    1.297774285932201,
                    1.3196221126198664,
                    1.1668447141587335,
                ),
                oil_density=(
                    657.1251440903429,
                    642.5894780343832,
                    684.8946122783024,
                ),
            ),
        )
    )
    rows_out = samples(data)
    assert len(rows_out) == 9
    assert chi_square(fluid, csp, rows_out, data) < 1e-6
    ledger = objectives(fluid, csp, data)
    assert ledger.dle is not None and ledger.dle < 1e-10


def test_a_zero_weight_is_refused() -> None:
    data = RegressionData(
        viscosity=(
            ViscosityPoint(temperature=320.0, pressure=5.0e5, viscosity=3.5e-4, phase="oil"),
        ),
        weights=ExperimentWeights(viscosity=0.0),
    )
    with pytest.raises(InvalidInputError, match="weight"):
        samples(data)
