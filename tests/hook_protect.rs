//! Tests of `spec-dna hook protect-tests` and of its entry in
//! `.claude/settings.json` (step 4, S4). Each test uses a temporary
//! repository. No test reads or writes a file of this repository.

mod common;

use common::{BIN, Repo};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const TESTS: [&str; 4] = ["--test", "**/tests/**", "--test", "**/*_test.*"];

/// Escapes a string for a JSON string literal.
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// A `PreToolUse` event of Claude Code, with the fields that the hook reads.
fn event(tool: &str, file: &str, cwd: &str) -> String {
    format!(
        "{{\"session_id\":\"s\",\"cwd\":\"{}\",\"hook_event_name\":\"PreToolUse\",\"tool_name\":\"{tool}\",\"tool_input\":{{\"file_path\":\"{}\",\"old_string\":\"a\",\"new_string\":\"b\"}},\"tool_use_id\":\"t\"}}",
        esc(cwd),
        esc(file)
    )
}

/// The answer of the hook for a tracked test file on a fix branch.
fn ask_fix(path: &str, branch: &str) -> String {
    format!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\"permissionDecision\":\"ask\",\"permissionDecisionReason\":\"spec-dna protect-tests: {path} is a test file that git tracks, and {branch} is a fix branch. A fix changes the code, not the test. Approve the edit only if the test is wrong.\"}}}}\n"
    )
}

fn ask_always(path: &str) -> String {
    format!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PreToolUse\",\"permissionDecision\":\"ask\",\"permissionDecisionReason\":\"spec-dna protect-tests: {path} is a test file that git tracks. Approve the edit only if the test is wrong.\"}}}}\n"
    )
}

/// Runs `spec-dna hook protect-tests` in `dir`. `project` is the value of
/// `CLAUDE_PROJECT_DIR`. Returns the exit code, stdout and stderr.
fn hook(dir: &Path, project: Option<&Path>, flags: &[&str], stdin: &[u8]) -> (i32, String, String) {
    let mut cmd = Command::new(BIN);
    cmd.args(["hook", "protect-tests"])
        .args(flags)
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
    let mut child = cmd.spawn().unwrap();
    // The hook can exit before it reads all the input.
    let _ = child.stdin.take().unwrap().write_all(stdin);
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A repository on branch `main` with tracked test files and tracked code.
fn project() -> Repo {
    let r = Repo::new();
    r.write("tests/a.rs", "fn a() {}\n");
    r.write("tests/it's [x].rs", "fn q() {}\n");
    r.write("src/lib.rs", "fn lib() {}\n");
    r.write("src/util_test.rs", "fn u() {}\n");
    r.commit("start");
    r
}

fn path_of(r: &Repo, rel: &str) -> String {
    r.dir.join(rel).to_string_lossy().into_owned()
}

/// Runs the hook for an `Edit` of `rel` with the project dir `r.dir`.
fn edit(r: &Repo, rel: &str, flags: &[&str]) -> (i32, String, String) {
    let ev = event("Edit", &path_of(r, rel), &r.dir.to_string_lossy());
    hook(&r.dir, Some(&r.dir), flags, ev.as_bytes())
}

fn fix_flags() -> Vec<&'static str> {
    let mut f = vec!["--mode", "fix-branches"];
    f.extend(TESTS);
    f
}

fn always_flags() -> Vec<&'static str> {
    let mut f = vec!["--mode", "always"];
    f.extend(TESTS);
    f
}

// ---- The cases of the hook ----

#[test]
fn a_tracked_test_file_on_a_fix_branch_asks() {
    let r = project();
    r.git(&["switch", "-q", "-c", "fix/x"]);
    let (code, out, err) = edit(&r, "tests/a.rs", &fix_flags());
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(out, ask_fix("tests/a.rs", "fix/x"));
    // The glob `**/*_test.*` also matches, and `Write` is covered as well.
    let ev = event(
        "Write",
        &path_of(&r, "src/util_test.rs"),
        &r.dir.to_string_lossy(),
    );
    let (code, out, _) = hook(&r.dir, Some(&r.dir), &fix_flags(), ev.as_bytes());
    assert_eq!(code, 0);
    assert_eq!(out, ask_fix("src/util_test.rs", "fix/x"));
}

