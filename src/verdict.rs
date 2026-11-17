//! The verdict of a skill (ADR 0016, 0.7.0 design D32 and D33): the paired
//! bootstrap of the runs of `eval run --ab`, the rules for `on`, `off` and
//! `inconclusive`, the table of the release and the reader of the result
//! files. The module reads no file and starts no process, so `eval verdict`
//! and the check SD507 use the same functions.
//!
//! The bootstrap is repeatable. The generator is SplitMix64 with the seed 24,
//! and an index is the next value modulo the count. One resample draws, in
//! this order: the index of an eval, `k` times for `k` evals, and then for
//! each drawn eval one index for each run of that eval. A drawn run keeps its
//! pair, so both arms use the same index. The 90 percent interval is the
//! 100th and the 1,900th of the 2,000 sorted values.

use crate::harness::Skill;
use crate::json;

/// The seed of the generator.
pub const SEED: u64 = 24;
/// The number of resamples.
pub const RESAMPLES: usize = 2_000;
/// The name of the pair of arms for the line of `AGENTS.md` (D34).
pub const AGENTS_LINE: &str = "agents-line";
/// The tolerance of a comparison of floating-point values.
const EPS: f64 = 1e-9;
/// The least gain of rule 1: 0.20 of the pass rate.
const GAIN_NUM: i64 = 1;
const GAIN_DEN: i64 = 5;
/// The largest loss that rule 2 accepts: 0.05 of the pass rate.
const LOSS_DEN: i64 = 20;
/// The largest ratio of the costs per correct outcome in rule 2.
const RATIO_MAX: f64 = 0.90;

/// The generator of the bootstrap.
pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(seed: u64) -> SplitMix64 {
        SplitMix64(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// An index below `n`: the next value modulo `n`. `n` is not 0.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// The result of one pair: the same eval and run number in both arms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pair {
    pub with_passed: bool,
    /// The cost that the cap counts for the run.
    pub with_cost: f64,
    pub without_passed: bool,
    pub without_cost: f64,
}

/// A value with the ends of its 90 percent interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub value: f64,
    pub low: f64,
    pub high: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    On,
    Off,
    Inconclusive,
}

impl Verdict {
    pub fn name(self) -> &'static str {
        match self {
            Verdict::On => "on",
            Verdict::Off => "off",
            Verdict::Inconclusive => "inconclusive",
        }
    }

    pub fn from_name(s: &str) -> Option<Verdict> {
        [Verdict::On, Verdict::Off, Verdict::Inconclusive]
            .into_iter()
            .find(|v| v.name() == s)
    }
}

/// The counts of one arm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arm {
    pub runs: u32,
    pub passes: u32,
    pub cost: f64,
}

impl Arm {
    pub fn pass_rate(&self) -> f64 {
        if self.runs == 0 {
            0.0
        } else {
            f64::from(self.passes) / f64::from(self.runs)
        }
    }

    /// The cost of the arm divided by its passes, or `None` with no pass.
    pub fn cost_per_correct(&self) -> Option<f64> {
        (self.passes > 0).then(|| self.cost / f64::from(self.passes))
    }
}

/// The analysis of the pairs of one skill.
#[derive(Clone, Debug, PartialEq)]
pub struct Analysis {
    pub evals: usize,
    pub with: Arm,
    pub without: Arm,
    pub difference: Interval,
    /// `None` when the ratio is undefined (a resample or the data has no
    /// pass in an arm).
    pub cost_ratio: Option<Interval>,
    pub verdict: Verdict,
    /// The rule that gives `on`: 1 or 2.
    pub rule: Option<u8>,
}

fn arm_of(pairs: &[&Pair], with: bool) -> Arm {
    Arm {
        runs: pairs.len() as u32,
        passes: pairs
            .iter()
            .filter(|p| {
                if with {
                    p.with_passed
                } else {
                    p.without_passed
                }
            })
            .count() as u32,
        cost: pairs
            .iter()
            .map(|p| if with { p.with_cost } else { p.without_cost })
            .sum(),
    }
}

/// The ends of the 90 percent interval of the sorted values.
fn ends(values: &mut [f64]) -> (f64, f64) {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    (values[n * 5 / 100 - 1], values[n * 95 / 100 - 1])
}

/// The ratio of the costs per correct outcome of the sums, or `None` when an
/// arm has no pass.
fn ratio(wp: u32, wc: f64, op: u32, oc: f64) -> Option<f64> {
    if wp == 0 || op == 0 {
        return None;
    }
    Some((wc / f64::from(wp)) / (oc / f64::from(op)))
}

/// Computes the analysis. `evals` holds the pairs of each eval. An eval with
/// no pair is dropped. The result is the same for the same input.
pub fn analyze(evals: &[Vec<Pair>]) -> Analysis {
    let evals: Vec<&Vec<Pair>> = evals.iter().filter(|e| !e.is_empty()).collect();
    let all: Vec<&Pair> = evals.iter().flat_map(|e| e.iter()).collect();
    let with = arm_of(&all, true);
    let without = arm_of(&all, false);
    let difference_value = if all.is_empty() {
        0.0
    } else {
        (i64::from(with.passes) - i64::from(without.passes)) as f64 / all.len() as f64
    };
    let point_ratio = ratio(with.passes, with.cost, without.passes, without.cost);

    let mut diffs = Vec::with_capacity(RESAMPLES);
    let mut ratios = Vec::with_capacity(RESAMPLES);
    let mut ratio_defined = true;
    let mut rng = SplitMix64::new(SEED);
    if !evals.is_empty() {
        for _ in 0..RESAMPLES {
            let (mut wp, mut op, mut n) = (0u32, 0u32, 0u32);
            let (mut wc, mut oc) = (0.0, 0.0);
            let drawn: Vec<usize> = (0..evals.len()).map(|_| rng.below(evals.len())).collect();
            for e in drawn {
                let runs = evals[e];
                for _ in 0..runs.len() {
                    let p = &runs[rng.below(runs.len())];
                    n += 1;
                    wp += u32::from(p.with_passed);
                    op += u32::from(p.without_passed);
                    wc += p.with_cost;
                    oc += p.without_cost;
                }
            }
            diffs.push((i64::from(wp) - i64::from(op)) as f64 / f64::from(n));
            match ratio(wp, wc, op, oc) {
                Some(r) => ratios.push(r),
                None => ratio_defined = false,
            }
        }
    }
    let (dl, dh) = if diffs.is_empty() {
        (0.0, 0.0)
    } else {
        ends(&mut diffs)
    };
    let difference = Interval {
        value: difference_value,
        low: dl,
        high: dh,
    };
    let cost_ratio = match point_ratio {
        Some(value) if ratio_defined && !ratios.is_empty() => {
            let (low, high) = ends(&mut ratios);
            Some(Interval { value, low, high })
        }
        _ => None,
    };
    let (verdict, rule) = decide(
        i64::from(with.passes) - i64::from(without.passes),
        all.len() as i64,
        &difference,
        cost_ratio.as_ref(),
    );
    Analysis {
        evals: evals.len(),
        with,
        without,
        difference,
        cost_ratio,
        verdict,
        rule,
    }
}

