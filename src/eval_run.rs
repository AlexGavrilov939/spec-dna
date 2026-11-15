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
use crate::json::{self, Obj, Value};
use crate::repo::{Fatal, Repo};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The tolerance of a comparison of USD sums. A sum of floating-point costs
/// such as 3 x 0.1 must not give a false refusal.
const EPS: f64 = 1e-9;
/// The most bytes of the stdout of `claude` that the runner keeps in memory.
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
}

/// An eval with the values that `eval run` uses: the `--runs` option replaces
/// the runs of the file, and the pass-at cannot be more than the runs.
struct Planned {
    eval: Eval,
    runs: u32,
    pass_at: u32,
    /// The files of the fixture: the source, and the path in the copy.
    fixture: Vec<(PathBuf, String)>,
}

/// The result of one run.
struct RunResult {
    claude_exit: i32,
    subtype: Option<String>,
    /// The `total_cost_usd` of `claude`, if it gave one.
    cost: Option<f64>,
    /// The cost that the cap counts: the cost, or the budget of the run.
    charged: f64,
    checks: Vec<(String, bool)>,
    passed: bool,
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
        self.results.iter().map(|r| r.charged).sum()
    }

    /// An eval passes when its passed runs are at least the pass-at. An eval
    /// that did not run all its runs (a stop by the cap) does not pass.
    fn passed(&self) -> bool {
        self.results.len() as u32 == self.runs && self.passed_runs() >= self.pass_at
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
    /// The number of runs that did not start, after a stop by the cap.
    stopped: Option<u32>,
}

impl Report {
    fn spend(&self) -> f64 {
        self.outcomes.iter().map(Outcome::spend).sum()
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
        let pass_at = eval.pass_at.min(runs);
        let fixture = copy_list(
            repo,
            &format!(":(literal){}", eval.fixture),
            &format!("{}/", eval.fixture),
        )?;
        plan.push(Planned {
            eval,
            runs,
            pass_at,
            fixture,
        });
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
    };
    let mut ctx = Ctx {
        harness,
        check_sh,
        isolate: o.isolate,
        counter: 0,
    };
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
            eprintln!("eval run: {} run {n} of {}", p.eval.id, p.runs);
            let result = match one_run(&mut ctx, p) {
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
        let path = write_result(repo, &dir, &report.started, &compact)?;
        eprintln!("eval run: the result is in {path}");
    }
    Ok(report.exit_code())
}

/// Checks the value of `--out`: a folder below the repository root, and not
/// `[evals] dir`, because the check SD506 reads each `*.json` file there.
fn normalize_out(path: &str, dir: &str) -> Result<String, Fatal> {
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
fn load_evals(repo: &Repo, table: &Evals) -> Result<Vec<Eval>, Fatal> {
    let dir = &table.dir;
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
        return Err(Fatal::Config(format!(
            "no eval file in `{dir}`; an eval is the file `{dir}/<id>.json`"
        )));
    }
    evals.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(evals)
}

fn read_limited(path: &Path, limit: u64) -> std::io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    fs::File::open(path)?.take(limit).read_to_end(&mut buf)?;
    Ok(buf)
}

