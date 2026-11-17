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

/// The verdicts that the binary holds for the release. The run step fills it
/// from the committed result of the pilot and of the full run before the tag.
/// It is empty until then, so no skill has a verdict of the release.
pub const RELEASE: &[(&str, Verdict)] = &[];

/// The verdict of the release for a name, if the table holds one.
pub fn release(name: &str) -> Option<Verdict> {
    RELEASE.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
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

/// True if the project or the release gives the name the verdict `on`.
pub fn is_on(name: &str, files: &[(String, Vec<u8>)]) -> bool {
    resolve(name, files).verdict == Some(Verdict::On)
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
        let none = resolve("verify", &files);
        assert_eq!((none.verdict, none.source), (None, Source::None));
        assert!(is_on("debug", &files));
        assert!(!is_on("tdd", &files));
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
    fn the_table_of_the_release_is_empty_until_the_run_fills_it() {
        assert!(RELEASE.is_empty());
        assert_eq!(release("tdd"), None);
        assert_eq!(names().len(), 12);
        assert_eq!(names()[11], "agents-line");
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
