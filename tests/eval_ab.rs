//! Tests of `spec-dna eval run --ab` and `spec-dna eval verdict` (ADR 0016,
//! 0.7.0 slice 5). A stub `claude` replaces the real program, so no test calls
//! the network or spends money. Each test uses a temporary repository and a
//! temporary `TMPDIR`. No test reads or writes a file of this repository.
//!
//! The stub appends one record to `$STUB_DIR/log` for each call: the
//! arguments, the files of `.claude/` in its folder and the text of the
//! instruction files. The file `$STUB_DIR/modes` has one line for each call
//! (`ok` if the line is absent): the way that the call ends. `STUB_PASS`
//! selects the arm that gets the edit that the check wants.
#![cfg(unix)]

mod common;

use common::{Repo, VERSION, path_with, stub_bin};
use std::fs;
use std::path::PathBuf;

const STUB: &str = r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "2.1.325 (Claude Code)"; exit 0; fi
n=$(cat "$STUB_DIR/n" 2>/dev/null || echo 0)
n=$((n+1))
echo "$n" > "$STUB_DIR/n"
{
    echo "CALL $n"
    echo "WHICH $(command -v spec-dna)"
    for a in "$@"; do echo "ARG $a"; done
    find .claude -type f 2>/dev/null | sort | sed 's/^/FILE /'
    [ -f eval-id.txt ] && echo "EVAL $(cat eval-id.txt)"
    for f in AGENTS.md CLAUDE.md .claude/skills/spec-dna/SKILL.md .claude/skills/spec-dna-review/SKILL.md .claude/agents/acceptor.md .claude/settings.json; do
        [ -f "$f" ] && sed "s|^|TEXT $f: |" "$f"
    done
    [ -f spec-dna.toml ] && sed 's/^/TOML /' spec-dna.toml
    echo '---'
} >> "$STUB_DIR/log"
mode=$(sed -n "${n}p" "$STUB_DIR/modes" 2>/dev/null)
[ -n "$mode" ] || mode=ok
account() {
    printf '{"type":"result","subtype":"success","is_error":true,"result":"%s","total_cost_usd":%s}\n' "$1" "${STUB_REFUSED_COST:-0.5}"
    exit 1
}
refuse() {
    printf '{"type":"result","subtype":"success","is_error":true,"api_error_status":%s,"result":"%s","total_cost_usd":%s}\n' "$1" "$2" "${STUB_REFUSED_COST:-0.5}"
    exit 1
}
case "$mode" in
    401) refuse 401 "Failed to authenticate. API Error: 401 Invalid authentication credentials" ;;
    403) refuse 403 "Failed to authenticate. API Error: 403 Attention Required! | Cloudflare" ;;
    429) refuse 429 "API Error: 429 Rate limit reached for requests" ;;
    503) refuse 503 "API Error: 503 Service Unavailable" ;;
    cf) printf '<title>Attention Required! | Cloudflare</title>\n'; exit 1 ;;
    turns)
        printf '{"type":"result","subtype":"error_max_turns","is_error":true,"result":"Reached max turns","total_cost_usd":0.1}\n'
        exit 1 ;;
    softlimit)
        printf '{"type":"result","subtype":"success","is_error":false,"result":"You hit the rate limit of the plan","total_cost_usd":0.1}\n'
        exit 0 ;;
    long429)
        pad=$(printf 'x%.0s' $(seq 1 700))
        printf '{"type":"result","subtype":"success","is_error":true,"result":"API Error: 429 %s","total_cost_usd":0.1}\n' "$pad"
        exit 1 ;;
    acct) account "You've hit your limit · resets 5pm (Europe/London)" ;;
    usage) account "Claude AI usage limit reached|1700000000" ;;
    timeout) account "Request timed out" ;;
    overloaded) account "Overloaded" ;;
    s401x) refuse 401 "Something went wrong" ;;
    s500) refuse 500 "Something went wrong" ;;
    s529) refuse 529 "x" ;;
    s503long)
        pad=$(printf 'x%.0s' $(seq 1 900))
        refuse 503 "$pad" ;;
    netpage) printf 'Error: read ECONNRESET\n'; exit 1 ;;
    maxbudget)
        printf '{"type":"result","subtype":"error_max_budget_usd","is_error":true,"result":"Reached the maximum budget","total_cost_usd":0.1}\n'
        exit 1 ;;
    turnsnotext)
        printf '{"type":"result","subtype":"error_max_turns","is_error":true,"total_cost_usd":0.429}\n'
        exit 1 ;;
    s400)
        printf '{"type":"result","subtype":"success","is_error":true,"api_error_status":400,"result":"Bad request","total_cost_usd":0.1}\n'
        exit 1 ;;
    s429noerr)
        printf '{"type":"result","subtype":"success","is_error":false,"api_error_status":429,"result":"done","total_cost_usd":0.1}\n'
        exit 0 ;;
    emptyexit) exit 1 ;;
    login) account "Not logged in · Please run /login" ;;
    credit) account "Credit balance is too low" ;;
    outusage) account "You're out of usage credits" ;;
    outextra) account "You're out of extra usage" ;;
    unable) account "API Error: Unable to connect to API" ;;
    connerr) account "Connection error." ;;
    stderrnet) echo "Error: read ECONNRESET" >&2; exit 1 ;;
    ctxlimit)
        printf '{"type":"result","subtype":"success","is_error":true,"result":"Context limit reached","total_cost_usd":0.1}\n'
        exit 1 ;;
    odd)
        printf '{"type":"result","subtype":"success","is_error":true,"result":"Something odd","total_cost_usd":0}\n'
        exit 1 ;;
    hang) sleep 30 ;;
    limit5) account "5-hour limit reached ∙ resets 3pm" ;;
    limitweek) account "Weekly limit reached ∙ resets Mon 9am" ;;
    limitplain) printf 'Weekly limit reached ∙ resets Mon 9am\n'; exit 1 ;;
    toklimit)
        printf '{"type":"result","subtype":"success","is_error":true,"result":"Token limit reached for this file","total_cost_usd":0.1}\n'
        exit 1 ;;
    plainbreak) printf 'Error: something broke\n'; exit 1 ;;
    bg)
        sleep 40 &
        ;;
    sighup) kill -HUP $PPID; sleep 30 ;;
    sigint) kill -INT $PPID; sleep 30 ;;
    sigterm) kill -TERM $PPID; sleep 30 ;;
    sigkill) kill -KILL $PPID; sleep 30 ;;
    snap) cp "$STUB_RESULTS"/skills-*.json "$STUB_DIR/snap-$n.json" 2>/dev/null ;;
    denied)
        echo 'status: done' > notes.txt
        printf '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.1,"permission_denials":[{"tool_name":"Skill"},{"tool_name":"Bash"}],"result":"ok"}\n'
        exit 0 ;;
    tokens)
        echo 'status: done' > notes.txt
        printf '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.1,"modelUsage":{"claude-b":{"inputTokens":10,"outputTokens":20,"cacheReadInputTokens":30,"cacheCreationInputTokens":40,"thinkingTokens":99},"claude-a":{"inputTokens":1,"outputTokens":2,"cacheReadInputTokens":3}},"permission_denials":[],"result":"ok"}\n'
        exit 0 ;;
    nousage)
        echo 'status: done' > notes.txt
        printf '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.1,"permission_denials":[],"result":"ok"}\n'
        exit 0 ;;
    badusage)
        echo 'status: done' > notes.txt
        printf '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.1,"modelUsage":{"claude-a":{"inputTokens":-1,"outputTokens":2}},"permission_denials":[],"result":"ok"}\n'
        exit 0 ;;
    plainfail) printf 'Segmentation fault\n'; exit 139 ;;
    break)
        rm -f "$STUB_BREAK_FILE"
        ;;
    check)
        spec-dna check --all > "$STUB_DIR/check-$n" 2>&1
        echo 'status: done' > notes.txt
        ;;
    commit)
        echo 'status: done' > notes.txt
        git add -A
        git commit -qm agent && git log -1 --format='%an <%ae> %cn <%ce>' > "$STUB_DIR/who-$n"
        ;;
esac
pass=0
case "$STUB_PASS" in
    skill) [ -f ".claude/skills/spec-dna-$STUB_SKILL/SKILL.md" ] && pass=1 ;;
    noskill) [ -f ".claude/skills/spec-dna-$STUB_SKILL/SKILL.md" ] || pass=1 ;;
    line) grep -q 'spec-dna context' AGENTS.md 2>/dev/null && pass=1 ;;
    none) ;;
    *) pass=1 ;;
esac
[ "$pass" = 1 ] && echo 'status: done' > notes.txt
printf '{"type":"result","subtype":"success","is_error":false,"total_cost_usd":%s,"num_turns":4,"modelUsage":{"claude-test-1":{}},"permission_denials":[],"result":"ok"}\n' "${STUB_COST:-0.1}"
"#;

const CHECK_SH: &str = "file_has() {\n    grep -q -F -- \"$2\" \"$1\"\n}\n";

/// The groups of the evals of the pair `agents-line`: `tdd-1`, `debug-1` and
/// `execute-10`, and the 2 evals that name the group.
const DISCIPLINED: [&str; 4] = ["tdd", "debug", "execute", "agents-line"];

fn pack_eval(id: &str, skill: &str, runs: u32) -> String {
    format!(
        "{{\"id\":\"{id}\",\"skill\":\"{skill}\",\"task\":\"Change draft to done.\",\"fixture\":\"evals/pack/fixtures/{id}\",\"allowed-tools\":[\"Read\",\"Edit\"],\"max-turns\":5,\"max-budget-usd\":1,\"runs\":{runs},\"checks\":[{{\"name\":\"notes-done\",\"run\":\"file_has notes.txt 'status: done'\"}}]}}\n"
    )
}

struct Fx {
    repo: Repo,
    /// Holds the stub, its logs and the temporary folder of the runner.
    stubs: Repo,
}

impl Fx {
    /// A project with the skills `on` written by `init`, and a pack of 5
    /// evals for each skill of `packed`, with `runs` runs.
    fn new(on: &[&str], packed: &[&str], runs: u32) -> Fx {
        let list: Vec<String> = on.iter().map(|s| format!("\"{s}\"")).collect();
        let repo = Repo::with_config(&format!(
            "\n[evals]\n\n[[roots]]\npath = \"docs\"\n\n[harness]\nlevel = \"lean\"\nskills = [{}]\n",
            list.join(", ")
        ));
        repo.write("docs/a.md", "# A\n\nText.\n");
        repo.commit("start");
        let (code, out, err) = repo.run(&["init"]);
        assert_eq!(code, 0, "{out}{err}");
        repo.write("evals/check.sh", CHECK_SH);
        for skill in packed {
            // The group `agents-line` has 2 evals of its own. The ids of
            // `execute` are 6 to 10, as in the pack (the pair uses `execute-10`).
            // The group `accept` has the skill `review` (design 0.8.0, D62).
            let (first, last) = match *skill {
                "agents-line" => (1, 2),
                "execute" => (6, 10),
                _ => (1, 5),
            };
            let key = if *skill == "accept" { "review" } else { skill };
            for n in first..=last {
                let id = format!("{skill}-{n}");
                repo.write(&format!("evals/pack/{id}.json"), &pack_eval(&id, key, runs));
                repo.write(
                    &format!("evals/pack/fixtures/{id}/notes.txt"),
                    "status: draft\n",
                );
                repo.write(
                    &format!("evals/pack/fixtures/{id}/spec-dna.toml"),
                    "spec-dna = \"0.0.1\"\n",
                );
            }
        }
        repo.commit("evals");
        let stubs = Repo::empty();
        stub_bin(&stubs.dir.join("bin"), "claude", STUB);
        fs::create_dir_all(stubs.dir.join("tmp")).unwrap();
        Fx { repo, stubs }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.stubs.dir.join(name)
    }

    fn modes(&self, modes: &[&str]) {
        fs::write(self.path("modes"), modes.join("\n") + "\n").unwrap();
    }

    fn run(&self, args: &[&str]) -> (i32, String, String) {
        self.run_env(args, &[])
    }

    /// `eval run --ab <args>`, with the stub first on `PATH` and no wait.
    fn run_env(&self, args: &[&str], extra: &[(&str, &str)]) -> (i32, String, String) {
        let mut full = vec!["eval", "run", "--ab"];
        full.extend_from_slice(args);
        self.raw(&full, extra)
    }

    fn raw(&self, args: &[&str], extra: &[(&str, &str)]) -> (i32, String, String) {
        let bin = path_with(&self.path("bin"));
        let dir = self.stubs.dir.to_string_lossy().into_owned();
        let tmp = self.path("tmp").to_string_lossy().into_owned();
        let results = self.repo.dir.join("evals/results");
        let results = results.to_string_lossy().into_owned();
        let mut set: Vec<(&str, &str)> = vec![
            ("STUB_RESULTS", &results),
            ("PATH", &bin),
            ("STUB_DIR", &dir),
            ("TMPDIR", &tmp),
            ("SPEC_DNA_EVAL_BACKOFF", "0"),
        ];
        set.extend_from_slice(extra);
        self.repo.run_env(args, &set, &[])
    }

    /// The records of the stub, after the `--version` call.
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

    /// The arguments of call `n` (0-based), without the prefix `ARG `.
    fn args(&self, n: usize) -> Vec<String> {
        self.calls()[n]
            .iter()
            .filter_map(|l| l.strip_prefix("ARG ").map(str::to_string))
            .collect()
    }

    fn has_file(&self, n: usize, file: &str) -> bool {
        self.calls()[n].iter().any(|l| l == &format!("FILE {file}"))
    }

