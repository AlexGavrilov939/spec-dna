//! Tests of `init` questions 12 to 14 and the starter files of the answer
//! `skeleton` (step 4, slice S9). Each test uses a temporary repository. No
//! test reads or writes a file of this repository, and none calls the
//! network. The golden files are in `tests/golden/evals/`.

mod common;

use common::*;
use std::fs;

const SKILLS: [&str; 4] = [
    ".claude/skills/spec-dna-intent/SKILL.md",
    ".claude/skills/spec-dna-spec/SKILL.md",
    ".claude/skills/spec-dna-plan/SKILL.md",
    ".claude/skills/spec-dna-review/SKILL.md",
];

const STARTER: [&str; 3] = [
    "evals/check.sh",
    "evals/example.json",
    "evals/fixtures/example/notes.txt",
];

const WORKFLOW: &str = ".github/workflows/agent-evals.yml";

const INCIDENT: &str = "---\ntype: incident\nstatus: active\n---\n# Hose leaks\n\n## Summary\n\nA hose leaked.\n\n## Root cause\n\nA worn seal.\n";

fn golden(name: &str) -> String {
    let p = format!("{}/tests/golden/evals/{name}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"))
}

/// A new project: an empty repository after `init --non-interactive`.
fn new_project() -> Repo {
    let r = Repo::new();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    r
}

/// An answers file in the repository folder. It is not part of the project.
fn init_with(r: &Repo, answers: &str, args: &[&str]) -> (i32, String, String) {
    let file = r.dir.join("..").join(format!(
        "spec-dna-answers-{}-{}.toml",
        std::process::id(),
        r.dir.file_name().unwrap().to_string_lossy()
    ));
    fs::write(&file, answers).unwrap();
    let mut a = vec!["init", "--answers", file.to_str().unwrap()];
    a.extend_from_slice(args);
    let res = r.run(&a);
    let _ = fs::remove_file(&file);
    res
}

fn lines_with<'a>(out: &'a str, code: &str) -> Vec<&'a str> {
    out.lines().filter(|l| l.contains(code)).collect()
}

#[test]
fn a_new_project_with_the_defaults_passes_check_and_has_no_sd506() {
    let r = new_project();
    for f in STARTER
        .iter()
        .chain(SKILLS.iter())
        .chain(["REVIEW.md"].iter())
    {
        assert!(r.dir.join(f).exists(), "{f}");
    }
    // The CI answer of an empty repository is `none`: no workflow.
    assert!(!r.dir.join(WORKFLOW).exists());
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains("\n[evals]  # agent evals (docs/config.md)\ndir = \"evals\"\nmin-pass-rate = 0.8  # the share of evals that must pass\nruns = 3  # the runs of an eval that sets no `runs`\nmax-total-usd = 20  # eval run refuses or stops above this spend\n"),
        "{cfg}"
    );
    // The table `[evals]` comes directly before `[harness]`.
    assert!(
        cfg.find("[evals]").unwrap() < cfg.find("[harness]").unwrap(),
        "{cfg}"
    );
    assert!(
        cfg.contains("skills = [\"intent\", \"spec\", \"plan\", \"review\"]\nreview-md = true\nprotect-tests = \"fix-branches\"\n"),
        "{cfg}"
    );
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(lines_with(&out, "SD506").is_empty(), "{out}");
    assert!(lines_with(&out, "SD50").is_empty(), "{out}");
}

#[test]
fn the_starter_files_match_the_golden_files() {
    let r = new_project();
    assert_eq!(r.read("evals/check.sh"), golden("check.sh"));
    assert_eq!(r.read("evals/example.json"), golden("example.json"));
    assert_eq!(
        r.read("evals/fixtures/example/notes.txt"),
        "status: draft\n"
    );
}

#[cfg(unix)]
#[test]
fn check_sh_has_mode_644_and_no_shebang() {
    use std::os::unix::fs::PermissionsExt;
    let r = new_project();
    let mode = fs::metadata(r.dir.join("evals/check.sh"))
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o644);
    assert!(!r.read("evals/check.sh").starts_with("#!"));
}

