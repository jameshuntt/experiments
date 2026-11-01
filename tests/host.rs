mod common;

use experiments::{
    ChangeRequest, CommandRecord, CommandSpec, Evidence, ExperimentError, ExperimentHost,
    ExperimentRunner, MeasurementParser, MeasurementPhase, MeasurementPlan, MeasurementSpec,
    MutationChange, MutationMode, MutationPlan, MutationRecord, PromotionDecision,
};
use scientific_method::{Direction, Hypothesis, ObservationRef, ScientificMethod};
use std::path::{Path, PathBuf};
use tempfile::tempdir;

/// A host that runs nothing itself: every command and change is answered from here, so a program that does not
/// exist on this machine proves the runner never ran it on its own.
#[derive(Default)]
struct Answering {
    commands: Vec<(String, String, PathBuf)>,
    changes: Vec<ChangeRequest>,
}

impl ExperimentHost for Answering {
    fn run_command(
        &mut self,
        phase: &str,
        command: &CommandSpec,
        cwd: &Path,
    ) -> Result<CommandRecord, ExperimentError> {
        self.commands
            .push((phase.to_owned(), command.label.clone(), cwd.to_path_buf()));
        let stdout = match phase {
            "measurement:baseline" => "100\n",
            "measurement:candidate" => "90\n",
            _ => "",
        };
        Ok(CommandRecord {
            phase: phase.to_owned(),
            label: command.label.clone(),
            program: command.program.clone(),
            args: command.args.clone(),
            cwd: cwd.to_path_buf(),
            display: format!("hosted {}", command.program),
            status: 0,
            stdout: stdout.into(),
            stderr: String::new(),
        })
    }

    fn apply_change(
        &mut self,
        change: &ChangeRequest,
    ) -> Result<(MutationRecord, Vec<Evidence>), String> {
        self.changes.push(change.clone());
        let record = MutationRecord {
            label: change.label.clone(),
            backend: change.backend,
            spec: change.spec.clone(),
            display: "hosted transform".into(),
            status: 0,
            status_word: "done".into(),
            stdout: String::new(),
            stderr: String::new(),
            transform_record: Some(change.record.clone()),
        };
        let evidence = vec![Evidence {
            kind: "hosted_transform".into(),
            name: change.label.clone(),
            path: change.record.clone(),
        }];
        Ok((record, evidence))
    }
}

#[test]
fn a_host_carries_out_every_command_and_every_bounded_change() {
    let target = tempdir().expect("target");
    let storage = tempdir().expect("storage");
    let absent = "experiments-no-such-program";
    let mut spec = common::base_spec(target.path());
    spec.id = "hosted".into();
    spec.scientific_method = ScientificMethod::new(Hypothesis::new("candidate lowers latency"))
        .criterion(scientific_method::Criterion::improvement(
            ObservationRef::new("baseline", "latency"),
            ObservationRef::new("candidate", "latency"),
            Direction::LowerIsBetter,
            0.05,
        ));
    spec.protocol.prepare = vec![common::command("prepare", absent, &[])];
    spec.protocol.baseline = vec![common::command("baseline", absent, &[])];
    spec.protocol.candidate = vec![common::command("candidate", absent, &["--x"])];
    spec.protocol.cleanup = vec![common::command("cleanup", absent, &[])];
    let measure = |phase, condition: &str| MeasurementSpec {
        phase,
        condition: condition.into(),
        name: "latency".into(),
        unit: None,
        command: common::command(condition, absent, &[]),
        parser: MeasurementParser::Number,
    };
    spec.measurements = MeasurementPlan {
        measurements: vec![
            measure(MeasurementPhase::Baseline, "baseline"),
            measure(MeasurementPhase::Candidate, "candidate"),
        ],
    };
    spec.verification.commands = vec![common::command("verify", absent, &[])];
    spec.mutation = MutationPlan {
        mode: MutationMode::Bounded,
        changes: vec![MutationChange {
            label: "one pass".into(),
            spec: "change.toml".into(),
        }],
        ..MutationPlan::default()
    };

    let mut host = Answering::default();
    let (result, path) = ExperimentRunner::new(storage.path())
        .run_with(&spec, &mut host)
        .expect("the hosted run completes");

    let phases: Vec<&str> = host
        .commands
        .iter()
        .map(|(phase, _, _)| phase.as_str())
        .collect();
    assert_eq!(
        phases,
        [
            "prepare",
            "baseline",
            "measurement:baseline",
            "candidate",
            "measurement:candidate",
            "verification",
            "cleanup"
        ]
    );
    assert!(host.commands.iter().all(|(_, _, cwd)| cwd == target.path()));
    assert_eq!(host.changes.len(), 1);
    let change = &host.changes[0];
    assert_eq!(
        change.spec,
        target.path().join("change.toml"),
        "the change spec, absolute"
    );
    assert!(
        change.record.starts_with(storage.path()),
        "the record goes with the run's evidence"
    );
    assert_eq!(result.mutations[0].display, "hosted transform");
    assert_eq!(result.evidence[0].kind, "hosted_transform");
    assert_eq!(result.commands.len(), 7);
    assert_eq!(result.promotion, PromotionDecision::Promote);
    assert_eq!(
        experiments::read_result(&path).expect("result reads back"),
        result
    );
}

#[test]
fn a_change_the_host_refuses_stops_the_run_before_the_candidate() {
    struct Refusing(Vec<String>);
    impl ExperimentHost for Refusing {
        fn run_command(
            &mut self,
            phase: &str,
            command: &CommandSpec,
            cwd: &Path,
        ) -> Result<CommandRecord, ExperimentError> {
            self.0.push(phase.to_owned());
            Answering::default().run_command(phase, command, cwd)
        }
        fn apply_change(
            &mut self,
            _change: &ChangeRequest,
        ) -> Result<(MutationRecord, Vec<Evidence>), String> {
            Err("not authorized".into())
        }
    }
    let target = tempdir().expect("target");
    let storage = tempdir().expect("storage");
    let mut spec = common::base_spec(target.path());
    spec.protocol.candidate = vec![common::command(
        "candidate",
        "experiments-no-such-program",
        &[],
    )];
    spec.mutation = MutationPlan {
        mode: MutationMode::Bounded,
        changes: vec![MutationChange {
            label: "refused".into(),
            spec: "change.toml".into(),
        }],
        ..MutationPlan::default()
    };
    let mut host = Refusing(Vec::new());
    let error = ExperimentRunner::new(storage.path())
        .run_with(&spec, &mut host)
        .expect_err("a refused change ends the run");
    assert!(
        matches!(error, ExperimentError::Mutation(ref m) if m == "not authorized"),
        "{error}"
    );
    assert!(!host.0.contains(&"candidate".to_owned()), "{:?}", host.0);
}
