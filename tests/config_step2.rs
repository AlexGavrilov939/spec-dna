//! Integration tests of the configuration keys of step 2 of 0.6.0: `language`,
//! `[lint] history-terms`, `[change] policy` and the rule groups `layers` and
//! `changes`. A bad value is a configuration error: exit code 3, and the
//! message names the key. A document with a table of terms checks the same
//! twice, with a cold and a warm cache. The helpers are small copies of the ones in tests/new_place.rs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_spec-dna");
const VERSION: &str = env!("CARGO_PKG_VERSION");
static COUNTER: AtomicUsize = AtomicUsize::new(0);

const TERMS: &str = "# Terms\n\n| Term | Meaning | Forbidden synonyms |\n| --- | --- | --- |\n| Valve | A device that stops a flow of water. | gate, shutter; door |\n";

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
    /// A repository with one mixed root `docs` and a `terms.md`. `head` is
    /// the text before the roots table, and `tail` the text after it.
    fn with(head: &str, tail: &str) -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("spec-dna-step2-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("docs")).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        fs::write(
            dir.join("spec-dna.toml"),
            format!("spec-dna = \"{VERSION}\"\n{head}\n[[roots]]\npath = \"docs\"\n\n{tail}"),
        )
        .unwrap();
        fs::write(dir.join("docs/terms.md"), TERMS).unwrap();
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

/// Runs `check --all` on a configuration, and asserts the exit code 0.
fn accepts(head: &str, tail: &str) {
    let r = Repo::with(head, tail);
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{head}{tail}\n{out}\n{err}");
}

#[test]
fn a_bad_language_is_an_error() {
    for bad in [
        "\"english\"",
        "\"en_GB\"",
        "\"\"",
        "\"e\"",
        "\"x-private\"",
        "\"en-\"",
    ] {
        rejects(
            &format!("language = {bad}\n"),
            "",
            &["`language` must be", "BCP 47"],
        );
    }
    rejects("language = 7\n", "", &["`language` must be a string"]);
}

#[test]
fn a_language_tag_is_accepted_and_does_not_change_the_result() {
    accepts("", "");
    for tag in ["en", "en-GB", "de", "de-AT", "zh-Hant-TW"] {
        accepts(&format!("language = \"{tag}\"\n"), "");
    }
}

#[test]
fn language_is_a_top_level_key_only() {
    rejects(
        "",
        "language = \"de\"\n",
        &["unknown key", "roots[0].language"],
    );
}

#[test]
fn a_bad_history_terms_list_is_an_error() {
    rejects(
        "",
        "[lint]\nhistory-terms = [\"formerly\", \"formerly\"]\n",
        &["`lint.history-terms`", "`formerly`", "more than once"],
    );
    rejects(
        "",
        "[lint]\nhistory-terms = [\" \"]\n",
        &["`lint.history-terms`", "empty phrase"],
    );
    rejects(
        "",
        "[lint]\nhistory-terms = \"formerly\"\n",
        &["`lint.history-terms`", "array of strings"],
    );
    rejects(
        "",
        "[lint]\nhistory-terms = [1]\n",
        &["`lint.history-terms`", "strings"],
    );
}

#[test]
fn history_terms_are_accepted() {
    accepts("", "[lint]\nhistory-terms = [\"formerly\", \"vormals\"]\n");
    accepts("", "[lint]\nhistory-terms = []\n");
}

#[test]
fn the_change_table_needs_a_policy() {
    rejects("", "[change]\n", &["`change.policy` is required"]);
    rejects(
        "",
        "[change]\npolicy = \"four-class\"\n",
        &["`change.policy` must be", "\"three-class\"", "`four-class`"],
    );
    rejects("", "[change]\npolicy = 3\n", &["`change.policy` must be"]);
}

#[test]
fn another_key_of_the_change_table_comes_in_a_later_step() {
    for key in [
        "gate = \"block\"",
        "high-risk-layers = [\"rules\"]",
        "high-risk-paths = []",
        "max-fanout = 8",
        "invented = true",
    ] {
        let name = key.split(' ').next().unwrap();
        rejects(
            "",
            &format!("[change]\npolicy = \"three-class\"\n{key}\n"),
            &[&format!("`change.{name}`"), "later step"],
        );
    }
}

#[test]
fn each_policy_is_accepted() {
    for policy in ["three-class", "two-class", "none"] {
        accepts("", &format!("[change]\npolicy = \"{policy}\"\n"));
    }
}

#[test]
fn the_new_rule_groups_and_codes_are_valid_in_rules() {
    for rules in [
        "[rules]\nlayers = \"off\"\n",
        "[rules]\nlayers = \"error\"\nchanges = \"warning\"\n",
        "[rules]\nSD901 = \"off\"\nSD902 = \"warning\"\nSD903 = \"off\"\nSD904 = \"error\"\n",
        // SD921 is valid in `[rules]`, also with no policy. It stays off.
        "[rules]\nSD921 = \"error\"\nchanges = \"error\"\n",
    ] {
        // `[rules]` is a top-level table, so it goes before the roots.
        accepts(rules, "");
        accepts(rules, "[change]\npolicy = \"three-class\"\n");
    }
    accepts("", "rules = { layers = \"off\", changes = \"off\" }\n");
    // `rules` in `[layers.rules]` is a layer name. It is not the `[rules]` table.
    accepts(
        "[rules]\nlayers = \"off\"\n",
        "[layers.rules]\ntypes = [\"rule\"]\n[layers.foundation]\ntypes = [\"terms\"]\nfiles = [\"terms.md\"]\n",
    );
}

#[test]
fn an_unknown_rule_group_names_the_new_groups() {
    rejects(
        "[rules]\nnope = \"off\"\n",
        "",
        &["`rules.nope`", "layers", "changes"],
    );
}

#[test]
fn a_document_with_a_table_of_terms_checks_twice_with_the_cache() {
    let r = Repo::with("", "");
    // The first run writes the index, and the second run reads it.
    for _ in 0..2 {
        let (code, out, err) = r.run(&["check", "--all", "--json"]);
        assert_eq!(code, 0, "{out}\n{err}");
    }
}
