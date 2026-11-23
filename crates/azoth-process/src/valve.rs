//! `ControlValveSizing`, the `ValveMechanicalDesign` default: the IEC 60534 simplified sizing.
//!
//! **The default sizing standard and not the IEC class.** `ValveMechanicalDesign`'s default
//! `valveSizingMethod` is `ControlValveSizing` (`setValveSizingStandard` selects the IEC
//! variants), and this module is that class: `calcKv`, its inverse flow and its outlet-pressure
//! solve, on the liquid and gas branches, with the opening characteristic and the fouling
//! fraction. **All nine `setValveSizingStandard` values are carried**; the five multiphase
//! chokes read the flash's labelled phases rather than one root.
//!
//! **The gas formula is the class's, unit mixing included.** It reads `Q` at standard conditions
//! (273.15 K, 101.325 kPa) from the *actual* volumetric flow through `P1`, `T` and `Z`, then
//! `Kv = Q_std/(N9 P1 Y) sqrt(M T Z / x)`, with `N9 = 24.6`, `M` in g/mol, `T` in kelvin and
//! `P1` in kPa. The liquid form is `Kv = Q/(sqrt(dP/SG))` with `SG = rho/1000`. Both divide by
//! the opening factor afterwards, so the reported `Kv` is the rated value at 100% opening.
//!
//! **The outlet-pressure solve is a bisection and it is the class's own.** The gas branch is not
//! invertible in closed form, so the class bisects `P2` in `[0.1, P1 - 0.001]` bara against a
//! `Kv` tolerance of `1e-6`; the liquid branch is the analytic `P2 = P1 - (Q/Kv)^2 SG`.

use azoth_core::units::{Pressure, pascals};
use azoth_core::{AzothError, Result};
use azoth_eos::hydrate_inhibitor_wt::PhaseLabel;
use azoth_eos::{Phase, molar_enthalpy_entropy, pr_molar_volume, pt_flash};

use crate::segment::phase::{
    labelled_phases_at_from_feed, liquid_view, mass_density, molar_mass_of,
    system_mass_density_from_feed, system_volumetric_flow_from_feed, vapour_view,
};
use crate::stream::Stream;

/// `ControlValveSizing.KV_TO_CV_FACTOR`: `Cv = Kv * 1.156`.
pub const KV_TO_CV_FACTOR: f64 = 1.156;
/// `ControlValveSizing.N9`, the IEC 60534 constant for standard m3/h, kPa, K and g/mol.
pub const N9: f64 = 24.6;
/// `ControlValveSizing.T_STD`: the standard temperature the gas flow is quoted at, K.
pub const STANDARD_TEMPERATURE_K: f64 = 273.15;
/// `ControlValveSizing.P_STD_KPA`: the standard pressure the gas flow is quoted at, kPa.
pub const STANDARD_PRESSURE_KPA: f64 = 101.325;
/// `ControlValveSizing.xT`, the pressure-drop ratio factor.
pub const DEFAULT_XT: f64 = 0.137;

pub const DEFAULT_CHOKE_DIAMETER_M: f64 = 0.0254;

/// `ControlValveSizing_IEC_60534.N1`.
pub const IEC_N1: f64 = 0.1;
/// `ControlValveSizing_IEC_60534.N9`.
pub const IEC_N9: f64 = 24.6;
/// `ControlValveSizing_IEC_60534.rho0`.
pub const IEC_RHO0: f64 = 999.103;
/// `ControlValveSizing_IEC_60534.FL`, the liquid pressure recovery factor.
pub const IEC_FL: f64 = 0.9;
/// `ControlValveSizing_simple.Cd`, the production choke's discharge coefficient.
pub const SIMPLE_CD: f64 = 0.85;
/// The ratio `Cg = Cv * 1360`.
pub const CG_PER_CV: f64 = 1360.0;
/// The bisection's `Kv` tolerance and cap, the class's own.
pub const BISECTION_TOLERANCE: f64 = 1.0e-6;
/// See [`BISECTION_TOLERANCE`].
pub const MAX_BISECTION_ITERATIONS: usize = 100;
/// `ThermodynamicConstantsInterface.R`, J/(mol*K), the constant `getGamma2` subtracts.
pub const R_J_PER_MOL_K: f64 = 8.314;
/// `ThrottlingValve.percentValveOpening`'s initialiser.
pub const DEFAULT_OPENING_PERCENT: f64 = 100.0;
/// `ValveMechanicalDesign`'s default characteristic.
pub const DEFAULT_CHARACTERISTIC: ValveCharacteristic = ValveCharacteristic::Linear;
/// `EqualPercentageCharacteristic`'s default rangeability.
pub const DEFAULT_RANGEABILITY: f64 = 50.0;
/// `ModifiedParabolicCharacteristic`'s default exponent.
pub const DEFAULT_PARABOLIC_EXPONENT: f64 = 2.0;

/// The class's four `ValveCharacteristic` implementations, by the names its setter takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValveCharacteristic {
    /// `LinearCharacteristic`: the opening itself.
    Linear,
    /// `EqualPercentageCharacteristic`: `R^(x-1)`, floored at `1/R` and capped at `1`.
    EqualPercentage,
    /// `QuickOpeningCharacteristic`: `sqrt(x)`.
    QuickOpening,
    /// `ModifiedParabolicCharacteristic`: `x^n`.
    ModifiedParabolic,
}

impl ValveCharacteristic {
    /// The characteristic a declaration's name selects.
    ///
    /// # Errors
    /// [`AzothError::InvalidInput`] on a name the class does not carry.
    pub fn named(name: &str) -> Result<Self> {
        match name {
            "linear" => Ok(Self::Linear),
            "equal_percentage" => Ok(Self::EqualPercentage),
            "quick_opening" => Ok(Self::QuickOpening),
            "modified_parabolic" => Ok(Self::ModifiedParabolic),
            other => Err(AzothError::invalid_input(
                "valve_characterization",
                format!(
                    "'{other}' is not one of linear, equal_percentage, quick_opening, \
                     modified_parabolic"
                ),
            )),
        }
    }

    /// `getOpeningFactor(percentOpening)`, the fraction of the rated `Kv` the opening gives.
    #[must_use]
    pub fn opening_factor(self, percent_opening: f64) -> f64 {
        let opening = percent_opening.clamp(0.0, 100.0);
        let x = opening / 100.0;
        match self {
            Self::Linear => x,
            Self::EqualPercentage => {
                if opening <= 0.0 {
                    1.0 / DEFAULT_RANGEABILITY
                } else if opening >= 100.0 {
                    1.0
                } else {
                    DEFAULT_RANGEABILITY.powf(x - 1.0)
                }
            }
            Self::QuickOpening => x.sqrt(),
            Self::ModifiedParabolic => x.powf(DEFAULT_PARABOLIC_EXPONENT),
        }
    }
}

/// The sizing's own configuration, as `ValveMechanicalDesign` and `ThrottlingValve` hold it.
#[derive(Debug, Clone, PartialEq)]
pub struct ValveSizingGeometry {
    /// The rated `Kv` at 100% opening, or `None` for the class's own sizing (`calcKv`).
    pub kv: Option<f64>,
    /// `percentValveOpening`, in percent.
    pub opening_percent: f64,
    /// `foulingFraction`: the effective `Kv` is `Kv * (1 - foulingFraction)`.
    pub fouling_fraction: f64,
    /// The opening characteristic.
    pub characteristic: ValveCharacteristic,
    /// `xT`.
    pub xt: f64,
    /// `allowChoked`: whether the sizing caps the flow at the choked limit.
    pub allow_choked: bool,
    /// `setChokeDiameter`: the multiphase correlations' throat, in metres.
    pub choke_diameter_m: f64,
    /// `setChokeDischargeCoefficient`: `None` is the selected model's own coefficient.
    pub discharge_coefficient: Option<f64>,
}

impl Default for ValveSizingGeometry {
    fn default() -> Self {
        Self {
            kv: None,
            opening_percent: DEFAULT_OPENING_PERCENT,
            fouling_fraction: 0.0,
            characteristic: DEFAULT_CHARACTERISTIC,
            xt: DEFAULT_XT,
            allow_choked: false,
            choke_diameter_m: DEFAULT_CHOKE_DIAMETER_M,
            discharge_coefficient: None,
        }
    }
}

