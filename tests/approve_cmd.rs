//! Integration tests of `spec-dna approve <slug> intent|spec|plan` (owner
//! review of 0.7.0, item 2). The command sets the `status` line of one
//! artifact and commits that file alone, so the commit is an approval commit
//! of the branch rule (ADR 0012, D11).
//!
//! Each fixture is a temporary git repository with example addresses only.
//! The command makes a real commit, so each run hides the git configuration
//! of the machine (no signing, no global e-mail) and sets the author.

mod common;

use common::*;

const APPROVER: &str = "approver@example.com";
const AGENT: &str = "agent@example.com";
const STRANGER: &str = "intruder@example.com";

const INTENT: &str = "docs/intent/valve-timeout/intent.md";
const SPEC: &str = "docs/intent/valve-timeout/spec.md";
const PLAN: &str = "docs/intent/valve-timeout/plan.md";
const CODE: &str = "src/valves.py";
const TEST: &str = "tests/test_valves.py";
const REQ: &str = "docs/requirements/valves.md";

const VALVES: &str = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **overflow**: When a bed overflows, the controller SHALL close the valve of that bed.\n";
const CODE_OLD: &str = "def close_valve(bed):\n    bed.valve_open = False\n";
const CODE_NEW: &str = "def close_valve(bed):\n    bed.valve_open = False\n\n\ndef on_sensor_timeout(bed):\n    close_valve(bed)\n";
const TEST_OLD: &str = "# covers: requirements/valves#overflow\ndef test_overflow():\n    ...\n";
const TEST_NEW: &str = "# covers: requirements/valves#overflow\ndef test_overflow():\n    ...\n\n\n# covers: requirements/valves#sensor-timeout\ndef test_sensor_timeout():\n    ...\n";
const ADD: &str = "- ADD requirements/valves#sensor-timeout: When a sensor gives no value for 60 seconds, the controller SHALL close the valve of its bed.";

fn config(change: &str) -> String {
    format!(
        "profile = \"standard\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n{change}\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n"
    )
}

fn branch_keys(approvers: &[&str]) -> String {
    let list: Vec<String> = approvers.iter().map(|a| format!("\"{a}\"")).collect();
    format!("approval = \"branch\"\napprovers = [{}]\n", list.join(", "))
}

fn intent_with(front: &str) -> String {
    format!(
        "---\n{front}---\n# Valve timeout\n\n## Intent\n\nClose the valve when a sensor times out.\n\n## Why now\n\nA valve stayed open.\n\n## Out of scope\n\n- An alarm.\n"
    )
}

fn intent(status: &str, class: &str) -> String {
    intent_with(&format!("status: {status}\nclass: {class}\n"))
}

fn spec(status: &str) -> String {
    format!(
        "---\nstatus: {status}\n---\n# Valve timeout\n\n## Delta\n\n{ADD}\n\n## Interfaces\n\nNone.\n\n## Decisions\n\nNone.\n\n## Concerns\n\n- [x] A slow sensor. Resolution: 60 seconds.\n\n## Open questions\n\n- [x] Does the valve reopen? Answer: no.\n"
    )
}

fn plan(status: &str) -> String {
    format!(
        "---\nstatus: {status}\n---\n# Valve timeout\n\n## Files\n\n- `{CODE}`\n- `{TEST}`\n\n## Order\n\n- [x] t1: Close the valve. Files: `src/valves.py`. Verify: the test passes.\n\n## Risks\n\n- A slow sensor.\n\n## Proof\n\nThe test covers `requirements/valves#sensor-timeout`.\n"
    )
}

/// The base commit on `main`, `origin/HEAD` on it, and the branch `feature`.
/// The repository keeps no signing and no global configuration.
fn start(change: &str) -> Repo {
    let r = Repo::with_config(&config(change));
    r.write(REQ, VALVES);
    r.write(CODE, CODE_OLD);
    r.write(TEST, TEST_OLD);
    r.write("src/other.py", "y = 1\n");
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.git(&["config", "commit.gpgsign", "false"]);
    r.git(&["config", "user.name", "Test"]);
    r.git(&["config", "user.email", "test@example.org"]);
    r
}

