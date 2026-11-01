use crate::{ExperimentResult, ExperimentSpec};
use std::{fs, io, path::Path};

pub fn read_spec(path: impl AsRef<Path>) -> io::Result<ExperimentSpec> {
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

pub fn write_spec(path: impl AsRef<Path>, spec: &ExperimentSpec) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(spec).map_err(io::Error::other)?;
    fs::write(path, bytes)
}

pub fn read_result(path: impl AsRef<Path>) -> io::Result<ExperimentResult> {
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(io::Error::other)
}

pub fn write_result(path: impl AsRef<Path>, result: &ExperimentResult) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(result).map_err(io::Error::other)?;
    fs::write(path, bytes)
}
