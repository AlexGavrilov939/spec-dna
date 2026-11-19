//! Integration tests of `spec-dna next` and `spec-dna status` (step 3, slice
//! S7): the stage of a change, the checklist, the skill names, the task
//! progress, the groups, the base and the exit codes.
//!
//! Each fixture has a `main` branch with the base commit, a
//! `refs/remotes/origin/HEAD` that names it, and a `feature` branch. A test
//! commits the files of a stage on `feature`, and then it runs the commands.

mod common;

use common::*;

const VALVES: &str = "---\nstatus: active\n---\n# Closing\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n- **open**: The controller SHALL open the valve in 5 seconds.\n";
const SENSORS: &str = "---\nstatus: active\n---\n# Timeouts\n\n## Requirements\n\n- **timeout**: The controller SHALL report a sensor timeout.\n";

const INTENT: &str = "docs/intent/dose/intent.md";
const SPEC: &str = "docs/intent/dose/spec.md";
const PLAN: &str = "docs/intent/dose/plan.md";
const CLOSING: &str = "docs/requirements/valves/closing.md";

/// The exact list of the skill names (the owner text, 0.7.0).
const SKILLS: [&str; 9] = [
    "brainstorm",
    "specify",
    "clarify",
    "write-plan",
    "execute",
    "tdd",
    "review",
    "verify",
    "finish",
];

fn config(change: &str, extra: &str) -> String {
    format!(
        "profile = \"standard\"\nareas = [\"valves\", \"sensors\"]\n\n[groups]\noperations = [\"valves\"]\nreporting = [\"sensors\"]\n\n[change]\n{change}\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.incidents]\ntypes = [\"incident\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\nmax-started = 3\n{extra}"
    )
}

fn intent(status: &str, class: &str, extra: &str) -> String {
    format!(
        "---\nstatus: {status}\nclass: {class}\n{extra}---\n# Dose limit\n\n## Intent\n\nThe dose is too high.\n"
    )
}

fn spec(status: &str, body: &str) -> String {
    format!("---\nstatus: {status}\n---\n# Spec\n\n{body}")
}

const ADD_FLUSH: &str = "## Delta\n\n- ADD requirements/valves/closing#flush: The controller SHALL flush the line before it closes.\n";

fn plan(status: &str, files: &[&str], order: &str) -> String {
    let items: Vec<String> = files.iter().map(|f| format!("- `{f}`")).collect();
    format!(
        "---\nstatus: {status}\n---\n# Plan\n\n## Files\n\n{}\n\n## Order\n\n{order}\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n",
        items.join("\n")
    )
}

/// The base commit on `main`, `origin/HEAD` on it (if `origin`), and the branch
/// `feature`.
fn start_with(change: &str, origin: bool) -> Repo {
    let r = Repo::with_config(&config(change, ""));
    r.write(CLOSING, VALVES);
    r.write("docs/requirements/sensors/timeouts.md", SENSORS);
    r.write("src/dose.py", "x = 1\n");
    r.write("tests/test_other.py", "# nothing\n");
    r.commit("base");
    if origin {
        origin_head(&r.dir, "main");
    }
    r.git(&["checkout", "-q", "-b", "feature"]);
    r
}

fn start() -> Repo {
    start_with("policy = \"three-class\"\ngate = \"block\"\n", true)
}

/// Commits a change of one stage on `feature`.
fn stage(r: &Repo, files: &[(&str, &str)]) {
    for (p, t) in files {
        r.write(p, t);
    }
    r.commit("stage");
}

fn next_json(r: &Repo) -> String {
    let (code, out, err) = r.run(&["next", "dose", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    out
}

fn next_text(r: &Repo) -> String {
    let (code, out, err) = r.run(&["next", "dose"]);
    assert_eq!(code, 0, "{out}{err}");
    out
}

/// The value of a string field of the first object that has it.
fn field<'a>(json: &'a str, name: &str) -> &'a str {
    let key = format!("\"{name}\":\"");
    let rest = json
        .split(&key)
        .nth(1)
        .unwrap_or_else(|| panic!("no `{name}` in {json}"));
    rest.split('"').next().unwrap()
}

/// The stage from `next --json`. `status --json` must give the same stage.
fn stage_of(r: &Repo) -> String {
    let stage = field(&next_json(r), "stage").to_string();
    let (code, out, err) = r.run(&["status", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(field(&out, "stage"), stage, "status and next differ: {out}");
    stage
}

/// Asserts the stage of the only change, through both commands.
fn assert_stage(r: &Repo, want: &str) {
    assert_eq!(stage_of(r), want);
}

/// The skill names of the items of a `next --json` output.
fn skills(json: &str) -> Vec<&str> {
    json.split("\"skill\":\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap())
        .collect()
}

/// The text lines of a `next` output without the 2 header lines.
fn items(text: &str) -> Vec<&str> {
    text.lines().skip(2).collect()
}

fn assert_in_order(text: &str, keys: &[&str]) {
    let mut at = 0;
    for k in keys {
        let p = text[at..]
            .find(k)
            .unwrap_or_else(|| panic!("`{k}` is missing or out of order in {text}"));
        at += p + k.len();
    }
}

// The stages

#[test]
fn a_closed_intent_is_in_the_stage_closed_and_the_next_step_is_finish() {
    let r = start();
    stage(&r, &[(INTENT, &intent("closed", "routine", ""))]);
    let text = next_text(&r);
    assert_stage(&r, "closed");
    assert!(
        text.starts_with("change dose (routine), stage closed\nbase "),
        "{text}"
    );
    let last = items(&text).last().copied().unwrap();
    assert!(
        last.contains("spec-dna finish dose") && last.ends_with("[skill: finish]"),
        "{text}"
    );
    let json = next_json(&r);
    assert_eq!(field(&json, "stage"), "closed");
    assert!(field(&json, "next").contains("spec-dna finish dose"));
}

#[test]
fn a_draft_intent_is_in_the_stage_plan() {
    let r = start();
    stage(&r, &[(INTENT, &intent("draft", "routine", ""))]);
    let text = next_text(&r);
    assert_stage(&r, "plan");
    assert!(text.contains("stage plan"), "{text}");
    let last = items(&text).last().copied().unwrap();
    assert_eq!(
        last,
        "[ ] set `status: accepted` in docs/intent/dose/intent.md [skill: brainstorm]"
    );
    assert_eq!(skills(&next_json(&r)), ["brainstorm"]);
}

#[test]
fn an_open_concern_or_question_of_a_spec_gives_the_skill_clarify_in_the_stage_plan() {
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("draft", "higher-risk", "")),
            (
                SPEC,
                &spec(
                    "draft",
                    "## Concerns\n\n- [ ] Is 5 seconds too long?\n\n## Open questions\n\n- [ ] Who owns the valve?\n",
                ),
            ),
        ],
    );
    assert_stage(&r, "plan");
    let json = next_json(&r);
    assert_eq!(skills(&json), ["clarify", "clarify", "brainstorm"]);
    let text = next_text(&r);
    assert!(text.contains("resolve 1 open concern(s)"), "{text}");
    assert!(text.contains("resolve 1 open question(s)"), "{text}");
    assert!(
        items(&text)
            .last()
            .unwrap()
            .starts_with("[ ] set `status: accepted`"),
        "{text}"
    );
}

