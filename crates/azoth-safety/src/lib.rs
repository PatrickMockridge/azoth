//! A safety study: the third document.
//!
//! **A study is not a flowsheet and not a calculation book.** It holds fire scenarios, each
//! one a registered `process.*` model and the inputs it was given, and bow-ties whose
//! labelled threats, barriers and consequences are document data while their frequencies and
//! RRF are `process.bow_tie`'s arithmetic. Results are derived and never saved.
//!
//! The document is [`study`]; the workspace and its commands come beside it. Native dispatch
//! is written out because the sheet dispatcher is generated from `specs/calcs` and the
//! safety ids are models.

pub mod study;
