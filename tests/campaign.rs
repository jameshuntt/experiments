//! Campaigns: the document in every format, the plan it normalizes to, the designs it expands, what it refuses
//! before anything runs, and the runner's decisions, stops, resume and summaries.

mod common;

use experiments::campaign::*;
use experiments::{read_campaign, write_campaign, write_spec, MutationMode, PromotionDecision};
use scientific_method::{Control, IndependentVariable, Measurement, Observation, Observations, Outcome};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};

/// An experiment spec at `dir/name.json` over `root`, declaring these independent variables and controls.
fn experiment(dir: &Path, root: &Path, name: &str, variables: &[&str], controls: &[&str], mode: MutationMode) -> PathBuf {
    let mut s = common::base_spec(root);
    s.id = name.into();
    for v in variables {
        s.scientific_method.independent_variables.push(IndependentVariable::new(*v));
    }
    for c in controls {
        s.scientific_method.controls.push(Control::new(*c));
    }
    s.mutation.mode = mode;
    s.measurements.measurements.push(experiments::MeasurementSpec {
        phase: experiments::MeasurementPhase::Candidate,
        condition: "candidate".into(),
        name: "latency".into(),
        unit: Some("ms".into()),
        command: common::command("latency", "printf", &["1"]),
        parser: experiments::MeasurementParser::Number,
    });
    let p = dir.join(format!("{name}.json"));
    write_spec(&p, &s).unwrap();
    p
}

struct Lab {
    _dir: TempDir,
    root: PathBuf,
    base: PathBuf,
}

/// A governed root and, beside it, a folder of experiments: a, b, c, d (no variables), bench (matcher, population,
/// geo_concentration; controls corpus, seed, demand_model), loose (unbounded).
fn lab() -> Lab {
    let dir = tempdir().unwrap();
    let root = dir.path().join("root");
    let base = dir.path().join("campaign");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&base).unwrap();
    let root = root.canonicalize().unwrap();
    for n in ["a", "b", "c", "d"] {
        experiment(&base, &root, n, &["implementation"], &[], MutationMode::Bounded);
    }
    experiment(&base, &root, "bench", &["matcher", "population", "geo_concentration"], &["corpus", "seed", "demand_model"], MutationMode::Bounded);
    experiment(&base, &root, "loose", &[], &[], MutationMode::Unbounded);
    Lab { _dir: dir, root, base }
}

fn load(lab: &Lab, file: &str, text: &str) -> Result<LoadedCampaign, CampaignError> {
    let p = lab.base.join(file);
    fs::write(&p, text).unwrap();
    LoadedCampaign::load(&p, &lab.root)
}

fn plan(lab: &Lab, file: &str, text: &str) -> CampaignPlan {
    load(lab, file, text).unwrap_or_else(|e| panic!("{file}: {e}")).plan
}

fn refused(lab: &Lab, text: &str) -> String {
    match load(lab, "refused.json", text) {
        Ok(_) => panic!("planned, but should be refused:\n{text}"),
        Err(e) => e.to_string(),
    }
}

const MATRIX: &str = r#"campaign {
  schema "experiments.campaign.v1"
  id "matrix"
  rigor "controlled"
  policy {
    max_parallel 4
  }
  factor {
    name "matcher"
    level "scalar"
    level "simd"
  }
  factor {
    name "population"
    level "1K"
    level "10K"
    level "100K"
    level "1M"
  }
  factor {
    name "geo_concentration"
    level "low"
    level "medium"
    level "high"
  }
  control {
    name "corpus"
    value "corpus-v3"
  }
  control {
    name "seed"
    value 7
  }
  control {
    name "demand_model"
    value "poisson"
  }
  block {
    name "machine"
    level "lab-1"
  }
  sweep {
    id "grid"
    spec "bench.json"
    design "factorial"
    factor "matcher"
    factor "population"
    factor "geo_concentration"
    repeats 10
    block "machine"
  }
}
"#;

#[test]
fn the_campaign_document_is_real_1o_and_round_trips_through_every_format() {
    let lab = lab();
    let p = lab.base.join("v1.matrix.1o.campaign");
    fs::write(&p, MATRIX).unwrap();
    let spec = read_campaign(&p).unwrap();
    assert_eq!(spec.factors.len(), 3);
    assert_eq!(spec.controls[1].value, Level::Integer(7));
    let rich = rich_campaign();
    for name in ["v1.rich.1o.campaign", "rich.json", "rich.toml", "rich.yaml", "rich.yml"] {
        let f = lab.base.join(name);
        write_campaign(&f, &rich).unwrap_or_else(|e| panic!("write {name}: {e}"));
        let back = read_campaign(&f).unwrap_or_else(|e| panic!("read {name}: {e}"));
        assert_eq!(back, rich, "{name}");
    }
    let oneo = fs::read_to_string(lab.base.join("v1.rich.1o.campaign")).unwrap();
    assert!(oneo.starts_with("campaign {\n") && !oneo.trim_start().starts_with('{'), "1o syntax, not JSON:\n{oneo}");
    // the steps keep their order across kinds, at every depth
    let (seq, par) = (oneo.find("  sequence {").unwrap(), oneo.find("  parallel {").unwrap());
    assert!(seq < par, "{oneo}");
    // the same campaign in every format plans the same
    let digests: Vec<String> = ["v1.rich.1o.campaign", "rich.json", "rich.toml", "rich.yaml"].iter().map(|n| LoadedCampaign::load(&lab.base.join(n), &lab.root).unwrap().plan.digest).collect();
    assert!(digests.windows(2).all(|w| w[0] == w[1]), "{digests:?}");
}