#[test]
fn a_higher_risk_change_with_no_approved_spec_is_in_the_stage_design() {
    let r = start();
    // No spec yet.
    stage(&r, &[(INTENT, &intent("accepted", "higher-risk", ""))]);
    let text = next_text(&r);
    assert_stage(&r, "design");
    assert!(text.contains("stage design"), "{text}");
    assert!(text.contains("spec-dna new spec dose"), "{text}");
    assert!(text.contains("[skill: specify]"), "{text}");
    // A draft spec with an open question and an open concern: clarify.
    stage(
        &r,
        &[(
            SPEC,
            &spec(
                "draft",
                &format!(
                    "{ADD_FLUSH}\n## Concerns\n\n- [ ] Is 5 seconds too long?\n\n## Open questions\n\n- [ ] Who owns the valve?\n"
                ),
            ),
        )],
    );
    let text = next_text(&r);
    assert_stage(&r, "design");
    assert!(text.contains("stage design"), "{text}");
    assert!(text.contains("resolve 1 open concern(s)"), "{text}");
    assert!(text.contains("resolve 1 open question(s)"), "{text}");
    assert!(text.contains("the `Delta` section has 1 line(s)"), "{text}");
    let json = next_json(&r);
    assert_eq!(
        skills(&json),
        ["brainstorm", "specify", "clarify", "clarify", "specify"]
    );
    let last = items(&text).last().copied().unwrap();
    assert_eq!(
        last,
        "[ ] set `status: approved` in docs/intent/dose/spec.md [skill: specify]"
    );
}

#[test]
fn the_stage_build_has_3_sub_cases() {
    let r = start();
    // No plan.
    stage(&r, &[(INTENT, &intent("accepted", "routine", ""))]);
    let text = next_text(&r);
    assert_stage(&r, "build");
    assert!(text.contains("stage build"), "{text}");
    assert!(text.contains("spec-dna new plan dose"), "{text}");
    assert!(skills(&next_json(&r)).contains(&"write-plan"));
    // A draft plan, and code that changed.
    stage(
        &r,
        &[
            (
                PLAN,
                &plan("draft", &["src/dose.py"], "- [ ] t1: Edit the code."),
            ),
            ("src/dose.py", "x = 2\n"),
        ],
    );
    let text = next_text(&r);
    assert_stage(&r, "build");
    assert!(text.contains("stage build"), "{text}");
    assert!(
        items(&text)
            .last()
            .unwrap()
            .starts_with("[ ] set `status: approved` in docs/intent/dose/plan.md"),
        "{text}"
    );
    let json = next_json(&r);
    assert!(json.contains("\"next_task\":{\"id\":\"t1\""), "{json}");
    // An approved plan, and no file of `Files` that changed against the base.
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (
                PLAN,
                &plan("approved", &["src/dose.py"], "- [ ] t1: Edit the code."),
            ),
        ],
    );
    let text = next_text(&r);
    assert_stage(&r, "build");
    assert!(text.contains("stage build"), "{text}");
    let last = items(&text).last().copied().unwrap();
    assert_eq!(
        last,
        "[ ] change a file that `Files` of docs/intent/dose/plan.md lists: none differs from the base [skill: execute]"
    );
    assert!(skills(&next_json(&r)).contains(&"execute"));
    // A change of a file outside `Files` does not move the stage.
    stage(&r, &[("tests/test_other.py", "# changed\n")]);
    assert_eq!(stage_of(&r), "build");
    // A change of a file of `Files` does not, while a task is open.
    stage(&r, &[("src/dose.py", "x = 2\n")]);
    assert_eq!(stage_of(&r), "build");
    // A change of a file of `Files` and no open task does.
    stage(
        &r,
        &[(
            PLAN,
            &plan("approved", &["src/dose.py"], "- [x] t1: Edit the code."),
        )],
    );
    assert_eq!(stage_of(&r), "deploy");
}

#[test]
fn tasks_under_a_heading_inside_order_are_tasks_of_the_plan() {
    // A plan with `Files: none` is done when each task is done. A heading
    // groups tasks, and is not a task: the tasks under it count.
    let order =
        "### Part one\n\n- [ ] t1: Read the code.\n\n### Part two\n\n- [ ] t2: Read the tests.";
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (PLAN, &plan_with_item("approved", "- None.", order)),
        ],
    );
    // Every task is open, so the stage stays at build.
    let text = next_text(&r);
    assert_stage(&r, "build");
    assert!(
        text.contains("tasks of `Order`: 0 of 2 done"),
        "the tasks of the headings are counted: {text}"
    );
    let json = next_json(&r);
    assert!(json.contains("\"next_task\":{\"id\":\"t1\""), "{json}");
    // The first task is done: the next task is the one under the next heading.
    let done = order.replace("- [ ] t1", "- [x] t1");
    stage(&r, &[(PLAN, &plan_with_item("approved", "- None.", &done))]);
    assert_stage(&r, "build");
    let json = next_json(&r);
    assert!(json.contains("\"next_task\":{\"id\":\"t2\""), "{json}");
    // Both tasks are done: the stage moves on.
    let done = done.replace("- [ ] t2", "- [x] t2");
    stage(&r, &[(PLAN, &plan_with_item("approved", "- None.", &done))]);
    assert_stage(&r, "deploy");
}

#[test]
fn an_add_line_without_evidence_is_in_the_stage_test_until_a_tag_names_it() {
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "higher-risk", "")),
            (SPEC, &spec("approved", ADD_FLUSH)),
            (
                PLAN,
                &plan("approved", &["src/dose.py"], "- [ ] t1: Edit the code."),
            ),
        ],
    );
    assert_eq!(stage_of(&r), "build");
    stage(&r, &[("src/dose.py", "x = 2\n")]);
    // The file differs, but the task is open.
    assert_stage(&r, "build");
    stage(
        &r,
        &[(
            PLAN,
            &plan("approved", &["src/dose.py"], "- [x] t1: Edit the code."),
        )],
    );
    let text = next_text(&r);
    assert_stage(&r, "test");
    assert!(text.contains("stage test"), "{text}");
    assert!(text.contains("1 of 1 requirement(s)"), "{text}");
    let last = items(&text).last().copied().unwrap();
    assert_eq!(
        last,
        "[ ] add `covers: requirements/valves/closing#flush` in a test file [skill: tdd]"
    );
    // The tag names the requirement before `finish` writes it.
    stage(
        &r,
        &[(
            "tests/test_valves.py",
            "# covers: requirements/valves/closing#flush\n",
        )],
    );
    let text = next_text(&r);
    assert_stage(&r, "deploy");
    assert!(text.contains("stage deploy"), "{text}");
    assert!(text.contains("each of the 1 requirement(s)"), "{text}");
}

#[test]
fn next_and_status_name_a_partial_tag_at_the_stage_test() {
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "higher-risk", "")),
            (SPEC, &spec("approved", ADD_FLUSH)),
            (
                PLAN,
                &plan("approved", &["src/dose.py"], "- [x] t1: Edit the code."),
            ),
            ("src/dose.py", "x = 2\n"),
            (
                "tests/test_valves.py",
                "# covers: partial requirements/valves/closing#flush\n",
            ),
        ],
    );
    // A partial tag is not evidence: the stage stays `test`.
    assert_stage(&r, "test");
    let text = next_text(&r);
    assert!(
        text.contains("1 of 1 requirement(s) that the change adds or changes have no full evidence; 1 of them have a partial `covers:` tag only"),
        "{text}"
    );
    assert_eq!(
        items(&text).last().copied().unwrap(),
        "[ ] `requirements/valves/closing#flush` has a partial `covers:` tag only (tests/test_valves.py:1); add a test that covers all of it [skill: tdd]"
    );
    // `status` shows the count in a token, also with a target of no tag.
    let (code, out, err) = r.run(&["status"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains(" tasks 1/1 partial 1 areas"), "{out}");
    let (_, out, _) = r.run(&["status", "--json"]);
    assert!(out.contains("\"partial\":1"), "{out}");
    // A full tag next to it is evidence.
    stage(
        &r,
        &[(
            "tests/test_all.py",
            "# covers: requirements/valves/closing#flush\n",
        )],
    );
    assert_stage(&r, "deploy");
    let (_, out, _) = r.run(&["status"]);
    assert!(!out.contains("partial"), "{out}");
    let (_, out, _) = r.run(&["status", "--json"]);
    assert!(out.contains("\"partial\":0"), "{out}");
}

