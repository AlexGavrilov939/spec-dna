//! Integration tests of `spec-dna hook`. Each test runs the binary in an empty
//! directory with its own `TMPDIR`, so the meter state is not shared.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_spec-dna");
static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A unique directory that is removed when the value is dropped.
struct Dir(PathBuf);

impl Dir {
    fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("spec-dna-hook-cli-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        Dir(dir)
    }

    fn state_files(&self) -> usize {
        fs::read_dir(self.0.join("tmp").join("spec-dna-meter"))
            .map(|d| d.count())
            .unwrap_or(0)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Runs a hook. It returns the exit code, stdout and stderr.
fn hook(dir: &Dir, args: &[&str], stdin: &str, env: &[(&str, &str)]) -> (i32, String, String) {
    let tmp = dir.0.join("tmp");
    fs::create_dir_all(&tmp).unwrap();
    let mut cmd = Command::new(BIN);
    cmd.arg("hook")
        .args(args)
        .current_dir(&dir.0)
        .env("TMPDIR", &tmp)
        .env_remove("SPEC_DNA_FANOUT_CAP")
        .env_remove("CLAUDE_PROJECT_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().unwrap();
    // The hook can exit before it reads all the input.
    let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn assistant(tokens: u64, text_len: usize) -> String {
    format!(
        r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"text","text":"{}"}}],"usage":{{"input_tokens":1,"cache_creation_input_tokens":0,"cache_read_input_tokens":{tokens},"output_tokens":5}}}}}}"#,
        "t".repeat(text_len)
    )
}

fn write_transcript(path: &Path, lines: &[String]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, lines.join("\n") + "\n").unwrap();
}

fn event(transcript: &Path, session: &str, agent: Option<&str>) -> String {
    let agent = agent.map_or(String::new(), |a| format!(r#","agent_id":"{a}""#));
    format!(
        r#"{{"session_id":"{session}","transcript_path":"{}","hook_event_name":"PostToolUse"{agent}}}"#,
        transcript.display()
    )
}

#[test]
fn meter_main_speaks_once_for_two_calls() {
    let dir = Dir::new();
    let t = dir.0.join("s1.jsonl");
    write_transcript(&t, &[assistant(310_000, 5)]);
    let ev = event(&t, "s1", None);
    let args = ["context-meter", "--levels", "300000,600000"];
    let (code, out, err) = hook(&dir, &args, &ev, &[]);
    assert_eq!(code, 0);
    assert_eq!(err, "");
    assert!(out.starts_with(r#"{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"spec-dna context meter: this session has about 310K tokens of context, and each request"#), "{out}");
    assert!(out.ends_with("\"}}\n"), "{out}");
    let (code, out, _) = hook(&dir, &args, &ev, &[]);
    assert_eq!((code, out.as_str()), (0, ""));
    assert_eq!(dir.state_files(), 1);
}

#[test]
fn meter_higher_level_and_reset_after_a_drop() {
    let dir = Dir::new();
    let t = dir.0.join("s1.jsonl");
    let ev = event(&t, "s1", None);
    let args = ["context-meter", "--levels", "300000,600000"];
    write_transcript(&t, &[assistant(610_000, 5)]);
    let (_, out, _) = hook(&dir, &args, &ev, &[]);
    assert!(
        out.contains("Save the state of the task in a file"),
        "{out}"
    );
    // A compaction: the context falls under 2/3 of the first level.
    write_transcript(&t, &[assistant(50_000, 5)]);
    let (_, out, _) = hook(&dir, &args, &ev, &[]);
    assert_eq!(out, "");
    assert_eq!(dir.state_files(), 0);
    write_transcript(&t, &[assistant(310_000, 5)]);
    let (_, out, _) = hook(&dir, &args, &ev, &[]);
    assert!(out.contains("Delegate long searches"), "{out}");
}

#[test]
fn meter_subagent_reads_the_subagent_transcript() {
    let dir = Dir::new();
    let parent = dir.0.join("proj").join("s1.jsonl");
    let sub = dir.0.join("proj/s1/subagents/agent-a1.jsonl");
    write_transcript(&parent, &[assistant(900_000, 5)]);
    write_transcript(&sub, &[assistant(260_000, 5)]);
    let ev = event(&parent, "s1", Some("a1"));
    let args = [
        "context-meter",
        "--levels",
        "300000",
        "--subagent",
        "250000",
    ];
    let (code, out, _) = hook(&dir, &args, &ev, &[]);
    assert_eq!(code, 0);
    assert!(out.contains("this subagent has about 260K tokens"), "{out}");
    // The subagent meter is off with 0.
    let off = ["context-meter", "--subagent", "0"];
    let (_, out, _) = hook(&dir, &off, &ev, &[]);
    assert_eq!(out, "");
}

#[test]
fn meter_reads_a_long_last_line() {
    let dir = Dir::new();
    let t = dir.0.join("s1.jsonl");
    write_transcript(&t, &[assistant(900_000, 5), assistant(320_000, 600_000)]);
    let (_, out, _) = hook(
        &dir,
        &["context-meter", "--levels", "300000"],
        &event(&t, "s1", None),
        &[],
    );
    assert!(out.contains("about 320K tokens"), "{out}");
}

/// An assistant line with a top-level usage that holds `top` tokens and one
/// iteration for each `(type, tokens)` pair.
fn assistant_iterations(top: u64, iterations: &[(&str, u64)]) -> String {
    let its: Vec<String> = iterations
        .iter()
        .map(|(kind, n)| {
            format!(
                r#"{{"input_tokens":1,"output_tokens":5,"cache_read_input_tokens":{n},"cache_creation_input_tokens":0,"type":"{kind}"}}"#
            )
        })
        .collect();
    format!(
        r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"text","text":"t"}}],"usage":{{"input_tokens":1,"cache_creation_input_tokens":0,"cache_read_input_tokens":{top},"output_tokens":5,"iterations":[{}]}}}}}}"#,
        its.join(",")
    )
}