#[test]
fn another_branch_gives_no_output() {
    let r = project();
    for branch in ["main", "feature/x", "hotfix/x", "fixture"] {
        if branch != "main" {
            r.git(&["switch", "-q", "-c", branch]);
        }
        let (code, out, err) = edit(&r, "tests/a.rs", &fix_flags());
        assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""), "{branch}");
    }
}

#[test]
fn a_detached_head_gives_no_output() {
    let r = project();
    r.git(&["switch", "-q", "--detach"]);
    let (code, out, _) = edit(&r, "tests/a.rs", &fix_flags());
    assert_eq!((code, out.as_str()), (0, ""));
}

#[test]
fn a_new_test_file_gives_no_output() {
    let r = project();
    r.git(&["switch", "-q", "-c", "fix/x"]);
    // The file does not exist, or it exists and git does not track it.
    let (code, out, err) = edit(&r, "tests/new.rs", &fix_flags());
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    r.write("tests/new.rs", "fn n() {}\n");
    let (code, out, err) = edit(&r, "tests/new.rs", &fix_flags());
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    // A staged file is tracked.
    r.git(&["add", "tests/new.rs"]);
    let (_, out, _) = edit(&r, "tests/new.rs", &fix_flags());
    assert_eq!(out, ask_fix("tests/new.rs", "fix/x"));
}

#[test]
fn a_file_that_is_not_a_test_gives_no_output() {
    let r = project();
    r.git(&["switch", "-q", "-c", "fix/x"]);
    let (code, out, err) = edit(&r, "src/lib.rs", &fix_flags());
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
}

#[test]
fn the_mode_always_asks_on_each_branch() {
    let r = project();
    let (code, out, err) = edit(&r, "tests/a.rs", &always_flags());
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(out, ask_always("tests/a.rs"));
    // A new file does not trigger the mode `always` either.
    let (_, out, _) = edit(&r, "tests/new.rs", &always_flags());
    assert_eq!(out, "");
    // The value can follow `--mode=`. An unknown value acts as `fix-branches`.
    let mut f = vec!["--mode=always"];
    f.extend(TESTS);
    assert_eq!(edit(&r, "tests/a.rs", &f).1, ask_always("tests/a.rs"));
    let mut f = vec!["--mode", "sometimes"];
    f.extend(TESTS);
    assert_eq!(edit(&r, "tests/a.rs", &f).1, "");
    r.git(&["switch", "-q", "-c", "fix/x"]);
    assert_eq!(edit(&r, "tests/a.rs", &f).1, ask_fix("tests/a.rs", "fix/x"));
}

#[test]
fn an_empty_list_of_globs_gives_no_output() {
    let r = project();
    r.git(&["switch", "-q", "-c", "fix/x"]);
    for flags in [
        vec![],
        vec!["--mode", "always"],
        vec!["--mode=fix-branches", "--unknown", "x"],
    ] {
        let (code, out, err) = edit(&r, "tests/a.rs", &flags);
        assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""), "{flags:?}");
    }
}

#[test]
fn a_path_with_a_quote_or_a_glob_character_is_a_literal_path() {
    let r = project();
    r.git(&["switch", "-q", "-c", "fix/x"]);
    let (_, out, _) = edit(&r, "tests/it's [x].rs", &fix_flags());
    assert_eq!(out, ask_fix("tests/it's [x].rs", "fix/x"));
    // The pathspec is literal: the name `a.r?` does not match `a.rs`.
    let (_, out, _) = edit(&r, "tests/a.r?", &fix_flags());
    assert_eq!(out, "");
}