fn start_branch() -> Repo {
    start(&branch_keys(&[APPROVER]))
}

/// Writes the files and commits them with the author e-mail `who`.
fn step(r: &Repo, who: &str, msg: &str, files: &[(&str, &str)]) {
    for (p, t) in files {
        r.write(p, t);
    }
    r.commit_as(msg, who);
}

/// Runs `spec-dna approve ...` as the person `who`. The environment hides the
/// git configuration of the machine.
fn approve(r: &Repo, who: &str, args: &[&str]) -> (i32, String, String) {
    let mut a = vec!["approve"];
    a.extend_from_slice(args);
    let home = r.dir.to_string_lossy().into_owned();
    r.run_env(
        &a,
        &[
            ("GIT_AUTHOR_EMAIL", who),
            ("GIT_AUTHOR_NAME", "Test"),
            ("GIT_COMMITTER_EMAIL", "committer@example.test"),
            ("GIT_COMMITTER_NAME", "Test"),
            ("GIT_CONFIG_GLOBAL", "/dev/null"),
            ("GIT_CONFIG_NOSYSTEM", "1"),
            ("HOME", &home),
        ],
        &["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"],
    )
}

fn head(r: &Repo) -> String {
    r.git_out(&["rev-parse", "HEAD"])
}

/// The paths of the last commit.
fn last_paths(r: &Repo) -> Vec<String> {
    r.git_out(&["show", "--name-only", "--format=", "HEAD"])
        .lines()
        .map(str::to_string)
        .collect()
}

/// A higher-risk change with a draft intent (the agent wrote it).
fn with_draft_intent() -> Repo {
    let r = start_branch();
    step(
        &r,
        AGENT,
        "intent draft",
        &[(INTENT, &intent("draft", "higher-risk"))],
    );
    r
}

#[test]
fn approve_sets_the_status_and_commits_it_alone() {
    let r = with_draft_intent();
    let before = head(&r);
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent"]);
    assert_eq!(code, 0, "{out}{err}");
    let short = &head(&r)[..12];
    assert!(
        out.contains(&format!("approved {INTENT} in {short} (author {APPROVER})")),
        "{out}"
    );
    assert_eq!(r.read(INTENT), intent("accepted", "higher-risk"));
    assert_eq!(last_paths(&r), [INTENT]);
    assert_eq!(
        r.git_out(&["log", "-1", "--format=%s"]),
        "docs: approve the intent of valve-timeout"
    );
    assert_eq!(r.git_out(&["log", "-1", "--format=%ae"]), APPROVER);
    assert_eq!(
        r.git_out(&["rev-list", "--count", &format!("{before}..HEAD")]),
        "1"
    );
    // The commit changes one line.
    assert_eq!(
        r.git_out(&["diff", "--numstat", "HEAD~1", "HEAD"]),
        format!("1\t1\t{INTENT}")
    );
    assert!(r.git_out(&["status", "--porcelain"]).is_empty());
}

#[test]
fn approve_commit_clears_sd910_in_branch_mode() {
    let r = start_branch();
    step(
        &r,
        AGENT,
        "intent draft",
        &[(INTENT, &intent("draft", "higher-risk"))],
    );
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent"]);
    assert_eq!(code, 0, "{out}{err}");
    step(&r, AGENT, "spec draft", &[(SPEC, &spec("draft"))]);
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "spec"]);
    assert_eq!(code, 0, "{out}{err}");
    step(&r, AGENT, "plan draft", &[(PLAN, &plan("draft"))]);
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "plan"]);
    assert_eq!(code, 0, "{out}{err}");
    step(&r, AGENT, "code", &[(CODE, CODE_NEW), (TEST, TEST_NEW)]);
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("SD910"), "{out}");
    // The 3 commits are approval commits of the approver.
    let (_, json, _) = r.run(&["check", "--json"]);
    assert_eq!(
        json.matches(&format!("\"author\":\"{APPROVER}\"")).count(),
        3,
        "{json}"
    );
}

