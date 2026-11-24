//! End-to-end tests of SD913 in the pull request that runs `finish`.
//!
//! The pull request of `finish` deletes the folder of the change, so the plan,
//! and its `Files` list, is not in the head tree. SD913 must read the scope of
//! the change from the base tree. The tests use a real git repository, a real
//! merge base and the real `finish` command.

mod common;

use common::*;

const VALVES: &str = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n- **open**: The controller SHALL open the valve in 5 seconds.\n";
const CONFIG: &str = "profile = \"standard\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n";

/// The configuration with the group `acceptance` off. These tests are about
/// SD913, and a plan that the branch adds would need an acceptance record.
fn no_record(config: &str) -> String {
    config.replace(
        "SD608 = \"off\"\n",
        "SD608 = \"off\"\nacceptance = \"off\"\n",
    )
}

const INTENT: &str = "---\nstatus: accepted\nclass: higher-risk\n---\n# Dose\n\n## Intent\n\nThe dose is too high.\n";
const PLAN: &str = "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/dose.py`\n- `tests/test_valves.py`\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n";
/// The same plan, with plain paths: no code span in `Files`.
const PLAN_PLAIN: &str = "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- src/dose.py\n- tests/test_valves.py.\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n";
/// A plan whose `Files` items are prose: they are not paths.
const PLAN_PROSE: &str = "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- the dose code and its test\n- no code here\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n";
const COVERS: &str = "# covers: requirements/valves#close\n";

/// The repository after the pull request of a change: the base commit on
/// `main` (with `origin/HEAD`), and on the branch `feature` the commit of the
/// implementation and the commit of `finish`. `extra` is an edit of the
/// implementation commit, as `(path, text)`. A `routine` change has no spec.
fn finished(routine: bool, extra: &[(&str, &str)]) -> Repo {
    finished_with(PLAN, routine, extra)
}

