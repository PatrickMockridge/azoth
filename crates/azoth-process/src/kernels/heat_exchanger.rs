//! `unit_ops.heat_exchanger` - two streams, coupled by an overall conductance.

use azoth_core::units::{
    Energy, MolarEnergy, Power, ThermalConductance, ThermodynamicTemperature, joules,
    joules_per_mole, kelvins, pascals, watts, watts_per_kelvin,
};
use azoth_core::{AzothError, Result};
use azoth_eos::{Phase, pt_flash};
use azoth_thermal::{RatingRequest, ShellSideMethod, rate as thermal_rate};

use crate::segment::phase::single_phase_view;
use crate::stream::Stream;

/// The flow arrangements `HeatExchanger` names, in azoth's own spelling.
///
/// NeqSim's are `"concentric tube counterflow"`, `"concentric tube paralellflow"` - its own
/// misspelling - and `"shell and tube"`, and its default is the first. An arrangement it
/// does not recognise takes the counterflow relation through `run`'s `else`; **this refuses
/// one instead**, because a typo that silently becomes counterflow is a plausible answer to
/// a question nobody asked.
pub const FLOW_ARRANGEMENTS: [&str; 3] = ["counterflow", "parallelflow", "shell_and_tube"];

/// Which side of the exchanger a stream is. Only the `outTemperature` mode needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Hot,
    Cold,
}

impl Side {
    fn other(self) -> Side {
        match self {
            Side::Hot => Side::Cold,
            Side::Cold => Side::Hot,
        }
    }
}

/// The numbers the effectiveness-NTU rating sized the exchanger by.
///
/// **Present only when a rating was asked for**, which is the default mode and not the only one:
/// a run handed a pinned outlet temperature energy-balances the other side instead, and reaches
/// the duty without any of these. That is why [`HeatExchangerOutcome::rating`] is an `Option` and
/// why the record writes `null` for them - the absence of a rating, not a rating of zero.
pub struct Rating {
    /// `UA / C_min`, dimensionless.
    pub ntu: f64,
    /// The swing the relation scaled, as a fraction of the capacity-limited side's.
    pub effectiveness: f64,
    /// The smaller of the two estimated capacities, W/K.
    pub c_min: ThermalConductance,
    /// The larger of them, W/K.
    pub c_max: ThermalConductance,
    /// `C_min / C_max`, dimensionless.
    pub capacity_ratio: f64,
}

/// What an exchanger did: both outlet records, the duty, and the rating where there was one.
pub struct HeatExchangerOutcome {
    /// The hot outlet record.
    pub hot_out: Stream,
    /// The cold outlet record.
    pub cold_out: Stream,
    /// The heat the hot side released, W - positive when the hot side cools, which is what the
    /// cold side's gain balances. **The same number either mode reaches**, and the one figure a
    /// faceplate shows whatever the exchanger was told to do.
    pub duty: Power,
    /// The rating's own numbers, or `None` where one outlet was pinned instead.
    pub rating: Option<Rating>,
    /// `runDeltaT`'s computed conductance, W/K; present only in that mode.
    pub ua_value: Option<f64>,
    /// Which side `runDeltaT` set, `"hot"` or `"cold"`; present only in that mode.
    pub delta_t_side: Option<&'static str>,
    /// The thermal-hydraulic rating's calculator quantities; present only in that mode.
    pub rating_diagnostics: Option<RatingDiagnostics>,
    /// The carried state after a `runTransient` step; absent on every steady branch.
    pub transient: Option<TransientState>,
}

/// The wall and holdup temperatures a dynamic step hands to the next one.
#[derive(Debug, Clone, Copy)]
pub struct TransientState {
    /// The post-step metal-wall temperature, K.
    pub wall_temperature: ThermodynamicTemperature,
    /// The shell-side fluid temperature the step solved, K.
    pub shell_fluid_temperature: Option<ThermodynamicTemperature>,
    /// The tube-side fluid temperature the step solved, K.
    pub tube_fluid_temperature: Option<ThermodynamicTemperature>,
    /// The post-step fluid-plus-wall thermal inventory, J.
    pub thermal_inventory_energy: Energy,
    /// The interval's inventory change, J.
    pub balance_accumulation: Energy,
    /// The interval's boundary energy, J.
    pub balance_boundary_energy: Energy,
    /// The interval's first-law residual, J.
    pub balance_residual: Energy,
}

/// One `HeatExchanger.runTransient` step's settings, in SI.
///
/// The class keeps these as fields and the one-step id carries them as inputs; the three
/// temperatures in [`TransientState`] are the ones the next call feeds back. The gate that
/// selects the dynamic branch (`dynamicModelEnabled`, `wallMass > 0`, `heatTransferArea > 0`)
/// is the model layer's, so this struct always describes the **active** branch.
#[derive(Debug, Clone, Copy)]
pub struct TransientSettings {
    /// The forward-Euler interval, s.
    pub time_step_s: f64,
    /// The metal wall's mass, kg.
    pub wall_mass_kg: f64,
    /// The metal wall's specific heat, J/(kg·K).
    pub wall_cp_j_per_kg_k: f64,
    /// The carried wall temperature; absent or NaN seeds from the two inlet temperatures.
    pub wall_temperature_k: Option<f64>,
    /// The shell-side fluid holdup, m³; zero leaves the side at its inlet temperature.
    pub shell_holdup_volume_m3: f64,
    /// The tube-side fluid holdup, m³.
    pub tube_holdup_volume_m3: f64,
    /// The shell-side film coefficient, W/(m²·K).
    pub shell_side_htc_w_per_m2_k: f64,
    /// The tube-side film coefficient, W/(m²·K).
    pub tube_side_htc_w_per_m2_k: f64,
    /// The wall's heat-transfer area, m².
    pub heat_transfer_area_m2: f64,
    /// The carried shell CSTR temperature; absent or NaN seeds from the shell inlet.
    pub shell_fluid_temperature_k: Option<f64>,
    /// The carried tube CSTR temperature; absent or NaN seeds from the tube inlet.
    pub tube_fluid_temperature_k: Option<f64>,
    /// `true` when the **tube** is the hot declaration.
    pub tube_is_hot: bool,
}

