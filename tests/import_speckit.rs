//! Tests of `spec-dna import speckit` (design 0.7.0, D22 and D23, and the
//! decisions I7 to I12 of the plan). The fixture project has a constitution,
//! 2 done features and 1 open feature (`tests/common/import.rs`).

mod common;

use common::import as fx;
use common::*;
use std::fs;

fn import(repo: &Repo, args: &[&str]) -> (i32, String, String) {
    let mut a = vec!["import", "speckit"];
    a.extend_from_slice(args);
    repo.run(&a)
}

fn status(repo: &Repo) -> String {
    repo.git_out(&["status", "--porcelain"])
}

/// The project after a write of `import`.
fn imported() -> Repo {
    let repo = fx::speckit();
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    repo
}

fn lines_with<'a>(text: &'a str, part: &str) -> Vec<&'a str> {
    text.lines().filter(|l| l.contains(part)).collect()
}

// --- the constitution ---

#[test]
fn the_constitution_becomes_a_rule_document_with_a_requirement_for_each_principle() {
    let repo = imported();
    let t = repo.read("docs/rules/constitution.md");
    assert!(t.starts_with("---\nstatus: active\n---\n"), "{t}");
    assert!(t.contains("# Albums Constitution"), "{t}");
    assert!(t.contains("\n## Rules\n"), "{t}");
    assert!(!t.contains("Core Principles"), "{t}");
    assert!(
        t.contains("- **library-first**: Every feature starts as a small library. A library MUST work on its own and MUST have one purpose."),
        "{t}"
    );
    // The second paragraph and the list stay with their principle.
    assert!(
        t.contains("- **test-first**: Tests come before the code."),
        "{t}"
    );
    assert!(
        t.contains("\n  The red-green-refactor cycle is strict.\n"),
        "{t}"
    );
    assert!(t.contains("\n  - Write the test.\n"), "{t}");
    // The other sections stay as text.
    assert!(t.contains("\n## Governance\n"), "{t}");
    assert!(t.contains("**Version**: 1.0.0"), "{t}");
    assert!(!repo.dir.join(".specify/memory/constitution.md").exists());
    let (code, out, err) = repo.run(&["show", "rules/constitution#library-first"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("MUST work on its own"), "{out}");
}

#[test]
fn a_constitution_with_a_placeholder_is_skipped_with_its_reason() {
    let repo = fx::speckit();
    repo.write(
        ".specify/memory/constitution.md",
        fx::SK_CONSTITUTION_TEMPLATE,
    );
    repo.commit("template");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("skipped .specify/memory/constitution.md:"),
        "{out}"
    );
    assert!(out.contains("[PROJECT_NAME]"), "{out}");
    assert!(!repo.dir.join("docs/rules").exists());
    assert!(repo.dir.join(".specify/memory/constitution.md").exists());
}

#[test]
fn a_constitution_with_no_core_principles_heading_is_skipped_and_stops_nothing() {
    let repo = fx::speckit();
    repo.write(
        ".specify/memory/constitution.md",
        "# Albums Constitution\n\n## Values\n\nBe kind.\n",
    );
    repo.commit("other headings");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("skipped .specify/memory/constitution.md: no `Core Principles` heading"),
        "{out}"
    );
}

#[test]
fn two_principles_with_one_key_stop_the_proof_with_sd204() {
    let repo = fx::speckit();
    repo.write(
        ".specify/memory/constitution.md",
        "# C\n\n## Core Principles\n\n### I. Simple\n\nKeep it small.\n\n### II. Simple\n\nKeep it very small.\n",
    );
    repo.commit("a key twice");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("SD204"), "{err}");
    assert_eq!(status(&repo), "");
}

// --- the done features ---

