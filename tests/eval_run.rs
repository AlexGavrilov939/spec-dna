//! Tests of `spec-dna eval run` (step 4, S6). A stub `claude` replaces the real
//! program, so no test calls the network. Each test uses a temporary
//! repository and a temporary `TMPDIR`. No test reads or writes a file of this
//! repository.
//!
//! The stub appends each argument of each call on its own line to
//! `$STUB_LOG`, and a line `---` after each call. For `--version` it prints
//! `2.1.325 (Claude Code)`. Otherwise it runs `$STUB_EDIT` in its folder,
//! writes the list of its files to `$STUB_FILES` and prints a result.
#![cfg(unix)]

mod common;

use common::{Repo, VERSION, path_with, stub_bin};
use std::fs;
use std::path::{Path, PathBuf};

const STUB: &str = r#"#!/bin/sh
{ for a in "$@"; do printf '%s\n' "$a"; done; echo '---'; } >> "$STUB_LOG"
if [ "$1" = "--version" ]; then echo "2.1.325 (Claude Code)"; exit 0; fi
{ echo "CLAUDE_PROJECT_DIR=${CLAUDE_PROJECT_DIR-unset}"; echo "CLAUDECODE=${CLAUDECODE-unset}"; } >> "$STUB_ENV"
eval "$STUB_EDIT"
find . \( -type f -o -type l \) -not -path './.git/*' | sort > "$STUB_FILES"
if [ "$STUB_SHAPE" = array ]; then
    printf '[{"type":"system","subtype":"init"},{"type":"result","subtype":"success","is_error":false,"total_cost_usd":%s,"result":"STUB-OUTPUT-TEXT"}]\n' "$STUB_COST"
elif [ "$STUB_SHAPE" = turns ]; then
    printf '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":%s,"num_turns":7,"modelUsage":{"claude-b":{"inputTokens":10,"outputTokens":20,"cacheReadInputTokens":30,"cacheCreationInputTokens":40},"claude-a":{"inputTokens":1,"outputTokens":2}},"result":"STUB-OUTPUT-TEXT"}\n' "$STUB_COST"
elif [ "$STUB_SHAPE" = badturns ]; then
    printf '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":%s,"num_turns":-3,"result":"x"}\n' "$STUB_COST"
elif [ "$STUB_SHAPE" = garbage ]; then
    printf 'not json STUB-OUTPUT-TEXT\n'
elif [ -n "$STUB_COST" ]; then
    printf '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":%s,"result":"STUB-OUTPUT-TEXT"}\n' "$STUB_COST"
else
    printf '{"type":"result","subtype":"success","is_error":false,"result":"STUB-OUTPUT-TEXT"}\n'
fi
exit "${STUB_EXIT:-0}"
"#;

const CHECK_SH: &str = r#"file_has() {
    grep -q -F -- "$2" "$1"
}
tests_unchanged() {
    [ "$#" -gt 0 ] || set -- tests
    git diff --quiet "$SPEC_DNA_EVAL_BASE" -- "$@"
}
"#;

const DONE: &str = "echo 'status: done' > notes.txt";

/// The text of an eval file.
struct Ev {
    id: &'static str,
    task: &'static str,
    tools: &'static str,
    turns: u32,
    budget: &'static str,
    runs: u32,
    pass_at: Option<u32>,
    branch: Option<&'static str>,
    change: Option<&'static str>,
    checks: Vec<(&'static str, &'static str)>,
}

fn ev(id: &'static str) -> Ev {
    Ev {
        id,
        task: "Change the word draft to done.",
        tools: r#""Read", "Edit""#,
        turns: 5,
        budget: "1",
        runs: 1,
        pass_at: None,
        branch: None,
        change: None,
        checks: vec![("notes-done", "file_has notes.txt 'status: done'")],
    }
}

impl Ev {
    fn json(&self) -> String {
        let mut s = format!(
            "{{\"id\":\"{}\",\"task\":\"{}\",\"fixture\":\"evals/fixtures/example\",\"allowed-tools\":[{}],\"max-turns\":{},\"max-budget-usd\":{},\"runs\":{}",
            self.id, self.task, self.tools, self.turns, self.budget, self.runs
        );
        if let Some(p) = self.pass_at {
            s.push_str(&format!(",\"pass-at\":{p}"));
        }
        if let Some(b) = self.branch {
            s.push_str(&format!(",\"branch\":\"{b}\""));
        }
        if let Some(c) = self.change {
            s.push_str(&format!(",\"change\":\"{c}\""));
        }
        let checks: Vec<String> = self
            .checks
            .iter()
            .map(|(n, r)| {
                format!(
                    "{{\"name\":\"{n}\",\"run\":\"{}\"}}",
                    r.replace('"', "\\\"")
                )
            })
            .collect();
        s.push_str(&format!(",\"checks\":[{}]}}", checks.join(",")));
        s
    }
}

/// A project with an eval folder, and a stub `claude`.
struct Fx {
    repo: Repo,
    /// Holds the stub, its logs and the temporary folder of the runner.
    stubs: Repo,
}

impl Fx {
    fn new(head: &str, evals: &[Ev]) -> Fx {
        let repo = Repo::with_config(&format!("{head}\n[[roots]]\npath = \"docs\"\n"));
        repo.write("docs/terms.md", "# Terms\n\nA valve closes a pipe.\n");
        repo.write("evals/check.sh", CHECK_SH);
        repo.write("evals/fixtures/example/notes.txt", "status: draft\n");
        for e in evals {
            repo.write(&format!("evals/{}.json", e.id), &e.json());
        }
        repo.commit("start");
        let stubs = Repo::empty();
        stub_bin(&stubs.dir.join("bin"), "claude", STUB);
        fs::create_dir_all(stubs.dir.join("tmp")).unwrap();
        Fx { repo, stubs }
    }

    /// `[evals]` with the default keys, and the evals.
    fn with(evals: &[Ev]) -> Fx {
        Fx::new("[evals]\n", evals)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.stubs.dir.join(name)
    }

    fn tmp(&self) -> PathBuf {
        self.path("tmp")
    }

    fn run(&self, args: &[&str]) -> (i32, String, String) {
        self.run_env(args, &[])
    }

