//! Tests of `spec-dna import openspec` (design 0.7.0, D22 and D24, and the
//! decisions I13 to I15 of the plan). The fixture project has 2 domain specs,
//! 4 open changes and an archived change (`tests/common/import.rs`).

mod common;

use common::import as fx;
use common::*;

fn import(repo: &Repo, args: &[&str]) -> (i32, String, String) {
    let mut a = vec!["import", "openspec"];
    a.extend_from_slice(args);
    repo.run(&a)
}

fn status(repo: &Repo) -> String {
    repo.git_out(&["status", "--porcelain"])
}

fn imported() -> Repo {
    let repo = fx::openspec();
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    repo
}

/// The hash that `show` gives to a requirement.
fn hash_of(repo: &Repo, id: &str) -> String {
    let (code, out, err) = repo.run(&["show", id, "--json"]);
    assert_eq!(code, 0, "{id}: {out}{err}");
    let at = out.find("\"hash\":\"").expect("a hash") + "\"hash\":\"".len();
    out[at..at + 16].to_string()
}

// --- the domain specs ---

#[test]
fn a_domain_spec_becomes_a_requirements_document_with_its_scenarios_below_each_requirement() {
    let repo = imported();
    let t = repo.read("docs/requirements/auth.md");
    assert!(
        t.starts_with("---\nstatus: active\naliases:\n  - auth\n---\n"),
        "{t}"
    );
    assert!(t.contains("# Auth Specification"), "{t}");
    assert!(t.contains("\n## Purpose\n"), "{t}");
    assert!(
        t.contains("- **password-sign-in**: The system SHALL accept a valid password for a known account.\n"),
        "{t}"
    );
    assert!(t.contains("\n  - Scenario: Valid password\n"), "{t}");
    assert!(
        t.contains("\n    - **WHEN** a person sends a valid password\n"),
        "{t}"
    );
    assert!(t.contains("\n  - Scenario: Wrong password\n"), "{t}");
    // A second paragraph stays with its requirement.
    assert!(
        t.contains(
            "- **session-expiry**: The system SHALL end a session after 30 minutes of no use.\n"
        ),
        "{t}"
    );
    assert!(t.contains("\n  Long sessions are a risk.\n"), "{t}");
    assert!(!repo.dir.join("openspec/specs/auth/spec.md").exists());
    assert!(repo.dir.join("docs/requirements/billing.md").exists());
    // The old id is an alias.
    let (code, out, err) = repo.run(&["show", "auth#password-sign-in"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_proof_counts_the_scenarios_of_the_domain_specs_and_of_the_deltas() {
    let repo = fx::openspec();
    let (code, out, err) = import(&repo, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    // Domain specs: 3 + 1. Deltas that move: 2FA 2, reports 1. The reason and
    // the migration of a REMOVED block are no scenario.
    assert!(out.contains("proof: ok ("), "{out}");
    assert!(out.contains(" 7 scenarios"), "{out}");
    let (code, out, err) = import(&repo, &["--dry-run", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"scenarios\":7"), "{out}");
}

#[test]
fn a_document_that_links_to_a_moved_domain_spec_gets_the_new_link() {
    let repo = imported();
    let t = repo.read("docs/requirements/core.md");
    assert!(t.contains("[the auth spec](auth.md)"), "{t}");
}

// --- the open changes ---

#[test]
fn a_change_with_delta_specs_becomes_a_draft_intent_spec_and_plan() {
    let repo = imported();
    let intent = repo.read("docs/intent/add-two-factor/intent.md");
    assert!(
        intent.starts_with("---\nstatus: draft\nclass: higher-risk\n---\n# Add two factor\n"),
        "{intent}"
    );
    // `Why` is `Why now`. `What Changes` and `Impact` are the intent.
    assert!(
        intent.contains("\n## Why now\n\nPasswords alone are weak.\n"),
        "{intent}"
    );
    assert!(intent.contains("\n## Intent\n"), "{intent}");
    assert!(
        intent.contains("- Add a second factor at sign in."),
        "{intent}"
    );
    assert!(
        intent.contains("Mobile clients need an update."),
        "{intent}"
    );
    // Another heading is a sub-section of the intent.
    assert!(intent.contains("\n### Capabilities\n"), "{intent}");
    assert!(intent.contains("[design](plan.md)"), "{intent}");

    let plan = repo.read("docs/intent/add-two-factor/plan.md");
    assert!(plan.contains("\n## Files\n\n- none\n"), "{plan}");
    assert!(plan.contains("\n## Design\n"), "{plan}");
    assert!(plan.contains("The code is a 6 digit number."), "{plan}");
    assert!(plan.contains("\n## Order\n"), "{plan}");
    assert!(plan.contains("- [ ] t1-1: Add the code table"), "{plan}");
    assert!(plan.contains("- [x] t1-2: Add the code check"), "{plan}");
    assert!(
        plan.contains("- [ ] t2-1: Ask for the code in the app"),
        "{plan}"
    );
    // A checkbox line with no number gets `t<n>`, its place in the file.
    assert!(plan.contains("- [ ] t4: Update the help page"), "{plan}");
    // A group heading is a line of text, not a task.
    assert!(
        plan.contains("1. Backend") && !plan.contains("## 1. Backend"),
        "{plan}"
    );
    assert!(!plan.contains("t1: Backend"), "{plan}");
    // The sources stay.
    assert!(
        repo.dir
            .join("openspec/changes/add-two-factor/proposal.md")
            .exists()
    );
}

#[test]
fn the_delta_of_a_change_has_a_line_for_each_block_with_the_hash_of_the_imported_text() {
    let repo = imported();
    repo.commit("import");
    let spec = repo.read("docs/intent/add-two-factor/spec.md");
    assert!(spec.starts_with("---\nstatus: draft\n---\n"), "{spec}");
    // ADDED is ADD. A requirement of 2 paragraphs is one paragraph.
    assert!(
        spec.contains("- ADD requirements/auth#second-factor: The system SHALL ask for a code after a valid password. The code MUST come from a device of the person."),
        "{spec}"
    );
    // MODIFIED is CHANGE, with the hash of the imported current text.
    let h = hash_of(&repo, "requirements/auth#password-sign-in");
    assert!(
        spec.contains(&format!("- CHANGE requirements/auth#password-sign-in@{h}: The system SHALL accept a valid password for a known account and then ask for a code.")),
        "{spec}"
    );
    // REMOVED is REMOVE.
    let h = hash_of(&repo, "requirements/auth#session-expiry");
    assert!(
        spec.contains(&format!("- REMOVE requirements/auth#session-expiry@{h}\n")),
        "{spec}"
    );
    // The scenarios, the reason and the migration go to `Scenarios`.
    assert!(spec.contains("\n## Scenarios\n"), "{spec}");
    assert!(spec.contains("#### Scenario: Valid code"), "{spec}");
    assert!(
        spec.contains("**Reason**: The new flow ends a session by the code."),
        "{spec}"
    );
    assert!(
        spec.contains("**Migration**: Use the code expiry."),
        "{spec}"
    );
    assert!(spec.contains("\n## Open questions\n\n- none.\n"), "{spec}");
    let (code, out, err) = repo.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn an_added_block_for_a_domain_with_no_spec_gives_an_add_line_for_a_new_document() {
    let repo = imported();
    let spec = repo.read("docs/intent/add-reports/spec.md");
    assert!(
        spec.contains("- ADD requirements/reports#monthly-report: The system SHALL give a report of the invoices of one month."),
        "{spec}"
    );
    assert!(spec.contains("#### Scenario: Report of a month"), "{spec}");
}

#[test]
fn a_change_with_renamed_stays_in_place_with_a_report_line() {
    let repo = fx::openspec();
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!repo.dir.join("docs/intent/rename-plan").exists());
    assert!(
        repo.dir
            .join("openspec/changes/rename-plan/proposal.md")
            .exists()
    );
    assert!(
        out.contains(
            "kept openspec/changes/rename-plan/specs/billing/spec.md: the change has `RENAMED`"
        ),
        "{out}"
    );
}

#[test]
fn a_change_with_no_base_requirement_stays_in_place_with_a_report_line() {
    let repo = fx::openspec();
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!repo.dir.join("docs/intent/limit-tries").exists());
    assert!(
        out.contains("kept openspec/changes/limit-tries/specs/security/spec.md: no base requirement for `rate-limit`"),
        "{out}"
    );
}

#[test]
fn the_archive_and_the_tool_files_stay_and_the_report_counts_the_archive() {
    let repo = fx::openspec();
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains(
            "kept openspec/changes/archive/2026-01-10-first-sign-in/: an archived change (2 files)"
        ),
        "{out}"
    );
    assert!(
        out.contains("kept openspec/config.yaml: a file of the tool"),
        "{out}"
    );
    assert!(
        out.contains("kept openspec/project.md: a legacy file"),
        "{out}"
    );
    assert!(
        repo.dir
            .join("openspec/changes/archive/2026-01-10-first-sign-in/tasks.md")
            .exists()
    );
}

#[test]
fn keep_research_moves_the_archived_files_as_research_documents() {
    let repo = fx::openspec_with(true, false);
    let (code, out, err) = import(&repo, &["--keep", "research"]);
    assert_eq!(code, 0, "{out}{err}");
    for file in ["proposal.md", "tasks.md"] {
        let to = format!("docs/research/imported/2026-01-10-first-sign-in/{file}");
        assert!(repo.dir.join(&to).exists(), "{to}");
    }
    assert!(
        !repo
            .dir
            .join("openspec/changes/archive/2026-01-10-first-sign-in/proposal.md")
            .exists()
    );
    let (code, out, err) = repo.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn skip_open_leaves_the_open_changes_and_imports_the_specs() {
    let repo = fx::openspec();
    let (code, out, err) = import(&repo, &["--skip-open"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(repo.dir.join("docs/requirements/auth.md").exists());
    assert!(!repo.dir.join("docs/intent").exists());
    assert!(
        out.contains("skipped openspec/changes/add-two-factor/proposal.md:"),
        "{out}"
    );
    assert_eq!(
        status(&repo)
            .lines()
            .filter(|l| l.contains("openspec/changes"))
            .count(),
        0
    );
}

#[test]
fn a_delta_that_has_no_requirement_stops_the_import_with_exit_code_3() {
    let repo = fx::openspec();
    repo.write(
        "openspec/changes/add-reports/specs/reports/spec.md",
        "## ADDED Requirements\n\nNothing yet.\n",
    );
    repo.commit("an empty delta");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(
        err.contains("openspec/changes/add-reports/specs/reports/spec.md"),
        "{err}"
    );
    assert!(err.contains("no requirement"), "{err}");
}

#[test]
fn the_key_of_a_requirement_is_the_slug_of_its_name_and_a_name_with_no_letter_stops_the_import() {
    let repo = fx::openspec();
    repo.write(
        "openspec/specs/billing/spec.md",
        "# Billing\n\n## Requirements\n\n### Requirement: !!!\n\nThe system SHALL do it.\n",
    );
    repo.commit("a bad name");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("openspec/specs/billing/spec.md"), "{err}");
    assert!(err.contains("key"), "{err}");
    assert_eq!(status(&repo), "");
}

#[test]
fn the_report_says_when_the_edit_of_a_moved_domain_spec_is_too_large_for_a_git_rename() {
    // Each requirement heading becomes a list item, so a small spec can fall
    // below the similarity of one half that git needs for a rename.
    let repo = fx::openspec();
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    repo.git(&["add", "-A"]);
    let names = repo.git_out(&["diff", "--cached", "-M", "--name-status"]);
    for (from, to) in [
        ("openspec/specs/auth/spec.md", "docs/requirements/auth.md"),
        (
            "openspec/specs/billing/spec.md",
            "docs/requirements/billing.md",
        ),
    ] {
        let renamed = names
            .lines()
            .any(|l| l.starts_with('R') && l.ends_with(&format!("{from}\t{to}")));
        let said = out.contains(&format!("the edit of `{to}` is large"));
        assert!(
            renamed || said,
            "{from}: no rename and no note:\n{names}\n{out}"
        );
    }
}

#[test]
fn a_link_of_a_proposal_to_a_moved_spec_goes_to_the_target() {
    let repo = imported();
    let t = repo.read("docs/intent/add-two-factor/intent.md");
    assert!(
        t.contains("[the auth spec](../../requirements/auth.md)"),
        "{t}"
    );
    assert!(t.contains("[design](plan.md)"), "{t}");
}

// --- the checker: text that must not vanish ---

#[test]
fn a_source_file_that_is_not_utf_8_stops_the_plan_and_names_the_file() {
    let repo = fx::openspec();
    let mut bytes = fx::OS_SPEC_BILLING.as_bytes().to_vec();
    let at = bytes.windows(7).position(|w| w == b"invoice").unwrap();
    bytes.splice(at..at + 7, *b"calf\xe9 in");
    std::fs::write(repo.dir.join("openspec/specs/billing/spec.md"), bytes).unwrap();
    repo.commit("a Latin-1 byte");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("openspec/specs/billing/spec.md"), "{err}");
    assert!(err.contains("not valid UTF-8"), "{err}");
    assert_eq!(status(&repo), "");
}

#[test]
fn an_existing_file_that_differs_only_in_case_stops_the_plan_before_a_write() {
    let repo = fx::openspec();
    repo.write(
        "docs/requirements/Billing.md",
        "---\nstatus: active\n---\n# Billing\n\n- **other-rule**: The system SHALL do it.\n",
    );
    repo.commit("a document in another case");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("docs/requirements/Billing.md"), "{err}");
    assert!(err.contains("docs/requirements/billing.md"), "{err}");
    assert!(err.contains("only in case"), "{err}");
    assert_eq!(status(&repo), "");
}

#[test]
fn an_area_for_a_domain_that_holds_only_its_spec_ends_in_a_second_run_that_changes_nothing() {
    let repo = fx::openspec();
    // The delta of a change keeps the folder `specs/auth/`, so no change here.
    repo.git(&["rm", "-rq", "openspec/changes", "openspec/specs/auth"]);
    repo.commit("only the billing spec");
    let areas = repo.dir.join("..").join(format!(
        "money-{}.toml",
        repo.dir.file_name().unwrap().to_string_lossy()
    ));
    std::fs::write(&areas, "[areas]\nbilling = \"money\"\n").unwrap();
    let a = areas.to_str().unwrap();
    for _ in 0..2 {
        let (code, out, err) = import(&repo, &["--areas", a]);
        assert_eq!(code, 0, "{out}{err}");
        repo.commit("import");
    }
    assert!(repo.dir.join("docs/requirements/money/billing.md").exists());
    let (code, out, err) = import(&repo, &["--areas", a]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out.trim(), "spec-dna import: nothing to change");
    std::fs::remove_file(&areas).unwrap();
}

#[test]
fn the_text_of_a_delta_spec_outside_the_blocks_stays_as_plain_text() {
    let repo = fx::openspec();
    let delta = format!(
        "Notes of the author before the blocks.\n\n{}\n### Notes\n\nThe rollout MUST be slow.\n\n## Migration plan\n\nCopy the data first.\n",
        fx::OS_DELTA_REPORTS
    );
    repo.write("openspec/changes/add-reports/specs/reports/spec.md", &delta);
    repo.commit("more text");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let t = repo.read("docs/intent/add-reports/spec.md");
    for part in [
        "Notes of the author before the blocks.",
        "The rollout MUST be slow.",
        "Copy the data first.",
    ] {
        assert!(t.contains(part), "`{part}` not in:\n{t}");
    }
}

#[test]
fn the_lines_of_tasks_md_that_are_no_task_stay_in_the_plan() {
    let repo = fx::openspec();
    let tasks = format!(
        "{}\nCheckpoint: the report MUST be reviewed.\n",
        fx::OS_TASKS_REPORTS
    );
    repo.write("openspec/changes/add-reports/tasks.md", &tasks);
    repo.commit("more text");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let plan = repo.read("docs/intent/add-reports/plan.md");
    assert!(
        plan.contains("Checkpoint: the report MUST be reviewed."),
        "{plan}"
    );
}

#[test]
fn a_requirement_with_a_list_and_no_sentence_stops_the_plan() {
    let repo = fx::openspec();
    repo.write(
        "openspec/specs/billing/spec.md",
        "# B\n\n## Requirements\n\n### Requirement: Invoice Issue\n\n- The system SHALL issue it.\n- The system SHALL mail it.\n",
    );
    repo.commit("a list only");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("openspec/specs/billing/spec.md:5"), "{err}");
    assert!(err.contains("no sentence"), "{err}");
    assert_eq!(status(&repo), "");
}
