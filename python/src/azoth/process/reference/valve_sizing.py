"""``ControlValveSizing``, the ``ValveMechanicalDesign`` default: the IEC 60534 simplified sizing.

The Python twin of ``crates/azoth-process/src/valve.rs``. Every step mirrors the Rust module, and
**the two halves are held to the same capture**: the Rust module's own tests and
``python/tests/test_cross_impl.py`` compare the rows of
``validation/neqsim/captures/process_valve_sizing.tsv``.

**The default sizing standard and not the IEC class.** ``setValveSizingStandard`` selects the
others and this module is the default: ``calcKv``, its inverse flow and its outlet-pressure solve,
on the liquid and gas branches, with the opening characteristic and the fouling fraction.

**The gas formula is the class's, unit mixing included.** ``Q`` at standard conditions (273.15 K,
101.325 kPa) comes from the actual volumetric flow through ``P1``, ``T`` and ``Z``, then
``Kv = Q_std/(N9 P1 Y) sqrt(M T Z / x)`` with ``N9 = 24.6``, ``M`` in g/mol, ``T`` in kelvin and
``P1`` in kPa. The liquid form is ``Kv = Q/sqrt(dP/SG)`` with ``SG = rho/1000``. Both divide by
the opening factor, so the reported ``Kv`` is the rated value at 100% opening.
"""

from __future__ import annotations

import math
from dataclasses import dataclass

from azoth.core.units import quantity
from azoth.eos import components as _components
from azoth.eos.reference.molar_enthalpy_entropy import molar_enthalpy_entropy
from azoth.eos.reference.pr_molar_volume import pr_molar_volume
from azoth.eos.reference.pt_flash import pt_flash
from azoth.process.reference import capacity

#: ``Cv = Kv * 1.156``.
KV_TO_CV_FACTOR = 1.156
#: The IEC 60534 constant for standard m3/h, kPa, K and g/mol.
N9 = 24.6
#: The standard temperature the gas flow is quoted at, K.
STANDARD_TEMPERATURE_K = 273.15
#: The standard pressure the gas flow is quoted at, kPa.
STANDARD_PRESSURE_KPA = 101.325
#: ``xT``'s own initialiser.
DEFAULT_XT = 0.137

DEFAULT_CHOKE_DIAMETER_M = 0.0254
#: ``ControlValveSizing_IEC_60534.N1``.
IEC_N1 = 0.1
#: ``ControlValveSizing_IEC_60534.rho0``.
IEC_RHO0 = 999.103
#: ``ControlValveSizing_IEC_60534.FL``, the liquid pressure recovery factor.
IEC_FL = 0.9
#: ``ControlValveSizing_simple.Cd``, the production choke's discharge coefficient.
SIMPLE_CD = 0.85
#: The nine ``setValveSizingStandard`` values.
SIZING_STANDARDS = (
    "default",
    "iec_60534",
    "iec_60534_full",
    "prod_choke",
    "sachdeva",
    "gilbert",
    "baxendell",
    "ros",
    "achong",
)
#: The five standards that read the flash's labelled phases.
CHOKE_STANDARDS = ("sachdeva", "gilbert", "baxendell", "ros", "achong")
#: The class each standard selects.
STANDARD_CLASSES = {
    "default": "ControlValveSizing",
    "iec_60534": "ControlValveSizing_IEC_60534",
    "iec_60534_full": "ControlValveSizing_IEC_60534_full",
    "prod_choke": "ControlValveSizing_simple",
    "sachdeva": "ControlValveSizing_MultiphaseChoke",
    "gilbert": "ControlValveSizing_MultiphaseChoke",
    "baxendell": "ControlValveSizing_MultiphaseChoke",
    "ros": "ControlValveSizing_MultiphaseChoke",
    "achong": "ControlValveSizing_MultiphaseChoke",
}


def default_allow_choked(standard: str) -> bool:
    """``setValveSizingStandard``'s constructor state: only the default starts ``false``."""
    return standard != "default"


#: ``Cg = Cv * 1360``.
CG_PER_CV = 1360.0
#: The bisection's ``Kv`` tolerance and cap, the class's own.
BISECTION_TOLERANCE = 1.0e-6
MAX_BISECTION_ITERATIONS = 100
#: ``ThermodynamicConstantsInterface.R``, J/(mol*K).
R_J_PER_MOL_K = 8.314
#: ``ThrottlingValve.percentValveOpening``'s initialiser.
DEFAULT_OPENING_PERCENT = 100.0
#: ``EqualPercentageCharacteristic``'s default rangeability.
DEFAULT_RANGEABILITY = 50.0
#: ``ModifiedParabolicCharacteristic``'s default exponent.
DEFAULT_PARABOLIC_EXPONENT = 2.0