fn rich_campaign() -> CampaignSpec {
    let run = |id: &str, spec: &str| RunStep { id: id.into(), spec: spec.into(), set: Vec::new(), blocks: Vec::new(), after: Vec::new(), from: None, workspace: None };
    let set = |f: &str, v: Level| Assignment { factor: f.into(), value: v };
    CampaignSpec {
        schema: CAMPAIGN_SCHEMA.into(),
        id: "rich".into(),
        description: Some("every step kind, nested".into()),
        rigor: Rigor::Exploratory,
        policy: CampaignPolicy { on_findings: Continuation::Stop, on_refusal: Continuation::Continue, on_error: Continuation::Stop, on_undecided: OnUndecided::Accept, max_parallel: 3 },
        factors: vec![
            Factor { name: "implementation".into(), description: Some("which one".into()), levels: vec![Level::Text("old".into()), Level::Text("new".into())], scope: FactorScope::Experiment },
            Factor { name: "threads".into(), description: None, levels: vec![Level::Integer(1), Level::Integer(4)], scope: FactorScope::Campaign },
            Factor { name: "ratio".into(), description: None, levels: vec![Level::Number(0.5), Level::Number(1.0)], scope: FactorScope::Campaign },
            Factor { name: "cached".into(), description: None, levels: vec![Level::Boolean(true), Level::Boolean(false)], scope: FactorScope::Campaign },
        ],
        controls: vec![ControlValue { name: "seed".into(), value: Level::Integer(7), description: Some("fixed".into()) }],
        blocks: vec![BlockFactor { name: "machine".into(), levels: vec![Level::Text("m1".into()), Level::Text("m2".into())], description: None }],
        exclude: vec!["run".into()],
        steps: vec![
            Step::Sequence(GroupStep {
                id: Some("history".into()),
                after: Vec::new(),
                from: None,
                workspace: None,
                steps: vec![
                    Step::Run(RunStep { set: vec![set("implementation", Level::Text("old".into())), set("ratio", Level::Number(0.5))], blocks: vec![BlockAssignment { name: "machine".into(), level: Level::Text("m1".into()) }], ..run("a", "a.json") }),
                    Step::Parallel(GroupStep { id: Some("fan".into()), after: Vec::new(), from: None, workspace: None, steps: vec![Step::Run(run("b", "b.json")), Step::Sequence(GroupStep { id: None, after: Vec::new(), from: None, workspace: None, steps: vec![Step::Run(run("c", "c.json")), Step::Repeat(RepeatStep { id: "again".into(), times: 2, after: Vec::new(), from: None, run: run("t", "d.json") })] })] }),
                    Step::Run(run("d", "d.json")),
                ],
            }),
            Step::Parallel(GroupStep { id: Some("side".into()), after: vec!["history".into()], from: Some("history".into()), workspace: None, steps: vec![Step::Run(run("e", "a.json"))] }),
            Step::Sweep(SweepStep { id: "grid".into(), spec: "a.json".into(), design: Design::Factorial, factors: vec!["implementation".into(), "cached".into()], reference: Vec::new(), treatments: Vec::new(), set: vec![set("threads", Level::Integer(4))], repeats: 2, blocks: vec!["machine".into()], after: vec!["side".into()], from: None }),
            Step::Sweep(SweepStep { id: "one".into(), spec: "a.json".into(), design: Design::OneFactorAtATime, factors: vec!["implementation".into(), "threads".into()], reference: vec![set("implementation", Level::Text("old".into())), set("threads", Level::Integer(1))], treatments: Vec::new(), set: Vec::new(), repeats: 1, blocks: Vec::new(), after: Vec::new(), from: None }),
            Step::Sweep(SweepStep { id: "named".into(), spec: "a.json".into(), design: Design::Explicit, factors: Vec::new(), reference: Vec::new(), treatments: vec![Treatment { name: "fast".into(), set: vec![set("implementation", Level::Text("new".into()))] }], set: Vec::new(), repeats: 1, blocks: Vec::new(), after: Vec::new(), from: None }),
            Step::Run(RunStep { after: vec!["d".into()], from: Some("e".into()), workspace: Some(WorkspaceStrategy::Continue), ..run("f", "b.json") }),
        ],
    }
}

