//! The command `spec-dna import` (design 0.7.0, D21 to D27; ADR 0014).

use crate::repo::{Fatal, Repo};

/// The tool that the project comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Speckit,
    Openspec,
}

/// What the command line gives to `run`.
#[derive(Clone, Debug)]
pub struct RunOpts {
    pub source: Source,
    pub dry_run: bool,
    pub skip_open: bool,
    /// `--areas`: a path from the current folder.
    pub areas: Option<String>,
    pub keep_research: bool,
    pub json: bool,
}

/// `spec-dna import`.
pub fn run(repo: &Repo, _o: &RunOpts) -> Result<u8, Fatal> {
    if !repo.config_path().exists() {
        return Err(Fatal::Config(
            "no spec-dna.toml: run `spec-dna init` first".into(),
        ));
    }
    Err(Fatal::Config("import is not built yet".into()))
}
