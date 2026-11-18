//! Proofs of the evals of the pack that have a reference solution: the pilot
//! evals (`execute`, `tdd` and `debug`), the harder evals of `verify` and the 2
//! evals of `agents-line`. For each eval, `tests/fixtures/eval-solutions/<id>/` has one
//! folder for each scenario:
//!
//! - `expect` says what the checks must give. `all-pass`: each check passes.
//!   `fail: <name>...`: each named check fails.
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

use crate::common::{Repo, path_with, stub_bin};
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
                let wanted = expect
                    .strip_prefix("fail:")
                    .unwrap_or_else(|| panic!("{id}/{name}: bad expect"));
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
            assert!((1.5..=2.0).contains(&budget), "{id}: {budget} USD");
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