#[test]
fn a_done_feature_becomes_a_requirements_document_with_its_requirements_and_an_alias() {
    let repo = imported();
    let t = repo.read("docs/requirements/photo-albums.md");
    assert!(
        t.starts_with("---\nstatus: active\naliases:\n  - 001-photo-albums\n---\n"),
        "{t}"
    );
    assert!(
        t.contains("- **FR-001**: The system MUST let a person create an album with a title."),
        "{t}"
    );
    assert!(
        t.contains("- **FR-003**: The system SHOULD keep the order"),
        "{t}"
    );
    // The link to a file that stays goes to the same file.
    assert!(
        t.contains("[plan](../../specs/001-photo-albums/plan.md)"),
        "{t}"
    );
    for key in ["FR-001", "FR-002", "FR-003"] {
        let id = format!("requirements/photo-albums#{key}");
        let (code, out, err) = repo.run(&["show", &id]);
        assert_eq!(code, 0, "{id}: {out}{err}");
    }
    // The old id is an alias.
    let (code, out, err) = repo.run(&["show", "001-photo-albums#FR-002"]);
    assert_eq!(code, 0, "{out}{err}");
    let t2 = repo.read("docs/requirements/album-sharing.md");
    assert!(t2.contains("  - 002-album-sharing\n"), "{t2}");
    assert!(!repo.dir.join("specs/001-photo-albums/spec.md").exists());
}

#[test]
fn a_document_that_links_to_a_moved_file_gets_the_new_link() {
    let repo = imported();
    let t = repo.read("docs/requirements/core.md");
    assert!(t.contains("[its own spec](photo-albums.md)"), "{t}");
    assert!(!t.contains("specs/001"), "{t}");
}

#[test]
fn the_tasks_of_a_done_feature_and_its_extra_files_stay_in_place() {
    let repo = fx::speckit();
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    for kept in [
        "specs/001-photo-albums/tasks.md: the tasks of a done feature",
        "specs/001-photo-albums/plan.md:",
        "specs/001-photo-albums/research.md:",
        "specs/001-photo-albums/contracts/albums.yaml:",
        "specs/003-album-export/data-model.md:",
        ".specify/init-options.json: a file of the tool",
    ] {
        assert!(
            out.contains(&format!("kept {kept}")),
            "kept {kept} not in:\n{out}"
        );
    }
    for path in [
        "specs/001-photo-albums/tasks.md",
        "specs/001-photo-albums/plan.md",
        "specs/001-photo-albums/contracts/albums.yaml",
        ".specify/init-options.json",
    ] {
        assert!(repo.dir.join(path).exists(), "{path}");
    }
}

#[test]
fn a_feature_with_no_task_line_is_open() {
    let repo = fx::speckit();
    repo.write(
        "specs/002-album-sharing/tasks.md",
        "# Tasks\n\nTo be written.\n",
    );
    repo.commit("no tasks yet");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        repo.dir
            .join("docs/intent/album-sharing/intent.md")
            .exists()
    );
    assert!(!repo.dir.join("docs/requirements/album-sharing.md").exists());
}

#[test]
fn a_feature_with_no_tasks_file_is_open_and_another_folder_is_skipped() {
    let repo = fx::speckit();
    repo.git(&["rm", "-q", "specs/002-album-sharing/tasks.md"]);
    repo.write("specs/notes/ideas.md", "# Ideas\n");
    repo.commit("changes");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(repo.dir.join("docs/intent/album-sharing/spec.md").exists());
    assert!(
        out.contains("skipped specs/notes/: not a feature folder (`<digits>-<name>`)"),
        "{out}"
    );
}

#[test]
fn two_features_with_one_target_stop_the_plan_and_name_both() {
    let repo = fx::speckit();
    repo.write("specs/004-photo-albums/spec.md", fx::SK_SPEC_2);
    repo.write("specs/004-photo-albums/tasks.md", fx::SK_TASKS_2);
    repo.commit("a second photo-albums");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("specs/001-photo-albums"), "{err}");
    assert!(err.contains("specs/004-photo-albums"), "{err}");
    assert!(err.contains("docs/requirements/photo-albums.md"), "{err}");
    assert_eq!(status(&repo), "");
}