#: The class's four ``ValveCharacteristic`` implementations.
CHARACTERISTICS = ("linear", "equal_percentage", "quick_opening", "modified_parabolic")


def opening_factor(characteristic: str, percent_opening: float) -> float:
    """``getOpeningFactor(percentOpening)``: the fraction of the rated ``Kv``."""
    opening = min(100.0, max(0.0, percent_opening))
    x = opening / 100.0
    if characteristic == "linear":
        return x
    if characteristic == "equal_percentage":
        if opening <= 0.0:
            return 1.0 / DEFAULT_RANGEABILITY
        if opening >= 100.0:
            return 1.0
        return float(DEFAULT_RANGEABILITY ** (x - 1.0))
    if characteristic == "quick_opening":
        return math.sqrt(x)
    return float(x**DEFAULT_PARABOLIC_EXPONENT)


@dataclass(frozen=True, slots=True)
class ValveSizingGeometry:
    """The sizing's configuration, as ``ValveMechanicalDesign`` and ``ThrottlingValve`` hold it."""

    kv: float | None = None
    opening_percent: float = DEFAULT_OPENING_PERCENT
    fouling_fraction: float = 0.0
    characteristic: str = "linear"
    xt: float = DEFAULT_XT
    allow_choked: bool = False
    choke_diameter_m: float = 0.0254
    discharge_coefficient: float | None = None


@dataclass(frozen=True, slots=True)
class ValveSizingReport:
    """What ``calcValveSize`` and its inverse solve report."""

    kv: float
    cv: float
    cg: float
    choked: bool
    gas_valve: bool
    calculated_molar_flow: float
    calculated_outlet_pressure: float


@dataclass(frozen=True, slots=True)
class _Reads:
    p1_bara: float
    t_k: float
    flow_m3hr: float
    density_kg_m3: float
    z: float
    gamma: float
    molar_mass_gr_mol: float


def gas_valve(components: list[str], t: float, p: float, z: list[float]) -> bool:
    """``isGasValve()``: a gas phase carrying more than half the volume."""
    mixture, labelled, _shares = capacity._labelled_phases(components, t, p, z)
    flash = pt_flash(mixture, quantity(t, "K"), quantity(p, "Pa"), list(z))
    if flash.phase == "all_vapour":
        return True
    if flash.phase in ("all_liquid", "trivial"):
        return False
    y = flash.vapour_fraction if flash.vapour_fraction is not None else 0.0
    # The first labelled phase is the gas and the second the liquid, as the class's own order.
    gas_density = capacity._mass_density(mixture, t, p, labelled[0][1], labelled[0][2])
    liquid_density = capacity._mass_density(mixture, t, p, labelled[1][1], labelled[1][2])
    gas_mass = capacity._molar_mass(mixture, labelled[0][1])
    liquid_mass = capacity._molar_mass(mixture, labelled[1][1])
    gas_volume = y * gas_mass / gas_density
    liquid_volume = (1.0 - y) * liquid_mass / liquid_density
    return gas_volume / (gas_volume + liquid_volume) > 0.5


def _reads(components: list[str], t: float, p: float, z: list[float], n: float) -> _Reads:
    # **The phase's own root, not a negative flash's trial phases.** NeqSim reads the gas
    # phase's density and flow off the feed's composition; a negative flash reports a pair of
    # hypothetical compositions whose roots are the wrong side of the cubic, and at 30 bara
    # that is `0.873` against the feed's `0.920`. `Stream::single_phase_root` is the Rust
    # half's statement of the same rule.
    density = capacity.system_mass_density(components, t, p, z, from_feed=True)
    flow_m3s = capacity.system_volumetric_flow(components, t, p, z, n, from_feed=True)
    mixture, ideal_gas = _components.mixture_of(components, eos="pr")
    molar_mass = capacity._molar_mass(mixture, list(z))
    flash = pt_flash(mixture, quantity(t, "K"), quantity(p, "Pa"), list(z))
    if flash.phase == "two_phase":
        from azoth.core.errors import InvalidInputError

        raise InvalidInputError(
            "stream",
            f"a two-phase state at {t} K and {p} Pa has no one root for the sizing",
        )
    from azoth.eos.reference._mixture_state import phase_state, reduced_parameters

    reduced = reduced_parameters(mixture, t, p)
    kij = mixture.effective_kij(reduced.t_kelvin)
    root = phase_state(reduced, kij, list(z), liquid=flash.phase != "all_vapour").z
    volume = pr_molar_volume(root, quantity(t, "K"), quantity(p, "Pa")).v.to("m**3/mol").magnitude
    state = molar_enthalpy_entropy(
        mixture, ideal_gas, quantity(t, "K"), quantity(p, "Pa"), list(z), root
    )
    cp = float(state.cp.to_base_units().magnitude)
    return _Reads(
        p1_bara=p / 1.0e5,
        t_k=t,
        flow_m3hr=flow_m3s * 3600.0,
        density_kg_m3=density,
        z=p * volume / (R_J_PER_MOL_K * t),
        gamma=cp / (cp - R_J_PER_MOL_K),
        molar_mass_gr_mol=molar_mass * 1000.0,
    )


