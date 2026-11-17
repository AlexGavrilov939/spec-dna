//! Tests of the tree binary (ADR 0013, D18 to D20). A binary is a tree binary
//! when the file `Cargo.toml` at the top level of the repository has
//! `name = "spec-dna"` and a `version` equal to the version of the binary. In
//! this mode, SD503 does not count a change of the line `spec-dna = "..."` of
//! `spec-dna.toml`. Each other rule of the pin stays.
//!
//! Reading of the rule for a pin change together with a document: the pin line
//! is not a change. Thus `spec-dna.toml` has no change, and SD503 (which needs
//! `spec-dna.toml` and another counted file) does not fire. Any other changed
//! line of `spec-dna.toml` counts, so SD503 fires with a document or with
//! `Cargo.toml`.

mod common;

use common::*;

/// The pin of the base commit. It is 0.6.0 or later, so SD503 applies, and it
/// is not the version of the binary.
const OLD: &str = "0.6.0";

fn config(pin: &str, extra: &str) -> String {
    format!("spec-dna = \"{pin}\"\n{extra}[[roots]]\npath = \"docs\"\n")
}

fn cargo(name: &str, version: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"{version}\"\nedition = \"2024\"\n")
}

/// A repository with 1 commit: the configuration that pins `OLD`, a
/// `Cargo.toml` with `name` and `cargo_version`, and 1 document.
fn base(name: &str, cargo_version: &str) -> Repo {
    let r = Repo::new();
    r.write("spec-dna.toml", &config(OLD, ""));
    r.write("Cargo.toml", &cargo(name, cargo_version));
    r.write("docs/a.md", "# A\n\nA text.\n");
    r.commit("base");
    r
}

/// The release shape: the pin moves to the version of the binary.
fn move_pin(r: &Repo, extra: &str) {
    r.write("spec-dna.toml", &config(VERSION, extra));
}

fn sd503(out: &str) -> usize {
    out.lines()
        .filter(|l| l.contains("error SD503 config-not-alone"))
        .count()
}

/// The 3 ways to read the head tree: the working tree with no base, the
/// working tree against a base, and the git index. The change is staged, so
/// the 3 ways see the same change.
fn modes(r: &Repo) -> Vec<(&'static str, Vec<&'static str>)> {
    r.git(&["add", "-A"]);
    vec![
        ("no base", vec!["check"]),
        ("base", vec!["check", "--base", "HEAD"]),
        ("staged", vec!["check", "--staged"]),
    ]
}

#[test]
fn a_pin_only_change_in_a_tree_binary_is_not_a_configuration_change() {
    // The release shape: the pin and the version of `Cargo.toml` move together.
    let r = base("spec-dna", OLD);
    move_pin(&r, "");
    r.write("Cargo.toml", &cargo("spec-dna", VERSION));
    for (name, args) in modes(&r) {
        let (code, out, err) = r.run(&args);
        assert_eq!(sd503(&out), 0, "{name}: {out}{err}");
        assert_eq!(code, 0, "{name}: {out}{err}");
    }
}

#[test]
fn a_pin_change_with_a_document_in_a_tree_binary_has_no_sd503() {
    // The pin line is not a change of `spec-dna.toml`. The document is the
    // only counted file, and SD503 needs the configuration and another file.
    let r = base("spec-dna", VERSION);
    move_pin(&r, "");
    r.write("docs/a.md", "# A\n\nA text.\n\nMore text.\n");
    for (name, args) in modes(&r) {
        let (code, out, err) = r.run(&args);
        assert_eq!(sd503(&out), 0, "{name}: {out}{err}");
        assert_eq!(code, 0, "{name}: {out}{err}");
    }
}

#[test]
fn a_pin_change_with_another_line_in_a_tree_binary_has_sd503() {
    // The other line counts, so the change has the configuration and a document.
    let r = base("spec-dna", VERSION);
    move_pin(&r, "[context]\nbudget = 9000\n");
    r.write("docs/a.md", "# A\n\nA text.\n\nMore text.\n");
    for (name, args) in modes(&r) {
        let (code, out, err) = r.run(&args);
        assert_eq!(sd503(&out), 1, "{name}: {out}{err}");
        assert!(
            out.contains("the change edits spec-dna.toml and 1 other file (first: `docs/a.md`)"),
            "{name}: {out}"
        );
        assert_eq!(code, 1, "{name}: {out}{err}");
    }
}