/// The rules of D33. `gain` is the passes with the skill minus the passes
/// without it, and `runs` is the number of pairs. The comparisons of the pass
/// rates use these integers, so 16 passes against 12 of 20 runs is a gain of
/// exactly 0.20.
///
/// 1. The gain is at least 0.20, and the low end of the interval is above 0.
/// 2. The loss is at most 0.05, and the ratio of the costs per correct outcome
///    is at most 0.90, and its high end is below 1.
///
/// The verdict is `off` when the high end of the interval of the difference is
/// below 0. Else it is `inconclusive`.
pub fn decide(
    gain: i64,
    runs: i64,
    difference: &Interval,
    cost_ratio: Option<&Interval>,
) -> (Verdict, Option<u8>) {
    if runs > 0 && gain * GAIN_DEN >= runs * GAIN_NUM && difference.low > 0.0 {
        return (Verdict::On, Some(1));
    }
    if runs > 0
        && gain * LOSS_DEN >= -runs
        && cost_ratio.is_some_and(|r| r.value <= RATIO_MAX + EPS && r.high < 1.0)
    {
        return (Verdict::On, Some(2));
    }
    if difference.high < 0.0 {
        return (Verdict::Off, None);
    }
    (Verdict::Inconclusive, None)
}

/// One row of the table of the release: the analysis of the newest batch of
/// the committed records (`bench/results/evals/`) that holds the name. A test
/// computes the table from the records and compares it with `RELEASE`, so the
/// constant and the records agree. Costs and intervals have 6 decimals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    pub name: &'static str,
    /// The stem of the file of the batch.
    pub batch: &'static str,
    pub claude_code: Option<&'static str>,
    /// The sorted union of the models of the records.
    pub models: &'static [&'static str],
    pub with: Arm,
    pub without: Arm,
    pub difference: Interval,
    pub cost_ratio: Option<Interval>,
    pub verdict: Verdict,
    pub rule: Option<u8>,
}

/// The table of the release. A name that has no record has no row, and
/// `eval verdict` prints it as `unmeasured`.
pub const RELEASE: &[Row] = &[
    Row {
        name: "execute",
        batch: "20261117T082858Z-execute",
        claude_code: Some("2.1.326"),
        models: &["claude-fable-5-2", "claude-opus-5-5", "claude-sonnet-5-5"],
        with: Arm {
            runs: 20,
            passes: 16,
            cost: 24.34259,
        },
        without: Arm {
            runs: 20,
            passes: 15,
            cost: 26.609628,
        },
        difference: Interval {
            value: 0.05,
            low: -0.25,
            high: 0.3,
        },
        cost_ratio: Some(Interval {
            value: 0.857629,
            low: 0.580575,
            high: 1.177308,
        }),
        verdict: Verdict::Inconclusive,
        rule: None,
    },
    Row {
        name: "tdd",
        batch: "20261117T082902Z-tdd",
        claude_code: Some("2.1.326"),
        models: &["claude-fable-5-2", "claude-opus-5-5"],
        with: Arm {
            runs: 20,
            passes: 10,
            cost: 19.937457,
        },
        without: Arm {
            runs: 20,
            passes: 1,
            cost: 19.680766,
        },
        difference: Interval {
            value: 0.45,
            low: 0.15,
            high: 0.75,
        },
        cost_ratio: None,
        verdict: Verdict::On,
        rule: Some(1),
    },
    Row {
        name: "verify",
        batch: "20261117T082906Z-verify",
        claude_code: Some("2.1.326"),
        models: &["claude-fable-5-2", "claude-opus-5-5", "claude-sonnet-5-5"],
        with: Arm {
            runs: 20,
            passes: 20,
            cost: 16.238337,
        },
        without: Arm {
            runs: 20,
            passes: 20,
            cost: 14.753787,
        },
        difference: Interval {
            value: 0.0,
            low: 0.0,
            high: 0.0,
        },
        cost_ratio: Some(Interval {
            value: 1.100622,
            low: 0.92935,
            high: 1.314506,
        }),
        verdict: Verdict::Inconclusive,
        rule: None,
    },
    Row {
        name: "debug",
        batch: "20261117T082910Z-debug",
        claude_code: Some("2.1.326"),
        models: &["claude-fable-5-2", "claude-opus-5-5", "claude-sonnet-5-5"],
        with: Arm {
            runs: 20,
            passes: 15,
            cost: 14.636573,
        },
        without: Arm {
            runs: 20,
            passes: 13,
            cost: 12.482311,
        },
        difference: Interval {
            value: 0.1,
            low: -0.1,
            high: 0.3,
        },
        cost_ratio: None,
        verdict: Verdict::Inconclusive,
        rule: None,
    },
    Row {
        name: "agents-line",
        batch: "20261117T082914Z-agents-line",
        claude_code: Some("2.1.326"),
        models: &["claude-fable-5-2", "claude-opus-5-5", "claude-sonnet-5-5"],
        with: Arm {
            runs: 20,
            passes: 13,
            cost: 21.098643,
        },
        without: Arm {
            runs: 20,
            passes: 13,
            cost: 22.61437,
        },
        difference: Interval {
            value: 0.0,
            low: -0.45,
            high: 0.4,
        },
        cost_ratio: None,
        verdict: Verdict::Inconclusive,
        rule: None,
    },
];

