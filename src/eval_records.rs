//! `spec-dna eval records <result file>`: the committed records of the runs
//! of `eval run --ab` (ADR 0016, eval-records change).
//!
//! The command reads one result file, and prints one record for each run of
//! each arm of each item that has a verdict, as JSON Lines on stdout. A record
//! has 9 keys: `skill`, `eval`, `run`, `arm`, `pass`, `cost`, `tokens`,
//! `models` and `claude_code`. It has no path, no text and no time.
//!
//! The writer is an allowlist. It builds each record from 9 values (12 for a
//! result of 0.8.0). It checks each value against its form, and it never copies
//! an object of the input.
//! A value outside its form stops the import. A message names the key and
//! never the value, because the value can be a secret. After the records are
//! built, the command computes each item again from them (`verdict::analyze`)
//! and compares the result with the item of the file. A mismatch stops the
//! import, and nothing is printed.

use crate::eval_run::{HarnessArm, r6};
use crate::json::{self, Obj, Value};
use crate::repo::Fatal;
use crate::verdict::{self, Pair};
use std::collections::BTreeSet;
use std::io::Read;

/// The most runs of one pair in a record.
const MAX_RUN: f64 = 100.0;
/// The most models of one record.
const MAX_MODELS: usize = 8;
/// The largest result file that the command reads: 16 MiB. A result of the
/// full run is about 100 KB.
const MAX_FILE: u64 = 16 * 1024 * 1024;
/// The most turns of one record.
const MAX_TURNS: u64 = 1_000_000;
/// The most seconds of one record.
const MAX_SECONDS: f64 = 10_000_000.0;
/// The tolerance of a cost sum: the records keep costs of 6 decimals.
const COST_TOLERANCE: f64 = 1e-5 + 1e-9;
/// The tolerance of the interval of the cost ratio.
const RATIO_TOLERANCE: f64 = 1e-4 + 1e-9;
/// The tolerance of the interval of the difference: both sides are rounded to
/// 6 decimals.
const DIFFERENCE_TOLERANCE: f64 = 1.5e-6;

/// True if a string has the shape of a secret or of a path of a machine: an
/// API key, a token of a host, an access key, a private key, a home folder or
/// an address. The forms below accept the characters of such values, so a
/// record refuses them by shape too.
fn secret_shaped(s: &str) -> bool {
    s.starts_with("sk-")
        || s.contains("-sk-")
        || [
            "ghp_",
            "gho_",
            "ghs_",
            "ghu_",
            "github_pat_",
            "AKIA",
            "-----BEGIN",
        ]
        .iter()
        .any(|p| s.contains(p))
        || ["Users/", "/private/", "/home/", "@"]
            .iter()
            .any(|p| s.contains(p))
}

/// True for an eval id: `^[a-z][a-z0-9-]{0,39}$`.
pub fn valid_eval_id(s: &str) -> bool {
    let b = s.as_bytes();
    (1..=40).contains(&b.len())
        && !secret_shaped(s)
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

/// True for a model name: 1 to 64 characters from `A-Z a-z 0-9 . _ : - [ ] /`,
/// with no `..` and no `/` at the start. Claude Code writes names such as
/// `claude-fable-5-2[1m]`. A path-like value is not a name.
pub fn valid_model(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._:-[]/".contains(&c))
        && !s.contains("..")
        && !s.starts_with('/')
        && !secret_shaped(s)
}

/// True for a version of Claude Code: `^[0-9A-Za-z.+-]{1,32}$`.
pub fn valid_claude_code(s: &str) -> bool {
    (1..=32).contains(&s.len())
        && !secret_shaped(s)
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".+-".contains(&c))
}

/// The tokens of one record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Tokens {
    input: u64,
    output: u64,
    cache_read: u64,
    cache_write: u64,
}

/// The 3 keys of 0.8.0 (ADR 0030).
struct Extra {
    /// The harness arm of the copy.
    harness: String,
    turns: Option<u32>,
    seconds: Option<f64>,
}

/// One record: the 9 values, and the 3 values of 0.8.0 if the file has them.
struct Record {
    skill: String,
    eval: String,
    run: u32,
    with: bool,
    /// The name of the arm of a batch of `--arms`. `None` for `with` and
    /// `without`.
    arm: Option<String>,
    extra: Option<Extra>,
    pass: bool,
    cost: Option<f64>,
    tokens: Option<Tokens>,
    models: Vec<String>,
    claude_code: Option<String>,
}

