"""The rich-gas envelope and critical point the Åsgard study could not compute.

The 13-component fluid is built from the databank's real substances plus three
Pedersen-SRK carbon-number cuts; the cut cards are computed by the library, not typed.
The continuation fails on this fluid, so these tests exercise the direct-flash fallback
and the critical-point refusal.
"""

from __future__ import annotations

import math

import pytest

import _helpers as h
from azoth import ureg, use_backend
from azoth.characterization import tbp_cut_properties, tbp_fraction_component
from azoth.core.result import Phase, PhaseClass
from azoth.core.units import quantity as q
from azoth.core.warnings import WarningCode
from azoth.eos import (
    IdealGasModel,
    critical_point,
    pt_phase_envelope,
    pt_flash_check,
    state_at_tp,
)
from azoth.eos import mixture as build_mixture
from azoth.eos.components import mixture_of
from azoth.eos.cubic import PR, SRK
from azoth.eos.reference.pr_peneloux_shift import pr_peneloux_shift
from azoth.eos.reference.srk_peneloux_shift import srk_peneloux_shift

Q = ureg.Quantity

SUPPLIED = [
    ("nitrogen", 0.21),
    ("co2", 7.98),
    ("methane", 79.38),
    ("ethane", 7.91),
    ("propane", 2.82),
    ("i-butane", 0.44),
    ("n-butane", 0.53),
    ("i-pentane", 0.28),
    ("n-pentane", 0.20),
    ("n-hexane", 0.11),
    ("C7", 0.09),
    ("C8", 0.05),
    ("C9", 0.01),
]
CUTS = {
    "C7": ("n-heptane", 100.204e-3, 684.4),
    "C8": ("n-octane", 114.232e-3, 702.8),
    "C9": ("n-nonane", 128.259e-3, 717.8),
}


def study_fluid(eos: str = "srk"):
    """The study's fluid, with its C7/C8/C9 cuts as Pedersen-SRK cards."""
    raw = [v / 100.0 for _, v in SUPPLIED]
    total = sum(raw)
    z = [v / total for v in raw]
    by_name = dict(zip((n for n, _ in SUPPLIED), z, strict=True))
    real = [n for n, _ in SUPPLIED if n not in CUTS and by_name[n] > 0.0]
    real_mix, ideal_real = mixture_of(real, eos=eos)
    cards = list(real_mix.components)
    ideal = [[], [], [], [], []]
    for vec, column in zip(
        ideal, (ideal_real.cp_a, ideal_real.cp_b, ideal_real.cp_c, ideal_real.cp_d, ideal_real.cp_e)
    ):
        vec.extend(float(v) for v in column)
    kij = {
        (i, j): real_mix.kij[i][j]
        for i in range(len(real))
        for j in range(i + 1, len(real))
        if real_mix.kij[i][j]
    }
    names = list(real)
    from azoth.eos.components import Component

    for cut, (analogue, mw, rho) in CUTS.items():
        props = tbp_cut_properties(molar_mass=q(mw, "kg/mol"), density=q(rho, "kg/m**3"),
                                   model="pedersen_srk")
        card = tbp_fraction_component(molar_mass=q(mw, "kg/mol"), density=q(rho, "kg/m**3"))
        tc = props.tc.to("K").magnitude
        pc = props.pc.to("Pa").magnitude
        omega = props.acentric_factor
        cards.append(
            Component(
                Tc=q(tc, "K"),
                Pc=q(pc, "Pa"),
                omega=omega,
                rackett_z=card.racket_z,
                molar_mass=q(mw, "kg/mol"),
                volume_shift=pr_peneloux_shift(
                    omega, q(tc, "K"), q(pc, "Pa"), card.racket_z
                )
                .c.to("m**3/mol")
                .magnitude,
                volume_shift_srk=srk_peneloux_shift(
                    omega, q(tc, "K"), q(pc, "Pa"), card.racket_z
                )
                .c.to("m**3/mol")
                .magnitude,
            )
        )
        names.append(cut)
        for vec, value in zip(
            ideal,
            (
                card.cp_a.to("J/(mol*K)").magnitude,
                card.cp_b.to("J/(mol*K**2)").magnitude,
                card.cp_c.to("J/(mol*K**3)").magnitude,
                card.cp_d.to("J/(mol*K**4)").magnitude,
                0.0,
            ),
        ):
            vec.append(float(value))
    mix = build_mixture(
        cards,
        kij=kij,
        cubic=SRK if eos == "srk" else PR,
        alpha=eos,
        names=names,
    )
    return names, [by_name[n] for n in names], mix, IdealGasModel(*[tuple(v) for v in ideal])


