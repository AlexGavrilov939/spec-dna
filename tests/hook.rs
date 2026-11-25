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

// ---- return-state ----

const REASON: &str = "spec-dna return state: end your report with one line: `State: COMPLETE`, `State: COMPLETE_WITH_CONCERNS`, `State: NEEDS_CONTEXT` or `State: BLOCKED`.";

fn block_output() -> String {
    format!("{{\"decision\":\"block\",\"reason\":\"{REASON}\"}}\n")
}

/// A JSON string of `text`. The test texts hold no other control character.
fn quoted(text: &str) -> String {
    let t = text
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{t}\"")
}

/// A SubagentStop input of a named subagent that never got the reason.
fn stop_event(report: &str) -> String {
    format!(
        r#"{{"session_id":"s1","hook_event_name":"SubagentStop","agent_id":"a1","agent_type":"general-purpose","stop_hook_active":false,"last_assistant_message":{}}}"#,
        quoted(report)
    )
}

fn return_state(dir: &Dir, input: &str) -> String {
    let (code, out, err) = hook(dir, &["return-state"], input, &[]);
    assert_eq!((code, err.as_str()), (0, ""), "{input}");
    out
}

#[test]
fn return_state_passes_each_of_the_4_values() {
    let dir = Dir::new();
    for value in [
        "COMPLETE",
        "COMPLETE_WITH_CONCERNS",
        "NEEDS_CONTEXT",
        "BLOCKED",
    ] {
        for report in [
            format!("The work.\n\nState: {value}"),
            // Empty lines at the end, and white space around the line.
            format!("The work.\n  State: {value}  \n\n  \n"),
            format!("The work.\r\nState: {value}\r\n"),
            format!("The work.\n**State: {value}**"),
            format!("The work.\n`State: {value}`"),
            format!("The work.\n **`State: {value}`** \n"),
        ] {
            assert_eq!(return_state(&dir, &stop_event(&report)), "", "{report:?}");
        }
    }
}

#[test]
fn return_state_asks_once_for_a_report_with_no_state_line() {
    let dir = Dir::new();
    assert_eq!(REASON.len(), 149);
    for report in [
        "The work is done.",
        "",
        "State: COMPLETE\nOne more sentence.",
        "State: DONE",
        "state: COMPLETE",
        "State: complete",
        "State:  COMPLETE",
        "State:COMPLETE",
        "State: COMPLETE.",
        "**State:** COMPLETE",
        "The report ends with State: COMPLETE",
    ] {
        let input = stop_event(report);
        assert_eq!(return_state(&dir, &input), block_output(), "{report:?}");
        // After one block, the same input gives nothing.
        let again = input.replace("\"stop_hook_active\":false", "\"stop_hook_active\":true");
        assert_ne!(again, input);
        assert_eq!(return_state(&dir, &again), "", "{report:?}");
    }
}

#[test]
fn return_state_is_silent_on_input_that_it_does_not_know() {
    let dir = Dir::new();
    let good = stop_event("The work.");
    // The positive control.
    assert_eq!(return_state(&dir, &good), block_output());
    for input in [
        "".to_string(),
        "not json".to_string(),
        "{}".to_string(),
        good.replace("\"agent_type\":\"general-purpose\"", "\"agent_type\":\"\""),
        good.replace("\"agent_type\":\"general-purpose\",", ""),
        good.replace("SubagentStop", "Stop"),
        // A key that is absent or not a boolean never blocks.
        good.replace("\"stop_hook_active\":false,", ""),
        good.replace(
            "\"stop_hook_active\":false",
            "\"stop_hook_active\":\"false\"",
        ),
        good.replace(",\"last_assistant_message\":\"The work.\"", ""),
        good.replace("\"The work.\"", "null"),
    ] {
        assert_ne!(input, good);
        assert_eq!(return_state(&dir, &input), "", "{input}");
    }
}

/// An assistant line of a transcript with one tool call.
fn tool_call(name: &str, input: &str) -> String {
    format!(
        r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"text","text":"x"}},{{"type":"tool_use","id":"t1","name":"{name}","input":{input}}}]}}}}"#
    )
}

