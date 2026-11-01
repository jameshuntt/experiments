use crate::{
    ChangeRequest, Evidence, ExperimentHost, ExperimentPaths, ExperimentSpec, MutationMode,
    MutationRecord,
};
use std::path::{Path, PathBuf};

pub(crate) fn validate(spec: &ExperimentSpec) -> Result<(), String> {
    if spec.mutation.mode == MutationMode::Bounded && !spec.protocol.intervention.is_empty() {
        return Err("bounded mutation mode refuses protocol.intervention commands; declare source changes under mutation.changes so the backend can prove and record them".into());
    }
    Ok(())
}

/// Each declared change, as a [`ChangeRequest`] the host makes, in order.
pub(crate) fn run(
    spec: &ExperimentSpec,
    paths: &ExperimentPaths,
    run_id: &str,
    host: &mut dyn ExperimentHost,
) -> Result<(Vec<MutationRecord>, Vec<Evidence>), String> {
    validate(spec)?;
    let root = absolute(&spec.target.root).map_err(|e| e.to_string())?;
    let mut records = Vec::new();
    let mut evidence = Vec::new();
    for (index, change) in spec.mutation.changes.iter().enumerate() {
        let request = ChangeRequest {
            index,
            label: change.label.clone(),
            backend: spec.mutation.backend,
            root: root.clone(),
            spec: if change.spec.is_absolute() {
                change.spec.clone()
            } else {
                root.join(&change.spec)
            },
            record: paths
                .evidence(run_id)
                .join(format!("mutation-{index:03}-code-passenger.json")),
        };
        let (record, items) = host.apply_change(&request)?;
        evidence.extend(items);
        records.push(record);
    }
    Ok((records, evidence))
}

/// The standalone host's change: Code Passenger's declared transform line, in this process.
#[cfg(feature = "code-passenger")]
pub(crate) fn code_passenger_change(
    change: &ChangeRequest,
) -> Result<(MutationRecord, Vec<Evidence>), String> {
    use code_passenger::lines::CodePassenger;
    use xccute_contract::CompositeShellCommand;

    let (root, change_spec, record_path) = (&change.root, change.spec.clone(), change.record.clone());
    let label = change.label.as_str();

    // Build the Code Passenger line from its own Simple Impl/XCCUTE declaration.
    // No hand-built process command and no shell string is introduced here.
    let line = CodePassenger::transform()
        .root(root.as_os_str().to_os_string())
        .spec(change_spec.as_os_str().to_os_string())
        .write()
        .out(record_path.as_os_str().to_os_string());
    let display = line.build_display();
    let argv = line.argv();

    // `run_argv` dispatches the typed line in-process. The executable argument
    // is used by Code Passenger's agent subcommand; transform does not spawn it.
    let outcome = code_passenger::lines::run_argv(&argv, Path::new("code-passenger"));
    let record = MutationRecord {
        label: label.to_owned(),
        backend: change.backend,
        spec: change_spec,
        display,
        status: outcome.code,
        status_word: outcome.word,
        stdout: outcome.stdout,
        stderr: outcome.stderr,
        transform_record: record_path.exists().then(|| record_path.clone()),
    };

    if record.status != 0 {
        return Err(format!(
            "Code Passenger mutation `{}` stopped with {} ({}): {}{}",
            record.label,
            record.status,
            record.status_word,
            record.stdout.trim(),
            if record.stderr.trim().is_empty() {
                String::new()
            } else {
                format!(" · {}", record.stderr.trim())
            }
        ));
    }

    let evidence = record
        .transform_record
        .iter()
        .map(|path| Evidence {
            kind: "code_passenger_transform".into(),
            name: label.to_owned(),
            path: path.clone(),
        })
        .collect();
    Ok((record, evidence))
}

#[cfg(not(feature = "code-passenger"))]
pub(crate) fn code_passenger_change(
    _change: &ChangeRequest,
) -> Result<(MutationRecord, Vec<Evidence>), String> {
    Err("experiments was built without the `code-passenger` feature".into())
}

fn absolute(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandSpec, MutationBackend, MutationChange, MutationPlan, Protocol};

    #[test]
    fn bounded_refuses_raw_intervention_commands() {
        let mut spec = crate::codec_test_spec();
        spec.mutation.mode = MutationMode::Bounded;
        spec.protocol.intervention.push(CommandSpec {
            label: "raw edit".into(),
            program: "touch".into(),
            args: vec!["src/lib.rs".into()],
            cwd: None,
        });
        assert!(validate(&spec).unwrap_err().contains("bounded mutation mode"));
    }

    #[cfg(feature = "code-passenger")]
    #[test]
    fn code_passenger_transform_is_built_from_its_declared_line() {
        use code_passenger::lines::CodePassenger;
        use xccute_contract::CompositeShellCommand;
        let argv = CodePassenger::transform()
            .root("/workspace")
            .spec("change.toml")
            .write()
            .out("record.json")
            .argv();
        let words: Vec<String> = argv.iter().map(|w| w.to_string_lossy().into_owned()).collect();
        assert!(words.contains(&"transform".into()));
        assert!(words.contains(&"--spec".into()));
        assert!(words.contains(&"change.toml".into()));
        assert!(words.contains(&"--write".into()));
    }

    #[test]
    fn unbounded_can_mix_raw_and_structured_changes() {
        let mut spec = crate::codec_test_spec();
        spec.mutation = MutationPlan {
            mode: MutationMode::Unbounded,
            backend: MutationBackend::CodePassenger,
            changes: vec![MutationChange { label: "structured".into(), spec: "change.toml".into() }],
        };
        spec.protocol = Protocol::default();
        assert!(validate(&spec).is_ok());
    }
}