#[test]
fn meter_is_not_cumulative() {
    // The meter reads the last request, not the sum of the requests.
    let dir = Dir::new();
    let t = dir.0.join("s1.jsonl");
    write_transcript(&t, &[assistant(100_000, 5), assistant(120_000, 5)]);
    let (_, out, _) = hook(
        &dir,
        &["context-meter", "--levels", "110000"],
        &event(&t, "s1", None),
        &[],
    );
    assert!(out.contains("about 120K tokens"), "{out}");
}

#[test]
fn meter_counts_the_last_executor_iteration() {
    // The top-level usage is the sum of 2 iterations of 130K: the context is 130K.
    let dir = Dir::new();
    let t = dir.0.join("s1.jsonl");
    let line = assistant_iterations(260_000, &[("message", 130_000), ("message", 130_000)]);
    write_transcript(&t, &[line]);
    let (code, out, _) = hook(
        &dir,
        &["context-meter", "--levels", "250000"],
        &event(&t, "s1", None),
        &[],
    );
    assert_eq!((code, out.as_str()), (0, ""));
    assert_eq!(dir.state_files(), 0);
}

#[test]
fn meter_skips_a_synthetic_line() {
    let dir = Dir::new();
    let t = dir.0.join("s1.jsonl");
    let ev = event(&t, "s1", None);
    let args = ["context-meter", "--levels", "250000"];
    write_transcript(&t, &[assistant(260_000, 5)]);
    let (_, out, _) = hook(&dir, &args, &ev, &[]);
    assert!(out.contains("about 260K tokens"), "{out}");
    // A synthetic line with a usage of 0 must not reset the state.
    let synthetic = r#"{"type":"assistant","message":{"role":"assistant","model":"<synthetic>","content":[{"type":"text","text":"x"}],"usage":{"input_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":0}}}"#;
    write_transcript(&t, &[assistant(260_000, 5), synthetic.to_string()]);
    let (_, out, _) = hook(&dir, &args, &ev, &[]);
    assert_eq!(out, "");
    assert_eq!(dir.state_files(), 1);
    // The next real line of the same size is the same crossing: no new notice.
    write_transcript(
        &t,
        &[
            assistant(260_000, 5),
            synthetic.to_string(),
            assistant(262_000, 5),
        ],
    );
    let (_, out, _) = hook(&dir, &args, &ev, &[]);
    assert_eq!(out, "");
}

