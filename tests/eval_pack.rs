//! Tests of `spec-dna eval pack` (ADR 0016, 0.7.0 slice 5). The command
//! writes the shipped evals and their small fixtures to `<[evals] dir>/pack/`.
//! A test checks that the pack holds the 62 evals of the 11 skills, of
//! `agents-line` and of the target `accept`. The other
//! tests read the
//! files that the binary writes, whatever their number, and check the rules
//! that hold for each of them. No test starts an agent.

mod common;
#[path = "eval_pack_pilot/mod.rs"]
mod pilot;

use common::{Repo, VERSION};
use std::fs;
use std::path::Path;

const SKILLS: [&str; 11] = [
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
];

fn project(evals: &str) -> Repo {
    let r = Repo::with_config(&format!("\n{evals}\n[[roots]]\npath = \"docs\"\n"));
    r.write("docs/a.md", "# A\n\nText.\n");
    r.write(
        "evals/check.sh",
        "file_has() {\n    grep -q -F -- \"$2\" \"$1\"\n}\n",
    );
    r.commit("start");
    r
}

fn pack(r: &Repo) -> (i32, String, String) {
    r.run(&["eval", "pack"])
}

/// The names of the files `<dir>/pack/*.json`, sorted.
fn packed(r: &Repo) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(r.dir.join("evals/pack"))
        .map(|d| {
            d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(".json"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// The first file under a folder, in the order of the names.
fn first_file(dir: &Path) -> std::path::PathBuf {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    let first = entries.remove(0);
    if first.is_dir() {
        first_file(&first)
    } else {
        first
    }
}

fn files_under(dir: &Path) -> usize {
    let mut n = 0;
    if let Ok(read) = fs::read_dir(dir) {
        for e in read {
            let p = e.unwrap().path();
            n += if p.is_dir() { files_under(&p) } else { 1 };
        }
    }
    n
}

#[test]
fn eval_pack_writes_the_evals_and_their_fixtures_below_the_pack_folder() {
    let r = project("[evals]\n");
    let (code, out, err) = pack(&r);
    assert_eq!(code, 0, "{out}{err}");
    let names = packed(&r);
    assert!(!names.is_empty(), "the binary holds no eval: {out}");
    for name in &names {
        let id = name.strip_suffix(".json").unwrap();
        let text = r.read(&format!("evals/pack/{name}"));
        assert!(text.contains(&format!("\"id\": \"{id}\"")), "{name}");
        // Each file has the key `skill`, and the id is `<group>-<n>`. The group
        // `accept` has the skill `review` (design 0.8.0, D62).
        let group = id.rsplit_once('-').unwrap().0;
        assert!(
            SKILLS.contains(&group) || ["agents-line", "accept"].contains(&group),
            "{id}"
        );
        let skill = if group == "accept" { "review" } else { group };
        assert!(text.contains(&format!("\"skill\": \"{skill}\"")), "{name}");
        assert!(
            text.contains("\"runs\": 4"),
            "{name}: the pack runs 4 times"
        );
        assert!(
            text.contains(&format!("\"fixture\": \"evals/pack/fixtures/{id}\"")),
            "{name}"
        );
        assert!(
            files_under(&r.dir.join(format!("evals/pack/fixtures/{id}"))) > 0,
            "{id}: no fixture file"
        );
    }
    assert!(
        out.contains(&format!("wrote evals/pack/{}", names[0])),
        "{out}"
    );
    assert!(out.contains("eval pack: "), "{out}");
}

#[test]
fn each_file_of_the_pack_is_valid_for_the_runner() {
    let r = project("[evals]\n");
    let (code, out, err) = pack(&r);
    assert_eq!(code, 0, "{out}{err}");
    let mut skills: Vec<String> = packed(&r)
        .iter()
        .map(|n| n.rsplit_once('-').unwrap().0.to_string())
        .collect();
    skills.dedup();
    assert!(!skills.is_empty());
    assert!(skills.iter().any(|s| s == "accept"), "{skills:?}");
    for skill in skills {
        // A dry run reads each file with the rules of SD506.
        let (code, out, err) = r.run(&[
            "eval",
            "run",
            "--ab",
            &skill,
            "--dry-run",
            "--max-total-usd",
            "1000",
            // The target `accept` needs a harness arm.
            "--harness",
            "lean",
        ]);
        assert_eq!(code, 0, "{skill}: {out}{err}");
    }
}

#[test]
fn the_evals_of_the_pack_are_not_read_by_eval_run_and_sd506() {
    let r = project("[evals]\n");
    let (code, ..) = pack(&r);
    assert_eq!(code, 0);
    r.write(
        "evals/example.json",
        "{\"id\":\"example\",\"task\":\"x\",\"fixture\":\"evals/pack/fixtures/tdd-1\",\"max-budget-usd\":1,\"checks\":[{\"name\":\"a\",\"run\":\"true\"}]}\n",
    );
    r.commit("pack");
    let (_, out, err) = r.run(&["eval", "run", "--dry-run"]);
    assert!(
        out.contains("1 evals"),
        "only example.json is an eval: {out}{err}"
    );
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(!out.contains("SD506"), "{out}");
}

#[test]
fn eval_pack_keeps_a_file_that_exists_and_names_it() {
    let r = project("[evals]\n");
    pack(&r);
    let name = packed(&r)[0].clone();
    let path = format!("evals/pack/{name}");
    r.write(&path, "{\"my\":\"edit\"}\n");
    let (code, out, err) = pack(&r);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.read(&path), "{\"my\":\"edit\"}\n", "the file stays");
    assert!(out.contains(&format!("kept {path}")), "{out}");
    // A file that is absent is written again.
    fs::remove_file(r.dir.join(&path)).unwrap();
    let (code, out, _) = pack(&r);
    assert_eq!(code, 0);
    assert!(out.contains(&format!("wrote {path}")), "{out}");
    // A kept fixture file stays too.
    let id = name.strip_suffix(".json").unwrap();
    let fixture = first_file(&r.dir.join(format!("evals/pack/fixtures/{id}")));
    fs::write(&fixture, "mine\n").unwrap();
    let (code, ..) = pack(&r);
    assert_eq!(code, 0);
    assert_eq!(fs::read_to_string(&fixture).unwrap(), "mine\n");
}

#[test]
fn eval_pack_json_lists_the_written_and_the_kept_files() {
    let r = project("[evals]\n");
    let (code, out, err) = r.run(&["eval", "pack", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.starts_with("{\"written\":["), "{out}");
    assert!(out.contains("evals/pack/"), "{out}");
    assert!(out.contains("\"kept\":[]"), "{out}");
    let (code, out, _) = r.run(&["eval", "pack", "--json"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"written\":[]"), "{out}");
}

#[test]
fn eval_pack_needs_the_evals_table_and_takes_no_argument() {
    let r = project("");
    let (code, _, err) = pack(&r);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("[evals]"), "{err}");
    assert!(!r.dir.join("evals/pack").exists());
    let r = project("[evals]\n");
    let (code, _, err) = r.run(&["eval", "pack", "tdd"]);
    assert_eq!(code, 2, "{err}");
}

#[test]
fn eval_pack_uses_the_folder_of_the_table_and_the_fixture_path_follows_it() {
    let r = Repo::with_config("\n[evals]\ndir = \"qa/evals\"\n\n[[roots]]\npath = \"docs\"\n");
    r.write("docs/a.md", "# A\n\nText.\n");
    r.write("qa/evals/check.sh", "true() { :; }\n");
    r.commit("start");
    let (code, out, err) = pack(&r);
    assert_eq!(code, 0, "{out}{err}");
    let any = fs::read_dir(r.dir.join("qa/evals/pack"))
        .unwrap()
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().ends_with(".json"))
        .unwrap();
    let text = fs::read_to_string(any.path()).unwrap();
    assert!(
        text.contains("\"fixture\": \"qa/evals/pack/fixtures/"),
        "{text}"
    );
}

#[test]
fn a_check_that_runs_spec_dna_check_names_the_codes_that_it_reads() {
    // The arms differ in the generated files, so the exit code of `check`
    // alone would differ with the arm. A check reads the codes.
    let r = project("[evals]\n");
    pack(&r);
    assert!(!packed(&r).is_empty(), "the binary holds no eval");
    for name in packed(&r) {
        let text = r.read(&format!("evals/pack/{name}"));
        // A tool of `allowed-tools`, such as `Bash(spec-dna check:*)`, is no check.
        for line in text
            .lines()
            .filter(|l| l.contains("spec-dna check") && !l.contains("Bash("))
        {
            assert!(line.contains("SD"), "{name}: a check names no code: {line}");
        }
    }
}

#[test]
fn the_fixture_names_no_consumer_project_and_pins_the_version_of_the_binary() {
    let r = project("[evals]\n");
    pack(&r);
    assert!(!packed(&r).is_empty(), "the binary holds no eval");
    for name in packed(&r) {
        let id = name.strip_suffix(".json").unwrap();
        let toml = r
            .dir
            .join(format!("evals/pack/fixtures/{id}/spec-dna.toml"));
        let text = fs::read_to_string(toml).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert!(
            text.starts_with(&format!("spec-dna = \"{VERSION}\"")),
            "{id}: {text}"
        );
    }
}

// The tests that need the 62 evals of the pack.

/// The ids of each skill: `verify`, `brainstorm` and `execute` have the ids 6
/// to 10, the others 1 to 5.
fn ids_of(skill: &str) -> Vec<String> {
    let (from, to) = if ["verify", "brainstorm", "execute"].contains(&skill) {
        (6, 10)
    } else {
        (1, 5)
    };
    (from..=to).map(|n| format!("{skill}-{n}")).collect()
}

#[test]
fn the_pack_has_5_evals_for_each_skill_2_of_agents_line_and_5_of_accept() {
    let r = project("[evals]\n");
    let (code, out, err) = pack(&r);
    assert_eq!(code, 0, "{out}{err}");
    let names = packed(&r);
    assert_eq!(names.len(), 62, "{names:?}");
    for skill in SKILLS {
        for id in ids_of(skill) {
            assert!(names.contains(&format!("{id}.json")), "{id}");
        }
    }
    // The 5 evals of the acceptance record have the skill `review`, and
    // their own target (design 0.8.0, D62).
    for n in 1..=5 {
        let id = format!("accept-{n}");
        let text = r.read(&format!("evals/pack/{id}.json"));
        assert!(names.contains(&format!("{id}.json")), "{id}");
        assert!(text.contains("\"skill\": \"review\""), "{id}");
    }
    for id in ["agents-line-1", "agents-line-2"] {
        let text = r.read(&format!("evals/pack/{id}.json"));
        assert!(text.contains("\"skill\": \"agents-line\""), "{id}");
        assert!(names.contains(&format!("{id}.json")), "{id}");
    }
    // The ids of the pilot evals of `verify`, `brainstorm` and `execute` are
    // gone: a record names its eval, and a new eval has a new id.
    for skill in ["verify", "brainstorm", "execute"] {
        for n in 1..=5 {
            assert!(!names.contains(&format!("{skill}-{n}.json")), "{skill}-{n}");
        }
    }
    // Ids are unique and have the form `<group>-<n>` with n from 1.
    for name in &names {
        let (group, n) = name
            .strip_suffix(".json")
            .unwrap()
            .rsplit_once('-')
            .unwrap();
        assert!(n.parse::<u32>().is_ok_and(|n| n >= 1), "{name}");
        assert!(
            SKILLS.contains(&group) || ["agents-line", "accept"].contains(&group),
            "{name}"
        );
    }
    // Each eval is valid for SD506 and for the runner.
    for skill in SKILLS {
        let (code, out, err) = r.run(&[
            "eval",
            "run",
            "--ab",
            skill,
            "--dry-run",
            "--max-total-usd",
            "1000",
        ]);
        assert_eq!(code, 0, "{skill}: {out}{err}");
        assert!(out.contains("5 evals, 40 runs"), "{skill}: {out}");
    }
}

#[test]
fn the_agents_line_set_is_tdd_1_debug_1_execute_10_and_2_own_evals() {
    let r = project("[evals]\n");
    pack(&r);
    let (code, out, err) = r.run(&[
        "eval",
        "run",
        "--ab",
        "agents-line",
        "--dry-run",
        "--max-total-usd",
        "1000",
        "--json",
    ]);
    assert_eq!(code, 0, "{out}{err}");
    let ids: Vec<&str> = out
        .split("{\"id\":\"")
        .skip(1)
        .map(|p| p.split('"').next().unwrap())
        .collect();
    assert_eq!(
        ids,
        [
            "tdd-1",
            "debug-1",
            "execute-10",
            "agents-line-1",
            "agents-line-2"
        ],
        "{out}"
    );
    // The 2 evals that were at the ceiling in both arms left the set.
    assert!(
        !out.contains("verify-") && !out.contains("review-1"),
        "{out}"
    );
}

#[test]
fn a_dry_run_of_a_skill_and_of_the_agents_line_each_print_40_runs_and_an_estimate() {
    let r = project("[evals]\n");
    pack(&r);
    for target in ["execute", "agents-line"] {
        let (code, out, err) = r.run(&[
            "eval",
            "run",
            "--ab",
            target,
            "--dry-run",
            "--max-total-usd",
            "1000",
        ]);
        assert_eq!(code, 0, "{target}: {out}{err}");
        assert!(
            out.contains(&format!(
                "eval run --ab {target} (dry run): 5 evals, 40 runs, estimate "
            )),
            "{target}: {out}"
        );
    }
    // `all` runs the 11 skills only: 55 evals, and no eval of `agents-line`.
    let (code, out, _) = r.run(&[
        "eval",
        "run",
        "--ab",
        "all",
        "--dry-run",
        "--max-total-usd",
        "1000",
    ]);
    assert_eq!(code, 0);
    assert!(out.contains("55 evals, 440 runs"), "{out}");
}

#[test]
fn the_covers_tags_of_the_pack_fixtures_give_no_sd105() {
    // A fixture holds test files with `covers:` tags. They are no test of the
    // project, so `check` reads none of them.
    let r = project("[evals]\n");
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("init");
    let (code, out, err) = pack(&r);
    assert_eq!(code, 0, "{out}{err}");
    let tagged = std::process::Command::new("grep")
        .args(["-rl", "covers:", "evals/pack/fixtures"])
        .current_dir(&r.dir)
        .output()
        .unwrap();
    assert!(
        !tagged.stdout.is_empty(),
        "the test needs a fixture with a covers tag"
    );
    r.commit("pack");
    let (_, out, _) = r.run(&["check", "--base", "HEAD~1"]);
    assert!(!out.contains("SD105"), "{out}");
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(!out.contains("SD105"), "{out}");
}

// The turn cap of the evals that the proof of the old eval `execute-1` measured
// (session 26). In the pilot, 4 of 4 runs of it with the line of `AGENTS.md`
// stopped at the cap of 25 turns with correct work: the line costs 3 to 7 turns
// that only run `spec-dna`. So an eval of `execute`, and the evals `tdd-1` and
// `debug-1` of the pair `agents-line`, have a cap of at least 40 turns.

/// The number after `"<key>": ` in the text of an eval file.
fn number(text: &str, key: &str) -> f64 {
    let at = text
        .find(&format!("\"{key}\": "))
        .unwrap_or_else(|| panic!("no key {key}"));
    let rest = &text[at + key.len() + 4..];
    let end = rest.find([',', '\n']).unwrap();
    rest[..end].trim().parse().unwrap()
}

#[test]
fn no_eval_of_execute_has_a_turn_cap_under_40_and_the_evals_of_agents_line_keep_headroom() {
    let r = project("[evals]\n");
    let (code, out, err) = pack(&r);
    assert_eq!(code, 0, "{out}{err}");
    for id in [
        "execute-6",
        "execute-7",
        "execute-8",
        "execute-9",
        "execute-10",
        "tdd-1",
        "debug-1",
    ] {
        let text = r.read(&format!("evals/pack/{id}.json"));
        let turns = number(&text, "max-turns");
        assert!(turns >= 40.0, "{id}: {turns} turns");
        // The budget stays the guard of the cost.
        assert!(number(&text, "max-budget-usd") <= 2.0, "{id}");
    }
}

#[test]
fn the_evals_of_verify_keep_headroom_of_turns_for_the_arm_with_the_skill() {
    // In the rerun of 0.7.1, 9 of 9 runs with the skill stopped at 20 turns:
    // the arm with the skill reads the skills `tdd` and `execute` and makes
    // 1 or 2 commits, so it needs about 2 turns more than the other arm.
    let r = project("[evals]\n");
    let (code, out, err) = pack(&r);
    assert_eq!(code, 0, "{out}{err}");
    for id in ["verify-6", "verify-7", "verify-8", "verify-9", "verify-10"] {
        let text = r.read(&format!("evals/pack/{id}.json"));
        let turns = number(&text, "max-turns");
        assert!(turns >= 40.0, "{id}: {turns} turns");
        // The budget stays the guard of the cost.
        assert!(number(&text, "max-budget-usd") <= 2.5, "{id}");
    }
}

#[test]
fn the_fixture_of_execute_10_has_a_requirement_that_a_covers_tag_can_name() {
    // `execute-10` is the plain edit of the pair `agents-line`, and an agent
    // with the line can tag a test with `covers:`.
    let r = project("[evals]\n");
    pack(&r);
    let fixture = "evals/pack/fixtures/execute-10";
    let counts = r.read(&format!("{fixture}/docs/requirements/wrap.md"));
    assert!(counts.contains("- **wrap-lines**:"), "{counts}");
    // A project with this document and a test that names the requirement
    // gives no SD105.
    let p = Repo::with_config(
        "\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n",
    );
    p.write("docs/requirements/wrap.md", &counts);
    p.write(
        "tests/test_wrap.py",
        "# covers: requirements/wrap#wrap-lines\nimport unittest\n",
    );
    p.commit("start");
    let (_, out, err) = p.run(&["check", "--all"]);
    assert!(!out.contains("SD105"), "{out}{err}");
    // The old tag names a section, and it is no requirement.
    p.write(
        "tests/test_wrap.py",
        "# covers: requirements/wrap#wrap\nimport unittest\n",
    );
    let (_, out, _) = p.run(&["check", "--all"]);
    assert!(out.contains("SD105"), "{out}");
}

#[test]
fn verify_8_runs_on_a_branch_so_that_the_proof_check_base_main_has_a_base() {
    // The proof of `verify-8` is `spec-dna check --base main`. If the agent
    // committed on `main`, the base would move and the proof would report no
    // new error. So the eval runs on a branch, and `main` stays at the base.
    let r = project("[evals]\n");
    pack(&r);
    let text = r.read("evals/pack/verify-8.json");
    assert!(
        text.contains("\"branch\": \"feature/overdue-fee\""),
        "{text}"
    );
}

#[test]
fn finish_1_and_finish_3_have_the_check_hours_line_that_a_result_lists_by_name() {
    // The check reads `Hours: 4.5` in the commit message. Each side of a result
    // of `eval run --ab` lists the check names, so a run shows if it failed.
    let r = project("[evals]\n");
    pack(&r);
    for id in ["finish-1", "finish-3"] {
        let text = r.read(&format!("evals/pack/{id}.json"));
        assert!(text.contains("\"name\": \"hours-line\""), "{id}");
        assert!(text.contains("^Hours: 4\\\\.5( |$)"), "{id}");
    }
}

// The 5 evals `accept-1` to `accept-5` (design 0.8.0, D62).

/// The slug, the planted paths and the probe of each eval. A probe is Python
/// code that exits 0 when the behavior is right. It exits 1 for each eval with a
/// planted defect, and 0 for the control, `accept-5`.
const ACCEPT: [(&str, &str, &[&str], &str); 5] = [
    (
        "accept-1",
        "bulk-discount",
        &["src/discount.py"],
        "from discount import discount_percent\nassert discount_percent(100) == 10\n",
    ),
    (
        "accept-2",
        "order-export",
        &["src/export.py", "tests/test_export.py"],
        "from export import export_lines\nassert len(export_lines([{'id': 2, 'total': 300}, {'id': 1, 'total': 250}])) == 2\n",
    ),
    (
        "accept-3",
        "partial-refund",
        &["src/refund.py"],
        "from refund import refund_cents\nassert isinstance(refund_cents(999, 50), int)\n",
    ),
    (
        "accept-4",
        "item-sync",
        &["src/sync.py"],
        "from sync import sync_items\n\n\ndef send(item):\n    raise OSError('down')\n\n\ntry:\n    result = sync_items(['a'], send)\nexcept OSError:\n    result = ['raised']\nassert result not in ([], None)\n",
    ),
    (
        "accept-5",
        "parcel-fee",
        &[],
        // The control meets each line of the delta: the fee is added to the
        // shipping, so a strict acceptor has no ground to reject it.
        "from parcel import parcel_fee_cents, parcel_shipping_cents\nassert [parcel_fee_cents(w) for w in (19, 20, 21)] == [0, 500, 500]\nassert [parcel_shipping_cents(w) for w in (19, 20, 21)] == [300, 800, 800]\n",
    ),
];

fn copy_over(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_over(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

#[test]
fn an_accept_eval_is_a_built_change_of_a_higher_risk_intent_with_the_same_task_in_both_arms() {
    let r = project("[evals]\n");
    pack(&r);
    for (id, slug, planted, _) in ACCEPT {
        let text = r.read(&format!("evals/pack/{id}.json"));
        // The change is a commit on a branch (D60), after the base commit.
        assert!(
            text.contains(&format!("\"branch\": \"feature/{slug}\"")),
            "{id}"
        );
        assert!(
            text.contains(&format!("\"change\": \"evals/pack/changes/{id}\"")),
            "{id}"
        );
        // The tools of both arms (D62, 2026-11-22 item 5): no tool `Agent`, so
        // the agent of the eval is the acceptor itself.
        let tools: Vec<&str> = text
            .split("\"allowed-tools\": [")
            .nth(1)
            .unwrap()
            .split(']')
            .next()
            .unwrap()
            .split(',')
            .map(|t| t.trim().trim_matches('"'))
            .collect();
        assert_eq!(
            tools,
            [
                "Read",
                "Grep",
                "Glob",
                "Write",
                "Edit",
                "Bash(spec-dna accept:*)",
                "Bash(spec-dna next:*)",
                "Bash(spec-dna show:*)",
                "Bash(spec-dna context:*)",
                "Bash(spec-dna check:*)",
                "Bash(git status:*)",
                "Bash(git diff:*)",
                "Bash(git log:*)",
                "Bash(git merge-base:*)",
                "Bash(git add:*)",
                "Bash(git commit:*)",
                "Bash(python3 -m pytest:*)",
                "Bash(sh test.sh:*)",
            ],
            "{id}"
        );
        assert_eq!(number(&text, "max-budget-usd"), 2.5, "{id}");
        assert_eq!(number(&text, "max-turns"), 40.0, "{id}");
        // The task is the text of D62. It names no step of the skill, and no
        // option of `accept`: the arm with the skill gets the brief from it.
        assert!(
            text.contains(&format!(
                "\"task\": \"The change `{slug}` is built. You did not build it. Before `spec-dna finish`, check it. Run `spec-dna accept {slug}` for the skeleton of the acceptance record, and fill it in.\""
            )),
            "{id}: {text}"
        );
        for word in ["--brief", "acceptor", "review", "skill", "blocker"] {
            let task = text.split("\"task\": \"").nth(1).unwrap();
            let task = task.split("\",\n").next().unwrap();
            assert!(!task.contains(word), "{id}: the task names `{word}`");
        }
        // The oracle: the checks name the planted paths, and the control has no
        // check of a blocker.
        let checks = text.split("\"checks\"").nth(1).unwrap();
        for path in planted {
            assert!(checks.contains(path), "{id}: {path}");
        }
        assert_eq!(
            checks.contains("\"name\": \"blocker-found\""),
            !planted.is_empty()
        );
        assert_eq!(
            checks.contains("\"name\": \"accepted\""),
            planted.is_empty()
        );
        // The base commit has the approved change with the class `higher-risk`,
        // and the commit of the change ticks the box and adds the code.
        let fixture = format!("evals/pack/fixtures/{id}");
        let change = format!("evals/pack/changes/{id}");
        let intent = r.read(&format!("{fixture}/docs/intent/{slug}/intent.md"));
        assert!(intent.contains("\nclass: higher-risk\n"), "{id}");
        for doc in ["spec", "plan"] {
            let text = r.read(&format!("{fixture}/docs/intent/{slug}/{doc}.md"));
            assert!(text.starts_with("---\nstatus: approved\n"), "{id} {doc}");
        }
        let toml = r.read(&format!("{fixture}/spec-dna.toml"));
        assert!(toml.contains("approval = \"base\""), "{id}");
        assert!(toml.contains("profile = \"standard\""), "{id}");
        assert!(
            r.read(&format!("{fixture}/docs/intent/{slug}/plan.md"))
                .contains("- [ ] t1:"),
            "{id}"
        );
        assert!(
            r.read(&format!("{change}/docs/intent/{slug}/plan.md"))
                .contains("- [x] t1:"),
            "{id}"
        );
        assert!(
            !r.dir
                .join(format!("{fixture}/docs/intent/{slug}/acceptance.toml"))
                .exists(),
            "{id}: the record is the work of the agent"
        );
    }
}

#[test]
fn the_suite_of_each_accept_fixture_passes_and_the_planted_defect_is_there_except_in_the_control() {
    // The change is correct for its own tests, so a run with no review passes
    // `sh test.sh`. A probe of the requirement exposes the defect in evals 1 to
    // 4, and it passes in the control.
    let r = project("[evals]\n");
    pack(&r);
    let work = Repo::empty();
    for (id, _, planted, probe) in ACCEPT {
        let dir = work.dir.join(id);
        copy_over(&r.dir.join(format!("evals/pack/fixtures/{id}")), &dir);
        copy_over(&r.dir.join(format!("evals/pack/changes/{id}")), &dir);
        let suite = std::process::Command::new("sh")
            .arg("test.sh")
            .current_dir(&dir)
            .output()
            .unwrap();
        assert!(
            suite.status.success(),
            "{id}: the suite fails: {}",
            String::from_utf8_lossy(&suite.stderr)
        );
        let probe = std::process::Command::new("python3")
            .args(["-B", "-c", probe])
            .current_dir(&dir)
            .env("PYTHONPATH", dir.join("src"))
            .output()
            .unwrap();
        assert_eq!(
            probe.status.success(),
            planted.is_empty(),
            "{id}: {}",
            String::from_utf8_lossy(&probe.stderr)
        );
        for path in planted {
            assert!(dir.join(path).is_file(), "{id}: {path}");
        }
    }
}

#[test]
fn a_second_eval_pack_writes_the_accept_evals_that_an_older_pack_lacks_and_keeps_the_others() {
    // Design 0.8.0, D62, "Upgrade": a project that ran `eval pack` with 0.7.1
    // has no file of the evals `accept-1` to `accept-5`.
    let r = project("[evals]\n");
    pack(&r);
    for n in 1..=5 {
        fs::remove_file(r.dir.join(format!("evals/pack/accept-{n}.json"))).unwrap();
        fs::remove_dir_all(r.dir.join(format!("evals/pack/fixtures/accept-{n}"))).unwrap();
    }
    fs::remove_dir_all(r.dir.join("evals/pack/changes")).unwrap();
    let (code, out, err) = r.run(&["eval", "pack", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let (written, kept) = out.split_once("\"kept\":").unwrap();
    assert!(written.contains("evals/pack/accept-1.json"), "{out}");
    assert!(written.contains("evals/pack/fixtures/accept-5/spec-dna.toml"));
    assert!(written.contains("evals/pack/changes/accept-3/src/refund.py"));
    assert!(!written.contains("tdd-1"), "{written}");
    assert!(kept.contains("evals/pack/tdd-1.json"), "{kept}");
    assert!(!kept.contains("accept-"), "{kept}");
}