def adjusted_kv(rated_kv: float, geometry: ValveSizingGeometry) -> float:
    """``getActualKv`` then ``adjustKv``."""
    return (
        rated_kv
        * opening_factor(geometry.characteristic, geometry.opening_percent)
        * (1.0 - geometry.fouling_fraction)
    )


def calc_kv(
    components: list[str],
    t: float,
    p: float,
    z: list[float],
    n: float,
    outlet_bara: float,
    geometry: ValveSizingGeometry,
) -> float:
    """``calcKv``: the rated ``Kv`` the current flow and drop require."""
    read = _reads(components, t, p, z, n)
    gas = gas_valve(components, t, p, z)
    delta_p = read.p1_bara - outlet_bara
    if delta_p <= 0.0:
        from azoth.core.errors import InvalidInputError

        raise InvalidInputError(
            "outlet_pressure",
            f"sizing needs a positive drop, and {outlet_bara} bara against {read.p1_bara} bara "
            f"is {delta_p} bar",
        )
    factor = opening_factor(geometry.characteristic, geometry.opening_percent)
    if factor <= 0.0:
        from azoth.core.errors import InvalidInputError

        raise InvalidInputError(
            "valve_opening",
            f"an opening of {geometry.opening_percent}% gives an opening factor of {factor}, "
            f"and `calcKv` divides by it",
        )
    if not gas:
        sg = read.density_kg_m3 / 1000.0
        kv = read.flow_m3hr / math.sqrt(delta_p / sg)
    else:
        kv = _gas_kv(read, delta_p, geometry)
    return kv / factor


def _gas_kv(read: _Reads, delta_p: float, geometry: ValveSizingGeometry) -> float:
    p1_kpa = read.p1_bara * 100.0
    x = delta_p / read.p1_bara
    f_gamma = read.gamma / 1.40
    choked = x >= f_gamma * geometry.xt
    x_effective = f_gamma * geometry.xt if choked and geometry.allow_choked else x
    y = max(1.0 - x_effective / (3.0 * f_gamma * geometry.xt), 2.0 / 3.0)
    standard_flow = (
        read.flow_m3hr
        * (p1_kpa / STANDARD_PRESSURE_KPA)
        * (STANDARD_TEMPERATURE_K / read.t_k)
        / read.z
    )
    x_scale = geometry.xt * f_gamma if choked and geometry.allow_choked else x
    return (
        standard_flow
        / (N9 * p1_kpa * y)
        * math.sqrt(read.molar_mass_gr_mol * read.t_k * read.z / x_scale)
    )


def calculated_molar_flow(
    components: list[str],
    t: float,
    p: float,
    z: list[float],
    n: float,
    outlet_bara: float,
    adjusted: float,
    geometry: ValveSizingGeometry,
) -> float:
    """``calculateMolarFlow()``: the mole flow the adjusted ``Kv`` and the drop give."""
    read = _reads(components, t, p, z, n)
    delta_p = read.p1_bara - outlet_bara
    if delta_p <= 0.0:
        return 0.0
    if not gas_valve(components, t, p, z):
        sg = read.density_kg_m3 / 1000.0
        flow_m3s = adjusted * math.sqrt(delta_p / sg) / 3600.0
    else:
        p1_kpa = read.p1_bara * 100.0
        x = delta_p / read.p1_bara
        f_gamma = read.gamma / 1.40
        choked = x >= f_gamma * geometry.xt
        x_effective = f_gamma * geometry.xt if choked and geometry.allow_choked else x
        y = max(1.0 - x_effective / (3.0 * f_gamma * geometry.xt), 2.0 / 3.0)
        x_scale = geometry.xt * f_gamma if choked and geometry.allow_choked else x
        standard_flow = (
            adjusted
            * N9
            * p1_kpa
            * y
            / math.sqrt(read.molar_mass_gr_mol * read.t_k * read.z / x_scale)
        )
        actual = (
            standard_flow
            * (STANDARD_PRESSURE_KPA / p1_kpa)
            * (read.t_k / STANDARD_TEMPERATURE_K)
            * read.z
        )
        flow_m3s = actual / 3600.0
    return flow_m3s * read.density_kg_m3 / (read.molar_mass_gr_mol / 1000.0)


