//! The `pvt` namespace's PyO3 bindings.
//!
//! Each `#[pyfunction]` is the extension's door for one registered `pvt.*` id; the wrapper,
//! the stub and the bridge are generated from the spec and this module's signature. The
//! module carries no arithmetic: the kernels are `azoth_pvt`'s, and a binding that restated
//! one would be a third implementation.
