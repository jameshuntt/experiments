use crate::{
    differences, CommandRecord, CommandSpec, Evidence, ExperimentPaths, ExperimentResult,
    ExperimentSpec, MeasurementParser, MeasurementPhase, MutationBackend, MutationMode,
    MutationRecord, PromotionDecision, VerificationResult,
};
use scientific_method::{Measurement, Observation, Observations, Outcome, Scalar};
use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use xccute_runtime::{CommandSpec as XccuteCommandSpec, ShellCommand};

#[derive(Debug)]
pub enum ExperimentError {
    Io(std::io::Error),
    Scientific(scientific_method::CriterionError),
    Measurement { name: String, detail: String },
    CommandFailed { label: String, status: i32 },
    Contract { label: String, detail: String },
    Mutation(String),
    Other(String),
}

impl fmt::Display for ExperimentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Scientific(error) => write!(f, "{error}"),
            Self::Measurement { name, detail } => write!(f, "measurement {name}: {detail}"),
            Self::CommandFailed { label, status } => {
                write!(f, "command {label} failed with status {status}")
            }
            Self::Contract { label, detail } => write!(f, "command contract {label}: {detail}"),
            Self::Mutation(message) => write!(f, "mutation: {message}"),
            Self::Other(message) => f.write_str(message),
        }
    }
}

impl Error for ExperimentError {}

impl From<std::io::Error> for ExperimentError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<scientific_method::CriterionError> for ExperimentError {
    fn from(error: scientific_method::CriterionError) -> Self {
        Self::Scientific(error)
    }
}

/// Where an experiment's operations run. The runner decides what runs and in which order; a host carries out
/// each operation and reports what happened. [`Standalone`] is this crate's own host: commands through XCCUTE's
/// argv-safe runtime contract, bounded changes through Code Passenger's declared transform line. A host that
/// governs the operations (Custodian) runs the same operations under its own receipts.
pub trait ExperimentHost {
    /// Runs one protocol, measurement or verification command in `cwd` (already resolved against the target
    /// root). A nonzero status is a record, not an error: the runner decides what it means.
    fn run_command(
        &mut self,
        phase: &str,
        command: &CommandSpec,
        cwd: &Path,
    ) -> Result<CommandRecord, ExperimentError>;

    /// Makes one bounded structured change and returns its record and the evidence it left. A change that did
    /// not complete is an error, and nothing after it runs.
    fn apply_change(
        &mut self,
        change: &ChangeRequest,
    ) -> Result<(MutationRecord, Vec<Evidence>), String>;
}

/// One structured change, as the runner asks a host to make it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeRequest {
    /// The change's place in `mutation.changes`.
    pub index: usize,
    pub label: String,
    pub backend: MutationBackend,
    /// The target root, absolute.
    pub root: PathBuf,
    /// The change spec, absolute.
    pub spec: PathBuf,
    /// Where this run keeps the change's transform record (the run's evidence folder).
    pub record: PathBuf,
}

/// This crate's own host: the operations run in this process, outside any governing host.
#[derive(Debug, Clone, Copy, Default)]
pub struct Standalone;

impl ExperimentHost for Standalone {
    fn run_command(
        &mut self,
        phase: &str,
        command: &CommandSpec,
        cwd: &Path,
    ) -> Result<CommandRecord, ExperimentError> {
        run_command_here(phase, command, cwd)
    }

    fn apply_change(
        &mut self,
        change: &ChangeRequest,
    ) -> Result<(MutationRecord, Vec<Evidence>), String> {
        crate::mutation::code_passenger_change(change)
    }
}

pub struct ExperimentRunner {
    storage_base: PathBuf,
}

impl ExperimentRunner {
    pub fn new(storage_base: impl Into<PathBuf>) -> Self {
        Self {
            storage_base: storage_base.into(),
        }
    }

    /// Runs the experiment on this crate's own host ([`Standalone`]).
    pub fn run(
        &self,
        spec: &ExperimentSpec,
    ) -> Result<(ExperimentResult, PathBuf), ExperimentError> {
        self.run_with(spec, &mut Standalone)
    }

