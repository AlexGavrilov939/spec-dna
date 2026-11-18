//! Integration tests of SD923 (`task-incomplete`) and SD924 (`task-too-large`),
//! and of the key `[change] max-task-files` (design 0.7.0, D40). The checks
//! read the tasks of an `approved` plan. They read one tree: a task has a
//! `Files:` label with exact paths, and a `Verify:` label, and it lists no
//! more paths than the limit. A draft plan is not checked. A plan that a commit
//! added when `spec-dna.toml` pinned a version before 0.7.0 is skipped, with a
//! note (design 0.7.1, B5).

mod common;

use common::*;

fn config(profile: &str, change: &str) -> String {
    format!(
        "profile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\n{change}\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n"
    )
}

/// A repository with the folder `src` in its tree, so that `src` names a
/// folder.
fn repo_with(cfg: &str) -> Repo {
    let r = Repo::with_config(cfg);
    r.write("src/a.rs", "fn a() {}\n");
    r.git(&["add", "src/a.rs"]);
    r
}

/// The change `dose`: an intent and a plan with the status `plan` and the
/// task lines `order`. The plan lists no file in `Files`.
fn change(r: &Repo, intent: &str, plan: &str, order: &str) {
    r.write(
        "docs/intent/dose/intent.md",
        &format!("---\nstatus: {intent}\n---\n# Dose\n\n## Intent\n\nText.\n"),
    );
    r.write(
        "docs/intent/dose/plan.md",
        &format!(
            "---\nstatus: {plan}\n---\n# Plan\n\n## Files\n\n- none\n\n## Order\n\n{order}\n\n## Proof\n\n- A test.\n"
        ),
    );
}

fn check(r: &Repo) -> (i32, String) {
    let (code, out, _) = r.run(&["check", "--all"]);
    (code, out)
}

/// The lines of `out` for `code`.
fn lines<'a>(out: &'a str, code: &str) -> Vec<&'a str> {
    out.lines()
        .filter(|l| {
            l.contains(&format!(": error {code} ")) || l.contains(&format!(": warning {code} "))
        })
        .collect()
}

const PROFILES: [&str; 3] = ["lite", "standard", "strict"];

/// Asserts the severity of one code in each profile. `want` has the severity
/// for `lite`, `standard` and `strict` (`None`: no diagnostic). `order` is the
/// text of the section `Order` of an `approved` plan.
fn matrix(code: &str, want: [Option<&str>; 3], change_keys: &str, order: &str) {
    for (profile, want) in PROFILES.iter().zip(want) {
        let r = repo_with(&config(profile, change_keys));
        change(&r, "accepted", "approved", order);
        let (_, out) = check(&r);
        let got = lines(&out, code);
        match want {
            Some(w) => {
                assert_eq!(got.len(), 1, "{code} in {profile}: {out}");
                assert!(
                    got[0].contains(&format!(": {w} {code} ")),
                    "{code} in {profile}: {out}"
                );
            }
            None => assert!(got.is_empty(), "{code} in {profile}: {out}"),
        }
    }
}

fn never(code: &str, change_keys: &str, order: &str) {
    matrix(code, [None, None, None], change_keys, order);
}

const FULL: &str = "- [ ] t1: Write it. Files: `src/a.rs`. Verify: `cargo test`.";

// SD923

#[test]
fn a_task_with_no_files_label_gives_sd923() {
    matrix(
        "SD923",
        [None, Some("warning"), Some("error")],
        "",
        "- [ ] t1: Write it. Verify: `cargo test`.",
    );
}

#[test]
fn a_task_with_no_verify_label_gives_sd923() {
    matrix(
        "SD923",
        [None, Some("warning"), Some("error")],
        "",
        "- [ ] t1: Write it. Files: `src/a.rs`.",
    );
    // A label with no text is no label.
    matrix(
        "SD923",
        [None, Some("warning"), Some("error")],
        "",
        "- [ ] t1: Write it. Files: `src/a.rs`. Verify:",
    );
}

