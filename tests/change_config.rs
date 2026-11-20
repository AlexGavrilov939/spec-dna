//! Integration tests of the keys of `[change]` that step 3 of 0.6.0 adds
//! (slice S2): `gate`, `high-risk-layers`, `high-risk-paths` and
//! `max-fanout`, and of the field `change_base` of `check --json`.
//!
//! The binary has no library. Thus the matrix of SD910 (the gate, the profile
//! and `[rules]`) and the profile rows of the new codes are unit tests in
//! `src/config.rs`, and this file tests what the binary shows: a valid value
//! passes, an invalid value is exit code 3 with the key in the message, and
//! the lifecycle base is in the JSON text.

mod common;

use common::*;

const TERMS: &str = "# Terms\n\n| Term | Meaning |\n| --- | --- |\n| Valve | A device that stops a flow of water. |\n";

/// A repository with one root `docs`, a `terms.md` and 1 commit. `head` is the
/// text before the roots table, `tail` the text after it.
fn repo(head: &str, tail: &str) -> Repo {
    let r = Repo::with_config(&format!("{head}\n[[roots]]\npath = \"docs\"\n\n{tail}"));
    r.write("docs/terms.md", TERMS);
    r.commit("start");
    r
}

fn rejects(tail: &str, needles: &[&str]) {
    let r = repo("", tail);
    let (code, _, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 3, "{tail}\n{err}");
    for n in needles {
        assert!(
            err.contains(n),
            "`{n}` is not in the message of\n{tail}\n{err}"
        );
    }
}

fn accepts(tail: &str) {
    let r = repo("", tail);
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{tail}\n{out}\n{err}");
}

fn table(keys: &str) -> String {
    format!("[change]\npolicy = \"three-class\"\n{keys}\n")
}

#[test]
fn the_gate_takes_block_warn_and_off() {
    for g in ["block", "warn", "off"] {
        accepts(&table(&format!("gate = \"{g}\"")));
    }
    for bad in ["\"error\"", "\"\"", "\"Block\"", "\"none\""] {
        rejects(
            &table(&format!("gate = {bad}")),
            &["`change.gate` must be", "\"block\"", "\"warn\"", "\"off\""],
        );
    }
    rejects(&table("gate = true"), &["`change.gate` must be a string"]);
}

#[test]
fn max_fanout_takes_a_non_negative_integer() {
    accepts(&table("max-fanout = 8"));
    accepts(&table("max-fanout = 1"));
    for bad in ["-1", "\"8\"", "1.5", "true"] {
        rejects(
            &table(&format!("max-fanout = {bad}")),
            &["`change.max-fanout` must be a non-negative integer"],
        );
    }
}

#[test]
fn high_risk_paths_take_globs_from_the_root() {
    accepts(&table("high-risk-paths = []"));
    accepts(&table("high-risk-paths = [\"src/api/**\", \"db/*.sql\"]"));
    for (bad, want) in [
        ("[\"\"]", "is empty"),
        ("[\"/etc/**\"]", "relative path"),
        ("[\"../up/**\"]", "`.` or `..` segment"),
        ("[3]", "must hold strings"),
        ("\"src\"", "must be an array of strings"),
    ] {
        rejects(
            &table(&format!("high-risk-paths = {bad}")),
            &["`change.high-risk-paths`", want],
        );
    }
}

#[test]
fn shared_paths_take_globs_from_the_root() {
    accepts(&table("shared-paths = []"));
    accepts(&table(
        "shared-paths = [\"docs/roadmap.md\", \"docs/questions.md\"]",
    ));
    for (bad, want) in [
        ("[\"\"]", "is empty"),
        ("[\"/etc/**\"]", "relative path"),
        ("[\"../up/**\"]", "`.` or `..` segment"),
        ("[\"a.md\", \"a.md\"]", "more than once"),
        ("[3]", "must hold strings"),
        ("\"docs\"", "must be an array of strings"),
    ] {
        rejects(
            &table(&format!("shared-paths = {bad}")),
            &["`change.shared-paths`", want],
        );
    }
}

