//! Tests of the stop rule (design 0.8.0, D74): the count of rejected records of a
//! change, counted from the commits of the branch, in "lives" of a record; the
//! gate SD941 with `[change] approval = "branch"`; the stop of `next`; the line
//! `Acceptance runs:` of `finish`; and the rule that no edit clears a rejection.
//! The fixtures use example names only.

mod common;

use common::*;

const SLUG: &str = "valve-timeout";
const INTENT: &str = "docs/intent/valve-timeout/intent.md";
const SPEC: &str = "docs/intent/valve-timeout/spec.md";
const PLAN: &str = "docs/intent/valve-timeout/plan.md";
const RECORD: &str = "docs/intent/valve-timeout/acceptance.toml";
const CODE: &str = "src/valves.py";
const TEST: &str = "tests/test_valves.py";
const REQ: &str = "docs/requirements/valves.md";

const VALVES: &str = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **overflow**: When a bed overflows, the controller SHALL close the valve of that bed.\n";
const CODE_OLD: &str = "def close_valve(bed):\n    bed.valve_open = False\n";
const CODE_NEW: &str = "def close_valve(bed):\n    bed.valve_open = False\n\n\ndef on_sensor_timeout(bed):\n    close_valve(bed)\n";
const TEST_OLD: &str = "# covers: requirements/valves#overflow\ndef test_overflow():\n    ...\n";
const TEST_NEW: &str = "# covers: requirements/valves#overflow\ndef test_overflow():\n    ...\n\n\n# covers: requirements/valves#sensor-timeout\ndef test_sensor_timeout():\n    ...\n";

const INTENT_TEXT: &str = "---\nstatus: accepted\nclass: routine\n---\n# Valve timeout\n\n## Intent\n\nClose the valve when a sensor times out.\n\n## Why now\n\nA valve stayed open.\n\n## Out of scope\n\n- An alarm.\n";
const SPEC_TEXT: &str = "---\nstatus: approved\n---\n# Valve timeout\n\n## Delta\n\n- ADD requirements/valves#sensor-timeout: When a sensor gives no value for 60 seconds, the controller SHALL close the valve of its bed.\n\n## Interfaces\n\nNone.\n\n## Decisions\n\nNone.\n\n## Concerns\n\n- [x] A slow sensor. Resolution: 60 seconds.\n\n## Open questions\n\n- [x] Does the valve reopen? Answer: no.\n";
const PLAN_TEXT: &str = "---\nstatus: approved\n---\n# Valve timeout\n\n## Files\n\n- `src/valves.py`\n- `tests/test_valves.py`\n\n## Order\n\n- [x] t1: Close the valve. Files: `src/valves.py`. Verify: the test passes.\n\n## Risks\n\n- A slow sensor.\n\n## Proof\n\nThe test covers `requirements/valves#sensor-timeout`.\n";

/// The configuration of a test. The tests of the record mechanics (the checks, the
/// compare of the brief, the archive file) need a record for each change, so the
/// fixture sets `acceptance-for = "all"` unless the test sets the key itself. The
/// rule of the need (D70) has its own tests, in `tests/acceptance_need.rs`.
fn config_with(profile: &str, change: &str, rules: &str) -> String {
    let all = if change.contains("acceptance-for") {
        ""
    } else {
        "acceptance-for = \"all\"\n"
    };
    format!(
        "profile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n{all}{change}\n[rules]\nSD608 = \"off\"\n{rules}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n"
    )
}

fn config(profile: &str, change: &str) -> String {
    config_with(profile, change, "")
}

fn base_files(r: &Repo) {
    r.write(REQ, VALVES);
    r.write(CODE, CODE_OLD);
    r.write(TEST, TEST_OLD);
    r.write("src/other.py", "y = 1\n");
    r.write("README.md", "# Project\n");
}

fn change_files(r: &Repo, intent: &str) {
    r.write(INTENT, intent);
    r.write(SPEC, SPEC_TEXT);
    r.write(PLAN, PLAN_TEXT);
}

