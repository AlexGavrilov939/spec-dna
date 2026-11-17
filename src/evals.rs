//! The eval file and its validator (ADR 0008, `docs/commands.md`, section
//! `eval`). An eval is the file `<dir>/<id>.json`, where `<dir>` is
//! `[evals] dir`. `parse_eval` is the one validator. `eval run` uses it, and
//! so does the check SD506. It reads no file: the caller gives the text, the
//! file stem, the `[evals]` table and a lookup for the fixture, so the same
//! function can read a working tree or a commit.

use crate::config::Evals;
use crate::harness::Skill;
use crate::json::{self, Value};
use crate::verdict::AGENTS_LINE;

/// The most bytes of an eval file.
pub const MAX_BYTES: usize = 64 * 1024;
/// The most bytes of the task of an eval.
pub const MAX_TASK_BYTES: usize = 8_000;
/// The default of `max-turns`.
pub const DEFAULT_MAX_TURNS: u32 = 20;
/// The folder of result files, below `[evals] dir`. It holds no eval.
pub const RESULTS: &str = "results";
/// The name of the file of the check functions, in `[evals] dir`.
pub const CHECK_FILE: &str = "check.sh";

/// The keys of an eval file, in the order of the validator.
const KEYS: [&str; 11] = [
    "id",
    "task",
    "fixture",
    "branch",
    "allowed-tools",
    "max-turns",
    "max-budget-usd",
    "runs",
    "pass-at",
    "skill",
    "checks",
];

/// A check of an eval: a name and a shell command.
#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub name: String,
    pub run: String,
}

/// A valid eval, with the defaults filled in.
#[derive(Clone, Debug, PartialEq)]
pub struct Eval {
    pub id: String,
    pub task: String,
    /// The folder of the fixture, from the repository root.
    pub fixture: String,
    pub branch: Option<String>,
    pub allowed_tools: Vec<String>,
    pub max_turns: u32,
    pub max_budget_usd: f64,
    pub runs: u32,
    pub pass_at: u32,
    /// The skill of the pack that the eval measures (`--ab`), or the group
    /// `agents-line`, if the file names one.
    pub skill: Option<String>,
    pub checks: Vec<Check>,
}

impl Eval {
    /// The branch that the runner switches to, or `None` for no branch and
    /// for `main`, the base branch.
    pub fn switch_branch(&self) -> Option<&str> {
        self.branch.as_deref().filter(|b| *b != "main")
    }
}

/// The first rule that an eval file breaks.
#[derive(Clone, Debug, PartialEq)]
pub struct EvalError {
    /// The key of the rule, or `file` for a rule of the whole file.
    pub key: String,
    /// The message. It names the key in backticks.
    pub message: String,
}

fn err<T>(key: &str, message: impl Into<String>) -> Result<T, EvalError> {
    Err(EvalError {
        key: key.to_string(),
        message: message.into(),
    })
}

/// True for `[a-z0-9][a-z0-9-]{0,63}`.
fn is_name(s: &str) -> bool {
    let b = s.as_bytes();
    (1..=64).contains(&b.len())
        && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit())
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
}

/// True for `[A-Za-z0-9][A-Za-z0-9._/-]{0,99}` with no `..` or `//`, and
/// no final `/` or `.lock`.
fn is_branch(s: &str) -> bool {
    let b = s.as_bytes();
    (1..=100).contains(&b.len())
        && b[0].is_ascii_alphanumeric()
        && b.iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'/' | b'-'))
        && !s.contains("..")
        && !s.contains("//")
        && !s.ends_with('/')
        && !s.ends_with(".lock")
}

/// An integer in `lo..=hi`. A JSON number with a fraction is not an integer.
fn int(v: &Value, lo: u32, hi: u32) -> Option<u32> {
    match v {
        Value::Num(n) if n.is_finite() && n.fract() == 0.0 && *n >= f64::from(lo) => {
            (*n <= f64::from(hi)).then_some(*n as u32)
        }
        _ => None,
    }
}

