mod common;

use experiments::{
    ExperimentError, ExperimentPaths, ExperimentRunner, MeasurementParser, MeasurementPhase, MeasurementSource,
    MeasurementPlan, MeasurementSpec, MutationMode, PromotionDecision, PromotionPolicy,
};
use scientific_method::{Direction, Hypothesis, ObservationRef, ScientificMethod};
use std::fs;
use tempfile::tempdir;

#[test]
fn runner_orders_phases_and_collects_baseline_before_intervention_and_candidate_after() {
    let target = tempdir().expect("target");
    let storage = tempdir().expect("storage");
    fs::write(target.path().join("value.txt"), "100\n").expect("baseline value");

    let mut spec = common::base_spec(target.path());
    spec.id = "phase-order".into();
    spec.scientific_method = ScientificMethod::new(Hypothesis::new("candidate lowers latency"))
        .criterion(scientific_method::Criterion::improvement(
            ObservationRef::new("baseline", "latency"),
            ObservationRef::new("candidate", "latency"),
            Direction::LowerIsBetter,
            0.05,
        ));
    spec.protocol.prepare = vec![common::shell("prepare", "echo prepare >> order.txt")];
    spec.protocol.baseline = vec![common::shell("baseline", "echo baseline >> order.txt")];
    spec.protocol.intervention = vec![common::shell(
        "intervention",
        "echo intervention >> order.txt; printf '90\\n' > value.txt",
    )];
    spec.protocol.candidate = vec![common::shell("candidate", "echo candidate >> order.txt")];
    spec.protocol.cleanup = vec![common::shell("cleanup", "echo cleanup >> order.txt")];
    spec.measurements = MeasurementPlan {
        measurements: vec![
            MeasurementSpec {
                phase: MeasurementPhase::Baseline,
                condition: "baseline".into(),
                name: "latency".into(),
                unit: Some("ms".into()),
                source: MeasurementSource::Command {
                    command: common::shell(
                        "baseline measurement",
                        "echo baseline_measurement >> order.txt; cat value.txt",
                    ),
                    parser: MeasurementParser::Number,
                },
            },
            MeasurementSpec {
                phase: MeasurementPhase::Candidate,
                condition: "candidate".into(),
                name: "latency".into(),
                unit: Some("ms".into()),
                source: MeasurementSource::Command {
                    command: common::shell(
                        "candidate measurement",
                        "echo candidate_measurement >> order.txt; cat value.txt",
                    ),
                    parser: MeasurementParser::Number,
                },
            },
        ],
    };
    spec.verification.commands = vec![common::shell(
        "verification",
        "echo verification >> order.txt; test \"$(cat value.txt)\" = 90",
    )];

    let (result, result_path) = ExperimentRunner::new(storage.path())
        .run(&spec)
        .expect("experiment succeeds");

    assert_eq!(result.promotion, PromotionDecision::Promote);
    assert_eq!(
        result
            .observations
            .get(&ObservationRef::new("baseline", "latency"))
            .expect("baseline")
            .value,
        scientific_method::Scalar::Number(100.0)
    );
    assert_eq!(
        result
            .observations
            .get(&ObservationRef::new("candidate", "latency"))
            .expect("candidate")
            .value,
        scientific_method::Scalar::Number(90.0)
    );
    assert_eq!(
        fs::read_to_string(target.path().join("order.txt")).expect("order"),
        "prepare\nbaseline\nbaseline_measurement\nintervention\ncandidate\ncandidate_measurement\nverification\ncleanup\n"
    );
    let phases: Vec<&str> = result.commands.iter().map(|record| record.phase.as_str()).collect();
    assert_eq!(
        phases,
        [
            "prepare",
            "baseline",
            "measurement:baseline",
            "intervention",
            "candidate",
            "measurement:candidate",
            "verification",
            "cleanup",
        ]
    );
    assert!(result_path.is_file());

    let paths = ExperimentPaths::under(storage.path()).expect("paths");
    assert!(paths.evidence(&result.run_id).is_dir());
    assert!(paths.receipts(&result.run_id).is_dir());
    assert!(paths.differences(&result.run_id).is_dir());
    assert_eq!(paths.result(&result.run_id), result_path);
}

#[test]
fn bounded_mode_refuses_raw_intervention_before_prepare_can_touch_the_target() {
    let target = tempdir().expect("target");
    let storage = tempdir().expect("storage");
    let mut spec = common::base_spec(target.path());
    spec.mutation.mode = MutationMode::Bounded;
    spec.protocol.prepare = vec![common::shell("prepare", "touch prepare-ran")];
    spec.protocol.intervention = vec![common::shell("raw edit", "touch raw-edit-ran")];

    let error = ExperimentRunner::new(storage.path())
        .run(&spec)
        .expect_err("bounded raw intervention must be refused");
    assert!(matches!(&error, ExperimentError::Mutation(_)), "{error}");
    assert!(!target.path().join("prepare-ran").exists());
    assert!(!target.path().join("raw-edit-ran").exists());
}

