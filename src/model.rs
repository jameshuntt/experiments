use scientific_method::{MethodEvaluation, Observations, ScientificMethod};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SCHEMA: &str = "experiments.spec.v1";
pub const RESULT_SCHEMA: &str = "experiments.result.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentTarget {
    pub root: PathBuf,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    pub label: String,
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Protocol {
    #[serde(default)]
    pub prepare: Vec<CommandSpec>,
    #[serde(default)]
    pub baseline: Vec<CommandSpec>,
    #[serde(default)]
    pub intervention: Vec<CommandSpec>,
    #[serde(default)]
    pub candidate: Vec<CommandSpec>,
    #[serde(default)]
    pub cleanup: Vec<CommandSpec>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verification {
    #[serde(default)]
    pub commands: Vec<CommandSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementParser {
    Number,
    Boolean,
    Text,
    ExitSuccess,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasurementSpec {
    pub condition: String,
    pub name: String,
    #[serde(default)]
    pub unit: Option<String>,
    pub command: CommandSpec,
    pub parser: MeasurementParser,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasurementPlan {
    #[serde(default)]
    pub measurements: Vec<MeasurementSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionPolicy {
    #[serde(default = "yes")]
    pub require_supported: bool,
    #[serde(default = "yes")]
    pub require_verification: bool,
}

const fn yes() -> bool { true }

impl Default for PromotionPolicy {
    fn default() -> Self {
        Self { require_supported: true, require_verification: true }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExperimentSpec {
    #[serde(default = "schema")]
    pub schema: String,
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    pub scientific_method: ScientificMethod,
    pub target: ExperimentTarget,
    #[serde(default)]
    pub protocol: Protocol,
    #[serde(default)]
    pub verification: Verification,
    #[serde(default)]
    pub measurements: MeasurementPlan,
    #[serde(default)]
    pub differences: Vec<crate::DifferencesRequest>,
    #[serde(default)]
    pub promotion: PromotionPolicy,
}

fn schema() -> String { SCHEMA.to_owned() }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRecord {
    pub phase: String,
    pub label: String,
    pub program: String,
    pub args: Vec<String>,
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationResult {
    pub passed: bool,
    pub commands: Vec<CommandRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub kind: String,
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionDecision {
    Promote,
    DoNotPromote,
    Undecided,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExperimentResult {
    #[serde(default = "result_schema")]
    pub schema: String,
    pub experiment_id: String,
    pub run_id: String,
    pub observations: Observations,
    pub evaluation: MethodEvaluation,
    pub verification: VerificationResult,
    pub commands: Vec<CommandRecord>,
    pub differences: Vec<crate::DifferencesReport>,
    pub evidence: Vec<Evidence>,
    pub promotion: PromotionDecision,
}

fn result_schema() -> String { RESULT_SCHEMA.to_owned() }
