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
    Ctx, EPS, Planned, RETRIES, RunResult, claude_version, copy_list, first_wait, harness_list,
    load_evals_in, make_out_dir, normalize_out, now_secs, one_attempt, r6, utc_time, wait_seconds,
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
/// The evals of the pair `agents-line`: the first eval of each skill that a
/// discipline names.
pub const AGENTS_LINE_EVALS: [&str; 5] = ["tdd-1", "debug-1", "verify-1", "review-1", "execute-1"];

/// The options of `eval run --ab`.
pub struct Opts {
    pub target: String,
    pub runs: Option<u32>,
    pub max_total_usd: Option<f64>,
    pub out: Option<String>,
    pub dry_run: bool,
    pub isolate: bool,
    pub json: bool,
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
struct Side {
    claude_exit: i32,
    subtype: Option<String>,
    cost: Option<f64>,
    charged: f64,
    excluded: u32,
    passed: bool,
}

struct PairRec {
    eval: String,
    run: u32,
    with: Side,
    without: Side,
}

/// Why a run stopped.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    MaxTotal,
    ApiError,
    /// A git or file error ended the run. The error is in the result.
    Error,
}

impl Stop {
    fn name(self) -> &'static str {
        match self {
            Stop::MaxTotal => "max-total-usd",
            Stop::ApiError => "api-error",
            Stop::Error => "error",
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
    let groups = select(repo, all, &target, o.runs, &pack_dir)?;

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
    if estimate > cap + EPS {
        return Err(Fatal::Config(format!(
            "the estimate {estimate:.2} USD (runs x 2 arms x max-budget-usd) is above max-total-usd {cap:.2} USD; no run started. Lower the runs, or raise max-total-usd"
        )));
    }
    let header =
        format!("{n_evals} evals, {total_runs} runs, estimate {estimate:.2} USD, cap {cap:.2} USD");
    if o.dry_run {
        dry_run(
            &groups, &o.target, &header, estimate, cap, total_runs, table, o.json,
        );
        return Ok(0);
    }

    // A bad `--out` must stop the command before the first run spends money.
    make_out_dir(repo, &out_dir)?;
    let claude_code = claude_version()?;
    let harness = harness_list(repo)?;
    if !o.json {
        println!("eval run --ab {}: {header}", o.target);
    }
    let started = utc_time(now_secs());
    let mut ctx = Ctx {
        harness,
        check_sh,
        isolate: o.isolate,
        counter: 0,
    };
    let mut state = State {
        cap,
        first,
        spend: 0.0,
        excluded: [0, 0],
        models: BTreeSet::new(),
    };
    let mut pairs: Vec<Vec<PairRec>> = groups.iter().map(|_| Vec::new()).collect();
    let mut stopped: Option<Stop> = None;
    // A git or file error ends the loop, but the paid runs are not lost: the
    // result is written before the command exits.
    let mut failed: Option<Fatal> = None;
    let planned_pairs: u32 = total_runs / 2;
    let mut done_pairs = 0;
    'groups: for (gi, g) in groups.iter().enumerate() {
        for p in &g.evals {
            for n in 1..=p.runs {
                // `with` runs first for an odd number, `without` for an even one.
                let order = if n % 2 == 1 {
                    [Arm::With, Arm::Without]
                } else {
                    [Arm::Without, Arm::With]
                };
                let mut sides: [Option<Side>; 2] = [None, None];
                for arm in order {
                    let done = match run_arm(&mut ctx, &mut state, g, p, arm, n) {
                        Ok(d) => d,
                        Err(e) => {
                            stopped = Some(Stop::Error);
                            failed = Some(e);
                            break 'groups;
                        }
                    };
                    match done {
                        Done::Counted(side) => sides[arm.index()] = Some(side),
                        Done::Stop(s) => {
                            stopped = Some(s);
                            break 'groups;
                        }
                    }
                }
                let [Some(with), Some(without)] = sides else {
                    unreachable!("both arms are done")
                };
                pairs[gi].push(PairRec {
                    eval: p.eval.id.clone(),
                    run: n,
                    with,
                    without,
                });
                done_pairs += 1;
                if state.spend > cap + EPS && done_pairs < planned_pairs {
                    stopped = Some(Stop::MaxTotal);
                    break 'groups;
                }
            }
        }
    }

    let items: Vec<Item> = groups
        .iter()
        .zip(&pairs)
        .map(|(g, ps)| item(g, ps))
        .collect();
    let result = ResultFile {
        claude_code,
        models: state.models.iter().cloned().collect(),
        started: started.clone(),
        target: o.target.clone(),
        cap,
        estimate,
        spend: state.spend,
        excluded: state.excluded,
        stopped,
        error: failed.as_ref().map(|e| error_text(repo, e.message())),
        items,
    };
    let compact = result_json(&result);
    if o.json {
        println!("{compact}");
    } else {
        for line in result_text(&result) {
            println!("{line}");
        }
    }
    let path = write_result(repo, &out_dir, &started, &compact);
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
                "eval run: stopped: the spend {:.2} USD is above max-total-usd {cap:.2} USD; the partial result is written",
                state.spend
            );
            Ok(3)
        }
        Some(Stop::Error) => unreachable!("an error returns above"),
        Some(Stop::ApiError) => {
            eprintln!(
                "eval run: stopped (api-error): the API refused the attempt {} times in a row, and the waits are over; the partial result is written. Run the target again later",
                RETRIES + 1
            );
            Ok(3)
        }
    }
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
}