LITERAL = {"C7": "n-heptane", "C8": "n-octane", "C9": "n-nonane"}


def literal_fluid(eos: str = "srk"):
    """The same 13 components, with the cuts as the databank's n-alkanes."""
    names = [LITERAL.get(name, name) for name, _ in SUPPLIED]
    raw = [value / 100.0 for _, value in SUPPLIED]
    total = sum(raw)
    mix, ideal = mixture_of(names, eos=eos)
    return names, [value / total for value in raw], mix, ideal


def test_the_critical_point_is_a_documented_refusal() -> None:
    _, z, mix, _ = study_fluid("srk")
    result = critical_point(mix, z=z)
    assert result.converged is False
    assert math.isnan(result.tc.to("K").magnitude)
    assert any(w.code is WarningCode.SOLVER_NOT_CONVERGED for w in result.warnings)


def test_the_envelope_falls_back_to_the_grid(monkeypatch: pytest.MonkeyPatch) -> None:
    import importlib

    envelope = importlib.import_module("azoth.eos.reference.pt_phase_envelope")
    monkeypatch.setattr(envelope, "_GRID_T_STEPS", 8)
    monkeypatch.setattr(envelope, "_GRID_P_STEPS", 8)
    monkeypatch.setattr(envelope, "_GRID_BISECTIONS", 4)
    _, z, mix, _ = study_fluid("srk")
    result = pt_phase_envelope(mix, P=q(1e5, "Pa"), z=z)
    assert math.isfinite(result.cricondenbar_pressure.to("bar").magnitude)
    assert math.isfinite(result.cricondentherm_temperature.to("K").magnitude)
    assert any(w.code is WarningCode.SOLVER_NOT_CONVERGED for w in result.warnings)
    assert math.isnan(result.critical_temperature.to("K").magnitude)


def _upper_at_10c(mix, z) -> float:
    """The upper two-phase pressure at 10 °C, by the study's direct-flash bisection."""
    t = 283.15
    pressures = [1.0e5 + (400.0e5 - 1.0e5) * i / 39.0 for i in range(40)]

    def two(p: float) -> bool:
        flash = pt_flash_check(mix, q(t, "K"), q(p, "Pa"), z, tolerance=1e-6, strict=False)
        beta = flash.vapour_fraction
        return (
            flash.converged
            and flash.phase is Phase.TWO_PHASE
            and beta is not None
            and 1e-9 < beta < 1 - 1e-9
        )

    flags = [two(p) for p in pressures]
    last = len(flags) - 1 - flags[::-1].index(True)
    a, b = pressures[last], pressures[last + 1]
    for _ in range(24):
        mid = 0.5 * (a + b)
        if two(mid):
            a = mid
        else:
            b = mid
    return 0.5 * (a + b) / 1e5


def _two_phase_band(mix, z, t: float) -> tuple[float, float]:
    """The lower and upper two-phase pressures at one temperature, in bar."""
    pressures = [1.0e5 + (400.0e5 - 1.0e5) * i / 39.0 for i in range(40)]

    def two(p: float) -> bool:
        flash = pt_flash_check(mix, q(t, "K"), q(p, "Pa"), z, tolerance=1e-6, strict=False)
        beta = flash.vapour_fraction
        return (
            flash.converged
            and flash.phase is Phase.TWO_PHASE
            and beta is not None
            and 1e-9 < beta < 1 - 1e-9
        )

    flags = [two(p) for p in pressures]
    if not any(flags):
        return math.nan, math.nan
    first = flags.index(True)
    last = len(flags) - 1 - flags[::-1].index(True)
    lower = math.nan
    if first > 0:
        a, b = pressures[first - 1], pressures[first]
        for _ in range(24):
            mid = 0.5 * (a + b)
            if two(mid):
                b = mid
            else:
                a = mid
        lower = 0.5 * (a + b) / 1e5
    upper = math.nan
    if last < len(pressures) - 1:
        a, b = pressures[last], pressures[last + 1]
        for _ in range(24):
            mid = 0.5 * (a + b)
            if two(mid):
                a = mid
            else:
                b = mid
        upper = 0.5 * (a + b) / 1e5
    return lower, upper


