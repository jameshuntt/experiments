mod common;

use experiments::{
    read_result, read_spec, write_result, write_spec, CommandRecord, Evidence, ExperimentResult,
    PromotionDecision, VerificationResult, RESULT_SCHEMA, SCHEMA,
};
use scientific_method::{
    Comparator, Criterion, Hypothesis, Measurement, Observation, Observations, Operand,
    ScientificMethod,
};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

fn write_text(path: &Path, text: &str) {
    fs::write(path, text).expect("write fixture");
}

#[test]
fn shipped_examples_in_all_four_formats_decode_to_the_same_experiment() {
    let oneo = read_spec(example("v1.minimal-latency.1o.experiment")).expect("1o");
    let json = read_spec(example("minimal-latency.json")).expect("json");
    let toml = read_spec(example("minimal-latency.toml")).expect("toml");
    let yaml = read_spec(example("minimal-latency.yaml")).expect("yaml");
    assert_eq!(oneo, json);
    assert_eq!(json, toml);
    assert_eq!(toml, yaml);
}

#[test]
fn rich_experiment_round_trips_through_1o_json_toml_yaml_and_yml() {
    let dir = tempdir().expect("tempdir");
    let spec = common::rich_spec(".");
    let files = [
        "v1.rich.1o.experiment",
        "rich.json",
        "rich.toml",
        "rich.yaml",
        "rich.yml",
    ];
    for name in files {
        let path = dir.path().join(name);
        write_spec(&path, &spec).unwrap_or_else(|error| panic!("write {name}: {error}"));
        let decoded = read_spec(&path).unwrap_or_else(|error| panic!("read {name}: {error}"));
        assert_eq!(decoded, spec, "{name}");
    }

    let oneo = fs::read_to_string(dir.path().join("v1.rich.1o.experiment")).expect("read 1o");
    let first = oneo.find("prediction {").expect("first prediction");
    let second = oneo[first + 1..]
        .find("prediction {")
        .map(|offset| first + 1 + offset)
        .expect("second prediction");
    assert!(first < second, "repeated entries preserve declaration order");
    assert!(oneo.starts_with("experiment {\n"), "the 1o writer must emit 1o syntax");
    assert!(oneo.contains("criterion {\n") && oneo.contains("negated {\n"));
}

#[test]
fn core_oneo_discovers_the_shipped_experiment_schema() {
    let workspace = core_1o::engine::Workspace::new(env!("CARGO_MANIFEST_DIR"));
    let schema = workspace
        .schema_for_file("experiment", "v1")
        .expect("experiment schema");
    assert_eq!(schema["title"].as_str(), Some("experiments experiment v1"));
}

#[test]
fn oneo_schema_rejects_unknown_keys() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("v1.unknown.1o.experiment");
    write_text(
        &path,
        r#"experiment {
  schema "experiments.spec.v3"
  id "unknown"
  scientific_method { hypothesis { statement "x" } }
  target { root "." }
  magical true
}
"#,
    );
    let error = read_spec(path).expect_err("unknown key must fail").to_string();
    assert!(error.contains("Unknown") || error.contains("unknown"), "{error}");
}

#[test]
fn oneo_schema_rejects_duplicate_single_keys() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("v1.duplicate.1o.experiment");
    write_text(
        &path,
        r#"experiment {
  schema "experiments.spec.v3"
  id "first"
  id "second"
  scientific_method { hypothesis { statement "x" } }
  target { root "." }
}
"#,
    );
    let error = read_spec(path)
        .expect_err("x-1o-single-keys must reject duplicate id")
        .to_string();
    assert!(error.contains("id") || error.contains("single"), "{error}");
}

#[test]
fn oneo_schema_rejects_wrong_types_and_missing_required_keys() {
    let dir = tempdir().expect("tempdir");
    let wrong_type = dir.path().join("v1.wrong-type.1o.experiment");
    write_text(
        &wrong_type,
        r#"experiment {
  schema "experiments.spec.v3"
  id 42
  scientific_method { hypothesis { statement "x" } }
  target { root "." }
}
"#,
    );
    assert!(read_spec(wrong_type).is_err());

    let missing = dir.path().join("v1.missing.1o.experiment");
    write_text(
        &missing,
        r#"experiment {
  schema "experiments.spec.v3"
  id "missing"
  scientific_method { hypothesis { statement "x" } }
}
"#,
    );
    assert!(read_spec(missing).is_err());
}