    /// Runs the binary with the stub first on `PATH`.
    fn run_env(&self, args: &[&str], extra: &[(&str, &str)]) -> (i32, String, String) {
        let bin = path_with(&self.path("bin"));
        let strs = |n: &str| self.path(n).to_string_lossy().into_owned();
        let (log, files, env, tmp) = (strs("log"), strs("files"), strs("env"), strs("tmp"));
        let mut set: Vec<(&str, &str)> = vec![
            ("PATH", &bin),
            ("STUB_LOG", &log),
            ("STUB_FILES", &files),
            ("STUB_ENV", &env),
            ("STUB_COST", "0.1"),
            ("STUB_EDIT", DONE),
            ("TMPDIR", &tmp),
        ];
        set.extend_from_slice(extra);
        let mut full = vec!["eval", "run"];
        full.extend_from_slice(args);
        self.repo.run_env(&full, &set, &[])
    }

    /// The calls of the stub: the arguments of each call.
    fn calls(&self) -> Vec<Vec<String>> {
        let text = fs::read_to_string(self.path("log")).unwrap_or_default();
        let mut calls = Vec::new();
        let mut cur = Vec::new();
        for line in text.lines() {
            if line == "---" {
                calls.push(std::mem::take(&mut cur));
            } else {
                cur.push(line.to_string());
            }
        }
        calls
    }

    /// The calls that run a task, after the `--version` call. The first call
    /// must be `--version`, so a wrong `PATH` fails here.
    fn runs(&self) -> Vec<Vec<String>> {
        let calls = self.calls();
        assert_eq!(calls.first(), Some(&vec!["--version".to_string()]));
        calls[1..].to_vec()
    }

    fn files(&self) -> String {
        fs::read_to_string(self.path("files")).unwrap()
    }

    /// Asserts that the runner left no folder in the temporary folder.
    fn assert_clean(&self) {
        let left: Vec<_> = fs::read_dir(self.tmp())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert!(left.is_empty(), "the runner left {left:?}");
    }
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap()
}

#[test]
fn a_run_that_passes() {
    let fx = Fx::with(&[ev("example")]);
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with("eval run: 1 evals, 1 runs, estimate 1.00 USD, cap 20.00 USD\n"),
        "{out}"
    );
    assert!(
        out.contains("example: 1 of 1 runs passed, pass-at 1: pass, 0.10 USD\n"),
        "{out}"
    );
    assert!(
        out.contains("pass rate 100.0 percent (1 of 1 evals), min-pass-rate 80.0 percent: pass\n"),
        "{out}"
    );
    assert!(out.ends_with("spend 0.10 USD\n"), "{out}");
    assert_eq!(fx.runs().len(), 1);
    fx.assert_clean();
}

#[test]
fn the_command_line_of_claude_has_the_exact_order_and_no_bare() {
    let fx = Fx::with(&[ev("example")]);
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 0, "{out}{err}");
    let want: Vec<&str> = vec![
        "-p",
        "Change the word draft to done.",
        "--allowedTools",
        "Read,Edit",
        "--output-format",
        "json",
        "--max-turns",
        "5",
        "--max-budget-usd",
        "1",
        "--permission-mode",
        "dontAsk",
    ];
    assert_eq!(fx.runs(), vec![want.clone()]);
    // `--isolate` adds `--setting-sources project --strict-mcp-config` at the end, and only then.
    let (code, out, err) = fx.run(&["--isolate"]);
    assert_eq!(code, 0, "{out}{err}");
    let mut isolated = want;
    isolated.extend(["--setting-sources", "project", "--strict-mcp-config"]);
    assert_eq!(
        fx.runs().last(),
        Some(&isolated.iter().map(|s| s.to_string()).collect())
    );
    for call in fx.calls() {
        assert!(!call.iter().any(|a| a.contains("bare")), "{call:?}");
    }
}

#[test]
fn a_tool_that_starts_with_a_dash_gives_exit_code_3_and_no_process() {
    let mut e = ev("example");
    e.tools = r#""--bare", "--dangerously-skip-permissions""#;
    let fx = Fx::with(&[e]);
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(
        err.contains("evals/example.json") && err.contains("allowed-tools"),
        "{err}"
    );
    assert!(!fx.path("log").exists(), "the stub ran");
}

#[test]
fn the_tools_are_one_argument_so_no_later_word_is_a_flag() {
    let mut e = ev("example");
    e.tools = r#""Read", "Bash(cargo test*)""#;
    let fx = Fx::with(&[e]);
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 0, "{out}{err}");
    let run = &fx.runs()[0];
    let at = run.iter().position(|a| a == "--allowedTools").unwrap();
    assert_eq!(run[at + 1], "Read,Bash(cargo test*)", "{run:?}");
    assert_eq!(run[at + 2], "--output-format", "{run:?}");
}

#[test]
fn an_empty_tool_list_gives_no_allowed_tools_flag() {
    let mut e = ev("example");
    e.tools = "";
    let fx = Fx::with(&[e]);
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 0, "{out}{err}");
    let run = &fx.runs()[0];
    assert!(!run.contains(&"--allowedTools".to_string()), "{run:?}");
    assert_eq!(run[2], "--output-format");
}

#[test]
fn a_check_that_fails_fails_the_run() {
    let fx = Fx::with(&[ev("example")]);
    let (code, out, err) = fx.run_env(&[], &[("STUB_EDIT", "true")]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.contains("example: 0 of 1 runs passed, pass-at 1: fail, 0.10 USD\n"),
        "{out}"
    );
    assert!(
        out.contains("pass rate 0.0 percent (0 of 1 evals), min-pass-rate 80.0 percent: fail\n"),
        "{out}"
    );
    fx.assert_clean();
}

#[test]
fn a_claude_that_exits_with_code_1_fails_the_run_but_the_checks_still_run() {
    let fx = Fx::with(&[ev("example")]);
    let (code, out, err) = fx.run_env(&["--json"], &[("STUB_EXIT", "1")]);
    assert_eq!(code, 1, "{out}{err}");
    // The check passed (the edit was made), and the run failed.
    assert!(out.contains("\"claude_exit\":1"), "{out}");
    assert!(
        out.contains("\"checks\":[{\"name\":\"notes-done\",\"passed\":true}]"),
        "{out}"
    );
    assert!(out.contains("\"passed_runs\":0"), "{out}");
    fx.assert_clean();
}