def calculated_outlet_pressure(
    components: list[str],
    t: float,
    p: float,
    z: list[float],
    n: float,
    adjusted: float,
    geometry: ValveSizingGeometry,
) -> float:
    """``findOutletPressureForFixedKv``: the outlet pressure an adjusted ``Kv`` gives, bara."""
    from azoth.core.errors import InvalidInputError

    if adjusted <= 0.0:
        raise InvalidInputError("kv", f"an adjusted Kv of {adjusted} gives no outlet pressure")
    read = _reads(components, t, p, z, n)
    if not gas_valve(components, t, p, z):
        sg = read.density_kg_m3 / 1000.0
        return read.p1_bara - (read.flow_m3hr / adjusted) ** 2 * sg
    p1_kpa = read.p1_bara * 100.0
    standard_flow = (
        read.flow_m3hr
        * (p1_kpa / STANDARD_PRESSURE_KPA)
        * (STANDARD_TEMPERATURE_K / read.t_k)
        / read.z
    )
    f_gamma = read.gamma / 1.40
    low = 0.1
    high = read.p1_bara - 0.001
    mid = (low + high) / 2.0
    for _ in range(MAX_BISECTION_ITERATIONS):
        mid = (low + high) / 2.0
        p2_kpa = mid * 100.0
        x = (p1_kpa - p2_kpa) / p1_kpa
        choked = x >= f_gamma * geometry.xt
        x_effective = f_gamma * geometry.xt if choked and geometry.allow_choked else x
        y = max(1.0 - x_effective / (3.0 * f_gamma * geometry.xt), 2.0 / 3.0)
        x_scale = geometry.xt * f_gamma if choked and geometry.allow_choked else x
        kv = (
            standard_flow
            / (N9 * p1_kpa * y)
            * math.sqrt(read.molar_mass_gr_mol * read.t_k * read.z / x_scale)
        )
        if abs(kv - adjusted) / adjusted < BISECTION_TOLERANCE:
            break
        if kv > adjusted:
            high = mid
        else:
            low = mid
    return mid


def sizing_report(
    components: list[str],
    t: float,
    p: float,
    z: list[float],
    n: float,
    outlet_bara: float,
    geometry: ValveSizingGeometry,
) -> ValveSizingReport:
    """The whole report: the sized (or stated) ``Kv`` and the inverse solves."""
    gas = gas_valve(components, t, p, z)
    read = _reads(components, t, p, z, n)
    if geometry.kv is not None:
        kv = geometry.kv
    elif read.p1_bara - outlet_bara > 0.0:
        kv = calc_kv(components, t, p, z, n, outlet_bara, geometry)
    else:
        # **A non-positive drop has no sizing**, and ``calcKv`` answers ``NaN`` there; the
        # ``acceptNegativeDP`` row's stream is still a state.
        kv = math.nan
    if not math.isfinite(kv):
        return ValveSizingReport(
            kv=kv,
            cv=math.nan,
            cg=math.nan,
            choked=False,
            gas_valve=gas,
            calculated_molar_flow=0.0,
            calculated_outlet_pressure=math.nan,
        )
    adjusted = adjusted_kv(kv, geometry)
    choked = (
        gas and (read.p1_bara - outlet_bara) / read.p1_bara >= (read.gamma / 1.40) * geometry.xt
    )
    return ValveSizingReport(
        kv=kv,
        cv=kv * KV_TO_CV_FACTOR,
        cg=kv * KV_TO_CV_FACTOR * CG_PER_CV,
        choked=choked,
        gas_valve=gas,
        calculated_molar_flow=calculated_molar_flow(
            components, t, p, z, n, outlet_bara, adjusted, geometry
        ),
        calculated_outlet_pressure=calculated_outlet_pressure(
            components, t, p, z, n, adjusted, geometry
        ),
    )


def _ieee_div(numerator: float, denominator: float) -> float:
    """Java's double division: a zero denominator gives an infinity, not a ``ZeroDivisionError``."""
    if denominator == 0.0:
        if numerator == 0.0:
            return math.nan
        return math.copysign(math.inf, numerator)
    return numerator / denominator


@dataclass(frozen=True, slots=True)
class _IecReads:
    reads: _Reads
    gas: bool
    pseudo_critical_pa: float