fn field<'a>(fields: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Checks the text of the file `<dir>/<stem>.json`, and returns the eval.
/// `has_file_under(path)` is true if the tree has at least one file under the
/// folder `path`. The rules, in this order: the whole file, then each key in
/// the order of the table of `docs/commands.md`.
pub fn parse_eval(
    stem: &str,
    bytes: &[u8],
    evals: &Evals,
    has_file_under: &dyn Fn(&str) -> bool,
) -> Result<Eval, EvalError> {
    if bytes.len() > MAX_BYTES {
        return err("file", "the file has more than 64 KiB");
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return err("file", "the file is not UTF-8");
    };
    let value = match json::parse(text) {
        Ok(v) => v,
        Err(e) => return err("file", format!("the file is not valid JSON ({e})")),
    };
    let Value::Obj(fields) = &value else {
        return err("file", "the file is not one JSON object");
    };
    for (i, (k, _)) in fields.iter().enumerate() {
        if !KEYS.contains(&k.as_str()) {
            return err(k, format!("unknown key `{k}`"));
        }
        if fields[..i].iter().any(|(p, _)| p == k) {
            return err(k, format!("the key `{k}` is in the file more than once"));
        }
    }

    let id = match field(fields, "id") {
        None => return err("id", "`id` is required"),
        Some(Value::Str(s)) if is_name(s) => s.clone(),
        Some(Value::Str(_)) => {
            return err(
                "id",
                "`id` must have 1 to 64 characters of a-z, 0-9 and `-`",
            );
        }
        Some(_) => return err("id", "`id` must be a string"),
    };
    if id != stem {
        return err(
            "id",
            format!("`id` must be the name of the file without `.json` (`{stem}`)"),
        );
    }

    let task = match field(fields, "task") {
        None => return err("task", "`task` is required"),
        Some(Value::Str(s)) if s.is_empty() => return err("task", "`task` is empty"),
        Some(Value::Str(s)) if s.len() > MAX_TASK_BYTES => {
            return err("task", "`task` has more than 8,000 bytes");
        }
        Some(Value::Str(s)) if s.starts_with('-') => {
            return err("task", "`task` must not start with `-`");
        }
        Some(Value::Str(s)) => s.clone(),
        Some(_) => return err("task", "`task` must be a string"),
    };

    let fixture = match field(fields, "fixture") {
        None => return err("fixture", "`fixture` is required"),
        Some(Value::Str(s)) => s.clone(),
        Some(_) => return err("fixture", "`fixture` must be a string"),
    };
    check_fixture(&fixture, evals, has_file_under)?;

    let branch = match field(fields, "branch") {
        None => None,
        Some(Value::Str(s)) if is_branch(s) => Some(s.clone()),
        Some(Value::Str(_)) => {
            return err(
                "branch",
                "`branch` must be a git branch name of at most 100 characters of A-Z, a-z, 0-9, `.`, `_`, `/` and `-`; it starts with a letter or a digit, has no `..` or `//`, and does not end with `/` or `.lock`",
            );
        }
        Some(_) => return err("branch", "`branch` must be a string"),
    };

    let allowed_tools = match field(fields, "allowed-tools") {
        None => Vec::new(),
        Some(Value::Arr(items)) => {
            let mut tools = Vec::new();
            for item in items {
                match item {
                    Value::Str(s)
                        if !s.is_empty()
                            && !s.starts_with('-')
                            && !s.chars().any(char::is_control) =>
                    {
                        tools.push(s.clone());
                    }
                    _ => {
                        return err(
                            "allowed-tools",
                            "`allowed-tools` must be a list of strings with no empty string, no string that starts with `-` and no control character",
                        );
                    }
                }
            }
            tools
        }
        Some(_) => return err("allowed-tools", "`allowed-tools` must be a list of strings"),
    };

    let max_turns = match field(fields, "max-turns") {
        None => DEFAULT_MAX_TURNS,
        Some(v) => match int(v, 1, 1_000) {
            Some(n) => n,
            None => {
                return err(
                    "max-turns",
                    "`max-turns` must be an integer from 1 to 1,000",
                );
            }
        },
    };

    let max_budget_usd = match field(fields, "max-budget-usd") {
        None => return err("max-budget-usd", "`max-budget-usd` is required"),
        Some(Value::Num(n)) if n.is_finite() && *n > 0.0 => {
            if *n > evals.max_total_usd {
                return err(
                    "max-budget-usd",
                    format!(
                        "`max-budget-usd` is above `max-total-usd` of [evals] ({})",
                        evals.max_total_usd
                    ),
                );
            }
            *n
        }
        Some(_) => {
            return err(
                "max-budget-usd",
                "`max-budget-usd` must be a number above 0",
            );
        }
    };

    let runs = match field(fields, "runs") {
        None => evals.runs,
        Some(v) => match int(v, 1, 100) {
            Some(n) => n,
            None => return err("runs", "`runs` must be an integer from 1 to 100"),
        },
    };

    let pass_at = match field(fields, "pass-at") {
        None => runs,
        Some(v) => match int(v, 1, runs) {
            Some(n) => n,
            None => {
                return err(
                    "pass-at",
                    format!("`pass-at` must be an integer from 1 to the runs of the eval ({runs})"),
                );
            }
        },
    };

    let skill = match field(fields, "skill") {
        None => None,
        Some(Value::Str(s)) if Skill::from_key(s).is_some() || s == AGENTS_LINE => Some(s.clone()),
        Some(Value::Str(_)) => {
            return err(
                "skill",
                format!(
                    "`skill` must name a skill of the pack or `{AGENTS_LINE}`; use {}, or `{AGENTS_LINE}`",
                    crate::harness::skill_keys()
                ),
            );
        }
        Some(_) => return err("skill", "`skill` must be a string"),
    };

    let checks = match field(fields, "checks") {
        None => return err("checks", "`checks` is required"),
        Some(Value::Arr(items)) if items.is_empty() => {
            return err("checks", "`checks` has no check");
        }
        Some(Value::Arr(items)) => parse_checks(items)?,
        Some(_) => return err("checks", "`checks` must be a list of objects"),
    };

    Ok(Eval {
        id,
        task,
        fixture,
        branch,
        allowed_tools,
        max_turns,
        max_budget_usd,
        runs,
        pass_at,
        skill,
        checks,
    })
}

