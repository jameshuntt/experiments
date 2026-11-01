use crate::{Evidence, ExperimentPaths, ExperimentSpec, MutationBackend, MutationMode, MutationRecord};
use std::path::{Path, PathBuf};

pub(crate) fn validate(spec: &ExperimentSpec) -> Result<(), String> {
    if spec.mutation.mode == MutationMode::Bounded && !spec.protocol.intervention.is_empty() {
        return Err("bounded mutation mode refuses protocol.intervention commands; declare source changes under mutation.changes so the backend can prove and record them".into());
    }
    Ok(())
}

pub(crate) fn run(
    spec: &ExperimentSpec,
    paths: &ExperimentPaths,
    run_id: &str,
) -> Result<(Vec<MutationRecord>, Vec<Evidence>), String> {
    validate(spec)?;
    let mut records = Vec::new();
    let mut evidence = Vec::new();
    for (index, change) in spec.mutation.changes.iter().enumerate() {
        match spec.mutation.backend {
            MutationBackend::CodePassenger => {
                let (record, item) = code_passenger_change(
                    &spec.target.root,
                    paths,
                    run_id,
                    index,
                    &change.label,
                    &change.spec,
                )?;
                if let Some(item) = item {
                    evidence.push(item);
                }
                records.push(record);
            }
        }
    }
    Ok((records, evidence))
}

#[cfg(feature = "code-passenger")]
fn code_passenger_change(
    root: &Path,
    paths: &ExperimentPaths,
    run_id: &str,
    index: usize,
    label: &str,
    change_spec: &Path,
) -> Result<(MutationRecord, Option<Evidence>), String> {
    use code_passenger::lines::CodePassenger;
    use xccute_contract::CompositeShellCommand;

    let root = absolute(root).map_err(|e| e.to_string())?;
    let change_spec = if change_spec.is_absolute() {
        change_spec.to_path_buf()
    } else {
        root.join(change_spec)
    };
    let record_path = paths
        .evidence(run_id)
        .join(format!("mutation-{index:03}-code-passenger.json"));

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
        backend: MutationBackend::CodePassenger,
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

    let evidence = record.transform_record.as_ref().map(|path| Evidence {
        kind: "code_passenger_transform".into(),
        name: label.to_owned(),
        path: path.clone(),
    });
    Ok((record, evidence))
}

#[cfg(not(feature = "code-passenger"))]
fn code_passenger_change(
    _root: &Path,
    _paths: &ExperimentPaths,
    _run_id: &str,
    _index: usize,
    _label: &str,
    _change_spec: &Path,
) -> Result<(MutationRecord, Option<Evidence>), String> {
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
    use crate::{CommandSpec, MutationChange, MutationPlan, Protocol};

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