#[test]
fn a_routine_requirement_with_a_new_hash_and_no_evidence_is_in_the_stage_test() {
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (
                PLAN,
                &plan(
                    "approved",
                    &["src/dose.py", CLOSING],
                    "- [x] t1: Edit the code.",
                ),
            ),
        ],
    );
    // The code changed, and no requirement changed: nothing to prove.
    stage(&r, &[("src/dose.py", "x = 2\n")]);
    let text = next_text(&r);
    assert_stage(&r, "deploy");
    assert!(text.contains("stage deploy"), "{text}");
    assert!(text.contains("adds or changes no requirement"), "{text}");
    // A requirement text changed.
    stage(
        &r,
        &[(
            CLOSING,
            &VALVES.replace("in 5 seconds.\n- **open", "in 4 seconds.\n- **open"),
        )],
    );
    let text = next_text(&r);
    assert_stage(&r, "test");
    assert!(text.contains("stage test"), "{text}");
    assert!(
        text.contains(
            "[ ] add `covers: requirements/valves/closing#close` in a test file [skill: tdd]"
        ),
        "{text}"
    );
    // The same text edit with a tag.
    stage(
        &r,
        &[(
            "tests/test_valves.py",
            "# covers: requirements/valves/closing#close\n",
        )],
    );
    assert_eq!(stage_of(&r), "deploy");
}

/// A plan with one `Files` item `item`, in the form that a plan with no files
/// has, and the tasks `order`.
fn plan_with_item(status: &str, item: &str, order: &str) -> String {
    format!(
        "---\nstatus: {status}\n---\n# Plan\n\n## Files\n\n{item}\n\n## Order\n\n{order}\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n"
    )
}

#[test]
fn a_routine_plan_with_files_none_reaches_the_finish_step() {
    // The plan states in `Files` that the work edits no file. The stage passes
    // the build step when each task is done, with no file of the diff.
    for origin in [true, false] {
        let r = start_with("policy = \"three-class\"\ngate = \"block\"\n", origin);
        stage(
            &r,
            &[
                (INTENT, &intent("accepted", "routine", "")),
                (
                    PLAN,
                    &plan_with_item(
                        "approved",
                        "- None: this change is research.",
                        "- [x] t1: Read the code.",
                    ),
                ),
            ],
        );
        let text = next_text(&r);
        assert_stage(&r, "deploy");
        assert!(
            text.contains("[x] the plan lists no files: the change edits no file"),
            "{text}"
        );
        assert!(
            text.contains("run `spec-dna finish dose --dry-run`"),
            "{text}"
        );
        assert!(!text.contains("lists none"), "{text}");
        assert!(!text.contains("are not paths"), "{text}");
    }

    // The checks of the diff do not complain about the approved plan: the
    // approval of the intent is in the base.
    let r = start_with("policy = \"three-class\"\ngate = \"block\"\n", true);
    r.git(&["checkout", "-q", "main"]);
    r.write(INTENT, &intent("accepted", "routine", ""));
    r.commit("approve the intent");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "feature"]);
    r.git(&["merge", "-q", "main"]);
    stage(
        &r,
        &[(
            PLAN,
            &plan_with_item("approved", "- None.", "- [x] t1: Read the code."),
        )],
    );
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("SD915"), "{out}");
    assert!(out.contains("0 new errors and 0 new warnings"), "{out}");
    // An edit of a file is outside the plan, and SD913 says so.
    stage(&r, &[("src/dose.py", "x = 3\n")]);
    let (_, out, _) = r.run(&["check"]);
    assert!(out.contains("SD913 outside-plan"), "{out}");

    // A plan with no task passes the build step too, and `NONE` in any case
    // and a bare `none` read as the same word.
    for item in ["- NONE", "- none: research only", "- None."] {
        let r = start();
        stage(
            &r,
            &[
                (INTENT, &intent("accepted", "routine", "")),
                (PLAN, &plan_with_item("approved", item, "")),
            ],
        );
        assert_stage(&r, "deploy");
    }

    // An open task keeps the stage at build. The text shows the task, and the
    // step of the files is done.
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (
                PLAN,
                &plan_with_item("approved", "- None.", "- [ ] t1: Read the code."),
            ),
        ],
    );
    let text = next_text(&r);
    assert_stage(&r, "build");
    assert!(text.contains("task t1: Read the code"), "{text}");
    assert!(
        text.contains("[x] the plan lists no files: the change edits no file"),
        "{text}"
    );
    assert!(!text.contains("lists none"), "{text}");
    // The last line is the open task, and so is the field `next`.
    assert!(
        items(&text).last().unwrap().starts_with("[ ] task t1"),
        "{text}"
    );
    let json = next_json(&r);
    assert!(
        field(&json, "next").starts_with("task t1: Read the code"),
        "{json}"
    );
    let (_, status, _) = r.run(&["status"]);
    assert!(status.contains("next: task t1: Read the code"), "{status}");

    // A word that only starts with `none`, and a second item, are not the
    // form.
    for item in ["- nonexistent.md", "- None.\n- Later."] {
        let r = start();
        stage(
            &r,
            &[
                (INTENT, &intent("accepted", "routine", "")),
                (PLAN, &plan_with_item("approved", item, "- [x] t1: Done.")),
            ],
        );
        assert_stage(&r, "build");
        assert!(!next_text(&r).contains("the plan lists no files"));
    }

    // A higher-risk change keeps the message: its spec has a delta.
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "higher-risk", "")),
            (SPEC, &spec("approved", ADD_FLUSH)),
            (
                PLAN,
                &plan_with_item("approved", "- None.", "- [x] t1: Done."),
            ),
        ],
    );
    let text = next_text(&r);
    assert_stage(&r, "build");
    assert!(!text.contains("the plan lists no files"), "{text}");
    // The `none` item is no wrong item, so the message is the one of a plan
    // with no `Files` item.
    assert!(text.contains("it lists none"), "{text}");
}

#[test]
fn the_stage_deploy_names_the_skills_of_its_steps_in_order() {
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (
                PLAN,
                &plan("approved", &["src/dose.py"], "- [x] t1: Edit the code."),
            ),
            ("src/dose.py", "x = 2\n"),
        ],
    );
    let json = next_json(&r);
    assert_eq!(field(&json, "stage"), "deploy");
    let s = skills(&json);
    assert_eq!(s[s.len() - 3..], ["verify", "review", "finish"]);
    let text = next_text(&r);
    assert_in_order(
        &text,
        &[
            "run the proof of docs/intent/dose/plan.md",
            "review the diff",
            "spec-dna finish dose --dry-run",
        ],
    );
    assert!(items(&text).last().unwrap().ends_with("[skill: finish]"));
}