/// The rating calculator's geometry, in SI. Every field is `None` until the caller states it,
/// and `None` leaves the class's constructor default in [`RatingRequest`].
#[derive(Debug, Clone, Default)]
pub struct RatingGeometry {
    pub tube_outside_diameter: Option<f64>,
    pub tube_inside_diameter: Option<f64>,
    pub tube_length: Option<f64>,
    pub tube_count: Option<f64>,
    pub tube_passes: Option<f64>,
    pub tube_pitch: Option<f64>,
    pub triangular_pitch: Option<bool>,
    pub shell_inside_diameter: Option<f64>,
    pub baffle_spacing: Option<f64>,
    pub baffle_count: Option<f64>,
    pub baffle_cut: Option<f64>,
    pub tube_wall_conductivity: Option<f64>,
    pub fouling_tube: Option<f64>,
    pub fouling_shell: Option<f64>,
    pub tube_fouling_layer_thickness: Option<f64>,
    pub tube_fouling_layer_conductivity: Option<f64>,
    pub shell_side_method: Option<ShellSideMethod>,
    pub shell_viscosity_wall: Option<f64>,
    pub has_sealing: Option<bool>,
    pub sealing_pairs: Option<f64>,
    pub tube_to_baffle_clearance: Option<f64>,
    pub shell_to_baffle_clearance: Option<f64>,
    pub bypass_area: Option<f64>,
}

/// What the rating calculator answered, as the equipment result publishes it.
#[derive(Debug, Clone)]
pub struct RatingDiagnostics {
    pub rating_u: f64,
    pub rating_ua: f64,
    pub rating_area: f64,
    pub tube_side_pressure_drop: f64,
    pub shell_side_pressure_drop: f64,
    pub tube_side_heat_transfer_coefficient: f64,
    pub shell_side_heat_transfer_coefficient: f64,
    pub tube_side_reynolds: f64,
    pub shell_side_reynolds: f64,
    pub tube_side_velocity: f64,
    pub shell_side_velocity: f64,
    pub jc: f64,
    pub jl: f64,
    pub jb: f64,
    pub js: f64,
    pub jr: f64,
}

impl HeatExchangerOutcome {
    /// The outcome of one solve, with the duty read off the hot side's own two states.
    ///
    /// **The duty is derived rather than carried from the branch that computed it**, because the
    /// two branches hold it in different terms - the rating scales a swing it chose, the pinned
    /// solve releases the swing it pinned - and the hot side's own enthalpy change is the one
    /// expression both reach. It is also what makes the sign one rule instead of two.
    #[must_use]
    fn of(hot: &Stream, hot_out: Stream, cold_out: Stream, rating: Option<Rating>) -> Self {
        let duty = watts(-total_swing(hot, &hot_out));
        Self {
            hot_out,
            cold_out,
            duty,
            rating,
            ua_value: None,
            delta_t_side: None,
            rating_diagnostics: None,
            transient: None,
        }
    }
}

