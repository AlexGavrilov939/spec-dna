//! `spec-dna eval run --ab <target>`: the evals of the pack in 2 arms
//! (ADR 0016, 0.7.0 design D32 to D35). The runner of `eval_run.rs` makes each
//! run. This module chooses the evals, changes the temporary copy for the arm,
//! pairs the runs, retries a run that the API refused, and writes the result.
//!
//! - `with`: the copy has the skill folder, also when the skill is off in the
//!   configuration. The text is the text of `skill show`.
//! - `without`: the copy has no skill folder, no discipline of the skill in
//!   the line of the instruction files and in the adapter skill, and the
//!   command denies `Bash(spec-dna skill:*)`.
//! - The pair `agents-line` (D34) has no skill folder in each arm. `with`
//!   has the generated line in `AGENTS.md` and `CLAUDE.md`, and `without` has
//!   no line and denies the command.
//! - The target `accept` (design 0.8.0, D62 and D76 item 3) runs the 5 evals
//!   `accept-1` to `accept-5`, which have the skill `review`. `with` is as for
//!   a skill. `without` has the text of `review` of 0.7.1, no agent file
//!   `acceptor.md` and the same denial. Both arms have the skill `verify`.
//!
//! The result keeps counts, costs, names and versions. It keeps no task text,
//! no check command and no output text of `claude`.

use crate::adapters::{self, VERSION};
use crate::config::{Config, Evals, Profile};
use crate::config_text::{insert_keys, k, set_value};
use crate::eval_run::{
    Ctx, EPS, Floor, HarnessArm, Limits, Low, Planned, Refusal, RunResult, Tokens,
    catch_interrupts, claude_version, first_wait, harness_list, interrupt_signal, load_evals,
    load_evals_in, make_out_dir, normalize_out, now_secs, one_attempt, plan_eval, r6,
    replace_result, retry_waits, select as select_ids, tokens_json, utc_time,
};
use crate::evals::{self, Eval};
use crate::harness::{self, Level, Skill};
use crate::harness_text;
use crate::json::{self, Obj};
use crate::repo::{Fatal, Repo};
use crate::verdict::{self, AGENTS_LINE, Analysis, Pair};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// The folder of the pack, below `[evals] dir`.
pub const PACK: &str = "pack";
/// The command that the arm `without` denies.
const DENIED: &str = "Bash(spec-dna skill:*)";
/// The target of the 5 evals `accept-1` to `accept-5` (design 0.8.0, D62). Their
/// key `skill` is `review`, so the result names the skill `review`, and the
/// target `review` keeps the 5 evals of 0.7.1.
pub const ACCEPT: &str = "accept";
/// True for an eval of the target `accept`: its id is `accept-<n>`.
fn is_accept(e: &Eval) -> bool {
    e.id.starts_with("accept-")
}
/// The evals of the pair `agents-line`: the first eval of `tdd` and `debug`,
/// the last eval of `execute` (a plain edit with a commit), and 2 evals that
/// name the group (`skill = "agents-line"`).
pub const AGENTS_LINE_EVALS: [&str; 5] = [
    "tdd-1",
    "debug-1",
    "execute-10",
    "agents-line-1",
    "agents-line-2",
];

/// The options of `eval run --ab`.
pub struct Opts {
    pub target: String,
    pub runs: Option<u32>,
    pub max_total_usd: Option<f64>,
    pub out: Option<String>,
    pub dry_run: bool,
    pub isolate: bool,
    pub json: bool,
    /// The result file of an earlier run of the same target to go on with.
    pub resume: Option<String>,
    /// `--harness <arm>` (D57). `Project` keeps the copy of 0.7.1.
    pub harness: HarnessArm,
    /// True when `--harness` is set (see `eval_run::Opts::named`).
    pub named: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Arm {
    With,
    Without,
}

impl Arm {
    fn index(self) -> usize {
        match self {
            Arm::With => 0,
            Arm::Without => 1,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Arm::With => "with",
            Arm::Without => "without",
        }
    }
}

/// What a group of evals measures.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Skill(Skill),
    AgentsLine,
    /// The 5 evals `accept-1` to `accept-5` (design 0.8.0, D62). They measure
    /// the skill `review` of this release against the text of 0.7.1.
    Accept,
}

/// The evals of one skill, or the 5 evals of `agents-line`.
struct Group {
    name: String,
    kind: Kind,
    evals: Vec<Planned>,
}

/// The side of one pair: the last attempt of one arm.
#[derive(Clone)]
struct Side {
    claude_exit: i32,
    subtype: Option<String>,
    /// The `total_cost_usd` of `claude`, if it gave one. A run with no cost
    /// counts the budget of its eval.
    cost: Option<f64>,
    excluded: u32,
    denials: Option<u32>,
    passed: bool,
    /// The sums of the token counts of the run, if `claude` gave them.
    tokens: Option<Tokens>,
    /// The sorted names of the models of the run.
    models: Vec<String>,
    /// The name and the result of each check. No command, no output.
    checks: Vec<(String, bool)>,
    /// The `num_turns` of the run, if `claude` gave it (D58).
    turns: Option<u32>,
    /// The number of result events of the run, its segments (D95). A side of
    /// a result of 0.8.0 has none.
    segments: Option<u32>,
    /// The wall time of the agent process. A side of a result of 0.7.1 has
    /// none.
    seconds: Option<f64>,
}

/// One eval and run number, with the 2 arms. An arm that did not run is
/// `None`: the run stopped, or it is still to do.
#[derive(Clone)]
struct PairRec {
    eval: String,
    run: u32,
    with: Option<Side>,
    without: Option<Side>,
}

impl PairRec {
    fn side(&self, arm: Arm) -> &Option<Side> {
        match arm {
            Arm::With => &self.with,
            Arm::Without => &self.without,
        }
    }

    fn side_mut(&mut self, arm: Arm) -> &mut Option<Side> {
        match arm {
            Arm::With => &mut self.with,
            Arm::Without => &mut self.without,
        }
    }

    fn complete(&self) -> bool {
        self.with.is_some() && self.without.is_some()
    }
}

/// Why a run stopped.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    MaxTotal,
    ApiError,
    /// A git or file error ended the run. The error is in the result.
    Error,
    /// SIGINT or SIGTERM ended the run.
    Interrupted,
    /// A folder is below `[evals] min-free-mib` (D100).
    DiskSpace,
}

impl Stop {
    fn name(self) -> &'static str {
        match self {
            Stop::DiskSpace => "disk-space",
            Stop::MaxTotal => "max-total-usd",
            Stop::ApiError => "api-error",
            Stop::Error => "error",
            Stop::Interrupted => "interrupted",
        }
    }
}

/// A target of the command line.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Target {
    Skill(String),
    All,
    AgentsLine,
    /// The 5 evals of the acceptance record (`accept-1` to `accept-5`).
    Accept,
}

impl Target {
    /// Reads the target: a skill, `all`, `agents-line` or `accept`. Anything
    /// else is a usage error.
    pub fn parse(s: &str) -> Result<Target, Fatal> {
        if s == "all" {
            Ok(Target::All)
        } else if s == AGENTS_LINE {
            Ok(Target::AgentsLine)
        } else if s == ACCEPT {
            Ok(Target::Accept)
        } else if Skill::from_key(s).is_some() {
            Ok(Target::Skill(s.to_string()))
        } else {
            Err(Fatal::Usage(format!(
                "`{s}` is no target of `eval run --ab`; use a skill ({}), `all`, `{AGENTS_LINE}` or `{ACCEPT}`",
                crate::harness::skill_keys()
            )))
        }
    }
}

/// The target `accept` needs a built-in harness arm (design 0.8.0, D62). Without
/// `--harness`, the copy has the files of the host, and the 2 arms would not have
/// the same skills (`verify`). The arm `none` writes no skill.
pub fn check_harness(target: &Target, harness: HarnessArm) -> Result<(), Fatal> {
    if *target == Target::Accept && matches!(harness, HarnessArm::Project | HarnessArm::None) {
        return Err(Fatal::Usage(
            "the target `accept` needs `--harness lean`, `lean-no-advisor` or `full`: without a harness arm the copies have the files of the host, and the 2 arms do not have the same skills"
                .into(),
        ));
    }
    Ok(())
}