/// What `calcValveSize` and its inverse solve report.
#[derive(Debug, Clone, PartialEq)]
pub struct ValveSizingReport {
    /// The rated `Kv` at 100% opening - the class's `getKv()`.
    pub kv: f64,
    /// `Cv = Kv * 1.156`.
    pub cv: f64,
    /// `Cg = Cv * 1360`.
    pub cg: f64,
    /// `x >= Fgamma * xT` on a gas valve.
    pub choked: bool,
    /// `isGasValve()`: a gas phase carrying more than half the volume.
    pub gas_valve: bool,
    /// `calculateMolarFlow()`: the mole flow the adjusted `Kv` and the drop give, mol/s.
    pub calculated_molar_flow: f64,
    /// `findOutletPressureForFixedKv`: the outlet pressure the trim gives, in bara.
    pub calculated_outlet_pressure: f64,
}

/// `isGasValve()`: a gas phase carrying more than half the volume.
///
/// # Errors
/// Whatever the inlet's flash and its two phase views raise.
pub fn gas_valve(inlet: &Stream) -> Result<bool> {
    let (mixture, _) = inlet.mixture()?;
    let flash = pt_flash(&mixture, inlet.t, inlet.p, &inlet.z)?;
    match flash.phase {
        Phase::AllVapour => Ok(true),
        Phase::AllLiquid | Phase::Trivial => Ok(false),
        _ => {
            let y = flash.vapour_fraction.unwrap_or(0.0);
            let gas = vapour_view(inlet)?;
            let liquid = liquid_view(inlet)?;
            let gas_volume = y * gas.molar_mass / gas.density;
            let liquid_volume = (1.0 - y) * liquid.molar_mass / liquid.density;
            Ok(gas_volume / (gas_volume + liquid_volume) > 0.5)
        }
    }
}

/// The inlet reads the sizing takes, in the class's own units.
struct Reads {
    p1_bara: f64,
    t_k: f64,
    vapor_flow_m3hr: f64,
    density_kg_m3: f64,
    z: f64,
    gamma: f64,
    molar_mass_gr_mol: f64,
}

fn reads(inlet: &Stream) -> Result<Reads> {
    // **The phase's own root, not a negative flash's trial phases.** NeqSim reads the gas
    // phase's density and flow off the feed's composition; the flash's trial roots are the
    // wrong side of the cubic, `0.873` against the feed's `0.920` at the 30 bara row.
    let density = system_mass_density_from_feed(inlet)?;
    let vapor_flow_m3s = system_volumetric_flow_from_feed(inlet)?;
    let molar_mass = inlet.molar_mass()?.value;
    // The cubic's own Z, not the translated volume's: the class's `getZ()` is the EOS root.
    let z = {
        let cubic_density = inlet.density()?;
        inlet.p.value * molar_mass / (cubic_density * R_J_PER_MOL_K * inlet.t.value)
    };
    let cp = inlet.molar_heat_capacity()?;
    Ok(Reads {
        p1_bara: inlet.p.value / 1.0e5,
        t_k: inlet.t.value,
        vapor_flow_m3hr: vapor_flow_m3s * 3600.0,
        density_kg_m3: density,
        z,
        gamma: cp / (cp - R_J_PER_MOL_K),
        molar_mass_gr_mol: molar_mass * 1000.0,
    })
}

/// `getActualKv` then `adjustKv`: the characteristic's fraction of the rated `Kv`, less fouling.
#[must_use]
pub fn adjusted_kv(rated_kv: f64, geometry: &ValveSizingGeometry) -> f64 {
    rated_kv
        * geometry
            .characteristic
            .opening_factor(geometry.opening_percent)
        * (1.0 - geometry.fouling_fraction)
}

/// `calcKv`: the rated `Kv` the current flow and drop require.
///
/// # Errors
/// [`AzothError::InvalidInput`] on a non-positive drop or opening, and whatever the reads raise.
pub fn calc_kv(inlet: &Stream, outlet_bara: f64, geometry: &ValveSizingGeometry) -> Result<f64> {
    let read = reads(inlet)?;
    let flow = GasOrLiquid::classify(inlet)?;
    let delta_p = read.p1_bara - outlet_bara;
    if delta_p <= 0.0 {
        return Err(AzothError::invalid_input(
            "outlet_pressure",
            format!(
                "sizing needs a positive drop, and {outlet_bara} bara against {} bara is \
                 {delta_p} bar",
                read.p1_bara
            ),
        ));
    }
    let opening_factor = geometry
        .characteristic
        .opening_factor(geometry.opening_percent);
    if opening_factor <= 0.0 {
        return Err(AzothError::invalid_input(
            "valve_opening",
            format!(
                "an opening of {}% gives an opening factor of {opening_factor}, and `calcKv` \
                 divides by it",
                geometry.opening_percent
            ),
        ));
    }
    let kv = match flow {
        GasOrLiquid::Liquid => {
            let sg = read.density_kg_m3 / 1000.0;
            read.vapor_flow_m3hr / (delta_p / sg).sqrt()
        }
        GasOrLiquid::Gas => {
            let p1_kpa = read.p1_bara * 100.0;
            let x = delta_p / read.p1_bara;
            let f_gamma = read.gamma / 1.40;
            let choked = x >= f_gamma * geometry.xt;
            let x_effective = if choked && geometry.allow_choked {
                f_gamma * geometry.xt
            } else {
                x
            };
            let y = (1.0 - x_effective / (3.0 * f_gamma * geometry.xt)).max(2.0 / 3.0);
            let standard_flow = read.vapor_flow_m3hr
                * (p1_kpa / STANDARD_PRESSURE_KPA)
                * (STANDARD_TEMPERATURE_K / read.t_k)
                / read.z;
            let x_for_scale = if choked && geometry.allow_choked {
                geometry.xt * f_gamma
            } else {
                x
            };
            standard_flow / (N9 * p1_kpa * y)
                * (read.molar_mass_gr_mol * read.t_k * read.z / x_for_scale).sqrt()
        }
    };
    Ok(kv / opening_factor)
}

/// `calculateMolarFlow` (through `calculateFlowRateFromValveOpening`): the mole flow an adjusted
/// `Kv` and the drop give, mol/s.
///
/// # Errors
/// Whatever the reads raise. A non-positive drop answers zero, which is the class's own branch.
pub fn calculated_molar_flow(
    inlet: &Stream,
    outlet_bara: f64,
    adjusted: f64,
    geometry: &ValveSizingGeometry,
) -> Result<f64> {
    let read = reads(inlet)?;
    let delta_p = read.p1_bara - outlet_bara;
    if delta_p <= 0.0 {
        return Ok(0.0);
    }
    let flow_m3s = match GasOrLiquid::classify(inlet)? {
        GasOrLiquid::Liquid => {
            let sg = read.density_kg_m3 / 1000.0;
            adjusted * (delta_p / sg).sqrt() / 3600.0
        }
        GasOrLiquid::Gas => {
            let p1_kpa = read.p1_bara * 100.0;
            let x = delta_p / read.p1_bara;
            let f_gamma = read.gamma / 1.40;
            let choked = x >= f_gamma * geometry.xt;
            let x_effective = if choked && geometry.allow_choked {
                f_gamma * geometry.xt
            } else {
                x
            };
            let y = (1.0 - x_effective / (3.0 * f_gamma * geometry.xt)).max(2.0 / 3.0);
            let x_for_scale = if choked && geometry.allow_choked {
                geometry.xt * f_gamma
            } else {
                x
            };
            let standard_flow = adjusted * N9 * p1_kpa * y
                / (read.molar_mass_gr_mol * read.t_k * read.z / x_for_scale).sqrt();
            let actual = standard_flow
                * (STANDARD_PRESSURE_KPA / p1_kpa)
                * (read.t_k / STANDARD_TEMPERATURE_K)
                * read.z;
            actual / 3600.0
        }
    };
    Ok(flow_m3s * read.density_kg_m3 / (read.molar_mass_gr_mol / 1000.0))
}

