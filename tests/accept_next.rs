//! Tests of the item `review` of `spec-dna next` for a change that needs a record
//! (design 0.8.0, D3 item 9 and D71 item 2). In the stage `deploy`, `next` has
//! one item `review`, before `finish`, with the skill `review`. For a change that
//! needs a record, that item is the record item, and it opens again after a code
//! edit. There is no item with the skill `accept`. The fixtures use example
//! names only.

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
        "profile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\nacceptance-for = \"all\"\ngate = \"block\"\n\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n"
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

/// The one item `review`, if it is the item of the record: the item of 0.7.1
/// says "review the diff against".
fn accept_line(out: &str) -> Option<&str> {
    items(out)
        .into_iter()
        .find(|l| l.ends_with("[skill: review]") && !l.contains("review the diff against"))
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
    let mut text = format!(
        "{text}\n[[attempt]]\nacceptor = 1\ntext = \"Sent a NaN value after the timeout.\"\noutcome = \"held\"\n"
    );
    // A `rejected` record has a reason (D72 item 2): a blocker.
    if result == "rejected" {
        text.push_str("\n[[finding]]\nacceptor = 1\nseverity = \"blocker\"\npath = \"src/valves.py\"\ntext = \"The timeout is not checked.\"\n");
    }
    r.write(RECORD, &text);
    r.commit("record");
}

#[test]
fn next_has_one_review_item_before_finish_and_no_item_accept() {
    let r = built("standard");
    let out = next_text(&r);
    let lines = items(&out);
    let at = |tail: &str| {
        lines
            .iter()
            .position(|l| l.ends_with(tail))
            .unwrap_or_else(|| panic!("no `{tail}` in\n{out}"))
    };
    let (verify, review, finish) = (
        at("[skill: verify]"),
        at("[skill: review]"),
        at("[skill: finish]"),
    );
    assert!(verify + 1 == review && review + 1 == finish, "{out}");
    // D71 item 2: one item `review`, not 2 items.
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.ends_with("[skill: review]"))
            .count(),
        1,
        "{out}"
    );
    assert!(!out.contains("[skill: accept]"), "{out}");
    let line = accept_line(&out).unwrap();
    assert!(line.starts_with("[ ] "), "{line}");
    assert!(
        line.contains("spec-dna accept valve-timeout --brief"),
        "{line}"
    );
    assert!(line.contains("with the prompt `cd '"), "{line}");
    // `finish` stays the last item: the next gate.
    assert!(lines.last().unwrap().ends_with("[skill: finish]"), "{out}");
}

