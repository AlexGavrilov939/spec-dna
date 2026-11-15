//! Integration tests of `log` and `stats --sdlc` (step 3 of 0.6.0, slice S8).
//!
//! The history of a change is the only record of it after `finish`, so each
//! test builds a repository history. The tests of the merge kinds use the
//! helpers of `tests/common/mod.rs`, which commit at the time of the clock:
//! they count entries and read no date. The tests of the indicators set the
//! date of each commit, so no test reads a clock and the golden file is the
//! same on each run.

mod common;

use common::*;
use std::fs;
use std::path::Path;

const FOLDER: &str = "docs/intent/dose";

fn config(change: &str) -> String {
    format!(
        "profile = \"standard\"\nareas = [\"valves\", \"sensors\"]\n\n[groups]\noperations = [\"valves\"]\n\n{change}\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n"
    )
}

const ON: &str = "[change]\npolicy = \"three-class\"\ngate = \"warn\"\n";

fn intent(title: &str, status: &str, class: &str, areas: &str) -> String {
    format!(
        "---\nstatus: {status}\nclass: {class}\nareas: [{areas}]\n---\n# {title}\n\n## Intent\n\nThe dose is too high.\n"
    )
}

/// A spec with 1 `ADD`, 1 `CHANGE` and 1 `REMOVE` line. `log` counts the
/// lines by their form, so the hashes need not match a requirement.
const SPEC: &str = "---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n- ADD requirements/valves#hold: The controller SHALL hold the valve.\n- CHANGE requirements/valves#close@0123456789abcdef: The controller SHALL close the valve in 4 seconds.\n- REMOVE requirements/valves#open@fedcba9876543210\n";

fn spec(lines: &[&str]) -> String {
    format!(
        "---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n{}\n",
        lines.join("\n")
    )
}

fn plan(status: &str, files: &[&str]) -> String {
    let items: Vec<String> = files.iter().map(|f| format!("- `{f}`")).collect();
    format!(
        "---\nstatus: {status}\n---\n# Plan\n\n## Files\n\n{}\n\n## Order\n\n- [ ] t1: Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n",
        items.join("\n")
    )
}

/// A repository whose `main` holds the change `dose` with a spec and a plan.
fn with_dose(status: &str) -> Repo {
    let r = Repo::with_config(&config(ON));
    r.write(
        &format!("{FOLDER}/intent.md"),
        &intent("Dose", status, "higher-risk", "valves"),
    );
    r.write(&format!("{FOLDER}/spec.md"), SPEC);
    r.write(
        &format!("{FOLDER}/plan.md"),
        &plan("approved", &["src/dose.py"]),
    );
    r.write("src/dose.py", "x = 1\n");
    r.commit("base");
    r
}

fn remove_folder(r: &Repo) {
    fs::remove_dir_all(r.dir.join(FOLDER)).unwrap();
}

fn log_json(r: &Repo, extra: &[&str]) -> String {
    let mut args = vec!["log", "--json"];
    args.extend_from_slice(extra);
    let (code, out, err) = r.run(&args);
    assert_eq!(code, 0, "{out}{err}");
    out
}

fn entries(out: &str) -> usize {
    out.matches("\"slug\":").count()
}

// log: the 3 kinds of merge

/// The finish branch brings the deletion. The folder came to `main` in an
/// earlier commit, so each kind of merge gives 1 entry.
fn one_entry_after(merge: fn(&Path)) {
    let r = with_dose("accepted");
    remove_folder(&r);
    merge(&r.dir);
    let out = log_json(&r, &[]);
    assert_eq!(entries(&out), 1, "{out}");
    assert!(out.contains("\"slug\":\"dose\""), "{out}");
    assert!(out.contains("\"result\":\"finished\""), "{out}");
    assert!(
        out.contains("\"delta\":{\"add\":1,\"change\":1,\"remove\":1}"),
        "{out}"
    );
    assert!(out.contains("\"class\":\"higher-risk\""), "{out}");
    assert!(out.contains("\"title\":\"Dose\""), "{out}");
    let (code, text, _) = r.run(&["log"]);
    assert_eq!(code, 0);
    assert_eq!(text.matches(" dose ").count(), 1, "{text}");
}