#[test]
fn an_incident_with_an_open_follow_up_is_listed_by_status() {
    let r = start();
    r.write(
        "docs/incidents/valve-stuck.md",
        "---\nstatus: active\n---\n# Valve stuck\n\n## Summary\n\nA valve stuck.\n\n## Follow-ups\n\n- [x] Replace the valve.\n- [ ] Add an alarm for a stuck valve.\n  It must page the owner.\n",
    );
    r.commit("incident");
    let (code, out, err) = r.run(&["status", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let follow = out.split("\"follow_ups\":").nth(1).unwrap();
    assert!(
        follow.starts_with("[{\"incident\":\"incidents/valve-stuck\",\"path\":\"docs/incidents/valve-stuck.md\",\"line\":13,\"text\":\"Add an alarm for a stuck valve. It must page the owner.\""),
        "{follow}"
    );
    assert!(!follow.contains("Replace the valve"), "{follow}");
    let (_, text, _) = r.run(&["status"]);
    assert!(
        text.contains(
            "follow-up incidents/valve-stuck: docs/incidents/valve-stuck.md:13 Add an alarm"
        ),
        "{text}"
    );
}

#[test]
fn an_open_follow_up_names_the_skill_post_mortem() {
    let r = start();
    r.write(
        "docs/incidents/valve-stuck.md",
        "---\nstatus: active\n---\n# Valve stuck\n\n## Summary\n\nA valve stuck.\n\n## Follow-ups\n\n- [x] Replace the valve.\n- [ ] Add an alarm for a stuck valve.\n  It must page the owner.\n",
    );
    r.commit("incident");
    let (code, out, err) = r.run(&["status", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let follow = out.split("\"follow_ups\":").nth(1).unwrap();
    assert!(
        follow.starts_with("[{\"incident\":\"incidents/valve-stuck\",\"path\":\"docs/incidents/valve-stuck.md\",\"line\":13,\"text\":\"Add an alarm for a stuck valve. It must page the owner.\",\"skill\":\"post-mortem\"}]"),
        "{follow}"
    );
    let (_, text, _) = r.run(&["status"]);
    assert!(
        text.contains(
            "follow-up incidents/valve-stuck: docs/incidents/valve-stuck.md:13 Add an alarm for a stuck valve. It must page the owner. [skill: post-mortem]\n"
        ),
        "{text}"
    );
}

// An intent with no class (design 0.7.0, section 15, item 2)

/// The text of an intent with a `class` line that `raw` gives, or with none.
fn intent_with_class_line(status: &str, raw: Option<&str>) -> String {
    let line = raw.map(|l| format!("{l}\n")).unwrap_or_default();
    format!(
        "---\nstatus: {status}\n{line}---\n# Dose limit\n\n## Intent\n\nThe dose is too high.\n"
    )
}

/// The 3 forms of no class: absent, empty and invalid.
const NO_CLASS: [Option<&str>; 3] = [None, Some("class:"), Some("class: urgent")];

#[test]
fn an_intent_with_no_class_has_the_class_null_and_the_stage_plan() {
    for raw in NO_CLASS {
        let r = start();
        stage(&r, &[(INTENT, &intent_with_class_line("accepted", raw))]);
        let json = next_json(&r);
        assert!(json.contains("\"class\":null"), "{raw:?}: {json}");
        assert_stage(&r, "plan");
        let (_, out, _) = r.run(&["status", "--json"]);
        assert!(out.contains("\"class\":null"), "{raw:?}: {out}");
        let (_, text, _) = r.run(&["status"]);
        assert!(
            text.contains("\ndose none plan intent accepted spec - plan -"),
            "{raw:?}: {text}"
        );
        let text = next_text(&r);
        assert!(
            text.starts_with("change dose (class not set), stage plan\nbase "),
            "{raw:?}: {text}"
        );
    }
}

#[test]
fn the_last_line_of_an_intent_with_no_class_asks_for_the_class_before_the_approval() {
    for status in ["draft", "accepted"] {
        for raw in NO_CLASS {
            let r = start();
            stage(&r, &[(INTENT, &intent_with_class_line(status, raw))]);
            let text = next_text(&r);
            let last = items(&text).last().copied().unwrap();
            assert!(
                last.contains("class") && last.contains("docs/intent/dose/intent.md"),
                "{status} {raw:?}: {text}"
            );
            // A draft intent: the class comes first, then the status. An
            // accepted intent needs no status.
            match status {
                "draft" => {
                    let class_at = last.find("class: routine").unwrap();
                    let status_at = last.find("status: accepted").unwrap();
                    assert!(class_at < status_at, "{status} {raw:?}: {text}");
                }
                _ => assert!(
                    !last.contains("status: accepted"),
                    "{status} {raw:?}: {text}"
                ),
            }
            assert!(
                last.ends_with("[skill: brainstorm]"),
                "{status} {raw:?}: {text}"
            );
        }
    }
}

#[test]
fn check_still_reads_an_intent_with_no_class_as_routine() {
    // The plan of a routine change needs an accepted intent only, so the plan
    // below gives no SD910 in a base that has the accepted intent.
    let r = start();
    stage(&r, &[(INTENT, &intent_with_class_line("accepted", None))]);
    r.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    stage(
        &r,
        &[(PLAN, &plan("approved", &["src/dose.py"], "- [ ] t1: Edit."))],
    );
    let (code, out, err) = r.run(&["check"]);
    assert!(!out.contains("SD910"), "{out}{err}");
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_change_with_no_plan_or_no_spec_has_no_approval_line() {
    // A routine change with no `plan.md`: the line asks to write it, and no
    // line asks to approve it.
    let r = start();
    stage(&r, &[(INTENT, &intent("accepted", "routine", ""))]);
    let text = next_text(&r);
    assert!(!text.contains("status: approved"), "{text}");
    let last = items(&text).last().copied().unwrap();
    assert!(
        last.contains("spec-dna new plan dose") && last.ends_with("[skill: write-plan]"),
        "{text}"
    );
    let json = next_json(&r);
    assert!(
        json.contains("\"next\":\"write `docs/intent/dose/plan.md` with `spec-dna new plan dose`"),
        "{json}"
    );
    // A higher-risk change with no `spec.md`.
    let r = start();
    stage(&r, &[(INTENT, &intent("accepted", "higher-risk", ""))]);
    let text = next_text(&r);
    assert!(!text.contains("status: approved"), "{text}");
    let last = items(&text).last().copied().unwrap();
    assert!(last.contains("spec-dna new spec dose"), "{text}");
    // With the file, the line comes back.
    stage(&r, &[(SPEC, &spec("draft", ADD_FLUSH))]);
    let text = next_text(&r);
    assert!(
        items(&text)
            .last()
            .unwrap()
            .starts_with("[ ] set `status: approved` in docs/intent/dose/spec.md"),
        "{text}"
    );
}

// The tasks

const TASKS: &str = "- [x] t1: Write the test. Files: `tests/test_valves.py`. Verify: the test fails.\n- [ ] t2: Close the valve. Files: `src/dose.py`, `src/valve.py`. Verify: `pytest tests/test_valves.py`.\n- [ ] t3: Update the doc.\n- A note, not a task.";

#[test]
fn the_progress_of_the_tasks_and_the_next_open_task_are_data() {
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (PLAN, &plan("approved", &["src/dose.py"], TASKS)),
        ],
    );
    let json = next_json(&r);
    assert_eq!(field(&json, "stage"), "build");
    assert!(
        json.contains("\"next_task\":{\"id\":\"t2\",\"text\":\"Close the valve\",\"files\":[\"src/dose.py\",\"src/valve.py\"],\"verify\":\"`pytest tests/test_valves.py`\"}"),
        "{json}"
    );
    let text = next_text(&r);
    assert!(text.contains("tasks of `Order`: 1 of 3 done"), "{text}");
    assert!(
        text.contains("[ ] task t2: Close the valve. Files: `src/dose.py`, `src/valve.py`. Verify: `pytest tests/test_valves.py` [skill: execute]"),
        "{text}"
    );
    let (code, out, err) = r.run(&["status", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains(
            "\"tasks\":{\"done\":1,\"skipped\":0,\"finish\":0,\"total\":3,\"next_task\":{\"id\":\"t2\""
        ),
        "{out}"
    );
    let (_, text, _) = r.run(&["status"]);
    assert!(text.contains("tasks 1/3 next-task t2"), "{text}");
    // A plan with no task has a progress of 0 of 0, and no next task.
    stage(
        &r,
        &[(
            PLAN,
            &plan("approved", &["src/dose.py"], "- Edit the code."),
        )],
    );
    let (_, out, _) = r.run(&["status", "--json"]);
    assert!(
        out.contains(
            "\"tasks\":{\"done\":0,\"skipped\":0,\"finish\":0,\"total\":0,\"next_task\":null}"
        ),
        "{out}"
    );
    // The task of a done plan is not the next task: a duplicate id does not
    // count, and a plan with all tasks done has none.
    stage(
        &r,
        &[(
            PLAN,
            &plan(
                "approved",
                &["src/dose.py"],
                "- [x] a: One.\n- [ ] a: Duplicate.\n- [x] b: Two.",
            ),
        )],
    );
    let (_, out, _) = r.run(&["status", "--json"]);
    assert!(
        out.contains(
            "\"tasks\":{\"done\":2,\"skipped\":0,\"finish\":0,\"total\":2,\"next_task\":null}"
        ),
        "{out}"
    );
}

// A task that is not needed

const SKIPPED_TASKS: &str = "- [x] t1: Write the test. Files: `tests/test_valves.py`. Verify: the test fails.\n- [x] t2: Try the timer. Files: `src/dose.py`. Verify: read the log. Hours: 1-2. Skip: the valve has no timer.\n- [ ] t3: Close the valve. Files: `src/dose.py`. Verify: `pytest`.";

#[test]
fn a_skipped_task_is_closed_for_next_and_status() {
    let r = start();
    // The box of a skipped task does not matter: `- [ ]` and `- [x]` both close it.
    let order = SKIPPED_TASKS.replace("- [x] t2", "- [ ] t2");
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (PLAN, &plan("approved", &["src/dose.py"], &order)),
        ],
    );
    let json = next_json(&r);
    assert!(json.contains("\"next_task\":{\"id\":\"t3\""), "{json}");
    let text = next_text(&r);
    assert!(
        text.contains("tasks of `Order`: 1 of 3 done, 1 not needed"),
        "{text}"
    );
    let (code, out, err) = r.run(&["status", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains(
            "\"tasks\":{\"done\":1,\"skipped\":1,\"finish\":0,\"total\":3,\"next_task\":{\"id\":\"t3\""
        ),
        "{out}"
    );
    let (_, text, _) = r.run(&["status"]);
    assert!(text.contains("tasks 1/3 skipped 1 next-task t3"), "{text}");
    // A task with `- [x]` and `Skip:` is skipped too. With t3 done, no task is open.
    let order = SKIPPED_TASKS.replace("- [ ] t3", "- [x] t3");
    stage(&r, &[(PLAN, &plan("approved", &["src/dose.py"], &order))]);
    let (_, out, _) = r.run(&["status", "--json"]);
    assert!(
        out.contains(
            "\"tasks\":{\"done\":2,\"skipped\":1,\"finish\":0,\"total\":3,\"next_task\":null}"
        ),
        "{out}"
    );
    let (_, text, _) = r.run(&["status"]);
    assert!(
        text.contains("tasks 2/3 skipped 1\n") || text.contains("tasks 2/3 skipped 1 "),
        "{text}"
    );
    assert!(!text.contains("next-task"), "{text}");
}

#[test]
fn the_counts_name_the_skipped_tasks() {
    // The line of the tasks has no tail when no task is skipped.
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (PLAN, &plan("approved", &["src/dose.py"], TASKS)),
        ],
    );
    let text = next_text(&r);
    assert!(
        text.contains("tasks of `Order`: 1 of 3 done [skill: execute]"),
        "{text}"
    );
    assert!(!text.contains("not needed"), "{text}");
    let (_, text, _) = r.run(&["status"]);
    assert!(!text.contains("skipped"), "{text}");
    // The item is checked when done + skipped = total, and a routine plan
    // that lists no files is built then.
    stage(
        &r,
        &[(
            PLAN,
            &plan_with_item(
                "approved",
                "None.",
                "- [x] t1: Read the code.\n- [ ] t2: Try it. Skip: the code is clear.",
            ),
        )],
    );
    assert_stage(&r, "deploy");
    // With files, the stage stays at build, and the item is checked.
    stage(
        &r,
        &[(
            PLAN,
            &plan(
                "approved",
                &["src/dose.py"],
                "- [x] t1: Read the code.\n- [ ] t2: Try it. Skip: the code is clear.",
            ),
        )],
    );
    let text = next_text(&r);
    assert!(
        text.contains("[x] tasks of `Order`: 1 of 2 done, 1 not needed"),
        "{text}"
    );
    assert_stage(&r, "build");
}

// A task that closes with `finish` (0.7.1, B1)

const FINISH_TASKS: &str = "- [x] t1: Read the code. Files: none. Verify: read it.\n- [ ] t2: Run `spec-dna finish dose` and commit the result. Files: none. Verify: `spec-dna check`.";

#[test]
fn a_task_that_names_finish_with_the_slug_does_not_keep_the_stage_build() {
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (PLAN, &plan_with_item("approved", "None.", FINISH_TASKS)),
        ],
    );
    assert_stage(&r, "deploy");
    let text = next_text(&r);
    let last = items(&text).last().copied().unwrap();
    assert!(last.contains("`spec-dna finish dose`"), "{last}");
    assert!(last.ends_with("[skill: finish]"), "{last}");
    assert!(!text.contains("task t2"), "{text}");
    assert!(field(&next_json(&r), "next").contains("spec-dna finish dose"));
    assert!(
        next_json(&r).contains("\"next_task\":null"),
        "{}",
        next_json(&r)
    );
}

