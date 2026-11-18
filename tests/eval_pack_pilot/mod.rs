//! Proofs of the evals of the pack that have a reference solution: the pilot
//! evals of `tdd` and `debug`, the harder evals of `execute` and `verify`, and
//! the 2 evals of `agents-line`. For each eval, `tests/fixtures/eval-solutions/<id>/` has one
//! folder for each scenario:
//!
//! - `expect` says what the checks must give. `all-pass`: each check passes.
//!   `fail: <name>...`: each named check fails. `only-fail: <name>...`: the
//!   named checks fail and each other check passes, so a wrong solution is
//!   refused for the one reason that its name gives. A line `pass: <name>...`
//!   can follow: each named check passes. A line that starts with `#` is a
//!   comment.
//! - `solve.sh` is the work that the stub `claude` does in the temporary copy.
//!   It reads `$SOL` (its own folder) and `$REF` (the folder `reference`). A
//!   scenario with no `solve.sh` leaves the fixture as it is.
//!
//! The scenario `untouched` proves that the checks fail on the fixture. The
//! scenario `reference` proves that the checks can pass. A scenario with
//! `all-pass` is another form that the checks accept, for example the red
//! commit of `tdd-1` with the subject `test: <text> (red)` and a commit body.
//! Each other scenario is a wrong solution that a named check must refuse. The test runs the real
//! runner `eval run`, so the checks get the environment that they get in a
//! real run. No test starts an agent.
#![cfg(unix)]

use crate::common::{BIN, Repo, git, git_out, path_with, stub_bin};
use std::fs;
use std::path::{Path, PathBuf};

const STUB: &str = r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "2.1.325 (Claude Code)"; exit 0; fi
if [ -n "$SOLVE" ]; then
    sh "$SOLVE" || { echo "the solution failed" >&2; exit 9; }
fi
echo '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.1,"result":"ok"}'
"#;

fn solutions() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/eval-solutions")
}

/// A project with the pack, a stub `claude` and a temporary folder.
struct Lab {
    repo: Repo,
    bin: PathBuf,
    tmp: Repo,
}

impl Lab {
    fn new() -> Lab {
        let repo =
            Repo::with_config("\n[evals]\nmax-total-usd = 50\n\n[[roots]]\npath = \"docs\"\n");
        repo.write("docs/a.md", "# A\n\nText.\n");
        repo.write(
            "evals/check.sh",
            "file_has() {\n    grep -q -F -- \"$2\" \"$1\"\n}\n",
        );
        repo.commit("start");
        let (code, out, err) = repo.run(&["eval", "pack"]);
        assert_eq!(code, 0, "{out}{err}");
        let tmp = Repo::empty();
        let bin = stub_bin(&tmp.dir.join("bin"), "claude", STUB);
        Lab {
            repo,
            bin: bin.parent().unwrap().to_path_buf(),
            tmp,
        }
    }

    /// The ids of the pack that belong to `skill`, in order.
    fn ids(&self, skill: &str) -> Vec<String> {
        let mut ids: Vec<String> = fs::read_dir(self.repo.dir.join("evals/pack"))
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter_map(|n| n.strip_suffix(".json").map(String::from))
            .filter(|id| id.rsplit_once('-').is_some_and(|(s, _)| s == skill))
            .collect();
        ids.sort();
        ids
    }