def test_the_ten_degree_upper_boundary_and_dropout() -> None:
    _, z, mix, ideal = study_fluid("srk")
    assert _upper_at_10c(mix, z) == pytest.approx(99.84, abs=0.2)
    flash = pt_flash_check(mix, q(283.15, "K"), q(60e5, "Pa"), z, tolerance=1e-10)
    beta = flash.vapour_fraction
    assert beta is not None
    liquid_mol = 1.0 - beta
    zl = flash.z_liquid
    zv = flash.z_vapour
    vl = zl * 8.31446261815324 * 283.15 / 60e5
    vv = zv * 8.31446261815324 * 283.15 / 60e5
    vm = (1 - beta) * vl + beta * vv
    liquid_vol = (1 - beta) * vl / vm
    assert liquid_mol == pytest.approx(0.005412, abs=2e-5)
    assert liquid_vol == pytest.approx(0.001763, abs=2e-5)


def test_the_low_pressure_dew_boundaries() -> None:
    _, z, mix, _ = study_fluid("srk")
    for celsius, expected in ((0, 1.60), (5, 2.38), (10, 3.51), (15, 5.15)):
        lower, _ = _two_phase_band(mix, z, 273.15 + celsius)
        assert lower == pytest.approx(expected, abs=0.02)


def test_the_two_reported_bad_states() -> None:
    """180 bara was `trivial`; 100 bara was `all_vapour` on a 230 kg/m3 fluid.

    The flash's own label for a single-phase dense state is the trivial solution or
    the negative flash's `all_vapour`; the screening class is what says it is dense.
    """
    dense = (PhaseClass.DENSE_SINGLE_PHASE, PhaseClass.DENSE_SUPERCRITICAL)
    _, z, mix, ideal = study_fluid("srk")
    for pressure in (180.0e5, 100.0e5):
        flash = pt_flash_check(
            mix, q(283.15, "K"), q(pressure, "Pa"), z, tolerance=1e-8, strict=False
        )
        assert flash.converged
        if flash.phase is Phase.TWO_PHASE:
            beta = flash.vapour_fraction
            assert beta is not None
            assert 0.0 < beta < 1.0
        state = state_at_tp(
            mix, ideal, q(283.15, "K"), q(pressure, "Pa"), z, tolerance=1e-8, strict=False
        )
        assert state.converged
        assert math.isfinite(state.density.to("kg/m**3").magnitude)
        assert state.phase_class in dense


@pytest.mark.slow_sweep
def test_the_pr_characterisation_numbers() -> None:
    with use_backend("rust"):
        _, z, mix, _ = study_fluid("pr")
        result = pt_phase_envelope(mix, P=q(1e5, "Pa"), z=z)
        lower, upper = _two_phase_band(mix, z, 283.15)
    assert result.cricondenbar_pressure.to("bar").magnitude == pytest.approx(102.07, abs=0.2)
    assert (
        result.cricondenbar_temperature.to("K").magnitude - 273.15
        == pytest.approx(-8.8, abs=0.3)
    )
    assert lower == pytest.approx(4.20, abs=0.2)
    assert upper == pytest.approx(93.90, abs=0.2)


@pytest.mark.slow_sweep
def test_the_literal_n_alkane_cuts() -> None:
    with use_backend("rust"):
        _, z, mix, _ = literal_fluid("srk")
        result = pt_phase_envelope(mix, P=q(1e5, "Pa"), z=z)
    assert result.cricondenbar_pressure.to("bar").magnitude == pytest.approx(99.85, abs=0.3)
    assert (
        result.cricondenbar_temperature.to("K").magnitude - 273.15
        == pytest.approx(-9.1, abs=0.3)
    )


@pytest.mark.slow_sweep
def test_the_full_study_envelope_numbers() -> None:
    """The full 40x40 grid, Rust backend, reported manually; ~1 minute."""
    with use_backend("rust"):
        _, z, mix, _ = study_fluid("srk")
        result = pt_phase_envelope(mix, P=q(1e5, "Pa"), z=z)
    assert result.cricondenbar_pressure.to("bar").magnitude == pytest.approx(105.39, abs=0.2)
    assert (
        result.cricondenbar_temperature.to("K").magnitude - 273.15 == pytest.approx(-5.7, abs=0.3)
    )