#[test]
fn a_skip_with_no_reason_gives_sd923() {
    for order in [
        "- [ ] t1: Write it. Files: `src/a.rs`. Verify: `cargo test`. Skip:",
        "- [x] t1: Write it. Files: `src/a.rs`. Verify: `cargo test`. Skip: .",
        "- [x] t1: Write it. Skip:   . Files: `src/a.rs`. Verify: `cargo test`.",
    ] {
        matrix("SD923", [None, Some("warning"), Some("error")], "", order);
    }
    let r = repo_with(&config("standard", ""));
    change(
        &r,
        "accepted",
        "approved",
        "- [ ] t1: Write it. Files: `src/a.rs`. Verify: `cargo test`. Skip:",
    );
    let (_, out) = check(&r);
    let got = lines(&out, "SD923");
    assert_eq!(got.len(), 1, "{out}");
    assert!(got[0].contains("its `Skip:` has no reason"), "{out}");
    assert!(got[0].contains("write the reason after `Skip:`"), "{out}");
    // The text of the other problems stays.
    assert!(
        !got[0].contains("no `Files:`") && !got[0].contains("no `Verify:`"),
        "{out}"
    );
    // A reason, and a `Hours:` label, are no problem.
    never(
        "SD923",
        "",
        "- [x] t1: Write it. Files: `src/a.rs`. Verify: `cargo test`. Hours: 1-2. Skip: the store keeps it.",
    );
    never(
        "SD923",
        "",
        "- [ ] t1: Write it. Files: `src/a.rs`. Verify: `cargo test`. Hours: 2.",
    );
    // The word `Skip:` in a code span is no label.
    never(
        "SD923",
        "",
        "- [ ] t1: Write `Skip:` it. Files: `src/a.rs`. Verify: `cargo test`.",
    );
}

#[test]
fn a_skipped_task_still_needs_files_and_verify() {
    let r = repo_with(&config("standard", ""));
    change(
        &r,
        "accepted",
        "approved",
        "- [x] t1: Write it. Skip: the store keeps it.",
    );
    let (_, out) = check(&r);
    let got = lines(&out, "SD923");
    assert_eq!(got.len(), 1, "{out}");
    assert!(
        got[0].contains("it has no `Files:`") && got[0].contains("it has no `Verify:`"),
        "{out}"
    );
    assert!(!got[0].contains("`Skip:` has no reason"), "{out}");
    // Both problems at once: one diagnostic, and both fixes.
    let r = repo_with(&config("standard", ""));
    change(&r, "accepted", "approved", "- [x] t1: Write it. Skip:");
    let (_, out) = check(&r);
    let got = lines(&out, "SD923");
    assert_eq!(got.len(), 1, "{out}");
    assert!(
        got[0].contains("it has no `Files:`; it has no `Verify:`; its `Skip:` has no reason"),
        "{out}"
    );
    assert!(
        got[0].contains("write the exact paths in `Files:` and the check in `Verify:`"),
        "{out}"
    );
    assert!(got[0].contains("write the reason after `Skip:`"), "{out}");
}

#[test]
fn a_task_that_misses_both_labels_gives_one_diagnostic_with_both_reasons() {
    let r = repo_with(&config("standard", ""));
    change(&r, "accepted", "approved", "- [ ] t1: Write it.");
    let (code, out) = check(&r);
    // A warning never fails `check`.
    assert_eq!(code, 0, "{out}");
    let got = lines(&out, "SD923");
    assert_eq!(got.len(), 1, "{out}");
    assert!(
        got[0].starts_with("docs/intent/dose/plan.md:"),
        "{}",
        got[0]
    );
    assert!(got[0].contains("the task `t1`"), "{}", got[0]);
    assert!(got[0].contains("no `Files:`"), "{}", got[0]);
    assert!(got[0].contains("no `Verify:`"), "{}", got[0]);
}

#[test]
fn a_complete_task_gives_no_sd923_or_sd924() {
    never("SD923", "", FULL);
    never("SD924", "", FULL);
    // Three paths and a command.
    never(
        "SD924",
        "",
        "- [ ] t1: Write it. Files: `src/a.rs`, `src/b.rs`, `src/c.rs`. Verify: `cargo test`.",
    );
    // A task with plain paths and a done box.
    never(
        "SD923",
        "",
        "- [x] t1: Write it. Files: src/a.rs, src/b.rs. Verify: run the tests.",
    );
}

