use std::{env, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    ExitCode::from(experiments::cli::main_argv(&args) as u8)
}
