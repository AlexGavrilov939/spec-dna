//! Tests of the acceptance record in `finish` (steps 5.2 and 5.5) and in `check`
//! (design 0.8.0, D3 items 1, 3, 7, 8 and 9): the rule "a change that needs a
//! record", the codes SD927 to SD931, SD933 and SD935, the archive file, and the
//! places where `check` runs them.
//!
//! The `finish` tests use a plan that the branch adds (`built`): such a plan
//! needs a record. The `check` tests use a plan on the base (`on_base`), as a
//! real pull request has it: an earlier pull request merged the plan, and the
//! pin of that commit decides if the change needs a record. The fixtures write
//! the version pin of an older commit as a text, never as `VERSION`, so the
//! tests keep their meaning when the binary moves to 0.8.0. The fixtures use
//! example names only.

mod common;

use common::*;

const SLUG: &str = "valve-timeout";
const FOLDER: &str = "docs/intent/valve-timeout";
const INTENT: &str = "docs/intent/valve-timeout/intent.md";
const SPEC: &str = "docs/intent/valve-timeout/spec.md";
const PLAN: &str = "docs/intent/valve-timeout/plan.md";
const RECORD: &str = "docs/intent/valve-timeout/acceptance.toml";
const ARCHIVE: &str = ".spec-dna/accepted/valve-timeout.toml";
const ARCHIVE_2: &str = ".spec-dna/accepted/valve-timeout-2.toml";
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

fn config_with(profile: &str, change: &str, rules: &str) -> String {
    format!(
        "profile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n{change}\n[rules]\nSD608 = \"off\"\n{rules}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n"
    )
}

fn config(profile: &str, change: &str) -> String {
    config_with(profile, change, "")
}