#[test]
fn files_none_is_a_complete_label() {
    never(
        "SD923",
        "",
        "- [ ] t1: Read it. Files: none. Verify: `cargo test`.",
    );
    never(
        "SD923",
        "",
        "- [ ] t1: Read it. Files: None: research only. Verify: read the note.",
    );
    // Prose that is not `none` and gives no path is not a list of paths.
    matrix(
        "SD923",
        [None, Some("warning"), Some("error")],
        "",
        "- [ ] t1: Read it. Files: the parser. Verify: `cargo test`.",
    );
}

/// The task of the review by the owner, as the skill `write-plan` gives it,
/// has the labels that SD923 needs, so the plan passes after its approval.
#[test]
fn the_review_task_of_the_write_plan_skill_is_complete() {
    let (code, out, err) = Repo::new().run(&["skill", "show", "write-plan"]);
    assert_eq!(code, 0, "{out}{err}");
    let example = out
        .split('`')
        .find(|s| s.contains("Review by the owner"))
        .unwrap_or_else(|| panic!("no review task in the skill\n{out}"));
    let task = example.replace("tN", "t2").replace("<n>", "1");
    assert!(
        task.starts_with("- [ ] t2: Review by the owner. "),
        "{task}"
    );
    assert!(task.contains("Files: none."), "{task}");
    let order = format!("- [ ] t1: Read it. Files: none. Verify: `cargo test`.\n{task}");
    never("SD923", "", &order);
}

#[test]
fn a_glob_or_a_folder_in_files_gives_sd923() {
    let cases = [
        ("`src/*.rs`", "the glob `src/*.rs`"),
        ("`src/**/a.rs`", "the glob `src/**/a.rs`"),
        ("`src/a?.rs`", "the glob `src/a?.rs`"),
        ("`docs/guides/`", "the folder `docs/guides/`"),
        // The tree has the folder `src`, so it is a folder with no final `/`.
        ("`src`", "the folder `src`"),
        ("src/*.rs", "the glob `src/*.rs`"),
    ];
    for (files, reason) in cases {
        let r = repo_with(&config("standard", ""));
        change(
            &r,
            "accepted",
            "approved",
            &format!("- [ ] t1: Write it. Files: {files}. Verify: `cargo test`."),
        );
        let (_, out) = check(&r);
        let got = lines(&out, "SD923");
        assert_eq!(got.len(), 1, "{files}: {out}");
        assert!(got[0].contains(reason), "{files}: {}", got[0]);
    }
    // A path that is a file is exact, also if a folder has the same start.
    never(
        "SD923",
        "",
        "- [ ] t1: Write it. Files: `src/a.rs`. Verify: `cargo test`.",
    );
}

#[test]
fn each_task_of_the_plan_is_checked_and_the_target_is_its_id() {
    let r = repo_with(&config("standard", ""));
    change(
        &r,
        "accepted",
        "approved",
        "- [ ] t1: One. Files: `src/a.rs`. Verify: `cargo test`.\n- [ ] t2: Two. Files: `src/a.rs`.\n- [ ] t3: Three. Verify: `cargo test`.",
    );
    let (_, out) = check(&r);
    let got = lines(&out, "SD923");
    assert_eq!(got.len(), 2, "{out}");
    assert!(got[0].contains("the task `t2`"), "{out}");
    assert!(got[1].contains("the task `t3`"), "{out}");
    let (_, json, _) = r.run(&["check", "--all", "--json"]);
    assert!(json.contains("\"target\":\"t2\""), "{json}");
    assert!(json.contains("\"target\":\"t3\""), "{json}");
}

#[test]
fn an_item_that_is_not_a_task_is_not_checked() {
    // No task box, or no id: the item is not a task of the plan.
    never("SD923", "", "- Write the code.\n- [ ] Write the tests.");
}