#[test]
fn the_counts_name_the_tasks_for_finish() {
    // The plan lists a file that the work changed, and `finish` is the last
    // open task: `next` and `status` count it apart from the done tasks.
    let r = start();
    let two = format!(
        "{FINISH_TASKS}\n- [ ] t3: Run `spec-dna finish other`.\n- [ ] t4: Skipped. Skip: not needed."
    );
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (PLAN, &plan("approved", &["src/dose.py"], &two)),
        ],
    );
    let text = next_text(&r);
    assert!(
        text.contains(
            "tasks of `Order`: 1 of 4 done, 1 not needed, 1 for `finish` [skill: execute]"
        ),
        "{text}"
    );
    // A task for another slug is a task of real work: it stays the next task.
    assert!(
        next_json(&r).contains("\"next_task\":{\"id\":\"t3\""),
        "{}",
        next_json(&r)
    );
    let (code, out, err) = r.run(&["status", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("\"tasks\":{\"done\":1,\"skipped\":1,\"finish\":1,\"total\":4,\"next_task\":{\"id\":\"t3\""),
        "{out}"
    );
    let (_, text, _) = r.run(&["status"]);
    assert!(
        text.contains("tasks 1/4 skipped 1 finish 1 next-task t3"),
        "{text}"
    );
    // With no task for `finish`, the line has no tail.
    stage(&r, &[(PLAN, &plan("approved", &["src/dose.py"], TASKS))]);
    let text = next_text(&r);
    assert!(!text.contains("for `finish`"), "{text}");
    let (_, text, _) = r.run(&["status"]);
    assert!(
        !text.contains("finish 0") && !text.contains(" finish "),
        "{text}"
    );
}

#[test]
fn a_task_for_finish_that_is_done_or_skipped_counts_as_done_or_skipped() {
    let r = start();
    let order = "- [x] t1: Run `spec-dna finish dose`.\n- [ ] t2: Run `spec-dna finish dose`. Skip: done by hand.";
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (PLAN, &plan("approved", &["src/dose.py"], order)),
        ],
    );
    let (_, out, _) = r.run(&["status", "--json"]);
    assert!(
        out.contains("\"tasks\":{\"done\":1,\"skipped\":1,\"finish\":0,\"total\":2,"),
        "{out}"
    );
}

#[test]
fn a_skip_with_no_reason_does_not_close_the_task() {
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (
                PLAN,
                &plan(
                    "approved",
                    &["src/dose.py"],
                    "- [ ] t1: Close the valve. Files: `src/dose.py`. Verify: `pytest`. Skip: .\n- [ ] t2: Two. Skip:",
                ),
            ),
        ],
    );
    let json = next_json(&r);
    assert!(json.contains("\"next_task\":{\"id\":\"t1\""), "{json}");
    let text = next_text(&r);
    assert!(
        text.contains("tasks of `Order`: 0 of 2 done [skill: execute]"),
        "{text}"
    );
    let (_, out, _) = r.run(&["status", "--json"]);
    assert!(
        out.contains("\"tasks\":{\"done\":0,\"skipped\":0,\"finish\":0,\"total\":2,"),
        "{out}"
    );
    // An empty `Skip:` and a box `- [x]` is a done task.
    stage(
        &r,
        &[(
            PLAN,
            &plan(
                "approved",
                &["src/dose.py"],
                "- [x] t1: One. Skip:\n- [ ] t2: Two.",
            ),
        )],
    );
    let (_, out, _) = r.run(&["status", "--json"]);
    assert!(
        out.contains(
            "\"tasks\":{\"done\":1,\"skipped\":0,\"finish\":0,\"total\":2,\"next_task\":{\"id\":\"t2\""
        ),
        "{out}"
    );
}

