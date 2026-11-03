use scientific_method::{MethodEvaluation, Observations, ScientificMethod};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SCHEMA: &str = "experiments.spec.v3";
pub const RESULT_SCHEMA: &str = "experiments.result.v3";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperimentTarget {
    pub root: PathBuf,
    #[serde(default)]
    pub label: Option<String>,
}

/// Declarative argv-safe operation. `program` and `args` are lowered into
/// `xccute_runtime::CommandSpec`; they are never concatenated into a shell line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSpec {
    pub label: String,
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<PathBuf>,
    /// Environment variables the program is given, in order, each exactly as written (no shell, no substitution).
    /// A campaign's binding of a trial's factor assignment, controls and blocks arrives here
    /// ([`crate::campaign::Binding`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<EnvVar>,
}

/// One environment variable a command's program is given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Protocol {
    #[serde(default)]
    pub prepare: Vec<CommandSpec>,
    #[serde(default)]
    pub baseline: Vec<CommandSpec>,
    /// In `bounded` mutation mode this must be empty: source changes are made
    /// only by the declared mutation backend. In `unbounded` mode these
    /// declared XCCUTE operations are permitted.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementPhase {
    Baseline,
    Candidate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeasurementSpec {
    /// When this observation is collected. Baseline observations are taken
    /// before intervention; candidate observations are taken after candidate.
    pub phase: MeasurementPhase,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MutationMode {
    /// All declared source-code intervention changes must go through the
    /// mutation backend. Raw `protocol.intervention` commands are refused.
    Bounded,
    /// Declared XCCUTE intervention commands may run directly. Structured
    /// backend changes can still be mixed in when desired.
    #[default]
    Unbounded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MutationBackend {
    /// Code Passenger proves a custodian_kit ChangeSpec on a scratch copy,
    /// writes it, checkpoints before/after, and emits a transform record.
    #[default]
    CodePassenger,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationChange {
    pub label: String,
    /// Code Passenger change spec, relative to the experiment target root
    /// unless absolute. Code Passenger accepts its normal 1o or TOML form.
    pub spec: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationPlan {
    #[serde(default)]
    pub mode: MutationMode,
    #[serde(default)]
    pub backend: MutationBackend,
    #[serde(default)]
    pub changes: Vec<MutationChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromotionPolicy {
    #[serde(default = "yes")]
    pub require_supported: bool,
    #[serde(default = "yes")]
    pub require_verification: bool,
}

const fn yes() -> bool {
    true
}

impl Default for PromotionPolicy {
    fn default() -> Self {
        Self {
            require_supported: true,
            require_verification: true,
        }
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
    pub mutation: MutationPlan,
    #[serde(default)]
    pub verification: Verification,
    #[serde(default)]
    pub measurements: MeasurementPlan,
    #[serde(default)]
    pub differences: Vec<crate::DifferencesRequest>,
    #[serde(default)]
    pub promotion: PromotionPolicy,
}

fn schema() -> String {
    SCHEMA.to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRecord {
    pub phase: String,
    pub label: String,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    /// The environment variables the program was given ([`CommandSpec::env`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<EnvVar>,
    /// XCCUTE's display rendering of the argv-safe command contract.
    pub display: String,
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationRecord {
    pub label: String,
    pub backend: MutationBackend,
    pub spec: PathBuf,
    pub display: String,
    pub status: i32,
    pub status_word: String,
    pub stdout: String,
    pub stderr: String,
    #[serde(default)]
    pub transform_record: Option<PathBuf>,
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
    pub mutations: Vec<MutationRecord>,
    pub differences: Vec<crate::DifferencesReport>,
    pub evidence: Vec<Evidence>,
    pub promotion: PromotionDecision,
}

fn result_schema() -> String {
    RESULT_SCHEMA.to_owned()
}
