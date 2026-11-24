//! `spec-dna eval run`: the runner of the evals (ADR 0008, `docs/commands.md`,
//! section `eval`). For each run, it copies a fixture and the harness files
//! of the project into a new temporary git repository, starts headless
//! `claude` in it, and runs the checks of the eval. spec-dna itself makes no
//! network call. The program `claude` does.
//!
//! The result keeps counts, costs, names and versions. It keeps no task text,
//! no check command, no output text of `claude`, no path of the machine and
//! no value of the environment, because they can hold secrets.

use crate::adapters::VERSION;
use crate::config::{Config, Evals};
use crate::evals::{self, Eval};
use crate::harness::{Harness, Level, Model, Role};
use crate::json::{self, Obj, Value};
use crate::repo::{Fatal, Repo};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant};

/// The tolerance of a comparison of USD sums. A sum of floating-point costs
/// such as 3 x 0.1 must not give a false refusal.
pub(crate) const EPS: f64 = 1e-9;
/// The most bytes of one line of the stdout of `claude`, and of the result
/// events together, that the runner keeps in memory.
const MAX_STDOUT: u64 = 16 * 1024 * 1024;
/// The date of the base commit of each run.
const BASE_DATE: &str = "2000-01-01T00:00:00Z";
/// The exit code of a check when the file `check.sh` cannot be sourced.
const SOURCE_FAILED: i32 = 97;

/// The options of `eval run`. The values are checked when the command line is
/// read.
pub struct Opts {
    pub ids: Vec<String>,
    pub runs: Option<u32>,
    pub min_pass_rate: Option<f64>,
    pub max_total_usd: Option<f64>,
    pub out: Option<String>,
    pub dry_run: bool,
    pub isolate: bool,
    pub json: bool,
    /// `--harness <arm>` (D57): the harness of the copy.
    pub harness: HarnessArm,
    /// True when `--harness` is set. The arm is then named: each arm, also
    /// `project`, gets the model flags, the pin and the adapter line.
    pub named: bool,
}

/// A harness arm (D57, ADR 0030): the team that the runner writes into the
/// copy of a run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HarnessArm {
    /// The files of the project that runs the command. 0.7.1 does this.
    Project,
    None,
    Lean,
    /// `lean` with the role `advisor` set to `none`.
    LeanNoAdvisor,
    Full,
}

impl HarnessArm {
    /// The names, in the order that the help text and the messages use.
    pub const ALL: [HarnessArm; 5] = [
        HarnessArm::None,
        HarnessArm::Lean,
        HarnessArm::LeanNoAdvisor,
        HarnessArm::Full,
        HarnessArm::Project,
    ];

    pub fn name(self) -> &'static str {
        match self {
            HarnessArm::Project => "project",
            HarnessArm::None => "none",
            HarnessArm::Lean => "lean",
            HarnessArm::LeanNoAdvisor => "lean-no-advisor",
            HarnessArm::Full => "full",
        }
    }

    /// Reads an arm name. An unknown name is a usage error.
    pub fn parse(s: &str) -> Result<HarnessArm, Fatal> {
        HarnessArm::ALL
            .into_iter()
            .find(|a| a.name() == s)
            .ok_or_else(|| {
                let names: Vec<&str> = HarnessArm::ALL.iter().map(|a| a.name()).collect();
                Fatal::Usage(format!("`{s}` is no harness arm; use {}", names.join(", ")))
            })
    }

    /// The harness that the arm writes into the copy, or `None` for `project`.
    /// Each built-in arm has no skill, no `REVIEW.md` and no hook
    /// `protect-tests`, so the arms differ only in the level and the roles.
    pub fn harness(self) -> Option<Harness> {
        let mut h = match self {
            HarnessArm::Project => return None,
            HarnessArm::None => Harness::new(Level::None),
            HarnessArm::Lean => Harness::new(Level::Lean),
            HarnessArm::LeanNoAdvisor => {
                let mut h = Harness::new(Level::Lean);
                h.cfg_mut(Role::Advisor).model = Model::None;
                h
            }
            HarnessArm::Full => Harness::new(Level::Full),
        };
        h.skills = Vec::new();
        h.review_md = false;
        Some(h)
    }
}

/// The `--model` and `--effort` that the runner passes in a named arm: the
/// values of the orchestrator of `lean`, in each arm (Q20).
fn model_flags() -> (String, String) {
    let lean = Harness::new(Level::Lean);
    let c = lean.cfg(Role::Orchestrator);
    (c.model.name().to_string(), c.effort.name().to_string())
}

/// An eval with the values that `eval run` uses: the `--runs` option replaces
/// the runs of the file, and the pass-at cannot be more than the runs.
pub(crate) struct Planned {
    pub(crate) eval: Eval,
    pub(crate) runs: u32,
    pub(crate) pass_at: u32,
    /// The files of the fixture: the source, and the path in the copy.
    pub(crate) fixture: Vec<(PathBuf, String)>,
    /// The files of the change (D60), or an empty list.
    pub(crate) change: Vec<(PathBuf, String)>,
}

/// The result of one run.
pub(crate) struct RunResult {
    pub(crate) claude_exit: i32,
    pub(crate) subtype: Option<String>,
    /// The `total_cost_usd` of `claude`, if it gave one for the whole run. A
    /// run that was killed, or whose stream is cut, has none.
    pub(crate) cost: Option<f64>,
    /// The last `total_cost_usd` of the stream, also of a run that is not
    /// whole. A refused attempt counts it, or 0.
    pub(crate) reported: Option<f64>,
    /// The cost that the cap counts: the cost, or the budget of the run. For
    /// an interrupted run, the cost that it has to pay: the budget if its
    /// agent ended and the run is not whole.
    pub(crate) charged: f64,
    pub(crate) checks: Vec<(String, bool)>,
    pub(crate) passed: bool,
    /// The class of the refusal, if the API refused the attempt (`refusal`).
    /// `eval run` without `--ab` ignores it, and counts the run as it is
    /// (ADR 0008).
    pub(crate) class: Option<Refusal>,
    /// The number of `permission_denials` of the result object, if it has the
    /// list.
    pub(crate) denials: Option<u32>,
    /// True if SIGINT or SIGTERM ended the attempt. The other fields mean
    /// nothing then.
    pub(crate) interrupted: bool,
    /// The names of the models in `modelUsage` of the result of `claude`.
    pub(crate) models: Vec<String>,
    /// The sums of the token counts over `modelUsage`, or `None`.
    pub(crate) tokens: Option<Tokens>,
    /// The sum of `num_turns` over the result events of `claude` (D95), or
    /// `None` if one event has no valid count.
    pub(crate) turns: Option<u32>,
    /// The number of result events, the segments of the run. `None` for an
    /// interrupted run.
    pub(crate) segments: Option<u32>,
    /// The wall time of the agent process, in seconds with 1 decimal.
    pub(crate) seconds: f64,
}

/// The token counts of one run: the sums over the entries of `modelUsage`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Tokens {
    pub(crate) input: u64,
    pub(crate) output: u64,
    pub(crate) cache_read: u64,
    pub(crate) cache_write: u64,
}

/// The runs of one eval.
struct Outcome {
    id: String,
    runs: u32,
    pass_at: u32,
    budget: f64,
    results: Vec<RunResult>,
}

impl Outcome {
    fn passed_runs(&self) -> u32 {
        self.results.iter().filter(|r| r.passed).count() as u32
    }

    fn spend(&self) -> f64 {
        // `sum` of no floats is -0.0, and the output would show `-0`.
        self.results.iter().fold(0.0, |sum, r| sum + r.charged)
    }

    /// An eval passes when its passed runs are at least the pass-at. An eval
    /// that did not run all its runs (a stop by the cap) does not pass.
    fn passed(&self) -> bool {
        self.results.len() as u32 == self.runs && self.passed_runs() >= self.pass_at
    }
}

/// Why a plain run stopped: the spend is above the cap, or a folder is below
/// the floor of free space (D100).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum StopBy {
    Cap,
    Disk,
}

impl StopBy {
    /// The value of the key `stopped` of the result.
    fn name(self) -> &'static str {
        match self {
            StopBy::Cap => "max-total-usd",
            StopBy::Disk => "disk-space",
        }
    }
}

/// All values that the output needs.
struct Report {
    claude_code: Option<String>,
    started: String,
    evals_dir: String,
    min_pass_rate: f64,
    max_total_usd: f64,
    estimate: f64,
    outcomes: Vec<Outcome>,
    /// The number of runs that did not start, after a stop.
    stopped: Option<u32>,
    /// The reason of the stop.
    stopped_by: StopBy,
    /// The harness of the copy of each run.
    harness: HarnessArm,
}

impl Report {
    fn spend(&self) -> f64 {
        self.outcomes.iter().fold(0.0, |sum, o| sum + o.spend())
    }

    fn passed_evals(&self) -> usize {
        self.outcomes.iter().filter(|o| o.passed()).count()
    }

    /// The share of evals that pass. `None` after a stop.
    fn pass_rate(&self) -> Option<f64> {
        if self.stopped.is_some() || self.outcomes.is_empty() {
            return None;
        }
        Some(self.passed_evals() as f64 / self.outcomes.len() as f64)
    }

    fn passed(&self) -> bool {
        self.pass_rate()
            .is_some_and(|r| r + EPS >= self.min_pass_rate)
    }

    fn exit_code(&self) -> u8 {
        if self.stopped.is_some() {
            3
        } else if self.passed() {
            0
        } else {
            1
        }
    }
}

pub fn run(repo: &Repo, cfg: &Config, o: &Opts) -> Result<u8, Fatal> {
    let table = cfg.evals.as_ref().ok_or_else(|| {
        Fatal::Config(
            "spec-dna.toml has no [evals] table, and `eval run` needs it; add the table (docs/config.md)"
                .into(),
        )
    })?;
    let out_dir = o
        .out
        .as_deref()
        .map(|p| normalize_out(p, &table.dir))
        .transpose()?;
    let cap = o.max_total_usd.unwrap_or(table.max_total_usd);
    let min_pass_rate = o.min_pass_rate.unwrap_or(table.min_pass_rate);

    let all = load_evals(repo, table)?;
    let check_sh = repo.root.join(&table.dir).join(evals::CHECK_FILE);
    if !check_sh.is_file() {
        return Err(Fatal::Config(format!(
            "{}/{}: the check file is absent; `eval run` sources it",
            table.dir,
            evals::CHECK_FILE
        )));
    }
    let selected = select(all, &o.ids, &table.dir)?;

    let mut plan = Vec::new();
    for eval in selected {
        let runs = o.runs.unwrap_or(eval.runs);
        plan.push(plan_eval(repo, eval, runs)?);
    }
    let total_runs: u32 = plan.iter().map(|p| p.runs).sum();
    let estimate: f64 = plan
        .iter()
        .map(|p| f64::from(p.runs) * p.eval.max_budget_usd)
        .sum();
    if estimate > cap + EPS {
        return Err(Fatal::Config(format!(
            "the estimate {estimate:.2} USD (runs x max-budget-usd) is above max-total-usd {cap:.2} USD; no run started. Lower the runs, or raise max-total-usd"
        )));
    }
    let header = format!(
        "{} evals, {total_runs} runs, estimate {estimate:.2} USD, cap {cap:.2} USD",
        plan.len()
    );
    if o.dry_run {
        dry_run(&plan, &header, estimate, cap, table, o.json);
        return Ok(0);
    }

    // A bad `--out` must stop the command before the first run spends money.
    if let Some(dir) = &out_dir {
        make_out_dir(repo, dir)?;
    }
    let claude_code = claude_version()?;
    let harness = harness_list(repo)?;
    if !o.json {
        println!("eval run: {header}");
    }
    let mut report = Report {
        claude_code,
        started: utc_time(now_secs()),
        evals_dir: table.dir.clone(),
        min_pass_rate,
        max_total_usd: cap,
        estimate,
        outcomes: Vec::new(),
        stopped: None,
        stopped_by: StopBy::Cap,
        harness: o.harness,
    };
    let mut ctx = Ctx {
        limits: Limits::from_env()?,
        merge_instructions: false,
        harness,
        check_sh,
        isolate: o.isolate,
        counter: 0,
        arm: o.harness,
        named: o.named,
    };
    let arm = o.harness;
    let named = o.named;
    let edit = |dir: &Path| crate::eval_ab::write_harness_arm(dir, arm, &[], named);
    // The folder of the result file, if the command writes one (D100).
    let result_folder = out_dir.as_ref().map(|d| (repo.root.join(d), d.to_string()));
    let mut floor = Floor::new(table.min_free_mib, result_folder);
    let mut spend = 0.0;
    let mut started_runs = 0;
    'evals: for p in &plan {
        let mut outcome = Outcome {
            id: p.eval.id.clone(),
            runs: p.runs,
            pass_at: p.pass_at,
            budget: p.eval.max_budget_usd,
            results: Vec::new(),
        };
        for n in 1..=p.runs {
            if let Some(low) = floor.low() {
                eprintln!("{}", low.message(None));
                report.stopped = Some(total_runs - started_runs);
                report.stopped_by = StopBy::Disk;
                report.outcomes.push(outcome);
                break 'evals;
            }
            eprintln!("eval run: {} run {n} of {}", p.eval.id, p.runs);
            let result = match one_attempt(&mut ctx, p, &edit, None) {
                Ok(r) => r,
                Err(e) => {
                    let sofar = spend + outcome.spend();
                    return Err(match e {
                        Fatal::Io(m) if sofar > 0.0 => {
                            Fatal::Io(format!("{m} (the runs so far spent {sofar:.2} USD)"))
                        }
                        e => e,
                    });
                }
            };
            outcome.results.push(result);
            started_runs += 1;
            let now = spend + outcome.spend();
            if now > cap + EPS && started_runs < total_runs {
                report.stopped = Some(total_runs - started_runs);
                report.outcomes.push(outcome);
                break 'evals;
            }
        }
        spend += outcome.spend();
        report.outcomes.push(outcome);
    }
    // An eval that did not start has an empty list of results.
    if report.stopped.is_some() {
        for p in &plan[report.outcomes.len()..] {
            report.outcomes.push(Outcome {
                id: p.eval.id.clone(),
                runs: p.runs,
                pass_at: p.pass_at,
                budget: p.eval.max_budget_usd,
                results: Vec::new(),
            });
        }
    }

    let compact = report_json(&report);
    if o.json {
        println!("{compact}");
    } else {
        for line in report_text(&report) {
            println!("{line}");
        }
    }
    if let Some(dir) = out_dir {
        let path = write_result(repo, &dir, "eval", &report.started, &compact)?;
        eprintln!("eval run: the result is in {path}");
    }
    Ok(report.exit_code())
}

