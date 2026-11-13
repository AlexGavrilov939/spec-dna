//! Integration tests: they run the binary on generic fixture repositories.
//! The golden files are in tests/golden. To regenerate them, run
//! `UPDATE_GOLDEN=1 cargo test`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_spec-dna");
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Fixtures and tests pin this version. The helpers write the version of the
/// binary instead, so that a version bump does not change the tests.
const FIXTURE_PIN: &str = "spec-dna = \"0.1.0\"";

fn pinned(text: &str) -> String {
    text.replace(FIXTURE_PIN, &format!("spec-dna = \"{VERSION}\""))
}
static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct Repo {
    dir: PathBuf,
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn git(dir: &Path, args: &[&str]) -> String {
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
        .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let target = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &target);
        } else {
            fs::copy(e.path(), target).unwrap();
        }
    }
}

impl Repo {
    fn empty() -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("spec-dna-test-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        Repo { dir }
    }

    /// Makes a repository and commits the fixture to it.
    fn fixture(name: &str) -> Repo {
        let r = Repo::empty();
        copy_dir(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(name),
            &r.dir,
        );
        let cfg = r.dir.join("spec-dna.toml");
        if let Ok(t) = fs::read_to_string(&cfg) {
            fs::write(&cfg, pinned(&t)).unwrap();
        }
        r.commit("fixture");
        r
    }

    fn commit(&self, msg: &str) {
        git(&self.dir, &["add", "-A"]);
        git(&self.dir, &["commit", "-q", "--allow-empty", "-m", msg]);
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

    fn write(&self, path: &str, text: &str) {
        let p = self.dir.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, pinned(text)).unwrap();
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.dir.join(path)).unwrap()
    }

    fn append(&self, path: &str, text: &str) {
        let t = self.read(path);
        self.write(path, &format!("{t}{text}"));
    }

    fn index(&self) -> PathBuf {
        self.dir.join(".git/spec-dna/index-v1.bin")
    }
}

/// Compares the output with a golden file. The golden file has `VERSION`
/// in place of the version of the binary, and `BASE` in place of the base
/// commit (the fixture commit has the pinned version, so its id changes too).
fn golden(name: &str, actual: &str) {
    let mut actual = actual.replace(VERSION, "VERSION");
    if let Some(i) = actual.find("\"base\":\"") {
        let at = i + "\"base\":\"".len();
        if let Some(end) = actual[at..].find('"') {
            actual.replace_range(at..at + end, "BASE");
        }
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("missing golden {name}; run with UPDATE_GOLDEN=1"));
    assert_eq!(
        actual, expected,
        "golden {name} differs; run with UPDATE_GOLDEN=1 to accept"
    );
}

/// Removes the cache counters, because their values differ between modes by
/// design.
fn without_counters(json: &str) -> String {
    let mut s = json.to_string();
    for key in ["\"parsed\":", "\"cached\":"] {
        if let Some(i) = s.find(key) {
            let end = i + s[i..].find([',', '}']).unwrap();
            s.replace_range(i..end, &format!("{key}0"));
        }
    }
    s
}

#[test]
fn check_all_matches_golden() {
    let r = Repo::fixture("corpus");
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 1, "{err}");
    golden("check-all.txt", &out);
    let (code, out, _) = r.run(&["check", "--all", "--json"]);
    assert_eq!(code, 1);
    golden("check-all.json", &without_counters(&out));
}

#[test]
fn context_and_show_match_golden() {
    let r = Repo::fixture("corpus");
    let (code, out, err) = r.run(&["context", "auth/session", "--path", "src/gateway.rs"]);
    assert_eq!(code, 0, "{err}");
    golden("context-session.txt", &out);
    let (code, out, _) = r.run(&[
        "context",
        "auth/session#requirements",
        "--budget",
        "900",
        "--json",
    ]);
    assert_eq!(code, 0);
    golden("context-section.json", &out);
    let (code, out, _) = r.run(&["show", "ADR-1"]);
    assert_eq!(code, 0);
    golden("show-adr.txt", &out);
    let (code, out, _) = r.run(&["show", "auth/session#expiry", "--json"]);
    assert_eq!(code, 0);
    golden("show-requirement.json", &out);
}

#[test]
fn full_warm_and_incremental_runs_agree() {
    let r = Repo::fixture("corpus");
    let full = |r: &Repo, args: &[&str]| {
        let mut a = args.to_vec();
        a.push("--no-cache");
        without_counters(&r.run(&a).1)
    };
    for args in [&["check", "--all", "--json"][..], &["check", "--json"][..]] {
        let _ = fs::remove_file(r.index());
        let reference = full(&r, args);
        let cold = without_counters(&r.run(args).1);
        assert!(r.index().exists(), "the first cached run writes the index");
        let warm = r.run(args).1;
        assert!(
            warm.contains("\"parsed\":0"),
            "a warm run parses nothing: {warm}"
        );
        assert_eq!(cold, reference);
        assert_eq!(without_counters(&warm), reference);
    }
    // Incremental run: change documents after spec-dna wrote the index.
    r.append(
        "docs/auth/tokens.md",
        "- **rotation**: The issuer SHALL rotate keys every 90 days.\n",
    );
    r.append("docs/guides/logging.md", "\nSee [[auth/nothing]].\n");
    fs::remove_file(r.dir.join("docs/billing/invoice.md")).unwrap();
    r.write(
        "docs/billing/refunds.md",
        "# Refunds\n\n- **window**: The service SHALL accept refunds for 30 days.\n",
    );
    for args in [
        &["check", "--all", "--json"][..],
        &["check", "--json"][..],
        &["context", "auth/session", "--json"][..],
    ] {
        let incremental = without_counters(&r.run(args).1);
        assert_eq!(incremental, full(&r, args), "{args:?}");
    }
    // `check` sees the new and changed files, and it reports the deleted file.
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1);
    assert!(
        out.contains("SD101 unresolved-reference: `auth/nothing`"),
        "{out}"
    );
    assert!(out.contains("SD202 id-removed"), "{out}");
    // The index is in the git directory, so git status shows nothing to commit.
    assert!(!git(&r.dir, &["status", "--porcelain"]).contains("spec-dna/"));
}

#[test]
fn gating_is_relative_to_the_base() {
    let r = Repo::fixture("corpus");
    // Errors that exist in the base do not fail the check.
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("0 new errors and 0 new warnings against"),
        "{out}"
    );
    git(&r.dir, &["checkout", "-q", "-b", "feature"]);
    r.append(
        "docs/auth/session.md",
        "\nAlso see [the old flow](old-flow.md).\n",
    );
    r.commit("feature");
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(code, 1);
    let new: Vec<&str> = out.lines().filter(|l| l.contains(" SD")).collect();
    assert_eq!(new.len(), 1, "{out}");
    assert!(
        new[0].starts_with("docs/auth/session.md:21: error SD102 unresolved-link"),
        "{out}"
    );
    let (code, _, err) = r.run(&["check", "--base", "no-such-branch"]);
    assert_eq!(code, 3, "{err}");
}

#[test]
fn moves_keep_ids_resolving() {
    let r = Repo::fixture("corpus");
    let (code, out, err) = r.run(&["mv", "auth/session#expiry", "auth/tokens"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("auth/session#expiry -> auth/tokens#expiry"));
    assert!(
        r.read("docs/auth/tokens.md")
            .contains("401. <!-- aliases: auth/session#expiry -->")
    );
    assert!(!r.read("docs/auth/session.md").contains("**expiry**"));
    let (code, out, err) = r.run(&["mv", "auth/tokens", "docs/security/credentials.md"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("auth/tokens -> security/credentials"), "{out}");
    assert!(
        r.read("docs/security/credentials.md")
            .contains("aliases:\n  - auth/tokens\n")
    );
    // `mv` updates the relative links to the moved file.
    assert!(
        r.read("docs/auth/session.md")
            .contains("[the token rules](../security/credentials.md#requirements)")
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "a move adds no error: {out}");
    // The old forms of the id resolve to the moved requirement.
    let (_, out, _) = r.run(&["show", "auth/session#expiry"]);
    assert!(
        out.starts_with("<!-- security/credentials#expiry h:"),
        "{out}"
    );
    // A removal without an alias is an error.
    fs::remove_file(r.dir.join("docs/guides/logging.md")).unwrap();
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1);
    assert!(
        out.contains("SD202 id-removed: `guides/logging` was removed"),
        "{out}"
    );
}

#[test]
fn lifecycle_rules_compare_base_and_head() {
    let r = Repo::fixture("corpus");
    let adr = "docs/decisions/ADR-1-session-store.md";
    r.write(
        adr,
        &r.read(adr)
            .replace("Where do sessions live?", "Where should sessions live?"),
    );
    r.write(
        "docs/guides/logging.md",
        &r.read("docs/guides/logging.md")
            .replace("status: active", "status: draft"),
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1);
    assert!(out.contains("SD405 decision-frozen"), "{out}");
    assert!(
        out.contains("SD401 status-transition: status cannot move from `active` to `draft`"),
        "{out}"
    );
    git(&r.dir, &["checkout", "-q", "--", "."]);
    r.append(adr, "\n## Amendments\n\n### Cookie scope\n\nThe gateway MUST set the cookie on the parent domain.\n\n### Typo\n\nFixed a typo.\n");
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    let warnings: Vec<&str> = out.lines().filter(|l| l.contains(" SD605")).collect();
    assert_eq!(warnings.len(), 1, "{out}");
    assert!(
        warnings[0].contains(":16: warning SD605 amendment-outcome"),
        "{out}"
    );
}

#[test]
fn localized_amendments_are_not_frozen_text() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"adr\"\ntype = \"decision\"\nrequired-sections = { decision = [\"Lösung\"] }\namendment-sections = { decision = [\"Nachtrag\", \"Nachträge\"] }\n[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nstatus = \"active\"\n",
    );
    let adr = "adr/d1.md";
    r.write(
        adr,
        "---\nstatus: accepted\n---\n# Session store\n\n## Kontext\n\nWhere do sessions live?\n\n## Lösung\n\nUse the key-value store.\n\n## Folgen\n\nOne more service.\n",
    );
    r.write("docs/s.md", "# Sessions\n\nSee [[d1]].\n");
    r.commit("docs");
    // A new amendment section in the middle and one at the end: no SD405.
    let text = r.read(adr).replace(
        "## Folgen",
        "## Nachtrag 2026-01-02: TTL\n\nThe gateway MUST expire a session after 30 min.\n\n## Folgen",
    );
    r.write(
        adr,
        &format!("{text}\n## Nachträge\n\n### Typo\n\nFixed a typo.\n"),
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    let warnings: Vec<&str> = out.lines().filter(|l| l.contains(" SD605")).collect();
    assert_eq!(warnings.len(), 1, "{out}");
    assert!(warnings[0].contains("adr/d1.md:14: warning SD605"), "{out}");
    // `context` serves the amendments with the outcome of a linked decision.
    let (_, out, _) = r.run(&["context", "s"]);
    for id in [
        "d1#lösung",
        "d1#nachtrag-2026-01-02-ttl",
        "d1#nachträge",
        "d1#typo",
    ] {
        assert!(out.contains(&format!("<!-- {id} h:")), "{id}: {out}");
    }
    assert!(!out.contains("<!-- d1#folgen h:"), "{out}");
    // An edit of other text is still an edit of the accepted decision.
    r.write(
        adr,
        &r.read(adr)
            .replace("One more service.", "Two more services."),
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains(
            "SD405 decision-frozen: the text outside the amendment and live sections changed"
        ),
        "{out}"
    );
}

#[test]
fn live_sections_change_in_place_and_frozen_text_does_not() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"adr\"\ntype = \"decision\"\nrequired-sections = { decision = [\"Решение\"] }\namendment-sections = { decision = [\"Поправки\"] }\nlive-sections = { decision = [\"Статус реализации\"] }\n",
    );
    let adr = "adr/d1.md";
    r.write(
        adr,
        "---\nstatus: accepted\n---\n# Histogram\n\n## Решение\n\nShip the histogram in 2 steps.\n\n## Статус реализации\n\n- step 1: done\n- step 2: open\n\n## Поправки\n\nNone yet.\n",
    );
    r.commit("docs");
    // A live section changes in place: no SD405, no SD605.
    r.write(
        adr,
        &r.read(adr).replace(
            "- step 2: open",
            "- step 2: done (the gateway SHALL record it)",
        ),
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("SD405") && !out.contains("SD605"), "{out}");
    r.commit("status");
    // The outcome is still frozen.
    r.write(adr, &r.read(adr).replace("in 2 steps", "in 3 steps"));
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("SD405 decision-frozen"), "{out}");
    // A live section of the wrong type is a configuration error.
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"adr\"\ntype = \"decision\"\nlive-sections = { spec = [\"Status\"] }\n",
    );
    let (code, _, err) = r.run(&["check"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("live-sections.spec"), "{err}");
}

