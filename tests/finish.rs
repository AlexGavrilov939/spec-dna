//! Integration tests of `spec-dna finish` (step 3, slice S6): the steps, the
//! stops, the edits of the delta, the write and the stage, and the merge kinds.
//!
//! Each fixture has a `main` branch with the base commit, a
//! `refs/remotes/origin/HEAD` that names it, and a `feature` branch. The base
//! commit holds the folder of the change `dose`. A test acts as the pull request
//! of the feature branch: it runs `finish`, and then it commits the result.

mod common;

use common::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const VALVES: &str = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n- **open**: The controller SHALL open the valve in 5 seconds.\n";
const HTTP: &str = "---\nstatus: active\n---\n# HTTP\n\n## Requirements\n\n- **ping**: The service SHALL answer a ping.\n";
const DECISION: &str =
    "---\nstatus: {status}\n---\n# Use a timer\n\n## Decision Outcome\n\nUse a timer.\n";

const FOLDER: &str = "docs/intent/dose";
const SPEC: &str = "docs/intent/dose/spec.md";
const INTENT: &str = "docs/intent/dose/intent.md";
const PLAN: &str = "docs/intent/dose/plan.md";
const VALVES_PATH: &str = "docs/requirements/valves.md";

const CHANGE_CLOSE: &str = "- CHANGE requirements/valves#close@{close}: The controller SHALL close the valve in 4 seconds.";
const ADD_FLUSH: &str =
    "- ADD requirements/valves#flush: The controller SHALL flush the line before it closes.";
const ADD_PUMP: &str = "- ADD requirements/pump#start: The pump SHALL start the flow.";
const REMOVE_OPEN: &str = "- REMOVE requirements/valves#open@{open}";
const COVERS_CLOSE_FLUSH: &str =
    "# covers: requirements/valves#close\n# covers: requirements/valves#flush\n";

struct Fx<'a> {
    profile: &'a str,
    rules: &'a str,
    root_extra: &'a str,
    /// The `moves-to` list of the layer `intent`, as TOML.
    moves_to: &'a str,
    /// The status of the intent.
    status: &'a str,
    /// The delta lines of the spec. `{close}` and `{open}` stand for the hash
    /// of the requirement. With no line and no extra, the change is routine and
    /// has no spec.
    delta: &'a [&'a str],
    /// Text that the spec has after the `Delta` section.
    spec_extra: &'a str,
    /// The status of the spec.
    spec_status: &'a str,
    /// The text of `requirements/valves`.
    valves: &'a str,
    /// Another file of the base commit.
    extra: &'a [(&'a str, &'a str)],
    /// True if `origin/HEAD` names the base commit.
    origin: bool,
    /// More tables of `spec-dna.toml`, after the layers.
    more_config: &'a str,
}

impl Default for Fx<'_> {
    fn default() -> Self {
        Fx {
            profile: "standard",
            rules: "",
            root_extra: "",
            moves_to: "\"requirements\"",
            status: "accepted",
            delta: &[],
            spec_extra: "",
            spec_status: "approved",
            valves: VALVES,
            extra: &[],
            origin: true,
            more_config: "",
        }
    }
}

fn hash(r: &Repo, id: &str) -> String {
    let (code, out, err) = r.run(&["show", id, "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let rest = out.split("\"hash\":\"").nth(1).expect("a hash");
    rest.split('"').next().unwrap().to_string()
}

impl Fx<'_> {
    fn config(&self) -> String {
        format!(
            "profile = \"{}\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n\n[rules]\nSD608 = \"off\"\n{}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n{}\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.contracts]\ntypes = [\"interface\"]\n\n[layers.decisions]\ntypes = [\"decision\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [{}]\nkeep = \"until-finished\"\n{}",
            self.profile, self.rules, self.root_extra, self.moves_to, self.more_config
        )
    }

    /// The base commit on `main`, `origin/HEAD` on it (if `origin`), and the
    /// branch `feature`.
    fn start(&self) -> Repo {
        let r = Repo::with_config(&self.config());
        r.write(VALVES_PATH, self.valves);
        r.write("docs/contracts/http.md", HTTP);
        r.write("src/dose.py", "x = 1\n");
        r.write("tests/test_other.py", "# nothing\n");
        for (p, t) in self.extra {
            r.write(p, t);
        }
        let close = hash(&r, "requirements/valves#close");
        let open = hash(&r, "requirements/valves#open");
        let routine = self.delta.is_empty() && self.spec_extra.is_empty();
        r.write(
            INTENT,
            &format!(
                "---\nstatus: {}\nclass: {}\n---\n# Dose\n\n## Intent\n\nThe dose is too high.\n",
                self.status,
                if routine { "routine" } else { "higher-risk" }
            ),
        );
        r.write(
            PLAN,
            "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/dose.py`\n- `tests/test_valves.py`\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n",
        );
        if !routine {
            let mut body = String::new();
            if !self.delta.is_empty() {
                body.push_str("## Delta\n\n");
                for l in self.delta {
                    body.push_str(&format!(
                        "{}\n",
                        l.replace("{close}", &close).replace("{open}", &open)
                    ));
                }
            }
            body.push_str(self.spec_extra);
            r.write(
                SPEC,
                &format!("---\nstatus: {}\n---\n# Spec\n\n{body}", self.spec_status),
            );
        }
        r.commit("base");
        if self.origin {
            origin_head(&r.dir, "main");
        }
        r.git(&["checkout", "-q", "-b", "feature"]);
        r
    }
}

fn fin(r: &Repo, args: &[&str]) -> (i32, String, String) {
    let mut v = vec!["finish", "dose"];
    v.extend(args);
    r.run(&v)
}

/// Each file of the working tree, with its bytes. The folder `.git` is not
/// part of it.
fn tree(r: &Repo) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for e in fs::read_dir(dir).unwrap() {
            let e = e.unwrap();
            let p = e.path();
            if p.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                out.insert(rel, fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(&r.dir, &r.dir, &mut out);
    out
}

fn check(r: &Repo, base: &str) -> (i32, String) {
    let (code, out, _) = r.run(&["check", "--base", base]);
    (code, out)
}

fn has(out: &str, code: &str) -> bool {
    out.lines().any(|l| {
        l.contains(&format!(": error {code} ")) || l.contains(&format!(": warning {code} "))
    })
}

fn staged(r: &Repo) -> Vec<String> {
    r.git_out(&["diff", "--cached", "--name-status"])
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

fn valves_with(close: &str) -> String {
    VALVES.replace("The controller SHALL close the valve in 5 seconds.", close)
}

// The whole run

#[test]
fn a_finish_applies_the_delta_deletes_the_folder_and_stages_the_paths() {
    let r = Fx {
        delta: &[CHANGE_CLOSE, ADD_FLUSH, REMOVE_OPEN],
        ..Fx::default()
    }
    .start();
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        r.read(VALVES_PATH),
        "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 4 seconds.\n- **flush**: The controller SHALL flush the line before it closes.\n"
    );
    assert!(!r.dir.join(FOLDER).exists(), "the folder stays");
    assert!(out.contains("wrote docs/requirements/valves.md"), "{out}");
    assert!(out.contains(&format!("removed {SPEC}")), "{out}");
    // The command stages the requirement file and the deletions, and it makes
    // no commit.
    assert_eq!(
        staged(&r),
        [
            "D docs/intent/dose/intent.md",
            "D docs/intent/dose/plan.md",
            "D docs/intent/dose/spec.md",
            "M docs/requirements/valves.md",
        ]
    );
    assert_eq!(r.git_out(&["rev-list", "--count", "HEAD"]), "1");
    // The pull request of `finish` passes `check`.
    r.commit("finish");
    let (code, out) = check(&r, "main");
    assert_eq!(code, 0, "{out}");
    for c in ["SD914", "SD915", "SD916", "SD920", "SD921", "SD610"] {
        assert!(!has(&out, c), "{c}: {out}");
    }
}

#[test]
fn a_routine_change_with_a_changed_requirement_needs_evidence() {
    let r = Fx::default().start();
    // The author edits the requirement with the code, as a routine change does.
    r.write(
        VALVES_PATH,
        &valves_with("The controller SHALL close the valve in 4 seconds."),
    );
    let (code, out, _) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains(": error SD916 ") && out.contains("`requirements/valves#close`"),
        "{out}"
    );
    assert!(r.dir.join(FOLDER).exists());
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#close\n",
    );
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(FOLDER).exists());
    // The requirement file is the author's edit: finish applies no delta.
    assert!(r.read(VALVES_PATH).contains("in 4 seconds"));
}

