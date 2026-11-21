//! Integration tests of the approval in the branch (ADR 0012, design 0.7.0,
//! D11 to D17): `[change] approval = "branch"` makes `check` walk the commits
//! of `base..HEAD`. An approval commit of an approver counts before the work
//! that it approves.
//!
//! Each fixture is a real git repository. A commit has the e-mail of its
//! author (`GIT_AUTHOR_EMAIL`), as the lab scripts of the design have it. The
//! tests use example addresses only, and they never read the git
//! configuration of the machine.
//!
//! The 5 variants of the lab model (`ok`, `late`, `stranger`, `mixed` and
//! `backdoor`) build a higher-risk change in one branch. The other tests give
//! the stops of D13, the case of D16, the cases of an approval commit, the
//! output in branch mode and the squash replays of the design.

mod common;

use common::*;

const APPROVER: &str = "approver@example.com";
const NOREPLY: &str = "1234+approver@users.noreply.github.com";
const AGENT: &str = "agent@example.com";
const STRANGER: &str = "intruder@example.com";

const DIR: &str = "docs/intent/valve-timeout";
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
    config_rules(change, "")
}

/// The configuration with more `[rules]` lines.
fn config_rules(change: &str, rules: &str) -> String {
    format!(
        "profile = \"standard\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n{change}\n[rules]\nSD608 = \"off\"\n{rules}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n"
    )
}

/// The keys of `[change]` that turn the branch rule on.
fn branch_keys(approvers: &[&str]) -> String {
    let list: Vec<String> = approvers.iter().map(|a| format!("\"{a}\"")).collect();
    format!("approval = \"branch\"\napprovers = [{}]\n", list.join(", "))
}

fn intent(status: &str, class: &str) -> String {
    format!(
        "---\nstatus: {status}\nclass: {class}\n---\n# Valve timeout\n\n## Intent\n\nClose the valve when a sensor times out.\n\n## Why now\n\nA valve stayed open.\n\n## Out of scope\n\n- An alarm.\n"
    )
}

fn spec(status: &str) -> String {
    format!(
        "---\nstatus: {status}\n---\n# Valve timeout\n\n## Delta\n\n{ADD}\n\n## Interfaces\n\nNone.\n\n## Decisions\n\nNone.\n\n## Concerns\n\n- [x] A slow sensor. Resolution: 60 seconds.\n\n## Open questions\n\n- [x] Does the valve reopen? Answer: no.\n"
    )
}

fn plan(status: &str, files: &[&str]) -> String {
    let items: Vec<String> = files.iter().map(|f| format!("- `{f}`")).collect();
    format!(
        "---\nstatus: {status}\n---\n# Valve timeout\n\n## Files\n\n{}\n\n## Order\n\n- [x] t1: Close the valve. Files: `src/valves.py`. Verify: the test passes.\n\n## Risks\n\n- A slow sensor.\n\n## Proof\n\nThe test covers `requirements/valves#sensor-timeout`.\n",
        items.join("\n")
    )
}

const FILES: [&str; 2] = [CODE, TEST];

/// The base commit on `main`, `origin/HEAD` on it, and the branch `feature`.
fn start(change: &str) -> Repo {
    let r = Repo::with_config(&config(change));
    r.write(REQ, VALVES);
    r.write(CODE, CODE_OLD);
    r.write(TEST, TEST_OLD);
    r.write("src/other.py", "y = 1\n");
    r.write("README.md", "# Valves\n");
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r
}

/// The default fixture: one approver.
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

fn step_intent_draft(r: &Repo) {
    step(
        r,
        AGENT,
        "intent draft",
        &[(INTENT, &intent("draft", "higher-risk"))],
    );
}
fn step_intent_approved(r: &Repo, who: &str) {
    step(
        r,
        who,
        "approve intent",
        &[(INTENT, &intent("accepted", "higher-risk"))],
    );
}
fn step_spec_draft(r: &Repo) {
    step(r, AGENT, "spec draft", &[(SPEC, &spec("draft"))]);
}
fn step_spec_approved(r: &Repo, who: &str) {
    step(r, who, "approve spec", &[(SPEC, &spec("approved"))]);
}
fn step_plan_draft(r: &Repo) {
    step(r, AGENT, "plan draft", &[(PLAN, &plan("draft", &FILES))]);
}
fn step_plan_approved(r: &Repo, who: &str) {
    step(r, who, "approve plan", &[(PLAN, &plan("approved", &FILES))]);
}
fn step_code(r: &Repo) {
    step(r, AGENT, "code", &[(CODE, CODE_NEW), (TEST, TEST_NEW)]);
}

/// The commits of the lab variant `ok` up to and including the approval of the
/// spec (4 commits).
fn first_four(r: &Repo) {
    step_intent_draft(r);
    step_intent_approved(r, APPROVER);
    step_spec_draft(r);
    step_spec_approved(r, APPROVER);
}

/// The 7 commits of the lab variant `ok`: a higher-risk change in one branch.
fn ok_flow(r: &Repo) {
    first_four(r);
    step_plan_draft(r);
    step_plan_approved(r, APPROVER);
    step_code(r);
}

/// The variants of the lab model.
fn lab(variant: &str) -> Repo {
    let r = start_branch();
    let who = if variant == "stranger" {
        STRANGER
    } else {
        APPROVER
    };
    step_intent_draft(&r);
    if variant == "mixed" {
        // The approval commit of the intent also edits the text.
        let text = format!("{}\nAn extra line.\n", intent("accepted", "higher-risk"));
        step(&r, who, "approve intent", &[(INTENT, &text)]);
    } else {
        step_intent_approved(&r, who);
    }
    step_spec_draft(&r);
    step_spec_approved(&r, who);
    step_plan_draft(&r);
    if variant != "late" {
        step_plan_approved(&r, who);
    }
    step_code(&r);
    if variant == "backdoor" {
        let files = ["src/valves.py", "tests/test_valves.py", "src/backdoor.py"];
        step(
            &r,
            AGENT,
            "widen the scope",
            &[
                (PLAN, &plan("approved", &files)),
                ("src/backdoor.py", "x = 1\n"),
            ],
        );
    }
    if variant == "late" {
        step_plan_approved(&r, APPROVER);
    }
    r
}

/// The SD910 findings of a text output: the path and the message.
fn findings_of(out: &str) -> Vec<(String, String)> {
    out.lines()
        .filter(|l| l.contains(": error SD910 ") || l.contains(": warning SD910 "))
        .map(|l| {
            let path = l.split(':').next().unwrap().to_string();
            let msg = l.split("gate-approval-missing: ").nth(1).unwrap_or(l);
            (path, msg.to_string())
        })
        .collect()
}