#[test]
fn a_bad_input_gives_no_output_and_exit_code_0() {
    let r = project();
    r.git(&["switch", "-q", "-c", "fix/x"]);
    let ok = path_of(&r, "tests/a.rs");
    let cwd = r.dir.to_string_lossy().into_owned();
    let bad: Vec<(&str, Vec<u8>)> = vec![
        ("empty", Vec::new()),
        ("not JSON", b"edit tests/a.rs".to_vec()),
        ("array", b"[1,2]".to_vec()),
        (
            "truncated",
            event("Edit", &ok, &cwd).as_bytes()[..40].to_vec(),
        ),
        ("not UTF-8", vec![0xff, 0xfe, 0x00, 0x7b]),
        ("another tool", event("Read", &ok, &cwd).into_bytes()),
        ("Bash", event("Bash", &ok, &cwd).into_bytes()),
        (
            "no tool_input",
            b"{\"tool_name\":\"Edit\",\"cwd\":\"/\"}".to_vec(),
        ),
        (
            "file_path is a number",
            b"{\"tool_name\":\"Edit\",\"tool_input\":{\"file_path\":5}}".to_vec(),
        ),
        (
            "a relative path",
            event("Edit", "tests/a.rs", &cwd).into_bytes(),
        ),
        (
            "a dot-dot segment",
            event("Edit", &format!("{cwd}/src/../tests/a.rs"), &cwd).into_bytes(),
        ),
        (
            "a path outside the root",
            event("Edit", "/etc/tests/passwd", &cwd).into_bytes(),
        ),
        (
            "a path that is the root",
            event("Edit", &cwd, &cwd).into_bytes(),
        ),
    ];
    for (name, input) in bad {
        let (code, out, err) = hook(&r.dir, Some(&r.dir), &fix_flags(), &input);
        assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""), "{name}");
    }
    // An unknown hook name.
    let out = Command::new(BIN)
        .args(["hook", "no-such-hook"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!((out.status.code(), out.stdout.len()), (Some(0), 0));
}

#[test]
fn a_directory_that_is_not_a_git_repository_gives_no_output() {
    let r = Repo::empty();
    r.write("tests/a.rs", "fn a() {}\n");
    let (code, out, err) = edit(&r, "tests/a.rs", &always_flags());
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
}

#[test]
fn the_root_is_the_cwd_when_the_project_dir_is_absent() {
    let r = project();
    r.git(&["switch", "-q", "-c", "fix/x"]);
    let ev = event("Edit", &path_of(&r, "tests/a.rs"), &r.dir.to_string_lossy());
    let (code, out, _) = hook(&r.dir, None, &fix_flags(), ev.as_bytes());
    assert_eq!(code, 0);
    assert_eq!(out, ask_fix("tests/a.rs", "fix/x"));
}

#[test]
fn the_project_dir_wins_over_the_cwd() {
    let r = project();
    r.git(&["switch", "-q", "-c", "fix/x"]);
    // The agent did `cd src`. The paths of the globs stay relative to the
    // project dir.
    let ev = event("Edit", &path_of(&r, "tests/a.rs"), &path_of(&r, "src"));
    let (_, out, _) = hook(&r.dir, Some(&r.dir), &fix_flags(), ev.as_bytes());
    assert_eq!(out, ask_fix("tests/a.rs", "fix/x"));
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_in_the_root_is_resolved() {
    let r = project();
    r.git(&["switch", "-q", "-c", "fix/x"]);
    let holder = Repo::empty();
    let link = holder.dir.join("link");
    std::os::unix::fs::symlink(&r.dir, &link).unwrap();
    let real = fs::canonicalize(&r.dir).unwrap();
    // The root has the spelling of the link, and the path has the real one.
    let ev = event(
        "Edit",
        &real.join("tests/a.rs").to_string_lossy(),
        &link.to_string_lossy(),
    );
    let (code, out, _) = hook(&r.dir, Some(&link), &fix_flags(), ev.as_bytes());
    assert_eq!(code, 0);
    assert_eq!(out, ask_fix("tests/a.rs", "fix/x"));
    // The root has the real spelling, and the path has the link.
    let ev = event(
        "Edit",
        &link.join("tests/a.rs").to_string_lossy(),
        &real.to_string_lossy(),
    );
    let (_, out, _) = hook(&r.dir, Some(&real), &fix_flags(), ev.as_bytes());
    assert_eq!(out, ask_fix("tests/a.rs", "fix/x"));
}

// ---- A worktree ----

/// A linked worktree of `r` on the new branch `branch`, in its own folder.
fn worktree(r: &Repo, branch: &str) -> Repo {
    let wt = Repo::empty();
    r.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        branch,
        wt.dir.to_str().unwrap(),
    ]);
    wt
}

