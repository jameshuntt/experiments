use crate::surface::{read, usage_lines, Line};
use crate::{ExperimentRunner, PromotionDecision};
use std::{ffi::OsString, path::PathBuf};

pub fn usage() -> String {
    let mut out = String::from("experiments: declarative reproducible software experiments\n\n");
    for help in usage_lines() {
        out.push_str(&format!("  {}\n", help.words));
    }
    out.push_str(
        "\nexit status: 0 promote, 1 refused/error, 2 do not promote, 3 inconclusive\n",
    );
    out
}

pub fn main_argv(args: &[OsString]) -> i32 {
    if args.is_empty()
        || (args.len() == 1 && (args[0] == "--help" || args[0] == "-h"))
    {
        print!("{}", usage());
        return if args.is_empty() { 1 } else { 0 };
    }

    let line = match read(args) {
        Ok(line) => line,
        Err(error) => {
            eprintln!("experiments: refused: {error}");
            return 1;
        }
    };

    let code = match &line {
        Line::Run(run) => {
            let spec_path = PathBuf::from(&run.spec);
            let storage = run
                .storage
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(".experiment-storage"));

            let spec = match crate::read_spec(&spec_path) {
                Ok(spec) => spec,
                Err(error) => {
                    eprintln!("experiments: {error}");
                    return 1;
                }
            };

            let runner = ExperimentRunner::new(storage);
            match runner.run(&spec) {
                Ok((result, path)) => {
                    println!("RESULT={}", path.display());
                    println!(
                        "SCIENTIFIC_OUTCOME={:?}",
                        result.evaluation.conclusion.outcome
                    );
                    println!("PROMOTION={:?}", result.promotion);
                    match result.promotion {
                        PromotionDecision::Promote => 0,
                        PromotionDecision::DoNotPromote => 2,
                        PromotionDecision::Undecided => 3,
                    }
                }
                Err(error) => {
                    eprintln!("experiments: {error}");
                    1
                }
            }
        }
    };

    eprintln!("experiments: {} ({})", line.display(), line.status_word(code));
    code
}
