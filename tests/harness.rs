//! Integration tests of the agent harness (ADR 0004): `init` and `check` on the
//! harness files. The helpers are small copies of the ones in tests/cli.rs.

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
        let _ = fs::remove_file(self.answers_path());
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
    /// A repository with one document and the given harness table.
    fn with_harness(harness: &str) -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("spec-dna-harness-test-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        let r = Repo { dir };
        r.write(
            "spec-dna.toml",
            &format!("spec-dna = \"{VERSION}\"\n\n[[roots]]\npath = \"docs\"\n\n{harness}"),
        );
        r.write("docs/a.md", "# A\n\nText.\n");
        r.commit("start");
        r
    }

    /// An empty repository.
    fn empty() -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("spec-dna-harness-test-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        Repo { dir }
    }

    /// The path of the answers file. It is outside the repository.
    fn answers_path(&self) -> PathBuf {
        PathBuf::from(format!("{}-answers.toml", self.dir.display()))
    }

    /// Runs `init` with an answers file and the other arguments.
    fn init_with(&self, answers: &str, extra: &[&str]) -> (i32, String, String) {
        let path = self.answers_path();
        fs::write(&path, answers).unwrap();
        let mut args = vec!["init", "--answers", path.to_str().unwrap()];
        args.extend_from_slice(extra);
        self.run3(&args)
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.dir.join(path)).unwrap()
    }

    fn exists(&self, path: &str) -> bool {
        self.dir.join(path).exists()
    }

    /// Like `run`, with the text of stderr.
    fn run3(&self, args: &[&str]) -> (i32, String, String) {
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

    fn commit(&self, msg: &str) {
        git(&self.dir, &["add", "-A"]);
        git(&self.dir, &["commit", "-q", "--allow-empty", "-m", msg]);
    }

    fn write(&self, path: &str, text: &str) {
        let p = self.dir.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn run(&self, args: &[&str]) -> (i32, String) {
        let out = Command::new(BIN)
            .args(args)
            .current_dir(&self.dir)
            .output()
            .expect("binary runs");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    }
}

#[test]
fn check_reports_each_missing_harness_file() {
    let r = Repo::with_harness("[harness]\nlevel = \"lean\"\n");
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    for file in [
        ".claude/agents/fallback-advisor.md",
        ".claude/agents/checker.md",
        ".claude/agents/Explore.md",
        ".claude/agents/model-team.md",
    ] {
        assert!(
            out.contains(&format!("{file}:1: error SD504 harness-missing")),
            "{file}: {out}"
        );
    }
    assert!(
        out.contains("CLAUDE.md:1: error SD504 harness-missing"),
        "{out}"
    );
    assert!(out.contains("SD504 fix: Run `spec-dna init`"), "{out}");
    // Without `[harness]`, with level none, and with the rule off: no finding.
    for harness in [
        "",
        "[harness]\nlevel = \"none\"\n",
        "[harness]\nlevel = \"lean\"\n[rules]\nSD504 = \"off\"\n",
    ] {
        let r = Repo::with_harness(harness);
        let (code, out) = r.run(&["check"]);
        assert!(!out.contains("SD504"), "{harness}: {code} {out}");
    }
}

#[test]
fn check_reads_untracked_files_and_reports_a_file_that_spec_dna_did_not_write() {
    let r = Repo::with_harness("[harness]\nlevel = \"lean\"\n");
    r.write(".claude/agents/checker.md", "my own agent\n");
    let (_, out) = r.run(&["check"]);
    assert!(
        out.contains("checker.md:1: error SD504 harness-missing: the file has no harness marker"),
        "{out}"
    );
    assert!(
        !out.contains("checker.md:1: error SD504 harness-missing: the generated file is absent")
    );
    // The gate against a base reads tracked files only.
    let (_, out) = r.run(&["check", "--base", "HEAD"]);
    assert!(
        out.contains("checker.md:1: error SD504 harness-missing: the generated file is absent"),
        "{out}"
    );
}

#[test]
fn check_reports_a_settings_file_that_is_not_json() {
    let r = Repo::with_harness("[harness]\nlevel = \"lean\"\n");
    r.write(".claude/settings.json", "{\"model\": ");
    let (code, out) = r.run(&["check", "--json"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("\"code\":\"SD505\",\"name\":\"harness-settings-invalid\""),
        "{out}"
    );
    assert!(!out.contains("\"code\":\"SD502\""), "{out}");
    r.write(".claude/settings.json", "{}");
    let (_, out) = r.run(&["check", "--json"]);
    assert!(!out.contains("SD505"), "{out}");
    assert!(out.contains("\"target\":\"advisorModel\""), "{out}");
}

#[test]
fn harness_files_do_not_make_a_configuration_change_impure() {
    let r = Repo::with_harness("");
    r.write(
        "spec-dna.toml",
        &format!(
            "spec-dna = \"{VERSION}\"\n\n[[roots]]\npath = \"docs\"\n\n[harness]\nlevel = \"lean\"\n"
        ),
    );
    r.write(".claude/settings.json", "{}\n");
    r.write(".claude/agents/checker.md", "text\n");
    r.write("CLAUDE.md", "# Rules\n");
    let (_, out) = r.run(&["check"]);
    assert!(!out.contains("SD503"), "{out}");
    r.write("src/new.py", "x = 1\n");
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("error SD503 config-not-alone: the change edits spec-dna.toml and 1 other file (first: `src/new.py`)"),
        "{out}"
    );
}

const LEAN_FILES: [&str; 4] = [
    ".claude/agents/fallback-advisor.md",
    ".claude/agents/checker.md",
    ".claude/agents/Explore.md",
    ".claude/agents/model-team.md",
];

/// A repository with the lean harness that `init` wrote.
fn lean_repo() -> Repo {
    let r = Repo::empty();
    let (code, out, err) = r.run3(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    r
}

/// Replaces the version of the marker of a generated file with an old one. The
/// hash covers the text above the marker, so it stays valid.
fn age_marker(text: &str) -> String {
    text.replace(&format!("harness v{VERSION} "), "harness v0.0.1 ")
}

#[test]
fn init_writes_the_lean_harness_and_check_is_clean() {
    let r = Repo::empty();
    let (code, out, err) = r.run3(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    for f in LEAN_FILES {
        assert!(r.exists(f), "{f}");
        assert!(out.contains(&format!("wrote {f}\n")), "{out}");
    }
    assert!(!r.exists(".claude/agents/implementer.md"));
    assert!(!r.exists(".claude/agents/spec-writer.md"));
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains("\n[harness]\nlevel = \"lean\"\ndeny = [\"NotebookEdit\"]\n"),
        "{cfg}"
    );
    let claude = r.read("CLAUDE.md");
    assert!(claude.contains("\n\nModel team: opus builds."), "{claude}");
    assert!(claude.contains("<!-- spec-dna v"), "{claude}");
    let settings = r.read(".claude/settings.json");
    for want in [
        "\"model\": \"opus\"",
        "\"advisorModel\": \"fable\"",
        "\"CLAUDE_CODE_SUBAGENT_MODEL\": \"haiku\"",
        "\"NotebookEdit\"",
        "spec-dna hook context-meter --levels 300000,600000 --subagent 250000 2>/dev/null || true",
        "spec-dna hook fanout-cap --cap 10 2>/dev/null || true",
    ] {
        assert!(settings.contains(want), "{want}\n{settings}");
    }
    assert!(out.contains("The harness adds about "), "{out}");
    assert!(
        out.contains("Each teammate needs spec-dna 0.5.0 or later on PATH for the hooks."),
        "{out}"
    );
    assert!(
        out.contains("\nRestart Claude Code. Then check: the header shows the main model, /advisor shows the advisor, and @ lists the agents.\n"),
        "{out}"
    );
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    // A second `init` has nothing to write.
    let (code, out, err) = r.run3(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with("spec-dna init: everything is up to date\n"),
        "{out}"
    );
    assert!(!out.contains("Restart Claude Code"), "{out}");
}

#[test]
fn a_dry_run_shows_the_harness_and_writes_nothing() {
    let r = Repo::empty();
    let (code, out, err) = r.run3(&["init", "--dry-run", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("Agent harness: lean (for Claude Code)\n"),
        "{out}"
    );
    assert!(
        out.contains("  create .claude/agents/checker.md (agent checker: opus, high)\n"),
        "{out}"
    );
    assert!(
        out.contains("  create .claude/settings.json (settings: ") && out.contains("2 hooks)\n"),
        "{out}"
    );
    assert!(
        out.contains("  create CLAUDE.md (one generated line; CLAUDE.md: model-team line)\n"),
        "{out}"
    );
    assert!(!r.exists(".claude"));
    assert!(!r.exists("spec-dna.toml"));
    let (code, out, _) = r.run3(&["init", "--dry-run", "--non-interactive", "--json"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("\"harness\":{\"level\":\"lean\",\"files\":[{\"path\":\".claude/agents/fallback-advisor.md\",\"action\":\"create\",\"note\":\"agent fallback-advisor: opus, xhigh\"}"),
        "{out}"
    );
}

#[test]
fn an_answers_file_writes_the_full_harness_with_the_chosen_models() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with(
        "harness = \"full\"\nharness-models = [\"orchestrator=sonnet:high\", \"advisor=opus/none\"]\nharness-deny = []\n",
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    for f in [
        ".claude/agents/implementer.md",
        ".claude/agents/spec-writer.md",
        ".claude/agents/checker.md",
        ".claude/agents/Explore.md",
        ".claude/agents/model-team.md",
    ] {
        assert!(r.exists(f), "{f}");
    }
    assert!(!r.exists(".claude/agents/fallback-advisor.md"));
    let cfg = r.read("spec-dna.toml");
    for want in [
        "level = \"full\"\ndeny = []\n",
        "orchestrator = { claude-code = \"sonnet\", effort = \"high\"",
        "advisor = { claude-code = \"opus\", agent = \"none\"",
        "implementer = { claude-code = \"sonnet\", effort = \"high\" }",
    ] {
        assert!(cfg.contains(want), "{want}\n{cfg}");
    }
    let settings = r.read(".claude/settings.json");
    assert!(settings.contains("\"model\": \"sonnet\""), "{settings}");
    assert!(
        settings.contains("\"advisorModel\": \"opus\""),
        "{settings}"
    );
    assert!(!settings.contains("NotebookEdit"), "{settings}");
    let claude = r.read("CLAUDE.md");
    assert!(claude.contains("Model team: sonnet builds."), "{claude}");
    assert!(
        claude.contains("Give a code change to `implementer`"),
        "{claude}"
    );
    assert!(!claude.contains("fallback-advisor"), "{claude}");
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    // A bad model line is a usage error, and nothing is written.
    let r = Repo::empty();
    for answers in [
        "harness = \"wide\"\n",
        "harness-models = [\"reviewer\"]\n",
        "harness-models = [\"judge=opus\"]\n",
        "harness-models = [\"implementer=opus\"]\n",
        "harness-models = [\"advisor=haiku\"]\n",
        "harness-models = [\"orchestrator=none\"]\n",
        "harness-models = [\"helper=haiku:high\"]\n",
    ] {
        let (code, out, err) = r.init_with(answers, &[]);
        assert_eq!(code, 2, "{answers}: {out}{err}");
        assert!(!r.exists("spec-dna.toml"), "{answers}");
    }
}

#[test]
fn reconfigure_moves_between_the_levels_and_removes_what_it_wrote() {
    let r = lean_repo();
    r.commit("init");
    let (code, out, err) = r.init_with("harness = \"full\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.exists(".claude/agents/implementer.md"));
    assert!(r.exists(".claude/agents/spec-writer.md"));
    assert!(
        out.contains("wrote .claude/agents/implementer.md\n"),
        "{out}"
    );
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    // Back to lean: the two files go.
    let (code, out, err) = r.init_with("harness = \"lean\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.exists(".claude/agents/implementer.md"));
    assert!(
        out.contains("removed .claude/agents/implementer.md\n"),
        "{out}"
    );
    // Off: the agent files, the team page and the model-team line go.
    r.commit("lean again");
    let (code, out, err) = r.init_with("harness = \"none\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    for f in LEAN_FILES {
        assert!(!r.exists(f), "{f}");
        assert!(out.contains(&format!("removed {f}\n")), "{out}");
    }
    let claude = r.read("CLAUDE.md");
    assert!(!claude.contains("Model team"), "{claude}");
    assert!(
        claude.contains("<!-- spec-dna v"),
        "the adapter line stays: {claude}"
    );
    // The scalar keys stay, the hook handlers go, and `init` says what stays.
    let settings = r.read(".claude/settings.json");
    assert!(settings.contains("\"model\": \"opus\""), "{settings}");
    assert!(
        settings.contains("\"advisorModel\": \"fable\""),
        "{settings}"
    );
    assert!(!settings.contains("spec-dna hook"), "{settings}");
    assert!(
        out.contains(
            "kept model: spec-dna no longer sets it; remove it by hand if you do not want it\n"
        ),
        "{out}"
    );
    assert!(
        r.read("spec-dna.toml")
            .contains("[harness]\nlevel = \"none\"\n")
    );
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    // A plain `init` after that writes no harness.
    let (code, out, _) = r.run3(&["init"]);
    assert_eq!(code, 0, "{out}");
    assert!(!r.exists(".claude/agents/checker.md"));
}

#[test]
fn reconfigure_keeps_a_generated_file_that_a_person_edited() {
    let r = lean_repo();
    let p = ".claude/agents/checker.md";
    r.write(
        p,
        &r.read(p)
            .replace("<!-- spec-dna harness", "Mine.\n<!-- spec-dna harness"),
    );
    let (code, out, err) = r.init_with("harness = \"none\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.exists(p));
    assert!(r.read(p).contains("Mine."));
    assert!(
        out.contains("kept .claude/agents/checker.md: edited by hand (SD501)\n"),
        "{out}"
    );
    assert!(!r.exists(".claude/agents/Explore.md"));
}

#[test]
fn a_file_of_the_project_makes_its_role_none_and_stays() {
    let r = Repo::empty();
    let own = "---\nname: checker\ndescription: mine\n---\n\nMy reviewer.\n";
    r.write(".claude/agents/checker.md", own);
    let (code, out, err) = r.run3(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.read(".claude/agents/checker.md"), own);
    assert!(
        out.contains("note: .claude/agents/checker.md holds the agent `checker`, and spec-dna did not generate it: the role `reviewer` is none\n"),
        "{out}"
    );
    assert!(
        r.read("spec-dna.toml")
            .contains("reviewer = { claude-code = \"none\"")
    );
    assert!(!r.read("CLAUDE.md").contains("`checker`"));
    assert!(r.exists(".claude/agents/Explore.md"));
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    // Another repository: the answers force the role. That is a usage error.
    let r = Repo::empty();
    r.write(".claude/agents/checker.md", own);
    let (code, out, err) = r.init_with("harness-models = [\"reviewer=opus\"]\n", &[]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("holds the agent `checker`"), "{err}");
    assert!(!r.exists("spec-dna.toml"));
    assert_eq!(r.read(".claude/agents/checker.md"), own);
    // An agent with the name of the advisor agent turns off only its file.
    let r = Repo::empty();
    r.write(".claude/agents/fallback-advisor.md", "own\n");
    let (code, out, err) = r.run3(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.read(".claude/agents/fallback-advisor.md"), "own\n");
    assert!(
        r.read("spec-dna.toml")
            .contains("advisor = { claude-code = \"fable\", agent = \"none\"")
    );
    assert!(
        r.read(".claude/settings.json")
            .contains("\"advisorModel\": \"fable\"")
    );
}

#[test]
fn a_git_ignore_of_the_claude_folder_refuses_the_harness() {
    let r = Repo::empty();
    r.write(".gitignore", ".claude/\n");
    for args in [
        &["init", "--non-interactive"][..],
        &["init", "--dry-run", "--non-interactive"][..],
    ] {
        let (code, out, err) = r.run3(args);
        assert_eq!(code, 2, "{out}{err}");
        assert!(err.contains("git ignores .claude/"), "{err}");
    }
    assert!(!r.exists("spec-dna.toml"));
    // The rule for the agents folder alone refuses too.
    r.write(".gitignore", ".claude/agents/\n");
    let (code, _, err) = r.run3(&["init", "--non-interactive"]);
    assert_eq!(code, 2, "{err}");
    // With the harness off, nothing is refused.
    let (code, out, err) = r.init_with("harness = \"none\"\nadapters = [\"agents-md\"]\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn settings_that_are_not_json_refuse_the_harness() {
    let r = Repo::empty();
    r.write(".claude/settings.json", "{\"model\": ");
    let (code, out, err) = r.run3(&["init", "--non-interactive"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.contains(".claude/settings.json is not valid JSON"),
        "{err}"
    );
    assert!(!r.exists("spec-dna.toml"));
    assert_eq!(r.read(".claude/settings.json"), "{\"model\": ");
}

#[test]
fn check_reports_each_kind_of_drift_after_init() {
    let r = lean_repo();
    r.commit("init");
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    let (agent, settings, explore) = (
        ".claude/agents/checker.md",
        ".claude/settings.json",
        ".claude/agents/Explore.md",
    );
    let (agent_text, settings_text, explore_text) =
        (r.read(agent), r.read(settings), r.read(explore));
    // SD501: a hand edit of a generated file.
    r.write(
        agent,
        &agent_text.replace("<!-- spec-dna harness", "Mine.\n<!-- spec-dna harness"),
    );
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("checker.md:") && out.contains("error SD501"),
        "{out}"
    );
    r.write(agent, &agent_text);
    // SD502: a settings entry that differs.
    r.write(
        settings,
        &settings_text.replace("\"advisorModel\": \"fable\"", "\"advisorModel\": \"opus\""),
    );
    let (code, out) = r.run(&["check", "--json"]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("\"code\":\"SD502\"") && out.contains("\"target\":\"advisorModel\""),
        "{out}"
    );
    // `init` writes the entry again.
    let (code, out, _) = r.run3(&["init"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(r.read(settings), settings_text);
    // SD504: a deleted file.
    fs::remove_file(r.dir.join(explore)).unwrap();
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("Explore.md:1: error SD504"), "{out}");
    r.write(explore, &explore_text);
    // SD505: settings that are not JSON.
    r.write(settings, "{\"model\": ");
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("error SD505"), "{out}");
    r.write(settings, &settings_text);
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn the_upgrade_regenerates_an_old_file_and_keeps_an_edited_one() {
    let r = lean_repo();
    let (old, edited) = (".claude/agents/checker.md", ".claude/agents/Explore.md");
    let (old_text, edited_text) = (r.read(old), r.read(edited));
    r.write(old, &age_marker(&old_text));
    assert_ne!(r.read(old), old_text);
    let by_hand = edited_text.replace("<!-- spec-dna harness", "Mine.\n<!-- spec-dna harness");
    r.write(edited, &by_hand);
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("checker.md:21: error SD502"), "{out}");
    let (code, out, err) = r.run3(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.read(old), old_text, "the old file is generated again");
    assert_eq!(r.read(edited), by_hand, "the edited file stays");
    assert!(out.contains("wrote .claude/agents/checker.md\n"), "{out}");
    assert!(!out.contains("wrote .claude/agents/Explore.md"), "{out}");
    assert!(
        out.contains("kept .claude/agents/Explore.md: edited by hand (SD501); delete it and run `spec-dna init` to generate it again\n"),
        "{out}"
    );
    assert!(out.contains("Restart Claude Code."), "{out}");
    // `--dry-run` shows the same and writes nothing.
    r.write(old, &age_marker(&old_text));
    let (code, out, _) = r.run3(&["init", "--dry-run"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("would change .claude/agents/checker.md ("),
        "{out}"
    );
    assert_eq!(r.read(old), age_marker(&old_text));
}

#[test]
fn a_configuration_without_a_harness_table_gets_none_until_reconfigure() {
    let r = Repo::with_harness("");
    let (code, out, err) = r.run3(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.exists(".claude/agents"), "an upgrade adds no harness");
    assert!(!r.exists(".claude/settings.json"));
    assert!(!out.contains("Agent harness"), "{out}");
    let (code, out, err) = r.run3(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        r.exists(".claude/agents/checker.md"),
        "reconfigure defaults to lean"
    );
    assert!(
        r.read("spec-dna.toml")
            .contains("[harness]\nlevel = \"lean\"")
    );
}

#[test]
fn init_keeps_foreign_settings_in_order() {
    let r = Repo::empty();
    r.write(
        ".claude/settings.json",
        "{\n  \"theme\": \"dark\",\n  \"permissions\": {\n    \"allow\": [\"Bash(ls)\"],\n    \"deny\": [\"WebFetch\"]\n  },\n  \"hooks\": {\n    \"PostToolUse\": [\n      {\n        \"matcher\": \"Edit\",\n        \"hooks\": [{\"type\": \"command\", \"command\": \"echo foreign\"}]\n      }\n    ]\n  },\n  \"zeta\": 1\n}\n",
    );
    let (code, out, err) = r.run3(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let s = r.read(".claude/settings.json");
    let at = |needle: &str| s.find(needle).unwrap_or_else(|| panic!("{needle}\n{s}"));
    assert!(at("\"theme\"") < at("\"permissions\""), "{s}");
    assert!(at("\"permissions\"") < at("\"hooks\""), "{s}");
    assert!(at("\"hooks\"") < at("\"zeta\""), "{s}");
    assert!(
        at("\"zeta\"") < at("\"model\""),
        "new keys go at the end: {s}"
    );
    assert!(at("\"Bash(ls)\"") < at("\"deny\""), "{s}");
    assert!(at("\"WebFetch\"") < at("\"NotebookEdit\""), "{s}");
    assert!(
        at("echo foreign") < at("spec-dna hook context-meter"),
        "{s}"
    );
    assert!(s.contains("\"matcher\": \"Edit\""), "{s}");
    assert!(s.ends_with("}\n"), "{s}");
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn the_detected_models_and_notebooks_set_the_defaults() {
    let r = Repo::empty();
    r.write(
        ".claude/settings.json",
        "{\"model\": \"sonnet\", \"advisorModel\": \"opus\"}\n",
    );
    r.write("notes/analysis.ipynb", "{}\n");
    r.commit("start");
    let (code, out, err) = r.run3(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(cfg.contains("deny = []\n"), "git tracks a notebook: {cfg}");
    assert!(
        cfg.contains("orchestrator = { claude-code = \"sonnet\""),
        "{cfg}"
    );
    assert!(cfg.contains("advisor = { claude-code = \"opus\""), "{cfg}");
    let s = r.read(".claude/settings.json");
    assert!(s.contains("\"model\": \"sonnet\""), "{s}");
    assert!(!s.contains("NotebookEdit"), "{s}");
}

#[test]
fn a_claude_file_with_an_import_gets_the_model_team_line_after_it() {
    let r = Repo::empty();
    r.write("AGENTS.md", "# Agents\n\nOwn text.\n");
    r.write("CLAUDE.md", "@AGENTS.md\n");
    let (code, out, err) = r.run3(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let claude = r.read("CLAUDE.md");
    assert!(claude.starts_with("@AGENTS.md\n\nModel team: "), "{claude}");
    assert!(
        !claude.contains("<!-- spec-dna v"),
        "the import gives the line: {claude}"
    );
    assert!(r.read("AGENTS.md").contains("<!-- spec-dna v"));
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    // The block of an older version: the import stays, and so does the line.
    let r = Repo::empty();
    r.write("AGENTS.md", "# Agents\n\nOwn text.\n");
    r.write(
        "CLAUDE.md",
        "<!-- spec-dna:begin claude v0.0.9 h:0000000000000000 -->\n@AGENTS.md\n<!-- spec-dna:end claude -->\n",
    );
    let (code, out, err) = r.run3(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let claude = r.read("CLAUDE.md");
    assert!(claude.starts_with("@AGENTS.md\n\nModel team: "), "{claude}");
    let (code, out) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    // Off again: the import stays alone.
    let (code, out, err) = r.init_with("harness = \"none\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.read("CLAUDE.md"), "@AGENTS.md\n");
}