def _require_single_phase(
    components: list[str], t: float, p: float, z: list[float], standard: str
) -> None:
    mixture, _ = _components.mixture_of(components, eos="pr")
    flash = pt_flash(mixture, quantity(t, "K"), quantity(p, "Pa"), list(z))
    if flash.phase == "two_phase":
        from azoth.core.errors import InvalidInputError

        raise InvalidInputError(
            "inlet_z",
            f"the {STANDARD_CLASSES[standard]} standard reads a single phase's root, "
            f"and this inlet is two phases at {t} K and {p} Pa",
        )


def _iec_reads(components: list[str], t: float, p: float, z: list[float], n: float) -> _IecReads:
    read = _reads(components, t, p, z, n)
    mixture, _ = _components.mixture_of(components, eos="pr")
    flash = pt_flash(mixture, quantity(t, "K"), quantity(p, "Pa"), list(z))
    pseudo_critical_pa = sum(
        fraction * _components.entry(name).Pc.to("Pa").magnitude
        for name, fraction in zip(components, z, strict=True)
    )
    return _IecReads(
        reads=read,
        gas=flash.phase in ("all_vapour", "two_phase"),
        pseudo_critical_pa=pseudo_critical_pa,
    )


def _iec_reads_with_flow(read: _IecReads, flow_m3s: float) -> _IecReads:
    original = read.reads
    return _IecReads(
        reads=_Reads(
            p1_bara=original.p1_bara,
            t_k=original.t_k,
            flow_m3hr=flow_m3s * 3600.0,
            density_kg_m3=original.density_kg_m3,
            z=original.z,
            gamma=original.gamma,
            molar_mass_gr_mol=original.molar_mass_gr_mol,
        ),
        gas=read.gas,
        pseudo_critical_pa=read.pseudo_critical_pa,
    )


def _iec_gas_kv(
    read: _IecReads, p1_pa: float, p2_pa: float, xt: float, allow_choked: bool
) -> tuple[float, bool]:
    r = read.reads
    loc_p1 = p1_pa / 1000.0
    loc_p2 = p2_pa / 1000.0
    d_p = loc_p1 - loc_p2
    x = _ieee_div(d_p, loc_p1)
    f_gamma = r.gamma / 1.40
    y = max(1.0 - _ieee_div(x, 3.0 * f_gamma * xt), 2.0 / 3.0)
    choked = x >= f_gamma * xt
    q_std = r.flow_m3hr * (loc_p1 / STANDARD_PRESSURE_KPA) * (STANDARD_TEMPERATURE_K / r.t_k) / r.z
    if choked and allow_choked:
        kv = (
            q_std
            / (N9 * loc_p1 * y)
            * math.sqrt(r.molar_mass_gr_mol * r.t_k * r.z / (xt * f_gamma))
        )
    else:
        kv = q_std / (N9 * loc_p1 * y) * math.sqrt(r.molar_mass_gr_mol * r.t_k * r.z / x)
    return kv, choked


def _iec_liquid_kv(
    read: _IecReads,
    psat_pa: float,
    pc_pa: float,
    p1_pa: float,
    p2_pa: float,
    allow_choked: bool,
) -> tuple[float, bool]:
    r = read.reads
    loc_p1 = p1_pa / 1000.0
    loc_p2 = p2_pa / 1000.0
    loc_psat = psat_pa / 1000.0
    loc_pc = pc_pa / 1000.0
    d_p = loc_p1 - loc_p2
    ff = 0.96 - 0.28 * math.sqrt(_ieee_div(loc_psat, loc_pc))
    choked = d_p >= IEC_FL * IEC_FL * (loc_p1 - ff * loc_psat)
    q_loc = r.flow_m3hr
    if choked and allow_choked:
        kv = q_loc / (
            IEC_N1
            * IEC_FL
            * math.sqrt(_ieee_div((loc_p1 - ff * loc_psat) * IEC_RHO0, r.density_kg_m3))
        )
    else:
        kv = q_loc / (IEC_N1 * math.sqrt(_ieee_div(d_p * IEC_RHO0, r.density_kg_m3)))
    return kv, choked


def _iec_gas_flow_m3s(
    read: _IecReads,
    adjusted: float,
    p1_pa: float,
    p2_pa: float,
    xt: float,
    allow_choked: bool,
) -> float:
    r = read.reads
    loc_p1 = p1_pa / 1000.0
    loc_p2 = p2_pa / 1000.0
    d_p = loc_p1 - loc_p2
    x = _ieee_div(d_p, loc_p1)
    f_gamma = r.gamma / 1.40
    y = max(1.0 - _ieee_div(x, 3.0 * f_gamma * xt), 2.0 / 3.0)
    choked = x >= f_gamma * xt
    if choked and allow_choked:
        q_std = (
            adjusted
            * N9
            * loc_p1
            * y
            / math.sqrt(r.molar_mass_gr_mol * r.t_k * r.z / (xt * f_gamma))
        )
    else:
        q_std = adjusted * N9 * loc_p1 * y / math.sqrt(r.molar_mass_gr_mol * r.t_k * r.z / x)
    q = q_std * (STANDARD_PRESSURE_KPA / loc_p1) * (r.t_k / STANDARD_TEMPERATURE_K) * r.z
    return q / 3600.0


