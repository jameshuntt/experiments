//! How a trial's assignment reaches its experiment: a typed binding, made once from the plan's node, given to every
//! command of the experiment as environment variables through the experiment's own [`CommandSpec::env`]. Nothing is
//! templated into a word and no shell sees it: the program is given `EXPERIMENT_FACTOR_<NAME>=<level>` exactly as the
//! plan shows it, and the command's record keeps what it was given.
//!
//! The variables, in order: `EXPERIMENT_CAMPAIGN` (the campaign's id), `EXPERIMENT_TRIAL` (the node's id),
//! `EXPERIMENT_REPLICATE` and `EXPERIMENT_TREATMENT` (a repeat's or a sweep's trial), then `EXPERIMENT_FACTOR_<NAME>`
//! for each factor the node sets, `EXPERIMENT_CONTROL_<NAME>` for each control, `EXPERIMENT_BLOCK_<NAME>` for each
//! block; `<NAME>` is the declared name upper-cased.

use super::plan::{change_of, target_of, PlanNode};
use crate::{CommandSpec, EnvVar, ExperimentSpec};
use std::path::Path;

/// A node's binding: the variables every command of its experiment is given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    vars: Vec<EnvVar>,
}

fn var(name: impl Into<String>, value: impl Into<String>) -> EnvVar {
    EnvVar { name: name.into(), value: value.into() }
}

impl Binding {
    /// The binding the plan made for `node`.
    pub fn of(node: &PlanNode) -> Binding {
        Binding { vars: node.bindings.clone() }
    }

    /// The binding of `node` in campaign `campaign`, made from its assignment, controls and blocks.
    pub fn of_parts(campaign: &str, node: &PlanNode) -> Binding {
        let mut vars = vec![var("EXPERIMENT_CAMPAIGN", campaign), var("EXPERIMENT_TRIAL", &node.id)];
        if let Some(r) = node.replicate {
            vars.push(var("EXPERIMENT_REPLICATE", r.to_string()));
        }
        if let Some(t) = &node.treatment {
            vars.push(var("EXPERIMENT_TREATMENT", t));
        }
        vars.extend(node.assignment.iter().map(|a| var(format!("EXPERIMENT_FACTOR_{}", a.factor.to_ascii_uppercase()), a.value.to_string())));
        vars.extend(node.controls.iter().map(|c| var(format!("EXPERIMENT_CONTROL_{}", c.name.to_ascii_uppercase()), c.value.to_string())));
        vars.extend(node.blocks.iter().map(|b| var(format!("EXPERIMENT_BLOCK_{}", b.name.to_ascii_uppercase()), b.level.to_string())));
        Binding { vars }
    }

    pub fn vars(&self) -> Vec<EnvVar> {
        self.vars.clone()
    }

    /// `spec` bound for a run in `workspace`: its target the workspace, each change spec where the experiment
    /// declared it (resolved against the declared target under `root`), every command given the binding.
    pub fn apply(&self, spec: &ExperimentSpec, root: &Path, workspace: &Path) -> Result<ExperimentSpec, String> {
        conflicts(spec, &self.vars)?;
        let mut out = spec.clone();
        let declared = target_of(spec, root);
        for c in &mut out.mutation.changes {
            c.spec = change_of(&declared, &c.spec);
        }
        // the tree an exact tree measurement must equal is where the experiment declared it, as a change spec is
        for m in &mut out.measurements.measurements {
            if let crate::MeasurementSource::ExactTree(e) = &mut m.source {
                e.equals_tree = change_of(&declared, &e.equals_tree);
            }
        }
        out.target.root = workspace.to_path_buf();
        for c in commands_mut(&mut out) {
            c.env.extend(self.vars.iter().cloned());
        }
        Ok(out)
    }
}

/// Refused when a command of `spec` already names a variable the binding gives: the binding would be a second
/// source of its value.
pub(crate) fn conflicts(spec: &ExperimentSpec, vars: &[EnvVar]) -> Result<(), String> {
    for c in commands(spec) {
        if let Some(v) = c.env.iter().find(|e| vars.iter().any(|b| b.name == e.name)) {
            return Err(format!("experiment {}: command `{}` names {} itself, which the campaign's binding gives", spec.id, c.label, v.name));
        }
    }
    Ok(())
}

fn commands(spec: &ExperimentSpec) -> impl Iterator<Item = &CommandSpec> {
    let p = &spec.protocol;
    p.prepare.iter().chain(&p.baseline).chain(&p.intervention).chain(&p.candidate).chain(&p.cleanup).chain(&spec.verification.commands).chain(spec.measurements.measurements.iter().filter_map(|m| m.command()))
}

fn commands_mut(spec: &mut ExperimentSpec) -> impl Iterator<Item = &mut CommandSpec> {
    let p = &mut spec.protocol;
    p.prepare.iter_mut().chain(p.baseline.iter_mut()).chain(p.intervention.iter_mut()).chain(p.candidate.iter_mut()).chain(p.cleanup.iter_mut()).chain(spec.verification.commands.iter_mut()).chain(spec.measurements.measurements.iter_mut().filter_map(|m| m.command_mut()))
}