fn check_fixture(
    fixture: &str,
    evals: &Evals,
    has_file_under: &dyn Fn(&str) -> bool,
) -> Result<(), EvalError> {
    let under = format!("{}/", evals.dir);
    let Some(rest) = fixture.strip_prefix(&under).filter(|r| !r.is_empty()) else {
        return err(
            "fixture",
            format!("`fixture` must be a folder under `{under}`"),
        );
    };
    if fixture.contains('\\')
        || fixture.chars().any(char::is_control)
        || fixture
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
    {
        return err(
            "fixture",
            "`fixture` must use `/` separators, with no `\\`, no empty segment and no `.` or `..` segment",
        );
    }
    if rest == RESULTS || rest.starts_with(&format!("{RESULTS}/")) {
        return err(
            "fixture",
            format!("`fixture` must not be under `{under}{RESULTS}/`"),
        );
    }
    if !has_file_under(fixture) {
        return err(
            "fixture",
            format!("`fixture` has no file under `{fixture}`"),
        );
    }
    Ok(())
}

fn parse_checks(items: &[Value]) -> Result<Vec<Check>, EvalError> {
    let mut checks: Vec<Check> = Vec::new();
    for item in items {
        let Value::Obj(f) = item else {
            return err("checks", "each item of `checks` must be an object");
        };
        if f.len() != 2 || field(f, "name").is_none() || field(f, "run").is_none() {
            return err(
                "checks",
                "each item of `checks` must have exactly the keys `name` and `run`",
            );
        }
        let name = match field(f, "name") {
            Some(Value::Str(s)) if is_name(s) => s.clone(),
            _ => {
                return err(
                    "checks",
                    "the `name` of a check must have 1 to 64 characters of a-z, 0-9 and `-`",
                );
            }
        };
        if checks.iter().any(|c| c.name == name) {
            return err(
                "checks",
                format!("the check name `{name}` is in the file more than once"),
            );
        }
        let run = match field(f, "run") {
            Some(Value::Str(s)) if !s.is_empty() => s.clone(),
            _ => {
                return err(
                    "checks",
                    "the `run` of a check must be a string that is not empty",
                );
            }
        };
        checks.push(Check { name, run });
    }
    Ok(checks)
}

