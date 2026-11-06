//! Exact tree measurements: a measurement is a command with its parser or an exact tree, never both and never
//! neither, in every format; every spec written before reads as it did; the standalone host refuses one (the
//! comparison is the governing host's); a host's record of it is the measurement's value and is kept in the result;
//! a campaign's binding resolves `equals_tree` where a change spec is resolved.

mod common;

use experiments::campaign::{Binding, LoadedCampaign};
use experiments::{
    read_result, read_spec, write_spec, ChangeRequest, CommandRecord, CommandSpec, Evidence, ExactTree,
    ExactTreeRecord, ExactTreeRequest, ExperimentError, ExperimentHost, ExperimentRunner, MeasurementParser,
    MeasurementPhase, MeasurementSource, MeasurementSpec, MutationRecord, PromotionDecision,
};
use scientific_method::{Comparator, Criterion, Operand, Scalar};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

const HEAD: &str = r#"schema = "experiments.spec.v3"
id = "x0020"

[scientific_method]
predictions = []
controls = []

[scientific_method.question]
text = "Does this Replay reproduce the recorded source state exactly?"

[scientific_method.hypothesis]
statement = "The declared Replay produces the exact next state."

[[scientific_method.independent_variables]]
name = "source_state"

[[scientific_method.dependent_variables]]
name = "candidate_state_exact"

[[scientific_method.criteria]]
kind = "compare"
comparator = "equal"
left = { kind = "observation", reference = { condition = "candidate", measurement = "candidate_state_exact" } }
right = { kind = "literal", value = { type = "boolean", value = true } }

[target]
root = "."
"#;

const COMMAND: &str = r#"
[[measurements.measurements]]
phase = "candidate"
condition = "candidate"
name = "candidate_state_exact"
parser = "exit_success"
[measurements.measurements.command]
label = "verify exact successor state"
program = "cargo"
args = ["run", "--quiet", "--bin", "bfcb-state-verify"]
"#;

const EXACT: &str = r#"
[[measurements.measurements]]
phase = "candidate"
condition = "candidate"
name = "candidate_state_exact"
[measurements.measurements.exact_tree]
path = "specimen"
equals_tree = "../../versions/0022-PASS16/tree"
"#;

fn read_toml(dir: &Path, name: &str, text: &str) -> Result<experiments::ExperimentSpec, String> {
    let p = dir.join(name);
    fs::write(&p, text).unwrap();
    read_spec(&p).map_err(|e| e.to_string())
}

#[test]
fn a_measurement_is_a_command_with_its_parser_or_an_exact_tree_in_every_format() {
    let dir = tempdir().unwrap();
    let spec = read_toml(dir.path(), "exact.toml", &format!("{HEAD}{EXACT}")).unwrap();
    let m = &spec.measurements.measurements[0];
    assert_eq!(
        m.source,
        MeasurementSource::ExactTree(ExactTree { path: "specimen".into(), equals_tree: "../../versions/0022-PASS16/tree".into() })
    );
    assert_eq!((m.command(), m.exact_tree().map(|e| e.path.as_path())), (None, Some(Path::new("specimen"))));
    // every format writes it and reads it back the same
    for name in ["v1.exact.1o.experiment", "exact.json", "exact.yaml", "exact.yml", "again.toml"] {
        let p = dir.path().join(name);
        write_spec(&p, &spec).unwrap();
        let text = fs::read_to_string(&p).unwrap();
        assert!(text.contains("exact_tree") && text.contains("equals_tree") && !text.contains("parser"), "{name}: {text}");
        assert_eq!(read_spec(&p).unwrap(), spec, "{name}");
    }
    // both, neither, a command without its parser, a parser without its command: refused as read, saying so
    let both = format!("{HEAD}{}", EXACT.replace("[measurements.measurements.exact_tree]", "parser = \"exit_success\"\n[measurements.measurements.command]\nlabel = \"v\"\nprogram = \"true\"\n[measurements.measurements.exact_tree]"));
    for (name, text, says) in [
        ("both.toml", both.as_str(), "names both a command or parser and an exact_tree: it is one or the other"),
        ("neither.toml", &format!("{HEAD}\n[[measurements.measurements]]\nphase = \"candidate\"\ncondition = \"candidate\"\nname = \"candidate_state_exact\"\n"), "names neither a command (with its parser) nor an exact_tree"),
        ("no-parser.toml", &format!("{HEAD}{}", COMMAND.replace("parser = \"exit_success\"\n", "")), "a command measurement needs its parser"),
        ("parser-and-tree.toml", &format!("{HEAD}{}", EXACT.replace("name = \"candidate_state_exact\"\n", "name = \"candidate_state_exact\"\nparser = \"boolean\"\n")), "names both a command or parser and an exact_tree"),
        ("unknown-key.toml", &format!("{HEAD}{}", EXACT.replace("path = \"specimen\"", "path = \"specimen\"\nmode = \"loose\"")), "unknown field `mode`"),
    ] {
        let e = read_toml(dir.path(), name, text).unwrap_err();
        assert!(e.contains(says), "{name}: {e}");
    }
    let e = read_spec({
        let p = dir.path().join("both.json");
        let mut v: serde_json::Value = serde_json::from_str(&fs::read_to_string(dir.path().join("exact.json")).unwrap()).unwrap();
        v["measurements"]["measurements"][0]["command"] = serde_json::json!({"label": "v", "program": "true"});
        v["measurements"]["measurements"][0]["parser"] = serde_json::json!("exit_success");
        fs::write(&p, v.to_string()).unwrap();
        p
    })
    .unwrap_err()
    .to_string();
    assert!(e.contains("one or the other"), "json: {e}");
}