/// The checks of the target `accept` read the record with `python3` and its
/// module `tomllib` (Python 3.11 or later). A `python3` without it fails each
/// check, so a paid run would look like 0 passes. Exit code 2, before any run.
fn preflight_tomllib() -> Result<(), Fatal> {
    let ok = std::process::Command::new("python3")
        .args(["-c", "import tomllib"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if ok {
        return Ok(());
    }
    Err(Fatal::Usage(
        "the checks of the target `accept` need `python3` with the module `tomllib` (Python 3.11 or later), and `python3 -c 'import tomllib'` fails; install it, then run the command again"
            .into(),
    ))
}

/// One line on the standard error: the pin that the runner writes into the
/// `spec-dna.toml` of each copy. It is the version of this binary.
fn announce_pin() {
    eprintln!("eval run: pin of the copies: spec-dna = \"{VERSION}\"");
}

pub fn run(repo: &Repo, cfg: &Config, o: &Opts) -> Result<u8, Fatal> {
    let target = Target::parse(&o.target)?;
    let first = first_wait()?;
    let limits = Limits::from_env()?;
    if o.resume.is_some() && (o.dry_run || o.out.is_some()) {
        return Err(Fatal::Usage(
            "--resume goes on in its own file; it takes no --dry-run and no --out".into(),
        ));
    }
    let table = cfg.evals.as_ref().ok_or_else(|| {
        Fatal::Config(
            "spec-dna.toml has no [evals] table, and `eval run --ab` needs it; add the table (docs/config.md)"
                .into(),
        )
    })?;
    let out_dir = match o.out.as_deref() {
        Some(p) => normalize_out(p, &table.dir)?,
        None => format!("{}/{}", table.dir, evals::RESULTS),
    };
    let cap = o.max_total_usd.unwrap_or(table.max_total_usd);

    let check_sh = repo.root.join(&table.dir).join(evals::CHECK_FILE);
    let pack_dir = format!("{}/{PACK}", table.dir);
    let all = load_evals_in(repo, table, &pack_dir, &|dir| {
        format!("no eval file in `{dir}`; run `spec-dna eval pack` to write the pack")
    })?;
    if !check_sh.is_file() {
        return Err(Fatal::Config(format!(
            "{}/{}: the check file is absent; `eval run` sources it",
            table.dir,
            evals::CHECK_FILE
        )));
    }
    // `--resume` goes on with the `runs` of its file. A higher `--runs` adds
    // runs to each eval, and the file keeps the new value. A lower `--runs`,
    // or `--runs` with a file that kept the runs of each eval, is a usage
    // error, because the pairs of the file would not fit the plan.
    let resumed = match &o.resume {
        Some(file) => Some(read_resume_file(repo, file, &o.target)?),
        None => None,
    };
    // A file of 0.7.1 has no `harness`: it is the arm `project`.
    if let (Some(file), Some(v)) = (&o.resume, &resumed) {
        let had = v.get("harness").as_str().unwrap_or("project");
        if had != o.harness.name() {
            return Err(Fatal::Usage(format!(
                "--resume `{file}`: the file has the harness `{had}`, and this run has `{}`; use the same --harness",
                o.harness.name()
            )));
        }
    }
    let runs = match &resumed {
        Some(v) => {
            let file_runs = match v.get("runs") {
                json::Value::Num(n) => Some(*n as u32),
                _ => None,
            };
            match (o.runs, file_runs) {
                (None, f) => f,
                (Some(n), Some(f)) if n >= f => Some(n),
                (Some(n), Some(f)) => {
                    return Err(Fatal::Usage(format!(
                        "--runs {n} is below the file of --resume, which has --runs {f}; use {f} or more, or leave --runs out"
                    )));
                }
                (Some(_), None) => {
                    return Err(Fatal::Usage(
                        "--runs with a file of --resume that kept the runs of each eval; leave --runs out".to_string(),
                    ));
                }
            }
        }
        None => o.runs,
    };
    let groups = select(repo, all, &target, runs, &pack_dir)?;

    let total_runs: u32 = groups
        .iter()
        .flat_map(|g| &g.evals)
        .map(|p| p.runs * 2)
        .sum();
    let estimate: f64 = groups
        .iter()
        .flat_map(|g| &g.evals)
        .map(|p| f64::from(p.runs) * 2.0 * p.eval.max_budget_usd)
        .sum();
    let n_evals: usize = groups.iter().map(|g| g.evals.len()).sum();
    let header =
        format!("{n_evals} evals, {total_runs} runs, estimate {estimate:.2} USD, cap {cap:.2} USD");
    if o.dry_run {
        dry_run(
            &groups, &o.target, &header, estimate, cap, total_runs, table, o.json,
        );
        return Ok(0);
    }

    // The earlier run, if there is one.
    let mut pairs: Vec<Vec<PairRec>> = groups.iter().map(|_| Vec::new()).collect();
    let mut state = State {
        cap,
        first,
        spend: 0.0,
        excluded: vec![0, 0],
        models: BTreeSet::new(),
        streak: 0,
        floor: result_floor(repo, table.min_free_mib, &out_dir, o.resume.as_deref()),
        low: None,
    };
    let mut started = utc_time(now_secs());
    let mut written: Option<String> = None;
    if let (Some(file), Some(v)) = (&o.resume, &resumed) {
        let prior = read_prior(v, &groups);
        started = prior.started;
        state.spend = prior.spend;
        state.excluded = prior.excluded;
        state.models = prior.models;
        pairs = prior.pairs;
        written = Some(file.clone());
    }
    let remaining: f64 = groups
        .iter()
        .zip(&pairs)
        .flat_map(|(g, ps)| {
            g.evals.iter().map(move |p| {
                let done: u32 = ps
                    .iter()
                    .filter(|r| r.eval == p.eval.id)
                    .map(|r| u32::from(r.with.is_some()) + u32::from(r.without.is_some()))
                    .sum();
                f64::from((p.runs * 2).saturating_sub(done)) * p.eval.max_budget_usd
            })
        })
        .sum();
    if state.spend + remaining > cap + EPS {
        // The run starts. The check before each run stops it at the cap, and
        // `--resume` goes on with a higher cap, so no paid run is lost.
        eprintln!(
            "eval run: note: the estimate {:.2} USD is above max-total-usd {cap:.2} USD; the run stops before a run that could pass the cap, and --resume goes on",
            state.spend + remaining
        );
    }

    // The target `accept` needs `python3` with `tomllib`: stop before a run.
    if target == Target::Accept {
        preflight_tomllib()?;
    }
    // A bad `--out` must stop the command before the first run spends money.
    make_out_dir(repo, &out_dir)?;
    let claude_code = claude_version()?;
    let harness = harness_list(repo)?;
    if !o.json {
        println!("eval run --ab {}: {header}", o.target);
    }
    announce_pin();
    catch_interrupts();
    let mut ctx = Ctx {
        limits,
        merge_instructions: true,
        harness,
        check_sh,
        isolate: o.isolate,
        counter: 0,
        arm: o.harness,
        named: o.named,
    };
    let meta = Meta {
        harness: o.harness,
        claude_code,
        started: started.clone(),
        target: o.target.clone(),
        runs,
        cap,
        estimate,
        out_dir,
    };
    let mut stopped: Option<Stop> = None;
    // A git or file error ends the loop, but the paid runs are not lost: the
    // result is written before the command exits.
    let mut failed: Option<Fatal> = None;
    'groups: for (gi, g) in groups.iter().enumerate() {
        for p in &g.evals {
            for n in 1..=p.runs {
                let at = match pairs[gi]
                    .iter()
                    .position(|r| r.eval == p.eval.id && r.run == n)
                {
                    Some(at) => at,
                    None => {
                        pairs[gi].push(PairRec {
                            eval: p.eval.id.clone(),
                            run: n,
                            with: None,
                            without: None,
                        });
                        pairs[gi].len() - 1
                    }
                };
                if pairs[gi][at].complete() {
                    continue;
                }
                // `with` runs first for an odd number, `without` for an even one.
                let order = if n % 2 == 1 {
                    [Arm::With, Arm::Without]
                } else {
                    [Arm::Without, Arm::With]
                };
                for arm in order {
                    if pairs[gi][at].side(arm).is_some() {
                        continue;
                    }
                    let done = match run_arm(&mut ctx, &mut state, g, p, arm, n) {
                        Ok(d) => d,
                        // A signal can break a git command of the runner. It is
                        // an interrupt, not an error.
                        Err(_) if interrupt_signal().is_some() => {
                            stopped = Some(Stop::Interrupted);
                            break 'groups;
                        }
                        Err(e) => {
                            stopped = Some(Stop::Error);
                            failed = Some(e);
                            break 'groups;
                        }
                    };
                    match done {
                        Done::Counted(side) => {
                            *pairs[gi][at].side_mut(arm) = Some(side);
                            // The result is on the disk after each arm, so a
                            // kill that nothing can trap (SIGKILL, a power
                            // loss) loses no paid arm. The file says
                            // `interrupted` until the end.
                            if let Err(e) = persist(
                                repo,
                                &meta,
                                &groups,
                                &pairs,
                                &state,
                                Some(Stop::Interrupted),
                                None,
                                &mut written,
                            ) {
                                eprintln!(
                                    "eval run: cannot write the partial result: {}",
                                    e.message()
                                );
                            }
                        }
                        Done::Stop(s) => {
                            stopped = Some(s);
                            break 'groups;
                        }
                    }
                }
                // The pair is complete: write the file again, so a half pair
                // that the loop above kept is on the disk too.
                if let Err(e) = persist(
                    repo,
                    &meta,
                    &groups,
                    &pairs,
                    &state,
                    Some(Stop::Interrupted),
                    None,
                    &mut written,
                ) {
                    eprintln!("eval run: cannot write the partial result: {}", e.message());
                }
            }
        }
    }

    let error = failed.as_ref().map(|e| error_text(repo, e.message()));
    let result = build_result(&meta, &groups, &pairs, &state, stopped, error.clone());
    let compact = result_json(&result);
    if o.json {
        println!("{compact}");
    } else {
        for line in result_text(&result) {
            println!("{line}");
        }
    }
    let path = persist(
        repo,
        &meta,
        &groups,
        &pairs,
        &state,
        stopped,
        error,
        &mut written,
    );
    if let Some(e) = failed {
        // The error of the run comes first. A failed write is only a note.
        match &path {
            Ok(p) => eprintln!("eval run: the partial result is in {p}"),
            Err(w) => eprintln!(
                "eval run: the partial result is not written: {}",
                w.message()
            ),
        }
        return Err(e);
    }
    let path = path?;
    eprintln!("eval run: the result is in {path}");
    match stopped {
        None => Ok(0),
        Some(Stop::MaxTotal) => {
            eprintln!(
                "eval run: stopped: the next run could pass max-total-usd {cap:.2} USD (the spend is {:.2} USD); the partial result is written",
                state.spend
            );
            Ok(3)
        }
        Some(Stop::Error) => unreachable!("an error returns above"),
        Some(Stop::DiskSpace) => {
            if let Some(low) = &state.low {
                eprintln!("{}", low.message(Some(&path)));
            }
            Ok(3)
        }
        Some(Stop::ApiError) => {
            eprintln!(
                "eval run: stopped (api-error): the API refused the attempts and the waits are over, or 5 attempts in a row failed with an error and no cost; the partial result is written. Run the target again later with --resume {path}"
            );
            Ok(3)
        }
        Some(Stop::Interrupted) => {
            let sig = interrupt_signal().unwrap_or(2);
            eprintln!(
                "eval run: interrupted; the partial result is written. Go on with --resume {path}"
            );
            Ok((128 + sig) as u8)
        }
    }
}

/// The values of a run that do not change.
struct Meta {
    /// The harness arm of the copy.
    harness: HarnessArm,
    claude_code: Option<String>,
    started: String,
    target: String,
    /// The value of `--runs`, or `None` if each eval keeps its own runs.
    runs: Option<u32>,
    cap: f64,
    estimate: f64,
    out_dir: String,
}

/// The values that change during the runs.
struct State {
    cap: f64,
    /// The first wait of the back-off, in seconds.
    first: u64,
    /// The cost of each attempt, also of an excluded one.
    spend: f64,
    /// The excluded attempts of each arm.
    excluded: Vec<u32>,
    models: BTreeSet<String>,
    /// The excluded attempts in a row that are errors with no cost and no
    /// marker (`Refusal::Silent`). A real run resets it.
    streak: u32,
    /// The check of the free space before each attempt (D100).
    floor: Floor,
    /// The folder that stopped the command with `Stop::DiskSpace`.
    low: Option<Low>,
}

/// The excluded errors with no cost in a row that stop the command.
const STREAK_STOP: u32 = 5;

enum Done {
    Counted(Side),
    Stop(Stop),
}

/// The check of the free space of a run of `--ab` or `--arms` (D100). The
/// folder of the result is the folder of the file of `--resume`, which the
/// command writes again, and else `out_dir`.
fn result_floor(repo: &Repo, min_mib: u64, out_dir: &str, resume: Option<&str>) -> Floor {
    let dir = match resume {
        Some(file) => match Path::new(file).parent() {
            Some(p) if !p.as_os_str().is_empty() => p.display().to_string(),
            _ => ".".to_string(),
        },
        None => out_dir.to_string(),
    };
    Floor::new(min_mib, Some((repo.root.join(&dir), dir)))
}

/// Builds the result from the state.
fn build_result(
    meta: &Meta,
    groups: &[Group],
    pairs: &[Vec<PairRec>],
    state: &State,
    stopped: Option<Stop>,
    error: Option<String>,
) -> ResultFile {
    ResultFile {
        harness: meta.harness,
        claude_code: meta.claude_code.clone(),
        models: state.models.iter().cloned().collect(),
        started: meta.started.clone(),
        target: meta.target.clone(),
        runs: meta.runs,
        cap: meta.cap,
        estimate: meta.estimate,
        spend: state.spend,
        excluded: state.excluded.clone(),
        stopped,
        error,
        items: groups
            .iter()
            .zip(pairs)
            .map(|(g, ps)| item(g, ps))
            .collect(),
    }
}

/// Writes the result to the disk. The first call picks a free name in the
/// folder of the results. The next calls replace that file with a rename.
#[allow(clippy::too_many_arguments)]
fn persist(
    repo: &Repo,
    meta: &Meta,
    groups: &[Group],
    pairs: &[Vec<PairRec>],
    state: &State,
    stopped: Option<Stop>,
    error: Option<String>,
    written: &mut Option<String>,
) -> Result<String, Fatal> {
    let compact = result_json(&build_result(meta, groups, pairs, state, stopped, error));
    match written {
        Some(path) => {
            replace_result(repo, path, &compact)?;
            Ok(path.clone())
        }
        None => {
            let path = write_result(repo, &meta.out_dir, &meta.started, &compact)?;
            *written = Some(path.clone());
            Ok(path)
        }
    }
}

/// Waits for `seconds`, and ends early if SIGINT or SIGTERM comes. Returns
/// false in that case.
fn sleep_unless_interrupted(seconds: u64) -> bool {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    while std::time::Instant::now() < end {
        if interrupt_signal().is_some() {
            return false;
        }
        std::thread::sleep(
            std::time::Duration::from_millis(50).min(end - std::time::Instant::now()),
        );
    }
    interrupt_signal().is_none()
}

/// The skills of the copy of `--ab --harness` (D57): the skills that `init`
/// proposes for the profile `standard` at the level of the harness arm (the
/// default skills of ADR 0023), and then the edit of the arm. `with` adds the
/// measured skill, and `without` removes it. The pair `agents-line` has no
/// skill in each arm. The target `accept` measures `review`, and both arms also
/// have `verify`, which is off by default: the builder checks its own proof
/// (D62). The arm `without` has no agent `acceptor`, because the role comes with
/// the skill, and `prepare` gives it the text of 0.7.1 (D76 item 3).
fn arm_skills(kind: Kind, arm: Arm, harness: HarnessArm) -> Vec<Skill> {
    let Some(h) = harness.harness() else {
        return Vec::new();
    };
    if h.level == Level::None {
        return Vec::new();
    }
    let skill = match kind {
        Kind::Skill(skill) => skill,
        Kind::Accept => Skill::Review,
        Kind::AgentsLine => return Vec::new(),
    };
    let mut list =
        crate::init::profile_skills(crate::config::Profile::Standard, h.level, &|s: Skill| {
            s.default_on()
        });
    list.retain(|s| *s != skill);
    if kind == Kind::Accept && !list.contains(&Skill::Verify) {
        list.push(Skill::Verify);
    }
    if arm == Arm::With {
        list.push(skill);
    }
    list.sort();
    list
}

/// One side of one run for `run_side`.
struct Job<'a> {
    /// The name that the messages use: the group, or `harness`.
    label: &'a str,
    p: &'a Planned,
    n: u32,
    arm_name: &'a str,
    /// The place of the arm in `State.excluded`.
    arm_index: usize,
    deny: Option<&'a str>,
    edit: &'a dyn Fn(&Path) -> Result<(), Fatal>,
}

