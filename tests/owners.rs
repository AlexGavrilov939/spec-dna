//! Tests of the table `[owners]` with agent roles (design 0.8.0, D5, D70 item 7
//! and D77, slice S6): the configuration and its warning for a person, the match
//! that makes a record needed, the roles of the brief, and SD933 for each role.
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

/// The configuration of a project. `owners` is the text of the table `[owners]`
/// (with its header, or empty). `change` is the text of more keys of `[change]`.
fn config(profile: &str, change: &str, owners: &str) -> String {
    format!(
        "profile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n{change}\n[rules]\nSD608 = \"off\"\nSD910 = \"off\"\n\n{owners}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n"
    )
}

fn owners(lines: &str) -> String {
    format!("[owners]\n{lines}")
}

fn base_files(r: &Repo) {
    r.write(REQ, VALVES);
    r.write(CODE, CODE_OLD);
    r.write(TEST, TEST_OLD);
    r.write("src/other.py", "y = 1\n");
    r.write("src/auth/login.py", "x = 1\n");
    r.write("docs/auth/login.md", "---\nstatus: active\n---\n# Login\n");
    r.write("README.md", "# Project\n");
}

fn change_files(r: &Repo) {
    r.write(INTENT, INTENT_TEXT);
    r.write(SPEC, SPEC_TEXT);
    r.write(PLAN, PLAN_TEXT);
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

/// A pull request that adds the plan in one commit and the code in the next.
fn built(config_text: &str) -> Repo {
    let r = start(config_text);
    change_files(&r);
    r.commit("plan");
    r.write(CODE, CODE_NEW);
    r.write(TEST, TEST_NEW);
    r.commit("code");
    r
}

fn has(out: &str, code: &str) -> bool {
    out.lines().any(|l| l.contains(&format!(" {code} ")))
}

fn finish(r: &Repo) -> (i32, String, String) {
    r.run(&["finish", SLUG])
}

fn accept(r: &Repo, args: &[&str]) -> (i32, String, String) {
    let mut a = vec!["accept", SLUG];
    a.extend_from_slice(args);
    r.run(&a)
}

fn brief(r: &Repo, extra: &[&str]) -> String {
    let mut a = vec!["--brief"];
    a.extend_from_slice(extra);
    let (code, out, err) = accept(r, &a);
    assert_eq!(code, 0, "{out}{err}");
    out
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap()
}

const ATTEMPT: &str = "\n[[attempt]]\nacceptor = 1\ntext = \"Sent a NaN value after the timeout.\"\noutcome = \"held\"\n";

/// The committed record of the default brief: `accept`, the hash of the brief, an
/// accepted result and one attempt. `roles` are the roles for `--role` (none: the
/// default roles of the brief).
fn commit_record(r: &Repo, roles: &[&str]) -> String {
    let mut args: Vec<&str> = vec!["--session", "s1"];
    for role in roles {
        args.push("--role");
        args.push(role);
    }
    let (code, out, err) = accept(r, &args);
    assert_eq!(code, 0, "{out}{err}");
    let hash = brief(
        r,
        &roles.iter().flat_map(|x| ["--role", x]).collect::<Vec<_>>(),
    )
    .lines()
    .last()
    .unwrap()
    .strip_prefix("brief ")
    .unwrap()
    .to_string();
    let t = r
        .read(RECORD)
        .replacen("brief = \"pending\"", &format!("brief = \"{hash}\""), 1)
        .replace("result = \"pending\"", "result = \"accepted\"");
    let t = format!("{t}{ATTEMPT}");
    r.write(RECORD, &t);
    r.commit("record");
    t
}

// The configuration.

#[test]
fn a_role_entry_is_valid_and_prints_no_warning() {
    let r = built(&config(
        "standard",
        "",
        &owners("\"src/auth/**\" = [\"security-reviewer\"]\n"),
    ));
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!err.contains("warning"), "{err}");
}