/// `findOutletPressureForFixedKv`: the outlet pressure an adjusted `Kv` gives, in bara.
///
/// # Errors
/// [`AzothError::InvalidInput`] on a non-positive adjusted `Kv`, and whatever the reads raise.
pub fn calculated_outlet_pressure(
    inlet: &Stream,
    adjusted: f64,
    geometry: &ValveSizingGeometry,
) -> Result<f64> {
    if adjusted <= 0.0 {
        return Err(AzothError::invalid_input(
            "kv",
            format!("an adjusted Kv of {adjusted} gives no outlet pressure"),
        ));
    }
    let read = reads(inlet)?;
    if !matches!(GasOrLiquid::classify(inlet)?, GasOrLiquid::Gas) {
        let sg = read.density_kg_m3 / 1000.0;
        let q_m3hr = read.vapor_flow_m3hr;
        let delta_p_bar = (q_m3hr / adjusted).powi(2) * sg;
        return Ok(read.p1_bara - delta_p_bar);
    }
    let p1_kpa = read.p1_bara * 100.0;
    let standard_flow = read.vapor_flow_m3hr
        * (p1_kpa / STANDARD_PRESSURE_KPA)
        * (STANDARD_TEMPERATURE_K / read.t_k)
        / read.z;
    let f_gamma = read.gamma / 1.40;
    let mut low = 0.1_f64;
    let mut high = read.p1_bara - 0.001;
    let mut mid = (low + high) / 2.0;
    for _ in 0..MAX_BISECTION_ITERATIONS {
        mid = (low + high) / 2.0;
        let p2_kpa = mid * 100.0;
        let x = (p1_kpa - p2_kpa) / p1_kpa;
        let choked = x >= f_gamma * geometry.xt;
        let x_effective = if choked && geometry.allow_choked {
            f_gamma * geometry.xt
        } else {
            x
        };
        let y = (1.0 - x_effective / (3.0 * f_gamma * geometry.xt)).max(2.0 / 3.0);
        let x_for_scale = if choked && geometry.allow_choked {
            geometry.xt * f_gamma
        } else {
            x
        };
        let kv = standard_flow / (N9 * p1_kpa * y)
            * (read.molar_mass_gr_mol * read.t_k * read.z / x_for_scale).sqrt();
        if (kv - adjusted).abs() / adjusted < BISECTION_TOLERANCE {
            break;
        }
        if kv > adjusted {
            high = mid;
        } else {
            low = mid;
        }
    }
    Ok(mid)
}

enum GasOrLiquid {
    Gas,
    Liquid,
}

impl GasOrLiquid {
    fn classify(inlet: &Stream) -> Result<Self> {
        Ok(if gas_valve(inlet)? {
            Self::Gas
        } else {
            Self::Liquid
        })
    }
}

/// The whole report: the sized (or stated) `Kv`, the two derived coefficients, and the inverse
/// solves the model publishes.
///
/// # Errors
/// Whatever [`calc_kv`], [`calculated_molar_flow`] or [`calculated_outlet_pressure`] raise.
pub fn sizing_report(
    inlet: &Stream,
    outlet_bara: f64,
    geometry: &ValveSizingGeometry,
) -> Result<ValveSizingReport> {
    let gas = gas_valve(inlet)?;
    let read = reads(inlet)?;
    let kv = match geometry.kv {
        Some(kv) => kv,
        // **A non-positive drop has no sizing**, and the class's own `calcKv` answers `NaN`
        // there: a stated outlet above the inlet is the `acceptNegativeDP` row, whose stream
        // state is still a state. So the report carries the NaNs rather than refusing.
        None if read.p1_bara - outlet_bara > 0.0 => calc_kv(inlet, outlet_bara, geometry)?,
        None => f64::NAN,
    };
    if !kv.is_finite() {
        return Ok(ValveSizingReport {
            kv,
            cv: f64::NAN,
            cg: f64::NAN,
            choked: false,
            gas_valve: gas,
            calculated_molar_flow: 0.0,
            calculated_outlet_pressure: f64::NAN,
        });
    }
    let adjusted = adjusted_kv(kv, geometry);
    let choked = if gas {
        let x = (read.p1_bara - outlet_bara) / read.p1_bara;
        x >= (read.gamma / 1.40) * geometry.xt
    } else {
        false
    };
    Ok(ValveSizingReport {
        kv,
        cv: kv * KV_TO_CV_FACTOR,
        cg: kv * KV_TO_CV_FACTOR * CG_PER_CV,
        choked,
        gas_valve: gas,
        calculated_molar_flow: calculated_molar_flow(inlet, outlet_bara, adjusted, geometry)?,
        calculated_outlet_pressure: calculated_outlet_pressure(inlet, adjusted, geometry)?,
    })
}

/// The nine `setValveSizingStandard` values, and the factory's mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizingStandard {
    /// `ControlValveSizing`, the `ValveMechanicalDesign` default.
    Default,
    /// `ControlValveSizing_IEC_60534`.
    Iec60534,
    /// `ControlValveSizing_IEC_60534_full`.
    Iec60534Full,
    /// `ControlValveSizing_simple`, the production choke.
    ProdChoke,
    /// `ControlValveSizing_MultiphaseChoke` with `SachdevaChokeFlow`.
    Sachdeva,
    /// The same class with `GilbertChokeFlow`'s Gilbert constants.
    Gilbert,
    /// The same class with `GilbertChokeFlow`'s Baxendell constants.
    Baxendell,
    /// The same class with `GilbertChokeFlow`'s Ros constants.
    Ros,
    /// The same class with `GilbertChokeFlow`'s Achong constants.
    Achong,
}

impl SizingStandard {
    /// The spec spelling.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Some(match name {
            "default" => Self::Default,
            "iec_60534" => Self::Iec60534,
            "iec_60534_full" => Self::Iec60534Full,
            "prod_choke" => Self::ProdChoke,
            "sachdeva" => Self::Sachdeva,
            "gilbert" => Self::Gilbert,
            "baxendell" => Self::Baxendell,
            "ros" => Self::Ros,
            "achong" => Self::Achong,
            _ => return None,
        })
    }

    /// `setValveSizingStandard`'s constructor state: the default class starts `false`, and
    /// the eight it selects start `true`.
    #[must_use]
    pub fn default_allow_choked(self) -> bool {
        !matches!(self, Self::Default)
    }

    /// Whether this route reads the flash's labelled phases rather than one root.
    #[must_use]
    pub fn is_choke(self) -> bool {
        matches!(
            self,
            Self::Sachdeva | Self::Gilbert | Self::Baxendell | Self::Ros | Self::Achong
        )
    }

    /// The class the standard selects.
    #[must_use]
    pub fn class_name(self) -> &'static str {
        match self {
            Self::Default => "ControlValveSizing",
            Self::Iec60534 => "ControlValveSizing_IEC_60534",
            Self::Iec60534Full => "ControlValveSizing_IEC_60534_full",
            Self::ProdChoke => "ControlValveSizing_simple",
            Self::Sachdeva | Self::Gilbert | Self::Baxendell | Self::Ros | Self::Achong => {
                "ControlValveSizing_MultiphaseChoke"
            }
        }
    }
}

/// The single-phase routes read one root, and refuse a two-phase state by name.
fn require_single_phase(inlet: &Stream, standard: SizingStandard) -> Result<()> {
    let (mixture, _) = inlet.mixture()?;
    let flash = pt_flash(&mixture, inlet.t, inlet.p, &inlet.z)?;
    if matches!(flash.phase, Phase::TwoPhase) {
        return Err(AzothError::invalid_input(
            "inlet_z",
            format!(
                "the {} standard reads a single phase's root, and this inlet is two phases at \
                 {} K and {} Pa",
                standard.class_name(),
                inlet.t.value,
                inlet.p.value
            ),
        ));
    }
    Ok(())
}

/// The IEC class's reads, in its own units.
struct IecReads {
    reads: Reads,
    /// `hasPhaseType(PhaseType.GAS)`, the IEC's own branch selector.
    gas: bool,
    /// `getPhase(0).getPseudoCriticalPressure()`: the mole-fraction-weighted `Pc`, in pascals.
    pseudo_critical_pa: f64,
}