/// Exchange heat between two streams.
///
/// Two modes, and which one runs is what the caller states:
///
/// * **`hot_outlet_temperature` or `cold_outlet_temperature`** pins that side's outlet at
///   its inlet pressure, and the other side is energy-balanced against it. Exactly one may
///   be given.
/// * **the effectiveness-NTU rating**, otherwise. `ua` is the overall conductance and
///   `flow_arrangement` picks the relation.
///
/// The rating follows `HeatExchanger.run`: each side's capacity is estimated by flashing it
/// at the **other side's inlet temperature** and dividing the total enthalpy change by the
/// temperature change, `NTU = UA / C_min`, and the effectiveness scales that swing. When
/// the effectiveness comes out at one the side that was not energy-balanced keeps the state
/// it was seeded at, which is the whole of how `run` expresses the infinite-area limit.
///
/// # Errors
/// [`azoth_core::AzothError::InvalidInput`] if neither or both outlet temperatures are
/// given, if the rating is asked for without a `ua`, if the arrangement is not one of
/// [`FLOW_ARRANGEMENTS`], or if the two inlets are at the same temperature so that there is
/// no driving force. Whatever the flashes refuse propagates too.
#[allow(clippy::too_many_arguments)] // one parameter per declared input
pub fn heat_exchanger(
    hot: &Stream,
    cold: &Stream,
    ua: Option<ThermalConductance>,
    flow_arrangement: &str,
    hot_outlet_temperature: Option<ThermodynamicTemperature>,
    cold_outlet_temperature: Option<ThermodynamicTemperature>,
    use_delta_t: bool,
    delta_t: ThermodynamicTemperature,
) -> Result<HeatExchangerOutcome> {
    if hot_outlet_temperature.is_some() && cold_outlet_temperature.is_some() {
        return Err(AzothError::invalid_input(
            "hot_outlet_temperature",
            "an exchanger pins one outlet at most: with both stated the other side has \
             nothing left to solve for",
        ));
    }
    if let Some(pinned) = hot_outlet_temperature {
        let (hot_out, cold_out) = specified_outlet(hot, cold, Side::Hot, pinned)?;
        return Ok(HeatExchangerOutcome::of(hot, hot_out, cold_out, None));
    }
    if let Some(pinned) = cold_outlet_temperature {
        let (hot_out, cold_out) = specified_outlet(hot, cold, Side::Cold, pinned)?;
        return Ok(HeatExchangerOutcome::of(hot, hot_out, cold_out, None));
    }
    // **A pin wins over `useDeltaT`**: the class's `runDeltaT` delegates to
    // `runSpecifiedStream` when an outlet is specified, so the delta-T branch is unreachable
    // with a pin stated.
    if use_delta_t {
        return delta_t_solve(hot, cold, delta_t);
    }

    let Some(ua) = ua else {
        return Err(AzothError::invalid_input(
            "ua",
            "the rating needs an overall conductance. NeqSim's `UAvalue` defaults to 500 \
             W/K, and this refuses to guess it: a default that only changes the answer is \
             not one a caller can see",
        ));
    };
    if !FLOW_ARRANGEMENTS.contains(&flow_arrangement) {
        return Err(AzothError::invalid_input(
            "flow_arrangement",
            format!(
                "{flow_arrangement} is not one of {FLOW_ARRANGEMENTS:?}. NeqSim's `run` \
                 falls through to the counterflow relation for an arrangement it does not \
                 know, which would make a misspelling a plausible answer"
            ),
        ));
    }
    if (hot.t.value - cold.t.value).abs()
        <= f64::EPSILON * hot.t.value.abs().max(cold.t.value.abs())
    {
        return Err(AzothError::invalid_input(
            "hot_in",
            format!(
                "both inlets are at {} K, so there is no driving force for the rating to \
                 size against",
                hot.t.value
            ),
        ));
    }

    // The seeded capacities: each side flashed at the *other's* inlet temperature, which
    // is `run`'s own estimate and is what makes the pass count not matter.
    let hot_seeded = Stream::from_pt(hot.components.clone(), hot.z.clone(), hot.n, hot.p, cold.t)?;
    let cold_seeded = Stream::from_pt(
        cold.components.clone(),
        cold.z.clone(),
        cold.n,
        cold.p,
        hot.t,
    )?;

    // **Absolute values, because a swing is signed and a capacity is not.** The hot side's
    // seeded swing is negative - it cools - and taking it at face value would make `C_min`
    // negative and the NTU with it, which is an exponential in the wrong direction rather
    // than an error.
    let span = (hot.t.value - cold.t.value).abs();
    let cold_capacity = total_swing(cold, &cold_seeded).abs() / span;
    let hot_capacity = total_swing(hot, &hot_seeded).abs() / span;
    let (c_min, c_max) = if cold_capacity < hot_capacity {
        (cold_capacity, hot_capacity)
    } else {
        (hot_capacity, cold_capacity)
    };
    let capacity_ratio = c_min / c_max;
    let ntu = ua.value / c_min;
    let effectiveness = effectiveness(ntu, capacity_ratio, flow_arrangement);
    // **The rating's own numbers, kept rather than dropped.** They are what sized the exchanger,
    // and a faceplate that shows a duty and nothing else cannot say whether the answer was
    // capacity-limited or area-limited - which is the difference between a bigger machine helping
    // and it not helping at all.
    let rating = Rating {
        ntu,
        effectiveness,
        c_min: watts_per_kelvin(c_min),
        c_max: watts_per_kelvin(c_max),
        capacity_ratio,
    };

    // `run` energy-balances the side it swapped *away* from, and the side it keeps is the
    // one whose seeded swing is the larger. Both are totals, so the comparison is between
    // the two sides' actual duties and not between their molar capacities.
    let hot_swing = total_swing(hot, &hot_seeded);
    let cold_swing = total_swing(cold, &cold_seeded);
    let set_side = if cold_swing.abs() > hot_swing.abs() {
        Side::Hot
    } else {
        Side::Cold
    };
    let (set, set_seeded) = match set_side {
        Side::Hot => (hot, &hot_seeded),
        Side::Cold => (cold, &cold_seeded),
    };
    let calculated = match set_side.other() {
        Side::Hot => hot,
        Side::Cold => cold,
    };
    let d = effectiveness * total_swing(set, set_seeded);

    let calculated_out = Stream::from_ph(
        calculated.components.clone(),
        calculated.z.clone(),
        calculated.n,
        calculated.p,
        moles_with(calculated, -d),
    )?;
    // **At an effectiveness of one the set side keeps its seeded state.** That is not an
    // omission in `run`: the seeded state is "this side at the other's inlet temperature",
    // which is exactly what an effectiveness of one means, and the branch exists because
    // re-flashing it would find the same state by a longer route.
    let set_out = if (effectiveness - 1.0).abs() > 1e-10 {
        Stream::from_ph(
            set.components.clone(),
            set.z.clone(),
            set.n,
            set.p,
            moles_with(set, d),
        )?
    } else {
        set_seeded.clone()
    };

    let (hot_out, cold_out) = match set_side {
        Side::Hot => (set_out, calculated_out),
        Side::Cold => (calculated_out, set_out),
    };
    Ok(HeatExchangerOutcome::of(
        hot,
        hot_out,
        cold_out,
        Some(rating),
    ))
}

