//! Tests of `spec-dna hook protect-approvals` (owner review of 0.7.0, item
//! 2). The hook runs on `PreToolUse` of `Bash`. It denies a call that runs
//! `spec-dna approve`, and a `git commit` that moves the status of an artifact
//! of a change to its approval value. Each test uses a temporary repository
//! with example data. No test reads or writes a file of this repository.

mod common;

use common::{BIN, Repo, path_with, stub_bin};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const INTENT: &str = "docs/intent/valve-timeout/intent.md";
const SPEC: &str = "docs/intent/valve-timeout/spec.md";
const PLAN: &str = "docs/intent/valve-timeout/plan.md";

const DENY_START: &str = "{\"hookSpecificOutput\":{\"hookEventName\":\"PreToolUse\",\"permissionDecision\":\"deny\",\"permissionDecisionReason\":\"spec-dna protect-approvals: ";

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// A `PreToolUse` event of Claude Code for a `Bash` call.
fn event(command: &str, cwd: &str) -> String {
    format!(
        "{{\"session_id\":\"s\",\"cwd\":\"{}\",\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"Bash\",\"tool_input\":{{\"command\":\"{}\"}},\"tool_use_id\":\"t\"}}",
        esc(cwd),
        esc(command)
    )
}

/// Runs the hook in `dir` with `stdin`. `path` is a folder that goes first in
/// `PATH`. Returns the exit code, stdout and stderr.
fn hook_in(
    dir: &Path,
    project: Option<&Path>,
    path: Option<&Path>,
    stdin: &[u8],
) -> (i32, String, String) {
    let mut cmd = Command::new(BIN);
    cmd.args(["hook", "protect-approvals", "--intent", "docs/intent"])
        .current_dir(dir)
        .env_remove("CLAUDE_PROJECT_DIR")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(p) = project {
        cmd.env("CLAUDE_PROJECT_DIR", p);
    }
    if let Some(p) = path {
        cmd.env("PATH", path_with(p));
    }
    let mut child = cmd.spawn().unwrap();
    let _ = child.stdin.take().unwrap().write_all(stdin);
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The answer of the hook for one `Bash` command in the repository `r`.
fn answer(r: &Repo, command: &str) -> String {
    let ev = event(command, &r.dir.to_string_lossy());
    let (code, out, err) = hook_in(&r.dir, None, None, ev.as_bytes());
    assert_eq!(code, 0, "{out}{err}");
    out
}

fn intent(status: &str) -> String {
    format!(
        "---\nstatus: {status}\nclass: higher-risk\n---\n# Valve timeout\n\n## Intent\n\nClose the valve.\n"
    )
}

fn plain(status: &str, body: &str) -> String {
    format!("---\nstatus: {status}\n---\n# Valve timeout\n\n{body}\n")
}

/// A repository with a draft intent, a draft spec and a draft plan, and a
/// file outside the folder of the changes.
fn project() -> Repo {
    let r = Repo::new();
    r.write(INTENT, &intent("draft"));
    r.write(SPEC, &plain("draft", "## Delta\n\nNone."));
    r.write(PLAN, &plain("draft", "## Files\n\n- `src/a.py`"));
    r.write("docs/notes/other/intent.md", &intent("draft"));
    r.write("src/a.py", "x = 1\n");
    r.commit("start");
    r
}

fn assert_denies(out: &str, needles: &[&str]) {
    assert!(out.starts_with(DENY_START), "{out}");
    assert!(out.ends_with("\"}}\n"), "{out}");
    for n in needles {
        assert!(out.contains(n), "`{n}` is not in {out}");
    }
}

#[test]
fn denies_spec_dna_approve_in_a_bash_call() {
    let r = project();
    for cmd in [
        "spec-dna approve valve-timeout intent",
        "cd /tmp && /usr/local/bin/spec-dna approve valve-timeout plan --json",
        "echo go; ./target/debug/spec-dna approve valve-timeout spec",
        "git status | spec-dna approve valve-timeout intent",
    ] {
        let out = answer(&r, cmd);
        assert_denies(&out, &["`spec-dna approve`", "a terminal of their own"]);
    }
    // Other `spec-dna` commands and other words pass.
    for cmd in [
        "spec-dna check",
        "spec-dna next valve-timeout",
        "echo approve",
        "approve valve-timeout intent",
        "spec-dna-x approve valve-timeout intent",
    ] {
        assert_eq!(answer(&r, cmd), "", "{cmd}");
    }
}

#[test]
fn denies_spec_dna_approve_inside_sh_c() {
    let r = project();
    for cmd in [
        "sh -c \"spec-dna approve valve-timeout intent\"",
        "bash -lc 'cd /tmp; spec-dna approve valve-timeout intent'",
        "zsh -c 'echo a && spec-dna  approve valve-timeout spec'",
    ] {
        let out = answer(&r, cmd);
        assert_denies(&out, &["`spec-dna approve`"]);
    }
}

#[test]
fn denies_a_commit_of_a_staged_status_change() {
    let r = project();
    r.write(INTENT, &intent("accepted"));
    r.git(&["add", INTENT]);
    let out = answer(&r, "git commit -m 'approve'");
    assert_denies(
        &out,
        &[
            INTENT,
            "`accepted`",
            "spec-dna approve valve-timeout intent",
            "a terminal of their own",
        ],
    );
    // A spec and a plan move to `approved`. The hook names the first path.
    let r = project();
    r.write(SPEC, &plain("approved", "## Delta\n\nNone."));
    r.write(PLAN, &plain("approved", "## Files\n\n- `src/a.py`"));
    r.git(&["add", "-A"]);
    let out = answer(&r, "git commit -m x");
    assert_denies(
        &out,
        &[PLAN, "`approved`", "spec-dna approve valve-timeout plan"],
    );
}

#[test]
fn denies_git_add_and_commit_of_an_unstaged_status_change() {
    let r = project();
    r.write(INTENT, &intent("accepted"));
    let out = answer(&r, "git add -A && git commit -m 'work'");
    assert_denies(&out, &[INTENT, "`accepted`"]);
    let out = answer(&r, "git add docs; git commit -m work");
    assert_denies(&out, &[INTENT]);
}

#[test]
fn denies_commit_all_of_an_unstaged_status_change() {
    let r = project();
    r.write(PLAN, &plain("approved", "## Files\n\n- `src/a.py`"));
    let out = answer(&r, "git commit -am 'work'");
    assert_denies(&out, &[PLAN, "`approved`"]);
}

#[test]
fn denies_amend_that_keeps_a_status_change() {
    let r = project();
    r.write(INTENT, &intent("accepted"));
    // A person made this commit. The amend of the agent keeps it.
    r.commit("approve the intent");
    assert_eq!(answer(&r, "git commit -m 'more work'"), "", "a new commit");
    let out = answer(&r, "git commit --amend --no-edit");
    assert_denies(&out, &[INTENT, "`accepted`"]);
}

#[test]
fn denies_a_new_artifact_with_an_approval_status() {
    let r = project();
    r.write("docs/intent/new-change/intent.md", &intent("accepted"));
    let out = answer(&r, "git add -A && git commit -m 'add a change'");
    assert_denies(&out, &["docs/intent/new-change/intent.md", "`accepted`"]);
}

#[test]
fn denies_an_edit_and_a_commit_in_one_call() {
    let r = project();
    let out = answer(
        &r,
        "sed -i '' 's/draft/accepted/' docs/intent/valve-timeout/intent.md && git commit -am approve",
    );
    assert_denies(&out, &["run the edit and the commit as 2 calls"]);
    let out = answer(
        &r,
        "printf 'status: approved' > docs/intent/valve-timeout/plan.md; git add -A; git commit -m x",
    );
    assert_denies(&out, &["run the edit and the commit as 2 calls"]);
}

#[test]
fn reads_the_folder_of_git_c() {
    let r = project();
    r.write(INTENT, &intent("accepted"));
    // The `cwd` of the event is a folder that is no repository.
    let other = Repo::empty();
    let dir = r.dir.to_string_lossy().into_owned();
    for cmd in [
        format!("git -C {dir} commit -am x"),
        format!("cd {dir} && git commit -am x"),
        format!("git -C '{dir}' commit -am x"),
    ] {
        let ev = event(&cmd, &other.dir.to_string_lossy());
        let (code, out, err) = hook_in(&other.dir, None, None, ev.as_bytes());
        assert_eq!(code, 0, "{err}");
        assert_denies(&out, &[INTENT]);
    }
    // A folder with a variable gives no output: the hook cannot read it.
    let ev = event(
        "cd $HOME/x && git commit -am x",
        &other.dir.to_string_lossy(),
    );
    let (code, out, _) = hook_in(&other.dir, None, None, ev.as_bytes());
    assert_eq!((code, out.as_str()), (0, ""));
    // The `cwd` comes before `CLAUDE_PROJECT_DIR`.
    let ev = event("git commit -am x", &r.dir.to_string_lossy());
    let (_, out, _) = hook_in(&other.dir, Some(&other.dir), None, ev.as_bytes());
    assert_denies(&out, &[INTENT]);
}

#[test]
fn allows_a_commit_of_plan_text_with_no_status_change() {
    let r = project();
    r.write(
        PLAN,
        &plain("draft", "## Files\n\n- `src/a.py`\n- `src/b.py`"),
    );
    assert_eq!(answer(&r, "git commit -am 'plan text'"), "");
    // An artifact that is approved in HEAD keeps its status in a text edit.
    r.write(SPEC, &plain("approved", "## Delta\n\nNone."));
    r.commit("a person approves the spec");
    r.write(SPEC, &plain("approved", "## Delta\n\nNone, as before."));
    assert_eq!(answer(&r, "git add -A && git commit -m 'text'"), "");
    // A positive control: the same call with a new approval is denied.
    r.write(PLAN, &plain("approved", "## Files\n\n- `src/a.py`"));
    assert_denies(&answer(&r, "git commit -am x"), &[PLAN]);
}

#[test]
fn allows_a_commit_that_sets_draft_or_closed() {
    let r = project();
    r.write(INTENT, &intent("accepted"));
    r.commit("a person approves the intent");
    r.write(INTENT, &intent("draft"));
    assert_eq!(answer(&r, "git commit -am 'set it back'"), "");
    r.write(INTENT, &intent("closed"));
    assert_eq!(answer(&r, "git commit -am 'close it'"), "");
}

#[test]
fn allows_a_commit_outside_the_intent_folders() {
    let r = project();
    r.write("docs/notes/other/intent.md", &intent("accepted"));
    r.write("src/a.py", "x = 2\n");
    assert_eq!(answer(&r, "git commit -am 'code'"), "");
    // A file of the folder that is no artifact of a change gives no output.
    r.write(
        "docs/intent/valve-timeout/notes.md",
        &plain("approved", "Text."),
    );
    r.write("docs/intent/readme.md", &plain("approved", "Text."));
    assert_eq!(answer(&r, "git add -A && git commit -m x"), "");
}

#[cfg(unix)]
#[test]
fn allows_a_command_with_no_commit_and_runs_no_git() {
    let r = project();
    r.write(INTENT, &intent("accepted"));
    let stubs = Repo::empty();
    let marker = stubs.dir.join("git-ran");
    let script = format!("#!/bin/sh\necho ran >> '{}'\nexit 1\n", marker.display());
    stub_bin(&stubs.dir.join("bin"), "git", &script);
    for cmd in [
        "git status",
        "git add -A && git diff",
        "echo accepted; ls",
        "spec-dna check",
        "git log --oneline",
    ] {
        let ev = event(cmd, &r.dir.to_string_lossy());
        let (code, out, err) = hook_in(&r.dir, None, Some(&stubs.dir.join("bin")), ev.as_bytes());
        assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""), "{cmd}");
    }
    assert!(!marker.exists(), "git ran for a command with no commit");
    // A positive control: a commit runs git.
    let ev = event("git commit -m x", &r.dir.to_string_lossy());
    let _ = hook_in(&r.dir, None, Some(&stubs.dir.join("bin")), ev.as_bytes());
    assert!(marker.exists(), "git runs for a commit");
}

