use pathgen::{AppPathRole, PathPolicy};
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

const APP: &str = "experiments";
const SPECS: &str = "specs";
const RUNS: &str = "runs";
const RESULTS: &str = "results";
const EVIDENCE: &str = "evidence";
const RECEIPTS: &str = "receipts";
const DIFFERENCES: &str = "differences";

#[derive(Debug, Clone)]
pub struct ExperimentPaths {
    pub data: PathBuf,
    pub state: PathBuf,
}

impl ExperimentPaths {
    pub fn under(base: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let policy = PathPolicy::dev(APP)?.with_dev_base(base.as_ref());
        let data = policy.ensure_dir(AppPathRole::Data)?.path;
        let state = policy.ensure_dir(AppPathRole::State)?.path;
        Ok(Self { data, state })
    }

    pub fn specs(&self) -> PathBuf {
        self.data.join(SPECS)
    }
    pub fn runs(&self) -> PathBuf {
        self.state.join(RUNS)
    }
    pub fn run(&self, run_id: &str) -> PathBuf {
        self.runs().join(run_id)
    }
    pub fn result(&self, run_id: &str) -> PathBuf {
        self.run(run_id).join(RESULTS).join("result.1o")
    }
    pub fn evidence(&self, run_id: &str) -> PathBuf {
        self.run(run_id).join(EVIDENCE)
    }
    pub fn receipts(&self, run_id: &str) -> PathBuf {
        self.run(run_id).join(RECEIPTS)
    }
    pub fn differences(&self, run_id: &str) -> PathBuf {
        self.run(run_id).join(DIFFERENCES)
    }

    pub fn ensure_run(&self, run_id: &str) -> std::io::Result<()> {
        fs::create_dir_all(self.evidence(run_id))?;
        fs::create_dir_all(self.receipts(run_id))?;
        fs::create_dir_all(self.differences(run_id))?;
        fs::create_dir_all(self.result(run_id).parent().expect("result has parent"))?;
        Ok(())
    }
}