/// The plan of one eval: its runs, its pass-at and the files of its fixture
/// and of its change.
pub(crate) fn plan_eval(repo: &Repo, eval: Eval, runs: u32) -> Result<Planned, Fatal> {
    let pass_at = eval.pass_at.min(runs);
    let fixture = copy_list(
        repo,
        &format!(":(literal){}", eval.fixture),
        &format!("{}/", eval.fixture),
    )?;
    let change = match &eval.change {
        Some(c) => copy_list(repo, &format!(":(literal){c}"), &format!("{c}/"))?,
        None => Vec::new(),
    };
    Ok(Planned {
        eval,
        runs,
        pass_at,
        fixture,
        change,
    })
}

/// Checks the value of `--out`: a folder below the repository root, and not
/// `[evals] dir`, because the check SD506 reads each `*.json` file there.
pub(crate) fn normalize_out(path: &str, dir: &str) -> Result<String, Fatal> {
    let mut s = path;
    while let Some(rest) = s.strip_prefix("./") {
        s = rest;
    }
    let s = s.trim_end_matches('/');
    if s.is_empty()
        || s.starts_with('/')
        || s.contains('\\')
        || s.contains(':')
        || s.chars().any(char::is_control)
        || s.split('/')
            .any(|seg| seg.is_empty() || seg == "." || seg == "..")
    {
        return Err(Fatal::Usage(format!(
            "--out: `{path}` must be a folder below the repository root, with no `..` or `.` segment"
        )));
    }
    if s == dir {
        return Err(Fatal::Usage(format!(
            "--out: `{path}` is [evals] dir; the check SD506 reads each *.json file there, so write the result in a folder of its own, for example `{dir}/results`"
        )));
    }
    Ok(s.to_string())
}

/// Reads and checks each eval file of the working tree: the tracked files, and
/// the untracked files that git does not ignore. It returns the evals in the
/// order of their ids.
pub(crate) fn load_evals(repo: &Repo, table: &Evals) -> Result<Vec<Eval>, Fatal> {
    load_evals_in(repo, table, &table.dir, &|dir| {
        format!("no eval file in `{dir}`; an eval is the file `{dir}/<id>.json`")
    })
}

/// Like `load_evals`, for the files `<folder>/<id>.json`. `none` gives the
/// message for a folder with no eval. The fixture of an eval is still a
/// folder under `[evals] dir`.
pub(crate) fn load_evals_in(
    repo: &Repo,
    table: &Evals,
    folder: &str,
    none: &dyn Fn(&str) -> String,
) -> Result<Vec<Eval>, Fatal> {
    let dir = folder;
    let mut files = repo
        .git
        .list_worktree(&[format!(":(literal){dir}")])
        .map_err(Fatal::Io)?;
    // A tracked file that is deleted in the working tree is not a file.
    files.retain(|f| fs::symlink_metadata(repo.root.join(f)).is_ok());
    let has_file_under = |p: &str| {
        let under = format!("{p}/");
        files.iter().any(|f| f.starts_with(&under))
    };
    let prefix = format!("{dir}/");
    let mut evals = Vec::new();
    let mut errors = Vec::new();
    for f in &files {
        let Some(name) = f.strip_prefix(&prefix) else {
            continue;
        };
        let Some(stem) = name.strip_suffix(".json").filter(|s| !s.is_empty()) else {
            continue;
        };
        if name.contains('/') {
            continue;
        }
        let bytes = match read_limited(&repo.root.join(f), evals::MAX_BYTES as u64 + 1) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!("{f}: the file cannot be read ({e})"));
                continue;
            }
        };
        match evals::parse_eval(stem, &bytes, table, &has_file_under) {
            Ok(e) => evals.push(e),
            Err(e) => errors.push(format!("{f}: {}", e.message)),
        }
    }
    if !errors.is_empty() {
        return Err(Fatal::Config(format!(
            "invalid eval file:\n{}",
            errors.join("\n")
        )));
    }
    if evals.is_empty() {
        return Err(Fatal::Config(none(dir)));
    }
    evals.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(evals)
}

pub(crate) fn read_limited(path: &Path, limit: u64) -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    fs::File::open(path)?.take(limit).read_to_end(&mut buf)?;
    Ok(buf)
}

/// Selects the evals of the ids. With no id, it selects each eval.
pub(crate) fn select(all: Vec<Eval>, ids: &[String], dir: &str) -> Result<Vec<Eval>, Fatal> {
    if ids.is_empty() {
        return Ok(all);
    }
    for id in ids {
        if !all.iter().any(|e| &e.id == id) {
            return Err(Fatal::Usage(format!(
                "no eval `{id}`; an eval is the file `{dir}/{id}.json`"
            )));
        }
    }
    Ok(all.into_iter().filter(|e| ids.contains(&e.id)).collect())
}

/// The files that git lists under a pathspec, as a list of copies. A symbolic
/// link is not copied, and stderr gets a warning. `strip` is the prefix that
/// the path in the copy does not have.
pub(crate) fn copy_list(
    repo: &Repo,
    spec: &str,
    strip: &str,
) -> Result<Vec<(PathBuf, String)>, Fatal> {
    let mut out = Vec::new();
    let paths = repo
        .git
        .list_worktree(&[spec.to_string()])
        .map_err(Fatal::Io)?;
    for p in paths {
        let src = repo.root.join(&p);
        // A path that is absent (a deleted file) is no file.
        let Ok(meta) = fs::symlink_metadata(&src) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            eprintln!("spec-dna: warning: `eval run` does not copy the symbolic link `{p}`");
            continue;
        }
        if !meta.is_file() {
            continue;
        }
        if let Some(rel) = p.strip_prefix(strip) {
            out.push((src, rel.to_string()));
        }
    }
    Ok(out)
}

/// The harness files of the project: `CLAUDE.md`, `AGENTS.md`, `REVIEW.md` and
/// the files of `.claude/`, but not `.claude/settings.local.json`, which is
/// the local state of one person. It uses the list of git, so an ignored file
/// is not copied.
pub(crate) fn harness_list(repo: &Repo) -> Result<Vec<(PathBuf, String)>, Fatal> {
    let specs: Vec<String> = ["CLAUDE.md", "AGENTS.md", "REVIEW.md", ".claude"]
        .iter()
        .map(|p| format!(":(literal){p}"))
        .collect();
    let paths = repo.git.list_worktree(&specs).map_err(Fatal::Io)?;
    let mut out = Vec::new();
    for p in paths {
        if p == ".claude/settings.local.json" {
            continue;
        }
        let src = repo.root.join(&p);
        let Ok(meta) = fs::symlink_metadata(&src) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            eprintln!("spec-dna: warning: `eval run` does not copy the symbolic link `{p}`");
        } else if meta.is_file() {
            out.push((src, p));
        }
    }
    Ok(out)
}

fn dry_run(plan: &[Planned], header: &str, estimate: f64, cap: f64, table: &Evals, json: bool) {
    if json {
        let evals = json::array(plan.iter().map(|p| {
            Obj::new()
                .str("id", &p.eval.id)
                .num("runs", p.runs)
                .num("pass_at", p.pass_at)
                .num("max_budget_usd", r6(p.eval.max_budget_usd))
                .num(
                    "estimate_usd",
                    r6(f64::from(p.runs) * p.eval.max_budget_usd),
                )
                .end()
        }));
        println!(
            "{}",
            Obj::new()
                .str("spec_dna", VERSION)
                .bool("dry_run", true)
                .str("evals_dir", &table.dir)
                .num("max_total_usd", r6(cap))
                .num("estimate_usd", r6(estimate))
                .raw("evals", &evals)
                .end()
        );
        return;
    }
    println!("eval run (dry run): {header}");
    for p in plan {
        println!(
            "{}: {} runs, pass-at {}, max-budget-usd {:.2}, estimate {:.2} USD",
            p.eval.id,
            p.runs,
            p.pass_at,
            p.eval.max_budget_usd,
            f64::from(p.runs) * p.eval.max_budget_usd
        );
    }
}

fn start_failed(e: &std::io::Error) -> Fatal {
    Fatal::Config(format!(
        "`claude` did not start ({e}); install Claude Code (https://code.claude.com/docs/en/setup.md) and put it on PATH"
    ))
}

/// Runs `claude --version` once. The version is the first word of stdout if
/// it has only digits and dots.
pub(crate) fn claude_version() -> Result<Option<String>, Fatal> {
    let out = Command::new("claude")
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| start_failed(&e))?;
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(text
        .split_whitespace()
        .next()
        .filter(|w| {
            w.starts_with(|c: char| c.is_ascii_digit())
                && w.chars().all(|c| c.is_ascii_digit() || c == '.')
        })
        .map(str::to_string))
}

/// State that all runs share.
pub(crate) struct Ctx {
    /// The time limits of an agent and of a check.
    pub(crate) limits: Limits,
    /// True for `--ab`: the files `AGENTS.md` and `CLAUDE.md` of the fixture
    /// keep their text, and the copy adds only the generated lines of the
    /// project to them.
    pub(crate) merge_instructions: bool,
    pub(crate) harness: Vec<(PathBuf, String)>,
    /// The absolute path of `<dir>/check.sh` of the project.
    pub(crate) check_sh: PathBuf,
    pub(crate) isolate: bool,
    pub(crate) counter: u32,
    /// The harness of the copy (D57). `Project` copies the harness files of
    /// the project. Another arm copies none, and its `edit` writes the arm.
    pub(crate) arm: HarnessArm,
    /// True for `--arms` and `--harness`: each arm, also `project`, gets the
    /// same `--model` and `--effort`, so the main model never differs between
    /// the arms. A run with no named arm keeps the command line of 0.7.1.
    pub(crate) named: bool,
}

/// A temporary folder. The drop removes it, also after an error.
struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn make_temp(counter: &mut u32) -> Result<TempDir, Fatal> {
    let base = std::env::temp_dir();
    loop {
        *counter += 1;
        let path = base.join(format!("spec-dna-eval-{}-{counter}", std::process::id()));
        #[cfg_attr(not(unix), allow(unused_mut))]
        let mut b = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            b.mode(0o700);
        }
        match b.create(&path) {
            Ok(()) => return Ok(TempDir(path)),
            // A folder of a run that a crash left. The next number is free.
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && *counter < 100_000 => {}
            Err(e) => {
                return Err(Fatal::Io(format!(
                    "cannot make the folder {}: {e}",
                    path.display()
                )));
            }
        }
    }
}

/// The environment of `claude`, of the git commands in the copy and of the
/// checks: the environment of the runner, without the variables that name a
/// repository or the project folder.
fn clean_env(cmd: &mut Command) {
    for name in [
        "CLAUDE_PROJECT_DIR",
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
    ] {
        cmd.env_remove(name);
    }
}

/// The git settings of the agent and of the checks: a neutral identity, no
/// signature, and no global or system configuration of the person. Thus a
/// commit of the agent or of a check never depends on the machine.
fn neutral_git_env(cmd: &mut Command) {
    cmd.env("GIT_AUTHOR_NAME", "spec-dna eval")
        .env("GIT_AUTHOR_EMAIL", "eval@example.invalid")
        .env("GIT_COMMITTER_NAME", "spec-dna eval")
        .env("GIT_COMMITTER_EMAIL", "eval@example.invalid")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_COUNT", "2")
        .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
        .env("GIT_CONFIG_VALUE_0", "false")
        .env("GIT_CONFIG_KEY_1", "tag.gpgsign")
        .env("GIT_CONFIG_VALUE_1", "false");
}

