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
    format!(
        "profile = \"standard\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n{change}\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n"
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
    assert_eq!(found, want(&[(CODE, 2), (TEST, 2)]), "{out}");
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
        let origin = Repo::empty();
        git(&origin.dir, &["init", "-q", "--bare", "-b", "main"]);
        let r = Repo::with_config(&config(change));
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
        let rp = Replay::new(&branch_keys(&[APPROVER]));
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
