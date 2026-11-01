#![allow(dead_code)]

use experiments::{
    CommandSpec, DifferencesRequest, ExperimentSpec, ExperimentTarget, MeasurementParser,
    MeasurementPhase, MeasurementPlan, MeasurementSpec, MutationBackend, MutationChange,
    MutationMode, MutationPlan, PromotionPolicy, Protocol, Verification, SCHEMA,
};
use scientific_method::{
    Comparator, Control, Criterion, DependentVariable, Direction, Hypothesis,
    IndependentVariable, Operand, Prediction, Question, ScientificMethod,
};
use std::fs;
use std::path::{Path, PathBuf};

pub fn command(label: &str, program: &str, args: &[&str]) -> CommandSpec {
    CommandSpec {
        label: label.into(),
        program: program.into(),
        args: args.iter().map(|value| (*value).to_owned()).collect(),
        cwd: None,
    }
}

pub fn shell(label: &str, script: &str) -> CommandSpec {
    command(label, "sh", &["-c", script])
}

pub fn base_spec(root: impl Into<PathBuf>) -> ExperimentSpec {
    ExperimentSpec {
        schema: SCHEMA.to_owned(),
        id: "qualification".into(),
        description: Some("qualification fixture".into()),
        scientific_method: ScientificMethod::new(Hypothesis::new("candidate changes")),
        target: ExperimentTarget {
            root: root.into(),
            label: Some("fixture".into()),
        },
        protocol: Protocol::default(),
        mutation: MutationPlan::default(),
        verification: Verification::default(),
        measurements: MeasurementPlan::default(),
        differences: Vec::new(),
        promotion: PromotionPolicy::default(),
    }
}

pub fn rich_spec(root: impl Into<PathBuf>) -> ExperimentSpec {
    let method = ScientificMethod::new(
        Hypothesis::new("the candidate improves latency")
            .with_rationale("the intervention removes avoidable work"),
    )
    .question(Question::new("Does the candidate improve latency without regression?"))
    .prediction(Prediction::new("candidate latency is lower"))
    .prediction(Prediction::new("verification remains successful"))
    .independent(
        IndependentVariable::new("implementation")
            .with_description("baseline versus candidate implementation"),
    )
    .dependent(
        DependentVariable::new("latency")
            .with_unit("ms")
            .with_description("wall-clock latency"),
    )
    .control(Control::new("machine").with_description("same machine and workload"))
    .criterion(Criterion::improvement(
        scientific_method::ObservationRef::new("baseline", "latency"),
        scientific_method::ObservationRef::new("candidate", "latency"),
        Direction::LowerIsBetter,
        0.05,
    ))
    .criterion(Criterion::regression_limit(
        scientific_method::ObservationRef::new("baseline", "latency"),
        scientific_method::ObservationRef::new("candidate", "latency"),
        Direction::LowerIsBetter,
        0.02,
    ))
    .criterion(Criterion::all([
        Criterion::compare(
            Operand::observation("candidate", "tests_pass"),
            Comparator::Equal,
            Operand::literal(true),
        ),
        Criterion::any([
            Criterion::compare(
                Operand::observation("candidate", "latency"),
                Comparator::LessThan,
                Operand::literal(100.0),
            ),
            Criterion::negate(Criterion::compare(
                Operand::observation("candidate", "status"),
                Comparator::Equal,
                Operand::literal("regressed"),
            )),
        ]),
    ]));

    ExperimentSpec {
        schema: SCHEMA.to_owned(),
        id: "rich-format-roundtrip".into(),
        description: Some("every repeated and nested shape used by the experiment document".into()),
        scientific_method: method,
        target: ExperimentTarget {
            root: root.into(),
            label: Some("rich fixture".into()),
        },
        protocol: Protocol {
            prepare: vec![command("prepare one", "printf", &["prepare"]), command("prepare two", "printf", &["again"])],
            baseline: vec![command("baseline", "printf", &["baseline"])],
            intervention: vec![command("intervention", "printf", &["candidate"])],
            candidate: vec![command("candidate", "printf", &["candidate"])],
            cleanup: vec![command("cleanup", "printf", &["cleanup"])],
        },
        mutation: MutationPlan {
            mode: MutationMode::Unbounded,
            backend: MutationBackend::CodePassenger,
            changes: vec![MutationChange {
                label: "structured change".into(),
                spec: "changes/candidate.toml".into(),
            }],
        },
        verification: Verification {
            commands: vec![command("verify one", "true", &[]), command("verify two", "true", &[])],
        },
        measurements: MeasurementPlan {
            measurements: vec![
                MeasurementSpec {
                    phase: MeasurementPhase::Baseline,
                    condition: "baseline".into(),
                    name: "latency".into(),
                    unit: Some("ms".into()),
                    command: command("baseline latency", "printf", &["100"]),
                    parser: MeasurementParser::Number,
                },
                MeasurementSpec {
                    phase: MeasurementPhase::Candidate,
                    condition: "candidate".into(),
                    name: "latency".into(),
                    unit: Some("ms".into()),
                    command: command("candidate latency", "printf", &["90"]),
                    parser: MeasurementParser::Number,
                },
                MeasurementSpec {
                    phase: MeasurementPhase::Candidate,
                    condition: "candidate".into(),
                    name: "tests_pass".into(),
                    unit: None,
                    command: command("tests pass", "true", &[]),
                    parser: MeasurementParser::ExitSuccess,
                },
                MeasurementSpec {
                    phase: MeasurementPhase::Candidate,
                    condition: "candidate".into(),
                    name: "status".into(),
                    unit: None,
                    command: command("status", "printf", &["healthy"]),
                    parser: MeasurementParser::Text,
                },
            ],
        },
        differences: vec![
            DifferencesRequest {
                line: vec!["tree".into(), "before".into(), "after".into()],
            },
            DifferencesRequest {
                line: vec!["count".into(), "before".into(), "after".into()],
            },
        ],
        promotion: PromotionPolicy {
            require_supported: false,
            require_verification: true,
        },
    }
}

pub fn write_fixture_crate(root: &Path) {
    fs::create_dir_all(root.join("src")).expect("create fixture src");
    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "fixture"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"#,
    )
    .expect("write fixture Cargo.toml");
    fs::write(
        root.join("src/lib.rs"),
        "pub fn existing() -> u64 { 1 }\n",
    )
    .expect("write fixture lib");
}

pub fn write_make_struct_change(path: &Path, name: &str, struct_name: &str, field_ty: &str) {
    let text = format!(
        r#"name = "{name}"
objective = "add a candidate type for the experiment"
operation = "make_struct"
target = "crate:fixture"

[item]
name = "{struct_name}"
vis = "pub"
derives = ["Debug", "Clone", "PartialEq"]

[[item.fields]]
name = "value"
ty = "{field_ty}"
vis = "pub"
"#
    );
    fs::write(path, text).expect("write Code Passenger change spec");
}
