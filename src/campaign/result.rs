//! The campaign's result: everything needed to reconstruct the campaign without its logs (every node's plan and
//! execution, every state digest, every result's path and digest, the order things actually ran in), and the group
//! summaries of its repeats and sweeps: counts and the plain statistics of each numeric measurement, nothing more.
//! No significance is claimed and no conclusion is drawn that the experiments and the design did not declare.

use super::plan::{CampaignPlan, PlanNode, PlannedGroup};
use super::run::{CampaignRunner, FileRef, NodeRecord, NodeStatus, OrderEvent, StateAction, StateIdentity};
use super::spec::*;
use crate::PromotionDecision;
use scientific_method::Scalar;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// A tool the campaign ran with: its version and, where the host could read it, its binary's digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolVersion {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

/// What the host establishes for the result: the tools, where the plan is kept, and what it actually controlled.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostFacts {
    pub tools: Vec<ToolVersion>,
    pub plan_file: Option<FileRef>,
    pub enforced: Vec<String>,
    pub excluded: Vec<String>,
}

/// One node: as planned, and as it ran.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeResult {
    pub plan: PlanNode,
    pub run: NodeRecord,
}

/// A workspace as the campaign left it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceResult {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted: Option<StateIdentity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<String>,
    pub discarded: bool,
    /// Discarded: the state it was left in, as read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<StateIdentity>,
}

/// The plain statistics of one measurement over a group's completed trials.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeasurementSummary {
    pub condition: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Numeric values seen.
    pub n: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mean: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub median: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// The sample standard deviation (n - 1), for two values or more.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stddev: Option<f64>,
    /// Boolean and text values, counted by value.
    #[serde(default)]
    pub values: BTreeMap<String, usize>,
}

/// One treatment of a repeat or a sweep, in one block combination.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupSummary {
    pub group: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design: Option<Design>,
    pub treatment: String,
    pub assignment: Vec<Assignment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub varied: Option<String>,
    pub blocks: Vec<BlockAssignment>,
    /// The state every trial of this summary started from (none: trials that never read a start).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_digest: Option<String>,
    pub requested: usize,
    pub completed: usize,
    /// Each trial: its node, its replicate, its result.
    pub trials: Vec<TrialRef>,
    pub outcomes: BTreeMap<String, usize>,
    pub promotions: BTreeMap<String, usize>,
    pub states: BTreeMap<String, usize>,
    pub measurements: Vec<MeasurementSummary>,
}

/// A trial of a group summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrialRef {
    pub node: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replicate: Option<u32>,
    pub status: NodeStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<FileRef>,
}

/// A sweep's measurements by one factor's level, over the treatments that have it, within one block combination and
/// one starting state (never pooled across either).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarginalSummary {
    pub group: String,
    pub factor: String,
    pub level: Level,
    pub blocks: Vec<BlockAssignment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_digest: Option<String>,
    pub trials: usize,
    pub completed: usize,
    pub measurements: Vec<MeasurementSummary>,
}

/// How the campaign ended, in Custodian's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignStatus {
    /// Every node completed and every candidate was accepted, with no finding.
    Done,
    /// Every node that ran completed; some were not promoted, or have findings (the campaign may have stopped on
    /// them by its policy).
    Findings,
    /// A node was refused or failed, or one could not run.
    Refused,
}

impl CampaignStatus {
    pub fn word(self) -> &'static str {
        match self {
            CampaignStatus::Done => "done",
            CampaignStatus::Findings => "findings",
            CampaignStatus::Refused => "refused",
        }
    }
}

/// How many nodes ended each way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Counts {
    pub nodes: usize,
    pub completed: usize,
    pub refused: usize,
    pub failed: usize,
    pub skipped: usize,
    pub promote: usize,
    pub do_not_promote: usize,
    pub undecided: usize,
    pub accepted: usize,
    pub restored: usize,
    pub discarded: usize,
}

/// The campaign, as it ran.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignResult {
    pub schema: String,
    pub campaign_id: String,
    pub run_id: String,
    pub campaign_digest: String,
    pub plan_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_file: Option<FileRef>,
    pub experiments_version: String,
    pub tools: Vec<ToolVersion>,
    pub rigor: Rigor,
    pub policy: CampaignPolicy,
    pub factors: Vec<Factor>,
    pub controls: Vec<ControlValue>,
    pub blocks: Vec<BlockFactor>,
    /// What the scientific state's identity leaves out (the host's runtime paths and the campaign's own).
    pub excluded: Vec<String>,
    pub groups: Vec<PlannedGroup>,
    /// The shared workspace's state when the campaign began.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<StateIdentity>,
    /// Each workspace's accepted state at the end.
    pub workspaces: Vec<WorkspaceResult>,
    pub nodes: Vec<NodeResult>,
    pub order: Vec<OrderEvent>,
    pub summaries: Vec<GroupSummary>,
    pub marginals: Vec<MarginalSummary>,
    pub counts: Counts,
    /// What the campaign established for every node, as the host enforced it; nothing more is claimed.
    pub enforced: Vec<String>,
    /// The plan's findings, then every node's.
    pub findings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped: Option<String>,
    /// The stop: its node, its reason, and exactly the nodes that were running then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop: Option<super::run::StopRecord>,
    /// Earlier attempts of nodes that ran again on a resume, kept with their results.
    #[serde(default)]
    pub attempts: Vec<NodeRecord>,
    pub status: CampaignStatus,
}

