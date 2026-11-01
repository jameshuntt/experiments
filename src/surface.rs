//! The `experiments` command line declared once through simple_impl_derive.
//!
//! The derived command is the contract: argv, validation, help, display and
//! exit-status meanings come from the same declaration.

use simple_impl_derive::{CompositeShell, SimpleSubCommand};
use std::ffi::{OsStr, OsString};
use xccute_contract::{
    CommandHelp, CommandValidationError, CommandValidationResult, CompositeShellCommand,
    ValidatedCommand,
};

#[derive(CompositeShell, Debug, Clone, Copy, Default)]
#[shell(program = "experiments")]
#[composite(command = "run", ty = Run, init = "spec")]
pub struct Experiments;

#[allow(clippy::duplicated_attributes)]
#[derive(SimpleSubCommand, Debug, Clone, Default)]
#[subcommand(segment = "run")]
#[exit_status(code = 0, word = "promote")]
#[exit_status(code = 1, word = "refused")]
#[exit_status(code = 2, word = "do_not_promote")]
#[exit_status(code = 3, word = "inconclusive")]
pub struct Run {
    /// Experiment spec: canonical 1o, JSON, TOML, YAML or YML.
    #[builder(init_required, into)]
    #[arg(positional, each = nonempty)]
    pub spec: OsString,

    /// PathGen development base. `.experiment-storage` by default.
    #[builder(into)]
    #[arg(positional, each = nonempty)]
    pub storage: Option<OsString>,
}

#[derive(Debug, Clone)]
pub enum Line {
    Run(Run),
}

impl Line {
    pub fn argv(&self) -> Vec<OsString> {
        match self {
            Line::Run(line) => line.clone().rooted::<Experiments>().argv(),
        }
    }

    pub fn display(&self) -> String {
        match self {
            Line::Run(line) => line.clone().rooted::<Experiments>().build_display(),
        }
    }

    pub fn validate(&self) -> CommandValidationResult {
        match self {
            Line::Run(line) => line.clone().rooted::<Experiments>().validate(),
        }
    }

    pub fn status_word(&self, code: i32) -> &'static str {
        match self {
            Line::Run(line) => line
                .clone()
                .rooted::<Experiments>()
                .exit_status_of(Some(code))
                .map_or("unnamed", |status| status.word),
        }
    }
}

fn refuse(field: &str, message: String) -> CommandValidationResult {
    Err(CommandValidationError::structural(message).with_field(field))
}

fn nonempty(field: &str, word: &OsStr) -> CommandValidationResult {
    if word.is_empty() {
        return refuse(field, format!("{field} is empty"));
    }
    Ok(())
}

pub fn usage_lines() -> Vec<xccute_contract::HelpLine> {
    Experiments::help_lines()
}

pub fn read(args: &[OsString]) -> Result<Line, String> {
    let words: Vec<String> = args
        .iter()
        .map(|arg| arg.to_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()
        .ok_or("a word is not UTF-8")?;

    let (segment, rest) = words.split_first().ok_or("no subcommand")?;
    let line = match segment.as_str() {
        "run" => {
            let spec = rest
                .first()
                .ok_or("experiments run names its experiment spec")?;
            let storage = rest.get(1);
            if rest.len() > 2 {
                return Err("experiments run takes SPEC [STORAGE]".into());
            }
            if spec.starts_with('-') {
                return Err("experiment spec must be a positional path".into());
            }
            if storage.is_some_and(|word| word.starts_with('-')) {
                return Err("storage must be a positional path".into());
            }
            Line::Run(Run {
                spec: OsString::from(spec.as_str()),
                storage: storage.map(|word| OsString::from(word.as_str())),
            })
        }
        other => return Err(format!("`experiments {other}`; the line is: run")),
    };

    line.validate().map_err(|error| error.to_string())?;
    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_run_line_reads_back_from_its_own_words() {
        let argv = Experiments::run("examples/v1.minimal-latency.1o.experiment")
            .storage(".experiment-storage")
            .argv();
        let line = read(&argv).expect("declared argv reads");
        assert_eq!(line.argv(), argv);
        assert_eq!(line.status_word(0), "promote");
        assert_eq!(line.status_word(2), "do_not_promote");
        assert_eq!(line.status_word(3), "inconclusive");
    }
}
