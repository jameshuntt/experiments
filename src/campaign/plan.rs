//! One deterministic plan from a campaign: its composition (`run`, `sequence`, `parallel`, `repeat`, `sweep`, and
//! the flat form's `after`/`from`) normalized into nodes, each with its stable id, the exact experiment spec and its
//! digest, its dependency set, its one state parent, its workspace, its full factor assignment, controls and blocks,
//! its replicate index, its group and design. Everything is checked before anything runs: a plan that cannot be made
//! is refused with every reason, and nothing is touched. Nested and flat forms of the same composition make the same
//! plan, with the same digest.

use super::digest;
use super::spec::*;
use crate::{EnvVar, ExperimentSpec, MutationMode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Component, Path, PathBuf};

/// The most trials one sweep may expand to: a bigger design is refused before anything is made.
pub const MAX_TRIALS: usize = 100_000;

/// Why a campaign could not be planned (every reason), or could not go on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CampaignError(pub Vec<String>);

impl CampaignError {
    pub fn one(why: impl Into<String>) -> CampaignError {
        CampaignError(vec![why.into()])
    }
}

impl fmt::Display for CampaignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.join("; "))
    }
}

impl std::error::Error for CampaignError {}

/// One experiment spec the plan runs, as declared, with its digest: the spec as read (any of its formats reads the
/// same) and the bytes of every change spec it names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedSpec {
    /// As the campaign names it, relative to the campaign file.
    pub path: String,
    pub experiment_id: String,
    pub digest: String,
    pub mutation: MutationMode,
    /// Each change spec: its label and the digest of its bytes.
    pub changes: Vec<(String, String)>,
    /// The independent variables the experiment declares.
    pub independent_variables: Vec<String>,
    /// The controls the experiment declares.
    pub controls: Vec<String>,
}

/// A treatment of a group: its name, its assignment, and the one factor it moves (one-factor-at-a-time).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedTreatment {
    pub name: String,
    pub assignment: Vec<Assignment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub varied: Option<String>,
}

/// A repeat or a sweep: the trials a summary is made over.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedGroup {
    pub id: String,
    /// `repeat` or `sweep`.
    pub kind: String,
    pub spec: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design: Option<Design>,
    pub factors: Vec<String>,
    pub blocks: Vec<String>,
    pub repeats: u32,
    pub treatments: Vec<PlannedTreatment>,
    pub nodes: Vec<String>,
}

/// A workspace a plan's nodes run in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedWorkspace {
    /// `shared`, or `isolated:<the node that makes it>`.
    pub id: String,
    pub strategy: WorkspaceStrategy,
    /// The state it is made from: its first node's state parent (`None`: the campaign's start).
    pub source: Option<String>,
    /// Its nodes, in plan order: each starts from the one before.
    pub nodes: Vec<String>,
}

/// One node of the plan: one execution of one experiment spec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanNode {
    pub id: String,
    /// Its place in the plan's order.
    pub index: usize,
    pub spec: String,
    pub spec_digest: String,
    pub experiment_id: String,
    /// Every node that must finish first (its `after`, and its state parent), in plan order.
    pub after: Vec<String>,
    /// The one node whose accepted state it starts from; `None`: the campaign's start.
    pub from: Option<String>,
    pub workspace: String,
    pub strategy: WorkspaceStrategy,
    /// Every factor the node sets, in the campaign's declaration order.
    pub assignment: Vec<Assignment>,
    pub controls: Vec<ControlValue>,
    pub blocks: Vec<BlockAssignment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replicate: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub treatment: Option<String>,
    /// The repeat or sweep it is a trial of.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design: Option<Design>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub varied: Option<String>,
    pub mutation: MutationMode,
    pub rigor: Rigor,
    pub policy: CampaignPolicy,
    /// What every command of the experiment is given: the campaign, the trial, its replicate, its assignment, the
    /// controls and its blocks, as environment variables ([`super::Binding`]).
    pub bindings: Vec<EnvVar>,
}

/// The campaign, normalized: what runs, in which order, from which state, with what.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignPlan {
    pub schema: String,
    pub campaign_id: String,
    /// The campaign document's digest (format-independent). Not part of the plan's digest: two documents that
    /// compose the same plan have the same plan.
    pub campaign_digest: String,
    /// The plan's digest: everything here but itself and the document's digest.
    pub digest: String,
    pub rigor: Rigor,
    pub policy: CampaignPolicy,
    pub factors: Vec<Factor>,
    pub controls: Vec<ControlValue>,
    pub blocks: Vec<BlockFactor>,
    pub exclude: Vec<String>,
    pub specs: Vec<PlannedSpec>,
    pub groups: Vec<PlannedGroup>,
    pub workspaces: Vec<PlannedWorkspace>,
    pub nodes: Vec<PlanNode>,
    /// The states an isolated workspace is made from (a node's id, or `campaign` for the start): kept when reached.
    pub snapshots: Vec<String>,
    /// What the plan found and did not refuse (a control an experiment declares that the campaign gives no value).
    pub findings: Vec<String>,
}

/// The name `snapshots` uses for the campaign's start.
pub const START: &str = "campaign";

impl CampaignPlan {
    pub fn node(&self, id: &str) -> Option<&PlanNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn spec(&self, path: &str) -> Option<&PlannedSpec> {
        self.specs.iter().find(|s| s.path == path)
    }

    /// The digest of everything but the digests themselves.
    pub fn core_digest(&self) -> String {
        let mut core = self.clone();
        core.digest = String::new();
        core.campaign_digest = String::new();
        digest::of("campaign-plan", &core)
    }

    /// The nodes that could run at once, wave by wave, if every node took the same time: the widest wave with
    /// `max_parallel` as the limit, the widest without, and the number of waves.
    pub fn expected_parallelism(&self) -> (usize, usize, usize) {
        let mut done: BTreeSet<&str> = BTreeSet::new();
        let (mut widest, mut ready_most, mut waves) = (0, 0, 0);
        while done.len() < self.nodes.len() {
            let ready: Vec<&str> = self.nodes.iter().filter(|n| !done.contains(n.id.as_str()) && n.after.iter().all(|a| done.contains(a.as_str()))).map(|n| n.id.as_str()).collect();
            if ready.is_empty() {
                break;
            }
            ready_most = ready_most.max(ready.len());
            let take = ready.len().min(self.policy.max_parallel.max(1) as usize);
            widest = widest.max(take);
            done.extend(ready.into_iter().take(take));
            waves += 1;
        }
        (widest, ready_most, waves)
    }
}