// The skill names

#[test]
fn the_skills_of_all_the_stages_are_exactly_the_list_of_the_9_names() {
    let mut seen: Vec<String> = Vec::new();
    let mut take = |json: &str| {
        for s in skills(json) {
            assert!(SKILLS.contains(&s), "`{s}` is not a skill name");
            if !seen.iter().any(|x| x == s) {
                seen.push(s.to_string());
            }
        }
    };
    let r = start();
    stage(&r, &[(INTENT, &intent("draft", "higher-risk", ""))]);
    take(&next_json(&r));
    stage(&r, &[(INTENT, &intent("accepted", "higher-risk", ""))]);
    take(&next_json(&r));
    stage(
        &r,
        &[(
            SPEC,
            &spec(
                "draft",
                &format!("{ADD_FLUSH}\n## Open questions\n\n- [ ] Q?\n"),
            ),
        )],
    );
    take(&next_json(&r));
    stage(&r, &[(SPEC, &spec("approved", ADD_FLUSH))]);
    take(&next_json(&r));
    stage(
        &r,
        &[(
            PLAN,
            &plan("approved", &["src/dose.py"], "- [x] t1: Edit the code."),
        )],
    );
    take(&next_json(&r));
    stage(&r, &[("src/dose.py", "x = 2\n")]);
    take(&next_json(&r));
    stage(
        &r,
        &[(
            "tests/test_valves.py",
            "# covers: requirements/valves/closing#flush\n",
        )],
    );
    take(&next_json(&r));
    stage(&r, &[(INTENT, &intent("closed", "higher-risk", ""))]);
    take(&next_json(&r));
    seen.sort();
    let mut want: Vec<String> = SKILLS.iter().map(|s| s.to_string()).collect();
    want.sort();
    assert_eq!(seen, want);
}

#[test]
fn the_text_output_marks_each_line_with_its_skill() {
    let r = start();
    stage(&r, &[(INTENT, &intent("draft", "routine", ""))]);
    for line in items(&next_text(&r)) {
        assert!(
            line.starts_with("[x] ") || line.starts_with("[ ] "),
            "{line}"
        );
        assert!(line.ends_with(']') && line.contains(" [skill: "), "{line}");
    }
}

// The limit of the lines

#[test]
fn the_output_has_at_most_12_lines_and_the_last_line_is_an_edit() {
    let r = start();
    let reqs: String = (1..=8)
        .map(|n| format!("- **req-{n}**: The controller SHALL do the thing {n}.\n"))
        .collect();
    let valves = format!("---\nstatus: active\n---\n# Closing\n\n## Requirements\n\n{reqs}");
    r.write(CLOSING, &valves);
    r.commit("more requirements");
    r.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    r.git(&["checkout", "-q", "-b", "work"]);
    let changed: String = (1..=8)
        .map(|n| format!("- **req-{n}**: The controller SHALL do the new thing {n}.\n"))
        .collect();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (
                PLAN,
                &plan(
                    "approved",
                    &["src/dose.py", CLOSING],
                    "- [x] t1: Edit the code.",
                ),
            ),
            ("src/dose.py", "x = 2\n"),
            (
                CLOSING,
                &format!("---\nstatus: active\n---\n# Closing\n\n## Requirements\n\n{changed}"),
            ),
        ],
    );
    let text = next_text(&r);
    assert_stage(&r, "test");
    assert!(text.contains("stage test"), "{text}");
    assert!(text.contains("8 of 8 requirement(s)"), "{text}");
    assert!(text.lines().count() <= 12, "{text}");
    let last = text.lines().last().unwrap();
    assert!(
        last.starts_with("[ ] add `covers: requirements/valves/closing#req-5`"),
        "{text}"
    );
}

// The JSON shape

#[test]
fn the_json_of_next_has_the_fields_in_order() {
    let r = start();
    stage(&r, &[(INTENT, &intent("accepted", "routine", ""))]);
    let json = next_json(&r);
    assert_in_order(
        &json,
        &[
            "{\"slug\":\"dose\"",
            "\"class\":\"routine\"",
            "\"stage\":\"build\"",
            "\"base\":\"",
            "\"items\":[{\"done\":true,\"text\":\"the intent is accepted\",\"skill\":\"brainstorm\"}",
            "\"next_task\":null",
            "\"next\":\"",
            "\"notes\":[]}",
        ],
    );
    let sha = r.git_out(&["rev-parse", "main"]);
    assert_eq!(field(&json, "base"), sha);
}

