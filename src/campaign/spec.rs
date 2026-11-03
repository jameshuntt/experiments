//! The campaign document: what runs, over which state, with which factor assignments, controls and blocks, how
//! rigorously, and under which policies. Every type refuses a key it does not know (`deny_unknown_fields`), in every
//! format: a typo never becomes a factor, a step or a policy.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// The campaign document's schema.
pub const CAMPAIGN_SCHEMA: &str = "experiments.campaign.v1";
/// The campaign result's schema.
pub const CAMPAIGN_RESULT_SCHEMA: &str = "experiments.campaign-result.v1";
/// The normalized plan's schema.
pub const CAMPAIGN_PLAN_SCHEMA: &str = "experiments.campaign-plan.v1";
/// A campaign run's progress (what resume reads).
pub const CAMPAIGN_PROGRESS_SCHEMA: &str = "experiments.campaign-progress.v1";

fn campaign_schema() -> String {
    CAMPAIGN_SCHEMA.to_owned()
}

/// A campaign: experiments composed over an evolving accepted state, and experimental designs over factors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignSpec {
    #[serde(default = "campaign_schema")]
    pub schema: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub rigor: Rigor,
    #[serde(default)]
    pub policy: CampaignPolicy,
    /// Independent variables: each a name, its levels, and its scope.
    #[serde(default)]
    pub factors: Vec<Factor>,
    /// Values held fixed for every trial.
    #[serde(default)]
    pub controls: Vec<ControlValue>,
    /// Nuisance variables a trial is grouped by, never treatments.
    #[serde(default)]
    pub blocks: Vec<BlockFactor>,
    /// Paths (relative to the governed root) left out of the scientific state's identity on top of the host's own
    /// runtime paths: an experiment's run output kept in the tree. Recorded in the plan and the result.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// The composition, in order.
    #[serde(default)]
    pub steps: Vec<Step>,
}

/// How much a campaign establishes before it accepts a candidate. Never the mutation mode: a rigor says what the
/// campaign checks and refuses; `bounded`/`unbounded` is an experiment's own declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Rigor {
    /// Provenance, assignments, measurements and results are still required; an undeclared change is recorded, not
    /// refused; Custodian is never bypassed.
    Exploratory,
    /// Declared manipulated variables, controls, source state and treatments; trials isolated; an undeclared change
    /// is a finding and the candidate is not accepted.
    #[default]
    Controlled,
    /// Exact starting state, only declared differences, a value for every control the experiment declares, bounded
    /// Code Passenger mutation only, explicit tool and spec digests; refused when any of these cannot be established.
    Strict,
}

impl Rigor {
    pub fn word(self) -> &'static str {
        match self {
            Rigor::Exploratory => "exploratory",
            Rigor::Controlled => "controlled",
            Rigor::Strict => "strict",
        }
    }
}

/// Whether a campaign goes on after a node of some kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Continuation {
    Continue,
    Stop,
}

/// What an undecided experiment's candidate becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OnUndecided {
    /// The previous accepted state is put back (the default).
    #[default]
    Restore,
    /// The candidate is accepted.
    Accept,
}

impl Continuation {
    pub fn word(self) -> &'static str {
        match self {
            Continuation::Continue => "continue",
            Continuation::Stop => "stop",
        }
    }
}

impl OnUndecided {
    pub fn word(self) -> &'static str {
        match self {
            OnUndecided::Restore => "restore",
            OnUndecided::Accept => "accept",
        }
    }
}

fn continue_() -> Continuation {
    Continuation::Continue
}

fn stop() -> Continuation {
    Continuation::Stop
}

fn one() -> u32 {
    1
}