#[test]
fn high_risk_layers_name_layers_of_the_graph() {
    // The graph of a project with no `[layers]` table has `rules`.
    accepts(&table("high-risk-layers = [\"rules\"]"));
    accepts(&table("high-risk-layers = []"));
    rejects(
        &table("high-risk-layers = [\"nope\"]"),
        &[
            "`change.high-risk-layers`",
            "`nope` is not a layer of the graph",
        ],
    );
    rejects(
        &table("high-risk-layers = [\"rules\", \"rules\"]"),
        &["lists `rules` more than once"],
    );
    rejects(
        &table("high-risk-layers = \"rules\""),
        &["must be an array of strings"],
    );
    // A project that has its own layers cannot name a layer that it lacks, not
    // even a name of the default.
    rejects(
        &format!(
            "{}[layers.mine]\ntypes = [\"rule\"]\n",
            table("high-risk-layers = [\"interfaces\"]")
        ),
        &["`interfaces` is not a layer of the graph", "mine"],
    );
}

#[test]
fn base_must_be_a_branch_name() {
    // A good name of a local branch.
    for ok in ["main", "master", "release/1.0", "feat-x", "v1.2"] {
        accepts(&table(&format!("base = \"{ok}\"")));
    }
    // A bad name is exit code 3. The message has the key path and the reason.
    // The reasons are text that only the check of the name prints.
    for (bad, why) in [
        ("", "is empty"),
        ("a b", "has a space"),
        ("a..b", "has `..`"),
        ("a~b", "has the character `~`"),
        ("a^b", "has the character `^`"),
        ("a:b", "has the character `:`"),
        ("a?b", "has the character `?`"),
        ("a*b", "has the character `*`"),
        ("a[b", "has the character `[`"),
        ("a\\\\b", "has the character `\\`"),
        ("-main", "starts with `-` or `/`"),
        ("/main", "starts with `-` or `/`"),
        ("main/", "ends with `/` or `.lock`"),
        ("main.lock", "ends with `/` or `.lock`"),
    ] {
        rejects(
            &table(&format!("base = \"{bad}\"")),
            &["`change.base`", why, "name of a local branch"],
        );
    }
    for bad in ["1", "true", "[\"main\"]"] {
        rejects(
            &table(&format!("base = {bad}")),
            &["`change.base` must be a string"],
        );
    }
}

#[test]
fn an_invented_key_is_unknown_and_policy_stays_required() {
    rejects(&table("invented = 1"), &["unknown key `change.invented`"]);
    rejects(
        "[change]\ngate = \"block\"\n",
        &["`change.policy` is required"],
    );
}

#[test]
fn a_rule_for_sd910_and_the_group_changes_are_valid() {
    for rules in [
        "[rules]\nSD910 = \"off\"\n",
        "[rules]\nchanges = \"warning\"\nSD910 = \"error\"\n",
    ] {
        let r = repo(rules, &table("gate = \"warn\""));
        let (code, out, err) = r.run(&["check", "--all"]);
        assert_eq!(code, 0, "{rules}\n{out}\n{err}");
    }
}

#[test]
fn check_json_has_no_change_base_without_a_policy() {
    for tail in ["", "[change]\npolicy = \"none\"\n"] {
        let r = repo("", tail);
        let sha = r.git_out(&["rev-parse", "HEAD"]);
        origin_head(&r.dir, &sha);
        let (code, out, err) = r.run(&["check", "--all", "--json"]);
        assert_eq!(code, 0, "{out}\n{err}");
        assert!(!out.contains("change_base"), "{out}");
        // No note about the base either.
        assert!(!err.contains("no base for the change checks"), "{err}");
    }
}

