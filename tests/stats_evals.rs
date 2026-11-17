//! Integration tests of the Test indicators of `stats --sdlc` (step 4 of
//! 0.6.0, slice S10).
//!
//! The pass-rate indicator reads the result files that the project commits in
//! `<[evals] dir>/results`. The incident indicator pairs each incident with the
//! eval that its `Eval` section names. Both read the tree of `HEAD`, so a file
//! that is not committed does not count. Each commit has a fixed date, so no
//! test reads a clock.

mod common;

use common::*;

const PASS_LINE: &str = "Test leading: The eval pass rate over time: ";
const TIME_LINE: &str =
    "Test leading: How long a production incident takes to become a permanent eval: ";
const NO_EVALS: &str = "n/a (the configuration has no [evals] table)\n";
const NO_RESULT: &str = "n/a (no result file in evals/results; run `spec-dna eval run --out evals/results` and commit the file";
const NO_INCIDENT: &str = "n/a (no incident names an eval file in its `Eval` section)\n";

fn config(evals: &str) -> String {
    format!(
        "profile = \"standard\"\nareas = [\"valves\"]\n\n[groups]\nops = [\"valves\"]\n\n[change]\npolicy = \"three-class\"\ngate = \"warn\"\n\n{evals}\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n"
    )
}

const EVALS: &str = "[evals]\ndir = \"evals\"\n";

fn at(day: &str) -> String {
    format!("2026-{day}T12:00:00+00:00")
}

/// The text of a result file, as `eval run --out` writes it.
fn result(started: &str, pass_rate: &str, stopped: &str) -> String {
    format!(
        "{{\n  \"spec_dna\": \"0.5.0\",\n  \"claude_code\": \"2.1.325\",\n  \"started\": \"{started}\",\n  \"evals_dir\": \"evals\",\n  \"min_pass_rate\": 0.8,\n  \"max_total_usd\": 20,\n  \"estimate_usd\": 1,\n  \"spend_usd\": 0.12,\n  \"stopped\": {stopped},\n  \"pass_rate\": {pass_rate},\n  \"passed\": true,\n  \"evals\": []\n}}\n"
    )
}

fn incident(eval_section: &str) -> String {
    format!(
        "---\ntype: incident\nstatus: active\n---\n# Hose leaks\n\n## Summary\n\nA hose leaked.\n\n## Root cause\n\nA worn seal.\n\n## Eval\n\n{eval_section}\n"
    )
}

fn repo(evals: &str) -> Repo {
    let r = Repo::with_config(&config(evals));
    r.write("docs/valves.md", "# Valves\n\nText.\n");
    r.commit_at("base", &at("01-01"));
    r
}

fn stats(r: &Repo) -> String {
    let (code, out, err) = r.run(&["stats", "--sdlc"]);
    assert_eq!(code, 0, "{out}{err}");
    out
}

fn stats_json(r: &Repo, extra: &[&str]) -> String {
    let mut args = vec!["stats", "--sdlc", "--json"];
    args.extend_from_slice(extra);
    let (code, out, err) = r.run(&args);
    assert_eq!(code, 0, "{out}{err}");
    out
}

/// The text line that starts with `prefix`, and the line after it if that line
/// is a note.
fn line<'a>(out: &'a str, prefix: &str) -> &'a str {
    out.lines()
        .find(|l| l.starts_with(prefix))
        .unwrap_or_else(|| panic!("no line `{prefix}` in {out}"))
}

fn note_after<'a>(out: &'a str, prefix: &str) -> Option<&'a str> {
    let mut lines = out.lines().skip_while(|l| !l.starts_with(prefix)).skip(1);
    lines.next().filter(|l| l.starts_with("  note: "))
}

fn row<'a>(json: &'a str, name: &str) -> &'a str {
    let at = json
        .find(&format!("\"name\":\"{name}"))
        .unwrap_or_else(|| panic!("no indicator `{name}` in {json}"));
    let start = json[..at].rfind("{\"play\"").unwrap();
    let end = at + json[at..].find('}').unwrap() + 1;
    &json[start..end]
}