    fn text(&self, n: usize, file: &str) -> String {
        let prefix = format!("TEXT {file}: ");
        self.calls()[n]
            .iter()
            .filter_map(|l| l.strip_prefix(&prefix))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn results(&self) -> Vec<String> {
        let dir = self.repo.dir.join("evals/results");
        let mut names: Vec<String> = fs::read_dir(dir)
            .map(|d| {
                d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    fn assert_clean(&self) {
        let left: Vec<_> = fs::read_dir(self.path("tmp"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert!(left.is_empty(), "the runner left {left:?}");
    }
}

fn skill_path(name: &str) -> String {
    format!(".claude/skills/spec-dna-{name}/SKILL.md")
}

/// The part of a compact JSON text between `"key":` and the next `,"`, for a
/// number or a string at the first place of the key.
fn field<'a>(json: &'a str, key: &str) -> &'a str {
    let at = json
        .find(&format!("\"{key}\":"))
        .unwrap_or_else(|| panic!("no key {key} in {json}"));
    let rest = &json[at + key.len() + 3..];
    let end = rest.find([',', '}']).unwrap_or(rest.len());
    &rest[..end]
}

// The usage errors.

#[test]
fn a_target_is_required_and_only_one_is_allowed() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    for args in [
        vec![],
        vec!["tdd", "debug"],
        vec!["nonsense"],
        vec!["intent"],
        vec!["spec-dna"],
    ] {
        let (code, _, err) = fx.run(&args);
        assert_eq!(code, 2, "{args:?}: {err}");
        assert!(err.contains("target"), "{args:?}: {err}");
        assert!(err.contains("Run `spec-dna --help` for usage."), "{err}");
    }
    assert!(fx.calls().is_empty(), "no process may start");
}

#[test]
fn min_pass_rate_and_ab_outside_eval_run_are_usage_errors() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    let (code, _, err) = fx.run(&["tdd", "--min-pass-rate", "0.5"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("--min-pass-rate"), "{err}");
    let (code, _, err) = fx.raw(&["check", "--ab"], &[]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("--ab is an option of `eval run`"), "{err}");
    let (code, _, err) = fx.raw(&["eval", "verdict", "--ab"], &[]);
    assert_eq!(code, 2, "{err}");
}

#[test]
fn the_backoff_variable_is_checked_before_any_run() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    for bad in ["", "abc", "-1", "3601", "1.5"] {
        let (code, _, err) = fx.run_env(
            &["tdd", "--max-total-usd", "100"],
            &[("SPEC_DNA_EVAL_BACKOFF", bad)],
        );
        assert_eq!(code, 2, "{bad}: {err}");
        assert!(err.contains("SPEC_DNA_EVAL_BACKOFF"), "{err}");
    }
    assert!(fx.calls().is_empty(), "no process may start");
    let (code, out, err) = fx.run_env(&["tdd", "--dry-run"], &[("SPEC_DNA_EVAL_BACKOFF", "3600")]);
    assert_eq!(code, 0, "{out}{err}");
}

// The estimate and the cap.

#[test]
fn a_dry_run_prints_the_pairs_and_the_estimate_and_starts_nothing() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 4);
    let (code, out, err) = fx.run(&["tdd", "--dry-run", "--max-total-usd", "100"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with(
            "eval run --ab tdd (dry run): 5 evals, 40 runs, estimate 40.00 USD, cap 100.00 USD\n"
        ),
        "{out}"
    );
    assert!(
        out.contains("tdd-1: 4 runs, max-budget-usd 1.00, estimate 8.00 USD\n"),
        "{out}"
    );
    assert!(fx.calls().is_empty());
    assert!(fx.results().is_empty(), "a dry run writes no file");
    let (code, out, err) = fx.run(&["tdd", "--dry-run", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    for part in [
        "\"dry_run\":true",
        "\"target\":\"tdd\"",
        "\"estimate_usd\":40",
        "\"runs\":40",
    ] {
        assert!(out.contains(part), "`{part}` not in {out}");
    }
    // `--runs` replaces the runs of each eval.
    let (code, out, _) = fx.run(&["tdd", "--dry-run", "--runs", "2", "--max-total-usd", "100"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("5 evals, 20 runs, estimate 20.00 USD"),
        "{out}"
    );
}

#[test]
fn an_estimate_above_the_cap_starts_and_stops_before_the_cap() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 4);
    // The default cap is 20, and the estimate is 40. Each run costs 2 USD, so
    // the 10th run ends at 20 USD, and the 11th could pass the cap.
    let (code, out, err) = fx.run_env(&["tdd", "--json"], &[("STUB_COST", "2")]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(
        err.contains("note: the estimate 40.00 USD is above max-total-usd 20.00 USD; the run stops before a run that could pass the cap, and --resume goes on"),
        "{err}"
    );
    assert_eq!(fx.calls().len(), 10);
    assert!(out.contains("\"stopped\":\"max-total-usd\""), "{out}");
    assert_eq!(fx.results().len(), 1, "the partial result is written");
    // `--resume` with a higher cap goes on, and the cap counts the spend of the file.
    let file = format!("evals/results/{}", fx.results()[0]);
    let (code, out, err) = fx.run(&["tdd", "--resume", &file, "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.calls().len(), 10 + 30);
    assert!(
        !err.contains("note: the estimate"),
        "no note for an estimate of the spend that fits: {err}"
    );
}

#[test]
fn a_dry_run_prints_the_estimate_above_the_cap_and_says_that_a_real_run_starts() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 4);
    let (code, out, err) = fx.run(&["tdd", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("5 evals, 40 runs, estimate 40.00 USD, cap 20.00 USD"),
        "{out}"
    );
    assert!(
        out.contains("a real run starts, and stops before a run that could pass the cap"),
        "{out}"
    );
    let (code, out, _) = fx.run(&["tdd", "--dry-run", "--json"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"above_cap\":true"), "{out}");
    let (_, out, _) = fx.run(&["tdd", "--dry-run", "--max-total-usd", "100"]);
    assert!(!out.contains("a real run starts"), "{out}");
    assert!(fx.calls().is_empty());
}

#[test]
fn the_spend_above_the_cap_stops_the_run_and_writes_the_partial_result() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 2);
    // 20 runs with a budget of 1 USD: the estimate is 20 USD. Each run reports
    // 2 USD, so the spend passes the cap of 20.5 USD after 11 runs.
    let (code, out, err) = fx.run_env(
        &["tdd", "--max-total-usd", "20.5", "--json"],
        &[("STUB_COST", "2")],
    );
    assert_eq!(code, 3, "{out}{err}");
    assert!(out.contains("\"stopped\":\"max-total-usd\""), "{out}");
    assert!(fx.calls().len() < 20, "the run must stop early");
    assert_eq!(fx.results().len(), 1, "the partial result is written");
    // No verdict for an incomplete skill.
    assert!(out.contains("\"verdict\":null"), "{out}");
    fx.assert_clean();
}

#[test]
fn a_kill_after_the_first_arm_keeps_the_paid_arm_in_the_result_file() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    // The second call kills the runner with SIGKILL. The first call, the arm
    // `with` of `tdd-1`, is paid, and the file must hold it.
    fx.modes(&["ok", "sigkill"]);
    let (code, _, _) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_ne!(code, 0);
    let names = fx.results();
    assert_eq!(names.len(), 1, "{names:?}");
    let text: String = fs::read_to_string(fx.repo.dir.join("evals/results").join(&names[0]))
        .unwrap()
        .split_whitespace()
        .collect();
    assert!(text.contains("\"stopped\":\"interrupted\""), "{text}");
    assert!(
        text.contains("\"eval\":\"tdd-1\",\"run\":1,\"with\":{\"claude_exit\":0"),
        "{text}"
    );
    assert!(text.contains("\"without\":null"), "{text}");
}

// The tokens, the models and the checks of each side (eval-records, R1 and R4).

/// The text of the first pair of the result: `tdd-1`, run 1.
/// The JSON text without the `"turns"` and `"seconds"` of each side: the
/// seconds differ from run to run.
fn no_time(json: &str) -> String {
    let mut out = String::new();
    let mut rest = json;
    while let Some(at) = rest.find(",\"turns\":") {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let sec = tail.find(",\"seconds\":").expect("seconds follow turns");
        let after = &tail[sec + ",\"seconds\":".len()..];
        let end = after.find([',', '}']).unwrap_or(after.len());
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

fn first_pair(json: &str) -> &str {
    let at = json
        .find("\"eval\":\"tdd-1\",\"run\":1")
        .expect("no first pair");
    let rest = &json[at..];
    &rest[..rest.find("{\"eval\"").map_or(rest.len(), |i| i.max(1))]
}

#[test]
fn each_side_keeps_the_tokens_and_the_models_of_its_run() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    // The first call is the arm `with` of `tdd-1`, and its tokens are the sums
    // over the 2 models. `thinkingTokens` is not added, and an absent field is 0.
    fx.modes(&["tokens"]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let pair = first_pair(&out);
    assert!(
        pair.contains("\"passed\":true,\"tokens\":{\"in\":11,\"out\":22,\"cache_read\":33,\"cache_write\":40},\"models\":[\"claude-a\",\"claude-b\"],\"checks\":"),
        "{pair}"
    );
    // The result file on the disk has the same text.
    let file: String = fs::read_to_string(fx.repo.dir.join("evals/results").join(&fx.results()[0]))
        .unwrap()
        .split_whitespace()
        .collect();
    assert!(
        file.contains("\"tokens\":{\"in\":11,\"out\":22,\"cache_read\":33,\"cache_write\":40}")
    );
    // The arm `without` ran with the default stub: one model, all fields 0.
    assert!(
        pair.contains("\"without\":{") && pair.contains("\"tokens\":{\"in\":0,\"out\":0,\"cache_read\":0,\"cache_write\":0},\"models\":[\"claude-test-1\"]"),
        "{pair}"
    );
}

#[test]
fn a_result_with_no_model_usage_has_null_tokens() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    // No `modelUsage`; then a value that is not an integer >= 0 (the models stay).
    fx.modes(&["nousage", "badusage"]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let pair = first_pair(&out);
    assert!(
        pair.contains("\"passed\":true,\"tokens\":null,\"models\":[],\"checks\":"),
        "{pair}"
    );
    assert!(
        pair.contains("\"passed\":true,\"tokens\":null,\"models\":[\"claude-a\"],\"checks\":"),
        "{pair}"
    );
}

#[test]
fn the_copy_has_the_line_of_this_binary_and_no_sd501_or_sd502_in_each_arm() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    // The line of the project is the line of another version, with a valid
    // marker: the hash covers the text of the line, not its version.
    let mark = format!("<!-- spec-dna v{VERSION} h:");
    for file in ["AGENTS.md", "CLAUDE.md"] {
        let text = fx.repo.read(file);
        assert!(text.contains(&mark), "{file}: {text}");
        fx.repo
            .write(file, &text.replace(&mark, "<!-- spec-dna v0.6.1 h:"));
    }
    fx.repo.commit("a line of another version");
    // The fixtures have a configuration with a harness, as a project has.
    let config = fx.repo.read("spec-dna.toml");
    for n in 1..=5 {
        fx.repo.write(
            &format!("evals/pack/fixtures/tdd-{n}/spec-dna.toml"),
            &config,
        );
    }
    fx.repo.commit("fixtures with a configuration");
    fx.modes(&["check"; 40]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let calls = fx.calls().len();
    assert_eq!(calls, 10, "5 evals, 2 arms");
    for n in 1..=calls {
        // The copy has the line of this binary.
        for file in ["AGENTS.md", "CLAUDE.md"] {
            let text = fx.text(n - 1, file);
            assert!(text.contains(&mark), "call {n} {file}: {text}");
            assert!(!text.contains("v0.6.1"), "call {n} {file}: {text}");
        }
        // `check --all` in the copy gives no SD501 and no SD502, in each arm.
        let report = fs::read_to_string(fx.path(&format!("check-{n}"))).unwrap_or_default();
        assert!(!report.is_empty(), "call {n}: no report");
        assert!(
            !report.contains("SD501") && !report.contains("SD502"),
            "call {n}: {report}"
        );
    }
}

#[test]
fn each_side_lists_its_check_names_and_results() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    // The arm `with` of `tdd-1` fails its check: it has the skill folder.
    let (code, out, err) = fx.run_env(
        &["tdd", "--max-total-usd", "100", "--json"],
        &[("STUB_PASS", "noskill"), ("STUB_SKILL", "tdd")],
    );
    assert_eq!(code, 0, "{out}{err}");
    let pair = no_time(first_pair(&out));
    let with = &pair[..pair.find("\"without\"").unwrap()];
    assert!(
        with.ends_with("\"checks\":[{\"name\":\"notes-done\",\"passed\":false}]},"),
        "{with}"
    );
    let without = &pair[pair.find("\"without\"").unwrap()..];
    assert!(
        without.contains("\"checks\":[{\"name\":\"notes-done\",\"passed\":true}]}"),
        "{without}"
    );
    // No command and no output of a check is in the result.
    assert!(!out.contains("file_has"), "{out}");
}

#[test]
fn a_resume_reads_back_the_tokens_the_models_and_the_checks() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.modes(&["tokens"]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--runs", "1", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let file = format!("evals/results/{}", fx.results()[0]);
    let (code, out, err) = fx.run(&[
        "tdd",
        "--resume",
        &file,
        "--runs",
        "2",
        "--max-total-usd",
        "100",
        "--json",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    let pair = first_pair(&out);
    assert!(
        pair.contains("\"tokens\":{\"in\":11,\"out\":22,\"cache_read\":33,\"cache_write\":40},\"models\":[\"claude-a\",\"claude-b\"],\"checks\":[{\"name\":\"notes-done\",\"passed\":true}]"),
        "{pair}"
    );
}

#[test]
fn resume_with_more_runs_extends_each_eval() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 4);
    // The smoke: 1 run of each of the 5 evals.
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--runs", "1", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.calls().len(), 10);
    let file = format!("evals/results/{}", fx.results()[0]);
    // The smoke becomes run 1 of the batch: 3 more runs of each eval.
    let (code, out, err) = fx.run(&[
        "tdd",
        "--resume",
        &file,
        "--runs",
        "4",
        "--max-total-usd",
        "100",
        "--json",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.calls().len(), 10 + 30);
    let top = &out[..out.find("\"skills\":").unwrap()];
    assert!(top.contains("\"runs\":4"), "{top}");
    assert!(out.contains("\"with\":{\"runs\":20,"), "{out}");
    assert!(out.contains("\"eval\":\"tdd-1\",\"run\":4"), "{out}");
    assert!(out.contains("\"stopped\":null"), "{out}");
    // The file on the disk is the same one, with `runs` 4.
    assert_eq!(fx.results().len(), 1);
    let disk: String = fs::read_to_string(fx.repo.dir.join(&file))
        .unwrap()
        .split_whitespace()
        .collect();
    assert!(disk[..disk.find("\"skills\":").unwrap()].contains("\"runs\":4"));
}

#[test]
fn resume_with_fewer_runs_is_a_usage_error() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 4);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--runs", "2", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let file = format!("evals/results/{}", fx.results()[0]);
    let before = fx.calls().len();
    let (code, _, err) = fx.run(&["tdd", "--resume", &file, "--runs", "1"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("--runs"), "{err}");
    // A file with no `runs` (each eval kept its own runs) takes no `--runs`.
    let fx2 = Fx::new(&["brainstorm"], &["tdd"], 1);
    let (code, out, err) = fx2.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let file2 = format!("evals/results/{}", fx2.results()[0]);
    let (code, _, err) = fx2.run(&["tdd", "--resume", &file2, "--runs", "2"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("--runs"), "{err}");
    assert_eq!(fx.calls().len(), before, "no process may start");
    assert_eq!(fx2.calls().len(), 10, "no process may start");
}

// The arms.

#[test]
fn the_arm_with_writes_the_skill_folder_also_when_the_skill_is_off() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    assert!(!fx.repo.dir.join(skill_path("tdd")).exists());
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.calls().len(), 10, "5 evals x 1 run x 2 arms");
    // The first arm of run 1 is `with`, and the second is `without`.
    assert!(fx.has_file(0, &skill_path("tdd")));
    assert!(!fx.has_file(1, &skill_path("tdd")));
    // The working tree never changes.
    assert!(!fx.repo.dir.join(skill_path("tdd")).exists());
    assert!(
        fx.repo
            .git_out(&["status", "--porcelain"])
            .contains("evals/results")
    );
    fx.assert_clean();
}

#[test]
fn the_skill_file_of_the_arm_with_holds_the_body_of_skill_show() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    // The stub copies the file into its log through a hook: read it from a run.
    let mut stub = STUB.to_string();
    stub = stub.replace(
        "    echo '---'\n} >>",
        "    [ -f .claude/skills/spec-dna-tdd/SKILL.md ] && sed 's|^|SKILL |' .claude/skills/spec-dna-tdd/SKILL.md\n    echo '---'\n} >>",
    );
    stub_bin(&fx.stubs.dir.join("bin"), "claude", &stub);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--runs", "1"]);
    assert_eq!(code, 0, "{out}{err}");
    let (_, shown, _) = fx.raw(&["skill", "show", "tdd"], &[]);
    let call = fx.calls()[0].clone();
    let skill: Vec<&str> = call
        .iter()
        .filter_map(|l| l.strip_prefix("SKILL "))
        .collect();
    let text = skill.join("\n");
    assert!(
        text.starts_with("---\nname: spec-dna-tdd\ndescription: "),
        "{text}"
    );
    assert!(text.contains(shown.trim_matches('\n')), "{text}");
}

