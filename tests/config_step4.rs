//! Integration tests of the configuration keys of step 4 of 0.6.0: the table
//! `[evals]` and the keys `skills`, `review-md` and `protect-tests` of
//! `[harness]`. A bad value is a configuration error: exit code 3, and the
//! message names the key. A key that the file does not have changes nothing.
//! The tests use the helpers of tests/common/mod.rs.

mod common;

use common::Repo;

const ROOTS: &str = "\n[[roots]]\npath = \"docs\"\n";

/// A repository with `head` (top keys and tables) before the roots.
fn repo(head: &str) -> Repo {
    let r = Repo::with_config(&format!("{head}{ROOTS}"));
    r.write("docs/terms.md", "# Terms\n\nA valve closes a pipe.\n");
    r.commit("start");
    r
}

/// Runs `check --all` and asserts exit code 3 and the words in the message.
fn rejects(head: &str, needles: &[&str]) {
    let (code, out, err) = repo(head).run(&["check", "--all"]);
    assert_eq!(code, 3, "{head}\n{out}{err}");
    for n in needles {
        assert!(
            err.contains(n),
            "`{n}` is not in the message of\n{head}\n{err}"
        );
    }
}

/// Runs `check --all` and asserts that the file is a valid configuration: not
/// exit code 3. A level other than `none` gives SD504 for each file that
/// `init` has not written yet.
fn parses(head: &str) {
    let (code, out, err) = repo(head).run(&["check", "--all"]);
    assert_ne!(code, 3, "{head}\n{out}{err}");
}

/// Runs `check --all` and asserts exit code 0.
fn accepts(head: &str) {
    let (code, out, err) = repo(head).run(&["check", "--all"]);
    assert_eq!(code, 0, "{head}\n{out}{err}");
}

#[test]
fn no_evals_table_is_valid() {
    accepts("");
}

#[test]
fn an_evals_table_with_each_key_is_valid() {
    accepts("[evals]\n");
    accepts("[evals]\ndir = \"evals\"\nmin-pass-rate = 0.8\nruns = 3\nmax-total-usd = 20\n");
    // The integers 0 and 1 are valid rates.
    accepts("[evals]\nmin-pass-rate = 0\n");
    accepts("[evals]\nmin-pass-rate = 1\n");
    accepts("[evals]\nmin-pass-rate = 0.0\nruns = 1\nmax-total-usd = 0.5\n");
    accepts("[evals]\nruns = 100\n");
    // A final `/` and a leading `./` drop.
    accepts("[evals]\ndir = \"./qa/evals/\"\n");
}

#[test]
fn an_evals_key_has_a_range() {
    for (cfg, want) in [
        ("min-pass-rate = 1.5", "evals.min-pass-rate"),
        ("min-pass-rate = -0.1", "evals.min-pass-rate"),
        ("min-pass-rate = 2", "evals.min-pass-rate"),
        ("min-pass-rate = nan", "evals.min-pass-rate"),
        ("min-pass-rate = \"x\"", "evals.min-pass-rate"),
        ("runs = 0", "evals.runs"),
        ("runs = 101", "evals.runs"),
        ("runs = 2.5", "evals.runs"),
        ("runs = \"3\"", "evals.runs"),
        ("max-total-usd = 0", "evals.max-total-usd"),
        ("max-total-usd = -1", "evals.max-total-usd"),
        ("max-total-usd = inf", "evals.max-total-usd"),
        ("max-total-usd = nan", "evals.max-total-usd"),
        ("max-total-usd = \"20\"", "evals.max-total-usd"),
        ("dir = 3", "evals.dir"),
    ] {
        rejects(&format!("[evals]\n{cfg}\n"), &[want]);
    }
}

#[test]
fn the_evals_dir_follows_the_rules_of_the_templates_dir() {
    for bad in [
        "\"\"",
        "\".\"",
        "\"/abs\"",
        "\"../x\"",
        "\"a/../b\"",
        "\"a/./b\"",
        "\"a//b\"",
        "\"a\\\\b\"",
        "\"c:x\"",
    ] {
        rejects(&format!("[evals]\ndir = {bad}\n"), &["`evals.dir`"]);
    }
}