// Without `[evals]`

#[test]
fn without_an_evals_table_both_rows_are_na_with_the_reason() {
    let r = repo("");
    let out = stats(&r);
    assert_eq!(
        line(&out, PASS_LINE),
        format!("{PASS_LINE}{}", NO_EVALS.trim_end())
    );
    assert_eq!(
        line(&out, TIME_LINE),
        format!("{TIME_LINE}{}", NO_EVALS.trim_end())
    );
    let json = stats_json(&r, &[]);
    for name in ["The eval pass rate", "How long a production incident"] {
        let ind = row(&json, name);
        for part in [
            "\"value\":null",
            "\"unit\":null",
            "\"count\":null",
            "\"reason\":\"the configuration has no [evals] table\"",
        ] {
            assert!(ind.contains(part), "`{part}` not in {ind}");
        }
    }
}

// The pass rate

#[test]
fn with_evals_and_no_result_file_the_row_says_how_to_make_one() {
    let r = repo(EVALS);
    let out = stats(&r);
    assert_eq!(line(&out, PASS_LINE), format!("{PASS_LINE}{NO_RESULT})"));
    let json = stats_json(&r, &[]);
    let ind = row(&json, "The eval pass rate");
    for part in [
        "\"value\":null",
        "\"unit\":\"percent\"",
        "\"count\":0",
        "no result file in evals/results; run `spec-dna eval run --out evals/results` and commit the file",
    ] {
        assert!(ind.contains(part), "`{part}` not in {ind}");
    }
}

#[test]
fn one_result_file_gives_its_pass_rate_in_percent() {
    let r = repo(EVALS);
    r.write(
        "evals/results/eval-1.json",
        &result("2026-02-01T10:00:00Z", "0.75", "null"),
    );
    r.commit_at("result", &at("02-01"));
    let out = stats(&r);
    assert_eq!(
        line(&out, PASS_LINE),
        format!("{PASS_LINE}75.0 percent (1 result file)")
    );
    assert_eq!(
        note_after(&out, PASS_LINE),
        Some(
            "  note: the value is the pass rate of the newest result file (2026-02-01T10:00:00Z); the committed files hold the history"
        )
    );
    let json = stats_json(&r, &[]);
    let ind = row(&json, "The eval pass rate");
    for part in ["\"value\":75.0", "\"unit\":\"percent\"", "\"count\":1"] {
        assert!(ind.contains(part), "`{part}` not in {ind}");
    }
}

#[test]
fn the_newest_file_by_started_wins_not_the_newest_name_or_commit() {
    let r = repo(EVALS);
    // The newest by `started` has the first name and the first commit.
    r.write(
        "evals/results/a-newest.json",
        &result("2026-03-01T10:00:00Z", "0.5", "null"),
    );
    r.commit_at("first", &at("02-01"));
    r.write(
        "evals/results/z-older.json",
        &result("2026-01-15T10:00:00Z", "1", "null"),
    );
    r.commit_at("second", &at("02-02"));
    let out = stats(&r);
    assert_eq!(
        line(&out, PASS_LINE),
        format!("{PASS_LINE}50.0 percent (2 result files)")
    );
    assert!(
        note_after(&out, PASS_LINE)
            .unwrap()
            .contains("(2026-03-01T10:00:00Z)")
    );
}

