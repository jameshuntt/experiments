//! The campaign's governor of meaning: which nodes are ready, what a node's outcome makes of its candidate (accepted
//! or put back), when the campaign stops, and what is kept so a run can be resumed. It runs nothing and writes nothing:
//! the host (Custodian) makes the workspaces, runs each node through its hosted experiment line, puts a rejected
//! candidate back through its own declared lines, and tells the runner what happened. Promotion is separate from the
//! scientific conclusion and from execution: a do-not-promote is a finding, never a failure.

use super::plan::{CampaignError, CampaignPlan, PlanNode};
use super::spec::*;
use crate::{Evidence, PromotionDecision};
use scientific_method::{Observations, Outcome};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// A tree's scientific identity as the host reads it: its digest (every file's path, content and mode, the host's
/// runtime paths and the campaign's `exclude` left out) and how many files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateIdentity {
    pub digest: String,
    pub files: usize,
}

/// A file kept as evidence: where, and its digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRef {
    pub path: PathBuf,
    pub digest: String,
}

/// How a node ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    /// The experiment ran to its conclusion (whatever its promotion).
    Completed,
    /// The host refused an operation (a preflight, a writer, a start state not as planned, a guarantee strict needs).
    Refused,
    /// The experiment could not run to its conclusion (a command failed, a measurement did not parse).
    Failed,
    /// Never run: what it waits for did not complete, or the campaign stopped.
    Skipped,
}

impl NodeStatus {
    pub fn word(self) -> &'static str {
        match self {
            NodeStatus::Completed => "completed",
            NodeStatus::Refused => "refused",
            NodeStatus::Failed => "failed",
            NodeStatus::Skipped => "skipped",
        }
    }
}

/// What became of a node's candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateAction {
    /// The candidate is the workspace's accepted state.
    Accepted,
    /// The previous accepted state was put back, exactly.
    Restored,
    /// The previous accepted state could not be put back: the workspace is not used again.
    Discarded,
}

impl StateAction {
    pub fn word(self) -> &'static str {
        match self {
            StateAction::Accepted => "accepted",
            StateAction::Restored => "restored",
            StateAction::Discarded => "discarded",
        }
    }
}

/// What the host saw when it ran a node.
#[derive(Debug, Clone, PartialEq)]
pub enum Execution {
    /// The experiment ran to its conclusion: its promotion, its outcome, its result file, its observations, its run.
    Concluded { promotion: PromotionDecision, outcome: Outcome, result: FileRef, run_id: String, observations: Observations },
    Refused(String),
    Failed(String),
}

/// What the runner decided from an execution: the node's status, whether its candidate is accepted, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub status: NodeStatus,
    pub accept: bool,
    pub reason: Option<String>,
    pub findings: Vec<String>,
}

/// One event of the actual order things ran in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderEvent {
    pub seq: u64,
    pub node: String,
    /// `started`, `finished` or `skipped`.
    pub event: String,
    /// How many nodes were running once this event happened.
    pub running: usize,
}

/// A node as it ended: everything the result says about its execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeRecord {
    pub id: String,
    pub status: NodeStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promotion: Option<PromotionDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Outcome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<StateAction>,
    /// Where the workspace is (the governed root for the shared one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_path: Option<PathBuf>,
    /// The state it started from, as read before it ran.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<StateIdentity>,
    /// The state its run left.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate: Option<StateIdentity>,
    /// The workspace's accepted state once it ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted: Option<StateIdentity>,
    /// The experiment as it ran (bound), kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_spec: Option<FileRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<FileRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// The host's receipts for the node's lines (the experiment line, its nested lines, a restore's).
    #[serde(default)]
    pub receipts: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    /// Each line that put the previous accepted state back, in the order it ran.
    #[serde(default)]
    pub restore: Vec<String>,
    #[serde(default)]
    pub findings: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observations: Option<Observations>,
}

impl NodeRecord {
    pub fn new(id: &str, status: NodeStatus) -> NodeRecord {
        NodeRecord { id: id.to_owned(), status, reason: None, promotion: None, outcome: None, state: None, workspace_path: None, start: None, candidate: None, accepted: None, bound_spec: None, result: None, run_id: None, receipts: Vec::new(), evidence: Vec::new(), restore: Vec::new(), findings: Vec::new(), observations: None }
    }
}

