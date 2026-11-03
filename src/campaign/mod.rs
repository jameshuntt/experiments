//! Campaigns: experiments composed over an evolving accepted state, and experimental designs over factors.
//!
//! A [`CampaignSpec`] (schema `experiments.campaign.v1`; canonical file `v1.<name>.1o.campaign`, or its JSON, TOML,
//! YAML twins) composes experiment specs with `run`, `sequence`, `parallel`, `repeat` and `sweep`, or names each run's
//! `after` and `from` (the flat form). [`LoadedCampaign::load`] reads it and every experiment it names and makes one
//! deterministic [`CampaignPlan`]: every node with its stable id, its experiment's digest, its dependencies, its one
//! state parent, its workspace, its factor assignment, controls, blocks, replicate, group and design. Nothing runs and
//! nothing is written while a plan is made.
//!
//! A host runs the plan: [`CampaignRunner`] says which nodes are ready, decides from each node's execution whether its
//! candidate is accepted or put back ([`CampaignRunner::decide`]), applies the campaign's policies, keeps the
//! progress a resume reads, and makes the [`CampaignResult`]. The host (Custodian) owns the workspaces, the
//! authorization, the writer and the receipts: every node runs through its hosted experiment line, and every source
//! change through Code Passenger.

mod bind;
pub mod digest;
mod plan;
mod result;
mod run;
mod spec;

pub use bind::Binding;
pub use plan::{CampaignError, CampaignPlan, LoadedCampaign, PlanNode, PlannedGroup, PlannedSpec, PlannedTreatment, PlannedWorkspace, MAX_TRIALS, START};
pub use result::{CampaignResult, CampaignStatus, Counts, GroupSummary, HostFacts, MarginalSummary, MeasurementSummary, NodeResult, ToolVersion, TrialRef, WorkspaceResult};
pub use run::{CampaignProgress, CampaignRunner, Decision, Execution, FileRef, NodeRecord, NodeStatus, OrderEvent, StateAction, StateIdentity, WorkspaceState};
pub use spec::*;