impl Record {
    fn line(&self) -> String {
        let obj = Obj::new()
            .str("skill", &self.skill)
            .str("eval", &self.eval)
            .num("run", self.run)
            .str(
                "arm",
                self.arm
                    .as_deref()
                    .unwrap_or(if self.with { "with" } else { "without" }),
            )
            .bool("pass", self.pass);
        let obj = match self.cost {
            Some(c) => obj.num("cost", c),
            None => obj.raw("cost", "null"),
        };
        let obj = match &self.tokens {
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
        let obj = obj
            .strs("models", &self.models)
            .opt_str("claude_code", self.claude_code.as_deref());
        let Some(x) = &self.extra else {
            return obj.end();
        };
        let obj = obj.str("harness", &x.harness);
        let obj = match x.turns {
            Some(n) => obj.num("turns", n),
            None => obj.raw("turns", "null"),
        };
        match x.seconds {
            Some(n) => obj.num("seconds", n),
            None => obj.raw("seconds", "null"),
        }
        .end()
    }
}

/// The error for a key that has no value in its form. The message names the
/// key and never the value.
fn bad(key: &str, form: &str) -> Fatal {
    Fatal::Config(format!(
        "eval records: the key `{key}` has a value outside its form ({form}); no record is written"
    ))
}

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Num(n) if n.is_finite() => Some(*n),
        _ => None,
    }
}

/// A count: a number with no fraction, of 0 or more.
fn count(v: &Value) -> Option<u64> {
    num(v)
        .filter(|n| *n >= 0.0 && n.fract() == 0.0 && *n < 9e15)
        .map(|n| n as u64)
}

/// The `turns` of a side: absent or `null` gives `None`, and a count gives
/// the count.
fn turns_of(v: &Value) -> Result<Option<u32>, Fatal> {
    match v {
        Value::Null => Ok(None),
        v => count(v)
            .filter(|n| *n <= MAX_TURNS)
            .map(|n| Some(n as u32))
            .ok_or_else(|| bad("turns", "null, or an integer from 0 to 1000000")),
    }
}

/// The `seconds` of a side: absent or `null` gives `None`, and a duration of
/// 0 to 10,000,000 seconds gives the number.
fn seconds_of(v: &Value) -> Result<Option<f64>, Fatal> {
    match v {
        Value::Null => Ok(None),
        v => num(v)
            .filter(|n| (0.0..=MAX_SECONDS).contains(n))
            .map(|n| Some(r6(n)))
            .ok_or_else(|| bad("seconds", "null, or a number from 0 to 10000000")),
    }
}

/// The harness arm of a result: a name of `HarnessArm`.
fn harness_of(v: &Value) -> Result<String, Fatal> {
    v.as_str()
        .and_then(|s| HarnessArm::parse(s).ok())
        .map(|a| a.name().to_string())
        .ok_or_else(|| bad("harness", "the name of a harness arm"))
}

/// The names of a list of models, each checked, or `None` if the key is
/// absent or `null`.
fn models_of(v: &Value) -> Result<Option<Vec<String>>, Fatal> {
    const FORM: &str =
        "a list of 1 to 8 names of 1 to 64 characters from A-Z a-z 0-9 . _ : - [ ] /, with no `..`";
    let Value::Arr(items) = v else {
        return match v {
            Value::Null => Ok(None),
            _ => Err(bad("models", FORM)),
        };
    };
    let mut names = BTreeSet::new();
    for item in items {
        match item.as_str() {
            Some(s) if valid_model(s) => {
                names.insert(s.to_string());
            }
            _ => return Err(bad("models", FORM)),
        }
    }
    if names.len() > MAX_MODELS {
        return Err(bad("models", FORM));
    }
    Ok(Some(names.into_iter().collect()))
}

fn tokens_of(v: &Value) -> Result<Option<Tokens>, Fatal> {
    const FORM: &str = "null, or 4 integers of 0 or more";
    match v {
        Value::Null => Ok(None),
        Value::Obj(_) => {
            let n = |k: &str| count(v.get(k)).ok_or_else(|| bad("tokens", FORM));
            Ok(Some(Tokens {
                input: n("in")?,
                output: n("out")?,
                cache_read: n("cache_read")?,
                cache_write: n("cache_write")?,
            }))
        }
        _ => Err(bad("tokens", FORM)),
    }
}

/// The records of one item, and its pairs for the self-check.
struct Built {
    records: Vec<Record>,
    /// The ids of the evals, in the order of their first appearance.
    evals: Vec<String>,
}