/// A workspace as the campaign last left it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkspaceState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted: Option<StateIdentity>,
    /// The last node that left it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<String>,
    #[serde(default)]
    pub discarded: bool,
}

/// Everything a campaign run keeps between nodes: what a resume reads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignProgress {
    pub schema: String,
    pub campaign_id: String,
    pub run_id: String,
    pub campaign_digest: String,
    pub plan_digest: String,
    /// Each experiment's digest, by its path.
    pub spec_digests: BTreeMap<String, String>,
    /// The factors, controls, blocks and designs the plan was made with (their digest).
    pub design_digest: String,
    /// The campaign's start: the shared workspace's state when the run began.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<StateIdentity>,
    pub nodes: BTreeMap<String, NodeRecord>,
    pub workspaces: BTreeMap<String, WorkspaceState>,
    pub order: Vec<OrderEvent>,
    #[serde(default)]
    pub running: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped: Option<String>,
    pub seq: u64,
}

fn design_digest(plan: &CampaignPlan) -> String {
    super::digest::of("campaign-design", &(&plan.factors, &plan.controls, &plan.blocks, &plan.groups, &plan.rigor, &plan.policy, &plan.exclude))
}

/// The campaign as it runs.
#[derive(Debug, Clone)]
pub struct CampaignRunner {
    plan: CampaignPlan,
    progress: CampaignProgress,
}

impl CampaignRunner {
    /// A fresh run of `plan`, named `run_id`.
    pub fn new(plan: CampaignPlan, run_id: &str) -> CampaignRunner {
        let progress = CampaignProgress {
            schema: CAMPAIGN_PROGRESS_SCHEMA.into(),
            campaign_id: plan.campaign_id.clone(),
            run_id: run_id.to_owned(),
            campaign_digest: plan.campaign_digest.clone(),
            plan_digest: plan.digest.clone(),
            spec_digests: plan.specs.iter().map(|s| (s.path.clone(), s.digest.clone())).collect(),
            design_digest: design_digest(&plan),
            start: None,
            nodes: BTreeMap::new(),
            workspaces: BTreeMap::new(),
            order: Vec::new(),
            running: Vec::new(),
            stopped: None,
            seq: 0,
        };
        CampaignRunner { plan, progress }
    }

    /// The run `progress` kept, resumed over `plan`: refused when anything the run was made from has moved (the
    /// campaign, an experiment, a factor, a design: their digests), so no result is reused against another plan. A
    /// node that was running, refused, failed or skipped runs again; a node that completed keeps its result.
    pub fn resume(plan: CampaignPlan, mut progress: CampaignProgress) -> Result<CampaignRunner, CampaignError> {
        let mut why = Vec::new();
        if progress.schema != CAMPAIGN_PROGRESS_SCHEMA {
            why.push(format!("the progress is `{}`, expected `{CAMPAIGN_PROGRESS_SCHEMA}`", progress.schema));
        }
        if progress.campaign_id != plan.campaign_id {
            why.push(format!("the progress is campaign {}'s, this is {}", progress.campaign_id, plan.campaign_id));
        }
        if progress.campaign_digest != plan.campaign_digest {
            why.push(format!("the campaign is {} now, the run began with {}", plan.campaign_digest, progress.campaign_digest));
        }
        for s in &plan.specs {
            match progress.spec_digests.get(&s.path) {
                Some(d) if d == &s.digest => {}
                Some(d) => why.push(format!("experiment {} is {} now, the run began with {d}", s.path, s.digest)),
                None => why.push(format!("experiment {} is new since the run began", s.path)),
            }
        }
        for p in progress.spec_digests.keys().filter(|p| plan.spec(p).is_none()) {
            why.push(format!("experiment {p} is gone since the run began"));
        }
        if progress.design_digest != design_digest(&plan) {
            why.push("the factors, controls, blocks, designs, rigor or policy moved since the run began".into());
        }
        if progress.plan_digest != plan.digest {
            why.push(format!("the plan is {} now, the run began with {}", plan.digest, progress.plan_digest));
        }
        if !why.is_empty() {
            why.insert(0, format!("campaign {} run {} cannot resume (run it fresh with --again)", plan.campaign_id, progress.run_id));
            return Err(CampaignError(why));
        }
        let again: Vec<String> = progress.nodes.values().filter(|r| r.status != NodeStatus::Completed).map(|r| r.id.clone()).collect();
        for id in again {
            progress.nodes.remove(&id);
        }
        progress.running.clear();
        progress.stopped = None;
        let mut me = CampaignRunner { plan, progress };
        // a stop the completed nodes decided stands
        let completed: Vec<NodeRecord> = me.plan.nodes.iter().filter_map(|n| me.progress.nodes.get(&n.id).cloned()).collect();
        for r in &completed {
            me.stop_rule(r);
        }
        Ok(me)
    }