#[test]
fn conventions_come_first_and_show_prints_them() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[conventions]\ncontract = [\"docs/README.md#contract-changes\"]\ndecision = [\"rules\"]\n[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nstatus = \"active\"\n",
    );
    r.write(
        "docs/README.md",
        "---\ntype: reference\n---\n# Docs\n\n## Layout\n\nOne file per area.\n\n## Contract changes\n\nA contract change needs a dated amendment and a `verified_against` update.\n\n### Example\n\nSee the token contract.\n\n## Other\n\nNot a convention.\n",
    );
    r.write(
        "docs/rules.md",
        "---\ntype: guide\nstatus: active\n---\n# Decision rules\n\nAmend, do not rewrite.\n",
    );
    r.write(
        "docs/token.md",
        "---\ntype: contract\nstatus: active\n---\n# Token\n\n- **ttl**: The gateway SHALL expire a token after 30 min.\n",
    );
    r.write("docs/s.md", "# S\n\n- **run**: The system SHALL run.\n");
    r.commit("docs");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    // `context` serves the conventions of the target type first.
    let (_, out, _) = r.run(&["context", "docs/token.md"]);
    let conv = out.find("<!-- docs#contract-changes h:").expect(&out);
    let example = out.find("<!-- docs#example h:").expect(&out);
    let entry = out.find("<!-- token#token h:").expect(&out);
    assert!(conv < example && example < entry, "{out}");
    assert!(out.contains("convention -->"), "{out}");
    assert!(!out.contains("<!-- docs#other h:"), "{out}");
    assert!(!out.contains("Amend, do not rewrite."), "{out}");
    // A spec has no conventions.
    let (_, out, _) = r.run(&["context", "s"]);
    assert!(!out.contains("convention -->"), "{out}");
    // `show --conventions <type>` prints them.
    let (code, out, _) = r.run(&["show", "--conventions", "contract"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.starts_with("<!-- conventions of contract: docs/README.md#contract-changes -->"),
        "{out}"
    );
    assert!(
        out.contains("verified_against") && out.contains("See the token contract."),
        "{out}"
    );
    assert!(!out.contains("Not a convention."), "{out}");
    let (_, out, _) = r.run(&["show", "--conventions", "decision"]);
    assert!(out.contains("Amend, do not rewrite."), "{out}");
    assert!(
        out.contains("<!-- decision: amendment sections: Amendments; live sections: none -->"),
        "{out}"
    );
    let (_, out, _) = r.run(&["show", "--conventions", "spec"]);
    assert!(out.contains("No conventions for `spec`"), "{out}");
    let (_, out, _) = r.run(&["show", "--conventions", "contract", "--json"]);
    assert!(
        out.contains("\"type\":\"contract\"") && out.contains("\"sections\":[{"),
        "{out}"
    );
    let (code, _, err) = r.run(&["show", "--conventions", "nope"]);
    assert_eq!(code, 2, "{err}");
    // A pointer that names no section is a new error after a rename.
    r.write(
        "docs/README.md",
        &r.read("docs/README.md")
            .replace("## Contract changes", "## Contracts"),
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("spec-dna.toml:1: error SD106 convention-unresolved: `conventions.contract` points to `docs/README.md#contract-changes`"),
        "{out}"
    );
    let (_, out, _) = r.run(&["show", "--conventions", "contract"]);
    assert!(
        out.contains("names no document, file or section (SD106)"),
        "{out}"
    );
    // An unknown type in `conventions` is a configuration error.
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[conventions]\nadr = [\"rules\"]\n[[roots]]\npath = \"docs\"\n",
    );
    let (code, _, err) = r.run(&["check"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("conventions.adr"), "{err}");
}

#[test]
fn conventions_can_name_a_file_outside_the_documents() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[conventions]\ncontract = [\"docs/README.md#contract-changes\", \"./CONTRIBUTING.md#docs\"]\n[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nstatus = \"active\"\nexclude = [\"README.md\"]\n",
    );
    // An excluded README and a file outside every root: not documents.
    r.write(
        "docs/README.md",
        &format!(
            "# Docs\n\n## Contract changes\n\nA contract change needs a dated amendment.\n\n{}\n\n### Example\n\nSee the token contract.\n\n## Other\n\nNot a convention.\n",
            "Each amendment names its date and its reason. ".repeat(12)
        ),
    );
    r.write(
        "CONTRIBUTING.md",
        "# Contributing\n\n## Docs\n\nWrite docs in plain English.\n\n## Code\n\nNot a docs rule.\n",
    );
    r.write(
        "docs/token.md",
        "---\ntype: contract\nstatus: active\n---\n# Token\n\n- **ttl**: The gateway SHALL expire a token after 30 min.\n",
    );
    r.commit("docs");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("SD106"), "{out}");
    // The files stay outside the corpus: no budget, id or link check sees them.
    let (_, out, _) = r.run(&["check", "--all", "--json"]);
    assert!(!out.contains("\"path\":\"docs/README.md\""), "{out}");
    // `context` serves the file sections first, in the convention tier.
    let (code, out, _) = r.run(&["context", "docs/token.md"]);
    assert_eq!(code, 0, "{out}");
    let readme = out
        .find("<!-- docs/README.md#contract-changes h:")
        .expect(&out);
    let example = out.find("<!-- docs/README.md#example h:").expect(&out);
    let contributing = out.find("<!-- CONTRIBUTING.md#docs h:").expect(&out);
    let entry = out.find("<!-- token#token h:").expect(&out);
    assert!(
        readme < example && example < entry && contributing < entry,
        "{out}"
    );
    assert!(out.contains("file convention -->"), "{out}");
    assert!(out.contains("Write docs in plain English."), "{out}");
    assert!(
        !out.contains("Not a convention.") && !out.contains("Not a docs rule."),
        "{out}"
    );
    let (_, out, _) = r.run(&["context", "docs/token.md", "--json"]);
    assert!(
        out.contains("\"type\":\"file\"") && out.contains("\"tier\":\"convention\""),
        "{out}"
    );
    // A small budget lists a file section; `show` reads it by its id.
    let (_, out, _) = r.run(&["context", "docs/token.md", "--budget", "400"]);
    assert!(
        out.contains("- `docs/README.md#contract-changes` h:"),
        "{out}"
    );
    let (code, out, _) = r.run(&["show", "docs/README.md#contract-changes"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("file docs/README.md -->") && out.contains("See the token contract."),
        "{out}"
    );
    let (_, out, _) = r.run(&["show", "CONTRIBUTING.md#docs", "--json"]);
    assert!(out.contains("\"type\":\"file\""), "{out}");
    // `show --conventions` prints the file sections.
    let (code, out, _) = r.run(&["show", "--conventions", "contract"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("A contract change needs a dated amendment.")
            && out.contains("Write docs in plain English."),
        "{out}"
    );
    // A change in the working tree is served at once (the file is in the
    // status pathspec, so the parse cache does not keep the old text).
    r.write(
        "CONTRIBUTING.md",
        "# Contributing\n\n## Docs\n\nWrite docs in Simplified Technical English.\n",
    );
    let (_, out, _) = r.run(&["show", "--conventions", "contract"]);
    assert!(out.contains("Simplified Technical English"), "{out}");
    // A renamed heading or a removed file is SD106.
    r.write(
        "CONTRIBUTING.md",
        "# Contributing\n\n## Documents\n\nWrite docs.\n",
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("SD106 convention-unresolved: `conventions.contract` points to `./CONTRIBUTING.md#docs`, which names no document, file or section"),
        "{out}"
    );
    r.commit("rename");
    fs::remove_file(r.dir.join("docs/README.md")).unwrap();
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("points to `docs/README.md#contract-changes`"),
        "{out}"
    );
    // A path that leaves the repository is SD106, not a git failure.
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[conventions]\ncontract = [\"../outside.md\", \"/abs.md\"]\n[[roots]]\npath = \"docs\"\n",
    );
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.contains("points to `../outside.md`") && out.contains("points to `/abs.md`"),
        "{out}"
    );
}

#[cfg(unix)]
#[test]
fn a_symlinked_convention_file_is_read_again_and_stays_inside() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[conventions]\nspec = [\"link.md#rule\"]\ndecision = [\"away.md\"]\n[[roots]]\npath = \"docs\"\ntype = \"spec\"\nstatus = \"active\"\n",
    );
    r.write("real.md", "# Real\n\n## Rule\n\nold rule\n");
    std::os::unix::fs::symlink("real.md", r.dir.join("link.md")).unwrap();
    // A link to a file outside the repository.
    let outside = r.dir.with_extension("outside.md");
    fs::write(&outside, "# Secret\n\nnot for agents\n").unwrap();
    std::os::unix::fs::symlink(&outside, r.dir.join("away.md")).unwrap();
    r.write("docs/s.md", "# S\n\n- **run**: The system SHALL run.\n");
    r.commit("docs");
    let (_, out, _) = r.run(&["show", "--conventions", "spec"]);
    assert!(out.contains("old rule"), "{out}");
    // The target changes, the link does not: the next run reads the new text.
    r.write(
        "real.md",
        "# Real\n\n## Intro\n\nA much longer introduction paragraph.\n\n## Rule\n\nnew rule\n",
    );
    let (_, out, _) = r.run(&["show", "--conventions", "spec"]);
    assert!(out.contains("new rule") && !out.contains("Intro"), "{out}");
    let (_, out, _) = r.run(&["context", "docs/s.md"]);
    assert!(out.contains("new rule"), "{out}");
    // spec-dna does not read a link that leaves the repository.
    let (_, out, _) = r.run(&["show", "--conventions", "decision"]);
    assert!(!out.contains("not for agents"), "{out}");
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("points to `away.md`"), "{out}");
    let _ = fs::remove_file(outside);
}

#[test]
fn many_warnings_of_one_code_are_one_count() {
    // A mature corpus without tests links: one SD608 for each requirement.
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\ntype = \"spec\"\nstatus = \"active\"\n",
    );
    let reqs = |n: usize| -> String {
        (1..=n)
            .map(|i| format!("- **r{i}**: The system SHALL do step {i}.\n"))
            .collect()
    };
    r.write("docs/a.md", &format!("# A\n\n{}", reqs(7)));
    r.write("docs/b.md", &format!("# B\n\n{}", reqs(5)));
    r.commit("docs");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains(
            "warning SD608 requirement-unverified: 12 in 2 documents (`check --json` lists them)"
        ),
        "{out}"
    );
    assert!(!out.contains("docs/a.md:3: warning SD608"), "{out}");
    assert!(out.contains("SD608 fix: "), "{out}");
    assert!(
        out.ends_with("spec-dna check: 2 documents; 0 errors and 12 warnings\n"),
        "{out}"
    );
    let (_, out, _) = r.run(&["check", "--all", "--json"]);
    assert_eq!(out.matches("\"code\":\"SD608\"").count(), 12, "{out}");
    // 10 or fewer are listed one by one, as in a change with new
    // requirements.
    r.write("docs/b.md", "# B\n\nNo requirements.\n");
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(out.contains("docs/a.md:3: warning SD608"), "{out}");
    assert!(!out.contains("(`check --json` lists them)"), "{out}");
}

#[test]
fn lint_severity_follows_the_root() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"old\"\ntype = \"spec\"\nstatus = \"draft\"\nlint-severity = \"off\"\n[[roots]]\npath = \"new\"\ntype = \"spec\"\nstatus = \"draft\"\nlint-severity = \"error\"\n[[roots]]\npath = \"docs\"\ntype = \"spec\"\nstatus = \"draft\"\nprofile = \"strict\"\n[[roots]]\npath = \"std\"\ntype = \"spec\"\nstatus = \"draft\"\n",
    );
    for root in ["old", "new", "docs", "std"] {
        r.write(
            &format!("{root}/{root}-spec.md"),
            "# S\n\n- **retry**: The client retries.\n",
        );
    }
    r.commit("docs");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 1, "{out}");
    assert!(!out.contains("old/old-spec.md"), "{out}");
    assert!(out.contains("new/new-spec.md:3: error SD801"), "{out}");
    // A lint error is listed. A lint warning is a count, and --lint lists it.
    assert!(
        out.contains(
            "warning SD801 requirement-keyword: 1 in docs/docs-spec.md (`check --lint` lists them)"
        ),
        "{out}"
    );
    // The `standard` profile has no lint.
    assert!(!out.contains("std/std-spec.md"), "{out}");
    let (_, out, _) = r.run(&["check", "--all", "--lint"]);
    assert!(out.contains("docs/docs-spec.md:3: warning SD801"), "{out}");
    // The base has the same error, so the gate passes.
    let (code, _, _) = r.run(&["check"]);
    assert_eq!(code, 0);
    let (_, json, _) = r.run(&["check", "--all", "--json"]);
    assert!(
        json.contains("\"code\":\"SD801\",\"name\":\"requirement-keyword\",\"severity\":\"error\""),
        "{json}"
    );
}

#[test]
fn fresh_project_init_check_uninstall() {
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.starts_with(
            "spec-dna init: greenfield repository; profile standard; 1 root (see spec-dna.toml)\nwrote spec-dna.toml\nwrote AGENTS.md\nwrote CLAUDE.md\nwrote .claude/skills/spec-dna/SKILL.md\nper-request prefix: "
        ),
        "{out}"
    );
    assert!(
        out.ends_with("\nto commit: git add spec-dna.toml AGENTS.md CLAUDE.md .claude/skills/spec-dna/SKILL.md\n"),
        "{out}"
    );
    assert_eq!(r.read("AGENTS.md").lines().count(), 1);
    assert_eq!(r.read("CLAUDE.md"), r.read("AGENTS.md"));
    let docs = [
        (
            "docs/api/orders.md",
            "# Orders\n\n## Requirements\n\n- **create**: When a client posts an order, the API SHALL return its id.\n- **cancel**: While an order is open, the client MAY cancel it.\n",
        ),
        (
            "docs/decisions/order-ids.md",
            "---\ntype: decision\nstatus: accepted\n---\n# Order ids\n\n## Context and Problem Statement\n\nOrders need ids that sort by time.\n\n## Decision Outcome\n\nChosen option: time-ordered ids, used by [[api/orders#create]].\n",
        ),
        (
            "docs/guides/errors.md",
            "---\ntype: guide\nstatus: active\napplies-to: [api]\n---\n# Errors\n\nReturn errors as problem details. See [orders](../api/orders.md#requirements).\n",
        ),
    ];
    for (p, t) in docs {
        r.write(p, t);
    }
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("0 errors and 0 warnings"), "{out}");
    r.commit("docs");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    let (code, out, _) = r.run(&["context", "api/orders"]);
    assert_eq!(code, 0);
    assert!(out.contains("<!-- guides/errors#errors h:"), "{out}");
    assert!(
        out.contains("<!-- decisions/order-ids#decision-outcome h:"),
        "{out}"
    );
    // Uninstall: delete the configuration, the adapters and the cache.
    for p in ["spec-dna.toml", "AGENTS.md", "CLAUDE.md"] {
        fs::remove_file(r.dir.join(p)).unwrap();
    }
    fs::remove_dir_all(r.dir.join(".claude")).unwrap();
    fs::remove_dir_all(r.dir.join(".git/spec-dna")).unwrap();
    for (p, t) in docs {
        assert_eq!(r.read(p), t, "{p} is unchanged");
    }
    let status = git(&r.dir, &["status", "--porcelain"]);
    assert_eq!(
        status,
        " D .claude/skills/spec-dna/SKILL.md\n D AGENTS.md\n D CLAUDE.md\n D spec-dna.toml\n"
    );
}

#[test]
fn adapters_are_checked() {
    let r = Repo::empty();
    r.write("AGENTS.md", "# Rules\n\nOwn text stays.\n");
    r.write("CLAUDE.md", "@AGENTS.md\n");
    r.run(&["init"]);
    let agents = r.read("AGENTS.md");
    assert!(agents.starts_with("# Rules\n\nOwn text stays.\n\nIf a task edits code or docs"));
    assert_eq!(agents.lines().count(), 5);
    assert_eq!(
        r.read("CLAUDE.md"),
        "@AGENTS.md\n",
        "an existing import is kept and gets no line"
    );
    r.commit("init");
    let (code, out, _) = r.run(&["init"]);
    assert_eq!(code, 0);
    assert!(
        out.starts_with("spec-dna init: everything is up to date\nper-request prefix: "),
        "{out}"
    );
    r.write(
        "AGENTS.md",
        &r.read("AGENTS.md")
            .replace("Before a commit", "Before a push"),
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1);
    assert!(
        out.contains("AGENTS.md:5: error SD501 adapter-modified"),
        "{out}"
    );
}

#[test]
fn exit_codes() {
    let r = Repo::fixture("corpus");
    assert_eq!(r.run(&["frobnicate"]).0, 2);
    assert_eq!(r.run(&["check", "--bogus"]).0, 2);
    assert_eq!(r.run(&["show", "no/such-doc"]).0, 5);
    assert_eq!(r.run(&["context", "no/such-doc"]).0, 5);
    r.write(
        "spec-dna.toml",
        &r.read("spec-dna.toml")
            .replace(&format!("spec-dna = \"{VERSION}\""), "spec-dna = \"9.9.9\""),
    );
    let (code, _, err) = r.run(&["check"]);
    assert_eq!(code, 3);
    assert!(
        err.contains("cargo install --locked --force --git"),
        "{err}"
    );
    assert!(
        err.contains(&format!("set `spec-dna = \"{VERSION}\"`")),
        "{err}"
    );
    let (code, _, err) = r.run(&["init"]);
    assert_eq!(code, 3, "init does not regenerate adapters for another pin");
    assert!(err.contains("pins version 9.9.9"), "{err}");
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\nkind = \"spec\"\n",
    );
    let (code, _, err) = r.run(&["check"]);
    assert_eq!(code, 3);
    assert!(err.contains("unknown key `roots[0].kind`"), "{err}");
    let outside = std::env::temp_dir();
    let out = Command::new(BIN)
        .arg("check")
        .current_dir(&outside)
        .output()
        .unwrap();
    assert_ne!(out.status.code(), Some(0));
    assert_eq!(
        Command::new(BIN).arg("--version").output().unwrap().stdout,
        format!("spec-dna {VERSION}\n").as_bytes()
    );
}