#[test]
fn oneo_schema_enforces_conditional_criterion_fields() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("v1.criterion.1o.experiment");
    write_text(
        &path,
        r#"experiment {
  schema "experiments.spec.v3"
  id "conditional"
  scientific_method {
    hypothesis { statement "x" }
    criterion {
      kind "improvement"
      direction "lower_is_better"
      minimum_fraction 0.05
    }
  }
  target { root "." }
}
"#,
    );
    let error = read_spec(path)
        .expect_err("improvement without baseline/candidate must fail")
        .to_string();
    assert!(error.contains("baseline") || error.contains("candidate"), "{error}");
}

#[test]
fn all_spec_formats_reject_the_wrong_experiment_schema_version() {
    let dir = tempdir().expect("tempdir");
    let mut spec = common::base_spec(".");
    spec.schema = "experiments.spec.v999".into();
    for name in ["wrong.json", "wrong.toml", "wrong.yaml", "wrong.yml"] {
        let path = dir.path().join(name);
        write_spec(&path, &spec).expect("writer may serialize an explicit historical schema");
        let error = read_spec(&path).expect_err("reader must gate the schema").to_string();
        assert!(error.contains(SCHEMA), "{name}: {error}");
    }
}

#[test]
fn unsupported_spec_extension_is_refused() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("experiment.txt");
    write_text(&path, "{}");
    let error = read_spec(path).expect_err("unsupported extension").to_string();
    assert!(error.contains("unsupported experiment format"), "{error}");
}

fn sample_result() -> ExperimentResult {
    let observations = Observations::new().with(Observation::new(
        "candidate",
        Measurement::new("tests_pass", true),
    ));
    let method = ScientificMethod::new(Hypothesis::new("candidate is valid")).criterion(
        Criterion::compare(
            Operand::observation("candidate", "tests_pass"),
            Comparator::Equal,
            Operand::literal(true),
        ),
    );
    let evaluation = method.evaluate(&observations).expect("evaluate");
    ExperimentResult {
        schema: RESULT_SCHEMA.into(),
        experiment_id: "result-roundtrip".into(),
        run_id: "run-1".into(),
        observations,
        evaluation,
        verification: VerificationResult {
            passed: true,
            commands: Vec::new(),
        },
        commands: vec![CommandRecord {
            phase: "verification".into(),
            label: "verify".into(),
            program: "true".into(),
            args: Vec::new(),
            cwd: ".".into(),
            display: "true".into(),
            status: 0,
            stdout: String::new(),
            stderr: String::new(),
        }],
        mutations: Vec::new(),
        differences: Vec::new(),
        evidence: vec![Evidence {
            kind: "fixture".into(),
            name: "proof".into(),
            path: "proof.json".into(),
        }],
        promotion: PromotionDecision::Promote,
    }
}

#[test]
fn results_round_trip_through_json_toml_yaml_and_yml() {
    let dir = tempdir().expect("tempdir");
    let result = sample_result();
    for name in ["result.json", "result.toml", "result.yaml", "result.yml"] {
        let path = dir.path().join(name);
        write_result(&path, &result).unwrap_or_else(|error| panic!("write {name}: {error}"));
        let decoded = read_result(&path).unwrap_or_else(|error| panic!("read {name}: {error}"));
        assert_eq!(decoded, result, "{name}");
    }
}


#[test]
fn result_reader_rejects_the_wrong_result_schema_version() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("result.json");
    let mut result = sample_result();
    result.schema = "experiments.result.v999".into();
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&result).expect("serialize wrong result"),
    )
    .expect("write wrong result");
    let error = read_result(path)
        .expect_err("wrong result schema must be refused")
        .to_string();
    assert!(error.contains(RESULT_SCHEMA), "{error}");
}

#[test]
fn result_oneo_is_refused_until_a_result_schema_exists() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("v1.run.1o.experiment");
    let error = write_result(path, &sample_result())
        .expect_err("result 1o must not be faked")
        .to_string();
    assert!(error.contains("result schema"), "{error}");
}
