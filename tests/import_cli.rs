//! End-to-end tests of `spec-dna import` (design 0.7.0, D21 to D27). Each
//! case uses real git and a fixture project of `tests/common/import.rs`.
//! `import` reads `HEAD`, so a case that runs it twice commits between the
//! runs.

mod common;

use common::import as fx;

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