#[test]
fn a_task_id_that_an_earlier_task_has_is_checked_once() {
    let r = repo_with(&config("standard", ""));
    change(&r, "accepted", "approved", "- [ ] t1: One.\n- [ ] t1: Two.");
    let (_, out) = check(&r);
    assert_eq!(lines(&out, "SD923").len(), 1, "{out}");
}

#[test]
fn a_draft_plan_is_not_checked() {
    // 0.6.1 plans are drafts or approved: only an `approved` plan has tasks
    // that must hold the labels.
    for order in [
        "- [ ] t1: Write it.",
        "- [ ] t1: Write it. Files: `src/*`. Verify: `cargo test`.",
        "- [ ] t1: Write it. Files: `src/a.rs`, `src/b.rs`, `src/c.rs`, `src/d.rs`. Verify: x.",
    ] {
        for profile in PROFILES {
            let r = repo_with(&config(profile, ""));
            change(&r, "accepted", "draft", order);
            let (code, out) = check(&r);
            assert_eq!(code, 0, "{profile}: {out}");
            assert!(lines(&out, "SD923").is_empty(), "{profile}: {out}");
            assert!(lines(&out, "SD924").is_empty(), "{profile}: {out}");
        }
    }
}

#[test]
fn a_closed_change_is_not_checked() {
    let r = repo_with(&config("strict", ""));
    change(&r, "closed", "approved", "- [ ] t1: Write it.");
    let (_, out) = check(&r);
    assert!(lines(&out, "SD923").is_empty(), "{out}");
}

#[test]
fn the_plan_of_an_intent_that_is_not_accepted_is_checked() {
    // The check reads the plan, not the stage of the intent.
    let r = repo_with(&config("standard", ""));
    change(&r, "draft", "approved", "- [ ] t1: Write it.");
    let (_, out) = check(&r);
    assert_eq!(lines(&out, "SD923").len(), 1, "{out}");
}