#[test]
fn a_session_in_a_linked_worktree_asks() {
    let r = project();
    let wt = worktree(&r, "fix/w");
    // The session starts in the worktree: the project dir is the worktree.
    let (code, out, err) = edit(&wt, "tests/a.rs", &fix_flags());
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(out, ask_fix("tests/a.rs", "fix/w"));
    // The main checkout is on `main`, so its edits get no question.
    let (_, out, _) = edit(&r, "tests/a.rs", &fix_flags());
    assert_eq!(out, "");
    r.git(&["worktree", "remove", "--force", wt.dir.to_str().unwrap()]);
}

#[test]
fn a_worktree_outside_the_main_checkout_is_not_seen_from_the_main_project_dir() {
    // D77. Claude Code keeps `CLAUDE_PROJECT_DIR` at the main checkout when
    // the agent enters a worktree, and only the `cwd` moves. The root is the
    // project dir, so a file of a worktree outside it has no relative path,
    // and the hook gives no output. This test pins that limit.
    let r = project();
    let wt = worktree(&r, "fix/w");
    let ev = event(
        "Edit",
        &path_of(&wt, "tests/a.rs"),
        &wt.dir.to_string_lossy(),
    );
    let (code, out, err) = hook(&wt.dir, Some(&r.dir), &fix_flags(), ev.as_bytes());
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    // With no project dir, the `cwd` is the root and the hook asks.
    let (_, out, _) = hook(&wt.dir, None, &fix_flags(), ev.as_bytes());
    assert_eq!(out, ask_fix("tests/a.rs", "fix/w"));
    r.git(&["worktree", "remove", "--force", wt.dir.to_str().unwrap()]);
}

#[test]
fn a_worktree_inside_the_main_checkout_is_not_seen_either() {
    // D77. The worktree is in `.claude/worktrees/` of the main checkout. The
    // relative path from the project dir starts with that folder, and the
    // main checkout does not track it, so the hook gives no output. The main
    // checkout is on a fix branch here, to show that its branch does not
    // matter.
    let r = project();
    r.git(&["switch", "-q", "-c", "fix/m"]);
    let inner = r.dir.join(".claude/worktrees/in");
    fs::create_dir_all(inner.parent().unwrap()).unwrap();
    r.git(&[
        "worktree",
        "add",
        "-q",
        "-b",
        "fix/in",
        inner.to_str().unwrap(),
    ]);
    let ev = event(
        "Edit",
        &inner.join("tests/a.rs").to_string_lossy(),
        &inner.to_string_lossy(),
    );
    let (code, out, err) = hook(&inner, Some(&r.dir), &fix_flags(), ev.as_bytes());
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    // The same file, with the worktree as the root, gets the question.
    let (_, out, _) = hook(&inner, Some(&inner), &fix_flags(), ev.as_bytes());
    assert_eq!(out, ask_fix("tests/a.rs", "fix/in"));
    r.git(&["worktree", "remove", "--force", inner.to_str().unwrap()]);
}