fn iec_reads(inlet: &Stream) -> Result<IecReads> {
    let read = reads(inlet)?;
    let (mixture, _) = inlet.mixture()?;
    let flash = pt_flash(&mixture, inlet.t, inlet.p, &inlet.z)?;
    let pseudo_critical_pa = mixture
        .components()
        .iter()
        .zip(&inlet.z)
        .map(|(component, fraction)| fraction * component.pc.value)
        .sum();
    Ok(IecReads {
        reads: read,
        gas: matches!(flash.phase, Phase::AllVapour | Phase::TwoPhase),
        pseudo_critical_pa,
    })
}

fn opening_factor(geometry: &ValveSizingGeometry) -> f64 {
    geometry
        .characteristic
        .opening_factor(geometry.opening_percent)
}

/// `sizeControlValveGas`'s `Kv` at 100 % opening, before the opening division.
fn iec_gas_kv(read: &IecReads, p1_pa: f64, p2_pa: f64, xt: f64, allow_choked: bool) -> (f64, bool) {
    let r = &read.reads;
    let loc_p1 = p1_pa / 1000.0;
    let loc_p2 = p2_pa / 1000.0;
    let d_p = loc_p1 - loc_p2;
    let x = d_p / loc_p1;
    let f_gamma = r.gamma / 1.40;
    let y = (1.0 - x / (3.0 * f_gamma * xt)).max(2.0 / 3.0);
    let choked = x >= f_gamma * xt;
    let q_std =
        r.vapor_flow_m3hr * (loc_p1 / STANDARD_PRESSURE_KPA) * (STANDARD_TEMPERATURE_K / r.t_k)
            / r.z;
    let kv = if choked && allow_choked {
        q_std / (IEC_N9 * loc_p1 * y) * (r.molar_mass_gr_mol * r.t_k * r.z / (xt * f_gamma)).sqrt()
    } else {
        q_std / (IEC_N9 * loc_p1 * y) * (r.molar_mass_gr_mol * r.t_k * r.z / x).sqrt()
    };
    (kv, choked)
}

/// `sizeControlValveLiquid`'s `Kv` at 100 % opening, before the opening division.
fn iec_liquid_kv(
    read: &IecReads,
    psat_pa: f64,
    pc_pa: f64,
    p1_pa: f64,
    p2_pa: f64,
    allow_choked: bool,
) -> (f64, bool) {
    let r = &read.reads;
    let loc_p1 = p1_pa / 1000.0;
    let loc_p2 = p2_pa / 1000.0;
    let loc_psat = psat_pa / 1000.0;
    let loc_pc = pc_pa / 1000.0;
    let d_p = loc_p1 - loc_p2;
    let ff = 0.96 - 0.28 * (loc_psat / loc_pc).sqrt();
    let choked = d_p >= IEC_FL * IEC_FL * (loc_p1 - ff * loc_psat);
    let q_loc = r.vapor_flow_m3hr;
    let kv = if choked && allow_choked {
        q_loc / (IEC_N1 * IEC_FL * ((loc_p1 - ff * loc_psat) * IEC_RHO0 / r.density_kg_m3).sqrt())
    } else {
        q_loc / (IEC_N1 * (d_p * IEC_RHO0 / r.density_kg_m3).sqrt())
    };
    (kv, choked)
}

/// `calculateFlowRateFromKvAndValveOpeningGas`: actual m3/s.
fn iec_gas_flow_m3s(
    read: &IecReads,
    adjusted: f64,
    p1_pa: f64,
    p2_pa: f64,
    xt: f64,
    allow_choked: bool,
) -> f64 {
    let r = &read.reads;
    let loc_p1 = p1_pa / 1000.0;
    let loc_p2 = p2_pa / 1000.0;
    let d_p = loc_p1 - loc_p2;
    let x = d_p / loc_p1;
    let f_gamma = r.gamma / 1.40;
    let y = (1.0 - x / (3.0 * f_gamma * xt)).max(2.0 / 3.0);
    let choked = x >= f_gamma * xt;
    let q_std = if choked && allow_choked {
        adjusted * IEC_N9 * loc_p1 * y / (r.molar_mass_gr_mol * r.t_k * r.z / (xt * f_gamma)).sqrt()
    } else {
        adjusted * IEC_N9 * loc_p1 * y / (r.molar_mass_gr_mol * r.t_k * r.z / x).sqrt()
    };
    let q = q_std * (STANDARD_PRESSURE_KPA / loc_p1) * (r.t_k / STANDARD_TEMPERATURE_K) * r.z;
    q / 3600.0
}

/// `calculateFlowRateFromValveOpeningLiquid`: actual m3/s.
fn iec_liquid_flow_m3s(
    read: &IecReads,
    adjusted: f64,
    psat_pa: f64,
    pc_pa: f64,
    p1_pa: f64,
    p2_pa: f64,
    allow_choked: bool,
) -> f64 {
    let r = &read.reads;
    let loc_p1 = p1_pa / 1000.0;
    let loc_p2 = p2_pa / 1000.0;
    let loc_psat = psat_pa / 1000.0;
    let d_p = loc_p1 - loc_p2;
    let ff = 0.96 - 0.28 * (loc_psat / (pc_pa / 1000.0)).sqrt();
    let choked = d_p >= IEC_FL * IEC_FL * (loc_p1 - ff * loc_psat);
    let q_loc = if choked && allow_choked {
        IEC_N1 * adjusted * IEC_FL * ((loc_p1 - ff * loc_psat) * IEC_RHO0 / r.density_kg_m3).sqrt()
    } else {
        IEC_N1 * adjusted * (d_p * IEC_RHO0 / r.density_kg_m3).sqrt()
    };
    q_loc / 3600.0
}

/// `findOutletPressureForFixedKvGas`: the Pa the bisection lands on.
fn iec_gas_pressure(read: &IecReads, adjusted: f64, p1_pa: f64, xt: f64, allow: bool) -> f64 {
    let mut low = 1.0_f64;
    let mut high = p1_pa - 1.0e-4;
    let mut mid = 0.5 * (low + high);
    for _ in 0..50 {
        let (required, _) = iec_gas_kv(read, p1_pa, mid, xt, allow);
        if required < adjusted {
            low = mid;
        } else {
            high = mid;
        }
        let old = mid;
        mid = 0.5 * (low + high);
        if mid > 1.0e-9 && (mid - old).abs() / mid < 1.0e-6 {
            break;
        }
    }
    mid
}

/// `findOutletPressureForFixedKvLiquid`: the kPa bisection, returned in Pa.
fn iec_liquid_pressure(
    read: &IecReads,
    adjusted: f64,
    opening: f64,
    allow: bool,
    divide_by_opening: bool,
) -> f64 {
    let p1_pa = read.reads.p1_bara * 1.0e5;
    let psat_pa = p1_pa;
    let pc_pa = read.pseudo_critical_pa;
    let mut low = 0.1_f64;
    let mut high = p1_pa / 1000.0 - 1.0e-4;
    let mut mid = 0.5 * (low + high);
    for _ in 0..50 {
        let (required, _) = iec_liquid_kv(read, psat_pa, pc_pa, p1_pa, mid * 1000.0, allow);
        let required = if divide_by_opening {
            required / opening
        } else {
            required
        };
        if required < adjusted {
            low = mid;
        } else {
            high = mid;
        }
        let old = mid;
        mid = 0.5 * (low + high);
        if (mid - old).abs() < 1.0e-6 {
            break;
        }
    }
    mid * 1000.0
}