#[test]
fn another_line_with_a_pin_change_counts_in_the_release_shape() {
    // The pin moves with the version, and the change adds one more line to
    // `spec-dna.toml`. `Cargo.toml` is the other counted file.
    let r = base("spec-dna", OLD);
    move_pin(&r, "[context]\nbudget = 9000\n");
    r.write("Cargo.toml", &cargo("spec-dna", VERSION));
    for (name, args) in modes(&r) {
        let (code, out, err) = r.run(&args);
        assert_eq!(sd503(&out), 1, "{name}: {out}{err}");
        assert!(out.contains("(first: `Cargo.toml`)"), "{name}: {out}");
        assert_eq!(code, 1, "{name}: {out}{err}");
    }
}

#[test]
fn a_change_of_the_configuration_alone_has_no_sd503_in_a_tree_binary() {
    // The pin and another line, and no other file: a change of its own.
    let r = base("spec-dna", VERSION);
    move_pin(&r, "[context]\nbudget = 9000\n");
    for (name, args) in modes(&r) {
        let (code, out, err) = r.run(&args);
        assert_eq!(sd503(&out), 0, "{name}: {out}{err}");
        assert_eq!(code, 0, "{name}: {out}{err}");
    }
}

#[test]
fn a_release_binary_in_a_repository_with_another_package_keeps_sd503() {
    let r = base("spec-dna", OLD);
    r.write("Cargo.toml", &cargo("tool", VERSION));
    move_pin(&r, "");
    for (mode, args) in modes(&r) {
        let (code, out, err) = r.run(&args);
        assert_eq!(sd503(&out), 1, "{mode}: {out}{err}");
        assert_eq!(code, 1, "{mode}: {out}{err}");
    }
}

#[test]
fn a_release_binary_in_a_repository_with_no_cargo_toml_keeps_sd503() {
    let r = base("spec-dna", VERSION);
    r.git(&["rm", "-q", "-f", "Cargo.toml"]);
    r.write("docs/a.md", "# A\n\nA text.\n\nMore text.\n");
    move_pin(&r, "");
    for (mode, args) in modes(&r) {
        let (code, out, err) = r.run(&args);
        assert!(
            out.contains("the change edits spec-dna.toml and 2 other files"),
            "{mode}: {out}{err}"
        );
        assert_eq!(code, 1, "{mode}: {out}{err}");
    }
}

#[test]
fn a_package_with_another_version_keeps_sd503() {
    // The name is right, the version is not the version of the binary.
    let r = base("spec-dna", OLD);
    r.write("Cargo.toml", &cargo("spec-dna", "9.9.9"));
    move_pin(&r, "");
    for (mode, args) in modes(&r) {
        let (code, out, err) = r.run(&args);
        assert_eq!(sd503(&out), 1, "{mode}: {out}{err}");
        assert_eq!(code, 1, "{mode}: {out}{err}");
    }
}

#[test]
fn a_cargo_toml_in_a_folder_does_not_make_a_tree_binary() {
    // Only the file at the top level of the repository counts.
    let r = base("spec-dna", OLD);
    r.git(&["rm", "-q", "-f", "Cargo.toml"]);
    r.write("docs/Cargo.toml", &cargo("spec-dna", VERSION));
    move_pin(&r, "");
    for (mode, args) in modes(&r) {
        let (code, out, err) = r.run(&args);
        assert_eq!(sd503(&out), 1, "{mode}: {out}{err}");
        assert_eq!(code, 1, "{mode}: {out}{err}");
    }
}

#[test]
fn a_pin_that_differs_still_stops_check_finish_and_eval_run() {
    // The tree binary has the version of `Cargo.toml`, and the pin differs.
    let r = base("spec-dna", VERSION);
    for args in [
        &["check"][..],
        &["check", "--base", "HEAD"],
        &["check", "--staged"],
        &["finish", "x"],
        &["eval", "run"],
    ] {
        let (code, out, err) = r.run(args);
        assert_eq!(code, 3, "{args:?}: {out}{err}");
        assert!(
            err.contains(&format!(
                "spec-dna.toml pins version {OLD}, but this binary is {VERSION}."
            )),
            "{args:?}: {err}"
        );
    }
}