#[test]
fn gives_no_output_on_bad_json_or_no_repository() {
    let r = project();
    r.write(INTENT, &intent("accepted"));
    for stdin in [
        "",
        "not json",
        "{\"tool_name\":\"Bash\"}",
        "{\"tool_name\":\"Edit\",\"tool_input\":{\"command\":\"spec-dna approve a intent\"}}",
        "{\"tool_name\":\"Bash\",\"tool_input\":{\"command\":5}}",
    ] {
        let (code, out, err) = hook_in(&r.dir, None, None, stdin.as_bytes());
        assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""), "{stdin}");
    }
    // A commit in a folder that is no repository.
    let other = Repo::empty();
    let ev = event("git commit -am x", &other.dir.to_string_lossy());
    let (code, out, err) = hook_in(&other.dir, None, None, ev.as_bytes());
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    // No `--intent` flag: nothing to protect from a commit.
    let ev = event("git commit -am x", &r.dir.to_string_lossy());
    let mut cmd = Command::new(BIN);
    cmd.args(["hook", "protect-approvals"])
        .current_dir(&r.dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    let _ = child.stdin.take().unwrap().write_all(ev.as_bytes());
    let out = child.wait_with_output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "");
    // `spec-dna approve` is denied also with no `--intent` flag.
    let ev = event("spec-dna approve a intent", &r.dir.to_string_lossy());
    let mut child = Command::new(BIN)
        .args(["hook", "protect-approvals"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let _ = child.stdin.take().unwrap().write_all(ev.as_bytes());
    let out = child.wait_with_output().unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).starts_with(DENY_START));
}

