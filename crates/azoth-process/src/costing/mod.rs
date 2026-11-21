//! Cost-estimation arithmetic, shared by the four `process.cost_*` ids.
//!
//! The port source is NeqSim's `process/costestimation/` package. Costs are USD
//! magnitudes carried as bare `f64`, the vocabulary having no currency unit; the
//! currency, cost year and location basis travel beside them in [`records::CostBasis`].

pub mod correlations;
pub mod equipment;
pub mod factors;
pub mod module_stack;
pub mod process;
pub mod records;
pub mod topsides;

pub use records::{CostBasis, EstimateClass, MtoItem};