#[test]
fn meter_ignores_an_iteration_with_no_token_fields() {
    // A new shape of an iteration must not reset the meter: the top-level usage counts.
    let dir = Dir::new();
    let t = dir.0.join("s1.jsonl");
    let ev = event(&t, "s1", None);
    let args = ["context-meter", "--levels", "250000"];
    write_transcript(&t, &[assistant(260_000, 5)]);
    let (_, out, _) = hook(&dir, &args, &ev, &[]);
    assert!(out.contains("about 260K tokens"), "{out}");
    let odd = r#"{"type":"assistant","message":{"role":"assistant","usage":{"cache_read_input_tokens":260000,"iterations":[{"type":"message"}]}}}"#;
    write_transcript(&t, &[assistant(260_000, 5), odd.to_string()]);
    let (_, out, _) = hook(&dir, &args, &ev, &[]);
    assert_eq!(out, "");
    assert_eq!(dir.state_files(), 1);
}

#[test]
fn meter_is_silent_on_bad_input() {
    let dir = Dir::new();
    let args = ["context-meter", "--levels", "1"];
    let missing = event(&dir.0.join("none.jsonl"), "s1", None);
    for input in ["", "not json", "[]", "{}", missing.as_str()] {
        let (code, out, err) = hook(&dir, &args, input, &[]);
        assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""), "{input}");
    }
    let t = dir.0.join("s1.jsonl");
    fs::write(&t, "{\"type\":\"user\"}\n").unwrap();
    let (code, out, _) = hook(&dir, &args, &event(&t, "s1", None), &[]);
    assert_eq!((code, out.as_str()), (0, ""));
    // An id with a path separator.
    let (code, out, _) = hook(&dir, &args, &event(&t, "../x", None), &[]);
    assert_eq!((code, out.as_str()), (0, ""));
    assert_eq!(dir.state_files(), 0);
}

#[test]
fn unknown_flags_and_names_exit_with_zero() {
    let dir = Dir::new();
    let t = dir.0.join("s1.jsonl");
    write_transcript(&t, &[assistant(310_000, 5)]);
    let ev = event(&t, "s1", None);
    let args = [
        "context-meter",
        "--wat",
        "--levels",
        "300000,x",
        "--subagent",
        "many",
        "--cap",
    ];
    let (code, out, err) = hook(&dir, &args, &ev, &[]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert!(out.contains("about 310K"), "{out}");
    for args in [&["nothing"][..], &["fanout-cap", "--cap", "x", "--wat"]] {
        let (code, out, err) = hook(&dir, args, &ev, &[]);
        assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    }
}

fn workflow_event(script: &str) -> String {
    let mut s = String::new();
    // A JSON string, escaped by hand for the characters of the test scripts.
    for c in script.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            c => s.push(c),
        }
    }
    format!(
        r#"{{"hook_event_name":"PreToolUse","tool_name":"Workflow","tool_input":{{"script":"{s}"}}}}"#
    )
}