/// `HeatExchanger.runTransient`'s enabled branch, one interval.
///
/// **The state is the class's own**: both fluid temperatures and the wall are solved
/// together from the interval's first law, with each side's enthalpy, heat capacity and
/// corrected density taken at the candidate temperature. The class's 40-iteration Newton,
/// its halving line search and the `max(1e-5, 1e-5/dt)` residual bound are transcribed, as
/// are the energy-balance receipt and the `q_shell_to_wall` duty. The class's stream 0 is
/// the **shell** and stream 1 the **tube**, the reverse of [`heat_exchanger_rating`]'s
/// convention; `settings.tube_is_hot` states the mapping.
///
/// # Errors
/// [`AzothError::InvalidInput`] for a non-positive side flow or timestep, a non-finite gate,
/// a two-phase candidate, or a solve that cannot reduce its residual.
pub fn heat_exchanger_transient(
    hot: &Stream,
    cold: &Stream,
    settings: &TransientSettings,
) -> Result<HeatExchangerOutcome> {
    for (field, flow) in [("hot_in_n", hot.n), ("cold_in_n", cold.n)] {
        if !flow.is_finite() || flow <= 0.0 {
            return Err(AzothError::invalid_input(
                field,
                format!("a unit operation needs a positive molar flow, and {flow} mol/s is not"),
            ));
        }
    }
    let dt = settings.time_step_s;
    if !dt.is_finite() || dt <= 0.0 {
        return Err(AzothError::invalid_input(
            "time_step",
            format!("a finite positive timestep is required, and {dt} s is not"),
        ));
    }
    for (field, value) in [
        ("wall_mass", settings.wall_mass_kg),
        ("wall_cp", settings.wall_cp_j_per_kg_k),
        ("heat_transfer_area", settings.heat_transfer_area_m2),
        ("shell_side_htc", settings.shell_side_htc_w_per_m2_k),
        ("tube_side_htc", settings.tube_side_htc_w_per_m2_k),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(AzothError::invalid_input(
                field,
                "Wall mass/Cp, both film coefficients and area must be finite and positive",
            ));
        }
    }
    for (field, value) in [
        ("shell_holdup_volume", settings.shell_holdup_volume_m3),
        ("tube_holdup_volume", settings.tube_holdup_volume_m3),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(AzothError::invalid_input(
                field,
                format!("a finite nonnegative volume is required, and {value} is not"),
            ));
        }
    }
    for (field, value) in [
        ("wall_temperature", settings.wall_temperature_k),
        (
            "shell_fluid_temperature",
            settings.shell_fluid_temperature_k,
        ),
        ("tube_fluid_temperature", settings.tube_fluid_temperature_k),
    ] {
        if let Some(value) = value {
            // NaN is the class's seed sentinel; an infinity is not a state.
            if !value.is_finite() && !value.is_nan() {
                return Err(AzothError::invalid_input(
                    field,
                    format!("a carried temperature is finite or NaN, and {value} is neither"),
                ));
            }
        }
    }

    let (shell, tube) = if settings.tube_is_hot {
        (cold, hot)
    } else {
        (hot, cold)
    };

    let shell_inlet = side_properties(shell, shell.t.value)?;
    let tube_inlet = side_properties(tube, tube.t.value)?;
    let mass_flow = [shell.mass_flow()?, tube.mass_flow()?];

    let t_shell = settings
        .shell_fluid_temperature_k
        .filter(|value| !value.is_nan())
        .unwrap_or(shell.t.value);
    let t_tube = settings
        .tube_fluid_temperature_k
        .filter(|value| !value.is_nan())
        .unwrap_or(tube.t.value);
    let wall = settings
        .wall_temperature_k
        .filter(|value| !value.is_nan())
        .unwrap_or(0.5 * (t_shell + t_tube));
    if !(wall.is_finite() && wall > 0.0) {
        return Err(AzothError::invalid_input(
            "wall_temperature",
            "the initial wall temperature must be finite and positive",
        ));
    }
    let shell_start = side_properties(shell, t_shell)?;
    let tube_start = side_properties(tube, t_tube)?;
    let start = ThermalState {
        temperature: [t_shell, t_tube, wall],
        enthalpy: [shell_start.h_mass, tube_start.h_mass],
        // The class fixes the holdup mass on the first interval and carries it; the inlet
        // density is the one its `initialState` used for that first inventory.
        mass: [
            settings.shell_holdup_volume_m3 * shell_inlet.density,
            settings.tube_holdup_volume_m3 * tube_inlet.density,
        ],
    };
    let wall_capacity = settings.wall_mass_kg * settings.wall_cp_j_per_kg_k;
    let conductance = [
        settings.shell_side_htc_w_per_m2_k * settings.heat_transfer_area_m2,
        settings.tube_side_htc_w_per_m2_k * settings.heat_transfer_area_m2,
    ];

    let evaluation = solve_transient(
        shell,
        tube,
        &start,
        mass_flow,
        [shell_inlet.h_mass, tube_inlet.h_mass],
        conductance,
        wall_capacity,
        dt,
    )?;

    let shell_out = Stream::from_pt(
        shell.components.clone(),
        shell.z.clone(),
        shell.n,
        shell.p,
        kelvins(evaluation.temperature[0]),
    )?;
    let tube_out = Stream::from_pt(
        tube.components.clone(),
        tube.z.clone(),
        tube.n,
        tube.p,
        kelvins(evaluation.temperature[1]),
    )?;
    let (hot_out, cold_out) = if settings.tube_is_hot {
        (tube_out, shell_out)
    } else {
        (shell_out, tube_out)
    };
    let mut outcome = HeatExchangerOutcome::of(hot, hot_out, cold_out, None);
    // The class's `getDuty()` is the shell-to-wall transfer; the steady branches publish
    // the hot-side release instead.
    outcome.duty = watts(conductance[0] * (evaluation.temperature[0] - evaluation.temperature[2]));

    let start_energy = start.mass[0] * start.enthalpy[0]
        + start.mass[1] * start.enthalpy[1]
        + wall_capacity * start.temperature[2];
    let end_energy = start.mass[0] * evaluation.enthalpy[0]
        + start.mass[1] * evaluation.enthalpy[1]
        + wall_capacity * evaluation.temperature[2];
    let boundary_energy = dt
        * (mass_flow[0] * (shell_inlet.h_mass - evaluation.enthalpy[0])
            + mass_flow[1] * (tube_inlet.h_mass - evaluation.enthalpy[1]));
    let accumulation = end_energy - start_energy;

    outcome.transient = Some(TransientState {
        wall_temperature: kelvins(evaluation.temperature[2]),
        shell_fluid_temperature: Some(kelvins(evaluation.temperature[0])),
        tube_fluid_temperature: Some(kelvins(evaluation.temperature[1])),
        thermal_inventory_energy: joules(end_energy),
        balance_accumulation: joules(accumulation),
        balance_boundary_energy: joules(boundary_energy),
        balance_residual: joules(accumulation - boundary_energy),
    });
    Ok(outcome)
}

