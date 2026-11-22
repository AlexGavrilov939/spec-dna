//! Tests of the committed fixtures of the upgrade (step 5, slice 1). The
//! fixtures are projects with the pin 0.4.2 or 0.5.0 (and `v061` and `v070`,
//! which `upgrade_v061.rs` and `upgrade_v070.rs` use). Only the real binaries
//! of those versions give the exact bytes and markers of their files, so
//! `tests/fixtures/upgrade/make.sh` records how the fixtures were made. A test
//! here runs no old binary. It reads the result that `make.sh` recorded in
//! `tests/fixtures/upgrade/recorded/<case>.txt`: the exit code of `check --all`,
//! each document that the old binary loaded, each finding and each file.

mod common;

use common::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

/// A finding of the old binary: its code and the path of its document.
type Finding = (&'static str, &'static str);

/// A case: a name, the fixture, the overlays, the pin of the project, and the
/// exit code that the old binary gave on it, the errors of its documents and
/// its findings of the harness (SD501, SD502, SD504 and SD505). Each finding
/// of the harness in a case is an error too.
struct Case {
    name: &'static str,
    base: &'static str,
    overlays: &'static [&'static str],
    pin: &'static str,
    exit: i32,
    errors: &'static [Finding],
    harness: &'static [Finding],
}

/// The errors of the corpus of the 0.4.2 and 0.5.0 fixtures. They are in the
/// files of the tags, and the upgrade must not make a new one. SD201 and SD203
/// of `valves.md` are the alias and the legacy key that the rest of the corpus
/// clashes with, and SD102 of `research.md` are its links to a missing file.
const CORPUS_ERRORS: &[Finding] = &[
    ("SD101", "docs/changes/add-drip-lines.md"),
    ("SD102", "docs/notes/research.md"),
    ("SD102", "docs/notes/research.md"),
    ("SD103", "docs/irrigation/valves.md"),
    ("SD201", "docs/irrigation/alias-clash.md"),
    ("SD201", "docs/irrigation/valves.md"),
    ("SD203", "docs/irrigation/valves.md"),
    ("SD305", "docs/decisions/ADR-2-pump-schedule.md"),
    ("SD402", "docs/harvest/records.md"),
    ("SD402", "docs/irrigation/legacy.md"),
];

/// The errors of the brownfield fixture: 2 files with the alias ADR-3, a file
/// that is not a decision in the folder of the decisions, and a `type: note`
/// file in the folder of the specs.
const BROWNFIELD_ERRORS: &[Finding] = &[
    ("SD201", "docs/adr/ADR-003-choice-3-notes.md"),
    ("SD201", "docs/adr/ADR-003-choice-3.md"),
    ("SD303", "docs/specs/review.md"),
    ("SD305", "docs/adr/ADR-003-choice-3-notes.md"),
    ("SD305", "docs/adr/README.md"),
];

const EDITED: Finding = ("SD501", ".claude/agents/checker.md");

const CASES: &[Case] = &[
    Case {
        name: "v04",
        base: "v04",
        overlays: &[],
        pin: "0.4.2",
        exit: 1,
        errors: CORPUS_ERRORS,
        harness: &[],
    },
    Case {
        name: "v05",
        base: "v05",
        overlays: &[],
        pin: "0.5.0",
        exit: 1,
        errors: CORPUS_ERRORS,
        harness: &[],
    },
    Case {
        name: "brownfield",
        base: "brownfield",
        overlays: &[],
        pin: "0.5.0",
        exit: 1,
        errors: BROWNFIELD_ERRORS,
        harness: &[],
    },
    Case {
        name: "harness-lean",
        base: "v05",
        overlays: &["harness-lean"],
        pin: "0.5.0",
        exit: 1,
        errors: CORPUS_ERRORS,
        harness: &[],
    },
    Case {
        name: "harness-full",
        base: "v05",
        overlays: &["harness-full"],
        pin: "0.5.0",
        exit: 1,
        errors: CORPUS_ERRORS,
        harness: &[],
    },
    // The one edited file: its hash in the marker does not match.
    Case {
        name: "harness-edited",
        base: "v05",
        overlays: &["harness-lean", "harness-edited"],
        pin: "0.5.0",
        exit: 1,
        errors: CORPUS_ERRORS,
        harness: &[EDITED],
    },
];