#[test]
fn an_unresolved_folder_inspects_the_cwd_and_the_project() {
    let r = project();
    r.write(INTENT, &intent("accepted"));
    let other = Repo::empty();
    let cwd = r.dir.to_string_lossy().into_owned();
    // The folder has a variable or `~`, so the hook cannot read it: it
    // inspects the repository of the `cwd` too.
    for cmd in [
        "cd $WORK && git commit -am x",
        "cd ~user/work && git commit -am x",
        "git -C $WORK commit -am x",
        "cd \"$(pwd)/sub\" && git commit -am x",
        "cd ${PWD} && git commit -am x",
    ] {
        let ev = event(cmd, &cwd);
        let (code, out, err) = hook_in(&other.dir, None, None, ev.as_bytes());
        assert_eq!(code, 0, "{err}");
        assert_denies(&out, &[INTENT]);
    }
    // The repository of `CLAUDE_PROJECT_DIR` too, when the `cwd` is another.
    let ev = event("cd $WORK && git commit -am x", &other.dir.to_string_lossy());
    let (_, out, _) = hook_in(&other.dir, Some(&r.dir), None, ev.as_bytes());
    assert_denies(&out, &[INTENT]);
    // `$CLAUDE_PROJECT_DIR` is a plain variable that the hook expands.
    let ev = event(
        "cd $CLAUDE_PROJECT_DIR && git commit -am x",
        &other.dir.to_string_lossy(),
    );
    let (_, out, _) = hook_in(&other.dir, Some(&r.dir), None, ev.as_bytes());
    assert_denies(&out, &[INTENT]);
    // None of these repositories has a pending approval: the call passes.
    let clean = project();
    let ev = event("cd $WORK && git commit -am x", &clean.dir.to_string_lossy());
    let (code, out, _) = hook_in(&clean.dir, Some(&clean.dir), None, ev.as_bytes());
    assert_eq!((code, out.as_str()), (0, ""));
}

#[test]
fn a_line_continuation_between_git_and_commit_is_one_segment() {
    let r = project();
    r.write(INTENT, &intent("accepted"));
    let out = answer(&r, "git \\\n  commit -am x");
    assert_denies(&out, &[INTENT]);
    let out = answer(&r, "git -c user.name=x \\\n  -C . commit -am x");
    assert_denies(&out, &[INTENT]);
}