#[test]
fn a_stopped_or_incomplete_file_is_skipped_and_counted_in_the_note() {
    let r = repo(EVALS);
    r.write(
        "evals/results/ok.json",
        &result("2026-02-01T10:00:00Z", "0.9", "null"),
    );
    // A run that the cap stopped has no pass rate.
    r.write(
        "evals/results/stopped.json",
        &result("2026-04-01T10:00:00Z", "null", "\"max-total-usd\""),
    );
    // A stop with a number is still a stop.
    r.write(
        "evals/results/stopped-number.json",
        &result("2026-04-02T10:00:00Z", "0.1", "\"max-total-usd\""),
    );
    r.write("evals/results/broken.json", "{ not json\n");
    r.write(
        "evals/results/text-rate.json",
        &result("2026-04-03T10:00:00Z", "\"0.2\"", "null"),
    );
    r.write(
        "evals/results/no-started.json",
        "{\"pass_rate\": 0.3, \"stopped\": null}\n",
    );
    // A file that is not JSON is not a result file.
    r.write("evals/results/README.txt", "notes\n");
    r.commit_at("results", &at("02-01"));
    let out = stats(&r);
    assert_eq!(
        line(&out, PASS_LINE),
        format!("{PASS_LINE}90.0 percent (1 result file)")
    );
    assert_eq!(
        note_after(&out, PASS_LINE),
        Some(
            "  note: the value is the pass rate of the newest result file (2026-02-01T10:00:00Z); the committed files hold the history; 5 files skipped: not a complete result"
        )
    );
}

#[test]
fn a_pass_rate_outside_0_to_1_is_not_a_complete_result() {
    let r = repo(EVALS);
    r.write(
        "evals/results/high.json",
        &result("2026-02-01T10:00:00Z", "1.5", "null"),
    );
    r.write(
        "evals/results/low.json",
        &result("2026-02-02T10:00:00Z", "-0.1", "null"),
    );
    r.commit_at("results", &at("02-01"));
    let out = stats(&r);
    assert_eq!(
        line(&out, PASS_LINE),
        format!("{PASS_LINE}{NO_RESULT}; 2 files skipped: not a complete result)")
    );
}

#[test]
fn only_files_directly_in_the_results_folder_of_head_count() {
    let r = repo(EVALS);
    // A file in a subfolder, and a result file in the evals folder itself.
    r.write(
        "evals/results/old/eval-1.json",
        &result("2026-02-01T10:00:00Z", "1", "null"),
    );
    r.write(
        "evals/eval-2.json",
        &result("2026-02-02T10:00:00Z", "1", "null"),
    );
    r.commit_at("misplaced", &at("02-01"));
    // A result file that is not committed.
    r.write(
        "evals/results/eval-3.json",
        &result("2026-02-03T10:00:00Z", "1", "null"),
    );
    let out = stats(&r);
    assert_eq!(line(&out, PASS_LINE), format!("{PASS_LINE}{NO_RESULT})"));
    // Staged is not committed either.
    r.git(&["add", "-A"]);
    let out = stats(&r);
    assert!(line(&out, PASS_LINE).contains("no result file"), "{out}");
}

#[test]
fn the_results_folder_follows_the_evals_dir() {
    let r = repo("[evals]\ndir = \".spec-dna/evals\"\n");
    r.write(
        ".spec-dna/evals/results/eval-1.json",
        &result("2026-02-01T10:00:00Z", "0.25", "null"),
    );
    r.commit_at("result", &at("02-01"));
    let out = stats(&r);
    assert_eq!(
        line(&out, PASS_LINE),
        format!("{PASS_LINE}25.0 percent (1 result file)")
    );
    let r = repo("[evals]\ndir = \".spec-dna/evals\"\n");
    let out = stats(&r);
    assert!(
        line(&out, PASS_LINE).contains(
            "no result file in .spec-dna/evals/results; run `spec-dna eval run --out .spec-dna/evals/results` and commit the file"
        ),
        "{out}"
    );
}

#[test]
fn a_shallow_clone_adds_no_note_to_the_pass_rate_row() {
    // The row reads the tree of `HEAD`, not the history.
    let r = repo(EVALS);
    r.write(
        "evals/results/eval-1.json",
        &result("2026-02-01T10:00:00Z", "1", "null"),
    );
    r.commit_at("result", &at("02-01"));
    let clone = Repo::empty();
    shallow_clone(&r.dir, &clone.dir, 1);
    let out = stats(&clone);
    assert_eq!(
        line(&out, PASS_LINE),
        format!("{PASS_LINE}100.0 percent (1 result file)")
    );
    assert!(
        !note_after(&out, PASS_LINE).unwrap().contains("shallow"),
        "{out}"
    );
}