#[test]
fn fanout_asks_above_the_cap() {
    let dir = Dir::new();
    let ev = workflow_event("[1, 2, 3, 4].map(x => agent(x))");
    let (code, out, err) = hook(&dir, &["fanout-cap", "--cap", "3"], &ev, &[]);
    assert_eq!((code, err.as_str()), (0, ""));
    assert_eq!(
        out,
        "{\"hookSpecificOutput\":{\"hookEventName\":\"PreToolUse\",\"permissionDecision\":\"ask\",\"permissionDecisionReason\":\"spec-dna fan-out cap: this workflow can start 4 agents, more than the cap of 3. Approve it only if you need the extra agents. To change the cap, start Claude Code with SPEC_DNA_FANOUT_CAP=<n>.\"}}\n"
    );
    // At the cap, and with the default cap of 10: silent.
    let (_, out, _) = hook(&dir, &["fanout-cap", "--cap", "4"], &ev, &[]);
    assert_eq!(out, "");
    let (_, out, _) = hook(&dir, &["fanout-cap"], &ev, &[]);
    assert_eq!(out, "");
}

#[test]
fn fanout_environment_cap_wins() {
    let dir = Dir::new();
    let ev = workflow_event("[1, 2, 3, 4].map(x => agent(x))");
    let (_, out, _) = hook(
        &dir,
        &["fanout-cap", "--cap", "3"],
        &ev,
        &[("SPEC_DNA_FANOUT_CAP", "4")],
    );
    assert_eq!(out, "");
    let (_, out, _) = hook(
        &dir,
        &["fanout-cap", "--cap", "9"],
        &ev,
        &[("SPEC_DNA_FANOUT_CAP", "2")],
    );
    assert!(out.contains("more than the cap of 2"), "{out}");
    let (_, out, _) = hook(
        &dir,
        &["fanout-cap", "--cap", "1"],
        &ev,
        &[("SPEC_DNA_FANOUT_CAP", "0")],
    );
    assert_eq!(out, "");
}

#[test]
fn fanout_asks_for_an_unbounded_list() {
    let dir = Dir::new();
    let ev =
        workflow_event("const xs = await load();\nfor (const x of xs) {\n  await agent(x);\n}");
    let (_, out, _) = hook(&dir, &["fanout-cap", "--cap", "5"], &ev, &[]);
    assert!(
        out.contains("line 3 can start agents without a limit: a loop over a list that the run makes, a loop without a number limit, a recursive call or a nested workflow."),
        "{out}"
    );
    assert!(out.contains("for example with .slice(0, 5)"), "{out}");
}

#[test]
fn fanout_reads_a_named_script_of_the_project() {
    let dir = Dir::new();
    let wf = dir.0.join("proj/.claude/workflows");
    fs::create_dir_all(&wf).unwrap();
    fs::write(wf.join("wide.js"), "items.map(x => agent(x))").unwrap();
    let ev = r#"{"tool_name":"Workflow","tool_input":{"name":"wide"}}"#;
    let project = dir.0.join("proj");
    let (code, out, _) = hook(
        &dir,
        &["fanout-cap"],
        ev,
        &[("CLAUDE_PROJECT_DIR", project.to_str().unwrap())],
    );
    assert_eq!(code, 0);
    assert!(out.contains("\"permissionDecision\":\"ask\""), "{out}");
    // No project and no cwd in the input: the script is not found.
    let (_, out, _) = hook(&dir, &["fanout-cap"], ev, &[]);
    assert_eq!(out, "");
}

#[test]
fn fanout_is_silent_on_bad_input() {
    let dir = Dir::new();
    for input in [
        "",
        "not json",
        "{}",
        r#"{"tool_input":{"script":"agent(1"}}"#,
        r#"{"tool_input":{"scriptPath":"absent.js"}}"#,
        r#"{"tool_input":{"name":"../../x"}}"#,
    ] {
        let (code, out, err) = hook(&dir, &["fanout-cap", "--cap", "1"], input, &[]);
        assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""), "{input}");
    }
}

// ---- subagent-card ----