/// A campaign read, its experiments read and planned.
#[derive(Debug, Clone)]
pub struct LoadedCampaign {
    pub spec: CampaignSpec,
    pub plan: CampaignPlan,
    /// The campaign file's folder: its experiment paths are relative to it.
    pub base: PathBuf,
    /// The root the campaign governs: every experiment's target.
    pub root: PathBuf,
    /// Each experiment, by the path the campaign names it with.
    pub experiments: BTreeMap<String, ExperimentSpec>,
}

impl LoadedCampaign {
    /// Reads the campaign at `path` and every experiment it names, and plans it over `root` (the governed root every
    /// experiment's target must be).
    pub fn load(path: &Path, root: &Path) -> Result<LoadedCampaign, CampaignError> {
        let spec = crate::read_campaign(path).map_err(|e| CampaignError::one(format!("{}: {e}", path.display())))?;
        let base = path.parent().map(Path::to_path_buf).unwrap_or_default();
        LoadedCampaign::of(spec, &base, root)
    }

    /// Plans `spec`, its experiment paths relative to `base`, over `root`.
    pub fn of(spec: CampaignSpec, base: &Path, root: &Path) -> Result<LoadedCampaign, CampaignError> {
        let mut errors = Vec::new();
        let mut experiments = BTreeMap::new();
        let mut planned = BTreeMap::new();
        for p in spec_paths(&spec.steps) {
            match read_experiment(base, root, &p) {
                Ok((e, ps)) => {
                    experiments.insert(p.clone(), e);
                    planned.insert(p, ps);
                }
                Err(e) => errors.push(e),
            }
        }
        if !errors.is_empty() {
            return Err(CampaignError(errors));
        }
        let plan = plan(&spec, &experiments, &planned)?;
        Ok(LoadedCampaign { spec, plan, base: base.to_path_buf(), root: root.to_path_buf(), experiments })
    }

    /// The experiment node `id` runs, bound for `workspace`: its target the workspace, its change specs where they
    /// were declared, its commands given the node's bindings. Refused when the experiment or a change spec it names
    /// is no longer what the plan read (its digest moved).
    pub fn bound(&self, id: &str, workspace: &Path) -> Result<ExperimentSpec, CampaignError> {
        let node = self.plan.node(id).ok_or_else(|| CampaignError::one(format!("{id}: no such node in the plan")))?;
        let (now, ps) = read_experiment(&self.base, &self.root, &node.spec).map_err(CampaignError::one)?;
        if ps.digest != node.spec_digest {
            return Err(CampaignError::one(format!("{id}: the experiment {} is {} now, the plan read {}", node.spec, ps.digest, node.spec_digest)));
        }
        super::Binding::of(node).apply(&now, &self.root, workspace).map_err(CampaignError::one)
    }
}

/// Every experiment path the steps name, in order, once each.
fn spec_paths(steps: &[Step]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |p: &Path| {
        let s = path_text(p);
        if !out.contains(&s) {
            out.push(s);
        }
    };
    fn walk(steps: &[Step], add: &mut dyn FnMut(&Path)) {
        for s in steps {
            match s {
                Step::Run(r) => add(&r.spec),
                Step::Sequence(g) | Step::Parallel(g) => walk(&g.steps, add),
                Step::Repeat(r) => add(&r.run.spec),
                Step::Sweep(s) => add(&s.spec),
            }
        }
    }
    walk(steps, &mut add);
    out
}

