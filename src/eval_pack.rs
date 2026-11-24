//! `spec-dna eval pack`: the evals that measure each skill (ADR 0016, design
//! D32 and D35). The pack has 5 evals for each of 11 skills, 2 evals of
//! the pair `agents-line` and the 5 evals `accept-1` to `accept-5` of the
//! acceptance record (D62), 62 in total,
//! with small fixtures that hold no data of a consumer project (rule zero).
//! The command writes each eval as `<[evals] dir>/pack/<id>.json`, and the
//! files of its fixture below `<[evals] dir>/pack/fixtures/<id>/`. It writes a
//! file once and never again, so a project can edit its copy.
//!
//! # How to add an eval
//!
//! An eval is one `PackEval` in the list of its group (`EXECUTE`, `TDD`,
//! `VERIFY`, `DEBUG`, `BRAINSTORM_SPECIFY`, `CLARIFY_WRITE_PLAN`,
//! `REVIEW_FINISH` or `POST_MORTEM`). The module needs no other change. The
//! 10 evals of `tdd` and `debug` are in the sibling file `eval_pack_pilot.rs`,
//! the 5 of `execute` are in `eval_pack_execute.rs`, the 5 of `verify` and
//! the 2 of `agents-line` are in `eval_pack_harder.rs`, and the 5 of the
//! acceptance record are in `eval_pack_accept.rs`, with their fixtures.
//! Each of them has a reference solution and wrong solutions in
//! `tests/fixtures/eval-solutions/<id>/`, and `tests/eval_pack_pilot/mod.rs`
//! runs them with a stub `claude`. The evals of `brainstorm` are in
//! `eval_pack_rest.rs`. Their solutions are in the same folder, and
//! `tests/eval_pack_rest.rs` runs them.
//!
//! - `id` is `<group>-<n>`, with n from 1. The ids of `brainstorm` and of
//!   `execute` are 6 to 10, and the ids of `verify` are 6 to 10. The pair
//!   `agents-line` uses `tdd-1`, `debug-1`, `execute-10`, `agents-line-1` and
//!   `agents-line-2`.
//! - `files` are the files of the fixture: the path in the project and the
//!   text. `change` (optional, D60) holds the files of a commit on `branch`
//!   that the runner makes after the base commit. The text `{{VERSION}}` becomes the version of the binary, so a
//!   fixture pins the version that wrote it. The runner also writes the
//!   version of the running binary into the pin line of `spec-dna.toml`.
//! - A check is a shell command. It runs in the copy after the agent ends, and
//!   `$SPEC_DNA_EVAL_BASE` is the base commit. A check that runs
//!   `spec-dna check` must read the codes that it needs (for example
//!   `grep -q 'SD919'`), never its exit code alone, because the 2 arms differ
//!   in the generated files and then differ in SD501.
//! - The `without` arm denies `spec-dna skill`, and a check must not call it.
//! - `runs` is `RUNS` for each eval of the pack.

use crate::json::{self, Obj};
use crate::repo::{Fatal, Repo};
use std::fs;
use std::io::Write;

/// The runs of each eval of the pack: 4 runs, 20 for one skill and one arm.
pub const RUNS: u32 = 4;

/// An eval of the pack, with the files of its fixture.
pub struct PackEval {
    /// `<skill>-<n>`.
    pub id: &'static str,
    /// The skill that the eval measures.
    pub skill: &'static str,
    /// The request that the agent gets.
    pub task: &'static str,
    pub allowed_tools: &'static [&'static str],
    pub max_turns: u32,
    pub max_budget_usd: f64,
    pub branch: Option<&'static str>,
    /// The files of the change (D60): the path in the project and the text.
    /// The runner copies them over the copy and commits them on `branch`.
    /// `write` puts them in `<dir>/pack/changes/<id>/`.
    pub change: Option<&'static [(&'static str, &'static str)]>,
    /// The name and the shell command of each check.
    pub checks: &'static [(&'static str, &'static str)],
    /// The files of the fixture: the path and the text.
    pub files: &'static [(&'static str, &'static str)],
}

/// The tools that an eval of a code task allows.
const CODE_TOOLS: &[&str] = &[
    "Read",
    "Edit",
    "Write",
    "Bash(git:*)",
    "Bash(sh:*)",
    "Bash(spec-dna:*)",
];

/// A fixture file: the pin of the version, and one root of documents.
const SPEC_DNA_TOML: &str = "spec-dna = \"{{VERSION}}\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n";

/// The pilot evals of `tdd` and `debug`, with their fixtures.
#[path = "eval_pack_pilot.rs"]
mod pilot;

/// The harder evals of `execute` (`execute-6` to `execute-10`).
#[path = "eval_pack_execute.rs"]
mod execute;

/// The harder evals of `verify` (`verify-6` to `verify-10`), and the 2 evals
/// of the pair `agents-line`, in `eval_pack_harder.rs`.
#[path = "eval_pack_harder.rs"]
mod harder;

/// The 5 evals of the acceptance record: `accept-1` to `accept-5`. Their key
/// `skill` is `review` (design 0.8.0, D62 and D71).
#[path = "eval_pack_accept.rs"]
mod accept;

