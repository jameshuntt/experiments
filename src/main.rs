use experiments::{ExperimentRunner, PromotionDecision, read_spec};
use std::{env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    match real_main() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("experiments: {error}");
            ExitCode::from(1)
        }
    }
}

fn real_main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let command = args
        .next()
        .ok_or("usage: experiments run <experiment.1o> [storage-base]")?;
    if command != std::ffi::OsStr::new("run") {
        return Err("usage: experiments run <experiment.1o> [storage-base]".into());
    }
    let spec_path = PathBuf::from(args.next().ok_or("missing experiment file")?);
    let storage = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".experiment-storage"));
    if args.next().is_some() {
        return Err("too many arguments".into());
    }

    let spec = read_spec(&spec_path)?;
    let runner = ExperimentRunner::new(storage);
    let (result, path) = runner.run(&spec)?;
    println!("RESULT={}", path.display());
    println!(
        "SCIENTIFIC_OUTCOME={:?}",
        result.evaluation.conclusion.outcome
    );
    println!("PROMOTION={:?}", result.promotion);
    Ok(match result.promotion {
        PromotionDecision::Promote => ExitCode::SUCCESS,
        PromotionDecision::DoNotPromote => ExitCode::from(2),
        PromotionDecision::Undecided => ExitCode::from(3),
    })
}