def _iec_liquid_flow_m3s(
    read: _IecReads,
    adjusted: float,
    psat_pa: float,
    pc_pa: float,
    p1_pa: float,
    p2_pa: float,
    allow_choked: bool,
) -> float:
    r = read.reads
    loc_p1 = p1_pa / 1000.0
    loc_p2 = p2_pa / 1000.0
    loc_psat = psat_pa / 1000.0
    d_p = loc_p1 - loc_p2
    ff = 0.96 - 0.28 * math.sqrt(_ieee_div(loc_psat, pc_pa / 1000.0))
    choked = d_p >= IEC_FL * IEC_FL * (loc_p1 - ff * loc_psat)
    if choked and allow_choked:
        q_loc = (
            IEC_N1
            * adjusted
            * IEC_FL
            * math.sqrt(_ieee_div((loc_p1 - ff * loc_psat) * IEC_RHO0, r.density_kg_m3))
        )
    else:
        q_loc = IEC_N1 * adjusted * math.sqrt(_ieee_div(d_p * IEC_RHO0, r.density_kg_m3))
    return q_loc / 3600.0


def _iec_gas_pressure(
    read: _IecReads, adjusted: float, p1_pa: float, xt: float, allow: bool
) -> float:
    low = 1.0
    high = p1_pa - 1.0e-4
    mid = 0.5 * (low + high)
    for _ in range(50):
        required, _ = _iec_gas_kv(read, p1_pa, mid, xt, allow)
        if required < adjusted:
            low = mid
        else:
            high = mid
        old = mid
        mid = 0.5 * (low + high)
        if mid > 1.0e-9 and abs(mid - old) / mid < 1.0e-6:
            break
    return mid


def _iec_liquid_pressure(
    read: _IecReads,
    adjusted: float,
    opening: float,
    allow: bool,
    divide_by_opening: bool,
) -> float:
    p1_pa = read.reads.p1_bara * 1.0e5
    psat_pa = p1_pa
    pc_pa = read.pseudo_critical_pa
    low = 0.1
    high = p1_pa / 1000.0 - 1.0e-4
    mid = 0.5 * (low + high)
    for _ in range(50):
        required, _ = _iec_liquid_kv(read, psat_pa, pc_pa, p1_pa, mid * 1000.0, allow)
        if divide_by_opening:
            required = required / opening
        if required < adjusted:
            low = mid
        else:
            high = mid
        old = mid
        mid = 0.5 * (low + high)
        if abs(mid - old) < 1.0e-6:
            break
    return mid * 1000.0


def _full_gas_flow_m3s_at(
    read: _IecReads, adjusted: float, outlet_bara: float, xt: float, allow: bool
) -> float:
    r = read.reads
    p1_pa = r.p1_bara * 1.0e5
    p2_pa = outlet_bara * 1.0e5
    x = (p1_pa - p2_pa) / p1_pa if p1_pa > p2_pa else 1.0e-3
    y = 0.8
    q_high_m3h = (
        N9 * adjusted * (p1_pa / 1000.0) * y / math.sqrt(r.molar_mass_gr_mol * r.t_k * r.z / x)
    )
    q_low = 0.0
    q_high = q_high_m3h / 3600.0 * 2.0
    q_mid = 0.0
    for _ in range(20):
        q_mid = 0.5 * (q_low + q_high)
        if q_mid < 1.0e-9:
            break
        required, _ = _iec_gas_kv(_iec_reads_with_flow(read, q_mid), p1_pa, p2_pa, xt, allow)
        if required < adjusted:
            q_low = q_mid
        else:
            q_high = q_mid
        if abs(q_high - q_low) < 1.0e-6:
            break
    return q_mid