fn build_item(
    name: &str,
    item: &Value,
    file_models: &[String],
    claude_code: &Option<String>,
    harness: &Option<String>,
) -> Result<Built, Fatal> {
    let pairs = item.get("pairs").as_arr();
    if pairs.is_empty() {
        return Err(Fatal::Config(format!(
            "eval records: the run of `{name}` is not complete; finish it with --resume"
        )));
    }
    let mut records = Vec::new();
    let mut evals: Vec<String> = Vec::new();
    let mut seen: BTreeSet<(String, u32)> = BTreeSet::new();
    for pair in pairs {
        let eval = pair
            .get("eval")
            .as_str()
            .filter(|s| valid_eval_id(s))
            .ok_or_else(|| bad("eval", "1 to 40 characters, a-z first, then a-z 0-9 -"))?;
        let run = num(pair.get("run"))
            .filter(|n| n.fract() == 0.0 && (1.0..=MAX_RUN).contains(n))
            .ok_or_else(|| bad("run", "an integer from 1 to 100"))? as u32;
        if !seen.insert((eval.to_string(), run)) {
            return Err(bad("pairs", "each eval and run once"));
        }
        if !evals.iter().any(|e| e == eval) {
            evals.push(eval.to_string());
        }
        for with in [true, false] {
            let side = pair.get(if with { "with" } else { "without" });
            if !matches!(side, Value::Obj(_)) {
                return Err(Fatal::Config(format!(
                    "eval records: the run of `{name}` is not complete; finish it with --resume"
                )));
            }
            let pass = match side.get("passed") {
                Value::Bool(b) => *b,
                _ => return Err(bad("passed", "true or false")),
            };
            let cost = match side.get("cost_usd") {
                Value::Null => None,
                v => Some(r6(num(v).filter(|c| *c >= 0.0).ok_or_else(|| {
                    bad("cost_usd", "a number of 0 or more, or null")
                })?)),
            };
            let models = match models_of(side.get("models"))? {
                Some(m) if !m.is_empty() => m,
                _ => file_models.to_vec(),
            };
            if models.is_empty() {
                return Err(bad("models", "a list of 1 to 8 names"));
            }
            let extra = match harness {
                Some(h) => Some(Extra {
                    harness: h.clone(),
                    turns: turns_of(side.get("turns"))?,
                    seconds: seconds_of(side.get("seconds"))?,
                }),
                None => None,
            };
            records.push(Record {
                skill: name.to_string(),
                eval: eval.to_string(),
                run,
                with,
                arm: None,
                extra,
                pass,
                cost,
                tokens: tokens_of(side.get("tokens"))?,
                models,
                claude_code: claude_code.clone(),
            });
        }
    }
    Ok(Built { records, evals })
}

/// The `max-budget-usd` of an eval of the pack of this binary.
fn budget(eval: &str) -> Option<f64> {
    crate::eval_pack::all()
        .iter()
        .find(|e| e.id == eval)
        .map(|e| e.max_budget_usd)
}