/// The card of every role but `acceptor`. The test holds its own copy, so a
/// change of one byte fails.
const CARD: &str = "Subagent card. Your agent file and your brief have priority over this card.
1. Do only the task of the brief. Change only the files of `Files`. If the task needs more, stop and report it.
2. Read the brief, then the files that it names. Run `spec-dna context <file>` before an edit. Search only for a fact that these do not give.
3. Do not commit, push or change the git state.
4. Report the result, the changed files, each command that you ran with its result, and the open issues.
5. For a rule that the project files do not state, add `Lessons:` before the last line, with one line for each: `- Target: <file>. Text: <sentence>. Evidence: <what happened>.`
6. End the report with one line: `State: COMPLETE`, `State: COMPLETE_WITH_CONCERNS`, `State: NEEDS_CONTEXT` or `State: BLOCKED`.";

const ACCEPTOR_LINE: &str = "3. Commit only `acceptance.toml`, and do not push.";

fn start_event(agent_type: &str) -> String {
    format!(
        r#"{{"session_id":"s1","hook_event_name":"SubagentStart","agent_id":"a1","agent_type":"{agent_type}"}}"#
    )
}

/// The output of the card: the card holds no quote and no backslash, so only
/// its line ends have an escape.
fn card_output(card: &str) -> String {
    format!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"SubagentStart\",\"additionalContext\":\"{}\"}}}}\n",
        card.replace('\n', "\\n")
    )
}

#[test]
fn subagent_card_gives_the_card_to_each_named_subagent() {
    let dir = Dir::new();
    assert_eq!(CARD.len(), 788);
    for agent_type in ["general-purpose", "verifier", "Explore", "my-own-agent"] {
        let (code, out, err) = hook(&dir, &["subagent-card"], &start_event(agent_type), &[]);
        assert_eq!((code, err.as_str()), (0, ""), "{agent_type}");
        assert_eq!(out, card_output(CARD), "{agent_type}");
    }
}

#[test]
fn subagent_card_gives_the_acceptor_its_own_third_line() {
    let dir = Dir::new();
    let card = CARD.replace(
        "3. Do not commit, push or change the git state.",
        ACCEPTOR_LINE,
    );
    assert_eq!(card.len(), 791);
    let (code, out, _) = hook(&dir, &["subagent-card"], &start_event("acceptor"), &[]);
    assert_eq!((code, out), (0, card_output(&card)));
}

#[test]
fn subagent_card_is_silent_with_no_agent_type_and_on_bad_input() {
    let dir = Dir::new();
    let good = start_event("general-purpose");
    // The positive control.
    assert_ne!(hook(&dir, &["subagent-card"], &good, &[]).1, "");
    for input in [
        "",
        "not json",
        "{}",
        &start_event(""),
        r#"{"hook_event_name":"SubagentStart","agent_id":"a1"}"#,
        r#"{"hook_event_name":"SubagentStart","agent_type":7}"#,
        &good.replace("SubagentStart", "SubagentStop"),
        r#"{"agent_type":"general-purpose"}"#,
    ] {
        let (code, out, err) = hook(&dir, &["subagent-card"], input, &[]);
        assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""), "{input}");
    }
}

#[test]
fn hook_help_prints_the_help_of_the_command() {
    let dir = Dir::new();
    let out = Command::new(BIN).args(["help", "hook"]).output().unwrap();
    let help = String::from_utf8(out.stdout).unwrap();
    assert!(
        help.starts_with("usage: spec-dna hook context-meter"),
        "{help}"
    );
    for args in [&["--help"][..], &["-h"][..], &[][..]] {
        let (code, out, _) = hook(&dir, args, "", &[]);
        assert_eq!(code, 0, "{args:?}");
        assert_eq!(out, help, "{args:?}");
    }
    // A hook name keeps its own flags, and it is silent.
    let (code, out, _) = hook(&dir, &["context-meter", "--help"], "", &[]);
    assert_eq!((code, out.as_str()), (0, ""));
}