/// The result that `make.sh` recorded for a case.
struct Recorded {
    version: String,
    exit: i32,
    /// Path, id, type and status of each document.
    docs: Vec<[String; 4]>,
    /// Code, severity and path of each finding.
    findings: Vec<[String; 3]>,
    /// The git blob id of each file, by path.
    files: BTreeMap<String, String>,
}

fn recorded(name: &str) -> Recorded {
    let path = fixture_dir("recorded").join(format!("{name}.txt"));
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut out = Recorded {
        version: String::new(),
        exit: -1,
        docs: Vec::new(),
        findings: Vec::new(),
        files: BTreeMap::new(),
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# case ") {
            out.version = rest.rsplit(' ').next().unwrap().to_string();
        } else if let Some(rest) = line.strip_prefix("exit ") {
            out.exit = rest.parse().unwrap();
        } else if let Some(rest) = line.strip_prefix("doc ") {
            out.docs.push(fields(rest, name));
        } else if let Some(rest) = line.strip_prefix("finding ") {
            out.findings.push(fields(rest, name));
        } else if let Some(rest) = line.strip_prefix("file ") {
            let [blob, path] = fields(rest, name);
            out.files.insert(path, blob);
        } else {
            panic!("{name}: unknown line `{line}`");
        }
    }
    out
}

fn fields<const N: usize>(rest: &str, name: &str) -> [String; N] {
    let parts: Vec<String> = rest.split(' ').map(str::to_string).collect();
    parts
        .try_into()
        .unwrap_or_else(|_| panic!("{name}: `{rest}` has not {N} fields"))
}

/// The git blob id of each file of a repository, by path.
fn blobs(repo: &Repo) -> BTreeMap<String, String> {
    repo.git_out(&["ls-files", "-s"])
        .lines()
        .map(|line| {
            let (meta, path) = line.split_once('\t').unwrap();
            (
                path.to_string(),
                meta.split(' ').nth(1).unwrap().to_string(),
            )
        })
        .collect()
}

/// The paths of the files of a repository, as git lists them.
fn tracked(repo: &Repo) -> BTreeSet<String> {
    blobs(repo).into_keys().collect()
}

#[test]
fn each_fixture_commits_and_has_a_pin_before_the_binary() {
    for case in CASES {
        let repo = Repo::from_fixture(case.base, case.overlays);
        assert_eq!(
            repo.git_out(&["rev-list", "--count", "HEAD"]),
            "1",
            "{}",
            case.name
        );
        assert_eq!(
            repo.git_out(&["status", "--porcelain"]),
            "",
            "{}: the commit holds the whole tree",
            case.name
        );
        assert!(
            repo.read("spec-dna.toml")
                .starts_with(&format!("spec-dna = \"{}\"\n", case.pin)),
            "{}: the pin is the first line",
            case.name
        );
        let (code, _, err) = repo.run(&["check", "--all"]);
        assert_eq!(code, 3, "{}: {err}", case.name);
        assert!(err.contains(case.pin), "{}: {err}", case.name);
    }
}

#[test]
fn each_fixture_is_valid_for_its_own_version() {
    for case in CASES {
        let repo = Repo::from_fixture(case.base, case.overlays);
        let rec = recorded(case.name);
        assert_eq!(
            rec.version, case.pin,
            "{}: the binary of the pin",
            case.name
        );
        assert_eq!(rec.exit, case.exit, "{}", case.name);
        // The recording is of these bytes: each file has the blob id that was
        // recorded.
        assert_eq!(
            rec.files,
            blobs(&repo),
            "{}: run tests/fixtures/upgrade/make.sh again",
            case.name
        );
        // The old binary loaded documents, and each one is a file of the tree.
        assert!(!rec.docs.is_empty(), "{}", case.name);
        for [path, id, ..] in &rec.docs {
            assert!(
                rec.files.contains_key(path),
                "{}: {id} at {path}",
                case.name
            );
        }
        // The errors are the known ones. The old binary gave no finding of
        // the harness other than the edited file.
        let errors = |rec: &Recorded| -> Vec<(String, String)> {
            let mut list: Vec<_> = rec
                .findings
                .iter()
                .filter(|[_, severity, _]| severity == "error")
                .map(|[code, _, path]| (code.clone(), path.clone()))
                .collect();
            list.sort();
            list
        };
        let mut expected: Vec<(String, String)> = case
            .errors
            .iter()
            .chain(case.harness)
            .map(|(code, path)| (code.to_string(), path.to_string()))
            .collect();
        expected.sort();
        assert_eq!(errors(&rec), expected, "{}", case.name);
        let mut harness: Vec<(String, String)> = rec
            .findings
            .iter()
            .filter(|[code, ..]| matches!(code.as_str(), "SD501" | "SD502" | "SD504" | "SD505"))
            .map(|[code, _, path]| (code.clone(), path.clone()))
            .collect();
        harness.sort();
        let want: Vec<_> = case
            .harness
            .iter()
            .map(|(code, path)| (code.to_string(), path.to_string()))
            .collect();
        assert_eq!(harness, want, "{}", case.name);
    }
}

