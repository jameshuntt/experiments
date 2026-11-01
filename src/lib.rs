#![forbid(unsafe_code)]

//! `experiments` is the executable/reproducible layer above `scientific-method`.
//!
//! The CLI is a declared simple_impl/xccute contract. Experiment operations lower
//! into xccute's argv-safe runtime command contract. Differences are answered
//! through the `differences` library surface rather than by spawning its binary.

mod codec;
pub mod cli;
mod differences;
mod layout;
mod model;
mod runner;
pub mod surface;

pub use codec::{read_result, read_spec, write_result, write_spec};
pub use differences::{DifferencesReport, DifferencesRequest, DifferencesSource};
pub use layout::ExperimentPaths;
pub use model::*;
pub use runner::{ExperimentError, ExperimentRunner};
