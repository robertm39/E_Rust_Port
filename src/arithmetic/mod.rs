//! Optional exact arithmetic reasoning subsystems.
//!
//! Arithmetic reasoning is isolated behind Cargo features so default Umlaut
//! builds and competition schedules remain independent of experimental
//! theory code and its dependencies.

#[cfg(feature = "viras-qe")]
pub mod viras;

#[cfg(feature = "viras-qe")]
pub mod typed_lira;
