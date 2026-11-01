use serde::{Deserialize, Serialize};
use std::{io, path::Path, process::Command};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifferencesRequest {
    pub kind: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DifferencesSource {
    Command,
    Embedded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifferencesReport {
    pub source: DifferencesSource,
    pub kind: String,
    pub args: Vec<String>,
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl DifferencesReport {
    pub fn differs(&self) -> bool {
        self.status != 0 || !self.stdout.trim().is_empty()
    }
}

#[cfg(feature = "differences")]
pub(crate) fn run(root: &Path, request: &DifferencesRequest) -> io::Result<DifferencesReport> {
    let output = Command::new("differences")
        .arg(&request.kind)
        .args(&request.args)
        .current_dir(root)
        .output()?;
    Ok(DifferencesReport {
        source: DifferencesSource::Command,
        kind: request.kind.clone(),
        args: request.args.clone(),
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}