/// Group of `execute`: `execute-6` to `execute-10`.
static EXECUTE: &[PackEval] = execute::EXECUTE;
/// Group of `tdd`.
static TDD: &[PackEval] = pilot::TDD;
/// Group of `verify`: `verify-6` to `verify-10`.
static VERIFY: &[PackEval] = harder::VERIFY;
/// Group of `debug`.
static DEBUG: &[PackEval] = pilot::DEBUG;
/// The 2 evals of the pair `agents-line` that no skill owns.
static AGENTS_LINE_OWN: &[PackEval] = harder::AGENTS_LINE;
/// Group of t14: `brainstorm` and `specify`.
static BRAINSTORM_SPECIFY: &[PackEval] = crate::eval_pack_rest::BRAINSTORM_SPECIFY;
/// Group of t15: `clarify` and `write-plan`.
static CLARIFY_WRITE_PLAN: &[PackEval] = crate::eval_pack_rest::CLARIFY_WRITE_PLAN;
/// Group of t16: `review` and `finish`.
static REVIEW_FINISH: &[PackEval] = crate::eval_pack_rest::REVIEW_FINISH;
/// The 5 evals of the acceptance record. The id is `accept-<n>`, the skill is
/// `review`.
static ACCEPT: &[PackEval] = accept::ACCEPT;
/// Group of t17: `post-mortem`.
static POST_MORTEM: &[PackEval] = crate::eval_pack_rest::POST_MORTEM;

/// Each eval of the pack, in the order of the groups.
pub fn all() -> Vec<&'static PackEval> {
    [
        EXECUTE,
        TDD,
        VERIFY,
        DEBUG,
        BRAINSTORM_SPECIFY,
        CLARIFY_WRITE_PLAN,
        REVIEW_FINISH,
        ACCEPT,
        POST_MORTEM,
        AGENTS_LINE_OWN,
    ]
    .into_iter()
    .flat_map(|g| g.iter())
    .collect()
}

/// The text of the file `<dir>/pack/<id>.json`.
pub fn eval_json(e: &PackEval, dir: &str) -> String {
    let checks = json::array(
        e.checks
            .iter()
            .map(|(name, run)| Obj::new().str("name", name).str("run", run).end()),
    );
    let tools: Vec<String> = e.allowed_tools.iter().map(ToString::to_string).collect();
    let mut obj = Obj::new()
        .str("id", e.id)
        .str("skill", e.skill)
        .str("task", e.task)
        .str("fixture", &format!("{dir}/pack/fixtures/{}", e.id));
    if let Some(b) = e.branch {
        obj = obj.str("branch", b);
    }
    if e.change.is_some() {
        obj = obj.str("change", &format!("{dir}/pack/changes/{}", e.id));
    }
    let compact = obj
        .strs("allowed-tools", &tools)
        .num("max-turns", e.max_turns)
        .num("max-budget-usd", e.max_budget_usd)
        .num("runs", RUNS)
        .raw("checks", &checks)
        .end();
    json::pretty(&json::parse(&compact).expect("the writer makes valid JSON"))
}

/// The paths that `write` wrote and kept, from the repository root.
#[derive(Default)]
pub struct Written {
    pub written: Vec<String>,
    pub kept: Vec<String>,
}

/// Writes the files of `evals` below `<dir>/pack/`. A file that exists stays
/// as it is. The files are the eval and the files of its fixture.
pub fn write(repo: &Repo, dir: &str, evals: &[&PackEval]) -> Result<Written, Fatal> {
    let mut out = Written::default();
    for e in evals {
        let mut files = vec![(format!("{dir}/pack/{}.json", e.id), eval_json(e, dir))];
        for (path, text) in e.files {
            files.push((
                format!("{dir}/pack/fixtures/{}/{path}", e.id),
                text.replace("{{VERSION}}", crate::adapters::VERSION),
            ));
        }
        for (path, text) in e.change.unwrap_or(&[]) {
            files.push((
                format!("{dir}/pack/changes/{}/{path}", e.id),
                text.replace("{{VERSION}}", crate::adapters::VERSION),
            ));
        }
        for (path, text) in files {
            if write_once(repo, &path, &text)? {
                out.written.push(path);
            } else {
                out.kept.push(path);
            }
        }
    }
    Ok(out)
}