#[test]
fn a_person_entry_gives_one_warning_and_exit_0() {
    for entry in ["@lead", "lead@example.test"] {
        let r = built(&config(
            "standard",
            "",
            &owners(&format!("\"src/auth/**\" = [\"{entry}\"]\n")),
        ));
        let (code, out, err) = r.run(&["check"]);
        assert_eq!(code, 0, "{entry}: {out}{err}");
        let want = format!(
            "warning: [owners] \"src/auth/**\": \"{entry}\" names a person; 0.8.0 checks agent roles only, so this entry needs nothing; the persons of [owners] come in 0.8.1"
        );
        assert_eq!(
            err.matches(&want).count(),
            1,
            "{entry}: the warning is printed once\n{err}"
        );
        // The entry is no role: a changed path under the glob needs nothing.
        r.write("src/auth/login.py", "x = 2\n");
        r.commit("auth edit");
        let (code, out, err) = r.run(&["status"]);
        assert_eq!(code, 0, "{entry}: {out}{err}");
        assert_eq!(err.matches("warning: [owners]").count(), 1, "{err}");
    }
}

#[test]
fn a_person_in_the_table_needs_no_record() {
    // The diff edits a path that only a person owns: no role, so no need.
    let r = built(&config(
        "standard",
        "",
        &owners("\"src/**\" = [\"@lead\", \"lead@example.test\"]\n"),
    ));
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(".spec-dna").exists());
}

#[test]
fn an_entry_that_is_no_role_and_no_person_is_exit_3_and_names_the_form() {
    for entry in ["Security Reviewer", "@", "", "Lead", "1abc", "a b"] {
        let r = built(&config(
            "standard",
            "",
            &owners(&format!("\"src/auth/**\" = [\"{entry}\"]\n")),
        ));
        let (code, out, err) = r.run(&["check"]);
        assert_eq!(code, 3, "`{entry}`: {out}{err}");
        assert!(err.contains("owners"), "{err}");
        assert!(err.contains("[a-z][a-z0-9-]*"), "the form of a role: {err}");
    }
    // A role of more than 40 characters.
    let long = "a".repeat(41);
    let r = built(&config(
        "standard",
        "",
        &owners(&format!("\"src/auth/**\" = [\"{long}\"]\n")),
    ));
    let (code, _, err) = r.run(&["check"]);
    assert_eq!(code, 3, "{err}");
}

#[test]
fn a_bad_glob_or_a_bad_value_is_exit_3() {
    for lines in [
        "\"/src/**\" = [\"security-reviewer\"]\n",
        "\"../src/**\" = [\"security-reviewer\"]\n",
        "\"src\\\\auth\" = [\"security-reviewer\"]\n",
        "\"src/auth/**\" = \"security-reviewer\"\n",
        "\"src/auth/**\" = [1]\n",
    ] {
        let r = built(&config("standard", "", &owners(lines)));
        let (code, out, err) = r.run(&["check"]);
        assert_eq!(code, 3, "{lines}: {out}{err}");
        assert!(err.contains("owners"), "{lines}: {err}");
    }
    let r = built(&config("standard", "", "owners = [\"a\"]\n"));
    let (code, _, err) = r.run(&["check"]);
    assert_eq!(code, 3, "{err}");
}

// The need.