#[test]
fn the_json_report_has_the_documented_keys() {
    let r = Fx {
        delta: &[ADD_FLUSH],
        ..Fx::default()
    }
    .start();
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    let (code, out, err) = fin(&r, &["--dry-run", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    for key in [
        "\"slug\":\"dose\"",
        "\"ok\":true",
        "\"dry_run\":true",
        "\"step\":6",
        "\"diagnostics\":[]",
        "\"action\":\"write\"",
        "\"action\":\"remove\"",
        "\"path\":\"docs/requirements/valves.md\"",
    ] {
        assert!(out.contains(key), "{key}: {out}");
    }
    assert!(out.contains("+- **flush**"), "{out}");
    // A stop has the step, the reason and the diagnostics.
    let r = Fx {
        delta: &[ADD_FLUSH],
        ..Fx::default()
    }
    .start();
    let (code, out, _) = fin(&r, &["--json"]);
    assert_eq!(code, 1, "{out}");
    for key in [
        "\"ok\":false",
        "\"dry_run\":false",
        "\"step\":5",
        "\"code\":\"SD916\"",
        "\"files\":[]",
    ] {
        assert!(out.contains(key), "{key}: {out}");
    }
}

// The dry run and the stops write nothing

#[test]
fn a_dry_run_writes_nothing_and_shows_what_a_run_writes() {
    let fx = Fx {
        delta: &[CHANGE_CLOSE, ADD_FLUSH, REMOVE_OPEN],
        ..Fx::default()
    };
    let r = fx.start();
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    let before = tree(&r);
    let status = r.git_out(&["status", "--porcelain"]);
    let (code, dry, err) = fin(&r, &["--dry-run"]);
    assert_eq!(code, 0, "{dry}{err}");
    assert_eq!(tree(&r), before, "a dry run changed a file");
    assert_eq!(r.git_out(&["status", "--porcelain"]), status);
    assert!(staged(&r).is_empty());
    for p in [INTENT, SPEC, PLAN] {
        assert!(dry.contains(&format!("remove {p}")), "{dry}");
    }
    assert!(dry.contains("--- a/docs/requirements/valves.md"), "{dry}");
    assert!(dry.contains("nothing changed"), "{dry}");
    // The run writes the same files as the dry run names, and each added line
    // of the diff is in the file.
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let new = r.read(VALVES_PATH);
    for l in dry
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
    {
        assert!(new.contains(&l[1..]), "{l}: {new}");
    }
    let mut names: Vec<String> = r
        .git_out(&["diff", "--cached", "--name-only"])
        .lines()
        .map(str::to_string)
        .collect();
    names.sort();
    let mut want: Vec<String> = dry
        .lines()
        .filter_map(|l| l.strip_prefix("remove ").map(str::to_string))
        .collect();
    want.push(VALVES_PATH.to_string());
    want.sort();
    assert_eq!(names, want);
}

#[test]
fn a_stop_at_step_5_leaves_the_tree_equal_and_a_later_run_equals_a_direct_run() {
    let fx = Fx {
        delta: &[CHANGE_CLOSE, ADD_FLUSH],
        ..Fx::default()
    };
    let r = fx.start();
    let before = tree(&r);
    let status = r.git_out(&["status", "--porcelain"]);
    let (code, out, _) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains(": error SD916 "), "{out}");
    assert_eq!(tree(&r), before, "a stop changed a file");
    assert_eq!(r.git_out(&["status", "--porcelain"]), status);
    // The author adds the evidence, and the next run succeeds.
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let later = tree(&r);
    // A direct run, with the evidence from the start.
    let d = fx.start();
    d.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    let (code, out, err) = fin(&d, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(later, tree(&d));
    assert_eq!(staged(&r), staged(&d));
}

// The 3 stops that the owner requires

#[test]
fn a_reference_from_unlocked_text_stops_finish_also_in_lite() {
    for profile in ["standard", "lite"] {
        for (name, path, text) in [
            (
                "link",
                "docs/requirements/pump.md",
                "---\nstatus: active\n---\n# Pump\n\n## Requirements\n\n- **run**: The pump SHALL run. See [the plan](../intent/dose/plan.md).\n",
            ),
            (
                "id",
                "docs/requirements/pump.md",
                "---\nstatus: active\n---\n# Pump\n\n## Requirements\n\n- **run**: The pump SHALL run. See [[intent/dose/plan]].\n",
            ),
            (
                "depends-on",
                "docs/requirements/pump.md",
                "---\nstatus: active\ndepends-on: intent/dose/intent\n---\n# Pump\n\n## Requirements\n\n- **run**: The pump SHALL run.\n",
            ),
            (
                "supersedes",
                "docs/requirements/pump.md",
                "---\nstatus: active\nsupersedes: intent/dose/plan\n---\n# Pump\n\n## Requirements\n\n- **run**: The pump SHALL run.\n",
            ),
            (
                "folder",
                "docs/requirements/pump.md",
                "---\nstatus: active\n---\n# Pump\n\n## Requirements\n\n- **run**: The pump SHALL run. See [the folder](../intent/dose/).\n",
            ),
            (
                "from",
                "docs/intent/other/intent.md",
                "---\nstatus: draft\nclass: routine\nfrom: intent/dose/intent\n---\n# Other\n\n## Intent\n\nText.\n",
            ),
            // The front matter of an accepted decision is not locked.
            (
                "decision front matter",
                "docs/decisions/timer.md",
                "---\nstatus: accepted\ndepends-on: intent/dose/intent\n---\n# Use a timer\n\n## Decision Outcome\n\nUse a timer.\n",
            ),
        ] {
            let r = Fx {
                profile,
                extra: &[(path, text)],
                ..Fx::default()
            }
            .start();
            let before = tree(&r);
            let (code, out, err) = fin(&r, &[]);
            assert_eq!(code, 1, "{profile} {name}: {out}{err}");
            let line = out
                .lines()
                .find(|l| l.starts_with(&format!("{path}:")))
                .unwrap_or_else(|| panic!("{profile} {name}: {out}"));
            assert!(
                line.contains(": error SD101 ") || line.contains(": error SD102 "),
                "{line}"
            );
            assert!(err.contains("step 0"), "{err}");
            assert_eq!(tree(&r), before, "{profile} {name}");
            assert!(r.dir.join(FOLDER).exists());
        }
    }
}

#[test]
fn a_reference_from_the_locked_text_of_a_decision_does_not_stop_finish() {
    let locked = "---\nstatus: accepted\n---\n# Use a timer\n\n## Decision Outcome\n\nUse a timer. It came from [the intent](../intent/dose/intent.md) and [[intent/dose/spec]].\n";
    let r = Fx {
        extra: &[("docs/decisions/timer.md", locked)],
        ..Fx::default()
    }
    .start();
    r.write("src/dose.py", "x = 2\n");
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(FOLDER).exists());
    // The locked text keeps its text, and `check` gives no SD101 or SD102.
    assert_eq!(r.read("docs/decisions/timer.md"), locked);
    r.commit("finish");
    let (code, out) = check(&r, "main");
    assert!(!has(&out, "SD101") && !has(&out, "SD102"), "{out}");
    assert_eq!(code, 0, "{out}");
    // The same reference in an amendment section is editable text: it stops.
    let amended = format!("{locked}\n## Amendments\n\n### Later\n\nSee [[intent/dose/intent]].\n");
    let r = Fx {
        extra: &[("docs/decisions/timer.md", &amended)],
        ..Fx::default()
    }
    .start();
    let (code, out, _) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.lines()
            .any(|l| l.starts_with("docs/decisions/timer.md:") && l.contains("SD101")),
        "{out}"
    );
    assert!(r.dir.join(FOLDER).exists());
}

#[test]
fn a_close_with_finish_deletes_the_folder_and_applies_no_delta() {
    let fx = Fx {
        delta: &[CHANGE_CLOSE, ADD_FLUSH],
        ..Fx::default()
    };
    let r = fx.start();
    // A pull request sets `closed`, and it merges: `main` has the close.
    r.write(
        INTENT,
        &r.read(INTENT).replace("status: accepted", "status: closed"),
    );
    r.commit("close");
    r.git(&["checkout", "-q", "main"]);
    r.git(&["merge", "-q", "--ff-only", "feature"]);
    origin_head(&r.dir, "main");
    // A later pull request runs `finish`.
    r.git(&["checkout", "-q", "-b", "finish"]);
    let valves = r.read(VALVES_PATH);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(FOLDER).exists());
    assert_eq!(r.read(VALVES_PATH), valves, "a close applied the delta");
    assert_eq!(
        staged(&r),
        [
            "D docs/intent/dose/intent.md",
            "D docs/intent/dose/plan.md",
            "D docs/intent/dose/spec.md",
        ]
    );
    r.commit("finish");
    let (code, out) = check(&r, "main");
    assert!(!has(&out, "SD915") && !has(&out, "SD916"), "{out}");
    assert_eq!(code, 0, "{out}");
    // A closed change needs no base: `finish` runs with no `origin/HEAD`.
    let r = Fx {
        status: "closed",
        origin: false,
        delta: &[CHANGE_CLOSE],
        ..Fx::default()
    }
    .start();
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_delta_hash_that_changed_after_the_approval_gives_sd914_and_writes_nothing() {
    let r = Fx {
        delta: &[CHANGE_CLOSE],
        ..Fx::default()
    }
    .start();
    // The requirement changes after the spec was approved.
    r.write(
        VALVES_PATH,
        &valves_with("The controller SHALL close the valve in 6 seconds."),
    );
    r.commit("edit the requirement");
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    let before = tree(&r);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}{err}");
    let line = out
        .lines()
        .find(|l| l.contains(": error SD914 "))
        .unwrap_or_else(|| panic!("{out}"));
    assert!(
        line.starts_with(&format!("{SPEC}:")) && line.contains("requirements/valves#close"),
        "{line}"
    );
    assert!(err.contains("step 3"), "{err}");
    assert_eq!(tree(&r), before);
    assert!(staged(&r).is_empty());
}

#[test]
fn a_delta_edit_with_a_new_approval_commit_in_the_pull_request_of_finish_stops_at_step_3() {
    let r = Fx {
        delta: &[CHANGE_CLOSE],
        ..Fx::default()
    }
    .start();
    // `approval = "branch"`: the pull request sets the spec back to `draft`,
    // and an approver commits the status line alone. A squash merge keeps no
    // commit of the branch, so `finish` still compares with the base tree.
    let config = r.read("spec-dna.toml").replace(
        "gate = \"block\"\n",
        "gate = \"block\"\napproval = \"branch\"\napprovers = [\"approver@example.com\"]\n",
    );
    r.write("spec-dna.toml", &config);
    r.commit("turn the approval in the branch on");
    let edited = r.read(SPEC).replace("in 4 seconds", "in 3 seconds");
    r.write(SPEC, &edited.replace("status: approved", "status: draft"));
    r.commit_as("delta after the spike", "agent@example.com");
    r.write(SPEC, &edited);
    r.commit_as("approve spec again", "approver@example.com");
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    let before = tree(&r);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.contains(": error SD914 ") && out.contains("approved delta"),
        "{out}"
    );
    assert!(err.contains("step 3"), "{err}");
    assert_eq!(tree(&r), before);
}

#[test]
fn a_delta_that_changed_after_it_was_approved_in_the_base_stops_finish() {
    let r = Fx {
        delta: &[CHANGE_CLOSE],
        ..Fx::default()
    }
    .start();
    // The pull request edits the line of the spec, which the base approved.
    r.write(SPEC, &r.read(SPEC).replace("in 4 seconds", "in 3 seconds"));
    r.commit("edit the delta");
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    let before = tree(&r);
    let (code, out, _) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains(": error SD914 ") && out.contains("approved delta"),
        "{out}"
    );
    assert_eq!(tree(&r), before);
    // A change that the base does not have is a change of this branch: there
    // is no approved delta to compare. The plan is new, so the group
    // `acceptance` is off: this test is about the delta.
    let r = Fx {
        rules: "acceptance = \"off\"\n",
        ..Fx::default()
    }
    .start();
    r.git(&["checkout", "-q", "main"]);
    r.git(&["checkout", "-q", "-b", "born"]);
    let hash_close = hash(&r, "requirements/valves#close");
    r.write(
        "docs/intent/dose2/intent.md",
        "---\nstatus: accepted\nclass: higher-risk\n---\n# Dose 2\n\n## Intent\n\nText.\n",
    );
    r.write(
        "docs/intent/dose2/plan.md",
        "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/dose.py`\n",
    );
    r.write(
        "docs/intent/dose2/spec.md",
        &format!(
            "---\nstatus: approved\n---\n# Spec\n\n## Delta\n\n- CHANGE requirements/valves#close@{hash_close}: The controller SHALL close the valve in 4 seconds.\n"
        ),
    );
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#close\n",
    );
    r.commit("a change born in the branch");
    let (code, out, err) = r.run(&["finish", "dose2"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.read(VALVES_PATH).contains("in 4 seconds"));
}

// The stops of the steps 1 to 4

#[test]
fn the_statuses_concerns_questions_and_decisions_stop_before_the_delta() {
    // Step 1.
    let r = Fx {
        status: "draft",
        delta: &[ADD_FLUSH],
        ..Fx::default()
    }
    .start();
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(
        err.contains("step 1") && err.contains("the intent is `draft`"),
        "{err}"
    );
    let r = Fx {
        spec_status: "draft",
        delta: &[ADD_FLUSH],
        ..Fx::default()
    }
    .start();
    let (code, _, err) = fin(&r, &[]);
    assert_eq!(code, 1);
    assert!(err.contains("the spec is `draft`"), "{err}");
    // Step 2: a concern, a question and a decision, whatever the policy and
    // the rules say.
    let r = Fx {
        rules: "SD919 = \"off\"\nSD922 = \"off\"\nSD920 = \"off\"\n",
        spec_extra: "\n## Concerns\n\n- [ ] Is 4 seconds enough?\n\n## Open questions\n\n- [ ] Who tests it?\n\n## Decisions\n\n- [[decisions/timer]]\n",
        extra: &[("docs/decisions/timer.md", &DECISION.replace("{status}", "proposed"))],
        ..Fx::default()
    }
    .start();
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}{err}");
    for c in ["SD919", "SD922", "SD920"] {
        assert!(out.contains(&format!(": error {c} ")), "{c}: {out}");
    }
    assert!(err.contains("step 2"), "{err}");
}

#[test]
fn an_absent_target_and_a_bad_layer_give_sd914() {
    let r = Fx {
        delta: &[
            "- CHANGE requirements/nothing#x@0123456789abcdef: The pump SHALL run.",
            "- REMOVE requirements/valves#nothing@0123456789abcdef",
            "- ADD contracts/http#pong: The service SHALL answer a pong.",
            "- ADD requirements/../escape#x: The pump SHALL run.",
        ],
        ..Fx::default()
    }
    .start();
    let before = tree(&r);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}{err}");
    let n = out.lines().filter(|l| l.contains(": error SD914 ")).count();
    assert_eq!(n, 4, "{out}");
    assert!(out.contains("names no document"), "{out}");
    assert!(out.contains("has no requirement `nothing`"), "{out}");
    // The layer `contracts` is not in `moves-to` of the layer `intent`.
    assert!(out.contains("the layer `contracts`"), "{out}");
    assert!(out.contains("not a document id"), "{out}");
    assert_eq!(tree(&r), before);
    assert!(!r.dir.join("docs/escape.md").exists());
}