#[test]
fn help_of_one_command_is_short() {
    let run = |args: &[&str]| {
        let o = Command::new(BIN).args(args).output().unwrap();
        (
            o.status.code(),
            String::from_utf8_lossy(&o.stdout).into_owned(),
        )
    };
    let (code, all) = run(&["--help"]);
    assert_eq!(code, Some(0));
    assert!(
        all.contains("Commands:") && all.contains("transcript"),
        "{all}"
    );
    let (code, again) = run(&["help", "help"]);
    assert_eq!((code, again), (Some(0), all.clone()));
    for args in [&["mv", "--help"][..], &["help", "mv"], &["mv", "-h"]] {
        let (code, one) = run(args);
        assert_eq!(code, Some(0), "{args:?}");
        assert!(
            one.starts_with("usage: spec-dna mv <from> <to> [--stub]\n"),
            "{one}"
        );
        assert!(!one.contains("transcript"), "{one}");
        assert!(one.len() * 4 < all.len(), "{} of {}", one.len(), all.len());
    }
    let (code, _) = run(&["nothing", "--help"]);
    assert_eq!(code, Some(2));
}

#[test]
fn hard_budget_is_an_error_for_new_and_growing_documents() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nstatus = \"active\"\n[[roots]]\npath = \"lax\"\ndefault-type = \"spec\"\nstatus = \"active\"\nenforce-budgets = false\n[budgets]\ngrowth = 20\n[budgets.spec]\nsoft = 100\nhard = 200\n",
    );
    let words = |n: usize| "word ".repeat(n);
    r.write("docs/big.md", &format!("# Big\n\n{}\n", words(50)));
    r.write("docs/small.md", "# Small\n\nShort.\n");
    r.write("lax/wide.md", &format!("# Wide\n\n{}\n", words(50)));
    // Without a base commit, every document is new.
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("docs/big.md:1: error SD702 budget-hard: 258 characters, over the hard budget of 200 (a new document)"),
        "{out}"
    );
    assert!(
        out.contains("lax/wide.md:1: warning SD704 budget-hard-kept"),
        "{out}"
    );
    r.commit("docs");
    // An overage that the base has and that does not grow is a warning, also
    // with --all.
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("docs/big.md:1: warning SD704 budget-hard-kept: 258 characters, over the hard budget of 200 (no growth against the base)"),
        "{out}"
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("SD704"), "an old warning is not new: {out}");
    // Growth within the allowance (`[budgets] growth`, here 20 characters)
    // is a warning: a task can add a short amendment without a split.
    r.append("docs/big.md", "\nA bit more.\n");
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("docs/big.md:1: warning SD703 budget-grew: 271 characters, over the hard budget of 200 (grew by 13 characters, at most 20 in one change)"),
        "{out}"
    );
    git(&r.dir, &["checkout", "-q", "--", "docs/big.md"]);
    // Net growth of a document over the hard budget beyond the allowance is
    // an error.
    r.append("docs/big.md", &format!("\n{}\n", words(10)));
    r.append("lax/wide.md", &format!("\n{}\n", words(10)));
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("docs/big.md:1: error SD702 budget-hard: 310 characters, over the hard budget of 200 (grew from 258 characters in the base)"),
        "{out}"
    );
    assert!(
        out.contains("lax/wide.md:1: warning SD704 budget-hard-kept: 311 characters, over the hard budget of 200 (grew from 259 characters in the base; the root does not enforce budgets)"),
        "{out}"
    );
    // A document that shrinks but stays over the budget is not an error.
    git(&r.dir, &["checkout", "-q", "--", "."]);
    r.write("docs/big.md", &format!("# Big\n\n{}\n", words(45)));
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    // A document that goes over the hard budget in this change is an error.
    r.append("docs/small.md", &format!("\n{}\n", words(50)));
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("docs/small.md:1: error SD702 budget-hard: 268 characters, over the hard budget of 200 (16 characters in the base)"),
        "{out}"
    );
}

#[test]
fn legacy_statuses_and_required_sections_follow_the_root() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nstatus = \"active\"\nstatus-map = { released = \"active\", signed-off = \"accepted\" }\nrequired-sections = { decision = [\"Lösung\"], spec = [\"Requirements\", \"Anforderungen\"] }\n",
    );
    r.write(
        "docs/cache.md",
        "---\nstatus: released\n---\n# Cache\n\n## Requirements\n\n- **ttl**: The cache SHALL drop an entry after 60 seconds.\n",
    );
    r.write(
        "docs/cache-store.md",
        "---\ntype: decision\nstatus: signed-off\n---\n# Cache store\n\n## Kontext\n\nEin gemeinsamer Cache für [[cache]].\n\n## Lösung\n\nRedis.\n",
    );
    r.write("docs/notes.md", "# Notes\n\nNo requirements here.\n");
    r.write(
        "docs/de.md",
        "# De\n\n## Anforderungen\n\n- **ttl**: The cache SHALL drop an entry after 60 seconds.\n",
    );
    r.write(
        "docs/old-store.md",
        "---\ntype: decision\nstatus: accepted\n---\n# Old store\n\n## Decision Outcome\n\nMemory.\n",
    );
    r.commit("docs");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 1, "{out}");
    let found: Vec<&str> = out.lines().filter(|l| l.contains(" error SD")).collect();
    assert_eq!(
        found,
        [
            "docs/notes.md:1: error SD305 section-missing: a spec needs a section headed `Requirements` or `Anforderungen`",
            "docs/old-store.md:1: error SD305 section-missing: a decision needs a section headed `Lösung`"
        ],
        "{out}"
    );
    // The mapped status is equal to `accepted`, so `context` serves the
    // decision with its outcome section.
    let (code, out, _) = r.run(&["context", "cache"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("decision/accepted decision -->\n## Lösung\n\nRedis."),
        "{out}"
    );
}

#[test]
fn self_urls_link_documents_inside_the_repository() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\nself-urls = [\"https://example.org/acme/specs/blob/*/\", \"https://example.org/acme/specs/tree/*/\"]\n[[roots]]\npath = \"docs\"\ndefault-type = \"guide\"\nstatus = \"active\"\n",
    );
    r.write(
        "docs/x/README.md",
        "# X\n\nSee [Y](https://example.org/acme/specs/tree/main/docs/y) and [Z](https://example.org/acme/specs/blob/main/docs/z.md#part), not [W](https://example.org/acme/specs/blob/old/docs/w.md).\n",
    );
    r.write("docs/y/README.md", "# Y\n\nY text.\n");
    r.write("docs/z.md", "# Z\n\n## Part\n\nZ text.\n");
    r.commit("docs");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    let (code, out, _) = r.run(&["context", "x"]);
    assert_eq!(code, 0);
    assert!(out.contains("(linked): Y\n"), "{out}");
    assert!(out.contains("(linked): Z\n"), "{out}");
}

#[test]
fn query_ranks_matching_sections_first() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\ndefault-type = \"guide\"\nstatus = \"active\"\n",
    );
    let filler = |w: &str| format!("{w} ").repeat(120);
    r.write(
        "docs/long.md",
        &format!(
            "# Long\n\nIntro.\n\n## Alpha\n\n{}\n\n## Beta\n\n{}\n\n## Gamma\n\nThe restart backoff limit is 300 seconds. {}\n",
            filler("apples"),
            filler("pears"),
            filler("plums")
        ),
    );
    r.commit("docs");
    let full = |out: &str, s: &str| out.contains(&format!("<!-- long#{s} h:"));
    let (code, out, _) = r.run(&["context", "long", "--budget", "1200"]);
    assert_eq!(code, 0);
    assert!(full(&out, "alpha") && !full(&out, "gamma"), "{out}");
    let args = [
        "context",
        "long",
        "--budget",
        "1200",
        "--query",
        "Lower the restart backoff",
    ];
    let (code, out, _) = r.run(&args);
    assert_eq!(code, 0);
    assert!(full(&out, "gamma") && !full(&out, "alpha"), "{out}");
    // A section that the target names comes first, whether it matches or not.
    let (_, named, _) = r.run(&[
        "context",
        "long#beta",
        "--budget",
        "1200",
        "--query",
        "restart backoff",
    ]);
    assert!(full(&named, "beta"), "{named}");
    assert!(out.contains("(query: lower restart backoff);"), "{out}");
    assert_eq!(r.run(&args).1, out, "same input, same output");
    let mut json = args.to_vec();
    json.push("--json");
    let (_, out, _) = r.run(&json);
    assert!(
        out.contains("\"query\":[\"lower\",\"restart\",\"backoff\"]"),
        "{out}"
    );
}

#[test]
fn legacy_requirements_without_a_colon_are_recognized() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nstatus = \"active\"\nlegacy-keys = [\"R-{n}\"]\n",
    );
    r.write(
        "docs/orders.md",
        "# Orders\n\n## Rules\n\n- **R-1** The API SHALL return the order id.\n- **R-2:** The client MAY cancel an open order.\n- **Note** a bold label is prose.\n",
    );
    r.write(
        "docs/table.md",
        "# Table\n\n| Key | Rule |\n| --- | --- |\n| R-3 | The API SHALL log. |\n",
    );
    r.commit("docs");
    let (code, out, _) = r.run(&["check", "--all", "--json"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("{\"id\":\"orders\",\"path\":\"docs/orders.md\",\"type\":\"spec\",\"status\":\"active\",\"chars\":"),
        "{out}"
    );
    assert!(out.contains(",\"requirements\":2}"), "{out}");
    assert!(
        out.contains("\"requirements\":2,"),
        "the summary counts them: {out}"
    );
    assert!(
        out.contains("\"code\":\"SD806\",\"name\":\"requirements-unrecognized\""),
        "{out}"
    );
    let (_, out, _) = r.run(&["show", "orders"]);
    assert!(
        out.starts_with("<!-- orders: 2 requirements -->\n"),
        "{out}"
    );
    let (_, out, _) = r.run(&["show", "orders", "--json"]);
    assert!(out.contains("\"requirements\":[\"R-1\",\"R-2\"]"), "{out}");
    let (code, out, _) = r.run(&["show", "orders#R-1"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("- **R-1** The API SHALL return the order id."),
        "{out}"
    );
}

#[test]
fn context_lists_inactive_dependencies() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nstatus = \"active\"\n",
    );
    r.write(
        "docs/app.md",
        "---\ndepends-on: [old-store, store]\n---\n# App\n\n## Requirements\n\n- **save**: The app SHALL save each order.\n",
    );
    r.write(
        "docs/old-store.md",
        "---\nstatus: superseded\n---\n# Old store\n\n## Requirements\n\n- **disk**: The store SHALL write to local disk.\n",
    );
    r.write(
        "docs/store.md",
        "---\nsupersedes: [old-store]\n---\n# Store\n\n## Requirements\n\n- **db**: The store SHALL write to the database.\n",
    );
    r.commit("docs");
    let (code, out, _) = r.run(&["context", "app"]);
    assert_eq!(code, 0);
    assert!(out.contains("<!-- store#requirements h:"), "{out}");
    assert!(!out.contains("local disk"), "{out}");
    assert!(
        out.contains("(depends): Old store [superseded; successor: `store`]\n"),
        "{out}"
    );
    let (_, out, _) = r.run(&["context", "app", "--json"]);
    assert!(
        out.contains("\"status\":\"superseded\",\"anchor\":\"old-store\""),
        "{out}"
    );
    assert!(out.contains("\"successors\":[\"store\"]"), "{out}");
    let (_, out, _) = r.run(&["context", "app", "--include-inactive"]);
    assert!(out.contains("local disk"), "{out}");
    // A target that names the document serves it.
    let (_, out, _) = r.run(&["context", "old-store"]);
    assert!(out.contains("local disk"), "{out}");
}

#[test]
fn context_serves_requirement_sections_first() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nstatus = \"active\"\nlegacy-keys = [\"R-{n}\"]\n",
    );
    r.write(
        "docs/pay.md",
        "# Payments\n\nIntro.\n\n## Background\n\n- **Note**: a label, not a requirement.\n\n## Rules\n\n- **R-1** The service SHALL retry a failed payment once.\n\n## History\n\nOld text.\n\n## Limits\n\n- **R-2** The service SHALL refuse amounts over 10,000.\n",
    );
    r.write(
        "docs/guide.md",
        "---\ntype: guide\n---\n# Guide\n\n## First\n\nA.\n\n## Second\n\nB.\n",
    );
    r.commit("docs");
    let (code, out, _) = r.run(&["context", "pay"]);
    assert_eq!(code, 0);
    let order: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with("<!-- pay"))
        .map(|l| l.split(' ').nth(1).unwrap())
        .collect();
    assert_eq!(order, ["pay#rules", "pay#limits", "pay#payments"], "{out}");
    assert!(out.contains("- `pay#background` h:"), "{out}");
    assert!(out.contains("- `pay#history` h:"), "{out}");
    // A document without requirements keeps the heading order.
    let (_, out, _) = r.run(&["context", "guide"]);
    let order: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with("<!-- guide"))
        .map(|l| l.split(' ').nth(1).unwrap())
        .collect();
    assert_eq!(
        order,
        ["guide#guide", "guide#first", "guide#second"],
        "{out}"
    );
}

#[test]
fn base_gate_reads_tracked_files_only() {
    let r = Repo::fixture("corpus");
    r.write("docs/guides/draft.md", "# Draft\n\nSee [gone](gone.md).\n");
    // The working-tree check sees an untracked document.
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("docs/guides/draft.md:3: error SD102"), "{out}");
    // A gate against a base reads tracked files only, as CI does.
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("draft.md"), "{out}");
    git(&r.dir, &["add", "docs/guides/draft.md"]);
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("docs/guides/draft.md:3: error SD102"), "{out}");
}