/// Runs one arm of one pair of `--ab`.
fn run_arm(
    ctx: &mut Ctx,
    state: &mut State,
    g: &Group,
    p: &Planned,
    arm: Arm,
    n: u32,
) -> Result<Done, Fatal> {
    let deny = (arm == Arm::Without).then_some(DENIED);
    let harness = ctx.arm;
    let skills = arm_skills(g.kind, arm, harness);
    // A harness arm has written the skill file with the generator, with its
    // marker: the edit keeps it, so only the files of the host get the text
    // of `skill show`.
    let keep = harness != HarnessArm::Project;
    let edit = |dir: &Path| {
        write_harness_arm(dir, harness, &skills, false)?;
        prepare(dir, g.kind, arm, keep)
    };
    run_side(
        ctx,
        state,
        &Job {
            label: &g.name,
            p,
            n,
            arm_name: arm.name(),
            arm_index: arm.index(),
            deny,
            edit: &edit,
        },
    )
}

/// Runs one side of one run. An attempt that the API refused is excluded: no
/// result counts it, and the spend gets the cost that it reports (0 if it
/// reports none). The retry makes a new copy, with the same eval, arm and run.
/// The cap is checked before each attempt: if the spend and the budget of the
/// next run could pass it, the run does not start.
fn run_side(ctx: &mut Ctx, state: &mut State, job: &Job) -> Result<Done, Fatal> {
    let (p, n) = (job.p, job.n);
    let mut excluded = 0;
    // An account limit gets the long ladder, and it stays long after it.
    let mut account = false;
    loop {
        if interrupt_signal().is_some() {
            return Ok(Done::Stop(Stop::Interrupted));
        }
        if state.spend + p.eval.max_budget_usd > state.cap + EPS {
            return Ok(Done::Stop(Stop::MaxTotal));
        }
        // Before each attempt, also the new attempt after a refusal (D100).
        if let Some(low) = state.floor.low() {
            state.low = Some(low);
            return Ok(Done::Stop(Stop::DiskSpace));
        }
        eprintln!(
            "eval run: {} {} run {n} of {} ({})",
            job.label, p.eval.id, p.runs, job.arm_name
        );
        let r: RunResult = one_attempt(ctx, p, job.edit, job.deny)?;
        if r.interrupted {
            // An agent run that ended before a check was interrupted is paid.
            state.spend += r.cost.unwrap_or(0.0);
            return Ok(Done::Stop(Stop::Interrupted));
        }
        state.models.extend(r.models.iter().cloned());
        let Some(class) = r.class else {
            state.spend += r.charged;
            state.streak = 0;
            return Ok(Done::Counted(Side {
                claude_exit: r.claude_exit,
                subtype: r.subtype,
                cost: r.cost,
                excluded,
                denials: r.denials,
                passed: r.passed,
                tokens: r.tokens,
                models: r.models,
                checks: r.checks,
                turns: r.turns,
                segments: r.segments,
                seconds: Some(r.seconds),
            }));
        };
        state.spend += r.cost.unwrap_or(0.0);
        state.excluded[job.arm_index] += 1;
        excluded += 1;
        account |= class == Refusal::Account;
        if class == Refusal::Silent {
            state.streak += 1;
            if state.streak >= STREAK_STOP {
                return Ok(Done::Stop(Stop::ApiError));
            }
        }
        let ladder = retry_waits(state.first, account);
        if state.spend + p.eval.max_budget_usd > state.cap + EPS {
            return Ok(Done::Stop(Stop::MaxTotal));
        }
        if excluded as usize > ladder.len() {
            return Ok(Done::Stop(Stop::ApiError));
        }
        let wait = ladder[excluded as usize - 1];
        eprintln!(
            "eval run: the API refused the attempt ({} {} run {n}, {}); it is excluded. wait {wait} seconds, then try again",
            job.label, p.eval.id, job.arm_name
        );
        if !sleep_unless_interrupted(wait) {
            return Ok(Done::Stop(Stop::Interrupted));
        }
    }
}

/// The result of an earlier run.
struct Prior {
    started: String,
    spend: f64,
    excluded: Vec<u32>,
    models: BTreeSet<String>,
    pairs: Vec<Vec<PairRec>>,
}

/// Reads the result file of an earlier run of the same target. `file` is a
/// path of the repository, with no `..`. The pairs go to the groups of the
/// plan, and a pair of an eval that the plan has no more is dropped.
fn read_prior(v: &json::Value, groups: &[Group]) -> Prior {
    let num = |v: &json::Value| match v {
        json::Value::Num(n) => Some(*n),
        _ => None,
    };
    let side = read_side;
    let mut pairs: Vec<Vec<PairRec>> = groups.iter().map(|_| Vec::new()).collect();
    for item in v.get("skills").as_arr() {
        let Some(gi) = groups
            .iter()
            .position(|g| Some(g.name.as_str()) == item.get("name").as_str())
        else {
            continue;
        };
        for pr in item.get("pairs").as_arr() {
            let Some(eval) = pr.get("eval").as_str() else {
                continue;
            };
            if !groups[gi].evals.iter().any(|p| p.eval.id == eval) {
                continue;
            }
            let rec = PairRec {
                eval: eval.to_string(),
                run: num(pr.get("run")).unwrap_or(0.0) as u32,
                with: side(pr.get("with")),
                without: side(pr.get("without")),
            };
            if rec.run > 0 {
                pairs[gi].push(rec);
            }
        }
    }
    let ex = v.get("excluded");
    Prior {
        started: v
            .get("started")
            .as_str()
            .unwrap_or("1970-01-01T00:00:00Z")
            .to_string(),
        spend: num(v.get("spend_usd")).unwrap_or(0.0),
        excluded: vec![
            num(ex.get("with")).unwrap_or(0.0) as u32,
            num(ex.get("without")).unwrap_or(0.0) as u32,
        ],
        models: v
            .get("models")
            .as_arr()
            .iter()
            .filter_map(|m| m.as_str().map(str::to_string))
            .collect(),
        pairs,
    }
}

/// Reads a side of a result file, or `None` for `null`.
fn read_side(v: &json::Value) -> Option<Side> {
    let num = |v: &json::Value| match v {
        json::Value::Num(n) => Some(*n),
        _ => None,
    };
    if !matches!(v, json::Value::Obj(_)) {
        return None;
    }
    Some(Side {
        claude_exit: num(v.get("claude_exit")).unwrap_or(-1.0) as i32,
        subtype: v.get("subtype").as_str().map(str::to_string),
        cost: num(v.get("cost_usd")),
        excluded: num(v.get("excluded")).unwrap_or(0.0) as u32,
        denials: num(v.get("permission_denials")).map(|n| n as u32),
        passed: v.get("passed").is_true(),
        tokens: read_tokens(v.get("tokens")),
        models: v
            .get("models")
            .as_arr()
            .iter()
            .filter_map(|m| m.as_str().map(str::to_string))
            .collect(),
        checks: v
            .get("checks")
            .as_arr()
            .iter()
            .filter_map(|c| {
                Some((
                    c.get("name").as_str()?.to_string(),
                    c.get("passed").is_true(),
                ))
            })
            .collect(),
        turns: num(v.get("turns")).map(|n| n as u32),
        segments: num(v.get("segments")).map(|n| n as u32),
        seconds: num(v.get("seconds")),
    })
}

/// Reads the `tokens` of a side of a result file: 4 integers, or `None`.
fn read_tokens(v: &json::Value) -> Option<Tokens> {
    let n = |key: &str| match v.get(key) {
        json::Value::Num(n) if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 => Some(*n as u64),
        _ => None,
    };
    Some(Tokens {
        input: n("in")?,
        output: n("out")?,
        cache_read: n("cache_read")?,
        cache_write: n("cache_write")?,
    })
}

/// Reads and checks the result file of `--resume`: a path of the repository
/// with no `..`, valid JSON, and the result of the same target.
fn read_resume_file(repo: &Repo, file: &str, target: &str) -> Result<json::Value, Fatal> {
    let bad = |m: String| Fatal::Usage(format!("--resume `{file}`: {m}"));
    let v = read_result_file(repo, file)?;
    if v.get("target").as_str() != Some(target) {
        return Err(bad(format!(
            "the file is the result of the target `{}`, not of `{target}`",
            v.get("target").as_str().unwrap_or("?")
        )));
    }
    Ok(v)
}