#[test]
fn the_arm_without_has_no_skill_no_discipline_and_denies_the_command() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 2);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100"]);
    assert_eq!(code, 0, "{out}{err}");
    // Run 1: `with` first. Run 2: `without` first.
    let with = 0;
    let without = 1;
    let file = skill_path("tdd");
    assert!(fx.has_file(with, &file));
    assert!(!fx.has_file(without, &file));
    // The line of `AGENTS.md` and of `CLAUDE.md` names the discipline in `with` only.
    for f in ["AGENTS.md", "CLAUDE.md"] {
        assert!(fx.text(with, f).contains("test first (tdd)"), "{f}");
        let t = fx.text(without, f);
        assert!(!t.contains("(tdd)"), "{f}: {t}");
        assert!(!t.contains("test first"), "{f}: {t}");
        // The other disciplines stay, and the marker is a well-formed line.
        assert!(t.contains("cause first (debug)"), "{f}: {t}");
        assert!(t.contains("<!-- spec-dna v"), "{f}: {t}");
    }
    // The adapter skill loses the bullet of the discipline, and no other one.
    let adapter = ".claude/skills/spec-dna/SKILL.md";
    assert!(fx.text(with, adapter).contains("- Test first:"));
    let t = fx.text(without, adapter);
    assert!(!t.contains("- Test first:"), "{t}");
    assert!(!t.contains("(`tdd`)"), "{t}");
    assert!(t.contains("- Cause first:"), "{t}");
    // The denied command goes after the place of `--allowedTools`.
    let want = [
        "-p",
        "Change draft to done.",
        "--allowedTools",
        "Read,Edit,Skill",
        "--disallowedTools",
        "Bash(spec-dna skill:*)",
        "--output-format",
        "json",
        "--max-turns",
        "5",
        "--max-budget-usd",
        "1",
        "--permission-mode",
        "dontAsk",
        "--setting-sources",
        "project",
        "--strict-mcp-config",
    ];
    assert_eq!(fx.args(without), want);
    let mut with_args: Vec<String> = want.iter().map(ToString::to_string).collect();
    with_args.drain(4..6);
    assert_eq!(fx.args(with), with_args, "`with` denies nothing");
    fx.assert_clean();
}

#[test]
fn the_arm_without_keeps_a_skill_that_the_project_has_off_and_removes_a_skill_that_it_has_on() {
    // `tdd` is on in the project: `without` removes its file, `with` keeps it.
    let fx = Fx::new(&["brainstorm", "tdd"], &["tdd"], 1);
    assert!(fx.repo.dir.join(skill_path("tdd")).is_file());
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--runs", "1"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(fx.has_file(0, &skill_path("tdd")));
    assert!(!fx.has_file(1, &skill_path("tdd")));
    // Another skill file stays in both arms.
    assert!(fx.has_file(0, &skill_path("brainstorm")));
    assert!(fx.has_file(1, &skill_path("brainstorm")));
}

#[test]
fn the_arm_without_of_a_skill_with_no_discipline_changes_no_line() {
    let fx = Fx::new(&["brainstorm"], &["specify"], 1);
    let (code, out, err) = fx.run(&["specify", "--max-total-usd", "100", "--runs", "1"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(fx.has_file(0, &skill_path("specify")));
    assert!(!fx.has_file(1, &skill_path("specify")));
    assert_eq!(fx.text(0, "AGENTS.md"), fx.text(1, "AGENTS.md"));
    assert_eq!(
        fx.text(0, ".claude/skills/spec-dna/SKILL.md"),
        fx.text(1, ".claude/skills/spec-dna/SKILL.md")
    );
}

#[test]
fn the_arm_without_of_write_plan_drops_the_replace_rule() {
    let fx = Fx::new(&["brainstorm"], &["write-plan"], 1);
    let (code, out, err) = fx.run(&["write-plan", "--max-total-usd", "100", "--runs", "1"]);
    assert_eq!(code, 0, "{out}{err}");
    let adapter = ".claude/skills/spec-dna/SKILL.md";
    assert!(fx.text(0, adapter).contains("`write-plan`"));
    assert!(!fx.text(1, adapter).contains("write-plan"));
}

#[test]
fn the_pairs_alternate_the_first_arm_and_the_copy_has_the_pin_of_the_binary() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 3);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100"]);
    assert_eq!(code, 0, "{out}{err}");
    let file = skill_path("tdd");
    // For each eval: run 1 with first, run 2 without first, run 3 with first.
    let order: Vec<bool> = (0..fx.calls().len())
        .map(|n| fx.has_file(n, &file))
        .collect();
    let one_eval = [true, false, false, true, true, false];
    for chunk in order.chunks(6) {
        assert_eq!(chunk, one_eval);
    }
    // The fixture pins 0.0.1, and the copy holds the version of the binary.
    let toml = fx.calls()[0]
        .iter()
        .filter_map(|l| l.strip_prefix("TOML "))
        .collect::<Vec<_>>()
        .join("\n");
    // SD501 and SD502 are off in the copy, under a new table `[rules]`.
    assert_eq!(
        toml,
        format!("spec-dna = \"{VERSION}\"\n\n[rules]\nSD501 = \"off\"\nSD502 = \"off\"")
    );
}

#[test]
fn the_target_agents_line_runs_5_evals_with_no_skill_folder_in_each_arm() {
    let fx = Fx::new(&["brainstorm", "tdd"], &DISCIPLINED, 1);
    let (code, out, err) = fx.run(&["agents-line", "--dry-run", "--max-total-usd", "100"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("eval run --ab agents-line (dry run): 5 evals, 10 runs"),
        "{out}"
    );
    for id in [
        "tdd-1",
        "debug-1",
        "execute-10",
        "agents-line-1",
        "agents-line-2",
    ] {
        assert!(out.contains(&format!("{id}: 1 runs")), "{id}: {out}");
    }
    assert!(!out.contains("tdd-2"), "{out}");
    assert!(
        !out.contains("verify-1") && !out.contains("review-1"),
        "{out}"
    );
    let (code, out, err) = fx.run(&["agents-line", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"target\":\"agents-line\""), "{out}");
    assert!(out.contains("\"name\":\"agents-line\""), "{out}");
    // No arm has a folder of `.claude/skills/`.
    for n in 0..fx.calls().len() {
        assert!(
            !fx.calls()[n]
                .iter()
                .any(|l| l.starts_with("FILE .claude/skills/")),
            "{:?}",
            fx.calls()[n]
        );
    }
    // `with` has the line in both files. `without` has no line and denies the command.
    for f in ["AGENTS.md", "CLAUDE.md"] {
        assert!(fx.text(0, f).contains("spec-dna context"), "{f}");
        assert!(!fx.text(1, f).contains("spec-dna context"), "{f}");
    }
    assert!(!fx.args(0).contains(&"--disallowedTools".to_string()));
    let args = fx.args(1);
    let at = args.iter().position(|a| a == "--disallowedTools").unwrap();
    assert_eq!(args[at + 1], "Bash(spec-dna skill:*)");
}

// The result file.

#[test]
fn the_result_has_the_arms_the_pairs_the_interval_and_the_verdict() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 4);
    let (code, out, err) = fx.run_env(
        &["tdd", "--max-total-usd", "100", "--json"],
        &[("STUB_PASS", "skill"), ("STUB_SKILL", "tdd")],
    );
    assert_eq!(code, 0, "{out}{err}");
    let keys = [
        "spec_dna",
        "claude_code",
        "models",
        "started",
        "target",
        "seed",
        "resamples",
        "max_total_usd",
        "estimate_usd",
        "spend_usd",
        "excluded",
        "stopped",
        "skills",
    ];
    let at: Vec<usize> = keys
        .iter()
        .map(|k| {
            out.find(&format!("\"{k}\":"))
                .unwrap_or_else(|| panic!("{k}: {out}"))
        })
        .collect();
    assert!(
        at.windows(2).all(|w| w[0] < w[1]),
        "the keys keep their order: {out}"
    );
    for part in [
        &format!("\"spec_dna\":\"{VERSION}\""),
        "\"claude_code\":\"2.1.325\"",
        "\"models\":[\"claude-test-1\"]",
        "\"target\":\"tdd\"",
        "\"seed\":24",
        "\"resamples\":2000",
        "\"excluded\":{\"with\":0,\"without\":0}",
        "\"stopped\":null",
        "\"name\":\"tdd\"",
        "\"evals\":5",
        "\"with\":{\"runs\":20,\"passes\":20,\"pass_rate\":1,",
        "\"without\":{\"runs\":20,\"passes\":0,\"pass_rate\":0,",
        "\"verdict\":\"on\"",
        "\"rule\":1",
        "\"cost_ratio\":null",
        "\"pairs\":[{\"eval\":\"tdd-1\",\"run\":1,\"with\":{\"claude_exit\":0,\"subtype\":\"success\",\"cost_usd\":0.1,\"excluded\":0,\"permission_denials\":0,\"passed\":true,\"tokens\":{\"in\":0,\"out\":0,\"cache_read\":0,\"cache_write\":0},\"models\":[\"claude-test-1\"],\"checks\":[{\"name\":\"notes-done\",\"passed\":true}],\"turns\":4,\"seconds\":",
    ] {
        assert!(out.contains(part), "`{part}` not in {out}");
    }
    assert!(!out.contains("\"pass_rate\":null,\"passed\""), "{out}");
    // The top level has no `pass_rate`, so `stats --sdlc` skips the file.
    let top = &out[..out.find("\"skills\":").unwrap()];
    assert!(!top.contains("pass_rate"), "{top}");
    // The file is in `<[evals] dir>/results`, with the name of the start time.
    let names = fx.results();
    assert_eq!(names.len(), 1, "{names:?}");
    let name = &names[0];
    assert!(
        name.starts_with("skills-") && name.ends_with("Z.json"),
        "{name}"
    );
    let stamp = &name["skills-".len()..name.len() - ".json".len()];
    assert_eq!(stamp.len(), 16, "{stamp}");
    assert_eq!(&stamp[8..9], "T");
    assert!(stamp[..8].bytes().all(|b| b.is_ascii_digit()));
    // The text is the result, as indented JSON, and has no output of `claude`.
    let text = fs::read_to_string(fx.repo.dir.join("evals/results").join(name)).unwrap();
    assert!(text.contains("\n  \"skills\": ["), "{text}");
    assert!(!text.contains("\"result\""), "no output text");
    assert!(!text.contains("Change draft"), "no task text");
    fx.assert_clean();
}

#[test]
fn the_verdict_is_off_when_the_skill_makes_it_worse_and_inconclusive_when_nothing_changes() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 4);
    let (code, out, err) = fx.run_env(
        &["tdd", "--max-total-usd", "100", "--json"],
        &[("STUB_PASS", "noskill"), ("STUB_SKILL", "tdd")],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"verdict\":\"off\""), "{out}");
    assert!(out.contains("\"rule\":null"), "{out}");
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"verdict\":\"inconclusive\""), "{out}");
    assert!(
        out.contains("\"difference\":{\"value\":0,\"low\":0,\"high\":0}"),
        "{out}"
    );
    assert!(out.contains("\"cost_ratio\":{\"value\":1,"), "{out}");
}

#[test]
fn out_replaces_the_folder_of_the_result_and_the_text_output_names_the_verdict() {
    // 3 runs of 5 evals are 15 pairs: the floor of the verdict holds.
    let fx = Fx::new(&["brainstorm"], &["tdd"], 3);
    let (code, out, err) = fx.run_env(
        &["tdd", "--max-total-usd", "100", "--out", "evals/out"],
        &[("STUB_PASS", "skill"), ("STUB_SKILL", "tdd")],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with("eval run --ab tdd: 5 evals, 30 runs, estimate 30.00 USD"),
        "{out}"
    );
    assert!(out.contains("tdd: with 15 of 15 runs passed"), "{out}");
    assert!(out.contains("verdict on"), "{out}");
    assert!(out.contains("spend 3.00 USD"), "{out}");
    assert!(
        err.contains("eval run: the result is in evals/out/skills-"),
        "{err}"
    );
    assert!(fx.results().is_empty());
    let n = fs::read_dir(fx.repo.dir.join("evals/out")).unwrap().count();
    assert_eq!(n, 1);
    // `--out` cannot be `[evals] dir`.
    let (code, _, err) = fx.run(&["tdd", "--max-total-usd", "100", "--out", "evals"]);
    assert_eq!(code, 2, "{err}");
}