enum Done {
    Counted(Side),
    Stop(Stop),
}

/// Runs one arm of one pair. An attempt that the API refused is excluded: no
/// result counts it, and the spend gets the cost that it reports (0 if it
/// reports none). The retry makes a new copy, with the same eval, arm and run.
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
    loop {
        eprintln!(
            "eval run: {} {} run {n} of {} ({})",
            g.name,
            p.eval.id,
            p.runs,
            arm.name()
        );
        let edit = |dir: &Path| prepare(dir, g.kind, arm);
        let r: RunResult = one_attempt(ctx, p, &edit, deny)?;
        state.models.extend(r.models.iter().cloned());
        if !r.refused {
            state.spend += r.charged;
            return Ok(Done::Counted(Side {
                claude_exit: r.claude_exit,
                subtype: r.subtype,
                cost: r.cost,
                charged: r.charged,
                excluded,
                passed: r.passed,
            }));
        }
        state.spend += r.cost.unwrap_or(0.0);
        state.excluded[arm.index()] += 1;
        excluded += 1;
        if state.spend > state.cap + EPS {
            return Ok(Done::Stop(Stop::MaxTotal));
        }
        if excluded > RETRIES {
            return Ok(Done::Stop(Stop::ApiError));
        }
        let wait = wait_seconds(state.first, excluded - 1);
        eprintln!(
            "eval run: the API refused the attempt ({} {} run {n}, {}); it is excluded. wait {wait} seconds, then try again",
            g.name,
            p.eval.id,
            arm.name()
        );
        std::thread::sleep(std::time::Duration::from_secs(wait));
    }
}

/// Chooses the evals of the target, and the files of each fixture.
fn select(
    repo: &Repo,
    all: Vec<Eval>,
    target: &Target,
    runs: Option<u32>,
    pack_dir: &str,
) -> Result<Vec<Group>, Fatal> {
    let plan = |eval: Eval| -> Result<Planned, Fatal> {
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
    pairs: Vec<PairView>,
    /// `None` when no pair is done.
    analysis: Option<Analysis>,
    /// True when each planned pair is done.
    complete: bool,
}

struct PairView {
    eval: String,
    run: u32,
    with: SideView,
    without: SideView,
}

struct SideView {
    claude_exit: i32,
    subtype: Option<String>,
    cost: Option<f64>,
    excluded: u32,
    passed: bool,
}

fn view(s: &Side) -> SideView {
    SideView {
        claude_exit: s.claude_exit,
        subtype: s.subtype.clone(),
        cost: s.cost,
        excluded: s.excluded,
        passed: s.passed,
    }
}

fn item(g: &Group, pairs: &[PairRec]) -> Item {
    let by_eval: Vec<Vec<Pair>> = g
        .evals
        .iter()
        .map(|p| {
            pairs
                .iter()
                .filter(|r| r.eval == p.eval.id)
                .map(|r| Pair {
                    with_passed: r.with.passed,
                    with_cost: r.with.charged,
                    without_passed: r.without.passed,
                    without_cost: r.without.charged,
                })
                .collect()
        })
        .collect();
    let planned: u32 = g.evals.iter().map(|p| p.runs).sum();
    let complete = pairs.len() as u32 == planned;
    Item {
        name: g.name.clone(),
        evals: g.evals.len(),
        pairs: pairs
            .iter()
            .map(|r| PairView {
                eval: r.eval.clone(),
                run: r.run,
                with: view(&r.with),
                without: view(&r.without),
            })
            .collect(),
        analysis: (!pairs.is_empty()).then(|| verdict::analyze(&by_eval)),
        complete,
    }
}

struct ResultFile {
    claude_code: Option<String>,
    models: Vec<String>,
    started: String,
    target: String,
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

fn interval(i: &verdict::Interval) -> String {
    Obj::new()
        .num("value", r6(i.value))
        .num("low", r6(i.low))
        .num("high", r6(i.high))
        .end()
}

fn arm_json(a: Option<&verdict::Arm>) -> String {
    let zero = verdict::Arm {
        runs: 0,
        passes: 0,
        cost: 0.0,
    };
    let a = a.unwrap_or(&zero);
    let obj = Obj::new()
        .num("runs", a.runs)
        .num("passes", a.passes)
        .num("pass_rate", r6(a.pass_rate()))
        .num("cost_usd", r6(a.cost));
    match a.cost_per_correct() {
        Some(c) => obj.num("cost_per_correct_usd", r6(c)),
        None => obj.raw("cost_per_correct_usd", "null"),
    }
    .end()
}

fn side_json(s: &SideView) -> String {
    let obj = Obj::new()
        .num("claude_exit", s.claude_exit)
        .opt_str("subtype", s.subtype.as_deref());
    let obj = match s.cost {
        Some(c) => obj.num("cost_usd", r6(c)),
        None => obj.raw("cost_usd", "null"),
    };
    obj.num("excluded", s.excluded)
        .bool("passed", s.passed)
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
        .raw("with", &arm_json(a.map(|a| &a.with)))
        .raw("without", &arm_json(a.map(|a| &a.without)));
    let obj = match a {
        Some(a) => obj.raw("difference", &interval(&a.difference)),
        None => obj.raw("difference", "null"),
    };
    let obj = match a.and_then(|a| a.cost_ratio.as_ref()) {
        Some(r) => obj.raw("cost_ratio", &interval(r)),
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
        .str("target", &r.target)
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
                .raw("evals", &list)
                .end()
        );
        return;
    }
    println!("eval run --ab {target} (dry run): {header}");
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