#[test]
fn transcript_reports_counts_and_ids_only() {
    let r = Repo::fixture("corpus");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/transcripts");
    let dir = dir.to_str().unwrap();
    let (code, out, err) = r.run(&[
        "transcript",
        dir,
        "--expect",
        "auth/tokens",
        "--expect",
        "ADR-1",
    ]);
    assert_eq!(code, 0, "{err}");
    let expected = [
        "spec-dna transcript: 1 session, 2 agents, 3 episodes, 10 requests\n",
        // Streamed lines repeat the usage of one request, and a forked
        // subagent repeats a request and a tool call of its parent. Each
        // counts once. A `<synthetic>` message is not a request.
        "  10 requests; tokens: input 26, cache creation 1700, cache read 10930, output 185\n",
        "  11 tool calls (Read 5, Bash 4, Edit 1, Grep 1); 1 error; 1 search\n",
        "    ADR-1-session-store: 1 read, full, via read\n",
        // A read with a line range gives sections. spec-dna output gives the
        // sections that it served, and only ids that are documents.
        "    auth/session: 2 reads, auth/session#requirements, via read+spec-dna\n",
        // One of the 2 reads covers the front matter only.
        "    auth/tokens: 2 reads, full, via read+shell\n",
        // A shell path relative to the working directory of the agent.
        "    billing/invoice: 1 read, full, via shell\n",
        "    guides/logging: 1 read, guides/logging#logging, via spec-dna\n",
        "    document edits: auth/session 1\n",
        "  expected: {\"read\":2,\"of\":2,\"first_relevant_ms\":4000}\n",
        // A compaction summary does not start an episode.
        "  episode 1 (main, human), 60 s:\n",
        "  episode 2 (a1, human), 2 s:\n",
        "  episode 3 (main, task-notification), 6 s:\n",
    ];
    for e in expected {
        assert!(out.contains(e), "missing {e:?} in\n{out}");
    }
    assert!(!out.contains("ghost/missing"), "{out}");
    // A workflow journal in the directory is not a transcript: a warning only.
    assert!(
        err.contains("journal.jsonl: no transcript adapter reads this file"),
        "{err}"
    );
    // A file that two arguments give is read once.
    let file = format!("{dir}/session-1.jsonl");
    let (_, twice, _) = r.run(&[
        "transcript",
        dir,
        &file,
        "--expect",
        "auth/tokens",
        "--expect",
        "ADR-1",
    ]);
    assert_eq!(twice, out);
    let (code, json, _) = r.run(&["transcript", dir, "--json"]);
    assert_eq!(code, 0);
    assert!(json.contains("\"documents_mapped\":true"), "{json}");
    assert!(
        json.contains(
            "\"tokens\":{\"input\":26,\"cache_creation\":1700,\"cache_read\":10930,\"output\":185}"
        ),
        "{json}"
    );
    assert!(json.contains("\"spec_tokens_estimated\":"), "{json}");
    // The transcripts hold secret text in prompts, commands, file contents,
    // paths and answers. None of it reaches the output.
    for text in [&out, &json] {
        assert!(!text.contains("SECRET"), "{text}");
        assert!(!text.contains("/work/export"), "{text}");
    }
    // Outside a configured repository, file reads are not mapped. The ids in
    // spec-dna output still count.
    let empty = Repo::empty();
    let (code, out, _) = empty.run(&["transcript", dir]);
    assert_eq!(code, 0);
    assert!(
        out.contains("no spec-dna.toml, so file reads are not mapped to documents"),
        "{out}"
    );
    assert!(out.contains("other files: 4 reads"), "{out}");
    assert!(!out.contains("SECRET"), "{out}");
    assert!(
        !out.contains("via read") && !out.contains("via shell"),
        "{out}"
    );
    assert_eq!(empty.run(&["transcript", "no/such/file.jsonl"]).0, 2);
    // A session id, agent id, origin, model or tool name that does not match
    // its strict pattern prints as `<redacted>`.
    let unsafe_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/transcripts-unsafe");
    let unsafe_dir = unsafe_dir.to_str().unwrap();
    for args in [
        vec!["transcript", unsafe_dir],
        vec!["transcript", unsafe_dir, "--json"],
    ] {
        let (code, out, _) = empty.run(&args);
        assert_eq!(code, 0);
        assert!(!out.contains("SECRET") && !out.contains("sk-"), "{out}");
        assert!(out.contains("<redacted>"), "{out}");
        assert!(out.contains("Read"), "{out}");
    }
    let (_, json, _) = empty.run(&["transcript", unsafe_dir, "--json"]);
    assert!(json.contains("\"models\":[\"<redacted>\"]"), "{json}");
    assert!(json.contains("\"models\":[\"claude-opus-5-5\"]"), "{json}");
    let (_, out, _) = empty.run(&["transcript", unsafe_dir]);
    assert!(out.contains("session <redacted> (claude-code)"), "{out}");
    assert!(out.contains("episode 1 (main, <redacted>)"), "{out}");
    assert!(out.contains("episode 2 (<redacted>, human)"), "{out}");
    assert!(out.contains("(<redacted> 1, Read 1)"), "{out}");
    empty.write("x.jsonl", "{\"not\":\"a transcript\"}\n");
    let (code, _, err) = empty.run(&["transcript", "x.jsonl"]);
    assert_eq!(code, 2);
    assert!(
        err.contains("no transcript adapter reads this file"),
        "{err}"
    );
}