/// The verdict of the release for a name, if the table holds one.
pub fn release(name: &str) -> Option<Verdict> {
    RELEASE.iter().find(|r| r.name == name).map(|r| r.verdict)
}

/// ADR 0023. `true`: a skill is on by default only with the verdict `on`.
/// `false`: D33 (a skill of D2 is on unless `off`; another skill only with
/// `on`). The owner sets the value after the full eval run. A test names no
/// default skill, so this line is the one edit of the change.
pub const DEFAULT_ON_NEEDS_VERDICT_ON: bool = true;

/// The rule of ADR 0023. It reads no file and no table.
pub fn on_by_default(in_d2: bool, verdict: Option<Verdict>, needs_on: bool) -> bool {
    match (needs_on, in_d2, verdict) {
        (_, _, Some(Verdict::On)) => true,
        (true, _, _) | (false, false, _) => false,
        (false, true, v) => v != Some(Verdict::Off),
    }
}

/// The default of a name in this release: the rule, the table of the release
/// (through `release(name)`, never the constant `RELEASE` itself) and the
/// constant.
pub fn default_on(name: &str, in_d2: bool) -> bool {
    on_by_default(in_d2, release(name), DEFAULT_ON_NEEDS_VERDICT_ON)
}

fn r6(x: f64) -> f64 {
    crate::eval_run::r6(x)
}

/// The JSON of an interval, with 6 decimals.
pub fn interval_json(i: &Interval) -> String {
    json::Obj::new()
        .num("value", r6(i.value))
        .num("low", r6(i.low))
        .num("high", r6(i.high))
        .end()
}

/// The JSON of the counts of an arm. An arm with no pass has the cost per
/// correct outcome `null`.
pub fn arm_json(a: Option<&Arm>) -> String {
    let zero = Arm {
        runs: 0,
        passes: 0,
        cost: 0.0,
    };
    let a = a.unwrap_or(&zero);
    let obj = json::Obj::new()
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

/// One line of `eval verdict --release`, for the report and the release notes.
pub fn release_line(r: &Row) -> String {
    let verdict = match r.rule {
        Some(rule) => format!("{} (rule {rule})", r.verdict.name()),
        None => r.verdict.name().to_string(),
    };
    let per = |a: &Arm| {
        a.cost_per_correct()
            .map_or("-".to_string(), |c| format!("{:.6}", r6(c)))
    };
    let ratio = r.cost_ratio.map_or("-".to_string(), |c| {
        format!("{:.2} [{:.2}, {:.2}]", c.value, c.low, c.high)
    });
    format!(
        "{} {verdict}: with {}/{} {:.2}, without {}/{} {:.2}, cost per correct {} and {} USD, difference {:.2} [{:.2}, {:.2}], cost ratio {ratio}, batch {}, claude {}",
        r.name,
        r.with.passes,
        r.with.runs,
        r.with.pass_rate(),
        r.without.passes,
        r.without.runs,
        r.without.pass_rate(),
        per(&r.with),
        per(&r.without),
        r.difference.value,
        r.difference.low,
        r.difference.high,
        r.batch,
        r.claude_code.unwrap_or("-"),
    )
}

/// The JSON of `eval verdict --release`.
pub fn release_json(rows: &[Row]) -> String {
    let items = rows.iter().map(|r| {
        let models: Vec<String> = r.models.iter().map(|m| (*m).to_string()).collect();
        let obj = json::Obj::new()
            .str("name", r.name)
            .str("verdict", r.verdict.name());
        let obj = match r.rule {
            Some(n) => obj.num("rule", n),
            None => obj.raw("rule", "null"),
        };
        obj.str("batch", r.batch)
            .opt_str("claude_code", r.claude_code)
            .strs("models", &models)
            .raw("with", &arm_json(Some(&r.with)))
            .raw("without", &arm_json(Some(&r.without)))
            .raw("difference", &interval_json(&r.difference))
            .raw(
                "cost_ratio",
                &r.cost_ratio
                    .as_ref()
                    .map_or("null".to_string(), interval_json),
            )
            .end()
    });
    json::Obj::new().raw("release", &json::array(items)).end()
}

/// The 11 names of the skills and `agents-line`, the names that a result and
/// `eval verdict` use.
pub fn names() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = Skill::ALL.iter().map(|s| s.key()).collect();
    v.push(AGENTS_LINE);
    v
}

/// True for the file name of a result of `--ab`: `skills-<stamp>.json`.
pub fn is_result_name(base: &str) -> bool {
    base.strip_prefix("skills-")
        .and_then(|r| r.strip_suffix(".json"))
        .is_some_and(|stamp| !stamp.is_empty() && !stamp.contains('/'))
}

/// The verdicts that a result file holds: the name and the verdict of each
/// item of `skills` that has a verdict. A file that is not JSON, or that has
/// no list `skills`, holds none.
pub fn read_result(bytes: &[u8]) -> Vec<(String, Verdict)> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Vec::new();
    };
    let Ok(value) = json::parse(text) else {
        return Vec::new();
    };
    value
        .get("skills")
        .as_arr()
        .iter()
        .filter_map(|item| {
            let name = item.get("name").as_str()?;
            let verdict = Verdict::from_name(item.get("verdict").as_str()?)?;
            Some((name.to_string(), verdict))
        })
        .collect()
}

/// Where a verdict comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// A result file of the project, with its path.
    Project(String),
    Release,
    None,
}

/// A verdict with its source. `verdict` is `None` for `unmeasured`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub verdict: Option<Verdict>,
    pub source: Source,
}

/// The verdict of a name: the newest result file that holds a verdict of the
/// name, and else the table of the release. `files` holds the path and the
/// bytes of each file of the folder of the results. A file counts when its
/// name is `skills-<stamp>.json`. The newest is the last name in order, as the
/// stamp is a UTC time.
pub fn resolve(name: &str, files: &[(String, Vec<u8>)]) -> Resolved {
    let mut candidates: Vec<&(String, Vec<u8>)> = files
        .iter()
        .filter(|(p, _)| is_result_name(p.rsplit('/').next().unwrap_or(p)))
        .collect();
    candidates
        .sort_by_key(|(p, _)| std::cmp::Reverse(result_order(p.rsplit('/').next().unwrap_or(p))));
    for (path, bytes) in candidates {
        if let Some((_, v)) = read_result(bytes).into_iter().find(|(n, _)| n == name) {
            return Resolved {
                verdict: Some(v),
                source: Source::Project(path.clone()),
            };
        }
    }
    match release(name) {
        Some(v) => Resolved {
            verdict: Some(v),
            source: Source::Release,
        },
        None => Resolved {
            verdict: None,
            source: Source::None,
        },
    }
}

