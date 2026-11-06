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
#[serde(try_from = "MeasurementForm", into = "MeasurementForm")]
pub struct MeasurementSpec {
    /// When this observation is collected. Baseline observations are taken
    /// before intervention; candidate observations are taken after candidate.
    pub phase: MeasurementPhase,
    pub condition: String,
    pub name: String,
    pub unit: Option<String>,
    /// What the value is taken from: a command and its parser, or an exact tree equality.
    pub source: MeasurementSource,
}

/// What a measurement's value is taken from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MeasurementSource {
    /// A command's output or status, read by its parser.
    Command {
        command: CommandSpec,
        parser: MeasurementParser,
    },
    /// A boolean: whether one tree equals another exactly ([`ExactTree`]).
    ExactTree(ExactTree),
}

/// An exact tree measurement: true exactly when the tree at `path` and the tree at `equals_tree` hold the same paths,
/// each with the same bytes and the same mode. The comparison is the governing host's (Custodian's), which reads a
/// tree by its one rule; this crate does not read trees.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactTree {
    /// The tree measured, relative to the target root unless absolute (where a command runs when it names no `cwd`).
    pub path: PathBuf,
    /// The tree it must equal, relative to the experiment's target unless absolute: the rule and place a change
    /// spec's `spec` is resolved by, a campaign's binding included.
    pub equals_tree: PathBuf,
}

/// A measurement as every format writes it: `command` with `parser`, or `exact_tree`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MeasurementForm {
    phase: MeasurementPhase,
    condition: String,
    name: String,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    command: Option<CommandSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parser: Option<MeasurementParser>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    exact_tree: Option<ExactTree>,
}

impl TryFrom<MeasurementForm> for MeasurementSpec {
    type Error = String;

    fn try_from(f: MeasurementForm) -> Result<Self, String> {
        let source = match (f.command, f.parser, f.exact_tree) {
            (Some(command), Some(parser), None) => MeasurementSource::Command { command, parser },
            (None, None, Some(exact)) => MeasurementSource::ExactTree(exact),
            (None, None, None) => {
                return Err(format!(
                    "measurement `{}` names neither a command (with its parser) nor an exact_tree: it is one or the other",
                    f.name
                ));
            }
            (Some(_), _, Some(_)) | (_, Some(_), Some(_)) => {
                return Err(format!(
                    "measurement `{}` names both a command or parser and an exact_tree: it is one or the other",
                    f.name
                ));
            }
            (Some(_), None, None) => {
                return Err(format!(
                    "measurement `{}`: a command measurement needs its parser",
                    f.name
                ));
            }
            (None, Some(_), None) => {
                return Err(format!(
                    "measurement `{}`: a parser reads a command, and there is none",
                    f.name
                ));
            }
        };
        Ok(MeasurementSpec {
            phase: f.phase,
            condition: f.condition,
            name: f.name,
            unit: f.unit,
            source,
        })
    }
}

impl From<MeasurementSpec> for MeasurementForm {
    fn from(m: MeasurementSpec) -> Self {
        let (command, parser, exact_tree) = match m.source {
            MeasurementSource::Command { command, parser } => (Some(command), Some(parser), None),
            MeasurementSource::ExactTree(e) => (None, None, Some(e)),
        };
        MeasurementForm {
            phase: m.phase,
            condition: m.condition,
            name: m.name,
            unit: m.unit,
            command,
            parser,
            exact_tree,
        }
    }
}

impl MeasurementSpec {
    /// The command a command measurement runs; none for an exact tree measurement.
    pub fn command(&self) -> Option<&CommandSpec> {
        match &self.source {
            MeasurementSource::Command { command, .. } => Some(command),
            MeasurementSource::ExactTree(_) => None,
        }
    }

    pub fn command_mut(&mut self) -> Option<&mut CommandSpec> {
        match &mut self.source {
            MeasurementSource::Command { command, .. } => Some(command),
            MeasurementSource::ExactTree(_) => None,
        }
    }

    /// The exact tree an exact tree measurement compares; none for a command measurement.
    pub fn exact_tree(&self) -> Option<&ExactTree> {
        match &self.source {
            MeasurementSource::ExactTree(e) => Some(e),
            MeasurementSource::Command { .. } => None,
        }
    }
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

/// One exact tree measurement as the host made it: the two trees as read, each one's digest over its paths, bytes and
/// modes, how many files each holds, and every path where they differ (`+path` only measured, `-path` only expected,
/// `~path` in both with other bytes or another mode). `equal` is the measurement's value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactTreeRecord {
    /// `measurement:baseline` or `measurement:candidate`.
    pub phase: String,
    pub name: String,
    /// The tree measured, absolute.
    pub path: PathBuf,
    /// The tree it must equal, absolute.
    pub equals_tree: PathBuf,
    pub observed_digest: String,
    pub expected_digest: String,
    pub observed_files: usize,
    pub expected_files: usize,
    pub equal: bool,
    #[serde(default)]
    pub differing_paths: Vec<String>,
    /// What the host left out of both trees, as it said it (a campaign's `exclude`); empty when nothing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded: Vec<String>,
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
    /// Every exact tree measurement as the host made it, in the order measured.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exact_trees: Vec<ExactTreeRecord>,
}

fn result_schema() -> String {
    RESULT_SCHEMA.to_owned()
}