/// Reads the byte counts from the `init` summary line.
fn prefix_bytes(out: &str) -> usize {
    let line = out
        .lines()
        .find(|l| l.starts_with("per-request prefix: "))
        .unwrap();
    line["per-request prefix: ".len()..]
        .split(' ')
        .next()
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
fn init_on_an_existing_project_adds_one_line() {
    let r = Repo::empty();
    let own = "# Project rules\n\n- Run the tests before a commit.\n";
    r.write("CLAUDE.md", own);
    r.commit("existing project");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{err}");
    let claude = r.read("CLAUDE.md");
    let added = claude.strip_prefix(own).expect("the own text stays first");
    assert_eq!(added.trim_start().lines().count(), 1, "{claude}");
    assert!(!claude.contains('@'), "no import: {claude}");
    assert!(added.len() <= 160, "{} bytes", added.len());
    assert!(
        !r.dir.join("AGENTS.md").exists(),
        "no new AGENTS.md when CLAUDE.md exists"
    );
    let bytes = prefix_bytes(&out);
    assert!(bytes <= 300, "{out}");
    assert!(out.contains(", 0 bytes for AGENTS.md"), "{out}");
    let (code, out, _) = r.run(&["init", "--json"]);
    assert_eq!(code, 0);
    assert!(
        out.contains(&format!("\"prefix_bytes\":{{\"claude_code\":{bytes},")),
        "{out}"
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");

    // An AGENTS.md without CLAUDE.md: the line goes into AGENTS.md, and
    // CLAUDE.md gets its own line, not an import.
    let r = Repo::empty();
    r.write("AGENTS.md", "# Agents\n");
    r.run(&["init"]);
    assert_eq!(r.read("AGENTS.md").lines().count(), 3);
    assert_eq!(r.read("CLAUDE.md").lines().count(), 1);
    assert!(!r.read("CLAUDE.md").contains("@AGENTS.md"));
}

#[test]
fn init_replaces_the_blocks_of_older_versions() {
    let r = Repo::empty();
    r.run(&["init"]);
    let block = "<!-- spec-dna:begin agents v0.0.9 h:0000000000000000 -->\n## Specifications\n\nOld text.\n<!-- spec-dna:end agents -->\n";
    r.write("AGENTS.md", &format!("# Rules\n\n{block}"));
    r.write(
        "CLAUDE.md",
        "<!-- spec-dna:begin claude v0.0.9 h:0000000000000000 -->\n@AGENTS.md\n<!-- spec-dna:end claude -->\n",
    );
    r.commit("old adapters");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 1);
    assert!(out.contains("AGENTS.md:3: error SD502"), "{out}");
    assert!(out.contains("CLAUDE.md:1: error SD502"), "{out}");
    r.run(&["init"]);
    assert_eq!(r.read("AGENTS.md").lines().count(), 3);
    assert_eq!(
        r.read("CLAUDE.md"),
        "@AGENTS.md\n",
        "the import stays, because AGENTS.md has text of its own"
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");

    // An AGENTS.md with only the old block: CLAUDE.md gets the line.
    let r = Repo::empty();
    r.run(&["init"]);
    r.write("AGENTS.md", block);
    r.write(
        "CLAUDE.md",
        "<!-- spec-dna:begin claude v0.0.9 h:0000000000000000 -->\n@AGENTS.md\n<!-- spec-dna:end claude -->\n",
    );
    r.run(&["init"]);
    assert_eq!(r.read("CLAUDE.md"), r.read("AGENTS.md"));
    assert_eq!(r.read("CLAUDE.md").lines().count(), 1);

    // A CLAUDE.md that imports AGENTS.md keeps no second copy of the line.
    let r = Repo::empty();
    r.run(&["init"]);
    r.write(
        "CLAUDE.md",
        &format!("@AGENTS.md\n\n{}", r.read("CLAUDE.md")),
    );
    r.run(&["init"]);
    assert_eq!(r.read("CLAUDE.md"), "@AGENTS.md\n");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
}

const LOANS_PY: &str = "MAX_RENEWALS = 1\n\n\ndef renew_loan(loan):\n    return loan\n\n\ndef checkoutBook(member, book):\n    return (member, book)\n";

fn code_project() -> Repo {
    let r = Repo::empty();
    r.run(&["init"]);
    r.write("src/library/loans.py", LOANS_PY);
    r.write(
        "src/library/books.py",
        "def find_book(isbn):\n    return isbn\n",
    );
    r.write("src/library/fees.py", "def daily_fee():\n    return 1\n");
    r.write(
        "docs/loans.md",
        "# Loans\n\nThe loan module.\n\n## Renewals\n\n- **renew-limit**: The library SHALL allow at most `MAX_RENEWALS` renewals of a loan.\n\n## Checkout\n\n- **checkout**: When a member borrows a book, the library SHALL record the loan.\n\n## History\n\nOld notes.\n\n## Code\n\nSee `src/library/loans.py:4`.\n",
    );
    r.write(
        "docs/books.md",
        "# Books\n\n## Catalog\n\nThe catalog uses `find_book()`.\n\n## Lending\n\nA loan calls `checkoutBook()` from the books page.\n",
    );
    r.write(
        "docs/layout.md",
        "---\ntype: guide\nstatus: active\n---\n# Layout\n\nAll library code is in src/library/.\n",
    );
    r.write(
        "docs/notes/idea.md",
        "---\ntype: note\n---\n# Idea\n\nRewrite [loans](../../src/library/loans.py).\n",
    );
    r.commit("project");
    r
}

#[test]
fn context_serves_the_sections_that_cite_a_code_path() {
    let r = code_project();
    let (code, out, err) = r.run(&["context", "src/library/loans.py"]);
    assert_eq!(code, 0, "{err}");
    let order: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with("<!-- ") && l.contains(" h:"))
        .map(|l| l.split(' ').nth(1).unwrap())
        .collect();
    assert_eq!(
        order,
        [
            "loans#code",
            "loans#renewals",
            "books#lending",
            "loans#loans",
            "loans#checkout",
            "books#books",
            "layout#layout",
        ],
        "{out}"
    );
    assert!(out.contains("<!-- loans#code h:"), "{out}");
    assert!(out.contains(" cites -->"), "{out}");
    assert!(
        out.contains("- `notes/idea#idea`"),
        "a note is only listed: {out}"
    );
    assert!(out.contains("- `loans#history`"), "{out}");
    assert!(
        out.contains("- `books#catalog`") && !out.contains("<!-- books#catalog h:"),
        "the other sections of a citing document are only listed: {out}"
    );
    // `--path` gives the same sections after an entry.
    let (_, out, _) = r.run(&["context", "books", "--path", "src/library/loans.py"]);
    assert!(out.contains("<!-- loans#code h:"), "{out}");
    // A file that no document cites gives a hint.
    r.write("src/other.py", "x = 1\n");
    r.commit("other");
    let (code, out, _) = r.run(&["context", "src/other.py"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("<!-- no document cites or applies to src/other.py;"),
        "{out}"
    );
}

#[test]
fn check_warns_when_a_change_removes_cited_code() {
    let r = code_project();
    let base = git(&r.dir, &["rev-parse", "HEAD"]).trim().to_string();
    // Rename a cited function, remove a cited constant, and move the module.
    r.write(
        "src/library/loans.py",
        &LOANS_PY
            .replace("MAX_RENEWALS = 1\n", "")
            .replace("renew_loan", "extend_loan")
            .replace("checkoutBook", "lend_book"),
    );
    git(
        &r.dir,
        &["mv", "src/library/fees.py", "src/library/fines.py"],
    );
    r.append("docs/loans.md", "\nFees: `src/library/fees.py`.\n");
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "drift is a warning: {out}");
    assert!(
        out.contains("docs/loans.md:7: warning SD607 cited-symbol-removed: `MAX_RENEWALS` was removed from `src/library/loans.py` in this change"),
        "{out}"
    );
    assert!(
        out.contains("docs/books.md:9: warning SD607 cited-symbol-removed: `checkoutBook` was removed from `src/library/loans.py`"),
        "{out}"
    );
    assert!(
        out.contains("warning SD606 cited-path-removed: `src/library/fees.py` was removed in this change (renamed to `src/library/fines.py`)"),
        "{out}"
    );
    assert!(!out.contains("renew_loan"), "not cited: {out}");
    // The documents are fixed in the same change: no warning.
    r.write(
        "docs/books.md",
        &r.read("docs/books.md")
            .replace("checkoutBook()", "lend_book()"),
    );
    r.write(
        "docs/loans.md",
        &r.read("docs/loans.md")
            .replace("`MAX_RENEWALS`", "one")
            .replace("fees.py", "fines.py"),
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0);
    assert!(!out.contains("SD60"), "{out}");
    // A cited identifier that still occurs in another file is not removed.
    r.write(
        "docs/books.md",
        &r.read("docs/books.md")
            .replace("lend_book()", "checkoutBook()"),
    );
    r.write("src/library/legacy.py", "def checkoutBook():\n    pass\n");
    r.commit("change");
    let (code, out, _) = r.run(&["check", "--base", &base]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("SD607"), "{out}");
    // CI-like mode against the base: the rename of the constant is drift.
    r.write(
        "docs/loans.md",
        &r.read("docs/loans.md")
            .replace("at most one", "at most `MAX_RENEWALS`"),
    );
    r.commit("cite again");
    let (code, out, _) = r.run(&["check", "--base", &base]);
    assert_eq!(code, 0);
    assert!(
        out.contains("warning SD607 cited-symbol-removed: `MAX_RENEWALS`"),
        "{out}"
    );
}

#[test]
fn drift_sees_the_working_tree_outside_the_roots() {
    // A plain `rm` or `mv` of a cited file outside the roots, without `git rm`.
    let r = code_project();
    fs::remove_file(r.dir.join("src/library/fees.py")).unwrap();
    r.append("docs/loans.md", "\nFees: `src/library/fees.py`.\n");
    let (_, out, _) = r.run(&["check"]);
    assert!(
        out.contains("warning SD606 cited-path-removed: `src/library/fees.py` was removed"),
        "{out}"
    );
    let r = code_project();
    fs::rename(
        r.dir.join("src/library/loans.py"),
        r.dir.join("src/library/lending.py"),
    )
    .unwrap();
    let (_, out, _) = r.run(&["check"]);
    assert!(
        out.contains("warning SD606 cited-path-removed: `src/library/loans.py`"),
        "{out}"
    );
    assert!(
        !out.contains("SD607"),
        "the identifiers are in the new file: {out}"
    );
    // `check` reads git, but it does not write the git index.
    let index = r.dir.join(".git/index");
    let before = fs::metadata(&index).unwrap().modified().unwrap();
    r.run(&["check"]);
    assert_eq!(fs::metadata(&index).unwrap().modified().unwrap(), before);
}

#[test]
fn drift_has_no_false_warnings() {
    let r = code_project();
    // A file name that still names one file after a move is not drift.
    r.write(
        "docs/books.md",
        &r.read("docs/books.md")
            .replace("uses `find_book()`.", "uses `find_book()` from `books.py`."),
    );
    r.commit("cite a file name");
    git(&r.dir, &["mv", "src/library/books.py", "src/books.py"]);
    let (_, out, _) = r.run(&["check"]);
    assert!(!out.contains("SD606"), "{out}");
    // The grep settings of the user's git config do not change the result.
    git(&r.dir, &["config", "grep.lineNumber", "true"]);
    git(&r.dir, &["config", "grep.column", "true"]);
    r.write("src/library/legacy.py", "def checkoutBook():\n    pass\n");
    r.commit("legacy copy");
    r.write(
        "src/library/loans.py",
        &LOANS_PY.replace("checkoutBook", "lend_book"),
    );
    let (_, out, _) = r.run(&["check"]);
    assert!(!out.contains("SD607"), "{out}");
    // A link whose text is the path gives only the link error.
    r.append(
        "docs/loans.md",
        "\nSee [src/library/fees.py](../src/library/fees.py).\n",
    );
    r.commit("link");
    git(&r.dir, &["rm", "-q", "src/library/fees.py"]);
    let (_, out, _) = r.run(&["check"]);
    assert!(out.contains("error SD102"), "{out}");
    assert!(!out.contains("SD606"), "{out}");
}

#[test]
fn context_keeps_applies_to_above_directory_mentions() {
    let r = code_project();
    for i in 0..12 {
        r.write(
            &format!("docs/area{i}.md"),
            &format!("# Area {i}\n\nAll code is under src/.\n\n## Rules\n\n- **rule-{i}**: The library SHALL keep area {i}.\n"),
        );
    }
    r.write(
        "docs/guides/style.md",
        "---\ntype: guide\nstatus: active\napplies-to: [src/library/**]\n---\n# Style\n\nUse type hints.\n",
    );
    r.commit("more docs");
    let (_, out, _) = r.run(&["context", "src/library/loans.py"]);
    let served: Vec<&str> = out
        .lines()
        .filter(|l| l.starts_with("<!-- ") && l.contains(" h:"))
        .map(|l| l.split(' ').nth(1).unwrap())
        .collect();
    let style = served.iter().position(|s| *s == "guides/style#style");
    let area = served.iter().position(|s| s.starts_with("area"));
    assert!(style.is_some(), "{out}");
    assert!(area.is_none_or(|a| a > style.unwrap()), "{out}");
    assert!(
        served.iter().filter(|s| s.starts_with("area")).count() <= 5,
        "{out}"
    );
    // An untracked file is a valid target.
    r.write("src/library/new.py", "MAX_RENEWALS = 2\n");
    let (code, out, err) = r.run(&["context", "src/library/new.py"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("<!-- loans#renewals h:"), "{out}");
}

#[test]
fn moving_a_document_keeps_accepted_decisions_frozen() {
    let r = Repo::empty();
    r.run(&["init"]);
    r.write(
        "docs/loans.md",
        "# Loans\n\n- **daily-fine**: When a member returns a book late, the library SHALL charge a fine.\n",
    );
    r.write(
        "docs/decisions/fine.md",
        "---\ntype: decision\nstatus: accepted\n---\n# Fine\n\n## Decision Outcome\n\nA flat fine. See [daily-fine](../loans.md#daily-fine).\n",
    );
    r.commit("docs");
    let (code, out, err) = r.run(&["mv", "loans", "circulation/loans"]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("moved document loans -> circulation/loans"),
        "{out}"
    );
    assert!(
        r.read("docs/decisions/fine.md")
            .contains("(../circulation/loans.md#daily-fine)"),
        "mv rewrites the link"
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(
        code, 0,
        "a rewritten link is not a change of the decision: {out}"
    );
    // A change of the text is still a change.
    r.write(
        "docs/decisions/fine.md",
        &r.read("docs/decisions/fine.md")
            .replace("A flat fine", "A capped fine"),
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1);
    assert!(out.contains("error SD405 decision-frozen"), "{out}");
}

fn split_project() -> Repo {
    let r = Repo::empty();
    r.run(&["init"]);
    r.write(
        "docs/loans.md",
        "---\nstatus: active\n---\n# Loans\n\nThe loan rules. See [membership](#membership).\n\n## Membership\n\n- **member-registration**: When a person applies, the desk SHALL check an identity document.\n- **membership-kinds**: The library SHALL have two membership kinds.\n\n## Fines\n\nThe fine rules.\n\n- **daily-fine**: When a member returns a book late, the library SHALL charge 25 cents for each late day.\n\n### Fine cap\n\n- **fine-cap**: The library SHALL limit a fine to the replacement cost of the book.\n\n## Renewals\n\n- **renewal-limit**: The library SHALL allow 2 renewals of a loan.\n",
    );
    r.write(
        "docs/policy.md",
        "---\nstatus: active\n---\n# Policy\n\n## Scope\n\n- **scope**: The library SHALL apply this policy to all branches.\n",
    );
    r.write(
        "docs/decisions/fines.md",
        "---\ntype: decision\nstatus: accepted\n---\n# Fines\n\n## Decision Outcome\n\nA capped fine: [daily-fine](../loans.md#daily-fine), [cap](../loans.md#fine-cap).\n",
    );
    r.write(
        "docs/guide.md",
        "---\ntype: guide\nstatus: active\n---\n# Desk guide\n\nCheck [the identity](loans.md#member-registration) and [the fines](loans.md#fines).\n",
    );
    r.commit("docs");
    r
}

#[test]
fn mv_splits_keys_into_a_new_spec() {
    let r = split_project();
    let (code, out, err) = r.run(&[
        "mv",
        "loans#member-registration",
        "loans#membership-kinds",
        "members",
    ]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(
        out.contains("moved 2 requirements to members (docs/members.md, new document)"),
        "{out}"
    );
    assert!(
        out.contains("preservation check: 6 requirement hashes unchanged, 6 ids resolve, 1 accepted decisions unchanged"),
        "{out}"
    );
    assert_eq!(
        r.read("docs/members.md"),
        "---\nstatus: active\n---\n# Members\n\n## Membership\n\n- **member-registration**: When a person applies, the desk SHALL check an identity document. <!-- aliases: loans#member-registration -->\n- **membership-kinds**: The library SHALL have two membership kinds. <!-- aliases: loans#membership-kinds -->\n"
    );
    let loans = r.read("docs/loans.md");
    assert!(
        !loans.contains("## Membership"),
        "the empty section goes: {loans}"
    );
    assert!(
        loans.contains("See [membership](#membership)."),
        "a link to a section that stays is kept: {loans}"
    );
    assert!(
        r.read("docs/guide.md")
            .contains("[the identity](members.md#member-registration)"),
        "links follow the key"
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    let (_, out, _) = r.run(&["show", "loans#membership-kinds"]);
    assert!(out.starts_with("<!-- members#membership-kinds h:"), "{out}");
}

#[test]
fn mv_moves_sections_and_rewrites_links_but_not_decision_text() {
    let r = split_project();
    let before = r.read("docs/decisions/fines.md");
    let (code, out, err) = r.run(&["mv", "loans#fines", "loans#renewals", "policy", "--json"]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(out.contains("\"kind\":\"split\""), "{out}");
    assert!(out.contains("\"requirements\":3,\"sections\":2"), "{out}");
    assert!(
        out.contains("\"preserved\":{\"hashes\":6,\"ids\":9,\"decisions\":1}"),
        "{out}"
    );
    let policy = r.read("docs/policy.md");
    assert!(
        policy.contains("## Fines\n\nThe fine rules.\n\n- **daily-fine**: When a member returns a book late, the library SHALL charge 25 cents for each late day. <!-- aliases: loans#daily-fine -->\n\n### Fine cap\n"),
        "{policy}"
    );
    assert!(
        policy.contains("## Renewals\n\n- **renewal-limit**:"),
        "{policy}"
    );
    assert!(
        r.read("docs/loans.md")
            .contains("## Fines\n\nThis section moved to [Policy](policy.md#fines).\n"),
        "{}",
        r.read("docs/loans.md")
    );
    let decision = r.read("docs/decisions/fines.md");
    assert_eq!(
        decision,
        before.replace("../loans.md#", "../policy.md#"),
        "only the link destinations change"
    );
    assert!(
        r.read("docs/guide.md")
            .contains("[the fines](policy.md#fines)")
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "no SD405, no broken link: {out}");
}

#[test]
fn mv_refuses_bad_splits_and_changes_nothing() {
    let r = split_project();
    let (code, _, err) = r.run(&["mv", "loans#daily-fine", "decisions/fines"]);
    assert_eq!(code, 2, "a decision has no requirements: {err}");
    let (code, _, err) = r.run(&["mv", "loans#loans", "x"]);
    assert_eq!(code, 2, "the title section: {err}");
    assert!(err.contains("to move the whole document"), "{err}");
    r.write(
        "docs/policy.md",
        &r.read("docs/policy.md")
            .replace("**scope**", "**daily-fine**"),
    );
    let (code, _, err) = r.run(&["mv", "loans#fines", "policy"]);
    assert_eq!(code, 2, "{err}");
    assert!(
        err.contains("the requirement key `daily-fine` two times"),
        "{err}"
    );
    let (code, _, err) = r.run(&["mv", "loans#nothing", "policy"]);
    assert_eq!(code, 5, "{err}");
    let status = git(&r.dir, &["status", "--porcelain"]);
    assert_eq!(status, " M docs/policy.md\n", "only the edit of the test");
}

#[test]
fn mv_keeps_the_hash_of_a_requirement_whose_link_it_rewrites() {
    // The links to a moved key are in requirements: the move rewrites them,
    // and the preservation check passes (defect D1 of 0.1.1).
    let r = split_project();
    r.write(
        "docs/policy.md",
        "---\nstatus: active\n---\n# Policy\n\n## Scope\n\n- **scope**: The library SHALL apply [the fine](loans.md#daily-fine) to all branches.\n",
    );
    r.append(
        "docs/loans.md",
        "- **renewal-fine**: The library SHALL charge [the daily fine](#daily-fine) for a late renewal.\n",
    );
    r.commit("links in requirements");
    let (code, out, err) = r.run(&["mv", "loans#daily-fine", "loans#fine-cap", "fines"]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(
        r.read("docs/policy.md")
            .contains("[the fine](fines.md#daily-fine)"),
        "{}",
        r.read("docs/policy.md")
    );
    assert!(
        r.read("docs/loans.md")
            .contains("[the daily fine](fines.md#daily-fine)")
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn mv_keeps_the_text_after_a_list_apart_from_the_moved_items() {
    // A paragraph follows the last requirement after a blank line, in the
    // source and in the destination. The moved items must not take it as a
    // lazy continuation line (the cause of defect D1 in a playground task).
    let r = Repo::empty();
    r.run(&["init"]);
    r.write(
        "docs/web.md",
        "# Web\n\n## Requirements\n\n- **web-home**: The web app SHALL list the events.\n- **web-locale**: The web app SHALL serve two locales.\n\nThe locale rules are in one place.\n\n## Sign-in\n\nText.\n",
    );
    r.write(
        "docs/i18n.md",
        "# Languages\n\n## Requirements\n\n- **locales**: The system SHALL support two locales.\n\nThe web app has the same rule.\n\n## Notes\n\nText.\n",
    );
    r.commit("docs");
    let (code, out, err) = r.run(&["mv", "web#web-locale", "i18n"]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(
        r.read("docs/i18n.md").contains(
            "- **locales**: The system SHALL support two locales.\n- **web-locale**: The web app SHALL serve two locales. <!-- aliases: web#web-locale -->\n\nThe web app has the same rule.\n"
        ),
        "{}",
        r.read("docs/i18n.md")
    );
    assert!(
        r.read("docs/web.md").contains(
            "- **web-home**: The web app SHALL list the events.\n\nThe locale rules are in one place.\n"
        ),
        "{}",
        r.read("docs/web.md")
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn mv_records_aliases_only_for_ids_of_the_base_tree() {
    // Keys that are new in the change get no alias (defect D4 of 0.1.1).
    let r = split_project();
    r.write(
        "docs/loans.md",
        &r.read("docs/loans.md").replace(
            "## Renewals\n",
            "## Holds\n\n- **hold-limit**: The library SHALL allow 3 holds for each member.\n- **hold-expiry**: The library SHALL release a hold after 7 days.\n\n## Renewals\n",
        ),
    );
    let (code, out, err) = r.run(&["mv", "loans#holds", "holds"]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(
        out.contains("no alias, because the keys are new in this change"),
        "{out}"
    );
    assert!(
        !r.read("docs/holds.md").contains("aliases"),
        "{}",
        r.read("docs/holds.md")
    );
    // A mix: one key of the base, one new key.
    r.write(
        "docs/loans.md",
        &r.read("docs/loans.md").replace(
            "- **renewal-limit**: The library SHALL allow 2 renewals of a loan.\n",
            "- **renewal-limit**: The library SHALL allow 2 renewals of a loan.\n- **renewal-notice**: The library SHALL send a notice before a loan ends.\n",
        ),
    );
    let (code, out, err) = r.run(&["mv", "loans#renewal-limit", "loans#renewal-notice", "holds"]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(
        out.contains("1 old ids stay as aliases; 1 keys are new in this change and get no alias"),
        "{out}"
    );
    let holds = r.read("docs/holds.md");
    assert!(
        holds.contains("renewals of a loan. <!-- aliases: loans#renewal-limit -->"),
        "{holds}"
    );
    assert!(
        holds.contains("before a loan ends.\n") && !holds.contains("loans#renewal-notice"),
        "{holds}"
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    let (code, out, _) = r.run(&["show", "loans#renewal-limit"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn install_source_goes_into_the_pin_message() {
    let r = Repo::empty();
    r.run(&["init"]);
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains("# install-source = "),
        "init documents the key: {cfg}"
    );
    r.write(
        "spec-dna.toml",
        &format!(
            "install-source = \"file:///srv/git/spec-dna\"\n{}",
            cfg.replace(
                &format!("spec-dna = \"{}\"", env!("CARGO_PKG_VERSION")),
                "spec-dna = \"9.9.9\""
            )
        ),
    );
    let (code, _, err) = r.run(&["check"]);
    assert_eq!(code, 3, "{err}");
    assert!(
        err.contains(
            "cargo install --locked --force --git file:///srv/git/spec-dna --tag v9.9.9 spec-dna"
        ),
        "{err}"
    );
}

#[test]
fn drift_severity_follows_the_root() {
    for (severity, code, expect) in [("error", 1, Some("error SD607")), ("off", 0, None)] {
        let r = code_project();
        r.append(
            "spec-dna.toml",
            &format!("drift-severity = \"{severity}\"\n"),
        );
        r.commit("severity");
        r.write(
            "src/library/loans.py",
            &LOANS_PY.replace("MAX_RENEWALS = 1\n", ""),
        );
        let (c, out, _) = r.run(&["check"]);
        assert_eq!(c, code, "{severity}: {out}");
        match expect {
            Some(e) => assert!(out.contains(e), "{out}"),
            None => assert!(!out.contains("SD607"), "{out}"),
        }
    }
}

#[test]
fn an_amendment_that_names_the_new_code_clears_drift_of_a_decision() {
    // The frozen text of an accepted decision cites code that the change
    // moves and renames. The amendment is the only valid fix (defect D3 of
    // 0.1.1).
    let r = code_project();
    let adr = "docs/decisions/lending.md";
    r.write(
        adr,
        "---\ntype: decision\nstatus: accepted\n---\n# Lending\n\n## Decision Outcome\n\nThe loan code is in `src/library/loans.py`, and `checkoutBook()` records a loan.\n",
    );
    r.commit("decision");
    let base = git(&r.dir, &["rev-parse", "HEAD"]).trim().to_string();
    git(
        &r.dir,
        &["mv", "src/library/loans.py", "src/library/lending.py"],
    );
    r.write(
        "src/library/lending.py",
        &LOANS_PY
            .replace("checkoutBook", "lend_book")
            .replace("return loan", "return (loan)"),
    );
    r.write(
        "docs/notes/idea.md",
        &r.read("docs/notes/idea.md")
            .replace("loans.py", "lending.py"),
    );
    let warned = |out: &str| {
        out.lines()
            .filter(|l| l.starts_with(adr) && l.contains("SD60"))
            .count()
    };
    let (_, out, _) = r.run(&["check"]);
    assert_eq!(warned(&out), 2, "SD606 and SD607 on the frozen text: {out}");
    // An amendment that names no new code does not clear the drift.
    let frozen = r.read(adr);
    r.write(
        adr,
        &format!("{frozen}\n## Amendments\n\n- The loan code moved.\n"),
    );
    let (_, out, _) = r.run(&["check"]);
    assert_eq!(warned(&out), 2, "{out}");
    // An amendment that names the new path and the new identifier clears it.
    r.write(
        adr,
        &format!("{frozen}\n## Amendments\n\n- The loan code moved to `src/library/lending.py`; `lend_book()` records a loan.\n"),
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(warned(&out), 0, "{out}");
    assert!(!out.contains("SD405"), "the frozen text is the same: {out}");
    // The documents that are not frozen still warn.
    assert!(out.contains("docs/books.md:9: warning SD607"), "{out}");
    // The same in CI mode, against the base.
    r.commit("amend");
    let (code, out, _) = r.run(&["check", "--base", &base]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(warned(&out), 0, "{out}");
    // A citation of removed code in the amendment itself still warns.
    r.write(
        adr,
        &format!("{}- See also `src/library/fees_old.py`.\n", r.read(adr)),
    );
    git(
        &r.dir,
        &["mv", "src/library/fees.py", "src/library/fees_old.py"],
    );
    r.commit("fees");
    git(&r.dir, &["rm", "-q", "src/library/fees_old.py"]);
    let (_, out, _) = r.run(&["check"]);
    assert!(
        out.contains("warning SD606 cited-path-removed: `src/library/fees_old.py`"),
        "{out}"
    );
}

#[test]
fn check_lists_errors_and_drift_before_the_prose_lint() {
    // Many lint warnings must not hide an error or a drift warning (defect
    // D5 of 0.1.1).
    let r = code_project();
    let mut rules = String::from("# Imported\n\n");
    for i in 1..=12 {
        rules.push_str(&format!(
            "- **rule-{i}**: The refund of ticket {i} SHALL be approved by the desk.\n"
        ));
    }
    rules.push_str("\nSee [the terms](terms.md).\n");
    r.write("docs/imported.md", &rules);
    r.write(
        "spec-dna.toml",
        &format!("{}\n[rules]\nlint = \"warning\"\n", r.read("spec-dna.toml")),
    );
    // A configuration change is a change of its own (SD503).
    git(&r.dir, &["add", "spec-dna.toml"]);
    git(&r.dir, &["commit", "-q", "-m", "lint on"]);
    r.write(
        "src/library/loans.py",
        &LOANS_PY.replace("MAX_RENEWALS = 1\n", ""),
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 6, "{out}");
    assert!(lines[0].contains("error SD102"), "{out}");
    assert!(lines[1].contains("warning SD607"), "{out}");
    assert_eq!(
        lines[2],
        "warning SD805 passive-voice: 12 in docs/imported.md (`check --lint` lists them)"
    );
    // The fix of each listed code follows, once.
    assert!(
        lines[3].starts_with("SD102 fix: Fix the link target."),
        "{out}"
    );
    assert!(lines[4].starts_with("SD607 fix: "), "{out}");
    assert!(lines[5].starts_with("spec-dna check: "), "{out}");
    assert!(
        lines[5].contains("13 new warnings"),
        "the count is the same: {out}"
    );
    let (_, out, _) = r.run(&["check", "--lint"]);
    assert_eq!(
        out.matches("warning SD805 passive-voice: `rule-").count(),
        12,
        "{out}"
    );
    let (_, json, _) = r.run(&["check", "--json"]);
    assert_eq!(json.matches("\"code\":\"SD805\"").count(), 12, "{json}");
}

#[test]
fn a_bold_slug_without_a_colon_warns() {
    let r = Repo::empty();
    r.run(&["init"]);
    r.write(
        "docs/rules.md",
        "# Rules\n\n- **member-registration** [1.1]: When a person applies, the desk SHALL check an identity document.\n- **note** this item is prose.\n",
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "a warning: {out}");
    assert!(
        out.contains(
            "docs/rules.md:3: warning SD807 requirement-colon-missing: `**member-registration**`"
        ),
        "{out}"
    );
    assert!(!out.contains("`**note**`"), "{out}");
}

/// A repository with one mixed root `docs` and the given profile line.
fn profile_repo(profile: &str) -> Repo {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        &format!(
            "spec-dna = \"0.1.0\"\n{profile}\n[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nstatus = \"active\"\n"
        ),
    );
    r.write(
        "docs/loans.md",
        "# Loans\n\n## Requirements\n\n- **loan-period**: When a member borrows a book, the library SHALL set the due date 14 days later.\n- **renew-limit**: The library SHALL allow at most 1 renewal. <!-- verified-by: tests/renew.py -->\n- **fines**: the fine is collected daily.\n",
    );
    r.write("tests/renew.py", "def test_renew():\n    pass\n");
    r.write(
        "tests/test_loans.py",
        "# covers: loans#loan-period\ndef test_period():\n    pass\n",
    );
    r.commit("start");
    r
}

#[test]
fn profiles_set_the_severity_of_each_rule_group() {
    // standard (the default): a broken link is an error, a requirement
    // without evidence is a warning, and there is no lint.
    let r = profile_repo("");
    r.append("docs/loans.md", "\nSee [the terms](terms.md).\n");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("error SD102"), "{out}");
    assert!(
        out.contains("docs/loans.md:7: warning SD608 requirement-unverified: `loans#fines` is active but no test covers it"),
        "{out}"
    );
    assert!(
        !out.contains("SD608 requirement-unverified: `loans#loan-period`"),
        "{out}"
    );
    assert!(!out.contains("`loans#renew-limit` is active"), "{out}");
    assert!(!out.contains("SD80"), "{out}");

    // lite: the broken link is a warning; evidence and lint are off.
    let r = profile_repo("profile = \"lite\"");
    r.append("docs/loans.md", "\nSee [the terms](terms.md).\n");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("warning SD102"), "{out}");
    assert!(!out.contains("SD608"), "{out}");

    // strict: a requirement without evidence is an error, and the lint gives
    // warnings. A rule of a root or of the project wins over the profile.
    let r = profile_repo("profile = \"strict\"");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("error SD608"), "{out}");
    assert!(
        out.contains("warning SD801 requirement-keyword: 1 in"),
        "{out}"
    );
    let r = profile_repo("profile = \"strict\"\n[rules]\nevidence = \"warning\"\nlint = \"off\"");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("warning SD608"), "{out}");
    assert!(!out.contains("SD801"), "{out}");
}

#[test]
fn evidence_resolves_covers_tags_and_verified_by() {
    let r = profile_repo("");
    // A tag in any comment syntax, and a tag that names no requirement.
    r.write(
        "tests/fines.test.ts",
        "/* covers: loans#fines */\ntest('x', () => {}) // covers: loans#fine, loans#loan-period\n",
    );
    r.write(
        "src/app.py",
        "# covers: loans#nothing (not a test file, so not a tag)\n",
    );
    r.append(
        "docs/loans.md",
        "- **holds**: The library SHALL keep a hold for 3 days. <!-- verified-by: tests/holds.py -->\n",
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("tests/fines.test.ts:2: error SD105 covers-unresolved: `covers:` tag: `loans` has no requirement `fine`"),
        "{out}"
    );
    assert!(
        out.contains("docs/loans.md:8: error SD103 evidence-missing: `holds`: `verified-by` path `tests/holds.py` is not in the git tree"),
        "{out}"
    );
    assert!(!out.contains("SD608"), "{out}");
    assert!(!out.contains("src/app.py"), "{out}");
    // `show` lists the evidence of a requirement and of a document.
    let (code, out, _) = r.run(&["show", "loans#loan-period"]);
    assert_eq!(code, 0);
    assert!(
        out.contains(
            "<!-- evidence -->\n- loans#loan-period: tests/fines.test.ts:2, tests/test_loans.py:1"
        ),
        "{out}"
    );
    let (_, out, _) = r.run(&["show", "loans"]);
    assert!(
        out.contains("- loans#renew-limit: tests/renew.py\n"),
        "{out}"
    );
    assert!(
        out.contains("- loans#fines: tests/fines.test.ts:1\n"),
        "{out}"
    );
    // The evidence check is base-relative: an old requirement without
    // evidence is not new.
    r.write("tests/fines.test.ts", "test('x', () => {})\n");
    r.write("tests/holds.py", "def test_holds():\n    pass\n");
    r.commit("tests");
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(
        out.contains("warning SD608 requirement-unverified: `loans#fines`"),
        "{out}"
    );
    // A moved requirement keeps its tag through the alias.
    let (code, out, _) = r.run(&["mv", "loans#loan-period", "periods"]);
    assert_eq!(code, 0, "{out}");
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("SD105"), "{out}");
}

#[test]
fn knowledge_types_have_templates_sections_and_context_rules() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nstatus = \"draft\"\n[[roots]]\npath = \"docs/decisions\"\ntype = \"decision\"\n",
    );
    r.write("src/pay/charge.py", "def charge_card():\n    pass\n");
    r.write(
        "docs/pay.md",
        "# Pay\n\n## Requirements\n\n- **charge**: The service SHALL charge the card once. See `src/pay/charge.py`.\n",
    );
    r.commit("start");
    // `new` writes each type into the right root, from its template.
    for (ty, id, path) in [
        ("invariant", "rules/money", "docs/rules/money.md"),
        ("runbook", "ops/restart", "docs/ops/restart.md"),
        ("incident", "ops/double-charge", "docs/ops/double-charge.md"),
        ("reference", "research/psp", "docs/research/psp.md"),
        ("decision", "psp-choice", "docs/decisions/psp-choice.md"),
    ] {
        let (code, out, err) = r.run(&["new", ty, id]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.starts_with(&format!("wrote {path} ({ty} template)")),
            "{out}"
        );
    }
    let (code, _, err) = r.run(&["new", "spec", "decisions/x"]);
    assert_eq!(code, 2, "{err}");
    let (code, _, err) = r.run(&["new", "invariant", "rules/money"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("exists"), "{err}");
    let inv = r.read("docs/rules/money.md");
    assert!(
        inv.starts_with(
            "---\ntype: invariant\nstatus: draft\napplies-to: [\"**\"]\n---\n# Money\n\n## Rules\n"
        ),
        "{inv}"
    );
    // The placeholders of a template stay errors until an author fills them.
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("docs/ops/double-charge.md"), "{out}");
    // Fill the documents.
    r.write(
        "docs/rules/money.md",
        "---\ntype: invariant\nstatus: active\napplies-to: [\"src/pay/**\"]\n---\n# Money\n\n## Rules\n\n- **minor-units**: The system SHALL store amounts in minor units.\n",
    );
    r.write(
        "docs/ops/restart.md",
        "---\ntype: runbook\nstatus: active\n---\n# Restart\n\n## When to use\n\nThe queue stops.\n",
    );
    r.write(
        "docs/ops/double-charge.md",
        "---\ntype: incident\n---\n# Double charge\n\n## Summary\n\nTwo charges for one order.\n\n## Root cause\n\nA retry without a key.\n",
    );
    r.write(
        "docs/research/psp.md",
        "---\ntype: reference\n---\n# PSP research\n\nThe notes cite `src/pay/charge.py` many times.\n",
    );
    r.write(
        "docs/decisions/psp-choice.md",
        "---\nstatus: accepted\n---\n# PSP choice\n\n## Decision Outcome\n\nOne PSP.\n",
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("docs/ops/restart.md:1: error SD305 section-missing: a runbook needs a section headed `Steps`"),
        "{out}"
    );
    assert!(
        out.contains("docs/ops/double-charge.md:1: warning SD609 incident-unlinked"),
        "{out}"
    );
    r.append(
        "docs/ops/restart.md",
        "\n## Steps\n\n1. Restart the worker.\n",
    );
    r.append(
        "docs/ops/double-charge.md",
        "\n## Links\n\n- [[pay#charge]], [[psp-choice]], [[ops/restart]]\n",
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("SD609"), "{out}");
    // `context` serves the matching invariant first, lists the incident, and
    // never serves the reference.
    let (code, out, _) = r.run(&["context", "src/pay/charge.py"]);
    assert_eq!(code, 0, "{out}");
    let first = out.lines().filter(|l| l.starts_with("<!-- ")).nth(1);
    assert!(
        first.is_some_and(|l| l.starts_with("<!-- rules/money")),
        "{out}"
    );
    assert!(out.contains("invariant/active invariant -->"), "{out}");
    assert!(!out.contains("research/psp"), "{out}");
    let (_, out, _) = r.run(&["context", "pay", "--path", "src/pay/charge.py"]);
    assert!(out.contains("rules/money"), "{out}");
    let (_, out, _) = r.run(&["context", "research/psp"]);
    assert!(out.contains("The notes cite"), "{out}");
    // An invariant out of scope is not served.
    r.write("src/web/page.py", "def page():\n    pass\n");
    let (_, out, _) = r.run(&["context", "src/web/page.py"]);
    assert!(!out.contains("rules/money"), "{out}");
}

#[test]
fn init_answers_file_on_a_new_project() {
    let r = Repo::empty();
    let answers = r.dir.join("../answers-new.toml");
    fs::write(
        &answers,
        "profile = \"strict\"\ntypes = [\"spec\", \"decision\", \"runbook\"]\nevidence = \"warning\"\nadapters = [\"skill\"]\nci = \"github\"\nhook = \"none\"\nbase = \"trunk\"\n",
    )
    .unwrap();
    let (code, out, err) = r.run(&["init", "--answers", answers.to_str().unwrap()]);
    let _ = fs::remove_file(&answers);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    for want in [
        "profile = \"strict\"",
        "types = [\"spec\", \"decision\", \"runbook\"]",
        "[rules]  # overrides of the profile, by rule group or by code\nevidence = \"warning\"\n",
        "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nstatus = \"draft\"\n",
    ] {
        assert!(cfg.contains(want), "{want}:\n{cfg}");
    }
    assert!(!r.dir.join("AGENTS.md").exists());
    assert!(!r.dir.join("CLAUDE.md").exists());
    assert!(r.dir.join(".claude/skills/spec-dna/SKILL.md").exists());
    let ci = r.read(".github/workflows/spec-dna.yml");
    assert!(
        ci.contains("spec-dna check --base \"origin/${GITHUB_BASE_REF:-trunk}\""),
        "{ci}"
    );
    assert!(out.contains("to commit: git add spec-dna.toml .claude/skills/spec-dna/SKILL.md .github/workflows/spec-dna.yml"), "{out}");
    // `new` follows the types: a runbook goes into its namespace, a
    // contract is not a type of this project.
    let (code, out, _) = r.run(&["new", "runbook", "restart"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.starts_with("wrote docs/runbooks/restart.md"), "{out}");
    let (code, _, err) = r.run(&["new", "contract", "api"]);
    assert_eq!(code, 2, "{err}");
    r.write(
        "docs/api.md",
        "---\ntype: contract\nstatus: draft\n---\n# Api\n",
    );
    let (_, out, _) = r.run(&["check"]);
    assert!(
        out.contains("error SD303 type-invalid: type `contract` is not a type of this project"),
        "{out}"
    );
    // A second `init` updates the adapters only.
    let (code, out, _) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("spec-dna.toml"), "{out}");
}

#[test]
fn init_detects_a_brownfield_repository() {
    let r = Repo::fixture("brownfield");
    let (code, out, err) = r.run(&["init", "--dry-run", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    for want in [
        "[evidence]\ntests = [\"**/tests/**\"]",
        "path = \"docs\"\ntype = \"mixed\"\ndefault-type = \"guide\"\nstatus = \"active\"\nstatus-map = { current = \"active\" }\n",
        "path = \"docs/adr\"\ntype = \"decision\"\nstatus = \"proposed\"\nexclude = [\"ADR-*-implementation-plan.md\", \"README.md\"]\nlegacy-ids = [\"ADR-{n}\"]\nrequired-sections = { decision = [\"Решение\", \"Decision Outcome\", \"Decision\"] }\n",
        "path = \"docs/guides\"\ntype = \"guide\"\nstatus = \"active\"\nexclude = [\"ext-*/**\"]\n",
        "path = \"docs/specs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nstatus = \"active\"\nlegacy-keys = [\"R-{n}*\"]\nstatus-map = { shipped = \"active\" }\n",
        "  change AGENTS.md (one generated line)",
        "  change .pre-commit-config.yaml (a local hook at the end: `spec-dna check`)",
        "dry run: nothing written",
    ] {
        assert!(out.contains(want), "{want}:\n{out}");
    }
    assert!(!out.contains("path = \"docs/specs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nstatus = \"active\"\nlegacy-keys = [\"R-{n}*\"]\nstatus-map = { shipped = \"active\" }\nrequired-sections"), "{out}");
    assert!(!r.dir.join("spec-dna.toml").exists());
    let (code, out, _) = r.run(&["init", "--dry-run", "--non-interactive", "--json"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"kind\":\"brownfield\""), "{out}");
    assert!(out.contains("\"stack\":[\"Rust\"]"), "{out}");
    assert!(out.contains("\"ci\":[\"GitHub Actions\"]"), "{out}");
    assert!(out.contains("\"hooks\":[\"pre-commit\"]"), "{out}");
    assert!(out.contains("\"written\":[]"), "{out}");

    // An answers file: drop a root, relax the budgets, no CI file.
    let answers = r.dir.join("../answers-brown.toml");
    fs::write(
        &answers,
        "mapping = \"accept\"\ndrop = [\"docs/guides\"]\nbudgets = \"relaxed\"\nci = \"none\"\n",
    )
    .unwrap();
    let (code, out, err) = r.run(&["init", "--answers", answers.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(!cfg.contains("docs/guides"), "{cfg}");
    assert!(
        cfg.contains("[budgets.spec]\nsoft = 20000\nhard = 40000\n"),
        "{cfg}"
    );
    assert!(!r.dir.join(".github/workflows/spec-dna.yml").exists());
    assert!(
        r.read(".pre-commit-config.yaml")
            .ends_with("  - repo: local\n    hooks:\n      - id: spec-dna\n        name: spec-dna check\n        entry: spec-dna check\n        language: system\n        pass_filenames: false\n        always_run: true\n")
    );
    assert!(
        r.read("AGENTS.md")
            .starts_with("# Agents\n\nRun the tests before a commit.\n")
    );
    // The documents did not change, and `check` finds no error in them.
    assert_eq!(git(&r.dir, &["status", "--porcelain", "--", "docs"]), "");
    r.commit("adopt");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("SD305"), "{out}");
    assert!(
        out.contains("4 documents") || out.contains("documents;"),
        "{out}"
    );

    // --reconfigure: the current values are the defaults, and the keys that
    // no question asks for stay.
    r.write(
        "spec-dna.toml",
        &r.read("spec-dna.toml").replace(
            "# install-source = \"https://github.com/AlexGavrilov939/spec-dna\"  # a local or private clone\n",
            "install-source = \"file:///srv/git/spec-dna\"\n\n[context]\nbudget = 9000\n",
        ),
    );
    fs::write(&answers, "profile = \"lite\"\n").unwrap();
    let (code, out, err) = r.run(&[
        "init",
        "--reconfigure",
        "--answers",
        answers.to_str().unwrap(),
    ]);
    let _ = fs::remove_file(&answers);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(cfg.contains("profile = \"lite\""), "{cfg}");
    assert!(
        cfg.contains("install-source = \"file:///srv/git/spec-dna\""),
        "{cfg}"
    );
    assert!(cfg.contains("[context]\nbudget = 9000\n"), "{cfg}");
    assert!(
        cfg.contains("path = \"docs/adr\"\ntype = \"decision\""),
        "{cfg}"
    );
    assert!(cfg.contains("legacy-ids = [\"ADR-{n}\"]"), "{cfg}");
    assert!(
        cfg.contains("[budgets.spec]\nsoft = 20000\nhard = 40000\n"),
        "{cfg}"
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn reconfigure_keeps_the_section_keys_and_the_conventions() {
    // The keys of 0.3 that no question asks for: the history, companion and
    // live sections of a root, and `[conventions]`.
    let r = Repo::empty();
    let cfg = "spec-dna = \"0.1.0\"\nprofile = \"standard\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nstatus = \"active\"\nhistory-sections = { spec = [\"Changelog\"] }\ncompanion-sections = { spec = [\"Acceptance map\"] }\n\n[[roots]]\npath = \"docs/adr\"\ntype = \"decision\"\nlive-sections = { decision = [\"Implementation status\"] }\n\n[conventions]\ndecision = [\"CONTRIBUTING.md#decisions\"]\nspec = [\"docs/guide.md#specs\", \"docs/guide.md#more\"]\n";
    r.write("spec-dna.toml", cfg);
    r.write(
        "docs/guide.md",
        "---\ntype: guide\nstatus: active\n---\n# Guide\n\n## Specs\n\nOne rule.\n\n## More\n\nAnother rule.\n",
    );
    r.write(
        "CONTRIBUTING.md",
        "# Contributing\n\n## Decisions\n\nAmend.\n",
    );
    r.commit("docs");
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let new = r.read("spec-dna.toml");
    for want in [
        "history-sections = { spec = [\"Changelog\"] }",
        "companion-sections = { spec = [\"Acceptance map\"] }",
        "live-sections = { decision = [\"Implementation status\"] }",
        "[conventions]",
        "decision = [\"CONTRIBUTING.md#decisions\"]",
        "spec = [\"docs/guide.md#specs\", \"docs/guide.md#more\"]",
    ] {
        assert!(new.contains(want), "{want}\n{new}");
    }
    let (_, out, _) = r.run(&["show", "--conventions", "decision"]);
    assert!(
        out.contains("<!-- decision: amendment sections: Amendments; live sections: Implementation status -->")
            && out.contains("Amend."),
        "{out}"
    );
    r.commit("reconfigure");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn init_detects_common_layouts() {
    let r = Repo::fixture("layouts");
    let (code, out, err) = r.run(&["init", "--dry-run", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    for want in [
        "path = \"doc/adr\"\ntype = \"decision\"\nstatus = \"proposed\"\nlegacy-ids = [{ alias = \"ADR-{n}\", file = \"{n}-*\" }]\n",
        "path = \"rfcs\"\ntype = \"change\"\nstatus = \"accepted\"\n",
        "path = \"runbooks\"\ntype = \"runbook\"\n",
        "path = \"incidents\"\ntype = \"incident\"\n",
        "path = \"postmortems\"\ntype = \"incident\"\n",
        "path = \"validation\"\ntype = \"reference\"\n",
        "path = \"research\"\ntype = \"reference\"\n",
        // A numbered directory in a mixed root takes the type of its files.
        "path = \"docs/records\"\ntype = \"incident\"\nlegacy-ids = [\"BUG-{n}\"]\n",
    ] {
        assert!(out.contains(want), "{want}:\n{out}");
    }
    let answers = r.dir.join("../answers-layouts.toml");
    fs::write(&answers, "mapping = \"accept\"\nci = \"none\"\n").unwrap();
    let (code, out, err) = r.run(&["init", "--answers", answers.to_str().unwrap()]);
    let _ = fs::remove_file(&answers);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("adopt");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("0 errors"), "{out}");
    // The adr-tools numbers give the aliases.
    let (code, out, _) = r.run(&["show", "ADR-2"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("# 2. Use Markdown"), "{out}");
}

/// Runs `spec-dna init` on a pseudo-terminal, writes the answers, and returns
/// the exit code and the output.
#[cfg(unix)]
fn on_terminal(dir: &Path, args: &[&str], answers: &str) -> (i32, String) {
    use std::io::{Read, Write};
    use std::os::fd::{FromRawFd, OwnedFd};
    use std::process::Stdio;
    let (mut master, mut slave) = (0, 0);
    // SAFETY: openpty writes two new file descriptors; the null pointers ask
    // for no name and the default terminal settings.
    let rc = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            // `*mut` also coerces to the `*const` arguments of Linux.
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(rc, 0, "openpty");
    // SAFETY: the descriptors are new and owned here only.
    let (master, slave) = unsafe {
        (
            std::fs::File::from_raw_fd(master),
            OwnedFd::from_raw_fd(slave),
        )
    };
    let mut child = {
        let mut cmd = Command::new(BIN);
        cmd.args(args)
            .current_dir(dir)
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave));
        cmd.spawn().expect("binary runs")
    };
    let mut reader = master.try_clone().unwrap();
    let out = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        // The read ends with EOF or EIO when the child closes the terminal.
        while let Ok(n) = reader.read(&mut chunk) {
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
        }
        buf
    });
    let mut writer = master;
    for line in answers.lines() {
        std::thread::sleep(std::time::Duration::from_millis(30));
        writer.write_all(format!("{line}\n").as_bytes()).unwrap();
    }
    let start = std::time::Instant::now();
    let code = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s.code().unwrap_or(-1);
        }
        if start.elapsed() > std::time::Duration::from_secs(30) {
            let _ = child.kill();
            panic!("init waited for more input");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    drop(writer);
    let text = String::from_utf8_lossy(&out.join().unwrap()).replace("\r\n", "\n");
    (code, text)
}

#[cfg(unix)]
#[test]
fn init_asks_on_a_terminal() {
    let r = Repo::fixture("brownfield");
    // 1 profile, 2 types, 3 evidence, 4 language, 5 mapping (4 roots), 6
    // budgets, 7 adapters, 8 CI, hook and base, then the confirmation.
    let answers = "strict\n\nerror, covers\n\n\ndrop\n\ntype=spec\n\noff\nclaude-md, skill\nnone, none, main\ny\n";
    let (code, out) = on_terminal(&r.dir, &["init"], answers);
    assert_eq!(code, 0, "{out}");
    for want in [
        "1/8 Profile (lite, standard, strict) [standard]: ",
        "    Types in use: spec, guide, decision, note. A shorter list makes each document of another type an error.",
        "2/8 Knowledge types [spec, contract, guide, decision, change, note, invariant, runbook, incident, reference]: ",
        "5/8 Mapping: 4 roots.",
        "    docs/adr (decision; 3 documents; status proposed; ids ADR-{n}; exclude ADR-*-implementation-plan.md, README.md) [accept]: ",
        "6/8 Budget level (default, relaxed, tight, off) [default]: ",
        "8/8 CI step (github, gitlab, none), hook (pre-commit, husky, lefthook, none) and base branch [github, pre-commit, main]: ",
        "--- spec-dna.toml ---",
        "Write these files? [y/N]: ",
        "wrote spec-dna.toml",
    ] {
        assert!(out.contains(want), "{want}:\n{out}");
    }
    let cfg = r.read("spec-dna.toml");
    assert!(cfg.contains("profile = \"strict\""), "{cfg}");
    assert!(
        cfg.contains("evidence = \"error\"\nbudgets = \"off\"\n"),
        "{cfg}"
    );
    assert!(!cfg.contains("path = \"docs/adr\""), "{cfg}");
    assert!(
        cfg.contains("path = \"docs/specs\"\ntype = \"spec\""),
        "{cfg}"
    );
    assert!(
        cfg.contains("path = \"docs/guides\"\ntype = \"guide\""),
        "{cfg}"
    );
    assert!(r.read("CLAUDE.md").contains("spec-dna check"));
    assert!(!r.read("AGENTS.md").contains("spec-dna"));
    assert!(!r.read(".pre-commit-config.yaml").contains("spec-dna"));

    // A "no" at the confirmation writes nothing.
    let r = Repo::empty();
    let (code, out) = on_terminal(&r.dir, &["init"], "\n\n\n\n\n\n\n\nn\n");
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("nothing written"), "{out}");
    assert!(!r.dir.join("spec-dna.toml").exists());
}

#[test]
fn review_fixes_on_the_command_line() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[evidence]\ntests = [\"/tests/**\"]\n[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nstatus = \"draft\"\n",
    );
    r.write(
        "docs/rules/money.md",
        "---\ntype: invariant\nstatus: active\n---\n# Money\n\n## Rules\n\n- **minor-units**: The system SHALL store amounts in minor units.\n",
    );
    r.write("tests/money.py", "# covers: rules/money#minor-units\n");
    r.commit("start");
    // A test glob with a leading slash works (git rejects it as a pathspec).
    let (code, out, err) = r.run(&["show", "rules/money"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("- rules/money#minor-units: tests/money.py:1"),
        "{out}"
    );
    // An invariant cannot move back to draft.
    r.write(
        "docs/rules/money.md",
        &r.read("docs/rules/money.md")
            .replace("status: active", "status: draft"),
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("error SD401 status-transition"), "{out}");
    git(&r.dir, &["checkout", "-q", "--", "."]);
    // A placeholder of a template that stays is an error.
    let (code, _, _) = r.run(&["new", "spec", "loans"]);
    assert_eq!(code, 0);
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("docs/loans.md:9: error SD306 placeholder-left: the template placeholder `{{trigger}}` is still in the document"),
        "{out}"
    );
    // init does not replace a workflow of the project, and `write = false`
    // shows the preview.
    r.write(".github/workflows/spec-dna.yml", "name: Own\n");
    let answers = r.dir.join("../answers-review.toml");
    fs::write(&answers, "ci = \"github\"\nwrite = false\n").unwrap();
    let (code, out, err) = r.run(&[
        "init",
        "--reconfigure",
        "--answers",
        answers.to_str().unwrap(),
    ]);
    let _ = fs::remove_file(&answers);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("--- spec-dna.toml ---"), "{out}");
    assert!(
        out.contains(".github/workflows/spec-dna.yml exists and differs"),
        "{out}"
    );
    assert!(out.contains("dry run: nothing written"), "{out}");
    assert_eq!(r.read(".github/workflows/spec-dna.yml"), "name: Own\n");
}

/// A project for the hygiene functions of 0.3.0: a mixed root with history
/// and companion sections, a decision root, two reference roots and a
/// registry of incidents.
fn hygiene_project() -> Repo {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nstatus = \"active\"\nlegacy-keys = [\"R-{n}\"]\nhistory-sections = { guide = [\"Iteration log\"] }\ncompanion-sections = { spec = [\"Acceptance map\"] }\n[[roots]]\npath = \"adr\"\ntype = \"decision\"\nlegacy-ids = [\"ADR-{n}\"]\nhistory-sections = { decision = [\"Review rounds\"] }\n[[roots]]\npath = \"refs/plans\"\ntype = \"reference\"\n[[roots]]\npath = \"refs/briefs\"\ntype = \"reference\"\n[[roots]]\npath = \"registry\"\ntype = \"incident\"\nlegacy-ids = [\"INC-{n}\"]\n",
    );
    r.write(
        "docs/billing.md",
        "# Billing\n\n## Charges\n\n- **R-1**: The desk SHALL charge 3 percent.\n- **R-2**: The desk SHALL send a receipt.\n\n## Refunds\n\n- **R-3**: The desk SHALL refund a payment within 5 days.\n\n## 4. Acceptance map\n\n| Key | Test |\n| --- | --- |\n| R-1 | test_charge |\n| R-2 | test_receipt |\n| R-3 | test_refund |\n",
    );
    r.write(
        "docs/guides/desk.md",
        "---\ntype: guide\nstatus: active\n---\n# Desk guide\n\n## Opening\n\nOpen the desk at 9.\n\n## Closing\n\nClose the desk at 5. See [refunds](../billing.md#refunds).\n\n### Cash count\n\nCount the cash.\n\n## Iteration log\n\n### Round 1\n\nWe tried 8.\n\n### Round 2\n\nWe tried 9.\n",
    );
    r.write(
        "docs/notes/links.md",
        "---\ntype: note\n---\n# Links\n\nSee [the count](../guides/desk.md#cash-count), [[guides/desk#closing]] and [the slow queue](../../registry/README.md#inc-2-slow-queue).\n",
    );
    r.write(
        "adr/ADR-3-cash.md",
        "---\nstatus: accepted\n---\n# Cash only\n\n## Decision Outcome\n\nThe desk takes cash only.\n\n## Review rounds\n\nRound 1 asked for cards.\n\n## Amendments\n\n### Amendment 1: coins\n\nThe desk SHALL accept coins too.\n\n### Amendment 2: wording\n\nA typo fix.\n",
    );
    r.write("refs/plans/rollout.md", "# Rollout plan\n\nA plan.\n");
    r.write("refs/plans/launch.md", "# Launch plan\n\nA plan.\n");
    r.write(
        "refs/briefs/rollout/README.md",
        "# Rollout brief\n\nA brief.\n",
    );
    for (n, title) in [(1, "Outage"), (2, "Slow queue")] {
        r.write(
            &format!("registry/INC-{n}.md"),
            &format!(
                "# {title}\n\n## Summary\n\nIt broke. See [[ADR-3]].\n\n## Root cause\n\nA bug.\n"
            ),
        );
    }
    r.write(
        "registry/README.md",
        "# Incidents\n\nThe register.\n\n## INC-1 Outage\n\nA long story.\n\n## INC-2 Slow queue\n\nAnother long story. See [[ADR-3]].\n\n## Summary\n\nTwo incidents.\n\n## Root cause\n\nMany.\n",
    );
    r.commit("hygiene");
    r
}

#[test]
fn mv_splits_sections_of_every_type_with_section_aliases() {
    let r = hygiene_project();
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{err}{out}");
    // A guide: the sections go to a new guide, and every old section id
    // resolves through an alias.
    let (code, out, err) = r.run(&["mv", "guides/desk#closing", "guides/closing"]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(
        out.starts_with(
            "moved 1 section to guides/closing (docs/guides/closing.md, guide, new document)"
        ),
        "{out}"
    );
    assert!(out.contains("2 section ids stay as aliases"), "{out}");
    let closing = r.read("docs/guides/closing.md");
    assert!(
        closing.starts_with("---\ntype: guide\nstatus: active\naliases:\n  - guides/desk#closing\n  - guides/desk#cash-count\n---\n# Closing\n"),
        "{closing}"
    );
    assert!(
        closing.contains("See [refunds](../billing.md#refunds)."),
        "{closing}"
    );
    let desk = r.read("docs/guides/desk.md");
    assert!(!desk.contains("Cash count"), "{desk}");
    assert!(
        desk.contains("## Closing\n\nThis section moved to [Closing](closing.md#closing).\n"),
        "the heading stays with a pointer: {desk}"
    );
    assert!(
        r.read("docs/notes/links.md")
            .contains("[the count](../guides/closing.md#cash-count), [[guides/desk#closing]]"),
        "a link follows the section; an id reference resolves by the alias"
    );
    let (code, out, _) = r.run(&["show", "guides/desk#cash-count"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("Count the cash."), "{out}");
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    // A document split out of a document over its hard budget holds no new
    // text: a warning (SD704), not a new-document error.
    git(&r.dir, &["checkout", "-q", "--", "."]);
    let _ = fs::remove_file(r.dir.join("docs/guides/closing.md"));
    r.append(
        "spec-dna.toml",
        "\n[budgets.guide]\nsoft = 50\nhard = 100\n",
    );
    git(&r.dir, &["commit", "-qam", "small guides"]);
    let (code, out, err) = r.run(&["mv", "guides/desk#closing", "guides/closing"]);
    assert_eq!(code, 0, "{err}{out}");
    git(&r.dir, &["add", "-A"]);
    let (code, out, _) = r.run(&["check", "--base", "HEAD"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("docs/guides/closing.md:1: warning SD704 budget-hard-kept: 115 characters, over the hard budget of 100 (split out of `guides/desk`, which was over the hard budget in the base)"),
        "{out}"
    );
    git(&r.dir, &["commit", "-qm", "split"]);
    // A guide section into an existing reference document: a pointer stays.
    let (code, out, err) = r.run(&["mv", "guides/desk#opening", "plans/rollout"]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(out.contains("left a one-line pointer"), "{out}");
    assert!(
        r.read("docs/guides/desk.md").contains(
            "## Opening\n\nThis section moved to [Rollout](../../refs/plans/rollout.md#opening).\n"
        ),
        "{}",
        r.read("docs/guides/desk.md")
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    // Requirements do not go into a document without requirements.
    let (code, _, err) = r.run(&["mv", "billing#charges", "guides/closing"]);
    assert_eq!(code, 2, "{err}");
    assert!(
        err.contains("requirements live in a spec, contract or invariant"),
        "{err}"
    );
}

#[test]
fn a_decision_keeps_its_frozen_text_and_moves_only_history() {
    let r = hygiene_project();
    let before = r.read("adr/ADR-3-cash.md");
    let (code, _, err) = r.run(&["mv", "ADR-3#decision-outcome", "docs/history/adr-3.md"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("is frozen text of the decision"), "{err}");
    let (code, _, err) = r.run(&["mv", "ADR-3#amendments", "billing"]);
    assert_eq!(code, 2, "{err}");
    assert!(
        err.contains("moves only into a `reference` document"),
        "{err}"
    );
    assert_eq!(
        r.read("adr/ADR-3-cash.md"),
        before,
        "a refusal changes nothing"
    );
    let (code, out, err) = r.run(&[
        "mv",
        "ADR-3#amendments",
        "ADR-3#review-rounds",
        "docs/history/adr-3.md",
    ]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(
        out.contains("candidates for a superseding decision (the moved amendments state a requirement, a BCP 14 keyword or a status word; spec-dna writes no decision): ADR-3-cash#amendment-1-coins"),
        "{out}"
    );
    let adr = r.read("adr/ADR-3-cash.md");
    assert!(
        adr.contains("## Decision Outcome\n\nThe desk takes cash only.\n"),
        "{adr}"
    );
    assert!(
        adr.contains(
            "## Amendments\n\nThis section moved to [Adr 3](../docs/history/adr-3.md#amendments).\n"
        ),
        "{adr}"
    );
    let history = r.read("docs/history/adr-3.md");
    assert!(
        history.starts_with("---\ntype: reference\naliases:\n"),
        "{history}"
    );
    assert!(
        history.contains("The history of [Cash only](../../adr/ADR-3-cash.md)."),
        "{history}"
    );
    assert!(
        history.contains("### Amendment 1: coins\n\nThe desk SHALL accept coins too."),
        "{history}"
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "the frozen text did not change: {out}");
    assert!(!out.contains("SD405"), "{out}");
    let (code, out, _) = r.run(&["show", "ADR-3-cash#amendment-2-wording"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn companion_rows_move_with_their_requirements() {
    let r = hygiene_project();
    let (code, out, err) = r.run(&["mv", "billing#charges", "payments"]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(
        out.contains("moved 2 companion rows with the requirements"),
        "{out}"
    );
    let payments = r.read("docs/payments.md");
    assert!(
        payments.contains("## 4. Acceptance map\n\n| Key | Test |\n| --- | --- |\n| R-1 | test_charge |\n| R-2 | test_receipt |\n"),
        "{payments}"
    );
    let billing = r.read("docs/billing.md");
    assert!(
        billing.contains("| Key | Test |\n| --- | --- |\n| R-3 | test_refund |\n")
            && !billing.contains("R-1"),
        "{billing}"
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    git(&r.dir, &["add", "-A"]);
    git(&r.dir, &["commit", "-q", "-m", "split"]);
    // A row of a key that moves and a key that stays cannot move.
    r.write(
        "docs/billing.md",
        &billing.replace("| R-3 | test_refund |", "| R-3, R-9 | test_refund |"),
    );
    r.append(
        "docs/billing.md",
        "\n## Extra\n\n- **R-9**: The desk SHALL close at 5.\n",
    );
    let (code, _, err) = r.run(&["mv", "billing#refunds", "refunds"]);
    assert_eq!(code, 1, "{err}");
    assert!(
        err.contains("names keys that move (R-3) and keys that stay (R-9)"),
        "{err}"
    );
    assert!(!r.dir.join("docs/refunds.md").exists());
    // The rows join the table of an existing companion section.
    git(&r.dir, &["checkout", "-q", "--", "."]);
    let (code, out, err) = r.run(&["mv", "billing#refunds", "payments"]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(
        r.read("docs/payments.md")
            .contains("| R-2 | test_receipt |\n| R-3 | test_refund |\n"),
        "{}",
        r.read("docs/payments.md")
    );
}

#[test]
fn history_moves_the_configured_sections_into_a_reference_document() {
    let r = hygiene_project();
    let (code, out, err) = r.run(&["history", "guides/desk"]);
    assert_eq!(code, 0, "{err}{out}");
    let h = r.read("docs/guides/desk-history.md");
    assert!(h.starts_with("---\ntype: reference\naliases:\n"), "{h}");
    assert!(h.contains("The history of [Desk guide](desk.md)."), "{h}");
    assert!(
        h.contains("## Iteration log\n\n### Round 1\n\nWe tried 8."),
        "{h}"
    );
    let desk = r.read("docs/guides/desk.md");
    assert!(
        desk.ends_with("## Iteration log\n\nThis section moved to [Desk history](desk-history.md#iteration-log).\n"),
        "{desk}"
    );
    // A decision root holds decisions only: name the destination.
    let (code, _, err) = r.run(&["history", "ADR-3"]);
    assert_eq!(code, 2, "{err}");
    assert!(
        err.contains("give the path of the reference document"),
        "{err}"
    );
    let (code, out, err) = r.run(&["history", "ADR-3", "docs/history/adr-3.md"]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(out.starts_with("moved 2 sections"), "{out}");
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    let (code, _, err) = r.run(&["history", "billing"]);
    assert_eq!(code, 5, "{err}");
}

#[test]
fn a_generated_index_replaces_a_hand_kept_registry() {
    let r = hygiene_project();
    let (code, _, err) = r.run(&["index", "registry"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("pass --replace"), "{err}");
    let (code, out, err) = r.run(&["index", "registry", "--replace"]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(out.contains("a generated index of 2 entries; links to its old sections now go to the entries in 1 documents"), "{out}");
    let index = r.read("registry/README.md");
    assert!(index.contains("generated: spec-dna index h:"), "{index}");
    assert!(
        index
            .contains("| [INC-1](INC-1.md) | Outage |  |\n| [INC-2](INC-2.md) | Slow queue |  |\n"),
        "{index}"
    );
    assert!(
        r.read("docs/notes/links.md")
            .contains("[the slow queue](../../registry/INC-2.md)")
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("SD610"), "{out}");
    // A new entry makes the index stale.
    r.write(
        "registry/INC-10.md",
        "# Lost keys\n\n## Summary\n\nX. See [[ADR-3]].\n\n## Root cause\n\nY.\n",
    );
    let (_, out, _) = r.run(&["check"]);
    assert!(out.contains("warning SD610 index-stale"), "{out}");
    let (code, out, err) = r.run(&["index", "registry"]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(r.read("registry/README.md").contains(
        "| [INC-2](INC-2.md) | Slow queue |  |\n| [INC-10](INC-10.md) | Lost keys |  |\n"
    ));
    let (_, out, _) = r.run(&["check"]);
    assert!(!out.contains("SD610"), "{out}");
    // A hand edit of the rows makes it stale too.
    r.write(
        "registry/README.md",
        &r.read("registry/README.md")
            .replace("| [INC-1](INC-1.md) | Outage |  |\n", ""),
    );
    let (_, out, _) = r.run(&["check"]);
    assert!(out.contains("edited by hand"), "{out}");
}

#[test]
fn mv_leaves_a_stub_for_tools_that_read_by_path() {
    let r = hygiene_project();
    let (code, out, err) = r.run(&["mv", "guides/desk", "guides/front-desk", "--stub"]);
    assert_eq!(code, 0, "{err}{out}");
    assert!(out.contains("left a stub at docs/guides/desk.md"), "{out}");
    assert_eq!(
        r.read("docs/guides/desk.md"),
        "---\nmoved-to: guides/front-desk\n---\n# Desk guide\n\nThis document moved to [Desk guide](front-desk.md).\n"
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "no duplicate id, no type rule for a stub: {out}");
    let (code, out, _) = r.run(&["show", "guides/desk#cash-count"]);
    assert_eq!(code, 0, "the old id names the moved document: {out}");
    r.write(
        "docs/guides/gone.md",
        "---\nmoved-to: guides/nowhere\n---\n# Gone\n",
    );
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("error SD101 unresolved-reference: `guides/nowhere` names no document"),
        "{out}"
    );
    let (code, _, err) = r.run(&["mv", "billing#charges", "x", "--stub"]);
    assert_eq!(code, 2, "{err}");
}

#[test]
fn a_configuration_change_must_be_its_own_change() {
    let r = hygiene_project();
    r.append("spec-dna.toml", "\n[context]\nbudget = 9000\n");
    let (code, out, _) = r.run(&["check", "--base", "HEAD"]);
    assert_eq!(code, 0, "the configuration alone: {out}");
    r.append("docs/billing.md", "\nMore text.\n");
    let (code, out, _) = r.run(&["check", "--base", "HEAD"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("spec-dna.toml:1: error SD503 config-not-alone: the change edits spec-dna.toml and 1 other file (first: `docs/billing.md`)"),
        "{out}"
    );
    git(&r.dir, &["checkout", "-q", "--", "docs/billing.md"]);
    r.write("src/new.py", "x = 1\n");
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 1, "an untracked file counts without --base: {out}");
    // The adoption: a new spec-dna.toml with the adapters is not checked.
    let fresh = Repo::empty();
    fresh.write("docs/a.md", "# A\n");
    fresh.commit("docs");
    fresh.run(&["init", "--non-interactive"]);
    let (code, out, _) = fresh.run(&["check", "--base", "HEAD"]);
    assert!(!out.contains("SD503"), "{code} {out}");
}

#[test]
fn ids_of_reference_roots_have_the_root_as_namespace() {
    let r = hygiene_project();
    let (code, out, _) = r.run(&["check", "--all", "--json"]);
    assert_eq!(
        code, 0,
        "no SD201 for the same slug in 2 reference roots: {out}"
    );
    assert!(
        out.contains("\"id\":\"plans/rollout\"") && out.contains("\"id\":\"briefs/rollout\""),
        "{out}"
    );
    let (code, out, _) = r.run(&["show", "plans/launch"]);
    assert_eq!(code, 0, "{out}");
    // The id of 0.2 stays a fallback name if it names one document.
    let (code, out, _) = r.run(&["show", "launch"]);
    assert_eq!(code, 0, "{out}");
    let (code, _, _) = r.run(&["show", "rollout"]);
    assert_eq!(code, 5, "two documents had the id `rollout`");
}

#[test]
fn a_diagnostic_that_moves_with_a_section_is_not_new() {
    let r = hygiene_project();
    r.write(
        "docs/guides/desk.md",
        &r.read("docs/guides/desk.md")
            .replace("Count the cash.", "Count the cash. See [[nowhere/doc]]."),
    );
    r.commit("an old broken reference");
    let (code, out, err) = r.run(&["mv", "guides/desk#closing", "guides/closing"]);
    assert_eq!(code, 0, "{err}{out}");
    git(&r.dir, &["add", "-A"]);
    let (code, out, _) = r.run(&["check", "--base", "HEAD"]);
    assert_eq!(code, 0, "the error was in the base: {out}");
    assert!(out.contains("0 new errors"), "{out}");
}