/// The order of a result file: the time of its stamp, then the number of its
/// suffix (`-2` is the second file of the same second), then its name. A
/// plain sort of names is wrong, because `-` sorts before `.`, so
/// `skills-<stamp>-2.json` would come before `skills-<stamp>.json`.
fn result_order(base: &str) -> (String, u32, String) {
    let rest = base
        .strip_prefix("skills-")
        .and_then(|r| r.strip_suffix(".json"))
        .unwrap_or("");
    match rest.split_once('-') {
        Some((stamp, n)) => (stamp.to_string(), n.parse().unwrap_or(0), base.to_string()),
        None => (rest.to_string(), 1, base.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(w: bool, o: bool) -> Pair {
        Pair {
            with_passed: w,
            with_cost: 1.0,
            without_passed: o,
            without_cost: 1.0,
        }
    }

    /// 5 evals of 4 runs. `with` and `without` list the passes of each eval.
    fn data(with: [u32; 5], without: [u32; 5]) -> Vec<Vec<Pair>> {
        (0..5)
            .map(|e| {
                (0..4)
                    .map(|r| pair(r < with[e], r < without[e]))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    #[test]
    fn splitmix64_gives_the_published_values_of_the_seed_0() {
        let mut g = SplitMix64::new(0);
        assert_eq!(g.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(g.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(g.next_u64(), 0x06C4_5D18_8009_454F);
    }

    #[test]
    fn an_index_is_the_next_value_modulo_the_count() {
        let mut a = SplitMix64::new(SEED);
        let mut b = SplitMix64::new(SEED);
        for n in [5usize, 4, 1, 20] {
            assert_eq!(a.below(n) as u64, b.next_u64() % n as u64);
        }
    }

    #[test]
    fn the_same_data_gives_the_same_analysis() {
        let d = data([4, 4, 4, 4, 4], [0, 0, 0, 0, 0]);
        assert_eq!(analyze(&d), analyze(&d));
    }

    #[test]
    fn a_gain_of_everything_is_on_by_rule_1() {
        let a = analyze(&data([4, 4, 4, 4, 4], [0, 0, 0, 0, 0]));
        assert_eq!((a.with.passes, a.without.passes), (20, 0));
        assert_eq!(a.difference.value, 1.0);
        assert_eq!((a.difference.low, a.difference.high), (1.0, 1.0));
        // No pass without the skill: the ratio is undefined.
        assert_eq!(a.cost_ratio, None);
        assert_eq!((a.verdict, a.rule), (Verdict::On, Some(1)));
    }

    #[test]
    fn a_loss_of_everything_is_off() {
        let a = analyze(&data([0, 0, 0, 0, 0], [4, 4, 4, 4, 4]));
        assert_eq!((a.verdict, a.rule), (Verdict::Off, None));
        assert_eq!(a.difference.value, -1.0);
    }

    #[test]
    fn no_difference_and_the_same_cost_is_inconclusive() {
        let a = analyze(&data([2, 2, 2, 2, 2], [2, 2, 2, 2, 2]));
        assert_eq!(a.difference.value, 0.0);
        let r = a.cost_ratio.unwrap();
        assert_eq!((r.value, r.low, r.high), (1.0, 1.0, 1.0));
        assert_eq!((a.verdict, a.rule), (Verdict::Inconclusive, None));
    }

    #[test]
    fn the_same_passes_at_half_the_cost_is_on_by_rule_2() {
        let mut d = data([2, 2, 2, 2, 2], [2, 2, 2, 2, 2]);
        for e in &mut d {
            for p in e {
                p.with_cost = 0.5;
            }
        }
        let a = analyze(&d);
        let r = a.cost_ratio.unwrap();
        assert_eq!((r.value, r.high), (0.5, 0.5));
        assert_eq!((a.verdict, a.rule), (Verdict::On, Some(2)));
    }

    #[test]
    fn a_gain_of_0_20_with_a_wide_interval_is_inconclusive() {
        // 16 passes against 12 of 20 runs: a gain of exactly 0.20, but the
        // evals differ, so the interval includes 0.
        let a = analyze(&data([4, 4, 4, 4, 0], [4, 2, 2, 2, 2]));
        assert_eq!((a.with.passes, a.without.passes), (16, 12));
        assert_eq!(a.difference.value, 0.2);
        assert!(a.difference.low <= 0.0, "{:?}", a.difference);
        assert_eq!((a.verdict, a.rule), (Verdict::Inconclusive, None));
    }

    #[test]
    fn the_rules_compare_counts_not_floating_point_rates() {
        let wide = Interval {
            value: 0.2,
            low: 0.05,
            high: 0.35,
        };
        // 12 against 8 of 20: 0.6 - 0.4 is below 0.2 in floating point.
        assert!(std::hint::black_box(0.6_f64) - 0.4 < 0.2);
        assert_eq!(decide(4, 20, &wide, None), (Verdict::On, Some(1)));
        assert_eq!(decide(3, 20, &wide, None), (Verdict::Inconclusive, None));
        // A gain of 0.20 with the low end at 0 is not enough.
        let zero = Interval { low: 0.0, ..wide };
        assert_eq!(decide(4, 20, &zero, None), (Verdict::Inconclusive, None));
        // Rule 2 accepts a loss of 0.05 (1 of 20), and not more.
        let cheap = Interval {
            value: 0.9,
            low: 0.5,
            high: 0.99,
        };
        let flat = Interval {
            value: -0.05,
            low: -0.2,
            high: 0.1,
        };
        assert_eq!(decide(-1, 20, &flat, Some(&cheap)), (Verdict::On, Some(2)));
        assert_eq!(
            decide(-2, 20, &flat, Some(&cheap)),
            (Verdict::Inconclusive, None)
        );
        // The ratio must be at most 0.90, and its high end below 1.
        let dear = Interval {
            value: 0.91,
            ..cheap
        };
        assert_eq!(
            decide(0, 20, &flat, Some(&dear)),
            (Verdict::Inconclusive, None)
        );
        let open = Interval { high: 1.0, ..cheap };
        assert_eq!(
            decide(0, 20, &flat, Some(&open)),
            (Verdict::Inconclusive, None)
        );
        // Rule 1 comes first when both hold.
        assert_eq!(decide(4, 20, &wide, Some(&cheap)), (Verdict::On, Some(1)));
        // `off` needs the high end below 0.
        let down = Interval {
            value: -0.3,
            low: -0.5,
            high: -0.01,
        };
        assert_eq!(decide(-6, 20, &down, None), (Verdict::Off, None));
    }

    #[test]
    fn an_interval_that_holds_an_undefined_ratio_has_no_ratio() {
        // 1 pass without the skill in 20 runs: some resample has no pass.
        let mut d = data([4, 4, 4, 4, 4], [0, 0, 0, 0, 0]);
        d[0][0].without_passed = true;
        let a = analyze(&d);
        assert!(a.cost_ratio.is_none());
        assert_eq!(a.verdict, Verdict::On);
    }

    #[test]
    fn the_interval_of_a_fixed_input_does_not_drift() {
        // A golden value: a change of the generator, of the order of the draws
        // or of the interval changes it, and then the committed results of the
        // runs of earlier versions no longer repeat.
        let a = analyze(&data([4, 3, 4, 2, 4], [2, 3, 1, 2, 2]));
        assert_eq!((a.with.passes, a.without.passes), (17, 10));
        assert_eq!(a.difference.value, 0.35);
        assert_eq!(
            (a.difference.low, a.difference.high),
            GOLDEN_DIFFERENCE,
            "{:?}",
            a.difference
        );
        assert_eq!((a.verdict, a.rule), (Verdict::On, Some(1)));
    }

    const GOLDEN_DIFFERENCE: (f64, f64) = (0.1, 0.6);

    #[test]
    fn a_result_file_gives_the_names_with_a_verdict() {
        let text = br#"{"skills":[{"name":"tdd","verdict":"on"},{"name":"debug","verdict":null},{"name":"verify","verdict":"maybe"},{"name":"execute","verdict":"inconclusive"},{"verdict":"on"}]}"#;
        assert_eq!(
            read_result(text),
            vec![
                ("tdd".to_string(), Verdict::On),
                ("execute".to_string(), Verdict::Inconclusive)
            ]
        );
        assert!(read_result(b"not json").is_empty());
        assert!(read_result(b"[]").is_empty());
        assert!(read_result(&[0xff, 0xfe]).is_empty());
    }

    #[test]
    fn the_newest_file_that_holds_the_name_decides() {
        let f = |p: &str, t: &str| (p.to_string(), t.as_bytes().to_vec());
        let files = vec![
            f(
                "evals/results/skills-20261120T100000Z.json",
                r#"{"skills":[{"name":"tdd","verdict":"on"},{"name":"debug","verdict":"on"}]}"#,
            ),
            f(
                "evals/results/skills-20261121T100000Z.json",
                r#"{"skills":[{"name":"tdd","verdict":"inconclusive"}]}"#,
            ),
            f(
                "evals/results/skills-20261122T100000Z.json",
                r#"{"skills":[{"name":"tdd","verdict":null}]}"#,
            ),
            f(
                "evals/results/eval-20261123T100000Z.json",
                r#"{"skills":[{"name":"tdd","verdict":"off"}]}"#,
            ),
        ];
        let r = resolve("tdd", &files);
        assert_eq!(r.verdict, Some(Verdict::Inconclusive));
        assert_eq!(
            r.source,
            Source::Project("evals/results/skills-20261121T100000Z.json".into())
        );
        assert_eq!(resolve("debug", &files).verdict, Some(Verdict::On));
        let none = resolve("brainstorm", &files);
        assert_eq!((none.verdict, none.source), (None, Source::None));
    }

    #[test]
    fn a_second_file_of_the_same_second_is_newer_and_the_suffix_sorts_by_number() {
        let f = |p: &str, v: &str| {
            (
                p.to_string(),
                format!(r#"{{"skills":[{{"name":"tdd","verdict":"{v}"}}]}}"#).into_bytes(),
            )
        };
        let dir = "evals/results/";
        let files = vec![
            f(&format!("{dir}skills-20261120T100000Z-10.json"), "off"),
            f(&format!("{dir}skills-20261120T100000Z.json"), "on"),
            f(
                &format!("{dir}skills-20261120T100000Z-9.json"),
                "inconclusive",
            ),
            f(&format!("{dir}skills-20261119T235959Z-99.json"), "on"),
        ];
        assert_eq!(resolve("tdd", &files).verdict, Some(Verdict::Off));
        let files = &files[1..3];
        assert_eq!(resolve("tdd", files).verdict, Some(Verdict::Inconclusive));
        assert_eq!(resolve("tdd", &files[..1]).verdict, Some(Verdict::On));
    }

    #[test]
    fn the_names_are_the_11_skills_and_agents_line() {
        assert_eq!(names().len(), 12);
        assert_eq!(names()[11], "agents-line");
    }

    // The committed records (`bench/results/evals/*.jsonl`).

    const RECORD_KEYS: [&str; 9] = [
        "skill",
        "eval",
        "run",
        "arm",
        "pass",
        "cost",
        "tokens",
        "models",
        "claude_code",
    ];

    struct Rec {
        skill: String,
        eval: String,
        run: u32,
        with: bool,
        pass: bool,
        cost: Option<f64>,
        models: Vec<String>,
        claude_code: Option<String>,
    }

    fn records_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/results/evals")
    }

    /// The file stems of the records in the order of a batch: the stamp, then
    /// the number of the suffix (`<stamp>-2-<target>` is the second file of
    /// the second), then the stem.
    fn batch_order(stem: &str) -> (String, u32, String) {
        let (stamp, rest) = stem.split_at(stem.find('Z').map_or(0, |i| i + 1));
        let n = rest
            .strip_prefix('-')
            .and_then(|r| r.split_once('-'))
            .and_then(|(n, _)| n.parse::<u32>().ok())
            .unwrap_or(1);
        (stamp.to_string(), n, stem.to_string())
    }

    /// Reads one record. It fails on a key that is missing, in another order,
    /// or outside its form.
    fn read_record(line: &str) -> Result<Rec, String> {
        let v = json::parse(line)?;
        let json::Value::Obj(fields) = &v else {
            return Err("no object".into());
        };
        let keys: Vec<&str> = fields.iter().map(|(k, _)| k.as_str()).collect();
        if keys != RECORD_KEYS {
            return Err(format!("keys {keys:?}"));
        }
        let text = |k: &str| v.get(k).as_str().map(str::to_string);
        let skill = text("skill")
            .filter(|s| names().contains(&s.as_str()))
            .ok_or("skill")?;
        let eval = text("eval")
            .filter(|s| crate::eval_records::valid_eval_id(s))
            .ok_or("eval")?;
        let run = match v.get("run") {
            json::Value::Num(n) if n.fract() == 0.0 && (1.0..=100.0).contains(n) => *n as u32,
            _ => return Err("run".into()),
        };
        let with = match v.get("arm").as_str() {
            Some("with") => true,
            Some("without") => false,
            _ => return Err("arm".into()),
        };
        let pass = match v.get("pass") {
            json::Value::Bool(b) => *b,
            _ => return Err("pass".into()),
        };
        let cost = match v.get("cost") {
            json::Value::Null => None,
            json::Value::Num(n) if *n >= 0.0 && (n * 1e6 - (n * 1e6).round()).abs() < 1e-6 => {
                Some(*n)
            }
            _ => return Err("cost".into()),
        };
        match v.get("tokens") {
            json::Value::Null => {}
            t @ json::Value::Obj(f) => {
                let k: Vec<&str> = f.iter().map(|(k, _)| k.as_str()).collect();
                if k != ["in", "out", "cache_read", "cache_write"]
                    || !k.iter().all(|k| {
                        matches!(t.get(k), json::Value::Num(n) if *n >= 0.0 && n.fract() == 0.0)
                    })
                {
                    return Err("tokens".into());
                }
            }
            _ => return Err("tokens".into()),
        }
        let models: Vec<String> = v
            .get("models")
            .as_arr()
            .iter()
            .map(|m| {
                m.as_str()
                    .filter(|m| crate::eval_records::valid_model(m))
                    .map(str::to_string)
            })
            .collect::<Option<_>>()
            .ok_or("models")?;
        let mut sorted = models.clone();
        sorted.sort();
        sorted.dedup();
        if models.is_empty() || models.len() > 8 || sorted != models {
            return Err("models".into());
        }
        let claude_code = match v.get("claude_code") {
            json::Value::Null => None,
            c => Some(
                c.as_str()
                    .filter(|c| crate::eval_records::valid_claude_code(c))
                    .ok_or("claude_code")?
                    .to_string(),
            ),
        };
        Ok(Rec {
            skill,
            eval,
            run,
            with,
            pass,
            cost,
            models,
            claude_code,
        })
    }

    /// Each file of the records: its stem and its records, in the order of
    /// the batches.
    fn load() -> Vec<(String, Vec<Rec>)> {
        let mut files: Vec<(String, Vec<Rec>)> = std::fs::read_dir(records_dir())
            .expect("bench/results/evals exists")
            .filter_map(Result::ok)
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let stem = name.strip_suffix(".jsonl")?.to_string();
                let text = std::fs::read_to_string(e.path()).unwrap();
                assert!(
                    text.ends_with('\n') && !text.contains("\n\n"),
                    "{name}: lines"
                );
                assert!(!text.contains('\r'), "{name}: CR");
                let recs = text
                    .lines()
                    .enumerate()
                    .map(|(i, l)| {
                        read_record(l).unwrap_or_else(|e| panic!("{name}:{}: {e}", i + 1))
                    })
                    .collect();
                Some((stem, recs))
            })
            .collect();
        files.sort_by_key(|(stem, _)| batch_order(stem));
        files
    }

    /// The `max-budget-usd` of an eval of the pack, for a record with no cost.
    fn budget(eval: &str) -> f64 {
        crate::eval_pack::all()
            .iter()
            .find(|e| e.id == eval)
            .unwrap_or_else(|| panic!("a record with no cost names `{eval}`, not in the pack"))
            .max_budget_usd
    }

    /// The analysis of the records of one name in one batch, with the sorted
    /// union of the models. The records keep the order of the file, and the
    /// evals the order of their first appearance.
    fn analysis_of(recs: &[Rec], name: &str) -> (Analysis, Vec<String>, Option<String>) {
        let mine: Vec<&Rec> = recs.iter().filter(|r| r.skill == name).collect();
        let mut evals: Vec<&str> = Vec::new();
        for r in &mine {
            if !evals.contains(&r.eval.as_str()) {
                evals.push(&r.eval);
            }
        }
        let by_eval: Vec<Vec<Pair>> = evals
            .iter()
            .map(|e| {
                let runs: Vec<&&Rec> = mine.iter().filter(|r| r.eval == *e).collect();
                runs.chunks(2)
                    .map(|c| {
                        assert!(c[0].with && !c[1].with, "{e}: arms");
                        assert_eq!(c[0].run, c[1].run, "{e}: the arms of a pair");
                        Pair {
                            with_passed: c[0].pass,
                            with_cost: c[0].cost.unwrap_or_else(|| budget(e)),
                            without_passed: c[1].pass,
                            without_cost: c[1].cost.unwrap_or_else(|| budget(e)),
                        }
                    })
                    .collect()
            })
            .collect();
        let mut models: Vec<String> = mine.iter().flat_map(|r| r.models.clone()).collect();
        models.sort();
        models.dedup();
        (
            analyze(&by_eval),
            models,
            mine.first().and_then(|r| r.claude_code.clone()),
        )
    }

    fn rounded(a: &Arm) -> Arm {
        Arm {
            runs: a.runs,
            passes: a.passes,
            cost: r6(a.cost),
        }
    }

    fn rounded_interval(i: &Interval) -> Interval {
        Interval {
            value: r6(i.value),
            low: r6(i.low),
            high: r6(i.high),
        }
    }

    /// The rows that the records give: for each name, the newest batch that
    /// has records of the name decides.
    fn computed() -> Vec<(String, Row2)> {
        let files = load();
        let mut rows = Vec::new();
        for name in names() {
            let Some((stem, recs)) = files
                .iter()
                .rev()
                .find(|(_, recs)| recs.iter().any(|r| r.skill == name))
            else {
                continue;
            };
            let (a, models, claude_code) = analysis_of(recs, name);
            rows.push((
                name.to_string(),
                Row2 {
                    batch: stem.clone(),
                    claude_code,
                    models,
                    with: rounded(&a.with),
                    without: rounded(&a.without),
                    difference: rounded_interval(&a.difference),
                    cost_ratio: a.cost_ratio.as_ref().map(rounded_interval),
                    verdict: a.verdict,
                    rule: a.rule,
                },
            ));
        }
        rows
    }

    /// A row with owned text.
    #[derive(Debug)]
    struct Row2 {
        batch: String,
        claude_code: Option<String>,
        models: Vec<String>,
        with: Arm,
        without: Arm,
        difference: Interval,
        cost_ratio: Option<Interval>,
        verdict: Verdict,
        rule: Option<u8>,
    }

    fn rust_text(rows: &[(String, Row2)]) -> String {
        let iv = |i: &Interval| {
            format!(
                "Interval {{ value: {:?}, low: {:?}, high: {:?} }}",
                i.value, i.low, i.high
            )
        };
        let arm = |a: &Arm| {
            format!(
                "Arm {{ runs: {}, passes: {}, cost: {:?} }}",
                a.runs, a.passes, a.cost
            )
        };
        let mut out = String::from("pub const RELEASE: &[Row] = &[\n");
        for (name, r) in rows {
            let models: Vec<String> = r.models.iter().map(|m| format!("{m:?}")).collect();
            let v = match r.verdict {
                Verdict::On => "On",
                Verdict::Off => "Off",
                Verdict::Inconclusive => "Inconclusive",
            };
            out += &format!(
                "    Row {{\n        name: {name:?},\n        batch: {:?},\n        claude_code: {},\n        models: &[{}],\n        with: {},\n        without: {},\n        difference: {},\n        cost_ratio: {},\n        verdict: Verdict::{v},\n        rule: {:?},\n    }},\n",
                r.batch,
                r.claude_code
                    .as_ref()
                    .map_or("None".to_string(), |c| format!("Some({c:?})")),
                models.join(", "),
                arm(&r.with),
                arm(&r.without),
                iv(&r.difference),
                r.cost_ratio
                    .as_ref()
                    .map_or("None".to_string(), |c| format!("Some({})", iv(c))),
                r.rule,
            );
        }
        out + "];\n"
    }

    #[test]
    fn the_release_table_agrees_with_the_committed_records() {
        let rows = computed();
        let same = rows.len() == RELEASE.len()
            && rows.iter().zip(RELEASE).all(|((name, r), c)| {
                *name == c.name
                    && r.batch == c.batch
                    && r.claude_code.as_deref() == c.claude_code
                    && r.models
                        .iter()
                        .map(String::as_str)
                        .eq(c.models.iter().copied())
                    && r.with == c.with
                    && r.without == c.without
                    && r.difference == c.difference
                    && r.cost_ratio == c.cost_ratio
                    && r.verdict == c.verdict
                    && r.rule == c.rule
            });
        assert!(
            same,
            "RELEASE does not agree with bench/results/evals. Paste this text:\n{}",
            rust_text(&rows)
        );
    }

    #[test]
    fn each_committed_record_has_the_9_keys_and_safe_values() {
        let files = load();
        assert!(!files.is_empty(), "no committed record");
        for (stem, recs) in &files {
            let at = |i: usize| format!("{stem}:{}", i + 1);
            // The pairs: the arm `with` comes first, and then its arm `without`.
            assert_eq!(recs.len() % 2, 0, "{stem}");
            for (i, c) in recs.chunks(2).enumerate() {
                assert!(c[0].with && !c[1].with, "{}: arms", at(i * 2));
                assert_eq!(
                    (&c[0].skill, &c[0].eval, c[0].run),
                    (&c[1].skill, &c[1].eval, c[1].run),
                    "{}: the 2 arms of a pair",
                    at(i * 2)
                );
            }
            // The runs of an eval are 1 to n, each once.
            let mut keys: Vec<(&str, &str)> = recs
                .iter()
                .map(|r| (r.skill.as_str(), r.eval.as_str()))
                .collect();
            keys.dedup();
            keys.sort_unstable();
            keys.dedup();
            for (skill, eval) in keys {
                let mut runs: Vec<u32> = recs
                    .iter()
                    .filter(|r| r.skill == skill && r.eval == eval && r.with)
                    .map(|r| r.run)
                    .collect();
                runs.sort_unstable();
                let want: Vec<u32> = (1..=runs.len() as u32).collect();
                assert_eq!(runs, want, "{stem}: {skill} {eval}");
            }
            // The text has no path, no address and no mail.
            let text =
                std::fs::read_to_string(records_dir().join(format!("{stem}.jsonl"))).unwrap();
            for bad in ["\"/", "..", "\\", "@", "http"] {
                assert!(!text.contains(bad), "{stem}: `{bad}`");
            }
        }
    }

    #[test]
    fn the_records_of_the_pilot_repeat_its_published_result() {
        let files = load();
        let pilot = |target: &str| {
            let stem = files
                .iter()
                .find(|(s, _)| {
                    s.starts_with("20261117T0828") && s.ends_with(&format!("Z-{target}"))
                        || (s.starts_with("20261117T0829") && s.ends_with(&format!("Z-{target}")))
                })
                .unwrap_or_else(|| panic!("no pilot file of {target}"));
            analysis_of(&stem.1, target).0
        };
        let near = |x: f64, y: f64| (x - y).abs() < 1e-4;
        // (name, with passes, without passes, difference, low, high, verdict, rule)
        let cases = [
            (
                "execute",
                16,
                15,
                0.05,
                -0.25,
                0.3,
                Verdict::Inconclusive,
                None,
            ),
            ("tdd", 10, 1, 0.45, 0.15, 0.75, Verdict::On, Some(1)),
            ("verify", 20, 20, 0.0, 0.0, 0.0, Verdict::Inconclusive, None),
            ("debug", 15, 13, 0.1, -0.1, 0.3, Verdict::Inconclusive, None),
            (
                "agents-line",
                13,
                13,
                0.0,
                -0.45,
                0.4,
                Verdict::Inconclusive,
                None,
            ),
        ];
        for (name, w, o, value, low, high, verdict, rule) in cases {
            let a = pilot(name);
            assert_eq!((a.with.runs, a.without.runs), (20, 20), "{name}");
            assert_eq!((a.with.passes, a.without.passes), (w, o), "{name}");
            assert!(
                near(a.difference.value, value),
                "{name}: {:?}",
                a.difference
            );
            assert!(near(a.difference.low, low), "{name}: {:?}", a.difference);
            assert!(near(a.difference.high, high), "{name}: {:?}", a.difference);
            assert_eq!((a.verdict, a.rule), (verdict, rule), "{name}");
            if name == "verify" {
                let r = a.cost_ratio.expect("verify has a cost ratio");
                assert!(near(r.value, 1.100622), "{r:?}");
                assert!(near(r.low, 0.92935), "{r:?}");
                assert!(near(r.high, 1.314506), "{r:?}");
            }
            if name == "tdd" {
                assert!(a.cost_ratio.is_none());
            }
        }
    }

    #[test]
    fn a_release_row_prints_as_a_line_and_as_json() {
        let row = Row {
            name: "tdd",
            batch: "20261117T082902Z-tdd",
            claude_code: Some("2.1.326"),
            models: &["claude-a", "claude-b"],
            with: Arm {
                runs: 20,
                passes: 10,
                cost: 19.937455,
            },
            without: Arm {
                runs: 20,
                passes: 1,
                cost: 19.680765,
            },
            difference: Interval {
                value: 0.45,
                low: 0.15,
                high: 0.75,
            },
            cost_ratio: None,
            verdict: Verdict::On,
            rule: Some(1),
        };
        assert_eq!(
            release_line(&row),
            "tdd on (rule 1): with 10/20 0.50, without 1/20 0.05, cost per correct 1.993746 and 19.680765 USD, difference 0.45 [0.15, 0.75], cost ratio -, batch 20261117T082902Z-tdd, claude 2.1.326"
        );
        let j = release_json(&[row]);
        assert!(j.starts_with("{\"release\":[{\"name\":\"tdd\",\"verdict\":\"on\",\"rule\":1,\"batch\":\"20261117T082902Z-tdd\",\"claude_code\":\"2.1.326\",\"models\":[\"claude-a\",\"claude-b\"],\"with\":{\"runs\":20,\"passes\":10,\"pass_rate\":0.5,\"cost_usd\":19.937455,\"cost_per_correct_usd\":1.993746},"), "{j}");
        assert!(j.ends_with("\"cost_ratio\":null}]}"), "{j}");
    }

    /// ADR 0023. The 8 cases of the table, for each value of the constant.
    #[test]
    fn the_rule_of_adr_0023_gives_each_default() {
        use Verdict::{Inconclusive, Off, On};
        // in D2, verdict, the default with `true`, the default with `false`
        let table: [(bool, Option<Verdict>, bool, bool); 8] = [
            (true, None, false, true),
            (true, Some(On), true, true),
            (true, Some(Inconclusive), false, true),
            (true, Some(Off), false, false),
            (false, None, false, false),
            (false, Some(On), true, true),
            (false, Some(Inconclusive), false, false),
            (false, Some(Off), false, false),
        ];
        for (in_d2, verdict, with_true, with_false) in table {
            assert_eq!(
                on_by_default(in_d2, verdict, true),
                with_true,
                "in D2 {in_d2}, verdict {verdict:?}, needs_on true"
            );
            assert_eq!(
                on_by_default(in_d2, verdict, false),
                with_false,
                "in D2 {in_d2}, verdict {verdict:?}, needs_on false"
            );
        }
    }

    /// The default of a name reads the table through `release(name)` and the
    /// constant. It does not name a verdict of the table.
    #[test]
    fn the_default_of_a_name_follows_the_table_and_the_constant() {
        for name in ["brainstorm", "tdd", "debug", "no-such-skill"] {
            for in_d2 in [true, false] {
                assert_eq!(
                    default_on(name, in_d2),
                    on_by_default(in_d2, release(name), DEFAULT_ON_NEEDS_VERDICT_ON),
                    "{name}"
                );
            }
        }
    }

    /// The simulation of D33 on the bootstrap above. Each trial draws 5 evals
    /// of 4 runs for each arm, with the true pass rates 0.8 with the skill
    /// and 0.6 without it. Each run costs 1 USD, so the ratio of the costs
    /// per correct outcome is the passes without the skill over the passes
    /// with it. Run it with
    /// `cargo test --release --bin spec-dna verdict -- --ignored --nocapture`.
    #[test]
    #[ignore = "the simulation of D33 (task t5); it takes some seconds in release"]
    fn simulation_of_d33() {
        const TRIALS: u32 = 10_000;
        let (mut rule1, mut rule2, mut on) = (0u32, 0u32, 0u32);
        for t in 0..TRIALS {
            let mut g = SplitMix64::new(1_000_000 + u64::from(t));
            let mut draw = |p: f64| ((g.next_u64() >> 11) as f64 / (1u64 << 53) as f64) < p;
            let d: Vec<Vec<Pair>> = (0..5)
                .map(|_| (0..4).map(|_| pair(draw(0.8), draw(0.6))).collect())
                .collect();
            let a = analyze(&d);
            let gain = i64::from(a.with.passes) - i64::from(a.without.passes);
            let n = i64::from(a.with.runs);
            let r1 = gain * GAIN_DEN >= n * GAIN_NUM && a.difference.low > 0.0;
            let r2 = gain * LOSS_DEN >= -n
                && a.cost_ratio
                    .is_some_and(|r| r.value <= RATIO_MAX + EPS && r.high < 1.0);
            rule1 += u32::from(r1);
            rule2 += u32::from(r2);
            on += u32::from(r1 || r2);
        }
        let share = |n: u32| f64::from(n) / f64::from(TRIALS);
        println!("simulation of D33: {TRIALS} trials of 5 evals x 4 runs, true rates 0.8 and 0.6");
        println!("share of rule 1: {:.3}", share(rule1));
        println!("share of rule 2: {:.3}", share(rule2));
        println!("share of on:     {:.3}", share(on));
    }
}