/// The number after `reason ` in a message, or 0 if the message names none.
fn reason(msg: &str) -> u8 {
    let Some(rest) = msg.split("reason ").nth(1) else {
        return 0;
    };
    rest.split(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// Runs `check` and returns the exit code, the text and the findings as the
/// sorted pairs (path, reason). The message of each finding names a reason.
fn check(r: &Repo) -> (i32, String, Vec<(String, u8)>) {
    let (code, out, err) = r.run(&["check"]);
    let mut v: Vec<(String, u8)> = findings_of(&out)
        .into_iter()
        .map(|(p, m)| (p, reason(&m)))
        .collect();
    v.sort();
    assert!(code == 0 || code == 1, "check stopped: {out}{err}");
    (code, out, v)
}

fn want(items: &[(&str, u8)]) -> Vec<(String, u8)> {
    let mut v: Vec<(String, u8)> = items.iter().map(|(p, n)| (p.to_string(), *n)).collect();
    v.sort();
    v
}

fn json(r: &Repo) -> String {
    let (code, out, err) = r.run(&["check", "--json"]);
    assert!(code == 0 || code == 1, "{out}{err}");
    out
}

/// The items of the field `approvals` of `check --json`: the text between
/// the brackets.
fn approvals_field(out: &str) -> &str {
    let rest = out
        .split("\"approvals\":[")
        .nth(1)
        .unwrap_or_else(|| panic!("no `approvals` in {out}"));
    rest.split(']').next().unwrap()
}

// The 5 variants of the lab model (design 0.7.0, section 4)

#[test]
fn variant_ok_the_approvers_approve_each_artifact_before_its_successor() {
    let r = lab("ok");
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn variant_late_the_approval_of_the_plan_comes_after_the_code() {
    let r = lab("late");
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[(CODE, 2), (TEST, 2)]), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn variant_stranger_the_approval_commits_come_from_an_author_that_is_not_an_approver() {
    let r = lab("stranger");
    let (code, out, found) = check(&r);
    // The 4 findings of the work and the 3 statuses that no approver set.
    assert_eq!(
        found,
        want(&[
            (SPEC, 3),
            (PLAN, 3),
            (CODE, 3),
            (TEST, 3),
            (INTENT, 3),
            (SPEC, 3),
            (PLAN, 3),
        ]),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
}

#[test]
fn variant_mixed_the_approval_commit_of_the_intent_also_edits_the_text() {
    let r = lab("mixed");
    let (code, out, found) = check(&r);
    // Approvals do not cascade: the spec is reported, the plan and the code
    // clear, and the intent fails the rule of the last approval.
    assert_eq!(found, want(&[(SPEC, 4), (INTENT, 4)]), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn variant_backdoor_a_later_commit_adds_a_path_to_files_and_creates_the_file() {
    let r = lab("backdoor");
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[("src/backdoor.py", 8)]), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn the_message_of_each_variant_names_its_reason_and_its_fix() {
    for (variant, n, fix) in [
        ("late", 2, "approval commit"),
        ("stranger", 3, "approvers"),
        ("mixed", 4, "`status` line"),
        ("backdoor", 8, "`Files`"),
    ] {
        let r = lab(variant);
        let (_, out, _) = check(&r);
        let found = findings_of(&out);
        assert!(!found.is_empty(), "{variant}: {out}");
        let msg = &found[0].1;
        assert!(msg.contains(&format!("reason {n}")), "{variant}: {msg}");
        assert!(msg.contains(fix), "{variant}: {msg}");
    }
}

#[test]
fn with_approval_base_the_approval_commits_do_not_count() {
    // The same branch as `ok`, with the 0.6 rule: the base tree holds no
    // approval, so the 4 items of work give SD910 with reason 1.
    let r = start("");
    ok_flow(&r);
    let (code, out, found) = check(&r);
    assert_eq!(
        found,
        want(&[(SPEC, 1), (PLAN, 1), (CODE, 1), (TEST, 1)]),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("get the approval of the earlier artifact into the base first"),
        "{out}"
    );
}

// The stops of D13

/// A new commit on `main` and the tracking ref `origin/main` on it. It is the
/// base that moved while the branch is open.
fn main_moves(r: &Repo) {
    r.git(&["checkout", "-q", "main"]);
    r.write("src/other.py", "y = 2\n");
    r.commit("main moves");
    r.git(&["update-ref", "refs/remotes/origin/main", "main"]);
    r.git(&["checkout", "-q", "feature"]);
}

#[test]
fn a_merge_of_the_base_into_the_branch_is_neutral() {
    let r = start_branch();
    first_four(&r);
    main_moves(&r);
    r.git(&["merge", "-q", "--no-ff", "-m", "merge main", "main"]);
    step_plan_draft(&r);
    step_plan_approved(&r, APPROVER);
    step_code(&r);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
    // The 3 approval commits still count.
    assert_eq!(approvals_field(&json(&r)).matches("\"commit\"").count(), 3);
}

#[test]
fn a_merge_of_another_branch_stops_the_rule() {
    let r = start_branch();
    r.git(&["checkout", "-q", "-b", "other", "main"]);
    r.write("other.txt", "other\n");
    r.commit("other work");
    r.git(&["checkout", "-q", "feature"]);
    first_four(&r);
    r.git(&["merge", "-q", "--no-ff", "-m", "merge other", "other"]);
    step_plan_draft(&r);
    step_plan_approved(&r, APPROVER);
    step_code(&r);
    let (code, out, found) = check(&r);
    // The branch counts no approval, so SD910 reports as in 0.6.
    assert_eq!(
        found,
        want(&[(SPEC, 6), (PLAN, 6), (CODE, 6), (TEST, 6)]),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
    assert_eq!(approvals_field(&json(&r)), "");
}

/// The test merge of a code host: a detached commit with the tip of the base
/// as its first parent and the head of the branch as its second parent.
fn test_merge(r: &Repo) {
    ok_flow(r);
    main_moves(r);
    r.git(&["checkout", "-q", "-b", "merge-ref", "main"]);
    r.git(&["merge", "-q", "--no-ff", "-m", "test merge", "feature"]);
}

#[test]
fn the_test_merge_of_a_host_is_neutral() {
    let r = start_branch();
    test_merge(&r);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
    assert_eq!(approvals_field(&json(&r)).matches("\"commit\"").count(), 3);
}

#[test]
fn a_test_merge_that_the_base_outgrew_is_still_neutral() {
    // The base moves again after the host made the test merge. The first
    // parent of the merge commit is an old tip, which is the merge base. This
    // differs from D13 as written ("the first parent is the base tip"): the
    // rule does not turn red when the base moves under an open pull request.
    let r = start_branch();
    test_merge(&r);
    r.git(&["checkout", "-q", "main"]);
    r.write("src/other.py", "y = 3\n");
    r.commit("main moves again");
    r.git(&["update-ref", "refs/remotes/origin/main", "main"]);
    r.git(&["checkout", "-q", "merge-ref"]);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_merge_queue_commit_stops_the_rule() {
    // The queue puts the pull request on top of another queued pull request:
    // the first parent of its merge commit is not the tip of the base.
    let r = start_branch();
    ok_flow(&r);
    r.git(&["checkout", "-q", "-b", "queued", "main"]);
    r.write("queued.txt", "queued\n");
    r.commit("queued work");
    r.git(&["checkout", "-q", "-b", "queue-1", "main"]);
    r.git(&["merge", "-q", "--no-ff", "-m", "queue entry 1", "queued"]);
    r.git(&["merge", "-q", "--no-ff", "-m", "queue entry 2", "feature"]);
    let (code, out, found) = check(&r);
    assert_eq!(
        found,
        want(&[(SPEC, 6), (PLAN, 6), (CODE, 6), (TEST, 6)]),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
}

/// A shallow clone of the test merge, 2 commits deep: the clone has the merge
/// commit and its 2 parents, and the parents have no history.
fn shallow_test_merge() -> (Repo, Repo) {
    let src = start_branch();
    test_merge(&src);
    let dst = Repo::empty();
    shallow_clone(&src.dir, &dst.dir, 2);
    let tip = src.git_out(&["rev-parse", "main"]);
    origin_head(&dst.dir, &tip);
    (src, dst)
}

#[test]
fn a_shallow_clone_with_an_incomplete_range_stops_the_rule() {
    let (_src, dst) = shallow_test_merge();
    assert_eq!(
        dst.git_out(&["rev-parse", "--is-shallow-repository"]),
        "true"
    );
    let (code, out, found) = check(&dst);
    assert_eq!(
        found,
        want(&[(SPEC, 5), (PLAN, 5), (CODE, 5), (TEST, 5)]),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
    assert!(findings_of(&out)[0].1.contains("shallow clone"), "{out}");
}

/// A depth-1 clone of the branch of `ok_flow`, with the tip of `main` fetched
/// at depth 1: the clone has no merge base.
fn depth_one_clone() -> (Repo, Repo) {
    let src = start_branch();
    ok_flow(&src);
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
    (src, dst)
}

#[test]
fn a_shallow_clone_with_no_merge_base_gives_reason_5_for_each_change_that_needs_an_approval() {
    // The base is known and the merge base is not reachable. Less history never
    // gives a pass (D13, ADR 0012).
    let (_src, dst) = depth_one_clone();
    for args in [&["check"][..], &["check", "--base", "origin/main"][..]] {
        let (code, out, err) = dst.run(args);
        let mut found: Vec<(String, u8)> = findings_of(&out)
            .into_iter()
            .map(|(p, m)| (p, reason(&m)))
            .collect();
        found.sort();
        assert_eq!(
            found,
            want(&[(SPEC, 5), (PLAN, 5), (CODE, 5), (TEST, 5)]),
            "{args:?}: {out}{err}"
        );
        assert_eq!(code, 1, "{args:?}: {out}{err}");
        assert!(findings_of(&out)[0].1.contains("shallow clone"), "{out}");
    }
}

#[test]
fn a_shallow_clone_with_no_merge_base_and_approval_base_keeps_the_note() {
    // With `approval = "base"` the rule of 0.6.1 stays: no change check.
    let src = start("");
    ok_flow(&src);
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
    let (code, out, err) = dst.run(&["check", "--base", "origin/main"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(findings_of(&out).is_empty(), "{out}");
    assert!(err.contains("no base for the change checks"), "{err}");
    assert!(!err.contains("pass --base"), "{err}");
}

#[test]
fn an_unknown_base_gives_no_change_check_and_the_note_of_0_6_1() {
    // No `origin/HEAD` and no `--base`: the note asks for a base.
    let src = start_branch();
    ok_flow(&src);
    let dst = Repo::empty();
    shallow_clone(&src.dir, &dst.dir, 1);
    dst.git(&["remote", "set-head", "origin", "-d"]);
    let (code, out, err) = dst.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(findings_of(&out).is_empty(), "{out}");
    assert!(
        err.contains("no base for the change checks; pass --base <rev>"),
        "{err}"
    );
    assert!(err.contains("shallow"), "{err}");
}

#[test]
fn the_note_of_a_base_with_no_merge_base_does_not_ask_for_the_base_that_was_passed() {
    // 2 histories that share no commit, and a full clone.
    let r = start_branch();
    r.git(&["checkout", "-q", "--orphan", "other"]);
    r.git(&["rm", "-rfq", "."]);
    r.write("x.txt", "x\n");
    r.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n{}", config("")),
    );
    r.commit("orphan");
    let (_, out, err) = r.run(&["check", "--base", "main"]);
    assert!(err.contains("no base for the change checks"), "{out}{err}");
    assert!(!err.contains("pass --base"), "{err}");
    assert!(err.contains("`main`"), "{err}");
}

#[test]
fn a_full_clone_has_no_stop() {
    // The same history, cloned in full, passes.
    let src = start_branch();
    test_merge(&src);
    let dst = Repo::empty();
    git(
        &src.dir,
        &[
            "clone",
            "-q",
            src.dir.to_str().unwrap(),
            dst.dir.to_str().unwrap(),
        ],
    );
    let tip = src.git_out(&["rev-parse", "main"]);
    origin_head(&dst.dir, &tip);
    let (code, out, found) = check(&dst);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
}

// A path cannot forge a commit of the walk

/// The files that make a header of the walk: a name that holds `\x01`, `\x02`
/// and an e-mail, a name with a line feed, and a name that starts with `\x01`.
fn crafted_names(forged: &str) -> Vec<String> {
    vec![
        forged.to_string(),
        "\u{1}only".to_string(),
        "line\nfeed.txt".to_string(),
    ]
}

#[test]
fn a_path_named_like_a_commit_header_forges_no_approval() {
    // The owner approves the plan (Z), and the bot sets it back to `draft`
    // (W). Then the bot approves it again in a commit that adds a file named
    // like the header of Z with the e-mail of the owner. The branch has no
    // approval of that status, and the code needs one.
    let r = start_branch();
    first_four(&r);
    step_plan_draft(&r);
    step_plan_approved(&r, APPROVER);
    let z = r.git_out(&["rev-parse", "HEAD"]);
    let zp = r.git_out(&["rev-parse", "HEAD~1"]);
    step(
        &r,
        AGENT,
        "plan back to draft",
        &[(PLAN, &plan("draft", &FILES))],
    );
    let forged = format!("\u{1}{z} {zp}\u{2}{APPROVER}");
    for name in crafted_names(&forged) {
        r.write(&name, "x\n");
    }
    step(
        &r,
        AGENT,
        "approve again",
        &[(PLAN, &plan("approved", &FILES))],
    );
    step_code(&r);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[(PLAN, 3), (CODE, 3), (TEST, 3)]), "{out}");
    assert_eq!(code, 1, "{out}");
    // The only approvals are the real ones: the 3 commits of the owner.
    let out = json(&r);
    assert!(
        !out.contains(&format!(
            "\"commit\":\"{z}\",\"path\":\"{PLAN}\",\"author\":\"{APPROVER}\"}},{{"
        )),
        "{out}"
    );
    assert_eq!(
        approvals_field(&out).matches("\"commit\"").count(),
        2,
        "{out}"
    );
}

#[test]
fn paths_with_odd_bytes_change_no_result_of_a_good_branch() {
    let r = start_branch();
    first_four(&r);
    step_plan_draft(&r);
    step_plan_approved(&r, APPROVER);
    let z = r.git_out(&["rev-parse", "HEAD"]);
    let forged = format!("\u{1}{z} {z}\u{2}intruder@example.com");
    for name in crafted_names(&forged) {
        r.write(&name, "x\n");
    }
    step_code(&r);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
    let out = json(&r);
    assert_eq!(
        approvals_field(&out).matches("\"commit\"").count(),
        3,
        "{out}"
    );
    assert!(!approvals_field(&out).contains("intruder"), "{out}");
}

// Work inside a merge commit

#[test]
fn code_inside_a_merge_of_the_base_comes_before_the_approval_of_the_plan() {
    // The merge of `main` also adds `src/a.txt`, the file of the plan. The
    // approval of the plan comes after the merge.
    let r = start_branch();
    let routine = |status: &str| intent(status, "routine");
    let plan_of = |status: &str| plan(status, &["src/a.txt"]);
    step(&r, AGENT, "intent", &[(INTENT, &routine("draft"))]);
    step(
        &r,
        APPROVER,
        "approve intent",
        &[(INTENT, &routine("accepted"))],
    );
    step(&r, AGENT, "plan", &[(PLAN, &plan_of("draft"))]);
    r.git(&["checkout", "-q", "main"]);
    r.write("src/other.txt", "o\n");
    r.commit_as("main moves", APPROVER);
    r.git(&["update-ref", "refs/remotes/origin/main", "main"]);
    r.git(&["checkout", "-q", "feature"]);
    r.git(&["merge", "-q", "--no-commit", "--no-ff", "main"]);
    r.write("src/a.txt", "a\n");
    r.commit_as("merge main and add the code", AGENT);
    step(
        &r,
        APPROVER,
        "approve plan",
        &[(PLAN, &plan_of("approved"))],
    );
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[("src/a.txt", 2)]), "{out}");
    assert_eq!(code, 1, "{out}");
}

// D16: a change that starts and ends in one branch

#[test]
fn a_change_added_and_finished_in_one_branch_gives_reason_7_with_approval_commits() {
    let r = start_branch();
    ok_flow(&r);
    r.git(&["rm", "-q", "-r", DIR]);
    r.commit_as("finish", AGENT);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[(INTENT, 7)]), "{out}");
    assert_eq!(code, 1, "{out}");
    assert!(
        findings_of(&out)[0].1.contains("added and finished"),
        "{out}"
    );
}

// An approval commit and the other cases

#[test]
fn an_approval_commit_that_edits_another_file_gives_no_approval() {
    let r = start_branch();
    step_intent_draft(&r);
    // The commit of the approver also edits `README.md`.
    r.write("README.md", "# Valves, changed\n");
    step_intent_approved(&r, APPROVER);
    step_spec_draft(&r);
    step_spec_approved(&r, APPROVER);
    let (_, out, found) = check(&r);
    assert_eq!(found, want(&[(SPEC, 4), (INTENT, 4)]), "{out}");
}

#[test]
fn an_approval_commit_that_edits_a_file_of_the_plan_gives_no_approval() {
    let r = start_branch();
    first_four(&r);
    step_plan_draft(&r);
    // The approval of the plan also edits the code.
    r.write(CODE, CODE_NEW);
    r.write(TEST, TEST_NEW);
    step_plan_approved(&r, APPROVER);
    let (_, out, found) = check(&r);
    assert_eq!(found, want(&[(CODE, 4), (TEST, 4), (PLAN, 4)]), "{out}");
}

#[test]
fn the_e_mail_compare_ignores_case_and_needs_an_exact_match() {
    // The list holds one case, the commits another case.
    let r = start(&branch_keys(&["Approver@Example.COM"]));
    ok_flow(&r);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
    // A longer address, and an address with the same name at another host,
    // are no approver.
    for who in ["xapprover@example.com", "approver@example.com.evil.test"] {
        let r = start_branch();
        step_intent_draft(&r);
        step_intent_approved(&r, who);
        let (_, out, found) = check(&r);
        assert_eq!(found, want(&[(INTENT, 3)]), "{who}: {out}");
    }
}

#[test]
fn only_the_author_counts_not_the_committer() {
    // A commit of the web editor of a host: the approver is the author, and
    // the committer is the host.
    let r = start_branch();
    step_intent_draft(&r);
    r.write(INTENT, &intent("accepted", "higher-risk"));
    r.commit_env(
        "approve intent",
        &[
            ("GIT_AUTHOR_EMAIL", APPROVER),
            ("GIT_COMMITTER_EMAIL", "noreply@example.org"),
        ],
    );
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
    // The approver as the committer only is no approval.
    let r = start_branch();
    step_intent_draft(&r);
    r.write(INTENT, &intent("accepted", "higher-risk"));
    r.commit_env(
        "approve intent",
        &[
            ("GIT_AUTHOR_EMAIL", AGENT),
            ("GIT_COMMITTER_EMAIL", APPROVER),
        ],
    );
    let (_, out, found) = check(&r);
    assert_eq!(found, want(&[(INTENT, 3)]), "{out}");
}

#[test]
fn a_noreply_address_in_the_list_counts() {
    let r = start(&branch_keys(&[APPROVER, NOREPLY]));
    step_intent_draft(&r);
    step_intent_approved(&r, NOREPLY);
    step_spec_draft(&r);
    step_spec_approved(&r, APPROVER);
    step_plan_draft(&r);
    step_plan_approved(&r, NOREPLY);
    step_code(&r);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
    // The list with only the other address gives no approval to the first one.
    let r = start(&branch_keys(&[APPROVER]));
    step_intent_draft(&r);
    step_intent_approved(&r, NOREPLY);
    let (_, out, found) = check(&r);
    assert_eq!(found, want(&[(INTENT, 3)]), "{out}");
}

#[test]
fn an_approval_of_a_plan_covers_the_files_that_it_has_at_the_approval_commit() {
    // The plan goes back to `draft`, gets the new path, and an approver
    // approves it again. The scope is the `Files` at that approval commit.
    let r = start_branch();
    first_four(&r);
    step_plan_draft(&r);
    step_plan_approved(&r, APPROVER);
    let files = ["src/valves.py", "tests/test_valves.py", "src/backdoor.py"];
    step(
        &r,
        AGENT,
        "plan back to draft",
        &[(PLAN, &plan("draft", &files))],
    );
    step(
        &r,
        APPROVER,
        "approve the plan again",
        &[(PLAN, &plan("approved", &files))],
    );
    step(
        &r,
        AGENT,
        "code",
        &[
            (CODE, CODE_NEW),
            (TEST, TEST_NEW),
            ("src/backdoor.py", "x = 1\n"),
        ],
    );
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_status_set_back_to_draft_takes_the_approval_away() {
    let r = start_branch();
    first_four(&r);
    step_plan_draft(&r);
    step_plan_approved(&r, APPROVER);
    step(
        &r,
        AGENT,
        "plan back to draft",
        &[(PLAN, &plan("draft", &FILES))],
    );
    step_code(&r);
    let (code, out, found) = check(&r);
    // A `draft` plan needs no approval, but the code needs an approved plan.
    // The step back names reason 9 (ADR 0019), where 0.7.0 part A gave reason 2.
    assert_eq!(found, want(&[(CODE, 9), (TEST, 9)]), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_rebase_that_reorders_the_commits_changes_the_result_not_the_dates() {
    // The commits of `late`, in the order of the commits: the code is before
    // the approval of the plan, although the approval has the older date.
    let r = start_branch();
    first_four(&r);
    step_plan_draft(&r);
    r.write(CODE, CODE_NEW);
    r.write(TEST, TEST_NEW);
    r.commit_env(
        "code",
        &[
            ("GIT_AUTHOR_EMAIL", AGENT),
            ("GIT_AUTHOR_DATE", "2026-01-03T10:00:00+00:00"),
            ("GIT_COMMITTER_DATE", "2026-01-03T10:00:00+00:00"),
        ],
    );
    r.write(PLAN, &plan("approved", &FILES));
    r.commit_env(
        "approve plan",
        &[
            ("GIT_AUTHOR_EMAIL", APPROVER),
            ("GIT_AUTHOR_DATE", "2026-01-01T10:00:00+00:00"),
            ("GIT_COMMITTER_DATE", "2026-01-01T10:00:00+00:00"),
        ],
    );
    let (_, out, found) = check(&r);
    assert_eq!(found, want(&[(CODE, 2), (TEST, 2)]), "{out}");
    // The order of the commits is the order of the ancestry. The cherry-pick
    // puts the approval before the code, with dates in the other order.
    let approve = r.git_out(&["rev-parse", "HEAD"]);
    let code = r.git_out(&["rev-parse", "HEAD~1"]);
    r.git(&["checkout", "-q", "-b", "reordered", "HEAD~2"]);
    r.git(&["cherry-pick", &approve]);
    r.git(&["cherry-pick", &code]);
    let (code_exit, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code_exit, 0, "{out}");
}

// Work that no commit holds

#[test]
fn an_uncommitted_edit_of_the_code_after_the_approval_counts_as_later_work() {
    let r = start_branch();
    first_four(&r);
    step_plan_draft(&r);
    step_plan_approved(&r, APPROVER);
    r.write(CODE, CODE_NEW);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn an_uncommitted_edit_of_the_code_before_the_approval_gives_sd910() {
    let r = start_branch();
    first_four(&r);
    step_plan_draft(&r);
    r.write(CODE, CODE_NEW);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[(CODE, 2)]), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_status_that_only_the_working_tree_sets_gives_no_finding_and_no_approval() {
    // The approver runs `check`, or the hook of `init`, before the approval
    // commit. A status that no commit holds yet gives no finding.
    let r = start_branch();
    step_intent_draft(&r);
    r.write(INTENT, &intent("accepted", "higher-risk"));
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
    assert_eq!(approvals_field(&json(&r)), "");
    r.git(&["add", INTENT]);
    let (code, out, _) = r.run(&["check", "--staged"]);
    assert!(findings_of(&out).is_empty(), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn work_that_depends_on_a_status_of_the_working_tree_gives_sd910() {
    // The plan is `approved` only in the working tree, so it approves no code.
    let r = start_branch();
    first_four(&r);
    step_plan_draft(&r);
    r.write(PLAN, &plan("approved", &FILES));
    r.write(CODE, CODE_NEW);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[(CODE, 2)]), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_merge_commit_of_an_approver_is_no_approval() {
    // Condition 1 of D11: the approver sets the status in a merge commit.
    let r = start_branch();
    step_intent_draft(&r);
    main_moves(&r);
    r.git(&["merge", "-q", "--no-commit", "--no-ff", "main"]);
    r.write(INTENT, &intent("accepted", "higher-risk"));
    r.commit_as("merge main and approve", APPROVER);
    step_spec_draft(&r);
    step_spec_approved(&r, APPROVER);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[(INTENT, 2), (SPEC, 2)]), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn an_approver_commit_that_adds_the_file_as_accepted_is_no_approval() {
    // Condition 3 of D11: the commit changes the `status` line only.
    let r = start_branch();
    step(
        &r,
        APPROVER,
        "add the intent as accepted",
        &[(INTENT, &intent("accepted", "higher-risk"))],
    );
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[(INTENT, 4)]), "{out}");
    assert_eq!(code, 1, "{out}");
}

// A draft needs no approval (design 0.7.0, section 15)

#[test]
fn a_draft_spec_and_a_draft_plan_need_no_approval_in_the_branch() {
    let r = start_branch();
    step_intent_draft(&r);
    step_spec_draft(&r);
    step_plan_draft(&r);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn the_code_needs_an_approved_plan_also_with_a_draft_plan() {
    let r = start_branch();
    step_intent_draft(&r);
    step_intent_approved(&r, APPROVER);
    step_plan_draft(&r);
    step_code(&r);
    let (_, out, found) = check(&r);
    assert_eq!(found, want(&[(CODE, 2), (TEST, 2)]), "{out}");
}

// The output in branch mode

#[test]
fn check_json_lists_the_approval_commits_that_cleared_a_gate() {
    let r = lab("ok");
    let out = json(&r);
    let items = approvals_field(&out);
    assert_eq!(items.matches("\"commit\"").count(), 3, "{out}");
    let shas: Vec<String> = ["HEAD~5", "HEAD~3", "HEAD~1"]
        .iter()
        .map(|rev| r.git_out(&["rev-parse", rev]))
        .collect();
    let mut at = 0;
    for (sha, path) in shas.iter().zip([INTENT, SPEC, PLAN]) {
        let item =
            format!("{{\"commit\":\"{sha}\",\"path\":\"{path}\",\"author\":\"{APPROVER}\"}}");
        let p = items[at..]
            .find(&item)
            .unwrap_or_else(|| panic!("`{item}` is missing or out of order in {items}"));
        at += p + item.len();
    }
}

#[test]
fn check_json_has_the_field_approvals_in_each_mode() {
    // `approval = "base"`: an empty list. The field follows `locked_links`.
    let r = start("");
    ok_flow(&r);
    let out = json(&r);
    assert_eq!(approvals_field(&out), "", "{out}");
    let a = out.find("\"locked_links\":").unwrap();
    let b = out.find("\"approvals\":").unwrap();
    let c = out.find("\"documents\":[").unwrap();
    assert!(a < b && b < c, "{out}");
    // An approval commit that no gate needs gives no item.
    let r = start_branch();
    step_intent_draft(&r);
    let out = json(&r);
    assert_eq!(approvals_field(&out), "", "{out}");
}

#[test]
fn the_last_approval_gives_an_item_also_when_no_work_depends_on_it() {
    let r = start_branch();
    step_intent_draft(&r);
    step_intent_approved(&r, APPROVER);
    let out = json(&r);
    assert_eq!(
        approvals_field(&out).matches("\"commit\"").count(),
        1,
        "{out}"
    );
}

// D7 and D8 in branch mode (the lines of `next` and `status`)

#[test]
fn next_tells_the_approver_to_commit_the_status_line_alone() {
    let r = start_branch();
    step_intent_draft(&r);
    let (code, out, err) = r.run(&["next", "valve-timeout"]);
    assert_eq!(code, 0, "{out}{err}");
    let last = out.lines().last().unwrap();
    assert!(
        last.contains("status: accepted") && last.contains("`status` line alone"),
        "{out}"
    );
    // With `approval = "base"`, the line does not change.
    let r = start("");
    step_intent_draft(&r);
    let (_, out, _) = r.run(&["next", "valve-timeout"]);
    let last = out.lines().last().unwrap();
    assert!(!last.contains("alone"), "{out}");
}

#[test]
fn status_names_the_source_of_each_approval() {
    let r = start_branch();
    first_four(&r);
    let short = r.git_out(&["rev-parse", "--short=12", "HEAD~2"]);
    let (code, out, err) = r.run(&["status", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains(&format!(
            "\"approved_in\":{{\"intent\":\"branch {short}\",\"spec\":\"branch {}\",\"plan\":null}}",
            r.git_out(&["rev-parse", "--short=12", "HEAD"])
        )),
        "{out}"
    );
    let (_, text, _) = r.run(&["status"]);
    assert!(
        text.contains(&format!("accepted (branch {short})")),
        "{text}"
    );
}

#[test]
fn status_names_base_as_the_source_of_an_approval_of_the_base_tree() {
    let r = Repo::with_config(&config(&branch_keys(&[APPROVER])));
    r.write(REQ, VALVES);
    r.write(INTENT, &intent("accepted", "higher-risk"));
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    let (code, out, err) = r.run(&["status", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("\"approved_in\":{\"intent\":\"base\",\"spec\":null,\"plan\":null}"),
        "{out}"
    );
    let (_, text, _) = r.run(&["status"]);
    assert!(text.contains("accepted (base)"), "{text}");
    // With `approval = "base"`, the text has no source.
    let r = start("");
    step_intent_draft(&r);
    step_intent_approved(&r, APPROVER);
    let (_, text, _) = r.run(&["status"]);
    assert!(
        !text.contains("(base)") && !text.contains("(branch"),
        "{text}"
    );
}

// The squash replays of the design (section 4, with the real binary)

/// A scratch repository with a bare origin, `gate = "block"` and a squash merge
/// as a code host makes it.
struct Replay {
    r: Repo,
    _origin: Repo,
}

impl Replay {
    fn new(change: &str) -> Replay {
        Replay::with_rules(change, "")
    }

    fn with_rules(change: &str, rules: &str) -> Replay {
        let origin = Repo::empty();
        git(&origin.dir, &["init", "-q", "--bare", "-b", "main"]);
        let r = Repo::with_config(&config_rules(change, rules));
        r.write(REQ, VALVES);
        r.write(CODE, CODE_OLD);
        r.write(TEST, TEST_OLD);
        r.commit("base");
        r.git(&["remote", "add", "origin", origin.dir.to_str().unwrap()]);
        r.git(&["push", "-q", "-u", "origin", "main"]);
        r.git(&["remote", "set-head", "origin", "main"]);
        Replay { r, _origin: origin }
    }

    /// `check --base origin/main`: the exit code and the SD910 findings.
    fn check(&self) -> (i32, String, Vec<(String, u8)>) {
        let (code, out, err) = self.r.run(&["check", "--base", "origin/main"]);
        assert!(code == 0 || code == 1, "{out}{err}");
        let mut found: Vec<(String, u8)> = findings_of(&out)
            .into_iter()
            .map(|(p, m)| (p, reason(&m)))
            .collect();
        found.sort();
        (code, out, found)
    }

    /// Merges the branch into `main` as one squash commit and pushes it.
    fn squash(&self, branch: &str, n: u32) {
        self.r.git(&["switch", "-q", "main"]);
        self.r.git(&["merge", "-q", "--squash", branch]);
        self.r
            .git(&["commit", "-q", "-m", &format!("{branch} (#{n})")]);
        self.r.git(&["push", "-q", "origin", "main"]);
        self.r.git(&["branch", "-q", "-D", branch]);
    }
}

/// The first pull request of a change: the artifacts and their approvals. A
/// higher-risk change also has a spec.
fn first_pull_request(r: &Repo, class: &str) {
    r.git(&["switch", "-q", "-c", "pr1"]);
    step(r, AGENT, "intent", &[(INTENT, &intent("draft", class))]);
    step(
        r,
        APPROVER,
        "approve intent",
        &[(INTENT, &intent("accepted", class))],
    );
    if class == "higher-risk" {
        step(r, AGENT, "spec", &[(SPEC, &spec("draft"))]);
        step(r, APPROVER, "approve spec", &[(SPEC, &spec("approved"))]);
    }
    let mut files = vec![CODE, TEST];
    if class == "routine" {
        files.push(REQ);
    }
    step(r, AGENT, "plan", &[(PLAN, &plan("draft", &files))]);
    step(
        r,
        APPROVER,
        "approve plan",
        &[(PLAN, &plan("approved", &files))],
    );
}

/// The second pull request: the code and `finish`.
fn second_pull_request(r: &Repo, class: &str) {
    r.git(&["switch", "-q", "-c", "pr2"]);
    r.write(CODE, CODE_NEW);
    r.write(TEST, TEST_NEW);
    if class == "routine" {
        let text = format!(
            "{VALVES}- **sensor-timeout**: When a sensor gives no value for 60 seconds, the controller SHALL close the valve of its bed.\n"
        );
        r.write(REQ, &text);
    }
    let (code, out, err) = r.run(&["finish", "valve-timeout"]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit_as("code and finish", AGENT);
}

fn replay(class: &str) {
    let rp = Replay::new(&branch_keys(&[APPROVER]));
    first_pull_request(&rp.r, class);
    let (code, out, found) = rp.check();
    assert_eq!(found, want(&[]), "pr1: {out}");
    assert_eq!(code, 0, "pr1: {out}");
    rp.squash("pr1", 1);
    second_pull_request(&rp.r, class);
    let (code, out, found) = rp.check();
    assert_eq!(found, want(&[]), "pr2: {out}");
    assert_eq!(code, 0, "pr2: {out}");
    rp.squash("pr2", 2);
    // The deletion of `intent.md` is on `main`, so `log` has the record.
    let (code, out, err) = rp.r.run(&["log"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("finished valve-timeout"), "{out}");
    assert!(out.contains(class), "{out}");
}

#[test]
fn a_routine_change_needs_2_pull_requests_with_a_squash_merge() {
    replay("routine");
}

#[test]
fn a_higher_risk_change_needs_2_pull_requests_with_a_squash_merge() {
    replay("higher-risk");
}

#[test]
fn the_same_changes_in_one_pull_request_give_one_error_reason_7() {
    for class in ["routine", "higher-risk"] {
        // The change is branch-born, so its plan is new and needs an acceptance
        // record. This test is about SD910, so the group `acceptance` is off.
        let rp = Replay::with_rules(&branch_keys(&[APPROVER]), "acceptance = \"off\"\n");
        first_pull_request(&rp.r, class);
        // `finish` in the same branch.
        rp.r.write(CODE, CODE_NEW);
        rp.r.write(TEST, TEST_NEW);
        if class == "routine" {
            let text = format!(
                "{VALVES}- **sensor-timeout**: When a sensor gives no value for 60 seconds, the controller SHALL close the valve of its bed.\n"
            );
            rp.r.write(REQ, &text);
        }
        let (code, out, err) = rp.r.run(&["finish", "valve-timeout"]);
        assert_eq!(code, 0, "{out}{err}");
        rp.r.commit_as("code and finish", AGENT);
        let (code, out, found) = rp.check();
        assert_eq!(found, want(&[(INTENT, 7)]), "{class}: {out}");
        assert_eq!(code, 1, "{class}: {out}");
    }
}

#[test]
fn the_squash_replay_with_strict_keeps_the_pull_requests_of_the_base() {
    // `profile = "strict"` reads the approvals of the base only, so the first
    // pull request of the replay still gives SD910.
    let origin = Repo::empty();
    git(&origin.dir, &["init", "-q", "--bare", "-b", "main"]);
    let r = Repo::with_config(&config("approval = \"base\"\n").replace("standard", "strict"));
    r.write(REQ, VALVES);
    r.write(CODE, CODE_OLD);
    r.write(TEST, TEST_OLD);
    r.commit("base");
    r.git(&["remote", "add", "origin", origin.dir.to_str().unwrap()]);
    r.git(&["push", "-q", "-u", "origin", "main"]);
    r.git(&["remote", "set-head", "origin", "main"]);
    first_pull_request(&r, "routine");
    let (code, out, _) = r.run(&["check", "--base", "origin/main"]);
    assert_eq!(code, 1, "{out}");
    assert!(!findings_of(&out).is_empty(), "{out}");
    assert!(
        findings_of(&out).iter().all(|(_, m)| reason(m) == 1),
        "{out}"
    );
}

#[test]
fn a_skip_label_in_an_approved_plan_gives_no_sd910() {
    // A commit of the agent adds `Hours:` and `Skip:` to the approved plan.
    // It adds no path to `Files`, so reason 8 does not apply.
    let r = start_branch();
    ok_flow(&r);
    let edited = plan("approved", &FILES).replace(
        "Verify: the test passes.",
        "Verify: the test passes. Hours: 1-2. Skip: the valve has no timer.",
    );
    assert_ne!(edited, plan("approved", &FILES));
    step(&r, AGENT, "skip the task", &[(PLAN, &edited)]);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
}

// SD926 approver-agent (owner review of 0.7.0, item 1): an item of
// `[change] approvers` that is the address of an agent or a bot.

/// The pairs (line, target) of the SD926 items of a `check --all --json`
/// output, in the order of the output.
fn sd926_items(out: &str) -> Vec<(u32, String)> {
    out.split("{\"code\":\"SD926\"")
        .skip(1)
        .map(|item| {
            let line = item
                .split("\"line\":")
                .nth(1)
                .and_then(|r| r.split(|c: char| !c.is_ascii_digit()).next())
                .and_then(|n| n.parse().ok())
                .expect("a line");
            let target = item
                .split("\"target\":\"")
                .nth(1)
                .and_then(|r| r.split('"').next())
                .expect("a target")
                .to_string();
            (line, target)
        })
        .collect()
}

/// The number of the line that starts with `approvers` in `spec-dna.toml`.
fn approvers_line(r: &Repo) -> u32 {
    let text = r.read("spec-dna.toml");
    let n = text
        .lines()
        .position(|l| l.starts_with("approvers"))
        .expect("an approvers line");
    u32::try_from(n).unwrap() + 1
}

#[test]
fn sd926_warns_on_an_agent_address_in_approvers() {
    let r = start(&branch_keys(&[APPROVER, "noreply@anthropic.com"]));
    // The item is in the base tree, so it is not new: `--all` lists it.
    let (code, all, err) = r.run(&["check", "--all", "--json"]);
    assert_eq!(code, 0, "a warning gives exit code 0: {all}{err}");
    assert_eq!(
        sd926_items(&all),
        [(approvers_line(&r), "noreply@anthropic.com".to_string())],
        "{all}"
    );
    assert!(all.contains("\"severity\":\"warning\""), "{all}");
    assert!(
        all.contains("`[change] approvers` lists `noreply@anthropic.com`, the address of an agent or a bot; a commit of this author can come from an agent, so it proves no approval of a person"),
        "{all}"
    );
    assert!(
        all.contains("Remove the address from `[change] approvers` in spec-dna.toml, in a pull request of its own (SD503). List the e-mails of the persons that approve."),
        "{all}"
    );
}

#[test]
fn sd926_warns_on_a_bot_address_in_approvers() {
    let bot_a = "41898282+github-actions[bot]@users.noreply.github.com";
    let bot_b = "dependabot[bot]@users.noreply.github.com";
    let r = start(&branch_keys(&[APPROVER, bot_a, bot_b]));
    let (code, all, err) = r.run(&["check", "--all", "--json"]);
    assert_eq!(code, 0, "{all}{err}");
    let line = approvers_line(&r);
    assert_eq!(
        sd926_items(&all),
        [(line, bot_a.to_string()), (line, bot_b.to_string())],
        "one diagnostic for each item: {all}"
    );
}

#[test]
fn sd926_is_silent_for_a_hidden_address_of_a_person() {
    // Guard: the hidden address of a person is no agent address. The same
    // fixture with an agent address warns, so the silence is the rule.
    let r = start(&branch_keys(&[
        APPROVER,
        NOREPLY,
        "bot@users.noreply.github.com",
    ]));
    let (code, all, err) = r.run(&["check", "--all", "--json"]);
    assert_eq!(code, 0, "{all}{err}");
    assert!(!all.contains("SD926"), "{all}");
    let r = start(&branch_keys(&[APPROVER, NOREPLY, "noreply@anthropic.com"]));
    let (_, all, _) = r.run(&["check", "--all", "--json"]);
    assert_eq!(sd926_items(&all).len(), 1, "{all}");
}

#[test]
fn sd926_is_off_with_policy_none() {
    let keys = branch_keys(&["noreply@anthropic.com"]);
    let r = start(&keys);
    let (_, all, _) = r.run(&["check", "--all", "--json"]);
    assert_eq!(sd926_items(&all).len(), 1, "on with the policy: {all}");
    let text = r
        .read("spec-dna.toml")
        .replace("policy = \"three-class\"", "policy = \"none\"");
    r.write("spec-dna.toml", &text);
    let (code, all, err) = r.run(&["check", "--all", "--json"]);
    assert_eq!(code, 0, "{all}{err}");
    assert!(!all.contains("SD926"), "{all}");
    // A rule of the group, or of the code, turns it off too.
    let r = start(&keys);
    let text = r
        .read("spec-dna.toml")
        .replace("[rules]\n", "[rules]\nSD926 = \"off\"\n");
    r.write("spec-dna.toml", &text);
    let (_, all, _) = r.run(&["check", "--all", "--json"]);
    assert!(!all.contains("SD926"), "{all}");
}

#[test]
fn sd926_is_new_when_the_branch_adds_the_address() {
    // Guard: the code is not a state code, and the base configuration gives
    // the base findings. A branch that adds the address gets the warning as a
    // new one, with exit code 0. A base that lists it already gives no new one.
    let r = start_branch();
    let text = r.read("spec-dna.toml").replace(
        &format!("approvers = [\"{APPROVER}\"]"),
        &format!("approvers = [\"{APPROVER}\", \"noreply@anthropic.com\"]"),
    );
    r.write("spec-dna.toml", &text);
    r.commit("list an agent");
    let (code, out, err) = r.run(&["check", "--base", "main"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains(
            ": warning SD926 approver-agent: `[change] approvers` lists `noreply@anthropic.com`"
        ),
        "{out}"
    );
    // The base already lists the address: the finding is not new.
    r.git(&["checkout", "-q", "-b", "later"]);
    r.write("src/other.py", "y = 2\n");
    r.commit("code");
    let (code, out, err) = r.run(&["check", "--base", "feature"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("SD926"), "{out}");
    let (_, all, _) = r.run(&["check", "--all"]);
    assert!(all.contains("SD926"), "{all}");
}

// A delta that depends on a spike (design 0.7.0, section 15, D11)

const SPIKE: &str = "src/spike.py";
const ADD_AFTER_SPIKE: &str = "- ADD requirements/valves#sensor-timeout: When a sensor gives no value for 30 seconds, the controller SHALL close the valve of its bed.";

/// The commits up to the approval of a plan that lists the spike file.
fn up_to_the_plan_with_a_spike(r: &Repo) {
    let files = [CODE, TEST, SPIKE];
    first_four(r);
    step(r, AGENT, "plan draft", &[(PLAN, &plan("draft", &files))]);
    step(
        r,
        APPROVER,
        "approve plan",
        &[(PLAN, &plan("approved", &files))],
    );
}

#[test]
fn a_delta_edit_after_the_approval_commit_needs_no_new_approval() {
    let r = start_branch();
    up_to_the_plan_with_a_spike(&r);
    step(&r, AGENT, "spike", &[(SPIKE, "print(30)\n")]);
    // The edit keeps the status `approved`. No commit approves it again.
    let edited = spec("approved").replace(ADD, ADD_AFTER_SPIKE);
    step(&r, AGENT, "delta after the spike", &[(SPEC, &edited)]);
    step_code(&r);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
}

// The next test follows the rule of SD910 for a plan that the branch sets back
// to `draft` after its code landed. If the decision about that rule changes,
// this test follows it.
#[test]
fn a_plan_set_back_to_draft_and_approved_again_takes_its_new_files() {
    let r = start_branch();
    up_to_the_plan_with_a_spike(&r);
    step(&r, AGENT, "spike", &[(SPIKE, "print(30)\n")]);
    // The spec goes back to `draft` with the edit, and a status-only commit of
    // the approver sets `approved` again.
    let edited = spec("draft").replace(ADD, ADD_AFTER_SPIKE);
    step(
        &r,
        AGENT,
        "spec back to draft, delta edit",
        &[(SPEC, &edited)],
    );
    // Between the step back and the new approval commit, the approved plan
    // has no approved spec at the head (reason 9, ADR 0019). The approval
    // commit below clears it.
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[(PLAN, 9)]), "{out}");
    assert_eq!(code, 1, "{out}");
    let edited = spec("approved").replace(ADD, ADD_AFTER_SPIKE);
    step(&r, APPROVER, "approve spec again", &[(SPEC, &edited)]);
    // The plan lists one more path. The same 2 commits approve it.
    let files = [CODE, TEST, SPIKE, "src/extra.py"];
    step(
        &r,
        AGENT,
        "plan back to draft",
        &[(PLAN, &plan("draft", &files))],
    );
    step(
        &r,
        APPROVER,
        "approve plan again",
        &[(PLAN, &plan("approved", &files))],
    );
    step(&r, AGENT, "extra file", &[("src/extra.py", "x = 1\n")]);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
}

// A step back after the approval (reason 9)

/// The commits up to the approval commit of the plan (no code).
fn approved_plan(r: &Repo) {
    first_four(r);
    step_plan_draft(r);
    step_plan_approved(r, APPROVER);
}

/// A commit of the agent that sets the plan back to `draft`.
fn step_plan_back(r: &Repo) {
    step(
        r,
        AGENT,
        "plan back to draft",
        &[(PLAN, &plan("draft", &FILES))],
    );
}

/// The first 12 characters of the id of `HEAD`.
fn short_head(r: &Repo) -> String {
    r.git_out(&["rev-parse", "HEAD"])[..12].to_string()
}

#[test]
fn a_plan_set_back_to_draft_after_the_code_gives_reason_9() {
    // The branch approves the plan, adds the code and sets the plan back to
    // `draft`. A squash merge would put the code and a `draft` plan on the
    // default branch, as `approval = "base"` reports (reason 1).
    let r = start_branch();
    ok_flow(&r);
    step_plan_back(&r);
    let back = short_head(&r);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[(CODE, 9), (TEST, 9)]), "{out}");
    assert_eq!(code, 1, "{out}");
    let msg = &findings_of(&out)[0].1;
    assert!(msg.contains(&format!("the commit {back} set `{PLAN}` back to `draft`")));
    assert!(msg.contains("after its approval"), "{msg}");
    assert!(
        msg.contains("approve it again with an approval commit"),
        "{msg}"
    );
}

#[test]
fn code_after_a_plan_set_back_to_draft_gives_reason_9() {
    let r = start_branch();
    approved_plan(&r);
    step_plan_back(&r);
    let back = short_head(&r);
    step_code(&r);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[(CODE, 9), (TEST, 9)]), "{out}");
    assert_eq!(code, 1, "{out}");
    assert!(findings_of(&out)[0].1.contains(&back), "{out}");
}

#[test]
fn work_between_a_draft_and_a_new_approval_gives_sd910() {
    // The code comes after the step back and before the new approval commit.
    // The head plan is `approved`, but no approval holds at the commit of the
    // code.
    let r = start_branch();
    approved_plan(&r);
    step_plan_back(&r);
    step_code(&r);
    step_plan_approved(&r, APPROVER);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[(CODE, 9), (TEST, 9)]), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_plan_approved_again_clears_the_work_after_the_new_approval() {
    // Guard: the new approval commit clears the work after it, and the
    // earlier commits stay as they were.
    let r = start_branch();
    approved_plan(&r);
    step_plan_back(&r);
    step_plan_approved(&r, APPROVER);
    step_code(&r);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[]), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_plan_set_back_to_draft_in_the_working_tree_gives_reason_9() {
    // No commit holds the step back, so the message names the working tree.
    let r = start_branch();
    ok_flow(&r);
    r.write(PLAN, &plan("draft", &FILES));
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[(CODE, 9), (TEST, 9)]), "{out}");
    assert_eq!(code, 1, "{out}");
    assert!(
        findings_of(&out)[0]
            .1
            .contains(&format!("the working tree set `{PLAN}` back to `draft`")),
        "{out}"
    );
}

/// The end of the message of reason 9 for the working tree (0.7.1, B6): the
/// way of the commit of the step back.
const STEP_BACK_WAY: &str = "; if this is the commit of the step back, a hook that runs `spec-dna check` rejects it: skip only that hook for this commit, name the skip in its body, and make the approval commit next";

#[test]
fn the_working_tree_step_back_names_the_way_of_the_commit_of_the_step_back() {
    let r = start_branch();
    ok_flow(&r);
    r.write(PLAN, &plan("draft", &FILES));
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[(CODE, 9), (TEST, 9)]), "{out}");
    assert_eq!(code, 1, "{out}");
    let msg = &findings_of(&out)[0].1;
    assert!(msg.ends_with(STEP_BACK_WAY), "{msg}");
    // The text of before stays in front of the new end.
    assert!(
        msg.contains(
            "approve it again with an approval commit before the work, or take the work out of the branch; if this is the commit"
        ),
        "{msg}"
    );
}

#[test]
fn a_commit_that_set_the_step_back_gets_no_hint_for_its_own_commit() {
    // The commit exists, so the hook already ran for it.
    let r = start_branch();
    ok_flow(&r);
    step_plan_back(&r);
    let (_, out, found) = check(&r);
    assert_eq!(found, want(&[(CODE, 9), (TEST, 9)]), "{out}");
    let msg = &findings_of(&out)[0].1;
    assert!(
        !msg.contains("if this is the commit of the step back"),
        "{msg}"
    );
}

#[test]
fn an_intent_set_back_to_draft_after_its_spec_gives_reason_9() {
    let r = start_branch();
    first_four(&r);
    step_intent_back(&r);
    let (code, out, found) = check(&r);
    assert_eq!(found, want(&[(SPEC, 9)]), "{out}");
    assert_eq!(code, 1, "{out}");
}

fn step_intent_back(r: &Repo) {
    step(
        r,
        AGENT,
        "intent back to draft",
        &[(INTENT, &intent("draft", "higher-risk"))],
    );
}

/// A repository whose base tree holds the change with an `accepted` intent, an
/// `approved` spec and an `approved` plan. The branch is `feature`.
fn start_with_approved_base() -> Repo {
    let r = Repo::with_config(&config(&branch_keys(&[APPROVER])));
    r.write(REQ, VALVES);
    r.write(CODE, CODE_OLD);
    r.write(TEST, TEST_OLD);
    r.write(INTENT, &intent("accepted", "higher-risk"));
    r.write(SPEC, &spec("approved"));
    r.write(PLAN, &plan("approved", &FILES));
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r
}

#[test]
fn a_base_plan_set_back_to_draft_gives_sd401_and_no_reason_9() {
    // Guard: an approval of the base tree keeps the result of 0.6. The code
    // comes before the step back and stays clear, and SD401 reports the move.
    let r = start_with_approved_base();
    step_code(&r);
    step_plan_back(&r);
    // SD401 compares the 2 trees, so it needs the explicit base.
    let (code, out, err) = r.run(&["check", "--base", "main"]);
    assert!(findings_of(&out).is_empty(), "{out}{err}");
    assert!(out.contains("SD401 status-transition"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn finish_in_the_branch_keeps_the_approval_of_a_deleted_artifact() {
    // Guard: the base tree has the change with an `accepted` intent and 2
    // drafts. The branch approves the spec and the plan, adds the code and
    // deletes the folder of the change. The head has no artifact, so the head
    // rule reads none.
    let r = Repo::with_config(&config(&branch_keys(&[APPROVER])));
    r.write(REQ, VALVES);
    r.write(CODE, CODE_OLD);
    r.write(TEST, TEST_OLD);
    r.write(INTENT, &intent("accepted", "higher-risk"));
    r.write(SPEC, &spec("draft"));
    r.write(PLAN, &plan("draft", &FILES));
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    step_spec_approved(&r, APPROVER);
    step_plan_approved(&r, APPROVER);
    step_code(&r);
    r.git(&["rm", "-q", "-r", DIR]);
    r.commit_as("finish", AGENT);
    let (_, out, found) = check(&r);
    assert!(found.iter().all(|(_, n)| *n != 9), "{out}");
    assert_eq!(found, want(&[]), "{out}");
}

// The git access of the walk

#[test]
fn the_walk_runs_the_same_number_of_git_processes_for_each_branch() {
    // A fixed number of processes: the commits of the branch do not count.
    // The trace of git lists each process that `check` starts.
    let count = |commits: usize| {
        let r = start_branch();
        step_intent_draft(&r);
        for i in 0..commits {
            step(
                &r,
                AGENT,
                "edit",
                &[("README.md", &format!("# Valves {i}\n"))],
            );
        }
        let trace = r.dir.join("trace.json");
        let (code, out, err) = r.run_env(
            &["check", "--no-cache"],
            &[("GIT_TRACE2_EVENT", trace.to_str().unwrap())],
            &[],
        );
        assert!(code == 0 || code == 1, "{out}{err}");
        let text = std::fs::read_to_string(&trace).unwrap();
        let all = text
            .lines()
            .filter(|l| l.contains("\"event\":\"start\""))
            .count();
        let diff_tree = text
            .lines()
            .filter(|l| l.contains("\"event\":\"start\"") && l.contains("\"diff-tree\""))
            .count();
        (all, diff_tree)
    };
    let (few, few_diff_tree) = count(3);
    let (many, many_diff_tree) = count(30);
    assert_eq!(few_diff_tree, 1);
    assert_eq!(many_diff_tree, 1);
    assert_eq!(
        few, many,
        "the number of git processes depends on the commits"
    );
}

// The acknowledgement of the acceptance record with `branch` (design 0.8.0, D3
// item 10, D73 item 7, slice S6): `approve <slug> acceptance`, SD932 and SD934.

const RECORD: &str = "docs/intent/valve-timeout/acceptance.toml";
const SECOND: &str = "second@example.com";

/// A higher-risk change in one branch, up to the code. The approvers are
/// `approvers`.
fn ack_repo(approvers: &[&str]) -> Repo {
    let r = start(&format!(
        "{}acceptance-for = \"all\"\n",
        branch_keys(approvers)
    ));
    ok_flow(&r);
    r
}

fn accept(r: &Repo, args: &[&str]) -> (i32, String, String) {
    let mut a = vec!["accept", "valve-timeout"];
    a.extend_from_slice(args);
    r.run(&a)
}

/// The record that the acceptor writes: the skeleton with the hash of its brief,
/// the accepted results, one attempt and one deviation. The deviation makes the
/// record need an acknowledgement. Nothing is committed.
fn write_record(r: &Repo, session: &str) -> String {
    let (code, out, err) = accept(r, &["--session", session]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = accept(r, &["--brief"]);
    assert_eq!(code, 0, "{out}{err}");
    let hash = out
        .lines()
        .last()
        .unwrap()
        .strip_prefix("brief ")
        .unwrap()
        .to_string();
    let mut t = r
        .read(RECORD)
        .replacen("brief = \"pending\"", &format!("brief = \"{hash}\""), 1)
        .replace("result = \"pending\"", "result = \"accepted\"")
        .replace("verdict = \"pending\"", "verdict = \"accepted\"");
    t.push_str("\n[[attempt]]\nacceptor = 1\ntext = \"Sent a NaN value.\"\noutcome = \"held\"\n");
    t.push_str("\n[[deviation]]\nid = \"d1\"\nkind = \"plan\"\ntext = \"The check moved.\"\nverdict = \"accepted\"\n");
    r.write(RECORD, &t);
    t
}

/// The record commit of the acceptor: the record alone.
fn record_commit(r: &Repo) {
    write_record(r, "s1");
    r.commit_as("record", AGENT);
}

fn as_person(r: &Repo, who: &str, args: &[&str]) -> (i32, String, String) {
    r.run_env(
        args,
        &[
            ("GIT_AUTHOR_NAME", "Test"),
            ("GIT_COMMITTER_NAME", "Test"),
            ("GIT_AUTHOR_EMAIL", who),
            ("GIT_COMMITTER_EMAIL", who),
        ],
        &[],
    )
}

fn approve_acceptance(r: &Repo, who: &str) -> (i32, String, String) {
    as_person(r, who, &["approve", "valve-timeout", "acceptance"])
}

fn check_base(r: &Repo) -> (i32, String) {
    let (code, out, err) = r.run(&["check", "--base", "origin/HEAD"]);
    assert!(code == 0 || code == 1, "{out}{err}");
    (code, out)
}

fn has(out: &str, code: &str) -> bool {
    out.lines().any(|l| l.contains(&format!(" {code} ")))
}

fn finish_run(r: &Repo) -> (i32, String) {
    let (code, out, err) = r.run(&["finish", "valve-timeout"]);
    (code, format!("{out}{err}"))
}

#[test]
fn a_record_commit_alone_gives_no_sd932_and_the_honest_flow_passes() {
    let r = ack_repo(&[APPROVER]);
    record_commit(&r);
    let (code, out) = check_base(&r);
    assert_eq!(code, 0, "{out}");
    assert!(!has(&out, "SD932"), "{out}");
}

#[test]
fn a_commit_that_changes_the_record_and_another_path_gives_sd932() {
    let r = ack_repo(&[APPROVER]);
    write_record(&r, "s1");
    r.write("README.md", "# Valves, edited\n");
    r.commit_as("record and readme", AGENT);
    let (code, out) = check_base(&r);
    assert_eq!(code, 1, "{out}");
    assert!(has(&out, "SD932"), "{out}");
    assert!(out.contains("README.md"), "the other path is named: {out}");
}

#[test]
fn a_later_record_commit_that_changes_another_path_too_gives_sd932_also_in_finish() {
    let r = ack_repo(&[APPROVER]);
    record_commit(&r);
    // A follow-up commit of the record that also edits a file (D73 item 7).
    let t = r.read(RECORD);
    r.write(
        RECORD,
        &format!("{t}\n[[attempt]]\nacceptor = 1\ntext = \"Another try.\"\noutcome = \"held\"\n"),
    );
    r.write("README.md", "# Valves, edited\n");
    r.commit_as("record and another file", AGENT);
    let (code, out) = finish_run(&r);
    assert_eq!(code, 1, "{out}");
    assert!(has(&out, "SD932"), "{out}");
}

#[test]
fn an_approval_commit_of_an_approver_makes_the_entry_count() {
    let r = ack_repo(&[APPROVER]);
    record_commit(&r);
    // With no acknowledgement, `finish` stops with SD934 and names the approvers.
    let (code, out) = finish_run(&r);
    assert_eq!(code, 1, "{out}");
    assert!(has(&out, "SD934"), "{out}");
    assert!(out.contains(APPROVER), "the approvers are named: {out}");
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        r.read(RECORD)
            .contains(&format!("acknowledged-by = [\"{APPROVER}\"]"))
    );
    let (code, out) = check_base(&r);
    assert_eq!(code, 0, "{out}");
    let (code, out) = finish_run(&r);
    assert_eq!(code, 0, "{out}");
    assert!(!has(&out, "SD934") && !has(&out, "SD932"), "{out}");
}

#[test]
fn an_entry_that_is_not_the_author_of_its_commit_gives_sd932() {
    let r = ack_repo(&[APPROVER]);
    record_commit(&r);
    // An agent writes the e-mail of the approver into the record.
    let t = r.read(RECORD).replace(
        "acknowledged-by = []",
        &format!("acknowledged-by = [\"{APPROVER}\"]"),
    );
    r.write(RECORD, &t);
    r.commit_as("forged acknowledgement", AGENT);
    let (code, out) = check_base(&r);
    assert_eq!(code, 1, "{out}");
    assert!(has(&out, "SD932"), "{out}");
    assert!(out.contains(APPROVER), "{out}");
    let (code, out) = finish_run(&r);
    assert_eq!(code, 1, "{out}");
    assert!(has(&out, "SD932"), "{out}");
}

#[test]
fn a_record_commit_that_adds_an_entry_with_the_record_gives_sd932() {
    let r = ack_repo(&[APPROVER]);
    let t = write_record(&r, "s1").replace(
        "acknowledged-by = []",
        &format!("acknowledged-by = [\"{APPROVER}\"]"),
    );
    r.write(RECORD, &t);
    r.commit_as("record with an entry", APPROVER);
    let (code, out) = check_base(&r);
    assert_eq!(code, 1, "{out}");
    assert!(has(&out, "SD932"), "{out}");
}

#[test]
fn a_commit_that_keeps_an_entry_gives_sd932_and_approve_again_clears_it() {
    let r = ack_repo(&[APPROVER]);
    record_commit(&r);
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    // The record changes after the approval, and the entry stays.
    let t = r.read(RECORD);
    r.write(
        RECORD,
        &format!("{t}\n[[attempt]]\nacceptor = 1\ntext = \"Another try.\"\noutcome = \"held\"\n"),
    );
    r.commit_as("another attempt", AGENT);
    let (code, out) = check_base(&r);
    assert_eq!(code, 1, "{out}");
    assert!(has(&out, "SD932"), "{out}");
    let (code, out) = finish_run(&r);
    assert_eq!(code, 1, "{out}");
    assert!(has(&out, "SD932"), "{out}");
    // The approver acknowledges the new text. No finding stays.
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out) = check_base(&r);
    assert_eq!(code, 0, "{out}");
    assert!(!has(&out, "SD932") && !has(&out, "SD934"), "{out}");
    let (code, out) = finish_run(&r);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn approve_when_the_entry_already_counts_makes_no_commit() {
    let r = ack_repo(&[APPROVER]);
    record_commit(&r);
    let (code, _, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{err}");
    let head = r.git_out(&["rev-parse", "HEAD"]);
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("already"), "{out}");
    assert_eq!(r.git_out(&["rev-parse", "HEAD"]), head, "no new commit");
}

#[test]
fn a_second_acceptor_after_the_approval_clears_the_entry() {
    let r = ack_repo(&[APPROVER]);
    record_commit(&r);
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    // `accept` adds an entry to a record that has an acknowledgement. The text
    // changes, so the entry would outlive it: `accept` writes the empty list, and
    // the approver acknowledges the new text.
    let (code, out, err) = accept(&r, &["--session", "s2", "--role", "second"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.read(RECORD);
    assert!(t.contains("acknowledged-by = []"), "{t}");
    assert!(!t.contains(APPROVER), "{t}");
}

#[test]
fn an_approval_commit_of_a_second_approver_keeps_the_first_entry() {
    let r = ack_repo(&[APPROVER, SECOND]);
    record_commit(&r);
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = approve_acceptance(&r, SECOND);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.read(RECORD);
    assert!(t.contains(APPROVER) && t.contains(SECOND), "{t}");
    let (code, out) = check_base(&r);
    assert_eq!(code, 0, "{out}");
    assert!(!has(&out, "SD932") && !has(&out, "SD934"), "{out}");
}

#[test]
fn the_commit_of_finish_moves_the_entry_and_the_check_after_it_finds_nothing() {
    let r = ack_repo(&[APPROVER]);
    record_commit(&r);
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out) = finish_run(&r);
    assert_eq!(code, 0, "{out}");
    r.commit_as("finish", AGENT);
    // The change was added and finished in this branch, so SD910 reports reason 7.
    // The codes of the record find nothing.
    let (_, out) = check_base(&r);
    assert!(
        !has(&out, "SD932") && !has(&out, "SD934") && !has(&out, "SD927"),
        "{out}"
    );
}

#[test]
fn the_archive_of_a_record_with_an_entry_nobody_added_gives_sd934_after_finish() {
    // The commit of `finish` keeps the entries that count. A record with no
    // acknowledgement keeps none, so the check after `finish` names SD934.
    let r = ack_repo(&[APPROVER]);
    record_commit(&r);
    let t = r.read(RECORD);
    r.git(&["rm", "-q", "-r", DIR]);
    r.write(".spec-dna/accepted/valve-timeout.toml", &t);
    r.commit_as("finish by hand", AGENT);
    let (code, out) = check_base(&r);
    assert_eq!(code, 1, "{out}");
    assert!(has(&out, "SD934"), "{out}");
    assert!(!has(&out, "SD932"), "{out}");
}

/// Like `ack_repo`, but the commit that adds the plan pins 0.8.0 in `spec-dna.toml`,
/// and the next commit sets the pin of the binary again. The plan is then new for a
/// later pull request (D70 item 5).
fn old_pin_repo() -> Repo {
    let r = start_branch_all();
    first_four(&r);
    let now = r.read("spec-dna.toml");
    r.write("spec-dna.toml", &now.replacen(VERSION, "0.8.0", 1));
    step_plan_draft(&r);
    r.write("spec-dna.toml", &now);
    r.commit_as("move the pin", AGENT);
    step_plan_approved(&r, APPROVER);
    step_code(&r);
    r
}

fn start_branch_all() -> Repo {
    start(&format!(
        "{}acceptance-for = \"all\"\n",
        branch_keys(&[APPROVER])
    ))
}

#[test]
fn a_later_pull_request_with_only_finish_reads_the_entry_of_the_base() {
    let r = old_pin_repo();
    record_commit(&r);
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    // The work merges into the base with the record and its entry.
    r.git(&["checkout", "-q", "main"]);
    r.git(&["merge", "-q", "--ff-only", "feature"]);
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "finish-pr"]);
    let (code, out) = finish_run(&r);
    assert_eq!(code, 0, "{out}");
    r.commit_as("finish", AGENT);
    let (_, out) = check_base(&r);
    assert!(
        !has(&out, "SD932") && !has(&out, "SD934") && !has(&out, "SD927"),
        "{out}"
    );
}

#[test]
fn the_test_merge_of_a_code_host_gives_the_same_result_as_the_branch() {
    let src = ack_repo(&[APPROVER]);
    record_commit(&src);
    let (code, out, err) = approve_acceptance(&src, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    src.git(&["checkout", "-q", "main"]);
    src.write("README.md", "# Valves, main moved\n");
    src.commit("main moves on");
    origin_head(&src.dir, "main");
    src.git(&["checkout", "-q", "-b", "queue", "main"]);
    src.git(&["merge", "-q", "--no-ff", "-m", "test merge", "feature"]);
    let (code, out) = check_base(&src);
    assert_eq!(code, 0, "{out}");
    assert!(!has(&out, "SD932") && !has(&out, "SD934"), "{out}");
}

#[test]
fn a_shallow_clone_cuts_the_walk_and_sd932_names_the_fix() {
    let src = ack_repo(&[APPROVER]);
    record_commit(&src);
    let (code, out, err) = approve_acceptance(&src, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    src.git(&["checkout", "-q", "main"]);
    src.write("README.md", "# Valves, main moved\n");
    src.commit("main moves on");
    src.git(&["checkout", "-q", "-b", "queue", "main"]);
    src.git(&["merge", "-q", "--no-ff", "-m", "test merge", "feature"]);
    let dst = Repo::empty();
    shallow_clone(&src.dir, &dst.dir, 2);
    let tip = src.git_out(&["rev-parse", "main"]);
    origin_head(&dst.dir, &tip);
    let (_, out, err) = dst.run(&["check"]);
    let line = out
        .lines()
        .find(|l| l.contains(" SD932 "))
        .unwrap_or_else(|| panic!("{out}{err}"));
    assert!(line.contains("fetch-depth: 0"), "{line}");
}

#[test]
fn the_staged_approval_commit_passes_the_hook_check_and_a_mixed_staged_commit_does_not() {
    let r = ack_repo(&[APPROVER]);
    record_commit(&r);
    // The index has the entry, and the commit has not been made yet: the hook of
    // `init` runs `check --staged` before the approval commit.
    let t = r.read(RECORD).replace(
        "acknowledged-by = []",
        &format!("acknowledged-by = [\"{APPROVER}\"]"),
    );
    r.write(RECORD, &t);
    r.git(&["add", RECORD]);
    let (code, out, err) = as_person(&r, APPROVER, &["check", "--staged"]);
    assert!(!has(&out, "SD932"), "{code} {out}{err}");
    // A staged record and another staged path are no record commit.
    r.write("README.md", "# Valves, staged\n");
    r.git(&["add", "README.md"]);
    let (_, out, err) = as_person(&r, APPROVER, &["check", "--staged"]);
    assert!(has(&out, "SD932"), "{out}{err}");
}

#[test]
fn with_approval_base_no_record_commit_is_checked_and_approve_acceptance_refuses() {
    let r = start("acceptance-for = \"all\"\n");
    step_intent_approved(&r, AGENT);
    step(&r, AGENT, "spec", &[(SPEC, &spec("approved"))]);
    step(&r, AGENT, "plan", &[(PLAN, &plan("approved", &FILES))]);
    step_code(&r);
    write_record(&r, "s1");
    r.write("README.md", "# Valves, with the record\n");
    r.commit_as("record and readme", AGENT);
    let (_, out) = check_base(&r);
    assert!(!has(&out, "SD932") && !has(&out, "SD934"), "{out}");
    let (code, out) = finish_run(&r);
    assert!(!has(&out, "SD932") && !has(&out, "SD934"), "{code} {out}");
    // `approve acceptance` is a command of the mode `branch`.
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("branch"), "{err}");
}

#[test]
fn approve_acceptance_refuses_an_author_outside_the_approvers_and_a_missing_record() {
    let r = ack_repo(&[APPROVER]);
    // No record yet: exit code 5.
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 5, "{out}{err}");
    record_commit(&r);
    let (code, out, err) = approve_acceptance(&r, STRANGER);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains(STRANGER) && err.contains("approvers"), "{err}");
    // A record that has an edit that HEAD does not have.
    let t = r.read(RECORD);
    r.write(RECORD, &format!("{t}\n# edit\n"));
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("commit"), "{err}");
}

#[test]
fn the_json_of_approve_acceptance_has_the_fields_of_approve() {
    let r = ack_repo(&[APPROVER]);
    record_commit(&r);
    let (code, out, err) = as_person(
        &r,
        APPROVER,
        &["approve", "valve-timeout", "acceptance", "--json"],
    );
    assert_eq!(code, 0, "{out}{err}");
    for key in ["slug", "artifact", "path", "status", "commit", "author"] {
        assert!(out.contains(&format!("\"{key}\":")), "{key}: {out}");
    }
    assert!(out.contains("\"artifact\":\"acceptance\""), "{out}");
}

#[test]
fn the_staged_result_of_finish_in_a_later_pull_request_passes_the_hook_check() {
    // The work and its record merged earlier. In the pull request of `finish`, the
    // command stages the archive file and the deletion of the folder together. The
    // commit of `finish` keeps the entries that count, so `check --staged` finds no
    // SD932 and no SD934. A change that the branch added and finished is not read by
    // `check --staged`: its diff holds no deleted folder (the cause is outside the
    // acknowledgement), so only this case reaches the arm.
    let r = old_pin_repo();
    record_commit(&r);
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    r.git(&["checkout", "-q", "main"]);
    r.git(&["merge", "-q", "--ff-only", "feature"]);
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "finish-pr"]);
    let (code, out) = finish_run(&r);
    assert_eq!(code, 0, "{out}");
    let (_, out, err) = r.run(&["check", "--staged"]);
    assert!(
        !has(&out, "SD932") && !has(&out, "SD934") && !has(&out, "SD927"),
        "{out}{err}"
    );
    // The same staged result, but the archive file was committed alone before: the
    // staged deletion of the folder ends the record, as the committed check reads it.
    r.git(&["reset", "-q", "--hard", "HEAD"]);
    r.git(&["clean", "-qfd"]);
    let t = r.read(RECORD);
    r.write(".spec-dna/accepted/valve-timeout.toml", &t);
    r.git(&["add", ".spec-dna"]);
    r.commit_as("archive alone", AGENT);
    r.git(&["rm", "-q", "-r", DIR]);
    let (_, out, err) = r.run(&["check", "--staged"]);
    assert!(has(&out, "SD934"), "{out}{err}");
}

const NOTE: &str = "\n[[finding]]\nacceptor = 1\nseverity = \"note\"\npath = \"src/valves.py\"\nline = 3\ntext = \"The name has no unit.\"\n";

/// A record with one note on `src/valves.py`, committed, then acknowledged. The
/// fix of the note is committed too. The follow-up is not written yet.
fn noted_and_fixed() -> Repo {
    let r = ack_repo(&[APPROVER]);
    let t = write_record(&r, "s1");
    r.write(RECORD, &format!("{t}{NOTE}"));
    r.commit_as("record", AGENT);
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    let code_text = r.read(CODE);
    r.write(
        CODE,
        &format!("{code_text}\n\ndef timeout_ms():\n    return 60000\n"),
    );
    r.commit_as("fix", AGENT);
    r
}

/// The follow-up as an agent writes it: the brief, then `accept --follow-up`, then
/// the fields. The record is written, not committed.
fn write_follow_up(r: &Repo) {
    let (code, out, err) = accept(r, &["--follow-up", "--brief"]);
    assert_eq!(code, 0, "{out}{err}");
    let hash = out
        .lines()
        .last()
        .unwrap()
        .strip_prefix("brief ")
        .unwrap()
        .to_string();
    let (code, out, err) = accept(r, &["--follow-up", "--session", "s2"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r
        .read(RECORD)
        .replacen("brief = \"pending\"", &format!("brief = \"{hash}\""), 1)
        .replacen("result = \"pending\"", "result = \"accepted\"", 1)
        .replacen(
            "text = \"pending\"",
            "text = \"The name has a unit now. Proof: the test passed.\"",
            1,
        );
    r.write(RECORD, &t);
}

#[test]
fn a_follow_up_after_the_approval_clears_the_entry_and_the_approver_acknowledges_again() {
    let r = noted_and_fixed();
    write_follow_up(&r);
    // The follow-up changes the text of the record, so the entry is gone.
    assert!(
        r.read(RECORD).contains("acknowledged-by = []"),
        "{}",
        r.read(RECORD)
    );
    r.commit_as("follow-up", AGENT);
    let (code, out) = check_base(&r);
    assert!(!has(&out, "SD932"), "{code} {out}");
    // No acknowledgement of the new text: `finish` stops with SD934.
    let (code, out) = finish_run(&r);
    assert_eq!(code, 1, "{out}");
    assert!(has(&out, "SD934"), "{out}");
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out) = check_base(&r);
    assert!(!has(&out, "SD932") && !has(&out, "SD934"), "{code} {out}");
    let (code, out) = finish_run(&r);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_follow_up_commit_that_also_changes_code_gives_sd932() {
    let r = noted_and_fixed();
    write_follow_up(&r);
    let code_text = r.read(CODE);
    r.write(CODE, &format!("{code_text}\n# a second edit\n"));
    r.commit_as("follow-up and code", AGENT);
    let (code, out) = check_base(&r);
    assert_eq!(code, 1, "{out}");
    assert!(has(&out, "SD932"), "{out}");
    assert!(out.contains(CODE), "the other path is named: {out}");
}

#[test]
fn a_follow_up_commit_that_keeps_an_entry_gives_sd932() {
    // The agent puts the entry of the approver back by hand in the follow-up commit.
    let r = noted_and_fixed();
    write_follow_up(&r);
    let t = r.read(RECORD).replace(
        "acknowledged-by = []",
        &format!("acknowledged-by = [\"{APPROVER}\"]"),
    );
    r.write(RECORD, &t);
    r.commit_as("follow-up with the old entry", AGENT);
    let (code, out) = check_base(&r);
    assert_eq!(code, 1, "{out}");
    assert!(has(&out, "SD932"), "{out}");
}

#[test]
fn the_per_role_rule_of_sd933_reads_the_roles_of_the_entries() {
    // The roles of `[owners]` need `roles` in the record, and the rule is the
    // same with `branch`: an entry with the role passes SD933.
    let r = ack_repo(&[APPROVER]);
    let cfg = r.read("spec-dna.toml");
    r.write(
        "spec-dna.toml",
        &format!("{cfg}\n[owners]\n\"src/**\" = [\"security-reviewer\"]\n"),
    );
    r.commit_as("owners", AGENT);
    let (code, out, err) = accept(&r, &["--session", "s1", "--role", "acceptor"]);
    assert_eq!(code, 0, "{out}{err}");
    let (_, out, _) = accept(&r, &["--brief", "--role", "acceptor"]);
    let hash = out.lines().last().unwrap().strip_prefix("brief ").unwrap();
    let t = r
        .read(RECORD)
        .replacen("brief = \"pending\"", &format!("brief = \"{hash}\""), 1)
        .replace("result = \"pending\"", "result = \"accepted\"")
        .replace("verdict = \"pending\"", "verdict = \"accepted\"");
    r.write(
        RECORD,
        &format!("{t}\n[[attempt]]\nacceptor = 1\ntext = \"Tried.\"\noutcome = \"held\"\n"),
    );
    r.commit_as("record", AGENT);
    let (code, out) = finish_run(&r);
    assert_eq!(code, 1, "{out}");
    assert!(
        has(&out, "SD933") && out.contains("security-reviewer"),
        "{out}"
    );
}

#[test]
fn a_merge_commit_that_carries_the_record_and_another_path_gives_sd932() {
    for extra in ["README.md", CODE] {
        let r = ack_repo(&[APPROVER]);
        r.git(&["checkout", "-q", "main"]);
        r.write("src/other.py", "y = 2\n");
        r.commit("main moves on");
        origin_head(&r.dir, "main");
        r.git(&["checkout", "-q", "feature"]);
        r.git(&["merge", "-q", "--no-commit", "--no-ff", "main"]);
        write_record(&r, "s1");
        let text = r.read(extra);
        r.write(extra, &format!("{text}\n# merge edit\n"));
        r.commit_as("merge with the record", AGENT);
        let (code, out) = check_base(&r);
        assert_eq!(code, 1, "{extra}: {out}");
        assert!(has(&out, "SD932"), "{extra}: {out}");
    }
}

#[test]
fn a_merge_of_the_base_that_carries_no_record_change_is_neutral() {
    let r = ack_repo(&[APPROVER]);
    record_commit(&r);
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    r.git(&["checkout", "-q", "main"]);
    r.write("src/other.py", "y = 2\n");
    r.commit("main moves on");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "feature"]);
    r.git(&["merge", "-q", "--no-ff", "-m", "merge the base", "main"]);
    let (code, out) = check_base(&r);
    assert_eq!(code, 0, "{out}");
    assert!(!has(&out, "SD932") && !has(&out, "SD934"), "{out}");
}

#[test]
fn next_names_the_acknowledgement_as_an_item_of_a_person() {
    let r = ack_repo(&[APPROVER]);
    record_commit(&r);
    let (code, out, err) = r.run(&["next", "valve-timeout"]);
    assert_eq!(code, 0, "{out}{err}");
    let line = out
        .lines()
        .find(|l| {
            l.contains("spec-dna approve valve-timeout acceptance")
                || l.contains("SD934")
                || l.contains("acknowledg")
        })
        .unwrap_or_else(|| panic!("{out}"));
    assert!(line.ends_with("[person]"), "{line}");
    let (_, json, _) = r.run(&["next", "valve-timeout", "--json"]);
    assert!(json.contains("\"person\":true"), "{json}");
    // After the approval the item is done.
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    let (_, out, _) = r.run(&["next", "valve-timeout"]);
    assert!(!out.contains("acknowledg"), "{out}");
}

#[test]
fn finish_says_who_acknowledged_the_deviations_in_each_mode() {
    // With `branch`, the entry of an approver acknowledged them, not the merge.
    let r = ack_repo(&[APPROVER]);
    record_commit(&r);
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out) = finish_run(&r);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("Deviations: 1"), "{out}");
    assert!(
        out.contains("an approver acknowledged the deviations in `acknowledged-by`"),
        "{out}"
    );
    assert!(
        !out.contains("the merge of the pull request acknowledges"),
        "{out}"
    );
    // With `base`, the merge acknowledges them.
    let r = start("acceptance-for = \"all\"\n");
    step_intent_approved(&r, AGENT);
    step(&r, AGENT, "spec", &[(SPEC, &spec("approved"))]);
    step(&r, AGENT, "plan", &[(PLAN, &plan("approved", &FILES))]);
    step_code(&r);
    write_record(&r, "s1");
    r.commit_as("record", AGENT);
    let (code, out) = finish_run(&r);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("the merge of the pull request acknowledges the deviations"),
        "{out}"
    );
}

#[test]
fn a_branch_cannot_lower_approval_to_base_to_skip_the_acknowledgement() {
    let r = ack_repo(&[APPROVER]);
    let cfg = r.read("spec-dna.toml");
    assert!(cfg.contains("approval = \"branch\""));
    r.write(
        "spec-dna.toml",
        &cfg.replace("approval = \"branch\"", "approval = \"base\""),
    );
    r.commit_as("lower the mode", AGENT);
    record_commit(&r);
    // `finish` reads the stricter value, and the message names the cause.
    let (code, out) = finish_run(&r);
    assert_eq!(code, 1, "{out}");
    assert!(has(&out, "SD934"), "{out}");
    assert!(
        out.contains("the base has `approval = \"branch\"`") && out.contains("cannot lower"),
        "{out}"
    );
    // `approve` reads the same value, so the approver can fix it.
    let (code, out, err) = approve_acceptance(&r, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out) = finish_run(&r);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn approve_in_a_shallow_clone_says_that_the_walk_stopped() {
    let src = ack_repo(&[APPROVER]);
    record_commit(&src);
    let (code, out, err) = approve_acceptance(&src, APPROVER);
    assert_eq!(code, 0, "{out}{err}");
    src.git(&["checkout", "-q", "main"]);
    src.write("README.md", "# Valves, main moved\n");
    src.commit("main moves on");
    src.git(&["checkout", "-q", "-b", "queue", "main"]);
    src.git(&["merge", "-q", "--no-ff", "-m", "test merge", "feature"]);
    let dst = Repo::empty();
    shallow_clone(&src.dir, &dst.dir, 2);
    let tip = src.git_out(&["rev-parse", "main"]);
    origin_head(&dst.dir, &tip);
    // The author is listed already, and the walk cannot run: no "already acknowledged".
    let (code, out, err) = approve_acceptance(&dst, APPROVER);
    assert_eq!(code, 1, "{out}{err}");
    assert!(!out.contains("already"), "{out}");
    assert!(err.contains("fetch-depth: 0"), "{err}");
}

#[test]
fn approve_with_no_git_identity_says_to_set_user_email() {
    // No `user.email` and no `GIT_AUTHOR_EMAIL`: the message names the fix, and the
    // exit code is 4 (a git error), as for a failed commit. This holds for each
    // artifact and for the record.
    let r = ack_repo(&[APPROVER]);
    record_commit(&r);
    for artifact in ["acceptance", "plan"] {
        let (code, out, err) = r.run(&["approve", "valve-timeout", artifact]);
        assert_eq!(code, 4, "{artifact}: {out}{err}");
        assert!(err.contains("user.email"), "{artifact}: {err}");
        assert!(err.contains("GIT_AUTHOR_EMAIL"), "{artifact}: {err}");
        assert!(!err.contains("panicked"), "{artifact}: {err}");
    }
}