fn pin_line(pin: &str, config_text: &str) -> String {
    format!("spec-dna = \"{pin}\"\n{config_text}")
}

fn add_code(r: &Repo) {
    r.write(CODE, CODE_NEW);
    r.write(TEST, TEST_NEW);
    r.commit("code");
}

/// A repository where a commit of `main` added the plan with the pin `pin`, and
/// a later commit sets the pin to the version of the binary. The branch adds the
/// code. The plan is on the base.
fn on_base(pin: &str, config_text: &str, intent: &str) -> Repo {
    let r = Repo::new();
    r.write("spec-dna.toml", &pin_line(pin, config_text));
    base_files(&r);
    change_files(&r, intent);
    r.commit("plan");
    r.write("spec-dna.toml", &pin_line(VERSION, config_text));
    // The commit always changes a file, also when the pin is the version.
    r.write("README.md", "# Project, with the new pin\n");
    r.commit("move the pin");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    add_code(&r);
    r
}

/// A plan on the base that needs a record.
fn base_flow() -> Repo {
    on_base("0.8.0", &config("standard", ""), INTENT_TEXT)
}

fn run(r: &Repo, args: &[&str]) -> (i32, String, String) {
    r.run(args)
}

fn has(out: &str, code: &str) -> bool {
    out.lines().any(|l| l.contains(&format!(" {code} ")))
}

fn last_line(text: &str) -> &str {
    text.lines().last().unwrap()
}

fn accept(r: &Repo, args: &[&str]) -> (i32, String, String) {
    let mut a = vec!["accept", SLUG];
    a.extend_from_slice(args);
    r.run(&a)
}

/// The hash on the last line of the brief of the change, or of its follow-up.
fn hash_of(r: &Repo, follow_up: bool) -> String {
    let args: &[&str] = if follow_up {
        &["--follow-up", "--brief"]
    } else {
        &["--brief"]
    };
    let (code, out, err) = accept(r, args);
    assert_eq!(code, 0, "{out}{err}");
    last_line(&out).strip_prefix("brief ").unwrap().to_string()
}

const ATTEMPT: &str = "\n[[attempt]]\nacceptor = 1\ntext = \"Sent a NaN value after the timeout.\"\noutcome = \"held\"\n";

/// The committed record, with one finding for each `(path, severity, text)`. The
/// top result is `accepted` when no finding is a blocker.
fn commit_record(r: &Repo, findings: &[(&str, &str, &str)]) -> String {
    let (code, out, err) = accept(r, &["--session", "s1"]);
    assert_eq!(code, 0, "{out}{err}");
    let hash = hash_of(r, false);
    let mut t = r
        .read(RECORD)
        .replacen("brief = \"pending\"", &format!("brief = \"{hash}\""), 1);
    let rejected = findings.iter().any(|f| f.1 == "blocker");
    t = t.replace(
        "result = \"pending\"",
        if rejected {
            "result = \"rejected\""
        } else {
            "result = \"accepted\""
        },
    );
    t.push_str(ATTEMPT);
    for (path, severity, text) in findings {
        t.push_str(&format!(
            "\n[[finding]]\nacceptor = 1\nseverity = \"{severity}\"\npath = \"{path}\"\nline = 3\ntext = \"{text}\"\n"
        ));
    }
    r.write(RECORD, &t);
    r.commit("record");
    t
}

/// Appends `extra` to a file and commits it as the fix of a note.
fn fix(r: &Repo, path: &str, extra: &str) {
    let t = r.read(path);
    r.write(path, &format!("{t}{extra}"));
    r.commit("fix");
}