#[test]
fn log_after_a_squash_merge_has_one_entry() {
    one_entry_after(merge_squash);
}

#[test]
fn log_after_a_merge_commit_has_one_entry() {
    one_entry_after(merge_commit);
}

#[test]
fn log_after_a_rebase_merge_has_one_entry() {
    one_entry_after(merge_rebase);
}

#[test]
fn the_entry_of_a_merge_commit_is_the_commit_of_the_branch() {
    let r = with_dose("accepted");
    remove_folder(&r);
    merge_commit(&r.dir);
    let out = log_json(&r, &[]);
    let sha = out.split("\"commit\":\"").nth(1).unwrap();
    let sha = sha.split('"').next().unwrap();
    assert_eq!(sha.len(), 40, "{out}");
    assert_eq!(r.git_out(&["log", "-1", "--format=%s", sha]), "feature a");
    assert_eq!(
        r.git_out(&["rev-list", "--parents", "-n", "1", sha])
            .split(' ')
            .count(),
        2,
        "the entry has 1 parent"
    );
}

// log: the result

#[test]
fn a_close_first_gives_closed() {
    let r = with_dose("accepted");
    r.write(
        &format!("{FOLDER}/intent.md"),
        &intent("Dose", "closed", "higher-risk", "valves"),
    );
    r.commit_at("close", "2024-02-01T12:00:00+00:00");
    remove_folder(&r);
    r.commit_at("finish later", "2024-02-10T12:00:00+00:00");
    let out = log_json(&r, &[]);
    assert_eq!(entries(&out), 1, "{out}");
    assert!(out.contains("\"result\":\"closed\""), "{out}");
    assert!(out.contains("\"date\":\"2024-02-10\""), "{out}");
}

#[test]
fn a_draft_in_the_parent_tree_gives_closed() {
    let r = with_dose("draft");
    remove_folder(&r);
    r.commit("delete a draft");
    let out = log_json(&r, &[]);
    assert!(out.contains("\"result\":\"closed\""), "{out}");
}

#[test]
fn a_change_of_one_pull_request_has_no_entry() {
    // The pull request adds the intent, sets it `accepted`, and deletes the
    // folder. A squash merge gives `main` 1 commit with none of the folder.
    let r = Repo::with_config(&config(ON));
    r.write("src/dose.py", "x = 1\n");
    r.commit("base");
    r.git(&["checkout", "-q", "-b", "pr"]);
    r.write(
        &format!("{FOLDER}/intent.md"),
        &intent("Dose", "draft", "routine", "valves"),
    );
    r.commit("add the intent");
    r.write(
        &format!("{FOLDER}/intent.md"),
        &intent("Dose", "accepted", "routine", "valves"),
    );
    r.commit("accept it");
    remove_folder(&r);
    r.write("src/dose.py", "x = 2\n");
    r.commit("finish");
    r.git(&["checkout", "-q", "main"]);
    r.git(&["merge", "-q", "--squash", "pr"]);
    r.commit("squash of the pull request");
    let out = log_json(&r, &[]);
    assert_eq!(entries(&out), 0, "{out}");
    let (code, text, _) = r.run(&["log"]);
    assert_eq!(code, 0);
    assert!(text.contains("no finished or closed change"), "{text}");
}

#[test]
fn a_move_out_of_the_intent_folder_reads_as_a_deletion() {
    // Deviation D30: git sees the move as a deletion of the path of the
    // intent.
    let r = with_dose("accepted");
    fs::create_dir_all(r.dir.join("docs/archive")).unwrap();
    r.git(&["mv", FOLDER, "docs/archive/dose"]);
    r.commit("archive");
    let out = log_json(&r, &[]);
    assert_eq!(entries(&out), 1, "{out}");
    assert!(out.contains("\"result\":\"finished\""), "{out}");
    // `stats` counts the same ending.
    let (_, text, _) = r.run(&["stats", "--sdlc"]);
    assert!(
        text.starts_with("1 change in the history: 1 finished, 0 closed, 0 open\n"),
        "{text}"
    );
}