#[test]
fn a_spec_written_before_reads_as_it_did_and_writes_the_same_keys() {
    let dir = tempdir().unwrap();
    let spec = read_toml(dir.path(), "command.toml", &format!("{HEAD}{COMMAND}")).unwrap();
    let m = &spec.measurements.measurements[0];
    assert!(matches!(&m.source, MeasurementSource::Command { command, parser: MeasurementParser::ExitSuccess } if command.program == "cargo"));
    assert_eq!(
        serde_json::to_string(m).unwrap(),
        r#"{"phase":"candidate","condition":"candidate","name":"candidate_state_exact","unit":null,"command":{"label":"verify exact successor state","program":"cargo","args":["run","--quiet","--bin","bfcb-state-verify"],"cwd":null},"parser":"exit_success"}"#,
        "the measurement's keys, in order, as 0.4 wrote them"
    );
    // the shipped examples, in every format, still one experiment
    let example = |n: &str| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples").join(n);
    let toml = read_spec(example("minimal-latency.toml")).unwrap();
    for n in ["v1.minimal-latency.1o.experiment", "minimal-latency.json", "minimal-latency.yaml"] {
        assert_eq!(read_spec(example(n)).unwrap(), toml, "{n}");
    }
    assert!(toml.measurements.measurements.iter().all(|m| m.command().is_some()));
}

/// A spec whose candidate measurement is the exact tree `specimen` = `../expected`, held true by its criterion.
fn exact_spec(root: &Path) -> experiments::ExperimentSpec {
    let mut s = common::base_spec(root);
    s.scientific_method = s.scientific_method.criterion(Criterion::compare(Operand::observation("candidate", "candidate_state_exact"), Comparator::Equal, Operand::literal(true)));
    s.promotion.require_verification = false;
    s.measurements.measurements.push(MeasurementSpec {
        phase: MeasurementPhase::Candidate,
        condition: "candidate".into(),
        name: "candidate_state_exact".into(),
        unit: None,
        source: MeasurementSource::ExactTree(ExactTree { path: "specimen".into(), equals_tree: "../expected".into() }),
    });
    s
}

#[test]
fn the_standalone_host_refuses_an_exact_tree_measurement_before_anything_runs() {
    let dir = tempdir().unwrap();
    let mut spec = exact_spec(dir.path());
    spec.protocol.prepare.push(common::command("prepare", "touch", &["prepared"]));
    let storage = dir.path().join("storage");
    let e = ExperimentRunner::new(&storage).run(&spec).unwrap_err().to_string();
    assert!(e.contains("candidate_state_exact") && e.contains("compared by the host that governs the run (Custodian"), "{e}");
    assert!(!dir.path().join("prepared").exists() && !storage.exists(), "nothing ran, nothing was stored");
    // and through run_with, the standalone host says the same
    let e = ExperimentRunner::new(&storage).run_with(&spec, &mut experiments::Standalone).unwrap_err().to_string();
    assert!(e.contains("experiments on its own does not compare trees"), "{e}");
}

/// A host that answers an exact tree measurement with a record and runs nothing else.
#[derive(Default)]
struct Comparing {
    asked: Vec<ExactTreeRequest>,
    equal: bool,
}

