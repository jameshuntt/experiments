#![forbid(unsafe_code)]

//! `experiments` is the executable/reproducible layer above `scientific-method`.
//! It defines experiment files, executes declared protocol and measurement commands,
//! records evidence, evaluates the scientific method, and emits a durable result.

mod codec;
mod differences;
mod layout;
mod model;
mod runner;

pub use codec::{read_result, read_spec, write_result, write_spec};
pub use differences::{DifferencesReport, DifferencesRequest, DifferencesSource};
pub use layout::ExperimentPaths;
pub use model::*;
pub use runner::{ExperimentError, ExperimentRunner};

#[cfg(feature = "custodian")]
pub mod custodian {
    //! Embedded integration boundary. The kits are re-exported side by side on purpose:
    //! custodian_kit performs controlled work; differences_kit reads/reports its histories.
    pub use custodian_kit;
    pub use differences_kit;
}
