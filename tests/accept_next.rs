//! Tests of the item `accept` of `spec-dna next` (design 0.8.0, D3 item 9 and
//! the paragraph "The agent file"). `next` names `accept` in the stage `deploy`,
//! after `review` and before `finish`, with the skill `accept`. It names it only
//! for a change that needs a record, and it names it again after a code edit.
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

fn config(profile: &str) -> String {
    format!(
        "profile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n"
    )
}

fn base_files(r: &Repo) {
    r.write(REQ, VALVES);
    r.write(CODE, CODE_OLD);
    r.write(TEST, TEST_OLD);
    r.write("README.md", "# Project\n");
}

fn change_files(r: &Repo) {
    r.write(INTENT, INTENT_TEXT);
    r.write(SPEC, SPEC_TEXT);
    r.write(PLAN, PLAN_TEXT);
}

fn add_code(r: &Repo) {
    r.write(CODE, CODE_NEW);
    r.write(TEST, TEST_NEW);
    r.commit("code");
}

/// A pull request that adds the plan in one commit and the code in the next.
/// The plan is new, so the change needs a record in `standard`.
fn built(profile: &str) -> Repo {
    let r = Repo::with_config(&config(profile));
    base_files(&r);
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    change_files(&r);
    r.commit("plan");
    add_code(&r);
    r
}

/// The plan is on the base, added when the pin was `pin`.
fn on_base(pin: &str) -> Repo {
    let r = Repo::new();
    r.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{pin}\"\n{}", config("standard")),
    );
    base_files(&r);
    change_files(&r);
    r.commit("plan");
    r.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n{}", config("standard")),
    );
    r.write("README.md", "# Project, with the new pin\n");
    r.commit("move the pin");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    add_code(&r);
    r
}

