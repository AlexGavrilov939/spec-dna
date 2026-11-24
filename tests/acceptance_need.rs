//! Tests of the rule "a change that needs a record" (design 0.8.0, D70 and
//! ADR 0032): the key `[change] acceptance-for`, the class of the change for the
//! acceptance (the head, the base tree, the commits of the branch and SD917), and
//! the configuration of the need (the stricter of the head and the base tree).
//! Where the rule needs the table `[owners]` (slice S6), the test is not here.
//! The fixtures use example names only.

mod common;

use common::*;

const SLUG: &str = "valve-timeout";
const FOLDER: &str = "docs/intent/valve-timeout";
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

fn higher_risk(text: &str) -> String {
    text.replace("class: routine", "class: higher-risk")
}

fn no_class(text: &str) -> String {
    text.replace("class: routine\n", "")
}

/// The configuration of the project. `change` is the text of more keys of
/// `[change]`, and `rules` the text of more rules.
fn config(profile: &str, change: &str, rules: &str) -> String {
    format!(
        "profile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n{change}\n[rules]\nSD608 = \"off\"\n{rules}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n"
    )
}

fn pin_line(pin: &str, text: &str) -> String {
    format!("spec-dna = \"{pin}\"\n{text}")
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

fn add_code(r: &Repo) {
    r.write(CODE, CODE_NEW);
    r.write(TEST, TEST_NEW);
    r.commit("code");
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

/// A pull request that adds the plan in one commit and the code in the next: the
/// plan is new.
fn built(config_text: &str, intent: &str) -> Repo {
    let r = start(config_text);
    change_files(&r, intent);
    r.commit("plan");
    add_code(&r);
    r
}

/// The plan and the intent are on the base, added at the pin 0.8.0. The branch
/// adds the code.
fn on_base(config_text: &str, intent: &str) -> Repo {
    let r = Repo::new();
    r.write("spec-dna.toml", &pin_line("0.8.0", config_text));
    base_files(&r);
    change_files(&r, intent);
    r.commit("plan");
    r.write("spec-dna.toml", &pin_line(VERSION, config_text));
    r.write("README.md", "# Project, with the new pin\n");
    r.commit("move the pin");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    add_code(&r);
    r
}

/// Edits `spec-dna.toml` in a commit of its own.
fn edit_config(r: &Repo, from: &str, to: &str) {
    let t = r.read("spec-dna.toml");
    assert!(t.contains(from), "{from}");
    r.write("spec-dna.toml", &t.replacen(from, to, 1));
    r.commit("edit the configuration");
}

fn has(out: &str, code: &str) -> bool {
    out.lines().any(|l| l.contains(&format!(" {code} ")))
}

fn finish(r: &Repo) -> (i32, String, String) {
    r.run(&["finish", SLUG])
}

// The key and the profile default.

#[test]
fn a_routine_change_in_standard_needs_no_record_and_next_names_the_review_of_0_7_1() {
    let r = built(&config("standard", "", ""), INTENT_TEXT);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(".spec-dna").exists(), "no archive file");
    // `next` names the review of 0.7.1 and no `accept`. Run it before `finish`.
    let r = built(&config("standard", "", ""), INTENT_TEXT);
    let (code, out, err) = r.run(&["next", SLUG, "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"skill\":\"review\""), "{out}");
    assert!(!out.contains("\"skill\":\"accept\""), "{out}");
    let (code, out, err) = r.run(&["next", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("[skill: accept]"), "{out}");
    assert!(
        out.lines()
            .any(|l| l.ends_with("[skill: review]") && l.contains("review the diff against")),
        "{out}"
    );
}

#[test]
fn a_higher_risk_change_in_standard_needs_a_record() {
    let r = built(&config("standard", "", ""), &higher_risk(INTENT_TEXT));
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
    let (code, out, err) = r.run(&["next", SLUG]);
    assert_eq!(code, 0, "{out}{err}");
    // One item `review` holds the record item (D71 item 2).
    assert!(!out.contains("[skill: accept]"), "{out}");
    assert!(
        out.lines()
            .any(|l| l.ends_with("[skill: review]") && l.contains("spec-dna accept")),
        "{out}"
    );
}

#[test]
fn a_routine_change_in_strict_needs_a_record() {
    let r = built(&config("strict", "", ""), INTENT_TEXT);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

#[test]
fn acceptance_for_all_in_standard_needs_a_record_for_a_routine_change() {
    let r = built(
        &config("standard", "acceptance-for = \"all\"\n", ""),
        INTENT_TEXT,
    );
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

#[test]
fn acceptance_for_none_needs_no_record_for_a_higher_risk_change() {
    let r = built(
        &config("standard", "acceptance-for = \"none\"\n", ""),
        &higher_risk(INTENT_TEXT),
    );
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
    // In `strict`, the key `none` is a choice of the project.
    let r = built(
        &config("strict", "acceptance-for = \"none\"\n", ""),
        INTENT_TEXT,
    );
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn acceptance_for_higher_risk_in_strict_needs_no_record_for_a_routine_change() {
    let r = built(
        &config("strict", "acceptance-for = \"higher-risk\"\n", ""),
        INTENT_TEXT,
    );
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_bad_value_of_the_key_is_exit_code_3_with_the_values() {
    for bad in ["\"routine\"", "\"\"", "\"All\"", "true", "1"] {
        let r = built(
            &config("standard", &format!("acceptance-for = {bad}\n"), ""),
            INTENT_TEXT,
        );
        for args in [["check"], ["status"]] {
            let (code, _, err) = r.run(&args);
            assert_eq!(code, 3, "{bad}: {err}");
            assert!(
                err.contains("`change.acceptance-for` must be"),
                "{bad}: {err}"
            );
        }
    }
    let r = built(
        &config("standard", "acceptance-for = \"sometimes\"\n", ""),
        INTENT_TEXT,
    );
    let (_, _, err) = r.run(&["check"]);
    assert!(
        err.contains("\"none\", \"higher-risk\" or \"all\", not `sometimes`"),
        "{err}"
    );
}

#[test]
fn in_lite_the_codes_are_off_and_a_rule_for_the_group_uses_the_default_higher_risk() {
    // `lite` with no rule: no record, also for a higher-risk change.
    let r = built(&config("lite", "", ""), &higher_risk(INTENT_TEXT));
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
    // A rule for the group turns the codes on, and the default of the key is
    // `higher-risk` in `lite`: a routine change needs no record.
    let text = config("lite", "", "acceptance = \"error\"\n");
    let r = built(&text, INTENT_TEXT);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
    let r = built(&text, &higher_risk(INTENT_TEXT));
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

// The class of the change for the acceptance (D70 items 2 to 4).

#[test]
fn the_base_has_higher_risk_and_the_branch_sets_routine_so_a_record_is_needed() {
    let r = on_base(&config("standard", "", ""), &higher_risk(INTENT_TEXT));
    r.write(INTENT, INTENT_TEXT);
    r.commit("set the class to routine");
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

#[test]
fn a_commit_of_the_branch_with_higher_risk_and_a_later_commit_with_routine_needs_a_record() {
    let r = start(&config("standard", "", ""));
    change_files(&r, &higher_risk(INTENT_TEXT));
    r.commit("plan");
    r.write(INTENT, INTENT_TEXT);
    r.commit("set the class to routine");
    add_code(&r);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

#[test]
fn an_intent_with_no_class_needs_a_record() {
    let r = built(&config("standard", "", ""), &no_class(INTENT_TEXT));
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

#[test]
fn a_routine_intent_and_a_path_of_high_risk_paths_needs_a_record() {
    let text = config("standard", "high-risk-paths = [\"src/valves.py\"]\n", "");
    let r = built(&text, INTENT_TEXT);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
    // The same project, with a path that no change touches.
    let text = config("standard", "high-risk-paths = [\"src/other.py\"]\n", "");
    let r = built(&text, INTENT_TEXT);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_change_that_the_base_has_as_routine_needs_no_record() {
    // A downgrade that a person merged into the base: no commit of the branch
    // changes the class.
    let r = on_base(&config("standard", "", ""), INTENT_TEXT);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(".spec-dna").exists(), "no archive file");
}

#[test]
fn a_higher_risk_change_whose_folder_the_branch_deletes_with_no_record_gives_sd927_in_check() {
    // `check` reads the same class for a folder that the diff deletes (D70 item
    // 4): the head has no intent, the base tree has `higher-risk`.
    let r = on_base(&config("standard", "", ""), &higher_risk(INTENT_TEXT));
    r.git(&["rm", "-q", "-r", FOLDER]);
    r.commit("finish with no record");
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
    // A routine change that the base has: no record.
    let r = on_base(&config("standard", "", ""), INTENT_TEXT);
    r.git(&["rm", "-q", "-r", FOLDER]);
    r.commit("finish a routine change");
    let (_, out, err) = r.run(&["check"]);
    assert!(!has(&out, "SD927"), "{out}{err}");
}

#[test]
fn a_higher_risk_change_needs_the_acceptors_of_its_base_class_also_when_the_branch_lowers_it() {
    // SD933 reads the class of the acceptance: the branch sets `routine`, the
    // base has `higher-risk`, `higher-risk-acceptors` is 2, and the record has 1
    // entry.
    let text = config("standard", "higher-risk-acceptors = 2\n", "");
    let r = on_base(&text, &higher_risk(INTENT_TEXT));
    r.write(INTENT, INTENT_TEXT);
    r.commit("set the class to routine");
    let (code, out, err) = r.run(&["accept", SLUG, "--session", "s1"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = r.run(&["accept", SLUG, "--brief"]);
    assert_eq!(code, 0, "{out}{err}");
    let hash = out.lines().last().unwrap().strip_prefix("brief ").unwrap();
    let t = r
        .read(RECORD)
        .replacen("brief = \"pending\"", &format!("brief = \"{hash}\""), 1)
        .replace("result = \"pending\"", "result = \"accepted\"");
    r.write(
        RECORD,
        &format!(
            "{t}\n[[attempt]]\nacceptor = 1\ntext = \"Sent a NaN value.\"\noutcome = \"held\"\n"
        ),
    );
    r.commit("record");
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD933"), "{out}");
}

// The configuration of the need (D70 item 6): the stricter of the head and the
// base tree.

/// The record of a change as the acceptor leaves it, with the values of the
/// skeleton (`pending`). The record is committed alone.
fn commit_skeleton(r: &Repo) {
    let (code, out, err) = r.run(&["accept", SLUG, "--session", "s1"]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("a skeleton");
}

#[test]
fn a_branch_that_sets_acceptance_for_none_still_needs_a_record_for_a_higher_risk_change() {
    let r = on_base(&config("standard", "", ""), &higher_risk(INTENT_TEXT));
    edit_config(
        &r,
        "gate = \"block\"\n",
        "gate = \"block\"\nacceptance-for = \"none\"\n",
    );
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

#[test]
fn a_branch_that_sets_the_profile_lite_still_needs_a_record_for_a_higher_risk_change() {
    let r = on_base(&config("standard", "", ""), &higher_risk(INTENT_TEXT));
    edit_config(&r, "profile = \"standard\"", "profile = \"lite\"");
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

#[test]
fn a_branch_that_turns_the_group_off_still_needs_a_record_and_the_checks_of_the_base_run() {
    let r = on_base(&config("standard", "", ""), &higher_risk(INTENT_TEXT));
    // The skeleton has the value `pending`. If the floor reached SD927 only, the
    // head would turn off SD928 and `finish` would go on.
    commit_skeleton(&r);
    edit_config(
        &r,
        "SD608 = \"off\"\n",
        "SD608 = \"off\"\nacceptance = \"off\"\n",
    );
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD928"), "{out}");
    assert!(r.dir.join(RECORD).exists(), "nothing is archived");
    // With no record at all, the need stops with SD927.
    let r = on_base(&config("standard", "", ""), &higher_risk(INTENT_TEXT));
    edit_config(
        &r,
        "SD608 = \"off\"\n",
        "SD608 = \"off\"\nacceptance = \"off\"\n",
    );
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
    // `check` reads the same floor for the record that the diff adds.
    let r = on_base(&config("standard", "", ""), &higher_risk(INTENT_TEXT));
    edit_config(
        &r,
        "SD608 = \"off\"\n",
        "SD608 = \"off\"\nacceptance = \"off\"\n",
    );
    commit_skeleton(&r);
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD928"), "{out}");
}

#[test]
fn a_base_with_acceptance_off_leaves_the_head_alone() {
    // The base has `lite`, and the head `standard`: the head turns the acceptance
    // on, and a higher-risk change needs a record.
    let r = on_base(&config("lite", "", ""), &higher_risk(INTENT_TEXT));
    edit_config(&r, "profile = \"lite\"", "profile = \"standard\"");
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

#[test]
fn a_base_configuration_that_does_not_parse_gives_the_strictest_values() {
    // The base has an unknown key, so it does not parse. The head repairs it. A
    // routine change then needs a record, as with `acceptance-for = "all"`.
    let bad = config("standard", "mood = 1\n", "");
    let r = on_base(&bad, INTENT_TEXT);
    edit_config(&r, "mood = 1\n", "");
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

#[test]
fn a_base_with_no_configuration_leaves_the_head_alone() {
    // A pull request that adopts spec-dna: the base tree has no `spec-dna.toml`.
    let r = Repo::new();
    base_files(&r);
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write(
        "spec-dna.toml",
        &pin_line(VERSION, &config("standard", "", "")),
    );
    change_files(&r, INTENT_TEXT);
    r.commit("adopt spec-dna, with a routine change");
    add_code(&r);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
    // The head decides: with the key `all`, the change needs a record.
    let r = Repo::new();
    base_files(&r);
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    let all = config("standard", "acceptance-for = \"all\"\n", "");
    r.write("spec-dna.toml", &pin_line(VERSION, &all));
    change_files(&r, INTENT_TEXT);
    r.commit("adopt spec-dna");
    add_code(&r);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

#[test]
fn a_pull_request_that_lowers_the_key_alone_and_merges_changes_the_next_change() {
    // The base has the lower value after the merge, so the next change reads it.
    let text = config("standard", "acceptance-for = \"none\"\n", "");
    let r = on_base(&text, &higher_risk(INTENT_TEXT));
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_branch_that_turns_off_one_code_of_the_group_still_gets_the_check_of_the_base() {
    // The head turns off SD928 alone, and the base has it on. The need is on, and
    // the gate follows the base, so the skeleton with its `pending` values does not
    // pass.
    let r = on_base(&config("standard", "", ""), &higher_risk(INTENT_TEXT));
    commit_skeleton(&r);
    edit_config(
        &r,
        "SD608 = \"off\"\n",
        "SD608 = \"off\"\nSD928 = \"off\"\n",
    );
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD928"), "{out}");
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD928"), "{out}");
}

#[test]
fn a_shallow_clone_that_cuts_the_branch_makes_a_routine_change_need_a_record() {
    // The class reads the commits of the branch. A shallow clone cuts them, so the
    // class is `higher-risk` as the stricter result, and SD927 names the cause and
    // `fetch-depth: 0`. The change is `routine` and the profile is `standard`:
    // with the full history, no record is needed.
    let src = on_base(&config("standard", "", ""), INTENT_TEXT);
    src.write(CODE, &format!("{CODE_NEW}\n\ndef more():\n    pass\n"));
    src.commit("more code");
    let (code, out, err) = finish(&src);
    assert_eq!(code, 0, "the full history needs no record: {out}{err}");
    src.git(&["reset", "-q", "--hard", "HEAD"]);
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
    let (code, out, err) = finish(&dst);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
    assert!(
        out.contains("cuts the branch") && out.contains("fetch-depth: 0"),
        "{out}"
    );
}

#[test]
fn a_deleted_folder_reads_the_newest_intent_so_an_intent_with_no_class_needs_a_record_in_check() {
    // D70 item 4: where the diff deletes the folder, the newest intent of the change
    // (the last commit of the branch that has the folder, else the base tree) takes
    // the place of the head intent. An absent class there is `higher-risk`, so
    // `check` reads the class that `finish` read. The plan is new: the branch adds
    // the change and deletes the folder with no record.
    let r = start(&config("standard", "", ""));
    change_files(&r, &no_class(INTENT_TEXT));
    r.commit("plan");
    add_code(&r);
    r.git(&["rm", "-q", "-r", FOLDER]);
    r.commit("finish with no record");
    let (code, out, err) = r.run(&["check"]);
    assert!(has(&out, "SD927"), "{code}: {out}{err}");
    // The history ratchet counts an explicit `higher-risk` only: a `routine` intent
    // that no commit marks needs no record.
    let r = start(&config("standard", "", ""));
    change_files(&r, INTENT_TEXT);
    r.commit("plan");
    add_code(&r);
    r.git(&["rm", "-q", "-r", FOLDER]);
    r.commit("finish a routine change");
    let (_, out, err) = r.run(&["check"]);
    assert!(!has(&out, "SD927"), "{out}{err}");
    // An intent of the base tree with no class counts too.
    let r = on_base(&config("standard", "", ""), &no_class(INTENT_TEXT));
    r.git(&["rm", "-q", "-r", FOLDER]);
    r.commit("finish with no record");
    let (_, out, err) = r.run(&["check"]);
    assert!(has(&out, "SD927"), "{out}{err}");
}

// A plan of a version before 0.8.0 (design 0.8.1, D86). The plan needs no
// record. The note names the prompt of the acceptor, and a record in the folder
// of the change ends the exemption.

/// The plan and the intent are on the base, added when `spec-dna.toml` pinned
/// `pin`. The branch adds the code.
fn on_old_base(pin: &str, config_text: &str, intent: &str) -> Repo {
    let r = Repo::new();
    r.write("spec-dna.toml", &pin_line(pin, config_text));
    base_files(&r);
    change_files(&r, intent);
    r.commit("plan");
    r.write("spec-dna.toml", &pin_line(VERSION, config_text));
    r.write("README.md", "# Project, with the new pin\n");
    r.commit("move the pin");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    add_code(&r);
    r
}

/// Writes an accepted record with the real brief, and commits it alone.
fn commit_accepted_record(r: &Repo) {
    let (code, out, err) = r.run(&["accept", SLUG, "--session", "s1"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = r.run(&["accept", SLUG, "--brief"]);
    assert_eq!(code, 0, "{out}{err}");
    let hash = out.lines().last().unwrap().strip_prefix("brief ").unwrap();
    let t = r
        .read(RECORD)
        .replacen("brief = \"pending\"", &format!("brief = \"{hash}\""), 1)
        .replace("result = \"pending\"", "result = \"accepted\"");
    r.write(
        RECORD,
        &format!(
            "{t}\n[[attempt]]\nacceptor = 1\ntext = \"Sent a NaN value.\"\noutcome = \"held\"\n"
        ),
    );
    r.commit("record");
}

/// The note of an old plan for the slug `valve-timeout`, in 2 parts: the text
/// up to the root, and the text after it.
const OLD_NOTE_HEAD: &str = "note: `docs/intent/valve-timeout/plan.md` was added when spec-dna.toml pinned 0.7.1, before 0.8.0, so the change needs no acceptance record; to opt in, start one agent that did not build the change with the prompt `cd '";
const OLD_NOTE_TAIL: &str =
    "' && spec-dna accept valve-timeout --brief`: its record ends the exemption";

#[test]
fn an_old_plan_with_no_record_keeps_the_exemption_and_the_note_names_the_prompt() {
    let r = on_old_base(
        "0.7.1",
        &config("standard", "", ""),
        &higher_risk(INTENT_TEXT),
    );
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(".spec-dna").exists(), "no archive file");
    assert!(err.contains(OLD_NOTE_HEAD), "{err}");
    assert!(err.contains(OLD_NOTE_TAIL), "{err}");
    // The root of the prompt is the root of the repository.
    let root = err
        .split(OLD_NOTE_HEAD)
        .nth(1)
        .and_then(|t| t.split(OLD_NOTE_TAIL).next())
        .unwrap();
    assert_eq!(
        std::fs::canonicalize(root).unwrap(),
        std::fs::canonicalize(&r.dir).unwrap(),
        "{err}"
    );
}

#[test]
fn a_record_in_the_folder_of_an_old_plan_ends_the_exemption() {
    // The record is fresh: step 5.2 passes, the record moves, and no note
    // prints, because the change no longer skips the record.
    let r = on_old_base(
        "0.7.1",
        &config("standard", "", ""),
        &higher_risk(INTENT_TEXT),
    );
    commit_accepted_record(&r);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!err.contains("before 0.8.0"), "{err}");
    assert!(r.dir.join(".spec-dna/accepted/valve-timeout.toml").exists());
    // The code changes after the acceptance: step 5.2 stops with SD929. Before
    // the opt-in, `finish` archived the record.
    let r = on_old_base(
        "0.7.1",
        &config("standard", "", ""),
        &higher_risk(INTENT_TEXT),
    );
    commit_accepted_record(&r);
    r.write(CODE, &format!("{CODE_NEW}\n\ndef more():\n    pass\n"));
    r.commit("edit the code after the acceptance");
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD929"), "{out}");
    assert!(!r.dir.join(".spec-dna").exists(), "nothing is archived");
}

#[test]
fn a_change_that_would_need_no_record_if_its_plan_were_new_gets_no_note() {
    // `lite`: the acceptance is off. A routine change in `standard`: no reason
    // for a record.
    let r = on_old_base("0.7.1", &config("lite", "", ""), &higher_risk(INTENT_TEXT));
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!err.contains("before 0.8.0"), "{err}");
    let r = on_old_base("0.7.1", &config("standard", "", ""), INTENT_TEXT);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!err.contains("before 0.8.0"), "{err}");
    // A plan of 0.8.0 gets no note: it needs a record.
    let r = on_base(&config("standard", "", ""), &higher_risk(INTENT_TEXT));
    let (_, out, err) = finish(&r);
    assert!(has(&out, "SD927"), "{out}{err}");
    assert!(!err.contains("before 0.8.0"), "{err}");
}