/// As `finished`, with the text of the plan.
fn finished_with(plan: &str, routine: bool, extra: &[(&str, &str)]) -> Repo {
    let r = Repo::with_config(CONFIG);
    r.write("docs/requirements/valves.md", VALVES);
    r.write("src/dose.py", "x = 1\n");
    r.write("src/other.py", "y = 1\n");
    let (code, out, err) = r.run(&["show", "requirements/valves#close", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let hash = out
        .split("\"hash\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("a hash")
        .to_string();
    let class = if routine { "routine" } else { "higher-risk" };
    r.write(
        "docs/intent/dose/intent.md",
        &INTENT.replace("higher-risk", class),
    );
    r.write("docs/intent/dose/plan.md", plan);
    if !routine {
        r.write(
            "docs/intent/dose/spec.md",
            &format!(
                "---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n- CHANGE requirements/valves#close@{hash}: The controller SHALL close the valve in 4 seconds.\n"
            ),
        );
    }
    r.commit_base_of_0_7_1("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    // The implementation.
    r.write("src/dose.py", "x = 2\n");
    r.write("tests/test_valves.py", COVERS);
    for (p, t) in extra {
        r.write(p, t);
    }
    r.commit("implement");
    // The pull request runs `finish` and commits its result.
    let (code, out, err) = r.run(&["finish", "dose"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        !r.dir.join("docs/intent/dose").exists(),
        "finish removes the folder"
    );
    r.commit("finish");
    r
}

fn sd913(out: &str) -> Vec<&str> {
    out.lines().filter(|l| l.contains(" SD913 ")).collect()
}

#[test]
fn files_of_the_deleted_plan_cover_the_implementation() {
    let r = finished(false, &[]);
    let (code, out, err) = r.run(&["check", "--base", "main"]);
    assert!(sd913(&out).is_empty(), "{out}{err}");
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn files_of_the_deleted_plan_of_a_routine_change_cover_the_implementation() {
    let r = finished(true, &[]);
    let (code, out, err) = r.run(&["check", "--base", "main"]);
    assert!(sd913(&out).is_empty(), "{out}{err}");
    assert_eq!(code, 0, "{out}{err}");
    let r = finished(true, &[("src/other.py", "y = 2\n")]);
    let (_, out, err) = r.run(&["check", "--base", "main"]);
    assert_eq!(sd913(&out).len(), 1, "{out}{err}");
}

#[test]
fn a_file_in_no_files_list_still_gives_sd913_after_finish() {
    let r = finished(false, &[("src/other.py", "y = 2\n")]);
    let (code, out, err) = r.run(&["check", "--base", "main"]);
    let found = sd913(&out);
    assert_eq!(found.len(), 1, "{out}{err}");
    assert!(found[0].starts_with("src/other.py:"), "{out}");
    // Only the foreign file is outside: the files of the plan are not.
    assert!(!out.contains("src/dose.py"), "{out}");
    assert!(!out.contains("tests/test_valves.py"), "{out}");
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn json_output_has_no_sd913_for_the_files_of_the_deleted_plan() {
    let r = finished(false, &[]);
    let (code, out, err) = r.run(&["check", "--base", "main", "--json"]);
    assert!(!out.contains("SD913"), "{out}{err}");
    assert_eq!(code, 0, "{out}{err}");
    // The negative case, in JSON.
    let r = finished(false, &[("src/other.py", "y = 2\n")]);
    let (code, out, err) = r.run(&["check", "--base", "main", "--json"]);
    assert_eq!(out.matches("\"code\":\"SD913\"").count(), 1, "{out}{err}");
    assert!(out.contains("\"path\":\"src/other.py\""), "{out}");
    assert_eq!(code, 0, "{out}{err}");
}

/// A plan whose first `Files` item has 2 code spans: each span is a path.
const PLAN_TWO_SPANS: &str = "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/dose.py`, `tests/test_valves.py`: the dose code and its test.\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n";

#[test]
fn each_code_span_of_a_files_item_covers_the_implementation() {
    for routine in [false, true] {
        let r = finished_with(PLAN_TWO_SPANS, routine, &[]);
        let (code, out, err) = r.run(&["check", "--base", "main"]);
        assert!(sd913(&out).is_empty(), "routine={routine}: {out}{err}");
        assert_eq!(code, 0, "routine={routine}: {out}{err}");
    }
    // A file that no span names is still outside.
    let r = finished_with(PLAN_TWO_SPANS, false, &[("src/other.py", "y = 2\n")]);
    let (_, out, err) = r.run(&["check", "--base", "main"]);
    let found = sd913(&out);
    assert_eq!(found.len(), 1, "{out}{err}");
    assert!(found[0].starts_with("src/other.py:"), "{out}");
}

#[test]
fn plain_paths_in_files_cover_the_implementation_after_finish() {
    for routine in [false, true] {
        let r = finished_with(PLAN_PLAIN, routine, &[]);
        let (code, out, err) = r.run(&["check", "--base", "main"]);
        assert!(sd913(&out).is_empty(), "routine={routine}: {out}{err}");
        assert_eq!(code, 0, "routine={routine}: {out}{err}");
        let (code, out, err) = r.run(&["check", "--base", "main", "--json"]);
        assert!(!out.contains("SD913"), "routine={routine}: {out}{err}");
        assert_eq!(code, 0, "{out}{err}");
    }
}

#[test]
fn a_file_outside_plain_files_still_gives_sd913_after_finish() {
    let r = finished_with(PLAN_PLAIN, false, &[("src/other.py", "y = 2\n")]);
    let (code, out, err) = r.run(&["check", "--base", "main"]);
    let found = sd913(&out);
    assert_eq!(found.len(), 1, "{out}{err}");
    assert!(found[0].starts_with("src/other.py:"), "{out}");
    assert_eq!(code, 0, "{out}{err}");
}

/// `next` for an approved plan whose `Files` items are prose. The plan lists
/// no path, and the message must say why.
#[test]
fn next_says_that_the_items_of_files_are_not_paths() {
    let r = Repo::with_config(CONFIG);
    r.write("src/dose.py", "x = 1\n");
    r.write("docs/intent/dose/intent.md", INTENT);
    r.write("docs/intent/dose/spec.md", "---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n- ADD requirements/valves#flush: The controller SHALL flush the line.\n");
    r.write("docs/requirements/valves.md", VALVES);
    r.write("docs/intent/dose/plan.md", PLAN_PROSE);
    r.commit_base_of_0_7_1("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("src/dose.py", "x = 2\n");
    r.commit("work");
    let (code, out, err) = r.run(&["next", "dose"]);
    assert_eq!(code, 0, "{out}{err}");
    // The plan of 0.7.1 gets the note of an old plan after the checklist
    // (design 0.8.1, D86), so the last item is the last line before it.
    let last = out.lines().rfind(|l| !l.starts_with("note: ")).unwrap();
    assert!(
        last.contains("not paths") && last.contains("the dose code and its test"),
        "{out}"
    );
    assert!(!last.contains("lists none"), "{out}");
    // `--json` gives the same text.
    let (code, out, err) = r.run(&["next", "dose", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("not paths"), "{out}{err}");
}

/// SD913 names the cause when a plan of the diff has items that are not paths.
#[test]
fn sd913_names_the_items_of_files_that_are_not_paths() {
    let r = finished_with(PLAN_PROSE, false, &[]);
    let (_, out, err) = r.run(&["check", "--base", "main"]);
    let found = sd913(&out);
    assert!(!found.is_empty(), "{out}{err}");
    assert!(
        found
            .iter()
            .all(|l| l.contains("not a path") && l.contains("the dose code and its test")),
        "{out}"
    );
}

/// SD915 names the items of `Files` that are not paths, for a change that has
/// no delta and a diff with no `Files` path.
#[test]
fn sd915_names_the_items_of_files_that_are_not_paths() {
    let r = Repo::with_config(CONFIG);
    r.write("src/dose.py", "x = 1\n");
    r.write(
        "docs/intent/dose/intent.md",
        &INTENT.replace("higher-risk", "routine"),
    );
    r.write("docs/intent/dose/plan.md", PLAN_PROSE);
    r.commit_base_of_0_7_1("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    // The pull request deletes the folder and changes no file of the plan.
    r.git(&["rm", "-q", "-r", "docs/intent/dose"]);
    r.commit("delete the folder");
    let (_, out, err) = r.run(&["check", "--base", "main"]);
    let found: Vec<&str> = out.lines().filter(|l| l.contains(" SD915 ")).collect();
    assert_eq!(found.len(), 1, "{out}{err}");
    assert!(
        found[0].contains("not a path") && found[0].contains("the dose code and its test"),
        "{out}"
    );
    assert!(found[0].contains("no `Files` path and no requirement hash"));
}

/// The plan of the first scenario: it lists one file.
const PLAN_ONE: &str = "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/dose.py`\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n";

/// The repository after a pull request that adds or edits the plan, does the
/// work and runs `finish`. The base has the intent (and the spec of a
/// `higher-risk` change), and a plan only if `base_plan` is `Some`. The branch
/// writes `branch_plan` with the code, then runs `finish` in its own commit.
fn finished_on_branch(base_plan: Option<&str>, branch_plan: &str, routine: bool) -> Repo {
    let r = Repo::with_config(&no_record(CONFIG));
    r.write("docs/requirements/valves.md", VALVES);
    r.write("src/dose.py", "x = 1\n");
    let (_, out, _) = r.run(&["show", "requirements/valves#close", "--json"]);
    let hash = out
        .split("\"hash\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("a hash")
        .to_string();
    let class = if routine { "routine" } else { "higher-risk" };
    r.write(
        "docs/intent/dose/intent.md",
        &INTENT.replace("higher-risk", class),
    );
    if !routine {
        r.write(
            "docs/intent/dose/spec.md",
            &format!(
                "---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n- CHANGE requirements/valves#close@{hash}: The controller SHALL close the valve in 4 seconds.\n"
            ),
        );
    }
    if let Some(p) = base_plan {
        r.write("docs/intent/dose/plan.md", p);
    }
    r.commit_base_of_0_7_1("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("docs/intent/dose/plan.md", branch_plan);
    r.write("src/dose.py", "x = 2\n");
    r.write("tests/test_valves.py", COVERS);
    r.commit("plan and implement");
    let (code, out, err) = r.run(&["finish", "dose"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join("docs/intent/dose").exists());
    r.commit("finish");
    r
}

#[test]
fn a_plan_added_on_the_branch_covers_the_implementation_after_finish() {
    for routine in [false, true] {
        let r = finished_on_branch(None, PLAN, routine);
        let (_, out, err) = r.run(&["check", "--base", "main"]);
        assert!(sd913(&out).is_empty(), "routine={routine}: {out}{err}");
        let (_, out, err) = r.run(&["check", "--base", "main", "--json"]);
        assert!(!out.contains("SD913"), "routine={routine}: {out}{err}");
    }
}

#[test]
fn a_file_added_to_files_on_the_branch_is_in_scope_after_finish() {
    for routine in [false, true] {
        let r = finished_on_branch(Some(PLAN_ONE), PLAN, routine);
        let (_, out, err) = r.run(&["check", "--base", "main"]);
        assert!(sd913(&out).is_empty(), "routine={routine}: {out}{err}");
    }
}

/// A file that no version of `Files` lists still gives SD913.
#[test]
fn a_file_in_no_version_of_files_still_gives_sd913() {
    for base_plan in [None, Some(PLAN_ONE)] {
        let r = Repo::with_config(&no_record(CONFIG));
        r.write("docs/requirements/valves.md", VALVES);
        r.write("src/dose.py", "x = 1\n");
        r.write("src/other.py", "y = 1\n");
        r.write(
            "docs/intent/dose/intent.md",
            &INTENT.replace("higher-risk", "routine"),
        );
        if let Some(p) = base_plan {
            r.write("docs/intent/dose/plan.md", p);
        }
        r.commit_base_of_0_7_1("base");
        origin_head(&r.dir, "main");
        r.git(&["checkout", "-q", "-b", "feature"]);
        r.write("docs/intent/dose/plan.md", PLAN);
        r.write("src/dose.py", "x = 2\n");
        r.write("src/other.py", "y = 2\n");
        r.write("tests/test_valves.py", COVERS);
        r.commit("plan and implement");
        let (code, out, err) = r.run(&["finish", "dose"]);
        assert_eq!(code, 0, "{out}{err}");
        r.commit("finish");
        let (_, out, err) = r.run(&["check", "--base", "main"]);
        let found = sd913(&out);
        assert_eq!(found.len(), 1, "{out}{err}");
        assert!(found[0].starts_with("src/other.py:"), "{out}");
    }
}

/// The note about an item that is not a path appears only if the plan has no
/// path at all.
#[test]
fn the_note_about_items_that_are_not_paths_needs_a_plan_with_no_path() {
    let mixed = "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/dose.py`\n- the test files\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n";
    let r = finished_with(mixed, false, &[]);
    let (_, out, err) = r.run(&["check", "--base", "main"]);
    let found = sd913(&out);
    assert_eq!(found.len(), 1, "{out}{err}");
    assert!(found[0].starts_with("tests/test_valves.py:"), "{out}");
    assert!(!found[0].contains("not a path"), "{out}");
    assert!(!found[0].contains("the test files"), "{out}");
}

/// The `Files` of the task lines of `Order` are in the scope.
#[test]
fn the_files_of_the_task_lines_are_in_scope_after_finish() {
    let plan = "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- None.\n\n## Order\n\n- [x] t1: Edit the code. Files: src/dose.py, tests/test_valves.py. Verify: run the tests.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n";
    for routine in [false, true] {
        let r = finished_with(plan, routine, &[]);
        let (_, out, err) = r.run(&["check", "--base", "main"]);
        assert!(sd913(&out).is_empty(), "routine={routine}: {out}{err}");
        let r = finished_with(plan, routine, &[("src/other.py", "y = 2\n")]);
        let (_, out, err) = r.run(&["check", "--base", "main"]);
        assert_eq!(sd913(&out).len(), 1, "routine={routine}: {out}{err}");
    }
}

/// The plan lists its path in a task line only: `Files` is `- None.`.
const PLAN_TASK_ONLY: &str = "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- None.\n\n## Order\n\n- [x] t1: Edit the code. Files: src/dose.py. Verify: run the tests.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n";

/// Every reader of `Files` sees the paths of the task lines. After `finish`,
/// a diff that edits only that path gives neither SD913 nor SD915.
#[test]
fn a_path_in_a_task_line_only_gives_no_sd913_and_no_sd915_after_finish() {
    for routine in [true, false] {
        let r = Repo::with_config(CONFIG);
        r.write("docs/requirements/valves.md", VALVES);
        r.write("src/dose.py", "x = 1\n");
        let class = if routine { "routine" } else { "higher-risk" };
        r.write(
            "docs/intent/dose/intent.md",
            &INTENT.replace("higher-risk", class),
        );
        if !routine {
            let (_, out, _) = r.run(&["show", "requirements/valves#close", "--json"]);
            let hash = out
                .split("\"hash\":\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
                .expect("a hash")
                .to_string();
            r.write(
                "docs/intent/dose/spec.md",
                &format!("---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n- CHANGE requirements/valves#close@{hash}: The controller SHALL close the valve in 4 seconds.\n"),
            );
        }
        r.write("docs/intent/dose/plan.md", PLAN_TASK_ONLY);
        r.commit_base_of_0_7_1("base");
        origin_head(&r.dir, "main");
        r.git(&["checkout", "-q", "-b", "feature"]);
        r.write("src/dose.py", "x = 2\n");
        if !routine {
            r.write("tests/test_valves.py", COVERS);
        }
        r.commit("implement");
        let (code, out, err) = r.run(&["finish", "dose"]);
        assert_eq!(code, 0, "{out}{err}");
        r.commit("finish");
        let (_, out, err) = r.run(&["check", "--base", "main"]);
        if routine {
            assert!(!out.contains(" SD915 "), "routine={routine}: {out}{err}");
        }
        let outside: Vec<&str> = sd913(&out)
            .into_iter()
            .filter(|l| !l.starts_with("tests/test_valves.py:"))
            .collect();
        assert!(outside.is_empty(), "routine={routine}: {out}{err}");
        assert!(!out.contains("not a path"), "routine={routine}: {out}{err}");
    }
}

/// `next` for a plan whose paths are in its task lines does not say that the
/// items of `Files` are not paths.
#[test]
fn next_reads_the_paths_of_the_task_lines() {
    let r = Repo::with_config(CONFIG);
    r.write("src/dose.py", "x = 1\n");
    r.write("docs/intent/dose/intent.md", INTENT);
    r.write("docs/intent/dose/spec.md", "---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n- ADD requirements/valves#flush: The controller SHALL flush the line.\n");
    r.write("docs/requirements/valves.md", VALVES);
    r.write("docs/intent/dose/plan.md", PLAN_TASK_ONLY);
    r.commit_base_of_0_7_1("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("src/dose.py", "x = 2\n");
    r.commit("work");
    let (code, out, err) = r.run(&["next", "dose"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("not paths"), "{out}");
    assert!(!out.contains("lists none"), "{out}");
}