fn higher_risk(text: &str) -> String {
    text.replace("class: routine", "class: higher-risk")
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

/// The base commit on `main`, `origin/HEAD` on it, and the branch `feature`.
fn start(config_text: &str) -> Repo {
    let r = Repo::with_config(config_text);
    base_files(&r);
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r
}

fn add_code(r: &Repo) {
    r.write(CODE, CODE_NEW);
    r.write(TEST, TEST_NEW);
    r.commit("code");
}

/// A pull request that adds the plan in one commit and the code in the next.
/// The plan is new: the branch adds it.
fn built_with(config_text: &str, intent: &str) -> Repo {
    let r = start(config_text);
    change_files(&r, intent);
    r.commit("plan");
    add_code(&r);
    r
}

fn built() -> Repo {
    built_with(&config("standard", ""), INTENT_TEXT)
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

fn last_line(text: &str) -> &str {
    text.lines().last().unwrap()
}

/// The hash of the brief of the change for a role.
fn brief_hash(r: &Repo, role: &str) -> String {
    let (code, out, err) = run(r, &["accept", SLUG, "--brief", "--role", role]);
    assert_eq!(code, 0, "{out}{err}");
    last_line(&out).strip_prefix("brief ").unwrap().to_string()
}

const ATTEMPT: &str = "\n[[attempt]]\nacceptor = {n}\ntext = \"Sent a NaN value after the timeout.\"\noutcome = \"held\"\n";

/// The text of a finished record of the roles, with the session `s<n>` for the
/// acceptor `n`. The briefs are the real ones. The file is not left on disk.
fn record_text(r: &Repo, roles: &[&str], result: &str) -> String {
    let (code, out, err) = run(r, &["accept", SLUG, "--role", roles[0], "--session", "s1"]);
    assert_eq!(code, 0, "{out}{err}");
    for (i, role) in roles.iter().enumerate().skip(1) {
        let s = format!("s{}", i + 1);
        let (code, out, err) = run(r, &["accept", SLUG, "--role", role, "--session", &s]);
        assert_eq!(code, 0, "{out}{err}");
    }
    let mut t = r.read(RECORD);
    std::fs::remove_file(r.dir.join(RECORD)).unwrap();
    for role in roles {
        let hash = brief_hash(r, role);
        t = t.replacen("brief = \"pending\"", &format!("brief = \"{hash}\""), 1);
    }
    t = t.replace("result = \"pending\"", &format!("result = \"{result}\""));
    for n in 1..=roles.len() {
        t.push_str(&ATTEMPT.replace("{n}", &n.to_string()));
    }
    t
}

/// Writes the record and commits it alone.
fn commit_record(r: &Repo, roles: &[&str], result: &str) -> String {
    let t = record_text(r, roles, result);
    r.write(RECORD, &t);
    r.commit("record");
    t
}

/// The record with a second entry that has the same role, session and brief,
/// and the `agent` value `agent` in both entries.
fn two_entries(t: &str, agent: &str) -> String {
    let entry = t
        .split("[[acceptor]]")
        .nth(1)
        .unwrap()
        .split("\n[[")
        .next()
        .unwrap()
        .to_string();
    let named = entry.replace("agent = \"unknown\"", &format!("agent = \"{agent}\""));
    format!(
        "{}\n[[acceptor]]{named}{}",
        t.replace(&entry, &named).trim_end(),
        ATTEMPT.replace("{n}", "2")
    )
}

fn has(out: &str, code: &str) -> bool {
    out.lines().any(|l| l.contains(&format!(" {code} ")))
}

fn json_step(out: &str) -> u32 {
    let rest = out.split("\"step\":").nth(1).expect("a step");
    rest.split(|c: char| !c.is_ascii_digit())
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

const CODES: [&str; 7] = [
    "SD927", "SD928", "SD929", "SD930", "SD931", "SD933", "SD935",
];

// finish: the rule "a change that needs a record".

#[test]
fn a_new_plan_and_no_record_stops_finish_at_step_5_with_sd927() {
    let r = built();
    let (code, out, err) = run(&r, &["finish", SLUG, "--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert_eq!(json_step(&out), 5, "{out}");
    assert!(out.contains("SD927"), "{out}");
    assert!(out.contains(RECORD), "{out}");
    assert!(r.dir.join(FOLDER).is_dir(), "nothing changed");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
    assert!(err.contains("stopped at step 5"), "{err}");
}

#[test]
fn a_plan_added_at_pin_0_8_0_and_no_record_stops_with_sd927() {
    let r = on_base("0.8.0", &config("standard", ""), INTENT_TEXT);
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

#[test]
fn a_plan_added_at_a_pin_before_0_8_0_needs_no_record_and_finish_prints_the_note() {
    let r = on_base("0.7.1", &config("standard", ""), INTENT_TEXT);
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(ARCHIVE).exists(), "no archive file");
    assert!(
        err.contains("note: `docs/intent/valve-timeout/plan.md` was added when spec-dna.toml pinned 0.7.1, before 0.8.0, so `finish` needs no acceptance record"),
        "{err}"
    );
}

#[test]
fn the_note_is_not_in_the_json_and_the_step_is_6() {
    let r = on_base("0.7.1", &config("standard", ""), INTENT_TEXT);
    let (code, out, err) = run(&r, &["finish", SLUG, "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(json_step(&out), 6, "{out}");
    assert!(!out.contains("note"), "{out}");
}

#[test]
fn a_plan_that_the_branch_added_after_a_pin_set_back_needs_a_record() {
    // The plan comes in a commit of the branch that pins 0.7.0, and a later
    // commit pins the version of the binary again.
    let r = start(&config("standard", ""));
    r.write("spec-dna.toml", &pin_line("0.7.0", &config("standard", "")));
    change_files(&r, INTENT_TEXT);
    r.commit("plan with a pin set back");
    r.write("spec-dna.toml", &pin_line(VERSION, &config("standard", "")));
    add_code(&r);
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

#[test]
fn a_plan_that_a_finished_change_deleted_and_the_branch_added_again_needs_a_record() {
    // The base history has an old plan of the slug, added at a pin before
    // 0.8.0 and deleted by `finish`. The branch adds a new plan with the slug.
    let r = Repo::new();
    r.write("spec-dna.toml", &pin_line("0.7.0", &config("standard", "")));
    base_files(&r);
    change_files(&r, INTENT_TEXT);
    r.commit("old plan");
    r.git(&["rm", "-q", "-r", FOLDER]);
    r.write("spec-dna.toml", &pin_line(VERSION, &config("standard", "")));
    r.commit("finished");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    change_files(&r, INTENT_TEXT);
    add_code(&r);
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

#[test]
fn in_lite_a_new_plan_needs_no_record_and_finish_is_as_before() {
    let r = built_with(&config("lite", ""), INTENT_TEXT);
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(".spec-dna").exists(), "no archive folder");
    assert!(!err.contains("note"), "{err}");
}

#[test]
fn in_lite_a_rule_for_the_group_turns_the_record_on() {
    let text = config_with("lite", "", "acceptance = \"error\"\n");
    let r = built_with(&text, INTENT_TEXT);
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

#[test]
fn finish_archives_a_record_that_is_present_also_in_lite_and_for_an_old_pin() {
    // `lite`: no rule needs the record, but a record in the folder is kept.
    let r = built_with(&config("lite", ""), INTENT_TEXT);
    let text = commit_record(&r, &["acceptor"], "accepted");
    let (code, out, err) = run(&r, &["finish", SLUG, "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains(ARCHIVE), "the dry-run lists the file: {out}");
    assert!(r.dir.join(RECORD).exists(), "the dry-run writes nothing");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.read(ARCHIVE), text);
    assert!(!r.dir.join(FOLDER).exists());
    let staged = r.git_out(&["diff", "--cached", "--name-only"]);
    assert!(staged.lines().any(|l| l == ARCHIVE), "{staged}");
    // A plan of an older pin: no step 5.2, but the record moves.
    let r = on_base("0.7.1", &config("standard", ""), INTENT_TEXT);
    let text = commit_record(&r, &["acceptor"], "accepted");
    let (code, out, err) = run(&r, &["finish", SLUG, "--json", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(json_step(&out), 6, "{out}");
    assert!(out.contains(&format!("\"path\":\"{ARCHIVE}\"")), "{out}");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.read(ARCHIVE), text);
    assert!(err.contains("before 0.8.0"), "the note stays: {err}");
}

#[test]
fn finish_with_no_record_and_no_need_writes_no_archive_file() {
    let r = on_base("0.7.1", &config("standard", ""), INTENT_TEXT);
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(".spec-dna").exists());
}

#[test]
fn a_closed_change_needs_no_record() {
    let r = built();
    r.write(
        INTENT,
        &INTENT_TEXT.replace("status: accepted", "status: closed"),
    );
    r.commit("close");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(".spec-dna").exists());
}

#[test]
fn a_shallow_clone_that_cuts_the_adding_commit_needs_a_record_and_sd927_names_the_cause() {
    // The base commit holds a plan of 0.7.1. The clone is shallow, so no commit
    // that added the plan is in its history: the stricter result.
    let src = on_base("0.7.1", &config("standard", ""), INTENT_TEXT);
    src.git(&["checkout", "-q", "main"]);
    src.write("README.md", "# Project, edited\n");
    src.commit("main moves on");
    src.git(&["checkout", "-q", "-b", "queue", "main"]);
    src.git(&["merge", "-q", "--no-ff", "-m", "test merge", "feature"]);
    let dst = Repo::empty();
    shallow_clone(&src.dir, &dst.dir, 2);
    let tip = src.git_out(&["rev-parse", "main"]);
    origin_head(&dst.dir, &tip);
    assert_eq!(
        dst.git_out(&["rev-parse", "--is-shallow-repository"]),
        "true"
    );
    let (code, out, err) = run(&dst, &["finish", SLUG]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
    assert!(
        out.contains("shallow") && out.contains("fetch-depth: 0"),
        "{out}"
    );
}

// finish: steps 5.2 and 5.5.

#[test]
fn a_valid_record_passes_finish_and_step_5_5_moves_it_to_the_archive() {
    let r = built();
    let text = commit_record(&r, &["acceptor"], "accepted");
    let (code, out, err) = run(&r, &["finish", SLUG, "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains(ARCHIVE),
        "the dry run lists the archive path: {out}"
    );
    assert!(!r.dir.join(ARCHIVE).exists(), "a dry run writes nothing");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains(&format!("wrote {ARCHIVE}")), "{out}");
    assert_eq!(
        r.read(ARCHIVE),
        text,
        "the archive has the bytes of the record"
    );
    assert!(!r.dir.join(FOLDER).exists());
    // `finish` stages the archive file, as it stages each file that it writes.
    let staged = r.git_out(&["diff", "--cached", "--name-only"]);
    assert!(staged.lines().any(|l| l == ARCHIVE), "{staged}");
}

#[test]
fn finish_json_lists_the_archive_file_and_keeps_the_step_6() {
    let r = built();
    commit_record(&r, &["acceptor"], "accepted");
    let (code, out, err) = run(&r, &["finish", SLUG, "--json", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(json_step(&out), 6, "{out}");
    assert!(out.contains(&format!("\"path\":\"{ARCHIVE}\"")), "{out}");
}

#[test]
fn a_second_change_with_the_same_slug_writes_a_numbered_archive_file() {
    let r = start(&config("standard", ""));
    r.write(ARCHIVE, "format = 1\n");
    r.commit("an earlier record of the same slug");
    change_files(&r, INTENT_TEXT);
    r.commit("plan");
    add_code(&r);
    let text = commit_record(&r, &["acceptor"], "accepted");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.read(ARCHIVE), "format = 1\n", "the old file stays");
    assert_eq!(r.read(ARCHIVE_2), text);
    // The third change of the slug: `-3`.
    let r = start(&config("standard", ""));
    r.write(ARCHIVE, "format = 1\n");
    r.write(ARCHIVE_2, "format = 1\n");
    r.commit("2 earlier records");
    change_files(&r, INTENT_TEXT);
    r.commit("plan");
    add_code(&r);
    let text = commit_record(&r, &["acceptor"], "accepted");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.read(".spec-dna/accepted/valve-timeout-3.toml"), text);
}

#[test]
fn an_ignored_archive_path_stops_finish_before_it_writes_and_names_the_rule() {
    let r = start(&config("standard", ""));
    r.write(".gitignore", ".spec-dna/\n");
    r.commit("ignore the folder");
    change_files(&r, INTENT_TEXT);
    r.commit("plan");
    add_code(&r);
    commit_record(&r, &["acceptor"], "accepted");
    let (code, out, err) = run(&r, &["finish", SLUG, "--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert_eq!(json_step(&out), 5, "{out}");
    assert!(out.contains(".gitignore:1:.spec-dna/"), "{out}");
    assert!(r.dir.join(FOLDER).is_dir(), "nothing changed");
    assert!(!r.dir.join(ARCHIVE).exists());
    assert_eq!(r.git_out(&["status", "--porcelain"]), "");
}

#[test]
fn a_record_that_breaks_a_rule_stops_with_sd928() {
    // `accepted` with an `important` finding.
    let r = built();
    let mut t = record_text(&r, &["acceptor"], "accepted");
    t.push_str("\n[[finding]]\nacceptor = 1\nseverity = \"important\"\npath = \"src/valves.py\"\ntext = \"The timeout has no unit.\"\n");
    r.write(RECORD, &t);
    r.commit("record");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD928"), "{out}");
    assert!(out.contains("important"), "{out}");
    // A record of 16,385 bytes.
    let r = built();
    let mut t = record_text(&r, &["acceptor"], "accepted");
    let pad = 16_385 - t.len() - 3;
    t.push_str(&format!("# {}\n", "x".repeat(pad)));
    assert_eq!(t.len(), 16_385);
    r.write(RECORD, &t);
    r.commit("record");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD928"), "{out}");
    assert!(out.contains("16384") || out.contains("16,384"), "{out}");
}

#[test]
fn a_brief_that_is_not_the_brief_of_the_adding_commit_stops_with_sd928() {
    let r = built();
    let t = record_text(&r, &["acceptor"], "accepted");
    let good = brief_hash(&r, "acceptor");
    r.write(RECORD, &t.replace(&good, "0123456789abcdef"));
    r.commit("record");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD928"), "{out}");
    assert!(
        out.contains("remove the entry, and accept again from a new brief"),
        "{out}"
    );
    assert!(out.contains("acceptor 1"), "{out}");
}

#[test]
fn a_pending_brief_stops_with_sd928() {
    let r = built();
    let t = record_text(&r, &["acceptor"], "accepted");
    let good = brief_hash(&r, "acceptor");
    r.write(
        RECORD,
        &t.replace(&format!("brief = \"{good}\""), "brief = \"pending\""),
    );
    r.commit("record");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD928"), "{out}");
    assert!(out.contains("pending"), "{out}");
}

#[test]
fn a_record_with_an_older_tree_after_a_code_edit_stops_with_sd929() {
    let r = built();
    commit_record(&r, &["acceptor"], "accepted");
    r.write(CODE, &format!("{CODE_NEW}\n\ndef more():\n    pass\n"));
    r.commit("edit the code");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD929"), "{out}");
    assert!(!has(&out, "SD928"), "{out}");
    assert!(out.contains("accept"), "the message gives the fix: {out}");
}

#[test]
fn a_markdown_edit_after_the_record_gives_no_sd929() {
    let r = built();
    commit_record(&r, &["acceptor"], "accepted");
    r.write("README.md", "# Project, edited\n");
    r.commit("edit a Markdown file");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_code_path_of_the_scope_that_the_record_does_not_list_stops_with_sd929() {
    let r = start(&config("standard", ""));
    change_files(&r, INTENT_TEXT);
    r.write(
        PLAN,
        &PLAN_TEXT.replace("- `src/valves.py`\n", "- `src/**`\n"),
    );
    r.commit("plan");
    add_code(&r);
    commit_record(&r, &["acceptor"], "accepted");
    r.write("src/extra.py", "z = 1\n");
    r.commit("a new code path in the scope");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD929"), "{out}");
    assert!(out.contains("src/extra.py"), "{out}");
}

#[test]
fn a_rejected_record_stops_with_sd930() {
    let r = built();
    commit_record(&r, &["acceptor"], "rejected");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD930"), "{out}");
}

#[test]
fn two_entries_with_one_session_and_one_agent_give_sd931_but_unknown_matches_nothing() {
    let r = built();
    let t = record_text(&r, &["acceptor"], "accepted");
    r.write(RECORD, &two_entries(&t, "a1b2c3"));
    r.commit("record");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD931"), "{out}");
    // `unknown` matches no value: 2 honest subagents of one session.
    r.write(RECORD, &two_entries(&t, "unknown"));
    r.commit("the same session, no agent id");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_higher_risk_change_needs_the_configured_number_of_acceptors_and_sd933_stops() {
    let cfg = config("standard", "higher-risk-acceptors = 2\n");
    let r = built_with(&cfg, &higher_risk(INTENT_TEXT));
    commit_record(&r, &["acceptor"], "accepted");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD933"), "{out}");
}

#[test]
fn a_routine_change_with_one_acceptor_and_the_key_set_to_2_passes() {
    let cfg = config("standard", "higher-risk-acceptors = 2\n");
    let r = built_with(&cfg, INTENT_TEXT);
    commit_record(&r, &["acceptor"], "accepted");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn two_acceptors_pass_a_higher_risk_change_with_the_key_set_to_2() {
    let cfg = config("standard", "higher-risk-acceptors = 2\n");
    let r = built_with(&cfg, &higher_risk(INTENT_TEXT));
    commit_record(&r, &["acceptor", "reviewer"], "accepted");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_later_pull_request_with_only_finish_runs_no_compare_for_an_entry_of_the_base() {
    // The record merged in an earlier pull request: the base has it. A plan
    // edit of this pull request changes the brief, and no SD928 follows.
    let r = base_flow();
    commit_record(&r, &["acceptor"], "accepted");
    r.git(&["checkout", "-q", "main"]);
    r.git(&["merge", "-q", "--ff-only", "feature"]);
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "finish-pr"]);
    r.write(
        PLAN,
        &PLAN_TEXT.replace("- A slow sensor.", "- A slow sensor, and a slow valve."),
    );
    r.commit("a tick or an edit of the plan");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("finish");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!has(&out, "SD928"), "{out}");
}

// check: the record in the folder.

#[test]
fn a_committed_record_with_the_brief_of_its_commit_gives_no_sd928() {
    let r = base_flow();
    commit_record(&r, &["acceptor"], "accepted");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!has(&out, "SD928"), "{out}");
}

#[test]
fn check_reports_sd928_for_a_broken_rule_a_wrong_hash_and_a_pending_hash() {
    for how in ["important", "hash", "pending"] {
        let r = base_flow();
        let mut t = record_text(&r, &["acceptor"], "accepted");
        let good = brief_hash(&r, "acceptor");
        match how {
            "important" => t.push_str("\n[[finding]]\nacceptor = 1\nseverity = \"important\"\npath = \"src/valves.py\"\ntext = \"A defect.\"\n"),
            "hash" => t = t.replace(&good, "0123456789abcdef"),
            _ => t = t.replace(&format!("brief = \"{good}\""), "brief = \"pending\""),
        }
        r.write(RECORD, &t);
        r.commit("record");
        let (code, out, err) = run(&r, &["check"]);
        assert_eq!(code, 1, "{how}: {out}{err}");
        assert!(has(&out, "SD928"), "{how}: {out}");
        if how == "hash" {
            assert!(
                out.contains("remove the entry, and accept again from a new brief"),
                "{out}"
            );
        }
    }
}

#[test]
fn a_later_edit_of_the_plan_after_the_record_commit_gives_no_sd928() {
    // The compare reads the tree of the adding commit, so a later tick of a box
    // changes nothing.
    let r = base_flow();
    commit_record(&r, &["acceptor"], "accepted");
    r.write(
        PLAN,
        &PLAN_TEXT.replace("- A slow sensor.", "- A slow sensor, and a slow valve."),
    );
    r.commit("edit the plan");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!has(&out, "SD928"), "{out}");
}

#[test]
fn an_entry_that_no_commit_has_is_compared_with_the_working_tree_or_the_index() {
    let r = base_flow();
    let t = record_text(&r, &["acceptor"], "accepted");
    r.write(RECORD, &t);
    // The working tree: the brief is the brief of the files now.
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");
    // With `--staged`, the record is in the index.
    r.git(&["add", RECORD]);
    let (code, out, err) = run(&r, &["check", "--staged"]);
    assert_eq!(code, 0, "{out}{err}");
    // A wrong hash in the index.
    let good = brief_hash(&r, "acceptor");
    r.write(RECORD, &t.replace(&good, "0123456789abcdef"));
    r.git(&["add", RECORD]);
    let (code, out, err) = run(&r, &["check", "--staged"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD928"), "{out}");
}

#[test]
fn a_rejected_record_then_a_code_fix_and_a_new_acceptor_compare_with_their_own_commit() {
    // The first record (role acceptor, session s1, hash H1) says `rejected`. The
    // builder fixes the code and edits the plan, and a commit deletes the
    // record. A new acceptor of the same session accepts from a new brief H2.
    let r = base_flow();
    commit_record(&r, &["acceptor"], "rejected");
    let h1 = brief_hash(&r, "acceptor");
    r.write(CODE, &format!("{CODE_NEW}\n\ndef fixed():\n    pass\n"));
    r.write(
        PLAN,
        &PLAN_TEXT.replace("- A slow sensor.", "- A slow sensor, and a fix."),
    );
    r.commit("fix the code and the plan");
    std::fs::remove_file(r.dir.join(RECORD)).unwrap();
    r.commit("delete the record");
    let h2 = brief_hash(&r, "acceptor");
    assert_ne!(h1, h2);
    let t = record_text(&r, &["acceptor"], "accepted");
    r.write(RECORD, &t);
    r.commit("new record");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!has(&out, "SD928"), "{out}");
    // The same session with the old hash H1: a new run starts at the new
    // commit, and the brief of that tree is H2.
    r.write(RECORD, &t.replace(&h2, &h1));
    r.commit("the old hash again");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD928"), "{out}");
}

#[test]
fn a_later_commit_that_edits_the_findings_does_not_move_the_adding_commit() {
    let r = base_flow();
    commit_record(&r, &["acceptor"], "accepted");
    r.write(
        PLAN,
        &PLAN_TEXT.replace("- A slow sensor.", "- A slow sensor, and a slow valve."),
    );
    r.commit("edit the plan");
    // A later commit adds a nit to the record. The entry keeps its triple.
    let mut t = r.read(RECORD);
    t.push_str("\n[[finding]]\nacceptor = 1\nseverity = \"nit\"\npath = \"src/valves.py\"\ntext = \"A name.\"\n");
    r.write(RECORD, &t);
    r.commit("a nit");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!has(&out, "SD928"), "{out}");
}

#[test]
fn check_reports_sd931_for_a_hand_written_agent_of_two_entries() {
    let r = base_flow();
    let t = record_text(&r, &["acceptor"], "accepted");
    r.write(RECORD, &two_entries(&t, "a1b2c3"));
    r.commit("record");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD931"), "{out}");
}

#[test]
fn a_rejected_record_and_a_code_fix_give_no_sd929_with_check_staged() {
    let r = base_flow();
    commit_record(&r, &["acceptor"], "rejected");
    r.write(CODE, &format!("{CODE_NEW}\n\ndef fixed():\n    pass\n"));
    r.git(&["add", CODE]);
    let (code, out, err) = run(&r, &["check", "--staged"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!has(&out, "SD929") && !has(&out, "SD930"), "{out}");
    // The same without `--staged`.
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_first_acceptor_of_two_can_commit_and_check_gives_no_sd933() {
    let cfg = config("standard", "higher-risk-acceptors = 2\n");
    let r = on_base("0.8.0", &cfg, &higher_risk(INTENT_TEXT));
    let t = record_text(&r, &["acceptor"], "accepted");
    r.write(RECORD, &t);
    r.git(&["add", RECORD]);
    let (code, out, err) = run(&r, &["check", "--staged"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!has(&out, "SD933"), "{out}");
}

// check: where the diff deletes the folder, and the branch-born change.

/// The pull request of `finish`, written by hand: the folder is deleted, the
/// delta is applied, and the archive file holds `record`.
fn finished_by_hand(r: &Repo, record: Option<&str>) {
    r.git(&["rm", "-q", "-r", FOLDER]);
    let valves = format!(
        "{VALVES}- **sensor-timeout**: When a sensor gives no value for 60 seconds, the controller SHALL close the valve of its bed.\n"
    );
    r.write(REQ, &valves);
    if let Some(t) = record {
        r.write(ARCHIVE, t);
    }
    r.commit("finish");
}

#[test]
fn a_folder_deleted_with_no_finish_and_no_archive_gives_sd927_also_when_the_base_has_a_record() {
    let r = base_flow();
    commit_record(&r, &["acceptor"], "accepted");
    r.git(&["checkout", "-q", "main"]);
    r.git(&["merge", "-q", "--ff-only", "feature"]);
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "finish-pr"]);
    // A person deletes the folder with no `finish`: the record stays only in
    // the folder of the base.
    finished_by_hand(&r, None);
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
    assert!(out.contains("the diff adds no archive file"), "{out}");
}

#[test]
fn an_older_archive_file_of_the_slug_in_the_head_does_not_satisfy_sd927() {
    let r = start(&config("standard", ""));
    r.write(ARCHIVE, "format = 1\n");
    r.commit("an earlier record of the same slug");
    r.write("spec-dna.toml", &pin_line("0.8.0", &config("standard", "")));
    change_files(&r, INTENT_TEXT);
    r.commit("plan");
    r.write("spec-dna.toml", &pin_line(VERSION, &config("standard", "")));
    r.write("README.md", "# Project, with the new pin\n");
    r.commit("move the pin");
    origin_head(&r.dir, "feature");
    r.git(&["checkout", "-q", "-b", "finish-pr"]);
    add_code(&r);
    finished_by_hand(&r, None);
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    let line = out.lines().find(|l| l.contains(" SD927 ")).expect("SD927");
    assert!(line.contains("the diff adds no archive file"), "{line}");
    assert!(!line.contains("the head has no archive file"), "{line}");
}

#[test]
fn the_pull_request_of_finish_gives_no_record_code_and_no_sd913() {
    let r = base_flow();
    commit_record(&r, &["acceptor"], "accepted");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("finish");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");
    for c in CODES {
        assert!(!has(&out, c), "{c}: {out}");
    }
    assert!(
        !has(&out, "SD913"),
        "the archive path is no path outside the plan: {out}"
    );
}

#[test]
fn a_record_in_the_folder_gives_no_sd913() {
    let r = base_flow();
    commit_record(&r, &["acceptor"], "accepted");
    let (_, out, err) = run(&r, &["check", "--all"]);
    assert!(!has(&out, "SD913"), "{out}{err}");
}

#[test]
fn a_branch_born_change_with_no_record_gives_sd927_beside_sd910() {
    let r = built();
    finished_by_hand(&r, None);
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
    assert!(has(&out, "SD910"), "{out}");
}

#[test]
fn the_archive_file_of_a_finish_in_the_diff_is_read_for_sd929_sd930_and_sd933() {
    // SD929: the code changed after the record.
    let r = base_flow();
    let t = commit_record(&r, &["acceptor"], "accepted");
    r.write(CODE, &format!("{CODE_NEW}\n\ndef more():\n    pass\n"));
    r.commit("edit the code");
    finished_by_hand(&r, Some(&t));
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD929"), "{out}");
    assert!(out.contains("revert the commit of `finish`"), "{out}");
    // SD930: the result is `rejected`.
    let r = base_flow();
    let t = commit_record(&r, &["acceptor"], "rejected");
    finished_by_hand(&r, Some(&t));
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD930"), "{out}");
    assert!(!has(&out, "SD929"), "{out}");
    // SD933: 1 entry, and the change needs 2.
    let cfg = config("standard", "higher-risk-acceptors = 2\n");
    let r = on_base("0.8.0", &cfg, &higher_risk(INTENT_TEXT));
    let t = commit_record(&r, &["acceptor"], "accepted");
    finished_by_hand(&r, Some(&t));
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD933"), "{out}");
}

#[test]
fn a_markdown_edit_after_the_record_gives_no_sd929_in_the_finish_pull_request() {
    let r = base_flow();
    let t = commit_record(&r, &["acceptor"], "accepted");
    r.write("README.md", "# Project, edited\n");
    r.commit("edit a Markdown file");
    finished_by_hand(&r, Some(&t));
    let (_, out, err) = run(&r, &["check"]);
    assert!(!has(&out, "SD929"), "{out}{err}");
}

#[test]
fn a_base_commit_that_edits_a_listed_path_makes_the_record_stale_in_the_test_merge() {
    let r = base_flow();
    let t = commit_record(&r, &["acceptor"], "accepted");
    finished_by_hand(&r, Some(&t));
    // `main` edits a listed path. The test merge of CI is the merge of `main`
    // into the branch.
    r.git(&["checkout", "-q", "main"]);
    r.write(CODE, &format!("# main\n{CODE_OLD}"));
    r.commit("main edits a listed path");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "feature"]);
    r.git(&["merge", "-q", "-m", "test merge", "main"]);
    let (_, out, err) = run(&r, &["check"]);
    assert!(has(&out, "SD929"), "{out}{err}");
    assert!(out.contains("revert the commit of `finish`"), "{out}");
}

#[test]
fn a_base_commit_to_another_path_leaves_the_record_fresh() {
    let r = base_flow();
    let t = commit_record(&r, &["acceptor"], "accepted");
    finished_by_hand(&r, Some(&t));
    r.git(&["checkout", "-q", "main"]);
    r.write("src/other.py", "y = 2\n");
    r.commit("main edits another path");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "feature"]);
    r.git(&["merge", "-q", "-m", "test merge", "main"]);
    let (_, out, err) = run(&r, &["check"]);
    assert!(!has(&out, "SD929"), "{out}{err}");
}

#[test]
fn a_shallow_clone_that_cuts_the_history_gives_sd928_that_names_fetch_depth() {
    // With the branch rule, a depth-1 clone with the tip of `main` has no merge
    // base: the tip is the base, and `base..HEAD` is cut.
    let cfg = config(
        "standard",
        "approval = \"branch\"\napprovers = [\"owner@example.org\"]\n",
    );
    let src = on_base("0.8.0", &cfg, INTENT_TEXT);
    commit_record(&src, &["acceptor"], "accepted");
    let dst = Repo::empty();
    shallow_clone(&src.dir, &dst.dir, 1);
    let tip = src.git_out(&["rev-parse", "main"]);
    dst.git(&[
        "fetch",
        "-q",
        "--depth",
        "1",
        "origin",
        &format!("{tip}:refs/remotes/origin/main"),
    ]);
    origin_head(&dst.dir, &tip);
    let (code, out, err) = run(&dst, &["check", "--base", "origin/main"]);
    assert_eq!(code, 1, "{out}{err}");
    let line = out
        .lines()
        .find(|l| l.contains(" SD928 "))
        .unwrap_or_else(|| panic!("no SD928: {out}"));
    assert!(line.contains("fetch-depth: 0"), "{line}");
}

// check: SD935 for the bytes of an archive file that the diff adds.

#[test]
fn an_archive_file_with_other_bytes_than_the_folder_record_gives_sd935() {
    // The bytes differ by one line.
    let r = base_flow();
    let t = commit_record(&r, &["acceptor"], "accepted");
    finished_by_hand(&r, Some(&format!("{t}# edited\n")));
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    let line = out.lines().find(|l| l.contains(" SD935 ")).expect("SD935");
    assert!(line.starts_with(ARCHIVE), "{line}");
    // The same bytes give none.
    let r = base_flow();
    let t = commit_record(&r, &["acceptor"], "accepted");
    finished_by_hand(&r, Some(&t));
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!has(&out, "SD935"), "{out}");
}

#[test]
fn a_later_commit_that_edits_the_archive_file_that_the_branch_added_gives_sd935() {
    // The branch adds the plan and the record, runs `finish`, and then a commit
    // edits the archive file.
    let r = built();
    commit_record(&r, &["acceptor"], "accepted");
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("finish");
    let (_, out, err) = run(&r, &["check"]);
    assert!(!has(&out, "SD935"), "{out}{err}");
    r.write(ARCHIVE, &format!("{}# edited\n", r.read(ARCHIVE)));
    r.commit("edit the archive file");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD935"), "{out}");
}

#[test]
fn an_archive_file_with_no_record_in_any_folder_gives_sd935() {
    // The record was never in the folder: a person wrote the archive file.
    let r = base_flow();
    let t = record_text(&r, &["acceptor"], "accepted");
    finished_by_hand(&r, Some(&t));
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    let line = out.lines().find(|l| l.contains(" SD935 ")).expect("SD935");
    assert!(line.starts_with(ARCHIVE), "{line}");
}

// check: SD935.

fn with_archive_on_the_base(profile: &str) -> Repo {
    let r = start(&config(profile, ""));
    r.write(".spec-dna/accepted/old-change.toml", "format = 1\n");
    r.commit("an archive file");
    origin_head(&r.dir, "feature");
    r
}

#[test]
fn a_diff_that_edits_or_deletes_an_archive_file_gives_sd935() {
    let r = with_archive_on_the_base("standard");
    r.write(
        ".spec-dna/accepted/old-change.toml",
        "format = 1\n# edited\n",
    );
    r.commit("edit an archive file");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD935"), "{out}");
    let r = with_archive_on_the_base("standard");
    r.git(&["rm", "-q", ".spec-dna/accepted/old-change.toml"]);
    r.commit("delete an archive file");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD935"), "{out}");
}

#[test]
fn a_diff_that_adds_an_archive_file_gives_no_sd935_and_lite_gives_none() {
    let r = with_archive_on_the_base("standard");
    r.write(".spec-dna/accepted/new-change.toml", "format = 1\n");
    r.commit("add an archive file");
    let (_, out, err) = run(&r, &["check"]);
    assert!(!has(&out, "SD935"), "{out}{err}");
    // `lite`: each code is off.
    let r = with_archive_on_the_base("lite");
    r.write(
        ".spec-dna/accepted/old-change.toml",
        "format = 1\n# edited\n",
    );
    r.commit("edit an archive file");
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!has(&out, "SD935"), "{out}");
}

// check: a project that does not use the record.

#[test]
fn in_lite_check_gives_no_record_code_for_a_deleted_folder_and_a_born_change() {
    let r = built_with(&config("lite", ""), INTENT_TEXT);
    finished_by_hand(&r, None);
    let (_, out, err) = run(&r, &["check"]);
    for c in CODES {
        assert!(!has(&out, c), "{c}: {out}{err}");
    }
    let r = on_base("0.8.0", &config("lite", ""), INTENT_TEXT);
    finished_by_hand(&r, None);
    let (_, out, err) = run(&r, &["check"]);
    for c in CODES {
        assert!(!has(&out, c), "{c}: {out}{err}");
    }
}

#[test]
fn a_plan_of_an_older_pin_on_the_base_gives_no_record_code_when_the_folder_is_deleted() {
    let r = on_base("0.7.1", &config("standard", ""), INTENT_TEXT);
    finished_by_hand(&r, None);
    let (_, out, err) = run(&r, &["check"]);
    for c in CODES {
        assert!(!has(&out, c), "{c}: {out}{err}");
    }
}

// CRLF (ADR 0019): with `core.autocrlf = true` the working tree has CRLF, and
// git has LF. The record is the same record, so no check reads an edit in it.

/// Sets `core.autocrlf` and writes the record of the folder with CRLF on disk.
/// `git status` stays clean, as it is with a checkout of `autocrlf`.
fn crlf_record(r: &Repo, text: &str) {
    r.git(&["config", "core.autocrlf", "true"]);
    std::fs::write(r.dir.join(RECORD), text.replace('\n', "\r\n")).unwrap();
    // The refresh of the index that a checkout does: the blob stays the LF blob.
    r.git(&["add", "-A"]);
    assert_eq!(r.git_out(&["status", "--porcelain"]).trim(), "");
}

#[test]
fn finish_writes_the_archive_with_lf_when_the_record_has_crlf_on_disk() {
    let r = built();
    let text = commit_record(&r, &["acceptor"], "accepted");
    crlf_record(&r, &text);
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    let bytes = std::fs::read(r.dir.join(ARCHIVE)).unwrap();
    assert!(!bytes.contains(&b'\r'), "the archive has a CR");
    assert_eq!(bytes, text.as_bytes());
    r.commit("finish");
    let (_, out, err) = run(&r, &["check"]);
    assert!(!has(&out, "SD935"), "{out}{err}");
}

#[test]
fn the_compare_of_the_record_and_its_archive_reads_crlf_as_lf() {
    // An archive file with CRLF on disk, and the record with LF in git.
    let r = base_flow();
    let t = commit_record(&r, &["acceptor"], "accepted");
    r.git(&["config", "core.autocrlf", "true"]);
    finished_by_hand(&r, Some(&t.replace('\n', "\r\n")));
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!has(&out, "SD935"), "{out}");
    // A real edit still gives SD935 with CRLF.
    let r = base_flow();
    let t = commit_record(&r, &["acceptor"], "accepted");
    r.git(&["config", "core.autocrlf", "true"]);
    finished_by_hand(&r, Some(&format!("{t}# edited\n").replace('\n', "\r\n")));
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD935"), "{out}");
}