#[test]
fn an_interface_that_the_diff_does_not_change_gives_sd914() {
    let fx = Fx {
        moves_to: "\"requirements\", \"contracts\"",
        spec_extra: "## Interfaces\n\n- CHANGE contracts/http: The ping answer has a body.\n",
        ..Fx::default()
    };
    let r = fx.start();
    let before = tree(&r);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.contains(": error SD914 ") && out.contains("contracts/http"),
        "{out}"
    );
    assert_eq!(tree(&r), before);
    // A change of the interface document in the pull request is enough.
    r.write(
        "docs/contracts/http.md",
        &HTTP.replace("# HTTP\n", "# HTTP\n\nThe service speaks plain HTTP.\n"),
    );
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    // A file that the interface document cites is also enough.
    let cited = "---\nstatus: active\n---\n# HTTP\n\nThe code is in `src/http.py`.\n\n## Requirements\n\n- **ping**: The service SHALL answer a ping.\n";
    let r = Fx {
        extra: &[
            ("docs/contracts/http.md", cited),
            ("src/http.py", "a = 1\n"),
        ],
        ..fx
    }
    .start();
    r.write("src/http.py", "a = 2\n");
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
}

const EVENTS: &str = "---\nstatus: active\n---\n# Events\n\n## Requirements\n\n- **emit**: The interface SHALL emit one event for a change.\n";

#[test]
fn an_add_item_of_interfaces_needs_its_document_or_a_delta_line_that_adds_it() {
    let fx = Fx {
        moves_to: "\"requirements\", \"contracts\"",
        spec_extra: "## Interfaces\n\n- ADD contracts/events: add the event contract.\n",
        ..Fx::default()
    };
    // No document and no delta line: step 3 stops, and nothing changes.
    let r = fx.start();
    let before = tree(&r);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.contains(
            ": error SD914 "
        ) && out.contains(
            "the spec adds the interface `contracts/events`, and the tree has no such document; write it in this pull request, or add its requirements with `ADD contracts/events#<key>` lines"
        ),
        "{out}"
    );
    assert_eq!(tree(&r), before);
    // The document is in the pull request.
    r.write("docs/contracts/events.md", EVENTS);
    r.write("tests/test_events.py", "# covers: contracts/events#emit\n");
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");

    // A delta line that adds the document is also enough. `finish` writes it.
    let r = Fx {
        delta: &["- ADD contracts/events#emit: The interface SHALL emit one event for a change."],
        extra: &[("tests/test_events.py", "# covers: contracts/events#emit\n")],
        ..fx
    }
    .start();
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.dir.join("docs/contracts/events.md").exists());
    // The pull request of `finish` deletes the folder. `check` reads the spec
    // of the base, and it finds the document in the head.
    r.commit("finish dose");
    let (_, out) = check(&r, "main");
    assert!(!has(&out, "SD914"), "{out}");
}

#[test]
fn an_added_requirement_in_a_new_document_with_a_required_section_gives_sd305_only_after() {
    let new = "- ADD requirements/pump/priming#prime: The pump SHALL prime for 4 seconds.";
    let r = Fx {
        delta: &[new],
        root_extra: "required-sections = { requirements = [[\"Requirements\"], [\"Rationale\"]] }\n",
        ..Fx::default()
    }
    .start();
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/pump/priming#prime\n",
    );
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        r.read("docs/requirements/pump/priming.md"),
        "---\nstatus: active\n---\n# Priming\n\n## Requirements\n\n- **prime**: The pump SHALL prime for 4 seconds.\n"
    );
    // The author adds the missing section: `check` names it.
    r.commit("finish");
    let (_, out) = check(&r, "main");
    assert!(
        out.lines()
            .any(|l| l.starts_with("docs/requirements/pump/priming.md:") && l.contains("SD305")),
        "{out}"
    );
}

// The preconditions

#[test]
fn an_untracked_file_in_the_folder_and_a_dirty_tree_stop_finish() {
    let r = Fx::default().start();
    r.write("docs/intent/dose/notes.md", "# Notes\n");
    let before = tree(&r);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("notes.md") && err.contains("step 0"), "{err}");
    assert_eq!(tree(&r), before);
    // A staged file that no commit holds.
    r.git(&["add", "docs/intent/dose/notes.md"]);
    let (code, _, err) = fin(&r, &[]);
    assert_eq!(code, 1, "{err}");
    r.commit("notes");
    // An edit of a file of the folder that is not committed.
    r.write(PLAN, &format!("{}\nMore.\n", r.read(PLAN)));
    let (code, _, err) = fin(&r, &[]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("plan.md"), "{err}");
    assert!(r.dir.join(FOLDER).exists());
    // A file outside the folder can be dirty.
    r.git(&["checkout", "--", PLAN]);
    r.write("src/dose.py", "x = 3\n");
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_exit_codes() {
    let r = Fx {
        delta: &[ADD_FLUSH],
        ..Fx::default()
    }
    .start();
    // 2: no slug, a slug with a path.
    for args in [
        &["finish"][..],
        &["finish", "../x"],
        &["finish", "a/b"],
        &["finish", "a", "b"],
    ] {
        let (code, _, err) = r.run(args);
        assert_eq!(code, 2, "{args:?}: {err}");
    }
    // 5: no folder.
    let (code, _, err) = r.run(&["finish", "nothing"]);
    assert_eq!(code, 5, "{err}");
    // 3: a bad base.
    let (code, _, err) = fin(&r, &["--base", "no-such-revision"]);
    assert_eq!(code, 3, "{err}");
    // 3: a different pin.
    let toml = r.read("spec-dna.toml");
    r.write("spec-dna.toml", &toml.replace(VERSION, "9.9.9"));
    let (code, _, err) = fin(&r, &[]);
    assert_eq!(code, 3, "{err}");
    r.write("spec-dna.toml", &toml);
    assert!(r.dir.join(FOLDER).exists());
    // 3: an accepted change and no base.
    let r = Fx {
        origin: false,
        delta: &[ADD_FLUSH],
        ..Fx::default()
    }
    .start();
    let (code, _, err) = fin(&r, &[]);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("no base for the change checks"), "{err}");
    // `--base` names the base in place of `origin/HEAD`.
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    let (code, out, err) = fin(&r, &["--base", "main"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn finish_stops_under_the_policy_none_too() {
    // The policy `none` turns the change codes off. `finish` still checks.
    let r = Fx {
        delta: &[ADD_FLUSH],
        ..Fx::default()
    }
    .start();
    let toml = r
        .read("spec-dna.toml")
        .replace("policy = \"three-class\"", "policy = \"none\"")
        .replace("gate = \"block\"\n", "");
    r.write("spec-dna.toml", &toml);
    r.commit("none");
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains(": error SD916 "), "{out}");
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
}

// The edits

#[test]
fn a_change_keeps_the_key_the_markers_and_the_nested_list() {
    let valves = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds. <!-- aliases: old/valves#close -->\n  - A nested note that stays.\n- **open**: The controller SHALL open the valve in 5 seconds. <!-- verified-by: tests/test_other.py -->\n";
    let r = Fx {
        valves,
        delta: &[
            CHANGE_CLOSE,
            "- CHANGE requirements/valves#open@{open}: The controller SHALL open the valve in 3 seconds.",
        ],
        ..Fx::default()
    }
    .start();
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#close\n",
    );
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        r.read(VALVES_PATH),
        "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 4 seconds. <!-- aliases: old/valves#close -->\n  - A nested note that stays.\n- **open**: The controller SHALL open the valve in 3 seconds. <!-- verified-by: tests/test_other.py -->\n"
    );
}

#[test]
fn an_add_follows_the_list_of_the_document_bulleted_or_ordered() {
    let ordered = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n3. **close**: The controller SHALL close the valve in 5 seconds.\n4. **open**: The controller SHALL open the valve in 5 seconds.\n\n## Notes\n\nText.\n";
    let r = Fx {
        valves: ordered,
        delta: &[
            ADD_FLUSH,
            "- ADD requirements/valves#drain: The controller SHALL drain the line.",
        ],
        ..Fx::default()
    }
    .start();
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#flush, requirements/valves#drain\n",
    );
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        r.read(VALVES_PATH),
        ordered.replace(
            "open the valve in 5 seconds.\n",
            "open the valve in 5 seconds.\n5. **flush**: The controller SHALL flush the line before it closes.\n6. **drain**: The controller SHALL drain the line.\n"
        )
    );
}

#[test]
fn a_remove_deletes_the_item_and_a_reference_to_it_stops_finish() {
    let r = Fx {
        delta: &[REMOVE_OPEN],
        ..Fx::default()
    }
    .start();
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        r.read(VALVES_PATH),
        "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n"
    );
    // A reference to the removed requirement would break: the result adds an
    // error, and nothing changes.
    let pump = "---\nstatus: active\n---\n# Pump\n\n## Requirements\n\n- **run**: The pump SHALL run when [[requirements/valves#open]] is true.\n";
    let r = Fx {
        delta: &[REMOVE_OPEN],
        extra: &[("docs/requirements/pump.md", pump)],
        ..Fx::default()
    }
    .start();
    let before = tree(&r);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.contains(": error SD101 ") && err.contains("step 5"),
        "{out}{err}"
    );
    assert_eq!(tree(&r), before);
}

#[test]
fn crlf_and_a_bom_stay() {
    let valves = format!("\u{feff}{}", VALVES.replace('\n', "\r\n"));
    let r = Fx {
        valves: &valves,
        delta: &[CHANGE_CLOSE, ADD_FLUSH, REMOVE_OPEN],
        ..Fx::default()
    }
    .start();
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let new = fs::read(r.dir.join(VALVES_PATH)).unwrap();
    let want = "\u{feff}---\r\nstatus: active\r\n---\r\n# Valves\r\n\r\n## Requirements\r\n\r\n- **close**: The controller SHALL close the valve in 4 seconds.\r\n- **flush**: The controller SHALL flush the line before it closes.\r\n";
    assert_eq!(String::from_utf8(new).unwrap(), want);
}

#[test]
fn a_file_without_a_final_newline_stays_without_one() {
    let valves = VALVES.trim_end();
    let r = Fx {
        valves,
        delta: &[ADD_FLUSH],
        ..Fx::default()
    }
    .start();
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    let new = r.read(VALVES_PATH);
    assert!(
        new.ends_with("\n- **flush**: The controller SHALL flush the line before it closes.")
            && !new.ends_with('\n'),
        "{new:?}"
    );
}

#[test]
fn a_relative_link_of_a_delta_sentence_follows_to_the_target_path() {
    // A link that resolves from the spec moves to the path from the target.
    let r = Fx {
        delta: &[
            "- ADD requirements/valves#flush: The controller SHALL flush as [the contract](../../contracts/http.md#requirements) says.",
        ],
        ..Fx::default()
    }
    .start();
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        r.read(VALVES_PATH)
            .contains("[the contract](../contracts/http.md#requirements)"),
        "{}",
        r.read(VALVES_PATH)
    );
}

// A generated index that lists the folder