#[test]
fn an_area_in_the_file_of_areas_places_the_done_feature_and_the_delta_lines() {
    let repo = fx::speckit();
    let areas = repo.dir.join("..").join(format!(
        "area-{}.toml",
        repo.dir.file_name().unwrap().to_string_lossy()
    ));
    fs::write(
        &areas,
        "[areas]\n\"001-photo-albums\" = \"media\"\n\"003-album-export\" = \"media\"\n",
    )
    .unwrap();
    let a = areas.to_str().unwrap();
    let (code, out, err) = import(&repo, &["--areas", a]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(status(&repo), "M  spec-dna.toml");
    repo.commit("config");
    let (code, out, err) = import(&repo, &["--areas", a]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        repo.dir
            .join("docs/requirements/media/photo-albums.md")
            .exists()
    );
    assert!(repo.dir.join("docs/requirements/album-sharing.md").exists());
    let spec = repo.read("docs/intent/album-export/spec.md");
    assert!(
        spec.contains("ADD requirements/media/album-export#fr-001:"),
        "{spec}"
    );
    fs::remove_file(&areas).unwrap();
}

// --- the open feature ---

#[test]
fn an_open_feature_becomes_a_draft_intent_spec_and_plan() {
    let repo = imported();
    let intent = repo.read("docs/intent/album-export/intent.md");
    assert!(
        intent.starts_with("---\nstatus: draft\nclass: higher-risk\n---\n# Album export\n"),
        "{intent}"
    );
    assert!(intent.contains("\n## Intent\n"), "{intent}");
    assert!(
        intent.contains("### User Story 1 - Export an album (Priority: P1)"),
        "{intent}"
    );
    assert!(
        intent.contains("A person downloads an album as one archive."),
        "{intent}"
    );
    assert!(intent.contains("\n### Success Criteria\n"), "{intent}");
    assert!(
        intent.contains("- **SC-001**: A person exports an album of 100 photos"),
        "{intent}"
    );
    // The requirements are in the delta, not in the intent.
    assert!(!intent.contains("FR-001"), "{intent}");

    let spec = repo.read("docs/intent/album-export/spec.md");
    assert!(spec.starts_with("---\nstatus: draft\n---\n"), "{spec}");
    assert!(spec.contains("\n## Delta\n"), "{spec}");
    assert!(
        spec.contains("- ADD requirements/album-export#fr-001: The system MUST export an album as one archive."),
        "{spec}"
    );
    // The marker is not in the sentence. It is an open question.
    assert!(
        spec.contains("- ADD requirements/album-export#fr-002: The system MUST offer as formats."),
        "{spec}"
    );
    assert!(spec.contains("- [ ] fr-002: which file formats?"), "{spec}");
    assert!(spec.contains("\n## Concerns\n\n- none.\n"), "{spec}");

    let plan = repo.read("docs/intent/album-export/plan.md");
    assert!(plan.starts_with("---\nstatus: draft\n---\n"), "{plan}");
    assert!(plan.contains("\n## Files\n"), "{plan}");
    assert!(plan.contains("- `src/export/__init__.py`"), "{plan}");
    assert!(plan.contains("- `src/export/zip.py`"), "{plan}");
    assert!(
        plan.contains("- [x] t001: Create the export module. Files: `src/export/__init__.py`."),
        "{plan}"
    );
    assert!(
        plan.contains("- [ ] t002: Write the archive builder. Files: `src/export/zip.py`."),
        "{plan}"
    );
    // The marks [P] and [US1] go. A task with no path has no label.
    assert!(!plan.contains("[P]") && !plan.contains("[US1]"), "{plan}");
    assert!(
        lines_with(&plan, "t003")
            .iter()
            .all(|l| !l.contains("Files:")),
        "{plan}"
    );
    // The sources stay.
    assert!(repo.dir.join("specs/003-album-export/spec.md").exists());
    assert!(repo.dir.join("specs/003-album-export/tasks.md").exists());
}

#[test]
fn check_reports_the_missing_check_of_each_task_only_after_the_approval() {
    let repo = imported();
    repo.commit("import");
    let (code, out, err) = repo.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("SD923"), "{out}");
}

// --- the proof ---