/// Selects the evals of the ids. With no id, it selects each eval.
fn select(all: Vec<Eval>, ids: &[String], dir: &str) -> Result<Vec<Eval>, Fatal> {
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
fn copy_list(repo: &Repo, spec: &str, strip: &str) -> Result<Vec<(PathBuf, String)>, Fatal> {
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
fn harness_list(repo: &Repo) -> Result<Vec<(PathBuf, String)>, Fatal> {
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
fn claude_version() -> Result<Option<String>, Fatal> {
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
struct Ctx {
    harness: Vec<(PathBuf, String)>,
    /// The absolute path of `<dir>/check.sh` of the project.
    check_sh: PathBuf,
    isolate: bool,
    counter: u32,
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
        fs::copy(src, &dst).map_err(|e| Fatal::Io(format!("cannot copy `{rel}`: {e}")))?;
    }
    Ok(())
}

/// Makes the copy, runs `claude` and the checks, and removes the copy.
fn one_run(ctx: &mut Ctx, p: &Planned) -> Result<RunResult, Fatal> {
    let temp = make_temp(&mut ctx.counter)?;
    let dir = temp.0.as_path();
    copy_files(dir, &p.fixture)?;
    copy_files(dir, &ctx.harness)?;
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
    let base = git_in(dir, &["rev-parse", "HEAD"])?;
    if let Some(branch) = p.eval.switch_branch() {
        git_in(dir, &["switch", "-q", "-c", branch])?;
    }

    let mut cmd = Command::new("claude");
    cmd.arg("-p").arg(&p.eval.task);
    if !p.eval.allowed_tools.is_empty() {
        cmd.arg("--allowedTools").args(&p.eval.allowed_tools);
    }
    cmd.args([
        "--output-format",
        "json",
        "--max-turns",
        &p.eval.max_turns.to_string(),
        "--max-budget-usd",
        &p.eval.max_budget_usd.to_string(),
        "--permission-mode",
        "dontAsk",
    ]);
    if ctx.isolate {
        cmd.args(["--setting-sources", "project"]);
    }
    cmd.current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    clean_env(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| start_failed(&e))?;
    let stdout = read_capped(child.stdout.take().expect("stdout is piped"));
    let status = child
        .wait()
        .map_err(|e| Fatal::Io(format!("cannot wait for `claude`: {e}")))?;
    let claude_exit = status.code().unwrap_or(-1);
    let (cost, subtype) = read_result(&stdout);

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
        let ok = cmd
            .status()
            .map_err(|e| Fatal::Io(format!("cannot run `sh`: {e}")))?
            .success();
        checks.push((c.name.clone(), ok));
    }
    drop(temp);
    let passed = claude_exit == 0 && checks.iter().all(|c| c.1);
    Ok(RunResult {
        claude_exit,
        subtype,
        cost,
        charged: cost.unwrap_or(p.eval.max_budget_usd),
        checks,
        passed,
    })
}

/// Reads at most `MAX_STDOUT` bytes, and discards the rest, so the process
/// never waits on a full pipe.
fn read_capped(mut r: impl Read) -> Vec<u8> {
    let mut buf = Vec::new();
    let _ = (&mut r).take(MAX_STDOUT).read_to_end(&mut buf);
    let _ = std::io::copy(&mut r, &mut std::io::sink());
    buf
}

/// Reads `total_cost_usd` and `subtype` of the result of `claude`. The result
/// is one object, or a list of events with the object `result` at its end.
/// Nothing else is read.
fn read_result(stdout: &[u8]) -> (Option<f64>, Option<String>) {
    let text = String::from_utf8_lossy(stdout);
    let Ok(value) = json::parse(text.trim()) else {
        return (None, None);
    };
    let obj = match &value {
        Value::Arr(items) => {
            match items
                .iter()
                .rev()
                .find(|v| v.get("type").as_str() == Some("result"))
            {
                Some(v) => v,
                None => return (None, None),
            }
        }
        v @ Value::Obj(_) => v,
        _ => return (None, None),
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
    (cost, subtype)
}

/// Rounds to 6 decimals, so a sum of costs has no floating-point tail.
fn r6(x: f64) -> f64 {
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
        lines.push(format!(
            "stopped: the spend {:.2} USD is above max-total-usd {:.2} USD; {n} runs did not start",
            r.spend(),
            r.max_total_usd
        ));
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
            obj.raw("checks", &checks).end()
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
        .num("min_pass_rate", r6(r.min_pass_rate))
        .num("max_total_usd", r6(r.max_total_usd))
        .num("estimate_usd", r6(r.estimate))
        .num("spend_usd", r6(r.spend()))
        .opt_str("stopped", r.stopped.map(|_| "max-total-usd"));
    obj = match r.pass_rate() {
        Some(rate) => obj.num("pass_rate", r6(rate)),
        None => obj.raw("pass_rate", "null"),
    };
    obj.bool("passed", r.passed()).raw("evals", &evals).end()
}

/// Writes the result to `<dir>/eval-<YYYYMMDD>T<HHMMSS>Z.json`. If the file
/// exists, the name gets `-2`, `-3` and so on. Returns the path from the
/// repository root.
fn write_result(repo: &Repo, dir: &str, started: &str, compact: &str) -> Result<String, Fatal> {
    let text = json::pretty(&json::parse(compact).map_err(Fatal::Io)?);
    let folder = repo.root.join(dir);
    fs::create_dir_all(&folder).map_err(|e| Fatal::Io(format!("cannot make `{dir}`: {e}")))?;
    // `2026-11-15T10:15:00Z` gives `20261115T101500Z`.
    let stamp: String = started
        .chars()
        .filter(|c| !matches!(c, '-' | ':'))
        .collect();
    for n in 1..1000 {
        let name = if n == 1 {
            format!("eval-{stamp}.json")
        } else {
            format!("eval-{stamp}-{n}.json")
        };
        let path = folder.join(&name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                use std::io::Write;
                f.write_all(text.as_bytes())
                    .map_err(|e| Fatal::Io(format!("cannot write `{dir}/{name}`: {e}")))?;
                return Ok(format!("{dir}/{name}"));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(Fatal::Io(format!("cannot write `{dir}/{name}`: {e}"))),
        }
    }
    Err(Fatal::Io(format!(
        "cannot find a free name for the result in `{dir}`"
    )))
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// A time in seconds since 1970, as `YYYY-MM-DDTHH:MM:SSZ` in UTC.
fn utc_time(secs: i64) -> String {
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
        let r = read_result(br#"{"total_cost_usd":0,"subtype":"error_max_turns"}"#);
        assert_eq!(r, (Some(0.0), Some("error_max_turns".to_string())));
    }

    #[test]
    fn a_list_of_events_gives_the_last_result() {
        let text = br#"[{"type":"system","subtype":"init","total_cost_usd":9},{"type":"assistant"},{"type":"result","subtype":"error_max_budget_usd","total_cost_usd":0.5}]"#;
        assert_eq!(
            read_result(text),
            (Some(0.5), Some("error_max_budget_usd".to_string()))
        );
        // No event `result`: nothing is read.
        assert_eq!(read_result(br#"[{"type":"system"}]"#), (None, None));
        assert_eq!(read_result(b"[]"), (None, None));
    }

    #[test]
    fn a_bad_cost_or_subtype_is_dropped() {
        for bad in ["-1", "\"1\"", "null", "true", "1e999"] {
            let text = format!("{{\"total_cost_usd\":{bad},\"subtype\":\"ok\"}}");
            assert_eq!(read_result(text.as_bytes()).0, None, "{bad}");
        }
        for bad in [
            "\"Success\"",
            "\"a-b\"",
            "\"\"",
            "5",
            "\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"",
        ] {
            let text = format!("{{\"subtype\":{bad}}}");
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
                    charged: 0.5,
                    checks: Vec::new(),
                    passed: *p,
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
}