fn word<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_default()
}

impl CampaignRunner {
    /// The result of a run that has ended (every node recorded).
    pub fn result(&self, facts: HostFacts) -> CampaignResult {
        let plan = self.plan();
        let p = self.progress();
        let nodes: Vec<NodeResult> = plan.nodes.iter().map(|n| NodeResult { plan: n.clone(), run: p.nodes.get(&n.id).cloned().unwrap_or_else(|| NodeRecord::new(&n.id, NodeStatus::Skipped)) }).collect();
        let mut counts = Counts { nodes: nodes.len(), ..Counts::default() };
        let mut findings: Vec<String> = plan.findings.clone();
        for n in &nodes {
            let r = &n.run;
            match r.status {
                NodeStatus::Completed => counts.completed += 1,
                NodeStatus::Refused => counts.refused += 1,
                NodeStatus::Failed => counts.failed += 1,
                NodeStatus::Skipped => counts.skipped += 1,
            }
            match r.promotion {
                Some(PromotionDecision::Promote) => counts.promote += 1,
                Some(PromotionDecision::DoNotPromote) => counts.do_not_promote += 1,
                Some(PromotionDecision::Undecided) => counts.undecided += 1,
                None => {}
            }
            match r.state {
                Some(StateAction::Accepted) => counts.accepted += 1,
                Some(StateAction::Restored) => counts.restored += 1,
                Some(StateAction::Discarded) => counts.discarded += 1,
                None => {}
            }
            findings.extend(r.findings.iter().map(|f| format!("{}: {f}", r.id)));
        }
        let status = if counts.refused + counts.failed > 0 || (counts.skipped > 0 && p.stopped.as_deref().is_none_or(|s| !s.contains("on_findings stop"))) {
            CampaignStatus::Refused
        } else if findings.is_empty() && counts.promote == counts.nodes && counts.accepted == counts.nodes {
            CampaignStatus::Done
        } else {
            CampaignStatus::Findings
        };
        let workspaces = plan.workspaces.iter().map(|w| {
            let s = p.workspaces.get(&w.id).cloned().unwrap_or_default();
            WorkspaceResult { id: w.id.clone(), path: s.path, accepted: if s.discarded { None } else { s.accepted }, left: s.left, last: s.last, discarded: s.discarded }
        }).collect();
        let (summaries, marginals) = summarize(plan, &nodes);
        CampaignResult {
            schema: CAMPAIGN_RESULT_SCHEMA.into(),
            campaign_id: plan.campaign_id.clone(),
            run_id: p.run_id.clone(),
            campaign_digest: plan.campaign_digest.clone(),
            plan_digest: plan.digest.clone(),
            plan_file: facts.plan_file,
            experiments_version: crate::VERSION.into(),
            tools: facts.tools,
            rigor: plan.rigor,
            policy: plan.policy,
            factors: plan.factors.clone(),
            controls: plan.controls.clone(),
            blocks: plan.blocks.clone(),
            excluded: facts.excluded,
            groups: plan.groups.clone(),
            start: p.start.clone(),
            workspaces,
            nodes,
            order: p.order.clone(),
            summaries,
            marginals,
            counts,
            enforced: facts.enforced,
            findings,
            stopped: p.stopped.clone(),
            stop: p.stop.clone(),
            attempts: p.attempts.clone(),
            status,
        }
    }
}

/// Every value a trial measured, numeric and not, by (condition, measurement).
type Values = BTreeMap<(String, String), (Option<String>, Vec<f64>, BTreeMap<String, usize>)>;

fn collect(into: &mut Values, r: &NodeRecord) {
    let Some(obs) = &r.observations else { return };
    for o in obs.iter() {
        let e = into.entry((o.condition.clone(), o.measurement.name.clone())).or_insert_with(|| (o.measurement.unit.clone(), Vec::new(), BTreeMap::new()));
        match &o.measurement.value {
            Scalar::Number(x) if x.is_finite() => e.1.push(*x),
            Scalar::Number(x) => *e.2.entry(x.to_string()).or_default() += 1,
            Scalar::Boolean(b) => *e.2.entry(b.to_string()).or_default() += 1,
            Scalar::Text(t) => *e.2.entry(t.clone()).or_default() += 1,
        }
    }
}