#[test]
fn an_sc_line_with_a_bcp_14_word_stops_the_proof_and_names_the_line_and_the_fixes() {
    let repo = fx::speckit();
    let spec = fx::SK_SPEC_1.replace(
        "- **SC-001**: A person creates an album in less than 2 minutes.",
        "- **SC-001**: A person MUST create an album in less than 2 minutes.",
    );
    repo.write("specs/001-photo-albums/spec.md", &spec);
    repo.commit("an SC line with MUST");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("SD203"), "{err}");
    assert!(err.contains("SC-001"), "{err}");
    assert!(err.contains("legacy-keys"), "{err}");
    assert!(err.contains("remove the bold key"), "{err}");
    // The write stopped: nothing changed.
    assert_eq!(status(&repo), "");
    assert!(repo.dir.join("specs/001-photo-albums/spec.md").exists());
    assert!(
        !repo
            .dir
            .join("docs/requirements")
            .join("photo-albums.md")
            .exists()
    );
}

#[test]
fn an_entity_line_with_a_bcp_14_word_stops_the_proof_too() {
    let repo = fx::speckit();
    let spec = fx::SK_SPEC_1.replace(
        "- **Album**: A named group of photos.",
        "- **Album**: A named group of photos. It MUST have a title.",
    );
    repo.write("specs/001-photo-albums/spec.md", &spec);
    repo.commit("an entity line with MUST");
    let (code, out, err) = import(&repo, &["--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("SD203"), "{err}");
    assert!(err.contains("Album"), "{err}");
    // A failed proof prints the object, too.
    assert!(out.contains("\"proof\":{\"ok\":false"), "{out}");
    assert_eq!(status(&repo), "");
}

#[test]
fn a_link_that_is_broken_in_the_source_stops_the_proof_and_names_the_file_and_the_line() {
    let repo = fx::speckit();
    let spec = fx::SK_SPEC_1.replace("(./plan.md)", "(./missing.md)");
    repo.write("specs/001-photo-albums/spec.md", &spec);
    repo.commit("a broken link");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("SD102"), "{err}");
    assert!(err.contains("missing.md"), "{err}");
    assert_eq!(status(&repo), "");
}

#[test]
fn the_proof_line_counts_the_requirements_and_the_ids() {
    let repo = fx::speckit();
    let (code, out, err) = import(&repo, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    // 2 principles + 3 + 2 requirements of done features + 2 delta lines.
    assert!(
        out.contains("proof: ok (9 requirements, 0 scenarios"),
        "{out}"
    );
}

// --- the similarity of a moved file ---

#[test]
fn git_still_sees_a_rename_for_each_moved_file() {
    let repo = imported();
    repo.git(&["add", "-A"]);
    let names = repo.git_out(&["diff", "--cached", "-M", "--name-status"]);
    for (from, to) in [
        (
            ".specify/memory/constitution.md",
            "docs/rules/constitution.md",
        ),
        (
            "specs/001-photo-albums/spec.md",
            "docs/requirements/photo-albums.md",
        ),
    ] {
        let line = format!("{from}\t{to}");
        assert!(
            names
                .lines()
                .any(|l| l.starts_with('R') && l.ends_with(&line)),
            "no rename for {from} in:\n{names}"
        );
    }
}

#[test]
fn a_link_of_a_written_source_to_a_moved_file_goes_to_the_target_and_a_second_run_is_stable() {
    let repo = imported();
    let t = repo.read("docs/intent/album-export/intent.md");
    assert!(
        t.contains("[the albums](../../requirements/photo-albums.md)"),
        "{t}"
    );
    repo.commit("import");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out.trim(), "spec-dna import: nothing to change");
}

// --- the checker: text that must not vanish ---

#[test]
fn a_source_file_that_is_not_utf_8_stops_the_plan_and_names_the_file() {
    let repo = fx::speckit();
    let spec = fx::SK_SPEC_1.replace(
        "create an album with a title",
        "create a calf\u{e9} with a title",
    );
    // The byte `0xE9` alone is Latin-1, no valid UTF-8.
    let mut bytes = spec.into_bytes();
    let at = bytes.windows(2).position(|w| w == [0xc3, 0xa9]).unwrap();
    bytes.splice(at..at + 2, [0xe9]);
    fs::write(repo.dir.join("specs/001-photo-albums/spec.md"), bytes).unwrap();
    repo.commit("a Latin-1 byte");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("specs/001-photo-albums/spec.md"), "{err}");
    assert!(err.contains("not valid UTF-8"), "{err}");
    assert_eq!(status(&repo), "");
}