/// A path as the plan writes it: `/` between its parts, `.` parts dropped.
fn path_text(p: &Path) -> String {
    p.components().filter(|c| !matches!(c, Component::CurDir)).map(|c| c.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
}

/// `p` with `.` and `..` taken out, lexically.
pub(crate) fn lexical(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// The experiment's target, absolute: relative to the governed root when relative.
pub(crate) fn target_of(spec: &ExperimentSpec, root: &Path) -> PathBuf {
    if spec.target.root.is_absolute() { lexical(&spec.target.root) } else { lexical(&root.join(&spec.target.root)) }
}

/// A change spec's file, absolute: relative to the experiment's target when relative.
pub(crate) fn change_of(target: &Path, change: &Path) -> PathBuf {
    if change.is_absolute() { change.to_path_buf() } else { lexical(&target.join(change)) }
}

fn same_dir(a: &Path, b: &Path) -> bool {
    let c = |p: &Path| p.canonicalize().unwrap_or_else(|_| lexical(p));
    c(a) == c(b)
}

/// The experiment at `base/path`, read, its target checked to be the governed root, and its digest taken.
fn read_experiment(base: &Path, root: &Path, path: &str) -> Result<(ExperimentSpec, PlannedSpec), String> {
    let file = base.join(path);
    let spec = crate::read_spec(&file).map_err(|e| format!("experiment {path}: {e}"))?;
    let target = target_of(&spec, root);
    if !same_dir(&target, root) {
        return Err(format!("experiment {path}: its target {} is not the governed root {}", target.display(), root.display()));
    }
    let mut changes = Vec::new();
    for c in &spec.mutation.changes {
        let f = change_of(&target, &c.spec);
        let bytes = std::fs::read(&f).map_err(|e| format!("experiment {path}: change {} ({}): {e}", c.label, f.display()))?;
        changes.push((c.label.clone(), digest::bytes(&bytes)));
    }
    let d = digest::of("experiment", &(&spec, &changes));
    let ps = PlannedSpec {
        path: path.to_owned(),
        experiment_id: spec.id.clone(),
        digest: d,
        mutation: spec.mutation.mode,
        changes,
        independent_variables: spec.scientific_method.independent_variables.iter().map(|v| v.name.clone()).collect(),
        controls: spec.scientific_method.controls.iter().map(|c| c.name.clone()).collect(),
    };
    Ok((spec, ps))
}

/// A reference a step makes before the plan knows every name: a node it made itself, or a name the flat form gave.
#[derive(Debug, Clone, PartialEq)]
enum Ref {
    Node(String),
    Name(String),
}

/// Where a node's candidate is made, as the composition decides it.
#[derive(Debug, Clone, PartialEq)]
enum Placement {
    Shared,
    Isolated,
    /// The workspace node `.0` is in, evolving in place.
    SameAs(String),
    /// The workspace of the node's state parent, evolving in place (the flat form's `continue`).
    OfParent,
}

/// What a composing step is given.
#[derive(Debug, Clone)]
struct Cx {
    from: Option<Ref>,
    after: Vec<Ref>,
    place: Placement,
    /// A step inside a sequence, a parallel or a repeat: `after`, `from` and `workspace` are the composition's.
    nested: bool,
}

/// What a step leaves: the nodes a following step runs after, and the node whose accepted state its lineage ends
/// at (none for a parallel, a repeat or a sweep).
#[derive(Debug, Clone, Default)]
struct Out {
    exits: Vec<String>,
    end: Option<String>,
}

/// A node before every name is known.
#[derive(Debug, Clone)]
struct Draft {
    node: PlanNode,
    from: Option<Ref>,
    after: Vec<Ref>,
    place: Placement,
}

struct Planner<'a> {
    spec: &'a CampaignSpec,
    experiments: &'a BTreeMap<String, ExperimentSpec>,
    planned: &'a BTreeMap<String, PlannedSpec>,
    drafts: Vec<Draft>,
    groups: Vec<PlannedGroup>,
    /// Every name, and what it stands for.
    names: BTreeMap<String, Out>,
    /// Names given twice.
    twice: Vec<String>,
    errors: Vec<String>,
    findings: Vec<String>,
}

fn plan(spec: &CampaignSpec, experiments: &BTreeMap<String, ExperimentSpec>, planned: &BTreeMap<String, PlannedSpec>) -> Result<CampaignPlan, CampaignError> {
    let mut p = Planner { spec, experiments, planned, drafts: Vec::new(), groups: Vec::new(), names: BTreeMap::new(), twice: Vec::new(), errors: Vec::new(), findings: Vec::new() };
    p.check_campaign();
    if spec.steps.is_empty() {
        p.errors.push("the campaign has no steps".into());
    }
    for step in &spec.steps {
        let cx = match top_cx(step) {
            Ok(cx) => cx,
            Err(e) => {
                p.errors.push(e);
                continue;
            }
        };
        p.step(step, cx);
    }
    for n in &p.twice {
        p.errors.push(format!("`{n}` names more than one step or node"));
    }
    let out = p.resolve();
    match out {
        Ok(plan) if p.errors.is_empty() => Ok(plan),
        Ok(_) => Err(CampaignError(p.errors)),
        Err(mut e) => {
            p.errors.append(&mut e);
            Err(CampaignError(p.errors))
        }
    }
}

/// The context a top-level step starts in: its own `after`, `from` and `workspace` (the flat form's), else the
/// campaign's start and the step's own default.
fn top_cx(step: &Step) -> Result<Cx, String> {
    let names = |v: &[String]| v.iter().map(|n| Ref::Name(n.clone())).collect::<Vec<_>>();
    let (after, from, ws, kind) = match step {
        Step::Run(r) => (names(&r.after), r.from.clone(), r.workspace, "run"),
        Step::Sequence(g) => (names(&g.after), g.from.clone(), g.workspace, "sequence"),
        Step::Parallel(g) => (names(&g.after), g.from.clone(), g.workspace, "parallel"),
        Step::Repeat(r) => (names(&r.after), r.from.clone(), None, "repeat"),
        Step::Sweep(s) => (names(&s.after), s.from.clone(), None, "sweep"),
    };
    let place = match (kind, ws) {
        (_, None | Some(WorkspaceStrategy::Shared)) => Placement::Shared,
        ("parallel", Some(WorkspaceStrategy::Isolated)) => Placement::Shared,
        (_, Some(WorkspaceStrategy::Isolated)) => Placement::Isolated,
        ("parallel", Some(WorkspaceStrategy::Continue)) => return Err("a parallel's branches each get their own isolated workspace; `continue` is no parallel's".into()),
        (_, Some(WorkspaceStrategy::Continue)) => {
            if from.is_none() {
                return Err(format!("{}: `continue` evolves its state parent's workspace in place; it needs `from`", step_name(step)));
            }
            Placement::OfParent
        }
    };
    if kind == "parallel" && ws == Some(WorkspaceStrategy::Shared) {
        return Err(format!("{}: a parallel's branches cannot share the governed workspace (one writer per workspace); they run isolated", step_name(step)));
    }
    Ok(Cx { from: from.map(Ref::Name), after, place, nested: false })
}

fn step_name(step: &Step) -> String {
    match step {
        Step::Run(r) => format!("run {}", r.id),
        Step::Sequence(g) => format!("sequence {}", g.id.as_deref().unwrap_or("(unnamed)")),
        Step::Parallel(g) => format!("parallel {}", g.id.as_deref().unwrap_or("(unnamed)")),
        Step::Repeat(r) => format!("repeat {}", r.id),
        Step::Sweep(s) => format!("sweep {}", s.id),
    }
}

/// A name a node, a group or a campaign may have: it is a folder's name too.
fn id_ok(id: &str) -> bool {
    !id.is_empty() && !id.starts_with('.') && id.len() <= 128 && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// A factor's, a control's or a block's name: a letter, then letters, digits and `_`.
fn variable_ok(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The environment variable a name is given as: `EXPERIMENT_<KIND>_<NAME>`, the name upper-cased.
fn env_name(kind: &str, name: &str) -> String {
    format!("EXPERIMENT_{kind}_{}", name.to_ascii_uppercase())
}

impl<'a> Planner<'a> {
    fn check_campaign(&mut self) {
        let s = self.spec;
        if s.schema != CAMPAIGN_SCHEMA {
            self.errors.push(format!("campaign schema is `{}`, expected `{CAMPAIGN_SCHEMA}`", s.schema));
        }
        if !id_ok(&s.id) {
            self.errors.push(format!("campaign id `{}`: letters, digits, `-`, `_` and `.` only, not first", s.id));
        }
        if s.policy.max_parallel == 0 {
            self.errors.push("policy max_parallel 0: at least one node must be able to run".into());
        }
        let mut seen: BTreeMap<String, &str> = BTreeMap::new();
        let mut env: BTreeMap<String, String> = BTreeMap::new();
        let kinds = s.factors.iter().map(|f| (f.name.as_str(), "factor", "FACTOR", f.levels.as_slice())).chain(s.blocks.iter().map(|b| (b.name.as_str(), "block", "BLOCK", b.levels.as_slice()))).chain(s.controls.iter().map(|c| (c.name.as_str(), "control", "CONTROL", std::slice::from_ref(&c.value))));
        for (name, kind, word, levels) in kinds {
            if !variable_ok(name) {
                self.errors.push(format!("{kind} `{name}`: a letter, then letters, digits and `_`"));
            }
            if let Some(other) = seen.insert(name.to_owned(), kind) {
                self.errors.push(format!("`{name}` is declared as a {other} and as a {kind}: a name is one variable"));
            }
            let e = env_name(word, name);
            if let Some(other) = env.insert(e.clone(), name.to_owned()) {
                self.errors.push(format!("{kind} `{name}` and `{other}` are both given as {e}"));
            }
            if levels.is_empty() {
                self.errors.push(format!("{kind} `{name}` has no levels"));
            }
            if let Some(first) = levels.first() {
                if levels.iter().any(|l| l.kind() != first.kind()) {
                    self.errors.push(format!("{kind} `{name}`: its levels are not all one kind ({})", levels.iter().map(Level::kind).collect::<Vec<_>>().join(", ")));
                }
                for (i, l) in levels.iter().enumerate() {
                    if levels[..i].contains(l) {
                        self.errors.push(format!("{kind} `{name}`: level {l} is declared twice"));
                    }
                    if let Level::Number(x) = l
                        && !x.is_finite()
                    {
                        self.errors.push(format!("{kind} `{name}`: level {l} is not a finite number"));
                    }
                }
            }
        }
        for e in &s.exclude {
            let p = Path::new(e);
            if e.is_empty() || p.is_absolute() || p.components().any(|c| matches!(c, Component::ParentDir)) {
                self.errors.push(format!("exclude `{e}`: a path inside the governed root, relative to it"));
            }
        }
        // the controls each experiment declares: given a value by the campaign, or (strict) refused
        for (path, ps) in self.planned {
            for c in &ps.controls {
                if !s.controls.iter().any(|v| &v.name == c) {
                    match s.rigor {
                        Rigor::Strict => self.errors.push(format!("experiment {path} declares the control `{c}` and the campaign gives it no value: strict holds every declared control at a declared value")),
                        _ => self.findings.push(format!("experiment {path} declares the control `{c}`; the campaign gives it no value, so it is not controlled by the campaign")),
                    }
                }
            }
            if ps.mutation == MutationMode::Unbounded && s.rigor == Rigor::Strict {
                self.errors.push(format!("experiment {path} is unbounded: strict allows source changes only through bounded Code Passenger mutation"));
            }
        }
    }

    fn name(&mut self, name: &str, out: Out) {
        if self.names.insert(name.to_owned(), out).is_some() {
            self.twice.push(name.to_owned());
        }
    }

    fn factor(&self, name: &str) -> Option<&'a Factor> {
        let spec: &'a CampaignSpec = self.spec;
        spec.factors.iter().find(|f| f.name == name)
    }

    /// `set` checked: each factor declared, once, at one of its levels, and (scope `experiment`) an independent
    /// variable of the experiment. In the campaign's declaration order.
    fn assignment(&mut self, at: &str, spec: &str, set: &[Assignment]) -> Vec<Assignment> {
        let mut out: Vec<Assignment> = Vec::new();
        for a in set {
            let Some(f) = self.factor(&a.factor) else {
                let known: Vec<&str> = self.spec.factors.iter().map(|f| f.name.as_str()).collect();
                self.errors.push(format!("{at}: `{}` is no declared factor (factors: {})", a.factor, if known.is_empty() { "none".into() } else { known.join(", ") }));
                continue;
            };
            if !f.levels.contains(&a.value) {
                self.errors.push(format!("{at}: factor `{}` has no level {} ({} levels: {})", a.factor, a.value, a.value.kind(), f.levels.iter().map(|l| l.to_string()).collect::<Vec<_>>().join(", ")));
            }
            if f.scope == FactorScope::Experiment && !self.planned.get(spec).is_some_and(|p| p.independent_variables.contains(&a.factor)) {
                self.errors.push(format!("{at}: factor `{}` is no independent variable of experiment {spec} (declare it there, or give the factor scope `campaign`)", a.factor));
            }
            if out.iter().any(|o| o.factor == a.factor) {
                self.errors.push(format!("{at}: factor `{}` is set twice", a.factor));
                continue;
            }
            out.push(a.clone());
        }
        let order = |a: &Assignment| self.spec.factors.iter().position(|f| f.name == a.factor).unwrap_or(usize::MAX);
        out.sort_by_key(order);
        out
    }

    fn block_assignment(&mut self, at: &str, set: &[BlockAssignment]) -> Vec<BlockAssignment> {
        let mut out: Vec<BlockAssignment> = Vec::new();
        for b in set {
            match self.spec.blocks.iter().find(|x| x.name == b.name) {
                None => self.errors.push(format!("{at}: `{}` is no declared block", b.name)),
                Some(x) if !x.levels.contains(&b.level) => self.errors.push(format!("{at}: block `{}` has no level {}", b.name, b.level)),
                _ => {}
            }
            if out.iter().any(|o| o.name == b.name) {
                self.errors.push(format!("{at}: block `{}` is set twice", b.name));
                continue;
            }
            out.push(b.clone());
        }
        let order = |b: &BlockAssignment| self.spec.blocks.iter().position(|x| x.name == b.name).unwrap_or(usize::MAX);
        out.sort_by_key(order);
        out
    }

    fn step(&mut self, step: &Step, cx: Cx) -> Out {
        match step {
            Step::Run(r) => self.run(r, cx, Trial::default()),
            Step::Sequence(g) => self.sequence(g, cx),
            Step::Parallel(g) => self.parallel(g, cx),
            Step::Repeat(r) => self.repeat(r, cx),
            Step::Sweep(s) => self.sweep(s, cx),
        }
    }

    /// A nested step that names its own `after`, `from` or `workspace`: the composition decides those.
    fn nested_fields(&mut self, cx: &Cx, what: &str, after: &[String], from: &Option<String>, ws: Option<WorkspaceStrategy>) {
        if cx.nested && (!after.is_empty() || from.is_some() || ws.is_some()) {
            self.errors.push(format!("{what}: inside a sequence, a parallel or a repeat, `after`, `from` and `workspace` are the composition's (contradictory composition)"));
        }
    }

    fn run(&mut self, r: &RunStep, cx: Cx, trial: Trial) -> Out {
        let id = trial.id.clone().unwrap_or_else(|| r.id.clone());
        if trial.id.is_none() {
            self.nested_fields(&cx, &format!("run {}", r.id), &r.after, &r.from, r.workspace);
        }
        if !id_ok(&id) {
            self.errors.push(format!("run `{id}`: an id is letters, digits, `-`, `_` and `.`, not `.` first"));
        }
        let spec = path_text(&r.spec);
        let at = format!("run {id}");
        let assignment = match &trial.assignment {
            Some(a) => a.clone(),
            None => self.assignment(&at, &spec, &r.set),
        };
        let blocks = match &trial.blocks {
            Some(b) => b.clone(),
            None => self.block_assignment(&at, &r.blocks),
        };
        let ps = self.planned.get(&spec).cloned();
        let node = PlanNode {
            id: id.clone(),
            index: 0,
            spec: spec.clone(),
            spec_digest: ps.as_ref().map(|p| p.digest.clone()).unwrap_or_default(),
            experiment_id: ps.as_ref().map(|p| p.experiment_id.clone()).unwrap_or_default(),
            after: Vec::new(),
            from: None,
            workspace: String::new(),
            strategy: WorkspaceStrategy::Shared,
            assignment,
            controls: self.spec.controls.clone(),
            blocks,
            replicate: trial.replicate,
            treatment: trial.treatment.clone(),
            group: trial.group.clone(),
            design: trial.design,
            varied: trial.varied.clone(),
            mutation: ps.as_ref().map(|p| p.mutation).unwrap_or_default(),
            rigor: self.spec.rigor,
            policy: self.spec.policy,
            bindings: Vec::new(),
        };
        self.drafts.push(Draft { node, from: cx.from.clone(), after: cx.after.clone(), place: cx.place.clone() });
        let out = Out { exits: vec![id.clone()], end: Some(id.clone()) };
        self.name(&id, out.clone());
        out
    }

    fn sequence(&mut self, g: &GroupStep, cx: Cx) -> Out {
        let what = format!("sequence {}", g.id.as_deref().unwrap_or("(unnamed)"));
        self.nested_fields(&cx, &what, &g.after, &g.from, g.workspace);
        if g.steps.is_empty() {
            self.errors.push(format!("{what} has no steps"));
        }
        let (mut from, mut after, mut place) = (cx.from.clone(), cx.after.clone(), cx.place.clone());
        let mut end = None;
        for s in &g.steps {
            let out = self.step(s, Cx { from: from.clone(), after: after.clone(), place: place.clone(), nested: true });
            after = out.exits.iter().cloned().map(Ref::Node).collect();
            if let Some(e) = out.end {
                from = Some(Ref::Node(e.clone()));
                if place != Placement::Shared {
                    place = Placement::SameAs(e.clone());
                }
                end = Some(e);
            }
        }
        let out = Out { exits: after.into_iter().filter_map(|r| if let Ref::Node(n) = r { Some(n) } else { None }).collect(), end };
        if let Some(id) = &g.id {
            self.name(id, out.clone());
        }
        out
    }

    fn parallel(&mut self, g: &GroupStep, cx: Cx) -> Out {
        let what = format!("parallel {}", g.id.as_deref().unwrap_or("(unnamed)"));
        self.nested_fields(&cx, &what, &g.after, &g.from, g.workspace);
        if g.steps.is_empty() {
            self.errors.push(format!("{what} has no branches"));
        }
        let mut exits = Vec::new();
        for s in &g.steps {
            let out = self.step(s, Cx { from: cx.from.clone(), after: cx.after.clone(), place: Placement::Isolated, nested: true });
            exits.extend(out.exits);
        }
        let out = Out { exits, end: None };
        if let Some(id) = &g.id {
            self.name(id, out.clone());
        }
        out
    }

    fn repeat(&mut self, r: &RepeatStep, cx: Cx) -> Out {
        let what = format!("repeat {}", r.id);
        if cx.nested && (!r.after.is_empty() || r.from.is_some()) {
            self.errors.push(format!("{what}: inside a sequence or a parallel, `after` and `from` are the composition's (contradictory composition)"));
        }
        if !r.run.after.is_empty() || r.run.from.is_some() || r.run.workspace.is_some() {
            self.errors.push(format!("{what}: its run's `after`, `from` and `workspace` are the repeat's (each trial is isolated, from the repeat's source)"));
        }
        if r.times == 0 {
            self.errors.push(format!("{what}: times 0; a repeat runs at least once"));
        }
        if !id_ok(&r.id) {
            self.errors.push(format!("{what}: an id is letters, digits, `-`, `_` and `.`"));
        }
        let spec = path_text(&r.run.spec);
        let at = format!("{what} run {}", r.run.id);
        let assignment = self.assignment(&at, &spec, &r.run.set);
        let blocks = self.block_assignment(&at, &r.run.blocks);
        let width = width(r.times as usize);
        let mut exits = Vec::new();
        for k in 1..=r.times {
            let trial = Trial { id: Some(format!("{}.r{k:0width$}", r.id)), assignment: Some(assignment.clone()), blocks: Some(blocks.clone()), replicate: Some(k), treatment: Some(r.run.id.clone()), group: Some(r.id.clone()), design: None, varied: None };
            let out = self.run(&r.run, Cx { from: cx.from.clone(), after: cx.after.clone(), place: Placement::Isolated, nested: true }, trial);
            exits.extend(out.exits);
        }
        self.groups.push(PlannedGroup { id: r.id.clone(), kind: "repeat".into(), spec, design: None, factors: assignment.iter().map(|a| a.factor.clone()).collect(), blocks: blocks.iter().map(|b| b.name.clone()).collect(), repeats: r.times, treatments: vec![PlannedTreatment { name: r.run.id.clone(), assignment, varied: None }], nodes: exits.clone() });
        let out = Out { exits, end: None };
        self.name(&r.id, out.clone());
        out
    }

    fn sweep(&mut self, s: &SweepStep, cx: Cx) -> Out {
        let what = format!("sweep {}", s.id);
        if cx.nested && (!s.after.is_empty() || s.from.is_some()) {
            self.errors.push(format!("{what}: inside a sequence or a parallel, `after` and `from` are the composition's (contradictory composition)"));
        }
        if !id_ok(&s.id) {
            self.errors.push(format!("{what}: an id is letters, digits, `-`, `_` and `.`"));
        }
        if s.repeats == 0 {
            self.errors.push(format!("{what}: repeats 0; each treatment runs at least once"));
        }
        let spec = path_text(&s.spec);
        let fixed = self.assignment(&format!("{what} set"), &spec, &s.set);
        let treatments = self.treatments(s, &spec, &what, &fixed);
        // the blocks crossed with the design: every combination of their levels
        let mut block_sets: Vec<Vec<BlockAssignment>> = vec![Vec::new()];
        for (i, name) in s.blocks.iter().enumerate() {
            if s.blocks[..i].contains(name) {
                self.errors.push(format!("{what}: block `{name}` is named twice"));
                continue;
            }
            let Some(b) = self.spec.blocks.iter().find(|b| &b.name == name) else {
                self.errors.push(format!("{what}: `{name}` is no declared block{}", if self.factor(name).is_some() { " (it is a factor: a block is never a treatment, a factor never a block)" } else { "" }));
                continue;
            };
            block_sets = block_sets.into_iter().flat_map(|set| b.levels.iter().map(move |l| {
                let mut s = set.clone();
                s.push(BlockAssignment { name: b.name.clone(), level: l.clone() });
                s
            })).collect();
        }
        let total = treatments.len().saturating_mul(block_sets.len()).saturating_mul(s.repeats as usize);
        if total > MAX_TRIALS {
            self.errors.push(format!("{what}: {} treatments × {} block combinations × {} repeats = {total} trials, more than {MAX_TRIALS}", treatments.len(), block_sets.len(), s.repeats));
            return Out::default();
        }
        let (tw, bw, rw) = (width(treatments.len()), width(block_sets.len()), width(s.repeats as usize));
        let mut exits = Vec::new();
        for (bi, blocks) in block_sets.iter().enumerate() {
            for (ti, t) in treatments.iter().enumerate() {
                for k in 1..=s.repeats {
                    let b = if s.blocks.is_empty() { String::new() } else { format!(".b{:0bw$}", bi + 1) };
                    let id = format!("{}.t{:0tw$}{b}.r{k:0rw$}", s.id, ti + 1);
                    let run = RunStep { id: id.clone(), spec: s.spec.clone(), set: Vec::new(), blocks: Vec::new(), after: Vec::new(), from: None, workspace: None };
                    let trial = Trial { id: Some(id), assignment: Some(t.assignment.clone()), blocks: Some(blocks.clone()), replicate: Some(k), treatment: Some(t.name.clone()), group: Some(s.id.clone()), design: Some(s.design), varied: t.varied.clone() };
                    let out = self.run(&run, Cx { from: cx.from.clone(), after: cx.after.clone(), place: Placement::Isolated, nested: true }, trial);
                    exits.extend(out.exits);
                }
            }
        }
        let factors = match s.design {
            Design::Explicit => {
                let mut f: Vec<String> = Vec::new();
                for t in &treatments {
                    for a in &t.assignment {
                        if !f.contains(&a.factor) {
                            f.push(a.factor.clone());
                        }
                    }
                }
                f
            }
            _ => s.factors.clone(),
        };
        self.groups.push(PlannedGroup { id: s.id.clone(), kind: "sweep".into(), spec, design: Some(s.design), factors, blocks: s.blocks.clone(), repeats: s.repeats, treatments, nodes: exits.clone() });
        let out = Out { exits, end: None };
        self.name(&s.id, out.clone());
        out
    }

    /// A sweep's treatments, by its design, each with its full assignment (the sweep's fixed levels included).
    fn treatments(&mut self, s: &SweepStep, spec: &str, what: &str, fixed: &[Assignment]) -> Vec<PlannedTreatment> {
        let with_fixed = |mut a: Vec<Assignment>, order: &[Factor]| {
            a.extend(fixed.iter().cloned());
            a.sort_by_key(|x| order.iter().position(|f| f.name == x.factor).unwrap_or(usize::MAX));
            a
        };
        let moved: Vec<&'a Factor> = {
            let mut v = Vec::new();
            for (i, name) in s.factors.iter().enumerate() {
                if s.factors[..i].contains(name) {
                    self.errors.push(format!("{what}: factor `{name}` is named twice"));
                    continue;
                }
                match self.factor(name) {
                    None => self.errors.push(format!("{what}: `{name}` is no declared factor{}", if self.spec.blocks.iter().any(|b| &b.name == name) { " (it is a block: a block is never a treatment)" } else { "" })),
                    Some(f) => {
                        if fixed.iter().any(|a| &a.factor == name) {
                            self.errors.push(format!("{what}: factor `{name}` is both moved by the design and held by `set`"));
                        }
                        if f.scope == FactorScope::Experiment && !self.planned.get(spec).is_some_and(|p| p.independent_variables.contains(name)) {
                            self.errors.push(format!("{what}: factor `{name}` is no independent variable of experiment {spec} (declare it there, or give the factor scope `campaign`)"));
                        }
                        v.push(f);
                    }
                }
            }
            v
        };
        let order = self.spec.factors.clone();
        match s.design {
            Design::Factorial => {
                if !s.treatments.is_empty() || !s.reference.is_empty() {
                    self.errors.push(format!("{what}: a factorial design takes `factor`s; `treatment` is an explicit design's and `reference` one-factor-at-a-time's"));
                }
                if moved.is_empty() {
                    self.errors.push(format!("{what}: a factorial design names at least one factor"));
                    return Vec::new();
                }
                let mut sets: Vec<Vec<Assignment>> = vec![Vec::new()];
                for f in &moved {
                    sets = sets.into_iter().flat_map(|set| f.levels.iter().map(move |l| {
                        let mut s = set.clone();
                        s.push(Assignment { factor: f.name.clone(), value: l.clone() });
                        s
                    })).collect();
                    if sets.len() > MAX_TRIALS {
                        self.errors.push(format!("{what}: more than {MAX_TRIALS} treatments"));
                        return Vec::new();
                    }
                }
                sets.into_iter().map(|a| PlannedTreatment { name: treatment_name(&a), assignment: with_fixed(a, &order), varied: None }).collect()
            }
            Design::OneFactorAtATime => {
                if !s.treatments.is_empty() {
                    self.errors.push(format!("{what}: one-factor-at-a-time takes `factor`s and their `reference`; `treatment` is an explicit design's"));
                }
                if moved.is_empty() {
                    self.errors.push(format!("{what}: one-factor-at-a-time names at least one factor"));
                    return Vec::new();
                }
                let reference = self.assignment(&format!("{what} reference"), spec, &s.reference);
                for f in &moved {
                    if !reference.iter().any(|a| a.factor == f.name) {
                        self.errors.push(format!("{what}: factor `{}` has no reference level (one-factor-at-a-time moves each factor from its reference)", f.name));
                    }
                }
                for a in &reference {
                    if !moved.iter().any(|f| f.name == a.factor) {
                        self.errors.push(format!("{what}: a reference for `{}`, which the design does not move", a.factor));
                    }
                }
                let mut out = vec![PlannedTreatment { name: "reference".into(), assignment: with_fixed(reference.clone(), &order), varied: None }];
                for f in &moved {
                    let Some(r) = reference.iter().find(|a| a.factor == f.name) else { continue };
                    for l in f.levels.iter().filter(|l| **l != r.value) {
                        let a: Vec<Assignment> = reference.iter().map(|x| if x.factor == f.name { Assignment { factor: x.factor.clone(), value: l.clone() } } else { x.clone() }).collect();
                        out.push(PlannedTreatment { name: format!("{}={l}", f.name), assignment: with_fixed(a, &order), varied: Some(f.name.clone()) });
                    }
                }
                out
            }
            Design::Explicit => {
                if !s.factors.is_empty() || !s.reference.is_empty() {
                    self.errors.push(format!("{what}: an explicit design lists its `treatment`s; `factor` and `reference` are the other designs'"));
                }
                if s.treatments.is_empty() {
                    self.errors.push(format!("{what}: an explicit design lists at least one treatment"));
                }
                let mut out: Vec<PlannedTreatment> = Vec::new();
                for t in &s.treatments {
                    if t.name.is_empty() {
                        self.errors.push(format!("{what}: a treatment has no name"));
                    }
                    let a = self.assignment(&format!("{what} treatment {}", t.name), spec, &t.set);
                    for x in &a {
                        if fixed.iter().any(|f| f.factor == x.factor) {
                            self.errors.push(format!("{what} treatment {}: factor `{}` is also held by the sweep's `set`", t.name, x.factor));
                        }
                    }
                    let a = with_fixed(a, &order);
                    if let Some(other) = out.iter().find(|o| o.name == t.name) {
                        self.errors.push(format!("{what}: treatment `{}` is named twice", other.name));
                    } else if let Some(other) = out.iter().find(|o| o.assignment == a) {
                        self.errors.push(format!("{what}: treatments `{}` and `{}` assign the same levels (that is one treatment, repeated)", other.name, t.name));
                    }
                    out.push(PlannedTreatment { name: t.name.clone(), assignment: a, varied: None });
                }
                out
            }
        }
    }

    /// Every name resolved, the order made, each node's workspace decided, the lineages checked.
    fn resolve(&mut self) -> Result<CampaignPlan, Vec<String>> {
        let mut errors = Vec::new();
        let ids: BTreeSet<String> = self.drafts.iter().map(|d| d.node.id.clone()).collect();
        let index: BTreeMap<String, usize> = self.drafts.iter().enumerate().map(|(i, d)| (d.node.id.clone(), i)).collect();
        // each draft's `after` and `from`, as node ids
        for d in &mut self.drafts {
            let id = d.node.id.clone();
            let mut deps: Vec<String> = Vec::new();
            for r in &d.after {
                match r {
                    Ref::Node(n) => deps.push(n.clone()),
                    Ref::Name(n) => match self.names.get(n) {
                        Some(o) => deps.extend(o.exits.iter().cloned()),
                        None => errors.push(format!("{id}: after `{n}`, which no step or node is named")),
                    },
                }
            }
            d.node.from = match &d.from {
                None => None,
                Some(Ref::Node(n)) => Some(n.clone()),
                Some(Ref::Name(n)) => match self.names.get(n) {
                    None => {
                        errors.push(format!("{id}: from `{n}`, which no step or node is named"));
                        None
                    }
                    Some(Out { end: Some(e), .. }) => Some(e.clone()),
                    Some(_) => {
                        errors.push(format!("{id}: from `{n}`, a parallel, repeat or sweep: its trials end in many states, `from` names one (a node, or a sequence)"));
                        None
                    }
                },
            };
            if let Some(f) = &d.node.from {
                if f == &id {
                    errors.push(format!("{id}: from itself"));
                }
                deps.push(f.clone());
            }
            if deps.contains(&id) {
                errors.push(format!("{id}: after itself"));
                deps.retain(|x| x != &id);
            }
            deps.sort_by_key(|x| index.get(x).copied().unwrap_or(usize::MAX));
            deps.dedup();
            d.node.after = deps;
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        // the order: dependencies first, ties broken by declaration (Kahn's, smallest declaration index first)
        let mut placed: Vec<usize> = Vec::new();
        let mut done: BTreeSet<&str> = BTreeSet::new();
        let mut left: BTreeSet<usize> = (0..self.drafts.len()).collect();
        while !left.is_empty() {
            let next = left.iter().copied().find(|&i| self.drafts[i].node.after.iter().all(|a| done.contains(a.as_str())));
            match next {
                Some(i) => {
                    left.remove(&i);
                    done.insert(self.drafts[i].node.id.as_str());
                    placed.push(i);
                }
                None => {
                    let cycle: Vec<&str> = left.iter().map(|&i| self.drafts[i].node.id.as_str()).collect();
                    return Err(vec![format!("a cycle: {} each wait for another of them", cycle.join(", "))]);
                }
            }
        }
        let mut nodes: Vec<PlanNode> = Vec::new();
        let mut ws_of: BTreeMap<String, String> = BTreeMap::new();
        for (pos, &i) in placed.iter().enumerate() {
            let d = &self.drafts[i];
            let mut n = d.node.clone();
            n.index = pos;
            let parent_ws = n.from.as_ref().and_then(|f| ws_of.get(f).cloned());
            let (ws, strategy) = match &d.place {
                Placement::Shared => ("shared".to_owned(), WorkspaceStrategy::Shared),
                Placement::Isolated => (format!("isolated:{}", n.id), WorkspaceStrategy::Isolated),
                Placement::SameAs(other) => {
                    let w = ws_of.get(other).cloned().unwrap_or_default();
                    let s = if w == "shared" { WorkspaceStrategy::Shared } else { WorkspaceStrategy::Continue };
                    (w, s)
                }
                Placement::OfParent => match &parent_ws {
                    Some(w) if w != "shared" => (w.clone(), WorkspaceStrategy::Continue),
                    Some(_) => {
                        errors.push(format!("{}: `continue` of {}, which is in the shared workspace: say `shared`", n.id, n.from.clone().unwrap_or_default()));
                        ("shared".into(), WorkspaceStrategy::Shared)
                    }
                    None => {
                        errors.push(format!("{}: `continue` needs a state parent in an isolated workspace", n.id));
                        (format!("isolated:{}", n.id), WorkspaceStrategy::Isolated)
                    }
                },
            };
            // a node evolving a workspace in place starts from that workspace's state: its parent is in it
            if strategy != WorkspaceStrategy::Isolated {
                match (&n.from, &parent_ws) {
                    (None, _) if ws != "shared" => errors.push(format!("{}: evolves {ws} in place but starts from the campaign's start", n.id)),
                    (Some(f), Some(pw)) if pw != &ws => errors.push(format!("{}: starts from {f}'s state in {pw}, but runs in {ws}: a workspace's next state is its own (no implicit merge); make it `isolated`", n.id)),
                    _ => {}
                }
            }
            if n.mutation == MutationMode::Unbounded && strategy != WorkspaceStrategy::Isolated && ws == "shared" {
                errors.push(format!("{}: experiment {} is unbounded; its changes in the shared workspace could not be put back through Code Passenger: run it isolated", n.id, n.spec));
            }
            ws_of.insert(n.id.clone(), ws.clone());
            n.workspace = ws;
            n.strategy = strategy;
            n.bindings = super::Binding::of_parts(&self.spec.id, &n).vars();
            nodes.push(n);
        }
        // one writer per workspace: in a workspace each state has at most one next node, and only its first node
        // starts from outside it
        let mut workspaces: Vec<PlannedWorkspace> = Vec::new();
        for n in &nodes {
            match workspaces.iter_mut().find(|w| w.id == n.workspace) {
                Some(w) => w.nodes.push(n.id.clone()),
                None => workspaces.push(PlannedWorkspace { id: n.workspace.clone(), strategy: if n.workspace == "shared" { WorkspaceStrategy::Shared } else { WorkspaceStrategy::Isolated }, source: n.from.clone(), nodes: vec![n.id.clone()] }),
            }
        }
        for w in &workspaces {
            let mut next: BTreeMap<Option<&str>, &str> = BTreeMap::new();
            for id in &w.nodes {
                let n = nodes.iter().find(|x| &x.id == id).expect("a node of the plan");
                if n.strategy == WorkspaceStrategy::Isolated {
                    continue;
                }
                let parent = n.from.as_deref();
                if let Some(other) = next.insert(parent, id) {
                    errors.push(format!("{other} and {id} both evolve {} from {}: one writer per workspace, one next state (use a sequence, or make one isolated)", w.id, parent.unwrap_or("the campaign's start")));
                }
            }
        }
        // the states isolated workspaces are made from
        let mut snapshots: Vec<String> = Vec::new();
        for n in nodes.iter().filter(|n| n.strategy == WorkspaceStrategy::Isolated) {
            let s = n.from.clone().unwrap_or_else(|| START.to_owned());
            if !snapshots.contains(&s) {
                snapshots.push(s);
            }
        }
        // a binding the experiment's own commands already name
        for n in &nodes {
            if let Some(e) = self.experiments.get(&n.spec)
                && let Err(why) = super::bind::conflicts(e, &n.bindings)
            {
                errors.push(format!("{}: {why}", n.id));
            }
        }
        if ids.contains(START) {
            errors.push(format!("`{START}` names the campaign's start; no node may take it"));
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let mut groups = self.groups.clone();
        for g in &mut groups {
            g.nodes.sort_by_key(|id| nodes.iter().position(|n| &n.id == id));
        }
        let mut specs: Vec<PlannedSpec> = self.planned.values().cloned().collect();
        specs.sort_by(|a, b| a.path.cmp(&b.path));
        let s = self.spec;
        let exclude = s.exclude.iter().map(|e| path_text(Path::new(e))).collect();
        let mut plan = CampaignPlan {
            schema: CAMPAIGN_PLAN_SCHEMA.into(),
            campaign_id: s.id.clone(),
            campaign_digest: digest::of("campaign", s),
            digest: String::new(),
            rigor: s.rigor,
            policy: s.policy,
            factors: s.factors.clone(),
            controls: s.controls.clone(),
            blocks: s.blocks.clone(),
            exclude,
            specs,
            groups,
            workspaces,
            nodes,
            snapshots,
            findings: self.findings.clone(),
        };
        plan.digest = plan.core_digest();
        Ok(plan)
    }
}

/// How a trial's node differs from its run's own: its id, its assignment and blocks, its place in its group.
#[derive(Debug, Clone, Default)]
struct Trial {
    id: Option<String>,
    assignment: Option<Vec<Assignment>>,
    blocks: Option<Vec<BlockAssignment>>,
    replicate: Option<u32>,
    treatment: Option<String>,
    group: Option<String>,
    design: Option<Design>,
    varied: Option<String>,
}

/// The digits an index of `n` is written with (two at least).
fn width(n: usize) -> usize {
    n.to_string().len().max(2)
}

/// A treatment named by its assignment: `matcher=simd,population=1K`.
fn treatment_name(a: &[Assignment]) -> String {
    a.iter().map(|x| format!("{}={}", x.factor, x.value)).collect::<Vec<_>>().join(",")
}