#[test]
fn the_evals_dir_and_the_templates_dir_do_not_overlap() {
    for cfg in [
        // The same folder.
        "[evals]\ndir = \".spec-dna/templates\"\n",
        "[evals]\ndir = \"tpl\"\n[templates]\ndir = \"./tpl/\"\n",
        // One holds the other.
        "[evals]\ndir = \".spec-dna\"\n",
        "[evals]\ndir = \".spec-dna/templates/evals\"\n",
        "[evals]\ndir = \"a/b\"\n[templates]\ndir = \"a\"\n",
    ] {
        rejects(cfg, &["`evals.dir` and `templates.dir` overlap"]);
    }
    // A folder that only shares a name prefix is not an overlap.
    accepts("[evals]\ndir = \"tpl2\"\n[templates]\ndir = \"tpl\"\n");
    accepts("[evals]\ndir = \".spec-dna/evals\"\n");
}

#[test]
fn an_unknown_evals_key_is_an_error() {
    // `pass-at` is a key of an eval file only.
    rejects("[evals]\npass-at = 1\n", &["unknown key `evals.pass-at`"]);
    rejects("[evals]\nfolder = \"x\"\n", &["unknown key `evals.folder`"]);
    rejects("evals = \"x\"\n", &["`evals` must be a table"]);
}

#[test]
fn the_evals_folder_is_never_part_of_a_root() {
    // A root `.` holds the folder of the evals. A fixture can hold a Markdown
    // file with no front matter and a broken link. `check` does not read it.
    let r = Repo::with_config("[evals]\n\n[[roots]]\npath = \".\"\ntype = \"mixed\"\n");
    r.write("terms.md", "# Terms\n\nA valve closes a pipe.\n");
    r.write(
        "evals/fixtures/a/notes.md",
        "# Notes\n\nSee [x](../missing.md).\n",
    );
    r.commit("start");
    let (code, out, err) = r.run(&["check", "--all", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("evals/"), "{out}");
}

#[test]
fn the_harness_keys_of_step_4_have_values() {
    for (cfg, want) in [
        (
            "skills = [\"intent\", \"nope\"]",
            "`harness.skills`: unknown skill `nope`; use intent, spec, plan, review",
        ),
        (
            "skills = [\"spec\", \"spec\"]",
            "`harness.skills` lists `spec` more than once",
        ),
        ("skills = \"intent\"", "harness.skills"),
        ("skills = [1]", "harness.skills"),
        (
            "review-md = \"yes\"",
            "`harness.review-md` must be true or false",
        ),
        (
            "protect-tests = \"sometimes\"",
            "`harness.protect-tests` must be \"fix-branches\", \"always\" or \"off\", not `sometimes`",
        ),
        (
            "protect-tests = true",
            "`harness.protect-tests` must be \"fix-branches\", \"always\" or \"off\"",
        ),
    ] {
        rejects(&format!("[harness]\nlevel = \"lean\"\n{cfg}\n"), &[want]);
        // Level `none` reads the keys too, and ignores their values.
        rejects(&format!("[harness]\nlevel = \"none\"\n{cfg}\n"), &[want]);
    }
}

#[test]
fn a_harness_with_the_keys_of_step_4_is_valid() {
    for level in ["none", "lean", "full"] {
        parses(&format!("[harness]\nlevel = \"{level}\"\n"));
        parses(&format!(
            "[harness]\nlevel = \"{level}\"\nskills = [\"review\", \"intent\"]\nreview-md = true\nprotect-tests = \"always\"\n"
        ));
        parses(&format!(
            "[harness]\nlevel = \"{level}\"\nskills = []\nreview-md = false\nprotect-tests = \"off\"\n"
        ));
        parses(&format!(
            "[harness]\nlevel = \"{level}\"\nprotect-tests = \"fix-branches\"\n"
        ));
    }
}

#[test]
fn an_unknown_harness_key_is_still_an_error() {
    rejects(
        "[harness]\nlevel = \"lean\"\nprotect = \"off\"\n",
        &["unknown key `harness.protect`"],
    );
}
