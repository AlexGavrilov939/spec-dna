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
    // is no approved delta to compare.
    let r = Fx::default().start();
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
            .ends_with("\"hours\":{\"low\":1.25,\"high\":2.25}}"),
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