#[test]
fn the_overlay_text_is_appended_and_the_file_append_is_not_copied() {
    for level in ["lean", "full"] {
        let repo = Repo::from_fixture("v05", &[&format!("harness-{level}")]);
        let base = Repo::from_fixture("v05", &[]);
        let append = fs::read_to_string(
            fixture_dir(&format!("harness-{level}")).join("spec-dna.toml.append"),
        )
        .unwrap();
        assert_eq!(
            repo.read("spec-dna.toml"),
            format!("{}{append}", base.read("spec-dna.toml"))
        );
        assert!(append.contains(&format!("level = \"{level}\"")));
        assert!(!repo.dir.join("spec-dna.toml.append").exists());
    }
}

#[test]
fn the_harness_levels_differ_by_their_agent_files() {
    let agents = |overlay: &str| -> BTreeSet<String> {
        let repo = Repo::from_fixture("v05", &[overlay]);
        tracked(&repo)
            .into_iter()
            .filter(|p| p.starts_with(".claude/agents/"))
            .collect()
    };
    let lean = agents("harness-lean");
    let full = agents("harness-full");
    assert!(lean.contains(".claude/agents/checker.md"));
    assert!(lean.is_subset(&full));
    let more: Vec<_> = full.difference(&lean).cloned().collect();
    assert_eq!(
        more,
        [
            ".claude/agents/implementer.md",
            ".claude/agents/spec-writer.md"
        ]
    );
}

#[test]
fn the_edited_overlay_differs_from_the_generated_file_by_one_line() {
    let lean =
        fs::read_to_string(fixture_dir("harness-lean").join(".claude/agents/checker.md")).unwrap();
    let edited =
        fs::read_to_string(fixture_dir("harness-edited").join(".claude/agents/checker.md"))
            .unwrap();
    assert_eq!(edited.lines().count(), lean.lines().count() + 1);
    let marker = |t: &str| t.lines().last().unwrap().to_string();
    assert_eq!(marker(&edited), marker(&lean), "the marker keeps its hash");
    assert!(marker(&lean).starts_with("<!-- spec-dna harness v0.5.0 h:"));
}

