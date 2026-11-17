//! Tests of `spec-dna eval pack` (ADR 0016, 0.7.0 slice 5). The command
//! writes the shipped evals and their small fixtures to `<[evals] dir>/pack/`.
//! A test checks that the pack holds the 57 evals of the 11 skills and of
//! `agents-line`. The other
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
        // Each file has the key `skill`, and the id is `<skill>-<n>`.
        let skill = id.rsplit_once('-').unwrap().0;
        assert!(SKILLS.contains(&skill) || skill == "agents-line", "{id}");
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
    let skills: Vec<String> = packed(&r)
        .iter()
        .map(|n| n.rsplit_once('-').unwrap().0.to_string())
        .collect();
    assert!(!skills.is_empty());
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
        for line in text.lines().filter(|l| l.contains("spec-dna check")) {
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

// The tests that need the 57 evals of the pack.

/// The ids of each skill: `verify` has the ids 6 to 10, the others 1 to 5.
fn ids_of(skill: &str) -> Vec<String> {
    let (from, to) = if skill == "verify" { (6, 10) } else { (1, 5) };
    (from..=to).map(|n| format!("{skill}-{n}")).collect()
}

#[test]
fn the_pack_has_5_evals_for_each_skill_and_2_of_agents_line() {
    let r = project("[evals]\n");
    let (code, out, err) = pack(&r);
    assert_eq!(code, 0, "{out}{err}");
    let names = packed(&r);
    assert_eq!(names.len(), 57, "{names:?}");
    for skill in SKILLS {
        for id in ids_of(skill) {
            assert!(names.contains(&format!("{id}.json")), "{id}");
        }
    }
    for id in ["agents-line-1", "agents-line-2"] {
        let text = r.read(&format!("evals/pack/{id}.json"));
        assert!(text.contains("\"skill\": \"agents-line\""), "{id}");
        assert!(names.contains(&format!("{id}.json")), "{id}");
    }
    // The ids of the pilot of `verify` are gone: a record names its eval.
    for n in 1..=5 {
        assert!(!names.contains(&format!("verify-{n}.json")), "verify-{n}");
    }
    // Ids are unique and have the form `<group>-<n>` with n from 1.
    for name in &names {
        let (group, n) = name
            .strip_suffix(".json")
            .unwrap()
            .rsplit_once('-')
            .unwrap();
        assert!(n.parse::<u32>().is_ok_and(|n| n >= 1), "{name}");
        assert!(SKILLS.contains(&group) || group == "agents-line", "{name}");
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
fn the_agents_line_set_is_tdd_1_debug_1_execute_1_and_2_own_evals() {
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
            "execute-1",
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

// The turn cap of the evals that the proof of `execute-1` measured (session 26).
// In the pilot, 4 of 4 runs of `execute-1` with the line of `AGENTS.md` stopped
// at the cap of 25 turns with correct work: the line costs 3 to 7 turns that
// only run `spec-dna`. So an eval of `execute`, and the evals `tdd-1` and
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
        "execute-1",
        "execute-2",
        "execute-3",
        "execute-4",
        "execute-5",
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
fn the_fixture_of_execute_1_has_a_requirement_that_a_covers_tag_can_name() {
    let r = project("[evals]\n");
    pack(&r);
    let fixture = "evals/pack/fixtures/execute-1";
    let counts = r.read(&format!("{fixture}/docs/counts.md"));
    assert!(counts.contains("- **count-words**:"), "{counts}");
    // A project with this document and a test that names the requirement
    // gives no SD105.
    let p = Repo::with_config(
        "\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n",
    );
    p.write("docs/counts.md", &counts);
    p.write(
        "tests/test_words.py",
        "# covers: counts#count-words\nimport unittest\n",
    );
    p.commit("start");
    let (_, out, err) = p.run(&["check", "--all"]);
    assert!(!out.contains("SD105"), "{out}{err}");
    // The old tag names a section, and it is no requirement.
    p.write(
        "tests/test_words.py",
        "# covers: counts#counts\nimport unittest\n",
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
    assert!(text.contains("\"branch\": \"feature/overdue-fee\""), "{text}");
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