/// `calculateFlowRateFromValveOpeningGas_full`: the full class's Q bisection, m3/s.
fn full_gas_flow_m3s_at(
    read: &IecReads,
    adjusted: f64,
    outlet_bara: f64,
    xt: f64,
    allow: bool,
) -> f64 {
    let r = &read.reads;
    let p1_pa = r.p1_bara * 1.0e5;
    let p2_pa = outlet_bara * 1.0e5;
    let x = if p1_pa > p2_pa {
        (p1_pa - p2_pa) / p1_pa
    } else {
        1.0e-3
    };
    let y = 0.8_f64;
    let q_high_m3h =
        IEC_N9 * adjusted * (p1_pa / 1000.0) * y / (r.molar_mass_gr_mol * r.t_k * r.z / x).sqrt();
    let mut q_low = 0.0_f64;
    let mut q_high = q_high_m3h / 3600.0 * 2.0;
    let mut q_mid = 0.0_f64;
    for _ in 0..20 {
        q_mid = 0.5 * (q_low + q_high);
        if q_mid < 1.0e-9 {
            break;
        }
        // The full sizing's early return at `D = 0` is the initial Kv, no opening division.
        let guessed = iec_reads_with_flow(read, q_mid);
        let (required, _) = iec_gas_kv(&guessed, p1_pa, p2_pa, xt, allow);
        if required < adjusted {
            q_low = q_mid;
        } else {
            q_high = q_mid;
        }
        if (q_high - q_low).abs() < 1.0e-6 {
            break;
        }
    }
    q_mid
}

/// The full class's liquid Q bisection at a stated outlet pressure, m3/s.
fn full_liquid_flow_m3s_at(read: &IecReads, adjusted: f64, outlet_bara: f64, allow: bool) -> f64 {
    let r = &read.reads;
    let p1_pa = r.p1_bara * 1.0e5;
    let p2_pa = outlet_bara * 1.0e5;
    let d_p_kpa = if p1_pa > p2_pa {
        (p1_pa - p2_pa) / 1000.0
    } else {
        1.0e-3
    };
    let q_high_m3h = IEC_N1 * adjusted * (d_p_kpa * IEC_RHO0 / r.density_kg_m3).sqrt();
    let mut q_low = 0.0_f64;
    let mut q_high = q_high_m3h / 3600.0 * 2.0;
    let mut q_mid = 0.0_f64;
    for _ in 0..20 {
        q_mid = 0.5 * (q_low + q_high);
        if q_mid < 1.0e-9 {
            break;
        }
        let guessed = iec_reads_with_flow(read, q_mid);
        let (required, _) = iec_liquid_kv(
            &guessed,
            p1_pa,
            read.pseudo_critical_pa,
            p1_pa,
            p2_pa,
            allow,
        );
        if required < adjusted {
            q_low = q_mid;
        } else {
            q_high = q_mid;
        }
        if (q_high - q_low).abs() < 1.0e-6 {
            break;
        }
    }
    q_mid
}

fn iec_reads_with_flow(read: &IecReads, flow_m3s: f64) -> IecReads {
    IecReads {
        reads: Reads {
            p1_bara: read.reads.p1_bara,
            t_k: read.reads.t_k,
            vapor_flow_m3hr: flow_m3s * 3600.0,
            density_kg_m3: read.reads.density_kg_m3,
            z: read.reads.z,
            gamma: read.reads.gamma,
            molar_mass_gr_mol: read.reads.molar_mass_gr_mol,
        },
        gas: read.gas,
        pseudo_critical_pa: read.pseudo_critical_pa,
    }
}

/// The IEC class's report, on the reachable `D = 0` branch when `full`.
fn iec_report(
    inlet: &Stream,
    outlet_bara: f64,
    geometry: &ValveSizingGeometry,
    full: bool,
    standard: SizingStandard,
) -> Result<ValveSizingReport> {
    require_single_phase(inlet, standard)?;
    let read = iec_reads(inlet)?;
    let opening = opening_factor(geometry);
    let p1_pa = read.reads.p1_bara * 1.0e5;
    let p2_pa = outlet_bara * 1.0e5;
    let (raw_kv, choked) = if read.gas {
        iec_gas_kv(&read, p1_pa, p2_pa, geometry.xt, geometry.allow_choked)
    } else {
        // The forward passes the system `Z` into the `Psat` slot and the unset `getPC()` as
        // `Pc`; that is the class's own call, reproduced.
        iec_liquid_kv(
            &read,
            read.reads.z,
            0.0,
            p1_pa,
            p2_pa,
            geometry.allow_choked,
        )
    };
    let kv = match geometry.kv {
        Some(kv) => kv,
        None if full => raw_kv,
        None => raw_kv / opening,
    };
    if !kv.is_finite() {
        return Ok(nan_report(gas_valve(inlet)?));
    }
    let adjusted = adjusted_kv(kv, geometry);
    let flow_m3s = if full {
        if read.gas {
            full_gas_flow_m3s_at(
                &read,
                adjusted,
                outlet_bara,
                geometry.xt,
                geometry.allow_choked,
            )
        } else {
            full_liquid_flow_m3s_at(&read, adjusted, outlet_bara, geometry.allow_choked)
        }
    } else if read.gas {
        iec_gas_flow_m3s(
            &read,
            adjusted,
            p1_pa,
            p2_pa,
            geometry.xt,
            geometry.allow_choked,
        )
    } else {
        iec_liquid_flow_m3s(
            &read,
            adjusted,
            p1_pa,
            read.pseudo_critical_pa,
            p1_pa,
            p2_pa,
            geometry.allow_choked,
        )
    };
    let molar_mass_kg = read.reads.molar_mass_gr_mol / 1000.0;
    let pressure_pa = if read.gas {
        iec_gas_pressure(&read, adjusted, p1_pa, geometry.xt, geometry.allow_choked)
    } else {
        iec_liquid_pressure(&read, adjusted, opening, geometry.allow_choked, !full)
    };
    Ok(ValveSizingReport {
        kv,
        cv: kv * KV_TO_CV_FACTOR,
        cg: kv * KV_TO_CV_FACTOR * CG_PER_CV,
        choked,
        gas_valve: gas_valve(inlet)?,
        calculated_molar_flow: flow_m3s * read.reads.density_kg_m3 / molar_mass_kg,
        calculated_outlet_pressure: pressure_pa / 1.0e5,
    })
}

fn nan_report(gas: bool) -> ValveSizingReport {
    ValveSizingReport {
        kv: f64::NAN,
        cv: f64::NAN,
        cg: f64::NAN,
        choked: false,
        gas_valve: gas,
        calculated_molar_flow: 0.0,
        calculated_outlet_pressure: f64::NAN,
    }
}

/// `ControlValveSizing_simple`, the production choke.
fn simple_report(
    inlet: &Stream,
    outlet_bara: f64,
    geometry: &ValveSizingGeometry,
    standard: SizingStandard,
) -> Result<ValveSizingReport> {
    require_single_phase(inlet, standard)?;
    let read = reads(inlet)?;
    let gas = gas_valve(inlet)?;
    let delta_p = read.p1_bara - outlet_bara;
    let kv = match geometry.kv {
        Some(kv) => kv,
        None if delta_p > 0.0 => {
            let base = calc_kv(inlet, outlet_bara, geometry)?;
            if gas { base / SIMPLE_CD } else { base }
        }
        None => f64::NAN,
    };
    if !kv.is_finite() {
        return Ok(nan_report(gas));
    }
    let adjusted = adjusted_kv(kv, geometry);
    let choked = if gas {
        let x = delta_p / read.p1_bara;
        x >= (read.gamma / 1.40) * geometry.xt
    } else {
        false
    };
    let flow_m3s = if gas {
        // `calculateMolarFlow` multiplies the adjusted Kv by `Cd` before inverting.
        let effective = adjusted * SIMPLE_CD;
        let p1_kpa = read.p1_bara * 100.0;
        let x = delta_p / read.p1_bara;
        let f_gamma = read.gamma / 1.40;
        let choked = x >= f_gamma * geometry.xt;
        let x_effective = if choked && geometry.allow_choked {
            f_gamma * geometry.xt
        } else {
            x
        };
        let y = (1.0 - x_effective / (3.0 * f_gamma * geometry.xt)).max(2.0 / 3.0);
        let x_for_scale = if choked && geometry.allow_choked {
            geometry.xt * f_gamma
        } else {
            x
        };
        let standard_flow = effective * N9 * p1_kpa * y
            / (read.molar_mass_gr_mol * read.t_k * read.z / x_for_scale).sqrt();
        let actual = standard_flow
            * (STANDARD_PRESSURE_KPA / p1_kpa)
            * (read.t_k / STANDARD_TEMPERATURE_K)
            * read.z;
        actual / 3600.0
    } else {
        default_liquid_flow_m3s(&read, adjusted, delta_p)
    };
    let pressure_bara = if gas {
        simple_gas_pressure(&read, adjusted, geometry)?
    } else {
        calculated_outlet_pressure(inlet, adjusted, geometry)?
    };
    Ok(ValveSizingReport {
        kv,
        cv: kv * KV_TO_CV_FACTOR,
        cg: kv * KV_TO_CV_FACTOR * CG_PER_CV,
        choked,
        gas_valve: gas,
        calculated_molar_flow: flow_m3s * read.density_kg_m3 / (read.molar_mass_gr_mol / 1000.0),
        calculated_outlet_pressure: pressure_bara,
    })
}