#[test]
fn the_json_of_status_has_the_fields_in_order() {
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "areas: [valves]\n")),
            (
                PLAN,
                &plan("draft", &["src/dose.py"], "- [ ] t1: Edit the code."),
            ),
        ],
    );
    let (code, out, err) = r.run(&["status", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let sha = r.git_out(&["rev-parse", "main"]);
    assert_in_order(
        &out,
        &[
            "{\"version\":\"",
            "\"policy\":\"three-class\"",
            &format!("\"base\":\"{sha}\""),
            "\"started\":1",
            "\"max_started\":3",
            "\"notes\":[]",
            "\"changes\":[{\"slug\":\"dose\"",
            "\"path\":\"docs/intent/dose\"",
            "\"title\":\"Dose limit\"",
            "\"class\":\"routine\"",
            "\"statuses\":{\"intent\":\"accepted\",\"spec\":null,\"plan\":\"draft\"}",
            "\"stage\":\"build\"",
            "\"next\":\"set `status: approved` in docs/intent/dose/plan.md\"",
            "\"stale\":false",
            "\"areas\":[\"valves\"]",
            "\"tasks\":{\"done\":0,\"skipped\":0,\"finish\":0,\"total\":1,\"next_task\":{\"id\":\"t1\"",
            "\"follow_ups\":[]}",
        ],
    );
    let (_, text, _) = r.run(&["status"]);
    assert!(text.starts_with("policy three-class, base "), "{text}");
    assert!(
        text.contains("\ndose routine build intent accepted spec - plan draft tasks 0/1 next-task t1 areas valves\n  next: set `status: approved` in docs/intent/dose/plan.md\n"),
        "{text}"
    );
    assert!(text.contains("1 started (max 3)"), "{text}");
}

// The groups

fn two_changes(r: &Repo) {
    // `dose` is in the area `valves` through its intent. `leak` has a delta
    // that targets a requirement of the area `sensors`.
    stage(
        r,
        &[
            (INTENT, &intent("accepted", "routine", "areas: [valves]\n")),
            (
                "docs/intent/leak/intent.md",
                &intent("accepted", "higher-risk", ""),
            ),
            (
                "docs/intent/leak/spec.md",
                &spec(
                    "draft",
                    "## Delta\n\n- ADD requirements/sensors/timeouts#alarm: The controller SHALL sound an alarm.\n",
                ),
            ),
            (
                "docs/intent/misc/intent.md",
                &intent("draft", "routine", ""),
            ),
        ],
    );
}

fn slugs(json: &str) -> Vec<&str> {
    json.split("{\"slug\":\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap())
        .collect()
}

#[test]
fn a_group_lists_the_changes_that_have_one_of_its_areas() {
    let r = start();
    two_changes(&r);
    let (code, all, _) = r.run(&["status", "--json"]);
    assert_eq!(code, 0, "{all}");
    assert_eq!(slugs(&all), ["dose", "leak", "misc"]);
    assert!(all.contains("\"areas\":[\"sensors\"]"), "{all}");
    let (_, ops, _) = r.run(&["status", "--group", "operations", "--json"]);
    assert_eq!(slugs(&ops), ["dose"]);
    let (_, rep, _) = r.run(&["status", "--group", "reporting", "--json"]);
    assert_eq!(slugs(&rep), ["leak"]);
    // The counts are the counts of the project.
    assert!(rep.contains("\"started\":2"), "{rep}");
    let (code, text, _) = r.run(&["status", "--group=reporting"]);
    assert_eq!(code, 0, "{text}");
    assert!(text.contains("leak higher-risk design"), "{text}");
    assert!(!text.contains("dose "), "{text}");
}

#[test]
fn an_unknown_group_is_a_usage_error() {
    let r = start();
    two_changes(&r);
    let (code, out, err) = r.run(&["status", "--group", "nothing"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.contains("not a group") && err.contains("operations, reporting"),
        "{err}"
    );
    // A project with no `[groups]` table.
    let r = Repo::with_config(
        "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n[change]\npolicy = \"three-class\"\n",
    );
    let (code, _, err) = r.run(&["status", "--group", "x"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("no [groups] table"), "{err}");
    // `next` has no such option.
    let r = start();
    stage(&r, &[(INTENT, &intent("draft", "routine", ""))]);
    let (code, _, err) = r.run(&["next", "dose", "--group", "operations"]);
    assert_eq!(code, 2, "{err}");
}

// The base

#[test]
fn without_a_base_the_commands_print_a_note_and_the_stage_stops_at_build() {
    let r = start_with("policy = \"three-class\"\ngate = \"block\"\n", false);
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (
                PLAN,
                &plan("approved", &["src/dose.py"], "- [x] t1: Edit the code."),
            ),
            ("src/dose.py", "x = 2\n"),
        ],
    );
    let note = "no base for the change checks; pass --base <rev>, or run git remote set-head origin <branch>, or set [change] base";
    let json = next_json(&r);
    assert_eq!(field(&json, "stage"), "build");
    assert!(json.contains("\"base\":null"), "{json}");
    assert!(json.contains(&format!("\"notes\":[\"{note}\"]")), "{json}");
    let text = next_text(&r);
    assert!(text.contains(&format!("base none: {note}")), "{text}");
    assert!(
        text.contains("the base is unknown, so pass --base <rev>"),
        "{text}"
    );
    let (_, out, _) = r.run(&["status", "--json"]);
    assert!(out.contains("\"base\":null") && out.contains(note), "{out}");
    assert!(out.contains("\"stale\":false"), "{out}");
    let (_, out, _) = r.run(&["status"]);
    assert!(out.contains(&format!("note: {note}")), "{out}");
    // `--base` gives the base.
    let (code, out, err) = r.run(&["next", "dose", "--base", "main", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(field(&out, "stage"), "deploy");
    assert!(out.contains("\"notes\":[]"), "{out}");
}

#[test]
fn a_started_change_with_no_commit_for_a_long_time_is_stale() {
    let r = Repo::with_config(&config(
        "policy = \"three-class\"\ngate = \"warn\"\n",
        "stale-days = 10\n",
    ));
    r.write(CLOSING, VALVES);
    r.write(INTENT, &intent("accepted", "routine", ""));
    r.commit_at("change", "2001-01-01T12:00:00+00:00");
    r.write("src/other.py", "y = 1\n");
    r.commit_at("later", "2001-06-01T12:00:00+00:00");
    origin_head(&r.dir, "main");
    let (code, out, err) = r.run(&["status", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"stale\":true"), "{out}");
    let (_, text, _) = r.run(&["status"]);
    assert!(text.contains(" stale\n"), "{text}");
}

#[test]
fn an_unknown_base_is_a_configuration_error() {
    let r = start();
    stage(&r, &[(INTENT, &intent("draft", "routine", ""))]);
    for cmd in [&["next", "dose"][..], &["status"]] {
        let mut args = cmd.to_vec();
        args.extend(["--base", "no-such-branch"]);
        let (code, _, err) = r.run(&args);
        assert_eq!(code, 3, "{err}");
        assert!(err.contains("not a commit"), "{err}");
    }
}

// The policy none, the exit codes and the pin

#[test]
fn the_policy_none_says_that_the_change_process_is_off() {
    // No commit, no `origin/HEAD`: the commands read no git data.
    let r = Repo::with_config("[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n");
    r.write(INTENT, &intent("accepted", "routine", ""));
    let (code, out, err) = r.run(&["status"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        out,
        "the change process is off: `[change] policy` is `none`; no change is listed\n"
    );
    let (code, out, err) = r.run(&["next", "dose"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        out,
        "the change process is off: `[change] policy` is `none`\n"
    );
    let (_, out, _) = r.run(&["status", "--json"]);
    assert!(
        out.contains("\"policy\":\"none\",\"base\":null,\"started\":0,\"max_started\":null")
            && out.contains("\"changes\":[],\"follow_ups\":[]}"),
        "{out}"
    );
    let (_, out, _) = r.run(&["next", "dose", "--json"]);
    assert!(
        out.contains("\"items\":[],\"next_task\":null,\"next\":null"),
        "{out}"
    );
    // A policy that is `none` still has a usage error for a bad slug.
    let (code, _, err) = r.run(&["next", "a/b"]);
    assert_eq!(code, 2, "{err}");
}

#[test]
fn the_exit_codes() {
    let r = start();
    stage(&r, &[(INTENT, &intent("draft", "routine", ""))]);
    // No folder.
    let (code, _, err) = r.run(&["next", "nothing"]);
    assert_eq!(code, 5, "{err}");
    assert!(err.contains("no folder"), "{err}");
    // A slug that is not a name.
    for args in [&["next", "a/b"][..], &["next"], &["status", "extra"]] {
        let (code, _, err) = r.run(args);
        assert_eq!(code, 2, "{args:?}: {err}");
    }
    // A change that exists, and a repository that has none.
    let (code, out, _) = r.run(&["next", "dose"]);
    assert_eq!(code, 0, "{out}");
    let (code, out, _) = r.run(&["status"]);
    assert_eq!(code, 0, "{out}");
    let empty = start();
    let (code, out, _) = empty.run(&["status"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("no change\n"), "{out}");
    // No configuration.
    let bare = Repo::new();
    let (code, _, err) = bare.run(&["status"]);
    assert_eq!(code, 3, "{err}");
}

#[test]
fn a_pin_mismatch_is_a_warning() {
    let r = start();
    stage(&r, &[(INTENT, &intent("draft", "routine", ""))]);
    let cfg = r.read("spec-dna.toml").replacen(VERSION, "0.0.1", 1);
    r.write("spec-dna.toml", &cfg);
    for args in [&["next", "dose"][..], &["status"]] {
        let (code, out, err) = r.run(args);
        assert_eq!(code, 0, "{out}{err}");
        assert!(err.contains("warning: spec-dna.toml pins 0.0.1"), "{err}");
    }
}

#[test]
fn the_help_of_the_commands_names_the_skills_and_the_exit_codes() {
    let r = Repo::new();
    let (code, out, _) = r.run(&["next", "--help"]);
    assert_eq!(code, 0);
    assert!(
        out.starts_with("usage: spec-dna next <slug> [--base <rev>]\n"),
        "{out}"
    );
    for s in SKILLS {
        assert!(out.contains(s), "`{s}` is not in the help of `next`");
    }
    let (code, out, _) = r.run(&["status", "--help"]);
    assert_eq!(code, 0);
    assert!(
        out.starts_with("usage: spec-dna status [--group <name>] [--base <rev>]\n"),
        "{out}"
    );
}

// A paragraph under `Files` (design 0.7.0, D28)

const D28_LINE: &str = "the section `Files` has a paragraph and no list item; write the paths as a list, or write `- none`";

#[test]
fn a_none_paragraph_under_files_states_no_files() {
    for para in [
        "None.",
        "none",
        "NONE: this change is research.",
        "None: the change edits no file.",
    ] {
        let r = start();
        stage(
            &r,
            &[
                (INTENT, &intent("accepted", "routine", "")),
                (
                    PLAN,
                    &plan_with_item("approved", para, "- [x] t1: Read the code."),
                ),
            ],
        );
        let text = next_text(&r);
        assert_stage(&r, "deploy");
        assert!(
            text.contains("[x] the plan lists no files: the change edits no file"),
            "{para}: {text}"
        );
        assert!(!text.contains("lists none"), "{para}: {text}");
        assert!(!text.contains(D28_LINE), "{para}: {text}");
    }
    // An open task keeps the stage at build, and the paragraph is no wrong text.
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (
                PLAN,
                &plan_with_item("approved", "None.", "- [ ] t1: Read the code."),
            ),
        ],
    );
    let text = next_text(&r);
    assert_stage(&r, "build");
    assert!(!text.contains(D28_LINE), "{text}");
    assert!(
        items(&text).last().unwrap().starts_with("[ ] task t1"),
        "{text}"
    );
}

#[test]
fn another_paragraph_under_files_gives_the_line_of_d28() {
    // "None of the old files stay" is no `none` paragraph: the next character
    // after the word is a space.
    for para in [
        "The dose code and its test.",
        "None of the old files stay.",
        "nonexistent.md",
    ] {
        let r = start();
        stage(
            &r,
            &[
                (INTENT, &intent("accepted", "routine", "")),
                (PLAN, &plan_with_item("approved", para, "- [x] t1: Done.")),
            ],
        );
        let text = next_text(&r);
        assert_stage(&r, "build");
        // The line is the last line, in place of the line "it lists none".
        assert_eq!(
            *items(&text).last().unwrap(),
            format!("[ ] {D28_LINE} [skill: execute]"),
            "{para}: {text}"
        );
        assert!(!text.contains("it lists none"), "{para}: {text}");
        assert!(!text.contains("the plan lists no files"), "{para}: {text}");
        let json = next_json(&r);
        assert_eq!(field(&json, "next"), D28_LINE, "{para}: {json}");
        let (code, status, err) = r.run(&["status"]);
        assert_eq!(code, 0, "{status}{err}");
        assert!(status.contains(&format!("next: {D28_LINE}")), "{status}");
    }
}

#[test]
fn a_draft_plan_with_a_paragraph_under_files_gives_the_line_before_the_approval() {
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (
                PLAN,
                &plan_with_item("draft", "The dose code.", "- [ ] t1: Edit the code."),
            ),
        ],
    );
    let text = next_text(&r);
    assert_in_order(
        &text,
        &[
            D28_LINE,
            "tasks of `Order`: 0 of 1 done",
            "task t1: Edit the code",
            "set `status: approved` in docs/intent/dose/plan.md",
        ],
    );
    // The approval line stays the last line.
    assert!(
        items(&text)
            .last()
            .unwrap()
            .starts_with("[ ] set `status: approved` in docs/intent/dose/plan.md"),
        "{text}"
    );
    let json = next_json(&r);
    assert!(
        field(&json, "next").starts_with("set `status: approved`"),
        "{json}"
    );
    // A draft plan with a `none` paragraph has no such line.
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (
                PLAN,
                &plan_with_item("draft", "None.", "- [ ] t1: Edit the code."),
            ),
        ],
    );
    assert!(!next_text(&r).contains(D28_LINE));
}

#[test]
fn a_list_item_under_files_decides_over_a_paragraph() {
    // The paragraph is text before the list. The list gives the paths.
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (
                PLAN,
                &plan_with_item(
                    "approved",
                    "The files of the work.\n\n- `src/dose.py`",
                    "- [x] t1: Done.",
                ),
            ),
        ],
    );
    let text = next_text(&r);
    assert!(!text.contains(D28_LINE), "{text}");
    assert!(
        text.contains("change a file that `Files` of docs/intent/dose/plan.md lists"),
        "{text}"
    );
    // A `none` paragraph and a path in the list: the plan lists a path.
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (
                PLAN,
                &plan_with_item("approved", "None.\n\n- `src/dose.py`", "- [x] t1: Done."),
            ),
        ],
    );
    let text = next_text(&r);
    assert!(!text.contains("the plan lists no files"), "{text}");
    assert!(!text.contains(D28_LINE), "{text}");
}