fn next_text(r: &Repo) -> String {
    let (code, out, err) = r.run(&["next", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    out
}

fn next_json(r: &Repo) -> String {
    let (code, out, err) = r.run(&["next", SLUG, "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    out
}

/// The text lines of a `next` output: no header.
fn items(out: &str) -> Vec<&str> {
    out.lines().skip(2).collect()
}

fn accept_line(out: &str) -> Option<&str> {
    items(out)
        .into_iter()
        .find(|l| l.ends_with("[skill: accept]"))
}

/// The hash of the brief of the change.
fn brief_hash(r: &Repo) -> String {
    let (code, out, err) = r.run(&["accept", SLUG, "--brief"]);
    assert_eq!(code, 0, "{out}{err}");
    out.lines()
        .last()
        .unwrap()
        .strip_prefix("brief ")
        .unwrap()
        .to_string()
}

/// A finished record with the real brief. The file is committed alone.
fn commit_record(r: &Repo, result: &str) {
    let (code, out, err) = r.run(&["accept", SLUG, "--session", "s1"]);
    assert_eq!(code, 0, "{out}{err}");
    let hash = brief_hash(r);
    let text = r
        .read(RECORD)
        .replacen("brief = \"pending\"", &format!("brief = \"{hash}\""), 1)
        .replace("result = \"pending\"", &format!("result = \"{result}\""));
    let text = format!(
        "{text}\n[[attempt]]\nacceptor = 1\ntext = \"Sent a NaN value after the timeout.\"\noutcome = \"held\"\n"
    );
    r.write(RECORD, &text);
    r.commit("record");
}

#[test]
fn next_names_accept_between_review_and_finish_with_the_skill_accept() {
    let r = built("standard");
    let out = next_text(&r);
    let lines = items(&out);
    let at = |tail: &str| {
        lines
            .iter()
            .position(|l| l.ends_with(tail))
            .unwrap_or_else(|| panic!("no `{tail}` in\n{out}"))
    };
    let (review, accept, finish) = (
        at("[skill: review]"),
        at("[skill: accept]"),
        at("[skill: finish]"),
    );
    assert!(review + 1 == accept && accept + 1 == finish, "{out}");
    let line = accept_line(&out).unwrap();
    assert!(line.starts_with("[ ] "), "{line}");
    assert!(
        line.contains("spec-dna accept valve-timeout --brief"),
        "{line}"
    );
    assert!(line.contains("`acceptor`"), "{line}");
    // `finish` stays the last item: the next gate.
    assert!(lines.last().unwrap().ends_with("[skill: finish]"), "{out}");
}

#[test]
fn next_json_gives_accept_as_the_skill_of_the_item() {
    let r = built("standard");
    let json = next_json(&r);
    let skills: Vec<&str> = json
        .split("\"skill\":\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap())
        .collect();
    let at = |n: &str| skills.iter().position(|s| *s == n).unwrap();
    assert!(
        at("review") < at("accept") && at("accept") < at("finish"),
        "{json}"
    );
    assert!(json.contains("\"stage\":\"deploy\""), "{json}");
}

#[test]
fn in_lite_next_has_no_accept_item() {
    let r = built("lite");
    let out = next_text(&r);
    assert!(
        accept_line(&out).is_none() && !out.contains("spec-dna accept"),
        "{out}"
    );
    assert!(
        items(&out).last().unwrap().ends_with("[skill: finish]"),
        "{out}"
    );
}

/// Without the item, the output is the output of 0.7.1 byte for byte: the
/// output of `standard` less the line of `accept` is the output of `lite`,
/// except for the id of the base commit, which differs in each repository.
#[test]
fn the_output_with_no_need_is_the_output_with_the_accept_line_removed() {
    // The 2nd line names the base commit, and it differs in each repository.
    let no_base = |text: &str| -> String {
        text.split_inclusive('\n')
            .filter(|l| !l.starts_with("base ") && !l.trim_end().ends_with("[skill: accept]"))
            .collect()
    };
    let standard = next_text(&built("standard"));
    let lite = next_text(&built("lite"));
    assert_eq!(
        no_base(&standard),
        no_base(&lite),
        "{standard}\n---\n{lite}"
    );
    // A plan of an earlier version has the same lines as `lite`.
    let old = next_text(&on_base("0.7.1"));
    assert_eq!(no_base(&old), no_base(&lite), "{old}\n---\n{lite}");
    let (s, l) = (next_json(&built("standard")), next_json(&built("lite")));
    assert!(s.contains("\"skill\":\"accept\""), "{s}");
    assert!(!l.contains("\"skill\":\"accept\""), "{l}");
}

#[test]
fn a_plan_that_was_added_before_0_8_0_needs_no_accept_item() {
    let r = on_base("0.7.1");
    let out = next_text(&r);
    assert!(out.contains("[skill: finish]"), "{out}");
    assert!(!out.contains("spec-dna accept"), "{out}");
    // The same plan, added at a pin of 0.8.0, needs the item.
    let r = on_base("0.8.0");
    assert!(accept_line(&next_text(&r)).is_some());
}

#[test]
fn a_fresh_accepted_record_makes_the_item_done_and_a_code_edit_opens_it_again() {
    let r = built("standard");
    commit_record(&r, "accepted");
    let out = next_text(&r);
    let line = accept_line(&out).unwrap_or_else(|| panic!("{out}"));
    assert!(line.starts_with("[x] "), "{line}");
    // A Markdown edit changes nothing.
    r.write("README.md", "# Project, edited\n");
    assert!(accept_line(&next_text(&r)).unwrap().starts_with("[x] "));
    // A code edit makes the record stale.
    r.write(CODE, &format!("{CODE_NEW}\n\ndef more():\n    pass\n"));
    r.commit("fix");
    let out = next_text(&r);
    let line = accept_line(&out).unwrap();
    assert!(line.starts_with("[ ] "), "{line}");
    assert!(line.contains("stale") && line.contains("accept"), "{line}");
}

#[test]
fn a_path_that_the_plan_adds_or_drops_after_the_record_opens_the_item_again() {
    for drop in [false, true] {
        let r = built("standard");
        commit_record(&r, "accepted");
        assert!(accept_line(&next_text(&r)).unwrap().starts_with("[x] "));
        let plan = std::fs::read_to_string(r.dir.join(PLAN)).unwrap();
        if drop {
            r.write(PLAN, &plan.replace("- `tests/test_valves.py`\n", ""));
        } else {
            r.write(
                PLAN,
                &plan.replace(
                    "- `tests/test_valves.py`\n",
                    "- `tests/test_valves.py`\n- `src/backdoor.py`\n",
                ),
            );
            r.write("src/backdoor.py", "def hidden():\n    pass\n");
        }
        r.commit("the plan changes its paths");
        let out = next_text(&r);
        let line = accept_line(&out).unwrap();
        assert!(line.starts_with("[ ] "), "{line}");
        assert!(line.contains("stale") && line.contains("accept"), "{line}");
    }
}

#[test]
fn a_rejected_record_leaves_the_item_open() {
    let r = built("standard");
    commit_record(&r, "rejected");
    let out = next_text(&r);
    let line = accept_line(&out).unwrap();
    assert!(
        line.starts_with("[ ] ") && line.contains("rejected"),
        "{line}"
    );
}

/// The item runs the checks of `finish` step 5.2: a record that `finish` stops
/// on is no done item (a wrong brief hash, SD931 and SD933).
#[test]
fn a_record_with_a_wrong_brief_leaves_the_item_open_with_the_first_problem() {
    let r = built("standard");
    commit_record(&r, "accepted");
    let text = r.read(RECORD);
    let hash = text
        .lines()
        .find_map(|l| l.strip_prefix("brief = \""))
        .unwrap()
        .trim_end_matches('"')
        .to_string();
    r.write(RECORD, &text.replace(&hash, "0123456789abcdef"));
    r.commit("a wrong brief");
    let out = next_text(&r);
    let line = accept_line(&out).unwrap();
    assert!(line.starts_with("[ ] ") && line.contains("brief"), "{line}");
}

#[test]
fn too_few_acceptors_and_a_self_accepted_record_leave_the_item_open() {
    // SD933: a higher-risk change needs 2 acceptors, and the record has 1.
    let two = config("standard").replace(
        "gate = \"block\"",
        "gate = \"block\"\nhigher-risk-acceptors = 2",
    );
    let r = Repo::with_config(&two);
    base_files(&r);
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    change_files(&r);
    r.write(
        INTENT,
        &INTENT_TEXT.replace("class: routine", "class: higher-risk"),
    );
    r.commit("plan");
    add_code(&r);
    commit_record(&r, "accepted");
    let out = next_text(&r);
    let line = accept_line(&out).unwrap();
    assert!(
        line.starts_with("[ ] ") && line.contains("acceptor entries"),
        "{line}"
    );
    // SD931: 2 entries with the same session and the same agent.
    let (code, out, err) = r.run(&["accept", SLUG, "--session", "s1"]);
    assert_eq!(code, 0, "{out}{err}");
    let hash = brief_hash(&r);
    let text = r
        .read(RECORD)
        .replace("agent = \"unknown\"", "agent = \"a1\"")
        .replacen("brief = \"pending\"", &format!("brief = \"{hash}\""), 1)
        .replace("result = \"pending\"", "result = \"accepted\"");
    let text = format!(
        "{text}\n[[attempt]]\nacceptor = 2\ntext = \"Sent a NaN value.\"\noutcome = \"held\"\n"
    );
    r.write(RECORD, &text);
    r.commit("a second entry");
    let out = next_text(&r);
    let line = accept_line(&out).unwrap();
    assert!(
        line.starts_with("[ ] ") && line.contains("same `session`"),
        "{line}"
    );
}

/// `finish` reads the record of `HEAD` (step 0 refuses an uncommitted record),
/// so `next` shows the item done only for a committed record.
#[test]
fn an_uncommitted_record_leaves_the_item_open_and_says_commit_the_record() {
    let r = built("standard");
    commit_record(&r, "accepted");
    assert!(accept_line(&next_text(&r)).unwrap().starts_with("[x] "));
    // A new edit of the committed record: the working tree differs from HEAD.
    let text = r.read(RECORD);
    r.write(RECORD, &format!("# a note\n{text}"));
    let out = next_text(&r);
    let line = accept_line(&out).unwrap();
    assert!(
        line.starts_with("[ ] ") && line.contains("commit the record"),
        "{line}"
    );
    // An untracked record: the commit of it is not made yet.
    let r = built("standard");
    commit_record(&r, "accepted");
    r.git(&["reset", "-q", "--hard", "HEAD~1"]);
    r.write(RECORD, &text);
    let out = next_text(&r);
    let line = accept_line(&out).unwrap();
    assert!(
        line.starts_with("[ ] ") && line.contains("commit the record"),
        "{line}"
    );
}

/// CRLF (ADR 0019): with `core.autocrlf = true` the record is CRLF on disk and
/// LF in `HEAD`, with a clean `git status`. It is the same record.
#[test]
fn a_record_with_crlf_on_disk_and_lf_in_head_leaves_the_item_done() {
    let r = built("standard");
    commit_record(&r, "accepted");
    r.git(&["config", "core.autocrlf", "true"]);
    let text = r.read(RECORD).replace('\n', "\r\n");
    std::fs::write(r.dir.join(RECORD), text).unwrap();
    // The refresh of the index that a checkout does: the blob stays the LF blob.
    r.git(&["add", "-A"]);
    assert_eq!(r.git_out(&["status", "--porcelain"]).trim(), "");
    let out = next_text(&r);
    let line = accept_line(&out).unwrap();
    assert!(line.starts_with("[x] "), "{line}");
}

/// `next` and `finish` share the decision of which checks of the record run. With
/// SD928 off and SD927 on, `finish` passes a record that does not read, so `next`
/// shows the item done.
#[test]
fn next_skips_the_checks_that_finish_skips() {
    let cfg = config("standard").replace("SD608 = \"off\"", "SD608 = \"off\"\nSD928 = \"off\"");
    let r = Repo::with_config(&cfg);
    base_files(&r);
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    change_files(&r);
    r.commit("plan");
    add_code(&r);
    r.write(RECORD, "this is = not a record [\n");
    r.commit("a record that does not read");
    let out = next_text(&r);
    let line = accept_line(&out).unwrap();
    assert!(line.starts_with("[x] "), "{line}");
    // The text says only what was checked: no "valid", no "accepted".
    assert!(line.contains("passes the checks that are on"), "{line}");
    assert!(
        !line.contains("valid") && !line.contains("accepted"),
        "{line}"
    );
    let (code, out, err) = r.run(&["finish", SLUG, "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    // With SD930 off, a `rejected` record is done, and the text does not say
    // that it is accepted.
    let cfg = config("standard").replace("SD608 = \"off\"", "SD608 = \"off\"\nSD930 = \"off\"");
    let r = Repo::with_config(&cfg);
    base_files(&r);
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    change_files(&r);
    r.commit("plan");
    add_code(&r);
    commit_record(&r, "rejected");
    let line = accept_line(&next_text(&r)).map(str::to_string).unwrap();
    assert!(line.starts_with("[x] "), "{line}");
    assert!(line.contains("passes the checks that are on"), "{line}");
    assert!(!line.contains("accepted"), "{line}");
    // With each check on, the text is the text of before.
    let r = built("standard");
    commit_record(&r, "accepted");
    let line = accept_line(&next_text(&r)).map(str::to_string).unwrap();
    assert!(line.contains("valid, fresh and accepted"), "{line}");
    // With SD928 on, both stop.
    let r = built("standard");
    r.write(RECORD, "this is = not a record [\n");
    r.commit("a record that does not read");
    let line = accept_line(&next_text(&r)).map(str::to_string).unwrap();
    assert!(line.starts_with("[ ] "), "{line}");
    let (code, _, _) = r.run(&["finish", SLUG, "--dry-run"]);
    assert_ne!(code, 0);
}

/// The item runs the checks of the sections too (design 0.8.0, D6 and D7): a
/// deviation that the record does not copy leaves it open, with the message of
/// SD938, and a bad item of the plan leaves it open with SD939.
#[test]
fn a_deviation_that_the_record_lacks_and_a_bad_deviation_leave_the_item_open() {
    let with_plan = |devs: &str| {
        let r = Repo::with_config(&config("standard"));
        base_files(&r);
        r.commit("base");
        origin_head(&r.dir, "main");
        r.git(&["checkout", "-q", "-b", "feature"]);
        change_files(&r);
        r.write(PLAN, &format!("{PLAN_TEXT}\n## Deviations\n\n{devs}\n"));
        r.commit("plan");
        add_code(&r);
        r
    };
    let r = with_plan("- d1 (plan): The check moved.");
    commit_record(&r, "accepted");
    // `commit_record` leaves the verdict `pending`, so the record is not valid.
    // Answer it, then remove the entry: the answer is missing.
    let t = r.read(RECORD);
    let cut = t.find("[[deviation]]").unwrap();
    let end = t[cut..].find("\n[[attempt]]").map_or(t.len(), |i| cut + i);
    r.write(RECORD, &format!("{}{}", &t[..cut], &t[end..]));
    r.commit("an answer is missing");
    let out = next_text(&r);
    let line = accept_line(&out).unwrap();
    assert!(
        line.starts_with("[ ] ") && line.contains("no entry for the deviation `d1`"),
        "{line}"
    );
    let r = with_plan("- d1: no kind");
    commit_record(&r, "accepted");
    let out = next_text(&r);
    let line = accept_line(&out).unwrap();
    assert!(
        line.starts_with("[ ] ") && line.contains("is not valid"),
        "{line}"
    );
}