    /// Runs the experiment with each command and each bounded change carried out by `host`.
    pub fn run_with(
        &self,
        spec: &ExperimentSpec,
        host: &mut dyn ExperimentHost,
    ) -> Result<(ExperimentResult, PathBuf), ExperimentError> {
        let run_id = run_id(&spec.id)?;
        let paths = ExperimentPaths::under(&self.storage_base)
            .map_err(|error| ExperimentError::Other(error.to_string()))?;
        paths.ensure_run(&run_id)?;

        crate::mutation::validate(spec).map_err(ExperimentError::Mutation)?;

        let mut records = Vec::new();
        let mut observations = Observations::new();

        run_phase(
            host,
            "prepare",
            &spec.protocol.prepare,
            &spec.target.root,
            &mut records,
            true,
        )?;
        run_phase(
            host,
            "baseline",
            &spec.protocol.baseline,
            &spec.target.root,
            &mut records,
            true,
        )?;
        collect_measurements(
            host,
            MeasurementPhase::Baseline,
            spec,
            &mut observations,
            &mut records,
        )?;

        if spec.mutation.mode == MutationMode::Unbounded {
            run_phase(
                host,
                "intervention",
                &spec.protocol.intervention,
                &spec.target.root,
                &mut records,
                true,
            )?;
        }
        let (mutation_records, evidence) = crate::mutation::run(spec, &paths, &run_id, host)
            .map_err(ExperimentError::Mutation)?;

        run_phase(
            host,
            "candidate",
            &spec.protocol.candidate,
            &spec.target.root,
            &mut records,
            true,
        )?;
        collect_measurements(
            host,
            MeasurementPhase::Candidate,
            spec,
            &mut observations,
            &mut records,
        )?;

        let mut verification_records = Vec::new();
        for command in &spec.verification.commands {
            verification_records.push(run_command(
                host,
                "verification",
                command,
                &spec.target.root,
            )?);
        }
        let verification = VerificationResult {
            passed: verification_records.iter().all(|record| record.status == 0),
            commands: verification_records.clone(),
        };
        records.extend(verification_records);

        let evaluation = spec.scientific_method.evaluate(&observations)?;

        let mut difference_reports = Vec::new();
        for request in &spec.differences {
            difference_reports.push(
                differences::run(&spec.target.root, request)
                    .map_err(|detail| ExperimentError::Other(format!("differences: {detail}")))?,
            );
        }

        let promotion = decide(spec, &evaluation.conclusion.outcome, verification.passed);

        // Cleanup is evidence too, but cleanup failure does not rewrite the
        // already-produced scientific conclusion.
        run_phase(
            host,
            "cleanup",
            &spec.protocol.cleanup,
            &spec.target.root,
            &mut records,
            false,
        )?;

        let result = ExperimentResult {
            schema: crate::RESULT_SCHEMA.to_owned(),
            experiment_id: spec.id.clone(),
            run_id: run_id.clone(),
            observations,
            evaluation,
            verification,
            commands: records,
            mutations: mutation_records,
            differences: difference_reports,
            evidence,
            promotion,
        };

        let result_path = paths.result(&run_id);
        crate::write_result(&result_path, &result)?;
        Ok((result, result_path))
    }
}

fn collect_measurements(
    host: &mut dyn ExperimentHost,
    phase: MeasurementPhase,
    spec: &ExperimentSpec,
    observations: &mut Observations,
    records: &mut Vec<CommandRecord>,
) -> Result<(), ExperimentError> {
    for measurement_spec in spec
        .measurements
        .measurements
        .iter()
        .filter(|measurement| measurement.phase == phase)
    {
        let phase_name = match phase {
            MeasurementPhase::Baseline => "measurement:baseline",
            MeasurementPhase::Candidate => "measurement:candidate",
        };
        let record = run_command(host, phase_name, &measurement_spec.command, &spec.target.root)?;
        let value = parse_measurement(
            &measurement_spec.name,
            measurement_spec.parser.clone(),
            &record,
        )?;
        let mut measurement = Measurement::new(&measurement_spec.name, value);
        if let Some(unit) = &measurement_spec.unit {
            measurement = measurement.with_unit(unit);
        }
        observations.insert(Observation::new(
            &measurement_spec.condition,
            measurement,
        ));
        records.push(record);
    }
    Ok(())
}

fn run_phase(
    host: &mut dyn ExperimentHost,
    phase: &str,
    commands: &[CommandSpec],
    root: &Path,
    records: &mut Vec<CommandRecord>,
    fail_fast: bool,
) -> Result<(), ExperimentError> {
    for command in commands {
        let record = run_command(host, phase, command, root)?;
        let status = record.status;
        let label = record.label.clone();
        records.push(record);
        if fail_fast && status != 0 {
            return Err(ExperimentError::CommandFailed { label, status });
        }
    }
    Ok(())
}

