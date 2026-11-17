//! `spec-dna eval pack`: the evals that measure each skill (ADR 0016, design
//! D32 and D35). The pack has 5 evals for each of the 11 skills, 55 in total,
//! with small fixtures that hold no data of a consumer project (rule zero).
//! The command writes each eval as `<[evals] dir>/pack/<id>.json`, and the
//! files of its fixture below `<[evals] dir>/pack/fixtures/<id>/`. It writes a
//! file once and never again, so a project can edit its copy.
//!
//! # How to add an eval
//!
//! An eval is one `PackEval` in the list of its group (`EXECUTE_TDD`,
//! `VERIFY_DEBUG`, `BRAINSTORM_SPECIFY`, `CLARIFY_WRITE_PLAN`,
//! `REVIEW_FINISH` or `POST_MORTEM`). The module needs no other change.
//!
//! - `id` is `<skill>-<n>`, with n from 1 to 5. The pair `agents-line` uses
//!   `tdd-1`, `debug-1`, `verify-1`, `review-1` and `execute-1`.
//! - `files` are the files of the fixture: the path in the project and the
//!   text. The text `{{VERSION}}` becomes the version of the binary, so a
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

/// The order of a test and its code in the history of the copy: the first
/// commit that touches `tests/` comes before the first commit that touches
/// `src/`. The copy has a base commit, and `$SPEC_DNA_EVAL_BASE` names it.
const TEST_BEFORE_CODE: &str = "t=$(git log --reverse --format=%H \"$SPEC_DNA_EVAL_BASE\"..HEAD -- tests | head -1); c=$(git log --reverse --format=%H \"$SPEC_DNA_EVAL_BASE\"..HEAD -- src | head -1); [ -n \"$t\" ] && [ -n \"$c\" ] && [ \"$t\" != \"$c\" ] && git merge-base --is-ancestor \"$t\" \"$c\"";

/// Group of t12: `execute` and `tdd`.
static EXECUTE_TDD: &[PackEval] = &[PackEval {
    id: "tdd-1",
    skill: "tdd",
    task: "In this project, `src/slug.sh` has the function `slug_lower`. Add the function `slug_trim` to the same file. It prints its first argument without the dashes at its start and at its end. Commit each step of your work.",
    allowed_tools: CODE_TOOLS,
    max_turns: 30,
    max_budget_usd: 2.0,
    branch: None,
    checks: &[
        ("function-exists", "grep -q 'slug_trim' src/slug.sh"),
        (
            "behaviour",
            ". ./src/slug.sh && [ \"$(slug_trim '--a-b--')\" = 'a-b' ]",
        ),
        ("tests-pass", "sh tests/test_slug.sh"),
        ("test-added", "grep -q 'slug_trim' tests/test_slug.sh"),
        ("test-before-code", TEST_BEFORE_CODE),
    ],
    files: &[
        ("spec-dna.toml", SPEC_DNA_TOML),
        ("docs/slug.md", "# Slug\n\nThe helpers of the slug.\n"),
        (
            "src/slug.sh",
            "slug_lower() {\n    printf '%s\\n' \"$1\" | tr 'A-Z' 'a-z'\n}\n",
        ),
        (
            "tests/test_slug.sh",
            ". \"$(dirname \"$0\")/../src/slug.sh\"\n[ \"$(slug_lower AbC)\" = abc ] || { echo 'slug_lower failed'; exit 1; }\n",
        ),
    ],
}];

/// Group of t13: `verify` and `debug`.
static VERIFY_DEBUG: &[PackEval] = &[];
/// Group of t14: `brainstorm` and `specify`.
static BRAINSTORM_SPECIFY: &[PackEval] = &[];
/// Group of t15: `clarify` and `write-plan`.
static CLARIFY_WRITE_PLAN: &[PackEval] = &[];
/// Group of t16: `review` and `finish`.
static REVIEW_FINISH: &[PackEval] = &[];
/// Group of t17: `post-mortem`.
static POST_MORTEM: &[PackEval] = &[];

/// Each eval of the pack, in the order of the groups.
pub fn all() -> Vec<&'static PackEval> {
    [
        EXECUTE_TDD,
        VERIFY_DEBUG,
        BRAINSTORM_SPECIFY,
        CLARIFY_WRITE_PLAN,
        REVIEW_FINISH,
        POST_MORTEM,
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
        Evals {
            dir: "evals".into(),
            min_pass_rate: 0.8,
            runs: 3,
            max_total_usd: 20.0,
        }
    }

    #[test]
    fn each_eval_of_the_pack_is_valid_for_the_runner_and_sd506() {
        let evals = all();
        assert!(!evals.is_empty());
        for e in evals {
            let text = eval_json(e, "evals");
            let parsed = crate::evals::parse_eval(e.id, text.as_bytes(), &table(), &|p| {
                p == format!("evals/pack/fixtures/{}", e.id)
            })
            .unwrap_or_else(|err| panic!("{}: {}", e.id, err.message));
            assert_eq!(parsed.skill.as_deref(), Some(e.skill), "{}", e.id);
            assert_eq!(parsed.runs, RUNS, "{}", e.id);
            assert!(!e.files.is_empty(), "{}: no fixture file", e.id);
            let (skill, n) = e.id.rsplit_once('-').unwrap();
            assert_eq!(skill, e.skill, "{}", e.id);
            assert!(("1"..="5").contains(&n), "{}", e.id);
            assert!(Skill::from_key(skill).is_some(), "{}", e.id);
        }
    }

    #[test]
    fn the_ids_are_unique_and_the_groups_hold_their_skills() {
        let mut ids: Vec<&str> = all().iter().map(|e| e.id).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "an id is twice in the pack");
        let groups: [(&[PackEval], &[&str]); 6] = [
            (EXECUTE_TDD, &["execute", "tdd"]),
            (VERIFY_DEBUG, &["verify", "debug"]),
            (BRAINSTORM_SPECIFY, &["brainstorm", "specify"]),
            (CLARIFY_WRITE_PLAN, &["clarify", "write-plan"]),
            (REVIEW_FINISH, &["review", "finish"]),
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
    #[ignore = "part B: t12-t17"]
    fn the_pack_has_5_evals_for_each_of_the_11_skills() {
        let evals = all();
        assert_eq!(evals.len(), 55);
        for skill in Skill::ALL {
            let n = evals.iter().filter(|e| e.skill == skill.key()).count();
            assert_eq!(n, 5, "{}", skill.key());
        }
    }
}