/// The paths of the starter files that `init` writes for the answer
/// `skeleton`, with the folder `dir` of `[evals]`.
pub fn skeleton_paths(dir: &str) -> [String; 4] {
    [
        format!("{dir}/{CHECK_FILE}"),
        format!("{dir}/example.json"),
        format!("{dir}/fixtures/example/notes.txt"),
        ".github/workflows/agent-evals.yml".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Evals {
        Evals {
            dir: "evals".into(),
            min_pass_rate: 0.8,
            runs: 3,
            max_total_usd: 20.0,
        }
    }

    const GOOD: &str = r#"{"id":"example","task":"Edit the file.","fixture":"evals/fixtures/example","max-budget-usd":1,"checks":[{"name":"done","run":"true"}]}"#;

    fn parse(text: &str) -> Result<Eval, EvalError> {
        parse_eval("example", text.as_bytes(), &table(), &|p| {
            p == "evals/fixtures/example" || p == "evals/fixtures/other"
        })
    }

    /// The text with `from` replaced by `to`.
    fn with(from: &str, to: &str) -> String {
        assert!(GOOD.contains(from), "{from}");
        GOOD.replace(from, to)
    }

    fn rejects(text: &str, key: &str, needle: &str) {
        let e = parse(text).expect_err(text);
        assert_eq!(e.key, key, "{text}: {}", e.message);
        assert!(e.message.contains(needle), "{}", e.message);
    }

    #[test]
    fn the_defaults_come_from_the_table() {
        let e = parse(GOOD).unwrap();
        assert_eq!(e.id, "example");
        assert_eq!(e.runs, 3);
        assert_eq!(e.pass_at, 3);
        assert_eq!(e.max_turns, 20);
        assert_eq!(e.branch, None);
        assert!(e.allowed_tools.is_empty());
        assert_eq!(e.max_budget_usd, 1.0);
        assert_eq!(e.checks.len(), 1);
    }

    #[test]
    fn each_key_is_read() {
        let text = r#"{"id":"example","task":"t","fixture":"evals/fixtures/other","branch":"fix/x","allowed-tools":["Read","Bash(cargo test*)"],"max-turns":7,"max-budget-usd":2.5,"runs":4,"pass-at":2,"checks":[{"name":"a","run":"x"},{"name":"b","run":"y"}]}"#;
        let e = parse(text).unwrap();
        assert_eq!(e.fixture, "evals/fixtures/other");
        assert_eq!(e.branch.as_deref(), Some("fix/x"));
        assert_eq!(e.switch_branch(), Some("fix/x"));
        assert_eq!(e.allowed_tools, ["Read", "Bash(cargo test*)"]);
        assert_eq!((e.max_turns, e.runs, e.pass_at), (7, 4, 2));
        assert_eq!(e.max_budget_usd, 2.5);
        assert_eq!(e.checks[1].name, "b");
    }

    #[test]
    fn the_key_skill_names_a_skill_of_the_pack() {
        assert_eq!(parse(GOOD).unwrap().skill, None);
        let e = parse(&with(
            "\"max-budget-usd\"",
            "\"skill\":\"write-plan\",\"max-budget-usd\"",
        ))
        .unwrap();
        assert_eq!(e.skill.as_deref(), Some("write-plan"));
        // The group `agents-line` is no skill, and its evals name it.
        let e = parse(&with(
            "\"max-budget-usd\"",
            "\"skill\":\"agents-line\",\"max-budget-usd\"",
        ))
        .unwrap();
        assert_eq!(e.skill.as_deref(), Some("agents-line"));
        for bad in [
            "\"intent\"",
            "\"spec-dna-tdd\"",
            "\"agents-line-1\"",
            "\"\"",
            "1",
            "[]",
        ] {
            let text = with(
                "\"max-budget-usd\"",
                &format!("\"skill\":{bad},\"max-budget-usd\""),
            );
            let e = parse(&text).expect_err(&text);
            assert_eq!(e.key, "skill", "{bad}");
        }
    }

    #[test]
    fn main_is_the_base_branch() {
        let e = parse(&with(
            "\"max-budget-usd\"",
            "\"branch\":\"main\",\"max-budget-usd\"",
        ))
        .unwrap();
        assert_eq!(e.switch_branch(), None);
    }

    #[test]
    fn the_whole_file_rules_come_first() {
        rejects(&"x".repeat(MAX_BYTES + 1), "file", "64 KiB");
        let e = parse_eval("example", &[0xff, 0xfe], &table(), &|_| true).unwrap_err();
        assert_eq!(e.message, "the file is not UTF-8");
        rejects("{", "file", "not valid JSON");
        rejects("[]", "file", "not one JSON object");
        rejects("\"x\"", "file", "not one JSON object");
        rejects(
            &with("\"task\"", "\"extra\":1,\"task\""),
            "extra",
            "unknown key `extra`",
        );
        rejects(
            &with("\"task\"", "\"id\":\"example\",\"task\""),
            "id",
            "more than once",
        );
        // An unknown key comes before a bad value.
        rejects(r#"{"id":5,"zzz":1}"#, "zzz", "unknown key `zzz`");
    }

    #[test]
    fn the_id_rules() {
        let no_id = GOOD.replace("\"id\":\"example\",", "");
        rejects(&no_id, "id", "required");
        rejects(&with("\"id\":\"example\"", "\"id\":5"), "id", "string");
        rejects(
            &with("\"id\":\"example\"", "\"id\":\"Example\""),
            "id",
            "a-z",
        );
        rejects(&with("\"id\":\"example\"", "\"id\":\"-a\""), "id", "a-z");
        rejects(&with("\"id\":\"example\"", "\"id\":\"\""), "id", "a-z");
        let long = "a".repeat(65);
        rejects(
            &with("\"id\":\"example\"", &format!("\"id\":\"{long}\"")),
            "id",
            "a-z",
        );
        rejects(
            &with("\"id\":\"example\"", "\"id\":\"other\""),
            "id",
            "(`example`)",
        );
        let ok = "a".repeat(64);
        let text = with("\"id\":\"example\"", &format!("\"id\":\"{ok}\""));
        assert!(parse_eval(&ok, text.as_bytes(), &table(), &|_| true).is_ok());
    }

    #[test]
    fn the_task_rules() {
        rejects(
            &GOOD.replace("\"task\":\"Edit the file.\",", ""),
            "task",
            "required",
        );
        rejects(&with("\"Edit the file.\"", "5"), "task", "string");
        rejects(&with("\"Edit the file.\"", "\"\""), "task", "empty");
        rejects(
            &with("\"Edit the file.\"", "\"-p x\""),
            "task",
            "start with `-`",
        );
        let long = "x".repeat(MAX_TASK_BYTES + 1);
        rejects(
            &with("\"Edit the file.\"", &format!("\"{long}\"")),
            "task",
            "8,000",
        );
        let max = "x".repeat(MAX_TASK_BYTES);
        assert!(parse(&with("\"Edit the file.\"", &format!("\"{max}\""))).is_ok());
    }

    #[test]
    fn the_fixture_rules() {
        let f = |v: &str| with("\"evals/fixtures/example\"", &format!("\"{v}\""));
        rejects(
            &GOOD.replace("\"fixture\":\"evals/fixtures/example\",", ""),
            "fixture",
            "required",
        );
        rejects(
            &with("\"evals/fixtures/example\"", "5"),
            "fixture",
            "string",
        );
        rejects(&f("fixtures/example"), "fixture", "under `evals/`");
        rejects(&f("evals"), "fixture", "under `evals/`");
        rejects(&f("evals/"), "fixture", "under `evals/`");
        rejects(&f("other/x"), "fixture", "under `evals/`");
        rejects(&f("evals/../x"), "fixture", "`..`");
        rejects(&f("evals/./x"), "fixture", "`.`");
        rejects(&f("evals//x"), "fixture", "empty segment");
        rejects(&f("evals/x/"), "fixture", "empty segment");
        rejects(&f("evals\\\\x"), "fixture", "under `evals/`");
        rejects(&f("evals/a\\\\b"), "fixture", "`\\`");
        rejects(&f("evals/results"), "fixture", "results");
        rejects(&f("evals/results/x"), "fixture", "results");
        // A name that starts with `results` is not the folder of the results.
        let e = parse_eval(
            "example",
            f("evals/results-old").as_bytes(),
            &table(),
            &|_| true,
        );
        assert!(e.is_ok(), "{e:?}");
        // No file under the folder.
        rejects(&f("evals/fixtures/none"), "fixture", "has no file");
    }

    #[test]
    fn the_fixture_is_under_a_dir_that_has_a_slash() {
        let mut t = table();
        t.dir = ".spec-dna/evals".into();
        let text = with("evals/fixtures/example", ".spec-dna/evals/fixtures/example");
        assert!(parse_eval("example", text.as_bytes(), &t, &|_| true).is_ok());
        let e = parse_eval("example", GOOD.as_bytes(), &t, &|_| true).unwrap_err();
        assert!(
            e.message.contains("under `.spec-dna/evals/`"),
            "{}",
            e.message
        );
    }

    #[test]
    fn the_branch_rules() {
        let b = |v: &str| {
            with(
                "\"max-budget-usd\"",
                &format!("\"branch\":\"{v}\",\"max-budget-usd\""),
            )
        };
        for ok in ["fix/x", "a", "A.b_c-d/e", "main", &"a".repeat(100)] {
            assert!(parse(&b(ok)).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "/x",
            "-x",
            ".x",
            "a..b",
            "a//b",
            "a/",
            "x.lock",
            "a b",
            "a~b",
            "a:b",
            "a@{b",
            &"a".repeat(101),
        ] {
            rejects(&b(bad), "branch", "branch name");
        }
        rejects(
            &with("\"max-budget-usd\"", "\"branch\":5,\"max-budget-usd\""),
            "branch",
            "string",
        );
    }

    #[test]
    fn the_allowed_tools_rules() {
        let t = |v: &str| {
            with(
                "\"max-budget-usd\"",
                &format!("\"allowed-tools\":{v},\"max-budget-usd\""),
            )
        };
        assert!(parse(&t("[]")).is_ok());
        for bad in ["[\"\"]", "[5]", "[\"a\\nb\"]", "[\"a\",null]"] {
            rejects(&t(bad), "allowed-tools", "strings");
        }
        rejects(&t("\"Read\""), "allowed-tools", "list");
    }

    #[test]
    fn the_integer_rules() {
        let k = |key: &str, v: &str| {
            with(
                "\"max-budget-usd\"",
                &format!("\"{key}\":{v},\"max-budget-usd\""),
            )
        };
        assert_eq!(parse(&k("max-turns", "1")).unwrap().max_turns, 1);
        assert_eq!(parse(&k("max-turns", "1000")).unwrap().max_turns, 1000);
        for bad in ["0", "1001", "1.5", "-1", "\"5\"", "null"] {
            rejects(&k("max-turns", bad), "max-turns", "1 to 1,000");
        }
        assert_eq!(parse(&k("runs", "100")).unwrap().runs, 100);
        for bad in ["0", "101", "2.5", "true"] {
            rejects(&k("runs", bad), "runs", "1 to 100");
        }
        // The pass-at is in 1 to the runs of the eval, and the runs come
        // from the table when the file has none.
        assert_eq!(parse(&k("pass-at", "3")).unwrap().pass_at, 3);
        for bad in ["0", "4", "1.5"] {
            rejects(&k("pass-at", bad), "pass-at", "from 1 to the runs");
        }
        let two = with(
            "\"max-budget-usd\"",
            "\"runs\":2,\"pass-at\":3,\"max-budget-usd\"",
        );
        rejects(&two, "pass-at", "(2)");
    }

    #[test]
    fn the_budget_rules() {
        rejects(
            &GOOD.replace("\"max-budget-usd\":1,", ""),
            "max-budget-usd",
            "required",
        );
        for bad in ["0", "-1", "\"1\"", "null"] {
            rejects(
                &with("\"max-budget-usd\":1", &format!("\"max-budget-usd\":{bad}")),
                "max-budget-usd",
                "above 0",
            );
        }
        rejects(
            &with("\"max-budget-usd\":1", "\"max-budget-usd\":20.5"),
            "max-budget-usd",
            "max-total-usd",
        );
        assert!(parse(&with("\"max-budget-usd\":1", "\"max-budget-usd\":20")).is_ok());
        assert!(parse(&with("\"max-budget-usd\":1", "\"max-budget-usd\":0.01")).is_ok());
    }

    #[test]
    fn the_check_rules() {
        let c = |v: &str| with("[{\"name\":\"done\",\"run\":\"true\"}]", v);
        rejects(
            &GOOD.replace(",\"checks\":[{\"name\":\"done\",\"run\":\"true\"}]", ""),
            "checks",
            "required",
        );
        rejects(&c("[]"), "checks", "no check");
        rejects(&c("{}"), "checks", "list of objects");
        rejects(&c("[5]"), "checks", "must be an object");
        rejects(&c("[{\"name\":\"a\"}]"), "checks", "exactly");
        rejects(
            &c("[{\"name\":\"a\",\"run\":\"x\",\"more\":1}]"),
            "checks",
            "exactly",
        );
        rejects(&c("[{\"name\":\"A\",\"run\":\"x\"}]"), "checks", "`name`");
        rejects(&c("[{\"name\":5,\"run\":\"x\"}]"), "checks", "`name`");
        rejects(&c("[{\"name\":\"a\",\"run\":\"\"}]"), "checks", "`run`");
        rejects(&c("[{\"name\":\"a\",\"run\":5}]"), "checks", "`run`");
        rejects(
            &c("[{\"name\":\"a\",\"run\":\"x\"},{\"name\":\"a\",\"run\":\"y\"}]"),
            "checks",
            "more than once",
        );
    }

    #[test]
    fn the_first_broken_rule_wins_in_the_order_of_the_table() {
        // A bad `checks` and a bad `id`: the id comes first.
        let text = r#"{"id":"other","task":"","fixture":"x","max-budget-usd":0,"checks":[]}"#;
        assert_eq!(parse(text).unwrap_err().key, "id");
        let text = r#"{"id":"example","task":"","fixture":"x","max-budget-usd":0,"checks":[]}"#;
        assert_eq!(parse(text).unwrap_err().key, "task");
        let text = r#"{"id":"example","task":"t","fixture":"x","max-budget-usd":0,"checks":[]}"#;
        assert_eq!(parse(text).unwrap_err().key, "fixture");
    }

    #[test]
    fn the_skeleton_has_four_paths() {
        assert_eq!(
            skeleton_paths("qa"),
            [
                "qa/check.sh",
                "qa/example.json",
                "qa/fixtures/example/notes.txt",
                ".github/workflows/agent-evals.yml"
            ]
        );
    }
}