#[test]
fn a_new_error_of_strict_fails_check_and_a_warning_does_not() {
    let r = repo_with(&config("strict", ""));
    change(&r, "accepted", "approved", "- [ ] t1: Write it.");
    let (code, out) = check(&r);
    assert_eq!(code, 1, "{out}");
    let r = repo_with(&config("standard", ""));
    change(&r, "accepted", "approved", "- [ ] t1: Write it.");
    let (code, out) = check(&r);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn the_policy_none_and_a_rule_set_the_severity() {
    let none = "profile = \"strict\"\n[change]\npolicy = \"none\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\n";
    let r = repo_with(none);
    change(&r, "accepted", "approved", "- [ ] t1: Write it.");
    let (_, out) = check(&r);
    assert!(lines(&out, "SD923").is_empty(), "{out}");
    // A rule of the file changes the severity of one code.
    let cfg =
        config("standard", "").replace("SD608 = \"off\"", "SD608 = \"off\"\nSD923 = \"error\"");
    let r = repo_with(&cfg);
    change(&r, "accepted", "approved", "- [ ] t1: Write it.");
    let (code, out) = check(&r);
    assert_eq!(code, 1, "{out}");
    assert!(lines(&out, "SD923")[0].contains(": error SD923 "), "{out}");
    // The group `changes` sets the 3 new codes too.
    let cfg =
        config("lite", "").replace("SD608 = \"off\"", "SD608 = \"off\"\nchanges = \"warning\"");
    let r = repo_with(&cfg);
    change(&r, "accepted", "approved", "- [ ] t1: Write it.");
    let (_, out) = check(&r);
    assert_eq!(lines(&out, "SD923").len(), 1, "{out}");
}

// SD924

const FOUR: &str = "- [ ] t1: Write it. Files: `src/a.rs`, `src/b.rs`, `src/c.rs`, `src/d.rs`. Verify: `cargo test`.";

#[test]
fn a_task_with_more_than_3_paths_gives_sd924() {
    matrix("SD924", [None, Some("warning"), Some("error")], "", FOUR);
    let r = repo_with(&config("standard", ""));
    change(&r, "accepted", "approved", FOUR);
    let (_, out) = check(&r);
    let got = lines(&out, "SD924");
    assert!(got[0].contains("the task `t1`"), "{}", got[0]);
    assert!(
        got[0].contains("lists 4 paths, more than `max-task-files = 3`"),
        "{}",
        got[0]
    );
    // The task gives no SD923: the labels are there.
    assert!(lines(&out, "SD923").is_empty(), "{out}");
}

#[test]
fn a_path_that_a_task_lists_twice_counts_once() {
    never(
        "SD924",
        "",
        "- [ ] t1: Write it. Files: `src/a.rs`, `src/b.rs`, `src/a.rs`, `src/c.rs`. Verify: `cargo test`.",
    );
}

#[test]
fn max_task_files_sets_the_limit() {
    never("SD924", "max-task-files = 4\n", FOUR);
    never("SD924", "max-task-files = 20\n", FOUR);
    matrix(
        "SD924",
        [None, Some("warning"), Some("error")],
        "max-task-files = 1\n",
        "- [ ] t1: Write it. Files: `src/a.rs`, `src/b.rs`. Verify: `cargo test`.",
    );
    never(
        "SD924",
        "max-task-files = 1\n",
        "- [ ] t1: Write it. Files: `src/a.rs`. Verify: `cargo test`.",
    );
}

#[test]
fn max_task_files_takes_an_integer_from_1_to_20() {
    for bad in ["0", "21", "-1", "\"3\"", "1.5", "true"] {
        let r = repo_with(&config("standard", &format!("max-task-files = {bad}\n")));
        let (code, _, err) = r.run(&["check", "--all"]);
        assert_eq!(code, 3, "{bad}: {err}");
        assert!(
            err.contains("`change.max-task-files` must be an integer from 1 to 20"),
            "{bad}: {err}"
        );
    }
    for good in ["1", "3", "20"] {
        let r = repo_with(&config("standard", &format!("max-task-files = {good}\n")));
        let (code, out, err) = r.run(&["check", "--all"]);
        assert_eq!(code, 0, "{good}: {out}{err}");
    }
}

#[test]
fn a_draft_plan_gives_no_sd924() {
    let r = repo_with(&config("strict", ""));
    change(&r, "accepted", "draft", FOUR);
    let (code, out) = check(&r);
    assert_eq!(code, 0, "{out}");
}

// The fingerprint of the gate

#[test]
fn only_a_new_task_diagnostic_fails_the_gate() {
    // The base has an approved plan with an incomplete task. A branch that
    // adds another incomplete task gives one new diagnostic, and the old one
    // is not new.
    let r = repo_with(&config("strict", ""));
    change(&r, "accepted", "approved", "- [ ] t1: Write it.");
    r.commit("base");
    r.git(&["checkout", "-q", "-b", "feature"]);
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(code, 0, "no new diagnostic: {out}");
    change(
        &r,
        "accepted",
        "approved",
        "- [ ] t1: Write it.\n- [ ] t2: Test it.",
    );
    r.commit("a task");
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(code, 1, "{out}");
    let got = lines(&out, "SD923");
    assert_eq!(got.len(), 1, "{out}");
    assert!(got[0].contains("the task `t2`"), "{out}");
}

// A `[` in a path

#[test]
fn a_task_path_with_brackets_is_an_exact_path() {
    // `[` and `]` are plain characters: the scope, SD910 and SD913 read them
    // so, and a common web framework has such folders. A glob has `*` or `?`
    // only.
    for files in ["`app/[id]/page.tsx`", "`src/[ab].rs`", "app/[id]/page.tsx"] {
        never(
            "SD923",
            "",
            &format!("- [ ] t1: Write it. Files: {files}. Verify: `cargo test`."),
        );
    }
}

#[test]
fn a_files_path_with_brackets_matches_the_same_path() {
    // Guard: the base tree approves the plan, and the head edits the file of
    // the path with brackets. The scope reads the path as it is, so no gate
    // reports the edit.
    let r = repo_with(&config("strict", ""));
    r.write("app/[id]/page.tsx", "export {};\n");
    r.write(
        "docs/intent/dose/intent.md",
        "---\nstatus: accepted\n---\n# Dose\n\n## Intent\n\nText.\n",
    );
    r.write(
        "docs/intent/dose/plan.md",
        "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `app/[id]/page.tsx`\n\n## Order\n\n- [ ] t1: Write it. Files: `app/[id]/page.tsx`. Verify: `cargo test`.\n\n## Proof\n\n- A test.\n",
    );
    r.commit("base");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("app/[id]/page.tsx", "export const a = 1;\n");
    r.commit("edit");
    let (code, out, err) = r.run(&["check", "--base", "main"]);
    for c in ["SD910", "SD913", "SD923"] {
        assert!(lines(&out, c).is_empty(), "{c}: {out}{err}");
    }
    assert_eq!(code, 0, "{out}{err}");
}

// A plan that was added under a pin before 0.7.0 (design 0.7.1, B5)

const PLAN_PATH: &str = "docs/intent/dose/plan.md";

/// The task lines of an early plan: `t1` has a folder in `Files:`, and `t2`
/// has no `Verify:`. Both give a finding.
const EARLY_ORDER: &str =
    "- [ ] t1: Write it. Files: `src`. Verify: `cargo test`.\n- [ ] t2: Test it. Files: none.";

/// The note of the plan `docs/intent/dose/plan.md` that `pin` marks, with `n`
/// tasks.
fn early_note(pin: &str, n: usize) -> String {
    format!(
        "note: `{PLAN_PATH}` was added when spec-dna.toml pinned {pin}, before 0.7.0, so SD923 and SD924 skip its {n} task(s) with a finding; to check them, give each task exact paths in `Files:` and one `Verify:`"
    )
}

/// A repository whose first commit pins `pin` and adds the `approved` plan
/// with the tasks `order`. The second commit moves the pin to the version of
/// the binary.
fn early_plan(profile: &str, pin: &str, order: &str) -> Repo {
    let r = Repo::new();
    let toml = config(profile, "");
    r.write("spec-dna.toml", &format!("spec-dna = \"{pin}\"\n{toml}"));
    r.write("src/a.rs", "fn a() {}\n");
    change(&r, "accepted", "approved", order);
    r.commit("add the plan");
    r.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n{toml}"),
    );
    // With the pin 0.7.0 the commit changes nothing.
    r.git(&["add", "-A"]);
    r.git(&["commit", "-q", "--allow-empty", "-m", "move the pin"]);
    r
}