    /// Runs the eval `id` once with the stub, and returns the exit code of
    /// the stub and the result of each check.
    fn run(&self, id: &str, scenario: &Path) -> (i64, Vec<(String, bool)>) {
        let json = fs::read_to_string(self.repo.dir.join(format!("evals/pack/{id}.json"))).unwrap();
        self.repo.write(&format!("evals/{id}.json"), &json);
        let path = path_with(&self.bin);
        let solve = scenario.join("solve.sh");
        let solve = if solve.exists() {
            solve.to_string_lossy().into_owned()
        } else {
            String::new()
        };
        let sol = scenario.to_string_lossy().into_owned();
        let reference = scenario
            .parent()
            .unwrap()
            .join("reference")
            .to_string_lossy()
            .into_owned();
        let tmp = self.tmp.dir.join("tmp");
        fs::create_dir_all(&tmp).unwrap();
        let tmp = tmp.to_string_lossy().into_owned();
        let set = [
            ("PATH", path.as_str()),
            ("TMPDIR", tmp.as_str()),
            ("SOLVE", solve.as_str()),
            ("SOL", sol.as_str()),
            ("REF", reference.as_str()),
            ("GIT_AUTHOR_NAME", "agent"),
            ("GIT_AUTHOR_EMAIL", "agent@example.invalid"),
            ("GIT_COMMITTER_NAME", "agent"),
            ("GIT_COMMITTER_EMAIL", "agent@example.invalid"),
            ("GIT_CONFIG_GLOBAL", "/dev/null"),
            ("GIT_CONFIG_NOSYSTEM", "1"),
        ];
        let (_, out, err) =
            self.repo
                .run_env(&["eval", "run", id, "--runs", "1", "--json"], &set, &[]);
        fs::remove_file(self.repo.dir.join(format!("evals/{id}.json"))).unwrap();
        let exit = out
            .split("\"claude_exit\":")
            .nth(1)
            .and_then(|t| t.split(',').next())
            .and_then(|t| t.parse::<i64>().ok())
            .unwrap_or_else(|| panic!("{id}: no result: {out}{err}"));
        (exit, checks_of(&out))
    }
}

/// The name and the result of each check in the JSON of `eval run`.
fn checks_of(json: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut rest = json;
    while let Some(i) = rest.find("{\"name\":\"") {
        rest = &rest[i + 9..];
        let end = rest.find('"').unwrap();
        let name = rest[..end].to_string();
        rest = &rest[end..];
        out.push((name, rest.starts_with("\",\"passed\":true")));
    }
    out
}

/// Runs each scenario of each eval of `skill`.
fn prove(skill: &str) {
    prove_n(skill, 5);
}

/// Runs each scenario of each of the `n` evals of `skill`.
fn prove_n(skill: &str, n: usize) {
    let lab = Lab::new();
    let ids = lab.ids(skill);
    assert_eq!(ids.len(), n, "the pack needs {n} evals of {skill}: {ids:?}");
    let mut seen_reference = 0;
    for id in &ids {
        let dir = solutions().join(id);
        let mut scenarios: Vec<PathBuf> = fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{id}: no solutions: {e}"))
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_dir())
            .collect();
        scenarios.sort();
        let names: Vec<String> = scenarios
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(
            names.iter().any(|n| n == "untouched"),
            "{id}: no scenario untouched"
        );
        assert!(
            names.iter().any(|n| n == "reference"),
            "{id}: no scenario reference"
        );
        assert!(names.len() >= 3, "{id}: no wrong solution: {names:?}");
        seen_reference += 1;
        for scenario in scenarios {
            let name = scenario.file_name().unwrap().to_string_lossy().into_owned();
            let expect = fs::read_to_string(scenario.join("expect")).unwrap();
            let (exit, checks) = lab.run(id, &scenario);
            assert!(!checks.is_empty(), "{id}/{name}: no check ran");
            if std::env::var_os("PILOT_VERBOSE").is_some() {
                let line: Vec<String> = checks.iter().map(|c| format!("{}={}", c.0, c.1)).collect();
                eprintln!("{id}/{name}: exit {exit}: {}", line.join(" "));
            }
            let has_solution = scenario.join("solve.sh").exists();
            if has_solution {
                assert_eq!(exit, 0, "{id}/{name}: the solution failed");
            }
            let expect: String = expect
                .lines()
                .filter(|l| !l.starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n");
            let expect = expect.trim();
            if expect == "all-pass" {
                let failed: Vec<&str> = checks
                    .iter()
                    .filter(|c| !c.1)
                    .map(|c| c.0.as_str())
                    .collect();
                assert!(
                    failed.is_empty(),
                    "{id}/{name}: these checks fail: {failed:?}"
                );
            } else {
                let mut lines = expect.lines();
                let first = lines
                    .next()
                    .unwrap_or_else(|| panic!("{id}/{name}: bad expect"));
                let exact = first.starts_with("only-fail:");
                let wanted = first
                    .strip_prefix("only-fail:")
                    .or_else(|| first.strip_prefix("fail:"))
                    .unwrap_or_else(|| panic!("{id}/{name}: bad expect"));
                if let Some(kept) = lines.next() {
                    let kept = kept
                        .strip_prefix("pass:")
                        .unwrap_or_else(|| panic!("{id}/{name}: bad expect"));
                    for want in kept.split_whitespace() {
                        let got = checks.iter().find(|c| c.0 == want);
                        assert!(got.is_some(), "{id}/{name}: no check {want}: {checks:?}");
                        assert!(
                            got.unwrap().1,
                            "{id}/{name}: the check {want} fails: {checks:?}"
                        );
                    }
                }
                if exact {
                    let mut failed: Vec<&str> = checks
                        .iter()
                        .filter(|c| !c.1)
                        .map(|c| c.0.as_str())
                        .collect();
                    let mut want: Vec<&str> = wanted.split_whitespace().collect();
                    failed.sort_unstable();
                    want.sort_unstable();
                    assert_eq!(failed, want, "{id}/{name}: the failed checks differ");
                }
                for want in wanted.split_whitespace() {
                    let got = checks.iter().find(|c| c.0 == want);
                    assert!(got.is_some(), "{id}/{name}: no check {want}: {checks:?}");
                    assert!(
                        !got.unwrap().1,
                        "{id}/{name}: the check {want} passes: {checks:?}"
                    );
                }
            }
        }
    }
    assert_eq!(seen_reference, ids.len());
}