struct ThermalState {
    temperature: [f64; 3],
    enthalpy: [f64; 2],
    mass: [f64; 2],
}

struct SideProperties {
    h_mass: f64,
    cp_mass: f64,
    density: f64,
}

fn side_properties(stream: &Stream, temperature: f64) -> Result<SideProperties> {
    let state = Stream::from_pt(
        stream.components.clone(),
        stream.z.clone(),
        1.0,
        stream.p,
        kelvins(temperature),
    )?;
    let (mixture, _) = stream.mixture()?;
    if pt_flash(&mixture, kelvins(temperature), stream.p, &stream.z)?.phase == Phase::TwoPhase {
        return Err(AzothError::invalid_input(
            "stream",
            "the exchanger's thermal inventory requires a finite stable single-phase fluid",
        ));
    }
    let molar_mass = state.molar_mass()?.value;
    Ok(SideProperties {
        h_mass: state.h.value / molar_mass,
        cp_mass: state.molar_heat_capacity()? / molar_mass,
        density: state.corrected_density()?,
    })
}

struct Evaluation {
    temperature: [f64; 3],
    enthalpy: [f64; 2],
    residual: [f64; 3],
    jacobian: [[f64; 3]; 3],
    maximum_residual: f64,
}

#[allow(clippy::too_many_arguments)]
fn solve_transient(
    shell: &Stream,
    tube: &Stream,
    start: &ThermalState,
    mass_flow: [f64; 2],
    inlet_enthalpy: [f64; 2],
    conductance: [f64; 2],
    wall_capacity: f64,
    dt: f64,
) -> Result<Evaluation> {
    let mut temperature = start.temperature;
    let mut minimum = start
        .temperature
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let mut maximum = start
        .temperature
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    for inlet in [shell.t.value, tube.t.value] {
        minimum = minimum.min(inlet);
        maximum = maximum.max(inlet);
    }
    for _ in 0..40 {
        let evaluated = evaluate_transient(
            shell,
            tube,
            start,
            mass_flow,
            inlet_enthalpy,
            conductance,
            wall_capacity,
            dt,
            temperature,
        )?;
        if evaluated.maximum_residual < (1.0e-5f64).max(1.0e-5 / dt) {
            return Ok(evaluated);
        }
        let change = match solve3(
            evaluated.jacobian,
            [
                -evaluated.residual[0],
                -evaluated.residual[1],
                -evaluated.residual[2],
            ],
        ) {
            Some(change) => change,
            None => {
                return Err(AzothError::invalid_input(
                    "time_step",
                    "the exchanger enthalpy Jacobian is singular",
                ));
            }
        };
        let mut accepted = false;
        let mut fraction = 1.0;
        while fraction >= 1.0e-6 {
            let mut trial = [0.0; 3];
            let mut bounded = true;
            for node in 0..3 {
                trial[node] = temperature[node] + fraction * change[node];
                bounded &= trial[node].is_finite()
                    && trial[node] >= minimum - 1.0e-8
                    && trial[node] <= maximum + 1.0e-8;
            }
            if bounded {
                let candidate = evaluate_transient(
                    shell,
                    tube,
                    start,
                    mass_flow,
                    inlet_enthalpy,
                    conductance,
                    wall_capacity,
                    dt,
                    trial,
                )?;
                if candidate.maximum_residual < evaluated.maximum_residual {
                    temperature = trial;
                    accepted = true;
                    break;
                }
            }
            fraction *= 0.5;
        }
        if !accepted {
            return Err(AzothError::invalid_input(
                "time_step",
                "bounded exchanger enthalpy solve failed to reduce its residual",
            ));
        }
    }
    Err(AzothError::invalid_input(
        "time_step",
        "exchanger enthalpy solve exceeded 40 iterations",
    ))
}

#[allow(clippy::too_many_arguments)]
fn evaluate_transient(
    shell: &Stream,
    tube: &Stream,
    start: &ThermalState,
    mass_flow: [f64; 2],
    inlet_enthalpy: [f64; 2],
    conductance: [f64; 2],
    wall_capacity: f64,
    dt: f64,
    temperature: [f64; 3],
) -> Result<Evaluation> {
    let sides = [
        side_properties(shell, temperature[0])?,
        side_properties(tube, temperature[1])?,
    ];
    let mut residual = [0.0; 3];
    let mut jacobian = [[0.0; 3]; 3];
    residual[2] = wall_capacity / dt * (temperature[2] - start.temperature[2]);
    jacobian[2][2] = wall_capacity / dt;
    for side in 0..2 {
        let properties = &sides[side];
        let transfer = conductance[side] * (temperature[side] - temperature[2]);
        residual[side] = start.mass[side] / dt * (properties.h_mass - start.enthalpy[side])
            + mass_flow[side] * (properties.h_mass - inlet_enthalpy[side])
            + transfer;
        jacobian[side][side] =
            (start.mass[side] / dt + mass_flow[side]) * properties.cp_mass + conductance[side];
        jacobian[side][2] = -conductance[side];
        residual[2] -= transfer;
        jacobian[2][side] = -conductance[side];
        jacobian[2][2] += conductance[side];
    }
    let maximum_residual = residual
        .iter()
        .fold(0.0f64, |largest, value| largest.max(value.abs()));
    Ok(Evaluation {
        temperature,
        enthalpy: [sides[0].h_mass, sides[1].h_mass],
        residual,
        jacobian,
        maximum_residual,
    })
}

