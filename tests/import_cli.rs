//! End-to-end tests of `spec-dna import` (design 0.7.0, D21 to D27). Each
//! case uses real git and a fixture project of `tests/common/import.rs`.
//! `import` reads `HEAD`, so a case that runs it twice commits between the
//! runs.

mod common;

use common::import as fx;
use common::*;
use std::fs;

#[test]
fn fixtures_speckit_project_is_committed_and_clean() {
    let repo = fx::speckit();
    for (path, text) in fx::speckit_files() {
        assert_eq!(repo.read(path), text, "{path}");
    }
    assert_eq!(repo.git_out(&["status", "--porcelain"]), "");
    let (code, out, err) = repo.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn fixtures_openspec_project_is_committed_and_clean() {
    let repo = fx::openspec();
    for (path, text) in fx::openspec_files() {
        assert_eq!(repo.read(path), text, "{path}");
    }
    assert_eq!(repo.git_out(&["status", "--porcelain"]), "");
    let (code, out, err) = repo.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn fixtures_config_has_the_layers_and_the_options() {
    let plain = fx::config(false, false);
    assert!(plain.contains("[layers.requirements]"));
    assert!(!plain.contains("[layers.research]"));
    assert!(!plain.contains("legacy-keys"));
    let full = fx::config(true, true);
    assert!(full.contains("[layers.research]"));
    assert!(full.contains("legacy-keys = [\"FR-{n}\"]"));
}

// --- the command ---

fn import(repo: &Repo, args: &[&str]) -> (i32, String, String) {
    let mut a = vec!["import"];
    a.extend_from_slice(args);
    repo.run(&a)
}

fn status(repo: &Repo) -> String {
    repo.git_out(&["status", "--porcelain"])
}

#[test]
fn a_tree_that_is_not_clean_is_exit_code_1_also_with_dry_run() {
    let repo = fx::speckit();
    repo.write("notes.txt", "a new file\n");
    for args in [&["speckit"][..], &["speckit", "--dry-run"]] {
        let (code, out, err) = import(&repo, args);
        assert_eq!(code, 1, "{args:?}: {out}{err}");
        assert!(err.contains("not clean"), "{err}");
        assert!(err.contains("notes.txt"), "{err}");
    }
    assert_eq!(status(&repo), "?? notes.txt");
}

#[test]
fn a_missing_tool_folder_is_exit_code_3_and_names_the_rule_and_the_versions() {
    let repo = fx::speckit();
    repo.git(&["rm", "-rq", ".specify"]);
    repo.commit("no tool folder");
    let (code, out, err) = import(&repo, &["speckit"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains(".specify/"), "{err}");
    assert!(err.contains("Spec Kit 1.4.1"), "{err}");

    let repo = fx::openspec();
    repo.git(&["rm", "-rq", "openspec"]);
    repo.commit("no tool folder");
    let (code, out, err) = import(&repo, &["openspec"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("openspec/"), "{err}");
    assert!(err.contains("OpenSpec 2.0.0"), "{err}");
    assert_eq!(status(&repo), "");
}

#[test]
fn a_spec_file_with_no_requirement_is_exit_code_3_and_names_the_file() {
    let repo = fx::speckit();
    repo.write(
        "specs/001-photo-albums/spec.md",
        "# Feature Specification: Photo albums\n\nOnly prose.\n",
    );
    repo.commit("no requirement");
    let (code, out, err) = import(&repo, &["speckit"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("specs/001-photo-albums/spec.md"), "{err}");
    assert!(err.contains("no requirement"), "{err}");
    assert_eq!(status(&repo), "");
}

#[test]
fn a_specs_folder_with_no_domain_folder_is_another_major_version_exit_code_3() {
    let repo = fx::openspec();
    repo.git(&["rm", "-rq", "openspec/specs"]);
    repo.write(
        "openspec/specs/auth.md",
        "# Auth\n\nA spec of an old layout.\n",
    );
    repo.commit("old layout");
    let (code, out, err) = import(&repo, &["openspec"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("openspec/specs"), "{err}");
    assert!(err.contains("another major version"), "{err}");
    assert!(err.contains("OpenSpec 2.0.0"), "{err}");
}

#[test]
fn skip_open_does_not_read_an_open_feature_so_it_cannot_stop_the_import() {
    let repo = fx::speckit();
    // An open feature that is still a template.
    repo.write(
        "specs/003-album-export/spec.md",
        "# Feature Specification: [FEATURE NAME]\n\nNothing here.\n",
    );
    repo.commit("template");
    let (code, _, err) = import(&repo, &["speckit"]);
    assert_eq!(code, 3, "{err}");
    let (code, out, err) = import(&repo, &["speckit", "--skip-open"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("skipped specs/003-album-export/spec.md:"),
        "{out}"
    );
    assert!(out.contains("--skip-open"), "{out}");
    assert!(!repo.dir.join("docs/intent").exists());
}

#[test]
fn keep_research_with_no_research_layer_is_exit_code_2_and_names_layer_add() {
    let repo = fx::speckit();
    let (code, out, err) = import(&repo, &["speckit", "--keep", "research"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.contains("spec-dna layer add research --types research"),
        "{err}"
    );
    assert_eq!(status(&repo), "");
}

#[test]
fn keep_research_moves_each_markdown_file_and_leaves_the_others() {
    let repo = fx::speckit_with(true, true);
    let (code, out, err) = import(&repo, &["speckit", "--keep", "research"]);
    assert_eq!(code, 0, "{out}{err}");
    let moved = [
        "specs/001-photo-albums/research.md",
        "specs/001-photo-albums/plan.md",
        "specs/003-album-export/plan.md",
        "specs/003-album-export/data-model.md",
    ];
    for from in moved {
        let (folder, file) = from
            .strip_prefix("specs/")
            .unwrap()
            .split_once('/')
            .unwrap();
        let to = format!("docs/research/imported/{folder}/{file}");
        assert!(repo.dir.join(&to).exists(), "{to}");
        assert!(!repo.dir.join(from).exists(), "{from}");
        assert!(out.contains(&format!("moved {from} -> {to}")), "{out}");
    }
    // A file that is not Markdown stays, and the report names it.
    assert!(
        repo.dir
            .join("specs/001-photo-albums/contracts/albums.yaml")
            .exists()
    );
    assert!(
        out.contains("kept specs/001-photo-albums/contracts/albums.yaml: not a Markdown file"),
        "{out}"
    );
    // The link of a moved file still goes to the same file.
    let (code, out, err) = repo.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_first_run_writes_only_spec_dna_toml_and_the_second_run_the_documents() {
    let repo = fx::speckit_with(false, false);
    let (code, out, err) = import(&repo, &["speckit"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(status(&repo), " M spec-dna.toml");
    assert!(
        repo.read("spec-dna.toml")
            .contains("legacy-keys = [\"FR-{n}\"]")
    );
    assert!(out.contains("wrote spec-dna.toml"), "{out}");
    assert!(
        out.contains("to commit: git add -A -- spec-dna.toml"),
        "{out}"
    );
    assert!(!repo.dir.join("docs/requirements/photo-albums.md").exists());
    // The tree is not clean now, so the second run needs the commit.
    let (code, _, err) = import(&repo, &["speckit"]);
    assert_eq!(code, 1, "{err}");
    repo.commit("config");
    let (code, out, err) = import(&repo, &["speckit"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(repo.dir.join("docs/requirements/photo-albums.md").exists());
    assert!(!out.contains("wrote spec-dna.toml"), "{out}");
}

#[test]
fn the_first_run_adds_an_area_and_a_missing_type() {
    let repo = fx::speckit_with(false, true);
    let cfg = repo
        .read("spec-dna.toml")
        .replace("profile = \"standard\"\n", "profile = \"standard\"\ntypes = [\"requirements\", \"intent\", \"spec\", \"plan\", \"decision\"]\n");
    repo.write("spec-dna.toml", &cfg);
    repo.commit("types");
    let areas = repo.dir.join("..").join(format!(
        "areas-{}.toml",
        repo.dir.file_name().unwrap().to_string_lossy()
    ));
    fs::write(&areas, "[areas]\n\"001-photo-albums\" = \"media\"\n").unwrap();
    let (code, out, err) = import(&repo, &["speckit", "--areas", areas.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = repo.read("spec-dna.toml");
    assert!(cfg.contains("areas = [\"media\"]"), "{cfg}");
    assert!(cfg.contains("\"rule\""), "{cfg}");
    assert_eq!(status(&repo), " M spec-dna.toml");
    fs::remove_file(&areas).unwrap();
}

#[test]
fn a_pair_of_areas_that_differ_only_in_case_is_exit_code_1() {
    let repo = fx::speckit_with(false, true);
    let areas = repo.dir.join("..").join(format!(
        "pair-{}.toml",
        repo.dir.file_name().unwrap().to_string_lossy()
    ));
    fs::write(
        &areas,
        "[areas]\n\"001-photo-albums\" = \"Media\"\n\"002-album-sharing\" = \"media\"\n",
    )
    .unwrap();
    let (code, out, err) = import(&repo, &["speckit", "--areas", areas.to_str().unwrap()]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("`Media`") && err.contains("`media`"), "{err}");
    assert!(err.contains("only in case"), "{err}");
    assert_eq!(status(&repo), "");
    fs::remove_file(&areas).unwrap();
}

#[test]
fn a_dry_run_and_a_write_give_the_same_plan() {
    let repo = fx::speckit();
    let (code, dry, err) = import(&repo, &["speckit", "--dry-run"]);
    assert_eq!(code, 0, "{dry}{err}");
    assert_eq!(status(&repo), "");
    let (code, real, err) = import(&repo, &["speckit"]);
    assert_eq!(code, 0, "{real}{err}");
    assert!(
        dry.contains("would move specs/001-photo-albums/spec.md"),
        "{dry}"
    );
    assert!(
        dry.contains("would write docs/intent/album-export/spec.md"),
        "{dry}"
    );
    let norm = |t: &str| {
        t.replace("would move", "moved")
            .replace("would write", "wrote")
    };
    assert_eq!(norm(&dry), real);
    assert!(
        status(&repo)
            .contains("specs/001-photo-albums/spec.md -> docs/requirements/photo-albums.md")
    );
}

#[test]
fn the_text_output_has_a_line_for_each_source_and_the_commands_to_run() {
    let repo = fx::speckit();
    let (code, out, err) = import(&repo, &["speckit"]);
    assert_eq!(code, 0, "{out}{err}");
    for line in [
        "moved .specify/memory/constitution.md -> docs/rules/constitution.md",
        "moved specs/001-photo-albums/spec.md -> docs/requirements/photo-albums.md",
        "moved specs/002-album-sharing/spec.md -> docs/requirements/album-sharing.md",
        "kept specs/001-photo-albums/tasks.md: the tasks of a done feature",
        "wrote docs/intent/album-export/intent.md (from specs/003-album-export/spec.md)",
        "wrote docs/intent/album-export/spec.md (from specs/003-album-export/spec.md)",
        "wrote docs/intent/album-export/plan.md (from specs/003-album-export/tasks.md)",
        "proof: ok",
        "to remove the sources: git rm -r -- specs/003-album-export",
    ] {
        assert!(out.contains(line), "`{line}` not in:\n{out}");
    }
    assert!(out.contains("to commit: git add -A -- "), "{out}");
    assert!(out.contains("docs/requirements/photo-albums.md"), "{out}");
    assert!(!out.contains("diff --git"), "{out}");
}

#[test]
fn the_json_output_has_the_fields_of_the_spec() {
    let repo = fx::speckit();
    let (code, out, err) = import(&repo, &["speckit", "--json", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    for part in [
        "\"version\":\"",
        "\"source\":\"speckit\"",
        "\"dry_run\":true",
        "\"config_only\":false",
        "\"sources\":[",
        "\"path\":\"specs/001-photo-albums/spec.md\"",
        "\"state\":\"moved\"",
        "\"state\":\"written\"",
        "\"state\":\"kept\"",
        "\"to\":[\"docs/requirements/photo-albums.md\"]",
        "\"reason\":",
        "\"pairs\":[",
        "\"from\":\"specs/001-photo-albums/spec.md:",
        "\"id\":\"requirements/photo-albums#FR-001\"",
        "\"delta\":null",
        "\"delta\":\"album-export\"",
        "\"proof\":{\"ok\":true",
        "\"requirements\":",
        "\"scenarios\":",
        "\"ids\":",
        "\"failures\":[]",
        "\"report\":{",
        "\"errors\":[",
        "\"outside_links\":",
        "\"notes\":[",
    ] {
        assert!(out.contains(part), "`{part}` not in:\n{out}");
    }
    assert_eq!(status(&repo), "");
}

#[test]
fn the_json_of_the_first_run_says_config_only() {
    let repo = fx::speckit_with(false, false);
    let (code, out, err) = import(&repo, &["speckit", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"config_only\":true"), "{out}");
    assert!(out.contains("\"dry_run\":false"), "{out}");
}

#[test]
fn a_second_run_after_the_commit_changes_nothing() {
    let repo = fx::speckit();
    let (code, out, err) = import(&repo, &["speckit"]);
    assert_eq!(code, 0, "{out}{err}");
    repo.commit("import");
    let (code, out, err) = import(&repo, &["speckit"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out.trim(), "spec-dna import: nothing to change");
    assert_eq!(status(&repo), "");
    let (code, out, _) = import(&repo, &["speckit", "--json"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"proof\":{\"ok\":true"), "{out}");
}

#[test]
fn a_second_run_of_openspec_changes_nothing_after_the_specs_moved() {
    let repo = fx::openspec();
    let (code, out, err) = import(&repo, &["openspec"]);
    assert_eq!(code, 0, "{out}{err}");
    repo.commit("import");
    assert!(!repo.dir.join("openspec/specs").exists());
    let (code, out, err) = import(&repo, &["openspec"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out.trim(), "spec-dna import: nothing to change");
}

/// The branch of a pull request: the import is a commit on `import`, and
/// `main` is the base that `check` reads from `origin/HEAD`.
fn branch_with_import(repo: &Repo, args: &[&str]) {
    origin_head(&repo.dir, "main");
    repo.git(&["checkout", "-q", "-b", "import"]);
    let (code, out, err) = import(repo, args);
    assert_eq!(code, 0, "{out}{err}");
    repo.commit("import");
}

#[test]
fn check_passes_after_the_import_with_and_without_skip_open() {
    for args in [&["speckit"][..], &["speckit", "--skip-open"]] {
        let repo = fx::speckit();
        branch_with_import(&repo, args);
        let (code, out, err) = repo.run(&["check"]);
        assert_eq!(code, 0, "{args:?}: {out}{err}");
        let (code, out, err) = repo.run(&["check", "--all"]);
        assert_eq!(code, 0, "{args:?}: {out}{err}");
    }
}

#[test]
fn with_strict_the_report_names_sd608_sd917_and_sd921() {
    let repo = fx::speckit();
    let cfg = repo.read("spec-dna.toml").replace("standard", "strict");
    repo.write("spec-dna.toml", &cfg);
    repo.commit("strict");
    let (code, out, err) = import(&repo, &["speckit", "--skip-open", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    for code in ["SD608", "SD917", "SD921"] {
        assert!(out.contains(code), "{code} not in:\n{out}");
    }
}

#[test]
fn a_symbolic_link_in_a_target_path_stops_the_write() {
    let repo = fx::speckit();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/tmp", repo.dir.join("docs/rules")).unwrap();
        repo.commit("a link");
        let (code, out, err) = import(&repo, &["speckit"]);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("symbolic link"), "{err}");
        assert_eq!(status(&repo), "");
    }
}

#[test]
fn a_target_that_exists_with_other_text_stops_the_plan_and_names_git_rm() {
    let repo = fx::speckit();
    let (code, out, err) = import(&repo, &["speckit"]);
    assert_eq!(code, 0, "{out}{err}");
    repo.write(
        "docs/intent/album-export/spec.md",
        "---\nstatus: draft\n---\n# Edited\n\n## Delta\n\n- none.\n",
    );
    repo.commit("import and an edit");
    let (code, out, err) = import(&repo, &["speckit"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("docs/intent/album-export/spec.md"), "{err}");
    assert!(err.contains("git rm"), "{err}");
}