/// Computes the item again from its records, and compares it with the item of
/// the file. The records come in the order of the file, and the analysis
/// groups them by eval in the order of the first appearance of the eval.
fn self_check(name: &str, item: &Value, built: &Built) -> Result<(), Fatal> {
    let mismatch = |what: &str| {
        Fatal::Config(format!(
            "eval records: the summary of `{name}` does not repeat from its runs ({what}); no record is written"
        ))
    };
    let mut by_eval: Vec<Vec<Pair>> = Vec::new();
    for eval in &built.evals {
        let mut pairs = Vec::new();
        let mut recs = built.records.iter().filter(|r| &r.eval == eval);
        while let (Some(a), Some(b)) = (recs.next(), recs.next()) {
            debug_assert!(a.with && !b.with);
            let cost = |r: &Record| {
                match r.cost {
                Some(c) => Ok(c),
                None => budget(eval).ok_or_else(|| {
                    Fatal::Config(format!(
                        "eval records: a run of `{name}` has no cost, and its eval is not in the pack of this binary; no record is written"
                    ))
                }),
            }
            };
            pairs.push(Pair {
                with_passed: a.pass,
                with_cost: cost(a)?,
                without_passed: b.pass,
                without_cost: cost(b)?,
            });
        }
        by_eval.push(pairs);
    }
    // A summary of 0.8.0 has `below_floor` and the floor of D59. A summary of
    // 0.7.x has no such key, and its verdict follows the rules of D33 only.
    let a = match item.get("below_floor") {
        Value::Bool(b) => {
            let a = verdict::analyze(&by_eval);
            if *b != a.below_floor {
                return Err(mismatch("below_floor"));
            }
            a
        }
        _ => verdict::analyze_unfloored(&by_eval),
    };
    let close = |x: f64, y: f64, tol: f64| (x - y).abs() <= tol;
    for (key, arm) in [("with", a.with), ("without", a.without)] {
        let v = item.get(key);
        if num(v.get("runs")) != Some(f64::from(arm.runs))
            || num(v.get("passes")) != Some(f64::from(arm.passes))
        {
            return Err(mismatch(if key == "with" {
                "with.runs or with.passes"
            } else {
                "without.runs or without.passes"
            }));
        }
        if !num(v.get("cost_usd")).is_some_and(|c| close(c, arm.cost, COST_TOLERANCE)) {
            return Err(mismatch(if key == "with" {
                "with.cost_usd"
            } else {
                "without.cost_usd"
            }));
        }
    }
    let d = item.get("difference");
    let diff_ok = [
        ("value", a.difference.value),
        ("low", a.difference.low),
        ("high", a.difference.high),
    ]
    .iter()
    .all(|(k, x)| num(d.get(k)).is_some_and(|y| close(y, *x, DIFFERENCE_TOLERANCE)));
    if !diff_ok {
        return Err(mismatch("difference"));
    }
    let ratio_ok = match (&a.cost_ratio, item.get("cost_ratio")) {
        (None, Value::Null) => true,
        (Some(r), v @ Value::Obj(_)) => [("value", r.value), ("low", r.low), ("high", r.high)]
            .iter()
            .all(|(k, x)| num(v.get(k)).is_some_and(|y| close(y, *x, RATIO_TOLERANCE))),
        _ => false,
    };
    if !ratio_ok {
        return Err(mismatch("cost_ratio"));
    }
    if item.get("verdict").as_str() != Some(a.verdict.name()) {
        return Err(mismatch("verdict"));
    }
    let rule = match item.get("rule") {
        Value::Null => None,
        v => num(v).map(|n| n as u8),
    };
    if rule != a.rule {
        return Err(mismatch("rule"));
    }
    Ok(())
}

/// The records of a result of `eval run --arms`: one record for each run of
/// each arm, in the order of the file (the eval, the run, then the arms in
/// the order of `arms`). A side that did not run (`null`) has no record. The
/// result has no summary to compare, so the command only checks each value
/// against its form.
fn arms_records(value: &Value) -> Result<(Vec<String>, Vec<String>), Fatal> {
    let form = "2 or 3 different names of harness arms";
    let mut arms: Vec<String> = Vec::new();
    for a in value.get("arms").as_arr() {
        let name = harness_of(a).map_err(|_| bad("arms", form))?;
        if arms.contains(&name) {
            return Err(bad("arms", form));
        }
        arms.push(name);
    }
    if !(2..=3).contains(&arms.len()) {
        return Err(bad("arms", form));
    }
    let claude_code = match value.get("claude_code") {
        Value::Null => None,
        v => Some(
            v.as_str()
                .filter(|s| valid_claude_code(s))
                .ok_or_else(|| bad("claude_code", "1 to 32 characters from A-Z a-z 0-9 . + -"))?
                .to_string(),
        ),
    };
    let file_models = models_of(value.get("models"))?.unwrap_or_default();
    let mut lines = Vec::new();
    let mut seen: BTreeSet<(String, u32)> = BTreeSet::new();
    for item in value.get("evals").as_arr() {
        let eval = item
            .get("id")
            .as_str()
            .filter(|s| valid_eval_id(s))
            .ok_or_else(|| bad("id", "1 to 40 characters, a-z first, then a-z 0-9 -"))?;
        for r in item.get("results").as_arr() {
            let run = num(r.get("run"))
                .filter(|n| n.fract() == 0.0 && (1.0..=MAX_RUN).contains(n))
                .ok_or_else(|| bad("run", "an integer from 1 to 100"))?
                as u32;
            if !seen.insert((eval.to_string(), run)) {
                return Err(bad("results", "each eval and run once"));
            }
            for arm in &arms {
                let side = r.get("arms").get(arm);
                if !matches!(side, Value::Obj(_)) {
                    continue;
                }
                let pass = match side.get("passed") {
                    Value::Bool(b) => *b,
                    _ => return Err(bad("passed", "true or false")),
                };
                let cost = match side.get("cost_usd") {
                    Value::Null => None,
                    v => Some(r6(num(v).filter(|c| *c >= 0.0).ok_or_else(|| {
                        bad("cost_usd", "a number of 0 or more, or null")
                    })?)),
                };
                let models = match models_of(side.get("models"))? {
                    Some(m) if !m.is_empty() => m,
                    _ => file_models.clone(),
                };
                if models.is_empty() {
                    return Err(bad("models", "a list of 1 to 8 names"));
                }
                lines.push(
                    Record {
                        skill: "harness".to_string(),
                        eval: eval.to_string(),
                        run,
                        with: false,
                        arm: Some(arm.clone()),
                        extra: Some(Extra {
                            harness: arm.clone(),
                            turns: turns_of(side.get("turns"))?,
                            seconds: seconds_of(side.get("seconds"))?,
                        }),
                        pass,
                        cost,
                        tokens: tokens_of(side.get("tokens"))?,
                        models,
                        claude_code: claude_code.clone(),
                    }
                    .line(),
                );
            }
        }
    }
    if lines.is_empty() {
        return Err(Fatal::Config(
            "eval records: the result of `eval run --arms` has no run".into(),
        ));
    }
    Ok((lines, vec!["harness".to_string()]))
}

