"""``pvt.water_correlations`` through its public wrapper."""

from __future__ import annotations

import pytest

from azoth.core.errors import InvalidInputError
from azoth.core.units import quantity
from azoth.pvt import water_correlations


def test_the_wrapper_publishes_quantities() -> None:
    result = water_correlations(
        [
            {
                "calculation": "brine_density_batzle_wang",
                "temperature": quantity(366.48, "K"),
                "pressure": quantity(20684000.0, "Pa"),
                "salinity_wt_fraction": 0.03,
            },
            {
                "calculation": "water_viscosity_mccain",
                "temperature": quantity(366.48, "K"),
                "pressure": quantity(20684000.0, "Pa"),
                "salinity_ppm": 30000.0,
            },
        ]
    )
    assert result.is_clean
    density = result.queries[0].density
    viscosity = result.queries[1].viscosity
    assert density is not None and viscosity is not None
    assert density.to("kg/m**3").magnitude == pytest.approx(994.5440241141248, rel=1.0e-9)
    assert viscosity.to("Pa*s").magnitude == pytest.approx(3.8518832767548723e-4, rel=1.0e-9)


def test_the_mccain_rsw_is_reduced_by_salinity() -> None:
    pure = water_correlations(
        [
            {
                "calculation": "rsw_mccain",
                "temperature": quantity(373.15, "K"),
                "pressure": quantity(10000000.0, "Pa"),
                "salinity_molality": quantity(0.0, "mol/kg"),
            }
        ]
    )
    saline = water_correlations(
        [
            {
                "calculation": "rsw_mccain",
                "temperature": quantity(373.15, "K"),
                "pressure": quantity(10000000.0, "Pa"),
                "salinity_molality": quantity(0.6206267976011888, "mol/kg"),
            }
        ]
    )
    assert pure.queries[0].rsw is not None and saline.queries[0].rsw is not None
    assert pure.queries[0].rsw == pytest.approx(1.5791188803621292, rel=1.0e-9)
    assert saline.queries[0].rsw < pure.queries[0].rsw


def test_a_negative_salinity_is_refused() -> None:
    with pytest.raises(InvalidInputError, match=r"queries\[0\]\.salinity_ppm"):
        water_correlations(
            [
                {
                    "calculation": "water_fvf_osif",
                    "temperature": quantity(366.48, "K"),
                    "pressure": quantity(20684000.0, "Pa"),
                    "salinity_ppm": -1.0,
                }
            ]
        )


def test_a_weight_fraction_outside_the_range_is_refused() -> None:
    with pytest.raises(InvalidInputError, match=r"queries\[0\]\.salinity_wt_fraction"):
        water_correlations(
            [
                {
                    "calculation": "brine_density_batzle_wang",
                    "temperature": quantity(366.48, "K"),
                    "pressure": quantity(20684000.0, "Pa"),
                    "salinity_wt_fraction": 0.31,
                }
            ]
        )


def test_an_unknown_calculation_is_refused() -> None:
    with pytest.raises(InvalidInputError, match=r"queries\[0\]\.calculation"):
        water_correlations([{"calculation": "not_a_calculation"}])
