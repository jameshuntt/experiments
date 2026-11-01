#![forbid(unsafe_code)]

//! `experiments` is the executable/reproducible layer above `scientific-method`.
//!
//! Experiment specs are real 1o (validated by `core-1o`) or serde-equivalent
//! JSON/TOML/YAML documents. The CLI is a declared Simple Impl/XCCUTE contract.
//! Bounded source mutations are delegated to Code Passenger's declared transform
//! line; unbounded intervention commands remain explicit XCCUTE operations.

mod codec;
pub mod cli;
mod differences;
mod layout;
mod model;
mod mutation;
mod runner;
pub mod surface;

pub use codec::{read_result, read_spec, write_result, write_spec};
pub use differences::{DifferencesReport, DifferencesRequest, DifferencesSource};
pub use layout::ExperimentPaths;
pub use model::*;
pub use runner::{ExperimentError, ExperimentRunner};

#[cfg(test)]
pub(crate) fn codec_test_spec() -> ExperimentSpec {
    use scientific_method::{Hypothesis, ScientificMethod};
    ExperimentSpec {
        schema: SCHEMA.to_owned(),
        id: "test".into(),
        description: None,
        scientific_method: ScientificMethod::new(Hypothesis::new("candidate changes")),
        target: ExperimentTarget { root: ".".into(), label: None },
        protocol: Protocol::default(),
        mutation: MutationPlan::default(),
        verification: Verification::default(),
        measurements: MeasurementPlan::default(),
        differences: Vec::new(),
        promotion: PromotionPolicy::default(),
    }
}