/// The follow-up as an agent writes it: the brief first, then `accept --follow-up`,
/// then the fields. `closes` replaces the list that the command writes. The
/// record is committed alone.
fn commit_follow_up(r: &Repo, session: &str, closes: Option<&str>) {
    let hash = hash_of(r, true);
    let (code, out, err) = accept(r, &["--follow-up", "--session", session]);
    assert_eq!(code, 0, "{out}{err}");
    let mut t = r
        .read(RECORD)
        .replacen("brief = \"pending\"", &format!("brief = \"{hash}\""), 1)
        .replacen("result = \"pending\"", "result = \"accepted\"", 1)
        .replacen(
            "text = \"pending\"",
            "text = \"The name has a unit now. Proof: the test passed.\"",
            1,
        );
    if let Some(c) = closes {
        let at = t.rfind("closes = ").unwrap();
        let end = at + t[at..].find('\n').unwrap();
        t.replace_range(at..end, &format!("closes = {c}"));
    }
    r.write(RECORD, &t);
    r.commit("follow-up");
}

const NOTE_TEXT: &str = "The constant has no unit.";

const APPROVER: &str = "approver@example.com";
const STRANGER: &str = "intruder@example.com";

const BLOCKER: (&str, &str, &str) = (CODE, "blocker", "The timeout is not checked.");

fn branch_keys() -> String {
    format!("approval = \"branch\"\napprovers = [\"{APPROVER}\"]\n")
}

/// The plan is on the base and approved there, so the work needs no approval
/// commit. The branch mode counts the approval commits of the plan.
fn branch_flow() -> Repo {
    on_base("0.8.0", &config("standard", &branch_keys()), INTENT_TEXT)
}

/// A record that says `rejected`, committed.
fn rejected_record(r: &Repo) {
    commit_record(r, &[BLOCKER]);
}

/// Deletes the record in a commit of its own: the life of the record ends.
fn delete_record(r: &Repo) {
    r.git(&["rm", "-q", RECORD]);
    r.commit("delete the record");
}

/// One round of the acceptance that ends in a deleted record.
fn round_rejected_and_deleted(r: &Repo) {
    rejected_record(r);
    delete_record(r);
}