#[test]
fn the_target_all_runs_the_skills_of_the_pack_and_not_the_agents_line() {
    let fx = Fx::new(&["brainstorm"], &["tdd", "debug"], 1);
    let (code, out, err) = fx.run(&["all", "--dry-run", "--max-total-usd", "100"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("10 evals, 20 runs"), "{out}");
    let (code, out, err) = fx.run(&["all", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"name\":\"tdd\""), "{out}");
    assert!(out.contains("\"name\":\"debug\""), "{out}");
    assert!(!out.contains("agents-line"), "{out}");
}

#[test]
fn a_target_with_no_eval_in_the_pack_gives_exit_code_3() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    let (code, _, err) = fx.run(&["debug", "--max-total-usd", "100"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("eval pack"), "{err}");
    // An invalid file of the pack gives exit code 3 too.
    fx.repo
        .write("evals/pack/tdd-1.json", "{\"id\":\"tdd-1\"}\n");
    let (code, _, err) = fx.run(&["tdd", "--max-total-usd", "100"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("evals/pack/tdd-1.json"), "{err}");
    assert!(fx.calls().is_empty());
}

// The runs that the API refused.

#[test]
fn a_refused_attempt_of_each_class_is_retried_and_never_counted() {
    for mode in ["401", "403", "429", "503", "cf"] {
        let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
        // The first attempt of the first run is refused. The others work.
        fx.modes(&[mode]);
        let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
        assert_eq!(code, 0, "{mode}: {out}{err}");
        // 10 counted runs and 1 refused attempt.
        assert_eq!(fx.calls().len(), 11, "{mode}");
        assert!(
            out.contains("\"excluded\":{\"with\":1,\"without\":0}"),
            "{mode}: {out}"
        );
        // The retry has the same arm: the first two calls both have the skill.
        assert!(fx.has_file(0, &skill_path("tdd")), "{mode}");
        assert!(fx.has_file(1, &skill_path("tdd")), "{mode}");
        assert!(
            out.contains("\"with\":{\"runs\":5,\"passes\":5,"),
            "{mode}: the refused attempt is no run: {out}"
        );
        // The pair that was retried counts the excluded attempt.
        assert!(
            out.contains("\"with\":{\"claude_exit\":0,\"subtype\":\"success\",\"cost_usd\":0.1,\"excluded\":1,\"permission_denials\":0,\"passed\":true,"),
            "{mode}: {out}"
        );
        // The cost of the arm has the 5 counted runs only. The spend has the
        // refused attempt too: 10 x 0.1, and 0.5 for a refusal that reports it.
        let spend = if mode == "cf" { "1" } else { "1.5" };
        assert_eq!(field(&out, "spend_usd"), spend, "{mode}: {out}");
        assert!(
            out.contains("\"with\":{\"runs\":5,\"passes\":5,\"pass_rate\":1,\"cost_usd\":0.5,"),
            "{mode}: {out}"
        );
        fx.assert_clean();
    }
}

#[test]
fn a_run_that_fails_for_another_reason_counts() {
    // A failed check, a stop by the turns, a short answer about a limit with
    // exit code 0, and a long error that names 429 are runs.
    for mode in ["turns", "softlimit", "long429"] {
        let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
        fx.modes(&[mode]);
        let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
        assert_eq!(code, 0, "{mode}: {out}{err}");
        assert_eq!(fx.calls().len(), 10, "{mode}: no retry");
        assert!(
            out.contains("\"excluded\":{\"with\":0,\"without\":0}"),
            "{mode}: {out}"
        );
    }
    // A failed check: the edit is not made.
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    let (code, out, err) = fx.run_env(
        &["tdd", "--max-total-usd", "100", "--json"],
        &[("STUB_PASS", "none")],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.calls().len(), 10);
    assert!(out.contains("\"with\":{\"runs\":5,\"passes\":0,"), "{out}");
    assert!(
        out.contains("\"excluded\":{\"with\":0,\"without\":0}"),
        "{out}"
    );
}

#[test]
fn an_account_limit_a_timeout_and_a_5xx_status_are_refusals() {
    // The shapes of the lab harness of session 15, and an `is_error` result
    // with an `api_error_status` of any text.
    for mode in [
        "acct",
        "usage",
        "timeout",
        "overloaded",
        "s401x",
        "s500",
        "s529",
        "s503long",
        "netpage",
        "emptyexit",
        "plainfail",
        "plainbreak",
        "limit5",
        "limitweek",
        "limitplain",
        "stderrnet",
        "login",
        "credit",
        "outusage",
        "outextra",
        "unable",
        "connerr",
    ] {
        let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
        fx.modes(&[mode]);
        let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
        assert_eq!(code, 0, "{mode}: {out}{err}");
        assert_eq!(
            fx.calls().len(),
            11,
            "{mode}: one refused attempt, then 10 runs"
        );
        assert!(
            out.contains("\"excluded\":{\"with\":1,\"without\":0}"),
            "{mode}: {out}"
        );
    }
}

#[test]
fn a_run_out_of_turns_or_budget_and_a_status_that_is_no_refusal_count() {
    for mode in ["maxbudget", "turnsnotext", "s400", "s429noerr", "ctxlimit"] {
        let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
        fx.modes(&[mode]);
        let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
        assert_eq!(code, 0, "{mode}: {out}{err}");
        assert_eq!(fx.calls().len(), 10, "{mode}: no retry");
        assert!(
            out.contains("\"excluded\":{\"with\":0,\"without\":0}"),
            "{mode}: {out}"
        );
    }
}

#[test]
fn an_error_in_the_middle_of_a_run_writes_the_partial_result_before_exit_code_4() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    // The stub removes a fixture file of `tdd-3` during call 3, so the copy
    // of `tdd-3` fails after the 4 runs of `tdd-1` and `tdd-2`.
    fx.modes(&["ok", "ok", "break"]);
    let broken = fx.repo.dir.join("evals/pack/fixtures/tdd-3/notes.txt");
    let (code, out, err) = fx.run_env(
        &["tdd", "--max-total-usd", "100", "--json"],
        &[("STUB_BREAK_FILE", broken.to_str().unwrap())],
    );
    assert_eq!(code, 4, "{out}{err}");
    assert_eq!(fx.calls().len(), 4, "the runs before the error are done");
    let names = fx.results();
    assert_eq!(names.len(), 1, "{names:?}: one file, no temporary file");
    let text = fs::read_to_string(fx.repo.dir.join("evals/results").join(&names[0])).unwrap();
    assert!(text.contains("\"stopped\": \"error\""), "{text}");
    assert!(text.contains("\"error\": \"cannot copy"), "{text}");
    assert!(text.contains("\"eval\": \"tdd-2\""), "{text}");
    assert!(!text.contains("\"eval\": \"tdd-3\""), "{text}");
    assert!(text.contains("\"verdict\": null"), "{text}");
    // No path of the machine in the file.
    assert!(!text.contains(fx.repo.dir.to_str().unwrap()), "{text}");
    assert!(!text.contains(fx.path("tmp").to_str().unwrap()), "{text}");
    // The same object is on stdout, and stderr names the error.
    assert!(out.contains("\"stopped\":\"error\""), "{out}");
    assert!(err.contains("cannot copy"), "{err}");
    fx.assert_clean();
}

/// A home folder with a git configuration that breaks a commit: a signature
/// with a program that fails, and another identity.
fn hostile_home(fx: &Fx) -> String {
    let home = fx.path("home");
    fs::create_dir_all(&home).unwrap();
    fs::write(
        home.join(".gitconfig"),
        "[user]\n\tname = Real Person\n\temail = real@example.org\n[commit]\n\tgpgsign = true\n[gpg]\n\tprogram = /usr/bin/false\n",
    )
    .unwrap();
    home.to_string_lossy().into_owned()
}

#[test]
fn the_agent_and_the_checks_commit_with_a_neutral_identity_and_no_signature() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    // The checks commit too, and they read the identity of the agent commit.
    let eval = pack_eval("tdd-1", "tdd", 1).replace(
        "\"checks\":[",
        "\"checks\":[{\"name\":\"agent-identity\",\"run\":\"git log -1 --format=%ae | grep -q '^eval@example.invalid$'\"},{\"name\":\"check-commits\",\"run\":\"git commit --allow-empty -qm c\"},",
    );
    fx.repo.write("evals/pack/tdd-1.json", &eval);
    fx.modes(&["commit", "commit"]);
    let home = hostile_home(&fx);
    let (code, out, err) = fx.run_env(
        &["tdd", "--max-total-usd", "100", "--json"],
        &[("HOME", &home), ("XDG_CONFIG_HOME", &home)],
    );
    assert_eq!(code, 0, "{out}{err}");
    for n in [1, 2] {
        let who = fs::read_to_string(fx.path(&format!("who-{n}")))
            .unwrap_or_else(|e| panic!("the agent commit {n} failed: {e}"));
        assert!(who.contains("<eval@example.invalid>"), "{who}");
        assert!(!who.contains("real@example.org"), "{who}");
    }
    let out = no_time(&out);
    assert!(
        out.contains("\"eval\":\"tdd-1\",\"run\":1,\"with\":{\"claude_exit\":0,\"subtype\":\"success\",\"cost_usd\":0.1,\"excluded\":0,\"permission_denials\":0,\"passed\":true,\"tokens\":{\"in\":0,\"out\":0,\"cache_read\":0,\"cache_write\":0},\"models\":[\"claude-test-1\"],\"checks\":[{\"name\":\"agent-identity\",\"passed\":true},{\"name\":\"check-commits\",\"passed\":true},{\"name\":\"notes-done\",\"passed\":true}]},\"without\":{\"claude_exit\":0,\"subtype\":\"success\",\"cost_usd\":0.1,\"excluded\":0,\"permission_denials\":0,\"passed\":true,"),
        "{out}"
    );
}

#[test]
fn a_refusal_of_the_arm_without_counts_in_its_own_arm() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    // The second call is the arm `without` of the first pair.
    fx.modes(&["ok", "429", "503"]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.calls().len(), 12);
    assert!(
        out.contains("\"excluded\":{\"with\":0,\"without\":2}"),
        "{out}"
    );
}

#[test]
fn the_back_off_ends_with_api_error_exit_code_3_and_the_partial_result() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    // Each attempt is refused. 1 attempt and 6 retries make 7 calls.
    fx.modes(&["429"; 20]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 3, "{out}{err}");
    assert_eq!(fx.calls().len(), 7);
    assert!(out.contains("\"stopped\":\"api-error\""), "{out}");
    assert!(
        out.contains("\"excluded\":{\"with\":7,\"without\":0}"),
        "{out}"
    );
    assert!(out.contains("\"verdict\":null"), "{out}");
    // The charged cost of a refused attempt is in the spend: 7 x 0.5.
    assert_eq!(field(&out, "spend_usd"), "3.5");
    assert_eq!(fx.results().len(), 1, "the partial result is written");
    assert!(err.contains("api-error"), "{err}");
    fx.assert_clean();
}

#[test]
fn the_runner_waits_the_first_wait_before_the_retry() {
    // The wait of 1 second runs once, so the test stays short. The doubling
    // of the waits has a unit test in `eval_run.rs`.
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.modes(&["429"]);
    let start = std::time::Instant::now();
    let (code, out, err) = fx.run_env(
        &["tdd", "--max-total-usd", "100", "--json"],
        &[("SPEC_DNA_EVAL_BACKOFF", "1")],
    );
    assert_eq!(code, 0, "{out}{err}");
    let waited = start.elapsed().as_secs_f64();
    assert!(waited >= 1.0, "waited {waited} seconds");
    assert!(err.contains("wait 1 seconds"), "{err}");
}

// `eval verdict`.

const FILE_ON: &str = "{\"spec_dna\":\"0.7.0\",\"started\":\"2026-11-20T10:00:00Z\",\"stopped\":null,\"skills\":[{\"name\":\"tdd\",\"verdict\":\"on\",\"rule\":1},{\"name\":\"debug\",\"verdict\":\"off\",\"rule\":null}]}\n";

fn verdicts(out: &str) -> Vec<(String, String, String)> {
    out.lines()
        .map(|l| {
            let mut w = l.splitn(3, ' ');
            (
                w.next().unwrap().to_string(),
                w.next().unwrap_or("").to_string(),
                w.next().unwrap_or("").trim().to_string(),
            )
        })
        .collect()
}

/// The names that the table of the release holds a row for.
const RELEASED: [&str; 12] = [
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
    "agents-line",
];

/// The names with the verdict `on` in the table of the release.
const RELEASED_ON: [&str; 6] = [
    "brainstorm",
    "clarify",
    "tdd",
    "finish",
    "post-mortem",
    "agents-line",
];

#[test]
fn eval_verdict_prints_the_release_for_a_released_name_and_unmeasured_for_the_others() {
    let fx = Fx::new(&["brainstorm"], &[], 1);
    let (code, out, err) = fx.raw(&["eval", "verdict"], &[]);
    assert_eq!(code, 0, "{out}{err}");
    let rows = verdicts(&out);
    let names: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
    assert_eq!(
        names,
        [
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
            "agents-line"
        ]
    );
    for (name, verdict, source) in rows {
        assert!(RELEASED.contains(&name.as_str()), "{name}");
        assert_eq!(source, "release", "{name}");
        let on = RELEASED_ON.contains(&name.as_str());
        assert_eq!(verdict, if on { "on" } else { "inconclusive" }, "{name}");
    }
    assert!(fx.calls().is_empty(), "the command starts no agent");
    let (code, out, _) = fx.raw(&["eval", "verdict", "--json"], &[]);
    assert_eq!(code, 0);
    assert!(
        out.contains("{\"name\":\"tdd\",\"verdict\":\"on\",\"source\":\"release\",\"file\":null,\"below_floor\":false}"),
        "{out}"
    );
    assert!(
        out.contains(
            "{\"name\":\"brainstorm\",\"verdict\":\"on\",\"source\":\"release\",\"file\":null,\"below_floor\":false}"
        ),
        "{out}"
    );
}

#[test]
fn eval_verdict_release_prints_the_table_of_the_release() {
    let fx = Fx::new(&["brainstorm"], &[], 1);
    let (code, out, err) = fx.raw(&["eval", "verdict", "--release"], &[]);
    assert_eq!(code, 0, "{out}{err}");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 12, "{out}");
    let names: Vec<&str> = lines.iter().map(|l| l.split(' ').next().unwrap()).collect();
    assert_eq!(names, RELEASED);
    // `tdd` is on by rule 1, and the names with no pass are inconclusive.
    let tdd = lines[5];
    assert!(
        tdd.starts_with(
            "tdd on (rule 1): with 11/15 0.73, without 0/15 0.00, cost per correct 1.402285"
        ),
        "{tdd}"
    );
    assert!(
        tdd.contains(
            "difference 0.73 [0.47, 0.93], cost ratio -, batch 20261118T181423Z-tdd, claude 2.1.327"
        ),
        "{tdd}"
    );
    for l in &lines {
        let name = l.split(' ').next().unwrap();
        if !RELEASED_ON.contains(&name) {
            assert!(l.split(' ').nth(1) == Some("inconclusive:"), "{l}");
        }
    }
    assert!(fx.calls().is_empty(), "the command starts no agent");

    let (code, json, err) = fx.raw(&["eval", "verdict", "--release", "--json"], &[]);
    assert_eq!(code, 0, "{json}{err}");
    assert!(
        json.starts_with("{\"release\":[{\"name\":\"brainstorm\","),
        "{json}"
    );
    assert!(
        json.contains("{\"name\":\"tdd\",\"verdict\":\"on\",\"rule\":1,\"batch\":\"20261118T181423Z-tdd\",\"claude_code\":\"2.1.327\",\"models\":[\"claude-fable-5-2\",\"claude-opus-5-5\"],\"with\":{\"runs\":15,\"passes\":11,"),
        "{json}"
    );
    assert!(json.contains("\"cost_ratio\":null"), "{json}");
    assert_eq!(json.matches("\"name\":").count(), 12, "{json}");
    // The table needs no project: it reads no result file of the project.
    fx.repo.write(
        "evals/results/skills-20261120T100000Z.json",
        "{\"skills\":[{\"name\":\"tdd\",\"verdict\":\"off\"}]}\n",
    );
    let (_, again, _) = fx.raw(&["eval", "verdict", "--release"], &[]);
    assert_eq!(again, out);
}

#[test]
fn release_is_an_option_of_eval_verdict_only() {
    let fx = Fx::new(&["brainstorm"], &[], 1);
    for args in [
        vec!["eval", "pack", "--release"],
        vec!["eval", "run", "--release"],
        vec!["eval", "records", "x.json", "--release"],
        vec!["check", "--release"],
        vec!["eval", "verdict", "--release", "tdd"],
    ] {
        let (code, out, err) = fx.raw(&args, &[]);
        assert_eq!(code, 2, "{args:?}: {out}{err}");
        assert!(
            err.contains("Run `spec-dna --help` for usage."),
            "{args:?}: {err}"
        );
    }
}

#[test]
fn eval_verdict_reads_the_newest_file_that_holds_each_name() {
    let fx = Fx::new(&["brainstorm"], &[], 1);
    fx.repo
        .write("evals/results/skills-20261120T100000Z.json", FILE_ON);
    // A newer file holds `debug` only, and a stopped file has no verdict.
    fx.repo.write(
        "evals/results/skills-20261121T100000Z.json",
        "{\"started\":\"2026-11-21T10:00:00Z\",\"stopped\":null,\"skills\":[{\"name\":\"debug\",\"verdict\":\"inconclusive\",\"rule\":null}]}\n",
    );
    fx.repo.write(
        "evals/results/skills-20261122T100000Z.json",
        "{\"started\":\"2026-11-22T10:00:00Z\",\"stopped\":\"api-error\",\"skills\":[{\"name\":\"tdd\",\"verdict\":null,\"rule\":null},{\"name\":\"debug\",\"verdict\":null,\"rule\":null}]}\n",
    );
    // Not a result of `--ab`: no effect.
    fx.repo.write(
        "evals/results/eval-20261123T100000Z.json",
        "{\"pass_rate\":1}\n",
    );
    fx.repo
        .write("evals/results/skills-bad.json", "{ not json\n");
    let (code, out, err) = fx.raw(&["eval", "verdict"], &[]);
    assert_eq!(code, 0, "{out}{err}");
    let rows = verdicts(&out);
    let get = |n: &str| rows.iter().find(|r| r.0 == n).unwrap().clone();
    assert_eq!(get("tdd").1, "on");
    assert_eq!(
        get("tdd").2,
        "project evals/results/skills-20261120T100000Z.json"
    );
    assert_eq!(get("debug").1, "inconclusive");
    assert_eq!(
        get("debug").2,
        "project evals/results/skills-20261121T100000Z.json"
    );
    assert_eq!(get("brainstorm").1, "on");
    assert_eq!(get("brainstorm").2, "release");
    // A name that the release holds, and no result file of the project holds.
    assert_eq!(get("verify").1, "inconclusive");
    assert_eq!(get("verify").2, "release");
    let (_, json, _) = fx.raw(&["eval", "verdict", "--json"], &[]);
    assert!(
        json.contains("{\"name\":\"tdd\",\"verdict\":\"on\",\"source\":\"project\",\"file\":\"evals/results/skills-20261120T100000Z.json\",\"below_floor\":false}"),
        "{json}"
    );
}