/// The campaign's policies. Promotion and state are separate from the scientific conclusion and from execution
/// success: a do-not-promote is a finding, never a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignPolicy {
    /// After a do-not-promote, an undecided, or a finding of the rigor (continue by default).
    #[serde(default = "continue_")]
    pub on_findings: Continuation,
    /// After a node the host refused (stop by default).
    #[serde(default = "stop")]
    pub on_refusal: Continuation,
    /// After a node that could not run to its conclusion (stop by default).
    #[serde(default = "stop")]
    pub on_error: Continuation,
    #[serde(default)]
    pub on_undecided: OnUndecided,
    /// At most this many nodes run at once (isolated workspaces only; the shared workspace has one writer).
    #[serde(default = "one")]
    pub max_parallel: u32,
}

impl Default for CampaignPolicy {
    fn default() -> Self {
        CampaignPolicy { on_findings: continue_(), on_refusal: stop(), on_error: stop(), on_undecided: OnUndecided::Restore, max_parallel: 1 }
    }
}

/// A typed level: the value a factor, a control or a block takes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Level {
    Boolean(bool),
    Integer(i64),
    Number(f64),
    Text(String),
}

impl Level {
    /// The level's kind, as a word.
    pub fn kind(&self) -> &'static str {
        match self {
            Level::Boolean(_) => "boolean",
            Level::Integer(_) => "integer",
            Level::Number(_) => "number",
            Level::Text(_) => "text",
        }
    }
}

impl fmt::Display for Level {
    /// The level's one text form: what a command is given and what the plan shows.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Level::Boolean(v) => write!(f, "{v}"),
            Level::Integer(v) => write!(f, "{v}"),
            Level::Number(v) => write!(f, "{v:?}"),
            Level::Text(v) => f.write_str(v),
        }
    }
}

/// Where a factor's name must be declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FactorScope {
    /// An independent variable every experiment it is assigned to declares (`scientific_method.independent_variables`).
    #[default]
    Experiment,
    /// The campaign's own: no experiment needs to declare it.
    Campaign,
}

impl FactorScope {
    pub fn word(self) -> &'static str {
        match self {
            FactorScope::Experiment => "experiment",
            FactorScope::Campaign => "campaign",
        }
    }
}

/// An independent variable (scientific-method's word) the campaign manipulates: its levels and its scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Factor {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub levels: Vec<Level>,
    #[serde(default)]
    pub scope: FactorScope,
}

/// A control: a value held fixed for every trial.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlValue {
    pub name: String,
    pub value: Level,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A block (nuisance) variable: trials are grouped by its levels, never treated by them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockFactor {
    pub name: String,
    pub levels: Vec<Level>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// One factor set to one of its levels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assignment {
    pub factor: String,
    pub value: Level,
}

/// One block set to one of its levels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockAssignment {
    pub name: String,
    pub level: Level,
}

/// Where a node's candidate is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceStrategy {
    /// The governed project's one accepted state, evolving: a promoted candidate becomes the next node's start.
    Shared,
    /// A new governed workspace, made by the host from the exact accepted state of the node's state parent.
    Isolated,
    /// The isolated workspace of the node's state parent, evolving in place.
    Continue,
}

impl WorkspaceStrategy {
    pub fn word(self) -> &'static str {
        match self {
            WorkspaceStrategy::Shared => "shared",
            WorkspaceStrategy::Isolated => "isolated",
            WorkspaceStrategy::Continue => "continue",
        }
    }
}

/// One step of the composition. Written in every format as a one-key map, its kind the key
/// (`{ "run": { ... } }`, `- run: ...`, `run { ... }`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "StepForm", into = "StepForm")]
pub enum Step {
    /// One experiment.
    Run(RunStep),
    /// Steps in declared order, each starting from the accepted state the one before left.
    Sequence(GroupStep),
    /// Branches from one source state, each in its own isolated workspace, no merge.
    Parallel(GroupStep),
    /// N independent trials of one treatment.
    Repeat(RepeatStep),
    /// A design's trials, expanded before anything runs.
    Sweep(SweepStep),
}

/// A step as every format writes it: one key, the step's kind.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StepForm {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    run: Option<RunStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sequence: Option<GroupStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parallel: Option<GroupStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    repeat: Option<RepeatStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sweep: Option<SweepStep>,
}

impl TryFrom<StepForm> for Step {
    type Error = String;