    pub fn plan(&self) -> &CampaignPlan {
        &self.plan
    }

    pub fn progress(&self) -> &CampaignProgress {
        &self.progress
    }

    pub fn record(&self, id: &str) -> Option<&NodeRecord> {
        self.progress.nodes.get(id)
    }

    pub fn stopped(&self) -> Option<&str> {
        self.progress.stopped.as_deref()
    }

    /// The campaign's start, read by the host before the first node (kept from the first run on resume).
    pub fn set_start(&mut self, s: StateIdentity) {
        self.progress.start.get_or_insert(s);
    }

    pub fn start_state(&self) -> Option<&StateIdentity> {
        self.progress.start.as_ref()
    }

    pub fn workspace(&self, id: &str) -> Option<&WorkspaceState> {
        self.progress.workspaces.get(id)
    }

    /// The state `node` must start from: its state parent's accepted state, else the campaign's start.
    pub fn expected_start(&self, node: &PlanNode) -> Option<StateIdentity> {
        match &node.from {
            None => self.progress.start.clone(),
            Some(p) => self.progress.nodes.get(p).and_then(|r| r.accepted.clone()),
        }
    }

    /// Whether every node has ended.
    pub fn finished(&self) -> bool {
        self.plan.nodes.iter().all(|n| self.progress.nodes.contains_key(&n.id))
    }

    fn event(&mut self, node: &str, event: &str) {
        self.progress.seq += 1;
        let running = self.progress.running.len();
        self.progress.order.push(OrderEvent { seq: self.progress.seq, node: node.to_owned(), event: event.to_owned(), running });
    }

    /// The nodes that may start now, in plan order: not started, everything they wait for completed. A node waiting
    /// for one that did not complete is skipped here, with why; nothing new is ready once the campaign stopped.
    pub fn ready(&mut self) -> Vec<PlanNode> {
        if self.progress.stopped.is_some() {
            return Vec::new();
        }
        loop {
            let mut skipped = false;
            let mut ready = Vec::new();
            for n in &self.plan.nodes {
                if self.progress.nodes.contains_key(&n.id) || self.progress.running.contains(&n.id) {
                    continue;
                }
                let mut wait = false;
                let mut blocked: Option<String> = None;
                for a in &n.after {
                    match self.progress.nodes.get(a) {
                        None => wait = true,
                        Some(r) if r.status != NodeStatus::Completed => blocked = Some(format!("waits for {a}, which was {}", r.status.word())),
                        Some(r) if n.from.as_deref() == Some(a.as_str()) && n.strategy != WorkspaceStrategy::Isolated && r.state == Some(StateAction::Discarded) => blocked = Some(format!("its state parent {a}'s workspace could not be put back")),
                        Some(_) => {}
                    }
                }
                if let Some(why) = blocked {
                    let mut r = NodeRecord::new(&n.id, NodeStatus::Skipped);
                    r.reason = Some(why);
                    self.progress.nodes.insert(n.id.clone(), r);
                    let id = n.id.clone();
                    self.event(&id, "skipped");
                    skipped = true;
                    break;
                }
                if !wait {
                    ready.push(n.clone());
                }
            }
            if !skipped {
                return ready;
            }
        }
    }

    /// `id` starts now.
    pub fn start(&mut self, id: &str) {
        if !self.progress.running.iter().any(|r| r == id) {
            self.progress.running.push(id.to_owned());
        }
        self.event(id, "started");
    }

