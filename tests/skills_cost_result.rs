//! Shape test of the result file of the cost measure of the stage skills
//! (step 4, S8). The test reads `bench/results/skills-cost-0.6.0.json` and
//! never starts `claude`, so it uses no network. The script that writes the
//! file is `bench/harness/skills-cost.py`. The 800-byte limit of the listing
//! has its own test in `src/harness_text.rs`.

use std::fs;

fn text() -> String {
    let p = format!(
        "{}/bench/results/skills-cost-0.6.0.json",
        env!("CARGO_MANIFEST_DIR")
    );
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// The raw text after `"key": ` up to the next `,` or newline.
fn raw<'a>(text: &'a str, key: &str) -> &'a str {
    let pat = format!("\"{key}\": ");
    let at = text
        .find(&pat)
        .unwrap_or_else(|| panic!("the key `{key}` is missing"));
    let rest = &text[at + pat.len()..];
    let end = rest.find([',', '\n']).unwrap_or(rest.len());
    rest[..end].trim()
}

fn num(text: &str, key: &str) -> f64 {
    raw(text, key)
        .parse()
        .unwrap_or_else(|_| panic!("the key `{key}` is not a number"))
}

/// The `total_tokens` values of one arm, in the order of the file.
fn arm_totals(text: &str, arm: &str) -> Vec<f64> {
    let at = text.find(&format!("\"{arm}\": [")).expect(arm);
    let rest = &text[at..];
    let end = rest.find(']').expect("end of the arm");
    rest[..end]
        .lines()
        .filter_map(|l| l.trim().strip_prefix("\"total_tokens\": "))
        .map(|v| v.trim_end_matches(',').parse().expect("a number"))
        .collect()
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

#[test]
fn the_result_file_has_the_fields_of_the_plan() {
    let t = text();
    assert!(t.starts_with("{\n") && t.ends_with("}\n"));
    for key in [
        "measure",
        "date",
        "claude_version",
        "spec_dna_commit",
        "isolation",
        "threshold_tokens",
        "runs",
        "skills_on",
        "skills_off",
        "median_tokens_skills_on",
        "median_tokens_skills_off",
        "median_difference_tokens",
        "above_threshold",
        "total_cost_usd",
    ] {
        assert!(
            t.contains(&format!("\"{key}\":")),
            "the key `{key}` is missing"
        );
    }
    assert!(raw(&t, "claude_version").contains("Claude Code"));
    assert_eq!(num(&t, "threshold_tokens"), 600.0);
}

#[test]
fn the_numbers_agree_with_the_runs() {
    let t = text();
    let runs = num(&t, "runs_per_arm") as usize;
    let on = arm_totals(&t, "skills_on");
    let off = arm_totals(&t, "skills_off");
    assert!(runs >= 3 && on.len() == runs && off.len() == runs);
    assert_eq!(num(&t, "median_tokens_skills_on"), median(on));
    assert_eq!(num(&t, "median_tokens_skills_off"), median(off));
    let diff = num(&t, "median_tokens_skills_on") - num(&t, "median_tokens_skills_off");
    assert_eq!(num(&t, "median_difference_tokens"), diff);
    assert_eq!(raw(&t, "above_threshold"), (diff > 600.0).to_string());
}

#[test]
fn the_measure_stays_in_the_budget_of_the_owner() {
    // O3: at most 3 USD in total. The file counts the 6 runs of the script.
    let t = text();
    let total = num(&t, "total_cost_usd");
    assert!(total > 0.0 && total < 3.0, "{total} USD");
    // The 4 descriptions are the only lists that differ between the arms.
    assert!(t.contains("\"only_off\": []"));
}

#[test]
fn the_result_file_holds_no_prompt_and_no_path_of_a_person() {
    let t = text();
    assert!(!t.contains("Reply with"));
    assert!(!t.contains("/Users/") && !t.contains("/home/") && !t.contains("/private/"));
}