#[test]
fn eval_verdict_works_with_no_evals_table_and_gives_3_for_a_bad_configuration() {
    let r = Repo::with_config("\n[[roots]]\npath = \"docs\"\n");
    r.write("docs/a.md", "# A\n\nText.\n");
    r.commit("start");
    let (code, out, err) = r.run(&["eval", "verdict"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("brainstorm on release\n"), "{out}");
    assert!(out.contains("tdd on release\n"), "{out}");
    r.write("spec-dna.toml", "this is = not [ toml\n");
    let (code, _, _) = r.run(&["eval", "verdict"]);
    assert_eq!(code, 3);
}

#[test]
fn eval_without_a_subcommand_names_run_pack_and_verdict() {
    let fx = Fx::new(&["brainstorm"], &[], 1);
    let (code, _, err) = fx.raw(&["eval"], &[]);
    assert_eq!(code, 2);
    assert!(err.contains("eval run"), "{err}");
    assert!(err.contains("eval pack"), "{err}");
    assert!(err.contains("eval verdict"), "{err}");
}

// The owner rule, the limits and the stop of a run (the checks of the slice).

#[test]
fn an_empty_output_with_a_failed_exit_is_a_refusal_and_a_context_limit_is_a_run() {
    // Covered by the lists of the 2 tests above; this test names the decision:
    // an empty output with a non-zero exit is retried and never counted.
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.modes(&["emptyexit"]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.calls().len(), 11);
    assert!(
        out.contains("\"excluded\":{\"with\":1,\"without\":0}"),
        "{out}"
    );
}

#[test]
fn five_counted_runs_in_a_row_with_an_error_and_no_cost_stop_the_command() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.modes(&["odd"; 8]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 3, "{out}{err}");
    assert_eq!(fx.calls().len(), 5, "the 5th run stops the command");
    assert!(out.contains("\"stopped\":\"api-error\""), "{out}");
    assert_eq!(fx.results().len(), 1, "the partial result is written");
}

#[test]
fn a_run_that_fails_with_no_cost_is_excluded_and_a_real_run_resets_the_row() {
    // 2 such results, a real run, and 2 more: 4 attempts are excluded, and
    // none counts. The 3rd result is a run, so the row starts again.
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.modes(&["odd", "odd", "ok", "odd", "odd"]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.calls().len(), 14, "10 runs and 4 excluded attempts");
    assert!(
        out.contains("\"excluded\":{\"with\":2,\"without\":2}"),
        "{out}"
    );
    assert!(out.contains("\"with\":{\"runs\":5,"), "{out}");
    // An error with a cost, and `error_max_turns`, are runs.
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.modes(&["turns", "ctxlimit", "toklimit"]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.calls().len(), 10);
    assert!(
        out.contains("\"excluded\":{\"with\":0,\"without\":0}"),
        "{out}"
    );
}

#[test]
fn a_plain_text_error_is_excluded_never_counted_and_never_charged() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.modes(&["plainbreak"; 20]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 3, "{out}{err}");
    assert_eq!(fx.calls().len(), 5);
    assert!(out.contains("\"stopped\":\"api-error\""), "{out}");
    assert!(out.contains("\"with\":{\"runs\":0,"), "{out}");
    assert!(out.contains("\"verdict\":null"), "{out}");
    assert_eq!(field(&out, "spend_usd"), "0", "{out}");
    // The same for an account limit as plain text: no cost, no charge.
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.modes(&["limitplain"]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(field(&out, "spend_usd"), "1", "{out}");
}

#[test]
fn the_result_has_runs_and_resume_uses_it() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 4);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--runs", "2", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"runs\":2,"), "{out}");
    let top = &out[..out.find("\"skills\":").unwrap()];
    assert!(top.contains("\"runs\":2"), "{top}");
    // Without `--runs`, the file says `null`: each eval keeps its own runs.
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    let (_, out, _) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    let top = &out[..out.find("\"skills\":").unwrap()];
    assert!(top.contains("\"runs\":null"), "{top}");

    // Interrupt a run of `--runs 2`, then resume.
    let fx = Fx::new(&["brainstorm"], &["tdd"], 4);
    fx.modes(&["ok", "ok", "ok", "ok", "sigint"]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--runs", "2", "--json"]);
    assert_eq!(code, 130, "{out}{err}");
    let file = format!("evals/results/{}", fx.results()[0]);
    let (code, _, err) = fx.run(&["tdd", "--resume", &file, "--runs", "1"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("--runs"), "{err}");
    let (code, out, err) = fx.run(&["tdd", "--resume", &file, "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    // 5 calls before, and the 16 runs that are left of 20.
    assert_eq!(fx.calls().len(), 5 + 16);
    assert!(out.contains("\"with\":{\"runs\":10,"), "{out}");
    // The same value of `--runs` is accepted.
    let (code, _, err) = fx.run(&["tdd", "--resume", &file, "--runs", "2"]);
    assert_eq!(code, 0, "{err}");
}

#[test]
fn an_interrupt_during_the_checks_keeps_the_cost_of_the_paid_agent_run() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    let eval = pack_eval("tdd-1", "tdd", 1).replace(
        "\"checks\":[",
        "\"checks\":[{\"name\":\"stop\",\"run\":\"kill -INT $PPID; sleep 30\"},",
    );
    fx.repo.write("evals/pack/tdd-1.json", &eval);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 130, "{out}{err}");
    assert_eq!(field(&out, "spend_usd"), "0.1", "{out}");
    fx.assert_clean();
}

#[test]
fn a_background_child_of_the_agent_does_not_hold_the_runner() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.modes(&["bg"]);
    let start = std::time::Instant::now();
    let (code, out, err) = fx.run_env(
        &["tdd", "--max-total-usd", "100", "--json"],
        &[("SPEC_DNA_EVAL_TIMEOUT", "3")],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        start.elapsed().as_secs() < 30,
        "the runner waited for a child of the agent"
    );
}

#[test]
fn sighup_is_trapped_like_sigterm() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.modes(&["ok", "ok", "sighup"]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 129, "{out}{err}");
    assert!(out.contains("\"stopped\":\"interrupted\""), "{out}");
    assert_eq!(fx.results().len(), 1);
    fx.assert_clean();
}

#[test]
fn an_account_limit_gets_the_long_ladder_and_another_refusal_the_short_one() {
    // 1 attempt and 16 retries (6 waits of the short ladder, 10 more).
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.modes(&["login"; 40]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "1000", "--json"]);
    assert_eq!(code, 3, "{out}{err}");
    assert_eq!(fx.calls().len(), 17);
    assert!(out.contains("\"stopped\":\"api-error\""), "{out}");
    assert!(
        out.contains("\"excluded\":{\"with\":17,\"without\":0}"),
        "{out}"
    );
    // A 503 keeps the ladder of 6 retries.
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.modes(&["503"; 40]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "1000", "--json"]);
    assert_eq!(code, 3, "{out}{err}");
    assert_eq!(fx.calls().len(), 7);
}

#[test]
fn the_agent_and_the_checks_find_the_binary_under_test_first_on_path() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    // A fake `spec-dna` is earlier on `PATH` than the binary under test.
    stub_bin(
        &fx.stubs.dir.join("bin"),
        "spec-dna",
        "#!/bin/sh\necho fake\n",
    );
    let eval = pack_eval("tdd-1", "tdd", 1).replace(
        "\"checks\":[",
        "\"checks\":[{\"name\":\"which\",\"run\":\"command -v spec-dna > \\\"$STUB_DIR/check-which\\\"\"},",
    );
    fx.repo.write("evals/pack/tdd-1.json", &eval);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--runs", "1"]);
    assert_eq!(code, 0, "{out}{err}");
    let want = format!("WHICH {}", common::BIN);
    for call in fx.calls() {
        assert!(call.contains(&want), "{want} not in {call:?}");
    }
    let which = fs::read_to_string(fx.path("check-which")).unwrap();
    assert_eq!(which.trim(), common::BIN);
}

#[test]
fn the_result_is_written_after_each_pair() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    // Call 5 is the first call of the 3rd pair. It copies the result file.
    fx.modes(&["ok", "ok", "ok", "ok", "snap"]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100"]);
    assert_eq!(code, 0, "{out}{err}");
    let snap = fs::read_to_string(fx.path("snap-5.json")).expect("a file after 2 pairs");
    assert!(snap.contains("\"eval\": \"tdd-2\""), "{snap}");
    assert!(!snap.contains("\"eval\": \"tdd-3\""), "{snap}");
    assert!(snap.contains("\"verdict\": null"), "{snap}");
    // One file at the end, and no temporary file.
    assert_eq!(fx.results().len(), 1, "{:?}", fx.results());
}

/// The part of a result from `"skills":` on, which has no time and no spend.
fn skills_of(json: &str) -> String {
    no_time(&json[json.find("\"skills\":").expect("skills")..])
}

#[test]
fn an_interrupt_writes_the_result_and_resume_gives_the_totals_of_an_uninterrupted_run() {
    for (mode, exit) in [("sigint", 130), ("sigterm", 143)] {
        let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
        // Call 5 is the first call of the 3rd pair: it sends the signal.
        fx.modes(&["ok", "ok", "ok", "ok", mode]);
        let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
        assert_eq!(code, exit, "{mode}: {out}{err}");
        assert!(out.contains("\"stopped\":\"interrupted\""), "{mode}: {out}");
        let names = fx.results();
        assert_eq!(names.len(), 1, "{mode}: {names:?}");
        let file = format!("evals/results/{}", names[0]);
        let text = fs::read_to_string(fx.repo.dir.join(&file)).unwrap();
        assert!(text.contains("\"eval\": \"tdd-2\""), "{mode}: {text}");
        assert!(!text.contains("\"eval\": \"tdd-3\""), "{mode}: {text}");
        fx.assert_clean();
        // The 5th call was killed with its process group: one call only.
        assert_eq!(fx.calls().len(), 5, "{mode}");

        let (code, resumed, err) =
            fx.run(&["tdd", "--resume", &file, "--max-total-usd", "100", "--json"]);
        assert_eq!(code, 0, "{mode}: {resumed}{err}");
        // 6 runs are left for the 3 evals that are not done: 10 - 4 = 6.
        assert_eq!(fx.calls().len(), 5 + 6, "{mode}: only the missing runs run");
        assert_eq!(fx.results().len(), 1, "{mode}: one merged file");
        assert!(resumed.contains("\"stopped\":null"), "{mode}: {resumed}");

        let whole = Fx::new(&["brainstorm"], &["tdd"], 1);
        let (code, full, err) = whole.run(&["tdd", "--max-total-usd", "100", "--json"]);
        assert_eq!(code, 0, "{full}{err}");
        assert_eq!(skills_of(&resumed), skills_of(&full), "{mode}");
        assert_eq!(field(&resumed, "spend_usd"), field(&full, "spend_usd"));
        // The first arm that the signal killed was not counted, and not charged.
        fx.assert_clean();
    }
}

#[test]
fn resume_checks_its_file_and_its_target() {
    let fx = Fx::new(&["brainstorm"], &["tdd", "debug"], 1);
    let (code, _, err) = fx.run(&["tdd", "--resume", "evals/results/absent.json"]);
    assert_eq!(code, 2, "{err}");
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let file = format!("evals/results/{}", fx.results()[0]);
    // Another target, `--out` and `--dry-run` with `--resume` are usage errors.
    for args in [
        vec!["debug", "--resume", &file],
        vec!["tdd", "--resume", &file, "--out", "evals/other"],
        vec!["tdd", "--resume", &file, "--dry-run"],
        vec!["tdd", "--resume", "../x.json"],
    ] {
        let (code, _, err) = fx.run(&args);
        assert_eq!(code, 2, "{args:?}: {err}");
    }
    // A complete file: nothing is left to run.
    let before = fx.calls().len();
    let (code, _, err) = fx.run(&["tdd", "--resume", &file, "--max-total-usd", "100"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(fx.calls().len(), before);
}

#[test]
fn an_agent_that_runs_too_long_is_killed_and_counts_as_a_failed_run() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.modes(&["hang"]);
    let start = std::time::Instant::now();
    let (code, out, err) = fx.run_env(
        &["tdd", "--max-total-usd", "100", "--json"],
        &[("SPEC_DNA_EVAL_TIMEOUT", "1")],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(start.elapsed().as_secs() < 25, "the hang was not cut");
    assert_eq!(fx.calls().len(), 10, "no retry: a timeout is no refusal");
    assert!(
        out.contains("\"with\":{\"claude_exit\":-1,\"subtype\":\"timeout\",\"cost_usd\":null,\"excluded\":0,\"permission_denials\":null,\"passed\":false,"),
        "{out}"
    );
    assert!(
        out.contains("\"excluded\":{\"with\":0,\"without\":0}"),
        "{out}"
    );
    fx.assert_clean();
    // A bad value is a usage error.
    for bad in ["0", "abc", "", "-5"] {
        let (code, _, err) = fx.run_env(
            &["tdd", "--max-total-usd", "100"],
            &[("SPEC_DNA_EVAL_TIMEOUT", bad)],
        );
        assert_eq!(code, 2, "{bad}: {err}");
    }
}

#[test]
fn a_check_that_runs_too_long_is_killed_and_fails() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    let eval = pack_eval("tdd-1", "tdd", 1).replace(
        "\"checks\":[",
        "\"checks\":[{\"name\":\"slow\",\"run\":\"sleep 30\"},",
    );
    fx.repo.write("evals/pack/tdd-1.json", &eval);
    let start = std::time::Instant::now();
    let (code, out, err) = fx.run_env(
        &["tdd", "--max-total-usd", "100", "--json"],
        &[("SPEC_DNA_EVAL_CHECK_TIMEOUT", "1")],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(start.elapsed().as_secs() < 25);
    assert!(
        out.contains(
            "\"eval\":\"tdd-1\",\"run\":1,\"with\":{\"claude_exit\":0,\"subtype\":\"timeout\""
        ),
        "{out}"
    );
    assert!(
        out.contains("\"excluded\":{\"with\":0,\"without\":0}"),
        "{out}"
    );
}

#[test]
fn the_fixture_text_of_the_instruction_files_stays_in_both_arms() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.repo.write(
        "evals/pack/fixtures/tdd-1/AGENTS.md",
        "Fixture rule: run sh tests/t.sh.\n",
    );
    fx.repo
        .write("evals/pack/fixtures/tdd-1/CLAUDE.md", "Fixture notes.\n");
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--runs", "1"]);
    assert_eq!(code, 0, "{out}{err}");
    // Call 0 is `with` and call 1 is `without` of `tdd-1`.
    for n in [0, 1] {
        assert!(
            fx.text(n, "AGENTS.md")
                .contains("Fixture rule: run sh tests/t.sh."),
            "{n}"
        );
        assert!(fx.text(n, "CLAUDE.md").contains("Fixture notes."), "{n}");
    }
    assert!(fx.text(0, "AGENTS.md").contains("test first (tdd)"));
    assert!(!fx.text(1, "AGENTS.md").contains("(tdd)"));
    assert!(fx.text(1, "AGENTS.md").contains("cause first (debug)"));
}

#[test]
fn the_agents_line_arms_keep_the_fixture_text_too() {
    let fx = Fx::new(&["brainstorm"], &DISCIPLINED, 1);
    fx.repo.write(
        "evals/pack/fixtures/tdd-1/AGENTS.md",
        "Fixture rule: run sh tests/t.sh.\n",
    );
    let (code, out, err) = fx.run(&["agents-line", "--max-total-usd", "100", "--runs", "1"]);
    assert_eq!(code, 0, "{out}{err}");
    // `tdd-1` is the first eval of the pair, `with` first.
    assert!(
        fx.text(0, "AGENTS.md")
            .contains("Fixture rule: run sh tests/t.sh.")
    );
    assert!(fx.text(0, "AGENTS.md").contains("spec-dna context"));
    assert!(
        fx.text(1, "AGENTS.md")
            .contains("Fixture rule: run sh tests/t.sh.")
    );
    assert!(!fx.text(1, "AGENTS.md").contains("spec-dna context"));
}

#[test]
fn the_cap_is_checked_before_each_run_and_the_first_arm_of_a_pair_is_kept() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    // The budget is 1 USD and each run costs 1.5 USD. The spend is 9 USD
    // after 6 runs, and 10.5 USD after the 7th: the 8th run could pass the
    // cap of 10.5 USD, so it does not start.
    let (code, out, err) = fx.run_env(
        &["tdd", "--max-total-usd", "10.5", "--json"],
        &[("STUB_COST", "1.5")],
    );
    assert_eq!(code, 3, "{out}{err}");
    assert_eq!(fx.calls().len(), 7);
    assert!(out.contains("\"stopped\":\"max-total-usd\""), "{out}");
    // The 7th run is the first arm of the pair `tdd-4`. The file keeps it.
    assert!(
        out.contains("\"eval\":\"tdd-4\",\"run\":1,\"with\":{\"claude_exit\":0"),
        "{out}"
    );
    assert!(out.contains("\"without\":null"), "{out}");
    // The incomplete pair is in no count.
    assert!(out.contains("\"with\":{\"runs\":3,"), "{out}");
}