/// The text of the last item of `next`, and `next` as it is.
fn next_text(r: &Repo) -> String {
    let (code, out, err) = run(r, &["next", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    out
}

fn accept_item(out: &str) -> String {
    out.lines()
        .find(|l| l.ends_with("[skill: review]") || l.contains("wait for the owner"))
        .unwrap_or_else(|| panic!("no accept item in\n{out}"))
        .to_string()
}

const WAIT: &str = "wait for the owner: 2 acceptance records of valve-timeout say `rejected` (";

/// `plan.md` as a draft, then the approval commit of an approver.
fn approve_plan_again(r: &Repo, approval_author: &str) {
    let plan = r.read(PLAN);
    r.write(PLAN, &plan.replace("status: approved", "status: draft"));
    r.commit("plan back to draft");
    r.write(PLAN, &plan);
    r.commit_as("approve the plan", approval_author);
}

// `next`: the item of a person.

#[test]
fn two_rejected_records_each_committed_and_deleted_stop_next_with_the_item_of_a_person() {
    let r = base_flow();
    round_rejected_and_deleted(&r);
    // One rejected record: the item is the item `accept` of before.
    let one = next_text(&r);
    assert!(
        accept_item(&one).contains("start one agent") && !one.contains("[person]"),
        "{one}"
    );
    round_rejected_and_deleted(&r);
    let out = next_text(&r);
    let item = accept_item(&out);
    assert!(item.contains(WAIT), "{item}");
    assert!(
        item.contains("); a third run needs the word of the owner"),
        "{item}"
    );
    assert!(item.ends_with("[person]"), "{item}");
    assert!(
        !out.contains("accept valve-timeout --brief"),
        "no prompt: {out}"
    );
    // The 2 commits are the commits that held the records, 7 characters each.
    let commits = item.split(WAIT).nth(1).unwrap().split(')').next().unwrap();
    assert_eq!(commits.split(", ").count(), 2, "{item}");
    assert!(commits.split(", ").all(|c| c.len() == 7), "{item}");
    let (code, json, err) = run(&r, &["next", SLUG, "--json"]);
    assert_eq!(code, 0, "{json}{err}");
    assert!(
        json.contains("\"person\":true") && json.contains("wait for the owner"),
        "{json}"
    );
}

#[test]
fn with_branch_the_item_adds_the_way_to_the_word_of_the_owner() {
    let r = branch_flow();
    round_rejected_and_deleted(&r);
    round_rejected_and_deleted(&r);
    let item = accept_item(&next_text(&r));
    assert!(item.contains(WAIT), "{item}");
    assert!(
        item.contains(": set `status: draft` in plan.md, then the approver runs `spec-dna approve valve-timeout plan`"),
        "{item}"
    );
}

#[test]
fn an_approval_of_the_plan_after_the_second_record_gives_the_count_0_again() {
    let r = branch_flow();
    round_rejected_and_deleted(&r);
    round_rejected_and_deleted(&r);
    approve_plan_again(&r, APPROVER);
    let item = accept_item(&next_text(&r));
    assert!(item.contains("start one agent"), "{item}");
    // The approval of a person who is no approver does not count.
    let r = branch_flow();
    round_rejected_and_deleted(&r);
    round_rejected_and_deleted(&r);
    approve_plan_again(&r, STRANGER);
    assert!(accept_item(&next_text(&r)).contains(WAIT));
    // An approval before the second record does not count either.
    let r = branch_flow();
    round_rejected_and_deleted(&r);
    approve_plan_again(&r, APPROVER);
    round_rejected_and_deleted(&r);
    assert!(accept_item(&next_text(&r)).contains("start one agent"));
}

// The count: lives of a record.

#[test]
fn a_rejected_record_whose_findings_change_in_2_commits_counts_once() {
    let r = base_flow();
    rejected_record(&r);
    for n in 1..=2 {
        let t = r.read(RECORD);
        r.write(
            RECORD,
            &format!("{t}\n[[finding]]\nacceptor = 1\nseverity = \"note\"\npath = \"{CODE}\"\ntext = \"Edit {n}.\"\n"),
        );
        r.commit("edit the findings");
    }
    let item = accept_item(&next_text(&r));
    assert!(!item.contains(WAIT) && item.contains("rejected"), "{item}");
}

#[test]
fn a_second_acceptor_in_a_later_commit_while_the_blocker_stays_counts_once() {
    let r = base_flow();
    rejected_record(&r);
    let (code, out, err) = run(&r, &["accept", SLUG, "--session", "s2"]);
    assert_eq!(code, 0, "{out}{err}");
    // The new entry is fully written, as an acceptor writes it.
    let hash = hash_of(&r, false);
    let t = r
        .read(RECORD)
        .replacen("brief = \"pending\"", &format!("brief = \"{hash}\""), 1)
        .replacen("result = \"pending\"", "result = \"rejected\"", 1);
    r.write(
        RECORD,
        &format!("{t}{}", ATTEMPT.replace("acceptor = 1", "acceptor = 2")),
    );
    r.commit("the second acceptor");
    delete_record(&r);
    let item = accept_item(&next_text(&r));
    assert!(
        !item.contains(WAIT),
        "one life, one rejected record: {item}"
    );
    round_rejected_and_deleted(&r);
    assert!(accept_item(&next_text(&r)).contains(WAIT));
}

#[test]
fn a_record_that_says_accepted_with_notes_and_a_follow_up_are_no_rejected_record() {
    let r = base_flow();
    // 2 accepted records with a note each, with a follow-up in the first.
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    fix(&r, CODE, "\n# the unit is seconds\n");
    commit_follow_up(&r, "s2", None);
    delete_record(&r);
    r.write(CODE, &format!("{}\n# more\n", r.read(CODE)));
    r.commit("more code");
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    delete_record(&r);
    let out = next_text(&r);
    assert!(!out.contains("wait for the owner"), "{out}");
    // A third life that says `rejected`: still 1.
    rejected_record(&r);
    delete_record(&r);
    assert!(!next_text(&r).contains("wait for the owner"));
}

#[test]
fn a_follow_up_that_says_rejected_makes_its_life_count_once() {
    let r = base_flow();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    fix(&r, CODE, "\n# the unit is seconds\n");
    commit_follow_up(&r, "s2", None);
    edit_record(&r, |t| {
        t.replacen(
            "result = \"accepted\"\ntext",
            "result = \"rejected\"\ntext",
            1,
        )
        .replacen(
            "result = \"accepted\"\nacknowledged",
            "result = \"rejected\"\nacknowledged",
            1,
        )
    });
    delete_record(&r);
    assert!(!next_text(&r).contains("wait for the owner"));
    rejected_record(&r);
    delete_record(&r);
    assert!(next_text(&r).contains("wait for the owner"));
}

fn edit_record(r: &Repo, edit: impl Fn(String) -> String) {
    let t = edit(r.read(RECORD));
    r.write(RECORD, &t);
    r.commit("edit the record");
}

// The gate SD941, with `branch`.

#[test]
fn with_branch_a_third_record_after_2_rejected_gives_sd941_in_check_staged_check_and_finish() {
    let r = branch_flow();
    round_rejected_and_deleted(&r);
    round_rejected_and_deleted(&r);
    // The third record, staged: the hook `check --staged` stops the commit.
    let (code, out, err) = run(&r, &["accept", SLUG, "--session", "s1"]);
    assert_eq!(code, 0, "{out}{err}");
    r.git(&["add", RECORD]);
    let (code, out, err) = run(&r, &["check", "--staged"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD941"), "{out}");
    assert!(
        out.contains("2 rejected records") && out.contains("spec-dna approve valve-timeout plan"),
        "{out}"
    );
    // Committed, with a full record: `check` and `finish`.
    r.git(&["reset", "-q", "HEAD", "--", RECORD]);
    std::fs::remove_file(r.dir.join(RECORD)).unwrap();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    let (_, out, _) = run(&r, &["check"]);
    assert!(has(&out, "SD941"), "{out}");
    let (code, out, err) = run(&r, &["finish", SLUG, "--dry-run"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD941"), "{out}");
    assert!(err.contains("step 5"), "{err}");
}

#[test]
fn with_branch_the_second_record_is_no_stop_and_a_new_approval_clears_the_third() {
    let r = branch_flow();
    round_rejected_and_deleted(&r);
    // The second record: no SD941.
    commit_record(&r, &[BLOCKER]);
    let (_, out, _) = run(&r, &["check"]);
    assert!(!has(&out, "SD941"), "{out}");
    delete_record(&r);
    // `status: draft`, then the approval by an approver: no SD941 for a third.
    round_rejected_and_deleted(&r);
    approve_plan_again(&r, APPROVER);
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    let (_, out, _) = run(&r, &["check"]);
    assert!(!has(&out, "SD941"), "{out}");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    // A person who is no approver clears nothing.
    let r = branch_flow();
    round_rejected_and_deleted(&r);
    round_rejected_and_deleted(&r);
    approve_plan_again(&r, STRANGER);
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    let (_, out, _) = run(&r, &["check"]);
    assert!(has(&out, "SD941"), "{out}");
}

#[test]
fn with_branch_check_reads_the_count_where_the_diff_deletes_the_folder() {
    let r = branch_flow();
    round_rejected_and_deleted(&r);
    round_rejected_and_deleted(&r);
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    r.git(&["rm", "-q", "-r", "docs/intent/valve-timeout"]);
    r.commit("finish");
    let (_, out, _) = run(&r, &["check"]);
    assert!(has(&out, "SD941"), "{out}");
}

#[test]
fn with_base_there_is_no_sd941_and_finish_prints_the_runs() {
    let r = base_flow();
    round_rejected_and_deleted(&r);
    round_rejected_and_deleted(&r);
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    let (_, out, _) = run(&r, &["check"]);
    assert!(!has(&out, "SD941"), "{out}");
    let (code, out, err) = run(&r, &["finish", SLUG, "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.lines().any(|l| l == "Acceptance runs: 3, 2 rejected"),
        "{out}"
    );
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("Acceptance runs: 3, 2 rejected"), "{out}");
    // `next` shows the item done: the record is valid, fresh and accepted.
}

#[test]
fn finish_prints_the_line_only_for_2_runs_or_more() {
    let r = base_flow();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    let (code, out, err) = run(&r, &["finish", SLUG, "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("Acceptance runs"), "{out}");
    let r = base_flow();
    round_rejected_and_deleted(&r);
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    let (_, out, _) = run(&r, &["finish", SLUG, "--dry-run"]);
    assert!(out.contains("Acceptance runs: 2, 1 rejected"), "{out}");
    let (_, json, _) = run(&r, &["finish", SLUG, "--dry-run", "--json"]);
    assert!(
        json.contains("\"acceptance_runs\":{\"runs\":2,\"rejected\":1}"),
        "{json}"
    );
}

#[test]
fn a_shallow_clone_that_cuts_the_branch_gives_sd941_with_fetch_depth_in_branch_mode() {
    let src = branch_flow();
    round_rejected_and_deleted(&src);
    commit_record(&src, &[(CODE, "note", NOTE_TEXT)]);
    src.git(&["checkout", "-q", "main"]);
    src.write("README.md", "# Project, edited\n");
    src.commit("main moves on");
    src.git(&["checkout", "-q", "-b", "queue", "main"]);
    src.git(&["merge", "-q", "--no-ff", "-m", "test merge", "feature"]);
    let dst = Repo::empty();
    shallow_clone(&src.dir, &dst.dir, 2);
    let tip = src.git_out(&["rev-parse", "main"]);
    origin_head(&dst.dir, &tip);
    let (_, out, err) = run(&dst, &["check"]);
    let line = out
        .lines()
        .find(|l| l.contains(" SD941 "))
        .unwrap_or_else(|| panic!("{out}{err}"));
    assert!(line.contains("fetch-depth: 0"), "{line}");
}

// No edit clears a rejection (D74 item 2).

#[test]
fn a_commit_of_the_record_alone_that_lowers_the_blocker_does_not_clear_the_rejection() {
    let msg = "the record said `rejected` at ";
    let r = base_flow();
    rejected_record(&r);
    edit_record(&r, |t| {
        t.replace("severity = \"blocker\"", "severity = \"note\"")
            .replace("result = \"rejected\"", "result = \"accepted\"")
    });
    let (code, out, err) = run(&r, &["finish", SLUG, "--dry-run"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD928") && out.contains(msg), "{out}");
    assert!(out.contains("a later edit cannot accept it: fix the code, delete the record, commit, and accept again"), "{out}");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD928") && out.contains(msg), "{out}");
    // A record that says `accepted` from its first commit is no such case.
    let r = base_flow();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    let (code, out, err) = run(&r, &["finish", SLUG, "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    // After a delete and a new record, the life is new.
    let r = base_flow();
    rejected_record(&r);
    delete_record(&r);
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    let (code, out, err) = run(&r, &["finish", SLUG, "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_record_that_the_base_holds_starts_a_life_that_counts_once_also_when_the_branch_keeps_it() {
    // The base tree holds a rejected record. The branch keeps it for a commit, then
    // deletes it, and a second rejected record comes: 2 records, not 3.
    let cfg = config("standard", "");
    let r = Repo::new();
    r.write("spec-dna.toml", &pin_line("0.8.0", &cfg));
    base_files(&r);
    change_files(&r, INTENT_TEXT);
    r.write(
        RECORD,
        "format = 2\nchange = \"valve-timeout\"\nscope = []\npaths = []\ntree = \"0123456789abcdef\"\nresult = \"rejected\"\nacknowledged-by = []\n",
    );
    r.commit("plan and a rejected record");
    r.write("spec-dna.toml", &pin_line(VERSION, &cfg));
    r.write("README.md", "# Project, with the new pin\n");
    r.commit("move the pin");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    add_code(&r);
    // The branch keeps the record, and the base life is the only record so far.
    assert!(!next_text(&r).contains("wait for the owner"));
    delete_record(&r);
    assert!(!next_text(&r).contains("wait for the owner"));
    rejected_record(&r);
    delete_record(&r);
    // The base life and this one: 2.
    let item = accept_item(&next_text(&r));
    assert!(item.contains(WAIT), "{item}");
}

#[test]
fn a_base_rejection_before_a_branch_approval_counts_for_nothing_after_it() {
    let cfg = config("standard", &branch_keys());
    let r = Repo::new();
    r.write("spec-dna.toml", &pin_line("0.8.0", &cfg));
    base_files(&r);
    change_files(&r, INTENT_TEXT);
    r.write(
        RECORD,
        "format = 2\nchange = \"valve-timeout\"\nscope = []\npaths = []\ntree = \"0123456789abcdef\"\nresult = \"rejected\"\nacknowledged-by = []\n",
    );
    r.commit("plan and a rejected record");
    r.write("spec-dna.toml", &pin_line(VERSION, &cfg));
    r.write("README.md", "# Project, with the new pin\n");
    r.commit("move the pin");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    add_code(&r);
    delete_record(&r);
    // The approval comes after the record of the base, and then one more rejected
    // record, deleted, and the third record. 2 rejected records in all, but only
    // one after the approval.
    approve_plan_again(&r, APPROVER);
    round_rejected_and_deleted(&r);
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    let (_, out, _) = run(&r, &["check"]);
    assert!(!has(&out, "SD941"), "{out}");
}

// A record that does not parse still counts as rejected (the count reads `result`
// leniently).

/// A rejected record that the strict reader refuses, committed and then deleted.
fn round_unreadable(r: &Repo, how: &str) {
    rejected_record(r);
    // The first commit of the record is the unreadable one.
    let t = r.read(RECORD);
    r.write(
        RECORD,
        &match how {
            "key" => t.replacen("outcome = \"held\"", "outcome = \"held\"\nduration = 5", 1),
            _ => t.replacen("[[finding]]", "[[finding]", 1),
        },
    );
    r.git(&["commit", "-q", "--amend", "-a", "--no-edit"]);
    delete_record(r);
}

#[test]
fn a_rejected_record_that_does_not_parse_still_counts_in_base_mode() {
    for how in ["key", "syntax"] {
        let r = base_flow();
        round_unreadable(&r, how);
        round_unreadable(&r, how);
        let item = accept_item(&next_text(&r));
        assert!(item.contains(WAIT), "{how}: {item}");
        commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
        let (code, out, err) = run(&r, &["finish", SLUG, "--dry-run"]);
        assert_eq!(code, 0, "{how}: {out}{err}");
        assert!(
            out.lines().any(|l| l == "Acceptance runs: 3, 2 rejected"),
            "{how}: {out}"
        );
    }
}

#[test]
fn a_rejected_record_that_does_not_parse_still_counts_in_branch_mode() {
    for how in ["key", "syntax"] {
        let r = branch_flow();
        round_unreadable(&r, how);
        round_unreadable(&r, how);
        commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
        let (_, out, _) = run(&r, &["check"]);
        assert!(has(&out, "SD941"), "{how}: {out}");
    }
}

#[test]
fn the_item_of_a_person_has_the_final_wording_with_the_skill_review_and_no_prompt() {
    let r = base_flow();
    round_rejected_and_deleted(&r);
    round_rejected_and_deleted(&r);
    let item = accept_item(&next_text(&r));
    let (head, tail) = item.split_once('(').unwrap();
    assert_eq!(
        head,
        "[ ] wait for the owner: 2 acceptance records of valve-timeout say `rejected` "
    );
    let (ids, tail) = tail.split_once(')').unwrap();
    assert_eq!(ids.split(", ").count(), 2);
    assert_eq!(tail, "; a third run needs the word of the owner [person]");
    // `--json`: the skill of the step, and the flag of a person; no prompt.
    let (_, json, _) = run(&r, &["next", SLUG, "--json"]);
    let at = json.find("wait for the owner").unwrap();
    let object = &json[json[..at].rfind('{').unwrap()..at + json[at..].find('}').unwrap()];
    assert!(
        object.contains("\"skill\":\"review\"") && object.contains("\"person\":true"),
        "{object}"
    );
    assert!(
        !json.contains("spec-dna accept valve-timeout --brief"),
        "{json}"
    );
}