fn solve3(matrix: [[f64; 3]; 3], rhs: [f64; 3]) -> Option<[f64; 3]> {
    let mut matrix = matrix;
    let mut rhs = rhs;
    for column in 0..3 {
        let mut pivot = column;
        for row in column + 1..3 {
            if matrix[row][column].abs() > matrix[pivot][column].abs() {
                pivot = row;
            }
        }
        if !matrix[pivot][column].is_finite() || matrix[pivot][column].abs() < 1.0e-300 {
            return None;
        }
        matrix.swap(column, pivot);
        rhs.swap(column, pivot);
        let diagonal = matrix[column][column];
        for row in column + 1..3 {
            let factor = matrix[row][column] / diagonal;
            for k in column..3 {
                matrix[row][k] -= factor * matrix[column][k];
            }
            rhs[row] -= factor * rhs[column];
        }
    }
    let mut solution = [0.0; 3];
    for row in (0..3).rev() {
        let mut value = rhs[row];
        for column in row + 1..3 {
            value -= matrix[row][column] * solution[column];
        }
        solution[row] = value / matrix[row][row];
    }
    Some(solution)
}

/// `HeatExchanger.runThermalHydraulicRating`: a fixed geometry, correlated UA and pressure
/// drops, and the effectiveness-NTU duty the class closes both outlets at.
///
/// `tube_is_hot` names which declaration the class's stream 0 is, because this port's
/// `hot`/`cold` carry meaning rather than constructor order.
///
/// # Errors
/// [`AzothError::InvalidInput`] for a two-phase inlet or a capacity clone that crosses a
/// phase transition, a pressure drop that exhausts an inlet, a non-positive conductance, or
/// an energy closure the PH flash misses. Whatever `thermal.shell_and_tube_rating` refuses
/// propagates too.
#[allow(clippy::too_many_lines)]
pub fn heat_exchanger_rating(
    hot: &Stream,
    cold: &Stream,
    tube_is_hot: bool,
    area: f64,
    geometry: &RatingGeometry,
    flow_arrangement: &str,
) -> Result<HeatExchangerOutcome> {
    if !area.is_finite() || area <= 0.0 {
        return Err(AzothError::invalid_input(
            "rating_area",
            "the thermal-hydraulic rating needs a positive outside area",
        ));
    }
    let (tube, shell) = if tube_is_hot {
        (hot, cold)
    } else {
        (cold, hot)
    };
    let tube_view = single_phase_view(tube)?;
    let shell_view = single_phase_view(shell)?;

    let mut request = RatingRequest::default();
    apply_geometry(&mut request, geometry);
    request.tube_density = tube_view.density;
    request.tube_cp = tube_view.cp_mass;
    request.tube_viscosity = tube_view.transport.mu.value;
    request.tube_conductivity = tube_view.transport.k.value;
    request.tube_mass_flow = tube.mass_flow()?;
    request.shell_density = shell_view.density;
    request.shell_cp = shell_view.cp_mass;
    request.shell_viscosity = shell_view.transport.mu.value;
    request.shell_conductivity = shell_view.transport.k.value;
    request.shell_mass_flow = shell.mass_flow()?;

    let calculator = thermal_rate(&request)?;
    let ua = calculator.overall_heat_transfer_coefficient.value * area;
    if !ua.is_finite() || ua <= 0.0 {
        return Err(AzothError::invalid_input(
            "rating_u",
            "the thermal-hydraulic rating produced an invalid conductance",
        ));
    }
    let tube_p_out = tube.p.value - calculator.tube_side_pressure_drop.value;
    let shell_p_out = shell.p.value - calculator.shell_side_pressure_drop.value;
    for (field, pressure) in [("tube", tube_p_out), ("shell", shell_p_out)] {
        if !pressure.is_finite() || pressure <= 0.0 {
            return Err(AzothError::invalid_input(
                field,
                "the correlated pressure drop exhausts the inlet pressure",
            ));
        }
    }

    // The capacities are the class's own: each side flashed at the *other* inlet
    // temperature, over the tube-minus-shell span, and the same phase kind required.
    let span = tube.t.value - shell.t.value;
    let (ntu, effectiveness_value, c_min, c_max, capacity_ratio, heat) = if span.abs() <= 1e-10 {
        // The class's own `Math.abs(temperatureDifference) > 1e-10` gate: with no
        // driving force it leaves every rating quantity at zero and still drops the
        // pressures, which is the equal-inlet row of the capture.
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    } else {
        let limit_tube = Stream::from_pt(
            tube.components.clone(),
            tube.z.clone(),
            tube.n,
            tube.p,
            shell.t,
        )?;
        let limit_shell = Stream::from_pt(
            shell.components.clone(),
            shell.z.clone(),
            shell.n,
            shell.p,
            tube.t,
        )?;
        let limit_tube_view = single_phase_view(&limit_tube)?;
        let limit_shell_view = single_phase_view(&limit_shell)?;
        for (side, limit, original) in [
            ("tube", &limit_tube_view, &tube_view),
            ("shell", &limit_shell_view, &shell_view),
        ] {
            if limit.kind != original.kind {
                return Err(AzothError::invalid_input(
                    side,
                    "the rating temperature interval crosses a phase transition",
                ));
            }
        }
        let tube_capacity =
            (((limit_tube.n * limit_tube.h.value) - (tube.n * tube.h.value)) / span).abs();
        let shell_capacity =
            (((limit_shell.n * limit_shell.h.value) - (shell.n * shell.h.value)) / span).abs();
        for (field, capacity) in [("tube", tube_capacity), ("shell", shell_capacity)] {
            if !capacity.is_finite() || capacity <= 0.0 {
                return Err(AzothError::invalid_input(
                    field,
                    "the thermal-hydraulic rating requires positive finite heat capacity rates",
                ));
            }
        }
        let c_min = tube_capacity.min(shell_capacity);
        let c_max = tube_capacity.max(shell_capacity);
        let capacity_ratio = c_min / c_max;
        let ntu = ua / c_min;
        // The class's own special case: the counterflow limit also removes the 0/0
        // singularity of the legacy shell-and-tube alias at Cr = 1.
        let effectiveness_value =
            if (capacity_ratio - 1.0).abs() < 1e-10 && flow_arrangement != "parallelflow" {
                ntu / (1.0 + ntu)
            } else {
                effectiveness(ntu, capacity_ratio, flow_arrangement)
            };
        let heat = effectiveness_value * c_min * span;
        if !heat.is_finite() || !effectiveness_value.is_finite() {
            return Err(AzothError::invalid_input(
                "rating",
                "the thermal-hydraulic rating produced an invalid effectiveness or duty",
            ));
        }
        (ntu, effectiveness_value, c_min, c_max, capacity_ratio, heat)
    };

    let tube_target = tube.h.value - heat / tube.n;
    let shell_target = shell.h.value + heat / shell.n;
    let tube_out = Stream::from_ph(
        tube.components.clone(),
        tube.z.clone(),
        tube.n,
        pascals(tube_p_out),
        joules_per_mole(tube_target),
    )?;
    let shell_out = Stream::from_ph(
        shell.components.clone(),
        shell.z.clone(),
        shell.n,
        pascals(shell_p_out),
        joules_per_mole(shell_target),
    )?;
    for (side, outlet, inlet_view, target) in [
        ("tube", &tube_out, &tube_view, tube_target),
        ("shell", &shell_out, &shell_view, shell_target),
    ] {
        let view = single_phase_view(outlet)?;
        if view.kind != inlet_view.kind {
            return Err(AzothError::invalid_input(
                side,
                "the rating outlet changed phase",
            ));
        }
        if !outlet.h.value.is_finite()
            || (outlet.h.value - target).abs() > 1e-5 * target.abs().max(1.0)
        {
            return Err(AzothError::invalid_input(
                side,
                "the rating outlet PH flash missed its energy closure",
            ));
        }
    }

    let (hot_out, cold_out) = if tube_is_hot {
        (tube_out, shell_out)
    } else {
        (shell_out, tube_out)
    };
    let diagnostics = RatingDiagnostics {
        rating_u: calculator.overall_heat_transfer_coefficient.value,
        rating_ua: ua,
        rating_area: area,
        tube_side_pressure_drop: calculator.tube_side_pressure_drop.value,
        shell_side_pressure_drop: calculator.shell_side_pressure_drop.value,
        tube_side_heat_transfer_coefficient: calculator.tube_side_heat_transfer_coefficient.value,
        shell_side_heat_transfer_coefficient: calculator.shell_side_heat_transfer_coefficient.value,
        tube_side_reynolds: calculator.tube_side_reynolds,
        shell_side_reynolds: calculator.shell_side_reynolds,
        tube_side_velocity: calculator.tube_side_velocity.value,
        shell_side_velocity: calculator.shell_side_velocity.value,
        jc: calculator.jc,
        jl: calculator.jl,
        jb: calculator.jb,
        js: calculator.js,
        jr: calculator.jr,
    };
    Ok(HeatExchangerOutcome {
        hot_out,
        cold_out,
        duty: watts(heat.abs()),
        rating: Some(Rating {
            ntu,
            effectiveness: effectiveness_value,
            c_min: watts_per_kelvin(c_min),
            c_max: watts_per_kelvin(c_max),
            capacity_ratio,
        }),
        ua_value: None,
        delta_t_side: None,
        rating_diagnostics: Some(diagnostics),
        transient: None,
    })
}