#[test]
fn every_format_refuses_an_unknown_key_at_any_depth() {
    let lab = lab();
    // 1o: at the top, in a nested step three deep, in a design
    for (name, text) in [
        ("v1.top.1o.campaign", "campaign {\n  id \"x\"\n  magic true\n  run { id \"a\" spec \"a.json\" }\n}\n"),
        ("v1.deep.1o.campaign", "campaign {\n  id \"x\"\n  sequence {\n    parallel {\n      sequence {\n        run { id \"a\" spec \"a.json\" magic 1 }\n      }\n    }\n  }\n}\n"),
        ("v1.sweep.1o.campaign", "campaign {\n  id \"x\"\n  sweep { id \"s\" spec \"a.json\" design \"factorial\" factr \"x\" }\n}\n"),
        ("v1.kind.1o.campaign", "campaign {\n  id \"x\"\n  run { id 4 spec \"a.json\" }\n}\n"),
        ("v1.enum.1o.campaign", "campaign {\n  id \"x\"\n  rigor \"sloppy\"\n  run { id \"a\" spec \"a.json\" }\n}\n"),
    ] {
        let p = lab.base.join(name);
        fs::write(&p, text).unwrap();
        assert!(read_campaign(&p).is_err(), "{name} must be refused");
    }
    // JSON, TOML, YAML: an unknown key, a typo'd factor field, an unknown step kind
    for (name, text) in [
        ("top.json", r#"{"id":"x","magic":true,"steps":[]}"#),
        ("deep.json", r#"{"id":"x","steps":[{"sequence":{"steps":[{"run":{"id":"a","spec":"a.json","sett":[]}}]}}]}"#),
        ("factor.json", r#"{"id":"x","factors":[{"name":"a","level":["x"]}],"steps":[]}"#),
        ("kind.json", r#"{"id":"x","steps":[{"loop":{"id":"a"}}]}"#),
        ("top.toml", "id = \"x\"\nmagic = true\nsteps = []\n"),
        ("top.yaml", "id: x\nmagic: true\nsteps: []\n"),
    ] {
        let p = lab.base.join(name);
        fs::write(&p, text).unwrap();
        assert!(read_campaign(&p).is_err(), "{name} must be refused");
    }
    // the schema word is gated in every format
    let mut wrong = rich_campaign();
    wrong.schema = "experiments.campaign.v9".into();
    for name in ["wrong.json", "wrong.toml", "wrong.yaml", "v1.wrong.1o.campaign"] {
        let p = lab.base.join(name);
        write_campaign(&p, &wrong).unwrap();
        assert!(read_campaign(&p).unwrap_err().to_string().contains(CAMPAIGN_SCHEMA), "{name}");
    }
    let txt = lab.base.join("campaign.txt");
    fs::write(&txt, "{}").unwrap();
    assert!(read_campaign(&txt).unwrap_err().to_string().contains("unsupported campaign format"));
}

#[test]
fn every_format_writes_a_step_as_a_one_key_map() {
    let lab = lab();
    let p = lab.base.join("map.yaml");
    fs::write(&p, "id: x\nsteps:\n  - run:\n      id: a\n      spec: a.json\n  - sequence:\n      steps:\n        - run: { id: b, spec: b.json }\n").unwrap();
    let s = read_campaign(&p).unwrap();
    assert!(matches!(&s.steps[1], Step::Sequence(g) if matches!(&g.steps[0], Step::Run(r) if r.id == "b")));
    let yaml = lab.base.join("back.yaml");
    write_campaign(&yaml, &s).unwrap();
    assert!(fs::read_to_string(&yaml).unwrap().contains("- run:\n"), "no YAML tag: a step is a map");
    // two kinds in one step, or none: refused
    for text in ["id: x\nsteps:\n  - run: { id: a, spec: a.json }\n    sweep: { id: s, spec: a.json, design: factorial }\n", "id: x\nsteps:\n  - {}\n"] {
        fs::write(&p, text).unwrap();
        assert!(read_campaign(&p).unwrap_err().to_string().contains("a step is"), "{text}");
    }
}

#[test]
fn the_shipped_matrix_example_plans_its_240_trials() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let loaded = LoadedCampaign::load(&root.join("examples/v1.matrix.1o.campaign"), &root).unwrap();
    assert_eq!(loaded.plan.nodes.len(), 240);
    assert!(loaded.plan.findings.is_empty(), "every control the bench declares has its value: {:?}", loaded.plan.findings);
    let json = root.join("examples/matrix-bench.json");
    assert_eq!(experiments::read_spec(&json).unwrap().scientific_method.independent_variables.len(), 3);
}

#[test]
fn core_oneo_discovers_the_shipped_campaign_schema() {
    let workspace = core_1o::engine::Workspace::new(env!("CARGO_MANIFEST_DIR"));
    let schema = workspace.schema_for_file("campaign", "v1").expect("campaign schema");
    assert_eq!(schema["title"].as_str(), Some("experiments campaign v1"));
}

#[test]
fn a_sequence_and_its_flat_form_are_one_plan_with_one_digest() {
    let lab = lab();
    let nested = plan(&lab, "v1.nested.1o.campaign", "campaign {\n  id \"h\"\n  sequence {\n    run { id \"A\" spec \"a.json\" }\n    run { id \"B\" spec \"b.json\" }\n    run { id \"C\" spec \"c.json\" }\n  }\n}\n");
    let flat = plan(&lab, "flat.json", r#"{"id":"h","steps":[
        {"run":{"id":"A","spec":"a.json"}},
        {"run":{"id":"B","spec":"b.json","after":["A"],"from":"A"}},
        {"run":{"id":"C","spec":"c.json","after":["B"],"from":"B"}}]}"#);
    assert_eq!(nested.nodes, flat.nodes);
    assert_eq!(nested.digest, flat.digest);
    assert_ne!(nested.campaign_digest, flat.campaign_digest, "two documents, one plan");
    let n: Vec<(&str, Vec<&str>, Option<&str>, &str)> = nested.nodes.iter().map(|n| (n.id.as_str(), n.after.iter().map(String::as_str).collect(), n.from.as_deref(), n.workspace.as_str())).collect();
    assert_eq!(n, [("A", vec![], None, "shared"), ("B", vec!["A"], Some("A"), "shared"), ("C", vec!["B"], Some("B"), "shared")]);
    // `from` alone is an execution dependency too
    let from_only = plan(&lab, "from.json", r#"{"id":"h","steps":[
        {"run":{"id":"A","spec":"a.json"}},
        {"run":{"id":"B","spec":"b.json","from":"A"}},
        {"run":{"id":"C","spec":"c.json","from":"B"}}]}"#);
    assert_eq!(from_only.digest, flat.digest);
    // the plan is the same each time it is made
    assert_eq!(plan(&lab, "flat.json", &fs::read_to_string(lab.base.join("flat.json")).unwrap()).digest, flat.digest);
}

#[test]
fn fifty_ordered_experiments_are_one_evolving_lineage() {
    let lab = lab();
    let mut text = String::from("campaign {\n  id \"history\"\n  sequence {\n");
    for k in 1..=50 {
        let p = experiment(&lab.base, &lab.root, &format!("t{k:02}"), &[], &[], MutationMode::Bounded);
        text.push_str(&format!("    run {{ id \"t{k:02}\" spec \"{}\" }}\n", p.file_name().unwrap().to_str().unwrap()));
    }
    text.push_str("  }\n}\n");
    let p = plan(&lab, "v1.history.1o.campaign", &text);
    assert_eq!(p.nodes.len(), 50);
    assert_eq!(p.workspaces.len(), 1);
    for (i, n) in p.nodes.iter().enumerate() {
        assert_eq!(n.workspace, "shared");
        assert_eq!(n.from, (i > 0).then(|| format!("t{i:02}")));
        assert_eq!(n.index, i);
    }
    assert!(p.snapshots.is_empty(), "no isolated workspace: nothing to copy");
    assert_eq!(p.expected_parallelism(), (1, 1, 50));
}

#[test]
fn parallel_branches_share_a_source_and_never_a_workspace_and_fan_in_without_merge() {
    let lab = lab();
    let p = plan(&lab, "v1.fan.1o.campaign", "campaign {\n  id \"fan\"\n  policy { max_parallel 2 }\n  sequence {\n    run { id \"A\" spec \"a.json\" }\n    parallel {\n      id \"branches\"\n      run { id \"B\" spec \"b.json\" }\n      sequence {\n        run { id \"C1\" spec \"c.json\" }\n        run { id \"C2\" spec \"c.json\" }\n      }\n    }\n    run { id \"D\" spec \"d.json\" }\n  }\n}\n");
    let n = |id: &str| p.node(id).unwrap();
    assert_eq!((n("B").from.as_deref(), n("C1").from.as_deref()), (Some("A"), Some("A")), "one source");
    assert_eq!((n("B").workspace.as_str(), n("C1").workspace.as_str(), n("C2").workspace.as_str()), ("isolated:B", "isolated:C1", "isolated:C1"));
    assert_eq!((n("C1").strategy, n("C2").strategy), (WorkspaceStrategy::Isolated, WorkspaceStrategy::Continue));
    // D waits for every branch (fan-in) and starts from A's accepted state: no implicit merge
    assert_eq!(n("D").after, ["A", "B", "C2"]);
    assert_eq!((n("D").from.as_deref(), n("D").workspace.as_str()), (Some("A"), "shared"));
    assert_eq!(p.snapshots, ["A"]);
    assert_eq!(p.expected_parallelism(), (2, 2, 4));
}

#[test]
fn a_repeat_is_n_isolated_trials_of_one_treatment_from_one_source() {
    let lab = lab();
    let p = plan(&lab, "rep.json", r#"{"id":"rep","factors":[{"name":"implementation","levels":["old","new"]}],"steps":[
        {"run":{"id":"base","spec":"a.json"}},
        {"repeat":{"id":"again","times":3,"from":"base","run":{"id":"new-impl","spec":"b.json","set":[{"factor":"implementation","value":"new"}]}}}]}"#);
    let trials: Vec<&PlanNode> = p.nodes.iter().filter(|n| n.group.as_deref() == Some("again")).collect();
    assert_eq!(trials.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(), ["again.r01", "again.r02", "again.r03"]);
    for (k, t) in trials.iter().enumerate() {
        assert_eq!((t.replicate, t.treatment.as_deref(), t.from.as_deref(), t.strategy), (Some(k as u32 + 1), Some("new-impl"), Some("base"), WorkspaceStrategy::Isolated));
        assert_eq!(t.workspace, format!("isolated:{}", t.id), "each its own workspace");
        assert_eq!(t.assignment, [Assignment { factor: "implementation".into(), value: Level::Text("new".into()) }]);
    }
    let g = &p.groups[0];
    assert_eq!((g.kind.as_str(), g.repeats, g.nodes.len()), ("repeat", 3, 3));
}