#[test]
fn a_rename_inside_the_intent_folder_is_not_an_end_for_log_or_stats() {
    // Both walks use the rename detection of git. The rename keeps the life,
    // and the later finish ends it under the new name.
    let r = with_dose("accepted");
    r.git(&["mv", FOLDER, "docs/intent/dose-2"]);
    r.commit("rename the change");
    assert_eq!(entries(&log_json(&r, &[])), 0);
    let (_, text, _) = r.run(&["stats", "--sdlc"]);
    assert!(
        text.starts_with("1 change in the history: 0 finished, 0 closed, 1 open\n"),
        "{text}"
    );
    fs::remove_dir_all(r.dir.join("docs/intent/dose-2")).unwrap();
    r.commit("finish");
    let out = log_json(&r, &[]);
    assert_eq!(entries(&out), 1, "{out}");
    assert!(out.contains("\"slug\":\"dose-2\""), "{out}");
    let (_, text, _) = r.run(&["stats", "--sdlc"]);
    assert!(
        text.starts_with("1 change in the history: 1 finished, 0 closed, 0 open\n"),
        "{text}"
    );
}

// log: the options and the output

#[test]
fn since_limits_the_walk() {
    let r = Repo::with_config(&config(ON));
    for (slug, day) in [("one", "2024-03-01"), ("two", "2024-03-02")] {
        r.write(
            &format!("docs/intent/{slug}/intent.md"),
            &intent(slug, "accepted", "routine", "valves"),
        );
        r.commit_at(&format!("add {slug}"), &format!("{day}T08:00:00+00:00"));
    }
    r.git(&["rm", "-q", "-r", "docs/intent/one"]);
    r.commit_at("finish one", "2024-03-03T08:00:00+00:00");
    let after_one = r.git_out(&["rev-parse", "HEAD"]);
    r.git(&["rm", "-q", "-r", "docs/intent/two"]);
    r.commit_at("finish two", "2024-03-04T08:00:00+00:00");
    let all = log_json(&r, &[]);
    assert_eq!(entries(&all), 2, "{all}");
    // The newest first.
    assert!(all.find("\"slug\":\"two\"") < all.find("\"slug\":\"one\""));
    let since = log_json(&r, &["--since", &after_one]);
    assert_eq!(entries(&since), 1, "{since}");
    assert!(since.contains("\"slug\":\"two\""), "{since}");
    let since_head = log_json(&r, &["--since", "HEAD"]);
    assert_eq!(entries(&since_head), 0, "{since_head}");
    // A revision that is not a commit is a configuration error.
    let (code, _, err) = r.run(&["log", "--since", "no-such-revision"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("no-such-revision"), "{err}");
}

#[test]
fn the_json_has_the_fields_of_the_contract_in_order() {
    let r = with_dose("accepted");
    remove_folder(&r);
    r.commit_at("finish", "2024-04-05T10:00:00+00:00");
    let out = log_json(&r, &[]);
    let sha = r.git_out(&["rev-parse", "HEAD"]);
    assert_eq!(
        out.trim_end(),
        format!(
            "{{\"changes\":[{{\"slug\":\"dose\",\"class\":\"higher-risk\",\"title\":\"Dose\",\"result\":\"finished\",\"delta\":{{\"add\":1,\"change\":1,\"remove\":1}},\"commit\":\"{sha}\",\"date\":\"2024-04-05\"}}],\"shallow\":false}}"
        )
    );
}

#[test]
fn the_text_has_one_line_for_each_change() {
    let r = with_dose("accepted");
    remove_folder(&r);
    r.commit_at("finish", "2024-04-05T10:00:00+00:00");
    let (code, text, _) = r.run(&["log"]);
    assert_eq!(code, 0);
    let sha = r.git_out(&["rev-parse", "HEAD"]);
    assert_eq!(
        text,
        format!(
            "2024-04-05 finished dose higher-risk add 1 change 1 remove 1 {} Dose\n",
            &sha[..12]
        )
    );
}

#[test]
fn a_shallow_clone_sets_shallow_and_misses_older_changes() {
    let r = with_dose("accepted");
    remove_folder(&r);
    r.commit("finish");
    let full = log_json(&r, &[]);
    assert_eq!(entries(&full), 1, "{full}");
    assert!(full.contains("\"shallow\":false"), "{full}");
    let clone = Repo::empty();
    shallow_clone(&r.dir, &clone.dir, 1);
    let out = log_json(&clone, &[]);
    assert!(out.contains("\"shallow\":true"), "{out}");
    assert_eq!(entries(&out), 0, "{out}");
    let (code, text, _) = clone.run(&["log"]);
    assert_eq!(code, 0);
    assert!(
        text.contains("shallow clone; older changes may be missing"),
        "{text}"
    );
    // The statistics say the same.
    let (code, stats, _) = clone.run(&["stats", "--sdlc"]);
    assert_eq!(code, 0);
    assert!(stats.contains("shallow clone"), "{stats}");
}

#[test]
fn a_repository_with_no_commit_has_no_entry() {
    let r = Repo::with_config(&config(ON));
    let out = log_json(&r, &[]);
    assert_eq!(out.trim_end(), "{\"changes\":[],\"shallow\":false}");
    let (code, out, err) = r.run(&["stats", "--sdlc", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"play\":\"Plan\""), "{out}");
}

#[test]
fn the_policy_none_says_that_the_process_is_off() {
    let r = Repo::with_config(&config(""));
    r.write(
        &format!("{FOLDER}/intent.md"),
        &intent("Dose", "accepted", "routine", "valves"),
    );
    r.commit("add");
    r.git(&["rm", "-q", "-r", FOLDER]);
    r.commit("delete");
    let (code, out, _) = r.run(&["log"]);
    assert_eq!(code, 0);
    assert!(out.contains("the change process is off"), "{out}");
    assert!(!out.contains("dose"), "{out}");
    let json = log_json(&r, &[]);
    assert_eq!(json.trim_end(), "{\"changes\":[],\"shallow\":false}");
    let (code, out, _) = r.run(&["stats", "--sdlc"]);
    assert_eq!(code, 0);
    assert!(out.contains("the change process is off"), "{out}");
    let (code, out, _) = r.run(&["stats", "--sdlc", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(out.trim_end(), "{\"indicators\":[]}");
}

#[test]
fn a_pin_mismatch_is_a_warning_for_log_and_stats() {
    let r = with_dose("accepted");
    let text = r.read("spec-dna.toml").replace(VERSION, "0.0.1");
    r.write("spec-dna.toml", &text);
    for args in [&["log"][..], &["stats", "--sdlc"][..]] {
        let (code, _, err) = r.run(args);
        assert_eq!(code, 0, "{err}");
        assert!(err.contains("warning: spec-dna.toml pins 0.0.1"), "{err}");
    }
}

#[test]
fn a_usage_error_is_exit_code_2() {
    let r = with_dose("accepted");
    let (code, _, err) = r.run(&["stats"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("--sdlc"), "{err}");
    let (code, _, err) = r.run(&["stats", "--sdlc", "--group", "nowhere"]);
    assert_eq!(code, 2, "{err}");
    assert!(
        err.contains("nowhere") && err.contains("operations"),
        "{err}"
    );
    for args in [
        &["log", "--group", "operations"][..],
        &["log", "--base", "HEAD"][..],
        &["log", "--sdlc"][..],
        &["stats", "--sdlc", "--since", "HEAD"][..],
        &["stats", "--sdlc", "--base", "HEAD"][..],
        &["status", "--since", "HEAD"][..],
        &["check", "--sdlc"][..],
        &["log", "extra"][..],
    ] {
        let (code, _, err) = r.run(args);
        assert_eq!(code, 2, "{args:?}: {err}");
    }
}

#[test]
fn the_help_lists_log_and_stats() {
    let r = Repo::new();
    let (code, out, _) = r.run(&["--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("log [--since <rev>]"), "{out}");
    assert!(out.contains("stats --sdlc [--group <name>]"), "{out}");
    let (code, out, _) = r.run(&["stats", "--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("n/a"), "{out}");
}

// stats --sdlc

const T: &str = "T12:00:00+00:00";

fn at(day: &str) -> String {
    format!("2024-{day}{T}")
}

/// The history of 4 changes, each commit on a fixed date.
///
/// - `alpha` (valves, higher-risk): intent 01-01, accepted 01-03, spec 01-05,
///   plan 01-06, plan approved 01-08, spec edited 01-09, finished 01-12 inside
///   its plan.
/// - `beta` (sensors): intent 01-02, accepted 01-04, plan approved 01-05,
///   finished 01-11 with a file that the plan does not list.
/// - `gamma` (valves): intent 01-03, accepted 01-04, closed 01-06, deleted
///   01-07.
/// - `delta` (sensors): a draft since 01-10, still open.
fn history() -> Repo {
    let r = Repo::with_config(&config(ON));
    let put = |path: &str, text: &str| r.write(path, text);
    let i = |slug: &str, title: &str, status: &str, class: &str, areas: &str| {
        put(
            &format!("docs/intent/{slug}/intent.md"),
            &intent(title, status, class, areas),
        );
    };
    r.write("src/valves.py", "v = 1\n");
    r.write("src/sensors.py", "s = 1\n");
    r.write("docs/requirements/valves.md", "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n");
    i("alpha", "Alpha", "draft", "higher-risk", "valves");
    r.commit_at("alpha draft", &at("01-01"));
    i("beta", "Beta", "draft", "routine", "sensors");
    r.commit_at("beta draft", &at("01-02"));
    i("gamma", "Gamma", "draft", "routine", "valves");
    r.commit_at("gamma draft", &at("01-03"));
    i("alpha", "Alpha", "accepted", "higher-risk", "valves");
    r.commit_at("alpha accepted", &at("01-03"));
    i("beta", "Beta", "accepted", "routine", "sensors");
    r.commit_at("beta accepted", &at("01-04"));
    i("gamma", "Gamma", "accepted", "routine", "valves");
    r.commit_at("gamma accepted", &at("01-04"));
    put(
        "docs/intent/alpha/spec.md",
        &spec(&["- ADD requirements/valves#hold: The controller SHALL hold the valve."]),
    );
    r.commit_at("alpha spec", &at("01-05"));
    put(
        "docs/intent/beta/plan.md",
        &plan("approved", &["src/sensors.py"]),
    );
    r.commit_at("beta plan approved", &at("01-05"));
    put(
        "docs/intent/alpha/plan.md",
        &plan("draft", &["src/valves.py"]),
    );
    r.commit_at("alpha plan", &at("01-06"));
    i("gamma", "Gamma", "closed", "routine", "valves");
    r.commit_at("gamma closed", &at("01-06"));
    fs::remove_dir_all(r.dir.join("docs/intent/gamma")).unwrap();
    r.commit_at("gamma deleted", &at("01-07"));
    put(
        "docs/intent/alpha/plan.md",
        &plan("approved", &["src/valves.py"]),
    );
    r.commit_at("alpha plan approved", &at("01-08"));
    put(
        "docs/intent/alpha/spec.md",
        &spec(&[
            "- ADD requirements/valves#hold: The controller SHALL hold the valve.",
            "- ADD requirements/valves#vent: The controller SHALL vent the valve.",
        ]),
    );
    r.commit_at("alpha spec edited", &at("01-09"));
    i("delta", "Delta", "draft", "routine", "sensors");
    r.commit_at("delta draft", &at("01-10"));
    fs::remove_dir_all(r.dir.join("docs/intent/beta")).unwrap();
    r.write("src/sensors.py", "s = 2\n");
    r.write("src/other.py", "o = 1\n");
    r.commit_at("beta finished", &at("01-11"));
    fs::remove_dir_all(r.dir.join("docs/intent/alpha")).unwrap();
    r.write("src/valves.py", "v = 2\n");
    r.write("docs/requirements/valves.md", "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n- **hold**: The controller SHALL hold the valve.\n");
    r.commit_at("alpha finished", &at("01-12"));
    r
}

/// The part of the JSON of `stats` for one indicator, found by its name.
fn indicator<'a>(json: &'a str, name: &str) -> &'a str {
    let at = json
        .find(&format!("\"name\":\"{name}"))
        .unwrap_or_else(|| panic!("no indicator `{name}` in {json}"));
    let start = json[..at].rfind("{\"play\"").unwrap();
    let end = at + json[at..].find('}').unwrap() + 1;
    &json[start..end]
}

fn stats_json(r: &Repo, extra: &[&str]) -> String {
    let mut args = vec!["stats", "--sdlc", "--json"];
    args.extend_from_slice(extra);
    let (code, out, err) = r.run(&args);
    assert_eq!(code, 0, "{out}{err}");
    out
}

fn has(ind: &str, parts: &[&str]) {
    for p in parts {
        assert!(ind.contains(p), "`{p}` not in {ind}");
    }
}

#[test]
fn log_reads_the_whole_history_of_the_fixture() {
    let r = history();
    let out = log_json(&r, &[]);
    assert_eq!(entries(&out), 3, "{out}");
    let a = out.find("\"slug\":\"alpha\"").unwrap();
    let b = out.find("\"slug\":\"beta\"").unwrap();
    let g = out.find("\"slug\":\"gamma\"").unwrap();
    assert!(a < b && b < g, "{out}");
    assert!(out.contains("\"date\":\"2024-01-12\""), "{out}");
    let (_, text, _) = r.run(&["log"]);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "{text}");
    assert!(lines[0].starts_with("2024-01-12 finished alpha higher-risk add 2 change 0 remove 0 "));
    assert!(lines[1].starts_with("2024-01-11 finished beta routine add 0 change 0 remove 0 "));
    assert!(lines[2].starts_with("2024-01-07 closed gamma routine add 0 change 0 remove 0 "));
}

#[test]
fn stats_gives_the_indicators_of_the_playbook() {
    let r = history();
    let out = stats_json(&r, &[]);
    assert!(out.starts_with("{\"indicators\":[{\"play\":\"Plan\",\"kind\":\"leading\""));
    // Plan, leading: the median of 2, 2 and 1 days.
    let ind = indicator(&out, "Time from first conversation");
    has(
        ind,
        &[
            "\"value\":2.0",
            "\"unit\":\"days\"",
            "\"count\":3",
            "not in git",
        ],
    );
    // Plan, lagging: 2 of 3 are accepted rather than closed. The open draft
    // does not count.
    let ind = indicator(&out, "Survival rate");
    has(
        ind,
        &["\"value\":66.7", "\"unit\":\"percent\"", "\"count\":3"],
    );
    // Design, leading: alpha only.
    let ind = indicator(&out, "Elapsed time between");
    has(ind, &["\"value\":4.0", "\"count\":1"]);
    // Design, lagging: 1 spec commit after the first plan commit.
    let ind = indicator(&out, "Requirements rework");
    has(
        ind,
        &[
            "\"value\":1.0",
            "\"unit\":\"commits per change\"",
            "\"count\":1",
        ],
    );
    // Build, leading: 4 and 6 days.
    let ind = indicator(&out, "Time from plan approval");
    has(ind, &["\"value\":5.0", "\"unit\":\"days\"", "\"count\":2"]);
    // Build, lagging: alpha stays in its plan, beta does not.
    let ind = indicator(&out, "How often the merged diff");
    has(
        ind,
        &["\"value\":50.0", "\"unit\":\"percent\"", "\"count\":2"],
    );
}

#[test]
fn an_indicator_that_git_cannot_give_has_a_null_value_and_a_reason() {
    let r = history();
    let out = stats_json(&r, &[]);
    let na = [
        ("Share of changes that merge", "pull request metadata"),
        ("Rework cycles", "pull request metadata"),
        ("The eval pass rate", "evals come in step 4"),
        ("How long a production incident", "evals come in step 4"),
        ("Regressions caught in CI", "incident tracker"),
        ("Time from band breach", "no bands"),
        ("Repeat incidents", "no class field"),
        ("The other indicators", "CI logs"),
    ];
    for (name, reason) in na {
        let ind = indicator(&out, name);
        has(
            ind,
            &["\"value\":null", "\"unit\":null", "\"count\":null", reason],
        );
    }
    // Each null value has a reason, also for an indicator that has no data.
    let empty = Repo::with_config(&config(ON));
    empty.write("src/a.py", "a\n");
    empty.commit("start");
    let out = stats_json(&empty, &[]);
    let rows: Vec<&str> = out.split("{\"play\"").skip(1).collect();
    assert_eq!(rows.len(), 14, "{out}");
    for row in rows {
        if row.contains("\"value\":null") {
            assert!(row.contains("\"reason\":\""), "{row}");
        }
    }
}

#[test]
fn the_json_of_stats_has_the_fields_of_the_contract_in_order() {
    let r = history();
    let out = stats_json(&r, &[]);
    let row = indicator(&out, "Survival rate");
    assert!(
        row.starts_with("{\"play\":\"Plan\",\"kind\":\"lagging\",\"name\":\"Survival rate"),
        "{row}"
    );
    let keys: Vec<&str> = [
        "\"play\"",
        "\"kind\"",
        "\"name\"",
        "\"value\"",
        "\"unit\"",
        "\"count\"",
        "\"reason\"",
    ]
    .into();
    let mut last = 0;
    for k in keys {
        let at = row.find(k).unwrap_or_else(|| panic!("{k} in {row}"));
        assert!(at >= last, "{k} out of order in {row}");
        last = at;
    }
    assert!(out.ends_with("}]}\n"), "{out}");
}

#[test]
fn a_group_counts_only_its_changes() {
    let r = history();
    let out = stats_json(&r, &["--group", "operations"]);
    // alpha and gamma have the area `valves`.
    has(
        indicator(&out, "Time from first conversation"),
        &["\"value\":1.5", "\"count\":2"],
    );
    has(
        indicator(&out, "Survival rate"),
        &["\"value\":50.0", "\"count\":2"],
    );
    has(
        indicator(&out, "Elapsed time between"),
        &["\"value\":4.0", "\"count\":1"],
    );
    has(
        indicator(&out, "Time from plan approval"),
        &["\"value\":4.0", "\"count\":1"],
    );
    has(
        indicator(&out, "How often the merged diff"),
        &["\"value\":100.0", "\"count\":1"],
    );
    let (code, text, _) = r.run(&["stats", "--sdlc", "--group", "operations"]);
    assert_eq!(code, 0);
    assert!(
        text.starts_with(
            "2 changes in the history (group operations): 1 finished, 1 closed, 0 open\n"
        ),
        "{text}"
    );
}

#[test]
fn build_leading_reads_the_first_parent_commit_of_the_deletion() {
    // Deviation D29. The branch commit that deletes the folder has the date
    // 01-10. The merge commit reaches `main` on 01-20. The plan was approved
    // on 01-02, so the span is 18 days, not 8.
    let r = Repo::with_config(&config(ON));
    r.write(
        "docs/intent/dose/intent.md",
        &intent("Dose", "accepted", "routine", "valves"),
    );
    r.write(
        "docs/intent/dose/plan.md",
        &plan("approved", &["src/dose.py"]),
    );
    r.write("src/dose.py", "x = 1\n");
    r.commit_at("base", &at("01-02"));
    r.git(&["checkout", "-q", "-b", "feature"]);
    fs::remove_dir_all(r.dir.join("docs/intent/dose")).unwrap();
    r.write("src/dose.py", "x = 2\n");
    r.commit_at("finish on the branch", &at("01-10"));
    r.git(&["checkout", "-q", "main"]);
    r.write("src/other.py", "o = 1\n");
    r.commit_at("main side", &at("01-12"));
    git_at(
        &r.dir,
        &["merge", "-q", "--no-ff", "-m", "merge", "feature"],
        &at("01-20"),
    );
    let out = stats_json(&r, &[]);
    has(
        indicator(&out, "Time from plan approval"),
        &["\"value\":18.0", "\"count\":1"],
    );
    // The merge shows the whole diff of the branch, and it is in the plan.
    has(
        indicator(&out, "How often the merged diff"),
        &["\"value\":100.0", "\"count\":1"],
    );
    // `log` gives the commit of the branch, with its own date.
    let log = log_json(&r, &[]);
    assert!(log.contains("\"date\":\"2024-01-10\""), "{log}");
}

#[test]
fn a_generated_index_is_inside_the_scope() {
    // SD913 excludes a generated index. `finish` rewrites an index that lists
    // a deleted file, so the diff of a finished change can have one.
    let r = Repo::with_config(&config(ON));
    r.write(
        "docs/intent/dose/intent.md",
        &intent("Dose", "accepted", "routine", "valves"),
    );
    r.write(
        "docs/intent/dose/plan.md",
        &plan("approved", &["src/dose.py"]),
    );
    r.write(
        "docs/intent/README.md",
        "---\ngenerated: spec-dna index 123\n---\n# Index\n\n- [dose](dose/intent.md)\n",
    );
    r.write("src/dose.py", "x = 1\n");
    r.commit_at("base", &at("01-02"));
    fs::remove_dir_all(r.dir.join("docs/intent/dose")).unwrap();
    r.write(
        "docs/intent/README.md",
        "---\ngenerated: spec-dna index 456\n---\n# Index\n\nNothing.\n",
    );
    r.write("src/dose.py", "x = 2\n");
    r.commit_at("finish", &at("01-05"));
    let out = stats_json(&r, &[]);
    has(
        indicator(&out, "How often the merged diff"),
        &["\"value\":100.0", "\"count\":1"],
    );
}

#[test]
fn the_scope_of_a_commit_is_the_union_of_the_changes_that_it_finished() {
    // SD913 passes a path that any change of the diff covers. One commit
    // finishes 2 changes, and each of them lists a file of its own.
    let r = Repo::with_config(&config(ON));
    for (slug, file) in [("one", "src/one.py"), ("two", "src/two.py")] {
        r.write(
            &format!("docs/intent/{slug}/intent.md"),
            &intent(slug, "accepted", "routine", "valves"),
        );
        r.write(
            &format!("docs/intent/{slug}/plan.md"),
            &plan("approved", &[file]),
        );
        r.write(file, "x = 1\n");
    }
    r.commit_at("base", &at("01-02"));
    fs::remove_dir_all(r.dir.join("docs/intent")).unwrap();
    r.write("src/one.py", "x = 2\n");
    r.write("src/two.py", "x = 2\n");
    r.commit_at("finish both", &at("01-05"));
    let out = stats_json(&r, &[]);
    has(
        indicator(&out, "How often the merged diff"),
        &["\"value\":100.0", "\"count\":2"],
    );
}

fn golden(name: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing golden {name}; run with UPDATE_GOLDEN=1"));
    assert_eq!(
        actual, expected,
        "golden {name} differs; run with UPDATE_GOLDEN=1 to accept"
    );
}

#[test]
fn stats_text_matches_the_golden_file() {
    let r = history();
    let (code, out, err) = r.run(&["stats", "--sdlc"]);
    assert_eq!(code, 0, "{out}{err}");
    // The text has no commit id and no version, so the file is the same on
    // each run.
    assert!(!out.contains(VERSION), "{out}");
    assert!(
        out.lines().all(|l| l == l.trim_end()),
        "a line has a trailing space: {out}"
    );
    golden("stats-sdlc.txt", &out);
}