def _full_liquid_flow_m3s_at(
    read: _IecReads, adjusted: float, outlet_bara: float, allow: bool
) -> float:
    r = read.reads
    p1_pa = r.p1_bara * 1.0e5
    p2_pa = outlet_bara * 1.0e5
    d_p_kpa = (p1_pa - p2_pa) / 1000.0 if p1_pa > p2_pa else 1.0e-3
    q_high_m3h = IEC_N1 * adjusted * math.sqrt(_ieee_div(d_p_kpa * IEC_RHO0, r.density_kg_m3))
    q_low = 0.0
    q_high = q_high_m3h / 3600.0 * 2.0
    q_mid = 0.0
    for _ in range(20):
        q_mid = 0.5 * (q_low + q_high)
        if q_mid < 1.0e-9:
            break
        required, _ = _iec_liquid_kv(
            _iec_reads_with_flow(read, q_mid),
            p1_pa,
            read.pseudo_critical_pa,
            p1_pa,
            p2_pa,
            allow,
        )
        if required < adjusted:
            q_low = q_mid
        else:
            q_high = q_mid
        if abs(q_high - q_low) < 1.0e-6:
            break
    return q_mid


def _nan_report(gas: bool) -> ValveSizingReport:
    return ValveSizingReport(
        kv=math.nan,
        cv=math.nan,
        cg=math.nan,
        choked=False,
        gas_valve=gas,
        calculated_molar_flow=0.0,
        calculated_outlet_pressure=math.nan,
    )


def iec_report(
    components: list[str],
    t: float,
    p: float,
    z: list[float],
    n: float,
    outlet_bara: float,
    standard: str,
    geometry: ValveSizingGeometry,
    *,
    full: bool,
) -> ValveSizingReport:
    _require_single_phase(components, t, p, z, standard)
    read = _iec_reads(components, t, p, z, n)
    opening = opening_factor(geometry.characteristic, geometry.opening_percent)
    p1_pa = read.reads.p1_bara * 1.0e5
    p2_pa = outlet_bara * 1.0e5
    if read.gas:
        raw_kv, choked = _iec_gas_kv(read, p1_pa, p2_pa, geometry.xt, geometry.allow_choked)
    else:
        # The forward passes the system Z into the Psat slot and the unset getPC() as Pc.
        raw_kv, choked = _iec_liquid_kv(
            read, read.reads.z, 0.0, p1_pa, p2_pa, geometry.allow_choked
        )
    if geometry.kv is not None:
        kv = geometry.kv
    elif full:
        kv = raw_kv
    else:
        kv = raw_kv / opening
    if not math.isfinite(kv):
        return _nan_report(gas_valve(components, t, p, z))
    adjusted = adjusted_kv(kv, geometry)
    if full:
        if read.gas:
            flow_m3s = _full_gas_flow_m3s_at(
                read, adjusted, outlet_bara, geometry.xt, geometry.allow_choked
            )
        else:
            flow_m3s = _full_liquid_flow_m3s_at(read, adjusted, outlet_bara, geometry.allow_choked)
    elif read.gas:
        flow_m3s = _iec_gas_flow_m3s(
            read, adjusted, p1_pa, p2_pa, geometry.xt, geometry.allow_choked
        )
    else:
        flow_m3s = _iec_liquid_flow_m3s(
            read,
            adjusted,
            p1_pa,
            read.pseudo_critical_pa,
            p1_pa,
            p2_pa,
            geometry.allow_choked,
        )
    molar_mass_kg = read.reads.molar_mass_gr_mol / 1000.0
    if read.gas:
        pressure_pa = _iec_gas_pressure(read, adjusted, p1_pa, geometry.xt, geometry.allow_choked)
    else:
        pressure_pa = _iec_liquid_pressure(read, adjusted, opening, geometry.allow_choked, not full)
    return ValveSizingReport(
        kv=kv,
        cv=kv * KV_TO_CV_FACTOR,
        cg=kv * KV_TO_CV_FACTOR * CG_PER_CV,
        choked=choked,
        gas_valve=gas_valve(components, t, p, z),
        calculated_molar_flow=flow_m3s * read.reads.density_kg_m3 / molar_mass_kg,
        calculated_outlet_pressure=pressure_pa / 1.0e5,
    )


