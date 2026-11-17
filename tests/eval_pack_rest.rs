//! Tests of the 35 evals of `brainstorm`, `specify`, `clarify`, `write-plan`,
//! `review`, `finish` and `post-mortem` (ADR 0016, 0.7.0 slice 5, tasks t14 to
//! t17). No test starts an agent. For each eval, the test writes the pack with
//! `eval pack`, makes the temporary copy as the runner does (a copy of the
//! fixture, one commit, the branch of the eval), and runs the checks of the
//! eval file in it. Then it proves 3 things:
//!
//! - The copy as it is fails the eval, so the checks measure work.
//! - The reference solution (`tests/fixtures/eval-solutions/<id>/good`) passes
//!   each check, so the checks can pass.
//! - Each wrong solution (`wrong-<name>`) fails the checks that its file
//!   `wrong-<name>.fails` names. A wrong solution is the trap that a check
//!   guards: a placeholder left, a status set to `approved`, a wrong review
//!   comment applied, a hash that is not the hash of the requirement.
//!
//! A solution is an overlay of files (a folder), a script (`<name>.sh`, run in
//! the copy with `SPEC_DNA_EVAL_BASE` set) or both. The binary under test is
//! first on `PATH`, as `spec-dna` is in a real run.

mod common;

use common::{BIN, Repo};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SOLUTIONS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/eval-solutions");

/// The evals of this file, by skill.
const SKILLS: [&str; 7] = [
    "brainstorm",
    "specify",
    "clarify",
    "write-plan",
    "review",
    "finish",
    "post-mortem",
];

// ---------------------------------------------------------------- JSON

#[derive(Debug, Clone)]
enum J {
    Str(String),
    Num(f64),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
    Other,
}

impl J {
    fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn str(&self) -> &str {
        match self {
            J::Str(s) => s,
            other => panic!("not a string: {other:?}"),
        }
    }
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

impl P<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) {
        self.ws();
        assert_eq!(self.b[self.i], c, "JSON: expected `{}`", c as char);
        self.i += 1;
    }

    fn string(&mut self) -> String {
        self.eat(b'"');
        let mut out = Vec::new();
        loop {
            let c = self.b[self.i];
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = self.b[self.i];
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'u' => {
                            let hex = std::str::from_utf8(&self.b[self.i..self.i + 4]).unwrap();
                            self.i += 4;
                            let ch = char::from_u32(u32::from_str_radix(hex, 16).unwrap())
                                .expect("a scalar value");
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        other => out.push(other),
                    }
                }
                other => out.push(other),
            }
        }
        String::from_utf8(out).expect("UTF-8")
    }

    fn value(&mut self) -> J {
        self.ws();
        match self.b[self.i] {
            b'"' => J::Str(self.string()),
            b'[' => {
                self.i += 1;
                let mut items = Vec::new();
                loop {
                    self.ws();
                    if self.b[self.i] == b']' {
                        self.i += 1;
                        break;
                    }
                    items.push(self.value());
                    self.ws();
                    if self.b[self.i] == b',' {
                        self.i += 1;
                    }
                }
                J::Arr(items)
            }
            b'{' => {
                self.i += 1;
                let mut items = Vec::new();
                loop {
                    self.ws();
                    if self.b[self.i] == b'}' {
                        self.i += 1;
                        break;
                    }
                    let k = self.string();
                    self.eat(b':');
                    items.push((k, self.value()));
                    self.ws();
                    if self.b[self.i] == b',' {
                        self.i += 1;
                    }
                }
                J::Obj(items)
            }
            _ => {
                let start = self.i;
                while self.i < self.b.len() && !b",]} \n".contains(&self.b[self.i]) {
                    self.i += 1;
                }
                let word = std::str::from_utf8(&self.b[start..self.i]).unwrap();
                word.parse().map(J::Num).unwrap_or(J::Other)
            }
        }
    }
}

fn parse(text: &str) -> J {
    P {
        b: text.as_bytes(),
        i: 0,
    }
    .value()
}

// ---------------------------------------------------------------- the pack

/// A project with the pack written by `eval pack`.
struct Pack {
    repo: Repo,
}

impl Pack {
    fn new() -> Pack {
        let repo = Repo::with_config("\n[evals]\n\n[[roots]]\npath = \"docs\"\n");
        repo.write("docs/a.md", "# A\n\nText.\n");
        repo.write(
            "evals/check.sh",
            "file_has() {\n    grep -q -F -- \"$2\" \"$1\"\n}\n",
        );
        repo.commit("start");
        let (code, out, err) = repo.run(&["eval", "pack"]);
        assert_eq!(code, 0, "{out}{err}");
        Pack { repo }
    }

    fn eval(&self, id: &str) -> J {
        parse(&self.repo.read(&format!("evals/pack/{id}.json")))
    }

    fn fixture(&self, id: &str) -> PathBuf {
        self.repo.dir.join(format!("evals/pack/fixtures/{id}"))
    }