#[test]
fn the_arms_allow_the_tool_skill_and_the_result_counts_the_permission_denials() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.modes(&["denied", "ok"]);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    for n in [0, 1] {
        let args = fx.args(n);
        let at = args.iter().position(|a| a == "--allowedTools").unwrap();
        assert_eq!(args[at + 1], "Read,Edit,Skill", "call {n}");
    }
    let out = no_time(&out);
    assert!(
        out.contains("\"eval\":\"tdd-1\",\"run\":1,\"with\":{\"claude_exit\":0,\"subtype\":\"success\",\"cost_usd\":0.1,\"excluded\":0,\"permission_denials\":2,\"passed\":true,\"tokens\":null,\"models\":[],\"checks\":[{\"name\":\"notes-done\",\"passed\":true}]},\"without\":{\"claude_exit\":0,\"subtype\":\"success\",\"cost_usd\":0.1,\"excluded\":0,\"permission_denials\":0,"),
        "{out}"
    );
}

#[test]
fn isolate_is_the_default_of_ab_and_no_isolate_switches_it_off() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    let (code, out, err) = fx.run(&["tdd", "--max-total-usd", "100", "--runs", "1"]);
    assert_eq!(code, 0, "{out}{err}");
    for n in 0..2 {
        let args = fx.args(n);
        let tail: Vec<&str> = args.iter().rev().take(3).map(String::as_str).collect();
        assert_eq!(
            tail,
            ["--strict-mcp-config", "project", "--setting-sources"]
        );
    }
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    let (code, out, err) = fx.run(&[
        "tdd",
        "--max-total-usd",
        "100",
        "--runs",
        "1",
        "--no-isolate",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    for n in 0..2 {
        assert!(
            !fx.args(n).contains(&"--setting-sources".to_string()),
            "{n}"
        );
    }
    let (code, _, err) = fx.run(&["tdd", "--isolate", "--no-isolate"]);
    assert_eq!(code, 2, "{err}");
    // `--no-isolate` is an option of `--ab` only.
    let (code, _, err) = fx.raw(&["eval", "run", "--no-isolate"], &[]);
    assert_eq!(code, 2, "{err}");
}

// D57: the harness arms (`--arms`, `--harness`).

/// The id of the plain evals of the arm tests, in the order of their ids.
const TEAM: [&str; 2] = ["team-01", "team-02"];

impl Fx {
    /// Adds the plain evals `ids` to `evals/`, each with its own fixture. The
    /// fixture names its eval id, so the stub can tell which eval it runs.
    /// The host also gets a file of `.claude/` and a rule in `AGENTS.md` that
    /// no arm may copy.
    fn add_plain(&self, ids: &[&str]) {
        for id in ids {
            self.repo.write(
                &format!("evals/{id}.json"),
                &format!(
                    "{{\"id\":\"{id}\",\"task\":\"Change draft to done.\",\"fixture\":\"evals/fixtures/{id}\",\"allowed-tools\":[\"Read\",\"Edit\"],\"max-turns\":5,\"max-budget-usd\":1,\"runs\":1,\"checks\":[{{\"name\":\"notes-done\",\"run\":\"file_has notes.txt 'status: done'\"}}]}}\n"
                ),
            );
            let fixture = format!("evals/fixtures/{id}");
            self.repo
                .write(&format!("{fixture}/notes.txt"), "status: draft\n");
            self.repo
                .write(&format!("{fixture}/eval-id.txt"), &format!("{id}\n"));
            // A fixture with a root, so `spec-dna check` loads its configuration.
            self.repo.write(
                &format!("{fixture}/spec-dna.toml"),
                "spec-dna = \"0.0.1\"\n\n[[roots]]\npath = \"docs\"\n",
            );
            self.repo
                .write(&format!("{fixture}/docs/a.md"), "# A\n\nText.\n");
        }
        self.repo.write(".claude/host-only.md", "host only\n");
        let agents = fs::read_to_string(self.repo.dir.join("AGENTS.md")).unwrap_or_default();
        self.repo
            .write("AGENTS.md", &format!("{agents}\nHOST-ONLY-RULE\n"));
        self.repo.commit("plain evals");
    }

    /// `eval run --arms <list> <args>`.
    fn arms(&self, list: &str, args: &[&str], extra: &[(&str, &str)]) -> (i32, String, String) {
        let mut full = vec!["eval", "run", "--arms", list];
        full.extend_from_slice(args);
        self.raw(&full, extra)
    }

    /// The harness arm that call `n` ran in, read from the `[harness]` table
    /// that the runner wrote into the copy.
    fn arm_of(&self, n: usize) -> String {
        let toml: Vec<String> = self.calls()[n]
            .iter()
            .filter_map(|l| l.strip_prefix("TOML ").map(str::to_string))
            .collect();
        if toml.iter().any(|l| l == "level = \"none\"") {
            "none".to_string()
        } else if toml
            .iter()
            .any(|l| l.starts_with("advisor = ") && l.contains("claude-code = \"none\""))
        {
            "lean-no-advisor".to_string()
        } else if toml.iter().any(|l| l == "level = \"lean\"") {
            "lean".to_string()
        } else if toml.iter().any(|l| l == "level = \"full\"") {
            "full".to_string()
        } else {
            "project".to_string()
        }
    }

    fn eval_of(&self, n: usize) -> String {
        self.calls()[n]
            .iter()
            .find_map(|l| l.strip_prefix("EVAL ").map(str::to_string))
            .unwrap_or_default()
    }

    /// The (eval, arm) of each call, in the order of the calls.
    fn order(&self) -> Vec<(String, String)> {
        (0..self.calls().len())
            .map(|n| (self.eval_of(n), self.arm_of(n)))
            .collect()
    }

    /// The call that ran in the arm `arm`, for the first eval of the run.
    fn call_of(&self, arm: &str) -> usize {
        (0..self.calls().len())
            .find(|n| self.arm_of(*n) == arm)
            .unwrap_or_else(|| panic!("no call in the arm {arm}"))
    }
}

fn pair(a: &str, b: &str) -> (String, String) {
    (a.to_string(), b.to_string())
}

#[test]
fn the_arm_none_has_no_claude_folder_and_only_the_adapter_line() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM[..1]);
    let (code, out, err) = fx.arms("none,lean", &["--max-total-usd", "100"], &[]);
    assert_eq!(code, 0, "{out}{err}");
    let n = fx.call_of("none");
    let call = fx.calls()[n].clone();
    let files: Vec<&String> = call.iter().filter(|l| l.starts_with("FILE ")).collect();
    assert!(files.is_empty(), "the arm none has no .claude: {files:?}");
    // The arm gets the adapter line of this binary, and nothing of the host.
    for rel in ["AGENTS.md", "CLAUDE.md"] {
        let text = fx.text(n, rel);
        assert!(
            text.contains(&format!("<!-- spec-dna v{VERSION} h:")),
            "{rel}: {text}"
        );
        assert!(!text.contains("HOST-ONLY-RULE"), "{rel}: {text}");
    }
    assert!(!fx.text(n, "CLAUDE.md").contains("advisor"));
    // The table `[harness]` is in the copy, and the pin is the version of the binary.
    let toml: Vec<String> = fx.calls()[n]
        .iter()
        .filter_map(|l| l.strip_prefix("TOML ").map(str::to_string))
        .collect();
    assert!(
        toml.contains(&format!("spec-dna = \"{VERSION}\"")),
        "{toml:?}"
    );
    assert!(toml.contains(&"[harness]".to_string()), "{toml:?}");
    fx.assert_clean();
}

#[test]
fn lean_no_advisor_has_no_advisor_model_and_no_fallback_agent_and_lean_has_both() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM[..1]);
    let (code, out, err) = fx.arms(
        "none,lean-no-advisor,lean",
        &["--max-total-usd", "100"],
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    let (b, c) = (fx.call_of("lean-no-advisor"), fx.call_of("lean"));
    assert!(fx.has_file(b, ".claude/settings.json"));
    assert!(!fx.text(b, ".claude/settings.json").contains("advisorModel"));
    assert!(!fx.has_file(b, ".claude/agents/fallback-advisor.md"));
    assert!(
        !fx.text(b, "CLAUDE.md").contains("advisor"),
        "{}",
        fx.text(b, "CLAUDE.md")
    );
    assert!(fx.text(c, ".claude/settings.json").contains("advisorModel"));
    assert!(fx.has_file(c, ".claude/agents/fallback-advisor.md"));
    assert!(fx.text(c, "CLAUDE.md").contains("advisor"));
    // The arms differ only in the advisor: the other roles are the same.
    for file in [".claude/agents/verifier.md", ".claude/agents/Explore.md"] {
        assert!(fx.has_file(b, file) && fx.has_file(c, file), "{file}");
    }
    assert!(fx.has_file(b, ".claude/agents/model-team.md"));
    assert!(
        fx.calls()[b]
            .iter()
            .all(|l| !l.starts_with("FILE .claude/skills/"))
    );
    assert!(
        fx.calls()[c]
            .iter()
            .all(|l| !l.starts_with("FILE .claude/skills/"))
    );
}

#[test]
fn no_file_of_the_host_harness_is_in_any_arm() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM[..1]);
    let (code, out, err) = fx.arms(
        "none,lean-no-advisor,lean",
        &["--max-total-usd", "100"],
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.calls().len(), 3);
    for n in 0..3 {
        for file in [".claude/host-only.md", ".claude/skills/spec-dna/SKILL.md"] {
            assert!(!fx.has_file(n, file), "call {n} has {file}");
        }
        assert!(!fx.text(n, "AGENTS.md").contains("HOST-ONLY-RULE"));
    }
}

#[test]
fn each_arm_runs_with_the_main_model_and_the_effort_of_lean_and_isolated() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM[..1]);
    let (code, out, err) = fx.arms("none,lean", &["--max-total-usd", "100"], &[]);
    assert_eq!(code, 0, "{out}{err}");
    for n in 0..2 {
        let args = fx.args(n);
        let at = args.iter().position(|a| a == "--model").expect("--model");
        assert_eq!(
            &args[at..at + 4],
            ["--model", "opus", "--effort", "xhigh"],
            "{args:?}"
        );
        let at = args.iter().position(|a| a == "--setting-sources").unwrap();
        assert_eq!(
            &args[at..at + 3],
            ["--setting-sources", "project", "--strict-mcp-config"]
        );
    }
    // A plain `--ab` keeps the command line of 0.7.1: no model, no effort.
    let (code, out, err) = fx.run(&["tdd", "--runs", "1", "--max-total-usd", "100"]);
    assert_eq!(code, 0, "{out}{err}");
    let args = fx.args(fx.calls().len() - 1);
    assert!(!args.contains(&"--model".to_string()), "{args:?}");
    assert!(!args.contains(&"--effort".to_string()), "{args:?}");
    // The arm `project` of `--arms` gets the same main model, the same effort,
    // the pin and the adapter line of the binary as each other arm (D57), so
    // the main model never differs between the arms.
    let before = fx.calls().len();
    let (code, out, err) = fx.arms("project,none", &["--max-total-usd", "100"], &[]);
    assert_eq!(code, 0, "{out}{err}");
    let project = (before..fx.calls().len())
        .find(|n| fx.arm_of(*n) == "project")
        .unwrap();
    let args = fx.args(project);
    let at = args.iter().position(|a| a == "--model").expect("--model");
    assert_eq!(
        &args[at..at + 4],
        ["--model", "opus", "--effort", "xhigh"],
        "{args:?}"
    );
    assert!(
        fx.has_file(project, ".claude/host-only.md"),
        "the arm project is the host"
    );
    let toml: Vec<String> = fx.calls()[project]
        .iter()
        .filter_map(|l| l.strip_prefix("TOML ").map(str::to_string))
        .collect();
    assert!(
        toml.contains(&format!("spec-dna = \"{VERSION}\"")),
        "the pin: {toml:?}"
    );
    assert!(toml.contains(&"SD502 = \"off\"".to_string()), "{toml:?}");
}

#[test]
fn the_arm_project_gets_the_adapter_line_of_the_binary_and_the_model_flags_of_harness_project() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM[..1]);
    // The host has the adapter line of another version.
    let host = fs::read_to_string(fx.repo.dir.join("AGENTS.md")).unwrap();
    let old = host.replace(
        &format!("<!-- spec-dna v{VERSION} "),
        "<!-- spec-dna v0.0.1 ",
    );
    assert_ne!(old, host, "the host has the adapter line");
    fx.repo.write("AGENTS.md", &old);
    fx.repo.commit("old line");
    let (code, out, err) = fx.arms("project,none", &["--max-total-usd", "100"], &[]);
    assert_eq!(code, 0, "{out}{err}");
    let project = fx.call_of("project");
    let text = fx.text(project, "AGENTS.md");
    assert!(
        text.contains(&format!("<!-- spec-dna v{VERSION} h:")),
        "{text}"
    );
    assert!(!text.contains("v0.0.1"), "{text}");
    // `--harness project` of a plain run is a named arm too: it gets the
    // flags. A plain run with no `--harness` keeps the command line of 0.7.1.
    let before = fx.calls().len();
    let (code, out, err) = fx.raw(
        &[
            "eval",
            "run",
            "--harness",
            "project",
            "--max-total-usd",
            "100",
        ],
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    let args = fx.args(before);
    assert!(
        args.windows(4)
            .any(|w| w == ["--model", "opus", "--effort", "xhigh"]),
        "{args:?}"
    );
    let (code, out, err) = fx.raw(&["eval", "run", "--max-total-usd", "100"], &[]);
    assert_eq!(code, 0, "{out}{err}");
    let args = fx.args(before + 1);
    assert!(!args.contains(&"--model".to_string()), "{args:?}");
}

#[test]
fn the_order_of_the_arms_rotates_by_the_eval_and_the_wave() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM);
    let (code, out, err) = fx.arms("none,lean", &["--runs", "2", "--max-total-usd", "100"], &[]);
    assert_eq!(code, 0, "{out}{err}");
    // Wave 1 runs both evals, and then wave 2. The arms rotate left by
    // (eval index + wave) mod 2.
    assert_eq!(
        fx.order(),
        vec![
            pair("team-01", "lean"),
            pair("team-01", "none"),
            pair("team-02", "none"),
            pair("team-02", "lean"),
            pair("team-01", "none"),
            pair("team-01", "lean"),
            pair("team-02", "lean"),
            pair("team-02", "none"),
        ]
    );
    fx.assert_clean();
}

