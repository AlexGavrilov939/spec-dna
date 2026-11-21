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

fn crlf(text: &str) -> String {
    text.replace('\n', "\r\n")
}

#[test]
fn a_line_end_of_crlf_is_no_difference_in_the_archive_compare() {
    // `core.autocrlf = true`: the record is CRLF on disk and LF in HEAD, and
    // `finish` writes the archive in CRLF. Both orders pass.
    let lf = record(&["owner@example.org"], "A name has no unit.");
    assert_eq!(
        answer(
            &staged_like_finish(&crlf(&lf), None),
            "git commit -m 'feat: finish'"
        ),
        "",
        "an archive in CRLF, a record in LF"
    );
    let r = project();
    r.write(RECORD, &crlf(&lf));
    r.commit("record in CRLF");
    r.git(&["rm", "-q", RECORD]);
    r.write(ARCHIVE, &lf);
    r.git(&["add", ARCHIVE]);
    assert_eq!(
        answer(&r, "git commit -m 'feat: finish'"),
        "",
        "an archive in LF, a record in CRLF"
    );
    // An archive file of the old tree that only changes its line ends is no edit.
    let r = staged_like_finish(&lf, None);
    r.commit("feat: finish");
    r.write(ARCHIVE, &crlf(&lf));
    assert_eq!(answer(&r, "git commit -am 'line ends'"), "");
}