fn matrix_plan(lab: &Lab) -> CampaignPlan {
    plan(lab, "v1.matrix.1o.campaign", MATRIX)
}

#[test]
fn a_factorial_sweep_expands_to_its_exact_cartesian_product_before_anything_runs() {
    let lab = lab();
    let p = matrix_plan(&lab);
    assert_eq!(p.nodes.len(), 2 * 4 * 3 * 10, "2 × 4 × 3 treatments × 10 repeats");
    let g = &p.groups[0];
    assert_eq!((g.design, g.treatments.len(), g.repeats), (Some(Design::Factorial), 24, 10));
    // every combination, once, first factor slowest
    let mut want = Vec::new();
    for m in ["scalar", "simd"] {
        for pop in ["1K", "10K", "100K", "1M"] {
            for geo in ["low", "medium", "high"] {
                want.push(format!("matcher={m},population={pop},geo_concentration={geo}"));
            }
        }
    }
    assert_eq!(g.treatments.iter().map(|t| t.name.clone()).collect::<Vec<_>>(), want);
    let first = &p.nodes[0];
    assert_eq!(first.id, "grid.t01.b01.r01");
    assert_eq!(p.nodes[9].id, "grid.t01.b01.r10");
    assert_eq!(p.nodes[10].id, "grid.t02.b01.r01");
    let ids: std::collections::BTreeSet<&str> = p.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids.len(), 240, "stable, distinct ids");
    for n in &p.nodes {
        // the full assignment, the controls, the block (never a factor), the replicate, the spec digest, the source
        assert_eq!(n.assignment.iter().map(|a| a.factor.as_str()).collect::<Vec<_>>(), ["matcher", "population", "geo_concentration"]);
        assert_eq!(n.controls.len(), 3);
        assert_eq!(n.blocks, [BlockAssignment { name: "machine".into(), level: Level::Text("lab-1".into()) }]);
        assert!(!n.assignment.iter().any(|a| a.factor == "machine"));
        assert!(n.replicate.is_some_and(|r| (1..=10).contains(&r)));
        assert_eq!(n.spec_digest, p.specs[0].digest);
        assert!(n.from.is_none() && n.strategy == WorkspaceStrategy::Isolated);
        assert_eq!(n.design, Some(Design::Factorial));
        let env: Vec<String> = n.bindings.iter().map(|v| format!("{}={}", v.name, v.value)).collect();
        assert!(env.contains(&format!("EXPERIMENT_TRIAL={}", n.id)));
        assert!(env.contains(&"EXPERIMENT_CONTROL_SEED=7".to_string()) && env.contains(&"EXPERIMENT_BLOCK_MACHINE=lab-1".to_string()), "{env:?}");
        for a in &n.assignment {
            assert!(env.contains(&format!("EXPERIMENT_FACTOR_{}={}", a.factor.to_ascii_uppercase(), a.value)));
        }
    }
    assert_eq!(p.snapshots, [START]);
    assert_eq!(p.expected_parallelism().0, 4, "max_parallel");
    // the same plan from the same files, every time
    assert_eq!(matrix_plan(&lab).digest, p.digest);
}

#[test]
fn the_matrix_as_explicit_treatments_is_the_same_trials_with_its_own_design() {
    let lab = lab();
    let factorial = matrix_plan(&lab);
    let mut treatments = String::new();
    for t in &factorial.groups[0].treatments {
        treatments.push_str(&format!("    treatment {{\n      name \"{}\"\n", t.name));
        for a in &t.assignment {
            treatments.push_str(&format!("      set {{ factor \"{}\" value \"{}\" }}\n", a.factor, a.value));
        }
        treatments.push_str("    }\n");
    }
    let text = MATRIX.replace("    design \"factorial\"\n    factor \"matcher\"\n    factor \"population\"\n    factor \"geo_concentration\"\n", &format!("    design \"explicit\"\n{treatments}"));
    let explicit = plan(&lab, "v1.explicit.1o.campaign", &text);
    assert_eq!(explicit.nodes.len(), factorial.nodes.len());
    for (e, f) in explicit.nodes.iter().zip(&factorial.nodes) {
        assert_eq!((&e.id, &e.assignment, &e.blocks, e.replicate, &e.treatment, &e.spec_digest, &e.from, &e.bindings), (&f.id, &f.assignment, &f.blocks, f.replicate, &f.treatment, &f.spec_digest, &f.from, &f.bindings));
        assert_eq!((e.design, f.design), (Some(Design::Explicit), Some(Design::Factorial)), "the design's identity is kept");
    }
}

#[test]
fn one_factor_at_a_time_moves_one_factor_from_its_reference_per_treatment() {
    let lab = lab();
    let text = MATRIX.replace("    design \"factorial\"\n", "    design \"one_factor_at_a_time\"\n    reference { factor \"matcher\" value \"scalar\" }\n    reference { factor \"population\" value \"1K\" }\n    reference { factor \"geo_concentration\" value \"low\" }\n").replace("    repeats 10\n", "    repeats 2\n");
    let p = plan(&lab, "v1.ofat.1o.campaign", &text);
    let g = &p.groups[0];
    // the reference, then 1 + 3 + 2 moves
    assert_eq!(g.treatments.len(), 1 + 1 + 3 + 2);
    let reference = &g.treatments[0];
    assert_eq!((reference.name.as_str(), reference.varied.as_deref()), ("reference", None));
    for t in &g.treatments[1..] {
        let moved: Vec<&Assignment> = t.assignment.iter().filter(|a| !reference.assignment.contains(a)).collect();
        assert_eq!(moved.len(), 1, "{}: one factor moved", t.name);
        assert_eq!(Some(moved[0].factor.as_str()), t.varied.as_deref());
    }
    assert_eq!(g.treatments[1].name, "matcher=simd");
    assert_eq!(p.nodes.len(), 7 * 2);
    assert!(p.nodes.iter().all(|n| n.design == Some(Design::OneFactorAtATime)));
    assert_eq!(p.nodes.iter().filter(|n| n.varied.as_deref() == Some("population")).count(), 3 * 2);
}