#[test]
fn change_base_is_null_without_origin_head_and_prints_the_note() {
    let r = repo("", &table(""));
    let (code, out, err) = r.run(&["check", "--all", "--json"]);
    assert_eq!(code, 0, "{out}\n{err}");
    assert!(out.contains("\"change_base\":null"), "{out}");
    // The field comes after `base`.
    assert!(
        out.find("\"base\"").unwrap() < out.find("\"change_base\"").unwrap(),
        "{out}"
    );
    // The JSON output has no note. The text output has it on stderr.
    assert!(!err.contains("no base for the change checks"), "{err}");
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}\n{err}");
    assert!(
        err.contains(
            "no base for the change checks; pass --base <rev>, or run git remote set-head origin <branch>, or set [change] base"
        ),
        "{err}"
    );
    assert!(!out.contains("no base for the change checks"), "{out}");
}

#[test]
fn change_base_is_the_merge_base_of_origin_head_and_head() {
    let r = repo("", &table(""));
    let first = r.git_out(&["rev-parse", "HEAD"]);
    // `main` moves on. `origin/HEAD` stays at `main`, and the branch of the
    // work leaves `first`.
    r.write("docs/more.md", "---\ntype: research\n---\n# More\n");
    r.commit("main moves on");
    let tip = r.git_out(&["rev-parse", "HEAD"]);
    r.git(&["checkout", "-q", "-b", "work", &first]);
    r.write("docs/work.md", "---\ntype: research\n---\n# Work\n");
    r.commit("work");
    origin_head(&r.dir, &tip);
    let (code, out, err) = r.run(&["check", "--all", "--json"]);
    assert_eq!(code, 0, "{out}\n{err}");
    // The merge base, not the tip of `origin/HEAD`.
    assert!(
        out.contains(&format!("\"change_base\":\"{first}\"")),
        "{out}"
    );
    assert!(
        !out.contains(&format!("\"change_base\":\"{tip}\"")),
        "{out}"
    );
    // `--base` replaces `origin/HEAD`.
    let (code, out, err) = r.run(&["check", "--base", &first, "--json"]);
    assert_eq!(code, 0, "{out}\n{err}");
    assert!(
        out.contains(&format!("\"change_base\":\"{first}\"")),
        "{out}"
    );
    // The text output has no note when the base is known.
    let (_, _, err) = r.run(&["check", "--all"]);
    assert!(!err.contains("no base for the change checks"), "{err}");
}

#[test]
fn a_bad_base_is_still_exit_code_3() {
    let r = repo("", &table(""));
    let (code, _, err) = r.run(&["check", "--base", "nope", "--json"]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("`nope` is not a commit"), "{err}");
}

// The keys `approval` and `approvers` (design 0.7.0, D14 and D15). The binary
// reads them in all cases, and the tests here check what it accepts and what
// it rejects. The rule of SD910 that uses them has its own tests.

#[test]
fn approval_takes_base_and_branch() {
    accepts(&table("approval = \"base\""));
    accepts(&table(
        "approval = \"branch\"\napprovers = [\"owner@example.test\"]",
    ));
    for bad in ["\"Branch\"", "\"\"", "\"none\"", "\"base \""] {
        rejects(
            &table(&format!("approval = {bad}")),
            &["`change.approval` must be", "\"base\"", "\"branch\""],
        );
    }
    for bad in ["true", "1", "[\"branch\"]"] {
        rejects(
            &table(&format!("approval = {bad}")),
            &["`change.approval` must be a string"],
        );
    }
}

#[test]
fn approvers_are_a_list_of_e_mails() {
    // With `approval = "base"` the list is free, and it can be empty or absent.
    accepts(&table("approvers = []"));
    accepts(&table(
        "approval = \"base\"\napprovers = [\"a@example.test\"]",
    ));
    accepts(&table(
        "approval = \"branch\"\napprovers = [\"a@example.test\", \"12+b@users.noreply.example.test\"]",
    ));
    rejects(
        &table("approvers = \"a@example.test\""),
        &["`change.approvers` must be an array of strings"],
    );
    rejects(
        &table("approvers = [1]"),
        &["`change.approvers` must hold strings"],
    );
    rejects(
        &table("approvers = [\"a@example.test\", \"  \"]"),
        &["`change.approvers`", "is empty"],
    );
}