#[test]
fn a_forged_entry_is_denied_also_when_the_file_has_crlf() {
    let forged = crlf(&record(&["forged@example.org"], "A name has no unit."));
    assert_denies(
        &answer(
            &staged_like_finish(&forged, None),
            "git commit -m 'feat: finish'",
        ),
        &[ARCHIVE, "not the record"],
    );
    // An edit of a committed archive file with a real change and CRLF.
    let lf = record(&[], "A name has no unit.");
    let r = staged_like_finish(&lf, None);
    r.commit("feat: finish");
    r.write(ARCHIVE, &forged);
    assert_denies(
        &answer(&r, "git commit -am 'edit'"),
        &[ARCHIVE, "edits an archive file"],
    );
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

// Rule D (design 0.8.0, D75 item 5): a `Bash` call of the agent `acceptor` commits
// the record of a change alone. The hook reads the text of the command and the index.

/// A `PreToolUse` event of a subagent: the input has `agent_type`.
fn event_as(command: &str, cwd: &str, agent_type: Option<&str>) -> String {
    let agent = agent_type.map_or(String::new(), |a| format!("\"agent_type\":\"{a}\","));
    format!(
        "{{\"session_id\":\"s\",{agent}\"cwd\":\"{}\",\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"Bash\",\"tool_input\":{{\"command\":\"{}\"}},\"tool_use_id\":\"t\"}}",
        esc(cwd),
        esc(command)
    )
}

fn answer_as(r: &Repo, command: &str, agent_type: Option<&str>) -> String {
    let ev = event_as(command, &r.dir.to_string_lossy(), agent_type);
    let (code, out, err) = hook_in(&r.dir, None, None, ev.as_bytes());
    assert_eq!(code, 0, "{out}{err}");
    out
}

/// A repository with a record that the acceptor wrote and has not committed.
fn project_with_record() -> Repo {
    let r = project();
    r.write(RECORD, "format = 2\n");
    r
}

#[test]
fn rule_d_denies_a_commit_of_the_acceptor_that_is_not_the_record_alone() {
    let r = project_with_record();
    r.write("src/a.py", "x = 2\n");
    for cmd in [
        // A path argument other than the record.
        "git commit -m record src/a.py docs/intent/valve-timeout/acceptance.toml",
        "git commit -m record -- src/a.py",
        // An option that commits more than the record.
        "git commit -am record",
        "git commit -a -m record",
        "git commit --all -m record",
        "git commit -p -m record",
        "git commit --patch",
        "git commit -i -m record docs/intent/valve-timeout/acceptance.toml",
        "git commit --include -m record docs/intent/valve-timeout/acceptance.toml",
        "git commit --interactive",
        // An earlier `git add`, `git rm` or `git mv` of another path in the call.
        "git add src/a.py && git commit -m record",
        "git add docs/intent/valve-timeout/acceptance.toml src/a.py; git commit -m record",
        "git add . && git commit -m record",
        "git add -A && git commit -m record",
        "git rm src/a.py && git commit -m record",
        "git mv src/a.py src/b.py && git commit -m record",
    ] {
        let out = answer_as(&r, cmd, Some("acceptor"));
        assert_denies(&out, &["`acceptor`", "the record"]);
    }
}

#[test]
fn rule_d_denies_a_commit_with_no_path_while_the_index_holds_another_path() {
    let r = project_with_record();
    r.write("src/a.py", "x = 2\n");
    r.git(&["add", "src/a.py"]);
    let out = answer_as(&r, "git commit -m record", Some("acceptor"));
    assert_denies(&out, &["`acceptor`", "src/a.py"]);
    // With the record only in the index, a commit with no path passes.
    let r = project_with_record();
    r.git(&["add", RECORD]);
    assert_eq!(answer_as(&r, "git commit -m record", Some("acceptor")), "");
}

#[test]
fn rule_d_allows_the_record_alone_and_a_call_with_no_agent_type() {
    let r = project_with_record();
    r.write("src/a.py", "x = 2\n");
    for cmd in [
        "git commit -m record docs/intent/valve-timeout/acceptance.toml",
        "git commit -m record -- docs/intent/valve-timeout/acceptance.toml",
        "git add docs/intent/valve-timeout/acceptance.toml && git commit -m record",
        "git status",
        "spec-dna check",
    ] {
        assert_eq!(answer_as(&r, cmd, Some("acceptor")), "", "{cmd}");
    }
    // No `agent_type` (the main session), or another agent: rule D does not run.
    for agent in [None, Some("general-purpose")] {
        for cmd in [
            "git commit -am record",
            "git add src/a.py && git commit -m record",
        ] {
            assert_eq!(answer_as(&r, cmd, agent), "", "{agent:?} {cmd}");
        }
    }
}

/// The prompt line of the acceptor has a `cd`, and a subagent starts in the folder of
/// the main session: rule D reads the index of the folder that the call moves to.
#[test]
fn rule_d_reads_the_folder_that_cd_or_git_c_gives_not_the_cwd_of_the_event() {
    let r = project_with_record();
    let elsewhere = Repo::new();
    let root = r.dir.to_string_lossy().into_owned();
    let ask = |cmd: &str| {
        // The project is `r`: the event starts in another folder.
        let ev = event_as(cmd, &elsewhere.dir.to_string_lossy(), Some("acceptor"));
        let (code, out, err) = hook_in(&elsewhere.dir, Some(&r.dir), None, ev.as_bytes());
        assert_eq!(code, 0, "{out}{err}");
        out
    };
    // A clean index in `r` (only the record is staged by the call): allow.
    for cmd in [
        format!("cd '{root}' && git add {RECORD} && git commit -m x"),
        format!("cd '{root}' && git commit -m x {RECORD}"),
        format!("git -C '{root}' commit -m x {RECORD}"),
        format!("cd '{root}/docs' && git commit -m x intent/valve-timeout/acceptance.toml"),
        format!("cd '{root}' && git commit -m x ./{RECORD}"),
        format!("cd '{root}' && git commit -m x '{root}/{RECORD}'"),
    ] {
        assert_eq!(ask(&cmd), "", "{cmd}");
    }
    // A dirty index in `r`: deny, also when the cwd of the event is another folder.
    r.write("src/a.py", "x = 2\n");
    r.git(&["add", "src/a.py"]);
    let out = ask(&format!("cd '{root}' && git commit -m x"));
    assert_denies(&out, &["`acceptor`", "src/a.py"]);
    let out = ask(&format!("git -C '{root}' commit -m x"));
    assert_denies(&out, &["src/a.py"]);
    // A path of the same name in a folder that is not the record: deny.
    let out = ask(&format!(
        "cd '{root}/src' && git commit -m x intent/valve-timeout/acceptance.toml"
    ));
    assert_denies(&out, &["is not the record"]);
}

#[test]
fn rule_d_denies_a_target_that_it_cannot_read_and_two_records() {
    let r = project_with_record();
    let other = "docs/intent/other-change/acceptance.toml";
    r.write(other, "format = 2\n");
    for cmd in [
        format!("git add {RECORD} {other} && git commit -m x"),
        format!("git commit -m x {RECORD} {other}"),
        format!("git add {RECORD} && git commit -m x {other}"),
        "cd $SOME_DIR && git commit -m x docs/intent/valve-timeout/acceptance.toml".to_string(),
    ] {
        let out = answer_as(&r, &cmd, Some("acceptor"));
        assert!(out.starts_with(DENY_START), "{cmd}: {out}");
    }
    r.git(&["add", RECORD, other]);
    let out = answer_as(&r, "git commit -m x", Some("acceptor"));
    assert_denies(&out, &["the index holds the records"]);
}

/// A first record is not tracked: the command that the agent file gives works.
#[test]
fn the_commit_command_of_the_acceptor_file_works_for_a_first_record_and_passes_rule_d() {
    let r = project_with_record();
    let cmd = format!("git add {RECORD} && git commit -m record {RECORD}");
    assert_eq!(answer_as(&r, &cmd, Some("acceptor")), "");
    r.git(&["add", RECORD]);
    r.git(&["commit", "-m", "record", RECORD]);
    // Without the `git add`, git refuses the path of a new file.
    let r = project_with_record();
    let out = Command::new("git")
        .args(["commit", "-m", "record", RECORD])
        .current_dir(&r.dir)
        .output()
        .unwrap();
    assert!(!out.status.success());
}

const NEW_RECORD: &str = "docs/intent/valve-timeout/acceptance.toml";

#[test]
fn rule_d_reads_a_quoted_message_as_one_word_and_never_as_a_path_or_an_option() {
    let r = project_with_record();
    r.write("src/a.py", "x = 2\n");
    for cmd in [
        format!("git commit -m \"accept the change\" {NEW_RECORD}"),
        format!("git commit -m 'record of demo' {NEW_RECORD}"),
        format!("git commit --message=\"two words\" {NEW_RECORD}"),
        format!("git commit --message 'two words' {NEW_RECORD}"),
        format!("git commit -m \"fix (typo)\" {NEW_RECORD}"),
        format!("git commit -m \"a; b && c | d\" {NEW_RECORD}"),
        format!("git commit -m \"it's -a and --amend\" {NEW_RECORD}"),
        format!("git commit -m 'say \"hi\" src/a.py' {NEW_RECORD}"),
        format!("git commit -m x '{NEW_RECORD}'"),
        format!("git commit -m x \"{NEW_RECORD}\""),
        format!("git commit -sm \"two words\" {NEW_RECORD}"),
        format!("git commit --no-verify -m \"two words\" -- {NEW_RECORD}"),
        format!("git add {NEW_RECORD} && git commit -m \"two words\""),
        format!("git add {NEW_RECORD} && git commit -m 'it'\"'\"'s two words'"),
        format!("git commit -m \"line one\nline two\" {NEW_RECORD}"),
    ] {
        assert_eq!(answer_as(&r, &cmd, Some("acceptor")), "", "{cmd}");
    }
    // A message that holds a command substitution runs code that the hook cannot read.
    for cmd in [
        format!("git commit -m \"$(git add -A)\" {NEW_RECORD}"),
        format!("git commit -m \"`git add -A`\" {NEW_RECORD}"),
    ] {
        let out = answer_as(&r, &cmd, Some("acceptor"));
        assert!(out.starts_with(DENY_START), "{cmd}: {out}");
    }
    // The text of a message does not hide a path that follows it.
    let out = answer_as(
        &r,
        "git commit -m \"accept the change\" src/a.py",
        Some("acceptor"),
    );
    assert_denies(&out, &["`src/a.py`", "is not the record"]);
}

#[test]
fn rule_d_reads_a_root_with_a_space_as_one_target_and_fails_closed_on_a_target_it_cannot_read() {
    let spaced = std::env::temp_dir().join(format!("spec-dna sp ace {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&spaced);
    std::fs::create_dir_all(spaced.join("docs/intent/valve-timeout")).unwrap();
    std::fs::create_dir_all(spaced.join("src")).unwrap();
    common::git(&spaced, &["init", "-q", "-b", "main"]);
    std::fs::write(spaced.join(NEW_RECORD), "format = 2\n").unwrap();
    std::fs::write(spaced.join("src/a.py"), "x = 1\n").unwrap();
    let elsewhere = Repo::new();
    let root = spaced.to_string_lossy().into_owned();
    let ask = |cmd: &str| {
        let ev = event_as(cmd, &elsewhere.dir.to_string_lossy(), Some("acceptor"));
        let (code, out, err) = hook_in(&elsewhere.dir, Some(&spaced), None, ev.as_bytes());
        assert_eq!(code, 0, "{out}{err}");
        out
    };
    for cmd in [
        format!("cd '{root}' && git commit -am r"),
        format!("cd '{root}' && git add -A && git commit -m r"),
        format!("cd \"{root}\" && git commit -a -m r"),
        format!("git -C '{root}' commit -am r"),
        // A target that the hook cannot read: it fails closed for a call with a commit.
        "cd $SCRATCH/r && git commit -am r".to_string(),
        "cd ~other/r && git commit -am r".to_string(),
    ] {
        let out = ask(&cmd);
        assert!(out.starts_with(DENY_START), "{cmd}: {out}");
    }
    for cmd in [
        format!("cd '{root}' && git commit -m \"two words\" {NEW_RECORD}"),
        format!("cd '{root}' && git add {NEW_RECORD} && git commit -m x"),
    ] {
        assert_eq!(ask(&cmd), "", "{cmd}");
    }
    let _ = std::fs::remove_dir_all(&spaced);
}

#[test]
fn rule_d_allows_only_the_exact_options_and_one_literal_path_after_git_commit() {
    let r = project_with_record();
    r.write("src/a.py", "x = 2\n");
    r.write("docs/intent/other-change/acceptance.toml", "format = 2\n");
    for cmd in [
        // Abbreviated options: git accepts a unique prefix.
        format!("git commit --inc -m x {NEW_RECORD}"),
        format!("git commit --interac -m x {NEW_RECORD}"),
        format!("git commit --patc -m x {NEW_RECORD}"),
        format!("git commit --al -m x {NEW_RECORD}"),
        // Other options that commit more than the record, or change the commit.
        format!("git commit --pathspec-from-file=list.txt -m x {NEW_RECORD}"),
        "git commit --pathspec-from-file list.txt -m x".to_string(),
        format!("git commit --amend -m x {NEW_RECORD}"),
        format!("git commit --only -m x {NEW_RECORD}"),
        format!("git commit -o -m x {NEW_RECORD}"),
        format!("git commit -m x --unknown-option {NEW_RECORD}"),
        // A glob, a pathspec magic and a folder commit more than one record.
        "git commit -m x 'docs/intent/*/acceptance.toml'".to_string(),
        "git commit -m x 'docs/intent/valve-timeout/accept*.toml'".to_string(),
        "git commit -m x ':(glob)docs/**/acceptance.toml'".to_string(),
        "git commit -m x docs/intent/valve-timeout".to_string(),
        // More than one path.
        format!("git commit -m x {NEW_RECORD} docs/intent/other-change/acceptance.toml"),
        format!("git commit -m x {NEW_RECORD} {NEW_RECORD}"),
        // An option of an earlier `git add` that names the record.
        format!("git add -f {NEW_RECORD} && git commit -m x"),
        format!("git add --intent-to-add {NEW_RECORD} && git commit -m x"),
        // The message text hides nothing.
        "git commit -m \"fix (typo)\" src/a.py".to_string(),
        "git commit -m \"a; b\" src/a.py".to_string(),
    ] {
        let out = answer_as(&r, &cmd, Some("acceptor"));
        assert!(out.starts_with(DENY_START), "{cmd}: {out}");
    }
}

#[test]
fn rule_d_runs_only_for_a_call_with_git_commit_in_the_repository_of_the_project() {
    let r = project_with_record();
    r.write("src/a.py", "x = 2\n");
    // No `git commit` in the call: nothing to read.
    for cmd in [
        "git add src/a.py",
        "git status && git add -A",
        "git rm -r src",
        "git mv src/a.py src/b.py",
    ] {
        assert_eq!(answer_as(&r, cmd, Some("acceptor")), "", "{cmd}");
    }
    // A scratch repository of the agent is another repository: the acceptor builds
    // its test repositories there, with commits of any form.
    let scratch = Repo::new();
    scratch.write("f", "1\n");
    let s = scratch.dir.to_string_lossy().into_owned();
    let root = r.dir.to_string_lossy().into_owned();
    for cmd in [
        format!("cd '{s}' && git add f && git commit -m x f"),
        format!("cd '{s}' && git add -A && git commit -am \"two words\""),
        format!("git -C '{s}' commit -a -m x"),
    ] {
        assert_eq!(answer_as(&r, &cmd, Some("acceptor")), "", "{cmd}");
    }
    // The same forms in the repository of the project are denied.
    let out = answer_as(&r, "git add -A && git commit -m x", Some("acceptor"));
    assert!(out.starts_with(DENY_START), "{out}");
    // A commit in the scratch repository and one in the project in one call: the
    // second is read.
    let out = answer_as(
        &r,
        &format!("cd '{s}' && git commit -am x && cd '{root}' && git commit -am y"),
        Some("acceptor"),
    );
    assert!(out.starts_with(DENY_START), "{out}");
}

/// The checker's bypasses of rule D: the hook reads a fixed grammar and denies
/// what it cannot read, so no form below passes. Each was allowed before.
#[test]
fn rule_d_denies_each_form_that_a_real_shell_reads_other_than_the_hook_does() {
    let r = project_with_record();
    r.write("src/a.py", "x = 2\n");
    let scratch = Repo::new();
    scratch.write("f", "1\n");
    let s = scratch.dir.to_string_lossy().into_owned();
    let root = r.dir.to_string_lossy().into_owned();
    let elsewhere = Repo::empty();
    let cases: Vec<String> = vec![
        // (1) other joins, a failed `cd`, a subshell, `cd` options, `pushd`.
        "cd /nonexistent/x; git add -A; git commit -m test".to_string(),
        "cd /nonexistent/x && git add -A && git commit -m test".to_string(),
        format!("(cd '{s}') && git commit -am x"),
        format!("false && cd '{s}'; git commit -am x"),
        format!("cd -P '{root}' && git commit -am x"),
        format!("cd -- '{root}' && git commit -am x"),
        format!("cd '{s}' && pushd '{root}' && git commit -am x"),
        format!("cd '{root}' || git commit -am x"),
        format!("cd '{root}' & git commit -am x"),
        format!("cd '{root}' | git commit -am x"),
        format!("cd '{root}'\ngit commit -am x"),
        // (2) a script that a shell runs.
        format!("cd '{root}' && bash -c 'git add -A && git commit -m x'"),
        "sh -c \"git commit -am x\"".to_string(),
        "eval \"git commit -am x\"".to_string(),
        "xargs git commit -am x".to_string(),
        "env GIT_DIR=.git git commit -am x".to_string(),
        "g''it commit -am x".to_string(),
        "git c\\ommit -am x".to_string(),
        "git-commit -am x".to_string(),
        // (3) a comment or `$'..'` with an apostrophe.
        "true # don't stage all\ngit add src/a.py\ngit commit -m \"done\"\n# that's it".to_string(),
        "echo $'\\'' ; git commit -am x ; echo $'\\''".to_string(),
        "git commit -am x # a comment".to_string(),
        // (4) a redirect character in a quoted word is text, and the option stays.
        "git commit -m '>' -a".to_string(),
        "git commit -m '>x' -a".to_string(),
        "git commit -m '>' src/a.py".to_string(),
        "git commit -m x > out.txt".to_string(),
        "git commit -m x 2>&1".to_string(),
        // (5) another git folder by an option.
        format!("git --git-dir='{root}/.git' --work-tree='{root}' commit -m x"),
        "git -c core.hooksPath=/x commit -m x".to_string(),
        format!("git -C '{root}' -C src commit -am x"),
        // (6) another verb, or an alias of a verb.
        "git stage -A && git commit -m x".to_string(),
        "git status && git commit -m x".to_string(),
        "git add -A && git commit -m x".to_string(),
        // (7) two commits, or two records.
        format!(
            "git commit -m x {RECORD} && git commit -m y docs/intent/other-change/acceptance.toml"
        ),
        format!("git commit -m x {RECORD} && git commit -m y {RECORD}"),
        // A part after the commit, and two `git add`.
        format!("git commit -m x {RECORD} && git status"),
        format!("git add {RECORD} && git add {RECORD} && git commit -m x"),
        // A second `cd`, a `cd` that is not first, a relative `cd`.
        format!("git add {RECORD} && cd '{root}' && git commit -m x"),
        format!("cd '{root}' && cd '{root}' && git commit -m x"),
        "cd docs && git commit -m x".to_string(),
        // Expansions outside and inside quotes.
        "git commit -m \"$HOME\"".to_string(),
        "git commit -m \"a\\\"b\"".to_string(),
        "git commit -m x ~/acceptance.toml".to_string(),
        "git commit -m x {a,b}".to_string(),
        // A quote that does not end, and an empty part.
        "git commit -m 'x".to_string(),
        "&& git commit -m x".to_string(),
        "git commit -m x &&".to_string(),
        // A `cd` or a `-C` folder that is no repository.
        format!("cd '{}' && git commit -m x", elsewhere.dir.display()),
        "git -C '/nonexistent/x' commit -m x".to_string(),
    ];
    for cmd in &cases {
        let out = answer_as(&r, cmd, Some("acceptor"));
        assert!(out.starts_with(DENY_START), "{cmd:?}: {out}");
        // The reason names the one allowed shape.
        assert!(
            out.contains("git add <path> && git commit -m '<text>' <path>"),
            "{cmd:?}: {out}"
        );
    }
}

/// The one allowed shape, in each variant.
#[test]
fn rule_d_allows_the_one_shape_and_nothing_near_it() {
    let r = project_with_record();
    r.write("src/a.py", "x = 2\n");
    let root = r.dir.to_string_lossy().into_owned();
    for cmd in [
        format!("git commit -m x {RECORD}"),
        format!("git add {RECORD} && git commit -m 'two words' {RECORD}"),
        format!("git add {RECORD} && git commit -m 'it'\"'\"'s' {RECORD}"),
        format!(
            "cd '{root}' && git add {RECORD} && git commit -m 'a; b (c) $ # ~ > < \\ | &' {RECORD}"
        ),
        format!("cd '{root}' && git add {RECORD} && git commit -m x"),
        format!("cd '{root}' && git -C '{root}' add {RECORD} && git -C '{root}' commit -m x"),
        format!("git -C '{root}' commit --message='two words' --no-verify -s {RECORD}"),
        format!("cd '{root}/docs' && git commit -m x intent/valve-timeout/acceptance.toml"),
    ] {
        assert_eq!(answer_as(&r, &cmd, Some("acceptor")), "", "{cmd}");
    }
}

/// Each character that a shell reads as an operator is denied also where a word
/// would hide it: in the text of a message that is not quoted. So the lexer itself
/// is proven, not only the checks of the paths after it.
#[test]
fn rule_d_denies_each_operator_character_inside_an_unquoted_word() {
    let r = project_with_record();
    for op in [
        ";", "|", "&", "(", ")", "{", "}", "<", ">", "$", "`", "#", "\\", "~",
    ] {
        for cmd in [
            format!("git commit -m x{op}y {RECORD}"),
            format!("git commit -m {op}y {RECORD}"),
        ] {
            let out = answer_as(&r, &cmd, Some("acceptor"));
            assert!(out.starts_with(DENY_START), "{cmd:?}: {out}");
        }
    }
    // A line feed ends a command in a shell.
    let out = answer_as(&r, &format!("git commit -m x\n{RECORD}"), Some("acceptor"));
    assert!(out.starts_with(DENY_START), "{out}");
    let out = answer_as(&r, "git commit -m x\r", Some("acceptor"));
    assert!(out.starts_with(DENY_START), "{out}");
}

/// In another repository the grammar still holds, and nothing may follow the commit.
#[test]
fn rule_d_keeps_the_grammar_for_a_scratch_repository() {
    let r = project_with_record();
    let scratch = Repo::new();
    scratch.write("f", "1\n");
    let s = scratch.dir.to_string_lossy().into_owned();
    for cmd in [
        format!("cd '{s}' && git commit -m x && git log"),
        format!("cd '{s}' && git add f && git commit -m x f; ls"),
        format!("cd '{s}' && git --git-dir=.git commit -m x"),
        format!("cd '{s}' && bash -c 'git commit -am x'"),
    ] {
        let out = answer_as(&r, &cmd, Some("acceptor"));
        assert!(out.starts_with(DENY_START), "{cmd:?}: {out}");
    }
}

/// Third pass: the trigger reads a call that hides the words, and a glob is no word
/// of the grammar.
#[test]
fn rule_d_reads_a_call_that_hides_the_word_commit_or_git() {
    let r = project_with_record();
    r.write("src/a.py", "x = 2\n");
    for cmd in [
        "GIT commit -am x",
        "git COMMIT -am x",
        "git comm\\\nit -am x",
        "git comm\\\r\nit -am x",
        "/usr/bin/g?t commit -am x",
        "git $'\\x63ommit' -am x",
        "x=omm; git c${x}it -am x",
        "GIT c$X -am x",
        "git co\"\"mmit -am x",
        "git c*t -am x",
        "git c[o]mmit -am x",
        "git {commit,status}",
        "git c? -am x",
        "git c{o,x}mmit -am x",
        "git c`echo o`mmit -am x",
        "git `echo commit` -am x",
    ] {
        let out = answer_as(&r, cmd, Some("acceptor"));
        assert!(out.starts_with(DENY_START), "{cmd:?}: {out}");
    }
    // A call with no `commit` and no dynamic word beside `git` is not read.
    for cmd in [
        "git log --oneline -5 | head",
        "git diff -- src/a.rs",
        "git status",
        "git status --short; ls",
        "git diff 'src/*.py'",
        "git log --format='%H $x'",
        "spec-dna check > /dev/null 2>&1",
        "echo $HOME",
        "ls src/*.py",
    ] {
        assert_eq!(answer_as(&r, cmd, Some("acceptor")), "", "{cmd}");
    }
}

#[test]
fn rule_d_denies_a_glob_in_each_word_and_in_the_folders() {
    let r = project_with_record();
    r.write("src/a.py", "x = 2\n");
    let root = r.dir.to_string_lossy().into_owned();
    for cmd in [
        "git commit -m *".to_string(),
        format!("git commit -m s* {RECORD}"),
        format!("git commit -m x? {RECORD}"),
        format!("git commit -m [x] {RECORD}"),
        format!("git commit -m x]y {RECORD}"),
        format!("git commit -m x[y {RECORD}"),
        format!("cd {root}/sr[c] && git commit -am x"),
        format!("cd '{root}/sr[c]' && git commit -am x"),
        format!("cd '{root}/s*' && git commit -m x {RECORD}"),
        "git -C sr[c] commit -am x".to_string(),
        "git -C 'sr[c]' commit -am x".to_string(),
        "git -C 's*' commit -m x".to_string(),
    ] {
        let out = answer_as(&r, &cmd, Some("acceptor"));
        assert!(out.starts_with(DENY_START), "{cmd:?}: {out}");
    }
    // A folder that exists under a literal name with a glob character is still a
    // pattern for the hook, and the index holds the record only.
    std::fs::create_dir_all(r.dir.join("sr[c]")).unwrap();
    std::fs::create_dir_all(r.dir.join("s*")).unwrap();
    r.git(&["add", RECORD]);
    for cmd in [
        format!("cd '{root}/sr[c]' && git commit -m x"),
        format!("cd '{root}/s*' && git commit -m x"),
        "git -C 'sr[c]' commit -m x".to_string(),
        "git -C 's*' commit -m x".to_string(),
    ] {
        let out = answer_as(&r, &cmd, Some("acceptor"));
        assert!(out.starts_with(DENY_START), "{cmd:?}: {out}");
    }
    assert_eq!(answer_as(&r, "git commit -m x", Some("acceptor")), "");
    // A glob character in a quoted message is text.
    assert_eq!(
        answer_as(
            &r,
            &format!("git commit -m 's* [x] ?' {RECORD}"),
            Some("acceptor")
        ),
        ""
    );
}

/// The prescribed step 7 form with a root that has a space and a quote: `next`
/// writes `'` as `'\''`, the way a shell puts a quote in a single-quoted word.
#[test]
fn rule_d_allows_the_quote_sequence_of_a_root_with_a_quote_and_nothing_else_with_a_backslash() {
    let dir = std::env::temp_dir().join(format!("spec-dna r o'q {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("docs/intent/valve-timeout")).unwrap();
    common::git(&dir, &["init", "-q", "-b", "main"]);
    std::fs::write(dir.join(NEW_RECORD), "format = 2\n").unwrap();
    let elsewhere = Repo::new();
    let quoted = format!("'{}'", dir.to_string_lossy().replace('\'', "'\\''"));
    let ask = |cmd: &str| {
        let ev = event_as(cmd, &elsewhere.dir.to_string_lossy(), Some("acceptor"));
        let (code, out, err) = hook_in(&elsewhere.dir, Some(&dir), None, ev.as_bytes());
        assert_eq!(code, 0, "{out}{err}");
        out
    };
    assert!(quoted.contains("'\\''"), "{quoted}");
    for cmd in [
        format!("cd {quoted} && git add {NEW_RECORD} && git commit -m 'two words' {NEW_RECORD}"),
        format!("cd {quoted} && git commit -m x {NEW_RECORD}"),
        format!("git -C {quoted} commit -m x {NEW_RECORD}"),
    ] {
        assert_eq!(ask(&cmd), "", "{cmd}");
    }
    // The same root, with a dirty option: denied, so the folder was read right.
    let out = ask(&format!("cd {quoted} && git commit -am x"));
    assert!(out.starts_with(DENY_START), "{out}");
    // Another use of a backslash is denied, also in the sequence of a message.
    for cmd in [
        format!("cd {quoted} && git commit -m x\\ y {NEW_RECORD}"),
        format!("cd {quoted} && git commit -m 'a'\\\" {NEW_RECORD}"),
        format!("cd {quoted} && git commit -m 'a'\\'b {NEW_RECORD}"),
        format!("cd {quoted} && git commit -m 'a'\\\\'b' {NEW_RECORD}"),
        format!("cd {quoted} && git commit -m \"a\"\\'b'' {NEW_RECORD}"),
    ] {
        let out = ask(&cmd);
        assert!(out.starts_with(DENY_START), "{cmd:?}: {out}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