#[test]
fn an_unknown_table_still_gives_exit_code_3_in_a_tree_binary() {
    // A key does not start the mode: the 0.6.1 behaviour stays.
    let r = base("spec-dna", VERSION);
    r.write("spec-dna.toml", &config(VERSION, "[dev]\ntree = true\n"));
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("unknown key"), "{err}");
}

#[test]
fn check_staged_reads_the_manifest_and_the_configuration_of_the_index() {
    // The index has the release shape. The working tree changes after the
    // staging, and `--staged` ignores that change.
    let r = base("spec-dna", OLD);
    move_pin(&r, "");
    r.write("Cargo.toml", &cargo("spec-dna", VERSION));
    r.git(&["add", "-A"]);
    r.write("Cargo.toml", &cargo("spec-dna", "9.9.9"));
    r.write(
        "spec-dna.toml",
        &config(VERSION, "[context]\nbudget = 9000\n"),
    );
    let (code, out, err) = r.run(&["check", "--staged"]);
    assert_eq!(sd503(&out), 0, "{out}{err}");
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn pin_only_change_with_crlf_in_the_working_tree_gives_no_sd503() {
    // The base has LF line ends. The working tree has CRLF line ends, as a
    // checkout with `core.autocrlf` gives, and the change moves the pin. The
    // TOML meaning of the 2 texts differs in the pin line only.
    let r = base("spec-dna", VERSION);
    r.write("spec-dna.toml", &config(VERSION, "").replace('\n', "\r\n"));
    r.write("docs/a.md", "# A\n\nA text.\n\nMore text.\n");
    for (name, args) in modes(&r) {
        let (code, out, err) = r.run(&args);
        assert_eq!(sd503(&out), 0, "{name}: {out}{err}");
        assert_eq!(code, 0, "{name}: {out}{err}");
    }
}

#[test]
fn a_change_of_the_line_ends_alone_is_no_pin_only_change() {
    // The pin does not change. A change of the line ends alone is a change of
    // the file, so SD503 counts it with the document. This guards D-B1.
    let r = base("spec-dna", VERSION);
    move_pin(&r, "");
    r.commit("move the pin");
    r.write("spec-dna.toml", &config(VERSION, "").replace('\n', "\r\n"));
    r.write("docs/a.md", "# A\n\nA text.\n\nMore text.\n");
    for (name, args) in modes(&r) {
        let (code, out, err) = r.run(&args);
        assert_eq!(sd503(&out), 1, "{name}: {out}{err}");
        assert_eq!(code, 1, "{name}: {out}{err}");
    }
}

/// A repository whose `Cargo.toml` is a symbolic link to a real file of the
/// tree. The link text is not a manifest.
#[cfg(unix)]
fn linked_base() -> Repo {
    let r = Repo::new();
    r.write("spec-dna.toml", &config(OLD, ""));
    r.write("manifest/Cargo.toml", &cargo("spec-dna", VERSION));
    std::os::unix::fs::symlink("manifest/Cargo.toml", r.dir.join("Cargo.toml")).unwrap();
    r.write("docs/a.md", "# A\n\nA text.\n");
    r.commit("base");
    r
}

#[cfg(unix)]
#[test]
fn a_symlinked_manifest_is_no_tree_binary_in_both_modes() {
    // The working tree follows the link, and the index holds the link text.
    // A link is no manifest, so the 2 modes give one answer: SD503 counts the
    // pin line.
    let r = linked_base();
    move_pin(&r, "");
    r.write("docs/a.md", "# A\n\nA text.\n\nMore text.\n");
    for (name, args) in modes(&r) {
        let (code, out, err) = r.run(&args);
        assert_eq!(sd503(&out), 1, "{name}: {out}{err}");
        assert_eq!(code, 1, "{name}: {out}{err}");
    }
}
