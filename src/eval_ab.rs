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
//!
//! The result keeps counts, costs, names and versions. It keeps no task text,
//! no check command and no output text of `claude`.

use crate::adapters::{self, VERSION};
use crate::config::{Config, Evals};
use crate::eval_run::{
    Ctx, EPS, Limits, Planned, Refusal, RunResult, Tokens, catch_interrupts, claude_version,
    copy_list, first_wait, harness_list, interrupt_signal, load_evals_in, make_out_dir,
    normalize_out, now_secs, one_attempt, r6, replace_result, retry_waits, utc_time,
};
use crate::evals::{self, Eval};
use crate::harness::Skill;
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
/// The evals of the pair `agents-line`: the first eval of `tdd`, `debug` and
/// `execute`, and 2 evals that name the group (`skill = "agents-line"`).
pub const AGENTS_LINE_EVALS: [&str; 5] = [
    "tdd-1",
    "debug-1",
    "execute-1",
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
}

impl Stop {
    fn name(self) -> &'static str {
        match self {
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
}

impl Target {
    /// Reads the target: a skill, `all` or `agents-line`. Anything else is a
    /// usage error.
    pub fn parse(s: &str) -> Result<Target, Fatal> {
        if s == "all" {
            Ok(Target::All)
        } else if s == AGENTS_LINE {
            Ok(Target::AgentsLine)
        } else if Skill::from_key(s).is_some() {
            Ok(Target::Skill(s.to_string()))
        } else {
            Err(Fatal::Usage(format!(
                "`{s}` is no target of `eval run --ab`; use a skill ({}), `all` or `{AGENTS_LINE}`",
                crate::harness::skill_keys()
            )))
        }
    }
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
        excluded: [0, 0],
        models: BTreeSet::new(),
        streak: 0,
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

    // A bad `--out` must stop the command before the first run spends money.
    make_out_dir(repo, &out_dir)?;
    let claude_code = claude_version()?;
    let harness = harness_list(repo)?;
    if !o.json {
        println!("eval run --ab {}: {header}", o.target);
    }
    catch_interrupts();
    let mut ctx = Ctx {
        limits,
        merge_instructions: true,
        harness,
        check_sh,
        isolate: o.isolate,
        counter: 0,
    };
    let meta = Meta {
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
                        }
                        Done::Stop(s) => {
                            stopped = Some(s);
                            break 'groups;
                        }
                    }
                }
                // The result is on the disk after each pair, so a crash or a
                // kill loses no paid run. The file says `interrupted` until
                // the end.
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
    excluded: [u32; 2],
    models: BTreeSet<String>,
    /// The excluded attempts in a row that are errors with no cost and no
    /// marker (`Refusal::Silent`). A real run resets it.
    streak: u32,
}

/// The excluded errors with no cost in a row that stop the command.
const STREAK_STOP: u32 = 5;

enum Done {
    Counted(Side),
    Stop(Stop),
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
        claude_code: meta.claude_code.clone(),
        models: state.models.iter().cloned().collect(),
        started: meta.started.clone(),
        target: meta.target.clone(),
        runs: meta.runs,
        cap: meta.cap,
        estimate: meta.estimate,
        spend: state.spend,
        excluded: state.excluded,
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

/// Runs one arm of one pair. An attempt that the API refused is excluded: no
/// result counts it, and the spend gets the cost that it reports (0 if it
/// reports none). The retry makes a new copy, with the same eval, arm and run.
/// The cap is checked before each attempt: if the spend and the budget of the
/// next run could pass it, the run does not start.
fn run_arm(
    ctx: &mut Ctx,
    state: &mut State,
    g: &Group,
    p: &Planned,
    arm: Arm,
    n: u32,
) -> Result<Done, Fatal> {
    let deny = (arm == Arm::Without).then_some(DENIED);
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
        eprintln!(
            "eval run: {} {} run {n} of {} ({})",
            g.name,
            p.eval.id,
            p.runs,
            arm.name()
        );
        let edit = |dir: &Path| prepare(dir, g.kind, arm);
        let r: RunResult = one_attempt(ctx, p, &edit, deny)?;
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
            }));
        };
        state.spend += r.cost.unwrap_or(0.0);
        state.excluded[arm.index()] += 1;
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
            g.name,
            p.eval.id,
            arm.name()
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
    excluded: [u32; 2],
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
    let side = |v: &json::Value| -> Option<Side> {
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
        })
    };
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
        excluded: [
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
    let v = json::parse(&text).map_err(|e| bad(format!("it is not valid JSON ({e})")))?;
    if v.get("target").as_str() != Some(target) {
        return Err(bad(format!(
            "the file is the result of the target `{}`, not of `{target}`",
            v.get("target").as_str().unwrap_or("?")
        )));
    }
    Ok(v)
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
        let pass_at = eval.pass_at.min(runs);
        let fixture = copy_list(
            repo,
            &format!(":(literal){}", eval.fixture),
            &format!("{}/", eval.fixture),
        )?;
        Ok(Planned {
            eval,
            runs,
            pass_at,
            fixture,
        })
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
                for e in all
                    .iter()
                    .filter(|e| e.skill.as_deref() == Some(skill.key()))
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
/// fixture never gives exit code 3 for a version.
fn prepare(dir: &Path, kind: Kind, arm: Arm) -> Result<(), Fatal> {
    pin_version(dir)?;
    let io = |what: &str, e: std::io::Error| {
        Fatal::Io(format!("cannot change `{what}` in the copy: {e}"))
    };
    match kind {
        Kind::Skill(skill) => {
            let rel = skill.path();
            let file = dir.join(&rel);
            match arm {
                Arm::With => {
                    if let Some(parent) = file.parent() {
                        fs::create_dir_all(parent).map_err(|e| io(&rel, e))?;
                    }
                    fs::write(&file, skill_file(skill)).map_err(|e| io(&rel, e))?;
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

/// The file of a skill in the arm `with`: the front matter and the text of
/// `skill show`.
fn skill_file(skill: Skill) -> String {
    format!(
        "---\nname: {}\ndescription: {}\n---\n\n{}\n",
        skill.name(),
        harness_text::description(skill),
        harness_text::body(skill)
    )
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
    claude_code: Option<String>,
    models: Vec<String>,
    started: String,
    target: String,
    runs: Option<u32>,
    cap: f64,
    estimate: f64,
    spend: f64,
    excluded: [u32; 2],
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
        Some(t) => obj.raw(
            "tokens",
            &Obj::new()
                .num("in", t.input)
                .num("out", t.output)
                .num("cache_read", t.cache_read)
                .num("cache_write", t.cache_write)
                .end(),
        ),
        None => obj.raw("tokens", "null"),
    };
    let checks = json::array(
        s.checks
            .iter()
            .map(|(name, passed)| Obj::new().str("name", name).bool("passed", *passed).end()),
    );
    obj.strs("models", &s.models).raw("checks", &checks).end()
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
    obj.raw("pairs", &pairs).end()
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
        .str("target", &r.target);
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
