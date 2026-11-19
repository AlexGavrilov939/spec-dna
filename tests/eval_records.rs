//! Tests of `spec-dna eval records <result file>` (ADR 0016, eval-records
//! change). The command reads one result file of `eval run --ab`, and prints
//! one record for each run of each item. It builds each record from 9 values
//! and copies no other field of the file. No test starts an agent.

mod common;

use common::Repo;
use std::fs;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/eval-records")
        .join(name)
}

/// Runs `spec-dna eval records` in an empty folder with the arguments.
fn records(args: &[&str]) -> (i32, String, String) {
    let dir = Repo::empty();
    let mut full = vec!["eval", "records"];
    full.extend_from_slice(args);
    dir.run(&full)
}

/// Runs the command on the text of a file of the lab.
fn records_of(text: &str) -> (i32, String, String) {
    let dir = Repo::empty();
    dir.write("result.json", text);
    dir.run(&["eval", "records", "result.json"])
}

fn small() -> String {
    fs::read_to_string(fixture("small.json")).unwrap()
}

const TOKENS: &str = r#""tokens":{"in":11,"out":22,"cache_read":33,"cache_write":44}"#;

/// The golden line of one record of `small.json`.
fn line(eval: &str, run: u32, arm: &str, cost: &str, first: bool) -> String {
    let pass = arm == "with";
    let (tokens, models) = if first {
        (TOKENS, r#"["claude-a"]"#)
    } else {
        (r#""tokens":null"#, r#"["claude-a","claude-b"]"#)
    };
    format!(
        r#"{{"skill":"tdd","eval":"{eval}","run":{run},"arm":"{arm}","pass":{pass},"cost":{cost},{tokens},"models":{models},"claude_code":"2.1.326"}}"#
    )
}

fn golden() -> String {
    let mut lines = Vec::new();
    for (eval, run, with_cost) in [
        ("tdd-1", 1, "0.5"),
        ("tdd-1", 2, "0.25"),
        ("tdd-2", 1, "0.75"),
        ("tdd-2", 2, "1"),
    ] {
        let first = eval == "tdd-1" && run == 1;
        lines.push(line(eval, run, "with", with_cost, first));
        lines.push(line(eval, run, "without", "0.5", false));
    }
    lines.join("\n") + "\n"
}

#[test]
fn eval_records_prints_one_record_per_run_with_the_9_keys() {
    let path = fixture("small.json");
    let (code, out, err) = records(&[path.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out, golden());
    assert_eq!(out.lines().count(), 8);
    assert_eq!(err, "eval records: 8 records of 1 names (tdd)\n");
}

#[test]
fn eval_records_copies_no_other_field_of_a_poisoned_file() {
    let path = fixture("poisoned.json");
    let (code, out, err) = records(&[path.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}{err}");
    // The poison is in fields that no record has, so the output is the golden text.
    assert_eq!(out, golden());
    for text in [&out, &err] {
        for bad in ["/", "sk-ant", "Users", "home", "secret", "AAAA", "\\"] {
            // The only `/` allowed is none: a record has no path.
            assert!(!text.contains(bad), "`{bad}` in {text}");
        }
    }
    // Each key and each value of a line has its form (no extra key).
    for l in out.lines() {
        let keys = [
            "skill",
            "eval",
            "run",
            "arm",
            "pass",
            "cost",
            "tokens",
            "models",
            "claude_code",
        ];
        let mut at = 0;
        for k in keys {
            let p = l[at..]
                .find(&format!("\"{k}\":"))
                .unwrap_or_else(|| panic!("no key {k} in order in {l}"));
            at += p;
        }
        assert_eq!(
            l.matches("\":").count(),
            9 + 4 * l.contains("\"in\":") as usize
        );
    }
}

#[test]
fn a_model_name_outside_the_form_stops_the_import() {
    for bad in [
        "a..b".to_string(),
        "/etc/x".to_string(),
        "a".repeat(65),
        "x y".to_string(),
        "a\\nb".to_string(),
    ] {
        let text = small().replace("\"claude-b\"", &format!("\"{bad}\""));
        let (code, out, err) = records_of(&text);
        assert_eq!(code, 3, "{bad}: {out}{err}");
        assert!(out.is_empty(), "no record for a stopped import: {out}");
        assert!(err.contains("models"), "{err}");
        assert!(!err.contains(&bad), "the message holds the value: {err}");
    }
    // A name in a side is checked too: the 2nd `"claude-a"` is in the first pair.
    let at = small().match_indices("\"claude-a\"").nth(1).unwrap().0;
    let mut text = small();
    text.replace_range(at..at + 10, "\"a..b\"");
    let (code, _, err) = records_of(&text);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("models") && !err.contains("a..b"), "{err}");
    // A bad version of Claude Code and a bad eval id stop the import too.
    for (from, to, key) in [
        ("\"2.1.326\"", "\"2.1 326\"", "claude_code"),
        ("\"tdd-2\"", "\"Tdd/2\"", "eval"),
    ] {
        let (code, _, err) = records_of(&small().replace(from, to));
        assert_eq!(code, 3, "{key}: {err}");
        assert!(err.contains(key), "{err}");
        assert!(!err.contains("Tdd/2") && !err.contains("2.1 326"), "{err}");
    }
}

#[test]
fn an_item_with_no_verdict_stops_the_import() {
    let text = small().replace("\"verdict\": \"on\"", "\"verdict\": null");
    let (code, out, err) = records_of(&text);
    assert_eq!(code, 3, "{out}{err}");
    assert!(out.is_empty(), "{out}");
    assert!(
        err.contains("the run of `tdd` is not complete; finish it with --resume"),
        "{err}"
    );
}

#[test]
fn a_summary_that_the_records_do_not_repeat_stops_the_import() {
    // `with.passes` is changed in the item, so the records disagree with it.
    let text = small().replacen("\"passes\": 4", "\"passes\": 3", 1);
    assert_ne!(text, small());
    let (code, out, err) = records_of(&text);
    assert_eq!(code, 3, "{out}{err}");
    assert!(out.is_empty(), "{out}");
    assert!(err.contains("`tdd`") && err.contains("passes"), "{err}");
    // A changed cost sum, a changed interval and a changed rule stop it too.
    for (from, to) in [
        ("\"cost_usd\": 2.5", "\"cost_usd\": 2.6"),
        ("\"low\": 1,", "\"low\": 0.5,"),
        ("\"rule\": 1", "\"rule\": 2"),
        ("\"verdict\": \"on\"", "\"verdict\": \"off\""),
    ] {
        let text = small().replacen(from, to, 1);
        assert_ne!(text, small(), "{from}");
        let (code, _, err) = records_of(&text);
        assert_eq!(code, 3, "{from}: {err}");
    }
}

#[test]
fn eval_records_takes_one_file_and_no_json() {
    let path = fixture("small.json");
    let p = path.to_str().unwrap();
    for args in [vec![], vec![p, p], vec![p, "--json"], vec!["--json"]] {
        let (code, out, err) = records(&args);
        assert_eq!(code, 2, "{args:?}: {out}{err}");
        assert!(out.is_empty(), "{out}");
        assert!(err.contains("eval records"), "{err}");
    }
    // An option of `eval run` is a usage error too.
    let (code, _, err) = records(&[p, "--runs", "2"]);
    assert_eq!(code, 2, "{err}");
}

#[test]
fn a_file_that_is_not_a_result_stops_the_import() {
    for text in [
        "not json",
        "[]",
        "{}",
        "{\"skills\": []}",
        "{\"skills\": 3}",
    ] {
        let (code, out, err) = records_of(text);
        assert_eq!(code, 3, "{text}: {out}{err}");
        assert!(out.is_empty(), "{out}");
        // No message holds the text of the file.
        assert!(!err.contains("not json"), "{err}");
    }
    let (code, _, err) = records(&["absent.json"]);
    assert_eq!(code, 3, "{err}");
}

#[test]
fn a_name_that_is_not_a_skill_or_agents_line_stops_the_import() {
    let text = small().replacen("\"name\": \"tdd\"", "\"name\": \"secret/tdd\"", 1);
    let (code, _, err) = records_of(&text);
    assert_eq!(code, 3, "{err}");
    assert!(!err.contains("secret"), "{err}");
}

#[test]
fn a_run_with_no_cost_counts_the_budget_of_its_eval_in_the_pack() {
    // The first run of `tdd-1` has no cost. `tdd-1` is in the pack, with the
    // budget 1.5, so the sum of the arm is 2.5 - 0.5 + 1.5 = 3.5.
    let text = small()
        .replacen("\"cost_usd\": 0.5,", "\"cost_usd\": null,", 1)
        .replacen("\"cost_usd\": 2.5", "\"cost_usd\": 3.5", 1)
        .replacen(
            "\"cost_per_correct_usd\": 0.625",
            "\"cost_per_correct_usd\": 0.875",
            1,
        );
    let (code, out, err) = records_of(&text);
    assert_eq!(code, 0, "{out}{err}");
    let first = out.lines().next().unwrap();
    assert!(
        first
            .contains("\"eval\":\"tdd-1\",\"run\":1,\"arm\":\"with\",\"pass\":true,\"cost\":null,"),
        "{first}"
    );
    // The summary of the file must repeat: the old sum is wrong.
    let (code, _, err) =
        records_of(&small().replacen("\"cost_usd\": 0.5,", "\"cost_usd\": null,", 1));
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("cost_usd"), "{err}");
}

#[test]
fn a_run_with_no_cost_of_an_eval_that_the_pack_lacks_stops_the_import() {
    let text = small().replace("\"tdd-2\"", "\"nothing-2\"").replacen(
        "\"cost_usd\": 0.75,",
        "\"cost_usd\": null,",
        1,
    );
    let (code, out, err) = records_of(&text);
    assert_eq!(code, 3, "{out}{err}");
    assert!(out.is_empty(), "{out}");
    assert!(err.contains("no cost") && err.contains("`tdd`"), "{err}");
    assert!(!err.contains("nothing-2"), "{err}");
}

#[test]
fn a_model_name_with_brackets_and_a_slash_round_trips() {
    // `claude-fable-5-2[1m]` is a name that Claude Code can write in `modelUsage`.
    for name in ["claude-fable-5-2[1m]", "vendor/model-1:beta"] {
        let text = small().replace("\"claude-b\"", &format!("\"{name}\""));
        let (code, out, err) = records_of(&text);
        assert_eq!(code, 0, "{name}: {out}{err}");
        assert!(
            out.contains(&format!("\"models\":[\"claude-a\",\"{name}\"]")),
            "{out}"
        );
    }
}

#[test]
fn a_secret_shaped_value_in_a_string_field_stops_the_import() {
    // Each value is inside the character set of its form, so only the shape
    // of a secret or a path can refuse it. The message names the key.
    let cases = [
        ("\"claude-b\"", "\"sk-ant-api03-AAAA\"", "models"),
        ("\"claude-b\"", "\"ghp_abcdefghijkl\"", "models"),
        ("\"claude-b\"", "\"AKIAIOSFODNN7EXAMPLE\"", "models"),
        ("\"claude-b\"", "\"x/Users/me\"", "models"),
        ("\"claude-b\"", "\"x/private/me\"", "models"),
        ("\"claude-b\"", "\"x/home/me\"", "models"),
        ("\"claude-b\"", "\"a@b\"", "models"),
        ("\"2.1.326\"", "\"sk-ant-1\"", "claude_code"),
        ("\"2.1.326\"", "\"ghp_x\"", "claude_code"),
        ("\"tdd-2\"", "\"sk-ant-2\"", "eval"),
        ("\"tdd-2\"", "\"a@b-2\"", "eval"),
    ];
    for (from, to, key) in cases {
        let (code, out, err) = records_of(&small().replace(from, to));
        assert_eq!(code, 3, "{to}: {out}{err}");
        assert!(out.is_empty(), "{to}: {out}");
        assert!(err.contains(key), "{to}: {err}");
        assert!(
            !err.contains(to.trim_matches('"')),
            "the message holds the value: {err}"
        );
    }
    // A plain eval id with the letters `sk-` inside a word is fine.
    let (code, out, err) = records_of(&small().replace("\"tdd-2\"", "\"task-2\""));
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_file_over_16_mib_stops_the_import_before_it_is_read() {
    let dir = Repo::empty();
    let path = dir.dir.join("big.json");
    let f = fs::File::create(&path).unwrap();
    f.set_len(16 * 1024 * 1024 + 1).unwrap();
    let (code, out, err) = dir.run(&["eval", "records", "big.json"]);
    assert_eq!(code, 3, "{out}{err}");
    assert!(out.is_empty());
    assert!(err.contains("16 MiB"), "{err}");
}

/// The committed records of the pilot (0.7.1, issue 70): each record of the 200
/// runs of the pilot has the 4 token counts. A record of another batch has
/// them or `null`.
#[test]
fn the_committed_pilot_records_have_token_counts() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/results/evals");
    let mut pilot = 0;
    for e in fs::read_dir(dir).unwrap() {
        let path = e.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if !name.ends_with(".jsonl") {
            continue;
        }
        let is_pilot = name.starts_with("20261117T0828") || name.starts_with("20261117T0829");
        for line in fs::read_to_string(&path).unwrap().lines() {
            let rest = line.split("\"tokens\":").nth(1).expect(&name);
            if rest.starts_with("null") {
                assert!(!is_pilot, "{name} has a record with null tokens");
                continue;
            }
            assert!(
                rest.starts_with("{\"in\":"),
                "{name}: the tokens of a record are null or an object"
            );
            pilot += usize::from(is_pilot);
        }
    }
    assert_eq!(pilot, 200, "the 5 files of the pilot hold 200 records");
}

// D58: the records have 12 keys (`harness`, `turns` and `seconds` follow the 9
// keys), and a result of `eval run --arms` gives the records of the batch.

/// The golden line of a record of `small-080.json`: the line of `small.json`
/// and the 3 new keys.
fn line12(eval: &str, run: u32, arm: &str, cost: &str, first: bool, i: usize) -> String {
    let old = line(eval, run, arm, cost, first);
    let turns = if i % 3 == 2 {
        "null".to_string()
    } else {
        (2 + i).to_string()
    };
    let seconds = 10.5 + 1.5 * i as f64;
    format!(
        "{},\"harness\":\"lean\",\"turns\":{turns},\"seconds\":{seconds}}}",
        &old[..old.len() - 1]
    )
}

fn golden12() -> String {
    let mut lines = Vec::new();
    let mut i = 0;
    for (eval, run, with_cost) in [
        ("tdd-1", 1, "0.5"),
        ("tdd-1", 2, "0.25"),
        ("tdd-2", 1, "0.75"),
        ("tdd-2", 2, "1"),
    ] {
        let first = eval == "tdd-1" && run == 1;
        lines.push(line12(eval, run, "with", with_cost, first, i));
        lines.push(line12(eval, run, "without", "0.5", false, i + 1));
        i += 2;
    }
    lines.join("\n") + "\n"
}

#[test]
fn a_result_of_0_8_0_gives_records_of_12_keys() {
    let path = fixture("small-080.json");
    let (code, out, err) = records(&[path.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out, golden12());
    assert_eq!(err, "eval records: 8 records of 1 names (tdd)\n");
    // The first line has the 12 keys in their order.
    let first = out.lines().next().unwrap();
    let mut at = 0;
    for k in [
        "skill",
        "eval",
        "run",
        "arm",
        "pass",
        "cost",
        "tokens",
        "models",
        "claude_code",
        "harness",
        "turns",
        "seconds",
    ] {
        let p = first[at..]
            .find(&format!("\"{k}\":"))
            .unwrap_or_else(|| panic!("no key {k} in order in {first}"));
        at += p;
    }
}

#[test]
fn a_record_of_9_keys_still_imports_and_stays_9_keys() {
    // `small.json` is a result of 0.7.0, with no `harness`, no `turns` and no
    // `seconds`. Its records stay as they were.
    let path = fixture("small.json");
    let (code, out, err) = records(&[path.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out, golden());
    assert!(!out.contains("\"harness\""), "{out}");
}

#[test]
fn a_side_of_0_7_1_in_a_result_of_0_8_0_reads_as_null_turns_and_null_seconds() {
    let text = fs::read_to_string(fixture("small-080.json")).unwrap();
    // The first side loses its turns and its seconds, as a side of a file of
    // 0.7.1 that `--resume` kept.
    let turns = text
        .find("\"turns\": 2,")
        .expect("the turns of the first side");
    let comma = text[..turns].rfind(',').unwrap();
    let seconds = "\"seconds\": 10.5";
    let end = text.find(seconds).expect("the seconds of the first side") + seconds.len();
    let stripped = format!("{}{}", &text[..comma], &text[end..]);
    let (code, out, err) = records_of(&stripped);
    assert_eq!(code, 0, "{out}{err}");
    let first = out.lines().next().unwrap();
    assert!(
        first.ends_with(",\"harness\":\"lean\",\"turns\":null,\"seconds\":null}"),
        "{first}"
    );
    // The other lines keep their values.
    assert_eq!(out.lines().nth(1), golden12().lines().nth(1));
}

#[test]
fn a_negative_seconds_stops_the_import() {
    let text = fs::read_to_string(fixture("small-080.json")).unwrap();
    for bad in ["-1.0", "-0.1", "\"12\"", "1e999", "true", "100000000"] {
        let (code, out, err) =
            records_of(&text.replacen("\"seconds\": 10.5", &format!("\"seconds\": {bad}"), 1));
        assert_eq!(code, 3, "{bad}: {out}{err}");
        assert!(err.contains("`seconds`"), "{bad}: {err}");
        assert_eq!(out, "", "nothing is printed: {bad}");
        assert!(
            !err.contains(bad.trim_matches('"')) || bad.len() < 3,
            "the message names no value: {err}"
        );
    }
}

#[test]
fn turns_and_harness_outside_their_form_stop_the_import() {
    let text = fs::read_to_string(fixture("small-080.json")).unwrap();
    for (from, to, key) in [
        ("\"turns\": 2", "\"turns\": -1", "turns"),
        ("\"turns\": 2", "\"turns\": 1.5", "turns"),
        ("\"turns\": 2", "\"turns\": \"2\"", "turns"),
        ("\"turns\": 2", "\"turns\": 100000000", "turns"),
        ("\"harness\": \"lean\"", "\"harness\": \"evil\"", "harness"),
        (
            "\"harness\": \"lean\"",
            "\"harness\": \"/home/x\"",
            "harness",
        ),
        ("\"harness\": \"lean\"", "\"harness\": 5", "harness"),
    ] {
        let (code, out, err) = records_of(&text.replacen(from, to, 1));
        assert_eq!(code, 3, "{to}: {out}{err}");
        assert!(err.contains(&format!("`{key}`")), "{to}: {err}");
        assert_eq!(out, "");
    }
}

#[test]
fn a_summary_below_the_floor_must_say_so() {
    let text = fs::read_to_string(fixture("small-080.json")).unwrap();
    // `below_floor` false with 4 pairs does not repeat from the runs.
    let (code, out, err) =
        records_of(&text.replace("\"below_floor\": true", "\"below_floor\": false"));
    assert_eq!(code, 3, "{out}{err}");
    assert!(
        err.contains("does not repeat") && err.contains("(below_floor)"),
        "{err}"
    );
    // A verdict `on` below the floor does not repeat either.
    let (code, out, err) =
        records_of(&text.replace("\"verdict\": \"inconclusive\"", "\"verdict\": \"on\""));
    assert_eq!(code, 3, "{out}{err}");
    assert!(
        err.contains("does not repeat") && err.contains("(verdict)"),
        "{err}"
    );
}

/// The golden lines of `arms.json`: 2 evals, 2 runs, 3 arms.
fn golden_arms() -> String {
    let arms = ["none", "lean-no-advisor", "lean"];
    let mut lines = Vec::new();
    let mut n = 0;
    for eval in ["team-01", "team-02"] {
        for run in 1..=2 {
            for arm in arms {
                n += 1;
                let cost = if n == 5 {
                    "null".to_string()
                } else {
                    format!("{}", (0.25 * n as f64 * 100.0).round() / 100.0)
                };
                let tokens = if n == 4 {
                    "null".to_string()
                } else {
                    "{\"in\":100,\"out\":200,\"cache_read\":300,\"cache_write\":400}".to_string()
                };
                let models = if arm == "lean" {
                    "[\"claude-a\",\"claude-b\"]"
                } else {
                    "[\"claude-a\"]"
                };
                let turns = if n == 7 {
                    "null".to_string()
                } else {
                    (n + 1).to_string()
                };
                let seconds = (n as f64 * 3.5 * 10.0).round() / 10.0;
                lines.push(format!(
                    "{{\"skill\":\"harness\",\"eval\":\"{eval}\",\"run\":{run},\"arm\":\"{arm}\",\"pass\":{},\"cost\":{cost},\"tokens\":{tokens},\"models\":{models},\"claude_code\":\"2.1.330\",\"harness\":\"{arm}\",\"turns\":{turns},\"seconds\":{seconds}}}",
                    n % 2 == 0
                ));
            }
        }
    }
    lines.join("\n") + "\n"
}

#[test]
fn a_result_of_arms_gives_one_record_for_each_run_of_each_arm() {
    let path = fixture("arms.json");
    let (code, out, err) = records(&[path.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out, golden_arms());
    assert_eq!(err, "eval records: 12 records of 1 names (harness)\n");
}

#[test]
fn a_side_of_arms_that_did_not_run_has_no_record() {
    let text = r#"{"arms":["none","lean"],"claude_code":"2.1.330","models":["claude-a"],"evals":[{"id":"team-01","runs":1,"max_budget_usd":1,"results":[{"run":1,"arms":{"none":{"passed":true,"cost_usd":0.5,"models":["claude-a"],"tokens":null,"turns":1,"seconds":1.5},"lean":null}}]}]}"#;
    let (code, out, err) = records_of(text);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        out,
        "{\"skill\":\"harness\",\"eval\":\"team-01\",\"run\":1,\"arm\":\"none\",\"pass\":true,\"cost\":0.5,\"tokens\":null,\"models\":[\"claude-a\"],\"claude_code\":\"2.1.330\",\"harness\":\"none\",\"turns\":1,\"seconds\":1.5}\n"
    );
    // No run at all: nothing to import.
    let (code, out, err) = records_of(
        &text
            .replace("\"passed\":true", "\"passed\":null")
            .replace("\"none\":{", "\"none\":null,\"x\":{"),
    );
    assert_eq!(code, 3, "{out}{err}");
}

#[test]
fn an_arm_outside_the_names_or_a_negative_seconds_stops_the_import_of_arms() {
    let text = fs::read_to_string(fixture("arms.json")).unwrap();
    let (code, out, err) = records_of(&text.replace(
        "\"arms\": [\n    \"none\",\n    \"lean-no-advisor\",\n    \"lean\"\n  ]",
        "\"arms\": [\n    \"none\",\n    \"nonsense\",\n    \"lean\"\n  ]",
    ));
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("`arms`"), "{err}");
    let (code, out, err) = records_of(&text.replace("\"seconds\": 3.5", "\"seconds\": -3.5"));
    assert_eq!(code, 3, "{out}{err}");
    assert!(err.contains("`seconds`"), "{err}");
    assert_eq!(out, "");
}