// ---- The entry in .claude/settings.json ----

const HANDLER: &str = "spec-dna hook protect-tests";

/// A committed project with a document and the table `[harness]`.
fn configured(harness: &str, extra: &str) -> Repo {
    let r = Repo::with_config(&format!(
        "\n[[roots]]\npath = \"docs\"\n{extra}\n[harness]\nlevel = \"lean\"\n{harness}"
    ));
    r.write("docs/a.md", "---\ntype: research\n---\n\n# A\n\nText.\n");
    r.commit("start");
    r
}

/// The commands of the handlers in `.claude/settings.json` that start with
/// `prefix`.
fn commands(r: &Repo, prefix: &str) -> Vec<String> {
    let text = r.read(".claude/settings.json");
    text.lines()
        .filter_map(|l| l.trim().strip_prefix("\"command\": \""))
        .map(|c| {
            c.trim_end_matches(',')
                .trim_end_matches('"')
                .replace("\\\"", "\"")
                .replace("\\\\", "\\")
        })
        .filter(|c| c.starts_with(prefix))
        .collect()
}

const DEFAULT_COMMAND: &str = "spec-dna hook protect-tests --mode fix-branches --test '**/tests/**' --test '**/test/**' --test '**/__tests__/**' --test '**/*_test.*' --test '**/*_tests.*' --test '**/test_*.*' --test '**/*.test.*' --test '**/*.spec.*' --test '**/*Test.*' --test '**/*_spec.*' 2>/dev/null || true";

fn golden(name: &str) -> String {
    let p = format!("{}/tests/golden/harness/{name}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("{p}: {e}"))
}

#[test]
fn init_writes_the_entry_with_the_default_globs() {
    let r = configured("protect-tests = \"fix-branches\"\n", "");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(commands(&r, HANDLER), [DEFAULT_COMMAND]);
    let settings = r.read(".claude/settings.json");
    // The handler has its own group, with the matcher `Edit|Write`.
    assert!(
        settings.contains("\"matcher\": \"Edit|Write\""),
        "{settings}"
    );
    assert_eq!(settings, golden("settings-lean.json"));
    // The entry changes `check` not at all.
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("SD50"), "{out}");
    // The hook text of the preview names the version that the hook needs.
    let (_, out, _) = r.run(&["init", "--dry-run"]);
    assert!(out.contains("spec-dna 0.6.0 or later"), "{out}");
}

#[test]
fn the_default_gives_no_entry() {
    for harness in ["", "protect-tests = \"off\"\n"] {
        let r = configured(harness, "");
        let (code, out, err) = r.run(&["init"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(commands(&r, HANDLER).is_empty(), "{harness}");
        assert!(!r.read(".claude/settings.json").contains("Edit|Write"));
        assert!(out.contains("spec-dna 0.5.0 or later"), "{out}");
        assert!(!out.contains("spec-dna 0.6.0 or later"), "{out}");
    }
}

#[test]
fn the_globs_of_evidence_go_into_the_command_and_a_quote_is_escaped() {
    let r = configured(
        "protect-tests = \"always\"\n",
        "\n[evidence]\ntests = [\"t/**\", \"**/it's_*.rs\"]\n",
    );
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        commands(&r, HANDLER),
        [
            "spec-dna hook protect-tests --mode always --test 't/**' --test '**/it'\\''s_*.rs' 2>/dev/null || true"
        ]
    );
}

#[test]
fn no_globs_and_no_entry_for_verified_by_or_an_empty_list() {
    let r = configured(
        "protect-tests = \"fix-branches\"\n",
        "\n[evidence]\ntests = []\n",
    );
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(commands(&r, HANDLER).is_empty());
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("SD50"), "{out}");
}

#[test]
fn level_none_gives_no_entry() {
    let r = Repo::with_config(
        "\n[[roots]]\npath = \"docs\"\n\n[harness]\nlevel = \"none\"\nprotect-tests = \"always\"\n",
    );
    r.write("docs/a.md", "---\ntype: research\n---\n\n# A\n\nText.\n");
    r.commit("start");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(".claude/settings.json").exists());
}

