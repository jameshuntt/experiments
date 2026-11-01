use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifferencesRequest {
    /// Words after the `differences` program name, for example:
    /// `["tree", "base", "candidate"]`.
    ///
    /// The experiment target root is supplied by the instrument, so experiment
    /// files do not need to carry a second copy of it.
    pub line: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DifferencesSource {
    Embedded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DifferencesReport {
    pub source: DifferencesSource,
    /// The typed Differences line after it has been read and validated.
    pub kind: String,
    pub argv: Vec<String>,
    pub status: i32,
    pub page: String,
    pub said: Vec<String>,
    pub differs: bool,
}

#[cfg(feature = "differences")]
pub(crate) fn run(
    root: &Path,
    request: &DifferencesRequest,
) -> Result<DifferencesReport, String> {
    use differences_program::cli::{answer, Answer};
    use differences_program::surface;
    use std::ffi::OsString;

    let mut words: Vec<OsString> = request.line.iter().map(OsString::from).collect();
    words.push(OsString::from("--root"));
    words.push(root.as_os_str().to_os_string());

    // This is the same declared simple_impl/xccute surface the Differences CLI
    // itself uses. No `std::process::Command("differences")`, no stdout scraping.
    let line = surface::read(&words)?;
    let kind = line.path().to_owned();
    let argv = line
        .argv()
        .into_iter()
        .map(|word| word.to_string_lossy().into_owned())
        .collect();

    match answer(&line)? {
        Answer::Difference(difference) => Ok(DifferencesReport {
            source: DifferencesSource::Embedded,
            kind,
            argv,
            status: i32::from(difference.differs),
            page: difference.page,
            said: difference.said,
            differs: difference.differs,
        }),
        Answer::Said(said, status) => Ok(DifferencesReport {
            source: DifferencesSource::Embedded,
            kind,
            argv,
            status,
            page: String::new(),
            said,
            differs: status != 0,
        }),
    }
}

#[cfg(not(feature = "differences"))]
pub(crate) fn run(
    _root: &Path,
    _request: &DifferencesRequest,
) -> Result<DifferencesReport, String> {
    Err("experiments was built without the `differences` feature".into())
}