/// Copy the stated geometry into the calculator request, leaving the class's defaults where
/// the caller stated nothing.
fn apply_geometry(request: &mut RatingRequest, geometry: &RatingGeometry) {
    macro_rules! set {
        ($field:ident) => {
            if let Some(value) = geometry.$field {
                request.$field = value;
            }
        };
    }
    set!(tube_outside_diameter);
    set!(tube_inside_diameter);
    set!(tube_length);
    set!(tube_count);
    set!(tube_passes);
    set!(tube_pitch);
    if let Some(value) = geometry.triangular_pitch {
        request.triangular_pitch = value;
    }
    set!(shell_inside_diameter);
    set!(baffle_spacing);
    set!(baffle_count);
    set!(baffle_cut);
    set!(tube_wall_conductivity);
    set!(fouling_tube);
    set!(fouling_shell);
    request.tube_fouling_layer_thickness = geometry.tube_fouling_layer_thickness;
    request.tube_fouling_layer_conductivity = geometry.tube_fouling_layer_conductivity;
    if let Some(value) = geometry.shell_side_method {
        request.shell_side_method = value;
    }
    request.shell_viscosity_wall = geometry.shell_viscosity_wall;
    request.has_sealing = geometry.has_sealing;
    request.sealing_pairs = geometry.sealing_pairs;
    request.tube_to_baffle_clearance = geometry.tube_to_baffle_clearance;
    request.shell_to_baffle_clearance = geometry.shell_to_baffle_clearance;
    request.bypass_area = geometry.bypass_area;
}