#[test]
fn an_estimate_above_the_cap_starts_no_process() {
    let mut e = ev("example");
    e.budget = "5";
    e.runs = 3;
    let fx = Fx::with(&[e]);
    let (code, out, err) = fx.run_env(&["--max-total-usd", "10"], &[]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(
        err.contains("15.00 USD") && err.contains("10.00 USD"),
        "{err}"
    );
    assert!(!fx.path("log").exists(), "the stub ran");
    fx.assert_clean();
}

#[test]
fn the_estimate_has_a_tolerance_for_the_rounding_of_a_sum() {
    // 3 x 0.1 is above 0.3 in floating point, and it must pass.
    let mut e = ev("example");
    e.budget = "0.1";
    e.runs = 3;
    let fx = Fx::with(&[e]);
    let (code, out, err) = fx.run(&["--max-total-usd", "0.3"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.runs().len(), 3);
}

#[test]
fn a_spend_above_the_cap_stops_the_next_run() {
    for shape in ["object", "array"] {
        let mut e = ev("example");
        e.runs = 3;
        let fx = Fx::new("[evals]\nmax-total-usd = 3\n", &[e]);
        let (code, out, err) = fx.run_env(&[], &[("STUB_COST", "2.5"), ("STUB_SHAPE", shape)]);
        assert_eq!(code, 3, "{shape}: {out}{err}");
        // The third run did not start.
        assert_eq!(fx.runs().len(), 2, "{shape}");
        assert!(
            out.contains(
                "stopped: the spend 5.00 USD is above max-total-usd 3.00 USD; 1 runs did not start\n"
            ),
            "{shape}: {out}"
        );
        assert!(out.contains("spend 5.00 USD\n"), "{out}");
        assert!(!out.contains("pass rate"), "{out}");
        fx.assert_clean();
    }
}

#[test]
fn a_stop_gives_a_result_with_no_pass_rate_and_an_empty_list_for_an_eval_that_did_not_start() {
    let mut a = ev("a-first");
    a.runs = 2;
    let b = ev("b-second");
    let fx = Fx::new("[evals]\nmax-total-usd = 3\n", &[a, b]);
    let (code, out, err) = fx.run_env(&["--json"], &[("STUB_COST", "2.5")]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(out.contains("\"stopped\":\"max-total-usd\""), "{out}");
    assert!(out.contains("\"pass_rate\":null"), "{out}");
    assert!(out.contains("\"passed\":false,\"evals\""), "{out}");
    assert!(out.contains("\"id\":\"b-second\""), "{out}");
    assert!(out.contains("\"results\":[]"), "{out}");
    assert_eq!(fx.runs().len(), 2);
}

#[test]
fn a_spend_above_the_cap_after_the_last_run_is_not_a_stop() {
    let fx = Fx::new("[evals]\nmax-total-usd = 3\n", &[ev("example")]);
    let (code, out, err) = fx.run_env(&[], &[("STUB_COST", "9")]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("stopped"), "{out}");
}

#[test]
fn a_result_that_is_not_json_gives_a_null_cost() {
    let fx = Fx::with(&[ev("example")]);
    let (code, out, err) = fx.run_env(&["--json"], &[("STUB_SHAPE", "garbage")]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"cost_usd\":null"), "{out}");
    assert!(out.contains("\"subtype\":null"), "{out}");
    // The cap counts the budget of the run.
    assert!(out.contains("\"spend_usd\":1,"), "{out}");
}

#[test]
fn pass_at_counts_the_passed_runs() {
    // 3 runs, and the stub fails the second one.
    let counter = "n=$(cat \"$STUB_COUNT\" 2>/dev/null || echo 0); n=$((n+1)); echo $n > \"$STUB_COUNT\"; [ \"$n\" -eq 2 ] || echo 'status: done' > notes.txt";
    for (pass_at, code_want, word) in [(2, 0, "pass"), (3, 1, "fail"), (1, 0, "pass")] {
        let mut e = ev("example");
        e.runs = 3;
        e.pass_at = Some(pass_at);
        let fx = Fx::with(&[e]);
        let count = fx.path("count");
        let (code, out, err) = fx.run_env(
            &["--min-pass-rate", "1"],
            &[
                ("STUB_EDIT", counter),
                ("STUB_COUNT", count.to_str().unwrap()),
            ],
        );
        assert_eq!(code, code_want, "pass-at {pass_at}: {out}{err}");
        assert!(
            out.contains(&format!(
                "example: 2 of 3 runs passed, pass-at {pass_at}: {word}, 0.30 USD\n"
            )),
            "{out}"
        );
        // Each run runs, also after the pass-at is reached.
        assert_eq!(fx.runs().len(), 3);
    }
}

#[test]
fn a_pass_at_above_the_runs_of_the_flag_counts_as_the_runs() {
    // The file says 3 runs and pass-at 3. `--runs 1` gives min(3, 1) = 1.
    let mut e = ev("example");
    e.runs = 3;
    e.pass_at = Some(3);
    let fx = Fx::with(&[e]);
    let (code, out, err) = fx.run(&["--runs", "1"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("1 of 1 runs passed, pass-at 1: pass"), "{out}");
    assert_eq!(fx.runs().len(), 1);
}

#[test]
fn min_pass_rate_decides_the_exit_code() {
    let mut pass = ev("a-pass");
    pass.task = "pass-me";
    let mut fail = ev("b-fail");
    fail.task = "fail-me";
    let fx = Fx::with(&[pass, fail]);
    let edit = "case \"$2\" in *pass-me*) echo 'status: done' > notes.txt ;; esac";
    for (rate, code_want, word) in [("0.5", 0, "pass"), ("0.51", 1, "fail"), ("0", 0, "pass")] {
        let (code, out, err) = fx.run_env(&["--min-pass-rate", rate], &[("STUB_EDIT", edit)]);
        assert_eq!(code, code_want, "{rate}: {out}{err}");
        assert!(out.contains("a-pass: 1 of 1 runs passed"), "{out}");
        assert!(out.contains("b-fail: 0 of 1 runs passed"), "{out}");
        assert!(out.contains("(1 of 2 evals)"), "{out}");
        assert!(out.contains(&format!(": {word}\nspend")), "{out}");
    }
}

#[test]
fn a_table_min_pass_rate_applies_without_a_flag() {
    let fx = Fx::new("[evals]\nmin-pass-rate = 1\n", &[ev("example")]);
    let (code, out, err) = fx.run_env(&[], &[("STUB_EDIT", "true")]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("min-pass-rate 100.0 percent: fail"), "{out}");
}

#[test]
fn the_runs_flag_replaces_the_runs_of_the_file() {
    let mut e = ev("example");
    e.runs = 3;
    let fx = Fx::with(&[e]);
    let (code, out, err) = fx.run(&["--runs", "2"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.runs().len(), 2);
}

#[test]
fn a_run_with_no_runs_key_uses_the_runs_of_the_table() {
    let fx = Fx::new("[evals]\nruns = 2\n", &[]);
    fx.repo.write(
        "evals/example.json",
        r#"{"id":"example","task":"t","fixture":"evals/fixtures/example","max-budget-usd":1,"checks":[{"name":"c","run":"true"}]}"#,
    );
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.runs().len(), 2);
}

#[test]
fn the_copy_has_the_harness_files_and_the_fixture() {
    let fx = Fx::with(&[ev("example")]);
    let r = &fx.repo;
    r.write("CLAUDE.md", "project\n");
    r.write("AGENTS.md", "agents\n");
    r.write(".claude/agents/verifier.md", "agent\n");
    r.write(".claude/skills/spec-dna-intent/SKILL.md", "skill\n");
    r.write(".claude/settings.json", "{}\n");
    r.write("REVIEW.md", "review\n");
    r.write("README.md", "not copied\n");
    // The fixture also holds a CLAUDE.md. The file of the project replaces it.
    r.write("evals/fixtures/example/CLAUDE.md", "fixture\n");
    r.write("evals/fixtures/example/sub/deep.txt", "deep\n");
    r.commit("harness");
    // The local state of a person, and an ignored folder: never copied.
    r.write(".claude/settings.local.json", "{\"local\":true}\n");
    r.write(".gitignore", ".claude/worktrees/\n");
    r.write(".claude/worktrees/w/x.txt", "ignored\n");
    // An untracked file that git does not ignore is copied.
    r.write(".claude/agents/new.md", "untracked\n");
    let edit = "cat CLAUDE.md > \"$STUB_FILES.claude\"; echo 'status: done' > notes.txt";
    let (code, out, err) = fx.run_env(&[], &[("STUB_EDIT", edit)]);
    assert_eq!(code, 0, "{out}{err}");
    let files = fx.files();
    for want in [
        "./notes.txt",
        "./sub/deep.txt",
        "./CLAUDE.md",
        "./AGENTS.md",
        "./REVIEW.md",
        "./.claude/agents/verifier.md",
        "./.claude/agents/new.md",
        "./.claude/skills/spec-dna-intent/SKILL.md",
        "./.claude/settings.json",
    ] {
        assert!(
            files.lines().any(|l| l == want),
            "{want} is not in\n{files}"
        );
    }
    for not in [
        "settings.local.json",
        "worktrees",
        "README.md",
        "spec-dna.toml",
        "docs/",
        "evals/",
        "check.sh",
    ] {
        assert!(!files.contains(not), "{not} is in\n{files}");
    }
    assert_eq!(read(&fx.path("files.claude")), "project\n");
}

#[test]
fn a_symbolic_link_is_not_copied_and_gives_a_warning() {
    let fx = Fx::with(&[ev("example")]);
    std::os::unix::fs::symlink("notes.txt", fx.repo.dir.join("evals/fixtures/example/link"))
        .unwrap();
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!fx.files().contains("link"), "{}", fx.files());
    assert!(
        err.contains("symbolic link") && err.contains("link"),
        "{err}"
    );
}

#[test]
fn an_untracked_eval_and_an_untracked_fixture_file_are_read() {
    let fx = Fx::with(&[ev("example")]);
    fx.repo.write("evals/fixtures/example/extra.txt", "x\n");
    let mut other = ev("other");
    other.task = "other task";
    fx.repo.write("evals/other.json", &other.json());
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("other: 1 of 1 runs passed"), "{out}");
    assert!(fx.files().contains("./extra.txt"), "{}", fx.files());
}

#[test]
fn the_check_file_of_the_project_is_the_one_that_runs() {
    let fx = Fx::with(&[ev("example")]);
    // The agent writes its own check file in the copy. It has no effect.
    let edit = "mkdir -p evals; echo 'file_has() { return 0; }' > evals/check.sh; echo 'file_has() { return 0; }' > check.sh";
    let (code, out, err) = fx.run_env(&[], &[("STUB_EDIT", edit)]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("example: 0 of 1 runs passed"), "{out}");
}

#[test]
fn a_check_gets_the_check_file_of_the_project_and_the_base_commit() {
    let mut e = ev("example");
    e.checks = vec![(
        "env",
        "test -f \"$SPEC_DNA_CHECK_SH\" && test \"$SPEC_DNA_CHECK_SH\" != \"$PWD/evals/check.sh\" && test -n \"$SPEC_DNA_EVAL_BASE\" && test -z \"$CLAUDE_PROJECT_DIR\"",
    )];
    let fx = Fx::with(&[e]);
    let (code, out, err) = fx.run_env(&[], &[("CLAUDE_PROJECT_DIR", "/somewhere")]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_dir_of_another_name_sets_the_path_of_check_sh() {
    let head = "[evals]\ndir = \"qa/agent\"\n";
    let fx = Fx::new(head, &[]);
    // The check file of `evals/` must not be the one that runs.
    fs::remove_file(fx.repo.dir.join("evals/check.sh")).unwrap();
    fx.repo.write("qa/agent/check.sh", CHECK_SH);
    fx.repo
        .write("qa/agent/fixtures/f/notes.txt", "status: draft\n");
    let json = ev("example")
        .json()
        .replace("evals/fixtures/example", "qa/agent/fixtures/f");
    fx.repo.write("qa/agent/example.json", &json);
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_branch_of_the_eval_is_the_branch_of_the_copy() {
    for (branch, want) in [
        (None, "main"),
        (Some("main"), "main"),
        (Some("fix/x"), "fix/x"),
    ] {
        let mut e = ev("example");
        e.branch = branch;
        let fx = Fx::with(&[e]);
        let edit = format!("git symbolic-ref --short HEAD > \"$STUB_FILES.branch\"; {DONE}");
        let (code, out, err) = fx.run_env(&[], &[("STUB_EDIT", &edit)]);
        assert_eq!(code, 0, "{branch:?}: {out}{err}");
        assert_eq!(read(&fx.path("files.branch")).trim(), want, "{branch:?}");
    }
}

/// The number after `"key":` in a compact JSON text, for the first place of
/// the key.
fn number_after(json: &str, key: &str) -> f64 {
    let at = json
        .find(&format!("\"{key}\":"))
        .unwrap_or_else(|| panic!("no key {key} in {json}"));
    let rest = &json[at + key.len() + 3..];
    let end = rest.find([',', '}']).unwrap_or(rest.len());
    rest[..end].parse().unwrap_or(f64::NAN)
}

// D58: the time, the turns, the tokens and the models of each run.

#[test]
fn a_plain_result_has_the_tokens_the_models_the_turns_and_the_seconds_of_each_run() {
    let fx = Fx::with(&[ev("example")]);
    // The edit takes a moment, so the seconds are above 0 at 1 decimal.
    let edit = format!("sleep 0.3; {DONE}");
    let (code, out, err) = fx.run_env(
        &["--json"],
        &[("STUB_SHAPE", "turns"), ("STUB_EDIT", &edit)],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"turns\":7"), "{out}");
    assert!(
        out.contains("\"tokens\":{\"in\":11,\"out\":22,\"cache_read\":30,\"cache_write\":40}"),
        "{out}"
    );
    assert!(
        out.contains("\"models\":[\"claude-a\",\"claude-b\"]"),
        "{out}"
    );
    let seconds = number_after(&out, "seconds");
    assert!((0.2..60.0).contains(&seconds), "{seconds} in {out}");
    // The seconds have 1 decimal.
    assert_eq!((seconds * 10.0).round() / 10.0, seconds, "{seconds}");
    // The keys are added after the old keys of a result: nothing moves.
    assert!(
        out.contains("\"checks\":[{\"name\":\"notes-done\",\"passed\":true}],\"tokens\""),
        "{out}"
    );
    fx.assert_clean();
}

#[test]
fn a_run_with_no_usage_has_null_turns_tokens_and_no_models() {
    let fx = Fx::with(&[ev("example")]);
    for shape in ["", "badturns", "garbage"] {
        let (code, out, err) = fx.run_env(&["--json"], &[("STUB_SHAPE", shape)]);
        assert_eq!(code, 0, "{shape}: {out}{err}");
        assert!(out.contains("\"turns\":null"), "{shape}: {out}");
        assert!(out.contains("\"tokens\":null"), "{shape}: {out}");
        assert!(out.contains("\"models\":[]"), "{shape}: {out}");
        assert!(out.contains("\"seconds\":"), "{shape}: {out}");
    }
}

// D60: the key `change`.

/// A project with a fixture and a change folder under `evals/`.
fn with_change(e: Ev) -> Fx {
    let fx = Fx::with(&[e]);
    fx.repo.write("evals/changes/example/added.txt", "new\n");
    fx.repo
        .write("evals/changes/example/notes.txt", "status: changed\n");
    fx.repo.commit("change");
    fx
}

#[test]
fn the_files_of_the_change_are_a_commit_on_the_branch_and_a_diff_against_main() {
    let mut e = ev("example");
    e.branch = Some("feature/discount");
    e.change = Some("evals/changes/example");
    e.checks = vec![
        ("sees-added", "file_has diff.txt added.txt"),
        ("sees-notes", "file_has diff.txt notes.txt"),
        ("two-commits", "[ \"$(git rev-list --count HEAD)\" = 2 ]"),
        (
            "clean-tree",
            "[ -z \"$(git status --porcelain | grep -v diff.txt)\" ]",
        ),
        // The base is the commit after the change: the change itself is no
        // diff against it (a check like `tests_unchanged` stays green).
        (
            "base-is-head",
            "[ \"$(git rev-parse HEAD)\" = \"$SPEC_DNA_EVAL_BASE\" ]",
        ),
        (
            "base-is-not-main",
            "[ \"$(git rev-parse main)\" != \"$SPEC_DNA_EVAL_BASE\" ]",
        ),
        (
            "change-is-no-diff-against-base",
            "git diff --quiet \"$SPEC_DNA_EVAL_BASE\" -- notes.txt added.txt",
        ),
    ];
    let fx = with_change(e);
    let edit = "git diff --name-only main > diff.txt; git log -1 --format='%s|%an|%ae|%G?' > \"$STUB_FILES.change\"; git symbolic-ref --short HEAD > \"$STUB_FILES.branch\"; cat notes.txt > \"$STUB_FILES.notes\"";
    let (code, out, err) = fx.run_env(&["--min-pass-rate", "0"], &[("STUB_EDIT", edit)]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("example: 1 of 1 runs passed"), "{out}");
    assert_eq!(read(&fx.path("files.branch")).trim(), "feature/discount");
    // The commit has the neutral identity, no signature and the fixed subject.
    let log = read(&fx.path("files.change"));
    assert_eq!(
        log.trim_end(),
        "spec-dna eval change|spec-dna|eval@spec-dna.invalid|N",
        "the identity of the base commit, and no signature"
    );
    // The change files overwrite the files of the fixture.
    assert_eq!(read(&fx.path("files.notes")), "status: changed\n");
    fx.assert_clean();
}

#[test]
fn an_eval_without_a_change_has_one_commit_and_no_change_files() {
    let mut e = ev("example");
    e.branch = Some("feature/discount");
    e.checks = vec![("one-commit", "[ \"$(git rev-list --count HEAD)\" = 1 ]")];
    let fx = Fx::with(&[e]);
    let (code, out, err) = fx.run(&["--min-pass-rate", "0"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("example: 1 of 1 runs passed"), "{out}");
}

#[test]
fn a_change_without_a_branch_gives_sd506_and_exit_code_3_and_no_process() {
    for branch in [None, Some("main")] {
        let mut e = ev("example");
        e.change = Some("evals/changes/example");
        e.branch = branch;
        let fx = with_change(e);
        let (code, out, err) = fx.run(&[]);
        assert_eq!(code, 3, "{branch:?}: {out}{err}");
        assert!(
            err.contains("evals/example.json") && err.contains("`change` needs a `branch`"),
            "{err}"
        );
        assert!(!fx.path("log").exists(), "the stub ran");
        let (_, out, err) = fx.repo.run(&["check", "--all"]);
        assert!(
            out.contains(
                "evals/example.json:1: error SD506 evals-invalid: `change` needs a `branch`"
            ),
            "{branch:?}: {out}{err}"
        );
    }
}

#[test]
fn the_change_folder_follows_the_rules_of_the_fixture_folder() {
    // (value, part of the message)
    for (change, message) in [
        (
            "other/changes/example",
            "`change` must be a folder under `evals/`",
        ),
        (
            "evals/changes/missing",
            "`change` has no file under `evals/changes/missing`",
        ),
        (
            "evals/changes/../changes/example",
            "`change` must use `/` separators",
        ),
        (
            "evals/results/example",
            "`change` must not be under `evals/results/`",
        ),
    ] {
        let mut e = ev("example");
        e.branch = Some("feature/x");
        e.change = Some(change);
        let fx = with_change(e);
        let (code, out, err) = fx.run(&[]);
        assert_eq!(code, 3, "{change}: {out}{err}");
        assert!(err.contains(message), "{change}: {err}");
    }
    // A change that is not a string.
    let fx = Fx::with(&[ev("example")]);
    fx.repo.write(
        "evals/example.json",
        &ev("example")
            .json()
            .replace("\"checks\"", "\"branch\":\"b\",\"change\":3,\"checks\""),
    );
    fx.repo.commit("bad");
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("`change` must be a string"), "{err}");
}

#[test]
fn the_base_commit_has_a_fixed_author_no_signature_and_a_fixed_date() {
    let fx = Fx::with(&[ev("example")]);
    let edit = format!(
        "git log --format='%an|%ae|%cn|%ce|%aI|%cI|%s|%G?' > \"$STUB_FILES.log\"; git rev-list --count HEAD > \"$STUB_FILES.count\"; {DONE}"
    );
    // A global config that signs each commit must have no effect.
    let (code, out, err) = fx.run_env(
        &[],
        &[
            ("STUB_EDIT", &edit),
            ("GIT_CONFIG_COUNT", "1"),
            ("GIT_CONFIG_KEY_0", "commit.gpgsign"),
            ("GIT_CONFIG_VALUE_0", "true"),
        ],
    );
    assert_eq!(code, 0, "{out}{err}");
    // Git writes a UTC date as `Z` or as `+00:00`, by its version.
    let line = read(&fx.path("files.log")).trim().replace("+00:00", "Z");
    assert_eq!(
        line,
        "spec-dna|eval@spec-dna.invalid|spec-dna|eval@spec-dna.invalid|2000-01-01T00:00:00Z|2000-01-01T00:00:00Z|spec-dna eval base|N"
    );
    assert_eq!(read(&fx.path("files.count")).trim(), "1");
}

#[test]
fn tests_unchanged_compares_with_the_base_commit() {
    let mut e = ev("example");
    e.checks = vec![
        ("notes-done", "file_has notes.txt 'status: done'"),
        ("tests-unchanged", "tests_unchanged tests"),
    ];
    let fx = Fx::with(&[e]);
    fx.repo
        .write("evals/fixtures/example/tests/t.txt", "test\n");
    fx.repo.commit("a test file");
    // The agent does not touch the test: pass. A new test file: pass.
    let (code, out, err) = fx.run_env(
        &[],
        &[("STUB_EDIT", &format!("echo new > tests/new.txt; {DONE}"))],
    );
    assert_eq!(code, 0, "{out}{err}");
    // The agent edits the test: fail. It deletes the test: fail.
    for edit in ["echo changed > tests/t.txt", "rm tests/t.txt"] {
        let (code, out, err) = fx.run_env(&[], &[("STUB_EDIT", &format!("{edit}; {DONE}"))]);
        assert_eq!(code, 1, "{edit}: {out}{err}");
    }
}

#[test]
fn the_exec_bit_of_a_fixture_file_stays() {
    use std::os::unix::fs::PermissionsExt;
    let fx = Fx::with(&[ev("example")]);
    fx.repo
        .write("evals/fixtures/example/run.sh", "#!/bin/sh\n");
    let p = fx.repo.dir.join("evals/fixtures/example/run.sh");
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
    let edit = format!("[ -x run.sh ] && echo yes > \"$STUB_FILES.x\"; {DONE}");
    let (code, out, err) = fx.run_env(&[], &[("STUB_EDIT", &edit)]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(read(&fx.path("files.x")).trim(), "yes");
}

#[test]
fn a_dry_run_starts_no_process() {
    let mut e = ev("example");
    e.runs = 3;
    let fx = Fx::with(&[e]);
    let (code, out, err) = fx.run(&["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!fx.path("log").exists(), "the stub ran");
    assert!(out.contains("example: 3 runs"), "{out}");
    assert!(out.contains("estimate 3.00 USD"), "{out}");
    // A dry run above the cap gives exit code 3.
    let (code, out, err) = fx.run(&["--dry-run", "--max-total-usd", "2"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(!fx.path("log").exists(), "the stub ran");
}

#[test]
fn an_id_selects_an_eval_and_an_unknown_id_is_a_usage_error() {
    let mut other = ev("other");
    other.task = "other task";
    let fx = Fx::with(&[ev("example"), other]);
    let (code, out, err) = fx.run(&["other"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("other: 1 of 1"), "{out}");
    assert!(!out.contains("example:"), "{out}");
    assert_eq!(fx.runs().len(), 1);
    let (code, out, err) = fx.run(&["nothing"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("nothing"), "{err}");
}

#[test]
fn the_evals_run_in_the_order_of_their_ids() {
    let mut z = ev("z-last");
    z.task = "task z";
    let mut a = ev("a-first");
    a.task = "task a";
    let fx = Fx::with(&[z, a]);
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 0, "{out}{err}");
    let tasks: Vec<String> = fx.runs().iter().map(|c| c[1].clone()).collect();
    assert_eq!(tasks, ["task a", "task z"]);
}

#[test]
fn a_bad_flag_value_is_a_usage_error() {
    let fx = Fx::with(&[ev("example")]);
    let cases: &[&[&str]] = &[
        &["--runs", "0"],
        &["--runs", "101"],
        &["--runs", "x"],
        &["--min-pass-rate", "1.5"],
        &["--min-pass-rate", "-0.1"],
        &["--max-total-usd", "0"],
        &["--max-total-usd", "x"],
        &["--runs"],
        // `--out` must not be the folder of the evals.
        &["--out", "evals"],
        &["--out", "evals/"],
        &["--out", "./evals"],
        &["--out", "/abs"],
        &["--out", "../x"],
    ];
    for args in cases {
        let (code, out, err) = fx.run(args);
        assert_eq!(code, 2, "{args:?}: {out}{err}");
    }
    assert!(!fx.path("log").exists(), "the stub ran");
}

#[test]
fn eval_without_a_subcommand_or_with_another_one_is_a_usage_error() {
    let fx = Fx::with(&[ev("example")]);
    for args in [&["eval"][..], &["eval", "list"], &["eval", "runs"]] {
        let (code, out, err) = fx.repo.run(args);
        assert_eq!(code, 2, "{args:?}: {out}{err}");
    }
}

#[test]
fn the_result_goes_to_a_file_with_counts_costs_and_names() {
    let mut e = ev("example");
    e.runs = 2;
    let fx = Fx::with(&[e]);
    let (code, out, err) = fx.run(&["--out", "evals/results"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out2, err2) = fx.run(&["--out", "evals/results", "--runs", "1"]);
    assert_eq!(code, 0, "{out2}{err2}");
    let names: Vec<String> = fs::read_dir(fx.repo.dir.join("evals/results"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 2, "{names:?}");
    for n in &names {
        // eval-YYYYMMDDTHHMMSSZ.json, or with -2, -3 and so on.
        let stem = n
            .strip_prefix("eval-")
            .and_then(|s| s.strip_suffix(".json"))
            .unwrap();
        let (stamp, n_suffix) = stem.split_once('-').unwrap_or((stem, ""));
        assert_eq!(stamp.len(), 16, "{n}");
        assert!(stamp.ends_with('Z') && stamp.as_bytes()[8] == b'T', "{n}");
        assert!(stamp[..8].bytes().all(|b| b.is_ascii_digit()), "{n}");
        assert!(stamp[9..15].bytes().all(|b| b.is_ascii_digit()), "{n}");
        assert!(
            n_suffix.is_empty() || n_suffix.parse::<u32>().is_ok(),
            "{n}"
        );
    }
    let text = read(&fx.repo.dir.join("evals/results").join(&names[0]));
    let text2 = read(&fx.repo.dir.join("evals/results").join(&names[1]));
    let first = if text.contains("\"passed_runs\": 2") {
        text
    } else {
        text2
    };
    assert!(first.ends_with("}\n"), "{first}");
    for want in [
        &format!("\"spec_dna\": \"{VERSION}\""),
        "\"claude_code\": \"2.1.325\"",
        "\"evals_dir\": \"evals\"",
        "\"min_pass_rate\": 0.8",
        "\"max_total_usd\": 20",
        "\"estimate_usd\": 2",
        "\"spend_usd\": 0.2",
        "\"stopped\": null",
        "\"pass_rate\": 1",
        "\"passed\": true",
        "\"id\": \"example\"",
        "\"runs\": 2",
        "\"pass_at\": 2",
        "\"max_budget_usd\": 1",
        "\"passed_runs\": 2",
        "\"claude_exit\": 0",
        "\"subtype\": \"success\"",
        "\"cost_usd\": 0.1",
        "\"name\": \"notes-done\"",
    ] {
        assert!(first.contains(want), "{want} is not in\n{first}");
    }
    // `started` is a UTC time.
    let started = first.split("\"started\": \"").nth(1).unwrap();
    let started = &started[..20];
    assert!(
        started.as_bytes()[4] == b'-' && started.ends_with('Z') && started.as_bytes()[10] == b'T',
        "{started}"
    );
}

#[test]
fn json_prints_the_same_result_on_stdout_and_nothing_else() {
    let mut e = ev("example");
    e.checks = vec![(
        "noisy",
        "echo CHECK-NOISE; file_has notes.txt 'status: done'",
    )];
    let fx = Fx::with(&[e]);
    let (code, out, err) = fx.run(&["--json", "--out", "evals/results"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with("{\"spec_dna\":") && out.ends_with("}\n"),
        "{out}"
    );
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(!out.contains("CHECK-NOISE"), "{out}");
    assert!(!out.contains("eval run:"), "{out}");
    let dir = fx.repo.dir.join("evals/results");
    let file = fs::read_dir(&dir).unwrap().next().unwrap().unwrap().path();
    // The file is the same result, as pretty JSON.
    let squeezed: String = read(&file).split_whitespace().collect::<Vec<_>>().join("");
    assert_eq!(
        squeezed,
        out.split_whitespace().collect::<Vec<_>>().join("")
    );
}

#[test]
fn the_result_has_no_task_text_no_check_text_no_output_text_and_no_path() {
    let mut e = ev("example");
    e.task = "TASK-SECRET-TEXT in notes";
    e.checks = vec![(
        "notes-done",
        "file_has notes.txt 'status: done' # CHECK-SECRET-TEXT",
    )];
    let fx = Fx::with(&[e]);
    let (code, out, err) = fx.run(&["--out", "out"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, json, err2) = fx.run(&["--json", "--out", "out"]);
    assert_eq!(code, 0, "{json}{err2}");
    let mut all = vec![out, json];
    for f in fs::read_dir(fx.repo.dir.join("out")).unwrap() {
        all.push(read(&f.unwrap().path()));
    }
    let root = fx.repo.dir.to_string_lossy().into_owned();
    for text in &all {
        for secret in [
            "TASK-SECRET-TEXT",
            "CHECK-SECRET-TEXT",
            "file_has",
            "STUB-OUTPUT-TEXT",
            "spec-dna-eval-",
            root.as_str(),
            "CLAUDE_PROJECT_DIR",
        ] {
            assert!(!text.contains(secret), "{secret} is in\n{text}");
        }
    }
    // Stderr holds no output text of claude either.
    assert!(!err.contains("STUB-OUTPUT-TEXT"), "{err}");
}

#[test]
fn the_stub_sees_no_claude_project_dir_and_keeps_claudecode() {
    let fx = Fx::with(&[ev("example")]);
    let (code, out, err) = fx.run_env(
        &[],
        &[("CLAUDE_PROJECT_DIR", "/somewhere"), ("CLAUDECODE", "1")],
    );
    assert_eq!(code, 0, "{out}{err}");
    let env = read(&fx.path("env"));
    assert!(env.contains("CLAUDE_PROJECT_DIR=unset"), "{env}");
    // The probe P3: `claude -p` starts under `CLAUDECODE=1`, so the runner
    // keeps the variable.
    assert!(env.contains("CLAUDECODE=1"), "{env}");
    assert_eq!(fx.runs().len(), 1);
}

#[test]
fn an_absent_claude_gives_exit_code_3_and_names_the_program() {
    let fx = Fx::with(&[ev("example")]);
    // The `PATH` of the test, without each folder that has a `claude`.
    let path: Vec<PathBuf> = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .filter(|d| !d.join("claude").exists())
        .collect();
    let path = std::env::join_paths(path).unwrap();
    let (code, out, err) =
        fx.repo
            .run_env(&["eval", "run"], &[("PATH", path.to_str().unwrap())], &[]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("`claude` did not start"), "{err}");
    assert!(err.contains("install Claude Code"), "{err}");
    assert!(!fx.path("log").exists());
}

#[test]
fn the_pin_of_another_version_stops_the_gate() {
    let fx = Fx::with(&[ev("example")]);
    let toml = fx.repo.read("spec-dna.toml").replace(VERSION, "0.1.0");
    fx.repo.write("spec-dna.toml", &toml);
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(!fx.path("log").exists());
}

#[test]
fn no_evals_table_gives_exit_code_3() {
    let fx = Fx::new("", &[ev("example")]);
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("[evals]"), "{err}");
    assert!(!fx.path("log").exists());
}

#[test]
fn an_invalid_eval_gives_exit_code_3_with_its_path_and_no_process() {
    let fx = Fx::with(&[ev("example")]);
    fx.repo
        .write("evals/bad.json", r#"{"id":"bad","task":"t"}"#);
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("evals/bad.json"), "{err}");
    assert!(!fx.path("log").exists());
    // The id must be the file stem.
    fx.repo.write("evals/bad.json", &ev("other").json());
    let (code, _, err) = fx.run(&[]);
    assert_eq!(code, 3, "{err}");
    assert!(
        err.contains("evals/bad.json") && err.contains("`id`"),
        "{err}"
    );
}

#[test]
fn an_out_that_is_a_file_gives_exit_code_4_before_any_process() {
    let fx = Fx::with(&[ev("example")]);
    fx.repo.write("result", "not a folder\n");
    let (code, out, err) = fx.run(&["--out", "result"]);
    assert_eq!(code, 4, "{out}{err}");
    assert!(err.contains("result"), "{err}");
    assert!(!fx.path("log").exists(), "the stub ran");
    // A folder below a file fails in the same way.
    let (code, out, err) = fx.run(&["--out", "result/sub"]);
    assert_eq!(code, 4, "{out}{err}");
    assert!(!fx.path("log").exists(), "the stub ran");
    // A dry run makes no folder, so it does not check one.
    let (code, out, err) = fx.run(&["--dry-run", "--out", "later/results"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!fx.repo.dir.join("later").exists());
}

#[test]
fn the_out_folder_exists_before_the_first_run() {
    let fx = Fx::with(&[ev("example")]);
    let made = fx.repo.dir.join("made/results");
    let made = made.to_str().unwrap();
    let edit = "test -d \"$OUT_DIR\" && touch \"$OUT_DIR/seen\"; echo 'status: done' > notes.txt";
    let (code, out, err) = fx.run_env(
        &["--out", "made/results"],
        &[("STUB_EDIT", edit), ("OUT_DIR", made)],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(fx.repo.dir.join("made/results/seen").exists());
}

#[test]
fn an_eval_folder_with_no_eval_gives_exit_code_3() {
    let fx = Fx::with(&[]);
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(!fx.path("log").exists());
}

#[test]
fn an_absent_check_file_gives_exit_code_3_before_any_run() {
    let fx = Fx::with(&[ev("example")]);
    fs::remove_file(fx.repo.dir.join("evals/check.sh")).unwrap();
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("check.sh"), "{err}");
    assert!(!fx.path("log").exists());
}

#[test]
fn a_json_file_in_a_subfolder_is_not_an_eval() {
    let fx = Fx::with(&[ev("example")]);
    fx.repo
        .write("evals/results/old.json", "{\"started\":\"x\"}");
    fx.repo.write("evals/fixtures/example/data.json", "{}");
    let (code, out, err) = fx.run(&[]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_temporary_folder_is_removed_after_a_failure_too() {
    let fx = Fx::with(&[ev("example")]);
    let (code, out, err) = fx.run_env(&[], &[("STUB_EXIT", "7"), ("STUB_EDIT", "true")]);
    assert_eq!(code, 1, "{out}{err}");
    fx.assert_clean();
}

// ADR 0016, amendment of 2026-11-16: the rule of the refused run applies to
// `--ab` only. `eval run` without `--ab` keeps ADR 0008: a refused run counts
// as a failed run, and the runner never retries it.

#[test]
fn a_refused_run_without_ab_counts_as_a_failed_run_and_is_not_retried() {
    let fx = Fx::with(&[ev("example")]);
    // This stub gives the answer that the API gives for a refused request.
    let refusing = "#!/bin/sh\n{ for a in \"$@\"; do printf '%s\\n' \"$a\"; done; echo '---'; } >> \"$STUB_LOG\"\nif [ \"$1\" = \"--version\" ]; then echo \"2.1.325 (Claude Code)\"; exit 0; fi\nprintf '{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":true,\"result\":\"API Error: 429 Rate limit reached\",\"total_cost_usd\":0.5}\\n'\nexit 1\n";
    stub_bin(&fx.stubs.dir.join("bin"), "claude", refusing);
    let (code, out, err) = fx.run_env(&["--min-pass-rate", "0"], &[("SPEC_DNA_EVAL_BACKOFF", "1")]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("example: 0 of 1 runs passed, pass-at 1: fail, 0.50 USD\n"),
        "{out}"
    );
    // One call of `claude`, and no wait: the rule of ADR 0008 holds.
    assert_eq!(fx.runs().len(), 1);
    assert!(!err.contains("wait"), "{err}");
    // The variable is read by `--ab` only.
    let (code, out, err) = fx.run_env(
        &["--min-pass-rate", "0"],
        &[("SPEC_DNA_EVAL_BACKOFF", "abc")],
    );
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_agent_and_the_checks_commit_with_a_neutral_identity_and_no_signature() {
    let fx = Fx::with(&[Ev {
        checks: vec![
            (
                "agent-identity",
                "git log -1 --format=%ae | grep -q '^eval@example.invalid$'",
            ),
            ("check-commits", "git commit --allow-empty -qm c"),
        ],
        ..ev("example")
    }]);
    // A git configuration of the person that breaks each commit.
    let home = fx.path("home");
    fs::create_dir_all(&home).unwrap();
    fs::write(
        home.join(".gitconfig"),
        "[user]\n\tname = Real Person\n\temail = real@example.org\n[commit]\n\tgpgsign = true\n[gpg]\n\tprogram = /usr/bin/false\n",
    )
    .unwrap();
    let h = home.to_string_lossy().into_owned();
    let (code, out, err) = fx.run_env(
        &[],
        &[
            ("HOME", &h),
            ("XDG_CONFIG_HOME", &h),
            (
                "STUB_EDIT",
                "echo 'status: done' > notes.txt; git add -A; git commit -qm agent",
            ),
        ],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("example: 1 of 1 runs passed"), "{out}");
}