#[test]
fn next_json_has_no_accept_skill_and_one_review_item() {
    let r = built("standard");
    let json = next_json(&r);
    let skills: Vec<&str> = json
        .split("\"skill\":\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap())
        .collect();
    assert!(!skills.contains(&"accept"), "{json}");
    assert_eq!(
        skills.iter().filter(|s| **s == "review").count(),
        1,
        "{json}"
    );
    let at = |n: &str| skills.iter().position(|s| *s == n).unwrap();
    assert!(at("review") < at("finish"), "{json}");
    assert!(json.contains("\"stage\":\"deploy\""), "{json}");
    // D71 item 2: the skill names are the 9 names of 0.7.1.
    for name in &skills {
        assert!(
            [
                "brainstorm",
                "specify",
                "clarify",
                "write-plan",
                "execute",
                "tdd",
                "review",
                "verify",
                "finish"
            ]
            .contains(name),
            "{name}"
        );
    }
}

#[test]
fn in_lite_next_has_no_accept_item() {
    let r = built("lite");
    let out = next_text(&r);
    assert!(
        accept_line(&out).is_none() && !out.contains("spec-dna accept"),
        "{out}"
    );
    // The review item of 0.7.1 stays.
    assert!(
        items(&out)
            .iter()
            .any(|l| l.ends_with("[skill: review]") && l.contains("review the diff against")),
        "{out}"
    );
    assert!(
        items(&out).last().unwrap().ends_with("[skill: finish]"),
        "{out}"
    );
}

/// Without the record item, the output is the output of 0.7.1 byte for byte: the
/// output of `lite` and the output of a plan of an earlier version have the same
/// lines, review line included. The output of `standard` has the same lines less
/// the review line, which is the record item. The id of the base commit differs in
/// each repository, so the 2nd line is out.
#[test]
fn the_record_item_replaces_only_the_review_line_and_no_need_gives_the_output_of_0_7_1() {
    let no_base = |text: &str| -> String {
        text.split_inclusive('\n')
            .filter(|l| !l.starts_with("base "))
            .collect()
    };
    let no_review = |text: &str| -> String {
        no_base(text)
            .split_inclusive('\n')
            .filter(|l| !l.trim_end().ends_with("[skill: review]"))
            .collect()
    };
    let standard = next_text(&built("standard"));
    let lite = next_text(&built("lite"));
    assert_eq!(
        no_review(&standard),
        no_review(&lite),
        "{standard}\n---\n{lite}"
    );
    // The record item is the one line `review` of `standard`.
    assert!(accept_line(&standard).is_some(), "{standard}");
    assert!(accept_line(&lite).is_none(), "{lite}");
    // A plan of an earlier version has the same lines as `lite`, with the review
    // line of 0.7.1.
    let old = next_text(&on_base("0.7.1"));
    assert_eq!(no_base(&old), no_base(&lite), "{old}\n---\n{lite}");
    let (s, l) = (next_json(&built("standard")), next_json(&built("lite")));
    assert!(!s.contains("\"skill\":\"accept\""), "{s}");
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

/// Commits the plan under the pin 0.8.0, then sets the pin of the binary in a
/// commit of its own. The sections `Invariants` and `Deviations` are read as 0.8.0
/// reads them only in a file whose adding commit has a pin of 0.8.0 or later.
fn commit_plan_at_0_8_0(r: &Repo) {
    let t = r.read("spec-dna.toml");
    let (_, rest) = t.split_once('\n').unwrap();
    r.write("spec-dna.toml", &format!("spec-dna = \"0.8.0\"\n{rest}"));
    r.commit("plan");
    r.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n{rest}"),
    );
    r.commit("move the pin");
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
        commit_plan_at_0_8_0(&r);
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

// D71 item 2 and D75 items 1 and 6, D73 item 10: the prompt line, the follow-up line
// and the open task of the acceptance.

/// The absolute root of the repository, as `next` prints it.
fn root_of(r: &Repo) -> String {
    std::fs::canonicalize(&r.dir)
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

/// The configuration with the harness at the level `lean` and the skill `review`:
/// the project has the agent `acceptor`.
fn with_agent(profile: &str) -> String {
    format!(
        "{}\n[harness]\nlevel = \"lean\"\nskills = [\"review\"]\n",
        config(profile)
    )
}

fn built_with_config(cfg: &str) -> Repo {
    let r = Repo::with_config(cfg);
    base_files(&r);
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    change_files(&r);
    r.commit("plan");
    add_code(&r);
    r
}

fn record_item(r: &Repo) -> String {
    let out = next_text(r);
    accept_line(&out)
        .unwrap_or_else(|| panic!("no record item in\n{out}"))
        .to_string()
}

#[test]
fn the_item_gives_the_prompt_line_with_the_absolute_root_and_no_word_to_add() {
    // The project has no agent `acceptor`: any agent that did not build the change.
    let r = built("standard");
    let want = format!(
        "[ ] start one agent that did not build the change with the prompt `cd '{}' && spec-dna accept valve-timeout --brief`, and add no word; it writes and commits the record [skill: review]",
        root_of(&r)
    );
    assert_eq!(record_item(&r), want);
    // The project has the agent `acceptor`.
    let r = built_with_config(&with_agent("standard"));
    let want = format!(
        "[ ] start one agent `acceptor` with the prompt `cd '{}' && spec-dna accept valve-timeout --brief`, and add no word; it writes and commits the record [skill: review]",
        root_of(&r)
    );
    assert_eq!(record_item(&r), want);
    // The text no longer asks the builder to give the whole output to an agent.
    assert!(!record_item(&r).contains("give the whole output"));
}

#[test]
fn a_root_with_a_space_and_a_quote_is_one_shell_word() {
    let mut r = built("standard");
    let old = r.dir.clone();
    let parent = old.parent().unwrap().to_path_buf();
    let new = parent.join(format!("spec dna's root {}", std::process::id()));
    std::fs::rename(&old, &new).unwrap();
    r.dir = new;
    let root = root_of(&r);
    let quoted = root.replace('\'', "'\\''");
    let item = record_item(&r);
    assert!(
        item.contains(&format!(
            "`cd '{quoted}' && spec-dna accept valve-timeout --brief`"
        )),
        "{item}"
    );
    assert!(root.contains(' ') && root.contains('\''));
}

#[test]
fn lite_with_a_rule_for_the_group_names_an_agent_that_did_not_build_the_change() {
    let cfg = config("lite").replace("SD608 = \"off\"", "SD608 = \"off\"\nacceptance = \"error\"");
    let r = built_with_config(&format!(
        "{cfg}\n[harness]\nlevel = \"lean\"\nskills = [\"review\"]\n"
    ));
    let item = record_item(&r);
    assert!(
        item.starts_with("[ ] start one agent that did not build the change with the prompt `cd '"),
        "{item}"
    );
}

/// A committed record with one note on `src/valves.py`.
fn commit_record_with_note(r: &Repo) {
    let (code, out, err) = r.run(&["accept", SLUG, "--session", "s1"]);
    assert_eq!(code, 0, "{out}{err}");
    let hash = brief_hash(r);
    let text = r
        .read(RECORD)
        .replacen("brief = \"pending\"", &format!("brief = \"{hash}\""), 1)
        .replace("result = \"pending\"", "result = \"accepted\"");
    r.write(
        RECORD,
        &format!(
            "{text}\n[[attempt]]\nacceptor = 1\ntext = \"Sent a NaN value.\"\noutcome = \"held\"\n\n[[finding]]\nacceptor = 1\nseverity = \"note\"\npath = \"src/valves.py\"\nline = 3\ntext = \"The constant has no unit.\"\n"
        ),
    );
    r.commit("record");
}

#[test]
fn a_fix_of_a_note_path_names_the_follow_up_and_any_other_edit_names_a_full_run() {
    let r = built_with_config(&with_agent("standard"));
    commit_record_with_note(&r);
    assert!(record_item(&r).starts_with("[x] "));
    // A fix of the path of the open note: the follow-up.
    r.write(CODE, &format!("{CODE_NEW}\n# the unit is seconds\n"));
    r.commit("fix");
    let item = record_item(&r);
    let want = format!(
        "[ ] the record is stale after a fix of an open note: start one agent `acceptor` with the prompt `cd '{}' && spec-dna accept valve-timeout --follow-up --brief`, and add no word; it writes and commits the follow-up [skill: review]",
        root_of(&r)
    );
    assert_eq!(item, want);
    // The item names the follow-up only when `accept --follow-up` does not refuse.
    let (code, out, err) = r.run(&["accept", SLUG, "--follow-up", "--brief"]);
    assert_eq!(code, 0, "{out}{err}");
    // Another edit, on a path that no open note names: a full run.
    let r = built_with_config(&with_agent("standard"));
    commit_record_with_note(&r);
    r.write(TEST, &format!("{TEST_NEW}\n# an edit\n"));
    r.commit("another edit");
    let item = record_item(&r);
    assert!(!item.contains("--follow-up"), "{item}");
    assert!(
        item.contains("is stale")
            && item.contains("prompt `cd '")
            && item.contains("accept valve-timeout --brief`"),
        "{item}"
    );
    let (code, _, _) = r.run(&["accept", SLUG, "--follow-up", "--brief"]);
    assert_eq!(code, 1);
}

// The open task of the acceptance (D75 item 6).

const ACCEPT_TASK: &str = "- [ ] t2: Run spec-dna accept valve-timeout from the brief.\n";

fn with_accept_task(cfg: &str, task: &str) -> Repo {
    let r = Repo::with_config(cfg);
    base_files(&r);
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write(INTENT, INTENT_TEXT);
    r.write(SPEC, SPEC_TEXT);
    let plan = PLAN_TEXT.replace("## Risks", &format!("{task}\n## Risks"));
    r.write(PLAN, &plan);
    r.commit("plan");
    add_code(&r);
    r
}

#[test]
fn an_open_task_that_names_accept_closes_with_finish_where_the_acceptance_is_on() {
    let r = with_accept_task(&config("standard"), ACCEPT_TASK);
    // `next` shows the stage `deploy`, with the task counted for `finish`.
    let (code, json, err) = r.run(&["next", SLUG, "--json"]);
    assert_eq!(code, 0, "{json}{err}");
    assert!(json.contains("\"stage\":\"deploy\""), "{json}");
    // `finish --dry-run` reaches step 5 (the record is missing), not step 0.
    let (code, out, err) = r.run(&["finish", SLUG, "--dry-run"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("step 5") && !err.contains("step 0"), "{err}");
    // With the record, `finish` passes.
    commit_record(&r, "accepted");
    let (code, out, err) = r.run(&["finish", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_task_stays_open_in_lite_and_for_another_slug() {
    // `lite`: the acceptance is off, so the task stops step 0, as in 0.7.1.
    let r = with_accept_task(&config("lite"), ACCEPT_TASK);
    let (code, out, err) = r.run(&["finish", SLUG, "--dry-run"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("step 0"), "{err}");
    let (_, json, _) = r.run(&["next", SLUG, "--json"]);
    assert!(json.contains("\"stage\":\"build\""), "{json}");
    // Another slug, and a command with another text, are no such task.
    for task in [
        "- [ ] t2: Run spec-dna accept other-change from the brief.\n",
        "- [ ] t2: Accept the change by hand.\n",
    ] {
        let r = with_accept_task(&config("standard"), task);
        let (code, out, err) = r.run(&["finish", SLUG, "--dry-run"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("step 0"), "{task}: {err}");
    }
}

#[test]
fn an_edit_that_head_does_not_have_asks_for_the_commit_before_a_follow_up() {
    let r = built_with_config(&with_agent("standard"));
    commit_record_with_note(&r);
    // The fix is not committed: `accept --follow-up` would refuse.
    r.write(CODE, &format!("{CODE_NEW}\n# the unit is seconds\n"));
    let item = record_item(&r);
    assert!(
        item.contains("is stale")
            && item.contains("commit the work, then run `spec-dna next` again"),
        "{item}"
    );
    assert!(
        !item.contains("--follow-up") && !item.contains("delete it"),
        "{item}"
    );
    let (code, _, _) = r.run(&["accept", SLUG, "--follow-up", "--brief"]);
    assert_eq!(code, 1);
}

#[test]
fn a_branch_that_turns_sd927_off_does_not_change_the_gate_of_the_open_task() {
    // The base has the acceptance on. The branch turns SD927 off in its own
    // `spec-dna.toml`. The need rule reads the stricter of the head and the base
    // (D70 item 6), so the task closes with `finish` as before, and step 5.2
    // still asks for the record.
    let r = with_accept_task(&config("standard"), ACCEPT_TASK);
    let off = r
        .read("spec-dna.toml")
        .replace("SD608 = \"off\"", "SD608 = \"off\"\nSD927 = \"off\"");
    r.write("spec-dna.toml", &off);
    r.commit("the branch turns SD927 off");
    let (code, out, err) = r.run(&["finish", SLUG, "--dry-run"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(!err.contains("step 0"), "{err}");
    let (_, json, _) = r.run(&["next", SLUG, "--json"]);
    assert!(json.contains("\"stage\":\"deploy\""), "{json}");
    // The base has the acceptance off, and the head too: the task stays open.
    let r = with_accept_task(&config("lite"), ACCEPT_TASK);
    let (_, out, err) = r.run(&["finish", SLUG, "--dry-run"]);
    assert!(err.contains("step 0"), "{out}{err}");
}