#[test]
fn blocks_are_recorded_apart_from_treatments_and_cross_the_design() {
    let lab = lab();
    let text = MATRIX.replace("    level \"lab-1\"\n", "    level \"lab-1\"\n    level \"lab-2\"\n").replace("    repeats 10\n", "    repeats 1\n");
    let p = plan(&lab, "v1.blocks.1o.campaign", &text);
    assert_eq!(p.nodes.len(), 24 * 2);
    assert_eq!(p.nodes[0].id, "grid.t01.b01.r01");
    assert_eq!(p.nodes[24].id, "grid.t01.b02.r01");
    for n in &p.nodes {
        assert_eq!(n.blocks.len(), 1);
        assert_eq!(n.assignment.len(), 3, "the block is not in the assignment");
    }
    // a block named as a factor is refused, and the other way round
    let e = refused(&lab, &serde_json::to_string(&serde_json::json!({"id":"x","blocks":[{"name":"machine","levels":["m"]}],"factors":[{"name":"matcher","levels":["a"],"scope":"campaign"}],"steps":[{"sweep":{"id":"s","spec":"a.json","design":"factorial","factors":["machine"]}}]})).unwrap());
    assert!(e.contains("it is a block"), "{e}");
    let e = refused(&lab, &serde_json::to_string(&serde_json::json!({"id":"x","factors":[{"name":"matcher","levels":["a"],"scope":"campaign"}],"steps":[{"sweep":{"id":"s","spec":"a.json","design":"factorial","factors":["matcher"],"blocks":["matcher"]}}]})).unwrap());
    assert!(e.contains("it is a factor"), "{e}");
}