/// Runs git in the copy. The author, the dates and the signature are fixed,
/// and the global and the system configuration do not apply.
fn git_in(dir: &Path, args: &[&str]) -> Result<String, Fatal> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(dir).args(args);
    clean_env(&mut cmd);
    cmd.env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_DATE", BASE_DATE)
        .env("GIT_COMMITTER_DATE", BASE_DATE)
        .env("LC_ALL", "C")
        .stdin(Stdio::null());
    // A Ctrl-C of the terminal must not kill a git helper: the interrupt would
    // look like a git error.
    own_process_group(&mut cmd);
    let out = cmd
        .output()
        .map_err(|e| Fatal::Io(format!("cannot run git: {e}")))?;
    if !out.status.success() {
        return Err(Fatal::Io(format!(
            "git {} failed in the copy: {}",
            args.first().copied().unwrap_or(""),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn copy_files(dir: &Path, files: &[(PathBuf, String)]) -> Result<(), Fatal> {
    for (src, rel) in files {
        let dst = dir.join(rel);
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Fatal::Io(format!("cannot copy `{rel}`: {e}")))?;
        }
        // A copy with `fs::copy` keeps the time of the source. A file of the
        // change that has the size and the time of the file of the fixture (a
        // ticked box has the same size) then looks unchanged to git, and
        // `git add -A` drops it from the change commit. A new write gives the
        // file the time of now.
        let bytes = fs::read(src).map_err(|e| Fatal::Io(format!("cannot copy `{rel}`: {e}")))?;
        fs::write(&dst, bytes).map_err(|e| Fatal::Io(format!("cannot copy `{rel}`: {e}")))?;
        // The mode of the source (a script keeps its execute bit).
        if let Ok(meta) = fs::metadata(src) {
            let _ = fs::set_permissions(&dst, meta.permissions());
        }
    }
    Ok(())
}

/// One attempt of a run. `edit` changes the copy after the files are copied
/// and before the base commit (`--ab` uses it for the arm). `deny` is the
/// value of `--disallowedTools`, which goes after the place of
/// `--allowedTools`.
pub(crate) fn one_attempt(
    ctx: &mut Ctx,
    p: &Planned,
    edit: &dyn Fn(&Path) -> Result<(), Fatal>,
    deny: Option<&str>,
) -> Result<RunResult, Fatal> {
    let temp = make_temp(&mut ctx.counter)?;
    let dir = temp.0.as_path();
    copy_files(dir, &p.fixture)?;
    if ctx.arm == HarnessArm::Project {
        if ctx.merge_instructions {
            copy_harness_merged(dir, &ctx.harness)?;
        } else {
            copy_files(dir, &ctx.harness)?;
        }
    }
    edit(dir)?;
    let hooks = dir.join(".git").join("no-hooks");
    let hooks = hooks.to_string_lossy().into_owned();
    git_in(dir, &["init", "-q", "-b", "main"])?;
    git_in(dir, &["add", "-A"])?;
    git_in(
        dir,
        &[
            "-c",
            "user.name=spec-dna",
            "-c",
            "user.email=eval@spec-dna.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            &format!("core.hooksPath={hooks}"),
            "commit",
            "-q",
            "--no-verify",
            "-m",
            "spec-dna eval base",
        ],
    )?;
    if let Some(branch) = p.eval.switch_branch() {
        git_in(dir, &["switch", "-q", "-c", branch])?;
    }
    if !p.change.is_empty() {
        // D60: the files of the change are one commit on the branch, so the
        // diff against `main` shows them.
        copy_files(dir, &p.change)?;
        git_in(dir, &["add", "-A"])?;
        git_in(
            dir,
            &[
                "-c",
                "user.name=spec-dna",
                "-c",
                "user.email=eval@spec-dna.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                &format!("core.hooksPath={hooks}"),
                "commit",
                "-q",
                "--no-verify",
                "--allow-empty",
                "-m",
                "spec-dna eval change",
            ],
        )?;
    }
    // The base of the checks is the commit before the agent starts: the commit
    // of the change comes before it, so a check like `tests_unchanged` does
    // not see the change as a diff. The branch `main` is the base of the diff
    // that the agent sees.
    let base = git_in(dir, &["rev-parse", "HEAD"])?;

    let mut cmd = Command::new("claude");
    cmd.arg("-p").arg(&p.eval.task);
    if !p.eval.allowed_tools.is_empty() {
        // One argument, so no later word of the list can be read as a flag.
        cmd.arg("--allowedTools")
            .arg(p.eval.allowed_tools.join(","));
    }
    if let Some(d) = deny {
        cmd.arg("--disallowedTools").arg(d);
    }
    cmd.args([
        "--output-format",
        "stream-json",
        "--verbose",
        "--max-turns",
        &p.eval.max_turns.to_string(),
        "--max-budget-usd",
        &p.eval.max_budget_usd.to_string(),
        "--permission-mode",
        "dontAsk",
    ]);
    if ctx.named {
        let (model, effort) = model_flags();
        cmd.args(["--model", &model, "--effort", &effort]);
    }
    if ctx.isolate {
        cmd.args(["--setting-sources", "project", "--strict-mcp-config"]);
    }
    cmd.current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    clean_env(&mut cmd);
    neutral_git_env(&mut cmd);
    binary_first_on_path(&mut cmd);
    own_process_group(&mut cmd);
    let agent_start = Instant::now();
    let mut child = cmd.spawn().map_err(|e| start_failed(&e))?;
    let out_reader = child
        .stdout
        .take()
        .map(|r| std::thread::spawn(move || read_events(r)));
    let err_reader = child
        .stderr
        .take()
        .map(|r| std::thread::spawn(move || tee_stderr(r)));
    let ended = wait_limited(&mut child, ctx.limits.agent)
        .map_err(|e| Fatal::Io(format!("cannot wait for `claude`: {e}")))?;
    // The wall time of the agent process, with 1 decimal.
    let seconds = (agent_start.elapsed().as_secs_f64() * 10.0).round() / 10.0;
    let Stream { out: stdout, cut } = out_reader.and_then(|h| h.join().ok()).unwrap_or_default();
    let stderr = err_reader.and_then(|h| h.join().ok()).unwrap_or_default();
    if ended == Ended::Interrupted {
        drop(temp);
        return Ok(RunResult {
            claude_exit: -1,
            subtype: None,
            cost: None,
            reported: None,
            charged: 0.0,
            checks: Vec::new(),
            passed: false,
            class: None,
            denials: None,
            interrupted: true,
            models: Vec::new(),
            tokens: None,
            turns: None,
            segments: None,
            seconds: 0.0,
        });
    }
    let timed_out = ended == Ended::TimedOut;
    let claude_exit = match ended {
        Ended::Exited(code) => code,
        _ => -1,
    };
    // The run is whole when `claude` ended by itself and no work followed its
    // last result event. Else the last result does not give the cost or the
    // turns of the run, and the run counts its budget (ADR 0033).
    let whole = matches!(ended, Ended::Exited(code) if code >= 0) && !cut;
    let (reported, mut subtype) = read_result(&stdout);
    let cost = reported.filter(|_| whole);
    let limited = first_limit(&stdout).is_some();
    let denials = result_denials(&stdout);
    let class = if timed_out {
        None
    } else {
        refusal(claude_exit, &stdout, &stderr)
    };
    if timed_out {
        subtype = Some("timeout".to_string());
    }
    let models = result_models(&stdout);
    let tokens = result_tokens(&stdout);
    let turns = result_turns(&stdout).filter(|_| whole);
    let segments = Some(result_events(&stdout).len() as u32);

    let mut checks = Vec::new();
    for c in &p.eval.checks {
        let script = format!(
            ". \"$SPEC_DNA_CHECK_SH\" || exit {SOURCE_FAILED}\n{}",
            c.run
        );
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(script)
            .current_dir(dir)
            .env("SPEC_DNA_CHECK_SH", &ctx.check_sh)
            .env("SPEC_DNA_EVAL_BASE", &base)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit());
        clean_env(&mut cmd);
        neutral_git_env(&mut cmd);
        binary_first_on_path(&mut cmd);
        own_process_group(&mut cmd);
        let mut child = cmd
            .spawn()
            .map_err(|e| Fatal::Io(format!("cannot run `sh`: {e}")))?;
        let ended = wait_limited(&mut child, ctx.limits.check)
            .map_err(|e| Fatal::Io(format!("cannot run `sh`: {e}")))?;
        if ended == Ended::Interrupted {
            drop(temp);
            // The agent ran, so its cost is paid.
            let paid = if whole {
                reported.unwrap_or(0.0)
            } else {
                p.eval.max_budget_usd
            };
            return Ok(RunResult {
                claude_exit,
                subtype,
                cost,
                reported,
                charged: paid,
                checks,
                passed: false,
                class: None,
                denials,
                interrupted: true,
                models,
                tokens,
                turns,
                segments,
                seconds,
            });
        }
        if ended == Ended::TimedOut {
            subtype = Some("timeout".to_string());
        }
        checks.push((c.name.clone(), ended == Ended::Exited(0)));
    }
    drop(temp);
    let passed = claude_exit == 0 && !limited && checks.iter().all(|c| c.1);
    Ok(RunResult {
        claude_exit,
        subtype,
        cost,
        reported,
        charged: cost.unwrap_or(p.eval.max_budget_usd),
        checks,
        passed,
        class,
        denials,
        interrupted: false,
        models,
        tokens,
        turns,
        segments,
        seconds,
    })
}

/// Copies the harness files. The files `AGENTS.md` and `CLAUDE.md` of the
/// fixture keep their text: the copy adds the generated lines of the project
/// (the line of the adapter and the line of the harness) after it. The line
/// of the adapter is the line of this binary, because the pin of the copy is
/// the version of this binary: a line of another version or build gives SD502
/// in the copy. A fixture with no such file gets only these lines, and a
/// project with no such line leaves the fixture file as it is.
fn copy_harness_merged(dir: &Path, files: &[(PathBuf, String)]) -> Result<(), Fatal> {
    for (src, rel) in files {
        if rel != "AGENTS.md" && rel != "CLAUDE.md" {
            copy_files(dir, &[(src.clone(), rel.clone())])?;
            continue;
        }
        let project = fs::read_to_string(src).unwrap_or_default();
        let lines = crate::adapters::generated_lines(&project);
        if lines.is_empty() {
            continue;
        }
        let dst = dir.join(rel);
        let mut text = fs::read_to_string(&dst).unwrap_or_default();
        if !text.is_empty() {
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text.push('\n');
        }
        for l in &lines {
            // The harness line has the marker `spec-dna harness v`.
            let own = if l.contains("<!-- spec-dna v") {
                crate::adapters::line()
            } else {
                l.clone()
            };
            text.push_str(&own);
            text.push('\n');
        }
        fs::write(&dst, text).map_err(|e| Fatal::Io(format!("cannot copy `{rel}`: {e}")))?;
    }
    Ok(())
}

/// The time limits of an agent and of a check: 25 and 5 minutes. A process
/// that runs longer is killed with its process group, and the run counts as
/// failed with the subtype `timeout`.
#[derive(Clone, Copy)]
pub(crate) struct Limits {
    pub(crate) agent: Duration,
    pub(crate) check: Duration,
}

const AGENT_LIMIT: u64 = 25 * 60;
const CHECK_LIMIT: u64 = 5 * 60;

impl Limits {
    /// Reads `SPEC_DNA_EVAL_TIMEOUT` (the agent) and
    /// `SPEC_DNA_EVAL_CHECK_TIMEOUT` (a check): integers of seconds from 1 to
    /// 86,400. Another value is a usage error.
    pub(crate) fn from_env() -> Result<Limits, Fatal> {
        Ok(Limits {
            agent: Duration::from_secs(secs_var("SPEC_DNA_EVAL_TIMEOUT", AGENT_LIMIT)?),
            check: Duration::from_secs(secs_var("SPEC_DNA_EVAL_CHECK_TIMEOUT", CHECK_LIMIT)?),
        })
    }
}

fn secs_var(name: &str, default: u64) -> Result<u64, Fatal> {
    let Ok(v) = std::env::var(name) else {
        return Ok(default);
    };
    v.parse::<u64>()
        .ok()
        .filter(|n| (1..=86_400).contains(n) && v.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(|| {
            Fatal::Usage(format!(
                "{name}: `{v}` is not an integer of seconds from 1 to 86400"
            ))
        })
}

/// The free space of a folder, in KiB, from the output of `df -Pk <folder>`
/// (POSIX: a header, and one line of 6 fields). The value is the field before
/// the first field of the second line that ends with `%` (the capacity), so a
/// name of a file system with spaces does not move it. `None` if the output
/// has no such field or the value is no integer.
pub(crate) fn parse_df_available(output: &str) -> Option<u64> {
    let line = output.lines().nth(1)?;
    let fields: Vec<&str> = line.split_whitespace().collect();
    let at = fields.iter().position(|f| f.ends_with('%'))?;
    fields.get(at.checked_sub(1)?)?.parse().ok()
}

/// The free space of a folder in MiB, with `df -Pk` (D100). `None` if `df` is
/// absent, fails, or gives output that does not parse. The program is found on
/// `PATH`. A call of `statvfs` would need a dependency or `unsafe` code.
fn free_mib(folder: &Path) -> Option<u64> {
    let out = Command::new("df")
        .arg("-Pk")
        .arg(folder)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_df_available(&String::from_utf8_lossy(&out.stdout)).map(|kib| kib / 1024)
}

/// `path` as one word of a POSIX shell, for a command that a message prints: as
/// it is when each character is safe, else in single quotes.
pub(crate) fn shell_word(path: &str) -> String {
    let safe = !path.is_empty()
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-./,:+@%=".contains(c));
    if safe {
        path.to_string()
    } else {
        format!("'{}'", path.replace('\'', "'\\''"))
    }
}

/// A folder that is below the floor of free space.
pub(crate) struct Low {
    pub(crate) folder: String,
    pub(crate) free_mib: u64,
    pub(crate) min_mib: u64,
}

impl Low {
    /// The message of the stop (D100). `resume` is the command that goes on:
    /// a plain `eval run` has none, so its message ends with "run the command
    /// again".
    pub(crate) fn message(&self, resume: Option<&str>) -> String {
        let again = match resume {
            Some(file) => format!("run the command again with `--resume {}`", shell_word(file)),
            None => "run the command again".to_string(),
        };
        format!(
            "spec-dna eval run: stopped before a run: `{}` has {} MiB free, below `[evals] min-free-mib` ({}); free space, then {again}",
            self.folder, self.free_mib, self.min_mib
        )
    }
}

/// The check of the free space before each run (D100). It reads the temporary
/// folder, where the copy goes, and the folder of the result file. A floor of
/// 0 turns the check off. If `df` gives no value, the command prints one
/// warning and runs with no check.
pub(crate) struct Floor {
    min_mib: u64,
    /// The folder, and the name that the messages use.
    folders: Vec<(PathBuf, String)>,
    off: bool,
}

impl Floor {
    /// A check of the temporary folder, and of `result` (the folder of the
    /// result file and its name) when there is one.
    pub(crate) fn new(min_mib: u64, result: Option<(PathBuf, String)>) -> Floor {
        let tmp = std::env::temp_dir();
        let name = tmp.display().to_string();
        let mut folders = vec![(tmp, name)];
        folders.extend(result);
        Floor {
            min_mib,
            folders,
            off: false,
        }
    }

    /// Reads the free space before a run. It returns the first folder that is
    /// below the floor.
    pub(crate) fn low(&mut self) -> Option<Low> {
        if self.min_mib == 0 || self.off {
            return None;
        }
        for (path, name) in &self.folders {
            match free_mib(path) {
                Some(n) if n < self.min_mib => {
                    return Some(Low {
                        folder: name.clone(),
                        free_mib: n,
                        min_mib: self.min_mib,
                    });
                }
                Some(_) => {}
                None => {
                    eprintln!(
                        "spec-dna: warning: cannot read the free space of `{name}` (`df -Pk`); the check of `[evals] min-free-mib` is off for this command"
                    );
                    self.off = true;
                    return None;
                }
            }
        }
        None
    }
}

/// The `PATH` of the agent and of the checks has the folder of the running
/// binary first, so `spec-dna` in a copy is the binary under test, not the
/// first one on the `PATH` of the person.
fn binary_first_on_path(cmd: &mut Command) {
    let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    else {
        return;
    };
    let mut paths = vec![dir];
    if let Some(old) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&old));
    }
    if let Ok(joined) = std::env::join_paths(paths) {
        cmd.env("PATH", joined);
    }
}