#[test]
fn branch_with_no_approver_is_exit_code_3() {
    for keys in [
        "approval = \"branch\"",
        "approval = \"branch\"\napprovers = []",
    ] {
        rejects(
            &table(keys),
            &[
                "`change.approvers`",
                "`change.approval = \"branch\"`",
                "at least one e-mail",
            ],
        );
    }
}

#[test]
fn strict_with_branch_is_exit_code_3() {
    let keys = "approval = \"branch\"\napprovers = [\"owner@example.test\"]";
    let r = repo("profile = \"strict\"\n", &table(keys));
    let (code, _, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 3, "{err}");
    for n in ["`change.approval = \"branch\"`", "`strict`", "base"] {
        assert!(err.contains(n), "`{n}` is not in\n{err}");
    }
    // `strict` with `base`, and the other profiles with `branch`, are valid.
    let r = repo("profile = \"strict\"\n", &table("approval = \"base\""));
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}\n{err}");
    for p in ["lite", "standard"] {
        let r = repo(&format!("profile = \"{p}\"\n"), &table(keys));
        let (code, out, err) = r.run(&["check", "--all"]);
        assert_eq!(code, 0, "{p}\n{out}\n{err}");
    }
}

#[test]
fn an_approver_that_is_no_e_mail_is_exit_code_3() {
    // A wildcard matches each author, so the condition would be empty (ADR
    // 0012). An item with no `@` is no e-mail.
    for item in ["*", "owner", "*@example.test", "owner@*"] {
        rejects(
            &table(&format!("approval = \"branch\"\napprovers = [\"{item}\"]")),
            &["change.approvers", item, "e-mail"],
        );
    }
    accepts(&table(
        "approval = \"branch\"\napprovers = [\"owner@example.test\"]",
    ));
}

// The keys of `[evidence]` for the proof levels (design 0.8.0, D2)

fn evidence(keys: &str) -> String {
    format!("[evidence]\n{keys}\n")
}

#[test]
fn reliable_for_without_reports_is_exit_code_3_with_the_fix_in_the_message() {
    let message = "[evidence] reliable-for needs [evidence] reports: set reports, or set reliable-for = \"none\"";
    for keys in [
        "reliable-for = \"higher-risk\"",
        "reliable-for = \"all\"\nreports = []",
        "reliable-for = \"all\"\nreliable-runs = 4",
    ] {
        let r = repo("", &evidence(keys));
        for args in [&["check"][..], &["check", "--all"][..], &["status"][..]] {
            let (code, out, err) = r.run(args);
            assert_eq!(code, 3, "{keys} {args:?}\n{out}{err}");
            assert!(err.contains(message), "{keys}\n{err}");
        }
    }
    // `none` needs no report, and a report list makes the other values valid.
    accepts(&evidence("reliable-for = \"none\""));
    accepts(&evidence(
        "reports = [\"out/*.xml\"]\nreliable-for = \"higher-risk\"",
    ));
    accepts(&evidence(
        "reports = [\"out/*.xml\"]\nreliable-for = \"all\"\nreliable-runs = 20",
    ));
}

#[test]
fn reliable_runs_is_an_integer_from_2_to_20_and_reliable_for_takes_3_values() {
    for bad in ["1", "0", "21", "-1", "\"3\"", "true"] {
        rejects(
            &evidence(&format!("reliable-runs = {bad}")),
            &["evidence.reliable-runs"],
        );
    }
    accepts(&evidence("reliable-runs = 2"));
    for bad in ["\"sometimes\"", "\"All\""] {
        rejects(
            &evidence(&format!("reports = [\"o/*\"]\nreliable-for = {bad}")),
            &[
                "evidence.reliable-for",
                "\"none\"",
                "\"higher-risk\"",
                "\"all\"",
            ],
        );
    }
    for bad in ["true", "1"] {
        rejects(
            &evidence(&format!("reports = [\"o/*\"]\nreliable-for = {bad}")),
            &["evidence.reliable-for"],
        );
    }
}