/// Reads the result file of `--resume`: a path of the repository with no
/// `..` and the end `.json`, and valid JSON.
fn read_result_file(repo: &Repo, file: &str) -> Result<json::Value, Fatal> {
    let bad = |m: String| Fatal::Usage(format!("--resume `{file}`: {m}"));
    if file.is_empty()
        || file.starts_with('/')
        || file.contains('\\')
        || file
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
        || !file.ends_with(".json")
    {
        return Err(bad(
            "give a result file below the repository root, with no `..` and the end `.json`".into(),
        ));
    }
    let text = fs::read(repo.root.join(file)).map_err(|e| bad(format!("cannot read it ({e})")))?;
    let text = String::from_utf8(text).map_err(|_| bad("it is not UTF-8".into()))?;
    json::parse(&text).map_err(|e| bad(format!("it is not valid JSON ({e})")))
}

/// Chooses the evals of the target, and the files of each fixture.
fn select(
    repo: &Repo,
    all: Vec<Eval>,
    target: &Target,
    runs: Option<u32>,
    pack_dir: &str,
) -> Result<Vec<Group>, Fatal> {
    let plan = |mut eval: Eval| -> Result<Planned, Fatal> {
        // The arm `with` needs the tool `Skill` under `--permission-mode
        // dontAsk`, and both arms get the same tools.
        if !eval.allowed_tools.iter().any(|t| t == "Skill") {
            eval.allowed_tools.push("Skill".to_string());
        }
        let runs = runs.unwrap_or(eval.runs);
        plan_eval(repo, eval, runs)
    };
    let mut groups = Vec::new();
    match target {
        Target::AgentsLine => {
            let mut evals = Vec::new();
            for id in AGENTS_LINE_EVALS {
                let Some(e) = all.iter().find(|e| e.id == id) else {
                    return Err(Fatal::Config(format!(
                        "the pair `{AGENTS_LINE}` needs the eval `{id}` in `{pack_dir}`; run `spec-dna eval pack`"
                    )));
                };
                evals.push(plan(e.clone())?);
            }
            groups.push(Group {
                name: AGENTS_LINE.to_string(),
                kind: Kind::AgentsLine,
                evals,
            });
        }
        Target::Accept => {
            // The evals have the key `skill` with the value `review`, so the
            // group has the name `review` and the result is a result of that
            // skill. The target `accept` selects them by their ids.
            let mut evals = Vec::new();
            for e in all.iter().filter(|e| is_accept(e)) {
                evals.push(plan(e.clone())?);
            }
            if evals.is_empty() {
                return Err(Fatal::Config(format!(
                    "no eval `accept-<n>` in `{pack_dir}`; run `spec-dna eval pack` to write the pack"
                )));
            }
            groups.push(Group {
                name: Skill::Review.key().to_string(),
                kind: Kind::Accept,
                evals,
            });
        }
        Target::Skill(_) | Target::All => {
            for skill in Skill::ALL {
                let wanted = match target {
                    Target::Skill(k) => k == skill.key(),
                    _ => true,
                };
                if !wanted {
                    continue;
                }
                let mut evals = Vec::new();
                // The evals of the target `accept` also have the skill `review`,
                // and they have a target of their own.
                for e in all
                    .iter()
                    .filter(|e| e.skill.as_deref() == Some(skill.key()) && !is_accept(e))
                {
                    evals.push(plan(e.clone())?);
                }
                if !evals.is_empty() {
                    groups.push(Group {
                        name: skill.key().to_string(),
                        kind: Kind::Skill(skill),
                        evals,
                    });
                }
            }
            if groups.is_empty() {
                return Err(Fatal::Config(match target {
                    Target::Skill(k) => format!(
                        "no eval of the skill `{k}` in `{pack_dir}`; run `spec-dna eval pack` to write the pack"
                    ),
                    _ => format!(
                        "no eval with the key `skill` in `{pack_dir}`; run `spec-dna eval pack` to write the pack"
                    ),
                }));
            }
        }
    }
    Ok(groups)
}

