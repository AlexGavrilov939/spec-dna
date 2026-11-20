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

#[test]
fn only_the_subcommand_commit_reads_the_repository() {
    let r = project();
    r.write(INTENT, &intent("accepted"));
    // The word `commit` is an argument here, not the subcommand.
    for cmd in [
        "git log --grep commit",
        "git show commit",
        "git -C . log --grep=x commit",
        "git branch commit",
    ] {
        assert_eq!(answer(&r, cmd), "", "{cmd}");
    }
    // Options before the subcommand do not hide it.
    for cmd in [
        "git -c core.editor=true commit -am x",
        "git --no-pager commit -am x",
        "git -C . commit -am x",
    ] {
        assert_denies(&answer(&r, cmd), &[INTENT]);
    }
}

// ---- The acknowledgement of an acceptance record (design 0.8.0, D3 item 11) ----

const RECORD: &str = "docs/intent/valve-timeout/acceptance.toml";
const ARCHIVE: &str = ".spec-dna/accepted/valve-timeout.toml";

/// A record text with the entries of `acknowledged-by` and one finding text.
fn record(acks: &[&str], finding: &str) -> String {
    let list: Vec<String> = acks.iter().map(|a| format!("\"{a}\"")).collect();
    format!(
        "format = 1\nchange = \"valve-timeout\"\nscope = [\"src/**\"]\npaths = [\"src/a.py\"]\ntree = \"0123456789abcdef\"\nresult = \"accepted\"\nacknowledged-by = [{}]\n\n[[acceptor]]\nrole = \"acceptor\"\nbrief = \"0123456789abcdef\"\nsession = \"s1\"\nagent = \"unknown\"\nresult = \"accepted\"\n\n[[finding]]\nacceptor = 1\nseverity = \"nit\"\npath = \"src/a.py\"\ntext = \"{finding}\"\n",
        list.join(", ")
    )
}

/// The project with a committed record that has the entries `acks`.
fn with_record(acks: &[&str]) -> Repo {
    let r = project();
    r.write(RECORD, &record(acks, "A name has no unit."));
    r.commit("record");
    r
}

#[test]
fn denies_a_commit_that_adds_an_entry_to_acknowledged_by() {
    let r = with_record(&[]);
    r.write(
        RECORD,
        &record(&["owner@example.org"], "A name has no unit."),
    );
    r.git(&["add", RECORD]);
    let out = answer(&r, "git commit -m 'acknowledge'");
    assert_denies(
        &out,
        &[
            RECORD,
            "`acknowledged-by`",
            "only a person acknowledges a record",
        ],
    );
    // The same edit in the working tree, with `git commit -a`.
    let r = with_record(&[]);
    r.write(
        RECORD,
        &record(&["owner@example.org"], "A name has no unit."),
    );
    assert_denies(&answer(&r, "git commit -am x"), &[RECORD]);
    assert_denies(
        &answer(&r, "git add -A && git commit -m x"),
        &["`acknowledged-by`"],
    );
}

#[test]
fn denies_a_new_record_that_has_an_entry_and_an_amend_that_keeps_one() {
    let r = project();
    r.write(
        RECORD,
        &record(&["owner@example.org"], "A name has no unit."),
    );
    assert_denies(
        &answer(&r, "git add -A && git commit -m x"),
        &[RECORD, "`acknowledged-by`"],
    );
    // A person made this commit. The amend of an agent keeps its entry.
    r.commit("acknowledge");
    assert_eq!(answer(&r, "git commit -m 'more work'"), "", "a new commit");
    assert_denies(
        &answer(&r, "git commit --amend --no-edit"),
        &["`acknowledged-by`"],
    );
}

#[test]
fn allows_a_commit_of_a_record_that_adds_no_entry() {
    // A first record with an empty list.
    let r = project();
    r.write(RECORD, &record(&[], "A name has no unit."));
    assert_eq!(answer(&r, "git add -A && git commit -m record"), "");
    // An edit of a finding keeps the list.
    let r = with_record(&[]);
    r.write(RECORD, &record(&[], "A name has no unit, and no comment."));
    assert_eq!(answer(&r, "git commit -am 'findings'"), "");
    // An entry that HEAD already has is no new entry.
    let r = with_record(&["owner@example.org"]);
    r.write(RECORD, &record(&["owner@example.org"], "Another finding."));
    assert_eq!(answer(&r, "git commit -am 'findings'"), "");
    // Removing an entry adds none.
    r.write(RECORD, &record(&[], "Another finding."));
    assert_eq!(answer(&r, "git commit -am 'findings'"), "");
}

#[test]
fn allows_the_commit_of_finish_that_moves_the_record_to_the_archive() {
    let r = with_record(&["owner@example.org"]);
    // `finish` deletes the folder record and writes the same bytes to the archive.
    let bytes = r.read(RECORD);
    std::fs::remove_file(r.dir.join(RECORD)).unwrap();
    r.write(ARCHIVE, &bytes);
    assert_eq!(answer(&r, "git add -A"), "");
    assert_eq!(answer(&r, "git commit -m 'feat: finish'"), "");
    assert_eq!(answer(&r, "git commit -am 'feat: finish'"), "");
}

/// The tree as `finish` and `git add` leave it for the commit: `git rm` of the
/// record in the folder and `git add` of the archive file. Git sees a rename
/// there, and `git diff --name-only` prints the new path only.
fn staged_like_finish(archive: &str, renames: Option<&str>) -> Repo {
    let r = with_record(&["owner@example.org"]);
    if let Some(v) = renames {
        r.git(&["config", "diff.renames", v]);
    }
    let bytes = if archive.is_empty() {
        r.read(RECORD)
    } else {
        archive.to_string()
    };
    r.git(&["rm", "-q", RECORD]);
    r.write(ARCHIVE, &bytes);
    r.git(&["add", ARCHIVE]);
    r
}