#[test]
fn the_tdd_evals_fail_on_the_fixture_and_pass_with_the_reference_solution() {
    prove("tdd");
}

#[test]
fn the_debug_evals_fail_on_the_fixture_and_pass_with_the_reference_solution() {
    prove("debug");
}

#[test]
fn the_verify_evals_fail_on_the_fixture_and_pass_with_the_reference_solution() {
    prove("verify");
}

/// The paths that SD913 names in the output of `spec-dna check`.
fn outside_plan(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|l| l.contains(" SD913 outside-plan: `"))
        .filter_map(|l| l.split('`').nth(1))
        .map(String::from)
        .collect()
}

#[test]
fn the_reference_of_each_verify_eval_keeps_inside_the_files_of_its_plan() {
    // The plan of a fixture must list each file that the reference edits. If
    // not, the check of the agent reports SD913 for the right work, and no
    // solution can be clean. Only `RESULT.md` is outside: the task asks for it.
    let lab = Lab::new();
    let ids = lab.ids("verify");
    assert_eq!(ids.len(), 5, "{ids:?}");
    let mut problems = Vec::new();
    for id in ids {
        let work = lab.tmp.dir.join(format!("scope-{id}"));
        copy_tree(
            &lab.repo.dir.join(format!("evals/pack/fixtures/{id}")),
            &work,
        );
        git(&work, &["init", "-q", "-b", "main"]);
        git(&work, &["add", "-A"]);
        git(&work, &["commit", "-q", "--no-verify", "-m", "base"]);
        let base = git_out(&work, &["rev-parse", "HEAD"]);
        let json = fs::read_to_string(lab.repo.dir.join(format!("evals/pack/{id}.json"))).unwrap();
        if let Some(branch) = json
            .split("\"branch\": \"")
            .nth(1)
            .and_then(|t| t.split('"').next())
        {
            git(&work, &["switch", "-q", "-c", branch]);
        }
        let reference = solutions().join(&id).join("reference");
        let status = std::process::Command::new("sh")
            .arg(reference.join("solve.sh"))
            .current_dir(&work)
            .env("SOL", &reference)
            .env("REF", &reference)
            .env("SPEC_DNA_EVAL_BASE", &base)
            .status()
            .unwrap();
        assert!(status.success(), "{id}: the reference solution failed");
        git(&work, &["add", "-A"]);
        let out = std::process::Command::new(BIN)
            .args(["check", "--base", &base])
            .current_dir(&work)
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(
            text.contains("spec-dna check: "),
            "{id}: no check ran: {text}"
        );
        let outside: Vec<String> = outside_plan(&text)
            .into_iter()
            .filter(|p| p != "RESULT.md")
            .collect();
        if !outside.is_empty() {
            problems.push(format!("{id}: {outside:?}"));
        }
    }
    assert!(
        problems.is_empty(),
        "the reference edits files that the plan does not list: {problems:?}"
    );
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

#[test]
fn the_execute_evals_fail_on_the_fixture_and_pass_with_the_reference_solution() {
    prove("execute");
}

#[test]
fn the_agents_line_evals_fail_on_the_fixture_and_pass_with_the_reference_solution() {
    prove_n("agents-line", 2);
}

/// The number after `"<key>": ` in the text of an eval file.
fn number(text: &str, key: &str) -> f64 {
    let at = text
        .find(&format!("\"{key}\": "))
        .unwrap_or_else(|| panic!("no key {key}"));
    let rest = &text[at + key.len() + 4..];
    let end = rest.find([',', '\n']).unwrap();
    rest[..end].trim().parse().unwrap()
}

#[test]
fn the_pilot_evals_keep_the_limits_of_the_design_and_name_no_skill_in_the_task() {
    let lab = Lab::new();
    const NAMES: [&str; 11] = [
        "brainstorm",
        "specify",
        "clarify",
        "write-plan",
        "execute",
        "tdd",
        "review",
        "verify",
        "finish",
        "debug",
        "post-mortem",
    ];
    for skill in ["execute", "tdd", "verify", "debug"] {
        for id in lab.ids(skill) {
            let text =
                fs::read_to_string(lab.repo.dir.join(format!("evals/pack/{id}.json"))).unwrap();
            let turns = number(&text, "max-turns");
            let budget = number(&text, "max-budget-usd");
            assert!((12.0..=40.0).contains(&turns), "{id}: {turns} turns");
            // The runs of `verify` reach 1.49 USD, and 2 failed at 1.55 USD
            // against a budget of 1.5, so its group has one higher budget.
            let top = if skill == "verify" { 2.5 } else { 2.0 };
            assert!((1.5..=top).contains(&budget), "{id}: {budget} USD");
            if skill == "verify" {
                assert_eq!(budget, 2.5, "{id}: the group verify has one budget");
            }
            let task = text
                .split("\"task\": \"")
                .nth(1)
                .and_then(|t| t.split("\",\n").next())
                .unwrap_or_else(|| panic!("{id}: no task"))
                .to_lowercase();
            for name in NAMES {
                assert!(!task.contains(name), "{id}: the task names `{name}`");
            }
            assert!(
                !task.contains("spec-dna") && !task.contains("skill"),
                "{id}"
            );
            // The fixture has no instruction file that names a skill.
            let dir = lab.repo.dir.join(format!("evals/pack/fixtures/{id}"));
            for file in ["AGENTS.md", "CLAUDE.md", "README.md"] {
                let text = fs::read_to_string(dir.join(file))
                    .unwrap_or_default()
                    .to_lowercase();
                for name in NAMES {
                    assert!(!text.contains(name), "{id}/{file} names `{name}`");
                }
                assert!(!text.contains("spec-dna"), "{id}/{file}");
            }
            assert!(dir.join("test.sh").exists(), "{id}: no test.sh");
        }
    }
}

#[test]
fn the_task_of_an_execute_eval_names_no_rule_of_its_skill() {
    // The skill must add the work. If the task gave the isolation, the scope of
    // `Files`, the review passes or the order of the `Verify:` and the tick, the
    // arm `without` could do it as well. A task states what a person asks: the
    // next task of the plan, the commit and the tick of the box. Only the task
    // that needs a stop (`execute-6`) has a file for a question.
    let lab = Lab::new();
    const RULES: [&str; 13] = [
        "subagent",
        "fresh",
        "compliance",
        "quality",
        "worktree",
        "branch",
        "isolat",
        "scope",
        "files:",
        "only the files",
        "review",
        "pass 1",
        "first open",
    ];
    for id in lab.ids("execute") {
        let text = fs::read_to_string(lab.repo.dir.join(format!("evals/pack/{id}.json"))).unwrap();
        let task = text
            .split("\"task\": \"")
            .nth(1)
            .and_then(|t| t.split("\",\n").next())
            .unwrap_or_else(|| panic!("{id}: no task"))
            .to_lowercase();
        for rule in RULES {
            assert!(!task.contains(rule), "{id}: the task says `{rule}`");
        }
        assert!(task.contains("next open task"), "{id}");
        assert_eq!(task.contains("status.md"), id == "execute-6", "{id}");
        // The ids of the plain tasks ask for the tick in the commit, as the
        // task of the old `execute-1` did.
        if ["execute-7", "execute-9", "execute-10"].contains(&id.as_str()) {
            assert!(task.contains("tick of its box"), "{id}: {task}");
        }
    }
}