    fn try_from(f: StepForm) -> Result<Step, String> {
        let mut found: Vec<Step> = Vec::new();
        found.extend(f.run.map(Step::Run));
        found.extend(f.sequence.map(Step::Sequence));
        found.extend(f.parallel.map(Step::Parallel));
        found.extend(f.repeat.map(Step::Repeat));
        found.extend(f.sweep.map(Step::Sweep));
        match found.len() {
            1 => Ok(found.remove(0)),
            0 => Err("a step is one of run, sequence, parallel, repeat, sweep".into()),
            _ => Err("a step is exactly one of run, sequence, parallel, repeat, sweep".into()),
        }
    }
}

impl From<Step> for StepForm {
    fn from(s: Step) -> StepForm {
        match s {
            Step::Run(r) => StepForm { run: Some(r), ..StepForm::default() },
            Step::Sequence(g) => StepForm { sequence: Some(g), ..StepForm::default() },
            Step::Parallel(g) => StepForm { parallel: Some(g), ..StepForm::default() },
            Step::Repeat(r) => StepForm { repeat: Some(r), ..StepForm::default() },
            Step::Sweep(w) => StepForm { sweep: Some(w), ..StepForm::default() },
        }
    }
}

/// One experiment: the spec it runs, the factor levels it sets, its blocks. `after`, `from` and `workspace` are the
/// flat form's; inside a sequence, a parallel or a repeat the composition decides them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunStep {
    pub id: String,
    /// The experiment spec, relative to the campaign file.
    pub spec: PathBuf,
    #[serde(default)]
    pub set: Vec<Assignment>,
    #[serde(default)]
    pub blocks: Vec<BlockAssignment>,
    /// Nodes or groups that must finish first.
    #[serde(default)]
    pub after: Vec<String>,
    /// The one node (or sequence) whose accepted state this run starts from; the campaign's start when none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceStrategy>,
}

/// A sequence or a parallel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupStep {
    /// A name other steps' `after` (and, for a sequence, `from`) may use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default)]
    pub after: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// A sequence's workspace: shared (default) or isolated (its own lineage).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceStrategy>,
    pub steps: Vec<Step>,
}

/// N independent trials of one run, each from the same source state in its own isolated workspace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepeatStep {
    pub id: String,
    pub times: u32,
    #[serde(default)]
    pub after: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    /// The treatment: its id names the treatment, its `set` the assignment.
    pub run: RunStep,
}

/// An experimental design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Design {
    /// Every combination of the named factors' levels.
    Factorial,
    /// The reference treatment, then each named factor moved alone through its other levels, every other factor at
    /// its reference value.
    OneFactorAtATime,
    /// The treatments as listed.
    Explicit,
}

impl Design {
    pub fn word(self) -> &'static str {
        match self {
            Design::Factorial => "factorial",
            Design::OneFactorAtATime => "one_factor_at_a_time",
            Design::Explicit => "explicit",
        }
    }
}

/// A named treatment of an explicit design.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Treatment {
    pub name: String,
    #[serde(default)]
    pub set: Vec<Assignment>,
}

/// A design over one experiment spec: its treatments, each repeated, in every combination of the named blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SweepStep {
    pub id: String,
    pub spec: PathBuf,
    pub design: Design,
    /// The factors a factorial or one-factor-at-a-time design moves, in order.
    #[serde(default)]
    pub factors: Vec<String>,
    /// One-factor-at-a-time: each moved factor's reference level.
    #[serde(default)]
    pub reference: Vec<Assignment>,
    /// Explicit: the treatments.
    #[serde(default)]
    pub treatments: Vec<Treatment>,
    /// Levels every treatment of the sweep holds (factors the design does not move).
    #[serde(default)]
    pub set: Vec<Assignment>,
    /// Independent executions of each treatment in each block.
    #[serde(default = "one")]
    pub repeats: u32,
    /// Block variables the design is crossed with (a complete block design).
    #[serde(default)]
    pub blocks: Vec<String>,
    #[serde(default)]
    pub after: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}
