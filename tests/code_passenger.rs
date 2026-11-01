mod common;

#[cfg(feature = "code-passenger")]
mod enabled {
    use super::common;
    use experiments::{
        ExperimentRunner, MutationBackend, MutationChange, MutationMode, MutationPlan,
        PromotionPolicy,
    };
    use serde_json::Value;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn bounded_experiment_applies_a_real_code_passenger_transform_and_keeps_its_record_as_evidence() {
        let target = tempdir().expect("target");
        let storage = tempdir().expect("storage");
        common::write_fixture_crate(target.path());
        common::write_make_struct_change(
            &target.path().join("candidate.toml"),
            "candidate",
            "Candidate",
            "u64",
        );

        let mut spec = common::base_spec(target.path());
        spec.id = "code-passenger-success".into();
        spec.mutation = MutationPlan {
            mode: MutationMode::Bounded,
            backend: MutationBackend::CodePassenger,
            changes: vec![MutationChange {
                label: "add Candidate".into(),
                spec: "candidate.toml".into(),
            }],
        };
        spec.promotion = PromotionPolicy {
            require_supported: false,
            require_verification: false,
        };

        let (result, _) = ExperimentRunner::new(storage.path())
            .run(&spec)
            .expect("Code Passenger transform succeeds");

        let source = fs::read_to_string(target.path().join("src/lib.rs")).expect("source");
        assert!(source.contains("pub struct Candidate"), "{source}");
        assert!(target.path().join(".passenger").is_dir());
        assert_eq!(result.mutations.len(), 1);
        assert_eq!(result.evidence.len(), 1);
        let mutation = &result.mutations[0];
        assert_eq!(mutation.backend, MutationBackend::CodePassenger);
        assert_eq!(mutation.status, 0);
        assert!(mutation.display.contains("code-passenger") && mutation.display.contains("transform"));
        let record = mutation.transform_record.as_ref().expect("transform record");
        assert!(record.is_file());
        assert_eq!(result.evidence[0].kind, "code_passenger_transform");
        assert_eq!(&result.evidence[0].path, record);

        let value: Value = serde_json::from_str(&fs::read_to_string(record).expect("record text"))
            .expect("record json");
        assert_eq!(value["schema"].as_str(), Some("custodian.transform.v1"));
        assert_eq!(value["operation"].as_str(), Some("make_struct"));
        assert_eq!(value["target"].as_str(), Some("crate:fixture"));
        assert_eq!(value["written"].as_bool(), Some(true));
        let checks = value["checks"].as_array().expect("checks");
        assert!(!checks.is_empty(), "Code Passenger must actually run its cargo proof");
        assert!(checks.iter().all(|check| check["ok"].as_bool() == Some(true)));
        assert!(value["checkpoint_before"].is_string());
        assert!(value["checkpoint_after"].is_string());
    }

    #[test]
    fn failed_code_passenger_proof_leaves_the_specimen_unchanged() {
        let target = tempdir().expect("target");
        let storage = tempdir().expect("storage");
        common::write_fixture_crate(target.path());
        common::write_make_struct_change(
            &target.path().join("broken.toml"),
            "broken",
            "Broken",
            "DefinitelyMissingType",
        );
        let before = fs::read_to_string(target.path().join("src/lib.rs")).expect("before");

        let mut spec = common::base_spec(target.path());
        spec.id = "code-passenger-failure".into();
        spec.mutation = MutationPlan {
            mode: MutationMode::Bounded,
            backend: MutationBackend::CodePassenger,
            changes: vec![MutationChange {
                label: "broken proof".into(),
                spec: "broken.toml".into(),
            }],
        };

        let error = ExperimentRunner::new(storage.path())
            .run(&spec)
            .expect_err("cargo proof must fail");
        assert!(error.to_string().contains("Code Passenger mutation"), "{error}");
        let after = fs::read_to_string(target.path().join("src/lib.rs")).expect("after");
        assert_eq!(after, before, "failed proof must not write source");
        assert!(!after.contains("struct Broken"));
    }
}