#[test]
fn a_plan_added_under_an_earlier_pin_gives_no_sd923_and_one_note() {
    let r = early_plan("standard", "0.6.1", EARLY_ORDER);
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(lines(&out, "SD923").is_empty(), "{out}");
    assert!(lines(&out, "SD924").is_empty(), "{out}");
    let note = early_note("0.6.1", 2);
    assert_eq!(err.matches(&note).count(), 1, "{err}");
    // `--json` has the note on stderr only, and no diagnostic.
    let (_, out, err) = r.run(&["check", "--all", "--json"]);
    assert!(
        !out.contains("SD923") && !out.contains("was added"),
        "{out}"
    );
    assert_eq!(err.matches(&note).count(), 1, "{err}");
}

#[test]
fn an_earlier_plan_that_a_later_commit_edits_is_still_skipped() {
    // The lookup reads the commit that added the plan, not the last commit
    // that touched it: a tick of `t1` after the move of the pin changes nothing.
    let r = early_plan("standard", "0.6.1", EARLY_ORDER);
    let ticked = EARLY_ORDER.replacen("- [ ] t1", "- [x] t1", 1);
    change(&r, "accepted", "approved", &ticked);
    r.commit("tick t1");
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(lines(&out, "SD923").is_empty(), "{out}");
    assert!(err.contains(&early_note("0.6.1", 2)), "{err}");
}

#[test]
fn an_earlier_plan_counts_a_task_once_for_both_codes() {
    // `t1` gives SD923 (a folder) and SD924 (4 paths): one task.
    let order = "- [ ] t1: Write it. Files: `src`, `a`, `b`, `c`. Verify: `cargo test`.";
    let r = early_plan("standard", "0.6.1", order);
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(lines(&out, "SD923").is_empty() && lines(&out, "SD924").is_empty());
    assert!(err.contains(&early_note("0.6.1", 1)), "{err}");
}

