mod common;

#[cfg(not(feature = "code-passenger"))]
#[test]
fn a_declared_code_passenger_change_is_explicitly_refused_when_the_feature_is_absent() {
    use experiments::{
        ExperimentRunner, MutationBackend, MutationChange, MutationMode, MutationPlan,
    };
    use tempfile::tempdir;

    let target = tempdir().expect("target");
    let storage = tempdir().expect("storage");
    let mut spec = common::base_spec(target.path());
    spec.mutation = MutationPlan {
        mode: MutationMode::Bounded,
        backend: MutationBackend::CodePassenger,
        changes: vec![MutationChange {
            label: "change".into(),
            spec: "change.toml".into(),
        }],
    };
    let error = ExperimentRunner::new(storage.path())
        .run(&spec)
        .expect_err("feature-disabled backend must refuse");
    assert!(
        error
            .to_string()
            .contains("without the `code-passenger` feature"),
        "{error}"
    );
}

#[cfg(not(feature = "differences"))]
#[test]
fn a_differences_request_is_explicitly_refused_when_the_feature_is_absent() {
    use experiments::{DifferencesRequest, ExperimentRunner};
    use tempfile::tempdir;

    let target = tempdir().expect("target");
    let storage = tempdir().expect("storage");
    let mut spec = common::base_spec(target.path());
    spec.differences.push(DifferencesRequest {
        line: vec!["tree".into(), "a".into(), "b".into()],
    });
    let error = ExperimentRunner::new(storage.path())
        .run(&spec)
        .expect_err("feature-disabled differences must refuse");
    assert!(
        error.to_string().contains("without the `differences` feature"),
        "{error}"
    );
}
