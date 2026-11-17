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
    for bad in ["a/b".to_string(), "a".repeat(65), "x y".to_string()] {
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
    text.replace_range(at..at + 10, "\"a/b\"");
    let (code, _, err) = records_of(&text);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("models") && !err.contains("a/b"), "{err}");
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