/// Changes the copy for an arm, after the files are copied. The pin line of
/// `spec-dna.toml` of the fixture gets the version of the binary, so a
/// fixture never gives exit code 3 for a version. SD501 and SD502 are off in
/// the copy: the arm `without` edits the adapter files by design, and the
/// adapter skill of the project lists the layer types of the project, not of
/// the fixture. Both arms get the same edit.
fn prepare(dir: &Path, kind: Kind, arm: Arm, keep_generated: bool) -> Result<(), Fatal> {
    pin_version(dir)?;
    adapter_rules_off(dir)?;
    let io = |what: &str, e: std::io::Error| {
        Fatal::Io(format!("cannot change `{what}` in the copy: {e}"))
    };
    match kind {
        Kind::Skill(_) | Kind::Accept => {
            let skill = match kind {
                Kind::Skill(skill) => skill,
                _ => Skill::Review,
            };
            let rel = skill.path();
            let file = dir.join(&rel);
            match arm {
                Arm::With if keep_generated && file.is_file() => {}
                Arm::With => {
                    if let Some(parent) = file.parent() {
                        fs::create_dir_all(parent).map_err(|e| io(&rel, e))?;
                    }
                    fs::write(&file, skill_file(skill, profile_of(dir)))
                        .map_err(|e| io(&rel, e))?;
                }
                // The target `accept` (D62 and D76 item 3): the arm `without`
                // has the text of `lite`, which is the text of 0.7.1, and no
                // agent `acceptor`. The discipline line stays, because the
                // review of 0.7.1 is a skill of the arm.
                Arm::Without if kind == Kind::Accept => {
                    if let Some(parent) = file.parent() {
                        fs::create_dir_all(parent).map_err(|e| io(&rel, e))?;
                    }
                    let text = if keep_generated {
                        harness::render_skill(skill, Profile::Lite)
                    } else {
                        skill_file(skill, Profile::Lite)
                    };
                    fs::write(&file, text).map_err(|e| io(&rel, e))?;
                    let _ = fs::remove_file(dir.join(".claude/agents/acceptor.md"));
                }
                Arm::Without => {
                    if let Some(parent) = file.parent() {
                        let _ = fs::remove_dir_all(parent);
                    }
                    for rel in [adapters::AGENTS, adapters::CLAUDE, adapters::SKILL] {
                        edit_text(dir, rel, |t| adapters::without_skill(t, skill.key()))?;
                    }
                }
            }
        }
        Kind::AgentsLine => {
            let _ = fs::remove_dir_all(dir.join(".claude/skills"));
            for rel in [adapters::AGENTS, adapters::CLAUDE] {
                match arm {
                    Arm::With => {
                        let path = dir.join(rel);
                        let text = fs::read_to_string(&path).unwrap_or_default();
                        fs::write(&path, adapters::upsert_line(&text)).map_err(|e| io(rel, e))?;
                    }
                    Arm::Without => {
                        edit_text(dir, rel, adapters::remove_line)?;
                        let path = dir.join(rel);
                        if fs::read_to_string(&path).is_ok_and(|t| t.trim().is_empty()) {
                            fs::remove_file(&path).map_err(|e| io(rel, e))?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Writes the harness arm into the copy (D57). `project` keeps the harness
/// files of the project as the copy. With `named` (`--arms` or `--harness`),
/// it gets the same items as each other arm: the pin of the binary, the rules
/// `SD501` and `SD502` off, and the adapter line of the binary. So no arm differs
/// in them. Without `named`, `project` writes nothing: `--ab` has its own edit.
/// Another arm gets, in this order:
///
/// 1. the pin of the binary and the rules `SD501` and `SD502` off, in
///    `spec-dna.toml` (the runner makes the file if the fixture has none),
/// 2. the adapter line of the binary in `AGENTS.md` and `CLAUDE.md`,
/// 3. the table `[harness]` of the arm, and
/// 4. the files that the generator of `init` writes for that table.
///
/// `skills` is the list of `[harness] skills`. A built-in arm has none, and
/// `--ab --harness` gives the default skills of its level. A fixture that has
/// its own `[harness]` table is an error, because the arm is the table.
pub(crate) fn write_harness_arm(
    dir: &Path,
    arm: HarnessArm,
    skills: &[Skill],
    named: bool,
) -> Result<(), Fatal> {
    let harness = arm.harness();
    if harness.is_none() && !named {
        return Ok(());
    }
    let io = |what: &str, e: std::io::Error| {
        Fatal::Io(format!("cannot change `{what}` in the copy: {e}"))
    };
    let toml_path = dir.join("spec-dna.toml");
    if !toml_path.is_file() {
        fs::write(&toml_path, format!("spec-dna = \"{VERSION}\"\n"))
            .map_err(|e| io("spec-dna.toml", e))?;
    }
    pin_version(dir)?;
    adapter_rules_off(dir)?;
    for rel in [adapters::AGENTS, adapters::CLAUDE] {
        let path = dir.join(rel);
        let text = fs::read_to_string(&path).unwrap_or_default();
        fs::write(&path, adapters::upsert_line(&text)).map_err(|e| io(rel, e))?;
    }
    let Some(mut h) = harness else {
        return Ok(());
    };
    h.skills = skills.to_vec();
    h.skills.sort();
    h.profile = profile_of(dir);
    let text = fs::read_to_string(&toml_path).map_err(|e| io("spec-dna.toml", e))?;
    if text
        .parse::<toml::Table>()
        .is_ok_and(|t| t.contains_key("harness"))
    {
        return Err(Fatal::Config(format!(
            "the fixture has a [harness] table in spec-dna.toml, and the harness arm `{}` writes that table; remove it from the fixture",
            arm.name()
        )));
    }
    let mut text = text;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push('\n');
    text.push_str(&h.toml_section());
    fs::write(&toml_path, text).map_err(|e| io("spec-dna.toml", e))?;
    let existing: Vec<(String, String)> = [harness::CLAUDE, harness::SETTINGS]
        .into_iter()
        .filter_map(|rel| {
            fs::read_to_string(dir.join(rel))
                .ok()
                .map(|t| (rel.to_string(), t))
        })
        .collect();
    let plan = harness::plan(&existing, None, &h, harness::Mode::Reconfigure)
        .map_err(|e| Fatal::Config(format!("the harness arm `{}`: {e}", arm.name())))?;
    for c in plan.changes {
        let path = dir.join(&c.path);
        match c.action {
            harness::Action::Remove => {
                let _ = fs::remove_file(&path);
            }
            harness::Action::Create | harness::Action::Change => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|e| io(&c.path, e))?;
                }
                fs::write(&path, &c.text).map_err(|e| io(&c.path, e))?;
            }
        }
    }
    Ok(())
}

/// Rewrites a text file of the copy, if it exists and is UTF-8.
fn edit_text(dir: &Path, rel: &str, f: impl Fn(&str) -> String) -> Result<(), Fatal> {
    let path = dir.join(rel);
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let new = f(&text);
    if new != text {
        fs::write(&path, new)
            .map_err(|e| Fatal::Io(format!("cannot change `{rel}` in the copy: {e}")))?;
    }
    Ok(())
}

/// Puts the version of the binary into the pin line of `spec-dna.toml` in the
/// copy, if the copy has the file.
fn pin_version(dir: &Path) -> Result<(), Fatal> {
    edit_text(dir, "spec-dna.toml", |text| {
        let mut done = false;
        let lines: Vec<String> = text
            .split_inclusive('\n')
            .map(|l| {
                let t = l.trim_start();
                if !done
                    && t.strip_prefix("spec-dna")
                        .is_some_and(|r| r.trim_start().starts_with('='))
                {
                    done = true;
                    let eol = if l.ends_with('\n') { "\n" } else { "" };
                    format!("spec-dna = \"{VERSION}\"{eol}")
                } else {
                    l.to_string()
                }
            })
            .collect();
        lines.concat()
    })
}

/// The rules that the copy turns off: the codes of the adapter files.
const RULES_OFF: [&str; 2] = ["SD501", "SD502"];

/// Sets `SD501 = "off"` and `SD502 = "off"` under `[rules]` of `spec-dna.toml`
/// in the copy, if the copy has the file. A table `[rules]` that exists keeps
/// its other keys.
fn adapter_rules_off(dir: &Path) -> Result<(), Fatal> {
    let path = dir.join("spec-dna.toml");
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(());
    };
    // A file that the edit cannot change (a text that does not parse, or the
    // dotted keys `rules.x = ...`) stays as it is. The run goes on, as it did
    // before this edit, and `check` in the copy can report SD501 or SD502.
    let Ok(new) = rules_off(&text) else {
        return Ok(());
    };
    if new != text {
        fs::write(&path, new)
            .map_err(|e| Fatal::Io(format!("cannot change `spec-dna.toml` in the copy: {e}")))?;
    }
    Ok(())
}

/// The text of `spec-dna.toml` with the codes of `RULES_OFF` set to `off`
/// under `[rules]`.
fn rules_off(text: &str) -> Result<String, Fatal> {
    let bad = |e: String| Fatal::Config(format!("spec-dna.toml of the copy: {e}"));
    let table: toml::Table = text
        .parse()
        .map_err(|e: toml::de::Error| bad(e.to_string()))?;
    let off = toml::Value::String("off".to_string());
    let Some(rules) = table.get("rules") else {
        let mut out = text.to_string();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("\n[rules]\n");
        for code in RULES_OFF {
            out.push_str(&format!("{code} = \"off\"\n"));
        }
        return Ok(out);
    };
    let toml::Value::Table(rules) = rules else {
        return Err(bad("`rules` is not a table".to_string()));
    };
    let mut out = text.to_string();
    let mut missing: Vec<(&str, toml::Value)> = Vec::new();
    for code in RULES_OFF {
        match rules.get(code) {
            Some(v) if *v == off => {}
            Some(_) => out = set_value(&out, &[k("rules"), k(code)], &off)?,
            None => missing.push((code, off.clone())),
        }
    }
    if !missing.is_empty() {
        out = match insert_keys(&out, &[k("rules")], &missing) {
            Ok(t) => t,
            // An inline table `rules = { ... }`: write it whole.
            Err(_) => {
                let mut all = rules.clone();
                for (code, v) in missing {
                    all.insert(code.to_string(), v);
                }
                set_value(&out, &[k("rules")], &toml::Value::Table(all))?
            }
        };
    }
    Ok(out)
}

/// The file of a skill in the arm `with`: the front matter and the text of
/// `skill show` for the profile of the project (design 0.8.0, D76 item 3).
fn skill_file(skill: Skill, profile: Profile) -> String {
    format!(
        "---\nname: {}\ndescription: {}\n---\n\n{}\n",
        skill.name(),
        harness_text::description(skill),
        harness_text::body(skill, profile)
    )
}

/// The profile of the project in the copy `dir`: the key `profile` of its
/// `spec-dna.toml`. A copy with no file, a file that does not parse and a file
/// with no valid key have the default profile, as `skill show` outside a project
/// has. It reads the key only, so a fixture that holds a part of a configuration
/// works.
fn profile_of(dir: &Path) -> Profile {
    fs::read_to_string(dir.join("spec-dna.toml"))
        .ok()
        .and_then(|t| t.parse::<toml::Table>().ok())
        .and_then(|t| Profile::from_name(t.get("profile")?.as_str()?))
        .unwrap_or(Profile::Standard)
}

// The analysis and the result.

/// The result of one group.
struct Item {
    name: String,
    evals: usize,
    pairs: Vec<PairRec>,
    /// `None` when no pair is done.
    analysis: Option<Analysis>,
    /// True when each planned pair is done.
    complete: bool,
}

fn item(g: &Group, pairs: &[PairRec]) -> Item {
    let by_eval: Vec<Vec<Pair>> = g
        .evals
        .iter()
        .map(|p| {
            pairs
                .iter()
                .filter(|r| r.eval == p.eval.id)
                .filter_map(|r| {
                    let (w, o) = (r.with.as_ref()?, r.without.as_ref()?);
                    Some(Pair {
                        with_passed: w.passed,
                        with_cost: w.cost.unwrap_or(p.eval.max_budget_usd),
                        without_passed: o.passed,
                        without_cost: o.cost.unwrap_or(p.eval.max_budget_usd),
                    })
                })
                .collect()
        })
        .collect();
    let planned: u32 = g.evals.iter().map(|p| p.runs).sum();
    let done = pairs.iter().filter(|r| r.complete()).count() as u32;
    // The pairs in the order of the plan, also after a resume.
    let order = |r: &PairRec| {
        (
            g.evals
                .iter()
                .position(|p| p.eval.id == r.eval)
                .unwrap_or(usize::MAX),
            r.run,
        )
    };
    let mut shown: Vec<PairRec> = pairs
        .iter()
        .filter(|r| r.with.is_some() || r.without.is_some())
        .cloned()
        .collect();
    shown.sort_by_key(order);
    Item {
        name: g.name.clone(),
        evals: g.evals.len(),
        pairs: shown,
        analysis: (done > 0).then(|| verdict::analyze(&by_eval)),
        complete: done == planned,
    }
}

struct ResultFile {
    harness: HarnessArm,
    claude_code: Option<String>,
    models: Vec<String>,
    started: String,
    target: String,
    runs: Option<u32>,
    cap: f64,
    estimate: f64,
    spend: f64,
    excluded: Vec<u32>,
    stopped: Option<Stop>,
    /// The message of the error that stopped the run, with no path.
    error: Option<String>,
    items: Vec<Item>,
}

/// The first line of an error message, with the paths of the repository and of
/// the temporary folder replaced, and at most 200 characters. The result
/// keeps no path of the machine.
fn error_text(repo: &Repo, message: &str) -> String {
    let mut text = message.lines().next().unwrap_or("").to_string();
    for (path, name) in [
        (std::env::temp_dir(), "<tmp>"),
        (repo.root.clone(), "<repo>"),
    ] {
        let p = path.to_string_lossy();
        if !p.is_empty() {
            text = text.replace(p.as_ref(), name);
        }
    }
    text.chars().take(200).collect()
}

fn side_json(s: &Option<Side>) -> String {
    let Some(s) = s else {
        return "null".to_string();
    };
    let obj = Obj::new()
        .num("claude_exit", s.claude_exit)
        .opt_str("subtype", s.subtype.as_deref());
    let obj = match s.cost {
        Some(c) => obj.num("cost_usd", r6(c)),
        None => obj.raw("cost_usd", "null"),
    };
    let obj = obj.num("excluded", s.excluded);
    let obj = match s.denials {
        Some(n) => obj.num("permission_denials", n),
        None => obj.raw("permission_denials", "null"),
    }
    .bool("passed", s.passed);
    let obj = match &s.tokens {
        Some(t) => obj.raw("tokens", &tokens_json(t)),
        None => obj.raw("tokens", "null"),
    };
    let checks = json::array(
        s.checks
            .iter()
            .map(|(name, passed)| Obj::new().str("name", name).bool("passed", *passed).end()),
    );
    let obj = obj.strs("models", &s.models).raw("checks", &checks);
    let obj = match s.turns {
        Some(n) => obj.num("turns", n),
        None => obj.raw("turns", "null"),
    };
    let obj = match s.segments {
        Some(n) => obj.num("segments", n),
        None => obj.raw("segments", "null"),
    };
    match s.seconds {
        Some(x) => obj.num("seconds", x),
        None => obj.raw("seconds", "null"),
    }
    .end()
}

fn item_json(it: &Item) -> String {
    let a = it.analysis.as_ref();
    let verdict_of = a.filter(|_| it.complete);
    let pairs = json::array(it.pairs.iter().map(|p| {
        Obj::new()
            .str("eval", &p.eval)
            .num("run", p.run)
            .raw("with", &side_json(&p.with))
            .raw("without", &side_json(&p.without))
            .end()
    }));
    let obj = Obj::new()
        .str("name", &it.name)
        .num("evals", it.evals)
        .raw("with", &verdict::arm_json(a.map(|a| &a.with)))
        .raw("without", &verdict::arm_json(a.map(|a| &a.without)));
    let obj = match a {
        Some(a) => obj.raw("difference", &verdict::interval_json(&a.difference)),
        None => obj.raw("difference", "null"),
    };
    let obj = match a.and_then(|a| a.cost_ratio.as_ref()) {
        Some(r) => obj.raw("cost_ratio", &verdict::interval_json(r)),
        None => obj.raw("cost_ratio", "null"),
    };
    let obj = obj.opt_str("verdict", verdict_of.map(|a| a.verdict.name()));
    let obj = match verdict_of.and_then(|a| a.rule) {
        Some(r) => obj.num("rule", r),
        None => obj.raw("rule", "null"),
    };
    obj.bool("below_floor", a.is_some_and(|a| a.below_floor))
        .raw("pairs", &pairs)
        .end()
}

fn result_json(r: &ResultFile) -> String {
    let models = json::array(r.models.iter().map(|m| {
        let mut s = String::new();
        json::string(&mut s, m);
        s
    }));
    let excluded = Obj::new()
        .num("with", r.excluded[0])
        .num("without", r.excluded[1])
        .end();
    let obj = Obj::new()
        .str("spec_dna", VERSION)
        .opt_str("claude_code", r.claude_code.as_deref())
        .raw("models", &models)
        .str("started", &r.started)
        .str("target", &r.target)
        .str("harness", r.harness.name());
    let obj = match r.runs {
        Some(n) => obj.num("runs", n),
        None => obj.raw("runs", "null"),
    };
    let obj = obj
        .num("seed", verdict::SEED)
        .num("resamples", verdict::RESAMPLES)
        .num("max_total_usd", r6(r.cap))
        .num("estimate_usd", r6(r.estimate))
        .num("spend_usd", r6(r.spend))
        .raw("excluded", &excluded)
        .opt_str("stopped", r.stopped.map(Stop::name));
    let obj = match &r.error {
        Some(e) => obj.str("error", e),
        None => obj,
    };
    obj.raw("skills", &json::array(r.items.iter().map(item_json)))
        .end()
}

fn result_text(r: &ResultFile) -> Vec<String> {
    let mut lines = Vec::new();
    for it in &r.items {
        let Some(a) = &it.analysis else {
            lines.push(format!("{}: no pair is done", it.name));
            continue;
        };
        let ratio = match &a.cost_ratio {
            Some(c) => format!("{:.2} ({:.2} to {:.2})", c.value, c.low, c.high),
            None => "undefined".to_string(),
        };
        let verdict = match (it.complete, a.verdict, a.rule) {
            (false, ..) => "none, the pairs are not all done".to_string(),
            (true, v, _) if a.below_floor => {
                format!("{}, {}", v.name(), verdict::floor_text(a.pairs, a.evals))
            }
            (true, v, Some(rule)) => format!("{} (rule {rule})", v.name()),
            (true, v, None) => v.name().to_string(),
        };
        lines.push(format!(
            "{}: with {} of {} runs passed, without {} of {}, difference {:+.2} ({:+.2} to {:+.2}), cost ratio {ratio}, verdict {verdict}",
            it.name,
            a.with.passes,
            a.with.runs,
            a.without.passes,
            a.without.runs,
            a.difference.value,
            a.difference.low,
            a.difference.high,
        ));
    }
    lines.push(format!("spend {:.2} USD", r.spend));
    if r.excluded != [0, 0] {
        lines.push(format!(
            "excluded attempts (the API refused them, no result counts them): with {}, without {}",
            r.excluded[0], r.excluded[1]
        ));
    }
    if let Some(s) = r.stopped {
        lines.push(format!("stopped: {}", s.name()));
    }
    if let Some(e) = &r.error {
        lines.push(format!("error: {e}"));
    }
    lines
}

/// Writes the result to `<dir>/skills-<YYYYMMDD>T<HHMMSS>Z.json`. If the file
/// exists, the name gets the first free suffix from `-2`.
fn write_result(repo: &Repo, dir: &str, started: &str, compact: &str) -> Result<String, Fatal> {
    crate::eval_run::write_result(repo, dir, "skills", started, compact)
}

#[allow(clippy::too_many_arguments)]
fn dry_run(
    groups: &[Group],
    target: &str,
    header: &str,
    estimate: f64,
    cap: f64,
    total_runs: u32,
    table: &Evals,
    json_out: bool,
) {
    let evals = groups.iter().flat_map(|g| &g.evals);
    if json_out {
        let list = json::array(evals.map(|p| {
            Obj::new()
                .str("id", &p.eval.id)
                .num("runs", p.runs)
                .num("max_budget_usd", r6(p.eval.max_budget_usd))
                .num(
                    "estimate_usd",
                    r6(f64::from(p.runs) * 2.0 * p.eval.max_budget_usd),
                )
                .end()
        }));
        println!(
            "{}",
            Obj::new()
                .str("spec_dna", VERSION)
                .bool("dry_run", true)
                .str("target", target)
                .str("evals_dir", &table.dir)
                .num("max_total_usd", r6(cap))
                .num("estimate_usd", r6(estimate))
                .num("runs", total_runs)
                .bool("above_cap", estimate > cap + EPS)
                .raw("evals", &list)
                .end()
        );
        return;
    }
    println!("eval run --ab {target} (dry run): {header}");
    if estimate > cap + EPS {
        println!(
            "a real run starts, and stops before a run that could pass the cap: the estimate {estimate:.2} USD is above max-total-usd {cap:.2} USD; --resume goes on"
        );
    }
    for p in evals {
        println!(
            "{}: {} runs, max-budget-usd {:.2}, estimate {:.2} USD",
            p.eval.id,
            p.runs,
            p.eval.max_budget_usd,
            f64::from(p.runs) * 2.0 * p.eval.max_budget_usd
        );
    }
}

// `eval run --arms` (D57).

/// The options of `eval run --arms`.
pub struct ArmsOpts {
    pub arms: Vec<HarnessArm>,
    pub ids: Vec<String>,
    pub runs: Option<u32>,
    pub max_total_usd: Option<f64>,
    pub out: Option<String>,
    pub dry_run: bool,
    pub json: bool,
    pub resume: Option<String>,
}

/// Reads the value of `--arms`: 1 to 3 different arm names (D100). Another
/// count, a name twice or an unknown name is a usage error.
pub fn parse_arms(list: &str) -> Result<Vec<HarnessArm>, Fatal> {
    let arms: Vec<HarnessArm> = list
        .split(',')
        .map(HarnessArm::parse)
        .collect::<Result<_, _>>()?;
    if !(1..=3).contains(&arms.len()) {
        return Err(Fatal::Usage(format!(
            "--arms takes 1 to 3 arms, got {}; use `--arms <arm>[,<arm>[,<arm>]]`",
            arms.len()
        )));
    }
    for (i, a) in arms.iter().enumerate() {
        if arms[..i].contains(a) {
            return Err(Fatal::Usage(format!(
                "--arms names the arm `{}` twice",
                a.name()
            )));
        }
    }
    Ok(arms)
}

/// One eval and run number of `--arms`, with a side for each arm. An arm that
/// did not run is `None`.
#[derive(Clone)]
struct ArmRun {
    eval: String,
    run: u32,
    sides: Vec<Option<Side>>,
}

impl ArmRun {
    fn complete(&self) -> bool {
        self.sides.iter().all(Option::is_some)
    }
}

/// The order of the arms of one eval in one wave: the arms, rotated left by
/// `(eval index + wave) mod arms`. The index counts from 0, and the wave from
/// 1.
fn rotation(arms: usize, eval_index: usize, wave: u32) -> Vec<usize> {
    let k = (eval_index + wave as usize) % arms;
    (0..arms).map(|j| (k + j) % arms).collect()
}

/// The values of an `--arms` run that do not change.
struct ArmsMeta {
    claude_code: Option<String>,
    started: String,
    arms: Vec<HarnessArm>,
    runs: Option<u32>,
    cap: f64,
    estimate: f64,
    out_dir: String,
}

fn arms_result_json(
    meta: &ArmsMeta,
    plan: &[Planned],
    runs: &[ArmRun],
    state: &State,
    stopped: Option<Stop>,
    error: Option<&str>,
) -> String {
    let names: Vec<String> = meta.arms.iter().map(|a| a.name().to_string()).collect();
    let models = json::array(state.models.iter().map(|m| {
        let mut s = String::new();
        json::string(&mut s, m);
        s
    }));
    let mut excluded = Obj::new();
    for (a, n) in meta.arms.iter().zip(&state.excluded) {
        excluded = excluded.num(a.name(), n);
    }
    let waves = verdict::complete_waves(
        &plan
            .iter()
            .map(|p| {
                (
                    p.runs,
                    runs.iter()
                        .filter(|r| r.eval == p.eval.id)
                        .map(|r| (r.run, r.complete()))
                        .collect(),
                )
            })
            .collect::<Vec<_>>(),
    );
    let evals = json::array(plan.iter().map(|p| {
        let mut mine: Vec<&ArmRun> = runs.iter().filter(|r| r.eval == p.eval.id).collect();
        mine.sort_by_key(|r| r.run);
        let results = json::array(
            mine.iter()
                .filter(|r| r.sides.iter().any(Option::is_some))
                .map(|r| {
                    let mut sides = Obj::new();
                    for (a, side) in meta.arms.iter().zip(&r.sides) {
                        sides = sides.raw(a.name(), &side_json(side));
                    }
                    Obj::new().num("run", r.run).raw("arms", &sides.end()).end()
                }),
        );
        Obj::new()
            .str("id", &p.eval.id)
            .num("runs", p.runs)
            .num("max_budget_usd", r6(p.eval.max_budget_usd))
            .raw("results", &results)
            .end()
    }));
    let obj = Obj::new()
        .str("spec_dna", VERSION)
        .opt_str("claude_code", meta.claude_code.as_deref())
        .raw("models", &models)
        .str("started", &meta.started)
        .strs("arms", &names);
    let obj = match meta.runs {
        Some(n) => obj.num("runs", n),
        None => obj.raw("runs", "null"),
    };
    let obj = obj
        .num("seed", verdict::SEED)
        .num("resamples", verdict::RESAMPLES)
        .num("max_total_usd", r6(meta.cap))
        .num("estimate_usd", r6(meta.estimate))
        .num("spend_usd", r6(state.spend))
        .raw("excluded", &excluded.end())
        .opt_str("stopped", stopped.map(Stop::name))
        .num("waves", waves);
    let obj = match error {
        Some(e) => obj.str("error", e),
        None => obj,
    };
    obj.raw("evals", &evals).end()
}

#[allow(clippy::too_many_arguments)]
fn arms_persist(
    repo: &Repo,
    meta: &ArmsMeta,
    plan: &[Planned],
    runs: &[ArmRun],
    state: &State,
    stopped: Option<Stop>,
    error: Option<&str>,
    written: &mut Option<String>,
) -> Result<String, Fatal> {
    let compact = arms_result_json(meta, plan, runs, state, stopped, error);
    match written {
        Some(path) => {
            replace_result(repo, path, &compact)?;
            Ok(path.clone())
        }
        None => {
            let path = crate::eval_run::write_result(
                repo,
                &meta.out_dir,
                "arms",
                &meta.started,
                &compact,
            )?;
            *written = Some(path.clone());
            Ok(path)
        }
    }
}

/// Reads the result of an earlier `--arms` run: a path of the repository
/// with no `..`, valid JSON, and the same arms in the same order.
fn read_arms_resume(repo: &Repo, file: &str, arms: &[HarnessArm]) -> Result<json::Value, Fatal> {
    let bad = |m: String| Fatal::Usage(format!("--resume `{file}`: {m}"));
    let v = read_result_file(repo, file)?;
    let had: Vec<&str> = v
        .get("arms")
        .as_arr()
        .iter()
        .filter_map(json::Value::as_str)
        .collect();
    let want: Vec<&str> = arms.iter().map(|a| a.name()).collect();
    if had.is_empty() {
        return Err(bad("the file is no result of `eval run --arms`".into()));
    }
    if had != want {
        return Err(bad(format!(
            "the file has the arms {}, and this run has {}; use the same arms in the same order",
            had.join(","),
            want.join(",")
        )));
    }
    Ok(v)
}

/// Reads the sides of a result of `--arms` into the plan.
fn read_arms_prior(v: &json::Value, plan: &[Planned], arms: &[HarnessArm]) -> (Prior, Vec<ArmRun>) {
    let mut runs = Vec::new();
    for item in v.get("evals").as_arr() {
        let Some(id) = item.get("id").as_str() else {
            continue;
        };
        if !plan.iter().any(|p| p.eval.id == id) {
            continue;
        }
        for r in item.get("results").as_arr() {
            let json::Value::Num(n) = r.get("run") else {
                continue;
            };
            let run = *n as u32;
            if run == 0 {
                continue;
            }
            let sides = arms
                .iter()
                .map(|a| read_side(r.get("arms").get(a.name())))
                .collect();
            runs.push(ArmRun {
                eval: id.to_string(),
                run,
                sides,
            });
        }
    }
    let ex = v.get("excluded");
    let num = |v: &json::Value| match v {
        json::Value::Num(n) => *n,
        _ => 0.0,
    };
    let prior = Prior {
        started: v
            .get("started")
            .as_str()
            .unwrap_or("1970-01-01T00:00:00Z")
            .to_string(),
        spend: num(v.get("spend_usd")),
        excluded: arms.iter().map(|a| num(ex.get(a.name())) as u32).collect(),
        models: v
            .get("models")
            .as_arr()
            .iter()
            .filter_map(|m| m.as_str().map(str::to_string))
            .collect(),
        pairs: Vec::new(),
    };
    (prior, runs)
}

/// `eval run --arms <arm>[,<arm>[,<arm>]] [<id>...]`: the plain evals of
/// `[evals] dir` in 1 to 3 harness arms, in waves (D57, D100). Wave `r` runs
/// each eval once in each arm, and the order of the arms rotates
/// (`rotation`). With 1 arm, a wave runs each eval once in that arm. The
/// result is `<dir>/results/arms-<time>.json`. A stop by the cap keeps the
/// complete waves, and the analysis drops a partial wave. The command gives
/// no pass rate, so it never exits with 1.
pub fn run_arms(repo: &Repo, cfg: &Config, o: &ArmsOpts) -> Result<u8, Fatal> {
    let first = first_wait()?;
    let limits = Limits::from_env()?;
    if o.resume.is_some() && (o.dry_run || o.out.is_some()) {
        return Err(Fatal::Usage(
            "--resume goes on in its own file; it takes no --dry-run and no --out".into(),
        ));
    }
    let table = cfg.evals.as_ref().ok_or_else(|| {
        Fatal::Config(
            "spec-dna.toml has no [evals] table, and `eval run --arms` needs it; add the table (docs/config.md)"
                .into(),
        )
    })?;
    let out_dir = match o.out.as_deref() {
        Some(p) => normalize_out(p, &table.dir)?,
        None => format!("{}/{}", table.dir, evals::RESULTS),
    };
    let cap = o.max_total_usd.unwrap_or(table.max_total_usd);
    let check_sh = repo.root.join(&table.dir).join(evals::CHECK_FILE);
    let all = load_evals(repo, table)?;
    if !check_sh.is_file() {
        return Err(Fatal::Config(format!(
            "{}/{}: the check file is absent; `eval run` sources it",
            table.dir,
            evals::CHECK_FILE
        )));
    }
    let resumed = match &o.resume {
        Some(file) => Some(read_arms_resume(repo, file, &o.arms)?),
        None => None,
    };
    // `--resume` takes the evals of its file. An id names the evals to go on
    // with, and it must be in the file. The other evals of the file keep their
    // results and their runs.
    let file_ids: Vec<String> = match &resumed {
        Some(v) => v
            .get("evals")
            .as_arr()
            .iter()
            .filter_map(|e| e.get("id").as_str().map(str::to_string))
            .collect(),
        None => Vec::new(),
    };
    let selected = match (&o.resume, &resumed) {
        (Some(file), Some(_)) => {
            if let Some(id) = o.ids.iter().find(|id| !file_ids.contains(id)) {
                return Err(Fatal::Usage(format!(
                    "--resume `{file}`: the file has no eval `{id}`; its evals are {}",
                    file_ids.join(", ")
                )));
            }
            select_ids(all, &file_ids, &table.dir)?
        }
        _ => select_ids(all, &o.ids, &table.dir)?,
    };
    let runs = match &resumed {
        Some(v) => {
            let file_runs = match v.get("runs") {
                json::Value::Num(n) => Some(*n as u32),
                _ => None,
            };
            match (o.runs, file_runs) {
                (None, f) => f,
                (Some(n), Some(f)) if n >= f => Some(n),
                (Some(n), Some(f)) => {
                    return Err(Fatal::Usage(format!(
                        "--runs {n} is below the file of --resume, which has --runs {f}; use {f} or more, or leave --runs out"
                    )));
                }
                (Some(_), None) => {
                    return Err(Fatal::Usage(
                        "--runs with a file of --resume that kept the runs of each eval; leave --runs out".to_string(),
                    ));
                }
            }
        }
        None => o.runs,
    };
    let mut plan = Vec::new();
    // The evals that this run goes on with. Without `--resume`, each eval.
    let mut active = Vec::new();
    for eval in selected {
        let go = o.ids.is_empty() || o.ids.contains(&eval.id);
        let mut n = runs.unwrap_or(eval.runs);
        if !go {
            // An eval that this run leaves keeps the runs of its file.
            let kept = resumed.as_ref().and_then(|v| {
                v.get("evals")
                    .as_arr()
                    .iter()
                    .find(|e| e.get("id").as_str() == Some(eval.id.as_str()))
                    .and_then(|e| match e.get("runs") {
                        json::Value::Num(k) => Some(*k as u32),
                        _ => None,
                    })
            });
            n = kept.unwrap_or(n);
        }
        plan.push(plan_eval(repo, eval, n)?);
        active.push(go);
    }
    let n_arms = o.arms.len() as u32;
    let todo = || {
        plan.iter()
            .zip(&active)
            .filter(|(_, a)| **a)
            .map(|(p, _)| p)
    };
    let total_runs: u32 = todo().map(|p| p.runs * n_arms).sum();
    let estimate: f64 = todo()
        .map(|p| f64::from(p.runs * n_arms) * p.eval.max_budget_usd)
        .sum();
    let names: Vec<&str> = o.arms.iter().map(|a| a.name()).collect();
    let header = format!(
        "{} evals, {} arms, {total_runs} runs, estimate {estimate:.2} USD, cap {cap:.2} USD",
        todo().count(),
        o.arms.len()
    );
    if o.dry_run {
        arms_dry_run(
            &plan, &names, &header, estimate, cap, total_runs, table, o.json,
        );
        return Ok(0);
    }

    let mut state = State {
        cap,
        first,
        spend: 0.0,
        excluded: vec![0; o.arms.len()],
        models: BTreeSet::new(),
        streak: 0,
        floor: result_floor(repo, table.min_free_mib, &out_dir, o.resume.as_deref()),
        low: None,
    };
    let mut started = utc_time(now_secs());
    let mut written: Option<String> = None;
    let mut results: Vec<ArmRun> = Vec::new();
    if let (Some(file), Some(v)) = (&o.resume, &resumed) {
        let (prior, prior_runs) = read_arms_prior(v, &plan, &o.arms);
        started = prior.started;
        state.spend = prior.spend;
        state.excluded = prior.excluded;
        state.models = prior.models;
        results = prior_runs;
        written = Some(file.clone());
    }
    let remaining: f64 = todo()
        .map(|p| {
            let done: u32 = results
                .iter()
                .filter(|r| r.eval == p.eval.id)
                .map(|r| r.sides.iter().filter(|s| s.is_some()).count() as u32)
                .sum();
            f64::from((p.runs * n_arms).saturating_sub(done)) * p.eval.max_budget_usd
        })
        .sum();
    if state.spend + remaining > cap + EPS {
        eprintln!(
            "eval run: note: the estimate {:.2} USD is above max-total-usd {cap:.2} USD; the run stops before a run that could pass the cap, and --resume goes on",
            state.spend + remaining
        );
    }
    make_out_dir(repo, &out_dir)?;
    let claude_code = claude_version()?;
    let harness = harness_list(repo)?;
    if !o.json {
        println!("eval run --arms {}: {header}", names.join(","));
    }
    announce_pin();
    catch_interrupts();
    let mut ctx = Ctx {
        limits,
        merge_instructions: false,
        harness,
        check_sh,
        isolate: true,
        counter: 0,
        arm: HarnessArm::Project,
        named: true,
    };
    let meta = ArmsMeta {
        claude_code,
        started: started.clone(),
        arms: o.arms.clone(),
        runs,
        cap,
        estimate,
        out_dir,
    };
    let mut stopped: Option<Stop> = None;
    let mut failed: Option<Fatal> = None;
    let waves = plan.iter().map(|p| p.runs).max().unwrap_or(0);
    // Wave, then eval, then arm: a stop by the cap leaves complete waves.
    'waves: for r in 1..=waves {
        for (ei, p) in plan.iter().enumerate() {
            if r > p.runs || !active[ei] {
                continue;
            }
            let at = match results
                .iter()
                .position(|x| x.eval == p.eval.id && x.run == r)
            {
                Some(at) => at,
                None => {
                    results.push(ArmRun {
                        eval: p.eval.id.clone(),
                        run: r,
                        sides: vec![None; o.arms.len()],
                    });
                    results.len() - 1
                }
            };
            for ai in rotation(o.arms.len(), ei, r) {
                if results[at].sides[ai].is_some() {
                    continue;
                }
                let arm = o.arms[ai];
                ctx.arm = arm;
                let edit = |dir: &Path| write_harness_arm(dir, arm, &[], true);
                let done = run_side(
                    &mut ctx,
                    &mut state,
                    &Job {
                        label: "harness",
                        p,
                        n: r,
                        arm_name: arm.name(),
                        arm_index: ai,
                        deny: None,
                        edit: &edit,
                    },
                );
                let done = match done {
                    Ok(d) => d,
                    Err(_) if interrupt_signal().is_some() => {
                        stopped = Some(Stop::Interrupted);
                        break 'waves;
                    }
                    Err(e) => {
                        stopped = Some(Stop::Error);
                        failed = Some(e);
                        break 'waves;
                    }
                };
                match done {
                    Done::Counted(side) => {
                        results[at].sides[ai] = Some(side);
                        // The result is on the disk after each side, as for
                        // `--ab`.
                        if let Err(e) = arms_persist(
                            repo,
                            &meta,
                            &plan,
                            &results,
                            &state,
                            Some(Stop::Interrupted),
                            None,
                            &mut written,
                        ) {
                            eprintln!("eval run: cannot write the partial result: {}", e.message());
                        }
                    }
                    Done::Stop(s) => {
                        stopped = Some(s);
                        break 'waves;
                    }
                }
            }
        }
    }

    let error = failed.as_ref().map(|e| error_text(repo, e.message()));
    let compact = arms_result_json(&meta, &plan, &results, &state, stopped, error.as_deref());
    if o.json {
        println!("{compact}");
    } else {
        for line in arms_text(&meta, &plan, &results, &state, stopped, error.as_deref()) {
            println!("{line}");
        }
    }
    let path = arms_persist(
        repo,
        &meta,
        &plan,
        &results,
        &state,
        stopped,
        error.as_deref(),
        &mut written,
    );
    if let Some(e) = failed {
        match &path {
            Ok(p) => eprintln!("eval run: the partial result is in {p}"),
            Err(w) => eprintln!(
                "eval run: the partial result is not written: {}",
                w.message()
            ),
        }
        return Err(e);
    }
    let path = path?;
    eprintln!("eval run: the result is in {path}");
    match stopped {
        None => Ok(0),
        Some(Stop::MaxTotal) => {
            eprintln!(
                "eval run: stopped: the next run could pass max-total-usd {cap:.2} USD (the spend is {:.2} USD); the partial result is written, and the analysis drops a partial wave",
                state.spend
            );
            Ok(3)
        }
        Some(Stop::Error) => unreachable!("an error returns above"),
        Some(Stop::DiskSpace) => {
            if let Some(low) = &state.low {
                eprintln!("{}", low.message(Some(&path)));
            }
            Ok(3)
        }
        Some(Stop::ApiError) => {
            eprintln!(
                "eval run: stopped (api-error): the API refused the attempts and the waits are over, or 5 attempts in a row failed with an error and no cost; the partial result is written. Run the arms again later with spec-dna eval run --arms {} --resume {path}",
                names.join(",")
            );
            Ok(3)
        }
        Some(Stop::Interrupted) => {
            let sig = interrupt_signal().unwrap_or(2);
            eprintln!(
                "eval run: interrupted; the partial result is written. Go on with spec-dna eval run --arms {} --resume {path}",
                names.join(",")
            );
            Ok((128 + sig) as u8)
        }
    }
}

/// The text of the result of `--arms`: a line for each arm, the waves, the
/// spend and the stop.
fn arms_text(
    meta: &ArmsMeta,
    plan: &[Planned],
    runs: &[ArmRun],
    state: &State,
    stopped: Option<Stop>,
    error: Option<&str>,
) -> Vec<String> {
    let waves = verdict::complete_waves(
        &plan
            .iter()
            .map(|p| {
                (
                    p.runs,
                    runs.iter()
                        .filter(|r| r.eval == p.eval.id)
                        .map(|r| (r.run, r.complete()))
                        .collect(),
                )
            })
            .collect::<Vec<_>>(),
    );
    let planned = plan.iter().map(|p| p.runs).max().unwrap_or(0);
    let mut lines = Vec::new();
    for (i, arm) in meta.arms.iter().enumerate() {
        let sides: Vec<&Side> = runs.iter().filter_map(|r| r.sides[i].as_ref()).collect();
        let passed = sides.iter().filter(|s| s.passed).count();
        // Not `sum`: of no floats it is -0.0.
        let cost = sides.iter().fold(0.0, |sum, s| sum + s.cost.unwrap_or(0.0));
        lines.push(format!(
            "{}: {passed} of {} runs passed, {cost:.2} USD",
            arm.name(),
            sides.len()
        ));
    }
    lines.push(format!(
        "waves: {waves} complete of {planned} planned; a partial wave is dropped"
    ));
    lines.push(format!("spend {:.2} USD", state.spend));
    if state.excluded.iter().any(|n| *n > 0) {
        let list: Vec<String> = meta
            .arms
            .iter()
            .zip(&state.excluded)
            .map(|(a, n)| format!("{} {n}", a.name()))
            .collect();
        lines.push(format!(
            "excluded attempts (the API refused them, no result counts them): {}",
            list.join(", ")
        ));
    }
    if let Some(s) = stopped {
        lines.push(format!("stopped: {}", s.name()));
    }
    if let Some(e) = error {
        lines.push(format!("error: {e}"));
    }
    lines
}

#[allow(clippy::too_many_arguments)]
fn arms_dry_run(
    plan: &[Planned],
    arms: &[&str],
    header: &str,
    estimate: f64,
    cap: f64,
    total_runs: u32,
    table: &Evals,
    json_out: bool,
) {
    let n = arms.len() as u32;
    if json_out {
        let list = json::array(plan.iter().map(|p| {
            Obj::new()
                .str("id", &p.eval.id)
                .num("runs", p.runs)
                .num("max_budget_usd", r6(p.eval.max_budget_usd))
                .num(
                    "estimate_usd",
                    r6(f64::from(p.runs * n) * p.eval.max_budget_usd),
                )
                .end()
        }));
        println!(
            "{}",
            Obj::new()
                .str("spec_dna", VERSION)
                .bool("dry_run", true)
                .strs(
                    "arms",
                    &arms.iter().map(|a| (*a).to_string()).collect::<Vec<_>>()
                )
                .str("evals_dir", &table.dir)
                .num("max_total_usd", r6(cap))
                .num("estimate_usd", r6(estimate))
                .num("runs", total_runs)
                .bool("above_cap", estimate > cap + EPS)
                .raw("evals", &list)
                .end()
        );
        return;
    }
    println!("eval run --arms {} (dry run): {header}", arms.join(","));
    if estimate > cap + EPS {
        println!(
            "a real run starts, and stops before a run that could pass the cap: the estimate {estimate:.2} USD is above max-total-usd {cap:.2} USD; --resume goes on"
        );
    }
    for p in plan {
        println!(
            "{}: {} runs, max-budget-usd {:.2}, estimate {:.2} USD",
            p.eval.id,
            p.runs,
            p.eval.max_budget_usd,
            f64::from(p.runs * n) * p.eval.max_budget_usd
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules_of(text: &str) -> toml::Table {
        let t: toml::Table = text.parse().unwrap();
        t["rules"].as_table().unwrap().clone()
    }

    /// The arm `without` of the skill `review` has no agent file `acceptor.md`
    /// either (design 0.8.0, D57 and D71): the role comes with the skill, so the
    /// edit of the skill list is the whole edit.
    #[test]
    fn the_arm_without_of_the_skill_review_has_no_acceptor_file() {
        for arm in [HarnessArm::Lean, HarnessArm::Full] {
            let has_file = |skills: Vec<Skill>| {
                let mut h = arm.harness().expect("a harness");
                h.skills = skills;
                harness::generated_files(&h)
                    .iter()
                    .any(|g| g.path == ".claude/agents/acceptor.md")
            };
            let with = arm_skills(Kind::Skill(Skill::Review), Arm::With, arm);
            let without = arm_skills(Kind::Skill(Skill::Review), Arm::Without, arm);
            assert!(with.contains(&Skill::Review), "{}", arm.name());
            assert!(!without.contains(&Skill::Review), "{}", arm.name());
            assert!(has_file(with), "{}", arm.name());
            assert!(!has_file(without), "{}", arm.name());
        }
    }

    #[test]
    fn a_file_with_no_rules_gets_the_table() {
        for text in ["spec-dna = \"0.7.0\"\n", "spec-dna = \"0.7.0\"", ""] {
            let new = rules_off(text).unwrap();
            let rules = rules_of(&new);
            assert_eq!(rules["SD501"].as_str(), Some("off"), "{new}");
            assert_eq!(rules["SD502"].as_str(), Some("off"), "{new}");
        }
        // The table goes after the other tables, so no key of them moves.
        let text = "spec-dna = \"0.7.0\"\n\n[[roots]]\npath = \"docs\"\n";
        let new = rules_off(text).unwrap();
        assert!(new.starts_with(text), "{new}");
        let all: toml::Table = new.parse().unwrap();
        assert_eq!(all["roots"][0]["path"].as_str(), Some("docs"));
    }

    #[test]
    fn a_table_of_rules_keeps_its_keys() {
        let text = "spec-dna = \"0.7.0\"\n\n[rules]\nlint = \"warning\"  # mine\nSD502 = \"error\"\n\n[[roots]]\npath = \"docs\"\n";
        let new = rules_off(text).unwrap();
        let rules = rules_of(&new);
        assert_eq!(rules["lint"].as_str(), Some("warning"), "{new}");
        assert_eq!(rules["SD501"].as_str(), Some("off"), "{new}");
        assert_eq!(rules["SD502"].as_str(), Some("off"), "{new}");
        assert!(new.contains("lint = \"warning\"  # mine"), "{new}");
        // A second call changes nothing.
        assert_eq!(rules_off(&new).unwrap(), new);
    }

    #[test]
    fn a_file_that_the_edit_cannot_change_gives_an_error_and_the_copy_keeps_it() {
        // Dotted keys, and a text that does not parse.
        assert!(rules_off("spec-dna = \"0.7.0\"\nrules.lint = \"off\"\n").is_err());
        assert!(rules_off("this is not toml").is_err());
        let dir = std::env::temp_dir().join(format!("spec-dna-rules-off-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let text = "spec-dna = \"0.7.0\"\nrules.lint = \"off\"\n";
        fs::write(dir.join("spec-dna.toml"), text).unwrap();
        adapter_rules_off(&dir).unwrap();
        assert_eq!(fs::read_to_string(dir.join("spec-dna.toml")).unwrap(), text);
        // A folder with no file is not an error.
        fs::remove_file(dir.join("spec-dna.toml")).unwrap();
        adapter_rules_off(&dir).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    /// Design 0.8.0, D76 item 3: the arm `with` writes the text of the profile of the
    /// project of the copy. A copy with no file, or a file that does not parse, has
    /// the default profile.
    #[test]
    fn the_skill_file_of_the_arm_with_follows_the_profile_of_the_copy() {
        let dir = std::env::temp_dir().join(format!("spec-dna-profile-of-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(profile_of(&dir), Profile::Standard);
        for (text, want) in [
            ("spec-dna = \"0.7.1\"\nprofile = \"lite\"\n", Profile::Lite),
            (
                "spec-dna = \"0.7.1\"\nprofile = \"strict\"\n",
                Profile::Strict,
            ),
            ("spec-dna = \"0.7.1\"\n", Profile::Standard),
            ("this is not toml", Profile::Standard),
        ] {
            fs::write(dir.join("spec-dna.toml"), text).unwrap();
            assert_eq!(profile_of(&dir), want, "{text}");
        }
        let _ = fs::remove_dir_all(&dir);
        // `lite` has the frozen text of 0.7.1, the other profiles the text of
        // this release.
        let lite = skill_file(Skill::Review, Profile::Lite);
        let standard = skill_file(Skill::Review, Profile::Standard);
        assert!(!lite.contains("acceptance.toml"), "{lite}");
        assert!(standard.contains("acceptance.toml"), "{standard}");
        assert_eq!(standard, skill_file(Skill::Review, Profile::Strict));
    }

    /// A harness arm writes the files of the generator with the profile of the
    /// copy: in `lite`, the text of 0.7.1 and no agent `acceptor` (design 0.8.0,
    /// D71 and D76).
    #[test]
    fn a_harness_arm_writes_the_files_of_the_profile_of_the_copy() {
        for (profile, lite) in [("lite", true), ("standard", false)] {
            let dir = std::env::temp_dir().join(format!(
                "spec-dna-harness-arm-{}-{profile}",
                std::process::id()
            ));
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("spec-dna.toml"),
                format!("spec-dna = \"0.7.1\"\nprofile = \"{profile}\"\n"),
            )
            .unwrap();
            write_harness_arm(&dir, HarnessArm::Lean, &[Skill::Review], false).unwrap();
            let skill = fs::read_to_string(dir.join(Skill::Review.path())).unwrap();
            assert_eq!(skill.contains("acceptance.toml"), !lite, "{profile}");
            assert_eq!(
                dir.join(".claude/agents/acceptor.md").exists(),
                !lite,
                "{profile}"
            );
            let _ = fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn an_inline_table_of_rules_keeps_its_keys() {
        let new = rules_off("spec-dna = \"0.7.0\"\nrules = { lint = \"off\" }\n").unwrap();
        let rules = rules_of(&new);
        assert_eq!(rules["lint"].as_str(), Some("off"), "{new}");
        assert_eq!(rules["SD501"].as_str(), Some("off"), "{new}");
        assert_eq!(rules["SD502"].as_str(), Some("off"), "{new}");
    }
}