    fn check_sh(&self) -> PathBuf {
        self.repo.dir.join("evals/check.sh")
    }
}

fn bin_dir() -> PathBuf {
    Path::new(BIN)
        .parent()
        .expect("the binary has a folder")
        .into()
}

fn path_env() -> String {
    format!(
        "{}:{}",
        bin_dir().display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

fn git_in(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=spec-dna",
            "-c",
            "user.email=eval@spec-dna.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn copy_tree(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for entry in fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let to = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &to);
        } else {
            fs::copy(entry.path(), &to).unwrap();
        }
    }
}

/// The temporary copy of the runner: the fixture, one base commit, and the
/// branch of the eval.
struct Copy {
    dir: PathBuf,
    base: String,
}

impl Copy {
    fn new(pack: &Pack, id: &str, eval: &J) -> Copy {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("spec-dna-eval-copy-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        copy_tree(&pack.fixture(id), &dir);
        git_in(&dir, &["init", "-q", "-b", "main"]);
        git_in(&dir, &["add", "-A"]);
        git_in(
            &dir,
            &["commit", "-q", "--no-verify", "-m", "spec-dna eval base"],
        );
        let base = git_in(&dir, &["rev-parse", "HEAD"]);
        if let Some(branch) = eval.get("branch") {
            git_in(&dir, &["switch", "-q", "-c", branch.str()]);
        }
        Copy { dir, base }
    }

    fn sh(&self, script: &str, check_sh: Option<&Path>) -> (bool, String) {
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(script)
            .current_dir(&self.dir)
            .env("PATH", path_env())
            .env("SPEC_DNA_EVAL_BASE", &self.base)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE");
        if let Some(p) = check_sh {
            cmd.env("SPEC_DNA_CHECK_SH", p);
        }
        let out = cmd.output().expect("sh runs");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        (out.status.success(), text)
    }
}

impl Drop for Copy {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Runs each check as the runner does, and returns the result of each.
fn run_checks(pack: &Pack, copy: &Copy, eval: &J) -> Vec<(String, bool, String)> {
    let J::Arr(checks) = eval.get("checks").expect("checks") else {
        panic!("checks is no array");
    };
    checks
        .iter()
        .map(|c| {
            let script = format!(
                ". \"$SPEC_DNA_CHECK_SH\" || exit 99\n{}",
                c.get("run").unwrap().str()
            );
            let (ok, text) = copy.sh(&script, Some(&pack.check_sh()));
            (c.get("name").unwrap().str().to_string(), ok, text)
        })
        .collect()
}

/// The variants of an eval in the folder of the solutions: `good`, and each
/// `wrong-<name>`. A variant is a folder, a script, or a `.fails` file.
fn variants(id: &str) -> Vec<String> {
    let dir = Path::new(SOLUTIONS).join(id);
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("{id}: no solutions: {e}")) {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        let stem = name
            .strip_suffix(".sh")
            .or_else(|| name.strip_suffix(".fails"))
            .unwrap_or(&name)
            .to_string();
        names.insert(stem);
    }
    names.into_iter().collect()
}

/// Applies a solution to the copy: the files of its folder, then its script.
fn apply(copy: &Copy, id: &str, variant: &str) {
    let dir = Path::new(SOLUTIONS).join(id);
    let overlay = dir.join(variant);
    if overlay.is_dir() {
        copy_tree(&overlay, &copy.dir);
    }
    let script = dir.join(format!("{variant}.sh"));
    if script.is_file() {
        let (ok, text) = copy.sh(&format!(". '{}'", script.display()), None);
        assert!(ok, "{id} {variant}: the script failed: {text}");
    }
}

fn expected_failures(id: &str, variant: &str) -> Vec<String> {
    let file = Path::new(SOLUTIONS)
        .join(id)
        .join(format!("{variant}.fails"));
    fs::read_to_string(&file)
        .unwrap_or_else(|e| panic!("{id} {variant}: no .fails file: {e}"))
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

fn report(results: &[(String, bool, String)]) -> String {
    results
        .iter()
        .map(|(n, ok, t)| {
            format!(
                "{} {n}{}",
                if *ok { "ok  " } else { "FAIL" },
                if *ok {
                    String::new()
                } else {
                    format!(": {}", t.trim())
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Proves the 3 things of the module doc for the 5 evals of one skill.
fn prove_skill(skill: &str) {
    let pack = Pack::new();
    for n in 1..=5 {
        let id = format!("{skill}-{n}");
        let eval = pack.eval(&id);
        let names: Vec<String> = match eval.get("checks") {
            Some(J::Arr(c)) => c
                .iter()
                .map(|c| c.get("name").unwrap().str().to_string())
                .collect(),
            _ => panic!("{id}: no checks"),
        };

        // The copy as it is fails the eval, and its first check (the file of
        // the artifact) fails. A check that guards a trap may pass.
        let copy = Copy::new(&pack, &id, &eval);
        let results = run_checks(&pack, &copy, &eval);
        assert!(
            !results[0].1,
            "{id}: the first check passes on the fixture\n{}",
            report(&results)
        );
        assert!(
            results.iter().any(|r| !r.1),
            "{id}: the fixture passes the eval"
        );
        drop(copy);

        let vs = variants(&id);
        assert!(
            vs.contains(&"good".to_string()),
            "{id}: no reference solution"
        );
        assert!(
            vs.iter().any(|v| v.starts_with("wrong-")),
            "{id}: no wrong solution"
        );
        for v in vs {
            let copy = Copy::new(&pack, &id, &eval);
            apply(&copy, &id, &v);
            let results = run_checks(&pack, &copy, &eval);
            if v == "good" {
                assert!(
                    results.iter().all(|r| r.1),
                    "{id}: the reference solution fails a check\n{}",
                    report(&results)
                );
            } else {
                let fails = expected_failures(&id, &v);
                assert!(!fails.is_empty(), "{id} {v}: no check named");
                for name in &fails {
                    assert!(names.contains(name), "{id} {v}: no check `{name}`");
                    let r = results.iter().find(|r| &r.0 == name).unwrap();
                    assert!(
                        !r.1,
                        "{id} {v}: the check `{name}` passes\n{}",
                        report(&results)
                    );
                }
            }
        }
    }
}

#[test]
fn the_5_evals_of_brainstorm_fail_the_fixture_and_pass_the_reference() {
    prove_skill("brainstorm");
}

#[test]
fn the_5_evals_of_specify_fail_the_fixture_and_pass_the_reference() {
    prove_skill("specify");
}

#[test]
fn the_5_evals_of_clarify_fail_the_fixture_and_pass_the_reference() {
    prove_skill("clarify");
}

#[test]
fn the_5_evals_of_write_plan_fail_the_fixture_and_pass_the_reference() {
    prove_skill("write-plan");
}

#[test]
fn the_5_evals_of_review_fail_the_fixture_and_pass_the_reference() {
    prove_skill("review");
}

#[test]
fn the_5_evals_of_finish_fail_the_fixture_and_pass_the_reference() {
    prove_skill("finish");
}

#[test]
fn the_5_evals_of_post_mortem_fail_the_fixture_and_pass_the_reference() {
    prove_skill("post-mortem");
}

// ---------------------------------------------------------------- the shape

#[test]
fn each_eval_has_the_budget_the_tools_and_the_runs_of_the_design() {
    let pack = Pack::new();
    for skill in SKILLS {
        for n in 1..=5 {
            let id = format!("{skill}-{n}");
            let e = pack.eval(&id);
            let J::Num(turns) = e.get("max-turns").expect("max-turns") else {
                panic!("{id}: max-turns")
            };
            assert!((15.0..=30.0).contains(turns), "{id}: max-turns {turns}");
            let J::Num(budget) = e.get("max-budget-usd").expect("budget") else {
                panic!("{id}: budget")
            };
            assert!((1.5..=2.0).contains(budget), "{id}: budget {budget}");
            let J::Num(runs) = e.get("runs").expect("runs") else {
                panic!("{id}: runs")
            };
            assert_eq!(*runs, 4.0, "{id}");
            let J::Arr(tools) = e.get("allowed-tools").expect("tools") else {
                panic!("{id}: tools")
            };
            assert!(!tools.is_empty() && tools.len() <= 9, "{id}: tools");
            for t in tools {
                let t = t.str();
                // A tool that runs any command is not minimal.
                assert!(
                    t != "Bash" && t != "Bash(*)" && t != "Bash(*:*)",
                    "{id}: {t}"
                );
            }
            assert!(e.get("branch").is_some(), "{id}: no branch");
            assert!(
                e.get("task").unwrap().str().len() > 60,
                "{id}: the task is short"
            );
        }
    }
}

#[test]
fn each_fixture_is_a_small_spec_dna_project_with_an_agents_file_that_names_no_skill() {
    let pack = Pack::new();
    for skill in SKILLS {
        for n in 1..=5 {
            let id = format!("{skill}-{n}");
            let dir = pack.fixture(&id);
            let toml = fs::read_to_string(dir.join("spec-dna.toml")).unwrap();
            assert!(toml.contains("profile = \"standard\""), "{id}");
            let agents = fs::read_to_string(dir.join("AGENTS.md")).unwrap();
            for name in SKILLS
                .iter()
                .chain(["execute", "tdd", "verify", "debug"].iter())
            {
                assert!(
                    !agents.to_lowercase().contains(&format!("skill {name}"))
                        && !agents.contains("spec-dna skill"),
                    "{id}: AGENTS.md names a skill"
                );
            }
            // No harness: the arm `with` adds the skill folder.
            assert!(!dir.join(".claude").exists(), "{id}: has .claude");
            assert!(!dir.join("CLAUDE.md").exists(), "{id}: has CLAUDE.md");
        }
    }
}