fn stats(values: Values) -> Vec<MeasurementSummary> {
    values
        .into_iter()
        .map(|((condition, name), (unit, mut xs, counted))| {
            xs.sort_by(|a, b| a.total_cmp(b));
            let n = xs.len();
            let mean = (n > 0).then(|| xs.iter().sum::<f64>() / n as f64);
            let median = (n > 0).then(|| if n % 2 == 1 { xs[n / 2] } else { (xs[n / 2 - 1] + xs[n / 2]) / 2.0 });
            let stddev = mean.filter(|_| n > 1).map(|m| (xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt());
            MeasurementSummary { condition, name, unit, n, mean, median, min: xs.first().copied(), max: xs.last().copied(), stddev, values: counted }
        })
        .collect()
}

fn summarize(plan: &CampaignPlan, nodes: &[NodeResult]) -> (Vec<GroupSummary>, Vec<MarginalSummary>) {
    let mut summaries = Vec::new();
    let mut marginals = Vec::new();
    let start_of = |m: &NodeResult| m.run.start.as_ref().map(|s| s.digest.clone());
    for g in &plan.groups {
        let members: Vec<&NodeResult> = nodes.iter().filter(|n| n.plan.group.as_deref() == Some(g.id.as_str())).collect();
        // one summary per treatment, block combination and starting state, in the order the trials were planned:
        // nothing is pooled across blocks or across starting states
        let mut keys: Vec<(String, Vec<BlockAssignment>, Option<String>)> = Vec::new();
        for m in &members {
            let k = (m.plan.treatment.clone().unwrap_or_default(), m.plan.blocks.clone(), start_of(m));
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        for (treatment, blocks, start) in keys {
            let trials: Vec<&&NodeResult> = members.iter().filter(|m| m.plan.treatment.as_deref() == Some(treatment.as_str()) && m.plan.blocks == blocks && start_of(m) == start).collect();
            let t = g.treatments.iter().find(|t| t.name == treatment);
            let mut values = Values::new();
            let (mut outcomes, mut promotions, mut states) = (BTreeMap::new(), BTreeMap::new(), BTreeMap::new());
            for m in &trials {
                let r = &m.run;
                if r.status == NodeStatus::Completed {
                    collect(&mut values, r);
                }
                if let Some(o) = &r.outcome {
                    *outcomes.entry(word(o)).or_insert(0) += 1;
                }
                if let Some(p) = &r.promotion {
                    *promotions.entry(word(p)).or_insert(0) += 1;
                }
                if let Some(s) = &r.state {
                    *states.entry(s.word().to_owned()).or_insert(0) += 1;
                }
            }
            summaries.push(GroupSummary {
                group: g.id.clone(),
                kind: g.kind.clone(),
                design: g.design,
                treatment: treatment.clone(),
                assignment: t.map(|t| t.assignment.clone()).unwrap_or_default(),
                varied: t.and_then(|t| t.varied.clone()),
                blocks,
                start_digest: start,
                requested: trials.len(),
                completed: trials.iter().filter(|m| m.run.status == NodeStatus::Completed).count(),
                trials: trials.iter().map(|m| TrialRef { node: m.plan.id.clone(), replicate: m.plan.replicate, status: m.run.status, result: m.run.result.clone() }).collect(),
                outcomes,
                promotions,
                states,
                measurements: stats(values),
            });
        }
        if g.kind != "sweep" {
            continue;
        }
        // by each factor's level, within one block combination and one starting state
        let mut cells: Vec<(Vec<BlockAssignment>, Option<String>)> = Vec::new();
        for m in &members {
            let k = (m.plan.blocks.clone(), start_of(m));
            if !cells.contains(&k) {
                cells.push(k);
            }
        }
        for (blocks, start) in cells {
            for f in &g.factors {
                let Some(d) = plan.factors.iter().find(|x| &x.name == f) else { continue };
                for level in &d.levels {
                    let trials: Vec<&&NodeResult> = members.iter().filter(|m| m.plan.blocks == blocks && start_of(m) == start && m.plan.assignment.iter().any(|a| &a.factor == f && &a.value == level)).collect();
                    if trials.is_empty() {
                        continue;
                    }
                    let mut values = Values::new();
                    for m in trials.iter().filter(|m| m.run.status == NodeStatus::Completed) {
                        collect(&mut values, &m.run);
                    }
                    marginals.push(MarginalSummary { group: g.id.clone(), factor: f.clone(), level: level.clone(), blocks: blocks.clone(), start_digest: start.clone(), trials: trials.len(), completed: trials.iter().filter(|m| m.run.status == NodeStatus::Completed).count(), measurements: stats(values) });
                }
            }
        }
    }
    (summaries, marginals)
}