#[test]
fn a_matched_role_makes_a_record_needed_for_a_routine_change_in_standard() {
    let cfg = config(
        "standard",
        "",
        &owners("\"src/**\" = [\"security-reviewer\"]\n"),
    );
    let r = built(&cfg);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
    assert!(
        out.contains("security-reviewer"),
        "the role is named: {out}"
    );
    // Without the table, the same change needs none.
    let r = built(&config("standard", "", ""));
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_matched_role_needs_a_record_also_with_acceptance_for_none() {
    let r = built(&config(
        "standard",
        "acceptance-for = \"none\"\n",
        &owners("\"src/**\" = [\"security-reviewer\"]\n"),
    ));
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

#[test]
fn a_glob_that_matches_no_code_path_of_the_diff_needs_no_record() {
    // A glob of Markdown only, and a glob of a path that the diff does not touch.
    for glob in ["docs/auth/**", "src/auth/**"] {
        let r = built(&config(
            "standard",
            "",
            &owners(&format!("\"{glob}\" = [\"security-reviewer\"]\n")),
        ));
        r.write(
            "docs/auth/login.md",
            "---\nstatus: active\n---\n# Login\n\nEdit.\n",
        );
        r.commit("docs edit");
        let (code, out, err) = finish(&r);
        assert_eq!(code, 0, "{glob}: {out}{err}");
    }
}

#[test]
fn a_branch_cannot_drop_a_glob_that_the_base_has() {
    let base_cfg = config(
        "standard",
        "",
        &owners("\"src/**\" = [\"security-reviewer\"]\n"),
    );
    let r = start(&base_cfg);
    // The branch removes the table in a commit of its own, then builds the change.
    r.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n{}", config("standard", "", "")),
    );
    r.commit("drop the table");
    change_files(&r);
    r.commit("plan");
    r.write(CODE, CODE_NEW);
    r.write(TEST, TEST_NEW);
    r.commit("code");
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
}

// The role paths (D70 item 7).

#[test]
fn a_path_outside_files_still_needs_its_role_and_sd913_warns() {
    let r = built(&config(
        "standard",
        "",
        &owners("\"src/other.py\" = [\"security-reviewer\"]\n"),
    ));
    r.write("src/other.py", "y = 2\n");
    r.commit("other edit");
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
    assert!(out.contains("security-reviewer"), "{out}");
}

#[test]
fn a_pull_request_with_only_finish_reads_the_paths_since_the_plan() {
    // The code merged in an earlier pull request. The pull request of `finish`
    // has no code path in `scope`, so the roles read the code paths that changed
    // since the commit that added the plan.
    let text = config(
        "standard",
        "",
        &owners("\"src/valves.py\" = [\"security-reviewer\"]\n"),
    );
    let r = Repo::new();
    r.write("spec-dna.toml", &format!("spec-dna = \"0.8.0\"\n{text}"));
    base_files(&r);
    r.commit("base");
    change_files(&r);
    r.commit("plan");
    r.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n{text}"),
    );
    r.write(CODE, CODE_NEW);
    r.write(TEST, TEST_NEW);
    r.commit("code, merged earlier");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
    assert!(out.contains("security-reviewer"), "{out}");
}

#[test]
fn a_squash_commit_that_holds_the_plan_and_the_code_still_gives_the_role() {
    // The same pull request, merged as one commit that adds the plan and the
    // code. The paths since the plan start at the first parent of that commit, as
    // the fallback of `accept` does (Q8 of the design 0.8.1).
    let text = config(
        "standard",
        "",
        &owners("\"src/valves.py\" = [\"security-reviewer\"]\n"),
    );
    let r = Repo::with_config(&text);
    base_files(&r);
    r.commit("base");
    change_files(&r);
    r.write(CODE, CODE_NEW);
    r.write(TEST, TEST_NEW);
    r.commit("the squash commit");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD927"), "{out}");
    assert!(out.contains("security-reviewer"), "{out}");
}

// The roles of the brief and the record.

#[test]
fn the_brief_names_each_matched_role_by_default_sorted() {
    let r = built(&config(
        "standard",
        "",
        &owners(
            "\"src/**\" = [\"security-reviewer\"]\n\"tests/**\" = [\"db-reviewer\"]\n\"src/valves.py\" = [\"security-reviewer\"]\n",
        ),
    ));
    let text = brief(&r, &[]);
    assert_eq!(
        first_line(&text),
        format!("spec-dna-brief 2 {SLUG} db-reviewer,security-reviewer")
    );
    assert!(
        text.contains("=== roles ===\n- db-reviewer\n- security-reviewer\n"),
        "{text}"
    );
    // `--role` selects a part of the roles for one entry.
    let text = brief(&r, &["--role", "security-reviewer"]);
    assert_eq!(
        first_line(&text),
        format!("spec-dna-brief 2 {SLUG} security-reviewer")
    );
}

#[test]
fn the_brief_names_acceptor_when_no_role_matches() {
    let r = built(&config(
        "standard",
        "",
        &owners("\"docs/auth/**\" = [\"security-reviewer\"]\n"),
    ));
    let text = brief(&r, &[]);
    assert_eq!(
        first_line(&text),
        format!("spec-dna-brief 2 {SLUG} acceptor")
    );
    assert!(!text.contains("security-reviewer"), "{text}");
}