def _simple_gas_pressure(read: _Reads, adjusted: float, geometry: ValveSizingGeometry) -> float:
    if adjusted <= 0.0:
        from azoth.core.errors import InvalidInputError

        raise InvalidInputError("kv", f"an adjusted Kv of {adjusted} gives no outlet pressure")
    p1_kpa = read.p1_bara * 100.0
    standard_flow = (
        read.flow_m3hr
        * (p1_kpa / STANDARD_PRESSURE_KPA)
        * (STANDARD_TEMPERATURE_K / read.t_k)
        / read.z
    )
    f_gamma = read.gamma / 1.40
    low = 0.1
    high = read.p1_bara - 0.001
    mid = 0.5 * (low + high)
    for _ in range(100):
        mid = 0.5 * (low + high)
        d_p_kpa = (read.p1_bara - mid) * 100.0
        x = d_p_kpa / p1_kpa
        choked = x >= f_gamma * geometry.xt
        x_effective = f_gamma * geometry.xt if choked and geometry.allow_choked else x
        y = max(1.0 - x_effective / (3.0 * f_gamma * geometry.xt), 2.0 / 3.0)
        if choked and geometry.allow_choked:
            calc_kv = (
                standard_flow
                / (N9 * p1_kpa * y)
                * math.sqrt(read.molar_mass_gr_mol * read.t_k * read.z / (geometry.xt * f_gamma))
                / SIMPLE_CD
            )
        else:
            calc_kv = (
                standard_flow
                / (N9 * p1_kpa * y)
                * math.sqrt(read.molar_mass_gr_mol * read.t_k * read.z / x)
                / SIMPLE_CD
            )
        if abs(calc_kv - adjusted) / adjusted < BISECTION_TOLERANCE:
            break
        if calc_kv > adjusted:
            high = mid
        else:
            low = mid
    return mid


def simple_report(
    components: list[str],
    t: float,
    p: float,
    z: list[float],
    n: float,
    outlet_bara: float,
    standard: str,
    geometry: ValveSizingGeometry,
) -> ValveSizingReport:
    _require_single_phase(components, t, p, z, standard)
    read = _reads(components, t, p, z, n)
    gas = gas_valve(components, t, p, z)
    delta_p = read.p1_bara - outlet_bara
    if geometry.kv is not None:
        kv = geometry.kv
    elif delta_p > 0.0:
        base = calc_kv(components, t, p, z, n, outlet_bara, geometry)
        kv = base / SIMPLE_CD if gas else base
    else:
        kv = math.nan
    if not math.isfinite(kv):
        return _nan_report(gas)
    adjusted = adjusted_kv(kv, geometry)
    choked = gas and (delta_p / read.p1_bara) >= (read.gamma / 1.40) * geometry.xt
    if gas:
        effective = adjusted * SIMPLE_CD
        p1_kpa = read.p1_bara * 100.0
        x = delta_p / read.p1_bara
        f_gamma = read.gamma / 1.40
        is_choked = x >= f_gamma * geometry.xt
        x_effective = f_gamma * geometry.xt if is_choked and geometry.allow_choked else x
        y = max(1.0 - x_effective / (3.0 * f_gamma * geometry.xt), 2.0 / 3.0)
        x_for_scale = geometry.xt * f_gamma if is_choked and geometry.allow_choked else x
        standard_flow = (
            effective
            * N9
            * p1_kpa
            * y
            / math.sqrt(read.molar_mass_gr_mol * read.t_k * read.z / x_for_scale)
        )
        actual = (
            standard_flow
            * (STANDARD_PRESSURE_KPA / p1_kpa)
            * (read.t_k / STANDARD_TEMPERATURE_K)
            * read.z
        )
        flow_m3s = actual / 3600.0
        pressure_bara = _simple_gas_pressure(read, adjusted, geometry)
    else:
        flow_m3s = adjusted * math.sqrt(delta_p / (read.density_kg_m3 / 1000.0)) / 3600.0
        pressure_bara = calculated_outlet_pressure(components, t, p, z, n, adjusted, geometry)
    return ValveSizingReport(
        kv=kv,
        cv=kv * KV_TO_CV_FACTOR,
        cg=kv * KV_TO_CV_FACTOR * CG_PER_CV,
        choked=choked,
        gas_valve=gas,
        calculated_molar_flow=flow_m3s * read.density_kg_m3 / (read.molar_mass_gr_mol / 1000.0),
        calculated_outlet_pressure=pressure_bara,
    )


def sizing_report_for(
    components: list[str],
    t: float,
    p: float,
    z: list[float],
    n: float,
    outlet_bara: float,
    standard: str,
    geometry: ValveSizingGeometry,
) -> ValveSizingReport:
    """The report for a selected ``setValveSizingStandard`` value."""
    if standard == "default":
        _require_single_phase(components, t, p, z, standard)
        return sizing_report(components, t, p, z, n, outlet_bara, geometry)
    if standard == "iec_60534":
        return iec_report(components, t, p, z, n, outlet_bara, standard, geometry, full=False)
    if standard == "iec_60534_full":
        return iec_report(components, t, p, z, n, outlet_bara, standard, geometry, full=True)
    if standard == "prod_choke":
        return simple_report(components, t, p, z, n, outlet_bara, standard, geometry)
    from azoth.core.errors import InvalidInputError

    raise InvalidInputError("sizing_standard", "no sizing report is built for this route")