#[test]
fn everything_is_refused_before_anything_runs() {
    let lab = lab();
    let cases: &[(&str, &str)] = &[
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"a.json"}},{"run":{"id":"a","spec":"b.json","workspace":"isolated"}}]}"#, "names more than one"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"nope.json"}}]}"#, "nope.json"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"a.json","after":["ghost"]}}]}"#, "ghost"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"a.json","after":["b"],"workspace":"isolated"}},{"run":{"id":"b","spec":"b.json","after":["a"],"workspace":"isolated"}}]}"#, "a cycle"),
        (r#"{"id":"x","steps":[{"parallel":{"id":"p","steps":[{"run":{"id":"a","spec":"a.json"}},{"run":{"id":"b","spec":"b.json"}}]}},{"run":{"id":"c","spec":"c.json","from":"p","workspace":"isolated"}}]}"#, "end in many states"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"a.json"}},{"run":{"id":"b","spec":"b.json"}}]}"#, "both evolve shared"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"a.json"}},{"run":{"id":"b","spec":"b.json","from":"a"}},{"run":{"id":"c","spec":"c.json","from":"a"}}]}"#, "both evolve shared"),
        (r#"{"id":"x","steps":[{"parallel":{"workspace":"shared","steps":[{"run":{"id":"a","spec":"a.json"}}]}}]}"#, "cannot share"),
        (r#"{"id":"x","steps":[{"repeat":{"id":"r","times":0,"run":{"id":"a","spec":"a.json"}}}]}"#, "times 0"),
        (r#"{"id":"x","steps":[{"sweep":{"id":"s","spec":"a.json","design":"factorial","factors":["implementation"],"repeats":0}}],"factors":[{"name":"implementation","levels":["a"]}]}"#, "repeats 0"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"a.json","workspace":"isolated"}},{"run":{"id":"b","spec":"b.json","from":"a"}}]}"#, "no implicit merge"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"loose.json"}}]}"#, "unbounded"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"a.json","set":[{"factor":"implementaton","value":"old"}]}}],"factors":[{"name":"implementation","levels":["old"]}]}"#, "`implementaton` is no declared factor"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"a.json","set":[{"factor":"implementation","value":"old"},{"factor":"implementation","value":"new"}]}}],"factors":[{"name":"implementation","levels":["old","new"]}]}"#, "set twice"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"a.json","set":[{"factor":"threads","value":4}]}}],"factors":[{"name":"threads","levels":[1,4]}]}"#, "no independent variable"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"a.json","set":[{"factor":"implementation","value":"newest"}]}}],"factors":[{"name":"implementation","levels":["old","new"]}]}"#, "has no level newest"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"a.json","set":[{"factor":"implementation","value":1.0}]}}],"factors":[{"name":"implementation","levels":[1]}]}"#, "has no level 1.0"),
        (r#"{"id":"x","factors":[{"name":"implementation","levels":["a",1]}],"steps":[{"run":{"id":"a","spec":"a.json"}}]}"#, "not all one kind"),
        (r#"{"id":"x","factors":[{"name":"implementation","levels":[]}],"steps":[{"run":{"id":"a","spec":"a.json"}}]}"#, "has no levels"),
        (r#"{"id":"x","factors":[{"name":"implementation","levels":["a","a"]}],"steps":[{"run":{"id":"a","spec":"a.json"}}]}"#, "declared twice"),
        (r#"{"id":"x","factors":[{"name":"seed","levels":[1]}],"controls":[{"name":"seed","value":1}],"steps":[{"run":{"id":"a","spec":"a.json"}}]}"#, "a name is one variable"),
        (r#"{"id":"x","controls":[{"name":"bad-name","value":1}],"steps":[{"run":{"id":"a","spec":"a.json"}}]}"#, "control `bad-name`"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"a.json","blocks":[{"name":"machine","level":"m9"}]}}],"blocks":[{"name":"machine","levels":["m1"]}]}"#, "block `machine` has no level m9"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"a.json","blocks":[{"name":"rack","level":"r1"}]}}]}"#, "`rack` is no declared block"),
        (r#"{"id":"x","factors":[{"name":"implementation","levels":["a","b"]}],"steps":[{"sweep":{"id":"s","spec":"a.json","design":"one_factor_at_a_time","factors":["implementation"]}}]}"#, "no reference level"),
        (r#"{"id":"x","factors":[{"name":"implementation","levels":["a","b"]}],"steps":[{"sweep":{"id":"s","spec":"a.json","design":"factorial"}}]}"#, "at least one factor"),
        (r#"{"id":"x","factors":[{"name":"implementation","levels":["a","b"]}],"steps":[{"sweep":{"id":"s","spec":"a.json","design":"explicit"}}]}"#, "at least one treatment"),
        (r#"{"id":"x","factors":[{"name":"implementation","levels":["a","b"]}],"steps":[{"sweep":{"id":"s","spec":"a.json","design":"explicit","treatments":[{"name":"t1","set":[{"factor":"implementation","value":"a"}]},{"name":"t2","set":[{"factor":"implementation","value":"a"}]}]}}]}"#, "assign the same levels"),
        (r#"{"id":"x","factors":[{"name":"implementation","levels":["a","b"]}],"steps":[{"sweep":{"id":"s","spec":"a.json","design":"factorial","factors":["implementation"],"set":[{"factor":"implementation","value":"a"}]}}]}"#, "both moved by the design and held"),
        (r#"{"id":"x","factors":[{"name":"implementation","levels":["a","b"]},{"name":"n","scope":"campaign","levels":[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50]}],"steps":[{"sweep":{"id":"s","spec":"a.json","design":"factorial","factors":["implementation","n"],"repeats":1001}}]}"#, "more than 100000"),
        (r#"{"id":"x","policy":{"max_parallel":0},"steps":[{"run":{"id":"a","spec":"a.json"}}]}"#, "max_parallel 0"),
        (r#"{"id":"x","steps":[{"sequence":{"steps":[{"run":{"id":"a","spec":"a.json","after":["b"]}}]}},{"run":{"id":"b","spec":"b.json","workspace":"isolated"}}]}"#, "contradictory composition"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"a.json","workspace":"continue"}}]}"#, "needs `from`"),
        (r#"{"id":"x","steps":[{"run":{"id":"a","spec":"a.json"}},{"run":{"id":"b","spec":"b.json","from":"a","workspace":"continue"}}]}"#, "`continue` of a"),
        (r#"{"id":"x","exclude":["../out"],"steps":[{"run":{"id":"a","spec":"a.json"}}]}"#, "exclude `../out`"),
        (r#"{"id":"x/y","steps":[{"run":{"id":"a","spec":"a.json"}}]}"#, "campaign id"),
        (r#"{"id":"x","steps":[{"run":{"id":"campaign","spec":"a.json"}}]}"#, "campaign's start"),
        (r#"{"id":"x","steps":[]}"#, "no steps"),
    ];
    for (text, why) in cases {
        let e = refused(&lab, text);
        assert!(e.contains(why), "expected `{why}` in: {e}\n{text}");
    }
    // a malformed experiment file, and an experiment whose target is not the governed root
    fs::write(lab.base.join("broken.json"), "{ not json").unwrap();
    assert!(refused(&lab, r#"{"id":"x","steps":[{"run":{"id":"a","spec":"broken.json"}}]}"#).contains("broken.json"));
    let elsewhere = tempdir().unwrap();
    experiment(&lab.base, elsewhere.path(), "elsewhere", &[], &[], MutationMode::Bounded);
    assert!(refused(&lab, r#"{"id":"x","steps":[{"run":{"id":"a","spec":"elsewhere.json"}}]}"#).contains("is not the governed root"));
    // a command that names a binding's variable itself
    let mut s = experiments::read_spec(lab.base.join("a.json")).unwrap();
    s.protocol.prepare.push(experiments::CommandSpec { env: vec![experiments::EnvVar { name: "EXPERIMENT_TRIAL".into(), value: "mine".into() }], ..common::command("p", "true", &[]) });
    write_spec(lab.base.join("self.json"), &s).unwrap();
    assert!(refused(&lab, r#"{"id":"x","steps":[{"run":{"id":"a","spec":"self.json"}}]}"#).contains("names EXPERIMENT_TRIAL itself"));
}

#[test]
fn strict_refuses_what_it_cannot_establish_and_controlled_reports_it() {
    let lab = lab();
    let doc = |rigor: &str, controls: &str| format!(r#"{{"id":"x","rigor":"{rigor}","controls":[{controls}],"factors":[{{"name":"matcher","levels":["scalar"]}}],"steps":[{{"run":{{"id":"a","spec":"bench.json"}}}}]}}"#);
    // bench declares corpus, seed and demand_model: strict needs a value for each
    let e = refused(&lab, &doc("strict", r#"{"name":"seed","value":7}"#));
    assert!(e.contains("declares the control `corpus`") && e.contains("strict"), "{e}");
    let controlled = plan(&lab, "c.json", &doc("controlled", r#"{"name":"seed","value":7}"#));
    assert_eq!(controlled.findings.len(), 2, "{:?}", controlled.findings);
    assert!(controlled.findings[0].contains("not controlled by the campaign"));
    let all = r#"{"name":"seed","value":7},{"name":"corpus","value":"v3"},{"name":"demand_model","value":"poisson"}"#;
    assert!(plan(&lab, "s.json", &doc("strict", all)).findings.is_empty());
    // strict refuses an unbounded experiment anywhere, even isolated
    let e = refused(&lab, r#"{"id":"x","rigor":"strict","steps":[{"run":{"id":"a","spec":"loose.json","workspace":"isolated"}}]}"#);
    assert!(e.contains("strict allows source changes only through bounded"), "{e}");
    // an unbounded experiment isolated is planned under controlled
    let p = plan(&lab, "u.json", r#"{"id":"x","steps":[{"run":{"id":"a","spec":"loose.json","workspace":"isolated"}}]}"#);
    assert_eq!(p.nodes[0].mutation, MutationMode::Unbounded);
}

#[test]
fn a_binding_reaches_every_command_as_typed_environment_and_nothing_else_changes() {
    let lab = lab();
    let loaded = load(&lab, "b.json", r#"{"id":"bind","factors":[{"name":"implementation","levels":["old","new"]},{"name":"ratio","scope":"campaign","levels":[0.5,1.0]}],"controls":[{"name":"seed","value":7}],"blocks":[{"name":"machine","levels":["m1"]}],
        "steps":[{"run":{"id":"one","spec":"a.json","set":[{"factor":"ratio","value":1.0},{"factor":"implementation","value":"new"}],"blocks":[{"name":"machine","level":"m1"}]}}]}"#).unwrap();
    let node = loaded.plan.node("one").unwrap();
    let ws = lab.root.join("elsewhere");
    let bound = loaded.bound("one", &ws).unwrap();
    let want = ["EXPERIMENT_CAMPAIGN=bind", "EXPERIMENT_TRIAL=one", "EXPERIMENT_FACTOR_IMPLEMENTATION=new", "EXPERIMENT_FACTOR_RATIO=1.0", "EXPERIMENT_CONTROL_SEED=7", "EXPERIMENT_BLOCK_MACHINE=m1"];
    let got: Vec<String> = node.bindings.iter().map(|v| format!("{}={}", v.name, v.value)).collect();
    assert_eq!(got, want, "in declaration order, one text form per level");
    let measured = &bound.measurements.measurements[0].command;
    assert_eq!(measured.env, node.bindings);
    assert_eq!(measured.args, ["1"], "no word is templated");
    assert_eq!(bound.target.root, ws);
    let declared = experiments::read_spec(lab.base.join("a.json")).unwrap();
    assert_eq!((bound.id, bound.scientific_method), (declared.id, declared.scientific_method));
    // a bound command's program gets exactly that environment (the standalone host runs it)
    let dir = tempdir().unwrap();
    let mut s = common::base_spec(dir.path());
    s.measurements.measurements.push(experiments::MeasurementSpec { phase: experiments::MeasurementPhase::Candidate, condition: "candidate".into(), name: "ratio".into(), unit: None, command: common::command("ratio", "printenv", &["EXPERIMENT_FACTOR_RATIO"]), parser: experiments::MeasurementParser::Number });
    let bound = Binding::of(node).apply(&s, dir.path(), dir.path()).unwrap();
    let (result, _) = experiments::ExperimentRunner::new(dir.path().join("storage")).run(&bound).unwrap();
    let m = result.observations.iter().find(|o| o.measurement.name == "ratio").unwrap();
    assert_eq!(m.measurement.value, scientific_method::Scalar::Number(1.0));
    assert_eq!(result.commands.iter().find(|c| c.label == "ratio").unwrap().env, node.bindings, "the record keeps what it was given");
    // the experiment moved since the plan: refused
    let mut moved = experiments::read_spec(lab.base.join("a.json")).unwrap();
    moved.description = Some("edited".into());
    write_spec(lab.base.join("a.json"), &moved).unwrap();
    assert!(loaded.bound("one", &ws).unwrap_err().to_string().contains("the plan read"));
}

// ---- the runner ----

fn concluded(p: PromotionDecision, latency: f64) -> Execution {
    Execution::Concluded {
        promotion: p,
        outcome: Outcome::Supported,
        result: FileRef { path: "r.json".into(), digest: "sha256:r".into() },
        run_id: "run".into(),
        observations: Observations::new().with(Observation::new("candidate", Measurement::new("latency", latency).with_unit("ms"))),
    }
}

/// A record the way a host makes it from an execution and the runner's decision.
fn record(r: &CampaignRunner, id: &str, e: &Execution, undeclared: &[String]) -> NodeRecord {
    let node = r.plan().node(id).unwrap().clone();
    let d = CampaignRunner::decide(&node, e, undeclared);
    let mut rec = NodeRecord::new(id, d.status);
    rec.reason = d.reason;
    rec.findings = d.findings;
    let s = StateIdentity { digest: format!("sha256:{id}"), files: 1 };
    rec.start = r.expected_start(&node);
    if let Execution::Concluded { promotion, outcome, result, observations, .. } = e {
        rec.promotion = Some(*promotion);
        rec.outcome = Some(*outcome);
        rec.result = Some(result.clone());
        rec.observations = Some(observations.clone());
    }
    rec.state = Some(if d.accept { StateAction::Accepted } else { StateAction::Restored });
    rec.accepted = if d.accept { Some(s) } else { rec.start.clone() };
    rec
}

fn drive(r: &mut CampaignRunner, outcome: impl Fn(&str) -> (Execution, Vec<String>)) {
    r.set_start(StateIdentity { digest: "sha256:start".into(), files: 1 });
    loop {
        let ready = r.ready();
        if ready.is_empty() {
            break;
        }
        for n in ready {
            r.start(&n.id);
            let (e, u) = outcome(&n.id);
            let rec = record(r, &n.id, &e, &u);
            r.finish(rec);
        }
    }
    r.close();
}

#[test]
fn promotion_decides_the_state_and_the_rigor_decides_an_undeclared_change() {
    let lab = lab();
    let p = plan(&lab, "s.json", r#"{"id":"s","steps":[{"sequence":{"steps":[{"run":{"id":"A","spec":"a.json"}},{"run":{"id":"B","spec":"b.json"}},{"run":{"id":"C","spec":"c.json"}}]}}]}"#);
    let mut r = CampaignRunner::new(p.clone(), "run-1");
    drive(&mut r, |id| (concluded(if id == "B" { PromotionDecision::DoNotPromote } else { PromotionDecision::Promote }, 1.0), Vec::new()));
    let res = r.result(HostFacts::default());
    let n = |id: &str| res.nodes.iter().find(|n| n.plan.id == id).unwrap().run.clone();
    assert_eq!((n("A").state, n("B").state, n("C").state), (Some(StateAction::Accepted), Some(StateAction::Restored), Some(StateAction::Accepted)));
    // C starts from B's accepted state, which is A's: B's candidate was put back
    assert_eq!(n("B").accepted, n("A").accepted);
    assert_eq!(n("C").start, n("A").accepted);
    assert_eq!(n("B").status, NodeStatus::Completed, "do-not-promote is no failure");
    assert_eq!(res.status, CampaignStatus::Findings);
    assert_eq!((res.counts.promote, res.counts.do_not_promote, res.counts.restored), (2, 1, 1));
    // undecided: restored by default, accepted when the policy says so
    let undecided = concluded(PromotionDecision::Undecided, 1.0);
    let node = p.node("A").unwrap().clone();
    assert!(!CampaignRunner::decide(&node, &undecided, &[]).accept);
    let mut accepting = node.clone();
    accepting.policy.on_undecided = OnUndecided::Accept;
    assert!(CampaignRunner::decide(&accepting, &undecided, &[]).accept);
    // an undeclared change: recorded (exploratory), not accepted (controlled), refused (strict)
    let promote = concluded(PromotionDecision::Promote, 1.0);
    let u = vec!["~src/extra.rs".to_string()];
    let mut rigor = node.clone();
    rigor.rigor = Rigor::Exploratory;
    let d = CampaignRunner::decide(&rigor, &promote, &u);
    assert!(d.accept && d.findings[0].contains("undeclared change"));
    rigor.rigor = Rigor::Controlled;
    let d = CampaignRunner::decide(&rigor, &promote, &u);
    assert!(!d.accept && d.status == NodeStatus::Completed);
    rigor.rigor = Rigor::Strict;
    let d = CampaignRunner::decide(&rigor, &promote, &u);
    assert_eq!((d.status, d.accept), (NodeStatus::Refused, false));
    assert!(d.reason.unwrap().contains("strict"));
}

#[test]
fn a_refusal_stops_the_campaign_and_skips_what_waits_for_it_by_default() {
    let lab = lab();
    let doc = |policy: &str| format!(r#"{{"id":"s","policy":{policy},"steps":[{{"sequence":{{"steps":[{{"run":{{"id":"A","spec":"a.json"}}}},{{"run":{{"id":"B","spec":"b.json"}}}},{{"run":{{"id":"C","spec":"c.json"}}}}]}}}},{{"run":{{"id":"D","spec":"d.json","workspace":"isolated"}}}}]}}"#);
    let refuse_b = |id: &str| (if id == "B" { Execution::Refused("preflight refused".into()) } else { concluded(PromotionDecision::Promote, 1.0) }, Vec::new());
    let mut r = CampaignRunner::new(plan(&lab, "stop.json", &doc("{}")), "run");
    drive(&mut r, refuse_b);
    let res = r.result(HostFacts::default());
    let st = |id: &str| res.nodes.iter().find(|n| n.plan.id == id).unwrap().run.clone();
    assert_eq!(st("B").status, NodeStatus::Refused);
    assert_eq!(st("C").status, NodeStatus::Skipped);
    assert!(res.stopped.as_deref().unwrap().contains("on_refusal stop"));
    assert_eq!(res.status, CampaignStatus::Refused);
    // on_refusal continue: what does not wait for B still runs, what waits for it is skipped with why
    let mut r = CampaignRunner::new(plan(&lab, "go.json", &doc(r#"{"on_refusal":"continue"}"#)), "run");
    drive(&mut r, refuse_b);
    let res = r.result(HostFacts::default());
    let st = |id: &str| res.nodes.iter().find(|n| n.plan.id == id).unwrap().run.clone();
    assert_eq!((st("C").status, st("D").status), (NodeStatus::Skipped, NodeStatus::Completed));
    assert!(st("C").reason.unwrap().contains("waits for B, which was refused"));
    // on_findings stop: a do-not-promote stops it, as findings
    let mut r = CampaignRunner::new(plan(&lab, "f.json", &doc(r#"{"on_findings":"stop"}"#)), "run");
    drive(&mut r, |id| (concluded(if id == "A" { PromotionDecision::DoNotPromote } else { PromotionDecision::Promote }, 1.0), Vec::new()));
    let res = r.result(HostFacts::default());
    assert_eq!(res.status, CampaignStatus::Findings);
    assert!(res.stopped.unwrap().contains("on_findings stop"));
}

#[test]
fn a_resume_keeps_completed_nodes_and_refuses_a_moved_plan() {
    let lab = lab();
    let text = r#"{"id":"s","factors":[{"name":"implementation","levels":["old","new"]}],"steps":[{"sequence":{"steps":[{"run":{"id":"A","spec":"a.json"}},{"run":{"id":"B","spec":"b.json"}},{"run":{"id":"C","spec":"c.json"}}]}}]}"#;
    let p = plan(&lab, "r.json", text);
    let mut r = CampaignRunner::new(p.clone(), "run-1");
    r.set_start(StateIdentity { digest: "sha256:start".into(), files: 1 });
    for n in r.ready() {
        r.start(&n.id);
        let rec = record(&r, &n.id, &concluded(PromotionDecision::Promote, 1.0), &[]);
        r.finish(rec);
    }
    // B was running when the process stopped
    let b = r.ready().remove(0);
    r.start(&b.id);
    let kept = r.progress().clone();
    let resumed = CampaignRunner::resume(p.clone(), kept.clone()).unwrap();
    assert_eq!(resumed.record("A").unwrap().result, Some(FileRef { path: "r.json".into(), digest: "sha256:r".into() }), "A's result is kept");
    let mut resumed = resumed;
    assert_eq!(resumed.ready().iter().map(|n| n.id.as_str()).collect::<Vec<_>>(), ["B"], "B runs again, A does not");
    // anything the run was made from moved: refused, with why
    let mut moved = experiments::read_spec(lab.base.join("c.json")).unwrap();
    moved.description = Some("edited".into());
    write_spec(lab.base.join("c.json"), &moved).unwrap();
    let e = CampaignRunner::resume(plan(&lab, "r.json", text), kept.clone()).unwrap_err().to_string();
    assert!(e.contains("experiment c.json is sha256:") && e.contains("--again"), "{e}");
    let e = CampaignRunner::resume(plan(&lab, "r2.json", &text.replace("\"new\"", "\"newer\"")), kept).unwrap_err().to_string();
    assert!(e.contains("factors, controls, blocks"), "{e}");
}

#[test]
fn group_summaries_count_and_describe_their_trials_and_claim_nothing_more() {
    let lab = lab();
    let p = plan(&lab, "g.json", r#"{"id":"g","factors":[{"name":"implementation","levels":["old","new"]}],"blocks":[{"name":"machine","levels":["m1","m2"]}],"steps":[{"sweep":{"id":"grid","spec":"a.json","design":"factorial","factors":["implementation"],"repeats":4,"blocks":["machine"]}}],"policy":{"max_parallel":4}}"#);
    assert_eq!(p.nodes.len(), 2 * 2 * 4);
    let mut r = CampaignRunner::new(p, "run");
    // latency = replicate (1..4) for old, 10 × replicate for new; one new trial in m2 fails
    drive(&mut r, |id| {
        let rep: f64 = id.rsplit(".r").next().unwrap().parse().unwrap();
        if id == "grid.t02.b02.r04" {
            return (Execution::Failed("measurement did not parse".into()), Vec::new());
        }
        let new = id.contains(".t02.");
        (concluded(if new { PromotionDecision::Promote } else { PromotionDecision::DoNotPromote }, if new { 10.0 * rep } else { rep }), Vec::new())
    });
    let res = r.result(HostFacts::default());
    assert_eq!(res.summaries.len(), 4, "treatment × block");
    let old_m1 = &res.summaries[0];
    assert_eq!((old_m1.treatment.as_str(), old_m1.requested, old_m1.completed), ("implementation=old", 4, 4));
    assert_eq!(old_m1.blocks[0].level, Level::Text("m1".into()));
    let m = &old_m1.measurements[0];
    assert_eq!((m.n, m.mean, m.median, m.min, m.max), (4, Some(2.5), Some(2.5), Some(1.0), Some(4.0)));
    assert!((m.stddev.unwrap() - 1.2909944487358056).abs() < 1e-12, "sample standard deviation");
    assert_eq!(old_m1.promotions.get("do_not_promote"), Some(&4));
    assert_eq!(old_m1.trials.iter().map(|t| t.replicate.unwrap()).collect::<Vec<_>>(), [1, 2, 3, 4]);
    let new_m2 = &res.summaries[3];
    assert_eq!((new_m2.requested, new_m2.completed, new_m2.measurements[0].n), (4, 3, 3), "the failed trial is counted, not measured");
    // the marginals: by each factor level and each block level
    let by: Vec<(String, String, usize)> = res.marginals.iter().map(|m| (m.by.clone(), format!("{}={}", m.name, m.level), m.completed)).collect();
    assert_eq!(by, [("factor".into(), "implementation=old".into(), 8), ("factor".into(), "implementation=new".into(), 7), ("block".into(), "machine=m1".into(), 8), ("block".into(), "machine=m2".into(), 7)]);
    let json = serde_json::to_string(&res).unwrap();
    assert!(!json.contains("significan") && !json.contains("p_value"), "no significance is claimed");
    // the result round-trips through its formats; 1o is refused
    let dir = tempdir().unwrap();
    for name in ["result.json", "result.toml", "result.yaml"] {
        let f = dir.path().join(name);
        experiments::write_campaign_result(&f, &res).unwrap();
        assert_eq!(experiments::read_campaign_result(&f).unwrap(), res, "{name}");
    }
    assert!(experiments::write_campaign_result(dir.path().join("v1.r.1o.experiment"), &res).is_err());
}
