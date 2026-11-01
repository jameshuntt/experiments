use experiments::read_spec;
use std::path::PathBuf;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples").join(name)
}

#[test]
fn all_four_formats_decode_to_the_same_experiment() {
    let oneo = read_spec(example("v1.minimal-latency.1o.experiment")).expect("1o");
    let json = read_spec(example("minimal-latency.json")).expect("json");
    let toml = read_spec(example("minimal-latency.toml")).expect("toml");
    let yaml = read_spec(example("minimal-latency.yaml")).expect("yaml");
    assert_eq!(oneo, json);
    assert_eq!(json, toml);
    assert_eq!(toml, yaml);
}

#[test]
fn core_oneo_discovers_the_shipped_experiment_schema() {
    let workspace = core_1o::engine::Workspace::new(env!("CARGO_MANIFEST_DIR"));
    assert!(workspace.schema_for_file("experiment", "v1").is_some());
}