#[test]
fn a_requirement_line_with_no_colon_is_a_delta_line_too() {
    let repo = fx::speckit();
    let spec = fx::SK_SPEC_3.replace(
        "- **FR-002**:",
        "- **FR-003** The system MUST log each export.\n- **FR-002**:",
    );
    repo.write("specs/003-album-export/spec.md", &spec);
    repo.commit("an FR line with no colon");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let t = repo.read("docs/intent/album-export/spec.md");
    assert!(
        t.contains("- ADD requirements/album-export#fr-003: The system MUST log each export.\n"),
        "{t}"
    );
}

#[test]
fn the_lines_of_tasks_md_that_are_no_task_stay_in_the_plan() {
    let repo = fx::speckit();
    let tasks = format!(
        "{}\n## Dependencies\n\n- Phase 2 needs phase 1.\n\nPrerequisites: a running database.\n\nCheckpoint: the export MUST pass before the release.\n",
        fx::SK_TASKS_3
    );
    repo.write("specs/003-album-export/tasks.md", &tasks);
    repo.commit("more text in tasks.md");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let plan = repo.read("docs/intent/album-export/plan.md");
    for part in [
        "- Phase 2 needs phase 1.",
        "Prerequisites: a running database.",
        "Checkpoint: the export MUST pass before the release.",
    ] {
        assert!(plan.contains(part), "`{part}` not in:\n{plan}");
    }
}

#[test]
fn the_report_names_the_files_that_it_wrote_and_never_a_folder_with_a_kept_file() {
    let repo = fx::speckit();
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    // The folder of the open feature holds `plan.md` and `data-model.md`,
    // which stay.
    let line = out
        .lines()
        .find(|l| l.starts_with("to remove the sources:"))
        .expect("a line to remove the sources");
    assert_eq!(
        line,
        "to remove the sources: git rm -- specs/003-album-export/spec.md specs/003-album-export/tasks.md"
    );
}

#[test]
fn a_crlf_source_keeps_one_line_ending_in_the_front_matter() {
    let repo = fx::speckit();
    repo.write(
        "specs/002-album-sharing/spec.md",
        &fx::SK_SPEC_2.replace('\n', "\r\n"),
    );
    repo.commit("crlf");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let bytes = fs::read(repo.dir.join("docs/requirements/album-sharing.md")).unwrap();
    let t = String::from_utf8(bytes).unwrap();
    assert!(
        t.starts_with("---\r\nstatus: active\r\naliases:\r\n  - 002-album-sharing\r\n---\r\n"),
        "{t:?}"
    );
    assert!(!t.replace("\r\n", "").contains('\n'), "a lone LF in {t:?}");
}

#[test]
fn a_run_after_the_sources_are_gone_changes_nothing() {
    let repo = imported();
    repo.commit("import");
    repo.git(&["rm", "-rq", "specs"]);
    repo.commit("remove the specs");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out.trim(), "spec-dna import: nothing to change");
}

// --- the second check ---

#[test]
fn the_text_in_parentheses_of_a_task_stays_and_a_held_line_is_no_dropped_line() {
    let repo = fx::speckit();
    let tasks = "# Tasks\n\nRun the tasks in order.\n\n- [ ] T002 [P] [US1] Write the archive builder in src/export/zip.py (each archive MUST be signed)\n- [ ] T003 Check that each archive is signed in tests/test_sign.py\n- [ ] T004 Document why each export MUST be logged\n- [ ] T005 The export MUST stream the photos in src/export/stream.py\n";
    repo.write("specs/003-album-export/tasks.md", tasks);
    repo.commit("tasks");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let plan = repo.read("docs/intent/album-export/plan.md");
    for part in [
        "Write the archive builder (each archive MUST be signed). Files: `src/export/zip.py`.",
        "Document why each export MUST be logged.",
        "The export MUST stream the photos. Files: `src/export/stream.py`.",
    ] {
        assert!(plan.contains(part), "`{part}` not in:\n{plan}");
    }
    assert!(!out.contains("dropped"), "{out}");
}

#[test]
fn a_held_line_with_a_rewritten_link_is_no_loss() {
    let repo = fx::speckit();
    let spec = format!(
        "{}\nThe album page MUST follow [the plan](./plan.md).\n",
        fx::SK_SPEC_1
    );
    repo.write("specs/001-photo-albums/spec.md", &spec);
    repo.commit("a line with a link");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("dropped"), "{out}");
}