fn default_liquid_flow_m3s(read: &Reads, adjusted: f64, delta_p: f64) -> f64 {
    let sg = read.density_kg_m3 / 1000.0;
    adjusted * (delta_p / sg).sqrt() / 3600.0
}

fn simple_gas_pressure(read: &Reads, adjusted: f64, geometry: &ValveSizingGeometry) -> Result<f64> {
    if adjusted <= 0.0 {
        return Err(AzothError::invalid_input(
            "kv",
            format!("an adjusted Kv of {adjusted} gives no outlet pressure"),
        ));
    }
    let p1_kpa = read.p1_bara * 100.0;
    let standard_flow = read.vapor_flow_m3hr
        * (p1_kpa / STANDARD_PRESSURE_KPA)
        * (STANDARD_TEMPERATURE_K / read.t_k)
        / read.z;
    let f_gamma = read.gamma / 1.40;
    let mut low = 0.1_f64;
    let mut high = read.p1_bara - 0.001;
    let mut mid = 0.5 * (low + high);
    for _ in 0..100 {
        mid = 0.5 * (low + high);
        let d_p_kpa = (read.p1_bara - mid) * 100.0;
        let x = d_p_kpa / p1_kpa;
        let choked = x >= f_gamma * geometry.xt;
        let x_effective = if choked && geometry.allow_choked {
            f_gamma * geometry.xt
        } else {
            x
        };
        let y = (1.0 - x_effective / (3.0 * f_gamma * geometry.xt)).max(2.0 / 3.0);
        let calc_kv = if choked && geometry.allow_choked {
            standard_flow / (N9 * p1_kpa * y)
                * (read.molar_mass_gr_mol * read.t_k * read.z / (geometry.xt * f_gamma)).sqrt()
                / SIMPLE_CD
        } else {
            standard_flow / (N9 * p1_kpa * y)
                * (read.molar_mass_gr_mol * read.t_k * read.z / x).sqrt()
                / SIMPLE_CD
        };
        if (calc_kv - adjusted).abs() / adjusted < BISECTION_TOLERANCE {
            break;
        }
        if calc_kv > adjusted {
            high = mid;
        } else {
            low = mid;
        }
    }
    Ok(mid)
}

/// The class's `getPhaseIndex("gas")` / `("oil")` / `("aqueous")` reads, at the inlet.
struct ChokeReads {
    p1_pa: f64,
    /// `fluid.getDensity("kg/m3")`, the whole stream's.
    system_density: f64,
    /// `fluid.getMolarMass("kg/mol")`.
    molar_mass_kg: f64,
    /// `calculateGasQuality`: the gas mass fraction.
    gas_quality: f64,
    /// `fluid.getGamma2()`: the phase-weighted `Cp/(Cp - R)`.
    gamma: f64,
    /// The gas phase's density, or the class's ideal-gas estimate when there is no gas.
    rho_gas: f64,
    /// `getLiquidDensity`: the oil's, the aqueous's, or the model's own default.
    rho_liquid: f64,
    /// `calculateGLR`, Sm3/Sm3; `+infinity` with no liquid.
    glr: f64,
}

fn choke_reads(inlet: &Stream, standard: SizingStandard) -> Result<ChokeReads> {
    let (mixture, ideal_gas) = inlet.mixture()?;
    let flash = pt_flash(&mixture, inlet.t, inlet.p, &inlet.z)?;
    let labelled = labelled_phases_at_from_feed(&mixture, inlet.t, inlet.p, &inlet.z, &flash)?;
    let total_n = inlet.n;
    let vapour_fraction = flash.vapour_fraction.unwrap_or(0.0);
    let share = |label: PhaseLabel| -> f64 {
        match flash.phase {
            Phase::AllVapour => {
                if label == PhaseLabel::Gas {
                    1.0
                } else {
                    0.0
                }
            }
            Phase::AllLiquid | Phase::Trivial => {
                if label == PhaseLabel::Gas {
                    0.0
                } else {
                    1.0
                }
            }
            Phase::TwoPhase => {
                if label == PhaseLabel::Gas {
                    vapour_fraction
                } else {
                    1.0 - vapour_fraction
                }
            }
        }
    };

    let mut gas: Option<(Vec<f64>, f64, f64)> = None;
    let mut liquid: Option<(Vec<f64>, f64, f64, PhaseLabel)> = None;
    for (label, composition, root) in &labelled {
        let molar_mass = molar_mass_of(&mixture, composition)?;
        let density = mass_density(&mixture, inlet.t, inlet.p, composition, *root, molar_mass)?;
        match label {
            PhaseLabel::Gas => gas = Some((composition.clone(), *root, density)),
            _ => liquid = Some((composition.clone(), *root, density, *label)),
        }
    }

    let gas_mass = gas
        .as_ref()
        .map(|(composition, _, _)| {
            Ok::<_, AzothError>(
                share(PhaseLabel::Gas) * total_n * molar_mass_of(&mixture, composition)?,
            )
        })
        .transpose()?
        .unwrap_or(0.0);
    let liquid_mass = liquid
        .as_ref()
        .map(|(composition, _, _, _)| {
            Ok::<_, AzothError>(
                (1.0 - share(PhaseLabel::Gas)) * total_n * molar_mass_of(&mixture, composition)?,
            )
        })
        .transpose()?
        .unwrap_or(0.0);
    let gas_quality = if gas.is_none() {
        0.0
    } else if liquid.is_none() {
        1.0
    } else {
        gas_mass / (gas_mass + liquid_mass)
    };

    let system_z = {
        let volume = system_volumetric_flow_from_feed(inlet)? / inlet.n;
        inlet.p.value * volume / (R_J_PER_MOL_K * inlet.t.value)
    };
    let rho_gas = match gas.as_ref() {
        Some((_, _, density)) => *density,
        None => {
            inlet.p.value * inlet.molar_mass()?.value / (system_z * R_J_PER_MOL_K * inlet.t.value)
        }
    };
    let liquid_default = if standard == SizingStandard::Sachdeva {
        1000.0
    } else {
        800.0
    };
    let rho_liquid = liquid
        .as_ref()
        .map(|(_, _, density, _)| *density)
        .unwrap_or(liquid_default);

    let cp = {
        let mut total = 0.0;
        for (label, composition, root) in &labelled {
            let state =
                molar_enthalpy_entropy(&mixture, &ideal_gas, inlet.t, inlet.p, composition, *root)?;
            total += share(*label) * state.cp.value;
        }
        total
    };
    let gamma = cp / (cp - R_J_PER_MOL_K);

    let gas_z = gas
        .as_ref()
        .map(|(_, root, _)| {
            let volume = pr_molar_volume(*root, inlet.t, inlet.p)?.v.value;
            Ok::<_, AzothError>(inlet.p.value * volume / (R_J_PER_MOL_K * inlet.t.value))
        })
        .transpose()?
        .unwrap_or(system_z);
    let gas_volume = gas.as_ref().map(|(composition, _, density)| {
        let moles = share(PhaseLabel::Gas) * total_n;
        Ok::<_, AzothError>(moles * molar_mass_of(&mixture, composition)? / density)
    });
    let liquid_volume = liquid.as_ref().map(|(composition, _, density, _)| {
        let moles = (1.0 - share(PhaseLabel::Gas)) * total_n;
        Ok::<_, AzothError>(moles * molar_mass_of(&mixture, composition)? / density)
    });
    let gas_std_volume = match gas_volume.transpose()? {
        Some(volume) => volume * gas_z * inlet.t.value / 288.15 * 101325.0 / inlet.p.value,
        None => 0.0,
    };
    let liquid_volume = liquid_volume.transpose()?.unwrap_or(0.0);
    let glr = if liquid_volume > 0.0 {
        gas_std_volume / liquid_volume
    } else {
        f64::INFINITY
    };

    Ok(ChokeReads {
        p1_pa: inlet.p.value,
        system_density: system_mass_density_from_feed(inlet)?,
        molar_mass_kg: inlet.molar_mass()?.value,
        gas_quality,
        gamma,
        rho_gas,
        rho_liquid,
        glr,
    })
}