/// Writes a file if it is absent. Returns false if the path exists.
fn write_once(repo: &Repo, rel: &str, text: &str) -> Result<bool, Fatal> {
    let path = repo.root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| Fatal::Io(format!("cannot make the folder of `{rel}`: {e}")))?;
    }
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut f) => {
            f.write_all(text.as_bytes())
                .map_err(|e| Fatal::Io(format!("cannot write `{rel}`: {e}")))?;
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(Fatal::Io(format!("cannot write `{rel}`: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Evals;
    use crate::harness::Skill;

    fn table() -> Evals {
        Evals::defaults("evals")
    }

    /// The skill of a group: the group `accept` measures the skill `review`.
    fn group_skill(group: &str) -> &str {
        if group == "accept" { "review" } else { group }
    }

    #[test]
    fn each_eval_of_the_pack_is_valid_for_the_runner_and_sd506() {
        let evals = all();
        assert!(!evals.is_empty());
        for e in evals {
            let text = eval_json(e, "evals");
            let parsed = crate::evals::parse_eval(e.id, text.as_bytes(), &table(), &|p| {
                p == format!("evals/pack/fixtures/{}", e.id)
                    || (e.change.is_some() && p == format!("evals/pack/changes/{}", e.id))
            })
            .unwrap_or_else(|err| panic!("{}: {}", e.id, err.message));
            assert_eq!(parsed.skill.as_deref(), Some(e.skill), "{}", e.id);
            assert_eq!(parsed.runs, RUNS, "{}", e.id);
            assert!(!e.files.is_empty(), "{}: no fixture file", e.id);
            // The id is `<group>-<n>`. The group is a skill, `agents-line`, or
            // `accept`, whose evals have the skill `review` (D62).
            let (group, n) = e.id.rsplit_once('-').unwrap();
            assert_eq!(group_skill(group), e.skill, "{}", e.id);
            assert!(n.parse::<u32>().is_ok_and(|n| n >= 1), "{}", e.id);
            assert!(
                Skill::from_key(group).is_some() || ["agents-line", "accept"].contains(&group),
                "{}",
                e.id
            );
        }
    }

    #[test]
    fn the_ids_are_unique_and_the_groups_hold_their_skills() {
        let mut ids: Vec<&str> = all().iter().map(|e| e.id).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "an id is twice in the pack");
        let groups: [(&[PackEval], &[&str]); 10] = [
            (EXECUTE, &["execute"]),
            (TDD, &["tdd"]),
            (VERIFY, &["verify"]),
            (DEBUG, &["debug"]),
            (AGENTS_LINE_OWN, &["agents-line"]),
            (BRAINSTORM_SPECIFY, &["brainstorm", "specify"]),
            (CLARIFY_WRITE_PLAN, &["clarify", "write-plan"]),
            (REVIEW_FINISH, &["review", "finish"]),
            (ACCEPT, &["review"]),
            (POST_MORTEM, &["post-mortem"]),
        ];
        for (group, skills) in groups {
            for e in group {
                assert!(skills.contains(&e.skill), "{} is in the wrong group", e.id);
            }
        }
    }

    #[test]
    fn a_check_that_runs_spec_dna_check_names_the_codes_that_it_reads() {
        for e in all() {
            for (name, run) in e.checks {
                if run.contains("spec-dna check") {
                    assert!(run.contains("SD"), "{} {name}: no code", e.id);
                }
                assert!(!run.contains("spec-dna skill"), "{} {name}", e.id);
            }
        }
    }

    #[test]
    fn a_fixture_pins_the_version_and_holds_no_path_outside_the_project() {
        for e in all() {
            for (path, _) in e.files {
                assert!(
                    !path.starts_with('/') && !path.split('/').any(|s| s == ".." || s == "."),
                    "{}: {path}",
                    e.id
                );
            }
            if let Some((_, text)) = e.files.iter().find(|f| f.0 == "spec-dna.toml") {
                assert!(text.starts_with("spec-dna = \"{{VERSION}}\""), "{}", e.id);
            }
        }
    }

    #[test]
    fn the_pack_has_5_evals_for_each_skill_2_of_agents_line_and_5_of_accept() {
        let evals = all();
        assert_eq!(evals.len(), 62);
        // The group of a skill is the prefix of the id. The 5 evals `accept-1`
        // to `accept-5` have the skill `review` (D62), so the skill `review`
        // has 10 evals and its group has 5.
        let in_group = |g: &str| {
            evals
                .iter()
                .filter(|e| e.id.rsplit_once('-').unwrap().0 == g)
                .count()
        };
        for skill in Skill::ALL {
            assert_eq!(in_group(skill.key()), 5, "{}", skill.key());
            let by_skill = evals.iter().filter(|e| e.skill == skill.key()).count();
            assert_eq!(by_skill, if skill.key() == "review" { 10 } else { 5 });
        }
        assert_eq!(in_group("accept"), 5);
        let accept: Vec<&str> = evals
            .iter()
            .filter(|e| e.id.starts_with("accept-"))
            .map(|e| e.id)
            .collect();
        assert_eq!(
            accept,
            ["accept-1", "accept-2", "accept-3", "accept-4", "accept-5"]
        );
        let own: Vec<&str> = evals
            .iter()
            .filter(|e| e.skill == "agents-line")
            .map(|e| e.id)
            .collect();
        assert_eq!(own, ["agents-line-1", "agents-line-2"]);
        // The pilot evals of `verify`, `brainstorm` and `execute` are gone: both
        // arms passed each run of them.
        for skill in ["verify", "brainstorm", "execute"] {
            let ids: Vec<&str> = evals
                .iter()
                .filter(|e| e.skill == skill)
                .map(|e| e.id)
                .collect();
            let want: Vec<String> = (6..=10).map(|n| format!("{skill}-{n}")).collect();
            assert_eq!(ids, want, "{skill}");
        }
    }
}
