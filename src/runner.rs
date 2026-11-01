use crate::{
    CommandRecord, CommandSpec, ExperimentPaths, ExperimentResult, ExperimentSpec,
    MeasurementParser, PromotionDecision, VerificationResult, differences,
};
use scientific_method::{Measurement, Observation, Observations, Outcome, Scalar};
use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug)]
pub enum ExperimentError {
    Io(std::io::Error),
    Scientific(scientific_method::CriterionError),
    Measurement { name: String, detail: String },
    CommandFailed { label: String, status: i32 },
    Other(String),
}

impl fmt::Display for ExperimentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Scientific(e) => write!(f, "{e}"),
            Self::Measurement { name, detail } => write!(f, "measurement {name}: {detail}"),
            Self::CommandFailed { label, status } => {
                write!(f, "command {label} failed with status {status}")
            }
            Self::Other(s) => f.write_str(s),
        }
    }
}
impl Error for ExperimentError {}
impl From<std::io::Error> for ExperimentError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<scientific_method::CriterionError> for ExperimentError {
    fn from(e: scientific_method::CriterionError) -> Self {
        Self::Scientific(e)
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

    pub fn run(
        &self,
        spec: &ExperimentSpec,
    ) -> Result<(ExperimentResult, PathBuf), ExperimentError> {
        let run_id = run_id(&spec.id)?;
        let paths = ExperimentPaths::under(&self.storage_base)
            .map_err(|e| ExperimentError::Other(e.to_string()))?;
        paths.ensure_run(&run_id)?;

        let mut records = Vec::new();
        run_phase(
            "prepare",
            &spec.protocol.prepare,
            &spec.target.root,
            &mut records,
            true,
        )?;
        run_phase(
            "baseline",
            &spec.protocol.baseline,
            &spec.target.root,
            &mut records,
            true,
        )?;
        run_phase(
            "intervention",
            &spec.protocol.intervention,
            &spec.target.root,
            &mut records,
            true,
        )?;
        run_phase(
            "candidate",
            &spec.protocol.candidate,
            &spec.target.root,
            &mut records,
            true,
        )?;

        let mut observations = Observations::new();
        for m in &spec.measurements.measurements {
            let record = run_command("measurement", &m.command, &spec.target.root)?;
            let value = parse_measurement(&m.name, m.parser.clone(), &record)?;
            let mut measurement = Measurement::new(&m.name, value);
            if let Some(unit) = &m.unit {
                measurement = measurement.with_unit(unit);
            }
            observations.insert(Observation::new(&m.condition, measurement));
            records.push(record);
        }

        let mut verification_records = Vec::new();
        for cmd in &spec.verification.commands {
            verification_records.push(run_command("verification", cmd, &spec.target.root)?);
        }
        let verification = VerificationResult {
            passed: verification_records.iter().all(|r| r.status == 0),
            commands: verification_records.clone(),
        };
        records.extend(verification_records);

        let evaluation = spec.scientific_method.evaluate(&observations)?;

        let mut difference_reports: Vec<crate::DifferencesReport> = Vec::new();
        #[cfg(feature = "differences")]
        for request in &spec.differences {
            difference_reports.push(differences::run(&spec.target.root, request)?);
        }

        let promotion = decide(spec, &evaluation.conclusion.outcome, verification.passed);

        // Cleanup is evidence too, but a cleanup failure should not rewrite the scientific result.
        run_phase(
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
            differences: difference_reports,
            evidence: Vec::new(),
            promotion,
        };
        let result_path = paths.result(&run_id);
        crate::write_result(&result_path, &result)?;
        Ok((result, result_path))
    }
}

fn run_phase(
    phase: &str,
    commands: &[CommandSpec],
    root: &Path,
    records: &mut Vec<CommandRecord>,
    fail_fast: bool,
) -> Result<(), ExperimentError> {
    for command in commands {
        let record = run_command(phase, command, root)?;
        let status = record.status;
        let label = record.label.clone();
        records.push(record);
        if fail_fast && status != 0 {
            return Err(ExperimentError::CommandFailed { label, status });
        }
    }
    Ok(())
}

fn run_command(
    phase: &str,
    spec: &CommandSpec,
    root: &Path,
) -> Result<CommandRecord, ExperimentError> {
    let cwd = spec
        .cwd
        .as_deref()
        .map(|p| {
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                root.join(p)
            }
        })
        .unwrap_or_else(|| root.to_path_buf());
    let output = Command::new(&spec.program)
        .args(&spec.args)
        .current_dir(cwd)
        .output()?;
    Ok(CommandRecord {
        phase: phase.to_owned(),
        label: spec.label.clone(),
        program: spec.program.clone(),
        args: spec.args.clone(),
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
                .map_err(|e| ExperimentError::Measurement {
                    name: name.to_owned(),
                    detail: e.to_string(),
                })
        }
        MeasurementParser::Boolean => {
            text.parse::<bool>()
                .map(Scalar::Boolean)
                .map_err(|e| ExperimentError::Measurement {
                    name: name.to_owned(),
                    detail: e.to_string(),
                })
        }
        MeasurementParser::Text => Ok(Scalar::Text(text.to_owned())),
        MeasurementParser::ExitSuccess => Ok(Scalar::Boolean(record.status == 0)),
    }
}

fn decide(spec: &ExperimentSpec, outcome: &Outcome, verification: bool) -> PromotionDecision {
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
        .map_err(|e| ExperimentError::Other(e.to_string()))?;
    let clean: String = experiment_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
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
    use crate::{ExperimentTarget, MeasurementPlan, PromotionPolicy, Protocol, Verification};
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