// The time from an incident to its eval

/// Commits an incident on `day` and then an eval file on `eval_day`. Returns
/// the repository.
fn paired(section: &str, eval_files: &[&str], eval_day: &str) -> Repo {
    let r = repo(EVALS);
    r.write("docs/incidents/hose-leak.md", &incident(section));
    r.commit_at("incident", &at("02-01"));
    for f in eval_files {
        r.write(&format!("evals/{f}"), "{}\n");
    }
    r.commit_at("eval", &at(eval_day));
    r
}

#[test]
fn an_incident_and_its_eval_give_the_days_between_their_commits() {
    let r = paired("`evals/hose-leak.json`", &["hose-leak.json"], "02-04");
    let out = stats(&r);
    assert_eq!(
        line(&out, TIME_LINE),
        format!("{TIME_LINE}3.0 days (1 incident)")
    );
    assert_eq!(note_after(&out, TIME_LINE), None);
    let json = stats_json(&r, &[]);
    let ind = row(&json, "How long a production incident");
    for part in [
        "\"play\":\"Test\"",
        "\"kind\":\"leading\"",
        "\"value\":3.0",
        "\"unit\":\"days\"",
        "\"count\":1",
    ] {
        assert!(ind.contains(part), "`{part}` not in {ind}");
    }
}

#[test]
fn with_evals_and_no_incident_the_row_says_what_is_missing() {
    let r = repo(EVALS);
    let out = stats(&r);
    assert_eq!(
        line(&out, TIME_LINE),
        format!("{TIME_LINE}{}", NO_INCIDENT.trim_end())
    );
    let json = stats_json(&r, &[]);
    let ind = row(&json, "How long a production incident");
    for part in ["\"value\":null", "\"unit\":\"days\"", "\"count\":0"] {
        assert!(ind.contains(part), "`{part}` not in {ind}");
    }
}

#[test]
fn the_earliest_eval_path_in_the_section_wins() {
    // The text names the second eval first. It was added 8 days after the
    // incident, the first one 3 days after.
    let r = repo(EVALS);
    r.write(
        "docs/incidents/hose-leak.md",
        &incident("See `evals/second.json`, then `evals/first.json`."),
    );
    r.commit_at("incident", &at("02-01"));
    r.write("evals/first.json", "{}\n");
    r.commit_at("first eval", &at("02-04"));
    r.write("evals/second.json", "{}\n");
    r.commit_at("second eval", &at("02-09"));
    let out = stats(&r);
    assert_eq!(
        line(&out, TIME_LINE),
        format!("{TIME_LINE}8.0 days (1 incident)")
    );
}

#[test]
fn a_named_file_that_is_not_in_head_is_skipped_for_the_next_one() {
    let r = repo(EVALS);
    r.write(
        "docs/incidents/hose-leak.md",
        &incident("`evals/ghost.json` or `evals/real.json`"),
    );
    r.commit_at("incident", &at("02-01"));
    r.write("evals/real.json", "{}\n");
    r.commit_at("eval", &at("02-03"));
    // An uncommitted eval does not count.
    r.write("evals/ghost.json", "{}\n");
    let out = stats(&r);
    assert_eq!(
        line(&out, TIME_LINE),
        format!("{TIME_LINE}2.0 days (1 incident)")
    );
}

#[test]
fn a_path_outside_the_eval_section_does_not_count() {
    let r = repo(EVALS);
    let text = "---\ntype: incident\nstatus: active\n---\n# Hose leaks\n\n## Summary\n\nThe eval is evals/hose-leak.json.\n\n## Root cause\n\nA worn seal.\n\n## Eval\n\nNone yet.\n";
    r.write("docs/incidents/hose-leak.md", text);
    r.commit_at("incident", &at("02-01"));
    r.write("evals/hose-leak.json", "{}\n");
    r.commit_at("eval", &at("02-04"));
    let out = stats(&r);
    assert_eq!(
        line(&out, TIME_LINE),
        format!("{TIME_LINE}{}", NO_INCIDENT.trim_end())
    );
}

