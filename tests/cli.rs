mod common;

use experiments::{
    write_spec, MeasurementParser, MeasurementPhase, MeasurementSource, MeasurementSpec, PromotionPolicy,
};
use scientific_method::{Comparator, Criterion, Hypothesis, Operand, ScientificMethod};
use std::ffi::OsString;
use tempfile::tempdir;

fn run_cli(mut spec: experiments::ExperimentSpec) -> i32 {
    let target = tempdir().expect("target");
    let holder = tempdir().expect("holder");
    let storage = tempdir().expect("storage");
    spec.target.root = target.path().to_path_buf();
    let path = holder.path().join("spec.json");
    write_spec(&path, &spec).expect("write spec");
    experiments::cli::main_argv(&[
        OsString::from("run"),
        path.into_os_string(),
        storage.path().as_os_str().to_os_string(),
    ])
}

fn boolean_candidate(value: bool) -> experiments::ExperimentSpec {
    let mut spec = common::base_spec(".");
    spec.scientific_method = ScientificMethod::new(Hypothesis::new("candidate is valid")).criterion(
        Criterion::compare(
            Operand::observation("candidate", "tests_pass"),
            Comparator::Equal,
            Operand::literal(true),
        ),
    );
    spec.measurements.measurements.push(MeasurementSpec {
        phase: MeasurementPhase::Candidate,
        condition: "candidate".into(),
        name: "tests_pass".into(),
        unit: None,
        source: MeasurementSource::Command {
            command: common::command(
                "tests pass",
                "printf",
                &[if value { "true" } else { "false" }],
            ),
            parser: MeasurementParser::Boolean,
        },
    });
    spec
}

#[test]
fn cli_exit_zero_means_promote() {
    assert_eq!(run_cli(boolean_candidate(true)), 0);
}

#[test]
fn cli_exit_two_means_do_not_promote() {
    assert_eq!(run_cli(boolean_candidate(false)), 2);
}

#[test]
fn cli_exit_three_means_inconclusive() {
    let mut spec = common::base_spec(".");
    spec.promotion = PromotionPolicy {
        require_supported: false,
        require_verification: false,
    };
    assert_eq!(run_cli(spec), 3);
}

#[test]
fn cli_exit_one_means_refused_or_error() {
    let holder = tempdir().expect("holder");
    let path = holder.path().join("bad.json");
    std::fs::write(&path, r#"{"schema":"wrong"}"#).expect("bad spec");
    assert_eq!(
        experiments::cli::main_argv(&[OsString::from("run"), path.into_os_string()]),
        1
    );
}

#[test]
fn cli_help_is_zero_and_empty_invocation_is_refused() {
    assert_eq!(experiments::cli::main_argv(&[OsString::from("--help")]), 0);
    assert_eq!(experiments::cli::main_argv(&[]), 1);
}
