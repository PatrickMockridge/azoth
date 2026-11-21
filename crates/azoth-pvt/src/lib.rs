//! PVT simulation.
//!
//! NeqSim's `pvtsimulation/` without its `flowassurance/`: the black-oil and gas-water
//! correlation tables, the decline and material-balance arithmetic, and the laboratory
//! experiments a reservoir fluid is described by. It is a namespace of its own rather than a
//! family under `eos` or `process`, mirroring NeqSim's package split: a PVT experiment is a
//! sequence of flashes and property reads, not an equation of state and not a unit operation.
//!
//! * [`standard`] — the standard state and the class conversion literals every id here states
//!   its inputs against.

pub mod black_oil_correlations;
pub mod gas_correlations;
pub mod model_gen;
pub mod model_records_gen;
pub mod standard;
pub mod water_correlations;

pub use black_oil_correlations::{
    BlackOilCorrelationsResult, BlackOilQuery, BlackOilResultRow, black_oil_correlations,
};
pub use gas_correlations::{GasCorrelationsResult, GasQuery, GasResultRow, gas_correlations};
pub use water_correlations::{
    WaterCorrelationsResult, WaterQuery, WaterResultRow, water_correlations,
};