fn handback(report: &str) -> String {
    tool_call(
        "SubagentHandback",
        &format!(r#"{{"message":{}}}"#, quoted(report)),
    )
}

fn with_transcript(event: &str, transcript: &Path) -> String {
    let path = quoted(transcript.to_str().unwrap());
    format!(
        "{},\"agent_transcript_path\":{path}}}",
        event.strip_suffix('}').unwrap()
    )
}

#[test]
fn return_state_reads_the_message_of_the_last_handback() {
    let dir = Dir::new();
    let t = dir.0.join("agent-a1.jsonl");
    // The report is the `message` of the call, and the final text is not.
    write_transcript(
        &t,
        &[handback("The work.\nState: BLOCKED"), assistant(10, 5)],
    );
    let input = with_transcript(&stop_event("I handed the report back."), &t);
    assert_eq!(return_state(&dir, &input), "");
    write_transcript(&t, &[handback("The work.")]);
    let input = with_transcript(&stop_event("State: COMPLETE"), &t);
    assert_eq!(return_state(&dir, &input), block_output());
    // The last call wins.
    write_transcript(
        &t,
        &[handback("State: COMPLETE"), handback("The second report.")],
    );
    let input = with_transcript(&stop_event("State: COMPLETE"), &t);
    assert_eq!(return_state(&dir, &input), block_output());
    // No `last_assistant_message`: the call alone is a report.
    write_transcript(&t, &[handback("State: COMPLETE")]);
    let input =
        with_transcript(&stop_event(""), &t).replace(",\"last_assistant_message\":\"\"", "");
    assert_eq!(return_state(&dir, &input), "");
    // A far call: the hook reads from the end of the file, past one chunk.
    write_transcript(&t, &[handback("State: COMPLETE"), assistant(10, 600_000)]);
    let input = with_transcript(&stop_event("x"), &t);
    assert_eq!(return_state(&dir, &input), "");
}

#[test]
fn return_state_takes_the_final_text_when_no_call_is_a_handback() {
    let dir = Dir::new();
    let t = dir.0.join("agent-a1.jsonl");
    // The name in a text or in another tool call is no handback.
    write_transcript(
        &t,
        &[
            tool_call("Bash", r#"{"command":"echo SubagentHandback","message":"State: COMPLETE"}"#),
            r#"{"type":"user","message":{"content":[{"type":"tool_use","name":"SubagentHandback","input":{"message":"State: COMPLETE"}}]}}"#.to_string(),
        ],
    );
    let input = with_transcript(&stop_event("The work."), &t);
    assert_eq!(return_state(&dir, &input), block_output());
    let input = with_transcript(&stop_event("State: COMPLETE"), &t);
    assert_eq!(return_state(&dir, &input), "");
    // A transcript that does not exist.
    let input = with_transcript(&stop_event("State: COMPLETE"), &dir.0.join("none.jsonl"));
    assert_eq!(return_state(&dir, &input), "");
}

// ---- fix-limit ----

const COMMAND: &str = "pytest tests/test_valve.py --maxfail=1";

fn limit_message(n: u32) -> String {
    format!(
        "spec-dna fix limit: this command failed after {n} fixes. Stop the fixes of this problem. Tell the person the command, the cause that each fix assumed, the error now, and the next step that you propose. A subagent ends its report with `State: BLOCKED`."
    )
}

fn limit_output(n: u32) -> String {
    format!(
        "{{\"hookSpecificOutput\":{{\"hookEventName\":\"PostToolUseFailure\",\"additionalContext\":\"{}\"}}}}\n",
        limit_message(n)
    )
}

/// A tool event of the session `s1`. `agent`: the `agent_id` of a subagent.
fn tool_event(event: &str, tool: &str, input: &str, agent: Option<&str>) -> String {
    let agent = agent.map_or(String::new(), |a| format!(r#""agent_id":"{a}","#));
    format!(
        r#"{{"session_id":"s1",{agent}"hook_event_name":"{event}","tool_name":"{tool}","tool_input":{input}}}"#
    )
}

fn bash_input(command: &str) -> String {
    format!(r#"{{"command":{}}}"#, quoted(command))
}

/// One project directory and one state folder, with the calls of one hook.
struct Fix {
    dir: Dir,
    args: Vec<String>,
}

impl Fix {
    fn new() -> Fix {
        let dir = Dir::new();
        fs::create_dir_all(dir.0.join("docs/intent/dose")).unwrap();
        fs::create_dir_all(dir.0.join("docs/intent-notes")).unwrap();
        fs::create_dir_all(dir.0.join("src")).unwrap();
        let args = ["fix-limit", "--limit", "3", "--intent", "docs/intent"]
            .map(str::to_string)
            .to_vec();
        Fix { dir, args }
    }

    fn run(&self, event: &str) -> String {
        let args: Vec<&str> = self.args.iter().map(String::as_str).collect();
        let root = self.dir.0.to_str().unwrap();
        let (code, out, err) = hook(&self.dir, &args, event, &[("CLAUDE_PROJECT_DIR", root)]);
        assert_eq!((code, err.as_str()), (0, ""), "{event}");
        out
    }

    /// A failed `Bash` call.
    fn fail_as(&self, command: &str, agent: Option<&str>) -> String {
        let input = bash_input(command);
        self.run(&tool_event("PostToolUseFailure", "Bash", &input, agent))
    }

    fn fail(&self) -> String {
        self.fail_as(COMMAND, None)
    }

    /// A `Bash` call that passed.
    fn pass(&self, command: &str) {
        let input = bash_input(command);
        let out = self.run(&tool_event("PostToolUse", "Bash", &input, None));
        assert_eq!(out, "");
    }

    /// An edit of the file at `path`, from the root of the project.
    fn edit_as(&self, tool: &str, path: &str, agent: Option<&str>) {
        let file = self.dir.0.join(path);
        let input = format!(r#"{{"file_path":{}}}"#, quoted(file.to_str().unwrap()));
        let out = self.run(&tool_event("PostToolUse", tool, &input, agent));
        assert_eq!(out, "");
    }

    fn edit(&self) {
        self.edit_as("Edit", "src/valve.py", None);
    }

    /// The names and the texts of the state files.
    fn state(&self) -> Vec<(String, String)> {
        let Ok(entries) = fs::read_dir(self.dir.0.join("tmp/spec-dna-fix")) else {
            return Vec::new();
        };
        let mut out: Vec<(String, String)> = entries
            .map(|e| {
                let e = e.unwrap();
                let name = e.file_name().into_string().unwrap();
                (name, fs::read_to_string(e.path()).unwrap())
            })
            .collect();
        out.sort();
        out
    }
}

#[test]
fn fix_limit_speaks_once_at_the_third_failed_fix() {
    let fix = Fix::new();
    assert_eq!(limit_message(3).len(), 249);
    // The first failure is the problem, and not a fix.
    assert_eq!(fix.fail(), "");
    for _ in 0..2 {
        fix.edit();
        assert_eq!(fix.fail(), "");
    }
    fix.edit();
    assert_eq!(fix.fail(), limit_output(3));
    for _ in 0..4 {
        fix.edit();
        assert_eq!(fix.fail(), "", "the message comes once for each key");
    }
}

#[test]
fn fix_limit_counts_no_run_without_an_edit() {
    let fix = Fix::new();
    // An edit before the first failure is no fix.
    fix.edit();
    for _ in 0..8 {
        assert_eq!(fix.fail(), "");
    }
    // One edit counts for one failure, also of another command.
    let fix = Fix::new();
    assert_eq!(fix.fail_as("make a", None), "");
    assert_eq!(fix.fail_as("make b", None), "");
    fix.edit();
    assert_eq!(fix.fail_as("make a", None), "");
    // The failure of `make a` removed the mark.
    for _ in 0..4 {
        assert_eq!(fix.fail_as("make b", None), "");
    }
    fix.edit();
    assert_eq!(fix.fail_as("make a", None), "");
    fix.edit();
    assert_eq!(fix.fail_as("make a", None), limit_output(3));
    // `make b` has its own count, which no edit raised.
    fix.edit();
    assert_eq!(fix.fail_as("make b", None), "");
}

#[test]
fn fix_limit_deletes_the_count_when_the_command_passes() {
    let fix = Fix::new();
    assert_eq!(fix.fail(), "");
    for _ in 0..2 {
        fix.edit();
        assert_eq!(fix.fail(), "");
    }
    // The pass of another command deletes nothing.
    fix.pass("ls");
    fix.pass(COMMAND);
    // A new problem: 3 more fixes before the message.
    assert_eq!(fix.fail(), "");
    for _ in 0..2 {
        fix.edit();
        assert_eq!(fix.fail(), "");
    }
    fix.edit();
    assert_eq!(fix.fail(), limit_output(3));
}

#[test]
fn fix_limit_takes_runs_of_white_space_as_one_space() {
    let fix = Fix::new();
    assert_eq!(fix.fail_as("go test ./pump", None), "");
    for command in ["go  test ./pump", "go\ttest\n./pump"] {
        fix.edit();
        assert_eq!(fix.fail_as(command, None), "");
    }
    fix.edit();
    assert_eq!(fix.fail_as("go test   ./pump", None), limit_output(3));
    // A changed command starts a new count.
    fix.edit();
    assert_eq!(fix.fail_as("go test ./pump -v", None), "");
}

#[test]
fn fix_limit_keeps_a_count_for_each_agent() {
    let fix = Fix::new();
    let (a, b) = (Some("a1"), Some("b2"));
    for agent in [a, b, None] {
        assert_eq!(fix.fail_as(COMMAND, agent), "");
    }
    // The edits of one subagent raise only its own count.
    for _ in 0..2 {
        fix.edit_as("Write", "src/valve.py", a);
        assert_eq!(fix.fail_as(COMMAND, b), "");
        assert_eq!(fix.fail_as(COMMAND, None), "");
        assert_eq!(fix.fail_as(COMMAND, a), "");
    }
    fix.edit_as("Edit", "src/valve.py", a);
    assert_eq!(fix.fail_as(COMMAND, a), limit_output(3));
    fix.edit();
    assert_eq!(fix.fail_as(COMMAND, None), "");
}

#[test]
fn fix_limit_marks_no_edit_below_an_intent_folder() {
    let fix = Fix::new();
    assert_eq!(fix.fail(), "");
    for path in [
        "docs/intent/dose/plan.md",
        "docs/intent/dose/acceptance.toml",
        "docs/intent/new.md",
    ] {
        for _ in 0..4 {
            fix.edit_as("Edit", path, None);
            assert_eq!(fix.fail(), "", "{path}");
        }
    }
    // A folder with a longer name is not below the folder: 3 edits count.
    for _ in 0..2 {
        fix.edit_as("Write", "docs/intent-notes/n.md", None);
        assert_eq!(fix.fail(), "");
    }
    fix.edit_as("Write", "docs/intent-notes/n.md", None);
    assert_eq!(fix.fail(), limit_output(3));
}

#[test]
fn fix_limit_takes_the_root_of_the_cwd_of_the_input() {
    let fix = Fix::new();
    assert_eq!(fix.fail(), "");
    let root = fix.dir.0.to_str().unwrap();
    let args: Vec<&str> = fix.args.iter().map(String::as_str).collect();
    // No `CLAUDE_PROJECT_DIR`: the `cwd` of the input is the root.
    let edit = |path: &str| {
        let file = fix.dir.0.join(path);
        let event = format!(
            r#"{{"session_id":"s1","cwd":{},"hook_event_name":"PostToolUse","tool_name":"Edit","tool_input":{{"file_path":{}}}}}"#,
            quoted(root),
            quoted(file.to_str().unwrap())
        );
        assert_eq!(hook(&fix.dir, &args, &event, &[]).1, "");
    };
    for _ in 0..4 {
        edit("docs/intent/dose/plan.md");
        assert_eq!(fix.fail(), "");
    }
    // The positive control.
    for _ in 0..2 {
        edit("src/valve.py");
        assert_eq!(fix.fail(), "");
    }
    edit("src/valve.py");
    assert_eq!(fix.fail(), limit_output(3));
}

#[test]
fn fix_limit_state_holds_no_byte_of_the_command() {
    let fix = Fix::new();
    let secret = "deploy --token=hunter2 ./zebra";
    assert_eq!(fix.fail_as(secret, None), "");
    fix.edit_as("Edit", "src/valve.py", Some("a1"));
    fix.edit();
    assert_eq!(fix.fail_as(secret, Some("a1")), "");
    let state = fix.state();
    // 2 counts and no mark of the agent `a1`, which its failure removed, and
    // the mark of the main agent.
    assert_eq!(state.len(), 3, "{state:?}");
    for (name, text) in &state {
        for word in ["deploy", "token", "hunter2", "zebra", "./"] {
            assert!(
                !name.contains(word) && !text.contains(word),
                "{name}: {text}"
            );
        }
        assert!(text.bytes().all(|b| b.is_ascii_digit()), "{name}: {text:?}");
        let parts: Vec<&str> = name.split('.').collect();
        assert_eq!((parts.len(), parts[0]), (3, "s1"), "{name}");
        assert!(["main", "a1"].contains(&parts[1]), "{name}");
        assert!(
            parts[2] == "edit"
                || (parts[2].len() == 16 && parts[2].bytes().all(|b| b.is_ascii_hexdigit())),
            "{name}"
        );
    }
    // The session id salts the hash: the same command has another name in
    // another session.
    let hash_of = |session: &str| {
        let fix = Fix::new();
        let event = tool_event("PostToolUseFailure", "Bash", &bash_input(secret), None)
            .replace("\"s1\"", &format!("\"{session}\""));
        assert_eq!(fix.run(&event), "");
        fix.state()[0].0.rsplit('.').next().unwrap().to_string()
    };
    assert_ne!(hash_of("s1"), hash_of("s2"));
}

#[test]
fn fix_limit_removes_a_state_file_that_is_older_than_3_days() {
    let fix = Fix::new();
    let state = fix.dir.0.join("tmp/spec-dna-fix");
    fs::create_dir_all(&state).unwrap();
    let old = state.join("s0.main.0123456789abcdef");
    fs::write(&old, "2").unwrap();
    let past = std::time::SystemTime::now() - std::time::Duration::from_secs(4 * 24 * 3600);
    fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(past)
        .unwrap();
    assert_eq!(fix.fail(), "");
    assert!(!old.exists());
    assert_eq!(fix.state().len(), 1);
}

/// A file outside the state folder that no write of the hook may change.
#[cfg(unix)]
fn victim(fix: &Fix, name: &str) -> PathBuf {
    let path = fix.dir.0.join(name);
    fs::write(&path, "keep this data").unwrap();
    path
}

#[cfg(unix)]
#[test]
fn fix_limit_writes_no_mark_through_a_symlink() {
    let fix = Fix::new();
    let target = victim(&fix, "victim");
    let state = fix.dir.0.join("tmp/spec-dna-fix");
    fs::create_dir_all(&state).unwrap();
    std::os::unix::fs::symlink(&target, state.join("s1.main.edit")).unwrap();
    fix.edit();
    assert_eq!(fs::read_to_string(&target).unwrap(), "keep this data");
}

#[cfg(unix)]
#[test]
fn fix_limit_writes_no_count_through_a_symlink() {
    let fix = Fix::new();
    let target = victim(&fix, "victim");
    assert_eq!(fix.fail(), "");
    let name = fix.state()[0].0.clone();
    let count = fix.dir.0.join("tmp/spec-dna-fix").join(name);
    fs::remove_file(&count).unwrap();
    std::os::unix::fs::symlink(&target, &count).unwrap();
    fix.edit();
    assert_eq!(fix.fail(), "");
    assert_eq!(fs::read_to_string(&target).unwrap(), "keep this data");
    // A regular file with the new count replaced the link.
    assert!(fs::symlink_metadata(&count).unwrap().is_file());
}

#[cfg(unix)]
#[test]
fn fix_limit_uses_no_state_folder_that_is_a_symlink() {
    // The name of the count of `COMMAND` in the session `s1`.
    let first = Fix::new();
    assert_eq!(first.fail(), "");
    let count = first.state()[0].0.clone();

    let fix = Fix::new();
    let other = fix.dir.0.join("other");
    fs::create_dir_all(&other).unwrap();
    for name in ["s1.main.edit", count.as_str(), "old"] {
        fs::write(other.join(name), "2").unwrap();
    }
    let past = std::time::SystemTime::now() - std::time::Duration::from_secs(4 * 24 * 3600);
    fs::File::options()
        .write(true)
        .open(other.join("old"))
        .unwrap()
        .set_modified(past)
        .unwrap();
    fs::create_dir_all(fix.dir.0.join("tmp")).unwrap();
    std::os::unix::fs::symlink(&other, fix.dir.0.join("tmp/spec-dna-fix")).unwrap();
    let before = fix.state();
    assert_eq!(before.len(), 3);
    // A pass, a failure and an edit: no one deletes or writes a file there.
    fix.pass(COMMAND);
    assert_eq!(fix.state(), before, "pass");
    assert_eq!(fix.fail(), "");
    assert_eq!(fix.state(), before, "failure");
    fix.edit();
    assert_eq!(fix.state(), before, "edit");
}

#[test]
fn fix_limit_writes_no_count_in_place() {
    // A second name of the count file keeps the old text, so a reader never
    // sees a count file that is empty or half written.
    let fix = Fix::new();
    assert_eq!(fix.fail(), "");
    let name = fix.state()[0].0.clone();
    let count = fix.dir.0.join("tmp/spec-dna-fix").join(name);
    let witness = fix.dir.0.join("witness");
    fs::hard_link(&count, &witness).unwrap();
    fix.edit();
    assert_eq!(fix.fail(), "");
    assert_eq!(fs::read_to_string(&count).unwrap(), "1");
    assert_eq!(fs::read_to_string(&witness).unwrap(), "0");
    // No temporary file stays.
    assert_eq!(fix.state().len(), 1);
}

#[test]
fn fix_limit_is_off_at_0_and_silent_on_input_that_it_does_not_know() {
    for args in [
        &["fix-limit", "--limit", "0"][..],
        &["fix-limit"][..],
        &["fix-limit", "--limit", "x"][..],
    ] {
        let mut fix = Fix::new();
        fix.args = args.iter().map(|a| a.to_string()).collect();
        for _ in 0..5 {
            assert_eq!(fix.fail(), "", "{args:?}");
            fix.edit();
        }
        assert!(fix.state().is_empty(), "{args:?}");
    }
    let fix = Fix::new();
    let good = tool_event("PostToolUseFailure", "Bash", &bash_input(COMMAND), None);
    for input in [
        "".to_string(),
        "not json".to_string(),
        "{}".to_string(),
        good.replace("\"s1\"", "\"../s1\""),
        good.replace("\"session_id\":\"s1\",", ""),
        good.replace("\"s1\",", "\"s1\",\"agent_id\":\"../a\","),
        good.replace("\"s1\",", "\"s1\",\"agent_id\":7,"),
        good.replace("\"Bash\"", "\"Read\""),
        good.replace("\"Bash\"", "\"Edit\""),
        good.replace("PostToolUseFailure", "PreToolUse"),
        good.replace("\"command\"", "\"cmd\""),
        // An edit with no path.
        tool_event("PostToolUse", "Edit", "{}", None),
        tool_event(
            "PostToolUse",
            "NotebookEdit",
            r#"{"file_path":"/x/y"}"#,
            None,
        ),
    ] {
        assert_eq!(fix.run(&input), "", "{input}");
        assert!(fix.state().is_empty(), "{input}");
    }
    // The positive control.
    assert_eq!(fix.run(&good), "");
    assert_eq!(fix.state().len(), 1);
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
