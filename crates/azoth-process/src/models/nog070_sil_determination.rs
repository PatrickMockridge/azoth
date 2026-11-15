//! `process.nog070_sil_determination` - the NOG 070 minimum-SIL table and compliance.
//!
//! Spec: `specs/models/process/nog070_sil_determination.toml`. The arithmetic is
//! `process/safety/risk/sis/nog070/Nog070SilDetermination.java` at the pin: the PFD-to-SIL
//! map, the 18-row catalogue and the compliance verdict.

use azoth_core::{AzothError, Result, Warning, apply_checks};
use serde::Serialize;

use crate::executor::json::warnings as wire_warnings;
use crate::model_gen;

/// The catalogue's minimum SIL for a standard SIF, or `None` for a type it does not know.
fn catalogue_minimum_sil(sif_type: &str) -> Option<u8> {
    let minimum = match sif_type {
        "psd_process_segment" => 2,
        "esd_topside_isolation" => 2,
        "esd_subsea_isolation" => 3,
        "hipps_pipeline" => 3,
        "blowdown_hydrocarbon_segment" => 2,
        "fg_gas_detection_esd" => 2,
        "fg_fire_detection_esd" => 2,
        "fg_hvac_shutdown" => 2,
        "fg_deluge_release" => 2,
        "psd_compressor_anti_surge" => 2,
        "psd_high_vibration" => 1,
        "psd_high_level" => 2,
        "psd_low_low_level" => 2,
        "psd_burner_management" => 2,
        "esd_riser" => 3,
        "esd_wellhead" => 2,
        "esd_loading_release" => 2,
        "custom" => 0,
        _ => return None,
    };
    Some(minimum)
}

/// The class's `pfdToSil`: `[0.1, 1)` reads SIL 0.
fn pfd_to_sil(pfd: f64) -> u8 {
    if pfd >= 1.0e-1 {
        0
    } else if pfd >= 1.0e-2 {
        1
    } else if pfd >= 1.0e-3 {
        2
    } else if pfd >= 1.0e-4 {
        3
    } else {
        4
    }
}

/// Result of `process.nog070_sil_determination`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Nog070SilDeterminationResult {
    /// The catalogue's minimum SIL for the type.
    pub catalogue_minimum_sil: f64,
    /// The minimum applied: the caller's when stated, otherwise the catalogue's.
    pub minimum_sil: f64,
    /// The SIL the PFD maps to.
    pub achieved_sil: f64,
    /// Whether the achieved SIL is at least the minimum.
    pub compliant: bool,
    /// `10^(minimum - achieved)` when the SIF is not compliant.
    pub required_pfd_reduction_factor: Option<f64>,
    /// Caveats.
    #[serde(serialize_with = "wire_warnings")]
    pub warnings: Vec<Warning>,
}

/// The catalogue minimum, the applied minimum, the achieved SIL and compliance.
///
/// # Errors
/// `InvalidInput` for a SIF type the catalogue does not name, a PFD outside `(0, 1)`, a
/// minimum SIL outside `[0, 4]` or not whole, or a non-finite value.
pub fn nog070_sil_determination(
    sif_type: &str,
    pfd_average: f64,
    minimum_sil: Option<f64>,
) -> Result<Nog070SilDeterminationResult> {
    let spec = &model_gen::NOG070_SIL_DETERMINATION_SPEC;
    let mut warnings = Vec::new();
    apply_checks(
        spec.input_checks(),
        |quantity| match quantity {
            "pfd_average" => Some(pfd_average),
            "minimum_sil" => minimum_sil,
            _ => None,
        },
        &mut warnings,
    )?;

    let catalogue = catalogue_minimum_sil(sif_type).ok_or_else(|| {
        AzothError::invalid_input(
            "sif_type",
            format!("the NOG 070 catalogue does not name {sif_type:?}"),
        )
    })?;
    if !pfd_average.is_finite() || pfd_average <= 0.0 || pfd_average >= 1.0 {
        return Err(AzothError::invalid_input(
            "pfd_average",
            format!("a PFD is above zero and below one, and {pfd_average} is not"),
        ));
    }
    let minimum = match minimum_sil {
        Some(value) => {
            if !value.is_finite() || value.fract() != 0.0 || !(0.0..=4.0).contains(&value) {
                return Err(AzothError::invalid_input(
                    "minimum_sil",
                    format!("a minimum SIL is a whole number in [0, 4], and {value} is not"),
                ));
            }
            value as u8
        }
        None => catalogue,
    };

    let achieved = pfd_to_sil(pfd_average);
    let compliant = achieved >= minimum;
    let required_pfd_reduction_factor = if compliant {
        None
    } else {
        Some(10f64.powi(i32::from(minimum) - i32::from(achieved)))
    };

    Ok(Nog070SilDeterminationResult {
        catalogue_minimum_sil: f64::from(catalogue),
        minimum_sil: f64::from(minimum),
        achieved_sil: f64::from(achieved),
        compliant,
        required_pfd_reduction_factor,
        warnings,
    })
}