#[test]
fn a_new_configuration_uses_the_globs_that_check_reads() {
    // `init` with no `spec-dna.toml`: the hook gets the globs of the text
    // that `render` wrote, as `check` reads them. An empty repository has
    // the 10 default globs. No SD502 follows.
    let r = Repo::new();
    r.write("docs/a.md", "---\ntype: research\n---\n\n# A\n\nText.\n");
    r.commit("start");
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    // The answers of the new configuration set no `protect-tests`, so the
    // value that the file has decides. Set it and run `--reconfigure`.
    let toml = r.read("spec-dna.toml");
    assert!(toml.contains("protect-tests = \"off\""), "{toml}");
    r.write(
        "spec-dna.toml",
        &toml.replace(
            "protect-tests = \"off\"",
            "protect-tests = \"fix-branches\"",
        ),
    );
    r.commit("protect");
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let cmds = commands(&r, HANDLER);
    assert_eq!(cmds.len(), 1, "{cmds:?}");
    // The documents of this repository have no layer for their type, so `check`
    // has an error that is not about the harness.
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(!out.contains("SD50"), "{out}");
}

#[test]
fn reconfigure_gives_the_hook_the_globs_of_the_new_text() {
    // The answer `verified-by` writes `tests = []`. The hook uses the globs
    // that `check` reads from the new text, so it has no glob and goes.
    let r = configured("protect-tests = \"fix-branches\"\n", "");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(commands(&r, HANDLER).len(), 1);
    r.commit("init");
    let answers = format!("{}-answers.toml", r.dir.display());
    fs::write(&answers, "evidence-style = \"verified-by\"\n").unwrap();
    let (code, out, err) = r.run(&["init", "--reconfigure", "--answers", &answers]);
    let _ = fs::remove_file(&answers);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.read("spec-dna.toml").contains("tests = []"));
    assert!(commands(&r, HANDLER).is_empty());
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(!out.contains("SD50"), "{out}");
}

#[test]
fn a_change_of_the_globs_gives_sd502_until_init_runs() {
    let r = configured("protect-tests = \"fix-branches\"\n", "");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    let toml = r.read("spec-dna.toml");
    r.write(
        "spec-dna.toml",
        &format!("{toml}\n[evidence]\ntests = [\"spec/**\"]\n"),
    );
    r.commit("globs");
    let (code, out, err) = r.run(&["check", "--all", "--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.contains("\"code\":\"SD502\"")
            && out.contains("hooks.PreToolUse (spec-dna hook protect-tests)"),
        "{out}"
    );
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        commands(&r, HANDLER),
        ["spec-dna hook protect-tests --mode fix-branches --test 'spec/**' 2>/dev/null || true"]
    );
    let (code, out, err) = r.run(&["check", "--all", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("SD502"), "{out}");
}

#[test]
fn a_missing_entry_gives_sd502() {
    let r = configured("protect-tests = \"fix-branches\"\n", "");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    r.write(".claude/settings.json", "{}\n");
    r.commit("empty settings");
    let (_, out, _) = r.run(&["check", "--all", "--json"]);
    assert!(
        out.contains("\"code\":\"SD502\"")
            && out.contains("hooks.PreToolUse (spec-dna hook protect-tests)"),
        "{out}"
    );
}