#[test]
fn check_sh_gives_a_message_when_no_test_glob_is_set() {
    let r = Repo::new();
    let (code, out, err) = init_with(&r, "evidence-style = \"verified-by\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let sh = r.read("evals/check.sh");
    assert!(
        sh.contains("  [ \"$#\" -gt 0 ] || { echo \"tests_unchanged: give the test pathspecs\" >&2; return 2; }\n"),
        "{sh}"
    );
    assert!(!sh.contains("set -- "), "{sh}");
    // No glob, no hook entry, and `check` passes.
    assert!(r.read("spec-dna.toml").contains("protect-tests = \"off\""));
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn check_sh_names_the_test_globs_of_the_configuration() {
    let r = Repo::new();
    r.write("spec/a_spec.rb", "# a\n");
    r.write("spec/b_spec.rb", "# b\n");
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    let sh = r.read("evals/check.sh");
    // Each glob of `[evidence] tests` is one quoted pathspec.
    let globs: Vec<&str> = cfg
        .lines()
        .find(|l| l.starts_with("tests = ["))
        .map(|l| l.split('#').next().unwrap())
        .map(|l| {
            l.trim_start_matches("tests = [")
                .trim_end()
                .trim_end_matches(']')
                .split(", ")
                .map(|g| g.trim_matches('"'))
                .collect()
        })
        .expect("a tests line");
    assert!(!globs.is_empty(), "{cfg}");
    for g in globs {
        assert!(sh.contains(&format!("':(glob){g}'")), "{g}\n{sh}");
    }
}

#[test]
fn the_ci_answer_github_writes_the_workflow_and_prints_the_2_commands() {
    let r = Repo::new();
    let (code, out, err) = init_with(&r, "ci = \"github\"\nbase = \"main\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.read(WORKFLOW), golden("agent-evals.yml"));
    assert!(out.contains(&format!("wrote {WORKFLOW}\n")), "{out}");
    let want = "To run the evals in CI, set the secret and the variable:\n  gh secret set ANTHROPIC_API_KEY\n  gh variable set SPEC_DNA_EVALS --body true\n";
    assert!(out.contains(want), "{out}");
    // The workflow has no marker.
    assert!(!r.read(WORKFLOW).contains("spec-dna harness"));
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_workflow_uses_the_base_branch_and_the_evals_folder() {
    let r = Repo::new();
    r.write("evals/keep.txt", "x\n");
    let (code, out, err) = init_with(&r, "ci = \"github\"\nbase = \"trunk\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let w = r.read(WORKFLOW);
    assert!(w.contains("    branches:\n      - trunk\n"), "{w}");
    assert!(w.contains("      - .spec-dna/evals/**\n"), "{w}");
    assert!(!w.contains("      - evals/**\n"), "{w}");
}

#[test]
fn the_workflow_follows_the_rules_of_agents_md() {
    let r = Repo::new();
    let (code, out, err) = init_with(&r, "ci = \"github\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let w = r.read(WORKFLOW);
    assert!(w.contains("\npermissions: {}\n"), "{w}");
    assert!(!w.contains("pull_request_target"), "{w}");
    assert!(w.contains("vars.SPEC_DNA_EVALS == 'true'"), "{w}");
    assert!(
        w.contains("github.event.pull_request.head.repo.full_name == github.repository"),
        "{w}"
    );
    assert!(
        w.contains("    permissions:\n      contents: read\n"),
        "{w}"
    );
    // Each `uses:` is a full commit SHA with a version tag.
    let uses: Vec<&str> = w.lines().filter(|l| l.contains("uses:")).collect();
    assert!(!uses.is_empty());
    for u in uses {
        let (_, rest) = u.split_once('@').unwrap();
        let sha = &rest[..40];
        assert!(sha.bytes().all(|b| b.is_ascii_hexdigit()), "{u}");
        assert!(rest[40..].starts_with(" # v"), "{u}");
    }
    // The SHA of `actions/checkout` is the SHA of `spec-dna.yml`, which `init`
    // writes in the same run.
    let specs = r.read(".github/workflows/spec-dna.yml");
    let pin = specs
        .lines()
        .find(|l| l.contains("actions/checkout@"))
        .unwrap()
        .split("uses: ")
        .nth(1)
        .unwrap();
    assert!(w.contains(pin), "{pin}\n{w}");
}

#[test]
fn the_ci_answers_gitlab_and_none_write_no_workflow() {
    for ci in ["gitlab", "none"] {
        let r = Repo::new();
        let (code, out, err) = init_with(&r, &format!("ci = \"{ci}\"\n"), &[]);
        assert_eq!(code, 0, "{ci}: {out}{err}");
        assert!(!r.dir.join(WORKFLOW).exists(), "{ci}");
        // The table and the starter files stay.
        assert!(r.read("spec-dna.toml").contains("\n[evals]"), "{ci}");
        assert!(r.dir.join("evals/check.sh").exists(), "{ci}");
        // No workflow, so `init` prints no command.
        assert!(!out.contains("gh variable set"), "{ci}: {out}");
        let (code, out, err) = r.run(&["check"]);
        assert_eq!(code, 0, "{ci}: {out}{err}");
    }
}

#[test]
fn json_has_the_field_evals_commands() {
    let r = Repo::new();
    let (code, json, err) = init_with(&r, "ci = \"github\"\n", &["--json"]);
    assert_eq!(code, 0, "{json}{err}");
    assert!(
        json.contains("\"evals_commands\":[\"gh secret set ANTHROPIC_API_KEY\",\"gh variable set SPEC_DNA_EVALS --body true\"]"),
        "{json}"
    );
    // A dry run prints nothing, so the array is empty.
    let r = Repo::new();
    let (code, json, err) = init_with(&r, "ci = \"github\"\n", &["--json", "--dry-run"]);
    assert_eq!(code, 0, "{json}{err}");
    assert!(json.contains("\"evals_commands\":[]"), "{json}");
    // The CI answer `none` writes no workflow.
    let r = Repo::new();
    let (code, json, err) = init_with(&r, "ci = \"none\"\n", &["--json"]);
    assert_eq!(code, 0, "{json}{err}");
    assert!(json.contains("\"evals_commands\":[]"), "{json}");
}

#[test]
fn the_preview_has_14_answers() {
    let r = Repo::new();
    let (code, out, err) = r.run(&["init", "--dry-run", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let shown = out.split("--- spec-dna.toml ---").next().unwrap();
    assert!(
        shown.contains("  14. evals: skeleton (default) -> [evals], evals/check.sh, evals/example.json, evals/fixtures/example/notes.txt\n"),
        "{shown}"
    );
    let (code, json, err) = r.run(&["init", "--dry-run", "--non-interactive", "--json"]);
    assert_eq!(code, 0, "{json}{err}");
    assert_eq!(json.matches("\"number\":").count(), 14, "{json}");
    assert!(
        json.contains("{\"number\":14,\"name\":\"evals\","),
        "{json}"
    );
    // The level `none` gives the answer `none`, and the preview says so.
    let (code, out, err) = init_with(&r, "harness = \"none\"\n", &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    let shown = out.split("--- spec-dna.toml ---").next().unwrap();
    assert!(
        shown.contains("  14. evals: none (default, writes nothing)\n"),
        "{shown}"
    );
}

#[test]
fn a_project_folder_evals_gives_the_dir_dot_spec_dna_evals() {
    let r = Repo::new();
    r.write("evals/data.csv", "a,b\n");
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(cfg.contains("dir = \".spec-dna/evals\""), "{cfg}");
    assert!(out.contains("note: ") && out.contains("`evals/`"), "{out}");
    for f in [
        ".spec-dna/evals/check.sh",
        ".spec-dna/evals/example.json",
        ".spec-dna/evals/fixtures/example/notes.txt",
    ] {
        assert!(r.dir.join(f).exists(), "{f}");
    }
    let json = r.read(".spec-dna/evals/example.json");
    assert!(
        json.contains("\"fixture\": \".spec-dna/evals/fixtures/example\""),
        "{json}"
    );
    // The folder of the project stays as it is.
    assert_eq!(r.read("evals/data.csv"), "a,b\n");
    assert!(!r.dir.join("evals/check.sh").exists());
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn an_existing_starter_file_stays_and_a_note_names_it() {
    let r = Repo::new();
    r.write("evals/data.csv", "a,b\n");
    r.write(".spec-dna/evals/check.sh", "mine() { :; }\n");
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.read(".spec-dna/evals/check.sh"), "mine() { :; }\n");
    assert!(
        out.contains("note: .spec-dna/evals/check.sh exists, so `init` did not write it"),
        "{out}"
    );
    assert!(r.dir.join(".spec-dna/evals/example.json").exists());
}

#[test]
fn a_second_init_changes_nothing() {
    let r = new_project();
    r.commit("init");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("everything is up to date"), "{out}");
    // `--reconfigure` with the current values changes no file but the comment
    // of the root, which `init` does not keep (docs/init.md, "Reconfigure").
    let before = r.read("spec-dna.toml");
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let after = r.read("spec-dna.toml");
    let kept: String = before
        .lines()
        .filter(|l| !l.starts_with("# new documents:"))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(after, kept);
    assert_eq!(r.git_out(&["status", "--porcelain"]), " M spec-dna.toml");
}

#[test]
fn an_upgraded_project_gets_no_evals() {
    // A 0.5 project: a configuration with a harness and no `[change]` and no
    // `[evals]`. `--reconfigure` keeps the answer `none`.
    let r = Repo::with_config("\n[[roots]]\npath = \"docs\"\n\n[harness]\nlevel = \"lean\"\n");
    r.write("docs/a.md", "# A\n\nText.\n");
    r.commit("start");
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    let shown = out.split("--- spec-dna.toml ---").next().unwrap();
    assert!(
        shown.contains("  14. evals: none (default, writes nothing)\n"),
        "{shown}"
    );
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(!cfg.contains("[evals]"), "{cfg}");
    assert!(!r.dir.join("evals").exists());
    assert!(!out.contains("gh variable set"), "{out}");
    // The policy is `none`, so the parts have no skill. `init` proposes the
    // other parts, because the file has none of the keys.
    assert!(
        cfg.contains("skills = []\nreview-md = true\nprotect-tests = \"fix-branches\"\n"),
        "{cfg}"
    );
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn reconfigure_keeps_the_current_value_of_a_part_key() {
    let r = Repo::with_config(
        "\n[[roots]]\npath = \"docs\"\n\n[harness]\nlevel = \"lean\"\nskills = [\"spec\"]\nreview-md = false\nprotect-tests = \"off\"\n",
    );
    r.write("docs/a.md", "# A\n\nText.\n");
    r.commit("start");
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains("skills = [\"spec\"]\nreview-md = false\nprotect-tests = \"off\"\n"),
        "{cfg}"
    );
    assert!(r.dir.join(SKILLS[1]).exists());
    assert!(!r.dir.join(SKILLS[0]).exists());
    assert!(!r.dir.join("REVIEW.md").exists());
}

#[test]
fn reconfigure_to_skeleton_writes_the_table_and_the_files_once() {
    let r = Repo::with_config("\n[[roots]]\npath = \"docs\"\n\n[harness]\nlevel = \"lean\"\n");
    r.write("docs/a.md", "# A\n\nText.\n");
    r.commit("start");
    let (code, out, err) = init_with(&r, "evals = \"skeleton\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.read("spec-dna.toml").contains("\n[evals]"));
    for f in STARTER {
        assert!(r.dir.join(f).exists(), "{f}");
    }
    // The answer `skeleton` with a current table keeps its values and writes
    // no file.
    let cfg = r
        .read("spec-dna.toml")
        .replace("min-pass-rate = 0.8", "min-pass-rate = 0.5");
    r.write("spec-dna.toml", &cfg);
    fs::remove_file(r.dir.join("evals/example.json")).unwrap();
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.read("spec-dna.toml").contains("min-pass-rate = 0.5"));
    assert!(!r.dir.join("evals/example.json").exists());
    // The answer `none` removes the table and keeps the files.
    let (code, out, err) = init_with(&r, "evals = \"none\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.read("spec-dna.toml").contains("[evals]"));
    assert!(r.dir.join("evals/check.sh").exists());
    assert!(out.contains("note: ") && out.contains("evals/"), "{out}");
}

#[test]
fn reconfigure_to_the_level_none_keeps_the_current_evals_table_and_files() {
    let r = new_project();
    r.commit("init");
    // Question 14 is not asked for the level `none`, so the current state
    // stays: the preview says `skeleton (default)`.
    let (code, out, err) = init_with(&r, "harness = \"none\"\n", &["--reconfigure", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    let shown = out.split("--- spec-dna.toml ---").next().unwrap();
    assert!(
        shown.contains("  14. evals: skeleton (default) -> [evals]\n"),
        "{shown}"
    );
    let (code, out, err) = init_with(&r, "harness = \"none\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains("\n[evals]") && cfg.contains("[harness]\nlevel = \"none\"\n"),
        "{cfg}"
    );
    for f in STARTER {
        assert!(r.dir.join(f).exists(), "{f}");
    }
    assert!(!r.dir.join("REVIEW.md").exists());
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
    // An answer `skeleton` is valid with a current table, and `none` removes it.
    let (code, out, err) = init_with(
        &r,
        "harness = \"none\"\nevals = \"skeleton\"\n",
        &["--reconfigure"],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.read("spec-dna.toml").contains("\n[evals]"));
    let (code, out, err) = init_with(&r, "evals = \"none\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.read("spec-dna.toml").contains("[evals]"));
    assert!(r.dir.join("evals/check.sh").exists());
}

#[test]
fn a_review_md_with_no_marker_gives_review_md_false() {
    let r = Repo::new();
    r.write("REVIEW.md", "# Our review rules\n");
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(cfg.contains("review-md = false\n"), "{cfg}");
    assert!(out.contains("note: REVIEW.md exists"), "{out}");
    assert_eq!(r.read("REVIEW.md"), "# Our review rules\n");
    // An explicit answer that asks for the file stops with exit code 2.
    let r = Repo::new();
    r.write("REVIEW.md", "# Our review rules\n");
    let (code, out, err) = init_with(&r, "harness-review-md = true\n", &[]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("REVIEW.md"), "{err}");
    assert!(!r.dir.join("spec-dna.toml").exists());
}

#[test]
fn a_skill_with_no_marker_leaves_the_default_skills() {
    let r = Repo::new();
    r.write(SKILLS[1], "---\nname: spec-dna-spec\n---\nMine.\n");
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains("skills = [\"intent\", \"plan\", \"review\"]\n"),
        "{cfg}"
    );
    assert!(out.contains(&format!("note: {}", SKILLS[1])), "{out}");
}

#[test]
fn the_answers_keys_set_the_parts() {
    let r = Repo::new();
    let (code, out, err) = init_with(
        &r,
        "harness-skills = [\"intent\", \"review\"]\nharness-review-md = false\nharness-protect-tests = \"always\"\n",
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains(
            "skills = [\"intent\", \"review\"]\nreview-md = false\nprotect-tests = \"always\"\n"
        ),
        "{cfg}"
    );
    assert!(r.dir.join(SKILLS[0]).exists() && r.dir.join(SKILLS[3]).exists());
    assert!(!r.dir.join(SKILLS[1]).exists() && !r.dir.join("REVIEW.md").exists());
    let settings = r.read(".claude/settings.json");
    assert!(
        settings.contains("spec-dna hook protect-tests --mode always --test"),
        "{settings}"
    );
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_bad_answer_of_the_parts_gives_exit_code_2_and_writes_nothing() {
    for (answers, want) in [
        ("harness-skills = [\"build\"]\n", "unknown skill `build`"),
        (
            "harness-skills = [\"spec\", \"spec\"]\n",
            "`spec` more than once",
        ),
        ("harness-skills = \"spec\"\n", "must be an array of strings"),
        ("harness-review-md = \"yes\"\n", "must be true or false"),
        (
            "harness-protect-tests = \"never\"\n",
            "fix-branches, always or off",
        ),
        ("evals = \"full\"\n", "none or skeleton"),
        (
            "harness = \"none\"\nevals = \"skeleton\"\n",
            "harness level",
        ),
        (
            "harness = \"none\"\nharness-skills = [\"spec\"]\n",
            "harness level",
        ),
    ] {
        let r = Repo::new();
        let (code, out, err) = init_with(&r, answers, &[]);
        assert_eq!(code, 2, "{answers}: {out}{err}");
        assert!(err.contains(want), "{answers}: {err}");
        assert!(!r.dir.join("spec-dna.toml").exists(), "{answers}");
    }
}

#[test]
fn the_level_none_gives_no_evals_and_no_part() {
    let r = Repo::new();
    let (code, out, err) = init_with(&r, "harness = \"none\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(!cfg.contains("[evals]"), "{cfg}");
    assert!(!r.dir.join("evals").exists());
    assert!(!r.dir.join("REVIEW.md").exists());
}

#[test]
fn the_policy_none_gives_no_skill_by_default() {
    let r = Repo::new();
    let (code, out, err) = init_with(&r, "change-policy = \"none\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains("skills = []\nreview-md = true\n"),
        "the skills follow the final policy:\n{cfg}"
    );
    assert!(!r.dir.join(SKILLS[0]).exists());
    assert!(r.dir.join("REVIEW.md").exists());
}

/// The number of the line "adds about N bytes" of `init` output.
fn request_bytes(out: &str) -> usize {
    let l = out
        .lines()
        .find(|l| l.contains("The harness adds about "))
        .unwrap_or_else(|| panic!("no prefix line\n{out}"));
    l.split("adds about ")
        .nth(1)
        .and_then(|r| r.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("{l}"))
}

#[test]
fn the_4_skill_descriptions_add_588_bytes_to_the_prefix_of_each_request() {
    // The sum has the length of the version in the markers, so the test
    // compares 2 runs of the same version.
    let with = {
        let r = Repo::new();
        let (code, out, err) = r.run(&["init", "--non-interactive"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains("the CLAUDE.md line, the agent descriptions and the skill descriptions")
        );
        assert!(out.contains("per-request prefix: "), "{out}");
        request_bytes(&out)
    };
    let without = {
        let r = Repo::new();
        let (code, out, err) = init_with(&r, "harness-skills = []\n", &[]);
        assert_eq!(code, 0, "{out}{err}");
        request_bytes(&out)
    };
    assert_eq!(with, without + 588);
}

#[test]
fn a_committed_new_project_passes_check_all() {
    // `init`, then `check --staged` in a repository with a commit.
    let r = new_project();
    r.commit("adopt");
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn sd918_warns_about_an_incident_with_no_eval_on_a_new_project() {
    let r = new_project();
    r.write("docs/requirements/valves.md", "# Valves\n\nText.\n");
    r.commit("adopt");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("docs/incidents/hose-leak.md", INCIDENT);
    r.commit("incident");
    let (code, out, err) = r.run(&["check"]);
    let found = lines_with(&out, ": warning SD918 ");
    assert_eq!(found.len(), 1, "{out}{err}");
    assert!(found[0].contains(": warning SD918 "), "{found:?}");
    assert_eq!(code, 0, "a warning does not fail check: {out}");
    // An eval file in the same change satisfies the check.
    r.write(
        "evals/hose-leak.json",
        &golden("example.json").replace("\"id\": \"example\"", "\"id\": \"hose-leak\""),
    );
    r.commit("eval");
    let (_, out, _) = r.run(&["check"]);
    assert!(lines_with(&out, ": warning SD918 ").is_empty(), "{out}");
}

#[cfg(unix)]
mod terminal {
    use super::*;

    /// Runs `spec-dna init` on a pseudo-terminal, writes the answers, and
    /// returns the exit code and the output.
    fn on_terminal(dir: &std::path::Path, answers: &[&str]) -> (i32, String) {
        use std::io::{Read, Write};
        use std::os::fd::{FromRawFd, OwnedFd};
        use std::process::{Command, Stdio};
        let (mut master, mut slave) = (0, 0);
        // SAFETY: openpty writes two new file descriptors. The null pointers
        // ask for no name and the default terminal settings.
        let rc = unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
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
        let mut child = Command::new(BIN)
            .arg("init")
            .current_dir(dir)
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave))
            .spawn()
            .expect("binary runs");
        let mut reader = master.try_clone().unwrap();
        let out = std::thread::spawn(move || {
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            while let Ok(n) = reader.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
            }
            buf
        });
        let mut writer = master;
        for line in answers {
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

    #[test]
    fn the_terminal_asks_the_parts_and_the_evals() {
        let r = Repo::new();
        // 1 profile, 2 graph, 3 areas, 4 policy, 5 gate, 6 evidence, 7
        // language, 8 mapping (1 root), 9 budgets, 10 adapters, 11 CI, 12
        // level, 12 parts, 13 models, 14 evals, then the confirmation.
        let answers = [
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "none, none, main",
            "",
            "skills=spec,plan review-md=false protect-tests=always",
            "",
            "none",
            "y",
        ];
        let (code, out) = on_terminal(&r.dir, &answers);
        assert_eq!(code, 0, "{out}");
        for want in [
            "12. Agent harness (none, lean, full) [lean]: ",
            "12. Harness parts (skills=<names or none> review-md=<true or false> protect-tests=<fix-branches, always or off>) [skills=intent,spec,plan,review review-md=true protect-tests=fix-branches]: ",
            "13. Models (role=model[:effort] ...) [",
            "14. Evals (none, skeleton) [skeleton]: ",
        ] {
            assert!(out.contains(want), "{want}:\n{out}");
        }
        let cfg = r.read("spec-dna.toml");
        assert!(
            cfg.contains(
                "skills = [\"spec\", \"plan\"]\nreview-md = false\nprotect-tests = \"always\"\n"
            ),
            "{cfg}"
        );
        assert!(!cfg.contains("[evals]"), "{cfg}");
        assert!(!r.dir.join("evals").exists());
    }

    #[test]
    fn the_level_none_asks_no_part_and_no_evals() {
        let r = Repo::new();
        let answers = [
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "none, none, main",
            "none",
            "y",
        ];
        let (code, out) = on_terminal(&r.dir, &answers);
        assert_eq!(code, 0, "{out}");
        assert!(!out.contains("Harness parts"), "{out}");
        assert!(!out.contains("13. Models"), "{out}");
        assert!(!out.contains("14. Evals"), "{out}");
        assert!(!r.read("spec-dna.toml").contains("[evals]"));
    }

    #[test]
    fn a_bad_parts_word_gives_exit_code_2() {
        let r = Repo::new();
        let answers = [
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "none, none, main",
            "",
            "skills=build",
        ];
        let (code, out) = on_terminal(&r.dir, &answers);
        assert_eq!(code, 2, "{out}");
        assert!(!r.dir.join("spec-dna.toml").exists());
    }
}