/// The `outTemperature` specification: pin one outlet, energy-balance the other.
/// `runDeltaT`: perturb each side by `delta_t` K, estimate its capacity from the enthalpy
/// change, set the side with the larger swing, and close the other by `PHflash`.
///
/// **The class's first pass is not modelled.** It seeds `outStream[0]` at
/// `guessOutTemperature` and recurses, and the `else` branch then re-clones both inlets, so
/// the seed cannot reach the answer - the two rows of `HeatExchangerTest.testRun2`, one
/// after an ordinary rating and one on a fresh exchanger, agree to `1e-3` on `UAvalue`.
fn delta_t_solve(
    hot: &Stream,
    cold: &Stream,
    delta_t: ThermodynamicTemperature,
) -> Result<HeatExchangerOutcome> {
    let t0 = hot.t.value;
    let t1 = cold.t.value;
    if (t0 - t1).abs() <= f64::EPSILON * t0.abs().max(t1.abs()) {
        return Err(AzothError::invalid_input(
            "hot_in",
            format!(
                "both inlets are at {t0} K, so `runDeltaT`'s sign is zero and nothing is perturbed"
            ),
        ));
    }
    if delta_t.value <= 0.0 {
        return Err(AzothError::invalid_input(
            "delta_t",
            format!(
                "`delta_t` is a positive temperature step, and {} K is not",
                delta_t.value
            ),
        ));
    }
    let sign = (t0 - t1).signum();
    let delta = delta_t.value;

    // The class perturbs `outStream[1]` to the hot inlet plus the step and `outStream[0]` to
    // the cold inlet less it.
    let cold_pert = Stream::from_pt(
        cold.components.clone(),
        cold.z.clone(),
        cold.n,
        cold.p,
        kelvins(t0 + sign * delta),
    )?;
    let hot_pert = Stream::from_pt(
        hot.components.clone(),
        hot.z.clone(),
        hot.n,
        hot.p,
        kelvins(t1 - sign * delta),
    )?;
    let cold_swing = cold.n * (cold_pert.h.value - cold.h.value);
    let hot_swing = hot.n * (hot_pert.h.value - hot.h.value);
    let cold_capacity = cold_swing.abs() / (t0 + sign * delta - t1).abs();
    let hot_capacity = hot_swing.abs() / (t1 - sign * delta - t0).abs();
    let _ = (cold_capacity, hot_capacity);

    // `if (|dH1| < |dH2|) swap`, where 1 is the cold side and 2 the hot one.
    let (set_is_hot, d_h) = if cold_swing.abs() < hot_swing.abs() {
        (true, hot_swing)
    } else {
        (false, cold_swing)
    };
    let (set_in, set_out, other_in) = if set_is_hot {
        (hot, hot_pert, cold)
    } else {
        (cold, cold_pert, hot)
    };
    let other_out = Stream::from_ph(
        other_in.components.clone(),
        other_in.z.clone(),
        other_in.n,
        other_in.p,
        joules_per_mole(other_in.h.value - d_h / other_in.n),
    )?;
    let ua_value = d_h / (set_out.t.value - set_in.t.value);

    let (hot_out, cold_out) = if set_is_hot {
        (set_out, other_out)
    } else {
        (other_out, set_out)
    };
    let mut outcome = HeatExchangerOutcome::of(hot, hot_out, cold_out, None);
    outcome.ua_value = Some(ua_value);
    outcome.delta_t_side = Some(if set_is_hot { "hot" } else { "cold" });
    Ok(outcome)
}

fn specified_outlet(
    hot: &Stream,
    cold: &Stream,
    side: Side,
    pinned: ThermodynamicTemperature,
) -> Result<(Stream, Stream)> {
    let (specified, other) = match side {
        Side::Hot => (hot, cold),
        Side::Cold => (cold, hot),
    };
    let specified_out = Stream::from_pt(
        specified.components.clone(),
        specified.z.clone(),
        specified.n,
        specified.p,
        pinned,
    )?;
    // The duty the pinned side released, as a total: `run` compares and applies totals.
    let d = total_swing(specified, &specified_out);
    let other_out = Stream::from_ph(
        other.components.clone(),
        other.z.clone(),
        other.n,
        other.p,
        moles_with(other, -d),
    )?;
    Ok(match side {
        Side::Hot => (specified_out, other_out),
        Side::Cold => (other_out, specified_out),
    })
}

/// A side's total enthalpy change against a second state of the same stream, in W.
fn total_swing(from: &Stream, to: &Stream) -> f64 {
    to.n * (to.h.value - from.h.value)
}

/// A stream's molar enthalpy shifted by `total` watts over its own flow.
fn moles_with(stream: &Stream, total: f64) -> MolarEnergy {
    joules_per_mole(stream.h.value + total / stream.n)
}

/// The effectiveness-NTU relations `HeatExchanger.calcThermalEffectivenes` carries.
///
/// NeqSim spells the counterflow and shell-and-tube cases with the same expression and the
/// parallel-flow one differently; that is reproduced rather than tidied, because it is what
/// the class answers and `shell and tube` is a name a caller can state.
fn effectiveness(ntu: f64, capacity_ratio: f64, arrangement: &str) -> f64 {
    if capacity_ratio == 0.0 {
        return 1.0 - (-ntu).exp();
    }
    if arrangement == "parallelflow" {
        return (1.0 - (-ntu * (1.0 + capacity_ratio)).exp()) / (1.0 + capacity_ratio);
    }
    if arrangement == "counterflow" && capacity_ratio == 1.0 {
        return ntu / (1.0 + ntu);
    }
    let exp = (-ntu * (1.0 - capacity_ratio)).exp();
    (1.0 - exp) / (1.0 - capacity_ratio * exp)
}