/// Puts the child in a new process group, so a kill reaches its children.
#[cfg(unix)]
fn own_process_group(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
}

#[cfg(not(unix))]
fn own_process_group(_: &mut Command) {}

/// SIGINT or SIGTERM, if one came. The handler only stores the number.
static INTERRUPT: AtomicI32 = AtomicI32::new(0);

/// The signal that interrupted the command, if one did.
pub(crate) fn interrupt_signal() -> Option<i32> {
    match INTERRUPT.load(Ordering::SeqCst) {
        0 => None,
        n => Some(n),
    }
}

#[cfg(unix)]
unsafe extern "C" {
    fn signal(signum: i32, handler: extern "C" fn(i32)) -> usize;
    fn kill(pid: i32, sig: i32) -> i32;
}

#[cfg(unix)]
extern "C" fn on_signal(sig: i32) {
    INTERRUPT.store(sig, Ordering::SeqCst);
}

/// Catches SIGHUP, SIGINT and SIGTERM. A runner of `--ab` then kills the agent, removes
/// the copy, writes the result and exits with 128 plus the signal.
pub(crate) fn catch_interrupts() {
    #[cfg(unix)]
    // SAFETY: `signal` is the C function. The handler only stores an integer
    // in an atomic, which is safe in a signal handler.
    unsafe {
        signal(1, on_signal);
        signal(2, on_signal);
        signal(15, on_signal);
    }
}

/// Kills the process group that `own_process_group` made for the pid.
fn kill_group_of(pid: u32) {
    #[cfg(unix)]
    // SAFETY: `kill` is the C function. A negative pid names the group.
    unsafe {
        kill(-(pid as i32), 9);
    }
    #[cfg(not(unix))]
    let _ = pid;
}

/// Kills the process group of a child that `own_process_group` made.
fn kill_group(child: &mut Child) {
    kill_group_of(child.id());
    let _ = child.kill();
}

/// How a process ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Ended {
    /// The exit code, or -1 if a signal ended it.
    Exited(i32),
    TimedOut,
    Interrupted,
}