#[test]
fn approve_keeps_other_staged_changes_out_of_the_commit() {
    let r = with_draft_intent();
    r.write("src/other.py", "y = 2\n");
    r.git(&["add", "src/other.py"]);
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(last_paths(&r), [INTENT]);
    assert_eq!(
        r.git_out(&["diff", "--cached", "--name-only"]),
        "src/other.py"
    );
}

#[test]
fn approve_keeps_a_crlf_line_end_and_a_quoted_value() {
    let r = start_branch();
    let text = intent_with("status: \"draft\"  # the owner decides\r\nclass: higher-risk\r\n")
        .replace("\n#", "\r\n#");
    r.git(&["config", "core.autocrlf", "false"]);
    step(&r, AGENT, "intent draft", &[(INTENT, &text)]);
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent"]);
    assert_eq!(code, 0, "{out}{err}");
    let want = text.replacen("\"draft\"", "\"accepted\"", 1);
    assert_eq!(r.read(INTENT), want);
    // A single-quoted value and an indented front matter keep their form.
    let r = start_branch();
    let text = intent_with("status: 'draft'\nclass: routine\n");
    step(&r, AGENT, "intent draft", &[(INTENT, &text)]);
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.read(INTENT), text.replacen("'draft'", "'accepted'", 1));
}

#[test]
fn approve_refuses_a_file_with_uncommitted_edits() {
    let msg = format!(
        "commit the text of `{INTENT}` first; an approval commit changes the `status` line alone"
    );
    // An edit in the working tree.
    let r = with_draft_intent();
    r.write(
        INTENT,
        &format!("{}\nAn extra line.\n", intent("draft", "higher-risk")),
    );
    let before = head(&r);
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains(&msg), "{err}");
    assert_eq!(head(&r), before);
    // An edit in the index.
    r.git(&["add", INTENT]);
    r.write(INTENT, &intent("draft", "higher-risk"));
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains(&msg), "{err}");
    assert_eq!(head(&r), before);
    // A file that git does not track.
    let r = start_branch();
    r.write(INTENT, &intent("draft", "higher-risk"));
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains(&msg), "{err}");
}

#[test]
fn approve_refuses_an_author_outside_approvers() {
    let r = with_draft_intent();
    let before = head(&r);
    let (code, out, err) = approve(&r, STRANGER, &["valve-timeout", "intent"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        err.contains(&format!(
            "`{STRANGER}` is not in `[change] approvers`, so the commit gives no approval"
        )),
        "{err}"
    );
    assert_eq!(head(&r), before);
    assert_eq!(r.read(INTENT), intent("draft", "higher-risk"));
    // The compare ignores the case.
    let (code, out, err) = approve(&r, "Approver@Example.com", &["valve-timeout", "intent"]);
    assert_eq!(code, 0, "{out}{err}");
    // With `approval = "base"`, the merge is the approval: no check of the author.
    let r = start(&"approval = \"base\"\n".to_string());
    step(
        &r,
        AGENT,
        "intent draft",
        &[(INTENT, &intent("draft", "higher-risk"))],
    );
    let (code, out, err) = approve(&r, STRANGER, &["valve-timeout", "intent"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn approve_refuses_a_plan_before_its_spec() {
    // A higher-risk plan needs an approved spec.
    let r = with_draft_intent();
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent"]);
    assert_eq!(code, 0, "{out}{err}");
    step(
        &r,
        AGENT,
        "spec and plan",
        &[(SPEC, &spec("draft")), (PLAN, &plan("draft"))],
    );
    let before = head(&r);
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "plan"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("spec"), "the message names the spec: {err}");
    assert_eq!(head(&r), before);
    // A spec needs an accepted intent.
    let r = with_draft_intent();
    step(&r, AGENT, "spec", &[(SPEC, &spec("draft"))]);
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "spec"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("intent"), "{err}");
    // A routine plan needs an accepted intent only.
    let r = start_branch();
    step(
        &r,
        AGENT,
        "routine",
        &[
            (INTENT, &intent("draft", "routine")),
            (PLAN, &plan("draft")),
        ],
    );
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "plan"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("intent"), "{err}");
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "plan"]);
    assert_eq!(code, 0, "a routine plan needs no spec: {out}{err}");
}