fn homogeneous_density(gas_quality: f64, rho_gas: f64, rho_liquid: f64) -> f64 {
    if gas_quality >= 1.0 {
        return rho_gas;
    }
    if gas_quality <= 0.0 {
        return rho_liquid;
    }
    1.0 / (gas_quality / rho_gas + (1.0 - gas_quality) / rho_liquid)
}

fn isentropic_critical_ratio(gamma: f64) -> f64 {
    (2.0 / (gamma + 1.0)).powf(gamma / (gamma - 1.0))
}

fn sachdeva_critical_ratio(gas_quality: f64, gamma: f64) -> f64 {
    if gas_quality <= 0.001 {
        return 0.90;
    }
    if gas_quality >= 0.999 {
        return isentropic_critical_ratio(gamma);
    }
    let xg = gas_quality.clamp(0.02, 0.98);
    (0.5847 - 0.0227 * xg.ln()).clamp(0.3, 0.9)
}

fn gilbert_critical_ratio(gas_quality: f64, gamma: f64) -> f64 {
    if gas_quality <= 0.01 {
        return 0.85;
    }
    if gas_quality >= 0.99 {
        return isentropic_critical_ratio(gamma);
    }
    0.5847 - 0.0227 * gas_quality.max(0.02).ln()
}

fn sachdeva_gas_only(read: &ChokeReads, p2_pa: f64, cd: f64, area: f64) -> f64 {
    let gamma = read.gamma;
    let critical = isentropic_critical_ratio(gamma);
    let ratio = p2_pa / read.p1_pa;
    if ratio <= critical {
        let factor = (2.0 / (gamma + 1.0)).powf((gamma + 1.0) / (2.0 * (gamma - 1.0)));
        cd * area * (gamma * read.p1_pa * read.rho_gas).sqrt() * factor
    } else {
        let term1 = 2.0 * gamma / (gamma - 1.0);
        let term2 = ratio.powf(2.0 / gamma) - ratio.powf((gamma + 1.0) / gamma);
        cd * area * (term1 * read.p1_pa * read.rho_gas * term2).sqrt()
    }
}

fn sachdeva_mass_flow(
    read: &ChokeReads,
    p2_pa: f64,
    geometry: &ValveSizingGeometry,
    diameter: f64,
) -> f64 {
    let cd = geometry.discharge_coefficient.unwrap_or(0.84);
    let area = std::f64::consts::PI * diameter * diameter / 4.0;
    let xg = read.gas_quality;
    if xg <= 0.001 {
        let delta_p = (read.p1_pa - p2_pa).max(0.0);
        return cd * area * (2.0 * read.rho_liquid * delta_p).sqrt();
    }
    if xg >= 0.999 {
        return sachdeva_gas_only(read, p2_pa, cd, area);
    }
    let y = (p2_pa / read.p1_pa).max(sachdeva_critical_ratio(xg, read.gamma));
    let n = 1.2_f64;
    let rho_gas2 = read.rho_gas * y.powf(1.0 / n);
    let area_ratio_sq = (diameter / 0.1016).powi(4);
    let xl = 1.0 - xg;
    let kinetic = (1.0 / (y * y) - area_ratio_sq)
        * (xg / (rho_gas2 * rho_gas2) + xl / (read.rho_liquid * read.rho_liquid));
    let liquid_work = 2.0 * xl / read.rho_liquid * (1.0 - y);
    let gas_work = 2.0 * xg / ((n - 1.0) * read.rho_gas) * (1.0 - y.powf((n - 1.0) / n));
    let denominator = kinetic + liquid_work + gas_work;
    if denominator <= 0.0 {
        return 0.0;
    }
    let rho_mix = homogeneous_density(xg, read.rho_gas, read.rho_liquid);
    cd * area * (2.0 * read.p1_pa * rho_mix / denominator).sqrt()
}

fn gilbert_mass_flow(
    read: &ChokeReads,
    p2_pa: f64,
    standard: SizingStandard,
    diameter: f64,
) -> f64 {
    let (constant, diameter_exponent, glr_exponent) = match standard {
        SizingStandard::Baxendell => (9.56, 1.93, 0.546),
        SizingStandard::Ros => (17.4, 2.0, 0.5),
        SizingStandard::Achong => (3.82, 1.88, 0.65),
        _ => (10.0, 1.89, 0.546),
    };
    let area = std::f64::consts::PI * diameter * diameter / 4.0;
    let p_psig = read.p1_pa / 6894.76 - 14.696;
    let d_64ths = diameter / 0.0254 * 64.0;
    let glr_scf_stb = read.glr * 35.3147 / 6.28981;
    if glr_scf_stb <= 0.0 {
        let delta_p = (read.p1_pa - p2_pa).max(0.0);
        return 0.85 * area * (2.0 * read.rho_liquid * delta_p).sqrt();
    }
    if p_psig <= 0.0 || d_64ths <= 0.0 {
        return 0.0;
    }
    let q_stb_day =
        p_psig * d_64ths.powf(diameter_exponent) / (constant * glr_scf_stb.powf(glr_exponent));
    let q_m3_s = q_stb_day * 0.158987 / 86400.0;
    let glr_sm3 = glr_scf_stb * 0.02832 / 0.158987;
    q_m3_s * read.rho_liquid + q_m3_s * glr_sm3 * 1.2
}

/// `ControlValveSizing_MultiphaseChoke`: the five two-phase choke correlations.
fn choke_report(
    inlet: &Stream,
    outlet_bara: f64,
    geometry: &ValveSizingGeometry,
    standard: SizingStandard,
) -> Result<ValveSizingReport> {
    let read = choke_reads(inlet, standard)?;
    let opening = opening_factor(geometry);
    let diameter = geometry.choke_diameter_m * opening.sqrt();
    let p2_pa = outlet_bara * 1.0e5;
    let (mass_flow, critical_ratio) = match standard {
        SizingStandard::Sachdeva => (
            sachdeva_mass_flow(&read, p2_pa, geometry, diameter),
            sachdeva_critical_ratio(read.gas_quality, read.gamma),
        ),
        _ => (
            gilbert_mass_flow(&read, p2_pa, standard, diameter),
            gilbert_critical_ratio(read.gas_quality, read.gamma),
        ),
    };
    let pressure_ratio = p2_pa / read.p1_pa;
    let choked = pressure_ratio <= critical_ratio;
    let delta_p_bar = (read.p1_pa - p2_pa) / 1.0e5;
    let kv = if delta_p_bar > 0.0 && read.system_density > 0.0 {
        let q_m3h = mass_flow / read.system_density * 3600.0;
        q_m3h * ((read.system_density / 1000.0) / delta_p_bar).sqrt()
    } else {
        0.0
    };
    Ok(ValveSizingReport {
        kv,
        cv: kv * KV_TO_CV_FACTOR,
        cg: kv * KV_TO_CV_FACTOR * CG_PER_CV,
        choked,
        gas_valve: gas_valve(inlet)?,
        calculated_molar_flow: mass_flow / read.molar_mass_kg,
        calculated_outlet_pressure: read.p1_pa * critical_ratio / 1.0e5,
    })
}