#[test]
fn a_higher_risk_plan_with_a_paragraph_under_files_gives_the_line_of_d28() {
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "higher-risk", "")),
            (SPEC, &spec("approved", ADD_FLUSH)),
            (
                PLAN,
                &plan_with_item("approved", "The dose code.", "- [x] t1: Done."),
            ),
        ],
    );
    let text = next_text(&r);
    assert_stage(&r, "build");
    assert_eq!(
        *items(&text).last().unwrap(),
        format!("[ ] {D28_LINE} [skill: execute]"),
        "{text}"
    );
}

// An open task gates the stage `build` of each plan, not only of a plan that
// lists no files (0.7.1, item 1).

#[test]
fn an_open_task_keeps_a_plan_with_files_in_the_stage_build() {
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (
                PLAN,
                &plan(
                    "approved",
                    &["src/dose.py"],
                    "- [ ] t1: Edit the code.\n- [ ] t2: Test the code.",
                ),
            ),
            ("src/dose.py", "x = 2\n"),
        ],
    );
    // The file differs from the base, but 2 tasks are open.
    let text = next_text(&r);
    assert_stage(&r, "build");
    assert!(text.contains("tasks of `Order`: 0 of 2 done"), "{text}");
    assert!(text.contains("t1"), "{text}");
    let last = items(&text).last().copied().unwrap();
    assert!(
        last.contains("t1") && last.ends_with("[skill: execute]"),
        "{last}"
    );
    assert!(!text.contains("none differs from the base"), "{text}");
    assert!(
        field(&next_json(&r), "next").contains("t1"),
        "{}",
        next_json(&r)
    );
    // A done task and a skipped task close the list: the stage moves on.
    stage(
        &r,
        &[(
            PLAN,
            &plan(
                "approved",
                &["src/dose.py"],
                "- [x] t1: Edit the code.\n- [ ] t2: Test the code. Skip: the code has a test.",
            ),
        )],
    );
    assert_stage(&r, "deploy");
}

#[test]
fn an_open_task_keeps_a_higher_risk_change_with_a_tag_in_the_stage_build() {
    // The case of the owner: each delta requirement has a `covers:` tag, no
    // task is done, and the stage was `deploy`.
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "higher-risk", "")),
            (SPEC, &spec("approved", ADD_FLUSH)),
            (
                PLAN,
                &plan(
                    "approved",
                    &["src/dose.py"],
                    "- [ ] t1: Edit the code.\n- [ ] t2: Test the code.",
                ),
            ),
            ("src/dose.py", "x = 2\n"),
            (
                "tests/test_valves.py",
                "# covers: requirements/valves/closing#flush\n",
            ),
        ],
    );
    assert_stage(&r, "build");
    let text = next_text(&r);
    assert!(text.contains("tasks of `Order`: 0 of 2 done"), "{text}");
    let last = items(&text).last().copied().unwrap();
    assert!(last.contains("t1"), "{last}");
    stage(
        &r,
        &[(
            PLAN,
            &plan(
                "approved",
                &["src/dose.py"],
                "- [x] t1: Edit the code.\n- [x] t2: Test the code.",
            ),
        )],
    );
    assert_stage(&r, "deploy");
}

#[test]
fn an_open_task_keeps_a_plan_that_lists_no_files_in_the_stage_build() {
    let r = start();
    stage(
        &r,
        &[
            (INTENT, &intent("accepted", "routine", "")),
            (
                PLAN,
                &plan_with_item("approved", "- None.", "- [x] t1: Read.\n- [ ] t2: Write."),
            ),
        ],
    );
    assert_stage(&r, "build");
}