#[test]
fn approve_refuses_an_intent_with_no_class() {
    let r = start_branch();
    step(
        &r,
        AGENT,
        "intent draft",
        &[(INTENT, &intent_with("status: draft\n"))],
    );
    let before = head(&r);
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        err.contains("set the class in a commit of its own first"),
        "{err}"
    );
    assert_eq!(head(&r), before);
    // A status that is not `draft` stops too.
    let r = start_branch();
    step(
        &r,
        AGENT,
        "closed",
        &[(INTENT, &intent("closed", "routine"))],
    );
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent"]);
    assert_eq!(code, 1, "{out}{err}");
}

#[test]
fn approve_is_a_no_op_for_an_approved_artifact() {
    let r = start_branch();
    step(
        &r,
        APPROVER,
        "intent",
        &[(INTENT, &intent("accepted", "higher-risk"))],
    );
    let before = head(&r);
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains(&format!("`{INTENT}` is already `accepted`")),
        "{out}"
    );
    assert_eq!(head(&r), before, "no commit");
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"commit\":null"), "{out}");
    assert!(out.contains("\"status\":\"accepted\""), "{out}");
}

#[cfg(unix)]
#[test]
fn approve_restores_the_file_when_the_commit_fails() {
    use std::os::unix::fs::PermissionsExt;
    let r = with_draft_intent();
    let hook = r.dir.join(".git/hooks/commit-msg");
    std::fs::create_dir_all(hook.parent().unwrap()).unwrap();
    std::fs::write(
        &hook,
        "#!/bin/sh\necho 'the message is not allowed' >&2\nexit 1\n",
    )
    .unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    let before = head(&r);
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent"]);
    assert_eq!(code, 4, "{out}{err}");
    assert!(err.contains("the message is not allowed"), "{err}");
    assert_eq!(head(&r), before);
    assert_eq!(r.read(INTENT), intent("draft", "higher-risk"));
    assert!(r.git_out(&["status", "--porcelain"]).is_empty());
}

#[test]
fn approve_unknown_slug_exits_5() {
    let r = start_branch();
    let (code, out, err) = approve(&r, APPROVER, &["no-such-change", "intent"]);
    assert_eq!(code, 5, "{out}{err}");
    // A folder with no file of the artifact exits 5 too.
    let r = with_draft_intent();
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "spec"]);
    assert_eq!(code, 5, "{out}{err}");
    assert!(err.contains("spec.md"), "{err}");
}

#[test]
fn approve_unknown_artifact_exits_2() {
    let r = with_draft_intent();
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "wheel"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("`wheel` is not an artifact"), "{err}");
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("approve"), "{err}");
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent", "--nope"]);
    assert_eq!(code, 2, "{out}{err}");
}

#[test]
fn approve_json_shape() {
    let r = with_draft_intent();
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let sha = head(&r);
    assert_eq!(
        out.trim_end(),
        format!(
            "{{\"slug\":\"valve-timeout\",\"artifact\":\"intent\",\"path\":\"{INTENT}\",\"status\":\"accepted\",\"commit\":\"{sha}\",\"author\":\"{APPROVER}\"}}"
        )
    );
}

#[test]
fn approve_stops_on_a_pin_of_another_version() {
    let r = with_draft_intent();
    let cfg = r.read("spec-dna.toml").replacen(VERSION, "0.0.1", 1);
    r.write("spec-dna.toml", &cfg);
    r.commit("pin");
    let (code, out, err) = approve(&r, APPROVER, &["valve-timeout", "intent"]);
    assert_eq!(code, 3, "{out}{err}");
}
