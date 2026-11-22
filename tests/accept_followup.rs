//! Tests of the follow-up of a note (design 0.8.0, D73): `accept <slug> --follow-up
//! [--brief]`, the chain of the record and its follow-ups, the rules of SD928 for a
//! `[[follow-up]]` entry, and SD929 against the tree of the last link. The fixtures
//! use example names only.

mod common;

use common::*;

const SLUG: &str = "valve-timeout";
const INTENT: &str = "docs/intent/valve-timeout/intent.md";
const SPEC: &str = "docs/intent/valve-timeout/spec.md";
const PLAN: &str = "docs/intent/valve-timeout/plan.md";
const RECORD: &str = "docs/intent/valve-timeout/acceptance.toml";
const ARCHIVE: &str = ".spec-dna/accepted/valve-timeout.toml";
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

/// A record with one note on `CODE`, a fix of `CODE` and its follow-up.
fn followed() -> Repo {
    let r = built();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    fix(&r, CODE, "\n# the unit is seconds\n");
    commit_follow_up(&r, "s2", None);
    r
}

// finish and SD929.

#[test]
fn a_fix_of_a_note_and_its_follow_up_pass_step_5_2() {
    let r = built();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    fix(&r, CODE, "\n# the unit is seconds\n");
    // Before the follow-up the record is stale (SD929).
    let (code, out, err) = run(&r, &["finish", SLUG, "--dry-run"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD929"), "{out}");
    commit_follow_up(&r, "s2", None);
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    let archive = r.read(ARCHIVE);
    assert!(archive.contains("[[follow-up]]"), "{archive}");
}

#[test]
fn a_fix_that_also_changes_another_path_of_paths_is_shown_whole_and_passes() {
    let r = built();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    r.write(CODE, &format!("{CODE_NEW}\n# the unit is seconds\n"));
    r.write(TEST, &format!("{TEST_NEW}\n# a test for the unit\n"));
    r.commit("fix of the code and of its test");
    let (code, out, err) = accept(&r, &["--follow-up", "--brief"]);
    assert_eq!(code, 0, "{out}{err}");
    let fix_part = out.split("=== fix ===\n").nth(1).unwrap();
    let fix_part = fix_part.split("=== diff ===\n").next().unwrap();
    assert!(fix_part.contains(CODE) && fix_part.contains(TEST), "{out}");
    let diff = out.split("=== diff ===\n").nth(1).unwrap();
    assert!(
        diff.contains("+# the unit is seconds") && diff.contains("+# a test for the unit"),
        "{out}"
    );
    commit_follow_up(&r, "s2", None);
    let (code, out, err) = run(&r, &["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn an_edit_after_the_follow_up_gives_sd929() {
    let r = followed();
    fix(&r, CODE, "\n# another edit\n");
    let (code, out, err) = run(&r, &["finish", SLUG, "--dry-run"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD929"), "{out}");
}

#[test]
fn a_markdown_edit_after_the_follow_up_changes_nothing() {
    let r = followed();
    r.write("README.md", "# Project, edited\n");
    r.commit("docs");
    let (code, out, err) = run(&r, &["finish", SLUG, "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
}

// The brief.

#[test]
fn the_follow_up_brief_has_the_parts_the_requirements_and_its_own_hash() {
    let r = built();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    fix(&r, CODE, "\n# the unit is seconds\n");
    let (code, b, err) = accept(&r, &["--follow-up", "--brief"]);
    assert_eq!(code, 0, "{b}{err}");
    assert!(
        b.starts_with(&format!("spec-dna-follow-up 2 {SLUG}\n")),
        "{b}"
    );
    let fixed_end = b.find("=== delta ===").unwrap();
    assert!((500..=1200).contains(&fixed_end), "{fixed_end}");
    for part in [
        "Run each command of `Proof`",
        "closes",
        "`rejected`",
        "Commit `acceptance.toml` alone.",
    ] {
        assert!(b[..fixed_end].contains(part), "{part}\n{b}");
    }
    // The delta lines, the section Proof of the plan and the open notes.
    assert!(
        b.contains("=== delta ===\n") && b.contains("ADD requirements/valves#sensor-timeout"),
        "{b}"
    );
    assert!(
        b.contains("=== proof ===\n")
            && b.contains("The test covers `requirements/valves#sensor-timeout`."),
        "{b}"
    );
    assert!(
        b.contains(&format!("=== notes ===\n1 {CODE}:3 {NOTE_TEXT}\n")),
        "{b}"
    );
    assert!(b.contains("=== trees ===\n"), "{b}");
    assert!(
        b.contains("=== fix ===\n") && b.contains("=== diff ===\n"),
        "{b}"
    );
    // The last line is the hash of the text before `=== diff ===`.
    let hash = last_line(&b).strip_prefix("brief ").unwrap();
    assert_eq!(hash.len(), 16);
    let pre = &b[..b.rfind("=== diff ===\n").unwrap()];
    assert_eq!(hash, sha16(pre.as_bytes()));
    // The same tree gives the same bytes.
    assert_eq!(b, accept(&r, &["--follow-up", "--brief"]).1);
}

fn sha16(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn the_brief_does_not_depend_on_the_diff_configuration_of_the_user() {
    let r = built();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    fix(
        &r,
        CODE,
        "\n# the unit is seconds\n\ndef more():\n    pass\n",
    );
    let plain = accept(&r, &["--follow-up", "--brief"]).1;
    r.git(&["config", "diff.algorithm", "histogram"]);
    r.git(&["config", "diff.noprefix", "true"]);
    r.git(&["config", "diff.mnemonicPrefix", "true"]);
    r.git(&["config", "diff.external", "false"]);
    r.git(&["config", "diff.context", "9"]);
    r.git(&["config", "diff.renames", "copies"]);
    let configured = accept(&r, &["--follow-up", "--brief"]);
    assert_eq!(configured.0, 0, "{}", configured.2);
    assert_eq!(plain, configured.1);
}

// The refusals of both forms.

fn refuses(r: &Repo, why: &str) {
    for extra in [&["--follow-up", "--brief"][..], &["--follow-up"][..]] {
        let (code, out, err) = accept(r, extra);
        assert_eq!(code, 1, "{out}{err}");
        let all = format!("{out}{err}");
        assert!(all.contains(why), "{why}: {all}");
        assert!(
            all.contains("full run"),
            "the message names a full run: {all}"
        );
    }
}

#[test]
fn a_record_with_no_open_note_a_rejected_record_and_a_bad_record_are_refused() {
    // No note.
    let r = built();
    commit_record(&r, &[]);
    fix(&r, CODE, "\n# edit\n");
    refuses(&r, "no open note");
    // A rejected record.
    let r = built();
    commit_record(&r, &[(CODE, "blocker", "The timeout is not checked.")]);
    fix(&r, CODE, "\n# edit\n");
    refuses(&r, "`rejected`");
    // A record that does not read, and a record of format 1.
    let r = built();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    fix(&r, CODE, "\n# edit\n");
    r.write(RECORD, "this is not toml = = =");
    refuses(&r, "does not read");
    // All notes closed by an earlier follow-up.
    let r = followed();
    fix(&r, CODE, "\n# edit 2\n");
    refuses(&r, "no open note");
}

#[test]
fn a_follow_up_with_no_record_says_that_the_change_has_no_record() {
    // D80 item 5: the file does not exist, so the message does not ask to delete it.
    let r = built();
    let want = format!(
        "the change `{SLUG}` has no record; run `spec-dna accept {SLUG} --brief` for a full run"
    );
    for extra in [
        &["--follow-up", "--brief"][..],
        &["--follow-up"][..],
        &["--follow-up", "--brief", "--json"][..],
        &["--follow-up", "--json"][..],
    ] {
        let (code, out, err) = accept(&r, extra);
        assert_eq!(code, 1, "{extra:?}: {out}{err}");
        assert!(err.contains(&want), "{extra:?}: {out}{err}");
        assert!(
            !err.contains("does not exist") && !err.contains("delete"),
            "{extra:?}: {err}"
        );
    }
    assert!(!r.dir.join(RECORD).exists());
}

#[test]
fn an_empty_diff_of_the_fix_and_a_fix_of_no_path_of_a_note_are_refused() {
    let r = built();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    // No fix yet: the diff is empty.
    refuses(&r, "empty");
    // The fix changes `TEST`, and the note is on `CODE`.
    fix(&r, TEST, "\n# edit\n");
    refuses(&r, "no path of an open note");
}

#[test]
fn a_new_file_in_scope_that_paths_does_not_list_is_refused() {
    let cfg = config("standard", "shared-paths = [\"gen/*.txt\"]\n");
    let r = built_with(&cfg, INTENT_TEXT);
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    r.write(CODE, &format!("{CODE_NEW}\n# the unit is seconds\n"));
    r.write("gen/new.txt", "x\n");
    r.commit("fix and a new file");
    refuses(&r, "gen/new.txt");
}

#[test]
fn a_binary_file_in_the_diff_of_the_fix_is_refused() {
    let r = start(&config("standard", ""));
    r.write("assets/logo.bin", "a\0b\0c\n");
    r.commit("a binary file on the base");
    change_files(&r, INTENT_TEXT);
    r.write(
        PLAN,
        &PLAN_TEXT.replace(
            "- `src/valves.py`\n",
            "- `src/valves.py`\n- `assets/logo.bin`\n",
        ),
    );
    r.commit("plan");
    r.write("assets/logo.bin", "a\0b\0d\n");
    add_code(&r);
    commit_record(&r, &[("assets/logo.bin", "note", "The logo is old.")]);
    r.write("assets/logo.bin", "a\0b\0e\n");
    r.commit("fix");
    refuses(&r, "binary");
}

#[test]
fn the_diff_of_the_whole_chain_has_one_budget_of_16384_bytes() {
    let r = built();
    commit_record(
        &r,
        &[
            (CODE, "note", "The constant has no unit."),
            (TEST, "note", "The test has no name."),
        ],
    );
    let big = format!("\n# {}\n", "x".repeat(10_000));
    fix(&r, CODE, &big);
    commit_follow_up(&r, "s2", Some("[1]"));
    // The first fix has about 10 KB. The second fix would bring the chain over the
    // limit, though each fix is under it.
    fix(&r, TEST, &big);
    refuses(&r, "16384");
}

// SD928: the rules of a follow-up.

fn finish_code(r: &Repo) -> (i32, String) {
    let (code, out, err) = run(r, &["finish", SLUG, "--dry-run"]);
    (code, format!("{out}{err}"))
}

/// The record text after `edit`, committed alone.
fn edit_record(r: &Repo, edit: impl Fn(String) -> String) {
    let t = edit(r.read(RECORD));
    r.write(RECORD, &t);
    r.commit("edit the record");
}

#[test]
fn a_follow_up_whose_tree_is_not_the_tree_of_its_commit_gives_sd928() {
    let r = followed();
    edit_record(&r, |t| {
        let at = t.rfind("tree = \"").unwrap() + 8;
        t[..at].to_string() + "0000000000000000" + &t[at + 16..]
    });
    let (code, out) = finish_code(&r);
    assert_eq!(code, 1, "{out}");
    assert!(
        has(&out, "SD928") && out.contains("follow-up 1: `tree` is"),
        "{out}"
    );
}

#[test]
fn a_follow_up_with_an_empty_closes_or_a_closes_of_a_blocker_gives_sd928() {
    let r = followed();
    edit_record(&r, |t| t.replacen("closes = [1]", "closes = []", 1));
    let (code, out) = finish_code(&r);
    assert_eq!(code, 1, "{out}");
    assert!(
        has(&out, "SD928") && out.contains("`closes` is empty"),
        "{out}"
    );
    // A number that is a blocker, and a number that no finding has.
    let r = built();
    commit_record(
        &r,
        &[(CODE, "note", NOTE_TEXT), (TEST, "note", "A test name.")],
    );
    fix(&r, CODE, "\n# edit\n");
    commit_follow_up(&r, "s2", Some("[1]"));
    edit_record(&r, |t| {
        t.replacen("severity = \"note\"", "severity = \"blocker\"", 2)
            .replacen("closes = [1]", "closes = [2]", 1)
    });
    let (_, out) = finish_code(&r);
    assert!(has(&out, "SD928") && out.contains("not a note"), "{out}");
    let r = followed();
    edit_record(&r, |t| t.replacen("closes = [1]", "closes = [7]", 1));
    let (_, out) = finish_code(&r);
    assert!(
        has(&out, "SD928") && out.contains("no such finding"),
        "{out}"
    );
}

#[test]
fn a_note_that_two_follow_ups_close_gives_sd928() {
    let r = built();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT), (CODE, "note", "A name.")]);
    fix(&r, CODE, "\n# edit\n");
    commit_follow_up(&r, "s2", Some("[1]"));
    fix(&r, CODE, "\n# edit 2\n");
    // `accept` offers note 2 only. A hand-written entry that closes note 1 again.
    commit_follow_up(&r, "s3", Some("[2, 1]"));
    let (code, out) = finish_code(&r);
    assert_eq!(code, 1, "{out}");
    assert!(
        has(&out, "SD928") && out.contains("more than once"),
        "{out}"
    );
}

#[test]
fn a_follow_up_whose_fix_changes_no_path_of_its_notes_gives_sd928() {
    let r = built();
    commit_record(
        &r,
        &[(CODE, "note", NOTE_TEXT), (TEST, "note", "A test name.")],
    );
    fix(&r, CODE, "\n# edit\n");
    // The fix changed `CODE`, and the follow-up closes the note on `TEST`.
    commit_follow_up(&r, "s2", Some("[2]"));
    let (code, out) = finish_code(&r);
    assert_eq!(code, 1, "{out}");
    assert!(
        has(&out, "SD928") && out.contains("changes no path of a note"),
        "{out}"
    );
}

#[test]
fn a_follow_up_with_the_brief_of_another_tree_gives_sd928() {
    let r = followed();
    edit_record(&r, |t| {
        let at = t.rfind("brief = \"").unwrap() + 9;
        t[..at].to_string() + "0123456789abcdef" + &t[at + 16..]
    });
    let (code, out) = finish_code(&r);
    assert_eq!(code, 1, "{out}");
    assert!(
        has(&out, "SD928") && out.contains("brief of follow-up 1"),
        "{out}"
    );
}

#[test]
fn the_tree_of_the_record_is_the_tree_of_its_own_commit_for_the_chain() {
    // The record binds the code of its commit. A record whose `tree` is not the
    // hash of the code at its own commit gives SD928 once a follow-up exists.
    let r = followed();
    edit_record(&r, |t| {
        let at = t.find("tree = \"").unwrap() + 8;
        t[..at].to_string() + "0000000000000000" + &t[at + 16..]
    });
    let (code, out) = finish_code(&r);
    assert_eq!(code, 1, "{out}");
    assert!(
        has(&out, "SD928") && out.contains("the record has `tree` 0000000000000000"),
        "{out}"
    );
}

#[test]
fn check_runs_the_same_rules_where_the_diff_adds_the_follow_up() {
    let r = base_flow();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    fix(&r, CODE, "\n# the unit is seconds\n");
    commit_follow_up(&r, "s2", None);
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 0, "{out}{err}");
    // A follow-up with an empty `closes` gives SD928 in `check`.
    edit_record(&r, |t| t.replacen("closes = [1]", "closes = []", 1));
    let (code, out, err) = run(&r, &["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD928"), "{out}");
}

#[test]
fn the_json_form_names_the_follow_up_and_the_command_refuses_a_role() {
    let r = built();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    fix(&r, CODE, "\n# the unit is seconds\n");
    let (code, out, err) = accept(&r, &["--follow-up", "--brief", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with(&format!("{{\"slug\":\"{SLUG}\",\"brief\":\"")),
        "{out}"
    );
    let (code, out, err) = accept(&r, &["--follow-up", "--session", "s2", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("\"follow_up\":1") && out.contains("\"session\":\"s2\""),
        "{out}"
    );
    let (code, _, err) = accept(&r, &["--follow-up", "--role", "x"]);
    assert_eq!(code, 2, "{err}");
}

/// A follow-up entry that an agent writes by hand, with a made-up tree and brief:
/// `accept --follow-up` refuses the fix, so only SD928 can speak.
fn hand_written(r: &Repo) {
    edit_record(r, |t| {
        format!(
            "{}\n[[follow-up]]\nbrief = \"0123456789abcdef\"\nsession = \"s2\"\nagent = \"unknown\"\ntree = \"0123456789abcdef\"\ncloses = [1]\nresult = \"accepted\"\ntext = \"Checked.\"\n",
            t.trim_end()
        )
    });
}

#[test]
fn sd928_reports_a_chain_over_the_budget_and_a_binary_file_in_a_fix() {
    let r = built();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    fix(&r, CODE, &format!("\n# {}\n", "x".repeat(20_000)));
    hand_written(&r);
    let (code, out) = finish_code(&r);
    assert_eq!(code, 1, "{out}");
    assert!(
        has(&out, "SD928") && out.contains("more than 16384 bytes"),
        "{out}"
    );
    // A binary file.
    let r = start(&config("standard", ""));
    r.write("assets/logo.bin", "a\0b\0c\n");
    r.commit("a binary file on the base");
    change_files(&r, INTENT_TEXT);
    r.write(
        PLAN,
        &PLAN_TEXT.replace(
            "- `src/valves.py`\n",
            "- `src/valves.py`\n- `assets/logo.bin`\n",
        ),
    );
    r.commit("plan");
    r.write("assets/logo.bin", "a\0b\0d\n");
    add_code(&r);
    commit_record(&r, &[("assets/logo.bin", "note", "The logo is old.")]);
    r.write("assets/logo.bin", "a\0b\0e\n");
    r.commit("fix");
    hand_written(&r);
    let (code, out) = finish_code(&r);
    assert_eq!(code, 1, "{out}");
    assert!(has(&out, "SD928") && out.contains("binary file"), "{out}");
}

#[test]
fn the_local_git_configuration_changes_neither_the_diff_text_nor_its_byte_count() {
    // The edit is in the middle of the file, so the diff has blank context lines
    // (`diff.suppressBlankEmpty`) and an `index` line (`core.abbrev`).
    let r = built();
    commit_record(&r, &[(CODE, "note", NOTE_TEXT)]);
    let t = r.read(CODE);
    r.write(
        CODE,
        &t.replacen(
            "bed.valve_open = False\n",
            "bed.valve_open = False  # closed\n",
            1,
        ),
    );
    r.commit("fix");
    let plain = accept(&r, &["--follow-up", "--brief"]);
    assert_eq!(plain.0, 0, "{}", plain.2);
    for (k, v) in [
        ("core.abbrev", "40"),
        ("diff.suppressBlankEmpty", "true"),
        ("diff.interHunkContext", "5"),
        ("diff.wsErrorHighlight", "all"),
        ("core.whitespace", "trailing-space,space-before-tab"),
        ("diff.ignoreSubmodules", "all"),
        ("diff.colorMoved", "zebra"),
        ("core.quotepath", "false"),
        ("diff.indentHeuristic", "false"),
    ] {
        r.git(&["config", k, v]);
    }
    let configured = accept(&r, &["--follow-up", "--brief"]);
    assert_eq!(configured.0, 0, "{}", configured.2);
    assert_eq!(plain.1, configured.1);
    assert_eq!(plain.1.len(), configured.1.len());
}