#[test]
fn allows_the_staged_commit_of_finish_with_each_setting_of_rename_detection() {
    // Git detects the rename by default, with `copies`, and not with `false`.
    for renames in [None, Some("true"), Some("copies"), Some("false")] {
        let r = staged_like_finish("", renames);
        assert_eq!(
            answer(&r, "git commit -m 'feat: finish'"),
            "",
            "diff.renames = {renames:?}"
        );
    }
}

#[test]
fn denies_the_staged_commit_of_finish_when_the_archive_bytes_differ() {
    // The same staged shape, and the bytes of the archive file have an entry.
    let edited = record(&["forged@example.org"], "A name has no unit.");
    for renames in [None, Some("copies"), Some("false")] {
        let r = staged_like_finish(&edited, renames);
        assert_denies(
            &answer(&r, "git commit -m 'feat: finish'"),
            &[ARCHIVE, "not the record"],
        );
    }
}

/// The project after `finish`: the record is deleted, and the archive file
/// has the bytes `archive`.
fn finished(archive: &str) -> Repo {
    let r = with_record(&[]);
    std::fs::remove_file(r.dir.join(RECORD)).unwrap();
    r.write(ARCHIVE, archive);
    r
}

#[test]
fn denies_an_archive_file_that_differs_from_the_record_that_the_commit_deletes() {
    // A hand edit adds an entry to `acknowledged-by` in the archive file.
    let edited = record(&["owner@example.org"], "A name has no unit.");
    let r = finished(&edited);
    for cmd in [
        "git add -A && git commit -m 'feat: finish'",
        "git commit -am 'feat: finish'",
    ] {
        assert_denies(&answer(&r, cmd), &[ARCHIVE, "not the record"]);
    }
    // The same bytes as the record pass.
    let same = record(&[], "A name has no unit.");
    assert_eq!(
        answer(
            &finished(&same),
            "git add -A && git commit -m 'feat: finish'"
        ),
        ""
    );
}

#[test]
fn denies_a_new_archive_file_when_the_commit_deletes_no_record() {
    let r = with_record(&[]);
    r.write(ARCHIVE, &record(&[], "A name has no unit."));
    assert_denies(
        &answer(&r, "git add -A && git commit -m 'forged'"),
        &[ARCHIVE, "not the record"],
    );
}

#[test]
fn denies_a_commit_that_edits_an_archive_file() {
    let r = finished(&record(&[], "A name has no unit."));
    r.commit("feat: finish");
    // An edit of a committed archive file, in the tree, in the index.
    r.write(
        ARCHIVE,
        &record(&["owner@example.org"], "A name has no unit."),
    );
    for cmd in ["git commit -am 'edit'", "git commit -m 'edit'"] {
        assert_denies(&answer(&r, cmd), &[ARCHIVE, "edits an archive file"]);
    }
    r.git(&["add", ARCHIVE]);
    assert_denies(
        &answer(&r, "git commit -m 'edit'"),
        &[ARCHIVE, "edits an archive file"],
    );
    // A commit that leaves the file alone passes.
    r.write(ARCHIVE, &record(&[], "A name has no unit."));
    r.git(&["add", ARCHIVE]);
    assert_eq!(answer(&r, "git commit -m 'no change'"), "");
}

#[test]
fn the_archive_folder_name_with_no_final_slash_is_no_edit_of_a_status() {
    let r = finished(&record(&[], "A name has no unit."));
    assert_eq!(
        answer(
            &r,
            "git add .spec-dna/accepted docs/intent && git commit -m 'feat: finish'"
        ),
        ""
    );
}

#[test]
fn the_reason_for_an_entry_does_not_name_a_command_that_does_not_exist_yet() {
    let r = with_record(&[]);
    r.write(
        RECORD,
        &record(&["owner@example.org"], "A name has no unit."),
    );
    let out = answer(&r, "git commit -am 'acknowledge'");
    assert_denies(&out, &["`acknowledged-by`"]);
    assert!(!out.contains("spec-dna approve"), "{out}");
}

#[test]
fn allows_one_call_that_adds_the_archive_path_and_commits_finish() {
    // The folder `.spec-dna/accepted/` has the word `accepted` in its name. A
    // path is no edit of a status, so Rule C does not count it.
    let r = with_record(&["owner@example.org"]);
    let bytes = r.read(RECORD);
    std::fs::remove_file(r.dir.join(RECORD)).unwrap();
    r.write(ARCHIVE, &bytes);
    assert_eq!(
        answer(
            &r,
            "git add .spec-dna/accepted/valve-timeout.toml docs/intent && git commit -m 'feat: finish'"
        ),
        ""
    );
    // The word in text is still an edit.
    let out = answer(&r, "echo 'status: accepted' >> x && git commit -am m");
    assert_denies(&out, &["run the edit and the commit as 2 calls"]);
}

#[test]
fn denies_one_call_that_writes_accepted_and_commits_the_record() {
    let r = with_record(&[]);
    let out = answer(
        &r,
        "printf 'result = \"accepted\"' >> docs/intent/valve-timeout/acceptance.toml && git commit -am record",
    );
    assert_denies(&out, &["run the edit and the commit as 2 calls"]);
}