#[test]
fn a_generated_index_that_lists_the_folder_is_written_again() {
    let r = Fx {
        more_config: "\n[layers.research]\ntypes = [\"research\"]\n",
        extra: &[(
            "docs/intent/README.md",
            "---\ntype: research\n---\n# Changes\n\nA hand-kept list.\n",
        )],
        ..Fx::default()
    }
    .start();
    let (code, out, err) = r.run(&["index", "docs/intent", "--replace"]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("index");
    r.git(&["checkout", "-q", "main"]);
    r.git(&["merge", "-q", "--ff-only", "feature"]);
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "finish"]);
    let (_, out) = check(&r, "main");
    assert!(!has(&out, "SD610"), "{out}");
    r.write("src/dose.py", "x = 3\n");
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("wrote docs/intent/README.md"), "{out}");
    assert!(!r.read("docs/intent/README.md").contains("dose"));
    r.commit("finish");
    let (code, out) = check(&r, "main");
    assert!(!has(&out, "SD610") && !has(&out, "SD102"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn the_refresh_of_a_generated_index_writes_the_status_of_its_place() {
    // An index of the folder `requirements` with no status and an old hash. The
    // change adds a document to the folder, so `finish` writes the index again.
    let r = Fx {
        delta: &[ADD_PUMP],
        extra: &[(
            "docs/requirements/README.md",
            "---\ngenerated: spec-dna index h:0000000000000000\n---\n# Requirements\n\n| Entry | Title | Status |\n| --- | --- | --- |\n| [valves](valves.md) | Valves | active |\n",
        )],
        ..Fx::default()
    }
    .start();
    r.write("tests/test_pump.py", "# covers: requirements/pump#start\n");
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("wrote docs/requirements/README.md"), "{out}");
    let index = r.read("docs/requirements/README.md");
    assert!(
        index.starts_with("---\nstatus: draft\ngenerated: spec-dna index h:"),
        "{index}"
    );
    assert!(index.contains("pump.md"), "{index}");
}

// The merge kinds

/// The finish pull request on the branch `finish`, and a `main` that moved.
fn finished_branch() -> (Repo, String) {
    let r = Fx {
        delta: &[CHANGE_CLOSE, ADD_FLUSH],
        ..Fx::default()
    }
    .start();
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("finish");
    let base = r.git_out(&["rev-parse", "main"]);
    r.git(&["checkout", "-q", "main"]);
    r.write("main-side.txt", "m\n");
    r.commit("main side");
    (r, base)
}

fn after_merge(r: &Repo, base: &str) {
    assert!(!r.dir.join(FOLDER).exists());
    assert!(r.read(VALVES_PATH).contains("in 4 seconds"));
    assert!(r.read(VALVES_PATH).contains("**flush**"));
    let (code, out) = check(r, base);
    assert_eq!(code, 0, "{out}");
    let (code, out, _) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}");
    // The deletion of the intent shows in the history once.
    let deleted = r.git_out(&["log", "--format=%H", "--diff-filter=D", "--", INTENT]);
    assert_eq!(deleted.lines().count(), 1, "{deleted}");
}

#[test]
fn a_squash_merge_of_the_finish_branch_checks_clean() {
    let (r, base) = finished_branch();
    r.git(&["merge", "-q", "--squash", "feature"]);
    r.commit("squash");
    after_merge(&r, &base);
}

#[test]
fn a_merge_commit_of_the_finish_branch_checks_clean() {
    let (r, base) = finished_branch();
    r.git(&["merge", "-q", "--no-ff", "-m", "merge", "feature"]);
    after_merge(&r, &base);
}

#[test]
fn a_rebase_merge_of_the_finish_branch_checks_clean() {
    let (r, base) = finished_branch();
    r.git(&["checkout", "-q", "feature"]);
    r.git(&["rebase", "-q", "main"]);
    r.git(&["checkout", "-q", "main"]);
    r.git(&["merge", "-q", "--ff-only", "feature"]);
    after_merge(&r, &base);
}

// More cases

#[test]
fn a_file_of_another_kind_in_the_folder_is_deleted_and_a_link_to_it_stops_finish() {
    let note = "---\nstatus: active\n---\n# Pump\n\n## Requirements\n\n- **run**: The pump SHALL run.\n\nSee ![the figure](../intent/dose/figure.png).\n";
    let r = Fx {
        extra: &[
            ("docs/intent/dose/figure.png", "not a picture\n"),
            ("docs/requirements/pump.md", note),
        ],
        ..Fx::default()
    }
    .start();
    let before = tree(&r);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.lines()
            .any(|l| l.starts_with("docs/requirements/pump.md:") && l.contains("SD102")),
        "{out}"
    );
    assert_eq!(tree(&r), before);
    // Without the link, `finish` deletes each tracked file of the folder.
    r.write(
        "docs/requirements/pump.md",
        &note.replace("\nSee ![the figure](../intent/dose/figure.png).\n", ""),
    );
    r.commit("remove the link");
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("removed docs/intent/dose/figure.png"), "{out}");
    assert!(!r.dir.join(FOLDER).exists());
}

#[test]
fn an_accepted_decision_of_the_spec_lets_finish_pass() {
    let r = Fx {
        spec_extra: "## Decisions\n\n- [[decisions/timer]]\n",
        extra: &[(
            "docs/decisions/timer.md",
            &DECISION.replace("{status}", "accepted"),
        )],
        ..Fx::default()
    }
    .start();
    r.write("src/dose.py", "x = 2\n");
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(FOLDER).exists());
    // The decision stays as it was.
    assert_eq!(
        r.read("docs/decisions/timer.md"),
        DECISION.replace("{status}", "accepted")
    );
}

#[test]
fn a_target_in_the_folder_of_the_change_gives_sd914() {
    let r = Fx {
        delta: &["- ADD intent/dose/notes#k: The pump SHALL run."],
        ..Fx::default()
    }
    .start();
    let before = tree(&r);
    let (code, out, _) = fin(&r, &[]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains(": error SD914 ") && out.contains("that `finish` deletes"),
        "{out}"
    );
    assert_eq!(tree(&r), before);
}

#[cfg(unix)]
#[test]
fn a_write_error_exits_with_4_and_changes_nothing() {
    use std::os::unix::fs::PermissionsExt;
    // The mode of the folder goes back before the repository is removed.
    struct Restore(std::path::PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
        }
    }
    let r = Fx {
        delta: &[CHANGE_CLOSE, ADD_FLUSH],
        ..Fx::default()
    }
    .start();
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    let dir = r.dir.join("docs/requirements");
    let _restore = Restore(dir.clone());
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    // The test has no purpose for a user that can write there (root).
    if fs::write(dir.join("probe"), "").is_ok() {
        return;
    }
    let before = tree(&r);
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 4, "{out}{err}");
    assert!(
        err.contains("docs/requirements/valves.md") && err.contains("nothing changed"),
        "{err}"
    );
    // No file changed, and no temporary file is left.
    assert_eq!(tree(&r), before);
    assert!(staged(&r).is_empty());
}

// The hours of the change

/// The plan of `Fx`, with the tasks `order` in `Order` and the section `extra`
/// after `Order`.
fn plan_with_tasks(order: &str, extra: &str) -> String {
    format!(
        "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/dose.py`\n- `tests/test_valves.py`\n\n## Order\n\n{order}\n\n{extra}## Risks\n\n- None.\n\n## Proof\n\n- A test.\n"
    )
}

/// A fixture whose plan has the tasks `order`, committed on `feature`.
fn with_tasks(order: &str) -> Repo {
    let r = Fx::default().start();
    r.write(PLAN, &plan_with_tasks(order, ""));
    r.commit("tasks");
    r
}

/// The task line with the label `Hours:` or `Skip:` that `labels` gives.
fn task(id: &str, done: bool, labels: &str) -> String {
    format!(
        "- [{}] {id}: Do it. Files: `src/dose.py`. Verify: `pytest`. {labels}",
        if done { "x" } else { " " }
    )
}

fn hours_line(out: &str) -> Option<&str> {
    out.lines().find(|l| l.starts_with("Hours:"))
}

#[test]
fn finish_adds_the_hours_of_the_tasks() {
    let order = format!(
        "{}\n{}",
        task("t1", true, "Hours: 1.5."),
        task("t2", true, "Hours: 2.")
    );
    let r = with_tasks(&order);
    // A dry run prints the line, and it writes nothing.
    let (code, out, err) = fin(&r, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.lines().any(|l| l == "Hours: 3.5"), "{out}");
    assert!(
        out.contains(
            "add this line to the commit message of the change; keep it in the squash message"
        ),
        "{out}"
    );
    assert!(!out.contains("note:"), "{out}");
    let (code, out, err) = fin(&r, &["--dry-run", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("\"hours\":{\"low\":3.5,\"high\":3.5}"),
        "{out}"
    );
    assert_eq!(out.trim_end().lines().count(), 1, "{out}");
    // A run prints it too. `finish` makes no commit.
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.lines().any(|l| l == "Hours: 3.5"), "{out}");
    assert_eq!(r.git_out(&["rev-list", "--count", "HEAD"]), "2");
    // The total has no zero after the last digit, a number can end with `h`,
    // and the sum of 0.1 and 0.2 is exact.
    let order = format!(
        "{}\n{}\n{}",
        task("t1", true, "Hours: 1.25h."),
        task("t2", true, "Hours: 0.75."),
        task("t3", true, "Hours: 1.")
    );
    let (_, out, _) = fin(&with_tasks(&order), &["--dry-run"]);
    assert!(out.lines().any(|l| l == "Hours: 3"), "{out}");
    let order = format!(
        "{}\n{}",
        task("t1", true, "Hours: 0.1."),
        task("t2", true, "Hours: 0.2.")
    );
    let (_, out, _) = fin(&with_tasks(&order), &["--dry-run"]);
    assert!(out.lines().any(|l| l == "Hours: 0.3"), "{out}");
    // The labels have a free order, and a duplicate id counts once.
    let order = format!(
        "- [x] t1: Do it. Hours: 2. Files: `src/dose.py`. Verify: `pytest`.\n{}",
        task("t1", true, "Hours: 5.")
    );
    let (_, out, _) = fin(&with_tasks(&order), &["--dry-run"]);
    assert!(out.lines().any(|l| l == "Hours: 2"), "{out}");
}