#[test]
fn the_text_before_the_title_of_an_open_spec_stays_in_the_intent() {
    let repo = fx::speckit();
    let spec = format!(
        "Review note: each export MUST be logged.\n\n{}",
        fx::SK_SPEC_3
    );
    repo.write("specs/003-album-export/spec.md", &spec);
    repo.commit("a note before the title");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let t = repo.read("docs/intent/album-export/intent.md");
    assert!(
        t.contains("Review note: each export MUST be logged."),
        "{t}"
    );
}

#[test]
fn a_folder_that_differs_only_in_case_from_a_tracked_folder_stops_the_plan() {
    let repo = fx::speckit();
    repo.write(
        "docs/Intent/x/intent.md",
        "---\nstatus: draft\n---\n# X\n\n## Intent\n\nx\n",
    );
    repo.commit("a folder in another case");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        err.contains("docs/Intent") && err.contains("docs/intent"),
        "{err}"
    );
    assert!(err.contains("only in case"), "{err}");
    assert_eq!(status(&repo), "");
    assert!(!repo.dir.join("docs/rules").exists());
}

#[test]
fn a_plain_git_commit_after_the_import_holds_every_edit() {
    let repo = fx::speckit();
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    // No `git add`: the import staged what it wrote.
    repo.git(&["commit", "-q", "-m", "plain commit"]);
    assert_eq!(status(&repo), "");
    let c = repo.git_out(&["show", "HEAD:docs/rules/constitution.md"]);
    assert!(
        c.contains("## Rules") && !c.contains("Core Principles"),
        "{c}"
    );
    let d = repo.git_out(&["show", "HEAD:docs/requirements/photo-albums.md"]);
    assert!(d.contains("  - 001-photo-albums"), "{d}");
}

#[test]
fn a_locked_index_gives_advice_for_a_lock_and_not_git_reset() {
    let repo = fx::speckit();
    fs::write(repo.dir.join(".git/index.lock"), "").unwrap();
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 4, "{out}{err}");
    assert!(err.contains("index.lock"), "{err}");
    assert!(!err.contains("git reset --hard"), "{err}");
    assert!(err.contains("Wait until it ends"), "{err}");
}

#[test]
fn a_held_line_is_no_dropped_line_in_the_baseline() {
    let repo = fx::speckit();
    let (code, out, err) = import(&repo, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("dropped"), "{out}");
}

// --- the third check ---

#[test]
fn a_heading_with_a_bcp_14_word_of_tasks_md_or_of_the_constitution_is_no_loss() {
    let repo = fx::speckit();
    let tasks = format!(
        "{}\n### Tests for User Story 1 (OPTIONAL - only if tests requested) \u{26a0}\u{fe0f}\n\n- [ ] T004 Write the test in tests/a.py\n",
        fx::SK_TASKS_3
    );
    repo.write("specs/003-album-export/tasks.md", &tasks);
    let constitution =
        fx::SK_CONSTITUTION.replace("### I. Library-First", "### I. Libraries MUST stay small");
    repo.write(".specify/memory/constitution.md", &constitution);
    repo.commit("headings with a BCP 14 word");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let plan = repo.read("docs/intent/album-export/plan.md");
    assert!(
        plan.contains(
            "**Tests for User Story 1 (OPTIONAL - only if tests requested) \u{26a0}\u{fe0f}**"
        ),
        "{plan}"
    );
}

#[test]
fn a_code_block_of_tasks_md_stays_as_it_is() {
    let repo = fx::speckit();
    let tasks = format!(
        "{}\n```bash\n# a comment\n## not a heading\nmake test\n```\n",
        fx::SK_TASKS_3
    );
    repo.write("specs/003-album-export/tasks.md", &tasks);
    repo.commit("a code block");
    let (code, out, err) = import(&repo, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let plan = repo.read("docs/intent/album-export/plan.md");
    assert!(
        plan.contains("```bash\n# a comment\n## not a heading\nmake test\n```\n"),
        "{plan}"
    );
}
