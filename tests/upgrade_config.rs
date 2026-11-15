//! Tests of the new `spec-dna.toml` of `upgrade` (step 5, slice 5). The golden
//! pairs of the plan are in `tests/golden/upgrade/config/`, and a unit test of
//! `src/upgrade_config.rs` checks that the plan gives them. This file checks
//! the property that the plan promises: the 0.6 binary loads the new file. The
//! documents of a fixture still have their 0.5 types, because the plan of the
//! documents is another slice, so `check --all` finds errors in the documents,
//! but it finds no error in the configuration.

mod common;

use common::*;
use std::fs;

/// A case: the golden name, the fixture and the overlays of its repository.
const CASES: &[(&str, &str, &[&str])] = &[
    ("v04", "v04", &[]),
    ("v05", "v05", &[]),
    ("brownfield", "brownfield", &[]),
    ("harness-lean", "v05", &["harness-lean"]),
    ("harness-full", "v05", &["harness-full"]),
];

fn golden(name: &str, side: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/upgrade/config")
        .join(format!("{name}.{side}.toml"));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The golden file pins the version 0.6.0. The test pins the version of the
/// binary, so that a later release does not change the golden files.
fn pinned(text: &str) -> String {
    assert!(text.starts_with("spec-dna = \"0.6.0\"\n"), "{text}");
    text.replacen("0.6.0", VERSION, 1)
}

#[test]
fn the_binary_loads_the_new_configuration_of_each_fixture() {
    for (name, base, overlays) in CASES {
        let repo = Repo::from_fixture(base, overlays);
        repo.write("spec-dna.toml", &pinned(&golden(name, "after")));
        let (code, out, err) = repo.run(&["check", "--all"]);
        // The documents have 0.5 types, so the exit code is 1. It is never 3,
        // the code of an error in the configuration or the pin.
        assert_eq!(code, 1, "{name}: {out}{err}");
        assert!(!err.contains("spec-dna.toml"), "{name}: {err}");
        // A command that reads the layers and the policy loads it too.
        let (code, out, err) = repo.run(&["layer", "list"]);
        assert_eq!(code, 0, "{name}: {out}{err}");
        assert!(out.contains("requirements"), "{name}: {out}");
    }
}

#[test]
fn the_old_configuration_of_each_fixture_stops_the_binary() {
    // The control: with the file before the plan, the exit code is 3.
    for (name, base, overlays) in CASES {
        let repo = Repo::from_fixture(base, overlays);
        repo.write("spec-dna.toml", &golden(name, "before"));
        let (code, _, err) = repo.run(&["check", "--all"]);
        assert_eq!(code, 3, "{name}: {err}");
        assert!(err.contains("spec-dna upgrade"), "{name}: {err}");
    }
}

#[test]
fn the_new_configuration_keeps_the_roots_of_the_old_one() {
    for (name, _, _) in CASES {
        let paths = |text: &str| -> Vec<String> {
            text.lines()
                .filter_map(|l| l.strip_prefix("path = "))
                .map(str::to_string)
                .collect()
        };
        assert_eq!(
            paths(&golden(name, "before")),
            paths(&golden(name, "after")),
            "{name}"
        );
    }
}