/// What the fixtures hold that a later slice needs. Each row is a line of the
/// plan: the document types, the statuses of a change, the legacy ids, the
/// roots and the links.
#[test]
fn the_fixtures_hold_what_the_plan_names() {
    let v05 = recorded("v05");
    let types: BTreeSet<&str> = v05.docs.iter().map(|d| d[2].as_str()).collect();
    for ty in [
        "spec",
        "contract",
        "invariant",
        "guide",
        "runbook",
        "decision",
        "change",
        "incident",
        "note",
        "reference",
    ] {
        assert!(types.contains(ty), "v05 has no {ty}");
    }
    let statuses = |rec: &Recorded, ty: &str| -> BTreeSet<String> {
        rec.docs
            .iter()
            .filter(|d| d[2] == ty)
            .map(|d| d[3].clone())
            .collect()
    };
    let set =
        |words: &[&str]| -> BTreeSet<String> { words.iter().map(|w| w.to_string()).collect() };
    assert_eq!(
        statuses(&v05, "change"),
        set(&["proposed", "accepted", "done", "rejected"])
    );
    assert_eq!(
        statuses(&recorded("brownfield"), "change"),
        set(&["proposed", "done"])
    );
    assert_eq!(statuses(&recorded("v04"), "change"), set(&["proposed"]));
    // A change in a subfolder, and a change named `index.md`: the id of the
    // second is its folder.
    let id_of = |path: &str| {
        v05.docs
            .iter()
            .find(|d| d[0] == path)
            .map(|d| d[1].as_str())
    };
    assert_eq!(
        id_of("docs/changes/planning/add-pump-timer.md"),
        Some("changes/planning/add-pump-timer")
    );
    assert_eq!(
        id_of("docs/changes/new-sensors/index.md"),
        Some("changes/new-sensors")
    );

    let repo = Repo::from_fixture("v05", &[]);
    let config = repo.read("spec-dna.toml");
    for needle in [
        "legacy-ids = [\"ADR-{n}\"",
        "alias = \"DR-{n}\"",
        "legacy-keys",
        "type = \"note\"",
        "[budgets.guide]",
        "[budgets.runbook]",
        "[budgets.note]",
    ] {
        assert!(config.contains(needle), "v05 configuration: {needle}");
    }
    assert!(config.lines().filter(|l| l.starts_with("# ")).count() >= 3);
    // A decision with the legacy id of a table pattern.
    assert_eq!(
        id_of("docs/decisions/4-drain-timing.md"),
        Some("4-drain-timing")
    );
    // The rejected change is cited by the locked text of an accepted decision.
    assert!(
        repo.read("docs/decisions/ADR-3-solar-pump.md")
            .contains("[[changes/solar-pump]]")
    );
    // The done change is cited by no document.
    for path in v05.files.keys().filter(|p| p.ends_with(".md")) {
        if path != "docs/changes/tune-pump.md" {
            assert!(!repo.read(path).contains("tune-pump"), "{path}");
        }
    }
    // The accepted change and the spec link each other.
    assert!(
        repo.read("docs/changes/rework-pumps.md")
            .contains("](../irrigation/pumps.md)")
    );
    assert!(
        repo.read("docs/irrigation/pumps.md")
            .contains("](../changes/rework-pumps.md)")
    );
    // A decision with no section `Status`.
    assert!(
        !repo
            .read("docs/decisions/ADR-2-pump-schedule.md")
            .contains("## Status")
    );
    // A nested mixed root with no default type.
    assert!(config.contains("path = \"docs/lab\"\ntype = \"mixed\"\n\n"));

    // The brownfield fixture: a root of one type for each folder, a `type:
    // note` file in the specs, and a second file with the alias ADR-3.
    let brown = Repo::from_fixture("brownfield", &[]);
    let config = brown.read("spec-dna.toml");
    for ty in [
        "decision",
        "spec",
        "runbook",
        "guide",
        "change",
        "note",
        "reference",
    ] {
        assert!(
            config.contains(&format!("type = \"{ty}\"")),
            "brownfield: {ty}"
        );
    }
    assert!(brown.read("docs/specs/review.md").contains("type: note"));
    assert!(
        brown
            .dir
            .join("docs/adr/ADR-003-choice-3-notes.md")
            .is_file()
    );

    // No fixture of 0.4 or 0.5 has a table `[harness]` without its overlay.
    for name in ["v04", "v05", "brownfield"] {
        let text = Repo::from_fixture(name, &[]).read("spec-dna.toml");
        assert!(!text.contains("[harness"), "{name}");
    }
}