/// A command's contract checked and its directory resolved against the target root, then run by the host.
fn run_command(
    host: &mut dyn ExperimentHost,
    phase: &str,
    spec: &CommandSpec,
    root: &Path,
) -> Result<CommandRecord, ExperimentError> {
    if spec.program.is_empty() {
        return Err(ExperimentError::Contract {
            label: spec.label.clone(),
            detail: "program is empty".into(),
        });
    }

    let cwd = spec
        .cwd
        .as_deref()
        .map(|path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            }
        })
        .unwrap_or_else(|| root.to_path_buf());
    host.run_command(phase, spec, &cwd)
}

/// [`Standalone`]'s command: run in this process.
fn run_command_here(
    phase: &str,
    spec: &CommandSpec,
    cwd: &Path,
) -> Result<CommandRecord, ExperimentError> {
    // Dynamic experiment operations lower into XCCUTE's argv-safe runtime
    // contract. XCCUTE owns conversion to the actual process; experiments does
    // not construct a raw Command or shell string.
    let contract = XccuteCommandSpec::new(spec.program.clone()).args(spec.args.clone());
    let display = contract.build();
    let mut command = contract.to_command();
    let output = command.current_dir(cwd).output()?;

    Ok(CommandRecord {
        phase: phase.to_owned(),
        label: spec.label.clone(),
        program: spec.program.clone(),
        args: spec.args.clone(),
        cwd: cwd.to_path_buf(),
        display,
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn parse_measurement(
    name: &str,
    parser: MeasurementParser,
    record: &CommandRecord,
) -> Result<Scalar, ExperimentError> {
    let text = record.stdout.trim();
    match parser {
        MeasurementParser::Number => {
            text.parse::<f64>()
                .map(Scalar::Number)
                .map_err(|error| ExperimentError::Measurement {
                    name: name.to_owned(),
                    detail: error.to_string(),
                })
        }
        MeasurementParser::Boolean => {
            text.parse::<bool>()
                .map(Scalar::Boolean)
                .map_err(|error| ExperimentError::Measurement {
                    name: name.to_owned(),
                    detail: error.to_string(),
                })
        }
        MeasurementParser::Text => Ok(Scalar::Text(text.to_owned())),
        MeasurementParser::ExitSuccess => Ok(Scalar::Boolean(record.status == 0)),
    }
}

fn decide(
    spec: &ExperimentSpec,
    outcome: &Outcome,
    verification: bool,
) -> PromotionDecision {
    if spec.promotion.require_supported && *outcome != Outcome::Supported {
        return PromotionDecision::DoNotPromote;
    }
    if spec.promotion.require_verification && !verification {
        return PromotionDecision::DoNotPromote;
    }
    if *outcome == Outcome::Inconclusive {
        PromotionDecision::Undecided
    } else {
        PromotionDecision::Promote
    }
}

fn run_id(experiment_id: &str) -> Result<String, ExperimentError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| ExperimentError::Other(error.to_string()))?;
    let clean: String = experiment_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect();
    Ok(format!(
        "{clean}-{}-{:09}",
        now.as_secs(),
        now.subsec_nanos()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ExperimentTarget, MeasurementPlan, PromotionPolicy, Protocol, Verification,
    };
    use scientific_method::{Comparator, Criterion, Hypothesis, Operand, ScientificMethod};

    #[test]
    fn promotion_is_separate_from_scientific_outcome() {
        let spec = ExperimentSpec {
            schema: crate::SCHEMA.to_owned(),
            id: "separation".into(),
            description: None,
            scientific_method: ScientificMethod::new(Hypothesis::new("candidate is valid"))
                .criterion(Criterion::compare(
                    Operand::observation("candidate", "tests_pass"),
                    Comparator::Equal,
                    Operand::literal(true),
                )),
            target: ExperimentTarget {
                root: ".".into(),
                label: None,
            },
            protocol: Protocol::default(),
            mutation: crate::MutationPlan::default(),
            verification: Verification::default(),
            measurements: MeasurementPlan::default(),
            differences: Vec::new(),
            promotion: PromotionPolicy::default(),
        };
        assert_eq!(
            decide(&spec, &Outcome::NotSupported, true),
            PromotionDecision::DoNotPromote
        );
        assert_eq!(
            decide(&spec, &Outcome::Supported, true),
            PromotionDecision::Promote
        );
    }
}
