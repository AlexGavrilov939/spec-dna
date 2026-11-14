//! Integration tests of the layer graph keys (`[layers.<name>]`, `areas`,
//! `[groups]`). A bad value is a configuration error: exit code 3, and the
//! message names the key. The helpers are small copies of the ones in
//! tests/cli.rs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_spec-dna");
const VERSION: &str = env!("CARGO_PKG_VERSION");
static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct Repo {
    dir: PathBuf,
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
        ])
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

impl Repo {
    /// A repository with one root, `docs`, one document and the given lines
    /// after the roots table. `head` is the text before the roots table.
    fn with(head: &str, tail: &str) -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("spec-dna-layers-test-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("docs")).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        fs::write(
            dir.join("spec-dna.toml"),
            format!("spec-dna = \"{VERSION}\"\n{head}\n[[roots]]\npath = \"docs\"\n\n{tail}"),
        )
        .unwrap();
        fs::write(
            dir.join("docs/hose-leaks.md"),
            "---\ntype: incident\n---\n\n# Hose leaks\n\n## Summary\n\nA hose leaked.\n\n## Root cause\n\nA worn seal.\n",
        )
        .unwrap();
        git(&dir, &["add", "-A"]);
        git(&dir, &["commit", "-q", "-m", "start"]);
        Repo { dir }
    }

    fn run(&self, args: &[&str]) -> (i32, String, String) {
        let out = Command::new(BIN)
            .args(args)
            .current_dir(&self.dir)
            .output()
            .expect("binary runs");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }
}

/// Runs `check --all` on a configuration, and asserts the exit code 3 and the
/// words in the message.
fn rejects(head: &str, tail: &str, needles: &[&str]) {
    let r = Repo::with(head, tail);
    let (code, _, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 3, "{head}{tail}\n{err}");
    for n in needles {
        assert!(
            err.contains(n),
            "`{n}` is not in the message of\n{head}{tail}\n{err}"
        );
    }
}

#[test]
fn a_cycle_of_moves_to_is_an_error() {
    rejects(
        "",
        "[layers.canals]\nmoves-to = [\"pumps\"]\n[layers.pumps]\nmoves-to = [\"valves\"]\n[layers.valves]\nmoves-to = [\"canals\"]\n",
        &["moves-to", "cycle", "canals -> pumps -> valves -> canals"],
    );
}

#[test]
fn a_type_in_2_layers_is_an_error() {
    rejects(
        "",
        "[layers.one]\ntypes = [\"rule\"]\n[layers.two]\ntypes = [\"decision\", \"rule\"]\n",
        &["layers.two.types", "`rule`", "`one`"],
    );
}

#[test]
fn an_absent_layer_in_an_edge_is_an_error() {
    rejects(
        "",
        "[layers.one]\nmoves-to = [\"nowhere\"]\n",
        &["layers.one.moves-to", "`nowhere`"],
    );
    rejects(
        "",
        "[layers.one]\nlinked-from = [\"nowhere\"]\n",
        &["layers.one.linked-from", "`nowhere`"],
    );
}

#[test]
fn files_with_dir_is_an_error() {
    rejects(
        "",
        "[layers.top]\nfiles = [\"terms.md\"]\ndir = \"top\"\n",
        &["layers.top", "files", "dir"],
    );
}

#[test]
fn two_layers_with_the_same_dir_are_an_error() {
    rejects(
        "",
        "[layers.one]\ndir = \"shared\"\n[layers.two]\ndir = \"shared\"\n",
        &["layers.two.dir", "`shared`", "`one`"],
    );
}

#[test]
fn an_unknown_key_in_a_layer_is_an_error() {
    rejects(
        "",
        "[layers.one]\nmoves_to = [\"two\"]\n",
        &["unknown key", "layers.one.moves_to"],
    );
}

#[test]
fn a_bad_enum_value_is_an_error() {
    rejects(
        "",
        "[layers.one]\nkeep = \"always\"\n",
        &["layers.one.keep", "forever", "until-finished"],
    );
    rejects(
        "",
        "[layers.one]\ncontext = \"never\"\n",
        &["layers.one.context", "on-request"],
    );
}

#[test]
fn a_bad_type_in_a_layer_is_an_error() {
    rejects(
        "",
        "[layers.one]\ntypes = [\"guide\"]\n",
        &["layers.one.types", "`procedure`"],
    );
}

#[test]
fn a_group_area_that_is_not_in_areas_is_an_error() {
    rejects(
        "areas = [\"valves\"]",
        "[groups]\noperations = [\"valves\", \"sensors\"]\n",
        &["groups.operations", "`sensors`", "areas"],
    );
}

#[test]
fn a_key_of_a_later_step_is_still_unknown() {
    rejects("language = \"en\"", "", &["unknown key", "language"]);
}

#[test]
fn layers_areas_and_groups_do_not_change_check() {
    let plain = Repo::with("", "");
    let layered = Repo::with(
        "areas = [\"valves\", \"sensors\"]",
        "[layers.canals]\ntypes = [\"incident\"]\nmoves-to = [\"pumps\"]\nkeep = \"forever\"\ncontext = \"on-request\"\nmax-docs = 5\n[layers.pumps]\ntypes = [\"rule\"]\ndir = \"pumps\"\n\n[groups]\noperations = [\"valves\"]\n",
    );
    let a = plain.run(&["check", "--all"]);
    let b = layered.run(&["check", "--all"]);
    assert_eq!(a.0, 0, "{}", a.2);
    assert_eq!(a, b);
}