#[test]
fn an_incident_with_no_eval_section_gives_no_value() {
    let r = repo(EVALS);
    let text = "---\ntype: incident\nstatus: active\n---\n# Hose leaks\n\n## Summary\n\nA hose leaked.\n\n## Root cause\n\nA worn seal.\n";
    r.write("docs/incidents/hose-leak.md", text);
    r.write("evals/hose-leak.json", "{}\n");
    r.commit_at("incident and eval", &at("02-01"));
    let out = stats(&r);
    assert!(line(&out, TIME_LINE).contains("no incident names"), "{out}");
}

#[test]
fn the_heading_is_found_in_any_case_and_level_and_keeps_its_subsections() {
    let r = repo(EVALS);
    let text = "---\ntype: incident\nstatus: active\n---\n# Hose leaks\n\n## Summary\n\nA hose leaked.\n\n### eval\n\nThe file:\n\n#### Detail\n\n`evals/hose-leak.json`\n\n## Links\n\n`evals/other.json`\n";
    r.write("docs/incidents/hose-leak.md", text);
    r.commit_at("incident", &at("02-01"));
    r.write("evals/hose-leak.json", "{}\n");
    r.write("evals/other.json", "{}\n");
    r.commit_at("evals", &at("02-06"));
    let out = stats(&r);
    assert_eq!(
        line(&out, TIME_LINE),
        format!("{TIME_LINE}5.0 days (1 incident)")
    );
}

#[test]
fn only_a_file_directly_in_the_evals_folder_is_an_eval_file() {
    // A result file, a fixture and a path in another folder are not evals.
    let r = paired(
        "`evals/results/eval-1.json`, `evals/fixtures/a.json`, `docs/evals/hose-leak.json`",
        &["results/eval-1.json", "fixtures/a.json"],
        "02-04",
    );
    r.write("docs/evals/hose-leak.json", "{}\n");
    r.commit_at("other", &at("02-05"));
    let out = stats(&r);
    assert_eq!(
        line(&out, TIME_LINE),
        format!("{TIME_LINE}{}", NO_INCIDENT.trim_end())
    );
}

#[test]
fn the_left_edge_of_a_path_decides_the_match() {
    // `docs/evals/x.json` is no `evals/x.json`; a bracket or a quote before
    // the path is an edge.
    let r = repo(EVALS);
    r.write(
        "docs/incidents/hose-leak.md",
        &incident("see docs/evals/x.json and [it](evals/hose-leak.json)"),
    );
    r.commit_at("incident", &at("02-01"));
    r.write("evals/x.json", "{}\n");
    r.write("evals/hose-leak.json", "{}\n");
    r.commit_at("evals", &at("02-03"));
    let out = stats(&r);
    assert_eq!(
        line(&out, TIME_LINE),
        format!("{TIME_LINE}2.0 days (1 incident)")
    );
}

#[test]
fn a_relative_or_root_link_to_the_eval_names_it() {
    for link in [
        "[eval](../../evals/hose-leak.json)",
        "[eval](./evals/hose-leak.json)",
        "[eval](/evals/hose-leak.json)",
        "[eval](../../../evals/hose-leak.json#top)",
    ] {
        let r = paired(link, &["hose-leak.json"], "02-04");
        let out = stats(&r);
        assert_eq!(
            line(&out, TIME_LINE),
            format!("{TIME_LINE}3.0 days (1 incident)"),
            "{link}"
        );
    }
    // A longer path before the folder is another folder.
    for link in [
        "[eval](../docs/evals/hose-leak.json)",
        "x../evals/hose-leak.json",
    ] {
        let r = paired(link, &["hose-leak.json"], "02-04");
        let out = stats(&r);
        assert_eq!(
            line(&out, TIME_LINE),
            format!("{TIME_LINE}{}", NO_INCIDENT.trim_end()),
            "{link}"
        );
    }
}