/// The fixture of 0.7.0 is the tree that the real binary 0.7.0 wrote, and it
/// has what the plan of 0.7.1 names: a harness, a change with an intent, a
/// spec and a plan, a decision, tasks with `Hours:`, a roadmap, a questions
/// file and a pre-commit file with the hook `typos`. It is not a case of
/// `CASES`: its pin is the version of the binary until the release moves it,
/// so `check --all` passes today.
#[test]
fn the_v070_fixture_is_the_tree_that_the_real_binary_wrote() {
    let repo = Repo::from_fixture("v070", &[]);
    let rec = recorded("v070");
    assert_eq!(rec.version, "0.7.0");
    assert_eq!(rec.exit, 0);
    assert!(rec.findings.is_empty(), "0.7.0 found: {:?}", rec.findings);
    assert_eq!(
        rec.files,
        blobs(&repo),
        "run tests/fixtures/upgrade/make.sh v070 again"
    );
    // The documents: one of each kind that the plan names.
    let types: BTreeSet<&str> = rec.docs.iter().map(|d| d[2].as_str()).collect();
    for ty in [
        "decision",
        "intent",
        "spec",
        "plan",
        "requirements",
        "roadmap",
        "questions",
    ] {
        assert!(types.contains(ty), "v070 has no {ty}");
    }
    let status = |ty: &str| -> BTreeSet<String> {
        rec.docs
            .iter()
            .filter(|d| d[2] == ty)
            .map(|d| d[3].clone())
            .collect()
    };
    assert_eq!(status("plan"), BTreeSet::from(["approved".to_string()]));
    assert_eq!(status("spec"), BTreeSet::from(["approved".to_string()]));
    // Each task of the plan has hours.
    let plan = repo.read("docs/intent/drip-schedule/plan.md");
    let tasks: Vec<&str> = plan.lines().filter(|l| l.starts_with("- [ ] t")).collect();
    assert!(tasks.len() >= 2, "{plan}");
    for task in tasks {
        assert!(task.contains(" Hours: "), "{task}");
    }
    // The configuration: the pin, the profile, the harness and the approval.
    let config = repo.read("spec-dna.toml");
    for needle in [
        "spec-dna = \"0.7.0\"\n",
        "profile = \"standard\"",
        "[harness]",
        "level = \"lean\"",
        "approval = \"branch\"",
    ] {
        assert!(config.contains(needle), "v070 configuration: {needle}");
    }
    // The generated files carry the marker of 0.7.0, and `typos` has no
    // setting for the hashes.
    assert!(repo.read("AGENTS.md").contains("<!-- spec-dna v0.7.0 h:"));
    assert!(repo.read(".pre-commit-config.yaml").contains("id: typos"));
    for name in ["_typos.toml", "typos.toml", ".typos.toml"] {
        assert!(!repo.dir.join(name).exists(), "{name}");
    }
}

/// The 3 fixtures of 0.7.1 are the trees that the real binary 0.7.1 wrote, one
/// for each profile. They share the hand-written documents, script and test, and
/// they differ in the files that `init` writes.
#[test]
fn the_v071_fixtures_are_the_trees_that_the_real_binary_wrote() {
    // The fixture, its profile, the approval and the skills of its list.
    let cases: [(&str, &str, &str, &[&str]); 3] = [
        (
            "v071",
            "standard",
            "approval = \"branch\"",
            &[
                "brainstorm",
                "specify",
                "clarify",
                "write-plan",
                "tdd",
                "review",
                "finish",
                "post-mortem",
            ],
        ),
        (
            "v071-lite",
            "lite",
            "approval = \"branch\"",
            &["brainstorm", "write-plan", "finish"],
        ),
        (
            "v071-strict",
            "strict",
            "approval = \"base\"",
            &[
                "brainstorm",
                "specify",
                "clarify",
                "write-plan",
                "tdd",
                "review",
                "finish",
                "post-mortem",
            ],
        ),
    ];
    let standard = Repo::from_fixture("v071", &[]);
    for (name, profile, approval, skills) in cases {
        let repo = Repo::from_fixture(name, &[]);
        let rec = recorded(name);
        assert_eq!(rec.version, "0.7.1", "{name}");
        assert_eq!(rec.exit, 0, "{name}");
        assert!(rec.findings.is_empty(), "{name}: {:?}", rec.findings);
        assert_eq!(
            rec.files,
            blobs(&repo),
            "{name}: run tests/fixtures/upgrade/make.sh v071 again"
        );
        let config = repo.read("spec-dna.toml");
        for needle in [
            "spec-dna = \"0.7.1\"\n".to_string(),
            format!("profile = \"{profile}\""),
            approval.to_string(),
            "level = \"lean\"".to_string(),
            format!(
                "skills = [{}]",
                skills
                    .iter()
                    .map(|s| format!("\"{s}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ] {
            assert!(config.contains(&needle), "{name} configuration: {needle}");
        }
        assert!(repo.read("AGENTS.md").contains("<!-- spec-dna v0.7.1 h:"));
        // The 0.7.1 files only: no agent of 0.8.0, and no skill `accept`.
        assert!(!repo.dir.join(".claude/agents/acceptor.md").exists());
        assert!(!repo.dir.join(".claude/skills/spec-dna-accept").exists());
        // The hand-written files are the files of `v071`.
        for path in [
            "docs/intent/drip-schedule/plan.md",
            "docs/requirements/valves.md",
            "src/schedule.sh",
            "tests/schedule_test.sh",
            ".pre-commit-config.yaml",
        ] {
            assert_eq!(repo.read(path), standard.read(path), "{name}: {path}");
        }
    }
}