#[test]
fn the_same_plan_added_after_the_move_of_the_pin_keeps_sd923() {
    let r = Repo::new();
    let toml = config("strict", "");
    r.write("spec-dna.toml", &format!("spec-dna = \"0.6.1\"\n{toml}"));
    r.write("src/a.rs", "fn a() {}\n");
    r.commit("start");
    r.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n{toml}"),
    );
    r.commit("move the pin");
    change(&r, "accepted", "approved", EARLY_ORDER);
    r.commit("add the plan");
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 1, "{out}{err}");
    let got = lines(&out, "SD923");
    assert_eq!(got.len(), 2, "{out}");
    assert!(got.iter().all(|l| l.contains(": error SD923 ")), "{out}");
    assert!(!err.contains("was added when"), "{err}");
}

#[test]
fn an_earlier_plan_in_strict_gives_no_error() {
    let r = early_plan("strict", "0.6.1", EARLY_ORDER);
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(lines(&out, "SD923").is_empty(), "{out}");
}

#[test]
fn a_plan_that_no_commit_added_is_the_current_form() {
    // The plan is in the working tree only: no commit adds it.
    let r = Repo::new();
    let toml = config("strict", "");
    r.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n{toml}"),
    );
    r.write("src/a.rs", "fn a() {}\n");
    r.commit("start");
    change(&r, "accepted", "approved", EARLY_ORDER);
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 1, "{out}{err}");
    assert_eq!(lines(&out, "SD923").len(), 2, "{out}");
    assert!(!err.contains("was added when"), "{err}");
}

#[test]
fn a_pin_of_0_7_0_or_later_at_the_add_is_the_current_form() {
    // Guard: the pin 0.7.0 at the add commit is not before 0.7.0.
    let r = early_plan("standard", "0.7.0", EARLY_ORDER);
    let (_, out, err) = r.run(&["check", "--all"]);
    assert_eq!(lines(&out, "SD923").len(), 2, "{out}");
    assert!(!err.contains("was added when"), "{err}");
}

#[test]
fn no_note_without_a_diagnostic_that_would_show() {
    // The finding is old: without `--all`, nothing shows, so there is no note.
    let r = early_plan("standard", "0.6.1", EARLY_ORDER);
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!err.contains("was added when"), "{err}");
    // A plan with no finding gives no note with `--all` either.
    let r = early_plan("standard", "0.6.1", FULL);
    let (_, out, err) = r.run(&["check", "--all"]);
    assert!(lines(&out, "SD923").is_empty(), "{out}");
    assert!(!err.contains("was added when"), "{err}");
}

#[test]
fn a_shallow_clone_finds_no_commit_and_keeps_sd923() {
    // The boundary commit of a shallow clone shows each file as added, with
    // the pin 0.6.1. The lookup must not read it as the add commit.
    let r = Repo::new();
    let toml = config("standard", "");
    r.write("spec-dna.toml", &format!("spec-dna = \"0.6.1\"\n{toml}"));
    r.write("src/a.rs", "fn a() {}\n");
    r.commit("start");
    change(&r, "accepted", "approved", EARLY_ORDER);
    r.commit("add the plan");
    r.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n{toml}"),
    );
    r.commit("move the pin");
    // The full history gives the note.
    let (_, out, err) = r.run(&["check", "--all"]);
    assert!(lines(&out, "SD923").is_empty(), "{out}");
    assert!(err.contains("was added when"), "{err}");
    let dst = Repo::empty();
    let clone = dst.dir.join("clone");
    shallow_clone(&r.dir, &clone, 2);
    let out = std::process::Command::new(BIN)
        .args(["check", "--all"])
        .current_dir(&clone)
        .output()
        .expect("binary runs");
    let (out, err) = (
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert_eq!(lines(&out, "SD923").len(), 2, "{out}{err}");
    assert!(!err.contains("was added when"), "{err}");
}