/// Waits for a child. It kills the group of the child at the time limit, and
/// when SIGINT or SIGTERM comes.
fn wait_limited(child: &mut Child, limit: Duration) -> std::io::Result<Ended> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            // A child of the process that is still alive must not hold the
            // pipes that the readers wait on. ESRCH (no process) is no error.
            kill_group_of(child.id());
            return Ok(Ended::Exited(status.code().unwrap_or(-1)));
        }
        if interrupt_signal().is_some() {
            kill_group(child);
            let _ = child.wait();
            return Ok(Ended::Interrupted);
        }
        if start.elapsed() >= limit {
            kill_group(child);
            let _ = child.wait();
            return Ok(Ended::TimedOut);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The most bytes of the stderr of `claude` that the runner keeps.
const MAX_STDERR: usize = 64 * 1024;

/// Copies the stderr of `claude` to the stderr of the runner, and keeps the
/// first bytes of it.
fn tee_stderr(mut r: impl Read) -> Vec<u8> {
    let mut kept = Vec::new();
    let mut buf = [0u8; 4096];
    let mut err = std::io::stderr();
    while let Ok(n) = r.read(&mut buf) {
        if n == 0 {
            break;
        }
        let _ = err.write_all(&buf[..n]);
        let room = MAX_STDERR.saturating_sub(kept.len());
        kept.extend_from_slice(&buf[..n.min(room)]);
    }
    kept
}

/// The most bytes of the lines that are no event, and that the runner keeps.
/// A plain text of an error (a refusal) is read from them.
const MAX_TEXT: usize = 64 * 1024;

/// Reads one line of at most `cap` bytes into `line`, with its end of line.
/// It discards the bytes of a longer line, and it returns the number of bytes
/// that it read and whether the line was longer than `cap`.
fn read_line_capped(
    r: &mut impl std::io::BufRead,
    line: &mut Vec<u8>,
    cap: usize,
) -> (usize, bool) {
    let (mut total, mut over) = (0, false);
    loop {
        let buf = match r.fill_buf() {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        if buf.is_empty() {
            break;
        }
        let (take, done) = match buf.iter().position(|b| *b == b'\n') {
            Some(at) => (at + 1, true),
            None => (buf.len(), false),
        };
        let room = cap.saturating_sub(line.len());
        over |= take > room;
        line.extend_from_slice(&buf[..take.min(room)]);
        r.consume(take);
        total += take;
        if done {
            break;
        }
    }
    (total, over)
}

/// What a line of the stream is.
enum Line {
    /// An event: a JSON object with a top-level `type` that is a string.
    Event(String),
    /// A JSON object that does not parse: its end is not there.
    Torn,
    /// Each other line: a plain text, or a JSON value that is no event.
    Text,
}

fn line_kind(line: &[u8]) -> Line {
    if !line.trim_ascii_start().starts_with(b"{") {
        return Line::Text;
    }
    match json::parse(String::from_utf8_lossy(line).trim()) {
        Ok(value @ Value::Obj(_)) => match value.get("type").as_str() {
            Some(kind) => Line::Event(kind.to_string()),
            None => Line::Text,
        },
        Ok(_) => Line::Text,
        Err(_) => Line::Torn,
    }
}

/// What the runner keeps of the stdout of `claude`.
#[derive(Default)]
struct Stream {
    /// The `result` events, one on each line, and the text.
    out: Vec<u8>,
    /// True if work followed the last `result` event: another event (a segment
    /// in flight), an object with no end (a cut line), or a line that the
    /// reader discarded. The last result then does not give the cost of the
    /// run.
    cut: bool,
}

/// Reads the stdout of `claude` line by line (`--output-format stream-json`,
/// D95). Of the events, it keeps the `result` events only: the top-level
/// `type` decides, so a tool result that holds the bytes of one does not
/// count. Another event is no text of the output: a run that ends with no
/// result event then has the class of a refusal of an empty output, from its
/// stderr. It also keeps the first `MAX_TEXT` bytes of the lines that are no
/// event, for the text of a refusal. It keeps their bytes as they are, with
/// their ends of line, because the limit of a refusal text counts them. So the
/// memory does not grow with the stream. It reads to the end, so the process
/// never waits on a full pipe. A line of more than `MAX_STDOUT` bytes is
/// discarded, and so is each result event after the first `MAX_STDOUT` bytes
/// of them.
fn read_events(r: impl Read) -> Stream {
    let mut r = std::io::BufReader::new(r);
    let (mut stream, mut line) = (Stream::default(), Vec::new());
    let (mut events, mut text) = (0usize, 0usize);
    loop {
        line.clear();
        let (n, over) = read_line_capped(&mut r, &mut line, MAX_STDOUT as usize);
        if n == 0 {
            break;
        }
        if over {
            stream.cut = true;
            continue;
        }
        match line_kind(&line) {
            Line::Event(kind) => {
                let keep = kind == "result" && events + line.len() < MAX_STDOUT as usize;
                stream.cut = !keep;
                if keep {
                    events += line.len() + 1;
                    // A text that the limit cut has no end of line.
                    if stream.out.last().is_some_and(|b| *b != b'\n') {
                        stream.out.push(b'\n');
                    }
                    stream.out.extend_from_slice(line.trim_ascii_end());
                    stream.out.push(b'\n');
                }
                continue;
            }
            Line::Torn => stream.cut = true,
            Line::Text => {}
        }
        let part = &line[..line.len().min(MAX_TEXT.saturating_sub(text))];
        text += part.len();
        stream.out.extend_from_slice(part);
    }
    stream
}

/// The first limit that a segment of the run stopped at: `error_max_turns` or
/// `error_max_budget_usd`. A later segment can succeed, and the run stopped at
/// the limit all the same (ADR 0033: a limit scores 0).
fn first_limit(stdout: &[u8]) -> Option<String> {
    result_events(stdout).iter().find_map(|event| {
        event
            .get("subtype")
            .as_str()
            .filter(|s| matches!(*s, "error_max_turns" | "error_max_budget_usd"))
            .map(str::to_string)
    })
}

/// The `result` events of the output of `claude`, in order: the lines that hold
/// the bytes `"result"`, whose top-level `type` is `result`.
fn result_events(stdout: &[u8]) -> Vec<Value> {
    String::from_utf8_lossy(stdout)
        .lines()
        .filter(|l| l.contains("\"result\""))
        .filter_map(|l| json::parse(l.trim()).ok())
        .filter(|v| matches!(v, Value::Obj(_)) && v.get("type").as_str() == Some("result"))
        .collect()
}

/// The last `result` event of the output of `claude`. The cost, the subtype,
/// the models and the tokens of a run are those of the session at its end.
fn result_object(stdout: &[u8]) -> Option<Value> {
    result_events(stdout).pop()
}

/// Reads `total_cost_usd` of the last result event of `claude`, and the
/// subtype of the run: the first limit of a segment, else the subtype of the
/// last event. Nothing else is read.
fn read_result(stdout: &[u8]) -> (Option<f64>, Option<String>) {
    let Some(obj) = result_object(stdout) else {
        return (None, None);
    };
    let cost = match obj.get("total_cost_usd") {
        Value::Num(n) if n.is_finite() && *n >= 0.0 => Some(*n),
        _ => None,
    };
    let subtype = obj
        .get("subtype")
        .as_str()
        .filter(|s| {
            (1..=40).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
        })
        .map(str::to_string);
    (cost, first_limit(stdout).or(subtype))
}

/// The sorted names of the models of `modelUsage` in the result of `claude`.
/// It reads names only, and a name has the form of a model name of a record.
fn result_models(stdout: &[u8]) -> Vec<String> {
    let Some(Value::Obj(fields)) = result_object(stdout).map(|o| o.get("modelUsage").clone())
    else {
        return Vec::new();
    };
    let mut names: Vec<String> = fields
        .into_iter()
        .map(|(k, _)| k)
        .filter(|k| crate::eval_records::valid_model(k))
        .collect();
    names.sort();
    names
}

/// The turns of a run (D95): the sum of `num_turns` over the result events,
/// one for each segment. Each count is an integer of 0 or more. The result is
/// `None` if there is no result event, or if one event has no valid count.
fn result_turns(stdout: &[u8]) -> Option<u32> {
    let events = result_events(stdout);
    if events.is_empty() {
        return None;
    }
    let mut sum = 0u32;
    for event in &events {
        match event.get("num_turns") {
            Value::Num(n) if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 && *n <= 1e6 => {
                sum = sum.checked_add(*n as u32)?;
            }
            _ => return None,
        }
    }
    Some(sum)
}

/// The sums over the entries of `modelUsage` of `inputTokens`, `outputTokens`,
/// `cacheReadInputTokens` and `cacheCreationInputTokens`. A field that an
/// entry does not have counts 0. The result is `None` if `modelUsage` is
/// absent or empty, an entry is no object, or a count is not an integer of 0
/// or more. Other fields (such as `thinkingTokens`) are not read.
pub(crate) fn result_tokens(stdout: &[u8]) -> Option<Tokens> {
    let Value::Obj(entries) = result_object(stdout)?.get("modelUsage").clone() else {
        return None;
    };
    if entries.is_empty() {
        return None;
    }
    let mut sums = [0u64; 4];
    for (_, entry) in &entries {
        if !matches!(entry, Value::Obj(_)) {
            return None;
        }
        for (sum, key) in sums.iter_mut().zip([
            "inputTokens",
            "outputTokens",
            "cacheReadInputTokens",
            "cacheCreationInputTokens",
        ]) {
            match entry.get(key) {
                Value::Null => {}
                Value::Num(n) if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 && *n < 9e15 => {
                    *sum = sum.checked_add(*n as u64)?;
                }
                _ => return None,
            }
        }
    }
    Some(Tokens {
        input: sums[0],
        output: sums[1],
        cache_read: sums[2],
        cache_write: sums[3],
    })
}

/// The most bytes of the text of an answer that is a refusal of the API. A
/// longer text is an answer of the agent that names a status.
const REFUSAL_TEXT_MAX: usize = 600;

/// The class of a refusal. An account limit, a credit or a login problem
/// lasts for hours, so it gets the long ladder of waits. The other refusals
/// (a 5xx status, a rate limit, a page of Cloudflare, a lost connection) get
/// the short one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Refusal {
    Account,
    Other,
    /// An error with no marker and no cost: a result object with an error
    /// and a cost of 0, or a plain text or no output with a non-zero exit. A
    /// real run of an agent never costs 0, so it is no run. The short ladder
    /// applies, and 5 in a row stop the command.
    Silent,
}

/// The phrases of a refusal of the account, in lower case.
const ACCOUNT_PHRASES: [&str; 7] = [
    "usage limit",
    "spend limit reached",
    "credit balance",
    "out of usage",
    "out of extra usage",
    "please run /login",
    "not logged in",
];

/// The phrases of the other refusals, in lower case: a rate limit, an
/// overloaded API, a page of Cloudflare and a failed connection. The lab
/// harness of session 15 gave the first ones. A phrase such as "limit
/// reached" alone is not here: `Context limit reached` is a run.
const OTHER_PHRASES: [&str; 18] = [
    "rate limit",
    "rate-limit",
    "rate_limit",
    "limit will reset",
    "overloaded",
    "failed to authenticate",
    "attention required",
    "cloudflare",
    "request timed out",
    "connection error",
    "unable to connect",
    "econnreset",
    "etimedout",
    "econnrefused",
    "enotfound",
    "socket hang up",
    "fetch failed",
    "could not resolve host",
];

/// True for the statuses of a refusal: 401, 403, 429 and 500 to 599.
fn is_refusal_status(status: u32) -> bool {
    matches!(status, 401 | 403 | 429 | 500..=599)
}

/// The rule of the owner for `eval run --ab` (ADR 0016, amendment of
/// 2026-11-16): an account limit, a credit or a login problem, 401, 403, 429, a
/// 5xx status, a page of Cloudflare or a lost connection refused the attempt.
/// It is no run, and no result counts it.
///
/// A result of `claude` with a `subtype` that starts with `error_max_` (the
/// agent ran out of turns or budget) is never a refusal, and nor is a run with
/// such a limit in any segment (`first_limit`). Nor is a run that exits with 0
/// and has no error flag. Else, by the last result event:
///
/// 1. A result object with `is_error` and an `api_error_status` of 401, 403,
///    429 or 500 to 599 is a refusal, whatever its text is.
/// 2. A result object with a short text (fewer than 600 bytes) that has a
///    marker is a refusal. The text is its `result`.
/// 3. With no result object and an empty stdout, a non-zero exit is a refusal
///    (decision D1). The stderr gives the class.
/// 4. With no result object and a stdout, a non-zero exit is a refusal. A
///    short text with a marker gives the class. Else it is `Silent`: an agent
///    never ends with a plain text and no cost.
/// 5. A result object with an error, no marker and a cost of 0 is `Silent`.
///
/// A marker is `API Error:` with a status of rule 1, the word `429`, `hit
/// your ... limit` (at most 40 characters between), `limit reached` with
/// `reset`, a window of the account (`5-hour limit`, `weekly limit`, `session
/// limit`, `opus limit`), or a phrase of `ACCOUNT_PHRASES` or `OTHER_PHRASES`,
/// in any case. A failed check and an error with a cost and no marker are
/// runs.
///
/// A real output looks like this (Claude Code 2.1.x):
/// `{"type":"result","subtype":"success","is_error":true,"api_error_status":403,"result":"Failed to authenticate. API Error: 403 Attention Required! | Cloudflare","total_cost_usd":0.5}`.
pub(crate) fn refusal(claude_exit: i32, stdout: &[u8], stderr: &[u8]) -> Option<Refusal> {
    let obj = result_object(stdout);
    let is_error = obj.as_ref().is_some_and(|o| o.get("is_error").is_true());
    // An agent that stopped at a limit in one segment did run.
    if (claude_exit == 0 && !is_error) || first_limit(stdout).is_some() {
        return None;
    }
    let short = |t: &str| {
        (t.len() < REFUSAL_TEXT_MAX)
            .then(|| marker_class(t))
            .flatten()
    };
    if let Some(o) = &obj {
        if o.get("subtype")
            .as_str()
            .is_some_and(|t| t.starts_with("error_max_"))
        {
            return None;
        }
        let by_text = short(o.get("result").as_str().unwrap_or(""));
        if is_error
            && let json::Value::Num(n) = o.get("api_error_status")
            && n.fract() == 0.0
            && *n >= 0.0
            && is_refusal_status(*n as u32)
        {
            return Some(by_text.unwrap_or(Refusal::Other));
        }
        if by_text.is_some() {
            return by_text;
        }
        // An error with a cost of 0 or no cost: no agent ran.
        let cost = match o.get("total_cost_usd") {
            json::Value::Num(n) => *n,
            _ => 0.0,
        };
        return (is_error && cost == 0.0).then_some(Refusal::Silent);
    }
    let out = String::from_utf8_lossy(stdout);
    if out.trim().is_empty() {
        let err = String::from_utf8_lossy(stderr);
        return Some(short(&err).unwrap_or(Refusal::Silent));
    }
    Some(short(&out).unwrap_or(Refusal::Silent))
}

/// True if the attempt was refused.
#[cfg(test)]
fn is_refused(claude_exit: i32, stdout: &[u8]) -> bool {
    refusal(claude_exit, stdout, b"").is_some()
}

/// The class of a short text, or `None` if it has no marker.
fn marker_class(text: &str) -> Option<Refusal> {
    let lower = text.to_ascii_lowercase();
    let window_named = [
        "5-hour limit",
        "weekly limit",
        "session limit",
        "opus limit",
    ]
    .iter()
    .any(|p| lower.contains(p));
    let account = ACCOUNT_PHRASES.iter().any(|p| lower.contains(p))
        || window_named
        || (lower.contains("limit reached") && lower.contains("reset"))
        || lower.find("hit your ").is_some_and(|at| {
            let tail = &lower[at + "hit your ".len()..];
            let window: String = tail.chars().take(40 + "limit".len()).collect();
            window.contains("limit")
        });
    if account {
        return Some(Refusal::Account);
    }
    if has_status_marker(text) || OTHER_PHRASES.iter().any(|p| lower.contains(p)) {
        return Some(Refusal::Other);
    }
    None
}

/// `API Error:` with a status of a refusal, or the word `429`.
fn has_status_marker(text: &str) -> bool {
    const API_ERROR: &str = "API Error:";
    let mut rest = text;
    while let Some(at) = rest.find(API_ERROR) {
        rest = &rest[at + API_ERROR.len()..];
        let digits: String = rest
            .trim_start()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        if digits.len() == 3 && digits.parse::<u32>().is_ok_and(is_refusal_status) {
            return true;
        }
    }
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(at) = text[from..].find("429") {
        let (a, b) = (from + at, from + at + 3);
        // A word: no letter or digit next to it, and not a part of a number
        // such as `0.429` or `429.5`.
        let digit = |i: usize| bytes.get(i).is_some_and(u8::is_ascii_digit);
        let before = a == 0
            || !(bytes[a - 1].is_ascii_alphanumeric()
                || (bytes[a - 1] == b'.' && a >= 2 && digit(a - 2)));
        let after = b >= bytes.len()
            || !(bytes[b].is_ascii_alphanumeric() || (bytes[b] == b'.' && digit(b + 1)));
        if before && after {
            return true;
        }
        from = a + 1;
    }
    false
}

/// The number of `permission_denials` of the result object of `claude`, if it
/// has the list.
fn result_denials(stdout: &[u8]) -> Option<u32> {
    match result_object(stdout)?.get("permission_denials") {
        Value::Arr(items) => Some(items.len() as u32),
        _ => None,
    }
}

/// The number of retries of a refused attempt of `--ab` with the short ladder.
pub(crate) const RETRIES: u32 = 6;
/// The first wait, in seconds, when `SPEC_DNA_EVAL_BACKOFF` is not set. The 6
/// waits are 60, 120, 240, 480, 960 and 1,920 seconds: 3,780 seconds, about 63
/// minutes.
const FIRST_WAIT: u64 = 60;
const BACKOFF_VAR: &str = "SPEC_DNA_EVAL_BACKOFF";

/// The first wait of the back-off: the value of `SPEC_DNA_EVAL_BACKOFF`, an
/// integer of seconds from 0 to 3,600, or 60. Another value is a usage error,
/// and `--ab` reads it before the first run.
pub(crate) fn first_wait() -> Result<u64, Fatal> {
    first_wait_of(std::env::var(BACKOFF_VAR).ok().as_deref())
}

fn first_wait_of(value: Option<&str>) -> Result<u64, Fatal> {
    let Some(v) = value else {
        return Ok(FIRST_WAIT);
    };
    v.parse::<u64>()
        .ok()
        .filter(|n| *n <= 3_600 && v.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(|| {
            Fatal::Usage(format!(
                "{BACKOFF_VAR}: `{v}` is not an integer of seconds from 0 to 3600"
            ))
        })
}

/// The waits before the retries of a refused attempt, in seconds. The first
/// 6 are the first wait, and then 2, 4, 8, 16 and 32 times it (60, 120, 240,
/// 480, 960 and 1,920 seconds, 63 minutes in total). For an account limit, 9
/// waits of 30 times the first wait follow (1,800 seconds), and a last wait
/// that fills the time up to 6 hours (1,620 seconds). A session window of 5
/// hours is then over before the last retry.
pub(crate) fn retry_waits(first: u64, account: bool) -> Vec<u64> {
    let mut waits: Vec<u64> = (0..RETRIES).map(|k| first << k).collect();
    if account {
        waits.extend([first * 30; 9]);
        waits.push(first * 27);
    }
    waits
}

/// Rounds to 6 decimals, so a sum of costs has no floating-point tail.
pub(crate) fn r6(x: f64) -> f64 {
    (x * 1e6).round() / 1e6
}

fn report_text(r: &Report) -> Vec<String> {
    let mut lines = Vec::new();
    for o in &r.outcomes {
        lines.push(if o.results.is_empty() {
            format!("{}: did not start", o.id)
        } else {
            let not_started = o.runs as usize - o.results.len();
            let note = if not_started > 0 {
                format!(", {not_started} runs did not start")
            } else {
                String::new()
            };
            format!(
                "{}: {} of {} runs passed{note}, pass-at {}: {}, {:.2} USD",
                o.id,
                o.passed_runs(),
                o.results.len(),
                o.pass_at,
                if o.passed() { "pass" } else { "fail" },
                o.spend()
            )
        });
    }
    if let Some(rate) = r.pass_rate() {
        lines.push(format!(
            "pass rate {:.1} percent ({} of {} evals), min-pass-rate {:.1} percent: {}",
            rate * 100.0,
            r.passed_evals(),
            r.outcomes.len(),
            r.min_pass_rate * 100.0,
            if r.passed() { "pass" } else { "fail" }
        ));
    }
    lines.push(format!("spend {:.2} USD", r.spend()));
    if let Some(n) = r.stopped {
        lines.push(match r.stopped_by {
            StopBy::Cap => format!(
                "stopped: the spend {:.2} USD is above max-total-usd {:.2} USD; {n} runs did not start",
                r.spend(),
                r.max_total_usd
            ),
            StopBy::Disk => format!("stopped: disk-space; {n} runs did not start"),
        });
    }
    lines
}

fn report_json(r: &Report) -> String {
    let evals = json::array(r.outcomes.iter().map(|o| {
        let results = json::array(o.results.iter().map(|x| {
            let checks = json::array(
                x.checks
                    .iter()
                    .map(|(name, ok)| Obj::new().str("name", name).bool("passed", *ok).end()),
            );
            let mut obj = Obj::new()
                .num("claude_exit", x.claude_exit)
                .opt_str("subtype", x.subtype.as_deref());
            obj = match x.cost {
                Some(c) => obj.num("cost_usd", r6(c)),
                None => obj.raw("cost_usd", "null"),
            };
            let obj = obj.raw("checks", &checks);
            let obj = match &x.tokens {
                Some(t) => obj.raw("tokens", &tokens_json(t)),
                None => obj.raw("tokens", "null"),
            };
            let obj = obj.strs("models", &x.models);
            let obj = match x.turns {
                Some(n) => obj.num("turns", n),
                None => obj.raw("turns", "null"),
            };
            let obj = match x.segments {
                Some(n) => obj.num("segments", n),
                None => obj.raw("segments", "null"),
            };
            obj.num("seconds", x.seconds).end()
        }));
        Obj::new()
            .str("id", &o.id)
            .num("runs", o.runs)
            .num("pass_at", o.pass_at)
            .num("max_budget_usd", r6(o.budget))
            .num("passed_runs", o.passed_runs())
            .bool("passed", o.passed())
            .num("spend_usd", r6(o.spend()))
            .raw("results", &results)
            .end()
    }));
    let mut obj = Obj::new()
        .str("spec_dna", VERSION)
        .opt_str("claude_code", r.claude_code.as_deref())
        .str("started", &r.started)
        .str("evals_dir", &r.evals_dir)
        .str("harness", r.harness.name())
        .num("min_pass_rate", r6(r.min_pass_rate))
        .num("max_total_usd", r6(r.max_total_usd))
        .num("estimate_usd", r6(r.estimate))
        .num("spend_usd", r6(r.spend()))
        .opt_str("stopped", r.stopped.map(|_| r.stopped_by.name()));
    obj = match r.pass_rate() {
        Some(rate) => obj.num("pass_rate", r6(rate)),
        None => obj.raw("pass_rate", "null"),
    };
    obj.bool("passed", r.passed()).raw("evals", &evals).end()
}

/// The JSON of the token counts of a run.
pub(crate) fn tokens_json(t: &Tokens) -> String {
    Obj::new()
        .num("in", t.input)
        .num("out", t.output)
        .num("cache_read", t.cache_read)
        .num("cache_write", t.cache_write)
        .end()
}

/// Makes the folder of `--out`, and checks that it is a folder.
pub(crate) fn make_out_dir(repo: &Repo, dir: &str) -> Result<(), Fatal> {
    let folder = repo.root.join(dir);
    fs::create_dir_all(&folder).map_err(|e| Fatal::Io(format!("cannot make `{dir}`: {e}")))?;
    if folder.is_dir() {
        Ok(())
    } else {
        Err(Fatal::Io(format!(
            "cannot make `{dir}`: it is not a folder"
        )))
    }
}

/// Writes the result to `<dir>/<prefix>-<YYYYMMDD>T<HHMMSS>Z.json`. If the file
/// exists, the name gets the first free suffix from `-2` to `-999`. Returns the path from the
/// repository root.
pub(crate) fn write_result(
    repo: &Repo,
    dir: &str,
    prefix: &str,
    started: &str,
    compact: &str,
) -> Result<String, Fatal> {
    let text = json::pretty(&json::parse(compact).map_err(Fatal::Io)?);
    let folder = repo.root.join(dir);
    make_out_dir(repo, dir)?;
    // `2026-11-15T10:15:00Z` gives `20261115T101500Z`.
    let stamp: String = started
        .chars()
        .filter(|c| !matches!(c, '-' | ':'))
        .collect();
    // The text goes to a temporary file first. A hidden name that does not end
    // with `.json` is no result file, so no reader sees a half-written file.
    let tmp = folder.join(format!(".{prefix}-{stamp}.{}.tmp", std::process::id()));
    fs::write(&tmp, text.as_bytes())
        .map_err(|e| Fatal::Io(format!("cannot write `{dir}`: {e}")))?;
    let result = place_result(&folder, &tmp, dir, prefix, &stamp);
    let _ = fs::remove_file(&tmp);
    result
}

/// Replaces the result file `rel` (from the repository root) with the pretty
/// text of `compact`. The text goes to a hidden temporary file in the same
/// folder, and a rename puts it in place, so a reader sees the old file or the
/// new one, never a half file.
pub(crate) fn replace_result(repo: &Repo, rel: &str, compact: &str) -> Result<(), Fatal> {
    let text = json::pretty(&json::parse(compact).map_err(Fatal::Io)?);
    let path = repo.root.join(rel);
    let tmp = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        std::process::id()
    ));
    fs::write(&tmp, text.as_bytes())
        .map_err(|e| Fatal::Io(format!("cannot write `{rel}`: {e}")))?;
    fs::rename(&tmp, &path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        Fatal::Io(format!("cannot write `{rel}`: {e}"))
    })
}