#[test]
fn reconfigure_with_off_removes_only_the_handler() {
    let r = configured("protect-tests = \"fix-branches\"\n", "");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    // A hook of the owner shares the event.
    let text = r.read(".claude/settings.json");
    assert!(text.contains("\"Edit|Write\""));
    let toml = r.read("spec-dna.toml");
    r.write(
        "spec-dna.toml",
        &toml.replace(
            "protect-tests = \"fix-branches\"",
            "protect-tests = \"off\"",
        ),
    );
    r.commit("off");
    // A plain `init` writes and never removes, as for the 2 hooks of 0.5.
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(commands(&r, HANDLER).len(), 1);
    assert!(!out.contains("kept hooks"), "{out}");
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
    // `--reconfigure` removes the handler and its empty group.
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(commands(&r, HANDLER).is_empty());
    let text = r.read(".claude/settings.json");
    assert!(!text.contains("Edit|Write"), "{text}");
    // The 2 other hooks stay.
    assert_eq!(commands(&r, "spec-dna hook ").len(), 2, "{text}");
}

#[test]
fn a_handler_of_the_owner_with_the_same_matcher_stays() {
    let r = configured("protect-tests = \"fix-branches\"\n", "");
    r.write(
        ".claude/settings.json",
        "{\n  \"hooks\": {\n    \"PreToolUse\": [\n      {\n        \"matcher\": \"Edit|Write\",\n        \"hooks\": [\n          {\n            \"type\": \"command\",\n            \"command\": \"./my-guard.sh\"\n          }\n        ]\n      }\n    ]\n  }\n}\n",
    );
    r.commit("owner hook");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(commands(&r, "./my-guard.sh"), ["./my-guard.sh"]);
    assert_eq!(commands(&r, HANDLER).len(), 1);
    let toml = r.read("spec-dna.toml");
    r.write(
        "spec-dna.toml",
        &toml.replace(
            "protect-tests = \"fix-branches\"",
            "protect-tests = \"off\"",
        ),
    );
    r.commit("off");
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(commands(&r, HANDLER).is_empty());
    assert_eq!(commands(&r, "./my-guard.sh"), ["./my-guard.sh"]);
}

// ---- The generated command, end to end ----

#[cfg(unix)]
#[test]
fn the_generated_command_runs_in_a_shell_with_a_quote_in_a_glob() {
    let r = configured(
        "protect-tests = \"fix-branches\"\n",
        "\n[evidence]\ntests = [\"**/it's_*.rs\", \"**/tests/**\"]\n",
    );
    r.write("tests/it's_a.rs", "fn q() {}\n");
    r.commit("tests");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    r.git(&["switch", "-q", "-c", "fix/q"]);
    let cmd = commands(&r, HANDLER).pop().unwrap();
    // The command starts `spec-dna`, so a temporary folder with a link of that
    // name is put on PATH.
    let holder = Repo::empty();
    let bin_dir = holder.dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    std::os::unix::fs::symlink(BIN, bin_dir.join("spec-dna")).unwrap();
    let path = common::path_with(&bin_dir);
    let run = |file: &str| {
        let ev = event("Edit", &path_of(&r, file), &r.dir.to_string_lossy());
        let mut child = Command::new("sh")
            .args(["-c", &cmd])
            .current_dir(&r.dir)
            .env("PATH", &path)
            .env("CLAUDE_PROJECT_DIR", &r.dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let _ = child.stdin.take().unwrap().write_all(ev.as_bytes());
        let out = child.wait_with_output().unwrap();
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            out.stderr.len(),
        )
    };
    let (code, out, errlen) = run("tests/it's_a.rs");
    assert_eq!((code, errlen), (Some(0), 0));
    assert_eq!(out, ask_fix("tests/it's_a.rs", "fix/q"));
    let (code, out, _) = run("src/lib.rs");
    assert_eq!((code, out.as_str()), (Some(0), ""));
    // The same command with no `spec-dna` on PATH gives no output, no error
    // text and exit code 0.
    let ev = event("Edit", &path_of(&r, "tests/it's_a.rs"), "/");
    let mut child = Command::new("/bin/sh")
        .args(["-c", &cmd])
        .current_dir(&r.dir)
        .env("PATH", "/nonexistent")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let _ = child.stdin.take().unwrap().write_all(ev.as_bytes());
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
}
