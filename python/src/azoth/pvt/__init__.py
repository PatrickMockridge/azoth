"""PVT simulation.

NeqSim's ``pvtsimulation/`` without its ``flowassurance/``: the black-oil and gas-water
correlation tables, the decline and material-balance arithmetic, and the laboratory
experiments a reservoir fluid is described by. The namespace is its own rather than a family
under :mod:`azoth.eos` or :mod:`azoth.process`, mirroring NeqSim's package split: a PVT
experiment is a sequence of flashes and property reads, not an equation of state and not a
unit operation.

The correlation ids are scalar record lists; the laboratory ids take a fluid as
:class:`azoth.characterization.FluidComponent` rows and publish one row per pressure,
temperature or separator stage. Both languages build the fluid through
:func:`azoth.characterization.mixture_from_characterization`, so a pseudo-component crosses
the same way an assay cut does.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any

from azoth._dispatch import resolve
from azoth.core.result import (
    BlackOilCorrelationsResult,
    ConstantMassExpansionResult,
    ConstantVolumeDepletionResult,
    DeclineCurveResult,
    DifferentialLiberationResult,
    FetkovichAquiferResult,
    GasCorrelationsResult,
    GasMaterialBalanceResult,
    MmpResult,
    OilMaterialBalanceResult,
    PropertySweepResult,
    RegressionResult,
    SaturationTestResult,
    SeparatorTestResult,
    SwellingTestResult,
    TuneToSaturationResult,
    VanEverdingenHurstAquiferResult,
    WaterCorrelationsResult,
)
from azoth.pvt import layers as layers

_BLACK_OIL_CORRELATIONS = "pvt.black_oil_correlations"
_CONSTANT_MASS_EXPANSION = "pvt.constant_mass_expansion"
_CONSTANT_VOLUME_DEPLETION = "pvt.constant_volume_depletion"
_DECLINE_CURVE = "pvt.decline_curve"
_DIFFERENTIAL_LIBERATION = "pvt.differential_liberation"
_FETKOVICH_AQUIFER = "pvt.fetkovich_aquifer"
_GAS_CORRELATIONS = "pvt.gas_correlations"
_GAS_MATERIAL_BALANCE = "pvt.gas_material_balance"
_MMP = "pvt.mmp"
_OIL_MATERIAL_BALANCE = "pvt.oil_material_balance"
_PROPERTY_SWEEP = "pvt.property_sweep"
_REGRESSION = "pvt.regression"
_SATURATION_TEST = "pvt.saturation_test"
_SEPARATOR_TEST = "pvt.separator_test"
_SWELLING_TEST = "pvt.swelling_test"
_TUNE_TO_SATURATION = "pvt.tune_to_saturation"
_VAN_EVERDINGEN_HURST_AQUIFER = "pvt.van_everdingen_hurst_aquifer"
_WATER_CORRELATIONS = "pvt.water_correlations"

__all__ = [
    "black_oil_correlations",
    "constant_mass_expansion",
    "constant_volume_depletion",
    "decline_curve",
    "differential_liberation",
    "fetkovich_aquifer",
    "gas_correlations",
    "gas_material_balance",
    "layers",
    "mmp",
    "oil_material_balance",
    "property_sweep",
    "regression",
    "saturation_test",
    "separator_test",
    "swelling_test",
    "tune_to_saturation",
    "van_everdingen_hurst_aquifer",
    "water_correlations",
]


def black_oil_correlations(
    queries: Sequence[Mapping[str, Any]],
) -> BlackOilCorrelationsResult:
    """The black-oil correlation table over the stated calls.

    ``queries`` is one record per correlation call, keyed by the field names the spec
    declares: ``calculation`` selects the answer, and the row states only the inputs that
    call reads. A missing required field, a non-positive value where the correlation
    divides by it or takes its logarithm, and a non-finite answer are refused.

    The SI boundary is the id's: a solution gas-oil ratio in m3/m3, a temperature in K, a
    pressure in Pa and a viscosity in Pa*s. The two classes convert with different
    literals; the ``_p_sat`` calculations use ``SaturationPressureCorrelation``'s and the
    others ``BlackOilCorrelations``'.

    Returns:
        One row per input row, with only the fields the calculation answers.

    Raises:
        InvalidInputError: for a missing field, a non-positive input or a non-finite answer.

    See :func:`azoth.pvt.reference.black_oil_correlations.black_oil_correlations`.
    """
    return resolve(_BLACK_OIL_CORRELATIONS)(  # type: ignore[no-any-return]
        queries=queries,
    )


def differential_liberation(
    components: Sequence[Mapping[str, Any]],
    *,
    pressures: Sequence[Any],
    eos: str | None = None,
    mixing_rule: str | None = None,
    temperature: Any = None,
    volume_correction: bool | None = None,
) -> DifferentialLiberationResult:
    """The differential liberation of a fluid over a pressure schedule.

    ``components`` is the characterization row list; the temperature and pressures are
    absolute. The gas phase is removed after every two-phase step, and the retained liquid's
    final 288.15 K / 1.01325 bar flash gives the stock-tank oil volume every Bo and Rs is
    measured against. A single-phase point leaves its gas fields and its liquid relative
    volume absent.

    Returns:
        The saturation scalars, the stock-tank volume and one ``points`` row per pressure.

    Raises:
        InvalidInputError: for an unknown ``eos`` or ``mixing_rule``, no pressures, or a
            non-positive temperature or pressure.

    See :func:`azoth.pvt.reference.differential_liberation.differential_liberation`.
    """
    return resolve(_DIFFERENTIAL_LIBERATION)(  # type: ignore[no-any-return]
        components=components,
        temperature=temperature,
        pressures=pressures,
        eos=eos,
        mixing_rule=mixing_rule,
        volume_correction=volume_correction,
    )


def constant_volume_depletion(
    components: Sequence[Mapping[str, Any]],
    *,
    pressures: Sequence[Any],
    eos: str | None = None,
    mixing_rule: str | None = None,
    temperature: Any = None,
    volume_correction: bool | None = None,
) -> ConstantVolumeDepletionResult:
    """The constant-volume depletion of a fluid over a pressure schedule.

    ``components`` is the characterization row list; the temperature and pressures are
    absolute. The saturation state is an upward scan on the feed. Each point whose volume
    exceeds the saturation volume has gas removed until it is restored, and the cumulative
    removed moles, the material-balance residual, Zmix, Zgas and the liquid dropout are
    reported per point. The K-values are the per-step equilibrium, not the class's QC method.

    Returns:
        The saturation scalars and one ``points`` row per pressure.

    Raises:
        InvalidInputError: for an unknown ``eos`` or ``mixing_rule``, no pressures, or a
            restore loop that overshoots, finds no gas, or fails to converge.

    See :func:`azoth.pvt.reference.constant_volume_depletion.constant_volume_depletion`.
    """
    return resolve(_CONSTANT_VOLUME_DEPLETION)(  # type: ignore[no-any-return]
        components=components,
        temperature=temperature,
        pressures=pressures,
        eos=eos,
        mixing_rule=mixing_rule,
        volume_correction=volume_correction,
    )


def separator_test(
    components: Sequence[Mapping[str, Any]],
    mode: str,
    *,
    stages: Sequence[Mapping[str, Any]],
    eos: str | None = None,
    mixing_rule: str | None = None,
    volume_correction: bool | None = None,
    reservoir_pressure: Any = None,
    reservoir_temperature: Any = None,
    optimize_first_stage: bool | None = None,
    first_stage_pressure_min: Any = None,
    first_stage_pressure_max: Any = None,
    first_stage_pressure_steps: Any = None,
    first_stage_temperature_min: Any = None,
    first_stage_temperature_max: Any = None,
    first_stage_temperature_steps: Any = None,
) -> SeparatorTestResult:
    """The separator experiment over the stated stages.

    ``mode`` is ``"continuous"`` (the GOR class, every stage reflashing the full feed),
    ``"staged"`` (the gas leaves and the liquid cascades) or ``"train"`` (the multi-stage
    train, which needs the reservoir conditions and may optimise its first stage over a
    pressure/temperature grid). ``stages`` is the stage record list, each with a pressure in
    Pa and a temperature in K.

    Returns:
        One ``stages`` row per stage, and in the train the total GOR, Bo, Rs, stock-tank
        density and API - plus the optimum's fields when the optimisation ran.

    Raises:
        InvalidInputError: for an unknown ``mode``, no stages, a non-positive pressure or
            temperature, a missing reservoir condition, or a bad optimisation grid.

    See :func:`azoth.pvt.reference.separator_test.separator_test`.
    """
    return resolve(_SEPARATOR_TEST)(  # type: ignore[no-any-return]
        components=components,
        mode=mode,
        stages=stages,
        eos=eos,
        mixing_rule=mixing_rule,
        volume_correction=volume_correction,
        reservoir_pressure=reservoir_pressure,
        reservoir_temperature=reservoir_temperature,
        optimize_first_stage=optimize_first_stage,
        first_stage_pressure_min=first_stage_pressure_min,
        first_stage_pressure_max=first_stage_pressure_max,
        first_stage_pressure_steps=first_stage_pressure_steps,
        first_stage_temperature_min=first_stage_temperature_min,
        first_stage_temperature_max=first_stage_temperature_max,
        first_stage_temperature_steps=first_stage_temperature_steps,
    )


def regression(
    components: Sequence[Mapping[str, Any]],
    parameters: Sequence[Mapping[str, Any]],
    *,
    cce: Sequence[Mapping[str, Any]] | None = None,
    cvd: Sequence[Mapping[str, Any]] | None = None,
    dle: Sequence[Mapping[str, Any]] | None = None,
    viscosity: Sequence[Mapping[str, Any]] | None = None,
    cce_weight: float | None = None,
    cvd_weight: float | None = None,
    dle_weight: float | None = None,
    viscosity_weight: float | None = None,
    eos: str | None = None,
    mixing_rule: str | None = None,
    volume_correction: bool | None = None,
    max_iterations: int | None = None,
) -> RegressionResult:
    """The PVT multi-target regression over a characterised fluid.

    ``components`` is the fluid's row list, ``parameters`` the fitting constants with their
    bounds and guesses, and the four experiment lists the measured points grouped by an
    ``experiment`` index. The fit minimises a weighted chi-square and publishes the class's
    second, unweighted objective ledger beside it.

    Returns:
        One row per fitted parameter, the standard errors and 95 % half-widths, one
        objective row per experiment, and the optimizer's chi-square, dof, AAD, iterations
        and exit reason.

    Raises:
        InvalidInputError: for an unknown parameter or phase name, a malformed experiment
            index, or an experiment whose rows disagree on their temperature.

    See :func:`azoth.pvt.reference.regression.fit`.
    """
    return resolve(_REGRESSION)(  # type: ignore[no-any-return]
        components=components,
        parameters=parameters,
        cce=cce,
        cvd=cvd,
        dle=dle,
        viscosity=viscosity,
        cce_weight=cce_weight,
        cvd_weight=cvd_weight,
        dle_weight=dle_weight,
        viscosity_weight=viscosity_weight,
        eos=eos,
        mixing_rule=mixing_rule,
        volume_correction=volume_correction,
        max_iterations=max_iterations,
    )


def swelling_test(
    components: Sequence[Mapping[str, Any]],
    *,
    cumulative_gas_injected: Sequence[Any],
    injection_gas: Sequence[Mapping[str, Any]] | None = None,
    eos: str | None = None,
    mixing_rule: str | None = None,
    temperature: Any = None,
    volume_correction: bool | None = None,
) -> SwellingTestResult:
    """The swelling of an oil by an injection gas.

    ``components`` is the base oil's row list and ``injection_gas`` the gas's; the schedule
    states the cumulative injection as fractions of the original oil moles. At each entry the
    mixture's bubble point is solved and its volume is reported against the base oil's.

    Returns:
        The base oil's amount and bubble-point volume, and one ``points`` row per entry.

    Raises:
        InvalidInputError: for an empty, negative or decreasing schedule, an injection without
            a gas, a non-positive temperature, or a failed bubble point.

    See :func:`azoth.pvt.reference.swelling_test.swelling_test`.
    """
    return resolve(_SWELLING_TEST)(  # type: ignore[no-any-return]
        components=components,
        injection_gas=injection_gas,
        eos=eos,
        mixing_rule=mixing_rule,
        temperature=temperature,
        cumulative_gas_injected=cumulative_gas_injected,
        volume_correction=volume_correction,
    )


def tune_to_saturation(
    components: Sequence[Mapping[str, Any]],
    plus_component: str,
    *,
    saturation_temperature: Any,
    saturation_pressure: Any,
    plus_model: str | None = None,
    number_of_lumps: int | None = None,
    eos: str | None = None,
    mixing_rule: str | None = None,
    volume_correction: bool | None = None,
    max_iterations: int | None = None,
) -> TuneToSaturationResult:
    """Tune a plus fraction's molar mass until its saturation pressure meets a target.

    Each iteration re-characterises the plus end through the split and its lumps, rebuilds
    the fluid and runs the saturation scan. The class steps the mass by ``dp/1000`` (bar) and
    flips the sign when the error grows.

    Returns:
        The tuned molar mass, the last saturation pressure, the steps taken, and whether the
        loop exited on its own tolerance rather than the cap.

    Raises:
        InvalidInputError: for a plus name the rows do not carry, a plus row that is not a
            pseudo row, another pseudo row present, or a name without a carbon number.

    See :func:`azoth.pvt.reference.tune_to_saturation.tune_to_saturation`.
    """
    return resolve(_TUNE_TO_SATURATION)(  # type: ignore[no-any-return]
        components=components,
        plus_component=plus_component,
        saturation_temperature=saturation_temperature,
        saturation_pressure=saturation_pressure,
        plus_model=plus_model,
        number_of_lumps=number_of_lumps,
        eos=eos,
        mixing_rule=mixing_rule,
        volume_correction=volume_correction,
        max_iterations=max_iterations,
    )


def mmp(
    oil_components: Sequence[Mapping[str, Any]],
    gas_components: Sequence[Mapping[str, Any]],
    *,
    temperature: Any,
    eos: str | None = None,
    mixing_rule: str | None = None,
    method: str | None = None,
    recovery_threshold: float | None = None,
    number_of_pressure_points: int | None = None,
    min_pressure: Any = None,
    max_pressure: Any = None,
    slim_tube_nodes: int | None = None,
    pv_injected: float | None = None,
) -> MmpResult:
    """The minimum miscibility pressure of an oil and an injection gas.

    ``oil_components`` and ``gas_components`` are the characterization row lists; every gas
    component must be one the oil carries, because the class injects only the components a
    cell already has. ``method`` is ``"slim_tube"`` (the default), ``"key_tie_line"`` or
    ``"rising_bubble"``, which the class delegates to the slim tube.

    Returns:
        The MMP (absent where the range never reaches the threshold), the mechanism, and the
        slim tube's ``points`` recovery curve.

    Raises:
        InvalidInputError: for a gas component the oil does not carry, an unknown method,
            fewer than three pressure points, a bad grid, or a non-positive temperature.

    See :func:`azoth.pvt.reference.mmp.mmp`.
    """
    return resolve(_MMP)(  # type: ignore[no-any-return]
        oil_components=oil_components,
        gas_components=gas_components,
        temperature=temperature,
        eos=eos,
        mixing_rule=mixing_rule,
        method=method,
        recovery_threshold=recovery_threshold,
        number_of_pressure_points=number_of_pressure_points,
        min_pressure=min_pressure,
        max_pressure=max_pressure,
        slim_tube_nodes=slim_tube_nodes,
        pv_injected=pv_injected,
    )


def property_sweep(
    components: Sequence[Mapping[str, Any]],
    mode: str,
    *,
    temperatures: Sequence[Any],
    pressures: Sequence[Any],
    eos: str | None = None,
    mixing_rule: str | None = None,
    volume_correction: bool | None = None,
) -> PropertySweepResult:
    """A density or viscosity sweep over a grid of states.

    ``mode`` is ``"density"`` (the phase densities) or ``"viscosity"`` (the phase viscosities
    and the availability flags). ``temperatures`` and ``pressures`` are parallel arrays of
    absolute states. **The wax sweeps are owed** with NeqSim's TBP wax model, which a
    characterization row cannot express.

    Returns:
        One ``points`` row per state.

    Raises:
        InvalidInputError: for an unknown mode, ``eos`` or ``mixing_rule``, empty or unequal
            arrays, a non-positive temperature or pressure, or a wax mode.

    See :func:`azoth.pvt.reference.property_sweep.property_sweep`.
    """
    return resolve(_PROPERTY_SWEEP)(  # type: ignore[no-any-return]
        components=components,
        mode=mode,
        temperatures=temperatures,
        pressures=pressures,
        eos=eos,
        mixing_rule=mixing_rule,
        volume_correction=volume_correction,
    )


def gas_correlations(
    queries: Sequence[Mapping[str, Any]],
) -> GasCorrelationsResult:
    """The gas correlation table over the stated calls.

    ``queries`` states the call and only the inputs it reads: a gas specific gravity, the
    acid-gas mole fractions, an absolute temperature in K, a pressure in Pa and a molar mass
    in kg/mol. Pseudo-critical properties answer K and Pa, the Z factors are dimensionless,
    the density is kg/m3, and the pseudo-pressure is the class's bara^2/cP.

    Returns:
        One row per input row, with only the fields the calculation answers.

    Raises:
        InvalidInputError: for a missing field, a gas gravity outside the correlation's
            range, a reduced state Hall-Yarborough refuses, or a non-finite answer.

    See :func:`azoth.pvt.reference.gas_correlations.gas_correlations`.
    """
    return resolve(_GAS_CORRELATIONS)(  # type: ignore[no-any-return]
        queries=queries,
    )


def water_correlations(
    queries: Sequence[Mapping[str, Any]],
) -> WaterCorrelationsResult:
    """The water and brine correlation table over the stated calls.

    ``queries`` states the call and only the inputs it reads. The salinity convention is the
    calculation's own: ppm (mg/L) for the water properties, a weight fraction for the
    Batzle-Wang density, and mol/kg water for the McCain Rsw. Viscosity answers Pa*s from
    the class's cP, compressibility 1/Pa from its 1/bara, and surface tension N/m from its
    mN/m.

    Returns:
        One row per input row, with only the fields the calculation answers.

    Raises:
        InvalidInputError: for a missing field, a non-positive temperature or pressure, a
            negative salinity, a weight fraction outside [0, 0.3], or a non-finite answer.

    See :func:`azoth.pvt.reference.water_correlations.water_correlations`.
    """
    return resolve(_WATER_CORRELATIONS)(  # type: ignore[no-any-return]
        queries=queries,
    )


def constant_mass_expansion(
    components: Sequence[Mapping[str, Any]],
    *,
    pressures: Sequence[Any],
    eos: str | None = None,
    mixing_rule: str | None = None,
    temperature: Any = None,
    volume_correction: bool | None = None,
) -> ConstantMassExpansionResult:
    """The constant-mass expansion of a fluid over a pressure schedule.

    ``components`` is the characterization row list; the temperature is absolute and the
    pressures absolute. The saturation state is the class's upward scan's last bisection
    midpoint, and the schedule is reported in its own order. The saturation and total volumes
    are untranslated; the phase densities and the gas volume are translated where
    ``volume_correction`` is not false. A two-phase point carries ``liquid_relative_volume``
    and ``y_factor`` and leaves ``viscosity`` and ``isothermal_compressibility`` absent; a
    single-phase point does the reverse.

    Returns:
        The saturation scalars and one ``points`` row per pressure.

    Raises:
        InvalidInputError: for an unknown ``eos`` or ``mixing_rule``, no pressures, or a
            non-positive temperature or pressure.

    See :func:`azoth.pvt.reference.constant_mass_expansion.constant_mass_expansion`.
    """
    return resolve(_CONSTANT_MASS_EXPANSION)(  # type: ignore[no-any-return]
        components=components,
        temperature=temperature,
        pressures=pressures,
        eos=eos,
        mixing_rule=mixing_rule,
        volume_correction=volume_correction,
    )


def decline_curve(
    calculation: str,
    *,
    qi: float | None = None,
    di: float | None = None,
    b: float | None = None,
    t: float | None = None,
    q_limit: float | None = None,
    d_nominal: float | None = None,
    d_effective: float | None = None,
    q1: float | None = None,
    t1: float | None = None,
    q2: float | None = None,
    t2: float | None = None,
    q3: float | None = None,
    t3: float | None = None,
    t_start: float | None = None,
    t_end: float | None = None,
    t_step: float | None = None,
    t_current: float | None = None,
    a: float | None = None,
    m: float | None = None,
    start_index: float | None = None,
    end_index: float | None = None,
    history_time: Sequence[float] | None = None,
    history_rate: Sequence[float] | None = None,
) -> DeclineCurveResult:
    """The decline-curve calculation over the stated inputs.

    Rates, times and cumulative volumes are unit-agnostic, the class's own statement; the
    day is the time unit and 365.25 days the year. ``calculation`` selects the answer, and
    only the inputs it reads are required. A zero decline leaves the time to the economic
    limit and the EUR absent, with a warning, rather than publishing an infinity.

    Returns:
        One result with the fields the calculation answers; the others are absent.

    Raises:
        InvalidInputError: for an unknown calculation, a missing required scalar, a
            non-positive rate where a fit needs one, a bad window, fewer than three
            history points, ``t2 <= t1``, a non-positive time step, or a non-finite answer.

    See :func:`azoth.pvt.reference.decline_curve.decline_curve`.
    """
    return resolve(_DECLINE_CURVE)(  # type: ignore[no-any-return]
        calculation=calculation,
        qi=qi,
        di=di,
        b=b,
        t=t,
        q_limit=q_limit,
        d_nominal=d_nominal,
        d_effective=d_effective,
        q1=q1,
        t1=t1,
        q2=q2,
        t2=t2,
        q3=q3,
        t3=t3,
        t_start=t_start,
        t_end=t_end,
        t_step=t_step,
        t_current=t_current,
        a=a,
        m=m,
        start_index=start_index,
        end_index=end_index,
        history_time=history_time,
        history_rate=history_rate,
    )


def gas_material_balance(
    calculation: str,
    *,
    pressure: Sequence[Any] | None = None,
    z: Sequence[float] | None = None,
    gp: Sequence[float] | None = None,
    we: Sequence[float] | None = None,
    pressure_value: Any = None,
    z_value: float | None = None,
    temperature: Any = None,
    gas_gravity: float | None = None,
) -> GasMaterialBalanceResult:
    """The gas material balance over the stated history.

    ``pressure`` is an absolute pressure and ``temperature`` an absolute temperature; the
    p/Z quantities are bar, the gas FVF a reservoir/surface ratio, and OGIP is in the
    cumulative gas unit. A p/Z line that does not decline leaves the OGIP absent with a
    warning rather than publishing an infinity.

    Returns:
        The published fields; the others are absent.

    Raises:
        InvalidInputError: for a missing input, arrays of different lengths or fewer than
            two points, a non-positive pressure, Z or temperature, a singular regression, a
            non-positive Cole expansion, or a non-finite answer.

    See :func:`azoth.pvt.reference.gas_material_balance.gas_material_balance`.
    """
    return resolve(_GAS_MATERIAL_BALANCE)(  # type: ignore[no-any-return]
        calculation=calculation,
        pressure=pressure,
        z=z,
        gp=gp,
        we=we,
        pressure_value=pressure_value,
        z_value=z_value,
        temperature=temperature,
        gas_gravity=gas_gravity,
    )


def saturation_test(
    components: Sequence[Mapping[str, Any]],
    mode: str,
    *,
    eos: str | None = None,
    mixing_rule: str | None = None,
    temperature: Any = None,
    pressure: Any = None,
    temperature_search_min: Any = None,
    temperature_search_max: Any = None,
) -> SaturationTestResult:
    """The upper saturation boundary of a fluid.

    ``components`` is the characterization row list the fluid is built from, the same rows
    :func:`azoth.characterization.mixture_from_characterization` takes. ``mode`` is
    ``"pressure"``, which needs ``temperature``, or ``"temperature"``, which needs
    ``pressure`` and accepts a pair of ``temperature_search_*`` bounds inside ``[30, 1200]``
    K. The search is the classes' descending grid for the uppermost two-phase crossing; an
    interval that brackets none returns the grid top with an ``OUT_OF_VALID_RANGE`` warning
    rather than refusing.

    Returns:
        ``saturation_pressure`` in pressure mode or ``saturation_temperature`` in temperature
        mode; the other is absent.

    Raises:
        InvalidInputError: for an unknown mode, ``eos`` or ``mixing_rule``, a missing fixed
            state, one bound without the other, or bounds outside the class grid.

    See :func:`azoth.pvt.reference.saturation_test.saturation_test`.
    """
    return resolve(_SATURATION_TEST)(  # type: ignore[no-any-return]
        components=components,
        mode=mode,
        eos=eos,
        mixing_rule=mixing_rule,
        temperature=temperature,
        pressure=pressure,
        temperature_search_min=temperature_search_min,
        temperature_search_max=temperature_search_max,
    )


def oil_material_balance(
    calculation: str,
    *,
    np: float | None = None,
    rp: float | None = None,
    bo: float | None = None,
    rs: float | None = None,
    bg: float | None = None,
    wp: float | None = None,
    bw: float | None = None,
    boi: float | None = None,
    rsi: float | None = None,
    bgi: float | None = None,
    cw: float | None = None,
    swi: float | None = None,
    cf: float | None = None,
    delta_p: float | None = None,
    m: float | None = None,
    n: float | None = None,
    eo_term: float | None = None,
    eg_term: float | None = None,
    efw_term: float | None = None,
    we_value: float | None = None,
    f_value: float | None = None,
    f_history: Sequence[float] | None = None,
    eo_history: Sequence[float] | None = None,
    eg_history: Sequence[float] | None = None,
    we_history: Sequence[float] | None = None,
) -> OilMaterialBalanceResult:
    """The oil material balance over the stated history.

    Every term is unit-consistent and crosses as a dimensionless magnitude. The four fits
    are the class's: depletion and water drive through the origin/linear fit, gas cap with
    the two-predictor normal equations.

    Returns:
        The published fields; the others are absent.

    Raises:
        InvalidInputError: for a missing scalar, a history of different lengths or fewer
            than two points, a zero Eo in the water-drive plot, a water saturation at or
            above one, a singular regression, or a non-finite answer.

    See :func:`azoth.pvt.reference.oil_material_balance.oil_material_balance`.
    """
    return resolve(_OIL_MATERIAL_BALANCE)(  # type: ignore[no-any-return]
        calculation=calculation,
        np=np,
        rp=rp,
        bo=bo,
        rs=rs,
        bg=bg,
        wp=wp,
        bw=bw,
        boi=boi,
        rsi=rsi,
        bgi=bgi,
        cw=cw,
        swi=swi,
        cf=cf,
        delta_p=delta_p,
        m=m,
        n=n,
        eo_term=eo_term,
        eg_term=eg_term,
        efw_term=efw_term,
        we_value=we_value,
        f_value=f_value,
        f_history=f_history,
        eo_history=eo_history,
        eg_history=eg_history,
        we_history=we_history,
    )


def van_everdingen_hurst_aquifer(
    calculation: str,
    *,
    dimensionless_time_value: float | None = None,
    dimensionless_times: Sequence[float] | None = None,
    dimensionless_outer_radius: float | None = None,
    permeability: Any = None,
    time: Any = None,
    porosity: float | None = None,
    viscosity: Any = None,
    total_compressibility: float | None = None,
    reservoir_radius: Any = None,
    thickness: Any = None,
    encroachment_angle: float | None = None,
    aquifer_constant_value: float | None = None,
    pressure_drop: Sequence[Any] | None = None,
) -> VanEverdingenHurstAquiferResult:
    """The radial aquifer influence functions, time and constant, and water influx.

    ``calculation`` selects the answer: the dimensionless pressure or its derivative at a
    time, the physical dimensionless time or aquifer constant from the aquifer's properties,
    the Carter-Tracy cumulative influx over a history, or the influence table. ``permeability``
    is an area, ``time`` a duration, ``viscosity`` a dynamic viscosity, the two radii and the
    thickness lengths, and ``pressure_drop`` a pressure history. The aquifer constant is in
    reservoir m3 per Pa, because ``total_compressibility`` is 1/Pa throughout.

    ``dimensionless_outer_radius`` absent, non-positive or infinite is infinite-acting; a
    finite radius selects the class's legacy bounded policy, the maximum of the infinite-acting
    pressure and its pseudo-steady form. A negative total pressure drop sets a Carter-Tracy
    increment to zero rather than creating a water credit.

    Returns:
        The dimensionless pressures, time and constant, or the cumulative influx history; the
        fields a calculation does not answer are absent.

    Raises:
        InvalidInputError: for an unknown calculation, a missing input, a physical input outside
            its domain, a non-increasing Carter-Tracy history or an unrepresentable answer.

    See :func:`azoth.pvt.reference.van_everdingen_hurst_aquifer.van_everdingen_hurst_aquifer`.
    """
    return resolve(_VAN_EVERDINGEN_HURST_AQUIFER)(  # type: ignore[no-any-return]
        calculation=calculation,
        dimensionless_time_value=dimensionless_time_value,
        dimensionless_times=dimensionless_times,
        dimensionless_outer_radius=dimensionless_outer_radius,
        permeability=permeability,
        time=time,
        porosity=porosity,
        viscosity=viscosity,
        total_compressibility=total_compressibility,
        reservoir_radius=reservoir_radius,
        thickness=thickness,
        encroachment_angle=encroachment_angle,
        aquifer_constant_value=aquifer_constant_value,
        pressure_drop=pressure_drop,
    )


def fetkovich_aquifer(
    total_compressibility: float,
    productivity: float,
    initial_pressure: Any,
    intervals: Sequence[Mapping[str, Any]],
    *,
    initial_water_volume: Any = None,
) -> FetkovichAquiferResult:
    """The finite, constant-property water aquifer over a prescribed interval schedule.

    ``intervals`` is one row per prescribed linear reservoir-pressure interval, with
    ``duration``, ``reservoir_pressure_start`` and ``reservoir_pressure_end``. Each interval
    is integrated exactly; the cumulative-influx minimum and maximum are sampled at any
    within-interval flow reversal. ``initial_water_volume`` absent is infinite storage; a
    present zero is zero capacity. The capacitance is absent exactly when storage is infinite.

    Returns:
        The capacitance and the initial state followed by one state per interval.

    Raises:
        InvalidInputError: for a negative or NaN coefficient, a non-positive pressure or
            duration, an unrepresentable finite capacity, or an interval that exhausts the
            finite water inventory.

    See :func:`azoth.pvt.reference.fetkovich_aquifer.fetkovich_aquifer`.
    """
    return resolve(_FETKOVICH_AQUIFER)(  # type: ignore[no-any-return]
        initial_water_volume=initial_water_volume,
        total_compressibility=total_compressibility,
        productivity=productivity,
        initial_pressure=initial_pressure,
        intervals=intervals,
    )