/// The report for a selected standard.
///
/// # Errors
/// Whatever the route raises, including the single-phase refusal.
pub fn sizing_report_for(
    inlet: &Stream,
    outlet_bara: f64,
    standard: SizingStandard,
    geometry: &ValveSizingGeometry,
) -> Result<ValveSizingReport> {
    match standard {
        SizingStandard::Default => {
            require_single_phase(inlet, standard)?;
            sizing_report(inlet, outlet_bara, geometry)
        }
        SizingStandard::Iec60534 => iec_report(inlet, outlet_bara, geometry, false, standard),
        SizingStandard::Iec60534Full => iec_report(inlet, outlet_bara, geometry, true, standard),
        SizingStandard::ProdChoke => simple_report(inlet, outlet_bara, geometry, standard),
        SizingStandard::Sachdeva
        | SizingStandard::Gilbert
        | SizingStandard::Baxendell
        | SizingStandard::Ros
        | SizingStandard::Achong => choke_report(inlet, outlet_bara, geometry, standard),
    }
}

/// The report's solved pressure as a [`Pressure`], for the model boundary.
#[must_use]
pub fn outlet_pressure(report: &ValveSizingReport) -> Pressure {
    pascals(report.calculated_outlet_pressure * 1.0e5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use azoth_core::units::{kelvins, pascals};

    /// The capture's `sizing_gas_rich` state: 0.1/0.2/0.4/0.3 methane/ethane/propane/n-butane
    /// at 296.25 K and 1.82925 bara, 14942.5 kg/hr - 97.21961972278174 mol/s.
    fn rich_gas() -> Stream {
        Stream::from_pt(
            ["methane", "ethane", "propane", "n-butane"]
                .iter()
                .map(|n| (*n).to_string())
                .collect(),
            vec![0.1, 0.2, 0.4, 0.3],
            97.219_619_722_781_74,
            pascals(1.82925e5),
            kelvins(296.25),
        )
        .expect("the rich gas resolves")
    }

    fn methane() -> Stream {
        Stream::from_pt(
            vec!["methane".to_string()],
            vec![1.0],
            82.228_663_245_306_04,
            pascals(10.0e5),
            kelvins(293.15),
        )
        .expect("the methane resolves")
    }

    fn water() -> Stream {
        Stream::from_pt(
            vec!["water".to_string()],
            vec![1.0],
            26_100.471_829_031_365,
            pascals(14.8e5),
            kelvins(289.15),
        )
        .expect("the water resolves")
    }

    /// **`1e-4` on the gas rows and `1e-9` on the liquid one.** The gas sizing reads `gamma`,
    /// and the two libraries' heat capacities differ by the divergence the plug-flow reactor's
    /// own test measures; the choked branch scales `Kv` as `1/sqrt(Fgamma)`, so that difference
    /// reaches the coefficient. The liquid form reads only the density, and it agrees to
    /// rounding.
    fn close(got: f64, want: f64, what: &str) {
        assert!(
            (got - want).abs() / want.abs() < 1e-9,
            "{what}: {got} against {want}"
        );
    }

    fn close_gas(got: f64, want: f64, what: &str) {
        assert!(
            (got - want).abs() / want.abs() < 1e-4,
            "{what}: {got} against {want}"
        );
    }

    fn close_loose(got: f64, want: f64, what: &str) {
        assert!(
            (got - want).abs() / want.abs() < 1e-5,
            "{what}: {got} against {want}"
        );
    }

    /// **`sizing_gas_rich`**: the default sizing on a gas valve, choked.
    #[test]
    fn the_rich_gas_row_is_reproduced() {
        let report = sizing_report(&rich_gas(), 1.41325, &ValveSizingGeometry::default())
            .expect("the sizing computes");
        assert!(report.gas_valve);
        assert!(report.choked);
        close_gas(report.kv, 607.954_033_813_071_4, "kv");
        close_gas(report.cv, 702.794_863_087_910_5, "cv");
        close_gas(report.cg, 955_801.013_799_558_3, "cg");
        close(report.calculated_molar_flow, 97.219_619_722_781_74, "flow");
        close_loose(
            report.calculated_outlet_pressure,
            1.413_249_538_421_630_6,
            "solved outlet",
        );
    }

    /// **`sizing_methane`**: a gas valve that is not choked.
    #[test]
    fn the_methane_row_is_reproduced() {
        let report = sizing_report(&methane(), 9.0, &ValveSizingGeometry::default())
            .expect("the sizing computes");
        assert!(report.gas_valve);
        assert!(!report.choked);
        close_gas(report.kv, 78.646_163_426_829_79, "kv");
        close_gas(report.cv, 90.914_964_921_415_23, "cv");
        close(report.calculated_molar_flow, 82.228_663_245_306_04, "flow");
        close_loose(
            report.calculated_outlet_pressure,
            8.999_995_656_967_162,
            "solved outlet",
        );
    }

    /// **`sizing_water_liquid`**: the liquid branch, analytic in both directions.
    #[test]
    fn the_water_row_is_reproduced() {
        let report = sizing_report(&water(), 4.46, &ValveSizingGeometry::default())
            .expect("the sizing computes");
        assert!(!report.gas_valve);
        assert!(!report.choked);
        // The liquid `Kv` reads the density, and the translated one carries the two
        // libraries' rounding at `3e-8`; the flow and the solved pressure are exact.
        assert!(
            (report.kv - 528.496_159_725_506_8).abs() / 528.496_159_725_506_8 < 1e-6,
            "kv: {}",
            report.kv
        );
        assert!(
            (report.cv - 610.941_560_642_685_8).abs() / 610.941_560_642_685_8 < 1e-6,
            "cv: {}",
            report.cv
        );
        close(report.calculated_molar_flow, 26_100.471_829_031_365, "flow");
        close_loose(
            report.calculated_outlet_pressure,
            4.460_000_000_000_003,
            "solved outlet",
        );
    }

    /// **The opening and the fouling scale the adjusted `Kv`**, and the flow with it: a linear
    /// characteristic at half opening is half the flow, and a ten-per-cent fouling is nine
    /// tenths of it.
    #[test]
    fn the_opening_and_fouling_scale_the_flow() {
        let stated = ValveSizingGeometry {
            kv: Some(607.954_033_813_071_4),
            ..ValveSizingGeometry::default()
        };
        let full = sizing_report(&rich_gas(), 1.41325, &stated)
            .expect("the sizing computes")
            .calculated_molar_flow;
        let half = sizing_report(
            &rich_gas(),
            1.41325,
            &ValveSizingGeometry {
                opening_percent: 50.0,
                ..stated.clone()
            },
        )
        .expect("the sizing computes")
        .calculated_molar_flow;
        close(half / full, 0.5, "half opening");
        let fouled = sizing_report(
            &rich_gas(),
            1.41325,
            &ValveSizingGeometry {
                fouling_fraction: 0.1,
                ..stated
            },
        )
        .expect("the sizing computes")
        .calculated_molar_flow;
        close(fouled / full, 0.9, "ten-per-cent fouling");
    }

    /// **A non-positive drop answers `NaN`**, which is the class's own `calcKv` (it takes the
    /// square root of a negative ratio), and the `acceptNegativeDP` row's stream is still a
    /// state the model publishes.
    #[test]
    fn a_zero_drop_answers_no_sizing() {
        let report = sizing_report(&rich_gas(), 1.82925, &ValveSizingGeometry::default())
            .expect("the report computes");
        assert!(report.kv.is_nan());
        assert!(report.calculated_outlet_pressure.is_nan());
        assert_eq!(report.calculated_molar_flow, 0.0);
    }

    /// The four `getOpeningFactor` forms.
    #[test]
    fn the_characteristics_are_the_class_own() {
        assert!((ValveCharacteristic::Linear.opening_factor(50.0) - 0.5).abs() < 1e-15);
        assert!(
            (ValveCharacteristic::EqualPercentage.opening_factor(50.0) - 50.0_f64.powf(-0.5)).abs()
                < 1e-15
        );
        assert!(
            (ValveCharacteristic::QuickOpening.opening_factor(50.0) - 0.5_f64.sqrt()).abs() < 1e-15
        );
        assert!((ValveCharacteristic::ModifiedParabolic.opening_factor(50.0) - 0.25).abs() < 1e-15);
    }
}