    /// What `node`'s execution makes of its candidate. `undeclared`: the files the run changed that no declared
    /// change accounts for (the host compared the start and candidate states with the change records).
    /// It reads nothing but the node (its rigor and policy) and what the host saw, so a host may decide where the
    /// node ran.
    pub fn decide(node: &PlanNode, execution: &Execution, undeclared: &[String]) -> Decision {
        let (promotion, mut d) = match execution {
            Execution::Refused(why) => return Decision { status: NodeStatus::Refused, accept: false, reason: Some(why.clone()), findings: Vec::new() },
            Execution::Failed(why) => return Decision { status: NodeStatus::Failed, accept: false, reason: Some(why.clone()), findings: Vec::new() },
            Execution::Concluded { promotion, .. } => (*promotion, Decision { status: NodeStatus::Completed, accept: false, reason: None, findings: Vec::new() }),
        };
        d.accept = match promotion {
            PromotionDecision::Promote => true,
            PromotionDecision::DoNotPromote => {
                d.findings.push("do not promote: the candidate is not accepted".into());
                false
            }
            PromotionDecision::Undecided => {
                let accept = node.policy.on_undecided == OnUndecided::Accept;
                d.findings.push(format!("undecided: the candidate is {} (on_undecided {})", if accept { "accepted" } else { "not accepted" }, if accept { "accept" } else { "restore" }));
                accept
            }
        };
        if !undeclared.is_empty() {
            let what = format!("undeclared change (no declared change accounts for it): {}", undeclared.join(", "));
            match node.rigor {
                Rigor::Exploratory => d.findings.push(what),
                Rigor::Controlled => {
                    d.findings.push(format!("{what}; controlled: the candidate is not accepted"));
                    d.accept = false;
                }
                Rigor::Strict => {
                    return Decision { status: NodeStatus::Refused, accept: false, reason: Some(format!("strict: {what}")), findings: Vec::new() };
                }
            }
        }
        d
    }

    /// `record` ended (`running` no longer). The workspace's accepted state is the record's; the stop rules apply.
    pub fn finish(&mut self, mut record: NodeRecord) {
        let id = record.id.clone();
        self.progress.running.retain(|r| r != &id);
        if let Some(n) = self.plan.node(&id) {
            let ws = self.progress.workspaces.entry(n.workspace.clone()).or_default();
            if record.workspace_path.is_some() {
                ws.path = record.workspace_path.clone();
            }
            if record.accepted.is_some() {
                ws.accepted = record.accepted.clone();
            }
            ws.last = Some(id.clone());
            if record.state == Some(StateAction::Discarded) {
                ws.discarded = true;
            }
        }
        if record.status == NodeStatus::Completed && record.promotion.is_none() {
            record.status = NodeStatus::Failed;
            record.reason.get_or_insert_with(|| "completed without a promotion".into());
        }
        self.stop_rule(&record);
        self.progress.nodes.insert(id.clone(), record);
        self.event(&id, "finished");
    }

    fn stop_rule(&mut self, r: &NodeRecord) {
        if self.progress.stopped.is_some() {
            return;
        }
        let Some(n) = self.plan.node(&r.id) else { return };
        let p = n.policy;
        let why = match r.status {
            NodeStatus::Refused if p.on_refusal == Continuation::Stop => Some(format!("{} was refused (on_refusal stop): {}", r.id, r.reason.clone().unwrap_or_default())),
            NodeStatus::Failed if p.on_error == Continuation::Stop => Some(format!("{} failed (on_error stop): {}", r.id, r.reason.clone().unwrap_or_default())),
            NodeStatus::Completed if p.on_findings == Continuation::Stop && !r.findings.is_empty() => Some(format!("{} has findings (on_findings stop): {}", r.id, r.findings.join("; "))),
            _ => None,
        };
        let why = why.or_else(|| (r.state == Some(StateAction::Discarded) && n.workspace == "shared").then(|| format!("{}: the shared workspace could not be put back to its accepted state", r.id)));
        self.progress.stopped = why;
    }

    /// Every node not ended, skipped with why (the campaign stopped). Called once nothing runs any more.
    pub fn close(&mut self) {
        let why = self.progress.stopped.clone().map(|s| format!("the campaign stopped: {s}")).unwrap_or_else(|| "never became ready".into());
        let left: Vec<String> = self.plan.nodes.iter().filter(|n| !self.progress.nodes.contains_key(&n.id)).map(|n| n.id.clone()).collect();
        for id in left {
            let mut r = NodeRecord::new(&id, NodeStatus::Skipped);
            r.reason = Some(why.clone());
            self.progress.running.retain(|x| x != &id);
            self.progress.nodes.insert(id.clone(), r);
            self.event(&id, "skipped");
        }
    }
}