#[test]
fn an_incident_and_an_eval_in_one_commit_take_0_days() {
    let r = repo(EVALS);
    r.write(
        "docs/incidents/hose-leak.md",
        &incident("`evals/hose-leak.json`"),
    );
    r.write("evals/hose-leak.json", "{}\n");
    r.commit_at("both", &at("02-01"));
    let out = stats(&r);
    assert_eq!(
        line(&out, TIME_LINE),
        format!("{TIME_LINE}0.0 days (1 incident)")
    );
}

#[test]
fn a_span_below_0_does_not_count() {
    // The eval commit has an earlier date than the incident commit.
    let r = repo(EVALS);
    r.write(
        "docs/incidents/hose-leak.md",
        &incident("`evals/hose-leak.json`"),
    );
    r.commit_at("incident", &at("02-05"));
    r.write("evals/hose-leak.json", "{}\n");
    r.commit_at("eval", &at("02-01"));
    let out = stats(&r);
    assert_eq!(
        line(&out, TIME_LINE),
        format!("{TIME_LINE}{}", NO_INCIDENT.trim_end())
    );
    let json = stats_json(&r, &[]);
    assert!(
        row(&json, "How long a production incident").contains("\"count\":0"),
        "{json}"
    );
}

#[test]
fn the_value_is_the_median_not_the_mean() {
    let r = repo(EVALS);
    for (slug, incident_day, eval_day) in [
        ("a", "02-01", "02-02"),
        ("b", "02-03", "02-05"),
        ("c", "02-06", "02-15"),
    ] {
        r.write(
            &format!("docs/incidents/{slug}.md"),
            &incident(&format!("`evals/{slug}.json`")),
        );
        r.commit_at(&format!("incident {slug}"), &at(incident_day));
        r.write(&format!("evals/{slug}.json"), "{}\n");
        r.commit_at(&format!("eval {slug}"), &at(eval_day));
    }
    let out = stats(&r);
    // The spans are 1, 2 and 9 days.
    assert_eq!(
        line(&out, TIME_LINE),
        format!("{TIME_LINE}2.0 days (3 incidents)")
    );
}

#[test]
fn an_eval_that_a_later_commit_edits_keeps_its_add_commit() {
    let r = paired("`evals/hose-leak.json`", &["hose-leak.json"], "02-03");
    r.write("evals/hose-leak.json", "{\"id\": \"hose-leak\"}\n");
    r.commit_at("edit the eval", &at("03-20"));
    let out = stats(&r);
    assert_eq!(
        line(&out, TIME_LINE),
        format!("{TIME_LINE}2.0 days (1 incident)")
    );
}

#[test]
fn an_eval_that_was_deleted_and_added_again_keeps_its_oldest_add() {
    let r = paired("`evals/hose-leak.json`", &["hose-leak.json"], "02-03");
    r.git(&["rm", "-q", "evals/hose-leak.json"]);
    r.commit_at("remove the eval", &at("02-10"));
    r.write("evals/hose-leak.json", "{}\n");
    r.commit_at("add it again", &at("02-20"));
    let out = stats(&r);
    assert_eq!(
        line(&out, TIME_LINE),
        format!("{TIME_LINE}2.0 days (1 incident)")
    );
}

#[test]
fn a_shallow_clone_adds_its_note_to_the_incident_row() {
    let r = paired("`evals/hose-leak.json`", &["hose-leak.json"], "02-04");
    let clone = Repo::empty();
    shallow_clone(&r.dir, &clone.dir, 1);
    let out = stats(&clone);
    assert!(
        note_after(&out, TIME_LINE)
            .unwrap()
            .contains("shallow clone"),
        "{out}"
    );
}

// Both rows