#[test]
fn three_arms_rotate_through_the_3_orders() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM[..1]);
    let (code, out, err) = fx.arms(
        "none,lean-no-advisor,lean",
        &["--runs", "3", "--max-total-usd", "100"],
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    let arms: Vec<String> = fx.order().into_iter().map(|p| p.1).collect();
    assert_eq!(
        arms,
        [
            "lean-no-advisor",
            "lean",
            "none", // wave 1: rotate by 1
            "lean",
            "none",
            "lean-no-advisor", // wave 2: rotate by 2
            "none",
            "lean-no-advisor",
            "lean", // wave 3: rotate by 0
        ]
    );
}

#[test]
fn a_stop_by_the_cap_after_1_and_a_half_waves_keeps_the_first_wave_only_in_the_analysis() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM);
    // A wave is 2 evals x 2 arms = 4 runs of 1 USD. The cap of 6.5 USD stops
    // before the 7th run: wave 1 is complete, and wave 2 has 2 runs.
    let (code, out, err) = fx.arms(
        "none,lean",
        &["--runs", "2", "--max-total-usd", "6.5", "--json"],
        &[("STUB_COST", "1")],
    );
    assert_eq!(code, 3, "{out}{err}");
    assert_eq!(fx.calls().len(), 6);
    assert!(out.contains("\"stopped\":\"max-total-usd\""), "{out}");
    assert!(out.contains("\"waves\":1"), "{out}");
    let names = fx.results();
    assert_eq!(names.len(), 1, "{names:?}");
    assert!(
        names[0].starts_with("arms-") && names[0].ends_with("Z.json"),
        "{names:?}"
    );
    // The file keeps the 6 runs, and the analysis uses 2 pairs of wave 1.
    let file = format!("evals/results/{}", names[0]);
    let (code, out, err) = fx.raw(
        &["eval", "verdict", "--compare", "lean,none", &file, "--json"],
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    for part in [
        "\"treatment\":\"lean\"",
        "\"control\":\"none\"",
        "\"waves\":1,\"planned_waves\":2",
        "\"evals\":2,\"pairs\":2",
        "\"verdict\":\"inconclusive\"",
        "\"below_floor\":true",
    ] {
        assert!(out.contains(part), "`{part}` not in {out}");
    }
    let (code, out, _) = fx.raw(&["eval", "verdict", "--compare", "lean,none", &file], &[]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("below the floor (2 pairs of 2 evals; the floor is 15 pairs of 5 evals)"),
        "{out}"
    );
    assert!(out.contains("waves: 1 complete of 2 planned"), "{out}");
    // `--resume` finishes wave 2.
    let (code, out, err) = fx.arms(
        "none,lean",
        &["--resume", &file, "--max-total-usd", "100", "--json"],
        &[("STUB_COST", "1")],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.calls().len(), 8, "only the 2 runs that were left");
    assert!(out.contains("\"waves\":2"), "{out}");
    assert!(out.contains("\"stopped\":null"), "{out}");
    assert_eq!(fx.results().len(), 1, "--resume writes the same file");
}

#[test]
fn the_result_of_arms_has_one_side_for_each_arm_with_the_turns_and_the_seconds() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM[..1]);
    let (code, out, err) = fx.arms("none,lean", &["--max-total-usd", "100", "--json"], &[]);
    assert_eq!(code, 0, "{out}{err}");
    for part in [
        &format!("\"spec_dna\":\"{VERSION}\""),
        "\"claude_code\":\"2.1.325\"",
        "\"arms\":[\"none\",\"lean\"]",
        "\"excluded\":{\"none\":0,\"lean\":0}",
        "\"waves\":1",
        "\"evals\":[{\"id\":\"team-01\",\"runs\":1,\"max_budget_usd\":1,\"results\":[{\"run\":1,\"arms\":{\"none\":{\"claude_exit\":0,",
        "\"turns\":4,\"seconds\":",
    ] {
        assert!(out.contains(part), "`{part}` not in {out}");
    }
    assert!(!out.contains("Change draft"), "no task text");
    assert!(
        !out.contains("\"verdict\""),
        "a result of arms has no verdict item"
    );
    // An arms file is no input of the verdict of a skill.
    assert!(fx.results()[0].starts_with("arms-"));
    let (code, out, _) = fx.raw(&["eval", "verdict"], &[]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn the_usage_errors_of_arms_and_harness_start_no_process() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM[..1]);
    let cases: [(&[&str], &str); 11] = [
        (&["--arms", "none,nonsense"], "no harness arm"),
        (&["--arms", "none"], "2 or 3 arms"),
        (&["--arms", "none,lean,full,project"], "2 or 3 arms"),
        (&["--arms", "none,none"], "twice"),
        (&["--arms", "none,lean", "--ab"], "exclude each other"),
        (&["--arms", "none,lean", "--no-isolate"], "--no-isolate"),
        (&["--harness", "lean", "--no-isolate"], "--no-isolate"),
        (&["--harness", "nonsense"], "no harness arm"),
        (
            &["--arms", "none,lean", "--harness", "lean"],
            "exclude each other",
        ),
        (
            &["--arms", "none,lean", "--min-pass-rate", "0.5"],
            "--min-pass-rate",
        ),
        (
            &["--ab", "tdd", "--harness", "lean", "--no-isolate"],
            "--no-isolate",
        ),
    ];
    for (args, message) in cases {
        let mut full = vec!["eval", "run"];
        full.extend_from_slice(args);
        let (code, _, err) = fx.raw(&full, &[]);
        assert_eq!(code, 2, "{args:?}: {err}");
        assert!(err.contains(message), "{args:?}: {err}");
    }
    assert!(fx.calls().is_empty(), "no process may start");
    // The options belong to `eval run`.
    for args in [
        vec!["check", "--arms", "none,lean"],
        vec!["eval", "verdict", "--arms", "none,lean"],
        vec!["eval", "pack", "--harness", "lean"],
        vec!["eval", "run", "--compare", "a,b"],
        vec!["check", "--compare", "a,b"],
    ] {
        let (code, _, err) = fx.raw(&args, &[]);
        assert_eq!(code, 2, "{args:?}: {err}");
    }
    // A usage error comes before the check of the pin.
    fx.repo.write(
        "spec-dna.toml",
        &fs::read_to_string(fx.repo.dir.join("spec-dna.toml"))
            .unwrap()
            .replace(VERSION, "0.0.1"),
    );
    let (code, _, err) = fx.arms("none,nonsense", &[], &[]);
    assert_eq!(code, 2, "{err}");
    let (code, _, err) = fx.arms("none,lean", &[], &[]);
    assert_eq!(code, 3, "the pin: {err}");
}

#[test]
fn a_dry_run_of_arms_prints_the_estimate_and_starts_nothing() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM);
    let (code, out, err) = fx.arms(
        "none,lean-no-advisor,lean",
        &["--runs", "5", "--dry-run", "--max-total-usd", "100"],
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with("eval run --arms none,lean-no-advisor,lean (dry run): 2 evals, 3 arms, 30 runs, estimate 30.00 USD, cap 100.00 USD\n"),
        "{out}"
    );
    assert!(
        out.contains("team-01: 5 runs, max-budget-usd 1.00, estimate 15.00 USD"),
        "{out}"
    );
    let (code, out, _) = fx.arms("none,lean", &["--dry-run", "--json"], &[]);
    assert_eq!(code, 0);
    assert!(out.contains("\"arms\":[\"none\",\"lean\"]"), "{out}");
    assert!(fx.calls().is_empty());
    assert!(fx.results().is_empty());
    // An id that is not an eval is a usage error.
    let (code, _, err) = fx.arms("none,lean", &["nonsense", "--dry-run"], &[]);
    assert_eq!(code, 2, "{err}");
    // Ids select the evals.
    let (code, out, _) = fx.arms("none,lean", &["team-02", "--dry-run"], &[]);
    assert_eq!(code, 0);
    assert!(out.contains("1 evals, 2 arms, 2 runs"), "{out}");
}

#[test]
fn resume_of_arms_refuses_other_arms_and_a_file_of_ab() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM[..1]);
    let (code, out, err) = fx.arms("none,lean", &["--max-total-usd", "100"], &[]);
    assert_eq!(code, 0, "{out}{err}");
    let file = format!("evals/results/{}", fx.results()[0]);
    for list in ["none,lean-no-advisor", "lean,none", "none,lean,full"] {
        let (code, _, err) = fx.arms(list, &["--resume", &file], &[]);
        assert_eq!(code, 2, "{list}: {err}");
        assert!(
            err.contains("the same arms in the same order"),
            "{list}: {err}"
        );
    }
    // `--resume` is for `--ab` and `--arms` only.
    let (code, _, err) = fx.raw(&["eval", "run", "--resume", &file], &[]);
    assert_eq!(code, 2, "{err}");
    // `--ab --resume` of a file with another harness.
    let (code, out, err) = fx.run(&["tdd", "--runs", "1", "--max-total-usd", "100"]);
    assert_eq!(code, 0, "{out}{err}");
    let ab = fx
        .results()
        .into_iter()
        .find(|n| n.starts_with("skills-"))
        .unwrap();
    let ab = format!("evals/results/{ab}");
    let (code, _, err) = fx.run(&["tdd", "--resume", &ab, "--harness", "lean"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("the file has the harness `project`"), "{err}");
    // A file of 0.7.1 has no key `harness`: it is the arm `project`.
    let text = fs::read_to_string(fx.repo.dir.join(&ab)).unwrap();
    assert!(text.contains("\"harness\": \"project\""), "{text}");
    fs::write(
        fx.repo.dir.join(&ab),
        text.replace("  \"harness\": \"project\",\n", ""),
    )
    .unwrap();
    let (code, out, err) = fx.run(&["tdd", "--resume", &ab, "--max-total-usd", "100"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn resume_of_arms_takes_the_eval_ids_of_its_file_and_runs_only_the_missing_sides() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM);
    // 1 eval, 2 runs, and a cap that stops before the 4th side.
    let (code, out, err) = fx.arms(
        "none,lean",
        &["team-02", "--runs", "2", "--max-total-usd", "3.5", "--json"],
        &[("STUB_COST", "1")],
    );
    assert_eq!(code, 3, "{out}{err}");
    assert_eq!(fx.calls().len(), 3);
    let file = format!("evals/results/{}", fx.results()[0]);
    // No id: the evals of the file, not all evals of the project.
    let (code, out, err) = fx.arms(
        "none,lean",
        &["--resume", &file, "--max-total-usd", "100", "--json"],
        &[("STUB_COST", "1")],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.calls().len(), 4, "only the 1 side that was left");
    assert_eq!(fx.eval_of(3), "team-02");
    assert!(out.contains("\"spend_usd\":4"), "{out}");
    assert!(!out.contains("team-01"), "{out}");
    assert_eq!(fx.results().len(), 1);
}

#[test]
fn resume_of_arms_with_fewer_ids_keeps_the_results_and_the_spend_of_the_other_evals() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM);
    // Wave 1 runs (4 sides, 4 USD), and the cap stops before wave 2.
    let (code, out, err) = fx.arms(
        "none,lean",
        &["--runs", "2", "--max-total-usd", "4.5", "--json"],
        &[("STUB_COST", "1")],
    );
    assert_eq!(code, 3, "{out}{err}");
    assert_eq!(fx.calls().len(), 4);
    let file = format!("evals/results/{}", fx.results()[0]);
    // An eval of the project that the file does not have is a usage error,
    // and no side runs.
    fx.add_plain(&["team-03"]);
    let (code, _, err) = fx.arms("none,lean", &["team-03", "--resume", &file], &[]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("team-03") && err.contains(&file), "{err}");
    assert_eq!(fx.calls().len(), 4);
    // The file has `team-01` and `team-02`, so `team-02` alone is allowed. It
    // runs the 2 sides of its run 2, and it does not run `team-01`.
    let (code, out, err) = fx.arms(
        "none,lean",
        &[
            "team-02",
            "--resume",
            &file,
            "--max-total-usd",
            "100",
            "--json",
        ],
        &[("STUB_COST", "1")],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.calls().len(), 6, "2 calls, not 4");
    assert_eq!(
        (fx.eval_of(4), fx.eval_of(5)),
        ("team-02".into(), "team-02".into())
    );
    // The spend of the file stays in the total, and `team-01` keeps its
    // wave-1 sides and its plan of 2 runs.
    assert!(out.contains("\"spend_usd\":6"), "{out}");
    for part in [
        "{\"id\":\"team-01\",\"runs\":2,",
        "{\"id\":\"team-02\",\"runs\":2,",
    ] {
        assert!(out.contains(part), "`{part}` not in {out}");
    }
    let text = fs::read_to_string(fx.repo.dir.join(&file)).unwrap();
    assert_eq!(text.matches("\"claude_exit\"").count(), 6, "{text}");
    // `team-01` has run 1 only, so wave 2 is not complete.
    assert!(out.contains("\"waves\":1"), "{out}");
}

#[test]
fn the_stop_message_of_arms_prints_the_command_that_goes_on() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM[..1]);
    fx.modes(&["ok", "sighup"]);
    let (code, out, err) = fx.arms("none,lean", &["--max-total-usd", "100"], &[]);
    assert_eq!(code, 129, "{out}{err}");
    let file = format!("evals/results/{}", fx.results()[0]);
    assert!(
        err.contains(&format!(
            "Go on with spec-dna eval run --arms none,lean --resume {file}\n"
        )),
        "{err}"
    );
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM[..1]);
    fx.modes(&["429"; 20]);
    let (code, out, err) = fx.arms("none,lean", &["--max-total-usd", "100"], &[]);
    assert_eq!(code, 3, "{out}{err}");
    let file = format!("evals/results/{}", fx.results()[0]);
    assert!(
        err.contains(&format!(
            "again later with spec-dna eval run --arms none,lean --resume {file}\n"
        )),
        "{err}"
    );
}

#[test]
fn compare_gives_usage_errors_for_the_arms_and_exit_code_3_for_other_files() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM[..1]);
    let (code, out, err) = fx.arms("none,lean", &["--max-total-usd", "100"], &[]);
    assert_eq!(code, 0, "{out}{err}");
    let file = format!("evals/results/{}", fx.results()[0]);
    for (pair, message) in [
        ("lean", "not 2 arm names"),
        ("lean,none,full", "not 2 arm names"),
        ("lean,", "not 2 arm names"),
        ("lean,lean", "named twice"),
        (
            "lean,full",
            "the file has no arm `full`; its arms are none, lean",
        ),
    ] {
        let (code, _, err) = fx.raw(&["eval", "verdict", "--compare", pair, &file], &[]);
        assert_eq!(code, 2, "{pair}: {err}");
        assert!(err.contains(message), "{pair}: {err}");
    }
    // The same name twice is a usage error before the file is read.
    let (code, _, err) = fx.raw(
        &[
            "eval",
            "verdict",
            "--compare",
            "lean,lean",
            "evals/missing.json",
        ],
        &[],
    );
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("named twice"), "{err}");
    // No file, 2 files, and `--release`.
    for args in [
        vec!["eval", "verdict", "--compare", "lean,none"],
        vec!["eval", "verdict", "--compare", "lean,none", &file, &file],
        vec![
            "eval",
            "verdict",
            "--compare",
            "lean,none",
            &file,
            "--release",
        ],
    ] {
        let (code, _, err) = fx.raw(&args, &[]);
        assert_eq!(code, 2, "{args:?}: {err}");
    }
    // A result of `--ab`, a file that is not JSON, and a missing file.
    let (code, out, err) = fx.run(&["tdd", "--runs", "1", "--max-total-usd", "100"]);
    assert_eq!(code, 0, "{out}{err}");
    let ab = fx
        .results()
        .into_iter()
        .find(|n| n.starts_with("skills-"))
        .unwrap();
    fs::write(fx.repo.dir.join("evals/results/not-json.json"), "nope").unwrap();
    for other in [
        format!("evals/results/{ab}"),
        "evals/results/not-json.json".to_string(),
        "evals/results/missing.json".to_string(),
    ] {
        let (code, _, err) = fx.raw(&["eval", "verdict", "--compare", "lean,none", &other], &[]);
        assert_eq!(code, 3, "{other}: {err}");
    }
    // The text of a complete run with the floor.
    let (code, out, err) = fx.raw(&["eval", "verdict", "--compare", "none,lean", &file], &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with("none against lean: none 1 of 1 runs passed, lean 1 of 1,"),
        "{out}"
    );
    assert!(
        out.contains("verdict inconclusive, below the floor (1 pairs of 1 evals;"),
        "{out}"
    );
}