impl ExperimentHost for Comparing {
    fn run_command(&mut self, _: &str, c: &CommandSpec, _: &Path) -> Result<CommandRecord, ExperimentError> {
        Err(ExperimentError::Other(format!("no command here: {}", c.label)))
    }

    fn apply_change(&mut self, _: &ChangeRequest) -> Result<(MutationRecord, Vec<Evidence>), String> {
        Err("no change here".into())
    }

    fn compare_exact_tree(&mut self, r: &ExactTreeRequest) -> Result<ExactTreeRecord, ExperimentError> {
        self.asked.push(r.clone());
        Ok(ExactTreeRecord {
            phase: r.phase.clone(),
            name: r.name.clone(),
            path: r.path.clone(),
            equals_tree: r.equals_tree.clone(),
            observed_digest: "sha256:aa".into(),
            expected_digest: if self.equal { "sha256:aa" } else { "sha256:bb" }.into(),
            observed_files: 2,
            expected_files: 2,
            equal: self.equal,
            differing_paths: if self.equal { Vec::new() } else { vec!["~src/lib.rs".into()] },
            excluded: Vec::new(),
        })
    }
}

#[test]
fn a_hosts_exact_tree_record_is_the_measurement_and_the_result_keeps_it() {
    let dir = tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    for equal in [true, false] {
        let mut host = Comparing { equal, ..Comparing::default() };
        let (result, file) = ExperimentRunner::new(root.join("storage")).run_with(&exact_spec(&root), &mut host).unwrap();
        // the tree measured where a command runs, the tree it must equal where a change spec is
        assert_eq!(host.asked.len(), 1);
        assert_eq!((host.asked[0].phase.as_str(), host.asked[0].path.as_path(), host.asked[0].equals_tree.as_path()), ("measurement:candidate", root.join("specimen").as_path(), root.join("../expected").as_path()));
        let o = result.observations.iter().find(|o| o.measurement.name == "candidate_state_exact").unwrap();
        assert_eq!(o.measurement.value, Scalar::Boolean(equal));
        assert_eq!(result.promotion, if equal { PromotionDecision::Promote } else { PromotionDecision::DoNotPromote });
        assert_eq!(result.exact_trees.len(), 1);
        assert_eq!(result.exact_trees[0].differing_paths.is_empty(), equal);
        assert!(result.commands.is_empty(), "no command ran");
        assert_eq!(read_result(&file).unwrap(), result, "kept in the result as made");
    }
    // a result written before (no exact_trees) still reads, and one with none writes no key
    let (plain, file) = ExperimentRunner::new(root.join("storage")).run(&common::base_spec(&root)).unwrap();
    assert!(plain.exact_trees.is_empty() && !fs::read_to_string(&file).unwrap().contains("exact_trees"));
}

#[test]
fn a_campaigns_binding_resolves_equals_tree_where_a_change_spec_is_resolved() {
    let dir = tempdir().unwrap();
    let root = dir.path().join("workspace");
    let base = dir.path().join("campaign");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&base).unwrap();
    let root = root.canonicalize().unwrap();
    let mut s = exact_spec(Path::new("."));
    s.id = "x".into();
    s.mutation.mode = experiments::MutationMode::Bounded;
    s.mutation.changes.push(experiments::MutationChange { label: "replay".into(), spec: "../changes/x/change.toml".into() });
    fs::create_dir_all(dir.path().join("changes/x")).unwrap();
    fs::write(dir.path().join("changes/x/change.toml"), "name = \"x\"\n").unwrap();
    write_spec(base.join("x.json"), &s).unwrap();
    let campaign = base.join("c.json");
    fs::write(&campaign, r#"{"id":"c","steps":[{"run":{"id":"one","spec":"x.json"}}]}"#).unwrap();
    let loaded = LoadedCampaign::load(&campaign, &root).unwrap();
    let ws = dir.path().join("elsewhere");
    let bound = loaded.bound("one", &ws).unwrap();
    let e = bound.measurements.measurements[0].exact_tree().unwrap();
    assert_eq!(e.path, Path::new("specimen"), "measured in the workspace it runs in");
    assert_eq!(e.equals_tree, dir.path().canonicalize().unwrap().join("expected"), "where the experiment declared it, against its declared target");
    assert_eq!(bound.mutation.changes[0].spec, dir.path().canonicalize().unwrap().join("changes/x/change.toml"), "the same rule as its change spec");
    let _ = Binding::of(loaded.plan.node("one").unwrap());
}