#[test]
fn the_group_filter_does_not_change_the_test_rows() {
    let r = paired("`evals/hose-leak.json`", &["hose-leak.json"], "02-04");
    r.write(
        "evals/results/eval-1.json",
        &result("2026-02-01T10:00:00Z", "0.5", "null"),
    );
    r.commit_at("result", &at("02-05"));
    let all = stats(&r);
    let (code, group, err) = r.run(&["stats", "--sdlc", "--group", "ops"]);
    assert_eq!(code, 0, "{group}{err}");
    for prefix in [PASS_LINE, TIME_LINE] {
        assert_eq!(line(&all, prefix), line(&group, prefix));
    }
}

#[test]
fn a_repository_with_no_commit_gives_the_no_data_rows() {
    let r = Repo::with_config(&config(EVALS));
    let out = stats(&r);
    assert!(
        line(&out, PASS_LINE).starts_with(&format!("{PASS_LINE}{NO_RESULT}")),
        "{out}"
    );
    assert_eq!(
        line(&out, TIME_LINE),
        format!("{TIME_LINE}{}", NO_INCIDENT.trim_end())
    );
}

#[test]
fn the_process_off_gives_no_indicator() {
    let r = Repo::with_config(&config(EVALS).replace("three-class", "none"));
    r.write("docs/valves.md", "# Valves\n\nText.\n");
    r.commit_at("base", &at("01-01"));
    let out = stats(&r);
    assert!(!out.contains("Test leading"), "{out}");
}

// The golden file

fn golden(name: &str, actual: &str) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing golden {name}; run with UPDATE_GOLDEN=1"));
    assert_eq!(
        actual, expected,
        "golden {name} differs; run with UPDATE_GOLDEN=1 to accept"
    );
}

#[test]
fn stats_text_with_evals_matches_the_golden_file() {
    let r = repo(EVALS);
    for (slug, incident_day, eval_day) in [("a", "02-01", "02-03"), ("b", "02-05", "02-12")] {
        r.write(
            &format!("docs/incidents/{slug}.md"),
            &incident(&format!("`evals/{slug}.json`")),
        );
        r.commit_at(&format!("incident {slug}"), &at(incident_day));
        r.write(&format!("evals/{slug}.json"), "{}\n");
        r.commit_at(&format!("eval {slug}"), &at(eval_day));
    }
    r.write(
        "evals/results/eval-1.json",
        &result("2026-02-13T10:00:00Z", "0.5", "null"),
    );
    r.write(
        "evals/results/eval-2.json",
        &result("2026-02-20T10:00:00Z", "0.8", "null"),
    );
    r.write(
        "evals/results/eval-3.json",
        &result("2026-02-21T10:00:00Z", "null", "\"max-total-usd\""),
    );
    r.commit_at("results", &at("02-21"));
    let out = stats(&r);
    assert!(!out.contains(VERSION), "{out}");
    golden("stats-sdlc-evals.txt", &out);
}

#[test]
fn the_result_of_eval_run_ab_is_no_complete_result() {
    // The result of `--ab` has no `pass_rate` at the top level, so the
    // indicator skips it and counts it in the note.
    let r = repo(EVALS);
    r.write(
        "evals/results/eval-20260201T100000Z.json",
        &result("2026-02-01T10:00:00Z", "0.9", "null"),
    );
    r.write(
        "evals/results/skills-20260301T100000Z.json",
        "{\"spec_dna\":\"0.7.0\",\"claude_code\":\"2.1.325\",\"models\":[],\"started\":\"2026-03-01T10:00:00Z\",\"target\":\"tdd\",\"seed\":24,\"resamples\":2000,\"max_total_usd\":100,\"estimate_usd\":40,\"spend_usd\":3,\"excluded\":{\"with\":0,\"without\":0},\"stopped\":null,\"skills\":[]}\n",
    );
    r.commit_at("results", &at("02-01"));
    let out = stats(&r);
    assert_eq!(
        line(&out, PASS_LINE),
        format!("{PASS_LINE}90.0 percent (1 result file)")
    );
    assert!(
        note_after(&out, PASS_LINE)
            .unwrap()
            .contains("; 1 file skipped: not a complete result"),
        "{out}"
    );
}