/// Reads the result file, and returns the lines of the records and the names
/// of the items.
pub fn records(bytes: &[u8]) -> Result<(Vec<String>, Vec<String>), Fatal> {
    let not_result = || {
        Fatal::Config(
            "eval records: the file is no result of `eval run --ab` or `eval run --arms` (not JSON, or no list `skills` and no list `arms`)"
                .into(),
        )
    };
    let text = std::str::from_utf8(bytes).map_err(|_| not_result())?;
    let value = json::parse(text).map_err(|_| not_result())?;
    if matches!(value.get("arms"), Value::Arr(_)) {
        return arms_records(&value);
    }
    let Value::Arr(items) = value.get("skills") else {
        return Err(not_result());
    };
    if items.is_empty() {
        return Err(not_result());
    }
    let claude_code = match value.get("claude_code") {
        Value::Null => None,
        v => Some(
            v.as_str()
                .filter(|s| valid_claude_code(s))
                .ok_or_else(|| bad("claude_code", "1 to 32 characters from A-Z a-z 0-9 . + -"))?
                .to_string(),
        ),
    };
    let file_models = models_of(value.get("models"))?.unwrap_or_default();
    // A result of 0.8.0 names the harness of its copies. A result of 0.7.x
    // has no such key, and its records keep the 9 keys.
    let harness = match value.get("harness") {
        Value::Null => None,
        v => Some(harness_of(v)?),
    };
    let known = verdict::names();
    let mut lines = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for item in items {
        let name = item
            .get("name")
            .as_str()
            .filter(|n| known.contains(n))
            .ok_or_else(|| bad("name", "one of the 12 skills or `agents-line`"))?;
        if names.iter().any(|n| n == name) {
            return Err(bad("name", "each name once"));
        }
        if item
            .get("verdict")
            .as_str()
            .and_then(verdict::Verdict::from_name)
            .is_none()
        {
            return Err(Fatal::Config(format!(
                "eval records: the run of `{name}` is not complete; finish it with --resume"
            )));
        }
        let built = build_item(name, item, &file_models, &claude_code, &harness)?;
        self_check(name, item, &built)?;
        lines.extend(built.records.iter().map(Record::line));
        names.push(name.to_string());
    }
    Ok((lines, names))
}

/// `eval records <file>`: prints the records and the summary.
pub fn run(path: &str) -> Result<u8, Fatal> {
    let unreadable = |e: std::io::Error| {
        Fatal::Config(format!(
            "eval records: cannot read the result file ({})",
            e.kind()
        ))
    };
    let file = std::fs::File::open(path).map_err(unreadable)?;
    let too_big = || {
        Fatal::Config(
            "eval records: the result file is over 16 MiB; it is no result of `eval run --ab`"
                .into(),
        )
    };
    if file.metadata().map_err(unreadable)?.len() > MAX_FILE {
        return Err(too_big());
    }
    // The limit holds also for a file that grows or has no size.
    let mut bytes = Vec::new();
    std::io::Read::take(file, MAX_FILE + 1)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;
    if bytes.len() as u64 > MAX_FILE {
        return Err(too_big());
    }
    let (lines, names) = records(&bytes)?;
    let mut out = lines.join("\n");
    out.push('\n');
    print!("{out}");
    eprintln!(
        "eval records: {} records of {} names ({})",
        lines.len(),
        names.len(),
        names.join(", ")
    );
    Ok(0)
}