#[test]
fn the_skeleton_has_one_entry_with_each_matched_role() {
    let r = built(&config(
        "standard",
        "",
        &owners("\"src/**\" = [\"security-reviewer\"]\n\"tests/**\" = [\"db-reviewer\"]\n"),
    ));
    let (code, out, err) = accept(&r, &["--session", "s1"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.read(RECORD);
    assert!(
        t.contains(
            "[[acceptor]]\nroles = [\"db-reviewer\", \"security-reviewer\"]\nbrief = \"pending\"\n"
        ),
        "{t}"
    );
    assert_eq!(t.matches("[[acceptor]]").count(), 1, "{t}");
}

// SD933: each matched role needs an entry.

#[test]
fn a_record_with_only_acceptor_gives_sd933_for_the_role() {
    let r = built(&config(
        "standard",
        "",
        &owners("\"src/**\" = [\"security-reviewer\"]\n"),
    ));
    commit_record(&r, &["acceptor"]);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD933"), "{out}");
    assert!(out.contains("security-reviewer"), "{out}");
}

#[test]
fn an_entry_with_the_role_satisfies_it() {
    let r = built(&config(
        "standard",
        "",
        &owners("\"src/**\" = [\"security-reviewer\"]\n"),
    ));
    commit_record(&r, &[]);
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!has(&out, "SD933"), "{out}");
}

#[test]
fn one_entry_with_2_roles_answers_both_globs() {
    let r = built(&config(
        "standard",
        "",
        &owners(
            "\"src/**\" = [\"db-reviewer\", \"security-reviewer\"]\n\"tests/**\" = [\"db-reviewer\"]\n",
        ),
    ));
    let t = commit_record(&r, &[]);
    assert!(
        t.contains("roles = [\"db-reviewer\", \"security-reviewer\"]"),
        "{t}"
    );
    let (code, out, err) = finish(&r);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!has(&out, "SD933"), "{out}");
}

#[test]
fn the_compare_of_the_brief_reads_the_roles_of_the_entry() {
    // The first line of the brief holds the roles of the entry, and SD928
    // computes the brief again from the record: no false SD928 for a role entry.
    let r = built(&config(
        "standard",
        "",
        &owners("\"src/**\" = [\"security-reviewer\"]\n"),
    ));
    commit_record(&r, &[]);
    let (code, out, err) = r.run(&["check", "--base", "origin/HEAD"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!has(&out, "SD928"), "{out}");
}

#[test]
fn check_gives_sd933_for_a_role_where_the_diff_deletes_the_folder() {
    let r = built(&config(
        "standard",
        "",
        &owners("\"src/**\" = [\"security-reviewer\"]\n"),
    ));
    let t = commit_record(&r, &["acceptor"]);
    // The commit of `finish` by hand: the folder goes, and the same bytes move to
    // the archive. `check` reads the archive file.
    r.git(&["rm", "-q", "-r", "docs/intent/valve-timeout"]);
    r.write(".spec-dna/accepted/valve-timeout.toml", &t);
    r.commit("finish");
    let (code, out, err) = r.run(&["check", "--base", "origin/HEAD"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(has(&out, "SD933"), "{out}");
    assert!(out.contains("security-reviewer"), "{out}");
}

#[test]
fn init_reconfigure_keeps_the_table_owners() {
    // The table is a gate. A rewrite of the file must not drop it.
    let r = built(&config(
        "standard",
        "",
        &owners("\"src/auth/**\" = [\"security-reviewer\", \"@lead\"]\n"),
    ));
    let (code, out, err) = r.run(&["init", "--reconfigure", "--dry-run", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let text = out.split("--- spec-dna.toml ---").nth(1).unwrap_or("");
    assert!(text.contains("[owners]"), "{out}");
    assert!(
        text.contains("\"src/auth/**\" = [\"security-reviewer\", \"@lead\"]"),
        "{out}"
    );
}

#[test]
fn upgrade_prints_the_warning_for_a_person_once_and_keeps_the_table() {
    let text = config(
        "standard",
        "",
        &owners("\"src/auth/**\" = [\"security-reviewer\", \"@lead\"]\n"),
    );
    let r = Repo::new();
    r.write("spec-dna.toml", &format!("spec-dna = \"0.7.0\"\n{text}"));
    r.write("docs/a.md", "---\nstatus: active\n---\n# A\n");
    r.commit("base");
    for args in [&["upgrade", "--dry-run"][..], &["upgrade"][..]] {
        let (code, out, err) = r.run(args);
        assert_eq!(code, 0, "{args:?}: {out}{err}");
        assert_eq!(
            err.matches("warning: [owners]").count(),
            1,
            "{args:?}: {err}"
        );
        assert!(!out.contains("-[owners]"), "{args:?}: {out}");
    }
    assert!(r.read("spec-dna.toml").contains("[owners]"));
}
