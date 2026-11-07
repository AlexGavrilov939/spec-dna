//! Integration tests: they run the binary on generic fixture repositories.
//! The golden files are in tests/golden. To regenerate them, run
//! `UPDATE_GOLDEN=1 cargo test`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_spec-dna");
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
        fs::write(p, text).unwrap();
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

fn golden(name: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, actual).unwrap();
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
    let warnings: Vec<&str> = out.lines().filter(|l| l.contains("SD605")).collect();
    assert_eq!(warnings.len(), 1, "{out}");
    assert!(
        warnings[0].contains(":16: warning SD605 amendment-outcome"),
        "{out}"
    );
}

#[test]
fn fresh_project_init_check_uninstall() {
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out,
        "wrote spec-dna.toml\nwrote AGENTS.md\nwrote CLAUDE.md\nwrote .claude/skills/spec-dna/SKILL.md\n"
    );
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
    assert!(
        r.read("AGENTS.md")
            .starts_with("# Rules\n\nOwn text stays.\n\n<!-- spec-dna:begin agents v")
    );
    assert_eq!(
        r.read("CLAUDE.md"),
        "@AGENTS.md\n",
        "an existing import is kept"
    );
    r.commit("init");
    let (code, out, _) = r.run(&["init"]);
    assert_eq!(
        (code, out.as_str()),
        (0, "spec-dna init: everything is up to date\n")
    );
    r.write(
        "AGENTS.md",
        &r.read("AGENTS.md")
            .replace("before you commit", "before you push"),
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
            .replace("spec-dna = \"0.1.0\"", "spec-dna = \"9.9.9\""),
    );
    let (code, _, err) = r.run(&["check"]);
    assert_eq!(code, 3);
    assert!(err.contains("cargo install --locked --git"), "{err}");
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
        b"spec-dna 0.1.0\n"
    );
}

#[test]
fn hard_budget_is_an_error_for_new_and_growing_documents() {
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nstatus = \"active\"\n[[roots]]\npath = \"lax\"\ndefault-type = \"spec\"\nstatus = \"active\"\nenforce-budgets = false\n[budgets.spec]\nsoft = 100\nhard = 200\n",
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
    // Net growth of a document over the hard budget is an error.
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
    let found: Vec<&str> = out.lines().filter(|l| l.contains(" SD")).collect();
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