#[test]
fn a_range_of_hours_gives_a_range_total() {
    let order = format!(
        "{}\n{}\n{}",
        task("t1", true, "Hours: 1-2."),
        task("t2", true, "Hours: 1.5."),
        task("t3", true, "Hours: 0 - 0.5h.")
    );
    let r = with_tasks(&order);
    let (code, out, err) = fin(&r, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(hours_line(&out), Some("Hours: 2.5-4"), "{out}");
    let (_, json, _) = fin(&r, &["--dry-run", "--json"]);
    assert!(
        json.contains("\"hours\":{\"low\":2.5,\"high\":4}"),
        "{json}"
    );
    // A range with two equal numbers is a number.
    let (_, out, _) = fin(
        &with_tasks(&task("t1", true, "Hours: 3-3.")),
        &["--dry-run"],
    );
    assert_eq!(hours_line(&out), Some("Hours: 3"), "{out}");
}

#[test]
fn a_skipped_task_adds_no_hours() {
    let order = format!(
        "{}\n{}\n{}",
        task("t1", true, "Hours: 1-2."),
        task("t2", true, "Hours: 1.5."),
        task("t3", true, "Hours: 4. Skip: the store keeps the old alert.")
    );
    let r = with_tasks(&order);
    let (code, out, err) = fin(&r, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(hours_line(&out), Some("Hours: 2.5-3.5"), "{out}");
    assert!(!out.contains("note:"), "{out}");
    // The box of a skipped task does not matter, and a skipped task with no
    // `Hours:` gives no note.
    let order = format!(
        "{}\n{}",
        task("t1", true, "Hours: 2."),
        task("t2", false, "Skip: not needed.")
    );
    let (_, out, _) = fin(&with_tasks(&order), &["--dry-run"]);
    assert_eq!(hours_line(&out), Some("Hours: 2"), "{out}");
    assert!(!out.contains("note:"), "{out}");
    // A task with an empty `Skip:` is not skipped.
    let order = format!(
        "{}\n{}",
        task("t1", true, "Hours: 2."),
        task("t2", true, "Hours: 1. Skip:")
    );
    let (_, out, _) = fin(&with_tasks(&order), &["--dry-run"]);
    assert_eq!(hours_line(&out), Some("Hours: 3"), "{out}");
}

#[test]
fn a_task_with_no_hours_gives_a_note_and_the_total_of_the_others() {
    let order = format!(
        "{}\n{}\n{}\n{}",
        task("t1", true, "Hours: 2."),
        task("t2", true, ""),
        task("t3", true, "Hours: 1."),
        task("t4", true, "")
    );
    let r = with_tasks(&order);
    let (code, out, err) = fin(&r, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("note: the tasks `t2`, `t4` have no `Hours:`; the total leaves them out\n"),
        "{out}"
    );
    assert_eq!(hours_line(&out), Some("Hours: 3"), "{out}");
    let (_, json, _) = fin(&r, &["--dry-run", "--json"]);
    assert!(json.contains("\"hours\":{\"low\":3,\"high\":3}"), "{json}");
    // One task with no `Hours:` gives the singular form.
    let order = format!(
        "{}\n{}",
        task("t1", true, "Hours: 2."),
        task("t2", true, "")
    );
    let (_, out, _) = fin(&with_tasks(&order), &["--dry-run"]);
    assert!(
        out.contains("note: the task `t2` has no `Hours:`; the total leaves it out\n"),
        "{out}"
    );
    // A plan with tasks and no `Hours:` at all prints no line and no note.
    let (_, out, _) = fin(&with_tasks(&task("t1", true, "")), &["--dry-run"]);
    assert!(!out.contains("Hours"), "{out}");
    assert!(!out.contains("note:"), "{out}");
    let (_, json, _) = fin(&with_tasks(&task("t1", true, "")), &["--dry-run", "--json"]);
    assert!(json.contains("\"hours\":null"), "{json}");
}

#[test]
fn an_invalid_hours_value_gives_a_note_and_no_total() {
    let order = format!(
        "{}\n{}\n{}\n{}\n{}",
        task("t1", true, "Hours: 2."),
        task("t2", true, "Hours: soon."),
        task("t3", true, "Hours: 3-1."),
        task("t4", true, ""),
        task("t5", true, "Hours: 12345.")
    );
    let r = with_tasks(&order);
    let (code, out, err) = fin(&r, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    for (id, value) in [("t2", "soon"), ("t3", "3-1"), ("t5", "12345")] {
        assert!(
            out.contains(&format!(
                "note: the task `{id}` has the value \"{value}\" in `Hours:`, which is no number of hours and no range; no total\n"
            )),
            "{out}"
        );
    }
    assert!(hours_line(&out).is_none(), "{out}");
    // An invalid value leaves out the note of the missing hours.
    assert!(!out.contains("have no `Hours:`"), "{out}");
    let (code, json, _) = fin(&r, &["--dry-run", "--json"]);
    assert_eq!(code, 0, "{json}");
    assert!(json.contains("\"hours\":null"), "{json}");
    // A skipped task with a bad value is not counted.
    let order = format!(
        "{}\n{}",
        task("t1", true, "Hours: 2."),
        task("t2", true, "Hours: soon. Skip: no.")
    );
    let (_, out, _) = fin(&with_tasks(&order), &["--dry-run"]);
    assert_eq!(hours_line(&out), Some("Hours: 2"), "{out}");
    assert!(!out.contains("note:"), "{out}");
}

#[test]
fn the_section_hours_is_not_read() {
    let r = Fx::default().start();
    r.write(
        PLAN,
        &plan_with_tasks(
            &task("t1", true, ""),
            "## Hours\n\n- 1.5: code\n- 2: review\n\n",
        ),
    );
    r.commit("hours section");
    let (code, out, err) = fin(&r, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("Hours:"), "{out}");
    assert!(!out.contains("note:"), "{out}");
    let (_, json, _) = fin(&r, &["--dry-run", "--json"]);
    assert!(json.contains("\"hours\":null"), "{json}");
}

#[test]
fn finish_json_gives_hours_as_low_and_high() {
    let r = Fx::default().start();
    let (_, json, _) = fin(&r, &["--dry-run", "--json"]);
    assert!(json.contains("\"hours\":null"), "{json}");
    let order = format!(
        "{}\n{}",
        task("t1", true, "Hours: 1-2."),
        task("t2", true, "Hours: 0.25.")
    );
    let (code, json, err) = fin(&with_tasks(&order), &["--dry-run", "--json"]);
    assert_eq!(code, 0, "{json}{err}");
    assert!(
        json.trim_end()
            .ends_with("\"hours\":{\"low\":1.25,\"high\":2.25},\"actual\":null}"),
        "{json}"
    );
}

// Open tasks (0.7.1, item 1)

const PLAN_OPEN: &str = "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/dose.py`\n- `tests/test_valves.py`\n\n## Order\n\n- [x] t1: Edit the code.\n- [ ] t2: Test the code.\n- [ ] t3: Write the docs.\n- [ ] t4: Done by hand. Skip: not needed.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n";

#[test]
fn an_open_task_stops_finish_at_step_0_and_lists_the_open_tasks() {
    for dry in [true, false] {
        let r = Fx {
            delta: &[CHANGE_CLOSE, ADD_FLUSH],
            ..Fx::default()
        }
        .start();
        r.write(PLAN, PLAN_OPEN);
        r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
        r.commit("work");
        let before = tree(&r);
        let args: &[&str] = if dry { &["--dry-run"] } else { &[] };
        let (code, out, err) = fin(&r, args);
        assert_eq!(code, 1, "{out}{err}");
        assert!(err.contains("stopped at step 0"), "{err}");
        assert!(err.contains("2 open task(s)"), "{err}");
        assert!(err.contains("t2: Test the code"), "{err}");
        assert!(err.contains("t3: Write the docs"), "{err}");
        assert!(!err.contains("t1"), "a done task is listed: {err}");
        assert!(!err.contains("t4"), "a skipped task is listed: {err}");
        assert_eq!(tree(&r), before);
        let mut json_args = vec!["--json"];
        json_args.extend(args);
        let (code, out, _) = fin(&r, &json_args);
        assert_eq!(code, 1, "{out}");
        assert!(out.contains("\"step\":0"), "{out}");
        assert!(out.contains("t3: Write the docs"), "{out}");
    }
    // The tasks are closed: `finish` runs.
    let r = Fx {
        delta: &[CHANGE_CLOSE, ADD_FLUSH],
        ..Fx::default()
    }
    .start();
    r.write(
        PLAN,
        &PLAN_OPEN.replace("- [ ] t2", "- [x] t2").replace(
            "- [ ] t3: Write the docs.",
            "- [ ] t3: Write the docs. Skip: no docs.",
        ),
    );
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    r.commit("work");
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_closed_change_with_an_open_task_is_deleted_by_finish() {
    let r = Fx {
        status: "closed",
        delta: &[CHANGE_CLOSE],
        ..Fx::default()
    }
    .start();
    r.write(PLAN, PLAN_OPEN);
    r.commit("work");
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(FOLDER).exists());
}

#[test]
fn the_stop_for_open_tasks_names_the_id_of_each_open_task() {
    let r = Fx::default().start();
    let tasks: Vec<String> = (1..=14)
        .map(|n| format!("- [ ] t{n}: Task number {n}."))
        .collect();
    r.write(PLAN, &PLAN_OPEN.replace("- [x] t1: Edit the code.\n- [ ] t2: Test the code.\n- [ ] t3: Write the docs.\n- [ ] t4: Done by hand. Skip: not needed.", &tasks.join("\n")));
    r.commit("work");
    let (code, _, err) = fin(&r, &[]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("14 open task(s)"), "{err}");
    assert!(err.contains("t10: Task number 10"), "{err}");
    assert!(err.contains("also open: t11, t12, t13, t14"), "{err}");
}

/// The order of the guide: the code is in the branch with the task `Review by
/// the owner` open. After the review, a commit ticks the task. Then `finish`
/// runs, and its commit passes `check`.
#[test]
fn finish_runs_after_a_commit_that_ticks_the_review_task_of_the_owner() {
    let r = Fx {
        delta: &[CHANGE_CLOSE, ADD_FLUSH],
        ..Fx::default()
    }
    .start();
    let review = "- [ ] t2: Review by the owner. Files: none. Verify: the owner reviewed the code of the pull request. Hours: 1.";
    let plan = PLAN_OPEN
        .replace("- [ ] t2: Test the code.", review)
        .replace("- [ ] t3: Write the docs.\n", "")
        .replace("- [ ] t4: Done by hand. Skip: not needed.\n", "");
    r.write(PLAN, &plan);
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    r.commit("work");
    // Before the review, `finish` stops at step 0 and names the task.
    let (code, out, err) = fin(&r, &["--dry-run"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("t2: Review by the owner"), "{err}");
    // The review: a commit ticks the task, with its real hours.
    r.write(
        PLAN,
        &plan.replace(
            "- [ ] t2: Review by the owner. Files: none. Verify: the owner reviewed the code of the pull request. Hours: 1.",
            "- [x] t2: Review by the owner. Files: none. Verify: the owner reviewed the code of the pull request. Hours: 2.",
        ),
    );
    r.commit("review");
    let (code, out, err) = fin(&r, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("Hours: 2"), "{out}");
    r.commit("finish");
    let (code, out) = check(&r, "main");
    assert_eq!(code, 0, "{out}");
}

// A task that closes with `finish` (0.7.1, B1)

#[test]
fn a_task_that_names_finish_with_its_slug_closes_with_finish() {
    // `finish` never waits for a task that only `finish` can do.
    let order = format!(
        "{}\n{}",
        task("t1", true, "Hours: 1."),
        "- [ ] t2: Run `spec-dna finish dose` and commit the result. Files: none. Verify: `spec-dna check`. Hours: 0.5."
    );
    let r = with_tasks(&order);
    let (code, out, err) = fin(&r, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    // The estimate of the task counts.
    assert_eq!(hours_line(&out), Some("Hours: 1.5"), "{out}");
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(FOLDER).exists());
    // The slug ends the text, or a character that is no part of a slug follows.
    for text in [
        "Run `spec-dna finish dose`",
        "Run spec-dna finish dose",
        "Run `spec-dna finish dose --dry-run`, then commit",
        "Run spec-dna finish dose, then commit",
    ] {
        let order = format!("- [ ] t1: {text}. Files: none. Verify: read the diff.");
        let (code, out, err) = fin(&with_tasks(&order), &["--dry-run"]);
        assert_eq!(code, 0, "{text}: {out}{err}");
    }
}

#[test]
fn a_task_that_names_finish_without_its_slug_stops_at_step_0_with_the_hint() {
    for text in [
        "Run `spec-dna finish` and commit the result",
        "Run `spec-dna finish other` and commit the result",
        "Run `spec-dna finish dose-2` and commit the result",
        "Run `spec-dna finish dose_x` and commit the result",
    ] {
        let order = format!(
            "{}\n- [ ] t2: {text}. Files: none. Verify: read the diff.",
            task("t1", true, "")
        );
        let r = with_tasks(&order);
        let before = tree(&r);
        let (code, out, err) = fin(&r, &["--dry-run"]);
        assert_eq!(code, 1, "{text}: {out}{err}");
        assert!(err.contains("stopped at step 0"), "{text}: {err}");
        assert!(err.contains("1 open task(s)"), "{text}: {err}");
        assert!(
            err.contains(
                "a task closes with `finish` only when its text names `spec-dna finish dose`"
            ),
            "{text}: {err}"
        );
        assert_eq!(tree(&r), before);
    }
    // A task of real work gets no hint, and a task that does not name `finish`
    // gets none.
    let (code, _, err) = fin(
        &with_tasks("- [ ] t1: Write the docs. Files: none. Verify: read them."),
        &["--dry-run"],
    );
    assert_eq!(code, 1, "{err}");
    assert!(!err.contains("a task closes with `finish`"), "{err}");
}

#[test]
fn a_task_that_closes_with_finish_does_not_hide_another_open_task() {
    let order = format!(
        "{}\n{}\n{}",
        task("t1", false, ""),
        "- [ ] t2: Run `spec-dna finish dose`. Files: none. Verify: read the diff.",
        task("t3", false, "")
    );
    let (code, _, err) = fin(&with_tasks(&order), &["--dry-run"]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("2 open task(s)"), "{err}");
    assert!(!err.contains("t2"), "{err}");
}

/// The task of the review by the owner, as the skill `write-plan` gives it.
fn review_task_of_the_skill(r: &Repo, id: &str, box_: &str, hours: &str) -> String {
    let (code, out, err) = r.run(&["skill", "show", "write-plan"]);
    assert_eq!(code, 0, "{out}{err}");
    let example = out
        .split('`')
        .find(|s| s.contains("Review by the owner"))
        .unwrap_or_else(|| panic!("no review task in the skill\n{out}"));
    example
        .replacen("- [ ]", &format!("- [{box_}]"), 1)
        .replace("tN", id)
        .replace("Hours: <n>.", hours)
}

#[test]
fn finish_runs_after_a_person_ticks_the_review_task_of_the_skill() {
    let r = Fx {
        delta: &[CHANGE_CLOSE, ADD_FLUSH],
        ..Fx::default()
    }
    .start();
    let open = review_task_of_the_skill(&r, "t2", " ", "Hours: 1.");
    let plan = |review: &str| {
        plan_with_tasks(
            &format!("{}\n{review}", task("t1", true, "Hours: 2. Actual: 3.")),
            "",
        )
    };
    r.write(PLAN, &plan(&open));
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    r.commit("work");
    // The owner has not reviewed: `finish` stops at step 0 and names the task.
    let (code, out, err) = fin(&r, &["--dry-run"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(err.contains("t2: Review by the owner"), "{err}");
    // A person ticks it with the real hours.
    let done = review_task_of_the_skill(&r, "t2", "x", "Hours: 1. Actual: 0.5.");
    r.write(PLAN, &plan(&done));
    r.commit("review");
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(hours_line(&out), Some("Hours: 3, actual 3.5"), "{out}");
    r.commit("finish");
    let (code, out) = check(&r, "main");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_plan_that_ends_with_a_task_for_finish_can_finish_and_pass_check() {
    let r = Fx {
        delta: &[CHANGE_CLOSE, ADD_FLUSH],
        ..Fx::default()
    }
    .start();
    let order = format!(
        "{}\n{}",
        task("t1", true, "Hours: 2."),
        "- [ ] t2: Run `spec-dna finish dose` and commit the result. Files: none. Verify: `spec-dna check`. Hours: 0.5."
    );
    r.write(PLAN, &plan_with_tasks(&order, ""));
    r.write("tests/test_valves.py", COVERS_CLOSE_FLUSH);
    r.commit("work");
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.dir.join(FOLDER).exists());
    assert_eq!(hours_line(&out), Some("Hours: 2.5"), "{out}");
    r.commit("finish");
    let (code, out) = check(&r, "main");
    assert_eq!(code, 0, "{out}");
}

// The actual hours (0.7.1, B7)

#[test]
fn finish_adds_the_actual_hours_and_prints_them_after_the_estimate() {
    let order = format!(
        "{}\n{}",
        task("t1", true, "Hours: 1-2. Actual: 3."),
        task("t2", true, "Hours: 1. Actual: 0.5.")
    );
    let r = with_tasks(&order);
    let (code, out, err) = fin(&r, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(hours_line(&out), Some("Hours: 2-3, actual 3.5"), "{out}");
    assert!(!out.contains("note:"), "{out}");
    let (code, json, err) = fin(&r, &["--dry-run", "--json"]);
    assert_eq!(code, 0, "{json}{err}");
    assert!(json.contains("\"actual\":3.5"), "{json}");
    assert!(json.contains("\"hours\":{\"low\":2,\"high\":3}"), "{json}");
    // The label order is free, and the first `Actual:` counts.
    let order = task("t1", true, "Actual: 2. Hours: 1. Actual: 9.");
    let (_, out, _) = fin(&with_tasks(&order), &["--dry-run"]);
    assert_eq!(hours_line(&out), Some("Hours: 1, actual 2"), "{out}");
}

#[test]
fn an_actual_total_with_no_estimate_has_no_estimate_part() {
    let order = format!(
        "{}\n{}",
        task("t1", true, "Actual: 1.5."),
        task("t2", true, "Actual: 2.")
    );
    let (code, out, err) = fin(&with_tasks(&order), &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(hours_line(&out), Some("Hours: actual 3.5"), "{out}");
    let (_, json, _) = fin(&with_tasks(&order), &["--dry-run", "--json"]);
    assert!(
        json.contains("\"hours\":null") && json.contains("\"actual\":3.5"),
        "{json}"
    );
}

#[test]
fn a_plan_with_no_actual_hours_prints_the_line_as_before() {
    let order = format!(
        "{}\n{}",
        task("t1", true, "Hours: 1."),
        task("t2", true, "Hours: 2.")
    );
    let r = with_tasks(&order);
    let (_, out, _) = fin(&r, &["--dry-run"]);
    assert_eq!(hours_line(&out), Some("Hours: 3"), "{out}");
    assert!(!out.contains("actual"), "{out}");
    assert!(!out.contains("note:"), "{out}");
    let (_, json, _) = fin(&r, &["--dry-run", "--json"]);
    assert!(json.contains("\"actual\":null"), "{json}");
}

#[test]
fn the_actual_hours_of_a_skipped_task_count_and_a_duplicate_does_not() {
    let order = format!(
        "{}\n{}\n{}",
        task("t1", true, "Hours: 1. Actual: 1."),
        task("t2", true, "Hours: 4. Actual: 0.5. Skip: the store has it."),
        task("t1", true, "Hours: 9. Actual: 9.")
    );
    let (code, out, err) = fin(&with_tasks(&order), &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    // The estimate leaves out the skipped task, and the actual total keeps it.
    assert_eq!(hours_line(&out), Some("Hours: 1, actual 1.5"), "{out}");
}

#[test]
fn an_invalid_actual_value_gives_a_note_and_no_actual_total() {
    for value in ["soon", "1-2", "3-3", "12345", "1.234"] {
        let order = format!(
            "{}\n{}",
            task("t1", true, "Hours: 1. Actual: 2."),
            task("t2", true, &format!("Hours: 1. Actual: {value}."))
        );
        let r = with_tasks(&order);
        let (code, out, err) = fin(&r, &["--dry-run"]);
        assert_eq!(code, 0, "{value}: {out}{err}");
        assert!(
            out.contains(&format!(
                "note: the task `t2` has the value \"{value}\" in `Actual:`, which is no number of hours; no actual total\n"
            )),
            "{value}: {out}"
        );
        assert_eq!(hours_line(&out), Some("Hours: 2"), "{value}: {out}");
        let (_, json, _) = fin(&r, &["--dry-run", "--json"]);
        assert!(json.contains("\"actual\":null"), "{value}: {json}");
    }
}

#[test]
fn a_done_task_with_no_actual_gives_a_note_and_the_total_of_the_others() {
    let order = format!(
        "{}\n{}\n{}\n{}",
        task("t1", true, "Hours: 1. Actual: 2."),
        task("t2", true, "Hours: 1."),
        task("t3", true, "Hours: 1. Actual: 1."),
        task("t4", true, "Hours: 1. Skip: no.")
    );
    let (code, out, err) = fin(&with_tasks(&order), &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    // A skipped task needs no `Actual:`, and the note names a done task only.
    assert!(
        out.contains("note: the done task `t2` has no `Actual:`; the actual total leaves it out\n"),
        "{out}"
    );
    assert_eq!(hours_line(&out), Some("Hours: 3, actual 3"), "{out}");
    let order = format!(
        "{}\n{}\n{}",
        task("t1", true, "Actual: 1."),
        task("t2", true, ""),
        task("t3", true, "")
    );
    let (_, out, _) = fin(&with_tasks(&order), &["--dry-run"]);
    assert!(
        out.contains(
            "note: the done tasks `t2`, `t3` have no `Actual:`; the actual total leaves them out\n"
        ),
        "{out}"
    );
}

// The invariants and the deviations in the record (design 0.8.0, D6 and D7,
// slice S5). The fixture is a pull request: the base commit is on `main`, and
// the branch `feature` adds the folder of the change, with a plan that is new,
// so the change needs a record. The names are example names.

const S5_SLUG: &str = "valve-timeout";
const S5_FOLDER: &str = "docs/intent/valve-timeout";
const S5_SPEC: &str = "docs/intent/valve-timeout/spec.md";
const S5_PLAN: &str = "docs/intent/valve-timeout/plan.md";
const S5_RECORD: &str = "docs/intent/valve-timeout/acceptance.toml";
const S5_ARCHIVE: &str = ".spec-dna/accepted/valve-timeout.toml";
const S5_RULE: &str = "---\nstatus: active\napplies-to: [\"**\"]\n---\n# Safety\n\n## Rules\n\n- **valve-closed**: The controller SHALL close each valve on a fault.\n- **alarm-sounds**: The controller SHALL sound the alarm on a fault.\n";
const S5_VALVES: &str = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **overflow**: When a bed overflows, the controller SHALL close the valve of that bed.\n";
const S5_CODE_OLD: &str = "def close_valve(bed):\n    bed.valve_open = False\n";
const S5_CODE_NEW: &str = "def close_valve(bed):\n    bed.valve_open = False\n\n\ndef on_sensor_timeout(bed):\n    close_valve(bed)\n";
const S5_TEST_OLD: &str = "# covers: requirements/valves#overflow\ndef test_overflow():\n    ...\n";
const S5_TEST_NEW: &str = "# covers: requirements/valves#overflow\ndef test_overflow():\n    ...\n\n\n# covers: requirements/valves#sensor-timeout\ndef test_sensor_timeout():\n    ...\n";
const S5_DELTA: &str = "- ADD requirements/valves#sensor-timeout: When a sensor gives no value for 60 seconds, the controller SHALL close the valve of its bed.";
const S5_ATTEMPT: &str = "\n[[attempt]]\nacceptor = 1\ntext = \"Sent a NaN value after the timeout.\"\noutcome = \"held\"\n";

fn s5_config(profile: &str, keys: &str, rules: &str) -> String {
    format!(
        "profile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n{keys}\n[rules]\nSD608 = \"off\"\n{rules}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.rules]\ntypes = [\"rule\"]\nmoves-to = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\", \"rules\"]\nkeep = \"until-finished\"\n"
    )
}

/// The text of the pin of a rule of the repository.
fn s5_pin(r: &Repo, key: &str) -> String {
    format!(
        "rules/safety#{key}@{}",
        hash(r, &format!("rules/safety#{key}"))
    )
}

/// The base on `main`, then the branch with the plan in one commit and the code
/// in the next. `invariants` are the items of the section `Invariants` (a
/// function of the repository, for the hash of a rule), and `deviations` the
/// items of the section `Deviations`. An empty text writes no section.
fn s5_built(
    profile: &str,
    keys: &str,
    rules: &str,
    invariants: impl Fn(&Repo) -> String,
    deviations: &str,
) -> Repo {
    let r = Repo::with_config(&s5_config(profile, keys, rules));
    r.write("docs/requirements/valves.md", S5_VALVES);
    r.write("docs/rules/safety.md", S5_RULE);
    r.write("src/valves.py", S5_CODE_OLD);
    r.write("tests/test_valves.py", S5_TEST_OLD);
    r.write("README.md", "# Project\n");
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    let inv = invariants(&r);
    r.write(
        "docs/intent/valve-timeout/intent.md",
        "---\nstatus: accepted\nclass: routine\n---\n# Valve timeout\n\n## Intent\n\nClose the valve when a sensor times out.\n",
    );
    let inv_section = if inv.is_empty() {
        String::new()
    } else {
        format!("## Invariants\n\n{inv}\n\n")
    };
    r.write(
        S5_SPEC,
        &format!(
            "---\nstatus: approved\n---\n# Valve timeout\n\n## Delta\n\n{S5_DELTA}\n\n## Interfaces\n\nNone.\n\n## Decisions\n\nNone.\n\n{inv_section}## Concerns\n\n- [x] A slow sensor. Resolution: 60 seconds.\n\n## Open questions\n\n- [x] Does the valve reopen? Answer: no.\n"
        ),
    );
    let dev_section = if deviations.is_empty() {
        String::new()
    } else {
        format!("\n## Deviations\n\n{deviations}\n")
    };
    r.write(
        S5_PLAN,
        &format!(
            "---\nstatus: approved\n---\n# Valve timeout\n\n## Files\n\n- `src/valves.py`\n- `tests/test_valves.py`\n\n## Order\n\n- [x] t1: Close the valve. Files: `src/valves.py`. Verify: the test passes.\n\n## Risks\n\n- A slow sensor.\n\n## Proof\n\nThe test covers `requirements/valves#sensor-timeout`.\n{dev_section}"
        ),
    );
    r.commit("plan");
    r.write("src/valves.py", S5_CODE_NEW);
    r.write("tests/test_valves.py", S5_TEST_NEW);
    r.commit("code");
    r
}

fn s5_one_pin(r: &Repo) -> String {
    format!("- {}", s5_pin(r, "valve-closed"))
}

const S5_DEV_PLAN: &str = "- d1 (plan): The timeout check moved to `src/sensors.rs`.";

/// The record of the change, filled as an acceptor fills it. `edit` changes the
/// text before the commit. The record is committed alone.
fn s5_record(r: &Repo, edit: impl Fn(String) -> String) -> String {
    let (code, out, err) = r.run(&["accept", S5_SLUG, "--session", "s1"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = r.run(&["accept", S5_SLUG, "--brief"]);
    assert_eq!(code, 0, "{out}{err}");
    let brief = out
        .lines()
        .last()
        .unwrap()
        .strip_prefix("brief ")
        .unwrap()
        .to_string();
    let mut t = r.read(S5_RECORD);
    t = t.replacen("brief = \"pending\"", &format!("brief = \"{brief}\""), 1);
    t = t.replace("result = \"pending\"", "result = \"accepted\"");
    t = t.replace("answer = \"pending\"", "answer = \"held\"");
    t = t.replace("verdict = \"pending\"", "verdict = \"accepted\"");
    t.push_str(S5_ATTEMPT);
    let t = edit(t);
    r.write(S5_RECORD, &t);
    r.commit("record");
    t
}

fn s5_finish(r: &Repo, args: &[&str]) -> (i32, String, String) {
    let mut v = vec!["finish", S5_SLUG];
    v.extend(args);
    r.run(&v)
}

/// The text of the record without the block of an entry of a table. The block
/// ends at the next table header or at the end of the text.
fn s5_without(text: &str, table: &str) -> String {
    let head = format!("[[{table}]]");
    let mut out = String::new();
    let mut skip = false;
    for l in text.split_inclusive('\n') {
        if l.trim() == head {
            skip = true;
            continue;
        }
        if skip && l.trim_start().starts_with("[[") {
            skip = false;
        }
        if !skip {
            out.push_str(l);
        }
    }
    out
}

#[test]
fn a_declared_invariant_with_no_answer_stops_finish_with_sd938_and_the_named_fix_works() {
    let r = s5_built("standard", "invariants = true\n", "", s5_one_pin, "");
    let pin = s5_pin(&r, "valve-closed");
    s5_record(&r, |t| s5_without(&t, "invariant"));
    let (code, out, err) = s5_finish(&r, &["--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.contains("\"step\":5") && out.contains("\"code\":\"SD938\""),
        "{out}"
    );
    assert!(r.dir.join(S5_FOLDER).is_dir(), "nothing changed");
    // The message names the entry to add: the pin, and the way to answer.
    assert!(out.contains(&pin) && out.contains("[[invariant]]"), "{out}");
    // The fix of the message: the acceptor adds the entry and commits it.
    let t = r.read(S5_RECORD);
    r.write(
        S5_RECORD,
        &format!(
            "{t}\n[[invariant]]\nid = \"{pin}\"\nanswer = \"not-affected\"\nreason = \"The timeout path does not touch the fault path.\"\n"
        ),
    );
    r.commit("the answer");
    let (code, out, err) = s5_finish(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        r.read(S5_ARCHIVE).contains(&format!("id = \"{pin}\"")),
        "the archive keeps the answer"
    );
}

#[test]
fn an_answer_with_another_pin_is_no_answer_and_the_same_pin_with_ticks_or_crlf_is_one() {
    let r = s5_built("standard", "invariants = true\n", "", s5_one_pin, "");
    let pin = s5_pin(&r, "valve-closed");
    // The answer names the rule with another hash: it answers another text.
    s5_record(&r, |t| {
        t.replace(&pin, "rules/safety#valve-closed@0123456789abcdef")
    });
    let (code, out, err) = s5_finish(&r, &[]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("SD938"), "{out}");
    // CRLF line ends in the record are the same record.
    let t = r
        .read(S5_RECORD)
        .replace("rules/safety#valve-closed@0123456789abcdef", &pin);
    r.write(S5_RECORD, &t.replace('\n', "\r\n"));
    r.commit("the right pin, with CRLF");
    let (code, out, err) = s5_finish(&r, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn an_honest_record_with_an_answer_and_a_later_tick_lets_finish_pass() {
    let r = s5_built(
        "standard",
        "invariants = true\n",
        "",
        s5_one_pin,
        S5_DEV_PLAN,
    );
    s5_record(&r, |t| t);
    // A later tick of a box in the plan, and a base commit to another path.
    let plan = r
        .read(S5_PLAN)
        .replace("- A slow sensor.", "- A slow sensor, checked.");
    r.write(S5_PLAN, &plan);
    r.commit("a later edit of the plan");
    let (code, out, err) = s5_finish(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_violated_answer_with_the_result_accepted_gives_sd928() {
    let r = s5_built("standard", "invariants = true\n", "", s5_one_pin, "");
    s5_record(&r, |t| {
        t.replace(
            "answer = \"held\"",
            "answer = \"violated\"\nreason = \"The fault path closes late.\"",
        )
    });
    let (code, out, err) = s5_finish(&r, &["--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("\"code\":\"SD928\""), "{out}");
    assert!(out.contains("violated"), "{out}");
    // With the result `rejected`, the stop is SD930.
    let t = r
        .read(S5_RECORD)
        .replace("result = \"accepted\"", "result = \"rejected\"");
    r.write(S5_RECORD, &t);
    r.commit("rejected");
    let (code, out, err) = s5_finish(&r, &["--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("\"code\":\"SD930\""), "{out}");
}

#[test]
fn a_deviation_that_the_record_lacks_or_copies_with_another_text_stops_with_sd938() {
    let r = s5_built("standard", "", "", |_| String::new(), S5_DEV_PLAN);
    s5_record(&r, |t| s5_without(&t, "deviation"));
    let (code, out, err) = s5_finish(&r, &["--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("\"code\":\"SD938\""), "{out}");
    assert!(
        out.contains("[[deviation]]") && out.contains("`d1`") && out.contains("src/sensors.rs"),
        "{out}"
    );
    // An entry with the same id and another text: SD938 too.
    let t = r.read(S5_RECORD);
    r.write(
        S5_RECORD,
        &format!(
            "{t}\n[[deviation]]\nid = \"d1\"\nkind = \"plan\"\ntext = \"Another text.\"\nverdict = \"accepted\"\n"
        ),
    );
    r.commit("an entry with another text");
    let (code, out, err) = s5_finish(&r, &["--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.contains("\"code\":\"SD938\"") && out.contains("not the text of the plan"),
        "{out}"
    );
    // The same text, with the white space of the plan: it passes.
    let t = r.read(S5_RECORD).replace(
        "text = \"Another text.\"",
        "text = \"The  timeout check moved to `src/sensors.rs`.\"",
    );
    r.write(S5_RECORD, &t);
    r.commit("the text again");
    let (code, out, err) = s5_finish(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_deviation_that_the_builder_adds_after_the_record_gives_sd938() {
    let r = s5_built("standard", "", "", |_| String::new(), S5_DEV_PLAN);
    s5_record(&r, |t| t);
    let plan = r.read(S5_PLAN).replace(
        S5_DEV_PLAN,
        &format!("{S5_DEV_PLAN}\n- d2 (plan): The order of the tasks changed."),
    );
    r.write(S5_PLAN, &plan);
    r.commit("a new deviation");
    let (code, out, err) = s5_finish(&r, &["--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.contains("\"code\":\"SD938\"") && out.contains("`d2`"),
        "{out}"
    );
}

#[test]
fn finish_prints_the_deviations_after_hours_with_the_verdict_also_with_dry_run_and_in_json() {
    let r = s5_built("standard", "", "", |_| String::new(), S5_DEV_PLAN);
    let plan = r.read(S5_PLAN).replace(
        "Verify: the test passes.",
        "Verify: the test passes. Hours: 1. Actual: 1.",
    );
    r.write(S5_PLAN, &plan);
    r.commit("hours");
    s5_record(&r, |t| t);
    for args in [vec!["--dry-run"], vec![]] {
        let (code, out, err) = s5_finish(&r, &args);
        assert_eq!(code, 0, "{out}{err}");
        let lines: Vec<&str> = out.lines().collect();
        let hours = lines.iter().position(|l| l.starts_with("Hours: ")).unwrap();
        let dev = lines
            .iter()
            .position(|l| l.starts_with("Deviations: 1"))
            .unwrap();
        assert!(dev > hours, "{out}");
        assert_eq!(
            lines[dev + 1],
            "- d1 (plan), accepted: The timeout check moved to `src/sensors.rs`.",
            "{out}"
        );
        // The dry run writes nothing, so the second run finds the same state.
        if args.is_empty() {
            break;
        }
    }
}

#[test]
fn finish_json_has_the_list_deviations_with_id_kind_text_and_verdict() {
    let r = s5_built("standard", "", "", |_| String::new(), S5_DEV_PLAN);
    s5_record(&r, |t| t);
    let (code, out, err) = s5_finish(&r, &["--dry-run", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("\"deviations\":[{\"id\":\"d1\",\"kind\":\"plan\",\"text\":\"The timeout check moved to `src/sensors.rs`.\",\"verdict\":\"accepted\"}]"),
        "{out}"
    );
    // No deviation: an empty list, and no line `Deviations:`.
    let r = s5_built("standard", "", "", |_| String::new(), "");
    s5_record(&r, |t| t);
    let (code, out, err) = s5_finish(&r, &["--dry-run", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"deviations\":[]"), "{out}");
    let (code, out, err) = s5_finish(&r, &["--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("Deviations:"), "{out}");
}

#[test]
fn in_lite_with_no_record_finish_prints_no_deviations_and_exits_0() {
    let r = s5_built("lite", "", "", |_| String::new(), S5_DEV_PLAN);
    let (code, out, err) = s5_finish(&r, &["--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"deviations\":[]"), "{out}");
}

#[test]
fn a_bad_item_in_the_plan_or_the_spec_stops_finish_before_the_answers() {
    // SD939: a deviation with no kind.
    let r = s5_built("standard", "", "", |_| String::new(), "- d1: no kind.");
    s5_record(&r, |t| t);
    let (code, out, err) = s5_finish(&r, &["--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("\"code\":\"SD939\""), "{out}");
    // SD940: a `spec` deviation with no fix.
    let r = s5_built(
        "standard",
        "",
        "",
        |_| String::new(),
        "- d1 (spec): The valve closes after 2 reads.",
    );
    s5_record(&r, |t| t);
    let (code, out, err) = s5_finish(&r, &["--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("\"code\":\"SD940\""), "{out}");
    // SD936: an item of `Invariants` that names a requirement.
    let r = s5_built(
        "standard",
        "invariants = true\n",
        "",
        |_| "- requirements/valves#overflow@0123456789abcdef".to_string(),
        "",
    );
    s5_record(&r, |t| t);
    let (code, out, err) = s5_finish(&r, &["--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("\"code\":\"SD936\""), "{out}");
    // SD937: an old hash.
    let r = s5_built(
        "standard",
        "invariants = true\n",
        "",
        |_| "- rules/safety#valve-closed@0123456789abcdef".to_string(),
        "",
    );
    s5_record(&r, |t| t);
    let (code, out, err) = s5_finish(&r, &["--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("\"code\":\"SD937\""), "{out}");
}

#[test]
fn a_spec_with_a_change_line_for_the_rule_and_the_same_hash_in_the_item_finishes() {
    // The item and the line pin the old text. After step 4 the rule has the new
    // text, and `check` of the pull request gives no SD937 (the spec is gone).
    let r = s5_built(
        "standard",
        "invariants = true\n",
        "",
        |r| {
            let h = hash(r, "rules/safety#valve-closed");
            format!("- rules/safety#valve-closed@{h}")
        },
        "",
    );
    let h = hash(&r, "rules/safety#valve-closed");
    let delta = format!(
        "{S5_DELTA}\n- CHANGE rules/safety#valve-closed@{h}: The controller SHALL close each valve within 2 seconds of a fault."
    );
    let spec = r.read(S5_SPEC).replace(S5_DELTA, &delta);
    r.write(S5_SPEC, &spec);
    r.write(
        "tests/test_valves.py",
        &format!(
            "{S5_TEST_NEW}\n\n# covers: rules/safety#valve-closed\ndef test_fault():\n    ...\n"
        ),
    );
    r.commit("a change line for the rule");
    s5_record(&r, |t| t);
    let (code, out, err) = s5_finish(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("finish");
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert!(!out.contains("SD937") && !out.contains("SD936"), "{out}");
    // The same item with another hash than the line: SD937 at step 5.
    let r = s5_built(
        "standard",
        "invariants = true\n",
        "",
        |r| {
            let h = hash(r, "rules/safety#alarm-sounds");
            format!("- rules/safety#valve-closed@{h}")
        },
        "",
    );
    let h = hash(&r, "rules/safety#valve-closed");
    let delta = format!(
        "{S5_DELTA}\n- CHANGE rules/safety#valve-closed@{h}: The controller SHALL close each valve within 2 seconds of a fault."
    );
    let spec = r.read(S5_SPEC).replace(S5_DELTA, &delta);
    r.write(S5_SPEC, &spec);
    r.commit("a change line for the rule");
    s5_record(&r, |t| t);
    let (code, out, err) = s5_finish(&r, &["--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("\"code\":\"SD937\""), "{out}");
}

/// The folder is deleted and the archive file holds `record`, as `finish` writes
/// them, but with the text that the test gives.
fn s5_finished_by_hand(r: &Repo, record: &str) {
    r.git(&["rm", "-q", "-r", S5_FOLDER]);
    let valves = format!(
        "{S5_VALVES}- **sensor-timeout**: When a sensor gives no value for 60 seconds, the controller SHALL close the valve of its bed.\n"
    );
    r.write("docs/requirements/valves.md", &valves);
    r.write(S5_ARCHIVE, record);
    r.commit("finish");
}

fn s5_in_test_merge(r: &Repo) {
    r.git(&["checkout", "-q", "main"]);
    r.write("README.md", "# Project, main moves on\n");
    r.commit("main moves on");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "--detach", "main"]);
    r.git(&["merge", "-q", "--no-ff", "-m", "test merge", "feature"]);
}

#[test]
fn check_gives_sd938_for_an_archive_without_the_answers_on_the_branch_and_the_test_merge() {
    let r = s5_built(
        "standard",
        "invariants = true\n",
        "",
        s5_one_pin,
        S5_DEV_PLAN,
    );
    // The record has no answer for the invariant and no entry for the deviation.
    let t = s5_record(&r, |t| {
        s5_without(&s5_without(&t, "invariant"), "deviation")
    });
    s5_finished_by_hand(&r, &t);
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    let found: Vec<&str> = out.lines().filter(|l| l.contains(" SD938 ")).collect();
    assert_eq!(found.len(), 2, "{out}");
    assert!(found.iter().all(|l| l.starts_with(S5_ARCHIVE)), "{out}");
    s5_in_test_merge(&r);
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(
        out.lines().filter(|l| l.contains(" SD938 ")).count(),
        2,
        "{out}"
    );
    // `main` merged into the branch, with a commit to another path.
    r.git(&["checkout", "-q", "feature"]);
    r.git(&["merge", "-q", "-m", "merge main", "main"]);
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(
        out.lines().filter(|l| l.contains(" SD938 ")).count(),
        2,
        "{out}"
    );
}

#[test]
fn check_gives_no_sd938_for_an_archive_with_the_answers_on_the_branch_and_the_test_merge() {
    let r = s5_built(
        "standard",
        "invariants = true\n",
        "",
        s5_one_pin,
        S5_DEV_PLAN,
    );
    let t = s5_record(&r, |t| t);
    let (code, out, err) = s5_finish(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("finish");
    assert_eq!(r.read(S5_ARCHIVE), t);
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert!(!out.contains("SD938"), "{out}");
    s5_in_test_merge(&r);
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert!(!out.contains("SD938"), "{out}");
}

#[test]
fn sd938_is_off_in_lite_and_the_invariants_need_the_key() {
    // `lite` with a record rule: the codes of the group are on, and SD938 reads
    // the deviations. Without the key it reads no invariant.
    let r = s5_built(
        "standard",
        "",
        "",
        |r| format!("- {}", s5_pin(r, "valve-closed")),
        "",
    );
    let t = s5_record(&r, |t| t);
    assert!(!t.contains("[[invariant]]"), "{t}");
    let (code, out, err) = s5_finish(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_key_that_a_later_commit_sets_does_not_change_the_brief_of_an_older_entry() {
    // The record is written with the key off. A later commit turns the key on.
    // The brief of the entry is the brief of its adding commit, which read the
    // configuration of that commit, so SD928 stays silent. SD938 then asks for
    // the answer of the invariant that the key brings in.
    let r = s5_built("standard", "", "", s5_one_pin, "");
    s5_record(&r, |t| t);
    let cfg = r.read("spec-dna.toml").replace(
        "gate = \"block\"\n",
        "gate = \"block\"\ninvariants = true\n",
    );
    r.write("spec-dna.toml", &cfg);
    r.commit("turn the invariants on");
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert!(!out.contains("SD928"), "{out}");
    let (code, out, err) = s5_finish(&r, &["--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("\"code\":\"SD938\""), "{out}");
    assert!(!out.contains("SD928"), "{out}");
}

#[test]
fn a_reordered_list_of_invariants_after_the_record_keeps_the_record_valid() {
    let r = s5_built(
        "standard",
        "invariants = true\n",
        "",
        |r| {
            format!(
                "- {}\n- {}",
                s5_pin(r, "valve-closed"),
                s5_pin(r, "alarm-sounds")
            )
        },
        "",
    );
    let t = s5_record(&r, |t| t);
    assert_eq!(t.matches("[[invariant]]").count(), 2, "{t}");
    // The builder reorders the 2 items after the record.
    let spec = r.read(S5_SPEC);
    let (a, b) = (s5_pin(&r, "valve-closed"), s5_pin(&r, "alarm-sounds"));
    let swapped = spec
        .replace(&format!("- {a}\n- {b}"), "- @@\n- &&")
        .replace("- @@", &format!("- {b}"))
        .replace("- &&", &format!("- {a}"));
    assert_ne!(spec, swapped);
    r.write(S5_SPEC, &swapped);
    r.commit("reorder");
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert!(!out.contains("SD928") && !out.contains("SD938"), "{out}");
    let (code, out, err) = s5_finish(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn finish_of_a_change_that_edits_a_rule_does_not_stop_on_the_stale_pin_of_another_open_spec() {
    // Change A has a `CHANGE` line for the rule. Change B is open and pins the
    // old text of the rule. After step 4 the rule has the new text, so B gets
    // SD937. The finish diff does not touch B, so the code is no error of the
    // result of `finish` of A, and it never stops it. The author of B pins the
    // rule again.
    let r = s5_built(
        "standard",
        "invariants = true\n",
        "",
        |r| {
            let h = hash(r, "rules/safety#valve-closed");
            format!("- rules/safety#valve-closed@{h}")
        },
        "",
    );
    let h = hash(&r, "rules/safety#valve-closed");
    let delta = format!(
        "{S5_DELTA}\n- CHANGE rules/safety#valve-closed@{h}: The controller SHALL close each valve within 2 seconds of a fault."
    );
    let spec = r.read(S5_SPEC).replace(S5_DELTA, &delta);
    r.write(S5_SPEC, &spec);
    r.write(
        "tests/test_valves.py",
        &format!(
            "{S5_TEST_NEW}\n\n# covers: rules/safety#valve-closed\ndef test_fault():\n    ...\n"
        ),
    );
    r.write(
        "docs/intent/other/intent.md",
        "---\nstatus: accepted\nclass: higher-risk\n---\n# Other\n\n## Intent\n\nAnother change.\n",
    );
    r.write(
        "docs/intent/other/spec.md",
        &format!(
            "---\nstatus: draft\n---\n# Other\n\n## Delta\n\n- ADD requirements/valves#other-timeout: The controller SHALL log each timeout.\n\n## Invariants\n\n- rules/safety#valve-closed@{h}\n"
        ),
    );
    r.commit("a change line for the rule, and another open change");
    s5_record(&r, |t| t);
    let (code, out, err) = s5_finish(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("finish");
    let (_, out, _) = r.run(&["check", "--all", "--base", "main"]);
    let line = out
        .lines()
        .find(|l| l.contains(" SD937 "))
        .unwrap_or_else(|| panic!("{out}"));
    assert!(line.starts_with("docs/intent/other/spec.md"), "{line}");
}

#[test]
fn a_rejected_deviation_with_the_result_accepted_gives_sd928() {
    let r = s5_built("standard", "", "", |_| String::new(), S5_DEV_PLAN);
    s5_record(&r, |t| {
        t.replace("verdict = \"accepted\"", "verdict = \"rejected\"")
    });
    let (code, out, err) = s5_finish(&r, &["--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.contains("\"code\":\"SD928\"") && out.contains("a deviation is `rejected`"),
        "{out}"
    );
    // With the result `rejected`, the stop is SD930.
    let t = r
        .read(S5_RECORD)
        .replace("result = \"accepted\"", "result = \"rejected\"");
    r.write(S5_RECORD, &t);
    r.commit("rejected");
    let (code, out, err) = s5_finish(&r, &["--json"]);
    assert_eq!(code, 1, "{out}{err}");
    assert!(out.contains("\"code\":\"SD930\""), "{out}");
}