#[test]
fn unbounded_mode_executes_declared_intervention() {
    let target = tempdir().expect("target");
    let storage = tempdir().expect("storage");
    let mut spec = common::base_spec(target.path());
    spec.promotion = PromotionPolicy {
        require_supported: false,
        require_verification: false,
    };
    spec.protocol.intervention = vec![common::shell("intervention", "printf yes > changed.txt")];

    let (result, _) = ExperimentRunner::new(storage.path())
        .run(&spec)
        .expect("unbounded intervention");
    assert_eq!(
        fs::read_to_string(target.path().join("changed.txt")).expect("changed"),
        "yes"
    );
    assert_eq!(result.promotion, PromotionDecision::Undecided);
}

#[test]
fn every_measurement_parser_records_the_expected_scalar() {
    let target = tempdir().expect("target");
    let storage = tempdir().expect("storage");
    let mut spec = common::base_spec(target.path());
    spec.measurements.measurements = vec![
        MeasurementSpec {
            phase: MeasurementPhase::Candidate,
            condition: "candidate".into(),
            name: "number".into(),
            unit: None,
            source: MeasurementSource::Command {
                command: common::command("number", "printf", &["12.5"]),
                parser: MeasurementParser::Number,
            },
        },
        MeasurementSpec {
            phase: MeasurementPhase::Candidate,
            condition: "candidate".into(),
            name: "boolean".into(),
            unit: None,
            source: MeasurementSource::Command {
                command: common::command("boolean", "printf", &["true"]),
                parser: MeasurementParser::Boolean,
            },
        },
        MeasurementSpec {
            phase: MeasurementPhase::Candidate,
            condition: "candidate".into(),
            name: "text".into(),
            unit: None,
            source: MeasurementSource::Command {
                command: common::command("text", "printf", &["hello"]),
                parser: MeasurementParser::Text,
            },
        },
        MeasurementSpec {
            phase: MeasurementPhase::Candidate,
            condition: "candidate".into(),
            name: "exit_success".into(),
            unit: None,
            source: MeasurementSource::Command {
                command: common::command("exit", "false", &[]),
                parser: MeasurementParser::ExitSuccess,
            },
        },
    ];

    let (result, _) = ExperimentRunner::new(storage.path()).run(&spec).expect("run");
    let value = |name| {
        &result
            .observations
            .get(&ObservationRef::new("candidate", name))
            .unwrap_or_else(|| panic!("missing {name}"))
            .value
    };
    assert_eq!(value("number"), &scientific_method::Scalar::Number(12.5));
    assert_eq!(value("boolean"), &scientific_method::Scalar::Boolean(true));
    assert_eq!(value("text"), &scientific_method::Scalar::Text("hello".into()));
    assert_eq!(value("exit_success"), &scientific_method::Scalar::Boolean(false));
}

#[test]
fn cleanup_failure_is_evidence_and_does_not_rewrite_the_scientific_conclusion() {
    let target = tempdir().expect("target");
    let storage = tempdir().expect("storage");
    let mut spec = common::base_spec(target.path());
    spec.promotion = PromotionPolicy {
        require_supported: false,
        require_verification: false,
    };
    spec.protocol.cleanup = vec![common::command("cleanup fails", "false", &[])];

    let (result, result_path) = ExperimentRunner::new(storage.path())
        .run(&spec)
        .expect("cleanup failure is recorded, not fatal");
    let cleanup = result.commands.last().expect("cleanup record");
    assert_eq!(cleanup.phase, "cleanup");
    assert_ne!(cleanup.status, 0);
    assert_eq!(
        result.evaluation.conclusion.outcome,
        scientific_method::Outcome::Inconclusive
    );
    assert!(result_path.is_file());
}

#[test]
fn fail_fast_prepare_stops_before_baseline_and_no_result_is_written() {
    let target = tempdir().expect("target");
    let storage = tempdir().expect("storage");
    let mut spec = common::base_spec(target.path());
    spec.id = "prepare-failure".into();
    spec.protocol.prepare = vec![common::command("prepare fails", "false", &[])];
    spec.protocol.baseline = vec![common::shell("baseline", "touch baseline-ran")];

    let error = ExperimentRunner::new(storage.path())
        .run(&spec)
        .expect_err("prepare must fail fast");
    assert!(matches!(&error, ExperimentError::CommandFailed { .. }), "{error}");
    assert!(!target.path().join("baseline-ran").exists());

    let paths = ExperimentPaths::under(storage.path()).expect("paths");
    for entry in fs::read_dir(paths.runs()).expect("runs").filter_map(Result::ok) {
        assert!(!entry.path().join("results/result.json").exists());
    }
}