#[test]
fn a_plain_run_with_harness_gets_the_arm_the_model_flags_and_the_name_in_its_result() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM[..1]);
    let (code, out, err) = fx.raw(
        &[
            "eval",
            "run",
            "--harness",
            "lean-no-advisor",
            "--max-total-usd",
            "100",
            "--json",
        ],
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"harness\":\"lean-no-advisor\""), "{out}");
    assert_eq!(fx.arm_of(0), "lean-no-advisor");
    assert!(!fx.text(0, ".claude/settings.json").contains("advisorModel"));
    assert!(!fx.has_file(0, ".claude/host-only.md"));
    let args = fx.args(0);
    assert!(
        args.windows(4)
            .any(|w| w == ["--model", "opus", "--effort", "xhigh"]),
        "{args:?}"
    );
    assert!(
        args.contains(&"--setting-sources".to_string()),
        "isolated: {args:?}"
    );
    // `--harness project` keeps the files of the host, and it isolates. It is a
    // named arm, so it gets the model flags of the other arms.
    let (code, out, err) = fx.raw(
        &[
            "eval",
            "run",
            "--harness",
            "project",
            "--max-total-usd",
            "100",
            "--json",
        ],
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"harness\":\"project\""), "{out}");
    let args = fx.args(1);
    assert!(
        args.contains(&"--setting-sources".to_string()),
        "isolated: {args:?}"
    );
    assert!(
        args.windows(4)
            .any(|w| w == ["--model", "opus", "--effort", "xhigh"]),
        "{args:?}"
    );
    assert!(fx.has_file(1, ".claude/host-only.md"));
    // Without `--harness`, a plain run is the arm `project`.
    let (code, out, err) = fx.raw(&["eval", "run", "--max-total-usd", "100", "--json"], &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"harness\":\"project\""), "{out}");
    assert!(!fx.args(2).contains(&"--model".to_string()));
    assert!(fx.has_file(2, ".claude/host-only.md"));
}

#[test]
fn ab_with_harness_gives_the_team_and_the_default_skills_to_each_arm() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    let (code, out, err) = fx.run(&[
        "tdd",
        "--harness",
        "lean",
        "--runs",
        "1",
        "--max-total-usd",
        "100",
        "--json",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"harness\":\"lean\""), "{out}");
    // Pairs run `with` first for run 1: calls 0 and 1 are one pair.
    let (with, without) = (0, 1);
    for n in [with, without] {
        assert_eq!(fx.arm_of(n), "lean");
        assert!(
            fx.has_file(n, ".claude/agents/fallback-advisor.md"),
            "the team of the arm"
        );
        assert!(
            !fx.has_file(n, ".claude/skills/spec-dna/SKILL.md"),
            "no file of the host"
        );
        assert!(
            fx.has_file(n, &skill_path("write-plan")),
            "a default skill: {:?}",
            fx.calls()[n]
        );
        let args = fx.args(n);
        assert!(
            args.windows(4)
                .any(|w| w == ["--model", "opus", "--effort", "xhigh"])
        );
    }
    // The edit of `--ab` adds the skill to `with` and removes it from `without`.
    assert!(fx.has_file(with, &skill_path("tdd")));
    assert!(!fx.has_file(without, &skill_path("tdd")));
    let toml = |n: usize| -> String {
        fx.calls()[n]
            .iter()
            .filter_map(|l| l.strip_prefix("TOML "))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert!(toml(with).contains("\"tdd\""), "{}", toml(with));
    assert!(!toml(without).contains("\"tdd\""), "{}", toml(without));
    fx.assert_clean();
}

#[test]
fn a_result_of_ab_has_the_harness_the_turns_the_seconds_and_the_floor() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    let (code, out, err) = fx.run_env(
        &["tdd", "--max-total-usd", "100", "--json"],
        &[("STUB_PASS", "skill"), ("STUB_SKILL", "tdd")],
    );
    assert_eq!(code, 0, "{out}{err}");
    // 5 pairs: the arms differ, but the sample is below the floor.
    assert!(out.contains("\"harness\":\"project\",\"runs\""), "{out}");
    assert!(
        out.contains("\"verdict\":\"inconclusive\",\"rule\":null,\"below_floor\":true,\"pairs\":["),
        "{out}"
    );
    assert!(out.contains("\"turns\":4,\"seconds\":"), "{out}");
    let (code, out, _) = fx.run_env(
        &["tdd", "--max-total-usd", "100"],
        &[("STUB_PASS", "skill"), ("STUB_SKILL", "tdd")],
    );
    assert_eq!(code, 0);
    assert!(
        out.contains("verdict inconclusive, below the floor (5 pairs of 5 evals; the floor is 15 pairs of 5 evals)"),
        "{out}"
    );
    // `eval verdict` reads the file as `inconclusive`, with the floor.
    let (code, out, _) = fx.raw(&["eval", "verdict", "--json"], &[]);
    assert_eq!(code, 0);
    assert!(
        out.contains("{\"name\":\"tdd\",\"verdict\":\"inconclusive\",\"source\":\"project\""),
        "{out}"
    );
    assert!(out.contains("\"below_floor\":true"), "{out}");
    // 3 runs of 5 evals are 15 pairs: the floor holds, and the verdict is `on`.
    let fx = Fx::new(&["brainstorm"], &["tdd"], 3);
    let (code, out, err) = fx.run_env(
        &["tdd", "--max-total-usd", "100", "--json"],
        &[("STUB_PASS", "skill"), ("STUB_SKILL", "tdd")],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("\"verdict\":\"on\",\"rule\":1,\"below_floor\":false"),
        "{out}"
    );
}

/// The codes `SDnnn` that a report of `check` names, sorted and once.
fn codes(report: &str) -> Vec<String> {
    let mut found: Vec<String> = report
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| {
            w.len() == 5 && w.starts_with("SD") && w[2..].bytes().all(|b| b.is_ascii_digit())
        })
        .map(str::to_string)
        .collect();
    found.sort();
    found.dedup();
    found
}

#[test]
fn the_copy_of_each_built_in_arm_loads_its_configuration_and_passes_check() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    fx.add_plain(&TEAM[..1]);
    fx.modes(&["check"; 8]);
    for (list, calls) in [("none,lean-no-advisor,lean", 3), ("full,lean", 2)] {
        let before = fx.calls().len();
        let (code, out, err) = fx.arms(list, &["--max-total-usd", "100"], &[]);
        assert_eq!(code, 0, "{list}: {out}{err}");
        assert_eq!(fx.calls().len(), before + calls);
        for n in before + 1..=before + calls {
            let report = fs::read_to_string(fx.path(&format!("check-{n}"))).unwrap_or_default();
            assert!(!report.is_empty(), "{list} call {n}: no report");
            // The configuration loads, and no code of the harness appears.
            assert!(
                !report.starts_with("spec-dna: "),
                "{list} call {n}: {report}"
            );
            assert!(
                codes(&report).iter().all(|c| !c.starts_with("SD50")),
                "{list} call {n} ({}): {report}",
                fx.arm_of(n - 1)
            );
        }
    }
}

#[test]
fn ab_with_harness_gives_check_the_same_codes_in_with_and_without() {
    let fx = Fx::new(&["brainstorm"], &["tdd"], 1);
    for n in 1..=5 {
        let fixture = format!("evals/pack/fixtures/tdd-{n}");
        fx.repo.write(
            &format!("{fixture}/spec-dna.toml"),
            "spec-dna = \"0.0.1\"\n\n[[roots]]\npath = \"docs\"\n",
        );
        fx.repo
            .write(&format!("{fixture}/docs/a.md"), "# A\n\nText.\n");
    }
    fx.repo.commit("fixtures with a root");
    fx.modes(&["check"; 10]);
    let (code, out, err) = fx.run(&[
        "tdd",
        "--harness",
        "lean",
        "--runs",
        "1",
        "--max-total-usd",
        "100",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(fx.calls().len(), 10);
    // Calls 1 and 2 are the arms of the first pair, and so on.
    for pair in 0..5 {
        let (with, without) = (2 * pair + 1, 2 * pair + 2);
        let report =
            |n: usize| fs::read_to_string(fx.path(&format!("check-{n}"))).unwrap_or_default();
        let (a, b) = (report(with), report(without));
        assert!(!a.is_empty() && !b.is_empty(), "pair {pair}: no report");
        assert!(!a.starts_with("spec-dna: "), "with: {a}");
        assert!(!b.starts_with("spec-dna: "), "without: {b}");
        assert!(
            codes(&a).iter().all(|c| !c.starts_with("SD50")),
            "with: {a}"
        );
        assert!(
            codes(&b).iter().all(|c| !c.starts_with("SD50")),
            "without: {b}"
        );
        assert_eq!(codes(&a), codes(&b), "pair {pair}: with {a} without {b}");
    }
}

// Design 0.8.0, D62 and D76 item 3: the target `accept`.

/// The first line of the part `When to use` of the review of 0.7.1. The text of
/// `standard` does not have it.
const LITE_REVIEW: &str =
    "When to use: Before a merge, or when a person asks for a review or answers one.";

#[test]
fn the_target_accept_runs_the_5_evals_of_accept_and_the_target_review_keeps_its_5() {
    let fx = Fx::new(&["brainstorm"], &["review", "accept", "tdd"], 4);
    let ids = |target: &str| -> Vec<String> {
        let (code, out, err) = fx.run(&[target, "--dry-run", "--json", "--max-total-usd", "100"]);
        assert_eq!(code, 0, "{target}: {out}{err}");
        out.split("{\"id\":\"")
            .skip(1)
            .map(|p| p.split('"').next().unwrap().to_string())
            .collect()
    };
    assert_eq!(
        ids("accept"),
        ["accept-1", "accept-2", "accept-3", "accept-4", "accept-5"]
    );
    assert_eq!(
        ids("review"),
        ["review-1", "review-2", "review-3", "review-4", "review-5"]
    );
    // `all` runs the skills of 0.7.1 only: no eval of `accept`.
    assert!(ids("all").iter().all(|id| !id.starts_with("accept-")));
    // A dry run of `accept` prints 5 evals, 40 runs and the estimate: 5 evals,
    // 4 runs, 2 arms, and 1 USD for each run in this fixture.
    let (code, out, err) = fx.run(&["accept", "--dry-run", "--max-total-usd", "100"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains(
            "eval run --ab accept (dry run): 5 evals, 40 runs, estimate 40.00 USD, cap 100.00 USD"
        ),
        "{out}"
    );
}

#[test]
fn the_arm_with_has_acceptor_md_and_the_arm_without_has_the_review_of_0_7_1() {
    let fx = Fx::new(&["brainstorm"], &["accept"], 1);
    let (code, out, err) = fx.run(&[
        "accept",
        "--harness",
        "lean",
        "--runs",
        "1",
        "--max-total-usd",
        "100",
        "--json",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    // Run 1 of `accept-1`: call 0 is `with`, call 1 is `without`.
    let (with, without) = (0, 1);
    let review = skill_path("review");
    for n in [with, without] {
        assert_eq!(fx.arm_of(n), "lean");
        // Both arms have `verify` (D62): the builder checks its own proof.
        assert!(
            fx.has_file(n, &skill_path("verify")),
            "{n}: {:?}",
            fx.calls()[n]
        );
    }
    assert!(fx.has_file(with, ".claude/agents/acceptor.md"));
    assert!(!fx.has_file(without, ".claude/agents/acceptor.md"));
    assert!(fx.has_file(with, &review));
    assert!(
        fx.has_file(without, &review),
        "the arm without has the review of 0.7.1: {:?}",
        fx.calls()[without]
    );
    let with_text = fx.text(with, &review);
    let without_text = fx.text(without, &review);
    assert!(with_text.contains("acceptance.toml"), "{with_text}");
    assert!(!with_text.contains(LITE_REVIEW), "{with_text}");
    assert!(without_text.contains(LITE_REVIEW), "{without_text}");
    assert!(!without_text.contains("acceptance.toml"), "{without_text}");
    // The arm without has no word of the acceptor in its instruction files.
    for file in ["AGENTS.md", "CLAUDE.md"] {
        let text = fx.text(without, file);
        assert!(!text.contains("acceptor"), "{file}: {text}");
    }
    // The review of 0.7.1 is a skill of the arm, so its discipline stays in the
    // line of `AGENTS.md` in both arms (the target `review` removes it).
    for n in [with, without] {
        let text = fx.text(n, "AGENTS.md");
        assert!(text.contains("review is data (review)"), "{n}: {text}");
    }
    // Only `without` denies `spec-dna skill`, so it cannot read the new text.
    let denies = |n: usize| fx.args(n).iter().any(|a| a == "Bash(spec-dna skill:*)");
    assert!(!denies(with));
    assert!(denies(without));
    // The task is the same text in both arms.
    let task = |n: usize| {
        let args = fx.args(n);
        args[args.iter().position(|a| a == "-p").unwrap() + 1].clone()
    };
    assert_eq!(task(with), task(without));
    fx.assert_clean();
}

#[test]
fn the_arm_without_of_the_target_review_keeps_no_skill_file() {
    // The 5 evals of 0.7.1 measure the text against no text: the text of
    // `lite` for the target `accept` must not leak into them.
    let fx = Fx::new(&["brainstorm"], &["review"], 1);
    let (code, out, err) = fx.run(&[
        "review",
        "--harness",
        "lean",
        "--runs",
        "1",
        "--max-total-usd",
        "100",
        "--json",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    let review = skill_path("review");
    assert!(fx.has_file(0, &review));
    assert!(!fx.has_file(1, &review), "{:?}", fx.calls()[1]);
    assert!(!fx.has_file(1, ".claude/agents/acceptor.md"));
}

#[test]
fn without_harness_the_target_accept_gives_the_arms_the_2_texts_of_review() {
    let fx = Fx::new(&["brainstorm"], &["accept"], 1);
    let (code, out, err) = fx.run(&["accept", "--runs", "1", "--max-total-usd", "100", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let review = skill_path("review");
    assert!(fx.text(0, &review).contains("acceptance.toml"));
    assert!(fx.text(1, &review).contains(LITE_REVIEW));
}

#[test]
fn the_result_of_the_target_accept_is_a_result_of_the_skill_review_that_eval_records_reads() {
    let fx = Fx::new(&["brainstorm"], &["accept"], 1);
    let (code, out, err) = fx.run(&[
        "accept",
        "--harness",
        "lean",
        "--runs",
        "1",
        "--max-total-usd",
        "100",
        "--json",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"target\":\"accept\""), "{out}");
    assert!(out.contains("\"name\":\"review\""), "{out}");
    assert!(out.contains("\"harness\":\"lean\""), "{out}");
    // Each side has the cost, the turns and the seconds (D58).
    assert!(out.contains("\"turns\":4,\"seconds\":"), "{out}");
    assert!(out.contains("\"cost_usd\":"), "{out}");
    let files = fx.results();
    assert_eq!(files.len(), 1, "{files:?}");
    let path = format!("evals/results/{}", files[0]);
    let (code, out, err) = fx.raw(&["eval", "records", &path], &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.starts_with("{\"skill\":\"review\",\"eval\":\"accept-1\""),
        "{out}"
    );
    assert_eq!(out.lines().count(), 10, "5 evals, 1 run, 2 arms: {out}");
}