/// Gives the temporary file its final name. A hard link never replaces a file
/// that exists, so the name gets the first free suffix from `-2` to `-999`.
fn place_result(
    folder: &Path,
    tmp: &Path,
    dir: &str,
    prefix: &str,
    stamp: &str,
) -> Result<String, Fatal> {
    for n in 1..1000 {
        let name = if n == 1 {
            format!("{prefix}-{stamp}.json")
        } else {
            format!("{prefix}-{stamp}-{n}.json")
        };
        let path = folder.join(&name);
        match fs::hard_link(tmp, &path) {
            Ok(()) => return Ok(format!("{dir}/{name}")),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            // A file system with no hard links: rename, if the name is free.
            Err(_) if !path.exists() => {
                return fs::rename(tmp, &path)
                    .map(|()| format!("{dir}/{name}"))
                    .map_err(|e| Fatal::Io(format!("cannot write `{dir}/{name}`: {e}")));
            }
            Err(e) => return Err(Fatal::Io(format!("cannot write `{dir}/{name}`: {e}"))),
        }
    }
    Err(Fatal::Io(format!(
        "cannot find a free name for the result in `{dir}`"
    )))
}

pub(crate) fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// A time in seconds since 1970, as `YYYY-MM-DDTHH:MM:SSZ` in UTC.
pub(crate) fn utc_time(secs: i64) -> String {
    // "civil_from_days" (Howard Hinnant).
    let z = secs.div_euclid(86_400) + 719_468;
    let rem = secs.rem_euclid(86_400);
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem / 60 % 60,
        rem % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_time_is_utc() {
        assert_eq!(utc_time(0), "1970-01-01T00:00:00Z");
        assert_eq!(utc_time(946_684_800), "2000-01-01T00:00:00Z");
        assert_eq!(utc_time(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(utc_time(1_794_737_700), "2026-11-15T10:15:00Z");
        assert_eq!(utc_time(86_399), "1970-01-01T23:59:59Z");
    }

    #[test]
    fn a_result_object_gives_the_cost_and_the_subtype() {
        let r = read_result(
            br#"{"type":"result","subtype":"success","total_cost_usd":0.12,"result":"TEXT"}"#,
        );
        assert_eq!(r, (Some(0.12), Some("success".to_string())));
        let r = read_result(br#"{"type":"result","total_cost_usd":0,"subtype":"error_max_turns"}"#);
        assert_eq!(r, (Some(0.0), Some("error_max_turns".to_string())));
    }

    #[test]
    fn a_stream_of_events_gives_the_last_result() {
        let text = br#"{"type":"system","subtype":"init","total_cost_usd":9}
{"type":"assistant"}
{"type":"result","subtype":"error_max_turns","total_cost_usd":0.2}
{"type": "result", "subtype": "error_max_budget_usd", "total_cost_usd": 0.5}
"#;
        // The cost is that of the last event, and the subtype is the first
        // limit of a segment.
        assert_eq!(
            read_result(text),
            (Some(0.5), Some("error_max_turns".to_string()))
        );
        // With no limit, the subtype is that of the last event.
        let text = br#"{"type":"result","subtype":"error_during_execution","total_cost_usd":0.2}
{"type":"result","subtype":"success","total_cost_usd":0.3}
"#;
        assert_eq!(read_result(text), (Some(0.3), Some("success".to_string())));
        // No event `result`: nothing is read. An object with no `type`, and an
        // event of another type that holds the bytes, are no result.
        assert_eq!(read_result(br#"{"type":"system"}"#), (None, None));
        assert_eq!(
            read_result(br#"{"total_cost_usd":1,"subtype":"x"}"#),
            (None, None)
        );
        assert_eq!(
            read_result(br#"{"type":"user","text":"\"result\"","total_cost_usd":1}"#),
            (None, None)
        );
        assert_eq!(read_result(b"[]"), (None, None));
    }

    #[test]
    fn the_reader_keeps_the_result_events_and_a_head_of_the_other_lines() {
        let mut input = Vec::new();
        for i in 0..2000 {
            input.extend_from_slice(
                format!(
                    "{{\"type\":\"assistant\",\"n\":{i},\"pad\":\"{}\"}}\nplain {i} {}\n",
                    "x".repeat(900),
                    "y".repeat(900)
                )
                .as_bytes(),
            );
        }
        input.extend_from_slice(b"{\"type\":\"result\",\"total_cost_usd\":0.5}\n");
        input.extend_from_slice(b"{\"type\":\"result\",\"total_cost_usd\":0.7}");
        let kept = read_events(&input[..]).out;
        // 64 KiB of the plain lines and the 2 result events (the last has no
        // end of line, and the text before them is cut at the limit). No other
        // event is kept.
        assert!(kept.len() < MAX_TEXT + 200, "{}", kept.len());
        assert!(!String::from_utf8_lossy(&kept).contains("assistant"));
        assert_eq!(result_events(&kept).len(), 2);
        assert_eq!(read_result(&kept).0, Some(0.7));
        // A text keeps its bytes, with and with no end of line.
        for text in ["API Error: 502 Bad Gateway", "a\r\n\nb\n"] {
            assert_eq!(read_events(text.as_bytes()).out, text.as_bytes());
        }
        assert!(read_events(&b""[..]).out.is_empty());
    }

    #[test]
    fn a_stream_is_cut_when_work_follows_the_last_result() {
        const RESULT: &str = "{\"type\":\"result\",\"total_cost_usd\":0.2}";
        let cut = |text: String| read_events(text.as_bytes()).cut;
        for whole in [
            format!("{RESULT}\n"),
            RESULT.to_string(),
            format!("{RESULT}\n\na plain line\n[1]\n{{\"no\":\"type\"}}\n"),
            // A later result closes the segment that was in flight.
            format!("{RESULT}\n{{\"type\":\"assistant\"}}\n{RESULT}\n"),
            format!("{{\"type\":\"assist\n{RESULT}\n"),
            String::new(),
        ] {
            assert!(!cut(whole.clone()), "{whole}");
        }
        for broken in [
            format!("{RESULT}\n{{\"type\":\"system\",\"subtype\":\"init\"}}\n"),
            format!("{RESULT}\n{{\"type\":\"result\",\"total_cos"),
            format!("{RESULT}\n{{\"type\":\"assist\n"),
            format!("{RESULT}\n{{\"type\":\"assist\na plain line\n"),
            "{\"type\":\"assistant\"}\n".to_string(),
            "{\"type\":\"res".to_string(),
        ] {
            assert!(cut(broken.clone()), "{broken}");
        }
    }

    /// The class of an attempt that failed, as the runner reads it: the stream
    /// through the reader, and then `refusal`.
    fn class_of_stream(stdout: &[u8], stderr: &[u8]) -> Option<Refusal> {
        refusal(1, &read_events(stdout).out, stderr)
    }

    const INIT: &[u8] = b"{\"type\":\"system\",\"subtype\":\"init\"}\n";

    #[test]
    fn an_init_event_and_a_login_error_on_stderr_is_an_account_refusal() {
        let err = b"Not logged in. Please run /login";
        assert_eq!(class_of_stream(INIT, err), Some(Refusal::Account));
    }

    #[test]
    fn an_init_event_and_another_error_on_stderr_is_the_other_refusal() {
        let err = b"Error: read ECONNRESET";
        assert_eq!(class_of_stream(INIT, err), Some(Refusal::Other));
        // Each event of the stream, not the first one only. A tool result that
        // holds the words of a refusal is no text of the API.
        let events = b"{\"type\":\"system\",\"subtype\":\"init\"}\n{\"type\": \"assistant\", \"message\": {}}\n{\"type\":\"user\",\"text\":\"usage limit \\\"result\\\"\"}\n";
        assert_eq!(class_of_stream(events, err), Some(Refusal::Other));
        // A stderr with no marker is silent, as with an empty output.
        assert_eq!(
            class_of_stream(INIT, b"Segmentation fault"),
            Some(Refusal::Silent)
        );
    }

    #[test]
    fn empty_output_and_empty_stderr_is_silent() {
        assert_eq!(class_of_stream(b"", b""), Some(Refusal::Silent));
        assert_eq!(class_of_stream(INIT, b""), Some(Refusal::Silent));
    }

    #[test]
    fn a_line_that_is_no_event_is_still_the_text_of_a_refusal() {
        // A plain text after an event: the text decides, not stderr.
        let mut out = INIT.to_vec();
        out.extend_from_slice(b"Weekly limit reached, resets Mon 9am\n");
        assert_eq!(class_of_stream(&out, b""), Some(Refusal::Account));
        // A JSON value with no `type` is no event.
        assert_eq!(
            class_of_stream(b"{\"message\":\"API Error: 503 down\"}\n", b""),
            Some(Refusal::Other)
        );
        assert_eq!(
            class_of_stream(b"[\"overloaded\"]\n", b""),
            Some(Refusal::Other)
        );
        // A line that is cut short does not parse, so it is a text.
        assert_eq!(
            class_of_stream(b"{\"type\":\"assistant\",\"text\":\"ov", b""),
            Some(Refusal::Silent)
        );
    }

    #[test]
    fn a_text_of_a_refusal_keeps_the_length_of_the_output() {
        // 0.8.0 counted the bytes of stdout. A text of 599 bytes is short, and
        // a text of 600 bytes is not, with or with no end of line. An event
        // before the text is no part of it.
        let text = |len: usize| format!("API Error: 429 {}", "x".repeat(len - 15));
        for (len, end, class) in [
            (599, "", Refusal::Other),
            (598, "\n", Refusal::Other),
            (597, "\r\n", Refusal::Other),
            (600, "", Refusal::Silent),
            (599, "\n", Refusal::Silent),
        ] {
            let out = format!("{}{end}", text(len));
            assert_eq!(out.len(), len + end.len());
            let mut stream = INIT.to_vec();
            stream.extend_from_slice(out.as_bytes());
            assert_eq!(
                class_of_stream(out.as_bytes(), b""),
                Some(class),
                "{len} {end:?}"
            );
            assert_eq!(
                class_of_stream(&stream, b""),
                Some(class),
                "after an event: {len} {end:?}"
            );
        }
    }

    #[test]
    fn the_free_space_is_the_field_before_the_capacity() {
        let head = "Filesystem 1024-blocks Used Available Capacity Mounted on\n";
        let value = |line: &str| parse_df_available(&format!("{head}{line}\n"));
        assert_eq!(
            value("/dev/disk3s5 971350180 123 456789 21% /System/Volumes/Data"),
            Some(456_789)
        );
        // A name of a file system with spaces, and a mount point with spaces
        // and a percent sign: the first field with a percent sign decides.
        assert_eq!(
            value("my disk name 1000 200 800 20% /Volumes/My Disk"),
            Some(800)
        );
        assert_eq!(
            value("/dev/x 1000 200 800 20% /Volumes/My 5% Disk"),
            Some(800)
        );
        assert_eq!(value("/dev/x 1000 200 0 100% /"), Some(0));
        assert_eq!(
            value("/dev/x 99999999999 1 18446744073709551615 0% /"),
            Some(u64::MAX)
        );
        // No capacity, a capacity with no value before it, a value that is no
        // integer, and a short output: no value.
        assert_eq!(value("/dev/x 1000 200 800 - /mnt"), None);
        assert_eq!(value("20% /mnt"), None);
        assert_eq!(value("/dev/x 1000 200 many 20% /mnt"), None);
        assert_eq!(value("/dev/x 1000 200 -5 20% /mnt"), None);
        assert_eq!(value(""), None);
        assert_eq!(parse_df_available(""), None);
        assert_eq!(parse_df_available("df: no such folder\n"), None);
        assert_eq!(parse_df_available(head), None);
    }

    #[test]
    fn the_message_of_the_stop_ends_with_the_command_that_goes_on() {
        let low = Low {
            folder: "evals/results".to_string(),
            free_mib: 3,
            min_mib: 2048,
        };
        assert_eq!(
            low.message(None),
            "spec-dna eval run: stopped before a run: `evals/results` has 3 MiB free, below `[evals] min-free-mib` (2048); free space, then run the command again"
        );
        assert!(
            low.message(Some("evals/results/skills-1.json")).ends_with(
                "then run the command again with `--resume evals/results/skills-1.json`"
            )
        );
    }

    #[test]
    fn a_floor_of_0_never_calls_df() {
        // A folder that does not exist would give a warning if `df` ran.
        let mut floor = Floor::new(0, Some((PathBuf::from("/no/such/folder"), "x".into())));
        assert!(floor.low().is_none());
        assert!(!floor.off);
    }

    #[test]
    fn the_turns_are_the_sum_over_the_result_events() {
        let text = b"{\"type\":\"system\",\"num_turns\":50}\n{\"type\":\"result\",\"num_turns\":101}\n{\"type\":\"user\",\"x\":\"\\\"result\\\"\"}\n{\"type\":\"result\",\"num_turns\":8}\n{\"type\": \"result\", \"num_turns\": 0}\n";
        assert_eq!(result_turns(text), Some(109));
        assert_eq!(result_events(text).len(), 3);
        // One event with no valid count, or no result event: unknown.
        for bad in [
            &b"{\"type\":\"result\",\"num_turns\":2}\n{\"type\":\"result\"}\n"[..],
            b"{\"type\":\"result\",\"num_turns\":-1}\n",
            b"{\"type\":\"result\",\"num_turns\":1.5}\n",
            b"{\"type\":\"result\",\"num_turns\":\"2\"}\n",
            b"{\"type\":\"assistant\",\"num_turns\":2}\n",
            b"",
        ] {
            assert_eq!(result_turns(bad), None, "{}", String::from_utf8_lossy(bad));
        }
    }

    #[test]
    fn a_line_over_the_limit_is_discarded_and_the_next_line_is_read() {
        let mut input = Vec::new();
        input.extend_from_slice(b"{\"type\":\"assistant\",\"pad\":\"");
        input.resize(MAX_STDOUT as usize + 100, b'x');
        input.extend_from_slice(b"\"}\n{\"type\":\"result\",\"total_cost_usd\":0.4}\n");
        let stream = read_events(&input[..]);
        assert_eq!(read_result(&stream.out).0, Some(0.4));
        assert!(stream.out.len() < 100, "{}", stream.out.len());
        assert!(!stream.cut);
        // A line that the reader discards after the last result cuts the run:
        // it can be the result of a later segment.
        let at = input.len();
        input.resize(at + MAX_STDOUT as usize + 100, b'x');
        let stream = read_events(&input[..]);
        assert_eq!(read_result(&stream.out).0, Some(0.4));
        assert!(stream.cut);
    }

    #[test]
    fn a_bad_cost_or_subtype_is_dropped() {
        for bad in ["-1", "\"1\"", "null", "true", "1e999"] {
            let text =
                format!("{{\"type\":\"result\",\"total_cost_usd\":{bad},\"subtype\":\"ok\"}}");
            assert_eq!(read_result(text.as_bytes()).0, None, "{bad}");
        }
        for bad in [
            "\"Success\"",
            "\"a-b\"",
            "\"\"",
            "5",
            "\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"",
        ] {
            let text = format!("{{\"type\":\"result\",\"subtype\":{bad}}}");
            assert_eq!(read_result(text.as_bytes()).1, None, "{bad}");
        }
        assert_eq!(read_result(b"not json"), (None, None));
        assert_eq!(read_result(b""), (None, None));
        assert_eq!(read_result(b"5"), (None, None));
    }

    #[test]
    fn the_out_folder_is_checked() {
        assert_eq!(normalize_out("out", "evals").unwrap(), "out");
        assert_eq!(normalize_out("./out/", "evals").unwrap(), "out");
        assert_eq!(
            normalize_out("evals/results", "evals").unwrap(),
            "evals/results"
        );
        for bad in [
            "", ".", "..", "a/../b", "/abs", "a//b", "a\\b", "evals", "./evals/", "c:x",
        ] {
            assert!(
                matches!(normalize_out(bad, "evals"), Err(Fatal::Usage(_))),
                "{bad}"
            );
        }
    }

    #[test]
    fn rounding_removes_the_floating_point_tail() {
        assert_eq!(r6(0.1 + 0.2), 0.3);
        assert_eq!(r6(2.5), 2.5);
    }

    fn outcome(runs: u32, pass_at: u32, passes: &[bool]) -> Outcome {
        Outcome {
            id: "x".into(),
            runs,
            pass_at,
            budget: 1.0,
            results: passes
                .iter()
                .map(|p| RunResult {
                    claude_exit: 0,
                    subtype: None,
                    cost: Some(0.5),
                    reported: Some(0.5),
                    charged: 0.5,
                    checks: Vec::new(),
                    passed: *p,
                    class: None,
                    denials: None,
                    interrupted: false,
                    models: Vec::new(),
                    tokens: None,
                    turns: None,
                    segments: None,
                    seconds: 0.0,
                })
                .collect(),
        }
    }

    #[test]
    fn an_eval_passes_at_pass_at_runs() {
        assert!(outcome(3, 2, &[true, false, true]).passed());
        assert!(!outcome(3, 3, &[true, false, true]).passed());
        assert!(outcome(3, 1, &[false, false, true]).passed());
        assert!(!outcome(3, 1, &[false, false, false]).passed());
        // A run that did not start is not a pass.
        assert!(!outcome(3, 1, &[true]).passed());
    }

    #[test]
    fn the_suite_passes_at_min_pass_rate() {
        let mk = |passes: &[bool], min: f64| Report {
            claude_code: None,
            started: String::new(),
            evals_dir: "evals".into(),
            min_pass_rate: min,
            max_total_usd: 20.0,
            estimate: 0.0,
            outcomes: passes.iter().map(|p| outcome(1, 1, &[*p])).collect(),
            stopped: None,
            stopped_by: StopBy::Cap,
            harness: HarnessArm::Project,
        };
        assert!(mk(&[true, false], 0.5).passed());
        assert!(!mk(&[true, false], 0.51).passed());
        assert!(mk(&[true, true, true, true, false], 0.8).passed());
        assert!(
            mk(
                &[true; 7]
                    .iter()
                    .chain([false; 3].iter())
                    .copied()
                    .collect::<Vec<_>>(),
                0.7
            )
            .passed()
        );
        assert!(mk(&[false], 0.0).passed());
        assert!(!mk(&[false], 0.01).passed());
        let mut stopped = mk(&[true], 0.0);
        stopped.stopped = Some(2);
        assert!(!stopped.passed());
        assert_eq!(stopped.pass_rate(), None);
        assert_eq!(stopped.exit_code(), 3);
    }

    /// The output of `claude` for a failed result object.
    fn failed(text: &str) -> Vec<u8> {
        format!(
            "{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":true,\"result\":\"{text}\",\"total_cost_usd\":0.7}}"
        )
        .into_bytes()
    }

    #[test]
    fn each_class_of_a_refusal_is_found() {
        // The form of a real output of Claude Code is the first one.
        for text in [
            "Failed to authenticate. API Error: 403 Attention Required! | Cloudflare",
            "Failed to authenticate. API Error: 401 Invalid authentication credentials",
            "API Error: 429 Rate limit reached for requests",
            "API Error: 500 Internal server error",
            "API Error: 503 Service Unavailable",
            "API Error: 599 something",
            "API Error:529 Overloaded",
            "Too many requests (429)",
            "The server is overloaded",
            "You reached the usage limit of the plan",
            "You hit the RATE LIMIT",
            "Attention Required! | Cloudflare",
        ] {
            assert!(is_refused(1, &failed(text)), "{text}");
            // The error flag alone is enough, also with exit code 0.
            assert!(is_refused(0, &failed(text)), "{text}");
        }
        // A page of Cloudflare, with no result object.
        assert!(is_refused(
            1,
            b"<title>Attention Required! | Cloudflare</title>"
        ));
        assert!(is_refused(1, b"API Error: 502 Bad Gateway"));
        // The shapes of the lab harness of session 15, and the network.
        for text in [
            "You've hit your limit · resets 5pm (Europe/London)",
            "You've hit your weekly Opus limit",
            "Claude AI usage limit reached|1700000000",
            "5-hour usage limit reached, resets 3pm",
            "Spend limit reached",
            "Credit balance is too low",
            "You're out of usage credits",
            "You're out of extra usage",
            "Not logged in · Please run /login",
            "API Error: Unable to connect to API",
            "Connection error.",
            "Request timed out",
            "Overloaded",
        ] {
            assert!(is_refused(1, &failed(text)), "{text}");
        }
        assert!(is_refused(1, b"Error: read ECONNRESET"));
        assert!(is_refused(1, b"TypeError: fetch failed"));
        // An `is_error` result with a refusal status is a refusal, whatever the text.
        for status in [401, 403, 429, 500, 503, 529, 599] {
            let out = format!(
                "{{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":true,\"api_error_status\":{status},\"result\":\"{}\"}}",
                "x".repeat(900)
            );
            assert!(is_refused(1, out.as_bytes()), "{status}");
        }
        // A stream of events: the last result decides.
        let events = b"{\"type\":\"system\"}\n{\"type\":\"result\",\"is_error\":true,\"result\":\"API Error: 503 down\"}\n";
        assert!(is_refused(1, events));
    }

    #[test]
    fn a_run_that_failed_for_another_reason_is_no_refusal() {
        // Exit code 0 and no error flag: a short answer about a limit counts.
        let ok = br#"{"type":"result","subtype":"success","is_error":false,"result":"The rate limit of the API is 5 per second"}"#;
        assert!(!is_refused(0, ok));
        // A failed run with no marker: a stop by the turns, a failed check.
        assert!(!is_refused(1, &failed("Reached max turns")));
        assert!(!is_refused(
            1,
            br#"{"type":"result","subtype":"error_max_turns","is_error":true}"#
        ));
        // An empty output with a non-zero exit is a refusal (decision D1).
        assert!(is_refused(1, b""));
        assert!(is_refused(-1, b""));
        assert!(!is_refused(0, b""));
        // The windows of an account that `claude` names with a reset time.
        for text in [
            "5-hour limit reached ∙ resets 3pm",
            "Weekly limit reached ∙ resets Mon 9am",
            "Session limit reached, resets at 4pm",
            "Opus limit reached",
            "You have used your weekly limit",
        ] {
            assert_eq!(
                refusal(1, &failed(text), b""),
                Some(Refusal::Account),
                "{text}"
            );
            assert_eq!(
                refusal(1, text.as_bytes(), b""),
                Some(Refusal::Account),
                "{text}"
            );
            let plain = format!("{{\"type\":\"result\",\"is_error\":true,\"result\":\"{text}\"}}");
            assert_eq!(
                refusal(1, plain.as_bytes(), b""),
                Some(Refusal::Account),
                "{text}"
            );
        }
        assert!(refusal(1, &failed("Token limit reached for this file"), b"").is_none());
        assert!(
            refusal(1, b"Token limit reached for this file", b"")
                .is_some_and(|c| c != Refusal::Account)
        );
        // A limit of the context of the agent is no refusal: a run.
        assert!(!is_refused(1, &failed("Context limit reached")));
        assert!(!is_refused(1, &failed("Token limit reached for this file")));
        // A result object with no `result` text: the number of the cost is no
        // status, and the output is not read as text.
        assert!(!is_refused(
            1,
            br#"{"type":"result","subtype":"error_max_turns","is_error":true,"total_cost_usd":0.429}"#
        ));
        assert!(!is_refused(
            1,
            br#"{"type":"result","subtype":"error_max_turns","is_error":true,"num_turns":429}"#
        ));
        assert!(!is_refused(1, &failed("cost 0.429 USD, 429.5 tokens")));
        assert!(is_refused(1, &failed("Too many requests, 429.")));
        // A plain text with a non-zero exit and no marker: no agent ran.
        assert_eq!(refusal(-1, b"not json", b""), Some(Refusal::Silent));
        assert_eq!(
            refusal(1, b"Segmentation fault", b""),
            Some(Refusal::Silent)
        );
        // An error with a cost of 0 is silent, and an error with a cost is a run.
        let zero = br#"{"type":"result","is_error":true,"result":"Failed","total_cost_usd":0}"#;
        assert_eq!(refusal(1, zero, b""), Some(Refusal::Silent));
        let nocost = br#"{"type":"result","is_error":true,"result":"Failed"}"#;
        assert_eq!(refusal(1, nocost, b""), Some(Refusal::Silent));
        let cost = br#"{"type":"result","is_error":true,"result":"Failed","total_cost_usd":0.2}"#;
        assert_eq!(refusal(1, cost, b""), None);
        let max =
            br#"{"type":"result","subtype":"error_max_turns","is_error":true,"total_cost_usd":0}"#;
        assert_eq!(refusal(1, max, b""), None);
        // A status that is not a refusal, or that is not a whole word.
        // The agent ran out of turns or budget: a run, with any text.
        for subtype in ["error_max_turns", "error_max_budget_usd"] {
            let out = format!(
                "{{\"type\":\"result\",\"subtype\":\"{subtype}\",\"is_error\":true,\"api_error_status\":503,\"result\":\"API Error: 503 rate limit\"}}"
            );
            assert!(!is_refused(1, out.as_bytes()), "{subtype}");
        }
        // A status that is not a refusal, with no error flag, or on a result with exit code 0.
        for out in [
            r#"{"type":"result","is_error":true,"api_error_status":400,"result":"Bad request","total_cost_usd":0.2}"#,
            r#"{"type":"result","is_error":true,"api_error_status":404,"result":"Not found","total_cost_usd":0.2}"#,
            r#"{"type":"result","is_error":true,"api_error_status":null,"result":"Failed","total_cost_usd":0.2}"#,
        ] {
            assert!(!is_refused(1, out.as_bytes()), "{out}");
        }
        assert!(!is_refused(
            0,
            br#"{"type":"result","is_error":false,"api_error_status":429,"result":"done"}"#
        ));
        let far = format!("hit your target {} limit", "x".repeat(60));
        assert!(!is_refused(1, &failed(&far)));
        assert!(!is_refused(1, &failed("API Error: 400 Bad request")));
        assert!(!is_refused(1, &failed("API Error: 404 Not found")));
        assert!(!is_refused(1, &failed("API Error: 5030 odd")));
        assert!(!is_refused(1, &failed("API Error: 600 odd")));
        assert!(!is_refused(1, &failed("step 14290 failed")));
        assert!(!is_refused(1, &failed("a1429b")));
        // A long answer that names a status counts: 600 bytes or more.
        let long = format!("API Error: 429 {}", "x".repeat(600));
        assert!(!is_refused(1, &failed(&long)));
        let just_under = format!("API Error: 429 {}", "x".repeat(600 - 15 - 1));
        assert_eq!(just_under.len(), 599);
        assert!(is_refused(1, &failed(&just_under)));
        let at_limit = format!("API Error: 429 {}", "x".repeat(600 - 15));
        assert_eq!(at_limit.len(), 600);
        assert!(!is_refused(1, &failed(&at_limit)));
    }

    #[test]
    fn the_stderr_of_claude_is_read_when_the_output_is_empty() {
        assert!(refusal(1, b"", b"Error: read ECONNRESET").is_some());
        assert!(refusal(1, b"", b"getaddrinfo ENOTFOUND api.example.invalid").is_some());
        assert!(refusal(1, b"", b"").is_some());
        // The stderr of a run with a result object is not read.
        assert!(
            refusal(
                1,
                br#"{"type":"result","is_error":true,"result":"Failed","total_cost_usd":0.2}"#,
                b"ECONNRESET"
            )
            .is_none()
        );
        assert!(refusal(0, b"", b"ECONNRESET").is_none());
        assert_eq!(
            refusal(1, b"Segmentation fault", b"ECONNRESET"),
            Some(Refusal::Silent)
        );
    }

    #[test]
    fn an_account_limit_has_its_own_class() {
        for text in [
            "You've hit your limit · resets 5pm",
            "Claude AI usage limit reached|1700000000",
            "Spend limit reached",
            "Credit balance is too low",
            "You're out of usage credits",
            "You're out of extra usage",
            "Not logged in · Please run /login",
        ] {
            assert_eq!(
                refusal(1, &failed(text), b""),
                Some(Refusal::Account),
                "{text}"
            );
        }
        for text in [
            "Overloaded",
            "API Error: 503 down",
            "Request timed out",
            "Connection error.",
        ] {
            assert_eq!(
                refusal(1, &failed(text), b""),
                Some(Refusal::Other),
                "{text}"
            );
        }
        let out = br#"{"type":"result","is_error":true,"api_error_status":429,"result":"You've hit your limit"}"#;
        assert_eq!(refusal(1, out, b""), Some(Refusal::Account));
        let out = br#"{"type":"result","is_error":true,"api_error_status":503,"result":"x"}"#;
        assert_eq!(refusal(1, out, b""), Some(Refusal::Other));
    }

    #[test]
    fn the_ladder_of_an_account_limit_lasts_6_hours_and_the_other_one_63_minutes() {
        let short = retry_waits(60, false);
        assert_eq!(short, [60, 120, 240, 480, 960, 1_920]);
        assert_eq!(short.iter().sum::<u64>(), 3_780);
        let long = retry_waits(60, true);
        assert_eq!(long.len(), 16);
        assert_eq!(long[..6], short[..]);
        assert!(long[6..15].iter().all(|w| *w == 1_800));
        assert_eq!(long[15], 1_620);
        assert_eq!(long.iter().sum::<u64>(), 6 * 3_600);
        assert!(retry_waits(0, true).iter().all(|w| *w == 0));
        assert_eq!(retry_waits(0, true).len(), 16);
        assert_eq!(retry_waits(1, true)[6], 30);
    }

    #[test]
    fn the_waits_double_and_the_variable_is_checked() {
        let waits = retry_waits(60, false);
        assert_eq!(waits, [60, 120, 240, 480, 960, 1_920]);
        assert_eq!(waits.iter().sum::<u64>(), 3_780, "about 63 minutes");
        assert_eq!(first_wait_of(None).unwrap(), 60);
        assert_eq!(first_wait_of(Some("0")).unwrap(), 0);
        assert_eq!(first_wait_of(Some("3600")).unwrap(), 3_600);
        for bad in ["", "-1", "3601", "1.5", "abc", "+5", " 5", "1e2"] {
            assert!(
                matches!(first_wait_of(Some(bad)), Err(Fatal::Usage(_))),
                "{bad}"
            );
        }
    }

    #[test]
    fn the_models_are_the_names_of_model_usage() {
        let out = br#"{"type":"result","modelUsage":{"claude-b-1":{"costUSD":1},"claude-a-2":{}}}"#;
        assert_eq!(result_models(out), ["claude-a-2", "claude-b-1"]);
        assert!(result_models(br#"{"type":"result"}"#).is_empty());
        assert!(result_models(b"x").is_empty());
        // The tokens: sums over the models; an absent field is 0; `thinkingTokens`
        // is not added.
        let t = result_tokens(
        br#"{"type":"result","modelUsage":{"a":{"inputTokens":1,"outputTokens":2,"cacheReadInputTokens":3,"cacheCreationInputTokens":4,"thinkingTokens":9},"b":{"inputTokens":10}}}"#,
    );
        assert_eq!(
            t,
            Some(Tokens {
                input: 11,
                output: 2,
                cache_read: 3,
                cache_write: 4
            })
        );
        for bad in [
            &br#"{"type":"result"}"#[..],
            b"x",
            br#"{"type":"result","modelUsage":{}}"#,
            br#"{"type":"result","modelUsage":[]}"#,
            br#"{"type":"result","modelUsage":{"a":3}}"#,
            br#"{"type":"result","modelUsage":{"a":{"inputTokens":-1}}}"#,
            br#"{"type":"result","modelUsage":{"a":{"inputTokens":1.5}}}"#,
            br#"{"type":"result","modelUsage":{"a":{"inputTokens":"1"}}}"#,
            // An object with no type is no result event.
            br#"{"modelUsage":{"a":{"inputTokens":1}}}"#,
        ] {
            assert_eq!(result_tokens(bad), None, "{}", String::from_utf8_lossy(bad));
        }
        assert!(result_models(br#"{"type":"result","modelUsage":{"a b":{}}}"#).is_empty());
        // A name with brackets or a slash is kept; a path-like name is not.
        assert_eq!(
            result_models(
                br#"{"type":"result","modelUsage":{"claude-fable-5-2[1m]":{},"v/m:1":{},"a..b":{},"/etc/x":{}}}"#
            ),
            ["claude-fable-5-2[1m]", "v/m:1"]
        );
    }
}
